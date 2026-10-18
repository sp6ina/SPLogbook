// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Prymitywy kryptograficzne dla szyfrowanego protokołu synchronizacji v2 (`src/sync/crypto.rs`).

use super::error::SyncError;
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use sha2::{Digest, Sha256};

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 24;
pub const SALT_LEN: usize = 16;
pub const SESSION_ID_LEN: usize = 32;
pub const AAD_LEN: usize = 41; // 32 B session_id + 1 B direction + 8 B counter

pub const DIRECTION_PEER_TO_HOST: u8 = 0;
pub const DIRECTION_HOST_TO_PEER: u8 = 1;

/// Wyprowadza 32-bajtowy klucz symetryczny za pomocą Argon2id (19 MiB RAM, t=2, p=1).
pub fn derive_key(secret: &str, salt: &[u8; SALT_LEN]) -> Result<[u8; KEY_LEN], SyncError> {
    if secret.trim().is_empty() {
        return Err(SyncError::SecretRequired);
    }

    let params = match Params::new(19 * 1024, 2, 1, Some(KEY_LEN)) {
        Ok(p) => p,
        Err(e) => return Err(SyncError::KeyDerivationFailed(e.to_string())),
    };

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    if let Err(e) = argon2.hash_password_into(secret.as_bytes(), salt, &mut key) {
        return Err(SyncError::KeyDerivationFailed(e.to_string()));
    }

    Ok(key)
}

/// Generuje losową 16-bajtową sól dla serwera.
pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::fill(&mut salt);
    salt
}

/// Generuje losowy 16-bajtowy nonce (dla ClientHello / ServerHello).
pub fn random_nonce_16() -> [u8; 16] {
    let mut nonce = [0u8; 16];
    rand::fill(&mut nonce);
    nonce
}

/// Generuje losowy 24-bajtowy nonce dla XChaCha20-Poly1305.
pub fn random_nonce_24() -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    rand::fill(&mut nonce);
    nonce
}

/// Oblicza unikalny identyfikator sesji: SHA-256(client_nonce || server_nonce).
pub fn compute_session_id(
    client_nonce: &[u8; 16],
    server_nonce: &[u8; 16],
) -> [u8; SESSION_ID_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(client_nonce);
    hasher.update(server_nonce);
    hasher.finalize().into()
}

/// Buduje wektor AAD o stałej długości 41 bajtów:
/// session_id (32 B) || direction (1 B) || counter (8 B BE).
pub fn build_aad(session_id: &[u8; SESSION_ID_LEN], direction: u8, counter: u64) -> [u8; AAD_LEN] {
    let mut aad = [0u8; AAD_LEN];
    aad[..32].copy_from_slice(session_id);
    aad[32] = direction;
    aad[33..41].copy_from_slice(&counter.to_be_bytes());
    aad
}

/// Szyfruje dane za pomocą XChaCha20-Poly1305 z losowym nonce i powiązanym AAD.
/// Zwraca format: [nonce 24 B][ciphertext + tag].
pub fn encrypt_payload(
    key: &[u8; KEY_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, SyncError> {
    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let nonce = random_nonce_24();
    let nonce_arr = XNonce::from(nonce);

    let payload = Payload {
        msg: plaintext,
        aad,
    };

    let ciphertext = cipher
        .encrypt(&nonce_arr, payload)
        .map_err(|_| SyncError::EncryptionFailed)?;

    let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Odszyfrowuje ramkę [nonce 24 B][ciphertext + tag] z weryfikacją AAD.
pub fn decrypt_payload(
    key: &[u8; KEY_LEN],
    aad: &[u8],
    frame: &[u8],
) -> Result<Vec<u8>, SyncError> {
    const MIN_FRAME_LEN: usize = NONCE_LEN + 16; // 24 B nonce + 16 B Poly1305 tag
    if frame.len() < MIN_FRAME_LEN {
        return Err(SyncError::FrameTruncated);
    }

    let (nonce_bytes, ciphertext) = frame.split_at(NONCE_LEN);
    let Ok(nonce_arr) = XNonce::try_from(nonce_bytes) else {
        return Err(SyncError::InvalidNonce);
    };

    let cipher = XChaCha20Poly1305::new(&Key::from(*key));
    let payload = Payload {
        msg: ciphertext,
        aad,
    };

    cipher
        .decrypt(&nonce_arr, payload)
        .map_err(|_| SyncError::DecryptionFailed)
}
