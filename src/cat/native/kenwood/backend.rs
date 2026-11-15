// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny backend Kenwood PC Control sterowania transceiverem.
//!
//! Zapewnia bezpośrednią, asynchroniczną komunikację ze sprzętem
//! przez porty szeregowe (RS-232C, USB Virtual COM) oraz interfejs sieciowy KNS LAN (TCP port 60000):
//! - rygorystyczne bezpieczeństwo PTT (brak PTT podczas connect/probe, brak ponawiania SetPtt(true)),
//! - pełna obsługa trybu Auto Information (`AI2;`) z redukcją zbędnego pollingu,
//! - dekodowanie ramek transceivera (`FA`, `FB`, `MD`, `TX`, `SM`, `FT`, `IF`),
//! - pełna zgodność z kontraktem [`RigBackend`] oraz typowanymi błędami [`RigError`].

use std::time::Duration;

use crate::cat::backend::{BoxFuture, CatBackendKind, RigBackend, RigProbeResult};
use crate::cat::native::kenwood::codec::{KenwoodCat, KenwoodCodec, KenwoodFrame, KenwoodIfInfo};
use crate::cat::native::kenwood::lan::{AsyncReaderWriter, KenwoodLanConfig, KenwoodLanTransport};
use crate::cat::native::kenwood::models::KenwoodPttContract;
use crate::cat::native::profile::RigModelProfile;
use crate::cat::native::transport::{
    AsyncFramedTransport, SerialConfig, TransportError, open_serial_stream,
};
use crate::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};

/// Twardy limit czasu jednej komendy PTT z udokumentowaną weryfikacją `IF;`.
const PTT_STATUS_TIMEOUT: Duration = Duration::from_millis(1500);

/// Opóźnienie po wysłaniu komendy PTT, po którym wysyłany jest udokumentowany odczyt statusu `IF;`.
const PTT_STATUS_QUERY_DELAY: Duration = Duration::from_millis(150);

/// Okno oczekiwania na udokumentowaną odpowiedź Auto Information (`TX P1;` / `RX;`) na modelach,
/// których dokument **nie** definiuje odczytu statusu `IF;`. Po jego upływie komenda jest uznana
/// za wysłaną, ale stan pozostaje **niepotwierdzony**.
const PTT_AI_ANSWER_WINDOW: Duration = Duration::from_millis(250);

/// Backend natywnego protokołu Kenwood PC Control.
pub struct KenwoodBackend {
    profile: RigModelProfile,
    serial_config: Option<SerialConfig>,
    lan_config: Option<KenwoodLanConfig>,
    transport: Option<AsyncFramedTransport<Box<dyn AsyncReaderWriter>>>,
    mock_stream: Option<Box<dyn AsyncReaderWriter>>,
    state: RigState,
    codec: KenwoodCodec,
    ai_active: bool,
    /// Stan PTT potwierdzony udokumentowaną odpowiedzią transceivera.
    confirmed_ptt: bool,
    /// Wysłano udokumentowaną komendę `TX;`, ale stan nie został potwierdzony.
    ///
    /// Służy wyłącznie jako obowiązek bezpiecznego `RX;` przy rozłączeniu. **Nie** jest
    /// potwierdzeniem fizycznego nadawania i nigdy nie jest raportowany jako `ConfirmedTx`.
    ptt_commanded: bool,
}

impl KenwoodBackend {
    /// Tworzy nowy backend dla połączenia szeregowego / USB Virtual COM.
    pub fn new_serial(config: SerialConfig, profile: RigModelProfile) -> Self {
        Self {
            profile,
            serial_config: Some(config),
            lan_config: None,
            transport: None,
            mock_stream: None,
            state: RigState::default(),
            codec: KenwoodCodec::new(),
            ai_active: false,
            confirmed_ptt: false,
            ptt_commanded: false,
        }
    }

    /// Tworzy nowy backend dla połączenia sieciowego KNS LAN (TS-890S, TS-990S).
    pub fn new_lan(lan_config: KenwoodLanConfig, profile: RigModelProfile) -> Self {
        Self {
            profile,
            serial_config: None,
            lan_config: Some(lan_config),
            transport: None,
            mock_stream: None,
            state: RigState::default(),
            codec: KenwoodCodec::new(),
            ai_active: false,
            confirmed_ptt: false,
            ptt_commanded: false,
        }
    }

    /// Tworzy backend z bezpośrednio wstrzykniętym strumieniem I/O (dla testów szeregowych).
    pub fn with_stream<S: AsyncReaderWriter + 'static>(
        stream: S,
        profile: RigModelProfile,
    ) -> Self {
        Self {
            profile,
            serial_config: None,
            lan_config: None,
            transport: None,
            mock_stream: Some(Box::new(stream)),
            state: RigState::default(),
            codec: KenwoodCodec::new(),
            ai_active: false,
            confirmed_ptt: false,
            ptt_commanded: false,
        }
    }

    /// Tworzy backend z wstrzykniętym strumieniem I/O i konfiguracją LAN (dla testów sieciowych).
    pub fn with_lan_stream<S: AsyncReaderWriter + 'static>(
        stream: S,
        lan_config: KenwoodLanConfig,
        profile: RigModelProfile,
    ) -> Self {
        Self {
            profile,
            serial_config: None,
            lan_config: Some(lan_config),
            transport: None,
            mock_stream: Some(Box::new(stream)),
            state: RigState::default(),
            codec: KenwoodCodec::new(),
            ai_active: false,
            confirmed_ptt: false,
            ptt_commanded: false,
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

    /// Zwraca flagę aktywności trybu Auto Information.
    pub fn is_ai_active(&self) -> bool {
        self.ai_active
    }

    /// Zwraca bieżący endpoint do komunikatów diagnostycznych.
    fn endpoint_name(&self) -> String {
        if let Some(ref serial) = self.serial_config {
            serial.port_name.clone()
        } else if let Some(ref lan) = self.lan_config {
            format!("{}:{}", lan.host, lan.port)
        } else {
            "MOCK_STREAM".to_string()
        }
    }

    /// Zwraca udokumentowany kontrakt PTT modelu tego profilu.
    ///
    /// Model spoza tabeli udokumentowanych kontraktów nie ma oficjalnego dokumentu → błąd
    /// `Unsupported` (fail-closed), bez wysyłania jakiejkolwiek komendy PTT.
    fn ptt_contract(&self) -> Result<&'static KenwoodPttContract, RigError> {
        KenwoodPttContract::for_model(self.profile.model_id).ok_or(RigError::Unsupported {
            backend: CatBackendKind::Kenwood,
            command: "SetPtt",
        })
    }

    /// Czy dokument tego modelu definiuje odczyt statusu `IF;`.
    fn documents_status_read(&self) -> bool {
        KenwoodPttContract::for_model(self.profile.model_id)
            .is_some_and(KenwoodPttContract::documents_status_read)
    }

    /// Buduje typowany błąd dla udokumentowanej ramki błędu transceivera (`?;`, `E;`, `O;`).
    fn documented_error(frame: &KenwoodFrame) -> Option<RigError> {
        frame.documented_error().map(|error| RigError::Protocol {
            detail: format!(
                "Kenwood documented error frame '{}': {}",
                error.wire(),
                error.description()
            ),
        })
    }

    /// Przetwarza zdekodowaną ramkę Kenwood i aktualizuje lokalny stan radia.
    fn handle_incoming_frame(&mut self, frame: &KenwoodFrame) {
        match frame.command.as_str() {
            "FA" => {
                if let Some((_, hz)) = KenwoodCat::parse_freq_resp(&frame.raw) {
                    self.state.frequency_hz = hz;
                }
            }
            "FB" => {
                if let Some((_, hz)) = KenwoodCat::parse_freq_resp(&frame.raw) {
                    self.state.tx_frequency_hz = Some(hz);
                }
            }
            "MD" => {
                if let Some(mode) = KenwoodCat::parse_mode_resp(&frame.raw) {
                    self.state.mode = mode.to_string();
                }
            }
            "TX" | "RX" => {
                if let Some(ptt) = KenwoodCat::parse_ptt_resp(&frame.raw) {
                    self.state.ptt = ptt;
                    self.confirmed_ptt = ptt;
                    if !ptt {
                        self.ptt_commanded = false;
                    }
                }
            }
            "SM" => {
                if let Some(raw) = KenwoodCat::parse_smeter_resp(&frame.raw) {
                    let (dbm, unit) = KenwoodCat::smeter_to_dbm_and_unit(raw);
                    self.state.s_meter_dbm = dbm;
                    self.state.s_meter_unit = unit;
                }
            }
            "FT" => {
                if let Some(split) = KenwoodCat::parse_split_resp(&frame.raw) {
                    self.state.split_enabled = split;
                }
            }
            "PC" => {
                if let Some(w) = KenwoodCat::parse_power_resp(&frame.raw) {
                    self.state.rf_power_watts = w;
                }
            }
            "IF" => {
                // `IF` jest udokumentowany wyłącznie w dokumencie TS-590S/TS-590SG; dla pozostałych
                // modeli ramka nie jest interpretowana (brak dokumentu = brak zgadywania).
                if self.documents_status_read()
                    && let Some(info) = KenwoodIfInfo::parse(&frame.raw)
                {
                    self.state.frequency_hz = info.frequency_hz;
                    self.state.mode = info.mode.to_string();
                    self.state.ptt = info.tx;
                    self.confirmed_ptt = info.tx;
                    if !info.tx {
                        self.ptt_commanded = false;
                    }
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

    /// Odbiera oczekujące w buforze lub na łączu ramki (nieblokująco lub z krótkim timeoutem).
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
                            detail: format!("Kenwood decode error: {e}"),
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
                        return Err(e.to_rig_error(CatBackendKind::Kenwood, &endpoint));
                    }
                }
            }
        } else {
            return Err(RigError::NotConnected);
        }

        for frame in frames {
            if let Some(error) = Self::documented_error(&frame) {
                return Err(error);
            }
            self.handle_incoming_frame(&frame);
        }

        Ok(())
    }

    /// Wysyła komendę i oczekuje na odpowiedź o zadanym prefiksie (np. "FA", "MD", "TX", "IF").
    async fn roundtrip_command(
        &mut self,
        cmd: &str,
        expected_cmd: &str,
    ) -> Result<KenwoodFrame, RigError> {
        let endpoint = self.endpoint_name();
        let encoded = self.codec.encode_command(cmd);

        let Some(transport) = self.transport.as_mut() else {
            return Err(RigError::NotConnected);
        };

        transport
            .write_all(&encoded)
            .await
            .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &endpoint))?;

        let start = tokio::time::Instant::now();
        let timeout_duration = Duration::from_millis(1500);
        let mut verification_sent = false;

        loop {
            if start.elapsed() >= timeout_duration {
                return Err(RigError::Timeout {
                    endpoint,
                    operation: "command_roundtrip",
                });
            }

            // Jeśli minęło 150 ms i nie ma jeszcze odpowiedzi (np. brak echa AI2 przy setterze),
            // wyślij zapytanie weryfikujące, aby nie blokować kolejki na pełne 1500 ms.
            if !verification_sent && start.elapsed() >= Duration::from_millis(150) {
                let getter = match expected_cmd {
                    "FA" => Some("FA;"),
                    "FB" => Some("FB;"),
                    "MD" => Some("MD;"),
                    "PC" => Some("PC;"),
                    "RT" => Some("RT;"),
                    "XT" => Some("XT;"),
                    "FR" => Some("FR;"),
                    "FT" => Some("FT;"),
                    _ => None,
                };
                if let Some(query) = getter
                    && let Some(transport) = self.transport.as_mut()
                {
                    let _ = transport.write_all(query.as_bytes()).await;
                }
                verification_sent = true;
            }

            let mut matched_frame = None;
            let mut frames_to_apply = Vec::new();

            {
                let Some(transport) = self.transport.as_mut() else {
                    return Err(RigError::NotConnected);
                };

                match self.codec.decode_frame(transport.buffer()) {
                    Ok(Some((frame, consumed))) => {
                        transport.consume(consumed);
                        if !frame.command.is_empty() {
                            // Udokumentowany komunikat błędu (`?;`, `E;`, `O;`) przerywa operację
                            // natychmiast — bez retry i bez fałszywego sukcesu.
                            if let Some(error) = Self::documented_error(&frame) {
                                return Err(error);
                            }
                            if frame.command == expected_cmd {
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
                                return Err(e.to_rig_error(CatBackendKind::Kenwood, &endpoint));
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
                            detail: format!("Kenwood codec error during roundtrip: {e}"),
                        });
                    }
                }
            }

            for frame in frames_to_apply {
                self.handle_incoming_frame(&frame);
            }

            if let Some(frame) = matched_frame {
                return Ok(frame);
            }
        }
    }

    /// Wysyła **udokumentowaną** komendę PTT (`TX;` / `RX;`) i próbuje potwierdzić stan wyłącznie
    /// kanałem, który dokumentuje dany model.
    ///
    /// * Modele z udokumentowanym `IF;` (TS-590S/TS-590SG): po 150 ms wysyłany jest `IF;`, a wynik
    ///   jest potwierdzony tylko wtedy, gdy `P8` odpowiada żądanemu stanowi. Brak odpowiedzi =
    ///   `Timeout` (fail-closed).
    /// * Modele bez `IF;` (TS-890S/TS-990S): jedynym udokumentowanym kanałem jest odpowiedź Auto
    ///   Information (`TX P1;` / `RX;`). Po jej otrzymaniu stan jest potwierdzony; po upływie okna
    ///   komenda jest uznana za **wysłaną**, ale stan pozostaje **niepotwierdzony** (`Ok(false)`).
    ///
    /// Zwraca `Ok(true)` tylko dla potwierdzonego stanu. Nigdy nie ponawia komendy PTT.
    async fn write_ptt_command(
        &mut self,
        command: &str,
        expected_cmd: &str,
        status_read: Option<&'static str>,
        requested: bool,
    ) -> Result<bool, RigError> {
        let endpoint = self.endpoint_name();
        let encoded = self.codec.encode_command(command);
        let status_command = status_read.map(|wire| wire.trim_end_matches(';'));

        let Some(transport) = self.transport.as_mut() else {
            return Err(RigError::NotConnected);
        };
        transport
            .write_all(&encoded)
            .await
            .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &endpoint))?;

        let window = if status_command.is_some() {
            PTT_STATUS_TIMEOUT
        } else {
            PTT_AI_ANSWER_WINDOW
        };
        let start = tokio::time::Instant::now();
        let mut status_query_sent = false;

        loop {
            // 1. Przetwórz wszystkie kompletne ramki obecne w buforze.
            let mut frames = Vec::new();
            {
                let Some(transport) = self.transport.as_mut() else {
                    return Err(RigError::NotConnected);
                };
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
                                detail: format!("Kenwood decode error during PTT: {e}"),
                            });
                        }
                    }
                }
            }

            for frame in frames {
                if let Some(error) = Self::documented_error(&frame) {
                    // Udokumentowane odrzucenie komendy — natychmiastowy błąd, bez retry.
                    self.handle_incoming_frame(&frame);
                    return Err(error);
                }
                if frame.command == expected_cmd {
                    self.handle_incoming_frame(&frame);
                    return Ok(true);
                }
                if status_command.is_some_and(|command| frame.command == command)
                    && let Some(info) = KenwoodIfInfo::parse(&frame.raw)
                {
                    if info.tx == requested {
                        self.handle_incoming_frame(&frame);
                        return Ok(true);
                    }
                    // Niezgodny odczyt statusu nie jest potwierdzeniem — kontynuuj do deadline.
                    continue;
                }
                self.handle_incoming_frame(&frame);
            }

            // 2. Po 150 ms wyślij udokumentowany odczyt statusu (tylko gdy model go definiuje).
            if !status_query_sent
                && status_command.is_some()
                && start.elapsed() >= PTT_STATUS_QUERY_DELAY
            {
                if let Some(read) = status_read {
                    let query = self.codec.encode_command(read);
                    if let Some(transport) = self.transport.as_mut() {
                        let _ = transport.write_all(&query).await;
                    }
                }
                status_query_sent = true;
            }

            // 3. Upłynął limit czasu.
            if start.elapsed() >= window {
                if status_command.is_some() {
                    return Err(RigError::Timeout {
                        endpoint,
                        operation: "ptt_status_read",
                    });
                }
                // Komenda została zapisana; stan pozostaje niepotwierdzony.
                return Ok(false);
            }

            // 4. Czekaj na kolejne bajty z łącza.
            //
            // Gdy model dokumentuje odczyt statusu i nie został on jeszcze wysłany, oczekiwanie
            // jest skrócone do chwili `PTT_STATUS_QUERY_DELAY`, aby odczyt statusu został wysłany
            // mimo braku jakiejkolwiek ramki z transceivera.
            let remaining = window.saturating_sub(start.elapsed());
            let slice = if !status_query_sent && status_command.is_some() {
                let until_query = PTT_STATUS_QUERY_DELAY.saturating_sub(start.elapsed());
                remaining.min(until_query.max(Duration::from_millis(1)))
            } else {
                remaining
            };
            let Some(transport) = self.transport.as_mut() else {
                return Err(RigError::NotConnected);
            };
            // Trzy warianty "brak nowych danych" (poprawny odczyt, limit transportu, wycinek czasu)
            // mają świadomie identyczne zachowanie: pętla sprawdza własny limit `window`.
            #[allow(clippy::match_same_arms)]
            match tokio::time::timeout(slice, transport.read_chunk()).await {
                Ok(Ok(_)) => {}
                Ok(Err(TransportError::Closed)) => return Err(RigError::NotConnected),
                Ok(Err(e)) => {
                    if !matches!(e, TransportError::Timeout { .. }) {
                        return Err(e.to_rig_error(CatBackendKind::Kenwood, &endpoint));
                    }
                }
                Err(_) => {}
            }
        }
    }

    /// Wykonuje `SetPtt` zgodnie z udokumentowanym kontraktem modelu.
    ///
    /// Kontrakt bezpieczeństwa:
    /// - komenda jest wysyłana dokładnie raz (brak retry PTT ON),
    /// - `state.ptt`/`confirmed_ptt` ustawiane są **wyłącznie** po udokumentowanym potwierdzeniu,
    /// - samo powodzenie zapisu nigdy nie jest raportowane jako potwierdzony TX,
    /// - po przyjętej komendzie `TX;` obowiązek bezpiecznego `RX;` przy rozłączeniu pozostaje
    ///   (`ptt_commanded`), nawet gdy stan jest niepotwierdzony.
    async fn execute_set_ptt(&mut self, ptt: bool) -> Result<(), RigError> {
        // Model bez oficjalnego dokumentu jest odrzucany **przed** jakimkolwiek zapisem.
        let contract = self.ptt_contract()?;
        if self.transport.is_none() {
            return Err(RigError::NotConnected);
        }
        let command = if ptt {
            contract.ptt_on
        } else {
            contract.ptt_off
        };
        let expected = if ptt { "TX" } else { "RX" };

        // Zamiar nadawania jest rejestrowany **przed** zapisem komendy: nawet przerwanie
        // (cancellation) w połowie zapisu pozostawia obowiązek bezpiecznego `RX;` przy rozłączeniu.
        if ptt {
            self.ptt_commanded = true;
        }

        let outcome = self
            .write_ptt_command(command, expected, contract.status_read, ptt)
            .await;

        match outcome {
            Ok(confirmed) => {
                if ptt {
                    // Komenda `TX;` została wysłana; bezpieczne RX; pozostaje obowiązkiem.
                    self.ptt_commanded = true;
                    if confirmed {
                        self.confirmed_ptt = true;
                        self.state.ptt = true;
                    }
                } else {
                    self.ptt_commanded = false;
                    self.confirmed_ptt = false;
                    self.state.ptt = false;
                }
                Ok(())
            }
            Err(error) => {
                if ptt {
                    // Zapis mógł częściowo dotrzeć do transceivera — utrzymaj obowiązek RX;.
                    self.ptt_commanded = true;
                } else {
                    self.ptt_commanded = false;
                    self.confirmed_ptt = false;
                    self.state.ptt = false;
                }
                Err(error)
            }
        }
    }
}

impl RigBackend for KenwoodBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::Kenwood
    }

    fn capabilities(&self) -> RigCapabilities {
        self.profile.capabilities
    }

    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        Box::pin(async move {
            if self.transport.is_some() {
                return Ok(());
            }

            let endpoint = self.endpoint_name();

            // 1. Inicjalizacja warstwy I/O
            if let Some(mock) = self.mock_stream.take() {
                if let Some(ref lan_cfg) = self.lan_config {
                    let lan = KenwoodLanTransport::from_stream(mock, lan_cfg).await?;
                    self.transport = Some(lan.into_framed());
                } else {
                    let framed = AsyncFramedTransport::from_stream(
                        mock,
                        1024,
                        Duration::from_millis(1000),
                        Duration::from_millis(1000),
                    );
                    self.transport = Some(framed);
                }
            } else if let Some(ref lan_cfg) = self.lan_config {
                let lan = KenwoodLanTransport::connect(lan_cfg).await?;
                self.transport = Some(lan.into_framed());
            } else if let Some(ref serial_cfg) = self.serial_config {
                let stream = open_serial_stream(serial_cfg)
                    .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &endpoint))?;
                let framed = AsyncFramedTransport::from_stream(
                    Box::new(stream) as Box<dyn AsyncReaderWriter>,
                    serial_cfg.max_buffer_size,
                    serial_cfg.read_timeout,
                    serial_cfg.write_timeout,
                );
                self.transport = Some(framed);
            } else {
                return Err(RigError::Connect {
                    endpoint,
                    detail: "missing transport configuration".to_string(),
                });
            }

            // 2. Rygorystyczny test obecności radia (Probe) — wyłącznie komendy bezpieczne READ-ONLY!
            // Zakaz jakiegokolwiek załączania PTT podczas connect/probe!
            // `IF;` jest udokumentowany tylko dla TS-590S/TS-590SG; fallback używa `ID;`
            // (udokumentowany dla wszystkich czterech obsługiwanych modeli).
            let probe_res = self
                .roundtrip_command(KenwoodCat::read_freq_cmd(false), "FA")
                .await;
            if probe_res.is_err() {
                let _ = self
                    .roundtrip_command(KenwoodCat::read_id_cmd(), "ID")
                    .await?;
            }

            // 3. Włączenie Auto Information (AI2) jeśli profil wspiera transceive_broadcast
            if self.profile.extended_capabilities.transceive_broadcast {
                let ai_cmd = KenwoodCat::set_ai_cmd(2);
                if self.roundtrip_command(&ai_cmd, "AI").await.is_ok() {
                    self.ai_active = true;
                }
            }

            self.confirmed_ptt = false;
            self.ptt_commanded = false;
            self.state.ptt = false;
            self.state.connected = true;
            Ok(())
        })
    }

    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        Box::pin(async move {
            if self.transport.is_none() {
                return Err(RigError::NotConnected);
            }

            // Odbierz wszelkie asynchroniczne powiadomienia AI obecne w buforze
            let _ = self.drain_incoming().await;

            let contract = KenwoodPttContract::for_model(self.profile.model_id);

            // `IF;` wolno wysłać wyłącznie modelom, których dokument go definiuje.
            if !self.ai_active
                && let Some(status_read) = contract.and_then(|contract| contract.status_read)
            {
                let status_cmd = status_read.trim_end_matches(';');
                let _ = self.roundtrip_command(status_read, status_cmd).await?;
            }

            // S-Meter jest miernikiem analogowym i nie jest rozsyłany samorzutnie przez AI.
            // Formularz parametru jest modelowy (`SM0;` dla TS-590S/SG/990S, `SM;` dla TS-890S).
            if self.profile.capabilities.s_meter
                && let Some(smeter_read) = contract.and_then(|contract| contract.smeter_read)
            {
                let _ = self.roundtrip_command(smeter_read, "SM").await;
            }

            self.state.connected = true;
            Ok(self.state.clone())
        })
    }

    fn execute<'a>(&'a mut self, command: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        Box::pin(async move {
            if self.transport.is_none() {
                return Err(RigError::NotConnected);
            }

            match command {
                RigCommand::SetFrequency(hz) => {
                    let cmd = KenwoodCat::set_freq_cmd(false, *hz);
                    self.roundtrip_command(&cmd, "FA").await?;
                    self.state.frequency_hz = *hz;
                    Ok(())
                }
                RigCommand::SetMode {
                    mode,
                    passband_hz: _,
                } => {
                    let Some(cmd) = KenwoodCat::set_mode_cmd(mode) else {
                        return Err(RigError::Protocol {
                            detail: format!("unsupported mode: {mode}"),
                        });
                    };
                    self.roundtrip_command(&cmd, "MD").await?;
                    self.state.mode.clone_from(mode);
                    Ok(())
                }
                RigCommand::SetPtt(ptt) => {
                    // BEZPIECZEŃSTWO PTT:
                    // SetPtt(true) NIE MOŻE być ponawiane w przypadku błędu/timeoutu!
                    // Komenda i kanał potwierdzenia pochodzą z udokumentowanego kontraktu modelu.
                    self.execute_set_ptt(*ptt).await
                }
                RigCommand::SetSplit { enabled, tx_vfo: _ } => {
                    let cmd = KenwoodCat::set_split_cmd(*enabled);
                    self.roundtrip_command(cmd, "FT").await?;
                    self.state.split_enabled = *enabled;
                    Ok(())
                }
                RigCommand::SetSplitFrequency(hz) => {
                    let cmd = KenwoodCat::set_freq_cmd(true, *hz);
                    self.roundtrip_command(&cmd, "FB").await?;
                    self.state.tx_frequency_hz = Some(*hz);
                    Ok(())
                }
                RigCommand::SetVfo(vfo) => {
                    let vfo_str = vfo.to_ascii_uppercase();
                    let cmd = if vfo_str.contains('B') {
                        "FR1;"
                    } else {
                        "FR0;"
                    };
                    self.roundtrip_command(cmd, "FR").await?;
                    self.state.vfo.clone_from(vfo);
                    Ok(())
                }
                RigCommand::SetPower(watts) => {
                    let cmd = KenwoodCat::set_power_cmd(*watts);
                    self.roundtrip_command(&cmd, "PC").await?;
                    self.state.rf_power_watts = *watts;
                    Ok(())
                }
                RigCommand::SetRit(hz) => {
                    let cmd = KenwoodCat::set_rit_cmd(*hz != 0);
                    self.roundtrip_command(cmd, "RT").await?;
                    self.state.rit_hz = *hz;
                    Ok(())
                }
                RigCommand::SetXit(hz) => {
                    let cmd = KenwoodCat::set_xit_cmd(*hz != 0);
                    self.roundtrip_command(cmd, "XT").await?;
                    self.state.xit_hz = *hz;
                    Ok(())
                }
                RigCommand::SetPassband(_) => {
                    // Szerokość filtru w Kenwood jest powiązana z trybem DSP (FW/FL/FH)
                    Ok(())
                }
            }
        })
    }

    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            // BEZPIECZEŃSTWO PTT:
            // Jeśli radio było w stanie nadawania **lub** przyjęło udokumentowaną komendę `TX;`
            // (stan niepotwierdzony), wykonaj jedną bezpieczną próbę przejścia na odbiór (`RX;`).
            // `RX;` jest udokumentowaną, idempotentną komendą; próba jest pojedyncza i ograniczona
            // czasowo, a jej wynik nie jest raportowany jako potwierdzenie fizycznego RX.
            if self.confirmed_ptt || self.ptt_commanded {
                if let Some(transport) = self.transport.as_mut() {
                    let rx_bytes = KenwoodCat::set_ptt_cmd(false);
                    let _ = tokio::time::timeout(
                        Duration::from_millis(200),
                        transport.write_all(rx_bytes.as_bytes()),
                    )
                    .await;
                }
                self.confirmed_ptt = false;
                self.ptt_commanded = false;
                self.state.ptt = false;
            }

            // Zamknięcie transportu KNS LAN lub portu szeregowego
            if let Some(mut transport) = self.transport.take() {
                if self.lan_config.is_some() {
                    let _ = tokio::time::timeout(
                        Duration::from_millis(200),
                        transport.write_all(b"##BY;\r\n"),
                    )
                    .await;
                }
                transport.close().await;
            }

            self.ai_active = false;
            self.state.connected = false;
        })
    }

    fn probe(&mut self) -> BoxFuture<'_, Result<RigProbeResult, RigError>> {
        Box::pin(async move {
            let was_connected = self.transport.is_some();
            if !was_connected {
                let endpoint = self.endpoint_name();
                if let Some(mock) = self.mock_stream.take() {
                    let framed = AsyncFramedTransport::from_stream(
                        mock,
                        1024,
                        Duration::from_millis(1000),
                        Duration::from_millis(1000),
                    );
                    self.transport = Some(framed);
                } else if let Some(ref lan_cfg) = self.lan_config {
                    let lan = KenwoodLanTransport::connect(lan_cfg).await?;
                    self.transport = Some(lan.into_framed());
                } else if let Some(ref serial_cfg) = self.serial_config {
                    let stream = open_serial_stream(serial_cfg)
                        .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &endpoint))?;
                    let framed = AsyncFramedTransport::from_stream(
                        Box::new(stream) as Box<dyn AsyncReaderWriter>,
                        serial_cfg.max_buffer_size,
                        serial_cfg.read_timeout,
                        serial_cfg.write_timeout,
                    );
                    self.transport = Some(framed);
                } else {
                    return Err(RigError::Connect {
                        endpoint,
                        detail: "missing transport configuration".to_string(),
                    });
                }
            }

            // Bezpieczny odczyt `FA;` lub `ID;` — bez AI2! Bez PTT!
            // `IF;` nie jest używany: jest udokumentowany wyłącznie dla TS-590S/TS-590SG,
            // a `ID;` jest udokumentowany dla wszystkich czterech obsługiwanych modeli.
            let probe_res = self
                .roundtrip_command(KenwoodCat::read_freq_cmd(false), "FA")
                .await;
            if probe_res.is_err() {
                let _ = self
                    .roundtrip_command(KenwoodCat::read_id_cmd(), "ID")
                    .await?;
            }

            let mut state = self.state.clone();
            state.connected = true;
            state.ptt = false;

            if !was_connected {
                self.disconnect().await;
            }

            Ok(RigProbeResult {
                reachable: true,
                identity: Some(format!("Kenwood ({})", self.profile.name)),
                model: Some(self.profile.name.to_string()),
                firmware: None,
                capabilities: self.capabilities(),
                state: Some(state),
                safe_diagnostic: format!(
                    "Połączenie z Kenwood ({}) powiodło się",
                    self.profile.name
                ),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    use crate::cat::native::registry::{KENWOOD_TS590S, KENWOOD_TS890S};

    #[tokio::test]
    async fn kenwood_backend_read_only_probe_no_ptt() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // 1. Probe: FA;
            let n = radio_io.read(&mut buf).await.unwrap();
            let query = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(query, "FA;");
            // Odpowiedź na probe
            radio_io.write_all(b"FA00014074000;").await.unwrap();

            // 2. Włączenie AI: AI2;
            let n = radio_io.read(&mut buf).await.unwrap();
            let query = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(query, "AI2;");
            radio_io.write_all(b"AI2;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);
        assert!(!backend.current_state().ptt);
        assert!(backend.is_ai_active());
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_poll_reads_state() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Probe FA;
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();

            // AI2;
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // Poll: odczyt SM0; (ponieważ AI jest aktywne, odpytywany jest tylko miernik)
            let n = radio_io.read(&mut buf).await.unwrap();
            let query = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(query, "SM0;");
            radio_io.write_all(b"SM00015;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        let state = backend.poll().await.expect("poll");

        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.s_meter_unit, "S9");
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_execute_set_frequency() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // SetFrequency 7.030.000 Hz
            let n = radio_io.read(&mut buf).await.unwrap();
            let query = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(query, "FA00007030000;");
            radio_io.write_all(b"FA00007030000;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        backend
            .execute(&RigCommand::SetFrequency(7_030_000))
            .await
            .expect("set freq");

        assert_eq!(backend.current_state().frequency_hz, 7_030_000);
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_execute_set_mode() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // SetMode CW -> MD3;
            let n = radio_io.read(&mut buf).await.unwrap();
            let query = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(query, "MD3;");
            radio_io.write_all(b"MD3;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        backend
            .execute(&RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            })
            .await
            .expect("set mode");

        assert_eq!(backend.current_state().mode, "CW");
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_execute_set_ptt() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // TX;
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "TX;");
            radio_io.write_all(b"TX1;").await.unwrap();

            // RX;
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "RX;");
            radio_io.write_all(b"RX;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("tx");
        assert!(backend.current_state().ptt);

        backend
            .execute(&RigCommand::SetPtt(false))
            .await
            .expect("rx");
        assert!(!backend.current_state().ptt);
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_timeout_handling() {
        let (client_io, _radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        // Brak odpowiedzi z radia symuluje timeout
        let res = backend.connect().await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn kenwood_backend_disconnect_idempotent() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        backend.disconnect().await;
        backend.disconnect().await; // Idempotentne
    }

    #[tokio::test]
    async fn kenwood_backend_no_ptt_during_probe() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(10);
        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let n = radio_io.read(&mut buf).await.unwrap();
            tx.send(buf[..n].to_vec()).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();

            let n = radio_io.read(&mut buf).await.unwrap();
            tx.send(buf[..n].to_vec()).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        let chunk1 = rx.recv().await.unwrap();
        let chunk2 = rx.recv().await.unwrap();
        let s1 = std::str::from_utf8(&chunk1).unwrap();
        let s2 = std::str::from_utf8(&chunk2).unwrap();
        assert!(!s1.contains("TX"));
        assert!(!s2.contains("TX"));
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_ptt_true_not_retried_on_error() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // Radio odbiera TX;, ale nie odpowiada (lub odpowiada błędem)
            let _ = radio_io.read(&mut buf).await.unwrap();
            // Brak odpowiedzi
        });

        backend.connect().await.expect("connect");
        let res = backend.execute(&RigCommand::SetPtt(true)).await;
        // Musi zgłosić natychmiastowy błąd, brak retry
        assert!(res.is_err());
        assert!(!backend.current_state().ptt);
    }

    #[tokio::test]
    async fn kenwood_backend_ai_updates_reduce_polling() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // Symulacja samorzutnego rozgłoszenia zmiany gałką częstotliwości z radia (AI broadcast)
            radio_io.write_all(b"FA00021074000;").await.unwrap();

            // Poll wysyła tylko zapytanie o miernik
            let n = radio_io.read(&mut buf).await.unwrap();
            let query = std::str::from_utf8(&buf[..n]).unwrap();
            assert_eq!(query, "SM0;");
            radio_io.write_all(b"SM00015;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        let state = backend.poll().await.expect("poll");

        assert_eq!(state.frequency_hz, 21_074_000);
        assert_eq!(state.s_meter_unit, "S9");
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_interleaved_ai_and_response() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // Przychodzi rozkaz SetMode CW (MD3;)
            let _ = radio_io.read(&mut buf).await.unwrap();
            // Radio najpierw przesyła spontaniczny pakiet AI (np. zmianę S-metra), a zaraz potem odpowiedź MD3;
            radio_io.write_all(b"SM00015;MD3;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        backend
            .execute(&RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            })
            .await
            .expect("set mode");

        assert_eq!(backend.current_state().mode, "CW");
        assert_eq!(backend.current_state().s_meter_unit, "S9");
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_partial_reads_over_stream() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            // Wysyłaj odpowiedź FA po 1 bajcie
            for b in b"FA00014074000;" {
                radio_io.write_all(&[*b]).await.unwrap();
            }
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();
        });

        backend.connect().await.expect("connect");
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);
        backend.disconnect().await;
    }

    #[tokio::test]
    async fn kenwood_backend_disconnect_with_confirmed_ptt_sends_rx() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(10);
        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // SetPtt(true)
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"TX1;").await.unwrap();

            // Disconnect musi wysłać RX;
            let n = radio_io.read(&mut buf).await.unwrap();
            tx.send(buf[..n].to_vec()).await.unwrap();
        });

        backend.connect().await.expect("connect");
        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("tx");
        assert!(backend.current_state().ptt);

        backend.disconnect().await;
        let rx_bytes = rx.recv().await.unwrap();
        assert_eq!(std::str::from_utf8(&rx_bytes).unwrap(), "RX;");
        assert!(!backend.current_state().ptt);
    }

    #[test]
    fn kenwood_backend_capabilities_match_profile() {
        let backend =
            KenwoodBackend::new_serial(SerialConfig::new("COM1", 115_200), KENWOOD_TS890S.clone());

        let caps = backend.capabilities();
        assert!(caps.frequency);
        assert!(caps.mode);
        assert!(caps.ptt);
        assert!(caps.split);
        assert!(caps.s_meter);
        assert_eq!(backend.kind(), CatBackendKind::Kenwood);
    }

    #[tokio::test]
    async fn kenwood_execute_setter_supports_power_rit_xit() {
        let (client_io, mut radio_io) = duplex(512);
        let mut backend = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());

        tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Connect
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"FA00014074000;").await.unwrap();
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"AI2;").await.unwrap();

            // SetPower 50W -> PC050;
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "PC050;");
            radio_io.write_all(b"PC050;").await.unwrap();

            // SetRit 120 -> RT1;
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "RT1;");
            radio_io.write_all(b"RT1;").await.unwrap();

            // SetXit -150 -> XT1;
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(std::str::from_utf8(&buf[..n]).unwrap(), "XT1;");
            radio_io.write_all(b"XT1;").await.unwrap();

            // SetPassband 2400 (no-op)
        });

        backend.connect().await.expect("connect");
        backend
            .execute(&RigCommand::SetPower(50.0))
            .await
            .expect("SetPower");
        assert_eq!(backend.current_state().rf_power_watts, 50.0);

        backend
            .execute(&RigCommand::SetRit(120))
            .await
            .expect("SetRit");
        assert_eq!(backend.current_state().rit_hz, 120);

        backend
            .execute(&RigCommand::SetXit(-150))
            .await
            .expect("SetXit");
        assert_eq!(backend.current_state().xit_hz, -150);

        backend
            .execute(&RigCommand::SetPassband(2400))
            .await
            .expect("SetPassband");
        backend.disconnect().await;
    }
}
