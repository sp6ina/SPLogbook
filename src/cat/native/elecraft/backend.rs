// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny backend sterowania transceiverami Elecraft (K3, K3S, KX2, KX3, K4, K4D).
//!
//! Zapewnia bezpośrednią komunikację ze sprzętem przez porty szeregowe / USB Virtual COM
//! oraz Ethernet TCP (Elecraft K4 port 9200):
//! - rygorystyczne bezpieczeństwo PTT (brak PTT podczas probe/connect, brak retry dla SetPtt(true)),
//! - dwufazowy connect (faza 1: read-only, faza 2: aktywacja Auto-Info `AI2;`),
//! - pełna obsługa trybu Auto Information z przeplataniem odpowiedzi na zapytania GET,
//! - dekodowanie ramek transceivera (`FA`, `FB`, `MD`, `DT`, `IF`, `TQ`, `PC`, `PO`, `BW`, `SM`, `SMH`),
//! - obsługa odpowiedzi błędu `?;`,
//! - pełna zgodność z kontraktem [`RigBackend`] oraz typowanymi błędami [`RigError`].

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::cat::backend::{CatBackendKind, RigBackend};
use crate::cat::native::elecraft::codec::{ElecraftCat, ElecraftCodec, ElecraftMessage};
use crate::cat::native::elecraft::k4_tcp::{ElecraftTcpConfig, open_k4_tcp_stream};
use crate::cat::native::elecraft::models::{ElecraftModelProfile, ElecraftProtocolFamily};
use crate::cat::native::transport::{AsyncFramedTransport, SerialConfig, open_serial_stream};
use crate::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};

/// Cecha strumienia we/wy dla testów deterministycznych i połączeń asynchronicznych.
pub trait AsyncReaderWriter: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncReaderWriter for T {}

/// Rodzaj fizycznego połączenia z transceiverem Elecraft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElecraftConnection {
    /// Port szeregowy RS-232 lub USB Virtual COM.
    Serial(SerialConfig),
    /// Kanał Ethernet TCP (K4 / K4D, domyślnie port 9200).
    Tcp(ElecraftTcpConfig),
    /// Połączenie testowe lub brak zdefiniowanego transportu.
    None,
}

/// Główny natywny backend Elecraft implementujący cechę [`RigBackend`].
pub struct ElecraftBackend {
    profile: ElecraftModelProfile,
    connection: ElecraftConnection,
    transport: Option<AsyncFramedTransport<Box<dyn AsyncReaderWriter>>>,
    mock_stream: Option<Box<dyn AsyncReaderWriter>>,
    state: RigState,
    codec: ElecraftCodec,
    session_active: bool,
    ai_active: bool,
    confirmed_ptt: bool,
    raw_smeter: Option<u16>,
}

impl ElecraftBackend {
    /// Tworzy nowy backend dla połączenia szeregowego / USB Virtual COM.
    pub fn new_serial(config: SerialConfig, profile: ElecraftModelProfile) -> Self {
        Self {
            profile,
            connection: ElecraftConnection::Serial(config),
            transport: None,
            mock_stream: None,
            state: RigState::default(),
            codec: ElecraftCodec::new(),
            session_active: false,
            ai_active: false,
            confirmed_ptt: false,
            raw_smeter: None,
        }
    }

    /// Tworzy nowy backend dla połączenia sieciowego TCP (K4).
    pub fn new_tcp(config: ElecraftTcpConfig, profile: ElecraftModelProfile) -> Self {
        Self {
            profile,
            connection: ElecraftConnection::Tcp(config),
            transport: None,
            mock_stream: None,
            state: RigState::default(),
            codec: ElecraftCodec::new(),
            session_active: false,
            ai_active: false,
            confirmed_ptt: false,
            raw_smeter: None,
        }
    }

    /// Tworzy backend z bezpośrednio wstrzykniętym strumieniem I/O (dla testów bez sprzętu).
    pub fn with_stream<S: AsyncReaderWriter + 'static>(
        stream: S,
        profile: ElecraftModelProfile,
    ) -> Self {
        Self {
            profile,
            connection: ElecraftConnection::None,
            transport: None,
            mock_stream: Some(Box::new(stream)),
            state: RigState::default(),
            codec: ElecraftCodec::new(),
            session_active: false,
            ai_active: false,
            confirmed_ptt: false,
            raw_smeter: None,
        }
    }

    /// Tworzy backend na podstawie ogólnej konfiguracji [`ElecraftConnection`].
    pub fn new(connection: ElecraftConnection, profile: ElecraftModelProfile) -> Self {
        Self {
            profile,
            connection,
            transport: None,
            mock_stream: None,
            state: RigState::default(),
            codec: ElecraftCodec::new(),
            session_active: false,
            ai_active: false,
            confirmed_ptt: false,
            raw_smeter: None,
        }
    }

    /// Zwraca profil modelu transceivera.
    pub fn profile(&self) -> &ElecraftModelProfile {
        &self.profile
    }

    /// Zwraca bieżący odczytany stan radia.
    pub fn current_state(&self) -> &RigState {
        &self.state
    }

    /// Zwraca informację, czy tryb Auto Information jest aktywny.
    pub fn is_ai_active(&self) -> bool {
        self.ai_active
    }

    /// Zwraca informację, czy sesja sterowania radiem jest aktywna.
    pub fn is_session_active(&self) -> bool {
        self.session_active
    }

    /// Zwraca ostatnio odczytaną surową wartość S-metra (słupki LCD).
    pub fn raw_smeter(&self) -> Option<u16> {
        self.raw_smeter
    }

    /// Zwraca nazwę punktu końcowego (COM lub IP:port) do logów i błędów.
    fn endpoint_name(&self) -> String {
        match &self.connection {
            ElecraftConnection::Serial(cfg) => cfg.port_name.clone(),
            ElecraftConnection::Tcp(cfg) => format!("{}:{}", cfg.host, cfg.port),
            ElecraftConnection::None => "memory_stream".to_string(),
        }
    }

    /// Otwiera transport bazowy (szeregowy, TCP lub mock).
    async fn open_transport(&mut self) -> Result<(), RigError> {
        let endpoint = self.endpoint_name();

        if let Some(stream) = self.mock_stream.take() {
            let boxed_stream: Box<dyn AsyncReaderWriter> = Box::new(stream);
            let transport = AsyncFramedTransport::from_stream(
                boxed_stream,
                self.profile.max_message_length.max(1024),
                Duration::from_millis(1500),
                Duration::from_millis(1500),
            );
            self.transport = Some(transport);
            return Ok(());
        }

        match &self.connection {
            ElecraftConnection::Serial(config) => {
                let stream = open_serial_stream(config)
                    .map_err(|e| e.to_rig_error(CatBackendKind::Elecraft, &endpoint))?;
                let boxed_stream: Box<dyn AsyncReaderWriter> = Box::new(stream);
                let transport = AsyncFramedTransport::from_stream(
                    boxed_stream,
                    self.profile.max_message_length.max(1024),
                    Duration::from_millis(1500),
                    Duration::from_millis(1500),
                );
                self.transport = Some(transport);
                Ok(())
            }
            ElecraftConnection::Tcp(config) => {
                if !self.profile.supports_tcp {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "tcp_transport",
                    });
                }
                let stream = open_k4_tcp_stream(config)
                    .await
                    .map_err(|e| e.to_rig_error(CatBackendKind::Elecraft, &endpoint))?;
                let boxed_stream: Box<dyn AsyncReaderWriter> = Box::new(stream);
                let transport = AsyncFramedTransport::from_stream(
                    boxed_stream,
                    config.max_buffer_size,
                    config.read_timeout,
                    config.write_timeout,
                );
                self.transport = Some(transport);
                Ok(())
            }
            ElecraftConnection::None => Err(RigError::NotConnected),
        }
    }

    /// Wysyła surowe polecenie ASCII zakończone średnikiem.
    async fn send_command_raw(&mut self, cmd: &str) -> Result<(), RigError> {
        let endpoint = self.endpoint_name();
        let bytes = self.codec.encode_command(cmd);

        let transport = self.transport.as_mut().ok_or(RigError::NotConnected)?;
        transport
            .write_all(&bytes)
            .await
            .map_err(|e| e.to_rig_error(CatBackendKind::Elecraft, &endpoint))
    }

    /// Aktualizuje wewnętrzny stan radia na podstawie zdekodowanego komunikatu.
    fn apply_message_to_state(&mut self, msg: &ElecraftMessage) {
        match msg {
            ElecraftMessage::Frequency { vfo_b: false, hz } => {
                self.state.frequency_hz = *hz;
            }
            ElecraftMessage::Frequency { vfo_b: true, hz } => {
                self.state.tx_frequency_hz = Some(*hz);
            }
            ElecraftMessage::Mode {
                vfo_b: false, mode, ..
            } => {
                self.state.mode = mode.to_string();
            }
            ElecraftMessage::DataSubMode {
                vfo_b: false, name, ..
            } => {
                if self.state.mode == "DATA" {
                    self.state.mode = name.to_string();
                }
            }
            ElecraftMessage::IfInfo(info) => {
                self.state.frequency_hz = info.frequency_hz;
                self.state.mode = info.mode.to_string();
                self.state.ptt = info.tx;
                self.state.split_enabled = info.split;
                self.state.rit_hz = if info.rit_on { info.rit_offset_hz } else { 0 };
                self.state.xit_hz = if info.xit_on { info.rit_offset_hz } else { 0 };
                self.confirmed_ptt = info.tx;
            }
            ElecraftMessage::TransmitStatus(tx) => {
                self.state.ptt = *tx;
                self.confirmed_ptt = *tx;
            }
            ElecraftMessage::TransmitVfo { split, .. } => {
                self.state.split_enabled = *split;
            }
            ElecraftMessage::ReceiveVfo { vfo_b } => {
                self.state.vfo = if *vfo_b {
                    "VFOB".to_string()
                } else {
                    "VFOA".to_string()
                };
            }
            ElecraftMessage::Passband {
                vfo_b: false,
                passband_hz,
            } => {
                self.state.passband_hz = *passband_hz;
            }
            ElecraftMessage::RitState {
                vfo_b: false,
                enabled,
            } => {
                if !*enabled {
                    self.state.rit_hz = 0;
                }
            }
            ElecraftMessage::XitState {
                vfo_b: false,
                enabled,
            } => {
                if !*enabled {
                    self.state.xit_hz = 0;
                }
            }
            ElecraftMessage::RitOffset {
                vfo_b: false,
                offset_hz,
            } => {
                self.state.rit_hz = *offset_hz;
            }
            ElecraftMessage::PowerControl { watts, .. } | ElecraftMessage::PowerOutput(watts) => {
                self.state.rf_power_watts = *watts;
            }
            ElecraftMessage::RawSMeter { vfo_b: false, raw } => {
                self.raw_smeter = Some(*raw);
            }
            ElecraftMessage::HighResSMeter {
                vfo_b: false, dbm, ..
            } => {
                self.state.s_meter_dbm = *dbm;
            }
            _ => {}
        }
    }

    /// Opróżnia bufor transportu, przetwarzając asynchroniczne zdarzenia Auto-Info.
    async fn drain_incoming_events(&mut self) -> Result<(), RigError> {
        let endpoint = self.endpoint_name();
        let family = self.profile.family;

        let mut messages = Vec::new();

        if let Some(transport) = self.transport.as_mut() {
            // 1. Najpierw przetwórz to, co już znajduje się w buforze transportu
            loop {
                let decode_res = self.codec.decode_frame(transport.buffer());
                match decode_res {
                    Ok(Some((frame, consumed))) => {
                        transport.consume(consumed);
                        if let Ok(msg) = self.codec.decode_message(&frame, family) {
                            messages.push(msg);
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        return Err(RigError::Protocol {
                            detail: format!("Elecraft codec error on {endpoint}: {e}"),
                        });
                    }
                }
            }

            // 2. Krótka próba odczytu asynchronicznych powiadomień ze strumienia
            let read_future = transport.read_chunk();
            if let Ok(res) = tokio::time::timeout(Duration::from_millis(20), read_future).await {
                match res {
                    Ok(_) => {
                        while let Ok(Some((frame, consumed))) =
                            self.codec.decode_frame(transport.buffer())
                        {
                            transport.consume(consumed);
                            if let Ok(msg) = self.codec.decode_message(&frame, family) {
                                messages.push(msg);
                            }
                        }
                    }
                    Err(crate::cat::native::transport::TransportError::Closed) => {
                        return Err(RigError::NotConnected);
                    }
                    Err(crate::cat::native::transport::TransportError::Timeout { .. }) => {}
                    Err(e) => {
                        return Err(e.to_rig_error(CatBackendKind::Elecraft, &endpoint));
                    }
                }
            }
        } else {
            return Err(RigError::NotConnected);
        }

        for msg in &messages {
            self.apply_message_to_state(msg);
        }

        Ok(())
    }

    /// Wysyła zapytanie GET i oczekuje na dopasowaną odpowiedź, bezpiecznie
    /// przetwarzając i zachowując przeplatane zdarzenia Auto-Info.
    async fn query_and_await_response(
        &mut self,
        cmd: &str,
        expected_prefix: &str,
    ) -> Result<ElecraftMessage, RigError> {
        let endpoint = self.endpoint_name();
        let family = self.profile.family;

        self.send_command_raw(cmd).await?;

        let deadline = tokio::time::Instant::now() + Duration::from_millis(1500);

        loop {
            // Sprawdź, czy w buforze znajduje się już gotowa ramka
            let maybe_frame = {
                let transport = self.transport.as_mut().ok_or(RigError::NotConnected)?;
                let res = self.codec.decode_frame(transport.buffer()).map_err(|e| {
                    RigError::Protocol {
                        detail: format!("Elecraft codec error on {endpoint}: {e}"),
                    }
                })?;
                if let Some((frame, consumed)) = res {
                    transport.consume(consumed);
                    Some(frame)
                } else {
                    None
                }
            };

            if let Some(frame) = maybe_frame {
                if frame.is_error() {
                    return Err(RigError::Rejected { code: -1 });
                }

                let msg =
                    self.codec
                        .decode_message(&frame, family)
                        .map_err(|e| RigError::Protocol {
                            detail: format!("Elecraft parse error on {endpoint}: {e}"),
                        })?;

                self.apply_message_to_state(&msg);

                if msg.matches_query_prefix(expected_prefix) {
                    return Ok(msg);
                }
                // W przeciwnym razie była to ramka Auto-Info; pętla kontynuuje oczekiwanie na dopasowaną odpowiedź
                continue;
            }

            // Oczekuj na nowe dane ze strumienia
            if tokio::time::Instant::now() >= deadline {
                return Err(RigError::Timeout {
                    endpoint,
                    operation: "query_response",
                });
            }

            let transport = self.transport.as_mut().ok_or(RigError::NotConnected)?;
            transport
                .read_chunk()
                .await
                .map_err(|e| e.to_rig_error(CatBackendKind::Elecraft, &endpoint))?;
        }
    }

    /// Sonda read-only weryfikująca obecność i tożsamość transceivera bez wysyłania komend PTT/TX ani zapisu menu.
    pub async fn probe(&mut self) -> Result<RigState, RigError> {
        self.open_transport().await?;

        // 1. Wyślij wyłącznie odczyt tożsamości (ID;)
        let id_msg = self.query_and_await_response("ID;", "ID").await?;
        if let ElecraftMessage::Identification(_) = id_msg {
            // Sukces identyfikacji
        } else {
            self.disconnect().await;
            return Err(RigError::Protocol {
                detail: "Unexpected response to ID; during probe".to_string(),
            });
        }

        // 2. Odczyt częstotliwości i emisji (read-only)
        let _ = self.query_and_await_response("FA;", "FA").await?;
        let _ = self.query_and_await_response("MD;", "MD").await?;

        let probed_state = self.state.clone();
        self.disconnect().await;
        Ok(probed_state)
    }
}

impl RigBackend for ElecraftBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::Elecraft
    }

    fn capabilities(&self) -> RigCapabilities {
        self.profile.capabilities
    }

    async fn connect(&mut self) -> Result<(), RigError> {
        if self.session_active {
            self.disconnect().await;
        }

        self.open_transport().await?;

        // Faza 1: Odczyt tożsamości i stanu początkowego (Strictly Read-Only)
        self.query_and_await_response("ID;", "ID").await?;
        let _ = self.query_and_await_response("FA;", "FA").await?;
        let _ = self.query_and_await_response("MD;", "MD").await;

        // Faza 2: Inicjalizacja sesji i Auto-Info (jeśli wspierane przez profil)
        if self.profile.supports_auto_info {
            let ai_cmd = ElecraftCat::set_auto_info(self.profile.preferred_auto_info_mode);
            match self.query_and_await_response(&ai_cmd, "AI").await {
                Ok(_) => {
                    self.ai_active = true;
                }
                Err(_) => {
                    // W razie braku akceptacji AI przez starszy firmware przejdź bezpiecznie do trybu pollingu
                    self.ai_active = false;
                }
            }
        }

        self.session_active = true;
        self.state.connected = true;
        Ok(())
    }

    async fn poll(&mut self) -> Result<RigState, RigError> {
        if !self.session_active || self.transport.is_none() {
            return Err(RigError::NotConnected);
        }

        // 1. Zawsze najpierw opróżnij bufor z nadchodzących zdarzeń Auto-Info
        self.drain_incoming_events().await?;

        // 2. Jeśli Auto-Info jest aktywne, zweryfikuj stan PTT, w przeciwnym razie odpytaj brakujące pola
        if self.ai_active {
            let _ = self.query_and_await_response("TQ;", "TQ").await;
        } else {
            let _ = self.query_and_await_response("FA;", "FA").await?;
            let _ = self.query_and_await_response("MD;", "MD").await?;
            let _ = self.query_and_await_response("TQ;", "TQ").await;
        }

        // 3. Odczyt mierników (w zależności od możliwości profilu)
        if self.profile.family == ElecraftProtocolFamily::K4 && self.profile.capabilities.s_meter {
            let _ = self.query_and_await_response("SMH;", "SMH").await;
        } else if self.profile.capabilities.power && self.state.ptt {
            let _ = self.query_and_await_response("PO;", "PO").await;
        }

        Ok(self.state.clone())
    }

    async fn execute(&mut self, command: &RigCommand) -> Result<(), RigError> {
        if !self.session_active || self.transport.is_none() {
            return Err(RigError::NotConnected);
        }

        match command {
            RigCommand::SetFrequency(hz) => {
                if !self.profile.capabilities.frequency {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetFrequency",
                    });
                }
                let cmd = ElecraftCat::set_freq(false, *hz);
                self.send_command_raw(&cmd).await?;
                self.state.frequency_hz = *hz;
                Ok(())
            }
            RigCommand::SetSplitFrequency(hz) => {
                if !self.profile.capabilities.split_frequency {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetSplitFrequency",
                    });
                }
                let cmd = ElecraftCat::set_freq(true, *hz);
                self.send_command_raw(&cmd).await?;
                self.state.tx_frequency_hz = Some(*hz);
                Ok(())
            }
            RigCommand::SetMode { mode, .. } => {
                if !self.profile.capabilities.mode {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetMode",
                    });
                }
                let (code, submode) =
                    self.profile
                        .map_mode_to_code(mode)
                        .ok_or(RigError::Unsupported {
                            backend: CatBackendKind::Elecraft,
                            command: "mode_not_supported",
                        })?;

                let cmd = ElecraftCat::set_mode(false, code);
                self.send_command_raw(&cmd).await?;

                if let Some(sm) = submode {
                    let sub_cmd = ElecraftCat::set_data_submode(false, sm);
                    self.send_command_raw(&sub_cmd).await?;
                }

                self.state.mode.clone_from(mode);
                Ok(())
            }
            RigCommand::SetPassband(hz) => {
                if !self.profile.capabilities.passband {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetPassband",
                    });
                }
                let cmd = ElecraftCat::set_passband(false, *hz);
                self.send_command_raw(&cmd).await?;
                self.state.passband_hz = *hz;
                Ok(())
            }
            RigCommand::SetSplit { enabled, .. } => {
                if !self.profile.capabilities.split {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetSplit",
                    });
                }
                let cmd = ElecraftCat::set_split(*enabled);
                self.send_command_raw(cmd).await?;
                self.state.split_enabled = *enabled;
                Ok(())
            }
            RigCommand::SetPtt(ptt) => {
                if !self.profile.capabilities.ptt {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetPtt",
                    });
                }

                if *ptt {
                    // Krytyczny rygor PTT: SetPtt(true) NIE JEST PONAWIANE w razie błędu/timeoutu
                    let cmd = ElecraftCat::set_ptt(true);
                    self.send_command_raw(cmd).await?;
                    self.confirmed_ptt = true;
                    self.state.ptt = true;
                } else {
                    // SetPtt(false): jednorazowa próba wyłączenia nadawania
                    let cmd = ElecraftCat::set_ptt(false);
                    self.send_command_raw(cmd).await?;
                    self.confirmed_ptt = false;
                    self.state.ptt = false;
                }
                Ok(())
            }
            RigCommand::SetVfo(vfo) => {
                if !self.profile.capabilities.vfo {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetVfo",
                    });
                }
                let vfo_b = vfo.to_ascii_uppercase().contains('B')
                    || vfo.to_ascii_uppercase().contains("SUB");
                let cmd = ElecraftCat::set_vfo(vfo_b);
                self.send_command_raw(cmd).await?;
                self.state.vfo = if vfo_b {
                    "VFOB".to_string()
                } else {
                    "VFOA".to_string()
                };
                Ok(())
            }
            RigCommand::SetRit(hz) => {
                if !self.profile.capabilities.rit {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetRit",
                    });
                }
                let enabled = *hz != 0;
                let cmd = ElecraftCat::set_rit(false, enabled);
                self.send_command_raw(cmd).await?;

                if enabled {
                    let off_cmd = ElecraftCat::set_rit_offset(false, *hz);
                    self.send_command_raw(&off_cmd).await?;
                }
                self.state.rit_hz = *hz;
                Ok(())
            }
            RigCommand::SetXit(hz) => {
                if !self.profile.capabilities.xit {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetXit",
                    });
                }
                let enabled = *hz != 0;
                let cmd = ElecraftCat::set_xit(false, enabled);
                self.send_command_raw(cmd).await?;

                if enabled {
                    let off_cmd = ElecraftCat::set_rit_offset(false, *hz);
                    self.send_command_raw(&off_cmd).await?;
                }
                self.state.xit_hz = *hz;
                Ok(())
            }
            RigCommand::SetPower(watts) => {
                if !self.profile.capabilities.power {
                    return Err(RigError::Unsupported {
                        backend: CatBackendKind::Elecraft,
                        command: "SetPower",
                    });
                }
                let is_k4 = self.profile.family == ElecraftProtocolFamily::K4;
                let cmd = ElecraftCat::set_power(*watts, is_k4);
                self.send_command_raw(&cmd).await?;
                self.state.rf_power_watts = *watts;
                Ok(())
            }
        }
    }

    async fn disconnect(&mut self) {
        if let Some(mut transport) = self.transport.take() {
            // Jeśli sesja była aktywna i potwierdzono załączenie PTT, wykonaj pojedynczą ograniczoną próbę RX;
            if self.session_active && self.confirmed_ptt && self.profile.capabilities.ptt {
                let rx_cmd = ElecraftCat::set_ptt(false);
                let bytes = self.codec.encode_command(rx_cmd);
                let _ =
                    tokio::time::timeout(Duration::from_millis(200), transport.write_all(&bytes))
                        .await;
                self.confirmed_ptt = false;
            }

            // Jeśli Auto-Info było aktywne, wykonaj pojedynczą ograniczoną próbę wyłączenia AI0;
            if self.session_active && self.ai_active && self.profile.supports_auto_info {
                let ai_off_cmd = ElecraftCat::set_auto_info(0);
                let bytes = self.codec.encode_command(&ai_off_cmd);
                let _ =
                    tokio::time::timeout(Duration::from_millis(200), transport.write_all(&bytes))
                        .await;
                self.ai_active = false;
            }
        }

        self.session_active = false;
        self.ai_active = false;
        self.confirmed_ptt = false;
        self.state.connected = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::native::elecraft::models::{ELECRAFT_K3_PROFILE, ELECRAFT_KX2_PROFILE};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    #[tokio::test]
    async fn elecraft_backend_connects_read_only_first() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Krok 1: Radio musi odebrać wyłącznie ID; jako pierwszy rozkaz
            let n = server.read(&mut buf).await.unwrap();
            let s = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(s, "ID;");
            server.write_all(b"ID017;").await.unwrap();

            // Następnie odpytanie stanu początkowego
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();

            // Faza 2: Aktywacja AI2;
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();
        });

        backend.connect().await.unwrap();
        assert!(backend.is_session_active());
        assert!(backend.is_ai_active());
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_probe_is_strictly_read_only() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Probe wysyła wyłącznie ID;, FA;, MD;
            let n = server.read(&mut buf).await.unwrap();
            let s = std::str::from_utf8(&buf[..n]).unwrap();
            assert!(!s.contains("TX"));
            assert!(!s.contains("RX"));
            assert!(!s.contains("AI"));
            server.write_all(b"ID017;").await.unwrap();

            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"FA00014074000;").await.unwrap();

            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"MD2;").await.unwrap();
        });

        let state = backend.probe().await.unwrap();
        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.mode, "USB");

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_connect_never_sends_tx() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 256];
            let mut all_received = String::new();
            while let Ok(n) = server.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                let s = std::str::from_utf8(&buf[..n]).unwrap();
                all_received.push_str(s);

                if s.contains("ID;") {
                    server.write_all(b"ID017;").await.unwrap();
                } else if s.contains("FA;") {
                    server
                        .write_all(
                            b"FA00014074000;FB00007074000;MD2;IF00014074000     +000000 000200001;",
                        )
                        .await
                        .unwrap();
                } else if s.contains("AI2;") {
                    server.write_all(b"AI2;").await.unwrap();
                    break;
                }
            }
            assert!(!all_received.contains("TX;"));
            assert!(!all_received.contains("TX1;"));
        });

        backend.connect().await.unwrap();
        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_enables_ai_only_after_confirmation() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;IF00014074000     +000000 000200001;")
                .await
                .unwrap();

            // Radio odrzuca komendę AI2; zwracając błąd ?;
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"?;").await.unwrap();
        });

        backend.connect().await.unwrap();
        // Ponieważ radio odrzuciło AI2;, backend musi pozostać w trybie ai_active == false
        assert!(!backend.is_ai_active());
        assert!(backend.is_session_active());

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_poll_reads_state() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // W pętli poll backend odpytuje TQ;
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"TQ0;").await.unwrap();
        });

        backend.connect().await.unwrap();
        let state = backend.poll().await.unwrap();
        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.mode, "USB");
        assert!(!state.ptt);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_execute_frequency() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // Odbierz SetFrequency
            let n = server.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "FA00007040000;");
        });

        backend.connect().await.unwrap();
        backend
            .execute(&RigCommand::SetFrequency(7_040_000))
            .await
            .unwrap();
        assert_eq!(backend.current_state().frequency_hz, 7_040_000);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_execute_mode() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // Odbierz SetMode("CW") -> MD3;
            let n = server.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "MD3;");
        });

        backend.connect().await.unwrap();
        backend
            .execute(&RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            })
            .await
            .unwrap();
        assert_eq!(backend.current_state().mode, "CW");

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_execute_split() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // SetSplit(true) -> FT1;
            let n = server.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "FT1;");
        });

        backend.connect().await.unwrap();
        backend
            .execute(&RigCommand::SetSplit {
                enabled: true,
                tx_vfo: "VFOB".to_string(),
            })
            .await
            .unwrap();
        assert!(backend.current_state().split_enabled);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_execute_ptt() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // Odbierz TX; oraz RX;
            let mut read_str = String::new();
            while !read_str.contains("RX;") {
                let n = server.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                read_str.push_str(std::str::from_utf8(&buf[..n]).unwrap());
            }
            assert!(read_str.contains("TX;"));
            assert!(read_str.contains("RX;"));
        });

        backend.connect().await.unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();
        assert!(backend.current_state().ptt);

        backend.execute(&RigCommand::SetPtt(false)).await.unwrap();
        assert!(!backend.current_state().ptt);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_rejects_unsupported_command() {
        let (client, mut server) = duplex(1024);
        // KX2 nie wspiera subreceivera ani nieobsługiwanych trybów
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_KX2_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();
        });

        backend.connect().await.unwrap();
        // Emisja niewspierana
        let res = backend
            .execute(&RigCommand::SetMode {
                mode: "UNKNOWN_MODE".to_string(),
                passband_hz: 500,
            })
            .await;
        assert!(matches!(res, Err(RigError::Unsupported { .. })));

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_timeout_is_typed() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            // Serwer milczy (nie odpowiada na ID;)
            tokio::time::sleep(Duration::from_millis(2000)).await;
        });

        let res = backend.connect().await;
        assert!(matches!(res, Err(RigError::Timeout { .. })));

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_disconnect_is_idempotent() {
        let (client, _) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        backend.disconnect().await;
        backend.disconnect().await;
        backend.disconnect().await;
        assert!(!backend.is_session_active());
    }

    #[tokio::test]
    async fn elecraft_backend_ptt_true_is_not_retried() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // Zamykamy połączenie od strony serwera tuż przed PTT
            drop(server);
        });

        backend.connect().await.unwrap();
        // Próba PTT przy zerwanym połączeniu
        let res = backend.execute(&RigCommand::SetPtt(true)).await;
        assert!(res.is_err());
        assert!(!backend.confirmed_ptt);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_handles_interleaved_ai_and_response() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // Backend pyta o TQ;, a serwer najpierw zwraca spontaniczne Auto-Info BG;
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"BG05R;TQ0;").await.unwrap();
        });

        backend.connect().await.unwrap();
        let st = backend.poll().await.unwrap();
        assert!(!st.ptt);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_backend_partial_reads_preserve_data() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            // Wysyłamy w kawałkach:
            server.write_all(b"FA00014").await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            server
                .write_all(b"074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();
        });

        backend.connect().await.unwrap();
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);

        server_jh.await.unwrap();
    }

    #[test]
    fn elecraft_backend_capabilities_match_profile() {
        let (client, _) = duplex(64);
        let backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());
        let caps = backend.capabilities();
        assert!(caps.frequency);
        assert!(caps.mode);
        assert!(caps.ptt);
        assert!(caps.power);
        assert!(!caps.s_meter);
    }

    #[tokio::test]
    async fn elecraft_disconnect_skips_ai_off_when_ai_inactive() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            // Odrzucenie AI
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"?;").await.unwrap();

            // Przy rozłączeniu AI0; nie powinno zostać wysłane
            let n = server.read(&mut buf).await.unwrap_or(0);
            if n > 0 {
                let s = std::str::from_utf8(&buf[..n]).unwrap();
                assert!(!s.contains("AI0;"));
            }
        });

        backend.connect().await.unwrap();
        assert!(!backend.is_ai_active());
        backend.disconnect().await;

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_disconnect_sends_ai_off_once_when_active() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // Przy rozłączeniu powinno przyjść AI0;
            let n = server.read(&mut buf).await.unwrap();
            let s = std::str::from_utf8(&buf[..n]).unwrap();
            assert!(s.contains("AI0;"));
        });

        backend.connect().await.unwrap();
        assert!(backend.is_ai_active());
        backend.disconnect().await;

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_disconnect_skips_rx_when_ptt_not_confirmed() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            let n = server.read(&mut buf).await.unwrap_or(0);
            if n > 0 {
                let s = std::str::from_utf8(&buf[..n]).unwrap();
                // Nie było PTT, więc nie wysyłaj RX;
                assert!(!s.contains("RX;"));
            }
        });

        backend.connect().await.unwrap();
        assert!(!backend.confirmed_ptt);
        backend.disconnect().await;

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_disconnect_sends_rx_once_when_ptt_confirmed() {
        let (client, mut server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());

        let server_jh = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"ID017;").await.unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = server.read(&mut buf).await.unwrap();
            server.write_all(b"AI2;").await.unwrap();

            // SetPtt(true) oraz rozłączenie z confirmed_ptt == true wysyła RX;
            let mut s = String::new();
            while !s.contains("RX;") {
                let n = server.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                s.push_str(std::str::from_utf8(&buf[..n]).unwrap());
            }
            assert!(s.contains("TX;"));
            assert!(s.contains("RX;"));
        });

        backend.connect().await.unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();
        assert!(backend.confirmed_ptt);

        backend.disconnect().await;
        assert!(!backend.confirmed_ptt);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_disconnect_cleanup_is_bounded() {
        let (client, server) = duplex(1024);
        let mut backend = ElecraftBackend::with_stream(client, ELECRAFT_K3_PROFILE.clone());
        drop(server); // Druga strona jest zamknięta

        // Disconnect nie może wisieć ani panikować
        let start = std::time::Instant::now();
        backend.disconnect().await;
        assert!(start.elapsed() < Duration::from_millis(500));
    }
}
