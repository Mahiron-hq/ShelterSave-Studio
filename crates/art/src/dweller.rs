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

//! Dweller picture: a port of the game's dweller shader on its body mesh.

use std::collections::HashMap;
use std::sync::Arc;

use shelter_core::catalog::{catalog, ArtDweller, ArtHelmet, ArtOutfit, ArtPiece};
use shelter_core::json::Value;
use shelter_core::vault;

use crate::bundle::crc32;
use crate::image::Image;

/// What the preview shows: the dweller's own look and the outfit they wear.
#[derive(Clone, PartialEq, Debug)]
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
    /// Cache key of the picture.
    pub fn key(&self) -> String {
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

/// Outfit the game draws on a dweller: the maternity dress while pregnant.
pub fn worn_outfit(d: &Value) -> Option<String> {
    let male = vault::int(d, &["gender"]) == Some(2);
    if !male && vault::flag(d, &["pregnant"]) == Some(true) {
        return Some("__pregnant".into());
    }
    d.get("equipedOutfit")
        .and_then(|o| o.get("id"))
        .and_then(Value::as_text)
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

/// The look of a dweller from the save.
pub fn look_of(d: &Value) -> Look {
    let happiness = d
        .at(&vault::p(&["happiness", "happinessValue"]))
        .and_then(Value::as_f64)
        .unwrap_or(50.0);
    let male = vault::int(d, &["gender"]) == Some(2);
    let worn = worn_outfit(d);
    let art = worn.as_deref().and_then(|o| outfit_art(male, o));
    let tint = if worn.as_deref() == Some("__pregnant") {
        0xFFFF_FFFF
    } else {
        outfit_color(art, vault::int(d, &["outfitColor"]).unwrap_or(0xFFFF_FFFF))
    };
    Look {
        male,
        skin: rgb(vault::int(d, &["skinColor"]).unwrap_or(0xFFFF_E9D6)),
        hair_color: rgb(vault::int(d, &["hairColor"]).unwrap_or(0xFFCF_B038)),
        hair: vault::text(d, &["hair"]),
        face_mask: vault::text(d, &["faceMask"]).filter(|m| !m.is_empty()),
        face: if vault::is_dead(d) {
            "Dead".into()
        } else {
            face_for(happiness)
        },
        outfit: art.and(worn),
        outfit_color: rgb(i64::from(tint)),
        helmet: vault::text(d, &["helmet"]),
        helmet_cover: vault::text(d, &["helmetCoverCustomization"]),
    }
}

/// Why the preview may differ from the game, as an interface string key.
pub fn preview_note(d: &Value) -> Option<&'static str> {
    let male = vault::int(d, &["gender"]) == Some(2);
    let gender = vault::int(d, &["gender"]).unwrap_or(1);
    let worn = worn_outfit(d);
    let art = worn.as_deref().and_then(|o| outfit_art(male, o));
    match art {
        None if worn
            .as_deref()
            .is_some_and(|w| !catalog().outfit_fits(w, gender)) =>
        {
            Some("dw.previewWrongGender")
        }
        None if d.get("equipedOutfit").is_some() && worn.is_some() => Some("dw.previewNoOutfit"),
        Some(a) if a.large_headgear => Some("dw.previewNoHeadgear"),
        _ => None,
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

/// Body mesh of one gender.
pub struct Mesh {
    pos: Vec<[f32; 2]>,
    uv: Vec<[f32; 2]>,
    bone: Vec<usize>,
    tris: Vec<[usize; 3]>,
}

fn le_f32(b: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Bundle node and byte ranges of the vertex and index data.
pub type MeshPlace = (String, (u64, u64), (u64, u64));

/// Where the mesh of a gender lies: node, then (offset, size) of vertices and indices.
pub fn mesh_place(gender: &str) -> Option<MeshPlace> {
    let m = &catalog().art.as_ref()?.dweller.get(gender)?.mesh;
    Some((
        m.node.clone(),
        (m.vertex_offset, m.vertex_size),
        (m.index_offset, m.index_size),
    ))
}

impl Mesh {
    /// Mesh from its vertex and index bytes; None when they differ from the catalog.
    pub fn new(gender: &str, v: &[u8], idx: &[u8]) -> Option<Mesh> {
        let m = &catalog().art.as_ref()?.dweller.get(gender)?.mesh;
        if crc32(v) != m.vertex_crc || crc32(idx) != m.index_crc {
            return None;
        }
        let (n, uv_start, bone_start) = (m.vertex_count, m.uv_start, m.bone_start);
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
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| usize::from(u16::from_le_bytes(*c)))
            .collect();
        if ids.iter().any(|i| *i >= n) {
            return None;
        }
        let tris = ids.as_chunks::<3>().0.to_vec();
        Some(Mesh {
            pos,
            uv,
            bone,
            tris,
        })
    }
}

/// Textures every dweller picture can use, loaded up front.
pub fn base_textures() -> Vec<String> {
    let Some(art) = catalog().art.as_ref() else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    for kit in art.dweller.values() {
        names.extend([
            kit.body.tex.clone(),
            kit.hands.tex.clone(),
            kit.outfit.tex.clone(),
        ]);
        names.extend(kit.faces.values().map(|p| p.tex.clone()));
    }
    for kit in art.dweller.values() {
        names.extend(kit.hairs.values().map(|p| p.tex.clone()));
        names.extend(kit.face_masks.values().map(|p| p.tex.clone()));
    }
    let mut seen = std::collections::HashSet::new();
    names.retain(|n| seen.insert(n.clone()));
    names
}

/// Everything one picture needs.
pub struct Parts {
    kit: &'static ArtDweller,
    images: HashMap<String, Arc<Image>>,
    look: Look,
    face: &'static ArtPiece,
    hair: Option<&'static ArtPiece>,
    mask: Option<&'static ArtPiece>,
    outfit: Option<&'static ArtOutfit>,
    helmet: Option<&'static ArtHelmet>,
}

impl Parts {
    /// The parts, or the textures still to be loaded (empty when it cannot be drawn).
    pub fn new(images: &HashMap<String, Arc<Image>>, look: &Look) -> Result<Parts, Vec<String>> {
        let gender = if look.male { "male" } else { "female" };
        let kit = catalog()
            .art
            .as_ref()
            .and_then(|a| a.dweller.get(gender))
            .ok_or_else(Vec::new)?;
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
        let mut have = HashMap::new();
        let mut missing = Vec::new();
        for n in needed {
            match images.get(n) {
                Some(img) => {
                    have.insert(n.clone(), img.clone());
                }
                None => missing.push(n.clone()),
            }
        }
        if !missing.is_empty() {
            missing.sort();
            missing.dedup();
            return Err(missing);
        }
        Ok(Parts {
            kit,
            images: have,
            look: look.clone(),
            face,
            hair,
            mask,
            outfit,
            helmet,
        })
    }
}

/// Premultiplied RGBA picture, top row first.
pub struct Picture {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
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
pub fn render(job: &Parts, mesh: &Mesh) -> Option<Picture> {
    let (kit, images, look) = (job.kit, &job.images, &job.look);
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
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return None;
    }
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
    let piece_st = |p: &ArtPiece, i: &Image, at: [f32; 2]| {
        let n = 1024.0 / i.w as f32;
        [
            bs[0] * n,
            bs[1] * n,
            p.offset[0] + at[0] * n,
            p.offset[1] + at[1] * n,
        ]
    };
    let hair_st = hair
        .zip(hair_img.as_ref())
        .map(|(p, i)| piece_st(p, i, kit.face_offset));
    let glove_st = glove
        .zip(glove_img.as_ref())
        .map(|(p, i)| piece_st(p, i, kit.hand_offset));
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
    Some(Picture {
        w: ow,
        h: oh,
        rgba: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happiness_faces_follow_the_game() {
        assert_eq!(face_for(100.0), "Smile");
        assert_eq!(face_for(55.0), "Neutral");
        assert_eq!(face_for(20.0), "Sad");
        assert_eq!(face_for(0.0), "Sad");
    }
}
