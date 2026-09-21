// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Sieciowa synchronizacja łączności wielostanowiskowej w sieci lokalnej (Multi-Op LAN)

use crate::core::qso::QsoRecord;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiOpMessage {
    NewQso(Box<QsoRecord>),
    ChatMessage { sender: String, text: String },
    Heartbeat { station: String },
}

pub struct MultiOpServer {
    tx: broadcast::Sender<MultiOpMessage>,
    port: u16,
    is_running: Arc<Mutex<bool>>,
}

impl MultiOpServer {
    pub fn new(port: u16) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            port,
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Uruchamia serwer nasłuchujący w sieci LAN
    pub async fn start(&self, incoming_qso_sender: tokio::sync::mpsc::UnboundedSender<QsoRecord>) -> Result<(), String> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| format!("Błąd bindowania TCP {}: {}", addr, e))?;
        info!("Serwer Multi-Op LAN nasłuchuje na {}", addr);

        let tx = self.tx.clone();
        let running_flag = self.is_running.clone();
        *running_flag.lock().await = true;

        tokio::spawn(async move {
            while *running_flag.lock().await {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("Nowe połączenie Multi-Op ze stacją: {}", peer_addr);
                        let q_sender = incoming_qso_sender.clone();
                        let mut rx = tx.subscribe();
                        let (read_half, mut write_half) = stream.into_split();
                        let mut reader = BufReader::new(read_half);

                        // Wątek odbiorczy od klienta
                        tokio::spawn(async move {
                            let mut line = String::new();
                            while let Ok(n) = reader.read_line(&mut line).await {
                                if n == 0 { break; }
                                if let Ok(MultiOpMessage::NewQso(qso)) = serde_json::from_str::<MultiOpMessage>(&line) {
                                    let _ = q_sender.send(*qso);
                                }
                                line.clear();
                            }
                        });

                        // Wątek nadawczy do klienta
                        tokio::spawn(async move {
                            while let Ok(msg) = rx.recv().await {
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    if write_half.write_all(format!("{}\n", json).as_bytes()).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Błąd akceptacji połączenia Multi-Op: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Rozgłasza nową łączność do wszystkich połączonych komputerów w sieci LAN
    pub fn broadcast_qso(&self, qso: &QsoRecord) {
        let _ = self.tx.send(MultiOpMessage::NewQso(Box::new(qso.clone())));
    }
}
