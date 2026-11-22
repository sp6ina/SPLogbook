// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

/// Liczba makr telegraficznych (klawisze F1..F8).
pub const CW_MACRO_COUNT: usize = 8;

pub use crate::core::station::default_cw_macros;

/// Stan makr telegraficznych CW i prędkości kluczowania.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwMacroState {
    pub show_window: bool,
    /// Makra F1..F8 (zawsze [`CW_MACRO_COUNT`] pozycji).
    pub macros: Vec<String>,
}

impl Default for CwMacroState {
    fn default() -> Self {
        Self {
            show_window: false,
            macros: default_cw_macros(),
        }
    }
}

impl CwMacroState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Makra z zapisanej konfiguracji; brakujące uzupełniają się domyślnymi.
    pub fn from_config(cw: &crate::core::station::CwConfig) -> Self {
        let mut macros = cw.macros.clone();
        macros.truncate(CW_MACRO_COUNT);
        let defaults = default_cw_macros();
        while macros.len() < CW_MACRO_COUNT {
            macros.push(defaults[macros.len()].clone());
        }
        Self {
            show_window: false,
            macros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = CwMacroState::default();
        assert!(!state.show_window);
        assert_eq!(state.macros.len(), CW_MACRO_COUNT);
        assert_eq!(state.macros[0], "CQ TEST %MYCALL% %MYCALL% TEST");
        assert_eq!(state.macros[1], "%HISCALL% 5NN %SERIAL%");
        assert_eq!(state.macros[7], "?");
        assert_eq!(CwMacroState::new(), state);
    }

    #[test]
    fn config_with_missing_or_extra_macros_is_normalised() {
        let mut cw = crate::core::station::CwConfig::default();
        cw.macros = vec!["CQ".to_string()];
        let state = CwMacroState::from_config(&cw);
        assert_eq!(state.macros.len(), CW_MACRO_COUNT);
        assert_eq!(state.macros[0], "CQ");
        assert_eq!(state.macros[1], default_cw_macros()[1]);

        cw.macros = vec!["X".to_string(); 12];
        assert_eq!(CwMacroState::from_config(&cw).macros.len(), CW_MACRO_COUNT);
    }
}
