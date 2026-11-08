// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy zgodności z XDG Base Directory oraz bezwzględnej ochrony danych użytkownika (M3-F)

use sha2::{Digest, Sha256};
use splogbook::cloud::updater::UpdatePlan;
use splogbook::core::xdg::{XdgError, XdgPaths, detect_legacy_portable_user_data};
use std::fs;
use std::path::Path;

fn sha256_of_file(path: &Path) -> String {
    let data = fs::read(path).expect("read file for sha256");
    let hash = Sha256::digest(&data);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn test_xdg_base_directory_resolution_enforces_absolute_paths() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home_user");
    let custom_cfg = temp.path().join("custom_config");
    let custom_data = temp.path().join("custom_data");
    let custom_state = temp.path().join("custom_state");
    let custom_cache = temp.path().join("custom_cache");

    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&custom_cfg).unwrap();
    fs::create_dir_all(&custom_data).unwrap();
    fs::create_dir_all(&custom_state).unwrap();
    fs::create_dir_all(&custom_cache).unwrap();

    let prev_home = std::env::var("HOME").ok();
    let prev_cfg = std::env::var("XDG_CONFIG_HOME").ok();
    let prev_data = std::env::var("XDG_DATA_HOME").ok();
    let prev_state = std::env::var("XDG_STATE_HOME").ok();
    let prev_cache = std::env::var("XDG_CACHE_HOME").ok();

    unsafe {
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CONFIG_HOME", &custom_cfg);
        std::env::set_var("XDG_DATA_HOME", &custom_data);
        std::env::set_var("XDG_STATE_HOME", &custom_state);
        std::env::set_var("XDG_CACHE_HOME", &custom_cache);
    }

    let paths = XdgPaths::resolve().expect("Resolve XDG paths");
    assert_eq!(paths.config_home, custom_cfg.join("splogbook"));
    assert_eq!(paths.data_home, custom_data.join("splogbook"));
    assert_eq!(paths.state_home, custom_state.join("splogbook"));
    assert_eq!(paths.cache_home, custom_cache.join("splogbook"));

    unsafe {
        if let Some(h) = prev_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(c) = prev_cfg {
            std::env::set_var("XDG_CONFIG_HOME", c);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        if let Some(d) = prev_data {
            std::env::set_var("XDG_DATA_HOME", d);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }
        if let Some(s) = prev_state {
            std::env::set_var("XDG_STATE_HOME", s);
        } else {
            std::env::remove_var("XDG_STATE_HOME");
        }
        if let Some(c) = prev_cache {
            std::env::set_var("XDG_CACHE_HOME", c);
        } else {
            std::env::remove_var("XDG_CACHE_HOME");
        }
    }
}

#[test]
fn test_xdg_relative_paths_are_rejected_and_fallback_to_home() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home_fallback");
    fs::create_dir_all(&home).unwrap();

    let prev_home = std::env::var("HOME").ok();
    let prev_cfg = std::env::var("XDG_CONFIG_HOME").ok();
    let prev_data = std::env::var("XDG_DATA_HOME").ok();
    let prev_state = std::env::var("XDG_STATE_HOME").ok();
    let prev_cache = std::env::var("XDG_CACHE_HOME").ok();

    unsafe {
        std::env::set_var("HOME", &home);
        // Ścieżki względne są niebezpieczne i muszą zostać zignorowane
        std::env::set_var("XDG_CONFIG_HOME", "relative/config");
        std::env::set_var("XDG_DATA_HOME", "../data");
        std::env::set_var("XDG_STATE_HOME", "./state");
        std::env::set_var("XDG_CACHE_HOME", "tmp/cache");
    }

    let paths = XdgPaths::resolve().expect("Resolve XDG paths");
    assert_eq!(paths.config_home, home.join(".config").join("splogbook"));
    assert_eq!(
        paths.data_home,
        home.join(".local").join("share").join("splogbook")
    );
    assert_eq!(
        paths.state_home,
        home.join(".local").join("state").join("splogbook")
    );
    assert_eq!(paths.cache_home, home.join(".cache").join("splogbook"));

    unsafe {
        if let Some(h) = prev_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(c) = prev_cfg {
            std::env::set_var("XDG_CONFIG_HOME", c);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        if let Some(d) = prev_data {
            std::env::set_var("XDG_DATA_HOME", d);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }
        if let Some(s) = prev_state {
            std::env::set_var("XDG_STATE_HOME", s);
        } else {
            std::env::remove_var("XDG_STATE_HOME");
        }
        if let Some(c) = prev_cache {
            std::env::set_var("XDG_CACHE_HOME", c);
        } else {
            std::env::remove_var("XDG_CACHE_HOME");
        }
    }
}

#[test]
fn test_xdg_missing_home_fails_closed() {
    let prev_home = std::env::var("HOME").ok();
    unsafe {
        std::env::remove_var("HOME");
    }

    let res = XdgPaths::resolve();
    assert!(matches!(res, Err(XdgError::MissingHome)));

    unsafe {
        if let Some(h) = prev_home {
            std::env::set_var("HOME", h);
        }
    }
}

#[test]
fn test_xdg_legacy_data_detection_discovers_all_candidates_without_modifying_them() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let db_path = root.join("default_log.db");
    let wal_path = root.join("default_log.db-wal");
    let shm_path = root.join("default_log.db-shm");
    let cfg_path = root.join("station_config.json");
    let adi_path = root.join("sp_contest_2026.adi");
    let cbr_path = root.join("cqww_2026.cbr");

    fs::write(&db_path, b"ORIGINAL_SQLITE_HEADER_DATA").unwrap();
    fs::write(&wal_path, b"ORIGINAL_WAL_DATA").unwrap();
    fs::write(&shm_path, b"ORIGINAL_SHM_DATA").unwrap();
    fs::write(&cfg_path, b"ORIGINAL_STATION_CONFIG_JSON").unwrap();
    fs::write(&adi_path, b"ORIGINAL_ADIF_RECORD").unwrap();
    fs::write(&cbr_path, b"ORIGINAL_CABRILLO_RECORD").unwrap();

    let initial_db_hash = sha256_of_file(&db_path);
    let initial_wal_hash = sha256_of_file(&wal_path);
    let initial_shm_hash = sha256_of_file(&shm_path);
    let initial_cfg_hash = sha256_of_file(&cfg_path);
    let initial_adi_hash = sha256_of_file(&adi_path);
    let initial_cbr_hash = sha256_of_file(&cbr_path);

    // Wykrywanie
    let findings = detect_legacy_portable_user_data(root);
    assert_eq!(findings.len(), 6);

    // Gwarancja nienaruszalności: sumy kontrolne przed i po wykrywaniu są w 100% identyczne
    assert_eq!(sha256_of_file(&db_path), initial_db_hash);
    assert_eq!(sha256_of_file(&wal_path), initial_wal_hash);
    assert_eq!(sha256_of_file(&shm_path), initial_shm_hash);
    assert_eq!(sha256_of_file(&cfg_path), initial_cfg_hash);
    assert_eq!(sha256_of_file(&adi_path), initial_adi_hash);
    assert_eq!(sha256_of_file(&cbr_path), initial_cbr_hash);
}

#[test]
fn test_user_data_byte_exactness_preserved_across_portable_update_and_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let active_dir = temp.path().join("SPLogbook");
    let staging_dir = temp.path().join("staging");
    let rollback_dir = temp.path().join("rollback");

    fs::create_dir_all(&active_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();
    fs::create_dir_all(&rollback_dir).unwrap();

    // 1. Inicjalne dane użytkownika i pliki programu
    let db_path = active_dir.join("default_log.db");
    let wal_path = active_dir.join("default_log.db-wal");
    let shm_path = active_dir.join("default_log.db-shm");
    let cfg_path = active_dir.join("station_config.json");
    let bin_path = active_dir.join("SPLogbook");

    fs::write(&db_path, b"CRITICAL_USER_QSO_LOG_DATABASE_V1").unwrap();
    fs::write(&wal_path, b"CRITICAL_USER_WAL_LOG_V1").unwrap();
    fs::write(&shm_path, b"CRITICAL_USER_SHM_LOG_V1").unwrap();
    fs::write(&cfg_path, b"CRITICAL_USER_STATION_CONFIG_V1").unwrap();
    fs::write(&bin_path, b"OLD_PROGRAM_BINARY_V1").unwrap();

    let initial_db_sha = sha256_of_file(&db_path);
    let initial_wal_sha = sha256_of_file(&wal_path);
    let initial_shm_sha = sha256_of_file(&shm_path);
    let initial_cfg_sha = sha256_of_file(&cfg_path);

    // 2. Nowa paczka aktualizacyjna w stagingu próbuje zaktualizować program
    // oraz podstępnie zawiera pliki bazy i konfiguracji
    fs::write(staging_dir.join("SPLogbook"), b"NEW_PROGRAM_BINARY_V2").unwrap();
    fs::write(staging_dir.join("splogbook-updater"), b"NEW_UPDATER_V2").unwrap();
    fs::write(
        staging_dir.join("default_log.db"),
        b"MALICIOUS_DB_OVERWRITE",
    )
    .unwrap();
    fs::write(
        staging_dir.join("station_config.json"),
        b"MALICIOUS_CFG_OVERWRITE",
    )
    .unwrap();

    // 3. Generowanie planu
    let plan = UpdatePlan::generate(&staging_dir, &active_dir).expect("UpdatePlan");
    plan.validate_safety("SPLogbook").expect("Validate safety");

    // 4. Kopia zapasowa .rollback (wyłącznie pliki programu)
    for (rel, orig) in &plan.replace_files {
        let dest = rollback_dir.join(rel);
        if let Some(p) = dest.parent() {
            let _ = fs::create_dir_all(p);
        }
        fs::copy(orig, &dest).unwrap();
    }

    // 5. Zastosowanie podmiany
    for (rel, orig) in &plan.replace_files {
        let src = staging_dir.join(rel);
        fs::copy(&src, orig).unwrap();
    }
    for (rel, orig) in &plan.create_files {
        let src = staging_dir.join(rel);
        fs::copy(&src, orig).unwrap();
    }

    // Weryfikacja po podmianie: binaria zaktualizowane, BAZY NIETKNIĘTE (100% zgodności SHA-256)
    assert_eq!(fs::read(&bin_path).unwrap(), b"NEW_PROGRAM_BINARY_V2");
    assert_eq!(sha256_of_file(&db_path), initial_db_sha);
    assert_eq!(sha256_of_file(&wal_path), initial_wal_sha);
    assert_eq!(sha256_of_file(&shm_path), initial_shm_sha);
    assert_eq!(sha256_of_file(&cfg_path), initial_cfg_sha);

    // 6. Rollback
    for (_rel, orig) in &plan.create_files {
        let _ = fs::remove_file(orig);
    }
    for (rel, orig) in &plan.replace_files {
        let bkp = rollback_dir.join(rel);
        fs::copy(&bkp, orig).unwrap();
    }

    // Weryfikacja po rollbacku: binaria przywrócone, BAZY WCIĄŻ W 100% IDENTYCZNE
    assert_eq!(fs::read(&bin_path).unwrap(), b"OLD_PROGRAM_BINARY_V1");
    assert_eq!(sha256_of_file(&db_path), initial_db_sha);
    assert_eq!(sha256_of_file(&wal_path), initial_wal_sha);
    assert_eq!(sha256_of_file(&shm_path), initial_shm_sha);
    assert_eq!(sha256_of_file(&cfg_path), initial_cfg_sha);
}
