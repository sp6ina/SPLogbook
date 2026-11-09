// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy ochrony danych użytkownika i bezpieczeństwa cyklu życia Inno Setup (M3-D)
//
// Bramka QA 2026-11-09: natywny E2E jest JAWNYM OPT-IN i nie zależy od
// przypadkowej zawartości ignorowanego katalogu `dist/`. Standardowy baseline
// nie wykonuje żadnego lokalnego instalatora.

use std::collections::HashMap;
use std::fs;
use std::time::Duration;

#[path = "support/native_artifact.rs"]
mod native_artifact;

use native_artifact::ArtifactDecision;

fn env_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn resolve_with<F>(env: F) -> Result<ArtifactDecision, String>
where
    F: Fn(&str) -> Option<String>,
{
    native_artifact::resolve_artifact(
        env,
        native_artifact::INNO_OPT_IN_ENV,
        native_artifact::INNO_PATH_ENV,
        native_artifact::INNO_SHA256_ENV,
        "exe",
        native_artifact::INNO_FILENAME_PREFIX,
        env!("CARGO_PKG_VERSION"),
    )
}

#[test]
fn inno_seed_databases_installed_with_uninsneveruninstall_and_onlyifdoesntexist() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");

    for seed in ["callbook.db", "serviceLOG.db", "clubs.csv"] {
        let line = iss_content
            .lines()
            .find(|l| l.contains(seed) && l.trim().starts_with("Source:"))
            .unwrap_or_else(|| panic!("Nie znaleziono definicji seeda {seed}"));

        assert!(
            line.contains("onlyifdoesntexist"),
            "Seed {seed} musi mieć flagę onlyifdoesntexist"
        );
        assert!(
            line.contains("uninsneveruninstall"),
            "Seed {seed} musi mieć flagę uninsneveruninstall"
        );
    }
}

#[test]
fn inno_no_wildcards_in_uninstall_delete() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    let uninst_section = iss_content
        .split("[UninstallDelete]")
        .nth(1)
        .and_then(|s| s.split('[').next())
        .expect("UninstallDelete section");

    for line in uninst_section.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.is_empty() {
            continue;
        }
        assert!(
            !trimmed.contains('*'),
            "Sekcja [UninstallDelete] nie może zawierać wildcardów (*): {trimmed}"
        );
        assert!(
            !trimmed.contains('?'),
            "Sekcja [UninstallDelete] nie może zawierać wildcardów (?): {trimmed}"
        );
    }
}

#[test]
fn inno_dirifempty_preserves_directory_with_user_files() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    let uninst_section = iss_content
        .split("[UninstallDelete]")
        .nth(1)
        .and_then(|s| s.split('[').next())
        .expect("UninstallDelete section");

    for line in uninst_section.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Type: dirifempty") {
            assert!(
                !trimmed.contains("recursesubdirs"),
                "dirifempty nie może używać recursesubdirs: {trimmed}"
            );
        }
    }
}

#[test]
fn inno_user_databases_and_configs_never_touched_by_uninstaller() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");

    for forbidden in [
        "default_log.db",
        "station_config.json",
        "splogbook_contacts.db",
    ] {
        assert!(
            !iss_content.contains(forbidden),
            "Plik danych użytkownika {forbidden} nie może występować w SPLogbook.iss"
        );
    }
}

#[test]
fn inno_sqlite_wal_shm_journals_preserved_during_uninstall() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    assert!(!iss_content.contains(".db-wal"));
    assert!(!iss_content.contains(".db-shm"));
    assert!(!iss_content.contains(".db-journal"));
}

#[test]
fn inno_adif_cabrillo_exports_preserved_during_uninstall() {
    let iss_content =
        fs::read_to_string("installer/windows/inno/SPLogbook.iss").expect("read SPLogbook.iss");
    assert!(!iss_content.contains(".adi"));
    assert!(!iss_content.contains(".adif"));
    assert!(!iss_content.contains(".cbr"));
    assert!(!iss_content.contains(".cabrillo"));
}

/// Kontrakt opt-in jest fail-closed na każdym etapie walidacji.
#[test]
fn inno_native_e2e_contract_is_fail_closed() {
    let version = env!("CARGO_PKG_VERSION");
    let canonical_name = format!("SPLogbook-Windows-x64-Setup-{version}.exe");
    let temp = tempfile::tempdir().expect("tempdir");

    // 1. Brak opt-in -> test jawnie niewykonany (nie FAIL).
    let none = env_map(&[]);
    assert!(matches!(
        resolve_with(|key| none.get(key).cloned()),
        Ok(ArtifactDecision::NotRequested)
    ));

    // 2. Opt-in z wartością inną niż `1` -> FAIL-CLOSED.
    let wrong_opt_in = env_map(&[(native_artifact::INNO_OPT_IN_ENV, "true")]);
    assert!(resolve_with(|key| wrong_opt_in.get(key).cloned()).is_err());

    // 3. Opt-in bez ścieżki -> FAIL-CLOSED.
    let no_path = env_map(&[(native_artifact::INNO_OPT_IN_ENV, "1")]);
    assert!(resolve_with(|key| no_path.get(key).cloned()).is_err());

    // 4. Ścieżka względna -> FAIL-CLOSED.
    let relative = env_map(&[
        (native_artifact::INNO_OPT_IN_ENV, "1"),
        (native_artifact::INNO_PATH_ENV, "relative/setup.exe"),
    ]);
    assert!(resolve_with(|key| relative.get(key).cloned()).is_err());

    // 5. Plik nie istnieje -> FAIL-CLOSED.
    let missing_path = temp.path().join("missing").join(&canonical_name);
    let missing = env_map(&[
        (native_artifact::INNO_OPT_IN_ENV, "1"),
        (
            native_artifact::INNO_PATH_ENV,
            &missing_path.display().to_string(),
        ),
    ]);
    assert!(resolve_with(|key| missing.get(key).cloned()).is_err());

    // 6. Katalog zamiast pliku -> FAIL-CLOSED.
    let directory = env_map(&[
        (native_artifact::INNO_OPT_IN_ENV, "1"),
        (
            native_artifact::INNO_PATH_ENV,
            &temp.path().display().to_string(),
        ),
    ]);
    assert!(resolve_with(|key| directory.get(key).cloned()).is_err());

    // 7. Złe rozszerzenie -> FAIL-CLOSED.
    let wrong_extension_path = temp.path().join("SPLogbook-Windows-x64-Setup-1.0.0.msi");
    fs::write(&wrong_extension_path, b"not an exe").expect("write wrong extension");
    let wrong_extension = env_map(&[
        (native_artifact::INNO_OPT_IN_ENV, "1"),
        (
            native_artifact::INNO_PATH_ENV,
            &wrong_extension_path.display().to_string(),
        ),
    ]);
    assert!(resolve_with(|key| wrong_extension.get(key).cloned()).is_err());

    // 8. Wersja niezgodna z Cargo.toml -> FAIL-CLOSED.
    let wrong_version_path = temp.path().join("SPLogbook-Windows-x64-Setup-0.0.1.exe");
    fs::write(&wrong_version_path, b"not the canonical artifact").expect("write wrong version");
    let wrong_version = env_map(&[
        (native_artifact::INNO_OPT_IN_ENV, "1"),
        (
            native_artifact::INNO_PATH_ENV,
            &wrong_version_path.display().to_string(),
        ),
    ]);
    assert!(resolve_with(|key| wrong_version.get(key).cloned()).is_err());

    // 9. Poprawny kontrakt -> artefakt zaakceptowany, SHA-256 policzony.
    let canonical_path = temp.path().join(&canonical_name);
    fs::write(&canonical_path, b"hermetic fixture").expect("write canonical artifact");
    let canonical = env_map(&[
        (native_artifact::INNO_OPT_IN_ENV, "1"),
        (
            native_artifact::INNO_PATH_ENV,
            &canonical_path.display().to_string(),
        ),
    ]);
    let requested = match resolve_with(|key| canonical.get(key).cloned()) {
        Ok(ArtifactDecision::Requested(artifact)) => artifact,
        other => panic!("oczekiwano zaakceptowanego artefaktu, otrzymano {other:?}"),
    };
    let digest = native_artifact::verify_sha256(&requested).expect("sha256");
    assert_eq!(digest.len(), 64);
    assert!(
        digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );

    // 10. Niezgodny przypięty SHA-256 -> FAIL-CLOSED.
    let pinned = native_artifact::resolve_artifact(
        |key| match key {
            native_artifact::INNO_OPT_IN_ENV => Some("1".to_string()),
            native_artifact::INNO_PATH_ENV => Some(canonical_path.display().to_string()),
            native_artifact::INNO_SHA256_ENV => Some("0".repeat(64)),
            _ => None,
        },
        native_artifact::INNO_OPT_IN_ENV,
        native_artifact::INNO_PATH_ENV,
        native_artifact::INNO_SHA256_ENV,
        "exe",
        native_artifact::INNO_FILENAME_PREFIX,
        version,
    )
    .expect("pinned artifact resolves");
    match pinned {
        ArtifactDecision::Requested(artifact) => {
            assert!(native_artifact::verify_sha256(&artifact).is_err());
        }
        ArtifactDecision::NotRequested => panic!("oczekiwano żądania artefaktu"),
    }
}

#[test]
fn inno_native_install_and_uninstall_lifecycle_e2e() {
    let artifact = match resolve_with(|key| std::env::var(key).ok()) {
        Ok(ArtifactDecision::NotRequested) => {
            eprintln!(
                "NATIVE INSTALLER E2E NOT REQUESTED: ustaw {}=1 oraz {}=<ścieżka absolutna>, \
                 aby wykonać natywny cykl życia Inno Setup",
                native_artifact::INNO_OPT_IN_ENV,
                native_artifact::INNO_PATH_ENV
            );
            return;
        }
        Ok(ArtifactDecision::Requested(artifact)) => artifact,
        Err(error) => panic!("NATIVE INSTALLER E2E FAIL-CLOSED: {error}"),
    };

    let digest = native_artifact::verify_sha256(&artifact)
        .unwrap_or_else(|error| panic!("NATIVE INSTALLER E2E FAIL-CLOSED: {error}"));
    eprintln!(
        "NATIVE INSTALLER E2E REQUESTED: artifact={} sha256={digest}",
        artifact.path.display()
    );

    let root = tempfile::tempdir().expect("tempdir");
    let root_path = root.path();
    let install_dir = root_path.join("install");
    let log_dir = root_path.join("logs");
    let test_home = root_path.join("home");
    let appdata = test_home.join("AppData").join("Roaming");
    let localappdata = test_home.join("AppData").join("Local");
    let test_temp = root_path.join("temp");
    for directory in [&install_dir, &log_dir, &appdata, &localappdata, &test_temp] {
        fs::create_dir_all(directory).expect("utworzenie katalogu testowego");
    }

    let setup_log = log_dir.join("setup.log");
    let install_arguments = vec![
        "/VERYSILENT".to_string(),
        "/SUPPRESSMSGBOXES".to_string(),
        "/NORESTART".to_string(),
        "/SP-".to_string(),
        format!("/LOG={}", setup_log.display()),
        format!("/DIR={}", install_dir.display()),
    ];
    let environment = vec![
        ("USERPROFILE".to_string(), test_home.display().to_string()),
        ("APPDATA".to_string(), appdata.display().to_string()),
        (
            "LOCALAPPDATA".to_string(),
            localappdata.display().to_string(),
        ),
        ("TEMP".to_string(), test_temp.display().to_string()),
        ("TMP".to_string(), test_temp.display().to_string()),
    ];

    let install_code = native_artifact::run_with_timeout(
        &artifact.path,
        &install_arguments,
        &environment,
        Duration::from_secs(300),
        "Inno Setup install",
    )
    .unwrap_or_else(|error| {
        panic!(
            "NATIVE INSTALLER E2E FAILED: {error}\n{}",
            native_artifact::log_tail(&setup_log, 60)
        )
    });
    assert_eq!(
        install_code,
        0,
        "NATIVE INSTALLER E2E FAILED: exit code {install_code}; log: {}\n{}",
        setup_log.display(),
        native_artifact::log_tail(&setup_log, 80)
    );
    assert!(
        setup_log.is_file(),
        "NATIVE INSTALLER E2E FAILED: brak logu instalatora {}",
        setup_log.display()
    );
    let setup_log_content = fs::read_to_string(&setup_log).expect("read setup log");
    assert!(
        setup_log_content.contains("Log closed"),
        "log instalatora jest niekompletny (brak 'Log closed')"
    );
    assert!(
        !setup_log_content.contains("Fatal exception"),
        "log instalatora zawiera błąd fatalny:\n{}",
        native_artifact::log_tail(&setup_log, 40)
    );

    // 1. Dowód instalacji plików allowlisty.
    for relative in [
        "SPLogbook.exe",
        "splogbook-updater.exe",
        ".splogbook-inno-marker",
        "unins000.exe",
        "databases/callbook.db",
        "databases/serviceLOG.db",
        "databases/clubs.csv",
    ] {
        assert!(
            install_dir.join(relative).exists(),
            "brak zainstalowanego pliku: {relative}"
        );
    }

    // 2. Tożsamość instalacji zapisana w markerze.
    let marker_content =
        fs::read_to_string(install_dir.join(".splogbook-inno-marker")).expect("read inno marker");
    let marker: serde_json::Value =
        serde_json::from_str(&marker_content).expect("marker musi być poprawnym JSON");
    assert_eq!(marker["package_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(marker["installation_type"], "WindowsInnoSetup");
    assert_eq!(marker["management_mode"], "SystemManaged");
    assert_eq!(marker["installation_scope"], "CurrentUser");
    assert_eq!(marker["app_id"], "{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}");
    let build_commit = marker["build_commit"]
        .as_str()
        .expect("build_commit musi być stringiem");
    assert_eq!(build_commit.len(), 40, "build_commit musi być pełnym SHA-1");
    assert!(
        build_commit
            .chars()
            .all(|character| character.is_ascii_hexdigit()),
        "build_commit musi być heksem"
    );

    // 3. Chronione dane użytkownika zapisane w katalogu instalacji.
    let user_db = install_dir.join("default_log.db");
    let user_wal = install_dir.join("default_log.db-wal");
    let user_config = install_dir.join("station_config.json");
    let unknown_file = install_dir.join("user_notes.txt");
    let user_adif = install_dir.join("contest.adi");

    let db_bytes = b"SQLITE_DATA_LOG_12345";
    let wal_bytes = b"SQLITE_WAL_BYTES_67890";
    let config_bytes = b"{\"call\":\"SP6INA\"}";
    let note_bytes = b"IMPORTANT NOTES PRESERVED";
    let adif_bytes = b"<call:6>SP6INA<eor>";

    fs::write(&user_db, db_bytes).expect("write user db");
    fs::write(&user_wal, wal_bytes).expect("write user wal");
    fs::write(&user_config, config_bytes).expect("write user config");
    fs::write(&unknown_file, note_bytes).expect("write unknown file");
    fs::write(&user_adif, adif_bytes).expect("write user adif");

    // 4. Silent uninstall.
    let uninstaller = install_dir.join("unins000.exe");
    let uninstall_log = log_dir.join("uninstall.log");
    let uninstall_arguments = vec![
        "/VERYSILENT".to_string(),
        "/SUPPRESSMSGBOXES".to_string(),
        "/NORESTART".to_string(),
        "/SP-".to_string(),
        format!("/LOG={}", uninstall_log.display()),
    ];
    let uninstall_code = native_artifact::run_with_timeout(
        &uninstaller,
        &uninstall_arguments,
        &environment,
        Duration::from_secs(300),
        "Inno Setup uninstall",
    )
    .unwrap_or_else(|error| {
        panic!(
            "NATIVE INSTALLER E2E FAILED: {error}\n{}",
            native_artifact::log_tail(&uninstall_log, 60)
        )
    });
    assert_eq!(
        uninstall_code,
        0,
        "NATIVE INSTALLER E2E FAILED: uninstaller exit code {uninstall_code}; log: {}\n{}",
        uninstall_log.display(),
        native_artifact::log_tail(&uninstall_log, 80)
    );

    // 5. Program usunięty.
    for relative in [
        "SPLogbook.exe",
        "splogbook-updater.exe",
        ".splogbook-inno-marker",
    ] {
        assert!(
            !install_dir.join(relative).exists(),
            "deinstalator nie usunął pliku programu: {relative}"
        );
    }

    // 6. Dane użytkownika byte-identical.
    assert_eq!(fs::read(&user_db).expect("read user db"), db_bytes);
    assert_eq!(fs::read(&user_wal).expect("read user wal"), wal_bytes);
    assert_eq!(
        fs::read(&user_config).expect("read user config"),
        config_bytes
    );
    assert_eq!(
        fs::read(&unknown_file).expect("read unknown file"),
        note_bytes
    );
    assert_eq!(fs::read(&user_adif).expect("read user adif"), adif_bytes);

    // 7. Seedy FirstInstallSeed pozostają nietknięte.
    for relative in [
        "databases/callbook.db",
        "databases/serviceLOG.db",
        "databases/clubs.csv",
    ] {
        assert!(
            install_dir.join(relative).exists(),
            "deinstalator usunął seed: {relative}"
        );
    }
}
