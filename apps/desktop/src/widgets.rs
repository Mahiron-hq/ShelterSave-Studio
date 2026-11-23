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
    /// The value is picked from a list, so no range is shown.
    pub choice: bool,
    /// The editor shows a value computed from the stored one, so the
    /// stored original is not shown.
    pub derived: bool,
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

    pub fn with_label(mut self, label: String) -> Opts {
        self.label = label;
        self
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

/// Bold label that wraps; its description appears as a tooltip after a
/// short hover.
pub fn hint_label(ui: &mut egui::Ui, label: &str, help: &str) -> egui::Response {
    let c = style::colors(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    job.append(
        label,
        0.0,
        egui::TextFormat {
            font_id: style::bold(14.0),
            color: c.text,
            ..Default::default()
        },
    );
    if !help.is_empty() {
        job.append(
            &format!(" {}", icons::HELP),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(12.0),
                color: c.muted,
                valign: egui::Align::Center,
                ..Default::default()
            },
        );
    }
    let r = ui.add(egui::Label::new(job).wrap().sense(egui::Sense::hover()));
    if help.is_empty() {
        r
    } else {
        r.on_hover_ui(|ui| {
            ui.set_max_width(380.0);
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
    // A value that did not exist in the opened file has nothing to go back to.
    let modified = open.doc.is_modified(path) && open.doc.original_at(path).is_some();
    let is_color = matches!(path.last(), Some(Seg::Key(k)) if k.ends_with("Color"));
    let original = if modified && !o.derived {
        open.doc.original_at(path).map(|v| match v.as_f64() {
            Some(n) if is_color => format!("#{:06X}", (n as u32) & 0x00ff_ffff),
            Some(n) if matches!(v, Value::Number(_)) => fmt_num(n),
            _ => short(v),
        })
    } else {
        None
    };
    let current = open.doc.root().at(path).cloned();
    let out_of_norm = match (current.as_ref().and_then(Value::as_f64), o.min, o.max) {
        (Some(v), Some(min), Some(max)) => !(min..=max).contains(&v),
        _ => false,
    };
    let expert = app.expert();
    let mut help = if o.help.is_empty() {
        fields::describe(app.lang, path).help
    } else {
        o.help.clone()
    };
    if let (Some(min), Some(max), false) = (o.min, o.max, o.choice) {
        if !help.is_empty() {
            help.push_str("\n\n");
        }
        help.push_str(&app.tf("c.range", &[("min", fmt_num(min)), ("max", fmt_num(max))]));
    }
    let frame_fill = if modified {
        c.mark_soft
    } else {
        egui::Color32::TRANSPARENT
    };
    let frame = egui::Frame::new()
        .fill(frame_fill)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let label = |ui: &mut egui::Ui| {
                hint_label(ui, &o.label, &help);
                if expert {
                    ui.add(
                        egui::Label::new(
                            RichText::new(path_to_string(path))
                                .font(style::mono(11.0))
                                .color(c.muted),
                        )
                        .wrap(),
                    );
                }
            };
            let body = |app: &mut App, ui: &mut egui::Ui| {
                ui.horizontal_wrapped(|ui| {
                    editor(app, ui);
                    if !o.unit.is_empty() {
                        unit_label(ui, &o.unit);
                    }
                    if modified {
                        ui.label(RichText::new(app.t("c.modified")).small().color(c.mark));
                        if crate::app::small_icon(ui, icons::RESET, &app.t("c.revert")).clicked() {
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
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("{} {text}", icons::WARN))
                                .small()
                                .color(c.warning),
                        )
                        .wrap(),
                    );
                }
            };
            // Narrow places (side panels) put the label above the editor.
            if ui.available_width() < 460.0 {
                label(ui);
                ui.add_space(2.0);
                body(app, ui);
            } else {
                ui.horizontal_top(|ui| {
                    let label_w = 240.0_f32.min(ui.available_width() * 0.4);
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_w, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(label_w);
                            label(ui);
                        },
                    );
                    ui.vertical(|ui| body(app, ui));
                });
            }
        });
    if modified {
        let r = frame.response.rect;
        ui.painter().rect_filled(
            egui::Rect::from_min_size(r.left_top(), egui::vec2(3.0, r.height())),
            1.0,
            c.mark,
        );
    }
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

/// Number without grouping: whole values without decimals, others with
/// up to two.
fn plain_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
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

/// Number box that can be dragged or typed into, see [`pick_number`].
pub fn drag_number(
    app: &mut App,
    ui: &mut egui::Ui,
    key: &str,
    current: f64,
    range: (f64, f64),
    int: bool,
) -> Option<f64> {
    pick_number(app, ui, key, current, range, None, int)
}

/// Decides whether the mouse wheel over a number changes it or scrolls the
/// page: a value changes only after the pointer was moved onto it, so a page
/// scroll that brings a field under a resting pointer keeps scrolling.
#[derive(Default)]
pub struct WheelState {
    last_move: f64,
    last_page_scroll: f64,
    taken: bool,
    acc: f32,
}

impl WheelState {
    pub fn begin_frame(&mut self, ctx: &egui::Context) {
        let (moved, now) = ctx.input(|i| (i.pointer.delta() != egui::Vec2::ZERO, i.time));
        if moved {
            self.last_move = now;
        }
        self.taken = false;
    }

    pub fn end_frame(&mut self, ctx: &egui::Context) {
        let (scroll, now) = ctx.input(|i| (wheel_notches(i) != 0.0, i.time));
        if scroll && !self.taken {
            self.last_page_scroll = now;
            self.acc = 0.0;
        }
    }

    /// Steps for a hovered value widget this frame, one per wheel notch;
    /// takes the wheel from the page when it applies.
    fn steps(&mut self, ui: &egui::Ui, hovered: bool) -> f64 {
        if !hovered || self.last_move <= self.last_page_scroll {
            return 0.0;
        }
        let (delta, smooth) = ui.input(|i| (wheel_notches(i), i.smooth_scroll_delta));
        if delta == 0.0 && smooth == egui::Vec2::ZERO {
            return 0.0;
        }
        self.taken = true;
        ui.ctx()
            .input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
        self.acc += delta;
        let notches = self.acc.trunc();
        if notches == 0.0 {
            return 0.0;
        }
        self.acc -= notches;
        f64::from(notches)
    }
}

/// Wheel notches this frame; touchpads count 40 points as one notch.
fn wheel_notches(i: &egui::InputState) -> f32 {
    i.raw
        .events
        .iter()
        .map(|e| match e {
            egui::Event::MouseWheel { unit, delta, .. } => {
                // Shift may turn vertical wheel motion into horizontal.
                let d = if delta.y != 0.0 { delta.y } else { delta.x };
                match unit {
                    egui::MouseWheelUnit::Point => d / 40.0,
                    _ => d,
                }
            }
            _ => 0.0,
        })
        .sum()
}

/// Largest change per drag step.
const MAX_STEP: f64 = 60.0;
/// Pointer travel for one drag step.
const STEP_PX: f64 = 6.0;
/// Pointer speed (points per second) above which dragging is fast.
const FAST_SPEED: f32 = 700.0;

/// Fast step: 60, but at most a fiftieth of the range and at least one.
fn fast_step(span: f64) -> f64 {
    if span.is_finite() {
        (span / 50.0).floor().clamp(1.0, MAX_STEP)
    } else {
        MAX_STEP
    }
}

/// Units per point of mouse travel: slow moves step by one, fast moves by `fast_step`.
fn drag_rate(speed: f32, fast: bool, span: f64) -> f64 {
    let step = if fast || speed > FAST_SPEED {
        fast_step(span)
    } else {
        1.0
    };
    step / STEP_PX
}

/// Number box plus an optional slider over `slider`. Dragging, the mouse
/// wheel and the slider change the value in whole steps; a click opens it
/// for typing. The value is kept in a buffer during a gesture and returned
/// once, when it ends, so one gesture makes one history step.
pub fn pick_number(
    app: &mut App,
    ui: &mut egui::Ui,
    key: &str,
    current: f64,
    range: (f64, f64),
    slider: Option<(f64, f64)>,
    int: bool,
) -> Option<f64> {
    let (lo, hi) = range;
    let mut v = app.drags.get(key).copied().unwrap_or(current);
    let mut busy = false;
    let now = ui.input(|i| i.time);
    let wheel_id = egui::Id::unique(("num_wheel", key));
    let span = if hi.is_finite() && lo.is_finite() {
        hi - lo
    } else {
        f64::INFINITY
    };
    let wheel = |app: &mut App, ui: &egui::Ui, v: &mut f64, hovered: bool| {
        let steps = app.wheel.steps(ui, hovered);
        if steps != 0.0 {
            *v = (*v + steps).clamp(lo, hi);
            app.drags.insert(key.to_string(), *v);
            ui.data_mut(|d| d.insert_temp(wheel_id, now));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(600));
        }
    };
    if let Some((s_lo, s_hi)) = slider.filter(|(a, b)| a.is_finite() && b.is_finite() && b > a) {
        // Big ranges use a cubic scale so small values stay easy to pick
        // and the whole track stays usable (no dead zone at zero).
        let s_span = s_hi - s_lo;
        let k = if s_span > 1000.0 { 3.0 } else { 1.0 };
        let mut t = ((v.clamp(s_lo, s_hi) - s_lo) / s_span).powf(1.0 / k);
        ui.spacing_mut().slider_width = (ui.available_width() - 240.0).clamp(80.0, 200.0);
        let rail = style::colors(ui.ctx()).border;
        let r = ui
            .scope(|ui| {
                ui.visuals_mut().widgets.inactive.bg_fill = rail;
                ui.add(
                    egui::Slider::new(&mut t, 0.0..=1.0)
                        .show_value(false)
                        .trailing_fill(true),
                )
            })
            .inner;
        if r.changed() {
            let raw = s_lo + s_span * t.powf(k);
            // Whole steps from the starting value, so a fraction is kept.
            v = (current + (raw - current).round()).clamp(lo, hi);
            app.drags.insert(key.to_string(), v);
        }
        busy |= r.dragged();
        wheel(app, ui, &mut v, r.hovered() && !r.dragged());
    }
    let r = number_box(app, ui, key, &mut v, (lo, hi), int, span);
    busy |= r.busy;
    wheel(app, ui, &mut v, r.hovered);
    let wheel_at = ui.data(|d| d.get_temp::<f64>(wheel_id));
    if wheel_at.is_some_and(|t| now - t < 0.5) {
        busy = true;
    } else if wheel_at.is_some() {
        ui.data_mut(|d| d.remove::<f64>(wheel_id));
    }
    if busy {
        return None;
    }
    let v = app.drags.remove(key)?;
    let v = if int { v.round() } else { v };
    (v != current).then_some(v)
}

/// Unit or note after a value; moves to the next line as a whole.
pub fn unit_label(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(RichText::new(text).color(style::colors(ui.ctx()).muted))
            .wrap_mode(egui::TextWrapMode::Extend),
    );
}

struct BoxResult {
    busy: bool,
    hovered: bool,
}

fn num_text(v: f64, int: bool) -> String {
    if int {
        format!("{}", v.round() as i64)
    } else {
        plain_num(v)
    }
}

/// Value box: drag sideways to change in whole steps (faster moves make
/// bigger steps), click or Enter to type, arrow keys for one step.
fn number_box(
    app: &mut App,
    ui: &mut egui::Ui,
    key: &str,
    v: &mut f64,
    (lo, hi): (f64, f64),
    int: bool,
    span: f64,
) -> BoxResult {
    let edit_key = format!("{key}#typing");
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley_w = ui
        .painter()
        .layout_no_wrap(num_text(*v, int), font.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let width = (galley_w + 24.0).clamp(64.0, 160.0);
    let height = ui.spacing().interact_size.y;
    if let Some(mut text) = app.bufs.get(&edit_key).cloned() {
        let id = egui::Id::unique(("num_typing", key));
        let resp = ui.add(
            egui::TextEdit::singleline(&mut text)
                .id(id)
                .desired_width(width)
                .align(egui::Align2::CENTER_CENTER),
        );
        let started = ui.data(|d| d.get_temp::<bool>(id)).is_none();
        if started {
            resp.request_focus();
            ui.data_mut(|d| d.insert_temp(id, true));
        }
        let (enter, esc, up, down) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        let clean: String = text
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '\u{202f}')
            .collect();
        let parsed = clean.replace(',', ".").parse::<f64>().ok();
        if (up || down) && resp.has_focus() {
            let base = parsed.unwrap_or(*v);
            text = num_text((base + if up { 1.0 } else { -1.0 }).clamp(lo, hi), int);
        }
        let done = enter || esc || (!started && !resp.has_focus());
        if done {
            app.bufs.remove(&edit_key);
            ui.data_mut(|d| d.remove::<bool>(id));
            if !esc {
                if let Some(n) = parsed {
                    let n = if int {
                        n.round()
                    } else {
                        (n * 100.0).round() / 100.0
                    };
                    *v = n.clamp(lo, hi);
                    app.drags.insert(key.to_string(), *v);
                }
            }
            return BoxResult {
                busy: false,
                hovered: false,
            };
        }
        app.bufs.insert(edit_key, text);
        return BoxResult {
            busy: true,
            hovered: false,
        };
    }
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
    let resp = resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
    let start_id = egui::Id::unique(("num_drag", key));
    if resp.drag_started() {
        ui.data_mut(|d| d.insert_temp(start_id, (*v, 0.0f64)));
    }
    if resp.dragged() {
        let (dx, speed, shift) = ui.input(|i| {
            (
                i.pointer.delta().x,
                i.pointer.velocity().x.abs(),
                i.modifiers.shift,
            )
        });
        if let Some((start, acc)) = ui.data(|d| d.get_temp::<(f64, f64)>(start_id)) {
            // Clamp the travel too, so moving back from past an end responds at once.
            let acc =
                (acc + f64::from(dx) * drag_rate(speed, shift, span)).clamp(lo - start, hi - start);
            ui.data_mut(|d| d.insert_temp(start_id, (start, acc)));
            *v = (start + acc.trunc()).clamp(lo, hi);
            app.drags.insert(key.to_string(), *v);
        }
    }
    if resp.drag_stopped() {
        ui.data_mut(|d| d.remove::<(f64, f64)>(start_id));
    }
    if resp.has_focus() {
        let step = ui.input(|i| {
            let up = i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowRight);
            let down = i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowLeft);
            f64::from(i32::from(up) - i32::from(down))
        });
        if step != 0.0 {
            *v = (*v + step).clamp(lo, hi);
            app.drags.insert(key.to_string(), *v);
        }
    }
    if resp.clicked() || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
        app.bufs.insert(edit_key, num_text(*v, int));
    }
    if ui.is_rect_visible(rect) {
        let vis = ui.style().interact(&resp);
        ui.painter().rect(
            rect,
            vis.corner_radius,
            ui.visuals().extreme_bg_color,
            vis.bg_stroke,
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            num_text(*v, int),
            font,
            vis.text_color(),
        );
    }
    let enabled = ui.is_enabled();
    let val = *v;
    resp.widget_info(|| egui::WidgetInfo::slider(enabled, val, ""));
    BoxResult {
        busy: resp.dragged(),
        hovered: resp.hovered() && !resp.dragged(),
    }
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
    let slider = match (o.min, o.max) {
        (Some(a), Some(b)) => Some((a.min(value), b.max(value))),
        _ => None,
    };
    let key = buf_key(path);
    if let Some(v) = pick_number(app, ui, &key, value, range, slider, int) {
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

/// Yes/no stored as 0 or 1.
pub fn int_flag_field(app: &mut App, ui: &mut egui::Ui, path: &[Seg], o: Opts) {
    row(app, ui, path, &o, |app, ui| {
        let Some(open) = &app.open else { return };
        let mut b = open.doc.root().at(path).and_then(Value::as_i64) == Some(1);
        let text = app.t(if b { "c.yesno.true" } else { "c.yesno.false" });
        if ui.checkbox(&mut b, text).changed() {
            app.act(Action::SetInt {
                path: path.to_vec(),
                value: i64::from(b),
            });
        }
    });
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
    let o = Opts { choice: true, ..o };
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
            .width(ui.available_width().min(220.0))
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
    let o = Opts { choice: true, ..o };
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
            .width(ui.available_width().min(220.0))
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
        Value::Number(n) if fields::is_int_flag(key) && matches!(n.as_str(), "0" | "1") => {
            int_flag_field(app, ui, p, opts);
        }
        Value::Number(_) => num_field(app, ui, p, opts),
        Value::String(_) => text_field(app, ui, p, opts),
        Value::Bool(_) => bool_field(app, ui, p, opts),
        Value::Null => {
            let empty = app.t("c.empty");
            row(app, ui, p, &opts, |_, ui| {
                ui.label(RichText::new(empty).color(style::colors(ui.ctx()).muted));
            });
        }
        Value::Array(a)
            if a.iter()
                .all(|x| !matches!(x, Value::Object(_) | Value::Array(_)))
                && a.len() <= 12 =>
        {
            let empty = app.t("c.empty");
            row(app, ui, p, &opts, |_, ui| {
                let s: Vec<String> = a.iter().map(short).collect();
                if s.is_empty() {
                    ui.label(RichText::new(empty).color(style::colors(ui.ctx()).muted));
                } else {
                    ui.add(
                        egui::Label::new(RichText::new(s.join(", ")).font(style::mono(13.0)))
                            .wrap(),
                    );
                }
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

pub use crate::style::markdown;
