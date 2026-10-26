// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::sync::{SyncEvent, SyncService};

/// Stan interfejsu użytkownika synchronizacji Multi-Op.
pub struct SyncUiState {
    pub show: bool,
    pub is_host: bool,
    pub status: String,
    pub log: Vec<String>,
    pub service: Option<SyncService>,
    pub events_tx: std::sync::mpsc::Sender<SyncEvent>,
    pub events_rx: std::sync::mpsc::Receiver<SyncEvent>,
}

impl Default for SyncUiState {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncUiState {
    pub fn new() -> Self {
        let (events_tx, events_rx) = std::sync::mpsc::channel();
        Self {
            show: false,
            is_host: true,
            status: "Off".to_string(),
            log: Vec::new(),
            service: None,
            events_tx,
            events_rx,
        }
    }

    /// Dopisuje wpis do dziennika zdarzeń GUI, ograniczając historię do 500 wpisów.
    pub fn push_log(&mut self, entry: impl Into<String>) {
        self.log.push(entry.into());
        if self.log.len() > 500 {
            let excess = self.log.len() - 500;
            self.log.drain(0..excess);
        }
    }

    /// Zatrzymuje działający serwis synchronizacji.
    pub fn stop(&mut self) {
        if let Some(mut srv) = self.service.take() {
            srv.stop();
        }
    }

    /// Sprawdza, czy serwis synchronizacji jest aktywny.
    pub fn is_running(&self) -> bool {
        self.service.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_ui_state_default() {
        let state = SyncUiState::default();
        assert!(!state.show);
        assert!(state.is_host);
        assert_eq!(state.status, "Off");
        assert!(state.log.is_empty());
        assert!(state.service.is_none());
        assert!(!state.is_running());
    }
}
