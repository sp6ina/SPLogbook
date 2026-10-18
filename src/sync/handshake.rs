// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Uwierzytelniony handshake protokołu synchronizacji v2 (`src/sync/handshake.rs`).

use super::crypto::{
    DIRECTION_HOST_TO_PEER, DIRECTION_PEER_TO_HOST, KEY_LEN, SALT_LEN, compute_session_id,
    derive_key, random_nonce_16,
};
use super::error::SyncError;
use super::frame::{read_frame, write_frame};
use super::protocol::{ClientHello, MAGIC, Message, PROTOCOL_VERSION, Reject, ServerHello};
use super::session::SyncSession;
use std::time::Duration;
use tokio::net::TcpStream;

/// Wynik udanego nawiązania sesji po obu stronach połączenia.
pub struct HandshakeResult {
    pub session: SyncSession,
    pub remote_station: String,
}

/// Wykonuje handshake po stronie Hosta (serwera).
/// Przyjmuje połączenie TCP, wstępnie wyliczony klucz serwera i sól serwera.
pub async fn host_handshake(
    mut stream: TcpStream,
    server_station: &str,
    server_salt: &[u8; SALT_LEN],
    server_key: &[u8; KEY_LEN],
) -> Result<HandshakeResult, SyncError> {
    // 1. Odbiór ClientHello
    let client_hello_bytes = read_frame(&mut stream).await?;
    let client_hello: ClientHello = if let Ok(h) = serde_json::from_slice(&client_hello_bytes) {
        h
    } else {
        let reject = Reject {
            expected: PROTOCOL_VERSION,
        };
        if let Ok(rej_bytes) = serde_json::to_vec(&reject) {
            let _ = write_frame(&mut stream, &rej_bytes).await;
        }
        return Err(SyncError::InvalidMagic);
    };

    if client_hello.magic != MAGIC {
        let reject = Reject {
            expected: PROTOCOL_VERSION,
        };
        if let Ok(rej_bytes) = serde_json::to_vec(&reject) {
            let _ = write_frame(&mut stream, &rej_bytes).await;
        }
        return Err(SyncError::InvalidMagic);
    }

    if client_hello.protocol != PROTOCOL_VERSION {
        let reject = Reject {
            expected: PROTOCOL_VERSION,
        };
        if let Ok(rej_bytes) = serde_json::to_vec(&reject) {
            let _ = write_frame(&mut stream, &rej_bytes).await;
        }
        return Err(SyncError::ProtocolMismatch {
            expected: PROTOCOL_VERSION,
            got: client_hello.protocol,
        });
    }

    // 2. Wysłanie ServerHello
    let server_nonce = random_nonce_16();
    let server_hello = ServerHello {
        protocol: PROTOCOL_VERSION,
        station: server_station.to_string(),
        salt: *server_salt,
        server_nonce,
    };
    let server_hello_bytes =
        serde_json::to_vec(&server_hello).map_err(|e| SyncError::Serialization(e.to_string()))?;
    write_frame(&mut stream, &server_hello_bytes).await?;

    // 3. Obliczenie session_id
    let session_id = compute_session_id(&client_hello.client_nonce, &server_nonce);

    // 4. Inicjalizacja sesji: Host wysyła z DIRECTION_HOST_TO_PEER, odbiera z DIRECTION_PEER_TO_HOST
    let mut session = SyncSession::new(
        stream,
        *server_key,
        session_id,
        DIRECTION_HOST_TO_PEER,
        DIRECTION_PEER_TO_HOST,
        client_hello.station.clone(),
    );

    // 5. Oczekiwanie na pierwszą szyfrowaną wiadomość klienta: Message::Hello
    if let Ok(Message::Hello { station }) = session.recv().await {
        Ok(HandshakeResult {
            session,
            remote_station: station,
        })
    } else {
        // Złe hasło lub uszkodzony pakiet: odczekaj 1 sekundę i rozłącz bez odpowiedzi
        tokio::time::sleep(Duration::from_secs(1)).await;
        Err(SyncError::DecryptionFailed)
    }
}

/// Wykonuje handshake po stronie Peera (klienta).
pub async fn peer_handshake(
    mut stream: TcpStream,
    client_station: &str,
    secret: &str,
) -> Result<HandshakeResult, SyncError> {
    if secret.trim().is_empty() {
        return Err(SyncError::SecretRequired);
    }

    // 1. Wysłanie ClientHello
    let client_nonce = random_nonce_16();
    let client_hello = ClientHello {
        magic: MAGIC.to_string(),
        protocol: PROTOCOL_VERSION,
        station: client_station.to_string(),
        client_nonce,
    };
    let client_hello_bytes =
        serde_json::to_vec(&client_hello).map_err(|e| SyncError::Serialization(e.to_string()))?;
    write_frame(&mut stream, &client_hello_bytes).await?;

    // 2. Odbiór ServerHello lub Reject
    let response_bytes = read_frame(&mut stream).await?;

    if let Ok(reject) = serde_json::from_slice::<Reject>(&response_bytes) {
        return Err(SyncError::ProtocolMismatch {
            expected: reject.expected,
            got: PROTOCOL_VERSION,
        });
    }

    let server_hello: ServerHello =
        serde_json::from_slice(&response_bytes).map_err(|_| SyncError::InvalidMagic)?;

    if server_hello.protocol != PROTOCOL_VERSION {
        return Err(SyncError::ProtocolMismatch {
            expected: PROTOCOL_VERSION,
            got: server_hello.protocol,
        });
    }

    // 3. Wyprowadzenie klucza z soli serwera
    let key = derive_key(secret, &server_hello.salt)?;
    let session_id = compute_session_id(&client_nonce, &server_hello.server_nonce);

    // 4. Inicjalizacja sesji: Peer wysyła z DIRECTION_PEER_TO_HOST, odbiera z DIRECTION_HOST_TO_PEER
    let session = SyncSession::new(
        stream,
        key,
        session_id,
        DIRECTION_PEER_TO_HOST,
        DIRECTION_HOST_TO_PEER,
        server_hello.station.clone(),
    );

    // 5. Wysłanie pierwszej szyfrowanej wiadomości Hello
    session
        .send(Message::Hello {
            station: client_station.to_string(),
        })
        .await?;

    Ok(HandshakeResult {
        session,
        remote_station: server_hello.station,
    })
}
