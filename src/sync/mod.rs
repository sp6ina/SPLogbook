// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Zunifikowany podsystem synchronizacji QSO Host/Peer (SYNC-1..SYNC-7).

pub mod crypto;
pub mod error;
pub mod frame;
pub mod handshake;
pub mod host;
pub mod peer;
pub mod protocol;
pub mod service;
pub mod session;

pub use crypto::*;
pub use error::SyncError;
pub use frame::{MAX_FRAME_BYTES, decode_length, encode_length, read_frame, write_frame};
pub use handshake::{HandshakeResult, host_handshake, peer_handshake};
pub use host::{IDLE_TIMEOUT, MAX_ACTIVE_PEERS, MAX_HANDSHAKES, PING_INTERVAL, SyncHost};
pub use peer::{INITIAL_BACKOFF, MAX_BACKOFF, ReconnectBackoff, SyncPeer};
pub use protocol::{ClientHello, MAGIC, Message, PROTOCOL_VERSION, Reject, ServerHello};
pub use service::{SyncConfig, SyncEvent, SyncRole, SyncState, SyncStatus};
pub use session::{QUEUE_CAPACITY, SessionSender, SyncSession};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::qso::QsoRecord;
    use std::io::Cursor;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    #[tokio::test]
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

        for _ in 0..100 {
            if host.connected_peers() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(host.connected_peers(), 1);

        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(46)).await;

        for _ in 0..100 {
            if host.connected_peers() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        assert_eq!(host.connected_peers(), 0);

        let mut timeout_reported = false;
        while let Ok(evt) = host_rx.try_recv() {
            if let SyncEvent::Error(SyncError::Timeout) = evt {
                timeout_reported = true;
            }
        }
        assert!(
            timeout_reported,
            "Timeout error must be reported for idle peer"
        );

        drop(handshake_res);
        host.stop();
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
}
