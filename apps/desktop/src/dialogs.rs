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

use eframe::egui::{self, RichText};
use shelter_core::actions::Action;
use shelter_core::catalog::catalog;
use shelter_core::diff::ChangeKind;
use shelter_core::i18n::Lang;
use shelter_core::json::path_to_string;
use shelter_core::rules::Mode;

use crate::app::{callout, AfterClose, App, Modal};
use crate::files;
use crate::icons;
use crate::settings::{LangPref, Settings, ThemePref, SCALES};
use crate::style;

pub const PRIVACY_EN: &str = include_str!("../../../legal/privacy.en.md");
pub const PRIVACY_RU: &str = include_str!("../../../legal/privacy.ru.md");
pub const THIRD_PARTY: &str = include_str!("../../../THIRD_PARTY.md");
pub const LICENSE_TEXT: &str = include_str!("../../../LICENSE");
pub const REPO: &str = "https://github.com/Mahiron-hq/ShelterSave-Studio";

enum Outcome {
    Keep,
    Close,
    Replace(Modal),
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(modal) = app.modals.pop() else {
        return;
    };
    let id = egui::Id::unique(("modal", app.modals.len()));
    let mut outcome = Outcome::Keep;
    let mut modal = modal;
    let resp = egui::Modal::new(id).show(ctx, |ui| {
        ui.set_max_width(720.0);
        outcome = match &mut modal {
            Modal::FirstRun => first_run(app, ui),
            Modal::Review => review(app, ui),
            Modal::Settings => settings(app, ui),
            Modal::About => about(app, ui),
            Modal::Help => help(app, ui),
            Modal::Journal => journal(app, ui),
            Modal::ExpertConfirm => expert_confirm(app, ui),
            Modal::CloseConfirm(after) => close_confirm(app, ui, after.clone()),
            Modal::ConfirmMass(action, n) => confirm_mass(app, ui, action.clone(), *n),
            Modal::ConfirmDelete(ids) => confirm_delete(app, ui, ids.clone()),
            Modal::NewDweller {
                first,
                last,
                gender,
            } => new_dweller(app, ui, first, last, gender),
            Modal::FromJournal {
                pets,
                query,
                pick,
                bonus,
            } => from_journal(app, ui, pets, query, pick, bonus),
            Modal::Restore => restore(app, ui),
            Modal::Recover(rec) => recover(app, ui, rec.clone()),
            Modal::Import {
                file,
                dwellers,
                picked,
            } => import(app, ui, file, dwellers, picked),
        };
    });
    let escape_closes = !matches!(modal, Modal::FirstRun);
    if resp.should_close() && escape_closes && matches!(outcome, Outcome::Keep) {
        outcome = Outcome::Close;
    }
    match outcome {
        Outcome::Keep => app.modals.push(modal),
        Outcome::Close => {}
        Outcome::Replace(m) => app.modals.push(m),
    }
}

fn title(ui: &mut egui::Ui, icon: &str, text: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(icon)
                .size(18.0)
                .color(style::colors(ui.ctx()).accent),
        );
        ui.label(RichText::new(text).font(style::bold(18.0)));
    });
    ui.add_space(6.0);
}

fn primary(ui: &mut egui::Ui, text: &str) -> bool {
    let c = style::colors(ui.ctx());
    ui.add(egui::Button::new(RichText::new(text).color(c.on_accent)).fill(c.accent))
        .clicked()
}

fn danger(ui: &mut egui::Ui, text: &str) -> bool {
    let c = style::colors(ui.ctx());
    ui.add(
        egui::Button::new(RichText::new(text).color(c.danger))
            .stroke(egui::Stroke::new(1.0, c.danger)),
    )
    .clicked()
}

pub const HELP_EN: &str = include_str!("../help/help.en.md");
pub const HELP_RU: &str = include_str!("../help/help.ru.md");

fn help(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    title(ui, icons::HELP, &app.t("tb.help"));
    let text = if app.lang == Lang::Ru {
        HELP_RU
    } else {
        HELP_EN
    };
    // The document's own first heading repeats the dialog title.
    let text = text.split_once('\n').map_or(text, |(_, rest)| rest);
    let screen = ui.ctx().content_rect();
    let max_h = (screen.height() * 0.7).max(240.0);
    let w = (screen.width() - 120.0).clamp(280.0, 700.0);
    egui::ScrollArea::vertical()
        .max_height(max_h)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.set_width(w);
            crate::widgets::markdown(ui, text);
        });
    ui.add_space(8.0);
    if primary(ui, &app.t("c.close")) {
        Outcome::Close
    } else {
        Outcome::Keep
    }
}

fn privacy(app: &App) -> &'static str {
    if app.lang == Lang::Ru {
        PRIVACY_RU
    } else {
        PRIVACY_EN
    }
}

fn first_run(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    title(ui, icons::SHIELD, &app.t("first.title"));
    ui.label(app.t("first.text"));
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .max_height(360.0)
        .show(ui, |ui| {
            egui::CollapsingHeader::new(app.t("about.privacy"))
                .default_open(true)
                .show(ui, |ui| {
                    crate::widgets::markdown(ui, privacy(app));
                });
            egui::CollapsingHeader::new(app.t("about.license")).show(ui, |ui| {
                ui.label(RichText::new(LICENSE_TEXT).font(style::mono(11.0)));
            });
        });
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if primary(ui, &app.t("first.accept")) {
            app.settings.accepted_terms = true;
            app.save_settings();
            out = Outcome::Close;
        }
        if ui.button(app.t("first.decline")).clicked() {
            app.allow_close = true;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
    out
}

fn review(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    let Some(report) = app.save_report() else {
        return Outcome::Close;
    };
    let changed_on_disk = app.changed_on_disk();
    let c = style::colors(ui.ctx());
    title(ui, icons::SAVE, &app.t("review.title"));
    ui.label(
        RichText::new(format!("{}  {}", icons::WARN, app.t("review.closeGame"))).color(c.muted),
    );
    if app.game_running {
        callout(
            ui,
            c.danger_soft,
            c.danger,
            icons::ALERT,
            &app.t("tb.gameRunning"),
            &app.t("tb.gameRunningHelp"),
        );
    }
    if changed_on_disk {
        callout(
            ui,
            c.warning_soft,
            c.warning,
            icons::WARN,
            &app.t("review.title"),
            &app.t("review.changedOnDisk"),
        );
    }
    ui.add_space(6.0);
    ui.label(
        RichText::new(app.tf("review.changes", &[("n", report.changes.len().to_string())]))
            .font(style::bold(14.0)),
    );
    if report.changes.is_empty() {
        ui.label(RichText::new(app.t("review.noChanges")).color(c.muted));
    } else {
        let root = app
            .open
            .as_ref()
            .map(|o| o.doc.shared_root())
            .unwrap_or_else(|| std::sync::Arc::new(shelter_core::json::Value::Null));
        let original = app
            .open
            .as_ref()
            .map(|o| o.doc.original().clone())
            .unwrap_or(shelter_core::json::Value::Null);
        let expert = app.expert();
        egui::ScrollArea::vertical()
            .id_salt("changes")
            .max_height(260.0)
            .show(ui, |ui| {
                for ch in report.changes.iter().take(300) {
                    let (icon, col) = match ch.kind {
                        ChangeKind::Changed => (icons::PENCIL, c.mark),
                        ChangeKind::Added => (icons::PLUS, c.success),
                        ChangeKind::Removed => (icons::TRASH, c.danger),
                    };
                    let mut job = egui::text::LayoutJob::default();
                    let fmt = |color: egui::Color32| egui::TextFormat {
                        font_id: egui::FontId::proportional(13.0),
                        color,
                        ..Default::default()
                    };
                    job.append(&format!("{icon}  "), 0.0, fmt(col));
                    // A removed entry is gone from the current file, so name it from the original.
                    let base = if matches!(ch.kind, ChangeKind::Removed) {
                        &original
                    } else {
                        &root
                    };
                    job.append(
                        &crate::sec_save::path_title(app, base, &ch.path),
                        0.0,
                        fmt(c.text),
                    );
                    let num = |s: &String| match s.parse::<f64>() {
                        Ok(v)
                            if s.bytes()
                                .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-') =>
                        {
                            crate::widgets::fmt_num(v)
                        }
                        _ => s.clone(),
                    };
                    // Whole objects and long lists are named by the path alone.
                    let whole = |s: &&String| !(s.ends_with(" keys}") || s.ends_with(" items]"));
                    let before = ch.before.as_ref().filter(whole).map(num);
                    let after = ch.after.as_ref().filter(whole).map(num);
                    match (&before, &after) {
                        (Some(b), Some(a)) => {
                            job.append(&format!(":  {b}"), 0.0, fmt(c.muted));
                            job.append(&format!("  →  {a}"), 0.0, fmt(c.text));
                        }
                        (None, Some(a)) => job.append(&format!(":  {a}"), 0.0, fmt(c.text)),
                        (Some(b), None) => job.append(&format!(":  {b}"), 0.0, fmt(c.muted)),
                        (None, None) => {}
                    }
                    ui.add(egui::Label::new(job).wrap());
                    if expert {
                        ui.add(
                            egui::Label::new(
                                RichText::new(path_to_string(&ch.path))
                                    .font(style::mono(11.0))
                                    .color(c.muted),
                            )
                            .wrap(),
                        );
                    }
                }
                if report.changes.len() > 300 {
                    ui.label(
                        RichText::new(app.tf(
                            "review.more",
                            &[("n", (report.changes.len() - 300).to_string())],
                        ))
                        .color(c.muted),
                    );
                }
            });
    }
    let errors = report
        .new_issues
        .iter()
        .filter(|i| i.severity == shelter_core::integrity::Severity::Error)
        .count();
    if !report.new_issues.is_empty() || !report.old_issues.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(app.t("review.problems")).font(style::bold(14.0)));
        egui::ScrollArea::vertical()
            .id_salt("issues")
            .max_height(140.0)
            .show(ui, |ui| {
                for (list, label) in [
                    (&report.new_issues, app.t("integ.new")),
                    (&report.old_issues, app.t("integ.old")),
                ] {
                    for i in list.iter() {
                        ui.add(
                            egui::Label::new(format!(
                                "{}  {} – {}",
                                icons::ALERT,
                                crate::sec_save::issue_text(app, i),
                                label
                            ))
                            .wrap(),
                        );
                    }
                }
            });
        if ui
            .button(format!("{}  {}", icons::WRENCH, app.t("integ.fixAll")))
            .clicked()
        {
            app.act(Action::FixIssues);
        }
    }
    if !report.new_warnings.is_empty() || !report.old_warnings.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(app.t("review.warnings")).font(style::bold(14.0)));
        egui::ScrollArea::vertical()
            .id_salt("warns")
            .max_height(140.0)
            .show(ui, |ui| {
                for (list, label) in [
                    (&report.new_warnings, app.t("integ.new")),
                    (&report.old_warnings, app.t("integ.old")),
                ] {
                    for w in list.iter() {
                        let place = app
                            .open
                            .as_ref()
                            .map(|o| crate::sec_save::path_title(app, o.doc.root(), &w.path))
                            .unwrap_or_default();
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!(
                                    "{}  {} · {} = {} – {}",
                                    icons::WARN,
                                    app.t(&format!("warn.{}", w.code)),
                                    place,
                                    w.value,
                                    label
                                ))
                                .color(c.warning),
                            )
                            .wrap(),
                        );
                    }
                }
            });
    }
    ui.add_space(10.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        let label = if errors > 0 {
            app.t("review.saveAnyway")
        } else {
            app.t("review.confirm")
        };
        if primary(ui, &format!("{}  {label}", icons::SAVE)) {
            let target = app.open.as_ref().and_then(|o| o.path.clone());
            match target {
                Some(t) => app.write_to(&t),
                None => app.save_as(),
            }
            out = Outcome::Close;
        }
        if ui.button(app.t("tb.saveAs")).clicked() {
            app.save_as();
            out = Outcome::Close;
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

fn setting_row(ui: &mut egui::Ui, label: &str, help: &str, add: impl FnOnce(&mut egui::Ui)) {
    let c = style::colors(ui.ctx());
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(300.0);
            ui.label(RichText::new(label).font(style::bold(14.0)));
            if !help.is_empty() {
                ui.label(RichText::new(help).small().color(c.muted));
            }
        });
        add(ui);
    });
    ui.add_space(6.0);
}

fn settings(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    title(ui, icons::SETTINGS, &app.t("set.title"));
    let before = app.settings.clone();
    let mut out = Outcome::Keep;
    let mut want_expert = false;
    egui::ScrollArea::vertical()
        .max_height(520.0)
        .show(ui, |ui| {
            ui.label(RichText::new(app.t("set.appearance")).font(style::bold(16.0)));
            let (l, h) = (app.t("set.language"), app.t("set.languageHelp"));
            let opts = [
                (LangPref::System, app.t("set.lang.system")),
                (LangPref::Ru, app.t("set.lang.ru")),
                (LangPref::En, app.t("set.lang.en")),
            ];
            setting_row(ui, &l, &h, |ui| {
                for (v, t) in &opts {
                    ui.radio_value(&mut app.settings.language, *v, t);
                }
            });
            let (l, h) = (app.t("set.theme"), app.t("set.themeHelp"));
            let opts = [
                (ThemePref::System, app.t("set.theme.system")),
                (ThemePref::Light, app.t("set.theme.light")),
                (ThemePref::Dark, app.t("set.theme.dark")),
            ];
            setting_row(ui, &l, &h, |ui| {
                for (v, t) in &opts {
                    ui.radio_value(&mut app.settings.theme, *v, t);
                }
            });
            let (l, h) = (app.t("set.scale"), app.t("set.scaleHelp"));
            setting_row(ui, &l, &h, |ui| {
                egui::ComboBox::from_id_salt("scale")
                    .selected_text(format!("{:.0} %", app.settings.scale * 100.0))
                    .show_ui(ui, |ui| {
                        for s in SCALES {
                            ui.selectable_value(
                                &mut app.settings.scale,
                                s,
                                format!("{:.0} %", s * 100.0),
                            );
                        }
                    });
            });
            ui.separator();
            ui.label(RichText::new(app.t("set.editor")).font(style::bold(16.0)));
            let (l, h) = (app.t("set.itemNames"), app.t("set.itemNamesHelp"));
            let (en, ru) = (app.t("set.itemNames.en"), app.t("set.itemNames.ru"));
            setting_row(ui, &l, &h, |ui| {
                ui.radio_value(&mut app.settings.russian_item_names, false, en);
                ui.radio_value(&mut app.settings.russian_item_names, true, ru);
            });
            let (l, h) = (app.t("set.mode"), app.t("set.modeHelp"));
            let (n, e) = (app.t("set.mode.normal"), app.t("set.mode.expert"));
            setting_row(ui, &l, &h, |ui| {
                let mut m = app.settings.mode;
                ui.radio_value(&mut m, Mode::Normal, n);
                ui.radio_value(&mut m, Mode::Expert, e);
                if m == Mode::Expert && app.settings.mode == Mode::Normal {
                    want_expert = true;
                } else {
                    app.settings.mode = m;
                }
            });
            let (l, h) = (app.t("set.confirmMass"), app.t("set.confirmMassHelp"));
            let (on, off) = (app.t("set.on"), app.t("set.off"));
            setting_row(ui, &l, &h, |ui| {
                ui.radio_value(&mut app.settings.confirm_mass, true, on);
                ui.radio_value(&mut app.settings.confirm_mass, false, off);
            });
            let (l, h) = (app.t("set.dwellerScroll"), app.t("set.dwellerScrollHelp"));
            let (keep, top) = (
                app.t("set.dwellerScroll.shared"),
                app.t("set.dwellerScroll.top"),
            );
            setting_row(ui, &l, &h, |ui| {
                ui.radio_value(
                    &mut app.settings.dweller_scroll,
                    crate::settings::DwellerScroll::Shared,
                    keep,
                );
                ui.radio_value(
                    &mut app.settings.dweller_scroll,
                    crate::settings::DwellerScroll::Top,
                    top,
                );
            });
            ui.separator();
            ui.label(RichText::new(app.t("set.files")).font(style::bold(16.0)));
            let (l, h) = (app.t("set.backups"), app.t("set.backupsHelp"));
            let (on, off) = (app.t("set.on"), app.t("set.off"));
            setting_row(ui, &l, &h, |ui| {
                ui.radio_value(&mut app.settings.backups, true, on);
                ui.radio_value(&mut app.settings.backups, false, off);
            });
            let dir = app
                .settings
                .backup_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            let (l, choose, open_l) = (
                app.t("set.backupDir"),
                app.t("set.choose"),
                app.t("set.openFolder"),
            );
            setting_row(ui, &l, &dir, |ui| {
                if ui.button(choose).clicked() {
                    if let Some(d) = rfd::FileDialog::new().pick_folder() {
                        app.settings.backup_dir = Some(d);
                    }
                }
                if ui.button(open_l).clicked() {
                    if let Some(d) = app.settings.backup_dir() {
                        let _ = std::fs::create_dir_all(&d);
                        ui.ctx()
                            .open_url(egui::OpenUrl::new_tab(format!("file:///{}", d.display())));
                    }
                }
            });
            let found = app
                .art
                .game_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            let status = match app.art.status() {
                crate::art::Status::Loading => app.t("art.loading"),
                crate::art::Status::NoGame => app.t("set.gameDirNone"),
                crate::art::Status::Mismatch => app.t("set.gameDirMismatch"),
                crate::art::Status::Ready => app.tf("set.gameDirFound", &[("path", found)]),
            };
            let (l, h, choose, auto) = (
                app.t("set.gameDir"),
                app.t("set.gameDirHelp"),
                app.t("set.choose"),
                app.t("set.gameDirAuto"),
            );
            setting_row(ui, &l, &h, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&status).small());
                    ui.horizontal(|ui| {
                        if ui.button(choose).clicked() {
                            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                                app.settings.game_dir = Some(d);
                                app.art.reset();
                            }
                        }
                        if app.settings.game_dir.is_some() && ui.button(auto).clicked() {
                            app.settings.game_dir = None;
                            app.art.reset();
                        }
                    });
                });
            });
            let l = app.t("set.backupKeep");
            setting_row(ui, &l, "", |ui| {
                ui.add(egui::DragValue::new(&mut app.settings.backup_keep).range(1..=100));
            });
            let (l, h, add) = (
                app.t("set.saveDirs"),
                app.t("set.saveDirsHelp"),
                app.t("set.addDir"),
            );
            setting_row(ui, &l, &h, |ui| {
                ui.vertical(|ui| {
                    for d in files::standard_save_dirs() {
                        ui.label(RichText::new(d.display().to_string()).small());
                    }
                    let mut remove = None;
                    for (i, d) in app.settings.save_dirs.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(d.display().to_string()).small());
                            if crate::app::small_icon(ui, icons::X, &app.t("c.remove")).clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        app.settings.save_dirs.remove(i);
                    }
                    if ui.button(add).clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            app.settings.save_dirs.push(d);
                        }
                    }
                });
            });
            ui.separator();
            let (l, h) = (app.t("set.reset"), app.t("set.resetHelp"));
            let reset = app.t("set.reset");
            setting_row(ui, &l, &h, |ui| {
                if ui.button(reset).clicked() {
                    let accepted = app.settings.accepted_terms;
                    let recent = std::mem::take(&mut app.settings.recent);
                    app.settings = Settings {
                        accepted_terms: accepted,
                        recent,
                        ..Settings::default()
                    };
                }
            });
        });
    if app.settings != before {
        app.lang = app.settings.lang();
        app.apply_appearance(ui.ctx());
        if app.settings.save_dirs != before.save_dirs {
            app.found = files::discover(&app.settings.save_dirs);
        }
        app.save_settings();
    }
    ui.add_space(8.0);
    if primary(ui, &app.t("c.close")) {
        out = Outcome::Close;
    }
    if want_expert {
        app.modals.push(Modal::Settings);
        return Outcome::Replace(Modal::ExpertConfirm);
    }
    out
}

fn import(
    app: &mut App,
    ui: &mut egui::Ui,
    file: &str,
    dwellers: &[(String, String)],
    picked: &mut std::collections::BTreeSet<usize>,
) -> Outcome {
    let c = style::colors(ui.ctx());
    title(
        ui,
        icons::DOWNLOAD,
        &app.tf("dw.importTitle", &[("file", file.to_string())]),
    );
    ui.add(egui::Label::new(RichText::new(app.t("dw.importNote")).color(c.muted)).wrap());
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.small_button(app.t("c.selectAll")).clicked() {
            *picked = (0..dwellers.len()).collect();
        }
        if ui.small_button(app.t("c.selectNone")).clicked() {
            picked.clear();
        }
    });
    // Room for the heading, note and buttons on a short window.
    let list_h = (ui.ctx().content_rect().height() - 320.0).clamp(120.0, 400.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .min_scrolled_height(list_h)
        .max_height(list_h)
        .show(ui, |ui| {
            for (i, (label, _)) in dwellers.iter().enumerate() {
                let mut on = picked.contains(&i);
                if ui.checkbox(&mut on, label).changed() {
                    if on {
                        picked.insert(i);
                    } else {
                        picked.remove(&i);
                    }
                }
            }
        });
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        let text = app.tf("dw.importGo", &[("n", picked.len().to_string())]);
        if ui
            .add_enabled(!picked.is_empty(), egui::Button::new(text))
            .clicked()
        {
            let mut done = 0;
            for i in picked.iter() {
                if app.act(shelter_core::actions::Action::ImportDweller {
                    json: dwellers[*i].1.clone(),
                }) {
                    done += 1;
                }
            }
            app.toast(app.tf("dw.importDone", &[("n", done.to_string())]), false);
            out = Outcome::Close;
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

fn recover(app: &mut App, ui: &mut egui::Ui, rec: std::rc::Rc<crate::files::Recovery>) -> Outcome {
    let c = style::colors(ui.ctx());
    title(ui, icons::HISTORY, &app.t("recover.title"));
    // Only the file name goes into the heading: a long path would not wrap there.
    let file = rec.path.as_ref().and_then(|p| p.file_name()).map_or_else(
        || rec.session.file_name.clone(),
        |n| n.to_string_lossy().to_string(),
    );
    let steps = rec.session.history.done().len();
    callout(
        ui,
        c.warning_soft,
        c.warning,
        icons::WARN,
        &file,
        &app.tf("recover.text", &[("n", steps.to_string())]),
    );
    if let Some(path) = &rec.path {
        ui.add(
            egui::Label::new(
                RichText::new(path.display().to_string())
                    .small()
                    .color(c.muted),
            )
            .wrap(),
        );
    }
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if primary(ui, &app.t("recover.restore")) {
            app.recover(&rec);
            out = Outcome::Close;
        }
        if ui.button(app.t("recover.discard")).clicked() {
            crate::files::clear_recovery();
            out = Outcome::Close;
        }
    });
    out
}

fn expert_confirm(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    let c = style::colors(ui.ctx());
    title(ui, icons::WARN, &app.t("set.expertTitle"));
    callout(
        ui,
        c.warning_soft,
        c.warning,
        icons::WARN,
        &app.t("set.mode.expert"),
        &app.t("set.expertText"),
    );
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if primary(ui, &app.t("set.expertEnable")) {
            app.settings.mode = Mode::Expert;
            app.save_settings();
            out = Outcome::Close;
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

fn about(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    let c = style::colors(ui.ctx());
    title(ui, icons::INFO, &app.t("about.title"));
    ui.label(RichText::new(app.t("app.name")).font(style::bold(20.0)));
    ui.label(app.tf(
        "about.version",
        &[("v", env!("CARGO_PKG_VERSION").to_string())],
    ));
    ui.label(app.t("app.copyright"));
    ui.label(app.t("about.license"));
    ui.label(RichText::new(app.t("about.offline")).color(c.muted));
    ui.label(app.tf(
        "about.catalog",
        &[
            ("c", catalog().catalog_version.clone()),
            ("g", catalog().game_version.clone()),
        ],
    ));
    ui.add_space(6.0);
    ui.label(RichText::new(app.t("about.disclaimer")).color(c.muted));
    ui.label(RichText::new(app.t("app.unaffiliated")).color(c.muted));
    ui.hyperlink_to(
        format!("{}  {}", icons::EXTERNAL, app.t("about.repo")),
        REPO,
    );
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            egui::CollapsingHeader::new(app.t("about.privacy")).show(ui, |ui| {
                crate::widgets::markdown(ui, privacy(app));
            });
            egui::CollapsingHeader::new(app.t("about.thirdParty")).show(ui, |ui| {
                ui.label(RichText::new(THIRD_PARTY).font(style::mono(11.0)));
            });
            egui::CollapsingHeader::new(app.t("about.license")).show(ui, |ui| {
                ui.label(RichText::new(LICENSE_TEXT).font(style::mono(11.0)));
            });
        });
    ui.add_space(8.0);
    if primary(ui, &app.t("c.close")) {
        Outcome::Close
    } else {
        Outcome::Keep
    }
}

fn journal(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    let c = style::colors(ui.ctx());
    title(ui, icons::HISTORY, &app.t("journal.title"));
    let Some(open) = &app.open else {
        return Outcome::Close;
    };
    let done: Vec<String> = open
        .doc
        .history()
        .done()
        .iter()
        .map(|t| format!("{} ({})", app.label_text(&t.label), t.affected))
        .collect();
    let undone: Vec<String> = open
        .doc
        .history()
        .undone()
        .map(|t| app.label_text(&t.label))
        .collect();
    let mut goto = None;
    egui::ScrollArea::vertical()
        .max_height(400.0)
        .show(ui, |ui| {
            let cur = done.len();
            if ui
                .selectable_label(cur == 0, format!("0. {}", app.t("journal.opened")))
                .on_hover_text(app.t("journal.goTo"))
                .clicked()
            {
                goto = Some(0);
            }
            for (i, l) in done.iter().enumerate() {
                if ui
                    .selectable_label(cur == i + 1, format!("{}. {l}", i + 1))
                    .on_hover_text(app.t("journal.goTo"))
                    .clicked()
                {
                    goto = Some(i + 1);
                }
            }
            for (i, l) in undone.iter().enumerate() {
                let n = cur + i + 1;
                if ui
                    .selectable_label(
                        false,
                        RichText::new(format!("{n}. {l} – {}", app.t("journal.undone")))
                            .color(c.muted),
                    )
                    .clicked()
                {
                    goto = Some(n);
                }
            }
        });
    if let Some(n) = goto {
        if let Some(open) = app.open.as_mut() {
            if let Err(e) = open.doc.go_to(n) {
                let t = app.tf("aerr.edit", &[("detail", e.to_string())]);
                app.toast_error(t);
            }
        }
    }
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if ui
            .button(format!("{}  {}", icons::RESET, app.t("journal.reset")))
            .on_hover_text(app.t("journal.resetHelp"))
            .clicked()
        {
            if let Some(open) = app.open.as_mut() {
                open.doc.reset_to_original();
            }
        }
        if primary(ui, &app.t("c.close")) {
            out = Outcome::Close;
        }
    });
    out
}

fn close_confirm(app: &mut App, ui: &mut egui::Ui, after: AfterClose) -> Outcome {
    title(ui, icons::WARN, &app.t("close.title"));
    ui.label(app.t("close.text"));
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if primary(ui, &format!("{}  {}", icons::SAVE, app.t("tb.save"))) {
            out = Outcome::Replace(Modal::Review);
        }
        if danger(ui, &app.t("close.discard")) {
            out = Outcome::Close;
            match &after {
                AfterClose::Exit => {
                    app.allow_close = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                AfterClose::Open(p) => {
                    app.open = None;
                    app.open_dialog_or(p.clone());
                }
                AfterClose::CloseFile => {
                    app.open = None;
                    app.found = files::discover(&app.settings.save_dirs);
                }
            }
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

fn confirm_mass(app: &mut App, ui: &mut egui::Ui, action: Action, n: usize) -> Outcome {
    title(ui, icons::USERS, &app.t("c.confirmTitle"));
    ui.label(app.tf("c.confirmMass", &[("n", n.to_string())]));
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if primary(ui, &app.t("c.confirm")) {
            app.act(action);
            out = Outcome::Close;
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

fn confirm_delete(app: &mut App, ui: &mut egui::Ui, ids: Vec<i64>) -> Outcome {
    title(ui, icons::TRASH, &app.t("c.confirmTitle"));
    ui.label(app.tf("c.dangerDelete", &[("n", ids.len().to_string())]));
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if danger(ui, &app.t("dw.delete")) {
            app.act(Action::DeleteDwellers { ids: ids.clone() });
            for id in &ids {
                app.dw.selected.remove(id);
            }
            if app.dw.focus.is_some_and(|f| ids.contains(&f)) {
                app.dw.focus = None;
            }
            out = Outcome::Close;
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

fn new_dweller(
    app: &mut App,
    ui: &mut egui::Ui,
    first: &mut String,
    last: &mut String,
    gender: &mut i64,
) -> Outcome {
    title(ui, icons::USER_PLUS, &app.t("dw.newTitle"));
    egui::Grid::new("newdw")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label(app.t("dw.firstName"));
            ui.add(egui::TextEdit::singleline(first).desired_width(240.0));
            ui.end_row();
            ui.label(app.t("dw.lastName"));
            ui.add(egui::TextEdit::singleline(last).desired_width(240.0));
            ui.end_row();
            ui.label(app.t("dw.gender"));
            ui.horizontal(|ui| {
                ui.radio_value(gender, 1, app.t("dw.gender.1"));
                ui.radio_value(gender, 2, app.t("dw.gender.2"));
            });
            ui.end_row();
        });
    ui.add_space(8.0);
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        if primary(ui, &app.t("c.add"))
            && app.act(Action::AddDweller {
                first: first.trim().to_string(),
                last: last.trim().to_string(),
                gender: *gender,
            })
        {
            if let Some(o) = &app.open {
                app.dw.focus =
                    shelter_core::vault::arr(o.doc.root(), &shelter_core::vault::DWELLERS)
                        .last()
                        .and_then(|d| shelter_core::vault::int(d, &["serializeId"]));
            }
            out = Outcome::Close;
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

/// Adds a legendary dweller or pet the way the game gives one, with its journal entry.
fn from_journal(
    app: &mut App,
    ui: &mut egui::Ui,
    pets: &mut bool,
    query: &mut String,
    pick: &mut Option<String>,
    bonus: &mut f64,
) -> Outcome {
    use shelter_core::vault;
    let c = style::colors(ui.ctx());
    title(ui, icons::STAR, &app.t("dw.journalTitle"));
    ui.add(egui::Label::new(RichText::new(app.t("dw.journalHelp")).color(c.muted)).wrap());
    ui.add_space(6.0);
    let root = app
        .open
        .as_ref()
        .map(|o| o.doc.shared_root())
        .unwrap_or_else(|| std::sync::Arc::new(shelter_core::json::Value::Null));
    ui.horizontal(|ui| {
        if ui
            .selectable_label(!*pets, app.t("dw.journalDwellers"))
            .clicked()
            && *pets
        {
            *pets = false;
            *pick = None;
        }
        if ui
            .selectable_label(*pets, app.t("dw.journalPets"))
            .clicked()
            && !*pets
        {
            *pets = true;
            *pick = None;
        }
    });
    ui.add(
        egui::TextEdit::singleline(query)
            .hint_text(app.t("c.search"))
            .desired_width(320.0),
    );
    let q = query.to_lowercase();
    let ru = app.settings.russian_item_names;
    let in_vault: Vec<String> = vault::arr(&root, &vault::DWELLERS)
        .iter()
        .filter_map(|d| vault::text(d, &["uniqueData"]))
        .collect();
    let mut rows: Vec<(String, String, String)> = if *pets {
        catalog()
            .pets
            .iter()
            .filter(|p| p.rarity == "Legendary")
            .map(|p| {
                let bonus = catalog()
                    .pet_bonuses
                    .get(&p.bonus)
                    .map_or(p.bonus.clone(), |n| n.get(ru).to_string());
                let range = format!(
                    "{}–{}",
                    crate::widgets::fmt_num(p.bonus_min),
                    crate::widgets::fmt_num(p.bonus_max)
                );
                (
                    p.id.clone(),
                    p.name.get(ru).to_string(),
                    crate::sec_vault::bonus_text(&bonus, &range),
                )
            })
            .collect()
    } else {
        catalog()
            .unique_dwellers
            .iter()
            .filter(|u| u.in_journal())
            .map(|u| {
                let outfit = catalog()
                    .outfit(&u.outfit)
                    .map_or(u.outfit.clone(), |o| o.name.get(ru).to_string());
                let mut extra = outfit;
                if in_vault.contains(&u.id) {
                    extra = format!("{extra}  ·  {}", app.t("dw.journalInVault"));
                }
                (
                    u.id.clone(),
                    format!("{} {}", u.first_name, u.last_name)
                        .trim()
                        .to_string(),
                    extra,
                )
            })
            .collect()
    };
    rows.retain(|r| {
        q.is_empty() || r.1.to_lowercase().contains(&q) || r.2.to_lowercase().contains(&q)
    });
    rows.sort_by(|a, b| {
        a.1.to_lowercase()
            .cmp(&b.1.to_lowercase())
            .then(a.2.cmp(&b.2))
    });
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.set_min_width(420.0);
            if rows.is_empty() {
                ui.label(RichText::new(app.t("c.noResults")).color(c.muted));
            }
            for (id, name, extra) in &rows {
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    name,
                    0.0,
                    egui::TextFormat::simple(egui::FontId::proportional(14.0), c.text),
                );
                job.append(
                    &format!("  ·  {extra}"),
                    0.0,
                    egui::TextFormat::simple(egui::FontId::proportional(12.0), c.muted),
                );
                let resp =
                    ui.add(egui::Button::selectable(pick.as_deref() == Some(id), job).wrap());
                let resp = if *pets {
                    crate::sec_vault::item_hover(
                        app,
                        resp,
                        shelter_core::catalog::ItemKind::Pet,
                        id,
                    )
                } else {
                    resp
                };
                if resp.clicked() {
                    *pick = Some(id.clone());
                    if let Some(p) = catalog().pet(id) {
                        *bonus = p.bonus_max.round();
                    }
                }
            }
        });
    ui.add_space(6.0);
    let mut ready = false;
    if let Some(id) = pick.clone() {
        if *pets {
            if let Some(p) = catalog().pet(&id) {
                let (lo, hi) = (p.bonus_min.floor(), p.bonus_max.ceil());
                ui.horizontal_wrapped(|ui| {
                    ui.label(app.t("dw.journalBonus"));
                    if let Some(v) = crate::widgets::pick_number(
                        app,
                        ui,
                        "journal_bonus",
                        bonus.clamp(lo, hi),
                        (lo, hi),
                        Some((lo, hi)),
                        true,
                    ) {
                        *bonus = v;
                    }
                    crate::widgets::unit_label(
                        ui,
                        &app.tf(
                            "dw.journalBonusRange",
                            &[
                                ("min", crate::widgets::fmt_num(lo)),
                                ("max", crate::widgets::fmt_num(hi)),
                            ],
                        ),
                    );
                });
                ready = true;
            }
        } else if let Some(u) = catalog().unique_dweller(&id) {
            let stats: Vec<String> = shelter_core::vault::SPECIAL_LETTERS
                .iter()
                .zip(&u.special)
                .map(|(l, v)| format!("{l} {v}"))
                .collect();
            ui.label(RichText::new(stats.join("  ")).font(style::mono(13.0)));
            let weapon = catalog()
                .weapon(&u.weapon)
                .map_or_else(|| app.t("dw.noWeapon"), |w| w.name.get(ru).to_string());
            ui.label(
                RichText::new(app.tf("dw.journalWeapon", &[("name", weapon)]))
                    .small()
                    .color(c.muted),
            );
            ready = true;
        }
    }
    let mut out = Outcome::Keep;
    ui.horizontal(|ui| {
        let add = ui
            .add_enabled(ready, egui::Button::new(app.t("c.add")))
            .clicked();
        if add {
            if let Some(id) = pick.clone() {
                let action = if *pets {
                    Action::AddPet {
                        id,
                        bonus_value: *bonus,
                    }
                } else {
                    Action::AddUniqueDweller { id }
                };
                if app.act(action) {
                    if !*pets {
                        if let Some(o) = &app.open {
                            app.dw.focus = vault::arr(o.doc.root(), &vault::DWELLERS)
                                .last()
                                .and_then(|d| vault::int(d, &["serializeId"]));
                        }
                    }
                    out = Outcome::Close;
                }
            }
        }
        if ui.button(app.t("c.cancel")).clicked() {
            out = Outcome::Close;
        }
    });
    out
}

fn restore(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    title(ui, icons::HISTORY, &app.t("save.restore"));
    ui.label(app.t("save.restoreHelp"));
    let mut out = Outcome::Keep;
    let mut pick = None;
    if app.restore_list.is_empty() {
        ui.label(app.t("save.noBackups"));
    }
    egui::ScrollArea::vertical()
        .max_height(320.0)
        .show(ui, |ui| {
            for (i, b) in app.restore_list.iter().enumerate() {
                let name = b
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if ui
                    .selectable_label(
                        false,
                        format!(
                            "{}  {name}  ·  {}",
                            icons::ARCHIVE,
                            files::format_time(b.modified)
                        ),
                    )
                    .clicked()
                {
                    pick = Some(i);
                }
            }
        });
    if let Some(i) = pick {
        let src = app.restore_list[i].path.clone();
        if let Some(target) = app.open.as_ref().and_then(|o| o.path.clone()) {
            let result = std::fs::read(&src).and_then(|data| {
                if let Some(root) = app.settings.backup_dir() {
                    if target.exists() {
                        files::make_backup(&root, &target, app.settings.backup_keep)?;
                    }
                }
                files::write_atomic(&target, &data)
            });
            match result {
                Ok(()) => {
                    app.open = None;
                    app.open_path(&target);
                    out = Outcome::Close;
                }
                Err(e) => {
                    let t = app.tf("err.io", &[("detail", e.to_string())]);
                    app.toast_error(t);
                }
            }
        }
    }
    ui.add_space(8.0);
    if ui.button(app.t("c.close")).clicked() {
        out = Outcome::Close;
    }
    out
}
