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
                                let tx_for_conn = tx.clone();
                                let (read_half, mut write_half) = stream.into_split();
                                let mut reader = BufReader::new(read_half);
                                let secret_for_conn = shared_secret.clone();
                                let shutdown_rx = shutdown.clone();
                                let client_count_rx = client_count.clone();

                                tokio::spawn(async move {
                                    let mut line = String::new();
                                    if let Some(ref expected_secret) = secret_for_conn {
                                        let auth_res = tokio::select! {
                                            () = shutdown_rx.notified() => {
                                                client_count_rx.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                                                return;
                                            }
                                            r = tokio::time::timeout(
                                                std::time::Duration::from_secs(5),
                                                read_line_limited(&mut reader, &mut line),
                                            ) => r,
                                        };

                                        let authenticated = match auth_res {
                                            Ok(Ok(n)) if n > 0 && line.len() <= MAX_LINE_BYTES => {
                                                matches!(
                                                    serde_json::from_str::<MultiOpMessage>(&line),
                                                    Ok(MultiOpMessage::Auth { secret }) if secret == *expected_secret
                                                )
                                            }
                                            Ok(Ok(_)) => {
                                                error!("Multi-Op: pusta lub zbyt długa ramka Auth od {peer_addr}, zrywam połączenie.");
                                                false
                                            }
                                            Ok(Err(e)) => {
                                                error!("Multi-Op: błąd odczytu Auth od {peer_addr}: {e}");
                                                false
                                            }
                                            Err(_) => {
                                                error!("Multi-Op: przekroczono czas oczekiwania na Auth (5s) od {peer_addr}, zrywam połączenie.");
                                                false
                                            }
                                        };

                                        if !authenticated {
                                            error!("Multi-Op: nieprawidłowe hasło od {peer_addr}, zrywam połączenie.");
                                            client_count_rx.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                                            return;
                                        }
                                    }

                                    // Sygnał zakończenia pojedynczego połączenia: gdy wątek
                                    // odbiorczy kończy pracę (rozłączenie/limit/błąd), budzi
                                    // wątek nadawczy, aby ten również się zakończył.
                                    let conn_shutdown = Arc::new(Notify::new());
                                    let conn_shutdown_tx = conn_shutdown.clone();
                                    let shutdown_tx = shutdown_rx.clone();
                                    let mut rx = tx_for_conn.subscribe();

                                    // Wątek nadawczy do klienta uruchamiany dopiero PO pomyślnej autoryzacji
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
                                                let payload = format!("{json}\n");
                                                match tokio::time::timeout(
                                                    std::time::Duration::from_secs(5),
                                                    write_half.write_all(payload.as_bytes()),
                                                )
                                                .await
                                                {
                                                    Ok(Ok(())) => {}
                                                    _ => break,
                                                }
                                            }
                                        }
                                    });

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
                                                        if secret_for_conn.as_deref() != Some(secret.as_str()) {
                                                            error!("Multi-Op: nieprawidłowe hasło od {peer_addr}, zrywam połączenie.");
                                                            break;
                                                        }
                                                    }
                                                    MultiOpMessage::NewQso(qso) => {
                                                        let _ = q_sender.send(*qso.clone());
                                                        let _ = tx_for_conn.send(MultiOpMessage::NewQso(qso));
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
                                    conn_shutdown.notify_waiters();
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

/// Klient synchronizacji łączności Multi-Op LAN łączący się ze stacją nadrzędną (`MultiOpServer`).
pub struct MultiOpClient {
    host: String,
    port: u16,
    shared_secret: String,
    tx: broadcast::Sender<MultiOpMessage>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    shutdown: Arc<Notify>,
}

impl MultiOpClient {
    pub fn new(host: impl Into<String>, port: u16, shared_secret: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            host: host.into(),
            port,
            shared_secret: shared_secret.into(),
            tx,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            shutdown: Arc::new(Notify::new()),
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Wysyła lokalnie zalogowane QSO do serwera Multi-Op.
    pub fn send_qso(&self, qso: &QsoRecord) {
        let _ = self.tx.send(MultiOpMessage::NewQso(Box::new(qso.clone())));
    }

    /// Alias dla `send_qso` zachowujący spójność z interfejsem `MultiOpServer`.
    pub fn broadcast_qso(&self, qso: &QsoRecord) {
        self.send_qso(qso);
    }

    /// Zatrzymuje klienta Multi-Op.
    pub fn stop(&self) {
        if self
            .is_running
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            self.shutdown.notify_waiters();
        }
    }

    /// Nawiązuje połączenie z serwerem Multi-Op, wysyła ramkę `Auth` i uruchamia pętle odbioru/nadawania.
    pub async fn start(
        &self,
        incoming_qso_sender: tokio::sync::mpsc::UnboundedSender<QsoRecord>,
    ) -> Result<(), String> {
        if self.shared_secret.trim().is_empty() {
            return Err("Wymagane jest hasło współdzielone dla klienta Multi-Op.".to_string());
        }
        if self
            .is_running
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Err("Klient Multi-Op jest już uruchomiony.".to_string());
        }

        let addr = format!("{}:{}", self.host, self.port);
        let stream = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                self.is_running
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                return Err(format!("Błąd połączenia z serwerem Multi-Op {addr}: {e}"));
            }
            Err(_) => {
                self.is_running
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                return Err(format!(
                    "Przekroczono czas oczekiwania na połączenie z {addr} (5s)"
                ));
            }
        };

        let (read_half, mut write_half) = stream.into_split();
        let auth_msg = MultiOpMessage::Auth {
            secret: self.shared_secret.clone(),
        };
        let auth_json = serde_json::to_string(&auth_msg)
            .map_err(|e| format!("Błąd serializacji Auth: {e}"))?;
        if let Err(e) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            write_half.write_all(format!("{auth_json}\n").as_bytes()),
        )
        .await
        .map_err(|_| "Przekroczono czas wysyłania Auth (5s)".to_string())
        .and_then(|r| r.map_err(|e| format!("Błąd wysyłania Auth: {e}")))
        {
            self.is_running
                .store(false, std::sync::atomic::Ordering::SeqCst);
            return Err(e);
        }

        let mut rx = self.tx.subscribe();
        let shutdown_rx = self.shutdown.clone();
        let shutdown_tx = self.shutdown.clone();
        let is_running = self.is_running.clone();
        let conn_shutdown = Arc::new(Notify::new());
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
                    let payload = format!("{json}\n");
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        write_half.write_all(payload.as_bytes()),
                    )
                    .await
                    {
                        Ok(Ok(())) => {}
                        _ => break,
                    }
                }
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();
            loop {
                line.clear();
                let res = tokio::select! {
                    () = shutdown_rx.notified() => break,
                    r = read_line_limited(&mut reader, &mut line) => r,
                };
                match res {
                    Ok(0) => break,
                    Ok(_) if line.len() > MAX_LINE_BYTES => break,
                    Ok(_) => {
                        let Ok(msg) = serde_json::from_str::<MultiOpMessage>(&line) else {
                            continue;
                        };
                        if let MultiOpMessage::NewQso(qso) = msg {
                            let _ = incoming_qso_sender.send(*qso);
                        }
                    }
                    Err(_) => break,
                }
            }
            conn_shutdown.notify_waiters();
            is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unauthenticated_client_does_not_receive_broadcast_qsos() {
        // Bind dynamic port via a temporary listener to find a free port
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let server = MultiOpServer::new_with_secret(port, Some("top-secret".into()))
            .with_bind_host("127.0.0.1");
        let (srv_tx, _srv_rx) = tokio::sync::mpsc::unbounded_channel();
        server.start(srv_tx).await.unwrap();

        // Connect raw TCP socket without sending Auth
        let mut rogue = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();

        // Broadcast a QSO from the server while rogue client hasn't authenticated
        let mut qso = QsoRecord::default();
        qso.callsign = "SP6INA".into();
        server.broadcast_qso(&qso);

        // Send invalid auth to close connection quickly
        rogue
            .write_all(b"{\"Auth\":{\"secret\":\"wrong\"}}\n")
            .await
            .unwrap();

        let mut buf = Vec::new();
        let _ = rogue.read_to_end(&mut buf).await;
        assert!(
            buf.is_empty(),
            "Nieautoryzowany klient nie może otrzymać żadnych danych: {:?}",
            String::from_utf8_lossy(&buf)
        );

        server.stop();
    }

    #[tokio::test]
    async fn authenticated_multi_op_client_and_server_exchange_qsos() {
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        let server = MultiOpServer::new_with_secret(port, Some("ham-secret".into()))
            .with_bind_host("127.0.0.1");
        let (srv_tx, mut srv_rx) = tokio::sync::mpsc::unbounded_channel();
        server.start(srv_tx).await.unwrap();

        let client = MultiOpClient::new("127.0.0.1", port, "ham-secret");
        let (cli_tx, mut cli_rx) = tokio::sync::mpsc::unbounded_channel();
        client.start(cli_tx).await.unwrap();

        // Give server a brief moment to process Auth and subscribe client
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut qso_from_server = QsoRecord::default();
        qso_from_server.callsign = "DL1ABC".into();
        server.broadcast_qso(&qso_from_server);

        let received_by_client =
            tokio::time::timeout(std::time::Duration::from_secs(2), cli_rx.recv())
                .await
                .expect("timeout")
                .expect("qso");
        assert_eq!(received_by_client.callsign, "DL1ABC");

        let mut qso_from_client = QsoRecord::default();
        qso_from_client.callsign = "JA1XYZ".into();
        client.send_qso(&qso_from_client);

        let received_by_server =
            tokio::time::timeout(std::time::Duration::from_secs(2), srv_rx.recv())
                .await
                .expect("timeout")
                .expect("qso");
        assert_eq!(received_by_server.callsign, "JA1XYZ");

        client.stop();
        server.stop();
    }
}
