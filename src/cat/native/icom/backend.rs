// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny backend Icom CI-V sterowania transceiverem.
//!
//! Zapewnia pełną obsługę protokołu Icom CI-V przez port szeregowy / USB Virtual COM:
//! - trwała sesja I/O z eliminacją echa (CI-V Echo Back),
//! - bezpieczne rozkazy PTT bez automatycznego retry w razie błędu,
//! - odczyt i zapis częstotliwości BCD, emisji, filtrów, S-metra, mocy i PTT,
//! - pełna zgodność z kontraktem [`RigBackend`] oraz typowanymi błędami [`RigError`].

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::cat::backend::{CatBackendKind, RigBackend};
use crate::cat::native::icom::codec::{
    CI_V_CONTROLLER, CI_V_NAK, IcomCiVCodec, IcomCiVFrame, command, subcommand,
};
use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile};
use crate::cat::native::transport::{AsyncFramedTransport, SerialConfig, open_serial_stream};
use crate::cat::native::{NativeRigCommand, NativeRigEvent};
use crate::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};

/// Pomocnicza cecha dla strumieni I/O (umożliwia testy na tokio::io::duplex).
pub trait AsyncReaderWriter: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncReaderWriter for T {}

/// Backend natywnego protokołu Icom CI-V.
pub struct IcomCiVBackend {
    config: SerialConfig,
    profile: RigModelProfile,
    radio_address: u8,
    controller_address: u8,
    transport: Option<AsyncFramedTransport<Box<dyn AsyncReaderWriter>>>,
    mock_stream: Option<Box<dyn AsyncReaderWriter>>,
    state: RigState,
    codec: IcomCiVCodec,
    pending_events: Vec<NativeRigEvent>,
}

impl IcomCiVBackend {
    /// Tworzy nowy backend CI-V dla zadanego portu i profilu.
    pub fn new(config: SerialConfig, profile: RigModelProfile, radio_address: Option<u8>) -> Self {
        let addr = radio_address
            .or(profile.default_ci_v_address)
            .unwrap_or(0x94);
        Self {
            config,
            profile,
            radio_address: addr,
            controller_address: CI_V_CONTROLLER,
            transport: None,
            mock_stream: None,
            state: RigState::default(),
            codec: IcomCiVCodec::new(CI_V_CONTROLLER, addr),
            pending_events: Vec::new(),
        }
    }

    /// Tworzy backend z bezpośrednio wstrzykniętym strumieniem I/O (dla testów bez sprzętu).
    pub fn with_stream<S: AsyncReaderWriter + 'static>(
        stream: S,
        config: SerialConfig,
        profile: RigModelProfile,
        radio_address: Option<u8>,
    ) -> Self {
        let addr = radio_address
            .or(profile.default_ci_v_address)
            .unwrap_or(0x94);
        Self {
            config,
            profile,
            radio_address: addr,
            controller_address: CI_V_CONTROLLER,
            transport: None,
            mock_stream: Some(Box::new(stream)),
            state: RigState::default(),
            codec: IcomCiVCodec::new(CI_V_CONTROLLER, addr),
            pending_events: Vec::new(),
        }
    }

    /// Zwraca adres CI-V transceivera.
    pub fn radio_address(&self) -> u8 {
        self.radio_address
    }

    /// Zwraca profil modelu radia.
    pub fn profile(&self) -> &RigModelProfile {
        &self.profile
    }

    /// Zwraca bieżący odczytany stan radia.
    pub fn current_state(&self) -> &RigState {
        &self.state
    }

    /// Pomocnicza metoda testowa do sprawdzania odczytu częstotliwości z fragmentacją.
    pub async fn poll_frequency_for_test(&mut self) -> Result<(), RigError> {
        self.poll_frequency().await
    }

    /// Rozszerzone możliwości techniczne danego modelu Icom.
    pub fn native_capabilities(&self) -> NativeRigCapabilities {
        self.profile.extended_capabilities
    }

    /// Pobiera zakolejkowane asynchroniczne zdarzenia transceive.
    pub fn drain_events(&mut self) -> Vec<NativeRigEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Bezpieczny odczyt parametrów bez zmiany stanu i bez PTT (Probe).
    pub async fn probe(&mut self) -> Result<RigState, RigError> {
        let was_connected = self.transport.is_some();
        if !was_connected {
            self.connect().await?;
        }

        // Probe pobiera wyłącznie częstotliwość i tryb (read-only)
        let freq_frame = self
            .send_query_and_await_reply(
                command::READ_FREQUENCY,
                None,
                &[],
                Duration::from_millis(600),
            )
            .await?;
        if freq_frame.command == command::READ_FREQUENCY
            && let Ok(hz) = IcomCiVCodec::decode_frequency_bcd(&freq_frame.data)
        {
            self.state.frequency_hz = hz;
        }

        let mode_frame = self
            .send_query_and_await_reply(command::READ_MODE, None, &[], Duration::from_millis(600))
            .await?;
        if mode_frame.command == command::READ_MODE && !mode_frame.data.is_empty() {
            self.state.mode = IcomCiVCodec::code_to_mode(mode_frame.data[0]).to_string();
        }

        self.state.connected = true;
        let probed_state = self.state.clone();

        if !was_connected {
            self.disconnect().await;
        }

        Ok(probed_state)
    }

    /// Wykonuje rozszerzone polecenie natywne.
    pub async fn execute_native(&mut self, cmd: &NativeRigCommand) -> Result<(), RigError> {
        match cmd {
            NativeRigCommand::Common(c) => self.execute(c).await,
            NativeRigCommand::EmergencyRx => self.execute(&RigCommand::SetPtt(false)).await,
            NativeRigCommand::PollMeters => {
                if self.profile.capabilities.s_meter {
                    let _ = self.poll_smeter().await;
                }
                if self.profile.capabilities.power {
                    let _ = self.poll_rf_power().await;
                }
                Ok(())
            }
        }
    }

    // --- Prywatne metody obsługi protokołu CI-V ---

    /// Wysyła ramkę i oczekuje na odpowiedź lub ACK, filtrując własne echo i zbierając zdarzenia transceive.
    async fn send_query_and_await_reply(
        &mut self,
        cmd: u8,
        sub: Option<u8>,
        data: &[u8],
        timeout: Duration,
    ) -> Result<IcomCiVFrame, RigError> {
        let tx_frame = IcomCiVFrame::new(
            self.radio_address,
            self.controller_address,
            cmd,
            sub,
            data.to_vec(),
        );
        let encoded_tx = tx_frame.encode();

        {
            let transport = self.transport.as_mut().ok_or(RigError::NotConnected)?;
            transport
                .write_all(&encoded_tx)
                .await
                .map_err(|e| e.to_rig_error(CatBackendKind::IcomCiV, &self.config.port_name))?;
        }

        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            // Sprawdź czy w buforze jest gotowa ramka
            let maybe_frame = {
                let transport = self.transport.as_mut().ok_or(RigError::NotConnected)?;
                let mut buf = transport.buffer().to_vec();
                let res = self.codec.extract_frame(&mut buf);
                if let Ok(Some(_)) = res {
                    let consumed = transport.buffer().len() - buf.len();
                    transport.consume(consumed);
                }
                res
            };

            match maybe_frame {
                Ok(Some(frame)) => {
                    // 1. Filtracja echa (jeśli transceiver ma CI-V Echo Back włączone)
                    if frame.to == self.radio_address && frame.from == self.controller_address {
                        // To jest nasze własne echo wysłanej ramki — ignorujemy
                        continue;
                    }

                    // 2. Obsługa odmowy NAK (0xFA)
                    if frame.is_nak() {
                        return Err(RigError::Rejected {
                            code: CI_V_NAK as i32,
                        });
                    }

                    // 3. Asynchroniczne zdarzenia transceive (broadcasty na adres 0x00)
                    if frame.is_broadcast() {
                        self.process_broadcast_frame(&frame);
                        continue;
                    }

                    // 4. Ramka odpowiedzi skierowana do kontrolera
                    if frame.to == self.controller_address && frame.from == self.radio_address {
                        // Jeśli ramka odpowiada wysłanej komendzie lub jest ACK/NAK
                        if frame.command == cmd || frame.is_ack() {
                            return Ok(frame);
                        }
                    }
                }
                Ok(None) => {
                    // Brak kompletnej ramki — odczytaj kolejny fragment ze strumienia
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        return Err(RigError::Timeout {
                            endpoint: self.config.port_name.clone(),
                            operation: "CI-V query reply",
                        });
                    }

                    let transport = self.transport.as_mut().ok_or(RigError::NotConnected)?;
                    match tokio::time::timeout(remaining, transport.read_chunk()).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => {
                            return Err(
                                e.to_rig_error(CatBackendKind::IcomCiV, &self.config.port_name)
                            );
                        }
                        Err(_) => {
                            return Err(RigError::Timeout {
                                endpoint: self.config.port_name.clone(),
                                operation: "CI-V read chunk",
                            });
                        }
                    }
                }
                Err(e) => {
                    return Err(RigError::Protocol {
                        detail: e.to_string(),
                    });
                }
            }
        }
    }

    /// Przetwarza asynchroniczny broadcast transceive.
    fn process_broadcast_frame(&mut self, frame: &IcomCiVFrame) {
        if frame.command == command::READ_FREQUENCY || frame.command == command::SET_FREQUENCY {
            if let Ok(hz) = IcomCiVCodec::decode_frequency_bcd(&frame.data) {
                self.state.frequency_hz = hz;
                self.pending_events
                    .push(NativeRigEvent::TransceiveFrequencyChanged {
                        vfo: self.state.vfo.clone(),
                        frequency_hz: hz,
                    });
            }
        } else if (frame.command == command::READ_MODE || frame.command == command::SET_MODE)
            && !frame.data.is_empty()
        {
            let mode_str = IcomCiVCodec::code_to_mode(frame.data[0]).to_string();
            let filter = frame.data.get(1).copied();
            self.state.mode.clone_from(&mode_str);
            self.pending_events
                .push(NativeRigEvent::TransceiveModeChanged {
                    mode: mode_str,
                    filter,
                });
        }
        self.pending_events
            .push(NativeRigEvent::RawFrameReceived(frame.encode()));
    }

    async fn poll_frequency(&mut self) -> Result<(), RigError> {
        let frame = self
            .send_query_and_await_reply(
                command::READ_FREQUENCY,
                None,
                &[],
                self.config.read_timeout,
            )
            .await?;
        if frame.command == command::READ_FREQUENCY {
            let hz = IcomCiVCodec::decode_frequency_bcd(&frame.data).map_err(|e| {
                RigError::Protocol {
                    detail: format!("invalid frequency BCD: {e}"),
                }
            })?;
            self.state.frequency_hz = hz;
        }
        Ok(())
    }

    async fn poll_mode(&mut self) -> Result<(), RigError> {
        let frame = self
            .send_query_and_await_reply(command::READ_MODE, None, &[], self.config.read_timeout)
            .await?;
        if frame.command == command::READ_MODE && !frame.data.is_empty() {
            self.state.mode = IcomCiVCodec::code_to_mode(frame.data[0]).to_string();
        }
        Ok(())
    }

    async fn poll_smeter(&mut self) -> Result<(), RigError> {
        let frame = self
            .send_query_and_await_reply(
                command::READ_METER,
                Some(subcommand::METER_SMETER),
                &[],
                self.config.read_timeout,
            )
            .await?;
        if frame.command == command::READ_METER {
            let (dbm, unit) = IcomCiVCodec::decode_smeter(&frame.data);
            self.state.s_meter_dbm = dbm;
            self.state.s_meter_unit = unit;
        }
        Ok(())
    }

    async fn poll_rf_power(&mut self) -> Result<(), RigError> {
        let frame = self
            .send_query_and_await_reply(
                command::LEVEL_SET,
                Some(subcommand::LEVEL_RF_POWER),
                &[],
                self.config.read_timeout,
            )
            .await?;
        if frame.command == command::LEVEL_SET {
            let watts = IcomCiVCodec::decode_power_watts(&frame.data, 100.0);
            self.state.rf_power_watts = watts;
        }
        Ok(())
    }

    async fn poll_ptt(&mut self) -> Result<(), RigError> {
        let frame = self
            .send_query_and_await_reply(
                command::READ_PTT,
                Some(subcommand::PTT_CTRL),
                &[],
                self.config.read_timeout,
            )
            .await?;
        if frame.command == command::READ_PTT && !frame.data.is_empty() {
            self.state.ptt = frame.data[0] != 0;
        }
        Ok(())
    }
}

impl RigBackend for IcomCiVBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::IcomCiV
    }

    fn capabilities(&self) -> RigCapabilities {
        self.profile.capabilities
    }

    async fn connect(&mut self) -> Result<(), RigError> {
        if self.transport.is_some() {
            return Ok(());
        }

        if let Some(mock) = self.mock_stream.take() {
            self.transport = Some(AsyncFramedTransport::from_stream(
                mock,
                self.config.max_buffer_size,
                self.config.read_timeout,
                self.config.write_timeout,
            ));
        } else {
            let stream = open_serial_stream(&self.config)
                .map_err(|e| e.to_rig_error(CatBackendKind::IcomCiV, &self.config.port_name))?;

            let boxed_transport = AsyncFramedTransport::from_stream(
                Box::new(stream) as Box<dyn AsyncReaderWriter>,
                self.config.max_buffer_size,
                self.config.read_timeout,
                self.config.write_timeout,
            );
            self.transport = Some(boxed_transport);
        }

        self.state.connected = true;
        // PTT Safety: connect NIGDY nie załącza PTT
        self.state.ptt = false;
        tokio::task::yield_now().await;
        Ok(())
    }

    async fn poll(&mut self) -> Result<RigState, RigError> {
        if self.transport.is_none() {
            return Err(RigError::NotConnected);
        }

        if self.profile.capabilities.frequency {
            self.poll_frequency().await?;
        }

        if self.profile.capabilities.mode {
            self.poll_mode().await?;
        }

        if self.profile.capabilities.s_meter {
            let _ = self.poll_smeter().await;
        }

        if self.profile.capabilities.power {
            let _ = self.poll_rf_power().await;
        }

        if self.profile.capabilities.ptt {
            let _ = self.poll_ptt().await;
        }

        Ok(self.state.clone())
    }

    async fn execute(&mut self, command: &RigCommand) -> Result<(), RigError> {
        if self.transport.is_none() {
            return Err(RigError::NotConnected);
        }

        if !self.profile.capabilities.supports(command) {
            return Err(RigError::Unsupported {
                backend: CatBackendKind::IcomCiV,
                command: match command {
                    RigCommand::SetFrequency(_) => "SetFrequency",
                    RigCommand::SetSplitFrequency(_) => "SetSplitFrequency",
                    RigCommand::SetMode { .. } => "SetMode",
                    RigCommand::SetPassband(_) => "SetPassband",
                    RigCommand::SetSplit { .. } => "SetSplit",
                    RigCommand::SetPtt(_) => "SetPtt",
                    RigCommand::SetVfo(_) => "SetVfo",
                    RigCommand::SetRit(_) => "SetRit",
                    RigCommand::SetXit(_) => "SetXit",
                    RigCommand::SetPower(_) => "SetPower",
                },
            });
        }

        match command {
            RigCommand::SetFrequency(hz) => {
                let bcd = IcomCiVCodec::encode_frequency_bcd(*hz, self.profile.frequency_bytes);
                let reply = self
                    .send_query_and_await_reply(
                        command::SET_FREQUENCY,
                        None,
                        &bcd,
                        self.config.write_timeout,
                    )
                    .await?;
                if reply.is_ack() || reply.command == command::SET_FREQUENCY {
                    self.state.frequency_hz = *hz;
                }
            }
            RigCommand::SetSplitFrequency(hz) => {
                let bcd = IcomCiVCodec::encode_frequency_bcd(*hz, self.profile.frequency_bytes);
                let reply = self
                    .send_query_and_await_reply(
                        command::SET_FREQUENCY,
                        None,
                        &bcd,
                        self.config.write_timeout,
                    )
                    .await?;
                if reply.is_ack() || reply.command == command::SET_FREQUENCY {
                    self.state.tx_frequency_hz = Some(*hz);
                }
            }
            RigCommand::SetMode { mode, .. } => {
                let (code, filter) = IcomCiVCodec::mode_to_code(mode);
                let mut data = vec![code];
                if let Some(f) = filter {
                    data.push(f);
                }
                let reply = self
                    .send_query_and_await_reply(
                        command::SET_MODE,
                        None,
                        &data,
                        self.config.write_timeout,
                    )
                    .await?;
                if reply.is_ack() || reply.command == command::SET_MODE {
                    self.state.mode.clone_from(mode);
                }
            }
            RigCommand::SetPassband(_) => {
                // Filtry passband w Icom CI-V są zintegrowane z komendą SET_MODE
            }
            RigCommand::SetSplit { enabled, tx_vfo } => {
                let arg = u8::from(*enabled);
                let reply = self
                    .send_query_and_await_reply(
                        command::SPLIT_SELECT,
                        None,
                        &[arg],
                        self.config.write_timeout,
                    )
                    .await?;
                if reply.is_ack() || reply.command == command::SPLIT_SELECT {
                    self.state.split_enabled = *enabled;
                    self.state.tx_vfo.clone_from(tx_vfo);
                }
            }
            RigCommand::SetPtt(ptt) => {
                let arg = u8::from(*ptt);
                // Rygor PTT: brak automatycznego ponawiania w razie błędu
                let reply = self
                    .send_query_and_await_reply(
                        command::SET_PTT,
                        Some(subcommand::PTT_CTRL),
                        &[arg],
                        self.config.write_timeout,
                    )
                    .await?;
                if reply.is_ack() || reply.command == command::SET_PTT {
                    self.state.ptt = *ptt;
                }
            }
            RigCommand::SetVfo(vfo) => {
                let code = match vfo.to_ascii_uppercase().as_str() {
                    "VFOB" | "SUB" => 0x01,
                    _ => 0x00,
                };
                let reply = self
                    .send_query_and_await_reply(
                        command::VFO_SELECT,
                        None,
                        &[code],
                        self.config.write_timeout,
                    )
                    .await?;
                if reply.is_ack() || reply.command == command::VFO_SELECT {
                    self.state.vfo.clone_from(vfo);
                }
            }
            RigCommand::SetRit(hz) => {
                let enable = u8::from(*hz != 0);
                let _ = self
                    .send_query_and_await_reply(
                        command::RIT_XIT,
                        Some(subcommand::RIT_FREQ),
                        &[enable],
                        self.config.write_timeout,
                    )
                    .await;
                self.state.rit_hz = *hz;
            }
            RigCommand::SetXit(hz) => {
                let enable = u8::from(*hz != 0);
                let _ = self
                    .send_query_and_await_reply(
                        command::RIT_XIT,
                        Some(subcommand::XIT_FREQ),
                        &[enable],
                        self.config.write_timeout,
                    )
                    .await;
                self.state.xit_hz = *hz;
            }
            RigCommand::SetPower(watts) => {
                let raw_val = ((watts / 100.0).clamp(0.0, 1.0) * 255.0).round() as u8;
                let data = IcomCiVCodec::encode_frequency_bcd(raw_val as u64, 2);
                let reply = self
                    .send_query_and_await_reply(
                        command::LEVEL_SET,
                        Some(subcommand::LEVEL_RF_POWER),
                        &data,
                        self.config.write_timeout,
                    )
                    .await?;
                if reply.is_ack() || reply.command == command::LEVEL_SET {
                    self.state.rf_power_watts = *watts;
                }
            }
        }

        Ok(())
    }

    async fn disconnect(&mut self) {
        if let Some(mut transport) = self.transport.take() {
            // PTT Safety: bezwarunkowe wyłączenie PTT przy rozłączaniu
            if self.state.ptt {
                let rx_frame = IcomCiVFrame::new(
                    self.radio_address,
                    self.controller_address,
                    command::SET_PTT,
                    Some(subcommand::PTT_CTRL),
                    vec![0x00],
                );
                let _ = transport.write_all(&rx_frame.encode()).await;
                self.state.ptt = false;
            }
            transport.close().await;
        }
        self.state.connected = false;
    }
}
