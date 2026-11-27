#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(clippy::too_many_lines)]
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use splogbook::core::database::LogDatabase;
use splogbook::core::i18n::{Msg, key, tr};
use splogbook::core::scp::ScpEngine;
use splogbook::core::station::AppConfig;
use splogbook::gui::app::SpLogApp;
use std::sync::{Arc, Mutex};

#[cfg(windows)]
mod win32_instance_mutex {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    const MUTEX_NAMES: [&str; 2] = [
        "SPLogbook_Instance_Mutex_{C8E28541-7B21-4A9B-8F5C-4EF298F613C1}",
        "SPLogbook",
    ];

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *const c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut c_void;
        fn CloseHandle(h_object: *mut c_void) -> i32;
        fn GetLastError() -> u32;
    }

    pub struct InstanceMutexGuard {
        handles: Vec<*mut c_void>,
    }

    impl InstanceMutexGuard {
        pub fn acquire() -> Self {
            let mut handles = Vec::with_capacity(MUTEX_NAMES.len());
            for name in MUTEX_NAMES {
                let wide: Vec<u16> = std::ffi::OsStr::new(name)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };
                if handle.is_null() {
                    let err = unsafe { GetLastError() };
                    log::warn!("CreateMutexW({name}) failed with Win32 error {err}");
                } else {
                    handles.push(handle);
                }
            }
            Self { handles }
        }
    }

    impl Drop for InstanceMutexGuard {
        fn drop(&mut self) {
            for handle in self.handles.drain(..) {
                if !handle.is_null() {
                    unsafe {
                        CloseHandle(handle);
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    // Wczesna obsługa CLI diagnostyki tożsamości wersji (przed Tokio, bazą danych, audio, CAT i GUI)
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!(
                    "SPLogbook {}",
                    splogbook::core::build_info::BuildIdentity::current().display_version()
                );
                std::process::exit(0);
            }
            "--build-info" => {
                println!(
                    "{}",
                    splogbook::core::build_info::BuildIdentity::current().format_diagnostic()
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    // Inicjalizacja loggera diagnostycznego (RUST_LOG lub domyślnie info)
    // Nagłówek jak domyślny w env_logger: `[czas-UTC POZIOM cel] treść`, czas z jiff
    // (funkcja `humantime` w env_logger jest wyłączona, by nie dublować jiff w grafie).
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write as _;
            let level_style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "[{} {level_style}{:<5}{level_style:#} {}] {}",
                jiff::Timestamp::now().strftime("%Y-%m-%dT%H:%M:%SZ"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();
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
            let ts = jiff::ZonedDateTime::now().strftime("%Y%m%d_%H%M%S");
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
            let identity = splogbook::core::build_info::BuildIdentity::current();
            let version = expected_version.unwrap_or_else(|| identity.package_version.to_string());
            let commit = expected_commit.unwrap_or_else(|| identity.git_commit_hash.to_string());
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
    let _instance_mutex_guard = win32_instance_mutex::InstanceMutexGuard::acquire();

    // Konsumpcja wyniku pomocnika aktualizacji Windows po restarcie (jeśli istnieje)
    if let Ok(Some(outcome)) = splogbook::cloud::updater::consume_update_result_on_startup(&exe_dir)
    {
        if outcome.was_success {
            eprintln!(
                "Aktualizacja zakończona sukcesem (próba {}, zachowany backup {} bajtów)",
                outcome.attempt_id, outcome.backup_retained_bytes
            );
        } else {
            eprintln!(
                "Ostatnia próba aktualizacji nie powiodła się: kategoria {:?}, kod {}",
                outcome.result_category, outcome.exit_code
            );
        }
    }

    #[cfg(windows)]
    let (app_data_dir, app_config_dir): (
        Option<std::path::PathBuf>,
        Option<std::path::PathBuf>,
    ) = {
        let base = std::env::var("APPDATA")
            .ok()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .map(|p| std::path::PathBuf::from(p).join("SPLogbook"));
        (base.clone(), base)
    };

    #[cfg(not(windows))]
    let (app_data_dir, app_config_dir): (
        Option<std::path::PathBuf>,
        Option<std::path::PathBuf>,
    ) = {
        // Wykrywanie danych legacy w katalogu portable (informacja dla użytkownika, bez cichej migracji)
        let legacy = splogbook::core::xdg::detect_legacy_portable_user_data(&exe_dir);
        if !legacy.is_empty() {
            eprintln!(
                "INFO [M3-F]: Wykryto {} plików danych użytkownika w katalogu portable ({}). Dane są chronione i nie podlegają modyfikacji.",
                legacy.len(),
                exe_dir.display()
            );
        }

        match splogbook::core::xdg::XdgPaths::resolve() {
            Ok(xdg) => (Some(xdg.data_home), Some(xdg.config_home)),
            Err(e) => {
                log::warn!("Nie można rozwiązać ścieżek XDG ({e}); używam katalogu roboczego.");
                (None, None)
            }
        }
    };

    // Dziennik główny (SQLite WAL) — kanoniczny katalog danych użytkownika (z fallbackiem
    // do katalogu roboczego wyłącznie gdy nie można rozwiązać app_data_dir).
    let log_db_path = app_data_dir.as_ref().map_or_else(
        || {
            std::path::PathBuf::from(splogbook::core::database::DATABASES_SUBDIR)
                .join(splogbook::core::database::DEFAULT_LOG_DB_FILE)
        },
        |dir| splogbook::core::database::default_log_db_path(dir),
    );

    // Czysty profil: katalog `databases` nie istnieje. Błąd tworzenia nie jest już połykany —
    // `LogDatabase::open` powtórzy próbę i zwróci czytelny komunikat z przyczyną.
    if let Err(e) = splogbook::core::database::ensure_parent_dir(&log_db_path) {
        log::error!(
            "Nie można utworzyć katalogu bazy danych dla {}: {e}",
            log_db_path.display()
        );
        eprintln!(
            "Nie można utworzyć katalogu bazy danych dla {}: {e}",
            log_db_path.display()
        );
    }

    // Dane referencyjne: pliki krajów, reguły regionalne i nadpisania użytkownika
    // z katalogu danych. Brak danych daje pusty, poprawny stan — bez pobierania i bez paniki.
    let reference_context = splogbook::core::reference::loader::ReferenceContext {
        root: splogbook::core::reference::loader::reference_root_for(&log_db_path),
    };
    let (prefix_matcher, reference_status) =
        splogbook::core::reference::loader::build_prefix_matcher(&reference_context.root);
    log::info!("reference data: {reference_status:?}");
    let table_status =
        splogbook::core::reference::loader::reload_user_tables(&reference_context.root);
    log::info!("user reference tables: {table_status:?}");
    splogbook::core::reference::loader::set_context(reference_context);

    let config_file_path = match app_config_dir.as_ref() {
        Some(dir) => {
            let nested = dir
                .join(splogbook::core::database::DATABASES_SUBDIR)
                .join("station_config.json");
            let flat = dir.join("station_config.json");
            if !nested.exists() && flat.exists() {
                flat
            } else {
                nested
            }
        }
        None => std::path::PathBuf::from(splogbook::core::database::DATABASES_SUBDIR)
            .join("station_config.json"),
    };

    // Okna błędów przy starcie mówią językiem zapisanym w konfiguracji.
    let startup_lang = AppConfig::peek_language(&config_file_path);

    let log_db = match LogDatabase::open(&log_db_path) {
        Ok(db) => db,
        Err(e) => {
            let err_msg = Msg::new(key::STARTUP_DB_ERROR_BODY)
                .arg("path", log_db_path.display())
                .arg("err", &e)
                .text(startup_lang);
            eprintln!("{err_msg}");
            let should_continue = rfd::MessageDialog::new()
                .set_title(tr(key::STARTUP_DB_ERROR_TITLE, startup_lang))
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

    // Baza wzorców Super Check Partial (SCP) — najpierw MASTER.SCP z katalogu danych
    // (jeśli pobrany), następnie znaki z lokalnego dziennika.
    let mut scp = ScpEngine::new();
    if let Some(scp_path) = log_db_path.parent().map(|dir| dir.join("MASTER.SCP"))
        && let Ok(file) = std::fs::File::open(&scp_path)
        && file.metadata().is_ok_and(|m| m.len() > 0)
    {
        scp.load_from_reader(std::io::BufReader::new(file));
    }

    // Wzbogać bazę podpowiedzi SCP o znaki znane z lokalnego dziennika
    // (umożliwia lokalne podpowiadanie oraz korektę rozmytych znaków).
    if let Ok(distinct_calls) = log_db.get_distinct_callsigns() {
        for call in distinct_calls {
            scp.insert(&call);
        }
    }

    if let Err(e) = splogbook::core::database::ensure_parent_dir(&config_file_path) {
        log::warn!(
            "Nie można utworzyć katalogu konfiguracji dla {}: {e}",
            config_file_path.display()
        );
    }

    let app_config = match AppConfig::load_from_file(&config_file_path) {
        Ok(config) => config,
        Err(error) => {
            let message = Msg::new(key::STARTUP_CREDENTIALS_BODY)
                .arg("path", config_file_path.display())
                .arg("err", &error)
                .text(startup_lang);
            log::error!("{message}");
            let should_continue = rfd::MessageDialog::new()
                .set_title(tr(key::STARTUP_CREDENTIALS_TITLE, startup_lang))
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
        .with_min_inner_size([960.0, 600.0])
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
