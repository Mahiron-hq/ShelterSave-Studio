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

//! Primitive edit operations, transactions and undo history.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::json::{self, JsonStr, Seg, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Op {
    Set {
        path: Vec<Seg>,
        #[serde(with = "raw")]
        old: Value,
        #[serde(with = "raw")]
        new: Value,
    },
    Insert {
        parent: Vec<Seg>,
        index: usize,
        key: Option<String>,
        #[serde(with = "raw")]
        value: Value,
    },
    Remove {
        parent: Vec<Seg>,
        index: usize,
        key: Option<String>,
        #[serde(with = "raw")]
        value: Value,
    },
}

/// Message key and parameters describing an action for the UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub key: String,
    #[serde(default)]
    pub params: Vec<(String, String)>,
}

impl Label {
    pub fn new(key: &str) -> Label {
        Label {
            key: key.to_string(),
            params: Vec::new(),
        }
    }

    pub fn with(mut self, name: &str, value: impl ToString) -> Label {
        self.params.push((name.to_string(), value.to_string()));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub label: Label,
    pub ops: Vec<Op>,
    /// Records (dwellers, items, rooms...) touched by the action.
    pub affected: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    PathNotFound(String),
    WrongContainer(String),
    IndexOutOfRange(String),
    Mismatch(String),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::PathNotFound(p) => write!(f, "path not found: {p}"),
            EditError::WrongContainer(p) => write!(f, "not a container: {p}"),
            EditError::IndexOutOfRange(p) => write!(f, "index out of range: {p}"),
            EditError::Mismatch(p) => write!(f, "value changed under edit: {p}"),
        }
    }
}

impl std::error::Error for EditError {}

impl Op {
    pub fn inverse(&self) -> Op {
        match self.clone() {
            Op::Set { path, old, new } => Op::Set {
                path,
                old: new,
                new: old,
            },
            Op::Insert {
                parent,
                index,
                key,
                value,
            } => Op::Remove {
                parent,
                index,
                key,
                value,
            },
            Op::Remove {
                parent,
                index,
                key,
                value,
            } => Op::Insert {
                parent,
                index,
                key,
                value,
            },
        }
    }

    pub fn apply(&self, root: &mut Value) -> Result<(), EditError> {
        match self {
            Op::Set { path, old, new } => {
                let target = root
                    .at_mut(path)
                    .ok_or_else(|| EditError::PathNotFound(json::path_to_string(path)))?;
                if target != old {
                    return Err(EditError::Mismatch(json::path_to_string(path)));
                }
                *target = new.clone();
            }
            Op::Insert {
                parent,
                index,
                key,
                value,
            } => {
                let p = json::path_to_string(parent);
                match (root.at_mut(parent), key) {
                    (Some(Value::Array(a)), None) if *index <= a.len() => {
                        a.insert(*index, value.clone())
                    }
                    (Some(Value::Object(o)), Some(k)) if *index <= o.0.len() => {
                        o.0.insert(*index, (JsonStr::new(k), value.clone()))
                    }
                    (Some(Value::Array(_) | Value::Object(_)), _) => {
                        return Err(EditError::IndexOutOfRange(p))
                    }
                    (Some(_), _) => return Err(EditError::WrongContainer(p)),
                    (None, _) => return Err(EditError::PathNotFound(p)),
                }
            }
            Op::Remove {
                parent,
                index,
                key,
                value,
            } => {
                let p = json::path_to_string(parent);
                match (root.at_mut(parent), key) {
                    (Some(Value::Array(a)), None) if *index < a.len() => {
                        if &a[*index] != value {
                            return Err(EditError::Mismatch(p));
                        }
                        a.remove(*index);
                    }
                    (Some(Value::Object(o)), Some(k)) if *index < o.0.len() => {
                        if !o.0[*index].0.eq_text(k) || &o.0[*index].1 != value {
                            return Err(EditError::Mismatch(p));
                        }
                        o.0.remove(*index);
                    }
                    (Some(Value::Array(_) | Value::Object(_)), _) => {
                        return Err(EditError::IndexOutOfRange(p))
                    }
                    (Some(_), _) => return Err(EditError::WrongContainer(p)),
                    (None, _) => return Err(EditError::PathNotFound(p)),
                }
            }
        }
        Ok(())
    }
}

/// Applies all ops; on failure rolls back the ones already applied.
pub fn apply_all(root: &mut Value, ops: &[Op]) -> Result<(), EditError> {
    for (i, op) in ops.iter().enumerate() {
        if let Err(e) = op.apply(root) {
            for done in ops[..i].iter().rev() {
                let _ = done.inverse().apply(root);
            }
            return Err(e);
        }
    }
    Ok(())
}

pub fn revert_all(root: &mut Value, ops: &[Op]) -> Result<(), EditError> {
    let inv: Vec<Op> = ops.iter().rev().map(Op::inverse).collect();
    apply_all(root, &inv)
}

/// Collects ops while mutating a working tree, so later steps of an action
/// see the results of earlier ones.
pub struct Builder<'a> {
    root: &'a mut Value,
    ops: Vec<Op>,
}

impl<'a> Builder<'a> {
    pub fn new(root: &'a mut Value) -> Builder<'a> {
        Builder {
            root,
            ops: Vec::new(),
        }
    }

    pub fn root(&self) -> &Value {
        self.root
    }

    pub fn get(&self, path: &[Seg]) -> Option<&Value> {
        self.root.at(path)
    }

    pub fn set(&mut self, path: Vec<Seg>, new: Value) -> Result<bool, EditError> {
        let old = self
            .root
            .at(&path)
            .cloned()
            .ok_or_else(|| EditError::PathNotFound(json::path_to_string(&path)))?;
        if old == new {
            return Ok(false);
        }
        let op = Op::Set { path, old, new };
        op.apply(self.root)?;
        self.ops.push(op);
        Ok(true)
    }

    pub fn insert(
        &mut self,
        parent: Vec<Seg>,
        index: usize,
        key: Option<&str>,
        value: Value,
    ) -> Result<(), EditError> {
        let op = Op::Insert {
            parent,
            index,
            key: key.map(str::to_string),
            value,
        };
        op.apply(self.root)?;
        self.ops.push(op);
        Ok(())
    }

    pub fn push(&mut self, parent: Vec<Seg>, value: Value) -> Result<(), EditError> {
        let len = match self.root.at(&parent) {
            Some(Value::Array(a)) => a.len(),
            Some(_) => return Err(EditError::WrongContainer(json::path_to_string(&parent))),
            None => return Err(EditError::PathNotFound(json::path_to_string(&parent))),
        };
        self.insert(parent, len, None, value)
    }

    pub fn remove(&mut self, parent: Vec<Seg>, index: usize) -> Result<Value, EditError> {
        let (key, value) = match self.root.at(&parent) {
            Some(Value::Array(a)) => (None, a.get(index).cloned()),
            Some(Value::Object(o)) => {
                o.0.get(index)
                    .map_or((None, None), |(k, v)| (Some(k.text()), Some(v.clone())))
            }
            Some(_) => return Err(EditError::WrongContainer(json::path_to_string(&parent))),
            None => return Err(EditError::PathNotFound(json::path_to_string(&parent))),
        };
        let value =
            value.ok_or_else(|| EditError::IndexOutOfRange(json::path_to_string(&parent)))?;
        let op = Op::Remove {
            parent,
            index,
            key,
            value: value.clone(),
        };
        op.apply(self.root)?;
        self.ops.push(op);
        Ok(value)
    }

    /// Sets an object key, adding it at the end when missing.
    pub fn set_key(&mut self, obj: Vec<Seg>, key: &str, value: Value) -> Result<bool, EditError> {
        let o = self
            .root
            .at(&obj)
            .and_then(Value::as_object)
            .ok_or_else(|| EditError::WrongContainer(json::path_to_string(&obj)))?;
        match o.index_of(key) {
            Some(_) => {
                let mut p = obj;
                p.push(Seg::Key(key.to_string()));
                self.set(p, value)
            }
            None => {
                let len = o.len();
                self.insert(obj, len, Some(key), value)?;
                Ok(true)
            }
        }
    }

    pub fn remove_key(&mut self, obj: Vec<Seg>, key: &str) -> Result<bool, EditError> {
        let idx = self
            .root
            .at(&obj)
            .and_then(Value::as_object)
            .and_then(|o| o.index_of(key));
        match idx {
            Some(i) => {
                self.remove(obj, i)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.ops.is_empty()
    }

    /// Ends the action. The working tree stays modified; the ops describe it.
    pub fn finish(self) -> Vec<Op> {
        self.ops
    }

    /// Drops the action and restores the tree.
    pub fn abort(self) {
        let _ = revert_all(self.root, &self.ops);
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct History {
    done: Vec<Transaction>,
    undone: Vec<Transaction>,
}

impl History {
    pub fn record(&mut self, tx: Transaction) {
        self.done.push(tx);
        self.undone.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    pub fn undo(&mut self, root: &mut Value) -> Result<Option<Label>, EditError> {
        let Some(tx) = self.done.pop() else {
            return Ok(None);
        };
        if let Err(e) = revert_all(root, &tx.ops) {
            self.done.push(tx);
            return Err(e);
        }
        let label = tx.label.clone();
        self.undone.push(tx);
        Ok(Some(label))
    }

    pub fn redo(&mut self, root: &mut Value) -> Result<Option<Label>, EditError> {
        let Some(tx) = self.undone.pop() else {
            return Ok(None);
        };
        if let Err(e) = apply_all(root, &tx.ops) {
            self.undone.push(tx);
            return Err(e);
        }
        let label = tx.label.clone();
        self.done.push(tx);
        Ok(Some(label))
    }

    /// Steps applied so far, oldest first.
    pub fn done(&self) -> &[Transaction] {
        &self.done
    }

    /// Steps that can be redone, next first.
    pub fn undone(&self) -> impl Iterator<Item = &Transaction> {
        self.undone.iter().rev()
    }

    /// Moves to the state after `n` applied steps (0 = file as opened).
    pub fn go_to(&mut self, root: &mut Value, n: usize) -> Result<(), EditError> {
        while self.done.len() > n {
            self.undo(root)?;
        }
        while self.done.len() < n && self.can_redo() {
            self.redo(root)?;
        }
        Ok(())
    }
}

mod raw {
    use super::*;

    pub fn serialize<S: Serializer>(v: &Value, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_compact())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Value, D::Error> {
        let text = String::deserialize(d)?;
        json::parse(&text).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Seg {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Seg::Key(k) => s.serialize_str(k),
            Seg::Index(i) => s.serialize_u64(*i as u64),
        }
    }
}

impl<'de> Deserialize<'de> for Seg {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Seg, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            K(String),
            I(u64),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::K(k) => Seg::Key(k),
            Raw::I(i) => Seg::Index(i as usize),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str) -> Seg {
        Seg::Key(s.into())
    }

    #[test]
    fn undo_redo_restores_exact_text() {
        let src = r#"{"a":[1,2,3],"b":{"x":1.50}}"#;
        let mut root = json::parse(src).unwrap();
        let mut h = History::default();
        let mut b = Builder::new(&mut root);
        b.set(vec![k("b"), k("x")], Value::float2(2.0)).unwrap();
        b.remove(vec![k("a")], 0).unwrap();
        b.set_key(vec![k("b")], "y", Value::int(7)).unwrap();
        let ops = b.finish();
        h.record(Transaction {
            label: Label::new("t"),
            ops,
            affected: 1,
        });
        assert_eq!(root.to_compact(), r#"{"a":[2,3],"b":{"x":2.00,"y":7}}"#);
        h.undo(&mut root).unwrap();
        assert_eq!(root.to_compact(), src);
        h.redo(&mut root).unwrap();
        assert_eq!(root.to_compact(), r#"{"a":[2,3],"b":{"x":2.00,"y":7}}"#);
        h.go_to(&mut root, 0).unwrap();
        assert_eq!(root.to_compact(), src);
    }

    #[test]
    fn failed_transaction_rolls_back() {
        let src = r#"{"a":1,"b":2}"#;
        let mut root = json::parse(src).unwrap();
        let ops = vec![
            Op::Set {
                path: vec![k("a")],
                old: Value::int(1),
                new: Value::int(5),
            },
            Op::Set {
                path: vec![k("b")],
                old: Value::int(9),
                new: Value::int(5),
            },
        ];
        assert!(apply_all(&mut root, &ops).is_err());
        assert_eq!(root.to_compact(), src);
    }

    #[test]
    fn ops_serialize() {
        let op = Op::Insert {
            parent: vec![k("a"), Seg::Index(2)],
            index: 0,
            key: None,
            value: Value::float2(1.0),
        };
        let text = serde_json::to_string(&op).unwrap();
        let back: Op = serde_json::from_str(&text).unwrap();
        assert_eq!(back, op);
    }
}
