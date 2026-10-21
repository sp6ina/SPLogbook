// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::station::{AppConfig, ViewPanelConfig};

/// Stan konfiguracji widoków i dokowania 9 paneli aplikacji.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewsState {
    pub panel_vfo: ViewPanelConfig,
    pub panel_qso: ViewPanelConfig,
    pub panel_log: ViewPanelConfig,
    pub panel_cluster: ViewPanelConfig,
    pub panel_solar: ViewPanelConfig,
    pub panel_bandmap: ViewPanelConfig,
    pub panel_satellites: ViewPanelConfig,
    pub panel_world_map: ViewPanelConfig,
    pub panel_waterfall: ViewPanelConfig,
}

impl Default for ViewsState {
    fn default() -> Self {
        Self {
            panel_vfo: ViewPanelConfig::default(),
            panel_qso: ViewPanelConfig::default(),
            panel_log: ViewPanelConfig::default(),
            panel_cluster: ViewPanelConfig::default(),
            panel_solar: ViewPanelConfig::default(),
            panel_bandmap: ViewPanelConfig::default(),
            panel_satellites: ViewPanelConfig::default(),
            panel_world_map: ViewPanelConfig::default(),
            panel_waterfall: ViewPanelConfig::default(),
        }
    }
}

impl ViewsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            panel_vfo: config.panel_vfo.clone(),
            panel_qso: config.panel_qso.clone(),
            panel_log: config.panel_log.clone(),
            panel_cluster: config.panel_cluster.clone(),
            panel_solar: config.panel_solar.clone(),
            panel_bandmap: config.panel_bandmap.clone(),
            panel_satellites: config.panel_satellites.clone(),
            panel_world_map: config.panel_world_map.clone(),
            panel_waterfall: config.panel_waterfall.clone(),
        }
    }

    /// Oblicza łączną liczbę widocznych paneli.
    pub fn visible_count(&self) -> usize {
        (self.panel_vfo.visible as usize)
            + (self.panel_qso.visible as usize)
            + (self.panel_log.visible as usize)
            + (self.panel_cluster.visible as usize)
            + (self.panel_bandmap.visible as usize)
            + (self.panel_solar.visible as usize)
            + (self.panel_satellites.visible as usize)
            + (self.panel_world_map.visible as usize)
            + (self.panel_waterfall.visible as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = ViewsState::default();
        let default_panel = ViewPanelConfig::default();
        assert_eq!(state.panel_vfo, default_panel);
        assert_eq!(state.panel_qso, default_panel);
        assert_eq!(state.panel_log, default_panel);
        assert_eq!(state.panel_cluster, default_panel);
        assert_eq!(state.panel_solar, default_panel);
        assert_eq!(state.panel_bandmap, default_panel);
        assert_eq!(state.panel_satellites, default_panel);
        assert_eq!(state.panel_world_map, default_panel);
        assert_eq!(state.panel_waterfall, default_panel);

        let new_state = ViewsState::new();
        assert_eq!(new_state, state);
        assert_eq!(state.visible_count(), 9); // default panels have visible: true
    }
}
