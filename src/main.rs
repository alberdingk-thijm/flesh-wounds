#![allow(dead_code)]
extern crate termion;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate failure;
extern crate strum;
//#[macro_use] extern crate strum_macros;

use termion::{clear, cursor, style};
use termion::raw::IntoRawMode;
use termion::input::{Keys, TermRead};
use termion::event::Key;
use failure::Error;

use std::io::{self, BufReader, BufWriter, Read, Write};
use std::fs::File;
use std::path::Path;

#[macro_use]
mod macros {
    /// Create a boxed closure for generating colored versions of strings.
    macro_rules! colorize {
        ( $col:expr ) => {
            Box::new(| s | {
                format!("{}{}{}", color::Fg($col), s, color::Fg(color::Reset))
            })
        }
    }
    /// Write a series of colored cells out, where each cell specifies how it is represented.
    macro_rules! color_cells {
        ( $fmt:expr, col = $col:expr, sep = $sep:expr, $( $e:expr => $f:expr ),* ) => {
            // Insert optional color wrapper around each block.
            write!($fmt, "{}", 
                &[
                    $(match $col {
                        Some(ref c) => {
                            // annotate type
                            let f : &Box<Fn(String) -> String> = c;
                            f(format!($f, $e))
                        },
                        None => format!($f, $e)
                    }),*
                ].join($sep)
            )
        }
    }
}

mod meters;
mod loader;
mod combatants;

use meters::Meter;
use combatants::{Combatant, Classes, Abilities};

// Box drawing characters
const TOP_RIGHT : &'static str = "┐";
const TOP_LEFT : &'static str = "┌";
const BOTTOM_RIGHT : &'static str = "┘";
const BOTTOM_LEFT : &'static str = "└";
//const CROSS : &'static str = "┼";
const HORZ : &'static str = "─";
//const LEFT_TEE : &'static str = "├";
//const RIGHT_TEE : &'static str = "┤";
const BOTTOM_TEE : &'static str = "┴";
const TOP_TEE : &'static str = "┬";
const VERT : &'static str = "│";

const MAX_COMBATANTS : usize = 32;

struct Battle<R: Read, W: Write> {
    stdin: Keys<R>,
    stdout: W,
    sel: Option<usize>,
    combatants: Vec<Combatant>,
    round: u32,
    pos: usize,
    width: u16,
    height: u16,
}

#[derive(Debug, Fail)]
enum BattleError {
    #[fail(display = "No input received")]
    NoInput,
    #[fail(display = "Parse error")]
    Parse,
}

impl<R: Read, W: Write> Battle<R, W> {
    fn new(stdin: R, stdout: W) -> Self {
        Battle {
            stdin: stdin.keys(),
            stdout: stdout,
            sel: None,
            combatants: Vec::with_capacity(MAX_COMBATANTS),
            round: 1,
            pos: 0,
            width: format!("{}", Combatant::default()).len() as u16,
            height: MAX_COMBATANTS as u16,
        }
    }

    /// Load combatants from a file.
    fn load_combatants<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let combatants : Vec<Combatant> = serde_json::from_reader(reader)?;
        self.combatants = combatants;
        Ok(())
    }

    fn save_combatants<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let f = File::create(path)?;
        let writer = BufWriter::new(f);
        let () = serde_json::to_writer_pretty(writer, &self.combatants)?;
        Ok(())
    }

    fn draw(&mut self) {
        // clear the screen
        write!(self.stdout, "{}{}", clear::All, cursor::Goto(1, 2)).unwrap();
        // write the top frame
        self.draw_border(true);
        self.stdout.write(b"\n\r").unwrap();
        // write combatant name and display combatant info
        for i in 0..self.combatants.len() {
            self.draw_combatant_row(i as usize);
        }
        // write the bottom frame
        self.draw_border(false);
        // draw prompt box
        self.draw_prompt_box();
        // draw details
        self.draw_details();
        write!(self.stdout, "{}", cursor::Goto(1, 1)).unwrap();
        self.stdout.flush().unwrap();
    }

    fn draw_border(&mut self, top: bool) {
        //let v = VERT.chars().next().unwrap();
        //let mut def = format!("{}", Combatant::default())
            //.replace(|c| c != v, HORZ);
        if top {
            self.stdout.write(TOP_LEFT.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            //def = def.replace(v, TOP_TEE);
            //def = format!("R: {:02}{}", self.round, def.chars().skip(5).collect::<String>());
            let def = format!("R: {rn:02}{h1}{sep}{t}{sep}{i}{sep}{hp}{h2}{sep}{at}{sep}{ac:02}{sep}{th:02}{sep}{st}",
                          rn = self.round, h1 = HORZ.repeat(11), t = "T", i = "I", hp = "HP", h2 = HORZ.repeat(7),
                          at = "Att", ac = "AC", th="TH", st=HORZ, sep=format!("{0}{1}{0}", HORZ, TOP_TEE));
            self.stdout.write(def.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            self.stdout.write(TOP_RIGHT.as_bytes()).unwrap();
        } else {
            self.stdout.write(BOTTOM_LEFT.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            color_cells!(self.stdout, col = None, sep = &format!("{0}{1}{0}", HORZ, BOTTOM_TEE),
                   HORZ.repeat(16) => "{}",
                   HORZ => "{}",
                   HORZ => "{}",
                   HORZ.repeat(9) => "{}",
                   HORZ.repeat(3) => "{}",
                   HORZ.repeat(2) => "{}",
                   HORZ.repeat(2) => "{}",
                   HORZ => "{}").unwrap();
            //def = def.replace(v, BOTTOM_TEE);
            //self.stdout.write(def.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            self.stdout.write(BOTTOM_RIGHT.as_bytes()).unwrap();
        }
    }

    fn draw_combatant_row(&mut self, i: usize) {
        self.stdout.write(VERT.as_bytes()).unwrap();
        if i < self.combatants.len() {
            let ref c = self.combatants[i];
            let ctext = format!("{}", c);
            match self.sel {
                Some(t) if t == i => {
                    write!(self.stdout, " {}{}{} ", style::Bold, ctext, style::Reset)
                },
                _ if self.pos == i => {
                    write!(self.stdout, " {}{}{} ", style::Invert, ctext, style::Reset)
                },
                _ => write!(self.stdout, " {} ", c),
            }.unwrap();
        } else {
            let c = Combatant::default();
            write!(self.stdout, " {} ", format!("{}", c).replace(|c : char| c != '│', " ")).unwrap();
        }
        self.stdout.write(VERT.as_bytes()).unwrap();
        self.stdout.write(b"\n\r").unwrap();
    }

    fn draw_prompt_box(&mut self) {
        write!(self.stdout, "{}> ", cursor::Goto(1, 1)).unwrap();
        self.stdout.flush().unwrap();
    }

    fn draw_details(&mut self) {
        if let Some(p) = self.sel {
            write!(self.stdout, "{}{}",
                   cursor::Goto(1, self.height + 4),
                   self.combatants[p].describe()).unwrap();
            self.stdout.flush().unwrap();
        }
    }

    /// Start the battle.
    fn start(&mut self) {
        loop {
            // iterator returns an Option<Result<Key, Error>>
            let b = self.stdin.next().unwrap().unwrap();
            use termion::event::Key::*;
            match b {
                Ctrl('s') => {
                    let p = self.read_line("Save to file: ").unwrap();
                    self.save_combatants(p).unwrap();
                },
                Ctrl('o') => {
                    let p = self.read_line("Open file: ").unwrap();
                    self.load_combatants(p).unwrap();
                }
                Char('\n') => {
                    self.sel = match self.sel {
                        Some(i) if i == self.pos => None,
                        _ => Some(self.pos),
                    };
                },
                Char('j') => self.down(),
                Char('k') => self.up(),
                Char('q') => return,
                Char('n') => {
                    // TODO: don't eat the error
                    self.add_combatant().unwrap_or(());
                },
                Char('i') => {
                    self.init_combatant();
                },
                Char('e') => {
                    self.add_abilities();
                },
                Char('+') => {
                    let atts = self.read_line("Attacks: ").unwrap().parse::<Meter<u32>>();
                    atts.map(|a| self.boost_attacks(a)).ok();
                },
                Char('a') => {
                    // make sure from has enough attacks
                    let dam = self.read_line("Damage: ").unwrap().parse::<i32>();
                    dam.map(|d| self.attack(d)).ok();
                },
                Char('d') => {
                    let dam = self.read_line("Damage: ").unwrap().parse::<i32>();
                    dam.map(|d| self.damage(d)).ok();
                }
                Char('h') => {
                    let heal = self.read_line("Healing: ").unwrap().parse::<i32>();
                    heal.map(|h| self.heal(h)).ok();
                },
                Char('x') => {
                    self.advance();
                },
                Char('y') => {
                    let s = self.read_line("Name: ").ok();
                    let name = s.and_then(|s| if s.len() == 0 {
                        None
                    } else {
                        Some(s)
                    });
                    self.copy_combatant(name);
                },
                Ctrl('c') => {
                    // terminate signal
                    return
                },
                _ => {},
            }

            self.draw();
            write!(self.stdout, "{}", cursor::Goto(1, 1)).unwrap();
            self.stdout.flush().unwrap();
        }
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

    fn read_char(&mut self, prompt: &str) -> Option<char> {
        write!(self.stdout, "{}> {}{}", cursor::Goto(1, 1), prompt, cursor::Show).unwrap();
        self.stdout.flush().unwrap();
        if let Some(Ok(Key::Char(c))) = self.stdin.next() {
            write!(self.stdout, "{}{}{}", c, cursor::Hide, cursor::Goto(1, 1)).unwrap();
            self.stdout.flush().unwrap();
            Some(c)
        } else {
            None
        }
    }

    fn read_line(&mut self, prompt: &str) -> Result<String, BattleError> {
        let mut s = String::from("");
        write!(self.stdout, "{}> {}{}", cursor::Goto(1, 1), prompt, cursor::Show).unwrap();
        self.stdout.flush().unwrap();
        while let Some(Ok(k)) = self.stdin.next() {
            match k {
                Key::Char('\n') | Key::Char('\r') => {
                    write!(self.stdout, "{}", clear::CurrentLine).unwrap();
                    break
                },
                Key::Char(c) => {
                    write!(self.stdout, "{}", c).unwrap();
                    self.stdout.flush().unwrap();
                    s.push(c)
                },
                Key::Ctrl('c') => {
                    return Err(BattleError::NoInput)
                },
                Key::Backspace => { 
                    // remove a character
                    if let Some(_) = s.pop() {
                        write!(self.stdout, "{}{}", cursor::Left(1), clear::UntilNewline).unwrap();
                        self.stdout.flush().unwrap();
                    }
                },
                _ => (),
            }
        }
        write!(self.stdout, "{}{}", cursor::Hide, cursor::Goto(1, 1)).unwrap();
        self.stdout.flush().unwrap();
        Ok(s)
    }

    /// Add a combatant to the battle.
    fn add_combatant(&mut self) -> Result<(), Error> {
        // receive info from user
        let name = self.read_line("Name: ")?;
        write!(self.stdout, "{}", clear::CurrentLine).unwrap();
        let mut input = self.read_line("HP: ")?;
        let hp = input.parse::<Meter<i32>>()?;
        input = self.read_line("Attacks: ")?;
        let atts = input.parse::<Meter<u32>>()?;
        let cstr = self.read_line("Class: ")?;
        let mut class = cstr.parse::<Classes>()?;
        // if no level provided, ask
        if !cstr.ends_with(char::is_numeric) {
            input = self.read_line("Level/HD: ")?;
            let lvlhd = input.parse::<u32>()?;
            class = class.lvl(lvlhd);
        }
        input = self.read_line("AC: ")?;
        let ac = input.parse::<i32>()?;
        let c = Combatant::new(name, hp, atts, class, ac);
        self.combatants.push(c);
        self.sort();
        Ok(())
    }

    fn add_abilities(&mut self) {
        if self.pos < self.combatants.len() {
            let abils = self.read_line("Ability scores (STR/INT/WIS/DEX/CON/CHA): ")
                .unwrap().parse::<Abilities>().ok();
            self.combatants[self.pos].abilities(abils);
        }
    }

    /// Initialize the combatant underneath the cursor.
    fn init_combatant(&mut self) {
        if self.pos < self.combatants.len() {
            let team = self.read_char("Team: ").and_then(|c| c.to_digit(10));
            self.combatants[self.pos].team(team);
            write!(self.stdout, "{}", clear::CurrentLine).unwrap();
            let init = self.read_char("Initiative: ").and_then(|c| c.to_digit(10));
            self.combatants[self.pos].init(init);
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
    fn attack(&mut self, dam: i32) {
        let t = self.pos;
        if let Some(f) = self.sel {
            if self.combatants[f].in_combat() {
                if self.combatants[f].can_attack() {
                    self.combatants[f].deal_hit(dam);
                    self.combatants[t].recv_hit(dam);
                } else {
                    write!(self.stdout, "Not enough attacks left!").unwrap();
                    self.stdout.flush().unwrap();
                }
            } else {
                write!(self.stdout, "Not in combat yet!").unwrap();
                self.stdout.flush().unwrap();
            }
        }
    }

    /// Change the selected combatant's attacks.
    fn boost_attacks(&mut self, atts: Meter<u32>) {
        if let Some(f) = self.sel {
            self.combatants[f].attacks(atts);
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
}

fn init<R: Read, W: Write>(stdin: R, mut stdout: W) {
    write!(stdout, "{}{}", clear::All, cursor::Hide).unwrap();
    let mut b = Battle::new(stdin, stdout);
    b.draw();
    b.start();
}

impl<R: Read, W: Write> Drop for Battle<R, W> {
    fn drop(&mut self) {
        // When done, restore the defaults to avoid messing with the terminal.
        write!(self.stdout, "{}{}{}{}", clear::All, style::Reset, cursor::Goto(1, 1), cursor::Show).unwrap();
    }
}

fn main() {
    println!("Hello, world!");

    let stdout = io::stdout();
    let stdout = stdout.lock();
    let stdin = io::stdin();
    let stdin = stdin.lock();

    let stdout = stdout.into_raw_mode().unwrap();

    let termsize = termion::terminal_size().ok();
    let _termwidth = termsize.map(|(w,_)| w - 2);
    let _termheight = termsize.map(|(_,h)| h - 2);
    init(stdin, stdout) //, 80, 40); //termwidth.unwrap_or(80), termheight.unwrap_or(40));
}
