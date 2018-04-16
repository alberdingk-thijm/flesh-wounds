#![allow(dead_code)]
extern crate termion;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use termion::{clear, cursor, color, style};
use termion::raw::IntoRawMode;
use termion::input::{Keys, TermRead};
use termion::event::Key;

use std::io::{self, Read, Write};

mod meters;
mod loader;
mod combatants;

use meters::Meter;
use combatants::{Combatant, Classes, Class};

// Box drawing characters
const TOP_RIGHT : &'static str = "┐";
const TOP_LEFT : &'static str = "┌";
const BOTTOM_RIGHT : &'static str = "┘";
const BOTTOM_LEFT : &'static str = "└";
const CROSS : &'static str = "┼";
const HORZ : &'static str = "─";
const LEFT_TEE : &'static str = "├";
const RIGHT_TEE : &'static str = "┤";
const BOTTOM_TEE : &'static str = "┴";
const TOP_TEE : &'static str = "┬";
const VERT : &'static str = "│";

const MAX_COMBATANTS : usize = 32;

enum State {
    Input,
    Target { from: usize, to: usize },
    Name,
}

struct Battle<R: Read, W: Write> {
    stdin: Keys<R>,
    stdout: W,
    state: State,
    combatants: Vec<Combatant>,
    round: u32,
    pos: usize,
    width: u16,
    height: u16,
}

impl<R: Read, W: Write> Battle<R, W> {
    fn new(stdin: R, stdout: W) -> Self {
        Battle {
            stdin: stdin.keys(),
            stdout: stdout,
            state: State::Input,
            combatants: Vec::with_capacity(MAX_COMBATANTS),
            round: 1,
            pos: 0,
            width: format!("{}", Combatant::default()).len() as u16,
            height: MAX_COMBATANTS as u16,
        }
    }

    fn draw(&mut self) {
        // clear the screen
        write!(self.stdout, "{}", clear::All).unwrap();
        // write the top frame
        self.draw_border(true);
            /*
        self.stdout.write(TOP_LEFT.as_bytes()).unwrap();
        self.stdout.write(HORZ.as_bytes()).unwrap();
        let round = format!("R: {:02}", self.round);
        self.stdout.write(round.as_bytes()).unwrap();
        for _ in 0..(self.width - 1 - round.len() as u16) {
            self.stdout.write(HORZ.as_bytes()).unwrap();
        }
        self.stdout.write(TOP_RIGHT.as_bytes()).unwrap();
        */
        self.stdout.write(b"\n\r").unwrap();
        // write combatant name and display combatant info
        for i in 0..self.height {
            self.draw_combatant_row(i as usize);
        }
        // write the bottom frame
        self.draw_border(false);
        // draw prompt box
        self.draw_prompt_box();

        write!(self.stdout, "{}", cursor::Goto(1, 1)).unwrap();
        self.stdout.flush().unwrap();
    }

    fn draw_border(&mut self, top: bool) {
        let v = VERT.chars().next().unwrap();
        let mut def = format!("{}", Combatant::default())
            .replace(|c| c != v, HORZ);
        if top {
            self.stdout.write(TOP_LEFT.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            def = def.replace(v, TOP_TEE);
            def = format!("R: {:02}{}", self.round, def.chars().skip(5).collect::<String>());
            self.stdout.write(def.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            self.stdout.write(TOP_RIGHT.as_bytes()).unwrap();
        } else {
            self.stdout.write(BOTTOM_LEFT.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            def = def.replace(v, BOTTOM_TEE);
            self.stdout.write(def.as_bytes()).unwrap();
            self.stdout.write(HORZ.as_bytes()).unwrap();
            self.stdout.write(BOTTOM_RIGHT.as_bytes()).unwrap();
        }
    }

    fn draw_combatant_row(&mut self, i: usize) {
        self.stdout.write(VERT.as_bytes()).unwrap();
        if i < self.combatants.len() {
            let ref c = self.combatants[i];
            let ctext = format!("{}", c);
            match self.state {
                State::Target { from: f, .. } if f == i => {
                    write!(self.stdout, " {}{}{} ", style::Bold, ctext, style::Reset)
                },
                _ if self.pos == i => {
                    write!(self.stdout, " {}{}{} ", style::Invert, ctext, style::Reset)
                },
                _ => write!(self.stdout, " {} ", c),
            }.unwrap();
            //self.stdout.write(b" ").unwrap();//.repeat(self.width as usize - 14 - ctext.len()).as_bytes()).unwrap();
        } else {
            let c = Combatant::default();
            write!(self.stdout, " {} ", format!("{}", c).replace(|c : char| c != '│', " ")).unwrap();
            //self.stdout.write_all(b" +").unwrap();
            //self.stdout.write_all(" ".repeat(format!("{}", c).len() - 10).as_bytes()).unwrap();
        }
        self.stdout.write(VERT.as_bytes()).unwrap();
        self.stdout.write(b"\n\r").unwrap();
    }

    fn draw_prompt_box(&mut self) {
        //TODO
    }

    /// Start the battle.
    fn start(&mut self) {
        loop {
            // iterator returns an Option<Result<Key, Error>>
            let b = self.stdin.next().unwrap().unwrap();
            use termion::event::Key::*;
            match b {
                Char('\n') => {
                    self.state = State::Target{ from: self.pos, to: self.pos };
                },
                Char('j') => self.down(),
                Char('k') => self.up(),
                Char('q') => return,
                // add combatant
                Char('n') => {
                    self.add_combatant();
                },
                Char('a') => {
                    // make sure from has enough attacks
                    let dam = self.read_line().parse::<i32>();
                    dam.map(|d| self.attack(d)).ok();
                },
                Char('h') => {
                    let heal = self.read_line().parse::<i32>();
                    heal.map(|h| self.heal(h)).ok();
                },
                Char('x') => {
                    self.advance();
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
            .map(|c| (c.init(), c))
            // filter out dead
            .filter(|&(i, _)| i > 0)
            .collect::<Vec<_>>();
        // sort with fastest at the top
        initiatives.sort_by(|a, b| b.0.cmp(&a.0));
        self.combatants = initiatives.into_iter()
            .map(|(_, c)| c)
            .collect::<Vec<_>>();
        // reset pos to 0 to avoid errors
        self.pos = 0;
    }

    fn read_char(&mut self) -> Option<char> {
        write!(self.stdout, "{}> {}", cursor::Goto(1, self.height + 3), cursor::Show).unwrap();
        if let Some(Ok(Key::Char(c))) = self.stdin.next() {
            write!(self.stdout, "{}{}{}", c, cursor::Hide, cursor::Goto(1, 1)).unwrap();
            self.stdout.flush().unwrap();
            Some(c)
        } else {
            None
        }
    }

    fn read_line(&mut self) -> String {
        let mut s = String::from("");
        write!(self.stdout, "{}> {}", cursor::Goto(1, self.height + 3), cursor::Show).unwrap();
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
                _ => (),
            }
        }
        write!(self.stdout, "{}{}", cursor::Hide, cursor::Goto(1, 1)).unwrap();
        self.stdout.flush().unwrap();
        s
    }

    /// Add a combatant to the battle.
    fn add_combatant(&mut self) {
        // receive info from user
        let name = self.read_line();
        let team = 'team: loop {
            let i = self.read_char().and_then(|c| c.to_digit(10));
            match i {
                Some(_) => break i,
                None => (),
            }
        }.unwrap();
        write!(self.stdout, "{}", clear::CurrentLine).unwrap();
        let init = 'init: loop {
            let i = self.read_char().and_then(|c| c.to_digit(10));
            match i {
                Some(_) => break i,
                None => (),
            }
        }.unwrap();
        let hp = self.read_line().parse::<Meter<i32>>().unwrap();
        let atts = self.read_line().parse::<Meter<u32>>().unwrap();
        // lvld: y/n
        // xp: y/n
        let c = Combatant::new(name, team, init, hp, atts, 1, Classes::Single(Class::Monster), false);
        self.combatants.push(c);
        self.sort();
    }

    /// Target an attack from one combatant upon another.
    fn target_combatant(&mut self, att_ix: usize, def_ix: usize) {
        self.state = State::Target { from: att_ix, to: def_ix };
    }

    fn attack(&mut self, dam: i32) {
        if let State::Target { from: f, to: t } = self.state {
            if self.combatants[f].can_attack() {
                self.combatants[f].deal_hit(dam);
                self.combatants[t].recv_hit(dam);
            }
        }
    }

    fn heal(&mut self, dam: i32) {
        if let State::Target { to: t, .. } = self.state {
            self.combatants[t].heal(dam);
        }
    }

    fn down(&mut self) {
        if self.pos + 1 < self.combatants.len() {
            self.pos += 1;
            if let State::Target { to: ref mut t, .. } = self.state {
                *t = self.pos;
            }
        }
    }

    fn up(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            if let State::Target { to: ref mut t, .. } = self.state {
                *t = self.pos;
            }
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
