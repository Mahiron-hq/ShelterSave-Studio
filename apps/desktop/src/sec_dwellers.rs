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
    let root = open.doc.root().clone();
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
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            mass_bar(app, ui);
            ui.add_space(8.0);
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
            panel(ui, |ui| {
                ui.label(RichText::new(app.t("dw.waiting")).font(style::bold(16.0)));
                widgets::object_form(
                    app,
                    ui,
                    &vault::p(&["dwellerSpawner", "dwellersWaiting"]),
                    0,
                );
            });
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
    ui.horizontal(|ui| {
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
    ui.separator();
    if shown.is_empty() {
        ui.label(RichText::new(app.t("c.noResults")).color(c.muted));
        return;
    }
    let row_h = 44.0;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        row_h,
        shown.len(),
        |ui, range| {
            for r in &shown[range] {
                let focused = app.dw.focus == Some(r.id);
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
                            if ui.checkbox(&mut on, "").changed() {
                                if on {
                                    app.dw.selected.insert(r.id);
                                } else {
                                    app.dw.selected.remove(&r.id);
                                }
                            }
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
                                    ui.label(title);
                                });
                                ui.label(
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
                                );
                            });
                        });
                    });
                let resp = resp.response.interact(Sense::click());
                if resp.clicked() {
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
        },
    );
}

fn mass_bar(app: &mut App, ui: &mut egui::Ui) {
    let ids: Vec<i64> = app.dw.selected.iter().copied().collect();
    let n = ids.len();
    let c = style::colors(ui.ctx());
    panel(ui, |ui| {
        ui.label(
            RichText::new(format!(
                "{} — {}",
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
                        Action::Revive { ids: ids.clone() },
                    ),
                ];
                for (icon, key, action) in items {
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
    let root = open.doc.root().clone();
    let Some(i) = vault::dweller_index(&root, id) else {
        return;
    };
    let d = vault::arr(&root, &vault::DWELLERS)[i].clone();
    let base = pi(&vault::DWELLERS, i, &[]);
    let at = |rest: &[&str]| join(&base, rest);
    let c = style::colors(ui.ctx());
    let lim = &catalog().limits;
    panel(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(vault::dweller_name(&d)).font(style::bold(20.0)));
            ui.label(
                RichText::new(format!("#{id}"))
                    .font(style::mono(13.0))
                    .color(c.muted),
            );
            if vault::is_dead(&d) {
                crate::app::badge(ui, &app.t("dw.dead"), c.danger_soft, c.danger);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(format!("{}  {}", icons::TRASH, app.t("dw.delete")))
                    .clicked()
                {
                    app.modals.push(Modal::ConfirmDelete(vec![id]));
                }
                if ui
                    .button(format!("{}  {}", icons::COPY, app.t("dw.clone")))
                    .clicked()
                {
                    app.act(Action::CloneDweller { id });
                }
                if vault::is_dead(&d)
                    && ui
                        .button(format!("{}  {}", icons::SPARKLES, app.t("dw.revive")))
                        .clicked()
                {
                    app.act(Action::Revive { ids: vec![id] });
                }
            });
        });
    });
    ui.add_space(8.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.identity")).font(style::bold(16.0)));
        let o = Opts::text(app.t("dw.firstName"));
        widgets::text_field(app, ui, &at(&["name"]), o);
        let o = Opts::text(app.t("dw.lastName"));
        widgets::text_field(app, ui, &at(&["lastName"]), o);
        let o = Opts::text(app.t("dw.gender")).range(1.0, 2.0);
        let g = vec![(1, app.t("dw.gender.1")), (2, app.t("dw.gender.2"))];
        widgets::int_choice_field(app, ui, &at(&["gender"]), o, &g);
        let o = Opts::text(app.t("dw.rarity"));
        let r: Vec<(String, String)> = RARITIES
            .iter()
            .map(|r| {
                (
                    r.to_string(),
                    catalog().names.rarity.get(*r).map_or_else(
                        || r.to_string(),
                        |n| n.get(app.lang == Lang::Ru).to_string(),
                    ),
                )
            })
            .collect();
        widgets::choice_field(app, ui, &at(&["rarity"]), o, &r);
        if d.get("uniqueData").is_some() {
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
                let mut l = level;
                let max = if app.expert() {
                    999
                } else {
                    lim.dweller_level_max
                };
                let resp = ui.add(egui::DragValue::new(&mut l).range(1..=max));
                if (resp.drag_stopped() || resp.lost_focus()) && l != level
                    || (resp.changed() && !resp.dragged() && !resp.has_focus() && l != level)
                {
                    app.act(Action::SetLevel {
                        ids: vec![id],
                        level: l,
                    });
                }
            },
        );
        for (key, label) in [
            ("experienceValue", "dw.xp"),
            ("wastelandExperience", "k.wastelandExperience"),
        ] {
            let path = at(&["experience", key]);
            if root.at(&path).is_some() {
                let o = Opts::text(app.t(label)).unit(app.t("c.units.xp"));
                widgets::num_field(app, ui, &path, o);
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
            let path = at(&["health", key]);
            let o = Opts::key(app, key);
            match root.at(&path) {
                Some(Value::Bool(_)) => widgets::bool_field(app, ui, &path, o),
                Some(Value::Number(_)) => widgets::num_field(app, ui, &path, o),
                _ => {}
            }
        }
        for key in ["deathTime", "deathSource"] {
            if d.get(key).is_some() {
                let o = Opts::key(app, key);
                widgets::num_field(app, ui, &at(&[key]), o);
            }
        }
    });
    ui.add_space(8.0);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.special")).font(style::bold(16.0)));
        let names = &catalog().names.special;
        let full = [
            "Strength",
            "Perception",
            "Endurance",
            "Charisma",
            "Intelligence",
            "Agility",
            "Luck",
        ];
        for s in 1..=7usize {
            let mut path = at(&["stats", "stats"]);
            path.push(Seg::Index(s));
            path.push(Seg::Key("value".into()));
            let label = names.get(full[s - 1]).map_or_else(
                || full[s - 1].to_string(),
                |n| n.get(app.lang == Lang::Ru).to_string(),
            );
            let o = Opts::text(format!("{} — {label}", SPECIAL_LETTERS[s - 1]))
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
        ui.label(RichText::new(app.t("dw.flags")).font(style::bold(16.0)));
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
            if d.get(key).is_some() {
                let o = Opts::key(app, key);
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
        let name_of = |r: &Value| {
            format!(
                "{} #{}",
                vault::text(r, &["type"]).unwrap_or_default(),
                vault::int(r, &["deserializeID"]).unwrap_or(-1)
            )
        };
        let o = Opts::text(app.t("dw.room"));
        widgets::row(app, ui, &at(&["savedRoom"]), &o, |app, ui| {
            let mut pick = None;
            egui::ComboBox::from_id_salt(("room", id))
                .selected_text(current.map_or_else(|| app.t("dw.noRoom"), name_of))
                .width(240.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(cur_id == -1, app.t("dw.noRoom"))
                        .clicked()
                    {
                        pick = Some(-1);
                    }
                    for r in rooms {
                        let rid = vault::int(r, &["deserializeID"]).unwrap_or(-1);
                        if vault::text(r, &["type"]).as_deref() == Some("Elevator") {
                            continue;
                        }
                        if ui.selectable_label(rid == cur_id, name_of(r)).clicked() {
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

fn looks(app: &mut App, ui: &mut egui::Ui, d: &Value, id: i64, base: &[Seg]) {
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.looks")).font(style::bold(16.0)));
        for (field, label) in [
            ("skinColor", "dw.skin"),
            ("hairColor", "dw.hair"),
            ("outfitColor", "dw.outfitColor"),
        ] {
            let Some(v) = vault::int(d, &[field]) else {
                continue;
            };
            let path = join(base, &[field]);
            let o = Opts::text(app.t(label));
            widgets::row(app, ui, &path, &o, |app, ui| {
                let mut rgb = rgb_of(v);
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    let packed =
                        (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2]);
                    app.act(Action::SetColor {
                        ids: vec![id],
                        field: field.into(),
                        rgb: packed,
                    });
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
                    if let Ok(packed) = u32::from_str_radix(text.trim().trim_start_matches('#'), 16)
                    {
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
        }
        if d.get("hair").is_some() {
            let o = Opts::text(app.t("dw.hairStyle"));
            let choices: Vec<(String, String)> = (1..=22)
                .map(|n| (format!("{n:02}"), format!("{n:02}")))
                .collect();
            widgets::choice_field(app, ui, &join(base, &["hair"]), o, &choices);
        }
        if d.get("faceMask").is_some() {
            let o = Opts::key(app, "faceMask");
            widgets::text_field(app, ui, &join(base, &["faceMask"]), o);
        }
    });
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
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&shown).font(style::bold(14.0)));
                        if let Some(i) = &cur {
                            if ui
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
                        let picked = crate::sec_vault::item_picker(app, ui, &key, &[kind], &mut q);
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
            pet_details(app, ui, extra, id);
        }
    });
}

fn pet_details(app: &mut App, ui: &mut egui::Ui, extra: &Value, id: i64) {
    let name = vault::text(extra, &["uniqueName"]).unwrap_or_default();
    let bonus = vault::text(extra, &["bonus"]).unwrap_or_default();
    let value = vault::int(extra, &["bonusValue"]).unwrap_or(0);
    let key = format!("pet:{id}");
    let mut buf = app.bufs.get(&key).cloned().unwrap_or_else(|| name.clone());
    ui.add_space(4.0);
    egui::Grid::new(("pet", id))
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            ui.label(app.t("dw.petName"));
            let r = ui.add(egui::TextEdit::singleline(&mut buf).desired_width(220.0));
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
                |n| n.get(app.lang == Lang::Ru).to_string(),
            );
            egui::ComboBox::from_id_salt(("bonus", id))
                .selected_text(shown)
                .width(260.0)
                .show_ui(ui, |ui| {
                    for (k, n) in &catalog().pet_bonuses {
                        if ui
                            .selectable_label(*k == bonus, n.get(app.lang == Lang::Ru))
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
            let mut v = value;
            let resp = ui.add(egui::DragValue::new(&mut v));
            if v != value && !resp.dragged() {
                app.act(Action::SetPetDetails {
                    id,
                    name: name.clone(),
                    bonus: bonus.clone(),
                    bonus_value: v,
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
                    .width(240.0)
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
        let o = Opts::key(app, "lastChildBorn");
        if d.get("lastChildBorn").is_some() {
            widgets::num_field(app, ui, &join(base, &["lastChildBorn"]), o);
        }
        egui::CollapsingHeader::new(Opts::key(app, "ascendants").label)
            .id_salt(("asc", id))
            .show(ui, |ui| {
                widgets::object_form(app, ui, &join(base, &["relations", "ascendants"]), 0);
            });
    });
}

fn pets(app: &mut App, ui: &mut egui::Ui, root: &Value) {
    let actors = vault::arr(root, &vault::ACTORS);
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("dw.pets")).font(style::bold(16.0)));
        ui.label(
            RichText::new(app.t("dw.petsHelp"))
                .small()
                .color(style::colors(ui.ctx()).muted),
        );
        if actors.is_empty() {
            ui.label(app.t("c.empty"));
        }
        for (i, a) in actors.iter().enumerate() {
            let name = vault::text(a, &["name"]).unwrap_or_default();
            let pet = vault::text(a, &["actorDataId"]).unwrap_or_default();
            let owner = vault::int(a, &["FollowedID"]).unwrap_or(-1);
            let owner_name = vault::dweller_index(root, owner)
                .map(|j| vault::dweller_name(&vault::arr(root, &vault::DWELLERS)[j]))
                .unwrap_or_else(|| "—".into());
            let mut parts = vec![name];
            if !pet.is_empty() {
                parts.push(pet);
            }
            if owner != -1 {
                parts.push(format!("{} {owner_name}", app.t("dw.owner")));
            }
            let title = format!("{}  {}", icons::PAW, parts.join(" · "));
            egui::CollapsingHeader::new(title)
                .id_salt(("actor", i))
                .show(ui, |ui| {
                    widgets::object_form(app, ui, &pi(&vault::ACTORS, i, &[]), 1);
                });
        }
    });
}
