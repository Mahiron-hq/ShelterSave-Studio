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

//! Differences between the file as opened and the current state.
//! Arrays of dwellers, actors and rooms are matched by their ids, so
//! deleting one record does not show every later record as changed.

use serde::Serialize;

use crate::json::{Seg, Value};

const ID_KEYS: [&str; 2] = ["serializeId", "deserializeID"];

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Changed,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub kind: ChangeKind,
    /// Path in the current tree (for removed values: in the original).
    pub path: Vec<Seg>,
    pub before: Option<String>,
    pub after: Option<String>,
}

const PREVIEW: usize = 120;

fn preview(v: &Value) -> String {
    let s = match v {
        Value::String(s) => format!("\"{}\"", s.text()),
        Value::Array(a) if a.len() > 8 => format!("[{} items]", a.len()),
        Value::Object(o) if o.len() > 4 => format!("{{{} keys}}", o.len()),
        other => other.to_compact(),
    };
    if s.chars().count() > PREVIEW {
        let cut: String = s.chars().take(PREVIEW).collect();
        format!("{cut}…")
    } else {
        s
    }
}

fn identity(v: &Value) -> Option<(usize, i64)> {
    let o = v.as_object()?;
    ID_KEYS
        .iter()
        .enumerate()
        .find_map(|(i, k)| o.get(k).and_then(Value::as_i64).map(|id| (i, id)))
}

fn keyed(a: &[Value]) -> bool {
    !a.is_empty() && a.iter().all(|v| identity(v).is_some())
}

pub fn diff(original: &Value, current: &Value) -> Vec<Change> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    walk(original, current, &mut path, &mut out);
    out
}

fn walk(a: &Value, b: &Value, path: &mut Vec<Seg>, out: &mut Vec<Change>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(oa), Value::Object(ob)) => {
            for (k, vb) in &ob.0 {
                let key = k.text();
                path.push(Seg::Key(key.clone()));
                match oa.get(&key) {
                    Some(va) => walk(va, vb, path, out),
                    None => out.push(Change {
                        kind: ChangeKind::Added,
                        path: path.clone(),
                        before: None,
                        after: Some(preview(vb)),
                    }),
                }
                path.pop();
            }
            for (k, va) in &oa.0 {
                let key = k.text();
                if ob.get(&key).is_none() {
                    path.push(Seg::Key(key));
                    out.push(Change {
                        kind: ChangeKind::Removed,
                        path: path.clone(),
                        before: Some(preview(va)),
                        after: None,
                    });
                    path.pop();
                }
            }
        }
        (Value::Array(xa), Value::Array(xb)) if keyed(xa) || keyed(xb) => {
            for (i, vb) in xb.iter().enumerate() {
                path.push(Seg::Index(i));
                match identity(vb).and_then(|id| xa.iter().find(|va| identity(va) == Some(id))) {
                    Some(va) => walk(va, vb, path, out),
                    None => out.push(Change {
                        kind: ChangeKind::Added,
                        path: path.clone(),
                        before: None,
                        after: Some(preview(vb)),
                    }),
                }
                path.pop();
            }
            for (i, va) in xa.iter().enumerate() {
                let id = identity(va);
                if id.is_none() || !xb.iter().any(|vb| identity(vb) == id) {
                    path.push(Seg::Index(i));
                    out.push(Change {
                        kind: ChangeKind::Removed,
                        path: path.clone(),
                        before: Some(preview(va)),
                        after: None,
                    });
                    path.pop();
                }
            }
        }
        (Value::Array(xa), Value::Array(xb)) => {
            let common = xa.len().min(xb.len());
            for i in 0..common {
                path.push(Seg::Index(i));
                walk(&xa[i], &xb[i], path, out);
                path.pop();
            }
            for (i, vb) in xb.iter().enumerate().skip(common) {
                path.push(Seg::Index(i));
                out.push(Change {
                    kind: ChangeKind::Added,
                    path: path.clone(),
                    before: None,
                    after: Some(preview(vb)),
                });
                path.pop();
            }
            for (i, va) in xa.iter().enumerate().skip(common) {
                path.push(Seg::Index(i));
                out.push(Change {
                    kind: ChangeKind::Removed,
                    path: path.clone(),
                    before: Some(preview(va)),
                    after: None,
                });
                path.pop();
            }
        }
        _ => out.push(Change {
            kind: ChangeKind::Changed,
            path: path.clone(),
            before: Some(preview(a)),
            after: Some(preview(b)),
        }),
    }
}

/// Finds the original counterpart of a path in the current tree.
pub fn original_at<'a>(original: &'a Value, current: &Value, path: &[Seg]) -> Option<&'a Value> {
    let mut o = original;
    let mut c = current;
    for seg in path {
        match (seg, o, c) {
            (Seg::Key(k), Value::Object(oo), Value::Object(co)) => {
                o = oo.get(k)?;
                c = co.get(k)?;
            }
            (Seg::Index(i), Value::Array(oa), Value::Array(ca)) => {
                let cv = ca.get(*i)?;
                o = match identity(cv) {
                    Some(id) if keyed(oa) => oa.iter().find(|v| identity(v) == Some(id))?,
                    _ => oa.get(*i)?,
                };
                c = cv;
            }
            _ => return None,
        }
    }
    Some(o)
}

/// True when the value at `path` differs from the file as opened.
pub fn is_modified(original: &Value, current: &Value, path: &[Seg]) -> bool {
    match (original_at(original, current, path), current.at(path)) {
        (Some(o), Some(c)) => o != c,
        (None, Some(_)) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn keyed_arrays_match_by_id() {
        let a = parse(
            r#"{"d":[{"serializeId":1,"n":1},{"serializeId":2,"n":2},{"serializeId":3,"n":3}]}"#,
        )
        .unwrap();
        let b = parse(r#"{"d":[{"serializeId":1,"n":1},{"serializeId":3,"n":30}]}"#).unwrap();
        let ch = diff(&a, &b);
        assert_eq!(ch.len(), 2);
        assert!(ch.iter().any(|c| c.kind == ChangeKind::Changed
            && c.path == vec![Seg::Key("d".into()), Seg::Index(1), Seg::Key("n".into())]));
        assert!(ch.iter().any(|c| c.kind == ChangeKind::Removed));
        let p = vec![Seg::Key("d".into()), Seg::Index(1), Seg::Key("n".into())];
        assert_eq!(original_at(&a, &b, &p).and_then(Value::as_i64), Some(3));
        assert!(is_modified(&a, &b, &p));
        assert!(!is_modified(&a, &b, &[Seg::Key("d".into()), Seg::Index(0)]));
    }
}
