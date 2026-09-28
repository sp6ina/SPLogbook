// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Wozniak (SP6INA)

//! Integracja z JS8Call przez TCP API (domyślny port 2442)
//! JS8Call implementuje prosty protokol JSON przez TCP
//! API: https://github.com/jsherer/js8call/blob/master/TCPAPI.md

use crate::core::qso::QsoRecord;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Wiadomosc API JS8Call (JSON)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Js8Message {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// Stan polaczenia z JS8Call
#[derive(Debug, Clone, Default)]
pub struct Js8CallState {
    pub connected: bool,
    pub dial_freq_hz: u64,
    pub offset_hz: i32,
    pub speed: i32,
    pub callsign: String,
    pub grid: String,
    pub last_heard: Vec<Js8HeardStation>,
}

/// Stacja ostatnio slyszana przez JS8Call
#[derive(Debug, Clone)]
pub struct Js8HeardStation {
    pub callsign: String,
    pub grid: String,
    pub snr: i32,
    pub freq_hz: i64,
    pub utc: String,
}

/// Klient TCP API JS8Call
pub struct Js8CallClient {
    host: String,
    port: u16,
}

impl Js8CallClient {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Laczy sie z JS8Call i odbiera wiadomosci w osobnym watku.
    /// Wiadomosci stanu przekazywane sa przez state_sender,
    /// a zakonczone QSO przez qso_sender do automatycznego logowania.
    pub fn start_listener(
        &self,
        state_sender: std::sync::mpsc::Sender<Js8CallState>,
        qso_sender: std::sync::mpsc::Sender<QsoRecord>,
    ) -> std::thread::JoinHandle<()> {
        let host = self.host.clone();
        let port = if self.port == 0 { 2442 } else { self.port };
        std::thread::spawn(move || {
            const MAX_LINE_LEN: usize = 64 * 1024;
            'listener: loop {
                // Proba nawiazania polaczenia TCP z JS8Call
                let parsed_addr = (host.as_str(), port)
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut it| it.next())
                    .unwrap_or_else(|| {
                        std::net::SocketAddr::from(([127, 0, 0, 1], port))
                    });

                if let Ok(stream) = TcpStream::connect_timeout(&parsed_addr, Duration::from_secs(3))
                {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let mut reader = BufReader::new(stream);
                    let mut state = Js8CallState {
                        connected: true,
                        ..Default::default()
                    };
                    // Powiadom aplikacje o podlaczeniu
                    if state_sender.send(state.clone()).is_err() {
                        break 'listener;
                    }

                    // Przetwarzanie linii JSON ze strumienia TCP
                    // Limit dlugosci linii chroni przed wyczerpaniem pamieci, gdyby druga
                    // strona (JS8Call lub proces podszywajacy sie pod niego) wyslala dane
                    // bez znaku nowej linii.
                    let mut buf = String::new();
                    loop {
                        buf.clear();
                        match (&mut reader)
                            .take((MAX_LINE_LEN + 1) as u64)
                            .read_line(&mut buf)
                        {
                            Ok(0) | Err(_) => break, // koniec strumienia lub blad odczytu
                            Ok(_) if buf.len() > MAX_LINE_LEN => break, // zbyt dluga linia - rozlaczenie
                            Ok(_) => {
                                let text = buf.trim();
                                if !text.is_empty() {
                                    if let Ok(msg) = serde_json::from_str::<Js8Message>(text) {
                                        if !process_message(
                                            msg,
                                            &mut state,
                                            &state_sender,
                                            &qso_sender,
                                        ) {
                                            break 'listener;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Utrata polaczenia - powiadom aplikacje
                    state.connected = false;
                    if state_sender.send(state).is_err() {
                        break 'listener;
                    }
                } else {
                    // Nie udalo sie polaczyc - JS8Call prawdopodobnie nie dziala
                }
                // Odczekaj przed kolejna proba reconnect
                std::thread::sleep(Duration::from_secs(5));
            }
        })
    }
}

fn value_as_u64_hz(val: &serde_json::Value) -> Option<u64> {
    val.as_u64()
        .or_else(|| {
            val.as_f64().map(|f| {
                if f > 0.0 && f < 1000.0 {
                    (f * 1_000_000.0).round() as u64
                } else {
                    f.max(0.0).round() as u64
                }
            })
        })
        .or_else(|| {
            val.as_str()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|f| {
                    if f > 0.0 && f < 1000.0 {
                        (f * 1_000_000.0).round() as u64
                    } else {
                        f.max(0.0).round() as u64
                    }
                })
        })
}

/// Przetwarza pojedyncza wiadomosc JSON odebrana z API JS8Call.
/// Zwraca `false`, gdy odbiorca kanału został zamknięty.
fn process_message(
    msg: Js8Message,
    state: &mut Js8CallState,
    state_sender: &std::sync::mpsc::Sender<Js8CallState>,
    qso_sender: &std::sync::mpsc::Sender<QsoRecord>,
) -> bool {
    match msg.msg_type.as_str() {
        "STATION.CALLSIGN" => {
            // Odebrano znak wywolawczy naszej stacji z JS8Call
            if let Some(v) = msg.value.as_str() {
                state.callsign = v.to_string();
                if state_sender.send(state.clone()).is_err() {
                    return false;
                }
            }
        }
        "STATION.GRID" => {
            // Odebrano lokator naszej stacji z JS8Call
            if let Some(v) = msg.value.as_str() {
                state.grid = v.to_string();
                if state_sender.send(state.clone()).is_err() {
                    return false;
                }
            }
        }
        "DIAL.FREQ" | "RIG.FREQ" => {
            // Aktualizacja czestotliwosci nosnej (dial frequency)
            let freq_opt = value_as_u64_hz(&msg.value).or_else(|| {
                msg.params.as_ref().and_then(|p| {
                    p.get("FREQ")
                        .or_else(|| p.get("DIAL"))
                        .and_then(value_as_u64_hz)
                })
            });
            if let Some(v) = freq_opt {
                state.dial_freq_hz = v;
                if state_sender.send(state.clone()).is_err() {
                    return false;
                }
            }
        }
        "LOG.QSO" => {
            // JS8Call zakonczyl QSO - auto-import do SPLogbook
            if let Some(params) = msg.params {
                let qso = build_qso_from_js8(&params, state);
                if qso_sender.send(qso).is_err() {
                    return false;
                }
            }
        }
        "RX.SPOT" => {
            // Aktualizacja listy slyszanych stacji
            if let Some(params) = &msg.params {
                let callsign = params
                    .get("CALL")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !callsign.is_empty() {
                    let heard = Js8HeardStation {
                        callsign,
                        grid: params
                            .get("GRID")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        snr: params
                            .get("SNR")
                            .and_then(|v| {
                                v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64))
                            })
                            .unwrap_or(0) as i32,
                        freq_hz: params
                            .get("FREQ")
                            .and_then(value_as_u64_hz)
                            .unwrap_or(0) as i64,
                        utc: Utc::now().format("%H:%M:%S").to_string(),
                    };
                    // Limit listy do 50 ostatnio slyszanych stacji
                    state.last_heard.insert(0, heard);
                    state.last_heard.truncate(50);
                    if state_sender.send(state.clone()).is_err() {
                        return false;
                    }
                }
            }
        }
        // Wiadomosci skierowane (RX.DIRECTED) i inne nieobslugiwane typy sa ignorowane.
        _ => {}
    }
    true
}

/// Buduje rekord QSO z parametrow wiadomosci LOG.QSO z JS8Call
fn build_qso_from_js8(params: &serde_json::Value, state: &Js8CallState) -> QsoRecord {
    let now = Utc::now();
    let freq_hz = params
        .get("FREQ")
        .or_else(|| params.get("DIAL"))
        .and_then(value_as_u64_hz)
        .filter(|&f| f > 0)
        .unwrap_or(state.dial_freq_hz);

    let band = params
        .get("BAND")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| freq_hz_to_band(freq_hz).to_string());

    let qso_date = params
        .get("QSO_DATE")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| now.format("%Y%m%d").to_string());

    let time_on = params
        .get("TIME_ON")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| now.format("%H%M").to_string());

    let rst_sent = params
        .get("RST_SENT")
        .and_then(|v| {
            v.as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string)
                .or_else(|| {
                    v.as_i64()
                        .or_else(|| v.as_f64().map(|f| f.round() as i64))
                        .map(|s| format!("{s:+}"))
                })
        })
        .unwrap_or_else(|| "+0".to_string());

    let rst_rcvd = params
        .get("RST_RCVD")
        .and_then(|v| {
            v.as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string)
                .or_else(|| {
                    v.as_i64()
                        .or_else(|| v.as_f64().map(|f| f.round() as i64))
                        .map(|s| format!("{s:+}"))
                })
        })
        .or_else(|| {
            params
                .get("SNR")
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
                .map(|s| format!("{s:+}"))
        })
        .unwrap_or_else(|| "+0".to_string());

    let name = params
        .get("NAME")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    let comment = params
        .get("COMMENTS")
        .or_else(|| params.get("COMMENT"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    QsoRecord {
        callsign: params
            .get("CALL")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_uppercase(),
        band,
        mode: "JS8".to_string(),
        qso_date,
        time_on,
        freq: Some(freq_hz as f64 / 1_000_000.0),
        gridsquare: params
            .get("GRID")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string),
        rst_sent,
        rst_rcvd,
        name,
        comment,
        my_gridsquare: if state.grid.is_empty() {
            None
        } else {
            Some(state.grid.clone())
        },
        ..QsoRecord::default()
    }
}

/// Zamienia czestotliwosc w Hz na nazwe pasma amatorskiego
fn freq_hz_to_band(hz: u64) -> &'static str {
    match hz {
        1_800_000..=2_000_000 => "160m",
        3_500_000..=4_000_000 => "80m",
        5_250_000..=5_450_000 => "60m",
        7_000_000..=7_300_000 => "40m",
        10_100_000..=10_150_000 => "30m",
        14_000_000..=14_350_000 => "20m",
        18_068_000..=18_168_000 => "17m",
        21_000_000..=21_450_000 => "15m",
        24_890_000..=24_990_000 => "12m",
        28_000_000..=29_700_000 => "10m",
        50_000_000..=54_000_000 => "6m",
        144_000_000..=148_000_000 => "2m",
        430_000_000..=440_000_000 => "70cm",
        _ => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freq_to_band() {
        assert_eq!(freq_hz_to_band(14_074_000), "20m");
        assert_eq!(freq_hz_to_band(7_074_000), "40m");
        assert_eq!(freq_hz_to_band(3_578_000), "80m");
        assert_eq!(freq_hz_to_band(144_174_000), "2m");
        assert_eq!(freq_hz_to_band(999_999), "Other");
    }

    #[test]
    fn test_js8_message_parse() {
        let json = r#"{"type":"STATION.CALLSIGN","value":"SP6INA"}"#;
        let msg: Js8Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "STATION.CALLSIGN");
        assert_eq!(msg.value.as_str(), Some("SP6INA"));
    }

    #[test]
    fn test_js8_message_parse_with_params() {
        let json =
            r#"{"type":"LOG.QSO","value":"","params":{"CALL":"K1ABC","GRID":"FN31","SNR":-10}}"#;
        let msg: Js8Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "LOG.QSO");
        let params = msg.params.unwrap();
        assert_eq!(params.get("CALL").and_then(|v| v.as_str()), Some("K1ABC"));
    }

    #[test]
    fn test_build_qso_from_js8() {
        let state = Js8CallState {
            connected: true,
            dial_freq_hz: 14_078_000,
            grid: "JO81WA".to_string(),
            callsign: "SP6INA".to_string(),
            ..Default::default()
        };
        let params = serde_json::json!({
            "CALL": "K1ABC",
            "GRID": "FN31pr",
            "SNR": -12
        });
        let qso = build_qso_from_js8(&params, &state);
        assert_eq!(qso.callsign, "K1ABC");
        assert_eq!(qso.band, "20m");
        assert_eq!(qso.mode, "JS8");
        assert_eq!(qso.rst_rcvd, "-12");
        assert_eq!(qso.gridsquare, Some("FN31pr".to_string()));
        assert_eq!(qso.my_gridsquare, Some("JO81WA".to_string()));
    }
}
