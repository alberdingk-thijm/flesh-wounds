extern crate tui;
extern crate termion;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate failure;
extern crate strum;
#[macro_use] extern crate strum_macros;

use termion::input::TermRead;
use termion::event;
use failure::Error;
use tui::backend::RawBackend;
use tui::Terminal;

use std::sync::mpsc;
use std::thread;

use std::io::{self, BufReader, BufWriter};
use std::fs::File;
use std::path::Path;

use std::collections::BTreeMap;

mod meters;
mod combatants;

use meters::Meter;
use combatants::{Combatant, Classes, Abilities};

/// Enum for handling thread-sent events.
#[derive(Debug, PartialEq)]
enum Event {
    /// Key input
    Input(event::Key),
    // Timer tick
    //Tick,
}

/// Controls for determining the input mode
/// of the battle.
#[derive(Debug, PartialEq)]
enum Mode {
    /// Awaiting zero or more characters, followed by a newline
    Insert(MsgType),
    // Awaiting one character
    //Char,
    // Awaiting key sequences to complete command
    //Command(MsgType),
    /// Awaiting a key interpreted as the start of a command
    Normal,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

/// Specifies the type of message we want to parse.
#[derive(EnumString, Display, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum MsgType {
    Abilities,
    AC,
    Attacks,
    Class,
    Healing,
    HP,
    HD,
    Init,
    Team,
    Damage,
    Name,
    SaveFileName,
    OpenFileName,
}

const _HELP : &'static str = "
    Flesh Wounds Help:\r
    F1          display help\r
    ctrl-c, q   quit\r
    ctrl-s      save\r
    ctrl-o      open\r
    n           new combatant\r
    i           set combatant team and initiative\r
    e           set combatant ability scores\r
    +           set combatant attacks\r
    H           set combatant HP\r
    a           attack self->other\r
    d           damage self\r
    h           heal self\r
    x           advance one round\r
    y           duplicate combatant\r
    z           display combatant xp\r
    Return      select combatant\r
    j           scroll down\r
    k           scroll up\r

    Press Enter to close this help and return to the program.\r
";

#[derive(Fail, Debug)]
enum CombatError {
    #[fail(display = "Not enough attacks left")]
    NotEnoughAttacks,
    #[fail(display = "Not in combat")]
    NotInCombat,
}

const MAX_COMBATANTS : usize = 32;

struct Battle {
    size: tui::layout::Rect,
    mode: Mode,
    input: String,
    requests: Vec<MsgType>,
    messages: BTreeMap<MsgType, String>,
    sel: Option<usize>,
    combatants: Vec<Combatant>,
    round: u32,
    pos: usize,
    autosave: Option<AutosaveSettings>,
}

struct AutosaveSettings {
    prefix: String,
    max_saves: u32,
    save: u32,
}

impl AutosaveSettings {
    fn get_save_path(&mut self) -> String {
        self.save = (self.save + 1) % self.max_saves;
        format!("{}{}.json", self.prefix, self.save)
    }
}

impl Default for AutosaveSettings {
    /// Create default autosave.
    fn default() -> Self {
        AutosaveSettings { prefix: ".auto".into(), max_saves: 5, save: 0 }
    }
}

// #[derive(Debug, Fail)]
// enum BattleError {
//     #[fail(display = "No input received")]
//     NoInput,
// }

impl Battle {
    fn new() -> Self {
        Battle {
            size: tui::layout::Rect::default(),
            mode: Mode::default(),
            input: String::new(),
            requests: vec![],
            messages: BTreeMap::new(),
            sel: None,
            combatants: Vec::with_capacity(MAX_COMBATANTS),
            round: 1,
            pos: 0,
            autosave: Some(AutosaveSettings::default()),
        }
    }

    /// Load combatants from a file.
    fn load_combat<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let (round, combatants) : (u32, Vec<Combatant>) = serde_json::from_reader(reader)?;
        self.round = round;
        self.combatants = combatants;
        Ok(())
    }

    fn save_combat<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let f = File::create(path)?;
        let writer = BufWriter::new(f);
        let () = serde_json::to_writer_pretty(writer, &(self.round, &self.combatants))?;
        Ok(())
    }

    /// Autosave game state.
    fn autosave(&mut self) -> Result<(), Error> {
        let x = if let Some(ref mut a) = self.autosave {
            a.get_save_path()
        } else {
            // jump out
            return Ok(())
        };
        self.save_combat(x)
    }

    // fn draw(&mut self) {
    //     // clear the screen
    //     write!(self.stdout, "{}{}", clear::All, cursor::Goto(1, 2)).unwrap();
    //     // write the top frame
    //     self.draw_border(true);
    //     self.stdout.write(b"\n\r").unwrap();
    //     // write combatant name and display combatant info
    //     for i in 0..self.combatants.len() {
    //         self.draw_combatant_row(i as usize);
    //     }
    //     // write the bottom frame
    //     self.draw_border(false);
    //     // draw prompt box
    //     self.draw_prompt_box();
    //     // draw details
    //     self.draw_details();
    //     write!(self.stdout, "{}", cursor::Goto(1, 1)).unwrap();
    //     self.stdout.flush().unwrap();
    // }

    // fn draw_details(&mut self) {
    //     if let Some(p) = self.sel {
    //         write!(self.stdout, "{}{}",
    //                cursor::Goto(1, self.height + 4),
    //                self.combatants[p].describe()).unwrap();
    //         self.stdout.flush().unwrap();
    //     }
    // }

    /// Update the battle based on the given event.
    fn update(&mut self, evt: Event) -> Result<(), Error> {
        macro_rules! get_or_req {
            ($msg:expr, $process:expr) => {
                {
                    if let Some(p) = self.messages.get(&$msg) {
                        //self.mode = Mode::Command(msg);
                        $process(p)
                    } else {
                        self.mode = Mode::Insert($msg);
                        return Ok(());
                    }
                }
            };
        }
        use termion::event::Key::*;
        match self.mode {
            Mode::Insert(msg) => {
                match evt {
                    Event::Input(input) => match input {
                        Char('\n') => {
                            self.messages.insert(msg, self.input.drain(..).collect());
                            if let Some(req) = self.requests.pop() {
                                self.mode = Mode::Insert(req);
                            } else {
                                self.mode = Mode::Normal;
                            }
                        },
                        Char(c) => {
                            self.input.push(c);
                        },
                        Backspace => {
                            self.input.pop();
                        },
                        Ctrl('c') => {
                            // erase input and cancel command
                            self.input.clear();
                            self.requests.clear();
                            self.mode = Mode::Normal;
                        },
                        _ => (),
                    },
                }
            },
            // Mode::Char => {
            //     if let Some(msg) = self.requests.pop() {
            //         match evt {
            //             Event::Input(input) => match input {
            //                 Char(c) => {
            //                     self.messages.insert(msg, c.to_string());
            //                 },
            //                 Ctrl('c') => {
            //                     // cancel command
            //                     self.requests.clear();
            //                     self.mode = Mode::Normal;
            //                 },
            //                 _ => (),
            //             },
            //         }
            //     } else {
            //         self.mode = Mode::Normal;
            //     }
            // },
            _ => {
                match evt {
                    Event::Input(input) => match input {
                        Ctrl('s') => {
                            let save = get_or_req!(MsgType::SaveFileName,
                                |p| p);
                            self.save_combat(save)?;
                        },
                        Ctrl('o') => {
                            let open = get_or_req!(MsgType::OpenFileName,
                                |p : &String| p.clone());
                            self.load_combat(open)?;
                        },
                        Char('j') => self.down(),
                        Char('k') => self.up(),
                        Char('x') => self.advance(),
                        Char('n') => {
                            self.requests = vec![MsgType::AC, MsgType::Class, MsgType::HD, MsgType::Attacks, MsgType::HP, MsgType::Name];
                            let name = get_or_req!(MsgType::Name,
                                |p: &String| p.clone());
                            let hp = get_or_req!(MsgType::HP,
                                |p: &String| p.parse::<Meter<i32>>())?;
                            let atts = get_or_req!(MsgType::Attacks,
                                |p: &String| p.parse::<Meter<u32>>())?;
                            // TODO: add HD as distinct from class/level
                            let _ = get_or_req!(MsgType::HD,
                                |p: &String| p.parse::<u32>())?;
                            let class = get_or_req!(MsgType::Class,
                                |p: &String| p.parse::<Classes>())?;
                            let ac = get_or_req!(MsgType::AC,
                                |p: &String| p.parse::<i32>())?;
                            self.add_combatant(name, hp, atts, class, ac)?;
                        },
                        Char('i') => {
                            let team = get_or_req!(MsgType::Team,
                                |p: &String| p.parse::<u32>()).ok();
                            let init = get_or_req!(MsgType::Init,
                                |p: &String| p.parse::<u32>()).ok();
                            self.init_combatant(team, init);
                        },
                        Char('e') => {
                            let abils = get_or_req!(MsgType::Abilities,
                                |p: &String| p.parse::<Abilities>()).ok();
                            self.add_abilities(abils);
                        },
                        Char('\n') => {
                            self.sel = match self.sel {
                                Some(i) if i == self.pos => None,
                                _ => Some(self.pos),
                            };
                        },
                        Char('+') => {
                            let atts = get_or_req!(MsgType::Attacks,
                                |p: &String| p.parse::<Meter<u32>>())?;
                            self.set_attacks(atts);
                        },
                        Char('a') => {
                            // make sure from has enough attacks
                            let dam = get_or_req!(MsgType::Damage,
                                |p: &String| p.parse::<i32>())?;
                            self.attack(dam)?;
                        },
                        Char('d') => {
                            let dam = get_or_req!(MsgType::Damage,
                                |p: &String| p.parse::<i32>())?;
                            self.damage(dam);
                        },
                        Char('H') => {
                            let hp = get_or_req!(MsgType::HP,
                                |p: &String| p.parse::<Meter<i32>>())?;
                            self.set_hp(hp);
                        },
                        Char('h') => {
                            let heal = get_or_req!(MsgType::Healing,
                                |p: &String| p.parse::<i32>())?;
                            self.heal(heal);
                        },
                        Char('y') => {
                            let s = get_or_req!(MsgType::Name,
                                |p: &String| p.clone());
                            let name = if s.len() == 0 {
                                None
                            } else {
                                Some(s)
                            };
                            self.copy_combatant(name);
                        },
                        Char('z') => {
                            self.get_xp().unwrap();
                        },
                        Char('C') => {
                            // Reset all combatants.
                            for comb in &mut self.combatants {
                                comb.reset();
                            }
                        },
                        F(1) => {
                            // display help
                        },
                        _ => (),
                    },
                }
                self.messages.clear();
                self.mode = Mode::Normal;
            },
        }
        self.autosave()?;
        Ok(())
    }

    /// Advance to the next round.
    fn advance(&mut self) {
        self.round += 1;
        self.sort();
        for c in &mut self.combatants {
            c.update();
        }
    }

    /// Sort the combatants' ordering based on initiative and status.
    /// Remove any combatants with Status::Dead from the table.
    fn sort(&mut self) {
        // produce iterator of combatants
        let mut initiatives = self.combatants.clone().into_iter()
            // calculate init
            .map(|c| (c.get_init(), c))
            // filter out dead, but keep uninitialized
            .filter(|&(i, _)| match i { Some(n) => n > 0, None => true })
            .collect::<Vec<_>>();
        // sort with fastest at the top (None elements go to bottom!)
        initiatives.sort_by(|a, b| b.0.cmp(&a.0));
        self.combatants = initiatives.into_iter()
            .map(|(_, c)| c)
            .collect::<Vec<_>>();
        // reset pos to 0 to avoid errors
        self.pos = 0;
    }

    /// Add a combatant to the battle.
    fn add_combatant(&mut self, name: String, hp: Meter<i32>, atts: Meter<u32>, class: Classes, ac: i32) -> Result<(), Error> {
        let c = Combatant::new(name, hp, atts, class, ac);
        self.combatants.push(c);
        self.sort();
        Ok(())
    }

    fn add_abilities(&mut self, abils: Option<Abilities>) {
        if self.pos < self.combatants.len() {
            self.combatants[self.pos].abilities = abils;
        }
    }

    /// Initialize the combatant underneath the cursor.
    fn init_combatant(&mut self, team: Option<u32>, init: Option<u32>) {
        if self.pos < self.combatants.len() {
            self.combatants[self.pos].team = team;
            self.combatants[self.pos].init = init;
        }
    }

    /// Duplicate the combatant underneath the cursor, renaming if given a new name.
    fn copy_combatant<S: Into<String>>(&mut self, name: Option<S>) {
        if let Some(f) = self.sel {
            let mut c = self.combatants[f].clone();
            if let Some(name) = name {
                c.rename(name);
            }
            self.combatants.push(c);
        }
    }

    /// Add damage to selected.
    fn damage(&mut self, dam: i32) {
        if let Some(f) = self.sel {
            self.combatants[f].recv_hit(dam);
        }
    }

    /// Perform an attack from selected to the current target, consuming attacks.
    fn attack(&mut self, dam: i32) -> Result<(), CombatError> {
        let t = self.pos;
        if let Some(f) = self.sel {
            if self.combatants[f].in_combat() {
                if self.combatants[f].can_attack() {
                    self.combatants[f].deal_hit(dam);
                    self.combatants[t].recv_hit(dam);
                } else {
                    return Err(CombatError::NotEnoughAttacks);
                }
            } else {
                return Err(CombatError::NotInCombat);
            }
        }
        Ok(())
    }

    /// Change the selected combatant's attacks.
    fn set_attacks(&mut self, atts: Meter<u32>) {
        if let Some(f) = self.sel {
            self.combatants[f].attacks = atts;
        }
    }

    /// Change the selected combatant's hp.
    fn set_hp(&mut self, hp: Meter<i32>) {
        if let Some(f) = self.sel {
            self.combatants[f].hp = hp;
        }
    }

    /// Heal the selected combatant.
    fn heal(&mut self, dam: i32) {
        if let Some(f) = self.sel {
            self.combatants[f].heal(dam);
        }
    }

    fn down(&mut self) {
        if self.pos + 1 < self.combatants.len() {
            self.pos += 1;
        }
    }

    fn up(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// Return xp earned by the selected combatant.
    fn get_xp(&mut self) -> Option<i32> {
        self.sel.map(|f| {
            let ref c = self.combatants[f];
            let n = self.combatants.len() as i32;
            let team_bonus = self.combatants.iter().filter(|x| x.team == c.team)
                .fold(0, |acc, ref x| acc + (x.team_xp() / n));
            c.xp(team_bonus)
        })
    }
}

fn draw(t: &mut Terminal<RawBackend>, b: &Battle) -> Result<(), Error> {
    use tui::widgets::{
        Widget, Table, Block, Row, Borders, Paragraph
    };
    use tui::style::{Style, Color};
    use tui::layout::{Group, Size, Direction};

    let row_style = Style::default().fg(Color::White);
    let mut rows = vec![];
    for comb in &b.combatants {
        let row_data = vec![
            comb.name.clone(),
            match comb.team {
                Some(t) => t.to_string(),
                None => String::from(""),
            },
            match comb.init {
                Some(t) => t.to_string(),
                None => String::from(""),
            },
            comb.hp.to_string(),
            comb.attacks.to_string(),
            comb.thac0.to_string(),
            comb.status.to_string()
        ];
        rows.push(Row::StyledData(row_data.into_iter(), &row_style));
    }

    Group::default()
        .direction(Direction::Vertical)
        .margin(1)
        .sizes(&[Size::Min(1), Size::Fixed(3)])
        .render(t, &b.size, |t, chunks| {
            Table::new(
                ["Name", "T", "I", "HP", "Att", "AC", "TH", ""].into_iter(),
                rows.into_iter()
                )
                .block(Block::default().title(&format!("Round: {}", b.round)).borders(Borders::ALL))
                .header_style(Style::default().fg(Color::Yellow))
                .widths(&[16, 1, 1, 9, 5, 2, 2, 1])
                .style(Style::default().fg(Color::White))
                .column_spacing(1)
                .render(t, &chunks[0]);
            Paragraph::default()
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().title("Prompt"))
                .text(match b.mode {
                    Mode::Insert(p) => format!("> {}: {}", p, b.input),
                    //Mode::Char => format!("> {:?}: {}", b.requests[0], b.input),
                    //Mode::Command => format!("{:?}", p),
                    _ => "".into(),
                }.as_str())
                .render(t, &chunks[1]);
        });

    t.draw()?;
    Ok(())
}

fn main() -> Result<(), Error> {
    // Start input thread
    let (tx, rx) = mpsc::channel();
    let input_tx = tx.clone();

    thread::spawn(move || {
        let stdin = io::stdin();
        for c in stdin.keys() {
            let evt = c.unwrap();
            input_tx.send(Event::Input(evt)).unwrap();
            if evt == event::Key::Char('q') {
                break;
            }
        }
    });

    let backend = RawBackend::new()?;
    let mut term = Terminal::new(backend)?;
    term.clear()?;
    term.hide_cursor()?;
    let mut b = Battle::new();

    loop {
        let size = term.size()?;
        if size != b.size {
            term.resize(size)?;
            b.size = size;
        }
        draw(&mut term, &b)?;

        use termion::event::Key::*;
        let evt = rx.recv().unwrap();
        match evt {
            Event::Input(Char('q')) => break,
            Event::Input(F(1)) => {
                // display help
            }
            _ => {
                // TODO: display possible errors
                b.update(evt).ok();
            },
        }
    }

    term.show_cursor()?;
    term.clear()?;

    Ok(())
}
