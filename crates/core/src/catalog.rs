// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Game reference data (`data/catalog.json`), shared by both editions.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub const CATALOG_JSON: &str = include_str!("../../../data/catalog.json");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Names {
    pub en: String,
    pub ru: String,
    #[serde(default)]
    pub ru_official: bool,
}

impl Names {
    /// Russian only when an official translation exists.
    pub fn get(&self, russian: bool) -> &str {
        if russian && self.ru_official {
            &self.ru
        } else {
            &self.en
        }
    }

    pub fn matches(&self, needle_lower: &str) -> bool {
        self.en.to_lowercase().contains(needle_lower)
            || self.ru.to_lowercase().contains(needle_lower)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Weapon {
    pub id: String,
    pub code: String,
    #[serde(default)]
    pub recipe: bool,
    pub rarity: String,
    pub damage_min: i64,
    pub damage_max: i64,
    pub weapon_type: i64,
    pub hidden: bool,
    pub name: Names,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outfit {
    pub id: String,
    pub code: String,
    #[serde(default)]
    pub recipe: bool,
    pub rarity: String,
    pub special: [i64; 7],
    pub category: i64,
    pub hidden: bool,
    pub name: Names,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Simple {
    pub id: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub rarity: Option<String>,
    pub name: Names,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pet {
    pub id: String,
    pub code: String,
    pub breed: String,
    pub breed_code: i64,
    pub pet_type: i64,
    pub rarity: String,
    pub bonus: String,
    pub bonus_min: f64,
    pub bonus_max: f64,
    pub name: Names,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    #[serde(default)]
    pub recipe: bool,
    pub room_type: i64,
    pub theme_type: i64,
    pub name: Names,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    #[serde(rename = "type")]
    pub kind: String,
    pub class: String,
    pub max_merge: i64,
    pub levels: i64,
    pub name: Vec<Names>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UniqueDweller {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub gender: i64,
    pub outfit: String,
    pub weapon: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub id: String,
    pub steam_id: String,
    pub name: Names,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub dweller_level_max: i64,
    pub special_min: i64,
    pub special_max: i64,
    pub happiness_min: i64,
    pub happiness_max: i64,
    pub health_base: f64,
    pub health_level_base: f64,
    pub health_endurance_factor: f64,
    pub health_initial_endurance_factor: f64,
    pub room_level_max: i64,
    pub merge_level_max: i64,
    pub vault_theme_max: i64,
    pub lunchbox_type_max: i64,
    pub grid_rows: i64,
    pub grid_cols: i64,
    pub max_pets: i64,
    pub base_inventory: i64,
    pub serialize_id_max: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiNames {
    pub special: BTreeMap<String, Names>,
    pub rarity: BTreeMap<String, Names>,
    pub resources: BTreeMap<String, Names>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub catalog_version: String,
    pub game_version: String,
    pub weapons: Vec<Weapon>,
    pub outfits: Vec<Outfit>,
    pub junk: Vec<Simple>,
    pub themes: Vec<Theme>,
    pub event_themes: Vec<Simple>,
    pub decorations: Vec<Simple>,
    pub pets: Vec<Pet>,
    pub pet_bonuses: BTreeMap<String, Names>,
    pub rooms: Vec<Room>,
    pub unique_dwellers: Vec<UniqueDweller>,
    pub quests: Vec<Simple>,
    pub objectives: Vec<Simple>,
    pub unlockables: Vec<Simple>,
    pub achievements: Vec<Achievement>,
    pub level_experience: Vec<i64>,
    pub limits: Limits,
    pub names: UiNames,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemKind {
    Weapon,
    Outfit,
    Junk,
    Pet,
}

impl ItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemKind::Weapon => "Weapon",
            ItemKind::Outfit => "Outfit",
            ItemKind::Junk => "Junk",
            ItemKind::Pet => "Pet",
        }
    }

    pub fn parse(s: &str) -> Option<ItemKind> {
        match s {
            "Weapon" => Some(ItemKind::Weapon),
            "Outfit" => Some(ItemKind::Outfit),
            "Junk" => Some(ItemKind::Junk),
            "Pet" => Some(ItemKind::Pet),
            _ => None,
        }
    }

    pub const ALL: [ItemKind; 4] = [
        ItemKind::Weapon,
        ItemKind::Outfit,
        ItemKind::Junk,
        ItemKind::Pet,
    ];
}

/// One entry of the item picker.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInfo<'a> {
    pub kind: ItemKind,
    pub id: &'a str,
    pub rarity: &'a str,
    pub name: &'a Names,
}

pub fn catalog() -> &'static Catalog {
    static CAT: OnceLock<Catalog> = OnceLock::new();
    CAT.get_or_init(|| serde_json::from_str(CATALOG_JSON).expect("embedded catalog.json is valid"))
}

impl Catalog {
    pub fn item(&self, kind: ItemKind, id: &str) -> Option<ItemInfo<'_>> {
        self.items(kind).find(|i| i.id == id)
    }

    pub fn items(&self, kind: ItemKind) -> Box<dyn Iterator<Item = ItemInfo<'_>> + '_> {
        match kind {
            ItemKind::Weapon => Box::new(self.weapons.iter().map(move |w| ItemInfo {
                kind,
                id: &w.id,
                rarity: &w.rarity,
                name: &w.name,
            })),
            ItemKind::Outfit => Box::new(self.outfits.iter().map(move |w| ItemInfo {
                kind,
                id: &w.id,
                rarity: &w.rarity,
                name: &w.name,
            })),
            ItemKind::Junk => Box::new(self.junk.iter().map(move |w| ItemInfo {
                kind,
                id: &w.id,
                rarity: w.rarity.as_deref().unwrap_or("Normal"),
                name: &w.name,
            })),
            ItemKind::Pet => Box::new(self.pets.iter().map(move |w| ItemInfo {
                kind,
                id: &w.id,
                rarity: &w.rarity,
                name: &w.name,
            })),
        }
    }

    /// Search by English name, Russian name or id, whatever the display setting.
    pub fn search(&self, kinds: &[ItemKind], query: &str) -> Vec<ItemInfo<'_>> {
        let q = query.trim().to_lowercase();
        kinds
            .iter()
            .flat_map(|k| self.items(*k))
            .filter(|i| q.is_empty() || i.id.to_lowercase().contains(&q) || i.name.matches(&q))
            .collect()
    }

    pub fn weapon(&self, id: &str) -> Option<&Weapon> {
        self.weapons.iter().find(|w| w.id == id)
    }

    pub fn outfit(&self, id: &str) -> Option<&Outfit> {
        self.outfits.iter().find(|w| w.id == id)
    }

    pub fn pet(&self, id: &str) -> Option<&Pet> {
        self.pets.iter().find(|w| w.id == id)
    }

    pub fn room(&self, kind: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.kind == kind)
    }

    /// Experience needed to reach `level` (1-based).
    pub fn level_xp(&self, level: i64) -> i64 {
        let i = (level.clamp(1, self.level_experience.len() as i64) - 1) as usize;
        self.level_experience.get(i).copied().unwrap_or(0)
    }

    /// Max health of a new level 1 dweller.
    pub fn initial_max_health(&self, endurance: i64) -> f64 {
        self.limits.health_base + self.limits.health_initial_endurance_factor * endurance as f64
    }

    /// Max health the game adds per level gained at the given Endurance.
    pub fn health_per_level(&self, endurance: i64) -> f64 {
        self.limits.health_level_base + self.limits.health_endurance_factor * endurance as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_finds() {
        let c = catalog();
        assert!(c.weapons.len() > 200);
        assert!(c.item(ItemKind::Weapon, "HuntingRifle_Enhanced").is_some());
        assert!(!c.search(&[ItemKind::Weapon], "охотничье").is_empty());
        assert_eq!(c.limits.dweller_level_max, 50);
        assert_eq!(c.initial_max_health(1), 105.0);
        assert_eq!(c.health_per_level(10), 7.5);
    }
}
