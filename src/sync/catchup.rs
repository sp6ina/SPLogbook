// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Mechanizm wyrównywania brakujących łączności Catch-Up (`src/sync/catchup.rs`).

use super::error::SyncError;
use super::protocol::Message;
use super::service::SyncEvent;
use super::session::SessionSender;
use super::store::SyncStore;
use crate::core::i18n::{Msg, key};
use crate::core::qso::{QsoIdentity, QsoRecord};
use std::collections::HashSet;
use std::sync::Arc;

pub const MAX_CHUNK_BYTES: usize = 1024 * 1024; // 1 MiB
pub const MAX_RECORD_BYTES: usize = 6 * 1024 * 1024; // 6 MiB

/// Dzieli listę tożsamości `QsoIdentity` na porcje `Message::Inventory` o rozmiarze <= 1 MiB JSON.
pub fn chunk_inventory(journal: &str, identities: &[QsoIdentity]) -> Vec<Message> {
    if identities.is_empty() {
        return vec![Message::Inventory {
            journal: journal.to_string(),
            identities: Vec::new(),
            last: true,
        }];
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();

    for id in identities {
        current.push(id.clone());
        let msg = Message::Inventory {
            journal: journal.to_string(),
            identities: current.clone(),
            last: false,
        };
        if serde_json::to_vec(&msg).map_or(0, |v| v.len()) > MAX_CHUNK_BYTES {
            current.pop();
            if !current.is_empty() {
                chunks.push(Message::Inventory {
                    journal: journal.to_string(),
                    identities: std::mem::take(&mut current),
                    last: false,
                });
            }
            current.push(id.clone());
        }
    }

    if !current.is_empty() {
        chunks.push(Message::Inventory {
            journal: journal.to_string(),
            identities: current,
            last: true,
        });
    } else if let Some(Message::Inventory { last, .. }) = chunks.last_mut() {
        *last = true;
    }

    chunks
}

/// Dzieli listę tożsamości `QsoIdentity` na porcje `Message::Request` o rozmiarze <= 1 MiB JSON.
pub fn chunk_requests(identities: &[QsoIdentity]) -> Vec<Message> {
    if identities.is_empty() {
        return vec![Message::Request {
            identities: Vec::new(),
            last: true,
        }];
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();

    for id in identities {
        current.push(id.clone());
        let msg = Message::Request {
            identities: current.clone(),
            last: false,
        };
        if serde_json::to_vec(&msg).map_or(0, |v| v.len()) > MAX_CHUNK_BYTES {
            current.pop();
            if !current.is_empty() {
                chunks.push(Message::Request {
                    identities: std::mem::take(&mut current),
                    last: false,
                });
            }
            current.push(id.clone());
        }
    }

    if !current.is_empty() {
        chunks.push(Message::Request {
            identities: current,
            last: true,
        });
    } else if let Some(Message::Request { last, .. }) = chunks.last_mut() {
        *last = true;
    }

    chunks
}

/// Dzieli listę rekordów `QsoRecord` na porcje `Message::Records` o rozmiarze <= 1 MiB JSON.
/// Rekordy przekraczające limit 6 MiB są pomijane i generują błąd/log bez przerywania synchronizacji.
pub fn chunk_records(
    records: &[QsoRecord],
    events: Option<&std::sync::mpsc::Sender<SyncEvent>>,
) -> Vec<Message> {
    if records.is_empty() {
        return vec![Message::Records {
            qsos: Vec::new(),
            last: true,
        }];
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();

    for rec in records {
        let rec_size = serde_json::to_vec(rec).map_or(0, |v| v.len());
        if rec_size > MAX_RECORD_BYTES {
            if let Some(tx) = events {
                let _ = tx.send(SyncEvent::Error(SyncError::RecordTooLarge {
                    size: rec_size,
                    limit: MAX_RECORD_BYTES,
                }));
                let _ = tx.send(SyncEvent::Log(Msg::new(
                    key::MULTIOP_ERROR_RECORD_TOO_LARGE,
                )));
            }
            continue;
        }

        current.push(rec.clone());
        let msg = Message::Records {
            qsos: current.clone(),
            last: false,
        };
        if serde_json::to_vec(&msg).map_or(0, |v| v.len()) > MAX_CHUNK_BYTES {
            current.pop();
            if !current.is_empty() {
                chunks.push(Message::Records {
                    qsos: std::mem::take(&mut current),
                    last: false,
                });
            }
            current.push(rec.clone());
        }
    }

    if !current.is_empty() {
        chunks.push(Message::Records {
            qsos: current,
            last: true,
        });
    } else if let Some(Message::Records { last, .. }) = chunks.last_mut() {
        *last = true;
    } else {
        chunks.push(Message::Records {
            qsos: Vec::new(),
            last: true,
        });
    }

    chunks
}

/// Zarządca stanu procedury Catch-Up dla pojedynczej sesji połączenia.
pub struct CatchUpHandler {
    journal: String,
    store: Arc<dyn SyncStore>,
    events: std::sync::mpsc::Sender<SyncEvent>,
    peer_inventory_accum: HashSet<QsoIdentity>,
    peer_inventory_complete: bool,
    peer_requests_accum: HashSet<QsoIdentity>,
    peer_requests_complete: bool,
}

impl CatchUpHandler {
    /// Tworzy nowy CatchUpHandler dla wskazanego aktywnego dziennika i magazynu.
    pub fn new(
        journal: String,
        store: Arc<dyn SyncStore>,
        events: std::sync::mpsc::Sender<SyncEvent>,
    ) -> Self {
        Self {
            journal,
            store,
            events,
            peer_inventory_accum: HashSet::new(),
            peer_inventory_complete: false,
            peer_requests_accum: HashSet::new(),
            peer_requests_complete: false,
        }
    }

    /// Zwraca aktywny dziennik synchronizacji.
    pub fn journal(&self) -> &str {
        &self.journal
    }

    /// Inicjuje wysłanie lokalnego inwentarza aktywnego dziennika do peera.
    pub fn send_inventory(&self, sender: &SessionSender) -> Result<(), SyncError> {
        let local_inv = self.store.inventory(&self.journal)?;
        let identities: Vec<QsoIdentity> = local_inv.into_iter().collect();
        let chunks = chunk_inventory(&self.journal, &identities);
        for chunk in chunks {
            sender.try_send(chunk)?;
        }
        Ok(())
    }

    /// Obsługuje odebraną porcję `Message::Inventory`.
    /// Po odebraniu ostatniej porcji (`last: true`) wylicza brakujące lokalnie rekordy i wysyła `Request`.
    pub fn handle_inventory(
        &mut self,
        journal: &str,
        identities: Vec<QsoIdentity>,
        last: bool,
        sender: &SessionSender,
    ) -> Result<(), SyncError> {
        // Ignorowanie inwentarza innego dziennika
        if journal != self.journal {
            return Ok(());
        }

        self.peer_inventory_accum.extend(identities);

        if last {
            self.peer_inventory_complete = true;
            let local_inv = self.store.inventory(&self.journal)?;
            let wanted: Vec<QsoIdentity> = self
                .peer_inventory_accum
                .difference(&local_inv)
                .cloned()
                .collect();

            let chunks = chunk_requests(&wanted);
            for chunk in chunks {
                sender.try_send(chunk)?;
            }
        }

        Ok(())
    }

    /// Obsługuje odebraną porcję `Message::Request`.
    /// Po odebraniu ostatniej porcji (`last: true`) pobiera żądane rekordy z magazynu i wysyła je w porcjach `Records`.
    pub fn handle_request(
        &mut self,
        identities: Vec<QsoIdentity>,
        last: bool,
        sender: &SessionSender,
    ) -> Result<(), SyncError> {
        self.peer_requests_accum.extend(identities);

        if last {
            self.peer_requests_complete = true;
            let records = self
                .store
                .records_for(&self.journal, &self.peer_requests_accum)?;

            let chunks = chunk_records(&records, Some(&self.events));
            for chunk in chunks {
                sender.try_send(chunk)?;
            }
        }

        Ok(())
    }

    /// Obsługuje odebraną porcję `Message::Records` i importuje ją do magazynu aktywnego dziennika.
    pub fn handle_records(&mut self, qsos: Vec<QsoRecord>) -> Result<usize, SyncError> {
        if qsos.is_empty() {
            return Ok(0);
        }
        let inserted = self.store.import(&self.journal, qsos)?;
        if inserted > 0 {
            let _ = self.events.send(SyncEvent::CatchUpApplied { inserted });
        }
        Ok(inserted)
    }

    /// Sprawdza, czy odebrano kompletny inwentarz od peera.
    pub fn is_peer_inventory_complete(&self) -> bool {
        self.peer_inventory_complete
    }

    /// Sprawdza, czy odebrano kompletne żądanie od peera.
    pub fn is_peer_requests_complete(&self) -> bool {
        self.peer_requests_complete
    }
}
