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

//! Setup wizard for Windows: installs, updates and removes ShelterSave-Studio.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
#[allow(dead_code)]
#[path = "../../desktop/src/icons.rs"]
mod icons;
#[cfg(windows)]
#[allow(dead_code)]
#[path = "../../desktop/src/style.rs"]
mod style;
#[cfg(windows)]
mod win;
#[cfg(windows)]
mod wizard;

#[cfg(windows)]
fn main() -> eframe::Result {
    wizard::run()
}

#[cfg(not(windows))]
fn main() {
    let lang = shelter_core::i18n::Lang::from_locale(&sys_locale::get_locale().unwrap_or_default());
    eprintln!("{}", shelter_core::i18n::tr(lang, "inst.windowsOnly"));
    std::process::exit(1);
}
