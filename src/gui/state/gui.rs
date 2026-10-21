// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Aggregated top-level GUI state.

use super::dialogs::DialogState;

/// Composite state representing the entire graphical interface.
#[derive(Default)]
pub struct GuiState {
    pub dialogs: DialogState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let gui = GuiState::default();
        assert!(!gui.dialogs.journal_dialog.is_open);
    }
}
