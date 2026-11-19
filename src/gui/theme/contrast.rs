// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use eframe::egui::Color32;

/// Oblicza względną luminancję (relative luminance) składowej sRGB zgodnie z WCAG 2.1.
pub fn relative_luminance(color: Color32) -> f32 {
    let srgb_to_linear = |c_u8: u8| -> f32 {
        let c = c_u8 as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    let r = srgb_to_linear(color.r());
    let g = srgb_to_linear(color.g());
    let b = srgb_to_linear(color.b());

    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Oblicza współczynnik kontrastu (contrast ratio) między dwoma kolorami zgodnie z WCAG 2.1.
/// Wynik mieści się w przedziale [1.0, 21.0].
pub fn contrast_ratio(c1: Color32, c2: Color32) -> f32 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let (l_bright, l_dark) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (l_bright + 0.05) / (l_dark + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::theme::ThemePreset;

    #[test]
    fn test_black_white_contrast() {
        let black = Color32::from_rgb(0, 0, 0);
        let white = Color32::from_rgb(255, 255, 255);

        assert!((relative_luminance(black) - 0.0).abs() < 1e-4);
        assert!((relative_luminance(white) - 1.0).abs() < 1e-4);

        let ratio = contrast_ratio(black, white);
        assert!((ratio - 21.0).abs() < 0.1);

        let self_ratio = contrast_ratio(white, white);
        assert!((self_ratio - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_high_contrast_wcag_compliance() {
        let p = ThemePreset::HighContrast.palette();
        // HighContrast: tekst >= 7:1, tokeny semantyczne >= 4.5:1
        assert!(
            contrast_ratio(p.text_primary, p.panel_bg) >= 7.0,
            "text_primary WCAG AAA failed"
        );
        assert!(
            contrast_ratio(p.text_muted, p.panel_bg) >= 7.0,
            "text_muted WCAG AAA failed"
        );
        assert!(
            contrast_ratio(p.accent, p.panel_bg) >= 4.5,
            "accent semantic contrast failed"
        );
        assert!(
            contrast_ratio(p.accent_alt, p.panel_bg) >= 4.5,
            "accent_alt semantic contrast failed"
        );
        assert!(
            contrast_ratio(p.success, p.panel_bg) >= 4.5,
            "success semantic contrast failed"
        );
        assert!(
            contrast_ratio(p.warning, p.panel_bg) >= 4.5,
            "warning semantic contrast failed"
        );
        assert!(
            contrast_ratio(p.danger, p.panel_bg) >= 4.5,
            "danger semantic contrast failed"
        );
        assert!(
            contrast_ratio(p.info, p.panel_bg) >= 4.5,
            "info semantic contrast failed"
        );
    }

    #[test]
    fn test_daylight_wcag_compliance() {
        let p = ThemePreset::Daylight.palette();
        // Daylight: tekst >= 4.5:1, tokeny semantyczne >= 3:1
        assert!(
            contrast_ratio(p.text_primary, p.panel_bg) >= 4.5,
            "Daylight text_primary failed"
        );
        assert!(
            contrast_ratio(p.text_muted, p.panel_bg) >= 4.5,
            "Daylight text_muted failed"
        );
        assert!(
            contrast_ratio(p.accent, p.panel_bg) >= 3.0,
            "Daylight accent failed"
        );
        assert!(
            contrast_ratio(p.accent_alt, p.panel_bg) >= 3.0,
            "Daylight accent_alt failed"
        );
        assert!(
            contrast_ratio(p.success, p.panel_bg) >= 3.0,
            "Daylight success failed"
        );
        assert!(
            contrast_ratio(p.warning, p.panel_bg) >= 3.0,
            "Daylight warning failed"
        );
        assert!(
            contrast_ratio(p.danger, p.panel_bg) >= 3.0,
            "Daylight danger failed"
        );
        assert!(
            contrast_ratio(p.info, p.panel_bg) >= 3.0,
            "Daylight info failed"
        );
    }

    #[test]
    fn test_operator_dark_wcag_compliance() {
        let p = ThemePreset::OperatorDark.palette();
        // OperatorDark: tekst >= 4.5:1, tokeny semantyczne >= 3:1
        assert!(
            contrast_ratio(p.text_primary, p.panel_bg) >= 4.5,
            "OperatorDark text_primary failed"
        );
        assert!(
            contrast_ratio(p.text_muted, p.panel_bg) >= 4.5,
            "OperatorDark text_muted failed"
        );
        assert!(
            contrast_ratio(p.accent, p.panel_bg) >= 3.0,
            "OperatorDark accent failed"
        );
        assert!(
            contrast_ratio(p.accent_alt, p.panel_bg) >= 3.0,
            "OperatorDark accent_alt failed"
        );
        assert!(
            contrast_ratio(p.success, p.panel_bg) >= 3.0,
            "OperatorDark success failed"
        );
        assert!(
            contrast_ratio(p.warning, p.panel_bg) >= 3.0,
            "OperatorDark warning failed"
        );
        assert!(
            contrast_ratio(p.danger, p.panel_bg) >= 3.0,
            "OperatorDark danger failed"
        );
        assert!(
            contrast_ratio(p.info, p.panel_bg) >= 3.0,
            "OperatorDark info failed"
        );
    }

    #[test]
    fn test_operator_dark_text_and_danger_on_window_bg() {
        let p = ThemePreset::OperatorDark.palette();
        for (name, fg) in [
            ("text_primary", p.text_primary),
            ("text_muted", p.text_muted),
            ("text_subtle", p.text_subtle),
            ("danger", p.danger),
            ("tx", p.tx),
            ("dupe", p.dupe),
            ("sync_error", p.sync_error),
        ] {
            for (bg_name, bg) in [("panel_bg", p.panel_bg), ("window_bg", p.window_bg)] {
                assert!(
                    contrast_ratio(fg, bg) >= 4.5,
                    "OperatorDark {name} na {bg_name} < 4.5:1"
                );
            }
        }
    }

    #[test]
    fn test_daylight_warning_on_cards_and_subtle_bg() {
        let p = ThemePreset::Daylight.palette();
        for (name, fg) in [
            ("warning", p.warning),
            ("split", p.split),
            ("multiplier", p.multiplier),
            ("qsl_queued", p.qsl_queued),
            ("sync_syncing", p.sync_syncing),
        ] {
            for (bg_name, bg) in [
                ("panel_bg", p.panel_bg),
                ("window_bg", p.window_bg),
                ("surface_bg", p.surface_bg),
                ("subtle_bg", p.subtle_bg),
            ] {
                assert!(
                    contrast_ratio(fg, bg) >= 4.5,
                    "Daylight {name} na {bg_name} < 4.5:1"
                );
            }
        }
    }

    #[test]
    fn test_high_contrast_window_bg_pairs() {
        let p = ThemePreset::HighContrast.palette();
        for (name, fg) in [
            ("text_primary", p.text_primary),
            ("text_muted", p.text_muted),
            ("text_subtle", p.text_subtle),
        ] {
            for (bg_name, bg) in [("panel_bg", p.panel_bg), ("window_bg", p.window_bg)] {
                assert!(
                    contrast_ratio(fg, bg) >= 7.0,
                    "HighContrast {name} na {bg_name} < 7:1"
                );
            }
        }
        for (name, fg) in [
            ("accent", p.accent),
            ("accent_alt", p.accent_alt),
            ("success", p.success),
            ("warning", p.warning),
            ("danger", p.danger),
            ("info", p.info),
            ("tx", p.tx),
            ("dupe", p.dupe),
            ("sync_error", p.sync_error),
        ] {
            for (bg_name, bg) in [
                ("panel_bg", p.panel_bg),
                ("window_bg", p.window_bg),
                ("subtle_bg", p.subtle_bg),
            ] {
                assert!(
                    contrast_ratio(fg, bg) >= 4.5,
                    "HighContrast {name} na {bg_name} < 4.5:1"
                );
            }
        }
    }

    #[test]
    fn test_high_contrast_semantic_states_are_distinct() {
        let p = ThemePreset::HighContrast.palette();
        assert_ne!(p.danger, p.text_primary, "danger == text_primary");
        assert_ne!(p.dupe, p.text_primary, "dupe == text_primary");
        assert_ne!(p.sync_error, p.text_primary, "sync_error == text_primary");
        assert_ne!(p.tx, p.accent, "tx == accent");
        assert_ne!(p.tx, p.warning, "tx == warning");
        assert_ne!(p.tx, p.selection_bg, "tx == selection_bg");
        assert_ne!(p.tx, p.rx, "tx == rx");
    }

    #[test]
    fn test_text_on_accent_buttons_all_themes() {
        // Przycisk „Zapisz QSO” (tło accent, tekst selection_text) oraz zaznaczenie.
        for preset in ThemePreset::ALL {
            let p = preset.palette();
            let min = if preset == ThemePreset::HighContrast {
                7.0
            } else {
                4.5
            };
            assert!(
                contrast_ratio(p.selection_text, p.accent) >= min,
                "{preset:?}: selection_text na accent < {min}:1"
            );
            assert!(
                contrast_ratio(p.selection_text, p.selection_bg) >= min,
                "{preset:?}: selection_text na selection_bg < {min}:1"
            );
        }
    }

    #[test]
    fn test_all_themes_primary_text_on_window_bg() {
        for preset in ThemePreset::ALL {
            let p = preset.palette();
            assert!(
                contrast_ratio(p.text_primary, p.window_bg) >= 4.5,
                "{preset:?}: text_primary na window_bg < 4.5:1"
            );
            assert!(
                contrast_ratio(p.text_muted, p.window_bg) >= 4.5,
                "{preset:?}: text_muted na window_bg < 4.5:1"
            );
            assert!(
                contrast_ratio(p.text_subtle, p.window_bg) >= 4.5,
                "{preset:?}: text_subtle na window_bg < 4.5:1"
            );
        }
    }
}
