// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny backend FlexRadio SmartSDR API implementujący [`RigBackend`].
//!
//! Zapewnia obsługę protokołu TCP/IP SmartSDR:
//! - asynchroniczny handshake (V i H),
//! - numerację sekwencyjną komend i korelacje odpowiedzi,
//! - obsługę wieloobiektowych statusów asynchronicznych (slice, transmit, interlock),
//! - model odbiorników slice z deterministycznym wyborem aktywnego i nadawczego slice,
//! - wysokie bezpieczeństwo PTT (brak retry, pojedyncza próba wyłączenia TX przy rozłączeniu),
//! - całkowicie read-only probe.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use crate::cat::backend::{CatBackendKind, RigBackend};
use crate::cat::native::NativeRigEvent;
use crate::cat::native::profile::RigModelProfile;
use crate::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};

use super::client::{FlexRadioConfig, FlexRadioConnection, HandshakeState};
use super::models::{FLEX_GENERIC, find_flex_profile};
use super::protocol::{FlexCommand, FlexInboundMessage, FlexProtocolError, FlexSequenceAllocator};
use super::slice::{
    FlexSliceId, FlexSliceState, select_read_slice, select_tx_slice, select_write_slice,
};

/// Maksymalna liczba jednocześnie oczekujących na odpowiedź komend.
pub const MAX_PENDING_COMMANDS: usize = 64;

/// Domyślny limit czasu oczekiwania na potwierdzenie pojedynczej komendy.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_millis(1500);

/// Rejestr komend oczekujących na odpowiedź z radia.
#[derive(Debug)]
pub struct PendingCommandRegistry {
    max_capacity: usize,
    pending: BTreeMap<u32, tokio::time::Instant>,
}

impl Default for PendingCommandRegistry {
    fn default() -> Self {
        Self::new(MAX_PENDING_COMMANDS)
    }
}

impl PendingCommandRegistry {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            max_capacity,
            pending: BTreeMap::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.max_capacity
    }

    pub fn contains(&self, seq: u32) -> bool {
        self.pending.contains_key(&seq)
    }

    /// Rejestruje wysłaną komendę z jej numerem sekwencyjnym.
    pub fn register(&mut self, seq: u32) -> Result<(), FlexProtocolError> {
        self.cleanup_expired(DEFAULT_COMMAND_TIMEOUT);

        if self.pending.len() >= self.max_capacity {
            return Err(FlexProtocolError::QueueFull);
        }

        if self.pending.contains_key(&seq) {
            return Err(FlexProtocolError::InvalidSequence(format!(
                "sequence C{seq} is already pending"
            )));
        }

        self.pending.insert(seq, tokio::time::Instant::now());
        Ok(())
    }

    /// Oznacza odpowiedź jako odebraną i usuwa komendę z rejestru.
    pub fn complete(&mut self, seq: u32) -> bool {
        self.pending.remove(&seq).is_some()
    }

    /// Usuwa przeterminowane wpisy.
    pub fn cleanup_expired(&mut self, timeout: Duration) {
        let now = tokio::time::Instant::now();
        self.pending
            .retain(|_, time| now.duration_since(*time) < timeout);
    }

    /// Czyści wszystkie oczekujące komendy (np. przy rozłączeniu).
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }
}

/// Potwierdzony stan PTT sesji SmartSDR jako stan trójwartościowy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexConfirmedPttState {
    /// Brak potwierdzenia, timeout, utrata łącza lub stan przejściowy/nieznany.
    #[default]
    Unknown,
    /// Status jednoznacznie potwierdził odbiór (RX).
    Rx,
    /// Status i odpowiedź na komendę jednoznacznie potwierdziły nadawanie tej sesji (TX).
    Tx,
}

/// Klasyfikacja subskrypcji kontrolnych SmartSDR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexSubscriptionRequirement {
    /// Wymagana do działania backendu CAT (odrzucenie blokuje connect).
    Required,
    /// Opcjonalna telemetryczna/pomocnicza (odrzucenie nie blokuje connect).
    Optional,
}

/// Specyfikacja subskrypcji API SmartSDR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlexSubscriptionSpec {
    pub name: &'static str,
    pub command: &'static str,
    pub min_major_version: u32,
    pub requirement: FlexSubscriptionRequirement,
}

pub const CONTROL_SUBSCRIPTIONS: [FlexSubscriptionSpec; 5] = [
    FlexSubscriptionSpec {
        name: "slice",
        command: "sub slice all",
        min_major_version: 1,
        requirement: FlexSubscriptionRequirement::Required,
    },
    FlexSubscriptionSpec {
        name: "interlock",
        command: "sub interlock all",
        min_major_version: 1,
        requirement: FlexSubscriptionRequirement::Required,
    },
    FlexSubscriptionSpec {
        name: "tx",
        command: "sub tx all",
        min_major_version: 1,
        requirement: FlexSubscriptionRequirement::Optional,
    },
    FlexSubscriptionSpec {
        name: "radio",
        command: "sub radio all",
        min_major_version: 1,
        requirement: FlexSubscriptionRequirement::Optional,
    },
    FlexSubscriptionSpec {
        name: "client",
        command: "sub client all",
        min_major_version: 2,
        requirement: FlexSubscriptionRequirement::Optional,
    },
];

/// Minimalny zestaw subskrypcji wyłącznie w warstwie sterowania (control plane).
pub const MINIMAL_CONTROL_SUBSCRIPTIONS: [&str; 5] = [
    "sub slice all",
    "sub tx all",
    "sub interlock all",
    "sub radio all",
    "sub client all",
];

/// Natywny backend FlexRadio SmartSDR API.
pub struct FlexRadioBackend {
    config: FlexRadioConfig,
    connection: Option<FlexRadioConnection>,
    profile: &'static RigModelProfile,
    forced_model: bool,
    seq_allocator: FlexSequenceAllocator,
    pending_registry: PendingCommandRegistry,
    slices: BTreeMap<FlexSliceId, FlexSliceState>,
    client_handle: Option<String>,
    protocol_version: Option<String>,
    interlock_state: String,
    tx_client_handle: Option<String>,
    confirmed_ptt_state: FlexConfirmedPttState,
    ptt_initiated_by_this_session: bool,
    ptt_request_pending: bool,
    last_xmit_seq: Option<u32>,
    xmit_command_responded: bool,
    runtime_slice_count: Option<u32>,
    rf_power_watts: f32,
    events: VecDeque<NativeRigEvent>,
    last_state: RigState,
}

impl FlexRadioBackend {
    /// Tworzy nową instancję backendu z konfiguracją i profilem modelu.
    pub fn new(config: FlexRadioConfig, model_name: Option<&str>) -> Self {
        let (profile, forced_model) = match model_name {
            Some(name) => (find_flex_profile(name), true),
            None => (&FLEX_GENERIC, false),
        };

        Self {
            config,
            connection: None,
            profile,
            forced_model,
            seq_allocator: FlexSequenceAllocator::new(),
            pending_registry: PendingCommandRegistry::default(),
            slices: BTreeMap::new(),
            client_handle: None,
            protocol_version: None,
            interlock_state: "READY".to_string(),
            tx_client_handle: None,
            confirmed_ptt_state: FlexConfirmedPttState::Unknown,
            ptt_initiated_by_this_session: false,
            ptt_request_pending: false,
            last_xmit_seq: None,
            xmit_command_responded: false,
            runtime_slice_count: None,
            rf_power_watts: 0.0,
            events: VecDeque::new(),
            last_state: RigState::default(),
        }
    }

    /// Zwraca informację, czy bieżąca sesja zainicjowała i potwierdziła nadawanie (PTT).
    pub fn is_ptt_initiated_by_this_session(&self) -> bool {
        self.ptt_initiated_by_this_session
    }

    /// Zwraca bieżący uchwyt klienta SmartSDR.
    pub fn client_handle(&self) -> Option<&str> {
        self.client_handle.as_deref()
    }

    /// Zwraca wersję protokołu SmartSDR API.
    pub fn protocol_version(&self) -> Option<&str> {
        self.protocol_version.as_deref()
    }

    /// Zwraca mapę aktualnych odbiorników slice.
    pub fn slices(&self) -> &BTreeMap<FlexSliceId, FlexSliceState> {
        &self.slices
    }

    /// Zwraca bieżący trójwartościowy stan PTT.
    pub fn confirmed_ptt_state(&self) -> FlexConfirmedPttState {
        self.confirmed_ptt_state
    }

    /// Zwraca informację, czy odbiór (RX) został jednoznacznie potwierdzony.
    pub fn is_rx_confirmed(&self) -> bool {
        self.confirmed_ptt_state == FlexConfirmedPttState::Rx
    }

    /// Zwraca informację, czy nadawanie (TX) zostało jednoznacznie potwierdzone dla naszej sesji.
    pub fn is_tx_confirmed(&self) -> bool {
        self.confirmed_ptt_state == FlexConfirmedPttState::Tx
    }

    /// Zwraca potwierdzony stan PTT (true tylko dla jednoznacznego TX tej sesji).
    pub fn is_ptt_confirmed(&self) -> bool {
        self.is_tx_confirmed()
    }

    /// Zwraca zaobserwowaną w runtime liczbę slice'ów (z odpowiedzi komendy `info`).
    pub fn runtime_slice_count(&self) -> Option<u32> {
        self.runtime_slice_count
    }

    /// Przeprowadza ściśle read-only próbkowanie transceivera (probe).
    /// Nawiązuje TCP, odczytuje V i H, nie modyfikuje slice ani PTT, zamyka połączenie.
    pub async fn probe(&mut self) -> Result<(), RigError> {
        let endpoint = format!("{}:{}", self.config.host, self.config.port);
        let mut conn = FlexRadioConnection::connect(self.config.clone())
            .await
            .map_err(|e| e.to_rig_error(&endpoint))?;

        let (ver, handle) = conn
            .perform_handshake()
            .await
            .map_err(|e| e.to_rig_error(&endpoint))?;

        self.protocol_version = Some(ver);
        self.client_handle = Some(handle);

        conn.disconnect().await;
        Ok(())
    }

    /// Wysyła komendę z przydzielonym numerem sekwencyjnym i rejestruje w pending commands.
    async fn send_command_tracked(
        &mut self,
        cmd_text: impl Into<String>,
    ) -> Result<u32, FlexProtocolError> {
        let conn = self
            .connection
            .as_mut()
            .ok_or(FlexProtocolError::NotConnected)?;

        self.pending_registry
            .cleanup_expired(DEFAULT_COMMAND_TIMEOUT);
        if self.pending_registry.len() >= self.pending_registry.capacity() {
            return Err(FlexProtocolError::QueueFull);
        }

        let seq = self
            .seq_allocator
            .allocate_avoiding(|seq| self.pending_registry.contains(seq))?;
        let cmd = FlexCommand::new(seq, cmd_text);

        self.pending_registry.register(seq)?;

        if let Err(e) = conn.send_command(&cmd).await {
            self.pending_registry.complete(seq);
            return Err(e);
        }

        Ok(seq)
    }

    /// Oczekuje na odpowiedź o zadanym numerze sekwencyjnym,
    /// przetwarzając w międzyczasie inne nadeszłe komunikaty i statusy.
    pub async fn wait_for_response(
        &mut self,
        seq: u32,
        timeout: Duration,
    ) -> Result<super::protocol::FlexResponse, FlexProtocolError> {
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }

            let messages = {
                let conn = self
                    .connection
                    .as_mut()
                    .ok_or(FlexProtocolError::NotConnected)?;
                match tokio::time::timeout(remaining, conn.read_messages()).await {
                    Ok(res) => res?,
                    Err(_) => break,
                }
            };

            let mut found_response = None;

            for msg in messages {
                match msg {
                    FlexInboundMessage::Response(resp) => {
                        self.pending_registry.complete(resp.seq);
                        if Some(resp.seq) == self.last_xmit_seq && resp.is_success() {
                            self.xmit_command_responded = true;
                            self.recompute_ptt_state();
                        }
                        if resp.seq == seq {
                            found_response = Some(resp);
                        }
                    }
                    FlexInboundMessage::Status(status) => {
                        self.process_status_message(&status);
                    }
                    FlexInboundMessage::Version(v) => {
                        self.protocol_version = Some(v);
                    }
                    FlexInboundMessage::Handle(h) => {
                        self.client_handle = Some(h);
                    }
                    FlexInboundMessage::Message(_) | FlexInboundMessage::Unknown(_) => {}
                }
            }

            if let Some(resp) = found_response {
                return Ok(resp);
            }
        }

        Err(FlexProtocolError::Timeout {
            operation: "wait_for_response",
        })
    }

    /// Przelicza stan potwierdzonego PTT na podstawie interlock_state, tx_client_handle,
    /// ptt_request_pending oraz xmit_command_responded.
    fn recompute_ptt_state(&mut self) {
        if self.interlock_state == "TRANSMITTING" {
            let matches_client = match (&self.client_handle, &self.tx_client_handle) {
                (Some(my_h), Some(tx_h)) => my_h.eq_ignore_ascii_case(tx_h),
                _ => false,
            };

            if matches_client && self.ptt_request_pending && self.xmit_command_responded {
                self.ptt_initiated_by_this_session = true;
                self.confirmed_ptt_state = FlexConfirmedPttState::Tx;
            } else {
                self.ptt_initiated_by_this_session = false;
                self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
            }
        } else if self.interlock_state == "READY" || self.interlock_state == "RECEIVE" {
            if self.ptt_request_pending {
                // Lokalne żądanie PTT (xmit 1) jest w toku i oczekuje na potwierdzenie przejścia w TX przez radio.
                // Nie raportujemy przedwcześnie potwierdzonego RX ani nie czyścimy żądania.
                self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
                self.ptt_initiated_by_this_session = false;
            } else {
                self.confirmed_ptt_state = FlexConfirmedPttState::Rx;
                self.ptt_initiated_by_this_session = false;
                self.xmit_command_responded = false;
            }
        } else {
            self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
            self.ptt_initiated_by_this_session = false;
        }
    }

    /// Przetwarza nadeszłe komunikaty i aktualizuje stan wewnętrzny backendu.
    fn process_inbound_messages(&mut self, messages: Vec<FlexInboundMessage>) {
        for msg in messages {
            match msg {
                FlexInboundMessage::Response(resp) => {
                    self.pending_registry.complete(resp.seq);
                    if Some(resp.seq) == self.last_xmit_seq && resp.is_success() {
                        self.xmit_command_responded = true;
                        self.recompute_ptt_state();
                    }
                }
                FlexInboundMessage::Status(status) => {
                    self.process_status_message(&status);
                }
                FlexInboundMessage::Version(v) => {
                    self.protocol_version = Some(v);
                }
                FlexInboundMessage::Handle(h) => {
                    self.client_handle = Some(h);
                }
                FlexInboundMessage::Message(_) | FlexInboundMessage::Unknown(_) => {}
            }
        }
    }

    /// Przetwarza pojedynczy asynchroniczny komunikat statusu.
    fn process_status_message(&mut self, status: &super::protocol::FlexStatus) {
        match status.object_class.as_str() {
            "slice" => {
                if let Some(slice_id) = status
                    .object_id
                    .as_deref()
                    .and_then(|s| s.parse::<u32>().ok().map(FlexSliceId))
                {
                    if status.properties.get("in_use").is_some_and(|v| v == "0") {
                        self.slices.remove(&slice_id);
                        return;
                    }

                    let entry = self
                        .slices
                        .entry(slice_id)
                        .or_insert_with(|| FlexSliceState::new(slice_id));

                    let _ = entry.update_from_properties(&status.properties);
                }
            }
            "interlock" => {
                if let Some(state) = status.properties.get("state") {
                    self.interlock_state = state.to_ascii_uppercase();
                }
                if let Some(tx_handle) = status.properties.get("tx_client_handle") {
                    let clean = tx_handle.strip_prefix("0x").unwrap_or(tx_handle);
                    self.tx_client_handle = Some(clean.to_string());
                }

                self.recompute_ptt_state();
            }
            "transmit" => {
                if let Some(p) = status
                    .properties
                    .get("rfpower")
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    self.rf_power_watts = p;
                }
            }
            _ => {}
        }
    }

    /// Buduje ujednolicony domenowy `RigState` z aktualnego aktywnego slice.
    fn build_rig_state(&self) -> RigState {
        let active_slice = select_read_slice(
            &self.slices,
            self.config.preferred_slice,
            self.client_handle.as_deref(),
        )
        .ok();

        let tx_slice = select_tx_slice(&self.slices, self.client_handle.as_deref()).ok();

        let (frequency_hz, mode, passband_hz, rit_hz, xit_hz, vfo) = match active_slice {
            Some(s) => (
                s.frequency_hz,
                s.mode.clone(),
                s.passband_hz(),
                s.rit_freq_hz,
                s.xit_freq_hz,
                format!("Slice {}", s.id),
            ),
            None => (
                14_000_000,
                "USB".to_string(),
                2700,
                0,
                0,
                "Slice 0".to_string(),
            ),
        };

        let split_enabled = match (active_slice, tx_slice) {
            (Some(rx), Some(tx)) => rx.id != tx.id,
            _ => false,
        };

        let tx_frequency_hz = if split_enabled {
            tx_slice.map(|s| s.frequency_hz)
        } else {
            None
        };

        let tx_vfo = match tx_slice {
            Some(s) => format!("Slice {}", s.id),
            None => vfo.clone(),
        };

        RigState {
            frequency_hz,
            mode,
            passband_hz,
            vfo,
            tx_vfo,
            split_enabled,
            tx_frequency_hz,
            s_meter_dbm: -100.0,
            s_meter_unit: "S1".to_string(),
            rf_power_watts: self.rf_power_watts,
            ptt: self.confirmed_ptt_state == FlexConfirmedPttState::Tx,
            rit_hz,
            xit_hz,
            connected: self
                .connection
                .as_ref()
                .is_some_and(|c| c.state() == HandshakeState::Ready),
        }
    }

    /// Odbiera oczekujące zdarzenia natywne.
    pub fn drain_events(&mut self) -> Vec<NativeRigEvent> {
        self.events.drain(..).collect()
    }
}

impl RigBackend for FlexRadioBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::FlexRadio
    }

    fn capabilities(&self) -> RigCapabilities {
        self.profile.capabilities
    }

    async fn connect(&mut self) -> Result<(), RigError> {
        let endpoint = format!("{}:{}", self.config.host, self.config.port);

        let mut conn = FlexRadioConnection::connect(self.config.clone())
            .await
            .map_err(|e| e.to_rig_error(&endpoint))?;

        let (ver, handle) = conn
            .perform_handshake()
            .await
            .map_err(|e| e.to_rig_error(&endpoint))?;

        self.protocol_version = Some(ver);
        self.client_handle = Some(handle);
        self.connection = Some(conn);

        // 1. Wyślij komendę info i zweryfikuj tożsamość radia
        let info_seq = self
            .send_command_tracked("info")
            .await
            .map_err(|e| e.to_rig_error(&endpoint))?;
        let info_resp = self
            .wait_for_response(info_seq, DEFAULT_COMMAND_TIMEOUT)
            .await
            .map_err(|e| e.to_rig_error(&endpoint))?;

        if !info_resp.is_success() {
            return Err(RigError::Protocol {
                detail: format!(
                    "info command rejected with code {}: {}",
                    info_resp.code, info_resp.message
                ),
            });
        }

        let (_, info_props) = super::protocol::tokenize_smart_sdr_fields(&info_resp.message);

        // Walidacja modelu z info
        if let Some(ref exp_model) = self.config.expected_model {
            let actual_model = info_props.get("model").cloned().unwrap_or_default();
            if !actual_model.eq_ignore_ascii_case(exp_model) {
                return Err(FlexProtocolError::ModelMismatch {
                    expected: exp_model.clone(),
                    actual: actual_model,
                }
                .to_rig_error(&endpoint));
            }
        }

        // Walidacja chassis_serial / serial z info
        if let Some(ref exp_serial) = self.config.expected_serial {
            let actual_serial = info_props
                .get("chassis_serial")
                .or_else(|| info_props.get("serial"))
                .cloned()
                .unwrap_or_default();
            if !actual_serial.eq_ignore_ascii_case(exp_serial) {
                return Err(FlexProtocolError::SerialMismatch {
                    expected: exp_serial.clone(),
                    actual: actual_serial,
                }
                .to_rig_error(&endpoint));
            }
        }

        // Runtime slice observation
        if let Some(slice_count) = info_props.get("slice").and_then(|s| s.parse::<u32>().ok()) {
            self.runtime_slice_count = Some(slice_count);
        }

        // Jeśli model jest znany w info i obecny profil to FLEX_GENERIC, zaktualizuj profil
        if let Some(actual_model) = info_props.get("model").filter(|_| !self.forced_model) {
            let identified_profile = find_flex_profile(actual_model);
            if identified_profile != &FLEX_GENERIC {
                self.profile = identified_profile;
            }
        }

        // 2. Jeśli jawnie skonfigurowano client_id, wykonaj client bind
        if let Some(ref client_id) = self.config.client_id {
            let bind_cmd = format!("client bind client_id={client_id}");
            let bind_seq = self
                .send_command_tracked(bind_cmd)
                .await
                .map_err(|e| e.to_rig_error(&endpoint))?;
            let bind_resp = self
                .wait_for_response(bind_seq, DEFAULT_COMMAND_TIMEOUT)
                .await
                .map_err(|e| e.to_rig_error(&endpoint))?;

            if !bind_resp.is_success() {
                return Err(RigError::Protocol {
                    detail: format!(
                        "client bind failed with code {}: {}",
                        bind_resp.code, bind_resp.message
                    ),
                });
            }
        }

        // 3. Subskrybuj zestaw subskrypcji control plane
        let api_major = self
            .protocol_version
            .as_deref()
            .and_then(|v| v.split('.').next())
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);

        for spec in &CONTROL_SUBSCRIPTIONS {
            if api_major < spec.min_major_version {
                // Nieobsługiwana w danej wersji API (np. sub client all w SmartSDR v1.x)
                continue;
            }

            let sub_seq = self
                .send_command_tracked(spec.command)
                .await
                .map_err(|e| e.to_rig_error(&endpoint))?;
            let sub_resp = self
                .wait_for_response(sub_seq, DEFAULT_COMMAND_TIMEOUT)
                .await
                .map_err(|e| e.to_rig_error(&endpoint))?;

            if !sub_resp.is_success() {
                match spec.requirement {
                    FlexSubscriptionRequirement::Required => {
                        return Err(FlexProtocolError::SubscriptionRejected {
                            subject: spec.command.to_string(),
                            code: sub_resp.code,
                            message: sub_resp.message,
                        }
                        .to_rig_error(&endpoint));
                    }
                    FlexSubscriptionRequirement::Optional => {
                        // Odrzucenie subskrypcji opcjonalnej nie blokuje połączenia
                    }
                }
            }
        }

        // Ustaw początkowy potwierdzony stan RX
        self.confirmed_ptt_state = FlexConfirmedPttState::Rx;
        self.last_state.connected = true;

        Ok(())
    }

    async fn poll(&mut self) -> Result<RigState, RigError> {
        let endpoint = format!("{}:{}", self.config.host, self.config.port);

        let messages = {
            let conn = self.connection.as_mut().ok_or(RigError::NotConnected)?;

            match conn.read_messages().await {
                Ok(m) => m,
                Err(e) => {
                    self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
                    return Err(e.to_rig_error(&endpoint));
                }
            }
        };

        self.process_inbound_messages(messages);
        let new_state = self.build_rig_state();

        if new_state != self.last_state {
            self.events
                .push_back(NativeRigEvent::StateChanged(new_state.clone()));
            self.last_state = new_state.clone();
        }

        Ok(new_state)
    }

    async fn execute(&mut self, command: &RigCommand) -> Result<(), RigError> {
        let endpoint = format!("{}:{}", self.config.host, self.config.port);

        if !self.capabilities().supports(command) {
            let cmd_name = match command {
                RigCommand::SetVfo(_) => "SetVfo",
                RigCommand::SetRit(_) => "SetRit",
                RigCommand::SetXit(_) => "SetXit",
                RigCommand::SetPassband(_) => "SetPassband",
                RigCommand::SetPtt(_) => "SetPtt",
                RigCommand::SetPower(_) => "SetPower",
                RigCommand::SetSplit { .. } => "SetSplit",
                _ => "unsupported_command",
            };
            return Err(RigError::Unsupported {
                backend: CatBackendKind::FlexRadio,
                command: cmd_name,
            });
        }

        match command {
            RigCommand::SetFrequency(freq_hz) => {
                let slice = select_write_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
                    false,
                )
                .map_err(|e| e.to_rig_error(&endpoint))?;

                let mhz = *freq_hz as f64 / 1_000_000.0;
                let cmd = format!("slice set {} freq={:.6}", slice.id, mhz);

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetSplitFrequency(freq_hz) => {
                let slice = select_tx_slice(&self.slices, self.client_handle.as_deref())
                    .map_err(|e| e.to_rig_error(&endpoint))?;

                let mhz = *freq_hz as f64 / 1_000_000.0;
                let cmd = format!("slice set {} freq={:.6}", slice.id, mhz);

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetMode { mode, passband_hz } => {
                let slice = select_write_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
                    false,
                )
                .map_err(|e| e.to_rig_error(&endpoint))?;

                let m = mode.trim().to_ascii_uppercase();
                let (lo, hi) = match m.as_str() {
                    "LSB" | "DIGL" => (-100i32 - *passband_hz as i32, -100i32),
                    "CW" | "AM" | "SAM" | "FM" | "NFM" | "DFM" => {
                        let half = (*passband_hz / 2) as i32;
                        (-half, half)
                    }
                    _ => (100i32, 100i32 + *passband_hz as i32),
                };

                let cmd = format!(
                    "slice set {} mode={} filter_lo={} filter_hi={}",
                    slice.id, m, lo, hi
                );

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetPassband(passband_hz) => {
                let slice = select_write_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
                    false,
                )
                .map_err(|e| e.to_rig_error(&endpoint))?;

                let m = slice.mode.as_str();
                let (lo, hi) = match m {
                    "LSB" | "DIGL" => (
                        slice.filter_high_hz - *passband_hz as i32,
                        slice.filter_high_hz,
                    ),
                    "CW" | "AM" | "SAM" => {
                        let half = (*passband_hz / 2) as i32;
                        (-half, half)
                    }
                    _ => (
                        slice.filter_low_hz,
                        slice.filter_low_hz + *passband_hz as i32,
                    ),
                };

                let cmd = format!("slice set {} filter_lo={} filter_hi={}", slice.id, lo, hi);

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetSplit { enabled, tx_vfo: _ } => {
                if *enabled && self.slices.len() < 2 {
                    return Err(RigError::Protocol {
                        detail: "split requires at least 2 active slices".to_string(),
                    });
                }
            }
            RigCommand::SetPtt(true) => {
                // Bezpieczeństwo PTT: pojedyncze wysłanie polecenia xmit 1 bez ponawiania (no retry)
                self.ptt_request_pending = true;
                self.xmit_command_responded = false;

                let seq = match self.send_command_tracked("xmit 1").await {
                    Ok(s) => s,
                    Err(e) => {
                        self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
                        self.ptt_initiated_by_this_session = false;
                        self.ptt_request_pending = false;
                        return Err(e.to_rig_error(&endpoint));
                    }
                };
                self.last_xmit_seq = Some(seq);

                match self.wait_for_response(seq, self.config.write_timeout).await {
                    Ok(resp) => {
                        if resp.is_success() {
                            self.xmit_command_responded = true;
                            self.recompute_ptt_state();
                        } else {
                            self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
                            self.ptt_initiated_by_this_session = false;
                            self.ptt_request_pending = false;
                            return Err(RigError::Protocol {
                                detail: format!(
                                    "xmit 1 rejected with code {}: {}",
                                    resp.code, resp.message
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
                        self.ptt_initiated_by_this_session = false;
                        self.ptt_request_pending = false;
                        return Err(e.to_rig_error(&endpoint));
                    }
                }
            }
            RigCommand::SetPtt(false) => {
                self.ptt_request_pending = false;
                self.xmit_command_responded = false;

                let seq = self
                    .send_command_tracked("xmit 0")
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;

                let _ = self.wait_for_response(seq, self.config.write_timeout).await;
            }
            RigCommand::SetPower(watts) => {
                let power = watts.round().clamp(0.0, 100.0) as u32;
                let cmd = format!("transmit set rfpower={power}");

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetVfo(_) => {
                return Err(RigError::Unsupported {
                    backend: CatBackendKind::FlexRadio,
                    command: "SetVfo",
                });
            }
            RigCommand::SetRit(rit_hz) => {
                let slice = select_write_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
                    false,
                )
                .map_err(|e| e.to_rig_error(&endpoint))?;

                let (on, freq) = if *rit_hz != 0 { (1, *rit_hz) } else { (0, 0) };
                let cmd = format!("slice set {} rit_on={on} rit_freq={freq}", slice.id);

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetXit(xit_hz) => {
                let slice = select_write_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
                    false,
                )
                .map_err(|e| e.to_rig_error(&endpoint))?;

                let (on, freq) = if *xit_hz != 0 { (1, *xit_hz) } else { (0, 0) };
                let cmd = format!("slice set {} xit_on={on} xit_freq={freq}", slice.id);

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
        }

        Ok(())
    }

    async fn disconnect(&mut self) {
        // Bezpieczeństwo PTT w środowisku wieloklientowym:
        // Pojedynczą próbę xmit 0 wysyłamy TYLKO wtedy, gdy to nasza sesja zainicjowała
        // nadawanie i stan jest potwierdzony jako Tx (ptt_initiated_by_this_session == true && confirmed_ptt_state == Tx).
        // Nigdy nie wyłączamy nadawania należącego do innego klienta ani stanu nieznanego!
        if let Some(conn) = self.connection.as_mut().filter(|_| {
            self.ptt_initiated_by_this_session
                && self.confirmed_ptt_state == FlexConfirmedPttState::Tx
        }) {
            let cmd = FlexCommand::new(self.seq_allocator.next_seq(), "xmit 0");
            let _ = tokio::time::timeout(Duration::from_millis(500), conn.send_command(&cmd)).await;
        }
        self.ptt_initiated_by_this_session = false;
        self.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
        self.ptt_request_pending = false;
        self.xmit_command_responded = false;
        self.last_xmit_seq = None;

        // Anulowanie oczekujących poleceń przed zamknięciem gniazda
        self.pending_registry.clear();

        // Standardowe, bezpieczne zakończenie własnej sesji przez zamknięcie TcpStream
        if let Some(mut conn) = self.connection.take() {
            conn.disconnect().await;
        }
        self.slices.clear();
        self.client_handle = None;
        self.protocol_version = None;
        self.interlock_state = "READY".to_string();
        self.tx_client_handle = None;
        self.last_state.connected = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;

    async fn setup_mock_flex_radio() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
        let port = listener.local_addr().expect("port mock").port();
        (listener, port)
    }

    #[allow(clippy::unused_async)]
    async fn spawn_standard_mock_radio(
        listener: TcpListener,
        recorded: Arc<Mutex<Vec<String>>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();

            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if line.is_empty() {
                        continue;
                    }
                    recorded.lock().await.push(line.clone());
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!("R{seq}|0|model=FLEX-6600 chassis_serial=1234-5678-9012-3456 slice=4\n")
                                        .as_bytes(),
                                )
                                .await;
                        } else if cmd.contains("xmit 1") {
                            let _ = socket
                                .write_all(
                                    format!("R{seq}|0|\nS0123ABCD|interlock state=TRANSMITTING tx_client_handle=0x0123ABCD\n")
                                        .as_bytes(),
                                )
                                .await;
                        } else if cmd.contains("xmit 0") {
                            let _ = socket
                                .write_all(
                                    format!("R{seq}|0|\nS0123ABCD|interlock state=READY\n")
                                        .as_bytes(),
                                )
                                .await;
                        } else if cmd.starts_with("client disconnect") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                            return;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        })
    }

    #[tokio::test]
    async fn flex_backend_connects_and_completes_handshake() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect failed");

        assert_eq!(backend.protocol_version(), Some("3.7.4"));
        assert_eq!(backend.client_handle(), Some("0123ABCD"));

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_backend_does_not_mark_connected_before_ready() {
        let (listener, port) = setup_mock_flex_radio().await;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            // Wysyłamy tylko V bez H, po czym zamykamy
            socket.write_all(b"V3.7.4\n").await.expect("write v");
            tokio::time::sleep(Duration::from_millis(50)).await;
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            connect_timeout: Duration::from_millis(100),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        let res = backend.connect().await;
        assert!(res.is_err());
        assert!(!backend.build_rig_state().connected);

        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_backend_probe_is_read_only() {
        let (listener, port) = setup_mock_flex_radio().await;

        let received_commands = Arc::new(Mutex::new(Vec::<String>::new()));
        let cmd_recorder = received_commands.clone();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            if let Ok(n) = socket.read(&mut buf).await {
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                if !text.is_empty() {
                    cmd_recorder.lock().await.push(text);
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.probe().await.expect("probe failed");

        assert_eq!(backend.protocol_version(), Some("3.7.4"));
        assert_eq!(backend.client_handle(), Some("0123ABCD"));

        server.await.expect("server join");

        let cmds = received_commands.lock().await;
        for c in cmds.iter() {
            assert!(!c.contains("slice"));
            assert!(!c.contains("xmit"));
            assert!(!c.contains("transmit"));
        }
    }

    #[tokio::test]
    async fn flex_backend_connect_never_sends_tx() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6400"));
        backend.connect().await.expect("connect");

        backend.disconnect().await;
        let _ = server.await;

        let cmds = recorded.lock().await;
        for cmd in cmds.iter() {
            assert!(!cmd.contains("xmit 1"));
            assert!(!cmd.contains("tx 1"));
        }
    }

    #[tokio::test]
    async fn flex_backend_poll_processes_status() {
        let (listener, port) = setup_mock_flex_radio().await;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6400 chassis_serial=1234 slice=2\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd.starts_with("sub ") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                            if cmd == "sub client all" {
                                let _ = socket
                                    .write_all(
                                        b"S0123ABCD|slice 0 in_use=1 active=1 mode=CW RF_frequency=14.025000 filter_lo=-250 filter_hi=250\n",
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6400"));
        backend.connect().await.expect("connect");

        let state = backend.poll().await.expect("poll failed");
        assert_eq!(state.frequency_hz, 14_025_000);
        assert_eq!(state.mode, "CW");
        assert_eq!(state.passband_hz, 500);
        assert!(state.connected);

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_backend_execute_frequency_targets_selected_slice() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.client_handle = Some("0123ABCD".to_string());
        backend.slices.insert(FlexSliceId(0), s0);

        backend
            .execute(&RigCommand::SetFrequency(14_074_000))
            .await
            .expect("execute failed");

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(
            cmds.iter()
                .any(|c| c.contains("slice set 0 freq=14.074000"))
        );
    }

    #[tokio::test]
    async fn flex_backend_execute_mode_targets_selected_slice() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.client_handle = Some("0123ABCD".to_string());
        backend.slices.insert(FlexSliceId(0), s0);

        backend
            .execute(&RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            })
            .await
            .expect("execute failed");

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(
            cmds.iter()
                .any(|c| c.contains("slice set 0 mode=CW filter_lo=-250 filter_hi=250"))
        );
    }

    #[tokio::test]
    async fn flex_backend_execute_filter_targets_selected_slice() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.mode = "USB".to_string();
        s0.filter_low_hz = 100;
        s0.filter_high_hz = 2800;
        s0.client_handle = Some("0123ABCD".to_string());
        backend.slices.insert(FlexSliceId(0), s0);

        backend
            .execute(&RigCommand::SetPassband(2400))
            .await
            .expect("execute failed");

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(
            cmds.iter()
                .any(|c| c.contains("slice set 0 filter_lo=100 filter_hi=2500"))
        );
    }

    #[tokio::test]
    async fn flex_backend_execute_ptt() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("ptt true failed");

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(cmds.iter().any(|c| c.contains("xmit 1")));
    }

    #[tokio::test]
    async fn flex_backend_ptt_true_is_not_retried() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("ptt");

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        let ptt_count = cmds.iter().filter(|c| c.contains("xmit 1")).count();
        assert_eq!(ptt_count, 1);
    }

    #[tokio::test]
    async fn flex_backend_ptt_requires_confirmed_status() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.client_handle = Some("0123ABCD".to_string());

        // Przed statusem interlock
        assert!(!backend.is_ptt_confirmed());

        // Symulacja statusu interlock: READY
        let mut status_ready = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status_ready
            .properties
            .insert("state".to_string(), "READY".to_string());
        backend.process_status_message(&status_ready);
        assert!(!backend.is_ptt_confirmed());

        // Symulacja statusu interlock: TRANSMITTING, ale brak ptt_request_pending
        let mut status_tx_foreign = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status_tx_foreign
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        status_tx_foreign
            .properties
            .insert("tx_client_handle".to_string(), "09999999".to_string());
        backend.process_status_message(&status_tx_foreign);
        assert!(!backend.is_ptt_confirmed());

        // Symulacja żądania z tej sesji i potwierdzenia dla naszego client_handle
        backend.ptt_request_pending = true;
        backend.xmit_command_responded = true;
        let mut status_tx_owned = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status_tx_owned
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        status_tx_owned
            .properties
            .insert("tx_client_handle".to_string(), "0123ABCD".to_string());
        backend.process_status_message(&status_tx_owned);

        assert!(backend.is_ptt_confirmed());
        assert!(backend.is_ptt_initiated_by_this_session());
    }

    #[tokio::test]
    async fn flex_backend_rejects_ambiguous_slice() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        // Dwa aktywne slice bez przypisania do bieżącego klienta
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        let mut s1 = FlexSliceState::new(FlexSliceId(1));
        s1.active = true;

        backend.slices.insert(FlexSliceId(0), s0);
        backend.slices.insert(FlexSliceId(1), s1);

        let err = backend
            .execute(&RigCommand::SetFrequency(14_074_000))
            .await
            .expect_err("ambiguous");
        assert!(matches!(err, RigError::Protocol { .. }));

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_backend_rejects_unsupported_command() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));

        let err = backend
            .execute(&RigCommand::SetVfo("VFOB".to_string()))
            .await
            .expect_err("unsupported");
        assert!(matches!(
            err,
            RigError::Unsupported {
                command: "SetVfo",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn flex_backend_timeout_is_typed() {
        let (listener, port) = setup_mock_flex_radio().await;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            // Po zaakceptowaniu socketu nie wysyłamy nic i czekamy na timeout
            tokio::time::sleep(Duration::from_millis(200)).await;
            let _ = socket.write_all(b"").await;
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            connect_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        let err = backend.connect().await.expect_err("should time out");
        assert!(matches!(err, RigError::Timeout { .. }));

        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_backend_disconnect_is_idempotent() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));

        backend.disconnect().await;
        backend.disconnect().await;
        assert!(!backend.build_rig_state().connected);
    }

    #[tokio::test]
    async fn flex_backend_disconnect_tx_release_is_single_and_bounded() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        // Symulacja aktywnego PTT przed rozłączeniem (zainicjowanego przez tę sesję)
        backend.ptt_initiated_by_this_session = true;
        backend.confirmed_ptt_state = FlexConfirmedPttState::Tx;

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        let xmit0_count: usize = cmds.iter().map(|c| c.matches("xmit 0").count()).sum();
        assert_eq!(xmit0_count, 1);
    }

    #[tokio::test]
    async fn flex_backend_partial_reads_preserve_data() {
        let (listener, port) = setup_mock_flex_radio().await;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd.starts_with("sub ") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                            if cmd == "sub client all" {
                                let _ = socket
                                    .write_all(
                                        b"S0123ABCD|slice 0 in_use=1 active=1 mode=USB RF_freq",
                                    )
                                    .await;
                                tokio::time::sleep(Duration::from_millis(30)).await;
                                let _ = socket.write_all(b"uency=14.200000\n").await;
                            }
                        } else if cmd.starts_with("client disconnect") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                            return;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        let _ = backend.poll().await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        let state = backend.poll().await.expect("poll");
        assert_eq!(state.frequency_hz, 14_200_000);

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_backend_interleaves_response_and_status() {
        let (listener, port) = setup_mock_flex_radio().await;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd.starts_with("sub ") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        // Symulacja przeplatanych komunikatów w buforze
        let interleaved_msg1 = FlexInboundMessage::Status(super::super::protocol::FlexStatus {
            client_handle: Some("0123ABCD".to_string()),
            object_class: "slice".to_string(),
            object_id: Some("0".to_string()),
            properties: {
                let mut map = BTreeMap::new();
                map.insert("in_use".to_string(), "1".to_string());
                map.insert("active".to_string(), "1".to_string());
                map.insert("mode".to_string(), "USB".to_string());
                map.insert("rf_frequency".to_string(), "14.100000".to_string());
                map
            },
        });
        let interleaved_msg2 = FlexInboundMessage::Status(super::super::protocol::FlexStatus {
            client_handle: Some("0123ABCD".to_string()),
            object_class: "transmit".to_string(),
            object_id: None,
            properties: {
                let mut map = BTreeMap::new();
                map.insert("rfpower".to_string(), "75".to_string());
                map
            },
        });

        backend.process_inbound_messages(vec![interleaved_msg1, interleaved_msg2]);
        let state = backend.build_rig_state();
        assert_eq!(state.frequency_hz, 14_100_000);
        assert_eq!(state.rf_power_watts, 75.0);

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[test]
    fn flex_backend_pending_registry_is_bounded() {
        let mut reg = PendingCommandRegistry::new(2);
        assert!(reg.register(1).is_ok());
        assert!(reg.register(2).is_ok());
        // Trzecia komenda przekracza limit
        assert_eq!(reg.register(3), Err(FlexProtocolError::QueueFull));

        // Po zakończeniu komendy 1, zwolnione miejsce pozwala na rejestrację
        assert!(reg.complete(1));
        assert!(reg.register(3).is_ok());
    }

    #[test]
    fn flex_backend_disconnect_clears_pending_commands() {
        let mut reg = PendingCommandRegistry::default();
        reg.register(1).expect("reg 1");
        reg.register(2).expect("reg 2");
        assert_eq!(reg.len(), 2);

        reg.clear();
        assert!(reg.is_empty());
    }

    #[tokio::test]
    async fn flex_backend_does_not_start_media_streams() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        backend.disconnect().await;
        let _ = server.await;

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(!c.contains("stream"));
            assert!(!c.contains("dax"));
            assert!(!c.contains("audio"));
            assert!(!c.contains("panadapter"));
            assert!(!c.contains("waterfall"));
        }
    }

    #[test]
    fn flex_backend_capabilities_match_runtime_state() {
        let config = FlexRadioConfig::default();
        let backend = FlexRadioBackend::new(config, Some("FLEX-6600"));

        assert_eq!(backend.kind(), CatBackendKind::FlexRadio);
        let caps = backend.capabilities();
        assert!(caps.frequency);
        assert!(caps.mode);
        assert!(caps.passband);
        assert!(caps.split);
        assert!(caps.ptt);
        assert!(!caps.vfo); // SmartSDR slices, not VFO A/B
        assert!(!caps.s_meter); // No UDP data plane in control plane
    }

    #[test]
    fn flex_pending_registry_rejects_capacity_overflow() {
        let mut reg = PendingCommandRegistry::new(2);
        assert!(reg.register(1).is_ok());
        assert!(reg.register(2).is_ok());
        assert_eq!(reg.register(3), Err(FlexProtocolError::QueueFull));
    }

    #[test]
    fn flex_pending_registry_removes_on_response() {
        let mut reg = PendingCommandRegistry::new(10);
        reg.register(5).expect("reg 5");
        assert_eq!(reg.len(), 1);
        assert!(reg.complete(5));
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn flex_pending_registry_removes_on_timeout() {
        let mut reg = PendingCommandRegistry::new(10);
        reg.register(5).expect("reg 5");
        assert_eq!(reg.len(), 1);
        reg.cleanup_expired(Duration::ZERO);
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn flex_pending_registry_clears_on_disconnect() {
        let mut reg = PendingCommandRegistry::new(10);
        reg.register(1).expect("reg 1");
        reg.register(2).expect("reg 2");
        assert_eq!(reg.len(), 2);
        reg.clear();
        assert!(reg.is_empty());
    }

    #[test]
    fn flex_late_response_does_not_complete_reused_sequence() {
        let mut reg = PendingCommandRegistry::new(10);
        reg.register(7).expect("reg 7");
        assert!(reg.complete(7));

        // Spóźniona odpowiedź na seq 7, gdy nie ma go w pending
        assert!(!reg.complete(7));
    }

    #[tokio::test]
    async fn flex_disconnect_does_not_release_other_clients_tx() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let rec = recorded.clone();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH01111111\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();

            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if line.is_empty() {
                        continue;
                    }
                    rec.lock().await.push(line.clone());
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd.starts_with("sub ") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                            if cmd == "sub client all" {
                                let _ = socket
                                    .write_all(
                                        b"S01111111|interlock state=TRANSMITTING tx_client_handle=0x99999999\n",
                                    )
                                    .await;
                            }
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            read_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        let _ = backend.poll().await;

        // Nasza sesja NIE jest właścicielem TX
        assert!(!backend.is_ptt_confirmed());
        assert!(!backend.is_ptt_initiated_by_this_session());

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(!c.contains("xmit 0"));
        }
    }

    #[tokio::test]
    async fn flex_disconnect_releases_tx_initiated_by_this_session_once() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let rec = recorded.clone();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH01111111\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();

            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));

                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }
                    rec.lock().await.push(line.clone());

                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|rest| rest.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd.contains("xmit 1") {
                            let resp = format!(
                                "R{seq}|0|\nS01111111|interlock state=TRANSMITTING tx_client_handle=0x01111111\n"
                            );
                            let _ = socket.write_all(resp.as_bytes()).await;
                        } else if cmd.contains("xmit 0") {
                            let resp = format!("R{seq}|0|\nS01111111|interlock state=READY\n");
                            let _ = socket.write_all(resp.as_bytes()).await;
                        } else {
                            let resp = format!("R{seq}|0|\n");
                            let _ = socket.write_all(resp.as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            read_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("ptt true");
        for _ in 0..10 {
            let _ = backend.poll().await;
            if backend.is_ptt_confirmed() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(backend.is_ptt_confirmed());
        assert!(backend.is_ptt_initiated_by_this_session());

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        let xmit0_count: usize = cmds.iter().map(|c| c.matches("xmit 0").count()).sum();
        assert_eq!(xmit0_count, 1);
    }

    #[test]
    fn flex_interlock_transmitting_alone_does_not_prove_ownership() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.client_handle = Some("MYCLIENT1".to_string());

        let mut status = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        status
            .properties
            .insert("tx_client_handle".to_string(), "OTHERCLIENT2".to_string());

        backend.process_status_message(&status);
        assert!(!backend.is_ptt_confirmed());
        assert!(!backend.is_ptt_initiated_by_this_session());
    }

    #[tokio::test]
    async fn flex_ptt_timeout_leaves_state_unknown() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd.starts_with("sub ") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        } else if cmd.contains("xmit 1") {
                            // Po odebraniu komendy nie wysyłamy odpowiedzi
                            tokio::time::sleep(Duration::from_millis(150)).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            write_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        let _ = backend.execute(&RigCommand::SetPtt(true)).await;
        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );
        assert!(!backend.is_ptt_confirmed());
        backend.disconnect().await;
        server.abort();
    }

    #[test]
    fn flex_ptt_success_requires_response_and_status() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.client_handle = Some("0123ABCD".to_string());

        assert!(!backend.is_ptt_confirmed());

        // Komenda wysłana lokalnie i odpowiedź sukcesu odebrana
        backend.ptt_request_pending = true;
        backend.xmit_command_responded = true;

        // Otrzymanie statusu z naszym handle
        let mut status = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        status
            .properties
            .insert("tx_client_handle".to_string(), "0123ABCD".to_string());

        backend.process_status_message(&status);
        assert!(backend.is_ptt_confirmed());
        assert!(backend.is_ptt_initiated_by_this_session());
    }

    #[test]
    fn flex_profiles_do_not_enforce_unverified_runtime_limits() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6400"));

        for i in 0..4 {
            let mut slice = FlexSliceState::new(FlexSliceId(i));
            slice.in_use = true;
            backend.slices.insert(FlexSliceId(i), slice);
        }

        assert_eq!(backend.slices().len(), 4);
    }

    #[test]
    fn flex_generic_profile_is_conservative() {
        assert!(!FLEX_GENERIC.capabilities.split);
        assert!(!FLEX_GENERIC.capabilities.rit);
        assert!(!FLEX_GENERIC.capabilities.xit);
        assert!(!FLEX_GENERIC.capabilities.power);
        assert!(!FLEX_GENERIC.capabilities.s_meter);
        assert!(!FLEX_GENERIC.capabilities.vfo);
    }

    #[tokio::test]
    async fn flex_backend_does_not_open_udp_4991() {
        let test_socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind free");
        let port = test_socket.local_addr().expect("addr").port();
        assert_ne!(port, 4991);
    }

    #[tokio::test]
    async fn flex_backend_does_not_create_slice_during_probe() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let rec = recorded.clone();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            let mut all_text = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                all_text.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
            rec.lock().await.push(all_text);
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.probe().await.expect("probe failed");

        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(!c.contains("slice create"));
            assert!(!c.contains("slice set"));
        }
    }

    // =========================================================================
    // AUDIT VERIFICATION TESTS (ETAP 6, 7, 9, 10, 11)
    // =========================================================================

    #[tokio::test]
    async fn flex_ptt_timeout_sets_unknown() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd.starts_with("sub ") {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        } else if cmd.contains("xmit 1") {
                            tokio::time::sleep(Duration::from_millis(150)).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            write_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        let err = backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect_err("should timeout");
        assert!(matches!(err, RigError::Timeout { .. }));
        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );
        assert!(!backend.is_ptt_initiated_by_this_session());
        assert!(!backend.is_rx_confirmed());
        assert!(!backend.is_tx_confirmed());

        backend.disconnect().await;
        server.abort();
    }

    #[test]
    fn flex_ptt_unknown_is_not_reported_as_rx() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.confirmed_ptt_state = FlexConfirmedPttState::Unknown;

        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );
        assert!(!backend.is_rx_confirmed());
        assert!(!backend.is_tx_confirmed());
        assert_ne!(backend.confirmed_ptt_state(), FlexConfirmedPttState::Rx);
        assert!(!backend.build_rig_state().ptt);
    }

    #[test]
    fn flex_ptt_confirmed_rx_sets_rx() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.client_handle = Some("0123ABCD".to_string());
        backend.confirmed_ptt_state = FlexConfirmedPttState::Unknown;

        let mut status = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status
            .properties
            .insert("state".to_string(), "READY".to_string());

        backend.process_status_message(&status);
        assert_eq!(backend.confirmed_ptt_state(), FlexConfirmedPttState::Rx);
        assert!(backend.is_rx_confirmed());
        assert!(!backend.is_tx_confirmed());
        assert!(!backend.is_ptt_initiated_by_this_session());
    }

    #[test]
    fn flex_ptt_owned_transmitting_sets_tx() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.client_handle = Some("0123ABCD".to_string());
        backend.ptt_request_pending = true;
        backend.xmit_command_responded = true;

        let mut status = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        status
            .properties
            .insert("tx_client_handle".to_string(), "0123ABCD".to_string());

        backend.process_status_message(&status);
        assert_eq!(backend.confirmed_ptt_state(), FlexConfirmedPttState::Tx);
        assert!(backend.is_tx_confirmed());
        assert!(backend.is_ptt_initiated_by_this_session());
    }

    #[tokio::test]
    async fn flex_transport_loss_sets_ptt_unknown() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            read_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        assert_eq!(backend.confirmed_ptt_state(), FlexConfirmedPttState::Rx);

        server.abort();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let poll_res = backend.poll().await;
        assert!(poll_res.is_err());
        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );
        assert!(!backend.is_rx_confirmed());
    }

    #[tokio::test]
    async fn flex_disconnect_does_not_treat_unknown_as_rx() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.confirmed_ptt_state = FlexConfirmedPttState::Unknown;

        backend.disconnect().await;
        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );
        assert!(!backend.is_rx_confirmed());
    }

    #[test]
    fn flex_ptt_success_requires_response_status_and_ownership() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.client_handle = Some("0123ABCD".to_string());

        // 1. Tylko status bez lokalnego żądania i odpowiedzi -> Unknown
        let mut status = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        status
            .properties
            .insert("tx_client_handle".to_string(), "0123ABCD".to_string());
        backend.process_status_message(&status);
        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );

        // 2. Odpowiedź nadeszła, ale interlock nie jest TRANSMITTING -> PTT pozostaje Unknown (nie Tx i nie Rx)
        backend.ptt_request_pending = true;
        backend.xmit_command_responded = true;
        let mut status_ready = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status_ready
            .properties
            .insert("state".to_string(), "READY".to_string());
        backend.process_status_message(&status_ready);
        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );
        assert!(!backend.is_rx_confirmed());

        // 3. Status TRANSMITTING z obcym uchwytem klienta -> Unknown
        backend.ptt_request_pending = true;
        backend.xmit_command_responded = true;
        let mut status_other = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status_other
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        status_other
            .properties
            .insert("tx_client_handle".to_string(), "99999999".to_string());
        backend.process_status_message(&status_other);
        assert_eq!(
            backend.confirmed_ptt_state(),
            FlexConfirmedPttState::Unknown
        );

        // 4. Pełna zgodność: lokalne żądanie + ACK odpowiedzi + TRANSMITTING z naszym handle -> Tx
        backend.process_status_message(&status);
        assert_eq!(backend.confirmed_ptt_state(), FlexConfirmedPttState::Tx);
        assert!(backend.is_ptt_initiated_by_this_session());
    }

    #[tokio::test]
    async fn flex_disconnect_does_not_release_unknown_tx() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        backend.confirmed_ptt_state = FlexConfirmedPttState::Unknown;
        backend.ptt_initiated_by_this_session = true;

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(!cmds.iter().any(|c| c.contains("xmit 0")));
    }

    #[tokio::test]
    async fn flex_connect_validates_info_identity() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            expected_model: Some("FLEX-6600".to_string()),
            expected_serial: Some("1234-5678-9012-3456".to_string()),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, None);
        backend.connect().await.expect("connect should succeed");

        assert_eq!(backend.profile.model_id, "FLEX-6600");
        assert_eq!(backend.profile.name, "FlexRadio FLEX-6600");
        assert_eq!(backend.runtime_slice_count(), Some(4));

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(cmds.iter().any(|c| c.contains("info")));
    }

    #[tokio::test]
    async fn flex_connect_validates_discovery_against_info() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            expected_model: Some("FLEX-6600".to_string()),
            expected_serial: Some("1234-5678-9012-3456".to_string()),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, None);
        assert!(backend.connect().await.is_ok());

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_connect_rejects_model_mismatch() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            expected_model: Some("FLEX-6400".to_string()), // radio zgłasza 6600
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, None);
        let err = backend
            .connect()
            .await
            .expect_err("should reject model mismatch");
        assert!(err.to_string().contains("ModelMismatch") || err.to_string().contains("FLEX-6400"));

        server.abort();
    }

    #[tokio::test]
    async fn flex_connect_rejects_serial_mismatch() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            expected_serial: Some("9999-9999".to_string()), // radio zgłasza 1234-5678-9012-3456
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, None);
        let err = backend
            .connect()
            .await
            .expect_err("should reject serial mismatch");
        assert!(
            err.to_string().contains("SerialMismatch") || err.to_string().contains("9999-9999")
        );

        server.abort();
    }

    #[tokio::test]
    async fn flex_connect_uses_runtime_slice_count_as_observation() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, None);
        backend.connect().await.expect("connect");
        assert_eq!(backend.runtime_slice_count(), Some(4));

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[test]
    fn flex_runtime_slice_count_does_not_override_hardware_safety() {
        let config = FlexRadioConfig::default();
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-GENERIC"));
        backend.runtime_slice_count = Some(8);

        // Nawet po zaobserwowaniu 8 slice'ów w runtime, generic profil sprzętowy pozostaje konserwatywny
        assert!(!backend.capabilities().ptt);
        assert!(!backend.capabilities().split);
        assert!(!backend.capabilities().power);
    }

    #[test]
    fn flex_client_handle_is_not_client_id() {
        let config = FlexRadioConfig {
            client_id: Some("uuid-station-1234".to_string()),
            ..Default::default()
        };
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.client_handle = Some("0123ABCD".to_string());

        assert_eq!(backend.client_handle(), Some("0123ABCD"));
        assert_eq!(
            backend.config.client_id.as_deref(),
            Some("uuid-station-1234")
        );
        assert_ne!(
            backend.client_handle().unwrap(),
            backend.config.client_id.as_ref().unwrap()
        );
    }

    #[tokio::test]
    async fn flex_non_gui_mode_does_not_claim_gui_resources() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(!c.contains("client gui"));
            assert!(!c.contains("gui"));
        }
    }

    #[tokio::test]
    async fn flex_client_bind_requires_explicit_client_id() {
        // A. Brak client_id w konfiguracji -> brak komendy client bind
        let (listener1, port1) = setup_mock_flex_radio().await;
        let recorded1 = Arc::new(Mutex::new(Vec::new()));
        let server1 = spawn_standard_mock_radio(listener1, recorded1.clone()).await;

        let config1 = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port: port1,
            client_id: None,
            ..Default::default()
        };
        let mut backend1 = FlexRadioBackend::new(config1, Some("FLEX-6600"));
        backend1.connect().await.expect("connect");
        backend1.disconnect().await;
        server1.await.expect("server join");

        let cmds1 = recorded1.lock().await;
        assert!(!cmds1.iter().any(|c| c.contains("client bind")));

        // B. Jawne podanie client_id w konfiguracji -> wysłanie client bind client_id=...
        let (listener2, port2) = setup_mock_flex_radio().await;
        let recorded2 = Arc::new(Mutex::new(Vec::new()));
        let server2 = spawn_standard_mock_radio(listener2, recorded2.clone()).await;

        let config2 = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port: port2,
            client_id: Some("station-uuid-99".to_string()),
            ..Default::default()
        };
        let mut backend2 = FlexRadioBackend::new(config2, Some("FLEX-6600"));
        backend2.connect().await.expect("connect");
        backend2.disconnect().await;
        server2.await.expect("server join");

        let cmds2 = recorded2.lock().await;
        assert!(
            cmds2
                .iter()
                .any(|c| c.contains("client bind client_id=station-uuid-99"))
        );
    }

    #[tokio::test]
    async fn flex_probe_does_not_register_gui() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.probe().await.expect("probe");
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(!c.contains("client gui"));
            assert!(!c.contains("client bind"));
            assert!(!c.contains("slice"));
        }
    }

    #[tokio::test]
    async fn flex_disconnect_closes_own_tcp_socket() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        assert!(backend.connection.is_some());

        backend.disconnect().await;
        assert!(backend.connection.is_none());
        server.await.expect("server joins cleanly on EOF");
    }

    #[tokio::test]
    async fn flex_disconnect_does_not_send_client_disconnect_without_handle() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            if let Some((_, cmd)) = c.strip_prefix('C').and_then(|r| r.split_once('|')) {
                assert_ne!(cmd.trim(), "client disconnect");
            }
        }
    }

    #[tokio::test]
    async fn flex_disconnect_does_not_send_client_disconnect_for_own_handle() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        let own_handle = backend.client_handle().unwrap().to_string();

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(
                !c.contains(&format!("client disconnect {own_handle}")),
                "own handle should not be sent with client disconnect: {c}"
            );
        }
    }

    #[tokio::test]
    async fn flex_disconnect_does_not_disconnect_other_clients() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(
                !c.contains("client disconnect"),
                "disconnect must never send administrative client disconnect: {c}"
            );
        }
    }

    #[tokio::test]
    async fn flex_disconnect_is_idempotent() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        backend.disconnect().await;
        assert!(backend.connection.is_none());
        backend.disconnect().await;
        assert!(backend.connection.is_none());
        backend.disconnect().await;
        assert!(backend.connection.is_none());

        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_disconnect_clears_pending_registry() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        let _ = backend.pending_registry.register(999);
        assert!(!backend.pending_registry.is_empty());

        backend.disconnect().await;
        assert!(backend.pending_registry.is_empty());
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_disconnect_owned_tx_cleanup_precedes_socket_shutdown() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.confirmed_ptt_state = FlexConfirmedPttState::Tx;
        backend.ptt_initiated_by_this_session = true;

        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(cmds.iter().any(|c| c.contains("xmit 0")));
    }

    #[tokio::test]
    async fn flex_subscription_slice_is_required_and_confirmed() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd == "sub slice all" {
                            let _ = socket
                                .write_all(format!("R{seq}|1|error slice\n").as_bytes())
                                .await;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        let err = backend
            .connect()
            .await
            .expect_err("required subscription must fail connect");
        assert!(
            err.to_string().contains("SubscriptionRejected")
                || err.to_string().contains("sub slice all")
        );
        server.abort();
    }

    #[tokio::test]
    async fn flex_subscription_interlock_is_required_and_confirmed() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd == "sub interlock all" {
                            let _ = socket
                                .write_all(format!("R{seq}|1|error interlock\n").as_bytes())
                                .await;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        let err = backend
            .connect()
            .await
            .expect_err("required subscription must fail connect");
        assert!(
            err.to_string().contains("SubscriptionRejected")
                || err.to_string().contains("sub interlock all")
        );
        server.abort();
    }

    #[tokio::test]
    async fn flex_subscription_tx_is_optional_and_tolerates_rejection() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd == "sub tx all" {
                            let _ = socket
                                .write_all(format!("R{seq}|1|error tx optional\n").as_bytes())
                                .await;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend
            .connect()
            .await
            .expect("optional tx subscription rejection must not block connect");
        assert!(backend.connection.is_some());
        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_subscription_radio_is_optional_and_tolerates_rejection() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd == "sub radio all" {
                            let _ = socket
                                .write_all(format!("R{seq}|1|error radio optional\n").as_bytes())
                                .await;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend
            .connect()
            .await
            .expect("optional radio subscription rejection must not block connect");
        assert!(backend.connection.is_some());
        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_subscription_client_is_optional_and_tolerates_rejection() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd == "sub client all" {
                            let _ = socket
                                .write_all(format!("R{seq}|1|error client optional\n").as_bytes())
                                .await;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend
            .connect()
            .await
            .expect("optional client subscription rejection must not block connect");
        assert!(backend.connection.is_some());
        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_subscription_client_unsupported_on_legacy_api_version() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let recorded_clone = recorded.clone();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            // SmartSDR v1.x legacy handshake
            let _ = socket.write_all(b"V1.11.0\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    recorded_clone.lock().await.push(line.clone());
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };
        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend
            .connect()
            .await
            .expect("legacy v1 API connection succeeds");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        // W v1.x sub client all jest pomijane jako unsupported
        assert!(!cmds.iter().any(|c| c.contains("sub client all")));
        assert!(cmds.iter().any(|c| c.contains("sub slice all")));
    }

    #[tokio::test]
    async fn flex_connect_registers_minimal_control_subscriptions() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(cmds.iter().any(|c| c.contains("sub slice all")));
        assert!(cmds.iter().any(|c| c.contains("sub tx all")));
        assert!(cmds.iter().any(|c| c.contains("sub interlock all")));
        assert!(cmds.iter().any(|c| c.contains("sub radio all")));
        assert!(cmds.iter().any(|c| c.contains("sub client all")));
    }

    #[tokio::test]
    async fn flex_connect_registers_only_control_plane_subscriptions() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            let Some((_, cmd)) = c.strip_prefix('C').and_then(|r| r.split_once('|')) else {
                continue;
            };
            if cmd.starts_with("sub ") {
                assert!(MINIMAL_CONTROL_SUBSCRIPTIONS.contains(&cmd));
            }
        }
    }

    #[tokio::test]
    async fn flex_connect_does_not_subscribe_data_plane() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        for c in cmds.iter() {
            assert!(!c.contains("sub dax"));
            assert!(!c.contains("sub audio"));
            assert!(!c.contains("sub panadapter"));
            assert!(!c.contains("sub waterfall"));
            assert!(!c.contains("sub meter"));
            assert!(!c.contains("sub iq"));
        }
    }

    #[tokio::test]
    async fn flex_connect_does_not_subscribe_audio_stream() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(!cmds.iter().any(|c| c.contains("audio_stream")));
    }

    #[tokio::test]
    async fn flex_connect_does_not_subscribe_dax() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(!cmds.iter().any(|c| c.contains("dax")));
    }

    #[tokio::test]
    async fn flex_connect_does_not_subscribe_panadapter() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded.clone()).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        backend.disconnect().await;
        server.await.expect("server join");

        let cmds = recorded.lock().await;
        assert!(!cmds.iter().any(|c| c.contains("panadapter")));
    }

    #[tokio::test]
    async fn flex_connect_waits_for_subscription_responses() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        assert!(backend.pending_registry.is_empty());

        backend.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_connect_waits_for_required_subscription_ack() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else {
                            tokio::time::sleep(Duration::from_millis(150)).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            connect_timeout: Duration::from_millis(50),
            read_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        let err = backend
            .connect()
            .await
            .expect_err("should timeout on subscription ack");
        assert!(matches!(err, RigError::Timeout { .. }));

        server.abort();
    }

    #[tokio::test]
    async fn flex_connect_fails_when_required_subscription_is_rejected() {
        let (listener, port) = setup_mock_flex_radio().await;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let _ = socket.write_all(b"V3.7.4\nH0123ABCD\n").await;
            let mut buf = [0u8; 1024];
            let mut pending = String::new();
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim().to_string();
                    pending = pending[pos + 1..].to_string();
                    if let Some((seq_str, cmd)) =
                        line.strip_prefix('C').and_then(|r| r.split_once('|'))
                    {
                        let seq: u32 = seq_str.parse().unwrap_or(0);
                        if cmd == "info" {
                            let _ = socket
                                .write_all(
                                    format!(
                                        "R{seq}|0|model=FLEX-6600 chassis_serial=1234 slice=4\n"
                                    )
                                    .as_bytes(),
                                )
                                .await;
                        } else if cmd == "sub slice all" {
                            let _ = socket
                                .write_all(format!("R{seq}|1|subscription error\n").as_bytes())
                                .await;
                        } else {
                            let _ = socket.write_all(format!("R{seq}|0|\n").as_bytes()).await;
                        }
                    }
                }
            }
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        let err = backend
            .connect()
            .await
            .expect_err("should fail when subscription rejected");
        assert!(
            err.to_string().contains("SubscriptionRejected")
                || err.to_string().contains("subscription error")
        );

        server.abort();
    }

    #[tokio::test]
    async fn flex_subscriptions_are_scoped_to_connection() {
        let (listener, port) = setup_mock_flex_radio().await;
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let server = spawn_standard_mock_radio(listener, recorded).await;

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");
        assert!(backend.connection.is_some());

        backend.disconnect().await;
        assert!(backend.connection.is_none());
        assert!(backend.pending_registry.is_empty());
        assert!(backend.slices.is_empty());

        server.await.expect("server join");
    }
}
