// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Application chrome state (status bar, toast notifications, palette, wizard, manual).

/// State for top/bottom chrome, toasts, manual, wizard, and command palette.
#[derive(Default)]
pub struct ChromeState {
    pub status_message: Option<String>,
    pub status_toast: Option<(String, std::time::Instant)>,
    pub show_welcome_wizard: bool,
    pub wizard_tab: u8,
    pub show_about_window: bool,
    pub show_shortcuts_window: bool,
    pub show_legend_window: bool,
    pub show_command_palette: bool,
    pub command_palette_query: String,
    pub command_palette_selected: usize,
    pub show_user_manual: bool,
    pub manual_section: Option<String>,
    pub manual_selected: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = ChromeState::default();
        assert!(state.status_message.is_none());
        assert!(state.status_toast.is_none());
        assert!(!state.show_welcome_wizard);
        assert_eq!(state.wizard_tab, 0);
        assert!(!state.show_about_window);
        assert!(!state.show_shortcuts_window);
        assert!(!state.show_legend_window);
        assert!(!state.show_command_palette);
        assert!(state.command_palette_query.is_empty());
        assert_eq!(state.command_palette_selected, 0);
        assert!(!state.show_user_manual);
        assert!(state.manual_section.is_none());
        assert_eq!(state.manual_selected, 0);
    }
}
