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

//! Names and descriptions of save fields (`i18n/fields.json`).
//!
//! Entries are keyed by `parent.key` or `key`; `parent.*` gives the
//! description for children without their own, and `@name` entries are
//! templates for keys that carry a number or a room type.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::catalog::catalog;
use crate::i18n::{self, Lang};
use crate::json::Seg;

const FIELDS: &str = include_str!("../../../i18n/fields.json");

type Table = HashMap<String, [String; 4]>;

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(FIELDS).expect("fields.json is valid"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub label: String,
    pub help: String,
    /// False when the field is not in the dictionary.
    pub known: bool,
}

fn pick(e: &[String; 4], lang: Lang) -> (&str, &str) {
    match lang {
        Lang::Ru => (&e[2], &e[3]),
        Lang::En => (&e[0], &e[1]),
    }
}

fn entry(name: &str) -> Option<&'static [String; 4]> {
    table().get(name)
}

/// Last object key of a path and the object key above it.
pub fn key_and_parent(path: &[Seg]) -> (Option<&str>, Option<&str>) {
    let mut keys = path.iter().rev().filter_map(|s| match s {
        Seg::Key(k) => Some(k.as_str()),
        Seg::Index(_) => None,
    });
    let key = keys.next();
    (key, keys.next())
}

pub fn describe(lang: Lang, path: &[Seg]) -> Field {
    if let Some(Seg::Index(i)) = path.last() {
        let label = entry("@item").map_or_else(
            || format!("#{}", i + 1),
            |e| pick(e, lang).0.replace("{n}", &(i + 1).to_string()),
        );
        return Field {
            label,
            help: String::new(),
            known: true,
        };
    }
    let (key, parent) = key_and_parent(path);
    describe_key(lang, parent, key.unwrap_or_default())
}

fn numbered(key: &str, prefix: &str) -> Option<String> {
    let n = key.strip_prefix(prefix)?;
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then(|| n.to_string())
}

fn template(lang: Lang, key: &str, parent: Option<&str>) -> Option<(String, String)> {
    if parent == Some("happinessManager") {
        for (prefix, name) in [("dweller", "@dweller"), ("room", "@room")] {
            if let (Some(n), Some(e)) = (numbered(key, prefix), entry(name)) {
                let (l, h) = pick(e, lang);
                return Some((l.replace("{n}", &n), h.to_string()));
            }
        }
    }
    if parent == Some("eventsThemes") {
        let (room, event) = key.split_once('_')?;
        let event_name = entry(&format!("@{event}")).map_or(event, |e| pick(e, lang).0);
        let room_name = catalog()
            .rooms
            .iter()
            .find(|r| r.kind == room)
            .and_then(|r| r.name.first())
            .map_or(room.to_string(), |n| title_case(n.get(lang == Lang::Ru)));
        let e = entry("@eventTheme")?;
        let (l, h) = pick(e, lang);
        return Some((
            l.replace("{room}", &room_name)
                .replace("{event}", event_name),
            h.to_string(),
        ));
    }
    None
}

fn title_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut c = lower.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// Name and description of `key` found under the object key `parent`.
pub fn describe_key(lang: Lang, parent: Option<&str>, key: &str) -> Field {
    let parent_help = || {
        parent
            .and_then(|p| entry(&format!("{p}.*")))
            .map(|e| pick(e, lang).1.to_string())
            .filter(|h| !h.is_empty())
    };
    if let Some((label, help)) = template(lang, key, parent) {
        return Field {
            label,
            help,
            known: true,
        };
    }
    let found = parent
        .and_then(|p| entry(&format!("{p}.{key}")))
        .or_else(|| entry(key));
    match found {
        Some(e) => {
            let (l, h) = pick(e, lang);
            let help = if h.is_empty() {
                parent_help().unwrap_or_default()
            } else {
                h.to_string()
            };
            Field {
                label: l.to_string(),
                help,
                known: true,
            }
        }
        None => Field {
            label: humanize(key),
            help: parent_help().unwrap_or_else(|| i18n::tr(lang, "c.fieldUnknown")),
            known: false,
        },
    }
}

/// `camelCase` or `snake_case` key as words with a capital first letter.
pub fn humanize(key: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in key.trim_start_matches("m_").chars() {
        if ch == '_' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if ch.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = ch.is_lowercase() || ch.is_ascii_digit();
        out.extend(ch.to_lowercase());
    }
    title_case(out.trim())
}

const INT_FLAGS: [&str; 14] = [
    "arrivalTimerClaimTutorialCompleted",
    "arrivalTimerTutorialCompleted",
    "builtUltraciteMine",
    "canShowArrivalTimerClaimTutorial",
    "canShowSeasonButton",
    "dwellerSpawnedDuringScorchedAttack",
    "questTutorialCompleted",
    "scorchedAttackWarningShown",
    "scorchedTimerTutorialCompleted",
    "seasonalQuestTutorialCompleted",
    "seasonTutorialCompleted",
    "skippedTutorial",
    "spinToWinTutorialCompleted",
    "ultraciteWeaponCraftingTutorialCompleted",
];

/// Yes/no values the game stores as the numbers 0 and 1.
pub fn is_int_flag(key: &str) -> bool {
    key.starts_with("hasSent") || INT_FLAGS.contains(&key)
}

/// Every dictionary entry, for checks.
pub fn entries() -> impl Iterator<Item = (&'static String, &'static [String; 4])> {
    table().iter()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Seg;

    fn path(parts: &[&str]) -> Vec<Seg> {
        parts.iter().map(|s| Seg::Key((*s).to_string())).collect()
    }

    #[test]
    fn dictionary_is_complete_in_both_languages() {
        for (k, e) in entries() {
            if k.ends_with(".*") {
                assert!(!e[1].is_empty() && !e[3].is_empty(), "{k}");
                continue;
            }
            assert!(!e[0].is_empty() && !e[2].is_empty(), "{k}");
            assert_eq!(e[1].is_empty(), e[3].is_empty(), "{k}");
        }
    }

    #[test]
    fn context_and_templates() {
        let level = describe(Lang::En, &path(&["vault", "rooms", "level"]));
        assert_eq!(level.label, "Upgrade level");
        let hm = describe(Lang::Ru, &path(&["happinessManager", "room132"]));
        assert_eq!(hm.label, "Комната №132");
        let stat = describe(Lang::En, &path(&["StatsWindow", "vaultData", "soldJunk"]));
        assert!(stat.known && !stat.help.is_empty());
        let theme = describe(
            Lang::En,
            &path(&["specialTheme", "eventsThemes", "Cafeteria_Xmas"]),
        );
        assert!(theme.label.contains("Christmas"), "{}", theme.label);
        let unknown = describe(Lang::En, &path(&["fooBarBaz"]));
        assert!(!unknown.known);
        assert_eq!(unknown.label, "Foo bar baz");
    }
}
