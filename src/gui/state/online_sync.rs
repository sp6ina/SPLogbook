// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Online synchronization UI state (LoTW, eQSL, Club Log, etc.).

use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, atomic::Ordering};

/// Strażnik znacznika „operacja w toku”: zwalnia flagę przy dowolnym zakończeniu zadania
/// (sukces, błąd lub anulowanie), więc przycisk w UI nie zostaje zablokowany na stałe.
#[derive(Debug)]
pub struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

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
    /// Trwa pobieranie potwierdzeń z LoTW (wyłącznie informacja zwrotna w UI).
    pub lotw_download_busy: Arc<AtomicBool>,
    /// Trwa pobieranie skrzynki odbiorczej eQSL (wyłącznie informacja zwrotna w UI).
    pub eqsl_download_busy: Arc<AtomicBool>,
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
            lotw_download_busy: Arc::new(AtomicBool::new(false)),
            eqsl_download_busy: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Oznacza pobieranie LoTW jako „w toku”; flaga wraca do `false` po zwolnieniu strażnika.
    #[must_use]
    pub fn begin_lotw_download(&self) -> BusyGuard {
        self.lotw_download_busy.store(true, Ordering::SeqCst);
        BusyGuard(Arc::clone(&self.lotw_download_busy))
    }

    /// Oznacza pobieranie eQSL jako „w toku”; flaga wraca do `false` po zwolnieniu strażnika.
    #[must_use]
    pub fn begin_eqsl_download(&self) -> BusyGuard {
        self.eqsl_download_busy.store(true, Ordering::SeqCst);
        BusyGuard(Arc::clone(&self.eqsl_download_busy))
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
        assert!(!state.lotw_download_busy.load(Ordering::SeqCst));
        assert!(!state.eqsl_download_busy.load(Ordering::SeqCst));
    }

    #[test]
    fn busy_guard_releases_flag_on_drop() {
        let state = OnlineSyncUiState::default();
        let guard = state.begin_lotw_download();
        assert!(state.lotw_download_busy.load(Ordering::SeqCst));
        assert!(!state.eqsl_download_busy.load(Ordering::SeqCst));
        drop(guard);
        assert!(!state.lotw_download_busy.load(Ordering::SeqCst));

        let guard = state.begin_eqsl_download();
        assert!(state.eqsl_download_busy.load(Ordering::SeqCst));
        drop(guard);
        assert!(!state.eqsl_download_busy.load(Ordering::SeqCst));
    }
}
