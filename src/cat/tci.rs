// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Transceiver Control Interface (TCI) Protocol (ExpertSDR / SunSDR / Thetis).
//!
//! Formalna specyfikacja TCI:
//! Dokument: "TCI Protocol.pdf" (ExpertSDR3 / TCI v1.x z oficjalnego repozytorium ExpertSDR3/TCI).
//! Sekcje specyfikacji zaimplementowane w niniejszym module:
//! - "Connection" / "Ready": Handshake WebSocket ws://<host>:<port> kończony instrukcją `ready;`
//! - "VFO": format `vfo:RECEIVER,VFO,FREQUENCY;` (częstotliwości VFO A i VFO B w Hz)
//! - "MODULATION": format `modulation:RECEIVER,MODE;` (mapowanie emisji SDR na nazwy domenowe)
//! - "TRX": format `trx:RECEIVER,STATE;` (sterowanie PTT nadawanie/odbiór)
//! - "SPLIT_ENABLE": format `split_enable:RECEIVER,STATE;` (tryb pracy Split)
//! - "RIT_ENABLE": format `rit_enable:RECEIVER,STATE;` (włączenie/wyłączenie RIT)
//! - "RIT_OFFSET": format `rit_offset:RECEIVER,OFFSET;` (przesunięcie częstotliwości RIT w Hz)
//! - "XIT_ENABLE": format `xit_enable:RECEIVER,STATE;` (włączenie/wyłączenie XIT)
//! - "XIT_OFFSET": format `xit_offset:RECEIVER,OFFSET;` (przesunięcie częstotliwości XIT w Hz)
//! - "DRIVE": format `drive:RECEIVER,VALUE;` (poziom mocy RF 0-100%)
//! - "SPOT": format `spot:CALLSIGN,FREQ,COLOR,TEXT;` (naniesienie spotu DX na wodospad)
//! - "CW_MACROS": format `cw_macros:SPEED,TEXT;` (nadawanie tekstu CW)

use std::collections::VecDeque;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};

use crate::cat::backend::{BoxFuture, CatBackendKind, RigBackend};
use crate::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Komunikat protokołu TCI (powiadomienie serwera lub zdekodowane polecenie).
#[derive(Debug, Clone, PartialEq)]
pub enum TciMessage {
    /// Sygnał zakończenia bloku inicjalizacyjnego (`ready;`).
    Ready,
    /// Aktualizacja częstotliwości VFO (`vfo:receiver,vfo,freq_hz;`).
    Vfo { receiver: u8, vfo: u8, freq_hz: u64 },
    /// Zmiana modulacji (`modulation:receiver,mode;`).
    Modulation { receiver: u8, mode: String },
    /// Stan nadawania PTT (`trx:receiver,transmitting;`).
    Trx { receiver: u8, transmitting: bool },
    /// Stan trybu Split (`split_enable:receiver,enabled;`).
    SplitEnable { receiver: u8, enabled: bool },
    /// Włączenie/wyłączenie RIT (`rit_enable:receiver,enabled;`).
    RitEnable { receiver: u8, enabled: bool },
    /// Przesunięcie RIT w Hz (`rit_offset:receiver,offset_hz;`).
    RitOffset { receiver: u8, offset_hz: i32 },
    /// Włączenie/wyłączenie XIT (`xit_enable:receiver,enabled;`).
    XitEnable { receiver: u8, enabled: bool },
    /// Przesunięcie XIT w Hz (`xit_offset:receiver,offset_hz;`).
    XitOffset { receiver: u8, offset_hz: i32 },
    /// Poziom wysterowania / mocy RF w procentach (`drive:receiver,value;`).
    Drive { receiver: u8, value: u32 },
    /// Spot DX na wodospadzie SDR (`spot:callsign,freq_hz,color,text;`).
    Spot {
        callsign: String,
        freq_hz: u64,
        color: u32,
        text: String,
    },
    /// Makro CW (`cw_macros:speed_wpm,text;`).
    CwMacro { speed_wpm: u8, text: String },
    /// Nieznana lub niewspierana instrukcja TCI.
    Unknown(String),
}

/// Kodek protokołu TCI: podział ramki tekstowej, parsowanie i formatowanie poleceń.
pub struct TciCodec;

/// Alias wstecznej zgodności dla istniejącego API.
pub type TciProtocol = TciCodec;

impl TciCodec {
    /// Formatuje polecenie ustawienia VFO (częstotliwości w Hz).
    pub fn cmd_set_vfo(receiver: u8, vfo: u8, freq_hz: u64) -> String {
        format!("vfo:{receiver},{vfo},{freq_hz};")
    }

    /// Formatuje polecenie zmiany modulacji (USB, LSB, CW, DIGU, etc.).
    pub fn cmd_set_mode(receiver: u8, mode: &str) -> String {
        let tci_mode = Self::mode_to_tci(mode);
        format!("modulation:{receiver},{tci_mode};")
    }

    /// Formatuje polecenie PTT (nadawanie / odbiór).
    pub fn cmd_set_trx(receiver: u8, enable_tx: bool) -> String {
        format!("trx:{receiver},{enable_tx};")
    }

    /// Formatuje polecenie włączenia/wyłączenia trybu Split.
    pub fn cmd_set_split(receiver: u8, enable: bool) -> String {
        format!("split_enable:{receiver},{enable};")
    }

    /// Formatuje polecenie RIT (włączenie + przesunięcie).
    pub fn cmd_set_rit(receiver: u8, enable: bool, offset_hz: i32) -> String {
        if enable {
            format!("rit_enable:{receiver},true;rit_offset:{receiver},{offset_hz};")
        } else {
            format!("rit_enable:{receiver},false;rit_offset:{receiver},0;")
        }
    }

    /// Formatuje polecenie XIT (włączenie + przesunięcie).
    pub fn cmd_set_xit(receiver: u8, enable: bool, offset_hz: i32) -> String {
        if enable {
            format!("xit_enable:{receiver},true;xit_offset:{receiver},{offset_hz};")
        } else {
            format!("xit_enable:{receiver},false;xit_offset:{receiver},0;")
        }
    }

    /// Formatuje polecenie wysterowania mocy RF (0-100).
    pub fn cmd_set_drive(receiver: u8, value: u32) -> String {
        format!("drive:{receiver},{value};")
    }

    /// Formatuje polecenie naniesienia spotu DX na wodospad SDR.
    pub fn cmd_add_spot(callsign: &str, freq_hz: u64, color_argb: u32, text: &str) -> String {
        format!(
            "spot:{},{},{},{};",
            callsign.trim().to_uppercase(),
            freq_hz,
            color_argb,
            text.trim()
        )
    }

    /// Formatuje polecenie nadania tekstu CW.
    pub fn cmd_send_cw(speed_wpm: u8, text: &str) -> String {
        format!("cw_macros:{speed_wpm},{text};")
    }

    /// Tłumaczy domenową nazwę trybu [`RigState`] na nazwę protokołu TCI.
    pub fn mode_to_tci(mode: &str) -> String {
        let m = mode.trim().to_uppercase();
        match m.as_str() {
            "PKTUSB" | "DIGU" => "DIGU".to_string(),
            "PKTLSB" | "DIGL" => "DIGL".to_string(),
            "FM" | "NFM" => "NFM".to_string(),
            "USB" => "USB".to_string(),
            "LSB" => "LSB".to_string(),
            "CW" => "CW".to_string(),
            "AM" => "AM".to_string(),
            "SAM" => "SAM".to_string(),
            "DSB" => "DSB".to_string(),
            "SPEC" => "SPEC".to_string(),
            "DRM" => "DRM".to_string(),
            _ => m,
        }
    }

    /// Tłumaczy nazwę trybu TCI na domenową nazwę [`RigState`].
    pub fn tci_to_mode(mode: &str) -> String {
        let m = mode.trim().to_uppercase();
        match m.as_str() {
            "DIGU" => "PKTUSB".to_string(),
            "DIGL" => "PKTLSB".to_string(),
            "NFM" | "WFM" => "FM".to_string(),
            "FM" => "FM".to_string(),
            "USB" => "USB".to_string(),
            "LSB" => "LSB".to_string(),
            "CW" => "CW".to_string(),
            "AM" => "AM".to_string(),
            "SAM" => "SAM".to_string(),
            "DSB" => "DSB".to_string(),
            "SPEC" => "SPEC".to_string(),
            "DRM" => "DRM".to_string(),
            _ => m,
        }
    }

    /// Parsuje pojedynczą odpowiedź TCI (metoda zgodności wstecznej).
    pub fn parse_message(raw: &str) -> TciMessage {
        let trimmed = raw.trim().trim_end_matches(';');
        Self::decode_instruction(trimmed).unwrap_or_else(|_| TciMessage::Unknown(raw.to_string()))
    }

    /// Dekoduje pojedynczą instrukcję TCI (bez średnika).
    pub fn decode_instruction(raw: &str) -> Result<TciMessage, RigError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(TciMessage::Unknown(String::new()));
        }

        if trimmed == "ready" {
            return Ok(TciMessage::Ready);
        }

        if let Some(rest) = trimmed.strip_prefix("vfo:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 3 {
                return Err(RigError::Protocol {
                    detail: format!("vfo instruction requires at least 3 arguments: '{trimmed}'"),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in vfo: {e}"),
                })?;
            let vfo = parts[1]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid vfo index in vfo: {e}"),
                })?;
            let freq_hz = parts[2]
                .trim()
                .parse::<u64>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid frequency in vfo: {e}"),
                })?;
            return Ok(TciMessage::Vfo {
                receiver,
                vfo,
                freq_hz,
            });
        }

        if let Some(rest) = trimmed.strip_prefix("modulation:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(RigError::Protocol {
                    detail: format!(
                        "modulation instruction requires at least 2 arguments: '{trimmed}'"
                    ),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in modulation: {e}"),
                })?;
            let mode = parts[1].trim().to_uppercase();
            if mode.is_empty() {
                return Err(RigError::Protocol {
                    detail: format!("empty mode in modulation instruction: '{trimmed}'"),
                });
            }
            return Ok(TciMessage::Modulation { receiver, mode });
        }

        if let Some(rest) = trimmed.strip_prefix("trx:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(RigError::Protocol {
                    detail: format!("trx instruction requires at least 2 arguments: '{trimmed}'"),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in trx: {e}"),
                })?;
            let transmitting = parse_tci_bool(parts[1]).ok_or_else(|| RigError::Protocol {
                detail: format!("invalid boolean in trx instruction: '{trimmed}'"),
            })?;
            return Ok(TciMessage::Trx {
                receiver,
                transmitting,
            });
        }

        if let Some(rest) = trimmed.strip_prefix("split_enable:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(RigError::Protocol {
                    detail: format!(
                        "split_enable instruction requires at least 2 arguments: '{trimmed}'"
                    ),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in split_enable: {e}"),
                })?;
            let enabled = parse_tci_bool(parts[1]).ok_or_else(|| RigError::Protocol {
                detail: format!("invalid boolean in split_enable: '{trimmed}'"),
            })?;
            return Ok(TciMessage::SplitEnable { receiver, enabled });
        }

        if let Some(rest) = trimmed.strip_prefix("rit_enable:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(RigError::Protocol {
                    detail: format!(
                        "rit_enable instruction requires at least 2 arguments: '{trimmed}'"
                    ),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in rit_enable: {e}"),
                })?;
            let enabled = parse_tci_bool(parts[1]).ok_or_else(|| RigError::Protocol {
                detail: format!("invalid boolean in rit_enable: '{trimmed}'"),
            })?;
            return Ok(TciMessage::RitEnable { receiver, enabled });
        }

        if let Some(rest) = trimmed.strip_prefix("rit_offset:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(RigError::Protocol {
                    detail: format!(
                        "rit_offset instruction requires at least 2 arguments: '{trimmed}'"
                    ),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in rit_offset: {e}"),
                })?;
            let offset_hz = parts[1]
                .trim()
                .parse::<i32>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid offset in rit_offset: {e}"),
                })?;
            return Ok(TciMessage::RitOffset {
                receiver,
                offset_hz,
            });
        }

        if let Some(rest) = trimmed.strip_prefix("xit_enable:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(RigError::Protocol {
                    detail: format!(
                        "xit_enable instruction requires at least 2 arguments: '{trimmed}'"
                    ),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in xit_enable: {e}"),
                })?;
            let enabled = parse_tci_bool(parts[1]).ok_or_else(|| RigError::Protocol {
                detail: format!("invalid boolean in xit_enable: '{trimmed}'"),
            })?;
            return Ok(TciMessage::XitEnable { receiver, enabled });
        }

        if let Some(rest) = trimmed.strip_prefix("xit_offset:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(RigError::Protocol {
                    detail: format!(
                        "xit_offset instruction requires at least 2 arguments: '{trimmed}'"
                    ),
                });
            }
            let receiver = parts[0]
                .trim()
                .parse::<u8>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid receiver index in xit_offset: {e}"),
                })?;
            let offset_hz = parts[1]
                .trim()
                .parse::<i32>()
                .map_err(|e| RigError::Protocol {
                    detail: format!("invalid offset in xit_offset: {e}"),
                })?;
            return Ok(TciMessage::XitOffset {
                receiver,
                offset_hz,
            });
        }

        if let Some(rest) = trimmed.strip_prefix("drive:") {
            let parts: Vec<&str> = rest.split(',').collect();
            let (receiver, value) = if parts.len() == 1 {
                let val = parts[0]
                    .trim()
                    .parse::<u32>()
                    .map_err(|e| RigError::Protocol {
                        detail: format!("invalid value in drive: {e}"),
                    })?;
                (0, val)
            } else {
                let rx = parts[0]
                    .trim()
                    .parse::<u8>()
                    .map_err(|e| RigError::Protocol {
                        detail: format!("invalid receiver in drive: {e}"),
                    })?;
                let val = parts[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|e| RigError::Protocol {
                        detail: format!("invalid value in drive: {e}"),
                    })?;
                (rx, val)
            };
            return Ok(TciMessage::Drive { receiver, value });
        }

        if let Some(rest) = trimmed.strip_prefix("spot:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() >= 4 {
                let callsign = parts[0].to_string();
                let freq_hz = parts[1]
                    .trim()
                    .parse::<u64>()
                    .map_err(|e| RigError::Protocol {
                        detail: format!("invalid frequency in spot: {e}"),
                    })?;
                let color = parts[2].trim().parse::<u32>().unwrap_or(0xFF00_FF00);
                let text = parts[3].to_string();
                return Ok(TciMessage::Spot {
                    callsign,
                    freq_hz,
                    color,
                    text,
                });
            }
            return Err(RigError::Protocol {
                detail: format!("spot instruction requires at least 4 arguments: '{trimmed}'"),
            });
        }

        if let Some(rest) = trimmed.strip_prefix("cw_macros:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() >= 2 {
                let speed_wpm = parts[0]
                    .trim()
                    .parse::<u8>()
                    .map_err(|e| RigError::Protocol {
                        detail: format!("invalid speed in cw_macros: {e}"),
                    })?;
                let text = parts[1].to_string();
                return Ok(TciMessage::CwMacro { speed_wpm, text });
            }
            return Err(RigError::Protocol {
                detail: format!("cw_macros instruction requires at least 2 arguments: '{trimmed}'"),
            });
        }

        Ok(TciMessage::Unknown(trimmed.to_string()))
    }

    /// Rozbija ramkę tekstową po średniku na listę zdekodowanych instrukcji TCI.
    pub fn decode_frame(text: &str) -> Result<Vec<TciMessage>, RigError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        if !trimmed.ends_with(';') {
            let last_part = match trimmed.rfind(';') {
                Some(idx) => &trimmed[idx + 1..],
                None => trimmed,
            }
            .trim();
            if !last_part.is_empty() {
                return Err(RigError::Protocol {
                    detail: format!(
                        "incomplete TCI instruction without terminating semicolon: '{last_part}'"
                    ),
                });
            }
        }

        let mut instructions = Vec::new();
        for piece in trimmed.split(';') {
            let p = piece.trim();
            if p.is_empty() {
                continue;
            }
            let msg = Self::decode_instruction(p)?;
            instructions.push(msg);
        }
        Ok(instructions)
    }
}

fn parse_tci_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "on" => Some(true),
        "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

/// Backend CAT dla radiostacji SDR wspierających protokół TCI przez WebSocket.
pub struct TciBackend {
    host: String,
    port: u16,
    endpoint: String,
    connect_timeout: Duration,
    send_timeout: Duration,
    short_poll_window: Duration,
    ws_stream: Option<WsStream>,
    state: RigState,
    handshake_completed: bool,
    trx: u8,
    active_vfo: u8,
    vfo_a_freq: u64,
    vfo_b_freq: u64,
    rit_enabled: bool,
    rit_offset: i32,
    xit_enabled: bool,
    xit_offset: i32,
    pending_messages: VecDeque<TciMessage>,
}

impl TciBackend {
    /// Tworzy nową instancję backendu TCI dla wskazanego hosta i portu.
    ///
    /// Konstruktor nie wykonuje operacji wejścia/wyjścia. Połączenie weryfikowane
    /// jest asynchronicznie przez [`RigBackend::connect`].
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        let host = host.into();
        let endpoint = format!("ws://{host}:{port}");
        Self {
            host,
            port,
            endpoint,
            connect_timeout: Duration::from_secs(5),
            send_timeout: Duration::from_secs(2),
            short_poll_window: Duration::from_millis(5),
            ws_stream: None,
            state: RigState {
                connected: false,
                ..Default::default()
            },
            handshake_completed: false,
            trx: 0,
            active_vfo: 0,
            vfo_a_freq: 14_025_000,
            vfo_b_freq: 14_025_000,
            rit_enabled: false,
            rit_offset: 0,
            xit_enabled: false,
            xit_offset: 0,
            pending_messages: VecDeque::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timeout(host: impl Into<String>, port: u16, timeout: Duration) -> Self {
        let mut b = Self::new(host, port);
        b.connect_timeout = timeout;
        b.send_timeout = timeout;
        b.short_poll_window = Duration::from_millis(5);
        b
    }

    /// Sprawdza, czy backend posiada aktywne połączenie WebSocket i ukończony handshake `ready;`.
    pub fn is_connected(&self) -> bool {
        self.handshake_completed && self.ws_stream.is_some()
    }

    fn apply_message(&mut self, msg: &TciMessage) {
        match msg {
            TciMessage::Ready => {
                self.handshake_completed = true;
                self.state.connected = true;
            }
            TciMessage::Vfo {
                receiver,
                vfo,
                freq_hz,
            } => {
                if *receiver != self.trx {
                    return;
                }
                if *vfo == 0 {
                    self.vfo_a_freq = *freq_hz;
                } else if *vfo == 1 {
                    self.vfo_b_freq = *freq_hz;
                }
                if *vfo == self.active_vfo {
                    self.state.frequency_hz = *freq_hz;
                } else {
                    self.state.tx_frequency_hz = Some(*freq_hz);
                }
            }
            TciMessage::Modulation { receiver, mode } => {
                if *receiver != self.trx {
                    return;
                }
                self.state.mode = TciCodec::tci_to_mode(mode);
            }
            TciMessage::Trx {
                receiver,
                transmitting,
            } => {
                if *receiver != self.trx {
                    return;
                }
                self.state.ptt = *transmitting;
            }
            TciMessage::SplitEnable { receiver, enabled } => {
                if *receiver != self.trx {
                    return;
                }
                self.state.split_enabled = *enabled;
                if *enabled {
                    self.state.tx_vfo = if self.active_vfo == 0 {
                        "VFOB".to_string()
                    } else {
                        "VFOA".to_string()
                    };
                    if self.state.tx_frequency_hz.is_none() {
                        self.state.tx_frequency_hz = Some(if self.active_vfo == 0 {
                            self.vfo_b_freq
                        } else {
                            self.vfo_a_freq
                        });
                    }
                } else {
                    self.state.tx_vfo = self.state.vfo.clone();
                    self.state.tx_frequency_hz = None;
                }
            }
            TciMessage::RitEnable { receiver, enabled } => {
                if *receiver != self.trx {
                    return;
                }
                self.rit_enabled = *enabled;
                self.state.rit_hz = if *enabled { self.rit_offset } else { 0 };
            }
            TciMessage::RitOffset {
                receiver,
                offset_hz,
            } => {
                if *receiver != self.trx {
                    return;
                }
                self.rit_offset = *offset_hz;
                self.state.rit_hz = *offset_hz;
            }
            TciMessage::XitEnable { receiver, enabled } => {
                if *receiver != self.trx {
                    return;
                }
                self.xit_enabled = *enabled;
                self.state.xit_hz = if *enabled { self.xit_offset } else { 0 };
            }
            TciMessage::XitOffset {
                receiver,
                offset_hz,
            } => {
                if *receiver != self.trx {
                    return;
                }
                self.xit_offset = *offset_hz;
                self.state.xit_hz = *offset_hz;
            }
            TciMessage::Drive { receiver, value } => {
                if *receiver != self.trx {
                    return;
                }
                self.state.rf_power_watts = *value as f32;
            }
            TciMessage::Spot { .. } | TciMessage::CwMacro { .. } | TciMessage::Unknown(_) => {}
        }
    }

    async fn connect_internal(&mut self) -> Result<(), RigError> {
        self.disconnect_internal().await;

        let url = format!("ws://{}:{}", self.host, self.port);
        let connect_fut = tokio_tungstenite::connect_async(&url);
        let (mut stream, _) = match tokio::time::timeout(self.connect_timeout, connect_fut).await {
            Ok(Ok(pair)) => pair,
            Ok(Err(e)) => {
                return Err(RigError::Connect {
                    endpoint: self.endpoint.clone(),
                    detail: e.to_string(),
                });
            }
            Err(_) => {
                return Err(RigError::Timeout {
                    endpoint: self.endpoint.clone(),
                    operation: "connect",
                });
            }
        };

        let deadline = tokio::time::Instant::now() + self.connect_timeout;
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return Err(RigError::Timeout {
                    endpoint: self.endpoint.clone(),
                    operation: "ready",
                });
            }
            let remaining = deadline - now;
            let next_msg = match tokio::time::timeout(remaining, stream.next()).await {
                Ok(Some(Ok(msg))) => msg,
                Ok(Some(Err(e))) => {
                    return Err(RigError::Connect {
                        endpoint: self.endpoint.clone(),
                        detail: e.to_string(),
                    });
                }
                Ok(None) => {
                    return Err(RigError::Protocol {
                        detail: "WebSocket stream closed before receiving 'ready;'".to_string(),
                    });
                }
                Err(_) => {
                    return Err(RigError::Timeout {
                        endpoint: self.endpoint.clone(),
                        operation: "ready",
                    });
                }
            };

            match next_msg {
                tokio_tungstenite::tungstenite::Message::Text(text) => {
                    let instructions = TciCodec::decode_frame(&text)?;
                    let mut found_ready = false;
                    for instr in instructions {
                        if instr == TciMessage::Ready {
                            found_ready = true;
                        }
                        self.apply_message(&instr);
                    }
                    if found_ready {
                        self.ws_stream = Some(stream);
                        self.handshake_completed = true;
                        self.state.connected = true;
                        return Ok(());
                    }
                }
                tokio_tungstenite::tungstenite::Message::Ping(payload) => {
                    if let Err(e) = stream
                        .send(tokio_tungstenite::tungstenite::Message::Pong(payload))
                        .await
                    {
                        return Err(RigError::Connect {
                            endpoint: self.endpoint.clone(),
                            detail: e.to_string(),
                        });
                    }
                }
                tokio_tungstenite::tungstenite::Message::Pong(_) => {}
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                    return Err(RigError::Protocol {
                        detail: "received Close frame before receiving 'ready;'".to_string(),
                    });
                }
                tokio_tungstenite::tungstenite::Message::Binary(_) => {
                    return Err(RigError::Protocol {
                        detail: "unexpected binary frame during handshake".to_string(),
                    });
                }
                tokio_tungstenite::tungstenite::Message::Frame(_) => {}
            }
        }
    }

    async fn disconnect_internal(&mut self) {
        if let Some(mut stream) = self.ws_stream.take() {
            let _ = tokio::time::timeout(
                Duration::from_millis(200),
                stream.send(tokio_tungstenite::tungstenite::Message::Close(None)),
            )
            .await;
        }
        self.handshake_completed = false;
        self.state.connected = false;
        self.pending_messages.clear();
    }

    async fn poll_internal(&mut self) -> Result<RigState, RigError> {
        if !self.is_connected() || self.ws_stream.is_none() {
            return Err(RigError::NotConnected);
        }

        let mut stream = match self.ws_stream.take() {
            Some(s) => s,
            None => return Err(RigError::NotConnected),
        };

        let mut messages_processed = 0;
        let max_messages = 256;

        let poll_res: Result<(), RigError> = async {
            while messages_processed < max_messages {
                if let Some(msg) = self.pending_messages.pop_front() {
                    self.apply_message(&msg);
                    messages_processed += 1;
                    continue;
                }

                let next_item =
                    match tokio::time::timeout(self.short_poll_window, stream.next()).await {
                        Ok(Some(Ok(msg))) => msg,
                        Ok(Some(Err(e))) => {
                            return Err(RigError::Connect {
                                endpoint: self.endpoint.clone(),
                                detail: e.to_string(),
                            });
                        }
                        Ok(None) => {
                            return Err(RigError::Connect {
                                endpoint: self.endpoint.clone(),
                                detail: "WebSocket stream closed".to_string(),
                            });
                        }
                        Err(_) => {
                            break;
                        }
                    };

                match next_item {
                    tokio_tungstenite::tungstenite::Message::Text(text) => {
                        let instructions = TciCodec::decode_frame(&text)?;
                        for instr in instructions {
                            self.pending_messages.push_back(instr);
                        }
                        if let Some(msg) = self.pending_messages.pop_front() {
                            self.apply_message(&msg);
                            messages_processed += 1;
                        }
                    }
                    tokio_tungstenite::tungstenite::Message::Ping(payload) => {
                        if let Err(e) = stream
                            .send(tokio_tungstenite::tungstenite::Message::Pong(payload))
                            .await
                        {
                            return Err(RigError::Connect {
                                endpoint: self.endpoint.clone(),
                                detail: e.to_string(),
                            });
                        }
                    }
                    tokio_tungstenite::tungstenite::Message::Pong(_) => {}
                    tokio_tungstenite::tungstenite::Message::Close(_) => {
                        return Err(RigError::Protocol {
                            detail: "received Close frame from TCI server".to_string(),
                        });
                    }
                    tokio_tungstenite::tungstenite::Message::Binary(_) => {
                        return Err(RigError::Protocol {
                            detail: "unexpected binary frame in TCI".to_string(),
                        });
                    }
                    tokio_tungstenite::tungstenite::Message::Frame(_) => {}
                }
            }
            Ok(())
        }
        .await;

        match poll_res {
            Ok(()) => {
                self.ws_stream = Some(stream);
                Ok(self.state.clone())
            }
            Err(e) => {
                self.ws_stream = None;
                self.handshake_completed = false;
                self.state.connected = false;
                Err(e)
            }
        }
    }

    async fn execute_cmd(&mut self, command: &RigCommand) -> Result<(), RigError> {
        if !self.is_connected() || self.ws_stream.is_none() {
            return Err(RigError::NotConnected);
        }

        if !self.capabilities().supports(command) {
            let cmd_name = match command {
                RigCommand::SetPassband(_) => "SetPassband",
                _ => "Unknown",
            };
            return Err(RigError::Unsupported {
                backend: CatBackendKind::Tci,
                command: cmd_name,
            });
        }

        let frame_text = match command {
            RigCommand::SetFrequency(hz) => {
                format!("vfo:{},{},{hz};", self.trx, self.active_vfo)
            }
            RigCommand::SetSplitFrequency(hz) => {
                format!("vfo:{},1,{hz};", self.trx)
            }
            RigCommand::SetMode { mode, .. } => {
                let tci_mode = TciCodec::mode_to_tci(mode);
                format!("modulation:{},{tci_mode};", self.trx)
            }
            RigCommand::SetPtt(ptt) => {
                format!("trx:{},{ptt};", self.trx)
            }
            RigCommand::SetSplit { enabled, .. } => {
                format!("split_enable:{},{enabled};", self.trx)
            }
            RigCommand::SetVfo(vfo) => {
                if vfo.eq_ignore_ascii_case("VFOB") || vfo == "1" {
                    self.active_vfo = 1;
                    self.state.vfo = "VFOB".to_string();
                    self.state.frequency_hz = self.vfo_b_freq;
                } else {
                    self.active_vfo = 0;
                    self.state.vfo = "VFOA".to_string();
                    self.state.frequency_hz = self.vfo_a_freq;
                }
                return Ok(());
            }
            RigCommand::SetRit(hz) => {
                if *hz != 0 {
                    format!("rit_enable:{},true;rit_offset:{},{hz};", self.trx, self.trx)
                } else {
                    format!("rit_enable:{},false;rit_offset:{},0;", self.trx, self.trx)
                }
            }
            RigCommand::SetXit(hz) => {
                if *hz != 0 {
                    format!("xit_enable:{},true;xit_offset:{},{hz};", self.trx, self.trx)
                } else {
                    format!("xit_enable:{},false;xit_offset:{},0;", self.trx, self.trx)
                }
            }
            RigCommand::SetPower(watts) => {
                let p = watts.clamp(0.0, 100.0).round() as u32;
                format!("drive:{},{p};", self.trx)
            }
            RigCommand::SetPassband(_) => {
                return Err(RigError::Unsupported {
                    backend: CatBackendKind::Tci,
                    command: "SetPassband",
                });
            }
        };

        let mut stream = match self.ws_stream.take() {
            Some(s) => s,
            None => return Err(RigError::NotConnected),
        };

        let send_fut = stream.send(tokio_tungstenite::tungstenite::Message::Text(
            frame_text.into(),
        ));
        match tokio::time::timeout(self.send_timeout, send_fut).await {
            Ok(Ok(())) => {
                self.ws_stream = Some(stream);
                Ok(())
            }
            Ok(Err(e)) => {
                self.ws_stream = None;
                self.handshake_completed = false;
                self.state.connected = false;
                Err(RigError::Connect {
                    endpoint: self.endpoint.clone(),
                    detail: e.to_string(),
                })
            }
            Err(_) => {
                self.ws_stream = None;
                self.handshake_completed = false;
                self.state.connected = false;
                Err(RigError::Timeout {
                    endpoint: self.endpoint.clone(),
                    operation: "execute",
                })
            }
        }
    }

    /// Asynchroniczne sprawdzenie dostępności i poprawności protokołu TCI (handshake `ready;`).
    pub async fn probe(host: &str, port: u16) -> Result<(), RigError> {
        let endpoint = format!("ws://{host}:{port}");
        let timeout = Duration::from_secs(5);
        let connect_fut = tokio_tungstenite::connect_async(&endpoint);
        let (mut stream, _) = match tokio::time::timeout(timeout, connect_fut).await {
            Ok(Ok(pair)) => pair,
            Ok(Err(e)) => {
                return Err(RigError::Connect {
                    endpoint,
                    detail: e.to_string(),
                });
            }
            Err(_) => {
                return Err(RigError::Timeout {
                    endpoint,
                    operation: "connect",
                });
            }
        };

        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return Err(RigError::Timeout {
                    endpoint,
                    operation: "probe",
                });
            }
            let remaining = deadline - now;
            let next_msg = match tokio::time::timeout(remaining, stream.next()).await {
                Ok(Some(Ok(msg))) => msg,
                Ok(Some(Err(e))) => {
                    return Err(RigError::Connect {
                        endpoint,
                        detail: e.to_string(),
                    });
                }
                Ok(None) => {
                    return Err(RigError::Protocol {
                        detail: "connection closed before 'ready;' during probe".to_string(),
                    });
                }
                Err(_) => {
                    return Err(RigError::Timeout {
                        endpoint,
                        operation: "probe",
                    });
                }
            };

            match next_msg {
                tokio_tungstenite::tungstenite::Message::Text(text) => {
                    let instructions = TciCodec::decode_frame(&text)?;
                    if instructions.iter().any(|m| *m == TciMessage::Ready) {
                        let _ = tokio::time::timeout(
                            Duration::from_millis(200),
                            stream.send(tokio_tungstenite::tungstenite::Message::Close(None)),
                        )
                        .await;
                        return Ok(());
                    }
                }
                tokio_tungstenite::tungstenite::Message::Ping(payload) => {
                    let _ = stream
                        .send(tokio_tungstenite::tungstenite::Message::Pong(payload))
                        .await;
                }
                tokio_tungstenite::tungstenite::Message::Pong(_) => {}
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                    return Err(RigError::Protocol {
                        detail: "received Close frame before 'ready;' during probe".to_string(),
                    });
                }
                tokio_tungstenite::tungstenite::Message::Binary(_) => {
                    return Err(RigError::Protocol {
                        detail: "unexpected binary frame during probe".to_string(),
                    });
                }
                tokio_tungstenite::tungstenite::Message::Frame(_) => {}
            }
        }
    }
}

impl RigBackend for TciBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::Tci
    }

    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities {
            frequency: true,
            mode: true,
            passband: false,
            split: true,
            split_frequency: true,
            ptt: true,
            vfo: true,
            rit: true,
            xit: true,
            power: true,
            s_meter: false,
        }
    }

    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        Box::pin(self.connect_internal())
    }

    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        Box::pin(self.poll_internal())
    }

    fn execute<'a>(&'a mut self, command: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        Box::pin(self.execute_cmd(command))
    }

    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        Box::pin(self.disconnect_internal())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use tokio_tungstenite::tungstenite::Message;

    async fn spawn_mock_tci_server<F, Fut>(handler: F) -> (u16, tokio::task::JoinHandle<()>)
    where
        F: FnOnce(tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>) -> Fut
            + Send
            + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let port = listener.local_addr().expect("local_addr").port();
        let handle = tokio::spawn(async move {
            if let Ok((tcp_stream, _)) = listener.accept().await {
                if let Ok(ws_stream) = tokio_tungstenite::accept_async(tcp_stream).await {
                    handler(ws_stream).await;
                }
            }
        });
        (port, handle)
    }

    #[test]
    fn test_tci_formatters() {
        assert_eq!(
            TciProtocol::cmd_set_vfo(0, 0, 14_074_000),
            "vfo:0,0,14074000;"
        );
        assert_eq!(TciProtocol::cmd_set_mode(0, "usb"), "modulation:0,USB;");
        assert_eq!(TciProtocol::cmd_set_trx(0, true), "trx:0,true;");
        assert_eq!(
            TciProtocol::cmd_add_spot("SP6INA", 14_025_000, 0xFFFF_0000, "599 SP"),
            "spot:SP6INA,14025000,4294901760,599 SP;"
        );
    }

    #[test]
    fn test_tci_parser() {
        let msg = TciProtocol::parse_message("vfo:0,0,7074000;");
        assert_eq!(
            msg,
            TciMessage::Vfo {
                receiver: 0,
                vfo: 0,
                freq_hz: 7_074_000
            }
        );

        let msg_mod = TciProtocol::parse_message("modulation:0,CW;");
        assert_eq!(
            msg_mod,
            TciMessage::Modulation {
                receiver: 0,
                mode: "CW".to_string()
            }
        );

        let msg_trx = TciProtocol::parse_message("trx:0,true;");
        assert_eq!(
            msg_trx,
            TciMessage::Trx {
                receiver: 0,
                transmitting: true
            }
        );
    }

    #[test]
    fn test_tci_codec_table() {
        let frame = "vfo:0,0,14074000;modulation:0,DIGU;trx:0,false;split_enable:0,true;ready;";
        let decoded = TciCodec::decode_frame(frame).expect("decode_frame");
        assert_eq!(
            decoded,
            vec![
                TciMessage::Vfo {
                    receiver: 0,
                    vfo: 0,
                    freq_hz: 14_074_000
                },
                TciMessage::Modulation {
                    receiver: 0,
                    mode: "DIGU".to_string()
                },
                TciMessage::Trx {
                    receiver: 0,
                    transmitting: false
                },
                TciMessage::SplitEnable {
                    receiver: 0,
                    enabled: true
                },
                TciMessage::Ready,
            ]
        );

        // Puste fragmenty
        let empty_decoded = TciCodec::decode_frame(";;  ;").expect("empty fragments");
        assert!(empty_decoded.is_empty());

        // Niekompletna instrukcja bez średnika
        let incomplete = TciCodec::decode_frame("vfo:0,0,14074000;incomplete_cmd");
        assert!(matches!(incomplete, Err(RigError::Protocol { .. })));

        // Błędne argumenty liczbowe
        let bad_num = TciCodec::decode_frame("vfo:0,abc,14074000;");
        assert!(matches!(bad_num, Err(RigError::Protocol { .. })));

        // Błędny bool
        let bad_bool = TciCodec::decode_frame("trx:0,not_a_bool;");
        assert!(matches!(bad_bool, Err(RigError::Protocol { .. })));

        // Nieznana instrukcja zachowana jako Unknown
        let unknown = TciCodec::decode_frame("volume:0,-20;").expect("unknown parsed");
        assert_eq!(
            unknown,
            vec![TciMessage::Unknown("volume:0,-20".to_string())]
        );

        // Mapowanie trybów
        assert_eq!(TciCodec::mode_to_tci("pktusb"), "DIGU");
        assert_eq!(TciCodec::mode_to_tci("pktlsb"), "DIGL");
        assert_eq!(TciCodec::mode_to_tci("fm"), "NFM");
        assert_eq!(TciCodec::tci_to_mode("DIGU"), "PKTUSB");
        assert_eq!(TciCodec::tci_to_mode("DIGL"), "PKTLSB");
        assert_eq!(TciCodec::tci_to_mode("NFM"), "FM");
    }

    #[tokio::test]
    async fn tci_connect_waits_for_ready() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("vfo:0,0,14074000;modulation:0,USB;".into()))
                .await
                .expect("send initial");
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        assert!(!backend.is_connected());

        backend.connect().await.expect("connect succeeds");
        assert!(backend.is_connected());

        let state = backend.poll().await.expect("poll succeeds");
        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.mode, "USB");
        assert!(state.connected);

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_connect_times_out_without_ready() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("vfo:0,0,14074000;".into()))
                .await
                .expect("send without ready");
            // Nie wysyła ready;, czeka na rozłączenie przez timeout klienta
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(100));
        let res = backend.connect().await;
        assert!(matches!(res, Err(RigError::Timeout { .. })));
        assert!(!backend.is_connected());

        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_poll_applies_notifications() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            // Kolejne powiadomienia
            ws.send(Message::Text(
                "vfo:0,0,7074000;modulation:0,CW;trx:0,true;split_enable:0,true;rit_offset:0,150;xit_offset:0,-200;drive:0,80;".into()
            ))
            .await
            .expect("send notifications");
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        let state = backend.poll().await.expect("poll");
        assert_eq!(state.frequency_hz, 7_074_000);
        assert_eq!(state.mode, "CW");
        assert!(state.ptt);
        assert!(state.split_enabled);
        assert_eq!(state.rit_hz, 150);
        assert_eq!(state.xit_hz, -200);
        assert!((state.rf_power_watts - 80.0).abs() < f32::EPSILON);

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_execute_sends_expected_frames() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");

            let mut received = Vec::new();
            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Text(text) = msg {
                    received.push(text);
                    if received.len() == 11 {
                        break;
                    }
                }
            }
            assert_eq!(
                received,
                vec![
                    "vfo:0,0,14074000;".to_string(),
                    "vfo:0,1,14075000;".to_string(),
                    "modulation:0,USB;".to_string(),
                    "modulation:0,DIGU;".to_string(),
                    "modulation:0,NFM;".to_string(),
                    "trx:0,true;".to_string(),
                    "split_enable:0,true;".to_string(),
                    "vfo:0,1,14076000;".to_string(),
                    "rit_enable:0,true;rit_offset:0,120;".to_string(),
                    "rit_enable:0,false;rit_offset:0,0;".to_string(),
                    "drive:0,65;".to_string(),
                ]
            );
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        backend
            .execute(&RigCommand::SetFrequency(14_074_000))
            .await
            .expect("set freq");
        backend
            .execute(&RigCommand::SetSplitFrequency(14_075_000))
            .await
            .expect("set split freq");
        backend
            .execute(&RigCommand::SetMode {
                mode: "USB".to_string(),
                passband_hz: 2400,
            })
            .await
            .expect("set mode USB");
        backend
            .execute(&RigCommand::SetMode {
                mode: "PKTUSB".to_string(),
                passband_hz: 3000,
            })
            .await
            .expect("set mode PKTUSB");
        backend
            .execute(&RigCommand::SetMode {
                mode: "FM".to_string(),
                passband_hz: 15000,
            })
            .await
            .expect("set mode FM");
        backend
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("set PTT");
        backend
            .execute(&RigCommand::SetSplit {
                enabled: true,
                tx_vfo: "VFOB".to_string(),
            })
            .await
            .expect("set split");
        backend
            .execute(&RigCommand::SetVfo("VFOB".to_string()))
            .await
            .expect("set VFO B");
        backend
            .execute(&RigCommand::SetFrequency(14_076_000))
            .await
            .expect("set freq on active VFO B");
        backend
            .execute(&RigCommand::SetRit(120))
            .await
            .expect("set RIT");
        backend
            .execute(&RigCommand::SetRit(0))
            .await
            .expect("clear RIT");
        backend
            .execute(&RigCommand::SetPower(65.0))
            .await
            .expect("set power");

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_unsupported_commands() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        let res = backend.execute(&RigCommand::SetPassband(500)).await;
        assert_eq!(
            res,
            Err(RigError::Unsupported {
                backend: CatBackendKind::Tci,
                command: "SetPassband",
            })
        );

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_splits_multi_command_frames() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            ws.send(Message::Text(
                "vfo:0,0,14074000;modulation:0,DIGU;trx:0,false;drive:0,100;".into(),
            ))
            .await
            .expect("send multi frame");
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        let state = backend.poll().await.expect("poll");
        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.mode, "PKTUSB");
        assert!(!state.ptt);
        assert!((state.rf_power_watts - 100.0).abs() < f32::EPSILON);

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_close_frame_is_error() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            ws.send(Message::Close(None)).await.expect("send close");
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        let poll_res = backend.poll().await;
        assert!(matches!(poll_res, Err(RigError::Protocol { .. })));
        assert!(!backend.is_connected());

        assert_eq!(backend.poll().await, Err(RigError::NotConnected));
        assert_eq!(
            backend.execute(&RigCommand::SetPtt(false)).await,
            Err(RigError::NotConnected)
        );

        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_ping_receives_pong() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            ws.send(Message::Ping(vec![0xAA, 0xBB].into()))
                .await
                .expect("send ping");
            if let Some(Ok(Message::Pong(payload))) = ws.next().await {
                assert_eq!(payload.as_ref(), &[0xAA, 0xBB]);
            } else {
                panic!("expected Pong frame");
            }
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        let _ = backend.poll().await.expect("poll");

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_probe_ok_and_timeout() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            let _ = ws.next().await;
        })
        .await;

        let probe_ok = TciBackend::probe("127.0.0.1", port).await;
        assert!(probe_ok.is_ok());

        // Port bez aktywnego nasłuchiwania -> błąd połączenia
        let probe_err = TciBackend::probe("127.0.0.1", 1).await;
        assert!(matches!(probe_err, Err(RigError::Connect { .. })));

        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_poll_without_connect_is_not_connected() {
        let mut backend = TciBackend::new("127.0.0.1", 40001);
        assert_eq!(backend.poll().await, Err(RigError::NotConnected));
    }

    #[tokio::test]
    async fn tci_execute_without_connect_is_not_connected() {
        let mut backend = TciBackend::new("127.0.0.1", 40001);
        assert_eq!(
            backend.execute(&RigCommand::SetPtt(true)).await,
            Err(RigError::NotConnected)
        );
    }

    #[tokio::test]
    async fn tci_disconnect_is_idempotent() {
        let mut backend = TciBackend::new("127.0.0.1", 40001);
        backend.disconnect().await;
        backend.disconnect().await;
        backend.disconnect().await;
        assert!(!backend.is_connected());
    }

    #[test]
    fn tci_kind_and_capabilities() {
        let backend = TciBackend::new("127.0.0.1", 40001);
        assert_eq!(backend.kind(), CatBackendKind::Tci);

        let caps = backend.capabilities();
        assert!(caps.frequency);
        assert!(caps.mode);
        assert!(!caps.passband);
        assert!(caps.split);
        assert!(caps.split_frequency);
        assert!(caps.ptt);
        assert!(caps.vfo);
        assert!(caps.rit);
        assert!(caps.xit);
        assert!(caps.power);
        assert!(!caps.s_meter);

        assert!(caps.supports(&RigCommand::SetFrequency(14_074_000)));
        assert!(caps.supports(&RigCommand::SetSplitFrequency(14_075_000)));
        assert!(caps.supports(&RigCommand::SetMode {
            mode: "CW".to_string(),
            passband_hz: 500
        }));
        assert!(!caps.supports(&RigCommand::SetPassband(500)));
        assert!(caps.supports(&RigCommand::SetSplit {
            enabled: true,
            tx_vfo: "VFOB".to_string()
        }));
        assert!(caps.supports(&RigCommand::SetPtt(true)));
        assert!(caps.supports(&RigCommand::SetVfo("VFOA".to_string())));
        assert!(caps.supports(&RigCommand::SetRit(100)));
        assert!(caps.supports(&RigCommand::SetXit(-50)));
        assert!(caps.supports(&RigCommand::SetPower(50.0)));
    }

    #[tokio::test]
    async fn tci_invalid_known_message_is_protocol_error() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            ws.send(Message::Text("vfo:not_a_num,0,14074000;".into()))
                .await
                .expect("send invalid");
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        let res = backend.poll().await;
        assert!(matches!(res, Err(RigError::Protocol { .. })));

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_unknown_message_follows_contract() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            ws.send(Message::Text(
                "unknown_telemetry:123,456;vfo:0,0,14074000;".into(),
            ))
            .await
            .expect("send unknown and vfo");
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        let state = backend.poll().await.expect("poll");
        assert_eq!(state.frequency_hz, 14_074_000);

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }

    #[tokio::test]
    async fn tci_poll_256_messages_boundary() {
        let (port, server_handle) = spawn_mock_tci_server(|mut ws| async move {
            ws.send(Message::Text("ready;".into()))
                .await
                .expect("send ready");
            // Wysyłamy 260 instrukcji w jednej ramce
            let mut batch = String::new();
            for i in 1..=260 {
                batch.push_str(&format!("vfo:0,0,{i};\n"));
            }
            ws.send(Message::Text(batch.into()))
                .await
                .expect("send 260 batch");
            let _ = ws.next().await;
        })
        .await;

        let mut backend = TciBackend::with_timeout("127.0.0.1", port, Duration::from_millis(500));
        backend.connect().await.expect("connect");

        // Pierwszy poll powinien przetworzyć dokładnie 256 komunikatów (częstotliwość dojdzie do 256)
        let state1 = backend.poll().await.expect("first poll");
        assert_eq!(state1.frequency_hz, 256);

        // Drugi poll powinien przetworzyć pozostałe 4 komunikaty (częstotliwość dojdzie do 260)
        let state2 = backend.poll().await.expect("second poll");
        assert_eq!(state2.frequency_hz, 260);

        backend.disconnect().await;
        server_handle.await.expect("server completed");
    }
}
