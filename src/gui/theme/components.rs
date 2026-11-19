// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Lekkie, reużywalne komponenty UI oparte na `Palette` i tokenach: odznaki stanu,
//! warianty przycisków (primary / secondary / danger) oraz nagłówki sekcji.

use eframe::egui::{Button, Color32, Response, RichText, Stroke, Ui};

use super::contrast::contrast_ratio;
use super::palette::Palette;
use super::tokens::{RADIUS_MD, SPACE_MD, TEXT_CAPTION, TEXT_SUBHEADING};
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
