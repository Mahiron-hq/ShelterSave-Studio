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

//! Lossless JSON tree: numbers and strings keep their exact source text,
//! objects keep key order and duplicate keys.

use std::fmt;

/// Maximum nesting depth accepted by the parser.
pub const MAX_DEPTH: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(JsonStr),
    Array(Vec<Value>),
    Object(Object),
}

/// A number stored as its literal text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Number(Box<str>);

/// A string stored as its escaped body (without quotes).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JsonStr(Box<str>);

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Object(pub Vec<(JsonStr, Value)>);

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Seg {
    Key(String),
    Index(usize),
}

pub type Path = Vec<Seg>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedEnd,
    UnexpectedChar,
    BadNumber,
    BadEscape,
    ControlCharInString,
    TooDeep,
    TrailingData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} at line {}, column {}",
            self.kind, self.line, self.column
        )
    }
}

impl std::error::Error for ParseError {}

impl Number {
    /// Accepts a strict JSON number literal, plus `NaN`, `Infinity` and
    /// `-Infinity`, which .NET can write for non-finite floats.
    pub fn from_literal(s: &str) -> Option<Number> {
        if is_number_literal(s) || matches!(s, "NaN" | "Infinity" | "-Infinity") {
            Some(Number(s.into()))
        } else {
            None
        }
    }

    pub fn from_i64(v: i64) -> Number {
        Number(v.to_string().into())
    }

    /// Fixed two decimals, the format the game uses for floats.
    pub fn from_f64_fixed2(v: f64) -> Number {
        if !v.is_finite() {
            return Number("0.00".into());
        }
        let s = format!("{v:.2}");
        if s == "-0.00" {
            Number("0.00".into())
        } else {
            Number(s.into())
        }
    }

    /// Builds a number for `v` in the same style as `self`: integer literals
    /// stay integers when `v` is integral, decimal literals keep two decimals.
    pub fn restyle(&self, v: f64) -> Number {
        if self.is_integer_literal() && v.fract() == 0.0 && v.abs() < 9.0e18 {
            Number::from_i64(v as i64)
        } else {
            Number::from_f64_fixed2(v)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_integer_literal(&self) -> bool {
        !self.0.contains(['.', 'e', 'E']) && !matches!(&*self.0, "NaN" | "Infinity" | "-Infinity")
    }

    pub fn is_finite_literal(&self) -> bool {
        !matches!(&*self.0, "NaN" | "Infinity" | "-Infinity")
    }

    pub fn as_i64(&self) -> Option<i64> {
        if self.is_integer_literal() {
            self.0.parse().ok()
        } else {
            None
        }
    }

    pub fn as_f64(&self) -> f64 {
        match &*self.0 {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            s => s.parse().unwrap_or(0.0),
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_number_literal(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    if b.get(i) == Some(&b'-') {
        i += 1;
    }
    match b.get(i) {
        Some(b'0') => i += 1,
        Some(b'1'..=b'9') => {
            while matches!(b.get(i), Some(b'0'..=b'9')) {
                i += 1;
            }
        }
        _ => return false,
    }
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while matches!(b.get(i), Some(b'0'..=b'9')) {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while matches!(b.get(i), Some(b'0'..=b'9')) {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    i == b.len()
}

impl JsonStr {
    /// Encodes text the way the game does: printable ASCII as is, everything
    /// else as lowercase `\uXXXX` UTF-16 units.
    pub fn new(text: &str) -> JsonStr {
        let mut out = String::with_capacity(text.len() + 2);
        for c in text.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                ' '..='~' => out.push(c),
                _ => {
                    let mut buf = [0u16; 2];
                    for unit in c.encode_utf16(&mut buf) {
                        out.push_str(&format!("\\u{:04x}", unit));
                    }
                }
            }
        }
        JsonStr(out.into())
    }

    pub fn raw(&self) -> &str {
        &self.0
    }

    /// Decoded text. Lone surrogates become U+FFFD.
    pub fn text(&self) -> String {
        if !self.0.contains('\\') {
            return self.0.to_string();
        }
        decode_escapes(&self.0)
    }

    pub fn eq_text(&self, s: &str) -> bool {
        if !self.0.contains('\\') {
            &*self.0 == s
        } else {
            self.text() == s
        }
    }
}

fn decode_escapes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut units: Vec<u16> = Vec::new();
    let flush = |units: &mut Vec<u16>, out: &mut String| {
        if !units.is_empty() {
            out.extend(char::decode_utf16(units.iter().copied()).map(|r| r.unwrap_or('\u{fffd}')));
            units.clear();
        }
    };
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            flush(&mut units, &mut out);
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                match u16::from_str_radix(&hex, 16) {
                    Ok(u) => units.push(u),
                    Err(_) => {
                        flush(&mut units, &mut out);
                        out.push('\u{fffd}');
                    }
                }
            }
            Some(e) => {
                flush(&mut units, &mut out);
                out.push(match e {
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
            }
            None => {}
        }
    }
    flush(&mut units, &mut out);
    out
}

impl Object {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k.eq_text(key)).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0
            .iter_mut()
            .find(|(k, _)| k.eq_text(key))
            .map(|(_, v)| v)
    }

    pub fn index_of(&self, key: &str) -> Option<usize> {
        self.0.iter().position(|(k, _)| k.eq_text(key))
    }

    pub fn keys(&self) -> impl Iterator<Item = String> + '_ {
        self.0.iter().map(|(k, _)| k.text())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::String(JsonStr::new(s))
    }

    pub fn int(v: i64) -> Value {
        Value::Number(Number::from_i64(v))
    }

    pub fn float2(v: f64) -> Value {
        Value::Number(Number::from_f64_fixed2(v))
    }

    pub fn obj(entries: Vec<(&str, Value)>) -> Value {
        Value::Object(Object(
            entries
                .into_iter()
                .map(|(k, v)| (JsonStr::new(k), v))
                .collect(),
        ))
    }

    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Object> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.as_i64().or_else(|| {
                let f = n.as_f64();
                (f.is_finite() && f.fract() == 0.0 && f.abs() < 9.0e18).then_some(f as i64)
            }),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(n.as_f64()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.text()),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object().and_then(|o| o.get(key))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.as_object_mut().and_then(|o| o.get_mut(key))
    }

    pub fn at(&self, path: &[Seg]) -> Option<&Value> {
        let mut cur = self;
        for seg in path {
            cur = match (seg, cur) {
                (Seg::Key(k), Value::Object(o)) => o.get(k)?,
                (Seg::Index(i), Value::Array(a)) => a.get(*i)?,
                (Seg::Index(i), Value::Object(o)) => &o.0.get(*i)?.1,
                _ => return None,
            };
        }
        Some(cur)
    }

    pub fn at_mut(&mut self, path: &[Seg]) -> Option<&mut Value> {
        let mut cur = self;
        for seg in path {
            cur = match (seg, cur) {
                (Seg::Key(k), Value::Object(o)) => o.get_mut(k)?,
                (Seg::Index(i), Value::Array(a)) => a.get_mut(*i)?,
                (Seg::Index(i), Value::Object(o)) => &mut o.0.get_mut(*i)?.1,
                _ => return None,
            };
        }
        Some(cur)
    }

    /// Number of nodes in the tree, including this one.
    pub fn node_count(&self) -> usize {
        match self {
            Value::Array(a) => 1 + a.iter().map(Value::node_count).sum::<usize>(),
            Value::Object(o) => 1 + o.0.iter().map(|(_, v)| v.node_count()).sum::<usize>(),
            _ => 1,
        }
    }

    pub fn to_compact(&self) -> String {
        let mut out = String::new();
        write_compact(self, &mut out);
        out
    }

    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        write_pretty(self, &mut out, 0);
        out.push('\n');
        out
    }
}

pub fn path_to_string(path: &[Seg]) -> String {
    let mut s = String::new();
    for seg in path {
        match seg {
            Seg::Key(k) => {
                if !s.is_empty() {
                    s.push('.');
                }
                s.push_str(k);
            }
            Seg::Index(i) => {
                s.push('[');
                s.push_str(&i.to_string());
                s.push(']');
            }
        }
    }
    s
}

fn write_compact(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(&n.0),
        Value::String(s) => {
            out.push('"');
            out.push_str(&s.0);
            out.push('"');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_compact(item, out);
            }
            out.push(']');
        }
        Value::Object(o) => {
            out.push('{');
            for (i, (k, item)) in o.0.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                out.push_str(&k.0);
                out.push_str("\":");
                write_compact(item, out);
            }
            out.push('}');
        }
    }
}

fn write_pretty(v: &Value, out: &mut String, indent: usize) {
    let pad = |out: &mut String, n: usize| out.extend(std::iter::repeat_n(' ', n * 2));
    match v {
        Value::Array(a) if !a.is_empty() => {
            out.push_str("[\n");
            for (i, item) in a.iter().enumerate() {
                pad(out, indent + 1);
                write_pretty(item, out, indent + 1);
                if i + 1 < a.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(out, indent);
            out.push(']');
        }
        Value::Object(o) if !o.0.is_empty() => {
            out.push_str("{\n");
            for (i, (k, item)) in o.0.iter().enumerate() {
                pad(out, indent + 1);
                out.push('"');
                out.push_str(&k.0);
                out.push_str("\": ");
                write_pretty(item, out, indent + 1);
                if i + 1 < o.0.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(out, indent);
            out.push('}');
        }
        other => write_compact(other, out),
    }
}

pub fn parse(text: &str) -> Result<Value, ParseError> {
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(p.err(ParseErrorKind::TrailingData));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, kind: ParseErrorKind) -> ParseError {
        let upto = &self.s[..self.i.min(self.s.len())];
        let line = upto.iter().filter(|&&b| b == b'\n').count() + 1;
        let line_start = upto.iter().rposition(|&b| b == b'\n').map_or(0, |p| p + 1);
        let column = String::from_utf8_lossy(&upto[line_start..]).chars().count() + 1;
        ParseError { kind, line, column }
    }

    fn ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.s.get(self.i) {
            self.i += 1;
        }
    }

    fn peek(&self) -> Result<u8, ParseError> {
        self.s
            .get(self.i)
            .copied()
            .ok_or_else(|| self.err(ParseErrorKind::UnexpectedEnd))
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth >= MAX_DEPTH {
            return Err(self.err(ParseErrorKind::TooDeep));
        }
        match self.peek()? {
            b'{' => self.object(depth),
            b'[' => self.array(depth),
            b'"' => Ok(Value::String(self.string()?)),
            b't' => self.word(b"true", Value::Bool(true)),
            b'f' => self.word(b"false", Value::Bool(false)),
            b'n' => self.word(b"null", Value::Null),
            b'-' | b'0'..=b'9' | b'N' | b'I' => self.number(),
            _ => Err(self.err(ParseErrorKind::UnexpectedChar)),
        }
    }

    fn word(&mut self, w: &[u8], v: Value) -> Result<Value, ParseError> {
        if self.s[self.i..].starts_with(w) {
            self.i += w.len();
            Ok(v)
        } else {
            Err(self.err(ParseErrorKind::UnexpectedChar))
        }
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        for special in [&b"-Infinity"[..], b"Infinity", b"NaN"] {
            if self.s[self.i..].starts_with(special) {
                self.i += special.len();
                let lit = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("NaN");
                return Ok(Value::Number(Number(lit.into())));
            }
        }
        while let Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') = self.s.get(self.i) {
            self.i += 1;
        }
        let lit = std::str::from_utf8(&self.s[start..self.i])
            .map_err(|_| self.err(ParseErrorKind::BadNumber))?;
        if !is_number_literal(lit) {
            self.i = start;
            return Err(self.err(ParseErrorKind::BadNumber));
        }
        Ok(Value::Number(Number(lit.into())))
    }

    fn string(&mut self) -> Result<JsonStr, ParseError> {
        self.i += 1;
        let start = self.i;
        loop {
            let b = self.peek()?;
            match b {
                b'"' => break,
                b'\\' => {
                    self.i += 1;
                    match self.peek()? {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => self.i += 1,
                        b'u' => {
                            self.i += 1;
                            for _ in 0..4 {
                                if !self.peek()?.is_ascii_hexdigit() {
                                    return Err(self.err(ParseErrorKind::BadEscape));
                                }
                                self.i += 1;
                            }
                        }
                        _ => return Err(self.err(ParseErrorKind::BadEscape)),
                    }
                }
                0..=0x1f => return Err(self.err(ParseErrorKind::ControlCharInString)),
                _ => self.i += 1,
            }
        }
        let raw = std::str::from_utf8(&self.s[start..self.i])
            .map_err(|_| self.err(ParseErrorKind::UnexpectedChar))?;
        self.i += 1;
        Ok(JsonStr(raw.into()))
    }

    fn array(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.peek()? == b']' {
            self.i += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.ws();
            items.push(self.value(depth + 1)?);
            self.ws();
            match self.peek()? {
                b',' => self.i += 1,
                b']' => {
                    self.i += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.err(ParseErrorKind::UnexpectedChar)),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.i += 1;
        let mut entries = Vec::new();
        self.ws();
        if self.peek()? == b'}' {
            self.i += 1;
            return Ok(Value::Object(Object(entries)));
        }
        loop {
            self.ws();
            if self.peek()? != b'"' {
                return Err(self.err(ParseErrorKind::UnexpectedChar));
            }
            let key = self.string()?;
            self.ws();
            if self.peek()? != b':' {
                return Err(self.err(ParseErrorKind::UnexpectedChar));
            }
            self.i += 1;
            self.ws();
            let v = self.value(depth + 1)?;
            entries.push((key, v));
            self.ws();
            match self.peek()? {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    return Ok(Value::Object(Object(entries)));
                }
                _ => return Err(self.err(ParseErrorKind::UnexpectedChar)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_literals() {
        let src = r#"{"a":35002.00,"t":636820115098349470,"s":" 2.7.0","u":"\u00e9x","n":null,"e":[],"o":{},"d":1,"d":2}"#;
        let v = parse(src).unwrap();
        assert_eq!(v.to_compact(), src);
    }

    #[test]
    fn whitespace_is_accepted_and_removed() {
        let v = parse(" {\n \"a\" : [ 1 , 2.50 ] }\r\n").unwrap();
        assert_eq!(v.to_compact(), r#"{"a":[1,2.50]}"#);
    }

    #[test]
    fn pretty_reparses_to_same_tree() {
        let src = r#"{"a":[1,{"b":"c"}],"e":{},"f":[]}"#;
        let v = parse(src).unwrap();
        assert_eq!(parse(&v.to_pretty()).unwrap(), v);
    }

    #[test]
    fn rejects_bad_input() {
        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\" 1}",
            "01",
            "1.",
            "\"\\x\"",
            "\"a\nb\"",
            "{} x",
            "tru",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn depth_limit() {
        let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert_eq!(parse(&deep).unwrap_err().kind, ParseErrorKind::TooDeep);
    }

    #[test]
    fn string_encoding_matches_game() {
        let s = JsonStr::new("Убежище \"1\"\n😀");
        assert_eq!(
            s.raw(),
            "\\u0423\\u0431\\u0435\\u0436\\u0438\\u0449\\u0435 \\\"1\\\"\\n\\ud83d\\ude00"
        );
        assert_eq!(s.text(), "Убежище \"1\"\n😀");
    }

    #[test]
    fn number_styles() {
        let f = Number::from_literal("3248.14").unwrap();
        assert_eq!(f.restyle(5000.0).as_str(), "5000.00");
        let i = Number::from_literal("15").unwrap();
        assert_eq!(i.restyle(20.0).as_str(), "20");
        assert_eq!(i.restyle(2.5).as_str(), "2.50");
        assert_eq!(
            Number::from_literal("636820115098349470").unwrap().as_i64(),
            Some(636820115098349470)
        );
        assert!(Number::from_literal("1e5").is_some());
        assert!(Number::from_literal("+1").is_none());
    }

    #[test]
    fn non_finite_literals_survive() {
        let v = parse("[NaN,-Infinity,Infinity]").unwrap();
        assert_eq!(v.to_compact(), "[NaN,-Infinity,Infinity]");
    }
}
