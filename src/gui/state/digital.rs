// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Digital modes runtime state (WSJT-X, JS8Call, FLDigi, PSK Reporter).

use std::sync::mpsc;

use crate::core::qso::QsoRecord;
use crate::digital::js8call::Js8CallState;
use crate::digital::wsjtx::WsjtxMessage;

/// Runtime channels and UI state for digital mode integrations.
pub struct DigitalModesState {
    pub wsjtx_tx: mpsc::Sender<WsjtxMessage>,
    pub wsjtx_rx: mpsc::Receiver<WsjtxMessage>,
    pub wsjtx_packets_count: u64,
    pub wsjtx_last_call: Option<String>,
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
