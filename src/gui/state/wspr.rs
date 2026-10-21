// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::wspr::{WsprError, WsprSpot};
use std::sync::{Arc, Mutex};

pub type WsprFetchSlot = Arc<Mutex<Option<Result<Vec<WsprSpot>, WsprError>>>>;

/// Stan monitora WSPR (wspr.live).
#[derive(Debug, Clone)]
pub struct WsprState {
    pub show_window: bool,
    pub spots: Vec<WsprSpot>,
    pub loading: bool,
    pub last_error: Option<String>,
    pub fetch_slot: Option<WsprFetchSlot>,
    pub distance_miles: bool,
}

impl Default for WsprState {
    fn default() -> Self {
        Self {
            show_window: false,
            spots: Vec::new(),
            loading: false,
            last_error: None,
            fetch_slot: None,
            distance_miles: false,
        }
    }
}

impl WsprState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = WsprState::default();
        assert!(!state.show_window);
        assert!(state.spots.is_empty());
        assert!(!state.loading);
        assert_eq!(state.last_error, None);
        assert!(state.fetch_slot.is_none());
        assert!(!state.distance_miles);

        let new_state = WsprState::new();
        assert_eq!(new_state.show_window, state.show_window);
        assert_eq!(new_state.loading, state.loading);
        assert_eq!(new_state.distance_miles, state.distance_miles);
    }
}
