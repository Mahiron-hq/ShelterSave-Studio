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

//! Pictures from the game file the user picks. The page reads the byte
//! ranges asked for here, so the file never has to fit in memory whole.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::json;
use shelter_art::bundle::{parse_head, Head, Index};
use shelter_art::dweller::{self, Mesh, Parts};
use shelter_art::image::{self, Image};
use shelter_core::catalog::catalog;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Default)]
pub struct Art {
    head: Option<Head>,
    index: Option<Index>,
    images: HashMap<String, Arc<Image>>,
    icons: HashMap<String, Image>,
    meshes: HashMap<String, Mesh>,
    atlases: HashMap<String, Vec<shelter_art::IconRect>>,
    keep: HashSet<String>,
    size: [u32; 2],
}

#[wasm_bindgen]
impl Art {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Art {
        Art {
            atlases: shelter_art::icons_by_atlas().into_iter().collect(),
            keep: dweller::base_textures().into_iter().collect(),
            ..Art::default()
        }
    }

    /// Bytes of the file start to pass to [`Art::head`].
    #[wasm_bindgen(js_name = headSize)]
    pub fn head_size() -> usize {
        shelter_art::bundle::HEAD_SIZE
    }

    /// File name the game uses for this bundle.
    #[wasm_bindgen(js_name = bundleName)]
    pub fn bundle_name() -> String {
        shelter_art::bundle_name().unwrap_or_default().to_string()
    }

    /// Reads the header; returns where the block list is (start, length).
    pub fn head(&mut self, bytes: &[u8]) -> Option<Vec<f64>> {
        let h = parse_head(bytes)?;
        self.head = Some(h);
        Some(vec![h.info_at as f64, h.info_len as f64])
    }

    /// Reads the block list.
    pub fn info(&mut self, bytes: &[u8]) -> bool {
        self.index = self.head.as_ref().and_then(|h| Index::new(h, bytes));
        self.index.is_some()
    }

    /// Texture names to load first: dweller basics and every atlas.
    pub fn plan(&self) -> Vec<String> {
        let mut out: Vec<String> = dweller::base_textures();
        out.extend(self.atlases.keys().cloned());
        let mut seen = HashSet::new();
        out.retain(|n| seen.insert(n.clone()));
        out
    }

    /// Packed file range (start, length) of the wasteland journal phrases.
    #[wasm_bindgen(js_name = textsRange)]
    pub fn texts_range(&self) -> Option<Vec<f64>> {
        let w = catalog().wasteland_texts.as_ref()?;
        let (off, size) = w.span()?;
        let (at, len) = self.index.as_ref()?.range(&w.node, off, size)?;
        Some(vec![at as f64, len as f64])
    }

    /// Reads the phrases from the bytes of their range; false when they do
    /// not match this game version.
    pub fn texts(&self, packed: &[u8]) -> bool {
        let Some(w) = catalog().wasteland_texts.as_ref() else {
            return false;
        };
        let Some((off, size)) = w.span() else {
            return false;
        };
        let Some(raw) = self
            .index
            .as_ref()
            .and_then(|i| i.extract(&w.node, off, size, packed))
        else {
            return false;
        };
        shelter_art::set_texts(off, &raw)
    }

    /// Packed file range (start, length) of a texture.
    pub fn range(&self, name: &str) -> Option<Vec<f64>> {
        let (node, off, size) = image::texture_place(name)?;
        let (at, len) = self.index.as_ref()?.range(&node, off, size)?;
        Some(vec![at as f64, len as f64])
    }

    /// Decodes a texture from the bytes of its range; false when it does not
    /// match this game version.
    pub fn texture(&mut self, name: &str, packed: &[u8]) -> bool {
        let Some((node, off, size)) = image::texture_place(name) else {
            return false;
        };
        let Some(raw) = self
            .index
            .as_ref()
            .and_then(|i| i.extract(&node, off, size, packed))
        else {
            return false;
        };
        let Some(img) = image::texture(name, raw) else {
            return false;
        };
        if let Some(list) = self.atlases.get(name) {
            for (key, rect) in list {
                if let Some(i) = image::cut_icon(&img, *rect, image::max_side(key)) {
                    self.icons.insert(key.clone(), i);
                }
            }
            if !self.keep.contains(name) {
                return true;
            }
        }
        self.images.insert(name.to_string(), Arc::new(img));
        true
    }

    /// Packed ranges of a mesh: vertex start, length, index start, length.
    #[wasm_bindgen(js_name = meshRanges)]
    pub fn mesh_ranges(&self, gender: &str) -> Option<Vec<f64>> {
        let (node, v, i) = dweller::mesh_place(gender)?;
        let index = self.index.as_ref()?;
        let a = index.range(&node, v.0, v.1)?;
        let b = index.range(&node, i.0, i.1)?;
        Some(vec![a.0 as f64, a.1 as f64, b.0 as f64, b.1 as f64])
    }

    pub fn mesh(&mut self, gender: &str, vertex: &[u8], indices: &[u8]) -> bool {
        let Some((node, v, i)) = dweller::mesh_place(gender) else {
            return false;
        };
        let Some(index) = self.index.as_ref() else {
            return false;
        };
        let (Some(vb), Some(ib)) = (
            index.extract(&node, v.0, v.1, vertex),
            index.extract(&node, i.0, i.1, indices),
        ) else {
            return false;
        };
        match Mesh::new(gender, &vb, &ib) {
            Some(m) => {
                self.meshes.insert(gender.to_string(), m);
                true
            }
            None => false,
        }
    }

    /// Straight-alpha RGBA of a picture (`Weapon:id`, `Room:kind:level:size`...).
    pub fn icon(&mut self, key: &str) -> Option<Vec<u8>> {
        let i = self.icons.get(key)?;
        self.size = [i.w, i.h];
        Some(i.rgba.clone())
    }

    /// Width and height of the last picture returned.
    #[wasm_bindgen(js_name = lastSize)]
    pub fn last_size(&self) -> Vec<u32> {
        self.size.to_vec()
    }

    /// Textures still needed to draw a dweller (JSON of the dweller object).
    #[wasm_bindgen(js_name = dwellerMissing)]
    pub fn dweller_missing(&self, dweller_json: &str) -> Vec<String> {
        let Ok(d) = shelter_core::json::parse(dweller_json) else {
            return Vec::new();
        };
        Parts::new(&self.images, &dweller::look_of(&d))
            .err()
            .unwrap_or_default()
    }

    /// The dweller as the game draws it, straight-alpha RGBA.
    pub fn dweller(&mut self, dweller_json: &str) -> Option<Vec<u8>> {
        let d = shelter_core::json::parse(dweller_json).ok()?;
        let look = dweller::look_of(&d);
        let mesh = self.meshes.get(if look.male { "male" } else { "female" })?;
        let parts = Parts::new(&self.images, &look).ok()?;
        let mut pic = dweller::render(&parts, mesh)?;
        for p in pic.rgba.as_chunks_mut::<4>().0 {
            let a = u32::from(p[3]);
            if a > 0 && a < 255 {
                for c in p.iter_mut().take(3) {
                    *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        self.size = [pic.w as u32, pic.h as u32];
        Some(pic.rgba)
    }

    /// Why the preview may differ from the game: an interface string key.
    #[wasm_bindgen(js_name = previewNote)]
    pub fn preview_note(dweller_json: &str) -> Option<String> {
        let d = shelter_core::json::parse(dweller_json).ok()?;
        dweller::preview_note(&d).map(str::to_string)
    }

    /// Key of the look, to cache pictures by.
    #[wasm_bindgen(js_name = lookKey)]
    pub fn look_key(dweller_json: &str) -> String {
        shelter_core::json::parse(dweller_json)
            .map(|d| dweller::look_of(&d).key())
            .unwrap_or_default()
    }

    pub fn stats(&self) -> String {
        json!({ "icons": self.icons.len(), "images": self.images.len(), "meshes": self.meshes.len() }).to_string()
    }
}
