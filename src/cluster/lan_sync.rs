// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Sieciowa synchronizacja łączności wielostanowiskowej w sieci lokalnej (Multi-Op LAN)

use crate::core::qso::QsoRecord;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiOpMessage {
    /// Musi być pierwszą wiadomością wysłaną przez klienta, jeśli host skonfigurował
    /// hasło współdzielone. Bez poprawnego hasła serwer natychmiast zrywa połączenie.
    Auth { secret: String },
    NewQso(Box<QsoRecord>),
    ChatMessage { sender: String, text: String },
    Heartbeat { station: String },
}

/// Maksymalna długość pojedynczej linii JSON odbieranej od klienta Multi-Op.
/// Zapobiega wyczerpaniu pamięci przez klienta wysyłającego dane bez znaku
/// nowej linii (celowo lub przez błąd).
const MAX_LINE_BYTES: usize = 256 * 1024;

pub struct MultiOpServer {
    tx: broadcast::Sender<MultiOpMessage>,
    port: u16,
    is_running: Arc<Mutex<bool>>,
    /// Opcjonalne hasło współdzielone wymagane od łączących się klientów.
    /// `None`/puste oznacza brak uwierzytelniania (tryb zgodności wstecznej dla
    /// zaufanych sieci LAN) — zalecane jest jednak jego ustawienie.
    shared_secret: Option<String>,
}

impl MultiOpServer {
    pub fn new(port: u16) -> Self {
        Self::new_with_secret(port, None)
    }

    pub fn new_with_secret(port: u16, shared_secret: Option<String>) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            port,
            is_running: Arc::new(Mutex::new(false)),
            shared_secret: shared_secret.filter(|s| !s.is_empty()),
        }
    }

    /// Uruchamia serwer nasłuchujący w sieci LAN
    pub async fn start(&self, incoming_qso_sender: tokio::sync::mpsc::UnboundedSender<QsoRecord>) -> Result<(), String> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| format!("Błąd bindowania TCP {}: {}", addr, e))?;
        info!("Serwer Multi-Op LAN nasłuchuje na {}", addr);

        let tx = self.tx.clone();
        let running_flag = self.is_running.clone();
        let shared_secret = self.shared_secret.clone();
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
                        let secret_for_conn = shared_secret.clone();

                        // Wątek odbiorczy od klienta
                        tokio::spawn(async move {
                            let mut line = String::new();
                            let mut authenticated = secret_for_conn.is_none();
                            loop {
                                line.clear();
                                match (&mut reader).take((MAX_LINE_BYTES + 1) as u64).read_line(&mut line).await {
                                    Ok(0) => break,
                                    Ok(_) if line.len() > MAX_LINE_BYTES => {
                                        error!("Multi-Op: linia od {} przekracza limit {} B, zrywam połączenie.", peer_addr, MAX_LINE_BYTES);
                                        break;
                                    }
                                    Ok(_) => {
                                        let Ok(msg) = serde_json::from_str::<MultiOpMessage>(&line) else { continue };
                                        match msg {
                                            MultiOpMessage::Auth { secret } => {
                                                authenticated = secret_for_conn.as_deref() == Some(secret.as_str());
                                                if !authenticated {
                                                    error!("Multi-Op: nieprawidłowe hasło od {}, zrywam połączenie.", peer_addr);
                                                    break;
                                                }
                                            }
                                            MultiOpMessage::NewQso(qso) => {
                                                if authenticated {
                                                    let _ = q_sender.send(*qso);
                                                } else {
                                                    error!("Multi-Op: odrzucono QSO od {} (brak uwierzytelnienia).", peer_addr);
                                                    break;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    Err(e) => {
                                        error!("Multi-Op: błąd odczytu od {}: {}", peer_addr, e);
                                        break;
                                    }
                                }
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
