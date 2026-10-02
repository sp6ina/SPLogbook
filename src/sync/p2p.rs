// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Szyfrowana, peer-to-peer synchronizacja dzienników bez chmury (LAN/VPN).
//
// Transport: pojedynczy TCP; ramki są szyfrowane XChaCha20-Poly1305 kluczem
// wyprowadzonym z hasła współdzielonego (Argon2id). Ramka na łączu ma postać:
//
//   [u32 BE długość][nonce 24 B][ciphertext+tag]
//
// Wiadomości są serializowane do JSON przed szyfrowaniem.

use crate::core::qso::QsoRecord;
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};

/// Długość klucza symetrycznego (32 B) i nonce XChaCha20-Poly1305 (24 B).
pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 24;

/// Maksymalna długość ramki na łączu (ochrona przed wyczerpaniem pamięci).
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

/// Wyprowadza klucz symetryczny z hasła współdzielonego i soli (Argon2id).
/// Determinizm: ta sama sól i hasło dają identyczny klucz.
pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
    let params = Params::new(19 * 1024, 2, 1, Some(KEY_LEN))
        .map_err(|e| format!("Błąd parametrów Argon2: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Błąd wyprowadzania klucza: {e}"))?;
    Ok(key)
}

/// Generuje kryptograficznie losową sól (16 B).
pub fn random_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::fill(&mut salt);
    salt
}

/// Szyfruje treść do ramki `[nonce][ciphertext+tag]`.
pub fn encrypt_frame(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let mut nonce = [0u8; NONCE_LEN];
    rand::fill(&mut nonce);
    let nonce_arr = XNonce::from(nonce);
    let ciphertext = cipher
        .encrypt(&nonce_arr, plaintext)
        .map_err(|_| "Szyfrowanie nie powiodło się".to_string())?;
    let mut frame = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    frame.extend_from_slice(&nonce);
    frame.extend_from_slice(&ciphertext);
    Ok(frame)
}

/// Odszyfrowuje ramkę `[nonce][ciphertext+tag]`. Wykrywa modyfikację danych
/// dzięki uwierzytelnieniu AEAD.
pub fn decrypt_frame(key: &[u8; KEY_LEN], frame: &[u8]) -> Result<Vec<u8>, String> {
    if frame.len() < NONCE_LEN {
        return Err("Ramka za krótka (brak nonce)".to_string());
    }
    let (nonce, ciphertext) = frame.split_at(NONCE_LEN);
    let nonce = XNonce::try_from(nonce).map_err(|_| "Nieprawidłowa długość nonce".to_string())?;
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| "Odszyfrowanie nie powiodło się (zły klucz lub uszkodzone dane)".to_string())
}

/// Serializuje wartość do JSON i szyfruje do ramki.
pub fn seal<T: Serialize>(key: &[u8; KEY_LEN], value: &T) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(value).map_err(|e| format!("Błąd serializacji: {e}"))?;
    encrypt_frame(key, &json)
}

/// Odszyfrowuje ramkę i deserializuje JSON do wartości.
pub fn open<T: for<'de> Deserialize<'de>>(key: &[u8; KEY_LEN], frame: &[u8]) -> Result<T, String> {
    let plain = decrypt_frame(key, frame)?;
    serde_json::from_slice(&plain).map_err(|e| format!("Błąd deserializacji: {e}"))
}

/// Długość ramki jako u32 BE (do ramkowania strumienia TCP).
pub fn encode_length(len: usize) -> [u8; 4] {
    (len as u32).to_be_bytes()
}

/// Odczytuje 4-bajtową długość ramki.
pub fn decode_length(bytes: &[u8; 4]) -> usize {
    u32::from_be_bytes(*bytes) as usize
}

/// Wiadomości wymieniane między węzłami synchronizacji P2P.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum P2pMessage {
    /// Uwierzytelnienie/nawiązanie sesji (nazwa stacji + wersja protokołu).
    Hello { station: String, protocol: u32 },
    /// Żądanie pełnego dziennika drugiej strony.
    FullSyncRequest,
    /// Odpowiedź z pełnym zbiorem rekordów.
    FullSyncResponse { qsos: Vec<QsoRecord> },
    /// Pojedynczy nowy/zmieniony rekord.
    QsoPush(Box<QsoRecord>),
    /// Żądanie usunięcia rekordu po identyfikatorze lokalnym.
    QsoDelete { id: i64 },
    /// Potwierdzenie.
    Ack,
}

pub const PROTOCOL_VERSION: u32 = 1;

/// Unikalny, treściowy klucz łączności używany do łączenia/dedup między węzłami.
/// Lokalne identyfikatory bazy różnią się między stacjami, więc scalanie opiera
/// się na zawartości: znak + pasmo + emisja + data + czas.
pub fn qso_key(qso: &QsoRecord) -> String {
    let date: String = qso.qso_date.trim().chars().filter(|&c| c != '-').collect();
    let time: String = qso
        .time_on
        .trim()
        .chars()
        .filter(|&c| c != ':')
        .take(4)
        .collect();
    format!(
        "{}|{}|{}|{}|{}",
        qso.callsign.trim().to_uppercase(),
        qso.band.trim().to_uppercase(),
        qso.mode.trim().to_uppercase(),
        date,
        time
    )
}

/// Liczy wypełnione pola opcjonalne (heurystyka "bogactwa" rekordu).
/// Pola liczbowe (`f64`/`u32`) oraz tekstowe są zliczane osobno, ponieważ
/// w tablicy muszą mieć jednolity typ.
fn richness(qso: &QsoRecord) -> usize {
    let mut count = 0;

    for filled in [
        qso.submode.as_ref(),
        qso.time_off.as_ref(),
        qso.name.as_ref(),
        qso.qth.as_ref(),
        qso.gridsquare.as_ref(),
        qso.state.as_ref(),
        qso.iota.as_ref(),
        qso.sota_ref.as_ref(),
        qso.pota_ref.as_ref(),
        qso.pga_ref.as_ref(),
        qso.country.as_ref(),
        qso.continent.as_ref(),
        qso.comment.as_ref(),
        qso.qsl_via.as_ref(),
        qso.qsl_manager.as_ref(),
        qso.sat_name.as_ref(),
        qso.sat_mode.as_ref(),
        qso.prop_mode.as_ref(),
        qso.srx_string.as_ref(),
        qso.stx_string.as_ref(),
        qso.vucc_grids.as_ref(),
        qso.audio_file.as_ref(),
        qso.qsl_sent_date.as_ref(),
        qso.qsl_rcvd_date.as_ref(),
        qso.lotw_qslrdate.as_ref(),
        qso.eqsl_qslrdate.as_ref(),
        qso.clublog_upload_status.as_ref(),
        qso.qrzcom_upload_status.as_ref(),
        qso.my_gridsquare.as_ref(),
        qso.my_state.as_ref(),
        qso.my_pota_ref.as_ref(),
        qso.my_sota_ref.as_ref(),
    ] {
        if filled.is_some() {
            count += 1;
        }
    }

    if qso.freq.is_some() {
        count += 1;
    }
    if qso.freq_rx.is_some() {
        count += 1;
    }
    if qso.dxcc.is_some() {
        count += 1;
    }
    if qso.cqz.is_some() {
        count += 1;
    }
    if qso.ituz.is_some() {
        count += 1;
    }
    if qso.srx.is_some() {
        count += 1;
    }
    if qso.stx.is_some() {
        count += 1;
    }

    count
}

/// Scala dwa zbiory rekordów. Przy kolizji klucza wygrywa rekord o większej
/// liczbie wypełnionych pól (deterministycznie — brak timestampu w `QsoRecord`).
/// Wynik zachowuje kolejność lokalnego dziennika, a nowe rekordy są dopisywane.
pub fn merge_logs(local: &[QsoRecord], remote: &[QsoRecord]) -> Vec<QsoRecord> {
    use std::collections::HashMap;

    let mut remote_by_key: HashMap<String, &QsoRecord> = HashMap::new();
    for qso in remote {
        remote_by_key.entry(qso_key(qso)).or_insert(qso);
    }

    let mut result: Vec<QsoRecord> = Vec::with_capacity(local.len() + remote.len());
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for qso in local {
        let key = qso_key(qso);
        seen.insert(key.clone());
        if let Some(remote_q) = remote_by_key.get(&key) {
            // Konflikt: wybierz rekord bogatszy w dane.
            if richness(remote_q) > richness(qso) {
                result.push((*remote_q).clone());
            } else {
                result.push(qso.clone());
            }
        } else {
            result.push(qso.clone());
        }
    }

    for qso in remote {
        let key = qso_key(qso);
        if seen.insert(key) {
            result.push(qso.clone());
        }
    }

    result
}

/// Rekordy zdalne, których brak w lokalnym dzienniku (do przyrostowej synchronizacji).
pub fn find_missing<'a>(local: &[QsoRecord], remote: &'a [QsoRecord]) -> Vec<&'a QsoRecord> {
    use std::collections::HashSet;
    let local_keys: HashSet<String> = local.iter().map(qso_key).collect();
    let mut seen = HashSet::new();
    remote
        .iter()
        .filter(|q| {
            let key = qso_key(q);
            seen.insert(key.clone()) && !local_keys.contains(&key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso(call: &str, band: &str, mode: &str, date: &str, time: &str) -> QsoRecord {
        let mut q = QsoRecord::new(call, band, mode);
        q.qso_date = date.to_string();
        q.time_on = time.to_string();
        q
    }

    #[test]
    fn key_derivation_is_deterministic() {
        let salt = b"fixed-salt-123456";
        let k1 = derive_key("tajne-haslo", salt).unwrap();
        let k2 = derive_key("tajne-haslo", salt).unwrap();
        assert_eq!(k1, k2);
        let k3 = derive_key("inne-haslo", salt).unwrap();
        assert_ne!(k1, k3);
    }

    #[test]
    fn different_salt_yields_different_key() {
        let k1 = derive_key("pass", b"aaaaaaaaaaaaaaaa").unwrap();
        let k2 = derive_key("pass", b"bbbbbbbbbbbbbbbb").unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = derive_key("haslo", b"sol-0123456789").unwrap();
        let frame = encrypt_frame(&key, b"tajne dane QSO").unwrap();
        let plain = decrypt_frame(&key, &frame).unwrap();
        assert_eq!(plain, b"tajne dane QSO");
    }

    #[test]
    fn tampered_frame_is_rejected() {
        let key = derive_key("haslo", b"sol-0123456789").unwrap();
        let mut frame = encrypt_frame(&key, b"dane").unwrap();
        let last = frame.len() - 1;
        frame[last] ^= 0xFF;
        assert!(decrypt_frame(&key, &frame).is_err());
    }

    #[test]
    fn wrong_key_is_rejected() {
        let key = derive_key("haslo-a", b"sol-0123456789").unwrap();
        let other = derive_key("haslo-b", b"sol-0123456789").unwrap();
        let frame = encrypt_frame(&key, b"dane").unwrap();
        assert!(decrypt_frame(&other, &frame).is_err());
    }

    #[test]
    fn seal_open_message_roundtrip() {
        let key = derive_key("haslo", b"sol-0123456789").unwrap();
        let msg = P2pMessage::Hello {
            station: "SP6INA".to_string(),
            protocol: PROTOCOL_VERSION,
        };
        let frame = seal(&key, &msg).unwrap();
        let decoded: P2pMessage = open(&key, &frame).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn length_encoding_roundtrip() {
        for len in [0usize, 1, 255, 65_536, 4_000_000_000] {
            let enc = encode_length(len);
            assert_eq!(decode_length(&enc), len);
        }
    }

    #[test]
    fn qso_key_normalizes_case_and_whitespace() {
        let a = qso(" sp6ina ", "20M", "cw", "2026-01-01", "12:00");
        let b = qso("SP6INA", "20m", "CW", "2026-01-01", "12:00");
        assert_eq!(qso_key(&a), qso_key(&b));
    }

    #[test]
    fn merge_keeps_local_order_and_appends_new() {
        let local = vec![qso("A", "20m", "CW", "2026-01-01", "00:00")];
        let remote = vec![
            qso("B", "20m", "CW", "2026-01-01", "00:01"),
            qso("A", "20m", "CW", "2026-01-01", "00:00"),
        ];
        let merged = merge_logs(&local, &remote);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].callsign, "A");
        assert_eq!(merged[1].callsign, "B");
    }

    #[test]
    fn merge_prefers_richer_record_on_conflict() {
        let local_q = qso("A", "20m", "CW", "2026-01-01", "00:00");
        let mut remote_q = qso("A", "20m", "CW", "2026-01-01", "00:00");
        remote_q.name = Some("Mariusz".to_string());
        remote_q.qth = Some("Warszawa".to_string());

        let merged = merge_logs(
            std::slice::from_ref(&local_q),
            std::slice::from_ref(&remote_q),
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name.as_deref(), Some("Mariusz"));
    }

    #[test]
    fn find_missing_returns_only_new_records() {
        let local = vec![qso("A", "20m", "CW", "2026-01-01", "00:00")];
        let remote = vec![
            qso("A", "20m", "CW", "2026-01-01", "00:00"),
            qso("B", "20m", "CW", "2026-01-01", "00:01"),
            qso("B", "20m", "CW", "2026-01-01", "00:01"), // duplikat
        ];
        let missing = find_missing(&local, &remote);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].callsign, "B");
    }
}
