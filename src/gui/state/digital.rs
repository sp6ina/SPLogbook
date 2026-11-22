// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Digital modes runtime state (WSJT-X, JS8Call, FLDigi, PSK Reporter).

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::mpsc;

use crate::core::qso::QsoRecord;
use crate::digital::js8call::Js8CallState;
use crate::digital::wsjtx::{
    WsjtxDecode, WsjtxMark, WsjtxMessage, WsjtxOutgoing, WsjtxSettings, WsjtxStatus,
};

/// Najwięcej dekodów trzymanych w oknie (najnowsze na początku).
pub const MAX_WSJTX_DECODES: usize = 300;

/// Dekod z oceną stacji nadawcy w chwili odbioru.
#[derive(Debug, Clone)]
pub struct WsjtxDecodeRow {
    pub decode: WsjtxDecode,
    /// Znak nadawcy (wielkie litery), jeśli komunikat go zawiera.
    pub call: Option<String>,
    /// Pasmo i emisja instancji w chwili dekodu (z ostatniego pakietu Status).
    pub band: Option<String>,
    pub mode: String,
    pub mark: WsjtxMark,
}

/// Dwukierunkowe połączenie z WSJT-X / JTDX (STAGE-V140-S8).
#[derive(Debug, Default)]
pub struct WsjtxLinkState {
    /// Kanał poleceń do zadania sieciowego; `None`, gdy nasłuch jest wyłączony.
    pub out: Option<tokio::sync::mpsc::UnboundedSender<WsjtxOutgoing>>,
    pub link: Option<tokio::task::JoinHandle<()>>,
    /// Ostatni stan każdej instancji po identyfikatorze.
    pub clients: BTreeMap<String, WsjtxStatus>,
    pub decodes: VecDeque<WsjtxDecodeRow>,
    /// Wyróżnienia wysłane do WSJT-X: (instancja, znak) → rodzaj.
    pub marks: HashMap<(String, String), WsjtxMark>,
    /// Stan nagród się zmienił (nowe QSO) — oceny i wyróżnienia trzeba przeliczyć.
    pub marks_stale: bool,
    pub show_window: bool,
    pub only_cq: bool,
    /// Edytowane ustawienia; trafiają do konfiguracji dopiero po zastosowaniu.
    pub draft: Option<WsjtxSettings>,
}

impl WsjtxLinkState {
    /// `true`, gdy zadanie nasłuchu działa.
    pub fn is_listening(&self) -> bool {
        self.link.as_ref().is_some_and(|link| !link.is_finished())
    }
}

/// Runtime channels and UI state for digital mode integrations.
pub struct DigitalModesState {
    pub wsjtx_tx: mpsc::Sender<WsjtxMessage>,
    pub wsjtx_rx: mpsc::Receiver<WsjtxMessage>,
    pub wsjtx_packets_count: u64,
    pub wsjtx_last_call: Option<String>,
    pub wsjtx: WsjtxLinkState,
    pub js8call_enabled: bool,
    pub js8call_host: String,
    pub js8call_port: u16,
    pub js8call_state: Js8CallState,
    pub js8call_state_rx: Option<mpsc::Receiver<Js8CallState>>,
    pub js8call_qso_rx: Option<mpsc::Receiver<QsoRecord>>,
    pub fldigi_test_result: Option<String>,
    pub show_pskreporter_window: bool,
}

impl Default for DigitalModesState {
    fn default() -> Self {
        Self::new()
    }
}

impl DigitalModesState {
    pub fn new() -> Self {
        let (wsjtx_tx, wsjtx_rx) = mpsc::channel();
        Self {
            wsjtx_tx,
            wsjtx_rx,
            wsjtx_packets_count: 0,
            wsjtx_last_call: None,
            wsjtx: WsjtxLinkState::default(),
            js8call_enabled: false,
            js8call_host: "127.0.0.1".to_string(),
            js8call_port: 2442,
            js8call_state: Js8CallState::default(),
            js8call_state_rx: None,
            js8call_qso_rx: None,
            fldigi_test_result: None,
            show_pskreporter_window: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = DigitalModesState::default();
        assert_eq!(state.wsjtx_packets_count, 0);
        assert!(state.wsjtx_last_call.is_none());
        assert!(state.wsjtx.out.is_none() && !state.wsjtx.is_listening());
        assert!(state.wsjtx.decodes.is_empty() && state.wsjtx.marks.is_empty());
        assert!(!state.js8call_enabled);
        assert_eq!(state.js8call_host, "127.0.0.1");
        assert_eq!(state.js8call_port, 2442);
        assert!(!state.js8call_state.connected);
        assert!(state.js8call_state_rx.is_none());
        assert!(state.js8call_qso_rx.is_none());
        assert!(state.fldigi_test_result.is_none());
        assert!(!state.show_pskreporter_window);
    }
}
