// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Sieciowa synchronizacja łączności wielostanowiskowej w sieci lokalnej (Multi-Op LAN)

use crate::core::qso::QsoRecord;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{Notify, broadcast};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiOpMessage {
    /// Musi być pierwszą wiadomością wysłaną przez klienta, jeśli host skonfigurował
    /// hasło współdzielone. Bez poprawnego hasła serwer natychmiast zrywa połączenie.
    Auth {
        secret: String,
    },
    NewQso(Box<QsoRecord>),
    ChatMessage {
        sender: String,
        text: String,
    },
    Heartbeat {
        station: String,
    },
}

/// Maksymalna długość pojedynczej linii JSON odbieranej od klienta Multi-Op.
/// Zapobiega wyczerpaniu pamięci przez klienta wysyłającego dane bez znaku
/// nowej linii (celowo lub przez błąd).
const MAX_LINE_BYTES: usize = 256 * 1024;

/// Maksymalna liczba równoczesnych klientów Multi-Op. Zapobiega wyczerpaniu
/// zasobów przez nadmiarowe połączenia.
const MAX_CLIENTS: usize = 32;

/// Czyta pojedynczą linię (ograniczoną do `MAX_LINE_BYTES`) ze strumienia.
/// Wyodrębnione jako funkcja, aby można było ją anulować przez `select!` przy
/// zamknięciu serwera.
async fn read_line_limited<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    line: &mut String,
) -> std::io::Result<usize> {
    reader
        .take((MAX_LINE_BYTES + 1) as u64)
        .read_line(line)
        .await
}

pub struct MultiOpServer {
    tx: broadcast::Sender<MultiOpMessage>,
    port: u16,
    /// Interfejs, na którym nasłuchuje serwer (domyślnie wszystkie interfejsy).
    bind_host: String,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    shutdown: Arc<Notify>,
    client_count: Arc<std::sync::atomic::AtomicUsize>,
    /// Hasło współdzielone wymagane od łączących się klientów. `None` oznacza,
    /// że serwer odmówi uruchomienia — niezabezpieczony tryb jest niedozwolony.
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
            bind_host: "0.0.0.0".to_string(),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            shutdown: Arc::new(Notify::new()),
            client_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            shared_secret: shared_secret.filter(|s| !s.is_empty()),
        }
    }

    /// Ustawia jawny interfejs nasłuchu (np. `"127.0.0.1"` zamiast `"0.0.0.0"`).
    pub fn with_bind_host(mut self, host: impl Into<String>) -> Self {
        self.bind_host = host.into();
        self
    }

    /// Czy serwer jest aktualnie uruchomiony.
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Uruchamia serwer nasłuchujący w sieci LAN. Wymaga ustawionego hasła
    /// współdzielonego — niezabezpieczony serwer zostanie odrzucony.
    pub async fn start(
        &self,
        incoming_qso_sender: tokio::sync::mpsc::UnboundedSender<QsoRecord>,
    ) -> Result<(), String> {
        // Bez hasła współdzielonego serwer mógłby przyjmować QSO od dowolnego
        // hosta w sieci — odmawiamy uruchomienia w takiej konfiguracji.
        if self.shared_secret.is_none() {
            return Err(
                "Odmowa uruchomienia serwera Multi-Op: wymagane jest ustawienie hasła współdzielonego."
                    .to_string(),
            );
        }
        if self
            .is_running
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Err("Serwer Multi-Op jest już uruchomiony.".to_string());
        }

        let addr = format!("{}:{}", self.bind_host, self.port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                self.is_running
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                return Err(format!("Błąd bindowania TCP {addr}: {e}"));
            }
        };
        info!("Serwer Multi-Op LAN nasłuchuje na {addr}");

        let tx = self.tx.clone();
        let shutdown = self.shutdown.clone();
        let client_count = self.client_count.clone();
        let shared_secret = self.shared_secret.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = shutdown.notified() => break,
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, peer_addr)) => {
                                if client_count.load(std::sync::atomic::Ordering::SeqCst) >= MAX_CLIENTS {
                                    error!("Multi-Op: osiągnięto limit {MAX_CLIENTS} klientów, odrzucam {peer_addr}.");
                                    continue;
                                }
                                client_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                info!("Nowe połączenie Multi-Op ze stacją: {peer_addr}");
                                let q_sender = incoming_qso_sender.clone();
                                let mut rx = tx.subscribe();
                                let (read_half, mut write_half) = stream.into_split();
                                let mut reader = BufReader::new(read_half);
                                let secret_for_conn = shared_secret.clone();
                                let shutdown_rx = shutdown.clone();
                                let client_count_rx = client_count.clone();
                                // Sygnał zakończenia pojedynczego połączenia: gdy wątek
                                // odbiorczy kończy pracę (rozłączenie/limit/błąd), budzi
                                // wątek nadawczy, aby ten również się zakończył.
                                let conn_shutdown = Arc::new(Notify::new());

                                // Wątek odbiorczy od klienta
                                let conn_shutdown_rx = conn_shutdown.clone();
                                tokio::spawn(async move {
                                    let mut line = String::new();
                                    let mut authenticated = secret_for_conn.is_none();
                                    loop {
                                        line.clear();
                                        let res = tokio::select! {
                                            () = shutdown_rx.notified() => break,
                                            r = read_line_limited(&mut reader, &mut line) => r,
                                        };
                                        match res {
                                            Ok(0) => break,
                                            Ok(_) if line.len() > MAX_LINE_BYTES => {
                                                error!("Multi-Op: linia od {peer_addr} przekracza limit {MAX_LINE_BYTES} B, zrywam połączenie.");
                                                break;
                                            }
                                            Ok(_) => {
                                                let Ok(msg) = serde_json::from_str::<MultiOpMessage>(&line) else { continue };
                                                match msg {
                                                    MultiOpMessage::Auth { secret } => {
                                                        authenticated = secret_for_conn.as_deref() == Some(secret.as_str());
                                                        if !authenticated {
                                                            error!("Multi-Op: nieprawidłowe hasło od {peer_addr}, zrywam połączenie.");
                                                            break;
                                                        }
                                                    }
                                                    MultiOpMessage::NewQso(qso) => {
                                                        if authenticated {
                                                            let _ = q_sender.send(*qso);
                                                        } else {
                                                            error!("Multi-Op: odrzucono QSO od {peer_addr} (brak uwierzytelnienia).");
                                                            break;
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            Err(e) => {
                                                error!("Multi-Op: błąd odczytu od {peer_addr}: {e}");
                                                break;
                                            }
                                        }
                                    }
                                    client_count_rx.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                                    conn_shutdown_rx.notify_waiters();
                                });

                                // Wątek nadawczy do klienta
                                let shutdown_tx = shutdown.clone();
                                let conn_shutdown_tx = conn_shutdown.clone();
                                tokio::spawn(async move {
                                    loop {
                                        let msg = tokio::select! {
                                            () = shutdown_tx.notified() => break,
                                            () = conn_shutdown_tx.notified() => break,
                                            m = rx.recv() => match m {
                                                Ok(msg) => msg,
                                                Err(_) => break,
                                            },
                                        };
                                        if let Ok(json) = serde_json::to_string(&msg) {
                                            if write_half.write_all(format!("{json}\n").as_bytes()).await.is_err() {
                                                break;
                                            }
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Błąd akceptacji połączenia Multi-Op: {e}");
                            }
                        }
                    }
                }
            }
            is_running.store(false, std::sync::atomic::Ordering::SeqCst);
            info!("Serwer Multi-Op LAN zatrzymany.");
        });

        Ok(())
    }

    /// Zatrzymuje serwer i budzi wszystkie zadania nasłuchujące oraz połączenia.
    pub fn stop(&self) {
        if self
            .is_running
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            self.shutdown.notify_waiters();
        }
    }

    /// Rozgłasza nową łączność do wszystkich połączonych komputerów w sieci LAN
    pub fn broadcast_qso(&self, qso: &QsoRecord) {
        let _ = self.tx.send(MultiOpMessage::NewQso(Box::new(qso.clone())));
    }
}
