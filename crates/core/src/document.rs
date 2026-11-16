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

//! An open save: original tree, current tree and undo history.

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
    root: Value,
    history: History,
    original_issues: Vec<Issue>,
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
            root,
            history: History::default(),
            original_issues,
        }
    }

    pub fn root(&self) -> &Value {
        &self.root
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
        let tx = actions::run(&mut self.root, &ctx, action)?;
        self.history.record(tx.clone());
        Ok(tx)
    }

    pub fn undo(&mut self) -> Result<Option<Label>, EditError> {
        self.history.undo(&mut self.root)
    }

    pub fn redo(&mut self) -> Result<Option<Label>, EditError> {
        self.history.redo(&mut self.root)
    }

    pub fn go_to(&mut self, step: usize) -> Result<(), EditError> {
        self.history.go_to(&mut self.root, step)
    }

    /// Resets every change as one undoable step.
    pub fn reset_to_original(&mut self) -> Option<Transaction> {
        if self.root == self.original {
            return None;
        }
        let op = Op::Set {
            path: Vec::new(),
            old: self.root.clone(),
            new: self.original.clone(),
        };
        self.root = self.original.clone();
        let tx = Transaction {
            label: Label::new("act.resetAll"),
            ops: vec![op],
            affected: 1,
        };
        self.history.record(tx.clone());
        Some(tx)
    }

    pub fn is_dirty(&self) -> bool {
        self.root != self.original
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
        self.original = self.root.clone();
        self.original_issues = integrity::check(&self.root);
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
            crate::edit::apply_all(&mut doc.root, &tx.ops).map_err(|_| FormatError::NotObject)?;
        }
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
}
