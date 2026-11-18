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

//! An open save: original tree, current tree and undo history.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::actions::{self, Action, ActionError, Ctx};
use crate::diff::{self, Change};
use crate::edit::{EditError, History, Label, Op, Transaction};
use crate::integrity::{self, Issue};
use crate::json::{Seg, Value};
use crate::rules::{self, Mode, Warning};
use crate::sav::{self, FormatError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Sav,
    Json,
}

pub struct Document {
    pub file_name: String,
    pub source: SourceKind,
    original: Value,
    /// Shared so the UI can hold it for a frame without copying the tree.
    root: Arc<Value>,
    history: History,
    original_issues: Vec<Issue>,
    /// Cached `root != original`, cleared on every change.
    dirty: std::cell::Cell<Option<bool>>,
    revision: u64,
}

/// Unique across documents, so a cache keyed by it never mixes two files.
fn next_revision() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Everything needed to restore an editing session (web local cache).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub file_name: String,
    pub source: SourceKind,
    pub original: String,
    pub history: History,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReport {
    pub changes: Vec<Change>,
    pub new_issues: Vec<Issue>,
    pub old_issues: Vec<Issue>,
    pub new_warnings: Vec<Warning>,
    pub old_warnings: Vec<Warning>,
}

impl Document {
    /// Opens `.sav` or plain JSON, detected by content.
    pub fn open(bytes: &[u8], file_name: &str) -> Result<Document, FormatError> {
        let (root, source) = if sav::looks_like_json(bytes) {
            (sav::read_json(bytes)?, SourceKind::Json)
        } else {
            (sav::read_sav(bytes)?, SourceKind::Sav)
        };
        Ok(Document::from_root(root, file_name, source))
    }

    pub fn from_root(root: Value, file_name: &str, source: SourceKind) -> Document {
        let original_issues = integrity::check(&root);
        Document {
            file_name: file_name.to_string(),
            source,
            original: root.clone(),
            root: Arc::new(root),
            history: History::default(),
            original_issues,
            dirty: std::cell::Cell::new(Some(false)),
            revision: next_revision(),
        }
    }

    pub fn root(&self) -> &Value {
        &self.root
    }

    /// The current tree, cheap to keep while the document is edited.
    pub fn shared_root(&self) -> Arc<Value> {
        self.root.clone()
    }

    pub fn original(&self) -> &Value {
        &self.original
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn apply(&mut self, mode: Mode, action: &Action) -> Result<Transaction, ActionError> {
        let ctx = Ctx {
            mode,
            original: &self.original,
        };
        let tx = actions::run(Arc::make_mut(&mut self.root), &ctx, action)?;
        self.touch();
        self.history.record(tx.clone());
        Ok(tx)
    }

    pub fn undo(&mut self) -> Result<Option<Label>, EditError> {
        self.touch();
        self.history.undo(Arc::make_mut(&mut self.root))
    }

    pub fn redo(&mut self) -> Result<Option<Label>, EditError> {
        self.touch();
        self.history.redo(Arc::make_mut(&mut self.root))
    }

    pub fn go_to(&mut self, step: usize) -> Result<(), EditError> {
        self.touch();
        self.history.go_to(Arc::make_mut(&mut self.root), step)
    }

    /// Resets every change as one undoable step.
    pub fn reset_to_original(&mut self) -> Option<Transaction> {
        if *self.root == self.original {
            return None;
        }
        let op = Op::Set {
            path: Vec::new(),
            old: (*self.root).clone(),
            new: self.original.clone(),
        };
        self.root = Arc::new(self.original.clone());
        self.dirty.set(Some(false));
        self.revision = next_revision();
        let tx = Transaction {
            label: Label::new("act.resetAll"),
            ops: vec![op],
            affected: 1,
            action: None,
        };
        self.history.record(tx.clone());
        Some(tx)
    }

    fn touch(&mut self) {
        self.dirty.set(None);
        self.revision = next_revision();
    }

    /// Changes whenever the tree or the saved baseline changes.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        if let Some(d) = self.dirty.get() {
            return d;
        }
        let d = *self.root != self.original;
        self.dirty.set(Some(d));
        d
    }

    pub fn is_modified(&self, path: &[Seg]) -> bool {
        diff::is_modified(&self.original, &self.root, path)
    }

    pub fn original_at(&self, path: &[Seg]) -> Option<&Value> {
        diff::original_at(&self.original, &self.root, path)
    }

    pub fn changes(&self) -> Vec<Change> {
        diff::diff(&self.original, &self.root)
    }

    pub fn issues(&self) -> (Vec<Issue>, Vec<Issue>) {
        let (old, new) = integrity::classify(&self.original_issues, integrity::check(&self.root));
        (new, old)
    }

    pub fn save_report(&self) -> SaveReport {
        let (new_issues, old_issues) = self.issues();
        let (old_warnings, new_warnings) = rules::new_warnings(&self.original, &self.root);
        SaveReport {
            changes: self.changes(),
            new_issues,
            old_issues,
            new_warnings,
            old_warnings,
        }
    }

    pub fn to_sav(&self) -> String {
        sav::write_sav(&self.root)
    }

    pub fn to_json_pretty(&self) -> String {
        self.root.to_pretty()
    }

    /// Marks the current state as saved: it becomes the new baseline.
    pub fn mark_saved(&mut self) {
        self.original = (*self.root).clone();
        self.dirty.set(Some(false));
        self.revision = next_revision();
        self.original_issues = integrity::check(&self.root);
    }

    /// The same file as written later by the game, with this document's edits
    /// repeated on it. Returns the new document and how many edits did not apply.
    pub fn rebase(&self, bytes: &[u8], mode: Mode) -> Result<(Document, usize), FormatError> {
        let mut doc = Document::open(bytes, &self.file_name)?;
        let mut failed = 0;
        for tx in self.history.done() {
            let action = tx
                .action
                .as_ref()
                .and_then(|a| serde_json::from_value::<Action>(a.clone()).ok());
            match action.map(|a| doc.apply(mode, &a)) {
                Some(Ok(_) | Err(ActionError::Nothing)) => {}
                _ => failed += 1,
            }
        }
        Ok((doc, failed))
    }

    pub fn session(&self) -> Session {
        Session {
            file_name: self.file_name.clone(),
            source: self.source,
            original: self.original.to_compact(),
            history: self.history.clone(),
        }
    }

    pub fn restore(session: Session) -> Result<Document, FormatError> {
        let root = sav::read_json(session.original.as_bytes())?;
        let mut doc = Document::from_root(root, &session.file_name, session.source);
        for tx in session.history.done() {
            crate::edit::apply_all(Arc::make_mut(&mut doc.root), &tx.ops)
                .map_err(|_| FormatError::NotObject)?;
        }
        doc.touch();
        doc.history = session.history;
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_edit_save_and_restore_session() {
        let src = sav::encrypt_text(
            r#"{"vault":{"VaultName":"1","storage":{"resources":{"Nuka":5}}},"dwellers":{"dwellers":[]}}"#,
        );
        let mut doc = Document::open(src.as_bytes(), "Vault1.sav").unwrap();
        assert_eq!(doc.to_sav(), src);
        doc.apply(
            Mode::Normal,
            &Action::SetResource {
                bonus: false,
                key: "Nuka".into(),
                value: 99.0,
            },
        )
        .unwrap();
        assert!(doc.is_dirty());
        assert_eq!(doc.changes().len(), 1);
        let s = serde_json::to_string(&doc.session()).unwrap();
        let back = Document::restore(serde_json::from_str(&s).unwrap()).unwrap();
        assert_eq!(back.root(), doc.root());
        doc.reset_to_original().unwrap();
        assert!(!doc.is_dirty());
        doc.undo().unwrap();
        assert!(doc.is_dirty());
    }

    #[test]
    fn edits_move_to_a_newer_file() {
        let old = sav::encrypt_text(
            r#"{"vault":{"VaultName":"1","storage":{"resources":{"Nuka":5,"Food":1}}},"dwellers":{"dwellers":[]}}"#,
        );
        let newer = sav::encrypt_text(
            r#"{"vault":{"VaultName":"1","storage":{"resources":{"Nuka":7,"Food":3}}},"dwellers":{"dwellers":[]}}"#,
        );
        let mut doc = Document::open(old.as_bytes(), "Vault1.sav").unwrap();
        doc.apply(
            Mode::Normal,
            &Action::SetResource {
                bonus: false,
                key: "Nuka".into(),
                value: 99.0,
            },
        )
        .unwrap();
        let (back, failed) = doc.rebase(newer.as_bytes(), Mode::Normal).unwrap();
        assert_eq!(failed, 0);
        let res = |d: &Document, k: &str| {
            d.root()
                .at(&crate::vault::p(&["vault", "storage", "resources", k]))
                .and_then(Value::as_f64)
        };
        assert_eq!(res(&back, "Nuka"), Some(99.0));
        assert_eq!(res(&back, "Food"), Some(3.0));
        assert!(back.history().can_undo());
    }
}
