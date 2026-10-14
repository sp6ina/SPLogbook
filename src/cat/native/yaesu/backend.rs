// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny backend Yaesu CAT sterowania transceiverem.
//!
//! Zapewnia bezpośrednią, asynchroniczną komunikację ze sprzętem
//! przez porty szeregowe (RS-232C, USB Virtual COM / Enhanced COM port):
//! - rygorystyczne bezpieczeństwo PTT (brak PTT podczas connect/probe, brak ponawiania SetPtt(true)),
//! - pełna obsługa trybu Auto Information (`AI1;`) z redukcją nadmiarowego pollingu,
//! - dekodowanie ramek transceivera (`FA`, `FB`, `MD0`, `TX`, `SM0`, `FT`, `IF`),
//! - wykrywanie ramek błędu `?;`,
//! - pełna zgodność z kontraktem [`RigBackend`] oraz typowanymi błędami [`RigError`].

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::cat::backend::{BoxFuture, CatBackendKind, RigBackend};
use crate::cat::native::profile::RigModelProfile;
use crate::cat::native::transport::{AsyncFramedTransport, SerialConfig, open_serial_stream};
use crate::cat::native::yaesu::codec::{YaesuAsciiCodec, YaesuCat, YaesuFrame, YaesuIfInfo};
use crate::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};

/// Pomocnicza cecha dla strumieni I/O (umożliwia testy na tokio::io::duplex).
pub trait AsyncReaderWriter: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncReaderWriter for T {}

/// Backend natywnego protokołu Yaesu CAT.
pub struct YaesuBackend {
    profile: RigModelProfile,
    serial_config: Option<SerialConfig>,
    transport: Option<AsyncFramedTransport<Box<dyn AsyncReaderWriter>>>,
    mock_stream: Option<Box<dyn AsyncReaderWriter>>,
    state: RigState,
    codec: YaesuAsciiCodec,
    ai_active: bool,
    confirmed_ptt: bool,
    raw_smeter: Option<u16>,
}

impl YaesuBackend {
    /// Tworzy nowy backend dla połączenia szeregowego / USB Virtual COM.
    pub fn new_serial(config: SerialConfig, profile: RigModelProfile) -> Self {
        Self {
            profile,
            serial_config: Some(config),
            transport: None,
            mock_stream: None,
            state: RigState::default(),
            codec: YaesuAsciiCodec::new(),
            ai_active: false,
            confirmed_ptt: false,
            raw_smeter: None,
        }
    }

    /// Tworzy backend z bezpośrednio wstrzykniętym strumieniem I/O (dla testów bez sprzętu).
    pub fn with_stream<S: AsyncReaderWriter + 'static>(
        stream: S,
        profile: RigModelProfile,
    ) -> Self {
        Self {
            profile,
            serial_config: None,
            transport: None,
            mock_stream: Some(Box::new(stream)),
            state: RigState::default(),
            codec: YaesuAsciiCodec::new(),
            ai_active: false,
            confirmed_ptt: false,
            raw_smeter: None,
        }
    }

    /// Zwraca profil modelu radia.
    pub fn profile(&self) -> &RigModelProfile {
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

    /// Zwraca ostatnio odczytaną surową wartość S-metra (000..255).
    /// Odczyt pochodzi bezpośrednio z transceivera (komenda SM0;) i nie jest
    /// poddawany arbitralnemu mapowaniu do nieoficjalnych progów S-jednostek ani dBm.
    pub fn raw_smeter(&self) -> Option<u16> {
        self.raw_smeter
    }

    /// Zwraca bieżący endpoint do komunikatów diagnostycznych.
    fn endpoint_name(&self) -> String {
        if let Some(ref serial) = self.serial_config {
            serial.port_name.clone()
        } else {
            "MOCK_STREAM".to_string()
        }
    }

    /// Zapewnia zainicjalizowany transport ramkowy (ze strumienia szeregowego lub mock streamu).
    fn ensure_transport(&mut self) -> Result<(), RigError> {
        let endpoint = self.endpoint_name();
        if self.transport.is_none() {
            if let Some(mock) = self.mock_stream.take() {
                let framed = AsyncFramedTransport::from_stream(
                    mock,
                    1024,
                    Duration::from_millis(1500),
                    Duration::from_millis(1500),
                );
                self.transport = Some(framed);
            } else if let Some(ref config) = self.serial_config {
                let stream = open_serial_stream(config)
                    .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                let boxed_stream: Box<dyn AsyncReaderWriter> = Box::new(stream);
                let framed = AsyncFramedTransport::from_stream(
                    boxed_stream,
                    1024,
                    Duration::from_millis(1500),
                    Duration::from_millis(1500),
                );
                self.transport = Some(framed);
            } else {
                return Err(RigError::Connect {
                    endpoint: "unspecified".to_string(),
                    detail: "neither serial config nor mock stream provided".to_string(),
                });
            }
        }
        Ok(())
    }

    /// Bezpieczny odczyt parametrów bez zmiany konfiguracji radia i bez PTT (Probe).
    /// Probe jest operacją ściśle tylko do odczytu (read-only): wysyła wyłącznie zapytania
    /// diagnostyczne (np. FA;) i pod żadnym pozorem nie wysyła komend modyfikujących
    /// konfigurację transceivera (np. AI1;, TX1;).
    pub async fn probe(&mut self) -> Result<RigState, RigError> {
        self.ensure_transport()?;

        if let Some(t) = self.transport.as_mut() {
            t.clear_buffer();
        }

        let probe_frame = self
            .roundtrip_command(YaesuCat::read_freq(false), "FA")
            .await?;
        if let Some((_, hz)) = YaesuCat::parse_freq_resp(&probe_frame.raw) {
            self.state.frequency_hz = hz;
        }

        Ok(self.state.clone())
    }

    /// Przetwarza zdekodowaną ramkę Yaesu i aktualizuje lokalny stan radia.
    fn handle_incoming_frame(&mut self, frame: &YaesuFrame) {
        match frame.command.as_str() {
            "FA" => {
                if let Some((_, hz)) = YaesuCat::parse_freq_resp(&frame.raw) {
                    self.state.frequency_hz = hz;
                }
            }
            "FB" => {
                if let Some((_, hz)) = YaesuCat::parse_freq_resp(&frame.raw) {
                    self.state.tx_frequency_hz = Some(hz);
                }
            }
            "MD0" | "MD" => {
                if let Some(mode) =
                    YaesuCat::parse_mode_resp_for_model(self.profile.model_id, &frame.raw)
                {
                    self.state.mode = mode.to_string();
                }
            }
            "TX" => {
                if let Some(ptt) = YaesuCat::parse_ptt_resp(&frame.raw) {
                    self.state.ptt = ptt;
                    self.confirmed_ptt = ptt;
                }
            }
            "SM0" | "SM" => {
                if let Some(raw) = YaesuCat::parse_smeter_resp(&frame.raw) {
                    self.raw_smeter = Some(raw);
                }
            }
            "FT" => {
                if let Some(split) = YaesuCat::parse_split_resp(&frame.raw) {
                    self.state.split_enabled = split;
                }
            }
            "IF" => {
                if let Some(info) = YaesuIfInfo::parse_for_model(self.profile.model_id, &frame.raw)
                {
                    self.state.frequency_hz = info.frequency_hz;
                    self.state.mode = info.mode.to_string();
                    self.state.ptt = info.tx;
                    self.confirmed_ptt = info.tx;
                    self.state.split_enabled = info.split;
                    self.state.rit_hz = info.rit_offset_hz;
                    self.state.vfo = if info.vfo_b {
                        "VFOB".to_string()
                    } else {
                        "VFOA".to_string()
                    };
                }
            }
            _ => {}
        }
    }

    /// Odbiera oczekujące w buforze lub na łączu ramki (z krótkim timeoutem).
    async fn drain_incoming(&mut self) -> Result<(), RigError> {
        let endpoint = self.endpoint_name();
        let mut frames = Vec::new();

        if let Some(transport) = self.transport.as_mut() {
            // 1. Zdekoduj wszystkie kompletne ramki już obecne w buforze transportu
            loop {
                match self.codec.decode_frame(transport.buffer()) {
                    Ok(Some((frame, consumed))) => {
                        transport.consume(consumed);
                        if !frame.command.is_empty() {
                            frames.push(frame);
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        transport.clear_buffer();
                        return Err(RigError::Protocol {
                            detail: format!("Yaesu decode error: {e}"),
                        });
                    }
                }
            }

            // 2. Spróbuj pobrać kolejny pakiet z krótkim timeoutem (50 ms)
            let read_future = transport.read_chunk();
            if let Ok(res) = tokio::time::timeout(Duration::from_millis(50), read_future).await {
                match res {
                    Ok(_) => {
                        while let Ok(Some((frame, consumed))) =
                            self.codec.decode_frame(transport.buffer())
                        {
                            transport.consume(consumed);
                            if !frame.command.is_empty() {
                                frames.push(frame);
                            }
                        }
                    }
                    Err(crate::cat::native::transport::TransportError::Closed) => {
                        return Err(RigError::NotConnected);
                    }
                    Err(e) => {
                        return Err(e.to_rig_error(CatBackendKind::Yaesu, &endpoint));
                    }
                }
            }
        } else {
            return Err(RigError::NotConnected);
        }

        for frame in frames {
            self.handle_incoming_frame(&frame);
        }

        Ok(())
    }

    /// Wysyła komendę i oczekuje na odpowiedź o zadanym prefiksie (np. "FA", "MD", "TX", "IF", "SM").
    async fn roundtrip_command(
        &mut self,
        cmd: &str,
        expected_cmd: &str,
    ) -> Result<YaesuFrame, RigError> {
        let endpoint = self.endpoint_name();
        let encoded = self.codec.encode_command(cmd);

        let Some(transport) = self.transport.as_mut() else {
            return Err(RigError::NotConnected);
        };

        transport
            .write_all(&encoded)
            .await
            .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;

        let start = tokio::time::Instant::now();
        let timeout_duration = Duration::from_millis(1500);

        loop {
            if start.elapsed() >= timeout_duration {
                return Err(RigError::Timeout {
                    endpoint,
                    operation: "roundtrip_command",
                });
            }

            let mut matched_frame = None;
            let mut frames_to_apply = Vec::new();
            let mut has_error = None;

            {
                let Some(transport) = self.transport.as_mut() else {
                    return Err(RigError::NotConnected);
                };

                match self.codec.decode_frame(transport.buffer()) {
                    Ok(Some((frame, consumed))) => {
                        transport.consume(consumed);
                        if frame.is_error() {
                            has_error = Some(frame.raw.clone());
                        } else if !frame.command.is_empty() {
                            if frame.command == expected_cmd
                                || frame.command.starts_with(expected_cmd)
                                || (expected_cmd == "MD0" && frame.command.starts_with("MD"))
                                || (expected_cmd == "SM0" && frame.command.starts_with("SM"))
                            {
                                matched_frame = Some(frame.clone());
                            }
                            frames_to_apply.push(frame);
                        }
                    }
                    Ok(None) => {
                        let remaining = timeout_duration.saturating_sub(start.elapsed());
                        let read_fut = transport.read_chunk();
                        match tokio::time::timeout(remaining, read_fut).await {
                            Ok(Ok(_)) => {}
                            Ok(Err(e)) => {
                                return Err(e.to_rig_error(CatBackendKind::Yaesu, &endpoint));
                            }
                            Err(_) => {
                                return Err(RigError::Timeout {
                                    endpoint,
                                    operation: "read_chunk",
                                });
                            }
                        }
                    }
                    Err(e) => {
                        return Err(RigError::Protocol {
                            detail: format!("Yaesu codec error during roundtrip: {e}"),
                        });
                    }
                }
            }

            for frame in frames_to_apply {
                self.handle_incoming_frame(&frame);
            }

            if let Some(raw_err) = has_error {
                return Err(RigError::Protocol {
                    detail: format!("Yaesu radio rejected command '{cmd}' with error '{raw_err}'"),
                });
            }

            if let Some(frame) = matched_frame {
                return Ok(frame);
            }
        }
    }
}

impl RigBackend for YaesuBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::Yaesu
    }

    fn capabilities(&self) -> RigCapabilities {
        self.profile.capabilities
    }

    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        Box::pin(async move {
            let endpoint = self.endpoint_name();

            // 1. Inicjalizacja transportu
            self.ensure_transport()?;

            // 2. Bezpieczny odczyt probe (wyłącznie odczyt częstotliwości FA;, pod żadnym pozorem nie dotykamy PTT)
            if let Some(t) = self.transport.as_mut() {
                t.clear_buffer();
            }

            let probe_frame = match self
                .roundtrip_command(YaesuCat::read_freq(false), "FA")
                .await
            {
                Ok(frame) => frame,
                Err(e) => {
                    // Nieudane połączenie: wycofanie transportu i zagwarantowanie, że AI nie pozostanie aktywne
                    self.ai_active = false;
                    self.state.connected = false;
                    if let Some(mut transport) = self.transport.take() {
                        transport.close().await;
                    }
                    return Err(e);
                }
            };

            if let Some((_, hz)) = YaesuCat::parse_freq_resp(&probe_frame.raw) {
                self.state.frequency_hz = hz;
            }

            // Sesja została potwierdzona odpowiedzią transceivera
            self.state.connected = true;

            // 3. Inicjalizacja modyfikująca konfigurację radia (state-changing):
            // Włączenie trybu Auto Information (AI1;) wyłącznie po potwierdzeniu sesji z radiem.
            if self.profile.extended_capabilities.transceive_broadcast {
                let ai_cmd = YaesuCat::set_ai(true);
                let encoded = self.codec.encode_command(ai_cmd);
                let write_res = if let Some(transport) = self.transport.as_mut() {
                    transport.write_all(&encoded).await
                } else {
                    Err(crate::cat::native::transport::TransportError::Closed)
                };

                if let Err(e) = write_res {
                    self.ai_active = false;
                    self.state.connected = false;
                    if let Some(mut transport) = self.transport.take() {
                        transport.close().await;
                    }
                    return Err(e.to_rig_error(CatBackendKind::Yaesu, &endpoint));
                }
                self.ai_active = true;
            }

            Ok(())
        })
    }

    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        Box::pin(async move {
            if self.transport.is_none() {
                return Err(RigError::NotConnected);
            }

            // 1. Zawsze zbierz oczekujące ramki z łącza
            self.drain_incoming().await?;

            // 2. Jeśli AI jest aktywne i mamy już częstotliwość, odpytujemy jedynie S-meter
            if self.ai_active && self.state.frequency_hz > 0 {
                if let Ok(frame) = self.roundtrip_command(YaesuCat::read_smeter(), "SM0").await
                    && let Some(raw) = YaesuCat::parse_smeter_resp(&frame.raw)
                {
                    self.raw_smeter = Some(raw);
                }
                return Ok(self.state.clone());
            }

            // 3. Standardowy pełny polling: próba IF; (jeśli wspierana) lub odrębne odpytania
            let if_cmd = YaesuCat::read_if();
            if let Ok(frame) = self.roundtrip_command(if_cmd, "IF").await {
                if let Some(info) = YaesuIfInfo::parse_for_model(self.profile.model_id, &frame.raw)
                {
                    self.state.frequency_hz = info.frequency_hz;
                    self.state.mode = info.mode.to_string();
                    self.state.ptt = info.tx;
                    self.confirmed_ptt = info.tx;
                    self.state.split_enabled = info.split;
                    self.state.rit_hz = info.rit_offset_hz;
                }
            } else {
                // Odrębne bezpieczne odpytania
                if let Ok(f) = self
                    .roundtrip_command(YaesuCat::read_freq(false), "FA")
                    .await
                    && let Some((_, hz)) = YaesuCat::parse_freq_resp(&f.raw)
                {
                    self.state.frequency_hz = hz;
                }

                if let Ok(f) = self.roundtrip_command(YaesuCat::read_mode(), "MD0").await
                    && let Some(mode) =
                        YaesuCat::parse_mode_resp_for_model(self.profile.model_id, &f.raw)
                {
                    self.state.mode = mode.to_string();
                }

                if let Ok(f) = self.roundtrip_command(YaesuCat::read_ptt(), "TX").await
                    && let Some(ptt) = YaesuCat::parse_ptt_resp(&f.raw)
                {
                    self.state.ptt = ptt;
                    self.confirmed_ptt = ptt;
                }
            }

            // Odczyt S-metra
            if let Ok(f) = self.roundtrip_command(YaesuCat::read_smeter(), "SM0").await
                && let Some(raw) = YaesuCat::parse_smeter_resp(&f.raw)
            {
                self.raw_smeter = Some(raw);
            }

            Ok(self.state.clone())
        })
    }

    fn execute<'a>(&'a mut self, command: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        Box::pin(async move {
            let endpoint = self.endpoint_name();
            let Some(transport) = self.transport.as_mut() else {
                return Err(RigError::NotConnected);
            };

            match command {
                RigCommand::SetFrequency(hz) => {
                    let cmd =
                        YaesuCat::format_freq_checked(false, *hz, self.profile.frequency_digits())
                            .ok_or_else(|| RigError::Protocol {
                                detail: format!(
                                    "frequency {hz} Hz out of bounds for profile {}",
                                    self.profile.model_id
                                ),
                            })?;
                    let encoded = self.codec.encode_command(&cmd);
                    transport
                        .write_all(&encoded)
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                    self.state.frequency_hz = *hz;
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetMode { mode, .. } => {
                    let Some(cmd) = YaesuCat::set_mode_for_model(self.profile.model_id, mode)
                    else {
                        return Err(RigError::Unsupported {
                            backend: CatBackendKind::Yaesu,
                            command: "SetMode",
                        });
                    };
                    let encoded = self.codec.encode_command(&cmd);
                    transport
                        .write_all(&encoded)
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                    self.state.mode.clone_from(mode);
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetPtt(true) => {
                    // RYGORYSTYCZNE BEZPIECZEŃSTWO PTT:
                    // Jednokrotna próba przejścia na nadawanie. W razie błędu/timeoutu NIGDY nie ponawiaj.
                    let cmd = YaesuCat::set_ptt(true);
                    let encoded = self.codec.encode_command(cmd);

                    if let Err(e) = transport.write_all(&encoded).await {
                        return Err(e.to_rig_error(CatBackendKind::Yaesu, &endpoint));
                    }

                    // Odbierz potwierdzenie z radia
                    self.confirmed_ptt = true;
                    self.state.ptt = true;
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetPtt(false) => {
                    let cmd = YaesuCat::set_ptt(false);
                    let encoded = self.codec.encode_command(cmd);

                    if let Err(e) = transport.write_all(&encoded).await {
                        return Err(e.to_rig_error(CatBackendKind::Yaesu, &endpoint));
                    }

                    self.confirmed_ptt = false;
                    self.state.ptt = false;
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetSplit { enabled, .. } => {
                    let cmd = YaesuCat::set_split(*enabled);
                    let encoded = self.codec.encode_command(cmd);
                    transport
                        .write_all(&encoded)
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                    self.state.split_enabled = *enabled;
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetSplitFrequency(hz) => {
                    let cmd =
                        YaesuCat::format_freq_checked(true, *hz, self.profile.frequency_digits())
                            .ok_or_else(|| RigError::Protocol {
                            detail: format!(
                                "split frequency {hz} Hz out of bounds for profile {}",
                                self.profile.model_id
                            ),
                        })?;
                    let encoded = self.codec.encode_command(&cmd);
                    transport
                        .write_all(&encoded)
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                    self.state.tx_frequency_hz = Some(*hz);
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetPassband(_) => Err(RigError::Unsupported {
                    backend: CatBackendKind::Yaesu,
                    command: "SetPassband",
                }),
                RigCommand::SetVfo(vfo) => {
                    let is_b = vfo.to_ascii_uppercase().contains('B');
                    let cmd = if is_b { "VS1;" } else { "VS0;" };
                    let encoded = self.codec.encode_command(cmd);
                    transport
                        .write_all(&encoded)
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                    self.state.vfo.clone_from(vfo);
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetRit(hz) => {
                    let cmd = if *hz == 0 { "RT0;" } else { "RT1;" };
                    let encoded = self.codec.encode_command(cmd);
                    transport
                        .write_all(&encoded)
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                    self.state.rit_hz = *hz;
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetXit(hz) => {
                    let cmd = if *hz == 0 { "XT0;" } else { "XT1;" };
                    let encoded = self.codec.encode_command(cmd);
                    transport
                        .write_all(&encoded)
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Yaesu, &endpoint))?;
                    self.state.xit_hz = *hz;
                    let _ = self.drain_incoming().await;
                    Ok(())
                }
                RigCommand::SetPower(_) => Err(RigError::Unsupported {
                    backend: CatBackendKind::Yaesu,
                    command: "SetPower",
                }),
            }
        })
    }

    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            // Idempotentne, bezpieczne rozłączenie sesji:
            let has_active_session = self.transport.is_some() && self.state.connected;

            if has_active_session {
                // 1. Jeśli PTT było potwierdzone i profil potwierdza właściwą komendę RX,
                //    podejmij pojedynczą, ograniczoną czasowo (500 ms) próbę przejścia do RX (TX0;).
                //    Brak retry, błąd I/O nie blokuje zamknięcia transportu.
                if self.confirmed_ptt && self.profile.capabilities.ptt {
                    if let Some(transport) = self.transport.as_mut() {
                        let rx_cmd = YaesuCat::set_ptt(false);
                        let encoded = self.codec.encode_command(rx_cmd);
                        let write_fut = transport.write_all(&encoded);
                        let _ = tokio::time::timeout(Duration::from_millis(500), write_fut).await;
                    }
                }

                // 2. Jeśli AI było aktywne, wyślij pojedynczą, ograniczoną czasowo (500 ms) komendę wyłączenia AI (AI0;),
                //    aby radio nie zalewało portu po zamknięciu sesji.
                if self.ai_active {
                    if let Some(transport) = self.transport.as_mut() {
                        let ai_cmd = YaesuCat::set_ai(false);
                        let encoded = self.codec.encode_command(ai_cmd);
                        let write_fut = transport.write_all(&encoded);
                        let _ = tokio::time::timeout(Duration::from_millis(500), write_fut).await;
                    }
                }
            }

            self.confirmed_ptt = false;
            self.state.ptt = false;
            self.ai_active = false;

            // 3. Zawsze zamknij transport
            if let Some(mut transport) = self.transport.take() {
                transport.close().await;
            }

            self.state.connected = false;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::native::registry::YAESU_FT891;

    #[tokio::test]
    async fn test_yaesu_backend_capabilities_match_profile() {
        let (client_io, _server_io) = tokio::io::duplex(1024);
        let backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());
        assert_eq!(backend.kind(), CatBackendKind::Yaesu);
        assert_eq!(backend.capabilities(), YAESU_FT891.capabilities);
    }

    #[tokio::test]
    async fn test_yaesu_backend_no_ptt_during_probe() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        // Serwer odpowiada na FA; i przyjmuje AI1;
        let server_task = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 128];
            let n = server_io.read(&mut buf).await.unwrap();
            let req = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(req, "FA;");
            // Upewnij się, że probe NIE wysyła żadnego polecenia PTT (TX1)
            assert!(!req.contains("TX1"));

            server_io.write_all(b"FA014074000;").await.unwrap();

            // Odczyt AI1;
            let n2 = server_io.read(&mut buf).await.unwrap();
            let req2 = std::str::from_utf8(&buf[..n2]).unwrap();
            assert_eq!(req2, "AI1;");
        });

        backend.connect().await.expect("connect should succeed");
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);
        assert!(!backend.current_state().ptt);
        assert!(backend.is_ai_active());

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_yaesu_backend_poll_reads_state() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 128];

            // 1. Connect: probe FA;
            let n = server_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();

            // Connect: AI1;
            let n = server_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"AI1;");

            // 2. Poll: Ponieważ AI jest aktywne, odpytywany jest jedynie SM0;
            let n = server_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"SM0;");
            server_io.write_all(b"SM00120;").await.unwrap();
        });

        backend.connect().await.unwrap();
        let state = backend.poll().await.expect("poll should succeed");
        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(backend.raw_smeter(), Some(120));

        server_task.await.unwrap();
    }

    async fn read_frame<R: tokio::io::AsyncReadExt + Unpin>(reader: &mut R) -> String {
        let mut bytes = Vec::new();
        while !bytes.ends_with(b";") {
            let mut b = [0u8; 1];
            if reader.read_exact(&mut b).await.is_err() {
                break;
            }
            bytes.push(b[0]);
        }
        String::from_utf8(bytes).unwrap()
    }

    #[tokio::test]
    async fn test_yaesu_backend_execute_set_frequency_and_mode() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;

            // Connect
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // Execute SetFrequency
            assert_eq!(read_frame(&mut server_io).await, "FA021074000;");

            // Execute SetMode
            assert_eq!(read_frame(&mut server_io).await, "MD0C;"); // DATA-U / FT8
        });

        backend.connect().await.unwrap();

        backend
            .execute(&RigCommand::SetFrequency(21_074_000))
            .await
            .unwrap();
        assert_eq!(backend.current_state().frequency_hz, 21_074_000);

        backend
            .execute(&RigCommand::SetMode {
                mode: "DATA-U".to_string(),
                passband_hz: 3000,
            })
            .await
            .unwrap();
        assert_eq!(backend.current_state().mode, "DATA-U");

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_yaesu_backend_ptt_safety_never_retried() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;

            // Connect
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // Serwer zrywa połączenie przed komendą PTT
            drop(server_io);
        });

        backend.connect().await.unwrap();
        server_task.await.unwrap();

        // Kolejne wywołanie SetPtt po zerwaniu połączenia natychmiast raportuje błąd i NIE ponawia
        let res = backend.execute(&RigCommand::SetPtt(true)).await;
        assert!(res.is_err(), "powinno zwrócić błąd po zerwaniu kanału");
        assert!(!backend.current_state().ptt);
    }

    #[tokio::test]
    async fn test_yaesu_backend_disconnect_with_confirmed_ptt_sends_rx() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;

            // Connect
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // SetPtt(true)
            assert_eq!(read_frame(&mut server_io).await, "TX1;");

            // Podczas disconnect, ponieważ PTT było confirmed true, backend wysyła TX0;
            assert_eq!(read_frame(&mut server_io).await, "TX0;");
            // Ponieważ AI było aktywne, backend wysyła również AI0;
            assert_eq!(read_frame(&mut server_io).await, "AI0;");
        });

        backend.connect().await.unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();

        backend.disconnect().await;
        assert!(!backend.current_state().connected);

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_probe_is_strictly_read_only() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let cmd1 = read_frame(&mut server_io).await;
            // Probe wysyła wyłącznie odczyt częstotliwości (FA;)
            assert_eq!(cmd1, "FA;");
            assert!(!cmd1.contains("TX"));
            assert!(!cmd1.contains("AI"));
            server_io.write_all(b"FA014074000;").await.unwrap();

            // Upewnij się, że probe nie wysyła żadnych komend modyfikujących
            let mut buf = [0u8; 32];
            let res =
                tokio::time::timeout(Duration::from_millis(100), server_io.read(&mut buf)).await;
            assert!(matches!(res, Ok(Ok(0)) | Err(_)));
        });

        let state = backend.probe().await.expect("probe should succeed");
        assert_eq!(state.frequency_hz, 14_074_000);
        assert!(!backend.is_ai_active(), "probe must not activate AI");
        assert!(!state.ptt, "probe must not set PTT");
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_connect_never_sends_tx() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let req1 = read_frame(&mut server_io).await;
            assert!(!req1.starts_with("TX"), "connect probe must not send TX");
            server_io.write_all(b"FA014074000;").await.unwrap();

            let req2 = read_frame(&mut server_io).await;
            assert!(!req2.starts_with("TX"), "connect init must not send TX");
        });

        backend.connect().await.unwrap();
        assert!(!backend.current_state().ptt);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_connect_enables_ai_only_after_confirmation() {
        // Scenariusz A: brak potwierdzenia radia (błąd probe) nie aktywuje AI
        {
            let (client_io, mut server_io) = tokio::io::duplex(1024);
            let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

            let server_task = tokio::spawn(async move {
                assert_eq!(read_frame(&mut server_io).await, "FA;");
                tokio::io::AsyncWriteExt::write_all(&mut server_io, b"?;")
                    .await
                    .unwrap();
            });

            let res = backend.connect().await;
            assert!(res.is_err());
            assert!(!backend.is_ai_active());
            assert!(!backend.current_state().connected);
            server_task.await.unwrap();
        }

        // Scenariusz B: sesja potwierdzona -> włączenie AI1; dopiero po FA;
        {
            let (client_io, mut server_io) = tokio::io::duplex(1024);
            let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

            let server_task = tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                assert_eq!(read_frame(&mut server_io).await, "FA;");
                server_io.write_all(b"FA014074000;").await.unwrap();
                assert_eq!(read_frame(&mut server_io).await, "AI1;");
            });

            backend.connect().await.unwrap();
            assert!(backend.is_ai_active());
            assert!(backend.current_state().connected);
            server_task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn yaesu_disconnect_skips_ai0_when_ai_inactive() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut custom_profile = YAESU_FT891.clone();
        custom_profile.extended_capabilities.transceive_broadcast = false;
        let mut backend = YaesuBackend::with_stream(client_io, custom_profile);

        let server_task = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();

            // Disconnect: AI nie było aktywne, więc AI0; nie może zostać wysłane
            let mut buf = [0u8; 32];
            let res =
                tokio::time::timeout(Duration::from_millis(100), server_io.read(&mut buf)).await;
            assert!(matches!(res, Ok(Ok(0)) | Err(_)));
        });

        backend.connect().await.unwrap();
        assert!(!backend.is_ai_active());

        backend.disconnect().await;
        assert!(!backend.current_state().connected);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_disconnect_sends_ai0_once_when_ai_active() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // Rozłączenie: wysyła AI0; dokładnie raz
            assert_eq!(read_frame(&mut server_io).await, "AI0;");

            // Drugi disconnect: brak kolejnych ramek
            let mut buf = [0u8; 32];
            let res =
                tokio::time::timeout(Duration::from_millis(100), server_io.read(&mut buf)).await;
            assert!(matches!(res, Ok(Ok(0)) | Err(_)));
        });

        backend.connect().await.unwrap();
        assert!(backend.is_ai_active());

        backend.disconnect().await;
        assert!(!backend.is_ai_active());
        assert!(!backend.current_state().connected);

        // Idempotentność: drugie wywołanie disconnect
        backend.disconnect().await;

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_disconnect_skips_rx_when_ptt_not_confirmed() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // Disconnect: PTT nie było aktywne, więc jedyna komenda to AI0; (brak TX0;)
            let frame = read_frame(&mut server_io).await;
            assert_eq!(frame, "AI0;");
            assert!(!frame.contains("TX"));
        });

        backend.connect().await.unwrap();
        assert!(!backend.current_state().ptt);

        backend.disconnect().await;
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_disconnect_sends_rx_once_when_ptt_confirmed() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            assert_eq!(read_frame(&mut server_io).await, "TX1;");

            // Rozłączenie przy potwierdzonym PTT: najpierw TX0;, potem AI0;
            assert_eq!(read_frame(&mut server_io).await, "TX0;");
            assert_eq!(read_frame(&mut server_io).await, "AI0;");
        });

        backend.connect().await.unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();
        assert!(backend.current_state().ptt);

        backend.disconnect().await;
        assert!(!backend.current_state().ptt);
        assert!(!backend.current_state().connected);

        // Drugie wywołanie disconnect nie ponawia TX0;
        backend.disconnect().await;

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_disconnect_cleanup_is_bounded() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");
            assert_eq!(read_frame(&mut server_io).await, "TX1;");
            // Symulacja zablokowania odpowiedzi ze strony transceivera
            tokio::time::sleep(Duration::from_millis(600)).await;
        });

        backend.connect().await.unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();

        let t0 = std::time::Instant::now();
        backend.disconnect().await;
        // Pojedyncza ograniczona czasowo próba powrotu do RX i wyłączenia AI musi skończyć się w zadanym oknie
        assert!(t0.elapsed() < Duration::from_millis(1500));
        assert!(!backend.current_state().connected);

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn yaesu_ptt_true_is_never_retried() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // Serwer celowo zrywa połączenie
            drop(server_io);
        });

        backend.connect().await.unwrap();
        server_task.await.unwrap();

        // Jednokrotna próba nadawania: błąd nie powoduje zapętlenia retry
        let res = backend.execute(&RigCommand::SetPtt(true)).await;
        assert!(res.is_err());
        assert!(!backend.current_state().ptt);
    }

    #[tokio::test]
    async fn yaesu_disconnect_rx_attempt_is_single_and_bounded() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // SetPtt(true)
            assert_eq!(read_frame(&mut server_io).await, "TX1;");

            // Disconnect próbuje jednorazowo wysłać TX0;
            assert_eq!(read_frame(&mut server_io).await, "TX0;");
            // oraz AI0;
            assert_eq!(read_frame(&mut server_io).await, "AI0;");
        });

        backend.connect().await.unwrap();
        backend.execute(&RigCommand::SetPtt(true)).await.unwrap();

        let t0 = std::time::Instant::now();
        backend.disconnect().await;
        // Bounded: disconnect nie może wisieć dłużej niż limit fail-safe
        assert!(t0.elapsed() < Duration::from_millis(1500));
        assert!(!backend.current_state().connected);

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_yaesu_backend_handles_error_frame() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 128];
            let _ = server_io.read(&mut buf).await.unwrap();
            // Odpowiedz ramką błędu ?;
            server_io.write_all(b"?;").await.unwrap();
        });

        let res = backend.connect().await;
        assert!(res.is_err());
        match res.unwrap_err() {
            RigError::Protocol { detail } => {
                assert!(detail.contains("error '?;'"));
            }
            other => panic!("nieoczekiwany typ błędu: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_yaesu_backend_timeout_handling() {
        let (client_io, _server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        // Brak odpowiedzi symuluje timeout
        let res = backend.connect().await;
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), RigError::Timeout { .. }));
    }

    #[tokio::test]
    async fn test_yaesu_backend_ai_updates_reduce_polling() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;

            // Connect
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // Symuluj asynchroniczne zdarzenie AI rozgłoszone przez radio (np. obrót gałką na 14.080.000 Hz i zmianę trybu na CW)
            server_io.write_all(b"FA014080000;MD03;").await.unwrap();

            // Poll: backend pyta tylko o SM0;
            assert_eq!(read_frame(&mut server_io).await, "SM0;");
            server_io.write_all(b"SM00060;").await.unwrap();
        });

        backend.connect().await.unwrap();
        let state = backend.poll().await.expect("poll should succeed");

        assert_eq!(state.frequency_hz, 14_080_000);
        assert_eq!(state.mode, "CW-U");
        assert_eq!(backend.raw_smeter(), Some(60));

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_yaesu_backend_interleaved_ai_and_response() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;

            // Connect
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            // Odbierz SM0; ale wyślij przed nim powiadomienie AI (np. TX1;), a dopiero potem odpowiedź SM0
            assert_eq!(read_frame(&mut server_io).await, "SM0;");
            server_io.write_all(b"TX1;SM00120;").await.unwrap();
        });

        backend.connect().await.unwrap();
        let state = backend.poll().await.expect("poll should succeed");

        assert!(state.ptt);
        assert_eq!(backend.raw_smeter(), Some(120));

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_yaesu_backend_partial_reads_over_stream() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;

            assert_eq!(read_frame(&mut server_io).await, "FA;");
            // Wyślij odpowiedź kawałkami po 2-3 bajty
            for chunk in b"FA014074000;".chunks(3) {
                server_io.write_all(chunk).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(read_frame(&mut server_io).await, "AI1;");
        });

        backend
            .connect()
            .await
            .expect("connect with fragmented packets");
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_yaesu_backend_execute_unsupported_commands() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");
        });

        backend.connect().await.unwrap();

        let err_power = backend.execute(&RigCommand::SetPower(100.0)).await;
        assert!(matches!(
            err_power,
            Err(RigError::Unsupported {
                command: "SetPower",
                ..
            })
        ));

        let err_passband = backend.execute(&RigCommand::SetPassband(2400)).await;
        assert!(matches!(
            err_passband,
            Err(RigError::Unsupported {
                command: "SetPassband",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_yaesu_backend_execute_split_freq_and_vfo() {
        let (client_io, mut server_io) = tokio::io::duplex(1024);
        let mut backend = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());

        let server_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;

            assert_eq!(read_frame(&mut server_io).await, "FA;");
            server_io.write_all(b"FA014074000;").await.unwrap();
            assert_eq!(read_frame(&mut server_io).await, "AI1;");

            assert_eq!(read_frame(&mut server_io).await, "FT1;");
            assert_eq!(read_frame(&mut server_io).await, "FB014075000;");
            assert_eq!(read_frame(&mut server_io).await, "VS1;");
            assert_eq!(read_frame(&mut server_io).await, "RT1;");
            assert_eq!(read_frame(&mut server_io).await, "XT1;");
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

        backend
            .execute(&RigCommand::SetSplitFrequency(14_075_000))
            .await
            .unwrap();
        assert_eq!(backend.current_state().tx_frequency_hz, Some(14_075_000));

        backend
            .execute(&RigCommand::SetVfo("VFOB".to_string()))
            .await
            .unwrap();
        assert_eq!(backend.current_state().vfo, "VFOB");

        backend.execute(&RigCommand::SetRit(150)).await.unwrap();
        assert_eq!(backend.current_state().rit_hz, 150);

        backend.execute(&RigCommand::SetXit(200)).await.unwrap();
        assert_eq!(backend.current_state().xit_hz, 200);

        server_task.await.unwrap();
    }
}
