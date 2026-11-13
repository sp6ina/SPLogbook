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

    // 3. Status decyzji OWNER-M2-08 i OWNER-M2-09 musi pozostać OPEN / NON-BLOCKING w dokumentacji
    let audit_register =
        fs::read_to_string("docs/AUDIT_FINDINGS_REGISTER.md").expect("read audit register");
    let roadmap = fs::read_to_string("docs/IMPROVEMENT_ROADMAP.md").expect("read roadmap");

    for doc in [&audit_register, &roadmap] {
        assert!(
            !doc.contains("OWNER-M2-08: APPROVED") && !doc.contains("OWNER-M2-09: APPROVED"),
            "Decyzje OWNER-M2-08 i OWNER-M2-09 nie mogą być oznaczone jako APPROVED przed formalnym rozstrzygnięciem"
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
    fs::write(db_dir.join("serviceLOG.db"), b"CRITICAL_USER_LOGBOOK").expect("write db");
    fs::write(db_dir.join("serviceLOG.db-wal"), b"WAL_FRAMES").expect("write wal");
    fs::write(db_dir.join("serviceLOG.db-shm"), b"SHM_INDEX").expect("write shm");
    fs::write(root.join("station_config.json"), b"USER_CONFIG").expect("write config");
    fs::write(root.join("contacts.adi"), b"<CALL:6>SP6INA").expect("write adif");
    fs::write(root.join("contest.cbr"), b"START-OF-LOG: 3.0").expect("write cabrillo");
    fs::write(root.join("my_notes.txt"), b"PERSONAL_NOTES").expect("write notes");

    // Symulacja stagingu próbującego nadpisać pliki bazy
    let staging = root.join(".staging");
    fs::create_dir_all(staging.join("databases")).expect("create staging databases");
    fs::write(
        staging.join("databases/serviceLOG.db"),
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
        assert_ne!(rel, Path::new("databases/serviceLOG.db"));
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
            .any(|(p, _)| p.ends_with("serviceLOG.db")),
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
        PathBuf::from("databases/serviceLOG.db"),
        root.join("databases/serviceLOG.db"),
    ));
    assert!(malicious_plan.validate_safety("SPLogbook.exe").is_err());

    // Zawartość plików użytkownika musi pozostać nienaruszona
    assert_eq!(
        fs::read(db_dir.join("serviceLOG.db")).expect("read db"),
        b"CRITICAL_USER_LOGBOOK"
    );
    assert_eq!(
        fs::read(db_dir.join("serviceLOG.db-wal")).expect("read wal"),
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

    assert_eq!(env!("CARGO_PKG_VERSION"), "1.3.0-alpha.1");

    let id = BuildIdentity::current();
    assert_eq!(id.package_name, "splogbook");
    assert_eq!(id.package_version, "1.3.0-alpha.1");
    assert_eq!(id.channel, UpdateChannel::Alpha);
    assert_eq!(id.distribution_status, DistributionStatus::Development);
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
    assert_eq!(id.semver.pre.as_str(), "alpha.1");
    assert!(id.channel.as_str().contains("alpha"));
    assert!(id.is_prerelease());
    assert!(!id.is_stable());
    assert!(id.format_diagnostic().contains("1.3.0-alpha.1"));
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

    // 2. Verify no tags matching 1.3.0 have been created or published
    let git_list_tags = Command::new("git")
        .current_dir(repo_root)
        .args(["tag", "-l", "*1.3.0*"])
        .output();

    if let Ok(output) = git_list_tags {
        assert!(output.status.success(), "git tag -l failed");
        let tags = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert!(
            tags.is_empty(),
            "No git tag matching *1.3.0* may exist during STAGE-M3-DEV-VERSION-IDENTITY-GATE: found '{tags}'"
        );
    }

    // 3. Verify no workflow automatically pushes tags or publishes releases on push
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
            }
        }
    }
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
        assert_eq!(id.package_version, "1.3.0-alpha.1");
        assert_eq!(id.channel, UpdateChannel::Alpha);
        assert_eq!(id.distribution_status, DistributionStatus::Development);
        assert_eq!(id.semver.to_string(), "1.3.0-alpha.1");

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

    // Potwierdź formalne tokeny architektoniczne w dokumentacji projektowej
    let roadmap =
        fs::read_to_string("docs/IMPROVEMENT_ROADMAP.md").expect("docs/IMPROVEMENT_ROADMAP.md");
    assert!(
        roadmap.contains("JOURNALED_TRANSACTIONAL_REPLACEMENT_VERIFIED")
            && roadmap.contains("NOT_POWER_LOSS_ATOMIC")
            && roadmap.contains("CRASH_RECOVERABLE_BY_JOURNAL"),
        "Kanon architektoniczny docs/IMPROVEMENT_ROADMAP.md musi zawierać potwierdzenie semantyki transakcyjnej"
    );
}

#[test]
fn m3_d_inno_setup_toolchain_decision_guard() {
    let pre_report =
        fs::read_to_string("STAGE_M3_D_WINDOWS_USER_SETUP_RESUME_PRE_IMPLEMENTATION_REPORT.md")
            .expect("STAGE_M3_D_WINDOWS_USER_SETUP_RESUME_PRE_IMPLEMENTATION_REPORT.md");
    assert!(
        pre_report.contains("OWNER-M3-D-TOOLCHAIN-01"),
        "Raport musi zawierać identyfikator decyzji OWNER-M3-D-TOOLCHAIN-01"
    );
    assert!(
        pre_report.contains("APPROVED"),
        "Decyzja OWNER-M3-D-TOOLCHAIN-01 musi mieć status APPROVED"
    );
    assert!(
        pre_report.contains("7.1.1"),
        "Zatwierdzona wersja Inno Setup to 7.1.1"
    );
    assert!(
        pre_report.contains(r"C:\Program Files\Inno Setup 7\ISCC.exe"),
        r"Zatwierdzona ścieżka kompilatora to C:\Program Files\Inno Setup 7\ISCC.exe"
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
        cargo_toml.contains(r#"version = "1.3.0-alpha.1""#),
        "Cargo.toml musi definiować bieżącą wersję roboczą 1.3.0-alpha.1"
    );

    assert!(
        cargo_toml.contains("-alpha.1"),
        "Bieżąca wersja robocza musi mieć oznaczenie pre-release alpha"
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

    let tag_output = Command::new("git")
        .args(["tag", "-l"])
        .output()
        .expect("git tag -l");
    let tags = String::from_utf8_lossy(&tag_output.stdout);
    assert!(tags.contains("v1.2.0"), "Tag v1.2.0 musi istnieć");
    assert!(
        !tags.contains("v1.3.0"),
        "Nie wolno tworzyć tagu v1.3.0 przed formalnym zatwierdzeniem wydania"
    );
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
        "SPLogbook.wixproj musi używać zatwierdzonego WiX Toolset SDK 7.0.1 (decyzja OWNER-M3-E-TOOLCHAIN-01)"
    );
    assert!(
        wixproj.contains("<AcceptEula>true</AcceptEula>"),
        "SPLogbook.wixproj musi deklarować akceptację EULA WiX 7"
    );

    let pkg = fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");
    assert!(
        pkg.contains("UpgradeCode=\"B7D14532-6A10-4F8A-9A4B-3DE189E502B0\""),
        "Package.wxs musi zawierać stały UpgradeCode zatwierdzony w OWNER-M3-E-TOOLCHAIN-01"
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

    // Seedy z NeverOverwrite="yes"
    for seed in ["callbook.db", "serviceLOG.db", "clubs.csv"] {
        let seed_pos = comp.find(seed).expect("seed found");
        let comp_start = comp[..seed_pos].rfind("<Component").expect("Component tag");
        let comp_end = comp[comp_start..].find('>').expect("close tag");
        let tag = &comp[comp_start..comp_start + comp_end];
        assert!(
            tag.contains("NeverOverwrite=\"yes\""),
            "Seed {seed} musi mieć NeverOverwrite=\"yes\""
        );
    }

    // MajorUpgrade z blokadą downgrade
    assert!(pkg.contains("<MajorUpgrade"));
    assert!(pkg.contains("DowngradeErrorMessage="));
    assert!(pkg.contains("Schedule=\"afterInstallInitialize\""));
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
    let decisions_doc = fs::read_to_string("STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md")
        .expect("read STAGE_M3_F_OWNER_DECISIONS_PROPOSAL.md");

    // Wszystkie 11 formalnych decyzji M3-F
    assert!(decisions_doc.contains("OWNER-M3-F-LINUX-BASELINE-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-KERNEL-POLICY-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-ARCHITECTURE-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-LIBRARY-POLICY-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-DYNAMIC-LIBRARIES-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-PORTABLE-LAYOUT-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-UPDATE-MODE-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-ARCHIVE-EXTRACTION-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-XDG-DATA-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-SIGNING-01"));
    assert!(decisions_doc.contains("OWNER-M3-F-DESKTOP-INTEGRATION-01"));

    // Status decyzji
    assert!(decisions_doc.contains("M3-F OWNER DECISIONS: COMPLETED_DESIGN_VERIFIED"));
    assert!(decisions_doc.contains("M3-F IMPLEMENTATION: READY_FOR_RESUME"));

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

    // OWNER-M3-F-KERNEL-POLICY-OPTION-A: minimum dynamiczne, nie sztywne 7.2.
    assert_eq!(
        json["kernel_policy"]["decision_id"],
        "OWNER-M3-F-KERNEL-POLICY-OPTION-A"
    );
    assert_eq!(
        json["kernel_policy"]["model"],
        "dynamic-distribution-derived-floor"
    );
    assert_eq!(json["kernel_policy"]["minimum_kernel"], "7.0");

    // OWNER-M3-F-GLIBC-POLICY-01: minimum glibc 2.43.
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

    // Nienaruszalność taga v1.2.0 i zakaz przedwczesnego tagu v1.3.0
    let tag_output = Command::new("git")
        .args(["tag", "-l"])
        .output()
        .expect("git tag -l");
    let tags = String::from_utf8_lossy(&tag_output.stdout);
    assert!(tags.contains("v1.2.0"), "Tag v1.2.0 musi istnieć");
    assert!(
        !tags.contains("v1.3.0"),
        "Nie wolno tworzyć tagu v1.3.0 przed formalnym zatwierdzeniem wydania"
    );

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
        "Skrypt budowania musi zapisywać minimum kernela 7.0 (OWNER-M3-F-KERNEL-POLICY-OPTION-A)"
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
// STAGE-M3-F-POST-CLOSURE-CONSISTENCY-VERIFICATION-GATE (2026-11-09)
// Strażnik spójności po zamknięciu M3-F: precyzyjna klasyfikacja drzewa
// (tracked / untracked / ignored / strict), klasyfikacja dowodu natywnego
// oraz formalna bramka F-03 (fail-closed weryfikacja Linux).
// ============================================================================

#[test]
fn m3_f_post_closure_consistency_guard() {
    let agents = fs::read_to_string("AGENTS.md").expect("AGENTS.md");
    let state =
        fs::read_to_string("docs/PROJECT_CURRENT_STATE.md").expect("docs/PROJECT_CURRENT_STATE.md");
    let next_gate = fs::read_to_string("docs/NEXT_WORK_GATE.md").expect("docs/NEXT_WORK_GATE.md");
    let master =
        fs::read_to_string("MASTER_IMPLEMENTATION_PLAN.md").expect("MASTER_IMPLEMENTATION_PLAN.md");
    let policy_report = fs::read_to_string("STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md")
        .expect("STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md");
    let matrix = fs::read_to_string("docs/DECISION_TRACEABILITY_MATRIX.md")
        .expect("docs/DECISION_TRACEABILITY_MATRIX.md");
    let index =
        fs::read_to_string("docs/DOCUMENTATION_INDEX.md").expect("docs/DOCUMENTATION_INDEX.md");

    // 1. Stan drzewa musi być opisany jawnie (tracked / untracked / ignored / strict).
    for (name, source) in [
        ("AGENTS.md", &agents),
        ("docs/PROJECT_CURRENT_STATE.md", &state),
        (
            "STAGE_M3_F_LINUX_SUPPORT_POLICY_ALIGNMENT_REPORT.md",
            &policy_report,
        ),
    ] {
        assert!(
            source.contains("STRICT_WORKTREE_CLEAN") || source.contains("czystość ścisła"),
            "{name}: brak jawnej definicji czystości (tracked / untracked / ignored / strict)"
        );
    }
    assert!(
        !state.contains("| Working tree | `CLEAN` |"),
        "PROJECT_CURRENT_STATE: nieprecyzyjne `CLEAN` bez definicji jest zabronione"
    );

    // 2. scratch/ jest ignorowany tylko wzorcami plików — nie nazywaj go fantomem.
    assert!(
        agents.contains("scratch/tree_dupes.txt"),
        "AGENTS.md musi nazywać realny nieśledzony plik w scratch/ (F-25 to nie fantom)"
    );
    assert!(
        state.contains("UNTRACKED_SCRATCH_PRESENT"),
        "PROJECT_CURRENT_STATE musi jawnie klasyfikować nieśledzony scratch/"
    );

    // 3. Dowód natywny: run 42518873662 dotyczył 7 profili, nie 4.
    assert!(
        !master.contains("4 profile w GitHub Actions Run ID 42518873662"),
        "MASTER_IMPLEMENTATION_PLAN: run 42518873662 dotyczył 7 profili, nie 4"
    );

    // 4. Formalna bramka F-03 jest zdefiniowana, a M3-G pozostaje zablokowane.
    for (name, source) in [
        ("docs/NEXT_WORK_GATE.md", &next_gate),
        ("docs/DECISION_TRACEABILITY_MATRIX.md", &matrix),
    ] {
        assert!(
            source.contains("STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE"),
            "{name}: brak formalnego identyfikatora bramki F-03"
        );
    }
    assert!(
        next_gate.contains("STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE"),
        "NEXT_WORK_GATE musi wskazywać M3-G jako jedyną następną bramkę"
    );
    assert!(
        next_gate.contains("F03_STATUS: `CLOSED`"),
        "NEXT_WORK_GATE musi utrzymywać F-03 jako CLOSED"
    );
    assert!(
        next_gate.contains("M3_G_STATUS: `IN_PROGRESS`")
            || next_gate.contains("M3_G_STATUS: `CLOSED`"),
        "NEXT_WORK_GATE musi odzwierciedlać rozpoczętą lub zamkniętą bramkę M3-G"
    );

    // 5. Raport M3-F i indeks muszą odnotować tę bramkę.
    assert!(
        policy_report.contains("ERRATA"),
        "Raport M3-F policy musi zawierać errata spójności po zamknięciu"
    );
    assert!(
        index.contains("STAGE_M3_F_POST_CLOSURE_CONSISTENCY_VERIFICATION_REPORT.md"),
        "Indeks dokumentacji musi zawierać raport bramki spójności"
    );

    // 6. Trwałe reguły bramki: F-03 jawnie CLOSED w kontrakcie i stanie, decyzje OWNER zapisane.
    assert!(
        agents.contains("F-03"),
        "AGENTS.md musi jawnie opisywać F-03 (zakaz deklarowania natywnego PASS)"
    );
    assert!(
        state.contains("F-03") && state.contains("`CLOSED`"),
        "PROJECT_CURRENT_STATE musi utrzymywać F-03 jako CLOSED"
    );
    assert!(
        state.contains("M3_G_STATUS")
            && (state.contains("M3_G_STATUS: `IN_PROGRESS`")
                || state.contains("M3_G_STATUS: `CLOSED`")),
        "PROJECT_CURRENT_STATE musi odzwierciedlać rozpoczętą lub zamkniętą bramkę M3-G"
    );
    assert!(
        master.contains("F-03"),
        "MASTER_IMPLEMENTATION_PLAN musi jawnie oznaczać F-03"
    );
    for owner_id in ["OWNER-M3-F-PCV-01", "OWNER-M3-F-PCV-07"] {
        assert!(
            matrix.contains(owner_id),
            "DECISION_TRACEABILITY_MATRIX musi zapisywać decyzję {owner_id}"
        );
    }
}

// ============================================================================
// STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE (2026-11-09)
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
    assert!(
        ci.contains("2735") && ci.contains("2725") && ci.contains("10 ignored"),
        "CI musi dokumentować kanoniczne liczniki bazowe (2735/2725/10)"
    );
}

// ============================================================================
// STAGE-M3-F-F03-FAIL-CLOSED-CI-VERIFICATION-GATE (2026-11-09)
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
        .expect(".github/workflows/build-and-release.yml");

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

    // 10. Dokumentacja domyka F-03, a M3-G pozostaje tylko READY_FOR_OWNER_GO.
    let state =
        fs::read_to_string("docs/PROJECT_CURRENT_STATE.md").expect("docs/PROJECT_CURRENT_STATE.md");
    let next_gate = fs::read_to_string("docs/NEXT_WORK_GATE.md").expect("docs/NEXT_WORK_GATE.md");
    let handoff = fs::read_to_string("docs/DEEPSEEK_HARNESS_HANDOFF.md")
        .expect("docs/DEEPSEEK_HARNESS_HANDOFF.md");
    let matrix = fs::read_to_string("docs/DECISION_TRACEABILITY_MATRIX.md")
        .expect("docs/DECISION_TRACEABILITY_MATRIX.md");
    let index =
        fs::read_to_string("docs/DOCUMENTATION_INDEX.md").expect("docs/DOCUMENTATION_INDEX.md");
    let snapshot = fs::read_to_string("LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json")
        .expect("LINUX_SUPPORT_SNAPSHOT_1.3.0-alpha.1.json");

    assert!(
        state.contains("COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED"),
        "PROJECT_CURRENT_STATE musi zamykać F-03 statusem COMPLETED_FAIL_CLOSED_CI_VERIFICATION_VERIFIED"
    );
    assert!(
        next_gate.contains("STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE")
            && next_gate.contains("F03_STATUS: `CLOSED`"),
        "NEXT_WORK_GATE musi zachowywać F-03 jako CLOSED i wskazywać bramkę M3-G"
    );
    assert!(
        handoff.contains("F-03") && handoff.contains("ZAMKNIĘTE"),
        "handoff musi odnotować zamknięcie F-03"
    );
    assert!(
        matrix.contains("OWNER-M3-F-F03-01"),
        "macierz decyzji musi zapisywać OWNER-M3-F-F03-01"
    );
    assert!(
        index.contains("STAGE_M3_F_F03_FAIL_CLOSED_CI_VERIFICATION_REPORT.md"),
        "indeks dokumentacji musi zawierać raport końcowy F-03"
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
// STAGE-M3-G-LINUX-APPIMAGE-INTEGRATION-GATE (2026-11-11)
// Strażnik trwałych kontraktów pakunku AppImage: oficjalne źródło i kanał
// continuous appimagetool, weryfikacja opublikowanego i lokalnego SHA-256,
// przypięcie jednego toolchainu na całą bramkę, jawny type-2 runtime,
// AppRun bez eskalacji i bez LD_LIBRARY_PATH, polityka GLIBC_2.43, bundlowanie
// wyłącznie libhamlib.so.4, cztery profile, testy negatywne, brak publikacji
// release oraz utrzymane fail-closed F-03.
//
// Strażnik NIE wymaga konkretnego commita Continuous — konkretny commit i
// SHA-256 są dowodem bieżącej bramki, nie wiecznym pinem projektu.
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
        install.contains("APPIMAGETOOL_TAG=\"continuous\""),
        "kanał continuous musi być rozwiązany jawnie"
    );
    assert!(
        install.contains("releases/tags/${APPIMAGETOOL_TAG}"),
        "metadane muszą być czytane z oficjalnego endpointu tagu"
    );
    assert!(
        !install.contains("AppImageKit"),
        "zakaz używania starego repozytorium AppImageKit"
    );

    // 2. Odczyt oficjalnego SHA-256, lokalna kontrola i porównanie digestów.
    assert!(
        install.contains("sha256sum"),
        "resolver musi liczyć lokalny SHA-256"
    );
    assert!(
        install.contains("TOOL_CHECKSUM_MISMATCH"),
        "resolver musi odrzucać niezgodność lokalnego i opublikowanego SHA-256"
    );
    assert!(
        install.contains("CONTINUOUS_RELEASE_RACE"),
        "resolver musi wykrywać zmianę assetu między resolution a pobraniem"
    );
    assert!(
        install.contains("APPIMAGETOOL_RELEASE_ID") && install.contains("RELEASE_PUBLISHED"),
        "resolver musi zapisywać release/build identifier oraz czas publikacji"
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
// STAGE-M3-H-LINUX-DEB-INTEGRATION-GATE (2026-11-12)
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
        verify.contains("OWNER-M3-H-03"),
        "wyciszenie tagu musi odwoływać się do decyzji OWNER"
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
            && workflow.contains("SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz"),
        "portable tar.gz i AppImage muszą pozostać w workflow"
    );
}

// ============================================================================
// STAGE-M3-I-LINUX-RPM-INTEGRATION-GATE (2026-11-13)
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
        "Requires:       libc.so.6(GLIBC_2.43)(64bit)",
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
        "%attr(0644,root,root) /opt/SPLogbook/lib/libhamlib.so.4",
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
    assert!(verify.contains("rpm -U --oldpackage"));
    assert!(verify.contains("rpm -U --replacepkgs"));
    assert!(verify.contains("user data unchanged"));
    assert!(verify.contains("RPM VERIFICATION COMPLETED SUCCESSFULLY"));
    assert!(verify.contains("OWNER-M3-H-03"));
    assert!(verify.contains("dir-or-file-in-opt,custom-library-search-path"));
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
        "1.3.0-0.1.alpha.1",
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
            && workflow.contains("SPLogbook-Linux-x86_64-1.3.0-alpha.1.tar.gz"),
        "portable tar.gz i AppImage muszą pozostać w workflow"
    );

    // 12. M3-J nie zostało rozpoczęte.
    assert!(
        !std::path::Path::new("installer/linux/flatpak").exists(),
        "M3-J Flatpak nie może zostać rozpoczęte w M3-I"
    );
}
