// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

/// Maksymalna długość pojedynczej linii tekstowej odbieranej z serwera DX Cluster
/// (Telnet). Chroni przed wyczerpaniem pamięci, gdyby serwer (lub ktoś
/// podszywający się pod niego / MITM) wysłał dane bez znaku nowej linii.
const MAX_LINE_LEN: usize = 16 * 1024;

/// Pojedynczy spot radiowy z DX Cluster z flagami FT8/Skimmer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DxSpot {
    pub spotter: String,
    pub frequency_khz: f64,
    pub dx_call: String,
    pub comment: String,
    pub time_utc: String,
    pub band: String,
    pub is_ft8: bool,
    pub is_skimmer: bool,
    /// Czas odebrania spotu jako uniksowy znacznik (sekundy), używany do pokazywania wieku
    /// oraz do usuwania przestarzałych spotów.
    #[serde(default)]
    pub received_at: i64,
}

#[derive(Debug, Clone)]
pub enum ClusterEvent {
    Connected(String),
    Disconnected(String),
    Spot(DxSpot),
    RawLine(String),
}

pub const CLUSTER_PRESETS: &[(&str, &str, u16)] = &[
    ("Polska - SP7PKA (Łódź)", "cluster.sp7pka.ampr.org", 8000),
    ("Polska - SR5DXC (Warszawa)", "sr5dxc.ampr.org", 8000),
    ("Europa - DXFun (Hiszpania)", "dxfun.com", 8000),
    ("Europa - DB0SUE (Niemcy)", "db0sue.de", 8000),
    (
        "Europa - GB7DXM (Wielka Brytania)",
        "gb7dxm.shacknet.nu",
        7300,
    ),
    ("Ameryka - VE7CC (Kanada)", "ve7cc.net", 23),
    ("Ameryka - W3LPL (USA)", "w3lpl.net", 7373),
];

/// Asynchroniczny klient do węzłów DX Cluster (Telnet) z pętlą automatycznego wznawiania (Auto-Reconnect)
pub struct DxClusterClient {
    host: String,
    port: u16,
    my_call: String,
    spot_sender: broadcast::Sender<DxSpot>,
}

impl DxClusterClient {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        my_call: impl Into<String>,
    ) -> (Self, broadcast::Receiver<DxSpot>) {
        let (tx, rx) = broadcast::channel(256);
        (
            Self {
                host: host.into(),
                port,
                my_call: my_call.into(),
                spot_sender: tx,
            },
            rx,
        )
    }

    /// Uruchamia pętlę nasłuchu ze sterowaniem zatrzymaniem i wysyłaniem zdarzeń do kanału mpsc.
    /// Zabezpieczone przed powstawaniem wątków zombie: jeśli event_tx.send() zwróci Err, pętla natychmiast się kończy.
    pub async fn run_with_events(
        host: String,
        port: u16,
        my_call: String,
        event_tx: std::sync::mpsc::Sender<ClusterEvent>,
        mut stop_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        let addr = format!("{host}:{port}");

        while !*stop_rx.borrow() {
            let connect_result = tokio::select! {
                res = TcpStream::connect(&addr) => Some(res),
                _ = stop_rx.changed() => None,
            };

            let stream = match connect_result {
                Some(Ok(s)) => s,
                Some(Err(e)) => {
                    if event_tx
                        .send(ClusterEvent::Disconnected(format!(
                            "Błąd połączenia z {addr}: {e}"
                        )))
                        .is_err()
                    {
                        break;
                    }
                    tokio::select! {
                        () = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                        _ = stop_rx.changed() => break,
                    }
                    continue;
                }
                None => break,
            };

            if event_tx
                .send(ClusterEvent::Connected(format!(
                    "Połączono z serwerem: {addr}"
                )))
                .is_err()
            {
                break;
            }

            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);

            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            if *stop_rx.borrow() {
                break;
            }
            let _ = writer.write_all(format!("{my_call}\n").as_bytes()).await;

            let mut raw_line = Vec::with_capacity(256);
            loop {
                if *stop_rx.borrow() {
                    break;
                }

                raw_line.clear();
                let mut limited = (&mut buf_reader).take((MAX_LINE_LEN + 1) as u64);
                let read_res = tokio::select! {
                    res = limited.read_until(b'\n', &mut raw_line) => res,
                    _ = stop_rx.changed() => break,
                };

                match read_res {
                    Ok(0) => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                            "Rozłączono przez serwer {addr}"
                        )));
                        break;
                    }
                    Ok(_) if raw_line.len() > MAX_LINE_LEN => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                            "Serwer {addr} wysłał zbyt długą linię (>{MAX_LINE_LEN} B), rozłączono."
                        )));
                        break;
                    }
                    Ok(_) => {
                        let line_cow = String::from_utf8_lossy(&raw_line);
                        let trimmed = line_cow.trim();
                        if !trimmed.is_empty()
                            && event_tx
                                .send(ClusterEvent::RawLine(trimmed.to_string()))
                                .is_err()
                        {
                            break;
                        }
                        if let Some(spot) = parse_dx_spot(trimmed) {
                            if event_tx.send(ClusterEvent::Spot(spot)).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                            "Błąd transmisji z {addr}: {e}"
                        )));
                        break;
                    }
                }
            }

            if *stop_rx.borrow() {
                break;
            }

            // Oczekiwanie przed ponowną próbą
            tokio::select! {
                () = tokio::time::sleep(std::time::Duration::from_secs(4)) => {}
                _ = stop_rx.changed() => break,
            }
        }

        let _ = event_tx.send(ClusterEvent::Disconnected(
            "Rozłączono z klastrem DX.".to_string(),
        ));
    }

    /// Łączy się z klastrem DX i transmituje odebrane spoty przez kanał broadcast
    pub async fn run(&self) {
        let addr = format!("{}:{}", self.host, self.port);

        loop {
            if let Ok(stream) = TcpStream::connect(&addr).await {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let _ = writer
                    .write_all(format!("{}\n", self.my_call).as_bytes())
                    .await;

                let mut raw_line = Vec::with_capacity(256);
                while let Ok(n) = (&mut buf_reader)
                    .take((MAX_LINE_LEN + 1) as u64)
                    .read_until(b'\n', &mut raw_line)
                    .await
                {
                    if n == 0 {
                        break;
                    }
                    if raw_line.len() > MAX_LINE_LEN {
                        raw_line.clear();
                        break;
                    }
                    let line_cow = String::from_utf8_lossy(&raw_line);
                    let trimmed = line_cow.trim();
                    if let Some(spot) = parse_dx_spot(trimmed) {
                        let _ = self.spot_sender.send(spot);
                    }
                    raw_line.clear();
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

/// Zwraca skompilowane wyrażenie regularne dla linii spotów DX (raz, współdzielone).
fn spot_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^DX de\s+([A-Z0-9/\-#]+):\s+([0-9.]+)\s+([A-Z0-9/]+)\s+(.*?)\s+([0-9]{4})Z?")
            .expect("spot regex musi być poprawny")
    })
}

/// Parsuje pojedynczą linię spotu DX Cluster na `DxSpot`. Zwraca `None` dla linii,
/// które nie są spotami (powitania, informacje serwera, pusta linia itd.).
pub fn parse_dx_spot(line: &str) -> Option<DxSpot> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let caps = spot_regex().captures(trimmed)?;

    let spotter = caps.get(1)?.as_str().to_string();
    let freq_str = caps.get(2)?.as_str();
    let dx_call = caps.get(3)?.as_str().to_string();
    let comment = caps.get(4)?.as_str().trim().to_string();
    let time_utc = caps.get(5)?.as_str().to_string();

    let frequency_khz: f64 = freq_str.parse().unwrap_or(0.0);
    let band = band_for_freq_khz(frequency_khz);

    let comment_upper = comment.to_uppercase();
    let is_ft8 = comment_upper.contains("FT8")
        || comment_upper.contains("FT4")
        || comment_upper.contains("JS8");
    let is_skimmer =
        spotter.contains("-#") || comment_upper.contains("BPS") || comment_upper.contains("WPM");

    Some(DxSpot {
        spotter,
        frequency_khz,
        dx_call,
        comment,
        time_utc,
        band,
        is_ft8,
        is_skimmer,
        received_at: chrono::Utc::now().timestamp(),
    })
}

/// Mapuje częstotliwość (w kHz) na nazwę pasma amatorskiego z pełnego bandplanu IARU.
pub fn band_for_freq_khz(khz: f64) -> String {
    crate::core::bandplan::freq_khz_to_band(khz)
        .unwrap_or("OTHER")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dx_spot_standard() {
        let spot = parse_dx_spot("DX de SP7PKA: 14074.0  K1ABC     FT8 -12 dB  1415Z").unwrap();
        assert_eq!(spot.dx_call, "K1ABC");
        assert_eq!(spot.frequency_khz, 14074.0);
        assert_eq!(spot.band, "20m");
        assert!(spot.is_ft8);
        assert!(!spot.is_skimmer);
    }

    #[test]
    fn test_parse_dx_spot_skimmer() {
        let spot = parse_dx_spot("DX de SK1MMR-#: 21074.0  DL1ABC     CW 25 dB  0800Z").unwrap();
        assert!(spot.is_skimmer);
        assert_eq!(spot.band, "15m");
    }

    #[test]
    fn test_parse_dx_spot_ignores_non_spot() {
        assert!(parse_dx_spot("Please enter your callsign: ").is_none());
        assert!(parse_dx_spot("").is_none());
        assert!(parse_dx_spot("Hello from DX cluster").is_none());
    }

    #[test]
    fn test_parse_dx_spot_lossy_non_utf8() {
        // Symulacja surowego strumienia z węzła DXSpider w kodowaniu Latin-1 (np. 'ü' = 0xfc)
        let raw_bytes: &[u8] =
            b"DX de DL1ABC: 14195.0  SP6INA    Gr\xfc\xdfe aus M\xfcnchen  1420Z\r\n";
        let lossy = String::from_utf8_lossy(raw_bytes);
        let spot = parse_dx_spot(lossy.trim())
            .expect("Spot ze znakami spoza UTF-8 powinien zostać sparsowany");
        assert_eq!(spot.dx_call, "SP6INA");
        assert_eq!(spot.band, "20m");
        assert!(spot.comment.contains("Gr"));
    }

    #[test]
    fn test_band_for_freq_khz_boundaries() {
        assert_eq!(band_for_freq_khz(1800.0), "160m");
        assert_eq!(band_for_freq_khz(2000.0), "160m");
        assert_eq!(band_for_freq_khz(2001.0), "OTHER");
        assert_eq!(band_for_freq_khz(14000.0), "20m");
        assert_eq!(band_for_freq_khz(14350.0), "20m");
        assert_eq!(band_for_freq_khz(10_489_500.0), "3cm");
        assert_eq!(band_for_freq_khz(0.0), "OTHER");
    }
}
