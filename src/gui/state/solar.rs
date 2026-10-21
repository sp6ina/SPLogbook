// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::solar::{SpaceWeather, SpaceWeatherError};
use crate::core::propagation::{BandOpeningStatus, PropagationForecast};
use crate::core::propagation_history::PropagationHistory;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type SolarFetchSlot = Arc<Mutex<Option<Result<SpaceWeather, SpaceWeatherError>>>>;

pub type SolarCacheKey = (u32, u32, u32, u32, u32);
pub type SolarCacheEntry = (SolarCacheKey, Vec<(&'static str, PropagationForecast)>);

/// Stan modułu pogody kosmicznej i prognoz propagacyjnych (HamQTH / VOACAP-lite).
#[derive(Debug, Clone)]
pub struct SolarState {
    pub space_weather: SpaceWeather,
    pub history: Vec<SpaceWeather>,
    pub loading: bool,
    pub last_alert: Option<String>,
    pub fetch_slot: Option<SolarFetchSlot>,
    pub propagation_cache: Option<SolarCacheEntry>,
    pub propagation_history: PropagationHistory,
    pub previous_band_status: HashMap<String, BandOpeningStatus>,
    pub band_alert_enabled: bool,
    pub band_alert_k_index_threshold: u8,
    pub show_panel: bool,
    pub show_propagation_explain: bool,
}

impl Default for SolarState {
    fn default() -> Self {
        Self {
            space_weather: SpaceWeather::default(),
            history: vec![SpaceWeather::default()],
            loading: false,
            last_alert: None,
            fetch_slot: None,
            propagation_cache: None,
            propagation_history: PropagationHistory::default(),
            previous_band_status: HashMap::new(),
            band_alert_enabled: true,
            band_alert_k_index_threshold: 4,
            show_panel: true,
            show_propagation_explain: false,
        }
    }
}

impl SolarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_propagation_history(propagation_history: PropagationHistory) -> Self {
        Self {
            propagation_history,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = SolarState::default();
        assert_eq!(state.space_weather.sfi, 140);
        assert_eq!(state.history.len(), 1);
        assert!(!state.loading);
        assert!(state.last_alert.is_none());
        assert!(state.fetch_slot.is_none());
        assert!(state.propagation_cache.is_none());
        assert!(state.previous_band_status.is_empty());
        assert!(state.band_alert_enabled);
        assert_eq!(state.band_alert_k_index_threshold, 4);
        assert!(state.show_panel);
        assert!(!state.show_propagation_explain);

        let new_state = SolarState::new();
        assert_eq!(new_state.show_panel, state.show_panel);
        assert_eq!(new_state.band_alert_enabled, state.band_alert_enabled);
    }
}
