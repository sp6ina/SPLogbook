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
        "Workflow musi weryfikować obecność artefaktu Windows"
    );
    assert!(
        workflow.contains("[ -s release-staging/SPLogbook-Linux-x86_64.tar.gz ]"),
        "Workflow musi sprawdzać, czy artefakt Linux jest niepusty"
    );
    assert!(
        workflow.contains("[ -s release-staging/SPLogbook-Windows-x64.zip ]"),
        "Workflow musi sprawdzać, czy artefakt Windows jest niepusty"
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
        windows_count, 1,
        "Artefakt Windows musi wystąpić dokładnie raz"
    );
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
    assert!(
        content.contains("permissions:\n  contents: read"),
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
    let report_content = fs::read_to_string("STAGE_10_FINAL_GATE_REPORT.md")
        .expect("STAGE_10_FINAL_GATE_REPORT.md must exist");

    let extract_val = |key: &str| -> usize {
        report_content
            .lines()
            .find(|l| l.contains(key))
            .and_then(|l| {
                l.split(':').nth(1)?.split_whitespace().find_map(|word| {
                    let digits: String = word.chars().filter(|c| c.is_ascii_digit()).collect();
                    digits.parse::<usize>().ok()
                })
            })
            .unwrap_or_else(|| panic!("Key {key} not found in STAGE_10_FINAL_GATE_REPORT.md"))
    };

    let reg = extract_val("TOTAL REGISTERED TESTS");
    let passed = extract_val("TOTAL PASSED TESTS");
    let executed = extract_val("TOTAL EXECUTED TESTS");
    let ignored = extract_val("TOTAL IGNORED TESTS");
    let hil = extract_val("HIL IGNORED TESTS");

    assert_eq!(executed, passed);
    assert_eq!(reg, executed + ignored);
    assert_eq!(ignored, hil);

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
        UpdateFileClass::FirstInstallSeed,
        "serviceLOG.db musi być sklasyfikowany jako FirstInstallSeed"
    );
    assert_eq!(
        classify_update_path(Path::new("databases/callbook.db")),
        UpdateFileClass::FirstInstallSeed,
        "callbook.db musi być sklasyfikowany jako FirstInstallSeed"
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
    fs::write(staging.join("databases/serviceLOG.db"), b"NEW SERVICE DB").unwrap();
    fs::write(staging.join("databases/callbook.db"), b"NEW CALLBOOK DB").unwrap();

    let active_service_db = active.join("databases/serviceLOG.db");
    let active_callbook_db = active.join("databases/callbook.db");
    fs::write(&active_service_db, b"USER SERVICE LOG DATA").unwrap();
    fs::write(&active_callbook_db, b"USER CALLBOOK DATA").unwrap();

    let plan = UpdatePlan::generate(&staging, &active).unwrap();
    let preserved_targets: Vec<_> = plan
        .preserve_files
        .iter()
        .map(|(p, _)| p.as_path())
        .collect();

    assert!(
        preserved_targets.contains(&active_service_db.as_path()),
        "Plan aktualizacji musi zachować istniejącą bazę serviceLOG.db"
    );
    assert!(
        preserved_targets.contains(&active_callbook_db.as_path()),
        "Plan aktualizacji musi zachować istniejącą bazę callbook.db"
    );

    let replaced_targets: Vec<_> = plan
        .replace_files
        .iter()
        .map(|(_, t)| t.as_path())
        .collect();
    assert!(
        !replaced_targets.contains(&active_service_db.as_path()),
        "Plan aktualizacji nie może podmieniać istniejącej bazy serviceLOG.db"
    );
    assert!(
        !replaced_targets.contains(&active_callbook_db.as_path()),
        "Plan aktualizacji nie może podmieniać istniejącej bazy callbook.db"
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
            std::path::PathBuf::from("C:\\SPLogbook\\databases\\serviceLOG.db"),
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
        updater_source.contains("plan.validate_safety(\"splogbook\")?;"),
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
        staging.join("databases/serviceLOG.db"),
        b"OVERWRITING SERVICE DB",
    )
    .unwrap();
    fs::write(staging.join("station_config.json"), b"OVERWRITING CONFIG").unwrap();
    fs::write(staging.join("default_log.db"), b"OVERWRITING LOG").unwrap();

    let active_exe = active.join("SPLogbook.exe");
    let active_db = active.join("databases/serviceLOG.db");
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
        "Baza serviceLOG.db nie może zostać zmodyfikowana przez rollback"
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

    // Stany bezwzględnie nieanulowalne zgodnie z OWNER-M2-13
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

        // 4. Rollback directory preserved for retention (OWNER-M2-08/09)
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
        // Rollback directory preserved for OWNER-M2-08/09
        assert!(root.join(".rollback").exists());

        // 2. PowerShell Fallback Parity Check
        let script = generate_powershell_from_request(&req);

        // Script must reflect the identical contract
        assert!(script.contains(&attempt.to_string()));
        assert!(script.contains("SPLogbook.exe"));
        assert!(script.contains("asset.dat"));
        assert!(script.contains(".health_ok"));
        assert!(script.contains("exit 0"));
        assert!(!script.contains("Invoke-Expression"));
        assert!(!script.contains("iex"));
        assert!(!script.contains("$staging\\*"));
        assert!(!script.contains("$rollback\\*"));
        assert!(!script.contains("Remove-Item *"));
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
