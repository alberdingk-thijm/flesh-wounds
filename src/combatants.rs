//! Combatant data.

use meters::Meter;
use std::fmt;
use std::str::FromStr;
use std::num::ParseIntError;
use termion::color;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    name: String,
    class: Classes,
    abilities: Option<Abilities>,
    hp: Meter<i32>,
    attacks: Meter<u32>,
    ac: i32,
    thac0: u32,
    status: Status,
    team: Option<u32>,
    init: Option<u32>,
    dealt: i32,
    recvd: i32,
    round: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Classes {
    Multi { name: Vec<Class>, lvl: u32 },
    Single { name: Class, lvl: u32 },
    Monster { magical: bool, hd: u32 },
}

impl Classes {
    // Clerics, druids and monks
    const CLERIC_THAC0 : [u32; 13] = [20, 20, 19, 18, 18, 17, 16, 16, 15, 14, 14, 13, 12];
    // Fighters, paladins, rangers and monsters
    const FIGHTER_THAC0 : [u32; 13] = [20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8];
    // Mages and illusionists
    const MAGE_THAC0 : [u32; 13] = [21, 21, 21, 20, 20, 19, 19, 19, 18, 18, 17, 17, 17];
    // Thieves, assassins and bards
    const THIEF_THAC0 : [u32; 13] = [21, 21, 20, 20, 19, 19, 18, 18, 17, 17, 16, 16, 15];

    /// Return a new Classes variant with the given hd/level.
    pub fn lvl(mut self, lvl: u32) -> Self {
        self = match self {
            Classes::Multi { name: n, .. } => Classes::Multi { name: n.clone(), lvl: lvl },
            Classes::Single { name: n, .. } => Classes::Single { name: n, lvl: lvl },
            Classes::Monster { magical: m, .. } => Classes::Monster { magical: m, hd: lvl },
        };
        self
    }

    /// Return THAC0 associated with the given class and level.
    pub fn thac0(&self) -> u32 {
        match *self {
            Classes::Multi { name: ref v, lvl: l } => {
                let idx = if l <= 1 {
                    0usize
                } else {
                    (l as usize - 1).min(Classes::FIGHTER_THAC0.len() - 1)
                };
                v.iter().map(|c| {
                    match *c {
                        Class::Cleric | Class::Druid | Class::Monk => Classes::CLERIC_THAC0[idx],
                        Class::Fighter | Class::Paladin | Class::Ranger => Classes::FIGHTER_THAC0[idx],
                        Class::Mage | Class::Illusionist => Classes::MAGE_THAC0[idx],
                        Class::Thief | Class::Assassin | Class::Bard => Classes::THIEF_THAC0[idx],
                    }
                }).min().unwrap_or(20)
            },
            Classes::Single { name: c, lvl: l } => {
                let idx = if l <= 1 {
                    0usize
                } else {
                    (l as usize - 1).min(Classes::FIGHTER_THAC0.len() - 1)
                };
                match c {
                    Class::Cleric | Class::Druid | Class::Monk => Classes::CLERIC_THAC0[idx],
                    Class::Fighter | Class::Paladin | Class::Ranger => Classes::FIGHTER_THAC0[idx],
                    Class::Mage | Class::Illusionist => Classes::MAGE_THAC0[idx],
                    Class::Thief | Class::Assassin | Class::Bard => Classes::THIEF_THAC0[idx],
                }
            },
            Classes::Monster { hd: h, .. } => {
                let idx = if h <= 1 {
                    0usize
                } else {
                    (h as usize - 1).min(Classes::FIGHTER_THAC0.len() - 1)
                };
                Classes::FIGHTER_THAC0[idx]
            },
        }
    }
}

#[derive(Debug, Fail)]
pub enum ParseClassError {
    #[fail(display = "Invalid integer value")]
    Int(#[cause] ParseIntError),
    #[fail(display = "Invalid class name")]
    Name,
}

impl From<ParseIntError> for ParseClassError {
    fn from(e: ParseIntError) -> Self {
        ParseClassError::Int(e)
    }
}

impl fmt::Display for Classes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &Classes::Multi { name: ref v, lvl: l} => {
                let names = v.iter().map(|&c| format!("{}", c))
                .collect::<Vec<_>>().join("/");
                format!("{} level {}", l, names)
            },
            &Classes::Single { name: c, lvl: l } => format!("{} level {}", l, c),
            &Classes::Monster { magical: m, hd: h } => {
                format!("{}{}-HD monster", if m { "magical " } else { "" }, h)
            },
        })
    }
}

impl FromStr for Classes {
    type Err = ParseClassError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // detach optional numeric portion
        let (s, n) = s.find(char::is_numeric).and_then(|i| {
            // split, return num and shortened s
            let (s, nums) = s.split_at(i);
            nums.parse::<u32>().map(|n| (s, n)).ok()
        }).unwrap_or((s, 1));
        match s {
            // magical monsters: ![N]
            "!" => Ok(Classes::Monster { magical: true, hd: n }),
            // regular monsters .[N]
            "." => Ok(Classes::Monster { magical: false, hd: n }),
            _ => {
                let classes : Result<Vec<Class>, ParseClassError> = s.split("/")
                    .map(|c| c.parse::<Class>()).collect();
                classes.and_then(|c| if c.len() > 1 {
                    Ok(Classes::Multi { name: c, lvl: n })
                } else if c.len() == 1 {
                    Ok(Classes::Single { name: c[0], lvl: n })
                } else {
                    Err(ParseClassError::Name)
                })
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Class {
    Cleric,
    Druid,
    Fighter,
    Paladin,
    Ranger,
    Mage,
    Illusionist,
    Thief,
    Assassin,
    Monk,
    Bard,
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Class::Cleric  => "cleric",
            Class::Druid  => "druid",
            Class::Fighter  => "fighter",
            Class::Paladin  => "paladin",
            Class::Ranger  => "ranger",
            Class::Mage  => "mage",
            Class::Illusionist  => "illusionist",
            Class::Thief  => "thief",
            Class::Assassin  => "assassin",
            Class::Monk  => "monk",
            Class::Bard  => "bard",
        })
    }
}

impl FromStr for Class {
    type Err = ParseClassError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "cleric" | "c" => Ok(Class::Cleric),
            "druid" | "d" => Ok(Class::Druid),
            "fighter" | "f" => Ok(Class::Fighter),
            "paladin" | "p" => Ok(Class::Paladin),
            "ranger" | "r" => Ok(Class::Ranger),
            "mage" | "ma" => Ok(Class::Mage),
            "illusionist" | "i" => Ok(Class::Illusionist),
            "thief" | "t" => Ok(Class::Thief),
            "assassin" | "a" => Ok(Class::Assassin),
            "monk" | "mo" => Ok(Class::Monk),
            "bard" | "b" => Ok(Class::Bard),
            _ => Err(ParseClassError::Name),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ClassRecord {
    name: Class,
    xp: Abilities,
    multi: Abilities,
    min: Abilities,
    thac0: [u32; 13],
    saves: Saves,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Saves {
    poison: [u32; 20],
    para: [u32; 20],
    poly: [u32; 20],
    rsw: [u32; 20],
    breath: [u32; 20],
    magic: [u32; 20],
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Abilities {
    #[serde(rename = "str")]
    strength: u32,
    #[serde(rename = "int")]
    intelligence: u32,
    #[serde(rename = "wis")]
    wisdom: u32,
    #[serde(rename = "dex")]
    dexterity: u32,
    #[serde(rename = "con")]
    constitution: u32,
    #[serde(rename = "cha")]
    charisma: u32,
}

#[derive(Debug, Clone, Fail)]
pub enum ParseAbilitiesError {
    #[fail(display = "Invalid integer value")]
    Int(#[cause] ParseIntError),
    #[fail(display = "Invalid number of ability fields")]
    NumArgs,
}

impl From<ParseIntError> for ParseAbilitiesError {
    fn from(e: ParseIntError) -> Self {
        ParseAbilitiesError::Int(e)
    }
}

impl FromStr for Abilities {
    type Err = ParseAbilitiesError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let terms : Result<Vec<u32>, ParseAbilitiesError> = s.split('/')
            .map(|s| s.parse::<u32>().map_err(|e| e.into()))
            .collect();
        terms.and_then(|v| {
            if v.len() == 6 {
                Ok(Abilities { strength: v[0], intelligence: v[1], wisdom: v[2],
                    dexterity: v[3], constitution: v[4], charisma: v[5]})
            } else {
                Err(ParseAbilitiesError::NumArgs)
        }})
    }
}

impl fmt::Display for Abilities {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "STR: {}\n\rINT: {}\n\rWIS: {}\n\rDEX: {}\n\rCON: {}\n\rCHA: {}",
               self.strength, self.intelligence, self.wisdom, self.dexterity,
               self.constitution, self.charisma)
    }
}

impl Default for Combatant {
    fn default() -> Self {
        Combatant {
            name: "?".repeat(16),
            team: None,
            init: None,
            hp: "1/1".parse::<Meter<i32>>().unwrap(),
            abilities: None,
            ac: 10,
            thac0: 20,
            status: Status::Healthy,
            attacks: "1/1".parse::<Meter<u32>>().unwrap(),
            class: Classes::Monster { magical: false, hd: 1 },
            dealt: 0,
            recvd: 0,
            round: 0,
        }
    }
}

impl fmt::Display for Combatant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // 4 digits seems like a reasonable limit on hp
        match self.status {
            Status::Healthy => write!(f, "{n:16.16}{sep}{t}{sep}{i}{sep}{hp:>9.9}{sep}{at}{sep}{ac:02}{sep}{th:02}{sep}{st}",
                                      n = self.name,
                                      t = self.team.map(|t| format!("{}", t)).unwrap_or("-".into()),
                                      i = self.init.map(|t| format!("{}", t)).unwrap_or("-".into()),
                                      // apply format so that the padding works correctly
                                      hp = format!("{}", self.hp),
                                      at = self.attacks,
                                      ac = self.ac,
                                      th = self.thac0,
                                      st = self.status,
                                      sep = " │ "),
            Status::Stunned(_) => write!(f, "{col}{n:16.16}{sep}{t}{sep}{i}{sep}{hp:>9.9}{sep}{at}{sep}{ac:02}{sep}{th:02}{sep}{st}{res}",
                                         n = self.name,
                                         t = self.team.map(|t| format!("{}", t)).unwrap_or("-".into()),
                                         i = self.init.map(|t| format!("{}", t)).unwrap_or("-".into()),
                                         // apply format so that the padding works correctly
                                         hp = format!("{}", self.hp),
                                         at = self.attacks,
                                         ac = self.ac,
                                         th = self.thac0,
                                         st = self.status,
                                         sep = " │ ",
                                         col = color::Fg(color::Yellow),
                                         res = color::Fg(color::Reset)),
            Status::Dead => write!(f, "{col}{n:16.16}{sep}{t}{sep}{i}{sep}{hp:>9.9}{sep}{at}{sep}{ac:02}{sep}{th:02}{sep}{st}{res}",
                                   n = self.name,
                                   t = self.team.map(|t| format!("{}", t)).unwrap_or("-".into()),
                                   i = self.init.map(|t| format!("{}", t)).unwrap_or("-".into()),
                                   // apply format so that the padding works correctly
                                   hp = format!("{}", self.hp),
                                   at = self.attacks,
                                   ac = self.ac,
                                   th = self.thac0,
                                   st = self.status,
                                   sep = " │ ",
                                   col = color::Fg(color::Red),
                                   res = color::Fg(color::Reset)),
        }
    }
}

impl Combatant {
    const LVLD_DEAD : i32 = -10;
    const UNLVLD_DEAD : i32 = -4;
    /// Modifier specifying total possible range of base init values.
    const INIT_MOD : u32 = 12;

    pub fn new<S: Into<String>>(name: S, hp: Meter<i32>, attacks: Meter<u32>, classes: Classes, ac: i32) -> Self {
        Combatant {
            name: name.into(),
            team: None,
            init: None,
            hp: hp,
            status: Status::Healthy,
            abilities: None,
            ac: ac,
            thac0: classes.thac0(),
            attacks: attacks,
            class: classes,
            dealt: 0,
            recvd: 0,
            round: 1,
        }
    }

    pub fn rename<S: Into<String>>(&mut self, name: S) {
        self.name = name.into();
    }

    pub fn team(&mut self, team: Option<u32>) {
        self.team = team;
    }

    pub fn init(&mut self, init: Option<u32>) {
        self.init = init;
    }

    pub fn abilities(&mut self, abilities: Option<Abilities>) {
        self.abilities = abilities;
    }

    pub fn update(&mut self) {
        self.round += 1;
        if let Status::Stunned(_) = self.status {
            // revert to healthy
            self.status = Status::Healthy;
        }
        // refill attacks
        self.attacks += self.attacks.max();
    }

    /// Calculate initiative relative to base initiative and current state.
    pub fn get_init(&self) -> Option<u32> {
        self.init.map(|i| match self.status {
            Status::Healthy => i + Combatant::INIT_MOD * 2,
            Status::Stunned(x) => i + Combatant::INIT_MOD - x,
            Status::Dead => 0,
        })
    }

    fn dead(&self) -> i32 {
        match self.class {
            Classes::Monster { .. } => Combatant::UNLVLD_DEAD,
            _ => Combatant::LVLD_DEAD,
        }
    }

    /// Return true if considered "in combat".
    /// Equivalent to having a team and initiative set.
    pub fn in_combat(&self) -> bool {
        self.init.is_some() && self.team.is_some()
    }

    /// Return true if able to attack.
    /// Must have attacks to spend.
    pub fn can_attack(&self) -> bool {
        self.attacks.curr() >= 1
    }

    /// Add to xp earnings for dealing a hit.
    pub fn deal_hit(&mut self, dam: i32) {
        self.dealt += dam;
        self.attacks -= 1;
        // TODO: missing some way of allowing for 1 extra hit every X rounds
    }

    /// Damage self.
    pub fn recv_hit(&mut self, dam: i32) {
        self.recvd += dam;
        self.status = match self.status {
            Status::Healthy | Status::Stunned(_) if (self.hp.curr() - dam <= self.dead()) => Status::Dead,
            // if the current stun is bigger, retain it
            s @ Status::Healthy | s @ Status::Stunned(_) => {
                let new = Status::stun_lock(dam, self.hp.curr());
                if new > s {
                    // decrement attacks available on a new greater stun
                    if let Status::Stunned(x) = new {
                        self.attacks -= x.min(self.attacks.curr());
                    }
                    new
                } else {
                    s
                }
            },
            s @ _ => s,
        };
        self.hp -= dam;
    }

    /// Heal self.
    pub fn heal(&mut self, dam: i32) {
        self.hp += dam;
    }

    /// Calculate xp earned.
    pub fn xp(&self, team_bonus: i32) -> i32 {
        // FIXME: change false to xp bonus calc
        ((self.dealt * 10 + self.recvd * 20 + team_bonus) as f64 
            * if false { 1.1 } else { 1.0 }) as i32
    }

    /// Return a detailed description of the Combatant's features.
    pub fn describe(&self) -> String {
        format!("{}, {}\n\r{}", self.name, self.class,
                self.abilities.map(|a| format!("{}", a)).unwrap_or("".into()))
    }
}

/// The status of the participant.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Healthy,
    Stunned(u32),
    Dead,
}

impl Default for Status {
    fn default() -> Self {
        Status::Healthy
    }
}

impl Status {
    /// Calculate stun lock effect based on damage versus hp.
    fn stun_lock(dam: i32, hp: i32) -> Self {
        if dam * 7 >= hp * 6 {
            Status::Stunned(8)
        } else if dam * 6 >= hp * 5 {
            Status::Stunned(7)
        } else if dam * 5 >= hp * 4 {
            Status::Stunned(6)
        } else if dam * 4 >= hp * 3 {
            Status::Stunned(5)
        } else if dam * 3 >= hp * 2 {
            Status::Stunned(4)
        } else if dam * 2 >= hp {
            Status::Stunned(3)
        } else if dam * 3 >= hp {
            Status::Stunned(2)
        } else if dam * 4 >= hp {
            Status::Stunned(1)
        } else {
            Status::Healthy
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Status::Dead => "#",
            Status::Stunned(_) => "*",
            Status::Healthy => "+",
        })
    }
}
