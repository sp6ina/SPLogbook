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

/// Docelowy maksymalny rozmiar porcji wiadomości w formacie JSON: 1 MiB (1 048 576 B).
/// Steruje grupowaniem wielu elementów (identyfikatorów QSO lub standardowych rekordów) w pojedynczą wiadomość.
pub const MAX_CHUNK_BYTES: usize = 1024 * 1024; // 1 MiB

/// Bezwzględny maksymalny dopuszczalny rozmiar pojedynczego rekordu QSO w formacie JSON: 6 MiB (6 291 456 B).
/// Rekord o rozmiarze powyżej 1 MiB i do 6 MiB jest legalnie przesyłany jako dedykowany, pojedynczy chunk
/// (\SINGLE_RECORD_EXCEPTION_IS_VALID\), ponieważ wraz z narzutem ramkowania (77-78 B) mieści się z dużym
/// zapasem w twardym limicie transportowym \MAX_FRAME_BYTES\ (8 MiB).
/// Rekordy przekraczające ten limit są odrzucane ze statusem \RecordTooLarge\ bez przerywania sesji.
pub const MAX_RECORD_BYTES: usize = 6 * 1024 * 1024; // 6 MiB

#[cfg(test)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationCounts {
    pub item_serializations: usize,
    pub empty_envelope_serializations: usize,
    pub final_chunk_serializations: usize,
    pub frame_encodings: usize,
    pub frame_encryptions: usize,
    pub validation_serializations: usize,
}

#[cfg(test)]
thread_local! {
    pub static OP_COUNTS: std::cell::Cell<OperationCounts> = const {
        std::cell::Cell::new(OperationCounts {
            item_serializations: 0,
            empty_envelope_serializations: 0,
            final_chunk_serializations: 0,
            frame_encodings: 0,
            frame_encryptions: 0,
            validation_serializations: 0,
        })
    };
}

#[cfg(test)]
pub fn reset_operation_counts() {
    OP_COUNTS.with(|c| c.set(OperationCounts::default()));
}

#[cfg(test)]
pub fn get_operation_counts() -> OperationCounts {
    OP_COUNTS.with(std::cell::Cell::get)
}

#[cfg(test)]
pub fn inc_item_serialization() {
    OP_COUNTS.with(|c| {
        let mut v = c.get();
        v.item_serializations += 1;
        c.set(v);
    });
}

#[cfg(test)]
pub fn inc_empty_envelope_serialization() {
    OP_COUNTS.with(|c| {
        let mut v = c.get();
        v.empty_envelope_serializations += 1;
        c.set(v);
    });
}

#[cfg(test)]
pub fn inc_final_chunk_serialization() {
    OP_COUNTS.with(|c| {
        let mut v = c.get();
        v.final_chunk_serializations += 1;
        c.set(v);
    });
}

#[cfg(test)]
pub fn inc_frame_encoding() {
    OP_COUNTS.with(|c| {
        let mut v = c.get();
        v.frame_encodings += 1;
        c.set(v);
    });
}

#[cfg(test)]
pub fn inc_frame_encryption() {
    OP_COUNTS.with(|c| {
        let mut v = c.get();
        v.frame_encryptions += 1;
        c.set(v);
    });
}

#[cfg(test)]
pub fn inc_validation_serialization() {
    OP_COUNTS.with(|c| {
        let mut v = c.get();
        v.validation_serializations += 1;
        c.set(v);
    });
}

#[cfg(test)]
pub fn reset_serialization_count() {
    reset_operation_counts();
}

#[cfg(test)]
pub fn get_serialization_count() -> usize {
    let counts = get_operation_counts();
    counts.item_serializations + counts.empty_envelope_serializations
}

/// Dzieli listę tożsamości `QsoIdentity` na porcje `Message::Inventory` o rozmiarze <= 1 MiB JSON.
pub fn chunk_inventory(journal: &str, identities: &[QsoIdentity]) -> Vec<Message> {
    if identities.is_empty() {
        return vec![Message::Inventory {
            journal: journal.to_string(),
            identities: Vec::new(),
            last: true,
        }];
    }

    let empty_msg = Message::Inventory {
        journal: journal.to_string(),
        identities: Vec::new(),
        last: false,
    };
    let empty_size = {
        #[cfg(test)]
        inc_empty_envelope_serialization();
        serde_json::to_vec(&empty_msg).map_or(0, |v| v.len())
    };

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_size = empty_size;

    for id in identities {
        let id_size = {
            #[cfg(test)]
            inc_item_serialization();
            serde_json::to_vec(id).map_or(0, |v| v.len())
        };

        let added_size = if current.is_empty() {
            id_size
        } else {
            1 + id_size
        };

        if !current.is_empty() && current_size + added_size > MAX_CHUNK_BYTES {
            chunks.push(Message::Inventory {
                journal: journal.to_string(),
                identities: std::mem::take(&mut current),
                last: false,
            });
            current_size = empty_size;
        }

        let added_size = if current.is_empty() {
            id_size
        } else {
            1 + id_size
        };
        current.push(id.clone());
        current_size += added_size;

        if current_size > MAX_CHUNK_BYTES {
            chunks.push(Message::Inventory {
                journal: journal.to_string(),
                identities: std::mem::take(&mut current),
                last: false,
            });
            current_size = empty_size;
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

    let empty_msg = Message::Request {
        identities: Vec::new(),
        last: false,
    };
    let empty_size = {
        #[cfg(test)]
        inc_empty_envelope_serialization();
        serde_json::to_vec(&empty_msg).map_or(0, |v| v.len())
    };

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_size = empty_size;

    for id in identities {
        let id_size = {
            #[cfg(test)]
            inc_item_serialization();
            serde_json::to_vec(id).map_or(0, |v| v.len())
        };

        let added_size = if current.is_empty() {
            id_size
        } else {
            1 + id_size
        };

        if !current.is_empty() && current_size + added_size > MAX_CHUNK_BYTES {
            chunks.push(Message::Request {
                identities: std::mem::take(&mut current),
                last: false,
            });
            current_size = empty_size;
        }

        let added_size = if current.is_empty() {
            id_size
        } else {
            1 + id_size
        };
        current.push(id.clone());
        current_size += added_size;

        if current_size > MAX_CHUNK_BYTES {
            chunks.push(Message::Request {
                identities: std::mem::take(&mut current),
                last: false,
            });
            current_size = empty_size;
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

    let empty_msg = Message::Records {
        qsos: Vec::new(),
        last: false,
    };
    let empty_size = {
        #[cfg(test)]
        inc_empty_envelope_serialization();
        serde_json::to_vec(&empty_msg).map_or(0, |v| v.len())
    };

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_size = empty_size;

    for rec in records {
        let rec_size = {
            #[cfg(test)]
            inc_item_serialization();
            serde_json::to_vec(rec).map_or(0, |v| v.len())
        };

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

        let added_size = if current.is_empty() {
            rec_size
        } else {
            1 + rec_size
        };

        if !current.is_empty() && current_size + added_size > MAX_CHUNK_BYTES {
            chunks.push(Message::Records {
                qsos: std::mem::take(&mut current),
                last: false,
            });
            current_size = empty_size;
        }

        let added_size = if current.is_empty() {
            rec_size
        } else {
            1 + rec_size
        };
        current.push(rec.clone());
        current_size += added_size;

        if current_size > MAX_CHUNK_BYTES {
            chunks.push(Message::Records {
                qsos: std::mem::take(&mut current),
                last: false,
            });
            current_size = empty_size;
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

#[cfg(test)]
pub fn chunk_inventory_reference(journal: &str, identities: &[QsoIdentity]) -> Vec<Message> {
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

#[cfg(test)]
pub fn chunk_requests_reference(identities: &[QsoIdentity]) -> Vec<Message> {
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

#[cfg(test)]
pub fn chunk_records_reference(
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
