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

use std::collections::BTreeMap;

use eframe::egui::{self, RichText, Sense};
use shelter_core::actions::Action;
use shelter_core::catalog::{catalog, ItemKind};
use shelter_core::json::{Seg, Value};
use shelter_core::rules::RESOURCE_MAX;
use shelter_core::vault::{self, p, pi, RESOURCE_KEYS};

use crate::app::{panel, section_title, App, VaultTab};
use crate::icons;
use crate::style;
use crate::widgets::{self, Opts};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    section_title(ui, icons::HOUSE, &app.t("nav.vault"), "");
    ui.horizontal(|ui| {
        for (tab, key) in [
            (VaultTab::Resources, "vault.tab.resources"),
            (VaultTab::Storage, "vault.tab.storage"),
            (VaultTab::Rooms, "vault.tab.rooms"),
            (VaultTab::Themes, "vault.tab.themes"),
        ] {
            if ui
                .selectable_label(app.vault_tab == tab, app.t(key))
                .clicked()
            {
                app.vault_tab = tab;
            }
        }
    });
    ui.separator();
    match app.vault_tab {
        VaultTab::Resources => {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| resources(app, ui))
                .inner
        }
        VaultTab::Storage => storage(app, ui),
        VaultTab::Rooms => rooms(app, ui),
        VaultTab::Themes => {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| themes(app, ui))
                .inner
        }
    }
}

fn resources(app: &mut App, ui: &mut egui::Ui) {
    for (bonus, title, help) in [
        (false, "vault.resources", "vault.resourcesHelp"),
        (true, "vault.bonus", "vault.bonusHelp"),
    ] {
        let base: &[&str] = if bonus {
            &vault::BONUS
        } else {
            &vault::RESOURCES
        };
        if app
            .open
            .as_ref()
            .and_then(|o| o.doc.root().at(&p(base)))
            .is_none()
        {
            continue;
        }
        panel(ui, |ui| {
            ui.label(RichText::new(app.t(title)).font(style::bold(16.0)));
            ui.label(
                RichText::new(app.t(help))
                    .small()
                    .color(style::colors(ui.ctx()).muted),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(format!("{}  {}", icons::SPARKLES, app.t("vault.fill")))
                    .on_hover_text(app.t("vault.fillHelp"))
                    .clicked()
                {
                    app.act(Action::FillResources { bonus });
                }
                ui.label(app.t("vault.setAll"));
                ui.add(
                    egui::DragValue::new(&mut app.resource_all)
                        .range(0.0..=RESOURCE_MAX as f64)
                        .speed(100.0),
                );
                if ui.button(app.t("c.apply")).clicked() {
                    let v = app.resource_all;
                    app.act(Action::SetAllResources { bonus, value: v });
                }
            });
            for key in RESOURCE_KEYS {
                let mut path = p(base);
                path.push(Seg::Key(key.into()));
                if app
                    .open
                    .as_ref()
                    .and_then(|o| o.doc.root().at(&path))
                    .is_none()
                {
                    continue;
                }
                let mut o =
                    Opts::text(app.t(&format!("res.{key}"))).range(0.0, RESOURCE_MAX as f64);
                if matches!(key, "Lunchbox" | "MrHandy" | "PetCarrier") {
                    o = o.help(app.t("res.counterHelp"));
                }
                widgets::num_field(app, ui, &path, o);
            }
        });
        ui.add_space(10.0);
    }
    lunchboxes(app, ui);
}

fn lunchboxes(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let types: Vec<i64> = vault::arr(open.doc.root(), &["vault", "LunchBoxesByType"])
        .iter()
        .filter_map(Value::as_i64)
        .collect();
    let mut counts = [0i64; 4];
    let mut other = 0;
    for t in &types {
        match t {
            0..=3 => counts[*t as usize] += 1,
            _ => other += 1,
        }
    }
    let mut edit = app.lunch_counts.unwrap_or(counts);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.lunchboxes")).font(style::bold(16.0)));
        ui.label(
            RichText::new(app.t("vault.lunchboxesHelp"))
                .small()
                .color(style::colors(ui.ctx()).muted),
        );
        egui::Grid::new("lunch")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for (i, v) in edit.iter_mut().enumerate() {
                    ui.label(app.t(&format!("vault.lb.{i}")));
                    ui.add(
                        egui::DragValue::new(v).range(0..=shelter_core::rules::LUNCHBOX_MAX as i64),
                    );
                    ui.end_row();
                }
            });
        if other > 0 {
            ui.label(RichText::new(app.tf("vault.lb.other", &[("n", other.to_string())])).small());
        }
        let changed = edit != counts;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(changed, egui::Button::new(app.t("c.apply")))
                .clicked()
            {
                if app.act(Action::SetLunchboxes { counts: edit }) {
                    app.lunch_counts = None;
                }
            } else if changed {
                app.lunch_counts = Some(edit);
            }
            if changed && ui.button(app.t("c.cancel")).clicked() {
                app.lunch_counts = None;
            }
        });
    });
}

fn item_name(app: &App, kind: ItemKind, id: &str) -> (String, String, bool) {
    match catalog().item(kind, id) {
        Some(i) => (
            i.name.get(app.settings.russian_item_names).to_string(),
            i.rarity.to_string(),
            true,
        ),
        None => (id.to_string(), String::new(), false),
    }
}

fn rarity_label(app: &App, r: &str) -> String {
    catalog().names.rarity.get(r).map_or_else(
        || r.to_string(),
        |n| n.get(app.lang == shelter_core::i18n::Lang::Ru).to_string(),
    )
}

fn storage(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let mut groups: BTreeMap<(ItemKind, String), usize> = BTreeMap::new();
    let mut other = 0;
    for it in vault::arr(open.doc.root(), &vault::ITEMS) {
        let kind = vault::text(it, &["type"]).and_then(|t| ItemKind::parse(&t));
        match (kind, vault::text(it, &["id"])) {
            (Some(k), Some(id)) => *groups.entry((k, id)).or_default() += 1,
            _ => other += 1,
        }
    }
    let total: usize = groups.values().sum::<usize>() + other;
    let c = style::colors(ui.ctx());
    ui.label(RichText::new(app.t("vault.storageHelp")).color(c.muted));
    add_item_bar(app, ui);
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(icons::SEARCH);
        let hint = app.t("c.search");
        ui.add(
            egui::TextEdit::singleline(&mut app.st.search)
                .hint_text(hint)
                .desired_width(200.0),
        );
        egui::ComboBox::from_id_salt("st_kind")
            .selected_text(app.st.kind.map_or_else(
                || app.t("c.all"),
                |k| app.t(&format!("it.kind.{}", k.as_str())),
            ))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(app.st.kind.is_none(), app.t("c.all"))
                    .clicked()
                {
                    app.st.kind = None;
                }
                for k in ItemKind::ALL {
                    if ui
                        .selectable_label(
                            app.st.kind == Some(k),
                            app.t(&format!("it.kind.{}", k.as_str())),
                        )
                        .clicked()
                    {
                        app.st.kind = Some(k);
                    }
                }
            });
        egui::ComboBox::from_id_salt("st_rarity")
            .selected_text(
                app.st
                    .rarity
                    .clone()
                    .map_or_else(|| app.t("c.all"), |r| rarity_label(app, &r)),
            )
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(app.st.rarity.is_none(), app.t("c.all"))
                    .clicked()
                {
                    app.st.rarity = None;
                }
                for r in ["Common", "Normal", "Rare", "Legendary"] {
                    if ui
                        .selectable_label(app.st.rarity.as_deref() == Some(r), rarity_label(app, r))
                        .clicked()
                    {
                        app.st.rarity = Some(r.to_string());
                    }
                }
            });
        ui.label(RichText::new(app.tf("it.total", &[("n", total.to_string())])).color(c.muted));
        let sel = app.st.selected.len();
        if ui
            .add_enabled(
                sel > 0,
                egui::Button::new(format!(
                    "{}  {}",
                    icons::TRASH,
                    app.t("vault.removeSelected")
                )),
            )
            .clicked()
        {
            let picked: Vec<(ItemKind, String)> = app.st.selected.iter().cloned().collect();
            let counts: Vec<(ItemKind, String, i64)> = picked
                .into_iter()
                .map(|(k, id)| {
                    let n = *groups.get(&(k, id.clone())).unwrap_or(&0) as i64;
                    (k, id, n)
                })
                .collect();
            for (k, id, n) in counts {
                app.act(Action::RemoveItems {
                    kind: k,
                    id,
                    count: n,
                });
            }
            app.st.selected.clear();
        }
    });
    let q = app.st.search.trim().to_lowercase();
    type Row = ((ItemKind, String), usize, String, String, bool);
    let rows: Vec<Row> = groups
        .into_iter()
        .filter_map(|((k, id), n)| {
            if app.st.kind.is_some_and(|f| f != k) {
                return None;
            }
            let (name, rarity, known) = item_name(app, k, &id);
            if app.st.rarity.as_ref().is_some_and(|r| *r != rarity) {
                return None;
            }
            if !q.is_empty() {
                let hit = id.to_lowercase().contains(&q)
                    || name.to_lowercase().contains(&q)
                    || catalog().item(k, &id).is_some_and(|i| i.name.matches(&q));
                if !hit {
                    return None;
                }
            }
            Some(((k, id), n, name, rarity, known))
        })
        .collect();
    if rows.is_empty() {
        ui.add_space(12.0);
        ui.label(
            RichText::new(if q.is_empty() {
                app.t("c.empty")
            } else {
                app.t("c.noResults")
            })
            .color(c.muted),
        );
        return;
    }
    let row_h = 34.0;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        row_h,
        rows.len(),
        |ui, range| {
            for ((k, id), n, name, rarity, known) in &rows[range] {
                ui.horizontal(|ui| {
                    ui.set_height(row_h - 6.0);
                    let key = (*k, id.clone());
                    let mut on = app.st.selected.contains(&key);
                    if ui.checkbox(&mut on, "").changed() {
                        if on {
                            app.st.selected.insert(key.clone());
                        } else {
                            app.st.selected.remove(&key);
                        }
                    }
                    let icon = match k {
                        ItemKind::Weapon => icons::SWORD,
                        ItemKind::Outfit => icons::SHIRT,
                        ItemKind::Junk => icons::WRENCH,
                        ItemKind::Pet => icons::PAW,
                    };
                    ui.label(RichText::new(icon).color(c.muted));
                    ui.label(RichText::new(name).font(style::bold(14.0)));
                    if !*known {
                        ui.label(
                            RichText::new(app.t("c.unknownItem"))
                                .small()
                                .color(c.warning),
                        );
                    }
                    if !rarity.is_empty() {
                        ui.label(
                            RichText::new(rarity_label(app, rarity))
                                .small()
                                .color(c.muted),
                        );
                    }
                    if ui
                        .add(
                            egui::Label::new(
                                RichText::new(id).font(style::mono(11.0)).color(c.muted),
                            )
                            .sense(Sense::click()),
                        )
                        .on_hover_text(app.t("c.copy"))
                        .clicked()
                    {
                        ui.ctx().copy_text(id.clone());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::app::small_icon(ui, icons::TRASH, &app.t("c.removeAll")).clicked()
                        {
                            app.act(Action::RemoveItems {
                                kind: *k,
                                id: id.clone(),
                                count: *n as i64,
                            });
                        }
                        if ui
                            .small_button("−")
                            .on_hover_text(app.t("vault.removeOne"))
                            .clicked()
                        {
                            app.act(Action::RemoveItems {
                                kind: *k,
                                id: id.clone(),
                                count: 1,
                            });
                        }
                        if ui.small_button("+").on_hover_text(app.t("c.add")).clicked() {
                            app.act(Action::AddItems {
                                kind: *k,
                                id: id.clone(),
                                count: 1,
                            });
                        }
                        ui.label(RichText::new(format!("× {n}")).font(style::mono(13.0)));
                    });
                });
            }
        },
    );
}

/// Search box with a list of reference items; returns the picked id.
pub fn item_picker(
    app: &mut App,
    ui: &mut egui::Ui,
    id_salt: &str,
    kinds: &[ItemKind],
    query: &mut String,
) -> Option<(ItemKind, String)> {
    let c = style::colors(ui.ctx());
    let resp = ui.add(
        egui::TextEdit::singleline(query)
            .hint_text(app.t("it.pick"))
            .desired_width(280.0),
    );
    let mut picked = None;
    let popup_id = ui.make_persistent_id(id_salt);
    if resp.gained_focus() || resp.changed() {
        egui::Popup::open_id(ui.ctx(), popup_id);
    }
    egui::Popup::from_response(&resp)
        .id(popup_id)
        .open_memory(None)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(360.0);
            let found = catalog().search(kinds, query);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for i in found.iter().take(200) {
                        let name = i.name.get(app.settings.russian_item_names);
                        let text = format!("{name}  ·  {}", rarity_label(app, i.rarity));
                        let r = ui.selectable_label(false, text).on_hover_text(i.id);
                        if r.clicked() {
                            picked = Some((i.kind, i.id.to_string()));
                        }
                    }
                    if found.is_empty() {
                        ui.label(RichText::new(app.t("c.noResults")).color(c.muted));
                    }
                });
            if app.expert() && !query.trim().is_empty() {
                ui.separator();
                for k in kinds {
                    if ui
                        .button(format!(
                            "{}: {} ({})",
                            app.t("it.manualId"),
                            query.trim(),
                            app.t(&format!("it.kind.{}", k.as_str()))
                        ))
                        .clicked()
                    {
                        picked = Some((*k, query.trim().to_string()));
                    }
                }
            }
        });
    if picked.is_some() {
        egui::Popup::close_id(ui.ctx(), popup_id);
    }
    picked
}

fn add_item_bar(app: &mut App, ui: &mut egui::Ui) {
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.addTitle")).font(style::bold(14.0)));
        ui.horizontal_wrapped(|ui| {
            let kinds: Vec<ItemKind> = app
                .st
                .add_kind
                .map_or_else(|| ItemKind::ALL.to_vec(), |k| vec![k]);
            egui::ComboBox::from_id_salt("add_kind")
                .selected_text(app.st.add_kind.map_or_else(
                    || app.t("c.all"),
                    |k| app.t(&format!("it.kind.{}", k.as_str())),
                ))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(app.st.add_kind.is_none(), app.t("c.all"))
                        .clicked()
                    {
                        app.st.add_kind = None;
                    }
                    for k in ItemKind::ALL {
                        if ui
                            .selectable_label(
                                app.st.add_kind == Some(k),
                                app.t(&format!("it.kind.{}", k.as_str())),
                            )
                            .clicked()
                        {
                            app.st.add_kind = Some(k);
                        }
                    }
                });
            let mut q = std::mem::take(&mut app.st.add_query);
            let picked = item_picker(app, ui, "add_item", &kinds, &mut q);
            app.st.add_query = q;
            ui.label(app.t("vault.quantity"));
            ui.add(
                egui::DragValue::new(&mut app.st.add_count)
                    .range(1..=shelter_core::rules::ITEM_ADD_MAX),
            );
            if let Some((k, id)) = picked {
                let n = app.st.add_count;
                if app.act(Action::AddItems {
                    kind: k,
                    id,
                    count: n,
                }) {
                    app.st.add_query.clear();
                }
            }
        });
    });
}

fn room_name(app: &App, r: &Value) -> String {
    let kind = vault::text(r, &["type"]).unwrap_or_default();
    let level = vault::int(r, &["level"]).unwrap_or(1).clamp(1, 3) as usize;
    match catalog().room(&kind) {
        Some(info) => {
            let n = info.name.get(level - 1).or_else(|| info.name.first());
            n.map_or(kind.clone(), |n| {
                if app.lang == shelter_core::i18n::Lang::Ru {
                    n.get(true).to_string()
                } else {
                    n.en.clone()
                }
            })
        }
        None => kind,
    }
}

fn rooms(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let root = open.doc.root().clone();
    let rooms = vault::arr(&root, &vault::ROOMS);
    let c = style::colors(ui.ctx());
    ui.label(RichText::new(app.t("vault.roomsHelp")).color(c.muted));
    egui::Panel::left("room_list")
        .resizable(true)
        .default_size(280.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    let mut order: Vec<&Value> = rooms.iter().collect();
                    order.sort_by_key(|r| {
                        (
                            vault::int(r, &["row"]).unwrap_or(0),
                            vault::int(r, &["col"]).unwrap_or(0),
                        )
                    });
                    for r in order {
                        let id = vault::int(r, &["deserializeID"]).unwrap_or(-1);
                        let sub = app.tf(
                            "room.position",
                            &[
                                ("row", vault::int(r, &["row"]).unwrap_or(0).to_string()),
                                ("col", vault::int(r, &["col"]).unwrap_or(0).to_string()),
                            ],
                        );
                        let text = format!("{}  ·  {}", room_name(app, r), sub);
                        if ui
                            .selectable_label(app.room_sel == Some(id), text)
                            .clicked()
                        {
                            app.room_sel = Some(id);
                        }
                    }
                });
        });
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            map(app, ui, &root);
            ui.add_space(10.0);
            match app.room_sel.and_then(|id| vault::room_index(&root, id)) {
                Some(i) => room_editor(app, ui, i),
                None => {
                    ui.label(RichText::new(app.t("vault.selectRoom")).color(c.muted));
                }
            }
            ui.add_space(10.0);
            rocks(app, ui, &root);
        });
}

fn class_color(c: &style::Colors, class: &str) -> egui::Color32 {
    match class {
        "Production" => c.accent_soft,
        "Training" => c.success_soft,
        "Crafting" => c.warning_soft,
        "Consumable" => c.danger_soft,
        _ => c.surface_alt,
    }
}

fn map(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let c = style::colors(ui.ctx());
    let lim = &catalog().limits;
    let rows = vault::arr(root, &vault::ROOMS);
    let max_row = rows
        .iter()
        .filter_map(|r| vault::int(r, &["row"]))
        .max()
        .unwrap_or(0)
        .max(
            vault::arr(root, &["vault", "rocks"])
                .iter()
                .filter_map(|r| vault::int(r, &["r"]))
                .max()
                .unwrap_or(0),
        )
        + 1;
    let max_row = max_row.clamp(1, lim.grid_rows);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.map")).font(style::bold(16.0)));
        ui.label(RichText::new(app.t("vault.mapHelp")).small().color(c.muted));
        let cell = ((ui.available_width() - 4.0) / lim.grid_cols as f32).clamp(10.0, 32.0);
        let h = cell * 1.4;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(cell * lim.grid_cols as f32, h * max_row as f32),
            Sense::hover(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, c.bg);
        for rock in vault::arr(root, &["vault", "rocks"]) {
            let (Some(r), Some(col)) = (vault::int(rock, &["r"]), vault::int(rock, &["c"])) else {
                continue;
            };
            let rr = egui::Rect::from_min_size(
                rect.min + egui::vec2(col as f32 * cell, r as f32 * h),
                egui::vec2(cell * 2.0, h),
            )
            .shrink(1.0);
            painter.rect_filled(rr, 2.0, c.border);
        }
        for r in rows {
            let (Some(row), Some(col)) = (vault::int(r, &["row"]), vault::int(r, &["col"])) else {
                continue;
            };
            let id = vault::int(r, &["deserializeID"]).unwrap_or(-1);
            let w = vault::room_width(r) as f32;
            let rr = egui::Rect::from_min_size(
                rect.min + egui::vec2(col as f32 * cell, row as f32 * h),
                egui::vec2(cell * w, h),
            )
            .shrink(1.0);
            let class = vault::text(r, &["class"]).unwrap_or_default();
            let sel = app.room_sel == Some(id);
            painter.rect_filled(rr, 3.0, class_color(&c, &class));
            painter.rect_stroke(
                rr,
                3.0,
                egui::Stroke::new(
                    if sel { 2.0 } else { 1.0 },
                    if sel { c.focus } else { c.border },
                ),
                egui::StrokeKind::Inside,
            );
            let resp = ui
                .interact(rr, ui.scope_id().with(("room", id)), Sense::click())
                .on_hover_text(room_name(app, r));
            if resp.clicked() {
                app.room_sel = Some(id);
            }
        }
    });
}

fn room_editor(app: &mut App, ui: &mut egui::Ui, i: usize) {
    let Some(open) = &app.open else { return };
    let r = vault::arr(open.doc.root(), &vault::ROOMS)[i].clone();
    let id = vault::int(&r, &["deserializeID"]).unwrap_or(-1);
    let kind = vault::text(&r, &["type"]).unwrap_or_default();
    let base = pi(&vault::ROOMS, i, &[]);
    let at = |rest: &[&str]| vault::join(&base, rest);
    let c = style::colors(ui.ctx());
    panel(ui, |ui| {
        ui.label(RichText::new(format!("{}  #{id}", room_name(app, &r))).font(style::bold(16.0)));
        if r.get("level").is_some() && kind != "Elevator" {
            let o = Opts::text(app.t("room.level")).range(1.0, 3.0);
            widgets::num_field(app, ui, &at(&["level"]), o);
        }
        if kind != "Elevator" {
            let max = shelter_core::rules::max_merge(&kind);
            let o = Opts::text(app.t("room.merge"))
                .help(app.t("room.mergeHelp"))
                .range(1.0, max as f64);
            let merge_path = at(&["mergeLevel"]);
            widgets::row(app, ui, &merge_path, &o, |app, ui| {
                let cur = vault::int(&r, &["mergeLevel"]).unwrap_or(1);
                for m in 1..=max {
                    let fits = app
                        .open
                        .as_ref()
                        .is_some_and(|o| shelter_core::actions::merge_fits(o.doc.root(), id, m));
                    if ui
                        .add_enabled(
                            fits || m == cur,
                            egui::Button::selectable(m == cur, m.to_string()),
                        )
                        .clicked()
                        && m != cur
                    {
                        app.act(Action::SetMergeLevel { room: id, merge: m });
                    }
                }
            });
        }
        for (key, label) in [
            ("power", "room.power"),
            ("broken", "room.broken"),
            ("withHole", "room.withHole"),
            ("emergencyDone", "k.emergencyDone"),
        ] {
            if r.get(key).is_some() {
                let o = Opts::text(app.t(label));
                widgets::bool_field(app, ui, &at(&[key]), o);
            }
        }
        if r.at(&p(&["roomHealth", "damageValue"])).is_some() {
            let o = Opts::text(app.t("room.damage"));
            widgets::num_field(app, ui, &at(&["roomHealth", "damageValue"]), o);
        }
        if r.get("currentStateName").is_some() {
            let o = Opts::text(app.t("room.state"));
            let choices = vec![
                ("Idle".to_string(), app.t("room.state.Idle")),
                ("Working".to_string(), app.t("room.state.Working")),
            ];
            widgets::choice_field(app, ui, &at(&["currentStateName"]), o, &choices);
        }
        if r.get("assignedDecoration").is_some() {
            let o = Opts::text(app.t("room.decoration"));
            let mut choices = vec![(String::new(), app.t("c.none"))];
            choices.extend(catalog().decorations.iter().map(|d| {
                (
                    d.id.clone(),
                    d.name
                        .get(app.lang == shelter_core::i18n::Lang::Ru)
                        .to_string(),
                )
            }));
            widgets::choice_field(app, ui, &at(&["assignedDecoration"]), o, &choices);
        }
        if let Some(Value::Object(st)) = r.at(&p(&["storage", "resources"])) {
            ui.add_space(6.0);
            ui.label(RichText::new(app.t("room.stored")).font(style::bold(14.0)));
            for (k, _) in &st.0 {
                let key = k.text();
                let o = Opts::text(app.t(&format!("res.{key}")));
                widgets::num_field(app, ui, &at(&["storage", "resources", &key]), o);
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new(app.t("room.dwellers")).font(style::bold(14.0)));
        let ids: Vec<i64> = r
            .get("dwellers")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        let cap = if kind == "Elevator" {
            0
        } else {
            2 * vault::int(&r, &["mergeLevel"]).unwrap_or(1)
        };
        ui.label(
            RichText::new(app.tf(
                "room.capacity",
                &[("n", ids.len().to_string()), ("max", cap.to_string())],
            ))
            .small()
            .color(c.muted),
        );
        let root = app
            .open
            .as_ref()
            .map(|o| o.doc.root().clone())
            .unwrap_or(Value::Null);
        let mut unassign = None;
        for d in &ids {
            let name = vault::dweller_index(&root, *d)
                .map(|j| vault::dweller_name(&vault::arr(&root, &vault::DWELLERS)[j]))
                .unwrap_or_else(|| format!("#{d}"));
            ui.horizontal(|ui| {
                ui.label(format!("{}  {name}", icons::USERS));
                if crate::app::small_icon(ui, icons::X, &app.t("c.remove")).clicked() {
                    unassign = Some(*d);
                }
            });
        }
        if let Some(d) = unassign {
            app.act(Action::AssignRoom {
                ids: vec![d],
                room: -1,
            });
        }
        if kind != "Elevator" {
            let mut pick = None;
            egui::ComboBox::from_id_salt(("assign", id))
                .selected_text(app.t("room.assign"))
                .width(260.0)
                .show_ui(ui, |ui| {
                    for d in vault::arr(&root, &vault::DWELLERS) {
                        let Some(did) = vault::int(d, &["serializeId"]) else {
                            continue;
                        };
                        if ids.contains(&did) || vault::is_dead(d) {
                            continue;
                        }
                        if ui.selectable_label(false, vault::dweller_name(d)).clicked() {
                            pick = Some(did);
                        }
                    }
                });
            if let Some(d) = pick {
                app.act(Action::AssignRoom {
                    ids: vec![d],
                    room: id,
                });
            }
        }
    });
}

fn rocks(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let list = vault::arr(root, &["vault", "rocks"]);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.rocks")).font(style::bold(16.0)));
        ui.label(
            RichText::new(app.tf("vault.rocksCount", &[("n", list.len().to_string())])).small(),
        );
        if list.is_empty() {
            return;
        }
        if ui
            .button(format!("{}  {}", icons::TRASH, app.t("c.removeAll")))
            .clicked()
        {
            app.act(Action::RemoveAllRocks);
        }
        let mut remove = None;
        egui::ScrollArea::vertical()
            .id_salt("rocks")
            .max_height(180.0)
            .show(ui, |ui| {
                for (i, r) in list.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(app.tf(
                            "vault.rock",
                            &[
                                ("r", vault::int(r, &["r"]).unwrap_or(0).to_string()),
                                ("c", vault::int(r, &["c"]).unwrap_or(0).to_string()),
                            ],
                        ));
                        if crate::app::small_icon(ui, icons::X, &app.t("c.remove")).clicked() {
                            remove = Some(i);
                        }
                    });
                }
            });
        if let Some(i) = remove {
            app.act(Action::RemoveRock { index: i });
        }
    });
}

fn themes(app: &mut App, ui: &mut egui::Ui) {
    let c = style::colors(ui.ctx());
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.themes")).font(style::bold(16.0)));
        let o = Opts::key(app, "VaultTheme").range(0.0, 3.0);
        let choices: Vec<(i64, String)> = (0..=3)
            .map(|i| (i, app.t(&format!("vault.theme.{i}"))))
            .collect();
        widgets::int_choice_field(app, ui, &p(&["vault", "VaultTheme"]), o, &choices);
        ui.add_space(6.0);
        ui.label(RichText::new(app.t("vault.eventThemes")).font(style::bold(14.0)));
        let ev = app.open.as_ref().and_then(|o| {
            o.doc
                .root()
                .at(&p(&["specialTheme", "eventsThemes"]))
                .and_then(Value::as_object)
                .cloned()
        });
        if let Some(ev) = ev {
            for (k, _) in &ev.0 {
                let key = k.text();
                let name = catalog()
                    .event_themes
                    .iter()
                    .find(|t| t.id == key)
                    .map_or_else(
                        || key.clone(),
                        |t| {
                            t.name
                                .get(app.lang == shelter_core::i18n::Lang::Ru)
                                .to_string()
                        },
                    );
                egui::CollapsingHeader::new(name)
                    .id_salt(("ev", key.clone()))
                    .show(ui, |ui| {
                        let base = p(&["specialTheme", "eventsThemes", &key, "extraData"]);
                        for (f, l) in [
                            ("partsCollectedCount", "theme.parts"),
                            ("IsCraftingInProgress", "theme.crafting"),
                            ("IsCrafted", "theme.crafted"),
                            ("IsClaimed", "theme.claimed"),
                            ("IsClaimedInCraftingRoom", "theme.claimedCraft"),
                            ("IsNew", "theme.new"),
                        ] {
                            let mut path = base.clone();
                            path.push(Seg::Key(f.into()));
                            let o = Opts::text(app.t(l));
                            if f == "partsCollectedCount" {
                                widgets::num_field(app, ui, &path, o);
                            } else {
                                widgets::bool_field(app, ui, &path, o);
                            }
                        }
                    });
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new(app.t("vault.roomThemes")).font(style::bold(14.0)));
        widgets::object_form(app, ui, &p(&["specialTheme", "themeByRoomType"]), 0);
    });
    ui.add_space(10.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.unlocks")).font(style::bold(16.0)));
        let Some(open) = &app.open else { return };
        let root = open.doc.root();
        let claimed = vault::arr(root, &["unlockableMgr", "claimed"]).len();
        let recipes = vault::arr(root, &["survivalW", "recipes"]).len();
        ui.label(app.tf(
            "vault.unlockedRooms",
            &[
                ("n", claimed.to_string()),
                ("total", catalog().unlockables.len().to_string()),
            ],
        ));
        ui.label(app.tf("vault.recipes", &[("n", recipes.to_string())]));
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(format!("{}  {}", icons::LOCK, app.t("vault.unlockRooms")))
                .clicked()
            {
                app.act(Action::UnlockAllRooms);
            }
            if ui
                .button(format!(
                    "{}  {}",
                    icons::SCROLL,
                    app.t("vault.unlockRecipes")
                ))
                .clicked()
            {
                app.act(Action::UnlockAllRecipes);
            }
            if ui
                .button(format!(
                    "{}  {}",
                    icons::BADGE,
                    app.t("vault.completeCollection")
                ))
                .clicked()
            {
                app.act(Action::CompleteCollection);
            }
        });
    });
    ui.add_space(10.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.emergency")).font(style::bold(16.0)));
        let active = app
            .open
            .as_ref()
            .and_then(|o| vault::flag(o.doc.root(), &["vault", "emergencyData", "active"]))
            .unwrap_or(false);
        if active {
            ui.label(
                RichText::new(format!(
                    "{}  {}",
                    icons::ALERT,
                    app.t("vault.emergencyActive")
                ))
                .color(c.danger),
            );
            if ui.button(app.t("vault.finishEmergency")).clicked() {
                app.act(Action::FinishEmergency);
            }
        } else {
            ui.label(RichText::new(app.t("vault.emergencyNone")).color(c.muted));
        }
        widgets::object_form(app, ui, &p(&["vault", "emergencyData"]), 0);
    });
}
