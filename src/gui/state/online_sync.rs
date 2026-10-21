// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Online synchronization UI state (LoTW, eQSL, Club Log, etc.).

use std::sync::mpsc;

/// State for the online synchronization window and log channel.
#[derive(Debug)]
pub struct OnlineSyncUiState {
    pub show_online_sync_window: bool,
    pub online_sync_logs: Vec<String>,
    pub sync_log_tx: mpsc::Sender<(String, bool)>,
    pub sync_log_rx: mpsc::Receiver<(String, bool)>,
}

impl OnlineSyncUiState {
    pub fn new() -> Self {
        let (sync_log_tx, sync_log_rx) = mpsc::channel();
        Self {
            show_online_sync_window: false,
            online_sync_logs: Vec::new(),
            sync_log_tx,
            sync_log_rx,
        }
    }
}

impl Default for OnlineSyncUiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = OnlineSyncUiState::default();
        assert!(!state.show_online_sync_window);
        assert!(state.online_sync_logs.is_empty());
        assert!(state.sync_log_rx.try_recv().is_err());
    }
}
