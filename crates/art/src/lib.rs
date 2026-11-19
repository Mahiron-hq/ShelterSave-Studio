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

//! Game pictures read from the user's own copy of Fallout Shelter.
//! Nothing from the game is shipped; the catalog only says where to look.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod dweller;
pub mod image;

use std::collections::HashMap;

use shelter_core::catalog::catalog;

/// Icon key and its rectangle (x, y, w, h) in the atlas.
pub type IconRect = (String, (u32, u32, u32, u32));

/// Pictures cut from one atlas: (key, rectangle).
pub fn icons_by_atlas() -> Vec<(String, Vec<IconRect>)> {
    let Some(art) = catalog().art.as_ref() else {
        return Vec::new();
    };
    let mut by: HashMap<&String, Vec<IconRect>> = HashMap::new();
    for (key, (tex, x, y, w, h)) in &art.icons {
        by.entry(tex)
            .or_default()
            .push((key.clone(), (*x, *y, *w, *h)));
    }
    let mut out: Vec<_> = by.into_iter().map(|(t, l)| (t.clone(), l)).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Reads the wasteland journal phrases from the game file; false when they
/// do not match the catalog.
pub fn load_texts<S: bundle::Source>(bundle: &bundle::Bundle<S>) -> bool {
    let Some(w) = catalog().wasteland_texts.as_ref() else {
        return false;
    };
    let Some((offset, size)) = w.span() else {
        return false;
    };
    let Some(data) = bundle.read(&w.node, offset, size) else {
        return false;
    };
    set_texts(offset, &data)
}

/// Keeps the phrases cut from `data`, the node bytes starting at `offset`.
pub fn set_texts(offset: u64, data: &[u8]) -> bool {
    let Some(texts) = catalog()
        .wasteland_texts
        .as_ref()
        .and_then(|w| w.read(offset, data))
    else {
        return false;
    };
    shelter_core::catalog::set_game_texts(texts);
    true
}

/// File name of the bundle inside the game's data folder.
pub fn bundle_name() -> Option<&'static str> {
    Some(catalog().art.as_ref()?.bundle.as_str())
}
