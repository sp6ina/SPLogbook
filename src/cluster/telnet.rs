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
    ("Polska - DXCluster.pl (SP)", "dxcluster.pl", 8000),
    ("Europa - DXFun (ES)", "dxfun.com", 8000),
    ("Europa - DB0SUE (DL)", "db0sue.de", 8000),
    ("Europa - PI4CC (PA)", "dxc.pi4cc.nl", 8000),
    ("Europa - DXSpider (G)", "dxspider.co.uk", 7300),
    ("Europa - OK0DXI (OK)", "ok0dxi.nagano.cz", 41112),
    ("Europa - EA4URE (ES)", "ea4ure.com", 7300),
    ("Ameryka - W3LPL (USA)", "w3lpl.net", 7373),
    ("Ameryka - NC7J (USA)", "dxc.nc7j.com", 7373),
    ("Ameryka - K3LR (USA)", "dx.k3lr.com", 23),
];

/// Sprawdza czy host należy do historycznych, nieaktywnych już serwerów domyślnych.
pub fn is_dead_legacy_cluster_host(host: &str) -> bool {
    let h = host.trim().to_ascii_lowercase();
    h.is_empty()
        || h == "cluster.sp7pka.ampr.org"
        || h == "sr5dxc.ampr.org"
        || h == "gb7dxm.shacknet.nu"
        || h == "ve7cc.net"
}

/// Normalizuje znak wywoławczy do logowania w klastrze Telnet (węzły DXSpider/AR-Cluster
/// odrzucają pusty znak, "N0CALL" oraz czasem sufiksy łamane `/P`).
pub fn sanitize_login_call(raw_call: &str) -> String {
    let trimmed = raw_call.trim().to_ascii_uppercase();
    if trimmed.is_empty() || trimmed == "N0CALL" || trimmed == "NOCALL" {
        return "SP0LOG".to_string();
    }
    let base = trimmed
        .split('/')
        .max_by_key(|part| part.len())
        .unwrap_or(&trimmed);
    let clean: String = base
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if clean.len() >= 3 && clean.chars().any(|c| c.is_ascii_digit()) {
        clean
    } else {
        "SP0LOG".to_string()
    }
}

/// Usuwa bajty negocjacji Telnet IAC (0xFF + 2 bajty) oraz znaki sterujące BELL (0x07).
fn strip_telnet_iac(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0xFF {
            if i + 1 < bytes.len() {
                let cmd = bytes[i + 1];
                if (251..=254).contains(&cmd) {
                    i += 3;
                    continue;
                }
                i += 2;
                continue;
            }
            break;
        }
        if bytes[i] != 0x07 && bytes[i] != 0x00 {
            out.push(bytes[i]);
        }
        i += 1;
    }
    out
}

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
        stop_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        Self::run_with_events_and_commands(host, port, my_call, event_tx, stop_rx, None).await;
    }

    /// Uruchamia pętlę nasłuchu ze sterowaniem zatrzymaniem, wysyłaniem zdarzeń do kanału mpsc
    /// oraz opcjonalnym kanałem wychodzących komend Telnet (np. `DX <freq> <call> <comment>`).
    pub async fn run_with_events_and_commands(
        host: String,
        port: u16,
        my_call: String,
        event_tx: std::sync::mpsc::Sender<ClusterEvent>,
        mut stop_rx: tokio::sync::watch::Receiver<bool>,
        mut cmd_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    ) {
        let (effective_host, effective_port) = if is_dead_legacy_cluster_host(&host) {
            ("dxcluster.pl".to_string(), 8000)
        } else {
            (host.trim().to_string(), if port == 0 { 8000 } else { port })
        };
        let addr = format!("{effective_host}:{effective_port}");
        let login_call = sanitize_login_call(&my_call);

        while !*stop_rx.borrow() {
            let connect_result = tokio::select! {
                res = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    TcpStream::connect(&addr),
                ) => Some(res),
                _ = stop_rx.changed() => None,
            };

            let stream = match connect_result {
                Some(Ok(Ok(s))) => s,
                Some(Ok(Err(e))) => {
                    if event_tx
                        .send(ClusterEvent::Disconnected(format!(
                            "Connection error ({addr}): {e}"
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
                Some(Err(_)) => {
                    if event_tx
                        .send(ClusterEvent::Disconnected(format!(
                            "Connection timeout ({addr}, 10s)"
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
                    "Connected to DX Cluster: {addr} (login: {login_call})"
                )))
                .is_err()
            {
                break;
            }

            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);

            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
            if *stop_rx.borrow() {
                break;
            }
            let _ = writer
                .write_all(format!("{login_call}\r\n").as_bytes())
                .await;
            let _ = writer.flush().await;

            // Po krótkiej chwili pobierz ostatnie 25 spotów, aby tabela od razu się zapełniła
            let mut sent_initial_sh_dx = false;
            let mut raw_line = Vec::with_capacity(256);

            loop {
                if *stop_rx.borrow() {
                    break;
                }

                if !sent_initial_sh_dx {
                    sent_initial_sh_dx = true;
                    let _ = writer.write_all(b"sh/dx 25\r\n").await;
                    let _ = writer.flush().await;
                }

                raw_line.clear();
                let mut limited = (&mut buf_reader).take((MAX_LINE_LEN + 1) as u64);

                enum LoopAction {
                    Read(Result<std::io::Result<usize>, tokio::time::error::Elapsed>),
                    Command(String),
                    Stop,
                }

                let action = tokio::select! {
                    res = tokio::time::timeout(
                        std::time::Duration::from_secs(300),
                        limited.read_until(b'\n', &mut raw_line),
                    ) => LoopAction::Read(res),
                    Some(cmd) = async {
                        match cmd_rx.as_mut() {
                            Some(rx) => rx.recv().await,
                            None => std::future::pending().await,
                        }
                    } => LoopAction::Command(cmd),
                    _ = stop_rx.changed() => LoopAction::Stop,
                };

                let read_res = match action {
                    LoopAction::Stop => break,
                    LoopAction::Command(cmd) => {
                        let formatted = if cmd.ends_with("\r\n") {
                            cmd
                        } else {
                            format!("{}\r\n", cmd.trim_end_matches(['\r', '\n']))
                        };
                        if let Err(e) = writer.write_all(formatted.as_bytes()).await {
                            let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                                "Write error ({addr}): {e}"
                            )));
                            break;
                        }
                        let _ = writer.flush().await;
                        continue;
                    }
                    LoopAction::Read(Ok(res)) => res,
                    LoopAction::Read(Err(_)) => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                            "Read timeout ({addr}, 300s)"
                        )));
                        break;
                    }
                };

                match read_res {
                    Ok(0) => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                            "Disconnected by server {addr}"
                        )));
                        break;
                    }
                    Ok(_) if raw_line.len() > MAX_LINE_LEN => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                            "Server {addr} sent line exceeding {MAX_LINE_LEN} B, disconnected."
                        )));
                        break;
                    }
                    Ok(_) => {
                        let cleaned = strip_telnet_iac(&raw_line);
                        let line_cow = String::from_utf8_lossy(&cleaned);
                        let trimmed = line_cow.trim();
                        if !trimmed.is_empty() {
                            let lower = trimmed.to_ascii_lowercase();
                            if lower.ends_with("login:")
                                || lower.ends_with("call:")
                                || lower.ends_with("callsign:")
                            {
                                let _ = writer
                                    .write_all(format!("{login_call}\r\n").as_bytes())
                                    .await;
                                let _ = writer.flush().await;
                            }
                            if event_tx
                                .send(ClusterEvent::RawLine(trimmed.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                        if let Some(spot) = parse_dx_spot(trimmed) {
                            if event_tx.send(ClusterEvent::Spot(spot)).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!(
                            "Transmission error ({addr}): {e}"
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
            "Disconnected from DX Cluster.".to_string(),
        ));
    }

    /// Łączy się z klastrem DX i transmituje odebrane spoty przez kanał broadcast
    pub async fn run(&self) {
        let addr = format!("{}:{}", self.host, self.port);
        let login_call = sanitize_login_call(&self.my_call);

        loop {
            if let Ok(Ok(stream)) = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                TcpStream::connect(&addr),
            )
            .await
            {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);

                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                let _ = writer
                    .write_all(format!("{login_call}\r\n").as_bytes())
                    .await;

                let mut raw_line = Vec::with_capacity(256);
                loop {
                    let mut limited = (&mut buf_reader).take((MAX_LINE_LEN + 1) as u64);
                    let Ok(Ok(n)) = tokio::time::timeout(
                        std::time::Duration::from_secs(300),
                        limited.read_until(b'\n', &mut raw_line),
                    )
                    .await
                    else {
                        break;
                    };
                    if n == 0 {
                        break;
                    }
                    if raw_line.len() > MAX_LINE_LEN {
                        raw_line.clear();
                        break;
                    }
                    let cleaned = strip_telnet_iac(&raw_line);
                    let line_cow = String::from_utf8_lossy(&cleaned);
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

/// Zwraca skompilowane wyrażenie regularne dla linii spotów DX z sufiksem `Z` (raz, współdzielone).
fn spot_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^DX de\s+([A-Z0-9/\-#]+):\s+([0-9.]+)\s+([A-Z0-9/]+)\s+(.*)\s+([0-9]{4})Z\s*$")
            .expect("spot regex musi być poprawny")
    })
}

/// Zwraca zapasowe wyrażenie regularne dla linii spotów DX, w których pominięto literę `Z` na końcu.
fn spot_regex_no_z() -> &'static Regex {
    static RE_NO_Z: OnceLock<Regex> = OnceLock::new();
    RE_NO_Z.get_or_init(|| {
        Regex::new(r"(?i)^DX de\s+([A-Z0-9/\-#]+):\s+([0-9.]+)\s+([A-Z0-9/]+)\s+(.*)\s+([0-9]{4})\s*$")
            .expect("spot fallback regex musi być poprawny")
    })
}

/// Wyrażenie regularne dla wierszy historii `sh/dx` zwracanych przez węzły DXSpider / AR-Cluster:
/// np. ` 14074.0  JA1ABC      29-Sep-2026 1420Z  FT8 -10 dB                    <SP6INA>`
fn sh_dx_regex() -> &'static Regex {
    static RE_SH_DX: OnceLock<Regex> = OnceLock::new();
    RE_SH_DX.get_or_init(|| {
        Regex::new(r"(?i)^\s*([0-9]+\.[0-9]+)\s+([A-Z0-9/]+)\s+[0-9]{1,2}-[A-Z]{3}-[0-9]{2,4}\s+([0-9]{4})Z?\s+(.*?)\s*<([A-Z0-9/\-#]+)>\s*$")
            .expect("sh/dx regex musi być poprawny")
    })
}

/// Parsuje pojedynczą linię spotu DX Cluster na `DxSpot`. Zwraca `None` dla linii,
/// które nie są spotami (powitania, informacje serwera, pusta linia itd.).
pub fn parse_dx_spot(line: &str) -> Option<DxSpot> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (spotter, freq_str, dx_call, comment, time_utc) = if let Some(caps) = spot_regex()
        .captures(trimmed)
        .or_else(|| spot_regex_no_z().captures(trimmed))
    {
        (
            caps.get(1)?.as_str().to_uppercase(),
            caps.get(2)?.as_str(),
            caps.get(3)?.as_str().to_uppercase(),
            caps.get(4)?.as_str().trim().to_string(),
            caps.get(5)?.as_str().to_string(),
        )
    } else if let Some(caps) = sh_dx_regex().captures(trimmed) {
        (
            caps.get(5)?.as_str().to_uppercase(),
            caps.get(1)?.as_str(),
            caps.get(2)?.as_str().to_uppercase(),
            caps.get(4)?.as_str().trim().to_string(),
            caps.get(3)?.as_str().to_string(),
        )
    } else {
        return None;
    };

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
    fn test_parse_dx_spot_with_four_digit_number_in_comment() {
        let spot = parse_dx_spot("DX de SP6INA: 7025.0 JA1ABC QSX 7150 UP 1420Z").unwrap();
        assert_eq!(spot.spotter, "SP6INA");
        assert_eq!(spot.dx_call, "JA1ABC");
        assert_eq!(spot.frequency_khz, 7025.0);
        assert_eq!(spot.band, "40m");
        assert_eq!(spot.comment, "QSX 7150 UP");
        assert_eq!(spot.time_utc, "1420");
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
