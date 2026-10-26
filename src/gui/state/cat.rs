// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::hamlib::RigState;
use crate::cat::rotor::RotorState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatConnection {
    #[default]
    Serial,
    Tcp,
}

impl CatConnection {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatConnection::Serial => "serial",
            CatConnection::Tcp => "tcp",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VfoState {
    pub split_offset_khz: f64,
    pub filter_preset: String,
    pub preamp_att: String,
    pub nb_nr: bool,
    pub agc_speed: String,
}

impl Default for VfoState {
    fn default() -> Self {
        Self {
            split_offset_khz: 1.0,
            filter_preset: "Normal".to_string(),
            preamp_att: "OFF".to_string(),
            nb_nr: false,
            agc_speed: "FAST".to_string(),
        }
    }
}

pub struct CatState {
    pub rig_state: RigState,
    pub rotor_state: RotorState,
    pub connected: bool,
    pub vfo_split: bool,
    pub test_result: Option<String>,
    pub rotor_test_result: Option<String>,
    pub tci_test_result: Option<String>,
    pub model_search: String,
    pub rigctld_supervisor: Option<crate::cat::supervisor::RigctldSupervisor>,
    pub state_tx: std::sync::mpsc::Sender<crate::cat::hamlib::RigState>,
    pub state_rx: std::sync::mpsc::Receiver<crate::cat::hamlib::RigState>,
    pub ptt_active: bool,
    pub mfg_selected: String,
    pub conn_type: CatConnection,
    pub sharing_active: bool,
    pub shared_state: std::sync::Arc<std::sync::RwLock<crate::cat::hamlib::RigState>>,
    pub proxy_server: Option<std::sync::Arc<crate::cat::server::HamlibProxyServer>>,
    pub proxy_rx: Option<tokio::sync::broadcast::Receiver<crate::cat::server::RigServerCommand>>,
    pub poll_abort: Option<tokio::task::AbortHandle>,
    pub runtime: crate::cat::runtime::CatRuntime,
    pub probe_rx: Option<
        std::sync::mpsc::Receiver<(
            u64,
            Result<crate::cat::backend::RigProbeResult, crate::cat::rig::RigError>,
        )>,
    >,
    pub probe_running: bool,
    pub probe_result:
        Option<Result<crate::cat::backend::RigProbeResult, crate::cat::rig::RigError>>,
    pub probe_state: crate::cat::probe::ProbeState,
    pub probe_generation: u64,
    pub probe_task: Option<tokio::task::JoinHandle<()>>,
    pub start_rx: Option<
        std::sync::mpsc::Receiver<
            Result<crate::cat::runtime::CatRuntime, crate::cat::runtime::CatRuntimeError>,
        >,
    >,
    pub selected_rig_idx: usize,
    pub form_cache: crate::gui::cat_settings::FormVariantCache,
    pub vfo: VfoState,
}

impl Default for CatState {
    fn default() -> Self {
        let (state_tx, state_rx) = std::sync::mpsc::channel();
        Self {
            rig_state: RigState::default(),
            rotor_state: RotorState::default(),
            connected: false,
            vfo_split: false,
            test_result: None,
            rotor_test_result: None,
            tci_test_result: None,
            model_search: String::new(),
            rigctld_supervisor: None,
            state_tx,
            state_rx,
            ptt_active: false,
            mfg_selected: String::new(),
            conn_type: CatConnection::default(),
            sharing_active: false,
            shared_state: std::sync::Arc::new(std::sync::RwLock::new(RigState::default())),
            proxy_server: None,
            proxy_rx: None,
            poll_abort: None,
            runtime: crate::cat::runtime::CatRuntime::default(),
            probe_rx: None,
            probe_running: false,
            probe_result: None,
            probe_state: crate::cat::probe::ProbeState::Idle,
            probe_generation: 0,
            probe_task: None,
            start_rx: None,
            selected_rig_idx: 0,
            form_cache: crate::gui::cat_settings::FormVariantCache::default(),
            vfo: VfoState::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cat_state_default_is_disconnected_neutral() {
        let state = CatState::default();
        assert!(!state.connected);
        assert!(!state.ptt_active);
        assert!(!state.vfo_split);
        assert_eq!(state.conn_type, CatConnection::Serial);
        assert!(state.test_result.is_none());
        assert!(state.rotor_test_result.is_none());
        assert!(state.tci_test_result.is_none());
        assert!(state.proxy_server.is_none());
        assert!(state.poll_abort.is_none());
        assert!(!state.probe_running);
        assert_eq!(state.probe_generation, 0);
        assert_eq!(state.rig_state.frequency_hz, 14_025_000);
        assert_eq!(state.rotor_state.azimuth_deg, 0.0);
        assert_eq!(state.rotor_state.elevation_deg, 0.0);
        assert_eq!(state.vfo.split_offset_khz, 1.0);
    }
}
