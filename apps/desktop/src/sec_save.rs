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

use eframe::egui::{self, RichText};
use shelter_core::actions::Action;
use shelter_core::document::SourceKind;
use shelter_core::integrity::{Issue, Severity};
use shelter_core::json::{path_to_string, Seg};
use shelter_core::vault::{self, p};

use crate::app::{callout, panel, section_title, App, Modal, OtherTab, Section};
use crate::icons;
use crate::style;
use crate::widgets::{self, Opts};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            section_title(ui, icons::SHIELD, &app.t("nav.save"), "");
            panel(ui, |ui| {
                ui.label(RichText::new(app.t("save.general")).font(style::bold(16.0)));
                let o = Opts::key(app, "VaultName");
                widgets::text_field(app, ui, &p(&["vault", "VaultName"]), o);
                let o = Opts::key(app, "VaultMode");
                let choices = vec![
                    ("Normal".to_string(), app.t("vm.Normal")),
                    ("Survival".to_string(), app.t("vm.Survival")),
                ];
                widgets::choice_field(app, ui, &p(&["vault", "VaultMode"]), o, &choices);
                for key in ["appVersion", "deviceName", "seasonId"] {
                    if app
                        .open
                        .as_ref()
                        .is_some_and(|o| o.doc.root().get(key).is_some())
                    {
                        let o = Opts::key(app, key);
                        widgets::text_field(app, ui, &p(&[key]), o);
                    }
                }
                let o = Opts::key(app, "versionCount");
                widgets::num_field(app, ui, &p(&["versionCount"]), o);
            });
            ui.add_space(10.0);
            panel(ui, |ui| {
                ui.label(RichText::new(app.t("save.time")).font(style::bold(16.0)));
                ui.label(
                    RichText::new(app.t("time.local"))
                        .small()
                        .color(style::colors(ui.ctx()).muted),
                );
                let o = Opts::text(app.t("time.begin"));
                widgets::ticks_field(app, ui, &p(&["timeMgr", "timeGameBegin"]), o);
                let o = Opts::text(app.t("time.saved"));
                widgets::ticks_field(app, ui, &p(&["timeMgr", "timeSaveDate"]), o);
            });
            ui.add_space(10.0);
            summary(app, ui);
            ui.add_space(10.0);
            integrity(app, ui);
            ui.add_space(10.0);
            panel(ui, |ui| {
                ui.label(RichText::new(app.t("save.json")).font(style::bold(16.0)));
                let muted = style::colors(ui.ctx()).muted;
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(format!("{}  {}", icons::DOWNLOAD, app.t("save.export")))
                        .on_hover_text(app.t("save.exportHelp"))
                        .clicked()
                    {
                        app.export_json();
                    }
                    if ui
                        .button(format!("{}  {}", icons::UPLOAD, app.t("save.import")))
                        .on_hover_text(app.t("save.importHelp"))
                        .clicked()
                    {
                        app.import_json();
                    }
                });
                ui.label(RichText::new(app.t("save.exportHelp")).small().color(muted));
            });
            ui.add_space(10.0);
            panel(ui, |ui| {
                ui.label(RichText::new(app.t("save.backups")).font(style::bold(16.0)));
                let muted = style::colors(ui.ctx()).muted;
                ui.label(RichText::new(app.t("set.backupsHelp")).small().color(muted));
                let has_path = app.open.as_ref().is_some_and(|o| o.path.is_some());
                if ui
                    .add_enabled(
                        has_path,
                        egui::Button::new(format!("{}  {}", icons::HISTORY, app.t("save.restore"))),
                    )
                    .clicked()
                {
                    if let (Some(root), Some(path)) = (
                        app.settings.backup_dir(),
                        app.open.as_ref().and_then(|o| o.path.clone()),
                    ) {
                        app.restore_list = crate::files::list_backups(&root, &path);
                    }
                    app.modals.push(Modal::Restore);
                }
            });
        });
}

fn summary(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let root = open.doc.root();
    let dwellers = vault::arr(root, &vault::DWELLERS);
    let alive = dwellers.iter().filter(|d| !vault::is_dead(d)).count();
    let items = vault::arr(root, &vault::ITEMS).len();
    let rooms = vault::arr(root, &vault::ROOMS).len();
    let pets = vault::arr(root, &vault::ACTORS).len();
    let size = root.to_compact().len();
    let version = vault::text(root, &["appVersion"]).unwrap_or_default();
    let keys = root.as_object().map_or(0, |o| o.len());
    let fmt = if open.doc.source == SourceKind::Sav {
        ".sav"
    } else {
        "JSON"
    };
    let res: Vec<(String, String)> = [
        "Nuka",
        "Food",
        "Energy",
        "Water",
        "StimPack",
        "RadAway",
        "NukaColaQuantum",
    ]
    .iter()
    .filter_map(|k| {
        let v = root
            .at(&p(&["vault", "storage", "resources", k]))?
            .as_f64()?;
        Some((app.t(&format!("res.{k}")), widgets::fmt_num(v.floor())))
    })
    .collect();
    let rows = [
        (
            app.t("sum.dwellers"),
            format!(
                "{} ({})",
                dwellers.len(),
                app.tf("sum.alive", &[("n", alive.to_string())])
            ),
        ),
        (app.t("sum.rooms"), rooms.to_string()),
        (app.t("sum.items"), items.to_string()),
        (app.t("sum.pets"), pets.to_string()),
        (app.t("sum.version"), version.trim().to_string()),
        (app.t("sum.format"), fmt.to_string()),
        (app.t("sum.size"), format!("{} KB", size / 1024)),
        (app.t("sum.rootKeys"), keys.to_string()),
    ];
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("save.summary")).font(style::bold(16.0)));
        egui::Grid::new("summary")
            .num_columns(2)
            .spacing([24.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                for (k, v) in rows.iter().chain(res.iter()) {
                    ui.label(RichText::new(k).color(style::colors(ui.ctx()).muted));
                    ui.label(RichText::new(v).font(style::mono(13.0)));
                    ui.end_row();
                }
            });
    });
}

pub fn issue_text(app: &App, i: &Issue) -> String {
    let params: Vec<(&str, String)> = i
        .params
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    app.tf(&format!("integ.{}", i.code), &params)
}

/// Opens the section that shows a path.
pub fn go_to(app: &mut App, path: &[Seg]) {
    let key = |i: usize| match path.get(i) {
        Some(Seg::Key(k)) => k.as_str(),
        _ => "",
    };
    match (key(0), key(1)) {
        ("dwellers", "dwellers") => {
            app.section = Section::Dwellers;
            if let Some(Seg::Index(i)) = path.get(2) {
                if let Some(open) = &app.open {
                    if let Some(id) = vault::arr(open.doc.root(), &vault::DWELLERS)
                        .get(*i)
                        .and_then(|d| vault::int(d, &["serializeId"]))
                    {
                        app.dw.focus = Some(id);
                        app.dw.selected = [id].into();
                    }
                }
            }
        }
        ("dwellers", _) => app.section = Section::Dwellers,
        ("vault", "rooms") => {
            app.section = Section::Vault;
            app.vault_tab = crate::app::VaultTab::Rooms;
            if let (Some(Seg::Index(i)), Some(open)) = (path.get(2), &app.open) {
                app.room_sel = vault::arr(open.doc.root(), &vault::ROOMS)
                    .get(*i)
                    .and_then(|r| vault::int(r, &["deserializeID"]));
            }
        }
        ("vault", _) => app.section = Section::Vault,
        _ => {
            app.section = Section::Other;
            if app.expert() {
                app.other_tab = OtherTab::Tree;
                app.tree_search = path_to_string(path);
            } else {
                app.other_tab = OtherTab::Service;
            }
        }
    }
}

fn integrity(app: &mut App, ui: &mut egui::Ui) {
    let Some(open) = &app.open else { return };
    let (new, old) = open.doc.issues();
    let c = style::colors(ui.ctx());
    let mut goto = None;
    let mut fix = false;
    panel(ui, |ui| {
        ui.label(RichText::new(app.t("save.integrity")).font(style::bold(16.0)));
        if new.is_empty() && old.is_empty() {
            ui.label(
                RichText::new(format!("{}  {}", icons::CHECK, app.t("integ.ok"))).color(c.success),
            );
            return;
        }
        if new.iter().chain(old.iter()).any(|i| i.fixable)
            && ui
                .button(format!("{}  {}", icons::WRENCH, app.t("integ.fixAll")))
                .clicked()
        {
            fix = true;
        }
        for (title, list) in [(app.t("integ.new"), &new), (app.t("integ.old"), &old)] {
            if list.is_empty() {
                continue;
            }
            ui.add_space(6.0);
            ui.label(RichText::new(format!("{title}: {}", list.len())).font(style::bold(14.0)));
            for i in list.iter().take(200) {
                ui.horizontal_wrapped(|ui| {
                    let (icon, col) = if i.severity == Severity::Error {
                        (icons::ALERT, c.danger)
                    } else {
                        (icons::WARN, c.warning)
                    };
                    ui.label(RichText::new(icon).color(col));
                    ui.label(issue_text(app, i));
                    if !i.fixable {
                        ui.label(RichText::new(app.t("integ.manual")).small().color(c.muted));
                    }
                    if ui.small_button(app.t("integ.goTo")).clicked() {
                        goto = Some(i.path.clone());
                    }
                });
            }
        }
    });
    if fix {
        app.act(Action::FixIssues);
    }
    if let Some(p) = goto {
        go_to(app, &p);
    }
    let _ = callout;
}
