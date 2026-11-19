// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Lekkie, reużywalne komponenty UI oparte na `Palette` i tokenach: odznaki stanu,
//! warianty przycisków (primary / secondary / danger) oraz nagłówki sekcji.

use eframe::egui::{
    self, Button, Color32, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind,
    TextStyle, Ui, WidgetInfo, WidgetType, vec2,
};

use super::contrast::contrast_ratio;
use super::palette::Palette;
use super::tokens::{RADIUS_MD, RADIUS_SM, SPACE_MD, SPACE_SM, TEXT_CAPTION, TEXT_SUBHEADING};
use crate::gui::state::ToastSeverity;

/// Rodzaj odznaki stanu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Ok,
    Warn,
    Err,
    Idle,
    /// Operacja w toku — odznaka pokazuje spinner zamiast ikony.
    Busy,
}

impl StatusKind {
    /// Kolor stanu z centralnej palety.
    #[must_use]
    pub fn color(self, palette: &Palette) -> Color32 {
        match self {
            Self::Ok => palette.success,
            Self::Warn => palette.warning,
            Self::Err => palette.danger,
            Self::Idle => palette.text_muted,
            Self::Busy => palette.accent,
        }
    }

    /// Ikona tekstowa (stan nie jest sygnalizowany wyłącznie kolorem).
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            Self::Ok => "✔",
            Self::Warn => "⚠",
            Self::Err => "✖",
            Self::Idle => "○",
            Self::Busy => "",
        }
    }
}

/// Kolor akcentu (obramowania i ikony) toastu danej ważności.
#[must_use]
pub fn toast_accent(palette: &Palette, severity: ToastSeverity) -> Color32 {
    match severity {
        ToastSeverity::Info => palette.info,
        ToastSeverity::Success => palette.success,
        ToastSeverity::Warning => palette.warning,
        ToastSeverity::Error => palette.danger,
    }
}

/// Czytelny kolor tekstu (biały lub czarny) na zadanym tle.
#[must_use]
pub fn readable_on(bg: Color32) -> Color32 {
    if contrast_ratio(Color32::WHITE, bg) >= contrast_ratio(Color32::BLACK, bg) {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

/// Odznaka stanu: ikona (lub spinner) + tekst w kolorze stanu.
pub fn status_badge(
    ui: &mut Ui,
    palette: &Palette,
    kind: StatusKind,
    text: impl Into<String>,
) -> Response {
    let color = kind.color(palette);
    let text = text.into();
    ui.horizontal(|ui| {
        if kind == StatusKind::Busy {
            ui.spinner();
        } else {
            ui.label(RichText::new(kind.icon()).size(TEXT_CAPTION).color(color));
        }
        ui.label(RichText::new(text).size(TEXT_CAPTION).color(color));
    })
    .response
}

/// Przycisk główny (np. Zapisz QSO): tło `accent`, tekst `selection_text`.
pub fn primary_button(palette: &Palette, text: impl Into<String>) -> Button<'static> {
    Button::new(
        RichText::new(text.into())
            .strong()
            .color(palette.selection_text),
    )
    .fill(palette.accent)
    .corner_radius(RADIUS_MD)
}

/// Przycisk pomocniczy (np. Wyczyść): obrys `border`, tekst `text_primary`.
pub fn secondary_button(palette: &Palette, text: impl Into<String>) -> Button<'static> {
    Button::new(RichText::new(text.into()).color(palette.text_primary))
        .stroke(Stroke::new(1.0_f32, palette.border))
        .corner_radius(RADIUS_MD)
}

/// Przycisk destrukcyjny (usuń, nieodwracalne akcje): tło `danger`.
pub fn danger_button(palette: &Palette, text: impl Into<String>) -> Button<'static> {
    Button::new(
        RichText::new(text.into())
            .strong()
            .color(readable_on(palette.danger)),
    )
    .fill(palette.danger)
    .corner_radius(RADIUS_MD)
}

/// Nagłówek sekcji w ustawieniach i dialogach.
pub fn section_header(ui: &mut Ui, palette: &Palette, title: impl Into<String>) {
    ui.add_space(SPACE_MD);
    ui.label(
        RichText::new(title.into())
            .strong()
            .size(TEXT_SUBHEADING)
            .color(palette.text_primary),
    );
    ui.separator();
}

/// Bok kwadratu ikony okna (px logiczne).
const WINDOW_ICON_SIZE: f32 = 18.0;

/// Kształty wektorowej ikony okna w kwadracie `rect`.
///
/// * `redock == false` — „odepnij” (pop-out): małe okno + strzałka ↗ wychodząca w prawy górny róg,
/// * `redock == true` — „zadokuj z powrotem”: większe okno + strzałka ↙ wchodząca do środka.
///
/// Rysowane ze `Shape`, bez znaków Unicode (które na części fontów renderują się jako kwadrat).
#[must_use]
pub fn window_icon_shapes(rect: Rect, color: Color32, redock: bool) -> Vec<Shape> {
    let unit = rect.width() / WINDOW_ICON_SIZE;
    let p = |x: f32, y: f32| -> Pos2 { rect.min + vec2(x * unit, y * unit) };
    let stroke = Stroke::new(1.5_f32, color);
    let line = |a: Pos2, b: Pos2| Shape::line_segment([a, b], stroke);
    let frame = |min: Pos2, max: Pos2| {
        Shape::rect_stroke(
            Rect::from_min_max(min, max),
            1.5_f32,
            stroke,
            StrokeKind::Inside,
        )
    };
    if redock {
        vec![
            frame(p(2.0, 4.0), p(14.0, 16.0)),
            line(p(16.0, 2.0), p(8.0, 10.0)),
            line(p(8.0, 10.0), p(8.0, 5.0)),
            line(p(8.0, 10.0), p(13.0, 10.0)),
        ]
    } else {
        vec![
            frame(p(2.0, 6.0), p(12.0, 16.0)),
            line(p(7.0, 11.0), p(15.0, 3.0)),
            line(p(15.0, 3.0), p(10.0, 3.0)),
            line(p(15.0, 3.0), p(15.0, 8.0)),
        ]
    }
}

fn window_icon_button(
    ui: &mut Ui,
    palette: &Palette,
    redock: bool,
    label: Option<&str>,
    tooltip: &str,
) -> Response {
    let galley = label.map(|text| {
        ui.painter().layout_no_wrap(
            text.to_owned(),
            TextStyle::Button.resolve(ui.style()),
            palette.text_primary,
        )
    });
    let text_width = galley.as_ref().map_or(0.0, |g| g.size().x + SPACE_SM);
    let size = vec2(WINDOW_ICON_SIZE + SPACE_SM * 2.0 + text_width, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tooltip));
    if ui.is_rect_visible(rect) {
        let active = response.hovered() || response.has_focus();
        let color = if active {
            palette.accent
        } else {
            palette.text_primary
        };
        if active {
            ui.painter().rect(
                rect,
                RADIUS_SM,
                palette.subtle_bg,
                Stroke::new(1.0_f32, palette.border),
                StrokeKind::Inside,
            );
        }
        let icon_rect = Rect::from_min_size(
            rect.min + vec2(SPACE_SM, (size.y - WINDOW_ICON_SIZE) / 2.0),
            vec2(WINDOW_ICON_SIZE, WINDOW_ICON_SIZE),
        );
        ui.painter()
            .extend(window_icon_shapes(icon_rect, color, redock));
        if let Some(galley) = galley {
            let pos = egui::pos2(
                icon_rect.max.x + SPACE_SM,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(pos, galley, color);
        }
    }
    response.on_hover_text(tooltip)
}

/// Przycisk „odepnij do osobnego okna” (wektorowa ikona pop-out) z tooltipem.
pub fn undock_button(ui: &mut Ui, palette: &Palette, tooltip: &str) -> Response {
    window_icon_button(ui, palette, false, None, tooltip)
}

/// Przycisk „zadokuj z powrotem” (wektorowa ikona + etykieta) dla okna odpiętego.
pub fn dock_button(ui: &mut Ui, palette: &Palette, label: &str, tooltip: &str) -> Response {
    window_icon_button(ui, palette, true, Some(label), tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::theme::ThemePreset;
    use eframe::egui;

    #[test]
    fn status_and_toast_colors_are_readable_in_all_themes() {
        for preset in ThemePreset::ALL {
            let p = preset.palette();
            for kind in [
                StatusKind::Ok,
                StatusKind::Warn,
                StatusKind::Err,
                StatusKind::Idle,
                StatusKind::Busy,
            ] {
                for (bg_name, bg) in [("panel_bg", p.panel_bg), ("window_bg", p.window_bg)] {
                    assert!(
                        contrast_ratio(kind.color(&p), bg) >= 4.5,
                        "{preset:?}: {kind:?} na {bg_name} < 4.5:1"
                    );
                }
            }
            for severity in [
                ToastSeverity::Info,
                ToastSeverity::Success,
                ToastSeverity::Warning,
                ToastSeverity::Error,
            ] {
                // Toast: tło `surface_bg`, tekst `text_primary`, obramowanie/ikona w kolorze ważności.
                let accent = toast_accent(&p, severity);
                assert!(
                    contrast_ratio(accent, p.surface_bg) >= 3.0,
                    "{preset:?}: obramowanie {severity:?} < 3:1 (element graficzny)"
                );
                assert!(
                    contrast_ratio(p.text_primary, p.surface_bg) >= 4.5,
                    "{preset:?}: tekst toastu < 4.5:1"
                );
            }
        }
    }

    #[test]
    fn button_text_is_readable_on_fills_in_all_themes() {
        for preset in ThemePreset::ALL {
            let p = preset.palette();
            assert!(
                contrast_ratio(p.selection_text, p.accent) >= 4.5,
                "{preset:?}: primary"
            );
            assert!(
                contrast_ratio(readable_on(p.danger), p.danger) >= 4.5,
                "{preset:?}: danger"
            );
            assert!(
                contrast_ratio(p.text_primary, p.panel_bg) >= 4.5,
                "{preset:?}: secondary"
            );
        }
    }

    #[test]
    fn window_icons_are_vector_shapes_inside_their_box() {
        let rect = Rect::from_min_size(
            egui::pos2(10.0, 20.0),
            vec2(WINDOW_ICON_SIZE, WINDOW_ICON_SIZE),
        );
        let undock = window_icon_shapes(rect, Color32::WHITE, false);
        let redock = window_icon_shapes(rect, Color32::WHITE, true);
        for shapes in [&undock, &redock] {
            assert_eq!(shapes.len(), 4, "ramka okna + trzon i dwa ramiona strzałki");
            for shape in shapes {
                assert!(
                    rect.expand(0.1)
                        .contains_rect(shape.visual_bounding_rect().shrink(1.0)),
                    "ikona nie może wychodzić poza swój kwadrat: {shape:?}"
                );
            }
        }
        assert_ne!(
            format!("{undock:?}"),
            format!("{redock:?}"),
            "undock i redock to różne ikony"
        );
    }

    #[test]
    fn dock_buttons_render_and_report_clicks_area() {
        let ctx = egui::Context::default();
        let palette = ThemePreset::OperatorDark.palette();
        let mut sizes = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            sizes.push(undock_button(ui, &palette, "odepnij").rect.size());
            sizes.push(dock_button(ui, &palette, "Przypnij", "zadokuj").rect.size());
        });
        output.textures_delta.clear();
        assert!(sizes[0].x >= WINDOW_ICON_SIZE && sizes[0].y >= WINDOW_ICON_SIZE);
        assert!(sizes[1].x > sizes[0].x, "wariant z etykietą jest szerszy");
    }

    #[test]
    fn components_render_without_panic() {
        let ctx = egui::Context::default();
        let palette = ThemePreset::OperatorDark.palette();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for kind in [
                StatusKind::Ok,
                StatusKind::Warn,
                StatusKind::Err,
                StatusKind::Idle,
                StatusKind::Busy,
            ] {
                status_badge(ui, &palette, kind, "stan");
            }
            ui.add(primary_button(&palette, "ok"));
            ui.add(secondary_button(&palette, "ok"));
            ui.add(danger_button(&palette, "ok"));
            section_header(ui, &palette, "sekcja");
        });
        output.textures_delta.clear();
    }
}
