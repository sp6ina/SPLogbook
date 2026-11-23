// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Theme personalization: base theme, an own accent colour checked against WCAG 2.1
//! contrast, and interface density.

use crate::core::i18n::{Msg, key, tr};
use crate::core::station::{ThemeCustomization, UiDensity};
use crate::gui::app::SpLogApp;
use crate::gui::theme::{
    ThemeExt, ThemePreset, accent_contrast, accent_is_readable, min_accent_contrast,
};
use eframe::egui;

/// Colour edited in the window before it reaches the configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentDraft {
    pub own: bool,
    pub rgb: [u8; 3],
}

/// Accent stored in the configuration for a draft: an own colour only when it has the
/// contrast the base theme requires. The flag tells that a colour was rejected.
pub fn accent_for(preset: ThemePreset, draft: AccentDraft) -> (Option<[u8; 3]>, bool) {
    if !draft.own {
        (None, false)
    } else if accent_is_readable(preset, draft.rgb) {
        (Some(draft.rgb), false)
    } else {
        (None, true)
    }
}

fn density_label(density: UiDensity, lang: crate::core::i18n::Language) -> &'static str {
    let k = match density {
        UiDensity::Compact => key::THEME_DENSITY_COMPACT,
        UiDensity::Normal => key::THEME_DENSITY_NORMAL,
        UiDensity::Comfortable => key::THEME_DENSITY_COMFORTABLE,
    };
    tr(k, lang)
}

impl SpLogApp {
    /// Opens the window with the colour in use: the own accent or the theme accent.
    pub fn open_theme_customizer(&mut self, current_accent: [u8; 3]) {
        let own = self.config.theme_custom.accent_rgb;
        self.gui.windows.theme_accent_draft = Some(AccentDraft {
            own: own.is_some(),
            rgb: own.unwrap_or(current_accent),
        });
    }

    /// Stores a new personalization and saves the configuration.
    pub fn apply_theme_customization(&mut self, custom: ThemeCustomization) {
        if self.config.theme_custom != custom {
            self.config.theme_custom = custom;
            self.save_station_config();
        }
    }
}

pub fn render_theme_customizer(app: &mut SpLogApp, ctx: &egui::Context) {
    let Some(mut draft) = app.gui.windows.theme_accent_draft else {
        return;
    };
    let lang = app.gui.appearance.language;
    let mut open = true;
    let mut custom = app.config.theme_custom;
    let mut preset = app.gui.appearance.theme_preset;
    egui::Window::new(tr(key::THEME_CUSTOM_TITLE, lang))
        .id(egui::Id::new("theme_customizer"))
        .open(&mut open)
        .resizable(false)
        .default_width(440.0)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.strong(tr(key::THEME_CUSTOM_BASE, lang));
            ui.horizontal_wrapped(|ui| {
                for candidate in ThemePreset::ALL {
                    if ui
                        .selectable_label(preset == candidate, candidate.label(lang))
                        .on_hover_text(candidate.description(lang))
                        .clicked()
                    {
                        preset = candidate;
                    }
                }
            });
            ui.separator();

            ui.strong(tr(key::THEME_CUSTOM_ACCENT, lang));
            ui.horizontal(|ui| {
                ui.checkbox(&mut draft.own, tr(key::THEME_CUSTOM_OWN_ACCENT, lang));
                ui.add_enabled_ui(draft.own, |ui| {
                    ui.color_edit_button_srgb(&mut draft.rgb);
                });
            });
            let ratio = accent_contrast(preset, draft.rgb);
            let minimum = min_accent_contrast(preset);
            let (accent, rejected) = accent_for(preset, draft);
            custom.accent_rgb = accent;
            if draft.own {
                let text = Msg::new(key::THEME_CUSTOM_CONTRAST)
                    .arg("ratio", format!("{ratio:.1}"))
                    .arg("min", format!("{minimum:.1}"))
                    .text(lang);
                let color = if rejected {
                    palette.danger
                } else {
                    palette.success
                };
                ui.colored_label(color, text);
                if rejected {
                    ui.label(
                        egui::RichText::new(tr(key::THEME_CUSTOM_ACCENT_REJECTED, lang))
                            .small()
                            .color(palette.warning),
                    );
                }
            }
            ui.separator();

            ui.strong(tr(key::THEME_CUSTOM_DENSITY, lang));
            ui.horizontal(|ui| {
                for density in UiDensity::ALL {
                    ui.radio_value(&mut custom.density, density, density_label(density, lang));
                }
            });
            ui.separator();

            ui.label(
                egui::RichText::new(tr(key::THEME_CUSTOM_PREVIEW, lang))
                    .small()
                    .color(palette.text_muted),
            );
            ui.horizontal_wrapped(|ui| {
                let _ = ui.button(tr(key::THEME_CUSTOM_PREVIEW_BUTTON, lang));
                ui.colored_label(
                    palette.accent,
                    format!("● {}", tr(key::THEME_CUSTOM_ACCENT, lang)),
                );
                ui.colored_label(
                    palette.success,
                    format!("● {}", tr(key::THEME_PREVIEW_SUCCESS, lang)),
                );
                ui.colored_label(
                    palette.warning,
                    format!("● {}", tr(key::THEME_PREVIEW_WARNING, lang)),
                );
                ui.colored_label(
                    palette.danger,
                    format!("● {}", tr(key::THEME_PREVIEW_DANGER, lang)),
                );
            });
            ui.separator();
            if ui.button(tr(key::THEME_CUSTOM_RESET, lang)).clicked() {
                custom = ThemeCustomization::default();
                draft.own = false;
            }
        });

    if preset != app.gui.appearance.theme_preset {
        app.gui.appearance.theme_preset = preset;
        app.save_station_config();
    }
    app.apply_theme_customization(custom);
    app.gui.windows.theme_accent_draft = open.then_some(draft);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_accent_is_kept_and_a_weak_one_is_rejected() {
        let bright = AccentDraft {
            own: true,
            rgb: [255, 200, 0],
        };
        assert_eq!(
            accent_for(ThemePreset::OperatorDark, bright),
            (Some([255, 200, 0]), false)
        );
        // Granat na ciemnym tle nie daje 3:1.
        let dark = AccentDraft {
            own: true,
            rgb: [20, 30, 60],
        };
        assert_eq!(accent_for(ThemePreset::OperatorDark, dark), (None, true));
        // Na jasnym motywie ten sam granat jest czytelny.
        assert_eq!(
            accent_for(ThemePreset::Daylight, dark),
            (Some([20, 30, 60]), false)
        );
        let off = AccentDraft {
            own: false,
            rgb: [20, 30, 60],
        };
        assert_eq!(accent_for(ThemePreset::OperatorDark, off), (None, false));
    }

    #[test]
    fn high_contrast_theme_needs_a_stronger_accent() {
        assert!(
            min_accent_contrast(ThemePreset::HighContrast)
                > min_accent_contrast(ThemePreset::Daylight)
        );
    }
}
