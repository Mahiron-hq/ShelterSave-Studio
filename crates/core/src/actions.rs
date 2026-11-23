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
        /// Send a wasteland team of a revived dweller back to the vault.
        #[serde(default)]
        home: bool,
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
    /// Dweller copied from another save (its JSON), with ties to that vault cut.
    ImportDweller {
        json: String,
    },
    /// Lets a quest be offered again: removes it from a history or skipped list
    /// (`list` from `QUEST_MEMORY`); an empty `quest` clears the list.
    ForgetQuest {
        list: String,
        quest: String,
    },
    /// Hair or face mask (beard, glasses...) by its game name; empty removes it.
    SetLook {
        id: i64,
        field: String,
        value: String,
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
        /// Expert mode: do not merge with equal neighbours.
        #[serde(default)]
        apart: bool,
    },
    /// Splits a merged room into rooms of the given sizes, left to right.
    SplitRoom {
        room: i64,
        parts: Vec<i64>,
    },
    AddRoom {
        kind: String,
        row: i64,
        col: i64,
        merge: i64,
        level: i64,
        /// Also unlocks the room in the blueprint book (the game crashes on built locked rooms).
        #[serde(default)]
        unlock: bool,
        /// Expert mode: do not merge with equal neighbours.
        #[serde(default)]
        apart: bool,
    },
    RemoveRoom {
        room: i64,
    },
    MoveRoom {
        room: i64,
        row: i64,
        col: i64,
        /// Expert mode: do not merge with equal neighbours.
        #[serde(default)]
        apart: bool,
    },
    SetRoomLevel {
        room: i64,
        level: i64,
        /// Expert mode: do not merge with equal neighbours.
        #[serde(default)]
        apart: bool,
    },
    RemoveRock {
        index: usize,
    },
    AddRock {
        row: i64,
        col: i64,
    },
    RemoveAllRocks,
    UnlockAllRooms,
    UnlockAllRecipes,
    CompleteCollection,
    FinishEmergency,
    CompleteTutorial,
    SetHints {
        shown: bool,
    },
    CompleteObjective {
        slot: usize,
    },
    MarkQuest {
        id: String,
        done: bool,
    },
    ReplaceObjective {
        slot: usize,
        id: String,
        level: i64,
    },
    /// Daily (`daily`), weekly (`weekly`) or standalone (`standalone`) quest offered now.
    ReplaceQuest {
        kind: String,
        index: usize,
        id: String,
    },
    SetAchievement {
        id: String,
        done: bool,
    },
    /// Mysterious Stranger: next visit after `seconds` (while he is away),
    /// or time left in the vault (while he is there).
    SetStrangerTime {
        seconds: f64,
    },
    /// Daily vault report: one day's rating or reward flag; the week rating follows like in the game.
    SetDayRating {
        index: usize,
        rating: Option<i64>,
        collected: Option<bool>,
    },
    /// Moves a game timer so it fires after `seconds`.
    SetTaskTime {
        id: i64,
        seconds: f64,
    },
    FixIssues,
    /// Blueprint book: room type unlocked or not.
    SetRoomUnlocked {
        room: String,
        unlocked: bool,
    },
    /// Crafting recipe known or not.
    SetRecipe {
        id: String,
        known: bool,
    },
    /// Clears the "new" marks in the journal and on room themes.
    MarkCollectionRead,
    /// Look of every room of a type; `None` restores the standard one.
    SetRoomTheme {
        room: String,
        theme: Option<String>,
    },
    /// Theme parts found (0..=9) and whether the theme is crafted.
    SetThemeState {
        id: String,
        parts: i64,
        crafted: bool,
    },
    /// Ready-to-collect amount in a production room.
    SetStored {
        room: i64,
        resource: String,
        value: f64,
    },
    AddTeamItems {
        team: usize,
        kind: ItemKind,
        id: String,
        count: i64,
        #[serde(default)]
        recipe: bool,
    },
    RemoveTeamItems {
        team: usize,
        indices: Vec<usize>,
    },
    SetTeamThemeParts {
        team: usize,
        id: String,
        parts: i64,
    },
    /// Room incident (fire, radroaches, molerats, radscorpions) with damage share `strength`.
    StartIncident {
        room: i64,
        kind: i64,
        strength: f64,
    },
    /// Room damage share for a room incident, raider health for a raid (`room` = None).
    SetIncidentStrength {
        room: Option<i64>,
        value: f64,
    },
    /// Legendary dweller from the journal, as the game adds one.
    AddUniqueDweller {
        id: String,
    },
    /// Pet with its journal entry.
    AddPet {
        id: String,
        bonus_value: f64,
    },
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
            action: serde_json::to_value(action).ok(),
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
        Action::Revive { ids, home } => {
            let mut n = for_dwellers(b, ids, revive)?;
            n += wake_teams(b, ids, *home)?;
            Ok((Label::new("act.revive"), n.min(ids.len().max(1))))
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
                let mut value = argb;
                // The game replaces any other outfit colour with the nearest allowed one.
                if field == "outfitColor" && mode == Mode::Normal {
                    let d = b.get(base).cloned().unwrap_or(Value::Null);
                    let outfit = d
                        .at(&p(&["equipedOutfit", "id"]))
                        .and_then(Value::as_text)
                        .unwrap_or_else(|| "jumpsuit".into());
                    if let Some(o) =
                        catalog().outfit_art(&outfit, int(&d, &["gender"]).unwrap_or(1))
                    {
                        value = o.game_color(argb);
                    }
                }
                Ok(b.set(join(base, &[field]), Value::int(i64::from(value)))?)
            })?;
            Ok((Label::new("act.setColor"), n))
        }
        Action::ImportDweller { json } => {
            let name = import_dweller(b, mode, json)?;
            Ok((Label::new("act.importDweller").with("name", name), 1))
        }
        Action::ForgetQuest { list, quest } => {
            let path = QUEST_MEMORY
                .iter()
                .find(|(k, _)| k == list)
                .map(|(_, p)| p.to_vec())
                .ok_or(RuleError::NotFound)?;
            let path = p(&path);
            let mut n = 0;
            while let Some(i) = b.get(&path).and_then(Value::as_array).and_then(|a| {
                a.iter().position(|e| {
                    quest.is_empty()
                        || e.get("questName").and_then(Value::as_text).as_deref() == Some(quest)
                })
            }) {
                b.remove(path.clone(), i)?;
                n += 1;
            }
            Ok((Label::new("act.forgetQuest").with("id", quest), n))
        }
        Action::SetLook { id, field, value } => {
            set_look(b, mode, *id, field, value)?;
            Ok((Label::new("act.setLook"), 1))
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
        Action::SetMergeLevel { room, merge, apart } => {
            set_merge(b, mode, *room, *merge)?;
            auto_merge(b, mode, *room, *apart)?;
            Ok((Label::new("act.mergeLevel").with("merge", merge), 1))
        }
        Action::SplitRoom { room, parts } => {
            let kind = room_kind(b.root(), *room)?;
            let n = split_room(b, mode, *room, parts)?;
            Ok((Label::new("act.splitRoom").with("room", kind), n))
        }
        Action::AddRoom {
            kind,
            row,
            col,
            merge,
            level,
            unlock,
            apart,
        } => {
            if *unlock && room_lock(b.root(), kind).is_some() {
                set_room_unlocked(b, mode, kind, true)?;
            }
            if mode == Mode::Normal && room_lock(b.root(), kind).is_some() {
                return Err(RuleError::RoomLocked.into());
            }
            let id = add_room(b, mode, kind, *row, *col, *merge, *level)?;
            auto_merge(b, mode, id, *apart)?;
            Ok((Label::new("act.addRoom").with("room", kind), 1))
        }
        Action::RemoveRoom { room } => {
            let kind = room_kind(b.root(), *room)?;
            let n = remove_room(b, mode, *room)?;
            Ok((Label::new("act.removeRoom").with("room", kind), n))
        }
        Action::MoveRoom {
            room,
            row,
            col,
            apart,
        } => {
            let kind = room_kind(b.root(), *room)?;
            move_room(b, mode, *room, *row, *col)?;
            auto_merge(b, mode, *room, *apart)?;
            Ok((Label::new("act.moveRoom").with("room", kind), 1))
        }
        Action::SetRoomLevel { room, level, apart } => {
            set_room_level(b, mode, *room, *level)?;
            auto_merge(b, mode, *room, *apart)?;
            Ok((Label::new("act.roomLevel").with("level", level), 1))
        }
        Action::RemoveRock { index } => {
            b.remove(p(&["vault", "rocks"]), *index)?;
            Ok((Label::new("act.removeRock"), 1))
        }
        Action::AddRock { row, col } => {
            map::fits(b.root(), *row, *col, 2, None).map_err(place_error)?;
            let rock = Value::obj(vec![("r", Value::int(*row)), ("c", Value::int(*col))]);
            b.push(p(&["vault", "rocks"]), rock)?;
            Ok((Label::new("act.addRock"), 1))
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
            let n = finish_emergency(b)?;
            Ok((Label::new("act.finishEmergency"), n.min(1)))
        }
        Action::CompleteTutorial => {
            let n = complete_tutorial(b)?;
            Ok((Label::new("act.completeTutorial"), n))
        }
        Action::SetHints { shown } => {
            let n = set_hints(b, *shown)?;
            Ok((Label::new("act.setHints"), n))
        }
        Action::CompleteObjective { slot } => {
            complete_objective(b, *slot)?;
            Ok((Label::new("act.completeObjective"), 1))
        }
        Action::MarkQuest { id, done } => {
            mark_quest(b, mode, id, *done)?;
            Ok((Label::new("act.markQuest").with("id", id.clone()), 1))
        }
        Action::ReplaceQuest { kind, index, id } => {
            replace_quest(b, mode, kind, *index, id)?;
            Ok((Label::new("act.replaceQuest").with("id", id.clone()), 1))
        }
        Action::ReplaceObjective { slot, id, level } => {
            replace_objective(b, mode, *slot, id, *level)?;
            Ok((Label::new("act.replaceObjective").with("id", id.clone()), 1))
        }
        Action::SetStrangerTime { seconds } => {
            set_stranger_time(b, *seconds)?;
            Ok((Label::new("act.setStrangerTime"), 1))
        }
        Action::SetDayRating {
            index,
            rating,
            collected,
        } => {
            set_day_rating(b, *index, *rating, *collected)?;
            Ok((
                Label::new("act.setDayRating").with("n", (index + 1).to_string()),
                1,
            ))
        }
        Action::SetTaskTime { id, seconds } => {
            set_task_time(b, *id, *seconds)?;
            Ok((Label::new("act.setTaskTime"), 1))
        }
        Action::SetAchievement { id, done } => {
            set_achievement(b, mode, id, *done)?;
            Ok((Label::new("act.setAchievement").with("id", id.clone()), 1))
        }
        Action::FixIssues => {
            let n = integrity::fix_all(b)?;
            Ok((Label::new("act.fixIssues").with("count", n), n))
        }
        Action::SetRoomUnlocked { room, unlocked } => {
            let n = set_room_unlocked(b, mode, room, *unlocked)?;
            let key = if *unlocked {
                "act.unlockRoom"
            } else {
                "act.lockRoom"
            };
            Ok((Label::new(key).with("room", room), n))
        }
        Action::SetRecipe { id, known } => {
            let n = set_recipe(b, mode, id, *known)?;
            let key = if *known {
                "act.learnRecipe"
            } else {
                "act.forgetRecipe"
            };
            Ok((Label::new(key).with("id", id), n))
        }
        Action::MarkCollectionRead => {
            let n = mark_collection_read(b)?;
            Ok((Label::new("act.markRead").with("count", n), n))
        }
        Action::SetRoomTheme { room, theme } => {
            let n = set_room_theme(b, mode, room, theme.as_deref())?;
            Ok((Label::new("act.roomTheme").with("room", room), n))
        }
        Action::SetThemeState { id, parts, crafted } => {
            let n = set_theme_state(b, mode, id, *parts, *crafted)?;
            Ok((Label::new("act.themeState").with("id", id), n))
        }
        Action::SetStored {
            room,
            resource,
            value,
        } => {
            let n = set_stored(b, mode, *room, resource, *value)?;
            Ok((
                Label::new("act.setStored")
                    .with("key", resource)
                    .with("value", value),
                usize::from(n),
            ))
        }
        Action::AddTeamItems {
            team,
            kind,
            id,
            count,
            recipe,
        } => {
            let n = add_team_items(b, mode, *team, *kind, id, *count, *recipe)?;
            Ok((
                Label::new("act.addTeamItems")
                    .with("id", id)
                    .with("count", n),
                n,
            ))
        }
        Action::RemoveTeamItems { team, indices } => {
            let n = remove_team_items(b, *team, indices)?;
            Ok((Label::new("act.removeTeamItems").with("count", n), n))
        }
        Action::SetTeamThemeParts { team, id, parts } => {
            let n = set_team_theme_parts(b, mode, *team, id, *parts)?;
            Ok((Label::new("act.teamThemeParts").with("id", id), n))
        }
        Action::StartIncident {
            room,
            kind,
            strength,
        } => {
            start_incident(b, mode, *room, *kind, *strength)?;
            Ok((Label::new("act.startIncident"), 1))
        }
        Action::SetIncidentStrength { room, value } => {
            let n = set_incident_strength(b, mode, *room, *value)?;
            Ok((Label::new("act.incidentStrength"), usize::from(n)))
        }
        Action::AddUniqueDweller { id } => {
            let name = add_unique_dweller(b, mode, id)?;
            Ok((Label::new("act.addDweller").with("name", name), 1))
        }
        Action::AddPet { id, bonus_value } => {
            add_pet(b, mode, id, *bonus_value)?;
            Ok((
                Label::new("act.addItems").with("id", id).with("count", 1),
                1,
            ))
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
        (_, "childRarity") => Some((1, 3)),
        (_, "rating" | "weekRating" | "lastWeekRating") => Some((0, 100)),
        (_, "pendingChildren") => Some((0, 3)),
        ("childStat", "") => Some((1, 7)),
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

/// Lists that keep a quest from being offered again, by name.
pub const QUEST_MEMORY: [(&str, [&str; 3]); 5] = [
    (
        "dailyHistory",
        [
            "completedQuestDataManager",
            "dailyQuestPicker",
            "historyDailies",
        ],
    ),
    (
        "weeklyHistory",
        [
            "completedQuestDataManager",
            "weeklyQuestPicker",
            "historyWeeklies",
        ],
    ),
    (
        "dailySkipped",
        [
            "completedQuestDataManager",
            "dailyQuestSkipper",
            "skippedQuests",
        ],
    ),
    (
        "weeklySkipped",
        [
            "completedQuestDataManager",
            "weeklyQuestSkipper",
            "skippedQuests",
        ],
    ),
    (
        "standaloneSkipped",
        [
            "completedQuestDataManager",
            "standaloneQuestSkipper",
            "skippedQuests",
        ],
    ),
];

/// Hair and face mask names the game has for a dweller of this gender.
pub fn look_choices(gender: i64, field: &str) -> Vec<&'static str> {
    let key = if gender == 2 { "male" } else { "female" };
    if field == "helmet" {
        return catalog()
            .art
            .as_ref()
            .and_then(|a| a.dweller.get(key))
            .map(|k| k.helmets.keys().map(String::as_str).collect())
            .unwrap_or_default();
    }
    let Some(l) = catalog().looks.get(key) else {
        return Vec::new();
    };
    match field {
        "hair" => l.hairs.iter().map(String::as_str).collect(),
        "faceMask" => l
            .beards
            .iter()
            .chain(&l.masks)
            .map(String::as_str)
            .collect(),
        _ => Vec::new(),
    }
}

fn set_look(b: &mut Builder<'_>, mode: Mode, id: i64, field: &str, value: &str) -> R<()> {
    if !matches!(field, "hair" | "faceMask" | "helmet") {
        return Err(RuleError::NotAllowed.into());
    }
    let base = dweller_path(b.root(), id)?;
    let gender = b.get(&base).and_then(|d| int(d, &["gender"])).unwrap_or(1);
    if mode == Mode::Normal && !value.is_empty() && !look_choices(gender, field).contains(&value) {
        return Err(RuleError::NotInCatalog.into());
    }
    let has = b.get(&join(&base, &[field])).is_some();
    if value.is_empty() {
        b.remove_key(base.clone(), field)?;
    } else if has {
        b.set(join(&base, &[field]), Value::str(value))?;
    } else {
        // Same place as the game writes it: hair, faceMask, helmet.
        let o = b
            .get(&base)
            .and_then(Value::as_object)
            .ok_or(RuleError::NotFound)?;
        let before: &[&str] = match field {
            "helmet" => &["faceMask", "hair", "pendingExperienceReward", "outfitColor"],
            "faceMask" => &["hair", "pendingExperienceReward", "outfitColor"],
            _ => &["pendingExperienceReward", "outfitColor"],
        };
        let at = before
            .iter()
            .find_map(|k| o.index_of(k))
            .map_or(o.len(), |i| i + 1);
        b.insert(base.clone(), at, Some(field), Value::str(value))?;
    }
    // The barber shop offers a hat or a face mask, never both.
    if !value.is_empty() && mode == Mode::Normal {
        match field {
            "helmet" => {
                b.remove_key(base, "faceMask")?;
            }
            "faceMask" => {
                b.remove_key(base, "helmet")?;
            }
            _ => {}
        }
    }
    Ok(())
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

/// Teams holding revived dwellers: a dead team explores again (as after a revive
/// in the game), or every such team is recalled to the vault when `home` is set.
fn wake_teams(b: &mut Builder<'_>, ids: &[i64], home: bool) -> R<usize> {
    const TEAMS: [&str; 3] = ["vault", "wasteland", "teams"];
    let snapshot = b.root().clone();
    let mut n = 0;
    for (t, team) in arr(&snapshot, &TEAMS).iter().enumerate() {
        let members = team.get("dwellers").and_then(Value::as_array);
        if !members.is_some_and(|m| {
            m.iter()
                .any(|v| v.as_i64().is_some_and(|id| ids.contains(&id)))
        }) {
            continue;
        }
        let status = text(team, &["status"]).unwrap_or_default();
        let dead = status == "Dead" || status == "DeadOnQuest";
        let away = matches!(
            status.as_str(),
            "Exploring" | "GoingToQuest" | "ResumingSurpriseQuest" | "WaitingOnSurpriseQuest"
        );
        let base = pi(&TEAMS, t, &[]);
        let mut changed = false;
        if home && (dead || away) {
            let explored = team
                .get("elapsedTimeAliveExploring")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            changed |= b.set(join(&base, &["status"]), Value::str("ReturningToVault"))?;
            changed |= set_num(b, join(&base, &["elapsedReturningTime"]), 0.0)?;
            changed |= set_num(
                b,
                join(&base, &["returnTripDuration"]),
                (explored / 2.0).floor(),
            )?;
            if dead && team.get("isForceReturningToVault").is_some() {
                changed |= b.set(join(&base, &["isForceReturningToVault"]), Value::Bool(true))?;
            }
        } else if dead {
            changed |= b.set(join(&base, &["status"]), Value::str("Exploring"))?;
        }
        if changed {
            n += 1;
        }
    }
    Ok(n)
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
    let mut wrong_gender = 0;
    for owner in ids {
        let Some(i) = dweller_index(b.root(), *owner) else {
            continue;
        };
        let base = pi(&DWELLERS, i, &[]);
        let gender = int(&arr(b.root(), &DWELLERS)[i], &["gender"]).unwrap_or(1);
        if let (Slot::Outfit, Some(id), Mode::Normal) = (slot, item, mode) {
            if !catalog().outfit_fits(id, gender) {
                wrong_gender += 1;
                continue;
            }
        }
        let old_outfit = b.get(&join(&base, &[key, "id"])).and_then(Value::as_text);
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
        // A new outfit gets one of its own colours, as the game does when equipping.
        if let (Slot::Outfit, true) = (slot, changed) {
            let new_id = item.unwrap_or("jumpsuit");
            if old_outfit.as_deref() != Some(new_id)
                && b.get(&join(&base, &["outfitColor"])).is_some()
            {
                let color = catalog()
                    .outfit_art(new_id, gender)
                    .and_then(|o| o.colors.first().copied())
                    .unwrap_or(0xFFFF_FFFF);
                b.set(join(&base, &["outfitColor"]), Value::int(i64::from(color)))?;
            }
        }
        if changed {
            n += 1;
        }
    }
    if n == 0 && wrong_gender > 0 {
        return Err(RuleError::OutfitGender.into());
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
    // Like Dweller.Evict: worn items go back to storage unless the dweller is away with a team.
    let on_quest = arr(
        &snapshot,
        &["questDataManager", "questTeam", "DwellersDictionary"],
    )
    .iter()
    .any(|v| v.as_i64() == Some(id));
    if team_of(&snapshot, id).is_none() && !on_quest && b.get(&p(&ITEMS)).is_some() {
        let d = &arr(&snapshot, &DWELLERS)[idx];
        for key in ["equipedOutfit", "equipedWeapon", "equippedPet"] {
            let Some(item) = d.get(key) else { continue };
            let item_id = text(item, &["id"]).unwrap_or_default();
            let keep = match key {
                "equipedOutfit" => {
                    item_id != "jumpsuit"
                        && catalog().outfit(&item_id).is_none_or(|o| o.category != 1)
                }
                "equipedWeapon" => item_id != "Fist",
                _ => !item_id.is_empty(),
            };
            if keep {
                b.push(p(&ITEMS), item.clone())?;
            }
        }
    }
    drop_from_rooms(b, id, None)?;
    for i in 0..arr(b.root(), &ROOMS).len() {
        let lp = pi(&ROOMS, i, &["partners"]);
        // A pregnancy outlives the father (DwellerPartnership.OnDwellerRemoved); without the mother it ends.
        while let Some(k) = b.get(&lp).and_then(Value::as_array).and_then(|a| {
            a.iter().position(|e| {
                text(e, &["s"]).as_deref() == Some("RaisingBaby") && int(e, &["f"]) == Some(id)
            })
        }) {
            let task = b
                .get(&lp)
                .and_then(|a| a.at(&[Seg::Index(k)]))
                .and_then(|e| int(e, &["t"]));
            b.remove(lp.clone(), k)?;
            remove_tasks(b, &task.into_iter().filter(|t| *t >= 0).collect::<Vec<_>>())?;
        }
        let n = b.get(&lp).and_then(Value::as_array).map_or(0, Vec::len);
        for k in 0..n {
            for key in ["m", "fatherId"] {
                let path = pi(&ROOMS, i, &["partners"])
                    .into_iter()
                    .chain([Seg::Index(k), Seg::Key(key.into())])
                    .collect::<Vec<_>>();
                if b.get(&path).and_then(Value::as_i64) == Some(id) {
                    b.set(path, Value::int(-1))?;
                }
            }
        }
        let cp = pi(&ROOMS, i, &["children"]);
        while let Some(k) = b
            .get(&cp)
            .and_then(Value::as_array)
            .and_then(|a| a.iter().position(|e| int(e, &["dwellerID"]) == Some(id)))
        {
            let task = b
                .get(&cp)
                .and_then(|a| a.at(&[Seg::Index(k)]))
                .and_then(|e| int(e, &["taskID"]));
            b.remove(cp.clone(), k)?;
            remove_tasks(b, &task.into_iter().filter(|t| *t >= 0).collect::<Vec<_>>())?;
        }
    }
    for (i, d) in arr(&snapshot, &DWELLERS).iter().enumerate() {
        for key in ["partner", "lastPartner"] {
            if int(d, &["relations", key]) == Some(id) {
                b.set(pi(&DWELLERS, i, &["relations", key]), Value::int(-1))?;
            }
        }
        let rl = pi(&DWELLERS, i, &["relations", "relations"]);
        while let Some(k) = b
            .get(&rl)
            .and_then(Value::as_array)
            .and_then(|a| a.iter().position(|e| int(e, &["dweller"]) == Some(id)))
        {
            b.remove(rl.clone(), k)?;
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
    let away = p(&["questDwellers", "dwellers"]);
    while let Some(k) = b
        .get(&away)
        .and_then(Value::as_array)
        .and_then(|a| a.iter().position(|e| int(e, &["serializeId"]) == Some(id)))
    {
        b.remove(away.clone(), k)?;
    }
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
    if mode == Mode::Normal && target.is_some() {
        let quest = arr(
            b.root(),
            &["questDataManager", "questTeam", "DwellersDictionary"],
        );
        if ids.iter().any(|id| {
            team_of(b.root(), *id).is_some() || quest.iter().any(|v| v.as_i64() == Some(*id))
        }) {
            return Err(RuleError::DwellerAway.into());
        }
    }
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
        } else {
            b.set(join(&base, &["savedRoom"]), Value::int(-1))?;
        }
        if b.get(&join(&base, &["assigned"])).is_some() {
            b.set(join(&base, &["assigned"]), Value::Bool(target.is_some()))?;
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
pub fn merge_fits(root: &Value, mode: Mode, room: i64, merge: i64) -> bool {
    merge_col(root, mode, room, merge).is_some()
}

/// Column the room starts at after resizing to `merge`, or None when no
/// placement keeps it free of overlaps (and connected, in normal mode).
pub fn merge_col(root: &Value, mode: Mode, room: i64, merge: i64) -> Option<i64> {
    let i = room_index(root, room)?;
    let r = &arr(root, &ROOMS)[i];
    let kind = text(r, &["type"]).unwrap_or_default();
    let (row, col) = (int(r, &["row"])?, int(r, &["col"])?);
    let cur = int(r, &["mergeLevel"]).unwrap_or(1);
    let delta = map::width_of(&kind, merge) - map::width_of(&kind, cur);
    // Growing: right, half each way, left. Shrinking: keep the left edge,
    // the middle, then the right edge, so the side touching a lift can stay.
    let mut cols = vec![col];
    if delta.abs() == 6 {
        cols.push(col - delta.signum() * 3);
    }
    if delta != 0 {
        cols.push(col - delta);
    }
    cols.into_iter()
        .find(|c| place(root, mode, &kind, row, *c, merge, Some(room)).is_ok())
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
    rules::check_room_shape(mode, &kind, merge, int(&r, &["level"]).unwrap_or(1))?;
    let (row, col) = (
        int(&r, &["row"]).unwrap_or(0),
        int(&r, &["col"]).unwrap_or(0),
    );
    let Some(new_col) = merge_col(b.root(), mode, room, merge) else {
        place(b.root(), mode, &kind, row, col, merge, Some(room))?;
        return Err(RuleError::RoomOverlap.into());
    };
    if new_col != col {
        set_num(b, pi(&ROOMS, i, &["col"]), new_col as f64)?;
    }
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
        rules::check_room_shape(mode, kind, merge, level)?;
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

/// Like LivingQuartersRoom.OnRemoveDweller: a couple that has no baby yet breaks up
/// when one of them leaves the room; their task goes and both lose their partner.
fn end_partnerships(b: &mut Builder<'_>, room: usize, id: i64) -> R<()> {
    let lp = pi(&ROOMS, room, &["partners"]);
    while let Some((k, e)) = b.get(&lp).and_then(Value::as_array).and_then(|a| {
        a.iter().enumerate().find_map(|(k, e)| {
            let s = text(e, &["s"]).unwrap_or_default();
            let ours = int(e, &["m"]) == Some(id) || int(e, &["f"]) == Some(id);
            (ours && s != "RaisingBaby" && s != "Ended").then(|| (k, e.clone()))
        })
    }) {
        b.remove(lp.clone(), k)?;
        if let Some(t) = int(&e, &["t"]).filter(|t| *t >= 0) {
            remove_tasks(b, &[t])?;
        }
        for who in [int(&e, &["m"]), int(&e, &["f"])].into_iter().flatten() {
            let Ok(base) = dweller_path(b.root(), who) else {
                continue;
            };
            if b.get(&join(&base, &["relations", "partner"])).is_some() {
                b.set(join(&base, &["relations", "partner"]), Value::int(-1))?;
            }
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
        end_partnerships(b, i, id)?;
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

/// Merges a room with equal neighbours the way the game does after a build,
/// a move or an upgrade. Returns the id of the room that remains.
fn auto_merge(b: &mut Builder<'_>, mode: Mode, id: i64, apart: bool) -> R<i64> {
    if apart && mode == Mode::Expert {
        return Ok(id);
    }
    let mut id = id;
    while let Some(group) = merge_group(b.root(), id) {
        id = merge_rooms(b, &group)?;
    }
    Ok(id)
}

/// The room and the neighbours the game would merge it with (left first, then right).
pub fn merge_group(root: &Value, id: i64) -> Option<Vec<i64>> {
    let list = map::rooms(root);
    let t = list.iter().find(|r| r.id == id)?;
    if t.is_elevator() || map::FIXED_ROOMS.contains(&t.kind.as_str()) {
        return None;
    }
    let max = rules::max_merge(&t.kind).min(3);
    let calm = |rid: i64| {
        room_index(root, rid).is_some_and(|i| {
            text(&arr(root, &ROOMS)[i], &["currentStateName"]).as_deref() != Some("RoomEmergency")
        })
    };
    if !calm(t.id) {
        return None;
    }
    let same = |r: &&map::Placed| {
        r.id != t.id && r.row == t.row && r.kind == t.kind && r.level == t.level && calm(r.id)
    };
    let mut total = t.merge;
    let mut group = vec![t.id];
    if let Some(l) = list.iter().filter(same).find(|r| r.end() == t.col) {
        if total + l.merge <= max {
            total += l.merge;
            group.push(l.id);
        }
    }
    if let Some(r) = list.iter().filter(same).find(|r| r.col == t.end()) {
        if total + r.merge <= max {
            total += r.merge;
            group.push(r.id);
        }
    }
    let shape_ok = rules::check_room_shape(Mode::Normal, &t.kind, total, t.level).is_ok();
    (group.len() > 1 && shape_ok).then_some(group)
}

/// Joins rooms of one row into the leftmost of them.
fn merge_rooms(b: &mut Builder<'_>, group: &[i64]) -> R<i64> {
    let placed: Vec<map::Placed> = map::rooms(b.root())
        .into_iter()
        .filter(|r| group.contains(&r.id))
        .collect();
    let keep = placed
        .iter()
        .min_by_key(|r| r.col)
        .ok_or(RuleError::NotFound)?;
    let (keep_id, col) = (keep.id, keep.col);
    let total: i64 = placed.iter().map(|r| r.merge).sum();
    for other in placed.iter().filter(|r| r.id != keep_id) {
        let oi = room_index(b.root(), other.id).ok_or(RuleError::NotFound)?;
        let o = arr(b.root(), &ROOMS)[oi].clone();
        let ki = room_index(b.root(), keep_id).ok_or(RuleError::NotFound)?;
        for list in [
            "dwellers",
            "deadDwellers",
            "dwellerWithPendingCompleteTraining",
            "mrHandyList",
            "partners",
            "children",
            "slots",
        ] {
            let Some(items) = o.get(list).and_then(Value::as_array) else {
                continue;
            };
            let target = pi(&ROOMS, ki, &[list]);
            if b.get(&target).is_none() {
                continue;
            }
            for item in items {
                b.push(target.clone(), item.clone())?;
            }
        }
        if let Some(res) = o
            .at(&p(&["storage", "resources"]))
            .and_then(Value::as_object)
        {
            for (k, v) in &res.0 {
                let key = k.text();
                let path = pi(&ROOMS, ki, &["storage", "resources", key.as_str()]);
                let add = v.as_f64().unwrap_or(0.0);
                if let Some(cur) = b.get(&path).and_then(Value::as_f64) {
                    if add != 0.0 {
                        set_num(b, path, cur + add)?;
                    }
                }
            }
        }
        let mut tasks = Vec::new();
        if let Some(state) = o.get("currentState") {
            owned_tasks(state, &mut tasks);
        }
        remove_tasks(b, &tasks)?;
        let oi = room_index(b.root(), other.id).ok_or(RuleError::NotFound)?;
        b.remove(p(&ROOMS), oi)?;
        for list in [&ACTORS, &DWELLERS] {
            let n = arr(b.root(), list).len();
            for ai in 0..n {
                let path = pi(list, ai, &["savedRoom"]);
                if b.get(&path).and_then(Value::as_i64) == Some(other.id) {
                    set_num(b, path, keep_id as f64)?;
                }
            }
        }
        let happy = p(&["happinessManager"]);
        if b.get(&happy).is_some() {
            b.remove_key(happy.clone(), &format!("room{}", other.id))?;
            remove_number(b, join(&happy, &["rooms"]), other.id)?;
        }
    }
    let ki = room_index(b.root(), keep_id).ok_or(RuleError::NotFound)?;
    set_num(b, pi(&ROOMS, ki, &["col"]), col as f64)?;
    set_num(b, pi(&ROOMS, ki, &["mergeLevel"]), total as f64)?;
    Ok(keep_id)
}

/// Ways a room of this size can be split, left to right.
pub fn split_options(merge: i64) -> Vec<Vec<i64>> {
    match merge {
        2 => vec![vec![1, 1]],
        3 => vec![vec![1, 1, 1], vec![1, 2], vec![2, 1]],
        _ => Vec::new(),
    }
}

/// Why a room cannot be split this way in this mode, if it cannot.
pub fn split_error(root: &Value, mode: Mode, id: i64, parts: &[i64]) -> Option<RuleError> {
    let Some(i) = room_index(root, id) else {
        return Some(RuleError::NotFound);
    };
    let r = &arr(root, &ROOMS)[i];
    let kind = text(r, &["type"]).unwrap_or_default();
    let merge = int(r, &["mergeLevel"]).unwrap_or(1);
    let level = int(r, &["level"]).unwrap_or(1);
    if map::FIXED_ROOMS.contains(&kind.as_str()) || kind == "Elevator" {
        return Some(RuleError::NotAllowed);
    }
    if parts.len() < 2 || parts.iter().any(|p| *p < 1) || parts.iter().sum::<i64>() != merge {
        return Some(RuleError::NotAllowed);
    }
    if mode == Mode::Normal {
        if parts
            .iter()
            .any(|p| rules::check_room_shape(mode, &kind, *p, level).is_err())
        {
            return Some(RuleError::RoomSize);
        }
        if room_busy(r) || text(r, &["currentStateName"]).as_deref() == Some("RoomEmergency") {
            return Some(RuleError::RoomBusy);
        }
    }
    None
}

fn split_room(b: &mut Builder<'_>, mode: Mode, id: i64, parts: &[i64]) -> R<usize> {
    if let Some(e) = split_error(b.root(), mode, id, parts) {
        return Err(e.into());
    }
    let i = room_index(b.root(), id).ok_or(RuleError::NotFound)?;
    let r = arr(b.root(), &ROOMS)[i].clone();
    let kind = text(&r, &["type"]).unwrap_or_default();
    let row = int(&r, &["row"]).unwrap_or(0);
    let mut col = int(&r, &["col"]).unwrap_or(0);
    let level = int(&r, &["level"]).unwrap_or(1);
    let class = text(&r, &["class"])
        .or_else(|| catalog().room(&kind).map(|c| c.class.clone()))
        .unwrap_or_else(|| "Facility".into());
    let mut movers = ints(&r, "dwellers").into_iter().skip(2 * parts[0] as usize);
    set_num(b, pi(&ROOMS, i, &["mergeLevel"]), parts[0] as f64)?;
    col += map::width_of(&kind, parts[0]);
    for part in &parts[1..] {
        let new_id = next_room_id(b)?;
        let room = new_room(b.root(), &kind, &class, (row, col), *part, level, new_id);
        b.push(p(&ROOMS), room)?;
        let group: Vec<i64> = movers.by_ref().take(2 * *part as usize).collect();
        for d in group {
            drop_from_rooms(b, d, None)?;
            let ni = room_index(b.root(), new_id).ok_or(RuleError::NotFound)?;
            b.push(pi(&ROOMS, ni, &["dwellers"]), Value::int(d))?;
            if let Ok(base) = dweller_path(b.root(), d) {
                set_num(b, join(&base, &["savedRoom"]), new_id as f64)?;
            }
        }
        col += map::width_of(&kind, *part);
    }
    Ok(parts.len())
}

fn set_room_level(b: &mut Builder<'_>, mode: Mode, id: i64, level: i64) -> R<()> {
    let i = room_index(b.root(), id).ok_or(RuleError::NotFound)?;
    let r = &arr(b.root(), &ROOMS)[i];
    let kind = text(r, &["type"]).unwrap_or_default();
    let merge = int(r, &["mergeLevel"]).unwrap_or(1);
    let max = catalog()
        .room(&kind)
        .map_or(catalog().limits.room_level_max, |r| {
            r.max_level(merge).max(1)
        });
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

/// Marks every contextual hint as already shown (or not yet shown).
fn set_hints(b: &mut Builder<'_>, shown: bool) -> R<usize> {
    let base = p(&["tutorialManager"]);
    let Some(o) = b.get(&base).and_then(Value::as_object).cloned() else {
        return Err(RuleError::NotFound.into());
    };
    let mut n = 0;
    for (k, v) in &o.0 {
        let key = k.text();
        if key.starts_with("Contextual")
            && matches!(v, Value::Bool(_))
            && b.set(join(&base, &[&key]), Value::Bool(shown))?
        {
            n += 1;
        }
    }
    Ok(n)
}

/// Marks a current objective done, as the game does when its goal is met;
/// the reward can then be collected in the game.
fn complete_objective(b: &mut Builder<'_>, slot: usize) -> R<()> {
    let base = vec![
        Seg::Key("objectiveMgr".into()),
        Seg::Key("slotArray".into()),
        Seg::Index(slot),
        Seg::Key("objective".into()),
    ];
    if b.get(&base).and_then(Value::as_object).is_none() {
        return Err(RuleError::NotFound.into());
    }
    let reqs = join(&base, &["requirements"]);
    let n = b.get(&reqs).and_then(Value::as_array).map_or(0, Vec::len);
    if n == 0 {
        let id = b
            .get(&join(&base, &["objectiveID"]))
            .and_then(Value::as_text)
            .unwrap_or_default();
        if let Some(g) = catalog().objective(&id).and_then(|o| o.goal.as_ref()) {
            let req = Value::obj(vec![
                ("requirementID", Value::str(&g.requirement_id)),
                ("satisfied", Value::Bool(true)),
            ]);
            if b.get(&reqs).is_some() {
                b.insert(reqs.clone(), 0, None, req)?;
            }
        }
    }
    for i in 0..n {
        let mut sat = reqs.clone();
        sat.push(Seg::Index(i));
        sat.push(Seg::Key("satisfied".into()));
        if b.get(&sat).is_some() {
            b.set(sat, Value::Bool(true))?;
        }
    }
    b.set(join(&base, &["completed"]), Value::Bool(true))?;
    Ok(())
}

/// The game checks the quest on load and picks another one if it is not available.
fn replace_quest(b: &mut Builder<'_>, mode: Mode, kind: &str, index: usize, id: &str) -> R<()> {
    let (prefix, path) = match kind {
        "daily" => (
            "Daily_",
            pi(
                &[
                    "completedQuestDataManager",
                    "dailyQuestPicker",
                    "currentDailies",
                ],
                index,
                &["questName"],
            ),
        ),
        "weekly" => (
            "Weekly_",
            pi(
                &[
                    "completedQuestDataManager",
                    "weeklyQuestPicker",
                    "currentWeeklies",
                ],
                index,
                &["questName"],
            ),
        ),
        "standalone" => (
            "Standalone_",
            p(&[
                "completedQuestDataManager",
                "standaloneQuestPicker",
                "currentStandalone",
            ]),
        ),
        _ => return Err(RuleError::NotFound.into()),
    };
    if mode == Mode::Normal
        && !(id.starts_with(prefix) && catalog().quests.iter().any(|q| q.id == id))
    {
        return Err(RuleError::NotInCatalog.into());
    }
    if b.get(&path).is_none() {
        return Err(RuleError::NotFound.into());
    }
    b.set(path, Value::str(id))?;
    Ok(())
}

/// Puts another objective into a slot. Requirements are left empty so the game sets
/// them up from scratch when the save is loaded.
fn replace_objective(b: &mut Builder<'_>, mode: Mode, slot: usize, id: &str, level: i64) -> R<()> {
    let slots = p(&["objectiveMgr", "slotArray"]);
    let list = b
        .get(&slots)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if slot >= list.len() {
        return Err(RuleError::NotFound.into());
    }
    let taken = list.iter().enumerate().any(|(i, s)| {
        i != slot
            && s.at(&p(&["objective", "objectiveID"]))
                .and_then(Value::as_text)
                .as_deref()
                == Some(id)
    });
    if taken {
        return Err(RuleError::NotAllowed.into());
    }
    let info = catalog().objective(id);
    if mode == Mode::Normal {
        let Some(o) = info else {
            return Err(RuleError::NotInCatalog.into());
        };
        let survival = text(b.root(), &["vault", "VaultMode"]).as_deref() == Some("Survival");
        if o.tutorial || o.season || (survival && !o.survival) || (!survival && !o.normal) {
            return Err(RuleError::NotAllowed.into());
        }
        let max = o
            .goal
            .as_ref()
            .and_then(|g| g.max_level)
            .unwrap_or(i64::MAX);
        rules::check_range(mode, level, 0, max)?;
    } else if level < 0 {
        return Err(RuleError::OutOfRange {
            min: 0,
            max: i64::MAX,
        }
        .into());
    }
    let obj = Value::obj(vec![
        ("objectiveID", Value::str(id)),
        ("requirements", Value::Array(Vec::new())),
        ("completed", Value::Bool(false)),
        ("incrementLevel", Value::int(level)),
    ]);
    let mut path = slots;
    path.push(Seg::Index(slot));
    path.push(Seg::Key("objective".into()));
    b.set(path, obj)?;
    Ok(())
}

/// The game counts time already waited up to a total, so both are set.
fn set_stranger_time(b: &mut Builder<'_>, seconds: f64) -> R<()> {
    if !(0.0..=1.0e7).contains(&seconds) {
        return Err(RuleError::OutOfRange {
            min: 0,
            max: 10_000_000,
        }
        .into());
    }
    let base = p(&["MysteriousStranger"]);
    let hiding =
        text(b.root(), &["MysteriousStranger", "currentState"]).as_deref() == Some("Hiding");
    let (total, passed) = if hiding {
        ("timeToAppear", "remainingTimeToAppear")
    } else {
        ("timeToHide", "remainingTimeToHide")
    };
    let t = b
        .get(&join(&base, &[total]))
        .and_then(Value::as_f64)
        .ok_or(RuleError::NotFound)?;
    let t = t.max(seconds);
    set_num(b, join(&base, &[total]), t)?;
    if b.get(&join(&base, &[passed])).is_some() {
        set_num(b, join(&base, &[passed]), t - seconds)?;
    } else {
        b.set(join(&base, &[passed]), Value::float2(t - seconds))?;
    }
    Ok(())
}

/// Average the game stores as the week rating (ceil of the mean).
pub fn week_average(ratings: &[i64]) -> Option<i64> {
    if ratings.is_empty() {
        return None;
    }
    let sum: i64 = ratings.iter().sum();
    Some((sum + ratings.len() as i64 - 1).div_euclid(ratings.len() as i64))
}

fn set_day_rating(
    b: &mut Builder<'_>,
    index: usize,
    rating: Option<i64>,
    collected: Option<bool>,
) -> R<()> {
    if let Some(r) = rating {
        if !(0..=100).contains(&r) {
            return Err(RuleError::OutOfRange { min: 0, max: 100 }.into());
        }
    }
    fn list(b: &Builder<'_>, name: &str) -> Vec<Value> {
        b.get(&p(&["ratingMgr", name]))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }
    let days = list(b, "ratings");
    let last = list(b, "ratingsLast");
    let day = days.get(index).ok_or(RuleError::NotFound)?;
    // After the seventh day the game copies the week into ratingsLast (same objects), so both change.
    let time = |v: &Value| v.get("time").map(Value::to_compact);
    let mirror =
        days.len() >= 7 && last.len() == days.len() && last.get(index).and_then(time) == time(day);
    let mut lists = vec!["ratings"];
    if mirror {
        lists.push("ratingsLast");
    }
    for name in lists {
        if let Some(r) = rating {
            let path = pi(&["ratingMgr", name], index, &["rating"]);
            if !set_num(b, path, r as f64)? {
                return Err(RuleError::NotFound.into());
            }
        }
        if let Some(c) = collected {
            let path = pi(&["ratingMgr", name], index, &["collected"]);
            if b.get(&path).is_some() {
                b.set(path, Value::Bool(c))?;
            }
        }
    }
    if mirror && rating.is_some() {
        let values: Vec<i64> = list(b, "ratings")
            .iter()
            .filter_map(|d| d.get("rating").and_then(Value::as_i64))
            .collect();
        if let Some(avg) = week_average(&values) {
            let path = p(&["ratingMgr", "weekRating"]);
            if b.get(&path).is_some() {
                set_num(b, path, avg as f64)?;
            }
        }
    }
    Ok(())
}

/// Shifts both ends of a task, so a recurrent task keeps its length when the game reloads it.
fn set_task_time(b: &mut Builder<'_>, id: i64, seconds: f64) -> R<()> {
    if !(0.0..=1.0e7).contains(&seconds) {
        return Err(RuleError::OutOfRange {
            min: 0,
            max: 10_000_000,
        }
        .into());
    }
    let now = b
        .get(&p(&["taskMgr", "time"]))
        .and_then(Value::as_f64)
        .ok_or(RuleError::NotFound)?;
    let tasks = arr(b.root(), &["taskMgr", "tasks"]);
    let i = tasks
        .iter()
        .position(|t| int(t, &["id"]) == Some(id))
        .ok_or(RuleError::NotFound)?;
    let end = tasks[i]
        .get("endTime")
        .and_then(Value::as_f64)
        .unwrap_or(now);
    let start = tasks[i]
        .get("startTime")
        .and_then(Value::as_f64)
        .unwrap_or(now);
    let delta = now + seconds - end;
    set_num(
        b,
        pi(&["taskMgr", "tasks"], i, &["startTime"]),
        start + delta,
    )?;
    set_num(b, pi(&["taskMgr", "tasks"], i, &["endTime"]), end + delta)?;
    Ok(())
}

/// Marks an achievement as earned or not. The game reports earned ones to Steam on load.
fn set_achievement(b: &mut Builder<'_>, mode: Mode, id: &str, done: bool) -> R<()> {
    if mode == Mode::Normal && !catalog().achievements.iter().any(|a| a.objective_id == id) {
        return Err(RuleError::NotInCatalog.into());
    }
    let base = p(&["vault", "Achievements"]);
    if b.get(&base).and_then(Value::as_object).is_none() {
        return Err(RuleError::NotFound.into());
    }
    let done_list = join(&base, &["completed"]);
    if b.get(&done_list).is_none() {
        b.set(done_list.clone(), Value::Array(Vec::new()))?;
    }
    let ids: Vec<Option<String>> = b
        .get(&done_list)
        .and_then(Value::as_array)
        .map(|a| a.iter().map(Value::as_text).collect())
        .unwrap_or_default();
    let progress = join(&base, &["objectivesInProgress"]);
    if done {
        if !ids.iter().any(|v| v.as_deref() == Some(id)) {
            b.insert(done_list, ids.len(), None, Value::str(id))?;
        }
        let entries = b
            .get(&progress)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for (i, e) in entries.iter().enumerate().rev() {
            if text(e, &["objectiveID"]).as_deref() == Some(id) {
                b.remove(progress.clone(), i)?;
            }
        }
    } else {
        for (i, v) in ids.iter().enumerate().rev() {
            if v.as_deref() == Some(id) {
                b.remove(done_list.clone(), i)?;
            }
        }
    }
    Ok(())
}

/// Adds a quest to the completed list or takes it out.
fn mark_quest(b: &mut Builder<'_>, mode: Mode, id: &str, done: bool) -> R<()> {
    if mode == Mode::Normal && !catalog().quests.iter().any(|q| q.id == id) {
        return Err(RuleError::NotInCatalog.into());
    }
    let list = p(&["completedQuestDataManager", "completedQuests"]);
    let pos = b.get(&list).and_then(Value::as_array).map(|a| {
        (
            a.len(),
            a.iter().position(|v| v.as_text().as_deref() == Some(id)),
        )
    });
    let Some((len, pos)) = pos else {
        return Err(RuleError::NotFound.into());
    };
    match (done, pos) {
        (true, None) => b.insert(list, len, None, Value::str(id))?,
        (false, Some(i)) => {
            b.remove(list, i)?;
        }
        _ => {}
    }
    Ok(())
}

const UNLOCKS: [&str; 1] = ["unlockableMgr"];

/// Blueprint objective of a room type when it is not unlocked yet.
pub fn room_lock(root: &Value, kind: &str) -> Option<String> {
    let id = catalog().room(kind)?.unlock.clone()?;
    if catalog().unlockables.iter().any(|u| u.id == id && u.season) {
        return None;
    }
    let claimed = arr(root, &["unlockableMgr", "claimed"])
        .iter()
        .any(|v| v.as_text().as_deref() == Some(id.as_str()));
    if claimed || root.get("unlockableMgr").is_none() {
        None
    } else {
        Some(id)
    }
}

fn objective_entry_id(v: &Value) -> String {
    v.as_text()
        .or_else(|| v.get("objectiveID").and_then(Value::as_text))
        .unwrap_or_default()
}

fn drop_objective(b: &mut Builder<'_>, list: &str, id: &str) -> R<usize> {
    let lp = join(&p(&UNLOCKS), &[list]);
    let mut n = 0;
    while let Some(i) = b
        .get(&lp)
        .and_then(Value::as_array)
        .and_then(|a| a.iter().position(|e| objective_entry_id(e) == id))
    {
        b.remove(lp.clone(), i)?;
        n += 1;
    }
    Ok(n)
}

fn set_room_unlocked(b: &mut Builder<'_>, mode: Mode, kind: &str, unlocked: bool) -> R<usize> {
    let id = catalog()
        .room(kind)
        .and_then(|r| r.unlock.clone())
        .ok_or(RuleError::NotAllowed)?;
    if b.get(&p(&UNLOCKS)).is_none() {
        return Err(RuleError::NotFound.into());
    }
    let mut n = drop_objective(b, "completed", &id)?;
    if unlocked {
        n += drop_objective(b, "objectivesInProgress", &id)?;
        n += ensure_strings(
            b,
            p(&["unlockableMgr", "claimed"]),
            [id].into_iter(),
            |_| None,
        )?;
    } else {
        if mode == Mode::Normal && map::rooms(b.root()).iter().any(|r| r.kind == kind) {
            return Err(RuleError::RoomLocked.into());
        }
        n += drop_objective(b, "claimed", &id)?;
    }
    Ok(n)
}

fn recipe_ids() -> Vec<&'static str> {
    let c = catalog();
    c.weapons
        .iter()
        .filter(|w| w.recipe)
        .map(|w| w.id.as_str())
        .chain(c.outfits.iter().filter(|o| o.recipe).map(|o| o.id.as_str()))
        .chain(c.themes.iter().filter(|t| t.recipe).map(|t| t.id.as_str()))
        .collect()
}

fn remove_string(b: &mut Builder<'_>, path: Vec<Seg>, value: &str) -> R<usize> {
    let mut n = 0;
    while let Some(i) = b
        .get(&path)
        .and_then(Value::as_array)
        .and_then(|a| a.iter().position(|v| v.as_text().as_deref() == Some(value)))
    {
        b.remove(path.clone(), i)?;
        n += 1;
    }
    Ok(n)
}

fn set_recipe(b: &mut Builder<'_>, mode: Mode, id: &str, known: bool) -> R<usize> {
    if mode == Mode::Normal && !recipe_ids().contains(&id) {
        return Err(RuleError::NotInCatalog.into());
    }
    let path = p(&["survivalW", "recipes"]);
    if b.get(&path).is_none() {
        return Err(RuleError::NotFound.into());
    }
    if known {
        ensure_strings(b, path, [id.to_string()].into_iter(), |_| None)
    } else {
        remove_string(b, path, id)
    }
}

const COLLECTION_LISTS: [&str; 7] = [
    "weapons",
    "outfits",
    "dwellers",
    "pets",
    "breeds",
    "decorations",
    "junk",
];

/// Clears the "new" marks in the journal and on room themes.
fn mark_collection_read(b: &mut Builder<'_>) -> R<usize> {
    let mut n = 0;
    for key in COLLECTION_LISTS {
        let path = p(&["survivalW", key]);
        let Some(Value::Array(cur)) = b.get(&path) else {
            continue;
        };
        let mut changed = 0;
        let list: Vec<Value> = cur
            .iter()
            .map(|v| match v.as_text() {
                Some(s) if s.starts_with('N') => {
                    changed += 1;
                    Value::str(&format!("O{}", &s[1..]))
                }
                _ => v.clone(),
            })
            .collect();
        if changed > 0 {
            b.set(path, Value::Array(list))?;
            n += changed;
        }
    }
    let mut flags: Vec<Vec<Seg>> = Vec::new();
    for (i, _) in arr(b.root(), &["survivalW", "collectedThemes", "themeList"])
        .iter()
        .enumerate()
    {
        flags.push(pi(
            &["survivalW", "collectedThemes", "themeList"],
            i,
            &["extraData", "IsNew"],
        ));
    }
    if let Some(ev) = b
        .get(&p(&["specialTheme", "eventsThemes"]))
        .and_then(Value::as_object)
    {
        for k in ev.keys() {
            flags.push(p(&[
                "specialTheme",
                "eventsThemes",
                &k,
                "extraData",
                "IsNew",
            ]));
        }
    }
    for path in flags {
        if b.get(&path).and_then(Value::as_bool) == Some(true) {
            b.set(path, Value::Bool(false))?;
            n += 1;
        }
    }
    Ok(n)
}

const THEME_LIST: [&str; 3] = ["survivalW", "collectedThemes", "themeList"];
pub const THEME_PARTS: i64 = 9;

fn theme_entry(id: &str, parts: i64, crafted: bool) -> Value {
    Value::obj(vec![
        ("id", Value::str(id)),
        ("type", Value::str("Theme")),
        ("hasBeenAssigned", Value::Bool(false)),
        ("hasRandonWeaponBeenAssigned", Value::Bool(false)),
        (
            "extraData",
            Value::obj(vec![
                ("partsCollectedCount", Value::int(parts)),
                ("IsCraftingInProgress", Value::Bool(false)),
                ("IsCrafted", Value::Bool(crafted)),
                ("IsClaimed", Value::Bool(crafted)),
                ("IsClaimedInCraftingRoom", Value::Bool(crafted)),
                ("IsNew", Value::Bool(crafted)),
            ]),
        ),
    ])
}

/// Parts found (0..=9) and whether the theme is crafted and can be applied.
pub fn theme_state(root: &Value, id: &str) -> (i64, bool) {
    arr(root, &THEME_LIST)
        .iter()
        .find(|t| text(t, &["id"]).as_deref() == Some(id))
        .map_or((0, false), |t| {
            (
                int(t, &["extraData", "partsCollectedCount"]).unwrap_or(0),
                flag(t, &["extraData", "IsCrafted"]).unwrap_or(false),
            )
        })
}

fn set_theme_state(
    b: &mut Builder<'_>,
    mode: Mode,
    id: &str,
    parts: i64,
    crafted: bool,
) -> R<usize> {
    if mode == Mode::Normal && catalog().theme(id).is_none() {
        return Err(RuleError::NotInCatalog.into());
    }
    rules::check_range(mode, parts, 0, THEME_PARTS)?;
    if b.get(&p(&THEME_LIST)).is_none() {
        return Err(RuleError::NotFound.into());
    }
    let parts = if crafted { THEME_PARTS } else { parts };
    let pos = arr(b.root(), &THEME_LIST)
        .iter()
        .position(|t| text(t, &["id"]).as_deref() == Some(id));
    let mut n = 0;
    match pos {
        None if parts > 0 => {
            b.push(p(&THEME_LIST), theme_entry(id, parts, crafted))?;
            n += 1;
        }
        None => {}
        Some(i) if parts == 0 => {
            b.remove(p(&THEME_LIST), i)?;
            n += 1;
        }
        Some(i) => {
            let was_crafted =
                flag(&arr(b.root(), &THEME_LIST)[i], &["extraData", "IsCrafted"]).unwrap_or(false);
            let base = pi(&THEME_LIST, i, &["extraData"]);
            let mut sets = vec![
                ("partsCollectedCount", Value::int(parts)),
                ("IsCrafted", Value::Bool(crafted)),
                ("IsClaimed", Value::Bool(crafted)),
                ("IsClaimedInCraftingRoom", Value::Bool(crafted)),
            ];
            if crafted != was_crafted {
                sets.push(("IsCraftingInProgress", Value::Bool(false)));
                sets.push(("IsNew", Value::Bool(crafted)));
            }
            for (k, v) in sets {
                n += usize::from(b.set_key(base.clone(), k, v)?);
            }
        }
    }
    let recipes = p(&["survivalW", "recipes"]);
    if b.get(&recipes).is_some() {
        n += if parts >= THEME_PARTS {
            ensure_strings(b, recipes, [id.to_string()].into_iter(), |_| None)?
        } else {
            remove_string(b, recipes, id)?
        };
    }
    if !crafted {
        if let Some(t) = catalog().theme(id) {
            let key = p(&["specialTheme", "themeByRoomType"]);
            let applied = b
                .get(&key)
                .and_then(|m| m.get(&t.room))
                .and_then(Value::as_text);
            if applied.as_deref() == Some(t.theme.as_str()) {
                n += usize::from(b.remove_key(key, &t.room)?);
            }
        }
    }
    Ok(n)
}

/// Theme shown by every room of the type; `None` restores the standard look.
fn set_room_theme(b: &mut Builder<'_>, mode: Mode, room: &str, theme: Option<&str>) -> R<usize> {
    let key = p(&["specialTheme", "themeByRoomType"]);
    if b.get(&key).is_none() {
        return Err(RuleError::NotFound.into());
    }
    let Some(id) = theme else {
        return Ok(usize::from(b.remove_key(key, room)?));
    };
    let t = catalog().theme(id).ok_or(RuleError::NotInCatalog)?;
    if mode == Mode::Normal && t.room != room {
        return Err(RuleError::NotAllowed.into());
    }
    let mut n = 0;
    if !theme_state(b.root(), id).1 {
        n += set_theme_state(b, mode, id, THEME_PARTS, true)?;
    }
    n += usize::from(b.set_key(key, room, Value::str(&t.theme))?);
    Ok(n)
}

/// Ready-to-collect amount of a production room.
fn set_stored(b: &mut Builder<'_>, mode: Mode, room: i64, resource: &str, value: f64) -> R<bool> {
    let i = room_index(b.root(), room).ok_or(RuleError::NotFound)?;
    let r = &arr(b.root(), &ROOMS)[i];
    if mode == Mode::Normal {
        let kind = text(r, &["type"]).unwrap_or_default();
        let batch = catalog()
            .room(&kind)
            .and_then(|info| {
                info.batch(
                    int(r, &["mergeLevel"]).unwrap_or(1),
                    int(r, &["level"]).unwrap_or(1),
                )
            })
            .and_then(|m| m.get(resource).copied())
            .ok_or(RuleError::NotAllowed)?;
        rules::check_range(mode, value.round() as i64, 0, batch.round() as i64)?;
    }
    let path = pi(&ROOMS, i, &["storage", "resources", resource]);
    if b.get(&path).is_none() {
        return Err(RuleError::NotFound.into());
    }
    set_num(b, path, value)
}

fn team_base(root: &Value, team: usize) -> R<Vec<Seg>> {
    let base = pi(&["vault", "wasteland", "teams"], team, &["teamEquipment"]);
    if root.at(&base).is_none() {
        return Err(RuleError::NotFound.into());
    }
    Ok(base)
}

/// Items a team carries that count against its limit (recipes do not).
pub fn team_load(team: &Value) -> usize {
    team.at(&p(&["teamEquipment", "inventory", "items"]))
        .and_then(Value::as_array)
        .map_or(0, |a| {
            a.iter()
                .filter(|it| {
                    !flag(it, &["extraData", "isRecipe"]).unwrap_or(false)
                        && text(it, &["id"]).as_deref() != Some("Ultracite")
                })
                .count()
        })
}

fn add_team_items(
    b: &mut Builder<'_>,
    mode: Mode,
    team: usize,
    kind: ItemKind,
    id: &str,
    count: i64,
    recipe: bool,
) -> R<usize> {
    let base = team_base(b.root(), team)?;
    if mode == Mode::Normal {
        if !matches!(kind, ItemKind::Weapon | ItemKind::Outfit | ItemKind::Junk) {
            return Err(RuleError::NotAllowed.into());
        }
        rules::check_item(mode, kind, id)?;
        if recipe && !recipe_ids().contains(&id) {
            return Err(RuleError::NotAllowed.into());
        }
        rules::check_range(mode, count, 1, rules::ITEM_ADD_MAX)?;
        let team_v = &arr(b.root(), &["vault", "wasteland", "teams"])[team];
        let room = catalog().limits.team_items_max as usize;
        if !recipe && team_load(team_v) + count.max(0) as usize > room {
            return Err(RuleError::InventoryFull.into());
        }
    }
    let mut item = item_value(kind, id);
    if recipe {
        if let Some(o) = item.as_object_mut() {
            o.0.push((
                json::JsonStr::new("extraData"),
                Value::obj(vec![("isRecipe", Value::Bool(true))]),
            ));
        }
    }
    let list = join(&base, &["inventory", "items"]);
    for _ in 0..count.max(0) {
        b.push(list.clone(), item.clone())?;
    }
    Ok(count.max(0) as usize)
}

fn remove_team_items(b: &mut Builder<'_>, team: usize, indices: &[usize]) -> R<usize> {
    let list = join(&team_base(b.root(), team)?, &["inventory", "items"]);
    let mut sorted = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut n = 0;
    for i in sorted.into_iter().rev() {
        b.remove(list.clone(), i)?;
        n += 1;
    }
    Ok(n)
}

fn set_team_theme_parts(
    b: &mut Builder<'_>,
    mode: Mode,
    team: usize,
    id: &str,
    parts: i64,
) -> R<usize> {
    if mode == Mode::Normal && catalog().theme(id).is_none() {
        return Err(RuleError::NotInCatalog.into());
    }
    rules::check_range(mode, parts, 0, THEME_PARTS)?;
    let list = join(
        &team_base(b.root(), team)?,
        &["collectedThemes", "themeList"],
    );
    let pos = b.get(&list).and_then(Value::as_array).and_then(|a| {
        a.iter()
            .position(|t| text(t, &["id"]).as_deref() == Some(id))
    });
    match pos {
        None if parts > 0 => {
            if b.get(&list).is_none() {
                return Err(RuleError::NotFound.into());
            }
            b.push(list, theme_entry(id, parts, false))?;
            Ok(1)
        }
        None => Ok(0),
        Some(i) if parts == 0 => {
            b.remove(list, i)?;
            Ok(1)
        }
        Some(i) => {
            let mut path = list;
            path.push(Seg::Index(i));
            path.push(Seg::Key("extraData".into()));
            Ok(usize::from(b.set_key(
                path,
                "partsCollectedCount",
                Value::int(parts),
            )?))
        }
    }
}

const EMERGENCY: [&str; 2] = ["vault", "emergencyData"];

/// Incidents that start inside a room; door attacks need enemy positions the save does not hold.
pub const ROOM_INCIDENTS: [i64; 4] = [0, 1, 4, 7];

/// Room types that never have incidents of their own.
pub const NO_INCIDENT_ROOMS: [&str; 3] = ["Elevator", "Entrance", "FakeWasteland"];

fn incident_rooms(root: &Value) -> Vec<usize> {
    arr(root, &ROOMS)
        .iter()
        .enumerate()
        .filter(|(_, r)| text(r, &["currentStateName"]).as_deref() == Some("RoomEmergency"))
        .map(|(i, _)| i)
        .collect()
}

pub fn room_max_health(r: &Value) -> Option<f64> {
    let kind = text(r, &["type"])?;
    let h = catalog().room(&kind)?.max_health(
        int(r, &["mergeLevel"]).unwrap_or(1),
        int(r, &["level"]).unwrap_or(1),
    );
    (h > 0.0).then_some(h)
}

/// Ends the current incident the way the game leaves a vault after one.
fn finish_emergency(b: &mut Builder<'_>) -> R<usize> {
    let mut n = 0;
    for i in incident_rooms(b.root()) {
        let base = pi(&ROOMS, i, &[]);
        n += usize::from(b.set_key(base.clone(), "currentStateName", Value::str("Idle"))?);
        b.set_key(base.clone(), "currentState", Value::obj(vec![]))?;
        if b.get(&join(&base, &["roomHealth", "damageValue"]))
            .is_some()
        {
            b.set(
                join(&base, &["roomHealth", "damageValue"]),
                Value::float2(0.0),
            )?;
        }
    }
    let em = p(&EMERGENCY);
    if b.get(&em).is_none() {
        return Ok(n);
    }
    let task = int(b.root(), &["vault", "emergencyData", "cicleTaskId"]);
    for (k, v) in [
        ("active", Value::Bool(false)),
        ("failedPreviousWave", Value::Bool(false)),
        ("currentWave", Value::int(0)),
        ("graceTimeLeft", Value::int(-1)),
    ] {
        if b.get(&join(&em, &[k])).is_some() {
            n += usize::from(b.set(join(&em, &[k]), v)?);
        }
    }
    for k in [
        "cicleTaskId",
        "mode",
        "consumedFood",
        "consumedEnergy",
        "consumedWater",
        "emergencyConsumption",
        "raider",
        "deathclaw",
        "ghoul",
        "protectron",
        "alien",
        "scorched",
        "securitron",
        "enclaveSecurity",
        "enclavePowerArmor",
        "gen1Synth",
        "gen2Synth",
    ] {
        n += usize::from(b.remove_key(em.clone(), k)?);
    }
    if let Some(t) = task.filter(|t| *t >= 0) {
        remove_tasks(b, &[t])?;
    }
    Ok(n)
}

fn start_incident(b: &mut Builder<'_>, mode: Mode, room: i64, kind: i64, strength: f64) -> R<()> {
    let i = room_index(b.root(), room).ok_or(RuleError::NotFound)?;
    let r = arr(b.root(), &ROOMS)[i].clone();
    let rk = text(&r, &["type"]).unwrap_or_default();
    if mode == Mode::Normal {
        if !ROOM_INCIDENTS.contains(&kind) || NO_INCIDENT_ROOMS.contains(&rk.as_str()) {
            return Err(RuleError::NotAllowed.into());
        }
        if flag(b.root(), &["vault", "emergencyData", "active"]) == Some(true) {
            return Err(RuleError::IncidentActive.into());
        }
        if (strength * 100.0).round() < 10.0 || strength > 1.0 {
            return Err(RuleError::OutOfRange { min: 10, max: 100 }.into());
        }
    }
    let max = room_max_health(&r).ok_or(RuleError::NotInCatalog)?;
    let damage = (max * strength.clamp(0.01, 1.0)).max(1.0);
    let base = pi(&ROOMS, i, &[]);
    for task in [
        r.at(&p(&["currentState", "taskId"]))
            .and_then(Value::as_i64),
        r.at(&p(&["currentState", "breedingTaskId"]))
            .and_then(Value::as_i64),
    ]
    .into_iter()
    .flatten()
    .filter(|t| *t >= 0)
    {
        remove_tasks(b, &[task])?;
    }
    b.set_key(
        base.clone(),
        "currentStateName",
        Value::str("RoomEmergency"),
    )?;
    b.set_key(
        base.clone(),
        "currentState",
        Value::obj(vec![
            ("mode", Value::int(kind)),
            ("controllerAdditionnalData", Value::obj(vec![])),
        ]),
    )?;
    b.set_key(
        base.clone(),
        "roomHealth",
        Value::obj(vec![
            ("damageValue", Value::float2(damage)),
            ("initialValue", Value::float2(damage)),
        ]),
    )?;
    for j in 0..arr(b.root(), &ROOMS).len() {
        let path = pi(&ROOMS, j, &["emergencyDone"]);
        if b.get(&path).and_then(Value::as_bool) == Some(true) {
            b.set(path, Value::Bool(false))?;
        }
    }
    let em = p(&EMERGENCY);
    if b.get(&em).is_none() {
        return Err(RuleError::NotFound.into());
    }
    for (k, v) in [
        ("active", Value::Bool(true)),
        ("cicleTaskId", Value::int(-1)),
        ("mode", Value::int(kind)),
        ("consumedFood", Value::float2(0.0)),
        ("consumedEnergy", Value::float2(0.0)),
        ("consumedWater", Value::float2(0.0)),
        ("emergencyConsumption", Value::obj(vec![])),
    ] {
        b.set_key(em.clone(), k, v)?;
    }
    Ok(())
}

/// Incident strength: room damage share for room incidents, raider health for raids.
fn set_incident_strength(
    b: &mut Builder<'_>,
    mode: Mode,
    room: Option<i64>,
    value: f64,
) -> R<bool> {
    match room {
        Some(id) => {
            let i = room_index(b.root(), id).ok_or(RuleError::NotFound)?;
            let r = &arr(b.root(), &ROOMS)[i];
            if text(r, &["currentStateName"]).as_deref() != Some("RoomEmergency") {
                return Err(RuleError::NotAllowed.into());
            }
            let max = room_max_health(r).ok_or(RuleError::NotInCatalog)?;
            if mode == Mode::Normal && !(0.01..=1.0).contains(&value) {
                return Err(RuleError::OutOfRange { min: 1, max: 100 }.into());
            }
            set_num(
                b,
                pi(&ROOMS, i, &["roomHealth", "damageValue"]),
                (max * value).max(1.0),
            )
        }
        None => {
            let base = p(&["vault", "emergencyData", "raider"]);
            let max = b
                .get(&join(&base, &["lifeMax"]))
                .and_then(Value::as_f64)
                .ok_or(RuleError::NotFound)?;
            if mode == Mode::Normal && !(1.0..=max.max(1.0)).contains(&value) {
                return Err(RuleError::OutOfRange {
                    min: 1,
                    max: max.round() as i64,
                }
                .into());
            }
            set_num(b, join(&base, &["life"]), value)
        }
    }
}

fn mark_in_journal(b: &mut Builder<'_>, list: &str, code: &str) -> R<usize> {
    let path = p(&["survivalW", list]);
    if b.get(&path).is_none() {
        return Ok(0);
    }
    ensure_strings(b, path, [format!("N{code}")].into_iter(), |s| {
        s.get(1..).map(str::to_string)
    })
}

/// Weapon and outfit of a new dweller go to the journal like in the game.
fn journal_equipment(b: &mut Builder<'_>, outfit: &str, weapon: &str) -> R<()> {
    if let Some(o) = catalog().outfit(outfit).filter(|o| !o.hidden) {
        mark_in_journal(b, "outfits", &o.code)?;
    }
    if let Some(w) = catalog().weapon(weapon).filter(|w| !w.hidden) {
        mark_in_journal(b, "weapons", &w.code)?;
    }
    Ok(())
}

fn add_unique_dweller(b: &mut Builder<'_>, mode: Mode, id: &str) -> R<String> {
    let u = catalog()
        .unique_dweller(id)
        .ok_or(RuleError::NotInCatalog)?;
    if mode == Mode::Normal {
        if !u.in_journal() {
            return Err(RuleError::NotAllowed.into());
        }
        if arr(b.root(), &DWELLERS).len() >= MAX_DWELLERS {
            return Err(RuleError::TooManyDwellers.into());
        }
    }
    let new_id = free_dweller_id(b.root())?;
    let mut d = new_dweller(new_id, &u.first_name, &u.last_name, u.gender);
    let endurance = u.special.get(2).copied().unwrap_or(1);
    let hp = catalog().initial_max_health(endurance);
    let weapon = if u.weapon.is_empty() {
        "Fist"
    } else {
        u.weapon.as_str()
    };
    let mut sets: Vec<(Vec<Seg>, Value)> = vec![
        (p(&["health", "healthValue"]), Value::float2(hp)),
        (p(&["health", "maxHealth"]), Value::float2(hp)),
        (p(&["outfitColor"]), Value::int(0xFFFF_FFFF)),
        (
            p(&["equipedOutfit"]),
            item_value(ItemKind::Outfit, &u.outfit),
        ),
        (p(&["equipedWeapon"]), item_value(ItemKind::Weapon, weapon)),
        (
            p(&["rarity"]),
            Value::str(if u.rarity.is_empty() {
                "Normal"
            } else {
                &u.rarity
            }),
        ),
    ];
    for (k, v) in u.special.iter().enumerate().take(7) {
        sets.push((
            vec![
                Seg::Key("stats".into()),
                Seg::Key("stats".into()),
                Seg::Index(k + 1),
                Seg::Key("value".into()),
            ],
            Value::int(*v),
        ));
    }
    if !u.random_body {
        if let Some(c) = u.skin_color {
            sets.push((p(&["skinColor"]), Value::int(i64::from(c))));
        }
        if let Some(c) = u.hair_color {
            sets.push((p(&["hairColor"]), Value::int(i64::from(c))));
        }
    }
    for (path, v) in sets {
        if let Some(slot) = d.at_mut(&path) {
            *slot = v;
        }
    }
    if let Some(o) = d.as_object_mut() {
        o.0.retain(|(k, _)| !k.eq_text("hair") && !k.eq_text("faceMask"));
        let at =
            o.0.iter()
                .position(|(k, _)| k.eq_text("pendingExperienceReward"))
                .map_or(o.0.len(), |i| i + 1);
        let mut extra = vec![("uniqueData", Value::str(&u.id))];
        if u.random_body {
            extra.push(("hair", Value::str("01")));
        } else {
            if let Some(h) = &u.hair {
                extra.push(("hair", Value::str(h)));
            }
            if let Some(f) = &u.face_mask {
                extra.push(("faceMask", Value::str(f)));
            }
        }
        for (j, (k, v)) in extra.into_iter().enumerate() {
            o.0.insert(at + j, (json::JsonStr::new(k), v));
        }
    }
    b.push(p(&DWELLERS), d)?;
    bump_dweller_counter(b, new_id)?;
    if u.rarity == "Legendary" {
        mark_in_journal(b, "dwellers", &u.id)?;
    }
    journal_equipment(b, &u.outfit, weapon)?;
    Ok(format!("{} {}", u.first_name, u.last_name)
        .trim()
        .to_string())
}

fn import_dweller(b: &mut Builder<'_>, mode: Mode, json_text: &str) -> R<String> {
    let src = json::parse(json_text).map_err(|e| ActionError::BadJson(e.to_string()))?;
    if src.as_object().is_none() || int(&src, &["gender"]).is_none() {
        return Err(RuleError::WrongType.into());
    }
    if mode == Mode::Normal && arr(b.root(), &DWELLERS).len() >= MAX_DWELLERS {
        return Err(RuleError::TooManyDwellers.into());
    }
    let gender = int(&src, &["gender"]).unwrap_or(1);
    let new_id = free_dweller_id(b.root())?;
    let pet = src.get("equippedPet").cloned();
    let mut d = clone_dweller(src, new_id);
    // Everything that pointed into the other vault is cleared.
    if let Some(r) = d.at_mut(&p(&["relations", "relations"])) {
        *r = Value::Array(Vec::new());
    }
    if let Some(Value::Array(a)) = d.at_mut(&p(&["relations", "ascendants"])) {
        for x in a.iter_mut() {
            *x = Value::int(-1);
        }
    }
    for key in ["daysOnWasteland", "hoursOnWasteland"] {
        if let Some(v) = d.at_mut(&p(&[key])) {
            *v = Value::int(0);
        }
    }
    if let Some(o) = d.as_object_mut() {
        o.0.retain(|(k, _)| {
            ![
                "dwellerRoom",
                "dwellerPosition",
                "dwellerCriticalMeter",
                "dwellerCriticalFactor",
                "equipment",
            ]
            .iter()
            .any(|x| k.eq_text(x))
        });
    }
    // Items this vault's game does not know, or that do not fit, become the defaults.
    if mode == Mode::Normal {
        let outfit = text(&d, &["equipedOutfit", "id"]).unwrap_or_default();
        if catalog().outfit(&outfit).is_none() || !catalog().outfit_fits(&outfit, gender) {
            if let Some(v) = d.at_mut(&p(&["equipedOutfit"])) {
                *v = item_value(ItemKind::Outfit, "jumpsuit");
            }
        }
        let weapon = text(&d, &["equipedWeapon", "id"]).unwrap_or_default();
        if catalog().weapon(&weapon).is_none() {
            if let Some(v) = d.at_mut(&p(&["equipedWeapon"])) {
                *v = item_value(ItemKind::Weapon, "Fist");
            }
        }
    }
    let name = dweller_name(&d);
    b.push(p(&DWELLERS), d)?;
    bump_dweller_counter(b, new_id)?;
    if let Some(pet_id) = pet
        .as_ref()
        .and_then(|v| v.get("id"))
        .and_then(Value::as_text)
        .filter(|id| mode == Mode::Expert || catalog().pet(id).is_some())
    {
        equip(b, mode, &[new_id], Slot::Pet, Some(&pet_id))?;
        // Keep the pet's own bonus and name from the other save.
        if let (Some(i), Some(v)) = (dweller_index(b.root(), new_id), pet) {
            let pet_name = v
                .at(&p(&["extraData", "uniqueName"]))
                .and_then(Value::as_text);
            b.set(pi(&DWELLERS, i, &["equippedPet"]), v)?;
            if let (Some(n), Some(ai)) = (pet_name, pet_actor_index(b.root(), new_id)) {
                b.set(pi(&ACTORS, ai, &["name"]), Value::str(&n))?;
            }
        }
    }
    if let Some(u) = text(
        b.get(&pi(
            &DWELLERS,
            dweller_index(b.root(), new_id).unwrap_or(0),
            &[],
        ))
        .unwrap_or(&Value::Null),
        &["uniqueData"],
    )
    .filter(|u| !u.is_empty())
    {
        if catalog()
            .unique_dweller(&u)
            .is_some_and(|x| x.rarity == "Legendary")
        {
            mark_in_journal(b, "dwellers", &u)?;
        }
    }
    Ok(name)
}

fn add_pet(b: &mut Builder<'_>, mode: Mode, id: &str, bonus_value: f64) -> R<()> {
    let pet = catalog().pet(id).ok_or(RuleError::NotInCatalog)?;
    if mode == Mode::Normal
        && !(pet.bonus_min.floor()..=pet.bonus_max.ceil()).contains(&bonus_value)
    {
        return Err(RuleError::OutOfRange {
            min: pet.bonus_min.floor() as i64,
            max: pet.bonus_max.ceil() as i64,
        }
        .into());
    }
    let mut item = item_value(ItemKind::Pet, id);
    if let Some(slot) = item.at_mut(&p(&["extraData", "bonusValue"])) {
        *slot = Value::int(bonus_value.round() as i64);
    }
    b.push(p(&ITEMS), item)?;
    mark_in_journal(b, "pets", &pet.code)?;
    mark_in_journal(b, "breeds", &pet.breed_code.to_string())?;
    Ok(())
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

    fn family() -> Value {
        json::parse(
            r#"{"taskMgr":{"id":9,"tasks":[{"id":5},{"id":6},{"id":7}],"pausedTasks":[]},
            "dwellers":{"dwellers":[
              {"serializeId":1,"gender":2,"relations":{"relations":[{"dweller":2,"value":3}],"partner":2,"lastPartner":-1},"savedRoom":10,
               "equipedOutfit":{"id":"MetalArmor_Sturdy","type":"Outfit","hasBeenAssigned":false,"hasRandonWeaponBeenAssigned":false},
               "equipedWeapon":{"id":"Fist","type":"Weapon","hasBeenAssigned":false,"hasRandonWeaponBeenAssigned":false},
               "equippedPet":{"id":"abyssinian_r","type":"Pet","hasBeenAssigned":false,"hasRandonWeaponBeenAssigned":false,"extraData":{"uniqueName":"C","bonus":"Resistance","bonusValue":40}}},
              {"serializeId":2,"gender":1,"pregnant":true,"relations":{"relations":[{"dweller":1,"value":3}],"partner":1,"lastPartner":1},"savedRoom":10},
              {"serializeId":3,"gender":2,"relations":{"relations":[],"partner":4,"lastPartner":-1},"savedRoom":10},
              {"serializeId":4,"gender":1,"relations":{"relations":[],"partner":3,"lastPartner":-1},"savedRoom":10},
              {"serializeId":5,"gender":1,"relations":{"relations":[],"partner":-1,"lastPartner":-1},"savedRoom":10},
              {"serializeId":6,"gender":2,"relations":{"relations":[],"partner":-1,"lastPartner":-1},"savedRoom":-1}],
             "actors":[{"serializeId":20,"characterType":3,"FollowedID":1}],"id":7,"mrhId":21},
            "vault":{"rooms":[
              {"type":"LivingQuarters","class":"Production","mergeLevel":3,"row":1,"col":0,"level":1,"deserializeID":10,"dwellers":[1,2,3,4,5],
               "partners":[{"m":1,"f":2,"s":"RaisingBaby","fatherId":1,"t":5},{"m":3,"f":4,"s":"MakingBaby","fatherId":3,"t":6}],
               "children":[{"taskID":7,"dwellerID":5,"notificationID":-1}]},
              {"type":"Gym","class":"Training","mergeLevel":1,"row":2,"col":0,"level":1,"deserializeID":11,"dwellers":[]}],
             "inventory":{"items":[]},"wasteland":{"teams":[{"dwellers":[6]}]}},
            "happinessManager":{"rooms":[],"dwellers":[]}}"#,
        )
        .unwrap()
    }

    fn act(root: &mut Value, mode: Mode, a: Action) -> Result<Transaction, ActionError> {
        let orig = root.clone();
        let ctx = Ctx {
            mode,
            original: &orig,
        };
        run(root, &ctx, &a)
    }

    #[test]
    fn delete_father_keeps_pregnancy_and_returns_items() {
        let mut root = family();
        act(
            &mut root,
            Mode::Normal,
            Action::DeleteDwellers { ids: vec![1] },
        )
        .unwrap();
        let room = &arr(&root, &ROOMS)[0];
        let baby = &arr(room, &["partners"])[0];
        assert_eq!(text(baby, &["s"]).as_deref(), Some("RaisingBaby"));
        assert_eq!(int(baby, &["m"]), Some(-1));
        assert_eq!(int(baby, &["fatherId"]), Some(-1));
        let items: Vec<String> = arr(&root, &ITEMS)
            .iter()
            .filter_map(|i| text(i, &["id"]))
            .collect();
        assert_eq!(items, ["MetalArmor_Sturdy", "abyssinian_r"]);
        assert!(arr(&root, &ACTORS).is_empty());
        let mother = &arr(&root, &DWELLERS)[0];
        assert_eq!(int(mother, &["relations", "partner"]), Some(-1));
        assert!(arr(mother, &["relations", "relations"]).is_empty());
    }

    #[test]
    fn delete_mother_and_child_drop_their_tasks() {
        let mut root = family();
        act(
            &mut root,
            Mode::Normal,
            Action::DeleteDwellers { ids: vec![2, 5] },
        )
        .unwrap();
        let room = &arr(&root, &ROOMS)[0];
        assert_eq!(arr(room, &["partners"]).len(), 1);
        assert!(arr(room, &["children"]).is_empty());
        let tasks: Vec<i64> = arr(&root, &["taskMgr", "tasks"])
            .iter()
            .filter_map(|t| int(t, &["id"]))
            .collect();
        assert_eq!(tasks, [6]);
    }

    #[test]
    fn moving_out_breaks_a_couple_without_baby() {
        let mut root = family();
        act(
            &mut root,
            Mode::Normal,
            Action::AssignRoom {
                ids: vec![3],
                room: 11,
            },
        )
        .unwrap();
        let room = &arr(&root, &ROOMS)[0];
        let left: Vec<Option<String>> = arr(room, &["partners"])
            .iter()
            .map(|e| text(e, &["s"]))
            .collect();
        assert_eq!(left, [Some("RaisingBaby".to_string())]);
        assert!(!ints(room, "dwellers").contains(&3));
        assert_eq!(
            int(&arr(&root, &DWELLERS)[3], &["relations", "partner"]),
            Some(-1)
        );
        let tasks: Vec<i64> = arr(&root, &["taskMgr", "tasks"])
            .iter()
            .filter_map(|t| int(t, &["id"]))
            .collect();
        assert_eq!(tasks, [5, 7]);
        // The mother moving away keeps the pregnancy.
        act(
            &mut root,
            Mode::Normal,
            Action::AssignRoom {
                ids: vec![2],
                room: 11,
            },
        )
        .unwrap();
        assert_eq!(arr(&arr(&root, &ROOMS)[0], &["partners"]).len(), 1);
        let err = act(
            &mut root,
            Mode::Normal,
            Action::AssignRoom {
                ids: vec![6],
                room: 11,
            },
        );
        assert!(matches!(
            err,
            Err(ActionError::Rule(RuleError::DwellerAway))
        ));
        assert!(act(
            &mut root,
            Mode::Expert,
            Action::AssignRoom {
                ids: vec![6],
                room: 11
            }
        )
        .is_ok());
    }

    #[test]
    fn day_rating_updates_full_week() {
        let day = |r: i64, t: &str| format!(r#"{{"rating":{r},"time":{t},"collected":true}}"#);
        let week: Vec<String> = (0..7).map(|i| day(80, &format!("{}.5", i * 10))).collect();
        let week = week.join(",");
        let mut root = json::parse(&format!(
            r#"{{"ratingMgr":{{"ratings":[{week}],"ratingsLast":[{week}],"weekRating":80,"lastWeekRating":70}}}}"#
        ))
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let set = Action::SetDayRating {
            index: 6,
            rating: Some(100),
            collected: Some(false),
        };
        run(&mut root, &ctx, &set).unwrap();
        let rm = root.get("ratingMgr").unwrap();
        assert_eq!(rm.get("weekRating").and_then(Value::as_i64), Some(83));
        assert_eq!(rm.get("lastWeekRating").and_then(Value::as_i64), Some(70));
        let last = &rm.get("ratingsLast").and_then(Value::as_array).unwrap()[6];
        assert_eq!(last.get("rating").and_then(Value::as_i64), Some(100));
        assert_eq!(last.get("collected").and_then(Value::as_bool), Some(false));

        let mut part = json::parse(&format!(
            r#"{{"ratingMgr":{{"ratings":[{}],"ratingsLast":[{week}],"weekRating":80}}}}"#,
            day(50, "99.5")
        ))
        .unwrap();
        let orig = part.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let set = Action::SetDayRating {
            index: 0,
            rating: Some(10),
            collected: None,
        };
        run(&mut part, &ctx, &set).unwrap();
        let rm = part.get("ratingMgr").unwrap();
        assert_eq!(rm.get("weekRating").and_then(Value::as_i64), Some(80));
        assert!(rm.to_compact().contains(r#""rating":10"#));
        assert_eq!(week_average(&[84, 80, 80, 81, 82, 96, 96]), Some(86));
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
            &Action::SetMergeLevel {
                room: 1,
                merge: 2,
                apart: false,
            }
        )
        .is_ok());
        assert!(run(
            &mut root,
            &ctx,
            &Action::SetMergeLevel {
                room: 1,
                merge: 3,
                apart: false,
            }
        )
        .is_err());
        assert!(run(
            &mut root,
            &ctx,
            &Action::SetMergeLevel {
                room: 2,
                merge: 3,
                apart: false,
            }
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
            unlock: false,
            apart: false,
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
        let sized = |kind: &str, row, col, merge| Action::AddRoom {
            kind: kind.into(),
            row,
            col,
            merge,
            level: 1,
            unlock: false,
            apart: false,
        };
        run(&mut root, &ctx, &sized("Overseer", 1, 0, 2)).unwrap_err();
        run(&mut root, &ctx, &add("Elevator", 2, 9)).unwrap();
        assert_eq!(
            err(run(&mut root, &ctx, &add("Overseer", 2, 10))),
            RuleError::RoomSize
        );
        run(&mut root, &ctx, &sized("Overseer", 2, 10, 2)).unwrap();
        assert_eq!(
            err(run(&mut root, &ctx, &sized("Overseer", 2, 3, 2))),
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
                    col: 10,
                    apart: false,
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
                col: 20,
                apart: false,
            },
        )
        .unwrap_err();
        run(
            &mut root,
            &ctx,
            &Action::MoveRoom {
                room: 6,
                row: 2,
                col: 16,
                apart: false,
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

        run(
            &mut root,
            &ctx,
            &Action::SetRoomLevel {
                room: 4,
                level: 3,
                apart: false,
            },
        )
        .unwrap();
        run(
            &mut root,
            &ctx,
            &Action::SetRoomLevel {
                room: 4,
                level: 4,
                apart: false,
            },
        )
        .unwrap_err();
    }

    #[test]
    fn dweller_import_and_quest_memory() {
        let mut root = json::parse(
            r#"{"dwellers":{"dwellers":[{"serializeId":1,"name":"A","gender":1}],"actors":[],"id":1,"mrhId":1},
            "survivalW":{"dwellers":[]},
            "completedQuestDataManager":{"dailyQuestPicker":{"historyDailies":[{"questName":"Daily_01"},{"questName":"Daily_02"}]}}}"#,
        )
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let other = r#"{"serializeId":7,"name":"Bob","lastName":"X","gender":2,"savedRoom":44,"assigned":true,
            "relations":{"relations":[{"id":3}],"partner":9,"lastPartner":9,"ascendants":[3,4,-1,-1,-1,-1]},
            "equipedOutfit":{"id":"NoSuchOutfit","type":"Outfit"},"equipedWeapon":{"id":"Fist","type":"Weapon"},
            "dwellerRoom":5,"daysOnWasteland":3}"#;
        run(
            &mut root,
            &ctx,
            &Action::ImportDweller { json: other.into() },
        )
        .unwrap();
        let d = &arr(&root, &DWELLERS)[1];
        assert_eq!(int(d, &["serializeId"]), Some(2));
        assert_eq!(int(d, &["savedRoom"]), Some(-1));
        assert_eq!(int(d, &["relations", "partner"]), Some(-1));
        assert!(arr(d, &["relations", "relations"]).is_empty());
        assert_eq!(arr(d, &["relations", "ascendants"])[0].as_i64(), Some(-1));
        assert_eq!(
            text(d, &["equipedOutfit", "id"]).as_deref(),
            Some("jumpsuit")
        );
        assert!(d.get("dwellerRoom").is_none());
        assert_eq!(int(d, &["daysOnWasteland"]), Some(0));
        run(
            &mut root,
            &ctx,
            &Action::ForgetQuest {
                list: "dailyHistory".into(),
                quest: "Daily_01".into(),
            },
        )
        .unwrap();
        let h = arr(
            &root,
            &[
                "completedQuestDataManager",
                "dailyQuestPicker",
                "historyDailies",
            ],
        );
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn outfits_follow_gender_and_colours() {
        let mut root = json::parse(
            r#"{"dwellers":{"dwellers":[
              {"serializeId":1,"gender":1,"outfitColor":4294967295,"equipedOutfit":{"id":"jumpsuit","type":"Outfit","hasBeenAssigned":false,"hasRandonWeaponBeenAssigned":false}},
              {"serializeId":2,"gender":2,"outfitColor":4294967295,"equipedOutfit":{"id":"jumpsuit","type":"Outfit","hasBeenAssigned":false,"hasRandonWeaponBeenAssigned":false}}]}}"#,
        )
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let male_only = "AbrahamSpecial";
        assert!(!catalog().outfit_fits(male_only, 1));
        let equip = |ids: Vec<i64>| Action::Equip {
            ids,
            slot: Slot::Outfit,
            item: Some(male_only.into()),
        };
        assert!(matches!(
            run(&mut root, &ctx, &equip(vec![1])),
            Err(ActionError::Rule(RuleError::OutfitGender))
        ));
        assert_eq!(
            run(&mut root, &ctx, &equip(vec![1, 2])).unwrap().affected,
            1
        );
        let colors = &catalog().outfit_art(male_only, 2).unwrap().colors;
        let d = &arr(&root, &DWELLERS)[1];
        assert_eq!(
            int(d, &["outfitColor"]),
            colors.first().map(|c| i64::from(*c))
        );
        run(
            &mut root,
            &ctx,
            &Action::SetColor {
                ids: vec![2],
                field: "outfitColor".into(),
                rgb: 0x123456,
            },
        )
        .ok();
        let v = int(&arr(&root, &DWELLERS)[1], &["outfitColor"]).unwrap() as u32;
        assert!(colors.contains(&v));
    }

    #[test]
    fn room_grows_left_when_right_is_taken() {
        let mut root = json::parse(
            r#"{"vault":{"rooms":[
              {"type":"Storage","class":"Facility","mergeLevel":1,"row":1,"col":12,"level":1,"dwellers":[],"deserializeID":1},
              {"type":"Elevator","class":"Utility","mergeLevel":1,"row":1,"col":15,"level":1,"dwellers":[],"deserializeID":2}],
              "rocks":[]}}"#,
        )
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Expert,
            original: &orig,
        };
        assert_eq!(merge_col(&root, Mode::Expert, 1, 2), Some(9));
        assert_eq!(merge_col(&root, Mode::Expert, 1, 3), Some(6));
        run(
            &mut root,
            &ctx,
            &Action::SetMergeLevel {
                room: 1,
                merge: 3,
                apart: false,
            },
        )
        .unwrap();
        let r = &arr(&root, &ROOMS)[0];
        assert_eq!(
            (int(r, &["col"]), int(r, &["mergeLevel"])),
            (Some(6), Some(3))
        );
    }

    #[test]
    fn hints_objectives_and_quests() {
        let mut root = json::parse(
            r#"{"tutorialManager":{"phase":"Phase5","ContextualBabies":false,"ContextualJunk":true},
            "objectiveMgr":{"slotArray":[{"objective":{"objectiveID":"MakeFriend1","requirements":[{"requirementID":"r1","satisfied":false}],"completed":false,"incrementLevel":1},"incLevel":1}]},
            "completedQuestDataManager":{"completedQuests":["TutorialQuest_01"]}}"#,
        )
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        run(&mut root, &ctx, &Action::SetHints { shown: true }).unwrap();
        assert_eq!(
            root.at(&p(&["tutorialManager", "ContextualBabies"])),
            Some(&Value::Bool(true))
        );
        run(&mut root, &ctx, &Action::CompleteObjective { slot: 0 }).unwrap();
        let obj = p(&["objectiveMgr", "slotArray"]);
        let done = root
            .at(&obj)
            .and_then(|a| a.at(&[Seg::Index(0), Seg::Key("objective".into())]))
            .cloned()
            .unwrap();
        assert_eq!(done.get("completed"), Some(&Value::Bool(true)));
        assert_eq!(
            done.at(&[
                Seg::Key("requirements".into()),
                Seg::Index(0),
                Seg::Key("satisfied".into())
            ]),
            Some(&Value::Bool(true))
        );
        let quest = catalog().quests[0].id.clone();
        run(
            &mut root,
            &ctx,
            &Action::MarkQuest {
                id: quest.clone(),
                done: true,
            },
        )
        .unwrap();
        let list = |r: &Value| {
            arr(r, &["completedQuestDataManager", "completedQuests"])
                .iter()
                .filter_map(Value::as_text)
                .collect::<Vec<_>>()
        };
        assert!(list(&root).contains(&quest));
        run(
            &mut root,
            &ctx,
            &Action::MarkQuest {
                id: quest.clone(),
                done: false,
            },
        )
        .unwrap();
        assert!(!list(&root).contains(&quest));
        assert!(run(
            &mut root,
            &ctx,
            &Action::MarkQuest {
                id: "NoSuchQuest".into(),
                done: true
            }
        )
        .is_err());
    }
    #[test]
    fn objectives_achievements_and_timers() {
        let mut root = json::parse(
            r#"{"taskMgr":{"id":9,"time":100.00,"tasks":[{"startTime":50.00,"endTime":150.00,"id":8,"paused":false}]},
            "vault":{"VaultMode":"Normal","Achievements":{"objectivesInProgress":[{"objectiveID":"merge_food_01","requirements":[],"completed":false,"incrementLevel":0}],"completed":[]}},
            "objectiveMgr":{"taskID":8,"slotArray":[
              {"objective":{"objectiveID":"MakeFriend1","requirements":[{"requirementID":"r1","satisfied":false}],"completed":false,"incrementLevel":1},"incLevel":1},
              {"objective":{"objectiveID":"Babies1","requirements":[],"completed":false,"incrementLevel":0},"incLevel":1}]}}"#,
        )
        .unwrap();
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let slot0 = |r: &Value| {
            r.at(&p(&["objectiveMgr", "slotArray"]))
                .and_then(|a| a.at(&[Seg::Index(0), Seg::Key("objective".into())]))
                .cloned()
                .unwrap()
        };
        let replace = |id: &str, level: i64| Action::ReplaceObjective {
            slot: 0,
            id: id.into(),
            level,
        };
        assert!(run(&mut root, &ctx, &replace("Babies1", 0)).is_err());
        assert!(run(&mut root, &ctx, &replace("KillDeathclawAtVaultDoor1", 0)).is_err());
        assert!(run(&mut root, &ctx, &replace("Babies2", 99)).is_err());
        run(&mut root, &ctx, &replace("Babies2", 2)).unwrap();
        let o = slot0(&root);
        assert_eq!(text(&o, &["objectiveID"]).as_deref(), Some("Babies2"));
        assert_eq!(int(&o, &["incrementLevel"]), Some(2));
        let info = catalog().objective("Babies2").unwrap();
        assert_eq!(info.goal_value(2), 6);
        assert!(info.text(false, 2, &[]).contains('6'));
        run(&mut root, &ctx, &Action::CompleteObjective { slot: 0 }).unwrap();
        let o = slot0(&root);
        assert_eq!(o.get("completed"), Some(&Value::Bool(true)));
        assert_eq!(arr(&o, &["requirements"]).len(), 1);

        let done = |r: &Value| {
            arr(r, &["vault", "Achievements", "completed"])
                .iter()
                .filter_map(Value::as_text)
                .collect::<Vec<_>>()
        };
        let ach = |done: bool| Action::SetAchievement {
            id: "merge_food_01".into(),
            done,
        };
        run(&mut root, &ctx, &ach(true)).unwrap();
        assert_eq!(done(&root), vec!["merge_food_01".to_string()]);
        assert!(arr(&root, &["vault", "Achievements", "objectivesInProgress"]).is_empty());
        run(&mut root, &ctx, &ach(false)).unwrap();
        assert!(done(&root).is_empty());

        run(
            &mut root,
            &ctx,
            &Action::SetTaskTime {
                id: 8,
                seconds: 10.0,
            },
        )
        .unwrap();
        let t = &arr(&root, &["taskMgr", "tasks"])[0];
        assert_eq!(t.get("endTime").and_then(Value::as_f64), Some(110.0));
        assert_eq!(t.get("startTime").and_then(Value::as_f64), Some(10.0));
    }

    #[test]
    fn blueprints_themes_incidents_and_journal() {
        let mut root = vault_with_lift();
        let extra = json::parse(
            r#"{"unlockableMgr":{"objectivesInProgress":[{"objectiveID":"BarUnlock","requirements":[]}],"completed":[],"claimed":["StorageUnlock","GymUnlock"]},
            "survivalW":{"weapons":["N1"],"outfits":[],"dwellers":[],"pets":[],"breeds":[],"decorations":[],"junk":["OAlarmClock"],"recipes":[],"claimedRecipes":[],"collectedThemes":{"themeList":[]}},
            "specialTheme":{"themeByRoomType":{},"eventsThemes":{"Cafeteria_Xmas":{"id":"Cafeteria_Xmas","extraData":{"IsNew":true}}}}}"#,
        )
        .unwrap();
        let vault_extra = json::parse(
            r#"{"emergencyData":{"active":false,"randomEventTaskId":-1},
            "wasteland":{"teams":[{"teamEquipment":{"storage":{},"inventory":{"items":[]},"collectedThemes":{"themeList":[]}}}]}}"#,
        )
        .unwrap();
        let top = root.as_object_mut().unwrap();
        top.0.extend(extra.as_object().unwrap().0.iter().cloned());
        let vault = root
            .at_mut(&p(&["vault"]))
            .unwrap()
            .as_object_mut()
            .unwrap();
        vault
            .0
            .extend(vault_extra.as_object().unwrap().0.iter().cloned());
        let orig = root.clone();
        let ctx = Ctx {
            mode: Mode::Normal,
            original: &orig,
        };
        let bar = |unlock| Action::AddRoom {
            kind: "Bar".into(),
            row: 1,
            col: 16,
            merge: 1,
            level: 1,
            unlock,
            apart: false,
        };
        assert!(matches!(
            run(&mut root, &ctx, &bar(false)),
            Err(ActionError::Rule(RuleError::RoomLocked))
        ));
        run(&mut root, &ctx, &bar(true)).unwrap();
        assert!(arr(&root, &["unlockableMgr", "objectivesInProgress"]).is_empty());
        assert!(room_lock(&root, "Bar").is_none());
        assert!(integrity::check(&root)
            .iter()
            .all(|i| i.code != "room_locked"));
        let lock = Action::SetRoomUnlocked {
            room: "Bar".into(),
            unlocked: false,
        };
        assert!(run(&mut root, &ctx, &lock).is_err());

        run(&mut root, &ctx, &Action::MarkCollectionRead).unwrap();
        assert_eq!(
            arr(&root, &["survivalW", "weapons"])[0]
                .as_text()
                .as_deref(),
            Some("O1")
        );
        let ev = p(&[
            "specialTheme",
            "eventsThemes",
            "Cafeteria_Xmas",
            "extraData",
            "IsNew",
        ]);
        assert_eq!(root.at(&ev).and_then(Value::as_bool), Some(false));

        let theme = "LivingQuartersBrotherOfStell";
        run(
            &mut root,
            &ctx,
            &Action::SetRoomTheme {
                room: "LivingQuarters".into(),
                theme: Some(theme.into()),
            },
        )
        .unwrap();
        let applied = p(&["specialTheme", "themeByRoomType", "LivingQuarters"]);
        assert_eq!(
            root.at(&applied).and_then(Value::as_text).as_deref(),
            Some("BrotherOfSteel")
        );
        assert_eq!(theme_state(&root, theme), (9, true));
        assert!(arr(&root, &["survivalW", "recipes"])
            .iter()
            .any(|v| v.as_text().as_deref() == Some(theme)));
        run(
            &mut root,
            &ctx,
            &Action::SetThemeState {
                id: theme.into(),
                parts: 3,
                crafted: false,
            },
        )
        .unwrap();
        assert_eq!(theme_state(&root, theme), (3, false));
        assert!(root.at(&applied).is_none());

        run(
            &mut root,
            &ctx,
            &Action::StartIncident {
                room: 4,
                kind: 1,
                strength: 0.5,
            },
        )
        .unwrap();
        let lq = &arr(&root, &ROOMS)[3];
        assert_eq!(
            text(lq, &["currentStateName"]).as_deref(),
            Some("RoomEmergency")
        );
        assert_eq!(
            lq.at(&p(&["roomHealth", "damageValue"]))
                .and_then(Value::as_f64),
            Some(50.0)
        );
        assert_eq!(
            flag(&root, &["vault", "emergencyData", "active"]),
            Some(true)
        );
        run(&mut root, &ctx, &Action::FinishEmergency).unwrap();
        assert_eq!(
            text(&arr(&root, &ROOMS)[3], &["currentStateName"]).as_deref(),
            Some("Idle")
        );
        assert_eq!(
            flag(&root, &["vault", "emergencyData", "active"]),
            Some(false)
        );
        assert!(root.at(&p(&["vault", "emergencyData", "mode"])).is_none());

        run(
            &mut root,
            &ctx,
            &Action::AddUniqueDweller {
                id: "L_Preston".into(),
            },
        )
        .unwrap();
        let d = arr(&root, &DWELLERS).last().unwrap();
        assert_eq!(text(d, &["uniqueData"]).as_deref(), Some("L_Preston"));
        assert_eq!(text(d, &["hair"]).as_deref(), Some("16"));
        assert_eq!(special(d, 3), Some(8));
        assert_eq!(int(d, &["gender"]), Some(2));
        assert!(arr(&root, &["survivalW", "dwellers"])
            .iter()
            .any(|v| v.as_text().as_deref() == Some("NL_Preston")));

        let add = Action::AddTeamItems {
            team: 0,
            kind: ItemKind::Weapon,
            id: "BBGun".into(),
            count: 2,
            recipe: false,
        };
        run(&mut root, &ctx, &add).unwrap();
        let items = p(&["vault", "wasteland", "teams"]);
        let team = &arr(&root, &["vault", "wasteland", "teams"])[0];
        assert_eq!(team_load(team), 2);
        run(
            &mut root,
            &ctx,
            &Action::RemoveTeamItems {
                team: 0,
                indices: vec![0],
            },
        )
        .unwrap();
        assert_eq!(
            team_load(&root.at(&items).unwrap().as_array().unwrap()[0]),
            1
        );
    }
}
