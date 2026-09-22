// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::sync::Arc;
use std::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::watch;
use crate::cat::hamlib::RigState;

/// Zdarzenia nadsyłane przez klientów TCP (np. WSJT-X, JTDX, FLDigi)
#[derive(Debug, Clone)]
pub enum RigServerCommand {
    SetFrequency(u64),
    SetMode(String),
    SetPtt(bool),
}

/// Serwer Hamlib rigctld proxy — udostępnia połączenie CAT dla WSJT-X, JTDX, FLDigi na porcie TCP (domyślnie 4534)
pub struct HamlibProxyServer {
    pub port: u16,
    pub shared_state: Arc<RwLock<RigState>>,
    pub cmd_sender: broadcast::Sender<RigServerCommand>,
    stop_tx: watch::Sender<bool>,
}

impl HamlibProxyServer {
    pub fn new(port: u16, shared_state: Arc<RwLock<RigState>>) -> (Self, broadcast::Receiver<RigServerCommand>) {
        let (cmd_sender, cmd_receiver) = broadcast::channel(64);
        let (stop_tx, _stop_rx) = watch::channel(false);
        (
            Self {
                port,
                shared_state,
                cmd_sender,
                stop_tx,
            },
            cmd_receiver,
        )
    }

    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        let mut stop_rx = self.stop_tx.subscribe();

        loop {
            tokio::select! {
                _ = stop_rx.changed() => {
                    if *stop_rx.borrow() {
                        break;
                    }
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, _peer)) => {
                            let state = self.shared_state.clone();
                            let cmd_tx = self.cmd_sender.clone();
                            let client_stop_rx = self.stop_tx.subscribe();
                            tokio::spawn(async move {
                                let _ = handle_client(stream, state, cmd_tx, client_stop_rx).await;
                            });
                        }
                        Err(_) => {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

async fn handle_client(
    mut stream: tokio::net::TcpStream,
    state: Arc<RwLock<RigState>>,
    cmd_tx: broadcast::Sender<RigServerCommand>,
    mut stop_rx: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        tokio::select! {
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    break;
                }
            }
            read_res = buf_reader.read_line(&mut line) => {
                match read_res {
                    Ok(0) => break, // EOF / Rozłączono
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        if trimmed == "q" || trimmed == "Q" {
                            break;
                        } else if trimmed == "\\dump_state" {
                            // Standardowa minimalna odpowiedź na dump_state zgodna z Hamlib rigctld
                            let dump_resp = "0\n2\n2\n100000 30000000 0xef -1 -1 0x3 0x3\n0 0 0 0 0 0 0\n0 0 0 0 0 0 0\n0xef 1\n0 0\n0xef 1\n0 0\n0 0\n0 0\n0\n0\n0\n0\n0\n";
                            writer.write_all(dump_resp.as_bytes()).await?;
                        } else if trimmed == "\\chk_vfo" {
                            writer.write_all(b"CHKVFO 0\n").await?;
                        } else if trimmed == "\\get_powerstat" {
                            writer.write_all(b"1\n").await?;
                        } else if trimmed == "f" {
                            let freq = state.read().map(|s| s.frequency_hz).unwrap_or(14074000);
                            writer.write_all(format!("{}\n", freq).as_bytes()).await?;
                        } else if trimmed.starts_with("F ") || trimmed.starts_with("f ") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 2 {
                                if let Ok(new_freq) = parts[1].parse::<u64>() {
                                    if let Ok(mut s) = state.write() {
                                        s.frequency_hz = new_freq;
                                    }
                                    let _ = cmd_tx.send(RigServerCommand::SetFrequency(new_freq));
                                }
                            }
                            writer.write_all(b"RPRT 0\n").await?;
                        } else if trimmed == "m" {
                            let (mode, pb) = state.read().map(|s| (s.mode.clone(), s.passband_hz)).unwrap_or_else(|_| ("USB".to_string(), 3000));
                            let hamlib_mode = match mode.to_uppercase().as_str() {
                                "CW" => "CW",
                                "LSB" => "LSB",
                                "USB" | "FT8" | "FT4" | "JS8" => "PKTUSB",
                                "AM" => "AM",
                                "FM" => "FM",
                                "RTTY" => "RTTY",
                                _ => "USB",
                            };
                            writer.write_all(format!("{}\n{}\n", hamlib_mode, pb).as_bytes()).await?;
                        } else if trimmed.starts_with("M ") || trimmed.starts_with("m ") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let new_mode = parts[1].to_string();
                                if let Ok(mut s) = state.write() {
                                    s.mode = new_mode.clone();
                                }
                                let _ = cmd_tx.send(RigServerCommand::SetMode(new_mode));
                            }
                            writer.write_all(b"RPRT 0\n").await?;
                        } else if trimmed == "t" {
                            let ptt = state.read().map(|s| s.ptt).unwrap_or(false);
                            let code = if ptt { "1\n" } else { "0\n" };
                            writer.write_all(code.as_bytes()).await?;
                        } else if trimmed.starts_with("T ") || trimmed.starts_with("t ") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let ptt = parts[1] == "1";
                                if let Ok(mut s) = state.write() {
                                    s.ptt = ptt;
                                }
                                let _ = cmd_tx.send(RigServerCommand::SetPtt(ptt));
                            }
                            writer.write_all(b"RPRT 0\n").await?;
                        } else if trimmed == "v" {
                            let vfo = state.read().map(|s| s.vfo.clone()).unwrap_or_else(|_| "VFOA".to_string());
                            writer.write_all(format!("{}\n", vfo).as_bytes()).await?;
                        } else if trimmed.starts_with("V ") || trimmed.starts_with("v ") {
                            writer.write_all(b"RPRT 0\n").await?;
                        } else if trimmed == "s" {
                            let split = state.read().map(|s| if s.split_enabled { 1 } else { 0 }).unwrap_or(0);
                            writer.write_all(format!("{}\nVFOA\n", split).as_bytes()).await?;
                        } else if trimmed.starts_with("S ") || trimmed.starts_with("s ") {
                            writer.write_all(b"RPRT 0\n").await?;
                        } else {
                            // Domyślna odpowiedź sukcesu RPRT 0 dla nierozpoznanych bezpiecznych poleceń
                            writer.write_all(b"RPRT 0\n").await?;
                        }
                        writer.flush().await?;
                    }
                    Err(_) => break,
                }
            }
        }
    }

    Ok(())
}
