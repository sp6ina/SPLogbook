// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Online synchronization UI state (LoTW, eQSL, Club Log, etc.).

use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, atomic::Ordering};

/// State for the online synchronization window and log channel.
#[derive(Debug)]
pub struct OnlineSyncUiState {
    pub show_online_sync_window: bool,
    pub online_sync_logs: Vec<String>,
    pub sync_log_tx: mpsc::Sender<(String, bool)>,
    pub sync_log_rx: mpsc::Receiver<(String, bool)>,
    /// Flaga anulowania bieżącej wysyłki do serwisu online.
    ///
    /// Anulowanie zatrzymuje kolejne paczki, ale nigdy nie zamienia braku
    /// dowodu akceptacji w sukces.
    pub upload_cancel: Arc<AtomicBool>,
}

impl OnlineSyncUiState {
    pub fn new() -> Self {
        let (sync_log_tx, sync_log_rx) = mpsc::channel();
        Self {
            show_online_sync_window: false,
            online_sync_logs: Vec::new(),
            sync_log_tx,
            sync_log_rx,
            upload_cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Żąda anulowania trwającej wysyłki (jeżeli jakaś trwa).
    pub fn request_upload_cancel(&self) {
        self.upload_cancel.store(true, Ordering::SeqCst);
    }

    /// Czy trwa (lub została zlecona) wysyłka do anulowania.
    pub fn upload_cancel_requested(&self) -> bool {
        self.upload_cancel.load(Ordering::SeqCst)
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
        assert_eq!(state.online_sync_logs.len(), 0);
        assert!(state.sync_log_rx.try_recv().is_err());
    }
}
