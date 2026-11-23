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

//! Texture decoding and cutting pictures out of atlases.

use shelter_core::catalog::catalog;

use crate::bundle::crc32;

/// Biggest side of a cut item picture; cards show them at 120 points.
pub const ICON_MAX: u32 = 160;
/// Room pictures and vault exteriors are kept larger.
pub const ROOM_MAX: u32 = 512;

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
    pub fn sample(&self, u: f32, v: f32) -> [f32; 4] {
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

/// Where a texture lies in the bundle: node, offset, size.
pub fn texture_place(name: &str) -> Option<(String, u64, u64)> {
    let info = catalog().art.as_ref()?.textures.get(name)?;
    Some((info.node.clone(), info.offset, info.size))
}

/// Texture from its raw bytes; None when they differ from the catalog.
pub fn texture(name: &str, raw: Vec<u8>) -> Option<Image> {
    let info = catalog().art.as_ref()?.textures.get(name)?;
    if crc32(&raw) != info.crc {
        return None;
    }
    let (w, h) = (info.width, info.height);
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return None;
    }
    let rgba = match info.format.as_str() {
        "RGBA32" => raw,
        "ARGB32" => {
            let mut v = raw;
            for p in v.as_chunks_mut::<4>().0 {
                p.rotate_left(1);
            }
            v
        }
        "RGB24" => raw
            .as_chunks::<3>()
            .0
            .iter()
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

/// Cuts one picture from an atlas (rectangle from the top) and shrinks it so
/// its bigger side is at most `max`; colours are averaged by alpha. The result
/// has the top row first.
pub fn cut_icon(atlas: &Image, (x, y, w, h): (u32, u32, u32, u32), max: u32) -> Option<Image> {
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

/// Largest side kept for a picture key (`Weapon:..`, `Room:..`).
pub fn max_side(key: &str) -> u32 {
    if key.starts_with("Room:") || key.starts_with("Exterior:") {
        ROOM_MAX
    } else {
        ICON_MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
