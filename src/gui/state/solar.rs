// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::solar::{SpaceWeather, SpaceWeatherError};
use crate::core::propagation::BandOpeningStatus;
use crate::core::propagation_history::PropagationHistory;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type SolarFetchSlot = Arc<Mutex<Option<Result<SpaceWeather, SpaceWeatherError>>>>;

/// Od czego zależą prognozy pasm: wskaźniki, czas (z dokładnością do minuty),
/// lokator operatora i lokator korespondenta (pusty = kontynenty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolarCacheKey {
    pub sfi: u32,
    pub k_index: u32,
    pub hour: u32,
    pub minute: u32,
    pub day_of_year: u32,
    pub my_locator: String,
    pub dx_locator: String,
}

pub type SolarCacheEntry = (SolarCacheKey, Vec<crate::core::propagation::BandOutlook>);

/// Stan modułu pogody kosmicznej i prognoz propagacyjnych (HamQTH / VOACAP-lite).
#[derive(Debug, Clone)]
pub struct SolarState {
    /// Ostatnie pobrane dane; `None`, dopóki pobranie się nie uda.
    pub space_weather: Option<SpaceWeather>,
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
            space_weather: None,
            history: Vec::new(),
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

    /// SFI i indeks K z prawdziwych danych; `None`, gdy danych jeszcze nie ma albo
    /// źródło nie podało SFI. Prognozy propagacji bez tych danych nie są liczone.
    pub fn indices(&self) -> Option<(u32, u32)> {
        self.space_weather
            .as_ref()
            .filter(|w| w.sfi > 0)
            .map(|w| (w.sfi, w.k_index))
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
        assert!(
            state.space_weather.is_none(),
            "no invented values before a fetch"
        );
        assert!(state.indices().is_none());
        assert_eq!(state.history.len(), 0);
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
