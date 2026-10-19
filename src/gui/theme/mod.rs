// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny system motywów, palet kolorystycznych i dostępności aplikacji SPLogbook.

pub mod contrast;
pub mod ext;
pub mod palette;

use eframe::egui;
use egui::Color32;

pub use contrast::{contrast_ratio, relative_luminance};
pub use ext::{ColorExt, ThemeExt};
pub use palette::{BandColors, MapColors, Palette, SegmentColors, ThemePreset};

/// Klucz tekstowy aktywnej palety w pamięci podręcznej `egui::Context`.
pub const PALETTE_ID_STR: &str = "splogbook_theme_palette";

/// Klucz tekstowy stanu ostatnio zastosowanego motywu.
pub const APPLIED_THEME_ID_STR: &str = "splogbook_applied_theme";

#[inline]
pub fn palette_id() -> egui::Id {
    egui::Id::new(PALETTE_ID_STR)
}

#[inline]
pub fn applied_theme_id() -> egui::Id {
    egui::Id::new(APPLIED_THEME_ID_STR)
}

/// Stan ostatnio zastosowanego motywu wraz ze skalą czcionki.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedTheme {
    pub preset: ThemePreset,
    pub font_scale_bits: u32,
}

impl AppliedTheme {
    pub fn new(preset: ThemePreset, font_scale: f32) -> Self {
        let clamped = font_scale.clamp(0.8, 1.5);
        Self {
            preset,
            font_scale_bits: clamped.to_bits(),
        }
    }

    pub fn font_scale(&self) -> f32 {
        f32::from_bits(self.font_scale_bits)
    }
}

/// Zapewnia, że visuals, skala UI, style i Palette są zaaplikowane w `egui::Context`.
/// Aktualizacja następuje wyłącznie przy pierwszym wywołaniu lub przy zmianie motywu / font_scale.
pub fn ensure_applied(ctx: &egui::Context, preset: ThemePreset, font_scale: f32) {
    let target = AppliedTheme::new(preset, font_scale);
    let current = ctx.data(|d| d.get_temp::<AppliedTheme>(applied_theme_id()));

    let needs_apply = match current {
        Some(applied) => applied != target,
        None => true,
    };

    if !needs_apply {
        let has_palette = ctx.data(|d| d.get_temp::<Palette>(palette_id()).is_some());
        if has_palette {
            return;
        }
    }

    let palette = preset.palette();

    let mut visuals = if palette.is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.hyperlink_color = palette.accent;
    visuals.selection.bg_fill = palette.accent;
    visuals.selection.stroke.color = if palette.is_dark {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    visuals.panel_fill = palette.panel_bg;
    visuals.window_fill = palette.window_bg;
    visuals.extreme_bg_color = palette.extreme_bg;
    visuals.override_text_color = None;
    visuals.widgets.hovered.bg_fill = blend(visuals.widgets.hovered.bg_fill, palette.accent, 0.12);
    visuals.widgets.active.bg_fill = blend(visuals.widgets.active.bg_fill, palette.accent, 0.22);

    ctx.set_visuals(visuals);
    ctx.set_zoom_factor(target.font_scale());

    let theme = ctx.theme();
    ctx.style_mut_of(theme, |s| {
        s.spacing.window_margin = egui::Margin::symmetric(6, 4);
        s.spacing.button_padding = egui::vec2(4.0, 2.0);
    });

    ctx.data_mut(|d| {
        d.insert_temp(palette_id(), palette);
        d.insert_temp(applied_theme_id(), target);
    });
}

fn blend(base: Color32, tint: Color32, t: f32) -> Color32 {
    let factor = t.clamp(0.0, 1.0);
    let lerp = |a: u8, b: u8| -> u8 { (a as f32 + (b as f32 - a as f32) * factor).round() as u8 };
    Color32::from_rgb(
        lerp(base.r(), tint.r()),
        lerp(base.g(), tint.g()),
        lerp(base.b(), tint.b()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uninitialized_palette_returns_operator_dark() {
        let ctx = egui::Context::default();
        let palette = ctx.palette();
        assert_eq!(palette, ThemePreset::OperatorDark.palette());
    }

    #[test]
    fn test_ensure_applied_applies_and_caches() {
        let ctx = egui::Context::default();
        ensure_applied(&ctx, ThemePreset::Daylight, 1.1);

        let applied = ctx.data(|d| d.get_temp::<AppliedTheme>(applied_theme_id()));
        assert_eq!(applied, Some(AppliedTheme::new(ThemePreset::Daylight, 1.1)));

        let palette = ctx.palette();
        assert_eq!(palette, ThemePreset::Daylight.palette());
        assert!(!palette.is_dark);
    }

    #[test]
    fn test_applied_theme_clamping() {
        let t1 = AppliedTheme::new(ThemePreset::HighContrast, 0.5);
        assert!((t1.font_scale() - 0.8).abs() < 1e-4);

        let t2 = AppliedTheme::new(ThemePreset::HighContrast, 2.5);
        assert!((t2.font_scale() - 1.5).abs() < 1e-4);
    }
}
