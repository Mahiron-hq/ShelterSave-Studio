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

//! Design tokens (`design/tokens.json`) applied to egui.

use std::sync::{Arc, OnceLock};

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke,
    TextStyle, Theme, Visuals,
};
use serde::Deserialize;

const TOKENS: &str = include_str!("../../../design/tokens.json");

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub bg: String,
    pub surface: String,
    pub surface_alt: String,
    pub border: String,
    pub text: String,
    pub text_muted: String,
    pub accent: String,
    pub accent_soft: String,
    pub on_accent: String,
    pub mark: String,
    pub mark_soft: String,
    pub warning: String,
    pub warning_soft: String,
    pub danger: String,
    pub danger_soft: String,
    pub success: String,
    pub success_soft: String,
    pub focus: String,
}

#[derive(Deserialize)]
struct Tokens {
    light: Palette,
    dark: Palette,
}

#[derive(Clone, Copy)]
pub struct Colors {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub mark: Color32,
    pub mark_soft: Color32,
    pub warning: Color32,
    pub warning_soft: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
    pub success: Color32,
    pub success_soft: Color32,
    pub focus: Color32,
}

fn hex(s: &str) -> Color32 {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl From<&Palette> for Colors {
    fn from(p: &Palette) -> Self {
        Colors {
            bg: hex(&p.bg),
            surface: hex(&p.surface),
            surface_alt: hex(&p.surface_alt),
            border: hex(&p.border),
            text: hex(&p.text),
            muted: hex(&p.text_muted),
            accent: hex(&p.accent),
            accent_soft: hex(&p.accent_soft),
            on_accent: hex(&p.on_accent),
            mark: hex(&p.mark),
            mark_soft: hex(&p.mark_soft),
            warning: hex(&p.warning),
            warning_soft: hex(&p.warning_soft),
            danger: hex(&p.danger),
            danger_soft: hex(&p.danger_soft),
            success: hex(&p.success),
            success_soft: hex(&p.success_soft),
            focus: hex(&p.focus),
        }
    }
}

fn tokens() -> &'static (Colors, Colors) {
    static T: OnceLock<(Colors, Colors)> = OnceLock::new();
    T.get_or_init(|| {
        let t: Tokens = serde_json::from_str(TOKENS).expect("tokens.json is valid");
        ((&t.light).into(), (&t.dark).into())
    })
}

pub fn colors(ctx: &egui::Context) -> Colors {
    let (light, dark) = tokens();
    if ctx.global_style().visuals.dark_mode {
        *dark
    } else {
        *light
    }
}

pub const MONO: &str = "mono";
pub const ICONS: &str = "icons";

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert(
        "inter".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "inter-semibold".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jetbrains".into(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );
    let mut icon = FontData::from_static(include_bytes!("../../../assets/icons/lucide-subset.ttf"));
    icon.tweak.y_offset_factor = 0.08;
    fonts.font_data.insert("lucide".into(), Arc::new(icon));
    fonts.families.insert(
        FontFamily::Proportional,
        vec!["lucide".into(), "inter".into(), "jetbrains".into()],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec!["lucide".into(), "jetbrains".into(), "inter".into()],
    );
    fonts.families.insert(
        FontFamily::Name("bold".into()),
        vec!["lucide".into(), "inter-semibold".into(), "inter".into()],
    );
    fonts
        .families
        .insert(FontFamily::Name(ICONS.into()), vec!["lucide".into()]);
    fonts.families.insert(
        FontFamily::Name(MONO.into()),
        vec!["lucide".into(), "jetbrains".into(), "inter".into()],
    );
    ctx.set_fonts(fonts);
}

fn visuals(c: &Colors, dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let r = CornerRadius::same(6);
    v.panel_fill = c.bg;
    v.window_fill = c.surface;
    v.extreme_bg_color = c.surface;
    v.faint_bg_color = c.surface_alt;
    v.code_bg_color = c.surface_alt;
    v.window_stroke = Stroke::new(1.0, c.border);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.hyperlink_color = c.accent;
    v.warn_fg_color = c.warning;
    v.error_fg_color = c.danger;
    v.override_text_color = Some(c.text);
    v.selection.bg_fill = c.accent_soft;
    v.selection.stroke = Stroke::new(1.0, c.accent);
    v.text_cursor.stroke = Stroke::new(2.0, c.accent);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c.surface;
    w.noninteractive.weak_bg_fill = c.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, c.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, c.text);
    w.noninteractive.corner_radius = r;
    w.inactive.bg_fill = c.surface;
    w.inactive.weak_bg_fill = c.surface_alt;
    w.inactive.bg_stroke = Stroke::new(1.0, c.border);
    w.inactive.fg_stroke = Stroke::new(1.0, c.text);
    w.inactive.corner_radius = r;
    w.hovered.bg_fill = c.accent_soft;
    w.hovered.weak_bg_fill = c.accent_soft;
    w.hovered.bg_stroke = Stroke::new(1.0, c.accent);
    w.hovered.fg_stroke = Stroke::new(1.0, c.text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;
    w.active.bg_fill = c.accent_soft;
    w.active.weak_bg_fill = c.accent_soft;
    w.active.bg_stroke = Stroke::new(2.0, c.focus);
    w.active.fg_stroke = Stroke::new(1.0, c.text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;
    w.open.bg_fill = c.surface_alt;
    w.open.weak_bg_fill = c.surface_alt;
    w.open.bg_stroke = Stroke::new(1.0, c.accent);
    w.open.corner_radius = r;
    v
}

pub fn apply(ctx: &egui::Context, reduce_motion: bool) {
    let (light, dark) = tokens();
    ctx.set_visuals_of(Theme::Light, visuals(light, false));
    ctx.set_visuals_of(Theme::Dark, visuals(dark, true));
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = 28.0;
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.window_margin = egui::Margin::same(16);
        s.animation_time = if reduce_motion { 0.0 } else { 0.15 };
        s.text_styles = [
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Heading,
                FontId::new(20.0, FontFamily::Name("bold".into())),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.0, FontFamily::Name(MONO.into())),
            ),
        ]
        .into();
    });
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MONO.into()))
}

/// Minimal Markdown for the bundled documents: headings, lists, bold, code.
pub fn markdown(ui: &mut egui::Ui, text: &str) {
    let c = colors(ui.ctx());
    let mut para = String::new();
    let flush = |ui: &mut egui::Ui, para: &mut String| {
        if !para.trim().is_empty() {
            ui.add(egui::Label::new(inline(ui, para.trim())).wrap());
            ui.add_space(4.0);
        }
        para.clear();
    };
    for line in text.lines() {
        let l = line.trim_end();
        let numbered = l
            .split_once(". ")
            .filter(|(n, _)| !n.is_empty() && n.chars().all(|ch| ch.is_ascii_digit()));
        if let Some(h) = l.strip_prefix("# ") {
            flush(ui, &mut para);
            ui.label(RichText::new(h).font(bold(18.0)));
        } else if let Some(h) = l.strip_prefix("## ") {
            flush(ui, &mut para);
            ui.add_space(8.0);
            ui.label(RichText::new(h).font(bold(15.0)));
        } else if let Some(h) = l.strip_prefix("### ") {
            flush(ui, &mut para);
            ui.label(RichText::new(h).font(bold(14.0)));
        } else if let Some(item) = l.strip_prefix("- ") {
            flush(ui, &mut para);
            ui.horizontal_top(|ui| {
                ui.label(RichText::new("•").color(c.accent));
                ui.add(egui::Label::new(inline(ui, item)).wrap());
            });
        } else if let Some((n, item)) = numbered {
            flush(ui, &mut para);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(20.0, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(20.0);
                        ui.label(RichText::new(format!("{n}.")).color(c.accent));
                    },
                );
                ui.add(egui::Label::new(inline(ui, item)).wrap());
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

/// One wrapped paragraph with **bold** and `code` runs.
fn inline(ui: &egui::Ui, text: &str) -> egui::text::LayoutJob {
    let c = colors(ui.ctx());
    let size = 14.0;
    let mut job = egui::text::LayoutJob::default();
    let (mut strong, mut code) = (false, false);
    let mut rest = text;
    while !rest.is_empty() {
        let next = [rest.find("**"), rest.find('`')]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(rest.len());
        let (run, tail) = rest.split_at(next);
        if !run.is_empty() {
            let font = if code {
                mono(size - 1.0)
            } else if strong {
                bold(size)
            } else {
                FontId::proportional(size)
            };
            job.append(
                run,
                0.0,
                egui::TextFormat {
                    font_id: font,
                    color: c.text,
                    background: if code {
                        c.surface_alt
                    } else {
                        Color32::TRANSPARENT
                    },
                    ..Default::default()
                },
            );
        }
        if let Some(t) = tail.strip_prefix("**") {
            strong = !strong;
            rest = t;
        } else if let Some(t) = tail.strip_prefix('`') {
            code = !code;
            rest = t;
        } else {
            rest = tail;
        }
    }
    job
}
