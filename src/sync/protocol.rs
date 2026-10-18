// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Definicje struktur i typów wiadomości protokołu synchronizacji v2 (`src/sync/protocol.rs`).

use super::error::SyncError;
use crate::core::qso::{QsoIdentity, QsoRecord};
use serde::{Deserialize, Serialize};

/// Wersja protokołu synchronizacji v2.
pub const PROTOCOL_VERSION: u16 = 2;

/// Unikalny identyfikator nagłówka protokołu ("SPLOGSYNC").
pub const MAGIC: &str = "SPLOGSYNC";

/// Początkowa jawna ramka wysyłana przez klienta (Peer) do serwera (Host).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientHello {
    pub magic: String,
    pub protocol: u16,
    pub station: String,
    pub client_nonce: [u8; 16],
}

/// Jawna ramka odpowiedzi serwera (Host) z wygenerowaną solą oraz noncem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerHello {
    pub protocol: u16,
    pub station: String,
    pub salt: [u8; 16],
    pub server_nonce: [u8; 16],
}

/// Jawna ramka odrzucenia połączenia przy niezgodności wersji lub magica.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reject {
    pub expected: u16,
}

/// Wiadomości przesyłane wewnątrz uwierzytelnionej, szyfrowanej sesji.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    Hello {
        station: String,
    },
    Qso(Box<QsoRecord>),
    Inventory {
        journal: String,
        identities: Vec<QsoIdentity>,
        last: bool,
    },
    Request {
        identities: Vec<QsoIdentity>,
        last: bool,
    },
    Records {
        qsos: Vec<QsoRecord>,
        last: bool,
    },
    Ping,
    Pong,
    Bye,
}

impl Message {
    /// Serializuje wiadomość do postaci bajtów JSON.
    pub fn to_json(&self) -> Result<Vec<u8>, SyncError> {
        serde_json::to_vec(self).map_err(|e| SyncError::Serialization(e.to_string()))
    }

    /// Deserializuje wiadomość z postaci bajtów JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self, SyncError> {
        serde_json::from_slice(bytes).map_err(|e| SyncError::Deserialization(e.to_string()))
    }
}
