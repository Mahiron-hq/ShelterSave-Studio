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

//! Game pictures read from the player's installed copy of the game.
//! Nothing from the game is shipped with the editor: the catalog only says
//! where the pictures are, and checksums tell when the game has changed.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;
use shelter_art::bundle::{Bundle, Source};
use shelter_art::dweller::{self, Mesh, Parts};
use shelter_art::image::{self, Image};
use shelter_core::catalog::{catalog, Art};

pub use shelter_art::dweller::{face_for, outfit_art, outfit_color, rgb, Look};

/// The bundle file on disk.
pub struct FileSource(PathBuf);

impl Source for FileSource {
    fn read_at(&self, at: u64, len: usize) -> Option<Vec<u8>> {
        let mut f = File::open(&self.0).ok()?;
        f.seek(SeekFrom::Start(at)).ok()?;
        let mut buf = vec![0u8; len];
        let n = f.read(&mut buf).ok()?;
        buf.truncate(n);
        Some(buf)
    }
}

type GameBundle = Bundle<FileSource>;

/// Folders where the game is usually installed.
pub fn find_game_dirs() -> Vec<PathBuf> {
    let mut steam_roots: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(out) = std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
            .creation_flags(0x0800_0000)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = text.lines().find(|l| l.contains("SteamPath")) {
                if let Some(i) = line.find("REG_SZ") {
                    steam_roots.push(PathBuf::from(line[i + 6..].trim().replace('/', "\\")));
                }
            }
        }
        for p in [r"C:\Program Files (x86)\Steam", r"C:\Program Files\Steam"] {
            steam_roots.push(PathBuf::from(p));
        }
    }
    if let Some(home) = dirs::home_dir() {
        steam_roots.push(home.join(".steam").join("steam"));
        steam_roots.push(home.join(".local").join("share").join("Steam"));
        steam_roots.push(
            home.join("Library")
                .join("Application Support")
                .join("Steam"),
        );
    }
    let mut libs = Vec::new();
    for root in &steam_roots {
        libs.push(root.clone());
        if let Ok(text) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf"))
        {
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("\"path\"") {
                    let v = rest.trim().trim_matches('"').replace("\\\\", "\\");
                    libs.push(PathBuf::from(v));
                }
            }
        }
    }
    let mut out: Vec<PathBuf> = libs
        .into_iter()
        .map(|l| l.join("steamapps").join("common").join("Fallout Shelter"))
        .collect();
    out.push(PathBuf::from(
        r"C:\Program Files (x86)\Bethesda.net Launcher\games\Fallout Shelter",
    ));
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.to_string_lossy().to_lowercase()));
    out
}

pub fn bundle_path(dir: &Path, art: &Art) -> PathBuf {
    dir.join("FalloutShelter_Data").join(&art.bundle)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Still looking for the game.
    Loading,
    /// No catalog data, or the game was not found.
    NoGame,
    /// The installed game differs from the one the catalog was made for.
    Mismatch,
    Ready,
}

/// Rendered dwellers kept as textures.
const RENDER_CACHE: usize = 48;

#[derive(Default)]
struct Shared {
    probed: bool,
    found: bool,
    dir: Option<PathBuf>,
    mismatch: bool,
    /// Textures the dweller preview is drawn from, by name.
    images: HashMap<String, Arc<Image>>,
    /// Item pictures cut from the atlases, top row first.
    icons: HashMap<String, Arc<Image>>,
    meshes: HashMap<String, Arc<Mesh>>,
    rendered: HashMap<String, egui::TextureHandle>,
    rendering: bool,
    bundle: Option<Arc<GameBundle>>,
    /// Textures read on demand (outfits), so each is asked for once.
    requested: std::collections::HashSet<String>,
    /// Wasteland journal phrases were read from the game.
    texts: bool,
}

/// Game pictures, loaded in the background as soon as the editor starts.
#[derive(Default)]
pub struct Store {
    started: Option<Option<String>>,
    shared: Arc<Mutex<Shared>>,
    icons: HashMap<String, egui::TextureHandle>,
    /// Last preview shown for each dweller, kept while a new one is drawn.
    last: HashMap<i64, egui::TextureHandle>,
}

impl Store {
    /// Starts loading once per game folder setting.
    pub fn ensure(&mut self, ctx: &egui::Context, custom: Option<&str>) {
        let custom = custom.filter(|c| !c.is_empty()).map(str::to_string);
        if self.started.as_ref() == Some(&custom) {
            return;
        }
        self.started = Some(custom.clone());
        let shared = self.shared.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || load_all(&shared, &ctx, custom));
    }

    /// Forgets everything, so the folder is looked for again.
    pub fn reset(&mut self) {
        *self = Store::default();
    }

    /// Game folder in use, once found.
    pub fn game_dir(&self) -> Option<PathBuf> {
        self.shared.lock().ok()?.dir.clone()
    }

    /// Whether the wasteland journal phrases were read from the game.
    pub fn texts(&self) -> bool {
        self.shared.lock().is_ok_and(|s| s.texts)
    }

    pub fn status(&self) -> Status {
        let Ok(s) = self.shared.lock() else {
            return Status::NoGame;
        };
        match (s.probed, s.found, s.mismatch) {
            (false, _, _) => Status::Loading,
            (true, false, _) => Status::NoGame,
            (true, true, true) => Status::Mismatch,
            (true, true, false) => Status::Ready,
        }
    }

    /// Item picture; None while loading or when it is not available.
    pub fn icon(
        &mut self,
        ctx: &egui::Context,
        kind: &str,
        id: &str,
    ) -> Option<egui::TextureHandle> {
        let key = format!("{kind}:{id}");
        if let Some(t) = self.icons.get(&key) {
            return Some(t.clone());
        }
        let img = self.shared.lock().ok()?.icons.get(&key).cloned()?;
        let color =
            egui::ColorImage::from_rgba_unmultiplied([img.w as usize, img.h as usize], &img.rgba);
        let handle = ctx.load_texture(format!("icon:{key}"), color, egui::TextureOptions::LINEAR);
        self.icons.insert(key, handle.clone());
        Some(handle)
    }

    /// Dweller in the outfit they wear. While a new look is being drawn the
    /// previous picture of the same dweller (`slot`) is returned.
    pub fn dweller(
        &mut self,
        ctx: &egui::Context,
        slot: i64,
        look: &Look,
    ) -> Option<egui::TextureHandle> {
        let key = look.key();
        let mut s = self.shared.lock().ok()?;
        if let Some(t) = s.rendered.get(&key) {
            let t = t.clone();
            self.last.insert(slot, t.clone());
            return Some(t);
        }
        if !s.rendering {
            let gender = if look.male { "male" } else { "female" };
            let mesh = s.meshes.get(gender).cloned();
            match (Parts::new(&s.images, look), mesh) {
                (Ok(parts), Some(mesh)) => {
                    s.rendering = true;
                    let shared = self.shared.clone();
                    let ctx = ctx.clone();
                    std::thread::spawn(move || run(&shared, &ctx, key, &parts, &mesh));
                }
                (Err(missing), _) => {
                    let fresh: Vec<String> = missing
                        .into_iter()
                        .filter(|n| s.requested.insert(n.clone()))
                        .collect();
                    if let (Some(bundle), false) = (s.bundle.clone(), fresh.is_empty()) {
                        let shared = self.shared.clone();
                        let ctx = ctx.clone();
                        std::thread::spawn(move || load_textures(&shared, &ctx, &bundle, &fresh));
                    }
                }
                (Ok(_), None) => {}
            }
        }
        drop(s);
        self.last.get(&slot).cloned()
    }
}

fn run(shared: &Mutex<Shared>, ctx: &egui::Context, key: String, parts: &Parts, mesh: &Mesh) {
    // A bad picture must not leave the preview stuck in "drawing".
    let pic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dweller::render(parts, mesh)
    }))
    .ok()
    .flatten();
    let handle = pic.map(|p| {
        let c = egui::ColorImage::from_rgba_premultiplied([p.w, p.h], &p.rgba);
        ctx.load_texture(format!("dweller:{key}"), c, egui::TextureOptions::LINEAR)
    });
    if let Ok(mut s) = shared.lock() {
        if s.rendered.len() >= RENDER_CACHE {
            s.rendered.clear();
        }
        if let Some(h) = handle {
            s.rendered.insert(key, h);
        }
        s.rendering = false;
    }
    ctx.request_repaint();
}

fn decode(bundle: &GameBundle, name: &str) -> Option<Image> {
    let (node, offset, size) = image::texture_place(name)?;
    image::texture(name, bundle.read(&node, offset, size)?)
}

fn load_textures(
    shared: &Mutex<Shared>,
    ctx: &egui::Context,
    bundle: &GameBundle,
    names: &[String],
) {
    for name in names {
        let img = decode(bundle, name);
        if let Ok(mut s) = shared.lock() {
            match img {
                Some(i) => {
                    s.images.insert(name.clone(), Arc::new(i));
                }
                None => s.mismatch = true,
            }
        }
    }
    ctx.request_repaint();
}

fn load_all(shared: &Mutex<Shared>, ctx: &egui::Context, custom: Option<String>) {
    let finish_probe = |dir: Option<PathBuf>| {
        if let Ok(mut s) = shared.lock() {
            s.probed = true;
            s.found = dir.is_some();
            s.dir = dir;
        }
        ctx.request_repaint();
    };
    let Some(art) = catalog().art.as_ref() else {
        finish_probe(None);
        return;
    };
    let candidates = match custom {
        Some(c) => vec![PathBuf::from(c)],
        None => find_game_dirs(),
    };
    let Some((dir, bundle)) = candidates.into_iter().find_map(|d| {
        let p = bundle_path(&d, art);
        let b = p.is_file().then(|| Bundle::open(FileSource(p))).flatten()?;
        Some((d, b))
    }) else {
        finish_probe(None);
        return;
    };
    finish_probe(Some(dir));
    let bundle = Arc::new(bundle);
    if let Ok(mut s) = shared.lock() {
        s.bundle = Some(bundle.clone());
    }
    let mismatch = || {
        if let Ok(mut s) = shared.lock() {
            s.mismatch = true;
        }
    };
    for gender in art.dweller.keys() {
        let mesh = dweller::mesh_place(gender).and_then(|(node, v, i)| {
            let vb = bundle.read(&node, v.0, v.1)?;
            let ib = bundle.read(&node, i.0, i.1)?;
            Mesh::new(gender, &vb, &ib)
        });
        match mesh {
            Some(m) => {
                if let Ok(mut s) = shared.lock() {
                    s.meshes.insert(gender.clone(), Arc::new(m));
                }
            }
            None => mismatch(),
        }
    }
    let texts = shelter_art::load_texts(&bundle);
    if let Ok(mut s) = shared.lock() {
        s.texts = texts;
    }
    ctx.request_repaint();
    for name in dweller::base_textures() {
        match decode(&bundle, &name) {
            Some(img) => {
                if let Ok(mut s) = shared.lock() {
                    s.images.insert(name, Arc::new(img));
                }
            }
            None => mismatch(),
        }
        ctx.request_repaint();
    }
    for (tex, list) in shelter_art::icons_by_atlas() {
        let Some(atlas) = decode(&bundle, &tex) else {
            mismatch();
            continue;
        };
        let cut: Vec<(String, Arc<Image>)> = list
            .into_iter()
            .filter_map(|(key, rect)| {
                let max = image::max_side(&key);
                image::cut_icon(&atlas, rect, max).map(|i| (key, Arc::new(i)))
            })
            .collect();
        if let Ok(mut s) = shared.lock() {
            s.icons.extend(cut);
        }
        ctx.request_repaint();
    }
}
