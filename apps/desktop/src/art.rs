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
use shelter_core::catalog::{catalog, Art, ArtDweller, ArtHelmet, ArtOutfit, ArtPiece};

/// Unity bundle with LZ4 blocks (UnityFS).
pub struct Bundle {
    path: PathBuf,
    blocks: Vec<Block>,
    nodes: HashMap<String, (u64, u64)>,
}

struct Block {
    raw_size: u32,
    packed_size: u32,
    flags: u16,
    packed_at: u64,
    raw_at: u64,
}

fn be_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be_u64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

fn cstr(b: &[u8], at: &mut usize) -> Option<String> {
    let end = b.get(*at..)?.iter().position(|c| *c == 0)? + *at;
    let s = String::from_utf8_lossy(&b[*at..end]).to_string();
    *at = end + 1;
    Some(s)
}

/// LZ4 block format (also used by LZ4HC).
pub fn lz4_decompress(src: &[u8], out_len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(out_len);
    let mut i = 0;
    while i < src.len() {
        let token = src[i];
        i += 1;
        let mut lit = usize::from(token >> 4);
        if lit == 15 {
            loop {
                let b = *src.get(i)?;
                i += 1;
                lit += usize::from(b);
                if b != 255 {
                    break;
                }
            }
        }
        out.extend_from_slice(src.get(i..i + lit)?);
        i += lit;
        if i >= src.len() {
            break;
        }
        let off = usize::from(u16::from_le_bytes([*src.get(i)?, *src.get(i + 1)?]));
        i += 2;
        let mut len = usize::from(token & 15);
        if len == 15 {
            loop {
                let b = *src.get(i)?;
                i += 1;
                len += usize::from(b);
                if b != 255 {
                    break;
                }
            }
        }
        len += 4;
        if off == 0 || off > out.len() {
            return None;
        }
        let start = out.len() - off;
        for k in 0..len {
            let c = out[start + k];
            out.push(c);
        }
    }
    (out.len() == out_len).then_some(out)
}

pub fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *e = c;
        }
        t
    });
    let mut c = !0u32;
    for b in data {
        c = t[((c ^ u32::from(*b)) & 0xff) as usize] ^ (c >> 8);
    }
    !c
}

impl Bundle {
    pub fn open(path: &Path) -> Option<Bundle> {
        let mut f = File::open(path).ok()?;
        let mut head = vec![0u8; 512];
        let n = f.read(&mut head).ok()?;
        head.truncate(n);
        let mut at = 0;
        if cstr(&head, &mut at)? != "UnityFS" {
            return None;
        }
        let version = be_u32(&head, at)?;
        at += 4;
        cstr(&head, &mut at)?;
        cstr(&head, &mut at)?;
        at += 8;
        let packed = be_u32(&head, at)? as usize;
        let raw = be_u32(&head, at + 4)? as usize;
        let flags = be_u32(&head, at + 8)?;
        at += 12;
        if version >= 7 {
            at = at.div_ceil(16) * 16;
        }
        if flags & 0x80 != 0 {
            return None;
        }
        let mut info = vec![0u8; packed];
        f.seek(SeekFrom::Start(at as u64)).ok()?;
        f.read_exact(&mut info).ok()?;
        let info = match flags & 0x3f {
            0 => info,
            2 | 3 => lz4_decompress(&info, raw)?,
            _ => return None,
        };
        let mut data_start = (at + packed) as u64;
        if flags & 0x200 != 0 {
            data_start = data_start.div_ceil(16) * 16;
        }
        let mut p = 16;
        let count = be_u32(&info, p)? as usize;
        p += 4;
        let mut blocks = Vec::with_capacity(count);
        let (mut packed_at, mut raw_at) = (data_start, 0u64);
        for _ in 0..count {
            let raw_size = be_u32(&info, p)?;
            let packed_size = be_u32(&info, p + 4)?;
            let fl = u16::from_be_bytes(info.get(p + 8..p + 10)?.try_into().ok()?);
            p += 10;
            blocks.push(Block {
                raw_size,
                packed_size,
                flags: fl,
                packed_at,
                raw_at,
            });
            packed_at += u64::from(packed_size);
            raw_at += u64::from(raw_size);
        }
        let n_nodes = be_u32(&info, p)? as usize;
        p += 4;
        let mut nodes = HashMap::new();
        for _ in 0..n_nodes {
            let off = be_u64(&info, p)?;
            let size = be_u64(&info, p + 8)?;
            p += 20;
            let name = cstr(&info, &mut p)?;
            nodes.insert(name, (off, size));
        }
        Some(Bundle {
            path: path.to_path_buf(),
            blocks,
            nodes,
        })
    }

    /// Bytes `offset..offset+size` of a node, decompressing only the blocks they touch.
    pub fn read(&self, node: &str, offset: u64, size: u64) -> Option<Vec<u8>> {
        let (node_at, node_size) = *self.nodes.get(node)?;
        if offset + size > node_size {
            return None;
        }
        let from = node_at + offset;
        let to = from + size;
        let first = self
            .blocks
            .partition_point(|b| b.raw_at + u64::from(b.raw_size) <= from);
        let mut f = File::open(&self.path).ok()?;
        let mut out = Vec::with_capacity(size as usize);
        let mut buf = Vec::new();
        for b in self.blocks.get(first..)? {
            if b.raw_at >= to {
                break;
            }
            buf.resize(b.packed_size as usize, 0);
            f.seek(SeekFrom::Start(b.packed_at)).ok()?;
            f.read_exact(&mut buf).ok()?;
            let raw = match b.flags & 0x3f {
                0 => buf.clone(),
                2 | 3 => lz4_decompress(&buf, b.raw_size as usize)?,
                _ => return None,
            };
            let s = from.saturating_sub(b.raw_at) as usize;
            let e = ((to - b.raw_at) as usize).min(raw.len());
            out.extend_from_slice(raw.get(s..e)?);
        }
        (out.len() as u64 == size).then_some(out)
    }
}

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

/// Picture decoded to RGBA rows, bottom row first like in the game files.
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    fn px(&self, x: i64, y: i64) -> [f32; 4] {
        let x = x.clamp(0, i64::from(self.w) - 1) as usize;
        let y = y.clamp(0, i64::from(self.h) - 1) as usize;
        let i = (y * self.w as usize + x) * 4;
        let p = &self.rgba[i..i + 4];
        [
            f32::from(p[0]) / 255.0,
            f32::from(p[1]) / 255.0,
            f32::from(p[2]) / 255.0,
            f32::from(p[3]) / 255.0,
        ]
    }

    /// Bilinear sample at texture coordinates (0,0 = bottom left).
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let x = u * self.w as f32 - 0.5;
        let y = v * self.h as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let a = self.px(x0, y0);
        let b = self.px(x0 + 1, y0);
        let c = self.px(x0, y0 + 1);
        let d = self.px(x0 + 1, y0 + 1);
        let mut o = [0.0; 4];
        for k in 0..4 {
            let top = a[k] + (b[k] - a[k]) * fx;
            let bot = c[k] + (d[k] - c[k]) * fx;
            o[k] = top + (bot - top) * fy;
        }
        o
    }
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

/// Biggest side of a cut item picture; the hover card shows them at 120 points.
const ICON_MAX: u32 = 160;
/// Room pictures and vault exteriors are kept at full size.
const ROOM_MAX: u32 = 512;
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
    bundle: Option<Arc<Bundle>>,
    /// Textures read on demand (outfits), so each is asked for once.
    requested: std::collections::HashSet<String>,
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

    /// Dweller in the basic jumpsuit. While a new look is being drawn the
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
            match Job::new(&s, look) {
                Ok(job) => {
                    s.rendering = true;
                    let shared = self.shared.clone();
                    let ctx = ctx.clone();
                    std::thread::spawn(move || job.run(&shared, &ctx, key));
                }
                Err(missing) => {
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
            }
        }
        drop(s);
        self.last.get(&slot).cloned()
    }
}

/// Everything one preview needs, taken out of the store.
struct Job {
    kit: &'static ArtDweller,
    mesh: Arc<Mesh>,
    images: HashMap<String, Arc<Image>>,
    look: Look,
    face: &'static ArtPiece,
    hair: Option<&'static ArtPiece>,
    mask: Option<&'static ArtPiece>,
    outfit: Option<&'static ArtOutfit>,
    helmet: Option<&'static ArtHelmet>,
}

impl Job {
    /// The job, or the textures still to be loaded (empty when it cannot be drawn).
    fn new(s: &Shared, look: &Look) -> Result<Job, Vec<String>> {
        let gender = if look.male { "male" } else { "female" };
        let kit = catalog()
            .art
            .as_ref()
            .and_then(|a| a.dweller.get(gender))
            .ok_or_else(Vec::new)?;
        let mesh = s.meshes.get(gender).ok_or_else(Vec::new)?.clone();
        let face = kit
            .faces
            .get(&look.face)
            .or_else(|| kit.faces.get("Neutral"))
            .ok_or_else(Vec::new)?;
        let outfit = look.outfit.as_ref().and_then(|o| kit.outfits.get(o));
        let helmet = worn_helmet(kit, outfit, look);
        let hair = look
            .hair
            .as_ref()
            .and_then(|h| kit.hairs.get(h))
            .filter(|_| !helmet.is_some_and(|h| h.exclusive));
        let mask = look
            .face_mask
            .as_ref()
            .and_then(|m| kit.face_masks.get(m))
            .filter(|_| !helmet.is_some_and(|h| h.no_mask));
        let body = match (outfit.is_some_and(|o| o.skirt), &kit.skirt_body) {
            (true, Some(b)) => b,
            _ => &kit.body,
        };
        let mut needed: Vec<&String> = vec![&body.tex, &kit.hands.tex, &face.tex];
        needed.extend(hair.map(|p| &p.tex));
        needed.extend(mask.map(|p| &p.tex));
        match outfit {
            Some(o) => {
                needed.push(&o.tex);
                match &o.coloring_mask {
                    Some(m) => needed.push(&m.tex),
                    None if !o.colors.is_empty() => needed.extend(kit.burn_mask.as_ref()),
                    None => {}
                }
                needed.extend(o.glove.as_ref().map(|p| &p.tex));
            }
            None => needed.push(&kit.outfit.tex),
        }
        if let Some(h) = helmet {
            needed.push(&h.tex);
            needed.extend(h.mask.as_ref().map(|p| &p.tex));
        }
        let mut images = HashMap::new();
        let mut missing = Vec::new();
        for n in needed {
            match s.images.get(n) {
                Some(img) => {
                    images.insert(n.clone(), img.clone());
                }
                None => missing.push(n.clone()),
            }
        }
        if !missing.is_empty() {
            return Err(missing);
        }
        Ok(Job {
            kit,
            mesh,
            images,
            look: look.clone(),
            face,
            hair,
            mask,
            outfit,
            helmet,
        })
    }

    fn run(self, shared: &Mutex<Shared>, ctx: &egui::Context, key: String) {
        // A bad picture must not leave the preview stuck in "drawing".
        let color =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| render_dweller(&self)))
                .ok()
                .flatten();
        let handle = color
            .map(|c| ctx.load_texture(format!("dweller:{key}"), c, egui::TextureOptions::LINEAR));
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
}

fn decode(bundle: &Bundle, name: &str) -> Option<Image> {
    let info = catalog().art.as_ref()?.textures.get(name)?;
    let raw = bundle
        .read(&info.node, info.offset, info.size)
        .filter(|raw| crc32(raw) == info.crc)?;
    let (w, h) = (info.width, info.height);
    let rgba = match info.format.as_str() {
        "RGBA32" => raw,
        "ARGB32" => {
            let mut v = raw;
            for p in v.chunks_exact_mut(4) {
                p.rotate_left(1);
            }
            v
        }
        "RGB24" => raw
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        "DXT1" => decode_bc(&raw, w, h, false)?,
        "DXT5" => decode_bc(&raw, w, h, true)?,
        _ => return None,
    };
    (rgba.len() == (w * h * 4) as usize).then_some(Image { w, h, rgba })
}

fn rgb565(c: u16) -> [u8; 3] {
    let r = (c >> 11) & 31;
    let g = (c >> 5) & 63;
    let b = c & 31;
    [
        ((r << 3) | (r >> 2)) as u8,
        ((g << 2) | (g >> 4)) as u8,
        ((b << 3) | (b >> 2)) as u8,
    ]
}

/// DXT1 (BC1) or DXT5 (BC3) blocks to RGBA rows, in the same row order as the data.
pub fn decode_bc(data: &[u8], w: u32, h: u32, alpha: bool) -> Option<Vec<u8>> {
    let (w, h) = (w as usize, h as usize);
    let bw = w.div_ceil(4);
    let block = if alpha { 16 } else { 8 };
    if data.len() < bw * h.div_ceil(4) * block {
        return None;
    }
    let mut out = vec![0u8; w * h * 4];
    for (i, b) in data
        .chunks_exact(block)
        .take(bw * h.div_ceil(4))
        .enumerate()
    {
        let (bx, by) = (i % bw * 4, i / bw * 4);
        let mut alphas = [255u8; 16];
        let c = if alpha {
            let (a0, a1) = (b[0], b[1]);
            let mut table = [a0, a1, 0, 0, 0, 0, 0, 255];
            if a0 > a1 {
                for k in 1..7u16 {
                    table[k as usize + 1] =
                        (((7 - k) * u16::from(a0) + k * u16::from(a1)) / 7) as u8;
                }
            } else {
                for k in 1..5u16 {
                    table[k as usize + 1] =
                        (((5 - k) * u16::from(a0) + k * u16::from(a1)) / 5) as u8;
                }
            }
            let bits = u64::from_le_bytes([b[2], b[3], b[4], b[5], b[6], b[7], 0, 0]);
            for (k, a) in alphas.iter_mut().enumerate() {
                *a = table[((bits >> (3 * k)) & 7) as usize];
            }
            &b[8..16]
        } else {
            &b[0..8]
        };
        let c0 = u16::from_le_bytes([c[0], c[1]]);
        let c1 = u16::from_le_bytes([c[2], c[3]]);
        let (p0, p1) = (rgb565(c0), rgb565(c1));
        let mix = |a: u8, b: u8, wa: u16, wb: u16, d: u16| {
            ((u16::from(a) * wa + u16::from(b) * wb) / d) as u8
        };
        let mut colors = [
            [p0[0], p0[1], p0[2], 255],
            [p1[0], p1[1], p1[2], 255],
            [0; 4],
            [0; 4],
        ];
        if c0 > c1 || alpha {
            colors[2] = [
                mix(p0[0], p1[0], 2, 1, 3),
                mix(p0[1], p1[1], 2, 1, 3),
                mix(p0[2], p1[2], 2, 1, 3),
                255,
            ];
            colors[3] = [
                mix(p0[0], p1[0], 1, 2, 3),
                mix(p0[1], p1[1], 1, 2, 3),
                mix(p0[2], p1[2], 1, 2, 3),
                255,
            ];
        } else {
            colors[2] = [
                mix(p0[0], p1[0], 1, 1, 2),
                mix(p0[1], p1[1], 1, 1, 2),
                mix(p0[2], p1[2], 1, 1, 2),
                255,
            ];
        }
        let idx = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
        for k in 0..16 {
            let (x, y) = (bx + k % 4, by + k / 4);
            if x >= w || y >= h {
                continue;
            }
            let mut px = colors[((idx >> (2 * k)) & 3) as usize];
            if alpha {
                px[3] = alphas[k];
            }
            out[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&px);
        }
    }
    Some(out)
}

fn load_textures(shared: &Mutex<Shared>, ctx: &egui::Context, bundle: &Bundle, names: &[String]) {
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

/// Cuts one picture from an atlas (rectangle from the top) and shrinks it so
/// its bigger side is at most `ICON_MAX`; colours are averaged by alpha.
fn cut_icon(atlas: &Image, (x, y, w, h): (u32, u32, u32, u32), max: u32) -> Option<Image> {
    if x + w > atlas.w || y + h > atlas.h || w == 0 || h == 0 {
        return None;
    }
    let f = w.max(h).div_ceil(max).max(1);
    let (ow, oh) = (w.div_ceil(f), h.div_ceil(f));
    let mut rgba = Vec::with_capacity((ow * oh * 4) as usize);
    for oy in 0..oh {
        for ox in 0..ow {
            let mut acc = [0f32; 4];
            let mut n = 0f32;
            for dy in 0..f {
                for dx in 0..f {
                    let (sx, sy) = (ox * f + dx, oy * f + dy);
                    if sx >= w || sy >= h {
                        continue;
                    }
                    let row = atlas.h - 1 - (y + sy);
                    let i = ((row * atlas.w + x + sx) * 4) as usize;
                    let p = &atlas.rgba[i..i + 4];
                    let a = f32::from(p[3]);
                    for k in 0..3 {
                        acc[k] += f32::from(p[k]) * a;
                    }
                    acc[3] += a;
                    n += 1.0;
                }
            }
            for k in 0..3 {
                rgba.push(if acc[3] > 0.0 {
                    (acc[k] / acc[3]).round() as u8
                } else {
                    0
                });
            }
            rgba.push((acc[3] / n.max(1.0)).round() as u8);
        }
    }
    Some(Image { w: ow, h: oh, rgba })
}

/// Icon key and its rectangle in an atlas.
type IconRect<'a> = (&'a String, u32, u32, u32, u32);

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
        let b = p.is_file().then(|| Bundle::open(&p)).flatten()?;
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
    for (gender, kit) in &art.dweller {
        match read_mesh(&bundle, kit) {
            Some(m) => {
                if let Ok(mut s) = shared.lock() {
                    s.meshes.insert(gender.clone(), Arc::new(m));
                }
            }
            None => mismatch(),
        }
    }
    let mut names: Vec<&String> = Vec::new();
    for kit in art.dweller.values() {
        names.extend([&kit.body.tex, &kit.hands.tex, &kit.outfit.tex]);
        names.extend(kit.faces.values().map(|p| &p.tex));
    }
    for kit in art.dweller.values() {
        names.extend(kit.hairs.values().map(|p| &p.tex));
        names.extend(kit.face_masks.values().map(|p| &p.tex));
    }
    let mut seen = std::collections::HashSet::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }
        match decode(&bundle, name) {
            Some(img) => {
                if let Ok(mut s) = shared.lock() {
                    s.images.insert(name.clone(), Arc::new(img));
                }
            }
            None => mismatch(),
        }
        ctx.request_repaint();
    }
    let mut by_atlas: HashMap<&String, Vec<IconRect<'_>>> = HashMap::new();
    for (key, (tex, x, y, w, h)) in &art.icons {
        by_atlas.entry(tex).or_default().push((key, *x, *y, *w, *h));
    }
    for (tex, list) in by_atlas {
        let Some(atlas) = decode(&bundle, tex) else {
            mismatch();
            continue;
        };
        let cut: Vec<(String, Arc<Image>)> = list
            .into_iter()
            .filter_map(|(key, x, y, w, h)| {
                let max = if key.starts_with("Room:") || key.starts_with("Exterior:") {
                    ROOM_MAX
                } else {
                    ICON_MAX
                };
                cut_icon(&atlas, (x, y, w, h), max).map(|i| (key.clone(), Arc::new(i)))
            })
            .collect();
        if let Ok(mut s) = shared.lock() {
            s.icons.extend(cut);
        }
        ctx.request_repaint();
    }
}

fn read_mesh(bundle: &Bundle, kit: &ArtDweller) -> Option<Mesh> {
    let m = &kit.mesh;
    let v = bundle
        .read(&m.node, m.vertex_offset, m.vertex_size)
        .filter(|d| crc32(d) == m.vertex_crc)?;
    let i = bundle
        .read(&m.node, m.index_offset, m.index_size)
        .filter(|d| crc32(d) == m.index_crc)?;
    Mesh::decode(&v, &i, m.vertex_count, m.uv_start, m.bone_start)
}

/// What the preview shows: the dweller's own look in the basic jumpsuit.
#[derive(Clone, PartialEq)]
pub struct Look {
    pub male: bool,
    pub skin: [f32; 3],
    pub hair_color: [f32; 3],
    pub hair: Option<String>,
    pub face_mask: Option<String>,
    pub face: String,
    /// Outfit id (`__pregnant` for the maternity dress); None draws the jumpsuit.
    pub outfit: Option<String>,
    pub outfit_color: [f32; 3],
    /// The dweller's own helmet and helmet cover (save keys `helmet`, `helmetCoverCustomization`).
    pub helmet: Option<String>,
    pub helmet_cover: Option<String>,
}

impl Look {
    fn key(&self) -> String {
        format!(
            "{}|{:?}|{:?}|{:?}|{:?}|{}|{:?}|{:?}|{:?}|{:?}",
            self.male,
            self.skin,
            self.hair_color,
            self.hair,
            self.face_mask,
            self.face,
            self.outfit,
            self.outfit_color,
            self.helmet,
            self.helmet_cover
        )
    }
}

/// Helmet the game draws inside the vault: the cover, else the outfit's own,
/// else the dweller's; helmets taken off in the vault are not drawn.
fn worn_helmet(
    kit: &'static ArtDweller,
    outfit: Option<&'static ArtOutfit>,
    look: &Look,
) -> Option<&'static ArtHelmet> {
    let own = |name: &Option<String>| name.as_ref().and_then(|n| kit.helmets.get(n));
    own(&look.helmet_cover)
        .or_else(|| outfit.and_then(|o| o.helmet.as_ref()))
        .or_else(|| {
            if outfit.is_some_and(|o| o.large_headgear) {
                None
            } else {
                own(&look.helmet)
            }
        })
        .filter(|h| !h.remove_in_vault)
}

/// Outfit art for a dweller; None when the game files here do not have it.
pub fn outfit_art(male: bool, id: &str) -> Option<&'static ArtOutfit> {
    let gender = if male { "male" } else { "female" };
    catalog().art.as_ref()?.dweller.get(gender)?.outfits.get(id)
}

/// The colour the game uses for an outfit: the nearest allowed one, or white.
pub fn outfit_color(outfit: Option<&ArtOutfit>, argb: i64) -> u32 {
    outfit.map_or(0xFFFF_FFFF, |o| o.game_color(argb as u32))
}

/// Face the game shows for a happiness value.
pub fn face_for(happiness: f64) -> String {
    let Some(art) = catalog().art.as_ref() else {
        return "Neutral".into();
    };
    if happiness <= 0.0 {
        return "Sad".into();
    }
    art.face_by_happiness
        .iter()
        .find(|(min, _)| happiness > *min)
        .map_or_else(|| "Sad".to_string(), |(_, f)| f.clone())
}

pub fn rgb(argb: i64) -> [f32; 3] {
    let c = argb as u32;
    [
        ((c >> 16) & 255) as f32 / 255.0,
        ((c >> 8) & 255) as f32 / 255.0,
        (c & 255) as f32 / 255.0,
    ]
}

struct Mesh {
    pos: Vec<[f32; 2]>,
    uv: Vec<[f32; 2]>,
    bone: Vec<usize>,
    tris: Vec<[usize; 3]>,
}

fn le_f32(b: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

impl Mesh {
    fn decode(v: &[u8], idx: &[u8], n: usize, uv_start: usize, bone_start: usize) -> Option<Mesh> {
        let mut pos = Vec::with_capacity(n);
        let mut uv = Vec::with_capacity(n);
        let mut bone = Vec::with_capacity(n);
        for k in 0..n {
            pos.push([le_f32(v, k * 12)?, le_f32(v, k * 12 + 4)?]);
            uv.push([
                le_f32(v, uv_start + k * 16)?,
                le_f32(v, uv_start + k * 16 + 4)?,
            ]);
            bone.push(u32::from_le_bytes(
                v.get(bone_start + k * 4..bone_start + k * 4 + 4)?
                    .try_into()
                    .ok()?,
            ) as usize);
        }
        let ids: Vec<usize> = idx
            .chunks_exact(2)
            .map(|c| usize::from(u16::from_le_bytes([c[0], c[1]])))
            .collect();
        if ids.iter().any(|i| *i >= n) {
            return None;
        }
        let tris = ids.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
        Some(Mesh {
            pos,
            uv,
            bone,
            tris,
        })
    }
}

type Affine = [f32; 6];

fn rotate_about(angle: f32, o: [f32; 2]) -> Affine {
    let (s, c) = angle.sin_cos();
    [
        c,
        -s,
        s,
        c,
        o[0] - c * o[0] + s * o[1],
        o[1] - s * o[0] - c * o[1],
    ]
}

fn compose(a: Affine, b: Affine) -> Affine {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[0] * b[4] + a[1] * b[5] + a[4],
        a[2] * b[4] + a[3] * b[5] + a[5],
    ]
}

/// Mask the game uses to keep face and hand pieces inside their area.
fn clip(c: [f32; 4], u: f32, v: f32) -> f32 {
    let x = (u * c[0] + c[2]).abs();
    let y = (v * c[1] + c[3]).abs();
    ((1.0 - x).min(1.0 - y) + 10.0).clamp(0.0, 1.0)
}

const HEIGHT: f32 = 330.0;
const SS: usize = 2;

/// Same layer order and blending as the game's dweller shader, with the arms
/// lowered from the bind pose.
fn render_dweller(job: &Job) -> Option<egui::ColorImage> {
    let (kit, mesh, images, look) = (job.kit, &*job.mesh, &job.images, &job.look);
    let (face, hair, mask, outfit) = (job.face, job.hair, job.mask, job.outfit);
    let img = |p: &ArtPiece| images.get(&p.tex).cloned();
    let tex = |t: &String| images.get(t).cloned();
    let body = match (outfit.is_some_and(|o| o.skirt), &kit.skirt_body) {
        (true, Some(b)) => b,
        _ => &kit.body,
    };
    let body_img = img(body)?;
    let hands_img = img(&kit.hands)?;
    let (outfit_img, os, oo) = match outfit {
        Some(o) => (tex(&o.tex)?, o.scale, o.offset),
        None => (img(&kit.outfit)?, kit.outfit.scale, kit.outfit.offset),
    };
    let coloring = outfit.and_then(|o| o.coloring_mask.as_ref());
    let coloring_img = coloring.and_then(img);
    // Without its own mask the outfit is tinted through the burn mask, at body scale.
    let burn_img = match (outfit, coloring) {
        (Some(o), None) if !o.colors.is_empty() => kit.burn_mask.as_ref().and_then(tex),
        _ => None,
    };
    let glove = outfit.and_then(|o| o.glove.as_ref());
    let glove_img = glove.and_then(img);
    let draw_hands = outfit.is_none_or(|o| o.glove.is_none() || o.glove_hands);
    let helmet = job.helmet;
    let helmet_img = helmet.and_then(|h| tex(&h.tex));
    let helmet_mask = helmet.and_then(|h| h.mask.as_ref());
    let helmet_mask_img = helmet_mask.and_then(img);
    let face_img = img(face)?;
    let hair_img = hair.and_then(img);
    let mask_img = mask.and_then(img);
    let mut pos = mesh.pos.clone();
    let mut tf: HashMap<usize, Affine> = HashMap::new();
    for (side, chain) in kit.arms.iter().enumerate() {
        let sign = if side == 0 { 1.0 } else { -1.0 };
        let angles = [80f32.to_radians() * sign, -12f32.to_radians() * sign, 0.0];
        let mut t: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        for (b, a) in chain.iter().zip(angles) {
            let joint = *kit.mesh.arm_joints.get(&b.to_string())?;
            t = compose(t, rotate_about(a, joint));
            tf.insert(*b, t);
        }
    }
    for (k, p) in pos.iter_mut().enumerate() {
        if let Some(t) = tf.get(&mesh.bone[k]) {
            let (x, y) = (p[0], p[1]);
            *p = [t[0] * x + t[1] * y + t[4], t[2] * x + t[3] * y + t[5]];
        }
    }
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in &pos {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    let scale = HEIGHT * SS as f32 / (y1 - y0).max(0.01);
    let pad = 4.0 * SS as f32;
    let w = (((x1 - x0) * scale + 2.0 * pad) as usize).div_ceil(SS) * SS;
    let h = (((y1 - y0) * scale + 2.0 * pad) as usize).div_ceil(SS) * SS;
    let mut buf = vec![[0f32; 4]; w * h];
    let skin = mask.and_then(|m| m.custom_skin).unwrap_or(look.skin);
    let deco = match mask {
        Some(m) if m.hair_color => look.hair_color,
        Some(m) if m.skin_color => skin,
        _ => [1.0, 1.0, 1.0],
    };
    let bs = body.scale;
    let bo = body.offset;
    let hn = 1024.0 / hands_img.w as f32;
    let hands_st = [
        bs[0] * hn,
        bs[1] * hn,
        kit.hands.offset[0] + kit.hand_offset[0] * hn,
        kit.hands.offset[1] + kit.hand_offset[1] * hn,
    ];
    let face_at = [
        face.offset[0] + kit.face_offset[0],
        face.offset[1] + kit.face_offset[1],
    ];
    let mask_at = mask.map(|m| {
        [
            m.offset[0] + kit.face_offset[0],
            m.offset[1] + kit.face_offset[1],
        ]
    });
    let hair_st = hair.zip(hair_img.as_ref()).map(|(p, i)| {
        let n = 1024.0 / i.w as f32;
        [
            bs[0] * n,
            bs[1] * n,
            p.offset[0] + kit.face_offset[0] * n,
            p.offset[1] + kit.face_offset[1] * n,
        ]
    });
    let glove_st = glove.zip(glove_img.as_ref()).map(|(p, i)| {
        let n = 1024.0 / i.w as f32;
        [
            bs[0] * n,
            bs[1] * n,
            p.offset[0] + kit.hand_offset[0] * n,
            p.offset[1] + kit.hand_offset[1] * n,
        ]
    });
    let helmet_st = helmet.zip(helmet_img.as_ref()).map(|(p, i)| {
        let n = 1024.0 / i.w as f32;
        [
            bs[0] * n,
            bs[1] * n,
            p.offset[0] + kit.face_offset[0] * n,
            p.offset[1] + kit.face_offset[1] * n,
        ]
    });
    let helmet_mask_at = helmet_mask.map(|m| {
        [
            m.offset[0] + kit.face_offset[0],
            m.offset[1] + kit.face_offset[1],
        ]
    });
    let tint = look.outfit_color;
    let shade = |u: f32, v: f32| -> [f32; 4] {
        let over = |col: &mut [f32; 3], a: &mut f32, rgb: [f32; 3], tint: [f32; 3], alpha: f32| {
            for k in 0..3 {
                col[k] = col[k] * (1.0 - alpha) + rgb[k] * tint[k] * alpha;
            }
            *a = (*a + alpha).min(1.0);
        };
        let (bu, bv) = (u * bs[0], v * bs[1]);
        let b = body_img.sample(bu + bo[0], bv + bo[1]);
        let mut col = [
            b[0] * skin[0] * b[3],
            b[1] * skin[1] * b[3],
            b[2] * skin[2] * b[3],
        ];
        let mut a = b[3];
        let hm = clip(kit.hand_clip, u, v);
        let fm = clip(kit.face_clip, u, v);
        if draw_hands {
            let hd = hands_img.sample(u * hands_st[0] + hands_st[2], v * hands_st[1] + hands_st[3]);
            over(
                &mut col,
                &mut a,
                [hd[0] * hm, hd[1] * hm, hd[2] * hm],
                skin,
                hd[3] * hm,
            );
        }
        if let (Some(gi), Some(st)) = (glove_img.as_ref(), glove_st) {
            let g = gi.sample(u * st[0] + st[2], v * st[1] + st[3]);
            over(
                &mut col,
                &mut a,
                [g[0] * hm, g[1] * hm, g[2] * hm],
                [1.0; 3],
                g[3] * hm,
            );
        }
        let o = outfit_img.sample(u * os[0] + oo[0], v * os[1] + oo[1]);
        // The colouring mask says where the outfit colour applies.
        let m = match (coloring, coloring_img.as_ref(), burn_img.as_ref()) {
            (Some(p), Some(i), _) => {
                i.sample(u * p.scale[0] + p.offset[0], v * p.scale[1] + p.offset[1])[3]
            }
            (None, _, Some(b)) => b.sample(u * bs[0], v * bs[1])[3],
            _ => 0.0,
        };
        let shaded = [
            o[0] * (1.0 + (tint[0] - 1.0) * m),
            o[1] * (1.0 + (tint[1] - 1.0) * m),
            o[2] * (1.0 + (tint[2] - 1.0) * m),
        ];
        over(&mut col, &mut a, shaded, [1.0; 3], o[3]);
        let f = face_img.sample(bu + face_at[0], bv + face_at[1]);
        over(
            &mut col,
            &mut a,
            [f[0] * fm, f[1] * fm, f[2] * fm],
            [1.0; 3],
            f[3] * fm,
        );
        if let (Some(mi), Some(at)) = (mask_img.as_ref(), mask_at) {
            let d = mi.sample(bu + at[0], bv + at[1]);
            over(
                &mut col,
                &mut a,
                [d[0] * fm, d[1] * fm, d[2] * fm],
                deco,
                d[3] * fm,
            );
        }
        let hair_clip = match (helmet_mask_img.as_ref(), helmet_mask_at) {
            (Some(mi), Some(at)) => fm * mi.sample(bu + at[0], bv + at[1])[3],
            _ => fm,
        };
        if let (Some(hi), Some(st)) = (hair_img.as_ref(), hair_st) {
            let r = hi.sample(u * st[0] + st[2], v * st[1] + st[3]);
            over(
                &mut col,
                &mut a,
                [r[0] * hair_clip, r[1] * hair_clip, r[2] * hair_clip],
                look.hair_color,
                r[3] * hair_clip,
            );
        }
        if let (Some(hi), Some(st)) = (helmet_img.as_ref(), helmet_st) {
            let r = hi.sample(u * st[0] + st[2], v * st[1] + st[3]);
            over(
                &mut col,
                &mut a,
                [r[0] * fm, r[1] * fm, r[2] * fm],
                [1.0; 3],
                r[3] * fm,
            );
        }
        [col[0], col[1], col[2], a]
    };
    for t in &mesh.tris {
        let p: Vec<[f32; 2]> = t
            .iter()
            .map(|&k| {
                [
                    (pos[k][0] - x0) * scale + pad,
                    (y1 - pos[k][1]) * scale + pad,
                ]
            })
            .collect();
        let uvs: Vec<[f32; 2]> = t.iter().map(|&k| mesh.uv[k]).collect();
        let den =
            (p[1][1] - p[2][1]) * (p[0][0] - p[2][0]) + (p[2][0] - p[1][0]) * (p[0][1] - p[2][1]);
        if den.abs() < 1e-9 {
            continue;
        }
        let minx = p
            .iter()
            .map(|q| q[0])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let maxx = (p.iter().map(|q| q[0]).fold(f32::MIN, f32::max).ceil() as usize).min(w);
        let miny = p
            .iter()
            .map(|q| q[1])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let maxy = (p.iter().map(|q| q[1]).fold(f32::MIN, f32::max).ceil() as usize).min(h);
        for yy in miny..maxy {
            for xx in minx..maxx {
                let (qx, qy) = (xx as f32 + 0.5, yy as f32 + 0.5);
                let w1 = ((p[1][1] - p[2][1]) * (qx - p[2][0])
                    + (p[2][0] - p[1][0]) * (qy - p[2][1]))
                    / den;
                let w2 = ((p[2][1] - p[0][1]) * (qx - p[2][0])
                    + (p[0][0] - p[2][0]) * (qy - p[2][1]))
                    / den;
                let w3 = 1.0 - w1 - w2;
                if w1 < -1e-5 || w2 < -1e-5 || w3 < -1e-5 {
                    continue;
                }
                let u = w1 * uvs[0][0] + w2 * uvs[1][0] + w3 * uvs[2][0];
                let v = w1 * uvs[0][1] + w2 * uvs[1][1] + w3 * uvs[2][1];
                let s = shade(u, v);
                if s[3] <= 0.0 {
                    continue;
                }
                let d = &mut buf[yy * w + xx];
                for k in 0..3 {
                    d[k] = s[k] + d[k] * (1.0 - s[3]);
                }
                d[3] = s[3] + d[3] * (1.0 - s[3]);
            }
        }
    }
    let (ow, oh) = (w / SS, h / SS);
    let mut out = Vec::with_capacity(ow * oh * 4);
    let n = (SS * SS) as f32;
    for y in 0..oh {
        for x in 0..ow {
            let mut acc = [0f32; 4];
            for dy in 0..SS {
                for dx in 0..SS {
                    let px = buf[(y * SS + dy) * w + x * SS + dx];
                    for k in 0..4 {
                        acc[k] += px[k];
                    }
                }
            }
            for v in acc {
                out.push((v / n * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
    }
    Some(egui::ColorImage::from_rgba_premultiplied([ow, oh], &out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_reference() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn lz4_literals_and_matches() {
        assert_eq!(
            lz4_decompress(b"\x50hello", 5).as_deref(),
            Some(&b"hello"[..])
        );
        let block = b"\x32abc\x03\x00\x00";
        assert_eq!(lz4_decompress(block, 9).as_deref(), Some(&b"abcabcabc"[..]));
        assert!(lz4_decompress(block, 8).is_none());
        assert!(lz4_decompress(b"\x0f\x05\x00", 4).is_none());
    }

    #[test]
    fn dxt_blocks_decode() {
        // Red and blue end points; first pixel red, second blue, rest red.
        let mut block = vec![0x00, 0xF8, 0x1F, 0x00];
        block.extend([0b0000_0100, 0, 0, 0]);
        let px = decode_bc(&block, 4, 4, false).unwrap();
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);
        assert_eq!(&px[4..8], &[0, 0, 255, 255]);
        let mut dxt5 = vec![255, 0, 0b0000_0001, 0, 0, 0, 0, 0];
        dxt5.extend(&block);
        let px = decode_bc(&dxt5, 4, 4, true).unwrap();
        assert_eq!(px[3], 0);
        assert_eq!(px[7], 255);
    }

    #[test]
    fn happiness_faces_follow_the_game() {
        assert_eq!(face_for(100.0), "Smile");
        assert_eq!(face_for(55.0), "Neutral");
        assert_eq!(face_for(20.0), "Sad");
        assert_eq!(face_for(0.0), "Sad");
    }
}
