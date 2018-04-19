//! Combatant data.

use meters::Meter;
use std::fmt;
use std::str::FromStr;
use strum::ParseError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    name: String,
    #[serde(rename = "level/hd")]
    level_hd: u32,
    class: Classes,
    abilities: Option<Abilities>,
    hp: Meter<i32>,
    attacks: Meter<u32>,
    ac: i32,
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
    Multi(Vec<Class>),
    Single(Class),
}

#[derive(Debug, Fail)]
#[fail(display = "Invalid class name")]
pub struct ClassParseError;

impl FromStr for Classes {
    type Err = ClassParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let classes : Result<Vec<Class>, ParseError> = s.split("/")
            .map(|c| c.parse::<Class>()).collect();
        classes.and_then(|c| if c.len() > 1 {
            Ok(Classes::Multi(c))
        } else {
            Ok(Classes::Single(c[0]))
        }).or_else(|_| Err(ClassParseError))
    }
}

#[derive(EnumString, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Class {
    #[strum(serialize = "c")]
    Cleric,
    #[strum(serialize = "d")]
    Druid,
    #[strum(serialize = "f")]
    Fighter,
    #[strum(serialize = "p")]
    Paladin,
    #[strum(serialize = "r")]
    Ranger,
    #[strum(serialize = "ma")]
    Mage,
    #[strum(serialize = "i")]
    Illusionist,
    #[strum(serialize = "t")]
    Thief,
    #[strum(serialize = "a")]
    Assassin,
    #[strum(serialize = "mo")]
    Monk,
    #[strum(serialize = "b")]
    Bard,
    #[strum(serialize = "-")]
    Monster,
    #[strum(serialize = "_")]
    MagicMonster,
}

// impl FromStr for Class {
//     type Err = ClassParseError;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s.to_lowercase().as_ref() {
//             "cleric" | "c" => Ok(Class::Cleric),
//             "druid" | "d" => Ok(Class::Druid),
//             "fighter" | "f" => Ok(Class::Fighter),
//             "paladin" | "p" => Ok(Class::Paladin),
//             "ranger" | "r" => Ok(Class::Ranger),
//             "mage" | "ma" => Ok(Class::Mage),
//             "illusionist" | "i" => Ok(Class::Illusionist),
//             "thief" | "t" => Ok(Class::Thief),
//             "assassin" | "a" => Ok(Class::Assassin),
//             "monk" | "mo" => Ok(Class::Monk),
//             "bard" | "b" => Ok(Class::Bard),
//             "monster" | "-" => Ok(Class::Monster),
//             "magicmonster" | "_" => Ok(Class::MagicMonster),
//             _ => Err(ClassParseError(s.into())),
//         }
//     }
// }

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ClassRefs {
    xp: Abilities,
    multi: Abilities,
    min: Abilities,
    thac0: Abilities,
    saves: Saves,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Saves {
    poisdeath: Abilities,
    parapet: Abilities,
    poly: Abilities,
    rsw: Abilities,
    breath: Abilities,
    magic: Abilities,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
    constituion: u32,
    #[serde(rename = "cha")]
    charisma: u32,
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
            status: Status::Healthy,
            attacks: "1/1".parse::<Meter<u32>>().unwrap(),
            level_hd: 1,
            class: Classes::Single(Class::Monster),
            dealt: 0,
            recvd: 0,
            round: 0,
        }
    }
}

impl fmt::Display for Combatant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // 4 digits seems like a reasonable limit on hp
        write!(f, "{:16.16}{sep}{}{sep}{}{sep}{:>9.9}{sep}{}{sep}{}",
               self.name,
               self.team.map(|t| format!("{}", t)).unwrap_or("-".into()),
               self.init.map(|t| format!("{}", t)).unwrap_or("-".into()),
               // apply format so that the padding works correctly
               format!("{}", self.hp),
               self.attacks,
               self.status,
               sep = " │ ")
    }
}

impl Combatant {
    const LVLD_DEAD : i32 = -10;
    const UNLVLD_DEAD : i32 = -4;
    /// Modifier specifying total possible range of base init values.
    const INIT_MOD : u32 = 12;

    pub fn new<S: Into<String>>(name: S, hp: Meter<i32>, attacks: Meter<u32>, hd: u32, classes: Classes) -> Self {
        Combatant {
            name: name.into(),
            team: None,
            init: None,
            hp: hp,
            status: Status::Healthy,
            abilities: None,
            ac: 10,
            attacks: attacks,
            level_hd: hd,
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

    fn min_hp(&self) -> i32 {
        if self.class != Classes::Single(Class::Monster) {
            Combatant::LVLD_DEAD
        } else {
            Combatant::UNLVLD_DEAD
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
            Status::Healthy | Status::Stunned(_) if (self.hp.curr() - dam < self.min_hp()) => Status::Dead,
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
