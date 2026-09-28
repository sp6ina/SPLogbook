// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::time::sleep;

/// Maksymalna długość pojedynczej linii odpowiedzi rigctld (wartości numeryczne/statusy).
const MAX_LINE_LEN: usize = 256;
/// Maksymalny czas oczekiwania na połączenie TCP z lokalnym lub zdalnym demonem rigctld.
const CAT_TCP_TIMEOUT: Duration = Duration::from_millis(1500);

async fn connect_timeout(host: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    match tokio::time::timeout(
        CAT_TCP_TIMEOUT,
        TcpStream::connect(format!("{host}:{port}")),
    )
    .await
    {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Przekroczono czas oczekiwania na połączenie z rigctld",
        )),
    }
}

async fn write_cmd<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    cmd: &[u8],
) -> Result<(), std::io::Error> {
    match tokio::time::timeout(CAT_TCP_TIMEOUT, writer.write_all(cmd)).await {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Przekroczono czas zapisu do rigctld",
        )),
    }
}

async fn read_line_timeout<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
    line: &mut String,
) -> Result<usize, std::io::Error> {
    line.clear();
    let mut limited = reader.take((MAX_LINE_LEN + 1) as u64);
    match tokio::time::timeout(CAT_TCP_TIMEOUT, limited.read_line(line)).await {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Przekroczono czas odczytu z rigctld",
        )),
    }
}

async fn send_setter_cmd(host: &str, port: u16, cmd: &str) -> Result<(), std::io::Error> {
    let stream = connect_timeout(host, port).await?;
    let (reader, mut writer) = stream.into_split();
    write_cmd(&mut writer, cmd.as_bytes()).await?;
    let mut buf_reader = BufReader::new(reader);
    let mut resp = String::new();
    let _ = tokio::time::timeout(
        Duration::from_millis(500),
        (&mut buf_reader)
            .take((MAX_LINE_LEN + 1) as u64)
            .read_line(&mut resp),
    )
    .await;
    Ok(())
}

/// Pełny stan transceivera zgodny z protokołem Hamlib 4.6+ (rigctld)
#[derive(Debug, Clone, PartialEq)]
pub struct RigState {
    pub frequency_hz: u64,
    pub mode: String,
    pub passband_hz: u32,
    pub vfo: String,    // np. "VFOA", "VFOB", "Main", "Sub"
    pub tx_vfo: String, // VFO używane do nadawania w trybie Split
    pub split_enabled: bool,
    pub tx_frequency_hz: Option<u64>, // Częstotliwość nadawania w trybie Split
    pub s_meter_dbm: f32,             // Sygnał S-Metra w dBm (lub S9+dB)
    pub s_meter_unit: String,         // np. "S9+10dB" lub "S7"
    pub rf_power_watts: f32,          // Moc wyjściowa w Watach
    pub ptt: bool,
    pub rit_hz: i32, // Przesunięcie RIT w Hz
    pub xit_hz: i32, // Przesunięcie XIT w Hz
    pub connected: bool,
}

impl Default for RigState {
    fn default() -> Self {
        Self {
            frequency_hz: 14_025_000,
            mode: "CW".to_string(),
            passband_hz: 500,
            vfo: "VFOA".to_string(),
            tx_vfo: "VFOA".to_string(),
            split_enabled: false,
            tx_frequency_hz: None,
            s_meter_dbm: -100.0,
            s_meter_unit: "S1".to_string(),
            rf_power_watts: 0.0,
            ptt: false,
            rit_hz: 0,
            xit_hz: 0,
            connected: false,
        }
    }
}

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
                                if !tx_vfo_trimmed.is_empty()
                                    && !tx_vfo_trimmed.starts_with("RPRT")
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
                            current_state.rf_power_watts = pwr * 100.0; // Proporcja mocy
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
