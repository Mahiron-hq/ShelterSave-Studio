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

use std::collections::BTreeMap;

use eframe::egui::{self, RichText, Sense};
use shelter_core::actions::Action;
use shelter_core::catalog::{catalog, ItemKind};
use shelter_core::fields;
use shelter_core::json::{Seg, Value};
use shelter_core::map::{self, PlaceError};
use shelter_core::rules::{Mode, RESOURCE_MAX};
use shelter_core::vault::{self, p, pi, RESOURCE_KEYS};

use crate::app::{panel, section_title, App, VaultTab};
use crate::icons;
use crate::style;
use crate::widgets::{self, Opts};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    section_title(ui, icons::HOUSE, &app.t("nav.vault"), "");
    ui.horizontal_wrapped(|ui| {
        for (tab, key) in [
            (VaultTab::Resources, "vault.tab.resources"),
            (VaultTab::Storage, "vault.tab.storage"),
            (VaultTab::Rooms, "vault.tab.rooms"),
            (VaultTab::Themes, "vault.tab.themes"),
            (VaultTab::Book, "vault.tab.book"),
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
                .id_salt("vault_resources")
                .auto_shrink(false)
                .show(ui, |ui| resources(app, ui))
                .inner
        }
        VaultTab::Storage => storage(app, ui),
        VaultTab::Rooms => rooms(app, ui),
        VaultTab::Themes => {
            egui::ScrollArea::vertical()
                .id_salt("vault_themes")
                .auto_shrink(false)
                .show(ui, |ui| themes(app, ui))
                .inner
        }
        VaultTab::Book => {
            egui::ScrollArea::vertical()
                .id_salt("vault_book")
                .auto_shrink(false)
                .show(ui, |ui| book(app, ui))
                .inner
        }
    }
}

fn resources(app: &mut App, ui: &mut egui::Ui) {
    for (bonus, title, help) in [
        (false, "vault.resources", "vault.resourcesHelp"),
        (true, "vault.bonus", "vault.bonusHelp"),
    ] {
        if bonus && !app.expert() {
            continue;
        }
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
                let cur = app.resource_all;
                if let Some(v) = widgets::drag_number(
                    app,
                    ui,
                    "resource_all",
                    cur,
                    (0.0, RESOURCE_MAX as f64),
                    true,
                ) {
                    app.resource_all = v;
                }
                if ui.button(app.t("c.apply")).clicked() {
                    let v = app.resource_all;
                    app.act(Action::SetAllResources { bonus, value: v });
                }
            });
            let expert = app.expert();
            for key in RESOURCE_KEYS {
                if !expert && !vault::PLAYER_RESOURCES.contains(&key) {
                    continue;
                }
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
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.lunchboxes")).font(style::bold(16.0)));
        ui.label(
            RichText::new(app.t("vault.lunchboxesHelp"))
                .small()
                .color(style::colors(ui.ctx()).muted),
        );
        for i in 0..counts.len() {
            // Same frame and columns as the resource rows above.
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_top(|ui| {
                        let label_w = 240.0_f32.min(ui.available_width() * 0.4);
                        ui.allocate_ui_with_layout(
                            egui::vec2(label_w, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(label_w);
                                widgets::hint_label(
                                    ui,
                                    &app.t(&format!("vault.lb.{i}")),
                                    &app.t(&format!("vault.lbHelp.{i}")),
                                );
                            },
                        );
                        let max = shelter_core::rules::LUNCHBOX_MAX as f64;
                        let cur = counts[i] as f64;
                        ui.horizontal_wrapped(|ui| {
                            if let Some(v) = widgets::pick_number(
                                app,
                                ui,
                                &format!("lunch:{i}"),
                                cur,
                                (0.0, max.max(cur)),
                                Some((0.0, max.max(cur))),
                                true,
                            ) {
                                let mut next = counts;
                                next[i] = v as i64;
                                app.act(Action::SetLunchboxes { counts: next });
                            }
                        });
                    });
                });
        }
        if other > 0 {
            ui.label(RichText::new(app.tf("vault.lb.other", &[("n", other.to_string())])).small());
        }
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
    catalog().rarity_name(r, app.lang == shelter_core::i18n::Lang::Ru)
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
        ui.label(RichText::new(app.t("it.type")).color(c.muted));
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
        ui.label(RichText::new(app.t("it.rarity")).color(c.muted));
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
                for r in ["Normal", "Rare", "Legendary"] {
                    if ui
                        .selectable_label(app.st.rarity.as_deref() == Some(r), rarity_label(app, r))
                        .clicked()
                    {
                        app.st.rarity = Some(r.to_string());
                    }
                }
            });
        widgets::unit_label(ui, &app.tf("it.total", &[("n", total.to_string())]));
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
            if app
                .st
                .rarity
                .as_ref()
                .is_some_and(|r| *r != rarity && !(r == "Normal" && rarity == "Common"))
            {
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
    egui::ScrollArea::vertical()
        .id_salt("storage_rows")
        .auto_shrink(false)
        .show_rows(ui, row_h, rows.len(), |ui, range| {
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
                    let name_resp = ui.add(
                        egui::Label::new(RichText::new(name).font(style::bold(14.0)))
                            .sense(Sense::hover()),
                    );
                    item_hover(app, name_resp, *k, id);
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
                    if app.expert()
                        && ui
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
        });
}

/// Pet bonus name with its value put in place of the game's `{0}`.
pub fn bonus_text(name: &str, value: &str) -> String {
    if name.contains("{0}") {
        name.replace("{0}", value)
    } else {
        format!("{name}: {value}")
    }
}

/// Card shown after a short hover over an item: game picture and item facts.
pub fn item_hover(app: &mut App, resp: egui::Response, kind: ItemKind, id: &str) -> egui::Response {
    resp.on_hover_ui(|ui| item_card(app, ui, kind, id))
}

fn item_card(app: &mut App, ui: &mut egui::Ui, kind: ItemKind, id: &str) {
    let c = style::colors(ui.ctx());
    let ru = app.settings.russian_item_names;
    let (name, rarity, _) = item_name(app, kind, id);
    let status = app.art.status();
    let icon = app.art.icon(ui.ctx(), kind.as_str(), id);
    let has_icon = catalog()
        .art
        .as_ref()
        .is_some_and(|a| a.icons.contains_key(&format!("{}:{id}", kind.as_str())));
    ui.set_max_width(340.0);
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
    ui.horizontal_top(|ui| {
        if let Some(t) = &icon {
            let size = t.size_vec2();
            let k = (120.0 / size.x.max(size.y)).min(1.0);
            ui.add(egui::Image::new(t).fit_to_exact_size(size * k));
        } else if matches!(
            status,
            crate::art::Status::Ready | crate::art::Status::Loading
        ) && has_icon
        {
            ui.spinner();
        }
        ui.vertical(|ui| {
            ui.label(RichText::new(&name).font(style::bold(15.0)));
            let mut sub = app.t(&format!("it.kind.{}", kind.as_str()));
            if !rarity.is_empty() {
                sub = format!("{sub}  ·  {}", rarity_label(app, &rarity));
            }
            ui.label(RichText::new(sub).small().color(c.muted));
            match kind {
                ItemKind::Weapon => {
                    if let Some(w) = catalog().weapon(id) {
                        ui.label(app.tf(
                            "card.damage",
                            &[
                                ("min", w.damage_min.to_string()),
                                ("max", w.damage_max.to_string()),
                            ],
                        ));
                    }
                }
                ItemKind::Outfit => {
                    if let Some(o) = catalog().outfit(id) {
                        let parts: Vec<String> = vault::SPECIAL_LETTERS
                            .iter()
                            .zip(o.special)
                            .filter(|(_, v)| *v != 0)
                            .map(|(l, v)| format!("{l} {v:+}"))
                            .collect();
                        if !parts.is_empty() {
                            ui.label(app.tf("card.special", &[("list", parts.join("  "))]));
                        }
                    }
                }
                ItemKind::Pet => {
                    if let Some(p) = catalog().pet(id) {
                        let bonus = catalog()
                            .pet_bonuses
                            .get(&p.bonus)
                            .map_or(p.bonus.clone(), |n| n.get(ru).to_string());
                        let range = format!(
                            "{}–{}",
                            widgets::fmt_num(p.bonus_min),
                            widgets::fmt_num(p.bonus_max)
                        );
                        ui.label(bonus_text(&bonus, &range));
                    }
                }
                ItemKind::Junk => {}
            }
        });
    });
    let desc = match kind {
        ItemKind::Weapon => catalog().weapon(id).and_then(|w| w.desc.clone()),
        ItemKind::Outfit => catalog().outfit(id).and_then(|o| o.desc.clone()),
        ItemKind::Pet => catalog().pet(id).and_then(|p| p.desc.clone()),
        ItemKind::Junk => catalog()
            .junk
            .iter()
            .find(|j| j.id == id)
            .and_then(|j| j.desc.clone()),
    };
    if let Some(d) = desc {
        let text = d
            .get(app.lang == shelter_core::i18n::Lang::Ru)
            .replace("\n", " ");
        ui.add(egui::Label::new(RichText::new(text).color(c.muted)).wrap());
    }
    let (craft, sell) = match kind {
        ItemKind::Weapon => catalog()
            .weapon(id)
            .map_or((None, 0), |w| (w.craft.as_ref(), w.sell)),
        ItemKind::Outfit => catalog()
            .outfit(id)
            .map_or((None, 0), |o| (o.craft.as_ref(), o.sell)),
        ItemKind::Junk => (
            None,
            catalog()
                .junk
                .iter()
                .find(|j| j.id == id)
                .map_or(0, |j| j.sell),
        ),
        ItemKind::Pet => (None, 0),
    };
    price_lines(app, ui, craft, sell, false);
    if icon.is_none() && has_icon {
        let note = match status {
            crate::art::Status::NoGame => Some(app.t("art.noGame")),
            crate::art::Status::Mismatch => Some(app.t("art.mismatch")),
            crate::art::Status::Ready | crate::art::Status::Loading => None,
        };
        if let Some(n) = note {
            ui.add(egui::Label::new(RichText::new(n).small().color(c.muted)).wrap());
        }
    }
    if app.expert() {
        ui.label(RichText::new(id).font(style::mono(11.0)).color(c.muted));
    }
}

/// Crafting cost (caps and junk) and sell price, as the game shows them.
fn price_lines(
    app: &App,
    ui: &mut egui::Ui,
    craft: Option<&shelter_core::catalog::Craft>,
    sell: i64,
    theme: bool,
) {
    let c = style::colors(ui.ctx());
    let ru = app.settings.russian_item_names;
    if let Some(cr) = craft {
        let junk: Vec<String> = cr
            .junk
            .iter()
            .map(|(id, n)| {
                let name = catalog()
                    .junk
                    .iter()
                    .find(|j| &j.id == id)
                    .map_or_else(|| id.clone(), |j| j.name.get(ru).to_string());
                format!("{name} × {n}")
            })
            .collect();
        ui.label(app.tf("card.craft", &[("caps", widgets::fmt_num(cr.caps as f64))]));
        if !junk.is_empty() {
            ui.add(egui::Label::new(RichText::new(junk.join(", ")).small().color(c.muted)).wrap());
        }
        if theme {
            ui.label(
                RichText::new(app.t("card.craftTheme"))
                    .small()
                    .color(c.muted),
            );
        }
    }
    if sell > 0 {
        ui.label(app.tf("card.sell", &[("caps", widgets::fmt_num(sell as f64))]));
    }
}

/// Card shown after a short hover over a room theme.
pub fn theme_hover(app: &mut App, resp: egui::Response, id: &str) -> egui::Response {
    resp.on_hover_ui(|ui| theme_card(app, ui, id))
}

fn theme_card(app: &mut App, ui: &mut egui::Ui, id: &str) {
    let c = style::colors(ui.ctx());
    let Some(t) = catalog().theme(id) else {
        return;
    };
    let icon = app.art.icon(ui.ctx(), "Theme", id);
    let has_icon = catalog()
        .art
        .as_ref()
        .is_some_and(|a| a.icons.contains_key(&format!("Theme:{id}")));
    let state = app
        .open
        .as_ref()
        .map(|o| shelter_core::actions::theme_state(o.doc.root(), id));
    ui.set_max_width(340.0);
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
    ui.horizontal_top(|ui| {
        if let Some(tex) = &icon {
            let size = tex.size_vec2();
            let k = (120.0 / size.x.max(size.y)).min(1.0);
            ui.add(egui::Image::new(tex).fit_to_exact_size(size * k));
        } else if has_icon && app.art.status() != crate::art::Status::NoGame {
            ui.spinner();
        }
        ui.vertical(|ui| {
            ui.label(RichText::new(theme_name(app, id)).font(style::bold(15.0)));
            let room = if t.room == "FakeWasteland" {
                app.t("theme.exterior")
            } else {
                kind_name(app, &t.room, 1)
            };
            ui.label(
                RichText::new(app.tf("card.themeFor", &[("room", room)]))
                    .small()
                    .color(c.muted),
            );
            if let Some((parts, crafted)) = state {
                let text = if crafted {
                    app.t("card.themeCrafted")
                } else {
                    app.tf("card.themeParts", &[("n", parts.to_string())])
                };
                ui.label(text);
            }
        });
    });
    let (kind, pic) = theme_picture(&t.room, Some(&t.theme));
    room_picture(app, ui, &kind, &pic, 340.0);
    price_lines(app, ui, t.craft.as_ref(), 0, true);
    if app.expert() {
        ui.label(RichText::new(id).font(style::mono(11.0)).color(c.muted));
    }
}

/// Search box with a list of reference items; returns the picked id.
pub fn item_picker(
    app: &mut App,
    ui: &mut egui::Ui,
    id_salt: &str,
    kinds: &[ItemKind],
    query: &mut String,
) -> Option<(ItemKind, String)> {
    item_picker_for(app, ui, id_salt, kinds, query, None)
}

/// Same, offering only outfits a dweller of `gender` can wear.
pub fn item_picker_for(
    app: &mut App,
    ui: &mut egui::Ui,
    id_salt: &str,
    kinds: &[ItemKind],
    query: &mut String,
    gender: Option<i64>,
) -> Option<(ItemKind, String)> {
    let c = style::colors(ui.ctx());
    let resp = ui.add(
        egui::TextEdit::singleline(query)
            .hint_text(app.t("it.pick"))
            .desired_width(ui.available_width().min(280.0)),
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
            let mut found = catalog().search(kinds, query);
            if let Some(g) = gender {
                found.retain(|i| i.kind != ItemKind::Outfit || catalog().outfit_fits(i.id, g));
            }
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for i in found.iter().take(200) {
                        let name = i.name.get(app.settings.russian_item_names);
                        let text = format!("{name}  ·  {}", rarity_label(app, i.rarity));
                        let r = ui.selectable_label(false, text);
                        let r = item_hover(app, r, i.kind, i.id);
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
        ui.add(
            egui::Label::new(
                RichText::new(app.t("vault.addHelp"))
                    .small()
                    .color(style::colors(ui.ctx()).muted),
            )
            .wrap(),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(app.t("it.type"));
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
            ui.label(app.t("vault.quantity"));
            ui.add(
                egui::DragValue::new(&mut app.st.add_count)
                    .range(1..=shelter_core::rules::ITEM_ADD_MAX),
            );
            let mut q = std::mem::take(&mut app.st.add_query);
            let picked = item_picker(app, ui, "add_item", &kinds, &mut q);
            app.st.add_query = q;
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

fn level_names(app: &App, kind: &str) -> Vec<String> {
    app.tx().level_names(kind)
}

fn kind_name(app: &App, kind: &str, level: i64) -> String {
    app.tx().kind_name(kind, level)
}

pub fn room_title(app: &App, r: &Value) -> String {
    app.tx().room_title(r)
}

fn room_name(app: &App, r: &Value) -> String {
    app.tx().room_name(r)
}

/// Room being placed from the map: the clicked cell and the chosen room.
#[derive(Clone, Debug)]
pub struct MapAdd {
    pub row: i64,
    pub at: i64,
    pub kind: String,
    pub merge: i64,
    pub level: i64,
    /// Place rocks instead of a room.
    pub rock: bool,
    /// Opened as a copy of an existing room.
    pub copy: bool,
}

/// Room dragged on the map and the cell of it under the pointer.
#[derive(Clone, Copy, Debug)]
pub struct MapDrag {
    pub room: i64,
    pub grab: i64,
}

enum Spot {
    Ok,
    Warn(&'static str),
    Bad(&'static str),
}

fn spot(
    app: &App,
    root: &Value,
    kind: &str,
    row: i64,
    col: i64,
    merge: i64,
    id: Option<i64>,
) -> Spot {
    match map::check_spot(root, kind, row, col, merge, id) {
        Ok(()) => Spot::Ok,
        Err(PlaceError::Disconnected) if app.expert() => Spot::Warn("map.disconnectedExpert"),
        Err(PlaceError::Disconnected) => Spot::Bad("map.disconnected"),
        Err(PlaceError::Overlap) => Spot::Bad("map.overlap"),
        Err(PlaceError::OutOfGrid) => Spot::Bad("map.outOfGrid"),
    }
}

fn add_col(root: &Value, add: &MapAdd) -> i64 {
    map::add_col(root, &add.kind, add.row, add.at, add.merge, add.rock)
}

fn buildable(app: &App, root: &Value) -> Vec<(String, String)> {
    let built = map::rooms(root);
    let mut out: Vec<(String, String)> = catalog()
        .rooms
        .iter()
        .filter(|r| {
            if map::FIXED_ROOMS.contains(&r.kind.as_str()) {
                return false;
            }
            if app.expert() {
                return true;
            }
            !map::NOT_BUILDABLE.contains(&r.kind.as_str())
                && !(map::UNIQUE_ROOMS.contains(&r.kind.as_str())
                    && built.iter().any(|b| b.kind == r.kind))
        })
        .map(|r| (r.kind.clone(), kind_name(app, &r.kind, 1)))
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

fn merges(kind: &str) -> Vec<i64> {
    map::merges(kind)
}

fn max_level(kind: &str, merge: i64) -> i64 {
    map::max_level(kind, merge)
}

fn map_view(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let c = style::colors(ui.ctx());
    let (grid_rows, grid_cols) = map::grid_size();
    let list = map::rooms(root);
    let blocks = map::obstacles(root);
    let burning: std::collections::HashSet<i64> = vault::arr(root, &vault::ROOMS)
        .iter()
        .filter(|r| vault::text(r, &["currentStateName"]).as_deref() == Some("RoomEmergency"))
        .filter_map(|r| vault::int(r, &["deserializeID"]))
        .collect();
    let used = list
        .iter()
        .map(|r| r.row)
        .chain(blocks.iter().map(|b| b.0))
        .max()
        .unwrap_or(0);
    let shown = (used + 2).clamp(1, grid_rows);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.map")).font(style::bold(16.0)));
        ui.label(RichText::new(app.t("vault.mapHelp")).small().color(c.muted));
        ui.add_space(4.0);
        let cell = ((ui.available_width() - 4.0) / grid_cols as f32).clamp(11.0, 40.0);
        let h = (cell * 1.6).max(34.0);
        let (rect, bg) = ui.allocate_exact_size(
            egui::vec2(cell * grid_cols as f32, h * shown as f32),
            Sense::click(),
        );
        let painter = ui.painter_at(rect);
        let span = |row: i64, col: i64, w: i64| {
            egui::Rect::from_min_size(
                rect.min + egui::vec2(col as f32 * cell, row as f32 * h),
                egui::vec2(cell * w as f32, h),
            )
            .shrink(1.0)
        };
        let to_cell = |pos: egui::Pos2| {
            (
                ((pos.y - rect.min.y) / h).floor() as i64,
                ((pos.x - rect.min.x) / cell).floor() as i64,
            )
        };
        let inside =
            |(row, col): (i64, i64)| (0..shown).contains(&row) && (0..grid_cols).contains(&col);
        painter.rect_filled(rect, 4.0, c.bg);
        let line = egui::Stroke::new(1.0, c.border.gamma_multiply(0.35));
        for row in 1..shown {
            let y = rect.min.y + row as f32 * h;
            painter.hline(rect.x_range(), y, line);
        }
        for col in 1..grid_cols {
            let x = rect.min.x + col as f32 * cell;
            painter.vline(x, rect.y_range(), line);
        }
        for (r, col) in &blocks {
            let rr = span(*r, *col, 2);
            let sel = app.rock_sel == Some((*r, *col));
            painter.rect_filled(rr, 2.0, c.border);
            painter.rect_stroke(
                rr,
                2.0,
                egui::Stroke::new(
                    if sel { 2.0 } else { 1.0 },
                    if sel {
                        c.focus
                    } else {
                        c.muted.gamma_multiply(0.5)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let resp = ui
                .interact(rr, ui.scope_id().with(("rock", *r, *col)), Sense::click())
                .on_hover_text(app.t("map.rockHint"));
            if resp.clicked() {
                app.rock_sel = Some((*r, *col));
                app.room_sel = None;
                app.map_add = None;
            }
        }
        let taken = map::occupancy(root);
        let blocked = |row: i64, col: i64| {
            taken.contains_key(&(row, col))
                || blocks
                    .iter()
                    .any(|(r, c)| *r == row && (*c..*c + 2).contains(&col))
        };

        if app.map_drag.is_none() {
            if let Some(pos) = bg.hover_pos() {
                let (row, col) = to_cell(pos);
                if inside((row, col)) && !blocked(row, col) {
                    painter.rect_filled(span(row, col, 1), 2.0, c.accent_soft);
                }
            }
        }
        if bg.clicked() {
            if let Some(pos) = bg.interact_pointer_pos() {
                let (row, col) = to_cell(pos);
                if inside((row, col)) && !blocked(row, col) {
                    let prev = app
                        .map_add
                        .take()
                        .filter(|a| !a.copy)
                        .or_else(|| app.last_add.clone());
                    app.map_add = Some(MapAdd {
                        row,
                        at: col,
                        kind: prev
                            .as_ref()
                            .map_or_else(|| "LivingQuarters".to_string(), |a| a.kind.clone()),
                        merge: prev.as_ref().map_or(1, |a| a.merge),
                        level: prev.as_ref().map_or(1, |a| a.level),
                        rock: prev.as_ref().is_some_and(|a| a.rock),
                        copy: false,
                    });
                    app.room_sel = None;
                    app.rock_sel = None;
                }
            }
        }

        let mut drop = None;
        for r in &list {
            let rr = span(r.row, r.col, r.width);
            let class = catalog()
                .room(&r.kind)
                .map_or_else(String::new, |i| i.class.clone());
            let sel = app.room_sel == Some(r.id);
            let dragging = app.map_drag.is_some_and(|d| d.room == r.id);
            let fill = if r.is_elevator() {
                c.surface_alt
            } else {
                class_color(&c, &class)
            };
            painter.rect_filled(
                rr,
                3.0,
                if dragging {
                    fill.gamma_multiply(0.4)
                } else {
                    fill
                },
            );
            let on_fire = burning.contains(&r.id);
            painter.rect_stroke(
                rr,
                3.0,
                egui::Stroke::new(
                    if sel || on_fire { 2.0 } else { 1.0 },
                    if sel {
                        c.focus
                    } else if on_fire {
                        c.danger
                    } else {
                        c.border
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let mut name = kind_name(app, &r.kind, r.level);
            if on_fire {
                name = format!("{} {name}", icons::ALERT);
            }
            let level_text = (!r.is_elevator() && max_level(&r.kind, r.merge) > 1)
                .then(|| app.tf("map.lvl", &[("n", r.level.to_string())]));
            room_label(&painter, rr, &name, level_text.as_deref(), h, &c);
            let fixed = map::FIXED_ROOMS.contains(&r.kind.as_str());
            let sense = if fixed {
                Sense::click()
            } else {
                Sense::click_and_drag()
            };
            let hint = app.t(if fixed { "map.fixed" } else { "map.dragHint" });
            let resp = ui
                .interact(rr, ui.scope_id().with(("room", r.id)), sense)
                .on_hover_text(format!("{name}\n{hint}"));
            if !fixed && (resp.hovered() || resp.dragged()) {
                ui.ctx().set_cursor_icon(if resp.dragged() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
            }
            if resp.clicked() {
                app.room_sel = Some(r.id);
                app.map_add = None;
                app.rock_sel = None;
            }
            if resp.drag_started() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    app.map_drag = Some(MapDrag {
                        room: r.id,
                        grab: (to_cell(pos).1 - r.col).clamp(0, r.width - 1),
                    });
                    app.room_sel = Some(r.id);
                    app.map_add = None;
                    app.rock_sel = None;
                }
            }
            if resp.drag_stopped() {
                drop = Some(r.id);
            }
        }

        let ghost = |painter: &egui::Painter, rr: egui::Rect, s: &Spot| {
            let (fill, stroke) = match s {
                Spot::Ok => (c.success_soft, c.success),
                Spot::Warn(_) => (c.warning_soft, c.warning),
                Spot::Bad(_) => (c.danger_soft, c.danger),
            };
            painter.rect_filled(rr, 3.0, fill.gamma_multiply(0.85));
            painter.rect_stroke(
                rr,
                3.0,
                egui::Stroke::new(2.0, stroke),
                egui::StrokeKind::Inside,
            );
        };

        if let Some(d) = app.map_drag {
            let target = ui
                .ctx()
                .pointer_latest_pos()
                .map(to_cell)
                .and_then(|(row, col)| {
                    list.iter()
                        .find(|r| r.id == d.room)
                        .map(|r| (r, row, col - d.grab))
                });
            if let Some((r, row, col)) = target {
                let s = spot(app, root, &r.kind, row, col, r.merge, Some(r.id));
                if (row, col) != (r.row, r.col) {
                    ghost(&painter, span(row, col, r.width), &s);
                }
                if drop == Some(r.id) {
                    app.map_drag = None;
                    if (row, col) != (r.row, r.col) {
                        app.act(Action::MoveRoom {
                            room: r.id,
                            row,
                            col,
                            apart: app.keep_apart,
                        });
                    }
                }
            }
            if drop.is_some() || !ui.ctx().input(|i| i.pointer.any_down()) {
                app.map_drag = None;
            }
        }

        if let Some(add) = app.map_add.clone() {
            let col = add_col(root, &add);
            let (s, w) = if add.rock {
                let s = match map::fits(root, add.row, col, 2, None) {
                    Ok(()) => Spot::Ok,
                    Err(_) => Spot::Bad("map.overlap"),
                };
                (s, 2)
            } else {
                (
                    spot(app, root, &add.kind, add.row, col, add.merge, None),
                    map::width_of(&add.kind, add.merge),
                )
            };
            ghost(&painter, span(add.row, col, w), &s);
        }
    });
}

/// Room name inside its box on the map: wrapped to the box and cut with an
/// ellipsis, with the upgrade level under it or beside it.
fn room_label(
    painter: &egui::Painter,
    rr: egui::Rect,
    name: &str,
    level: Option<&str>,
    h: f32,
    c: &style::Colors,
) {
    let inner = rr.shrink2(egui::vec2(5.0, 3.0));
    if inner.width() < 16.0 {
        return;
    }
    let size = (h * 0.3).clamp(9.0, 12.5);
    let line_h = size * 1.25;
    let rows_fit = ((inner.height() / line_h).floor() as usize).max(1);
    let level_g = level.map(|l| {
        painter.layout_no_wrap(
            l.to_string(),
            egui::FontId::proportional((size - 1.5).max(8.5)),
            c.muted,
        )
    });
    let mut level_g = level_g;
    if rows_fit < 2
        && level_g
            .as_ref()
            .is_some_and(|g| inner.width() - g.size().x < 48.0)
    {
        level_g = None;
    }
    let stacked = level_g.is_some() && rows_fit >= 2;
    let name_w = match (&level_g, stacked) {
        (Some(g), false) => inner.width() - g.size().x - 4.0,
        _ => inner.width(),
    };
    if name_w < 12.0 {
        return;
    }
    let mut job = egui::text::LayoutJob::simple(
        name.to_string(),
        egui::FontId::proportional(size),
        c.text,
        name_w,
    );
    let first_word = name.split_whitespace().next().unwrap_or(name);
    let word_w = painter
        .layout_no_wrap(
            first_word.to_string(),
            egui::FontId::proportional(size),
            c.text,
        )
        .size()
        .x;
    job.wrap.max_rows = if word_w > name_w {
        1
    } else if stacked {
        rows_fit - 1
    } else {
        rows_fit
    };
    job.wrap.overflow_character = Some('…');
    let name_g = painter.layout_job(job);
    let total_h = name_g.size().y
        + if stacked {
            level_g.as_ref().map_or(0.0, |g| g.size().y)
        } else {
            0.0
        };
    let top = inner.center().y - total_h / 2.0;
    let painter = painter.with_clip_rect(rr);
    painter.galley(egui::pos2(inner.left(), top), name_g.clone(), c.text);
    if let Some(g) = level_g {
        let pos = if stacked {
            egui::pos2(inner.left(), top + name_g.size().y)
        } else {
            egui::pos2(
                inner.right() - g.size().x,
                inner.center().y - g.size().y / 2.0,
            )
        };
        painter.galley(pos, g, c.muted);
    }
}

fn add_panel(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let Some(mut add) = app.map_add.clone() else {
        return;
    };
    let c = style::colors(ui.ctx());
    let kinds = buildable(app, root);
    if !kinds.iter().any(|(k, _)| *k == add.kind) {
        if let Some((k, _)) = kinds.first() {
            add.kind = k.clone();
        }
    }
    let sizes = merges(&add.kind);
    if !sizes.contains(&add.merge) {
        add.merge = sizes
            .iter()
            .copied()
            .min_by_key(|m| (m - add.merge).abs())
            .unwrap_or(1);
    }
    add.level = add.level.clamp(1, max_level(&add.kind, add.merge));
    let col = add_col(root, &add);
    let locked = !add.rock && shelter_core::actions::room_lock(root, &add.kind).is_some();
    let mut build = false;
    let mut cancel = false;
    panel(ui, |ui| {
        let heading = if add.copy { "map.copy" } else { "map.new" };
        ui.label(RichText::new(app.t(heading)).font(style::bold(16.0)));
        ui.label(
            RichText::new(app.tf(
                "map.position",
                &[("row", add.row.to_string()), ("col", col.to_string())],
            ))
            .small()
            .color(c.muted),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(!add.rock, app.t("map.placeRoom"))
                .clicked()
            {
                add.rock = false;
            }
            if ui
                .selectable_label(add.rock, app.t("map.placeRock"))
                .on_hover_text(app.t("map.placeRockHelp"))
                .clicked()
            {
                add.rock = true;
            }
        });
        ui.add_space(4.0);
        if add.rock {
            ui.add(
                egui::Label::new(RichText::new(app.t("map.placeRockHelp")).color(c.muted)).wrap(),
            );
        } else {
            widgets::hint_label(ui, &app.t("map.type"), &app.t("map.typeHelp"));
            let width = ui.available_width().min(300.0);
            egui::ComboBox::from_id_salt("map_add_kind")
                .selected_text(kind_name(app, &add.kind, add.level))
                .width(width)
                .height(420.0)
                .show_ui(ui, |ui| {
                    ui.set_max_width(width.max(260.0));
                    for (k, _) in &kinds {
                        let names = level_names(app, k);
                        let mut job = egui::text::LayoutJob::default();
                        job.append(
                            &names.first().cloned().unwrap_or_else(|| k.clone()),
                            0.0,
                            egui::TextFormat::simple(egui::FontId::proportional(14.0), c.text),
                        );
                        if names.len() > 1 {
                            job.append(
                                &format!("\n{}", names[1..].join(" · ")),
                                0.0,
                                egui::TextFormat::simple(egui::FontId::proportional(12.0), c.muted),
                            );
                        }
                        if shelter_core::actions::room_lock(root, k).is_some() {
                            job.append(
                                &format!("\n{}  {}", icons::LOCK, app.t("map.lockedShort")),
                                0.0,
                                egui::TextFormat::simple(
                                    egui::FontId::proportional(12.0),
                                    c.warning,
                                ),
                            );
                        }
                        if ui
                            .add(egui::Button::selectable(*k == add.kind, job).wrap())
                            .clicked()
                        {
                            add.kind = k.clone();
                        }
                    }
                });
            if sizes.len() > 1 {
                ui.add_space(4.0);
                widgets::hint_label(ui, &app.t("map.size"), &app.t("map.sizeHelp"));
                ui.horizontal(|ui| {
                    for m in sizes.iter().copied() {
                        if ui
                            .add(egui::Button::selectable(add.merge == m, m.to_string()))
                            .clicked()
                        {
                            add.merge = m;
                        }
                    }
                });
            }
            if max_level(&add.kind, add.merge) > 1 {
                ui.add_space(4.0);
                let f = fields::describe_key(app.lang, Some("rooms"), "level");
                widgets::hint_label(ui, &f.label, &f.help);
                ui.horizontal(|ui| {
                    for l in 1..=max_level(&add.kind, add.merge) {
                        if ui
                            .add(egui::Button::selectable(add.level == l, l.to_string()))
                            .clicked()
                        {
                            add.level = l;
                        }
                    }
                });
            }
        }
        ui.add_space(4.0);
        let s = if add.rock {
            match map::fits(root, add.row, add_col(root, &add), 2, None) {
                Ok(()) => Spot::Ok,
                Err(PlaceError::OutOfGrid) => Spot::Bad("map.outOfGrid"),
                Err(_) => Spot::Bad("map.overlap"),
            }
        } else {
            spot(
                app,
                root,
                &add.kind,
                add.row,
                add_col(root, &add),
                add.merge,
                None,
            )
        };
        let (s_ok, s_build) = if add.rock {
            ("map.rockOk", "map.placeRockButton")
        } else {
            ("map.ok", "map.build")
        };
        let (icon, text, color) = match s {
            Spot::Ok => (icons::CHECK, app.t(s_ok), c.success),
            Spot::Warn(k) => (icons::WARN, app.t(k), c.warning),
            Spot::Bad(k) => (icons::X, app.t(k), c.danger),
        };
        ui.add(egui::Label::new(RichText::new(format!("{icon}  {text}")).color(color)).wrap());
        if locked {
            ui.add_space(4.0);
            let text = app.tf("map.locked", &[("room", kind_name(app, &add.kind, 1))]);
            ui.add(
                egui::Label::new(RichText::new(format!("{}  {text}", icons::LOCK)).color(c.danger))
                    .wrap(),
            );
        }
        ui.horizontal_wrapped(|ui| {
            let label = if locked { "map.unlockBuild" } else { s_build };
            let mut btn = ui.add_enabled(
                !matches!(s, Spot::Bad(_)),
                egui::Button::new(format!("{}  {}", icons::PLUS, app.t(label))),
            );
            if locked {
                btn = btn.on_hover_text(app.t("map.unlockBuildHelp"));
            }
            if btn.clicked() {
                build = true;
            }
            if ui.button(app.t("c.cancel")).clicked() {
                cancel = true;
            }
        });
    });
    if cancel {
        app.map_add = None;
        return;
    }
    let col = add_col(root, &add);
    if build && add.rock {
        if app.act(Action::AddRock { row: add.row, col }) {
            app.last_add = Some(add.clone());
            app.map_add = None;
            app.rock_sel = Some((add.row, col));
        }
        return;
    }
    if build {
        let action = Action::AddRoom {
            kind: add.kind.clone(),
            row: add.row,
            col,
            merge: add.merge,
            level: add.level,
            unlock: locked,
            apart: app.keep_apart,
        };
        if app.act(action) {
            if !add.copy {
                app.last_add = Some(add.clone());
            }
            app.map_add = None;
            app.room_sel = app.open.as_ref().and_then(|o| {
                map::rooms(o.doc.root())
                    .into_iter()
                    .find(|r| r.row == add.row && r.col <= col && col < r.end())
                    .map(|r| r.id)
            });
        }
        return;
    }
    app.map_add = Some(add);
}

fn rooms(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let root = open.doc.shared_root();
    let rooms = vault::arr(&root, &vault::ROOMS);
    let c = style::colors(ui.ctx());
    let _ = rooms;
    let gap = 12.0;
    let full = ui.available_rect_before_wrap();
    let side = (full.width() * 0.32)
        .clamp(250.0, 400.0)
        .min(full.width() * 0.45);
    let map_rect =
        egui::Rect::from_min_max(full.min, egui::pos2(full.max.x - side - gap, full.max.y));
    let side_rect = egui::Rect::from_min_max(egui::pos2(full.max.x - side, full.min.y), full.max);
    ui.scope_builder(egui::UiBuilder::new().max_rect(map_rect), |ui| {
        egui::ScrollArea::both()
            .id_salt("rooms_main")
            .auto_shrink(false)
            .show(ui, |ui| {
                map_view(app, ui, &root);
            });
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(side_rect), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("rooms_side")
            .auto_shrink(false)
            .show(ui, |ui| {
                if app.map_add.is_some() {
                    add_panel(app, ui, &root);
                } else if let Some(rock) = app.rock_sel {
                    rock_panel(app, ui, &root, rock);
                } else {
                    match app.room_sel.and_then(|id| vault::room_index(&root, id)) {
                        Some(i) => room_editor(app, ui, i),
                        None => panel(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(app.t("vault.selectRoom")).color(c.muted),
                                )
                                .wrap(),
                            );
                        }),
                    }
                }
                ui.add_space(10.0);
                rocks(app, ui, &root);
                if app.mode() == Mode::Expert {
                    ui.add_space(10.0);
                    panel(ui, |ui| {
                        let (label, help) = (app.t("room.keepApart"), app.t("room.keepApartHelp"));
                        ui.checkbox(&mut app.keep_apart, label).on_hover_text(help);
                    });
                }
            });
    });
    ui.allocate_rect(full, Sense::hover());
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

fn room_product(kind: &str) -> &'static [&'static str] {
    map::room_product(kind)
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
        let expert = app.expert();
        let head = if expert {
            format!("{}  #{id}", room_name(app, &r))
        } else {
            room_name(app, &r)
        };
        ui.add(egui::Label::new(RichText::new(head).font(style::bold(16.0))).wrap());
        ui.label(
            RichText::new(app.tf(
                "room.position",
                &[
                    ("row", vault::int(&r, &["row"]).unwrap_or(0).to_string()),
                    ("col", vault::int(&r, &["col"]).unwrap_or(0).to_string()),
                ],
            ))
            .small()
            .color(c.muted),
        );
        let root = app.open.as_ref().map(|o| o.doc.shared_root());
        if let Some(root) = &root {
            let (pk, pid) = room_picture_id(root, &r);
            room_picture(app, ui, &pk, &pid, 420.0);
        }
        let fixed = map::FIXED_ROOMS.contains(&kind.as_str());
        if fixed {
            ui.label(RichText::new(app.t("map.fixed")).small().color(c.muted));
        }
        let themed = kind == "FakeWasteland" || catalog().themes.iter().any(|t| t.room == kind);
        if let (true, Some(root)) = (themed, &root) {
            ui.add_space(4.0);
            widgets::hint_label(ui, &app.t("room.theme"), &app.t("room.themeHelp"));
            theme_picker(app, ui, root, &kind);
            let full =
                vault::int(&r, &["mergeLevel"]) == Some(3) && vault::int(&r, &["level"]) == Some(3);
            if kind != "FakeWasteland" && !full {
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("{}  {}", icons::INFO, app.t("room.themeHidden")))
                            .small()
                            .color(c.muted),
                    )
                    .wrap(),
                );
            }
        }
        if r.get("level").is_some()
            && max_level(&kind, vault::int(&r, &["mergeLevel"]).unwrap_or(1)) > 1
        {
            let level_path = at(&["level"]);
            let o = Opts::at(app, &level_path);
            let cur = vault::int(&r, &["level"]).unwrap_or(1);
            widgets::row(app, ui, &level_path, &o, |app, ui| {
                let merge = vault::int(&r, &["mergeLevel"]).unwrap_or(1);
                for l in 1..=max_level(&kind, merge).max(cur) {
                    if ui
                        .add(egui::Button::selectable(l == cur, l.to_string()))
                        .clicked()
                        && l != cur
                    {
                        app.act(Action::SetRoomLevel {
                            room: id,
                            level: l,
                            apart: app.keep_apart,
                        });
                    }
                }
            });
        }
        if kind != "Elevator" && !fixed {
            let sizes = merges(&kind);
            let o = Opts::text(app.t("room.merge")).help(app.t("room.mergeHelp"));
            let merge_path = at(&["mergeLevel"]);
            widgets::row(app, ui, &merge_path, &o, |app, ui| {
                let cur = vault::int(&r, &["mergeLevel"]).unwrap_or(1);
                let mut shown = sizes.clone();
                if !shown.contains(&cur) {
                    shown.push(cur);
                    shown.sort_unstable();
                }
                let mode = app.mode();
                for m in shown {
                    let (fits, free) = app.open.as_ref().map_or((false, false), |o| {
                        let root = o.doc.root();
                        (
                            shelter_core::actions::merge_fits(root, mode, id, m),
                            shelter_core::actions::merge_fits(root, Mode::Expert, id, m),
                        )
                    });
                    let why = if free {
                        app.t("room.mergeCuts")
                    } else {
                        app.t("room.mergeNoSpace")
                    };
                    if ui
                        .add_enabled(
                            fits || m == cur,
                            egui::Button::selectable(m == cur, m.to_string()),
                        )
                        .on_disabled_hover_text(why)
                        .clicked()
                        && m != cur
                    {
                        app.act(Action::SetMergeLevel {
                            room: id,
                            merge: m,
                            apart: app.keep_apart,
                        });
                    }
                }
            });
            let cur = vault::int(&r, &["mergeLevel"]).unwrap_or(1);
            let options = shelter_core::actions::split_options(cur);
            if !options.is_empty() {
                let o = Opts::text(app.t("room.split")).help(app.t("room.splitHelp"));
                widgets::row(app, ui, &merge_path, &o, |app, ui| {
                    let mode = app.mode();
                    for parts in options {
                        let err = app.open.as_ref().and_then(|o| {
                            shelter_core::actions::split_error(o.doc.root(), mode, id, &parts)
                        });
                        let why = match err {
                            Some(shelter_core::rules::RuleError::RoomSize) => {
                                app.t("room.splitSize")
                            }
                            Some(shelter_core::rules::RuleError::RoomBusy) => {
                                app.t("room.splitBusy")
                            }
                            _ => String::new(),
                        };
                        let label = parts
                            .iter()
                            .map(i64::to_string)
                            .collect::<Vec<_>>()
                            .join(" + ");
                        if ui
                            .add_enabled(err.is_none(), egui::Button::new(label))
                            .on_disabled_hover_text(why)
                            .clicked()
                        {
                            app.act(Action::SplitRoom {
                                room: id,
                                parts: parts.clone(),
                            });
                        }
                    }
                });
            }
        }
        for (key, label) in [
            ("power", "room.power"),
            ("broken", "room.broken"),
            ("withHole", "room.withHole"),
            ("emergencyDone", "room.emergencyDone"),
        ] {
            if (expert || key != "emergencyDone") && r.get(key).is_some() {
                let mut o = Opts::text(app.t(label));
                if key != "emergencyDone" {
                    o = o.help(app.t(&format!("{label}Help")));
                }
                widgets::bool_field(app, ui, &at(&[key]), o);
            }
        }
        if r.get("onlyIncreaseHappiness").is_some() {
            let o = Opts::text(app.t("room.radioVault")).help(app.t("room.radioVaultHelp"));
            widgets::bool_field(app, ui, &at(&["onlyIncreaseHappiness"]), o);
        }
        if r.get("newDwellerReady").is_some() {
            let o = Opts::text(app.t("room.radioReady")).help(app.t("room.radioReadyHelp"));
            widgets::bool_field(app, ui, &at(&["newDwellerReady"]), o);
        }
        room_timers(app, ui, &r);
        future_children(app, ui, i, &r);
        if let Some(root) = &root {
            crate::sec_other::room_incident(app, ui, root, &r);
        }
        if r.at(&p(&["roomHealth", "damageValue"])).is_some() && (expert || kind == "Entrance") {
            let (label, help) = if kind == "Entrance" {
                ("room.doorDamage", "room.doorDamageHelp")
            } else {
                ("room.damage", "room.damageHelp")
            };
            let o = Opts::text(app.t(label)).help(app.t(help));
            widgets::num_field(app, ui, &at(&["roomHealth", "damageValue"]), o);
        }
        if expert && r.get("currentStateName").is_some() {
            let o = Opts::text(app.t("room.state"));
            let choices = vec![
                ("Idle".to_string(), app.t("room.state.Idle")),
                ("Working".to_string(), app.t("room.state.Working")),
            ];
            widgets::choice_field(app, ui, &at(&["currentStateName"]), o, &choices);
        }
        if expert && r.get("assignedDecoration").is_some() {
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
            let product = room_product(&kind);
            let keys: Vec<String> =
                st.0.iter()
                    .filter(|(k, v)| {
                        let k = k.text();
                        expert
                            || product.contains(&k.as_str())
                            || (RESOURCE_KEYS.contains(&k.as_str())
                                && v.as_f64().is_some_and(|x| x > 0.0))
                    })
                    .map(|(k, _)| k.text())
                    .collect();
            if !keys.is_empty() {
                ui.add_space(6.0);
                widgets::hint_label(ui, &app.t("room.stored"), &app.t("room.storedHelp"));
                let batch = catalog().room(&kind).and_then(|info| {
                    info.batch(
                        vault::int(&r, &["mergeLevel"]).unwrap_or(1),
                        vault::int(&r, &["level"]).unwrap_or(1),
                    )
                });
                for key in keys {
                    let path = at(&["storage", "resources", &key]);
                    match batch.and_then(|b| b.get(&key)).copied() {
                        Some(full) if full >= 1.0 => stored_row(app, ui, id, &key, &path, full),
                        _ => {
                            let o = Opts::text(app.t(&format!("res.{key}")));
                            widgets::num_field(app, ui, &path, o);
                        }
                    }
                }
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
            .map(|o| o.doc.shared_root())
            .unwrap_or_else(|| std::sync::Arc::new(Value::Null));
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
                .width(ui.available_width().min(260.0))
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
        if let Some(me) = map::placed(&r) {
            if !map::merge_neighbours(&map::rooms(&root), &me).is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{}  {}",
                        icons::INFO,
                        app.t("room.mergeNeighbours")
                    ))
                    .small()
                    .color(c.warning),
                );
            }
        }
        if !fixed {
            ui.add_space(8.0);
            let blocked = if app.expert() {
                None
            } else if shelter_core::actions::room_busy(&r) {
                Some(app.t("aerr.roomBusy"))
            } else if !map::can_remove(&root, id) {
                Some(app.t("aerr.roomDisconnected"))
            } else {
                None
            };
            let no_copy = if app.expert() {
                None
            } else if map::UNIQUE_ROOMS.contains(&kind.as_str()) {
                Some(app.t("room.cloneUnique"))
            } else if map::NOT_BUILDABLE.contains(&kind.as_str()) {
                Some(app.t("room.cloneNotBuildable"))
            } else {
                None
            };
            let keys = ui.ctx().egui_wants_keyboard_input();
            let (key_del, key_copy) = if keys {
                (false, false)
            } else {
                ui.ctx().input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                        i.consume_key(egui::Modifiers::COMMAND, egui::Key::D),
                    )
                })
            };
            let mut remove = false;
            let mut copy = false;
            ui.horizontal_wrapped(|ui| {
                let tip = no_copy.clone().unwrap_or_else(|| app.t("room.cloneHelp"));
                copy = ui
                    .add_enabled(
                        no_copy.is_none(),
                        egui::Button::new(format!("{}  {}", icons::COPY, app.t("room.clone"))),
                    )
                    .on_hover_text(&tip)
                    .on_disabled_hover_text(&tip)
                    .clicked()
                    || (key_copy && no_copy.is_none());
                let tip = blocked.clone().unwrap_or_else(|| app.t("room.removeHelp"));
                remove = ui
                    .add_enabled(
                        blocked.is_none(),
                        egui::Button::new(format!("{}  {}", icons::TRASH, app.t("room.remove"))),
                    )
                    .on_hover_text(&tip)
                    .on_disabled_hover_text(&tip)
                    .clicked()
                    || key_del;
            });
            if remove && app.act(Action::RemoveRoom { room: id }) {
                app.room_sel = None;
            }
            if copy {
                start_copy(app, &root, &r);
            }
        }
    });
}

/// Ready-to-collect amount out of a full batch. The game keeps a fraction while
/// the batch fills and rounds down once it is full.
fn stored_row(app: &mut App, ui: &mut egui::Ui, room: i64, key: &str, path: &[Seg], full: f64) {
    let c = style::colors(ui.ctx());
    let Some(cur) = app
        .open
        .as_ref()
        .and_then(|o| o.doc.root().at(path))
        .and_then(Value::as_f64)
    else {
        return;
    };
    let o = Opts::text(app.t(&format!("res.{key}")))
        .help(app.t("room.storedRowHelp"))
        .unit(app.tf("room.storedOf", &[("n", widgets::fmt_num(full))]));
    let o = Opts { derived: true, ..o };
    let shown = cur.floor().min(full);
    widgets::row(app, ui, path, &o, |app, ui| {
        let key_id = format!("stored:{room}:{key}");
        if let Some(v) = widgets::pick_number(
            app,
            ui,
            &key_id,
            shown,
            (0.0, full),
            Some((0.0, full)),
            true,
        ) {
            if v != shown {
                app.act(Action::SetStored {
                    room,
                    resource: key.to_string(),
                    value: v,
                });
            }
        }
    });
    let (text, color) = if cur >= full - 0.005 {
        (app.t("room.storedReady"), c.success)
    } else {
        let pct = ((cur / full) * 100.0).floor().clamp(0.0, 99.0);
        (
            app.tf("room.storedFilling", &[("pct", pct.to_string())]),
            c.muted,
        )
    };
    ui.add(egui::Label::new(RichText::new(text).small().color(color)).wrap());
}

/// Running timers of a room: training, pregnancy, radio.
/// The baby a couple in the living quarters is expecting: the game rolls it
/// when the pregnancy starts and uses these values at birth.
fn future_children(app: &mut App, ui: &mut egui::Ui, i: usize, r: &Value) {
    let partners = vault::arr(r, &["partners"]);
    let c = style::colors(ui.ctx());
    for (k, pr) in partners.iter().enumerate() {
        if !matches!(
            vault::text(pr, &["s"]).as_deref(),
            Some("MakingBaby" | "RaisingBaby")
        ) {
            continue;
        }
        let base = pi(&vault::ROOMS, i, &["partners"]);
        let at = |rest: &[&str]| {
            let mut p = base.clone();
            p.push(Seg::Index(k));
            p.extend(rest.iter().map(|s| Seg::Key((*s).into())));
            p
        };
        let parents = format!(
            "{} {} + {} {}",
            vault::text(pr, &["maleName"]).unwrap_or_default(),
            vault::text(pr, &["maleLast"]).unwrap_or_default(),
            vault::text(pr, &["femaleName"]).unwrap_or_default(),
            vault::text(pr, &["femaleLast"]).unwrap_or_default()
        );
        ui.add_space(6.0);
        widgets::hint_label(ui, &app.t("room.baby"), &app.t("room.babyHelp"));
        ui.label(RichText::new(parents.trim()).small().color(c.muted));
        if pr.get("childRarity").is_some() {
            let o = Opts::text(app.t("room.babyRarity")).help(app.t("room.babyRarityHelp"));
            let choices = vec![
                (
                    1,
                    catalog().rarity_name("Normal", app.lang == shelter_core::i18n::Lang::Ru),
                ),
                (
                    2,
                    catalog().rarity_name("Rare", app.lang == shelter_core::i18n::Lang::Ru),
                ),
                (
                    3,
                    catalog().rarity_name("Legendary", app.lang == shelter_core::i18n::Lang::Ru),
                ),
            ];
            widgets::int_choice_field(app, ui, &at(&["childRarity"]), o, &choices);
        }
        if !vault::arr(pr, &["childStat"]).is_empty() {
            let mut path = at(&["childStat"]);
            path.push(Seg::Index(0));
            let o = Opts::text(app.t("room.babyStat")).help(app.t("room.babyStatHelp"));
            let choices: Vec<(i64, String)> = (1..=7)
                .map(|s| (s, app.t(&format!("dw.specialName.{s}"))))
                .collect();
            widgets::int_choice_field(app, ui, &path, o, &choices);
        }
        if pr.get("pendingChildren").is_some() {
            let o = Opts::text(app.t("room.babyCount")).help(app.t("room.babyCountHelp"));
            let choices = vec![
                (0, app.t("room.babyCountGame")),
                (1, app.t("room.babyCount1")),
                (2, app.t("room.babyCount2")),
                (3, app.t("room.babyCount3")),
            ];
            widgets::int_choice_field(app, ui, &at(&["pendingChildren"]), o, &choices);
        }
        for (key, label, look) in [
            ("childColor", "room.babySkin", "skin"),
            ("childHairColor", "room.babyHair", "hair"),
        ] {
            let Some(cur) = vault::int(pr, &[key]) else {
                continue;
            };
            let path = at(&[key]);
            let o = Opts::text(app.t(label));
            let presets: Vec<u32> = catalog()
                .looks
                .get("female")
                .map(|l| {
                    if look == "skin" {
                        l.skin_colors.clone()
                    } else {
                        l.hair_colors.clone()
                    }
                })
                .unwrap_or_default();
            widgets::row(app, ui, &path, &o, |app, ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let now = (cur as u32) & 0x00ff_ffff;
                    let own = now | 0xff00_0000;
                    for col in
                        std::iter::once(own).chain(presets.iter().copied().filter(|c| *c != own))
                    {
                        let rgb = [(col >> 16) as u8, (col >> 8) as u8, col as u8];
                        let resp = egui::widgets::color_picker::show_color(
                            ui,
                            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                            egui::vec2(18.0, 14.0),
                        )
                        .interact(Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if col & 0x00ff_ffff == now {
                            ui.painter().rect_stroke(
                                resp.rect.expand(1.5),
                                2.0,
                                egui::Stroke::new(1.5, c.accent),
                                egui::StrokeKind::Outside,
                            );
                        }
                        if resp.clicked() && col & 0x00ff_ffff != now {
                            app.act(Action::SetInt {
                                path: path.clone(),
                                value: i64::from(col | 0xff00_0000),
                            });
                        }
                    }
                });
            });
        }
    }
}

fn room_timers(app: &mut App, ui: &mut egui::Ui, r: &Value) {
    let Some(root) = app.open.as_ref().map(|o| o.doc.shared_root()) else {
        return;
    };
    let name = |id: i64| {
        vault::dweller_index(&root, id)
            .map(|j| vault::dweller_name(&vault::arr(&root, &vault::DWELLERS)[j]))
            .unwrap_or_else(|| format!("#{id}"))
    };
    let mut rows: Vec<(i64, String, &str)> = Vec::new();
    for s in r
        .get("slots")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let (Some(d), Some(t)) = (
            vault::int(s, &["dwellerID"]).filter(|d| *d >= 0),
            vault::int(s, &["taskID"]).filter(|t| *t >= 0),
        ) {
            rows.push((
                t,
                app.tf("room.trainingOf", &[("name", name(d))]),
                "room.trainingHelp",
            ));
        }
    }
    for pr in r
        .get("partners")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if vault::text(pr, &["s"]).as_deref() != Some("RaisingBaby") {
            continue;
        }
        if let (Some(f), Some(t)) = (
            vault::int(pr, &["f"]),
            vault::int(pr, &["t"]).filter(|t| *t >= 0),
        ) {
            rows.push((
                t,
                app.tf("room.birthOf", &[("name", name(f))]),
                "room.birthHelp",
            ));
        }
    }
    for ch in r
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let (Some(d), Some(t)) = (
            vault::int(ch, &["dwellerID"]),
            vault::int(ch, &["taskID"]).filter(|t| *t >= 0),
        ) {
            rows.push((
                t,
                app.tf("room.growUpOf", &[("name", name(d))]),
                "room.growUpHelp",
            ));
        }
    }
    if vault::text(r, &["type"]).as_deref() == Some("Radio") {
        if let Some(t) = vault::int(r, &["currentState", "taskId"]).filter(|t| *t >= 0) {
            rows.push((t, app.t("room.radioIn"), "room.radioInHelp"));
        }
    }
    let running: Vec<_> = rows
        .into_iter()
        .filter(|(t, _, _)| {
            vault::arr(&root, &["taskMgr", "tasks"])
                .iter()
                .any(|x| vault::int(x, &["id"]) == Some(*t))
        })
        .collect();
    let crafting = vault::text(r, &["CraftingItemId"]).filter(|s| !s.is_empty());
    if running.is_empty() && crafting.is_none() {
        return;
    }
    ui.add_space(6.0);
    widgets::hint_label(ui, &app.t("room.timers"), &app.t("room.timersHelp"));
    for (t, label, help) in running {
        let help = app.t(help);
        crate::sec_other::task_timer(app, ui, t, &label, &help);
    }
    let Some(item) = crafting else { return };
    let ru = app.lang == shelter_core::i18n::Lang::Ru;
    let c = style::colors(ui.ctx());
    let weapon = catalog().weapon(&item);
    let outfit = catalog().outfit(&item);
    let theme = catalog().themes.iter().find(|t| t.id == item);
    let (kind, name, craft) = if let Some(w) = weapon {
        (
            Some(ItemKind::Weapon),
            w.name.get(ru).to_string(),
            w.craft.as_ref(),
        )
    } else if let Some(o) = outfit {
        (
            Some(ItemKind::Outfit),
            o.name.get(ru).to_string(),
            o.craft.as_ref(),
        )
    } else if let Some(t) = theme {
        (None, t.name.get(ru).to_string(), t.craft.as_ref())
    } else {
        (None, item.clone(), None)
    };
    let required = craft.map_or(0.0, |cr| cr.time);
    let done = r
        .get("CompletedTime")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let ready = required > 0.0 && done >= required;
    let label = ui.label(app.tf("room.craftingOf", &[("name", name)]));
    match kind {
        Some(k) => {
            item_hover(app, label, k, &item);
        }
        None if theme.is_some() => {
            theme_hover(app, label, &item);
        }
        None => {}
    }
    if required > 0.0 {
        let part = (done / required).clamp(0.0, 1.0);
        ui.add(
            egui::ProgressBar::new(part as f32)
                .desired_width(ui.available_width().min(320.0))
                .text(app.tf(
                    "room.craftProgress",
                    &[("pct", format!("{:.1}", part * 100.0))],
                )),
        )
        .on_hover_text(app.t("room.craftProgressHelp"));
    }
    if ready {
        ui.label(
            RichText::new(format!("{}  {}", icons::CHECK, app.t("room.craftReady")))
                .color(c.success),
        );
        return;
    }
    if ui
        .button(format!("{}  {}", icons::CHECK, app.t("room.craftNow")))
        .on_hover_text(app.t("room.craftNowHelp"))
        .clicked()
    {
        if let Some(i) =
            vault::int(r, &["deserializeID"]).and_then(|id| vault::room_index(&root, id))
        {
            // The game clamps progress to the recipe time on load.
            let value = if required > 0.0 {
                required
            } else {
                9_999_999.0
            };
            app.act(Action::SetNumber {
                path: pi(&vault::ROOMS, i, &["CompletedTime"]),
                value,
            });
        }
    }
}

/// Opens the new room form with the same type, size and level, placed at the
/// nearest free spot that passes every check.
fn start_copy(app: &mut App, root: &Value, r: &Value) {
    let Some(me) = map::placed(r) else { return };
    let ((row, col), ok) = map::copy_spot(root, &me);
    if !ok {
        app.toast(app.t("map.copyNoSpot"), false);
    }
    app.map_add = Some(MapAdd {
        row,
        at: col,
        kind: me.kind,
        merge: me.merge,
        level: me.level,
        rock: false,
        copy: true,
    });
    app.room_sel = None;
    app.rock_sel = None;
}

/// Selected rocks on the map.
fn rock_panel(app: &mut App, ui: &mut egui::Ui, root: &Value, (row, col): (i64, i64)) {
    let list = vault::arr(root, &["vault", "rocks"]);
    let Some(index) = list
        .iter()
        .position(|r| vault::int(r, &["r"]) == Some(row) && vault::int(r, &["c"]) == Some(col))
    else {
        app.rock_sel = None;
        return;
    };
    let c = style::colors(ui.ctx());
    let mut remove = false;
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("map.rock")).font(style::bold(16.0)));
        ui.label(
            RichText::new(app.tf(
                "room.position",
                &[("row", row.to_string()), ("col", col.to_string())],
            ))
            .small()
            .color(c.muted),
        );
        ui.add(egui::Label::new(RichText::new(app.t("map.rockHelp")).color(c.muted)).wrap());
        ui.add_space(6.0);
        remove = ui
            .button(format!("{}  {}", icons::TRASH, app.t("map.removeRock")))
            .clicked();
    });
    if remove && app.act(Action::RemoveRock { index }) {
        app.rock_sel = None;
    }
}

fn rocks(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let list = vault::arr(root, &["vault", "rocks"]);
    let c = style::colors(ui.ctx());
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.rocks")).font(style::bold(16.0)));
        ui.label(app.tf("vault.rocksCount", &[("n", list.len().to_string())]));
        ui.add(
            egui::Label::new(
                RichText::new(app.t("vault.rocksHelp"))
                    .small()
                    .color(c.muted),
            )
            .wrap(),
        );
        if !list.is_empty()
            && ui
                .button(format!(
                    "{}  {}",
                    icons::TRASH,
                    app.t("vault.removeAllRocks")
                ))
                .clicked()
        {
            app.act(Action::RemoveAllRocks);
            app.rock_sel = None;
        }
    });
}

use shelter_core::pictures::{applied_theme, theme_picture, HOLIDAYS, THEME_ROOMS};

/// Room picture from the game, as wide as the space allows.
fn room_picture(app: &mut App, ui: &mut egui::Ui, kind: &str, id: &str, max_w: f32) -> bool {
    let tex = if crate::art::Store::known(kind, id) {
        app.art.icon(ui.ctx(), kind, id)
    } else {
        let alt = shelter_core::pictures::room_picture_fallback(id);
        alt.and_then(|alt| app.art.icon(ui.ctx(), kind, &alt))
    };
    let Some(t) = tex else {
        return false;
    };
    let size = t.size_vec2();
    let mut w = max_w.min(ui.available_width()).max(40.0);
    let mut h = size.y * w / size.x.max(1.0);
    // Single rooms are nearly square; keep them as tall as a full-width room picture.
    if h > 110.0 {
        w *= 110.0 / h;
        h = 110.0;
    }
    ui.add(egui::Image::new(&t).fit_to_exact_size(egui::vec2(w, h)));
    true
}

fn room_picture_id(root: &Value, r: &Value) -> (String, String) {
    shelter_core::pictures::room_picture(root, r)
}

/// Picture and pick list of the theme for one room type.
fn theme_picker(app: &mut App, ui: &mut egui::Ui, root: &Value, room: &str) {
    let c = style::colors(ui.ctx());
    let list: Vec<_> = catalog().themes.iter().filter(|t| t.room == room).collect();
    let cur = applied_theme(root, room);
    let cur_id = cur
        .as_ref()
        .and_then(|v| list.iter().find(|t| &t.theme == v))
        .map(|t| t.id.clone());
    let selected = match (&cur, &cur_id) {
        (None, _) => app.t("theme.standard"),
        (Some(_), Some(id)) => theme_name(app, id),
        (Some(v), None) => HOLIDAYS
            .iter()
            .find(|(k, _)| k == v)
            .map_or_else(|| v.clone(), |(k, _)| holiday_label(app, k)),
    };
    let mut pick: Option<Option<String>> = None;
    let width = ui.available_width().min(320.0);
    egui::ComboBox::from_id_salt(("theme_room", room))
        .selected_text(selected)
        .width(width)
        .height(420.0)
        .show_ui(ui, |ui| {
            let r = ui.selectable_label(cur.is_none(), app.t("theme.standard"));
            let r = standard_hover(app, r, room);
            if r.clicked() {
                pick = Some(None);
            }
            for t in &list {
                let crafted = shelter_core::actions::theme_state(root, &t.id).1;
                let mut label = theme_name(app, &t.id);
                if !crafted {
                    label = format!("{label}  ·  {}", app.t("theme.willCraft"));
                }
                let r = ui.selectable_label(cur_id.as_deref() == Some(t.id.as_str()), label);
                if theme_hover(app, r, &t.id).clicked() {
                    pick = Some(Some(t.id.clone()));
                }
            }
        })
        .response
        .on_hover_text(app.t("theme.pickHelp"));
    if let Some(choice) = pick {
        if choice != cur_id || (choice.is_none() && cur.is_some()) {
            app.act(Action::SetRoomTheme {
                room: room.to_string(),
                theme: choice,
            });
        }
    }
    if cur.is_some() && cur_id.is_none() {
        ui.add(
            egui::Label::new(
                RichText::new(format!("{}  {}", icons::INFO, app.t("theme.holidaySet")))
                    .small()
                    .color(c.warning),
            )
            .wrap(),
        );
    }
}

/// Card for the standard look of a room type.
fn standard_hover(app: &mut App, resp: egui::Response, room: &str) -> egui::Response {
    resp.on_hover_ui(|ui| {
        ui.set_max_width(400.0);
        ui.label(RichText::new(app.t("theme.standard")).font(style::bold(15.0)));
        let (kind, id) = theme_picture(room, None);
        if !room_picture(app, ui, &kind, &id, 384.0) {
            let c = style::colors(ui.ctx());
            ui.label(
                RichText::new(app.t("theme.noPicture"))
                    .small()
                    .color(c.muted),
            );
        }
    })
}

fn muted_note(ui: &mut egui::Ui, text: &str) {
    let c = style::colors(ui.ctx());
    ui.add(egui::Label::new(RichText::new(text).color(c.muted)).wrap());
}

fn theme_name(app: &App, id: &str) -> String {
    catalog().theme(id).map_or_else(
        || id.to_string(),
        |t| {
            t.name
                .get(app.lang == shelter_core::i18n::Lang::Ru)
                .to_string()
        },
    )
}

/// Whether today is inside a [month, day] window that may wrap over the new year.
fn in_season(start: [u32; 2], end: [u32; 2]) -> bool {
    let now = jiff::Zoned::now();
    let today = (now.month() as u32, now.day() as u32);
    let s = (start[0], start[1]);
    let e = (end[0], end[1]);
    if s <= e {
        s <= today && today <= e
    } else {
        today >= s || today <= e
    }
}

fn holiday_label(app: &App, key: &str) -> String {
    app.t(&format!("theme.holiday.{key}"))
}

fn themes(app: &mut App, ui: &mut egui::Ui) {
    let c = style::colors(ui.ctx());
    let Some(root) = app.open.as_ref().map(|o| o.doc.shared_root()) else {
        return;
    };
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("theme.title")).font(style::bold(16.0)));
        muted_note(ui, &app.t("theme.help"));
        if root.at(&p(&["specialTheme", "themeByRoomType"])).is_none() {
            muted_note(ui, &app.t("theme.noData"));
            return;
        }
        for room in THEME_ROOMS {
            let list: Vec<_> = catalog().themes.iter().filter(|t| t.room == room).collect();
            if list.is_empty() {
                continue;
            }
            ui.add_space(10.0);
            let title = if room == "FakeWasteland" {
                app.t("theme.exterior")
            } else {
                kind_name(app, room, 1)
            };
            ui.label(RichText::new(title).font(style::bold(14.0)));
            theme_picker(app, ui, &root, room);
            let cur = applied_theme(&root, room);
            let (kind, pic) = theme_picture(room, cur.as_deref());
            if !room_picture(app, ui, &kind, &pic, 420.0)
                && app.art.status() == crate::art::Status::Ready
                && !crate::art::Store::known(&kind, &pic)
            {
                muted_note(ui, &app.t("theme.noPicture"));
            }
            egui::CollapsingHeader::new(app.t("theme.states"))
                .id_salt(("theme_states", room))
                .show(ui, |ui| {
                    egui::Grid::new(("theme_grid", room))
                        .num_columns(3)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            for t in &list {
                                let (parts, crafted) =
                                    shelter_core::actions::theme_state(&root, &t.id);
                                let r = ui.label(theme_name(app, &t.id));
                                theme_hover(app, r, &t.id);
                                ui.horizontal(|ui| {
                                    let key = format!("theme_parts:{}", t.id);
                                    if let Some(v) = widgets::pick_number(
                                        app,
                                        ui,
                                        &key,
                                        parts as f64,
                                        (0.0, 9.0),
                                        None,
                                        true,
                                    ) {
                                        let v = v as i64;
                                        if v != parts {
                                            app.act(Action::SetThemeState {
                                                id: t.id.clone(),
                                                parts: v,
                                                crafted: crafted && v >= 9,
                                            });
                                        }
                                    }
                                    widgets::unit_label(ui, &app.t("theme.partsOf"));
                                });
                                {
                                    let mut c_now = crafted;
                                    if ui
                                        .checkbox(&mut c_now, app.t("theme.crafted"))
                                        .on_hover_text(app.t("theme.craftedHelp"))
                                        .changed()
                                    {
                                        app.act(Action::SetThemeState {
                                            id: t.id.clone(),
                                            parts: if c_now { 9 } else { parts },
                                            crafted: c_now,
                                        });
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
        }
    });
    ui.add_space(10.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("theme.holidays")).font(style::bold(16.0)));
        muted_note(ui, &app.t("theme.holidaysHelp"));
        for (key, id) in HOLIDAYS {
            let Some(ev) = catalog().event_themes.iter().find(|e| e.id == id) else {
                continue;
            };
            let dates = app.tf(
                "theme.dates",
                &[
                    ("from", format!("{:02}.{:02}", ev.start[1], ev.start[0])),
                    ("to", format!("{:02}.{:02}", ev.end[1], ev.end[0])),
                ],
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(holiday_label(app, key)).strong());
                ui.label(RichText::new(dates).color(c.muted));
                if in_season(ev.start, ev.end) {
                    ui.label(RichText::new(app.t("theme.now")).color(c.success));
                }
            });
        }
    });
    if app.expert() {
        ui.add_space(10.0);
        panel(ui, |ui| {
            ui.label(RichText::new(app.t("theme.expert")).font(style::bold(16.0)));
            let o = Opts::at(app, &p(&["vault", "VaultTheme"]))
                .help(app.t("theme.vaultThemeHelp"))
                .range(0.0, 3.0);
            widgets::num_field(app, ui, &p(&["vault", "VaultTheme"]), o);
            ui.add_space(6.0);
            ui.label(RichText::new(app.t("vault.roomThemes")).font(style::bold(14.0)));
            widgets::object_form(app, ui, &p(&["specialTheme", "themeByRoomType"]), 0);
            ui.add_space(6.0);
            ui.label(RichText::new(app.t("vault.eventThemes")).font(style::bold(14.0)));
            widgets::object_form(app, ui, &p(&["specialTheme", "eventsThemes"]), 0);
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum RecipeFilter {
    #[default]
    All,
    Weapons,
    Outfits,
    Themes,
}

/// Blueprint book, crafting recipes and journal marks.
fn book(app: &mut App, ui: &mut egui::Ui) {
    let c = style::colors(ui.ctx());
    let Some(root) = app.open.as_ref().map(|o| o.doc.shared_root()) else {
        return;
    };
    let expert = app.expert();
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.unlocks")).font(style::bold(16.0)));
        muted_note(ui, &app.t("vault.unlocksHelp"));
        if root.get("unlockableMgr").is_none() {
            muted_note(ui, &app.t("theme.noData"));
            return;
        }
        let built: Vec<String> = map::rooms(&root).into_iter().map(|r| r.kind).collect();
        let mut rows: Vec<(i64, &shelter_core::catalog::Room)> = catalog()
            .rooms
            .iter()
            .filter_map(|r| {
                let id = r.unlock.as_ref()?;
                let u = catalog().unlockables.iter().find(|u| &u.id == id)?;
                (expert || !u.season).then_some((u.goal, r))
            })
            .collect();
        rows.sort_by_key(|(goal, r)| (*goal, r.kind.clone()));
        let mut change = None;
        for (goal, r) in rows {
            let unlocked = shelter_core::actions::room_lock(&root, &r.kind).is_none();
            let is_built = built.contains(&r.kind);
            ui.horizontal_wrapped(|ui| {
                let mut v = unlocked;
                let can = !unlocked || !is_built || expert;
                let resp =
                    ui.add_enabled(can, egui::Checkbox::new(&mut v, kind_name(app, &r.kind, 1)));
                let resp = if can {
                    resp
                } else {
                    resp.on_disabled_hover_text(app.t("vault.lockBuilt"))
                };
                if resp.changed() {
                    change = Some((r.kind.clone(), v));
                }
                ui.label(
                    RichText::new(app.tf("vault.unlockAt", &[("n", goal.to_string())]))
                        .small()
                        .color(c.muted),
                );
                if is_built && !unlocked {
                    ui.label(
                        RichText::new(format!("{}  {}", icons::ALERT, app.t("vault.builtLocked")))
                            .small()
                            .color(c.danger),
                    );
                }
            });
        }
        if let Some((room, unlocked)) = change {
            app.act(Action::SetRoomUnlocked { room, unlocked });
        }
        ui.add_space(4.0);
        if ui
            .button(format!("{}  {}", icons::LOCK, app.t("vault.unlockRooms")))
            .on_hover_text(app.t("vault.unlockRoomsHelp"))
            .clicked()
        {
            app.act(Action::UnlockAllRooms);
        }
    });
    ui.add_space(10.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.recipesTitle")).font(style::bold(16.0)));
        muted_note(ui, &app.t("vault.recipesHelp"));
        let known: std::collections::HashSet<String> = vault::arr(&root, &["survivalW", "recipes"])
            .iter()
            .filter_map(Value::as_text)
            .collect();
        let ru = app.lang == shelter_core::i18n::Lang::Ru;
        let mut all: Vec<(RecipeFilter, String, String, String)> = Vec::new();
        for w in catalog().weapons.iter().filter(|w| w.recipe) {
            all.push((
                RecipeFilter::Weapons,
                w.id.clone(),
                w.name.get(ru).to_string(),
                w.rarity.clone(),
            ));
        }
        for o in catalog().outfits.iter().filter(|o| o.recipe) {
            all.push((
                RecipeFilter::Outfits,
                o.id.clone(),
                o.name.get(ru).to_string(),
                o.rarity.clone(),
            ));
        }
        for t in catalog().themes.iter().filter(|t| t.recipe) {
            all.push((
                RecipeFilter::Themes,
                t.id.clone(),
                t.name.get(ru).to_string(),
                String::new(),
            ));
        }
        let total = all.len();
        let have = all.iter().filter(|r| known.contains(&r.1)).count();
        ui.label(app.tf(
            "vault.recipesCount",
            &[("n", have.to_string()), ("total", total.to_string())],
        ));
        let fid = egui::Id::unique("recipe_filter");
        let qid = egui::Id::unique("recipe_query");
        let mut filter: RecipeFilter = ui.data(|d| d.get_temp(fid)).unwrap_or_default();
        let mut query: String = ui.data(|d| d.get_temp(qid)).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            for (f, key) in [
                (RecipeFilter::All, "vault.recipesAll"),
                (RecipeFilter::Weapons, "vault.recipesWeapons"),
                (RecipeFilter::Outfits, "vault.recipesOutfits"),
                (RecipeFilter::Themes, "vault.recipesThemes"),
            ] {
                if ui.selectable_label(filter == f, app.t(key)).clicked() {
                    filter = f;
                }
            }
            ui.add(
                egui::TextEdit::singleline(&mut query)
                    .hint_text(app.t("c.search"))
                    .desired_width(180.0),
            );
        });
        ui.data_mut(|d| {
            d.insert_temp(fid, filter);
            d.insert_temp(qid, query.clone());
        });
        let q = query.to_lowercase();
        let mut list: Vec<&(RecipeFilter, String, String, String)> = all
            .iter()
            .filter(|r| filter == RecipeFilter::All || r.0 == filter)
            .filter(|r| {
                q.is_empty() || r.2.to_lowercase().contains(&q) || r.1.to_lowercase().contains(&q)
            })
            .collect();
        list.sort_by_key(|a| a.2.to_lowercase());
        let mut change = None;
        egui::ScrollArea::vertical()
            .id_salt("recipe_list")
            .min_scrolled_height(300.0)
            .max_height(420.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if list.is_empty() {
                    muted_note(ui, &app.t("c.nothingFound"));
                }
                for (kind, id, name, rarity) in list {
                    ui.horizontal_wrapped(|ui| {
                        let mut v = known.contains(id);
                        let r = ui.checkbox(&mut v, name);
                        let r = match kind {
                            RecipeFilter::Weapons => item_hover(app, r, ItemKind::Weapon, id),
                            RecipeFilter::Outfits => item_hover(app, r, ItemKind::Outfit, id),
                            _ => theme_hover(app, r, id),
                        };
                        if r.changed() {
                            change = Some((id.clone(), v));
                        }
                        if !rarity.is_empty() {
                            ui.label(
                                RichText::new(rarity_label(app, rarity))
                                    .small()
                                    .color(c.muted),
                            );
                        }
                    });
                }
            });
        if let Some((id, known)) = change {
            app.act(Action::SetRecipe { id, known });
        }
        ui.add_space(4.0);
        if ui
            .button(format!(
                "{}  {}",
                icons::SCROLL,
                app.t("vault.unlockRecipes")
            ))
            .on_hover_text(app.t("vault.unlockRecipesHelp"))
            .clicked()
        {
            app.act(Action::UnlockAllRecipes);
        }
    });
    ui.add_space(10.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("vault.journal")).font(style::bold(16.0)));
        muted_note(ui, &app.t("vault.journalHelp"));
        let fresh: usize = [
            "weapons",
            "outfits",
            "dwellers",
            "pets",
            "breeds",
            "decorations",
            "junk",
        ]
        .iter()
        .map(|k| {
            vault::arr(&root, &["survivalW", k])
                .iter()
                .filter(|v| v.as_text().is_some_and(|s| s.starts_with('N')))
                .count()
        })
        .sum::<usize>()
            + vault::arr(&root, &["survivalW", "collectedThemes", "themeList"])
                .iter()
                .filter(|t| vault::flag(t, &["extraData", "IsNew"]) == Some(true))
                .count()
            + root
                .at(&p(&["specialTheme", "eventsThemes"]))
                .and_then(Value::as_object)
                .map_or(0, |m| {
                    m.0.iter()
                        .filter(|(_, t)| vault::flag(t, &["extraData", "IsNew"]) == Some(true))
                        .count()
                });
        ui.label(app.tf("vault.newMarks", &[("n", fresh.to_string())]));
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    fresh > 0,
                    egui::Button::new(format!("{}  {}", icons::CHECK, app.t("vault.markRead"))),
                )
                .on_hover_text(app.t("vault.markReadHelp"))
                .clicked()
            {
                app.act(Action::MarkCollectionRead);
            }
            if ui
                .button(format!(
                    "{}  {}",
                    icons::BADGE,
                    app.t("vault.completeCollection")
                ))
                .on_hover_text(app.t("vault.completeCollectionHelp"))
                .clicked()
            {
                app.act(Action::CompleteCollection);
            }
        });
    });
}
