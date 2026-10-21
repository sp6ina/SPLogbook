// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::satellite::SatellitePass;

/// Stan podsystemu śledzenia satelitów amatorskich i obliczania Dopplera.
#[derive(Debug, Clone)]
pub struct SatelliteState {
    pub show_window: bool,
    pub selected_satellite: String,
    pub azimuth: f32,
    pub elevation: f32,
    pub range_km: f32,
    pub altitude_km: f32,
    pub downlink_mhz: f64,
    pub uplink_mhz: f64,
    pub rx_doppler_khz: f32,
    pub tx_doppler_khz: f32,
    pub auto_track_rotator: bool,
    pub auto_tune_radio: bool,
    pub passes: Vec<SatellitePass>,
    pub passes_sat: String,
    pub passes_grid: String,
    pub passes_minute: u64,
}

impl Default for SatelliteState {
    fn default() -> Self {
        Self {
            show_window: false,
            selected_satellite: "ISS (ZARYA)".to_string(),
            azimuth: 142.5,
            elevation: 38.2,
            range_km: 685.0,
            altitude_km: 418.0,
            downlink_mhz: 145.8000,
            uplink_mhz: 437.8000,
            rx_doppler_khz: -2.85,
            tx_doppler_khz: 8.52,
            auto_track_rotator: true,
            auto_tune_radio: true,
            passes: Vec::new(),
            passes_sat: String::new(),
            passes_grid: String::new(),
            passes_minute: u64::MAX,
        }
    }
}

impl SatelliteState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sprawdza, czy predykcja przelotów i parametry wymagają przeliczenia
    /// dla zadanego satelity, lokatora stacji i bieżącej minuty uniksowej.
    pub fn needs_refresh(&self, sat: &str, grid: &str, minute: u64) -> bool {
        self.passes_sat != sat || self.passes_grid != grid || self.passes_minute != minute
    }

    /// Aktualizuje klucze buforowania po odświeżeniu predykcji.
    pub fn mark_refreshed(&mut self, sat: &str, grid: &str, minute: u64) {
        self.passes_sat = sat.to_string();
        self.passes_grid = grid.to_string();
        self.passes_minute = minute;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = SatelliteState::default();
        assert!(!state.show_window);
        assert_eq!(state.selected_satellite, "ISS (ZARYA)");
        assert_eq!(state.azimuth, 142.5);
        assert_eq!(state.elevation, 38.2);
        assert_eq!(state.range_km, 685.0);
        assert_eq!(state.altitude_km, 418.0);
        assert_eq!(state.downlink_mhz, 145.8000);
        assert_eq!(state.uplink_mhz, 437.8000);
        assert_eq!(state.rx_doppler_khz, -2.85);
        assert_eq!(state.tx_doppler_khz, 8.52);
        assert!(state.auto_track_rotator);
        assert!(state.auto_tune_radio);
        assert!(state.passes.is_empty());
        assert_eq!(state.passes_sat, "");
        assert_eq!(state.passes_grid, "");
        assert_eq!(state.passes_minute, u64::MAX);

        let new_state = SatelliteState::new();
        assert_eq!(new_state.selected_satellite, state.selected_satellite);
    }

    #[test]
    fn test_needs_refresh() {
        let mut state = SatelliteState::default();
        assert!(state.needs_refresh("ISS (ZARYA)", "KO02md", 12345));

        state.mark_refreshed("ISS (ZARYA)", "KO02md", 12345);
        assert!(!state.needs_refresh("ISS (ZARYA)", "KO02md", 12345));

        // Zmiana minuty wymaga odświeżenia
        assert!(state.needs_refresh("ISS (ZARYA)", "KO02md", 12346));

        // Zmiana lokatora stacji wymaga odświeżenia
        assert!(state.needs_refresh("ISS (ZARYA)", "JO91qt", 12345));

        // Zmiana satelity wymaga odświeżenia
        assert!(state.needs_refresh("AO-91", "KO02md", 12345));
    }
}
