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

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Key, KeyboardShortcut, Layout, Modifiers, RichText, Sense};
use shelter_core::actions::{Action, ActionError};
use shelter_core::catalog::ItemKind;
use shelter_core::document::{Document, SourceKind};
use shelter_core::i18n::{self, Lang};
use shelter_core::rules::{Mode, RuleError};

use crate::files::{self, FoundSave, Stamp};
use crate::icons;
use crate::settings::Settings;
use crate::style::{self, Colors};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Save,
    Vault,
    Dwellers,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VaultTab {
    #[default]
    Resources,
    Storage,
    Rooms,
    Themes,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OtherTab {
    #[default]
    Quests,
    Wasteland,
    Objectives,
    Stats,
    Tutorial,
    Events,
    Service,
    Tree,
}

pub struct OpenDoc {
    pub doc: Document,
    pub path: Option<PathBuf>,
    pub stamp: Option<Stamp>,
    pub backed_up: bool,
}

pub enum Modal {
    FirstRun,
    Review,
    Settings,
    About,
    Journal,
    ExpertConfirm,
    CloseConfirm(AfterClose),
    ConfirmMass(Action, usize),
    ConfirmDelete(Vec<i64>),
    NewDweller {
        first: String,
        last: String,
        gender: i64,
    },
    Restore,
}

#[derive(Clone)]
pub enum AfterClose {
    Exit,
    Open(Option<PathBuf>),
    CloseFile,
}

pub struct Toast {
    pub text: String,
    pub undo: bool,
    pub error: bool,
    pub until: Instant,
}

#[derive(Default)]
pub struct DwellerUi {
    pub search: String,
    pub filter: usize,
    pub sort: usize,
    pub desc: bool,
    pub selected: BTreeSet<i64>,
    pub focus: Option<i64>,
    pub mass_field: String,
    pub mass_value: String,
    pub equip_kind: usize,
    pub equip_query: String,
}

#[derive(Default)]
pub struct StorageUi {
    pub search: String,
    pub kind: Option<ItemKind>,
    pub rarity: Option<String>,
    pub add_query: String,
    pub add_kind: Option<ItemKind>,
    pub add_count: i64,
    pub selected: BTreeSet<(ItemKind, String)>,
}

pub struct App {
    pub settings: Settings,
    pub lang: Lang,
    pub open: Option<OpenDoc>,
    pub section: Section,
    pub vault_tab: VaultTab,
    pub other_tab: OtherTab,
    pub found: Vec<FoundSave>,
    pub modals: Vec<Modal>,
    pub toasts: Vec<Toast>,
    pub dw: DwellerUi,
    pub st: StorageUi,
    pub room_sel: Option<i64>,
    pub map_add: Option<crate::sec_vault::MapAdd>,
    pub map_drag: Option<crate::sec_vault::MapDrag>,
    pub tree_search: String,
    pub tree_edit: HashMap<String, String>,
    pub bufs: HashMap<String, String>,
    pub drags: HashMap<String, f64>,
    pub global_search: String,
    pub allow_close: bool,
    pub last_focus_check: bool,
    pub resource_all: f64,
    pub load_error: Option<String>,
    pub tree_add_key: String,
    pub tree_add_value: String,
    pub restore_list: Vec<files::Backup>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let settings = Settings::load();
        let lang = settings.lang();
        style::install_fonts(&cc.egui_ctx);
        style::apply(&cc.egui_ctx, false);
        let mut app = App {
            lang,
            open: None,
            section: Section::Save,
            vault_tab: VaultTab::default(),
            other_tab: OtherTab::default(),
            found: files::discover(&settings.save_dirs),
            modals: Vec::new(),
            toasts: Vec::new(),
            dw: DwellerUi::default(),
            st: StorageUi {
                add_count: 1,
                ..StorageUi::default()
            },
            room_sel: None,
            map_add: None,
            map_drag: None,
            tree_search: String::new(),
            tree_edit: HashMap::new(),
            bufs: HashMap::new(),
            drags: HashMap::new(),
            global_search: String::new(),
            allow_close: false,
            last_focus_check: true,
            resource_all: 999_999.0,
            load_error: None,
            tree_add_key: String::new(),
            tree_add_value: String::new(),
            restore_list: Vec::new(),
            settings,
        };
        app.apply_appearance(&cc.egui_ctx);
        if !app.settings.accepted_terms {
            app.modals.push(Modal::FirstRun);
        }
        if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) {
            app.open_path(&path);
        }
        app
    }

    pub fn t(&self, key: &str) -> String {
        i18n::tr(self.lang, key)
    }

    pub fn tf(&self, key: &str, params: &[(&str, String)]) -> String {
        i18n::trf(self.lang, key, params)
    }

    pub fn mode(&self) -> Mode {
        self.settings.mode
    }

    pub fn expert(&self) -> bool {
        self.settings.mode == Mode::Expert
    }

    pub fn apply_appearance(&self, ctx: &egui::Context) {
        use crate::settings::ThemePref;
        ctx.set_theme(match self.settings.theme {
            ThemePref::System => egui::ThemePreference::System,
            ThemePref::Light => egui::ThemePreference::Light,
            ThemePref::Dark => egui::ThemePreference::Dark,
        });
        ctx.set_zoom_factor(self.settings.scale);
    }

    pub fn save_settings(&mut self) {
        if let Err(e) = self.settings.save() {
            self.toast_error(self.tf("err.io", &[("detail", e.to_string())]));
        }
    }

    pub fn toast(&mut self, text: String, undo: bool) {
        self.toasts.push(Toast {
            text,
            undo,
            error: false,
            until: Instant::now() + Duration::from_secs(6),
        });
    }

    pub fn toast_error(&mut self, text: String) {
        self.toasts.push(Toast {
            text,
            undo: false,
            error: true,
            until: Instant::now() + Duration::from_secs(8),
        });
    }

    pub fn label_text(&self, label: &shelter_core::edit::Label) -> String {
        let params: Vec<(&str, String)> = label
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();
        self.tf(&label.key, &params)
    }

    pub fn error_text(&self, e: &ActionError) -> String {
        match e {
            ActionError::Rule(RuleError::OutOfRange { min, max }) => self.tf(
                "aerr.outOfRange",
                &[("min", min.to_string()), ("max", max.to_string())],
            ),
            ActionError::Rule(RuleError::NotAllowed) => self.t("aerr.notAllowed"),
            ActionError::Rule(RuleError::NotInCatalog) => self.t("aerr.notInCatalog"),
            ActionError::Rule(RuleError::WrongType) => self.t("aerr.wrongType"),
            ActionError::Rule(RuleError::ExpertOnly) => self.t("aerr.expertOnly"),
            ActionError::Rule(RuleError::TooManyDwellers) => self.t("aerr.tooManyDwellers"),
            ActionError::Rule(RuleError::RoomOverlap) => self.t("aerr.roomOverlap"),
            ActionError::Rule(RuleError::RoomDisconnected) => self.t("aerr.roomDisconnected"),
            ActionError::Rule(RuleError::RoomBusy) => self.t("aerr.roomBusy"),
            ActionError::Rule(RuleError::RoomFixed) => self.t("aerr.roomFixed"),
            ActionError::Rule(RuleError::RoomUnique) => self.t("aerr.roomUnique"),
            ActionError::Rule(RuleError::NotFound) => self.t("aerr.notFound"),
            ActionError::BadJson(d) => self.tf("aerr.badJson", &[("detail", d.clone())]),
            ActionError::Nothing => self.t("aerr.nothing"),
            ActionError::Edit(d) => self.tf("aerr.edit", &[("detail", d.clone())]),
        }
    }

    /// Runs an action; mass actions get a toast with an undo button.
    pub fn act(&mut self, action: Action) -> bool {
        let mode = self.mode();
        let Some(open) = self.open.as_mut() else {
            return false;
        };
        match open.doc.apply(mode, &action) {
            Ok(tx) => {
                if tx.affected > 1 {
                    let text = self.tf(
                        "c.undoToast",
                        &[
                            ("label", self.label_text(&tx.label)),
                            ("n", tx.affected.to_string()),
                        ],
                    );
                    self.toast(text, true);
                }
                true
            }
            Err(ActionError::Nothing) => false,
            Err(e) => {
                let text = self.error_text(&e);
                self.toast_error(text);
                false
            }
        }
    }

    /// Mass action with optional confirmation.
    pub fn act_mass(&mut self, action: Action, count: usize) {
        if self.settings.confirm_mass && count > 1 {
            self.modals.push(Modal::ConfirmMass(action, count));
        } else {
            self.act(action);
        }
    }

    pub fn undo(&mut self) {
        let Some(open) = self.open.as_mut() else {
            return;
        };
        if let Err(e) = open.doc.undo() {
            let text = self.tf("aerr.edit", &[("detail", e.to_string())]);
            self.toast_error(text);
        }
    }

    pub fn redo(&mut self) {
        let Some(open) = self.open.as_mut() else {
            return;
        };
        if let Err(e) = open.doc.redo() {
            let text = self.tf("aerr.edit", &[("detail", e.to_string())]);
            self.toast_error(text);
        }
    }

    pub fn dirty(&self) -> bool {
        self.open.as_ref().is_some_and(|o| o.doc.is_dirty())
    }

    pub fn request_open(&mut self, path: Option<PathBuf>) {
        if self.dirty() {
            self.modals
                .push(Modal::CloseConfirm(AfterClose::Open(path)));
        } else {
            self.open_dialog_or(path);
        }
    }

    pub fn open_dialog_or(&mut self, path: Option<PathBuf>) {
        let path = path.or_else(|| {
            let mut d = rfd::FileDialog::new().add_filter("Fallout Shelter", &["sav", "json"]);
            if let Some(dir) = files::standard_save_dirs().into_iter().find(|d| d.is_dir()) {
                d = d.set_directory(dir);
            }
            d.pick_file()
        });
        if let Some(p) = path {
            self.open_path(&p);
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        self.load_error = None;
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.load_error = Some(self.tf("err.io", &[("detail", e.to_string())]));
                return;
            }
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        match Document::open(&bytes, &name) {
            Ok(doc) => {
                self.open = Some(OpenDoc {
                    doc,
                    path: Some(path.to_path_buf()),
                    stamp: files::stamp(path),
                    backed_up: false,
                });
                self.reset_view();
                self.settings.push_recent(path);
                self.save_settings();
            }
            Err(e) => {
                let detail = match &e {
                    shelter_core::sav::FormatError::NotJson(p) => p.to_string(),
                    _ => String::new(),
                };
                self.load_error = Some(format!(
                    "{} {}",
                    self.tf(&format!("err.{}", e.code()), &[("detail", detail)]),
                    self.t("err.what")
                ));
            }
        }
    }

    pub fn open_bytes(&mut self, bytes: &[u8], name: &str) {
        self.load_error = None;
        match Document::open(bytes, name) {
            Ok(doc) => {
                self.open = Some(OpenDoc {
                    doc,
                    path: None,
                    stamp: None,
                    backed_up: false,
                });
                self.reset_view();
            }
            Err(e) => {
                self.load_error = Some(format!(
                    "{} {}",
                    self.tf(&format!("err.{}", e.code()), &[("detail", String::new())]),
                    self.t("err.what")
                ));
            }
        }
    }

    fn reset_view(&mut self) {
        self.section = Section::Save;
        self.dw = DwellerUi::default();
        self.st = StorageUi {
            add_count: 1,
            ..StorageUi::default()
        };
        self.room_sel = None;
        self.map_add = None;
        self.map_drag = None;
        self.bufs.clear();
        self.drags.clear();
        self.tree_edit.clear();
    }

    pub fn close_file(&mut self) {
        if self.dirty() {
            self.modals.push(Modal::CloseConfirm(AfterClose::CloseFile));
        } else {
            self.open = None;
            self.found = files::discover(&self.settings.save_dirs);
        }
    }

    pub fn changed_on_disk(&self) -> bool {
        match &self.open {
            Some(OpenDoc {
                path: Some(p),
                stamp: Some(s),
                ..
            }) => files::stamp(p).is_some_and(|now| now != *s),
            _ => false,
        }
    }

    /// Writes the document to `target` (backup first, atomic write).
    pub fn write_to(&mut self, target: &Path) {
        let keep = self.settings.backup_keep;
        let backups = self.settings.backups;
        let backup_root = self.settings.backup_dir();
        let Some(open) = self.open.as_mut() else {
            return;
        };
        let as_json = target
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
            || (open.doc.source == SourceKind::Json && open.path.as_deref() == Some(target));
        let data = if as_json {
            open.doc.to_json_pretty()
        } else {
            open.doc.to_sav()
        };
        let mut backup_note = None;
        if backups && target.exists() && (!open.backed_up || open.path.as_deref() != Some(target)) {
            if let Some(root) = backup_root {
                match files::make_backup(&root, target, keep) {
                    Ok(p) => {
                        backup_note = Some(p);
                        if open.path.as_deref() == Some(target) {
                            open.backed_up = true;
                        }
                    }
                    Err(e) => {
                        let text = i18n::trf(self.lang, "err.io", &[("detail", e.to_string())]);
                        self.toast_error(text);
                        return;
                    }
                }
            }
        }
        match files::write_atomic(target, data.as_bytes()) {
            Ok(()) => {
                open.doc.mark_saved();
                open.path = Some(target.to_path_buf());
                open.stamp = files::stamp(target);
                open.doc.file_name = target
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let mut text = i18n::trf(
                    self.lang,
                    "review.saved",
                    &[("file", target.display().to_string())],
                );
                if let Some(b) = backup_note {
                    text.push_str(". ");
                    text.push_str(&i18n::trf(
                        self.lang,
                        "review.backup",
                        &[("file", b.display().to_string())],
                    ));
                }
                self.toast(text, false);
                self.settings.push_recent(target);
                self.save_settings();
            }
            Err(e) => {
                let text = i18n::trf(self.lang, "err.io", &[("detail", e.to_string())]);
                self.toast_error(text);
            }
        }
    }

    pub fn save_as(&mut self) {
        let Some(open) = &self.open else { return };
        let name = open.doc.file_name.clone();
        let mut d = rfd::FileDialog::new()
            .add_filter("Fallout Shelter", &["sav"])
            .add_filter("JSON", &["json"])
            .set_file_name(&name);
        if let Some(dir) = open.path.as_ref().and_then(|p| p.parent()) {
            d = d.set_directory(dir);
        }
        if let Some(target) = d.save_file() {
            self.write_to(&target);
        }
    }

    pub fn export_json(&mut self) {
        let Some(open) = &self.open else { return };
        let stem = Path::new(&open.doc.file_name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "vault".into());
        let text = open.doc.to_json_pretty();
        if let Some(target) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name(format!("{stem}.json"))
            .save_file()
        {
            if let Err(e) = files::write_atomic(&target, text.as_bytes()) {
                let t = self.tf("err.io", &[("detail", e.to_string())]);
                self.toast_error(t);
            } else {
                let t = self.tf("review.saved", &[("file", target.display().to_string())]);
                self.toast(t, false);
            }
        }
    }

    pub fn import_json(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                let t = self.tf("err.io", &[("detail", e.to_string())]);
                self.toast_error(t);
                return;
            }
        };
        match shelter_core::sav::read_json(&bytes) {
            Ok(root) => {
                let json = root.to_compact();
                self.act(Action::SetRawRoot { json });
            }
            Err(e) => {
                let t = format!(
                    "{} {}",
                    self.tf(&format!("err.{}", e.code()), &[("detail", String::new())]),
                    self.t("err.what")
                );
                self.toast_error(t);
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = Modifiers::COMMAND;
        let pressed = |k: Key, m: Modifiers| {
            ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)))
        };
        if self.modals.is_empty() {
            if pressed(Key::Z, cmd | Modifiers::SHIFT) || pressed(Key::Y, cmd) {
                self.redo();
            } else if pressed(Key::Z, cmd) {
                self.undo();
            }
            if pressed(Key::S, cmd | Modifiers::SHIFT) {
                self.save_as();
            } else if pressed(Key::S, cmd) && self.open.is_some() {
                self.modals.push(Modal::Review);
            }
            if pressed(Key::O, cmd) {
                self.request_open(None);
            }
            if self.open.is_some() {
                for (k, s) in [
                    (Key::Num1, Section::Save),
                    (Key::Num2, Section::Vault),
                    (Key::Num3, Section::Dwellers),
                    (Key::Num4, Section::Other),
                ] {
                    if pressed(k, Modifiers::ALT) {
                        self.section = s;
                    }
                }
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let c = style::colors(ui.ctx());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(RichText::new(self.t("app.name")).font(style::bold(16.0)));
            ui.separator();
            if icon_button(ui, icons::OPEN, &self.t("tb.open"), true).clicked() {
                self.request_open(None);
            }
            let has = self.open.is_some();
            let can_undo = self
                .open
                .as_ref()
                .is_some_and(|o| o.doc.history().can_undo());
            let can_redo = self
                .open
                .as_ref()
                .is_some_and(|o| o.doc.history().can_redo());
            if icon_button(ui, icons::SAVE, &self.t("tb.save"), has).clicked() {
                self.modals.push(Modal::Review);
            }
            if icon_button(ui, icons::DOWNLOAD, &self.t("tb.saveAs"), has).clicked() {
                self.save_as();
            }
            ui.separator();
            if icon_button(
                ui,
                icons::UNDO,
                &format!("{} (Ctrl+Z)", self.t("tb.undo")),
                can_undo,
            )
            .clicked()
            {
                self.undo();
            }
            if icon_button(
                ui,
                icons::REDO,
                &format!("{} (Ctrl+Y)", self.t("tb.redo")),
                can_redo,
            )
            .clicked()
            {
                self.redo();
            }
            if icon_button(ui, icons::HISTORY, &self.t("tb.journal"), has).clicked() {
                self.modals.push(Modal::Journal);
            }
            ui.separator();
            crate::search::toolbar_search(self, ui);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, icons::INFO, &self.t("tb.about"), true).clicked() {
                    self.modals.push(Modal::About);
                }
                if icon_button(ui, icons::SETTINGS, &self.t("tb.settings"), true).clicked() {
                    self.modals.push(Modal::Settings);
                }
                let (mode_text, fill, fg) = if self.expert() {
                    (self.t("tb.mode.expert"), c.warning_soft, c.warning)
                } else {
                    (self.t("tb.mode.normal"), c.surface_alt, c.muted)
                };
                badge(ui, &mode_text, fill, fg);
                if has {
                    let (t, fill, fg) = if self.dirty() {
                        (format!("● {}", self.t("tb.unsaved")), c.mark_soft, c.text)
                    } else {
                        (self.t("tb.saved"), c.surface_alt, c.muted)
                    };
                    badge(ui, &t, fill, fg);
                }
            });
        });
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        let c = style::colors(ui.ctx());
        ui.add_space(8.0);
        for (s, icon, key, n) in [
            (Section::Save, icons::SHIELD, "nav.save", 1),
            (Section::Vault, icons::HOUSE, "nav.vault", 2),
            (Section::Dwellers, icons::USERS, "nav.dwellers", 3),
            (Section::Other, icons::LAYERS, "nav.other", 4),
        ] {
            let selected = self.section == s;
            let text = RichText::new(format!("{icon}  {}", self.t(key))).size(15.0);
            let text = if selected {
                text.color(c.accent).font(style::bold(15.0))
            } else {
                text
            };
            let resp = ui
                .add_sized(
                    [ui.available_width(), 36.0],
                    egui::Button::selectable(selected, text),
                )
                .on_hover_text(format!("Alt+{n}"));
            accessible(&resp, &self.t(key));
            if resp.clicked() {
                self.section = s;
            }
        }
        ui.add_space(12.0);
        ui.separator();
        if let Some(open) = &self.open {
            ui.add_space(4.0);
            ui.label(RichText::new(&open.doc.file_name).font(style::bold(13.0)));
            if let Some(p) = &open.path {
                ui.label(
                    RichText::new(p.display().to_string())
                        .small()
                        .color(c.muted),
                )
                .on_hover_text(p.display().to_string());
            }
            ui.add_space(6.0);
            if ui
                .button(format!("{}  {}", icons::X, self.t("tb.close")))
                .clicked()
            {
                self.close_file();
            }
        }
    }

    fn toasts_ui(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.toasts.retain(|t| t.until > now);
        if self.toasts.is_empty() {
            return;
        }
        let c = style::colors(ctx);
        let mut undo = false;
        let mut dismiss = None;
        egui::Area::new(egui::Id::unique("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                for (i, t) in self.toasts.iter().enumerate() {
                    let (fill, stroke) = if t.error {
                        (c.danger_soft, c.danger)
                    } else {
                        (c.surface, c.border)
                    };
                    egui::Frame::new()
                        .fill(fill)
                        .stroke(egui::Stroke::new(1.0, stroke))
                        .corner_radius(8)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.set_max_width(420.0);
                            ui.horizontal_wrapped(|ui| {
                                let icon = if t.error { icons::ALERT } else { icons::CHECK };
                                ui.label(RichText::new(icon).color(if t.error {
                                    c.danger
                                } else {
                                    c.success
                                }));
                                ui.label(&t.text);
                                if t.undo && ui.button(i18n::tr(self.lang, "c.undo")).clicked() {
                                    undo = true;
                                    dismiss = Some(i);
                                }
                                if crate::app::small_icon(
                                    ui,
                                    icons::X,
                                    &i18n::tr(self.lang, "c.close"),
                                )
                                .clicked()
                                {
                                    dismiss = Some(i);
                                }
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        if let Some(i) = dismiss {
            self.toasts.remove(i);
        }
        if undo {
            self.undo();
        }
        let next = self.toasts.iter().map(|t| t.until).min();
        if let Some(n) = next {
            ctx.request_repaint_after(n.saturating_duration_since(now));
        }
    }

    fn handle_drop(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(f) = dropped.into_iter().next() {
            let p = f.path().to_path_buf();
            if p.as_os_str().is_empty() {
                if let Ok(bytes) = f.bytes() {
                    self.open_bytes(&bytes, "Vault.sav");
                }
            } else {
                self.request_open(Some(p));
            }
        }
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if hovering {
            let c = style::colors(ctx);
            let screen = ctx.content_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::unique("drop"),
            ));
            painter.rect_filled(screen, 0.0, c.accent_soft.gamma_multiply(0.92));
            painter.text(
                screen.center(),
                egui::Align2::CENTER_CENTER,
                self.t("start.drop"),
                style::bold(22.0),
                c.text,
            );
        }
    }

    fn close_guard(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && self.dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self
                .modals
                .iter()
                .any(|m| matches!(m, Modal::CloseConfirm(_)))
            {
                self.modals.push(Modal::CloseConfirm(AfterClose::Exit));
            }
        }
    }

    fn start_screen(&mut self, ui: &mut egui::Ui) {
        let c = style::colors(ui.ctx());
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new(icons::ARCHIVE).size(40.0).color(c.accent));
                ui.add_space(8.0);
                ui.heading(self.t("start.title"));
                ui.label(RichText::new(self.t("start.text")).color(c.muted));
                ui.add_space(12.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!("{}  {}", icons::OPEN, self.t("start.choose")))
                                .color(c.on_accent),
                        )
                        .fill(c.accent),
                    )
                    .clicked()
                {
                    self.request_open(None);
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{}  {}", icons::WARN, self.t("start.gameClosed")))
                        .small()
                        .color(c.muted),
                );
            });
            if let Some(err) = self.load_error.clone() {
                ui.add_space(16.0);
                callout(
                    ui,
                    c.danger_soft,
                    c.danger,
                    icons::ALERT,
                    &self.t("err.title"),
                    &err,
                );
            }
            ui.add_space(24.0);
            ui.label(RichText::new(self.t("start.found")).font(style::bold(16.0)));
            ui.add_space(4.0);
            if self.found.is_empty() {
                ui.label(RichText::new(self.t("start.foundNone")).color(c.muted));
            }
            let mut open = None;
            for f in &self.found {
                let title = format!(
                    "{}  {} {}",
                    icons::HOUSE,
                    f.vault_name,
                    f.path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                );
                let sub = format!(
                    "{} · {}",
                    i18n::trf(
                        self.lang,
                        "start.dwellers",
                        &[("n", f.dwellers.to_string())]
                    ),
                    i18n::trf(
                        self.lang,
                        "start.modified",
                        &[("date", files::format_time(f.modified))]
                    )
                );
                if card_button(ui, &title, &sub).clicked() {
                    open = Some(f.path.clone());
                }
            }
            if !self.settings.recent.is_empty() {
                ui.add_space(16.0);
                ui.label(RichText::new(self.t("start.recent")).font(style::bold(16.0)));
                for p in &self.settings.recent {
                    if p.exists()
                        && card_button(ui, &format!("{}  {}", icons::HISTORY, p.display()), "")
                            .clicked()
                    {
                        open = Some(p.clone());
                    }
                }
            }
            if let Some(p) = open {
                self.request_open(Some(p));
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let reduce = false;
        let _ = reduce;
        self.close_guard(&ctx);
        self.shortcuts(&ctx);
        self.handle_drop(&ctx);
        if ctx.input(|i| i.focused) && !self.last_focus_check && self.changed_on_disk() {
            let text = self.t("review.changedOnDisk");
            self.toast_error(text);
        }
        self.last_focus_check = ctx.input(|i| i.focused);
        let c: Colors = style::colors(&ctx);

        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(c.surface)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(egui::Stroke::new(1.0, c.border)),
            )
            .resizable(false)
            .show(ui, |ui| self.toolbar(ui));
        if self.open.is_some() {
            egui::Panel::left("nav")
                .frame(
                    egui::Frame::new()
                        .fill(c.surface)
                        .inner_margin(egui::Margin::same(10))
                        .stroke(egui::Stroke::new(1.0, c.border)),
                )
                .resizable(false)
                .exact_size(210.0)
                .show(ui, |ui| self.nav(ui));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(c.bg)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ui, |ui| {
                if self.open.is_none() {
                    self.start_screen(ui);
                    return;
                }
                match self.section {
                    Section::Save => crate::sec_save::show(self, ui),
                    Section::Vault => crate::sec_vault::show(self, ui),
                    Section::Dwellers => crate::sec_dwellers::show(self, ui),
                    Section::Other => crate::sec_other::show(self, ui),
                }
            });
        crate::dialogs::show(self, &ctx);
        self.toasts_ui(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.settings.save();
    }
}

pub fn icon_button(ui: &mut egui::Ui, icon: &str, tip: &str, enabled: bool) -> egui::Response {
    let resp = ui
        .add_enabled(
            enabled,
            egui::Button::new(RichText::new(icon).size(16.0)).min_size(egui::vec2(32.0, 30.0)),
        )
        .on_hover_text(tip)
        .on_disabled_hover_text(tip);
    accessible(&resp, tip);
    resp
}

pub fn small_icon(ui: &mut egui::Ui, icon: &str, tip: &str) -> egui::Response {
    let resp = ui.small_button(icon).on_hover_text(tip);
    accessible(&resp, tip);
    resp
}

/// Gives a widget a spoken name without icon glyphs.
pub fn accessible(resp: &egui::Response, name: &str) {
    let name: String = name
        .chars()
        .filter(|c| !('\u{e000}'..='\u{f8ff}').contains(c))
        .collect();
    let name = name.trim().to_string();
    let enabled = resp.enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, name.clone()));
}

pub fn badge(ui: &mut egui::Ui, text: &str, fill: egui::Color32, fg: egui::Color32) {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().color(fg));
        });
}

pub fn callout(
    ui: &mut egui::Ui,
    fill: egui::Color32,
    accent: egui::Color32,
    icon: &str,
    title: &str,
    text: &str,
) {
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, accent))
        .corner_radius(8)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(accent).size(16.0));
                ui.label(RichText::new(title).font(style::bold(14.0)));
            });
            ui.label(text);
        });
}

pub fn card_button(ui: &mut egui::Ui, title: &str, sub: &str) -> egui::Response {
    let c = style::colors(ui.ctx());
    let resp = egui::Frame::new()
        .fill(c.surface)
        .stroke(egui::Stroke::new(1.0, c.border))
        .corner_radius(8)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).font(style::bold(14.0)));
            if !sub.is_empty() {
                ui.label(RichText::new(sub).small().color(c.muted));
            }
        })
        .response;
    let resp = resp.interact(Sense::click());
    if resp.hovered() {
        ui.painter().rect_stroke(
            resp.rect,
            8.0,
            egui::Stroke::new(1.0, c.accent),
            egui::StrokeKind::Inside,
        );
    }
    if resp.has_focus() {
        ui.painter().rect_stroke(
            resp.rect,
            8.0,
            egui::Stroke::new(2.0, c.focus),
            egui::StrokeKind::Inside,
        );
    }
    ui.add_space(6.0);
    resp
}

pub fn section_title(ui: &mut egui::Ui, icon: &str, title: &str, help: &str) {
    let c = style::colors(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(18.0).color(c.accent));
        ui.label(RichText::new(title).font(style::bold(18.0)));
    });
    if !help.is_empty() {
        ui.label(RichText::new(help).color(c.muted));
    }
    ui.add_space(6.0);
}

pub fn panel<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let c = style::colors(ui.ctx());
    egui::Frame::new()
        .fill(c.surface)
        .stroke(egui::Stroke::new(1.0, c.border))
        .corner_radius(8)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}
