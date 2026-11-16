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

//! Interface strings shared by both editions (`i18n/*.json`).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub const EN_JSON: &str = include_str!("../../../i18n/en.json");
pub const RU_JSON: &str = include_str!("../../../i18n/ru.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Lang {
    #[default]
    En,
    Ru,
}

impl Lang {
    /// Russian for a Russian system or browser locale, English otherwise.
    pub fn from_locale(locale: &str) -> Lang {
        let l = locale.to_ascii_lowercase();
        if l == "ru" || l.starts_with("ru-") || l.starts_with("ru_") {
            Lang::Ru
        } else {
            Lang::En
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
        }
    }
}

fn table(lang: Lang) -> &'static HashMap<String, String> {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static RU: OnceLock<HashMap<String, String>> = OnceLock::new();
    match lang {
        Lang::En => EN.get_or_init(|| serde_json::from_str(EN_JSON).expect("en.json is valid")),
        Lang::Ru => RU.get_or_init(|| serde_json::from_str(RU_JSON).expect("ru.json is valid")),
    }
}

/// Text for a key, or None when the key is unknown.
pub fn get(lang: Lang, key: &str) -> Option<&'static str> {
    table(lang).get(key).map(String::as_str)
}

/// Text for a key; the key itself when it is unknown.
pub fn tr(lang: Lang, key: &str) -> String {
    get(lang, key).map_or_else(|| key.to_string(), str::to_string)
}

/// Text with `{name}` placeholders replaced.
pub fn trf(lang: Lang, key: &str, params: &[(&str, String)]) -> String {
    let mut s = tr(lang, key);
    for (k, v) in params {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_keys_and_lookup() {
        assert_eq!(table(Lang::En).len(), table(Lang::Ru).len());
        assert!(table(Lang::En)
            .keys()
            .all(|k| table(Lang::Ru).contains_key(k)));
        assert_eq!(
            trf(Lang::Ru, "start.dwellers", &[("n", "5".into())]),
            "жителей: 5"
        );
        assert_eq!(Lang::from_locale("ru-RU"), Lang::Ru);
        assert_eq!(Lang::from_locale("uk-UA"), Lang::En);
    }
}
