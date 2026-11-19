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

//! Which game picture shows a room, a theme or a room type.

use crate::catalog::catalog;
use crate::json::Value;
use crate::vault::{int, p, text};

/// Room types that can wear a theme, in the order of the game's theme window.
pub const THEME_ROOMS: [&str; 5] = [
    "LivingQuarters",
    "Cafeteria",
    "NukaCola",
    "WeaponFactory",
    "FakeWasteland",
];

/// Holiday themes the game turns on by date: (theme name in the save, journal id).
pub const HOLIDAYS: [(&str, &str); 3] = [
    ("Xmas", "Cafeteria_Xmas"),
    ("Halloween", "Cafeteria_Halloween"),
    ("ThanksGiving", "Cafeteria_ThanksGiving"),
];

/// Theme set for a room type in `specialTheme.themeByRoomType`, if any.
pub fn applied_theme(root: &Value, room: &str) -> Option<String> {
    root.at(&p(&["specialTheme", "themeByRoomType", room]))
        .and_then(Value::as_text)
}

/// Picture key (kind, id) of the standard look or of a theme for a room type.
pub fn theme_picture(room: &str, theme: Option<&str>) -> (String, String) {
    if room == "FakeWasteland" {
        return ("Exterior".into(), theme.unwrap_or("None").to_string());
    }
    let level = theme
        .and_then(|t| {
            HOLIDAYS
                .iter()
                .position(|(k, _)| *k == t)
                .map(|i| i as i64 + 1)
                .or_else(|| {
                    catalog()
                        .themes
                        .iter()
                        .find(|x| x.room == room && x.theme == t)
                        .map(|x| x.theme_type)
                })
        })
        .map_or(3, |tt| 3 + tt);
    ("Room".into(), format!("{room}:{level}:3"))
}

/// Picture the game shows for a built room (its zoomed-out look).
pub fn room_picture(root: &Value, r: &Value) -> (String, String) {
    let kind = text(r, &["type"]).unwrap_or_default();
    let merge = int(r, &["mergeLevel"]).unwrap_or(1);
    let level = int(r, &["level"]).unwrap_or(1);
    let theme = applied_theme(root, &kind);
    match kind.as_str() {
        "FakeWasteland" => theme_picture(&kind, theme.as_deref()),
        "Entrance" => ("Room".into(), format!("Entrance:{level}:1")),
        "Elevator" => ("Room".into(), "Elevator:1:1".into()),
        _ if merge == 3 && level == 3 && theme.is_some() => theme_picture(&kind, theme.as_deref()),
        _ => ("Room".into(), format!("{kind}:{level}:{merge}")),
    }
}

/// Picture the game uses when a room picture is missing: the ultracite
/// workshop for weapon workshops, the plain level 3 room otherwise.
pub fn room_picture_fallback(id: &str) -> Option<String> {
    let (room, _) = id.split_once(':')?;
    Some(if room == "WeaponFactory" {
        "UltraciteWeaponFactory:3:3".to_string()
    } else {
        format!("{room}:3:3")
    })
}
