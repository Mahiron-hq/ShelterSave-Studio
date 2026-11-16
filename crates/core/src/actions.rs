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

//! Editor actions. Each action becomes one undoable transaction.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::catalog::{catalog, ItemKind};
use crate::edit::{Builder, EditError, Label, Transaction};
use crate::integrity;
use crate::json::{self, Number, Seg, Value};
use crate::map;
use crate::rules::{self, Mode, RuleError, LUNCHBOX_MAX, MAX_DWELLERS, RESOURCE_MAX};
use crate::vault::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Slot {
    Weapon,
    Outfit,
    Pet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Action {
    /// Number field, written in the style of the existing literal.
    SetNumber {
        path: Vec<Seg>,
        value: f64,
    },
    /// Integer field set exactly (ticks and other large numbers).
    SetInt {
        path: Vec<Seg>,
        value: i64,
    },
    SetText {
        path: Vec<Seg>,
        value: String,
    },
    SetBool {
        path: Vec<Seg>,
        value: bool,
    },
    /// Any JSON value as text (expert mode).
    SetRaw {
        path: Vec<Seg>,
        json: String,
    },
    /// Replaces the whole save with imported JSON.
    SetRawRoot {
        json: String,
    },
    /// Restores one value to the file as opened.
    RevertValue {
        path: Vec<Seg>,
    },
    InsertRaw {
        parent: Vec<Seg>,
        index: usize,
        key: Option<String>,
        json: String,
    },
    RemoveAt {
        parent: Vec<Seg>,
        index: usize,
    },

    SetSpecial {
        ids: Vec<i64>,
        stat: usize,
        value: i64,
    },
    MaxSpecial {
        ids: Vec<i64>,
    },
    SetLevel {
        ids: Vec<i64>,
        level: i64,
    },
    FullHealth {
        ids: Vec<i64>,
    },
    ClearRadiation {
        ids: Vec<i64>,
    },
    MaxHappiness {
        ids: Vec<i64>,
    },
    Revive {
        ids: Vec<i64>,
    },
    Equip {
        ids: Vec<i64>,
        slot: Slot,
        item: Option<String>,
    },
    SetPetDetails {
        id: i64,
        name: String,
        bonus: String,
        bonus_value: i64,
    },
    /// Same relative field on every listed dweller.
    SetDwellerField {
        ids: Vec<i64>,
        rel: Vec<Seg>,
        json: String,
    },
    SetColor {
        ids: Vec<i64>,
        field: String,
        rgb: u32,
    },
    AddDweller {
        first: String,
        last: String,
        gender: i64,
    },
    CloneDweller {
        id: i64,
    },
    DeleteDwellers {
        ids: Vec<i64>,
    },
    AssignRoom {
        ids: Vec<i64>,
        room: i64,
    },
    SetPartner {
        id: i64,
        partner: i64,
    },

    SetResource {
        bonus: bool,
        key: String,
        value: f64,
    },
    FillResources {
        bonus: bool,
    },
    SetAllResources {
        bonus: bool,
        value: f64,
    },
    SetLunchboxes {
        counts: [i64; 4],
    },
    AddItems {
        kind: ItemKind,
        id: String,
        count: i64,
    },
    RemoveItems {
        kind: ItemKind,
        id: String,
        count: i64,
    },
    SetMergeLevel {
        room: i64,
        merge: i64,
    },
    AddRoom {
        kind: String,
        row: i64,
        col: i64,
        merge: i64,
        level: i64,
    },
    RemoveRoom {
        room: i64,
    },
    MoveRoom {
        room: i64,
        row: i64,
        col: i64,
    },
    SetRoomLevel {
        room: i64,
        level: i64,
    },
    RemoveRock {
        index: usize,
    },
    RemoveAllRocks,
    UnlockAllRooms,
    UnlockAllRecipes,
    CompleteCollection,
    FinishEmergency,
    CompleteTutorial,
    FixIssues,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "detail")]
pub enum ActionError {
    Edit(String),
    Rule(RuleError),
    BadJson(String),
    Nothing,
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionError::Edit(e) => write!(f, "edit: {e}"),
            ActionError::Rule(r) => write!(f, "rule: {r}"),
            ActionError::BadJson(e) => write!(f, "json: {e}"),
            ActionError::Nothing => f.write_str("nothing to change"),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<EditError> for ActionError {
    fn from(e: EditError) -> Self {
        ActionError::Edit(e.to_string())
    }
}

impl From<RuleError> for ActionError {
    fn from(e: RuleError) -> Self {
        ActionError::Rule(e)
    }
}

type R<T> = Result<T, ActionError>;

pub struct Ctx<'a> {
    pub mode: Mode,
    pub original: &'a Value,
}

/// Runs an action against the tree. On success the tree is changed and the
/// transaction describes the change; on error the tree is left as it was.
pub fn run(root: &mut Value, ctx: &Ctx<'_>, action: &Action) -> R<Transaction> {
    let mut b = Builder::new(root);
    let result = dispatch(&mut b, ctx, action);
    match result {
        Ok((label, affected)) if b.has_changes() => Ok(Transaction {
            label,
            ops: b.finish(),
            affected,
        }),
        Ok(_) => {
            b.abort();
            Err(ActionError::Nothing)
        }
        Err(e) => {
            b.abort();
            Err(e)
        }
    }
}

fn dispatch(b: &mut Builder<'_>, ctx: &Ctx<'_>, action: &Action) -> R<(Label, usize)> {
    let mode = ctx.mode;
    match action {
        Action::SetNumber { path, value } => {
            field_rule(b.root(), path, mode, *value)?;
            let cur = b.get(path).ok_or(RuleError::NotFound)?;
            let new = match cur {
                Value::Number(n) => Value::Number(n.restyle(*value)),
                _ if mode == Mode::Expert => Value::Number(Number::from_f64_fixed2(*value)),
                _ => return Err(RuleError::WrongType.into()),
            };
            b.set(path.clone(), new)?;
            Ok((
                Label::new("act.setValue").with("path", json::path_to_string(path)),
                1,
            ))
        }
        Action::SetInt { path, value } => {
            field_rule(b.root(), path, mode, *value as f64)?;
            match b.get(path).ok_or(RuleError::NotFound)? {
                Value::Number(n) if n.is_integer_literal() => {}
                Value::Number(_) => {
                    return dispatch(
                        b,
                        ctx,
                        &Action::SetNumber {
                            path: path.clone(),
                            value: *value as f64,
                        },
                    );
                }
                _ if mode == Mode::Expert => {}
                _ => return Err(RuleError::WrongType.into()),
            }
            b.set(path.clone(), Value::int(*value))?;
            Ok((
                Label::new("act.setValue").with("path", json::path_to_string(path)),
                1,
            ))
        }
        Action::SetText { path, value } => {
            let cur = b.get(path).ok_or(RuleError::NotFound)?;
            if !matches!(cur, Value::String(_)) && mode == Mode::Normal {
                return Err(RuleError::WrongType.into());
            }
            text_rule(path, mode, value)?;
            b.set(path.clone(), Value::str(value))?;
            Ok((
                Label::new("act.setValue").with("path", json::path_to_string(path)),
                1,
            ))
        }
        Action::SetBool { path, value } => {
            let cur = b.get(path).ok_or(RuleError::NotFound)?;
            if !matches!(cur, Value::Bool(_)) && mode == Mode::Normal {
                return Err(RuleError::WrongType.into());
            }
            b.set(path.clone(), Value::Bool(*value))?;
            Ok((
                Label::new("act.setValue").with("path", json::path_to_string(path)),
                1,
            ))
        }
        Action::SetRaw { path, json } => {
            expert(mode)?;
            let v = json::parse(json).map_err(|e| ActionError::BadJson(e.to_string()))?;
            b.set(path.clone(), v)?;
            Ok((
                Label::new("act.setValue").with("path", json::path_to_string(path)),
                1,
            ))
        }
        Action::SetRawRoot { json } => {
            let v = json::parse(json).map_err(|e| ActionError::BadJson(e.to_string()))?;
            if v.as_object().is_none() {
                return Err(RuleError::WrongType.into());
            }
            b.set(Vec::new(), v)?;
            Ok((Label::new("act.importJson"), 1))
        }
        Action::RevertValue { path } => {
            let orig = crate::diff::original_at(ctx.original, b.root(), path)
                .cloned()
                .ok_or(RuleError::NotFound)?;
            b.set(path.clone(), orig)?;
            Ok((
                Label::new("act.revertValue").with("path", json::path_to_string(path)),
                1,
            ))
        }
        Action::InsertRaw {
            parent,
            index,
            key,
            json,
        } => {
            expert(mode)?;
            let v = json::parse(json).map_err(|e| ActionError::BadJson(e.to_string()))?;
            b.insert(parent.clone(), *index, key.as_deref(), v)?;
            Ok((
                Label::new("act.insert").with("path", json::path_to_string(parent)),
                1,
            ))
        }
        Action::RemoveAt { parent, index } => {
            expert(mode)?;
            b.remove(parent.clone(), *index)?;
            Ok((
                Label::new("act.remove").with("path", json::path_to_string(parent)),
                1,
            ))
        }
        Action::SetSpecial { ids, stat, value } => {
            let lim = &catalog().limits;
            rules::check_range(mode, *value, lim.special_min, lim.special_max)?;
            if !(1..=7).contains(stat) {
                return Err(RuleError::NotFound.into());
            }
            let n = for_dwellers(b, ids, |b, base| {
                let mut path = join(base, &["stats", "stats"]);
                path.push(Seg::Index(*stat));
                path.push(Seg::Key("value".into()));
                set_num(b, path, *value as f64)
            })?;
            Ok((
                Label::new("act.setSpecial")
                    .with("stat", SPECIAL_LETTERS[stat - 1])
                    .with("value", value),
                n,
            ))
        }
        Action::MaxSpecial { ids } => {
            let max = catalog().limits.special_max as f64;
            let n = for_dwellers(b, ids, |b, base| {
                let mut any = false;
                for s in 1..=7 {
                    let mut path = join(base, &["stats", "stats"]);
                    path.push(Seg::Index(s));
                    path.push(Seg::Key("value".into()));
                    any |= set_num(b, path, max)?;
                }
                Ok(any)
            })?;
            Ok((Label::new("act.maxSpecial"), n))
        }
        Action::SetLevel { ids, level } => {
            rules::check_range(mode, *level, 1, catalog().limits.dweller_level_max)?;
            let n = for_dwellers(b, ids, |b, base| set_level(b, base, *level))?;
            Ok((Label::new("act.setLevel").with("level", level), n))
        }
        Action::FullHealth { ids } => {
            let n = for_dwellers(b, ids, |b, base| {
                let max = b
                    .get(&join(base, &["health", "maxHealth"]))
                    .and_then(Value::as_f64)
                    .unwrap_or(100.0);
                set_num(b, join(base, &["health", "healthValue"]), max)
            })?;
            Ok((Label::new("act.fullHealth"), n))
        }
        Action::ClearRadiation { ids } => {
            let n = for_dwellers(b, ids, |b, base| {
                set_num(b, join(base, &["health", "radiationValue"]), 0.0)
            })?;
            Ok((Label::new("act.clearRadiation"), n))
        }
        Action::MaxHappiness { ids } => {
            let max = catalog().limits.happiness_max as f64;
            let n = for_dwellers(b, ids, |b, base| {
                set_num(b, join(base, &["happiness", "happinessValue"]), max)
            })?;
            Ok((Label::new("act.maxHappiness"), n))
        }
        Action::Revive { ids } => {
            let n = for_dwellers(b, ids, revive)?;
            Ok((Label::new("act.revive"), n))
        }
        Action::Equip { ids, slot, item } => {
            let n = equip(b, mode, ids, *slot, item.as_deref())?;
            Ok((
                Label::new("act.equip").with("item", item.clone().unwrap_or_default()),
                n,
            ))
        }
        Action::SetPetDetails {
            id,
            name,
            bonus,
            bonus_value,
        } => {
            let base = dweller_path(b.root(), *id)?;
            let pet = join(&base, &["equippedPet"]);
            let pet_id = b
                .get(&pet)
                .and_then(|v| v.get("id"))
                .and_then(Value::as_text)
                .ok_or(RuleError::NotFound)?;
            if mode == Mode::Normal {
                let info = catalog().pet(&pet_id).ok_or(RuleError::NotInCatalog)?;
                if !catalog().pet_bonuses.contains_key(bonus) {
                    return Err(RuleError::NotAllowed.into());
                }
                if *bonus == info.bonus {
                    rules::check_range(
                        mode,
                        *bonus_value,
                        info.bonus_min.floor() as i64,
                        info.bonus_max.ceil() as i64,
                    )?;
                }
                if name.chars().count() > 40 {
                    return Err(RuleError::NotAllowed.into());
                }
            }
            if b.get(&join(&pet, &["extraData"])).is_none() {
                b.set_key(pet.clone(), "extraData", Value::obj(vec![]))?;
            }
            let extra = join(&pet, &["extraData"]);
            b.set_key(extra.clone(), "uniqueName", Value::str(name))?;
            b.set_key(extra.clone(), "bonus", Value::str(bonus))?;
            b.set_key(extra, "bonusValue", Value::int(*bonus_value))?;
            if let Some(ai) = pet_actor_index(b.root(), *id) {
                b.set(pi(&ACTORS, ai, &["name"]), Value::str(name))?;
            }
            Ok((Label::new("act.petDetails"), 1))
        }
        Action::SetDwellerField { ids, rel, json } => {
            let v = json::parse(json).map_err(|e| ActionError::BadJson(e.to_string()))?;
            let n = for_dwellers(b, ids, |b, base| {
                let mut path = base.to_vec();
                path.extend(rel.iter().cloned());
                let Some(cur) = b.get(&path) else {
                    return Ok(false);
                };
                if mode == Mode::Normal {
                    if cur.type_name() != v.type_name() {
                        return Err(RuleError::WrongType.into());
                    }
                    if let Some(x) = v.as_f64() {
                        field_rule(b.root(), &path, mode, x)?;
                    }
                    if is_id_field(&path) {
                        return Err(RuleError::ExpertOnly.into());
                    }
                }
                let new = match (cur, &v) {
                    (Value::Number(n), Value::Number(_)) => {
                        Value::Number(n.restyle(v.as_f64().unwrap_or(0.0)))
                    }
                    _ => v.clone(),
                };
                Ok(b.set(path, new)?)
            })?;
            Ok((
                Label::new("act.setField").with("field", json::path_to_string(rel)),
                n,
            ))
        }
        Action::SetColor { ids, field, rgb } => {
            if !["skinColor", "hairColor", "outfitColor"].contains(&field.as_str()) {
                return Err(RuleError::NotAllowed.into());
            }
            let argb = 0xff00_0000u32 | (rgb & 0x00ff_ffff);
            let n = for_dwellers(b, ids, |b, base| {
                Ok(b.set(join(base, &[field]), Value::int(argb as i64))?)
            })?;
            Ok((Label::new("act.setColor"), n))
        }
        Action::AddDweller {
            first,
            last,
            gender,
        } => {
            rules::check_range(mode, *gender, 1, 2)?;
            if mode == Mode::Normal && arr(b.root(), &DWELLERS).len() >= MAX_DWELLERS {
                return Err(RuleError::TooManyDwellers.into());
            }
            let id = free_dweller_id(b.root())?;
            let d = new_dweller(id, first, last, *gender);
            b.push(p(&DWELLERS), d)?;
            bump_dweller_counter(b, id)?;
            Ok((
                Label::new("act.addDweller").with("name", format!("{first} {last}").trim()),
                1,
            ))
        }
        Action::CloneDweller { id } => {
            if mode == Mode::Normal && arr(b.root(), &DWELLERS).len() >= MAX_DWELLERS {
                return Err(RuleError::TooManyDwellers.into());
            }
            let idx = dweller_index(b.root(), *id).ok_or(RuleError::NotFound)?;
            let src = arr(b.root(), &DWELLERS)[idx].clone();
            let new_id = free_dweller_id(b.root())?;
            let copy = clone_dweller(src, new_id);
            b.insert(p(&DWELLERS), idx + 1, None, copy)?;
            bump_dweller_counter(b, new_id)?;
            Ok((Label::new("act.cloneDweller"), 1))
        }
        Action::DeleteDwellers { ids } => {
            let mut n = 0;
            for id in ids {
                if delete_dweller(b, *id)? {
                    n += 1;
                }
            }
            Ok((Label::new("act.deleteDwellers").with("count", n), n))
        }
        Action::AssignRoom { ids, room } => {
            let n = assign_room(b, mode, ids, *room)?;
            Ok((Label::new("act.assignRoom"), n))
        }
        Action::SetPartner { id, partner } => {
            set_partner(b, *id, *partner)?;
            Ok((Label::new("act.setPartner"), 1))
        }
        Action::SetResource { bonus, key, value } => {
            if !RESOURCE_KEYS.contains(&key.as_str()) && mode == Mode::Normal {
                return Err(RuleError::NotAllowed.into());
            }
            rules::check_range(mode, value.round() as i64, 0, RESOURCE_MAX)?;
            let base = p(if *bonus { &BONUS } else { &RESOURCES });
            let changed = set_resource(b, &base, key, *value)?;
            Ok((
                Label::new("act.setResource")
                    .with("key", key)
                    .with("value", value),
                usize::from(changed),
            ))
        }
        Action::FillResources { bonus } => {
            let base = p(if *bonus { &BONUS } else { &RESOURCES });
            let mut n = 0;
            for key in [
                "Nuka",
                "Food",
                "Energy",
                "Water",
                "StimPack",
                "RadAway",
                "NukaColaQuantum",
            ] {
                n += usize::from(set_resource(b, &base, key, RESOURCE_MAX as f64)?);
            }
            Ok((Label::new("act.fillResources"), n))
        }
        Action::SetAllResources { bonus, value } => {
            rules::check_range(mode, value.round() as i64, 0, RESOURCE_MAX)?;
            let base = p(if *bonus { &BONUS } else { &RESOURCES });
            let mut n = 0;
            for key in RESOURCE_KEYS {
                n += usize::from(set_resource(b, &base, key, *value)?);
            }
            Ok((Label::new("act.setAllResources").with("value", value), n))
        }
        Action::SetLunchboxes { counts } => {
            for c in counts {
                rules::check_range(mode, *c, 0, LUNCHBOX_MAX as i64)?;
            }
            let n = set_lunchboxes(b, counts)?;
            Ok((Label::new("act.lunchboxes"), n))
        }
        Action::AddItems { kind, id, count } => {
            rules::check_item(mode, *kind, id)?;
            rules::check_range(mode, *count, 1, rules::ITEM_ADD_MAX)?;
            let item = item_value(*kind, id);
            for _ in 0..(*count).max(0) {
                b.push(p(&ITEMS), item.clone())?;
            }
            Ok((
                Label::new("act.addItems")
                    .with("id", id)
                    .with("count", count),
                *count as usize,
            ))
        }
        Action::RemoveItems { kind, id, count } => {
            let mut left = *count;
            let mut n = 0;
            while left > 0 {
                let pos = arr(b.root(), &ITEMS).iter().rposition(|it| {
                    text(it, &["id"]).as_deref() == Some(id)
                        && text(it, &["type"]).as_deref() == Some(kind.as_str())
                });
                let Some(i) = pos else { break };
                b.remove(p(&ITEMS), i)?;
                left -= 1;
                n += 1;
            }
            Ok((
                Label::new("act.removeItems")
                    .with("id", id)
                    .with("count", n),
                n,
            ))
        }
        Action::SetMergeLevel { room, merge } => {
            set_merge(b, mode, *room, *merge)?;
            Ok((Label::new("act.mergeLevel").with("merge", merge), 1))
        }
        Action::AddRoom {
            kind,
            row,
            col,
            merge,
            level,
        } => {
            add_room(b, mode, kind, *row, *col, *merge, *level)?;
            Ok((Label::new("act.addRoom").with("room", kind), 1))
        }
        Action::RemoveRoom { room } => {
            let kind = room_kind(b.root(), *room)?;
            let n = remove_room(b, mode, *room)?;
            Ok((Label::new("act.removeRoom").with("room", kind), n))
        }
        Action::MoveRoom { room, row, col } => {
            let kind = room_kind(b.root(), *room)?;
            move_room(b, mode, *room, *row, *col)?;
            Ok((Label::new("act.moveRoom").with("room", kind), 1))
        }
        Action::SetRoomLevel { room, level } => {
            set_room_level(b, mode, *room, *level)?;
            Ok((Label::new("act.roomLevel").with("level", level), 1))
        }
        Action::RemoveRock { index } => {
            b.remove(p(&["vault", "rocks"]), *index)?;
            Ok((Label::new("act.removeRock"), 1))
        }
        Action::RemoveAllRocks => {
            let n = arr(b.root(), &["vault", "rocks"]).len();
            b.set(p(&["vault", "rocks"]), Value::Array(Vec::new()))?;
            Ok((Label::new("act.removeAllRocks").with("count", n), n))
        }
        Action::UnlockAllRooms => {
            let n = unlock_all_rooms(b)?;
            Ok((Label::new("act.unlockAllRooms"), n))
        }
        Action::UnlockAllRecipes => {
            let c = catalog();
            let ids: Vec<&str> = c
                .weapons
                .iter()
                .filter(|w| w.recipe)
                .map(|w| w.id.as_str())
                .chain(c.outfits.iter().filter(|o| o.recipe).map(|o| o.id.as_str()))
                .chain(c.themes.iter().filter(|t| t.recipe).map(|t| t.id.as_str()))
                .collect();
            let n = ensure_strings(
                b,
                p(&["survivalW", "recipes"]),
                ids.iter().map(|s| s.to_string()),
                |_| None,
            )?;
            Ok((Label::new("act.unlockAllRecipes"), n))
        }
        Action::CompleteCollection => {
            let n = complete_collection(b)?;
            Ok((Label::new("act.completeCollection"), n))
        }
        Action::FinishEmergency => {
            let base = p(&["vault", "emergencyData"]);
            let mut n = 0;
            for (k, v) in [
                ("active", Value::Bool(false)),
                ("failedPreviousWave", Value::Bool(false)),
                ("currentWave", Value::int(0)),
                ("graceTimeLeft", Value::int(-1)),
            ] {
                if b.get(&join(&base, &[k])).is_some() {
                    n += usize::from(b.set(join(&base, &[k]), v)?);
                }
            }
            Ok((Label::new("act.finishEmergency"), n.min(1)))
        }
        Action::CompleteTutorial => {
            let n = complete_tutorial(b)?;
            Ok((Label::new("act.completeTutorial"), n))
        }
        Action::FixIssues => {
            let n = integrity::fix_all(b)?;
            Ok((Label::new("act.fixIssues").with("count", n), n))
        }
    }
}

fn expert(mode: Mode) -> R<()> {
    if mode == Mode::Normal {
        return Err(RuleError::ExpertOnly.into());
    }
    Ok(())
}

fn is_id_field(path: &[Seg]) -> bool {
    matches!(path.last(), Some(Seg::Key(k)) if k == "serializeId" || k == "deserializeID")
}

fn key_at(path: &[Seg], from_end: usize) -> Option<&str> {
    path.len()
        .checked_sub(from_end + 1)
        .and_then(|i| match &path[i] {
            Seg::Key(k) => Some(k.as_str()),
            Seg::Index(_) => None,
        })
}

/// Normal-mode range of a number field addressed by full path.
fn field_rule(root: &Value, path: &[Seg], mode: Mode, v: f64) -> R<()> {
    if mode == Mode::Expert {
        return Ok(());
    }
    if !v.is_finite() {
        return Err(RuleError::OutOfRange { min: 0, max: 0 }.into());
    }
    let lim = &catalog().limits;
    let x = v.round() as i64;
    let last = key_at(path, 0).unwrap_or("");
    let parent = key_at(path, 1).unwrap_or("");
    if is_id_field(path) {
        return Err(RuleError::ExpertOnly.into());
    }
    let r = match (parent, last) {
        (_, "value") if matches!(path.get(path.len().saturating_sub(3)), Some(Seg::Key(k)) if k == "stats") => {
            Some((lim.special_min, lim.special_max))
        }
        ("experience", "currentLevel") => Some((1, lim.dweller_level_max)),
        ("happiness", "happinessValue") => Some((lim.happiness_min, lim.happiness_max)),
        (_, "gender") => Some((1, 2)),
        (_, "level") if path.starts_with(&p(&ROOMS)) => Some((1, lim.room_level_max)),
        (_, "mergeLevel") => return Err(RuleError::ExpertOnly.into()),
        ("vault", "VaultTheme") => Some((0, lim.vault_theme_max)),
        ("resources" | "bonus", k) if RESOURCE_KEYS.contains(&k) => Some((0, RESOURCE_MAX)),
        ("health", "healthValue" | "radiationValue") => {
            let mut mp = path.to_vec();
            mp.pop();
            mp.push(Seg::Key("maxHealth".into()));
            let max = root.at(&mp).and_then(Value::as_f64).unwrap_or(0.0).floor() as i64;
            Some((0, max.max(0)))
        }
        _ => None,
    };
    if let Some((min, max)) = r {
        if !(min as f64..=max as f64).contains(&v) {
            return Err(RuleError::OutOfRange { min, max }.into());
        }
    }
    let _ = x;
    Ok(())
}

fn text_rule(path: &[Seg], mode: Mode, value: &str) -> R<()> {
    if mode == Mode::Expert {
        return Ok(());
    }
    let last = key_at(path, 0).unwrap_or("");
    match last {
        "rarity" => rules::check_choice(mode, value, &rules::RARITIES)?,
        "VaultMode" => rules::check_choice(mode, value, &rules::VAULT_MODES)?,
        "name" | "lastName" | "VaultName" | "uniqueName" if value.chars().count() > 64 => {
            return Err(RuleError::NotAllowed.into())
        }
        _ => {}
    }
    Ok(())
}

fn set_num(b: &mut Builder<'_>, path: Vec<Seg>, v: f64) -> R<bool> {
    let new = match b.get(&path) {
        Some(Value::Number(n)) => Value::Number(n.restyle(v)),
        Some(_) => return Err(RuleError::WrongType.into()),
        None => return Ok(false),
    };
    Ok(b.set(path, new)?)
}

fn dweller_path(root: &Value, id: i64) -> R<Vec<Seg>> {
    let i = dweller_index(root, id).ok_or(RuleError::NotFound)?;
    Ok(pi(&DWELLERS, i, &[]))
}

fn for_dwellers(
    b: &mut Builder<'_>,
    ids: &[i64],
    mut f: impl FnMut(&mut Builder<'_>, &[Seg]) -> R<bool>,
) -> R<usize> {
    let mut n = 0;
    for id in ids {
        let Some(i) = dweller_index(b.root(), *id) else {
            continue;
        };
        if f(b, &pi(&DWELLERS, i, &[]))? {
            n += 1;
        }
    }
    Ok(n)
}

fn effective_endurance(d: &Value) -> i64 {
    special(d, 3).unwrap_or(1) + special_mod(d, 3)
}

fn set_level(b: &mut Builder<'_>, base: &[Seg], level: i64) -> R<bool> {
    let c = catalog();
    let d = b.get(base).cloned().ok_or(RuleError::NotFound)?;
    let mut changed = set_num(b, join(base, &["experience", "currentLevel"]), level as f64)?;
    changed |= set_num(
        b,
        join(base, &["experience", "experienceValue"]),
        c.level_xp(level) as f64,
    )?;
    if d.at(&p(&["experience", "needLvUp"])).is_some() {
        changed |= b.set(join(base, &["experience", "needLvUp"]), Value::Bool(false))?;
    }
    let e = effective_endurance(&d);
    let last = int(&d, &["health", "lastLevelUpdated"]).unwrap_or(1);
    let max = d
        .at(&p(&["health", "maxHealth"]))
        .and_then(Value::as_f64)
        .unwrap_or(c.initial_max_health(e));
    let new_max = if level >= last {
        max + (level - last) as f64 * c.health_per_level(e)
    } else {
        c.initial_max_health(e) + (level - 1) as f64 * c.health_per_level(e)
    };
    changed |= set_num(b, join(base, &["health", "maxHealth"]), new_max)?;
    changed |= set_num(b, join(base, &["health", "lastLevelUpdated"]), level as f64)?;
    let hp = d
        .at(&p(&["health", "healthValue"]))
        .and_then(Value::as_f64)
        .unwrap_or(new_max);
    if hp > new_max {
        changed |= set_num(b, join(base, &["health", "healthValue"]), new_max)?;
    }
    Ok(changed)
}

fn revive(b: &mut Builder<'_>, base: &[Seg]) -> R<bool> {
    let d = b.get(base).cloned().ok_or(RuleError::NotFound)?;
    if !is_dead(&d)
        && d.at(&p(&["health", "healthValue"]))
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            > 0.0
    {
        return Ok(false);
    }
    let id = int(&d, &["serializeId"]).unwrap_or(-1);
    let mut changed = set_num(b, join(base, &["deathTime"]), -1.0)?;
    changed |= set_num(b, join(base, &["deathSource"]), 0.0)?;
    if d.at(&p(&["health", "permaDeath"])).is_some() {
        changed |= b.set(join(base, &["health", "permaDeath"]), Value::Bool(false))?;
    }
    let max = d
        .at(&p(&["health", "maxHealth"]))
        .and_then(Value::as_f64)
        .unwrap_or(100.0);
    changed |= set_num(b, join(base, &["health", "healthValue"]), max)?;
    changed |= set_num(b, join(base, &["health", "radiationValue"]), 0.0)?;
    for (i, r) in arr(&b.root().clone(), &ROOMS).iter().enumerate() {
        if let Some(Value::Array(dead)) = r.get("deadDwellers") {
            if let Some(j) = dead.iter().position(|v| v.as_i64() == Some(id)) {
                b.remove(pi(&ROOMS, i, &["deadDwellers"]), j)?;
                changed = true;
            }
        }
    }
    Ok(changed)
}

pub fn item_value(kind: ItemKind, id: &str) -> Value {
    let mut entries = vec![
        ("id", Value::str(id)),
        ("type", Value::str(kind.as_str())),
        ("hasBeenAssigned", Value::Bool(false)),
        ("hasRandonWeaponBeenAssigned", Value::Bool(false)),
    ];
    if kind == ItemKind::Pet {
        let (name, bonus, value) = catalog()
            .pet(id)
            .map(|p| {
                (
                    p.name.en.clone(),
                    p.bonus.clone(),
                    p.bonus_max.round() as i64,
                )
            })
            .unwrap_or_else(|| (id.to_string(), "None".to_string(), 0));
        entries.push((
            "extraData",
            Value::obj(vec![
                ("uniqueName", Value::str(&name)),
                ("bonus", Value::str(&bonus)),
                ("bonusValue", Value::int(value)),
            ]),
        ));
    }
    Value::obj(entries)
}

fn pet_actor_index(root: &Value, owner: i64) -> Option<usize> {
    arr(root, &ACTORS).iter().position(|a| {
        int(a, &["characterType"]) == Some(3) && int(a, &["FollowedID"]) == Some(owner)
    })
}

fn empty_storage() -> Value {
    let res = || Value::obj(RESOURCE_KEYS.iter().map(|k| (*k, Value::int(0))).collect());
    Value::obj(vec![("resources", res()), ("bonus", res())])
}

fn pet_actor(serialize_id: i64, pet_id: &str, name: &str, owner: i64) -> Value {
    Value::obj(vec![
        ("characterType", Value::int(3)),
        ("actorDataId", Value::str(pet_id)),
        ("serializeId", Value::int(serialize_id)),
        ("name", Value::str(name)),
        ("canCollect", Value::Bool(false)),
        ("willGoToWasteland", Value::Bool(false)),
        (
            "equipment",
            Value::obj(vec![
                ("storage", empty_storage()),
                (
                    "inventory",
                    Value::obj(vec![("items", Value::Array(vec![]))]),
                ),
                ("dwellers", Value::Array(vec![])),
                ("mrHandyList", Value::Array(vec![])),
                ("questClues", Value::Array(vec![])),
                (
                    "collectedThemes",
                    Value::obj(vec![("themeList", Value::Array(vec![]))]),
                ),
            ]),
        ),
        ("health", Value::int(5000)),
        ("death", Value::Bool(false)),
        ("savedRoom", Value::int(-1)),
        ("FollowedID", Value::int(owner)),
    ])
}

fn equip(b: &mut Builder<'_>, mode: Mode, ids: &[i64], slot: Slot, item: Option<&str>) -> R<usize> {
    let kind = match slot {
        Slot::Weapon => ItemKind::Weapon,
        Slot::Outfit => ItemKind::Outfit,
        Slot::Pet => ItemKind::Pet,
    };
    if let Some(id) = item {
        rules::check_item(mode, kind, id)?;
    }
    let key = match slot {
        Slot::Weapon => "equipedWeapon",
        Slot::Outfit => "equipedOutfit",
        Slot::Pet => "equippedPet",
    };
    let mut n = 0;
    for owner in ids {
        let Some(i) = dweller_index(b.root(), *owner) else {
            continue;
        };
        let base = pi(&DWELLERS, i, &[]);
        let changed = match (slot, item) {
            (Slot::Outfit, None) => b.set_key(base.clone(), key, item_value(kind, "jumpsuit"))?,
            (Slot::Weapon, None) => b.set_key(base.clone(), key, item_value(kind, "Fist"))?,
            (Slot::Pet, None) => {
                let mut c = b.remove_key(base.clone(), key)?;
                if let Some(ai) = pet_actor_index(b.root(), *owner) {
                    b.remove(p(&ACTORS), ai)?;
                    c = true;
                }
                c
            }
            (Slot::Pet, Some(pet)) => {
                let value = item_value(kind, pet);
                let name = value
                    .at(&p(&["extraData", "uniqueName"]))
                    .and_then(Value::as_text)
                    .unwrap_or_default();
                let mut c = b.set_key(base.clone(), key, value)?;
                match pet_actor_index(b.root(), *owner) {
                    Some(ai) => {
                        c |= b.set(pi(&ACTORS, ai, &["actorDataId"]), Value::str(pet))?;
                        c |= b.set(pi(&ACTORS, ai, &["name"]), Value::str(&name))?;
                    }
                    None if b.get(&p(&ACTORS)).is_some() => {
                        let next = int(b.root(), &["dwellers", "mrhId"]).unwrap_or(1).max(
                            arr(b.root(), &ACTORS)
                                .iter()
                                .filter_map(|a| int(a, &["serializeId"]))
                                .max()
                                .unwrap_or(0)
                                + 1,
                        );
                        b.push(p(&ACTORS), pet_actor(next, pet, &name, *owner))?;
                        if b.get(&p(&["dwellers", "mrhId"])).is_some() {
                            b.set(p(&["dwellers", "mrhId"]), Value::int(next + 1))?;
                        }
                        c = true;
                    }
                    None => {}
                }
                c
            }
            (_, Some(id)) => b.set_key(base.clone(), key, item_value(kind, id))?,
        };
        if changed {
            n += 1;
        }
    }
    Ok(n)
}

fn free_dweller_id(root: &Value) -> R<i64> {
    let used: HashSet<i64> = dweller_ids(root)
        .into_iter()
        .chain(
            arr(root, &QUEST_DWELLERS)
                .iter()
                .filter_map(|d| int(d, &["serializeId"])),
        )
        .collect();
    let max = used.iter().copied().max().unwrap_or(0);
    if max < 32767 {
        return Ok(max + 1);
    }
    (1..=32767)
        .find(|i| !used.contains(i))
        .ok_or_else(|| RuleError::TooManyDwellers.into())
}

fn bump_dweller_counter(b: &mut Builder<'_>, id: i64) -> R<()> {
    if let Some(cur) = int(b.root(), &["dwellers", "id"]) {
        if cur != 0 && cur < id {
            b.set(p(&["dwellers", "id"]), Value::int(id))?;
        }
    }
    Ok(())
}

pub fn new_dweller(id: i64, first: &str, last: &str, gender: i64) -> Value {
    let c = catalog();
    let hp = c.initial_max_health(1);
    let stat = |v: i64| {
        Value::obj(vec![
            ("value", Value::int(v)),
            ("mod", Value::int(0)),
            ("exp", Value::float2(0.0)),
        ])
    };
    Value::obj(vec![
        ("serializeId", Value::int(id)),
        ("name", Value::str(first)),
        ("lastName", Value::str(last)),
        (
            "happiness",
            Value::obj(vec![("happinessValue", Value::float2(50.0))]),
        ),
        (
            "health",
            Value::obj(vec![
                ("healthValue", Value::float2(hp)),
                ("radiationValue", Value::float2(0.0)),
                ("permaDeath", Value::Bool(false)),
                ("lastLevelUpdated", Value::int(1)),
                ("maxHealth", Value::float2(hp)),
            ]),
        ),
        ("deathSource", Value::int(0)),
        (
            "experience",
            Value::obj(vec![
                ("experienceValue", Value::float2(0.0)),
                ("currentLevel", Value::int(1)),
                ("storage", Value::int(0)),
                ("accum", Value::int(0)),
                ("needLvUp", Value::Bool(false)),
                ("wastelandExperience", Value::int(0)),
            ]),
        ),
        (
            "relations",
            Value::obj(vec![
                ("relations", Value::Array(vec![])),
                ("partner", Value::int(-1)),
                ("lastPartner", Value::int(-1)),
                ("ascendants", Value::Array(vec![Value::int(-1); 6])),
            ]),
        ),
        ("gender", Value::int(gender)),
        (
            "stats",
            Value::obj(vec![(
                "stats",
                Value::Array((0..8).map(|_| stat(1)).collect()),
            )]),
        ),
        ("pregnant", Value::Bool(false)),
        ("babyReady", Value::Bool(false)),
        ("assigned", Value::Bool(false)),
        ("sawIncident", Value::Bool(false)),
        ("WillGoToWasteland", Value::Bool(false)),
        ("WillBeEvicted", Value::Bool(false)),
        ("IsEvictedWaitingForFollowers", Value::Bool(false)),
        ("skinColor", Value::int(0xFFFF_E9D6)),
        ("hairColor", Value::int(0xFFCF_B038)),
        ("outfitColor", Value::int(0xFF8C_7B73)),
        ("pendingExperienceReward", Value::int(0)),
        ("hair", Value::str("01")),
        ("equipedOutfit", item_value(ItemKind::Outfit, "jumpsuit")),
        ("equipedWeapon", item_value(ItemKind::Weapon, "Fist")),
        ("savedRoom", Value::int(-1)),
        ("wasTemporarilyAssigned", Value::Bool(false)),
        ("lastChildBorn", Value::int(-1)),
        ("rarity", Value::str("Normal")),
        ("deathTime", Value::int(-1)),
    ])
}

fn clone_dweller(mut d: Value, new_id: i64) -> Value {
    let reset: [(&[&str], Value); 10] = [
        (&["serializeId"], Value::int(new_id)),
        (&["relations", "partner"], Value::int(-1)),
        (&["relations", "lastPartner"], Value::int(-1)),
        (&["savedRoom"], Value::int(-1)),
        (&["assigned"], Value::Bool(false)),
        (&["pregnant"], Value::Bool(false)),
        (&["babyReady"], Value::Bool(false)),
        (&["WillGoToWasteland"], Value::Bool(false)),
        (&["WillBeEvicted"], Value::Bool(false)),
        (&["wasTemporarilyAssigned"], Value::Bool(false)),
    ];
    for (path, v) in reset {
        if let Some(slot) = d.at_mut(&p(path)) {
            *slot = v;
        }
    }
    if let Some(o) = d.as_object_mut() {
        o.0.retain(|(k, _)| !k.eq_text("equippedPet") && !k.eq_text("storageWillBringToWasteland"));
    }
    d
}

fn remove_number(b: &mut Builder<'_>, list: Vec<Seg>, id: i64) -> R<usize> {
    let mut n = 0;
    while let Some(i) = b
        .get(&list)
        .and_then(Value::as_array)
        .and_then(|a| a.iter().position(|v| v.as_i64() == Some(id)))
    {
        b.remove(list.clone(), i)?;
        n += 1;
    }
    Ok(n)
}

fn delete_dweller(b: &mut Builder<'_>, id: i64) -> R<bool> {
    let Some(idx) = dweller_index(b.root(), id) else {
        return Ok(false);
    };
    let snapshot = b.root().clone();
    for (i, r) in arr(&snapshot, &ROOMS).iter().enumerate() {
        remove_number(b, pi(&ROOMS, i, &["dwellers"]), id)?;
        remove_number(
            b,
            pi(&ROOMS, i, &["dwellerWithPendingCompleteTraining"]),
            id,
        )?;
        remove_number(b, pi(&ROOMS, i, &["deadDwellers"]), id)?;
        for (j, s) in r
            .get("slots")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .enumerate()
        {
            if int(s, &["dwellerID"]) == Some(id) {
                let mut path = pi(&ROOMS, i, &["slots"]);
                path.push(Seg::Index(j));
                path.push(Seg::Key("dwellerID".into()));
                b.set(path, Value::int(-1))?;
            }
        }
        for list in ["partners", "children"] {
            let lp = pi(&ROOMS, i, &[list]);
            loop {
                let pos = b.get(&lp).and_then(Value::as_array).and_then(|a| {
                    a.iter().position(|e| {
                        ["m", "f", "fatherId", "dwellerId", "id"]
                            .iter()
                            .any(|k| int(e, &[k]) == Some(id))
                    })
                });
                match pos {
                    Some(k) => {
                        b.remove(lp.clone(), k)?;
                    }
                    None => break,
                }
            }
        }
    }
    for (i, d) in arr(&snapshot, &DWELLERS).iter().enumerate() {
        for key in ["partner", "lastPartner"] {
            if int(d, &["relations", key]) == Some(id) {
                b.set(pi(&DWELLERS, i, &["relations", key]), Value::int(-1))?;
            }
        }
    }
    let mut ai = 0;
    while ai < arr(b.root(), &ACTORS).len() {
        let a = &arr(b.root(), &ACTORS)[ai];
        if int(a, &["FollowedID"]) == Some(id) {
            if int(a, &["characterType"]) == Some(3) {
                b.remove(p(&ACTORS), ai)?;
                continue;
            }
            b.set(pi(&ACTORS, ai, &["FollowedID"]), Value::int(-1))?;
        }
        ai += 1;
    }
    let team = p(&["questDataManager", "questTeam"]);
    remove_number(b, join(&team, &["DwellersDictionary"]), id)?;
    let eq = join(&team, &["InitialEquipment"]);
    while let Some(k) = b
        .get(&eq)
        .and_then(Value::as_array)
        .and_then(|a| a.iter().position(|e| int(e, &["dweller"]) == Some(id)))
    {
        b.remove(eq.clone(), k)?;
    }
    let teams = p(&["vault", "wasteland", "teams"]);
    let mut t = 0;
    while t < arr(b.root(), &["vault", "wasteland", "teams"]).len() {
        let mut tp = teams.clone();
        tp.push(Seg::Index(t));
        remove_number(b, join(&tp, &["dwellers"]), id)?;
        if b.get(&join(&tp, &["dwellers"]))
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            b.remove(teams.clone(), t)?;
            continue;
        }
        t += 1;
    }
    let waiting = p(&["dwellerSpawner", "dwellersWaiting"]);
    while let Some(k) = b
        .get(&waiting)
        .and_then(Value::as_array)
        .and_then(|a| a.iter().position(|e| int(e, &["dwellerId"]) == Some(id)))
    {
        b.remove(waiting.clone(), k)?;
    }
    let hm = p(&["happinessManager"]);
    b.remove_key(hm.clone(), &format!("dweller{id}"))?;
    remove_number(b, join(&hm, &["dwellers"]), id)?;
    clear_requirement_refs(b, id)?;
    let idx = dweller_index(b.root(), id).unwrap_or(idx);
    b.remove(p(&DWELLERS), idx)?;
    Ok(true)
}

fn clear_requirement_refs(b: &mut Builder<'_>, id: i64) -> R<()> {
    fn find(v: &Value, path: &mut Vec<Seg>, id: i64, out: &mut Vec<Vec<Seg>>) {
        match v {
            Value::Object(o) => {
                if o.get("requirementID").is_some()
                    && o.get("dweller").and_then(Value::as_i64) == Some(id)
                {
                    let mut pth = path.clone();
                    pth.push(Seg::Key("dweller".into()));
                    out.push(pth);
                }
                for (k, c) in &o.0 {
                    path.push(Seg::Key(k.text()));
                    find(c, path, id, out);
                    path.pop();
                }
            }
            Value::Array(a) => {
                for (i, c) in a.iter().enumerate() {
                    path.push(Seg::Index(i));
                    find(c, path, id, out);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let mut hits = Vec::new();
    for key in ["objectiveMgr", "unlockableMgr", "vault"] {
        if let Some(v) = b.root().get(key) {
            let mut path = vec![Seg::Key(key.into())];
            find(v, &mut path, id, &mut hits);
        }
    }
    for h in hits {
        b.set(h, Value::int(-1))?;
    }
    Ok(())
}

fn room_capacity(room: &Value) -> usize {
    match text(room, &["type"]).as_deref() {
        Some("Elevator") => 0,
        _ => 2 * int(room, &["mergeLevel"]).unwrap_or(1).max(1) as usize,
    }
}

fn assign_room(b: &mut Builder<'_>, mode: Mode, ids: &[i64], room: i64) -> R<usize> {
    let target = if room == -1 {
        None
    } else {
        Some(room_index(b.root(), room).ok_or(RuleError::NotFound)?)
    };
    if let (Some(t), Mode::Normal) = (target, mode) {
        let r = &arr(b.root(), &ROOMS)[t];
        let current: HashSet<i64> = r
            .get("dwellers")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        let total = current.len() + ids.iter().filter(|id| !current.contains(id)).count();
        if total > room_capacity(r) {
            return Err(RuleError::OutOfRange {
                min: 0,
                max: room_capacity(r) as i64,
            }
            .into());
        }
    }
    let mut n = 0;
    for id in ids {
        if dweller_index(b.root(), *id).is_none() {
            continue;
        }
        drop_from_rooms(b, *id, target)?;
        let base = dweller_path(b.root(), *id)?;
        if let Some(t) = target {
            let list = pi(&ROOMS, t, &["dwellers"]);
            let present = b
                .get(&list)
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().any(|v| v.as_i64() == Some(*id)));
            if !present {
                b.push(list, Value::int(*id))?;
            }
            b.set(join(&base, &["savedRoom"]), Value::int(room))?;
            b.set(join(&base, &["assigned"]), Value::Bool(true))?;
        } else {
            b.set(join(&base, &["savedRoom"]), Value::int(-1))?;
            b.set(join(&base, &["assigned"]), Value::Bool(false))?;
        }
        n += 1;
    }
    Ok(n)
}

fn set_partner(b: &mut Builder<'_>, id: i64, partner: i64) -> R<()> {
    let base = dweller_path(b.root(), id)?;
    let old = b
        .get(&join(&base, &["relations", "partner"]))
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    if old != -1 {
        if let Ok(ob) = dweller_path(b.root(), old) {
            if b.get(&join(&ob, &["relations", "partner"]))
                .and_then(Value::as_i64)
                == Some(id)
            {
                b.set(join(&ob, &["relations", "partner"]), Value::int(-1))?;
            }
        }
    }
    b.set(join(&base, &["relations", "partner"]), Value::int(partner))?;
    if partner != -1 {
        let pb = dweller_path(b.root(), partner)?;
        let their_old = b
            .get(&join(&pb, &["relations", "partner"]))
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        if their_old != -1 && their_old != id {
            if let Ok(tb) = dweller_path(b.root(), their_old) {
                b.set(join(&tb, &["relations", "partner"]), Value::int(-1))?;
            }
        }
        b.set(join(&pb, &["relations", "partner"]), Value::int(id))?;
    }
    Ok(())
}

fn set_resource(b: &mut Builder<'_>, base: &[Seg], key: &str, v: f64) -> R<bool> {
    let mut path = base.to_vec();
    path.push(Seg::Key(key.into()));
    set_num(b, path, v)
}

fn set_lunchboxes(b: &mut Builder<'_>, counts: &[i64; 4]) -> R<usize> {
    let types_path = p(&["vault", "LunchBoxesByType"]);
    let names_path = p(&["vault", "LunchBoxDeckNames"]);
    let mut types: Vec<Value> = b
        .get(&types_path)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let names_cur: Option<Vec<Value>> = b.get(&names_path).and_then(Value::as_array).cloned();
    let keep_names = names_cur
        .as_ref()
        .is_some_and(|n| !n.is_empty() || types.is_empty());
    let mut names = names_cur.clone().unwrap_or_default();
    names.resize(types.len(), Value::str("Normal"));
    let mut changed = 0;
    for (t, want) in counts.iter().enumerate() {
        let t = t as i64;
        let have = types.iter().filter(|v| v.as_i64() == Some(t)).count() as i64;
        if have > *want {
            let mut extra = have - want;
            let mut i = types.len();
            while extra > 0 && i > 0 {
                i -= 1;
                if types[i].as_i64() == Some(t) {
                    types.remove(i);
                    names.remove(i);
                    extra -= 1;
                }
            }
            changed += (have - want) as usize;
        } else {
            for _ in have..*want {
                types.push(Value::int(t));
                names.push(Value::str("Normal"));
            }
            changed += (want - have) as usize;
        }
    }
    if changed == 0 {
        return Ok(0);
    }
    let count = types.len() as i64;
    b.set(types_path, Value::Array(types))?;
    if keep_names {
        b.set(names_path, Value::Array(names))?;
    }
    if b.get(&p(&["vault", "LunchBoxesCount"])).is_some() {
        b.set(p(&["vault", "LunchBoxesCount"]), Value::int(count))?;
    }
    Ok(changed)
}

/// Checks that a room at a new merge level stays inside the grid and does
/// not overlap other rooms or rocks in its row.
pub fn merge_fits(root: &Value, room: i64, merge: i64) -> bool {
    let Some(i) = room_index(root, room) else {
        return false;
    };
    let rooms = arr(root, &ROOMS);
    let r = &rooms[i];
    let (Some(row), Some(col)) = (int(r, &["row"]), int(r, &["col"])) else {
        return false;
    };
    let width = if text(r, &["type"]).as_deref() == Some("Elevator") {
        1
    } else {
        3 * merge
    };
    let end = col + width;
    if col < 0 || end > catalog().limits.grid_cols {
        return false;
    }
    for (j, o) in rooms.iter().enumerate() {
        if j == i || int(o, &["row"]) != Some(row) {
            continue;
        }
        let oc = int(o, &["col"]).unwrap_or(0);
        let oe = oc + room_width(o);
        if oc < end && col < oe {
            return false;
        }
    }
    for rock in arr(root, &["vault", "rocks"]) {
        if int(rock, &["r"]) == Some(row) {
            let rc = int(rock, &["c"]).unwrap_or(-10);
            if rc < end && col < rc + 2 {
                return false;
            }
        }
    }
    true
}

fn set_merge(b: &mut Builder<'_>, mode: Mode, room: i64, merge: i64) -> R<()> {
    let i = room_index(b.root(), room).ok_or(RuleError::NotFound)?;
    let r = arr(b.root(), &ROOMS)[i].clone();
    let kind = text(&r, &["type"]).unwrap_or_default();
    if map::FIXED_ROOMS.contains(&kind.as_str()) {
        return Err(RuleError::RoomFixed.into());
    }
    if kind == "Elevator" && mode == Mode::Normal {
        return Err(RuleError::NotAllowed.into());
    }
    rules::check_range(mode, merge, 1, rules::max_merge(&kind))?;
    let (row, col) = (
        int(&r, &["row"]).unwrap_or(0),
        int(&r, &["col"]).unwrap_or(0),
    );
    place(b.root(), mode, &kind, row, col, merge, Some(room))?;
    set_num(b, pi(&ROOMS, i, &["mergeLevel"]), merge as f64)?;
    let cap = 2 * merge.max(1) as usize;
    for id in ints(&r, "dwellers").iter().skip(cap) {
        unassign(b, *id)?;
    }
    Ok(())
}

fn place_error(e: map::PlaceError) -> ActionError {
    match e {
        map::PlaceError::OutOfGrid | map::PlaceError::Overlap => RuleError::RoomOverlap.into(),
        map::PlaceError::Disconnected => RuleError::RoomDisconnected.into(),
    }
}

/// Overlap is never allowed; connectivity is checked in normal mode only.
fn place(
    root: &Value,
    mode: Mode,
    kind: &str,
    row: i64,
    col: i64,
    merge: i64,
    id: Option<i64>,
) -> R<()> {
    let r = if mode == Mode::Normal {
        map::check_spot(root, kind, row, col, merge, id)
    } else {
        map::fits(root, row, col, map::width_of(kind, merge), id)
    };
    r.map_err(place_error)
}

fn ints(v: &Value, key: &str) -> Vec<i64> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

fn room_kind(root: &Value, id: i64) -> R<String> {
    let i = room_index(root, id).ok_or(RuleError::NotFound)?;
    Ok(text(&arr(root, &ROOMS)[i], &["type"]).unwrap_or_default())
}

fn next_room_id(b: &mut Builder<'_>) -> R<i64> {
    let counter = int(b.root(), &["constructMgr", "roomDeserializeID"]).unwrap_or(0);
    let max = room_ids(b.root()).into_iter().max().unwrap_or(0);
    let id = counter.max(max) + 1;
    let path = p(&["constructMgr", "roomDeserializeID"]);
    if b.get(&path).is_some() {
        set_num(b, path, id as f64)?;
    }
    Ok(id)
}

/// A freshly built room as the game writes it.
fn new_room(
    root: &Value,
    kind: &str,
    class: &str,
    at: (i64, i64),
    merge: i64,
    level: i64,
    id: i64,
) -> Value {
    let keys: Vec<String> = root
        .at(&p(&RESOURCES))
        .and_then(Value::as_object)
        .map(|o| o.keys().collect())
        .unwrap_or_else(|| RESOURCE_KEYS.iter().map(|k| k.to_string()).collect());
    let zeros = || Value::obj(keys.iter().map(|k| (k.as_str(), Value::int(0))).collect());
    let mut fields = vec![
        ("emergencyDone", Value::Bool(false)),
        ("type", Value::str(kind)),
        ("class", Value::str(class)),
        ("mergeLevel", Value::int(merge)),
        ("row", Value::int(at.0)),
        ("col", Value::int(at.1)),
        ("power", Value::Bool(true)),
        (
            "roomHealth",
            Value::obj(vec![
                ("damageValue", Value::int(0)),
                ("initialValue", Value::int(0)),
            ]),
        ),
        ("mrHandyList", Value::Array(vec![])),
        ("rushTask", Value::int(-1)),
        ("level", Value::int(level)),
        ("dwellers", Value::Array(vec![])),
        ("deadDwellers", Value::Array(vec![])),
        ("currentStateName", Value::str("Idle")),
        ("currentState", Value::obj(vec![])),
        ("deserializeID", Value::int(id)),
        ("assignedDecoration", Value::str("")),
        ("roomVisibility", Value::Bool(false)),
        ("roomOutline", Value::Bool(false)),
    ];
    match (kind, class) {
        ("Elevator", _) => fields.push(("withHole", Value::Bool(false))),
        ("LivingQuarters", _) => {
            fields.push(("children", Value::Array(vec![])));
            fields.push(("partners", Value::Array(vec![])));
        }
        ("Radio", _) => {
            fields.push(("newDwellerReady", Value::Bool(false)));
            fields.push(("onlyIncreaseHappiness", Value::Bool(true)));
        }
        (_, "Production" | "Consumable") => {
            fields.push((
                "storage",
                Value::obj(vec![("resources", zeros()), ("bonus", zeros())]),
            ));
            fields.push(("numberOfProductionCycle", Value::int(0)));
            fields.push(("ExperienceRewardIsDirty", Value::Bool(false)));
        }
        (_, "Training") => {
            fields.push(("slots", Value::Array(vec![])));
            fields.push(("dwellerWithPendingCompleteTraining", Value::Array(vec![])));
        }
        (_, "Crafting") => {
            fields.push(("IngredientItemIds", Value::Array(vec![])));
            fields.push(("CraftingItemId", Value::str("")));
            fields.push(("CompletedTime", Value::int(0)));
        }
        _ => {}
    }
    Value::obj(fields)
}

fn add_room(
    b: &mut Builder<'_>,
    mode: Mode,
    kind: &str,
    row: i64,
    col: i64,
    merge: i64,
    level: i64,
) -> R<i64> {
    let info = catalog().rooms.iter().find(|r| r.kind == kind);
    if mode == Mode::Normal {
        let info = info.ok_or(RuleError::NotInCatalog)?;
        if map::NOT_BUILDABLE.contains(&kind) {
            return Err(RuleError::NotAllowed.into());
        }
        let max_merge = if kind == "Elevator" {
            1
        } else {
            info.max_merge
        };
        rules::check_range(mode, merge, 1, max_merge)?;
        rules::check_range(mode, level, 1, info.levels)?;
        if map::UNIQUE_ROOMS.contains(&kind) && map::rooms(b.root()).iter().any(|r| r.kind == kind)
        {
            return Err(RuleError::RoomUnique.into());
        }
    }
    place(b.root(), mode, kind, row, col, merge, None)?;
    let class = info.map_or_else(|| "Facility".to_string(), |i| i.class.clone());
    let id = next_room_id(b)?;
    let room = new_room(b.root(), kind, &class, (row, col), merge, level, id);
    b.push(p(&ROOMS), room)?;
    Ok(id)
}

/// Task ids a room owns: any non-negative number under a key containing "task".
fn owned_tasks(v: &Value, out: &mut Vec<i64>) {
    match v {
        Value::Object(o) => {
            for (k, child) in &o.0 {
                if k.text().to_ascii_lowercase().contains("task") {
                    if let Some(id) = child.as_i64().filter(|id| *id >= 0) {
                        out.push(id);
                    }
                }
                owned_tasks(child, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| owned_tasks(x, out)),
        _ => {}
    }
}

fn remove_tasks(b: &mut Builder<'_>, ids: &[i64]) -> R<()> {
    if ids.is_empty() {
        return Ok(());
    }
    for list in ["tasks", "pausedTasks"] {
        let path = p(&["taskMgr", list]);
        while let Some(i) = b.get(&path).and_then(Value::as_array).and_then(|a| {
            a.iter().position(|t| {
                t.get("id")
                    .and_then(Value::as_i64)
                    .is_some_and(|id| ids.contains(&id))
            })
        }) {
            b.remove(path.clone(), i)?;
        }
    }
    Ok(())
}

/// Takes a dweller out of every room and training slot.
fn unassign(b: &mut Builder<'_>, id: i64) -> R<()> {
    drop_from_rooms(b, id, None)?;
    if let Ok(base) = dweller_path(b.root(), id) {
        b.set(join(&base, &["savedRoom"]), Value::int(-1))?;
        if b.get(&join(&base, &["assigned"])).is_some() {
            b.set(join(&base, &["assigned"]), Value::Bool(false))?;
        }
    }
    Ok(())
}

/// Removes a dweller from room lists and training slots, except in room `keep`.
fn drop_from_rooms(b: &mut Builder<'_>, id: i64, keep: Option<usize>) -> R<()> {
    let n = arr(b.root(), &ROOMS).len();
    for i in 0..n {
        if Some(i) == keep {
            continue;
        }
        remove_number(b, pi(&ROOMS, i, &["dwellers"]), id)?;
        remove_number(b, pi(&ROOMS, i, &["deadDwellers"]), id)?;
        remove_number(
            b,
            pi(&ROOMS, i, &["dwellerWithPendingCompleteTraining"]),
            id,
        )?;
        let slots = pi(&ROOMS, i, &["slots"]);
        while let Some(j) = b.get(&slots).and_then(Value::as_array).and_then(|a| {
            a.iter()
                .position(|s| s.get("dwellerID").and_then(Value::as_i64) == Some(id))
        }) {
            let task = b
                .get(&slots)
                .and_then(|s| s.at(&[Seg::Index(j), Seg::Key("taskID".into())]))
                .and_then(Value::as_i64);
            b.remove(slots.clone(), j)?;
            if let Some(t) = task {
                remove_tasks(b, &[t])?;
            }
        }
    }
    Ok(())
}

fn remove_room(b: &mut Builder<'_>, mode: Mode, id: i64) -> R<usize> {
    let i = room_index(b.root(), id).ok_or(RuleError::NotFound)?;
    let r = arr(b.root(), &ROOMS)[i].clone();
    let kind = text(&r, &["type"]).unwrap_or_default();
    if map::FIXED_ROOMS.contains(&kind.as_str()) {
        return Err(RuleError::RoomFixed.into());
    }
    if mode == Mode::Normal {
        if !map::can_remove(b.root(), id) {
            return Err(RuleError::RoomDisconnected.into());
        }
        if room_busy(&r) {
            return Err(RuleError::RoomBusy.into());
        }
    }
    let row = int(&r, &["row"]).unwrap_or(0);
    let people: Vec<i64> = ints(&r, "dwellers")
        .into_iter()
        .chain(ints(&r, "deadDwellers"))
        .collect();
    let mut tasks = Vec::new();
    owned_tasks(&r, &mut tasks);
    remove_tasks(b, &tasks)?;
    b.remove(p(&ROOMS), i)?;
    for d in &people {
        unassign(b, *d)?;
    }
    for handy in ints(&r, "mrHandyList") {
        rehome_handy(b, handy, row)?;
    }
    for list in [&ACTORS, &DWELLERS] {
        let n = arr(b.root(), list).len();
        for ai in 0..n {
            let path = pi(list, ai, &["savedRoom"]);
            if b.get(&path).and_then(Value::as_i64) == Some(id) {
                set_num(b, path, -1.0)?;
            }
        }
    }
    let happy = p(&["happinessManager"]);
    if b.get(&happy).is_some() {
        b.remove_key(happy.clone(), &format!("room{id}"))?;
        remove_number(b, join(&happy, &["rooms"]), id)?;
    }
    Ok(1 + people.len())
}

/// Rooms whose removal would lose game state: bodies, couples, babies or
/// an item being crafted.
pub fn room_busy(r: &Value) -> bool {
    let non_empty = |k: &str| {
        r.get(k)
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
    };
    non_empty("deadDwellers")
        || non_empty("partners")
        || non_empty("children")
        || text(r, &["CraftingItemId"]).is_some_and(|s| !s.is_empty())
}

/// Moves a Mr. Handy to another room, preferring the same floor.
fn rehome_handy(b: &mut Builder<'_>, handy: i64, row: i64) -> R<()> {
    let list = map::rooms(b.root());
    let usable =
        |r: &&map::Placed| !map::FIXED_ROOMS.contains(&r.kind.as_str()) && !r.is_elevator();
    let target = list
        .iter()
        .filter(usable)
        .find(|r| r.row == row)
        .or_else(|| {
            list.iter()
                .filter(usable)
                .min_by_key(|r| (r.row - row).abs())
        });
    let actor = arr(b.root(), &ACTORS)
        .iter()
        .position(|a| int(a, &["serializeId"]) == Some(handy));
    let target_id = match target {
        Some(t) => {
            let ti = room_index(b.root(), t.id).ok_or(RuleError::NotFound)?;
            b.push(pi(&ROOMS, ti, &["mrHandyList"]), Value::int(handy))?;
            t.id
        }
        None => -1,
    };
    if let Some(ai) = actor {
        set_num(b, pi(&ACTORS, ai, &["savedRoom"]), target_id as f64)?;
    }
    Ok(())
}

fn move_room(b: &mut Builder<'_>, mode: Mode, id: i64, row: i64, col: i64) -> R<()> {
    let i = room_index(b.root(), id).ok_or(RuleError::NotFound)?;
    let r = map::placed(&arr(b.root(), &ROOMS)[i]).ok_or(RuleError::NotFound)?;
    if map::FIXED_ROOMS.contains(&r.kind.as_str()) {
        return Err(RuleError::RoomFixed.into());
    }
    place(b.root(), mode, &r.kind, row, col, r.merge, Some(id))?;
    set_num(b, pi(&ROOMS, i, &["row"]), row as f64)?;
    set_num(b, pi(&ROOMS, i, &["col"]), col as f64)?;
    Ok(())
}

fn set_room_level(b: &mut Builder<'_>, mode: Mode, id: i64, level: i64) -> R<()> {
    let i = room_index(b.root(), id).ok_or(RuleError::NotFound)?;
    let kind = text(&arr(b.root(), &ROOMS)[i], &["type"]).unwrap_or_default();
    let max = catalog()
        .rooms
        .iter()
        .find(|r| r.kind == kind)
        .map_or(catalog().limits.room_level_max, |r| r.levels);
    rules::check_range(mode, level, 1, max)?;
    set_num(b, pi(&ROOMS, i, &["level"]), level as f64)?;
    Ok(())
}

fn ensure_strings(
    b: &mut Builder<'_>,
    path: Vec<Seg>,
    want: impl Iterator<Item = String>,
    existing_key: impl Fn(&str) -> Option<String>,
) -> R<usize> {
    let Some(Value::Array(cur)) = b.get(&path) else {
        return Ok(0);
    };
    let have: HashSet<String> = cur
        .iter()
        .filter_map(Value::as_text)
        .map(|s| existing_key(&s).unwrap_or(s))
        .collect();
    let mut list = cur.clone();
    let mut n = 0;
    for w in want {
        let key = existing_key(&w).unwrap_or_else(|| w.clone());
        if !have.contains(&key) {
            list.push(Value::str(&w));
            n += 1;
        }
    }
    if n > 0 {
        b.set(path, Value::Array(list))?;
    }
    Ok(n)
}

fn unlock_all_rooms(b: &mut Builder<'_>) -> R<usize> {
    let ids: Vec<String> = catalog().unlockables.iter().map(|u| u.id.clone()).collect();
    let base = p(&["unlockableMgr"]);
    if b.get(&base).is_none() {
        return Ok(0);
    }
    for list in ["objectivesInProgress", "completed"] {
        let lp = join(&base, &[list]);
        while let Some(i) = b.get(&lp).and_then(Value::as_array).and_then(|a| {
            a.iter().position(|e| {
                let id = e
                    .as_text()
                    .or_else(|| e.get("objectiveID").and_then(Value::as_text))
                    .unwrap_or_default();
                ids.contains(&id)
            })
        }) {
            b.remove(lp.clone(), i)?;
        }
    }
    ensure_strings(b, join(&base, &["claimed"]), ids.into_iter(), |_| None)
}

fn complete_collection(b: &mut Builder<'_>) -> R<usize> {
    let c = catalog();
    let strip = |s: &str| s.get(1..).map(str::to_string);
    let mut n = 0;
    let groups: Vec<(&str, Vec<String>)> = vec![
        (
            "weapons",
            c.weapons
                .iter()
                .filter(|w| !w.hidden)
                .map(|w| w.code.clone())
                .collect(),
        ),
        (
            "outfits",
            c.outfits
                .iter()
                .filter(|o| !o.hidden)
                .map(|o| o.code.clone())
                .collect(),
        ),
        (
            "junk",
            c.junk.iter().filter_map(|j| j.code.clone()).collect(),
        ),
        ("pets", c.pets.iter().map(|p| p.code.clone()).collect()),
        ("breeds", {
            let mut v: Vec<String> = c.pets.iter().map(|p| p.breed_code.to_string()).collect();
            v.sort();
            v.dedup();
            v
        }),
    ];
    for (key, codes) in groups {
        n += ensure_strings(
            b,
            p(&["survivalW", key]),
            codes.into_iter().map(|c| format!("N{c}")),
            strip,
        )?;
    }
    Ok(n)
}

fn complete_tutorial(b: &mut Builder<'_>) -> R<usize> {
    let base = p(&["tutorialManager"]);
    let Some(o) = b.get(&base).and_then(Value::as_object).cloned() else {
        return Ok(0);
    };
    let mut n = 0;
    for (k, v) in &o.0 {
        let key = k.text();
        let done = key.starts_with("Contextual")
            || key.ends_with("Completed")
            || key.ends_with("TutorialMessage") && !key.starts_with("showing")
            || key == "exploreWastelandMessageShown"
            || key == "scorchedAttackWarningShown";
        if !done {
            continue;
        }
        let new = match v {
            Value::Bool(_) => Value::Bool(true),
            Value::Number(num) if num.is_integer_literal() => Value::int(1),
            _ => continue,
        };
        if b.set(join(&base, &[&key]), new)? {
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        let mut root = json::parse(
            r#"{"taskMgr":{"id":1,"tasks":[],"pausedTasks":[]},"dwellers":{"dwellers":[],"actors":[],"id":0,"mrhId":1},
            "vault":{"rooms":[{"type":"Storage","class":"Facility","mergeLevel":1,"row":1,"col":0,"level":1,"dwellers":[],"deserializeID":1},
                              {"type":"Gym","class":"Training","mergeLevel":1,"row":1,"col":6,"level":1,"dwellers":[],"deserializeID":2}],
                     "rocks":[{"r":1,"c":12}],"storage":{"resources":{"Nuka":10,"Food":5.50},"bonus":{"Nuka":0}},"inventory":{"items":[]},
                     "LunchBoxesByType":[0,3],"LunchBoxDeckNames":["Normal","Normal"],"LunchBoxesCount":2},
            "happinessManager":{"rooms":[],"dwellers":[]}}"#,
        )
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        for name in ["A", "B"] {
            run(
                &mut root,
                &ctx,
                &Action::AddDweller {
                    first: name.into(),
                    last: "X".into(),
                    gender: 1,
                },
            )
            .unwrap();
        }
        root
    }

    #[test]
    fn mass_action_is_one_transaction_and_undoes_exactly() {
        let mut root = sample();
        let before = root.to_compact();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let tx = run(
            &mut root,
            &ctx,
            &Action::SetLevel {
                ids: vec![1, 2],
                level: 50,
            },
        )
        .unwrap();
        assert_eq!(tx.affected, 2);
        assert!(root.to_compact().contains("\"currentLevel\":50"));
        crate::edit::revert_all(&mut root, &tx.ops).unwrap();
        assert_eq!(root.to_compact(), before);
    }

    #[test]
    fn normal_mode_limits() {
        let mut root = sample();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        assert!(run(
            &mut root,
            &ctx,
            &Action::SetLevel {
                ids: vec![1],
                level: 51
            }
        )
        .is_err());
        assert!(run(
            &mut root,
            &ctx,
            &Action::SetSpecial {
                ids: vec![1],
                stat: 1,
                value: 11
            }
        )
        .is_err());
        assert!(run(
            &mut root,
            &ctx,
            &Action::AddItems {
                kind: ItemKind::Weapon,
                id: "NoSuchGun".into(),
                count: 1
            }
        )
        .is_err());
        let ex = Ctx {
            mode: Mode::Expert,
            original: &orig,
        };
        assert!(run(
            &mut root,
            &ex,
            &Action::SetSpecial {
                ids: vec![1],
                stat: 1,
                value: 11
            }
        )
        .is_ok());
        assert!(!rules::warnings(&root).is_empty());
    }

    #[test]
    fn delete_leaves_no_broken_refs() {
        let mut root = sample();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        run(
            &mut root,
            &ctx,
            &Action::AssignRoom {
                ids: vec![1, 2],
                room: 2,
            },
        )
        .unwrap();
        run(&mut root, &ctx, &Action::SetPartner { id: 1, partner: 2 }).unwrap();
        run(
            &mut root,
            &ctx,
            &Action::Equip {
                ids: vec![1],
                slot: Slot::Pet,
                item: Some("blacklab_c".into()),
            },
        )
        .unwrap();
        assert_eq!(arr(&root, &ACTORS).len(), 1);
        run(&mut root, &ctx, &Action::DeleteDwellers { ids: vec![1] }).unwrap();
        assert!(
            integrity::check(&root).is_empty(),
            "{:?}",
            integrity::check(&root)
        );
        assert_eq!(arr(&root, &ACTORS).len(), 0);
    }

    #[test]
    fn lunchboxes_and_merge_and_items() {
        let mut root = sample();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        run(
            &mut root,
            &ctx,
            &Action::SetLunchboxes {
                counts: [3, 1, 0, 0],
            },
        )
        .unwrap();
        assert_eq!(int(&root, &["vault", "LunchBoxesCount"]), Some(4));
        assert_eq!(arr(&root, &["vault", "LunchBoxDeckNames"]).len(), 4);
        assert!(run(
            &mut root,
            &ctx,
            &Action::SetMergeLevel { room: 1, merge: 2 }
        )
        .is_ok());
        assert!(run(
            &mut root,
            &ctx,
            &Action::SetMergeLevel { room: 1, merge: 3 }
        )
        .is_err());
        assert!(run(
            &mut root,
            &ctx,
            &Action::SetMergeLevel { room: 2, merge: 3 }
        )
        .is_err());
        run(
            &mut root,
            &ctx,
            &Action::AddItems {
                kind: ItemKind::Weapon,
                id: "Fist".into(),
                count: 3,
            },
        )
        .unwrap();
        run(
            &mut root,
            &ctx,
            &Action::RemoveItems {
                kind: ItemKind::Weapon,
                id: "Fist".into(),
                count: 2,
            },
        )
        .unwrap();
        assert_eq!(arr(&root, &ITEMS).len(), 1);
        run(
            &mut root,
            &ctx,
            &Action::SetResource {
                bonus: false,
                key: "Food".into(),
                value: 100.0,
            },
        )
        .unwrap();
        assert!(root.to_compact().contains("\"Food\":100.00"));
    }

    fn vault_with_lift() -> Value {
        let mut root = json::parse(
            r#"{"taskMgr":{"id":9,"tasks":[{"id":7}],"pausedTasks":[]},"dwellers":{"dwellers":[],"actors":[],"id":0,"mrhId":1},
            "vault":{"rooms":[
              {"type":"Entrance","class":"Facility","mergeLevel":2,"row":0,"col":3,"level":1,"dwellers":[],"deadDwellers":[],"mrHandyList":[],"deserializeID":1},
              {"type":"Elevator","class":"Utility","mergeLevel":1,"row":0,"col":9,"level":1,"dwellers":[],"deadDwellers":[],"mrHandyList":[],"deserializeID":2},
              {"type":"Elevator","class":"Utility","mergeLevel":1,"row":1,"col":9,"level":1,"dwellers":[],"deadDwellers":[],"mrHandyList":[],"deserializeID":3},
              {"type":"LivingQuarters","class":"Facility","mergeLevel":1,"row":1,"col":10,"level":1,"dwellers":[],"deadDwellers":[],"mrHandyList":[],"deserializeID":4,"children":[],"partners":[]},
              {"type":"Gym","class":"Training","mergeLevel":1,"row":1,"col":13,"level":1,"dwellers":[],"deadDwellers":[],"mrHandyList":[],"deserializeID":5,"slots":[],"dwellerWithPendingCompleteTraining":[]}],
              "rocks":[{"r":1,"c":20}],"storage":{"resources":{"Nuka":10,"Food":5}},"inventory":{"items":[]}},
            "constructMgr":{"roomDeserializeID":5},
            "happinessManager":{"room5":[],"rooms":[5],"dwellers":[]}}"#,
        )
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Expert,
            original: &orig,
        };
        for name in ["A", "B"] {
            run(
                &mut root,
                &ctx,
                &Action::AddDweller {
                    first: name.into(),
                    last: "X".into(),
                    gender: 1,
                },
            )
            .unwrap();
        }
        run(
            &mut root,
            &ctx,
            &Action::AssignRoom {
                ids: vec![1],
                room: 4,
            },
        )
        .unwrap();
        run(
            &mut root,
            &ctx,
            &Action::AssignRoom {
                ids: vec![2],
                room: 5,
            },
        )
        .unwrap();
        run(
            &mut root,
            &ctx,
            &Action::SetRaw {
                path: pi(&ROOMS, 4, &["slots"]),
                json: r#"[{"needLvUp":false,"dwellerID":2,"taskID":7}]"#.into(),
            },
        )
        .unwrap();
        root
    }

    #[test]
    fn room_editing_keeps_vault_consistent() {
        let mut root = vault_with_lift();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let add = |kind: &str, row, col| Action::AddRoom {
            kind: kind.into(),
            row,
            col,
            merge: 1,
            level: 1,
        };
        let err = |r: R<Transaction>| match r {
            Err(ActionError::Rule(e)) => e,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            err(run(&mut root, &ctx, &add("Storage", 2, 10))),
            RuleError::RoomDisconnected
        );
        assert_eq!(
            err(run(&mut root, &ctx, &add("Storage", 1, 12))),
            RuleError::RoomOverlap
        );
        assert_eq!(
            err(run(&mut root, &ctx, &add("Storage", 1, 19))),
            RuleError::RoomOverlap
        );
        assert_eq!(
            err(run(&mut root, &ctx, &add("Entrance", 1, 16))),
            RuleError::NotAllowed
        );
        run(&mut root, &ctx, &add("Storage", 1, 16)).unwrap();
        assert_eq!(int(&root, &["constructMgr", "roomDeserializeID"]), Some(6));
        let last = arr(&root, &ROOMS).last().cloned().unwrap();
        assert_eq!(int(&last, &["deserializeID"]), Some(6));
        assert!(last.get("storage").is_none());
        run(&mut root, &ctx, &add("Overseer", 1, 0)).unwrap_err();
        run(&mut root, &ctx, &add("Elevator", 2, 9)).unwrap();
        run(&mut root, &ctx, &add("Overseer", 2, 10)).unwrap();
        assert_eq!(
            err(run(&mut root, &ctx, &add("Overseer", 2, 3))),
            RuleError::RoomUnique
        );

        assert_eq!(
            err(run(&mut root, &ctx, &Action::RemoveRoom { room: 3 })),
            RuleError::RoomDisconnected
        );
        assert_eq!(
            err(run(&mut root, &ctx, &Action::RemoveRoom { room: 1 })),
            RuleError::RoomFixed
        );
        assert_eq!(
            err(run(
                &mut root,
                &ctx,
                &Action::MoveRoom {
                    room: 6,
                    row: 3,
                    col: 10
                }
            )),
            RuleError::RoomDisconnected
        );
        run(
            &mut root,
            &ctx,
            &Action::MoveRoom {
                room: 6,
                row: 2,
                col: 16,
            },
        )
        .unwrap_err();
        run(
            &mut root,
            &ctx,
            &Action::MoveRoom {
                room: 6,
                row: 2,
                col: 13,
            },
        )
        .unwrap();

        let tx = run(&mut root, &ctx, &Action::RemoveRoom { room: 5 }).unwrap();
        assert_eq!(tx.affected, 2);
        assert!(room_index(&root, 5).is_none());
        assert!(arr(&root, &["taskMgr", "tasks"]).is_empty());
        assert!(root.at(&p(&["happinessManager", "room5"])).is_none());
        assert!(arr(&root, &["happinessManager", "rooms"]).is_empty());
        let d = &arr(&root, &DWELLERS)[dweller_index(&root, 2).unwrap()];
        assert_eq!(int(d, &["savedRoom"]), Some(-1));
        assert_eq!(arr(&root, &DWELLERS).len(), 2);
        assert!(
            integrity::check(&root).is_empty(),
            "{:?}",
            integrity::check(&root)
        );

        run(&mut root, &ctx, &Action::SetRoomLevel { room: 4, level: 3 }).unwrap();
        run(&mut root, &ctx, &Action::SetRoomLevel { room: 4, level: 4 }).unwrap_err();
    }
}
