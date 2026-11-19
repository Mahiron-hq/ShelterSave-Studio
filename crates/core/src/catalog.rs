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
use std::sync::{Arc, OnceLock, RwLock};

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

/// What crafting an item costs in the workshop.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Craft {
    pub caps: i64,
    /// Junk id and count.
    pub junk: Vec<(String, i64)>,
    /// Base crafting time in seconds; the save counts progress in these seconds.
    #[serde(default)]
    pub time: f64,
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
    #[serde(default)]
    pub desc: Option<Names>,
    #[serde(default)]
    pub sell: i64,
    #[serde(default)]
    pub craft: Option<Craft>,
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
    #[serde(default)]
    pub desc: Option<Names>,
    #[serde(default)]
    pub sell: i64,
    #[serde(default)]
    pub craft: Option<Craft>,
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
    #[serde(default)]
    pub desc: Option<Names>,
    #[serde(default)]
    pub sell: i64,
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
    #[serde(default)]
    pub desc: Option<Names>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    #[serde(default)]
    pub recipe: bool,
    pub room_type: i64,
    pub theme_type: i64,
    /// Room type name used as the key of `specialTheme.themeByRoomType`.
    #[serde(default)]
    pub room: String,
    /// Theme name used as the value of `specialTheme.themeByRoomType`.
    #[serde(default)]
    pub theme: String,
    pub name: Names,
    #[serde(default)]
    pub craft: Option<Craft>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    #[serde(rename = "type")]
    pub kind: String,
    pub class: String,
    pub max_merge: i64,
    /// Narrowest size the game builds; workshops only come in full size.
    #[serde(default = "one")]
    pub min_merge: i64,
    pub levels: i64,
    /// Upgrade levels the game has for each merge level (`[merge-1]`, 0 = no such size).
    #[serde(default)]
    pub levels_by_merge: Vec<i64>,
    pub name: Vec<Names>,
    /// Base durability before level and merge multipliers.
    #[serde(default)]
    pub health: f64,
    /// Blueprint objective that unlocks the room; none for rooms available from the start.
    #[serde(default)]
    pub unlock: Option<String>,
    /// Production batch per merge level and room level (`[merge-1][level-1]`).
    #[serde(default)]
    pub reserve: Vec<Vec<BTreeMap<String, f64>>>,
}

fn one() -> i64 {
    1
}

fn mr_handy_health() -> i64 {
    5000
}

impl Room {
    /// Upgrade levels for a merge level; 0 when the game has no room of that size.
    pub fn max_level(&self, merge: i64) -> i64 {
        if merge < self.min_merge || merge > self.max_merge {
            return 0;
        }
        match usize::try_from(merge - 1)
            .ok()
            .and_then(|i| self.levels_by_merge.get(i))
        {
            Some(n) => *n,
            None => self.levels,
        }
    }

    /// Merge levels the game can show.
    pub fn merges(&self) -> Vec<i64> {
        (1..=3).filter(|m| self.max_level(*m) > 0).collect()
    }

    /// Full durability of the room, as the game computes it.
    pub fn max_health(&self, merge: i64, level: i64) -> f64 {
        let l = &catalog().limits;
        let pick = |v: &[f64], i: i64| {
            usize::try_from(i - 1)
                .ok()
                .and_then(|i| v.get(i))
                .copied()
                .unwrap_or(0.0)
        };
        self.health * pick(&l.room_health_per_level, level) * pick(&l.room_health_per_merge, merge)
    }

    /// Resources a full batch holds, without the caps bonus.
    pub fn batch(&self, merge: i64, level: i64) -> Option<&BTreeMap<String, f64>> {
        let m = usize::try_from(merge.clamp(1, 3) - 1).ok()?;
        let l = usize::try_from(level.clamp(1, 3) - 1).ok()?;
        self.reserve.get(m)?.get(l)
    }
}

/// Holiday theme; the game uses it only between `start` and `end` ([month, day]).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTheme {
    pub id: String,
    pub name: Names,
    #[serde(default)]
    pub start: [u32; 2],
    #[serde(default)]
    pub end: [u32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unlockable {
    pub id: String,
    pub name: Names,
    /// Dwellers needed in the vault.
    #[serde(default)]
    pub goal: i64,
    /// Only in seasonal vaults.
    #[serde(default)]
    pub season: bool,
    #[serde(default)]
    pub room: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UniqueDweller {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    /// Save convention: 1 female, 2 male.
    pub gender: i64,
    pub outfit: String,
    pub weapon: String,
    #[serde(default)]
    pub rarity: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub random_name: bool,
    #[serde(default)]
    pub random_body: bool,
    #[serde(default)]
    pub special: Vec<i64>,
    #[serde(default)]
    pub hair: Option<String>,
    #[serde(default)]
    pub face_mask: Option<String>,
    #[serde(default)]
    pub skin_color: Option<u32>,
    #[serde(default)]
    pub hair_color: Option<u32>,
}

impl UniqueDweller {
    /// Listed in the game journal (survival guide).
    pub fn in_journal(&self) -> bool {
        self.rarity == "Legendary" && !self.hidden
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub id: String,
    /// Id used in the save (`vault.Achievements`); can differ from `id` and can be empty.
    #[serde(default)]
    pub objective_id: String,
    pub steam_id: String,
    pub name: Names,
    /// Count the requirement needs; 0 when it is a condition rather than a count.
    #[serde(default)]
    pub goal: i64,
}

/// Extra description parameter (`{1}`) of an objective.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalExtra {
    #[serde(default)]
    pub text: Option<Names>,
    /// Enemy type; null inside `Some` means random (stored in the save).
    #[serde(default, deserialize_with = "some_or_null")]
    pub enemy: Option<Option<String>>,
    #[serde(default, deserialize_with = "some_or_null")]
    pub stat: Option<Option<String>>,
    #[serde(default)]
    pub weapon: bool,
}

fn some_or_null<'de, D>(d: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(d)?))
}

/// Goal of an objective: `base + step * level`, level capped at `max_level`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub class: String,
    pub requirement_id: String,
    pub base: i64,
    pub step: i64,
    pub max_level: Option<i64>,
    #[serde(default)]
    pub extra: Vec<GoalExtra>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Objective {
    pub id: String,
    pub name: Names,
    pub plural: Option<Names>,
    pub level: i64,
    pub tutorial: bool,
    pub normal: bool,
    pub survival: bool,
    pub season: bool,
    pub reward: String,
    pub reward_amount: i64,
    pub reward_step: i64,
    pub goal: Option<Goal>,
}

impl Objective {
    /// Level actually used by the game for a saved `incrementLevel`.
    pub fn level_used(&self, increment_level: i64) -> i64 {
        let lvl = increment_level.max(0);
        match self.goal.as_ref().and_then(|g| g.max_level) {
            Some(m) if self.goal.as_ref().is_some_and(|g| g.step != 0) => lvl.min(m),
            _ => lvl,
        }
    }

    pub fn goal_value(&self, increment_level: i64) -> i64 {
        let lvl = self.level_used(increment_level);
        self.goal.as_ref().map_or(1, |g| g.base + g.step * lvl)
    }

    pub fn reward_value(&self, increment_level: i64) -> i64 {
        self.reward_amount + self.reward_step * self.level_used(increment_level)
    }

    /// Description like the game shows it. `params` fills `{1}`, `{2}` and so on.
    pub fn text(&self, russian: bool, increment_level: i64, params: &[String]) -> String {
        let goal = self.goal_value(increment_level);
        let tpl = match (&self.plural, goal >= 2) {
            (Some(p), true) => p.get(russian),
            _ => self.name.get(russian),
        };
        let mut out = tpl.replace("{0}", &goal.to_string());
        for (i, p) in params.iter().enumerate() {
            out = out.replace(&format!("{{{}}}", i + 1), p);
        }
        out
    }
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
    #[serde(default)]
    pub room_health_per_level: Vec<f64>,
    #[serde(default)]
    pub room_health_per_merge: Vec<f64>,
    /// Share of the room durability an incident starts with.
    #[serde(default)]
    pub room_damage_min: f64,
    #[serde(default)]
    pub room_damage_max: f64,
    /// Items a wasteland team can carry (recipes not counted).
    #[serde(default)]
    pub team_items_max: i64,
    #[serde(default = "mr_handy_health")]
    pub mr_handy_health: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiNames {
    pub special: BTreeMap<String, Names>,
    pub rarity: BTreeMap<String, Names>,
    pub resources: BTreeMap<String, Names>,
    #[serde(default)]
    pub enemies: BTreeMap<String, EnemyNames>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyNames {
    pub one: Names,
    pub many: Names,
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
    pub event_themes: Vec<EventTheme>,
    pub decorations: Vec<Simple>,
    pub pets: Vec<Pet>,
    pub pet_bonuses: BTreeMap<String, Names>,
    pub rooms: Vec<Room>,
    pub unique_dwellers: Vec<UniqueDweller>,
    pub quests: Vec<Simple>,
    pub objectives: Vec<Objective>,
    pub unlockables: Vec<Unlockable>,
    pub achievements: Vec<Achievement>,
    pub level_experience: Vec<i64>,
    pub limits: Limits,
    pub names: UiNames,
    /// Where the wasteland journal phrases lie in the game file.
    #[serde(default)]
    pub wasteland_texts: Option<WastelandTexts>,
    /// Hair, beard and face mask names per gender ("male", "female"), in game order.
    #[serde(default)]
    pub looks: BTreeMap<String, Looks>,
    /// Where the game keeps its pictures; the pictures themselves are read from the installed game.
    #[serde(default)]
    pub art: Option<Art>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Looks {
    pub hairs: Vec<String>,
    pub beards: Vec<String>,
    pub masks: Vec<String>,
    /// Colours the game gives new dwellers and the barber shop offers (ARGB).
    #[serde(default)]
    pub skin_colors: Vec<u32>,
    #[serde(default)]
    pub hair_colors: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Art {
    /// Bundle file inside `FalloutShelter_Data`.
    pub bundle: String,
    pub textures: BTreeMap<String, ArtTexture>,
    /// `"Kind:id"` -> atlas name, x, y (from the top), width, height.
    pub icons: BTreeMap<String, (String, u32, u32, u32, u32)>,
    pub dweller: BTreeMap<String, ArtDweller>,
    /// Happiness above the first number shows the face named second.
    pub face_by_happiness: Vec<(f64, String)>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtTexture {
    /// Bundle node holding the pixels, and the position inside it.
    pub node: String,
    pub offset: u64,
    pub size: u64,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub crc: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtPiece {
    #[serde(default)]
    pub name: String,
    pub tex: String,
    pub offset: [f32; 2],
    #[serde(default)]
    pub scale: [f32; 2],
    #[serde(default)]
    pub hair_color: bool,
    #[serde(default)]
    pub skin_color: bool,
    #[serde(default)]
    pub custom_skin: Option<[f32; 3]>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtMesh {
    pub node: String,
    pub vertex_offset: u64,
    pub vertex_size: u64,
    pub vertex_crc: u32,
    pub vertex_count: usize,
    pub uv_start: usize,
    pub bone_start: usize,
    pub index_offset: u64,
    pub index_size: u64,
    pub index_crc: u32,
    pub arm_joints: BTreeMap<String, [f32; 2]>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtDweller {
    pub body: ArtPiece,
    pub hands: ArtPiece,
    pub faces: BTreeMap<String, ArtPiece>,
    pub hairs: BTreeMap<String, ArtPiece>,
    pub face_masks: BTreeMap<String, ArtPiece>,
    pub outfit: ArtPiece,
    pub hand_offset: [f32; 2],
    pub face_offset: [f32; 2],
    pub face_clip: [f32; 4],
    pub hand_clip: [f32; 4],
    pub mesh: ArtMesh,
    pub arms: Vec<Vec<usize>>,
    /// Body used under outfits with a skirt.
    #[serde(default)]
    pub skirt_body: Option<ArtPiece>,
    /// Outfit pieces by outfit id; `__pregnant` is what pregnant dwellers wear.
    #[serde(default)]
    pub outfits: BTreeMap<String, ArtOutfit>,
    /// Helmets a dweller can have of their own (save keys `helmet`, `helmetCoverCustomization`).
    #[serde(default)]
    pub helmets: BTreeMap<String, ArtHelmet>,
    /// Colouring mask for outfits that have none of their own.
    #[serde(default)]
    pub burn_mask: Option<String>,
}

/// One outfit as the dweller shader draws it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtOutfit {
    pub tex: String,
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    /// Colours the game allows (ARGB); the save's `outfitColor` is snapped to the nearest.
    #[serde(default)]
    pub colors: Vec<u32>,
    #[serde(default)]
    pub skirt: bool,
    /// Drawn by the game as a separate mesh; the preview leaves it out.
    #[serde(default)]
    pub large_headgear: bool,
    #[serde(default)]
    pub coloring_mask: Option<ArtPiece>,
    #[serde(default)]
    pub glove: Option<ArtPiece>,
    #[serde(default = "yes")]
    pub glove_hands: bool,
    #[serde(default)]
    pub helmet: Option<ArtHelmet>,
}

fn yes() -> bool {
    true
}

impl ArtOutfit {
    /// The colour the game uses: the nearest allowed one, or white when there is none.
    pub fn game_color(&self, argb: u32) -> u32 {
        let ch = |c: u32, s: u32| f64::from((c >> s) & 255) / 255.0;
        let dist = |c: u32| -> f64 {
            [24, 16, 8, 0]
                .iter()
                .map(|s| (ch(c, *s) - ch(argb, *s)).powi(2))
                .sum()
        };
        self.colors
            .iter()
            .copied()
            .min_by(|a, b| dist(*a).total_cmp(&dist(*b)))
            .unwrap_or(0xFFFF_FFFF)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtHelmet {
    pub tex: String,
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    /// Hides the hair.
    #[serde(default)]
    pub exclusive: bool,
    /// Hides the face mask.
    #[serde(default)]
    pub no_mask: bool,
    /// The game takes it off inside the vault.
    #[serde(default)]
    pub remove_in_vault: bool,
    /// Part of the hair the helmet covers.
    #[serde(default)]
    pub mask: Option<ArtPiece>,
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

/// Positions of the wasteland journal phrases in the game file; the phrases
/// themselves are read from the player's game.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WastelandTexts {
    /// Bundle node that holds the phrases.
    pub node: String,
    /// CRC-32 of all phrases in order (texts, then keys; English then Russian).
    pub crc: u32,
    /// By the short id the save stores (`{0}` = parameter): English offset,
    /// length, Russian offset, length.
    pub texts: BTreeMap<String, [u64; 4]>,
    /// Lines stored by their localization key.
    pub keys: BTreeMap<String, [u64; 4]>,
}

/// Wasteland journal phrases read from the game.
#[derive(Debug, Default)]
pub struct GameTexts {
    pub texts: BTreeMap<String, Names>,
    pub keys: BTreeMap<String, Names>,
}

impl WastelandTexts {
    /// Offset and size of the part of the node that holds every phrase.
    pub fn span(&self) -> Option<(u64, u64)> {
        let mut lo = u64::MAX;
        let mut hi = 0;
        for r in self.texts.values().chain(self.keys.values()) {
            lo = lo.min(r[0]).min(r[2]);
            hi = hi.max(r[0] + r[1]).max(r[2] + r[3]);
        }
        (hi > lo).then(|| (lo, hi - lo))
    }

    /// Phrases cut from `data`, the node bytes starting at `start`; `None`
    /// when they do not match this catalog.
    pub fn read(&self, start: u64, data: &[u8]) -> Option<GameTexts> {
        let mut crc = 0u32;
        let mut cut = |off: u64, len: u64| -> Option<String> {
            let from = usize::try_from(off.checked_sub(start)?).ok()?;
            let to = from.checked_add(usize::try_from(len).ok()?)?;
            let bytes = data.get(from..to)?;
            crc = crc32_update(crc, bytes);
            String::from_utf8(bytes.to_vec()).ok()
        };
        let mut out = GameTexts::default();
        for (group, map) in [(&self.texts, &mut out.texts), (&self.keys, &mut out.keys)] {
            for (key, r) in group {
                let en = cut(r[0], r[1])?;
                let ru = cut(r[2], r[3])?;
                map.insert(
                    key.clone(),
                    Names {
                        en: en.trim().to_string(),
                        ru: ru.trim().to_string(),
                        ru_official: true,
                    },
                );
            }
        }
        (crc == self.crc).then_some(out)
    }
}

fn crc32_update(crc: u32, data: &[u8]) -> u32 {
    let mut c = !crc;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
        }
    }
    !c
}

static GAME_TEXTS: RwLock<Option<Arc<GameTexts>>> = RwLock::new(None);

/// Keeps the phrases read from the game for every later text.
pub fn set_game_texts(texts: GameTexts) {
    if let Ok(mut g) = GAME_TEXTS.write() {
        *g = Some(Arc::new(texts));
    }
}

/// Phrases read from the game, if any.
pub fn game_texts() -> Option<Arc<GameTexts>> {
    GAME_TEXTS.read().ok()?.clone()
}

pub fn catalog() -> &'static Catalog {
    static CAT: OnceLock<Catalog> = OnceLock::new();
    CAT.get_or_init(|| serde_json::from_str(CATALOG_JSON).expect("embedded catalog.json is valid"))
}

impl Catalog {
    pub fn objective(&self, id: &str) -> Option<&Objective> {
        self.objectives.iter().find(|o| o.id == id)
    }

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

    pub fn unique_dweller(&self, id: &str) -> Option<&UniqueDweller> {
        self.unique_dwellers.iter().find(|u| u.id == id)
    }

    /// Outfit pieces for a gender (2 = male); None when the game has no such
    /// outfit for that gender, or when the catalog has no picture data.
    pub fn outfit_art(&self, id: &str, gender: i64) -> Option<&ArtOutfit> {
        let key = if gender == 2 { "male" } else { "female" };
        self.art.as_ref()?.dweller.get(key)?.outfits.get(id)
    }

    /// Whether a dweller of this gender can wear the outfit; the game hides other
    /// outfits and swaps them for a random casual one on load.
    pub fn outfit_fits(&self, id: &str, gender: i64) -> bool {
        let key = if gender == 2 { "male" } else { "female" };
        match self.art.as_ref().and_then(|a| a.dweller.get(key)) {
            Some(kit) if !kit.outfits.is_empty() => kit.outfits.contains_key(id),
            _ => true,
        }
    }

    pub fn theme(&self, id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == id)
    }

    /// Display name of a rarity. Saves call the common rarity "Normal".
    pub fn rarity_name(&self, rarity: &str, russian: bool) -> String {
        let key = if rarity == "Normal" { "Common" } else { rarity };
        self.names.rarity.get(key).map_or_else(
            || rarity.to_string(),
            |n| {
                let s = n.get(russian).to_lowercase();
                let mut c = s.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            },
        )
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
