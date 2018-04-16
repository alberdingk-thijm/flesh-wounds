use combatants::{Combatant, Classes};

type CombLoaders = Vec<CombLoader>;

#[derive(Serialize, Deserialize)]
pub struct CombLoader {
    name: String,
    #[serde(rename = "level/hd")]
    level_hd: u32,
    class: Classes,
    abilities: Option<Abilities>,
    hp: String,
    ac: u32,
}

impl From<Combatant> for CombLoader {
    fn from(from: Combatant) -> Self {
        CombLoader {
            name: from.name,
            level_hd: from.hd,
            class: from.class,
            abilities: None,
            hp: format!("{}", from.hp),
            ac: 10,
        }
    }
}

#[derive(Serialize, Deserialize)]
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

pub fn load_combs() {
}
