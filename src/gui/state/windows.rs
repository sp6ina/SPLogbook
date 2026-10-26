// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Window visibility and window-specific UI state.

/// Visibility flags and transient state for secondary and floating windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowState {
    pub show_bandmap_window: bool,
    pub bandmap_selected_band: String,
    pub bandmap_auto_track: bool,
    pub show_vfo_panel: bool,
    pub show_cluster_panel: bool,
    pub show_cat_settings_window: bool,
    pub show_workspace_profiles_window: bool,
    pub show_multi_op_window: bool,
    pub show_awards_matrix_window: bool,
    pub awards_matrix_tab: usize,
    pub awards_other_subtab: usize,
    pub awards_other_search: String,
    pub show_quick_access_customizer: bool,
    pub show_column_settings: bool,
    pub column_preset_name: String,
    pub show_operator_assistant: bool,
    pub show_station_profiles_window: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            show_bandmap_window: false,
            bandmap_selected_band: "20m".to_string(),
            bandmap_auto_track: true,
            show_vfo_panel: true,
            show_cluster_panel: true,
            show_cat_settings_window: false,
            show_workspace_profiles_window: false,
            show_multi_op_window: false,
            show_awards_matrix_window: false,
            awards_matrix_tab: 0,
            awards_other_subtab: 0,
            awards_other_search: String::new(),
            show_quick_access_customizer: false,
            show_column_settings: false,
            column_preset_name: String::new(),
            show_operator_assistant: false,
            show_station_profiles_window: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let ws = WindowState::default();
        assert!(!ws.show_bandmap_window);
        assert_eq!(ws.bandmap_selected_band, "20m");
        assert!(ws.bandmap_auto_track);
        assert!(ws.show_vfo_panel);
        assert!(ws.show_cluster_panel);
        assert!(!ws.show_cat_settings_window);
        assert!(!ws.show_workspace_profiles_window);
        assert!(!ws.show_multi_op_window);
        assert!(!ws.show_awards_matrix_window);
        assert_eq!(ws.awards_matrix_tab, 0);
        assert_eq!(ws.awards_other_subtab, 0);
        assert_eq!(ws.awards_other_search, "");
        assert!(!ws.show_quick_access_customizer);
        assert!(!ws.show_column_settings);
        assert_eq!(ws.column_preset_name, "");
        assert!(!ws.show_operator_assistant);
        assert!(!ws.show_station_profiles_window);
    }
}
