// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Typy usług synchronizacji, stanu i zdarzeń (`src/sync/service.rs`).

use super::error::SyncError;
use crate::core::i18n::Msg;
use crate::core::qso::QsoRecord;

/// Rola węzła w synchronizacji Multi-Op (Host lub Peer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncRole {
    Host { port: u16 },
    Peer { host: String, port: u16 },
}

/// Konfiguracja usługi synchronizacji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConfig {
    pub role: SyncRole,
    pub secret: String,
    pub station: String,
    pub journal: String,
}

/// Stan maszyny stanów synchronizacji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Starting,
    Listening,
    Connected,
    Reconnecting,
    Failed,
}

/// Status operacyjny synchronizacji przekazywany do GUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStatus {
    pub state: SyncState,
    pub peers: usize,
}

/// Zdarzenie podsystemu synchronizacji odbierane przez pętlę zdarzeń aplikacji.
#[derive(Debug)]
pub enum SyncEvent {
    Status(SyncStatus),
    IncomingQso(Box<QsoRecord>),
    CatchUpApplied { inserted: usize },
    Log(Msg),
    Error(SyncError),
}

use super::host::SyncHost;
use super::peer::SyncPeer;
use super::store::SyncStore;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

enum ServiceBackend {
    Host(SyncHost),
    Peer(SyncPeer),
}

/// Usługa zarządzająca cyklem życia węzła synchronizacji Multi-Op (Host lub Peer).
pub struct SyncService {
    backend: Option<ServiceBackend>,
    journal: Arc<Mutex<String>>,
    _runtime: Option<Arc<tokio::runtime::Runtime>>,
}

impl SyncService {
    /// Uruchamia usługę synchronizacji zgodnie z przekazaną konfiguracją.
    pub fn start(
        config: SyncConfig,
        store: Arc<dyn SyncStore>,
        events: std::sync::mpsc::Sender<SyncEvent>,
    ) -> Result<Self, SyncError> {
        if config.secret.trim().is_empty() {
            return Err(SyncError::SecretRequired);
        }
        if config.journal.trim().is_empty() {
            return Err(SyncError::JournalRequired);
        }

        let station = if config.station.trim().is_empty() {
            "UNKNOWN".to_string()
        } else {
            config.station.trim().to_uppercase()
        };

        let journal_holder = Arc::new(Mutex::new(config.journal));

        let runtime_holder = if tokio::runtime::Handle::try_current().is_err() {
            Some(Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| SyncError::Io(e.to_string()))?,
            ))
        } else {
            None
        };
        let _guard = runtime_holder.as_ref().map(|rt| rt.enter());

        let backend = match config.role {
            SyncRole::Host { port } => {
                let std_listener = std::net::TcpListener::bind(format!("0.0.0.0:{port}"))
                    .map_err(|e| SyncError::Io(e.to_string()))?;
                std_listener
                    .set_nonblocking(true)
                    .map_err(|e| SyncError::Io(e.to_string()))?;
                let host = SyncHost::from_std_listener(
                    std_listener,
                    station,
                    config.secret,
                    Some(journal_holder.clone()),
                    Some(store),
                    events,
                )?;
                ServiceBackend::Host(host)
            }
            SyncRole::Peer { host, port } => {
                if host.trim().is_empty() {
                    return Err(SyncError::HostRequired);
                }
                let target = format!("{}:{port}", host.trim());
                let peer = SyncPeer::start_with_target(
                    target,
                    station,
                    config.secret,
                    Some(journal_holder.clone()),
                    Some(store),
                    events,
                );
                ServiceBackend::Peer(peer)
            }
        };

        Ok(Self {
            backend: Some(backend),
            journal: journal_holder,
            _runtime: runtime_holder,
        })
    }

    /// Retransmituje lokalne QSO do sieci Multi-Op.
    pub fn broadcast(&self, qso: &QsoRecord) {
        match &self.backend {
            Some(ServiceBackend::Host(h)) => {
                h.broadcast_qso(qso.clone());
            }
            Some(ServiceBackend::Peer(p)) => {
                let _ = p.broadcast_qso(qso.clone());
            }
            None => {}
        }
    }

    /// Aktualizuje nazwę aktywnego dziennika do procedury synchronizacji wstecznej.
    pub fn set_journal(&self, journal: &str) {
        if let Ok(mut j) = self.journal.lock() {
            *j = journal.to_string();
        }
    }

    /// Zatrzymuje działanie usługi synchronizacji i zwalnia zasoby (idempotentne).
    pub fn stop(&mut self) {
        if let Some(backend) = self.backend.take() {
            match backend {
                ServiceBackend::Host(mut h) => h.stop(),
                ServiceBackend::Peer(mut p) => p.stop(),
            }
        }
    }

    /// Zwraca lokalny adres gniazda TCP (gdy usługa działa w roli Hosta).
    pub fn local_addr(&self) -> Option<SocketAddr> {
        match &self.backend {
            Some(ServiceBackend::Host(h)) => Some(h.local_addr()),
            _ => None,
        }
    }
}

impl Drop for SyncService {
    fn drop(&mut self) {
        self.stop();
    }
}
