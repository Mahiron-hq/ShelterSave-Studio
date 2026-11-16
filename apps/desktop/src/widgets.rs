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

//! Labeled field rows bound to a path in the save.

use eframe::egui::{self, RichText};
use shelter_core::actions::Action;
use shelter_core::fields;
use shelter_core::json::{path_to_string, Seg, Value};

use crate::app::App;
use crate::icons;
use crate::style;

#[derive(Default, Clone)]
pub struct Opts {
    pub label: String,
    pub help: String,
    pub unit: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub read_only: bool,
}

impl Opts {
    /// Label and description from the field dictionary.
    pub fn at(app: &App, path: &[Seg]) -> Opts {
        let f = fields::describe(app.lang, path);
        Opts {
            label: f.label,
            help: f.help,
            ..Opts::default()
        }
    }

    pub fn text(label: String) -> Opts {
        Opts {
            label,
            ..Opts::default()
        }
    }

    pub fn help(mut self, help: String) -> Opts {
        self.help = help;
        self
    }

    pub fn unit(mut self, unit: String) -> Opts {
        self.unit = unit;
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Opts {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
}

/// Bold label; its description appears as a tooltip after a short hover.
pub fn hint_label(ui: &mut egui::Ui, label: &str, help: &str) -> egui::Response {
    let c = style::colors(ui.ctx());
    let r = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new(label).font(style::bold(14.0)));
            if !help.is_empty() {
                ui.label(RichText::new(icons::HELP).size(12.0).color(c.muted));
            }
        })
        .response;
    if help.is_empty() {
        r
    } else {
        r.on_hover_ui(|ui| {
            ui.set_max_width(360.0);
            ui.label(help);
        })
    }
}

/// Common row: label column, editor, then markers and revert button.
pub fn row(
    app: &mut App,
    ui: &mut egui::Ui,
    path: &[Seg],
    o: &Opts,
    editor: impl FnOnce(&mut App, &mut egui::Ui),
) {
    let c = style::colors(ui.ctx());
    let Some(open) = &app.open else { return };
    let modified = open.doc.is_modified(path);
    let original = if modified {
        open.doc.original_at(path).map(short)
    } else {
        None
    };
    let current = open.doc.root().at(path).cloned();
    let out_of_norm = match (current.as_ref().and_then(Value::as_f64), o.min, o.max) {
        (Some(v), Some(min), Some(max)) => !(min..=max).contains(&v),
        _ => false,
    };
    let expert = app.expert();
    let help = if o.help.is_empty() {
        fields::describe(app.lang, path).help
    } else {
        o.help.clone()
    };
    let frame_fill = if modified {
        c.mark_soft
    } else {
        egui::Color32::TRANSPARENT
    };
    egui::Frame::new()
        .fill(frame_fill)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if modified {
                let r = ui.max_rect();
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        r.left_top() - egui::vec2(8.0, 6.0),
                        egui::vec2(3.0, r.height() + 12.0),
                    ),
                    1.0,
                    c.mark,
                );
            }
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(240.0_f32.min(ui.available_width() * 0.4));
                    hint_label(ui, &o.label, &help);
                    if let (Some(min), Some(max)) = (o.min, o.max) {
                        ui.label(
                            RichText::new(format!("{}–{}", fmt_num(min), fmt_num(max)))
                                .small()
                                .color(c.muted),
                        );
                    }
                    if expert {
                        ui.label(
                            RichText::new(path_to_string(path))
                                .font(style::mono(11.0))
                                .color(c.muted),
                        );
                    }
                });
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        editor(app, ui);
                        if !o.unit.is_empty() {
                            ui.label(RichText::new(&o.unit).color(c.muted));
                        }
                        if modified {
                            ui.label(RichText::new(app.t("c.modified")).small().color(c.mark));
                            if crate::app::small_icon(ui, icons::RESET, &app.t("c.revert"))
                                .clicked()
                            {
                                app.act(Action::RevertValue {
                                    path: path.to_vec(),
                                });
                            }
                        }
                    });
                    if let Some(orig) = original {
                        ui.label(
                            RichText::new(app.tf("c.original", &[("value", orig)]))
                                .small()
                                .color(c.muted),
                        );
                    }
                    if out_of_norm {
                        let text = app.tf(
                            "c.outOfNorm",
                            &[
                                ("min", fmt_num(o.min.unwrap_or(0.0))),
                                ("max", fmt_num(o.max.unwrap_or(0.0))),
                            ],
                        );
                        ui.label(
                            RichText::new(format!("{} {text}", icons::WARN))
                                .small()
                                .color(c.warning),
                        );
                    }
                });
            });
        });
}

pub fn short(v: &Value) -> String {
    match v {
        Value::String(s) => s.text(),
        Value::Bool(b) => b.to_string(),
        Value::Array(a) => format!("[{}]", a.len()),
        Value::Object(o) => format!("{{{}}}", o.len()),
        other => other.to_compact(),
    }
}

pub fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        let s = format!("{}", v as i64);
        group(&s)
    } else {
        format!("{v:.2}")
    }
}

fn group(s: &str) -> String {
    let (sign, digits) = s.strip_prefix('-').map_or(("", s), |d| ("-", d));
    if digits.len() <= 4 {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(ch);
    }
    format!("{sign}{out}")
}

fn buf_key(path: &[Seg]) -> String {
    path_to_string(path)
}

/// Range used by a number box: the norm, widened to include the current
/// value so an out-of-norm value is never clamped just by showing it.
fn drag_range(current: f64, min: Option<f64>, max: Option<f64>, expert: bool) -> (f64, f64) {
    if expert {
        return (f64::NEG_INFINITY, f64::INFINITY);
    }
    (
        min.map_or(f64::NEG_INFINITY, |m| m.min(current)),
        max.map_or(f64::INFINITY, |m| m.max(current)),
    )
}

/// Number box that can be dragged or typed into. The value is kept in a
/// buffer while the user drags or types and is returned once, when the edit
/// ends, so one gesture makes one history step.
pub fn drag_number(
    app: &mut App,
    ui: &mut egui::Ui,
    key: &str,
    current: f64,
    range: (f64, f64),
    int: bool,
) -> Option<f64> {
    let (lo, hi) = range;
    let mut v = app.drags.get(key).copied().unwrap_or(current);
    let speed = if lo.is_finite() && hi.is_finite() {
        (hi - lo) / 300.0
    } else {
        current.abs() / 200.0
    };
    let speed = speed.clamp(if int { 0.05 } else { 0.01 }, 1e6);
    let id = ui.next_auto_id();
    let mut dv = egui::DragValue::new(&mut v)
        .range(lo..=hi)
        .speed(speed)
        .update_while_editing(false);
    dv = if int {
        dv.max_decimals(0)
    } else {
        dv.max_decimals(2)
    };
    let resp = ui.add(dv);
    if resp.dragged() {
        // egui keeps the unclamped drag position; clamp it so dragging back
        // from past the end responds at once.
        ui.data_mut(|d| {
            if let Some(p) = d.get_temp::<f64>(id) {
                d.insert_temp(id, p.clamp(lo, hi));
            }
        });
    }
    if resp.changed() {
        app.drags.insert(key.to_string(), v);
    }
    if resp.dragged() || resp.has_focus() {
        return None;
    }
    let v = app.drags.remove(key)?;
    let v = if int { v.round() } else { v };
    (v != current).then_some(v)
}

/// Number editor bound to a path: drag, arrow keys or click and type.
/// Values too large for a drag box (such as .NET ticks) use a text box.
pub fn num_editor(app: &mut App, ui: &mut egui::Ui, path: &[Seg], o: &Opts) {
    let Some(open) = &app.open else { return };
    let cur = open.doc.root().at(path).cloned();
    let Some(Value::Number(n)) = cur else {
        ui.label(RichText::new(cur.map(|v| short(&v)).unwrap_or_default()).font(style::mono(13.0)));
        return;
    };
    let int = n.is_integer_literal();
    let Some(value) = n.as_str().parse::<f64>().ok().filter(|v| v.abs() < 1e12) else {
        num_text_editor(app, ui, path, 180.0, o.read_only);
        return;
    };
    if o.read_only {
        ui.label(RichText::new(fmt_num(value)).font(style::mono(13.0)));
        return;
    }
    let range = drag_range(value, o.min, o.max, app.expert());
    let key = buf_key(path);
    if let Some(v) = drag_number(app, ui, &key, value, range, int) {
        if int {
            app.act(Action::SetInt {
                path: path.to_vec(),
                value: v as i64,
            });
        } else {
            app.act(Action::SetNumber {
                path: path.to_vec(),
                value: v,
            });
        }
    }
}

/// Text box for numbers: committed on Enter or focus loss.
fn num_text_editor(app: &mut App, ui: &mut egui::Ui, path: &[Seg], width: f32, read_only: bool) {
    let Some(open) = &app.open else { return };
    let cur = open.doc.root().at(path).cloned();
    let Some(Value::Number(n)) = cur else {
        ui.label(RichText::new(cur.map(|v| short(&v)).unwrap_or_default()).font(style::mono(13.0)));
        return;
    };
    let key = buf_key(path);
    let shown = n.as_str().to_string();
    let mut text = app.bufs.get(&key).cloned().unwrap_or_else(|| shown.clone());
    let valid = text.trim().replace(',', ".").parse::<f64>().is_ok();
    let c = style::colors(ui.ctx());
    let mut edit = egui::TextEdit::singleline(&mut text)
        .desired_width(width)
        .font(style::mono(13.0));
    if !valid {
        edit = edit.text_color(c.danger);
    }
    let resp = ui.add_enabled(!read_only, edit);
    if resp.changed() {
        app.bufs.insert(key.clone(), text.clone());
    }
    let commit =
        resp.lost_focus() || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
    if commit {
        app.bufs.remove(&key);
        if text.trim() != shown {
            if let (true, Ok(v)) = (n.is_integer_literal(), text.trim().parse::<i64>()) {
                app.act(Action::SetInt {
                    path: path.to_vec(),
                    value: v,
                });
            } else if let Ok(v) = text.trim().replace(',', ".").parse::<f64>() {
                app.act(Action::SetNumber {
                    path: path.to_vec(),
                    value: v,
                });
            } else {
                let msg = app.t("aerr.wrongType");
                app.toast_error(msg);
            }
        }
    } else if !resp.has_focus() && app.bufs.get(&key).is_some_and(|b| *b == shown) {
        app.bufs.remove(&key);
    }
}

pub fn text_editor(app: &mut App, ui: &mut egui::Ui, path: &[Seg], width: f32) {
    let Some(open) = &app.open else { return };
    let Some(Value::String(s)) = open.doc.root().at(path).cloned() else {
        return;
    };
    let key = buf_key(path);
    let shown = s.text();
    let mut text = app.bufs.get(&key).cloned().unwrap_or_else(|| shown.clone());
    let resp = ui.add(egui::TextEdit::singleline(&mut text).desired_width(width));
    if resp.changed() {
        app.bufs.insert(key.clone(), text.clone());
    }
    if resp.lost_focus() || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
        app.bufs.remove(&key);
        if text != shown {
            app.act(Action::SetText {
                path: path.to_vec(),
                value: text,
            });
        }
    }
}

pub fn bool_editor(app: &mut App, ui: &mut egui::Ui, path: &[Seg]) {
    let Some(open) = &app.open else { return };
    let Some(Value::Bool(mut b)) = open.doc.root().at(path).cloned() else {
        return;
    };
    let text = if b {
        app.t("c.yesno.true")
    } else {
        app.t("c.yesno.false")
    };
    if ui.checkbox(&mut b, text).changed() {
        app.act(Action::SetBool {
            path: path.to_vec(),
            value: b,
        });
    }
}

pub fn num_field(app: &mut App, ui: &mut egui::Ui, path: &[Seg], o: Opts) {
    let o2 = o.clone();
    row(app, ui, path, &o, |app, ui| num_editor(app, ui, path, &o2));
}

pub fn text_field(app: &mut App, ui: &mut egui::Ui, path: &[Seg], o: Opts) {
    row(app, ui, path, &o, |app, ui| {
        text_editor(app, ui, path, 260.0)
    });
}

pub fn bool_field(app: &mut App, ui: &mut egui::Ui, path: &[Seg], o: Opts) {
    row(app, ui, path, &o, |app, ui| bool_editor(app, ui, path));
}

/// Choice among fixed text values.
pub fn choice_field(
    app: &mut App,
    ui: &mut egui::Ui,
    path: &[Seg],
    o: Opts,
    choices: &[(String, String)],
) {
    row(app, ui, path, &o, |app, ui| {
        let Some(open) = &app.open else { return };
        let cur = open
            .doc
            .root()
            .at(path)
            .and_then(Value::as_text)
            .unwrap_or_default();
        let shown = choices
            .iter()
            .find(|(v, _)| *v == cur)
            .map_or_else(|| cur.clone(), |(_, l)| l.clone());
        let mut pick = None;
        egui::ComboBox::from_id_salt(path_to_string(path))
            .selected_text(shown)
            .width(220.0)
            .show_ui(ui, |ui| {
                for (v, l) in choices {
                    if ui.selectable_label(*v == cur, l).clicked() {
                        pick = Some(v.clone());
                    }
                }
            });
        if let Some(v) = pick {
            app.act(Action::SetText {
                path: path.to_vec(),
                value: v,
            });
        }
    });
}

/// Choice among integer values.
pub fn int_choice_field(
    app: &mut App,
    ui: &mut egui::Ui,
    path: &[Seg],
    o: Opts,
    choices: &[(i64, String)],
) {
    row(app, ui, path, &o, |app, ui| {
        let Some(open) = &app.open else { return };
        let cur = open
            .doc
            .root()
            .at(path)
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        let shown = choices
            .iter()
            .find(|(v, _)| *v == cur)
            .map_or_else(|| cur.to_string(), |(_, l)| l.clone());
        let mut pick = None;
        egui::ComboBox::from_id_salt(path_to_string(path))
            .selected_text(shown)
            .width(220.0)
            .show_ui(ui, |ui| {
                for (v, l) in choices {
                    if ui.selectable_label(*v == cur, l).clicked() {
                        pick = Some(*v);
                    }
                }
            });
        if let Some(v) = pick {
            app.act(Action::SetNumber {
                path: path.to_vec(),
                value: v as f64,
            });
        }
    });
}

/// .NET ticks shown as date and time.
pub fn ticks_field(app: &mut App, ui: &mut egui::Ui, path: &[Seg], o: Opts) {
    row(app, ui, path, &o, |app, ui| {
        let Some(open) = &app.open else { return };
        let Some(t) = open.doc.root().at(path).and_then(Value::as_i64) else {
            return;
        };
        let key = buf_key(path);
        let shown = shelter_core::ticks::format(t).unwrap_or_else(|| t.to_string());
        let mut text = app.bufs.get(&key).cloned().unwrap_or_else(|| shown.clone());
        let c = style::colors(ui.ctx());
        let valid = shelter_core::ticks::parse(&text, t).is_some();
        let mut edit = egui::TextEdit::singleline(&mut text)
            .desired_width(180.0)
            .font(style::mono(13.0));
        if !valid {
            edit = edit.text_color(c.danger);
        }
        let resp = ui.add(edit).on_hover_text(app.t("time.local"));
        if resp.changed() {
            app.bufs.insert(key.clone(), text.clone());
        }
        if resp.lost_focus() || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            app.bufs.remove(&key);
            if text != shown {
                match shelter_core::ticks::parse(&text, t) {
                    Some(v) => {
                        app.act(Action::SetInt {
                            path: path.to_vec(),
                            value: v,
                        });
                    }
                    None => {
                        let m = app.t("time.invalid");
                        app.toast_error(m);
                    }
                }
            }
        }
        if app.expert() {
            ui.label(
                RichText::new(t.to_string())
                    .font(style::mono(11.0))
                    .color(c.muted),
            );
        }
    });
}

/// Generic editor for any object: scalar fields as rows, nested values collapsible.
pub fn object_form(app: &mut App, ui: &mut egui::Ui, path: &[Seg], depth: usize) {
    let Some(open) = &app.open else { return };
    let Some(v) = open.doc.root().at(path).cloned() else {
        return;
    };
    match &v {
        Value::Object(o) => {
            for (k, child) in &o.0 {
                let key = k.text();
                let mut p = path.to_vec();
                p.push(Seg::Key(key.clone()));
                value_row(app, ui, &p, &key, child, depth);
            }
            if o.is_empty() {
                ui.label(RichText::new(app.t("c.empty")).color(style::colors(ui.ctx()).muted));
            }
        }
        Value::Array(a) => {
            for (i, child) in a.iter().enumerate().take(500) {
                let mut p = path.to_vec();
                p.push(Seg::Index(i));
                value_row(app, ui, &p, &format!("#{}", i + 1), child, depth);
            }
            if a.is_empty() {
                ui.label(RichText::new(app.t("c.empty")).color(style::colors(ui.ctx()).muted));
            }
        }
        _ => {}
    }
}

fn value_row(app: &mut App, ui: &mut egui::Ui, p: &[Seg], key: &str, child: &Value, depth: usize) {
    let opts = if key.starts_with('#') {
        Opts::text(key.to_string())
    } else {
        Opts::at(app, p)
    };
    match child {
        Value::Number(_) => num_field(app, ui, p, opts),
        Value::String(_) => text_field(app, ui, p, opts),
        Value::Bool(_) => bool_field(app, ui, p, opts),
        Value::Null => {
            row(app, ui, p, &opts, |_, ui| {
                ui.label(RichText::new("null").font(style::mono(13.0)));
            });
        }
        Value::Array(a)
            if a.iter()
                .all(|x| !matches!(x, Value::Object(_) | Value::Array(_)))
                && a.len() <= 12 =>
        {
            row(app, ui, p, &opts, |_, ui| {
                let s: Vec<String> = a.iter().map(short).collect();
                ui.label(
                    RichText::new(if s.is_empty() {
                        "[]".to_string()
                    } else {
                        s.join(", ")
                    })
                    .font(style::mono(13.0)),
                );
            });
        }
        Value::Array(_) | Value::Object(_) => {
            let count = match child {
                Value::Array(a) => format!(" [{}]", a.len()),
                _ => String::new(),
            };
            let title = format!("{}{count}", opts.label);
            let help = opts.help.clone();
            let r = egui::CollapsingHeader::new(title)
                .id_salt(path_to_string(p))
                .default_open(false)
                .show(ui, |ui| {
                    if depth < 12 {
                        object_form(app, ui, p, depth + 1);
                    }
                });
            if !help.is_empty() {
                r.header_response.on_hover_text(help);
            }
        }
    }
}

/// Minimal Markdown for the bundled documents: headings, lists, bold.
pub fn markdown(ui: &mut egui::Ui, text: &str) {
    let c = style::colors(ui.ctx());
    let mut para = String::new();
    let flush = |ui: &mut egui::Ui, para: &mut String| {
        if !para.trim().is_empty() {
            ui.label(para.trim().replace("**", ""));
            ui.add_space(4.0);
        }
        para.clear();
    };
    for line in text.lines() {
        let l = line.trim_end();
        if let Some(h) = l.strip_prefix("# ") {
            flush(ui, &mut para);
            ui.label(RichText::new(h).font(style::bold(18.0)));
        } else if let Some(h) = l.strip_prefix("## ") {
            flush(ui, &mut para);
            ui.add_space(4.0);
            ui.label(RichText::new(h).font(style::bold(15.0)));
        } else if let Some(item) = l.strip_prefix("- ") {
            flush(ui, &mut para);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("•").color(c.accent));
                ui.label(item.replace("**", ""));
            });
        } else if l.is_empty() {
            flush(ui, &mut para);
        } else {
            para.push_str(l);
            para.push(' ');
        }
    }
    flush(ui, &mut para);
}
