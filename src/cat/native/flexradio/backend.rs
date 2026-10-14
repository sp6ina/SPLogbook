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
use super::slice::{FlexSliceId, FlexSliceState, select_active_slice, select_tx_slice};

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

/// Natywny backend FlexRadio SmartSDR API.
pub struct FlexRadioBackend {
    config: FlexRadioConfig,
    connection: Option<FlexRadioConnection>,
    profile: &'static RigModelProfile,
    seq_allocator: FlexSequenceAllocator,
    pending_registry: PendingCommandRegistry,
    slices: BTreeMap<FlexSliceId, FlexSliceState>,
    client_handle: Option<String>,
    protocol_version: Option<String>,
    interlock_state: String,
    tx_client_handle: Option<String>,
    confirmed_ptt: bool,
    rf_power_watts: f32,
    events: VecDeque<NativeRigEvent>,
    last_state: RigState,
}

impl FlexRadioBackend {
    /// Tworzy nową instancję backendu z konfiguracją i profilem modelu.
    pub fn new(config: FlexRadioConfig, model_name: Option<&str>) -> Self {
        let profile = match model_name {
            Some(name) => find_flex_profile(name),
            None => &FLEX_GENERIC,
        };

        Self {
            config,
            connection: None,
            profile,
            seq_allocator: FlexSequenceAllocator::new(),
            pending_registry: PendingCommandRegistry::default(),
            slices: BTreeMap::new(),
            client_handle: None,
            protocol_version: None,
            interlock_state: "READY".to_string(),
            tx_client_handle: None,
            confirmed_ptt: false,
            rf_power_watts: 0.0,
            events: VecDeque::new(),
            last_state: RigState::default(),
        }
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

    /// Zwraca potwierdzony stan PTT.
    pub fn is_ptt_confirmed(&self) -> bool {
        self.confirmed_ptt
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

        let seq = self.seq_allocator.next_seq();
        let cmd = FlexCommand::new(seq, cmd_text);

        self.pending_registry.register(seq)?;

        if let Err(e) = conn.send_command(&cmd).await {
            self.pending_registry.complete(seq);
            return Err(e);
        }

        Ok(seq)
    }

    /// Przetwarza nadeszłe komunikaty i aktualizuje stan wewnętrzny backendu.
    fn process_inbound_messages(&mut self, messages: Vec<FlexInboundMessage>) {
        for msg in messages {
            match msg {
                FlexInboundMessage::Response(resp) => {
                    self.pending_registry.complete(resp.seq);
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
                    // Sprawdź czy to informacja o usunięciu (in_use=0)
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
                    self.confirmed_ptt = self.interlock_state == "TRANSMITTING";
                }
                if let Some(tx_handle) = status.properties.get("tx_client_handle") {
                    self.tx_client_handle = Some(tx_handle.clone());
                }
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
        let active_slice = select_active_slice(
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
            ptt: self.confirmed_ptt,
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

        // Subskrybuj wymagane obiekty statusu
        let _ = self.send_command_tracked("sub slice all").await;
        let _ = self.send_command_tracked("sub tx all").await;
        let _ = self.send_command_tracked("sub interlock all").await;

        Ok(())
    }

    async fn poll(&mut self) -> Result<RigState, RigError> {
        let endpoint = format!("{}:{}", self.config.host, self.config.port);

        let messages = {
            let conn = self.connection.as_mut().ok_or(RigError::NotConnected)?;

            conn.read_messages()
                .await
                .map_err(|e| e.to_rig_error(&endpoint))?
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
                _ => "unsupported_command",
            };
            return Err(RigError::Unsupported {
                backend: CatBackendKind::FlexRadio,
                command: cmd_name,
            });
        }

        match command {
            RigCommand::SetFrequency(freq_hz) => {
                let slice = select_active_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
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
                let slice = select_active_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
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
                let slice = select_active_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
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
                if *enabled {
                    // Weryfikacja czy posiadamy co najmniej 2 odbiorniki slice
                    if self.slices.len() < 2 {
                        return Err(RigError::Protocol {
                            detail: "split requires at least 2 active slices".to_string(),
                        });
                    }
                }
            }
            RigCommand::SetPtt(true) => {
                // Bezpieczeństwo PTT: pojedyncze wysłanie polecenia xmit 1 bez ponawiania (no retry)
                self.send_command_tracked("xmit 1")
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetPtt(false) => {
                self.send_command_tracked("xmit 0")
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
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
                let slice = select_active_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
                )
                .map_err(|e| e.to_rig_error(&endpoint))?;

                let (on, freq) = if *rit_hz != 0 { (1, *rit_hz) } else { (0, 0) };
                let cmd = format!("slice set {} rit_on={on} rit_freq={freq}", slice.id);

                self.send_command_tracked(cmd)
                    .await
                    .map_err(|e| e.to_rig_error(&endpoint))?;
            }
            RigCommand::SetXit(xit_hz) => {
                let slice = select_active_slice(
                    &self.slices,
                    self.config.preferred_slice,
                    self.client_handle.as_deref(),
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
        // Jeśli PTT było potwierdzone jako aktywne, wykonujemy pojedynczą
        // ograniczoną czasowo próbę wyłączenia nadawania (xmit 0) bez retry.
        if self.confirmed_ptt {
            if let Some(conn) = self.connection.as_mut() {
                let cmd = FlexCommand::new(self.seq_allocator.next_seq(), "xmit 0");
                let _ =
                    tokio::time::timeout(Duration::from_millis(500), conn.send_command(&cmd)).await;
            }
            self.confirmed_ptt = false;
        }

        if let Some(mut conn) = self.connection.take() {
            conn.disconnect().await;
        }

        self.pending_registry.clear();
        self.slices.clear();
        self.client_handle = None;
        self.protocol_version = None;
        self.interlock_state = "READY".to_string();
        self.tx_client_handle = None;
        self.confirmed_ptt = false;
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

    #[tokio::test]
    async fn flex_backend_connects_and_completes_handshake() {
        let (listener, port) = setup_mock_flex_radio().await;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write handshake");
            // Odpowiedzi na subskrypcje
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(b"R1|0|\nR2|0|\nR3|0|\n")
                .await
                .expect("write resp");
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect failed");

        assert_eq!(backend.protocol_version(), Some("3.7.4"));
        assert_eq!(backend.client_handle(), Some("0123ABCD"));

        server.await.expect("server join");
        backend.disconnect().await;
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
        let rec = recorded.clone();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.expect("read");
            let s = String::from_utf8_lossy(&buf[..n]).to_string();
            rec.lock().await.push(s);
            socket
                .write_all(b"R1|0|\nR2|0|\nR3|0|\n")
                .await
                .expect("write resp");
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6400"));
        backend.connect().await.expect("connect");

        server.await.expect("server join");
        backend.disconnect().await;

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
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(b"R1|0|\nR2|0|\nR3|0|\nS0123ABCD|slice 0 in_use=1 active=1 mode=CW RF_frequency=14.025000 filter_lo=-250 filter_hi=250\n")
                .await
                .expect("resp");
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
        backend.connect().await.expect("connect");

        // Wstawiamy aktywny slice 0
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

        // Symulacja statusu interlock: TRANSMITTING
        let mut status_tx = super::super::protocol::FlexStatus {
            client_handle: None,
            object_class: "interlock".to_string(),
            object_id: None,
            properties: BTreeMap::new(),
        };
        status_tx
            .properties
            .insert("state".to_string(), "TRANSMITTING".to_string());
        backend.process_status_message(&status_tx);
        assert!(backend.is_ptt_confirmed());
    }

    #[tokio::test]
    async fn flex_backend_rejects_ambiguous_slice() {
        let (listener, port) = setup_mock_flex_radio().await;

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(b"R1|0|\nR2|0|\nR3|0|\n")
                .await
                .expect("resp");
        });

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

        server.await.expect("server join");
        backend.disconnect().await;
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
        backend.connect().await.expect("connect");

        // Symulacja aktywnego PTT przed rozłączeniem
        backend.confirmed_ptt = true;

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
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(b"R1|0|\nR2|0|\nR3|0|\n")
                .await
                .expect("resp");

            // Wysyłamy fragmentaryczny status
            socket
                .write_all(b"S0123ABCD|slice 0 in_use=1 active=1 mode=USB RF_freq")
                .await
                .expect("w1");
            tokio::time::sleep(Duration::from_millis(30)).await;
            socket.write_all(b"uency=14.200000\n").await.expect("w2");
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        // Pierwszy poll odczytuje fragment statusu i zapisuje go w buforze
        let _ = backend.poll().await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Drugi poll odczytuje resztę i poprawnie dekoduje całą linię
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
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;

            // Przeplatane odpowiedzi i statusy
            socket
                .write_all(b"S0123ABCD|slice 0 in_use=1 active=1 mode=USB RF_frequency=14.100000\nR1|0|\nS0123ABCD|transmit rfpower=75\nR2|0|\nR3|0|\n")
                .await
                .expect("write");
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        let state = backend.poll().await.expect("poll");
        assert_eq!(state.frequency_hz, 14_100_000);
        assert_eq!(state.rf_power_watts, 75.0);

        server.await.expect("server join");
        backend.disconnect().await;
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
        let rec = recorded.clone();

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            socket
                .write_all(b"V3.7.4\nH0123ABCD\n")
                .await
                .expect("write");
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.expect("read");
            rec.lock()
                .await
                .push(String::from_utf8_lossy(&buf[..n]).to_string());
            socket
                .write_all(b"R1|0|\nR2|0|\nR3|0|\n")
                .await
                .expect("resp");
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = FlexRadioBackend::new(config, Some("FLEX-6600"));
        backend.connect().await.expect("connect");

        server.await.expect("server join");
        backend.disconnect().await;

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
}
