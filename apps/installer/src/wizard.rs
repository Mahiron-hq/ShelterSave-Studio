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

use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, RichText};
use shelter_core::i18n::{self, Lang};

use crate::{icons, style, win};

static PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.bin"));
const LICENSE: &str = include_str!("../../../LICENSE");
const PRIVACY_EN: &str = include_str!("../../../legal/privacy.en.md");
const PRIVACY_RU: &str = include_str!("../../../legal/privacy.ru.md");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> eframe::Result {
    let uninstall = std::env::args().any(|a| a == "--uninstall");
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/installer-icon.png"))
            .unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ShelterSave-Studio")
            .with_inner_size([700.0, 540.0])
            .with_min_inner_size([560.0, 440.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "ShelterSave-Studio setup",
        options,
        Box::new(move |cc| {
            style::install_fonts(&cc.egui_ctx);
            style::apply(&cc.egui_ctx, false);
            Ok(Box::new(Wizard::new(uninstall)))
        }),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Welcome,
    License,
    Privacy,
    Folder,
    Options,
    Work,
    Done,
    Failed,
    Remove,
    Removed,
}

#[derive(Default)]
struct Progress {
    step: usize,
    result: Option<Result<(), String>>,
}

struct Wizard {
    lang: Lang,
    page: Page,
    found: Option<win::Installed>,
    dir: String,
    start_menu: bool,
    desktop: bool,
    assoc: bool,
    accepted: bool,
    remove_data: bool,
    launch: bool,
    blocked: bool,
    error: String,
    progress: Arc<Mutex<Progress>>,
}

impl Wizard {
    fn new(uninstall: bool) -> Wizard {
        let lang = Lang::from_locale(&sys_locale::get_locale().unwrap_or_default());
        let found = win::installed();
        let dir = found
            .as_ref()
            .map_or_else(win::default_dir, |f| f.dir.clone())
            .display()
            .to_string();
        Wizard {
            lang,
            page: if uninstall {
                Page::Remove
            } else {
                Page::Welcome
            },
            found,
            dir,
            start_menu: true,
            desktop: true,
            assoc: false,
            accepted: false,
            remove_data: false,
            launch: true,
            blocked: false,
            error: String::new(),
            progress: Arc::default(),
        }
    }

    fn t(&self, key: &str) -> String {
        i18n::tr(self.lang, key)
    }

    fn tf(&self, key: &str, params: &[(&str, String)]) -> String {
        i18n::trf(self.lang, key, params)
    }

    fn uninstalling(&self) -> bool {
        matches!(self.page, Page::Remove | Page::Removed)
    }

    fn heading(&self) -> String {
        match self.page {
            Page::Welcome => self.t("inst.welcome"),
            Page::License => self.t("inst.license"),
            Page::Privacy => self.t("inst.privacy"),
            Page::Folder => self.t("inst.folder"),
            Page::Options => self.t("inst.options"),
            Page::Work => self.t("inst.progress"),
            Page::Done => self.t("inst.done"),
            Page::Failed => self.t("inst.failed"),
            Page::Remove => self.t("inst.uTitle"),
            Page::Removed => self.t("inst.uTitle"),
        }
    }

    fn target(&self) -> PathBuf {
        PathBuf::from(self.dir.trim())
    }

    fn start_install(&mut self) {
        let dir = self.target();
        if win::running(&dir) {
            self.blocked = true;
            return;
        }
        self.blocked = false;
        self.page = Page::Work;
        let opt = win::Options {
            dir,
            start_menu: self.start_menu,
            desktop: self.desktop,
            assoc: self.assoc,
        };
        let progress = Arc::new(Mutex::new(Progress::default()));
        self.progress = progress.clone();
        let no_access = self.t("inst.noAccess");
        std::thread::spawn(move || {
            let set_step = |n: usize| {
                if let Ok(mut p) = progress.lock() {
                    p.step = n;
                }
            };
            let res = win::install(&opt, PAYLOAD, LICENSE, VERSION, |s| {
                set_step(match s {
                    win::Step::Files => 1,
                    win::Step::Shortcuts => 2,
                    win::Step::Register => 3,
                });
            })
            .map_err(|e| match e.kind() {
                ErrorKind::PermissionDenied => no_access,
                _ => e.to_string(),
            });
            if let Ok(mut p) = progress.lock() {
                p.step = 4;
                p.result = Some(res);
            }
        });
    }

    fn start_remove(&mut self) {
        let dir = self.target();
        if win::running(&dir) {
            self.blocked = true;
            return;
        }
        self.blocked = false;
        match win::uninstall(&dir, self.remove_data) {
            Ok(()) => self.page = Page::Removed,
            Err(e) => {
                self.error = e.to_string();
                self.page = Page::Failed;
            }
        }
    }

    fn body(&mut self, ui: &mut egui::Ui) {
        let c = style::colors(ui.ctx());
        match self.page {
            Page::Welcome => {
                ui.label(self.tf("inst.welcomeText", &[("v", VERSION.into())]));
                if let Some(f) = &self.found {
                    ui.add_space(8.0);
                    let text = self.tf(
                        "inst.found",
                        &[
                            ("v", f.version.clone()),
                            ("dir", f.dir.display().to_string()),
                        ],
                    );
                    note(ui, icons::INFO, &text, c.accent_soft);
                }
                if PAYLOAD.is_empty() {
                    ui.add_space(8.0);
                    note(ui, icons::WARN, &self.t("inst.noPayload"), c.danger_soft);
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(self.t("inst.lang"));
                    ui.selectable_value(&mut self.lang, Lang::Ru, "Русский");
                    ui.selectable_value(&mut self.lang, Lang::En, "English");
                });
            }
            Page::License => {
                ui.label(self.t("inst.licenseText"));
                ui.add_space(6.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new(LICENSE).font(style::mono(11.0)));
                });
            }
            Page::Privacy => {
                let text = if self.lang == Lang::Ru {
                    PRIVACY_RU
                } else {
                    PRIVACY_EN
                };
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 40.0)
                    .show(ui, |ui| style::markdown(ui, text));
                ui.add_space(6.0);
                let label = self.t("inst.privacyAccept");
                ui.checkbox(&mut self.accepted, label);
            }
            Page::Folder => {
                if let Some(f) = &self.found {
                    ui.label(self.t("inst.folderFixed"));
                    ui.label(RichText::new(f.dir.display().to_string()).font(style::mono(13.0)));
                } else {
                    ui.label(self.t("inst.folderText"));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.dir)
                                .desired_width(ui.available_width() - 110.0),
                        );
                        if ui.button(self.t("inst.browse")).clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .set_directory(&self.dir)
                                .pick_folder()
                            {
                                self.dir = p.join("ShelterSave-Studio").display().to_string();
                            }
                        }
                    });
                }
            }
            Page::Options => {
                let (sm, dk, assoc) = (
                    self.t("inst.startMenu"),
                    self.t("inst.desktop"),
                    self.t("inst.assoc"),
                );
                ui.checkbox(&mut self.start_menu, sm);
                ui.checkbox(&mut self.desktop, dk);
                ui.checkbox(&mut self.assoc, assoc);
                ui.label(
                    RichText::new(self.t("inst.assocHelp"))
                        .small()
                        .color(c.muted),
                );
                if self.blocked {
                    ui.add_space(8.0);
                    note(ui, icons::WARN, &self.t("inst.running"), c.warning_soft);
                }
            }
            Page::Work => {
                let step = self.progress.lock().map(|p| p.step).unwrap_or(0);
                ui.add(egui::ProgressBar::new(step as f32 / 4.0).show_percentage());
                ui.add_space(8.0);
                for (n, key) in ["inst.stepFiles", "inst.stepShortcuts", "inst.stepRegister"]
                    .iter()
                    .enumerate()
                {
                    let mark = if step > n + 1 { icons::CHECK } else { "·" };
                    let text = format!("{mark}  {}", self.t(key));
                    let color = if step > n { c.text } else { c.muted };
                    ui.label(RichText::new(text).color(color));
                }
            }
            Page::Done => {
                ui.label(self.tf(
                    "inst.doneText",
                    &[("v", VERSION.into()), ("dir", self.dir.clone())],
                ));
                ui.add_space(8.0);
                let label = self.t("inst.launch");
                ui.checkbox(&mut self.launch, label);
            }
            Page::Failed => {
                note(ui, icons::WARN, &self.error, c.danger_soft);
            }
            Page::Remove => {
                let dir = self
                    .found
                    .as_ref()
                    .map_or_else(|| self.dir.clone(), |f| f.dir.display().to_string());
                ui.label(self.tf("inst.uText", &[("dir", dir)]));
                ui.add_space(8.0);
                let label = self.t("inst.uData");
                ui.checkbox(&mut self.remove_data, label);
                ui.label(
                    RichText::new(self.t("inst.uDataHelp"))
                        .small()
                        .color(c.muted),
                );
                if self.blocked {
                    ui.add_space(8.0);
                    note(ui, icons::WARN, &self.t("inst.running"), c.warning_soft);
                }
            }
            Page::Removed => {
                ui.label(self.t("inst.uDone"));
            }
        }
    }

    fn buttons(&mut self, ui: &mut egui::Ui) {
        let close = |ui: &egui::Ui| ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| match self.page {
                Page::Work => {
                    ui.add_enabled(false, egui::Button::new(self.t("inst.cancel")));
                }
                Page::Done => {
                    if primary(ui, &self.t("inst.finish")) {
                        if self.launch {
                            win::launch(&self.target());
                        }
                        close(ui);
                    }
                }
                Page::Failed | Page::Removed => {
                    if primary(ui, &self.t("inst.finish")) {
                        close(ui);
                    }
                }
                Page::Remove => {
                    let label = if self.blocked {
                        self.t("inst.retry")
                    } else {
                        self.t("inst.remove")
                    };
                    if primary(ui, &label) {
                        self.start_remove();
                    }
                    if ui.button(self.t("inst.cancel")).clicked() {
                        close(ui);
                    }
                }
                page => {
                    let next = match page {
                        Page::Welcome => Some(Page::License),
                        Page::License => Some(Page::Privacy),
                        Page::Privacy => Some(Page::Folder),
                        Page::Folder => Some(Page::Options),
                        _ => None,
                    };
                    let can = match page {
                        Page::Welcome => !PAYLOAD.is_empty(),
                        Page::Privacy => self.accepted,
                        Page::Folder => self.target().is_absolute(),
                        _ => true,
                    };
                    let label = match (next, &self.found, self.blocked) {
                        (Some(_), _, _) => self.t("inst.next"),
                        (None, _, true) => self.t("inst.retry"),
                        (None, Some(_), false) => self.t("inst.update"),
                        (None, None, false) => self.t("inst.install"),
                    };
                    if ui.add_enabled(can, primary_button(ui, &label)).clicked() {
                        match next {
                            Some(p) => self.page = p,
                            None => self.start_install(),
                        }
                    }
                    if page != Page::Welcome && ui.button(self.t("inst.back")).clicked() {
                        self.blocked = false;
                        self.page = match page {
                            Page::License => Page::Welcome,
                            Page::Privacy => Page::License,
                            Page::Folder => Page::Privacy,
                            _ => Page::Folder,
                        };
                    }
                    ui.add_space(12.0);
                    if ui.button(self.t("inst.cancel")).clicked() {
                        close(ui);
                    }
                }
            },
        );
    }
}

impl eframe::App for Wizard {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let c = style::colors(ui.ctx());
        if self.page == Page::Work {
            let done = self.progress.lock().ok().and_then(|p| p.result.clone());
            match done {
                Some(Ok(())) => self.page = Page::Done,
                Some(Err(e)) => {
                    self.error = e;
                    self.page = Page::Failed;
                }
                None => ui
                    .ctx()
                    .request_repaint_after(std::time::Duration::from_millis(50)),
            }
        }
        let title = if self.uninstalling() {
            self.t("inst.uTitle")
        } else {
            self.tf("inst.title", &[("v", VERSION.into())])
        };
        egui::Panel::top("head")
            .frame(
                egui::Frame::new()
                    .fill(c.surface)
                    .inner_margin(egui::Margin::symmetric(20, 14))
                    .stroke(egui::Stroke::new(1.0, c.border)),
            )
            .resizable(false)
            .show(ui, |ui| {
                ui.label(RichText::new(title).small().color(c.muted));
                ui.label(RichText::new(self.heading()).font(style::bold(20.0)));
            });
        egui::Panel::bottom("buttons")
            .frame(
                egui::Frame::new()
                    .fill(c.surface)
                    .inner_margin(egui::Margin::symmetric(20, 12))
                    .stroke(egui::Stroke::new(1.0, c.border)),
            )
            .resizable(false)
            .show(ui, |ui| self.buttons(ui));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(c.bg)
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |ui| self.body(ui));
    }
}

fn primary_button(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    let c = style::colors(ui.ctx());
    egui::Button::new(RichText::new(text.to_string()).color(c.on_accent)).fill(c.accent)
}

fn primary(ui: &mut egui::Ui, text: &str) -> bool {
    let b = primary_button(ui, text);
    ui.add(b).clicked()
}

fn note(ui: &mut egui::Ui, icon: &str, text: &str, fill: egui::Color32) {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(icon);
                ui.label(text);
            });
        });
}
