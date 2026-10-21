// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

/// Stan makr telegraficznych CW i prędkości kluczowania.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwMacroState {
    pub show_window: bool,
    pub wpm: u32,
    pub macro_f1: String,
    pub macro_f2: String,
    pub macro_f3: String,
    pub macro_f4: String,
    pub macro_f5: String,
    pub macro_f6: String,
    pub macro_f7: String,
    pub macro_f8: String,
}

impl Default for CwMacroState {
    fn default() -> Self {
        Self {
            show_window: false,
            wpm: 28,
            macro_f1: "CQ TEST %MYCALL% %MYCALL% TEST".to_string(),
            macro_f2: "%HISCALL% 5NN %SERIAL%".to_string(),
            macro_f3: "TU %MYCALL% TEST".to_string(),
            macro_f4: "%MYCALL%".to_string(),
            macro_f5: "%HISCALL%".to_string(),
            macro_f6: "QTH %MYQTH% LOC %MYLOC%".to_string(),
            macro_f7: "%SERIAL%".to_string(),
            macro_f8: "?".to_string(),
        }
    }
}

impl CwMacroState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = CwMacroState::default();
        assert!(!state.show_window);
        assert_eq!(state.wpm, 28);
        assert_eq!(state.macro_f1, "CQ TEST %MYCALL% %MYCALL% TEST");
        assert_eq!(state.macro_f2, "%HISCALL% 5NN %SERIAL%");
        assert_eq!(state.macro_f3, "TU %MYCALL% TEST");
        assert_eq!(state.macro_f4, "%MYCALL%");
        assert_eq!(state.macro_f5, "%HISCALL%");
        assert_eq!(state.macro_f6, "QTH %MYQTH% LOC %MYLOC%");
        assert_eq!(state.macro_f7, "%SERIAL%");
        assert_eq!(state.macro_f8, "?");

        let new_state = CwMacroState::new();
        assert_eq!(new_state, state);
    }
}
