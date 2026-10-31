// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use splogbook::cloud::updater::{
    REVOKED_KEY_IDS, REVOKED_PUBLIC_KEYS, TRUSTED_KEYS, TrustedKey, is_key_revoked, sha256_hex,
    verify_manifest_signature, verify_manifest_signature_with_keys,
};
use std::collections::HashSet;
use std::fs;
use std::process::Command;

#[test]
fn updater_trusted_keys_reject_known_test_and_trivial_seeds() {
    let revoked_fingerprints: HashSet<String> = REVOKED_PUBLIC_KEYS
        .iter()
        .map(|pk| sha256_hex(pk))
        .collect();

    let revoked_key_ids: HashSet<&str> = REVOKED_KEY_IDS.iter().copied().collect();

    assert!(
        revoked_key_ids.contains("splogbook-release-2026-01"),
        "Klucz 'splogbook-release-2026-01' musi znajdować się na liście REVOKED_KEY_IDS (SEC-001)"
    );
    assert!(
        !REVOKED_PUBLIC_KEYS.is_empty(),
        "Lista REVOKED_PUBLIC_KEYS nie może być pusta"
    );

    let mut seen_key_ids = HashSet::new();

    for key in TRUSTED_KEYS {
        assert_eq!(
            key.public_key.len(),
            32,
            "Klucz {} musi mieć dokładnie 32 bajty",
            key.key_id
        );

        assert!(
            seen_key_ids.insert(key.key_id),
            "Identyfikator klucza '{}' jest zduplikowany w TRUSTED_KEYS",
            key.key_id
        );

        let fp = sha256_hex(&key.public_key);

        assert!(
            !revoked_fingerprints.contains(&fp),
            "Klucz '{}' posiada odwołany fingerprint SHA256: {}",
            key.key_id,
            fp
        );

        if revoked_key_ids.contains(key.key_id) {
            assert!(
                !key.active,
                "Klucz '{}' jest odwołany w REVOKED_KEY_IDS i nie może mieć statusu active: true",
                key.key_id
            );
            assert!(
                is_key_revoked(key.key_id, &key.public_key),
                "is_key_revoked musi zwracać true dla klucza '{}'",
                key.key_id
            );
        }
    }

    let revoked_manifest = r#"{
        "manifest_version": 1,
        "key_id": "splogbook-release-2026-01",
        "product": "SPLogbook",
        "version": "1.2.1",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "0000000000000000000000000000000000000000",
        "assets": []
    }"#;

    let secret_bytes: [u8; 32] = rand::random();
    let ephemeral_signing_key = SigningKey::from_bytes(&secret_bytes);
    let ephemeral_sig = ephemeral_signing_key.sign(revoked_manifest.as_bytes());
    let sig_b64 = STANDARD.encode(ephemeral_sig.to_bytes());

    let result = verify_manifest_signature(revoked_manifest.as_bytes(), &sig_b64);
    assert!(
        result.is_err(),
        "verify_manifest_signature musi odrzucić manifest z odwołanym key_id"
    );

    let untrusted_manifest = r#"{
        "manifest_version": 1,
        "key_id": "untrusted-test-key-999",
        "product": "SPLogbook",
        "version": "1.2.1",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "0000000000000000000000000000000000000000",
        "assets": []
    }"#;
    let untrusted_sig = ephemeral_signing_key.sign(untrusted_manifest.as_bytes());
    let untrusted_sig_b64 = STANDARD.encode(untrusted_sig.to_bytes());

    let res_untrusted =
        verify_manifest_signature(untrusted_manifest.as_bytes(), &untrusted_sig_b64);
    assert!(
        res_untrusted.is_err(),
        "verify_manifest_signature musi odrzucić podpis wykonany kluczem testowym"
    );

    let active_count = TRUSTED_KEYS.iter().filter(|k| k.active).count();
    assert_eq!(
        active_count, 1,
        "Po wdrożeniu SEC-001 Etap 1B musi istnieć dokładnie jeden aktywny klucz zaufany"
    );
}

#[test]
fn trusted_keys_have_unique_key_ids() {
    let mut seen = HashSet::new();
    for key in TRUSTED_KEYS {
        assert!(
            seen.insert(key.key_id),
            "Zduplikowany key_id w TRUSTED_KEYS: {}",
            key.key_id
        );
    }
}

#[test]
fn trusted_keys_have_unique_public_keys() {
    let mut seen = HashSet::new();
    for key in TRUSTED_KEYS {
        assert!(
            seen.insert(key.public_key),
            "Zduplikowany public_key w TRUSTED_KEYS dla key_id: {}",
            key.key_id
        );
    }
}

#[test]
fn trusted_keys_have_valid_public_key_lengths() {
    for key in TRUSTED_KEYS {
        assert_eq!(
            key.public_key.len(),
            32,
            "Klucz {} ma niepoprawną długość",
            key.key_id
        );
    }
    for revoked in REVOKED_PUBLIC_KEYS {
        assert_eq!(
            revoked.len(),
            32,
            "Odwołany klucz publiczny ma niepoprawną długość"
        );
    }
}

#[test]
fn exactly_one_release_key_is_active() {
    let active_keys: Vec<&TrustedKey> = TRUSTED_KEYS.iter().filter(|k| k.active).collect();
    assert_eq!(
        active_keys.len(),
        1,
        "W TRUSTED_KEYS musi być dokładnie jeden aktywny klucz"
    );
    assert_eq!(
        active_keys[0].key_id, "splogbook-release-2026-02",
        "Aktywny klucz musi mieć identyfikator splogbook-release-2026-02"
    );
}

#[test]
fn active_release_key_is_not_revoked() {
    let active = TRUSTED_KEYS
        .iter()
        .find(|k| k.active)
        .expect("Brak aktywnego klucza");
    assert!(
        !REVOKED_KEY_IDS.contains(&active.key_id),
        "Aktywny key_id nie może znajdować się na liście REVOKED_KEY_IDS"
    );
    assert!(
        !REVOKED_PUBLIC_KEYS.contains(&active.public_key),
        "Aktywny klucz publiczny nie może znajdować się na liście REVOKED_PUBLIC_KEYS"
    );
    assert!(
        !is_key_revoked(active.key_id, &active.public_key),
        "is_key_revoked musi zwracać false dla aktywnego klucza"
    );
}

#[test]
fn revoked_key_ids_can_never_be_active() {
    for revoked_id in REVOKED_KEY_IDS {
        for key in TRUSTED_KEYS {
            if key.key_id == *revoked_id {
                assert!(
                    !key.active,
                    "Klucz '{}' z listy REVOKED_KEY_IDS nie może być aktywny",
                    key.key_id
                );
            }
        }
    }
}

#[test]
fn revoked_public_keys_can_never_be_trusted() {
    for revoked_pk in REVOKED_PUBLIC_KEYS {
        for key in TRUSTED_KEYS.iter().filter(|k| k.active) {
            assert_ne!(
                &key.public_key, revoked_pk,
                "Aktywny klucz {} posiada odwołany materiał publiczny",
                key.key_id
            );
        }
    }
}

#[test]
fn new_release_key_has_expected_public_fingerprint() {
    let active = TRUSTED_KEYS
        .iter()
        .find(|k| k.active)
        .expect("Brak aktywnego klucza");
    assert_eq!(active.key_id, "splogbook-release-2026-02");
    let fp = sha256_hex(&active.public_key);
    assert_eq!(
        fp, "2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a",
        "Fingerprint aktywnego klucza publicznego musi odpowiadać potwierdzonej wartości właściciela"
    );
}

#[test]
fn manifest_signed_by_matching_active_key_is_accepted() {
    // Generujemy bezpieczny efemeryczny klucz w pamięci (izolowany do testu)
    let secret_bytes: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let public_bytes = signing_key.verifying_key().to_bytes();

    let ephemeral_key = TrustedKey {
        key_id: "test-ephemeral-2026-key",
        public_key: public_bytes,
        active: true,
        min_version: Some("1.2.0"),
        max_version: None,
    };

    let manifest_json = r#"{
        "manifest_version": 1,
        "key_id": "test-ephemeral-2026-key",
        "product": "SPLogbook",
        "version": "1.2.1",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "1111111111111111111111111111111111111111",
        "assets": [
            {
                "platform": "windows",
                "arch": "x86_64",
                "package_type": "zip",
                "filename": "SPLogbook-Windows-x64.zip",
                "size": 12345,
                "sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
            }
        ]
    }"#;

    let signature = signing_key.sign(manifest_json.as_bytes());
    let sig_b64 = STANDARD.encode(signature.to_bytes());

    let test_trusted_keys = [ephemeral_key];
    let test_revoked_ids: [&str; 0] = [];
    let test_revoked_pks: [[u8; 32]; 0] = [];

    // 1. Poprawny podpis przechodzi pomyślnie
    let verified = verify_manifest_signature_with_keys(
        manifest_json.as_bytes(),
        &sig_b64,
        &test_trusted_keys,
        &test_revoked_ids,
        &test_revoked_pks,
    );
    assert!(
        verified.is_ok(),
        "Poprawny podpis aktywnym kluczem musi zostać zaakceptowany: {:?}",
        verified.err()
    );
    let (key_ref, manifest) = verified.unwrap();
    assert_eq!(key_ref.key_id, "test-ephemeral-2026-key");
    assert_eq!(manifest.version, "1.2.1");

    // 2. Zły podpis nie przechodzi
    let mut bad_sig_bytes = signature.to_bytes();
    bad_sig_bytes[0] ^= 0xFF;
    let bad_sig_b64 = STANDARD.encode(bad_sig_bytes);
    let res_bad_sig = verify_manifest_signature_with_keys(
        manifest_json.as_bytes(),
        &bad_sig_b64,
        &test_trusted_keys,
        &test_revoked_ids,
        &test_revoked_pks,
    );
    assert!(
        res_bad_sig.is_err(),
        "Zmodyfikowany podpis musi zostać odrzucony"
    );

    // 3. Modyfikacja manifestu po podpisaniu nie przechodzi
    let tampered_manifest = manifest_json.replace("1.2.1", "1.2.2");
    let res_tampered = verify_manifest_signature_with_keys(
        tampered_manifest.as_bytes(),
        &sig_b64,
        &test_trusted_keys,
        &test_revoked_ids,
        &test_revoked_pks,
    );
    assert!(
        res_tampered.is_err(),
        "Zmodyfikowana treść manifestu musi unieważnić podpis"
    );

    // 4. Inny key_id w manifeście nie przechodzi
    let mismatched_manifest =
        manifest_json.replace("test-ephemeral-2026-key", "different-declared-key");
    let mismatch_sig = signing_key.sign(mismatched_manifest.as_bytes());
    let mismatch_sig_b64 = STANDARD.encode(mismatch_sig.to_bytes());
    let res_mismatch = verify_manifest_signature_with_keys(
        mismatched_manifest.as_bytes(),
        &mismatch_sig_b64,
        &test_trusted_keys,
        &test_revoked_ids,
        &test_revoked_pks,
    );
    assert!(
        res_mismatch.is_err(),
        "Niezgodność key_id w manifeście musi zostać odrzucona"
    );
}

#[test]
fn all_revoked_release_keys_are_rejected() {
    let secret_bytes: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let public_bytes = signing_key.verifying_key().to_bytes();

    let manifest_json = r#"{
        "manifest_version": 1,
        "key_id": "revoked-test-key-id",
        "product": "SPLogbook",
        "version": "1.2.1",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "0000000000000000000000000000000000000000",
        "assets": []
    }"#;

    let signature = signing_key.sign(manifest_json.as_bytes());
    let sig_b64 = STANDARD.encode(signature.to_bytes());

    // 1. Klucz z odwołanym key_id
    let revoked_key = TrustedKey {
        key_id: "revoked-test-key-id",
        public_key: public_bytes,
        active: true,
        min_version: Some("1.1.0"),
        max_version: None,
    };
    let trusted_keys = [revoked_key];
    let revoked_ids = ["revoked-test-key-id"];
    let empty_revoked_pks: [[u8; 32]; 0] = [];

    let res = verify_manifest_signature_with_keys(
        manifest_json.as_bytes(),
        &sig_b64,
        &trusted_keys,
        &revoked_ids,
        &empty_revoked_pks,
    );
    assert!(
        res.is_err(),
        "Klucz z odwołanym key_id musi zostać bezwzględnie odrzucony"
    );

    // 2. Klucz z odwołanymi publicznymi bajtami
    let revoked_pk_list = [public_bytes];
    let empty_revoked_ids: [&str; 0] = [];
    let res_pk = verify_manifest_signature_with_keys(
        manifest_json.as_bytes(),
        &sig_b64,
        &trusted_keys,
        &empty_revoked_ids,
        &revoked_pk_list,
    );
    assert!(
        res_pk.is_err(),
        "Klucz z odwołanym kluczem publicznym musi zostać bezwzględnie odrzucony"
    );

    // 3. Manifest wskazujący stary historyczny key_id odrzucony przez produkcyjne TRUSTED_KEYS
    let legacy_manifest = r#"{
        "manifest_version": 1,
        "key_id": "splogbook-release-2026-01",
        "product": "SPLogbook",
        "version": "1.2.1",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "0000000000000000000000000000000000000000",
        "assets": []
    }"#;
    let res_legacy = verify_manifest_signature(legacy_manifest.as_bytes(), &sig_b64);
    assert!(
        res_legacy.is_err(),
        "Produkcyjny updater musi odrzucić manifest z odwołanym splogbook-release-2026-01"
    );

    // 4. Manifest z nieaktywnym kluczem (active: false)
    let inactive_key = TrustedKey {
        key_id: "inactive-test-key",
        public_key: public_bytes,
        active: false,
        min_version: Some("1.1.0"),
        max_version: None,
    };
    let inactive_keys = [inactive_key];
    let inactive_manifest = manifest_json.replace("revoked-test-key-id", "inactive-test-key");
    let inactive_sig = signing_key.sign(inactive_manifest.as_bytes());
    let inactive_sig_b64 = STANDARD.encode(inactive_sig.to_bytes());
    let res_inactive = verify_manifest_signature_with_keys(
        inactive_manifest.as_bytes(),
        &inactive_sig_b64,
        &inactive_keys,
        &empty_revoked_ids,
        &empty_revoked_pks,
    );
    assert!(
        res_inactive.is_err(),
        "Klucz z active: false musi zostać odrzucony"
    );
}

#[test]
fn signer_generated_key_signs_and_updater_verifies_manifest() {
    let signer_bin = env!("CARGO_BIN_EXE_splogbook-signer");
    let temp_dir =
        std::env::temp_dir().join(format!("splog_signer_test_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).expect("Nie udało się utworzyć katalogu tymczasowego");

    let priv_key_path = temp_dir.join("test_key.priv");
    let manifest_path = temp_dir.join("test_manifest.json");

    // 1. Wywołanie keygen
    let keygen_out = Command::new(signer_bin)
        .args(["keygen", priv_key_path.to_str().unwrap()])
        .output()
        .expect("Wywołanie keygen nie powiodło się");
    assert!(keygen_out.status.success(), "keygen zakończył się błędem");

    let stdout_keygen = String::from_utf8_lossy(&keygen_out.stdout);
    assert!(
        stdout_keygen.contains("Twój nowy kod zaufanego klucza publicznego"),
        "Wyjście keygen musi zawierać klucz publiczny"
    );

    // 2. Odczytanie wygenerowanego klucza prywatnego (efemerycznego testowego)
    let priv_bytes = fs::read(&priv_key_path).expect("Nie można odczytać wygenerowanego klucza");
    assert_eq!(priv_bytes.len(), 32);
    let priv_arr: [u8; 32] = priv_bytes.try_into().unwrap();
    let signing_key = SigningKey::from_bytes(&priv_arr);
    let public_bytes = signing_key.verifying_key().to_bytes();

    // 3. Utworzenie manifestu testowego
    let manifest_content = r#"{
        "manifest_version": 1,
        "key_id": "signer-test-ephemeral-key",
        "product": "SPLogbook",
        "version": "1.2.1",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "0000000000000000000000000000000000000000",
        "assets": [
            {
                "platform": "windows",
                "arch": "x86_64",
                "package_type": "zip",
                "filename": "SPLogbook-Windows-x64.zip",
                "size": 12345,
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            }
        ]
    }"#;
    fs::write(&manifest_path, manifest_content).expect("Błąd zapisu manifestu");
    let manifest_bytes = fs::read(&manifest_path).expect("Błąd odczytu zapisanego manifestu");

    // 4. Podpisanie manifestu narzędziem signer
    let sign_out = Command::new(signer_bin)
        .args([
            "sign",
            priv_key_path.to_str().unwrap(),
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .expect("Wywołanie sign nie powiodło się");
    assert!(sign_out.status.success(), "sign zakończył się błędem");

    // 5. Sprawdzenie pliku podpisu
    let sig_path = temp_dir.join("test_manifest.json.sig");
    assert!(sig_path.exists(), "Plik podpisu nie został utworzony");
    let sig_b64 = fs::read_to_string(&sig_path).expect("Błąd odczytu podpisu");

    // 6. Weryfikacja przez updater helper
    let trusted_key = TrustedKey {
        key_id: "signer-test-ephemeral-key",
        public_key: public_bytes,
        active: true,
        min_version: Some("1.1.0"),
        max_version: None,
    };
    let trusted_keys = [trusted_key];
    let empty_revoked_ids: [&str; 0] = [];
    let empty_revoked_pks: [[u8; 32]; 0] = [];

    let res = verify_manifest_signature_with_keys(
        &manifest_bytes,
        sig_b64.trim(),
        &trusted_keys,
        &empty_revoked_ids,
        &empty_revoked_pks,
    );
    assert!(
        res.is_ok(),
        "Manifest podpisany przez signer musi zostać poprawnie zweryfikowany przez updater"
    );

    // Czyszczenie
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn signer_rejects_invalid_private_key_length() {
    let signer_bin = env!("CARGO_BIN_EXE_splogbook-signer");
    let temp_dir =
        std::env::temp_dir().join(format!("splog_signer_invalid_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let short_key_path = temp_dir.join("short.key");
    fs::write(&short_key_path, [0u8; 16]).unwrap();

    let manifest_path = temp_dir.join("manifest.json");
    fs::write(&manifest_path, "{}").unwrap();

    let output = Command::new(signer_bin)
        .args([
            "sign",
            short_key_path.to_str().unwrap(),
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .expect("Wywołanie sign nie powiodło się");

    assert!(
        !output.status.success(),
        "Signer musi zwrócić błąd dla klucza o długości różnej od 32 bajtów"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn signer_never_prints_private_key_in_error() {
    let signer_bin = env!("CARGO_BIN_EXE_splogbook-signer");
    let temp_dir = std::env::temp_dir().join(format!("splog_signer_sec_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let bad_key_path = temp_dir.join("bad.key");
    // Unikalna sekwencja bajtów
    let unique_bytes = [0xFE, 0xED, 0xFA, 0xCE, 0x11, 0x22, 0x33, 0x44];
    fs::write(&bad_key_path, unique_bytes).unwrap();

    let manifest_path = temp_dir.join("manifest.json");
    fs::write(&manifest_path, "{}").unwrap();

    let output = Command::new(signer_bin)
        .args([
            "sign",
            bad_key_path.to_str().unwrap(),
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    assert!(
        !combined.contains("feedface"),
        "Signer nie może wypisywać zawartości niepoprawnego klucza"
    );
    assert!(
        !combined.contains("FE, ED, FA, CE"),
        "Signer nie może wypisywać tablicy bajtów klucza w błędzie"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn signer_fingerprint_matches_public_key() {
    let signer_bin = env!("CARGO_BIN_EXE_splogbook-signer");
    let temp_dir = std::env::temp_dir().join(format!("splog_signer_fp_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let priv_key_path = temp_dir.join("fp_test.key");

    let keygen_out = Command::new(signer_bin)
        .args(["keygen", priv_key_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(keygen_out.status.success());

    let fp_out = Command::new(signer_bin)
        .args(["fingerprint", priv_key_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(fp_out.status.success());

    let stdout_fp = String::from_utf8_lossy(&fp_out.stdout);

    // Obliczenie referencyjnego fingerprintu bezpośrednio
    let priv_bytes = fs::read(&priv_key_path).unwrap();
    let signing_key = SigningKey::from_bytes((&priv_bytes[..32]).try_into().unwrap());
    let public_bytes = signing_key.verifying_key().to_bytes();
    let expected_fp = sha256_hex(&public_bytes);

    assert!(
        stdout_fp.contains(&expected_fp),
        "Fingerprint wypisany przez signer musi odpowiadać SHA-256 klucza publicznego"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn release_workflow_references_secret_without_embedding_key_material() {
    let workflow_path = ".github/workflows/build-and-release.yml";
    let content = fs::read_to_string(workflow_path)
        .expect("Nie można odczytać .github/workflows/build-and-release.yml");

    // 1. Workflow musi odwoływać się do sekretu RELEASE_SIGNING_KEY
    assert!(
        content.contains("secrets.RELEASE_SIGNING_KEY"),
        "Workflow musi odwoływać się do secrets.RELEASE_SIGNING_KEY"
    );

    // 2. YAML nie może zawierać zahardkodowanego klucza prywatnego
    assert!(
        !content.contains("BEGIN PRIVATE KEY"),
        "Workflow nie może zawierać nagłówków klucza prywatnego"
    );
    assert!(
        !content.contains("BEGIN OPENSSH PRIVATE KEY"),
        "Workflow nie może zawierać kluczy OpenSSH"
    );

    // 3. Sprawdzenie, czy klucz tymczasowy nie jest ujawniany w artefaktach release
    let release_step_pos = content
        .find("Create GitHub Release")
        .expect("Brak kroku Create GitHub Release");
    let after_release_step = &content[release_step_pos..];
    assert!(
        !after_release_step.contains("priv.key"),
        "Plik priv.key nie może trafić do artefaktów GitHub Release"
    );

    // 4. Zakaz wypisywania sekretu w echo
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("echo ") && trimmed.contains("RELEASE_SIGNING_KEY") {
            assert!(
                trimmed.contains("base64 -d > priv.key"),
                "echo może być użyte wyłącznie do zdekodowania do pliku priv.key: {}",
                trimmed
            );
        }
    }
}

#[test]
fn repository_contains_no_tracked_private_key_material() {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .expect("Nie udało się wykonać polecenia git ls-files");

    assert!(
        output.status.success(),
        "git ls-files zakończyło się błędem"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tracked_files: Vec<&str> = stdout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let forbidden_exact = [
        "src/scratch.rs",
        "scratch.rs",
        ".signer_priv.key",
        "klucz.key",
        ".test.key",
    ];

    for file in &tracked_files {
        for forbidden in &forbidden_exact {
            assert_ne!(
                file, forbidden,
                "Zabroniony plik klucza/sekretu jest śledzony przez Git: {}",
                file
            );
        }

        assert!(
            !file.ends_with(".key"),
            "Plik z rozszerzeniem .key jest śledzony w Git: {}",
            file
        );
        assert!(
            !file.ends_with(".p12"),
            "Plik archiwum kluczy .p12 jest śledzony w Git: {}",
            file
        );
        assert!(
            !file.ends_with(".pfx"),
            "Plik archiwum kluczy .pfx jest śledzony w Git: {}",
            file
        );
    }

    for file in &tracked_files {
        if !file.ends_with(".rs") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(file) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    continue;
                }
                if (trimmed.contains("include_bytes!") || trimmed.contains("include_str!"))
                    && (trimmed.contains(".key")
                        || trimmed.contains(".pem")
                        || trimmed.contains(".p12")
                        || trimmed.contains(".pfx")
                        || trimmed.contains("scratch.rs"))
                {
                    panic!("Podejrzany wzorzec include_* w pliku {}: {}", file, trimmed);
                }

                if (trimmed.starts_with("pub const ")
                    || trimmed.starts_with("const ")
                    || trimmed.starts_with("static "))
                    && (trimmed.contains("PRIVATE_KEY")
                        || trimmed.contains("SECRET_SEED")
                        || trimmed.contains("SIGNING_KEY"))
                {
                    panic!(
                        "Zabroniona stała klucza prywatnego w pliku {}: {}",
                        file, trimmed
                    );
                }
            }
        }
    }
}
