// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

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
    ("Europa - GB7DXM (Wielka Brytania)", "gb7dxm.shacknet.nu", 7300),
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
    pub fn new(host: impl Into<String>, port: u16, my_call: impl Into<String>) -> (Self, broadcast::Receiver<DxSpot>) {
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
        let addr = format!("{}:{}", host, port);
        let spot_regex = Regex::new(
            r"^DX de\s+([A-Z0-9/\-#]+):\s+([0-9.]+)\s+([A-Z0-9/]+)\s+(.*?)\s+([0-9]{4})Z?"
        ).unwrap();

        while !*stop_rx.borrow() {
            let connect_result = tokio::select! {
                res = TcpStream::connect(&addr) => Some(res),
                _ = stop_rx.changed() => None,
            };

            let stream = match connect_result {
                Some(Ok(s)) => s,
                Some(Err(e)) => {
                    if event_tx.send(ClusterEvent::Disconnected(format!("Błąd połączenia z {}: {}", addr, e))).is_err() {
                        break;
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
                        _ = stop_rx.changed() => break,
                    }
                    continue;
                }
                None => break,
            };

            if event_tx.send(ClusterEvent::Connected(format!("Połączono z serwerem: {}", addr))).is_err() {
                break;
            }

            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);

            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            if *stop_rx.borrow() {
                break;
            }
            let _ = writer.write_all(format!("{}\n", my_call).as_bytes()).await;

            let mut line = String::new();
            loop {
                if *stop_rx.borrow() {
                    break;
                }

                line.clear();
                let read_res = tokio::select! {
                    res = buf_reader.read_line(&mut line) => res,
                    _ = stop_rx.changed() => break,
                };

                match read_res {
                    Ok(0) => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!("Rozłączono przez serwer {}", addr)));
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            if event_tx.send(ClusterEvent::RawLine(trimmed.to_string())).is_err() {
                                break;
                            }
                        }
                        if let Some(caps) = spot_regex.captures(trimmed) {
                            let spotter = caps.get(1).map_or("", |m| m.as_str()).to_string();
                            let freq_str = caps.get(2).map_or("", |m| m.as_str());
                            let dx_call = caps.get(3).map_or("", |m| m.as_str()).to_string();
                            let comment = caps.get(4).map_or("", |m| m.as_str()).trim().to_string();
                            let time_utc = caps.get(5).map_or("", |m| m.as_str()).to_string();

                            let freq_khz: f64 = freq_str.parse().unwrap_or(0.0);
                            let band = Self::freq_khz_to_band(freq_khz);

                            let comment_upper = comment.to_uppercase();
                            let is_ft8 = comment_upper.contains("FT8") || comment_upper.contains("FT4") || comment_upper.contains("JS8");
                            let is_skimmer = spotter.contains("-#") || comment_upper.contains("BPS") || comment_upper.contains("WPM") || comment_upper.contains("DB");

                            let spot = DxSpot {
                                spotter,
                                frequency_khz: freq_khz,
                                dx_call,
                                comment,
                                time_utc,
                                band,
                                is_ft8,
                                is_skimmer,
                            };

                            if event_tx.send(ClusterEvent::Spot(spot)).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(ClusterEvent::Disconnected(format!("Błąd transmisji z {}: {}", addr, e)));
                        break;
                    }
                }
            }

            if *stop_rx.borrow() {
                break;
            }

            // Oczekiwanie przed ponowną próbą
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(4)) => {}
                _ = stop_rx.changed() => break,
            }
        }

        let _ = event_tx.send(ClusterEvent::Disconnected("Rozłączono z klastrem DX.".to_string()));
    }

    /// Łączy się z klastrem DX i transmituje odebrane spoty przez kanał broadcast
    pub async fn run(&self) {
        let addr = format!("{}:{}", self.host, self.port);
        let spot_regex = Regex::new(
            r"^DX de\s+([A-Z0-9/\-#]+):\s+([0-9.]+)\s+([A-Z0-9/]+)\s+(.*?)\s+([0-9]{4})Z?"
        ).unwrap();

        loop {
            if let Ok(stream) = TcpStream::connect(&addr).await {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let _ = writer.write_all(format!("{}\n", self.my_call).as_bytes()).await;

                let mut line = String::new();
                while let Ok(n) = buf_reader.read_line(&mut line).await {
                    if n == 0 {
                        break;
                    }
                    let trimmed = line.trim();
                    if let Some(caps) = spot_regex.captures(trimmed) {
                        let spotter = caps.get(1).map_or("", |m| m.as_str()).to_string();
                        let freq_str = caps.get(2).map_or("", |m| m.as_str());
                        let dx_call = caps.get(3).map_or("", |m| m.as_str()).to_string();
                        let comment = caps.get(4).map_or("", |m| m.as_str()).trim().to_string();
                        let time_utc = caps.get(5).map_or("", |m| m.as_str()).to_string();

                        let freq_khz: f64 = freq_str.parse().unwrap_or(0.0);
                        let band = Self::freq_khz_to_band(freq_khz);

                        let comment_upper = comment.to_uppercase();
                        let is_ft8 = comment_upper.contains("FT8") || comment_upper.contains("FT4") || comment_upper.contains("JS8");
                        let is_skimmer = spotter.contains("-#") || comment_upper.contains("BPS") || comment_upper.contains("WPM") || comment_upper.contains("DB");

                        let spot = DxSpot {
                            spotter,
                            frequency_khz: freq_khz,
                            dx_call,
                            comment,
                            time_utc,
                            band,
                            is_ft8,
                            is_skimmer,
                        };

                        let _ = self.spot_sender.send(spot);
                    }
                    line.clear();
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    fn freq_khz_to_band(khz: f64) -> String {
        let mhz = khz / 1000.0;
        if (1.8..=2.0).contains(&mhz) {
            "160m".to_string()
        } else if (3.5..=3.8).contains(&mhz) {
            "80m".to_string()
        } else if (5.25..=5.45).contains(&mhz) {
            "60m".to_string()
        } else if (7.0..=7.3).contains(&mhz) {
            "40m".to_string()
        } else if (10.1..=10.15).contains(&mhz) {
            "30m".to_string()
        } else if (14.0..=14.35).contains(&mhz) {
            "20m".to_string()
        } else if (18.068..=18.168).contains(&mhz) {
            "17m".to_string()
        } else if (21.0..=21.45).contains(&mhz) {
            "15m".to_string()
        } else if (24.89..=24.99).contains(&mhz) {
            "12m".to_string()
        } else if (28.0..=29.7).contains(&mhz) {
            "10m".to_string()
        } else if (50.0..=54.0).contains(&mhz) {
            "6m".to_string()
        } else if (69.9..=70.5).contains(&mhz) {
            "4m".to_string()
        } else if (144.0..=148.0).contains(&mhz) {
            "2m".to_string()
        } else if (430.0..=440.0).contains(&mhz) {
            "70cm".to_string()
        } else if (1240.0..=1300.0).contains(&mhz) {
            "23cm".to_string()
        } else {
            "OTHER".to_string()
        }
    }
}
