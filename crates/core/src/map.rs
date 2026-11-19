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

//! Vault grid layout: room placement, overlap and connectivity rules
//! matching the game's construction grid.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::Serialize;

use crate::catalog::catalog;
use crate::json::Value;
use crate::vault::{arr, int, text, ROOMS};

/// Room types the editor never adds, moves or removes.
pub const FIXED_ROOMS: [&str; 2] = ["Entrance", "FakeWasteland"];

/// Room types that cannot be built by the editor in normal mode.
pub const NOT_BUILDABLE: [&str; 3] = ["Entrance", "FakeWasteland", "UltraciteMining"];

/// Room types the game allows only once.
pub const UNIQUE_ROOMS: [&str; 1] = ["Overseer"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub id: i64,
    pub kind: String,
    pub row: i64,
    pub col: i64,
    pub width: i64,
    pub merge: i64,
    pub level: i64,
}

impl Placed {
    pub fn end(&self) -> i64 {
        self.col + self.width
    }

    pub fn is_elevator(&self) -> bool {
        self.kind == "Elevator"
    }
}

/// Cells a room of this type and merge level covers in its row.
pub fn width_of(kind: &str, merge: i64) -> i64 {
    match kind {
        "Elevator" => 1,
        // The game always loads the vault door as a double room.
        "Entrance" => 6,
        _ => 3 * merge.max(1),
    }
}

pub fn placed(room: &Value) -> Option<Placed> {
    let kind = text(room, &["type"])?;
    let merge = int(room, &["mergeLevel"]).unwrap_or(1).max(1);
    Some(Placed {
        id: int(room, &["deserializeID"])?,
        row: int(room, &["row"])?,
        col: int(room, &["col"])?,
        width: width_of(&kind, merge),
        level: int(room, &["level"]).unwrap_or(1),
        merge,
        kind,
    })
}

pub fn rooms(root: &Value) -> Vec<Placed> {
    arr(root, &ROOMS).iter().filter_map(placed).collect()
}

/// Rocks and ultracite deposits: (row, col), two cells wide.
pub fn obstacles(root: &Value) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    for list in [["vault", "rocks"], ["vault", "ultracite"]] {
        for o in arr(root, &list) {
            if let (Some(r), Some(c)) = (int(o, &["r"]), int(o, &["c"])) {
                out.push((r, c));
            }
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaceError {
    OutOfGrid,
    Overlap,
    Disconnected,
}

pub fn grid_size() -> (i64, i64) {
    let l = &catalog().limits;
    (l.grid_rows, l.grid_cols)
}

/// Checks bounds and overlap for a room span, ignoring room `skip`.
pub fn fits(
    root: &Value,
    row: i64,
    col: i64,
    width: i64,
    skip: Option<i64>,
) -> Result<(), PlaceError> {
    let (rows, cols) = grid_size();
    if row < 0 || row >= rows || col < 0 || col + width > cols {
        return Err(PlaceError::OutOfGrid);
    }
    let end = col + width;
    for r in rooms(root) {
        if Some(r.id) == skip || r.row != row {
            continue;
        }
        if r.col < end && col < r.end() {
            return Err(PlaceError::Overlap);
        }
    }
    for (r, c) in obstacles(root) {
        if r == row && c < end && col < c + 2 {
            return Err(PlaceError::Overlap);
        }
    }
    Ok(())
}

fn adjacent(a: &Placed, b: &Placed) -> bool {
    if a.row == b.row {
        return a.end() == b.col || b.end() == a.col;
    }
    a.is_elevator() && b.is_elevator() && a.col == b.col && (a.row - b.row).abs() == 1
}

/// Rooms reachable from the vault door. Without a door every room counts
/// as reachable, so damaged layouts are not made worse by the check.
pub fn reachable(list: &[Placed]) -> HashSet<i64> {
    let Some(start) = list.iter().find(|r| r.kind == "Entrance") else {
        return list.iter().map(|r| r.id).collect();
    };
    let mut seen = HashSet::from([start.id]);
    let mut queue = VecDeque::from([start.clone()]);
    while let Some(cur) = queue.pop_front() {
        for other in list {
            if !seen.contains(&other.id) && adjacent(&cur, other) {
                seen.insert(other.id);
                queue.push_back(other.clone());
            }
        }
    }
    seen
}

/// True when `after` keeps every room that was reachable in `before`
/// reachable, and `must` (a new or moved room) is reachable too.
pub fn keeps_connected(before: &[Placed], after: &[Placed], must: Option<i64>) -> bool {
    let was = reachable(before);
    let now = reachable(after);
    let present: HashSet<i64> = after.iter().map(|r| r.id).collect();
    if was
        .iter()
        .any(|id| present.contains(id) && !now.contains(id))
    {
        return false;
    }
    must.is_none_or(|id| now.contains(&id))
}

/// Full check for putting a room of `kind` at a spot, as the editor would
/// after the change. `id` is the room being moved or resized, if any.
pub fn check_spot(
    root: &Value,
    kind: &str,
    row: i64,
    col: i64,
    merge: i64,
    id: Option<i64>,
) -> Result<(), PlaceError> {
    let width = width_of(kind, merge);
    fits(root, row, col, width, id)?;
    let before = rooms(root);
    let mut after: Vec<Placed> = before
        .iter()
        .filter(|r| Some(r.id) != id)
        .cloned()
        .collect();
    let new_id = id.unwrap_or(i64::MIN);
    after.push(Placed {
        id: new_id,
        kind: kind.to_string(),
        row,
        col,
        width,
        merge,
        level: 1,
    });
    if keeps_connected(&before, &after, Some(new_id)) {
        Ok(())
    } else {
        Err(PlaceError::Disconnected)
    }
}

/// True when removing room `id` leaves every other reachable room reachable.
pub fn can_remove(root: &Value, id: i64) -> bool {
    let before = rooms(root);
    let after: Vec<Placed> = before.iter().filter(|r| r.id != id).cloned().collect();
    keeps_connected(&before, &after, None)
}

/// Neighbours of the same type and level that the game would merge with
/// this room at its next merge check.
pub fn merge_neighbours(list: &[Placed], room: &Placed) -> Vec<i64> {
    let max = catalog()
        .rooms
        .iter()
        .find(|r| r.kind == room.kind)
        .map_or(3, |r| r.max_merge);
    if room.is_elevator() || FIXED_ROOMS.contains(&room.kind.as_str()) {
        return Vec::new();
    }
    list.iter()
        .filter(|o| {
            o.id != room.id
                && o.kind == room.kind
                && o.row == room.row
                && o.level == room.level
                && (o.end() == room.col || room.end() == o.col)
                && o.merge + room.merge <= max
        })
        .map(|o| o.id)
        .collect()
}

/// Cells of the grid by owner, for drawing and hit tests.
pub fn occupancy(root: &Value) -> HashMap<(i64, i64), i64> {
    let mut out = HashMap::new();
    for r in rooms(root) {
        for c in r.col..r.end() {
            out.insert((r.row, c), r.id);
        }
    }
    out
}

/// Start column for a new room (or a pair of rocks) so that it covers the
/// clicked cell `at`, preferring a spot that passes every check.
pub fn add_col(root: &Value, kind: &str, row: i64, at: i64, merge: i64, rock: bool) -> i64 {
    if rock {
        return [at, at - 1]
            .into_iter()
            .find(|c| fits(root, row, *c, 2, None).is_ok())
            .unwrap_or(at);
    }
    let w = width_of(kind, merge);
    let starts: Vec<i64> = (0..w).map(|off| at - off).collect();
    starts
        .iter()
        .find(|c| check_spot(root, kind, row, **c, merge, None).is_ok())
        .or_else(|| {
            starts
                .iter()
                .find(|c| fits(root, row, **c, w, None).is_ok())
        })
        .copied()
        .unwrap_or(at)
}

/// Sizes the game has for a room type.
pub fn merges(kind: &str) -> Vec<i64> {
    match catalog().room(kind) {
        _ if kind == "Elevator" => vec![1],
        Some(r) if !r.merges().is_empty() => r.merges(),
        _ => (1..=crate::rules::max_merge(kind)).collect(),
    }
}

/// Highest upgrade level of a room type at a size.
pub fn max_level(kind: &str, merge: i64) -> i64 {
    catalog()
        .room(kind)
        .map_or(catalog().limits.room_level_max, |r| {
            r.max_level(merge).max(1)
        })
}

/// Resources a production room makes and keeps until they are collected.
pub fn room_product(kind: &str) -> &'static [&'static str] {
    match kind {
        "Cafeteria" | "Hydroponic" => &["Food"],
        "Geothermal" | "Energy2" => &["Energy"],
        "WaterPlant" | "Water2" => &["Water"],
        "MedBay" => &["StimPack"],
        "ScienceLab" => &["RadAway"],
        "NukaCola" => &["Food", "Water"],
        _ => &[],
    }
}

/// Nearest place for a copy of a room: the closest spot that passes every
/// check (true), else the closest free one (false), else the room's own spot.
pub fn copy_spot(root: &Value, me: &Placed) -> ((i64, i64), bool) {
    let (rows, cols) = grid_size();
    let width = width_of(&me.kind, me.merge);
    let nearest = |ok: &dyn Fn(i64, i64) -> bool| {
        (0..rows)
            .flat_map(|row| (0..cols).map(move |col| (row, col)))
            .filter(|(row, col)| ok(*row, *col))
            .min_by_key(|(row, col)| ((row - me.row).abs(), (col - me.col).abs()))
    };
    if let Some(best) =
        nearest(&|row, col| check_spot(root, &me.kind, row, col, me.merge, None).is_ok())
    {
        return (best, true);
    }
    let loose = nearest(&|row, col| fits(root, row, col, width, None).is_ok());
    (loose.unwrap_or((me.row, me.col)), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(id: i64, kind: &str, row: i64, col: i64, merge: i64) -> Placed {
        Placed {
            id,
            kind: kind.into(),
            row,
            col,
            width: width_of(kind, merge),
            merge,
            level: 1,
        }
    }

    #[test]
    fn connectivity_through_elevators() {
        let list = vec![
            room(1, "Entrance", 0, 3, 2),
            room(2, "Elevator", 0, 9, 1),
            room(3, "Elevator", 1, 9, 1),
            room(4, "LivingQuarters", 1, 10, 1),
            room(5, "Storage", 2, 10, 1),
        ];
        let r = reachable(&list);
        assert!(r.contains(&4));
        assert!(!r.contains(&5), "rooms are not linked vertically");
        let without_lift: Vec<Placed> = list.iter().filter(|x| x.id != 3).cloned().collect();
        assert!(!keeps_connected(&list, &without_lift, None));
    }

    #[test]
    fn merge_candidates() {
        let a = room(1, "LivingQuarters", 1, 10, 1);
        let b = room(2, "LivingQuarters", 1, 13, 2);
        let c = room(3, "LivingQuarters", 1, 19, 1);
        let list = vec![a.clone(), b.clone(), c];
        assert_eq!(merge_neighbours(&list, &a), vec![2]);
        assert_eq!(merge_neighbours(&list, &b), vec![1, 3]);
    }
}
