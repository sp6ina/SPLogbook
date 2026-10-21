// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::plugins::marketplace::{MarketplaceError, PluginCatalogEntry};
use std::sync::mpsc::Receiver;

pub struct MarketplaceState {
    pub show_window: bool,
    pub catalog: Vec<PluginCatalogEntry>,
    pub loading: bool,
    pub error: Option<String>,
    pub search: String,
    pub category: Option<String>,
    pub busy_id: Option<String>,
    pub status: Option<String>,
    pub fetch_rx: Option<Receiver<Result<Vec<PluginCatalogEntry>, MarketplaceError>>>,
    pub install_rx: Option<Receiver<(String, Result<(), MarketplaceError>)>>,
}

impl Default for MarketplaceState {
    fn default() -> Self {
        Self {
            show_window: false,
            catalog: Vec::new(),
            loading: false,
            error: None,
            search: String::new(),
            category: None,
            busy_id: None,
            status: None,
            fetch_rx: None,
            install_rx: None,
        }
    }
}

impl MarketplaceState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = MarketplaceState::default();
        assert!(!state.show_window);
        assert!(state.catalog.is_empty());
        assert!(!state.loading);
        assert!(state.error.is_none());
        assert!(state.search.is_empty());
        assert!(state.category.is_none());
        assert!(state.busy_id.is_none());
        assert!(state.status.is_none());
        assert!(state.fetch_rx.is_none());
        assert!(state.install_rx.is_none());

        let new_state = MarketplaceState::new();
        assert!(!new_state.show_window);
        assert!(!new_state.loading);
    }
}
