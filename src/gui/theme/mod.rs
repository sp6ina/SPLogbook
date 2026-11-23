// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny system motywów, palet kolorystycznych i dostępności aplikacji SPLogbook.

pub mod components;
pub mod contrast;
pub mod ext;
pub mod palette;
pub mod tokens;

use crate::core::station::{ThemeCustomization, UiDensity};
use eframe::egui;
use egui::Color32;

pub use components::{
    StatusKind, danger_button, dock_button, primary_button, readable_on, secondary_button,
    section_header, status_badge, toast_accent, undock_button, window_icon_shapes,
};
pub use contrast::{contrast_ratio, relative_luminance};
pub use ext::{ColorExt, ThemeExt};
pub use palette::{
    BandColors, MapColors, Palette, SegmentColors, ThemePreset, WaterfallPalette, waterfall_color,
};
pub use tokens::{
    RADIUS_MD, RADIUS_SM, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL, SPACE_XS, TEXT_BODY,
    TEXT_CALLSIGN, TEXT_CAPTION, TEXT_FREQ, TEXT_SUBHEADING, callsign_font, mono_callsign,
    mono_font, mono_freq, mono_table_cell,
};

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

/// Stan ostatnio zastosowanego motywu wraz ze skalą czcionki i personalizacją.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedTheme {
    pub preset: ThemePreset,
    pub font_scale_bits: u32,
    pub custom: ThemeCustomization,
}

/// Najmniejszy kontrast akcentu z tłem paneli i okien (WCAG 2.1, kryterium 1.4.11
/// dla elementów interfejsu); motyw wysokiego kontrastu wymaga 4,5:1.
pub fn min_accent_contrast(preset: ThemePreset) -> f32 {
    if preset == ThemePreset::HighContrast {
        4.5
    } else {
        3.0
    }
}

/// Najsłabszy kontrast koloru z tłem paneli i okien motywu.
pub fn accent_contrast(preset: ThemePreset, rgb: [u8; 3]) -> f32 {
    let base = preset.palette();
    let accent = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    contrast_ratio(accent, base.panel_bg).min(contrast_ratio(accent, base.window_bg))
}

pub fn accent_is_readable(preset: ThemePreset, rgb: [u8; 3]) -> bool {
    accent_contrast(preset, rgb) >= min_accent_contrast(preset)
}

/// Paleta motywu z własnym akcentem. Akcent bez wymaganego kontrastu jest pomijany.
pub fn custom_palette(preset: ThemePreset, custom: &ThemeCustomization) -> Palette {
    let mut palette = preset.palette();
    if let Some(rgb) = custom
        .accent_rgb
        .filter(|rgb| accent_is_readable(preset, *rgb))
    {
        let accent = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        palette.accent = accent;
        palette.info = accent;
        palette.selection_bg = accent;
        palette.selection_text = readable_on(accent);
    }
    palette
}

impl AppliedTheme {
    pub fn new(preset: ThemePreset, font_scale: f32) -> Self {
        Self::with_custom(preset, font_scale, ThemeCustomization::default())
    }

    pub fn with_custom(preset: ThemePreset, font_scale: f32, custom: ThemeCustomization) -> Self {
        let clamped = font_scale.clamp(0.8, 1.5);
        Self {
            preset,
            font_scale_bits: clamped.to_bits(),
            custom,
        }
    }

    pub fn font_scale(&self) -> f32 {
        f32::from_bits(self.font_scale_bits)
    }
}

/// Zapewnia, że visuals, skala UI, style i Palette są zaaplikowane w `egui::Context`.
/// Aktualizacja następuje wyłącznie przy pierwszym wywołaniu lub przy zmianie motywu / font_scale.
pub fn ensure_applied(ctx: &egui::Context, preset: ThemePreset, font_scale: f32) {
    ensure_applied_custom(ctx, preset, font_scale, &ThemeCustomization::default());
}

/// Jak [`ensure_applied`], z własnym akcentem i gęstością interfejsu.
pub fn ensure_applied_custom(
    ctx: &egui::Context,
    preset: ThemePreset,
    font_scale: f32,
    custom: &ThemeCustomization,
) {
    let target = AppliedTheme::with_custom(preset, font_scale, *custom);
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

    let palette = custom_palette(preset, custom);

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
    let (window_margin, button_padding, item_spacing) = match custom.density {
        UiDensity::Compact => (
            egui::Margin::symmetric(4, 2),
            egui::vec2(SPACE_XS, 1.0),
            egui::vec2(6.0, SPACE_XS),
        ),
        UiDensity::Normal => (
            egui::Margin::symmetric(6, 4),
            egui::vec2(SPACE_SM, SPACE_XS),
            egui::vec2(SPACE_MD, 3.0),
        ),
        UiDensity::Comfortable => (
            egui::Margin::symmetric(10, 8),
            egui::vec2(SPACE_MD, SPACE_SM),
            egui::vec2(SPACE_LG, 6.0),
        ),
    };
    ctx.style_mut_of(theme, |s| {
        s.spacing.window_margin = window_margin;
        s.spacing.button_padding = button_padding;
        s.spacing.item_spacing = item_spacing;
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
    fn custom_accent_is_used_only_when_readable() {
        let bright = ThemeCustomization {
            accent_rgb: Some([255, 200, 0]),
            density: UiDensity::Normal,
        };
        let palette = custom_palette(ThemePreset::OperatorDark, &bright);
        assert_eq!(palette.accent, Color32::from_rgb(255, 200, 0));
        assert_eq!(palette.selection_bg, palette.accent);
        assert!(contrast_ratio(palette.selection_text, palette.selection_bg) >= 4.5);

        let weak = ThemeCustomization {
            accent_rgb: Some([20, 30, 60]),
            density: UiDensity::Normal,
        };
        assert_eq!(
            custom_palette(ThemePreset::OperatorDark, &weak),
            ThemePreset::OperatorDark.palette()
        );
    }

    #[test]
    fn density_changes_spacing_and_reapplies_the_theme() {
        let spacing = |density| {
            let ctx = egui::Context::default();
            let custom = ThemeCustomization {
                accent_rgb: None,
                density,
            };
            ensure_applied_custom(&ctx, ThemePreset::Daylight, 1.0, &custom);
            let applied = ctx.data(|d| d.get_temp::<AppliedTheme>(applied_theme_id()));
            assert_eq!(applied.map(|a| a.custom), Some(custom));
            ctx.style_of(ctx.theme()).spacing.item_spacing
        };
        let compact = spacing(UiDensity::Compact);
        let normal = spacing(UiDensity::Normal);
        let comfortable = spacing(UiDensity::Comfortable);
        assert!(compact.x < normal.x && normal.x < comfortable.x);
        assert!(compact.y < normal.y && normal.y < comfortable.y);
    }

    #[test]
    fn test_applied_theme_clamping() {
        let t1 = AppliedTheme::new(ThemePreset::HighContrast, 0.5);
        assert!((t1.font_scale() - 0.8).abs() < 1e-4);

        let t2 = AppliedTheme::new(ThemePreset::HighContrast, 2.5);
        assert!((t2.font_scale() - 1.5).abs() < 1e-4);
    }
}
