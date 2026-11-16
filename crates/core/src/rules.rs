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

//! Normal-mode limits and out-of-norm warnings.

use serde::{Deserialize, Serialize};

use crate::catalog::{catalog, ItemKind};
use crate::json::{Seg, Value};
use crate::vault::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Normal,
    Expert,
}

pub const RARITIES: [&str; 4] = ["Normal", "Rare", "Legendary", "Common"];
pub const VAULT_MODES: [&str; 2] = ["Normal", "Survival"];
pub const MAX_DWELLERS: usize = 200;
pub const RESOURCE_MAX: i64 = 999_999;
pub const LUNCHBOX_MAX: usize = 9_999;
pub const ITEM_ADD_MAX: i64 = 1_000;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    pub code: &'static str,
    pub path: Vec<Seg>,
    pub value: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleError {
    OutOfRange { min: i64, max: i64 },
    NotAllowed,
    NotInCatalog,
    WrongType,
    ExpertOnly,
    TooManyDwellers,
    RoomOverlap,
    RoomDisconnected,
    RoomBusy,
    RoomFixed,
    RoomUnique,
    NotFound,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RuleError {}

pub fn check_range(mode: Mode, v: i64, min: i64, max: i64) -> Result<(), RuleError> {
    if mode == Mode::Normal && !(min..=max).contains(&v) {
        return Err(RuleError::OutOfRange { min, max });
    }
    Ok(())
}

pub fn check_choice(mode: Mode, v: &str, allowed: &[&str]) -> Result<(), RuleError> {
    if mode == Mode::Normal && !allowed.contains(&v) {
        return Err(RuleError::NotAllowed);
    }
    Ok(())
}

pub fn check_item(mode: Mode, kind: ItemKind, id: &str) -> Result<(), RuleError> {
    if mode == Mode::Normal && catalog().item(kind, id).is_none() {
        return Err(RuleError::NotInCatalog);
    }
    Ok(())
}

pub fn max_merge(room_type: &str) -> i64 {
    catalog().room(room_type).map_or(3, |r| r.max_merge)
}

fn warn(
    out: &mut Vec<Warning>,
    code: &'static str,
    path: Vec<Seg>,
    v: &Value,
    min: Option<f64>,
    max: Option<f64>,
) {
    out.push(Warning {
        code,
        path,
        value: v.to_compact(),
        min,
        max,
    });
}

fn range(
    out: &mut Vec<Warning>,
    code: &'static str,
    path: Vec<Seg>,
    root: &Value,
    min: f64,
    max: f64,
) {
    if let Some(v) = root.at(&path) {
        if let Some(x) = v.as_f64() {
            if !(min..=max).contains(&x) {
                warn(out, code, path, v, Some(min), Some(max));
            }
        }
    }
}

fn item_known(out: &mut Vec<Warning>, path: Vec<Seg>, root: &Value) {
    let Some(item) = root.at(&path) else { return };
    let (Some(id), Some(kind)) = (
        item.get("id").and_then(Value::as_text),
        item.get("type")
            .and_then(Value::as_text)
            .and_then(|t| ItemKind::parse(&t)),
    ) else {
        return;
    };
    if catalog().item(kind, &id).is_none() {
        warn(
            out,
            "item_unknown",
            path,
            item.get("id").unwrap_or(&Value::Null),
            None,
            None,
        );
    }
}

/// Values outside what the game accepts without side effects.
pub fn warnings(root: &Value) -> Vec<Warning> {
    let mut out = Vec::new();
    let lim = &catalog().limits;
    for (list, prefix) in [(DWELLERS, "dweller"), (QUEST_DWELLERS, "quest")] {
        for (i, d) in arr(root, &list).iter().enumerate() {
            let at = |rest: &[&str]| pi(&list, i, rest);
            for s in 1..=7usize {
                let mut path = at(&["stats", "stats"]);
                path.push(Seg::Index(s));
                path.push(Seg::Key("value".into()));
                range(
                    &mut out,
                    "special",
                    path,
                    root,
                    lim.special_min as f64,
                    lim.special_max as f64,
                );
            }
            range(
                &mut out,
                "level",
                at(&["experience", "currentLevel"]),
                root,
                1.0,
                lim.dweller_level_max as f64,
            );
            range(
                &mut out,
                "happiness",
                at(&["happiness", "happinessValue"]),
                root,
                lim.happiness_min as f64,
                lim.happiness_max as f64,
            );
            if let Some(g) = d.get("gender") {
                if !matches!(g.as_i64(), Some(1 | 2)) {
                    warn(&mut out, "gender", at(&["gender"]), g, Some(1.0), Some(2.0));
                }
            }
            if let Some(r) = d.get("rarity") {
                if !r.as_text().is_some_and(|t| RARITIES.contains(&t.as_str())) {
                    warn(&mut out, "rarity", at(&["rarity"]), r, None, None);
                }
            }
            let max_hp = int(d, &["health", "maxHealth"])
                .map(|v| v as f64)
                .or_else(|| d.at(&p(&["health", "maxHealth"])).and_then(Value::as_f64));
            if let Some(max_hp) = max_hp {
                range(
                    &mut out,
                    "health",
                    at(&["health", "healthValue"]),
                    root,
                    0.0,
                    max_hp,
                );
                range(
                    &mut out,
                    "radiation",
                    at(&["health", "radiationValue"]),
                    root,
                    0.0,
                    max_hp,
                );
            }
            for slot in ["equipedOutfit", "equipedWeapon", "equippedPet"] {
                item_known(&mut out, at(&[slot]), root);
            }
            let _ = prefix;
        }
    }
    for (i, r) in arr(root, &ROOMS).iter().enumerate() {
        let kind = text(r, &["type"]).unwrap_or_default();
        range(
            &mut out,
            "room_level",
            pi(&ROOMS, i, &["level"]),
            root,
            1.0,
            lim.room_level_max as f64,
        );
        range(
            &mut out,
            "room_merge",
            pi(&ROOMS, i, &["mergeLevel"]),
            root,
            1.0,
            max_merge(&kind) as f64,
        );
        if !kind.is_empty() && catalog().room(&kind).is_none() {
            warn(
                &mut out,
                "room_unknown",
                pi(&ROOMS, i, &["type"]),
                r.get("type").unwrap_or(&Value::Null),
                None,
                None,
            );
        }
    }
    for (i, _) in arr(root, &ITEMS).iter().enumerate() {
        item_known(&mut out, pi(&ITEMS, i, &[]), root);
    }
    range(
        &mut out,
        "vault_theme",
        p(&["vault", "VaultTheme"]),
        root,
        0.0,
        lim.vault_theme_max as f64,
    );
    if let Some(m) = root.at(&p(&["vault", "VaultMode"])) {
        if !m
            .as_text()
            .is_some_and(|t| VAULT_MODES.contains(&t.as_str()))
        {
            warn(
                &mut out,
                "vault_mode",
                p(&["vault", "VaultMode"]),
                m,
                None,
                None,
            );
        }
    }
    for base in [RESOURCES, BONUS] {
        for key in RESOURCE_KEYS {
            let mut path = p(&base);
            path.push(Seg::Key(key.into()));
            range(&mut out, "resource", path, root, 0.0, RESOURCE_MAX as f64);
        }
    }
    for (i, t) in arr(root, &["vault", "LunchBoxesByType"]).iter().enumerate() {
        if !t.as_i64().is_some_and(|x| (0..=7).contains(&x)) {
            let mut path = p(&["vault", "LunchBoxesByType"]);
            path.push(Seg::Index(i));
            warn(&mut out, "lunchbox_type", path, t, Some(0.0), Some(7.0));
        }
    }
    let alive = arr(root, &DWELLERS).len();
    if alive > MAX_DWELLERS {
        out.push(Warning {
            code: "too_many_dwellers",
            path: p(&DWELLERS),
            value: alive.to_string(),
            min: None,
            max: Some(MAX_DWELLERS as f64),
        });
    }
    out
}

/// Warnings that were not in the file as opened.
pub fn new_warnings(original: &Value, current: &Value) -> (Vec<Warning>, Vec<Warning>) {
    let before: std::collections::HashSet<(String, String)> = warnings(original)
        .into_iter()
        .map(|w| (crate::json::path_to_string(&w.path), w.value))
        .collect();
    warnings(current)
        .into_iter()
        .partition(|w| before.contains(&(crate::json::path_to_string(&w.path), w.value.clone())))
}
