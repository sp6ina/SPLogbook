// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::station::VoiceKeyerMessage;

/// Stan cyfrowego rejestratora i odtwarzacza głosu (Voice Keyer / DVK).
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceKeyerState {
    pub show_window: bool,
    pub messages: Vec<VoiceKeyerMessage>,
    pub active: Option<usize>,
    pub recording: Option<usize>,
}

impl Default for VoiceKeyerState {
    fn default() -> Self {
        Self {
            show_window: false,
            messages: Vec::new(),
            active: None,
            recording: None,
        }
    }
}

impl VoiceKeyerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_messages(messages: Vec<VoiceKeyerMessage>) -> Self {
        Self {
            messages,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = VoiceKeyerState::default();
        assert!(!state.show_window);
        assert!(state.messages.is_empty());
        assert_eq!(state.active, None);
        assert_eq!(state.recording, None);

        let new_state = VoiceKeyerState::new();
        assert_eq!(new_state, state);
    }
}
