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

use eframe::egui::{self, Color32, RichText, Sense};
use shelter_core::actions::{Action, Slot};
use shelter_core::catalog::{catalog, ItemKind};
use shelter_core::i18n::Lang;
use shelter_core::json::{Seg, Value};
use shelter_core::rules::RARITIES;
use shelter_core::vault::{self, join, pi, SPECIAL_LETTERS};

use crate::app::{panel, section_title, App, Modal};
use crate::icons;
use crate::style;
use crate::widgets::{self, Opts};

struct Row {
    id: i64,
    name: String,
    level: i64,
    happiness: f64,
    health: f64,
    special: i64,
    rarity: String,
    dead: bool,
    gender: i64,
}

fn rows(root: &Value) -> Vec<Row> {
    vault::arr(root, &vault::DWELLERS)
        .iter()
        .filter_map(|d| {
            Some(Row {
                id: vault::int(d, &["serializeId"])?,
                name: vault::dweller_name(d),
                level: vault::int(d, &["experience", "currentLevel"]).unwrap_or(1),
                happiness: d
                    .at(&vault::p(&["happiness", "happinessValue"]))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                health: d
                    .at(&vault::p(&["health", "healthValue"]))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                special: (1..=7).filter_map(|s| vault::special(d, s)).sum(),
                rarity: vault::text(d, &["rarity"]).unwrap_or_default(),
                dead: vault::is_dead(d),
                gender: vault::int(d, &["gender"]).unwrap_or(0),
            })
        })
        .collect()
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let root = open.doc.shared_root();
    let all = rows(&root);
    section_title(ui, icons::USERS, &app.t("nav.dwellers"), "");
    let c = style::colors(ui.ctx());
    egui::Panel::left("dw_list")
        .resizable(true)
        .default_size(340.0)
        .min_size(260.0)
        .show(ui, |ui| {
            list(app, ui, &all, &c);
        });
    let mut area = egui::ScrollArea::vertical()
        .id_salt("dweller_main")
        .auto_shrink(false);
    if app.dw.shown != app.dw.focus {
        if app.settings.dweller_scroll == crate::settings::DwellerScroll::Top {
            area = area.vertical_scroll_offset(0.0);
        }
        app.dw.shown = app.dw.focus;
    }
    area.show(ui, |ui| {
        if app.dw.selected.len() >= 2 {
            mass_bar(app, ui, &root);
            ui.add_space(8.0);
        } else {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(8, 2))
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(app.t("dw.massHint")).small().color(c.muted),
                        )
                        .wrap(),
                    );
                });
            ui.add_space(4.0);
        }
        match app
            .dw
            .focus
            .filter(|id| vault::dweller_index(&root, *id).is_some())
        {
            Some(id) => card(app, ui, id),
            None => {
                ui.label(RichText::new(app.t("dw.none")).color(c.muted));
            }
        }
        ui.add_space(10.0);
        pets(app, ui, &root);
        ui.add_space(10.0);
        waiting(app, ui, &root);
    });
}

fn list(app: &mut App, ui: &mut egui::Ui, all: &[Row], c: &style::Colors) {
    ui.horizontal(|ui| {
        ui.label(icons::SEARCH);
        let hint = app.t("c.search");
        ui.add(
            egui::TextEdit::singleline(&mut app.dw.search)
                .hint_text(hint)
                .desired_width(f32::INFINITY),
        );
    });
    ui.horizontal_wrapped(|ui| {
        let filters = [
            "dw.filter.all",
            "dw.filter.alive",
            "dw.filter.dead",
            "dw.filter.male",
            "dw.filter.female",
        ];
        egui::ComboBox::from_id_salt("dw_filter")
            .selected_text(app.t(filters[app.dw.filter]))
            .show_ui(ui, |ui| {
                for (i, f) in filters.iter().enumerate() {
                    if ui.selectable_label(app.dw.filter == i, app.t(f)).clicked() {
                        app.dw.filter = i;
                    }
                }
            });
        let sorts = [
            "dw.sort.name",
            "dw.sort.level",
            "dw.sort.happiness",
            "dw.sort.health",
            "dw.sort.special",
            "dw.sort.rarity",
            "dw.sort.id",
        ];
        egui::ComboBox::from_id_salt("dw_sort")
            .selected_text(format!("{} {}", icons::SORT, app.t(sorts[app.dw.sort])))
            .show_ui(ui, |ui| {
                for (i, s) in sorts.iter().enumerate() {
                    if ui.selectable_label(app.dw.sort == i, app.t(s)).clicked() {
                        app.dw.sort = i;
                    }
                }
            });
        if ui
            .small_button(if app.dw.desc { "↓" } else { "↑" })
            .clicked()
        {
            app.dw.desc = !app.dw.desc;
        }
    });
    let q = app.dw.search.trim().to_lowercase();
    let mut shown: Vec<&Row> = all
        .iter()
        .filter(|r| match app.dw.filter {
            1 => !r.dead,
            2 => r.dead,
            3 => r.gender == 2,
            4 => r.gender == 1,
            _ => true,
        })
        .filter(|r| q.is_empty() || r.name.to_lowercase().contains(&q) || r.id.to_string() == q)
        .collect();
    let rank = |r: &str| RARITIES.iter().position(|x| *x == r).unwrap_or(9);
    shown.sort_by(|a, b| {
        let o = match app.dw.sort {
            1 => a.level.cmp(&b.level),
            2 => a.happiness.total_cmp(&b.happiness),
            3 => a.health.total_cmp(&b.health),
            4 => a.special.cmp(&b.special),
            5 => rank(&a.rarity).cmp(&rank(&b.rarity)),
            6 => a.id.cmp(&b.id),
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        };
        if app.dw.desc {
            o.reverse()
        } else {
            o
        }
    });
    // Everything here wraps: a row wider than the panel would be cut off by the
    // panel and leave an empty strip over the list.
    ui.horizontal_wrapped(|ui| {
        if ui.small_button(app.t("c.selectAll")).clicked() {
            app.dw.selected = shown.iter().map(|r| r.id).collect();
        }
        if ui.small_button(app.t("c.selectNone")).clicked() {
            app.dw.selected.clear();
        }
        ui.label(
            RichText::new(app.tf("c.selected", &[("n", app.dw.selected.len().to_string())]))
                .small()
                .color(c.muted),
        );
    });
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(format!("{}  {}", icons::USER_PLUS, app.t("dw.add")))
            .clicked()
        {
            app.modals.push(Modal::NewDweller {
                first: String::new(),
                last: String::new(),
                gender: 1,
            });
        }
        if ui
            .button(format!("{}  {}", icons::DOWNLOAD, app.t("dw.import")))
            .on_hover_text(app.t("dw.importHelp"))
            .clicked()
        {
            import_from_file(app);
        }
        if ui
            .button(format!("{}  {}", icons::STAR, app.t("dw.fromJournal")))
            .on_hover_text(app.t("dw.fromJournalHelp"))
            .clicked()
        {
            app.modals.push(Modal::FromJournal {
                pets: false,
                query: String::new(),
                pick: None,
                bonus: 0.0,
            });
        }
    });
    ui.separator();
    if shown.is_empty() {
        let text = if all.is_empty() {
            app.t("dw.emptyVault")
        } else {
            app.t("c.noResults")
        };
        ui.label(RichText::new(text).color(c.muted));
        return;
    }
    let row_h = 44.0;
    egui::ScrollArea::vertical()
        .id_salt("dweller_rows")
        .auto_shrink(false)
        .show_rows(ui, row_h, shown.len(), |ui, range| {
            for r in &shown[range] {
                let focused = app.dw.focus == Some(r.id);
                let mut box_rect = egui::Rect::NOTHING;
                let fill = if focused {
                    c.accent_soft
                } else {
                    Color32::TRANSPARENT
                };
                let resp = egui::Frame::new()
                    .fill(fill)
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(row_h - 10.0);
                        ui.horizontal(|ui| {
                            let mut on = app.dw.selected.contains(&r.id);
                            box_rect = ui.checkbox(&mut on, "").rect.expand(4.0);
                            ui.vertical(|ui| {
                                let mut title = RichText::new(if r.name.is_empty() {
                                    format!("#{}", r.id)
                                } else {
                                    r.name.clone()
                                })
                                .font(style::bold(14.0));
                                if r.dead {
                                    title = title.color(c.muted).strikethrough();
                                }
                                ui.horizontal(|ui| {
                                    if r.dead {
                                        ui.label(RichText::new(icons::SKULL).color(c.muted));
                                    }
                                    ui.add(egui::Label::new(title).truncate());
                                });
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{} {} · {} {:.0} · {} {:.0}",
                                            app.t("dw.level"),
                                            r.level,
                                            icons::SMILE,
                                            r.happiness,
                                            icons::HEART_PULSE,
                                            r.health
                                        ))
                                        .small()
                                        .color(c.muted),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                    });
                let resp = resp.response.interact(Sense::click());
                let on_box = resp
                    .interact_pointer_pos()
                    .is_some_and(|p| box_rect.contains(p));
                if resp.clicked() && on_box {
                    if app.dw.selected.remove(&r.id) {
                        if app.dw.focus == Some(r.id) {
                            app.dw.focus = app.dw.selected.first().copied();
                        }
                    } else {
                        app.dw.selected.insert(r.id);
                        if app.dw.focus.is_none() {
                            app.dw.focus = Some(r.id);
                        }
                    }
                } else if resp.clicked() {
                    app.dw.focus = Some(r.id);
                    if ui.input(|i| i.modifiers.command) {
                        if !app.dw.selected.remove(&r.id) {
                            app.dw.selected.insert(r.id);
                        }
                    } else if !app.dw.selected.contains(&r.id) {
                        app.dw.selected = [r.id].into();
                    }
                }
            }
        });
}

/// Reads another save (it is only read) and offers its dwellers.
fn import_from_file(app: &mut App) {
    let mut d = rfd::FileDialog::new().add_filter("Fallout Shelter", &["sav", "json"]);
    if let Some(dir) = crate::files::standard_save_dirs()
        .into_iter()
        .find(|d| d.is_dir())
    {
        d = d.set_directory(dir);
    }
    let Some(path) = d.pick_file() else { return };
    let doc = std::fs::read(&path)
        .ok()
        .and_then(|b| shelter_core::document::Document::open(&b, "").ok());
    let Some(doc) = doc else {
        app.toast_error(app.t("dw.importBad"));
        return;
    };
    let dwellers: Vec<(String, String)> = vault::arr(doc.root(), &vault::DWELLERS)
        .iter()
        .map(|d| {
            let level = vault::int(d, &["experience", "currentLevel"]).unwrap_or(1);
            let rarity = vault::text(d, &["rarity"]).unwrap_or_default();
            let label = format!(
                "{}  ·  {} {level}  ·  {}",
                vault::dweller_name(d),
                app.t("dw.level"),
                catalog().rarity_name(&rarity, app.lang == Lang::Ru)
            );
            (label, d.to_compact())
        })
        .collect();
    if dwellers.is_empty() {
        app.toast_error(app.t("dw.importEmpty"));
        return;
    }
    app.modals.push(Modal::Import {
        file: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        dwellers,
        picked: std::collections::BTreeSet::new(),
    });
}

fn mass_bar(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let ids: Vec<i64> = app.dw.selected.iter().copied().collect();
    let n = ids.len();
    let c = style::colors(ui.ctx());
    panel(ui, |ui| {
        ui.label(
            RichText::new(format!(
                "{} – {}",
                app.t("dw.mass"),
                app.tf("c.selected", &[("n", n.to_string())])
            ))
            .font(style::bold(14.0)),
        );
        ui.add_enabled_ui(n > 0, |ui| {
            ui.horizontal_wrapped(|ui| {
                let items: [(&str, &str, Action); 6] = [
                    (
                        icons::DNA,
                        "dw.mass.special",
                        Action::MaxSpecial { ids: ids.clone() },
                    ),
                    (
                        icons::STAR,
                        "dw.mass.level",
                        Action::SetLevel {
                            ids: ids.clone(),
                            level: 50,
                        },
                    ),
                    (
                        icons::HEART_PULSE,
                        "dw.mass.health",
                        Action::FullHealth { ids: ids.clone() },
                    ),
                    (
                        icons::RADIATION,
                        "dw.mass.radiation",
                        Action::ClearRadiation { ids: ids.clone() },
                    ),
                    (
                        icons::SMILE,
                        "dw.mass.happiness",
                        Action::MaxHappiness { ids: ids.clone() },
                    ),
                    (
                        icons::SPARKLES,
                        "dw.mass.revive",
                        Action::Revive {
                            ids: ids.clone(),
                            home: false,
                        },
                    ),
                ];
                let any_dead = ids.iter().any(|id| {
                    vault::dweller_index(root, *id)
                        .is_some_and(|j| vault::is_dead(&vault::arr(root, &vault::DWELLERS)[j]))
                });
                for (icon, key, action) in items {
                    if key == "dw.mass.revive" && !any_dead {
                        continue;
                    }
                    if ui.button(format!("{icon}  {}", app.t(key))).clicked() {
                        app.act_mass(action, n);
                    }
                }
                if ui
                    .button(format!("{}  {}", icons::TRASH, app.t("dw.delete")))
                    .clicked()
                {
                    app.modals.push(Modal::ConfirmDelete(ids.clone()));
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(app.t("dw.mass.equip"));
                let kinds = [
                    (Slot::Weapon, ItemKind::Weapon),
                    (Slot::Outfit, ItemKind::Outfit),
                    (Slot::Pet, ItemKind::Pet),
                ];
                egui::ComboBox::from_id_salt("eq_kind")
                    .selected_text(
                        app.t(&format!("it.kind.{}", kinds[app.dw.equip_kind].1.as_str())),
                    )
                    .show_ui(ui, |ui| {
                        for (i, (_, k)) in kinds.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    app.dw.equip_kind == i,
                                    app.t(&format!("it.kind.{}", k.as_str())),
                                )
                                .clicked()
                            {
                                app.dw.equip_kind = i;
                            }
                        }
                    });
                let (slot, kind) = kinds[app.dw.equip_kind];
                let mut q = std::mem::take(&mut app.dw.equip_query);
                let picked = crate::sec_vault::item_picker(app, ui, "mass_equip", &[kind], &mut q);
                app.dw.equip_query = q;
                if let Some((_, id)) = picked {
                    app.act_mass(
                        Action::Equip {
                            ids: ids.clone(),
                            slot,
                            item: Some(id),
                        },
                        n,
                    );
                    app.dw.equip_query.clear();
                }
            });
            if app.expert() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(app.t("dw.mass.field"))
                        .on_hover_text(app.t("dw.mass.fieldHelp"));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.dw.mass_field)
                            .hint_text("health.radiationValue")
                            .desired_width(200.0)
                            .font(style::mono(13.0)),
                    );
                    ui.label("=");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.dw.mass_value)
                            .hint_text("0")
                            .desired_width(120.0)
                            .font(style::mono(13.0)),
                    );
                    if ui.button(app.t("c.apply")).clicked() {
                        let rel: Vec<Seg> = app
                            .dw
                            .mass_field
                            .split('.')
                            .filter(|s| !s.is_empty())
                            .map(|s| {
                                s.parse::<usize>()
                                    .map_or_else(|_| Seg::Key(s.to_string()), Seg::Index)
                            })
                            .collect();
                        if !rel.is_empty() {
                            let json = app.dw.mass_value.trim().to_string();
                            app.act_mass(
                                Action::SetDwellerField {
                                    ids: ids.clone(),
                                    rel,
                                    json,
                                },
                                n,
                            );
                        }
                    }
                });
            }
        });
        if n == 0 {
            ui.label(RichText::new(app.t("dw.none")).small().color(c.muted));
        }
    });
}

fn rgb_of(v: i64) -> [u8; 3] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

fn card(app: &mut App, ui: &mut egui::Ui, id: i64) {
    let Some(open) = &app.open else { return };
    let root = open.doc.shared_root();
    let Some(i) = vault::dweller_index(&root, id) else {
        return;
    };
    let d = vault::arr(&root, &vault::DWELLERS)[i].clone();
    let base = pi(&vault::DWELLERS, i, &[]);
    let at = |rest: &[&str]| join(&base, rest);
    let c = style::colors(ui.ctx());
    let lim = &catalog().limits;
    let expert = app.expert();
    panel(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(vault::dweller_name(&d)).font(style::bold(20.0)));
            if expert {
                ui.label(
                    RichText::new(format!("#{id}"))
                        .font(style::mono(13.0))
                        .color(c.muted),
                );
            }
            if vault::is_dead(&d) {
                crate::app::badge(ui, &app.t("dw.dead"), c.danger_soft, c.danger);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if vault::is_dead(&d) {
                let in_team = vault::team_of(&root, id).is_some();
                let mut b = ui.button(format!("{}  {}", icons::SPARKLES, app.t("dw.revive")));
                if in_team {
                    b = b.on_hover_text(app.t("dw.reviveTeamHelp"));
                }
                if b.clicked() {
                    app.act(Action::Revive {
                        ids: vec![id],
                        home: false,
                    });
                }
                if in_team
                    && ui
                        .button(format!("{}  {}", icons::SPARKLES, app.t("dw.reviveHome")))
                        .on_hover_text(app.t("dw.reviveHomeHelp"))
                        .clicked()
                {
                    app.act(Action::Revive {
                        ids: vec![id],
                        home: true,
                    });
                }
            }
            if ui
                .button(format!("{}  {}", icons::COPY, app.t("dw.clone")))
                .on_hover_text(app.t("dw.cloneHelp"))
                .clicked()
            {
                app.act(Action::CloneDweller { id });
            }
            if ui
                .button(format!("{}  {}", icons::TRASH, app.t("dw.delete")))
                .clicked()
            {
                app.modals.push(Modal::ConfirmDelete(vec![id]));
            }
        });
    });
    ui.add_space(8.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.identity")).font(style::bold(16.0)));
        let o = Opts::text(app.t("dw.firstName"));
        widgets::text_field(app, ui, &at(&["name"]), o);
        let o = Opts::text(app.t("dw.lastName"));
        widgets::text_field(app, ui, &at(&["lastName"]), o);
        let o = Opts::at(app, &at(&["gender"])).with_label(app.t("dw.gender"));
        let g = vec![(1, app.t("dw.gender.1")), (2, app.t("dw.gender.2"))];
        widgets::int_choice_field(app, ui, &at(&["gender"]), o, &g);
        let o = Opts::at(app, &at(&["rarity"])).with_label(app.t("dw.rarity"));
        let cur_rarity = vault::text(&d, &["rarity"]).unwrap_or_default();
        let r: Vec<(String, String)> = RARITIES
            .iter()
            .filter(|r| **r != "Common" || cur_rarity == "Common")
            .map(|r| {
                (
                    r.to_string(),
                    catalog().rarity_name(r, app.lang == Lang::Ru),
                )
            })
            .collect();
        widgets::choice_field(app, ui, &at(&["rarity"]), o, &r);
        if expert && d.get("uniqueData").is_some() {
            let o = Opts::text(app.t("dw.unique"));
            widgets::text_field(app, ui, &at(&["uniqueData"]), o);
        }
    });
    ui.add_space(8.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.progress")).font(style::bold(16.0)));
        let level = vault::int(&d, &["experience", "currentLevel"]).unwrap_or(1);
        let o = Opts::text(app.t("dw.level"))
            .help(app.t("dw.levelHelp"))
            .range(1.0, lim.dweller_level_max as f64);
        widgets::row(
            app,
            ui,
            &at(&["experience", "currentLevel"]),
            &o,
            |app, ui| {
                let max = if app.expert() {
                    999
                } else {
                    lim.dweller_level_max
                };
                let range = (1.0_f64.min(level as f64), (max as f64).max(level as f64));
                let slider = (1.0, (lim.dweller_level_max as f64).max(level as f64));
                let key = format!("level:{id}");
                if let Some(l) =
                    widgets::pick_number(app, ui, &key, level as f64, range, Some(slider), true)
                {
                    app.act(Action::SetLevel {
                        ids: vec![id],
                        level: l as i64,
                    });
                }
            },
        );
        for (key, label) in [("experienceValue", "dw.xp"), ("wastelandExperience", "")] {
            let path = at(&["experience", key]);
            if root.at(&path).is_some() {
                let o = if label.is_empty() {
                    Opts::at(app, &path)
                } else {
                    Opts::text(app.t(label))
                }
                .unit(app.t("c.units.xp"));
                widgets::num_field(app, ui, &path, o);
            }
        }
        for (key, label) in [
            ("hoursOnWasteland", "dw.wlHours"),
            ("daysOnWasteland", "dw.wlDays"),
        ] {
            if d.get(key).is_some() {
                let o = Opts::text(app.t(label))
                    .help(app.t("dw.wlTimeHelp"))
                    .range(0.0, 100_000.0);
                widgets::num_field(app, ui, &at(&[key]), o);
            }
        }
        let o = Opts::text(app.t("dw.happiness"))
            .range(lim.happiness_min as f64, lim.happiness_max as f64)
            .unit(app.t("c.units.percent"));
        widgets::num_field(app, ui, &at(&["happiness", "happinessValue"]), o);
    });
    ui.add_space(8.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.state")).font(style::bold(16.0)));
        let max_hp = d
            .at(&vault::p(&["health", "maxHealth"]))
            .and_then(Value::as_f64)
            .unwrap_or(100.0);
        let o = Opts::text(app.t("dw.health"))
            .range(0.0, max_hp.floor())
            .unit(app.t("c.units.hp"));
        widgets::num_field(app, ui, &at(&["health", "healthValue"]), o);
        let o = Opts::text(app.t("dw.maxHealth")).unit(app.t("c.units.hp"));
        widgets::num_field(app, ui, &at(&["health", "maxHealth"]), o);
        let o = Opts::text(app.t("dw.radiation"))
            .range(0.0, max_hp.floor())
            .unit(app.t("c.units.rad"));
        widgets::num_field(app, ui, &at(&["health", "radiationValue"]), o);
        for key in ["permaDeath", "lastLevelUpdated"] {
            if !expert && key != "permaDeath" {
                continue;
            }
            let path = at(&["health", key]);
            let o = Opts::at(app, &path);
            match root.at(&path) {
                Some(Value::Bool(_)) => widgets::bool_field(app, ui, &path, o),
                Some(Value::Number(_)) => widgets::num_field(app, ui, &path, o),
                _ => {}
            }
        }
        for key in ["deathTime", "deathSource"] {
            if expert && d.get(key).is_some() {
                let o = Opts::at(app, &at(&[key]));
                widgets::num_field(app, ui, &at(&[key]), o);
            }
        }
    });
    ui.add_space(8.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.special")).font(style::bold(16.0)));
        for s in 1..=7usize {
            let mut path = at(&["stats", "stats"]);
            path.push(Seg::Index(s));
            path.push(Seg::Key("value".into()));
            let label = app.t(&format!("dw.specialName.{s}"));
            let o = Opts::text(format!("{} – {label}", SPECIAL_LETTERS[s - 1]))
                .help(app.t(&format!("dw.specialHelp.{s}")))
                .range(lim.special_min as f64, lim.special_max as f64);
            widgets::num_field(app, ui, &path, o);
        }
    });
    ui.add_space(8.0);
    looks(app, ui, &d, id, &base);
    ui.add_space(8.0);
    equipment(app, ui, &d, id, &base);
    ui.add_space(8.0);
    relations(app, ui, &d, id, &base, &root);
    ui.add_space(8.0);
    panel(ui, |ui| {
        ui.label(
            RichText::new(app.t(if expert { "dw.flags" } else { "dw.placement" }))
                .font(style::bold(16.0)),
        );
        for key in [
            "pregnant",
            "babyReady",
            "assigned",
            "sawIncident",
            "WillGoToWasteland",
            "WillBeEvicted",
            "IsEvictedWaitingForFollowers",
            "wasTemporarilyAssigned",
        ] {
            if expert && d.get(key).is_some() {
                let o = Opts::at(app, &at(&[key]));
                widgets::bool_field(app, ui, &at(&[key]), o);
            }
        }
        let rooms = vault::arr(&root, &vault::ROOMS);
        let current = rooms.iter().find(|r| {
            r.get("dwellers")
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().any(|v| v.as_i64() == Some(id)))
        });
        let cur_id = current
            .and_then(|r| vault::int(r, &["deserializeID"]))
            .unwrap_or(-1);
        let o = Opts::at(app, &at(&["savedRoom"])).with_label(app.t("dw.room"));
        widgets::row(app, ui, &at(&["savedRoom"]), &o, |app, ui| {
            let mut pick = None;
            egui::ComboBox::from_id_salt(("room", id))
                .selected_text(current.map_or_else(
                    || app.t("dw.noRoom"),
                    |r| crate::sec_vault::room_title(app, r),
                ))
                .width(ui.available_width().min(260.0))
                .height(400.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(cur_id == -1, app.t("dw.noRoom"))
                        .clicked()
                    {
                        pick = Some(-1);
                    }
                    let mut sorted: Vec<&Value> = rooms
                        .iter()
                        .filter(|r| {
                            !matches!(
                                vault::text(r, &["type"]).as_deref(),
                                Some("Elevator" | "Entrance" | "FakeWasteland")
                            )
                        })
                        .collect();
                    sorted.sort_by_key(|r| {
                        (
                            vault::int(r, &["row"]).unwrap_or(0),
                            vault::int(r, &["col"]).unwrap_or(0),
                        )
                    });
                    for r in sorted {
                        let rid = vault::int(r, &["deserializeID"]).unwrap_or(-1);
                        if ui
                            .selectable_label(rid == cur_id, crate::sec_vault::room_title(app, r))
                            .clicked()
                        {
                            pick = Some(rid);
                        }
                    }
                });
            if let Some(r) = pick {
                if r != cur_id {
                    app.act(Action::AssignRoom {
                        ids: vec![id],
                        room: r,
                    });
                }
            }
        });
    });
}

/// Outfit the game draws on a dweller: the maternity dress while pregnant.
fn worn_outfit(d: &Value) -> Option<String> {
    let male = vault::int(d, &["gender"]) == Some(2);
    if !male && vault::flag(d, &["pregnant"]) == Some(true) {
        return Some("__pregnant".into());
    }
    d.get("equipedOutfit")
        .and_then(|o| o.get("id"))
        .and_then(Value::as_text)
}

/// The dweller as the game draws it inside the vault, in the outfit they wear.
fn preview(app: &mut App, ui: &mut egui::Ui, d: &Value, id: i64, max_h: f32) {
    let c = style::colors(ui.ctx());
    let happiness = d
        .at(&vault::p(&["happiness", "happinessValue"]))
        .and_then(Value::as_f64)
        .unwrap_or(50.0);
    let male = vault::int(d, &["gender"]) == Some(2);
    let worn = worn_outfit(d);
    let art = worn
        .as_deref()
        .and_then(|o| crate::art::outfit_art(male, o));
    let tint = if worn.as_deref() == Some("__pregnant") {
        0xFFFF_FFFF
    } else {
        crate::art::outfit_color(art, vault::int(d, &["outfitColor"]).unwrap_or(0xFFFF_FFFF))
    };
    let look = crate::art::Look {
        male,
        skin: crate::art::rgb(vault::int(d, &["skinColor"]).unwrap_or(0xFFFF_E9D6)),
        hair_color: crate::art::rgb(vault::int(d, &["hairColor"]).unwrap_or(0xFFCF_B038)),
        hair: vault::text(d, &["hair"]),
        face_mask: vault::text(d, &["faceMask"]).filter(|m| !m.is_empty()),
        face: if vault::is_dead(d) {
            "Dead".into()
        } else {
            crate::art::face_for(happiness)
        },
        outfit: art.and(worn.clone()),
        outfit_color: crate::art::rgb(i64::from(tint)),
        helmet: vault::text(d, &["helmet"]),
        helmet_cover: vault::text(d, &["helmetCoverCustomization"]),
    };
    let status = app.art.status();
    let tex = app.art.dweller(ui.ctx(), id, &look);
    match (status, tex) {
        (_, Some(t)) => {
            let size = t.size_vec2();
            let h = max_h.min(size.y);
            let w = size.x * h / size.y.max(1.0);
            egui::Frame::new()
                .fill(c.surface_alt)
                .corner_radius(8)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.add(egui::Image::new(&t).fit_to_exact_size(egui::vec2(w, h)))
                        .on_hover_text(app.t("dw.previewHelp"));
                });
            let gender = vault::int(d, &["gender"]).unwrap_or(1);
            let note = match art {
                None if worn
                    .as_deref()
                    .is_some_and(|w| !catalog().outfit_fits(w, gender)) =>
                {
                    Some("dw.previewWrongGender")
                }
                None if look.outfit.is_none() && d.get("equipedOutfit").is_some() => {
                    Some("dw.previewNoOutfit")
                }
                Some(a) if a.large_headgear => Some("dw.previewNoHeadgear"),
                _ => None,
            };
            if let Some(n) = note {
                ui.add(egui::Label::new(RichText::new(app.t(n)).small().color(c.muted)).wrap());
            }
        }
        (crate::art::Status::Ready | crate::art::Status::Loading, None) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new(app.t("dw.previewLoading")).color(c.muted));
            });
        }
        (crate::art::Status::Mismatch, None) => {
            ui.add(
                egui::Label::new(RichText::new(app.t("art.mismatch")).small().color(c.muted))
                    .wrap(),
            );
        }
        (crate::art::Status::NoGame, None) => {
            ui.add(
                egui::Label::new(RichText::new(app.t("art.noGame")).small().color(c.muted)).wrap(),
            );
        }
    }
}

fn looks(app: &mut App, ui: &mut egui::Ui, d: &Value, id: i64, base: &[Seg]) {
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.looks")).font(style::bold(16.0)));
        // Preview on the left and the controls beside it, so the palette never covers the picture.
        let h = if ui.available_width() < 420.0 {
            190.0
        } else {
            260.0
        };
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_min_width(h * 0.5 + 16.0);
                ui.set_max_width(h * 0.6 + 16.0);
                preview(app, ui, d, id, h);
            });
            ui.vertical(|ui| {
                look_colors(app, ui, d, id, base);
                look_parts(app, ui, d, id, base);
            });
        });
    });
}

/// Skin, hair and outfit colours. The palette opens inside the card; while it
/// is dragged only the swatch changes, and the colour is written on release.
fn look_colors(app: &mut App, ui: &mut egui::Ui, d: &Value, id: i64, base: &[Seg]) {
    let open_key = format!("cpick:{id}");
    for (field, label) in [
        ("skinColor", "dw.skin"),
        ("hairColor", "dw.hair"),
        ("outfitColor", "dw.outfitColor"),
    ] {
        let Some(v) = vault::int(d, &[field]) else {
            continue;
        };
        let path = join(base, &[field]);
        if field == "outfitColor" && outfit_colors(app, ui, d, id, &path, v) {
            continue;
        }
        let pending_key = format!("pcolor:{id}:{field}");
        let pending = app
            .bufs
            .get(&pending_key)
            .and_then(|h| u32::from_str_radix(h, 16).ok());
        let rgb = rgb_of(pending.map_or(v, i64::from));
        let is_open = app.bufs.get(&open_key).map(String::as_str) == Some(field);
        let o = Opts::text(app.t(label));
        widgets::row(app, ui, &path, &o, |app, ui| {
            let swatch = egui::widgets::color_picker::show_color(
                ui,
                Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                egui::vec2(40.0, 22.0),
            )
            .interact(Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(app.t("dw.colorPick"));
            if swatch.clicked() {
                if is_open {
                    app.bufs.remove(&open_key);
                } else {
                    app.bufs.insert(open_key.clone(), field.to_string());
                }
            }
            let key = format!("color:{id}:{field}");
            let shown = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            let mut text = app.bufs.get(&key).cloned().unwrap_or_else(|| shown.clone());
            let resp = ui
                .add(
                    egui::TextEdit::singleline(&mut text)
                        .desired_width(90.0)
                        .font(style::mono(13.0)),
                )
                .on_hover_text(app.t("dw.colorCode"));
            if resp.changed() {
                app.bufs.insert(key.clone(), text.clone());
            }
            if resp.lost_focus() {
                app.bufs.remove(&key);
                if let Ok(packed) = u32::from_str_radix(text.trim().trim_start_matches('#'), 16) {
                    if text.trim().trim_start_matches('#').len() == 6 && text != shown {
                        app.act(Action::SetColor {
                            ids: vec![id],
                            field: field.into(),
                            rgb: packed,
                        });
                    }
                }
            }
        });
        let gender = if vault::int(d, &["gender"]) == Some(2) {
            "male"
        } else {
            "female"
        };
        let presets = catalog().looks.get(gender).map(|l| match field {
            "skinColor" => l.skin_colors.as_slice(),
            "hairColor" => l.hair_colors.as_slice(),
            _ => &[],
        });
        if let Some(list) = presets.filter(|l| !l.is_empty()) {
            let cur = (v as u32) & 0x00ff_ffff;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for col in list {
                    let rgb = rgb_of(i64::from(*col));
                    let resp = egui::widgets::color_picker::show_color(
                        ui,
                        Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                        egui::vec2(18.0, 14.0),
                    )
                    .interact(Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(app.t("dw.gameColor"));
                    if col & 0x00ff_ffff == cur {
                        ui.painter().rect_stroke(
                            resp.rect.expand(1.5),
                            2.0,
                            egui::Stroke::new(1.5, style::colors(ui.ctx()).accent),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if resp.clicked() && col & 0x00ff_ffff != cur {
                        app.act(Action::SetColor {
                            ids: vec![id],
                            field: field.into(),
                            rgb: col & 0x00ff_ffff,
                        });
                    }
                }
            });
        }
        if is_open {
            if let Some(c) = palette(ui, ("palette", id, field), rgb) {
                let packed = (u32::from(c[0]) << 16) | (u32::from(c[1]) << 8) | u32::from(c[2]);
                app.bufs
                    .insert(pending_key.clone(), format!("{packed:06x}"));
            }
            if ui
                .small_button(format!("{}  {}", icons::CHECK, app.t("dw.paletteClose")))
                .clicked()
            {
                app.bufs.remove(&open_key);
            }
            ui.add_space(4.0);
        }
        if !ui.input(|i| i.pointer.any_down()) {
            if let Some(packed) = app
                .bufs
                .remove(&pending_key)
                .and_then(|h| u32::from_str_radix(&h, 16).ok())
            {
                app.act(Action::SetColor {
                    ids: vec![id],
                    field: field.into(),
                    rgb: packed,
                });
            }
        }
    }
}

/// Saturation/brightness square and hue strip; returns the colour while it is changed.
fn palette(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    rgb: [u8; 3],
) -> Option<[u8; 3]> {
    use egui::ecolor::Hsva;
    let id = ui.make_persistent_id(salt);
    // The hue is kept while dragging, so grey and black do not reset it.
    let mut hsv = Hsva::from_srgb(rgb);
    let kept: Option<f32> = ui.data(|d| d.get_temp(id));
    if let Some(h) = kept.filter(|_| hsv.s < 0.01 || hsv.v < 0.01) {
        hsv.h = h;
    }
    let w = ui.available_width().clamp(120.0, 240.0);
    let n = 16;
    let (square, sq) = ui.allocate_exact_size(egui::vec2(w, w * 0.6), Sense::click_and_drag());
    let (strip, hs) = ui.allocate_exact_size(egui::vec2(w, 18.0), Sense::click_and_drag());
    let mut mesh = egui::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (s, v) = (i as f32 / n as f32, 1.0 - j as f32 / n as f32);
            let pos = square.min + egui::vec2(square.width() * s, square.height() * (1.0 - v));
            mesh.colored_vertex(pos, Color32::from(Hsva::new(hsv.h, s, v, 1.0)));
        }
    }
    let row = n + 1;
    for j in 0..n {
        for i in 0..n {
            let a = j * row + i;
            mesh.add_triangle(a, a + 1, a + row);
            mesh.add_triangle(a + 1, a + row + 1, a + row);
        }
    }
    for i in 0..=n {
        let h = i as f32 / n as f32;
        let x = strip.min.x + strip.width() * h;
        let c = Color32::from(Hsva::new(h, 1.0, 1.0, 1.0));
        mesh.colored_vertex(egui::pos2(x, strip.min.y), c);
        mesh.colored_vertex(egui::pos2(x, strip.max.y), c);
    }
    let base = row * row;
    for i in 0..n {
        let a = base + 2 * i;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    let painter = ui.painter();
    painter.add(egui::Shape::mesh(mesh));
    let stroke = egui::Stroke::new(2.0, Color32::WHITE);
    let dark = egui::Stroke::new(1.0, Color32::BLACK);
    let mark = square.min + egui::vec2(square.width() * hsv.s, square.height() * (1.0 - hsv.v));
    painter.circle_stroke(mark, 6.0, dark);
    painter.circle_stroke(mark, 5.0, stroke);
    let hx = strip.min.x + strip.width() * hsv.h;
    let bar = egui::Rect::from_min_max(
        egui::pos2(hx - 2.0, strip.min.y - 2.0),
        egui::pos2(hx + 2.0, strip.max.y + 2.0),
    );
    painter.rect_stroke(bar, 1.0, stroke, egui::StrokeKind::Middle);
    let mut changed = false;
    if let Some(p) = sq.interact_pointer_pos() {
        hsv.s = ((p.x - square.min.x) / square.width()).clamp(0.0, 1.0);
        hsv.v = (1.0 - (p.y - square.min.y) / square.height()).clamp(0.0, 1.0);
        changed = true;
    }
    if let Some(p) = hs.interact_pointer_pos() {
        hsv.h = ((p.x - strip.min.x) / strip.width()).clamp(0.0, 0.9999);
        changed = true;
    }
    if !changed {
        return None;
    }
    ui.data_mut(|d| d.insert_temp(id, hsv.h));
    Some(hsv.to_srgb())
}

/// Outfit colour as the game allows it: one of the outfit's own colours.
/// Returns false when the outfit is unknown here, so the free palette is shown.
fn outfit_colors(
    app: &mut App,
    ui: &mut egui::Ui,
    d: &Value,
    id: i64,
    path: &[Seg],
    v: i64,
) -> bool {
    let male = vault::int(d, &["gender"]) == Some(2);
    let Some(worn) = worn_outfit(d) else {
        return false;
    };
    let c = style::colors(ui.ctx());
    let gender = if male { 2 } else { 1 };
    let Some(art) = crate::art::outfit_art(male, &worn) else {
        if catalog().outfit_fits(&worn, gender) {
            return false;
        }
        let o = Opts::text(app.t("dw.outfitColor")).help(app.t("dw.outfitColorHelp"));
        widgets::row(app, ui, path, &o, |app, ui| {
            ui.add(
                egui::Label::new(RichText::new(app.t("dw.outfitColorWrong")).color(c.muted)).wrap(),
            );
        });
        return true;
    };
    let used = crate::art::outfit_color(Some(art), v);
    let o = Opts::text(app.t("dw.outfitColor")).help(app.t("dw.outfitColorHelp"));
    widgets::row(app, ui, path, &o, |app, ui| {
        if worn == "__pregnant" {
            ui.label(RichText::new(app.t("dw.outfitColorPregnant")).color(c.muted));
            return;
        }
        if art.colors.is_empty() {
            ui.label(RichText::new(app.t("dw.outfitColorFixed")).color(c.muted));
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for col in &art.colors {
                let rgb = rgb_of(i64::from(*col));
                let resp = egui::widgets::color_picker::show_color(
                    ui,
                    Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                    egui::vec2(28.0, 22.0),
                )
                .interact(Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(format!("#{:06X}", col & 0x00ff_ffff));
                if *col == used {
                    ui.painter().rect_stroke(
                        resp.rect.expand(2.0),
                        3.0,
                        egui::Stroke::new(2.0, c.accent),
                        egui::StrokeKind::Outside,
                    );
                }
                if resp.clicked() && (v as u32) != *col {
                    app.act(Action::SetColor {
                        ids: vec![id],
                        field: "outfitColor".into(),
                        rgb: col & 0x00ff_ffff,
                    });
                }
            }
        });
    });
    true
}

/// Hair and face mask pickers.
fn look_parts(app: &mut App, ui: &mut egui::Ui, d: &Value, id: i64, base: &[Seg]) {
    let gender = vault::int(d, &["gender"]).unwrap_or(1);
    for (field, label, none) in [
        ("hair", "dw.hairStyle", "dw.bald"),
        ("faceMask", "dw.faceMask", "c.none"),
        ("helmet", "dw.helmet", "c.none"),
    ] {
        let path = join(base, &[field]);
        let cur = vault::text(d, &[field]).unwrap_or_default();
        let mut choices = vec![(String::new(), app.t(none))];
        choices.extend(
            shelter_core::actions::look_choices(gender, field)
                .into_iter()
                .map(|n| {
                    let name = if field == "helmet" {
                        hat_name(app, n)
                    } else {
                        None
                    };
                    (n.to_string(), name.unwrap_or_else(|| look_name(app, n)))
                }),
        );
        if !cur.is_empty() && !choices.iter().any(|(v, _)| *v == cur) {
            choices.push((cur.clone(), cur.clone()));
        }
        let o = Opts::text(app.t(label)).help(app.t(&format!("{label}Help")));
        widgets::row(app, ui, &path, &o, |app, ui| {
            let shown = choices
                .iter()
                .find(|(v, _)| *v == cur)
                .map_or_else(|| cur.clone(), |(_, l)| l.clone());
            let mut pick = None;
            egui::ComboBox::from_id_salt(("look", id, field))
                .selected_text(shown.clone())
                .truncate()
                .width(ui.available_width().min(220.0))
                .height(360.0)
                .show_ui(ui, |ui| {
                    for (v, l) in &choices {
                        if ui.selectable_label(*v == cur, l).clicked() && *v != cur {
                            pick = Some(v.clone());
                        }
                    }
                })
                .response
                .on_hover_text(shown);
            if let Some(value) = pick {
                app.act(Action::SetLook {
                    id,
                    field: field.into(),
                    value,
                });
            }
        });
    }
}

fn hat_name(app: &App, key: &str) -> Option<String> {
    app.tx().hat_name(key)
}

fn look_name(app: &App, name: &str) -> String {
    app.tx().look_name(name)
}

fn equipment(app: &mut App, ui: &mut egui::Ui, d: &Value, id: i64, base: &[Seg]) {
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.equipment")).font(style::bold(16.0)));
        for (slot, kind, key, label) in [
            (Slot::Weapon, ItemKind::Weapon, "equipedWeapon", "dw.weapon"),
            (Slot::Outfit, ItemKind::Outfit, "equipedOutfit", "dw.outfit"),
            (Slot::Pet, ItemKind::Pet, "equippedPet", "dw.pet"),
        ] {
            let cur = d
                .get(key)
                .and_then(|v| v.get("id"))
                .and_then(Value::as_text);
            let path = join(base, &[key]);
            let o = Opts::text(app.t(label));
            let shown = cur.as_ref().map_or_else(
                || app.t("dw.noPet"),
                |i| {
                    catalog().item(kind, i).map_or_else(
                        || i.clone(),
                        |x| x.name.get(app.settings.russian_item_names).to_string(),
                    )
                },
            );
            let detail = cur.as_ref().map(|i| match kind {
                ItemKind::Weapon => catalog()
                    .weapon(i)
                    .map(|w| {
                        app.tf(
                            "it.damage",
                            &[
                                ("min", w.damage_min.to_string()),
                                ("max", w.damage_max.to_string()),
                            ],
                        )
                    })
                    .unwrap_or_default(),
                ItemKind::Outfit => catalog()
                    .outfit(i)
                    .map(|o| {
                        o.special
                            .iter()
                            .zip(SPECIAL_LETTERS)
                            .filter(|(v, _)| **v != 0)
                            .map(|(v, l)| format!("{l}+{v}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default(),
                _ => String::new(),
            });
            widgets::row(app, ui, &path, &o, |app, ui| {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        let name = ui.add(
                            egui::Label::new(RichText::new(&shown).font(style::bold(14.0)))
                                .sense(Sense::hover()),
                        );
                        if let Some(i) = &cur {
                            crate::sec_vault::item_hover(app, name, kind, i);
                        }
                        if let Some(i) = &cur {
                            if app.expert()
                                && ui
                                    .add(
                                        egui::Label::new(RichText::new(i).font(style::mono(11.0)))
                                            .sense(Sense::click()),
                                    )
                                    .on_hover_text(app.t("c.copy"))
                                    .clicked()
                            {
                                ui.ctx().copy_text(i.clone());
                            }
                            if catalog().item(kind, i).is_none() {
                                ui.label(RichText::new(app.t("c.unknownItem")).small());
                            }
                        }
                        if let Some(det) = &detail {
                            ui.label(RichText::new(det).small());
                        }
                    });
                    ui.horizontal(|ui| {
                        let key = format!("eq:{id}:{key}");
                        let mut q = app.bufs.remove(&key).unwrap_or_default();
                        let gender = vault::int(d, &["gender"]);
                        let picked = crate::sec_vault::item_picker_for(
                            app,
                            ui,
                            &key,
                            &[kind],
                            &mut q,
                            gender,
                        );
                        if !q.is_empty() {
                            app.bufs.insert(key.clone(), q);
                        }
                        if let Some((_, item)) = picked {
                            app.bufs.remove(&key);
                            app.act(Action::Equip {
                                ids: vec![id],
                                slot,
                                item: Some(item),
                            });
                        }
                        if slot == Slot::Pet
                            && cur.is_some()
                            && ui.button(app.t("dw.noPet")).clicked()
                        {
                            app.act(Action::Equip {
                                ids: vec![id],
                                slot,
                                item: None,
                            });
                        }
                    });
                });
            });
        }
        if let Some(extra) = d.at(&vault::p(&["equippedPet", "extraData"])) {
            let pet_id = vault::text(d, &["equippedPet", "id"]).unwrap_or_default();
            pet_details(app, ui, extra, id, &pet_id);
        }
    });
}

fn pet_details(app: &mut App, ui: &mut egui::Ui, extra: &Value, id: i64, pet_id: &str) {
    let name = vault::text(extra, &["uniqueName"]).unwrap_or_default();
    let bonus = vault::text(extra, &["bonus"]).unwrap_or_default();
    let value = vault::int(extra, &["bonusValue"]).unwrap_or(0);
    let pet_range = catalog()
        .pet(pet_id)
        .filter(|p| p.bonus == bonus)
        .map(|p| (p.bonus_min, p.bonus_max));
    let key = format!("pet:{id}");
    let mut buf = app.bufs.get(&key).cloned().unwrap_or_else(|| name.clone());
    ui.add_space(4.0);
    egui::Grid::new(("pet", id))
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            ui.label(app.t("dw.petName"));
            let r = ui.add(
                egui::TextEdit::singleline(&mut buf).desired_width(ui.available_width().min(220.0)),
            );
            if r.changed() {
                app.bufs.insert(key.clone(), buf.clone());
            }
            if r.lost_focus() && buf != name {
                app.bufs.remove(&key);
                app.act(Action::SetPetDetails {
                    id,
                    name: buf.clone(),
                    bonus: bonus.clone(),
                    bonus_value: value,
                });
            }
            ui.end_row();
            ui.label(app.t("dw.petBonus"));
            let mut pick = None;
            let shown = catalog().pet_bonuses.get(&bonus).map_or_else(
                || bonus.clone(),
                |n| {
                    n.get(app.lang == Lang::Ru)
                        .replace("{0}", &value.to_string())
                },
            );
            egui::ComboBox::from_id_salt(("bonus", id))
                .selected_text(shown)
                .width(ui.available_width().min(260.0))
                .show_ui(ui, |ui| {
                    for (k, n) in &catalog().pet_bonuses {
                        if ui
                            .selectable_label(
                                *k == bonus,
                                n.get(app.lang == Lang::Ru).replace("{0}", "…"),
                            )
                            .clicked()
                        {
                            pick = Some(k.clone());
                        }
                    }
                });
            if let Some(b) = pick {
                app.act(Action::SetPetDetails {
                    id,
                    name: name.clone(),
                    bonus: b,
                    bonus_value: value,
                });
            }
            ui.end_row();
            ui.label(app.t("dw.petBonusValue"));
            let range = if app.expert() {
                (f64::NEG_INFINITY, f64::INFINITY)
            } else {
                pet_range.map_or((0.0, 1000.0), |(a, b)| (a.floor(), b.ceil()))
            };
            let range = (range.0.min(value as f64), range.1.max(value as f64));
            if let Some(v) = widgets::drag_number(
                app,
                ui,
                &format!("petbonus:{id}"),
                value as f64,
                range,
                true,
            ) {
                app.act(Action::SetPetDetails {
                    id,
                    name: name.clone(),
                    bonus: bonus.clone(),
                    bonus_value: v as i64,
                });
            }
            ui.end_row();
        });
}

fn relations(app: &mut App, ui: &mut egui::Ui, d: &Value, id: i64, base: &[Seg], root: &Value) {
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.relations")).font(style::bold(16.0)));
        let partner = vault::int(d, &["relations", "partner"]).unwrap_or(-1);
        let name_of = |pid: i64| {
            vault::dweller_index(root, pid).map_or_else(
                || format!("#{pid}"),
                |j| vault::dweller_name(&vault::arr(root, &vault::DWELLERS)[j]),
            )
        };
        let o = Opts::text(app.t("dw.partner"));
        widgets::row(
            app,
            ui,
            &join(base, &["relations", "partner"]),
            &o,
            |app, ui| {
                let mut pick = None;
                let shown = if partner == -1 {
                    app.t("c.none")
                } else {
                    name_of(partner)
                };
                egui::ComboBox::from_id_salt(("partner", id))
                    .selected_text(shown)
                    .width(ui.available_width().min(240.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(partner == -1, app.t("c.none"))
                            .clicked()
                        {
                            pick = Some(-1);
                        }
                        for other in vault::arr(root, &vault::DWELLERS) {
                            let Some(oid) = vault::int(other, &["serializeId"]) else {
                                continue;
                            };
                            if oid == id {
                                continue;
                            }
                            if ui
                                .selectable_label(oid == partner, vault::dweller_name(other))
                                .clicked()
                            {
                                pick = Some(oid);
                            }
                        }
                    });
                if let Some(p) = pick {
                    if p != partner {
                        app.act(Action::SetPartner { id, partner: p });
                    }
                }
            },
        );
        let last = vault::int(d, &["relations", "lastPartner"]).unwrap_or(-1);
        let o = Opts::text(app.t("dw.lastPartner"));
        widgets::row(
            app,
            ui,
            &join(base, &["relations", "lastPartner"]),
            &o,
            |app, ui| {
                ui.label(if last == -1 {
                    app.t("c.none")
                } else {
                    name_of(last)
                });
            },
        );
        if app.expert() {
            let o = Opts::at(app, &join(base, &["lastChildBorn"]));
            if d.get("lastChildBorn").is_some() {
                widgets::num_field(app, ui, &join(base, &["lastChildBorn"]), o);
            }
            egui::CollapsingHeader::new(
                Opts::at(app, &join(base, &["relations", "ascendants"])).label,
            )
            .id_salt(("asc", id))
            .show(ui, |ui| {
                widgets::object_form(app, ui, &join(base, &["relations", "ascendants"]), 0);
            });
        } else {
            let asc_path = join(base, &["relations", "ascendants"]);
            let names: Vec<String> = d
                .at(&vault::p(&["relations", "ascendants"]))
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_i64)
                        .filter(|x| *x >= 0)
                        .map(&name_of)
                        .collect()
                })
                .unwrap_or_default();
            let shown = if names.is_empty() {
                app.t("dw.noAscendants")
            } else {
                names.join(", ")
            };
            let o = Opts::at(app, &asc_path);
            widgets::row(app, ui, &asc_path, &o, |_, ui| {
                ui.add(egui::Label::new(shown).wrap());
            });
        }
    });
}

fn pets(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let actors = vault::arr(root, &vault::ACTORS);
    let expert = app.expert();
    let ru = app.settings.russian_item_names;
    panel(ui, |ui| {
        let c = style::colors(ui.ctx());
        ui.label(RichText::new(app.t("dw.pets")).font(style::bold(16.0)));
        ui.add(egui::Label::new(RichText::new(app.t("dw.petsHelp")).small().color(c.muted)).wrap());
        if ui
            .button(format!("{}  {}", icons::PAW, app.t("dw.petFromJournal")))
            .on_hover_text(app.t("dw.petFromJournalHelp"))
            .clicked()
        {
            app.modals.push(Modal::FromJournal {
                pets: true,
                query: String::new(),
                pick: None,
                bonus: 0.0,
            });
        }
        if actors.is_empty() {
            ui.label(RichText::new(app.t("c.empty")).color(c.muted));
        }
        for (i, a) in actors.iter().enumerate() {
            let is_pet = vault::int(a, &["characterType"]) == Some(3);
            let owner = vault::int(a, &["FollowedID"]).unwrap_or(-1);
            let owner_name = vault::dweller_index(root, owner)
                .map(|j| vault::dweller_name(&vault::arr(root, &vault::DWELLERS)[j]));
            let title = if is_pet {
                let id = vault::text(a, &["actorDataId"]).unwrap_or_default();
                let pet = catalog().pet(&id);
                let breed = pet.map_or(id.clone(), |p| p.name.get(ru).to_string());
                let name = vault::text(a, &["name"]).unwrap_or_default();
                let default_name =
                    pet.is_some_and(|p| p.name.en == name || p.name.get(true) == name);
                let mut t = format!("{}  {breed}", icons::PAW);
                if !name.is_empty() && name != breed && !default_name {
                    t.push_str(&format!(" «{name}»"));
                }
                match owner_name {
                    Some(o) => format!("{t}  ·  {}", app.tf("dw.follows", &[("name", o)])),
                    None => t,
                }
            } else {
                let room = vault::int(a, &["savedRoom"]).and_then(|r| vault::room_index(root, r));
                let mut t = format!("{}  {}", icons::WRENCH, app.t("dw.mrHandy"));
                if let Some(ri) = room {
                    t.push_str(&format!(
                        "  ·  {}",
                        crate::sec_vault::room_title(app, &vault::arr(root, &vault::ROOMS)[ri])
                    ));
                }
                t
            };
            if expert {
                egui::CollapsingHeader::new(title)
                    .id_salt(("actor", i))
                    .show(ui, |ui| {
                        widgets::object_form(app, ui, &pi(&vault::ACTORS, i, &[]), 1);
                    });
            } else if is_pet {
                ui.add(egui::Label::new(title).wrap());
            } else {
                egui::CollapsingHeader::new(title)
                    .id_salt(("handy", i))
                    .show(ui, |ui| mr_handy(app, ui, a, i));
            }
        }
        ui.add_space(4.0);
        if ui
            .button(format!("{}  {}", icons::WRENCH, app.t("dw.handyAdd")))
            .on_hover_text(app.t("dw.handyAddHelp"))
            .clicked()
        {
            let mut counts = [0i64; 4];
            for t in vault::arr(root, &["vault", "LunchBoxesByType"]) {
                if let Some(k) = t.as_i64().and_then(|k| usize::try_from(k).ok()) {
                    if k < 4 {
                        counts[k] += 1;
                    }
                }
            }
            counts[1] += 1;
            app.act(Action::SetLunchboxes { counts });
        }
    });
}

/// Name, health and work settings of one Mr. Handy.
fn mr_handy(app: &mut App, ui: &mut egui::Ui, a: &Value, i: usize) {
    let at = |k: &str| pi(&vault::ACTORS, i, &[k]);
    let max = catalog().limits.mr_handy_health as f64;
    if a.get("name").is_some() {
        widgets::text_field(app, ui, &at("name"), Opts::text(app.t("dw.handyName")));
    }
    let dead = vault::flag(a, &["death"]) == Some(true);
    if dead {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(app.t("dw.handyBroken")).color(style::colors(ui.ctx()).danger));
            if ui
                .button(format!("{}  {}", icons::SPARKLES, app.t("dw.handyRepair")))
                .clicked()
            {
                app.act(Action::SetBool {
                    path: at("death"),
                    value: false,
                });
                app.act(Action::SetNumber {
                    path: at("health"),
                    value: max,
                });
            }
        });
    }
    if a.get("health").is_some() {
        let o = Opts::text(app.t("dw.handyHealth"))
            .help(app.t("dw.handyHealthHelp"))
            .range(0.0, max)
            .unit(app.t("c.units.hp"));
        widgets::num_field(app, ui, &at("health"), o);
    }
    if a.get("canCollect").is_some() {
        let o = Opts::text(app.t("dw.handyCollect")).help(app.t("dw.handyCollectHelp"));
        widgets::bool_field(app, ui, &at("canCollect"), o);
    }
}

/// Dwellers the game will bring through the door when the vault loads.
fn waiting(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let path = vault::p(&["dwellerSpawner", "dwellersWaiting"]);
    let list = vault::arr(root, &["dwellerSpawner", "dwellersWaiting"]);
    if list.is_empty() && !app.expert() {
        return;
    }
    panel(ui, |ui| {
        let c = style::colors(ui.ctx());
        ui.label(RichText::new(app.t("dw.waiting")).font(style::bold(16.0)));
        ui.add(
            egui::Label::new(
                RichText::new(app.t("dw.waitingHelp"))
                    .small()
                    .color(c.muted),
            )
            .wrap(),
        );
        for w in list {
            let id = vault::int(w, &["dwellerId"]).unwrap_or(-1);
            let name = if vault::text(w, &["charType"]).as_deref() == Some("MrHandy") {
                app.t("dw.mrHandy")
            } else {
                vault::dweller_index(root, id)
                    .map(|j| vault::dweller_name(&vault::arr(root, &vault::DWELLERS)[j]))
                    .unwrap_or_else(|| app.t("dw.waitingNew"))
            };
            ui.label(format!("{}  {name}", icons::USER_PLUS));
        }
        if app.expert() {
            egui::CollapsingHeader::new(app.t("c.raw"))
                .id_salt("waiting_raw")
                .show(ui, |ui| widgets::object_form(app, ui, &path, 0));
        }
    });
}
