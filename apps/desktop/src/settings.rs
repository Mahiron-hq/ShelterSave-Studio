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

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use shelter_core::i18n::Lang;
use shelter_core::rules::Mode;

/// Settings file format version. Newer files are read field by field;
/// unknown fields are ignored and missing ones get defaults.
pub const SETTINGS_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LangPref {
    #[default]
    System,
    En,
    Ru,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

/// Where the dweller card scrolls when another dweller is opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DwellerScroll {
    /// Same place for every dweller.
    #[default]
    Shared,
    Top,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub accepted_terms: bool,
    pub language: LangPref,
    pub theme: ThemePref,
    pub russian_item_names: bool,
    pub mode: Mode,
    pub confirm_mass: bool,
    pub dweller_scroll: DwellerScroll,
    pub scale: f32,
    pub backups: bool,
    pub backup_dir: Option<PathBuf>,
    pub backup_keep: u32,
    pub save_dirs: Vec<PathBuf>,
    pub recent: Vec<PathBuf>,
    /// Game folder for pictures; None means look for it automatically.
    pub game_dir: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: SETTINGS_VERSION,
            accepted_terms: false,
            language: LangPref::System,
            theme: ThemePref::System,
            russian_item_names: false,
            mode: Mode::Normal,
            confirm_mass: true,
            dweller_scroll: DwellerScroll::Shared,
            scale: 1.0,
            backups: true,
            backup_dir: None,
            backup_keep: 10,
            save_dirs: Vec::new(),
            recent: Vec::new(),
            game_dir: None,
        }
    }
}

pub const SCALES: [f32; 6] = [0.85, 1.0, 1.15, 1.3, 1.5, 1.75];

impl Settings {
    pub fn dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ShelterSave-Studio"))
    }

    pub fn path() -> Option<PathBuf> {
        Settings::dir().map(|d| d.join("settings.json"))
    }

    pub fn default_backup_dir() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("ShelterSave-Studio").join("backups"))
    }

    pub fn backup_dir(&self) -> Option<PathBuf> {
        self.backup_dir
            .clone()
            .or_else(Settings::default_backup_dir)
    }

    pub fn load() -> Settings {
        let Some(path) = Settings::path() else {
            return Settings::default();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Settings::default();
        };
        Settings::from_json(&text)
    }

    /// Reads any version; a broken file falls back to defaults.
    pub fn from_json(text: &str) -> Settings {
        let mut s: Settings = serde_json::from_str(text).unwrap_or_default();
        if !SCALES.contains(&s.scale) {
            s.scale = 1.0;
        }
        s.backup_keep = s.backup_keep.clamp(1, 100);
        s.recent.truncate(10);
        if s.version < SETTINGS_VERSION {
            s.version = SETTINGS_VERSION;
        }
        s
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Settings::path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut copy = self.clone();
        copy.version = copy.version.max(SETTINGS_VERSION);
        let text = serde_json::to_string_pretty(&copy).map_err(std::io::Error::other)?;
        crate::files::write_atomic(&path, text.as_bytes())
    }

    pub fn lang(&self) -> Lang {
        match self.language {
            LangPref::En => Lang::En,
            LangPref::Ru => Lang::Ru,
            LangPref::System => {
                sys_locale::get_locale().map_or(Lang::En, |l| Lang::from_locale(&l))
            }
        }
    }

    pub fn push_recent(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerant_reading() {
        let s = Settings::from_json(
            r#"{"version":7,"scale":3.0,"unknownField":1,"russianItemNames":true}"#,
        );
        assert!(s.russian_item_names);
        assert_eq!(s.scale, 1.0);
        assert_eq!(s.version, 7);
        assert_eq!(Settings::from_json("garbage"), Settings::default());
    }
}
