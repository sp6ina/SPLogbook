// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy ochrony danych użytkownika i bezpieczeństwa cyklu życia Inno Setup (M3-D)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_setup_exe_path() -> Option<PathBuf> {
    let p = PathBuf::from("dist/SPLogbook-Windows-x64-Setup-1.3.0-alpha.1.exe");
    if p.exists() { Some(p) } else { None }
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

#[test]
fn inno_native_install_and_uninstall_lifecycle_e2e() {
    let setup_exe = match get_setup_exe_path() {
        Some(p) => p,
        None => return, // Jeśli artefakt nie został jeszcze skompilowany, pomiń test natywny
    };

    let temp_sandbox = tempfile::tempdir().unwrap();
    let sandbox_path = temp_sandbox.path();
    let log_path = sandbox_path.join("setup_test.log");

    // 1. Silent install do sandboxa
    let status = Command::new(&setup_exe)
        .arg("/SILENT")
        .arg("/NORESTART")
        .arg(format!("/DIR={}", sandbox_path.display()))
        .arg(format!("/LOG={}", log_path.display()))
        .status()
        .expect("uruchomienie instalatora");

    assert!(status.success(), "Instalator musi zakończyć się kodem 0");

    // Weryfikacja obecności zainstalowanych plików
    assert!(sandbox_path.join("SPLogbook.exe").exists());
    assert!(sandbox_path.join("splogbook-updater.exe").exists());
    assert!(sandbox_path.join(".splogbook-inno-marker").exists());
    assert!(sandbox_path.join("unins000.exe").exists());
    assert!(sandbox_path.join("databases/callbook.db").exists());
    assert!(sandbox_path.join("databases/serviceLOG.db").exists());
    assert!(sandbox_path.join("databases/clubs.csv").exists());

    // 2. Dodanie chronionych danych użytkownika
    let user_db = sandbox_path.join("default_log.db");
    let user_wal = sandbox_path.join("default_log.db-wal");
    let user_config = sandbox_path.join("station_config.json");
    let unknown_file = sandbox_path.join("user_notes.txt");
    let user_adif = sandbox_path.join("contest.adi");

    let db_bytes = b"SQLITE_DATA_LOG_12345";
    let wal_bytes = b"SQLITE_WAL_BYTES_67890";
    let config_bytes = b"{\"call\":\"SP6INA\"}";
    let note_bytes = b"IMPORTANT NOTES PRESERVED";
    let adif_bytes = b"<call:6>SP6INA<eor>";

    fs::write(&user_db, db_bytes).unwrap();
    fs::write(&user_wal, wal_bytes).unwrap();
    fs::write(&user_config, config_bytes).unwrap();
    fs::write(&unknown_file, note_bytes).unwrap();
    fs::write(&user_adif, adif_bytes).unwrap();

    // 3. Silent uninstall
    let unins_exe = sandbox_path.join("unins000.exe");
    let unins_log = sandbox_path.join("unins.log");
    let unins_status = Command::new(&unins_exe)
        .arg("/SILENT")
        .arg("/NORESTART")
        .arg(format!("/LOG={}", unins_log.display()))
        .status()
        .expect("uruchomienie deinstalatora");

    assert!(
        unins_status.success(),
        "Deinstalator musi zakończyć się kodem 0"
    );

    // 4. Weryfikacja usunięcia plików programu
    assert!(!sandbox_path.join("SPLogbook.exe").exists());
    assert!(!sandbox_path.join("splogbook-updater.exe").exists());
    assert!(!sandbox_path.join(".splogbook-inno-marker").exists());

    // 5. Weryfikacja nienaruszenia danych użytkownika (byte-identical)
    assert_eq!(fs::read(&user_db).unwrap(), db_bytes);
    assert_eq!(fs::read(&user_wal).unwrap(), wal_bytes);
    assert_eq!(fs::read(&user_config).unwrap(), config_bytes);
    assert_eq!(fs::read(&unknown_file).unwrap(), note_bytes);
    assert_eq!(fs::read(&user_adif).unwrap(), adif_bytes);

    // 6. Weryfikacja seedów z uninsneveruninstall
    assert!(sandbox_path.join("databases/callbook.db").exists());
    assert!(sandbox_path.join("databases/serviceLOG.db").exists());
    assert!(sandbox_path.join("databases/clubs.csv").exists());
}
