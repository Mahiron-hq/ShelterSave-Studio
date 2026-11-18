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

//! Full JSON tree (expert mode).

use eframe::egui::{self, RichText};
use shelter_core::actions::Action;
use shelter_core::json::{path_to_string, Seg, Value};

use crate::app::App;
use crate::icons;
use crate::style;

const MAX_MATCHES: usize = 300;
const MAX_CHILDREN: usize = 400;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let c = style::colors(ui.ctx());
    if !app.expert() {
        ui.label(
            RichText::new(format!("{}  {}", icons::LOCK, app.t("o.treeLocked"))).color(c.muted),
        );
        return;
    }
    ui.label(RichText::new(app.t("o.treeHelp")).color(c.muted));
    ui.horizontal(|ui| {
        ui.label(icons::SEARCH);
        let hint = app.t("o.searchTree");
        ui.add(
            egui::TextEdit::singleline(&mut app.tree_search)
                .hint_text(hint)
                .desired_width(320.0),
        );
    });
    let Some(open) = &app.open else { return };
    let root = open.doc.shared_root();
    let q = app.tree_search.trim().to_lowercase();
    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        if q.is_empty() {
            node(app, ui, &root, &mut Vec::new(), "root");
        } else {
            let mut hits = Vec::new();
            find(&root, &mut Vec::new(), &q, &mut hits);
            ui.label(
                RichText::new(app.tf("o.matches", &[("n", hits.len().to_string())]))
                    .small()
                    .color(c.muted),
            );
            for path in hits.iter().take(MAX_MATCHES) {
                if let Some(v) = root.at(path) {
                    let mut pth = path.clone();
                    node(app, ui, v, &mut pth, &path_to_string(path));
                }
            }
        }
    });
}

fn find(v: &Value, path: &mut Vec<Seg>, q: &str, out: &mut Vec<Vec<Seg>>) {
    if out.len() >= MAX_MATCHES {
        return;
    }
    let ps = path_to_string(path).to_lowercase();
    let hit_path = !path.is_empty() && ps.contains(q);
    match v {
        Value::Object(o) => {
            if hit_path {
                out.push(path.clone());
            }
            for (k, c) in &o.0 {
                path.push(Seg::Key(k.text()));
                find(c, path, q, out);
                path.pop();
            }
        }
        Value::Array(a) => {
            if hit_path {
                out.push(path.clone());
            }
            for (i, c) in a.iter().enumerate() {
                path.push(Seg::Index(i));
                find(c, path, q, out);
                path.pop();
            }
        }
        other => {
            let text = match other {
                Value::String(s) => s.text(),
                x => x.to_compact(),
            };
            if hit_path || text.to_lowercase().contains(q) {
                out.push(path.clone());
            }
        }
    }
}

fn node(app: &mut App, ui: &mut egui::Ui, v: &Value, path: &mut Vec<Seg>, label: &str) {
    let c = style::colors(ui.ctx());
    match v {
        Value::Object(o) => {
            let title = format!("{label}  {{{}}}", o.len());
            let resp = egui::CollapsingHeader::new(RichText::new(title).font(style::mono(13.0)))
                .id_salt(path_to_string(path))
                .show(ui, |ui| {
                    for (i, (k, child)) in o.0.iter().enumerate().take(MAX_CHILDREN) {
                        let key = k.text();
                        path.push(Seg::Key(key.clone()));
                        ui.horizontal(|ui| {
                            if crate::app::small_icon(ui, icons::X, &app.t("c.remove")).clicked() {
                                let mut parent = path.clone();
                                parent.pop();
                                app.act(Action::RemoveAt { parent, index: i });
                            }
                            ui.vertical(|ui| node(app, ui, child, path, &key));
                        });
                        path.pop();
                    }
                    add_row(app, ui, path, true, o.len());
                });
            let _ = resp;
        }
        Value::Array(a) => {
            let title = format!("{label}  [{}]", a.len());
            egui::CollapsingHeader::new(RichText::new(title).font(style::mono(13.0)))
                .id_salt(path_to_string(path))
                .show(ui, |ui| {
                    for (i, child) in a.iter().enumerate().take(MAX_CHILDREN) {
                        path.push(Seg::Index(i));
                        ui.horizontal(|ui| {
                            if crate::app::small_icon(ui, icons::X, &app.t("c.remove")).clicked() {
                                let mut parent = path.clone();
                                parent.pop();
                                app.act(Action::RemoveAt { parent, index: i });
                            }
                            ui.vertical(|ui| node(app, ui, child, path, &format!("[{i}]")));
                        });
                        path.pop();
                    }
                    if a.len() > MAX_CHILDREN {
                        ui.label(
                            RichText::new(format!("… {}", a.len() - MAX_CHILDREN)).color(c.muted),
                        );
                    }
                    add_row(app, ui, path, false, a.len());
                });
        }
        scalar => {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).font(style::mono(13.0)).color(c.muted));
                let key = format!("tree:{}", path_to_string(path));
                let shown = scalar.to_compact();
                let mut text = app
                    .tree_edit
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| shown.clone());
                let valid = shelter_core::json::parse(&text).is_ok();
                let mut edit = egui::TextEdit::singleline(&mut text)
                    .desired_width(260.0)
                    .font(style::mono(13.0));
                if !valid {
                    edit = edit.text_color(c.danger);
                }
                let resp = ui.add(edit);
                if resp.changed() {
                    app.tree_edit.insert(key.clone(), text.clone());
                }
                if resp.lost_focus() {
                    app.tree_edit.remove(&key);
                    if text != shown {
                        app.act(Action::SetRaw {
                            path: path.clone(),
                            json: text,
                        });
                    }
                }
                if app.open.as_ref().is_some_and(|o| o.doc.is_modified(path)) {
                    ui.label(RichText::new("●").color(c.mark))
                        .on_hover_text(app.t("c.modified"));
                    if crate::app::small_icon(ui, icons::RESET, &app.t("c.revert")).clicked() {
                        app.act(Action::RevertValue { path: path.clone() });
                    }
                }
            });
        }
    }
}

fn add_row(app: &mut App, ui: &mut egui::Ui, path: &[Seg], object: bool, len: usize) {
    ui.horizontal(|ui| {
        if object {
            let hint = app.t("o.key");
            ui.add(
                egui::TextEdit::singleline(&mut app.tree_add_key)
                    .hint_text(hint)
                    .desired_width(140.0),
            );
        }
        let hint = app.t("o.value");
        ui.add(
            egui::TextEdit::singleline(&mut app.tree_add_value)
                .hint_text(hint)
                .desired_width(180.0)
                .font(style::mono(13.0)),
        );
        let label = if object {
            app.t("o.addKey")
        } else {
            app.t("o.addElement")
        };
        if ui
            .small_button(format!("{} {label}", icons::PLUS))
            .clicked()
        {
            let key = object.then(|| app.tree_add_key.trim().to_string());
            if key.as_ref().is_none_or(|k| !k.is_empty()) {
                let json = if app.tree_add_value.trim().is_empty() {
                    "null".to_string()
                } else {
                    app.tree_add_value.trim().to_string()
                };
                if app.act(Action::InsertRaw {
                    parent: path.to_vec(),
                    index: len,
                    key,
                    json,
                }) {
                    app.tree_add_key.clear();
                    app.tree_add_value.clear();
                }
            }
        }
    });
}
