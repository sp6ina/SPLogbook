#![cfg_attr(windows, windows_subsystem = "windows")]
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use splogbook::core::database::LogDatabase;
use splogbook::core::prefix::PrefixMatcher;
use splogbook::core::scp::ScpEngine;
use splogbook::core::station::AppConfig;
use splogbook::gui::app::SpLogApp;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), eframe::Error> {
    // Panic hook - log awarii do crash.log
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Wystąpił nieoczekiwany błąd w SPLogbook:\n\n{}", info);
        eprintln!("{}", msg);
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let _ = std::fs::write(dir.join("crash.log"), &msg);
            }
        }
    }));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Nie udało się zainicjalizować środowiska asynchronicznego Tokio");
    let _tokio_guard = rt.enter();

    // Ścieżki bazy danych i konfiguracji (XDG na Linux, APPDATA na Windows)
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    #[cfg(windows)]
    let (app_data_dir, app_config_dir) = {
        let base = std::env::var("APPDATA")
            .map(|p| std::path::PathBuf::from(p).join("SPLogbook"))
            .unwrap_or_else(|_| exe_dir.clone());
        (base.clone(), base)
    };

    #[cfg(not(windows))]
    let (app_data_dir, app_config_dir) = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let data = std::env::var("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from(&home).join(".local").join("share"))
            .join("splogbook");
        let config = std::env::var("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from(&home).join(".config"))
            .join("splogbook");
        (data, config)
    };

    let service_db_candidates = [
        std::path::PathBuf::from("databases/serviceLOG.db"),
        exe_dir.join("databases/serviceLOG.db"),
        app_data_dir.join("databases/serviceLOG.db"),
        std::path::PathBuf::from("/usr/share/splogbook/databases/serviceLOG.db"),
        std::path::PathBuf::from("/usr/local/share/splogbook/databases/serviceLOG.db"),
        exe_dir.join("../databases/serviceLOG.db"),
        exe_dir.join("../../databases/serviceLOG.db"),
    ];
    let service_db_path = service_db_candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| exe_dir.join("databases/serviceLOG.db"));

    let prefix_matcher = if service_db_path.exists() {
        match PrefixMatcher::load_from_db(&service_db_path) {
            Ok(pm) => pm,
            Err(e) => {
                eprintln!("Błąd ładowania {}: {}", service_db_path.display(), e);
                PrefixMatcher::empty()
            }
        }
    } else {
        PrefixMatcher::empty()
    };

    // Baza wzorców Super Check Partial (SCP)
    let mut scp = ScpEngine::new();
    scp.insert("SP6INA");
    scp.insert("SP6ZDA");
    scp.insert("SQ6ABC");
    scp.insert("SP6PAZ");
    scp.insert("3Z100POL");
    scp.insert("W1AW");
    scp.insert("DL1ABC");
    scp.insert("K1TTT");
    scp.insert("JA1ZLO");
    scp.insert("VP8G");
    scp.insert("3D2V");
    scp.insert("KH6J");

    // Dziennik główny (SQLite WAL)
    let log_db_candidates = [
        std::path::PathBuf::from("databases/default_log.db"),
        exe_dir.join("databases/default_log.db"),
        app_data_dir.join("databases/default_log.db"),
        exe_dir.join("../databases/default_log.db"),
        exe_dir.join("../../databases/default_log.db"),
    ];
    let log_db_path = log_db_candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            if app_data_dir.exists() || !exe_dir.join("databases").exists() {
                app_data_dir.join("databases/default_log.db")
            } else {
                exe_dir.join("databases/default_log.db")
            }
        });

    if let Some(parent) = log_db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let log_db = LogDatabase::open(&log_db_path).unwrap_or_else(|_| {
        LogDatabase::open_in_memory().unwrap()
    });

    let config_candidates = [
        std::path::PathBuf::from("databases/station_config.json"),
        exe_dir.join("databases/station_config.json"),
        app_config_dir.join("databases/station_config.json"),
        app_config_dir.join("station_config.json"),
        exe_dir.join("../databases/station_config.json"),
    ];
    let config_file_path = config_candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            if app_config_dir.exists() || !exe_dir.join("databases").exists() {
                app_config_dir.join("databases/station_config.json")
            } else {
                exe_dir.join("databases/station_config.json")
            }
        });

    if let Some(parent) = config_file_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let app_config = AppConfig::load_from_file(&config_file_path);

    let log_db_arc = Arc::new(Mutex::new(log_db));
    let prefix_matcher_arc = Arc::new(prefix_matcher);
    let scp_arc = Arc::new(Mutex::new(scp));

    // Ikona aplikacji
    let icon_data = eframe::egui::IconData {
        rgba: include_bytes!("../assets/icon.rgba").to_vec(),
        width: 256,
        height: 256,
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("SPLogbook")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1024.0, 700.0])
            .with_icon(std::sync::Arc::new(icon_data))
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "SPLogbook",
        options,
        Box::new(move |cc| {
            Ok(Box::new(SpLogApp::new(
                cc,
                log_db_arc,
                prefix_matcher_arc,
                scp_arc,
                app_config,
                config_file_path,
                log_db_path,
            )))
        }),
    )
}
