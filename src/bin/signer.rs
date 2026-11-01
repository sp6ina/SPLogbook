// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::env;
use std::fs;

fn print_usage() {
    println!("Użycie: splogbook-signer <komenda> [argumenty]");
    println!("Komendy:");
    println!("  keygen <out_priv.key>             - Generuje nową parę kluczy Ed25519");
    println!("  sign <priv.key> <manifest.json>   - Podpisuje manifest i tworzy manifest.json.sig");
    println!("  verify <manifest.json> <sig_file> [key_file] - Weryfikuje podpis manifestu");
    println!(
        "  fingerprint <priv.key>            - Oblicza fingerprint SHA-256 klucza publicznego"
    );
    println!(
        "  hash <plik>                       - Oblicza SHA-256 dla pliku (do wklejenia w manifest)"
    );
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Err("Zbyt mało argumentów".to_string());
    }

    let command = args[1].as_str();
    match command {
        "keygen" => {
            if args.len() < 3 {
                return Err("Brakujący argument <out_priv.key>".to_string());
            }
            keygen(&args[2])?;
        }
        "sign" => {
            if args.len() < 4 {
                return Err("Brakujące argumenty: sign <priv.key> <manifest.json>".to_string());
            }
            sign(&args[2], &args[3])?;
        }
        "verify" => {
            if args.len() < 4 {
                return Err(
                    "Brakujące argumenty: verify <manifest.json> <sig_file> [key_file]".to_string(),
                );
            }
            let key_opt = if args.len() >= 5 {
                Some(args[4].as_str())
            } else {
                None
            };
            verify_manifest(&args[2], &args[3], key_opt)?;
        }
        "fingerprint" => {
            if args.len() < 3 {
                return Err("Brakujący argument <priv.key>".to_string());
            }
            show_fingerprint(&args[2])?;
        }
        "hash" => {
            if args.len() < 3 {
                return Err("Brakujący argument <plik>".to_string());
            }
            hash_file(&args[2])?;
        }
        _ => {
            print_usage();
            return Err(format!("Nieznana komenda: {}", command));
        }
    }

    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    use std::fmt::Write;
    for b in result {
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

fn keygen(out_path: &str) -> Result<(), String> {
    use ed25519_dalek::SigningKey;

    let secret_bytes: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&secret_bytes);

    let secret_bytes = signing_key.to_bytes();
    let public_bytes = signing_key.verifying_key().to_bytes();

    fs::write(out_path, secret_bytes)
        .map_err(|e| format!("Błąd zapisu klucza prywatnego: {}", e))?;

    println!("Klucz prywatny zapisany w: {}", out_path);
    println!("NIGDY NIE PUBLIKUJ TEGO KLUCZA.");
    println!("========================================");
    println!("Twój nowy kod zaufanego klucza publicznego (Rust array):");
    println!("{:?}", public_bytes);
    println!(
        "Fingerprint SHA-256 (public key): {}",
        sha256_hex(&public_bytes)
    );
    println!("Wklej powyższą tablicę do `TRUSTED_KEYS` w `src/cloud/updater.rs`.");

    Ok(())
}

fn show_fingerprint(priv_key_path: &str) -> Result<(), String> {
    use ed25519_dalek::SigningKey;

    let secret_bytes =
        fs::read(priv_key_path).map_err(|e| format!("Błąd odczytu pliku klucza: {}", e))?;
    if secret_bytes.len() != 32 {
        return Err("Plik klucza prywatnego musi mieć dokładnie 32 bajty.".to_string());
    }

    let secret_arr: [u8; 32] = secret_bytes.try_into().map_err(|_| {
        "Błąd konwersji klucza prywatnego: nieprawidłowa długość tablicy.".to_string()
    })?;
    let signing_key = SigningKey::from_bytes(&secret_arr);
    let public_bytes = signing_key.verifying_key().to_bytes();

    println!("Public key: {:?}", public_bytes);
    println!("Fingerprint SHA-256: {}", sha256_hex(&public_bytes));
    Ok(())
}

fn sign(priv_key_path: &str, manifest_path: &str) -> Result<(), String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signer, SigningKey};

    let secret_bytes =
        fs::read(priv_key_path).map_err(|e| format!("Błąd odczytu klucza prywatnego: {}", e))?;
    if secret_bytes.len() != 32 {
        return Err("Plik klucza prywatnego musi mieć dokładnie 32 bajty.".to_string());
    }

    let secret_arr: [u8; 32] = secret_bytes.try_into().map_err(|_| {
        "Błąd konwersji klucza prywatnego: nieprawidłowa długość tablicy.".to_string()
    })?;
    let signing_key = SigningKey::from_bytes(&secret_arr);

    let manifest_bytes =
        fs::read(manifest_path).map_err(|e| format!("Błąd odczytu manifestu: {}", e))?;

    let signature = signing_key.sign(&manifest_bytes);

    let sig_base64 = STANDARD.encode(signature.to_bytes());

    let out_sig_path = format!("{}.sig", manifest_path);
    fs::write(&out_sig_path, sig_base64)
        .map_err(|e| format!("Błąd zapisu pliku podpisu: {}", e))?;

    println!("Pomyślnie utworzono podpis: {}", out_sig_path);
    Ok(())
}

fn hash_file(file_path: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = fs::File::open(file_path).map_err(|e| format!("Błąd otwarcia pliku: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];

    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Błąd odczytu pliku: {}", e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    use std::fmt::Write;
    for b in result {
        let _ = write!(hex, "{:02x}", b);
    }

    println!("SHA-256: {}", hex);

    Ok(())
}

fn verify_manifest(
    manifest_path: &str,
    sig_path: &str,
    maybe_key_path: Option<&str>,
) -> Result<(), String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let manifest_bytes =
        fs::read(manifest_path).map_err(|e| format!("Błąd odczytu manifestu: {}", e))?;
    let sig_raw =
        fs::read_to_string(sig_path).map_err(|e| format!("Błąd odczytu pliku podpisu: {}", e))?;
    let sig_str = sig_raw.trim();

    let sig_bytes = STANDARD
        .decode(sig_str)
        .map_err(|e| format!("Błąd dekodowania podpisu Base64: {}", e))?;
    if sig_bytes.len() != 64 {
        return Err("Nieprawidłowa długość podpisu Ed25519 (wymagane 64 bajty)".to_string());
    }
    let signature =
        Signature::from_slice(&sig_bytes).map_err(|e| format!("Błąd ładowania podpisu: {}", e))?;

    let try_key_bytes = |bytes: &[u8]| -> Option<VerifyingKey> {
        if bytes.len() != 32 {
            return None;
        }
        let arr: [u8; 32] = match bytes.try_into() {
            Ok(a) => a,
            Err(_) => return None,
        };

        let sk = ed25519_dalek::SigningKey::from_bytes(&arr);
        let vk = sk.verifying_key();
        if vk.verify(&manifest_bytes, &signature).is_ok() {
            return Some(vk);
        }

        if let Ok(raw_vk) = VerifyingKey::from_bytes(&arr)
            && raw_vk.verify(&manifest_bytes, &signature).is_ok()
        {
            return Some(raw_vk);
        }

        None
    };

    if let Some(key_path) = maybe_key_path {
        let key_bytes =
            fs::read(key_path).map_err(|e| format!("Błąd odczytu klucza z {}: {}", key_path, e))?;
        if let Some(vk) = try_key_bytes(&key_bytes) {
            println!(
                "Podpis zweryfikowany pomyślnie kluczem {} (fingerprint: {})",
                key_path,
                sha256_hex(&vk.to_bytes())
            );
            return Ok(());
        }
        return Err(format!(
            "Weryfikacja podpisu kluczem {} nie powiodła się: sygnatura nie pasuje do manifestu",
            key_path
        ));
    }

    match splogbook::cloud::updater::verify_manifest_signature(&manifest_bytes, sig_str) {
        Ok((trusted_key, manifest)) => {
            println!(
                "Podpis zweryfikowany pomyślnie zaufanym kluczem produkcyjnym: {} ({}) dla wersji {}",
                trusted_key.key_id,
                sha256_hex(&trusted_key.public_key),
                manifest.version
            );
            Ok(())
        }
        Err(e) => {
            for candidate in &["dry_run.key", "../dry_run.key", "priv.key", "../priv.key"] {
                if let Ok(key_bytes) = fs::read(candidate)
                    && let Some(vk) = try_key_bytes(&key_bytes)
                {
                    println!(
                        "Podpis zweryfikowany pomyślnie kluczem roboczym: {} (fingerprint: {})",
                        candidate,
                        sha256_hex(&vk.to_bytes())
                    );
                    return Ok(());
                }
            }
            Err(format!("Weryfikacja podpisu nie powiodła się: {e}"))
        }
    }
}
