// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

/// Stan cyfrowego rejestratora i odtwarzacza głosu (Voice Keyer / DVK).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoiceKeyerState {
    pub show_window: bool,
    pub active: Option<usize>,
    pub recording: Option<usize>,
}

impl VoiceKeyerState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = VoiceKeyerState::default();
        assert!(!state.show_window);
        assert_eq!(state.active, None);
        assert_eq!(state.recording, None);

        let new_state = VoiceKeyerState::new();
        assert_eq!(new_state, state);
    }
}
