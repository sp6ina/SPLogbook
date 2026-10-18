// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Zunifikowany podsystem synchronizacji QSO Host/Peer (SYNC-1..SYNC-7).

pub mod crypto;
pub mod error;
pub mod frame;
pub mod protocol;

pub use crypto::*;
pub use error::SyncError;
pub use frame::{MAX_FRAME_BYTES, decode_length, encode_length, read_frame, write_frame};
pub use protocol::{ClientHello, MAGIC, Message, PROTOCOL_VERSION, Reject, ServerHello};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::qso::QsoRecord;
    use std::io::Cursor;

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

        // Odbiorca utrzymuje oczekiwany licznik wiadomości
        let mut expected_counter: u64 = 0;

        let aad_msg0 = build_aad(&sid, DIRECTION_PEER_TO_HOST, 0);
        let frame0 = encrypt_payload(&key, &aad_msg0, b"first-message").unwrap();

        // Pierwsza wiadomość odszyfrowana pomyślnie
        let dec0 = decrypt_payload(&key, &aad_msg0, &frame0).unwrap();
        assert_eq!(dec0, b"first-message");
        expected_counter += 1; // oczekiwany licznik to 1

        // Próba ponownego przesłania (replay) wiadomości z licznikiem 0
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

        // Ramka krótsza niż NONCE_LEN (24) + Poly1305 tag (16) = 40 B
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
}
