// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3 of the License.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Integrity rules: ids, counters and cross references.
//!
//! Rules follow what the game itself writes. Two rules from the original
//! description are not errors in real saves and are kept only as automatic
//! maintenance during edits: `dwellers.id` (the game never reads it) and
//! `savedRoom` matching the room list (often differs in game files).
//! Room references in objective requirements and `viewManager` are not
//! checked either: the game leaves them pointing at removed rooms.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::Serialize;

use crate::edit::{Builder, EditError};
use crate::json::{path_to_string, Seg, Value};
use crate::vault::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub code: &'static str,
    pub severity: Severity,
    pub path: Vec<Seg>,
    pub params: Vec<(String, String)>,
    pub fixable: bool,
}

impl Issue {
    fn new(code: &'static str, severity: Severity, path: Vec<Seg>) -> Issue {
        Issue { code, severity, path, params: Vec::new(), fixable: true }
    }

    fn param(mut self, k: &str, v: impl ToString) -> Issue {
        self.params.push((k.to_string(), v.to_string()));
        self
    }

    fn manual(mut self) -> Issue {
        self.fixable = false;
        self
    }

    /// Stable identity used to tell pre-existing problems from new ones.
    pub fn fingerprint(&self) -> String {
        let mut s = format!("{}|{}", self.code, path_to_string(&self.path));
        for (k, v) in &self.params {
            s.push_str(&format!("|{k}={v}"));
        }
        s
    }
}

fn seg_path(base: &[&str], i: usize, rest: &[&str]) -> Vec<Seg> {
    pi(base, i, rest)
}

pub fn check(root: &Value) -> Vec<Issue> {
    let mut out = Vec::new();
    let dwellers = arr(root, &DWELLERS);
    let rooms = arr(root, &ROOMS);
    let actors = arr(root, &ACTORS);

    let mut ids: HashMap<i64, usize> = HashMap::new();
    for (i, d) in dwellers.iter().enumerate() {
        match int(d, &["serializeId"]) {
            Some(id) => {
                if !(0..=32767).contains(&id) {
                    out.push(Issue::new("dweller_id_range", Severity::Error, seg_path(&DWELLERS, i, &["serializeId"])).param("id", id).manual());
                }
                if ids.insert(id, i).is_some() {
                    out.push(Issue::new("dweller_id_duplicate", Severity::Error, seg_path(&DWELLERS, i, &["serializeId"])).param("id", id).manual());
                }
            }
            None => out.push(Issue::new("dweller_id_missing", Severity::Error, seg_path(&DWELLERS, i, &[])).manual()),
        }
    }
    let quest_ids: HashSet<i64> = arr(root, &QUEST_DWELLERS).iter().filter_map(|d| int(d, &["serializeId"])).collect();
    let known_dweller = |id: i64| ids.contains_key(&id) || quest_ids.contains(&id);

    let mut actor_ids = HashSet::new();
    let mut max_actor = -1;
    for (i, a) in actors.iter().enumerate() {
        if let Some(id) = int(a, &["serializeId"]) {
            max_actor = max_actor.max(id);
            if !actor_ids.insert(id) {
                out.push(Issue::new("actor_id_duplicate", Severity::Error, seg_path(&ACTORS, i, &["serializeId"])).param("id", id).manual());
            }
        }
        if let Some(f) = int(a, &["FollowedID"]) {
            if f != -1 && !known_dweller(f) {
                out.push(Issue::new("pet_owner_missing", Severity::Error, seg_path(&ACTORS, i, &["FollowedID"])).param("id", f));
            }
        }
    }
    if let Some(ctr) = int(root, &["dwellers", "mrhId"]) {
        if ctr <= max_actor {
            out.push(Issue::new("actor_counter", Severity::Error, p(&["dwellers", "mrhId"])).param("value", ctr).param("max", max_actor));
        }
    }

    let mut room_set = HashSet::new();
    let mut max_room = -1;
    for (i, r) in rooms.iter().enumerate() {
        if let Some(id) = int(r, &["deserializeID"]) {
            max_room = max_room.max(id);
            if !room_set.insert(id) {
                out.push(Issue::new("room_id_duplicate", Severity::Error, seg_path(&ROOMS, i, &["deserializeID"])).param("id", id).manual());
            }
        }
    }
    if let Some(ctr) = int(root, &["constructMgr", "roomDeserializeID"]) {
        if ctr < max_room {
            out.push(Issue::new("room_counter", Severity::Error, p(&["constructMgr", "roomDeserializeID"])).param("value", ctr).param("max", max_room));
        }
    }

    let mut seen_in_room: HashMap<i64, i64> = HashMap::new();
    for (i, r) in rooms.iter().enumerate() {
        let rid = int(r, &["deserializeID"]).unwrap_or(-1);
        for (j, v) in r.get("dwellers").and_then(Value::as_array).map_or(&[][..], Vec::as_slice).iter().enumerate() {
            let Some(id) = v.as_i64() else { continue };
            let mut path = seg_path(&ROOMS, i, &["dwellers"]);
            path.push(Seg::Index(j));
            if !ids.contains_key(&id) {
                out.push(Issue::new("room_dweller_missing", Severity::Error, path).param("id", id).param("room", rid));
            } else if let Some(other) = seen_in_room.insert(id, rid) {
                out.push(Issue::new("dweller_in_two_rooms", Severity::Error, path).param("id", id).param("room", rid).param("other", other));
            }
        }
        for (j, s) in r.get("slots").and_then(Value::as_array).map_or(&[][..], Vec::as_slice).iter().enumerate() {
            if let Some(id) = int(s, &["dwellerID"]) {
                if id != -1 && !ids.contains_key(&id) {
                    let mut path = seg_path(&ROOMS, i, &["slots"]);
                    path.push(Seg::Index(j));
                    path.push(Seg::Key("dwellerID".into()));
                    out.push(Issue::new("slot_dweller_missing", Severity::Error, path).param("id", id).param("room", rid));
                }
            }
        }
    }

    for (i, d) in dwellers.iter().enumerate() {
        let me = int(d, &["serializeId"]).unwrap_or(-1);
        for key in ["partner", "lastPartner"] {
            if let Some(pid) = int(d, &["relations", key]) {
                if pid != -1 && !ids.contains_key(&pid) {
                    out.push(Issue::new("partner_missing", Severity::Error, seg_path(&DWELLERS, i, &["relations", key])).param("id", pid));
                }
            }
        }
        if let Some(pid) = int(d, &["relations", "partner"]) {
            if let Some(&j) = ids.get(&pid) {
                if int(&dwellers[j], &["relations", "partner"]) != Some(me) {
                    out.push(Issue::new("partner_not_mutual", Severity::Warning, seg_path(&DWELLERS, i, &["relations", "partner"])).param("id", me).param("partner", pid));
                }
            }
        }
        if let Some(rid) = int(d, &["savedRoom"]) {
            if rid != -1 && !room_set.contains(&rid) {
                out.push(Issue::new("saved_room_missing", Severity::Error, seg_path(&DWELLERS, i, &["savedRoom"])).param("room", rid));
            }
        }
        for key in ["skinColor", "hairColor", "outfitColor"] {
            if let Some(c) = int(d, &[key]) {
                if (c as u64 >> 24) & 0xff != 0xff {
                    out.push(Issue::new("color_alpha", Severity::Warning, seg_path(&DWELLERS, i, &[key])).param("id", me));
                }
            }
        }
    }

    let team = p(&["questDataManager", "questTeam"]);
    if let Some(Value::Array(list)) = root.at(&join(&team, &["DwellersDictionary"])) {
        for (j, v) in list.iter().enumerate() {
            if let Some(id) = v.as_i64() {
                if !known_dweller(id) {
                    let mut path = join(&team, &["DwellersDictionary"]);
                    path.push(Seg::Index(j));
                    out.push(Issue::new("quest_dweller_missing", Severity::Warning, path).param("id", id));
                }
            }
        }
    }
    if let Some(Value::Array(list)) = root.at(&join(&team, &["InitialEquipment"])) {
        for (j, v) in list.iter().enumerate() {
            if let Some(id) = int(v, &["dweller"]) {
                if id != -1 && !known_dweller(id) {
                    let mut path = join(&team, &["InitialEquipment"]);
                    path.push(Seg::Index(j));
                    out.push(Issue::new("quest_dweller_missing", Severity::Warning, path).param("id", id));
                }
            }
        }
    }

    check_happiness(root, &ids, &room_set, &mut out);
    check_lunchboxes(root, &mut out);
    check_tasks(root, &mut out);
    check_requirements(root, &ids, &mut out);
    out
}

fn check_happiness(root: &Value, ids: &HashMap<i64, usize>, rooms: &HashSet<i64>, out: &mut Vec<Issue>) {
    let Some(hm) = root.get("happinessManager").and_then(Value::as_object) else { return };
    for (prefix, list_key) in [("room", "rooms"), ("dweller", "dwellers")] {
        let keys: BTreeSet<i64> = hm
            .keys()
            .filter_map(|k| k.strip_prefix(prefix).and_then(|n| n.parse::<i64>().ok()))
            .collect();
        let listed: BTreeSet<i64> =
            hm.get(list_key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
        for id in keys.symmetric_difference(&listed) {
            out.push(Issue::new("happiness_list_mismatch", Severity::Warning, p(&["happinessManager", list_key])).param("id", id));
        }
        for id in keys.union(&listed) {
            let exists = if prefix == "room" { rooms.contains(id) } else { ids.contains_key(id) };
            if !exists {
                out.push(Issue::new("happiness_stale", Severity::Warning, p(&["happinessManager"])).param("key", format!("{prefix}{id}")));
            }
        }
    }
}

fn check_lunchboxes(root: &Value, out: &mut Vec<Issue>) {
    let Some(v) = root.get("vault") else { return };
    let types = v.get("LunchBoxesByType").and_then(Value::as_array);
    let len = types.map_or(0, Vec::len) as i64;
    if let Some(count) = int(v, &["LunchBoxesCount"]) {
        if types.is_some() && count != len {
            out.push(Issue::new("lunchbox_count", Severity::Error, p(&["vault", "LunchBoxesCount"])).param("value", count).param("expected", len));
        }
    }
    if let Some(Value::Array(names)) = v.get("LunchBoxDeckNames") {
        if !names.is_empty() && names.len() as i64 != len {
            out.push(Issue::new("lunchbox_deck_names", Severity::Warning, p(&["vault", "LunchBoxDeckNames"])).param("value", names.len()).param("expected", len));
        }
    }
}

fn check_tasks(root: &Value, out: &mut Vec<Issue>) {
    let tasks: HashSet<i64> = task_ids(root, "taskMgr").into_iter().collect();
    let max = tasks.iter().copied().max().unwrap_or(-1);
    if let Some(ctr) = int(root, &["taskMgr", "id"]) {
        if ctr < max {
            out.push(Issue::new("task_counter", Severity::Error, p(&["taskMgr", "id"])).param("value", ctr).param("max", max));
        }
    }
    for (i, r) in arr(root, &ROOMS).iter().enumerate() {
        let mut refs: Vec<Vec<&str>> = vec![vec!["rushTask"], vec!["currentState", "taskId"], vec!["currentState", "breedingTaskId"]];
        let slots = r.get("slots").and_then(Value::as_array).map_or(0, Vec::len);
        let slot_paths: Vec<Vec<Seg>> = (0..slots)
            .map(|j| {
                let mut path = seg_path(&ROOMS, i, &["slots"]);
                path.push(Seg::Index(j));
                path.push(Seg::Key("taskID".into()));
                path
            })
            .collect();
        let mut paths: Vec<Vec<Seg>> = refs.drain(..).map(|rel| seg_path(&ROOMS, i, &rel)).collect();
        paths.extend(slot_paths);
        for path in paths {
            if let Some(id) = root.at(&path).and_then(Value::as_i64) {
                if id != -1 && !tasks.contains(&id) {
                    out.push(Issue::new("task_missing", Severity::Error, path).param("task", id));
                }
            }
        }
    }
}

fn check_requirements(root: &Value, ids: &HashMap<i64, usize>, out: &mut Vec<Issue>) {
    fn walk(v: &Value, path: &mut Vec<Seg>, ids: &HashMap<i64, usize>, out: &mut Vec<Issue>) {
        match v {
            Value::Object(o) => {
                if o.get("requirementID").is_some() {
                    if let Some(d) = o.get("dweller").and_then(Value::as_i64) {
                        if d != -1 && !ids.contains_key(&d) {
                            let mut pth = path.clone();
                            pth.push(Seg::Key("dweller".into()));
                            out.push(Issue::new("requirement_dweller_missing", Severity::Warning, pth).param("id", d));
                        }
                    }
                }
                for (k, child) in &o.0 {
                    path.push(Seg::Key(k.text()));
                    walk(child, path, ids, out);
                    path.pop();
                }
            }
            Value::Array(a) => {
                for (i, child) in a.iter().enumerate() {
                    path.push(Seg::Index(i));
                    walk(child, path, ids, out);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    for key in ["objectiveMgr", "unlockableMgr"] {
        if let Some(v) = root.get(key) {
            let mut path = vec![Seg::Key(key.into())];
            walk(v, &mut path, ids, out);
        }
    }
    if let Some(v) = root.at(&p(&["vault", "Achievements"])) {
        let mut path = p(&["vault", "Achievements"]);
        walk(v, &mut path, ids, out);
    }
}

fn param<'a>(issue: &'a Issue, key: &str) -> Option<&'a str> {
    issue.params.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// Applies the automatic fix of one issue. Returns false when nothing was done.
pub fn fix(b: &mut Builder<'_>, issue: &Issue) -> Result<bool, EditError> {
    if !issue.fixable {
        return Ok(false);
    }
    let num = |k: &str| param(issue, k).and_then(|v| v.parse::<i64>().ok());
    match issue.code {
        "pet_owner_missing" | "slot_dweller_missing" | "partner_missing" | "saved_room_missing" | "task_missing"
        | "requirement_dweller_missing" | "partner_not_mutual" => {
            b.set(issue.path.clone(), Value::int(-1))
        }
        "actor_counter" => b.set(issue.path.clone(), Value::int(num("max").unwrap_or(0) + 1)),
        "room_counter" | "task_counter" => b.set(issue.path.clone(), Value::int(num("max").unwrap_or(0))),
        "lunchbox_count" => b.set(issue.path.clone(), Value::int(num("expected").unwrap_or(0))),
        "lunchbox_deck_names" => {
            let expected = num("expected").unwrap_or(0).max(0) as usize;
            let mut names: Vec<Value> = b.get(&issue.path).and_then(Value::as_array).cloned().unwrap_or_default();
            names.resize(expected, Value::str("Normal"));
            b.set(issue.path.clone(), Value::Array(names))
        }
        "room_dweller_missing" | "dweller_in_two_rooms" | "quest_dweller_missing" => {
            let (parent, last) = issue.path.split_at(issue.path.len() - 1);
            match last.first() {
                Some(Seg::Index(i)) => {
                    b.remove(parent.to_vec(), *i)?;
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
        "color_alpha" => {
            let c = b.get(&issue.path).and_then(Value::as_i64).unwrap_or(0);
            b.set(issue.path.clone(), Value::int((c & 0x00ff_ffff) | 0xff00_0000))
        }
        "happiness_list_mismatch" | "happiness_stale" => {
            let rooms: HashSet<i64> = room_ids(b.root()).into_iter().collect();
            let ids: HashSet<i64> = dweller_ids(b.root()).into_iter().collect();
            sync_happiness(b, &rooms, &ids)
        }
        _ => Ok(false),
    }
}

/// Makes `happinessManager` lists and dynamic keys agree with existing rooms and dwellers.
pub fn sync_happiness(b: &mut Builder<'_>, rooms: &HashSet<i64>, ids: &HashSet<i64>) -> Result<bool, EditError> {
    let hm_path = p(&["happinessManager"]);
    let Some(hm) = b.get(&hm_path).and_then(Value::as_object).cloned() else { return Ok(false) };
    let mut changed = false;
    let mut keep: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    for (prefix, list_key, exists) in [("room", "rooms", rooms), ("dweller", "dwellers", ids)] {
        let mut valid = Vec::new();
        for k in hm.keys() {
            if let Some(id) = k.strip_prefix(prefix).and_then(|n| n.parse::<i64>().ok()) {
                if exists.contains(&id) {
                    valid.push(id);
                } else {
                    changed |= b.remove_key(hm_path.clone(), &k)?;
                }
            }
        }
        keep.insert(list_key, valid);
    }
    for (list_key, valid) in keep {
        if let Some(Value::Array(cur)) = hm.get(list_key) {
            let current: Vec<i64> = cur.iter().filter_map(Value::as_i64).collect();
            let mut want: Vec<i64> = current.iter().copied().filter(|id| valid.contains(id)).collect();
            for id in &valid {
                if !want.contains(id) {
                    want.push(*id);
                }
            }
            if want != current {
                changed |= b.set(join(&hm_path, &[list_key]), Value::Array(want.into_iter().map(Value::int).collect()))?;
            }
        }
    }
    Ok(changed)
}

/// Fixes everything that can be fixed automatically.
pub fn fix_all(b: &mut Builder<'_>) -> Result<usize, EditError> {
    let mut fixed = 0;
    for _ in 0..64 {
        let issues = check(b.root());
        let Some(issue) = issues.iter().find(|i| i.fixable) else { break };
        if fix(b, issue)? {
            fixed += 1;
        } else {
            break;
        }
    }
    Ok(fixed)
}

/// Splits issues into those already in the original file and new ones.
pub fn classify(original: &[Issue], current: Vec<Issue>) -> (Vec<Issue>, Vec<Issue>) {
    let before: HashSet<String> = original.iter().map(Issue::fingerprint).collect();
    current.into_iter().partition(|i| before.contains(&i.fingerprint()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn detects_and_fixes_broken_refs() {
        let mut root = parse(
            r#"{"taskMgr":{"id":5,"tasks":[{"id":5}],"pausedTasks":[]},
            "dwellers":{"dwellers":[{"serializeId":1,"relations":{"partner":2,"lastPartner":-1},"savedRoom":9,"skinColor":16777215},
                                    {"serializeId":2,"relations":{"partner":-1,"lastPartner":-1},"savedRoom":-1}],
                        "actors":[{"serializeId":1,"FollowedID":7}],"id":0,"mrhId":1},
            "constructMgr":{"roomDeserializeID":1},
            "vault":{"rooms":[{"deserializeID":3,"dwellers":[1,4],"rushTask":6},{"deserializeID":4,"dwellers":[1]}],
                     "LunchBoxesByType":[0,1],"LunchBoxDeckNames":["Normal"],"LunchBoxesCount":5},
            "happinessManager":{"rooms":[3],"dwellers":[1],"room3":[],"dweller1":[],"dweller8":[]},
            "viewManager":{"currentRoom":3}}"#,
        )
        .unwrap();
        let issues = check(&root);
        let codes: BTreeSet<&str> = issues.iter().map(|i| i.code).collect();
        for c in [
            "pet_owner_missing", "actor_counter", "room_counter", "room_dweller_missing", "dweller_in_two_rooms",
            "partner_not_mutual", "saved_room_missing", "color_alpha", "lunchbox_count", "lunchbox_deck_names",
            "task_missing", "happiness_list_mismatch", "happiness_stale",
        ] {
            assert!(codes.contains(c), "missing {c}: {codes:?}");
        }
        let mut b = Builder::new(&mut root);
        fix_all(&mut b).unwrap();
        let left: Vec<Issue> = check(b.root()).into_iter().filter(|i| i.fixable).collect();
        assert!(left.is_empty(), "{left:?}");
    }
}
