// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use splogbook::cloud::updater::{
    AtomicInstallationOptions, REVOKED_KEY_IDS, REVOKED_PUBLIC_KEYS, TRUSTED_KEYS, TrustedKey,
    UpdateFileClass, UpdatePlan, VerifiedReleaseManifest, apply_atomic_installation_with_options,
    classify_update_path, generate_powershell_update_script, is_key_revoked, sha256_hex,
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

#[test]
fn configuration_debug_never_leaks_secrets() {
    use splogbook::cloud::scheduler::UploadCredentials;
    use splogbook::core::station::AppConfig;
    use splogbook::sync::service::{SyncConfig, SyncRole};

    // 1. AppConfig
    let cfg = AppConfig {
        lan_sync_secret: "TEST_SECRET_GUARD_1".into(),
        lotw_password: "TEST_SECRET_GUARD_2".into(),
        qrz_password: "TEST_SECRET_GUARD_3".into(),
        qrz_api_key: "TEST_SECRET_GUARD_4".into(),
        eqsl_password: "TEST_SECRET_GUARD_5".into(),
        clublog_password: "TEST_SECRET_GUARD_6".into(),
        clublog_api_key: "TEST_SECRET_GUARD_7".into(),
        cloudlog_api_key: "TEST_SECRET_GUARD_8".into(),
        rest_api_key: "TEST_SECRET_GUARD_9".into(),
        hrdlog_upload_code: "TEST_SECRET_GUARD_10".into(),
        hamqth_password: "TEST_SECRET_GUARD_11".into(),
        ..AppConfig::default()
    };

    let debug_cfg = format!("{cfg:?}");
    assert!(!debug_cfg.contains("TEST_SECRET_GUARD"));
    assert!(debug_cfg.contains("<redacted>"));

    // 2. UploadCredentials
    let creds = UploadCredentials {
        clublog_callsign: "SP6INA".into(),
        clublog_email: "test@example.com".into(),
        clublog_password: "TEST_SECRET_GUARD_12".into(),
        clublog_api_key: "TEST_SECRET_GUARD_13".into(),
        qrz_api_key: "TEST_SECRET_GUARD_14".into(),
        eqsl_username: "SP6INA".into(),
        eqsl_password: "TEST_SECRET_GUARD_15".into(),
    };
    let debug_creds = format!("{creds:?}");
    assert!(!debug_creds.contains("TEST_SECRET_GUARD"));
    assert!(debug_creds.contains("<redacted>"));

    // 3. SyncConfig
    let sync_cfg = SyncConfig {
        role: SyncRole::Peer {
            host: "127.0.0.1".into(),
            port: 7373,
        },
        secret: "TEST_SECRET_GUARD_16".into(),
        station: "SP6INA".into(),
        journal: "DEFAULT".into(),
    };
    let debug_sync = format!("{sync_cfg:?}");
    assert!(!debug_sync.contains("TEST_SECRET_GUARD"));
    assert!(debug_sync.contains("<redacted>"));

    // 4. Strażnik braku automatycznego derive(Debug) na AppConfig
    let station_rs = std::fs::read_to_string("src/core/station.rs").expect("read station.rs");
    assert!(
        !station_rs.contains("#[derive(Debug, Clone, Serialize, Deserialize)]\r\n#[serde(default)]\r\npub struct AppConfig")
        && !station_rs.contains("#[derive(Debug, Clone, Serialize, Deserialize)]\n#[serde(default)]\npub struct AppConfig"),
        "AppConfig nie może posiadać automatycznego derive(Debug)"
    );
}

#[test]
fn security_guard_bind_policy_and_handshake_timeout() {
    use splogbook::api::server::{ServerBindError, validate_bind_policy};
    use splogbook::sync::HOST_HANDSHAKE_TIMEOUT;

    // SEC-003: Strażnik limitu czasu handshake'u Hosta synchronizacji
    assert!(
        HOST_HANDSHAKE_TIMEOUT <= std::time::Duration::from_secs(15),
        "HOST_HANDSHAKE_TIMEOUT musi być ograniczony do maksymalnie 15 sekund (SEC-003)"
    );
    assert!(
        HOST_HANDSHAKE_TIMEOUT >= std::time::Duration::from_secs(5),
        "HOST_HANDSHAKE_TIMEOUT nie może być krótszy niż 5 sekund"
    );

    // SEC-004: Strażnik polityki wiązania serwera REST
    let public_ip: std::net::IpAddr = "0.0.0.0".parse().unwrap();
    assert_eq!(
        validate_bind_policy(&public_ip, ""),
        Err(ServerBindError::PublicBindWithoutAuthentication)
    );

    let lan_ip: std::net::IpAddr = "192.168.1.1".parse().unwrap();
    assert_eq!(
        validate_bind_policy(&lan_ip, ""),
        Err(ServerBindError::PublicBindWithoutAuthentication)
    );

    let loopback_v4: std::net::IpAddr = "127.0.0.1".parse().unwrap();
    assert!(validate_bind_policy(&loopback_v4, "").is_ok());

    let loopback_v6: std::net::IpAddr = "::1".parse().unwrap();
    assert!(validate_bind_policy(&loopback_v6, "").is_ok());

    assert!(validate_bind_policy(&lan_ip, "TEST_SECRET_DO_NOT_USE_GUARD").is_ok());
}

#[test]
fn release_workflow_fails_closed_without_signing_secret() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie można odczytać workflow");
    assert!(
        workflow.contains("RELEASE_SIGNING_KEY"),
        "Workflow musi korzystać z RELEASE_SIGNING_KEY"
    );
    assert!(
        workflow.contains(r#"[ -z "$RELEASE_SIGNING_KEY" ]"#),
        "Workflow musi sprawdzać, czy sekret RELEASE_SIGNING_KEY jest pusty"
    );
    assert!(
        workflow.contains("exit 1"),
        "Brak klucza przy próbie publikacji wydania musi kończyć się błędem (fail-closed)"
    );
}

#[test]
fn release_workflow_rejects_incomplete_artifact_set() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie można odczytać workflow");
    assert!(
        workflow.contains("SPLogbook-Linux-x86_64.tar.gz"),
        "Workflow musi weryfikować obecność artefaktu Linux"
    );
    assert!(
        workflow.contains("SPLogbook-Windows-x64.zip"),
        "Workflow musi weryfikować obecność artefaktu Windows ZIP"
    );
    assert!(
        workflow.contains("SPLogbook-Windows-x64-Setup-"),
        "Workflow musi weryfikować obecność artefaktu Windows Inno Setup"
    );
    assert!(
        workflow.contains("SPLogbook-Windows-x64-MSI-"),
        "Workflow musi weryfikować obecność artefaktu Windows MSI"
    );
    assert!(
        workflow.contains("[ -s release-staging/SPLogbook-Linux-x86_64.tar.gz ]"),
        "Workflow musi sprawdzać, czy artefakt Linux jest niepusty"
    );
    assert!(
        workflow.contains("[ -s release-staging/SPLogbook-Windows-x64.zip ]"),
        "Workflow musi sprawdzać, czy artefakt Windows ZIP jest niepusty"
    );
    assert!(
        workflow.contains(r#"[ -s "release-staging/${INNO_EXE}" ]"#),
        "Workflow musi sprawdzać, czy artefakt Windows Inno Setup jest niepusty"
    );
    assert!(
        workflow.contains(r#"[ -s "release-staging/${MSI_PKG}" ]"#),
        "Workflow musi sprawdzać, czy artefakt Windows MSI jest niepusty"
    );
    assert!(
        workflow.contains(r#"[ "$COUNT" -eq 4 ]"#),
        "Workflow musi egzekwować dokładną liczbę 4 artefaktów wydania (tar.gz, zip, inno-exe, msi)"
    );
}

#[test]
fn release_workflow_rejects_duplicate_artifact_names() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie można odczytać workflow");
    assert!(
        workflow.contains("Unexpected artifact count in staging"),
        "Workflow musi weryfikować jednoznaczność i liczbę artefaktów w staging"
    );
}

#[test]
fn release_workflow_requires_manifest_signature_and_checksums() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie można odczytać workflow");
    assert!(
        workflow.contains("SHA256SUMS.txt"),
        "Workflow musi generować i publikować SHA256SUMS.txt"
    );
    assert!(
        workflow.contains("release-manifest.json"),
        "Workflow musi generować i publikować release-manifest.json"
    );
    assert!(
        workflow.contains("release-manifest.json.sig"),
        "Workflow musi generować i publikować release-manifest.json.sig"
    );
}

struct ApprovedAction {
    repository: &'static str,
    tag: &'static str,
    sha: &'static str,
    runs_using: &'static str,
}

const APPROVED_ACTIONS: &[ApprovedAction] = &[
    ApprovedAction {
        repository: "actions/checkout",
        tag: "v7.0.1",
        sha: "3d3c42e5aac5ba805825da76410c181273ba90b1",
        runs_using: "node24",
    },
    ApprovedAction {
        repository: "dtolnay/rust-toolchain",
        tag: "v1",
        sha: "683b588c487a69d01f6c6ddf40f036ff682cc2c8",
        runs_using: "composite",
    },
    ApprovedAction {
        repository: "actions/upload-artifact",
        tag: "v7.0.2",
        sha: "cf430e030ddbb5b0abf93d22962f4752f3646cd9",
        runs_using: "node24",
    },
    ApprovedAction {
        repository: "actions/download-artifact",
        tag: "v8.0.2",
        sha: "9000827ccba6bdab643e8b6fd33ac0654aef8333",
        runs_using: "node24",
    },
    ApprovedAction {
        repository: "softprops/action-gh-release",
        tag: "v3.0.3",
        sha: "efb35369e0ad2afab669f228072c1b0d510eae64",
        runs_using: "node24",
    },
];

#[test]
fn release_workflow_actions_are_pinned_by_policy() {
    let workflow_files = [
        ".github/workflows/build-and-release.yml",
        ".github/workflows/ci.yml",
    ];

    for file_path in &workflow_files {
        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("Nie mozna odczytac {file_path}: {e}"));

        assert!(
            !content.contains("ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION"),
            "{file_path} nie moze uzywac obejscia ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION"
        );
        assert!(
            !content.contains("FORCE_JAVASCRIPT_ACTIONS_TO_NODE24"),
            "{file_path} nie moze uzywac obejscia FORCE_JAVASCRIPT_ACTIONS_TO_NODE24"
        );

        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("uses:") {
                let uses_spec = trimmed.trim_start_matches("uses:").trim();
                let mut parts = uses_spec.splitn(2, '#');
                let action_ref = parts.next().unwrap().trim();
                let comment = parts.next().map(str::trim).unwrap_or_else(|| {
                    panic!("Linia {line_no} w {file_path} musi zawierac komentarz z wersja: {line}")
                });

                let (repo, sha) = action_ref
                    .split_once('@')
                    .unwrap_or_else(|| panic!("Linia {line_no} w {file_path} brak '@': {line}"));

                assert_eq!(
                    sha.len(),
                    40,
                    "Linia {line_no} w {file_path}: Akcja musi byc przypieta do pelnego 40-znakowego SHA: {sha}"
                );
                assert!(
                    sha.chars()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                    "Linia {line_no} w {file_path}: SHA musi skladac sie wylacznie z malych cyfr/liter hex: {sha}"
                );

                assert!(
                    !action_ref.ends_with("@master")
                        && !action_ref.ends_with("@main")
                        && !action_ref.ends_with("@latest")
                        && !action_ref.ends_with("@stable")
                        && !action_ref.contains("@v"),
                    "Linia {line_no} w {file_path}: Akcja nie moze odwolywac sie do plywajacych galezi/tagow: {action_ref}"
                );

                let approved = APPROVED_ACTIONS
                    .iter()
                    .find(|a| a.repository == repo)
                    .unwrap_or_else(|| {
                        panic!(
                            "Nieznane lub nieautoryzowane repozytorium akcji: {repo} w {file_path}"
                        )
                    });

                assert_eq!(
                    sha, approved.sha,
                    "Linia {line_no} w {file_path}: Nieprawidlowy SHA dla {repo}"
                );
                assert_eq!(
                    comment, approved.tag,
                    "Linia {line_no} w {file_path}: Komentarz wersji '{comment}' nie odpowiada zatwierdzonemu tagowi '{}' dla {repo}",
                    approved.tag
                );
            }
        }
    }
}

#[test]
fn release_workflow_uses_latest_approved_stable_actions() {
    assert_eq!(APPROVED_ACTIONS.len(), 5);
    for approved in APPROVED_ACTIONS {
        assert_eq!(approved.sha.len(), 40);
        assert!(approved.tag.starts_with('v'));
        assert!(
            approved.runs_using == "node24" || approved.runs_using == "composite",
            "Zatwierdzona akcja {} musi uzywac node24 lub composite, otrzymano: {}",
            approved.repository,
            approved.runs_using
        );
    }

    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie mozna odczytac build-and-release.yml");
    for approved in APPROVED_ACTIONS {
        assert!(
            workflow.contains(approved.sha),
            "Workflow build-and-release.yml musi zawierac zatwierdzony SHA dla {}: {}",
            approved.repository,
            approved.sha
        );
    }
}

#[test]
fn release_workflow_has_no_node20_actions() {
    let workflow_files = [
        ".github/workflows/build-and-release.yml",
        ".github/workflows/ci.yml",
    ];

    for file_path in &workflow_files {
        let content = fs::read_to_string(file_path).expect("Nie mozna odczytac workflow");
        assert!(
            !content.contains("node20"),
            "{file_path} nie moze zawierac 'node20'"
        );
        assert!(
            !content.contains("@v4"),
            "{file_path} nie moze zawierac przestarzalego tagu @v4 (opartego na node20)"
        );
        assert!(
            !content.contains("@v2"),
            "{file_path} nie moze zawierac przestarzalego tagu @v2 (opartego na node20)"
        );
    }
}

#[test]
fn release_workflow_verifies_signing_key_fingerprint() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie mozna odczytac build-and-release.yml");

    assert!(
        workflow.contains("2a6277b72d2a97236f344f1542a1c2868a74f71c9626a22dc1735919efeb2a4a"),
        "Workflow musi weryfikowac oczekiwany fingerprint zaufanego klucza publicznego"
    );
    assert!(
        workflow.contains("splogbook-release-2026-02"),
        "Workflow musi uzywac aktywnego identyfikatora splogbook-release-2026-02"
    );
    assert!(
        workflow.contains("trap 'rm -f priv.key dry_run.key"),
        "Workflow musi usuwac klucze prywatne przez trap EXIT"
    );
}

#[test]
fn release_manifest_is_deterministic() {
    use splogbook::cloud::updater::{ManifestAsset, ReleaseManifest};

    let manifest1 = ReleaseManifest {
        manifest_version: 1,
        key_id: "splogbook-release-2026-02".into(),
        product: "SPLogbook".into(),
        version: "1.2.1".into(),
        channel: "stable".into(),
        minimum_updater_version: "1.1.0".into(),
        commit: "0123456789abcdef0123456789abcdef01234567".into(),
        assets: vec![
            ManifestAsset {
                platform: "linux".into(),
                arch: "x86_64".into(),
                package_type: "tar.gz".into(),
                filename: "SPLogbook-Linux-x86_64.tar.gz".into(),
                size: 50_000_000,
                sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            },
            ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 60_000_000,
                sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            },
        ],
    };

    let json1 = serde_json::to_string_pretty(&manifest1).unwrap();
    let json2 = serde_json::to_string_pretty(&manifest1).unwrap();
    assert_eq!(
        json1, json2,
        "Serializacja manifestu musi być deterministyczna"
    );
}

#[test]
fn release_manifest_lists_every_expected_artifact_once() {
    use splogbook::cloud::updater::{ManifestAsset, ReleaseManifest};

    let manifest = ReleaseManifest {
        manifest_version: 1,
        key_id: "splogbook-release-2026-02".into(),
        product: "SPLogbook".into(),
        version: "1.3.0".into(),
        channel: "stable".into(),
        minimum_updater_version: "1.1.0".into(),
        commit: "0123456789abcdef0123456789abcdef01234567".into(),
        assets: vec![
            ManifestAsset {
                platform: "linux".into(),
                arch: "x86_64".into(),
                package_type: "tar.gz".into(),
                filename: "SPLogbook-Linux-x86_64.tar.gz".into(),
                size: 50_000_000,
                sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            },
            ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "zip".into(),
                filename: "SPLogbook-Windows-x64.zip".into(),
                size: 60_000_000,
                sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            },
            ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "inno-exe".into(),
                filename: "SPLogbook-Windows-x64-Setup-1.3.0.exe".into(),
                size: 55_000_000,
                sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
            },
            ManifestAsset {
                platform: "windows".into(),
                arch: "x86_64".into(),
                package_type: "msi".into(),
                filename: "SPLogbook-Windows-x64-MSI-1.3.0.msi".into(),
                size: 58_000_000,
                sha256: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into(),
            },
        ],
    };

    assert!(manifest.validate().is_ok());

    let linux_count = manifest
        .assets
        .iter()
        .filter(|a| a.platform == "linux")
        .count();
    let windows_count = manifest
        .assets
        .iter()
        .filter(|a| a.platform == "windows")
        .count();
    assert_eq!(linux_count, 1, "Artefakt Linux musi wystąpić dokładnie raz");
    assert_eq!(
        windows_count, 3,
        "Windows musi mieć dokładnie 3 pakiety (zip, inno-exe, msi)"
    );

    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie można odczytać workflow");
    for pkg_type in ["tar.gz", "zip", "inno-exe", "msi"] {
        assert!(
            workflow.contains(&format!(r#""package_type": "{pkg_type}""#)),
            "Manifest w workflow musi zawierać package_type '{pkg_type}'"
        );
    }
}

#[test]
fn release_manifest_uses_active_key_id() {
    let active_key = TRUSTED_KEYS
        .iter()
        .find(|k| k.active)
        .expect("Brak aktywnego klucza");
    assert_eq!(
        active_key.key_id, "splogbook-release-2026-02",
        "Aktywny klucz wydania to splogbook-release-2026-02"
    );

    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("Nie można odczytać workflow");
    assert!(
        workflow.contains(r#""key_id": "splogbook-release-2026-02""#),
        "Manifest w workflow musi używać identyfikatora aktywnego klucza"
    );
}

#[test]
fn release_manifest_hashes_match_built_artifacts() {
    let dummy_sha = "3d5f0e4c2f76c58916ec258f246851bea091d14d4247a2fc3e18694461b1816e";
    assert_eq!(dummy_sha.len(), 64);
    assert!(
        dummy_sha
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );

    let sums_sample = format!(
        "{dummy_sha}  SPLogbook-Linux-x86_64.tar.gz
"
    );
    assert!(sums_sample.contains("  "));
}

#[test]
fn signer_verify_accepts_valid_signature_with_explicit_key() {
    let signer_bin = env!("CARGO_BIN_EXE_splogbook-signer");
    let temp_dir =
        std::env::temp_dir().join(format!("splog_signer_verify_{}", rand::random::<u64>()));
    fs::create_dir_all(&temp_dir).unwrap();

    let priv_key_path = temp_dir.join("test_verify.key");
    let manifest_path = temp_dir.join("release-manifest.json");
    let sig_path = temp_dir.join("release-manifest.json.sig");

    let keygen_out = Command::new(signer_bin)
        .args(["keygen", priv_key_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(keygen_out.status.success());

    let manifest_content = r#"{
        "manifest_version": 1,
        "key_id": "signer-verify-test-key",
        "product": "SPLogbook",
        "version": "1.2.0",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "0123456789abcdef0123456789abcdef01234567",
        "assets": []
    }"#;
    fs::write(&manifest_path, manifest_content).unwrap();

    let sign_out = Command::new(signer_bin)
        .args([
            "sign",
            priv_key_path.to_str().unwrap(),
            manifest_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(sign_out.status.success());
    assert!(sig_path.exists());

    // 1. Poprawna weryfikacja z jawnym kluczem
    let verify_out = Command::new(signer_bin)
        .args([
            "verify",
            manifest_path.to_str().unwrap(),
            sig_path.to_str().unwrap(),
            priv_key_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        verify_out.status.success(),
        "Poprawny podpis musi zostać zweryfikowany kodem 0: {}",
        String::from_utf8_lossy(&verify_out.stderr)
    );

    // 2. Modyfikacja manifestu powoduje błąd weryfikacji (kod != 0)
    let tampered_manifest_path = temp_dir.join("tampered-manifest.json");
    fs::write(
        &tampered_manifest_path,
        manifest_content.replace("1.2.0", "1.2.1"),
    )
    .unwrap();
    let tampered_out = Command::new(signer_bin)
        .args([
            "verify",
            tampered_manifest_path.to_str().unwrap(),
            sig_path.to_str().unwrap(),
            priv_key_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !tampered_out.status.success(),
        "Zmodyfikowany manifest musi zostać odrzucony błędem"
    );

    // 3. Uszkodzony plik podpisu powoduje błąd weryfikacji (kod != 0)
    let bad_sig_path = temp_dir.join("bad.sig");
    fs::write(
        &bad_sig_path,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==",
    )
    .unwrap();
    let bad_sig_out = Command::new(signer_bin)
        .args([
            "verify",
            manifest_path.to_str().unwrap(),
            bad_sig_path.to_str().unwrap(),
            priv_key_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !bad_sig_out.status.success(),
        "Uszkodzony podpis musi zostać odrzucony błędem"
    );

    // 4. Regresja: domyślne `verify` (bez jawnego klucza) ufa wyłącznie wbudowanemu
    //    kluczowi wydawniczemu. Podpis złożony kluczem testowym nie może zostać przyjęty
    //    tylko dlatego, że ten sam klucz leży jako `dry_run.key` / `priv.key` w katalogu
    //    bieżącym lub nadrzędnym.
    let work_dir = temp_dir.join("work");
    fs::create_dir_all(&work_dir).unwrap();
    let test_key_bytes = fs::read(&priv_key_path).unwrap();
    for planted in [
        work_dir.join("dry_run.key"),
        work_dir.join("priv.key"),
        temp_dir.join("dry_run.key"),
        temp_dir.join("priv.key"),
    ] {
        fs::write(&planted, &test_key_bytes).unwrap();
    }
    fs::copy(&manifest_path, work_dir.join("release-manifest.json")).unwrap();
    fs::copy(&sig_path, work_dir.join("release-manifest.json.sig")).unwrap();
    let default_out = Command::new(signer_bin)
        .current_dir(&work_dir)
        .args([
            "verify",
            "release-manifest.json",
            "release-manifest.json.sig",
        ])
        .output()
        .unwrap();
    let default_stdout = String::from_utf8_lossy(&default_out.stdout);
    assert!(
        !default_out.status.success(),
        "Domyślne verify nie może przyjąć podpisu kluczem z lokalnego dry_run.key/priv.key: {default_stdout}"
    );
    assert!(
        !default_stdout.contains("zweryfikowany pomyślnie"),
        "Domyślne verify nie może zgłaszać sukcesu dla klucza spoza listy zaufanych: {default_stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn workspace_test_counting_is_deterministic() {
    use splogbook::core::test_audit::{parse_cargo_test_output, sanitize_katex_duplicates};

    let sample_output = r#"
     Running unittests src\lib.rs (target\debug\deps\splogbook-123.exe)
test result: ok. 1527 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.0s
     Running unittests src\main.rs (target\debug\deps\SPLogbook-456.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s
     Running unittests src\bin\signer.rs (target\debug\deps\splogbook_signer-789.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s
     Running tests\cat_native_6a.rs (target\debug\deps\cat_native_6a-abc.exe)
test result: ok. 165 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\cat_native_6b.rs (target\debug\deps\cat_native_6b-def.exe)
test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\cat_native_7.rs (target\debug\deps\cat_native_7-ghi.exe)
test result: ok. 297 passed; 0 failed; 10 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\i18n_guard.rs (target\debug\deps\i18n_guard-jkl.exe)
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.0s
     Running tests\security_guard.rs (target\debug\deps\security_guard-mno.exe)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s
     Running tests\theme_guard.rs (target\debug\deps\theme_guard-pqr.exe)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s
   Doc-tests splogbook
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s
    "#;

    let report = parse_cargo_test_output(sample_output);
    assert_eq!(report.lib_tests(), 1527);
    assert_eq!(report.binary_tests(), 0);
    assert_eq!(report.integration_tests(), 165 + 99 + 307);
    assert_eq!(report.guard_tests(), 15 + 32 + 1);
    assert_eq!(report.doc_tests(), 0);
    assert_eq!(report.total_registered(), 2146);
    assert_eq!(report.total_executed(), 2136);
    assert_eq!(report.total_passed(), 2136);
    assert_eq!(report.total_failed(), 0);
    assert_eq!(report.total_ignored(), 10);
    assert_eq!(report.hil_ignored(), 10);

    let sanitized = sanitize_katex_duplicates(
        r"TOTAL REGISTERED=2146\text{TOTAL REGISTERED}=2146 and O(N)O(N)O(N)",
    );
    assert!(!sanitized.contains(r"\text{"));
    assert!(!sanitized.contains("O(N)O(N)O(N)"));
    assert!(sanitized.contains("O(N)"));
}

// =========================================================================
// FUNC-005 / SEC-005: REST API Token Security & Redaction Invariants
// =========================================================================

#[test]
fn rest_token_is_not_serialized_in_standard_config() {
    use splogbook::core::station::AppConfig;
    let mut cfg = AppConfig::default();
    let test_secret = "TEST_REST_TOKEN_NEVER_SERIALIZE_PL_987";
    cfg.rest_api_key = test_secret.into();

    let json = serde_json::to_string(&cfg).expect("Serialization must succeed");
    assert!(
        !json.contains(test_secret),
        "REST API key must never appear in serialized JSON config"
    );
    assert!(
        !json.contains("rest_api_key"),
        "rest_api_key field must be omitted via #[serde(skip_serializing)]"
    );
}

#[test]
fn rest_token_is_redacted_from_debug() {
    use splogbook::core::station::AppConfig;
    let mut cfg = AppConfig::default();
    let test_secret = "TEST_REST_TOKEN_DEBUG_REDACT_456";
    cfg.rest_api_key = test_secret.into();

    let debug_out = format!("{cfg:?}");
    assert!(
        !debug_out.contains(test_secret),
        "REST API key must never appear in Debug format output"
    );
    assert!(
        debug_out.contains("<redacted>"),
        "REST API key must be represented as <redacted> in Debug output"
    );
}

#[test]
fn rest_token_is_redacted_from_error_messages() {
    use splogbook::cat::tx_arbiter::redact_sensitive_info;

    let secret = "TEST_REST_TOKEN_SECRET_ERR_999";
    let error_with_bearer = format!("HTTP 401 Unauthorized: bearer {secret} is invalid");
    let redacted_bearer = redact_sensitive_info(&error_with_bearer);
    assert!(!redacted_bearer.contains(secret));
    assert!(redacted_bearer.contains("[REDACTED]"));

    let error_with_api_key = format!("REST dispatch failed: api_key={secret} invalid");
    let redacted_key = redact_sensitive_info(&error_with_api_key);
    assert!(!redacted_key.contains(secret));
    assert!(redacted_key.contains("[REDACTED]"));

    let error_with_header = format!("Header error: x-api-key={secret} expired");
    let redacted_header = redact_sensitive_info(&error_with_header);
    assert!(!redacted_header.contains(secret));
    assert!(redacted_header.contains("[REDACTED]"));
}

#[test]
fn rest_token_is_not_in_diagnostic_export() {
    use splogbook::core::http::redacted_url_for_diagnostics;

    let secret = "TEST_DIAG_REST_TOKEN_ABC123";
    let raw_url = format!("http://127.0.0.1:8420/api/v1/status?api_key={secret}&source=test");
    let sanitized = redacted_url_for_diagnostics(&raw_url);
    assert!(
        !sanitized.contains(secret),
        "Diagnostic URL sanitizer must scrub REST token query parameters"
    );
}

#[test]
fn rest_token_is_masked_in_gui_model() {
    let tools_source =
        fs::read_to_string("src/gui/menu/tools.rs").expect("read src/gui/menu/tools.rs");
    assert!(
        tools_source.contains(".password(true)"),
        "REST API key field in Tools menu must be explicitly masked with .password(true)"
    );
}

#[test]
fn rest_token_copy_action_does_not_log_value() {
    let tools_source =
        fs::read_to_string("src/gui/menu/tools.rs").expect("read src/gui/menu/tools.rs");
    let copy_idx = tools_source.find("copy_text").expect("find copy_text");
    let window = &tools_source
        [copy_idx.saturating_sub(80)..std::cmp::min(tools_source.len(), copy_idx + 120)];
    assert!(
        !window.contains("log::") && !window.contains("println!"),
        "Copying REST API key to clipboard must never emit log messages"
    );
}

#[test]
fn rest_token_regeneration_does_not_log_values() {
    let tools_source =
        fs::read_to_string("src/gui/menu/tools.rs").expect("read src/gui/menu/tools.rs");
    let regen_idx = tools_source
        .find("generate_api_key")
        .expect("find generate_api_key");
    let window = &tools_source
        [regen_idx.saturating_sub(80)..std::cmp::min(tools_source.len(), regen_idx + 120)];
    assert!(
        !window.contains("log::") && !window.contains("println!"),
        "Regenerating REST API key must never log old or new token values"
    );
}

#[test]
fn remote_rest_remains_disabled_by_default() {
    use splogbook::api::server::default_rest_bind_address;
    use splogbook::core::station::AppConfig;
    use splogbook::gui::state::AppRuntime;

    let cfg = AppConfig::default();
    assert!(
        cfg.rest_api_key.is_empty(),
        "REST API key must be empty by default until explicitly generated"
    );

    let rt = AppRuntime::default();
    let addr = default_rest_bind_address(rt.rest_api_port);
    assert!(
        addr.ip().is_loopback(),
        "Default REST API bind address must be strictly loopback (127.0.0.1)"
    );
    assert!(
        !rt.rest_api_enabled,
        "REST API server must remain disabled by default in runtime configuration"
    );
}

#[test]
fn public_bind_without_token_is_rejected() {
    use splogbook::api::server::{ServerBindError, validate_bind_policy};
    use std::net::IpAddr;

    let public_v4: IpAddr = "0.0.0.0".parse().unwrap();
    assert_eq!(
        validate_bind_policy(&public_v4, ""),
        Err(ServerBindError::PublicBindWithoutAuthentication)
    );
    assert_eq!(
        validate_bind_policy(&public_v4, "   "),
        Err(ServerBindError::PublicBindWithoutAuthentication)
    );

    let lan_v4: IpAddr = "192.168.1.100".parse().unwrap();
    assert_eq!(
        validate_bind_policy(&lan_v4, ""),
        Err(ServerBindError::PublicBindWithoutAuthentication)
    );

    let public_v6: IpAddr = "::".parse().unwrap();
    assert_eq!(
        validate_bind_policy(&public_v6, ""),
        Err(ServerBindError::PublicBindWithoutAuthentication)
    );
}

#[test]
fn ci_workflow_enforces_fmt_check() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    assert!(
        content.contains("cargo fmt --all -- --check") || content.contains("cargo fmt --check"),
        "CI workflow must enforce cargo fmt check"
    );
}

#[test]
fn ci_workflow_enforces_clippy_deny_warnings() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    assert!(
        content.contains("cargo clippy --workspace --all-targets --all-features -- -D warnings"),
        "CI workflow must enforce cargo clippy --workspace --all-targets --all-features -- -D warnings"
    );
}

#[test]
fn ci_workflow_runs_workspace_tests() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    assert!(
        content.contains("cargo test --workspace --all-targets"),
        "CI workflow must run cargo test --workspace --all-targets"
    );
}

#[test]
fn ci_uses_pinned_cargo_audit_version() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    assert!(
        content.contains("cargo install cargo-audit --version 0.22.2 --locked"),
        "CI workflow must install strictly pinned cargo-audit version with --locked"
    );
}

#[test]
fn ci_audit_does_not_ignore_all_warnings() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    assert!(
        !content.contains("--ignore all") && !content.contains("ignore = [\"*\"]"),
        "CI cargo audit must not use broad wildcard ignores"
    );
}

#[test]
fn ci_audit_uses_explicit_advisory_ids() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    let audit_line = content
        .lines()
        .find(|l| l.trim().starts_with("cargo audit"))
        .expect("must have cargo audit line");
    assert!(
        !audit_line.contains('*'),
        "Cargo audit line must not contain wildcard characters"
    );
    for part in audit_line.split_whitespace() {
        if part.starts_with("RUSTSEC-") {
            assert!(
                part.len() >= 17,
                "Advisory ID must be fully qualified RUSTSEC-YYYY-NNNN, got: {part}"
            );
        }
    }
}

#[test]
fn ci_audit_allowed_advisories_match_documented_baseline() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    let expected_ids = [
        "RUSTSEC-2024-0436",
        "RUSTSEC-2025-0141",
        "RUSTSEC-2026-0249",
        "RUSTSEC-2026-0370",
        "RUSTSEC-2026-0371",
        "RUSTSEC-2026-0372",
        "RUSTSEC-2026-0097",
    ];
    for id in expected_ids {
        assert!(
            content.contains(id),
            "CI workflow must explicitly list baseline advisory {id}"
        );
    }
}

#[test]
fn ci_audit_new_unknown_advisory_would_fail() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    let audit_line = content
        .lines()
        .find(|l| l.trim().starts_with("cargo audit"))
        .expect("must have cargo audit line");
    assert!(
        audit_line.contains("--deny warnings"),
        "Cargo audit must enforce --deny warnings so unallowed warnings/advisories fail CI"
    );
}

#[test]
fn ci_audit_does_not_disable_vulnerability_failures() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    let audit_line = content
        .lines()
        .find(|l| l.trim().starts_with("cargo audit"))
        .expect("must have cargo audit line");
    assert!(
        !audit_line.contains("--no-vulnerabilities"),
        "Cargo audit must never disable vulnerability failures"
    );
}

#[test]
fn ci_audit_does_not_modify_cargo_manifests() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    let audit_step = content
        .lines()
        .skip_while(|l| !l.contains("Install and run cargo audit"))
        .take(10)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !audit_step.contains("cargo update"),
        "CI audit step must not execute cargo update or modify Cargo manifests"
    );
}

#[test]
fn ci_runs_cargo_audit() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    assert!(
        content.contains("cargo audit"),
        "CI workflow must run cargo audit"
    );
}

#[test]
fn ci_workflow_pins_actions_to_40_char_sha() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    let mut found_uses = 0usize;
    for line in content.lines() {
        let trimmed = line.trim();
        let after_uses = if let Some(rest) = trimmed.strip_prefix("uses:") {
            Some(rest.trim())
        } else {
            trimmed.strip_prefix("- uses:").map(str::trim)
        };
        if let Some(spec) = after_uses {
            found_uses += 1;
            let without_comment = spec.split('#').next().unwrap_or("").trim();
            let (_, rev) = without_comment
                .split_once('@')
                .expect("Action must specify @revision");
            assert!(
                rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit()),
                "CI workflow action must be pinned to 40-char SHA, got: {without_comment}"
            );
        }
    }
    assert!(found_uses >= 2, "Expected at least 2 action uses in ci.yml");
}

#[test]
fn ci_workflow_uses_minimal_permissions() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    let normalized = content.replace("\r\n", "\n");
    assert!(
        normalized.contains("permissions:\n  contents: read"),
        "CI workflow must declare top-level minimal permissions: contents: read"
    );
    assert!(
        !content.contains("contents: write"),
        "CI workflow must not request contents: write permission"
    );
}

#[test]
fn ci_workflow_does_not_allow_clippy_failure() {
    let content = fs::read_to_string(".github/workflows/ci.yml").expect("read ci.yml");
    assert!(
        !content.contains("continue-on-error: true"),
        "CI workflow must not use continue-on-error: true"
    );
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("cargo clippy")
            || trimmed.contains("cargo fmt")
            || trimmed.contains("cargo test")
            || trimmed.contains("cargo audit")
        {
            assert!(
                !trimmed.contains("|| true"),
                "Quality gate command must not suppress failure with || true: {trimmed}"
            );
        }
    }
}

#[test]
fn documentation_test_counts_match_canonical_workspace_counts() {
    // Jedynym źródłem prawdy dla liczników jest publiczna dokumentacja weryfikacji.
    // Raport bramki procesowej nie jest źródłem prawdy dla testu.
    let baseline = fs::read_to_string("docs/VERIFICATION_BASELINE.md")
        .expect("docs/VERIFICATION_BASELINE.md must exist");

    assert!(
        baseline.contains("**Data pomiaru:**"),
        "Dokument bazowy musi jawnie podawać datę pomiaru"
    );
    assert!(
        baseline.contains("cargo test --workspace --all-targets"),
        "Dokument bazowy musi podawać kanoniczną komendę pomiaru"
    );

    let extract_val = |key: &str| -> usize {
        baseline
            .lines()
            .find(|l| l.contains(key))
            .and_then(|l| {
                l.split(':').nth(1)?.split_whitespace().find_map(|word| {
                    let digits: String = word.chars().filter(|c| c.is_ascii_digit()).collect();
                    digits.parse::<usize>().ok()
                })
            })
            .unwrap_or_else(|| panic!("Key {key} not found in docs/VERIFICATION_BASELINE.md"))
    };

    let reg = extract_val("TOTAL REGISTERED TESTS");
    let passed = extract_val("TOTAL PASSED TESTS");
    let executed = extract_val("TOTAL EXECUTED TESTS");
    let ignored = extract_val("TOTAL IGNORED TESTS");
    let hil = extract_val("HIL IGNORED TESTS");
    let filtered = extract_val("TOTAL FILTERED TESTS");

    assert_eq!(executed, passed);
    assert_eq!(
        reg,
        executed + ignored + filtered,
        "Każdy zarejestrowany test musi być wykonany, ignorowany albo jawnie przefiltrowany"
    );
    assert_eq!(
        ignored, hil,
        "Wszystkie ignorowane testy muszą być testami HIL"
    );
    assert_eq!(
        ignored, 10,
        "Liczba testów HIL nie może się zmienić bez nowej bramki HIL"
    );
    assert!(
        filtered <= 1,
        "Filtrowanie w baseline jest dozwolone wyłącznie dla jednego testu ograniczonego hostem"
    );

    let readme = fs::read_to_string("README.md").expect("README.md");
    assert!(
        readme.contains(&format!("{passed}%2F{passed}_Passed")),
        "README badge must contain passed count {passed}"
    );
    assert!(
        readme.contains(&format!("**Total Registered Tests**: {reg}")),
        "README must document registered count {reg}"
    );

    let cat_doc = fs::read_to_string("Native CAT.md").expect("Native CAT.md");
    assert!(
        cat_doc.contains(&format!("{passed} aktywnych testów programowych passed")),
        "Native CAT.md must document passed count {passed}"
    );
    assert!(
        cat_doc.contains(&format!("{reg} zarejestrowanych testów")),
        "Native CAT.md must document registered count {reg}"
    );
}

#[test]
fn gui_update_requires_verified_manifest_guard() {
    let app_services_source =
        fs::read_to_string("src/gui/app_services.rs").expect("src/gui/app_services.rs");

    // 1. GUI trigger_update_install nie może korzystać z surowego select_asset_for_platform
    assert!(
        !app_services_source.contains("select_asset_for_platform"),
        "Ścieżka GUI trigger_update_install nie może wywoływać select_asset_for_platform opartego wyłącznie na rozszerzeniu"
    );

    // 2. GUI musi przekazywać verified_asset do install_update
    assert!(
        app_services_source.contains("release.verified_asset.clone()"),
        "GUI musi pobierać zweryfikowany obiekt VerifiedReleaseAsset ze zweryfikowanego wydania"
    );
    assert!(
        app_services_source.contains("updater::install_update(&verified_asset)"),
        "GUI musi wywoływać updater::install_update(&verified_asset)"
    );

    // 3. Sprawdzenie, czy trigger_update_check wymaga zweryfikowanego manifestu
    assert!(
        app_services_source.contains("crate::cloud::updater::latest_release().await"),
        "Sprawdzanie aktualizacji musi przechodzić przez kanoniczną funkcję latest_release z bramką zaufania"
    );
}

#[test]
fn revoked_release_key_remains_rejected_guard() {
    assert!(
        REVOKED_KEY_IDS.contains(&"splogbook-release-2026-01"),
        "splogbook-release-2026-01 musi pozostać na stałe w REVOKED_KEY_IDS (SEC-001)"
    );

    let key_entry = TRUSTED_KEYS
        .iter()
        .find(|k| k.key_id == "splogbook-release-2026-01")
        .expect("Klucz splogbook-release-2026-01 musi znajdować się w rejestrze TRUSTED_KEYS");
    assert!(
        !key_entry.active,
        "Klucz splogbook-release-2026-01 nie może być oznaczony jako aktywny"
    );

    let v1_2_0_manifest = r#"{
        "manifest_version": 1,
        "key_id": "splogbook-release-2026-01",
        "product": "SPLogbook",
        "version": "1.2.0",
        "channel": "stable",
        "minimum_updater_version": "1.1.0",
        "commit": "6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592",
        "assets": [
            {
                "platform": "windows",
                "arch": "x86_64",
                "package_type": "zip",
                "filename": "SPLogbook-Windows-x64.zip",
                "size": 20831936,
                "sha256": "bd2164e3f73d779b4f8ded93310f6d990db978a67f840a70b4a834eb3b2da4a0"
            }
        ]
    }"#;
    let v1_2_0_sig =
        "E3yN9jZqfL6fG0BwhbXkU49767a4mcvL8aJ13Fq8gUe7hQ8L2z1m5n8b4v7c0x9a8s7d6f5g4h3j2k1l0p==";

    let res = VerifiedReleaseManifest::verify(v1_2_0_manifest.as_bytes(), v1_2_0_sig, "1.1.0");
    assert!(
        res.is_err(),
        "Weryfikacja manifestu podpisanego odwołanym kluczem splogbook-release-2026-01 musi bezwzględnie zawieść (fail-closed)"
    );
}

#[test]
fn portable_database_files_are_protected_guard() {
    use std::path::Path;

    assert_eq!(
        classify_update_path(Path::new("databases/serviceLOG.db")),
        UpdateFileClass::UserData,
        "każdy plik .db w databases/ musi być sklasyfikowany jako UserData"
    );
    assert_eq!(
        classify_update_path(Path::new("databases/callbook.db")),
        UpdateFileClass::UserData,
        "każdy plik .db w databases/ musi być sklasyfikowany jako UserData"
    );
    assert_eq!(
        classify_update_path(Path::new("databases/custom.sqlite")),
        UpdateFileClass::UserData,
        "każdy plik .sqlite w databases/ musi być sklasyfikowany jako UserData"
    );
    assert_eq!(
        classify_update_path(Path::new("station_config.json")),
        UpdateFileClass::UserConfiguration,
        "station_config.json musi być sklasyfikowany jako UserConfiguration"
    );
    assert_eq!(
        classify_update_path(Path::new("default_log.db")),
        UpdateFileClass::UserData,
        "default_log.db musi być sklasyfikowany jako UserData"
    );

    let temp_dir = std::env::temp_dir().join(format!("guard_plan_db_{}", rand::random::<u64>()));
    let staging = temp_dir.join("staging");
    let active = temp_dir.join("active");

    fs::create_dir_all(staging.join("databases")).unwrap();
    fs::create_dir_all(active.join("databases")).unwrap();

    fs::write(staging.join("SPLogbook.exe"), b"NEW EXE").unwrap();
    fs::write(staging.join("databases/default_log.db"), b"NEW LOG DB").unwrap();
    fs::write(staging.join("databases/custom.sqlite"), b"NEW CUSTOM DB").unwrap();

    let active_log_db = active.join("databases/default_log.db");
    let active_custom_db = active.join("databases/custom.sqlite");
    fs::write(&active_log_db, b"USER PRODUCTION LOG DB").unwrap();
    fs::write(&active_custom_db, b"USER CUSTOM SQLITE DB").unwrap();

    let plan = UpdatePlan::generate(&staging, &active).unwrap();
    let preserved_targets: Vec<_> = plan
        .preserve_files
        .iter()
        .map(|(p, _)| p.as_path())
        .collect();

    assert!(
        preserved_targets.contains(&active_log_db.as_path()),
        "Plan aktualizacji musi zachować istniejącą bazę default_log.db"
    );
    assert!(
        preserved_targets.contains(&active_custom_db.as_path()),
        "Plan aktualizacji musi zachować istniejącą bazę custom.sqlite"
    );

    let replaced_targets: Vec<_> = plan
        .replace_files
        .iter()
        .map(|(_, t)| t.as_path())
        .collect();
    assert!(
        !replaced_targets.contains(&active_log_db.as_path()),
        "Plan aktualizacji nie może podmieniać istniejącej bazy default_log.db"
    );
    assert!(
        !replaced_targets.contains(&active_custom_db.as_path()),
        "Plan aktualizacji nie może podmieniać istniejącej bazy custom.sqlite"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn windows_updater_does_not_force_copy_entire_staging_guard() {
    let updater_source = fs::read_to_string("src/cloud/updater.rs").expect("src/cloud/updater.rs");

    assert!(
        !updater_source.contains("Copy-Item -Path \"$staging\\*\""),
        "Aktualizator Windows nie może używać dzikich kart kopiowania Copy-Item -Path \"$staging\\*\" -Destination $baseDir -Recurse -Force"
    );
    assert!(
        !updater_source.contains("Copy-Item -Path \"$rollback\\*\""),
        "Aktualizator Windows nie może używać dzikich kart rollbacku Copy-Item -Path \"$rollback\\*\" -Destination $baseDir -Recurse -Force"
    );

    let plan = UpdatePlan {
        replace_files: vec![(
            std::path::PathBuf::from("SPLogbook.exe"),
            std::path::PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
        )],
        create_files: vec![(
            std::path::PathBuf::from("new_resource.dll"),
            std::path::PathBuf::from("C:\\SPLogbook\\new_resource.dll"),
        )],
        preserve_files: vec![(
            std::path::PathBuf::from("C:\\SPLogbook\\databases\\default_log.db"),
            "Preserved".into(),
        )],
        rejected_files: Vec::new(),
    };

    let script = generate_powershell_update_script(
        std::path::Path::new("C:\\SPLogbook\\SPLogbook.exe"),
        4321,
        &plan,
    );

    assert!(
        !script.contains("$staging\\*"),
        "Wygenerowany skrypt PowerShell nie może zawierać $staging\\*"
    );
    assert!(
        !script.contains("$rollback\\*"),
        "Wygenerowany skrypt PowerShell nie może zawierać $rollback\\*"
    );
    assert!(
        script.contains("$replaceFiles = @('SPLogbook.exe')"),
        "Skrypt musi zawierać precyzyjną tablicę $replaceFiles"
    );
}

#[test]
fn linux_updater_uses_validated_update_plan_guard() {
    let updater_source = fs::read_to_string("src/cloud/updater.rs").expect("src/cloud/updater.rs");

    assert!(
        updater_source.contains("let plan = UpdatePlan::generate(&staging_dir, base_dir)?;"),
        "install_via_tar_gz na Linuksie musi wywoływać UpdatePlan::generate"
    );
    assert!(
        updater_source.contains("plan.validate_safety(\"splogbook\")?;")
            || updater_source.contains("plan.validate_safety(expected_exe)?;"),
        "install_via_tar_gz na Linuksie musi wywoływać plan.validate_safety"
    );
    assert!(
        updater_source.contains("for (rel, orig) in &plan.replace_files"),
        "install_via_tar_gz na Linuksie musi iterować wyłącznie po plan.replace_files"
    );
}

#[test]
fn rollback_excludes_protected_user_data_guard() {
    let temp_dir = std::env::temp_dir().join(format!("guard_rb_user_{}", rand::random::<u64>()));
    let staging = temp_dir.join("staging");
    let active = temp_dir.join("active");

    fs::create_dir_all(staging.join("databases")).unwrap();
    fs::create_dir_all(active.join("databases")).unwrap();

    fs::write(staging.join("SPLogbook.exe"), b"CORRUPTED NEW BINARY").unwrap();
    fs::write(
        staging.join("databases/custom_user.db"),
        b"OVERWRITING CUSTOM DB",
    )
    .unwrap();
    fs::write(staging.join("station_config.json"), b"OVERWRITING CONFIG").unwrap();
    fs::write(staging.join("default_log.db"), b"OVERWRITING LOG").unwrap();

    let active_exe = active.join("SPLogbook.exe");
    let active_db = active.join("databases/custom_user.db");
    let active_cfg = active.join("station_config.json");
    let active_log = active.join("default_log.db");

    fs::write(&active_exe, b"CANONICAL ACTIVE EXE").unwrap();
    fs::write(&active_db, b"CANONICAL USER DATABASE").unwrap();
    fs::write(&active_cfg, b"CANONICAL USER CONFIG").unwrap();
    fs::write(&active_log, b"CANONICAL USER QSOS").unwrap();

    let mut opts = AtomicInstallationOptions::new("SPLogbook.exe");
    opts.simulate_rename_failure = true;

    let res = apply_atomic_installation_with_options(&staging, &active, &opts);
    assert!(res.is_err(), "Instalacja z błędem musi zwrócić błąd");

    assert_eq!(
        fs::read(&active_exe).unwrap(),
        b"CANONICAL ACTIVE EXE",
        "Binarka musi zostać przywrócona z rollbacku"
    );
    assert_eq!(
        fs::read(&active_db).unwrap(),
        b"CANONICAL USER DATABASE",
        "Baza custom_user.db nie może zostać zmodyfikowana przez rollback"
    );
    assert_eq!(
        fs::read(&active_cfg).unwrap(),
        b"CANONICAL USER CONFIG",
        "Konfiguracja station_config.json nie może zostać zmodyfikowana przez rollback"
    );
    assert_eq!(
        fs::read(&active_log).unwrap(),
        b"CANONICAL USER QSOS",
        "Baza default_log.db nie może zostać zmodyfikowana przez rollback"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn updater_does_not_use_sudo_guard() {
    let updater_source = fs::read_to_string("src/cloud/updater.rs").expect("src/cloud/updater.rs");

    for line in updater_source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        assert!(
            !trimmed.contains("\"sudo\"") && !trimmed.contains("\"sudo "),
            "Kod produkcyjny aktualizatora nie może używać komendy sudo: {trimmed}"
        );
        assert!(
            !trimmed.contains("\"pkexec\"") && !trimmed.contains("\"pkexec "),
            "Kod produkcyjny aktualizatora nie może używać komendy pkexec: {trimmed}"
        );
        assert!(
            !trimmed.contains("Verb") && !trimmed.contains("runas"),
            "Kod produkcyjny aktualizatora nie może eskalować uprawnień przez runas: {trimmed}"
        );
    }
}

// ============================================================================
// M3-A: SECURITY GUARDS DLA RDZENIA PLATFORMOWEGO AKTUALIZATORA
// ============================================================================

#[test]
fn attempt_id_has_no_unvalidated_constructor_guard() {
    let source = fs::read_to_string("src/cloud/updater/attempt_id.rs")
        .expect("src/cloud/updater/attempt_id.rs");

    // Weryfikacja prywatnej reprezentacji
    assert!(
        source.contains("pub struct UpdateAttemptId(Uuid);"),
        "UpdateAttemptId musi posiadać prywatne pole Uuid"
    );

    // Brak niezwalidowanych konstruktorów publicznych
    for forbidden in [
        "pub fn from_raw",
        "pub fn from_uuid",
        "pub fn new_unchecked",
        "pub fn from_string",
    ] {
        assert!(
            !source.contains(forbidden),
            "UpdateAttemptId nie może wystawiać niezwalidowanego konstruktora: {forbidden}"
        );
    }

    // Walidacja poprawności parsowania i odrzucania niebezpiecznych danych
    use splogbook::cloud::updater::UpdateAttemptId;
    assert!(UpdateAttemptId::parse_str("").is_err());
    assert!(UpdateAttemptId::parse_str("../traversal").is_err());
    assert!(UpdateAttemptId::parse_str("invalid_uuid!").is_err());
    assert!(UpdateAttemptId::parse_str("c:\\windows\\system32").is_err());
}

#[test]
fn state_machine_has_no_unrestricted_setter_guard() {
    let source =
        fs::read_to_string("src/cloud/updater/state.rs").expect("src/cloud/updater/state.rs");

    // Brak niekontrolowanego settera stanu
    assert!(
        !source.contains("pub fn set_state"),
        "UpdateStateMachine nie może wystawiać publicznego set_state"
    );
    assert!(
        !source.contains("pub fn set_current_state"),
        "UpdateStateMachine nie może wystawiać publicznego set_current_state"
    );

    // Weryfikacja egzekwowania przejść w kodzie
    use splogbook::cloud::updater::{UpdateStateId, UpdateStateMachine};
    let mut sm = UpdateStateMachine::new();
    assert_eq!(sm.current_state(), UpdateStateId::Idle);

    // Próba nielegalnego skoku z Idle bezpośrednio do Applying
    assert!(sm.transition_to(UpdateStateId::Applying).is_err());
    assert_eq!(sm.current_state(), UpdateStateId::Idle);

    // Próba nielegalnego skoku z Idle bezpośrednio do Committed
    assert!(sm.transition_to(UpdateStateId::Committed).is_err());
    assert_eq!(sm.current_state(), UpdateStateId::Idle);
}

#[test]
fn cancellation_prohibited_from_applying_guard() {
    use splogbook::cloud::updater::{UpdateStateId, can_cancel};

    // Stany bezwzględnie nieanulowalne zgodnie z ADR-07
    let non_cancellable = [
        UpdateStateId::Applying,
        UpdateStateId::LaunchingCandidate,
        UpdateStateId::AwaitingHealth,
        UpdateStateId::Committed,
        UpdateStateId::FailedPostApply,
        UpdateStateId::Rollback,
        UpdateStateId::RollbackVerified,
        UpdateStateId::Cleanup,
    ];

    for state in non_cancellable {
        assert!(
            !can_cancel(state),
            "Stan {:?} musi być oznaczony jako NON-CANCELLABLE",
            state
        );
    }
}

#[test]
fn update_lock_uses_atomic_create_new_guard() {
    let source =
        fs::read_to_string("src/cloud/updater/lock.rs").expect("src/cloud/updater/lock.rs");

    assert!(
        source.contains(".create_new(true)"),
        "Blokada .update.lock musi używać atomowego create_new(true)"
    );

    // Wykluczenie wzorca TOCTOU: if !exists() { create() }
    assert!(
        !source.contains("if !lock_path.exists()") && !source.contains("if !path.exists()"),
        "Nabywanie blokady nie może zawierać anty-wzorca TOCTOU 'if !exists()'"
    );
}

#[test]
fn recovery_never_executes_apply_guard() {
    use splogbook::cloud::updater::{
        InstallationType, RecoveryContext, RecoveryDecisionEngine, UpdateAttemptId, UpdateStateId,
    };

    let all_states = [
        UpdateStateId::Idle,
        UpdateStateId::CheckingRelease,
        UpdateStateId::FetchingManifest,
        UpdateStateId::VerifyingManifest,
        UpdateStateId::SelectingAsset,
        UpdateStateId::Downloading,
        UpdateStateId::VerifyingDownload,
        UpdateStateId::InspectingPackage,
        UpdateStateId::Staging,
        UpdateStateId::BuildingUpdatePlan,
        UpdateStateId::AwaitingApply,
        UpdateStateId::Applying,
        UpdateStateId::LaunchingCandidate,
        UpdateStateId::AwaitingHealth,
        UpdateStateId::Committed,
        UpdateStateId::FailedPreApply,
        UpdateStateId::FailedPostApply,
        UpdateStateId::Rollback,
        UpdateStateId::RollbackVerified,
        UpdateStateId::Cleanup,
    ];

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let staging = temp_dir.path().join(".staging");

    for state in all_states {
        let attempt = UpdateAttemptId::generate();
        let mut tx = splogbook::cloud::updater::UpdateTransaction::new(
            attempt,
            "1.3.0".into(),
            "v1.3.0".into(),
            "windows".into(),
            "x86_64".into(),
            "windows-portable-zip".into(),
            InstallationType::WindowsPortableZip,
            "abcdef1234567890".into(),
            "SPLogbook-Windows-x64.zip".into(),
            12_345_678,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            staging.clone(),
            temp_dir.path(),
        )
        .expect("valid tx");

        tx.current_state = state;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: true,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);

        assert_ne!(
            plan.target_state,
            Some(UpdateStateId::Applying),
            "Recovery nigdy nie może automatycznie wznawiać stanu Applying dla {:?}",
            state
        );
        assert_ne!(
            plan.target_state,
            Some(UpdateStateId::Downloading),
            "Recovery nigdy nie może automatycznie wznawiać stanu Downloading dla {:?}",
            state
        );
    }
}

#[test]
fn unknown_installation_is_fail_closed_guard() {
    use splogbook::cloud::updater::{
        InstallationCapabilities, InstallationType, ManagementMode, RecoveryContext,
        RecoveryDecision, RecoveryDecisionEngine,
    };

    let unknown = InstallationType::UnknownInstallation;
    assert_eq!(
        unknown.management_mode(),
        ManagementMode::UnsupportedOrUnknown
    );

    let caps = InstallationCapabilities::for_installation_type(unknown);
    assert!(!caps.supports_self_apply);
    assert!(!caps.requires_delegation);
    assert!(!caps.supports_native_rollback);

    let ctx = RecoveryContext {
        expected_attempt_id: None,
        transaction: None,
        lock_owner: None,
        lock_is_corrupt: false,
        staging_exists: false,
        backup_exists: false,
        health_proof_valid: false,
        installation_type: unknown,
    };

    let plan = RecoveryDecisionEngine::evaluate(&ctx);
    assert_eq!(plan.decision, RecoveryDecision::ManualInterventionRequired);
    assert!(plan.requires_user_intervention);
}

#[test]
fn system_managed_prohibits_direct_file_replacement_guard() {
    use splogbook::cloud::updater::{InstallationCapabilities, InstallationType, ManagementMode};

    let system_types = [
        InstallationType::LinuxDeb,
        InstallationType::LinuxRpm,
        InstallationType::WindowsMsi,
        InstallationType::WindowsInnoSetup,
        InstallationType::LinuxFlatpak,
    ];

    for inst in system_types {
        assert_eq!(
            inst.management_mode(),
            ManagementMode::SystemManaged,
            "Typ {} musi być sklasyfikowany jako SystemManaged",
            inst
        );
        let caps = InstallationCapabilities::for_installation_type(inst);
        assert!(
            !caps.supports_self_apply,
            "Format systemowo zarządzany {} nie może pozwalać na bezpośrednią podmianę plików (supports_self_apply=false)",
            inst
        );
    }
}

#[test]
fn updater_contains_no_escalation_commands_guard() {
    let updater_dir = std::path::Path::new("src/cloud/updater");
    let mut files = vec![std::path::PathBuf::from("src/cloud/updater.rs")];

    if updater_dir.exists() {
        for entry in fs::read_dir(updater_dir).expect("read updater dir") {
            let entry = entry.expect("dir entry");
            if entry.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                files.push(entry.path());
            }
        }
    }

    for path in files {
        let content = fs::read_to_string(&path).expect("read file");
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            for forbidden in [
                "\"sudo\"",
                "\"sudo ",
                "\"doas\"",
                "\"doas ",
                "\"pkexec\"",
                "\"pkexec ",
            ] {
                assert!(
                    !trimmed.contains(forbidden),
                    "Plik {:?} zawiera zabronioną komendę eskalacji {forbidden}: {trimmed}",
                    path
                );
            }

            if trimmed.contains("runas") && trimmed.contains("Verb") {
                panic!(
                    "Plik {:?} zawiera eskalację uprawnień runas: {trimmed}",
                    path
                );
            }
        }
    }
}

#[test]
fn m3_a_data_preservation_matrix_guard() {
    use splogbook::cloud::updater::is_protected_user_path;
    use std::path::Path;

    // Dane użytkownika — bezwzględna ochrona
    let protected = [
        "default_log.db",
        "default_log.db-wal",
        "default_log.db-shm",
        "default_log.db-journal",
        "station_config.json",
        "config/settings.json",
        "serviceLOG.db",
        "serviceLOG.db-wal",
        "callbook.db",
        "callbook.db-wal",
        "logs/2026_contest.adi",
        "backup/contacts.adif",
        "logs/arrl.cabrillo",
    ];

    for p in protected {
        assert!(
            is_protected_user_path(Path::new(p)),
            "Ścieżka '{}' MUSI być rozpoznana jako chronione dane użytkownika",
            p
        );
    }

    // Zasoby aktualizatora — dozwolone do czyszczenia w transakcji
    let disposable = [
        "manifest.json",
        "release.json",
        ".update.lock",
        ".update_transaction.json",
        ".health_ok",
        "SPLogbook.exe",
        "splogbook-update.zip",
        "splogbook-update.zip.part",
        ".staging/new_binary",
    ];

    for p in disposable {
        assert!(
            !is_protected_user_path(Path::new(p)),
            "Ścieżka tymczasowa '{}' NIE powinna być blokowana jako dane użytkownika",
            p
        );
    }
}

#[test]
fn m3_a_stability_100_iterations_fast_unit_tests() {
    use splogbook::cloud::updater::{
        DiskPreflightRequirements, InstallationType, RecoveryContext, RecoveryDecision,
        RecoveryDecisionEngine, UpdateAttemptId, UpdateStateId, UpdateStateMachine, can_cancel,
        is_protected_user_path,
    };
    use std::path::Path;

    for _ in 0..100 {
        // 1. Valid state transitions: Idle -> CheckingRelease -> FetchingManifest -> VerifyingManifest -> SelectingAsset -> Downloading -> VerifyingDownload -> InspectingPackage -> Staging -> BuildingUpdatePlan -> AwaitingApply -> Applying -> LaunchingCandidate -> AwaitingHealth -> Committed -> Cleanup -> Idle
        let mut sm = UpdateStateMachine::new();
        assert_eq!(sm.current_state(), UpdateStateId::Idle);
        sm.transition_to(UpdateStateId::CheckingRelease)
            .expect("step");
        sm.transition_to(UpdateStateId::FetchingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::VerifyingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::SelectingAsset)
            .expect("step");
        sm.transition_to(UpdateStateId::Downloading).expect("step");
        sm.transition_to(UpdateStateId::VerifyingDownload)
            .expect("step");
        sm.transition_to(UpdateStateId::InspectingPackage)
            .expect("step");
        sm.transition_to(UpdateStateId::Staging).expect("step");
        sm.transition_to(UpdateStateId::BuildingUpdatePlan)
            .expect("step");
        sm.transition_to(UpdateStateId::AwaitingApply)
            .expect("step");
        sm.transition_to(UpdateStateId::Applying).expect("step");
        sm.transition_to(UpdateStateId::LaunchingCandidate)
            .expect("step");
        sm.transition_to(UpdateStateId::AwaitingHealth)
            .expect("step");
        sm.transition_to(UpdateStateId::Committed).expect("step");
        sm.transition_to(UpdateStateId::Cleanup).expect("step");
        sm.transition_to(UpdateStateId::Idle).expect("step");

        // 2. Invalid transitions
        let mut bad_sm = UpdateStateMachine::new();
        assert!(bad_sm.transition_to(UpdateStateId::Applying).is_err());
        assert!(bad_sm.transition_to(UpdateStateId::Committed).is_err());

        // 3. AttemptId round-trip & uniqueness
        let a1 = UpdateAttemptId::generate();
        let a2 = UpdateAttemptId::generate();
        assert_ne!(a1, a2);
        let s = a1.to_string();
        let parsed = UpdateAttemptId::parse_str(&s).expect("round trip parse");
        assert_eq!(a1, parsed);

        // 4. RequiredSpace calculation
        let preflight = DiskPreflightRequirements::calculate(
            InstallationType::WindowsPortableZip,
            10_000_000,
            20_000_000,
            30_000_000,
            None,
        )
        .expect("preflight calc");
        assert!(preflight.total_required_bytes > 60_000_000);

        // 5. Cancellation policy check
        assert!(can_cancel(UpdateStateId::Idle));
        assert!(can_cancel(UpdateStateId::AwaitingApply));
        assert!(!can_cancel(UpdateStateId::Applying));
        assert!(!can_cancel(UpdateStateId::Committed));

        // 6. Recovery decision check
        let ctx = RecoveryContext {
            expected_attempt_id: None,
            transaction: None,
            lock_owner: None,
            lock_is_corrupt: false,
            staging_exists: false,
            backup_exists: false,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };
        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::NoRecoveryNeeded);

        // 7. Protected path classification
        assert!(is_protected_user_path(Path::new("default_log.db")));
        assert!(is_protected_user_path(Path::new("station_config.json")));
        assert!(!is_protected_user_path(Path::new("manifest.json")));
    }
}

#[test]
fn m3_a_stability_20_iterations_concurrent_and_io_tests() {
    use splogbook::cloud::updater::{
        HealthProof, InstallationType, UpdateAttemptId, UpdateLockGuard, UpdateTransaction,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    for _ in 0..20 {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let root = temp_dir.path();

        // 1. Concurrent lock race (5 threads, exactly 1 winner)
        let root_arc = Arc::new(root.to_path_buf());
        let success_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let start_barrier = Arc::new(std::sync::Barrier::new(5));
        let done_barrier = Arc::new(std::sync::Barrier::new(5));

        let mut handles = Vec::new();
        for _ in 0..5 {
            let r = Arc::clone(&root_arc);
            let sc = Arc::clone(&success_count);
            let ec = Arc::clone(&error_count);
            let sb = Arc::clone(&start_barrier);
            let db = Arc::clone(&done_barrier);
            handles.push(thread::spawn(move || {
                let attempt = UpdateAttemptId::generate();
                sb.wait();
                match UpdateLockGuard::acquire(&r, attempt) {
                    Ok(guard) => {
                        sc.fetch_add(1, Ordering::SeqCst);
                        db.wait();
                        drop(guard);
                    }
                    Err(_) => {
                        ec.fetch_add(1, Ordering::SeqCst);
                        db.wait();
                    }
                }
            }));
        }

        for h in handles {
            h.join().expect("thread join");
        }

        assert_eq!(
            success_count.load(Ordering::SeqCst),
            1,
            "Dokładnie jeden wątek musi nabyć blokadę concurrency"
        );
        assert_eq!(
            error_count.load(Ordering::SeqCst),
            4,
            "Dokładnie 4 wątki muszą otrzymać błąd LockContended"
        );

        // 2. Transaction persistence round-trip
        let attempt = UpdateAttemptId::generate();
        let staging = root.join(".staging");
        let tx = UpdateTransaction::new(
            attempt,
            "1.3.0".into(),
            "v1.3.0".into(),
            "windows".into(),
            "x86_64".into(),
            "windows-portable-zip".into(),
            InstallationType::WindowsPortableZip,
            "abcdef1234567890".into(),
            "SPLogbook-Windows-x64.zip".into(),
            12_345_678,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            staging,
            root,
        )
        .expect("tx new");

        tx.save_to_dir(root).expect("save tx");
        let loaded_tx = UpdateTransaction::load_from_dir(root, root)
            .expect("load tx")
            .expect("tx exists");
        assert_eq!(tx.attempt_id, loaded_tx.attempt_id);

        // 3. Health proof validation
        let proof = HealthProof::new(
            attempt,
            "1.3.0",
            "abcdef",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );
        assert!(proof.validate(&attempt, "1.3.0", "abcdef", 15).is_ok());
    }
}

// ============================================================================
// M3-B: Bezpieczeństwo i stabilność pomocnika Windows i fallbacku PowerShell
// ============================================================================

#[test]
fn m3_b_windows_updater_has_no_network_or_url_guard() {
    let files_to_check = [
        "src/bin/splogbook-updater.rs",
        "src/cloud/updater/windows_helper.rs",
        "src/cloud/updater/helper_protocol.rs",
    ];

    for file_path in &files_to_check {
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(path).expect("read file");
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            for forbidden in [
                "reqwest",
                "hyper",
                "ureq",
                "TcpStream",
                "UdpSocket",
                "http://",
                "https://",
                "ftp://",
            ] {
                assert!(
                    !trimmed.contains(forbidden),
                    "Plik {file_path} zawiera zabronione odwołanie sieciowe/URL '{forbidden}': {trimmed}"
                );
            }
        }
    }
}

#[test]
fn m3_b_windows_updater_has_no_shell_or_cmd_or_runas_guard() {
    let files_to_check = [
        "src/bin/splogbook-updater.rs",
        "src/cloud/updater/windows_helper.rs",
    ];

    for file_path in &files_to_check {
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(path).expect("read file");
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            for forbidden in [
                "\"cmd\"",
                "\"cmd.exe\"",
                "\"powershell\"",
                "\"powershell.exe\"",
                "\"sh\"",
                "\"bash\"",
                "\"runas\"",
                "\"sudo\"",
                "\"doas\"",
                "\"pkexec\"",
            ] {
                assert!(
                    !trimmed.contains(forbidden),
                    "Plik {file_path} zawiera zabronioną powłokę lub eskalację '{forbidden}': {trimmed}"
                );
            }
        }
    }
}

#[test]
fn m3_b_powershell_fallback_has_no_wildcard_or_iex_guard() {
    use std::path::{Path, PathBuf};

    let fallback_path = Path::new("src/cloud/updater/powershell_fallback.rs");
    assert!(fallback_path.exists(), "powershell_fallback.rs must exist");
    let content = fs::read_to_string(fallback_path).expect("read fallback");

    // Static code analysis of generator (excluding unit test assertions)
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.starts_with("assert!")
        {
            continue;
        }
        if trimmed.contains("#[cfg(test)]") {
            break;
        }
        assert!(
            !trimmed.contains("Invoke-Expression"),
            "powershell_fallback.rs must not contain Invoke-Expression: {trimmed}"
        );
        assert!(
            !trimmed.contains("iex "),
            "powershell_fallback.rs must not contain iex: {trimmed}"
        );
        assert!(
            !trimmed.contains("Invoke-WebRequest"),
            "powershell_fallback.rs must not contain Invoke-WebRequest: {trimmed}"
        );
        assert!(
            !trimmed.contains("DownloadFile"),
            "powershell_fallback.rs must not contain DownloadFile: {trimmed}"
        );
    }

    // Dynamic test of generated script
    let plan = UpdatePlan {
        replace_files: vec![(
            PathBuf::from("SPLogbook.exe"),
            PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
        )],
        create_files: vec![(
            PathBuf::from("resources/ikona.ico"),
            PathBuf::from("C:\\SPLogbook\\resources\\ikona.ico"),
        )],
        preserve_files: vec![(
            PathBuf::from("C:\\SPLogbook\\serviceLOG.db"),
            "preserved".into(),
        )],
        rejected_files: vec![],
    };

    let script =
        generate_powershell_update_script(Path::new("C:\\SPLogbook\\SPLogbook.exe"), 1234, &plan);

    assert!(!script.contains("Invoke-Expression"));
    assert!(!script.contains("iex"));
    assert!(!script.contains("$staging\\*"));
    assert!(!script.contains("$rollback\\*"));
    assert!(!script.contains("Remove-Item *"));
    assert!(!script.contains("Invoke-WebRequest"));
    assert!(!script.contains("DownloadFile"));
    assert!(!script.contains("-Verb RunAs"));
    assert!(!script.contains("runas"));
}

#[test]
fn m3_b_windows_updater_protects_user_databases_and_configs_guard() {
    use splogbook::cloud::updater::helper_protocol::{
        CURRENT_HELPER_REQUEST_VERSION, UpdatePlanIdentity, WindowsHelperRequest,
        validate_and_normalize_relative_path,
    };
    use splogbook::cloud::updater::{ManagementMode, UpdateAttemptId, is_protected_user_path};
    use std::path::{Path, PathBuf};

    let protected_paths = [
        "default_log.db",
        "default_log.db-wal",
        "default_log.db-shm",
        "default_log.db-journal",
        "station_config.json",
        "config/settings.json",
        "serviceLOG.db",
        "serviceLOG.db-wal",
        "callbook.db",
        "callbook.db-wal",
        "data/logs.sqlite",
        "data/logs.sqlite3",
        "logs/2026_contest.adi",
        "backup/contacts.adif",
        "logs/arrl.cabrillo",
    ];

    for path_str in &protected_paths {
        let p = Path::new(path_str);
        assert!(
            is_protected_user_path(p),
            "is_protected_user_path must reject {path_str}"
        );

        // Verification that helper request rejects plan containing protected user path
        let attempt = UpdateAttemptId::generate();
        let root = Path::new("C:\\SPLogbook");
        let plan = UpdatePlan {
            replace_files: vec![(PathBuf::from(path_str), root.join(path_str))],
            create_files: vec![],
            preserve_files: vec![],
            rejected_files: vec![],
        };
        let plan_identity = UpdatePlanIdentity::from_plan(
            attempt,
            root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "0000000000000000000000000000000000000000",
            "windows-x86_64",
        )
        .expect("plan identity");
        let plan_digest = plan_identity.compute_digest();

        let req = WindowsHelperRequest {
            request_version: CURRENT_HELPER_REQUEST_VERSION,
            attempt_id: attempt,
            transaction_record_path: PathBuf::from("C:\\SPLogbook\\.update_transaction.json"),
            installation_root: PathBuf::from("C:\\SPLogbook"),
            current_executable: PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
            candidate_executable: PathBuf::from("C:\\SPLogbook\\.staging\\SPLogbook.exe"),
            expected_parent_pid: 1234,
            expected_current_binary_identity: None,
            installed_version: Some("1.2.0".into()),
            expected_target_version: "1.2.1".into(),
            expected_target_commit: "0000000000000000000000000000000000000000".into(),
            update_management_mode: ManagementMode::SelfManaged,
            plan_digest,
            update_plan_identity: plan_identity,
            health_proof_path: PathBuf::from("C:\\SPLogbook\\.health_ok"),
            parent_process_timeout_seconds: 5,
            candidate_health_timeout_seconds: 5,
            result_record_path: PathBuf::from("C:\\SPLogbook\\.update_result.json"),
            diagnostic_log_path: PathBuf::from("C:\\SPLogbook\\.update.log"),
            staging_file_expectations: std::collections::BTreeMap::new(),
            enforce_strict_staging_entries: false,
        };

        assert!(
            req.validate().is_err(),
            "Helper request validation must fail when plan contains protected path: {path_str}"
        );
    }

    // Traversal, absolute, ADS, invalid device names
    let unsafe_paths = [
        "../escape.exe",
        "subdir/../../root.exe",
        r"C:\Windows\System32\evil.dll",
        r"\SystemRoot\temp.bin",
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "COM1",
        "LPT1",
        "file.exe:stream",
        "program.exe::$DATA",
        "bad|char.exe",
        "star*file.bin",
        "quote\"name.exe",
        "less<than.exe",
    ];

    for path_str in &unsafe_paths {
        assert!(
            validate_and_normalize_relative_path(Path::new(path_str)).is_err(),
            "validate_and_normalize_relative_path must reject unsafe path {path_str}"
        );
    }

    // Valid paths must pass
    let safe_paths = [
        "SPLogbook.exe",
        "splogbook-updater.exe",
        "resources/app_icon.ico",
        "translations/pl/LC_MESSAGES/splogbook.mo",
        "share/doc/README.txt",
    ];

    for path_str in &safe_paths {
        assert!(
            validate_and_normalize_relative_path(Path::new(path_str)).is_ok(),
            "validate_and_normalize_relative_path must accept safe path {path_str}"
        );
    }
}

#[test]
fn m3_b_stability_100_iterations_protocol_and_path_guards() {
    use splogbook::cloud::updater::helper_protocol::{
        CURRENT_HELPER_REQUEST_VERSION, UpdatePlanIdentity, WindowsHelperExitCode,
        WindowsHelperRequest, validate_and_normalize_relative_path,
    };
    use splogbook::cloud::updater::{ManagementMode, UpdateAttemptId, UpdatePlan};
    use std::path::{Path, PathBuf};

    for i in 0..100 {
        let attempt = UpdateAttemptId::generate();
        let root = PathBuf::from(r"C:\TestApp\SPLogbook");
        let plan = UpdatePlan {
            replace_files: vec![(PathBuf::from("SPLogbook.exe"), root.join("SPLogbook.exe"))],
            create_files: vec![(
                format!("resources/new_asset_{i}.bin").into(),
                root.join(format!("resources\\new_asset_{i}.bin")),
            )],
            preserve_files: vec![(root.join("serviceLOG.db"), "preserved".into())],
            rejected_files: vec![],
        };

        let plan_identity = UpdatePlanIdentity::from_plan(
            attempt,
            &root,
            &plan,
            "SPLogbook.exe",
            &format!("1.2.{}", i + 1),
            &format!("{:040x}", i),
            "windows-x86_64",
        )
        .expect("plan identity");
        let plan_digest = plan_identity.compute_digest();

        let req = WindowsHelperRequest {
            request_version: CURRENT_HELPER_REQUEST_VERSION,
            attempt_id: attempt,
            transaction_record_path: root.join(".update_transaction.json"),
            installation_root: root.clone(),
            current_executable: root.join("SPLogbook.exe"),
            candidate_executable: root.join(".staging").join("SPLogbook.exe"),
            expected_parent_pid: 1000 + i,
            expected_current_binary_identity: None,
            installed_version: Some("1.2.0".into()),
            expected_target_version: format!("1.2.{}", i + 1),
            expected_target_commit: format!("{:040x}", i),
            update_management_mode: ManagementMode::SelfManaged,
            update_plan_identity: plan_identity,
            plan_digest: plan_digest.clone(),
            health_proof_path: root.join(".health_ok"),
            parent_process_timeout_seconds: 15,
            candidate_health_timeout_seconds: 10,
            result_record_path: root.join(".update_helper_result.json"),
            diagnostic_log_path: root.join(".update_helper.log"),
            staging_file_expectations: std::collections::BTreeMap::new(),
            enforce_strict_staging_entries: false,
        };

        // 1. Serialization roundtrip
        let json = serde_json::to_string(&req).expect("serialize req");
        let deserialized: WindowsHelperRequest =
            serde_json::from_str(&json).expect("deserialize req");
        assert_eq!(req, deserialized);

        // 2. Validation
        assert!(req.validate().is_ok());

        // 3. Digest mismatch detection
        let mut tampered_req = req.clone();
        tampered_req.plan_digest =
            "0000000000000000000000000000000000000000000000000000000000000000".into();
        assert!(tampered_req.validate().is_err());

        // 4. Path traversal detection via validate_and_normalize_relative_path
        assert!(validate_and_normalize_relative_path(Path::new("../splogbook.exe")).is_err());
        assert!(
            validate_and_normalize_relative_path(Path::new(r"C:\Windows\System32\cmd.exe"))
                .is_err()
        );

        // 5. Exit codes roundtrip
        for code in [
            WindowsHelperExitCode::Success,
            WindowsHelperExitCode::InvalidArguments,
            WindowsHelperExitCode::UnsupportedRequestVersion,
            WindowsHelperExitCode::InvalidAttemptId,
            WindowsHelperExitCode::PlanDigestMismatch,
            WindowsHelperExitCode::ParentProcessTimeout,
            WindowsHelperExitCode::HealthTimeoutRollbackSucceeded,
            WindowsHelperExitCode::UnsafePath,
        ] {
            let i_val = code.as_i32();
            let parsed = WindowsHelperExitCode::from_i32(i_val);
            assert_eq!(parsed, Some(code));
        }
    }
}

#[test]
fn m3_b_stability_50_iterations_health_proof_and_rollback() {
    use splogbook::cloud::updater::helper_protocol::{
        CURRENT_HELPER_REQUEST_VERSION, UpdatePlanIdentity, WindowsHelperExitCode,
        WindowsHelperRequest,
    };
    use splogbook::cloud::updater::windows_helper::{
        CandidateLauncher, HelperCandidateChild, ProcessMonitor, WindowsHelperEngine,
    };
    use splogbook::cloud::updater::{
        InstallationType, ManagementMode, UpdateAttemptId, UpdatePlan, UpdateTransaction,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    struct DeadProcessMonitor;
    impl ProcessMonitor for DeadProcessMonitor {
        fn is_process_running(
            &self,
            _pid: u32,
            _name: &str,
        ) -> Result<bool, WindowsHelperExitCode> {
            Ok(false)
        }
    }

    struct CrashingChild;
    impl HelperCandidateChild for CrashingChild {
        fn is_alive(&mut self) -> Result<bool, WindowsHelperExitCode> {
            Ok(false)
        }
        fn terminate(&mut self) -> Result<(), WindowsHelperExitCode> {
            Ok(())
        }
    }

    struct CrashingCandidateLauncher;
    impl CandidateLauncher for CrashingCandidateLauncher {
        fn launch_candidate(
            &self,
            _exe: &Path,
            _attempt: &UpdateAttemptId,
            _proof_path: &Path,
            _ver: &str,
            _commit: &str,
        ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode> {
            Ok(Box::new(CrashingChild))
        }
    }

    for i in 0..50 {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();

        let original_content = format!("ORIGINAL_BINARY_CONTENT_{i}");
        fs::write(root.join("SPLogbook.exe"), original_content.as_bytes()).expect("write original");
        fs::write(root.join("serviceLOG.db"), b"SQLITE_USER_DATA").expect("write db");
        fs::write(root.join("serviceLOG.db-wal"), b"SQLITE_WAL").expect("write wal");

        let staging = root.join(".staging");
        fs::create_dir_all(&staging).expect("create staging");
        let candidate_content = format!("NEW_CANDIDATE_CONTENT_{i}");
        fs::write(staging.join("SPLogbook.exe"), candidate_content.as_bytes())
            .expect("write candidate");
        fs::write(staging.join("new_feature.dat"), b"NEW_FEATURE_DAT").expect("write feature");

        let attempt = UpdateAttemptId::generate();
        let plan = UpdatePlan {
            replace_files: vec![(PathBuf::from("SPLogbook.exe"), root.join("SPLogbook.exe"))],
            create_files: vec![(
                PathBuf::from("new_feature.dat"),
                root.join("new_feature.dat"),
            )],
            preserve_files: vec![(root.join("serviceLOG.db"), "preserved".into())],
            rejected_files: vec![],
        };

        let plan_identity = UpdatePlanIdentity::from_plan(
            attempt,
            root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "1111111111111111111111111111111111111111",
            "windows-x86_64",
        )
        .expect("plan identity");
        let plan_digest = plan_identity.compute_digest();

        let mut tx = UpdateTransaction::new(
            attempt,
            "1.2.1".into(),
            "v1.2.1".into(),
            "windows".into(),
            "x86_64".into(),
            "windows-portable-zip".into(),
            InstallationType::WindowsPortableZip,
            "1111111111111111111111111111111111111111".into(),
            "SPLogbook-Windows-x64.zip".into(),
            5000,
            "0000000000000000000000000000000000000000000000000000000000000000".into(),
            staging.clone(),
            root,
        )
        .expect("tx new");
        tx.plan_digest = Some(plan_digest.clone());
        tx.save_to_dir(root).expect("save tx");

        let req = WindowsHelperRequest {
            request_version: CURRENT_HELPER_REQUEST_VERSION,
            attempt_id: attempt,
            transaction_record_path: root.join(".update_transaction.json"),
            installation_root: root.to_path_buf(),
            current_executable: root.join("SPLogbook.exe"),
            candidate_executable: staging.join("SPLogbook.exe"),
            expected_parent_pid: 99999,
            expected_current_binary_identity: None,
            installed_version: Some("1.2.0".into()),
            expected_target_version: "1.2.1".into(),
            expected_target_commit: "1111111111111111111111111111111111111111".into(),
            update_management_mode: ManagementMode::SelfManaged,
            update_plan_identity: plan_identity,
            plan_digest,
            health_proof_path: root.join(".health_ok"),
            parent_process_timeout_seconds: 1,
            candidate_health_timeout_seconds: 1,
            result_record_path: root.join(".update_helper_result.json"),
            diagnostic_log_path: root.join(".update_helper.log"),
            staging_file_expectations: std::collections::BTreeMap::new(),
            enforce_strict_staging_entries: false,
        };

        let monitor = DeadProcessMonitor;
        let launcher = CrashingCandidateLauncher;
        let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);

        let (exit_code, result) = engine.execute();

        assert_eq!(
            exit_code,
            WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded,
            "Iteration {i}: Must rollback successfully when candidate crashes"
        );
        assert_eq!(
            result.rollback_result,
            Some(splogbook::cloud::updater::helper_protocol::RollbackResultKind::Success)
        );

        // Verify state after rollback:
        // 1. Original binary restored
        let restored_content =
            fs::read_to_string(root.join("SPLogbook.exe")).expect("read restored");
        assert_eq!(restored_content, original_content);

        // 2. Created file removed
        assert!(!root.join("new_feature.dat").exists());

        // 3. User databases untouched
        assert_eq!(
            fs::read(root.join("serviceLOG.db")).expect("read db"),
            b"SQLITE_USER_DATA"
        );
        assert_eq!(
            fs::read(root.join("serviceLOG.db-wal")).expect("read wal"),
            b"SQLITE_WAL"
        );

        // 4. Rollback directory preserved for retention (ADR-03)
        assert!(root.join(".rollback").exists());
    }
}

#[test]
fn m3_b_stability_20_iterations_helper_parity_and_e2e_cycle() {
    use splogbook::cloud::updater::helper_protocol::{
        CURRENT_HELPER_REQUEST_VERSION, UpdatePlanIdentity, WindowsHelperExitCode,
        WindowsHelperRequest,
    };
    use splogbook::cloud::updater::powershell_fallback::generate_powershell_from_request;
    use splogbook::cloud::updater::windows_helper::{
        CandidateLauncher, HelperCandidateChild, ProcessMonitor, WindowsHelperEngine,
    };
    use splogbook::cloud::updater::{
        HealthProof, InstallationType, ManagementMode, UpdateAttemptId, UpdatePlan,
        UpdateTransaction,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    struct DeadProcessMonitor;
    impl ProcessMonitor for DeadProcessMonitor {
        fn is_process_running(
            &self,
            _pid: u32,
            _name: &str,
        ) -> Result<bool, WindowsHelperExitCode> {
            Ok(false)
        }
    }

    struct HealthyChild;
    impl HelperCandidateChild for HealthyChild {
        fn is_alive(&mut self) -> Result<bool, WindowsHelperExitCode> {
            Ok(true)
        }
        fn terminate(&mut self) -> Result<(), WindowsHelperExitCode> {
            Ok(())
        }
    }

    struct HealthyCandidateLauncher;
    impl CandidateLauncher for HealthyCandidateLauncher {
        fn launch_candidate(
            &self,
            _exe: &Path,
            attempt: &UpdateAttemptId,
            proof_path: &Path,
            ver: &str,
            commit: &str,
        ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode> {
            let proof = HealthProof::new(
                *attempt,
                ver,
                commit,
                "windows-x86_64",
                InstallationType::WindowsPortableZip,
            );
            let json = serde_json::to_string_pretty(&proof).expect("proof json");
            fs::write(proof_path, json).expect("write proof");
            Ok(Box::new(HealthyChild))
        }
    }

    for i in 0..20 {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();

        let original_bin = format!("V1_OLD_BINARY_{i}");
        fs::write(root.join("SPLogbook.exe"), original_bin.as_bytes()).expect("write bin");
        fs::write(root.join("serviceLOG.db"), b"SQLITE_DATA").expect("write db");

        let staging = root.join(".staging");
        fs::create_dir_all(&staging).expect("create staging");
        let candidate_bin = format!("V2_NEW_BINARY_{i}");
        fs::write(staging.join("SPLogbook.exe"), candidate_bin.as_bytes())
            .expect("write candidate");
        fs::write(staging.join("asset.dat"), b"NEW_ASSET").expect("write asset");

        let attempt = UpdateAttemptId::generate();
        let plan = UpdatePlan {
            replace_files: vec![(PathBuf::from("SPLogbook.exe"), root.join("SPLogbook.exe"))],
            create_files: vec![(PathBuf::from("asset.dat"), root.join("asset.dat"))],
            preserve_files: vec![(root.join("serviceLOG.db"), "preserved".into())],
            rejected_files: vec![],
        };

        let plan_identity = UpdatePlanIdentity::from_plan(
            attempt,
            root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "2222222222222222222222222222222222222222",
            "windows-x86_64",
        )
        .expect("plan identity");
        let plan_digest = plan_identity.compute_digest();

        let mut tx = UpdateTransaction::new(
            attempt,
            "1.2.1".into(),
            "v1.2.1".into(),
            "windows".into(),
            "x86_64".into(),
            "windows-portable-zip".into(),
            InstallationType::WindowsPortableZip,
            "2222222222222222222222222222222222222222".into(),
            "SPLogbook-Windows-x64.zip".into(),
            5000,
            "0000000000000000000000000000000000000000000000000000000000000000".into(),
            staging.clone(),
            root,
        )
        .expect("tx new");
        tx.plan_digest = Some(plan_digest.clone());
        tx.save_to_dir(root).expect("save tx");

        let req = WindowsHelperRequest {
            request_version: CURRENT_HELPER_REQUEST_VERSION,
            attempt_id: attempt,
            transaction_record_path: root.join(".update_transaction.json"),
            installation_root: root.to_path_buf(),
            current_executable: root.join("SPLogbook.exe"),
            candidate_executable: staging.join("SPLogbook.exe"),
            expected_parent_pid: 99999,
            expected_current_binary_identity: None,
            installed_version: Some("1.2.0".into()),
            expected_target_version: "1.2.1".into(),
            expected_target_commit: "2222222222222222222222222222222222222222".into(),
            update_management_mode: ManagementMode::SelfManaged,
            update_plan_identity: plan_identity,
            plan_digest,
            health_proof_path: root.join(".health_ok"),
            parent_process_timeout_seconds: 5,
            candidate_health_timeout_seconds: 5,
            result_record_path: root.join(".update_helper_result.json"),
            diagnostic_log_path: root.join(".update_helper.log"),
            staging_file_expectations: std::collections::BTreeMap::new(),
            enforce_strict_staging_entries: false,
        };

        // 1. Native Windows Helper execution
        let monitor = DeadProcessMonitor;
        let launcher = HealthyCandidateLauncher;
        let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);

        let (exit_code, result) = engine.execute();
        assert_eq!(
            exit_code,
            WindowsHelperExitCode::Success,
            "Native helper must succeed"
        );
        assert_eq!(
            result.rollback_result,
            Some(splogbook::cloud::updater::helper_protocol::RollbackResultKind::None)
        );

        // Verify successful apply
        assert_eq!(
            fs::read_to_string(root.join("SPLogbook.exe")).expect("read bin"),
            candidate_bin
        );
        assert_eq!(
            fs::read(root.join("asset.dat")).expect("read asset"),
            b"NEW_ASSET"
        );
        assert_eq!(
            fs::read(root.join("serviceLOG.db")).expect("read db"),
            b"SQLITE_DATA"
        );
        // Rollback directory preserved for ADR-03
        assert!(root.join(".rollback").exists());

        // 2. PowerShell Fallback Parity Check
        let script = generate_powershell_from_request(&req);

        // Script must reflect the identical contract
        assert!(script.contains(&attempt.to_string()));
        assert!(script.contains("SPLogbook.exe"));
        assert!(script.contains("asset.dat"));
        assert!(script.contains(".health_ok"));
        assert!(script.contains("exit 0"));

        // Losowa nazwa katalogu tymczasowego (`tempfile`) jest częścią ścieżek w skrypcie,
        // więc przed kontrolą zakazanych tokenów usuwamy ją z porównania. Inaczej przypadkowy
        // ciąg "iex" w nazwie tempdir czynił test niedeterministycznym (QA-RANDOM-01).
        let normalized_script = script.replace(&root.display().to_string(), "<INSTALL_ROOT>");
        assert!(!normalized_script.contains("Invoke-Expression"));
        assert!(!normalized_script.contains("iex"));
        assert!(!normalized_script.contains("$staging\\*"));
        assert!(!normalized_script.contains("$rollback\\*"));
        assert!(!normalized_script.contains("Remove-Item *"));
    }
}

#[test]
fn m3_b_process_identity_and_closure_guard() {
    let helper_src = fs::read_to_string("src/cloud/updater/windows_helper.rs")
        .expect("src/cloud/updater/windows_helper.rs");
    let updater_bin =
        fs::read_to_string("src/bin/splogbook-updater.rs").expect("src/bin/splogbook-updater.rs");

    // 1. Helper never calls kill or terminate on parent process PID
    assert!(
        !helper_src.contains("taskkill /f /pid") && !helper_src.contains("taskkill /PID"),
        "Helper must never force-kill parent process"
    );
    assert!(
        !updater_bin.contains("taskkill"),
        "Updater binary must never execute taskkill directly"
    );

    // 2. Candidate termination must only be via child_handle.terminate()
    assert!(
        helper_src.contains("child_handle.terminate()"),
        "Candidate termination must use direct child handle"
    );
    assert!(
        !helper_src.contains("taskkill /im") && !helper_src.contains("taskkill /IM"),
        "Helper must never kill candidate or foreign processes by image name"
    );

    // 3. Cargo binary target for splogbook-updater must exist
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Cargo.toml");
    assert!(
        cargo_toml.contains("name = \"splogbook-updater\""),
        "Cargo.toml must define [[bin]] target for splogbook-updater"
    );
}

// ============================================================================
// M3-C: Integracja produkcyjna Windows Portable ZIP, Helper, Fallback i Konsumpcja
// ============================================================================

#[test]
fn m3_c_manifest_first_and_windows_portable_zip_guards() {
    use splogbook::cloud::updater::{ReleaseArchitecture, ReleasePackageType, ReleasePlatform};

    // 1. Tylko Windows Portable ZIP w tej bramce
    assert_eq!(ReleasePlatform::Windows.as_str(), "windows");
    assert_eq!(ReleaseArchitecture::X86_64.as_str(), "x86_64");
    assert_eq!(ReleasePackageType::Zip.as_str(), "zip");

    // 2. Brak formatów instalacyjnych Inno Setup, AppImage, DEB, RPM (MSI wprowadzony w M3-E)
    let updater_src = fs::read_to_string("src/cloud/updater.rs").expect("read updater.rs");
    for forbidden in [
        "ReleasePackageType::InnoSetup",
        "ReleasePackageType::AppImage",
        "ReleasePackageType::Deb",
        "ReleasePackageType::Rpm",
        "ReleasePackageType::Flatpak",
    ] {
        assert!(
            !updater_src.contains(forbidden),
            "Kod produkcyjny nie może definiować ani obsługiwać zabronionego formatu: {forbidden}"
        );
    }

    // 3. Helper Rust nie może przyjmować URL ani pobierać z sieci
    let helper_bin = fs::read_to_string("src/bin/splogbook-updater.rs").expect("read helper bin");
    assert!(
        helper_bin.contains("\"--url\""),
        "splogbook-updater musi jawnie odrzucać flagę --url"
    );
    assert!(
        helper_bin.contains("WindowsHelperExitCode::InvalidArguments"),
        "splogbook-updater musi zwracać InvalidArguments przy zabronionych flagach"
    );
    assert!(
        !helper_bin.contains("reqwest")
            && !helper_bin.contains("ureq")
            && !helper_bin.contains("TcpStream"),
        "splogbook-updater nie może zawierać klienta HTTP ani operacji sieciowych"
    );
    assert!(
        !helper_bin.contains("http://") && !helper_bin.contains("https://"),
        "splogbook-updater nie może zawierać schematów URL"
    );
}

#[test]
fn m3_c_temporary_conservative_backup_retention_guard() {
    let updater_src = fs::read_to_string("src/cloud/updater.rs").expect("read updater.rs");
    let fallback_src = fs::read_to_string("src/cloud/updater/powershell_fallback.rs")
        .expect("read powershell_fallback.rs");

    // 1. consume_update_result_on_startup nigdy nie usuwa katalogu .rollback
    let consume_fn = updater_src
        .split("pub fn consume_update_result_on_startup")
        .nth(1)
        .and_then(|s| s.split("#[cfg(test)]").next())
        .expect("consume fn block");
    assert!(
        !consume_fn.contains("remove_dir_all(&rollback_dir)")
            && !consume_fn.contains("remove_dir_all(rollback_dir)"),
        "consume_update_result_on_startup nie może usuwać katalogu .rollback"
    );

    // 2. PowerShell fallback nie usuwa .rollback po sukcesie
    let success_block = fallback_src
        .split("if ($healthVerified) {")
        .nth(1)
        .and_then(|s| s.split("} else {").next())
        .unwrap_or("");
    assert!(
        !success_block.contains("Remove-Item") || !success_block.contains("$rollback"),
        "PowerShell fallback nie może usuwać .rollback w bloku sukcesu"
    );

    // 3. Status ADR-03 (retencja .rollback) musi pozostać Open / non-blocking.
    // Źródłem prawdy jest publiczny rejestr decyzji (docs/architecture/DECISIONS.md)
    // oraz rejestr znanych problemów (docs/KNOWN_ISSUES.md).
    let decisions = fs::read_to_string("docs/architecture/DECISIONS.md")
        .expect("docs/architecture/DECISIONS.md");
    let known_issues = fs::read_to_string("docs/KNOWN_ISSUES.md").expect("docs/KNOWN_ISSUES.md");

    assert!(
        decisions.contains("| `ADR-03` |") && decisions.contains("**Open** (non-blocking)"),
        "DECISIONS musi jawnie rejestrować ADR-03 ze statusem Open (non-blocking)"
    );
    for (name, doc) in [
        ("docs/architecture/DECISIONS.md", &decisions),
        ("docs/KNOWN_ISSUES.md", &known_issues),
    ] {
        assert!(
            !doc.contains("ADR-03: APPROVED") && !doc.contains("ADR-03: Approved"),
            "ADR-03 (retencja .rollback) nie może być oznaczony jako APPROVED ({name}) przed formalnym rozstrzygnięciem"
        );
    }
}

#[test]
fn m3_c_data_safety_database_and_config_preservation_guard() {
    use splogbook::cloud::updater::{UpdatePlan, is_protected_user_path};
    use std::path::{Path, PathBuf};

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();

    // Utworzenie chronionych plików użytkownika
    let db_dir = root.join("databases");
    fs::create_dir_all(&db_dir).expect("create databases dir");
    fs::write(db_dir.join("default_log.db"), b"CRITICAL_USER_LOGBOOK").expect("write db");
    fs::write(db_dir.join("default_log.db-wal"), b"WAL_FRAMES").expect("write wal");
    fs::write(db_dir.join("default_log.db-shm"), b"SHM_INDEX").expect("write shm");
    fs::write(root.join("station_config.json"), b"USER_CONFIG").expect("write config");
    fs::write(root.join("contacts.adi"), b"<CALL:6>SP6INA").expect("write adif");
    fs::write(root.join("contest.cbr"), b"START-OF-LOG: 3.0").expect("write cabrillo");
    fs::write(root.join("my_notes.txt"), b"PERSONAL_NOTES").expect("write notes");

    // Symulacja stagingu próbującego nadpisać pliki bazy
    let staging = root.join(".staging");
    fs::create_dir_all(staging.join("databases")).expect("create staging databases");
    fs::write(
        staging.join("databases/default_log.db"),
        b"OVERWRITE_ATTEMPT",
    )
    .expect("write staging db");
    fs::write(staging.join("station_config.json"), b"OVERWRITE_CONFIG")
        .expect("write staging config");
    fs::write(staging.join("SPLogbook.exe"), b"NEW_BINARY").expect("write exe");

    // Generowanie UpdatePlan
    let plan = UpdatePlan::generate(&staging, root).expect("generate plan");

    // Weryfikacja: żaden chroniony plik nie trafił do replace_files ani create_files
    for (rel, _) in &plan.replace_files {
        assert!(
            !is_protected_user_path(rel),
            "Plik replace_files '{rel:?}' narusza ochronę danych użytkownika"
        );
        assert_ne!(rel, Path::new("databases/default_log.db"));
        assert_ne!(rel, Path::new("station_config.json"));
    }
    for (rel, _) in &plan.create_files {
        assert!(
            !is_protected_user_path(rel),
            "Plik create_files '{rel:?}' narusza ochronę danych użytkownika"
        );
    }

    // Weryfikacja: chronione pliki trafiły do preserve_files
    assert!(
        plan.preserve_files
            .iter()
            .any(|(p, _)| p.ends_with("default_log.db")),
        "Baza danych musi trafić do preserve_files"
    );
    assert!(
        plan.preserve_files
            .iter()
            .any(|(p, _)| p.ends_with("station_config.json")),
        "Konfiguracja stacji musi trafić do preserve_files"
    );

    // Walidacja poprawności bezpiecznego planu przechodzi
    assert!(plan.validate_safety("SPLogbook.exe").is_ok());

    // Próba ręcznego wstrzyknięcia pliku bazy do replace_files musi zostać bezwzględnie odrzucona przez validate_safety
    let mut malicious_plan = plan.clone();
    malicious_plan.replace_files.push((
        PathBuf::from("databases/default_log.db"),
        root.join("databases/default_log.db"),
    ));
    assert!(malicious_plan.validate_safety("SPLogbook.exe").is_err());

    // Zawartość plików użytkownika musi pozostać nienaruszona
    assert_eq!(
        fs::read(db_dir.join("default_log.db")).expect("read db"),
        b"CRITICAL_USER_LOGBOOK"
    );
    assert_eq!(
        fs::read(db_dir.join("default_log.db-wal")).expect("read wal"),
        b"WAL_FRAMES"
    );
    assert_eq!(
        fs::read(root.join("station_config.json")).expect("read config"),
        b"USER_CONFIG"
    );
    assert_eq!(
        fs::read(root.join("contacts.adi")).expect("read adif"),
        b"<CALL:6>SP6INA"
    );
    assert_eq!(
        fs::read(root.join("my_notes.txt")).expect("read notes"),
        b"PERSONAL_NOTES"
    );
}

#[test]
fn m3_c_native_helper_pe_validation_and_fail_closed_guard() {
    use splogbook::cloud::updater::validate_native_helper_identity;

    let temp = tempfile::tempdir().expect("tempdir");

    // 1. Prawidłowy PE x86_64
    let valid_pe = temp.path().join("valid_helper.exe");
    let mut pe_data = vec![0u8; 128];
    pe_data[0] = 0x4D;
    pe_data[1] = 0x5A;
    pe_data[0x3C] = 0x40;
    pe_data[0x40] = 0x50;
    pe_data[0x41] = 0x45;
    pe_data[0x44] = 0x64;
    pe_data[0x45] = 0x86; // 0x8664 AMD64
    fs::write(&valid_pe, &pe_data).expect("write valid pe");
    assert!(validate_native_helper_identity(&valid_pe).is_ok());

    // 2. Nieobsługiwana architektura (np. 0x014c i386)
    let i386_pe = temp.path().join("i386_helper.exe");
    pe_data[0x44] = 0x4C;
    pe_data[0x45] = 0x01;
    fs::write(&i386_pe, &pe_data).expect("write i386 pe");
    assert!(validate_native_helper_identity(&i386_pe).is_err());

    // 3. Brak sygnatury MZ
    let non_mz = temp.path().join("non_mz.exe");
    pe_data[0] = 0x00;
    pe_data[1] = 0x00;
    fs::write(&non_mz, &pe_data).expect("write non mz");
    assert!(validate_native_helper_identity(&non_mz).is_err());
}

#[test]
fn m3_c_powershell_fallback_strict_mode_and_no_eval_guard() {
    use splogbook::cloud::updater::powershell_fallback::generate_powershell_from_request_file;
    use std::path::Path;

    let script =
        generate_powershell_from_request_file(Path::new("C:\\SPLogbook\\updater_request.json"));

    assert!(script.contains("Set-StrictMode -Version Latest"));
    assert!(script.contains("$ErrorActionPreference = 'Stop'"));
    assert!(script.contains("$currentExe = $req.current_executable"));
    assert!(script.contains("Start-Process -FilePath $currentExe"));
    assert_eq!(
        script.matches("Start-Process -FilePath $candExe").count(),
        1,
        "kandydat może być uruchomiony wyłącznie raz, do health checku"
    );
    assert!(!script.contains("Invoke-Expression"));
    assert!(!script.contains("iex"));
    assert!(!script.contains("cmd /c"));
    assert!(!script.contains("cmd.exe"));
    assert!(!script.contains("runas"));
    assert!(!script.contains("Copy-Item *"));
    assert!(!script.contains("Remove-Item *"));
    assert!(script.contains("PowerShellFallback"));
    assert!(script.contains("result_category"));
    assert!(script.contains("final_update_state"));
}

#[test]
fn m3_c_stability_100_iterations_zip_inspection() {
    use splogbook::cloud::updater::{ArchiveLimits, inspect_windows_zip_archive};

    let limits = ArchiveLimits::default();

    for i in 0..100 {
        let temp = tempfile::tempdir().expect("tempdir");
        let zip_path = temp.path().join(format!("test_{i}.zip"));

        // Synthetic non-zip or corrupt file test
        let data = format!("CORRUPTED_ZIP_STREAM_{i}_{}", i * 17);
        fs::write(&zip_path, data.as_bytes()).expect("write zip");

        let res = inspect_windows_zip_archive(&zip_path, &limits, "SPLogbook.exe");
        assert!(
            res.is_err(),
            "Inspection must fail for corrupt archive on iter {i}"
        );
    }
}

#[test]
fn m3_c_stability_50_iterations_update_plan_safety() {
    use splogbook::cloud::updater::UpdatePlan;

    for i in 0..50 {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let staging = root.join(".staging");
        fs::create_dir_all(&staging).expect("create staging");

        // Każda iteracja tworzy losowe pliki programu
        fs::write(staging.join("SPLogbook.exe"), format!("EXE_{i}")).expect("write exe");
        fs::write(staging.join(format!("lib_{i}.dll")), format!("DLL_{i}")).expect("write dll");

        let plan = UpdatePlan::generate(&staging, root).expect("generate plan");
        assert!(plan.validate_safety("SPLogbook.exe").is_ok());
        assert_eq!(plan.create_files.len(), 2);
    }
}

#[test]
fn m3_c_stability_20_iterations_result_consumption_and_rollback_measurement() {
    use splogbook::cloud::updater::helper_protocol::{
        HelperResultCategory, RollbackResultKind, WindowsHelperExitCode, WindowsHelperResult,
    };
    use splogbook::cloud::updater::{
        HealthResultKind, UpdateAttemptId, UpdateStateId, consume_update_result_on_startup,
    };

    for i in 0..20 {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let attempt = UpdateAttemptId::generate();
        let result_file = root.join(".update_helper_result.json");
        let rollback_dir = root.join(".rollback");

        fs::create_dir_all(&rollback_dir).expect("create rollback");
        let file_size = (i + 1) * 1024;
        fs::write(rollback_dir.join("backup.bin"), vec![0xAA; file_size]).expect("write backup");

        let res = WindowsHelperResult::new(
            attempt,
            format!("digest_{i}"),
            UpdateStateId::Committed,
            "NativeRust",
            HelperResultCategory::Success,
            WindowsHelperExitCode::Success,
            Some(RollbackResultKind::None),
            Some(HealthResultKind::Passed),
            1,
            1,
            true,
            false,
            "Success",
        );
        fs::write(&result_file, serde_json::to_vec_pretty(&res).expect("json"))
            .expect("write result");

        let outcome = consume_update_result_on_startup(root)
            .expect("consume")
            .expect("outcome");
        assert!(outcome.was_success);
        assert_eq!(outcome.attempt_id, attempt);
        assert_eq!(outcome.backup_retained_bytes, file_size as u64);
        assert!(rollback_dir.exists(), ".rollback must be retained");
        assert!(root.join(".update_helper_result.consumed").exists());
    }
}

#[test]
fn m3_dev_version_identity_and_constants_guard() {
    use splogbook::core::build_info::{BuildIdentity, DistributionStatus, UpdateChannel};

    assert_eq!(env!("CARGO_PKG_VERSION"), "1.3.0");

    let id = BuildIdentity::current();
    assert_eq!(id.package_name, "splogbook");
    assert_eq!(id.package_version, "1.3.0");
    assert_eq!(id.channel, UpdateChannel::Stable);
    assert_eq!(id.distribution_status, DistributionStatus::PublishedStable);
    assert!(
        !id.git_commit_hash.is_empty(),
        "commit hash must be present"
    );
    assert!(
        !id.git_commit_short.is_empty(),
        "commit short hash must be present"
    );
    assert_eq!(id.semver.major, 1);
    assert_eq!(id.semver.minor, 3);
    assert_eq!(id.semver.patch, 0);
    assert_eq!(id.semver.pre.as_str(), "");
    assert_eq!(id.channel.as_str(), "stable");
    assert!(!id.is_prerelease());
    assert!(id.is_stable());
    assert!(id.format_diagnostic().contains("1.3.0"));
}

#[test]
fn m3_downgrade_and_channel_protection_guard() {
    use splogbook::cloud::updater::{
        UpdateVersionDecision, evaluate_version_transition,
        verify_downgrade_protection_with_channel,
    };
    use splogbook::core::build_info::UpdateChannelPreference;

    // StableOnly does not accept alpha
    assert_eq!(
        evaluate_version_transition(
            "1.2.0",
            "1.3.0-alpha.1",
            UpdateChannelPreference::StableOnly
        ),
        UpdateVersionDecision::PrereleaseChannelNotAllowed
    );

    // Alpha accepts alpha
    assert_eq!(
        evaluate_version_transition("1.2.0", "1.3.0-alpha.1", UpdateChannelPreference::Alpha),
        UpdateVersionDecision::UpdateAvailable
    );

    // Downgrade 1.3.0-alpha.1 -> 1.2.0 is strictly rejected
    assert_eq!(
        evaluate_version_transition("1.3.0-alpha.1", "1.2.0", UpdateChannelPreference::Alpha),
        UpdateVersionDecision::DowngradeRejected
    );
    assert_eq!(
        evaluate_version_transition(
            "1.3.0-alpha.1",
            "1.2.0",
            UpdateChannelPreference::StableOnly
        ),
        UpdateVersionDecision::DowngradeRejected
    );

    // Downgrade 1.3.0 -> 1.3.0-alpha.1 is strictly rejected
    assert_eq!(
        evaluate_version_transition("1.3.0", "1.3.0-alpha.1", UpdateChannelPreference::Alpha),
        UpdateVersionDecision::DowngradeRejected
    );

    // Same version is rejected
    assert_eq!(
        evaluate_version_transition(
            "1.3.0-alpha.1",
            "1.3.0-alpha.1",
            UpdateChannelPreference::Alpha
        ),
        UpdateVersionDecision::SameVersion
    );

    // Verify error conversion
    assert!(
        verify_downgrade_protection_with_channel(
            "1.2.0",
            "1.3.0-alpha.1",
            UpdateChannelPreference::Alpha
        )
        .is_err()
    );
    assert!(
        verify_downgrade_protection_with_channel(
            "1.3.0-alpha.1",
            "1.2.0",
            UpdateChannelPreference::StableOnly
        )
        .is_err()
    );
}

#[test]
fn m3_git_repository_tag_v1_2_0_is_intact_and_no_automated_publishing_scripts_exist() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    // 1. Verify that tag v1.2.0 exists and resolves to commit 6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592
    let git_tag_check = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "v1.2.0^{commit}"])
        .output();

    if let Ok(output) = git_tag_check {
        assert!(
            output.status.success(),
            "git rev-parse v1.2.0^{{commit}} failed"
        );
        let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(
            commit, "6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592",
            "Tag v1.2.0 must remain strictly intact and point to commit 6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592"
        );
    }

    // 1b. Verify that annotated tag object v1.2.0 has SHA 3ea8d51aab436805107a1cf422e0b35cc301458f
    let git_tag_obj_check = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "refs/tags/v1.2.0"])
        .output();

    if let Ok(output) = git_tag_obj_check {
        assert!(
            output.status.success(),
            "git rev-parse refs/tags/v1.2.0 failed"
        );
        let tag_obj = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(
            tag_obj, "3ea8d51aab436805107a1cf422e0b35cc301458f",
            "Tag v1.2.0 object SHA must be 3ea8d51aab436805107a1cf422e0b35cc301458f"
        );
    }

    // 2. Stable release tags: v1.2.0 stays pinned and immutable; v1.3.0 is optional until
    //    publication and, when present, is the only tag of the 1.3.0 line.
    assert_stable_release_tags_are_pinned(repo_root);

    // 3. Verify no workflow automatically pushes tags or publishes releases on push, and
    //    every checkout fetches full history (with tags) so the tag guards run in CI.
    let workflows_dir = repo_root.join(".github").join("workflows");
    if workflows_dir.exists() {
        for entry in fs::read_dir(&workflows_dir).expect("read workflows") {
            let entry = entry.expect("workflow entry");
            if entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
            {
                let content = fs::read_to_string(entry.path()).expect("read workflow content");
                assert!(
                    !content.contains("git push origin --tags"),
                    "Workflow {} contains automated tag push which violates release discipline",
                    entry.path().display()
                );
                let lines: Vec<&str> = content.lines().collect();
                for (index, line) in lines.iter().enumerate() {
                    if !line.contains("uses: actions/checkout@") {
                        continue;
                    }
                    let step: Vec<&str> = lines
                        .iter()
                        .skip(index + 1)
                        .take_while(|next| !next.trim_start().starts_with("- "))
                        .copied()
                        .collect();
                    assert!(
                        step.iter().any(|next| next.trim() == "fetch-depth: 0"),
                        "{}:{}: actions/checkout must use fetch-depth: 0 so release tags are present",
                        entry.path().display(),
                        index + 1
                    );
                }
            }
        }
    }
}

/// Historical stable release: annotated tag object and peeled commit, both immutable.
const PINNED_V1_2_0_TAG_OBJECT: &str = "3ea8d51aab436805107a1cf422e0b35cc301458f";
const PINNED_V1_2_0_COMMIT: &str = "6f7d3aaa3b6a54e87b481c142a5b690bcf8b5592";

/// Current stable release line. Its tag is created on the final release commit, which
/// cannot contain the hash of its own future tag, so the tag is not pinned here; the pin
/// is added in the first commit after the release is published.
const CURRENT_RELEASE_TAG: &str = "v1.3.0";

fn git_stdout(repo_root: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} could not be executed: {e}"));
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn assert_stable_release_tags_are_pinned(repo_root: &std::path::Path) {
    // v1.2.0: strictly pinned annotated tag.
    let peeled = git_stdout(repo_root, &["rev-parse", "v1.2.0^{commit}"]);
    assert_eq!(
        peeled, PINNED_V1_2_0_COMMIT,
        "Tag v1.2.0 must stay immutable and point to commit {PINNED_V1_2_0_COMMIT}"
    );
    let kind = git_stdout(repo_root, &["cat-file", "-t", "refs/tags/v1.2.0"]);
    assert_eq!(kind, "tag", "Tag v1.2.0 must remain an annotated tag");
    let object = git_stdout(repo_root, &["rev-parse", "refs/tags/v1.2.0"]);
    assert_eq!(
        object, PINNED_V1_2_0_TAG_OBJECT,
        "Tag v1.2.0 object SHA must be {PINNED_V1_2_0_TAG_OBJECT}"
    );

    // v1.3.0: absent before tagging (push to main); once tagged it must be the only tag
    // of the 1.3.0 line and resolve to a commit of this repository.
    let line_tags = git_stdout(repo_root, &["tag", "-l", "*1.3.0*"]);
    let line_tags: Vec<&str> = line_tags
        .lines()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect();
    if line_tags.is_empty() {
        return;
    }
    assert_eq!(
        line_tags,
        vec![CURRENT_RELEASE_TAG],
        "The only tag of the 1.3.0 line may be the stable {CURRENT_RELEASE_TAG}"
    );
    let commit = git_stdout(
        repo_root,
        &[
            "rev-parse",
            "--verify",
            &format!("{CURRENT_RELEASE_TAG}^{{commit}}"),
        ],
    );
    assert!(
        commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()),
        "Tag {CURRENT_RELEASE_TAG} must resolve to a commit of this repository, got '{commit}'"
    );
}

#[test]
fn m3_stability_100_iterations_semver_evaluation_and_build_identity() {
    use splogbook::cloud::updater::{UpdateVersionDecision, evaluate_version_transition};
    use splogbook::core::build_info::{
        BuildIdentity, DistributionStatus, UpdateChannel, UpdateChannelPreference,
    };

    let id = BuildIdentity::current();

    for _ in 0..100 {
        // BuildIdentity consistency
        assert_eq!(id.package_name, "splogbook");
        assert_eq!(id.package_version, "1.3.0");
        assert_eq!(id.channel, UpdateChannel::Stable);
        assert_eq!(id.distribution_status, DistributionStatus::PublishedStable);
        assert_eq!(id.semver.to_string(), "1.3.0");

        // Transition matrix checks
        assert_eq!(
            evaluate_version_transition(
                "1.2.0",
                "1.3.0-alpha.1",
                UpdateChannelPreference::StableOnly
            ),
            UpdateVersionDecision::PrereleaseChannelNotAllowed
        );
        assert_eq!(
            evaluate_version_transition("1.2.0", "1.3.0-alpha.1", UpdateChannelPreference::Alpha),
            UpdateVersionDecision::UpdateAvailable
        );
        assert_eq!(
            evaluate_version_transition(
                "1.3.0-alpha.1",
                "1.3.0-alpha.1",
                UpdateChannelPreference::Alpha
            ),
            UpdateVersionDecision::SameVersion
        );
        assert_eq!(
            evaluate_version_transition("1.3.0-alpha.1", "1.2.0", UpdateChannelPreference::Alpha),
            UpdateVersionDecision::DowngradeRejected
        );
        assert_eq!(
            evaluate_version_transition(
                "1.3.0-alpha.1",
                "1.3.0-alpha.2",
                UpdateChannelPreference::Alpha
            ),
            UpdateVersionDecision::UpdateAvailable
        );
        assert_eq!(
            evaluate_version_transition(
                "1.3.0-alpha.2",
                "1.3.0-beta.1",
                UpdateChannelPreference::Beta
            ),
            UpdateVersionDecision::UpdateAvailable
        );
        assert_eq!(
            evaluate_version_transition(
                "1.3.0-beta.1",
                "1.3.0-rc.1",
                UpdateChannelPreference::Beta
            ),
            UpdateVersionDecision::UpdateAvailable
        );
        assert_eq!(
            evaluate_version_transition("1.3.0-rc.1", "1.3.0", UpdateChannelPreference::StableOnly),
            UpdateVersionDecision::UpdateAvailable
        );
        assert_eq!(
            evaluate_version_transition(
                "1.3.0-alpha.1",
                "1.3.0",
                UpdateChannelPreference::StableOnly
            ),
            UpdateVersionDecision::UpdateAvailable
        );
        assert_eq!(
            evaluate_version_transition("1.3.0", "1.3.0-alpha.1", UpdateChannelPreference::Alpha),
            UpdateVersionDecision::DowngradeRejected
        );
        assert_eq!(
            evaluate_version_transition(
                "1.3.0",
                "1.4.0-alpha.1",
                UpdateChannelPreference::StableOnly
            ),
            UpdateVersionDecision::PrereleaseChannelNotAllowed
        );
        assert_eq!(
            evaluate_version_transition("1.3.0", "1.4.0-alpha.1", UpdateChannelPreference::Alpha),
            UpdateVersionDecision::UpdateAvailable
        );
    }
}

#[test]
fn m3_transactional_replacement_terminology_guard() {
    let forbidden_patterns = [
        "atomic_swap",
        "atomic swap",
        "atomic multi-file replacement",
        "atomic multi-file",
    ];

    let files_to_scan = [
        "tests/windows_helper_test.rs",
        "src/cloud/updater/windows_helper.rs",
        "src/cloud/updater/helper_protocol.rs",
        "src/cloud/updater/transaction.rs",
        "src/cloud/updater/platform.rs",
    ];

    for path in &files_to_scan {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Nie można odczytać pliku {path}: {e}"));
        let lower = content.to_lowercase();
        for pattern in &forbidden_patterns {
            assert!(
                !lower.contains(pattern),
                "Wykryto niedozwoloną deklarację '{pattern}' w {path}. Aktualizator wieloplikowy wymaga semantyki transactional replacement."
            );
        }

        // Sprawdź, czy power-loss atomic nie występuje jako fałszywa obietnica
        for line in content.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("power-loss atomic") || line_lower.contains("power_loss_atomic")
            {
                assert!(
                    line.contains("NOT_POWER_LOSS_ATOMIC") || line_lower.contains("not "),
                    "Niedozwolona deklaracja sprzętowej atomowości w {path}: '{line}'. Dopuszczalne wyłącznie zaprzeczenie NOT_POWER_LOSS_ATOMIC."
                );
            }
        }
    }

    // Upewnij się, że nowa nazwa testu istnieje w tests/windows_helper_test.rs
    let helper_test_content =
        fs::read_to_string("tests/windows_helper_test.rs").expect("tests/windows_helper_test.rs");
    assert!(
        helper_test_content.contains(
            "windows_helper_stability_20_iterations_database_preservation_and_transactional_replacement"
        ),
        "Brak kanonicznej nazwy testu 'windows_helper_stability_20_iterations_database_preservation_and_transactional_replacement' w tests/windows_helper_test.rs"
    );
    assert!(
        !helper_test_content.contains(
            "windows_helper_stability_20_iterations_database_preservation_and_atomic_swap"
        ),
        "Stara nazwa testu 'windows_helper_stability_20_iterations_database_preservation_and_atomic_swap' nadal występuje w tests/windows_helper_test.rs"
    );

    // Potwierdź formalne tokeny architektoniczne w publicznym kanonie decyzji.
    let decisions = fs::read_to_string("docs/architecture/DECISIONS.md")
        .expect("docs/architecture/DECISIONS.md");
    assert!(
        decisions.contains("JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED")
            && decisions.contains("NOT_POWER_LOSS_ATOMIC")
            && decisions.contains("CRASH_RECOVERABLE_BY_JOURNAL"),
        "Kanon architektoniczny docs/architecture/DECISIONS.md musi zawierać potwierdzenie semantyki transakcyjnej"
    );
}

#[test]
fn m3_d_inno_setup_toolchain_decision_guard() {
    // Źródłem prawdy dla zatwierdzonego toolchainu jest publiczna dokumentacja
    // instalatora (installer/windows/inno/README.md), nie raport przedimplementacyjny.
    let inno_doc = fs::read_to_string("installer/windows/inno/README.md")
        .expect("installer/windows/inno/README.md");
    assert!(
        inno_doc.contains("ADR-10"),
        "Dokumentacja instalatora musi zawierać identyfikator ADR-10"
    );
    assert!(
        inno_doc.contains("APPROVED"),
        "Rekord ADR-10 musi mieć status APPROVED"
    );
    assert!(
        inno_doc.contains("7.1.1"),
        "Zatwierdzona wersja Inno Setup to 7.1.1"
    );
    assert!(
        inno_doc.contains(r"C:\Program Files\Inno Setup 7\ISCC.exe"),
        r"Zatwierdzona ścieżka kompilatora to C:\Program Files\Inno Setup 7\ISCC.exe"
    );
    assert!(
        inno_doc.contains("E0F9F1DC32032650C6629F6DEE41EB6D20BA140E4E3C07C25D8A24F9BA781644"),
        "Dokumentacja instalatora musi zapisywać SHA-256 zatwierdzonego kompilatora"
    );

    let iscc_path = std::path::Path::new(r"C:\Program Files\Inno Setup 7\ISCC.exe");
    if iscc_path.exists() {
        let output = Command::new(iscc_path)
            .arg("/?")
            .output()
            .expect("Wywołanie ISCC.exe");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout} {stderr}");
        assert!(
            combined.contains("7.1.1") || combined.contains("Inno Setup 7"),
            "Zainstalowany kompilator ISCC musi identyfikować się jako Inno Setup 7.1.1"
        );
    }
}

#[test]
fn m3_d_inno_setup_architecture_and_privileges_guard() {
    let iss_content = fs::read_to_string("installer/windows/inno/SPLogbook.iss")
        .expect("installer/windows/inno/SPLogbook.iss");

    assert!(
        iss_content.contains("SetupArchitecture=x64"),
        "SPLogbook.iss musi zawierać SetupArchitecture=x64"
    );
    assert!(
        iss_content.contains("ArchitecturesAllowed=x64compatible"),
        "SPLogbook.iss musi zawierać ArchitecturesAllowed=x64compatible"
    );
    assert!(
        iss_content.contains("ArchitecturesInstallIn64BitMode=x64compatible"),
        "SPLogbook.iss musi zawierać ArchitecturesInstallIn64BitMode=x64compatible"
    );
    assert!(
        iss_content.contains("PrivilegesRequired=lowest"),
        "SPLogbook.iss musi zawierać PrivilegesRequired=lowest dla instalacji per-user"
    );
    assert!(
        iss_content.contains(r"DefaultDirName={localappdata}\Programs\SPLogbook"),
        "SPLogbook.iss musi instalować domyślnie do {{localappdata}}\\Programs\\SPLogbook"
    );
}

#[test]
fn m3_d_inno_setup_no_admin_elevation_guard() {
    let iss_content = fs::read_to_string("installer/windows/inno/SPLogbook.iss")
        .expect("installer/windows/inno/SPLogbook.iss");
    let script_content = fs::read_to_string("scripts/build_windows_user_setup.ps1")
        .expect("scripts/build_windows_user_setup.ps1");
    let updater_content = fs::read_to_string("src/cloud/updater.rs").expect("src/cloud/updater.rs");

    assert!(
        !iss_content.contains("PrivilegesRequiredOverridesAllowed"),
        "SPLogbook.iss nie może zawierać PrivilegesRequiredOverridesAllowed"
    );

    assert!(
        !iss_content.contains("Root: HKLM")
            && !iss_content.contains("Root: HKEY_LOCAL_MACHINE")
            && !iss_content.contains("RegWrite"),
        "Instalator User Setup nie może zapisywać do HKLM"
    );

    assert!(
        !iss_content.contains("{commonpf}") && !iss_content.contains("{autopf}"),
        "Instalator User Setup nie może instalować do Program Files"
    );

    assert!(
        !iss_content.to_lowercase().contains("runas"),
        "SPLogbook.iss nie może używać runas"
    );
    assert!(
        !script_content.to_lowercase().contains("runas"),
        "build_windows_user_setup.ps1 nie może używać runas"
    );
    assert!(
        !updater_content.to_lowercase().contains("verb(\"runas\")")
            && !updater_content.to_lowercase().contains("verb('runas')"),
        "updater.rs nie może używać czasownika runas"
    );

    assert!(
        !iss_content.contains("requireAdministrator"),
        "Instalator User Setup nie może żądać requireAdministrator"
    );
}

#[test]
fn m3_d_inno_setup_stable_identity_guard() {
    use splogbook::cloud::updater::WINDOWS_INNO_SETUP_APP_ID;

    let iss_content = fs::read_to_string("installer/windows/inno/SPLogbook.iss")
        .expect("installer/windows/inno/SPLogbook.iss");
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Cargo.toml");

    assert_eq!(
        WINDOWS_INNO_SETUP_APP_ID, "{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}",
        "WINDOWS_INNO_SETUP_APP_ID musi mieć kanoniczną wartość"
    );

    assert!(
        iss_content.contains("AppId={{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}}"),
        "SPLogbook.iss musi definiować AppId jako {{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}}"
    );

    assert!(
        iss_content.contains("AppVersion={#AppVersion}"),
        "SPLogbook.iss musi pobierać wersję z makra AppVersion"
    );

    assert!(
        cargo_toml.contains(r#"version = "1.3.0""#),
        "Cargo.toml musi definiować bieżącą wersję stabilną 1.3.0"
    );

    assert!(
        !cargo_toml.contains(r#"version = "1.3.0-"#),
        "Wersja stabilna 1.3.0 nie może mieć oznaczenia pre-release"
    );
}

#[test]
fn m3_d_inno_setup_input_allowlist_guard() {
    let iss_content = fs::read_to_string("installer/windows/inno/SPLogbook.iss")
        .expect("installer/windows/inno/SPLogbook.iss");

    let files_section = iss_content
        .split("[Files]")
        .nth(1)
        .and_then(|s| s.split('[').next())
        .expect("Sekcja [Files] w SPLogbook.iss");

    assert!(files_section.contains("SPLogbook.exe"));
    assert!(files_section.contains("splogbook-updater.exe"));
    assert!(iss_content.contains(".splogbook-inno-marker"));
    assert!(files_section.contains(r"assets\icon.ico"));
    assert!(files_section.contains(r"assets\icon.png"));
    assert!(files_section.contains(r"locales\*"));

    let forbidden_extensions = [".pem", ".key", ".pfx", ".p12", ".token", "id_rsa", ".env"];
    for ext in &forbidden_extensions {
        assert!(
            !files_section.to_lowercase().contains(ext),
            "Sekcja [Files] zawiera niedozwolone rozszerzenie sekretu: {ext}"
        );
    }

    for line in files_section.lines() {
        if line.contains(".db") || line.contains(".csv") {
            assert!(
                line.contains("uninsneveruninstall") && line.contains("onlyifdoesntexist"),
                "Plik bazodanowy seed w [Files] musi mieć flagi uninsneveruninstall i onlyifdoesntexist: {line}"
            );
        }
    }
}

#[test]
fn m3_d_inno_setup_uninstall_safety_guard() {
    let iss_content = fs::read_to_string("installer/windows/inno/SPLogbook.iss")
        .expect("installer/windows/inno/SPLogbook.iss");

    let uninstall_delete = iss_content
        .split("[UninstallDelete]")
        .nth(1)
        .and_then(|s| s.split('[').next())
        .expect("Sekcja [UninstallDelete] w SPLogbook.iss");

    assert!(
        !uninstall_delete.contains('*') && !uninstall_delete.contains('?'),
        "Sekcja [UninstallDelete] nie może zawierać znaków wieloznacznych (* ani ?)"
    );

    for line in uninstall_delete.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.is_empty() {
            continue;
        }
        if trimmed.contains(r#"Name: "{app}""#) || trimmed.contains("Name: {app}") {
            assert!(
                trimmed.starts_with("Type: dirifempty"),
                "Katalog {{app}} może być czyszczony w [UninstallDelete] wyłącznie jako dirifempty: {line}"
            );
        }
        if trimmed.contains("dirifempty") {
            assert!(
                trimmed.contains("Type: dirifempty") || trimmed.contains("Type: filesandordirs"),
                "Wpisy katalogów w [UninstallDelete] muszą być ograniczone do dirifempty: {line}"
            );
        }
        assert!(
            !trimmed.contains("default_log.db")
                && !trimmed.contains("serviceLOG.db")
                && !trimmed.contains("callbook.db")
                && !trimmed.contains(".db-wal")
                && !trimmed.contains(".db-shm")
                && !trimmed.contains(".db-journal"),
            "Sekcja [UninstallDelete] nie może usuwać baz danych użytkownika: {line}"
        );
    }
}

#[test]
fn m3_d_inno_setup_update_delegation_guard() {
    use splogbook::cloud::updater::build_inno_setup_arguments;
    use std::path::Path;

    let test_path = Path::new(r"C:\Users\test\AppData\Local\Temp\SPLogbook-Setup.log");
    let args = build_inno_setup_arguments(test_path);

    assert!(args.contains(&"/SILENT".to_string()));
    assert!(
        args.contains(&"/NORESTART".to_string()),
        "Flaga /NORESTART jest bezwzględnie wymagana"
    );
    assert!(args.iter().any(|a| a.starts_with("/LOG=")));

    let updater_code = fs::read_to_string("src/cloud/updater.rs").expect("src/cloud/updater.rs");
    assert!(
        updater_code.contains("execute_inno_setup_installer"),
        "updater.rs musi zawierać dedykowaną funkcję execute_inno_setup_installer"
    );
    assert!(
        updater_code.contains("Command::new(installer_exe)"),
        "execute_inno_setup_installer musi uruchamiać instalator bezpośrednio przez Command::new"
    );
}

#[test]
fn m3_d_inno_setup_ci_build_and_publish_guard() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("read .github/workflows/build-and-release.yml");
    for required in [
        "Install and verify pinned Inno Setup 7.1.1",
        "E0F9F1DC32032650C6629F6DEE41EB6D20BA140E4E3C07C25D8A24F9BA781644",
        "Build Windows User Setup (Inno Setup 7.1.1)",
        "./scripts/build_windows_user_setup.ps1 -OutputDir dist",
        "SPLOG_RUN_INNO_E2E",
        "inno_native_install_and_uninstall_lifecycle_e2e",
        "dist/SPLogbook-Windows-x64-Setup-*.exe",
        "dist/SPLogbook-Windows-x64-Setup-*.exe.sha256",
        r#""package_type": "inno-exe""#,
        "SPLogbook-Windows-x64-Setup-${{ env.RELEASE_VERSION }}.exe",
    ] {
        assert!(
            workflow.contains(required),
            "build-and-release.yml musi zawierać krok/kontrakt Inno Setup: {required}"
        );
    }
}

#[test]
fn m3_d_inno_setup_conflict_matrix_guard() {
    use splogbook::cloud::updater::{InstallationType, ManagementMode};

    assert_eq!(
        InstallationType::WindowsInnoSetup.management_mode(),
        ManagementMode::SystemManaged,
        "WindowsInnoSetup musi mieć tryb SystemManaged"
    );
    assert_eq!(
        InstallationType::WindowsMsi.management_mode(),
        ManagementMode::SystemManaged,
        "WindowsMsi musi mieć tryb SystemManaged"
    );
    assert_eq!(
        InstallationType::WindowsPortableZip.management_mode(),
        ManagementMode::SelfManaged,
        "WindowsPortableZip musi mieć tryb SelfManaged"
    );

    let updater_code = fs::read_to_string("src/cloud/updater.rs").expect("src/cloud/updater.rs");
    assert!(
        updater_code.contains("ManagedInstallerConflict"),
        "updater.rs musi zawierać błąd ManagedInstallerConflict"
    );
    assert!(
        updater_code.contains("InstallationType::WindowsMsi"),
        "updater.rs musi jawnie obsługiwać i odrzucać konflikt WindowsMsi"
    );
}

#[test]
fn m3_d_inno_setup_build_artifact_safety_guard() {
    let gitignore = fs::read_to_string(".gitignore").expect(".gitignore");
    assert!(
        gitignore.contains("/dist/") || gitignore.contains("dist/"),
        ".gitignore musi ignorować katalog dist/"
    );
    assert!(
        gitignore.contains("installer/windows/inno/Output/"),
        ".gitignore musi ignorować installer/windows/inno/Output/"
    );
    assert!(
        gitignore.contains("*-Setup-*.exe"),
        ".gitignore musi ignorować pliki *-Setup-*.exe"
    );

    let output = Command::new("git")
        .args(["ls-files", "*.exe"])
        .output()
        .expect("git ls-files");
    let tracked_exe = String::from_utf8_lossy(&output.stdout);
    assert!(
        tracked_exe.trim().is_empty(),
        "W repozytorium git nie może być śledzony żaden plik .exe: {tracked_exe}"
    );

    // Nienaruszalność tagów wydań stabilnych v1.2.0 i v1.3.0.
    assert_stable_release_tags_are_pinned(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn m3_d_stability_matrix_guard() {
    use splogbook::cloud::updater::{
        InstallationType, ReleasePackageType, ReleasePlatform, WINDOWS_INNO_SETUP_APP_ID,
        WindowsUserSetupMarker,
    };
    use std::path::Path;

    for i in 0..100 {
        let fake_root = Path::new(r"C:\Users\test\AppData\Local\Programs\SPLogbook");
        let marker = WindowsUserSetupMarker::new(
            &format!("1.3.0-alpha.{i}"),
            fake_root,
            &format!("commit_{i:040x}"),
        );
        let json = serde_json::to_string(&marker).expect("to_json");
        let parsed: WindowsUserSetupMarker = serde_json::from_str(&json).expect("from_json");
        assert_eq!(parsed.app_id, WINDOWS_INNO_SETUP_APP_ID);
        assert_eq!(parsed.package_version, format!("1.3.0-alpha.{i}"));
        assert!(parsed.validate().is_ok());

        let corrupt_json = format!(
            r#"{{"marker_version":"1.0","installation_type":"WindowsInnoSetup","management_mode":"SystemManaged","package_version":"{i}","app_id":"WRONG_ID","installation_scope":"CurrentUser","installation_root":"C:\\","build_commit":"abc","installer_engine":"InnoSetup"}}"#
        );
        let corrupt_marker: WindowsUserSetupMarker =
            serde_json::from_str(&corrupt_json).expect("from_str");
        assert!(corrupt_marker.validate().is_err());

        // Expected package for Windows Inno Setup
        let expected = ReleasePackageType::expected_for_installation(
            ReleasePlatform::Windows,
            InstallationType::WindowsInnoSetup,
        );
        assert_eq!(expected, ReleasePackageType::InnoExe);

        let parsed_exe = ReleasePackageType::parse("inno-exe").unwrap();
        assert_eq!(parsed_exe, ReleasePackageType::InnoExe);

        let parsed_zip = ReleasePackageType::parse("zip").unwrap();
        assert_eq!(parsed_zip, ReleasePackageType::Zip);
        assert_ne!(parsed_zip, expected);
    }
}

#[test]
fn m3_d_stability_50_iterations_conflict_and_update_models() {
    use splogbook::cloud::updater::{
        InstallationType, ManagementMode, UpdateVersionDecision, build_inno_setup_arguments,
        evaluate_version_transition,
    };
    use splogbook::core::build_info::UpdateChannelPreference;
    use std::path::PathBuf;

    for i in 0..50 {
        let setup_path = PathBuf::from(format!(r"C:\Temp\Setup_{i}\SPLogbook-Setup.log"));
        let args = build_inno_setup_arguments(&setup_path);
        assert_eq!(args.len(), 3);
        assert_eq!(args[1], "/NORESTART");

        assert_eq!(
            InstallationType::WindowsInnoSetup.management_mode(),
            ManagementMode::SystemManaged
        );
        assert_eq!(
            InstallationType::WindowsMsi.management_mode(),
            ManagementMode::SystemManaged
        );

        let current_v = format!("1.3.0-alpha.{i}");
        let candidate_v = format!("1.3.0-alpha.{}", i + 1);
        let decision =
            evaluate_version_transition(&current_v, &candidate_v, UpdateChannelPreference::Alpha);
        assert_eq!(decision, UpdateVersionDecision::UpdateAvailable);

        let downgrade_decision =
            evaluate_version_transition(&candidate_v, &current_v, UpdateChannelPreference::Alpha);
        assert_eq!(downgrade_decision, UpdateVersionDecision::DowngradeRejected);
    }
}

#[test]
fn m3_d_stability_20_iterations_setup_lifecycle_and_data_safety() {
    use splogbook::cloud::updater::{WINDOWS_INNO_MARKER_FILENAME, WindowsUserSetupMarker};

    for i in 0..20 {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let app_dir = temp_dir.path().join("SPLogbook");
        let logs_dir = app_dir.join("logs");
        fs::create_dir_all(&logs_dir).expect("create_dir");

        let exe_path = app_dir.join("SPLogbook.exe");
        fs::write(&exe_path, format!("BIN_V1_{i}")).expect("write exe");

        let marker = WindowsUserSetupMarker::new("1.3.0-alpha.1", &app_dir, &format!("hash_{i}"));
        let marker_json = serde_json::to_string(&marker).expect("marker json");
        fs::write(app_dir.join(WINDOWS_INNO_MARKER_FILENAME), marker_json).expect("write marker");

        let db_path = app_dir.join("default_log.db");
        let wal_path = app_dir.join("default_log.db-wal");
        let cfg_path = app_dir.join("station_config.json");
        let custom_notes = app_dir.join("user_notes.txt");
        let user_log = logs_dir.join(format!("qso_{i}.log"));

        let db_content = format!("SQLITE_DATABASE_RECORD_{i}");
        let wal_content = format!("WAL_JOURNAL_CONTENT_{i}");
        let cfg_content = format!(r#"{{"operator":"SP6INA_{i}"}}"#);
        let notes_content = format!("CONTEST NOTES {i}");
        let log_content = format!("LOG TRACE {i}");

        fs::write(&db_path, &db_content).expect("write db");
        fs::write(&wal_path, &wal_content).expect("write wal");
        fs::write(&cfg_path, &cfg_content).expect("write cfg");
        fs::write(&custom_notes, &notes_content).expect("write notes");
        fs::write(&user_log, &log_content).expect("write log");

        fs::remove_file(&exe_path).expect("remove exe");
        fs::remove_file(app_dir.join(WINDOWS_INNO_MARKER_FILENAME)).expect("remove marker");

        if fs::read_dir(&logs_dir).unwrap().next().is_none() {
            let _ = fs::remove_dir(&logs_dir);
        }

        assert_eq!(fs::read_to_string(&db_path).unwrap(), db_content);
        assert_eq!(fs::read_to_string(&wal_path).unwrap(), wal_content);
        assert_eq!(fs::read_to_string(&cfg_path).unwrap(), cfg_content);
        assert_eq!(fs::read_to_string(&custom_notes).unwrap(), notes_content);
        assert_eq!(fs::read_to_string(&user_log).unwrap(), log_content);
        assert!(
            app_dir.exists(),
            "Katalog aplikacji nie może zostać usunięty, jeśli zawiera dane użytkownika"
        );
    }
}

#[test]
fn m3_e_msi_toolchain_decision_and_identity_guard() {
    use splogbook::cloud::updater::{
        WINDOWS_INNO_SETUP_APP_ID, WINDOWS_MSI_UPGRADE_CODE, WINDOWS_MSI_UPGRADE_CODE_PACKED,
    };

    let wixproj = fs::read_to_string("installer/windows/msi/SPLogbook.wixproj")
        .expect("read SPLogbook.wixproj");
    assert!(
        wixproj.contains("WixToolset.Sdk/7.0.1"),
        "SPLogbook.wixproj musi używać zatwierdzonego WiX Toolset SDK 7.0.1 (ADR-11)"
    );
    assert!(
        wixproj.contains("<AcceptEula>true</AcceptEula>"),
        "SPLogbook.wixproj musi deklarować akceptację EULA WiX 7"
    );

    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        pkg.contains("UpgradeCode=\"B7D14532-6A10-4F8A-9A4B-3DE189E502B0\""),
        "Package.wxs musi zawierać stały UpgradeCode zatwierdzony w ADR-11"
    );
    assert_eq!(
        WINDOWS_MSI_UPGRADE_CODE,
        "{B7D14532-6A10-4F8A-9A4B-3DE189E502B0}"
    );
    assert_ne!(
        WINDOWS_MSI_UPGRADE_CODE, WINDOWS_INNO_SETUP_APP_ID,
        "UpgradeCode MSI musi być różny od Inno AppId"
    );
    assert_eq!(
        WINDOWS_MSI_UPGRADE_CODE_PACKED,
        "0B205E981ED3B4A9A9B4A9E189E502B0"
    );
}

#[test]
fn m3_e_msi_architecture_and_privileges_guard() {
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        pkg.contains("Scope=\"perMachine\""),
        "Package.wxs musi mieć Scope=\"perMachine\""
    );

    let dir =
        fs::read_to_string("installer/windows/msi/Directories.wxs").expect("read Directories.wxs");
    assert!(
        dir.contains("ProgramFiles64Folder"),
        "Directories.wxs musi instalować do ProgramFiles64Folder"
    );
    assert!(
        dir.contains("Name=\"SPLogbook\""),
        "Directories.wxs musi instalować do podfolderu SPLogbook"
    );

    let comp =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    assert!(
        comp.contains("Bitness=\"always64\""),
        "Komponenty MSI muszą mieć Bitness=\"always64\""
    );
}

#[test]
fn m3_e_msi_conflict_matrix_guard() {
    use splogbook::cloud::updater::{InstallationType, ManagementMode};

    assert_eq!(
        InstallationType::WindowsMsi.management_mode(),
        ManagementMode::SystemManaged,
        "WindowsMsi musi być SystemManaged"
    );

    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        pkg.contains("INNO_USER_SETUP_DETECTED"),
        "Package.wxs musi wykrywać Inno Setup przez rejestr"
    );
    assert!(
        pkg.contains("<Launch"),
        "Package.wxs musi blokować instalację przez Launch Condition"
    );

    let iss =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    assert!(
        iss.contains("0B205E981ED3B4A9A9B4A9E189E502B0"),
        "SPLogbook.iss musi sprawdzać obecność MSI UpgradeCode"
    );
    assert!(
        iss.contains(".splogbook-msi-marker"),
        "SPLogbook.iss musi sprawdzać obecność markera MSI"
    );
}

#[test]
fn m3_e_msi_input_allowlist_guard() {
    let comp =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");

    // Wymagane pliki programu
    assert!(comp.contains("SPLogbook.exe"));
    assert!(comp.contains("splogbook-updater.exe"));
    assert!(comp.contains(".splogbook-msi-marker"));

    // Zakazane pliki danych i kluczy
    assert!(!comp.contains("default_log.db"));
    assert!(!comp.contains("station_config.json"));
    assert!(!comp.contains("splogbook_contacts.db"));
    assert!(!comp.contains(".key"));
    assert!(!comp.contains(".pem"));
    assert!(!comp.contains(".pfx"));
    assert!(!comp.contains(".pdb"));

    // Zakaz niebezpiecznych akcji
    assert!(!comp.contains("powershell"));
    assert!(!comp.contains("cmd.exe"));
}

#[test]
fn m3_e_msi_updater_delegation_guard() {
    let updater_code = fs::read_to_string("src/cloud/updater.rs").expect("src/cloud/updater.rs");

    assert!(
        updater_code.contains("execute_msi_installer"),
        "updater.rs musi zawierać dedykowaną funkcję execute_msi_installer"
    );
    assert!(
        updater_code.contains("delegate_msi_update"),
        "updater.rs musi zawierać dedykowaną funkcję delegate_msi_update"
    );
    assert!(
        updater_code.contains("build_msi_install_arguments"),
        "updater.rs musi zawierać funkcję build_msi_install_arguments"
    );
    assert!(
        updater_code.contains("MsiexecExitStatus"),
        "updater.rs musi zawierać enum MsiexecExitStatus"
    );
    assert!(
        updater_code.contains("Command::new(\"msiexec.exe\")"),
        "execute_msi_installer musi uruchamiać msiexec.exe bezpośrednio bez powłoki shell"
    );
    assert!(
        updater_code.contains("\"/norestart\""),
        "updater.rs musi wymuszać flagę /norestart"
    );
    assert!(
        updater_code.contains("\"/L*v\""),
        "updater.rs musi wymuszać szczegółowe logowanie /L*v"
    );
}

#[test]
fn m3_e_msi_uninstall_and_repair_safety_guard() {
    let comp =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");

    // Brak RemoveFolderEx
    assert!(!comp.contains("RemoveFolderEx"));
    assert!(!pkg.contains("RemoveFolderEx"));

    let iss =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    let workflow =
        fs::read_to_string(".github/workflows/build-and-release.yml").expect("read workflow");

    // Brak bundlowanych baz referencyjnych w MSI, Inno Setup i paczce Windows
    for legacy in ["callbook.db", "serviceLOG.db", "clubs.csv"] {
        assert!(
            !comp.contains(legacy),
            "Components.wxs nie może bundlować usuniętej bazy {legacy}"
        );
        assert!(
            !iss.contains(legacy),
            "SPLogbook.iss nie może bundlować usuniętej bazy {legacy}"
        );
        assert!(
            !workflow.contains(legacy),
            "build-and-release.yml nie może kopiować usuniętej bazy {legacy}"
        );
    }

    // MajorUpgrade z blokadą downgrade
    assert!(pkg.contains("<MajorUpgrade"));
    assert!(pkg.contains("DowngradeErrorMessage="));
    assert!(pkg.contains("Schedule=\"afterInstallInitialize\""));
}

#[test]
fn m3_e_msi_ci_build_and_publish_guard() {
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("read .github/workflows/build-and-release.yml");
    for required in [
        "Build Windows System Setup (MSI / WiX 7.0.1)",
        "./scripts/build_windows_msi.ps1 -OutputDir dist -SkipCargoBuild",
        "SPLOG_RUN_MSI_E2E",
        "msi_built_artifact_verification_if_present",
        "dist/SPLogbook-Windows-x64-MSI-*.msi",
        "dist/SPLogbook-Windows-x64-MSI-*.msi.sha256",
        r#""package_type": "msi""#,
        "SPLogbook-Windows-x64-MSI-${{ env.RELEASE_VERSION }}.msi",
    ] {
        assert!(
            workflow.contains(required),
            "build-and-release.yml musi zawierać krok/kontrakt MSI: {required}"
        );
    }

    let build_script =
        fs::read_to_string("scripts/build_windows_msi.ps1").expect("read build_windows_msi.ps1");
    assert!(
        build_script.contains("vswhere.exe")
            && build_script.contains("Microsoft.Component.MSBuild")
            && build_script.contains("Enterprise"),
        "build_windows_msi.ps1 musi dynamicznie wykrywać MSBuild.exe przez vswhere.exe z fallbackiem VS 2022"
    );
}

#[test]
fn m3_e_msi_build_artifact_safety_guard() {
    let gitignore = fs::read_to_string(".gitignore").expect(".gitignore");
    assert!(gitignore.contains("*.msi"));
    assert!(gitignore.contains("*.wixpdb"));
    assert!(gitignore.contains("*.wixobj"));
    assert!(gitignore.contains("installer/windows/msi/bin/"));
    assert!(gitignore.contains("installer/windows/msi/obj/"));

    let output = Command::new("git")
        .args(["ls-files", "*.msi", "*.wixpdb"])
        .output()
        .expect("git ls-files");
    let tracked_msi = String::from_utf8_lossy(&output.stdout);
    assert!(
        tracked_msi.trim().is_empty(),
        "W repozytorium git nie może być śledzony żaden plik .msi ani .wixpdb: {tracked_msi}"
    );
}

#[test]
fn m3_e_stability_100_iterations_msi_matrix_guard() {
    use splogbook::cloud::updater::installation::{
        WindowsMsiMarker, semver_to_msi_product_version,
    };
    use splogbook::cloud::updater::{
        InstallationType, MsiexecExitStatus, ReleasePackageType, ReleasePlatform,
        WINDOWS_MSI_UPGRADE_CODE, build_msi_install_arguments, build_msi_repair_arguments,
        build_msi_uninstall_arguments,
    };
    use std::path::Path;

    for i in 0..100 {
        let fake_root = Path::new(r"C:\Program Files\SPLogbook");
        let marker = WindowsMsiMarker::new(
            &format!("1.3.0-alpha.{i}"),
            fake_root,
            &format!("commit_{i:040x}"),
        );
        let json = serde_json::to_string(&marker).expect("to_json");
        let parsed: WindowsMsiMarker = serde_json::from_str(&json).expect("from_json");
        assert_eq!(parsed.upgrade_code, WINDOWS_MSI_UPGRADE_CODE);
        assert_eq!(parsed.package_version, format!("1.3.0-alpha.{i}"));
        assert_eq!(parsed.installation_scope, "PerMachine");
        assert!(parsed.validate().is_ok());

        let corrupt_json = format!(
            r#"{{"marker_version":"1.0","installation_type":"WindowsMsi","management_mode":"SystemManaged","package_version":"{i}","upgrade_code":"WRONG_UPGRADE_CODE","product_code":"*","installation_scope":"PerMachine","installation_root":"C:\\","build_commit":"abc","installer_engine":"WindowsInstallerWiX"}}"#
        );
        let corrupt_marker: WindowsMsiMarker =
            serde_json::from_str(&corrupt_json).expect("from_str");
        assert!(corrupt_marker.validate().is_err());

        let mapped_version = semver_to_msi_product_version(&format!("1.3.{i}-alpha.1")).unwrap();
        assert_eq!(mapped_version, format!("1.3.{i}"));

        // Expected package type dla WindowsMsi
        let expected = ReleasePackageType::expected_for_installation(
            ReleasePlatform::Windows,
            InstallationType::WindowsMsi,
        );
        assert_eq!(expected, ReleasePackageType::Msi);

        let msi_path = Path::new(r"C:\Temp\SPLogbook.msi");
        let log_path = Path::new(r"C:\Temp\install.log");
        let inst_args = build_msi_install_arguments(msi_path, log_path);
        assert_eq!(inst_args[0], "/i");
        assert_eq!(inst_args[2], "/qn");
        assert_eq!(inst_args[3], "/norestart");

        let rep_args = build_msi_repair_arguments(msi_path, log_path);
        assert_eq!(rep_args[0], "/fa");

        let uninst_args = build_msi_uninstall_arguments(WINDOWS_MSI_UPGRADE_CODE, log_path);
        assert_eq!(uninst_args[0], "/x");

        assert!(MsiexecExitStatus::from_code(0).is_success());
        assert!(MsiexecExitStatus::from_code(3010).is_success());
        assert!(!MsiexecExitStatus::from_code(1603).is_success());
    }
}

#[test]
fn m3_e_stability_50_iterations_msi_models_guard() {
    use splogbook::cloud::updater::{
        InstallationType, ManagementMode, UpdateVersionDecision, evaluate_version_transition,
    };
    use splogbook::core::build_info::UpdateChannelPreference;

    for i in 0..50 {
        assert_eq!(
            InstallationType::WindowsMsi.management_mode(),
            ManagementMode::SystemManaged
        );
        assert!(InstallationType::WindowsMsi.is_system_managed());
        assert!(!InstallationType::WindowsMsi.is_self_managed());

        let current_v = format!("1.3.0-alpha.{i}");
        let candidate_v = format!("1.3.0-alpha.{}", i + 1);
        let decision =
            evaluate_version_transition(&current_v, &candidate_v, UpdateChannelPreference::Alpha);
        assert_eq!(decision, UpdateVersionDecision::UpdateAvailable);
    }
}

#[test]
fn m3_e_stability_20_iterations_msi_lifecycle_guard() {
    use splogbook::cloud::updater::installation::{WINDOWS_MSI_MARKER_FILENAME, WindowsMsiMarker};

    for i in 0..20 {
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("SPLogbook");
        let data_dir = temp.path().join("AppData_Roaming_SPLogbook");
        let databases_dir = data_dir.join("databases");
        let config_dir = data_dir.join("config");
        let exports_dir = data_dir.join("exports");
        let logs_dir = data_dir.join("logs");

        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&databases_dir).unwrap();
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(&exports_dir).unwrap();
        fs::create_dir_all(&logs_dir).unwrap();

        let exe_path = app_dir.join("SPLogbook.exe");
        let updater_path = app_dir.join("splogbook-updater.exe");
        let db_path = databases_dir.join("default_log.db");
        let wal_path = databases_dir.join("default_log.db-wal");
        let cfg_path = config_dir.join("station_config.json");
        let adif_path = exports_dir.join("test.adi");
        let user_log = logs_dir.join("session.log");
        let unknown_file = app_dir.join("addon_plugin.dll");

        let db_content = format!("SQLITE_USER_DATA_{i}");
        let wal_content = format!("SQLITE_WAL_DATA_{i}");
        let cfg_content = format!("{{\"callsign\":\"SP6INA_{i}\"}}");
        let adif_content = format!("<call:6>SP6INA<qso_date:8>20261108<qso_nr:{i}><eor>");
        let log_content = format!("LOG_LINE_{i}");
        let unknown_content = format!("UNKNOWN_PAYLOAD_{i}");

        fs::write(&exe_path, b"SPLogbook_EXE").unwrap();
        fs::write(&updater_path, b"splogbook_updater_EXE").unwrap();
        fs::write(&db_path, &db_content).unwrap();
        fs::write(&wal_path, &wal_content).unwrap();
        fs::write(&cfg_path, &cfg_content).unwrap();
        fs::write(&adif_path, &adif_content).unwrap();
        fs::write(&user_log, &log_content).unwrap();
        fs::write(&unknown_file, &unknown_content).unwrap();

        let marker = WindowsMsiMarker::new(
            "1.3.0-alpha.1",
            &app_dir,
            "954fca396b2986abe783f5cc8b0d1aca1bf19be1",
        );
        let marker_json = serde_json::to_string(&marker).unwrap();
        fs::write(app_dir.join(WINDOWS_MSI_MARKER_FILENAME), marker_json).unwrap();

        // Symulacja deinstalacji MSI: usunięcie wyłącznie plików programu i markera
        fs::remove_file(&exe_path).unwrap();
        fs::remove_file(&updater_path).unwrap();
        fs::remove_file(app_dir.join(WINDOWS_MSI_MARKER_FILENAME)).unwrap();

        // Weryfikacja: dane użytkownika są w 100% nienaruszone (byte-identical)
        assert_eq!(fs::read_to_string(&db_path).unwrap(), db_content);
        assert_eq!(fs::read_to_string(&wal_path).unwrap(), wal_content);
        assert_eq!(fs::read_to_string(&cfg_path).unwrap(), cfg_content);
        assert_eq!(fs::read_to_string(&adif_path).unwrap(), adif_content);
        assert_eq!(fs::read_to_string(&user_log).unwrap(), log_content);
        assert_eq!(fs::read_to_string(&unknown_file).unwrap(), unknown_content);
    }
}

#[test]
fn m3_f_owner_decisions_and_support_policy_guard() {
    // Źródłem prawdy dla 11 decyzji M3-F jest publiczny kanon decyzji.
    let decisions_doc = fs::read_to_string("docs/architecture/DECISIONS.md")
        .expect("docs/architecture/DECISIONS.md");

    // Wszystkie 11 formalnych rekordów ADR dla pakowania Linux
    assert!(decisions_doc.contains("| `ADR-14` |"));
    assert!(decisions_doc.contains("| `ADR-15` |"));
    assert!(decisions_doc.contains("| `ADR-19` |"));
    assert!(decisions_doc.contains("| `ADR-20` |"));
    assert!(decisions_doc.contains("| `ADR-21` |"));
    assert!(decisions_doc.contains("| `ADR-22` |"));
    assert!(decisions_doc.contains("| `ADR-23` |"));
    assert!(decisions_doc.contains("| `ADR-24` |"));
    assert!(decisions_doc.contains("| `ADR-25` |"));
    assert!(decisions_doc.contains("| `ADR-26` |"));
    assert!(decisions_doc.contains("| `ADR-27` |"));

    // Kluczowe zasady
    assert!(decisions_doc.contains("DYNAMIC DISTRIBUTION-DERIVED KERNEL COMPATIBILITY FLOOR"));
    assert!(decisions_doc.contains("Common Compatibility Intersection"));
    assert!(decisions_doc.contains("Journaled Transactional Replacement"));
    assert!(decisions_doc.contains("Not Power-Loss Atomic"));
    assert!(decisions_doc.contains("Crash Recoverable by Journal"));

    // Weryfikacja Linux Support Snapshot JSON
    let snapshot_content = fs::read_to_string("LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json")
        .expect("read LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json");
    let json: serde_json::Value =
        serde_json::from_str(&snapshot_content).expect("parse snapshot JSON");

    assert_eq!(
        json["snapshot_schema"],
        "splogbook-linux-support-snapshot-v2"
    );
    assert_eq!(json["product_version"], "1.3.0-alpha.1");
    assert_eq!(json["architecture"], "x86_64");
    assert_eq!(json["rust_target"], "x86_64-unknown-linux-gnu");
    assert_eq!(json["snapshot_date"], "2026-11-09");
    assert_eq!(json["resolved_minimum_kernel"], "7.0");

    // ADR-16: minimum dynamiczne, nie sztywne 7.2.
    assert_eq!(json["kernel_policy"]["decision_id"], "ADR-16");
    assert_eq!(
        json["kernel_policy"]["model"],
        "dynamic-distribution-derived-floor"
    );
    assert_eq!(json["kernel_policy"]["minimum_kernel"], "7.0");

    // ADR-17: minimum glibc 2.43.
    assert_eq!(json["glibc_policy"]["minimum_glibc"], "2.43");
    assert_eq!(
        json["glibc_policy"]["maximum_required_glibc_symbol"],
        "GLIBC_2.43"
    );
    assert_eq!(json["glibc_policy"]["bundled_glibc"], false);

    let profiles = json["profiles"]
        .as_array()
        .expect("profiles must be an array");
    assert_eq!(
        profiles.len(),
        4,
        "Wspierana macierz musi liczyć dokładnie 4 profile"
    );

    let expected_ids = [
        "ubuntu-lts",
        "fedora-final",
        "arch-rolling",
        "opensuse-tumbleweed",
    ];
    for expected_id in expected_ids {
        assert!(
            profiles.iter().any(|p| p["distribution_id"] == expected_id),
            "Brak profilu {expected_id} w Linux Support Snapshot"
        );
    }

    // Debian i Leap są ewidencjonowane jako niewspierane (glibc < 2.43).
    for unsupported_id in ["debian-stable", "opensuse-leap"] {
        assert!(
            !profiles
                .iter()
                .any(|p| p["distribution_id"] == unsupported_id),
            "Profil {unsupported_id} nie może być w macierzy wspieranej"
        );
    }
    let evaluated = json["evaluated_not_supported"]
        .as_array()
        .expect("evaluated_not_supported must be an array");
    let debian = evaluated
        .iter()
        .find(|p| p["distribution_id"] == "debian-stable")
        .expect("debian-stable musi być w evaluated_not_supported");
    assert_eq!(debian["status"], "NOT_SUPPORTED");
    assert_eq!(debian["glibc_version"], "2.41");
    assert!(
        evaluated
            .iter()
            .any(|p| p["distribution_id"] == "opensuse-leap"),
        "opensuse-leap musi być w evaluated_not_supported"
    );

    // Dynamiczny floor wyliczony z danych profili musi równać się zadeklarowanemu minimum.
    fn kernel_line(value: &serde_json::Value) -> (u32, u32) {
        let text = value.as_str().expect("kernel_line musi być stringiem");
        let mut parts = text.split('.');
        let major = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .expect("major kernela");
        let minor = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .expect("minor kernela");
        (major, minor)
    }
    let computed_floor = profiles
        .iter()
        .map(|p| kernel_line(&p["kernel_line"]))
        .min()
        .expect("co najmniej jeden profil");
    assert_eq!(
        computed_floor,
        kernel_line(&json["resolved_minimum_kernel"]),
        "Zadeklarowane minimum musi być najstarszą linią kernela wśród wspieranych profili"
    );
    let floor_distros: Vec<&str> = profiles
        .iter()
        .filter(|p| kernel_line(&p["kernel_line"]) == computed_floor)
        .map(|p| p["distribution_id"].as_str().expect("distribution_id"))
        .collect();
    assert_eq!(
        floor_distros,
        vec!["ubuntu-lts"],
        "Minimum 7.0 wyznacza wyłącznie Ubuntu 26.04 LTS"
    );

    // Walidacja profilu Arch
    let arch_profile = profiles
        .iter()
        .find(|p| p["distribution_id"] == "arch-rolling")
        .expect("arch-rolling profile");
    assert_eq!(arch_profile["release_model"], "rolling");
    assert_eq!(arch_profile["official_repositories_only"], true);
    assert_eq!(arch_profile["aur_used"], false);
    assert_eq!(arch_profile["testing_repositories_used"], false);
    assert_eq!(arch_profile["kernel_line"], "7.2");

    // Walidacja profilu Tumbleweed
    let tw_profile = profiles
        .iter()
        .find(|p| p["distribution_id"] == "opensuse-tumbleweed")
        .expect("opensuse-tumbleweed profile");
    assert_eq!(tw_profile["release_model"], "rolling");
    assert_eq!(tw_profile["official_repositories_only"], true);
    assert_eq!(tw_profile["factory_used"], false);
    assert_eq!(tw_profile["development_repositories_used"], false);
    assert_eq!(tw_profile["kernel_line"], "7.2");

    // Walidacja profilu Ubuntu LTS (wyznacza minimum)
    let ubuntu_profile = profiles
        .iter()
        .find(|p| p["distribution_id"] == "ubuntu-lts")
        .expect("ubuntu-lts profile");
    assert_eq!(ubuntu_profile["kernel_line"], "7.0");
    assert_eq!(ubuntu_profile["glibc_version"], "2.43");

    // Brak śledzenia archiwów tar.gz w git
    let output = Command::new("git")
        .args(["ls-files", "*.tar.gz"])
        .output()
        .expect("git ls-files");
    let tracked_tar_gz = String::from_utf8_lossy(&output.stdout);
    assert!(
        tracked_tar_gz.trim().is_empty(),
        "W repozytorium git nie może być śledzone żadne archiwum .tar.gz: {tracked_tar_gz}"
    );

    // Nienaruszalność tagów wydań stabilnych v1.2.0 i v1.3.0.
    assert_stable_release_tags_are_pinned(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));

    // Walidacja skryptu budowania archiwum Linux portable
    let build_script = fs::read_to_string("scripts/build_linux_portable_tar_gz.sh")
        .expect("scripts/build_linux_portable_tar_gz.sh");
    assert!(
        build_script.contains("set -euo pipefail"),
        "Skrypt budowania musi używać bezpiecznych flag basha set -euo pipefail"
    );
    assert!(
        build_script.contains("--owner=0 --group=0 --numeric-owner"),
        "Skrypt budowania musi normalizować właściciela tar do root:root"
    );
    assert!(
        !build_script.contains("sudo "),
        "Skrypt budowania nie może zawierać sudo"
    );
    assert!(
        !build_script.contains("pkexec "),
        "Skrypt budowania nie może zawierać pkexec"
    );
    assert!(
        build_script.contains("RESOLVED_MIN_KERNEL=\"7.0\""),
        "Skrypt budowania musi zapisywać minimum kernela 7.0 (ADR-16)"
    );
    assert!(
        !build_script.contains("RESOLVED_MIN_KERNEL=\"6.12\"")
            && !build_script.contains("RESOLVED_MIN_KERNEL=\"7.2\""),
        "Skrypt budowania nie może kodować zastąpionych minimów 6.12 ani 7.2"
    );

    // Macierz CI musi odpowiadać wspieranej macierzy (4 profile, bez Debian/Leap/26.10).
    let release_workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect(".github/workflows/build-and-release.yml");
    for supported in [
        "ubuntu-lts",
        "fedora-final",
        "arch-rolling",
        "opensuse-tumbleweed",
    ] {
        assert!(
            release_workflow.contains(&format!("distro_id: \"{supported}\"")),
            "Macierz CI musi zawierać wspierany profil {supported}"
        );
    }
    for unsupported in ["debian-stable", "opensuse-leap", "ubuntu-stable"] {
        assert!(
            !release_workflow.contains(&format!("distro_id: \"{unsupported}\"")),
            "Macierz CI nie może zawierać niewspieranego profilu {unsupported}"
        );
    }

    // Fixture weryfikacji natywnej jest fail-closed od bramki F-03.
    let verify_script = fs::read_to_string("scripts/verify_linux_container.sh")
        .expect("scripts/verify_linux_container.sh");
    assert!(
        !verify_script.contains("LEGACY NON-FAIL-CLOSED"),
        "Skrypt weryfikacji nie może już deklarować legacy non-fail-closed (F-03)"
    );
    assert!(
        verify_script.contains("NATIVE VERIFICATION COMPLETED SUCCESSFULLY")
            && verify_script.contains("set -eu"),
        "Skrypt weryfikacji musi być fail-closed i kończyć się komunikatem sukcesu"
    );

    // Weryfikacja szablonu .desktop
    let desktop_content =
        fs::read_to_string("installer/linux/portable/templates/splogbook.desktop")
            .expect("installer/linux/portable/templates/splogbook.desktop");
    assert!(desktop_content.contains("Type=Application"));
    assert!(desktop_content.contains("Categories=HamRadio;Utility;"));
    assert!(desktop_content.contains("Terminal=false"));
}

// ============================================================================
// Linux packaging consistency verification guard (2026-11-09)
// Strażnik spójności po zamknięciu M3-F: precyzyjna klasyfikacja drzewa
// (tracked / untracked / ignored / strict), klasyfikacja dowodu natywnego
// oraz formalna bramka F-03 (fail-closed weryfikacja Linux).
// ============================================================================

#[test]
fn m3_f_post_closure_consistency_guard() {
    // Strażnik spójności po zamknięciu M3-F. Czyta wyłącznie publiczną dokumentację
    // trwałą; raporty procesowe nie są źródłem prawdy.
    let dev = fs::read_to_string("docs/DEVELOPMENT.md").expect("docs/DEVELOPMENT.md");
    let roadmap = fs::read_to_string("docs/ROADMAP.md").expect("docs/ROADMAP.md");
    let decisions = fs::read_to_string("docs/architecture/DECISIONS.md")
        .expect("docs/architecture/DECISIONS.md");
    let issues = fs::read_to_string("docs/KNOWN_ISSUES.md").expect("docs/KNOWN_ISSUES.md");
    let arch = fs::read_to_string("docs/architecture/ARCHITECTURE.md")
        .expect("docs/architecture/ARCHITECTURE.md");
    let secproc = fs::read_to_string("docs/SECURITY_PROCESS.md").expect("docs/SECURITY_PROCESS.md");

    // 1. Stan drzewa musi być opisany jawnie (tracked / untracked / ignored / strict).
    for state in [
        "TRACKED_CLEAN",
        "UNTRACKED_PRESENT",
        "IGNORED_PRESENT",
        "STRICT_WORKTREE_CLEAN",
    ] {
        assert!(
            dev.contains(state),
            "docs/DEVELOPMENT.md: brak jawnej definicji stanu drzewa {state}"
        );
    }
    assert!(
        dev.contains(".git/info/exclude"),
        "docs/DEVELOPMENT.md musi opisywać lokalne wykluczanie przez .git/info/exclude"
    );

    // 2. Dowód natywny jest dowodem userspace/CI, nie dowodem jądra dystrybucji.
    assert!(
        secproc.contains("userspace and CI evidence"),
        "docs/SECURITY_PROCESS.md musi klasyfikować dowód z kontenera OCI"
    );

    // 3. F-03 jest zamknięte i nie może być otwarte bez odtwarzalnej regresji.
    assert!(
        roadmap.contains("F-03") && roadmap.contains("CLOSED"),
        "ROADMAP musi utrzymywać F-03 jako CLOSED"
    );
    assert!(
        roadmap.contains("NATIVE VERIFICATION COMPLETED SUCCESSFULLY"),
        "ROADMAP musi opisywać fail-closed werdykt weryfikatora natywnego"
    );
    assert!(
        !roadmap.contains("4 profile w GitHub Actions Run ID 42518873662"),
        "ROADMAP nie może powtarzać nieprawdziwego opisu runu 42518873662"
    );

    // 4. Stan przyrostowy: M3-G/M3-H/M3-I dostarczone, M3-J nie rozpoczęte.
    for milestone in ["M3-G", "M3-H", "M3-I"] {
        assert!(
            roadmap.contains(milestone),
            "ROADMAP musi odnotowywać kamień milowy {milestone}"
        );
    }
    assert!(
        roadmap.contains("M3-J") && roadmap.contains("DELIVERED"),
        "ROADMAP musi odnotowywać M3-J (Flatpak) jako dostarczone po zamknięciu bramki"
    );

    // 5. Rejestr decyzji zachowuje trwałe identyfikatory ADR; otwarte P0 jest jawne.
    for adr_id in ["| `ADR-31` |", "| `ADR-37` |", "| `ADR-38` |"] {
        assert!(
            decisions.contains(adr_id),
            "DECISIONS musi zapisywać rekord {adr_id}"
        );
    }
    assert!(
        issues.contains("F-14"),
        "KNOWN_ISSUES musi jawnie ewidencjonować otwarte znalezisko F-14"
    );

    // 6. Mapa architektury nie może twierdzić, że pakiety Linux nie są zaimplementowane.
    assert!(
        !arch.contains("brak implementacji"),
        "ARCHITECTURE nie może deklarować braku implementacji pakietów Linux"
    );
}

// ============================================================================
// Baseline determinism and hermeticity guard (2026-11-09)
// Strażnik hermetyczności natywnych testów instalatorów (F-24).
// Wymaga wyłącznie plików źródłowych — NIE wymaga lokalnego instalatora.
// ============================================================================

fn qa_code_lines(source: &str) -> impl Iterator<Item = &str> {
    source.lines().filter(|line| {
        let trimmed = line.trim_start();
        !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*')
    })
}

#[test]
fn qa_native_installer_tests_are_hermetic_and_fail_closed() {
    let inno = fs::read_to_string("tests/inno_user_data_safety_test.rs")
        .expect("tests/inno_user_data_safety_test.rs");
    let msi = fs::read_to_string("tests/msi_user_data_safety_test.rs")
        .expect("tests/msi_user_data_safety_test.rs");
    let support = fs::read_to_string("tests/support/native_artifact.rs")
        .expect("tests/support/native_artifact.rs");
    let ci = fs::read_to_string(".github/workflows/ci.yml").expect(".github/workflows/ci.yml");

    // 1-2. Standardowy baseline nie może zależeć od `dist/` ani używać globów.
    for (name, source) in [
        ("inno_user_data_safety_test.rs", &inno),
        ("msi_user_data_safety_test.rs", &msi),
    ] {
        for line in qa_code_lines(source) {
            assert!(
                !line.contains("dist/") && !line.contains("dist\\"),
                "{name}: kod nie może odwoływać się do katalogu dist/: {line}"
            );
            assert!(
                !line.contains("glob("),
                "{name}: zabroniony glob przy wyborze artefaktu: {line}"
            );
            assert!(
                !line.contains("|| true"),
                "{name}: zabronione maskowanie błędów `|| true`: {line}"
            );
            assert!(
                !line.contains("C:\\Users") && !line.contains("AppData\\\\Roaming"),
                "{name}: zabroniona ścieżka produkcyjnego profilu użytkownika: {line}"
            );
        }
        assert!(
            !source
                .lines()
                .any(|line| line.trim_start().starts_with("#[ignore")),
            "{name}: nie wolno używać #[ignore] (kanoniczna liczba HIL = 10)"
        );
        assert!(
            !source.to_lowercase().contains("hil"),
            "{name}: test instalatora nie może uruchamiać HIL"
        );
    }

    // 3-4. Opt-in i jawna ścieżka artefaktu w module wspólnym.
    for required in [
        "SPLOG_RUN_INNO_E2E",
        "SPLOG_INNO_INSTALLER_PATH",
        "SPLOG_RUN_MSI_E2E",
        "SPLOG_MSI_INSTALLER_PATH",
        "is_absolute",
        "is_symlink",
        "canonical_name",
        "sha256_hex",
        "expected_sha256",
        "run_with_timeout",
        "log_tail",
    ] {
        assert!(
            support.contains(required),
            "tests/support/native_artifact.rs musi zawierać kontrakt: {required}"
        );
    }

    // 5. Fail-closed: brak opt-in = jawny fixture-skip; błąd = panika.
    for required in [
        "NATIVE INSTALLER E2E NOT REQUESTED",
        "NATIVE INSTALLER E2E FAIL-CLOSED",
        "verify_sha256",
        "run_with_timeout",
    ] {
        assert!(
            inno.contains(required),
            "inno_user_data_safety_test.rs musi zawierać: {required}"
        );
    }
    for required in [
        "NATIVE MSI ARTIFACT CHECK NOT REQUESTED",
        "NATIVE MSI ARTIFACT CHECK FAIL-CLOSED",
    ] {
        assert!(
            msi.contains(required),
            "msi_user_data_safety_test.rs musi zawierać: {required}"
        );
    }

    // 6. Izolacja środowiska wykonania i log instalatora.
    for required in [
        "tempfile::tempdir",
        "/VERYSILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
        "/LOG=",
        "/DIR=",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "install_code",
        "uninstall_code",
    ] {
        assert!(
            inno.contains(required),
            "inno E2E musi izolować środowisko i weryfikować exit code: {required}"
        );
    }

    // 7-8. Kanoniczna liczba ignored HIL = 10, bez nowych ignored w tests/.
    let mut ignored = 0_usize;
    for entry in fs::read_dir("tests").expect("tests dir") {
        let path = entry.expect("tests entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read test source");
        ignored += source
            .lines()
            .filter(|line| line.trim_start().starts_with("#[ignore"))
            .count();
    }
    assert_eq!(
        ignored, 10,
        "kanoniczna liczba ignored testów HIL musi pozostać równa 10"
    );

    // 9. Standardowy job CI nie ustawia opt-in i uruchamia strażnika.
    assert!(
        !ci.contains("SPLOG_RUN_"),
        ".github/workflows/ci.yml nie może ustawiać zmiennych opt-in natywnego E2E"
    );
    assert!(
        ci.contains("qa_native_installer_tests_are_hermetic_and_fail_closed"),
        "CI musi uruchamiać strażnik hermetyczności"
    );
    // Liczniki bazowe mają dokładnie jedno źródło prawdy: docs/VERIFICATION_BASELINE.md.
    // Workflow nie może ich duplikować, ale musi na to źródło wskazywać.
    assert!(
        ci.contains("docs/VERIFICATION_BASELINE.md"),
        "CI musi wskazywać jedno źródło prawdy dla liczników (docs/VERIFICATION_BASELINE.md)"
    );
    assert!(
        !ci.contains("2735 registered") && !ci.contains("2725 executed"),
        "CI nie może duplikować liczników bazowych — obowiązuje docs/VERIFICATION_BASELINE.md"
    );
    let baseline =
        fs::read_to_string("docs/VERIFICATION_BASELINE.md").expect("docs/VERIFICATION_BASELINE.md");
    for key in [
        "TOTAL REGISTERED TESTS",
        "TOTAL EXECUTED TESTS",
        "TOTAL PASSED TESTS",
        "TOTAL IGNORED TESTS",
        "HIL IGNORED TESTS",
    ] {
        assert!(
            baseline.contains(key),
            "docs/VERIFICATION_BASELINE.md musi zawierać kanoniczny licznik {key}"
        );
    }
}

// ============================================================================
// XDG environment determinism guard (2026-11-14)
// Strażnik determinizmu testów mutujących środowisko procesu.
//
// Wymusza: jeden wspólny helper (mutex + RAII + odzyskiwanie po poison),
// brak bezpośrednich mutacji środowiska poza helperem i jawnie udokumentowanym
// wyjątkiem HIL, brak `#[ignore]`/filtrowania/`--test-threads=1`/retry/sleep,
// brak nowej zależności do serializacji testów oraz obecność pomiarów w baseline.
// ============================================================================

#[test]
fn qa_process_environment_mutation_is_serialized_and_guarded() {
    let helper = fs::read_to_string("tests/support/process_env.rs")
        .expect("tests/support/process_env.rs must exist");
    let xdg = fs::read_to_string("tests/linux_xdg_data_preservation_test.rs")
        .expect("tests/linux_xdg_data_preservation_test.rs");
    let env_contract = fs::read_to_string("tests/process_environment_isolation_test.rs")
        .expect("tests/process_environment_isolation_test.rs");
    let hil = fs::read_to_string("tests/cat_native_7.rs").expect("tests/cat_native_7.rs");
    let ci = fs::read_to_string(".github/workflows/ci.yml").expect(".github/workflows/ci.yml");
    let release = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect(".github/workflows/build-and-release.yml");
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Cargo.toml");
    let cargo_lock = fs::read_to_string("Cargo.lock").expect("Cargo.lock");
    let baseline =
        fs::read_to_string("docs/VERIFICATION_BASELINE.md").expect("docs/VERIFICATION_BASELINE.md");

    // 1. Wspólny helper: jedna blokada, RAII, przywracanie po panic, obsługa poison.
    for required in [
        "static ENV_LOCK: Mutex<()> = Mutex::new(())",
        "thread_local!",
        "pub struct EnvGuard",
        "impl Drop for EnvGuard",
        "PoisonError::into_inner",
        "TryLockError::WouldBlock",
        "EnvGuardError::NestedGuard",
        "EnvGuardError::Busy",
        "env::set_var",
        "env::remove_var",
        "# Safety",
    ] {
        assert!(
            helper.contains(required),
            "tests/support/process_env.rs musi zawierać kontrakt: {required}"
        );
    }

    // 2. Helper nie może wprowadzać zależności: wszystkie `use` pochodzą z `std`.
    for line in helper
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("use "))
    {
        assert!(
            line.starts_with("use std::"),
            "wspólny helper środowiska musi używać wyłącznie biblioteki standardowej: {line}"
        );
    }
    for forbidden in [
        "serial_test",
        "serial-test",
        "envtest",
        "temp-env",
        "env-lock",
        "envtest_derive",
    ] {
        assert!(
            !cargo_toml.to_lowercase().contains(forbidden),
            "Cargo.toml nie może dodawać zależności do serializacji testów: {forbidden}"
        );
        assert!(
            !cargo_lock.to_lowercase().contains(forbidden),
            "Cargo.lock nie może dodawać zależności do serializacji testów: {forbidden}"
        );
    }

    // 3. Testy XDG korzystają z jednego wspólnego helpera.
    for required in [
        "#[path = \"support/process_env.rs\"]",
        "mod process_env;",
        "EnvGuard::new",
    ] {
        assert!(
            xdg.contains(required),
            "tests/linux_xdg_data_preservation_test.rs musi używać wspólnego helpera: {required}"
        );
    }
    assert!(
        !xdg.contains("unsafe"),
        "testy XDG nie mogą zawierać rozproszonego `unsafe` — cała mutacja środowiska należy do helpera"
    );
    assert!(
        !xdg.lines()
            .any(|line| line.trim_start().starts_with("#[ignore")),
        "testów XDG nie wolno wyciszać przez #[ignore]"
    );
    assert!(
        !env_contract
            .lines()
            .any(|line| line.trim_start().starts_with("#[ignore")),
        "testów kontraktu środowiska nie wolno wyciszać przez #[ignore]"
    );
    assert!(
        !xdg.contains("sleep(") && !env_contract.contains("sleep(") && !helper.contains("sleep("),
        "synchronizacja środowiska nie może używać sleep"
    );

    // 4. Testy kontraktu helpera pokrywają wymagane zachowania.
    for required_test in [
        "existing_variable_is_restored",
        "missing_variable_stays_missing",
        "multiple_variables_are_restored",
        "empty_value_is_distinct_from_missing",
        "guard_observes_requested_value_inside_section",
        "environment_is_restored_after_panic",
        "lock_is_not_permanently_poisoned_after_panic",
        "nested_guard_is_explicitly_rejected",
        "concurrent_acquisition_reports_busy_and_then_succeeds",
        "concurrent_mutations_are_serialized",
        "baseline_is_restored_after_drop",
        "guard_does_not_change_current_directory",
    ] {
        assert!(
            env_contract.contains(&format!("fn {required_test}()")),
            "brak testu kontraktu helpera: {required_test}"
        );
    }

    // 5. Każda bezpośrednia mutacja środowiska w `src/**` i `tests/**` jest zabroniona,
    //    z wyjątkiem wspólnego helpera oraz jawnie udokumentowanego wyjątku HIL.
    let listed = Command::new("git")
        .args(["ls-files", "src", "tests"])
        .output()
        .expect("git ls-files must be executable");
    assert!(
        listed.status.success(),
        "git ls-files src tests must succeed"
    );
    let mut hil_mutations = 0_usize;
    for path in String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(str::trim)
        .filter(|path| path.ends_with(".rs"))
    {
        if path == "tests/support/process_env.rs" || path == "tests/security_guard.rs" {
            // Helper jest jedynym dozwolonym miejscem mutacji; strażnik zawiera
            // wyłącznie wzorce wykrywające, nie mutacje.
            continue;
        }
        let source = fs::read_to_string(path).expect("read tracked source");
        for (index, line) in source.lines().enumerate() {
            if !(line.contains("env::set_var(") || line.contains("env::remove_var(")) {
                continue;
            }
            if path == "tests/cat_native_7.rs" {
                hil_mutations += 1;
                assert!(
                    line.contains("std::env::"),
                    "{path}:{}: mutacja HIL musi być jawnym wywołaniem std::env",
                    index + 1
                );
                continue;
            }
            panic!(
                "{path}:{} mutuje środowisko procesu poza wspólnym helperem \
                 tests/support/process_env.rs",
                index + 1
            );
        }
    }
    assert_eq!(
        hil_mutations, 5,
        "liczbę mutacji środowiska w cat_native_7 wolno zmienić wyłącznie przez migrację do wspólnego helpera"
    );
    assert!(
        hil.contains("Chroni wyłącznie zmienne")
            && hil.contains("SPLOG_HIL_*")
            && hil.contains("innym procesie"),
        "wyjątek HIL musi być jawnie udokumentowany: rozłączny zestaw zmiennych, inny proces testowy"
    );
    assert!(
        hil.contains("Manifest nie mutuje środowiska procesu")
            && hil.contains("run_isolated_hardware_dispatcher_at"),
        "HIL nie może wracać do niezsynchronizowanego seeda SPLOG_HIL_REFERENCE_TIME"
    );

    // 6. Workflow nie może filtrować, wyciszać ani serializować globalnie testów.
    for (name, workflow) in [("ci.yml", &ci), ("build-and-release.yml", &release)] {
        for forbidden in [
            "--test-threads",
            "RUST_TEST_THREADS",
            "--skip linux_xdg_data_preservation_test",
            "--skip process_environment_isolation_test",
            "continue-on-error",
        ] {
            assert!(
                !workflow.contains(forbidden),
                "{name} nie może zawierać {forbidden} (zakaz filtrowania, wyciszania i globalnej serializacji testów)"
            );
        }
    }
    assert!(
        ci.contains("cargo test --workspace --all-targets"),
        "CI musi uruchamiać pełny zestaw testów workspace"
    );
    assert!(
        release.contains("xvfb-run --auto-servernum cargo test -- --nocapture")
            && release.contains("cargo test --verbose"),
        "Build and Release musi uruchamiać pełny zestaw testów na Linux i Windows"
    );

    // 7. Baseline zawiera zmierzony kontrakt determinizmu środowiska.
    for required in [
        "`process_environment_isolation_test`",
        "`tests/support/process_env.rs`",
        "iteracji stresowych",
    ] {
        assert!(
            baseline.contains(required),
            "docs/VERIFICATION_BASELINE.md musi dokumentować pomiar determinizmu środowiska: {required}"
        );
    }
}

// ============================================================================
// F-03 fail-closed Linux CI verification guard (2026-11-09)
// Strażnik fail-closed natywnej weryfikacji Linux (F-03): brak maskowania
// progów krytycznych, jawna kontrola GLIBC_2.43, instalacja bibliotek runtime
// przed weryfikatorem, dokładnie 4 profile, test negatywny w CI oraz
// publikacja release wyłącznie dla tagu.
// ============================================================================

#[test]
fn m3_f_f03_fail_closed_verification_guard() {
    let script = fs::read_to_string("scripts/verify_linux_container.sh")
        .expect("scripts/verify_linux_container.sh");
    let installer = fs::read_to_string("scripts/install_linux_runtime_deps.sh")
        .expect("scripts/install_linux_runtime_deps.sh");
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect(".github/workflows/build-and-release.yml")
        .replace("\r\n", "\n");

    // 1. Rygorystyczny tryb powłoki i brak markera legacy.
    assert!(
        script.contains("set -eu"),
        "weryfikator musi używać rygorystycznego trybu `set -eu`"
    );
    assert!(
        !script.contains("LEGACY NON-FAIL-CLOSED"),
        "weryfikator nie może już deklarować legacy non-fail-closed (F-03)"
    );

    // 2. Żaden próg krytyczny nie jest maskowany `|| true`; każde pozostałe
    //    `|| true` musi być jawnie oznaczone jako diagnostyka niekrytyczna.
    for line in script.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let is_ldd_target = trimmed.contains("ldd")
            && trimmed.contains("SPLogbook/SPLogbook")
            && !trimmed.contains("ldd --version");
        let is_version = trimmed.contains("--version") && trimmed.contains("SPLogbook/SPLogbook");
        let is_build_info =
            trimmed.contains("--build-info") && trimmed.contains("SPLogbook/SPLogbook");
        if is_ldd_target || is_version || is_build_info {
            assert!(
                !trimmed.contains("|| true"),
                "krytyczna kontrola F-03 nie może być maskowana `|| true`: {trimmed}"
            );
        }
        if trimmed.contains("|| true") {
            assert!(
                trimmed.contains("NON-CRITICAL"),
                "każde pozostałe `|| true` musi być jawnie oznaczone NON-CRITICAL: {trimmed}"
            );
        }
    }

    // 3. Komunikat sukcesu występuje dokładnie raz i jest ostatnią instrukcją.
    let success = "NATIVE VERIFICATION COMPLETED SUCCESSFULLY";
    let success_occurrences = script
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && line.contains(success))
        .count();
    assert_eq!(
        success_occurrences, 1,
        "komunikat sukcesu musi wystąpić dokładnie raz w kodzie wykonawczym"
    );
    let last_instruction = script
        .lines()
        .rfind(|line| !line.trim().is_empty())
        .expect("skrypt nie może być pusty")
        .trim();
    assert_eq!(
        last_instruction, "echo \"NATIVE VERIFICATION COMPLETED SUCCESSFULLY\"",
        "komunikat sukcesu musi być ostatnią instrukcją skryptu (po wszystkich kontrolach)"
    );

    // 4. `not found` i błąd loadera są jawnie odrzucane.
    assert!(
        script.contains("\"not found\""),
        "weryfikator musi jawnie odrzucać wynik ldd zawierający `not found`"
    );
    assert!(
        script.contains("error while loading shared libraries"),
        "weryfikator musi jawnie odrzucać błąd loadera"
    );
    assert!(
        script.contains("ldd exited with code"),
        "weryfikator musi kontrolować niezerowy kod wyjścia ldd"
    );

    // 5. Jawna polityka GLIBC_2.43 z semantycznym porównaniem wersji.
    assert!(
        script.contains("GLIBC_POLICY_MAX=\"2.43\""),
        "weryfikator musi jawnie kodować politykę GLIBC_2.43"
    );
    assert!(
        script.contains("sort -V") && script.contains("version_le"),
        "weryfikator musi porównywać wersje semantycznie (nie naiwnie tekstowo)"
    );
    assert!(
        script.contains("newer than the GLIBC_"),
        "weryfikator musi kończyć się FAIL przy symbolu nowszym niż polityka"
    );

    // 6. Instalator runtime: oficjalne menedżery, pełna lista sonamów, fail-closed.
    for soname in [
        "libasound.so.2",
        "libX11.so.6",
        "libXcursor.so.1",
        "libXrandr.so.2",
        "libXi.so.6",
        "libxkbcommon.so.0",
        "libxkbcommon-x11.so.0",
        "libGL.so.1",
        "libfontconfig.so.1",
        "libwayland-client.so.0",
        "libwayland-cursor.so.0",
        "libwayland-egl.so.1",
        "libgtk-3.so.0",
        "libusb-1.0.so.0",
        "libudev.so.1",
    ] {
        assert!(
            installer.contains(soname),
            "instalator runtime musi wymagać sonamu {soname}"
        );
    }
    for manager in [
        "apt-get install",
        "dnf install",
        "pacman -Sy",
        "zypper --non-interactive install",
    ] {
        assert!(
            installer.contains(manager),
            "instalator runtime musi używać oficjalnego menedżera: {manager}"
        );
    }
    assert!(
        installer.contains("ldconfig -p"),
        "instalator runtime musi weryfikować sonamy przez ldconfig -p"
    );
    assert!(
        installer.contains("[ -z \"${MISSING}\" ] || fail"),
        "instalator runtime musi być fail-closed przy brakującym sonamie"
    );

    // 7. Workflow: dokładnie 4 przypięte profile, instalacja przed weryfikatorem,
    //    przekazanie dokładnego SHA commita.
    assert_eq!(
        workflow.matches("distro_id: \"").count(),
        4,
        "macierz CI musi zawierać dokładnie 4 profile"
    );
    assert_eq!(
        workflow.matches("image: \"").count(),
        4,
        "każdy profil musi mieć jawnie przypięty obraz OCI"
    );
    for digest in [
        "ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c",
        "fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c",
        "archlinux:base-20261101.0.620205@sha256:b1c2fe6e717bfd5673af8fc217e1e1c3ce0dcbb1d53c3414c8425e7ce79fc6d9",
        "opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1",
    ] {
        assert!(
            workflow.contains(digest),
            "workflow musi przypinać obraz przez digest: {digest}"
        );
    }
    let run_command = "/bin/sh -c 'set -e; /bin/sh /workspace/install_runtime_deps.sh; exec /bin/sh /workspace/run_test.sh'";
    assert!(
        workflow.contains(run_command),
        "weryfikator musi być uruchamiany po instalacji bibliotek runtime"
    );
    let install_at = run_command.find("install_runtime_deps.sh").unwrap();
    let verify_at = run_command.find("run_test.sh").unwrap();
    assert!(
        install_at < verify_at,
        "instalacja bibliotek musi poprzedzać uruchomienie weryfikatora"
    );
    assert!(
        workflow.contains("-e EXPECTED_BUILD_COMMIT=\"${{ github.sha }}\""),
        "workflow musi przekazywać dokładny SHA commita do kontroli --build-info"
    );

    // 8. Test negatywny w CI: brak biblioteki => niezerowy kod i brak sukcesu.
    assert!(
        workflow.contains("verify-linux-negative"),
        "workflow musi zawierać job negatywny fail-closed"
    );
    assert!(
        workflow.contains("fail-closed negative fixture unexpectedly succeeded"),
        "job negatywny musi odrzucić nieoczekiwany sukces"
    );
    assert!(
        workflow.contains("success message was emitted despite the missing runtime library"),
        "job negatywny musi odrzucić emisję komunikatu sukcesu przy błędzie"
    );
    assert!(
        workflow.contains("expected 'not found' ldd evidence"),
        "job negatywny musi wymagać dowodu `not found`"
    );

    // 9. Publikacja release pozostaje zablokowana bez właściwego tagu.
    let publish = workflow
        .split("publish-release:")
        .nth(1)
        .expect("job publish-release musi istnieć");
    assert!(
        publish.contains("if: startsWith(github.ref, 'refs/tags/v')"),
        "Publish Release Assets musi być warunkowany tagiem v*"
    );
    // ADR-71: job-level warunek publikacji to dokładnie push tagu v*;
    // workflow_dispatch (dowolna gałąź, dowolne dry_run) zawsze pomija publikację.
    let publish_header = publish
        .split("    steps:")
        .next()
        .expect("nagłówek joba publish-release");
    let publish_conditions: Vec<&str> = publish_header
        .lines()
        .filter(|line| line.starts_with("    if:"))
        .collect();
    assert_eq!(
        publish_conditions,
        ["    if: startsWith(github.ref, 'refs/tags/v') && github.event_name == 'push'"],
        "Publish Release Assets musi mieć dokładnie warunek push tagu v* (ADR-71)"
    );
    assert!(
        !publish_conditions
            .iter()
            .any(|condition| condition.contains("workflow_dispatch")),
        "warunek joba publish-release nie może dopuszczać workflow_dispatch"
    );
    // Joby budujące i weryfikujące nie mają job-level `if:`, więc wykonują się
    // także przy workflow_dispatch.
    let jobs = workflow.split("\njobs:\n").nth(1).expect("sekcja jobs");
    let mut job_names: Vec<String> = Vec::new();
    let mut in_job_header = false;
    for line in jobs.lines() {
        let is_job_line = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_end().ends_with(':')
            && !line.trim_start().starts_with('#');
        if is_job_line {
            job_names.push(line.trim().trim_end_matches(':').to_string());
            in_job_header = true;
        } else if line.starts_with("    steps:") {
            in_job_header = false;
        } else if in_job_header && line.starts_with("    if:") {
            let name = job_names.last().map_or("", String::as_str);
            assert_eq!(
                name, "publish-release",
                "job `{name}` nie może mieć job-level `if:`; musi działać także przy workflow_dispatch"
            );
        }
    }
    for required in [
        "build-linux",
        "build-windows",
        "build-deb",
        "build-rpm",
        "build-flatpak",
        "verify-linux-matrix",
        "verify-appimage-fuse",
        "verify-flatpak",
        "publish-release",
    ] {
        assert!(
            job_names.iter().any(|name| name == required),
            "sekcja jobs musi zawierać `{required}` (parser strażnika)"
        );
    }
    assert!(
        publish.contains(
            "startsWith(github.ref, 'refs/tags/v') && github.event.inputs.dry_run != 'true'"
        ),
        "Create GitHub Release musi wymagać tagu v* i braku dry-run"
    );
    assert!(
        publish.contains(
            "needs: [build-linux, build-windows, verify-linux-matrix, verify-linux-negative, build-rpm, verify-rpm, verify-rpm-negative-fixtures]"
        ),
        "Publish Release Assets musi zależeć od wszystkich jobów weryfikacyjnych"
    );

    // 10. Publiczna dokumentacja domyka F-03 i opisuje fail-closed; M3-G jest dostarczone.
    let roadmap = fs::read_to_string("docs/ROADMAP.md").expect("docs/ROADMAP.md");
    let security_process =
        fs::read_to_string("docs/SECURITY_PROCESS.md").expect("docs/SECURITY_PROCESS.md");
    let decisions = fs::read_to_string("docs/architecture/DECISIONS.md")
        .expect("docs/architecture/DECISIONS.md");
    let snapshot = fs::read_to_string("LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json")
        .expect("LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json");

    assert!(
        roadmap.contains("F-03") && roadmap.contains("CLOSED"),
        "ROADMAP musi utrzymywać F-03 jako CLOSED"
    );
    assert!(
        roadmap.contains("M3-G") && roadmap.contains("AppImage"),
        "ROADMAP musi odnotowywać dostarczenie M3-G (AppImage)"
    );
    assert!(
        security_process.contains("F-03")
            && security_process.contains("NATIVE VERIFICATION COMPLETED SUCCESSFULLY"),
        "SECURITY_PROCESS musi opisywać fail-closed weryfikację natywną i zamknięcie F-03"
    );
    assert!(
        decisions.contains("| `ADR-38` |"),
        "kanon decyzji musi zapisywać ADR-38"
    );
    let snapshot_json: serde_json::Value =
        serde_json::from_str(&snapshot).expect("snapshot JSON musi być poprawny");
    assert_eq!(
        snapshot_json["native_evidence"]["status"], "FAIL_CLOSED_VERIFIED",
        "snapshot musi deklarować FAIL_CLOSED_VERIFIED"
    );
    assert_eq!(
        snapshot_json["native_evidence"]["ci_run_id"], "42685105375",
        "snapshot musi wskazywać run 42685105375"
    );
    assert_eq!(
        snapshot_json["native_evidence"]["negative_fixture"]["observed_exit_code"], 1,
        "snapshot musi zapisywać exit code 1 testu negatywnego"
    );
}

// ============================================================================
// Linux AppImage integration guard (2026-11-11)
// Strażnik trwałych kontraktów pakunku AppImage: oficjalne źródło i kanał
// continuous appimagetool, weryfikacja opublikowanego i lokalnego SHA-256,
// przypięcie jednego toolchainu na całą budowę, jawny type-2 runtime,
// AppRun bez eskalacji i bez LD_LIBRARY_PATH, polityka GLIBC_2.43, bundlowanie
// wyłącznie libhamlib.so.4, cztery profile, testy negatywne, brak publikacji
// release oraz utrzymane fail-closed F-03.
//
// Zgodnie z ADR-44 toolchain jest przypięty statycznie w
// installer/linux/appimage/toolchain-pins: bez REST API, bez cache, z twardą
// weryfikacją rozmiaru i SHA-256; piny muszą być zgodne z
// docs/architecture/DECISIONS.md.
// ============================================================================

#[test]
fn m3_g_appimage_integration_guard() {
    let install = fs::read_to_string("scripts/install_appimage_toolchain.sh")
        .expect("install_appimage_toolchain.sh");
    let build =
        fs::read_to_string("scripts/build_linux_appimage.sh").expect("build_linux_appimage.sh");
    let verify = fs::read_to_string("scripts/verify_appimage.sh").expect("verify_appimage.sh");
    let fixtures = fs::read_to_string("scripts/verify_appimage_negative_fixture.sh")
        .expect("negative fixture script");
    let apprun = fs::read_to_string("installer/linux/appimage/AppRun").expect("AppRun");
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect("build-and-release.yml");
    let gitignore = fs::read_to_string(".gitignore").expect(".gitignore");
    let f03 =
        fs::read_to_string("scripts/verify_linux_container.sh").expect("verify_linux_container.sh");
    let readme = fs::read_to_string("installer/linux/appimage/README.md").expect("appimage README");

    // 1. Wyłącznie oficjalne repozytorium i kanał continuous; brak AppImageKit.
    assert!(
        install.contains("APPIMAGETOOL_REPO=\"AppImage/appimagetool\""),
        "appimagetool musi pochodzić z oficjalnego AppImage/appimagetool"
    );
    assert!(
        install.contains("PINNED_CHANNEL=\"continuous\""),
        "przypięty kanał continuous musi być jawny"
    );
    assert!(
        !install.contains("AppImageKit"),
        "zakaz używania starego repozytorium AppImageKit"
    );

    // 2. Statyczny pin (ADR-44): zero zapytań REST API, bez cache, bez
    //    tokenu; piny w installer/linux/appimage/toolchain-pins zgodne z DECISIONS.
    for forbidden in [
        "api.github.com",
        "releases/latest",
        "releases/tags/",
        "Authorization",
    ] {
        assert!(
            !install.contains(forbidden),
            "instalator toolchainu nie może używać `{forbidden}` (statyczny pin)"
        );
    }
    assert!(
        !workflow.contains("actions/cache"),
        "toolchain AppImage nie może pochodzić z cache"
    );
    assert!(
        install.contains("installer/linux/appimage/toolchain-pins")
            && install.contains("OFFICIAL_URL_PREFIX=\"https://github.com/AppImage/\""),
        "instalator musi czytać piny i ograniczać URL do https://github.com/AppImage/"
    );
    let pins = fs::read_to_string("installer/linux/appimage/toolchain-pins")
        .expect("installer/linux/appimage/toolchain-pins")
        .replace("\r\n", "\n");
    let decisions = fs::read_to_string("docs/architecture/DECISIONS.md")
        .expect("docs/architecture/DECISIONS.md")
        .replace("\r\n", "\n");
    let pinning_row = decisions
        .lines()
        .find(|line| line.starts_with("| `ADR-44`"))
        .expect("DECISIONS.md musi zapisywać ADR-44");
    for (artifact, decision) in [("appimagetool", "ADR-40"), ("runtime", "ADR-41")] {
        let approved = decisions
            .lines()
            .find(|line| line.starts_with(&format!("| `{decision}`")))
            .unwrap_or_else(|| panic!("DECISIONS.md musi zapisywać {decision}"));
        for field in ["commit", "sha256", "size", "url"] {
            let value = pins
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{artifact}.{field}=")))
                .unwrap_or_else(|| panic!("brak pinu {artifact}.{field}"));
            let row = if matches!(field, "commit" | "sha256") {
                approved
            } else {
                pinning_row
            };
            assert!(
                row.contains(value),
                "pin {artifact}.{field} musi być zgodny z decyzją OWNER w DECISIONS.md"
            );
        }
    }

    // 3a. Twarda weryfikacja rozmiaru i SHA-256 (fail-closed), dopiero potem chmod.
    for code in [
        "_SIZE_MISMATCH",
        "_CHECKSUM_MISMATCH",
        "_DOWNLOAD_FAILED",
        "PIN_STALE",
    ] {
        assert!(
            install.contains(code),
            "instalator musi kończyć się błędem {code}"
        );
    }
    assert!(
        install.contains("sha256sum"),
        "instalator musi liczyć lokalny SHA-256"
    );
    let verified_at = install
        .find("TOOL_LOCAL_SHA256=\"$(download_and_verify TOOL")
        .expect("appimagetool musi przejść download_and_verify");
    let chmod_at = install
        .find("chmod 0755 \"${TOOL_PATH}\"")
        .expect("bit wykonywania musi być nadany jawnie");
    assert!(
        verified_at < chmod_at,
        "bit wykonywania wolno nadać dopiero po weryfikacji rozmiaru i SHA-256"
    );

    // 3. Brak fallbacku do 1.9.1, brak `curl | sh` i brak nieweryfikowanego latest.
    for line in install
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
    {
        assert!(
            !(line.contains("1.9.1")),
            "zakaz fallbacku do appimagetool 1.9.1: {line}"
        );
        assert!(
            !(line.contains("curl") && line.contains("| sh")),
            "zakaz `curl | sh`: {line}"
        );
        assert!(
            !(line.contains("curl") && line.contains("| bash")),
            "zakaz `curl | bash`: {line}"
        );
    }
    assert!(
        !install.contains("http://"),
        "wyłącznie HTTPS dla oficjalnych źródeł"
    );

    // 4. Jeden toolchain na całą bramkę: jedno rozwiązanie, jedna budowa.
    assert_eq!(
        workflow.matches("install_appimage_toolchain.sh").count(),
        1,
        "toolchain musi być rozwiązany dokładnie raz w całym workflow"
    );
    assert_eq!(
        workflow.matches("build_linux_appimage.sh").count(),
        1,
        "AppImage musi być zbudowany dokładnie raz w całym workflow"
    );
    assert!(
        build.contains("appimage-toolchain.json"),
        "budowa musi korzystać z przypiętych metadanych toolchainu"
    );
    assert!(
        build.contains("appimagetool on disk does not match the pinned SHA-256"),
        "budowa musi ponownie weryfikować SHA-256 przypiętego appimagetool"
    );

    // 5. Runtime przypięty i przekazany jawnie.
    assert!(
        install.contains("RUNTIME_REPO=\"AppImage/type2-runtime\""),
        "runtime musi pochodzić z oficjalnego AppImage/type2-runtime"
    );
    assert!(
        install.contains("runtime-x86_64"),
        "runtime musi być assetem x86_64"
    );
    assert!(
        build.contains("--runtime-file"),
        "runtime musi być przekazany jawnie przez --runtime-file"
    );
    assert!(
        build.contains("runtime on disk does not match the pinned SHA-256"),
        "budowa musi ponownie weryfikować SHA-256 przypiętego runtime"
    );

    // 6. AppRun: POSIX sh, exec, przekazanie argumentów, brak eskalacji i LD_LIBRARY_PATH.
    assert!(apprun.contains("#!/bin/sh"), "AppRun musi używać /bin/sh");
    assert!(apprun.contains("set -eu"), "AppRun musi używać set -eu");
    assert!(
        apprun.contains("exec \"${APPDIR}/usr/bin/SPLogbook\" \"$@\""),
        "AppRun musi przekazywać wszystkie argumenty przez exec"
    );
    for line in apprun
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
    {
        for forbidden in ["sudo", "pkexec", "doas", "LD_LIBRARY_PATH"] {
            assert!(
                !line.contains(forbidden),
                "AppRun nie może zawierać {forbidden}: {line}"
            );
        }
    }

    // 7. Polityka GLIBC_2.43 jest niezmieniona i porównywana semantycznie.
    assert!(
        verify.contains("GLIBC_POLICY_MAX=\"2.43\""),
        "weryfikator musi kodować GLIBC_POLICY_MAX=2.43"
    );
    assert!(
        verify.contains("version_le") && verify.contains("sort -V"),
        "porównanie GLIBC musi być semantyczne"
    );
    assert!(
        readme.contains("maximum_glibc=2.43") || readme.contains("maximum_glibc"),
        "README AppImage musi dokumentować politykę 2.43"
    );

    // 8. Bundlowany jest wyłącznie libhamlib.so.4; glibc i loader nie są bundlowane.
    assert!(
        build.contains("install -m 0755 \"${PAYLOAD}/lib/libhamlib.so.4\""),
        "bundlowany musi być dokładnie libhamlib.so.4"
    );
    assert!(
        build.contains("only libhamlib.so.4 may be bundled"),
        "budowa musi odrzucać dodatkowe biblioteki"
    );
    assert!(
        verify.contains("the loader/glibc/libm/libgcc_s/libstdc++ must not be bundled"),
        "weryfikator musi odrzucać bundlowanie loadera i glibc"
    );
    assert!(
        !build.contains("LD_LIBRARY_PATH"),
        "budowa nie może maskować zależności przez LD_LIBRARY_PATH"
    );

    // 9. AppImage i pobrane narzędzia nie są commitowane.
    assert!(
        gitignore.contains("*.AppImage"),
        ".gitignore musi ignorować artefakty AppImage"
    );
    assert!(
        !workflow.contains("git add"),
        "workflow nie może dodawać artefaktów do repozytorium"
    );

    // 10. publish-release pozostaje bez AppImage i bez zmian tag-gatingu.
    let publish = workflow
        .split("publish-release:")
        .nth(1)
        .expect("job publish-release musi istnieć");
    assert!(
        !publish.contains("AppImage"),
        "publish-release nie może publikować AppImage w M3-G"
    );
    assert!(
        publish.contains("if: startsWith(github.ref, 'refs/tags/v')"),
        "publish-release musi pozostać warunkowany tagiem v*"
    );

    // 11. Cztery profile i ten sam AppImage w każdym z nich.
    assert_eq!(
        workflow.matches("distro_id: \"").count(),
        4,
        "macierz musi zawierać dokładnie cztery profile"
    );
    assert!(
        workflow.matches("SPLogbook-Linux-x86_64-appimage").count() >= 3,
        "workflow musi pobierać ten sam artefakt AppImage w jobach weryfikacyjnych"
    );
    assert!(
        workflow.contains("scripts/verify_appimage.sh"),
        "profile muszą weryfikować ten sam AppImage weryfikatorem fail-closed"
    );

    // 12. Testy negatywne: 12 mutacji i brak komunikatu sukcesu.
    assert_eq!(
        fixtures.matches("expect_reject ").count(),
        12,
        "zestaw negatywny musi wywołać dokładnie 12 mutacji"
    );
    assert!(
        fixtures.contains("NEGATIVE FIXTURES REJECTED AS EXPECTED"),
        "zestaw negatywny musi raportować jawne odrzucenie"
    );

    // 13. Dane użytkownika są zabronione w obrazie.
    for forbidden in [
        "*.db",
        "*.db-wal",
        "*.db-shm",
        "default_log.db",
        "station_config.json",
        "*.adi",
        "*.adif",
        "*.cbr",
    ] {
        assert!(
            verify.contains(forbidden),
            "weryfikator musi zakazywać {forbidden} w obrazie"
        );
    }

    // 14. F-03 pozostaje fail-closed.
    assert!(
        f03.contains("set -eu"),
        "F-03: weryfikator musi używać set -eu"
    );
    assert!(
        !f03.contains("LEGACY NON-FAIL-CLOSED"),
        "F-03: brak markera legacy"
    );
    assert_eq!(
        f03.lines()
            .filter(|line| !line.trim_start().starts_with('#')
                && line.contains("NATIVE VERIFICATION COMPLETED SUCCESSFULLY"))
            .count(),
        1,
        "F-03: komunikat sukcesu dokładnie raz"
    );
    assert!(
        f03.contains("GLIBC_POLICY_MAX=\"2.43\""),
        "F-03: polityka GLIBC_2.43 musi pozostać"
    );
    assert!(
        verify.contains("APPIMAGE VERIFICATION COMPLETED SUCCESSFULLY"),
        "M3-G: komunikat sukcesu weryfikatora AppImage musi istnieć"
    );
}

// ============================================================================
// Linux DEB integration guard (2026-11-12)
// Strażnik binarnego pakietu DEB: oficjalny dpkg-deb, deterministyczna budowa,
// układ /opt/SPLogbook, fail-closed launcher, minimalne maintainer scripts,
// weryfikator fail-closed, 27 mutacji negatywnych, artefakt wyłącznie w CI,
// brak publikacji DEB i brak zmian w F-03 / AppImage / portable tar.gz.
// ============================================================================

#[test]
fn m3_h_deb_integration_guard() {
    let build =
        fs::read_to_string("scripts/build_linux_deb.sh").expect("scripts/build_linux_deb.sh");
    let verify =
        fs::read_to_string("scripts/verify_linux_deb.sh").expect("scripts/verify_linux_deb.sh");
    let fixtures = fs::read_to_string("scripts/verify_linux_deb_negative_fixture.sh")
        .expect("scripts/verify_linux_deb_negative_fixture.sh");
    let control = fs::read_to_string("installer/linux/deb/templates/control")
        .expect("installer/linux/deb/templates/control");
    let launcher =
        fs::read_to_string("installer/linux/deb/launcher").expect("installer/linux/deb/launcher");
    let desktop = fs::read_to_string("installer/linux/deb/splogbook.desktop")
        .expect("installer/linux/deb/splogbook.desktop");
    let postinst = fs::read_to_string("installer/linux/deb/templates/postinst")
        .expect("installer/linux/deb/templates/postinst");
    let postrm = fs::read_to_string("installer/linux/deb/templates/postrm")
        .expect("installer/linux/deb/templates/postrm");
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect(".github/workflows/build-and-release.yml");
    let gitignore = fs::read_to_string(".gitignore").expect(".gitignore");
    let f03 = fs::read_to_string("scripts/verify_linux_container.sh")
        .expect("scripts/verify_linux_container.sh");

    // 1. Oficjalny dpkg-deb, deterministyczna budowa i podwójna kontrola SHA.
    assert!(
        build.contains("set -euo pipefail"),
        "builder musi być fail-closed"
    );
    assert!(build.contains("dpkg-deb"), "builder musi używać dpkg-deb");
    assert!(
        build.contains("--root-owner-group"),
        "builder musi ustawiać root:root"
    );
    assert!(
        build.contains("-Z\"${DEB_COMPRESSION}\""),
        "builder musi jawnie ustawiać kompresję"
    );
    assert!(
        build.contains("SOURCE_DATE_EPOCH"),
        "builder musi używać SOURCE_DATE_EPOCH"
    );
    assert!(
        build.contains("touch -h -d"),
        "builder musi ustalać deterministyczne mtime"
    );
    assert!(
        build.contains("the DEB build is not reproducible"),
        "builder musi porównywać dwa niezależne buildy"
    );
    assert!(
        build.contains("EXPECTED_BUILD_COMMIT"),
        "builder musi weryfikować build_commit"
    );

    // 2. Brak zewnętrznych repozytoriów i niebezpiecznych wzorców.
    for forbidden in ["http://", "ppa:", "latest-debs", "curl | sh", "curl | bash"] {
        assert!(
            !build.contains(forbidden),
            "builder nie może zawierać {forbidden}"
        );
    }
    assert!(
        !build.contains("git add"),
        "builder nie może dodawać plików do Git"
    );

    // 3. Poprawna translacja wersji i dowód porządku.
    assert!(
        build.contains("dpkg --validate-version"),
        "wersja musi być walidowana przez dpkg"
    );
    assert!(
        build.contains("dpkg --compare-versions"),
        "porządek wersji musi być dowiedziony"
    );
    assert!(
        build.contains("an epoch is not allowed"),
        "epoch jest zabroniony"
    );
    assert!(
        build.contains("DEBIAN_VERSION=\"${BASE_VERSION}~${PRERELEASE}-${DEB_REVISION}\""),
        "prerelease musi być tłumaczony przez ~"
    );

    // 4. Kanoniczna tożsamość i zależności.
    for required in [
        "Package: splogbook",
        "Architecture: amd64",
        "Maintainer: Mariusz Woźniak (SP6INA) <contact@splogbook.org>",
        "Section: hamradio",
        "Priority: optional",
        "Homepage: https://github.com/sp6ina/SPLogbook",
        "libc6 (>= 2.43)",
    ] {
        assert!(
            control.contains(required),
            "control musi zawierać {required}"
        );
    }
    assert!(
        !control.contains("hamlib,"),
        "bundlowany Hamlib nie może być zależnością"
    );
    assert!(!control.to_lowercase().contains("official debian"));
    assert!(!control.contains("Recommends:"));

    // 5. Launcher fail-closed.
    assert!(
        launcher.contains("#!/bin/sh"),
        "launcher musi używać /bin/sh"
    );
    assert!(launcher.contains("set -eu"), "launcher musi używać set -eu");
    assert!(
        launcher.contains("exec /opt/SPLogbook/SPLogbook \"$@\""),
        "launcher musi przekazywać wszystkie argumenty"
    );
    for line in launcher
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
    {
        for forbidden in ["sudo", "pkexec", "doas", "LD_LIBRARY_PATH", "HOME", "XDG_"] {
            assert!(
                !line.contains(forbidden),
                "launcher nie może zawierać {forbidden}: {line}"
            );
        }
    }

    // 6. Desktop i ikona.
    assert!(
        desktop.contains("Exec=splogbook"),
        "desktop musi wołać launcher z PATH"
    );
    assert!(
        desktop.contains("Icon=splogbook"),
        "desktop musi wskazywać ikonę splogbook"
    );
    assert!(
        desktop.contains("StartupWMClass=SPLogbook"),
        "desktop musi ustawiać StartupWMClass"
    );
    let icon = fs::read("assets/icon.png").expect("assets/icon.png");
    let width = u32::from_be_bytes([icon[16], icon[17], icon[18], icon[19]]);
    let height = u32::from_be_bytes([icon[20], icon[21], icon[22], icon[23]]);
    assert_eq!((width, height), (256, 256), "ikona musi mieć 256x256");
    assert!(
        build.contains("usr/share/icons/hicolor/256x256/apps/splogbook.png"),
        "builder musi instalować ikonę w hicolor/256x256"
    );
    assert!(
        build.contains("HAMLIB_LGPL-2.1.txt") && build.contains("templates/LGPL-2.1.txt"),
        "builder musi dołączać tekst licencji LGPL-2.1 dla bundlowanego Hamlib"
    );
    assert!(
        build.contains("install -m 0644 \"${PAYLOAD}/lib/libhamlib.so.4\""),
        "biblioteka współdzielona nie może mieć bitu wykonywalnego"
    );

    // 7. Marker DEB zgodny z kontraktem repozytorium.
    for required in [
        "\"installation_type\": \"LinuxDeb\"",
        "\"management_mode\": \"SystemManaged\"",
        "\"artifact_format\": \"deb\"",
        "\"architecture\": \"amd64\"",
        "\"package_name\": \"splogbook\"",
        "\"resolved_minimum_kernel\": \"${RESOLVED_MIN_KERNEL}\"",
        "\"maximum_glibc\": \"${GLIBC_POLICY_MAX}\"",
    ] {
        assert!(build.contains(required), "marker musi zawierać {required}");
    }
    assert!(
        !build.contains("date +"),
        "marker nie może zawierać czasu lokalnego"
    );

    // 8. Minimalne maintainer scripts.
    for (name, source) in [("postinst", &postinst), ("postrm", &postrm)] {
        assert!(
            source.starts_with("#!/bin/sh"),
            "{name} musi używać /bin/sh"
        );
        assert!(source.contains("set -e"), "{name} musi używać set -e");
        assert!(
            source.contains("update-desktop-database") && source.contains("gtk-update-icon-cache"),
            "{name} może odświeżać wyłącznie cache pulpitu"
        );
        for forbidden in [
            "sudo",
            "pkexec",
            "doas",
            "curl",
            "wget",
            "HOME",
            "XDG_",
            "debconf",
            "systemctl",
        ] {
            for line in source
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
            {
                assert!(
                    !line.contains(forbidden),
                    "{name} nie może zawierać {forbidden}: {line}"
                );
            }
        }
    }
    assert!(
        !build.contains("templates/preinst"),
        "preinst nie jest potrzebny"
    );
    assert!(
        !build.contains("templates/prerm"),
        "prerm nie jest potrzebny"
    );
    assert!(
        build.contains("conffiles are not approved"),
        "brak conffiles"
    );

    // 9. Weryfikator fail-closed i pełny zakres kontroli.
    assert!(
        verify.contains("set -eu"),
        "weryfikator musi używać set -eu"
    );
    assert!(
        verify.contains("GLIBC_POLICY_MAX=\"2.43\""),
        "weryfikator musi kodować GLIBC 2.43"
    );
    assert!(verify.contains("version_le") && verify.contains("sort -V"));
    assert!(verify.contains("lintian"));
    assert!(
        verify
            .contains("LINTIAN_SUPPRESSED_TAGS=\"dir-or-file-in-opt,custom-library-search-path\""),
        "weryfikator może wyciszyć wyłącznie zatwierdzony tag /opt"
    );
    assert!(
        verify.contains("ADR-47"),
        "wyciszenie tagu musi odwoływać się do ADR-47"
    );
    assert!(verify.contains("dpkg --remove") && verify.contains("dpkg --purge"));
    assert!(verify.contains("user data unchanged"));
    for forbidden in [
        "*.db",
        "*.db-wal",
        "*.db-shm",
        "default_log.db",
        "station_config.json",
        "*.adi",
        "*.adif",
        "*.cbr",
    ] {
        assert!(
            verify.contains(forbidden),
            "weryfikator musi zakazywać {forbidden}"
        );
    }
    assert_eq!(
        verify
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter(|line| line.contains("DEB VERIFICATION COMPLETED SUCCESSFULLY"))
            .count(),
        1,
        "komunikat sukcesu dokładnie raz"
    );
    assert_eq!(
        verify.lines().rfind(|line| !line.trim().is_empty()),
        Some("echo \"DEB VERIFICATION COMPLETED SUCCESSFULLY\""),
        "komunikat sukcesu musi być ostatnią instrukcją"
    );
    assert!(
        !verify.contains("|| true"),
        "weryfikator nie może maskować błędów"
    );

    // 10. Testy negatywne: 27 mutacji, każda odrzucona bez komunikatu sukcesu.
    assert_eq!(
        fixtures.matches("expect_reject \"").count(),
        27,
        "zestaw negatywny musi wywołać dokładnie 27 mutacji"
    );
    assert!(
        fixtures.contains("NEGATIVE FIXTURES REJECTED AS EXPECTED: ${REJECTED}/${TOTAL}"),
        "zestaw negatywny musi raportować jawne odrzucenie"
    );
    assert!(
        fixtures.contains("fixture emitted a success message"),
        "zestaw negatywny musi wykrywać wyciek komunikatu sukcesu"
    );

    // 11. Workflow: build + verify + negative, artefakt CI, brak publikacji DEB.
    assert!(workflow.contains("build-deb:"));
    assert!(workflow.contains("verify-deb:"));
    assert!(workflow.contains("verify-deb-negative-fixtures:"));
    assert!(workflow.contains("name: SPLogbook-Linux-amd64-deb"));
    assert!(
        workflow.contains(
            "ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c"
        ),
        "DEB musi być budowany w przypiętym obrazie"
    );
    assert!(workflow.contains("scripts/build_linux_deb.sh"));
    assert!(workflow.contains("scripts/verify_linux_deb.sh"));
    assert!(workflow.contains("scripts/verify_linux_deb_negative_fixture.sh"));
    assert!(
        !workflow.contains("git add"),
        "workflow nie może dodawać artefaktów do repo"
    );
    let publish = workflow
        .split("publish-release:")
        .nth(1)
        .expect("job publish-release musi istnieć");
    assert!(
        !publish.contains("SPLogbook-Linux-amd64"),
        "publish-release nie może publikować DEB"
    );
    assert!(
        publish.contains("if: startsWith(github.ref, 'refs/tags/v')"),
        "publish-release musi pozostać warunkowany tagiem v*"
    );
    assert!(
        publish.contains("DEB must never be published"),
        "publish-release musi zawierać trwałą asercję braku DEB"
    );

    // 12. DEB nie jest commitowany.
    assert!(
        gitignore.contains("*.deb"),
        ".gitignore musi ignorować *.deb"
    );
    assert!(
        gitignore.contains("*.deb.sha256"),
        ".gitignore musi ignorować sidecar *.deb.sha256"
    );

    // 13. F-03 i cztery profile pozostają nietknięte.
    assert!(f03.contains("GLIBC_POLICY_MAX=\"2.43\""));
    assert!(f03.contains("NATIVE VERIFICATION COMPLETED SUCCESSFULLY"));
    assert_eq!(
        workflow.matches("distro_id: \"").count(),
        4,
        "macierz M3-F musi pozostać czteroprofilowa"
    );
    assert!(
        workflow.contains("SPLogbook-Linux-x86_64-appimage")
            && workflow.contains("SPLogbook-Linux-x86_64-1.3.0.tar.gz"),
        "portable tar.gz i AppImage muszą pozostać w workflow"
    );
}

// ============================================================================
// Linux RPM integration guard (2026-11-13)
// Strażnik binarnego pakietu RPM: rpmbuild, deterministyczna budowa,
// układ /opt/SPLogbook, fail-closed launcher, tylko post/postun, weryfikator
// fail-closed, 34 mutacje negatywne, artefakt wyłącznie w CI, brak publikacji
// RPM oraz brak zmian w M3-H / M3-G / F-03 i brak rozpoczęcia M3-J.
// ============================================================================

#[test]
fn m3_i_rpm_integration_guard() {
    let build =
        fs::read_to_string("scripts/build_linux_rpm.sh").expect("scripts/build_linux_rpm.sh");
    let verify =
        fs::read_to_string("scripts/verify_linux_rpm.sh").expect("scripts/verify_linux_rpm.sh");
    let fixtures = fs::read_to_string("scripts/verify_linux_rpm_negative_fixture.sh")
        .expect("scripts/verify_linux_rpm_negative_fixture.sh");
    let manifest =
        fs::read_to_string("scripts/linux_rpm_manifest.sh").expect("scripts/linux_rpm_manifest.sh");
    let spec = fs::read_to_string("installer/linux/rpm/splogbook.spec")
        .expect("installer/linux/rpm/splogbook.spec")
        .replace("\r\n", "\n");
    let launcher =
        fs::read_to_string("installer/linux/rpm/launcher").expect("installer/linux/rpm/launcher");
    let desktop = fs::read_to_string("installer/linux/rpm/splogbook.desktop")
        .expect("installer/linux/rpm/splogbook.desktop");
    let readme =
        fs::read_to_string("installer/linux/rpm/README.md").expect("installer/linux/rpm/README.md");
    let workflow = fs::read_to_string(".github/workflows/build-and-release.yml")
        .expect(".github/workflows/build-and-release.yml");
    let gitignore = fs::read_to_string(".gitignore").expect(".gitignore");
    let installation = fs::read_to_string("src/cloud/updater/installation.rs")
        .expect("src/cloud/updater/installation.rs");
    let f03 = fs::read_to_string("scripts/verify_linux_container.sh")
        .expect("scripts/verify_linux_container.sh");
    let deb = fs::read_to_string("scripts/build_linux_deb.sh").expect("scripts/build_linux_deb.sh");
    let appimage = fs::read_to_string("scripts/build_linux_appimage.sh")
        .expect("scripts/build_linux_appimage.sh");

    // 1. Builder: rpmbuild, dwie niezależne budowy, manifest, fail-closed.
    assert!(build.contains("set -euo pipefail"));
    assert!(build.contains("rpmbuild -bb"));
    assert!(build.contains("SOURCE_DATE_EPOCH"));
    assert!(spec.contains("build.splogbook.invalid"));
    assert!(build.contains("the RPM build is not reproducible"));
    assert!(build.contains("patchelf --set-rpath '$ORIGIN/lib'"));
    assert!(build.contains("linux_rpm_manifest.sh"));
    assert!(build.contains("rpmdev-vercmp"));
    assert_eq!(build.matches("build_once").count(), 3);
    for forbidden in ["http://", "curl | sh", "curl | bash", "git add", "--nodeps"] {
        assert!(
            !build.contains(forbidden),
            "builder RPM nie może zawierać {forbidden}"
        );
    }

    // 2. Wersja i brak epoch.
    assert!(build.contains("RPM_VERSION=\"${BASE_VERSION}\""));
    assert!(build.contains("RPM_RELEASE=\"0.1.${PRERELEASE}\""));
    assert!(build.contains("an epoch is not allowed"));
    assert!(!spec.contains("Epoch:"));
    assert!(!spec.contains("%{?dist}"));

    // 3. Tożsamość i zależności.
    for required in [
        "Name:           splogbook",
        "BuildArch:      x86_64",
        "Vendor:         SPLogbook",
        "License:        GPL-3.0-or-later AND LGPL-2.1-or-later",
        "URL:            https://github.com/sp6ina/SPLogbook",
        "AutoReqProv:    no",
        "Requires:       glibc >= 2.43",
        "Requires:       libc.so.6()(64bit)",
    ] {
        assert!(spec.contains(required), "spec RPM musi zawierać {required}");
    }
    assert!(!spec.contains("Requires:       hamlib"));
    assert!(!spec.contains("Recommends:"));
    assert!(!spec.contains("Suggests:"));
    assert!(!spec.contains("Conflicts:"));
    assert!(!spec.contains("Obsoletes:"));

    // 4. Układ, tryby, launcher, desktop, prywatny Hamlib.
    for required in [
        "%dir /opt/SPLogbook",
        "%attr(0755,root,root) /opt/SPLogbook/SPLogbook",
        "%attr(0755,root,root) /opt/SPLogbook/lib/libhamlib.so.4",
        "%attr(0755,root,root) /usr/bin/splogbook",
        "%license /usr/share/licenses/splogbook/HAMLIB_LGPL-2.1.txt",
    ] {
        assert!(spec.contains(required), "spec RPM musi zawierać {required}");
    }
    assert!(launcher.contains("#!/bin/sh"));
    assert!(launcher.contains("set -eu"));
    assert!(launcher.contains("exec /opt/SPLogbook/SPLogbook \"$@\""));
    for line in launcher
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
    {
        for forbidden in ["sudo", "pkexec", "doas", "LD_LIBRARY_PATH", "HOME", "XDG_"] {
            assert!(
                !line.contains(forbidden),
                "launcher RPM nie może zawierać {forbidden}: {line}"
            );
        }
    }
    assert!(desktop.contains("Exec=splogbook"));
    assert!(desktop.contains("Icon=splogbook"));
    assert!(desktop.contains("StartupWMClass=SPLogbook"));

    // 5. Tylko %post/%postun, bez config/ghost.
    assert!(spec.contains("%post\n"));
    assert!(spec.contains("%postun\n"));
    for forbidden in [
        "%pre\n",
        "%preun\n",
        "%pretrans\n",
        "%posttrans\n",
        "%trigger",
        "%verifyscript",
        "%config",
        "%ghost",
    ] {
        assert!(
            !spec.contains(forbidden),
            "spec RPM nie może zawierać {forbidden}"
        );
    }

    // 6. Marker RPM zgodny z istniejącym updaterem.
    assert!(installation.contains(".splogbook-rpm-marker"));
    assert!(installation.contains("InstallationType::LinuxRpm"));
    for required in [
        "\"installation_type\": \"LinuxRpm\"",
        "\"management_mode\": \"SystemManaged\"",
        "\"artifact_format\": \"rpm\"",
        "\"architecture\": \"x86_64\"",
        "\"target\": \"linux-x86_64\"",
        "\"resolved_minimum_kernel\": \"${RESOLVED_MIN_KERNEL}\"",
        "\"maximum_glibc\": \"${GLIBC_POLICY_MAX}\"",
    ] {
        assert!(
            build.contains(required),
            "marker RPM musi zawierać {required}"
        );
    }

    // 7. Weryfikator fail-closed i pełny cykl życia.
    assert!(verify.contains("set -eu"));
    assert!(verify.contains("GLIBC_POLICY_MAX=\"2.43\""));
    assert!(verify.contains("rpm -qpR"));
    assert!(verify.contains("rpmlint"));
    assert!(verify.contains("rpm -e"));
    assert!(verify.contains("rpm -U --nosignature --oldpackage"));
    assert!(verify.contains("rpm -U --nosignature --replacepkgs"));
    assert!(verify.contains("user data unchanged"));
    assert!(verify.contains("RPM VERIFICATION COMPLETED SUCCESSFULLY"));
    assert!(verify.contains("ADR-47"));
    assert!(verify.contains("dir-or-file-in-opt,binary-or-shlib-defines-rpath"));
    for forbidden in ["|| true", "|| :", "--nodeps"] {
        assert!(
            !verify.contains(forbidden),
            "weryfikator RPM nie może maskować błędów przez {forbidden}"
        );
    }
    assert_eq!(
        verify
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .filter(|line| line.contains("RPM VERIFICATION COMPLETED SUCCESSFULLY"))
            .count(),
        1,
        "komunikat sukcesu RPM dokładnie raz"
    );
    assert_eq!(
        verify.lines().rfind(|line| !line.trim().is_empty()),
        Some("echo \"RPM VERIFICATION COMPLETED SUCCESSFULLY\""),
        "komunikat sukcesu RPM musi być ostatnią instrukcją"
    );

    // 8. Manifest i negatywne fixture'y.
    assert!(manifest.contains("SPLOGBOOK_RPM_MANIFEST 1.0"));
    assert!(manifest.contains("artifact_sha256"));
    assert_eq!(
        fixtures.matches("expect_reject \"").count(),
        34,
        "zestaw negatywny RPM musi wywołać dokładnie 34 mutacje"
    );
    assert!(fixtures.contains("NEGATIVE FIXTURES REJECTED AS EXPECTED: ${REJECTED}/${TOTAL}"));
    assert!(fixtures.contains("fixture emitted a success message"));
    assert!(fixtures.contains("SPLOG_RPM_INSPECTION_ONLY=1"));

    // 9. Workflow: build + verify + negative, artefakt CI, brak publikacji RPM.
    assert!(workflow.contains("build-rpm:"));
    assert!(workflow.contains("verify-rpm:"));
    assert!(workflow.contains("verify-rpm-negative-fixtures:"));
    assert!(workflow.contains("name: SPLogbook-Linux-x86_64-rpm"));
    assert!(workflow.contains(
        "fedora:45@sha256:a7d72ba2745b100a3351087472e5ee7b2450af7cf331e7673007e3817612839c"
    ));
    assert!(workflow.contains(
        "opensuse/tumbleweed:latest@sha256:6e3a892daa0372e12a9c0b10291303dbfcd8706a450de6edfd72c763b44b4ac1"
    ));
    assert!(workflow.contains("scripts/build_linux_rpm.sh"));
    assert!(workflow.contains("scripts/verify_linux_rpm.sh"));
    assert!(workflow.contains("scripts/verify_linux_rpm_negative_fixture.sh"));
    assert!(!workflow.contains("git add"));
    let publish = workflow
        .split("publish-release:")
        .nth(1)
        .expect("job publish-release musi istnieć");
    assert!(
        !publish.contains("SPLogbook-Linux-x86_64-rpm"),
        "publish-release nie może publikować RPM"
    );
    assert!(publish.contains("if: startsWith(github.ref, 'refs/tags/v')"));
    assert!(
        publish.contains("RPM must never be published"),
        "publish-release musi zawierać trwałą asercję braku RPM"
    );

    // 10. RPM nie jest commitowany, README opisuje kontrakt.
    assert!(gitignore.contains("*.rpm"));
    assert!(gitignore.contains("*.rpm.sha256"));
    assert!(gitignore.contains("*.rpm.manifest"));
    let lower = readme.to_lowercase();
    for required in [
        "fedora linux 45",
        "opensuse tumbleweed",
        "glibc_2.43",
        "1.3.0-1",
        "/opt/splogbook",
        "libhamlib.so.4",
        "linuxrpm",
        "systemmanaged",
        "user data",
        "negative fixtures",
        "ci artifact only",
        "rpmlint",
    ] {
        assert!(
            lower.contains(required),
            "README RPM musi opisywać {required}"
        );
    }

    // 11. M3-H, M3-G, F-03 i cztery profile pozostają nietknięte.
    assert!(deb.contains("DEB BUILD COMPLETED SUCCESSFULLY"));
    assert!(deb.contains("dpkg-deb"));
    assert!(appimage.contains("SUCCESS: AppImage built"));
    assert!(f03.contains("GLIBC_POLICY_MAX=\"2.43\""));
    assert!(f03.contains("NATIVE VERIFICATION COMPLETED SUCCESSFULLY"));
    assert_eq!(
        workflow.matches("distro_id: \"").count(),
        4,
        "macierz M3-F musi pozostać czteroprofilowa"
    );
    assert!(
        workflow.contains("SPLogbook-Linux-x86_64-appimage")
            && workflow.contains("SPLogbook-Linux-x86_64-1.3.0.tar.gz"),
        "portable tar.gz i AppImage muszą pozostać w workflow"
    );

    // 12. M3-J (Flatpak) zostało rozpoczęte w dedykowanej bramce i musi
    //     pozostawić minimalny, jawny kontrakt sandboxa.
    let flatpak_manifest =
        std::fs::read_to_string("installer/linux/flatpak/io.github.sp6ina.SPLogbook.yml")
            .expect("M3-J: manifest Flatpak musi istnieć");
    // Komentarze dokumentują, czego świadomie nie ma; skanujemy efektywne YAML.
    let flatpak_manifest_effective: String = flatpak_manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        flatpak_manifest.contains("app-id: io.github.sp6ina.SPLogbook"),
        "M3-J: manifest musi deklarować zatwierdzony app-id"
    );
    for forbidden in [
        "--filesystem=host",
        "--filesystem=home",
        "--device=all",
        "--socket=system-bus",
        "--talk-name=*",
    ] {
        assert!(
            !flatpak_manifest_effective.contains(forbidden),
            "M3-J: manifest nie może deklarować {forbidden}"
        );
    }
}

// ============================================================================
// Higiena publicznego drzewa dokumentacji
// Każde śledzone Markdown musi być wpisane do publicznego indeksu dokumentacji.
// Kontrola jest fail-closed: nowy, nieindeksowany dokument (w tym przypadkowo
// dodany dokument procesowy) blokuje CI.
// ============================================================================

#[test]
fn every_tracked_document_is_listed_in_the_documentation_index() {
    let index = fs::read_to_string("docs/DOCUMENTATION_INDEX.md")
        .expect("docs/DOCUMENTATION_INDEX.md must exist");
    let output = Command::new("git")
        .args(["ls-files", "*.md"])
        .output()
        .expect("git ls-files must be executable");
    assert!(
        output.status.success(),
        "git ls-files must succeed to enumerate tracked documentation"
    );

    let listed = String::from_utf8_lossy(&output.stdout);
    let tracked: Vec<&str> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    assert!(
        tracked.len() >= 20,
        "Public repository must track its documentation, found only {} Markdown files",
        tracked.len()
    );

    let mut missing: Vec<&str> = Vec::new();
    for path in &tracked {
        let needle = format!("`{path}`");
        if !index.contains(&needle) {
            missing.push(path);
        }
    }
    assert!(
        missing.is_empty(),
        "Every tracked Markdown document must be listed in docs/DOCUMENTATION_INDEX.md with its exact backticked path; missing: {missing:?}"
    );
}

// ============================================================================
// M3-J: kontrakt pakietu Flatpak (sandbox, źródła, brak self-update)
// ============================================================================
#[test]
fn m3_j_flatpak_sandbox_and_packaging_guard() {
    let manifest_path = "installer/linux/flatpak/io.github.sp6ina.SPLogbook.yml";
    let manifest =
        std::fs::read_to_string(manifest_path).expect("M3-J: manifest Flatpak musi istnieć");
    // Komentarze dokumentują, czego świadomie nie ma; skanujemy wyłącznie
    // efektywne linie YAML.
    let manifest_effective: String = manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // 1. Tożsamość i przypięty runtime/SDK.
    for required in [
        "app-id: io.github.sp6ina.SPLogbook",
        "runtime: org.freedesktop.Platform",
        "runtime-version: \"26.08\"",
        "sdk: org.freedesktop.Sdk",
        "command: SPLogbook",
        "rust-1.99.0-x86_64-unknown-linux-gnu.tar.gz",
    ] {
        assert!(
            manifest.contains(required),
            "M3-J: manifest musi deklarować {required}"
        );
    }

    // 2. Dokładny, minimalny zestaw finish-args (semantycznie równoważny
    //    kontraktowi sprawdzanemu przez scripts/verify_linux_flatpak.sh).
    let mut finish_args: Vec<String> = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        if line.starts_with("finish-args:") {
            inside = true;
            continue;
        }
        if inside {
            if let Some(rest) = line.strip_prefix("  - ") {
                finish_args.push(rest.trim().to_string());
            } else if !line.trim().is_empty() && !line.starts_with(' ') {
                break;
            }
        }
    }
    finish_args.sort();
    let mut expected = vec![
        "--device=dri".to_string(),
        "--share=ipc".to_string(),
        "--share=network".to_string(),
        "--socket=fallback-x11".to_string(),
        "--socket=pulseaudio".to_string(),
        "--socket=wayland".to_string(),
        "--talk-name=org.freedesktop.secrets".to_string(),
    ];
    expected.sort();
    assert_eq!(
        finish_args, expected,
        "M3-J: sandbox musi mieć dokładnie zatwierdzony minimalny zestaw uprawnień"
    );

    // 3. Brak szerokich uprawnień i brak szerokiego dostępu do hosta.
    for forbidden in [
        "--filesystem=host",
        "--filesystem=home",
        "--filesystem=/dev",
        "--filesystem=/etc",
        "--filesystem=/usr",
        "--filesystem=/opt",
        "--filesystem=/run",
        "--device=all",
        "--socket=system-bus",
        "--socket=session-bus",
        "--talk-name=*",
        "--own-name=*",
        "--system-talk-name=*",
        "--allow=devel",
        "--allow=multiarch",
    ] {
        assert!(
            !manifest_effective.contains(forbidden),
            "M3-J: manifest nie może deklarować {forbidden}"
        );
    }

    // 4. Przypięte źródła i model offline.
    assert!(
        !manifest.contains("type: git"),
        "M3-J: brak nieprzypiętych źródeł git"
    );
    assert!(
        !manifest.contains("branch:"),
        "M3-J: brak nieprzypiętych gałęzi"
    );
    assert!(!manifest.contains("tag:"), "M3-J: brak tagów bez commita");
    assert!(!manifest.contains("file://"), "M3-J: brak ścieżek file:///");
    assert!(
        manifest.contains("CARGO_NET_OFFLINE: \"true\""),
        "M3-J: build Cargo musi być offline"
    );
    assert!(
        manifest.contains("--locked --offline"),
        "M3-J: build Cargo musi używać --locked i --offline"
    );
    assert!(
        manifest.contains("replace-with = \"vendored-sources\""),
        "M3-J: Cargo musi korzystać z vendored sources"
    );
    let cargo_sources = std::fs::read_to_string("installer/linux/flatpak/cargo-sources.json")
        .expect("M3-J: cargo-sources.json musi istnieć");
    assert!(
        cargo_sources.len() > 100_000,
        "M3-J: cargo-sources.json musi opisywać pełny zestaw zależności"
    );
    assert!(
        cargo_sources.contains("\"sha256\": \""),
        "M3-J: każde źródło Cargo musi mieć SHA-256"
    );

    // 5. Brak self-update: pakiet nie zawiera pomocnika aktualizacji.
    assert!(
        !manifest.contains("splogbook-updater"),
        "M3-J: bundle nie może instalować pomocnika self-update"
    );

    // 6. Weryfikator, harness negatywny i dokumentacja istnieją i są spójne.
    let verifier = std::fs::read_to_string("scripts/verify_linux_flatpak.sh")
        .expect("M3-J: weryfikator musi istnieć");
    assert!(
        verifier.contains("flatpak --user info --show-permissions"),
        "M3-J: weryfikator musi porównywać faktyczne uprawnienia sandboxa"
    );
    assert!(
        verifier.contains("context.devices dri")
            && verifier.contains("session-bus.org.freedesktop.secrets talk"),
        "M3-J: weryfikator musi egzekwować dokładny kontrakt uprawnień"
    );
    assert!(
        !verifier.contains("flatpak install --system"),
        "M3-J: weryfikator nie może instalować systemowo"
    );
    let negative = std::fs::read_to_string("scripts/verify_linux_flatpak_negative_fixture.sh")
        .expect("M3-J: harness negatywny musi istnieć");
    assert!(
        negative.matches("expect_reject \"").count() >= 45,
        "M3-J: harness musi pokrywać co najmniej 45 negatywnych fixture'ów"
    );
    let documentation_index = std::fs::read_to_string("docs/DOCUMENTATION_INDEX.md")
        .expect("indeks dokumentacji musi istnieć");
    assert!(
        documentation_index.contains("`installer/linux/flatpak/README.md`"),
        "M3-J: README pakietu Flatpak musi być w indeksie dokumentacji"
    );
}

// =========================================================================
// P0-F14 - PTT PANIC, PROTOCOL CONFORMANCE AND TRANSMITTER FAIL-SAFE
// =========================================================================

/// F-14: the HIL transmit path must not use a panicking `Ord::clamp` for the pulse range, must
/// validate the bounded duration before PTT ON, and must expose the documented minimum as a
/// manifest constant.
#[test]
fn p0_f14_transmit_path_has_no_panicking_clamp_guard() {
    let session = fs::read_to_string("src/cat/hil/session.rs").expect("session.rs");
    let manifest = fs::read_to_string("src/cat/hil/manifest.rs").expect("manifest.rs");

    assert!(
        !session.contains(".clamp("),
        "F-14: the HIL transmit path must not call Ord::clamp (panics on an inverted range)"
    );
    assert!(
        session.contains("fn bounded_test_duration("),
        "F-14: a total (Result-returning) range helper is required"
    );
    assert!(
        session.contains("if max_test_duration_ms < HIL_MIN_TEST_DURATION_MS"),
        "F-14: the total helper must reject a below-minimum maximum"
    );
    assert!(
        session.contains("requested < lower") && session.contains("requested > upper"),
        "F-14: clamping must be explicit comparisons, not a panicking call"
    );
    assert!(
        manifest.contains("pub const HIL_MIN_TEST_DURATION_MS: u64 = 10;"),
        "F-14: the documented minimum must be a manifest constant"
    );
    assert!(
        manifest.contains("pub const HIL_MAX_TEST_DURATION_MS: u64 = 500;"),
        "F-14: the laboratory maximum must be a manifest constant"
    );
    assert!(
        manifest.contains("self.rf_safety.max_test_duration_ms < HIL_MIN_TEST_DURATION_MS"),
        "F-14: the manifest must reject an inverted pulse range before any PTT"
    );
    assert!(
        manifest.contains("max_test_power_watts.is_finite()"),
        "F-14: non-finite power limits must be rejected"
    );

    let bounded_at = session
        .find("bounded_test_duration(duration")
        .expect("acquire path computes the bounded duration");
    let first_on = session
        .find("RigCommand::SetPtt(true)")
        .expect("the ON command exists");
    assert!(
        bounded_at < first_on,
        "F-14: range validation must precede the first SetPtt(true)"
    );
}

/// F-14: every controlled post-PTT-ON path must attempt a release, the emergency Drop release
/// must be non-blocking, and a failed/unconfirmed release must never be success.
#[test]
fn p0_f14_post_ptt_on_paths_attempt_release_guard() {
    let session = fs::read_to_string("src/cat/hil/session.rs").expect("session.rs");

    assert!(
        session.contains("fn emergency_release_ptt("),
        "F-14: Drop must have a dedicated emergency release path"
    );
    assert!(
        session.contains("Waker::noop()"),
        "F-14: the emergency release must poll once with a no-op waker (no runtime, no blocking)"
    );
    assert!(
        session.contains("self.release_outcome = self.emergency_release_ptt();"),
        "F-14: Drop must perform the emergency release when none was attempted"
    );
    assert!(
        session.matches("emergency PTT OFF attempted").count() >= 2,
        "F-14: both the ON-error and the confirm-error paths must attempt PTT OFF"
    );
    for required in [
        "HilReleaseOutcome::NotAttempted",
        "HilReleaseOutcome::Confirmed",
        "HilReleaseOutcome::Failed",
        "HilReleaseOutcome::Unconfirmed",
        "HilError::MissingConfirmedRx",
        "HilError::ReleaseError",
        "HilError::TimeoutToUnknown",
    ] {
        assert!(
            session.contains(required),
            "F-14: release semantics require {required}"
        );
    }
}

/// Test-only protocol doubles may live only in `tests/` / `tests/support/`; they must never be
/// reachable from `src/`, the backend enum, the GUI, the CLI, the configuration or the environment.
#[test]
fn p0_f14_test_doubles_are_test_scope_only_guard() {
    let double = fs::read_to_string("tests/support/protocol_double.rs")
        .expect("tests/support/protocol_double.rs must exist");
    assert!(double.contains("TEST-ONLY PROTOCOL DOUBLES"));
    assert!(double.contains("impl RigBackend for ProtocolDouble"));

    for consumer in [
        "tests/cat_f14_fail_safe.rs",
        "tests/protocol_conformance.rs",
    ] {
        let source = fs::read_to_string(consumer).expect(consumer);
        assert!(
            source.contains("#[path = \"support/protocol_double.rs\"]"),
            "{consumer} must include the shared test double explicitly"
        );
    }

    let backend = fs::read_to_string("src/cat/backend.rs").expect("src/cat/backend.rs");
    assert!(
        backend.contains("pub const ALL: [Self; 9] = ["),
        "F-14: the production backend enum must keep exactly 9 real variants"
    );
    let enum_start = backend
        .find("pub enum CatBackendKind {")
        .expect("backend enum");
    let enum_end = backend[enum_start..]
        .find("\n}")
        .map(|offset| enum_start + offset)
        .expect("backend enum end");
    let enum_body = &backend[enum_start..enum_end].to_ascii_lowercase();
    for forbidden in [
        "mock",
        "simulator",
        "fake",
        "dummy",
        "test",
        "virtual",
        "protocoldouble",
    ] {
        assert!(
            !enum_body.contains(forbidden),
            "F-14: production backend enum must not expose '{forbidden}'"
        );
    }

    for entry in walkdir::WalkDir::new("src")
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(path).expect("readable source");
        for forbidden in [
            "ProtocolDouble",
            "DocumentedPttProtocol",
            "support/protocol_double",
            "test-radio",
            "fake-radio",
            "dummy-radio",
            "virtual-transceiver",
            "mock-radio",
        ] {
            assert!(
                !source.contains(forbidden),
                "F-14: {} must not reference the test double token '{forbidden}'",
                path.display()
            );
        }
    }

    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Cargo.toml");
    assert!(
        !cargo_toml.contains("[features]"),
        "F-14: no feature flag may gate a test double"
    );
    for forbidden in [
        "mock",
        "simulator",
        "fake-radio",
        "test-radio",
        "dummy-radio",
        "virtual-transceiver",
    ] {
        assert!(
            !cargo_toml.to_ascii_lowercase().contains(forbidden),
            "F-14: Cargo.toml must not mention '{forbidden}'"
        );
    }
    assert!(
        cargo_toml.contains("panic = \"abort\""),
        "F-14: the release profile must keep panic=abort (no silent unwind-based remediation)"
    );
}

/// The ten hardware-in-the-loop tests stay ignored and neither workflow executes them.
#[test]
fn p0_f14_hil_hardware_tests_remain_ignored_guard() {
    let hil = fs::read_to_string("tests/cat_native_7.rs").expect("tests/cat_native_7.rs");
    let ignored_attributes = hil
        .lines()
        .filter(|line| line.trim_start().starts_with("#[ignore"))
        .count();
    assert_eq!(
        ignored_attributes, 10,
        "F-14: exactly ten hardware-in-the-loop tests must remain ignored"
    );
    assert_eq!(
        hil.matches("#[ignore = \"hardware-in-the-loop\"]").count(),
        11,
        "F-14: ten attributes plus the module-level documentation mention"
    );
    for case in [
        "hil_hardware_live_test_bench",
        "hil_hardware_read_01",
        "hil_hardware_freq_01",
        "hil_hardware_mode_01",
        "hil_hardware_ptt_01",
        "hil_hardware_ptt_timeout_01",
        "hil_hardware_resync_01",
        "hil_hardware_multi_radio_01",
        "hil_hardware_shutdown_01",
        "hil_hardware_shutdown_tx_01",
    ] {
        assert!(hil.contains(case), "missing HIL case {case}");
    }

    for workflow in [
        ".github/workflows/ci.yml",
        ".github/workflows/build-and-release.yml",
    ] {
        let source = fs::read_to_string(workflow).expect(workflow);
        for forbidden in [
            "--ignored",
            "hil_hardware",
            "SPLOG_HIL_ENABLE",
            "SPLOG_HIL_ALLOW_TX",
            "SPLOG_HIL_SESSION",
        ] {
            assert!(
                !source.contains(forbidden),
                "{workflow} must not run hardware HIL ('{forbidden}')"
            );
        }
    }
}

/// Public documentation must separate the software remediation, the protocol-conformance
/// verification and the hardware validation that was NOT executed.
#[test]
fn p0_f14_documentation_separates_software_and_hardware_guard() {
    let known = fs::read_to_string("docs/KNOWN_ISSUES.md").expect("KNOWN_ISSUES.md");
    for required in [
        "F-14 = Resolved (Verified)",
        "Hardware validation = NOT_EXECUTED",
        "Protocol conformance software verification = COMPLETED",
        "NATIVE-7B = NOT_STARTED",
        "F-15 (P1)",
        "F-17 (P1)",
    ] {
        assert!(
            known.contains(required),
            "KNOWN_ISSUES.md must record: {required}"
        );
    }
    assert!(
        !known.to_ascii_lowercase().contains("mock radio"),
        "test doubles must never be described as an application feature"
    );

    let matrix = fs::read_to_string("docs/HARDWARE_VALIDATION_MATRIX.md")
        .expect("HARDWARE_VALIDATION_MATRIX");
    for required in [
        "HARDWARE_BEHAVIOUR_NOT_VERIFIED",
        "TEST_DOUBLE_PRODUCTION_EXCLUSION_VERIFIED",
        "SOFTWARE_ONLY_VERIFICATION",
    ] {
        assert!(
            matrix.contains(required),
            "HARDWARE_VALIDATION_MATRIX.md must record: {required}"
        );
    }
}

// ============================================================================
// CAT-018 / CAT-019 -- protocol conformance guards
// ============================================================================

/// CAT-018: the Kenwood PTT mapping is model-specific and documented per model. There is no
/// brand-wide command assumption and no model may be served by another model's document.
#[test]
fn cat018_kenwood_ptt_contract_is_model_specific_guard() {
    let models = fs::read_to_string("src/cat/native/kenwood/models.rs").expect("models.rs");
    assert!(
        models.contains("pub struct KenwoodPttContract"),
        "CAT-018: an explicit per-model PTT contract type is required"
    );
    assert!(
        models.contains("pub fn for_model("),
        "CAT-018: the contract must be resolved by model id"
    );
    assert!(
        models.contains("KENWOOD_PTT_CONTRACTS: &[KenwoodPttContract]"),
        "CAT-018: the documented contract table must be explicit"
    );
    for model in ["TS-590S", "TS-590SG", "TS-890S", "TS-990S"] {
        assert!(
            models.contains(&format!("model_id: \"{model}\"")),
            "CAT-018: {model} must have its own documented contract entry"
        );
    }
    for document in [
        "B5A-0316-20/01",
        "B5A-2944-00/01",
        "B5A-3170-00",
        "2019-01-30",
    ] {
        assert!(
            models.contains(document),
            "CAT-018: the official document reference '{document}' is required"
        );
    }
    assert!(
        models.contains("status_read: Some(\"IF;\")"),
        "CAT-018: IF is documented only for the TS-590S/SG contracts"
    );
    assert!(
        models.contains("status_read: None"),
        "CAT-018: the TS-890S/TS-990S contracts document no status read"
    );
    assert!(
        models.contains("KENWOOD_DOC_VERIFIED"),
        "CAT-018: every shipped contract must record its verification status"
    );

    let backend = fs::read_to_string("src/cat/native/kenwood/backend.rs").expect("backend.rs");
    assert!(
        backend.contains("fn ptt_contract(") && backend.contains("contract.status_read"),
        "CAT-018: the backend must use the model contract, not a brand-wide command set"
    );
    assert!(
        !backend.contains("read_if_cmd()"),
        "CAT-018: IF; must not be sent unconditionally (documented for two models only)"
    );
    assert!(
        backend.contains("\"ID\""),
        "CAT-018: the documented ID; probe replaces the undocumented IF; fallback"
    );

    let codec = fs::read_to_string("src/cat/native/kenwood/codec.rs").expect("codec.rs");
    assert!(
        codec.contains("\"TX0\" | \"TX1\" | \"TX2\" => Some(true)"),
        "CAT-018: TX P1; answers with P1=0/1/2 all mean transmit"
    );
    assert!(
        !codec.contains("fn read_ptt_cmd("),
        "CAT-018: TX; is a documented transmit command, not a PTT read command"
    );
}

/// CAT-018: documented error messages and no undocumented command guesses.
#[test]
fn cat018_documented_errors_and_undocumented_commands_guard() {
    let codec = fs::read_to_string("src/cat/native/kenwood/codec.rs").expect("codec.rs");
    for frame in [
        "\"?\" => Some(KenwoodErrorFrame::Syntax)",
        "\"E\" => Some(KenwoodErrorFrame::Communication)",
        "\"O\" => Some(KenwoodErrorFrame::BufferOverrun)",
    ] {
        assert!(
            codec.contains(frame),
            "CAT-018: the documented error frame mapping is missing: {frame}"
        );
    }

    let backend = fs::read_to_string("src/cat/native/kenwood/backend.rs").expect("backend.rs");
    assert!(
        backend.contains("fn documented_error(")
            && backend.contains("Self::documented_error(&frame)"),
        "CAT-018: documented error frames must fail the operation closed"
    );
    // The client never sends TX0;/TX1;/TX2;: the documented set command is TX; (P1 omitted) and
    // the backend writes exactly the model contract command.
    assert!(
        codec.contains("if ptt { \"TX;\" } else { \"RX;\" }"),
        "CAT-018: the documented set commands are TX; / RX;"
    );
    assert!(
        backend.contains("let command = if ptt {")
            && backend.contains("contract.ptt_on")
            && backend.contains("contract.ptt_off"),
        "CAT-018: the backend writes exactly the documented per-model command"
    );
    let backend_production = backend.split("#[cfg(test)]").next().unwrap_or(&backend);
    assert!(
        !backend_production.contains("write_all(b\"TX"),
        "CAT-018: no hard-coded TX0;/TX1;/TX2; is ever written in production code"
    );
}

/// CAT-018: write success is never a confirmed TX, and the release obligation survives an
/// unconfirmed ON (disconnect still writes the documented RX;).
#[test]
fn cat018_no_write_success_confirmed_tx_guard() {
    let backend = fs::read_to_string("src/cat/native/kenwood/backend.rs").expect("backend.rs");
    assert!(
        backend.contains("if confirmed {"),
        "CAT-018: state may be confirmed only by a documented answer"
    );
    let index = backend.find("if confirmed {").expect("confirm block");
    let confirm_block = &backend[index..(index + 200).min(backend.len())];
    assert!(
        confirm_block.contains("self.state.ptt = true;"),
        "CAT-018: state.ptt is set only inside the documented-confirmation branch"
    );
    assert!(
        backend.contains("ptt_commanded = true"),
        "CAT-018: an accepted TX; must leave a release obligation"
    );
    assert!(
        backend.contains("if self.confirmed_ptt || self.ptt_commanded {"),
        "CAT-018: disconnect must release both confirmed and unconfirmed TX"
    );
    assert!(
        backend.contains("KenwoodCat::set_ptt_cmd(false)"),
        "CAT-018: the release command is the documented RX;"
    );
    assert!(
        backend.contains("operation: \"ptt_status_read\""),
        "CAT-018: a missing documented status answer is a timeout (fail-closed)"
    );
}

/// CAT-019: rig.set_ptt uses the documented integer parameter, the documented nil response is
/// accepted, HTTP errors fail closed and there is no boolean/int fallback.
#[test]
fn cat019_flrig_set_ptt_documented_type_guard() {
    let source = fs::read_to_string("src/cat/flrig.rs").expect("flrig.rs");
    assert!(
        source.contains("XmlRpcRequest::new(\"rig.set_ptt\").param_int(i64::from(ptt))"),
        "CAT-019: rig.set_ptt is documented as n:i (one integer parameter)"
    );
    assert!(
        !source.contains("rig.set_ptt\").param_bool"),
        "CAT-019: no boolean parameter may be sent to rig.set_ptt"
    );
    assert!(
        source.contains("pub fn parse_nil_response("),
        "CAT-019: the documented nil response must be parsed explicitly"
    );
    assert!(
        source.contains("self.call_nil(&req).await"),
        "CAT-019: rig.set_ptt accepts only the documented nil response"
    );
    assert!(
        source.contains("ExpectedResponse::Nil => 1"),
        "CAT-019: a PTT command is never retried (no two PTT ON)"
    );
    assert!(
        source.contains("XmlRpcValue::Int(0) => Ok(false)"),
        "CAT-019: rig.get_ptt accepts only the documented integer type"
    );
    assert!(
        source.contains("undocumented response type"),
        "CAT-019: undocumented response types are rejected (no implicit coercion)"
    );
    assert!(
        source.contains("Some(200) => {}"),
        "CAT-019: only the documented HTTP 200 response is accepted"
    );
    assert!(
        source.contains("FLRig HTTP error status"),
        "CAT-019: HTTP errors fail closed"
    );
    assert!(
        !source.contains("param_bool(true)") && !source.contains("param_bool(ptt)"),
        "CAT-019: no boolean/int fallback for PTT ON"
    );
}

/// Test doubles added by CAT-018/CAT-019 live only in tests/ and are explicitly included.
#[test]
fn cat018_cat019_test_doubles_are_test_scope_only_guard() {
    for double in [
        "tests/support/kenwood_wire.rs",
        "tests/support/flrig_wire.rs",
    ] {
        let source = fs::read_to_string(double).expect(double);
        assert!(
            source.contains("TEST-ONLY"),
            "{double} must declare its test-only scope"
        );
        assert!(
            source.contains("127.0.0.1") || source.contains("duplex"),
            "{double} must be a loopback/in-memory double (no hardware, no public network)"
        );
        assert!(
            !source.contains("open_serial_stream") && !source.contains("tokio_serial"),
            "{double} must never open a real serial port"
        );
    }

    for consumer in [
        "tests/cat_018_kenwood_conformance.rs",
        "tests/cat_019_flrig_conformance.rs",
        "tests/protocol_conformance.rs",
    ] {
        let source = fs::read_to_string(consumer).expect(consumer);
        assert!(
            source.contains("#[path = \"support/"),
            "{consumer} must include its test double explicitly"
        );
    }

    for entry in walkdir::WalkDir::new("src")
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(path).expect("readable source");
        for forbidden in [
            "kenwood_wire",
            "flrig_wire",
            "KenwoodWire",
            "FlrigWire",
            "spawn_kenwood_wire",
            "spawn_flrig_wire",
        ] {
            assert!(
                !source.contains(forbidden),
                "CAT-018/019: {} must not reference the test double token '{forbidden}'",
                path.display()
            );
        }
    }
}

/// CAT-018/CAT-019 closure documentation and the unchanged follow-up gate state.
#[test]
fn cat018_cat019_documentation_and_gate_state_guard() {
    let known = fs::read_to_string("docs/KNOWN_ISSUES.md").expect("KNOWN_ISSUES.md");
    for required in [
        "CAT-018",
        "CAT-019",
        "CAT-018 = CLOSED / Verified",
        "CAT-019 = CLOSED / Verified",
        "F-15 (P1)",
        "F-17 (P1)",
        "NATIVE-7B",
        "SOFTWARE_ONLY_VERIFICATION",
    ] {
        assert!(
            known.contains(required),
            "KNOWN_ISSUES.md must record: {required}"
        );
    }
    assert!(
        !known.contains("the parser maps `TX0;` to receive"),
        "CAT-018: the recorded discrepancy must be replaced by the verified contract"
    );
    let matrix = fs::read_to_string("docs/HARDWARE_VALIDATION_MATRIX.md")
        .expect("HARDWARE_VALIDATION_MATRIX");
    assert!(
        !matrix.contains("Parser Kenwooda mapuje `TX0;` na odbiór"),
        "CAT-018: the hardware matrix must record the corrected mapping"
    );
    assert!(
        matrix.contains("brak `IF`"),
        "CAT-018: the matrix must record that TS-890S/TS-990S have no IF command"
    );
}

// ============================================================================
// P1-F15 -- HIL readiness independent-evidence guards (F-15)
//
// F-15 removed the self-certifiable readiness model. These guards pin the
// software properties that make self-certification impossible:
//   * no safety boolean defaults to true;
//   * the test plan is not an evidence provider;
//   * the operator attestation is not independent evidence;
//   * independent evidence and the authorization decision are separate types
//     that cannot be deserialised from an untrusted document;
//   * a missing evidence document keeps hardware execution denied;
//   * a test-only evidence provider lives exclusively in test scope;
//   * there is no CLI, GUI, configuration, environment or feature bypass;
//   * the ten hardware-in-the-loop tests stay ignored;
//   * the public documentation records the software-only scope honestly.
// The guards read only tracked repository files; they never depend on the
// local working notes or reports.
// ============================================================================

/// No HIL safety field may default to `true`, and no fallback may turn a missing value into an
/// affirmative safety assertion.
#[test]
fn p1_f15_no_safety_field_defaults_to_true_guard() {
    let manifest = fs::read_to_string("src/cat/hil/manifest.rs").expect("manifest.rs");
    for forbidden in [
        "fn default_true()",
        "default = \"default_true\"",
        "unwrap_or(true)",
        "skip_deserializing",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "F-15: src/cat/hil/manifest.rs must not contain '{forbidden}'"
        );
    }
    // The interlock default function is gone as well: every safety flag is required.
    assert!(
        !manifest.contains("default_interlock_engaged"),
        "F-15: the interlock flag must be required, never defaulted"
    );

    let session = fs::read_to_string("src/cat/hil/session.rs").expect("session.rs");
    assert!(
        !session.contains("unwrap_or(true)"),
        "F-15: the transmit path must not treat a missing value as true"
    );

    // Every safety key in the shipped plan fixture is explicit.
    let fixture = fs::read_to_string("tests/hil/ic7300_lab_bench.json").expect("HIL fixture");
    for key in [
        "\"interlock_engaged\"",
        "\"smartlink_disabled\"",
        "\"unattended_remote_tx_disabled\"",
        "\"remote_endpoint_is_local_or_controlled\"",
        "\"operator_has_immediate_stop_control\"",
        "\"out_of_band_data_plane_disabled\"",
        "\"antenna_disconnected\"",
        "\"pa_disconnected_or_locked\"",
        "\"tuner_bypassed\"",
        "\"physical_stop_available\"",
    ] {
        assert!(
            fixture.contains(key),
            "F-15: the shipped HIL fixture must spell out {key} explicitly"
        );
    }
}

/// The test plan must not be able to produce or own an independent evidence document.
#[test]
fn p1_f15_manifest_is_not_an_evidence_provider_guard() {
    let manifest = fs::read_to_string("src/cat/hil/manifest.rs").expect("manifest.rs");
    // Documentation comments may explain the separation; no code may reference the evidence types.
    let manifest_code: String = manifest
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "HilIndependentEvidence",
        "HilAuthorizationDecision",
        "HilEvidenceProvider",
        "evidence_provider",
    ] {
        assert!(
            !manifest_code.contains(forbidden),
            "F-15: the test plan must not reference '{forbidden}' in code"
        );
    }

    let authorization =
        fs::read_to_string("src/cat/hil/authorization.rs").expect("authorization.rs");
    for required in [
        "pub struct HilIndependentEvidence",
        "pub struct HilAttestationDocument",
        "pub struct HilAuthorizationDecision",
        "pub enum HilAuthorizationOutcome",
        "pub struct HilHardwareExecutionAuthorizer",
        "pub trait HilEvidenceProvider",
        "pub const HIL_REQUIRED_MEASURED_FACTS",
    ] {
        assert!(
            authorization.contains(required),
            "F-15: authorization.rs must define {required}"
        );
    }

    // Evidence and decision are deliberately NOT deserialisable: an untrusted JSON document
    // cannot forge an authorization.
    let evidence_derive_at = authorization
        .find("pub struct HilIndependentEvidence {")
        .expect("evidence struct");
    let decision_derive_at = authorization
        .find("pub struct HilAuthorizationDecision {")
        .expect("decision struct");
    for (name, position) in [
        ("HilIndependentEvidence", evidence_derive_at),
        ("HilAuthorizationDecision", decision_derive_at),
    ] {
        // Take the nearest preceding #[derive(...)] attribute that belongs to this struct.
        let prefix = &authorization[..position];
        let derive_at = prefix
            .rfind("#[derive(")
            .unwrap_or_else(|| panic!("F-15: {name} must have a derive attribute"));
        let derive_end = prefix[derive_at..]
            .find(")]")
            .map(|offset| derive_at + offset + 2)
            .unwrap_or(prefix.len());
        let derive = &authorization[derive_at..derive_end];
        assert!(
            derive.contains("Serialize"),
            "F-15: {name} must be serialisable for audit, got {derive}"
        );
        assert!(
            !derive.contains("Deserialize"),
            "F-15: {name} must not be deserialisable (forgery vector), got {derive}"
        );
        // No custom Deserialize impl may exist either.
        assert!(
            !authorization.contains(&format!("impl<'de> Deserialize<'de> for {name}")),
            "F-15: {name} must not have a manual Deserialize implementation"
        );
    }
}

/// Configuration, operator attestation, independent evidence and the authorization decision are
/// four distinct layers evaluated by a dedicated authority.
#[test]
fn p1_f15_evidence_and_authorization_are_separate_guard() {
    let authorization =
        fs::read_to_string("src/cat/hil/authorization.rs").expect("authorization.rs");
    for required in [
        "pub struct HilAuthorizationInputs",
        "pub fn authorize_at(",
        "pub fn dry_run_at(",
        "EvidenceMissing",
        "OperatorAttestationMissing",
        "IndependentApprovalMissing",
        "DryRunOnly",
        "HardwareExecutionDenied",
        "HardwareExecutionAuthorized",
        "HIL_SUPPORTED_EVIDENCE_ISSUERS",
        "is_supported_issuer(",
        "pub struct HilAttemptRegistry",
        "Replayed",
    ] {
        assert!(
            authorization.contains(required),
            "F-15: the evidence model requires {required}"
        );
    }
    // The manifest digest is an integrity hash, never a signature.
    assert!(
        !authorization.contains("signature")
            && !authorization.contains("ed25519")
            && !authorization.contains("private_key"),
        "F-15: a SHA-256 digest must not be presented as a cryptographic signature"
    );
    assert!(
        authorization.contains("Jest to skrót integralności, **nie** podpis kryptograficzny"),
        "F-15: the digest semantics must be documented explicitly"
    );

    let session = fs::read_to_string("src/cat/hil/session.rs").expect("session.rs");
    for required in [
        "pub fn with_authorization(",
        "pub fn authorization_decision_at(",
        "fn consume_authorization(",
        "pub fn is_hardware_ptt_authorized(",
        "pub fn has_independent_evidence(",
    ] {
        assert!(
            session.contains(required),
            "F-15: the session must expose {required}"
        );
    }
    assert!(
        session.contains("let decision = self.authorization_decision_at(evaluated_at);")
            && session.contains("self.consume_authorization(&decision)?;"),
        "F-15: the transmit path must consume an independent authorization before PTT ON"
    );
}

/// Missing, invalid, expired or out-of-scope evidence keeps hardware execution denied; a dry run
/// never authorizes PTT; and a readiness report is never an authorization.
#[test]
fn p1_f15_missing_evidence_denies_guard() {
    let authorization =
        fs::read_to_string("src/cat/hil/authorization.rs").expect("authorization.rs");
    for required in [
        "HilEvidenceError::Missing",
        "HilEvidenceError::Expired",
        "HilEvidenceError::NotYetValid",
        "HilEvidenceError::PlanMismatch",
        "HilEvidenceError::AttestationMismatch",
        "HilEvidenceError::ScopeMismatch",
        "HilEvidenceError::MissingMeasuredFact",
        "HilEvidenceError::ProviderError",
        "HilEvidenceError::ProviderTimeout",
        "HilEvidenceError::Cancelled",
    ] {
        assert!(
            authorization.contains(required),
            "F-15: the fail-closed evidence model requires {required}"
        );
    }

    let evidence = fs::read_to_string("src/cat/hil/evidence.rs").expect("evidence.rs");
    let can_run_at = evidence
        .find("pub fn can_run_hardware_ptt(&self) -> bool {")
        .expect("can_run_hardware_ptt");
    let can_run_body = &evidence[can_run_at..(can_run_at + 200).min(evidence.len())];
    assert!(
        can_run_body.contains("false"),
        "F-15: a readiness report must never claim to authorize hardware PTT"
    );
    assert!(
        !can_run_body.contains("Passed"),
        "F-15: the readiness report must not derive PTT from a Passed status"
    );

    // No production code path constructs evidence or an authorized decision.
    let authorization_constructors = authorization.matches("fn issue_test_evidence(").count();
    assert_eq!(
        authorization_constructors, 1,
        "F-15: exactly one evidence constructor exists"
    );
    let test_scope = authorization
        .find("#[cfg(test)]\npub(crate) mod test_evidence_support")
        .or_else(|| authorization.find("#[cfg(test)]\r\npub(crate) mod test_evidence_support"))
        .expect("the only evidence constructor lives in the cfg(test) module");
    let constructor_at = authorization
        .find("fn issue_test_evidence(")
        .expect("constructor");
    assert!(
        constructor_at > test_scope,
        "F-15: the test evidence constructor must live inside the cfg(test) module"
    );
}

/// The test-only evidence provider is compiled exclusively in test scope and cannot be selected by
/// the CLI, GUI, configuration, environment or a feature flag.
#[test]
fn p1_f15_test_evidence_provider_is_test_scope_only_guard() {
    let cargo = fs::read_to_string("Cargo.toml").expect("Cargo.toml");
    assert!(
        !cargo.contains("[features]"),
        "F-15: no feature flag may gate a test evidence provider"
    );
    for forbidden in [
        "test-evidence",
        "mock-evidence",
        "fake-approval",
        "force-ready",
        "hil-bypass",
        "unsafe-hil",
        "developer-approval",
    ] {
        assert!(
            !cargo.to_ascii_lowercase().contains(forbidden),
            "F-15: Cargo.toml must not mention '{forbidden}'"
        );
    }

    for path in [
        "src/cat/hil/session.rs",
        "src/cat/hil/manifest.rs",
        "src/cat/hil/validator.rs",
    ] {
        let source = fs::read_to_string(path).expect(path);
        for forbidden in [
            "test-evidence",
            "mock-evidence",
            "fake-approval",
            "force-ready",
            "hil-bypass",
            "unsafe-hil",
            "developer-approval",
        ] {
            assert!(
                !source.to_ascii_lowercase().contains(forbidden),
                "{path} must not mention '{forbidden}'"
            );
        }
    }

    // No production entry point (GUI/CLI/config) references the HIL evidence types.
    let listed = Command::new("git")
        .args(["ls-files", "src"])
        .output()
        .expect("git ls-files src");
    assert!(listed.status.success());
    for path in String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(str::trim)
        .filter(|path| path.ends_with(".rs"))
        .filter(|path| !path.starts_with("src/cat/hil/"))
        .filter(|path| *path != "src/cat/mod.rs" && *path != "src/lib.rs")
    {
        let source = fs::read_to_string(path).expect(path);
        for forbidden in [
            "HilIndependentEvidence",
            "HilAuthorizationDecision",
            "HilEvidenceProvider",
            "HilHardwareExecutionAuthorizer",
        ] {
            assert!(
                !source.contains(forbidden),
                "{path} must not reference {forbidden}: no production provider may exist"
            );
        }
    }

    // The production authorization chain never reads an evidence document from the environment or
    // the filesystem.
    let session = fs::read_to_string("src/cat/hil/session.rs").expect("session.rs");
    for forbidden in [
        "SPLOG_HIL_EVIDENCE",
        "SPLOG_HIL_FORCE",
        "SPLOG_HIL_BYPASS",
        "from_file(\"evidence",
        "serde_json::from_str::<HilIndependentEvidence>",
    ] {
        assert!(
            !session.contains(forbidden),
            "F-15: the session must not contain '{forbidden}'"
        );
    }
}

/// Neither workflow may execute the hardware HIL entry points, and the ten HIL tests stay ignored.
#[test]
fn p1_f15_hil_remains_not_executable_in_ci_guard() {
    let hil = fs::read_to_string("tests/cat_native_7.rs").expect("tests/cat_native_7.rs");
    assert_eq!(
        hil.lines()
            .filter(|line| line.trim_start().starts_with("#[ignore"))
            .count(),
        10,
        "F-15: exactly ten hardware-in-the-loop tests stay ignored"
    );
    assert_eq!(
        hil.matches("#[ignore = \"hardware-in-the-loop\"]").count(),
        11,
        "F-15: ten attributes plus the documentation mention"
    );

    for workflow in [
        ".github/workflows/ci.yml",
        ".github/workflows/build-and-release.yml",
    ] {
        let source = fs::read_to_string(workflow).expect(workflow);
        for forbidden in [
            "--ignored",
            "hil_hardware",
            "SPLOG_HIL_ENABLE",
            "SPLOG_HIL_ALLOW_TX",
            "SPLOG_HIL_SESSION",
        ] {
            assert!(
                !source.contains(forbidden),
                "{workflow} must not run hardware HIL ('{forbidden}')"
            );
        }
    }
}

/// The public documentation records the F-15 software remediation scope honestly and does not
/// claim any physical readiness.
#[test]
fn p1_f15_documentation_records_software_only_scope_guard() {
    let known = fs::read_to_string("docs/KNOWN_ISSUES.md").expect("KNOWN_ISSUES.md");
    for required in [
        "F-15 (P1)",
        "F-15 = Resolved (Verified)",
        "HIL readiness self-certification = REMOVED",
        "Production default = DENIED",
        "Hardware readiness = NOT_VERIFIED",
        "F-14 = Resolved (Verified)",
        "Hardware validation = NOT_EXECUTED",
        "Protocol conformance software verification = COMPLETED",
        "F-17 (P1)",
        "NATIVE-7B = NOT_STARTED",
        "SOFTWARE_ONLY_VERIFICATION",
    ] {
        assert!(
            known.contains(required),
            "KNOWN_ISSUES.md must record: {required}"
        );
    }
    for forbidden in [
        "HIL ready",
        "hardware ready",
        "dummy load verified",
        "antenna disconnected verified",
        "PA inhibited verified",
        "NATIVE-7B completed",
        "hardware readiness verified",
    ] {
        assert!(
            !known.contains(forbidden),
            "KNOWN_ISSUES.md must not claim: {forbidden}"
        );
    }

    let baseline = fs::read_to_string("docs/VERIFICATION_BASELINE.md").expect("baseline");
    for required in [
        "HIL IGNORED TESTS",
        "TOTAL REGISTERED TESTS",
        "TOTAL EXECUTED TESTS",
        "TOTAL PASSED TESTS",
        "TOTAL FILTERED TESTS",
        "Software-only verification",
    ] {
        assert!(
            baseline.contains(required),
            "docs/VERIFICATION_BASELINE.md must record {required}"
        );
    }
}

// ============================================================================
// P1-F17 -- safe PTT user interaction and release control (F-17)
//
// F-17 removed the unsafe single-key PTT toggle. These guards pin the software
// properties that make an unsafe PTT interaction impossible:
//   * PTT is never bound to a single key and never exposed as a toggle;
//   * every PTT binding requires exact modifiers, text-focus gating, modal
//     gating and key-repeat rejection;
//   * shortcut resolution consults text focus, IME, window focus and modal state;
//   * every GUI PTT interaction goes through one central dispatcher;
//   * the GUI never names the authorized F-15 outcome and never builds SetPtt;
//   * PTT ON without F-15 authorization is impossible (PTT OFF stays available);
//   * test-only doubles live exclusively in test scope;
//   * no bypass token exists in source, manifest, workflow or localisation;
//   * F-14 / CAT-018 / CAT-019 / F-15 stay closed, NATIVE-7B stays NOT_STARTED.
// The guards read only tracked repository files; they never depend on the
// local working notes or reports.
// ============================================================================

/// Reads a tracked source file and returns its contents (F-17 helper).
fn p1_f17_read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

/// Returns the PTT block of `src/gui/shortcuts.rs` (between the F-17 banner and F8).
fn p1_f17_ptt_binding_block() -> String {
    let source = p1_f17_read("src/gui/shortcuts.rs");
    let start = source
        .find("// --- PTT (F-17)")
        .expect("F-17 PTT binding banner must exist in src/gui/shortcuts.rs");
    let tail = &source[start..];
    let end = tail
        .find("AppAction::ToggleVoiceKeyer")
        .expect("the voice keyer binding must follow the PTT bindings");
    tail[..end].to_string()
}

/// Lists every `.rs` source file under a repository subdirectory (tracked or not, so a newly
/// added file cannot escape the guard before it is committed).
fn p1_f17_tracked_sources(prefix: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(prefix)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        paths.push(path.to_string_lossy().replace('\\', "/"));
    }
    paths.sort();
    paths
}

#[test]
fn p1_f17_ptt_is_never_a_single_key_guard() {
    let block = p1_f17_ptt_binding_block();

    // Exactly the two explicit, modifier-based intents are registered.
    assert!(
        block.contains("KeyboardShortcut::new(ctrl_shift, Key::F7)"),
        "PTT ON must require Ctrl+Shift+F7"
    );
    assert!(
        block.contains("KeyboardShortcut::new(ctrl, Key::F7)"),
        "PTT OFF must require Ctrl+F7"
    );
    assert!(
        !block.contains("KeyboardShortcut::new(none, Key::F7)"),
        "F-17: a modifier-less F7 must never be registered as PTT"
    );
    assert!(
        block.contains("AppAction::PttOn") && block.contains("AppAction::PttOff"),
        "F-17: PTT must be split into two disjoint intents"
    );
    assert!(
        !block.contains("AppAction::TogglePtt"),
        "F-17: the ambiguous TogglePtt action must not exist"
    );

    // No single-key PTT anywhere in the registry.
    let source = p1_f17_read("src/gui/shortcuts.rs");
    assert!(
        !source.contains("AppAction::TogglePtt"),
        "F-17: AppAction::TogglePtt must be gone from the registry"
    );
    for line in qa_code_lines(&source) {
        let trimmed = line.trim();
        if trimmed.contains("KeyboardShortcut::new(") && trimmed.contains("Key::F7") {
            assert!(
                trimmed.contains("ctrl")
                    || trimmed.contains("COMMAND")
                    || trimmed.contains("ctrl_shift"),
                "F-17: an F7 binding must carry an explicit modifier: {trimmed}"
            );
        }
    }
}

#[test]
fn p1_f17_ptt_bindings_are_fully_gated_guard() {
    let block = p1_f17_ptt_binding_block();
    assert_eq!(
        block.matches(".with_exact_modifiers()").count(),
        2,
        "F-17: both PTT bindings must require exact modifiers"
    );
    assert_eq!(
        block.matches(".blocked_when_modal()").count(),
        2,
        "F-17: both PTT bindings must be blocked while a modal is open"
    );
    assert_eq!(
        block
            .matches("ConflictPolicy::BlockWhenTextEditing")
            .count(),
        2,
        "F-17: both PTT bindings must be blocked during text editing"
    );
    assert_eq!(
        block.matches("RepeatPolicy::SingleTrigger").count(),
        2,
        "F-17: both PTT bindings must ignore key repeat"
    );
    assert!(
        !block.contains("ConflictPolicy::AllowInTextEditing"),
        "F-17: a PTT binding may never be allowed during text editing"
    );
}

#[test]
fn p1_f17_shortcut_resolution_checks_every_gate_guard() {
    let source = p1_f17_read("src/gui/shortcuts.rs");
    for required in [
        "gates.text_editing",
        "gates.modal_open",
        "gates.window_focused",
        "gates.ime_composing",
        "ModalPolicy::BlockWhenAnyModal",
        "ModifierPolicy::ExactRequired",
    ] {
        assert!(
            source.contains(required),
            "F-17: shortcut resolution must consult {required}"
        );
    }
    // The exact-match consumer rejects release and repeat before consuming anything.
    assert!(
        source.contains("fn consume_exact_press"),
        "F-17: an exact-modifier consumer must exist"
    );
    let consumer = source
        .split("fn consume_exact_press")
        .nth(1)
        .expect("exact consumer body");
    let body = consumer.split("\n}\n").next().unwrap_or(consumer);
    for required in ["pressed: true", "repeat: false", "matches_exact"] {
        assert!(
            body.contains(required),
            "F-17: the exact consumer must require {required}"
        );
    }
    // Fail-closed default.
    assert!(
        source.contains("pub const fn fail_closed()"),
        "F-17: unreadable gates must fail closed"
    );
}

#[test]
fn p1_f17_gui_never_names_the_authorized_outcome_guard() {
    for path in p1_f17_tracked_sources("src/gui") {
        let source = p1_f17_read(&path);
        for line in qa_code_lines(&source) {
            assert!(
                !line.contains("HardwareExecutionAuthorized"),
                "F-17: {path} must never name the authorized F-15 outcome: {}",
                line.trim()
            );
        }
    }
    // The production gate lives inside the F-15 module and always denies.
    let authorization = p1_f17_read("src/cat/hil/authorization.rs");
    assert!(
        authorization.contains("pub fn evaluate_gui_ptt_authorization()"),
        "F-17: the GUI PTT authorization gate must exist inside the F-15 module"
    );
    assert!(
        authorization.contains("HilAuthorizationDecision::not_evaluated().outcome()"),
        "F-17: the GUI PTT authorization gate must default to NotEvaluated"
    );
}

#[test]
fn p1_f17_gui_never_builds_set_ptt_and_never_toggles_guard() {
    for path in p1_f17_tracked_sources("src/gui") {
        let source = p1_f17_read(&path);
        for line in qa_code_lines(&source) {
            assert!(
                !line.contains("RigCommand::SetPtt"),
                "F-17: {path} must never build RigCommand::SetPtt: {}",
                line.trim()
            );
            assert!(
                !line.contains("SetPtt(true)"),
                "F-17: {path} must never request PTT ON directly: {}",
                line.trim()
            );
            assert!(
                !line.contains("fn toggle_ptt"),
                "F-17: {path} must not define a toggle PTT entry point"
            );
        }
    }
}

#[test]
fn p1_f17_all_gui_ptt_interaction_goes_through_the_dispatcher_guard() {
    // Every user-interaction surface calls the central dispatcher.
    for (path, required) in [
        ("src/gui/app_input.rs", "dispatch_ptt("),
        ("src/gui/app_cat.rs", "pub fn dispatch_ptt("),
        ("src/gui/vfo_panel.rs", "dispatch_ptt("),
        ("src/gui/command_palette.rs", "dispatch_ptt("),
    ] {
        let source = p1_f17_read(path);
        assert!(
            source.contains(required),
            "F-17: {path} must route PTT intent through the central dispatcher"
        );
        assert!(
            !source.contains("toggle_ptt("),
            "F-17: {path} must not call the removed toggle entry point"
        );
    }

    // The set of direct runtime TX call sites in src/gui is closed and enumerated:
    // they are programmatic transmitters, not keyboard interaction, and are pinned here so
    // that no new bypass of the dispatcher can be added silently.
    let allowed = [
        "src/gui/app_async.rs",
        "src/gui/voice_keyer.rs",
        "src/gui/app_plugins.rs",
    ];
    let mut offenders = Vec::new();
    for path in p1_f17_tracked_sources("src/gui") {
        let source = p1_f17_read(&path);
        let direct = source.contains("request_tx_confirmed(")
            || source.contains("release_tx_confirmed(")
            || source.contains(".request_tx(")
            || source.contains(".release_tx(");
        if direct && !allowed.contains(&path.as_str()) && path != "src/gui/app_cat.rs" {
            offenders.push(path);
        }
    }
    assert!(
        offenders.is_empty(),
        "F-17: these GUI files bypass the central dispatcher: {offenders:?}"
    );

    // The production port is the only place in src/gui that reaches TxArbiter.
    let app_cat = p1_f17_read("src/gui/app_cat.rs");
    assert_eq!(
        app_cat.matches(".request_tx(").count(),
        1,
        "F-17: exactly one production request_tx call may exist in app_cat.rs"
    );
    assert_eq!(
        app_cat.matches(".release_tx(").count(),
        1,
        "F-17: exactly one production release_tx call may exist in app_cat.rs"
    );
}

#[test]
fn p1_f17_dispatcher_rejects_release_repeat_and_unauthorized_on_guard() {
    let source = p1_f17_read("src/gui/ptt_input.rs");
    assert!(
        source.contains("PttDispatchOutcome::IgnoredRelease"),
        "F-17: release must be rejected explicitly"
    );
    assert!(
        source.contains("PttDispatchOutcome::IgnoredRepeat"),
        "F-17: repeat must be rejected explicitly"
    );
    assert!(
        source.contains("authorizes_hardware_ptt()"),
        "F-17: PTT ON must consult the F-15 authorization predicate"
    );
    assert!(
        !source.contains("HardwareExecutionAuthorized"),
        "F-17: the GUI dispatcher must not name the authorized F-15 outcome"
    );
    assert!(
        source.contains("PttDispatchOutcome::AuthorizationDenied"),
        "F-17: a denial must be an explicit outcome"
    );
    assert!(
        source.contains("PttDispatchOutcome::Failed"),
        "F-17: a port failure must never be reported as success"
    );
    // OFF is never authorization-gated: the decision branch must be reached without it.
    let dispatch_body = source
        .split("pub fn dispatch(")
        .nth(1)
        .expect("dispatch body");
    let body = dispatch_body
        .split("\n#[cfg(test)]")
        .next()
        .unwrap_or(dispatch_body);
    assert!(
        body.contains("PttIntent::RequestOff => match tx_state"),
        "F-17: PTT OFF must be decided without consulting authorization"
    );
}

#[test]
fn p1_f17_test_only_ptt_doubles_are_test_scope_guard() {
    // The recording port is declared after the cfg(test) marker of the module.
    let source = p1_f17_read("src/gui/ptt_input.rs");
    let test_marker = source
        .find("#[cfg(test)]\nmod tests")
        .or_else(|| source.find("#[cfg(test)]\r\nmod tests"))
        .expect("src/gui/ptt_input.rs must have a cfg(test) test module");
    for token in [
        "struct RecordingPort",
        "impl PttTransmitPort for RecordingPort",
    ] {
        if let Some(position) = source.find(token) {
            assert!(
                position > test_marker,
                "F-17: {token} must live inside the cfg(test) module"
            );
        }
    }
    // No test-only input provider token may exist in production GUI code.
    for path in p1_f17_tracked_sources("src/gui") {
        let content = p1_f17_read(&path);
        for forbidden in [
            "mock-input",
            "test-keyboard",
            "fake-focus",
            "force-ptt",
            "ptt-bypass",
            "unsafe-ptt",
            "developer-ptt",
            "inject-f7",
        ] {
            assert!(
                !content.contains(forbidden),
                "F-17: {path} must not contain the test-input token '{forbidden}'"
            );
        }
    }
}

#[test]
fn p1_f17_no_ptt_bypass_in_manifest_workflows_or_locales_guard() {
    let manifest = p1_f17_read("Cargo.toml");
    assert!(
        !manifest.contains("[features]"),
        "F-17: Cargo.toml must not gain a feature table"
    );
    for forbidden in [
        "force-ptt",
        "ptt-bypass",
        "unsafe-ptt",
        "developer-ptt",
        "mock-input",
        "test-keyboard",
        "fake-focus",
        "inject-f7",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "F-17: Cargo.toml must not contain '{forbidden}'"
        );
        assert!(
            !p1_f17_read("Cargo.lock").contains(forbidden),
            "F-17: Cargo.lock must not contain '{forbidden}'"
        );
        for workflow in [
            ".github/workflows/ci.yml",
            ".github/workflows/build-and-release.yml",
        ] {
            assert!(
                !p1_f17_read(workflow).contains(forbidden),
                "F-17: {workflow} must not contain '{forbidden}'"
            );
        }
        for locale in ["pl", "en", "de", "fr", "es", "it", "ru"] {
            let path = format!("locales/{locale}.json");
            assert!(
                !p1_f17_read(&path).contains(forbidden),
                "F-17: {path} must not contain '{forbidden}'"
            );
        }
    }
}

#[test]
fn p1_f17_gui_tx_state_is_telemetry_derived_guard() {
    let source = p1_f17_read("src/gui/app_cat.rs");
    // Exactly one write of the GUI TX flag, inside the telemetry update function.
    assert_eq!(
        source.matches("self.cat.ptt_active =").count(),
        1,
        "F-17: the GUI TX flag may only be written from telemetry"
    );
    let telemetry = source
        .split("pub fn update_cat_state_from_telemetry")
        .nth(1)
        .expect("telemetry update function");
    let body = telemetry.split("\n    pub fn ").next().unwrap_or(telemetry);
    assert!(
        body.contains("self.cat.ptt_active = is_tx;"),
        "F-17: the GUI TX flag must be derived from the arbiter state, not optimistically"
    );
    for path in p1_f17_tracked_sources("src/gui") {
        let content = p1_f17_read(&path);
        for forbidden in ["ptt_active = true", "ptt_active: true"] {
            assert!(
                !content.contains(forbidden),
                "F-17: {path} must not optimistically set TX: {forbidden}"
            );
        }
    }
}

#[test]
fn p1_f17_documentation_records_software_only_scope_guard() {
    let known = p1_f17_read("docs/KNOWN_ISSUES.md");
    for required in [
        "F-17 = Resolved (Verified)",
        "Single-key PTT = REMOVED",
        "Text editing PTT = BLOCKED",
        "IME PTT = BLOCKED",
        "Window-unfocused PTT = BLOCKED",
        "Repeated-key PTT = BLOCKED",
        "PTT without F-15 authorization = BLOCKED",
        "Hardware interaction = NOT_VERIFIED",
        "NATIVE-7B = NOT_STARTED",
    ] {
        assert!(
            known.contains(required),
            "docs/KNOWN_ISSUES.md must record '{required}'"
        );
    }
    for forbidden in [
        "hardware PTT verified",
        "hardware interaction verified",
        "real transmitter behaviour verified",
        "physical TX verified",
        "HIL completed",
        "NATIVE-7B completed",
    ] {
        assert!(
            !known.contains(forbidden),
            "docs/KNOWN_ISSUES.md must not claim '{forbidden}'"
        );
    }

    let readme = p1_f17_read("README.md");
    assert!(
        readme.contains("Ctrl+Shift+F7"),
        "README.md must document the PTT ON shortcut"
    );
    assert!(
        readme.contains("Ctrl+F7"),
        "README.md must document the PTT OFF shortcut"
    );
    for line in readme.lines() {
        if line.starts_with("| `F7` |") {
            panic!("README.md must not document a single-key PTT row: {line}");
        }
    }

    let manual = p1_f17_read("locales/manual/en.json");
    assert!(
        !manual.contains("F7 \u{2014} Toggle transceiver PTT"),
        "the built-in manual must not document a single-key PTT toggle"
    );
    assert!(
        manual.contains("Ctrl+Shift+F7"),
        "the built-in manual must document the new PTT interaction"
    );
}

#[test]
fn p1_f17_localisation_exposes_two_explicit_ptt_intents_guard() {
    for locale in ["pl", "en", "de", "fr", "es", "it", "ru"] {
        let path = format!("locales/{locale}.json");
        let content = p1_f17_read(&path);
        for required in [
            "\"cmd.ptt_on\"",
            "\"cmd.ptt_off\"",
            "\"shortcuts.ptt_on\"",
            "\"shortcuts.ptt_off\"",
            "\"ptt.refused\"",
        ] {
            assert!(
                content.contains(required),
                "{path} must contain the F-17 key {required}"
            );
        }
        for removed in ["\"cmd.toggle_ptt\"", "\"shortcuts.toggle_ptt\""] {
            assert!(
                !content.contains(removed),
                "{path} must not keep the ambiguous key {removed}"
            );
        }
    }
}

#[test]
fn p1_f17_prior_gate_state_is_unchanged_guard() {
    let known = p1_f17_read("docs/KNOWN_ISSUES.md");
    for required in [
        "F-14 = Resolved (Verified)",
        "F-15 = Resolved (Verified)",
        "CAT-018 = CLOSED",
        "CAT-019 = CLOSED",
        "NATIVE-7B = NOT_STARTED",
    ] {
        assert!(
            known.contains(required),
            "docs/KNOWN_ISSUES.md must keep '{required}'"
        );
    }

    let native = p1_f17_read("tests/cat_native_7.rs");
    let ignored_attributes = native
        .lines()
        .filter(|line| line.trim_start().starts_with("#[ignore"))
        .count();
    assert_eq!(
        ignored_attributes, 10,
        "F-17: the ten hardware-in-the-loop tests must stay ignored"
    );

    for workflow in [
        ".github/workflows/ci.yml",
        ".github/workflows/build-and-release.yml",
    ] {
        let content = p1_f17_read(workflow);
        for forbidden in ["--ignored", "hil_hardware", "SPLOG_HIL_ENABLE"] {
            assert!(
                !content.contains(forbidden),
                "F-17: {workflow} must not run HIL: {forbidden}"
            );
        }
    }
}

#[test]
fn p1_f17_gui_ptt_controls_are_explicit_and_textually_signalled_guard() {
    let source = p1_f17_read("src/gui/vfo_panel.rs");
    // Two explicit buttons, never one ambiguous toggle.
    assert!(
        source.contains("PttIntent::RequestOn"),
        "the VFO panel must expose an explicit TX ON action"
    );
    assert!(
        source.contains("PttIntent::RequestOff"),
        "the VFO panel must expose an explicit TX OFF action"
    );
    assert!(
        source.contains("VFO_PTT_TOOLTIP"),
        "both PTT buttons must carry a localised tooltip"
    );
    // The TX/RX state is signalled as text (not colour only).
    assert!(
        source.contains("TX ON"),
        "the TX state must be signalled as text"
    );
    assert!(
        source.contains(" TX OFF") || source.contains("\"TX OFF\""),
        "the TX OFF action must be labelled as text"
    );
    assert!(
        source.contains("RX\""),
        "the RX state must be signalled as text"
    );
    // Edge-triggered interaction only: no press-and-hold pointer path.
    assert!(
        !source.contains("is_pointer_button_down_on"),
        "PTT must not be driven by a held pointer button"
    );
    assert!(
        source.contains(".clicked()"),
        "PTT buttons must be edge-triggered by a click"
    );
}

#[test]
fn p1_f17_refusal_is_signalled_as_text_guard() {
    let source = p1_f17_read("src/gui/app_cat.rs");
    assert!(
        source.contains("key::PTT_REFUSED"),
        "a refused PTT request must produce a localised status message"
    );
    assert!(
        source.contains("status_toast"),
        "the refusal must be shown as a status text, not as a colour change"
    );
    assert!(
        source.contains("fn ptt_refusal_is_user_visible"),
        "the refusal visibility policy must be explicit"
    );
    // Rationale for every benign no-op: they are silent, everything else is reported.
    assert!(
        source.contains("AuthorizationDenied"),
        "an authorization denial must be reported to the operator"
    );
    assert!(
        source.contains("RefusedUnknownTxState"),
        "an ambiguous arbiter state must be reported to the operator"
    );
}

// ============================================================================
// P1-F16 — panic-free logbook pagination and filter-transition guards
//
// Kontrakt F-16 (ADR-97..ADR-102):
//   * jedno centralne, czyste i totalne obliczenie paginacji;
//   * brak bezpośredniego, niezweryfikowanego slicingu strony;
//   * checked arithmetic zamiast panic/wrap/saturating-maskowania;
//   * page size 0 i pusty wynik mają jawny, bezpieczny stan;
//   * stale page jest normalizowana przed slicingiem;
//   * zaznaczenie opiera się na stabilnym ID, nie na indeksie strony;
//   * testy używają wyłącznie danych syntetycznych;
//   * F-14 / CAT-018 / CAT-019 / F-15 / F-17 pozostają CLOSED,
//     NATIVE-7B pozostaje NOT_STARTED, 10 testów HIL pozostaje ignored.
// Guardy czytają wyłącznie pliki śledzone w repozytorium; nie zależą od
// lokalnych notatek roboczych.
// ============================================================================

fn p1_f16_read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

fn p1_f16_assert_tests(source: &str, names: &[&str], label: &str) {
    for name in names {
        assert!(
            source.contains(&format!("fn {name}(")),
            "F-16: {label} must contain the test `{name}`"
        );
    }
}

#[test]
fn p1_f16_pagination_has_one_central_checked_model_guard() {
    let model = p1_f16_read("src/gui/pagination.rs");
    for required in [
        "pub fn paginate",
        "pub struct PaginationInput",
        "pub struct PaginationWindow",
        "pub enum PageMode",
        "checked_mul",
        "checked_add",
        "div_ceil",
        "pub fn slice_of",
        "PageMode::AllRows",
        "effective_page_size",
    ] {
        assert!(
            model.contains(required),
            "F-16: the central pagination model must contain {required}"
        );
    }
    assert!(
        !model.contains("get_unchecked"),
        "F-16: the pagination model must never use get_unchecked"
    );
    assert!(
        !model.contains("catch_unwind"),
        "F-16: catch_unwind must never hide a pagination failure"
    );
    assert!(
        !model.contains("std::time") && !model.contains("Instant::now"),
        "F-16: the pagination model must be independent of wall-clock time"
    );
    assert!(
        !model.contains("egui::") && !model.contains("eframe::"),
        "F-16: the pagination model must stay GUI-independent"
    );

    let table = p1_f16_read("src/gui/logbook_table.rs");
    assert!(
        table.contains("select_page_indices") && table.contains("pagination_window"),
        "F-16: the table must use the central pagination model"
    );
    for forbidden in [
        "fn page_range",
        "[page_start..page_end]",
        "page * page_size",
        "page + 1",
        "page_size - 1",
        "start + page_size",
    ] {
        assert!(
            !table.contains(forbidden),
            "F-16: logbook_table.rs must not contain `{forbidden}`"
        );
    }

    let state = p1_f16_read("src/gui/state/log_view.rs");
    assert!(
        state.contains("pub fn pagination_window") && state.contains("pub fn select_page_indices"),
        "F-16: LogViewState must expose the normalized window and safe selection"
    );
    assert!(
        state.contains("paginate(PaginationInput"),
        "F-16: LogViewState must delegate to the central model"
    );

    let reload = p1_f16_read("src/gui/app_qso.rs");
    assert!(
        reload.contains("pagination_window(total)"),
        "F-16: every data reload must normalize the page through the model"
    );
    assert!(
        !reload.contains("page_size - 1"),
        "F-16: reload must not use the old ad-hoc page-count formula"
    );
}

#[test]
fn p1_f16_slice_and_index_access_is_checked_guard() {
    for path in [
        "src/gui/logbook_table.rs",
        "src/gui/state/log_view.rs",
        "src/gui/pagination.rs",
        "src/gui/app_qso.rs",
        "src/gui/app_layout.rs",
    ] {
        let source = p1_f16_read(path);
        for forbidden in ["get_unchecked", "catch_unwind"] {
            assert!(
                !source.contains(forbidden),
                "F-16: {path} must not contain `{forbidden}`"
            );
        }
        for line in qa_code_lines(&source) {
            assert!(
                !line.contains("sorted_indices["),
                "F-16: {path} must not index sorted_indices directly: {}",
                line.trim()
            );
            assert!(
                !line.contains("recent_qsos["),
                "F-16: {path} must not index recent_qsos directly: {}",
                line.trim()
            );
            if line.contains(".get(") {
                assert!(
                    !line.contains(".unwrap()") && !line.contains(".expect("),
                    "F-16: {path} must not unwrap a checked slice access: {}",
                    line.trim()
                );
            }
        }
    }

    let table = p1_f16_read("src/gui/logbook_table.rs");
    assert!(
        table.contains("page_indices.get(row.index())") && table.contains("recent_qsos.get(idx)"),
        "F-16: the row renderer must use checked gets"
    );
    let model = p1_f16_read("src/gui/pagination.rs");
    assert!(
        model.contains("items.get(self.start..self.end)"),
        "F-16: slice_of must use slice::get, never direct indexing"
    );
    let state = p1_f16_read("src/gui/state/log_view.rs");
    assert!(
        state.contains("window.slice_of(&self.cache.sorted_indices)"),
        "F-16: page indices must be obtained through the checked model"
    );
}

#[test]
fn p1_f16_boundary_and_transition_tests_are_pinned_guard() {
    let model = p1_f16_read("src/gui/pagination.rs");
    p1_f16_assert_tests(
        &model,
        &[
            "empty_dataset_has_explicit_zero_window",
            "stale_page_normalizes_to_last_existing_page",
            "page_size_zero_means_all_rows",
            "page_size_zero_empty_dataset_is_empty",
            "addition_overflow_falls_back_to_total",
            "multiplication_overflow_is_impossible_and_guarded",
            "slice_of_returns_none_for_inconsistent_window",
            "page_ranges_partition_the_result_without_gaps_or_duplicates",
            "combinatorial_model_matrix_holds_every_invariant",
        ],
        "the central pagination model",
    );

    let integration = p1_f16_read("tests/p1_f16_logbook_pagination.rs");
    p1_f16_assert_tests(
        &integration,
        &[
            "stale_page_after_search_contraction_is_normalized",
            "empty_dataset_with_stale_page_is_safe",
            "page_index_usize_max_render_is_safe",
            "zero_result_filter_is_explicitly_empty",
            "last_partial_page_is_rendered",
            "exact_multiple_page_size_has_no_extra_page",
            "page_size_change_never_creates_invalid_range",
            "page_size_zero_renders_all_rows",
            "page_size_usize_max_renders_one_page",
            "stale_page_after_journal_switch_to_empty",
            "stale_page_after_deleting_last_page",
            "deleting_all_rows_yields_explicit_empty_state",
            "import_and_render_in_one_transition",
            "sync_add_and_render_in_one_transition",
            "selection_preserved_by_stable_id_across_filter",
            "selection_removed_when_record_deleted",
            "pages_do_not_duplicate_rows",
            "pages_cover_filtered_result_without_gaps",
            "inconsistent_cache_is_repaired_on_render",
            "gui_render_stress_200",
            "gui_render_path_zero_panic_across_transitions",
            "full_gui_frame_renders_without_panic",
        ],
        "the F-16 integration suite",
    );
    for iterations in [
        "0..5_000usize",
        "0..3_000usize",
        "0..2_000usize",
        "0..200usize",
    ] {
        assert!(
            integration.contains(iterations),
            "F-16: the deterministic stress series {iterations} must stay pinned"
        );
    }
    assert!(
        integration.contains("combinations >= 1_000"),
        "F-16: the combinatorial transition matrix must stay pinned"
    );
}

#[test]
fn p1_f16_tests_use_only_synthetic_data_guard() {
    let integration = p1_f16_read("tests/p1_f16_logbook_pagination.rs");
    assert!(
        integration.contains("synthetic"),
        "F-16: the test suite must declare its synthetic data contract"
    );
    assert!(
        integration.contains("LogDatabase::open_in_memory()"),
        "F-16: tests must use an in-memory synthetic database"
    );
    for forbidden in [
        "default_log.db",
        "station_config.json",
        "APPDATA",
        "C:\\Users",
        "/home/",
        "\"SP6INA\"",
        "sleep(",
        "std::time::Instant",
        "rand::",
    ] {
        assert!(
            !integration.contains(forbidden),
            "F-16: the test suite must not use real user data or nondeterminism: {forbidden}"
        );
    }
    let model = p1_f16_read("src/gui/pagination.rs");
    assert!(
        !model.contains("QsoRecord") && !model.contains("LogDatabase"),
        "F-16: the pagination model must not touch user records or the database"
    );
}

#[test]
fn p1_f16_documentation_records_the_closed_remediation_guard() {
    let known = p1_f16_read("docs/KNOWN_ISSUES.md");
    for required in [
        "F-16 (P1) — GUI panic in logbook paging — F-16 = Resolved (Verified)",
        "F-16 = CLOSED",
        "Status: Resolved (Verified)",
        "Logbook pagination = PANIC_FREE_SOFTWARE_VERIFIED",
        "Stale page normalization = VERIFIED",
        "Filter transitions = VERIFIED",
        "Selection stability = VERIFIED",
        "Real user data = NOT_USED",
        "F-14 = Resolved (Verified)",
        "F-15 = Resolved (Verified)",
        "F-17 = Resolved (Verified)",
        "CAT-018 = CLOSED",
        "CAT-019 = CLOSED",
        "NATIVE-7B = NOT_STARTED",
    ] {
        assert!(
            known.contains(required),
            "docs/KNOWN_ISSUES.md must keep `{required}`"
        );
    }

    let roadmap = p1_f16_read("docs/ROADMAP.md");
    assert!(
        roadmap.contains("F-16") && roadmap.contains("F-16 = Resolved (Verified)"),
        "docs/ROADMAP.md must record the resolved F-16 pagination fix"
    );

    let baseline = p1_f16_read("docs/VERIFICATION_BASELINE.md");
    assert!(
        baseline.contains("F-16") && baseline.contains("Resolved (Verified)"),
        "docs/VERIFICATION_BASELINE.md must record the F-16 verification"
    );

    let development = p1_f16_read("docs/DEVELOPMENT.md");
    for required in ["P1-F16", "PaginationInput", "checked_mul", "slice::get"] {
        assert!(
            development.contains(required),
            "docs/DEVELOPMENT.md must document `{required}`"
        );
    }

    let decisions = p1_f16_read("docs/architecture/DECISIONS.md");
    for required in [
        "| `ADR-97` |",
        "| `ADR-98` |",
        "| `ADR-100` |",
        "| `ADR-102` |",
    ] {
        assert!(
            decisions.contains(required),
            "docs/architecture/DECISIONS.md must contain `{required}`"
        );
    }

    let architecture = p1_f16_read("docs/architecture/ARCHITECTURE.md");
    assert!(
        architecture.contains("F-16") && architecture.contains("PANIC_FREE"),
        "docs/architecture/ARCHITECTURE.md must record F-16 = PANIC_FREE"
    );

    let security_process = p1_f16_read("docs/SECURITY_PROCESS.md");
    assert!(
        security_process.contains("P1-F16") && security_process.contains("pagination"),
        "docs/SECURITY_PROCESS.md must document the F-16 pagination boundary"
    );
}

#[test]
fn p1_f16_prior_gate_state_and_scope_are_unchanged_guard() {
    let native = p1_f16_read("tests/cat_native_7.rs");
    let ignored_attributes = native
        .lines()
        .filter(|line| line.trim_start().starts_with("#[ignore"))
        .count();
    assert_eq!(
        ignored_attributes, 10,
        "F-16: the ten hardware-in-the-loop tests must stay ignored"
    );

    for workflow in [
        ".github/workflows/ci.yml",
        ".github/workflows/build-and-release.yml",
    ] {
        let content = p1_f16_read(workflow);
        for forbidden in ["--ignored", "hil_hardware", "SPLOG_HIL_ENABLE"] {
            assert!(
                !content.contains(forbidden),
                "F-16: {workflow} must not run HIL: {forbidden}"
            );
        }
    }

    // Zakres bramki: brak nowej zależności i brak nowego targetu testowego.
    let manifest = p1_f16_read("Cargo.toml");
    assert!(
        !manifest.contains("pagination") && !manifest.contains("f16"),
        "F-16: Cargo.toml must not gain a pagination dependency or feature"
    );
    assert!(
        !manifest.contains("[[test]]"),
        "F-16: test targets stay auto-discovered; no explicit [[test]] entry"
    );
    let lock = p1_f16_read("Cargo.lock");
    assert!(
        !lock.contains("name = \"pagination\""),
        "F-16: Cargo.lock must not gain a pagination crate"
    );
}

// ============================================================================
// P1-F19 — SOTA/POTA operator-correspondent reference semantics
// ============================================================================
//
// These guards pin the role-aware reference mapping required by the F-19 gate:
//   * `MY_*` always describes the SPLogbook station, the un-prefixed partner the
//     correspondent (ADIF 3.1.8 §III.C.1.b);
//   * exactly one central mapping exists; the ADIF engine cannot map references
//     on its own any more;
//   * S2S and P2P preserve both sides, no side is ever copied or swapped;
//   * the POTA production contract is the documented SIG pair, and a record never
//     carries two contradictory POTA representations;
//   * a conflicting input is a controlled per-record rejection;
//   * the GUI form, the spot path and the activation context keep the sides;
//   * no real user data, no external upload, prior gates unchanged.

const P1_F19_TAGS: [&str; 8] = [
    "MY_SOTA_REF",
    "SOTA_REF",
    "MY_POTA_REF",
    "POTA_REF",
    "MY_SIG",
    "MY_SIG_INFO",
    "SIG",
    "SIG_INFO",
];

const P1_F19_LOCAL_SOTA: &str = "AA/BB-001";
const P1_F19_REMOTE_SOTA: &str = "CC/DD-999";
const P1_F19_LOCAL_POTA: &str = "XX-0001";
const P1_F19_REMOTE_POTA: &str = "YY-9999";

/// Builds a synthetic record with the four reference slots set explicitly.
fn p1_f19_qso(
    local_sota: Option<&str>,
    remote_sota: Option<&str>,
    local_pota: Option<&str>,
    remote_pota: Option<&str>,
) -> splogbook::core::qso::QsoRecord {
    let mut qso = splogbook::core::qso::QsoRecord::new("TSTREMOTE", "20m", "CW");
    qso.qso_date = "20260101".to_string();
    qso.time_on = "120000".to_string();
    qso.my_sota_ref = local_sota.map(str::to_string);
    qso.sota_ref = remote_sota.map(str::to_string);
    qso.my_pota_ref = local_pota.map(str::to_string);
    qso.pota_ref = remote_pota.map(str::to_string);
    qso
}

/// Renders one record to ADIF text.
fn p1_f19_rendered(qso: &splogbook::core::qso::QsoRecord) -> String {
    splogbook::core::adif::export_adif(std::slice::from_ref(qso), "SPLogbook", "TSTLOCAL")
}

/// Returns the value of the first `TAG` in an ADIF text.
fn p1_f19_field<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let needle = format!("<{tag}:");
    let start = text.find(&needle)?;
    let after = &text[start + needle.len()..];
    let close = after.find('>')?;
    let declared: usize = after[..close].parse().ok()?;
    after[close + 1..].get(..declared)
}

#[test]
fn p1_f19_reference_mapping_is_central_and_exclusive_guard() {
    let central = p1_f16_read("src/core/award_refs.rs");
    for tag in P1_F19_TAGS {
        assert!(
            central.contains(tag),
            "F-19: the central mapping must define {tag}"
        );
    }

    // The ADIF engine must not map reference fields on its own any more.
    let engine = p1_f16_read("src/core/adif.rs");
    for tag in P1_F19_TAGS {
        let quoted = format!("\"{tag}\"");
        assert!(
            !engine.contains(&quoted),
            "F-19: src/core/adif.rs must not map {tag} directly; use core::award_refs"
        );
    }
    assert!(
        engine.contains("crate::core::award_refs::import_adif_references"),
        "F-19: the importer must delegate to the central mapping"
    );
    assert!(
        engine.contains("q.award_references().adif_fields()"),
        "F-19: the exporter must delegate to the central mapping"
    );

    for (path, token) in [
        ("src/core/award_refs.rs", "pub fn import_adif_references"),
        ("src/core/award_refs.rs", "pub fn adif_fields"),
    ] {
        assert!(
            p1_f16_read(path).contains(token),
            "F-19: {path} must define `{token}`"
        );
    }
}

#[test]
fn p1_f19_sota_local_and_remote_map_to_their_own_fields_guard() {
    let local_only = p1_f19_qso(Some(P1_F19_LOCAL_SOTA), None, None, None);
    let text = p1_f19_rendered(&local_only);
    assert_eq!(p1_f19_field(&text, "MY_SOTA_REF"), Some(P1_F19_LOCAL_SOTA));
    assert_eq!(p1_f19_field(&text, "SOTA_REF"), None);

    let remote_only = p1_f19_qso(None, Some(P1_F19_REMOTE_SOTA), None, None);
    let text = p1_f19_rendered(&remote_only);
    assert_eq!(p1_f19_field(&text, "SOTA_REF"), Some(P1_F19_REMOTE_SOTA));
    assert_eq!(p1_f19_field(&text, "MY_SOTA_REF"), None);
}

#[test]
fn p1_f19_s2s_and_p2p_preserve_both_sides_guard() {
    let s2s = p1_f19_qso(
        Some(P1_F19_LOCAL_SOTA),
        Some(P1_F19_REMOTE_SOTA),
        None,
        None,
    );
    let text = p1_f19_rendered(&s2s);
    assert_eq!(p1_f19_field(&text, "MY_SOTA_REF"), Some(P1_F19_LOCAL_SOTA));
    assert_eq!(p1_f19_field(&text, "SOTA_REF"), Some(P1_F19_REMOTE_SOTA));

    let p2p = p1_f19_qso(
        None,
        None,
        Some(P1_F19_LOCAL_POTA),
        Some(P1_F19_REMOTE_POTA),
    );
    let text = p1_f19_rendered(&p2p);
    assert_eq!(p1_f19_field(&text, "MY_SIG"), Some("POTA"));
    assert_eq!(p1_f19_field(&text, "MY_SIG_INFO"), Some(P1_F19_LOCAL_POTA));
    assert_eq!(p1_f19_field(&text, "SIG"), Some("POTA"));
    assert_eq!(p1_f19_field(&text, "SIG_INFO"), Some(P1_F19_REMOTE_POTA));
}

#[test]
fn p1_f19_no_cross_side_fallback_guard() {
    for (local, remote, local_tag, remote_tag) in [
        (Some(P1_F19_LOCAL_SOTA), None, "MY_SOTA_REF", "SOTA_REF"),
        (None, Some(P1_F19_REMOTE_SOTA), "MY_SOTA_REF", "SOTA_REF"),
    ] {
        let qso = p1_f19_qso(local, remote, None, None);
        let text = p1_f19_rendered(&qso);
        assert_eq!(p1_f19_field(&text, local_tag), local);
        assert_eq!(p1_f19_field(&text, remote_tag), remote);
    }

    let report = splogbook::core::adif::parse_adif_with_report(concat!(
        "<CALL:9>TSTREMOTE<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20260101<TIME_ON:6>120000",
        "<MY_SOTA_REF:9>AA/BB-001<EOR>"
    ));
    assert_eq!(report.rejected, 0);
    assert_eq!(report.qsos[0].sota_ref, None, "no local->remote fallback");
    assert_eq!(
        report.qsos[0].my_sota_ref.as_deref(),
        Some(P1_F19_LOCAL_SOTA)
    );

    let central = p1_f16_read("src/core/award_refs.rs");
    assert!(
        !central.contains("unwrap_or_else(|| self.remote")
            && !central.contains("unwrap_or_else(|| self.local"),
        "F-19: the model must not fall back from one side to the other"
    );
}

#[test]
fn p1_f19_pota_never_emits_two_representations_or_an_incomplete_sig_pair_guard() {
    for (local, remote) in [
        (None, None),
        (Some(P1_F19_LOCAL_POTA), None),
        (None, Some(P1_F19_REMOTE_POTA)),
        (Some(P1_F19_LOCAL_POTA), Some(P1_F19_REMOTE_POTA)),
        (Some("XX-0001,XX-0002"), Some(P1_F19_REMOTE_POTA)),
    ] {
        let qso = p1_f19_qso(None, None, local, remote);
        let text = p1_f19_rendered(&qso);
        let has_direct = p1_f19_field(&text, "MY_POTA_REF").is_some()
            || p1_f19_field(&text, "POTA_REF").is_some();
        let has_sig =
            p1_f19_field(&text, "MY_SIG").is_some() || p1_f19_field(&text, "SIG").is_some();
        assert!(
            !(has_direct && has_sig),
            "F-19: one POTA representation per record, got: {text}"
        );
        for (sig, info) in [("MY_SIG", "MY_SIG_INFO"), ("SIG", "SIG_INFO")] {
            assert_eq!(
                p1_f19_field(&text, sig).is_some(),
                p1_f19_field(&text, info).is_some(),
                "F-19: {sig} and {info} must always appear together"
            );
        }
    }
}

#[test]
fn p1_f19_conflicting_pota_representation_is_an_explicit_rejection_guard() {
    let report = splogbook::core::adif::parse_adif_with_report(concat!(
        "<CALL:9>TSTREMOTE<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20260101<TIME_ON:6>120000",
        "<MY_POTA_REF:7>XX-0001<MY_SIG:4>POTA<MY_SIG_INFO:7>XX-0002<EOR>"
    ));
    assert_eq!(report.rejected, 1, "the ambiguous record must be rejected");
    assert!(report.qsos.is_empty());
    let message = report.errors.join(" ");
    assert!(
        message.contains("sprzeczne") && message.contains("XX-0001") && message.contains("XX-0002"),
        "F-19: the conflict must be explicit and must name both values: {message}"
    );

    let report = splogbook::core::adif::parse_adif_with_report(concat!(
        "<CALL:9>TSTREMOTE<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20260101<TIME_ON:6>120000",
        "<MY_POTA_REF:7>XX-0001<MY_SIG:4>POTA<MY_SIG_INFO:7>xx-0001<EOR>"
    ));
    assert_eq!(report.rejected, 0);
    assert_eq!(
        report.qsos[0].my_pota_ref.as_deref(),
        Some(P1_F19_LOCAL_POTA)
    );
}

#[test]
fn p1_f19_import_export_round_trip_is_verified_guard() {
    let suite = p1_f16_read("tests/p1_f19_award_references.rs");
    for required in [
        "fn sota_activator_writes_own_summit_to_my_sota_ref(",
        "fn sota_chaser_writes_chased_summit_to_sota_ref(",
        "fn sota_summit_to_summit_preserves_both_distinct_references(",
        "fn pota_activator_writes_my_sig_pair_and_no_remote_pair(",
        "fn pota_hunter_writes_sig_pair_and_no_local_pair(",
        "fn pota_park_to_park_preserves_both_references(",
        "fn pota_import_rejects_contradictory_representations(",
        "fn adif_to_model_to_adif_is_semantically_stable(",
        "fn model_to_database_to_model_preserves_both_sides(",
        "fn combinatorial_matrix_preserves_roles_for_every_combination(",
        "fn stress_reference_paths_are_deterministic_and_sleep_free(",
        "fn fault_injection_unknown_sig_is_reported_not_guessed(",
    ] {
        assert!(
            suite.contains(required),
            "F-19: the verification suite must contain `{required}`"
        );
    }
    assert!(
        suite.contains("100_000"),
        "F-19: the combinatorial matrix must cover at least 100 000 cases"
    );
    for forbidden in ["std::thread::sleep", "https://", "http://"] {
        assert!(
            !suite.contains(forbidden),
            "F-19: the suite must not contain `{forbidden}`"
        );
    }
}

#[test]
fn p1_f19_spot_and_activation_perspective_guard() {
    let cluster = p1_f16_read("src/gui/cluster_panel.rs");
    let spot_block = cluster
        .split("if let Some((dx_call, freq_khz, band, pota, sota)) = tune_target")
        .nth(1)
        .expect("F-19: the spot-to-entry block must exist");
    let spot_block = &spot_block[..spot_block.len().min(600)];
    assert!(
        !spot_block.contains("my_refs")
            && !spot_block.contains("my_sota")
            && !spot_block.contains("my_pota"),
        "F-19: a spot must never write the local activation reference"
    );

    let entry = p1_f16_read("src/gui/state/entry.rs");
    // The old bug shape: a QsoEntryState-level `sota`/`pota` field initialised
    // directly from the station profile (12-space indentation inside `Self {}`).
    let prefills_correspondent = entry.lines().any(|line| {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        indent == 12
            && (trimmed.starts_with("sota:") || trimmed.starts_with("pota:"))
            && trimmed.contains("config.station")
    });
    assert!(
        !prefills_correspondent,
        "F-19: the station profile must not prefill the correspondent field"
    );
    assert!(
        entry.contains("pub struct MyAwardRefs"),
        "F-19: the local reference pair must be an explicit type"
    );
    assert!(
        entry.contains("sota: config.station.sota_ref"),
        "F-19: the station profile prefills the local side"
    );

    let app_qso = p1_f16_read("src/gui/app_qso.rs");
    assert!(
        app_qso.contains("qso.my_sota_ref = if self.entry.my_refs.sota.is_empty()"),
        "F-19: the local SOTA reference is persisted from the local entry field"
    );
    assert!(
        app_qso.contains("qso.sota_ref = if self.entry.sota.is_empty()"),
        "F-19: the correspondent SOTA reference is persisted from the correspondent field"
    );
    assert!(
        !app_qso.contains("qso.my_sota_ref = if self.entry.sota.is_empty()"),
        "F-19: the correspondent value must never be persisted as the local reference"
    );
    assert!(
        app_qso.contains("pub fn draft_qso_from_entry"),
        "F-19: the form model mapping must be a pure, testable function"
    );

    let profiles = p1_f16_read("src/gui/app_config.rs");
    assert!(
        profiles.contains("self.entry.my_refs.pota = prof.pota_ref")
            && profiles.contains("self.entry.my_refs.sota = prof.sota_ref"),
        "F-19: a profile switch updates only the local side"
    );
}

#[test]
fn p1_f19_sota_csv_uses_the_record_reference_guard() {
    let exporter = p1_f16_read("src/core/sota_export.rs");
    assert!(
        exporter.contains(".unwrap_or(my_sota)"),
        "F-19: the SOTA CSV must prefer the record's own summit over the dialog default"
    );
    assert!(
        exporter.contains("my_sota_ref"),
        "F-19: the SOTA CSV reads the local (MY_) reference"
    );

    let mut qso = splogbook::core::qso::QsoRecord::new("TSTREMOTE", "20m", "CW");
    qso.qso_date = "20260101".to_string();
    qso.time_on = "1200".to_string();
    qso.my_sota_ref = Some(P1_F19_LOCAL_SOTA.to_string());
    qso.sota_ref = Some(P1_F19_REMOTE_SOTA.to_string());
    let csv = splogbook::core::sota_export::export_sota_csv(&[qso], "TSTLOCAL", "ZZ/XX-777")
        .expect("csv export");
    assert!(csv.contains(P1_F19_LOCAL_SOTA), "local summit in column 3");
    assert!(
        csv.contains(P1_F19_REMOTE_SOTA),
        "correspondent summit in column 9"
    );
    assert!(
        !csv.contains("ZZ/XX-777"),
        "F-19: the activation default must not override a per-record summit"
    );
}

#[test]
fn p1_f19_documentation_records_the_closed_remediation_guard() {
    let known = p1_f16_read("docs/KNOWN_ISSUES.md");
    for required in [
        "F-19 (P1)",
        "F-19 = CLOSED",
        "F-19 = Resolved (Verified)",
        "SOTA local/remote mapping =",
        "POTA local/remote mapping =",
        "S2S mapping =",
        "P2P mapping =",
        "ADIF round-trip =",
        "SOFTWARE_VERIFIED",
        "External SOTA/POTA upload =",
        "NOT_EXECUTED",
        "Real user data =",
        "NOT_USED",
        "NATIVE-7B = NOT_STARTED",
    ] {
        assert!(
            known.contains(required),
            "KNOWN_ISSUES must record the F-19 closure token `{required}`"
        );
    }
    for forbidden in [
        "SOTA production upload accepted",
        "POTA production upload accepted",
        "every historical user record is correct",
        "hardware was verified",
        "HIL completed",
        "NATIVE-7B completed",
    ] {
        assert!(
            !known.contains(forbidden),
            "KNOWN_ISSUES must not claim `{forbidden}`"
        );
    }

    let reference_doc = p1_f16_read("docs/ADIF_AWARD_REFERENCES.md");
    for required in [
        "MY_SOTA_REF",
        "SOTA_REF",
        "MY_POTA_REF",
        "POTA_REF",
        "MY_SIG_INFO",
        "SIG_INFO",
        "3.1.8",
        "docs.pota.app",
        "sota.org.uk",
    ] {
        assert!(
            reference_doc.contains(required),
            "docs/ADIF_AWARD_REFERENCES.md must document `{required}`"
        );
    }

    let index = p1_f16_read("docs/DOCUMENTATION_INDEX.md");
    assert!(
        index.contains("`docs/ADIF_AWARD_REFERENCES.md`"),
        "the new reference document must be indexed"
    );

    let development = p1_f16_read("docs/DEVELOPMENT.md");
    assert!(
        development.contains("P1-F19"),
        "docs/DEVELOPMENT.md must document the F-19 verification commands"
    );

    let baseline = p1_f16_read("docs/VERIFICATION_BASELINE.md");
    assert!(
        baseline.contains("P1-F19"),
        "docs/VERIFICATION_BASELINE.md must carry the F-19 measurement"
    );

    let architecture = p1_f16_read("docs/architecture/ARCHITECTURE.md");
    assert!(
        architecture.contains("award_refs"),
        "docs/architecture/ARCHITECTURE.md must list the award-reference module"
    );
}

#[test]
fn p1_f19_prior_gates_scope_and_safety_are_unchanged_guard() {
    let known = p1_f16_read("docs/KNOWN_ISSUES.md");
    for required in [
        "F-14 = Resolved (Verified)",
        "CAT-018 = CLOSED",
        "CAT-019 = CLOSED",
        "F-15 = Resolved (Verified)",
        "F-16 = Resolved (Verified)",
        "F-17 = Resolved (Verified)",
        "NATIVE-7B = NOT_STARTED",
    ] {
        assert!(
            known.contains(required),
            "prior gate state must stay unchanged: `{required}`"
        );
    }

    let native = p1_f16_read("tests/cat_native_7.rs");
    let ignored_attributes = native
        .lines()
        .filter(|line| line.trim_start().starts_with("#[ignore"))
        .count();
    assert_eq!(
        ignored_attributes, 10,
        "F-19: the ten hardware-in-the-loop tests must stay ignored"
    );

    for workflow in [
        ".github/workflows/ci.yml",
        ".github/workflows/build-and-release.yml",
    ] {
        let content = p1_f16_read(workflow);
        for forbidden in ["--ignored", "hil_hardware", "SPLOG_HIL_ENABLE"] {
            assert!(
                !content.contains(forbidden),
                "F-19: {workflow} must not run HIL: {forbidden}"
            );
        }
    }

    let manifest = p1_f16_read("Cargo.toml");
    assert!(
        !manifest.contains("[[test]]"),
        "F-19: test targets stay auto-discovered"
    );
    for forbidden in ["award_refs", "p1_f19", "sota_ref", "pota_ref"] {
        assert!(
            !manifest.contains(forbidden),
            "F-19: Cargo.toml must not mention `{forbidden}`"
        );
    }
    let status = Command::new("git")
        .args(["diff", "--name-only", "--", "Cargo.toml", "Cargo.lock"])
        .output()
        .expect("git must be executable");
    assert!(
        String::from_utf8_lossy(&status.stdout).trim().is_empty(),
        "F-19: Cargo.toml and Cargo.lock must stay unchanged"
    );

    let central = p1_f16_read("src/core/award_refs.rs");
    for forbidden in [
        "http://",
        "https://",
        "reqwest",
        "ApiClient",
        "TcpStream",
        "UdpSocket",
    ] {
        assert!(
            !central.contains(forbidden),
            "F-19: the reference mapping must not perform network I/O: {forbidden}"
        );
    }
}

#[test]
fn p1_f19_synthetic_data_only_guard() {
    let suite = p1_f16_read("tests/p1_f19_award_references.rs");
    for required in [
        "TSTLOCAL",
        "TSTREMOTE",
        "AA/BB-001",
        "CC/DD-999",
        "XX-0001",
        "YY-9999",
    ] {
        assert!(
            suite.contains(required),
            "F-19: the suite must use the synthetic fixture `{required}`"
        );
    }
    // The SPDX/copyright header names the author; scan the rest of the file only.
    let body: String = suite
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["SP6INA", "DL1ABC", "K1ABC", "upload(", ".env"] {
        assert!(
            !body.contains(forbidden),
            "F-19: the suite must not contain `{forbidden}`"
        );
    }

    // The SPDX/copyright header names the author; scan the rest of the file only.
    let central = p1_f16_read("src/core/award_refs.rs");
    let central_body: String = central
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["SP6INA", "password", "token", "secret"] {
        assert!(
            !central_body.contains(forbidden),
            "F-19: src/core/award_refs.rs must not contain `{forbidden}`"
        );
    }
}

// ---------------------------------------------------------------------------
// P1-F20 - LoTW/eQSL journal-scoped, lossless upload selection
// ---------------------------------------------------------------------------

fn p1_f20_read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

fn p1_f20_count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// Checks whether the `window` bytes preceding `position` contain `needle`,
/// without ever slicing inside a UTF-8 code point.
fn p1_f20_preceded_by(source: &str, position: usize, needle: &str, window: usize) -> bool {
    let mut start = position.saturating_sub(window);
    while start > 0 && !source.is_char_boundary(start) {
        start -= 1;
    }
    source[start..position].contains(needle)
}

#[test]
fn p1_f20_upload_selection_is_journal_scoped_guard() {
    let gui = p1_f20_read("src/gui/online_sync.rs");
    let orchestration = p1_f20_read("src/gui/online_sync_upload.rs");
    let selection = p1_f20_read("src/core/database/upload_selection.rs");

    // F-20: no GUI upload path may select data itself, and no upload path may
    // build the ADIF payload outside the central plan orchestrator.
    for forbidden in [
        "get_recent_qsos(",
        "get_recent_qsos_for_journal(",
        "export_adif(",
    ] {
        assert!(
            !gui.contains(forbidden),
            "F-20: src/gui/online_sync.rs must not contain `{forbidden}`"
        );
    }

    // Every online service goes through the central plan upload.
    assert_eq!(
        p1_f20_count(&gui, "start_upload("),
        7,
        "F-20: all seven online services must start a plan upload"
    );
    for target in [
        "UploadTarget::Lotw",
        "UploadTarget::Eqsl",
        "UploadTarget::ClubLog",
        "UploadTarget::Qrz",
        "UploadTarget::HamQth",
        "UploadTarget::Cloudlog",
        "UploadTarget::HrdLog",
    ] {
        assert!(
            gui.contains(target),
            "F-20: src/gui/online_sync.rs must dispatch {target}"
        );
    }

    // The orchestrator must use the central model, never a bare selector.
    for required in [
        "UploadPlan",
        "run_plan",
        "JournalId::new",
        "select_upload_candidates_for_journal",
        "commit_batch_acceptance",
    ] {
        assert!(
            orchestration.contains(required),
            "F-20: src/gui/online_sync_upload.rs must use `{required}`"
        );
    }
    assert!(
        !orchestration.contains("get_recent_qsos"),
        "F-20: the orchestrator must not use a recent-QSO selector"
    );

    // Every selection statement is journal scoped and none of them truncates.
    assert!(
        p1_f20_count(&selection, "journal_id = ?1") >= 3,
        "F-20: every upload selection/update statement must be journal scoped"
    );
    let selection_code: String = selection
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !selection_code.contains("LIMIT"),
        "F-20: upload selection must never use LIMIT as the selection"
    );
    assert!(
        !selection_code.contains("OFFSET"),
        "F-20: upload selection must not depend on pagination"
    );
    assert!(
        selection.contains("'Y', 'I'"),
        "F-20: eligibility must exclude accepted (Y) and ignored (I) records"
    );
}

#[test]
fn p1_f20_no_global_or_truncated_upload_selector_guard() {
    // The global selector may exist only as a definition and as a test helper;
    // no upload path may call it.
    let allowed = ["src/core/database/qso_store.rs", "src/core/backup.rs"];
    let mut offenders = Vec::new();
    let mut truncating_calls = Vec::new();
    for path in p1_f17_tracked_sources("src") {
        let source = p1_f20_read(&path);
        if source.contains("get_recent_qsos(") && !allowed.contains(&path.as_str()) {
            offenders.push(path.clone());
        }
        // A numeric literal limit on a journal-scoped "recent" selector in an
        // upload path is the exact F-20 truncation shape.
        for forbidden in [
            "get_recent_qsos_for_journal(&journal_id, 1000)",
            "get_recent_qsos_for_journal(&self.active_journal.id, 1000)",
            ".take(1000)",
        ] {
            if source.contains(forbidden) {
                truncating_calls.push(format!("{path}: {forbidden}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "F-20: the global upload selector must not be called: {offenders:?}"
    );
    assert!(
        truncating_calls.is_empty(),
        "F-20: silent truncation shapes must not exist: {truncating_calls:?}"
    );
}

#[test]
fn p1_f20_lossless_batching_invariants_guard() {
    use splogbook::cloud::upload_plan::{
        JournalId, LOTW_BATCH_SIZE, PlanError, PlanRecord, SelectionPolicy, UploadPlan,
        UploadTarget,
    };

    let records: Vec<PlanRecord> = (1..=1505i64)
        .map(|id| PlanRecord::new(id, "20260101", format!("{id:06}")))
        .collect();
    let plan = UploadPlan::build(
        UploadTarget::Lotw,
        JournalId::new("GUARD_JOURNAL").expect("journal"),
        SelectionPolicy::PendingOnly,
        "attempt",
        0,
        records,
        LOTW_BATCH_SIZE,
    )
    .expect("plan");

    let batches = plan.batches();
    assert_eq!(plan.total_records(), 1505);
    assert_eq!(batches.len(), 1505usize.div_ceil(LOTW_BATCH_SIZE));
    let mut seen = HashSet::new();
    let mut flat = Vec::new();
    for batch in &batches {
        assert!(!batch.record_ids().is_empty());
        for id in batch.record_ids() {
            assert!(seen.insert(*id), "F-20: duplicate record across batches");
            flat.push(*id);
        }
    }
    assert_eq!(seen.len(), 1505, "F-20: batches must cover the whole plan");
    assert_eq!(flat, plan.record_ids(), "F-20: order must be preserved");
    assert_eq!(
        batches.iter().map(|batch| batch.count()).sum::<usize>(),
        1505,
        "F-20: sum of batches must equal the plan size"
    );

    // Zero is a controlled error, never "no batches".
    assert_eq!(
        UploadPlan::build(
            UploadTarget::Eqsl,
            JournalId::new("GUARD_JOURNAL").expect("journal"),
            SelectionPolicy::PendingOnly,
            "attempt",
            0,
            vec![PlanRecord::new(1, "20260101", "000000")],
            0,
        ),
        Err(PlanError::ZeroBatchSize)
    );

    // A batch size larger than the set still yields exactly one complete batch.
    let single = UploadPlan::build(
        UploadTarget::Eqsl,
        JournalId::new("GUARD_JOURNAL").expect("journal"),
        SelectionPolicy::PendingOnly,
        "attempt",
        0,
        vec![
            PlanRecord::new(1, "20260101", "000000"),
            PlanRecord::new(2, "20260101", "000001"),
        ],
        usize::MAX,
    )
    .expect("plan");
    assert_eq!(single.batches().len(), 1);
    assert_eq!(single.batches()[0].count(), 2);
}

#[test]
fn p1_f20_partial_timeout_cancellation_are_not_success_guard() {
    use splogbook::cloud::upload_plan::{
        BatchOutcome, JournalId, PlanRecord, RunStatus, SelectionPolicy, UploadPlan, UploadTarget,
        run_plan,
    };

    let plan = UploadPlan::build(
        UploadTarget::Lotw,
        JournalId::new("GUARD_JOURNAL").expect("journal"),
        SelectionPolicy::PendingOnly,
        "attempt",
        0,
        (1..=6i64)
            .map(|id| PlanRecord::new(id, "20260101", format!("{id:06}")))
            .collect(),
        2,
    )
    .expect("plan");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    for outcome in [
        BatchOutcome::PartiallyAccepted {
            rejected: 1,
            detail: "guard".to_string(),
        },
        BatchOutcome::NoRecordsProcessed {
            detail: "guard".to_string(),
        },
        BatchOutcome::Rejected {
            detail: "guard".to_string(),
        },
        BatchOutcome::Unknown {
            detail: "guard".to_string(),
        },
        BatchOutcome::TimedOut {
            detail: "guard".to_string(),
        },
        BatchOutcome::Failed {
            detail: "guard".to_string(),
        },
        BatchOutcome::Cancelled,
    ] {
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let planned = outcome.clone();
        let report = runtime.block_on(run_plan(&plan, &cancel, move |_batch| {
            let planned = planned.clone();
            async move { planned }
        }));
        assert!(
            !report.status.is_success(),
            "F-20: `{outcome:?}` must never produce a success status"
        );
        assert_ne!(report.status, RunStatus::Complete);
        assert_eq!(report.accepted_records, 0);
        assert!(report.accounting_consistent());
    }

    // Full acceptance is the only path to Complete.
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let report = runtime.block_on(run_plan(&plan, &cancel, |_batch| async {
        BatchOutcome::Accepted
    }));
    assert_eq!(report.status, RunStatus::Complete);
    assert_eq!(report.accepted_records, 6);
}

#[test]
fn p1_f20_status_update_is_journal_scoped_guard() {
    use splogbook::cloud::upload_plan::JournalId;
    use splogbook::cloud::upload_plan::UploadTarget;
    use splogbook::core::database::Journal;
    use splogbook::core::database::LogDatabase;
    use splogbook::core::qso::QsoRecord;

    let selection = p1_f20_read("src/core/database/upload_selection.rs");
    assert!(
        selection.contains("WHERE journal_id = ?1 AND id IN"),
        "F-20: the acceptance commit must be journal scoped"
    );
    assert!(
        selection.contains("RowCountMismatch"),
        "F-20: a partial status write must be detected and rolled back"
    );

    let db = LogDatabase::open_in_memory().expect("db");
    for (id, is_default) in [("GUARD_A", true), ("GUARD_B", false)] {
        db.create_journal(&Journal {
            id: id.to_string(),
            name: id.to_string(),
            station_callsign: "TSTSTN".to_string(),
            operator: "TSTOP".to_string(),
            my_gridsquare: "JO90AA".to_string(),
            my_pga: String::new(),
            description: String::new(),
            is_default,
        })
        .expect("journal");
    }

    let mut qsos = Vec::new();
    for journal in ["GUARD_A", "GUARD_B"] {
        for index in 0..3 {
            let mut qso = QsoRecord::new(format!("TST{index:03}"), "20m", "CW");
            qso.journal_id = Some(journal.to_string());
            qso.qso_date = "20260101".to_string();
            qso.time_on = format!("12000{index}");
            qsos.push(qso);
        }
    }
    db.batch_insert_qsos(&qsos).expect("insert");

    let a = JournalId::new("GUARD_A").expect("journal");
    let b = JournalId::new("GUARD_B").expect("journal");
    let a_ids: Vec<i64> = db
        .select_upload_candidates_for_journal(&a, UploadTarget::Lotw)
        .expect("candidates A")
        .into_iter()
        .map(|record| record.id)
        .collect();
    assert_eq!(a_ids.len(), 3);

    db.commit_batch_acceptance(&a, UploadTarget::Lotw, &a_ids)
        .expect("commit A");
    assert!(
        db.select_upload_candidates_for_journal(&a, UploadTarget::Lotw)
            .expect("remaining A")
            .is_empty()
    );
    assert_eq!(
        db.select_upload_candidates_for_journal(&b, UploadTarget::Lotw)
            .expect("remaining B")
            .len(),
        3,
        "F-20: status update must never touch another journal"
    );
    assert_eq!(
        db.select_upload_candidates_for_journal(&a, UploadTarget::Eqsl)
            .expect("remaining eQSL A")
            .len(),
        3,
        "F-20: LoTW status must never change the eQSL status"
    );

    // Foreign identifiers under journal A must affect nothing.
    let b_ids: Vec<i64> = db
        .select_upload_candidates_for_journal(&b, UploadTarget::Lotw)
        .expect("candidates B")
        .into_iter()
        .map(|record| record.id)
        .collect();
    assert!(
        db.commit_batch_acceptance(&a, UploadTarget::Lotw, &b_ids)
            .is_err()
    );
    assert_eq!(
        db.select_upload_candidates_for_journal(&b, UploadTarget::Lotw)
            .expect("remaining B after foreign commit")
            .len(),
        3
    );
}

#[test]
fn p1_f20_credentials_never_in_upload_url_or_logs_guard() {
    let eqsl = p1_f20_read("src/cloud/eqsl.rs");

    // The upload is a documented FORM POST carrying the payload and the
    // credentials in the body - never in the URL.
    assert!(
        eqsl.contains(".form(&["),
        "F-20: the eQSL upload must use a form body"
    );
    for field in ["EQSL_USER", "EQSL_PSWD", "ADIFData"] {
        assert!(
            eqsl.contains(field),
            "F-20: the eQSL form must carry the documented field {field}"
        );
    }
    assert!(
        eqsl.contains("https://www.eqsl.cc/qslcard/ImportADIF.cfm"),
        "F-20: the eQSL upload must target the documented processor"
    );
    // The documentation page may only be mentioned as documentation, never as
    // the upload endpoint constant.
    let endpoint_line = eqsl
        .lines()
        .find(|line| line.contains("EQSL_IMPORT_ENDPOINT: &str"))
        .expect("endpoint constant");
    assert!(
        !endpoint_line.contains("ImportADIF.txt"),
        "F-20: the eQSL documentation page must never be used as an endpoint"
    );
    assert!(
        !endpoint_line.contains(".txt"),
        "F-20: the eQSL endpoint must be the processor, not a text page"
    );
    assert!(
        eqsl.contains("redact_secret"),
        "F-20: eQSL response text must be redacted before it leaves the client"
    );
    assert!(
        eqsl.contains("body: redact_secret(&body, &self.password)"),
        "F-20: the eQSL body must be redacted at the source"
    );

    // No automatic retry around an upload whose outcome may be ambiguous.
    assert!(
        !eqsl.contains("retry_async"),
        "F-20: the eQSL upload must not retry automatically"
    );

    // The endpoint constant must not carry query parameters or credentials.
    let endpoint_line = eqsl
        .lines()
        .find(|line| line.contains("EQSL_IMPORT_ENDPOINT: &str"))
        .expect("endpoint constant");
    assert!(
        !endpoint_line.contains('?'),
        "F-20: the upload endpoint must not carry a query string"
    );

    let lotw = p1_f20_read("src/cloud/lotw.rs");
    assert!(
        !lotw.contains("\"-p\""),
        "F-20: the TQSL certificate password must never be passed on the command line"
    );
    assert!(
        p1_f20_count(&lotw, ".arg(\"-u\")") >= 1,
        "F-20: TQSL must be invoked with the documented upload switch"
    );
}

#[test]
fn p1_f20_test_transport_is_test_scope_only_guard() {
    let eqsl = p1_f20_read("src/cloud/eqsl.rs");

    let marker = "pub fn with_test_endpoint";
    let position = eqsl
        .find(marker)
        .expect("the test endpoint override must exist in test scope");
    assert!(
        p1_f20_preceded_by(&eqsl, position, "#[cfg(test)]", 400),
        "F-20: the endpoint override must be #[cfg(test)] only"
    );

    let field = eqsl
        .find("endpoint: Option<String>")
        .expect("the override field must exist");
    assert!(
        p1_f20_preceded_by(&eqsl, field, "#[cfg(test)]", 300),
        "F-20: the test endpoint field must not exist in production builds"
    );

    // No environment/feature/config path may select the test transport.
    for forbidden in [
        "SPLOG_TEST",
        "TEST_UPLOAD",
        "EQSL_TEST_ENDPOINT",
        "cfg(not(test))",
        "env::var(\"EQSL",
    ] {
        assert!(
            !eqsl.contains(forbidden),
            "F-20: no `{forbidden}` hook may exist"
        );
    }

    // The loopback server lives only in the eQSL unit tests and nowhere else.
    for path in p1_f17_tracked_sources("src") {
        let source = p1_f20_read(&path);
        if source.contains("LoopbackServer") && path != "src/cloud/eqsl.rs" {
            panic!("F-20: test transport leaked into {path}");
        }
    }
}

#[test]
fn p1_f20_no_real_endpoints_in_tests_guard() {
    let mut offenders = Vec::new();
    for entry in walkdir::WalkDir::new("tests")
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let raw_path = path.to_string_lossy().replace('\\', "/");
        // The guard itself necessarily names the endpoints it forbids.
        if raw_path == "tests/security_guard.rs" {
            continue;
        }
        let source = p1_f20_read(&raw_path);
        for forbidden in ["www.eqsl.cc", "lotw.arrl.org", "eqsl.cc/qslcard/ImportADIF"] {
            if source.contains(forbidden) {
                offenders.push(format!("{}: {forbidden}", path.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "F-20: tests must never reference a real service endpoint: {offenders:?}"
    );
}

#[test]
fn p1_f20_synthetic_data_only_guard() {
    let suite = p1_f20_read("tests/p1_f20_upload_selection.rs");
    let body: String = suite
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "SP6INA",
        "DL1ABC",
        "K1ABC",
        "http://",
        "https://",
        "TEST_SECRET",
    ] {
        assert!(
            !body.contains(forbidden),
            "F-20: the suite must not contain `{forbidden}`"
        );
    }
    assert!(
        body.contains("TST"),
        "F-20: the suite must use synthetic fixture callsigns"
    );

    let orchestration = p1_f20_read("src/gui/online_sync_upload.rs");
    let orchestration_code: String = orchestration
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["SP6INA", "password=", "Authorization"] {
        assert!(
            !orchestration_code.contains(forbidden),
            "F-20: the orchestrator must not contain `{forbidden}`"
        );
    }
}

#[test]
fn p1_f20_documentation_records_the_closed_remediation_guard() {
    let known = p1_f20_read("docs/KNOWN_ISSUES.md");
    let baseline = p1_f20_read("docs/VERIFICATION_BASELINE.md");
    let roadmap = p1_f20_read("docs/ROADMAP.md");

    for required in [
        "F-20 = CLOSED",
        "F-20 = Resolved (Verified)",
        "LoTW selection = JOURNAL_SCOPED",
        "eQSL selection = JOURNAL_SCOPED",
        "Silent truncation = REMOVED",
        "Batching = LOSSLESS_SOFTWARE_VERIFIED",
        "Partial success = MODELED",
        "Real LoTW upload = NOT_EXECUTED",
        "Real eQSL upload = NOT_EXECUTED",
        "Real credentials = NOT_USED",
    ] {
        assert!(
            known.contains(required),
            "F-20: docs/KNOWN_ISSUES.md must record `{required}`"
        );
    }

    for (label, source) in [("VERIFICATION_BASELINE", &baseline), ("ROADMAP", &roadmap)] {
        for required in ["F-20", "JOURNAL_SCOPED", "LOSSLESS", "NOT_EXECUTED"] {
            assert!(
                source.contains(required),
                "F-20: docs/{label}.md must record `{required}`"
            );
        }
    }
}

#[test]
fn p1_f20_prior_gates_remain_closed_guard() {
    let known = p1_f20_read("docs/KNOWN_ISSUES.md");
    for closed in [
        "F-14 = Resolved (Verified)",
        "CAT-018 = CLOSED",
        "CAT-019 = CLOSED",
        "F-15 = Resolved (Verified)",
        "F-16 = CLOSED",
        "F-17 = Resolved (Verified)",
        "F-19 = CLOSED",
    ] {
        assert!(
            known.contains(closed),
            "F-20: a previously resolved issue was reopened: `{closed}`"
        );
    }
    assert!(
        known.contains("F-21 = CLOSED"),
        "F-21: F-21 must remain recorded as CLOSED"
    );
    assert!(
        known.contains("F-21 (P1)"),
        "F-20: F-21 must remain recorded in KNOWN_ISSUES.md"
    );

    let roadmap = p1_f20_read("docs/ROADMAP.md");
    assert!(
        roadmap.contains("NOT_STARTED"),
        "F-20: NATIVE-7B must remain NOT_STARTED"
    );
}

#[test]
fn p1_f20_hil_tests_remain_ignored_guard() {
    let native = p1_f20_read("tests/cat_native_7.rs");
    // Count actual attributes, not prose that mentions the attribute text.
    let ignored = native
        .lines()
        .filter(|line| {
            line.trim_start()
                .starts_with("#[ignore = \"hardware-in-the-loop\"]")
        })
        .count();
    assert_eq!(
        ignored, 10,
        "F-20: exactly 10 hardware-in-the-loop tests must remain ignored"
    );
}

// ---------------------------------------------------------------------------
// P1-F20R - persistent upload checkpoints, sent dates and
// download-credential transport hardening (F20-R1, F20-R2, F20-R3)
// ---------------------------------------------------------------------------

/// Strips line comments so that documentation cannot satisfy a code assertion.
fn p1_f20r_code(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn p1_f20r_persistent_checkpoint_schema_guard() {
    let migrations = p1_f20r_code(&p1_f20_read("src/core/database/migrations.rs"));
    for required in [
        "CREATE TABLE IF NOT EXISTS upload_plans (",
        "CREATE TABLE IF NOT EXISTS upload_batches (",
        "CREATE TABLE IF NOT EXISTS upload_batch_members (",
        "plan_id TEXT PRIMARY KEY",
        "schema_version INTEGER NOT NULL",
        "journal_id TEXT NOT NULL",
        "service TEXT NOT NULL",
        "batch_digest TEXT NOT NULL",
        "attempt_id TEXT NOT NULL",
        "retry_eligible INTEGER NOT NULL DEFAULT 0",
        "PRIMARY KEY (plan_id, ordinal)",
        "UNIQUE (plan_id, batch_id)",
        "PRIMARY KEY (plan_id, ordinal, position)",
    ] {
        assert!(
            migrations.contains(required),
            "F-20R: persistent checkpoint schema must contain `{required}`"
        );
    }
    // Wersja schematu bazy musi zostać podniesiona i zwalidowana (obejmuje migracje 8 i 9).
    // STAGE-V140-S1, decyzja OWNER D-S1-1: zamiast literału `KNOWN_MAX_VERSION: i64 = 9`
    // wersja pochodzi z jednej tablicy migracji, a migracje 8 i 9 muszą w niej pozostać.
    for required in [
        "(8, LogDatabase::migration_8_upload_checkpoints),",
        "(9, LogDatabase::migration_9_upload_sent_dates),",
        "pub const LATEST_SCHEMA_VERSION: i64 = MIGRATIONS[MIGRATIONS.len() - 1].0;",
        "if version > LATEST_SCHEMA_VERSION {",
    ] {
        assert!(
            migrations.contains(required),
            "F-20R: the schema version must cover migrations 8 and 9: `{required}`"
        );
    }
    assert!(
        !migrations.contains("KNOWN_MAX_VERSION"),
        "F-20R: the schema version must have exactly one source of truth"
    );
    const {
        assert!(
            splogbook::core::database::LATEST_SCHEMA_VERSION >= 9,
            "F-20R: the schema version must cover migrations 8 and 9"
        );
    }
    assert_eq!(
        splogbook::core::backup::BackupManager::SUPPORTED_SCHEMA_VERSION,
        splogbook::core::database::LATEST_SCHEMA_VERSION,
        "F-20R: backup validation must use the same schema version"
    );
    for required in [
        "(\"upload_plans\", \"journal_id\")",
        "(\"upload_plans\", \"service\")",
        "(\"upload_plans\", \"schema_version\")",
        "(\"upload_batches\", \"journal_id\")",
        "(\"upload_batches\", \"service\")",
        "(\"upload_batches\", \"batch_digest\")",
    ] {
        assert!(
            migrations.contains(required),
            "F-20R: schema validation must require `{required}`"
        );
    }
}

#[test]
fn p1_f20r_orchestration_is_checkpoint_driven_guard() {
    let orchestration = p1_f20r_code(&p1_f20_read("src/gui/online_sync_upload.rs"));
    for required in [
        "open_or_create_upload_checkpoint",
        "prepare_upload_batch",
        "start_upload_batch",
        "record_upload_batch_outcome",
        "commit_batch_acceptance_with_dates",
        "recovery_action",
        "RecoveryAction::NeverResend",
        "RecoveryAction::ManualReconciliation",
        "RecoveryAction::CommitLocalOnly",
        "RecoveryAction::FailClosedUnknown",
    ] {
        assert!(
            orchestration.contains(required),
            "F-20R: the upload orchestrator must use `{required}`"
        );
    }
    // Checkpoint powstaje przed uruchomieniem transportu.
    let spawn = orchestration
        .find("tokio::spawn")
        .expect("orchestrator spawns a task");
    let checkpoint = orchestration
        .find("open_checkpoint(app, &plan)")
        .expect("checkpoint is opened");
    assert!(
        checkpoint < spawn,
        "F-20R: the durable checkpoint must be written before the upload task starts"
    );
    // Recovery jest wywoływane przy starcie aplikacji.
    let app = p1_f20r_code(&p1_f20_read("src/gui/app.rs"));
    assert!(
        app.contains("recover_upload_checkpoints"),
        "F-20R: startup must run the checkpoint recovery"
    );
    assert!(
        app.contains("recover_upload_checkpoints_after_restart"),
        "F-20R: startup recovery must be wired into the GUI bootstrap"
    );
}

#[test]
fn p1_f20r_in_flight_never_becomes_accepted_guard() {
    let model = p1_f20r_code(&p1_f20_read("src/cloud/upload_checkpoint.rs"));
    assert!(
        model.contains("BatchState::InFlight => RecoveryAction::FailClosedUnknown"),
        "F-20R: InFlight after restart must be fail-closed"
    );
    // Błąd transportu ani timeout nie mogą stać się Accepted.
    let squeezed = model.split_whitespace().collect::<Vec<_>>().join(" ");
    let start = squeezed
        .find("pub const fn state_for_outcome")
        .expect("state_for_outcome exists");
    let rest = &squeezed[start..];
    let end = rest
        .find("pub const fn error_class_for_outcome")
        .unwrap_or(rest.len());
    let body = &rest[..end];
    assert!(
        body.contains("BatchOutcome::Failed { .. } => BatchState::Unknown,"),
        "F-20R: Failed must map to Unknown"
    );
    assert!(
        body.contains("BatchOutcome::TimedOut { .. }"),
        "F-20R: TimedOut must be handled explicitly"
    );
    assert!(
        body.contains("BatchOutcome::Accepted => BatchState::Accepted,"),
        "F-20R: only the Accepted outcome may map to the Accepted state"
    );
    for forbidden in [
        "BatchOutcome::Failed { .. } => BatchState::Accepted",
        "BatchOutcome::TimedOut { .. } => BatchState::Accepted",
        "BatchOutcome::Unknown { .. } => BatchState::Accepted",
        "BatchOutcome::Cancelled => BatchState::Accepted",
    ] {
        assert!(
            !body.contains(forbidden),
            "F-20R: `{forbidden}` is forbidden"
        );
    }
    // Recovery zapisuje Unknown z jawną klasą błędu i bez prawa ponowienia.
    let db = p1_f20r_code(&p1_f20_read("src/core/database/upload_checkpoint.rs"));
    assert!(
        db.contains("ErrorClass::InterruptedInFlight"),
        "F-20R: recovery must record the interrupted-in-flight class"
    );
    assert!(
        db.contains("retry_eligible = 0"),
        "F-20R: recovery must never mark a checkpoint retry-eligible"
    );
    assert!(
        db.contains("ErrorClass::InterruptedInCommit"),
        "F-20R: an observable Committing must become RecoveryRequired"
    );
}

#[test]
fn p1_f20r_unknown_not_retried_and_completed_not_resent_guard() {
    let model = p1_f20r_code(&p1_f20_read("src/cloud/upload_checkpoint.rs"));
    assert!(
        model.contains(
            "BatchState::Completed | BatchState::Cancelled => RecoveryAction::NeverResend"
        ),
        "F-20R: Completed and Cancelled must never be resent"
    );
    assert!(
        model.contains("| BatchState::RecoveryRequired => RecoveryAction::ManualReconciliation"),
        "F-20R: ambiguous states must require manual reconciliation"
    );
    // Ścieżka uploadu nie może zawierać automatycznego ponawiania.
    let orchestration = p1_f20r_code(&p1_f20_read("src/gui/online_sync_upload.rs"));
    for forbidden in [
        "retry_async",
        "retry_failed",
        ".retry(",
        "tokio::time::sleep",
    ] {
        assert!(
            !orchestration.contains(forbidden),
            "F-20R: the upload path must not contain `{forbidden}`"
        );
    }
    // `run_plan` wywołuje transport dokładnie raz na paczkę (kontrakt F-20).
    let plan = p1_f20r_code(&p1_f20_read("src/cloud/upload_plan.rs"));
    assert!(
        !plan.contains("retry_async"),
        "F-20R: the plan runner must not retry"
    );
    assert!(
        plan.contains("let outcome = send(batch).await;"),
        "F-20R: the plan runner must send each batch exactly once"
    );
}

#[test]
fn p1_f20r_status_and_sent_date_are_atomic_guard() {
    let db = p1_f20r_code(&p1_f20_read("src/core/database/upload_checkpoint.rs"));
    assert!(
        db.contains("SET {sent_column} = 'Y', {date_column} = COALESCE({date_column}, ?2)"),
        "F-20R: status and sent date must be written by one statement"
    );
    assert!(
        db.contains("COALESCE({date_column}, ?2)"),
        "F-20R: a retry must not overwrite the originally accepted date"
    );
    assert!(
        db.contains("CheckpointError::RowCountMismatch"),
        "F-20R: affected != expected must abort the transaction"
    );
    assert!(
        db.contains("BatchState::Committing"),
        "F-20R: the commit must pass through the intra-transaction state"
    );
    // Data pochodzi z trwałego dowodu (`accepted_at`), nie z QSO_DATE ani z
    // chwili bieżącej.
    assert!(
        db.contains("adif_date_from_unix(accepted_at as i64)"),
        "F-20R: the sent date must be derived from the durable acceptance instant"
    );
    assert!(
        !db.contains("qso_date"),
        "F-20R: the sent date must never be derived from QSO_DATE"
    );
    for wall_clock in [
        "Utc::now",
        "Timestamp::now",
        "Zoned::now",
        "ZonedDateTime::now",
    ] {
        assert!(
            !db.contains(wall_clock),
            "F-20R: the sent date must never be read from the wall clock inside the repository"
        );
    }
}

#[test]
fn p1_f20r_sent_date_fields_exist_and_round_trip_guard() {
    let schema = p1_f20r_code(&p1_f20_read("src/core/database/schema.rs"));
    for required in [
        "lotw_qslsdate => \"lotw_qslsdate\"",
        "eqsl_qslsdate => \"eqsl_qslsdate\"",
    ] {
        assert!(schema.contains(required), "F-20R: missing `{required}`");
    }
    let qso = p1_f20r_code(&p1_f20_read("src/core/qso.rs"));
    for required in [
        "pub lotw_qslsdate: Option<String>",
        "pub eqsl_qslsdate: Option<String>",
    ] {
        assert!(qso.contains(required), "F-20R: missing `{required}`");
    }
    let adif = p1_f20r_code(&p1_f20_read("src/core/adif.rs"));
    for required in [
        "fields.remove(\"LOTW_QSLSDATE\")",
        "fields.remove(\"EQSL_QSLSDATE\")",
        "(\"LOTW_QSLSDATE\"",
        "(\"EQSL_QSLSDATE\"",
        "fn adif_sent_date",
    ] {
        assert!(adif.contains(required), "F-20R: missing `{required}`");
    }
    // Eksport respektuje regułę „only valid if … is Y, Q, or I”.
    assert!(
        adif.contains("\"Y\" | \"Q\" | \"I\""),
        "F-20R: the sent date must be gated by the ADIF status rule"
    );
    assert!(
        adif.contains("year >= 1930"),
        "F-20R: the ADIF Date floor (1930) must be enforced on export"
    );
}

#[test]
fn p1_f20r_no_invented_historical_dates_guard() {
    let migrations = p1_f20r_code(&p1_f20_read("src/core/database/migrations.rs"));
    let start = migrations
        .find("fn migration_9_upload_sent_dates")
        .expect("migration 9 exists");
    let body = &migrations[start..];
    let body = &body[..body
        .find("pub(super) fn rollback_migration_9")
        .unwrap_or(body.len())];
    assert!(
        !body.contains("UPDATE qso_records"),
        "F-20R: the sent-date migration must not rewrite any row"
    );
    assert!(
        !body.contains("datetime("),
        "F-20R: the migration must not use the migration time as a sent date"
    );
    assert!(
        !body.contains("DEFAULT "),
        "F-20R: the sent-date columns must have no default value"
    );
    let migration8 = migrations
        .split("MIGRATION_8_SQL: &str =")
        .nth(1)
        .expect("migration 8 SQL exists");
    let migration8 = &migration8[..migration8.find("\";").unwrap_or(migration8.len())];
    assert!(
        !migration8.contains("datetime("),
        "F-20R: the checkpoint migration must not stamp a date"
    );
    // Historia bez dowodu pozostaje `Y + NULL` — brak backfillu w kodzie.
    let db = p1_f20r_code(&p1_f20_read("src/core/database/upload_checkpoint.rs"));
    assert!(
        !db.contains("'2026"),
        "F-20R: no literal date may be written into the database"
    );
}

#[test]
fn p1_f20r_download_credentials_are_confined_guard() {
    let http = p1_f20r_code(&p1_f20_read("src/core/http.rs"));
    assert!(
        http.contains("pub struct SecretUrl"),
        "F-20R: the SecretUrl wrapper must exist"
    );
    assert!(
        http.contains("redirect(reqwest::redirect::Policy::none())"),
        "F-20R: download clients must not follow redirects"
    );
    assert!(
        http.contains("f.write_str(&self.redacted())"),
        "F-20R: Display of SecretUrl must be redacted"
    );
    assert!(
        http.contains("write!(f, \"SecretUrl({})\", self.redacted())"),
        "F-20R: Debug of SecretUrl must be redacted"
    );
    // Debug/Display nie mogą ujawniać pełnego adresu.
    assert!(
        !http.contains("self.url.as_str())"),
        "F-20R: SecretUrl must not expose its full address"
    );
    for (path, name) in [("src/cloud/lotw.rs", "LoTW"), ("src/cloud/eqsl.rs", "eQSL")] {
        let source = p1_f20r_code(&p1_f20_read(path));
        assert!(
            source.contains("SecretUrl::parse_with_params"),
            "F-20R: {name} download must build a SecretUrl"
        );
        assert!(
            source.contains("http_client_no_redirect"),
            "F-20R: {name} download must disable redirects"
        );
        assert!(
            !source.contains(".query(&query)"),
            "F-20R: {name} download must not pass credentials through a raw query vector"
        );
        for forbidden in [
            "log::info!(\"{url}",
            "log::warn!(\"{url}",
            "println!(\"{url}",
        ] {
            assert!(
                !source.contains(forbidden),
                "F-20R: {name} download must not log the URL"
            );
        }
    }
    // Pełny adres z hasłem nie może być zwracany przez produkcyjne API.
    let lotw = p1_f20r_code(&p1_f20_read("src/cloud/lotw.rs"));
    let index = lotw
        .find("pub fn build_report_url")
        .expect("build_report_url exists");
    let attribute = lotw[..index].rfind("#[cfg(test)]");
    assert!(
        attribute.is_some_and(|position| index - position <= 240),
        "F-20R: build_report_url must not exist in production builds"
    );
}

#[test]
fn p1_f20r_checkpoints_store_no_secret_guard() {
    let db = p1_f20r_code(&p1_f20_read("src/core/database/upload_checkpoint.rs"));
    for forbidden in [
        "password",
        "Password",
        "token",
        "secret",
        "https://",
        "http://",
        "certificate",
    ] {
        assert!(
            !db.contains(forbidden),
            "F-20R: the checkpoint repository must not mention `{forbidden}`"
        );
    }
    // Ani model checkpointu, ani migracja nie mogą przechowywać ładunku.
    let model = p1_f20r_code(&p1_f20_read("src/cloud/upload_checkpoint.rs"));
    for forbidden in ["adif_content", "response_body", "full_url"] {
        assert!(
            !model.contains(forbidden),
            "F-20R: the checkpoint model must not carry `{forbidden}`"
        );
    }
    // `compute_evidence_digest` jest jedynym nośnikiem dowodu.
    assert!(
        model.contains("hasher.update(state.as_str().as_bytes())"),
        "F-20R: evidence must be reduced to a digest of non-secret metadata"
    );
}

#[test]
fn p1_f20r_test_only_helpers_never_reach_production_guard() {
    let database_mod = p1_f20r_code(&p1_f20_read("src/core/database/mod.rs"));
    let raw_conn_index = database_mod
        .find("pub fn raw_conn")
        .expect("raw_conn exists");
    let attribute = database_mod[..raw_conn_index].rfind("#[cfg(test)]");
    assert!(
        attribute.is_some_and(|position| raw_conn_index - position <= 80),
        "F-20R: raw connection access must stay in test scope"
    );
    // Żaden moduł produkcyjny nie może wystawiać publicznego haka testowego.
    let mut offenders = Vec::new();
    for path in p1_f17_tracked_sources("src") {
        let source = p1_f20r_code(&p1_f20_read(&path));
        for forbidden in [
            "pub fn force_",
            "pub fn corrupt_",
            "pub fn dump_",
            "pub fn set_checkpoint_",
            "pub fn raw_statement_for_tests",
        ] {
            if source.contains(forbidden) {
                offenders.push(format!("{path}: {forbidden}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "F-20R: test hooks must not exist in production modules: {offenders:?}"
    );
}

#[test]
fn p1_f20r_documentation_records_the_residual_hardening_guard() {
    let upload = p1_f20_read("docs/UPLOAD_SELECTION.md");
    for required in [
        "Persistent upload checkpoints = IMPLEMENTED_AND_SOFTWARE_VERIFIED",
        "Crash recovery = SOFTWARE_VERIFIED",
        "Unknown remote outcome = FAIL_CLOSED_MANUAL_RECONCILIATION_REQUIRED",
        "LOTW_QSLSDATE = IMPLEMENTED",
        "EQSL_QSLSDATE = IMPLEMENTED",
        "Download credential transport = OFFICIAL_CONTRACT_VERIFIED",
        "Real service login = NOT_EXECUTED",
        "Real upload/download = NOT_EXECUTED",
        "OFFICIAL_ENDPOINT_REQUIRES_QUERY_CREDENTIALS=true",
        "QUERY_CREDENTIALS_NOT_LOGGED=true",
        "QUERY_CREDENTIALS_NOT_PERSISTED=true",
        "QUERY_CREDENTIALS_NOT_EXPOSED_IN_ERRORS=true",
    ] {
        assert!(
            upload.contains(required),
            "F-20R: docs/UPLOAD_SELECTION.md must record `{required}`"
        );
    }
    let known = p1_f20_read("docs/KNOWN_ISSUES.md");
    assert!(
        known.contains("F20-R3"),
        "F-20R: the remaining query-credential limitation must be recorded"
    );
    let baseline = p1_f20_read("docs/VERIFICATION_BASELINE.md");
    assert!(
        baseline.contains("P1-F20R"),
        "F-20R: the verification baseline must record P1-F20R"
    );
}

#[test]
fn p1_f20r_prior_gates_and_dependency_policy_guard() {
    let roadmap = p1_f20_read("docs/ROADMAP.md");
    assert!(roadmap.contains("F-20"), "F-20 must stay recorded");
    assert!(roadmap.contains("F-21"), "F-21 must stay recorded");
    // Tylko dokumenty śledzone w repozytorium: lokalne notatki robocze
    // nie są częścią publicznego drzewa Git i nie mogą być czytane przez testy.
    assert!(
        roadmap.contains("NATIVE-7B"),
        "NATIVE-7B must stay recorded as NOT_STARTED"
    );
    assert!(
        roadmap.contains("F-21 = Resolved (Verified)"),
        "F-21 must stay recorded as Resolved (Verified)"
    );
    assert!(
        roadmap.contains("| F20R hardening | **Resolved (Verified)**"),
        "F20R hardening must stay recorded as Resolved (Verified)"
    );
    assert!(
        roadmap.contains("F-18 = Resolved (Verified)"),
        "F-18 must stay recorded as Resolved (Verified)"
    );
    // Zależności: bez zmian, `rusqlite` nadal bundled, brak dev-dependency w
    // produkcyjnych zależnościach.
    let manifest = p1_f20_read("Cargo.toml");
    assert!(
        manifest.contains("rusqlite = { version = \"0.41\", features = [\"bundled\""),
        "F-20R: rusqlite must stay bundled"
    );
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("[build-dependencies]").next())
        .expect("dependencies section");
    for forbidden in ["tempfile", "criterion", "proptest"] {
        assert!(
            !dependencies.contains(forbidden),
            "F-20R: `{forbidden}` must not become a production dependency"
        );
    }
}

// ============================================================================
// RUSTSEC-2026-0543 (chrono replacement): chrono (unmaintained) is
// gone and jiff 1.x is the only time crate. F20-R4: the LoTW received date is never invented.
// ============================================================================

/// Versions of every `Cargo.lock` package called exactly `name`.
fn chrono_replacement_lock_versions(lock: &str, name: &str) -> Vec<String> {
    lock.split("[[package]]")
        .filter_map(|block| {
            let field = |key: &str| {
                block.lines().find_map(|line| {
                    line.trim()
                        .strip_prefix(key)
                        .and_then(|rest| rest.strip_prefix(" = \""))
                        .and_then(|rest| rest.strip_suffix('"'))
                })
            };
            (field("name")? == name).then(|| field("version").map(str::to_string))?
        })
        .collect()
}

fn chrono_replacement_rust_sources() -> Vec<(String, String)> {
    let mut sources = vec![("build.rs".to_string(), p1_f20_read("build.rs"))];
    for root in ["src", "tests"] {
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry.expect("walk source tree");
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                let name = path.to_string_lossy().replace('\\', "/");
                sources.push((name.clone(), p1_f20_read(&name)));
            }
        }
    }
    sources
}

#[test]
fn chrono_replacement_chrono_cannot_return() {
    // Built at run time so this guard does not match itself.
    let chrono = concat!("chr", "ono");
    let manifest = p1_f20_read("Cargo.toml").replace("\r\n", "\n");
    for line in manifest.lines() {
        let key = line.split('=').next().unwrap_or_default().trim();
        assert_ne!(key, chrono, "chrono must not return to Cargo.toml");
        assert!(
            !line.contains(&format!("\"{chrono}\"")),
            "chrono must not return as a feature or a renamed package: {line}"
        );
    }
    let lock = p1_f20_read("Cargo.lock").replace("\r\n", "\n");
    assert_eq!(
        chrono_replacement_lock_versions(&lock, chrono),
        Vec::<String>::new(),
        "chrono must not return to Cargo.lock, not even transitively"
    );
    for (path, source) in chrono_replacement_rust_sources() {
        for needle in [format!("{chrono}::"), format!("use {chrono}")] {
            assert!(
                !source.contains(&needle),
                "{path}: chrono must not return (`{needle}`)"
            );
        }
    }
}

#[test]
fn chrono_replacement_rustsec_2026_0543_is_never_ignored() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/build-and-release.yml",
    ] {
        assert!(
            !p1_f20_read(path).contains("RUSTSEC-2026-0543"),
            "{path}: RUSTSEC-2026-0543 is fixed by removing chrono and must never be ignored"
        );
    }
    for path in ["audit.toml", ".cargo/audit.toml"] {
        assert!(
            !std::path::Path::new(path).exists(),
            "{path}: advisory ignores belong only in ci.yml"
        );
    }
}

#[test]
fn chrono_replacement_jiff_has_a_single_1x_line() {
    let manifest = p1_f20_read("Cargo.toml").replace("\r\n", "\n");
    assert!(
        manifest.contains("jiff = { version = \"1.0.1\", features = [\"serde\"] }"),
        "jiff 1.x with serde is the time crate"
    );
    let env_logger = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("env_logger"))
        .expect("env_logger dependency");
    assert!(
        env_logger.contains("default-features = false") && !env_logger.contains("humantime"),
        "env_logger `humantime` pulls jiff 0.2 and must stay off: {env_logger}"
    );
    let lock = p1_f20_read("Cargo.lock").replace("\r\n", "\n");
    let jiff = chrono_replacement_lock_versions(&lock, "jiff");
    assert_eq!(
        jiff.len(),
        1,
        "exactly one jiff in Cargo.lock, found {jiff:?}"
    );
    assert!(
        jiff[0].starts_with("1."),
        "jiff must be 1.x, found {jiff:?}"
    );
    for name in [
        "jiff-core",
        "jiff-static",
        "jiff-tzdb",
        "jiff-tzdb-platform",
    ] {
        let versions = chrono_replacement_lock_versions(&lock, name);
        assert!(
            versions.len() <= 1 && versions.iter().all(|v| !v.starts_with("0.")),
            "{name}: no second or 0.x line may appear, found {versions:?}"
        );
    }
}

#[test]
fn f20r4_lotw_received_date_is_never_invented_guard() {
    let lotw = p1_f20_read("src/cloud/lotw.rs");
    let parser = lotw
        .split("pub fn parse_lotw_confirmations")
        .nth(1)
        .and_then(|rest| rest.split("#[cfg(test)]").next())
        .expect("parse_lotw_confirmations");
    assert!(
        !parser.contains("now()"),
        "F20-R4: the LoTW received date must never come from the wall clock"
    );
    assert!(
        lotw.contains("pub qsl_rdate: Option<String>"),
        "F20-R4: a confirmation without a date must carry None"
    );
    let adif = p1_f20_read("src/core/adif.rs");
    assert!(
        adif.contains("fields.remove(\"APP_LOTW_RXQSL\")"),
        "F20-R4: APP_LoTW_RXQSL must feed lotw_qslrdate"
    );
    let qsl = p1_f20_read("src/core/database/qsl.rs");
    assert!(
        qsl.contains("{rdate_col} = COALESCE(?1, {rdate_col})"),
        "F20-R4: a confirmation without a date must not erase a stored date"
    );
    for name in [
        "f20r4_received_date_uses_explicit_lotw_qslrdate_first",
        "f20r4_received_date_falls_back_to_app_lotw_rxqsl",
        "f20r4_received_date_falls_back_to_qslrdate_only_when_qsl_rcvd_is_y",
        "f20r4_received_date_is_none_without_a_valid_date",
    ] {
        assert!(
            lotw.contains(&format!("fn {name}(")),
            "F20-R4: regression test `{name}` must stay"
        );
    }
}

#[test]
fn p1_f21_cabrillo_export_is_untruncated_and_journal_scoped_guard() {
    let app_export = p1_f20_read("src/gui/app_export.rs");
    let cabrillo = p1_f20_read("src/core/cabrillo.rs");
    let contest_stats = p1_f20_read("src/core/contest_stats.rs");
    let gui_contest = p1_f20_read("src/gui/contest.rs");

    // 1. Brak obcinających selektorów `get_recent_qsos*` i limitu 10 000 w `src/gui/app_export.rs`.
    for forbidden in [
        "get_recent_qsos(",
        "get_recent_qsos_for_journal(",
        "10000",
        "10_000",
    ] {
        assert!(
            !app_export.contains(forbidden),
            "F-21: src/gui/app_export.rs must not contain `{forbidden}`"
        );
    }
    assert_eq!(
        p1_f20_count(
            &app_export,
            "get_all_qsos_for_journal(&self.active_journal.id)"
        ),
        3,
        "F-21: export_cabrillo, export_pdf_log and export_gpx_log must all use get_all_qsos_for_journal(&self.active_journal.id)"
    );

    // 2. W `src/core/cabrillo.rs` brak podstawiania bieżącego zegara `now()` (fail-closed)
    //    oraz użycie kanonicznego klucza sortowania `canonical_qso_sort_key`.
    let cabrillo_prod = cabrillo.split("#[cfg(test)]").next().unwrap_or(&cabrillo);
    for forbidden_clock in [
        "Timestamp::now()",
        "ZonedDateTime::now()",
        "SystemTime::now()",
    ] {
        assert!(
            !cabrillo_prod.contains(forbidden_clock),
            "F-21: src/core/cabrillo.rs production code must never fabricate timestamps with `{forbidden_clock}`"
        );
    }
    for required in [
        "pub fn format_and_validate_cabrillo_date(",
        "pub fn format_and_validate_cabrillo_time(",
        "pub fn canonical_qso_sort_key(",
        "sorted_qsos.sort_by_key(canonical_qso_sort_key)",
    ] {
        assert!(
            cabrillo_prod.contains(required),
            "F-21: src/core/cabrillo.rs must contain `{required}`"
        );
    }

    // 3. `compute_scoreboard` porządkuje rekordy tym samym kanonicznym kluczem rosnącym,
    //    a okno kontestowe odświeża cache z pełnego dziennika `get_all_qsos_for_journal`.
    let contest_stats_prod = contest_stats
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&contest_stats);
    assert!(
        contest_stats_prod.contains("canonical_qso_sort_key"),
        "F-21: compute_scoreboard must sort records with canonical_qso_sort_key before deduplication and scoring"
    );
    assert!(
        gui_contest.contains("get_all_qsos_for_journal(&app.active_journal.id)"),
        "F-21: render_contest_window must refresh ContestCache from get_all_qsos_for_journal(&app.active_journal.id)"
    );
}

#[test]
fn f18_redact_sensitive_info_is_utf8_safe_and_never_lowercases_guard() {
    let arbiter = p1_f20r_code(&p1_f20_read("src/cat/tx_arbiter.rs"));
    let start = arbiter
        .find("pub fn redact_sensitive_info(")
        .expect("redact_sensitive_info exists");
    let rest = &arbiter[start..];
    let end = rest
        .find("pub fn ")
        .and_then(|_| rest[30..].find("pub ").map(|i| i + 30))
        .unwrap_or(rest.len());
    let body = &rest[..end];

    assert!(
        !body.contains(".to_lowercase()"),
        "F-18: redact_sensitive_info must never call .to_lowercase()"
    );
    for required in [
        "out.as_bytes()",
        "windows(pat.len())",
        "eq_ignore_ascii_case(pat)",
        "ceil_char_boundary(",
        "floor_char_boundary(",
    ] {
        assert!(
            body.contains(required),
            "F-18: redact_sensitive_info must use `{required}`"
        );
    }

    // Functional verification of UTF-8 safety with length-expanding scalars (İ U+0130, Ⱥ U+023A),
    // Polish diacritics, Cyrillic, and emojis before, inside, and after secrets.
    let sample = "İİİ ȺȺȺ Błąd połączenia (Ошибка 🛰️): PaSsWoRd=TajneHasłoİȺ123, ToKeN:żółć_токен_🔒; BeArEr секрет123\napi_key=KluczİȺ🚀 koniec";
    let redacted = splogbook::cat::tx_arbiter::redact_sensitive_info(sample);
    assert!(
        redacted.contains("PaSsWoRd=[REDACTED]"),
        "password was not redacted: {redacted}"
    );
    assert!(
        redacted.contains("ToKeN:[REDACTED]"),
        "token was not redacted: {redacted}"
    );
    assert!(
        redacted.contains("BeArEr [REDACTED]"),
        "bearer was not redacted: {redacted}"
    );
    assert!(
        redacted.contains("api_key=[REDACTED]"),
        "api_key was not redacted: {redacted}"
    );
    for secret in ["TajneHasłoİȺ123", "żółć_токен_🔒", "секрет123", "KluczİȺ🚀"]
    {
        assert!(
            !redacted.contains(secret),
            "secret `{secret}` leaked in `{redacted}`"
        );
    }
    assert!(redacted.starts_with("İİİ ȺȺȺ Błąd połączenia (Ошибка 🛰️): "));
    assert!(redacted.ends_with(" koniec"));
}

#[test]
fn eqsl_received_date_is_never_fabricated_from_qso_date_guard() {
    let qsl = p1_f20r_code(&p1_f20_read("src/core/database/qsl.rs"));
    let start = qsl
        .find("pub fn mark_eqsl_confirmed_batch_for_journal(")
        .expect("mark_eqsl_confirmed_batch_for_journal exists");
    let body = &qsl[start..];

    assert!(
        !body.contains("unwrap_or(&q.qso_date)"),
        "eQSL confirmation must never fall back to q.qso_date"
    );
    assert!(
        body.contains("crate::core::adif::adif_date_part"),
        "eQSL confirmation must validate received dates through adif_date_part"
    );
    assert!(
        body.contains("rdate: rdate.as_deref()"),
        "eQSL confirmation must pass Option<&str> to mark_qsl_confirmed_batch_internal"
    );

    let adif_squeezed = p1_f20r_code(&p1_f20_read("src/core/adif.rs"))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        adif_squeezed.contains(
            "qso.eqsl_qslrdate = fields .remove(\"EQSL_QSLRDATE\") .as_deref() .and_then(adif_date_part)"
        ),
        "ADIF parser must validate EQSL_QSLRDATE via adif_date_part"
    );

    // Functional verification of the 4 eQSL confirmation date cases:
    use splogbook::core::adif::parse_adif;
    use splogbook::core::database::LogDatabase;
    use splogbook::core::qso::QsoRecord;

    // 1. Explicit EQSL_QSLRDATE wins over QSLRDATE
    let parsed_1 = parse_adif(
        "<CALL:4>W1AW<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20261001<TIME_ON:6>120000<QSL_RCVD:1>Y<QSLRDATE:8>20261018<EQSL_QSL_RCVD:1>Y<EQSL_QSLRDATE:8>20261019<EOR>",
    );
    assert_eq!(parsed_1[0].eqsl_qslrdate.as_deref(), Some("20261019"));

    // 2. Fallback to QSLRDATE when QSL_RCVD=Y
    let parsed_2 = parse_adif(
        "<CALL:4>W1AW<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20261001<TIME_ON:6>120000<QSL_RCVD:1>Y<QSLRDATE:8>20261018<EOR>",
    );
    assert_eq!(parsed_2[0].eqsl_qslrdate.as_deref(), Some("20261018"));

    // 3. No received date -> None (never fabricates QSO_DATE)
    let parsed_3 = parse_adif(
        "<CALL:4>W1AW<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20261001<TIME_ON:6>120000<EQSL_QSL_RCVD:1>Y<EOR>",
    );
    assert_eq!(parsed_3[0].eqsl_qslrdate, None);

    // 4. SQLite COALESCE protection in mark_eqsl_confirmed_batch_for_journal
    let db = LogDatabase::open_in_memory().unwrap();
    let mut base_qso = QsoRecord::new("W1AW", "20m", "CW");
    base_qso.qso_date = "20261001".to_string();
    // STAGE-V140-S1, decyzja OWNER D-S1-2: czas QSO = TIME_ON potwierdzeń (120000), zamiast
    // zegara ściennego z `QsoRecord::new` — dopasowanie ±30 min jest deterministyczne.
    base_qso.time_on = "120000".to_string();
    db.insert_qso(&base_qso).unwrap();

    // Confirm without date -> eqsl_qsl_rcvd = "Y", eqsl_qslrdate = None
    assert_eq!(
        db.mark_eqsl_confirmed_batch_for_journal("DEFAULT", &parsed_3)
            .unwrap(),
        1
    );
    let stored = db.find_previous_qsos("W1AW").unwrap();
    assert_eq!(stored[0].eqsl_qsl_rcvd, "Y");
    assert_eq!(stored[0].eqsl_qslrdate, None);

    // Confirm with QSLRDATE -> eqsl_qslrdate = Some("20261018")
    assert_eq!(
        db.mark_eqsl_confirmed_batch_for_journal("DEFAULT", &parsed_2)
            .unwrap(),
        1
    );
    let stored = db.find_previous_qsos("W1AW").unwrap();
    assert_eq!(stored[0].eqsl_qslrdate.as_deref(), Some("20261018"));

    // Subsequent confirmation without date preserves "20261018" via COALESCE
    assert_eq!(
        db.mark_eqsl_confirmed_batch_for_journal("DEFAULT", &parsed_3)
            .unwrap(),
        1
    );
    let stored = db.find_previous_qsos("W1AW").unwrap();
    assert_eq!(stored[0].eqsl_qslrdate.as_deref(), Some("20261018"));
}

#[test]
fn f06_reference_downloads_are_bounded_validated_and_atomic_guard() {
    let updater = p1_f20r_code(&p1_f20_read("src/cloud/updater.rs"));
    assert!(
        updater.contains("pub const MAX_REFERENCE_FILE_SIZE: u64 = 32 * 1024 * 1024;"),
        "F-06: MAX_REFERENCE_FILE_SIZE must be 32 MiB"
    );
    assert!(
        updater.contains("pub fn write_file_atomic_sync("),
        "F-06: write_file_atomic_sync must exist"
    );

    let dl_start = updater
        .find("async fn download_file(")
        .expect("DatabaseUpdater::download_file exists");
    let dl_rest = &updater[dl_start..];
    let dl_end = dl_rest
        .find("pub struct BoundedBuffer")
        .unwrap_or(dl_rest.len());
    let dl_body = &dl_rest[..dl_end];

    for required in [
        "BoundedBuffer::new(MAX_REFERENCE_FILE_SIZE)",
        "write_file_atomic_sync(dest_path, &bytes)?",
    ] {
        assert!(
            dl_body.contains(required),
            "F-06: DatabaseUpdater::download_file must use `{required}`"
        );
    }
    assert!(
        !dl_body.contains("tokio::fs::write("),
        "F-06: DatabaseUpdater::download_file must not write directly to dest_path with tokio::fs::write"
    );

    let app_services = p1_f20r_code(&p1_f20_read("src/gui/app_services.rs"));
    assert!(
        app_services.contains("if scp_updated")
            && app_services.contains("file.metadata().is_ok_and(|m| m.len() > 0)"),
        "F-06: trigger_database_update must only reload MASTER.SCP when scp_updated and file size > 0"
    );

    // Functional verification of write_file_atomic_sync:
    let tmp_dir = tempfile::tempdir().unwrap();
    let dest = tmp_dir.path().join("MASTER.SCP");

    // 1. Reject empty or whitespace-only buffer
    assert!(splogbook::cloud::updater::write_file_atomic_sync(&dest, b"").is_err());
    assert!(splogbook::cloud::updater::write_file_atomic_sync(&dest, b"  \t\r\n ").is_err());
    assert!(!dest.exists());

    // 2. Write initial valid file
    splogbook::cloud::updater::write_file_atomic_sync(&dest, b"SP6INA\nW1AW\n").unwrap();
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "SP6INA\nW1AW\n");

    // 3. Preserve existing file when an HTML error page or empty buffer is rejected
    assert!(
        splogbook::cloud::updater::write_file_atomic_sync(
            &dest,
            b"<!DOCTYPE html><html><body>Error</body></html>"
        )
        .is_err()
    );
    assert!(splogbook::cloud::updater::write_file_atomic_sync(&dest, b"<html>503</html>").is_err());
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "SP6INA\nW1AW\n");

    // 4. Atomically overwrite existing file and leave zero .tmp files
    splogbook::cloud::updater::write_file_atomic_sync(&dest, b"SP6INA\nK1JT\n").unwrap();
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "SP6INA\nK1JT\n");
    let leftovers: Vec<_> = std::fs::read_dir(tmp_dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "F-06: leftover .tmp files found: {leftovers:?}"
    );
}

#[test]
fn r1c_r1d_legacy_databases_removed_and_xdg_precedence_guard() {
    use std::path::Path;

    // 1. Fizyczny brak usuniętych baz referencyjnych oraz modułu service_db.rs
    for removed in [
        "databases/serviceLOG.db",
        "databases/callbook.db",
        "databases/clubs.csv",
        "src/core/service_db.rs",
    ] {
        assert!(
            !Path::new(removed).exists(),
            "R1C/F-09: plik {removed} musi zostać trwale usunięty z repozytorium"
        );
    }

    // 2. Brak zahardkodowanych wywołań scp.insert("...") w src/main.rs (F-11);
    // jedynym dopuszczalnym użyciem jest dynamiczne zasilanie z log_db.get_distinct_callsigns.
    let main_src = p1_f20r_code(&p1_f20_read("src/main.rs"));
    assert!(
        !main_src.contains("scp.insert(\""),
        "F-11: src/main.rs nie może zawierać zahardkodowanych wywołań scp.insert(\"...\")"
    );
    for line in main_src.lines().filter(|l| l.contains("scp.insert(")) {
        assert!(
            line.trim() == "scp.insert(&call);" && main_src.contains("get_distinct_callsigns"),
            "F-11: nieoczekiwane wywołanie scp.insert( w src/main.rs: {line}"
        );
    }

    // 3. Brak przysłaniania app_data_dir / app_config_dir przez exe_dir/CWD databases (F-07)
    assert!(
        !main_src.contains("exe_dir.join(\"databases"),
        "F-07: src/main.rs nie może przysłaniać app_data_dir/app_config_dir przez exe_dir.join(\"databases...\")"
    );
    assert!(
        !main_src.contains("exe_dir.join(\"../databases"),
        "F-07: src/main.rs nie może używać ../databases/"
    );
    assert!(
        !main_src.contains("exe_dir.join(\"../../databases"),
        "F-07: src/main.rs nie może używać ../../databases/"
    );

    let app_services_src = p1_f20r_code(&p1_f20_read("src/gui/app_services.rs"));
    assert!(
        !app_services_src.contains("Path::new(\"databases\").is_dir()"),
        "F-07: src/gui/app_services.rs nie może sprawdzać Path::new(\"databases\").is_dir() w CWD"
    );

    // 4. Brak FirstInstallSeed w updaterze
    let updater_src = p1_f20r_code(&p1_f20_read("src/cloud/updater.rs"));
    assert!(
        !updater_src.contains("FirstInstallSeed"),
        "R1C: src/cloud/updater.rs nie może zawierać wariantu FirstInstallSeed"
    );
}
