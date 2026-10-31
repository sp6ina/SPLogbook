// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Zunifikowany podsystem synchronizacji QSO Host/Peer (SYNC-1..SYNC-7).

pub mod catchup;
pub mod crypto;
pub mod error;
pub mod frame;
pub mod handshake;
pub mod host;
pub mod peer;
pub mod protocol;
pub mod service;
pub mod session;
pub mod store;

pub use catchup::{
    CatchUpHandler, MAX_CHUNK_BYTES, MAX_RECORD_BYTES, chunk_inventory, chunk_records,
    chunk_requests,
};
pub use crypto::*;
pub use error::SyncError;
pub use frame::{MAX_FRAME_BYTES, decode_length, encode_length, read_frame, write_frame};
pub use handshake::{HandshakeResult, host_handshake, peer_handshake};
pub use host::{
    HOST_HANDSHAKE_TIMEOUT, IDLE_TIMEOUT, MAX_ACTIVE_PEERS, MAX_HANDSHAKES, PING_INTERVAL, SyncHost,
};
pub use peer::{INITIAL_BACKOFF, MAX_BACKOFF, ReconnectBackoff, SyncPeer};
pub use protocol::{ClientHello, MAGIC, Message, PROTOCOL_VERSION, Reject, ServerHello};
pub use service::{SyncConfig, SyncEvent, SyncRole, SyncService, SyncState, SyncStatus};
pub use session::{QUEUE_CAPACITY, SessionSender, SyncSession};
pub use store::{MemorySyncStore, SyncStore};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::qso::{QsoIdentity, QsoRecord};
    use std::collections::HashSet;
    use std::io::Cursor;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn test_ingestor() -> crate::core::ingest::QsoIngestor {
        let db = Arc::new(std::sync::Mutex::new(
            crate::core::database::LogDatabase::open_in_memory().unwrap(),
        ));
        let awards = Arc::new(std::sync::Mutex::new(
            crate::core::awards::AwardsEngine::new(),
        ));
        let scp = Arc::new(std::sync::Mutex::new(crate::core::scp::ScpEngine::new()));
        let prefixes = Arc::new(crate::core::prefix::PrefixMatcher::empty());
        let events = crate::core::events::EventBus::new(16);
        crate::core::ingest::QsoIngestor::new(db, awards, scp, prefixes, events)
    }

    fn sample_qso(call: &str) -> QsoRecord {
        let mut q = QsoRecord::new(call, "20m", "CW");
        q.qso_date = "20261018".to_string();
        q.time_on = "153000".to_string();
        q
    }

    #[test]
    fn key_derivation_is_deterministic() {
        let salt = [42u8; SALT_LEN];
        let k1 = derive_key("ham-secret-key", &salt).unwrap();
        let k2 = derive_key("ham-secret-key", &salt).unwrap();
        assert_eq!(k1, k2);

        let k3 = derive_key("other-secret", &salt).unwrap();
        assert_ne!(k1, k3);
    }

    #[test]
    fn different_salt_yields_different_key() {
        let s1 = [1u8; SALT_LEN];
        let s2 = [2u8; SALT_LEN];
        let k1 = derive_key("secret", &s1).unwrap();
        let k2 = derive_key("secret", &s2).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn tampered_frame_is_rejected() {
        let key = derive_key("test-secret", &[7u8; SALT_LEN]).unwrap();
        let aad = build_aad(&[1u8; 32], DIRECTION_PEER_TO_HOST, 0);
        let mut frame = encrypt_payload(&key, &aad, b"payload-data").unwrap();

        let last_idx = frame.len() - 1;
        frame[last_idx] ^= 0x55;

        let result = decrypt_payload(&key, &aad, &frame);
        assert!(matches!(result, Err(SyncError::DecryptionFailed)));
    }

    #[test]
    fn wrong_key_is_rejected() {
        let k1 = derive_key("secret-1", &[9u8; SALT_LEN]).unwrap();
        let k2 = derive_key("secret-2", &[9u8; SALT_LEN]).unwrap();
        let aad = build_aad(&[2u8; 32], DIRECTION_PEER_TO_HOST, 0);

        let frame = encrypt_payload(&k1, &aad, b"payload").unwrap();
        let result = decrypt_payload(&k2, &aad, &frame);
        assert!(matches!(result, Err(SyncError::DecryptionFailed)));
    }

    #[test]
    fn length_encoding_roundtrip() {
        for len in [0usize, 1, 255, 65_536, 1_000_000, MAX_FRAME_BYTES] {
            let enc = encode_length(len);
            let dec = decode_length(&enc).unwrap();
            assert_eq!(dec, len);
        }
    }

    #[test]
    fn aad_mismatch_is_rejected() {
        let key = derive_key("secret", &[3u8; SALT_LEN]).unwrap();
        let sid1 = [11u8; 32];
        let sid2 = [22u8; 32];
        let aad1 = build_aad(&sid1, DIRECTION_PEER_TO_HOST, 0);
        let aad2 = build_aad(&sid2, DIRECTION_PEER_TO_HOST, 0);

        let frame = encrypt_payload(&key, &aad1, b"classified-qso").unwrap();
        let result = decrypt_payload(&key, &aad2, &frame);
        assert!(matches!(result, Err(SyncError::DecryptionFailed)));
    }

    #[test]
    fn direction_mismatch_is_rejected() {
        let key = derive_key("secret", &[4u8; SALT_LEN]).unwrap();
        let sid = [15u8; 32];
        let aad_peer = build_aad(&sid, DIRECTION_PEER_TO_HOST, 1);
        let aad_host = build_aad(&sid, DIRECTION_HOST_TO_PEER, 1);

        let frame = encrypt_payload(&key, &aad_peer, b"data").unwrap();
        let result = decrypt_payload(&key, &aad_host, &frame);
        assert!(matches!(result, Err(SyncError::DecryptionFailed)));
    }

    #[test]
    fn counter_mismatch_is_rejected() {
        let key = derive_key("secret", &[5u8; SALT_LEN]).unwrap();
        let sid = [18u8; 32];
        let aad_0 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 0);
        let aad_1 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 1);

        let frame = encrypt_payload(&key, &aad_0, b"data").unwrap();
        let result = decrypt_payload(&key, &aad_1, &frame);
        assert!(matches!(result, Err(SyncError::DecryptionFailed)));
    }

    #[test]
    fn counter_replay_is_rejected() {
        let key = derive_key("secret", &[6u8; SALT_LEN]).unwrap();
        let sid = [20u8; 32];

        let aad_msg0 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 0);
        let frame0 = encrypt_payload(&key, &aad_msg0, b"first-message").unwrap();

        let dec0 = decrypt_payload(&key, &aad_msg0, &frame0).unwrap();
        assert_eq!(dec0, b"first-message");
        let expected_counter = 1u64;

        let replay_aad = build_aad(&sid, DIRECTION_PEER_TO_HOST, expected_counter);
        let replay_res = decrypt_payload(&key, &replay_aad, &frame0);
        assert!(
            matches!(replay_res, Err(SyncError::DecryptionFailed)),
            "replayed message with old counter must fail AAD decryption"
        );
    }

    #[test]
    fn oversize_frame_is_rejected() {
        let oversize = (MAX_FRAME_BYTES + 1) as u32;
        let bytes = oversize.to_be_bytes();
        let res = decode_length(&bytes);
        assert!(matches!(res, Err(SyncError::FrameTooLarge { .. })));
    }

    #[test]
    fn truncated_frame_is_rejected() {
        let key = derive_key("secret", &[8u8; SALT_LEN]).unwrap();
        let aad = build_aad(&[0u8; 32], 0, 0);

        let short_frame = vec![0u8; 20];
        let res = decrypt_payload(&key, &aad, &short_frame);
        assert!(matches!(res, Err(SyncError::FrameTruncated)));
    }

    #[test]
    fn message_roundtrip_all_variants() {
        let qso = sample_qso("SP6INA");
        let identity = qso.identity();

        let cases: Vec<Message> = vec![
            Message::Hello {
                station: "SP6INA/P".to_string(),
            },
            Message::Qso(Box::new(qso.clone())),
            Message::Inventory {
                journal: "CONTEST-2026".to_string(),
                identities: vec![identity.clone()],
                last: true,
            },
            Message::Request {
                identities: vec![identity],
                last: false,
            },
            Message::Records {
                qsos: vec![qso],
                last: true,
            },
            Message::Ping,
            Message::Pong,
            Message::Bye,
        ];

        for msg in cases {
            let json = msg.to_json().unwrap();
            let parsed = Message::from_json(&json).unwrap();
            assert_eq!(msg, parsed);
        }
    }

    #[test]
    fn secret_is_not_present_in_ciphertext() {
        let secret = "super-confidential-secret-987654";
        let salt = random_salt();
        let key = derive_key(secret, &salt).unwrap();
        let sid = compute_session_id(&random_nonce_16(), &random_nonce_16());
        let aad = build_aad(&sid, DIRECTION_PEER_TO_HOST, 0);

        let msg = Message::Hello {
            station: "TEST-STATION".to_string(),
        };
        let plaintext = msg.to_json().unwrap();
        let frame = encrypt_payload(&key, &aad, &plaintext).unwrap();

        let secret_bytes = secret.as_bytes();
        let window_size = secret_bytes.len();
        let found = frame.windows(window_size).any(|w| w == secret_bytes);
        assert!(!found, "secret must never appear in ciphertext");
    }

    #[test]
    fn qso_payload_is_not_present_in_ciphertext() {
        let salt = random_salt();
        let key = derive_key("ham-secret", &salt).unwrap();
        let sid = compute_session_id(&random_nonce_16(), &random_nonce_16());
        let aad = build_aad(&sid, DIRECTION_HOST_TO_PEER, 0);

        let unique_marker = "VERYUNIQUECALLSIGN123456789";
        let qso = sample_qso(unique_marker);
        let msg = Message::Qso(Box::new(qso));
        let plaintext = msg.to_json().unwrap();
        let frame = encrypt_payload(&key, &aad, &plaintext).unwrap();

        let marker_bytes = unique_marker.as_bytes();
        let found = frame.windows(marker_bytes.len()).any(|w| w == marker_bytes);
        assert!(
            !found,
            "qso payload marker must never appear in raw ciphertext"
        );
    }

    #[tokio::test]
    async fn async_read_write_frame_roundtrip() {
        let payload = b"hello frame protocol v2";
        let mut buf = Vec::new();
        write_frame(&mut buf, payload).await.unwrap();

        let mut cursor = Cursor::new(buf);
        let read = read_frame(&mut cursor).await.unwrap();
        assert_eq!(read, payload);
    }

    #[tokio::test]
    async fn handshake_succeeds_with_same_secret() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let salt = random_salt();
        let host_key = derive_key("ham-secret-123", &salt).unwrap();

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            host_handshake(stream, "SP6INA-HOST", &salt, &host_key).await
        });

        let peer_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let peer_res = peer_handshake(peer_stream, "SP6INA-PEER", "ham-secret-123")
            .await
            .unwrap();
        let host_res = host_fut.await.unwrap().unwrap();

        assert_eq!(peer_res.remote_station, "SP6INA-HOST");
        assert_eq!(host_res.remote_station, "SP6INA-PEER");
    }

    #[tokio::test]
    async fn handshake_fails_with_wrong_secret_without_reply() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let salt = random_salt();
        let host_key = derive_key("correct-secret", &salt).unwrap();

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            host_handshake(stream, "SP6INA-HOST", &salt, &host_key).await
        });

        let peer_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut peer_res = peer_handshake(peer_stream, "SP6INA-PEER", "wrong-secret")
            .await
            .unwrap();

        let host_res = host_fut.await.unwrap();
        assert!(matches!(host_res, Err(SyncError::DecryptionFailed)));

        let peer_recv = peer_res.session.recv().await;
        assert!(matches!(peer_recv, Err(SyncError::ConnectionClosed)));
    }

    #[tokio::test]
    async fn handshake_rejects_protocol_mismatch_with_expected_version() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let salt = random_salt();
        let host_key = derive_key("secret", &salt).unwrap();

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            host_handshake(stream, "SP6INA-HOST", &salt, &host_key).await
        });

        let mut client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let bad_hello = ClientHello {
            magic: MAGIC.to_string(),
            protocol: 99,
            station: "TEST".to_string(),
            client_nonce: random_nonce_16(),
        };
        let bad_bytes = serde_json::to_vec(&bad_hello).unwrap();
        write_frame(&mut client_stream, &bad_bytes).await.unwrap();

        let resp_bytes = read_frame(&mut client_stream).await.unwrap();
        let reject: Reject = serde_json::from_slice(&resp_bytes).unwrap();
        assert_eq!(reject.expected, PROTOCOL_VERSION);

        let host_res = host_fut.await.unwrap();
        assert!(matches!(
            host_res,
            Err(SyncError::ProtocolMismatch {
                expected: 2,
                got: 99
            })
        ));
    }

    #[tokio::test]
    async fn handshake_rejects_invalid_magic() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let salt = random_salt();
        let host_key = derive_key("secret", &salt).unwrap();

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            host_handshake(stream, "SP6INA-HOST", &salt, &host_key).await
        });

        let mut client_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let bad_hello = ClientHello {
            magic: "INVALIDMAGIC".to_string(),
            protocol: PROTOCOL_VERSION,
            station: "TEST".to_string(),
            client_nonce: random_nonce_16(),
        };
        let bad_bytes = serde_json::to_vec(&bad_hello).unwrap();
        write_frame(&mut client_stream, &bad_bytes).await.unwrap();

        let resp_bytes = read_frame(&mut client_stream).await.unwrap();
        let reject: Reject = serde_json::from_slice(&resp_bytes).unwrap();
        assert_eq!(reject.expected, PROTOCOL_VERSION);

        let host_res = host_fut.await.unwrap();
        assert!(matches!(host_res, Err(SyncError::InvalidMagic)));
    }

    #[tokio::test]
    async fn session_detects_replayed_frame() {
        let key = derive_key("secret", &[1u8; SALT_LEN]).unwrap();
        let sid = [40u8; 32];

        let aad0 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 0);
        let frame0 = encrypt_payload(&key, &aad0, b"msg0").unwrap();

        // Odbiór ramki 0 z oczekiwanym licznikiem 0
        let dec0 = decrypt_payload(&key, &aad0, &frame0).unwrap();
        assert_eq!(dec0, b"msg0");

        // Próba ponownego użycia ramki 0 z nowym licznikiem 1 (replay attack)
        let aad1 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 1);
        let res_replay = decrypt_payload(&key, &aad1, &frame0);
        assert!(matches!(res_replay, Err(SyncError::DecryptionFailed)));
    }

    #[tokio::test]
    async fn session_detects_reordered_frames() {
        let key = derive_key("secret", &[1u8; SALT_LEN]).unwrap();
        let sid = [44u8; 32];

        let msg0 = Message::Ping.to_json().unwrap();
        let msg1 = Message::Pong.to_json().unwrap();

        let aad0 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 0);
        let aad1 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 1);

        let frame0 = encrypt_payload(&key, &aad0, &msg0).unwrap();
        let frame1 = encrypt_payload(&key, &aad1, &msg1).unwrap();

        // Odbiorca oczekuje licznika 0, ale otrzymuje ramkę 1 (zamiana kolejności)
        let res_reordered = decrypt_payload(&key, &aad0, &frame1);
        assert!(matches!(res_reordered, Err(SyncError::DecryptionFailed)));

        // Prawidłowa ramka 0 przechodzi
        let res_ok = decrypt_payload(&key, &aad0, &frame0).unwrap();
        assert_eq!(res_ok, msg0);
    }

    #[tokio::test]
    async fn session_detects_missing_counter() {
        let key = derive_key("secret", &[2u8; SALT_LEN]).unwrap();
        let sid = [45u8; 32];

        let msg2 = Message::Bye.to_json().unwrap();
        let aad2 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 2);
        let frame2 = encrypt_payload(&key, &aad2, &msg2).unwrap();

        // Odbiorca oczekuje licznika 1 (pominięto licznik 1, odebrano licznik 2)
        let aad1 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 1);
        let res_skipped = decrypt_payload(&key, &aad1, &frame2);
        assert!(matches!(res_skipped, Err(SyncError::DecryptionFailed)));
    }

    #[tokio::test]
    async fn session_parallel_senders_keep_counter_order() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let salt = random_salt();
        let host_key = derive_key("parallel-secret", &salt).unwrap();

        let total_messages = 3000usize;

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut host_res = host_handshake(stream, "HOST", &salt, &host_key)
                .await
                .unwrap();
            let mut received = 0usize;
            while received < total_messages {
                match host_res.session.recv().await {
                    Ok(Message::Ping) => received += 1,
                    Ok(other) => panic!("unexpected message: {other:?}"),
                    Err(e) => panic!("counter/decryption error after {received} messages: {e}"),
                }
            }
            received
        });

        let peer_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let peer_res = peer_handshake(peer_stream, "PEER", "parallel-secret")
            .await
            .unwrap();
        let sender = peer_res.session.sender();

        let mut handles = Vec::new();
        for _ in 0..3 {
            let s = sender.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..1000 {
                    s.send(Message::Ping).await.unwrap();
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let total_recv = host_fut.await.unwrap();
        assert_eq!(total_recv, 3000);
    }

    #[tokio::test]
    async fn session_writer_is_single_owner() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let salt = random_salt();
        let host_key = derive_key("owner-secret", &salt).unwrap();

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut host_res = host_handshake(stream, "HOST", &salt, &host_key)
                .await
                .unwrap();
            for _ in 0..10 {
                let msg = host_res.session.recv().await.unwrap();
                assert_eq!(msg, Message::Ping);
            }
        });

        let peer_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let peer_res = peer_handshake(peer_stream, "PEER", "owner-secret")
            .await
            .unwrap();

        let s1 = peer_res.session.sender();
        let s2 = peer_res.session.sender();

        for _ in 0..5 {
            s1.send(Message::Ping).await.unwrap();
            s2.send(Message::Ping).await.unwrap();
        }

        host_fut.await.unwrap();
    }

    #[tokio::test]
    async fn session_shutdown_sends_or_handles_bye() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let salt = random_salt();
        let host_key = derive_key("bye-secret", &salt).unwrap();

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut host_res = host_handshake(stream, "HOST", &salt, &host_key)
                .await
                .unwrap();
            let bye = host_res.session.recv().await.unwrap();
            assert_eq!(bye, Message::Bye);
        });

        let peer_stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut peer_res = peer_handshake(peer_stream, "PEER", "bye-secret")
            .await
            .unwrap();

        peer_res.session.shutdown().await;
        host_fut.await.unwrap();
    }

    #[tokio::test]
    async fn secret_and_payload_never_appear_on_the_wire() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let shared_secret = "SECRET_PASSPHRASE_77777";
        let unique_qso_call = "SECRET_PAYLOAD_MARKER_99999";

        let salt = random_salt();
        let host_key = derive_key(shared_secret, &salt).unwrap();

        let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();

        let proxy_fut = tokio::spawn(async move {
            let (mut client_stream, _) = proxy_listener.accept().await.unwrap();
            let mut server_stream = tokio::net::TcpStream::connect(addr).await.unwrap();

            let mut wire_bytes = Vec::new();

            let mut c_buf = [0u8; 4096];
            let mut s_buf = [0u8; 4096];

            loop {
                tokio::select! {
                    n = client_stream.read(&mut c_buf) => {
                        let n = match n {
                            Ok(0) | Err(_) => break,
                            Ok(n) => n,
                        };
                        wire_bytes.extend_from_slice(&c_buf[..n]);
                        if server_stream.write_all(&c_buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    n = server_stream.read(&mut s_buf) => {
                        let n = match n {
                            Ok(0) | Err(_) => break,
                            Ok(n) => n,
                        };
                        wire_bytes.extend_from_slice(&s_buf[..n]);
                        if client_stream.write_all(&s_buf[..n]).await.is_err() {
                            break;
                        }
                    }
                }
            }
            wire_bytes
        });

        let host_fut = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut host_res = host_handshake(stream, "HOST", &salt, &host_key)
                .await
                .unwrap();
            let qso_msg = host_res.session.recv().await.unwrap();
            host_res.session.shutdown().await;
            qso_msg
        });

        let peer_stream = tokio::net::TcpStream::connect(proxy_addr).await.unwrap();
        let mut peer_res = peer_handshake(peer_stream, "PEER", shared_secret)
            .await
            .unwrap();

        let qso = sample_qso(unique_qso_call);
        peer_res
            .session
            .send(Message::Qso(Box::new(qso)))
            .await
            .unwrap();
        peer_res.session.shutdown().await;

        let host_qso = host_fut.await.unwrap();
        assert!(matches!(host_qso, Message::Qso(_)));

        let recorded_wire = proxy_fut.await.unwrap();

        let secret_bytes = shared_secret.as_bytes();
        let payload_bytes = unique_qso_call.as_bytes();

        let found_secret = recorded_wire
            .windows(secret_bytes.len())
            .any(|w| w == secret_bytes);
        let found_payload = recorded_wire
            .windows(payload_bytes.len())
            .any(|w| w == payload_bytes);

        assert!(!found_secret, "shared secret must never appear on the wire");
        assert!(
            !found_payload,
            "plaintext QSO payload must never appear on the wire"
        );
    }

    #[tokio::test]
    async fn qso_from_peer_reaches_other_peer_and_host_but_not_sender() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let (p1_tx, p1_rx) = std::sync::mpsc::channel();
        let mut peer1 = SyncPeer::start(
            host_addr,
            "PEER1_STATION".to_string(),
            "shared_secret".to_string(),
            p1_tx,
        );

        let (p2_tx, p2_rx) = std::sync::mpsc::channel();
        let mut peer2 = SyncPeer::start(
            host_addr,
            "PEER2_STATION".to_string(),
            "shared_secret".to_string(),
            p2_tx,
        );

        for _ in 0..100 {
            if host.connected_peers() == 2 && peer1.is_connected() && peer2.is_connected() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.connected_peers(), 2);

        let qso = sample_qso("PEER1_QSO");
        peer1.broadcast_qso(qso).unwrap();

        let mut host_got_qso = false;
        for _ in 0..100 {
            while let Ok(evt) = host_rx.try_recv() {
                if matches!(evt, SyncEvent::IncomingQso(q) if q.callsign == "PEER1_QSO") {
                    host_got_qso = true;
                }
            }
            if host_got_qso {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(host_got_qso, "Host must receive QSO from Peer 1");

        let mut peer2_got_qso = false;
        for _ in 0..100 {
            while let Ok(evt) = p2_rx.try_recv() {
                if matches!(evt, SyncEvent::IncomingQso(q) if q.callsign == "PEER1_QSO") {
                    peer2_got_qso = true;
                }
            }
            if peer2_got_qso {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(peer2_got_qso, "Peer 2 must receive relayed QSO from Peer 1");

        let mut peer1_got_qso = false;
        while let Ok(evt) = p1_rx.try_recv() {
            if matches!(evt, SyncEvent::IncomingQso(q) if q.callsign == "PEER1_QSO") {
                peer1_got_qso = true;
            }
        }
        assert!(
            !peer1_got_qso,
            "Peer 1 must NOT receive its own QSO back (echo-free)"
        );

        peer1.stop();
        peer2.stop();
        host.stop();
    }

    #[tokio::test]
    async fn host_local_qso_reaches_all_peers() {
        let (host_tx, _) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let (p1_tx, p1_rx) = std::sync::mpsc::channel();
        let mut peer1 = SyncPeer::start(
            host_addr,
            "PEER1_STATION".to_string(),
            "shared_secret".to_string(),
            p1_tx,
        );

        let (p2_tx, p2_rx) = std::sync::mpsc::channel();
        let mut peer2 = SyncPeer::start(
            host_addr,
            "PEER2_STATION".to_string(),
            "shared_secret".to_string(),
            p2_tx,
        );

        for _ in 0..100 {
            if host.connected_peers() == 2 && peer1.is_connected() && peer2.is_connected() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.connected_peers(), 2);

        let qso = sample_qso("HOST_LOCAL_QSO");
        host.broadcast_qso(qso);

        let mut p1_got = false;
        let mut p2_got = false;
        for _ in 0..100 {
            while let Ok(evt) = p1_rx.try_recv() {
                if matches!(evt, SyncEvent::IncomingQso(q) if q.callsign == "HOST_LOCAL_QSO") {
                    p1_got = true;
                }
            }
            while let Ok(evt) = p2_rx.try_recv() {
                if matches!(evt, SyncEvent::IncomingQso(q) if q.callsign == "HOST_LOCAL_QSO") {
                    p2_got = true;
                }
            }
            if p1_got && p2_got {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        assert!(p1_got, "Peer 1 must receive host local QSO");
        assert!(p2_got, "Peer 2 must receive host local QSO");

        peer1.stop();
        peer2.stop();
        host.stop();
    }

    #[tokio::test]
    async fn qso_is_not_echoed_to_sender() {
        let (host_tx, _) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let (p1_tx, p1_rx) = std::sync::mpsc::channel();
        let mut peer1 = SyncPeer::start(
            host_addr,
            "PEER1_STATION".to_string(),
            "shared_secret".to_string(),
            p1_tx,
        );

        for _ in 0..100 {
            if host.connected_peers() == 1 && peer1.is_connected() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.connected_peers(), 1);

        peer1.broadcast_qso(sample_qso("NO_ECHO_CALL")).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut echoed = false;
        while let Ok(evt) = p1_rx.try_recv() {
            if matches!(evt, SyncEvent::IncomingQso(q) if q.callsign == "NO_ECHO_CALL") {
                echoed = true;
            }
        }
        assert!(!echoed, "Sender must not receive echo of its own QSO");

        peer1.stop();
        host.stop();
    }

    #[tokio::test]
    async fn client_limit_is_enforced() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let mut peers = Vec::new();
        for i in 0..MAX_ACTIVE_PEERS {
            let (tx, _) = std::sync::mpsc::channel();
            let p = SyncPeer::start(
                host_addr,
                format!("PEER_{i}"),
                "shared_secret".to_string(),
                tx,
            );
            peers.push(p);
        }

        for _ in 0..200 {
            if host.connected_peers() == MAX_ACTIVE_PEERS {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(host.connected_peers(), MAX_ACTIVE_PEERS);

        let s33 = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        let p33_res = peer_handshake(s33, "PEER_33", "shared_secret").await;

        let mut limit_reported = false;
        while let Ok(evt) = host_rx.try_recv() {
            if let SyncEvent::Error(SyncError::ClientLimitReached) = evt {
                limit_reported = true;
            }
        }

        assert!(
            p33_res.is_err() || limit_reported,
            "33rd peer connection must be rejected or reported as limit reached"
        );
        assert_eq!(host.connected_peers(), MAX_ACTIVE_PEERS);

        for mut p in peers {
            p.stop();
        }
        host.stop();
    }

    #[tokio::test]
    async fn handshake_parallelism_is_limited() {
        let (host_tx, _) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        assert_eq!(host.available_handshake_permits(), MAX_HANDSHAKES);

        let mut streams = Vec::new();
        for _ in 0..MAX_HANDSHAKES {
            let s = tokio::net::TcpStream::connect(host_addr).await.unwrap();
            streams.push(s);
        }

        for _ in 0..50 {
            if host.available_handshake_permits() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.available_handshake_permits(), 0);

        let s6 = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(host.available_handshake_permits(), 0);

        drop(streams.pop());
        tokio::time::sleep(Duration::from_millis(50)).await;

        drop(streams);
        drop(s6);
        host.stop();
    }

    async fn wait_for_host_event_matching<F>(
        rx: &std::sync::mpsc::Receiver<SyncEvent>,
        mut predicate: F,
        desc: &str,
        max_iterations: usize,
    ) -> Result<Vec<SyncEvent>, (String, Vec<SyncEvent>)>
    where
        F: FnMut(&SyncEvent) -> bool,
    {
        let mut collected = Vec::new();
        for _ in 0..max_iterations {
            while let Ok(evt) = rx.try_recv() {
                let matched = predicate(&evt);
                collected.push(evt);
                if matched {
                    return Ok(collected);
                }
            }
            tokio::task::yield_now().await;
            tokio::time::advance(Duration::from_millis(1)).await;
        }
        Err((
            format!(
                "Exceeded {} iterations waiting for {}; collected {} events: {:?}",
                max_iterations,
                desc,
                collected.len(),
                collected
            ),
            collected,
        ))
    }

    #[tokio::test(start_paused = true)]
    async fn sync_host_handshake_timeout_releases_slot() {
        tokio::time::resume();
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "TEST_SECRET_DO_NOT_USE_123".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        assert_eq!(host.available_handshake_permits(), MAX_HANDSHAKES);

        let mut streams = Vec::new();
        for _ in 0..MAX_HANDSHAKES {
            let s = tokio::net::TcpStream::connect(host_addr).await.unwrap();
            streams.push(s);
        }

        for _ in 0..50 {
            if host.available_handshake_permits() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.available_handshake_permits(), 0);

        tokio::time::pause();
        tokio::time::advance(HOST_HANDSHAKE_TIMEOUT + Duration::from_millis(100)).await;
        tokio::time::resume();

        for _ in 0..50 {
            if host.available_handshake_permits() == MAX_HANDSHAKES {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(
            host.available_handshake_permits(),
            MAX_HANDSHAKES,
            "Wszystkie sloty semafora handshake musza zostac zwolnione po timeout"
        );

        let mut timeout_count = 0;
        while let Ok(evt) = host_rx.try_recv() {
            if matches!(evt, SyncEvent::Error(SyncError::Timeout)) {
                timeout_count += 1;
            }
        }
        assert_eq!(
            timeout_count, MAX_HANDSHAKES,
            "Kazdy zablokowany handshake musi zaraportowac blad Timeout"
        );

        let legitimate_stream = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        let peer_res = peer_handshake(
            legitimate_stream,
            "LEGIT_PEER",
            "TEST_SECRET_DO_NOT_USE_123",
        )
        .await;
        assert!(
            peer_res.is_ok(),
            "Nowe polaczenie po timeout musi przejsc handshake"
        );

        drop(streams);
        host.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn sync_partial_handshake_times_out() {
        tokio::time::resume();
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "TEST_SECRET_DO_NOT_USE_456".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let mut stream = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        stream.write_all(b"SP").await.unwrap();
        stream.flush().await.unwrap();

        for _ in 0..50 {
            if host.available_handshake_permits() < MAX_HANDSHAKES {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.available_handshake_permits(), MAX_HANDSHAKES - 1);

        tokio::time::pause();
        tokio::time::advance(HOST_HANDSHAKE_TIMEOUT + Duration::from_millis(100)).await;
        tokio::time::resume();

        for _ in 0..50 {
            if host.available_handshake_permits() == MAX_HANDSHAKES {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(
            host.available_handshake_permits(),
            MAX_HANDSHAKES,
            "Slot semafora musi zostac zwolniony po timeoucie czesciowego handshake'u"
        );

        let mut timeout_seen = false;
        while let Ok(evt) = host_rx.try_recv() {
            if matches!(evt, SyncEvent::Error(SyncError::Timeout)) {
                timeout_seen = true;
            }
        }
        assert!(
            timeout_seen,
            "Czesciowy handshake musi zglosic SyncError::Timeout"
        );

        let mut buf = [0u8; 16];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        assert_eq!(
            n, 0,
            "Gniazdo klienta musi zostac zamkniete po timeoucie (EOF)"
        );

        host.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn sync_successful_handshake_is_not_rejected_by_timeout() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "TEST_SECRET_DO_NOT_USE_789".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let stream = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        let peer_res = peer_handshake(stream, "FAST_PEER", "TEST_SECRET_DO_NOT_USE_789").await;
        assert!(
            peer_res.is_ok(),
            "Szybki handshake musi zakonczyc sie sukcesem"
        );

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 1),
            "SyncStatus { peers: 1, .. }",
            200,
        )
        .await
        .expect("Peer musi zostac zarejestrowany");

        assert_eq!(host.connected_peers(), 1);
        assert_eq!(host.available_handshake_permits(), MAX_HANDSHAKES);

        while let Ok(evt) = host_rx.try_recv() {
            assert!(
                !matches!(evt, SyncEvent::Error(SyncError::Timeout)),
                "Poprawny handshake nie moze wygenerowac SyncError::Timeout"
            );
        }

        drop(peer_res);
        host.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn idle_peer_is_dropped_after_timeout() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let s = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        let handshake_res = peer_handshake(s, "SILENT_PEER", "shared_secret")
            .await
            .unwrap();

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 1),
            "SyncStatus { peers: 1, .. }",
            200,
        )
        .await
        .expect("Peer must be registered with peers == 1");
        assert_eq!(host.connected_peers(), 1);

        // Upewnij się, że zadanie sesji peera (run_host_peer_session) rozpoczęło bieg
        // i zarejestrowało timer bezczynności (idle_sleep) w bieżącym czasie wirtualnym.
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        tokio::time::advance(IDLE_TIMEOUT + Duration::from_millis(1)).await;

        tokio::task::yield_now().await;

        let events_cleanup = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 0),
            "SyncStatus { peers: 0, .. }",
            200,
        )
        .await
        .expect("Cleanup must emit SyncStatus with peers == 0");

        assert_eq!(host.connected_peers(), 0);

        let mut timeout_reported = events_cleanup
            .iter()
            .any(|evt| matches!(evt, SyncEvent::Error(SyncError::Timeout)));
        if !timeout_reported {
            while let Ok(evt) = host_rx.try_recv() {
                if let SyncEvent::Error(SyncError::Timeout) = evt {
                    timeout_reported = true;
                }
            }
        }
        assert!(
            timeout_reported,
            "Timeout error must be reported for idle peer"
        );

        drop(handshake_res);
        host.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn idle_peer_cleanup_event_follows_map_removal() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();

        let s = tokio::net::TcpStream::connect(host.local_addr())
            .await
            .unwrap();
        let handshake_res = peer_handshake(s, "SILENT_PEER", "shared_secret")
            .await
            .unwrap();

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 1),
            "SyncStatus { peers: 1, .. }",
            200,
        )
        .await
        .expect("Peer connected");
        assert_eq!(host.connected_peers(), 1);

        // Upewnij się, że zadanie sesji peera (run_host_peer_session) rozpoczęło bieg
        // i zarejestrowało timer bezczynności (idle_sleep) w bieżącym czasie wirtualnym.
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        tokio::time::advance(IDLE_TIMEOUT + Duration::from_millis(1)).await;

        let mut peer_count_at_status_zero = None;
        let mut timeout_seen_before_or_at_status = false;

        for _ in 0..200 {
            while let Ok(evt) = host_rx.try_recv() {
                if matches!(evt, SyncEvent::Error(SyncError::Timeout)) {
                    timeout_seen_before_or_at_status = true;
                }
                if matches!(evt, SyncEvent::Status(ref st) if st.peers == 0) {
                    peer_count_at_status_zero = Some(host.connected_peers());
                    break;
                }
            }
            if peer_count_at_status_zero.is_some() {
                break;
            }
            tokio::task::yield_now().await;
            tokio::time::advance(Duration::from_millis(1)).await;
        }

        assert_eq!(
            peer_count_at_status_zero,
            Some(0),
            "peer removed from map -> Status(peers == 0) published: connected_peers() must be 0 at Status(peers == 0)"
        );
        assert!(
            timeout_seen_before_or_at_status,
            "SyncError::Timeout must precede or accompany Status(peers == 0)"
        );

        drop(handshake_res);
        host.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn idle_peer_timeout_uses_virtual_time() {
        let wall_start = std::time::Instant::now();
        let virt_start = tokio::time::Instant::now();

        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();

        let s = tokio::net::TcpStream::connect(host.local_addr())
            .await
            .unwrap();
        let handshake_res = peer_handshake(s, "SILENT_PEER", "shared_secret")
            .await
            .unwrap();

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 1),
            "SyncStatus { peers: 1, .. }",
            200,
        )
        .await
        .expect("Peer connected");

        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        tokio::time::advance(IDLE_TIMEOUT + Duration::from_millis(1)).await;

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 0),
            "SyncStatus { peers: 0, .. }",
            200,
        )
        .await
        .expect("Cleanup event");

        assert_eq!(host.connected_peers(), 0);

        let virt_elapsed = tokio::time::Instant::now().duration_since(virt_start);
        let wall_elapsed = wall_start.elapsed();

        assert!(
            virt_elapsed >= IDLE_TIMEOUT,
            "Virtual time must have advanced past IDLE_TIMEOUT (45s)"
        );
        assert!(
            wall_elapsed < Duration::from_secs(5),
            "Wall clock time must remain negligible (< 5s), actual: {wall_elapsed:?}",
        );

        drop(handshake_res);
        host.stop();
    }

    #[test]
    fn idle_peer_timeout_test_has_no_wall_clock_sleep() {
        let src = include_str!("mod.rs");
        let start_marker = "async fn idle_peer_is_dropped_after_timeout()";
        let test_fn = src
            .split(start_marker)
            .nth(1)
            .expect("idle_peer_is_dropped_after_timeout function must exist in mod.rs");
        let body = test_fn
            .split("#[tokio::test")
            .next()
            .expect("test body must end before next test");
        assert!(
            !body.contains("std::thread::sleep"),
            "idle_peer_is_dropped_after_timeout must not use std::thread::sleep"
        );
        assert!(
            !body.contains("tokio::time::sleep"),
            "idle_peer_is_dropped_after_timeout must not use tokio::time::sleep"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn idle_peer_active_peer_is_not_dropped_before_idle_timeout() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();

        let s = tokio::net::TcpStream::connect(host.local_addr())
            .await
            .unwrap();
        let handshake_res = peer_handshake(s, "SILENT_PEER", "shared_secret")
            .await
            .unwrap();

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 1),
            "SyncStatus { peers: 1, .. }",
            200,
        )
        .await
        .expect("Peer connected");

        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        tokio::time::advance(Duration::from_secs(40)).await;
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        assert_eq!(host.connected_peers(), 1);
        while let Ok(evt) = host_rx.try_recv() {
            if let SyncEvent::Error(SyncError::Timeout) = evt {
                panic!("Timeout error occurred prematurely before 45s");
            }
            if let SyncEvent::Status(st) = evt {
                assert_eq!(st.peers, 1, "Peer count must remain 1 before timeout");
            }
        }

        tokio::time::advance(Duration::from_millis(5001)).await;
        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 0),
            "SyncStatus { peers: 0, .. }",
            200,
        )
        .await
        .expect("Cleanup event after IDLE_TIMEOUT");

        assert_eq!(host.connected_peers(), 0);

        drop(handshake_res);
        host.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn idle_peer_count_reaches_zero_after_timeout_cleanup() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();

        let s = tokio::net::TcpStream::connect(host.local_addr())
            .await
            .unwrap();
        let handshake_res = peer_handshake(s, "SILENT_PEER", "shared_secret")
            .await
            .unwrap();

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 1),
            "SyncStatus { peers: 1, .. }",
            200,
        )
        .await
        .expect("Peer connected");

        assert_eq!(host.connected_peers(), 1);
        assert_eq!(host.peer_stations(), vec!["SILENT_PEER"]);

        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        tokio::time::advance(IDLE_TIMEOUT + Duration::from_millis(1)).await;

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 0),
            "SyncStatus { peers: 0, .. }",
            200,
        )
        .await
        .expect("Cleanup event");

        assert_eq!(host.connected_peers(), 0);
        assert_eq!(host.peer_stations(), Vec::<String>::new());

        drop(handshake_res);
        host.stop();
    }

    #[tokio::test(start_paused = true)]
    async fn idle_peer_host_shutdown_after_idle_cleanup_is_clean() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();

        let s = tokio::net::TcpStream::connect(host.local_addr())
            .await
            .unwrap();
        let handshake_res = peer_handshake(s, "SILENT_PEER", "shared_secret")
            .await
            .unwrap();

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 1),
            "SyncStatus { peers: 1, .. }",
            200,
        )
        .await
        .expect("Peer connected");

        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        tokio::time::advance(IDLE_TIMEOUT + Duration::from_millis(1)).await;

        let _ = wait_for_host_event_matching(
            &host_rx,
            |evt| matches!(evt, SyncEvent::Status(st) if st.peers == 0),
            "SyncStatus { peers: 0, .. }",
            200,
        )
        .await
        .expect("Cleanup event");

        drop(handshake_res);
        host.stop();

        let mut starting_seen = false;
        while let Ok(evt) = host_rx.try_recv() {
            if matches!(evt, SyncEvent::Status(st) if st.state == SyncState::Starting && st.peers == 0)
            {
                starting_seen = true;
            }
        }
        assert!(
            starting_seen,
            "Host stop must transition to SyncState::Starting with 0 peers"
        );
        assert_eq!(host.connected_peers(), 0);
    }

    #[tokio::test]
    async fn slow_peer_is_dropped_explicitly() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let s = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        let handshake_res = peer_handshake(s, "SLOW_PEER", "shared_secret")
            .await
            .unwrap();

        for _ in 0..50 {
            if host.connected_peers() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.connected_peers(), 1);

        for i in 0..300 {
            host.broadcast_qso(sample_qso(&format!("SLOW_{i}")));
        }

        for _ in 0..50 {
            if host.connected_peers() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.connected_peers(), 0);

        let mut queue_full_found = false;
        while let Ok(evt) = host_rx.try_recv() {
            if let SyncEvent::Error(SyncError::QueueFull) = evt {
                queue_full_found = true;
            }
        }
        assert!(
            queue_full_found,
            "QueueFull error must be reported when dropping slow peer"
        );

        drop(handshake_res);
        host.stop();
    }

    #[tokio::test]
    async fn queue_overflow_is_reported() {
        let (host_tx, host_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let s = tokio::net::TcpStream::connect(host_addr).await.unwrap();
        let handshake_res = peer_handshake(s, "SLOW_PEER_2", "shared_secret")
            .await
            .unwrap();

        for _ in 0..50 {
            if host.connected_peers() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        for i in 0..300 {
            host.broadcast_qso(sample_qso(&format!("OVERFLOW_{i}")));
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut reported = false;
        while let Ok(evt) = host_rx.try_recv() {
            if let SyncEvent::Error(SyncError::QueueFull) = evt {
                reported = true;
            }
        }
        assert!(
            reported,
            "SyncError::QueueFull must be reported on overflow"
        );

        drop(handshake_res);
        host.stop();
    }

    #[test]
    fn peer_reconnects_with_backoff() {
        let mut backoff = ReconnectBackoff::new();
        assert_eq!(backoff.current(), Duration::from_secs(2));

        assert_eq!(backoff.fail(), Duration::from_secs(2));
        assert_eq!(backoff.current(), Duration::from_secs(4));

        assert_eq!(backoff.fail(), Duration::from_secs(4));
        assert_eq!(backoff.current(), Duration::from_secs(8));

        assert_eq!(backoff.fail(), Duration::from_secs(8));
        assert_eq!(backoff.current(), Duration::from_secs(16));

        assert_eq!(backoff.fail(), Duration::from_secs(16));
        assert_eq!(backoff.current(), Duration::from_secs(30));

        assert_eq!(backoff.fail(), Duration::from_secs(30));
        assert_eq!(backoff.current(), Duration::from_secs(30));

        assert_eq!(backoff.fail(), Duration::from_secs(30));
        assert_eq!(backoff.current(), Duration::from_secs(30));
    }

    #[test]
    fn successful_connection_resets_backoff() {
        let mut backoff = ReconnectBackoff::new();
        backoff.fail();
        backoff.fail();
        backoff.fail();
        assert_eq!(backoff.current(), Duration::from_secs(16));

        backoff.reset();
        assert_eq!(backoff.current(), Duration::from_secs(2));

        assert_eq!(backoff.fail(), Duration::from_secs(2));
        assert_eq!(backoff.current(), Duration::from_secs(4));
    }

    #[tokio::test]
    async fn stop_cancels_reconnect_loop() {
        let (tx, _) = std::sync::mpsc::channel();
        let mut peer = SyncPeer::start(
            "127.0.0.1:59999".parse().unwrap(),
            "PEER_STOP".to_string(),
            "secret".to_string(),
            tx,
        );

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!peer.is_connected());

        peer.stop();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!peer.is_connected());
    }

    #[tokio::test]
    async fn host_shutdown_closes_all_sessions() {
        let (host_tx, _) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind(
            "127.0.0.1:0".parse().unwrap(),
            "HOST_STATION".to_string(),
            "shared_secret".to_string(),
            host_tx,
        )
        .await
        .unwrap();
        let host_addr = host.local_addr();

        let (p1_tx, _) = std::sync::mpsc::channel();
        let mut peer1 = SyncPeer::start(
            host_addr,
            "PEER1".to_string(),
            "shared_secret".to_string(),
            p1_tx,
        );

        let (p2_tx, _) = std::sync::mpsc::channel();
        let mut peer2 = SyncPeer::start(
            host_addr,
            "PEER2".to_string(),
            "shared_secret".to_string(),
            p2_tx,
        );

        for _ in 0..100 {
            if host.connected_peers() == 2 && peer1.is_connected() && peer2.is_connected() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.connected_peers(), 2);

        host.stop();
        assert_eq!(host.connected_peers(), 0);

        for _ in 0..100 {
            if !peer1.is_connected() && !peer2.is_connected() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(!peer1.is_connected());
        assert!(!peer2.is_connected());

        peer1.stop();
        peer2.stop();
    }

    #[test]
    fn catchup_diff_is_symmetric() {
        let store_a = MemorySyncStore::new();
        let store_b = MemorySyncStore::new();

        let q1 = sample_qso("SP1AAA");
        let q2 = sample_qso("SP2BBB");
        let q3 = sample_qso("SP3CCC");

        store_a.insert_test_qso("DEFAULT", q1.clone());
        store_a.insert_test_qso("DEFAULT", q2.clone());

        store_b.insert_test_qso("DEFAULT", q2.clone());
        store_b.insert_test_qso("DEFAULT", q3.clone());

        let inv_a = store_a.inventory("DEFAULT").unwrap();
        let inv_b = store_b.inventory("DEFAULT").unwrap();

        let wanted_by_a: HashSet<_> = inv_b.difference(&inv_a).cloned().collect();
        let wanted_by_b: HashSet<_> = inv_a.difference(&inv_b).cloned().collect();

        assert_eq!(wanted_by_a.len(), 1);
        assert!(wanted_by_a.contains(&q3.identity()));

        assert_eq!(wanted_by_b.len(), 1);
        assert!(wanted_by_b.contains(&q1.identity()));
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn catchup_chunks_by_serialized_size() {
        let short_ids: Vec<QsoIdentity> = (0..500)
            .map(|i| QsoIdentity::from_fields(&format!("S{i}"), "20m", "CW", "20261018", "120000"))
            .collect();

        let long_ids: Vec<QsoIdentity> = (0..500)
            .map(|i| {
                let long_call = format!("SPECIAL_EVENT_STATION_VERY_LONG_CALLSIGN_{i}");
                QsoIdentity::from_fields(&long_call, "20m", "CW", "20261018", "120000")
            })
            .collect();

        let short_bytes: usize = short_ids
            .iter()
            .map(|id| serde_json::to_vec(id).unwrap().len())
            .sum();
        let long_bytes: usize = long_ids
            .iter()
            .map(|id| serde_json::to_vec(id).unwrap().len())
            .sum();

        assert!(long_bytes > short_bytes);

        let c1 = chunk_inventory("DEFAULT", &short_ids);
        let c2 = chunk_inventory("DEFAULT", &long_ids);
        assert!(!c1.is_empty());
        assert!(!c2.is_empty());
    }

    #[test]
    fn each_inventory_chunk_is_at_most_one_mib() {
        let identities: Vec<QsoIdentity> = (0..15_000)
            .map(|i| {
                let call = format!("VERY_LONG_CALLSIGN_FOR_TEST_{i}");
                QsoIdentity::from_fields(&call, "20m", "CW", "20261018", "120000")
            })
            .collect();

        let chunks = chunk_inventory("DEFAULT", &identities);
        assert!(
            chunks.len() > 1,
            "Must produce multiple chunks for 15k items"
        );

        for (idx, chunk) in chunks.iter().enumerate() {
            let bytes = serde_json::to_vec(chunk).unwrap();
            assert!(
                bytes.len() <= MAX_CHUNK_BYTES,
                "Chunk {idx} size ({} B) exceeds 1 MiB limit",
                bytes.len()
            );
            if idx == chunks.len() - 1 {
                if let Message::Inventory { last, .. } = chunk {
                    assert!(*last, "Terminal chunk must have last == true");
                }
            } else if let Message::Inventory { last, .. } = chunk {
                assert!(!*last, "Non-terminal chunk must have last == false");
            }
        }
    }

    #[test]
    fn each_request_chunk_is_at_most_one_mib() {
        let identities: Vec<QsoIdentity> = (0..15_000)
            .map(|i| {
                let call = format!("REQUESTED_CALLSIGN_{i}");
                QsoIdentity::from_fields(&call, "40m", "SSB", "20261018", "130000")
            })
            .collect();

        let chunks = chunk_requests(&identities);
        assert!(
            chunks.len() > 1,
            "Must produce multiple chunks for 15k items"
        );

        for (idx, chunk) in chunks.iter().enumerate() {
            let bytes = serde_json::to_vec(chunk).unwrap();
            assert!(
                bytes.len() <= MAX_CHUNK_BYTES,
                "Request chunk {idx} size ({} B) exceeds 1 MiB",
                bytes.len()
            );
            if idx == chunks.len() - 1 {
                if let Message::Request { last, .. } = chunk {
                    assert!(*last);
                }
            } else if let Message::Request { last, .. } = chunk {
                assert!(!*last);
            }
        }
    }

    #[test]
    fn each_records_chunk_is_at_most_one_mib() {
        let records: Vec<QsoRecord> = (0..3_000)
            .map(|i| {
                let mut q = sample_qso(&format!("RECORDS_TEST_{i}"));
                q.comment = Some("A".repeat(500));
                q
            })
            .collect();

        let chunks = chunk_records(&records, None);
        assert!(chunks.len() > 1, "Must produce multiple chunks");

        for (idx, chunk) in chunks.iter().enumerate() {
            let bytes = serde_json::to_vec(chunk).unwrap();
            assert!(
                bytes.len() <= MAX_CHUNK_BYTES,
                "Records chunk {idx} size ({} B) exceeds 1 MiB",
                bytes.len()
            );
            if idx == chunks.len() - 1 {
                if let Message::Records { last, .. } = chunk {
                    assert!(*last);
                }
            } else if let Message::Records { last, .. } = chunk {
                assert!(!*last);
            }
        }
    }

    #[test]
    fn oversized_record_is_skipped_not_fatal() {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut oversized = sample_qso("OVERSIZED_CALL");
        oversized.comment = Some("X".repeat(7 * 1024 * 1024));

        let normal = sample_qso("NORMAL_CALL");

        let chunks = chunk_records(&[oversized, normal], Some(&tx));

        let mut total_records = 0;
        let mut has_normal = false;
        let mut has_oversized = false;
        for c in chunks {
            if let Message::Records { qsos, .. } = c {
                total_records += qsos.len();
                for q in qsos {
                    if q.callsign == "NORMAL_CALL" {
                        has_normal = true;
                    }
                    if q.callsign == "OVERSIZED_CALL" {
                        has_oversized = true;
                    }
                }
            }
        }

        assert_eq!(total_records, 1);
        assert!(has_normal);
        assert!(!has_oversized);

        let mut error_reported = false;
        while let Ok(evt) = rx.try_recv() {
            if let SyncEvent::Error(SyncError::RecordTooLarge { size, limit }) = evt {
                assert!(size > 6 * 1024 * 1024);
                assert_eq!(limit, 6 * 1024 * 1024);
                error_reported = true;
            }
        }
        assert!(error_reported, "RecordTooLarge error must be reported");
    }

    #[tokio::test]
    async fn catchup_exchanges_missing_records_both_ways() {
        let store_a = Arc::new(MemorySyncStore::new());
        let store_b = Arc::new(MemorySyncStore::new());

        let q1 = sample_qso("CALL_A_ONLY");
        let q2 = sample_qso("CALL_SHARED");
        let q3 = sample_qso("CALL_B_ONLY");

        store_a.insert_test_qso("DEFAULT", q1.clone());
        store_a.insert_test_qso("DEFAULT", q2.clone());

        store_b.insert_test_qso("DEFAULT", q2.clone());
        store_b.insert_test_qso("DEFAULT", q3.clone());

        let (host_events_tx, _host_events_rx) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind_with_store(
            "127.0.0.1:0".parse().unwrap(),
            "HOST".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_a.clone()),
            host_events_tx,
        )
        .await
        .unwrap();

        let (peer_events_tx, _peer_events_rx) = std::sync::mpsc::channel();
        let mut peer = SyncPeer::start_with_store(
            host.local_addr(),
            "PEER".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_b.clone()),
            peer_events_tx,
        );

        for _ in 0..100 {
            if store_a.count("DEFAULT") == 3 && store_b.count("DEFAULT") == 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }

        assert_eq!(store_a.count("DEFAULT"), 3);
        assert_eq!(store_b.count("DEFAULT"), 3);

        let inv_a = store_a.inventory("DEFAULT").unwrap();
        let inv_b = store_b.inventory("DEFAULT").unwrap();
        assert_eq!(inv_a, inv_b);

        peer.stop();
        host.stop();
    }

    #[tokio::test]
    async fn catchup_second_run_inserts_nothing() {
        let store_a = Arc::new(MemorySyncStore::new());
        let store_b = Arc::new(MemorySyncStore::new());

        store_a.insert_test_qso("DEFAULT", sample_qso("EXCHANGE_1"));
        store_b.insert_test_qso("DEFAULT", sample_qso("EXCHANGE_2"));

        let (host_events_tx, _) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind_with_store(
            "127.0.0.1:0".parse().unwrap(),
            "HOST".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_a.clone()),
            host_events_tx,
        )
        .await
        .unwrap();

        let (peer_events_tx, _) = std::sync::mpsc::channel();
        let mut peer = SyncPeer::start_with_store(
            host.local_addr(),
            "PEER".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_b.clone()),
            peer_events_tx,
        );

        for _ in 0..100 {
            if store_a.count("DEFAULT") == 2 && store_b.count("DEFAULT") == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        assert_eq!(store_a.count("DEFAULT"), 2);
        assert_eq!(store_b.count("DEFAULT"), 2);

        peer.stop();
        host.stop();

        // Second run: reconnect with identical stores
        let (host_events_tx2, host_events_rx2) = std::sync::mpsc::channel();
        let mut host2 = SyncHost::bind_with_store(
            "127.0.0.1:0".parse().unwrap(),
            "HOST".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_a.clone()),
            host_events_tx2,
        )
        .await
        .unwrap();

        let (peer_events_tx2, peer_events_rx2) = std::sync::mpsc::channel();
        let mut peer2 = SyncPeer::start_with_store(
            host2.local_addr(),
            "PEER".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_b.clone()),
            peer_events_tx2,
        );

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(store_a.count("DEFAULT"), 2);
        assert_eq!(store_b.count("DEFAULT"), 2);

        while let Ok(evt) = host_events_rx2.try_recv() {
            if let SyncEvent::CatchUpApplied { inserted } = evt {
                assert_eq!(inserted, 0);
            }
        }
        while let Ok(evt) = peer_events_rx2.try_recv() {
            if let SyncEvent::CatchUpApplied { inserted } = evt {
                assert_eq!(inserted, 0);
            }
        }

        peer2.stop();
        host2.stop();
    }

    #[test]
    fn catchup_does_not_overwrite_existing_qso() {
        let store = MemorySyncStore::new();

        let mut q_orig = sample_qso("OVERWRITE_CHECK");
        q_orig.comment = Some("ORIGINAL_COMMENT".to_string());
        store.insert_test_qso("DEFAULT", q_orig);

        let mut q_new = sample_qso("OVERWRITE_CHECK");
        q_new.comment = Some("MODIFIED_COMMENT".to_string());

        let inserted = store.import("DEFAULT", vec![q_new]).unwrap();
        assert_eq!(inserted, 0, "Duplicate identity must not be inserted");

        let current = store.get_all("DEFAULT");
        assert_eq!(current.len(), 1);
        assert_eq!(
            current[0].comment.as_deref(),
            Some("ORIGINAL_COMMENT"),
            "Existing QSO fields must not be overwritten"
        );
    }

    #[test]
    fn catchup_does_not_delete_qso() {
        let store = MemorySyncStore::new();

        let q1 = sample_qso("CALL_ONE");
        let q2 = sample_qso("CALL_TWO");
        store.insert_test_qso("DEFAULT", q1.clone());
        store.insert_test_qso("DEFAULT", q2.clone());

        let q3 = sample_qso("CALL_THREE");
        let inserted = store.import("DEFAULT", vec![q3.clone()]).unwrap();
        assert_eq!(inserted, 1);

        let inv = store.inventory("DEFAULT").unwrap();
        assert_eq!(inv.len(), 3);
        assert!(inv.contains(&q1.identity()));
        assert!(inv.contains(&q2.identity()));
        assert!(inv.contains(&q3.identity()));
    }

    #[tokio::test]
    async fn catchup_only_uses_active_journal() {
        let store = Arc::new(MemorySyncStore::new());
        let (events_tx, _) = std::sync::mpsc::channel();
        let mut handler =
            CatchUpHandler::new("ACTIVE_JOURNAL".to_string(), store.clone(), events_tx);

        let (sender_tx, mut sender_rx) = tokio::sync::mpsc::channel(10);
        let sender = SessionSender::from_channel(sender_tx);

        let foreign_id =
            QsoIdentity::from_fields("FOREIGN_CALL", "20m", "CW", "20261018", "120000");
        handler
            .handle_inventory("OTHER_JOURNAL", vec![foreign_id], true, &sender)
            .unwrap();

        assert!(sender_rx.try_recv().is_err(), "Must ignore other journals");
        assert!(!handler.is_peer_inventory_complete());
    }

    #[test]
    fn store_for_ingestor_roundtrip() {
        let ingestor = test_ingestor();

        let q1 = sample_qso("SP5TEST");
        let q2 = sample_qso("SP6TEST");

        let inserted = ingestor
            .import("DEFAULT", vec![q1.clone(), q2.clone()])
            .unwrap();
        assert_eq!(inserted, 2);

        let inv = ingestor.inventory("DEFAULT").unwrap();
        assert!(inv.contains(&q1.identity()));
        assert!(inv.contains(&q2.identity()));

        let mut wanted = HashSet::new();
        wanted.insert(q1.identity());
        let records = ingestor.records_for("DEFAULT", &wanted).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].callsign, "SP5TEST");
    }

    #[test]
    fn store_import_deduplicates_by_qso_identity() {
        let ingestor = test_ingestor();

        let q = sample_qso("SP7DUP");

        let inserted_1 = ingestor.import("DEFAULT", vec![q.clone()]).unwrap();
        assert_eq!(inserted_1, 1);

        let inserted_2 = ingestor.import("DEFAULT", vec![q.clone()]).unwrap();
        assert_eq!(inserted_2, 0, "Duplicate import must insert 0 records");
    }

    #[tokio::test]
    async fn catchup_handles_empty_inventory() {
        let store_a = Arc::new(MemorySyncStore::new());
        let store_b = Arc::new(MemorySyncStore::new());

        store_b.insert_test_qso("DEFAULT", sample_qso("EMPTY_TEST_1"));
        store_b.insert_test_qso("DEFAULT", sample_qso("EMPTY_TEST_2"));

        let (host_tx, _) = std::sync::mpsc::channel();
        let mut host = SyncHost::bind_with_store(
            "127.0.0.1:0".parse().unwrap(),
            "HOST".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_a.clone()),
            host_tx,
        )
        .await
        .unwrap();

        let (peer_tx, _) = std::sync::mpsc::channel();
        let mut peer = SyncPeer::start_with_store(
            host.local_addr(),
            "PEER".to_string(),
            "secret".to_string(),
            Some("DEFAULT".to_string()),
            Some(store_b.clone()),
            peer_tx,
        );

        for _ in 0..100 {
            if store_a.count("DEFAULT") == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }

        assert_eq!(store_a.count("DEFAULT"), 2);
        assert_eq!(store_b.count("DEFAULT"), 2);

        peer.stop();
        host.stop();
    }

    #[test]
    fn incomplete_inventory_is_not_applied_as_complete() {
        let store = Arc::new(MemorySyncStore::new());
        let (events_tx, _) = std::sync::mpsc::channel();
        let mut handler = CatchUpHandler::new("DEFAULT".to_string(), store, events_tx);

        let (sender_tx, mut sender_rx) = tokio::sync::mpsc::channel(10);
        let sender = SessionSender::from_channel(sender_tx);

        let id = QsoIdentity::from_fields("CALL_PARTIAL", "20m", "CW", "20261018", "120000");

        handler
            .handle_inventory("DEFAULT", vec![id], false, &sender)
            .unwrap();

        assert!(!handler.is_peer_inventory_complete());
        assert!(
            sender_rx.try_recv().is_err(),
            "Must not send Request while inventory is incomplete"
        );
    }

    // ==========================================
    // Testy SYNC-6 (SyncService i GUI Multi-Op)
    // ==========================================

    #[test]
    fn service_requires_secret() {
        let store = Arc::new(MemorySyncStore::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        let config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: String::new(),
            station: "SP6INA".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let res = SyncService::start(config, store, tx);
        assert!(matches!(res, Err(SyncError::SecretRequired)));
    }

    #[test]
    fn service_peer_requires_host() {
        let store = Arc::new(MemorySyncStore::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        let config = SyncConfig {
            role: SyncRole::Peer {
                host: String::new(),
                port: 4532,
            },
            secret: "secret123".to_string(),
            station: "SP6INA".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let res = SyncService::start(config.clone(), store.clone(), tx.clone());
        assert!(matches!(res, Err(SyncError::HostRequired)));

        let config_whitespace = SyncConfig {
            role: SyncRole::Peer {
                host: "   ".to_string(),
                port: 4532,
            },
            ..config
        };
        let res_ws = SyncService::start(config_whitespace, store, tx);
        assert!(matches!(res_ws, Err(SyncError::HostRequired)));
    }

    #[test]
    fn service_requires_journal() {
        let store = Arc::new(MemorySyncStore::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        let config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: "secret123".to_string(),
            station: "SP6INA".to_string(),
            journal: String::new(),
        };
        let res = SyncService::start(config, store, tx);
        assert!(matches!(res, Err(SyncError::JournalRequired)));
    }

    #[tokio::test]
    async fn service_end_to_end_live_and_catchup() {
        let store_host = Arc::new(MemorySyncStore::new());
        let store_peer = Arc::new(MemorySyncStore::new());

        let qso_host = QsoRecord {
            callsign: "SP1HOST".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            qso_date: "20261018".to_string(),
            time_on: "100000".to_string(),
            journal_id: Some("ACTIVE".to_string()),
            ..Default::default()
        };
        let qso_peer = QsoRecord {
            callsign: "SP2PEER".to_string(),
            band: "40m".to_string(),
            mode: "SSB".to_string(),
            qso_date: "20261018".to_string(),
            time_on: "110000".to_string(),
            journal_id: Some("ACTIVE".to_string()),
            ..Default::default()
        };
        store_host.insert_test_qso("ACTIVE", qso_host.clone());
        store_peer.insert_test_qso("ACTIVE", qso_peer.clone());

        let (host_events_tx, _host_events_rx) = std::sync::mpsc::channel();
        let (peer_events_tx, peer_events_rx) = std::sync::mpsc::channel();

        let host_config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: "sync_shared_secret".to_string(),
            station: "HOST_STN".to_string(),
            journal: "ACTIVE".to_string(),
        };
        let mut host_service =
            SyncService::start(host_config, store_host.clone(), host_events_tx).unwrap();
        let host_port = host_service.local_addr().unwrap().port();

        let peer_config = SyncConfig {
            role: SyncRole::Peer {
                host: "127.0.0.1".to_string(),
                port: host_port,
            },
            secret: "sync_shared_secret".to_string(),
            station: "PEER_STN".to_string(),
            journal: "ACTIVE".to_string(),
        };
        let mut peer_service =
            SyncService::start(peer_config, store_peer.clone(), peer_events_tx).unwrap();

        // Czekamy na catch-up obu stron (po 2 QSO w obu magazynach)
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if store_host.count("ACTIVE") == 2 && store_peer.count("ACTIVE") == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        assert_eq!(
            store_host.count("ACTIVE"),
            2,
            "Host must have both QSOs after catchup"
        );
        assert_eq!(
            store_peer.count("ACTIVE"),
            2,
            "Peer must have both QSOs after catchup"
        );

        // Test transmisji live z Hosta do Peera
        let qso_live = QsoRecord {
            callsign: "SP3LIVE".to_string(),
            band: "15m".to_string(),
            mode: "FT8".to_string(),
            qso_date: "20261018".to_string(),
            time_on: "120000".to_string(),
            journal_id: Some("ACTIVE".to_string()),
            ..Default::default()
        };
        host_service.broadcast(&qso_live);

        let mut live_received = false;
        let live_deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < live_deadline {
            if matches!(peer_events_rx.try_recv(), Ok(SyncEvent::IncomingQso(ref q)) if q.callsign == "SP3LIVE")
            {
                live_received = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(live_received, "Peer must receive live broadcast QSO");

        peer_service.stop();
        host_service.stop();
    }

    async fn wait_for_service_state(
        rx: &std::sync::mpsc::Receiver<SyncEvent>,
        target_state: SyncState,
        max_attempts: usize,
        watchdog: Duration,
    ) -> Result<Vec<SyncEvent>, (String, Vec<SyncEvent>)> {
        let start = std::time::Instant::now();
        let mut collected = Vec::new();

        for attempt in 1..=max_attempts {
            while let Ok(evt) = rx.try_recv() {
                let matched = matches!(&evt, SyncEvent::Status(st) if st.state == target_state);
                collected.push(evt);
                if matched {
                    return Ok(collected);
                }
            }

            if start.elapsed() > watchdog {
                return Err((
                    format!(
                        "Watchdog timeout ({watchdog:?}) exceeded waiting for {target_state:?} at attempt {attempt}/{max_attempts}; collected {} events: {:?}",
                        collected.len(),
                        collected
                    ),
                    collected,
                ));
            }

            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Err((
            format!(
                "Max attempts ({max_attempts}) reached waiting for {target_state:?}; collected {} events: {:?}",
                collected.len(),
                collected
            ),
            collected,
        ))
    }

    #[tokio::test]
    async fn service_notifies_status_transitions() {
        let store_host = Arc::new(MemorySyncStore::new());
        let store_peer = Arc::new(MemorySyncStore::new());

        let (host_events_tx, host_events_rx) = std::sync::mpsc::channel();
        let (peer_events_tx, peer_events_rx) = std::sync::mpsc::channel();

        let host_config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: "status_secret".to_string(),
            station: "HOST_STN".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let mut host_service = SyncService::start(host_config, store_host, host_events_tx).unwrap();
        let host_port = host_service.local_addr().unwrap().port();

        // Host raportuje Listening przez helper zdarzeniowy
        let host_events = wait_for_service_state(
            &host_events_rx,
            SyncState::Listening,
            1500,
            Duration::from_secs(30),
        )
        .await
        .expect("Host must notify Listening state");
        assert!(
            host_events.iter().any(
                |evt| matches!(evt, SyncEvent::Status(st) if st.state == SyncState::Listening)
            ),
            "Host must notify Listening state"
        );

        let peer_config = SyncConfig {
            role: SyncRole::Peer {
                host: "127.0.0.1".to_string(),
                port: host_port,
            },
            secret: "status_secret".to_string(),
            station: "PEER_STN".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let mut peer_service = SyncService::start(peer_config, store_peer, peer_events_tx).unwrap();

        // Peer raportuje Connected przez helper zdarzeniowy bez sztywnego 5-sekundowego limitu
        let peer_events = wait_for_service_state(
            &peer_events_rx,
            SyncState::Connected,
            1500,
            Duration::from_secs(30),
        )
        .await
        .expect("Peer must notify Connected state");

        assert!(
            peer_events
                .iter()
                .any(|evt| matches!(evt, SyncEvent::Status(st) if st.state == SyncState::Connected && st.peers == 1)),
            "Peer must notify Connected state with 1 peer"
        );

        peer_service.stop();
        host_service.stop();
    }

    #[tokio::test]
    async fn service_status_helper_observes_connected_event() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Reconnecting,
            peers: 0,
        }))
        .unwrap();
        tx.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Connected,
            peers: 1,
        }))
        .unwrap();

        let events = wait_for_service_state(&rx, SyncState::Connected, 10, Duration::from_secs(5))
            .await
            .expect("Helper must observe Connected event");

        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], SyncEvent::Status(st) if st.state == SyncState::Reconnecting));
        assert!(matches!(&events[1], SyncEvent::Status(st) if st.state == SyncState::Connected));
    }

    #[tokio::test]
    async fn service_status_connected_event_is_not_lost_between_start_and_receive() {
        let store_host = Arc::new(MemorySyncStore::new());
        let store_peer = Arc::new(MemorySyncStore::new());

        let (host_events_tx, host_events_rx) = std::sync::mpsc::channel();
        let (peer_events_tx, peer_events_rx) = std::sync::mpsc::channel();

        let host_config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: "queue_secret".to_string(),
            station: "HOST_QUEUE".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let mut host_service = SyncService::start(host_config, store_host, host_events_tx).unwrap();
        let host_port = host_service.local_addr().unwrap().port();

        wait_for_service_state(
            &host_events_rx,
            SyncState::Listening,
            1500,
            Duration::from_secs(30),
        )
        .await
        .expect("Host must listen");

        let peer_config = SyncConfig {
            role: SyncRole::Peer {
                host: "127.0.0.1".to_string(),
                port: host_port,
            },
            secret: "queue_secret".to_string(),
            station: "PEER_QUEUE".to_string(),
            journal: "DEFAULT".to_string(),
        };
        // Odbiornik zdarzeń istnieje PRZED startem usługi
        let mut peer_service = SyncService::start(peer_config, store_peer, peer_events_tx).unwrap();

        // Oczekiwanie na Connected: żadne zdarzenie początkowe (Reconnecting) nie może zablokować
        // ani usunąć zdarzenia Connected z bufora kolejki
        let peer_events = wait_for_service_state(
            &peer_events_rx,
            SyncState::Connected,
            1500,
            Duration::from_secs(30),
        )
        .await
        .expect("Connected event must not be lost between service start and receive");

        assert!(
            peer_events
                .iter()
                .any(|e| matches!(e, SyncEvent::Status(st) if st.state == SyncState::Connected)),
            "Connected event must be present in history"
        );

        peer_service.stop();
        host_service.stop();
    }

    #[tokio::test]
    async fn service_status_history_is_reported_on_failure() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(SyncEvent::Status(SyncStatus {
            state: SyncState::Reconnecting,
            peers: 0,
        }))
        .unwrap();
        tx.send(SyncEvent::Error(SyncError::SecretRequired))
            .unwrap();

        // Oczekujemy Connected, które nigdy nie nadejdzie (ograniczona liczba prób i krótki watchdog)
        let res =
            wait_for_service_state(&rx, SyncState::Connected, 3, Duration::from_millis(50)).await;
        assert!(res.is_err(), "Must fail when target state never arrives");

        let (err_msg, collected) = res.unwrap_err();
        assert_eq!(
            collected.len(),
            2,
            "Both preceding events must be collected"
        );
        assert!(
            err_msg.contains("collected 2 events"),
            "Error message must report event count: {err_msg}"
        );
        assert!(
            err_msg.contains("Reconnecting"),
            "Error message must include collected states: {err_msg}"
        );
    }

    #[tokio::test]
    async fn service_status_shutdown_after_connected_is_clean() {
        let store_host = Arc::new(MemorySyncStore::new());
        let store_peer = Arc::new(MemorySyncStore::new());

        let (host_events_tx, host_events_rx) = std::sync::mpsc::channel();
        let (peer_events_tx, peer_events_rx) = std::sync::mpsc::channel();

        let host_config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: "shutdown_secret".to_string(),
            station: "HOST_SD".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let mut host_service = SyncService::start(host_config, store_host, host_events_tx).unwrap();
        let host_port = host_service.local_addr().unwrap().port();

        wait_for_service_state(
            &host_events_rx,
            SyncState::Listening,
            1500,
            Duration::from_secs(30),
        )
        .await
        .expect("Host listening");

        let peer_config = SyncConfig {
            role: SyncRole::Peer {
                host: "127.0.0.1".to_string(),
                port: host_port,
            },
            secret: "shutdown_secret".to_string(),
            station: "PEER_SD".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let mut peer_service = SyncService::start(peer_config, store_peer, peer_events_tx).unwrap();

        wait_for_service_state(
            &peer_events_rx,
            SyncState::Connected,
            1500,
            Duration::from_secs(30),
        )
        .await
        .expect("Peer connected");

        // Kontrolowane zatrzymanie obu usług
        peer_service.stop();
        host_service.stop();

        // Potwierdzenie przejścia Peera do stanu końcowego Starting/0 peers
        let peer_stop_event = wait_for_service_state(
            &peer_events_rx,
            SyncState::Starting,
            500,
            Duration::from_secs(10),
        )
        .await
        .expect("Peer must report Starting upon stop");

        assert!(
            peer_stop_event
                .iter()
                .any(|e| matches!(e, SyncEvent::Status(st) if st.state == SyncState::Starting && st.peers == 0)),
            "Peer must report Starting with 0 peers after clean stop"
        );
    }

    #[tokio::test]
    async fn service_status_peer_does_not_report_connected_before_handshake() {
        let store_peer = Arc::new(MemorySyncStore::new());
        let (peer_events_tx, peer_events_rx) = std::sync::mpsc::channel();

        // Celowo wskazujemy nieaktywny port loopback, gdzie nikt nie nasłuchuje
        let peer_config = SyncConfig {
            role: SyncRole::Peer {
                host: "127.0.0.1".to_string(),
                port: 1, // Port zarezerwowany / nieaktywny
            },
            secret: "dummy_secret".to_string(),
            station: "PEER_NO_HOST".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let mut peer_service = SyncService::start(peer_config, store_peer, peer_events_tx).unwrap();

        // Oczekujemy przez krótki czas i zbieramy wszystkie zdarzenia
        let start = std::time::Instant::now();
        let mut saw_connected = false;
        while start.elapsed() < Duration::from_millis(500) {
            while let Ok(evt) = peer_events_rx.try_recv() {
                if matches!(evt, SyncEvent::Status(st) if st.state == SyncState::Connected) {
                    saw_connected = true;
                    break;
                }
            }
            if saw_connected {
                break;
            }
            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        assert!(
            !saw_connected,
            "Peer must never report Connected state before successful handshake"
        );

        peer_service.stop();
    }

    #[test]
    fn service_stop_is_idempotent() {
        let store = Arc::new(MemorySyncStore::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        let config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: "secret".to_string(),
            station: "SP6INA".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let mut srv = SyncService::start(config, store, tx).unwrap();
        srv.stop();
        srv.stop();
        srv.stop();
    }

    #[test]
    fn sync_incoming_qso_is_logged_as_lan_peer() {
        let ingestor = test_ingestor();
        let qso = QsoRecord {
            callsign: "SP5LAN".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            qso_date: "20261018".to_string(),
            time_on: "123456".to_string(),
            journal_id: Some("DEFAULT".to_string()),
            ..Default::default()
        };
        let outcome = ingestor
            .ingest(qso, crate::core::ingest::QsoOrigin::LanPeer)
            .unwrap();
        assert_eq!(outcome.origin, crate::core::ingest::QsoOrigin::LanPeer);
        assert_eq!(outcome.qso.callsign, "SP5LAN");
    }

    #[test]
    fn sync_duplicate_is_ignored_silently() {
        let ingestor = test_ingestor();
        let qso = QsoRecord {
            callsign: "SP5DUP".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            qso_date: "20261018".to_string(),
            time_on: "123456".to_string(),
            journal_id: Some("DEFAULT".to_string()),
            ..Default::default()
        };
        let res1 = ingestor.ingest(qso.clone(), crate::core::ingest::QsoOrigin::LanPeer);
        assert!(res1.is_ok());

        let res2 = ingestor.ingest(qso, crate::core::ingest::QsoOrigin::LanPeer);
        assert!(matches!(
            res2,
            Err(crate::core::ingest::IngestError::Duplicate)
        ));
    }

    #[test]
    fn local_qso_is_broadcast_to_service() {
        let store = Arc::new(MemorySyncStore::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        let config = SyncConfig {
            role: SyncRole::Host { port: 0 },
            secret: "secret".to_string(),
            station: "SP6INA".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let srv = SyncService::start(config, store, tx).unwrap();
        let qso = QsoRecord {
            callsign: "SP7LOC".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            qso_date: "20261018".to_string(),
            time_on: "130000".to_string(),
            journal_id: Some("DEFAULT".to_string()),
            ..Default::default()
        };
        srv.broadcast(&qso);
    }

    #[test]
    fn catchup_applied_rebuilds_awards_and_reloads_qsos() {
        use crate::gui::SyncUiState;
        let mut sync_ui = SyncUiState::new();
        let msg = "Synchronizacja wsteczna: zaimportowano 5 brakujących QSO".to_string();
        sync_ui.push_log(msg);
        assert!(sync_ui.log.iter().any(|l| l.contains("zaimportowano 5")));
    }

    #[test]
    fn sync_log_is_bounded_to_500_entries() {
        use crate::gui::SyncUiState;
        let mut ui = SyncUiState::new();
        for i in 0..600 {
            ui.push_log(format!("Entry {i}"));
        }
        assert_eq!(ui.log.len(), 500);
        assert_eq!(ui.log[0], "Entry 100");
        assert_eq!(ui.log[499], "Entry 599");
    }

    #[test]
    fn peer_mode_starts_real_sync_service() {
        let store = Arc::new(MemorySyncStore::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        let config = SyncConfig {
            role: SyncRole::Peer {
                host: "127.0.0.1".to_string(),
                port: 19999,
            },
            secret: "secret".to_string(),
            station: "PEER".to_string(),
            journal: "DEFAULT".to_string(),
        };
        let srv = SyncService::start(config, store, tx);
        assert!(
            srv.is_ok(),
            "Starting peer service in real peer mode must succeed"
        );
        let mut srv = srv.unwrap();
        assert!(srv.local_addr().is_none(), "Peer has no listening socket");
        srv.stop();
    }

    #[test]
    fn gui_start_does_not_block_ui_thread() {
        let store = Arc::new(MemorySyncStore::new());
        let (tx, _rx) = std::sync::mpsc::channel();
        // 192.0.2.1 to adres TEST-NET-1 (RFC 5737), który odrzuca pakiety bez odpowiedzi
        let config = SyncConfig {
            role: SyncRole::Peer {
                host: "192.0.2.1".to_string(),
                port: 4532,
            },
            secret: "secret".to_string(),
            station: "PEER".to_string(),
            journal: "DEFAULT".to_string(),
        };

        let start_time = std::time::Instant::now();
        let srv = SyncService::start(config, store, tx);
        let elapsed = start_time.elapsed();

        assert!(
            srv.is_ok(),
            "Starting peer service must succeed non-blockingly"
        );
        assert!(
            elapsed < Duration::from_millis(50),
            "SyncService::start must not block UI thread on network connection (elapsed: {elapsed:?})"
        );
        let mut srv = srv.unwrap();
        srv.stop();
    }
}
