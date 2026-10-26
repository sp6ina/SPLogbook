// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::station::CustomContest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContestState {
    pub show_window: bool,
    pub name: String,
    pub stx: u32,
    pub show_custom_editor: bool,
    pub custom_edit_idx: Option<usize>,
    pub custom_draft: CustomContest,
}

impl Default for ContestState {
    fn default() -> Self {
        Self {
            show_window: false,
            name: "SP DX Contest".to_string(),
            stx: 1,
            show_custom_editor: false,
            custom_edit_idx: None,
            custom_draft: CustomContest::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = ContestState::default();
        assert!(!state.show_window);
        assert_eq!(state.name, "SP DX Contest");
        assert_eq!(state.stx, 1);
        assert!(!state.show_custom_editor);
        assert!(state.custom_edit_idx.is_none());
        assert_eq!(state.custom_draft, CustomContest::default());
    }
}
