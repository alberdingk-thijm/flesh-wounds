//! Combatant data.

use meters::Meter;
use std::fmt;

/// A participant in the battle.
#[derive(Debug, Clone)]
pub struct Combatant {
    pub name: String,
    pub team: u32,
    pub init: u32,
    pub hp: Meter<i32>,
    pub status: Status,
    pub attacks: Meter<u32>,
    pub hd: u32,
    pub class: Classes,
    dealt: i32,
    recvd: i32,
    xp_bonus: bool,
    round: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Classes {
    Multi(Vec<Class>),
    Single(Class),
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
    Monster,
}

impl Default for Combatant {
    fn default() -> Self {
        Combatant {
            name: "?".repeat(16),
            team: 0,
            init: 0,
            hp: "0/0".parse::<Meter<i32>>().unwrap(),
            status: Status::Healthy,
            attacks: "0/0".parse::<Meter<u32>>().unwrap(),
            hd: 0,
            class: Classes::Single(Class::Monster),
            dealt: 0,
            recvd: 0,
            xp_bonus: false,
            round: 0,
        }
    }
}

impl fmt::Display for Combatant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // 4 digits seems like a reasonable limit on hp
        write!(f, "{:16.16}{sep}{}{sep}{}{sep}{:>9.9}{sep}{}{sep}{}",
               self.name,
               self.team,
               self.init,
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

    pub fn new<S: Into<String>>(name: S, team: u32, init: u32, hp: Meter<i32>, attacks: Meter<u32>, hd: u32, classes: Classes, xp: bool) -> Self {
        Combatant {
            name: name.into(),
            team: team,
            init: init,
            hp: hp,
            status: Status::Healthy,
            attacks: attacks,
            hd: hd,
            class: classes,
            dealt: 0,
            recvd: 0,
            xp_bonus: xp,
            round: 1,
        }
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
    pub fn init(&self) -> u32 {
        match self.status {
            Status::Healthy => self.init + Combatant::INIT_MOD * 2,
            Status::Stunned(x) => self.init + Combatant::INIT_MOD - x,
            Status::Dead => self.init,
        }
    }

    fn min_hp(&self) -> i32 {
        if self.class != Classes::Single(Class::Monster) {
            Combatant::LVLD_DEAD
        } else {
            Combatant::UNLVLD_DEAD
        }
    }

    /// Return true if able to attack.
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
        ((self.dealt * 10 + self.recvd * 20 + team_bonus) as f64 
            * if self.xp_bonus { 1.1 } else { 1.0 }) as i32
    }
}

/// The status of the participant.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Healthy,
    Stunned(u32),
    Dead,
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