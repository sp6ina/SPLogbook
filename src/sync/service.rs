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
