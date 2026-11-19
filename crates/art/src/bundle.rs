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

//! Unity bundles with LZ4 blocks (UnityFS), read piece by piece.

use std::collections::HashMap;

/// Bytes of the file start that hold the header.
pub const HEAD_SIZE: usize = 512;

struct Block {
    raw_size: u32,
    packed_size: u32,
    flags: u16,
    packed_at: u64,
    raw_at: u64,
}

/// Where the block and node list sits, read from the header.
#[derive(Clone, Copy, Debug)]
pub struct Head {
    pub info_at: u64,
    pub info_len: usize,
    raw_len: usize,
    flags: u32,
}

/// Block and node list of a bundle; data is read separately.
pub struct Index {
    blocks: Vec<Block>,
    nodes: HashMap<String, (u64, u64)>,
}

/// Something the bundle bytes can be read from.
pub trait Source {
    fn read_at(&self, at: u64, len: usize) -> Option<Vec<u8>>;
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
        if off == 0 || off > out.len() || out.len() + len > out_len {
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

/// Reads the header at the start of the file.
pub fn parse_head(head: &[u8]) -> Option<Head> {
    let mut at = 0;
    if cstr(head, &mut at)? != "UnityFS" {
        return None;
    }
    let version = be_u32(head, at)?;
    at += 4;
    cstr(head, &mut at)?;
    cstr(head, &mut at)?;
    at += 8;
    let packed = be_u32(head, at)? as usize;
    let raw = be_u32(head, at + 4)? as usize;
    let flags = be_u32(head, at + 8)?;
    at += 12;
    if version >= 7 {
        at = at.div_ceil(16) * 16;
    }
    // Lists stored at the end of the file are not used by the game builds.
    if flags & 0x80 != 0 || packed > 64 << 20 || raw > 64 << 20 {
        return None;
    }
    Some(Head {
        info_at: at as u64,
        info_len: packed,
        raw_len: raw,
        flags,
    })
}

impl Index {
    /// Block and node list from the bytes at `head.info_at`.
    pub fn new(head: &Head, info: &[u8]) -> Option<Index> {
        let info = match head.flags & 0x3f {
            0 => info.to_vec(),
            2 | 3 => lz4_decompress(info, head.raw_len)?,
            _ => return None,
        };
        let mut data_start = head.info_at + head.info_len as u64;
        if head.flags & 0x200 != 0 {
            data_start = data_start.div_ceil(16) * 16;
        }
        let mut p = 16;
        let count = be_u32(&info, p)? as usize;
        p += 4;
        let mut blocks = Vec::with_capacity(count.min(1 << 16));
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
        Some(Index { blocks, nodes })
    }

    fn span(
        &self,
        node: &str,
        offset: u64,
        size: u64,
    ) -> Option<(u64, u64, std::ops::Range<usize>)> {
        let (node_at, node_size) = *self.nodes.get(node)?;
        if offset.checked_add(size)? > node_size {
            return None;
        }
        let from = node_at + offset;
        let to = from + size;
        let first = self
            .blocks
            .partition_point(|b| b.raw_at + u64::from(b.raw_size) <= from);
        let last = self.blocks.partition_point(|b| b.raw_at < to);
        Some((from, to, first..last))
    }

    /// Packed file bytes (start, length) that hold `size` bytes of a node.
    pub fn range(&self, node: &str, offset: u64, size: u64) -> Option<(u64, usize)> {
        let (_, _, blocks) = self.span(node, offset, size)?;
        let bs = self.blocks.get(blocks)?;
        let start = bs.first()?.packed_at;
        let end = bs.last().map(|b| b.packed_at + u64::from(b.packed_size))?;
        Some((start, usize::try_from(end - start).ok()?))
    }

    /// Bytes of a node from the packed bytes returned for [`Index::range`].
    pub fn extract(&self, node: &str, offset: u64, size: u64, packed: &[u8]) -> Option<Vec<u8>> {
        let (from, to, blocks) = self.span(node, offset, size)?;
        let bs = self.blocks.get(blocks)?;
        let base = bs.first()?.packed_at;
        let mut out = Vec::with_capacity(usize::try_from(size).ok()?);
        for b in bs {
            let at = usize::try_from(b.packed_at - base).ok()?;
            let src = packed.get(at..at + b.packed_size as usize)?;
            let raw = match b.flags & 0x3f {
                0 => src.to_vec(),
                2 | 3 => lz4_decompress(src, b.raw_size as usize)?,
                _ => return None,
            };
            let s = from.saturating_sub(b.raw_at) as usize;
            let e = ((to - b.raw_at) as usize).min(raw.len());
            out.extend_from_slice(raw.get(s..e)?);
        }
        (out.len() as u64 == size).then_some(out)
    }
}

/// A bundle on something that can be read at any position.
pub struct Bundle<S: Source> {
    src: S,
    index: Index,
}

impl<S: Source> Bundle<S> {
    pub fn open(src: S) -> Option<Bundle<S>> {
        let head = parse_head(&src.read_at(0, HEAD_SIZE)?)?;
        let info = src.read_at(head.info_at, head.info_len)?;
        let index = Index::new(&head, &info)?;
        Some(Bundle { src, index })
    }

    /// Bytes `offset..offset+size` of a node, decompressing only the blocks they touch.
    pub fn read(&self, node: &str, offset: u64, size: u64) -> Option<Vec<u8>> {
        let (at, len) = self.index.range(node, offset, size)?;
        let packed = self.src.read_at(at, len)?;
        self.index.extract(node, offset, size, &packed)
    }
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
    fn bad_head_is_refused() {
        assert!(parse_head(b"NotUnity\0").is_none());
        assert!(parse_head(b"").is_none());
    }
}
