use std::env;
use std::fs;

fn print_usage() {
    println!("Użycie: splogbook-signer <komenda> [argumenty]");
    println!("Komendy:");
    println!("  keygen <out_priv.key>             - Generuje nową parę kluczy Ed25519");
    println!("  sign <priv.key> <manifest.json>   - Podpisuje manifest i tworzy manifest.json.sig");
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
    println!("Wklej powyższą tablicę do `TRUSTED_KEYS` w `src/cloud/updater.rs`.");

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

    let secret_arr: [u8; 32] = secret_bytes.try_into().unwrap();
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
