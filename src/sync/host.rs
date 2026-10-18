// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Implementacja węzła Host synchronizacji LAN/P2P z echo-free relay (`src/sync/host.rs`).

use super::crypto::random_salt;
use super::error::SyncError;
use super::handshake::host_handshake;
use super::protocol::Message;
use super::service::{SyncEvent, SyncState, SyncStatus};
use super::session::{SessionSender, SyncSession};
use crate::core::qso::QsoRecord;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};
use tokio::task::{AbortHandle, JoinHandle};

pub const MAX_ACTIVE_PEERS: usize = 32;
pub const MAX_HANDSHAKES: usize = 5;
pub const PING_INTERVAL: Duration = Duration::from_secs(15);
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(45);

struct PeerEntry {
    station: String,
    sender: SessionSender,
    abort_handle: AbortHandle,
}

/// Węzeł Host synchronizacji LAN.
pub struct SyncHost {
    local_addr: SocketAddr,
    peers: Arc<Mutex<HashMap<u64, PeerEntry>>>,
    handshake_sem: Arc<Semaphore>,
    stop_tx: watch::Sender<bool>,
    events: std::sync::mpsc::Sender<SyncEvent>,
    listener_handle: Option<JoinHandle<()>>,
}

impl SyncHost {
    /// Rozpoczyna nasłuchiwanie na podanym adresie TCP.
    pub async fn bind(
        addr: SocketAddr,
        station: String,
        secret: String,
        events: std::sync::mpsc::Sender<SyncEvent>,
    ) -> Result<Self, SyncError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| SyncError::Io(e.to_string()))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| SyncError::Io(e.to_string()))?;

        let peers = Arc::new(Mutex::new(HashMap::new()));
        let next_peer_id = Arc::new(AtomicU64::new(1));
        let handshake_sem = Arc::new(Semaphore::new(MAX_HANDSHAKES));
        let (stop_tx, stop_rx) = watch::channel(false);

        let listener_handle = tokio::spawn(run_listener_loop(
            listener,
            peers.clone(),
            next_peer_id,
            handshake_sem.clone(),
            station,
            secret,
            events.clone(),
            stop_rx,
            stop_tx.clone(),
        ));

        let _ = events.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Listening,
            peers: 0,
        }));

        Ok(Self {
            local_addr,
            peers,
            handshake_sem,
            stop_tx,
            events,
            listener_handle: Some(listener_handle),
        })
    }

    /// Zwraca lokalny adres gniazda TCP.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Zwraca liczbę aktualnie połączonych i uwierzytelnionych peerów.
    pub fn connected_peers(&self) -> usize {
        self.peers.lock().unwrap().len()
    }

    /// Zwraca listę znaków stacji aktualnie połączonych peerów.
    pub fn peer_stations(&self) -> Vec<String> {
        self.peers
            .lock()
            .unwrap()
            .values()
            .map(|p| p.station.clone())
            .collect()
    }

    /// Zwraca liczbę wolnych zezwoleń semafora handshake (do celów testowych).
    pub fn available_handshake_permits(&self) -> usize {
        self.handshake_sem.available_permits()
    }

    /// Retransmituje lokalne QSO Hosta do wszystkich połączonych peerów.
    /// Zwraca liczbę peerów, do których wysłano wiadomość.
    pub fn broadcast_qso(&self, qso: QsoRecord) -> usize {
        relay_to_peers(&self.peers, &self.events, None, Message::Qso(Box::new(qso)))
    }

    /// Zatrzymuje działanie Hosta, rozłącza wszystkich peerów i zamyka gniazdo.
    pub fn stop(&mut self) {
        let _ = self.stop_tx.send(true);
        if let Some(handle) = self.listener_handle.take() {
            handle.abort();
        }

        let mut p = self.peers.lock().unwrap();
        for (_, peer) in p.drain() {
            let _ = peer.sender.try_send(Message::Bye);
            peer.abort_handle.abort();
        }

        let _ = self.events.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Starting,
            peers: 0,
        }));
    }
}

impl Drop for SyncHost {
    fn drop(&mut self) {
        self.stop();
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_listener_loop(
    listener: TcpListener,
    peers: Arc<Mutex<HashMap<u64, PeerEntry>>>,
    next_peer_id: Arc<AtomicU64>,
    handshake_sem: Arc<Semaphore>,
    station: String,
    secret: String,
    events: std::sync::mpsc::Sender<SyncEvent>,
    mut stop_rx: watch::Receiver<bool>,
    stop_tx: watch::Sender<bool>,
) {
    loop {
        let stream = tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_ok() && *stop_rx.borrow() {
                    break;
                }
                continue;
            }
            res = listener.accept() => {
                match res {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        let _ = events.send(SyncEvent::Error(SyncError::Io(e.to_string())));
                        continue;
                    }
                }
            }
        };

        // Sprawdzenie limitu aktywnych klientów przed handshake'iem
        if peers.lock().unwrap().len() >= MAX_ACTIVE_PEERS {
            let _ = events.send(SyncEvent::Error(SyncError::ClientLimitReached));
            drop(stream);
            continue;
        }

        let sem = handshake_sem.clone();
        let peers = peers.clone();
        let next_peer_id = next_peer_id.clone();
        let events = events.clone();
        let peer_stop_rx = stop_tx.subscribe();
        let station = station.clone();
        let secret = secret.clone();

        tokio::spawn(async move {
            let permit = match sem.acquire_owned().await {
                Ok(p) => p,
                Err(_) => return,
            };

            let server_salt = random_salt();
            let server_key = match super::crypto::derive_key(&secret, &server_salt) {
                Ok(k) => k,
                Err(err) => {
                    let _ = events.send(SyncEvent::Error(err));
                    return;
                }
            };
            let handshake_result =
                match host_handshake(stream, &station, &server_salt, &server_key).await {
                    Ok(res) => res,
                    Err(err) => {
                        let _ = events.send(SyncEvent::Error(err));
                        return;
                    }
                };

            drop(permit);

            let peer_id = next_peer_id.fetch_add(1, Ordering::Relaxed);
            let session = handshake_result.session;
            let peer_station = handshake_result.remote_station;
            let sender = session.sender();

            // Uruchomienie sesji peera
            let session_handle = tokio::spawn(run_host_peer_session(
                peer_id,
                session,
                peers.clone(),
                events.clone(),
                peer_stop_rx,
            ));

            let abort_handle = session_handle.abort_handle();

            {
                let mut p = peers.lock().unwrap();
                if p.len() >= MAX_ACTIVE_PEERS {
                    let _ = events.send(SyncEvent::Error(SyncError::ClientLimitReached));
                    abort_handle.abort();
                    return;
                }
                p.insert(
                    peer_id,
                    PeerEntry {
                        station: peer_station,
                        sender,
                        abort_handle,
                    },
                );
                let _ = events.send(SyncEvent::Status(SyncStatus {
                    state: SyncState::Listening,
                    peers: p.len(),
                }));
            }

            // Oczekiwanie na zakończenie sesji peera
            let _ = session_handle.await;

            // Sprzątanie po rozłączeniu peera
            {
                let mut p = peers.lock().unwrap();
                p.remove(&peer_id);
                let _ = events.send(SyncEvent::Status(SyncStatus {
                    state: SyncState::Listening,
                    peers: p.len(),
                }));
            }
        });
    }
}

async fn run_host_peer_session(
    peer_id: u64,
    mut session: SyncSession,
    peers: Arc<Mutex<HashMap<u64, PeerEntry>>>,
    events: std::sync::mpsc::Sender<SyncEvent>,
    mut stop_rx: watch::Receiver<bool>,
) {
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
            _ = &mut idle_sleep => {
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
                    Ok(Message::Pong) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                    }
                    Ok(Message::Qso(qso)) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                        let _ = events.send(SyncEvent::IncomingQso(qso.clone()));
                        // Echo-free relay: przekazanie do pozostałych peerów oprócz nadawcy!
                        relay_to_peers(&peers, &events, Some(peer_id), Message::Qso(qso));
                    }
                    Ok(Message::Bye) => {
                        break;
                    }
                    Ok(Message::Hello { .. }) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
                    }
                    Ok(Message::Inventory { .. })
                    | Ok(Message::Request { .. })
                    | Ok(Message::Records { .. }) => {
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + IDLE_TIMEOUT);
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

fn relay_to_peers(
    peers: &Arc<Mutex<HashMap<u64, PeerEntry>>>,
    events: &std::sync::mpsc::Sender<SyncEvent>,
    exclude_id: Option<u64>,
    msg: Message,
) -> usize {
    let mut sent = 0;
    let mut slow_peers = Vec::new();
    {
        let p = peers.lock().unwrap();
        for (&id, peer) in p.iter() {
            if Some(id) == exclude_id {
                continue;
            }
            match peer.sender.try_send(msg.clone()) {
                Ok(()) => {
                    sent += 1;
                }
                Err(SyncError::QueueFull) => {
                    slow_peers.push((id, peer.abort_handle.clone()));
                }
                Err(_) => {}
            }
        }
    }

    if !slow_peers.is_empty() {
        let mut p = peers.lock().unwrap();
        for (slow_id, abort_handle) in slow_peers {
            p.remove(&slow_id);
            abort_handle.abort();
            let _ = events.send(SyncEvent::Error(SyncError::QueueFull));
        }
        let _ = events.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Listening,
            peers: p.len(),
        }));
    }

    sent
}
