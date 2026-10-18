// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Typy błędów domenowych podsystemu synchronizacji (`src/sync/error.rs`).

use crate::core::i18n::{Msg, key};

/// Błąd domenowy protokołu synchronizacji Host/Peer (SYNC-1..SYNC-7).
#[derive(Debug)]
pub enum SyncError {
    SecretRequired,
    HostRequired,
    JournalRequired,
    KeyDerivationFailed(String),
    EncryptionFailed,
    DecryptionFailed,
    InvalidNonce,
    FrameTooLarge { size: usize, limit: usize },
    FrameTruncated,
    InvalidMagic,
    ProtocolMismatch { expected: u16, got: u16 },
    CounterMismatch { expected: u64, got: u64 },
    CounterReplay,
    AadMismatch,
    DirectionMismatch,
    Serialization(String),
    Deserialization(String),
    Io(String),
    ClientLimitReached,
    QueueFull,
    Timeout,
    ConnectionClosed,
    RecordTooLarge { size: usize, limit: usize },
    Store(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SecretRequired => write!(f, "Shared synchronization secret is required"),
            Self::HostRequired => write!(f, "Host address is required in client mode"),
            Self::JournalRequired => {
                write!(f, "Active journal name is required for synchronization")
            }
            Self::KeyDerivationFailed(e) => write!(f, "Key derivation failed: {e}"),
            Self::EncryptionFailed => write!(f, "Payload encryption failed"),
            Self::DecryptionFailed => {
                write!(
                    f,
                    "Payload decryption failed (invalid key or corrupted data)"
                )
            }
            Self::InvalidNonce => write!(f, "Invalid cryptographic nonce"),
            Self::FrameTooLarge { size, limit } => {
                write!(f, "Frame size ({size} B) exceeds maximum limit ({limit} B)")
            }
            Self::FrameTruncated => write!(f, "Frame payload was truncated unexpectedly"),
            Self::InvalidMagic => write!(f, "Invalid synchronization protocol magic header"),
            Self::ProtocolMismatch { expected, got } => {
                write!(
                    f,
                    "Protocol version mismatch: expected {expected}, got {got}"
                )
            }
            Self::CounterMismatch { expected, got } => {
                write!(
                    f,
                    "Message counter mismatch: expected {expected}, got {got}"
                )
            }
            Self::CounterReplay => write!(f, "Replayed message counter detected"),
            Self::AadMismatch => write!(f, "AAD verification failed during frame decryption"),
            Self::DirectionMismatch => write!(f, "Message direction mismatch in AAD"),
            Self::Serialization(e) => write!(f, "Message serialization error: {e}"),
            Self::Deserialization(e) => write!(f, "Message deserialization error: {e}"),
            Self::Io(e) => write!(f, "Network I/O error: {e}"),
            Self::ClientLimitReached => write!(f, "Maximum connected peer limit (32) reached"),
            Self::QueueFull => write!(f, "Client send queue is full (slow peer disconnected)"),
            Self::Timeout => write!(f, "Frame timeout exceeded (no frames received for 45s)"),
            Self::ConnectionClosed => write!(f, "Connection closed by remote peer"),
            Self::RecordTooLarge { size, limit } => {
                write!(
                    f,
                    "QSO record exceeds maximum allowed size ({size} B > {limit} B)"
                )
            }
            Self::Store(e) => write!(f, "Sync storage error: {e}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl SyncError {
    /// Zwraca zlokalizowany komunikat przeznaczony dla interfejsu graficznego.
    pub fn msg(&self) -> Msg {
        match self {
            Self::SecretRequired => Msg::new(key::MULTIOP_ERROR_SECRET_REQUIRED),
            Self::HostRequired => Msg::new(key::MULTIOP_ERROR_HOST_REQUIRED),
            Self::JournalRequired => Msg::new(key::MULTIOP_ERROR_JOURNAL_REQUIRED),
            Self::KeyDerivationFailed(_)
            | Self::EncryptionFailed
            | Self::DecryptionFailed
            | Self::InvalidNonce => Msg::new(key::MULTIOP_ERROR_AUTH_FAILED),
            Self::FrameTooLarge { size, limit } => Msg::new(key::MULTIOP_ERROR_FRAME_TOO_LARGE)
                .arg("size", size)
                .arg("limit", limit),
            Self::FrameTruncated => Msg::new(key::MULTIOP_ERROR_FRAME_TRUNCATED),
            Self::InvalidMagic => Msg::new(key::MULTIOP_ERROR_INVALID_MAGIC),
            Self::ProtocolMismatch { expected, got } => {
                Msg::new(key::MULTIOP_ERROR_PROTOCOL_MISMATCH)
                    .arg("expected", expected)
                    .arg("got", got)
            }
            Self::CounterMismatch { expected, got } => {
                Msg::new(key::MULTIOP_ERROR_COUNTER_MISMATCH)
                    .arg("expected", expected)
                    .arg("got", got)
            }
            Self::CounterReplay => Msg::new(key::MULTIOP_ERROR_COUNTER_REPLAY),
            Self::AadMismatch => Msg::new(key::MULTIOP_ERROR_AAD_MISMATCH),
            Self::DirectionMismatch => Msg::new(key::MULTIOP_ERROR_DIRECTION_MISMATCH),
            Self::Serialization(e) => Msg::new(key::MULTIOP_ERROR_SERIALIZATION).arg("error", e),
            Self::Deserialization(e) => {
                Msg::new(key::MULTIOP_ERROR_DESERIALIZATION).arg("error", e)
            }
            Self::Io(e) => Msg::new(key::MULTIOP_ERROR_IO).arg("error", e),
            Self::ClientLimitReached => Msg::new(key::MULTIOP_ERROR_CLIENT_LIMIT),
            Self::QueueFull => Msg::new(key::MULTIOP_ERROR_QUEUE_FULL),
            Self::Timeout => Msg::new(key::MULTIOP_ERROR_TIMEOUT),
            Self::ConnectionClosed => Msg::new(key::MULTIOP_ERROR_CONNECTION_CLOSED),
            Self::RecordTooLarge { size, limit } => Msg::new(key::MULTIOP_ERROR_RECORD_TOO_LARGE)
                .arg("size", size)
                .arg("limit", limit),
            Self::Store(e) => Msg::new(key::MULTIOP_ERROR_STORE).arg("error", e),
        }
    }
}

impl From<std::io::Error> for SyncError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for SyncError {
    fn from(err: serde_json::Error) -> Self {
        Self::Deserialization(err.to_string())
    }
}
