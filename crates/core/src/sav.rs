// ShelterSave-Studio
// Copyright (C) 2026 Lev Burmistrov (Mahiron-hq)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3 of the License.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! `.sav` container: Base64 of AES-256-CBC (PKCS7) encrypted UTF-8 JSON.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::Engine;

use crate::json::{self, ParseError, Value};

const KEY: [u8; 32] = [
    0xa7, 0xca, 0x9f, 0x33, 0x66, 0xd8, 0x92, 0xc2, 0xf0, 0xbe, 0xf4, 0x17, 0x34, 0x1c, 0xa9, 0x71, 0xb6, 0x9a, 0xe9,
    0xf7, 0xba, 0xcc, 0xcf, 0xfc, 0xf4, 0x3c, 0x62, 0xd1, 0xd7, 0xd0, 0x21, 0xf9,
];
const IV: [u8; 16] = *b"tu89geji340t89u2";

/// Largest input accepted, in bytes.
pub const MAX_FILE_SIZE: usize = 32 * 1024 * 1024;

type Enc = cbc::Encryptor<aes::Aes256>;
type Dec = cbc::Decryptor<aes::Aes256>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatError {
    TooLarge,
    Empty,
    NotBase64,
    BadLength,
    BadPadding,
    NotUtf8,
    NotJson(ParseError),
    NotObject,
}

impl FormatError {
    pub fn code(&self) -> &'static str {
        match self {
            FormatError::TooLarge => "too_large",
            FormatError::Empty => "empty",
            FormatError::NotBase64 => "not_base64",
            FormatError::BadLength => "bad_length",
            FormatError::BadPadding => "bad_padding",
            FormatError::NotUtf8 => "not_utf8",
            FormatError::NotJson(_) => "not_json",
            FormatError::NotObject => "not_object",
        }
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::NotJson(e) => write!(f, "not_json: {e}"),
            other => f.write_str(other.code()),
        }
    }
}

impl std::error::Error for FormatError {}

pub fn decrypt_text(file: &[u8]) -> Result<String, FormatError> {
    if file.len() > MAX_FILE_SIZE {
        return Err(FormatError::TooLarge);
    }
    let trimmed = trim_ascii(file);
    if trimmed.is_empty() {
        return Err(FormatError::Empty);
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(trimmed).map_err(|_| FormatError::NotBase64)?;
    if bytes.is_empty() || bytes.len() % 16 != 0 {
        return Err(FormatError::BadLength);
    }
    let plain = Dec::new(&KEY.into(), &IV.into())
        .decrypt_padded_vec_mut::<Pkcs7>(&bytes)
        .map_err(|_| FormatError::BadPadding)?;
    String::from_utf8(plain).map_err(|_| FormatError::NotUtf8)
}

pub fn encrypt_text(json_text: &str) -> String {
    let cipher = Enc::new(&KEY.into(), &IV.into()).encrypt_padded_vec_mut::<Pkcs7>(json_text.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(cipher)
}

/// Decrypts and parses a `.sav` file. The root must be an object.
pub fn read_sav(file: &[u8]) -> Result<Value, FormatError> {
    let text = decrypt_text(file)?;
    parse_root(&text)
}

/// Parses plain JSON (import). A UTF-8 BOM is ignored.
pub fn read_json(file: &[u8]) -> Result<Value, FormatError> {
    if file.len() > MAX_FILE_SIZE {
        return Err(FormatError::TooLarge);
    }
    let file = file.strip_prefix(b"\xef\xbb\xbf").unwrap_or(file);
    let text = std::str::from_utf8(file).map_err(|_| FormatError::NotUtf8)?;
    if text.trim().is_empty() {
        return Err(FormatError::Empty);
    }
    parse_root(text)
}

fn parse_root(text: &str) -> Result<Value, FormatError> {
    let v = json::parse(text).map_err(FormatError::NotJson)?;
    if v.as_object().is_none() {
        return Err(FormatError::NotObject);
    }
    Ok(v)
}

pub fn write_sav(root: &Value) -> String {
    encrypt_text(&root.to_compact())
}

/// Guesses whether bytes are a `.sav` (Base64) or plain JSON.
pub fn looks_like_json(file: &[u8]) -> bool {
    let t = trim_ascii(file.strip_prefix(b"\xef\xbb\xbf").unwrap_or(file));
    matches!(t.first(), Some(b'{' | b'['))
}

fn trim_ascii(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(b.len());
    let end = b.iter().rposition(|c| !c.is_ascii_whitespace()).map_or(start, |p| p + 1);
    &b[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector() {
        assert_eq!(encrypt_text(r#"{"a":1}"#), "u9bcSUF3rVpSfbPAFIiDjw==");
        assert_eq!(decrypt_text(b"u9bcSUF3rVpSfbPAFIiDjw==").unwrap(), r#"{"a":1}"#);
        assert_eq!(decrypt_text(b"  u9bcSUF3rVpSfbPAFIiDjw==\r\n").unwrap(), r#"{"a":1}"#);
    }

    #[test]
    fn error_per_step() {
        assert_eq!(decrypt_text(b"   ").unwrap_err(), FormatError::Empty);
        assert_eq!(decrypt_text(b"not base64!").unwrap_err(), FormatError::NotBase64);
        assert_eq!(decrypt_text(b"AAAA").unwrap_err(), FormatError::BadLength);
        assert_eq!(decrypt_text(b"AAAAAAAAAAAAAAAAAAAAAA==").unwrap_err(), FormatError::BadPadding);
        let not_utf8 = base64::engine::general_purpose::STANDARD
            .encode(Enc::new(&KEY.into(), &IV.into()).encrypt_padded_vec_mut::<Pkcs7>(&[0xff, 0xfe]));
        assert_eq!(decrypt_text(not_utf8.as_bytes()).unwrap_err(), FormatError::NotUtf8);
        assert!(matches!(read_sav(encrypt_text("{oops").as_bytes()).unwrap_err(), FormatError::NotJson(_)));
        assert_eq!(read_sav(encrypt_text("[1]").as_bytes()).unwrap_err(), FormatError::NotObject);
    }

    #[test]
    fn json_detection() {
        assert!(looks_like_json(b"\xef\xbb\xbf {\"a\":1}"));
        assert!(!looks_like_json(b"u9bcSUF3rVpSfbPAFIiDjw=="));
    }
}
