// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::time::sleep;

/// Pełny stan transceivera zgodny z protokołem Hamlib 4.6+ (rigctld)
#[derive(Debug, Clone, PartialEq)]
pub struct RigState {
    pub frequency_hz: u64,
    pub mode: String,
    pub passband_hz: u32,
    pub vfo: String,             // np. "VFOA", "VFOB", "Main", "Sub"
    pub split_enabled: bool,
    pub tx_frequency_hz: Option<u64>, // Częstotliwość nadawania w trybie Split
    pub s_meter_dbm: f32,        // Sygnał S-Metra w dBm (lub S9+dB)
    pub s_meter_unit: String,     // np. "S9+10dB" lub "S7"
    pub rf_power_watts: f32,     // Moc wyjściowa w Watach
    pub ptt: bool,
    pub rit_hz: i32,             // Przesunięcie RIT w Hz
    pub xit_hz: i32,             // Przesunięcie XIT w Hz
    pub connected: bool,
}

impl Default for RigState {
    fn default() -> Self {
        Self {
            frequency_hz: 14_025_000,
            mode: "CW".to_string(),
            passband_hz: 500,
            vfo: "VFOA".to_string(),
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

    /// Konwertuje wartość RAW S-metra (dB względem S9) na czytelny format S-unit
    pub fn raw_str_to_s_unit(db: f32) -> String {
        // W standardzie IARU S9 = -73 dBm (dla HF), a każdy stopień S to 6 dB
        if db >= 0.0 {
            format!("S9+{:02.0}dB", db)
        } else {
            let s_val = ((db + 54.0) / 6.0).clamp(0.0, 9.0).round() as u8;
            format!("S{}", s_val)
        }
    }

    /// Uruchamia pętlę ciągłego odpytywania stanu transceivera w tle
    pub async fn run_poll_loop(&self, poll_interval_ms: u64) {
        let addr = format!("{}:{}", self.host, self.port);

        loop {
            match TcpStream::connect(&addr).await {
                Ok(stream) => {
                    let (reader, mut writer) = stream.into_split();
                    let mut buf_reader = BufReader::new(reader);
                    let mut current_state = RigState {
                        connected: true,
                        ..Default::default()
                    };

                    while current_state.connected {
                        // 1. Odpytaj o częstotliwość ('f')
                        if writer.write_all(b"f\n").await.is_err() {
                            break;
                        }
                        let mut line = String::new();
                        if buf_reader.read_line(&mut line).await.is_err() || line.is_empty() {
                            break;
                        }
                        if let Ok(freq) = line.trim().parse::<u64>() {
                            current_state.frequency_hz = freq;
                        }

                        // 2. Odpytaj o emisję i szerokość filtru ('m')
                        if writer.write_all(b"m\n").await.is_err() {
                            break;
                        }
                        line.clear();
                        if buf_reader.read_line(&mut line).await.is_err() || line.is_empty() {
                            break;
                        }
                        current_state.mode = line.trim().to_uppercase();

                        line.clear();
                        if buf_reader.read_line(&mut line).await.is_ok() {
                            if let Ok(pb) = line.trim().parse::<u32>() {
                                current_state.passband_hz = pb;
                            }
                        }

                        // 3. Odpytaj o S-Meter ('l RAWSTR')
                        if writer.write_all(b"l RAWSTR\n").await.is_ok() {
                            line.clear();
                            if buf_reader.read_line(&mut line).await.is_ok() {
                                if let Ok(val) = line.trim().parse::<f32>() {
                                    current_state.s_meter_dbm = val;
                                    current_state.s_meter_unit = Self::raw_str_to_s_unit(val);
                                }
                            }
                        }

                        // 4. Odpytaj o Split ('s')
                        if writer.write_all(b"s\n").await.is_ok() {
                            line.clear();
                            if buf_reader.read_line(&mut line).await.is_ok() {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if let Some(&s_flag) = parts.first() {
                                    current_state.split_enabled = s_flag == "1";
                                }
                            }
                        }

                        // 5. Odpytaj o moc RF ('l RFPOWER')
                        if writer.write_all(b"l RFPOWER\n").await.is_ok() {
                            line.clear();
                            if buf_reader.read_line(&mut line).await.is_ok() {
                                if let Ok(pwr) = line.trim().parse::<f32>() {
                                    current_state.rf_power_watts = pwr * 100.0; // Proporcja mocy
                                }
                            }
                        }

                        // 6. Odpytaj o stan PTT ('t')
                        if writer.write_all(b"t\n").await.is_ok() {
                            line.clear();
                            if buf_reader.read_line(&mut line).await.is_ok() {
                                current_state.ptt = line.trim() == "1";
                            }
                        }

                        let _ = self.state_sender.send(current_state.clone());
                        sleep(Duration::from_millis(poll_interval_ms)).await;
                    }
                }
                Err(_) => {
                    let offline = RigState {
                        connected: false,
                        ..Default::default()
                    };
                    let _ = self.state_sender.send(offline);
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    /// Przestawia częstotliwość radia (w Hz)
    pub async fn set_frequency(host: &str, port: u16, freq_hz: u64) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let cmd = format!("F {}\n", freq_hz);
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Przestawia emisję i filtr radia
    pub async fn set_mode(host: &str, port: u16, mode: &str, passband_hz: u32) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let cmd = format!("M {} {}\n", mode, passband_hz);
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Włącza lub wyłącza tryb Split
    pub async fn set_split(host: &str, port: u16, enabled: bool, tx_vfo: &str) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let cmd = format!("S {} {}\n", if enabled { 1 } else { 0 }, tx_vfo);
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Załącza lub wyłącza PTT (nadawanie)
    pub async fn set_ptt(host: &str, port: u16, ptt: bool) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let cmd = format!("T {}\n", if ptt { 1 } else { 0 });
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Wybiera aktywny VFO (np. "VFOA", "VFOB", "Main", "Sub")
    pub async fn set_vfo(host: &str, port: u16, vfo: &str) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let cmd = format!("V {}\n", vfo);
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Ustawia przesunięcie RIT w Hz (może być ujemne)
    pub async fn set_rit(host: &str, port: u16, rit_hz: i32) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let cmd = format!("J {}\n", rit_hz);
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Ustawia przesunięcie XIT w Hz (może być ujemne)
    pub async fn set_xit(host: &str, port: u16, xit_hz: i32) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let cmd = format!("Z {}\n", xit_hz);
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Ustawia moc wyjściową (0–100 W), wysyłając znormalizowaną wartość 0.0–1.0
    pub async fn set_power(host: &str, port: u16, watts: f32) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let norm = (watts / 100.0).clamp(0.0, 1.0);
        let cmd = format!("L RFPOWER {:.4}\n", norm);
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
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
        TcpStream::connect(&addr)
            .await
            .map(|_| ())
            .map_err(|e| format!("Brak połączenia z rigctld {}: {}", addr, e))
    }

    async fn poll_state(&mut self) -> Result<RigState, String> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("Brak połączenia z rigctld {}: {}", addr, e))?;
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut state = RigState {
            connected: true,
            ..Default::default()
        };

        // Częstotliwość ('f')
        if writer.write_all(b"f\n").await.is_err() {
            return Err("rigctld: brak odpowiedzi (częstotliwość)".to_string());
        }
        let mut line = String::new();
        if buf_reader.read_line(&mut line).await.is_err() || line.is_empty() {
            return Err("rigctld: brak odpowiedzi (częstotliwość)".to_string());
        }
        if let Ok(freq) = line.trim().parse::<u64>() {
            state.frequency_hz = freq;
        }

        // Emisja i szerokość filtru ('m')
        if writer.write_all(b"m\n").await.is_err() {
            return Err("rigctld: brak odpowiedzi (emisja)".to_string());
        }
        line.clear();
        if buf_reader.read_line(&mut line).await.is_ok() {
            state.mode = line.trim().to_uppercase();
        }
        line.clear();
        if buf_reader.read_line(&mut line).await.is_ok() {
            if let Ok(pb) = line.trim().parse::<u32>() {
                state.passband_hz = pb;
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
