#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(clippy::too_many_lines)]
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use splogbook::core::database::LogDatabase;
use splogbook::core::prefix::PrefixMatcher;
use splogbook::core::scp::ScpEngine;
use splogbook::core::station::AppConfig;
use splogbook::gui::app::SpLogApp;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), eframe::Error> {
    // Inicjalizacja loggera diagnostycznego (RUST_LOG lub domyślnie info)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("SPLogbook v{} inicjalizacja...", env!("CARGO_PKG_VERSION"));

    // Panic hook - log awarii do pliku z sygnaturą czasową, aby kolejne awarie
    // nie nadpisywały historii poprzednich zgłoszeń.
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Wystąpił nieoczekiwany błąd w SPLogbook:\n\n{info}");
        eprintln!("{msg}");
        log::error!("{msg}");
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let path = dir.join(format!("crash_{ts}.log"));
            let _ = std::fs::write(&path, &msg);
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
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    // Zdrowie po aktualizacji (health check dla updatera) - bez otwierania produkcyjnych baz danych
    if std::env::args().any(|arg| arg == "--check-health-startup") {
        let mut attempt_id: Option<splogbook::cloud::updater::UpdateAttemptId> = None;
        let mut proof_path: Option<std::path::PathBuf> = None;
        let mut expected_commit: Option<String> = None;
        let mut expected_version: Option<String> = None;

        let mut iter = std::env::args();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--update-attempt-id" => {
                    if let Some(val) = iter.next() {
                        attempt_id =
                            splogbook::cloud::updater::UpdateAttemptId::parse_str(&val).ok();
                    }
                }
                "--health-proof-path" => {
                    if let Some(val) = iter.next() {
                        proof_path = Some(std::path::PathBuf::from(val));
                    }
                }
                "--expected-commit" => {
                    if let Some(val) = iter.next() {
                        expected_commit = Some(val);
                    }
                }
                "--expected-version" => {
                    if let Some(val) = iter.next() {
                        expected_version = Some(val);
                    }
                }
                _ => {}
            }
        }

        let target_proof_path = proof_path
            .unwrap_or_else(|| exe_dir.join(splogbook::cloud::updater::HEALTH_PROOF_FILENAME));

        if let Some(aid) = attempt_id {
            let ctx = splogbook::cloud::updater::InstallationDetectionContext::default();
            let raw_inst =
                splogbook::cloud::updater::detect_installation_with_context(&exe_dir, &ctx)
                    .unwrap_or(splogbook::cloud::updater::InstallationType::UnknownInstallation);
            let inst_type = match raw_inst {
                splogbook::cloud::updater::InstallationType::UnknownInstallation => {
                    if cfg!(windows) {
                        splogbook::cloud::updater::InstallationType::WindowsPortableZip
                    } else {
                        splogbook::cloud::updater::InstallationType::LinuxPortableTarGz
                    }
                }
                other => other,
            };
            let version = expected_version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
            let commit = expected_commit
                .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());
            let platform = if cfg!(windows) {
                "windows-x86_64"
            } else {
                "linux-x86_64"
            };

            let proof = splogbook::cloud::updater::HealthProof::new(
                aid, version, commit, platform, inst_type,
            );

            if let Ok(json) = serde_json::to_string_pretty(&proof) {
                let _ = std::fs::write(&target_proof_path, json);
            }
        } else {
            let _ = std::fs::write(&target_proof_path, "OK");
        }

        std::process::exit(0);
    }

    #[cfg(windows)]
    let (app_data_dir, app_config_dir) = {
        let base = std::env::var("APPDATA").map_or_else(
            |_| exe_dir.clone(),
            |p| std::path::PathBuf::from(p).join("SPLogbook"),
        );
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

    // Kolejność ma znaczenie: najpierw ścieżki absolutne (obok pliku wykonywalnego
    // i w katalogu danych aplikacji), a względna ścieżka w bieżącym katalogu
    // roboczym dopiero na końcu — zapobiega to wczytaniu obcej/nieaktualnej bazy,
    // gdyby program uruchomiono z katalogu zawierającego własny podkatalog `databases`.
    let service_db_candidates = [
        exe_dir.join("databases/serviceLOG.db"),
        app_data_dir.join("databases/serviceLOG.db"),
        std::path::PathBuf::from("/usr/share/splogbook/databases/serviceLOG.db"),
        std::path::PathBuf::from("/usr/local/share/splogbook/databases/serviceLOG.db"),
        exe_dir.join("../databases/serviceLOG.db"),
        exe_dir.join("../../databases/serviceLOG.db"),
        std::path::PathBuf::from("databases/serviceLOG.db"),
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
        exe_dir.join("databases/default_log.db"),
        app_data_dir.join("databases/default_log.db"),
        exe_dir.join("../databases/default_log.db"),
        exe_dir.join("../../databases/default_log.db"),
        std::path::PathBuf::from("databases/default_log.db"),
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

    let log_db = match LogDatabase::open(&log_db_path) {
        Ok(db) => db,
        Err(e) => {
            let err_msg = format!(
                "Nie udało się otworzyć bazy danych dziennika:\n{}\n\nBłąd: {}\n\nJeśli będziesz kontynuować, wszystkie zalogowane łączności zostaną utracone po zamknięciu programu (baza tymczasowa w pamięci)!",
                log_db_path.display(),
                e
            );
            eprintln!("{err_msg}");
            let should_continue = rfd::MessageDialog::new()
                .set_title("SPLogbook — krytyczny błąd bazy danych")
                .set_description(&err_msg)
                .set_level(rfd::MessageLevel::Error)
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if !matches!(should_continue, rfd::MessageDialogResult::Yes) {
                std::process::exit(1);
            }
            LogDatabase::open_in_memory()
                .expect("Nie udało się utworzyć nawet tymczasowej bazy danych w pamięci")
        }
    };

    // Wzbogać bazę podpowiedzi SCP o znaki znane z lokalnego dziennika
    // (umożliwia lokalne podpowiadanie oraz korektę rozmytych znaków).
    if let Ok(distinct_calls) = log_db.get_distinct_callsigns(50_000) {
        for call in distinct_calls {
            scp.insert(&call);
        }
    }

    let config_candidates = [
        exe_dir.join("databases/station_config.json"),
        app_config_dir.join("databases/station_config.json"),
        app_config_dir.join("station_config.json"),
        exe_dir.join("../databases/station_config.json"),
        std::path::PathBuf::from("databases/station_config.json"),
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

    let app_config = match AppConfig::load_from_file(&config_file_path) {
        Ok(config) => config,
        Err(error) => {
            let message = format!(
                "Nie można odczytać poświadczeń dla konfiguracji {}:\n{}\n\nCzy chcesz uruchomić program bez zapisanych haseł (np. tryb Portable na nowym komputerze)?\nOryginalny plik nie zostanie nadpisany do czasu ręcznego zapisu ustawień.",
                config_file_path.display(),
                error
            );
            log::error!("{message}");
            let should_continue = rfd::MessageDialog::new()
                .set_title("SPLogbook — problem z magazynem poświadczeń")
                .set_description(&message)
                .set_level(rfd::MessageLevel::Warning)
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if matches!(should_continue, rfd::MessageDialogResult::Yes) {
                AppConfig::load_without_secrets(&config_file_path).unwrap_or_default()
            } else {
                std::process::exit(1);
            }
        }
    };

    let log_db_arc = Arc::new(Mutex::new(log_db));
    let prefix_matcher_arc = Arc::new(prefix_matcher);
    let scp_arc = Arc::new(Mutex::new(scp));

    // Ikona aplikacji
    let icon_data = eframe::egui::IconData {
        rgba: include_bytes!("../assets/icon.rgba").to_vec(),
        width: 256,
        height: 256,
    };

    // Przywracanie geometrii okna głównego (pozycja na dowolnym monitorze,
    // rozmiar oraz stan maksymalizacji) zapisanej przy poprzednim zamknięciu.
    let mut viewport_builder = eframe::egui::ViewportBuilder::default()
        .with_title("SPLogbook")
        .with_inner_size(app_config.main_window_size.unwrap_or([1400.0, 900.0]))
        .with_min_inner_size([1024.0, 700.0])
        .with_icon(std::sync::Arc::new(icon_data))
        .with_active(true);
    if let Some([x, y]) = app_config.main_window_pos {
        viewport_builder = viewport_builder.with_position(eframe::egui::pos2(x, y));
    }
    if app_config.main_window_maximized {
        viewport_builder = viewport_builder.with_maximized(true);
    }

    let options = eframe::NativeOptions {
        viewport: viewport_builder,
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
