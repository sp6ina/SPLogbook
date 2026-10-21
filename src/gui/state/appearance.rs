// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Application visual appearance state (language, theme preset, fullscreen).

use crate::core::i18n::Language;
use crate::gui::theme::ThemePreset;

/// Appearance and display state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceState {
    pub language: Language,
    pub theme_preset: ThemePreset,
    pub applied: Option<(ThemePreset, u32)>,
    pub is_fullscreen: bool,
}

impl Default for AppearanceState {
    fn default() -> Self {
        Self {
            language: Language::Pl,
            theme_preset: ThemePreset::OperatorDark,
            applied: None,
            is_fullscreen: false,
        }
    }
}

impl AppearanceState {
    pub fn from_config(config: &crate::core::station::AppConfig) -> Self {
        let language = Language::from_code(&config.current_language);
        let theme_preset = if config.theme_preset.is_empty() {
            if !config.dark_theme {
                ThemePreset::Daylight
            } else {
                ThemePreset::OperatorDark
            }
        } else {
            ThemePreset::from_id(&config.theme_preset)
        };
        Self {
            language,
            theme_preset,
            applied: None,
            is_fullscreen: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = AppearanceState::default();
        assert_eq!(state.language, Language::Pl);
        assert_eq!(state.theme_preset, ThemePreset::OperatorDark);
        assert!(state.applied.is_none());
        assert!(!state.is_fullscreen);
    }
}
