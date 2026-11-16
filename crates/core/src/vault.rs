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

//! Paths and read helpers for the save structure.

use crate::json::{Seg, Value};

pub fn p(parts: &[&str]) -> Vec<Seg> {
    parts.iter().map(|s| Seg::Key((*s).to_string())).collect()
}

pub fn pi(base: &[&str], index: usize, rest: &[&str]) -> Vec<Seg> {
    let mut v = p(base);
    v.push(Seg::Index(index));
    v.extend(rest.iter().map(|s| Seg::Key((*s).to_string())));
    v
}

pub fn join(base: &[Seg], rest: &[&str]) -> Vec<Seg> {
    let mut v = base.to_vec();
    v.extend(rest.iter().map(|s| Seg::Key((*s).to_string())));
    v
}

pub const DWELLERS: [&str; 2] = ["dwellers", "dwellers"];
pub const ACTORS: [&str; 2] = ["dwellers", "actors"];
pub const ROOMS: [&str; 2] = ["vault", "rooms"];
pub const ITEMS: [&str; 3] = ["vault", "inventory", "items"];
pub const RESOURCES: [&str; 3] = ["vault", "storage", "resources"];
pub const BONUS: [&str; 3] = ["vault", "storage", "bonus"];
pub const QUEST_DWELLERS: [&str; 2] = ["questDwellers", "dwellers"];

pub const RESOURCE_KEYS: [&str; 15] = [
    "Nuka",
    "Food",
    "Energy",
    "Water",
    "StimPack",
    "RadAway",
    "Lunchbox",
    "MrHandy",
    "PetCarrier",
    "CraftedOutfit",
    "CraftedWeapon",
    "NukaColaQuantum",
    "CraftedTheme",
    "DummyUltracite",
    "PokerChip",
];

pub const SPECIAL_LETTERS: [&str; 7] = ["S", "P", "E", "C", "I", "A", "L"];

pub fn arr<'a>(root: &'a Value, path: &[&str]) -> &'a [Value] {
    root.at(&p(path))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

pub fn int(v: &Value, path: &[&str]) -> Option<i64> {
    v.at(&p(path)).and_then(Value::as_i64)
}

pub fn text(v: &Value, path: &[&str]) -> Option<String> {
    v.at(&p(path)).and_then(Value::as_text)
}

pub fn flag(v: &Value, path: &[&str]) -> Option<bool> {
    v.at(&p(path)).and_then(Value::as_bool)
}

pub fn dweller_ids(root: &Value) -> Vec<i64> {
    arr(root, &DWELLERS)
        .iter()
        .filter_map(|d| int(d, &["serializeId"]))
        .collect()
}

pub fn room_ids(root: &Value) -> Vec<i64> {
    arr(root, &ROOMS)
        .iter()
        .filter_map(|r| int(r, &["deserializeID"]))
        .collect()
}

pub fn dweller_index(root: &Value, id: i64) -> Option<usize> {
    arr(root, &DWELLERS)
        .iter()
        .position(|d| int(d, &["serializeId"]) == Some(id))
}

pub fn room_index(root: &Value, id: i64) -> Option<usize> {
    arr(root, &ROOMS)
        .iter()
        .position(|r| int(r, &["deserializeID"]) == Some(id))
}

/// Value of SPECIAL stat 1..=7 (S..L).
pub fn special(d: &Value, stat: usize) -> Option<i64> {
    d.at(&[
        Seg::Key("stats".into()),
        Seg::Key("stats".into()),
        Seg::Index(stat),
        Seg::Key("value".into()),
    ])
    .and_then(Value::as_i64)
}

pub fn special_mod(d: &Value, stat: usize) -> i64 {
    d.at(&[
        Seg::Key("stats".into()),
        Seg::Key("stats".into()),
        Seg::Index(stat),
        Seg::Key("mod".into()),
    ])
    .and_then(Value::as_i64)
    .unwrap_or(0)
}

pub fn dweller_name(d: &Value) -> String {
    let first = text(d, &["name"]).unwrap_or_default();
    let last = text(d, &["lastName"]).unwrap_or_default();
    format!("{first} {last}").trim().to_string()
}

pub fn is_dead(d: &Value) -> bool {
    int(d, &["deathTime"]).is_some_and(|t| t >= 0)
        || d.at(&p(&["deathTime"]))
            .and_then(Value::as_f64)
            .is_some_and(|t| t >= 0.0)
        || flag(d, &["health", "permaDeath"]) == Some(true)
}

/// Grid cells a room covers in its row.
pub fn room_width(room: &Value) -> i64 {
    let kind = text(room, &["type"]).unwrap_or_default();
    crate::map::width_of(&kind, int(room, &["mergeLevel"]).unwrap_or(1))
}

/// Task ids present in a scheduler (`taskMgr` or `task`).
pub fn task_ids(root: &Value, scheduler: &str) -> Vec<i64> {
    let mut out = Vec::new();
    for list in ["tasks", "pausedTasks"] {
        for t in arr(root, &[scheduler, list]) {
            if let Some(id) = int(t, &["id"]) {
                out.push(id);
            }
        }
    }
    out
}
