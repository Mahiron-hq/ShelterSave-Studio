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

//! One search box for editor sections, dwellers and items.

use eframe::egui::{self, RichText};
use shelter_core::catalog::{catalog, ItemKind};
use shelter_core::vault;

use crate::app::{App, Modal, OtherTab, Section, VaultTab};
use crate::icons;
use crate::style;

#[derive(Clone)]
enum Target {
    Section(Section),
    Vault(VaultTab),
    Other(OtherTab),
    Settings,
    Dweller(i64),
    Item(ItemKind, String),
}

/// Editor places reachable from search, by string key.
const PLACES: &[(&str, &str)] = &[
    ("nav.save", "s"),
    ("save.general", "s"),
    ("save.time", "s"),
    ("save.summary", "s"),
    ("save.integrity", "s"),
    ("save.json", "s"),
    ("save.backups", "s"),
    ("nav.vault", "v"),
    ("vault.resources", "vr"),
    ("vault.bonus", "vr"),
    ("vault.lunchboxes", "vr"),
    ("res.Nuka", "vr"),
    ("res.Food", "vr"),
    ("res.Energy", "vr"),
    ("res.Water", "vr"),
    ("res.StimPack", "vr"),
    ("res.RadAway", "vr"),
    ("res.NukaColaQuantum", "vr"),
    ("vault.storage", "vs"),
    ("vault.rooms", "vm"),
    ("vault.map", "vm"),
    ("vault.rocks", "vm"),
    ("vault.themes", "vt"),
    ("vault.unlocks", "vt"),
    ("vault.emergency", "vt"),
    ("nav.dwellers", "d"),
    ("dw.pets", "d"),
    ("dw.waiting", "d"),
    ("nav.other", "o"),
    ("o.quests", "oq"),
    ("o.wasteland", "ow"),
    ("o.objectives", "oo"),
    ("o.stats", "os"),
    ("o.tutorial", "ot"),
    ("o.events", "oe"),
    ("o.service", "ov"),
    ("o.tree", "ox"),
    ("set.title", "set"),
    ("set.language", "set"),
    ("set.theme", "set"),
    ("set.mode", "set"),
    ("set.itemNames", "set"),
    ("set.backups", "set"),
];

fn place(code: &str) -> Target {
    match code {
        "s" => Target::Section(Section::Save),
        "v" => Target::Section(Section::Vault),
        "vr" => Target::Vault(VaultTab::Resources),
        "vs" => Target::Vault(VaultTab::Storage),
        "vm" => Target::Vault(VaultTab::Rooms),
        "vt" => Target::Vault(VaultTab::Themes),
        "d" => Target::Section(Section::Dwellers),
        "oq" => Target::Other(OtherTab::Quests),
        "ow" => Target::Other(OtherTab::Wasteland),
        "oo" => Target::Other(OtherTab::Objectives),
        "os" => Target::Other(OtherTab::Stats),
        "ot" => Target::Other(OtherTab::Tutorial),
        "oe" => Target::Other(OtherTab::Events),
        "ov" => Target::Other(OtherTab::Service),
        "ox" => Target::Other(OtherTab::Tree),
        "set" => Target::Settings,
        _ => Target::Section(Section::Other),
    }
}

fn results(app: &App, q: &str) -> Vec<(String, String, Target)> {
    let q = q.trim().to_lowercase();
    let mut out = Vec::new();
    if q.is_empty() {
        return out;
    }
    let has_doc = app.open.is_some();
    for (key, code) in PLACES {
        let text = app.t(key);
        if text.to_lowercase().contains(&q) && (has_doc || *code == "set") {
            out.push((
                format!("{}  {text}", icons::LAYERS),
                String::new(),
                place(code),
            ));
        }
    }
    if let Some(open) = &app.open {
        let root = open.doc.root();
        for d in vault::arr(root, &vault::DWELLERS) {
            let name = vault::dweller_name(d);
            let Some(id) = vault::int(d, &["serializeId"]) else {
                continue;
            };
            if name.to_lowercase().contains(&q) || id.to_string() == q {
                out.push((
                    format!("{}  {name}", icons::USERS),
                    format!("#{id}"),
                    Target::Dweller(id),
                ));
            }
            if out.len() > 60 {
                break;
            }
        }
        let mut seen = std::collections::HashSet::new();
        for it in vault::arr(root, &vault::ITEMS) {
            let (Some(id), Some(kind)) = (
                vault::text(it, &["id"]),
                vault::text(it, &["type"]).and_then(|t| ItemKind::parse(&t)),
            ) else {
                continue;
            };
            if !seen.insert((kind, id.clone())) {
                continue;
            }
            let info = catalog().item(kind, &id);
            let hit =
                id.to_lowercase().contains(&q) || info.as_ref().is_some_and(|i| i.name.matches(&q));
            if hit {
                let name = info.map_or_else(
                    || id.clone(),
                    |i| i.name.get(app.settings.russian_item_names).to_string(),
                );
                out.push((
                    format!("{}  {name}", icons::ARCHIVE),
                    id.clone(),
                    Target::Item(kind, id),
                ));
            }
            if out.len() > 100 {
                break;
            }
        }
    }
    out
}

pub fn toolbar_search(app: &mut App, ui: &mut egui::Ui) {
    let hint = format!("{}  {}", icons::SEARCH, app.t("tb.search"));
    let resp = ui.add(
        egui::TextEdit::singleline(&mut app.global_search)
            .hint_text(hint)
            .desired_width(280.0),
    );
    if ui.input_mut(|i| {
        i.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND,
            egui::Key::F,
        ))
    }) {
        resp.request_focus();
    }
    let popup_id = ui.make_persistent_id("global_search");
    if resp.changed() || resp.gained_focus() {
        egui::Popup::open_id(ui.ctx(), popup_id);
    }
    let found = results(app, &app.global_search);
    if found.is_empty() {
        return;
    }
    let muted = style::colors(ui.ctx()).muted;
    let mut pick = None;
    egui::Popup::from_response(&resp)
        .id(popup_id)
        .open_memory(None)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(360.0);
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for (i, (title, sub, target)) in found.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(false, title).clicked() {
                                pick = Some(target.clone());
                            }
                            if !sub.is_empty() {
                                ui.label(RichText::new(sub).font(style::mono(11.0)).color(muted));
                            }
                        });
                        let _ = i;
                    }
                });
        });
    if let Some(t) = pick {
        egui::Popup::close_id(ui.ctx(), popup_id);
        app.global_search.clear();
        match t {
            Target::Section(s) => app.section = s,
            Target::Vault(v) => {
                app.section = Section::Vault;
                app.vault_tab = v;
            }
            Target::Other(o) => {
                app.section = Section::Other;
                app.other_tab = o;
            }
            Target::Settings => app.modals.push(Modal::Settings),
            Target::Dweller(id) => {
                app.section = Section::Dwellers;
                app.dw.focus = Some(id);
                app.dw.selected = [id].into();
            }
            Target::Item(kind, id) => {
                app.section = Section::Vault;
                app.vault_tab = VaultTab::Storage;
                app.st.kind = Some(kind);
                app.st.search = id;
            }
        }
    }
}
