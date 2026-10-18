// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Implementacja węzła Peer synchronizacji LAN z automatycznym reconnectem i backoffem (`src/sync/peer.rs`).

use super::error::SyncError;
use super::handshake::peer_handshake;
use super::protocol::Message;
use super::service::{SyncEvent, SyncState, SyncStatus};
use super::session::{SessionSender, SyncSession};
use super::store::SyncStore;
use crate::core::qso::QsoRecord;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
pub const MAX_BACKOFF: Duration = Duration::from_secs(30);
pub const PING_INTERVAL: Duration = Duration::from_secs(15);
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(45);

/// Kalkulator opóźnień ponownych prób połączenia (wykładniczy backoff).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectBackoff {
    current: Duration,
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self::new()
    }
}

impl ReconnectBackoff {
    pub fn new() -> Self {
        Self {
            current: INITIAL_BACKOFF,
        }
    }

    pub fn current(&self) -> Duration {
        self.current
    }

    /// Rejestruje niepowodzenie: zwraca czas do odczekania, a kolejny backoff podwaja (max 30s).
    pub fn fail(&mut self) -> Duration {
        let delay = self.current;
        self.current = (self.current * 2).min(MAX_BACKOFF);
        delay
    }

    /// Resetuje backoff do wartości początkowej (2s) po udanym połączeniu.
    pub fn reset(&mut self) {
        self.current = INITIAL_BACKOFF;
    }
}

/// Węzeł Peer łączący się z węzłem Host.
pub struct SyncPeer {
    stop_tx: watch::Sender<bool>,
    current_backoff: Arc<Mutex<Duration>>,
    active_sender: Arc<Mutex<Option<SessionSender>>>,
    runner_handle: Option<JoinHandle<()>>,
    events: std::sync::mpsc::Sender<SyncEvent>,
}

impl SyncPeer {
    /// Tworzy i uruchamia węzeł Peer w tle.
    pub fn start(
        host_addr: SocketAddr,
        station: String,
        secret: String,
        events: std::sync::mpsc::Sender<SyncEvent>,
    ) -> Self {
        Self::start_with_store(host_addr, station, secret, None, None, events)
    }

    /// Tworzy i uruchamia węzeł Peer z opcjonalnym magazynem i aktywnym dziennikiem do procedury Catch-Up.
    pub fn start_with_store(
        host_addr: SocketAddr,
        station: String,
        secret: String,
        journal: Option<String>,
        store: Option<Arc<dyn SyncStore>>,
        events: std::sync::mpsc::Sender<SyncEvent>,
    ) -> Self {
        let (stop_tx, stop_rx) = watch::channel(false);
        let current_backoff = Arc::new(Mutex::new(INITIAL_BACKOFF));
        let active_sender = Arc::new(Mutex::new(None));

        let runner_handle = tokio::spawn(run_peer_loop(
            host_addr,
            station,
            secret,
            journal,
            store,
            events.clone(),
            stop_rx,
            current_backoff.clone(),
            active_sender.clone(),
        ));

        Self {
            stop_tx,
            current_backoff,
            active_sender,
            runner_handle: Some(runner_handle),
            events,
        }
    }

    /// Zwraca aktualne opóźnienie backoff.
    pub fn current_backoff(&self) -> Duration {
        *self.current_backoff.lock().unwrap()
    }

    /// Sprawdza, czy Peer jest aktualnie połączony z Hostem.
    pub fn is_connected(&self) -> bool {
        self.active_sender.lock().unwrap().is_some()
    }

    /// Wysyła lokalne QSO do Hosta (który zretransmituje je do pozostałych peerów).
    pub fn broadcast_qso(&self, qso: QsoRecord) -> Result<(), SyncError> {
        let sender = { self.active_sender.lock().unwrap().clone() };
        match sender {
            Some(s) => s.try_send(Message::Qso(Box::new(qso))),
            None => Err(SyncError::ConnectionClosed),
        }
    }

    /// Zatrzymuje pętlę reconnectu i zamyka bieżące połączenie.
    pub fn stop(&mut self) {
        let _ = self.stop_tx.send(true);
        if let Some(handle) = self.runner_handle.take() {
            handle.abort();
        }
        {
            let mut sender_lock = self.active_sender.lock().unwrap();
            if let Some(sender) = sender_lock.take() {
                let _ = sender.try_send(Message::Bye);
            }
        }
        let _ = self.events.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Starting,
            peers: 0,
        }));
    }
}

impl Drop for SyncPeer {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn run_peer_loop(
    host_addr: SocketAddr,
    station: String,
    secret: String,
    journal: Option<String>,
    store: Option<Arc<dyn SyncStore>>,
    events: std::sync::mpsc::Sender<SyncEvent>,
    mut stop_rx: watch::Receiver<bool>,
    current_backoff: Arc<Mutex<Duration>>,
    active_sender: Arc<Mutex<Option<SessionSender>>>,
) {
    let mut backoff = ReconnectBackoff::new();
    {
        *current_backoff.lock().unwrap() = backoff.current();
    }

    loop {
        if *stop_rx.borrow() {
            break;
        }

        let _ = events.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Reconnecting,
            peers: 0,
        }));

        // Próba połączenia TCP
        let stream_res = tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_ok() && *stop_rx.borrow() {
                    break;
                }
                continue;
            }
            res = TcpStream::connect(host_addr) => res,
        };

        match stream_res {
            Ok(stream) => {
                // Handshake z Hostem
                let handshake_res = tokio::select! {
                    changed = stop_rx.changed() => {
                        if changed.is_ok() && *stop_rx.borrow() {
                            break;
                        }
                        continue;
                    }
                    res = peer_handshake(stream, &station, &secret) => res,
                };

                match handshake_res {
                    Ok(handshake_result) => {
                        // Sukces: reset backoffu do wartości początkowej!
                        backoff.reset();
                        {
                            *current_backoff.lock().unwrap() = backoff.current();
                        }

                        let session = handshake_result.session;
                        {
                            *active_sender.lock().unwrap() = Some(session.sender());
                        }

                        let _ = events.send(SyncEvent::Status(SyncStatus {
                            state: SyncState::Connected,
                            peers: 1,
                        }));

                        // Pętla sesji
                        run_peer_session(
                            session,
                            events.clone(),
                            &mut stop_rx,
                            journal.clone(),
                            store.clone(),
                        )
                        .await;

                        // Koniec sesji (rozłączenie)
                        {
                            *active_sender.lock().unwrap() = None;
                        }

                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = events.send(SyncEvent::Error(err));
                    }
                }
            }
            Err(err) => {
                let _ = events.send(SyncEvent::Error(SyncError::Io(err.to_string())));
            }
        }

        if *stop_rx.borrow() {
            break;
        }

        // Zastosowanie backoffu
        let delay = backoff.fail();
        {
            *current_backoff.lock().unwrap() = backoff.current();
        }

        let _ = events.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Reconnecting,
            peers: 0,
        }));

        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_ok() && *stop_rx.borrow() {
                    break;
                }
            }
            () = tokio::time::sleep(delay) => {},
        }
    }

    let _ = events.send(SyncEvent::Status(SyncStatus {
        state: SyncState::Starting,
        peers: 0,
    }));
}

async fn run_peer_session(
    mut session: SyncSession,
    events: std::sync::mpsc::Sender<SyncEvent>,
    stop_rx: &mut watch::Receiver<bool>,
    journal: Option<String>,
    store: Option<Arc<dyn SyncStore>>,
) {
    let mut catchup = match (journal, store) {
        (Some(j), Some(s)) => {
            let handler = super::catchup::CatchUpHandler::new(j, s, events.clone());
            let _ = handler.send_inventory(&session.sender());
            Some(handler)
        }
        _ => None,
    };

    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ping_interval.tick().await;

    let mut idle_sleep = Box::pin(tokio::time::sleep(IDLE_TIMEOUT));

    loop {
        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_ok() && *stop_rx.borrow() {
                    let _ = session.send(Message::Bye).await;
                    break;
                }
            }
            () = &mut idle_sleep => {
                let _ = events.send(SyncEvent::Error(SyncError::Timeout));
                break;
            }
            _ = ping_interval.tick() => {
                if session.sender().try_send(Message::Ping).is_err() {
                    break;
                }
            }
            msg = session.recv() => {
                match msg {
                    Ok(Message::Ping) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                        let _ = session.sender().try_send(Message::Pong);
                    }
                    Ok(Message::Pong | Message::Hello { .. }) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                    }
                    Ok(Message::Qso(qso)) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                        let _ = events.send(SyncEvent::IncomingQso(qso));
                    }
                    Ok(Message::Bye) => {
                        break;
                    }
                    Ok(Message::Inventory { journal, identities, last }) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                        if let Some(ref mut c) = catchup {
                            let _ = c.handle_inventory(&journal, identities, last, &session.sender());
                        }
                    }
                    Ok(Message::Request { identities, last }) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                        if let Some(ref mut c) = catchup {
                            let _ = c.handle_request(identities, last, &session.sender());
                        }
                    }
                    Ok(Message::Records { qsos, .. }) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                        if let Some(ref mut c) = catchup {
                            let _ = c.handle_records(qsos);
                        }
                    }
                    Err(err) => {
                        let _ = events.send(SyncEvent::Error(err));
                        break;
                    }
                }
            }
        }
    }
}
