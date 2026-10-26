// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Aggregated top-level GUI state.

use super::appearance::AppearanceState;
use super::chrome::ChromeState;
use super::dialogs::DialogState;
use super::windows::WindowState;
use super::workspace::DockWorkspace;

/// Composite state representing the entire graphical interface.
#[derive(Default)]
pub struct GuiState {
    pub chrome: ChromeState,
    pub appearance: AppearanceState,
    pub dialogs: DialogState,
    pub workspace: DockWorkspace,
    pub windows: WindowState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let gui = GuiState::default();
        assert!(!gui.dialogs.journal_dialog.is_open);
        assert!(!gui.chrome.show_about_window);
        assert_eq!(gui.appearance.language, crate::core::i18n::Language::Pl);
        assert_eq!(
            gui.appearance.theme_preset,
            crate::gui::theme::ThemePreset::OperatorDark
        );
        assert!(!gui.windows.show_bandmap_window);
        assert!(gui.windows.show_vfo_panel);
    }
}
