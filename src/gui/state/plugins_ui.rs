// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::plugins::bridge::PluginLookupResult;
use std::sync::mpsc::Receiver;

/// Stan interfejsu menedżera wtyczek (Rhai) oraz kanał asynchronicznych lookupów.
#[derive(Debug)]
pub struct PluginUiState {
    pub show_manager: bool,
    pub log: Vec<String>,
    pub notifications: Vec<String>,
    pub lookup_rx: Option<Receiver<PluginLookupResult>>,
}

impl Default for PluginUiState {
    fn default() -> Self {
        Self {
            show_manager: false,
            log: Vec::new(),
            notifications: Vec::new(),
            lookup_rx: None,
        }
    }
}

impl PluginUiState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = PluginUiState::default();
        assert!(!state.show_manager);
        assert!(state.log.is_empty());
        assert!(state.notifications.is_empty());
        assert!(state.lookup_rx.is_none());

        let new_state = PluginUiState::new();
        assert_eq!(new_state.show_manager, state.show_manager);
        assert_eq!(new_state.log.len(), state.log.len());
    }
}
