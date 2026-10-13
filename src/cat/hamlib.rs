// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::tcp::{self, MAX_LINE_LEN};
use std::future::Future;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::broadcast;
use tokio::time::sleep;

use crate::cat::backend::{CatBackendKind, RigBackend};
use crate::cat::rig::{RigCapabilities, RigCommand, RigError};

const DAEMON: &str = "rigctld";

async fn connect_timeout(host: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    tcp::connect_timeout(&format!("{host}:{port}"), DAEMON).await
}

async fn write_cmd<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    cmd: &[u8],
) -> Result<(), std::io::Error> {
    tcp::write_cmd(writer, cmd, DAEMON).await
}

async fn read_line_timeout<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
    line: &mut String,
) -> Result<usize, std::io::Error> {
    tcp::read_line_timeout(reader, line, DAEMON).await
}

async fn send_setter_cmd(host: &str, port: u16, cmd: &str) -> Result<(), std::io::Error> {
    tcp::send_cmd_ignore_reply(&format!("{host}:{port}"), cmd, DAEMON).await
}

// Re-eksport przejściowy RigState z modułu domenowego crate::cat::rig
pub use crate::cat::rig::RigState;

/// Asynchroniczny klient do demona Hamlib (rigctld) wspierający rozszerzenia Hamlib 4.6+
pub struct HamlibClient {
    host: String,
    port: u16,
    state_sender: broadcast::Sender<RigState>,
}

impl HamlibClient {
    pub fn new(host: impl Into<String>, port: u16) -> (Self, broadcast::Receiver<RigState>) {
        let (tx, rx) = broadcast::channel(64);
        (
            Self {
                host: host.into(),
                port,
                state_sender: tx,
            },
            rx,
        )
    }

    /// Konwertuje skalibrowaną wartość S-metra (dB względem S9, komenda `l STRENGTH`) na czytelny format S-unit
    pub fn raw_str_to_s_unit(db: f32) -> String {
        // W standardzie IARU S9 = 0 dB (względnie w Hamlib STRENGTH), a każdy stopień S poniżej S9 to 6 dB (-54..0 dB)
        if db >= 0.0 {
            format!("S9+{db:02.0}dB")
        } else {
            let s_val = ((db + 54.0) / 6.0).clamp(0.0, 9.0).round() as u8;
            format!("S{s_val}")
        }
    }

    /// Uruchamia pętlę ciągłego odpytywania stanu transceivera w tle
    pub async fn run_poll_loop(&self, poll_interval_ms: u64) {
        'poll: loop {
            if let Ok(stream) = connect_timeout(&self.host, self.port).await {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);
                let mut current_state = RigState {
                    connected: true,
                    ..Default::default()
                };

                while current_state.connected {
                    // 1. Odpytaj o częstotliwość ('f')
                    if write_cmd(&mut writer, b"f\n").await.is_err() {
                        break;
                    }
                    let mut line = String::new();
                    if read_line_timeout(&mut buf_reader, &mut line).await.is_err()
                        || line.is_empty()
                        || line.len() > MAX_LINE_LEN
                    {
                        break;
                    }
                    if let Ok(freq) = line.trim().parse::<u64>() {
                        current_state.frequency_hz = freq;
                    }

                    // 2. Odpytaj o emisję i szerokość filtru ('m')
                    if write_cmd(&mut writer, b"m\n").await.is_err() {
                        break;
                    }
                    if read_line_timeout(&mut buf_reader, &mut line).await.is_err()
                        || line.is_empty()
                        || line.len() > MAX_LINE_LEN
                    {
                        break;
                    }
                    let trimmed_mode = line.trim();
                    if !trimmed_mode.starts_with("RPRT") {
                        current_state.mode = trimmed_mode.to_uppercase();
                        if read_line_timeout(&mut buf_reader, &mut line).await.is_ok() {
                            if let Ok(pb) = line.trim().parse::<u32>() {
                                current_state.passband_hz = pb;
                            }
                        }
                    }

                    // 3. Odpytaj o skalibrowany S-Meter ('l STRENGTH' — dB względem S9 w Hamlib)
                    if write_cmd(&mut writer, b"l STRENGTH\n").await.is_ok()
                        && read_line_timeout(&mut buf_reader, &mut line).await.is_ok()
                    {
                        if let Ok(val) = line.trim().parse::<f32>() {
                            current_state.s_meter_dbm = val;
                            current_state.s_meter_unit = Self::raw_str_to_s_unit(val);
                        }
                    }

                    // 4. Odpytaj o Split ('s')
                    if write_cmd(&mut writer, b"s\n").await.is_ok()
                        && read_line_timeout(&mut buf_reader, &mut line).await.is_ok()
                    {
                        let trimmed = line.trim();
                        if !trimmed.starts_with("RPRT") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if let Some(&s_flag) = parts.first() {
                                current_state.split_enabled = s_flag == "1";
                            }
                            // Druga linia odpowiedzi 's' w rigctld to TX VFO
                            let mut tx_vfo_line = String::new();
                            if read_line_timeout(&mut buf_reader, &mut tx_vfo_line)
                                .await
                                .is_ok()
                            {
                                let tx_vfo_trimmed = tx_vfo_line.trim();
                                if !tx_vfo_trimmed.is_empty() && !tx_vfo_trimmed.starts_with("RPRT")
                                {
                                    current_state.tx_vfo = tx_vfo_trimmed.to_string();
                                }
                            }
                        }
                    }

                    // 5. Odpytaj o moc RF ('l RFPOWER')
                    if write_cmd(&mut writer, b"l RFPOWER\n").await.is_ok()
                        && read_line_timeout(&mut buf_reader, &mut line).await.is_ok()
                    {
                        if let Ok(pwr) = line.trim().parse::<f32>() {
                            current_state.rf_power_watts = pwr * 100.0; // Hamlib zwraca RFPOWER w zakresie 0.0–1.0; tutaj zachowujemy skalę 0–100.
                        }
                    }

                    // 6. Odpytaj o stan PTT ('t')
                    if write_cmd(&mut writer, b"t\n").await.is_ok()
                        && read_line_timeout(&mut buf_reader, &mut line).await.is_ok()
                    {
                        let trimmed_ptt = line.trim();
                        if !trimmed_ptt.starts_with("RPRT") {
                            current_state.ptt = trimmed_ptt == "1";
                        }
                    }

                    if self.state_sender.send(current_state.clone()).is_err() {
                        break 'poll;
                    }
                    sleep(Duration::from_millis(poll_interval_ms)).await;
                }
            } else {
                let offline = RigState {
                    connected: false,
                    ..Default::default()
                };
                if self.state_sender.send(offline).is_err() {
                    break 'poll;
                }
                sleep(Duration::from_secs(2)).await;
            }
        }
    }

    /// Przestawia częstotliwość radia (w Hz)
    pub async fn set_frequency(host: &str, port: u16, freq_hz: u64) -> Result<(), std::io::Error> {
        let cmd = format!("F {freq_hz}\n");
        send_setter_cmd(host, port, &cmd).await
    }

    /// Przestawia częstotliwość nadawania w trybie Split (w Hz)
    pub async fn set_split_frequency(
        host: &str,
        port: u16,
        freq_hz: u64,
    ) -> Result<(), std::io::Error> {
        let cmd = format!("\\set_split_freq {freq_hz}\n");
        send_setter_cmd(host, port, &cmd).await
    }

    /// Przestawia emisję i filtr radia
    pub async fn set_mode(
        host: &str,
        port: u16,
        mode: &str,
        passband_hz: u32,
    ) -> Result<(), std::io::Error> {
        let cmd = format!("M {mode} {passband_hz}\n");
        send_setter_cmd(host, port, &cmd).await
    }

    /// Włącza lub wyłącza tryb Split
    pub async fn set_split(
        host: &str,
        port: u16,
        enabled: bool,
        tx_vfo: &str,
    ) -> Result<(), std::io::Error> {
        let cmd = format!("S {} {}\n", i32::from(enabled), tx_vfo);
        send_setter_cmd(host, port, &cmd).await
    }

    /// Załącza lub wyłącza PTT (nadawanie)
    pub async fn set_ptt(host: &str, port: u16, ptt: bool) -> Result<(), std::io::Error> {
        let cmd = format!("T {}\n", i32::from(ptt));
        send_setter_cmd(host, port, &cmd).await
    }

    /// Wybiera aktywny VFO (np. "VFOA", "VFOB", "Main", "Sub")
    pub async fn set_vfo(host: &str, port: u16, vfo: &str) -> Result<(), std::io::Error> {
        let cmd = format!("V {vfo}\n");
        send_setter_cmd(host, port, &cmd).await
    }

    /// Ustawia przesunięcie RIT w Hz (może być ujemne)
    pub async fn set_rit(host: &str, port: u16, rit_hz: i32) -> Result<(), std::io::Error> {
        let cmd = format!("J {rit_hz}\n");
        send_setter_cmd(host, port, &cmd).await
    }

    /// Ustawia przesunięcie XIT w Hz (może być ujemne)
    pub async fn set_xit(host: &str, port: u16, xit_hz: i32) -> Result<(), std::io::Error> {
        let cmd = format!("Z {xit_hz}\n");
        send_setter_cmd(host, port, &cmd).await
    }

    /// Ustawia moc wyjściową (0–100 W), wysyłając znormalizowaną wartość 0.0–1.0
    pub async fn set_power(host: &str, port: u16, watts: f32) -> Result<(), std::io::Error> {
        let norm = (watts / 100.0).clamp(0.0, 1.0);
        let cmd = format!("L RFPOWER {norm:.4}\n");
        send_setter_cmd(host, port, &cmd).await
    }
}

/// Implementacja wspólnego interfejsu [`crate::cat::backend::CatBackend`]
/// dla demona Hamlib (`rigctld`).
impl crate::cat::backend::CatBackend for HamlibClient {
    fn kind(&self) -> crate::cat::backend::CatBackendKind {
        crate::cat::backend::CatBackendKind::Hamlib
    }

    async fn connect(&mut self) -> Result<(), String> {
        let addr = format!("{}:{}", self.host, self.port);
        connect_timeout(&self.host, self.port)
            .await
            .map(|_| ())
            .map_err(|e| format!("Brak połączenia z rigctld {addr}: {e}"))
    }

    async fn poll_state(&mut self) -> Result<RigState, String> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = connect_timeout(&self.host, self.port)
            .await
            .map_err(|e| format!("Brak połączenia z rigctld {addr}: {e}"))?;
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut state = RigState {
            connected: true,
            ..Default::default()
        };

        // Częstotliwość ('f')
        if write_cmd(&mut writer, b"f\n").await.is_err() {
            return Err("rigctld: brak odpowiedzi (częstotliwość)".to_string());
        }
        let mut line = String::new();
        if read_line_timeout(&mut buf_reader, &mut line).await.is_err()
            || line.is_empty()
            || line.len() > MAX_LINE_LEN
        {
            return Err("rigctld: brak odpowiedzi (częstotliwość)".to_string());
        }
        if let Ok(freq) = line.trim().parse::<u64>() {
            state.frequency_hz = freq;
        }

        // Emisja i szerokość filtru ('m')
        if write_cmd(&mut writer, b"m\n").await.is_err() {
            return Err("rigctld: brak odpowiedzi (emisja)".to_string());
        }
        if read_line_timeout(&mut buf_reader, &mut line).await.is_ok() {
            let trimmed_mode = line.trim();
            if !trimmed_mode.is_empty() && !trimmed_mode.starts_with("RPRT") {
                state.mode = trimmed_mode.to_uppercase();
                if read_line_timeout(&mut buf_reader, &mut line).await.is_ok() {
                    if let Ok(pb) = line.trim().parse::<u32>() {
                        state.passband_hz = pb;
                    }
                }
            }
        }

        Ok(state)
    }

    async fn set_frequency(&self, freq_hz: u64) -> Result<(), String> {
        HamlibClient::set_frequency(&self.host, self.port, freq_hz)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_mode(&self, mode: &str, passband_hz: u32) -> Result<(), String> {
        HamlibClient::set_mode(&self.host, self.port, mode, passband_hz)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_split(&self, enabled: bool, tx_vfo: &str) -> Result<(), String> {
        HamlibClient::set_split(&self.host, self.port, enabled, tx_vfo)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_ptt(&self, ptt: bool) -> Result<(), String> {
        HamlibClient::set_ptt(&self.host, self.port, ptt)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_vfo(&self, vfo: &str) -> Result<(), String> {
        HamlibClient::set_vfo(&self.host, self.port, vfo)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_rit(&self, rit_hz: i32) -> Result<(), String> {
        HamlibClient::set_rit(&self.host, self.port, rit_hz)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_xit(&self, xit_hz: i32) -> Result<(), String> {
        HamlibClient::set_xit(&self.host, self.port, xit_hz)
            .await
            .map_err(|e| e.to_string())
    }

    async fn set_power(&self, watts: f32) -> Result<(), String> {
        HamlibClient::set_power(&self.host, self.port, watts)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Trwały backend protokołu Hamlib (`rigctld`) utrzymujący pojedynczą sesję TCP.
pub struct HamlibBackend {
    host: String,
    port: u16,
    endpoint: String,
    timeout: Duration,
    reader: Option<BufReader<OwnedReadHalf>>,
    writer: Option<OwnedWriteHalf>,
    last_mode: Option<String>,
}

impl HamlibBackend {
    /// Tworzy nowy backend Hamlib dla wskazanego hosta i portu.
    ///
    /// Konstruktor nie wykonuje operacji wejścia/wyjścia. Połączenie nawiązywane
    /// jest asynchronicznie przez [`RigBackend::connect`].
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        let host = host.into();
        let endpoint = format!("{host}:{port}");
        Self {
            host,
            port,
            endpoint,
            timeout: Duration::from_millis(1500),
            reader: None,
            writer: None,
            last_mode: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timeout(host: impl Into<String>, port: u16, timeout: Duration) -> Self {
        let mut backend = Self::new(host, port);
        backend.timeout = timeout;
        backend
    }

    /// Sprawdza, czy sesja TCP jest obecnie aktywna.
    pub fn is_connected(&self) -> bool {
        self.reader.is_some() && self.writer.is_some()
    }

    async fn write_cmd_internal(&mut self, cmd: &str) -> Result<(), RigError> {
        let writer = match self.writer.as_mut() {
            Some(w) => w,
            None => return Err(RigError::NotConnected),
        };
        match tokio::time::timeout(self.timeout, writer.write_all(cmd.as_bytes())).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                self.reader = None;
                self.writer = None;
                Err(RigError::Connect {
                    endpoint: self.endpoint.clone(),
                    detail: e.to_string(),
                })
            }
            Err(_) => {
                self.reader = None;
                self.writer = None;
                Err(RigError::Timeout {
                    endpoint: self.endpoint.clone(),
                    operation: "write",
                })
            }
        }
    }

    async fn read_line_internal(&mut self) -> Result<String, RigError> {
        let reader = match self.reader.as_mut() {
            Some(r) => r,
            None => return Err(RigError::NotConnected),
        };
        let mut line = String::new();
        let read_res = {
            let mut limited = reader.take((MAX_LINE_LEN + 1) as u64);
            tokio::time::timeout(self.timeout, limited.read_line(&mut line)).await
        };
        match read_res {
            Ok(Ok(0)) => {
                self.reader = None;
                self.writer = None;
                Err(RigError::Protocol {
                    detail: "connection closed by remote host (EOF)".to_string(),
                })
            }
            Ok(Ok(bytes_read)) => {
                if bytes_read > MAX_LINE_LEN
                    || (bytes_read == MAX_LINE_LEN + 1 && !line.ends_with('\n'))
                {
                    self.reader = None;
                    self.writer = None;
                    return Err(RigError::Protocol {
                        detail: format!(
                            "response line exceeded maximum length of {MAX_LINE_LEN} bytes"
                        ),
                    });
                }
                Ok(line)
            }
            Ok(Err(e)) => {
                self.reader = None;
                self.writer = None;
                Err(RigError::Connect {
                    endpoint: self.endpoint.clone(),
                    detail: e.to_string(),
                })
            }
            Err(_) => {
                self.reader = None;
                self.writer = None;
                Err(RigError::Timeout {
                    endpoint: self.endpoint.clone(),
                    operation: "read",
                })
            }
        }
    }

    async fn connect_internal(&mut self) -> Result<(), RigError> {
        self.disconnect_internal().await;

        let endpoint = format!("{}:{}", self.host, self.port);
        self.endpoint = endpoint;

        let connect_fut = TcpStream::connect(&self.endpoint);
        match tokio::time::timeout(self.timeout, connect_fut).await {
            Ok(Ok(stream)) => {
                let (reader, writer) = stream.into_split();
                self.reader = Some(BufReader::new(reader));
                self.writer = Some(writer);
                Ok(())
            }
            Ok(Err(e)) => Err(RigError::Connect {
                endpoint: self.endpoint.clone(),
                detail: e.to_string(),
            }),
            Err(_) => Err(RigError::Timeout {
                endpoint: self.endpoint.clone(),
                operation: "connect",
            }),
        }
    }

    async fn disconnect_internal(&mut self) {
        self.reader = None;
        self.writer = None;
    }

    async fn execute_cmd(&mut self, command: &RigCommand) -> Result<(), RigError> {
        if !self.is_connected() {
            return Err(RigError::NotConnected);
        }

        let cmd_str = match command {
            RigCommand::SetFrequency(hz) => format!("F {hz}\n"),
            RigCommand::SetSplitFrequency(hz) => format!("\\set_split_freq {hz}\n"),
            RigCommand::SetMode { mode, passband_hz } => format!("M {mode} {passband_hz}\n"),
            RigCommand::SetPassband(passband_hz) => {
                let mode = match &self.last_mode {
                    Some(m) if !m.is_empty() => m.clone(),
                    _ => {
                        return Err(RigError::Protocol {
                            detail: "cannot set passband without previously known mode".to_string(),
                        });
                    }
                };
                format!("M {mode} {passband_hz}\n")
            }
            RigCommand::SetSplit { enabled, tx_vfo } => {
                let flag = if *enabled { 1 } else { 0 };
                format!("S {flag} {tx_vfo}\n")
            }
            RigCommand::SetPtt(enabled) => {
                let flag = if *enabled { 1 } else { 0 };
                format!("T {flag}\n")
            }
            RigCommand::SetVfo(vfo) => format!("V {vfo}\n"),
            RigCommand::SetRit(hz) => format!("J {hz}\n"),
            RigCommand::SetXit(hz) => format!("Z {hz}\n"),
            RigCommand::SetPower(watts) => {
                let norm = (watts / 100.0).clamp(0.0, 1.0);
                format!("L RFPOWER {norm:.4}\n")
            }
        };

        self.write_cmd_internal(&cmd_str).await?;
        let reply = self.read_line_internal().await?;
        parse_rprt(&reply)?;

        if let RigCommand::SetMode { mode, .. } = command {
            self.last_mode = Some(mode.clone());
        }

        Ok(())
    }

    async fn poll_internal(&mut self) -> Result<RigState, RigError> {
        if !self.is_connected() {
            return Err(RigError::NotConnected);
        }

        let mut state = RigState {
            connected: true,
            ..Default::default()
        };

        // 1. Częstotliwość ('f')
        self.write_cmd_internal("f\n").await?;
        let f_line = self.read_line_internal().await?;
        let trimmed_f = f_line.trim();
        if trimmed_f.starts_with("RPRT") {
            parse_rprt(trimmed_f)?;
        } else if let Ok(freq) = trimmed_f.parse::<u64>() {
            state.frequency_hz = freq;
        } else {
            return Err(RigError::Protocol {
                detail: format!("invalid frequency response: '{trimmed_f}'"),
            });
        }

        // 2. Emisja i szerokość filtru ('m')
        self.write_cmd_internal("m\n").await?;
        let m_line = self.read_line_internal().await?;
        let trimmed_mode = m_line.trim();
        if !trimmed_mode.starts_with("RPRT") && !trimmed_mode.is_empty() {
            let upper_mode = trimmed_mode.to_uppercase();
            state.mode = upper_mode.clone();
            self.last_mode = Some(upper_mode);
            let pb_line = self.read_line_internal().await?;
            if let Ok(pb) = pb_line.trim().parse::<u32>() {
                state.passband_hz = pb;
            }
        }

        // 3. S-Meter ('l STRENGTH')
        self.write_cmd_internal("l STRENGTH\n").await?;
        let s_line = self.read_line_internal().await?;
        let trimmed_s = s_line.trim();
        if !trimmed_s.starts_with("RPRT") && !trimmed_s.is_empty() {
            if let Ok(db) = trimmed_s.parse::<f32>() {
                state.s_meter_dbm = db;
                state.s_meter_unit = HamlibClient::raw_str_to_s_unit(db);
            }
        }

        // 4. Split ('s')
        self.write_cmd_internal("s\n").await?;
        let s_flag_line = self.read_line_internal().await?;
        let trimmed_split = s_flag_line.trim();
        if !trimmed_split.starts_with("RPRT") && !trimmed_split.is_empty() {
            let parts: Vec<&str> = trimmed_split.split_whitespace().collect();
            if let Some(&s_flag) = parts.first() {
                state.split_enabled = s_flag == "1";
            }
            let tx_vfo_line = self.read_line_internal().await?;
            let trimmed_tx_vfo = tx_vfo_line.trim();
            if !trimmed_tx_vfo.is_empty() && !trimmed_tx_vfo.starts_with("RPRT") {
                state.tx_vfo = trimmed_tx_vfo.to_string();
            }
        }

        // 5. Moc RF ('l RFPOWER')
        self.write_cmd_internal("l RFPOWER\n").await?;
        let pwr_line = self.read_line_internal().await?;
        let trimmed_pwr = pwr_line.trim();
        if !trimmed_pwr.starts_with("RPRT") && !trimmed_pwr.is_empty() {
            if let Ok(pwr) = trimmed_pwr.parse::<f32>() {
                state.rf_power_watts = pwr * 100.0;
            }
        }

        // 6. Stan PTT ('t')
        self.write_cmd_internal("t\n").await?;
        let ptt_line = self.read_line_internal().await?;
        let trimmed_ptt = ptt_line.trim();
        if !trimmed_ptt.starts_with("RPRT") && !trimmed_ptt.is_empty() {
            state.ptt = trimmed_ptt == "1";
        }

        Ok(state)
    }
}

/// Parsuje odpowiedź `RPRT <code>` demona rigctld.
fn parse_rprt(line: &str) -> Result<(), RigError> {
    let trimmed = line.trim();
    if !trimmed.starts_with("RPRT") {
        return Err(RigError::Protocol {
            detail: format!("expected RPRT response, got: '{trimmed}'"),
        });
    }
    let code_str = trimmed.strip_prefix("RPRT").unwrap_or("").trim();
    if code_str.is_empty() {
        return Err(RigError::Protocol {
            detail: "missing status code in RPRT response".to_string(),
        });
    }
    match code_str.parse::<i32>() {
        Ok(0) => Ok(()),
        Ok(code) => Err(RigError::Rejected { code }),
        Err(_) => Err(RigError::Protocol {
            detail: format!("invalid numeric status code in RPRT response: '{code_str}'"),
        }),
    }
}

impl RigBackend for HamlibBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::Hamlib
    }

    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities::ALL
    }

    fn connect(&mut self) -> impl Future<Output = Result<(), RigError>> + Send {
        self.connect_internal()
    }

    fn poll(&mut self) -> impl Future<Output = Result<RigState, RigError>> + Send {
        self.poll_internal()
    }

    fn execute(
        &mut self,
        command: &RigCommand,
    ) -> impl Future<Output = Result<(), RigError>> + Send {
        self.execute_cmd(command)
    }

    fn disconnect(&mut self) -> impl Future<Output = ()> + Send {
        self.disconnect_internal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_str_to_s_unit_characteristics() {
        let cases: &[(f32, &str)] = &[
            (0.0, "S9+00dB"),
            (10.0, "S9+10dB"),
            (20.0, "S9+20dB"),
            (25.4, "S9+25dB"),
            (-6.0, "S8"),
            (-12.0, "S7"),
            (-54.0, "S0"),
            (-60.0, "S0"),
        ];

        for &(input, expected) in cases {
            let actual = HamlibClient::raw_str_to_s_unit(input);
            assert_eq!(
                actual, expected,
                "raw_str_to_s_unit({input}) expected {expected}, got {actual}"
            );
        }
    }

    async fn run_mock_server<F, Fut>(handler: F) -> (u16, tokio::task::JoinHandle<()>)
    where
        F: FnOnce(tokio::net::TcpStream) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock listener");
        let port = listener.local_addr().expect("local addr").port();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept connection");
            handler(stream).await;
        });
        (port, handle)
    }

    #[tokio::test]
    async fn hamlib_poll_parses_all_fields() {
        let (port, server_task) = run_mock_server(|stream| async move {
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            // 1. f
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "f");
            line.clear();
            writer.write_all(b"14074000\n").await.unwrap();

            // 2. m
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "m");
            line.clear();
            writer.write_all(b"USB\n2400\n").await.unwrap();

            // 3. l STRENGTH
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "l STRENGTH");
            line.clear();
            writer.write_all(b"10.0\n").await.unwrap();

            // 4. s
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "s");
            line.clear();
            writer.write_all(b"1\nVFOB\n").await.unwrap();

            // 5. l RFPOWER
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "l RFPOWER");
            line.clear();
            writer.write_all(b"0.5\n").await.unwrap();

            // 6. t
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "t");
            line.clear();
            writer.write_all(b"1\n").await.unwrap();
        })
        .await;

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();
        let state = backend.poll().await.unwrap();
        backend.disconnect().await;
        server_task.await.unwrap();

        assert_eq!(state.frequency_hz, 14_074_000);
        assert_eq!(state.mode, "USB");
        assert_eq!(state.passband_hz, 2400);
        assert_eq!(state.s_meter_dbm, 10.0);
        assert_eq!(state.s_meter_unit, "S9+10dB");
        assert!(state.split_enabled);
        assert_eq!(state.tx_vfo, "VFOB");
        assert!((state.rf_power_watts - 50.0).abs() < 0.01);
        assert!(state.ptt);
        assert!(state.connected);
    }

    #[tokio::test]
    async fn hamlib_setter_ok() {
        let (port, server_task) = run_mock_server(|stream| async move {
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "F 7074000");
            writer.write_all(b"RPRT 0\n").await.unwrap();
        })
        .await;

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();
        let res = backend.execute(&RigCommand::SetFrequency(7_074_000)).await;
        backend.disconnect().await;
        server_task.await.unwrap();

        assert_eq!(res, Ok(()));
    }

    #[tokio::test]
    async fn hamlib_setter_rprt_error_is_rejected() {
        let (port, server_task) = run_mock_server(|stream| async move {
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "T 1");
            writer.write_all(b"RPRT -1\n").await.unwrap();
        })
        .await;

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();
        let res = backend.execute(&RigCommand::SetPtt(true)).await;
        backend.disconnect().await;
        server_task.await.unwrap();

        assert_eq!(res, Err(RigError::Rejected { code: -1 }));
    }

    #[tokio::test]
    async fn hamlib_set_passband_uses_last_mode() {
        let (port, server_task) = run_mock_server(|stream| async move {
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            // 1. SetMode CW 500
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "M CW 500");
            line.clear();
            writer.write_all(b"RPRT 0\n").await.unwrap();

            // 2. SetPassband 250 -> M CW 250
            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "M CW 250");
            writer.write_all(b"RPRT 0\n").await.unwrap();
        })
        .await;

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();
        backend
            .execute(&RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            })
            .await
            .unwrap();
        let res = backend.execute(&RigCommand::SetPassband(250)).await;
        backend.disconnect().await;
        server_task.await.unwrap();

        assert_eq!(res, Ok(()));
    }

    #[tokio::test]
    async fn hamlib_connection_closed_is_error() {
        let (port, server_task) = run_mock_server(|stream| async move {
            drop(stream);
        })
        .await;

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();
        let res = backend.execute(&RigCommand::SetFrequency(14_000_000)).await;
        backend.disconnect().await;
        server_task.await.unwrap();

        assert!(matches!(
            res,
            Err(RigError::Protocol { .. }) | Err(RigError::Connect { .. })
        ));
    }

    #[tokio::test]
    async fn hamlib_reuses_single_connection_for_poll_and_execute() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            for cmd in ["f", "m", "l STRENGTH", "s", "l RFPOWER", "t"] {
                buf_reader.read_line(&mut line).await.unwrap();
                assert_eq!(line.trim(), cmd);
                line.clear();
                match cmd {
                    "f" => writer.write_all(b"14000000\n").await.unwrap(),
                    "m" => writer.write_all(b"CW\n500\n").await.unwrap(),
                    "l STRENGTH" => writer.write_all(b"0.0\n").await.unwrap(),
                    "s" => writer.write_all(b"0\nVFOA\n").await.unwrap(),
                    "l RFPOWER" => writer.write_all(b"1.0\n").await.unwrap(),
                    "t" => writer.write_all(b"0\n").await.unwrap(),
                    _ => unreachable!(),
                }
            }

            buf_reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "F 14050000");
            line.clear();
            writer.write_all(b"RPRT 0\n").await.unwrap();

            for cmd in ["f", "m", "l STRENGTH", "s", "l RFPOWER", "t"] {
                buf_reader.read_line(&mut line).await.unwrap();
                assert_eq!(line.trim(), cmd);
                line.clear();
                match cmd {
                    "f" => writer.write_all(b"14050000\n").await.unwrap(),
                    "m" => writer.write_all(b"CW\n500\n").await.unwrap(),
                    "l STRENGTH" => writer.write_all(b"0.0\n").await.unwrap(),
                    "s" => writer.write_all(b"0\nVFOA\n").await.unwrap(),
                    "l RFPOWER" => writer.write_all(b"1.0\n").await.unwrap(),
                    "t" => writer.write_all(b"0\n").await.unwrap(),
                    _ => unreachable!(),
                }
            }
        });

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();

        let state1 = backend.poll().await.unwrap();
        assert_eq!(state1.frequency_hz, 14_000_000);

        backend
            .execute(&RigCommand::SetFrequency(14_050_000))
            .await
            .unwrap();

        let state2 = backend.poll().await.unwrap();
        assert_eq!(state2.frequency_hz, 14_050_000);

        backend.disconnect().await;
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn hamlib_execute_command_matrix() {
        let expected_cmds = vec![
            ("F 14074000\n", RigCommand::SetFrequency(14_074_000)),
            (
                "\\set_split_freq 14075000\n",
                RigCommand::SetSplitFrequency(14_075_000),
            ),
            (
                "M USB 2400\n",
                RigCommand::SetMode {
                    mode: "USB".to_string(),
                    passband_hz: 2400,
                },
            ),
            ("M USB 1800\n", RigCommand::SetPassband(1800)),
            (
                "S 1 VFOB\n",
                RigCommand::SetSplit {
                    enabled: true,
                    tx_vfo: "VFOB".to_string(),
                },
            ),
            (
                "S 0 VFOA\n",
                RigCommand::SetSplit {
                    enabled: false,
                    tx_vfo: "VFOA".to_string(),
                },
            ),
            ("T 1\n", RigCommand::SetPtt(true)),
            ("T 0\n", RigCommand::SetPtt(false)),
            ("V VFOA\n", RigCommand::SetVfo("VFOA".to_string())),
            ("J 50\n", RigCommand::SetRit(50)),
            ("J -50\n", RigCommand::SetRit(-50)),
            ("Z 100\n", RigCommand::SetXit(100)),
            ("L RFPOWER 0.5000\n", RigCommand::SetPower(50.0)),
            ("L RFPOWER 1.0000\n", RigCommand::SetPower(150.0)),
        ];

        let server_expected = expected_cmds
            .iter()
            .map(|(cmd, _)| cmd.to_string())
            .collect::<Vec<_>>();

        let (port, server_task) = run_mock_server(move |stream| async move {
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            for expected in server_expected {
                buf_reader.read_line(&mut line).await.unwrap();
                assert_eq!(line, expected);
                line.clear();
                writer.write_all(b"RPRT 0\n").await.unwrap();
            }
        })
        .await;

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();

        for (_, cmd) in expected_cmds {
            backend.execute(&cmd).await.unwrap();
        }

        backend.disconnect().await;
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn hamlib_not_connected_errors() {
        let mut backend = HamlibBackend::new("127.0.0.1", 4532);
        assert_eq!(backend.poll().await, Err(RigError::NotConnected));
        assert_eq!(
            backend.execute(&RigCommand::SetFrequency(14_000_000)).await,
            Err(RigError::NotConnected)
        );
    }

    #[tokio::test]
    async fn hamlib_set_passband_without_mode_is_error() {
        let (port, server_task) = run_mock_server(|_stream| async move {}).await;
        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();

        let res = backend.execute(&RigCommand::SetPassband(500)).await;
        assert!(matches!(res, Err(RigError::Protocol { .. })));

        backend.disconnect().await;
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn hamlib_protocol_errors() {
        let (port, server_task) = run_mock_server(|stream| async move {
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            buf_reader.read_line(&mut line).await.unwrap();
            line.clear();
            writer.write_all(b"SOMETHING_UNEXPECTED\n").await.unwrap();

            buf_reader.read_line(&mut line).await.unwrap();
            line.clear();
            writer.write_all(b"RPRT NaN\n").await.unwrap();
        })
        .await;

        let mut backend = HamlibBackend::new("127.0.0.1", port);
        backend.connect().await.unwrap();

        let res1 = backend.execute(&RigCommand::SetFrequency(14_000_000)).await;
        assert!(matches!(res1, Err(RigError::Protocol { .. })));

        let res2 = backend.execute(&RigCommand::SetFrequency(14_000_000)).await;
        assert!(matches!(res2, Err(RigError::Protocol { .. })));

        backend.disconnect().await;
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn hamlib_disconnect_is_idempotent() {
        let mut backend = HamlibBackend::new("127.0.0.1", 4532);
        backend.disconnect().await;
        backend.disconnect().await;
        assert!(!backend.is_connected());
    }

    #[tokio::test]
    async fn hamlib_connect_error_unreachable_endpoint() {
        let mut backend = HamlibBackend::with_timeout("127.0.0.1", 1, Duration::from_millis(50));
        let res = backend.connect().await;
        assert!(matches!(
            res,
            Err(RigError::Connect { .. }) | Err(RigError::Timeout { .. })
        ));
    }

    #[tokio::test]
    async fn hamlib_kind_and_capabilities() {
        let backend = HamlibBackend::new("127.0.0.1", 4532);
        assert_eq!(backend.kind(), CatBackendKind::Hamlib);
        assert_eq!(backend.capabilities(), RigCapabilities::ALL);
    }
}
