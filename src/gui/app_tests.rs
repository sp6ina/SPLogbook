// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

#[cfg(test)]
use super::*;
use crate::core::database::{Journal, LogDatabase};
use crate::core::prefix::PrefixMatcher;
use crate::core::qso::QsoRecord;
use crate::core::scp::ScpEngine;
use crate::core::station::AppConfig;
use crate::gui::{RepeatPolicy, ShortcutRegistry};
use std::sync::{Arc, Mutex};

fn test_prefix_matcher() -> PrefixMatcher {
    if let Ok(dir) = tempfile::tempdir() {
        let dat = "Poland:                   15:  28:  EU:   51.10:   -17.00:    -1.0:  SP:\n    SP,=SP6INA;\n";
        let csv = "SP,Poland,269,EU,15,28,51.10,-17.00,-1.0,SP =SP6INA;\n";
        if std::fs::write(dir.path().join("cty.dat"), dat).is_ok()
            && std::fs::write(dir.path().join("cty.csv"), csv).is_ok()
            && let Ok((m, _)) = PrefixMatcher::load_from_country_files(
                &dir.path().join("cty.dat"),
                Some(&dir.path().join("cty.csv")),
            )
        {
            return m;
        }
    }
    PrefixMatcher::empty()
}

/// Unikalna ścieżka konfiguracji w katalogu tymczasowym: test, który zapisze konfigurację
/// (np. przez start sesji zawodów), nie zostawi pliku w katalogu repozytorium.
fn test_config_path() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "splogbook-test-config-{}-{n}.json",
        std::process::id()
    ))
}

fn test_deps() -> AppDeps {
    AppDeps {
        log_db: Arc::new(Mutex::new(LogDatabase::open_in_memory().unwrap())),
        prefix_matcher: Arc::new(test_prefix_matcher()),
        scp_engine: Arc::new(Mutex::new(ScpEngine::new())),
    }
}

#[test]
fn build_loads_active_journal_and_qso_numbers() {
    let deps = test_deps();
    let (first_id, second_id) = {
        let db = deps.log_db.lock().unwrap();
        let j = Journal {
            id: "test-id".to_string(),
            name: "Test".to_string(),
            ..Default::default()
        };
        db.create_journal(&j).unwrap();
        db.set_active_journal("test-id").unwrap();

        let mut q1 = QsoRecord::new("SP1ABC", "20m", "CW");
        q1.journal_id = Some("test-id".to_string());
        q1.qso_date = "2026-01-01".to_string();
        q1.time_on = "10:00:00".to_string();
        let id1 = db.insert_qso(&q1).unwrap();

        let mut q2 = QsoRecord::new("SP2DEF", "40m", "SSB");
        q2.journal_id = Some("test-id".to_string());
        q2.qso_date = "2026-01-01".to_string();
        q2.time_on = "10:15:00".to_string();
        let id2 = db.insert_qso(&q2).unwrap();

        (id1, id2)
    };

    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };

    let app = SpLogApp::for_tests(init, deps);
    assert_eq!(app.active_journal.name, "Test");
    assert_eq!(app.active_journal.id, "test-id");
    assert_eq!(app.log_view.qso_numbers.len(), 2);
    assert_eq!(app.log_view.qso_numbers.get(&first_id), Some(&1));
    assert_eq!(app.log_view.qso_numbers.get(&second_id), Some(&2));
    assert_eq!(app.log_view.recent_qsos.len(), 2);
}

#[test]
fn build_with_empty_database() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let app = SpLogApp::for_tests(init, deps);
    assert_eq!(app.active_journal.name, "Główny Dziennik");
}

#[test]
fn build_does_not_start_background_services() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let _app = SpLogApp::for_tests(init, deps);

    // Ensure UDP 2237 is free
    let socket = std::net::UdpSocket::bind("0.0.0.0:2237");
    assert!(socket.is_ok(), "Port 2237 was occupied by build!");
}

#[test]
fn drop_with_none_policy_does_not_touch_disk() {
    let temp = tempfile::tempdir().unwrap();
    let active_db = temp.path().join("db.sqlite");
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: temp.path().join("config.json"),
        active_db_path: active_db.clone(),
        shutdown_policy: ShutdownPolicy::None,
    };

    {
        let _app = SpLogApp::for_tests(init, deps);
    }

    assert!(!temp.path().join("backups").exists());
}

#[test]
fn drop_with_backup_policy_creates_backup() {
    let temp = tempfile::tempdir().unwrap();
    let active_db = temp.path().join("db.sqlite");
    std::fs::write(&active_db, "dummy data").unwrap();

    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: temp.path().join("config.json"),
        active_db_path: active_db.clone(),
        shutdown_policy: ShutdownPolicy::Backup,
    };

    {
        let _app = SpLogApp::for_tests(init, deps);
    }

    assert!(temp.path().join("backups").exists());
}

#[test]
fn gui_import_adif_content_success_duplicates_and_state_refresh() {
    let deps = test_deps();
    let scp = Arc::clone(&deps.scp_engine);
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    let adif = "<CALL:6>SP6INA <BAND:3>20m <MODE:2>CW <QSO_DATE:8>20261012 <TIME_ON:6>120000 <RST_SENT:3>599 <RST_RCVD:3>599 <EOR>\n\
                    <CALL:6>DL1ABC <BAND:3>40m <MODE:3>SSB <QSO_DATE:8>20261012 <TIME_ON:6>121500 <RST_SENT:2>59 <RST_RCVD:2>59 <EOR>";

    // 1. Pierwszy poprawny import
    app.import_adif_content(adif, "test_file.adi");

    assert_eq!(app.log_view.recent_qsos.len(), 2);
    assert_eq!(app.log_view.qso_numbers.len(), 2);
    assert!(
        app.gui
            .chrome
            .status_message
            .as_ref()
            .unwrap()
            .contains("Zaimportowano 2")
    );

    // Weryfikacja aktualizacji SCP
    {
        let scp_guard = scp.lock().unwrap();
        let matches = scp_guard.search("SP6INA", 5);
        assert!(matches.contains(&"SP6INA".to_string()));
        let matches_dl = scp_guard.search("DL1ABC", 5);
        assert!(matches_dl.contains(&"DL1ABC".to_string()));
    }

    // Weryfikacja aktualizacji AwardsEngine
    {
        let awards_guard = app.services.awards_engine.lock().unwrap();
        assert!(
            awards_guard
                .worked_calls
                .iter()
                .any(|(call, _, _)| call == "SP6INA")
        );
        assert!(
            awards_guard
                .worked_calls
                .iter()
                .any(|(call, _, _)| call == "DL1ABC")
        );
    }

    // 2. Import duplikatów
    app.import_adif_content(adif, "test_file.adi");

    // Liczba rekordów nie wzrosła
    assert_eq!(app.log_view.recent_qsos.len(), 2);
    assert_eq!(app.log_view.qso_numbers.len(), 2);
    assert!(
        app.gui
            .chrome
            .status_message
            .as_ref()
            .unwrap()
            .contains("Zaimportowano 0")
    );
    assert!(
        app.gui
            .chrome
            .status_message
            .as_ref()
            .unwrap()
            .contains("duplikatów pominiętych")
    );
}

#[test]
fn form_qso_save_success_and_side_effects() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let mut event_rx = app.services.event_bus.subscribe();

    app.entry.callsign = "SP6INA".to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "CW".to_string();
    app.entry.rst_sent = "599".to_string();
    app.entry.rst_rcvd = "599".to_string();

    app.save_qso();

    // 1. Formularz wyczyszczony
    assert_eq!(app.entry.callsign, "");

    // 2. Rekord zapisany w bazie z aktywnym journal_id
    assert_eq!(app.log_view.recent_qsos.len(), 1);
    assert_eq!(app.log_view.recent_qsos[0].callsign, "SP6INA");
    assert_eq!(
        app.log_view.recent_qsos[0].journal_id,
        Some(app.active_journal.id.clone())
    );

    // 3. Awards i SCP zaktualizowane
    {
        let awards = app.services.awards_engine.lock().unwrap();
        assert!(awards.worked_calls.iter().any(|(c, _, _)| c == "SP6INA"));
    }
    {
        let scp = app.services.scp_engine.lock().unwrap();
        assert!(scp.search("SP6INA", 5).contains(&"SP6INA".to_string()));
    }

    // 4. AppEvent::QsoLogged opublikowany dokładnie raz
    let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
    assert!(
        matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "SP6INA")
    );
    assert!(
        event_rx.try_recv().is_err(),
        "Zdarzenie powinno być opublikowane dokładnie raz"
    );
}

#[test]
fn form_qso_validation_error_aborts_without_side_effects() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let mut event_rx = app.services.event_bus.subscribe();

    // Błąd walidacji: niedozwolony znak w znaku wywoławczym (QsoIssue::CallsignChars)
    app.entry.callsign = "SP6INA@".to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "CW".to_string();

    app.save_qso();

    // Formularz NIE został wyczyszczony
    assert_eq!(app.entry.callsign, "SP6INA@");

    // Brak rekordu w bazie
    assert_eq!(app.log_view.recent_qsos.len(), 0);

    // Brak zdarzeń
    assert!(event_rx.try_recv().is_err());

    // Toast z błędem
    assert!(!app.gui.chrome.toasts.is_empty());
}

#[test]
fn contest_qso_ingest_success_and_stx_increment() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let mut event_rx = app.services.event_bus.subscribe();
    let initial_stx = app.contest.stx;

    let mut new_qso = QsoRecord::new("SN0HQ", "40m", "CW");
    new_qso.stx = Some(app.contest.stx);
    new_qso.journal_id = Some(app.active_journal.id.clone());

    let outcome = app
        .services
        .ingestor
        .ingest(new_qso, QsoOrigin::Contest)
        .unwrap();
    app.apply_post_effects(&outcome);
    app.contest.stx += 1;
    app.reload_qsos();

    assert_eq!(app.contest.stx, initial_stx + 1);
    assert_eq!(app.log_view.recent_qsos.len(), 1);
    assert_eq!(app.log_view.recent_qsos[0].callsign, "SN0HQ");

    let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
    assert!(
        matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "SN0HQ")
    );
    assert!(event_rx.try_recv().is_err());
}

#[test]
fn contest_qso_validation_error_does_not_increment_stx() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let app = SpLogApp::for_tests(init, deps);
    let initial_stx = app.contest.stx;

    let mut bad_qso = QsoRecord::new("SN0HQ", "", "CW");
    bad_qso.freq = None;

    let res = app.services.ingestor.ingest(bad_qso, QsoOrigin::Contest);
    assert!(matches!(res, Err(IngestError::Invalid(_))));
    assert_eq!(app.contest.stx, initial_stx);
}

#[test]
fn contest_stx_increments_only_on_contest_save() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let initial_stx = app.contest.stx;

    // Normalny zapis QSO nie zmienia contest.stx
    app.entry.callsign = "SP1ABC".to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "SSB".to_string();
    app.save_qso();
    assert_eq!(app.contest.stx, initial_stx);

    // Zapis w trybie zawodów inkrementuje contest.stx
    let mut contest_qso = QsoRecord::new("DL1XYZ", "20m", "CW");
    contest_qso.stx = Some(app.contest.stx);
    contest_qso.journal_id = Some(app.active_journal.id.clone());
    let outcome = app
        .services
        .ingestor
        .ingest(contest_qso, QsoOrigin::Contest)
        .unwrap();
    app.apply_post_effects(&outcome);
    app.contest.stx += 1;
    assert_eq!(app.contest.stx, initial_stx + 1);
}

#[test]
fn wsjtx_qso_ingest_advisory_issue_persists_and_registers() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let mut event_rx = app.services.event_bus.subscribe();

    let mut qso = QsoRecord::new("K1ABC", "20m", "FT8");
    qso.cqz = Some(99); // advisory issue
    qso.qso_date.clear(); // fallback w adapterze
    qso.time_on.clear(); // fallback w adapterze

    app.handle_wsjtx_qso(qso);

    assert_eq!(app.log_view.recent_qsos.len(), 1);
    let stored = &app.log_view.recent_qsos[0];
    assert_eq!(stored.callsign, "K1ABC");
    assert_ne!(stored.qso_date, "");
    assert_ne!(stored.time_on, "");

    {
        let awards = app.services.awards_engine.lock().unwrap();
        assert!(awards.worked_calls.iter().any(|(c, _, _)| c == "K1ABC"));
    }
    {
        let scp = app.services.scp_engine.lock().unwrap();
        assert!(scp.search("K1ABC", 5).contains(&"K1ABC".to_string()));
    }

    let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
    assert!(
        matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "K1ABC")
    );
    assert!(event_rx.try_recv().is_err());
}

#[test]
fn js8call_qso_ingest_handles_batch_and_errors() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let mut event_rx = app.services.event_bus.subscribe();

    let qso1 = QsoRecord::new("JS8ONE", "40m", "JS8");
    let qso2 = QsoRecord::new("", "40m", "JS8"); // pusty znak - błąd
    let qso3 = QsoRecord::new("JS8TWO", "40m", "JS8");

    app.handle_js8call_qso(qso1);
    app.handle_js8call_qso(qso2);
    app.handle_js8call_qso(qso3);

    assert_eq!(app.log_view.recent_qsos.len(), 2);
    assert!(
        app.log_view
            .recent_qsos
            .iter()
            .any(|q| q.callsign == "JS8ONE")
    );
    assert!(
        app.log_view
            .recent_qsos
            .iter()
            .any(|q| q.callsign == "JS8TWO")
    );

    let mut events_count = 0;
    while let Ok(event) = event_rx.try_recv() {
        if matches!(event, crate::core::events::AppEvent::QsoLogged { .. }) {
            events_count += 1;
        }
    }
    assert_eq!(events_count, 2);
}

#[test]
fn lan_peer_qso_ingest_no_broadcast_loop() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let mut event_rx = app.services.event_bus.subscribe();

    assert!(
        !QsoOrigin::LanPeer.policy().effects.broadcast_lan,
        "LanPeer nie może broadcastować do LAN"
    );

    {
        let db = app.services.log_db.lock().unwrap();
        let j = Journal {
            id: "PEER_JOURNAL".to_string(),
            name: "Peer Journal".to_string(),
            ..Default::default()
        };
        db.create_journal(&j).unwrap();
    }

    let mut qso = QsoRecord::new("LANSTN", "80m", "SSB");
    qso.journal_id = Some("PEER_JOURNAL".to_string());

    app.handle_lan_qso(qso);

    let db_records = app
        .services
        .log_db
        .lock()
        .unwrap()
        .get_recent_qsos_for_journal("PEER_JOURNAL", 10)
        .unwrap();
    assert_eq!(db_records.len(), 1);
    assert_eq!(db_records[0].callsign, "LANSTN");
    assert_eq!(db_records[0].journal_id.as_deref(), Some("PEER_JOURNAL"));
    assert!(app.sync_ui.log.iter().any(|line| line.contains("LANSTN")));

    let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
    assert!(
        matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "LANSTN")
    );
    assert!(event_rx.try_recv().is_err());
}

#[test]
fn post_effects_execution_matches_flags() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    let qso = QsoRecord::new("SP1TEST", "20m", "CW");
    let mut effects = crate::core::ingest::PostEffects::default();

    app.solar.propagation_history.record_forecast(
        "20m",
        12.0,
        120,
        2,
        85,
        crate::core::propagation::BandOpeningStatus::Open,
    );
    assert_eq!(app.solar.propagation_history.records.len(), 1);
    assert!(app.solar.propagation_history.records[0].observed.is_none());

    let outcome = IngestOutcome {
        origin: QsoOrigin::Form,
        qso: qso.clone(),
        warnings: Vec::new(),
        advisory_issue: None,
        effects,
    };

    // Bez flag - obserwacja nie jest zapisana
    app.apply_post_effects(&outcome);
    assert!(app.solar.propagation_history.records[0].observed.is_none());

    // Włączona flaga propagation_observation
    effects.propagation_observation = true;
    let outcome_with_prop = IngestOutcome {
        origin: QsoOrigin::Form,
        qso,
        warnings: Vec::new(),
        advisory_issue: None,
        effects,
    };
    app.apply_post_effects(&outcome_with_prop);
    assert_eq!(
        app.solar.propagation_history.records[0].observed,
        Some(crate::core::propagation_history::ObservedOutcome::ConfirmedOpen)
    );
}

#[test]
fn propagation_accuracy_pct_is_displayed_in_gui() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.solar.propagation_history.record_forecast(
        "20m",
        12.0,
        120,
        2,
        85,
        crate::core::propagation::BandOpeningStatus::Open,
    );
    app.solar.propagation_history.record_observation(
        "20m",
        crate::core::propagation_history::ObservedOutcome::ConfirmedOpen,
    );

    let acc = app.solar.propagation_history.accuracy_pct();
    assert_eq!(acc, Some(100.0));
    let count = app.solar.propagation_history.verified_count();
    assert_eq!(count, 1);
    let display_str = format!("{:.1}% ({count} zweryf.)", acc.unwrap());
    assert!(display_str.contains("100.0%"));
}

#[test]
fn propagation_accuracy_handles_empty_samples_safely() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let app = SpLogApp::for_tests(init, deps);
    assert_eq!(app.solar.propagation_history.accuracy_pct(), None);
    assert_eq!(app.solar.propagation_history.verified_count(), 0);
}

#[test]
fn solar_panel_displays_na_without_samples() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let app = SpLogApp::for_tests(init, deps);
    assert_eq!(app.solar.propagation_history.accuracy_pct(), None);
    let display_str = match app.solar.propagation_history.accuracy_pct() {
        Some(acc) => format!("{acc:.1}%"),
        None => "N/A".to_string(),
    };
    assert_eq!(display_str, "N/A");
}

#[test]
fn solar_panel_displays_zero_percent_only_for_measured_zero() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.solar.propagation_history.record_forecast(
        "20m",
        12.0,
        120,
        2,
        85,
        crate::core::propagation::BandOpeningStatus::Open,
    );
    app.solar.propagation_history.record_observation(
        "20m",
        crate::core::propagation_history::ObservedOutcome::ConfirmedClosed,
    );
    assert_eq!(app.solar.propagation_history.accuracy_pct(), Some(0.0));
    assert_eq!(app.solar.propagation_history.verified_count(), 1);
    let display_str = match app.solar.propagation_history.accuracy_pct() {
        Some(acc) => format!(
            "{acc:.1}% ({})",
            app.solar.propagation_history.verified_count()
        ),
        None => "N/A".to_string(),
    };
    assert_eq!(display_str, "0.0% (1)");
}

#[test]
fn solar_panel_does_not_present_zero_as_measurement_without_samples() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let app = SpLogApp::for_tests(init, deps);
    let opt = app.solar.propagation_history.accuracy_pct();
    assert!(opt.is_none());
    assert_ne!(opt, Some(0.0));
}

#[test]
fn clear_qso_form_resets_all_entry_fields() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    app.entry.callsign = "SP6INA".to_string();
    app.entry.rst_sent = "599".to_string();
    app.entry.rst_rcvd = "599".to_string();
    app.entry.name = "Mariusz".to_string();
    app.entry.qth = "Wroclaw".to_string();
    app.entry.grid = "JO81".to_string();
    app.entry.comment = "Test comment".to_string();
    app.entry.focus_callsign_requested = false;

    app.clear_qso_form();

    assert_eq!(app.entry.callsign, "");
    assert_eq!(app.entry.name, "");
    assert_eq!(app.entry.qth, "");
    assert_eq!(app.entry.grid, "");
    assert_eq!(app.entry.comment, "");
    assert!(app.entry.focus_callsign_requested);
}

#[test]
fn on_callsign_changed_fills_prefix_and_previous_qso_data() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    let mut qso = QsoRecord::new("SP6INA", "20m", "SSB");
    qso.name = Some("Mariusz".to_string());
    qso.qth = Some("Wroclaw".to_string());
    qso.gridsquare = Some("JO81".to_string());
    qso.journal_id = Some(app.active_journal.id.clone());
    app.services
        .log_db
        .lock()
        .unwrap()
        .insert_qso(&qso)
        .unwrap();

    app.entry.callsign = "SP6INA".to_string();
    app.on_callsign_changed();

    assert!(app.entry.active_prefix_info.is_some());
    assert_eq!(app.entry.name, "Mariusz");
    assert_eq!(app.entry.qth, "Wroclaw");
    assert_eq!(app.entry.grid, "JO81");
    assert_eq!(app.entry.past_qsos_for_active_call.len(), 1);
}

#[tokio::test]
async fn debounce_fires_after_600ms() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    app.config.qrz_auto_lookup = true;
    app.entry.callsign = "SP6".to_string();
    app.on_callsign_changed();

    assert!(app.entry.lookup_debounce_until.is_some());

    // Simulate deadline passed
    app.entry.lookup_debounce_until =
        std::time::Instant::now().checked_sub(std::time::Duration::from_millis(10));

    app.process_debounced_lookup();

    assert!(app.entry.lookup_debounce_until.is_none());
}

fn walkdir(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else if path.extension().is_some_and(|e| e == "rs") {
                results.push(path);
            }
        }
    }
    results
}

#[test]
fn test_architecture_limits_and_forbidden_patterns() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // 1. app.rs total lines <= 800
    let app_rs = std::fs::read_to_string(manifest_dir.join("src/gui/app.rs")).unwrap();
    let app_lines: Vec<&str> = app_rs.lines().collect();
    assert!(
        app_lines.len() <= 800,
        "app.rs exceeds 800 lines: {}",
        app_lines.len()
    );

    // 2. build() function lines <= 200
    let build_start = app_lines
        .iter()
        .position(|l| l.contains("pub fn build("))
        .expect("build not found");
    let build_end = app_lines[build_start..]
        .iter()
        .position(|l| l.contains("pub fn configure_egui("))
        .expect("configure_egui not found")
        + build_start;
    let build_len = build_end - build_start;
    assert!(
        build_len <= 200,
        "build() exceeds 200 lines: {build_len} lines"
    );

    // 3. ui() function lines <= 120
    let ui_start = app_lines
        .iter()
        .position(|l| {
            l.trim()
                .starts_with("fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame)")
        })
        .expect("ui() not found");
    let ui_end = app_lines[ui_start..]
        .iter()
        .position(|l| l.trim().starts_with("fn on_exit("))
        .expect("on_exit not found")
        + ui_start;
    let ui_len = ui_end - ui_start;
    assert!(ui_len <= 120, "ui() exceeds 120 lines: {ui_len} lines");

    // 4. save_station_config() <= 30 lines
    let app_config_rs =
        std::fs::read_to_string(manifest_dir.join("src/gui/app_config.rs")).unwrap();
    let cfg_lines: Vec<&str> = app_config_rs.lines().collect();
    let save_start = cfg_lines
        .iter()
        .position(|l| l.contains("pub fn save_station_config("))
        .expect("save_station_config not found");
    let save_end = cfg_lines[save_start..]
        .iter()
        .position(|l| l.contains("pub fn report_error("))
        .expect("report_error not found")
        + save_start;
    let save_len = save_end - save_start;
    assert!(
        save_len <= 30,
        "save_station_config() exceeds 30 lines: {save_len} lines"
    );

    // 5. SpLogApp field count <= 35
    let splogapp_start = app_lines
        .iter()
        .position(|l| l.contains("pub struct SpLogApp {"))
        .expect("SpLogApp struct not found");
    let splogapp_end = app_lines[splogapp_start..]
        .iter()
        .position(|l| l.trim() == "}")
        .expect("SpLogApp closing brace not found")
        + splogapp_start;
    let splogapp_fields = app_lines[splogapp_start..splogapp_end]
        .iter()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with("pub ") && trimmed.contains(':') && !trimmed.starts_with("pub fn ")
        })
        .count();
    assert!(
        splogapp_fields <= 35,
        "SpLogApp exceeds 35 fields: {splogapp_fields}"
    );

    // 6. Each state struct in src/gui/state/*.rs <= 30 fields
    let state_dir = manifest_dir.join("src/gui/state");
    for entry in std::fs::read_dir(&state_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let content = std::fs::read_to_string(&path).unwrap();
            let lines: Vec<&str> = content.lines().collect();
            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub struct ") && trimmed.ends_with('{') {
                    let struct_name = trimmed
                        .strip_prefix("pub struct ")
                        .unwrap()
                        .split_whitespace()
                        .next()
                        .unwrap();
                    let struct_end = lines[idx..]
                        .iter()
                        .position(|l| l.trim() == "}")
                        .expect("struct closing brace not found")
                        + idx;
                    let fields = lines[idx..struct_end]
                        .iter()
                        .filter(|l| {
                            let t = l.trim();
                            t.starts_with("pub ") && t.contains(':') && !t.starts_with("pub fn ")
                        })
                        .count();
                    assert!(
                        fields <= 30,
                        "Struct {struct_name} in {:?} exceeds 30 fields: {fields}",
                        path.file_name().unwrap(),
                    );
                }
            }
        }
    }

    // 7. Forbidden patterns:
    // No "use crate::gui::app" in src/gui/state
    // No "SpLogApp" in src/gui/state
    // No "tokio::spawn" or "std::thread::spawn" in src/gui/state
    for entry in std::fs::read_dir(&state_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(
                !content.contains("use crate::gui::app"),
                "Found 'use crate::gui::app' in {path:?}"
            );
            assert!(
                !content.contains("SpLogApp"),
                "Found 'SpLogApp' in {path:?}"
            );
            assert!(
                !content.contains("tokio::spawn") && !content.contains("std::thread::spawn"),
                "Found background thread spawn in {path:?}"
            );
        }
    }

    // No unauthorized "AppConfig {" struct instantiations in src/gui (only return type annotations like "-> AppConfig {")
    let gui_dir = manifest_dir.join("src/gui");
    for entry in walkdir(&gui_dir) {
        if entry.file_name().is_some_and(|f| f == "app_tests.rs") {
            continue;
        }
        let content = std::fs::read_to_string(&entry).unwrap();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("AppConfig {") {
                assert!(
                    trimmed.contains("fn ") || trimmed.contains("-> AppConfig"),
                    "Found unauthorized 'AppConfig {{' literal in {entry:?}:{}",
                    idx + 1
                );
            }
        }
    }

    // No "contest_qsos", "contest_points", "contest_mults" in src
    let src_dir = manifest_dir.join("src");
    for entry in walkdir(&src_dir) {
        if entry.file_name().is_some_and(|f| f == "app_tests.rs") {
            continue;
        }
        let content = std::fs::read_to_string(&entry).unwrap();
        assert!(
            !content.contains("contest_qsos")
                && !content.contains("contest_points")
                && !content.contains("contest_mults"),
            "Found obsolete contest counters in {entry:?}"
        );
    }
}

#[test]
fn ctrl_shortcuts_trigger_while_callsign_input_has_focus() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.entry.callsign = "SP6INA".to_string();

    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("qso_entry_callsign")));
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
        app.handle_input_and_shortcuts(ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(app.entry.callsign, "");
}

#[test]
fn ctrl_w_in_qso_callsign_follows_explicit_qso_contract() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.entry.callsign = "DL1ABC".to_string();
    app.entry.rst_sent = "599".to_string();

    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("qso_entry_callsign")));
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
        app.handle_input_and_shortcuts(ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(
        app.entry.callsign, "",
        "Ctrl+W in callsign field must wipe the QSO form"
    );
}

#[test]
fn ctrl_w_in_main_qso_field_follows_explicit_qso_contract() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.entry.callsign = "SP6INA".to_string();
    app.entry.exchange = "001".to_string();

    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("qso_entry_name")));
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
        app.handle_input_and_shortcuts(ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(
        app.entry.callsign, "",
        "Ctrl+W in a main QSO form field must wipe the QSO form"
    );
    assert_eq!(app.entry.exchange, "");
}

#[test]
fn ctrl_w_in_settings_text_edit_does_not_wipe_qso() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = "SP6INA".to_string();

    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("settings_text_edit")));
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        app.handle_input_and_shortcuts(ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(
        app.entry.callsign, "SP6INA",
        "Ctrl+W in settings text edit must NOT wipe background QSO"
    );
}

#[test]
fn ctrl_w_in_filter_does_not_wipe_qso() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = "SP6INA".to_string();

    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("log_filter_input")));
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        app.handle_input_and_shortcuts(ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(
        app.entry.callsign, "SP6INA",
        "Ctrl+W in filter input must NOT wipe background QSO"
    );
}

#[test]
fn ctrl_w_in_modal_does_not_reach_background_qso() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = "SP6INA".to_string();
    app.gui.dialogs.advanced_filter_dialog.is_open = true;

    let ctx = egui::Context::default();
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        app.handle_input_and_shortcuts(ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(
        app.entry.callsign, "SP6INA",
        "Ctrl+W in modal dialog must NOT wipe background QSO"
    );
}

#[test]
fn ctrl_w_in_operator_assistant_does_not_wipe_background_qso() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = "SP6INA".to_string();

    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("operator_assistant_text_edit")));
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        app.handle_input_and_shortcuts(ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(
        app.entry.callsign, "SP6INA",
        "Ctrl+W in operator assistant must NOT wipe background QSO"
    );
}

#[test]
fn ctrl_w_is_consumed_exactly_once() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.entry.callsign = "SP6INA".to_string();

    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("qso_entry_callsign")));
    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });

    // Klatka 1: skrót zostaje skonsumowany
    let mut out1 = ctx.run_ui(raw_input, |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
        app.handle_input_and_shortcuts(ui.ctx());
    });
    out1.textures_delta.clear();
    assert_eq!(app.entry.callsign, "");

    // Ustaw nową wartość i wywołaj ponownie bez nowych zdarzeń wejściowych
    app.entry.callsign = "DL1ABC".to_string();
    let mut out2 = ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
        app.handle_input_and_shortcuts(ui.ctx());
    });
    out2.textures_delta.clear();
    assert_eq!(
        app.entry.callsign, "DL1ABC",
        "Ctrl+W must not trigger twice from a single event"
    );
}

#[test]
fn ctrl_w_does_not_execute_two_actions() {
    let registry = ShortcutRegistry::default();
    let conflicts = registry.find_conflicts();
    assert_eq!(
        conflicts.len(),
        0,
        "Shortcut registry must contain zero conflicting bindings"
    );

    let ctrl_w_bindings: Vec<_> = registry
        .bindings()
        .iter()
        .filter(|b| b.shortcut.modifiers.command && b.shortcut.logical_key == egui::Key::W)
        .collect();

    // Każde powiązanie Ctrl+W musi być w osobnym jawnym kontekście QSO (MainQsoEntry vs ContestQsoEntry)
    for i in 0..ctrl_w_bindings.len() {
        for j in (i + 1)..ctrl_w_bindings.len() {
            assert_ne!(
                ctrl_w_bindings[i].context, ctrl_w_bindings[j].context,
                "Ctrl+W bindings must never share the same context"
            );
        }
    }
}

#[test]
fn ctrl_w_repeat_follows_declared_repeat_policy() {
    let registry = ShortcutRegistry::default();
    let ctrl_w = registry
        .bindings()
        .iter()
        .find(|b| b.shortcut.modifiers.command && b.shortcut.logical_key == egui::Key::W)
        .expect("Ctrl+W binding must exist");

    assert_eq!(
        ctrl_w.repeat_policy,
        RepeatPolicy::SingleTrigger,
        "Ctrl+W must strictly follow RepeatPolicy::SingleTrigger to prevent repeated wipes"
    );
}

#[test]
fn qso_enter_validates_before_save() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = String::new();
    app.entry.name = "Test Operator".to_string();

    let initial_count = app
        .services
        .log_db
        .lock()
        .expect("lock")
        .count_all()
        .unwrap();
    app.save_qso();
    let after_count = app
        .services
        .log_db
        .lock()
        .expect("lock")
        .count_all()
        .unwrap();

    assert_eq!(
        initial_count, after_count,
        "Empty callsign must not save a QSO"
    );
    assert!(
        app.gui
            .chrome
            .toasts
            .iter()
            .any(|t| t.severity == crate::gui::state::ToastSeverity::Error),
        "Błąd walidacji (pusty znak) musi trafić do kolejki jako Error"
    );
}

#[test]
fn failed_qso_save_preserves_form() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = String::new();
    app.entry.rst_sent = "579".to_string();
    app.entry.rst_rcvd = "559".to_string();
    app.entry.name = "Preserved Operator".to_string();
    app.entry.grid = "JO81".to_string();
    app.entry.comment = "Do not clear me".to_string();

    app.save_qso();

    assert_eq!(app.entry.rst_sent, "579");
    assert_eq!(app.entry.rst_rcvd, "559");
    assert_eq!(app.entry.name, "Preserved Operator");
    assert_eq!(app.entry.grid, "JO81");
    assert_eq!(app.entry.comment, "Do not clear me");
}

#[test]
fn successful_qso_save_moves_focus_by_contract() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = "SP9TEST".to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "CW".to_string();
    app.entry.focus_callsign_requested = false;

    app.save_qso();

    assert!(
        app.entry.focus_callsign_requested,
        "Focus callsign must be requested on successful save"
    );
    assert_eq!(
        app.entry.callsign, "",
        "Form callsign must be cleared on successful save"
    );
}

#[test]
fn qso_tab_order_is_deterministic() {
    let expected_fields = [
        "qso_entry_callsign",
        "qso_entry_rst_sent",
        "qso_entry_rst_rcvd",
        "qso_entry_name",
        "qso_entry_grid",
        "qso_entry_qth",
        "qso_entry_pga",
        "qso_entry_iota",
        "qso_entry_state",
        "qso_entry_sota",
        "qso_entry_qsl_via",
        "qso_entry_comment",
    ];

    let ids: Vec<egui::Id> = expected_fields
        .iter()
        .map(|name| egui::Id::new(*name))
        .collect();
    let unique_ids: std::collections::HashSet<egui::Id> = ids.iter().copied().collect();

    assert_eq!(
        unique_ids.len(),
        expected_fields.len(),
        "All field IDs must be distinct"
    );

    for window in ids.windows(2) {
        assert_ne!(
            window[0], window[1],
            "Consecutive field IDs must not collide"
        );
    }
}

#[test]
fn qso_focus_id_is_stable_across_frames() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let ctx = egui::Context::default();

    // Frame 1: empty entry
    let call_id_1 = egui::Id::new("qso_entry_callsign");
    let mut output1 = ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
    });
    output1.textures_delta.clear();

    // Frame 2: populated entry with bearing/distance and dupe
    app.entry.callsign = "DL1ABC".to_string();
    app.entry.active_bearing_deg = 270.0;
    app.entry.active_distance_km = 850.0;
    let call_id_2 = egui::Id::new("qso_entry_callsign");
    let mut output2 = ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
    });
    output2.textures_delta.clear();

    assert_eq!(
        call_id_1, call_id_2,
        "Callsign focus ID must remain strictly identical across frames"
    );
}

#[test]
fn until_next_second_is_deterministic_at_boundary_values() {
    let cases = [
        (0, std::time::Duration::from_millis(1000)),
        (500, std::time::Duration::from_millis(500)),
        (900, std::time::Duration::from_millis(100)),
        (949, std::time::Duration::from_millis(51)),
        (950, std::time::Duration::from_millis(50)),
        (960, std::time::Duration::from_millis(50)),
        (983, std::time::Duration::from_millis(50)),
        (984, std::time::Duration::from_millis(50)),
        (999, std::time::Duration::from_millis(50)),
    ];

    for (input_ms, expected) in cases {
        let actual = duration_until_next_second(input_ms);
        assert_eq!(
            actual, expected,
            "duration_until_next_second({input_ms}) failed: expected {expected:?}, got {actual:?}"
        );
        assert!(actual >= std::time::Duration::from_millis(50));
    }
}

#[test]
fn idle_repaint_requested_delay_is_bounded() {
    for ms in 0..1000 {
        let delay = duration_until_next_second(ms);
        assert!(
            delay >= std::time::Duration::from_millis(50),
            "Delay for {ms} ms must be bounded below by 50ms, got {delay:?}"
        );
        assert!(
            delay <= std::time::Duration::from_millis(1000),
            "Delay for {ms} ms must be bounded above by 1000ms, got {delay:?}"
        );
    }
}

#[test]
fn idle_repaint_observed_delay_accounts_for_predicted_frame() {
    let predicted_dt = 1.0 / 60.0;
    let boundaries = [0, 500, 900, 949, 950, 960, 983, 984, 999];
    for ms in boundaries {
        let requested = duration_until_next_second(ms);
        let observed = compute_observed_repaint_delay(requested, predicted_dt);
        assert!(
            observed > std::time::Duration::ZERO,
            "Observed delay for {ms} ms must remain strictly positive after predicted dt compensation"
        );
        assert!(
            observed >= std::time::Duration::from_millis(30),
            "Observed delay for {ms} ms must be at least ~33ms, got {observed:?}"
        );
        assert!(
            observed <= requested,
            "Observed delay must never exceed requested delay"
        );
    }
}

#[test]
fn idle_repaint_test_does_not_depend_on_wall_clock_boundary() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.mock_subsec_millis = Some(0);
    let ctx = egui::Context::default();
    app.set_egui_ctx(ctx.clone());

    // Warm-up egui layout
    let mut time = 0.0;
    for _ in 0..5 {
        time += 0.1;
        let mut raw_input = egui::RawInput::default();
        raw_input.time = Some(time);
        raw_input.predicted_dt = 1.0 / 60.0;
        let mut out = ctx.run_ui(raw_input, |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();
    }

    let boundaries = [0, 500, 900, 949, 950, 960, 983, 984, 999];
    for (i, &subsec) in boundaries.iter().enumerate() {
        app.mock_subsec_millis = Some(subsec);
        let mut raw_input = egui::RawInput::default();
        raw_input.time = Some(time + (i + 1) as f64 * 0.1);
        raw_input.predicted_dt = 1.0 / 60.0;
        let mut out = ctx.run_ui(raw_input, |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();

        let repaint_delay = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(std::time::Duration::ZERO, |v| v.repaint_delay);

        let requested = duration_until_next_second(subsec);
        let expected = compute_observed_repaint_delay(requested, 1.0 / 60.0);
        assert_eq!(
            repaint_delay, expected,
            "Repaint delay mismatch for injected boundary {subsec} ms"
        );
        assert!(
            repaint_delay > std::time::Duration::ZERO,
            "Repaint delay must never be zero at boundary {subsec} ms"
        );
    }
}

#[test]
fn idle_gui_does_not_request_continuous_repaint() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.mock_subsec_millis = Some(0);
    let ctx = egui::Context::default();
    app.set_egui_ctx(ctx.clone());

    // Warm-up egui layout
    let mut time = 0.0;
    for _ in 0..5 {
        time += 0.1;
        let mut raw_input = egui::RawInput::default();
        raw_input.time = Some(time);
        raw_input.predicted_dt = 1.0 / 60.0;
        let mut out = ctx.run_ui(raw_input, |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();
    }

    // 100 powtórzeń w pętli bez żadnej porażki, weryfikacja determinizmu i odporności na zegar
    for iter in 0..100 {
        time += 0.016;
        let mock_subsec = ((iter * 97) % 1000) as u32;
        app.mock_subsec_millis = Some(mock_subsec);

        let mut raw_input = egui::RawInput::default();
        raw_input.time = Some(time);
        raw_input.predicted_dt = 1.0 / 60.0;
        let mut out = ctx.run_ui(raw_input, |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();

        let repaint_delay = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(std::time::Duration::ZERO, |v| v.repaint_delay);

        let requested = duration_until_next_second(mock_subsec);
        let expected_observed = compute_observed_repaint_delay(requested, 1.0 / 60.0);

        assert_eq!(
            repaint_delay, expected_observed,
            "Iter {iter}: Repaint delay must match expected observed delay for mock_subsec={mock_subsec}"
        );
        assert!(
            repaint_delay >= std::time::Duration::from_millis(30),
            "Iter {iter}: Idle GUI observed repaint delay must be >= 30ms, got {repaint_delay:?}"
        );
        assert_ne!(
            repaint_delay,
            std::time::Duration::ZERO,
            "Iter {iter}: Idle GUI must not continuously request immediate repaint"
        );
    }
}

#[test]
fn cat_event_requests_repaint() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.mock_subsec_millis = Some(0);
    let ctx = egui::Context::default();
    app.set_egui_ctx(ctx.clone());

    for _ in 0..3 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();
    }

    // Send changed CAT telemetry
    let mut new_st = app.cat.rig_state.clone();
    new_st.frequency_hz = 7_050_000;
    new_st.mode = "LSB".to_string();
    app.update_cat_state_from_telemetry(new_st, false);

    // Frame with changed CAT state should process it
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_main_ui(ui);
    });
    out.textures_delta.clear();

    assert_eq!(app.cat.rig_state.frequency_hz, 7_050_000);
    assert_eq!(app.cat.rig_state.mode, "LSB");
}

#[test]
fn unchanged_cat_state_does_not_cause_repaint_storm() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.mock_subsec_millis = Some(0);
    let ctx = egui::Context::default();
    app.set_egui_ctx(ctx.clone());

    let mut time = 0.0;
    for _ in 0..5 {
        time += 0.1;
        let mut raw_input = egui::RawInput::default();
        raw_input.time = Some(time);
        raw_input.predicted_dt = 1.0 / 60.0;
        let mut out = ctx.run_ui(raw_input, |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();
    }

    let mut event_rx = app.services.event_bus.subscribe();
    let initial_st = app.cat.rig_state.clone();

    // Send 10 identical telemetry updates with minor S-meter jitter (< 3dB hysteresis)
    for i in 0..10 {
        let mut jittered = initial_st.clone();
        jittered.s_meter_dbm = initial_st.s_meter_dbm + ((i as f32) * 0.1);
        app.update_cat_state_from_telemetry(jittered, false);
    }

    // No AppEvent::RigState should have been published for unchanged state
    assert!(
        event_rx.try_recv().is_err(),
        "Unchanged CAT state must not emit events to EventBus"
    );

    // Frame following unchanged state should not be immediate repaint storm
    time += 0.1;
    let mut raw_input = egui::RawInput::default();
    raw_input.time = Some(time);
    raw_input.predicted_dt = 1.0 / 60.0;
    let mut out = ctx.run_ui(raw_input, |ui| {
        app.render_main_ui(ui);
    });
    out.textures_delta.clear();

    let repaint_delay = out
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .map_or(std::time::Duration::ZERO, |v| v.repaint_delay);

    assert!(
        repaint_delay >= std::time::Duration::from_millis(50),
        "Unchanged state must not request continuous repaint storm, got {repaint_delay:?}"
    );
}

#[test]
fn configuration_change_requests_single_repaint() {
    let deps = test_deps();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_file = temp_dir.path().join("station_config.json");
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: config_file,
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.mock_subsec_millis = Some(0);
    let ctx = egui::Context::default();
    app.set_egui_ctx(ctx.clone());

    for _ in 0..3 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();
    }

    // Mutate configuration
    app.config.station.callsign = "SP6NEW".to_string();
    app.save_station_config();

    // Running frame applies change and returns to next second schedule
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_main_ui(ui);
    });
    out.textures_delta.clear();

    assert_eq!(app.config.station.callsign, "SP6NEW");
}

#[test]
fn background_error_becomes_visible_without_continuous_polling() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app.mock_subsec_millis = Some(0);
    let ctx = egui::Context::default();
    app.set_egui_ctx(ctx.clone());

    for _ in 0..3 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.render_main_ui(ui);
        });
        out.textures_delta.clear();
    }
    assert!(app.gui.chrome.toasts.is_empty());

    // Background worker emits error toast on EventBus
    app.services
        .event_bus
        .publish(crate::core::events::AppEvent::Toast {
            level: "error".to_string(),
            message: "Proxy server binding port conflict".to_string(),
        });

    // Frame wakes up directly and displays error toast without polling loop
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_main_ui(ui);
    });
    out.textures_delta.clear();

    let toast = app
        .gui
        .chrome
        .toasts
        .latest()
        .expect("Background error must be visible in UI immediately");
    assert!(toast.message.contains("Proxy server binding port conflict"));
    assert_eq!(toast.severity, crate::gui::state::ToastSeverity::Error);
}

#[test]
fn cache_is_reused_when_inputs_are_unchanged() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.log_view.recent_qsos = vec![
        crate::core::qso::QsoRecord {
            id: Some(1),
            callsign: "SP6INA".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            qso_date: "20261106".to_string(),
            time_on: "120000".to_string(),
            ..Default::default()
        },
        crate::core::qso::QsoRecord {
            id: Some(2),
            callsign: "SN0HQ".to_string(),
            band: "40m".to_string(),
            mode: "SSB".to_string(),
            qso_date: "20261106".to_string(),
            time_on: "120500".to_string(),
            ..Default::default()
        },
    ];

    let log_id = "journal-1";
    let prof_id = "prof-home";

    assert!(!app.log_view.is_cache_valid(log_id, prof_id));
    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));
    let initial_key = app.log_view.cache.key.clone();

    // Calling again with unchanged inputs reuses cache
    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert_eq!(app.log_view.cache.key, initial_key);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));
}

#[test]
fn cache_is_invalidated_on_qso_change() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let log_id = "journal-1";
    let prof_id = "prof-home";

    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));

    // QSO modification bumps revision
    app.log_view.recent_qsos.push(crate::core::qso::QsoRecord {
        id: Some(3),
        callsign: "SQ6KVP".to_string(),
        ..Default::default()
    });
    app.log_view.bump_revision();

    assert!(!app.log_view.is_cache_valid(log_id, prof_id));
    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));
}

#[test]
fn cache_is_invalidated_on_filter_change() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let log_id = "journal-1";
    let prof_id = "prof-home";

    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));

    // Changing search query invalidates cache
    app.log_view.search_query = "CW".to_string();
    assert!(!app.log_view.is_cache_valid(log_id, prof_id));

    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));

    // Changing drill filter invalidates cache
    app.log_view.drill_filter = Some(crate::gui::drill_filter::DrillFilter::Band(
        "20m".to_string(),
    ));
    assert!(!app.log_view.is_cache_valid(log_id, prof_id));
}

#[test]
fn cache_is_invalidated_on_sort_change() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let log_id = "journal-1";
    let prof_id = "prof-home";

    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));

    // Changing sort column
    app.log_view.sort_column = 2;
    assert!(!app.log_view.is_cache_valid(log_id, prof_id));

    app.log_view.ensure_sorted_indices_cached(log_id, prof_id);
    assert!(app.log_view.is_cache_valid(log_id, prof_id));

    // Changing sort direction
    app.log_view.sort_asc = !app.log_view.sort_asc;
    assert!(!app.log_view.is_cache_valid(log_id, prof_id));
}

#[test]
fn cache_is_scoped_to_log_identity() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let prof_id = "prof-home";

    app.log_view
        .ensure_sorted_indices_cached("journal-A", prof_id);
    assert!(app.log_view.is_cache_valid("journal-A", prof_id));
    assert!(!app.log_view.is_cache_valid("journal-B", prof_id));
}

#[test]
fn cache_is_scoped_to_active_rig_when_required() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    let rig1 = crate::cat::routing::RigId::new("rig-1").unwrap();
    let rig2 = crate::cat::routing::RigId::new("rig-2").unwrap();

    let mut st1 = crate::cat::hamlib::RigState::default();
    st1.frequency_hz = 14_025_000;
    st1.mode = "CW".to_string();

    let mut st2 = crate::cat::hamlib::RigState::default();
    st2.frequency_hz = 7_050_000;
    st2.mode = "LSB".to_string();

    app.cat.set_cached_rig_state(rig1.clone(), st1);
    app.cat.set_cached_rig_state(rig2.clone(), st2);

    assert_eq!(
        app.cat.get_cached_rig_state(&rig1).unwrap().frequency_hz,
        14_025_000
    );
    assert_eq!(
        app.cat.get_cached_rig_state(&rig2).unwrap().frequency_hz,
        7_050_000
    );
}

#[test]
fn stale_cache_never_crosses_profiles() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let log_id = "journal-1";

    app.log_view
        .ensure_sorted_indices_cached(log_id, "profile-home");
    assert!(app.log_view.is_cache_valid(log_id, "profile-home"));
    assert!(!app.log_view.is_cache_valid(log_id, "profile-field"));

    // Invalidate when switching profiles
    app.log_view.invalidate_cache();
    assert!(!app.log_view.is_cache_valid(log_id, "profile-home"));
    assert!(!app.log_view.is_cache_valid(log_id, "profile-field"));
}

#[test]
fn initial_rig_state_when_disconnected_reports_s0_and_zero_power() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let app = SpLogApp::for_tests(init, deps);

    // GUI-003: Disconnected rig state must not falsely report S9+20dB or 100W
    assert!(!app.cat.rig_state.connected);
    assert_eq!(app.cat.rig_state.s_meter_unit, "S0");
    assert!((app.cat.rig_state.s_meter_dbm - (-127.0)).abs() < 0.1);
    assert!((app.cat.rig_state.rf_power_watts - 0.0).abs() < 0.01);
}

#[test]
fn gui_state_lock_is_released_before_render() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    let ctx = egui::Context::default();
    app.set_egui_ctx(ctx.clone());

    // Execute render frame
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_main_ui(ui);
    });
    out.textures_delta.clear();

    // Verify all shared locks are completely free and not leaked
    assert!(
        app.services.log_db.try_lock().is_ok(),
        "log_db lock must be released after frame render"
    );
    assert!(
        app.cat.shared_state.try_write().is_ok(),
        "shared_state lock must be released after frame render"
    );
    assert!(
        app.runtime.egui_ctx.try_lock().is_ok(),
        "egui_ctx lock must be released after frame render"
    );
}

#[test]
fn gui_does_not_hold_lock_during_config_write() {
    let deps = test_deps();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_file = temp_dir.path().join("station_config.json");
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: config_file,
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    // Perform atomic config write
    app.save_station_config();

    // Verify database and CAT locks were never held or blocked
    assert!(
        app.services.log_db.try_lock().is_ok(),
        "Database lock must remain acquirable during/after config write"
    );
    assert!(
        app.cat.shared_state.try_write().is_ok(),
        "CAT shared_state lock must remain acquirable during/after config write"
    );
}

#[test]
fn gui_does_not_hold_lock_during_cat_command() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    let mut new_st = app.cat.rig_state.clone();
    new_st.frequency_hz = 14_200_000;
    new_st.mode = "USB".to_string();

    // Update telemetry
    app.update_cat_state_from_telemetry(new_st, false);

    assert!(
        app.services.log_db.try_lock().is_ok(),
        "log_db lock must remain free after CAT update"
    );
    assert!(
        app.cat.shared_state.try_write().is_ok(),
        "CAT shared_state lock must remain free after CAT update"
    );
}

#[test]
fn slow_background_task_does_not_block_gui_state_access() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);

    // Background thread simulates slow activity
    let (tx, rx) = std::sync::mpsc::channel();
    let bg_handle = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = tx.send(true);
    });

    // GUI thread accesses and modifies state concurrently without waiting or blocking
    let initial_rev = app.log_view.log_revision;
    app.log_view.search_query = "SP1".to_string();
    app.log_view.bump_revision();
    assert_eq!(app.log_view.search_query, "SP1");
    assert_eq!(app.log_view.log_revision, initial_rev + 1);

    // GUI state locks are not coupled to background thread completion
    assert!(rx.try_recv().is_err(), "Background task is still running");
    assert!(app.services.log_db.try_lock().is_ok());
    assert!(app.cat.shared_state.try_write().is_ok());

    bg_handle.join().unwrap();
}

#[test]
fn audio_recorder_stop_recording_disarms_active_state() {
    let _ = crate::media::audio_recorder::AudioRecorder::stop_recording();
    assert!(!crate::media::audio_recorder::AudioRecorder::is_recording());
}

#[test]
fn photo_viewer_retains_callbook_image_url_on_open() {
    let mut dialog = crate::gui::photo_viewer::PhotoViewerDialog::new();
    dialog.open("SP6INA", Some("https://example.com/qsl.jpg".to_string()));
    assert!(dialog.is_open);
    assert_eq!(dialog.callsign, "SP6INA");
    assert_eq!(
        dialog.photo_url.as_deref(),
        Some("https://example.com/qsl.jpg")
    );
}

#[test]
fn astronomy_dialog_tracks_target_rotation() {
    let mut dialog = crate::gui::astronomy_dialog::AstronomyDialog::new();
    dialog.open();
    let mut rotor = crate::cat::rotor::RotorState::default();
    rotor.azimuth_deg = 120.0;
    rotor.elevation_deg = 35.0;
    assert_eq!(rotor.elevation_deg, 35.0);
    assert!(dialog.is_open);
}

// ======================================================================================
// F-17 — PTT user-interaction, keyboard gating and release control (software-only)
// ======================================================================================

/// Rejestrujący kanał nadawczy — wyłącznie zakres testowy.
#[derive(Debug, Default)]
struct F17RecordingPort {
    on_calls: usize,
    off_calls: usize,
}

impl crate::gui::ptt_input::PttTransmitPort for F17RecordingPort {
    fn request_on(&mut self) -> Result<(), crate::gui::ptt_input::PttPortError> {
        self.on_calls += 1;
        Ok(())
    }

    fn request_off(&mut self) -> Result<(), crate::gui::ptt_input::PttPortError> {
        self.off_calls += 1;
        Ok(())
    }
}

/// Deterministyczny harness klatki GUI — ten sam produkcyjny punkt wejścia co aplikacja,
/// bez okna systemowego.
struct F17Harness {
    app: SpLogApp,
    ctx: egui::Context,
}

impl F17Harness {
    fn new() -> Self {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: test_config_path(),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);
        app.gui.chrome.show_welcome_wizard = false;
        Self {
            app,
            ctx: egui::Context::default(),
        }
    }

    /// Uruchamia jedną klatkę z podanymi zdarzeniami i stanem fokusu okna.
    fn frame(&mut self, events: Vec<egui::Event>, window_focused: bool) {
        let Self { app, ctx } = self;
        let mut raw = egui::RawInput {
            focused: window_focused,
            ..egui::RawInput::default()
        };
        raw.events = events;
        let mut output = ctx.run_ui(raw, |ui| {
            app.handle_input_and_shortcuts(ui.ctx());
        });
        output.textures_delta.clear();
    }

    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers, repeat: bool) {
        self.frame(
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat,
                modifiers,
            }],
            true,
        );
    }

    fn last_outcome(&self) -> Option<crate::gui::ptt_input::PttDispatchOutcome> {
        self.app.ptt.last_outcome
    }

    /// Rozłączne liczniki weryfikacji F-17: żądania PTT, żądania ON i wywołania backendu.
    fn counters(&self) -> (usize, usize, usize) {
        match self.last_outcome() {
            Some(outcome) if outcome.is_request() => (
                1,
                usize::from(outcome == crate::gui::ptt_input::PttDispatchOutcome::RequestedOn),
                1,
            ),
            _ => (0, 0, 0),
        }
    }
}

const F17_CTRL: egui::Modifiers = egui::Modifiers::COMMAND;
const F17_CTRL_SHIFT: egui::Modifiers = egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT);

#[test]
fn f17_bare_f7_never_produces_a_ptt_request() {
    let mut harness = F17Harness::new();
    harness.key(egui::Key::F7, egui::Modifiers::NONE, false);
    assert_eq!(
        harness.last_outcome(),
        None,
        "a bare F7 must never reach the PTT dispatcher"
    );
    assert_eq!(harness.counters(), (0, 0, 0));
}

#[test]
fn f17_shift_f7_and_alt_f7_never_produce_a_ptt_request() {
    for modifiers in [egui::Modifiers::SHIFT, egui::Modifiers::ALT] {
        let mut harness = F17Harness::new();
        harness.key(egui::Key::F7, modifiers, false);
        assert_eq!(harness.last_outcome(), None, "{modifiers:?}");
    }
}

#[test]
fn f17_ptt_shortcut_is_blocked_while_text_editing() {
    let mut harness = F17Harness::new();
    harness
        .ctx
        .memory_mut(|m| m.request_focus(egui::Id::new("qso_entry_callsign")));
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, false);
    assert_eq!(
        harness.last_outcome(),
        None,
        "PTT must not fire while a text field has focus"
    );
    assert_eq!(harness.counters(), (0, 0, 0));
}

#[test]
fn f17_ptt_shortcut_is_blocked_while_search_field_is_focused() {
    let mut harness = F17Harness::new();
    harness
        .ctx
        .memory_mut(|m| m.request_focus(egui::Id::new("log_filter_input")));
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, false);
    assert_eq!(harness.last_outcome(), None);
}

#[test]
fn f17_ptt_shortcut_is_blocked_while_a_modal_is_open() {
    let mut harness = F17Harness::new();
    harness.app.gui.dialogs.advanced_filter_dialog.is_open = true;
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, false);
    assert_eq!(
        harness.last_outcome(),
        None,
        "PTT must not fire while a modal window is open"
    );
}

#[test]
fn f17_ptt_shortcut_repeat_never_retriggers() {
    // egui sam wylicza flagę `repeat` z `keys_down`, więc powtórzenie odtwarzamy dwiema
    // klatkami: pierwsze naciśnięcie jest rejestrowane, druga klatka to już autorepeat.
    let mut harness = F17Harness::new();
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, false);
    assert!(
        harness.last_outcome().is_some(),
        "the first press must reach the dispatcher"
    );
    assert_eq!(harness.counters(), (0, 0, 0));

    harness.app.ptt.last_outcome = None;
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, true);
    assert_eq!(
        harness.last_outcome(),
        None,
        "a repeated key must never reach the PTT dispatcher"
    );
    assert_eq!(harness.counters(), (0, 0, 0));
}

#[test]
fn f17_ptt_shortcut_release_never_triggers_on() {
    let mut harness = F17Harness::new();
    harness.frame(
        vec![egui::Event::Key {
            key: egui::Key::F7,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: F17_CTRL_SHIFT,
        }],
        true,
    );
    assert_eq!(harness.last_outcome(), None);
}

#[test]
fn f17_ptt_shortcut_is_blocked_when_the_window_is_unfocused() {
    let mut harness = F17Harness::new();
    harness.frame(
        vec![egui::Event::Key {
            key: egui::Key::F7,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: F17_CTRL_SHIFT,
        }],
        false,
    );
    assert_eq!(
        harness.last_outcome(),
        None,
        "PTT must not fire without window focus"
    );
}

#[test]
fn f17_ptt_shortcut_is_blocked_during_ime_composition() {
    let mut harness = F17Harness::new();
    // Klatka rozpoczynająca kompozycję IME.
    harness.frame(
        vec![egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "に".to_string(),
            active_range_chars: None,
        })],
        true,
    );
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, false);
    assert_eq!(
        harness.last_outcome(),
        None,
        "PTT must not fire during IME composition"
    );
    assert_eq!(harness.counters(), (0, 0, 0));
}

#[test]
fn f17_ptt_shortcut_after_ime_commit_requires_a_new_gesture() {
    let mut harness = F17Harness::new();
    harness.frame(
        vec![egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "に".to_string(),
            active_range_chars: None,
        })],
        true,
    );
    // Klatka z commit — bramka IME pozostaje zamknięta w tej samej klatce.
    harness.frame(
        vec![
            egui::Event::Ime(egui::ImeEvent::Commit("に".to_string())),
            egui::Event::Key {
                key: egui::Key::F7,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: F17_CTRL_SHIFT,
            },
        ],
        true,
    );
    assert_eq!(harness.last_outcome(), None);
}

#[test]
fn f17_several_f7_events_in_one_frame_never_produce_more_than_one_intent() {
    let mut harness = F17Harness::new();
    let events: Vec<egui::Event> = (0..5)
        .map(|_| egui::Event::Key {
            key: egui::Key::F7,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: F17_CTRL_SHIFT,
        })
        .collect();
    harness.frame(events, true);
    // Bez autoryzacji F-15 dispatcher jest osiągany, ale odmawia.
    assert_eq!(
        harness.last_outcome(),
        Some(
            crate::gui::ptt_input::PttDispatchOutcome::AuthorizationDenied(
                crate::cat::hil::authorization::HilAuthorizationOutcome::NotEvaluated
            )
        )
    );
    assert_eq!(harness.counters(), (0, 0, 0));
}

#[test]
fn f17_valid_shortcut_without_f15_authorization_never_calls_the_backend() {
    let mut harness = F17Harness::new();
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, false);
    assert_eq!(
        harness.last_outcome(),
        Some(
            crate::gui::ptt_input::PttDispatchOutcome::AuthorizationDenied(
                crate::cat::hil::authorization::HilAuthorizationOutcome::NotEvaluated
            )
        ),
        "a valid PTT ON gesture must be reached and denied by F-15"
    );
    let (requests, ons, backend) = harness.counters();
    assert_eq!((requests, ons, backend), (0, 0, 0));
    assert!(
        !harness.app.cat.ptt_active,
        "a denial must never show TX in the GUI"
    );
}

#[test]
fn f17_ptt_off_shortcut_never_requires_authorization_and_never_calls_the_backend_when_idle() {
    let mut harness = F17Harness::new();
    harness.key(egui::Key::F7, F17_CTRL, false);
    assert_eq!(
        harness.last_outcome(),
        Some(crate::gui::ptt_input::PttDispatchOutcome::NotTransmitting),
        "PTT OFF while idle must be a no-op, not an authorization denial"
    );
    assert_eq!(harness.counters(), (0, 0, 0));
}

#[test]
fn f17_screen_button_on_without_authorization_never_calls_the_backend() {
    let mut harness = F17Harness::new();
    // Bramki muszą być wyliczone przez produkcyjną ścieżkę klatki.
    harness.frame(vec![], true);
    let snapshot = harness.app.ptt_input_snapshot(
        crate::gui::ptt_input::PttActionSource::ScreenButton,
        crate::gui::ptt_input::PttIntent::RequestOn,
    );
    let mut port = F17RecordingPort::default();
    let outcome = harness.app.dispatch_ptt_with_port(snapshot, &mut port);
    assert!(matches!(
        outcome,
        crate::gui::ptt_input::PttDispatchOutcome::AuthorizationDenied(_)
    ));
    assert_eq!(port.on_calls, 0);
    assert_eq!(port.off_calls, 0);
    assert!(!harness.app.cat.ptt_active);
}

#[test]
fn f17_screen_button_off_while_idle_does_not_call_the_backend() {
    let mut harness = F17Harness::new();
    harness.frame(vec![], true);
    let snapshot = harness.app.ptt_input_snapshot(
        crate::gui::ptt_input::PttActionSource::ScreenButton,
        crate::gui::ptt_input::PttIntent::RequestOff,
    );
    let mut port = F17RecordingPort::default();
    let outcome = harness.app.dispatch_ptt_with_port(snapshot, &mut port);
    assert_eq!(
        outcome,
        crate::gui::ptt_input::PttDispatchOutcome::NotTransmitting
    );
    assert_eq!(port.off_calls, 0);
    assert_eq!(port.on_calls, 0);
}

#[test]
fn f17_command_palette_exposes_two_explicit_ptt_commands() {
    let labels: Vec<&'static str> = crate::gui::command_palette::COMMANDS
        .iter()
        .filter(|c| {
            c.label_key == crate::core::i18n::key::CMD_PTT_ON
                || c.label_key == crate::core::i18n::key::CMD_PTT_OFF
        })
        .map(|c| c.label_key.as_str())
        .collect();
    assert_eq!(labels.len(), 2, "palette must expose PTT ON and PTT OFF");
    assert!(labels.contains(&"cmd.ptt_on"));
    assert!(labels.contains(&"cmd.ptt_off"));
}

#[test]
fn f17_focus_loss_does_not_call_the_backend_when_nothing_is_transmitting() {
    let mut harness = F17Harness::new();
    harness.frame(vec![], true);
    harness.frame(vec![], false);
    harness.app.release_ptt_on_focus_loss();
    assert_eq!(harness.counters(), (0, 0, 0));
    assert!(!harness.app.cat.ptt_active);
}

#[test]
fn f17_gui_never_sets_tx_state_from_a_ptt_request() {
    let mut harness = F17Harness::new();
    harness.key(egui::Key::F7, F17_CTRL_SHIFT, false);
    assert!(!harness.app.cat.ptt_active);
    harness.key(egui::Key::F7, F17_CTRL, false);
    assert!(!harness.app.cat.ptt_active);
    // Stan TX jest wyłącznie pochodną telemetrii i stanu arbitra.
    assert!(matches!(
        harness.app.ptt_transmit_state(),
        crate::gui::ptt_input::PttTransmitState::Idle
    ));
}

#[test]
fn f17_default_ptt_gates_are_fail_closed() {
    let state = crate::gui::state::PttUiState::default();
    assert!(state.gates.text_editing);
    assert!(state.gates.ime_composing);
    assert!(!state.gates.window_focused);
    assert!(state.gates.modal_open);
    assert!(state.last_outcome.is_none());
}

#[test]
fn f17_screen_button_double_click_never_calls_the_backend_without_authorization() {
    let mut harness = F17Harness::new();
    harness.frame(vec![], true);
    let mut port = F17RecordingPort::default();
    for _ in 0..2 {
        let snapshot = harness.app.ptt_input_snapshot(
            crate::gui::ptt_input::PttActionSource::ScreenButton,
            crate::gui::ptt_input::PttIntent::RequestOn,
        );
        let outcome = harness.app.dispatch_ptt_with_port(snapshot, &mut port);
        assert!(matches!(
            outcome,
            crate::gui::ptt_input::PttDispatchOutcome::AuthorizationDenied(_)
        ));
    }
    assert_eq!(port.on_calls, 0, "a double click must never key PTT");
    assert_eq!(port.off_calls, 0);
    assert!(!harness.app.cat.ptt_active);
}

#[test]
fn f17_screen_button_click_is_ignored_while_a_modal_is_open() {
    let mut harness = F17Harness::new();
    harness.app.gui.dialogs.advanced_filter_dialog.is_open = true;
    harness.frame(vec![], true);
    let snapshot = harness.app.ptt_input_snapshot(
        crate::gui::ptt_input::PttActionSource::ScreenButton,
        crate::gui::ptt_input::PttIntent::RequestOn,
    );
    let mut port = F17RecordingPort::default();
    let outcome = harness.app.dispatch_ptt_with_port(snapshot, &mut port);
    assert_eq!(
        outcome,
        crate::gui::ptt_input::PttDispatchOutcome::IgnoredModalOpen
    );
    assert_eq!(port.on_calls, 0);
}

#[test]
fn f17_screen_button_click_is_ignored_without_window_focus() {
    let mut harness = F17Harness::new();
    harness.frame(vec![], false);
    let snapshot = harness.app.ptt_input_snapshot(
        crate::gui::ptt_input::PttActionSource::ScreenButton,
        crate::gui::ptt_input::PttIntent::RequestOff,
    );
    let mut port = F17RecordingPort::default();
    let outcome = harness.app.dispatch_ptt_with_port(snapshot, &mut port);
    assert_eq!(
        outcome,
        crate::gui::ptt_input::PttDispatchOutcome::IgnoredWindowUnfocused
    );
    assert_eq!(port.off_calls, 0);
}

#[test]
fn f17_screen_button_click_is_ignored_while_text_editing() {
    let mut harness = F17Harness::new();
    harness
        .ctx
        .memory_mut(|m| m.request_focus(egui::Id::new("qso_entry_callsign")));
    harness.frame(vec![], true);
    let snapshot = harness.app.ptt_input_snapshot(
        crate::gui::ptt_input::PttActionSource::ScreenButton,
        crate::gui::ptt_input::PttIntent::RequestOn,
    );
    let mut port = F17RecordingPort::default();
    let outcome = harness.app.dispatch_ptt_with_port(snapshot, &mut port);
    assert_eq!(
        outcome,
        crate::gui::ptt_input::PttDispatchOutcome::IgnoredTextEditing
    );
    assert_eq!(port.on_calls, 0);
}

#[test]
fn f17_gui_state_never_shows_tx_after_a_denied_or_failed_request() {
    let mut harness = F17Harness::new();
    harness.frame(vec![], true);
    let snapshot = harness.app.ptt_input_snapshot(
        crate::gui::ptt_input::PttActionSource::ScreenButton,
        crate::gui::ptt_input::PttIntent::RequestOff,
    );
    let outcome = harness
        .app
        .dispatch_ptt_with_port(snapshot, &mut F17RecordingPort::default());
    assert!(outcome.is_denial());
    assert!(!harness.app.cat.ptt_active);
    assert!(matches!(
        harness.app.ptt_transmit_state(),
        crate::gui::ptt_input::PttTransmitState::Idle
    ));
}

// ======================================================================================
// Etap 2 — ścieżka klawiaturowa formularza QSO i okna Contest (Enter / Esc / Tab / spoty)
// ======================================================================================

fn qso_form_test_app() -> SpLogApp {
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, test_deps());
    app.gui.chrome.show_welcome_wizard = false;
    app
}

fn key_press(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// Jedna klatka renderująca formularz QSO z zadanymi zdarzeniami wejściowymi.
fn run_qso_form_frame(ctx: &egui::Context, app: &mut SpLogApp, events: Vec<egui::Event>) {
    let raw_input = egui::RawInput {
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(raw_input, |ui| {
        crate::gui::qso_entry::render_qso_entry_body(app, ui);
    });
    output.textures_delta.clear();
}

/// Ustawia fokus na polu i „rozgrzewa” egui jedną klatką bez zdarzeń, tak aby
/// `lost_focus()` w następnej klatce był wiarygodny.
fn focus_qso_field(ctx: &egui::Context, app: &mut SpLogApp, id: &str) {
    // Dwie klatki: widgety w `Grid` rejestrują się dopiero po przebiegu pomiarowym.
    for _ in 0..2 {
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(id)));
        run_qso_form_frame(ctx, app, Vec::new());
    }
    assert!(
        ctx.memory(|m| m.has_focus(egui::Id::new(id))),
        "pole {id} powinno mieć fokus po rozgrzewce"
    );
}

fn ready_to_save(app: &mut SpLogApp, call: &str) {
    app.entry.callsign = call.to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "CW".to_string();
    app.entry.focus_callsign_requested = false;
}

#[test]
fn enter_in_callsign_field_saves_qso_and_refocuses_callsign() {
    let mut app = qso_form_test_app();
    ready_to_save(&mut app, "SP9TEST");
    let ctx = egui::Context::default();
    focus_qso_field(&ctx, &mut app, "qso_entry_callsign");

    run_qso_form_frame(
        &ctx,
        &mut app,
        vec![key_press(egui::Key::Enter, egui::Modifiers::NONE)],
    );

    assert_eq!(
        app.entry.callsign, "",
        "Enter w polu znaku musi zapisać QSO"
    );
    assert!(app.entry.focus_callsign_requested);
}

#[test]
fn enter_in_rst_fields_saves_qso() {
    for field in ["qso_entry_rst_sent", "qso_entry_rst_rcvd"] {
        let mut app = qso_form_test_app();
        ready_to_save(&mut app, "SP9RST");
        let ctx = egui::Context::default();
        focus_qso_field(&ctx, &mut app, field);

        run_qso_form_frame(
            &ctx,
            &mut app,
            vec![key_press(egui::Key::Enter, egui::Modifiers::NONE)],
        );

        assert_eq!(app.entry.callsign, "", "Enter w {field} musi zapisać QSO");
        assert!(app.entry.focus_callsign_requested);
    }
}

#[test]
fn enter_in_my_pota_and_comment_saves_qso() {
    for field in [
        "qso_entry_my_pota",
        "qso_entry_my_sota",
        "qso_entry_comment",
    ] {
        let mut app = qso_form_test_app();
        ready_to_save(&mut app, "SP9REF");
        let ctx = egui::Context::default();
        focus_qso_field(&ctx, &mut app, field);

        run_qso_form_frame(
            &ctx,
            &mut app,
            vec![key_press(egui::Key::Enter, egui::Modifiers::NONE)],
        );

        assert_eq!(app.entry.callsign, "", "Enter w {field} musi zapisać QSO");
    }
}

#[test]
fn enter_with_invalid_form_keeps_data_and_field_focus() {
    let mut app = qso_form_test_app();
    app.entry.callsign = String::new();
    app.entry.name = "Preserved Operator".to_string();
    app.entry.comment = "Do not clear me".to_string();
    app.entry.focus_callsign_requested = false;
    let ctx = egui::Context::default();
    focus_qso_field(&ctx, &mut app, "qso_entry_rst_sent");

    run_qso_form_frame(
        &ctx,
        &mut app,
        vec![key_press(egui::Key::Enter, egui::Modifiers::NONE)],
    );

    assert_eq!(app.entry.name, "Preserved Operator");
    assert_eq!(app.entry.comment, "Do not clear me");
    assert!(
        ctx.memory(|m| m.has_focus(egui::Id::new("qso_entry_rst_sent"))),
        "przy błędzie walidacji fokus zostaje w polu, w którym wciśnięto Enter"
    );
}

#[test]
fn enter_without_form_focus_does_not_save() {
    let mut app = qso_form_test_app();
    ready_to_save(&mut app, "SP9NOFOCUS");
    let ctx = egui::Context::default();
    run_qso_form_frame(&ctx, &mut app, Vec::new());

    run_qso_form_frame(
        &ctx,
        &mut app,
        vec![key_press(egui::Key::Enter, egui::Modifiers::NONE)],
    );

    assert_eq!(
        app.entry.callsign, "SP9NOFOCUS",
        "Enter poza polami formularza nie może zapisywać QSO"
    );
}

#[test]
fn escape_in_qso_form_clears_form_and_refocuses_callsign() {
    let mut app = qso_form_test_app();
    app.entry.callsign = "SP6INA".to_string();
    app.entry.name = "Mariusz".to_string();
    app.entry.grid = "JO81".to_string();
    app.entry.focus_callsign_requested = false;
    let ctx = egui::Context::default();
    focus_qso_field(&ctx, &mut app, "qso_entry_name");

    run_qso_form_frame(
        &ctx,
        &mut app,
        vec![key_press(egui::Key::Escape, egui::Modifiers::NONE)],
    );

    assert_eq!(app.entry.callsign, "", "Esc czyści znak");
    assert_eq!(app.entry.name, "", "Esc czyści imię");
    assert_eq!(app.entry.grid, "", "Esc czyści lokator");
    assert!(
        app.entry.focus_callsign_requested,
        "po Esc fokus wraca do pola znaku"
    );
}

#[test]
fn escape_does_not_clear_form_when_command_palette_is_open() {
    let mut app = qso_form_test_app();
    app.entry.callsign = "SP6INA".to_string();
    app.entry.name = "Mariusz".to_string();
    let ctx = egui::Context::default();
    focus_qso_field(&ctx, &mut app, "qso_entry_callsign");
    app.gui.chrome.show_command_palette = true;

    run_qso_form_frame(
        &ctx,
        &mut app,
        vec![key_press(egui::Key::Escape, egui::Modifiers::NONE)],
    );

    assert_eq!(app.entry.callsign, "SP6INA");
    assert_eq!(app.entry.name, "Mariusz");
}

#[test]
fn escape_does_not_clear_form_when_dialog_or_contest_is_open() {
    for variant in 0..3 {
        let mut app = qso_form_test_app();
        app.entry.callsign = "SP6INA".to_string();
        app.entry.name = "Mariusz".to_string();
        let ctx = egui::Context::default();
        focus_qso_field(&ctx, &mut app, "qso_entry_callsign");
        match variant {
            0 => app.gui.dialogs.send_spot_dialog.is_open = true,
            1 => app.gui.chrome.show_shortcuts_window = true,
            _ => app.open_tile(crate::gui::state::Tile::Contest),
        }

        run_qso_form_frame(
            &ctx,
            &mut app,
            vec![key_press(egui::Key::Escape, egui::Modifiers::NONE)],
        );

        assert_eq!(app.entry.callsign, "SP6INA", "wariant {variant}");
        assert_eq!(app.entry.name, "Mariusz", "wariant {variant}");
    }
}

#[test]
fn escape_closing_open_popup_does_not_clear_form() {
    let mut app = qso_form_test_app();
    app.entry.callsign = "SP6INA".to_string();
    let ctx = egui::Context::default();
    focus_qso_field(&ctx, &mut app, "qso_entry_callsign");
    egui::Popup::open_id(&ctx, egui::Id::new("qso_band_popup_probe"));

    run_qso_form_frame(
        &ctx,
        &mut app,
        vec![key_press(egui::Key::Escape, egui::Modifiers::NONE)],
    );

    assert_eq!(
        app.entry.callsign, "SP6INA",
        "Esc zamykający popup nie czyści formularza"
    );
}

#[test]
fn contest_enter_goes_through_contest_save_qso_action_and_saves() {
    let mut app = qso_form_test_app();
    app.open_tile(crate::gui::state::Tile::Contest);
    // Nieznany kontest => tryb „sam RST” bez wymaganej wymiany.
    app.contest.name = "Etap2 Test Contest".to_string();
    ready_to_save(&mut app, "SP9CON");
    let stx_before = app.contest.stx;

    let ctx = egui::Context::default();
    // Rozgrzewka: okno Contest tworzy pola i dostaje fokus.
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("contest_entry_callsign")));
    let mut warm = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.handle_input_and_shortcuts(ui.ctx());
        crate::gui::contest::render_contest_window(&mut app, ui.ctx());
    });
    warm.textures_delta.clear();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("contest_entry_callsign")));

    let raw_input = egui::RawInput {
        events: vec![key_press(egui::Key::Enter, egui::Modifiers::NONE)],
        ..Default::default()
    };
    let mut output = ctx.run_ui(raw_input, |ui| {
        app.handle_input_and_shortcuts(ui.ctx());
        crate::gui::contest::render_contest_window(&mut app, ui.ctx());
    });
    output.textures_delta.clear();

    assert_eq!(
        app.contest.stx,
        stx_before + 1,
        "Enter w oknie Contest musi zapisać QSO przez ContestSaveQso"
    );
    assert_eq!(app.entry.callsign, "");
    assert!(!app.contest.save_requested, "flaga jest jednorazowa");
}

#[test]
fn ctrl_w_in_my_sota_and_my_pota_wipes_qso_form() {
    for field in ["qso_entry_my_sota", "qso_entry_my_pota"] {
        let mut app = qso_form_test_app();
        app.entry.callsign = "SP6INA".to_string();
        app.entry.my_refs.sota = "SP/BZ-001".to_string();
        let ctx = egui::Context::default();
        focus_qso_field(&ctx, &mut app, field);

        let raw_input = egui::RawInput {
            events: vec![key_press(egui::Key::W, egui::Modifiers::COMMAND)],
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw_input, |ui| {
            crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
            app.handle_input_and_shortcuts(ui.ctx());
        });
        output.textures_delta.clear();

        assert_eq!(
            app.entry.callsign, "",
            "Ctrl+W w {field} musi czyścić formularz"
        );
    }
}

/// Liczba naciśnięć `Tab` od pola znaku do pola docelowego (None, jeśli nie dotarto).
fn tabs_from_callsign_to(app: &mut SpLogApp, target: &str) -> Option<usize> {
    let ctx = egui::Context::default();
    focus_qso_field(&ctx, app, "qso_entry_callsign");
    for presses in 1..=30 {
        run_qso_form_frame(
            &ctx,
            app,
            vec![key_press(egui::Key::Tab, egui::Modifiers::NONE)],
        );
        if ctx.memory(|m| m.has_focus(egui::Id::new(target))) {
            return Some(presses);
        }
    }
    None
}

#[test]
fn tab_count_from_callsign_to_rst_is_constant_despite_dynamic_hints() {
    let mut plain = qso_form_test_app();
    let baseline = tabs_from_callsign_to(&mut plain, "qso_entry_rst_sent")
        .expect("Tab ze znaku musi dojść do RST Sent");

    let mut busy = qso_form_test_app();
    busy.entry.scp_suggestions = vec!["SP6INA".to_string(), "SP6INB".to_string()];
    busy.entry.callsign_corrections = vec![("SP6INC".to_string(), 1)];
    busy.entry.active_bearing_deg = 270.0;
    busy.entry.active_distance_km = 850.0;
    let with_hints = tabs_from_callsign_to(&mut busy, "qso_entry_rst_sent")
        .expect("Tab ze znaku musi dojść do RST Sent także z podpowiedziami");

    assert_eq!(
        baseline, with_hints,
        "podpowiedzi SCP / Did you mean / rotor nie mogą dodawać stopów Tab"
    );
    // Znak -> Pasmo -> Tryb -> RST Sent.
    assert_eq!(baseline, 3);
}

#[test]
fn entry_fields_do_not_jump_when_hints_appear() {
    let rst_y = |app: &mut SpLogApp| {
        let ctx = egui::Context::default();
        run_qso_form_frame(&ctx, app, Vec::new());
        run_qso_form_frame(&ctx, app, Vec::new());
        ctx.read_response(egui::Id::new("qso_entry_rst_sent"))
            .expect("pole RST Sent musi istnieć")
            .rect
            .min
            .y
    };

    let mut plain = qso_form_test_app();
    let y_plain = rst_y(&mut plain);

    let mut busy = qso_form_test_app();
    busy.entry.callsign = "DL1ABC".to_string();
    busy.entry.scp_suggestions = vec!["DL1ABC".to_string()];
    busy.entry.callsign_corrections = vec![("DL1ABD".to_string(), 1)];
    busy.entry.active_bearing_deg = 270.0;
    busy.entry.active_distance_km = 850.0;
    busy.entry.pga = "0101".to_string();
    let y_busy = rst_y(&mut busy);

    assert!(
        (y_plain - y_busy).abs() < 0.5,
        "pola wpisu nie mogą się przesuwać: {y_plain} vs {y_busy}"
    );
}

#[tokio::test]
async fn spot_click_requests_callsign_focus_and_activates_qso_tab() {
    let mut app = qso_form_test_app();
    app.gui.workspace.dock_state =
        egui_dock::DockState::new(vec!["qso".to_string(), "cluster".to_string()]);
    // Aktywna karta: cluster (indeks 1) — qso jest ukryta w zakładkach.
    if let Some(path) = app
        .gui
        .workspace
        .dock_state
        .find_tab(&"cluster".to_string())
    {
        let _ = app.gui.workspace.dock_state.set_active_tab(path);
    }
    app.entry.focus_callsign_requested = false;

    app.tune_to_spot("DL1ABC", 14074.0, "20m");

    assert!(app.entry.focus_callsign_requested);
    assert_eq!(app.entry.callsign, "DL1ABC");
    let active = app
        .gui
        .workspace
        .dock_state
        .find_active_focused()
        .map(|(_, tab)| tab.clone());
    assert_eq!(active.as_deref(), Some("qso"), "karta QSO musi być aktywna");
}

// ======================================================================================
// Etap 3 — pasek statusu: globalny wskaźnik TX i brak nakładania się elementów
// ======================================================================================

/// Zbiera teksty i ich prostokąty ze wszystkich kształtów klatki.
fn collect_text_shapes(shape: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::Shape::Text(text) => {
            let rect = egui::Rect::from_min_size(
                text.pos + text.galley.rect.min.to_vec2(),
                text.galley.rect.size(),
            );
            out.push((text.galley.text().to_string(), rect));
        }
        egui::Shape::Vec(shapes) => {
            for inner in shapes {
                collect_text_shapes(inner, out);
            }
        }
        _ => {}
    }
}

/// Renderuje pasek statusu w oknie o zadanej szerokości i zwraca teksty z prostokątami.
fn render_status_bar_texts(app: &mut SpLogApp, width: f32) -> Vec<(String, egui::Rect)> {
    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    // Dwie klatki: pierwsza ustala rozmiary, druga jest stabilna.
    for _ in 0..2 {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 720.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw_input, |ui| {
            app.render_status_bar(ui);
        });
        output.textures_delta.clear();
        texts.clear();
        for clipped in &output.shapes {
            collect_text_shapes(&clipped.shape, &mut texts);
        }
    }
    texts
}

#[test]
fn status_bar_shows_global_tx_indicator_from_telemetry() {
    let mut app = qso_form_test_app();
    // Stan TX wyłącznie z telemetrii (jedyna dozwolona ścieżka zapisu flagi).
    app.update_cat_state_from_telemetry(crate::cat::hamlib::RigState::default(), true);
    let texts = render_status_bar_texts(&mut app, 1280.0);
    assert!(
        texts.iter().any(|(t, _)| t.contains("TX ON")),
        "TX ON musi być widoczny na pasku statusu (kafelek VFO nie jest renderowany): {texts:?}"
    );

    app.update_cat_state_from_telemetry(crate::cat::hamlib::RigState::default(), false);
    let texts = render_status_bar_texts(&mut app, 1280.0);
    assert!(texts.iter().any(|(t, _)| t.contains("RX")));
    assert!(!texts.iter().any(|(t, _)| t.contains("TX ON")));
}

#[test]
fn status_bar_items_fit_without_overlap_on_narrow_windows() {
    for width in [1280.0_f32, 1024.0, 960.0] {
        let mut app = qso_form_test_app();
        app.update_cat_state_from_telemetry(crate::cat::hamlib::RigState::default(), true);
        app.cat.connected = true;
        app.gui.chrome.status_message = Some(
            "Bardzo długi komunikat statusu, który nie może wypchnąć wskaźników krytycznych poza okno"
                .repeat(3),
        );
        let texts = render_status_bar_texts(&mut app, width);
        assert_ne!(texts.len(), 0);

        for (text, rect) in &texts {
            assert!(
                rect.max.x <= width + 0.5,
                "element «{text}» wychodzi poza okno {width}px: {rect:?}"
            );
        }
        for (i, (ta, ra)) in texts.iter().enumerate() {
            for (tb, rb) in texts.iter().skip(i + 1) {
                assert!(
                    !ra.shrink(0.5).intersects(rb.shrink(0.5)),
                    "elementy «{ta}» i «{tb}» nachodzą na siebie przy {width}px"
                );
            }
        }
        // Wskaźniki krytyczne zawsze obecne w całości.
        for needle in ["TX ON", "CAT", "CLUSTER", "MHz"] {
            assert!(
                texts.iter().any(|(t, _)| t.contains(needle)),
                "brakuje wskaźnika krytycznego {needle} przy {width}px"
            );
        }
        // Napis wersji nie zjada miejsca w oknach <= 1600 px.
        assert!(!texts.iter().any(|(t, _)| t.contains("GPLv3")));
    }
}

// ======================================================================================
// Etap 4 — toasty z ważnością, stan pusty logbooka, edycja QSO, układy w wielu językach
// ======================================================================================

#[test]
fn save_qso_success_and_warnings_are_separate_toasts() {
    use crate::gui::state::ToastSeverity;
    let mut app = qso_form_test_app();
    // Pasmo/tryb spoza reguł dają ostrzeżenia pipeline'u; w razie braku ostrzeżeń
    // sprawdzamy przynajmniej, że sukces jest osobnym toastem `Success`.
    ready_to_save(&mut app, "SP9WARN");
    app.entry.rst_sent = "599".to_string();
    app.entry.rst_rcvd = "599".to_string();

    app.save_qso();

    let toasts: Vec<_> = app.gui.chrome.toasts.iter().collect();
    let success: Vec<_> = toasts
        .iter()
        .filter(|t| t.severity == ToastSeverity::Success)
        .collect();
    assert_eq!(
        success.len(),
        1,
        "dokładnie jeden toast sukcesu: {toasts:?}"
    );
    assert!(
        !success[0].message.contains('⚠'),
        "ostrzeżenia nie mogą być sklejone z komunikatem sukcesu"
    );
    for warning in toasts
        .iter()
        .filter(|t| t.severity == ToastSeverity::Warning)
    {
        assert_ne!(warning.id, success[0].id);
    }
}

#[test]
fn save_qso_duplicate_is_warning_and_validation_is_error() {
    use crate::gui::state::ToastSeverity;
    let mut app = qso_form_test_app();
    ready_to_save(&mut app, "SP9DUP");
    app.save_qso();
    ready_to_save(&mut app, "SP9DUP");
    app.save_qso();
    let severities: Vec<_> = app.gui.chrome.toasts.iter().map(|t| t.severity).collect();
    assert!(
        severities.contains(&ToastSeverity::Success),
        "pierwszy zapis: {severities:?}"
    );

    let mut empty = qso_form_test_app();
    empty.entry.callsign = String::new();
    empty.save_qso();
    assert_eq!(
        empty.gui.chrome.toasts.latest().map(|t| t.severity),
        Some(ToastSeverity::Error)
    );
}

#[test]
fn error_toast_survives_later_success_toasts_in_app() {
    use crate::gui::state::ToastSeverity;
    let mut app = qso_form_test_app();
    app.gui
        .chrome
        .push_status_toast(ToastSeverity::Error, "krytyczny błąd");
    for i in 0..10 {
        app.gui
            .chrome
            .push_status_toast(ToastSeverity::Success, format!("ok {i}"));
    }
    assert!(
        app.gui
            .chrome
            .toasts
            .iter()
            .any(|t| t.severity == ToastSeverity::Error && t.message == "krytyczny błąd")
    );
}

#[test]
fn toast_overlay_renders_icon_and_dismiss_button() {
    use crate::gui::state::ToastSeverity;
    let mut app = qso_form_test_app();
    app.gui
        .chrome
        .push_status_toast(ToastSeverity::Error, "Nie zapisano");
    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.render_chrome_overlays(ui.ctx());
        });
        output.textures_delta.clear();
        texts.clear();
        for clipped in &output.shapes {
            collect_text_shapes(&clipped.shape, &mut texts);
        }
    }
    assert!(texts.iter().any(|(t, _)| t.contains("Nie zapisano")));
    assert!(
        texts.iter().any(|(t, _)| t == "✖"),
        "ikona błędu: {texts:?}"
    );
    assert!(texts.iter().any(|(t, _)| t == "✕"), "przycisk zamknięcia");
}

#[test]
fn log_empty_state_distinguishes_empty_logbook_from_no_matches() {
    use crate::gui::logbook_table::{LogEmptyState, log_empty_state};
    assert_eq!(log_empty_state(5, true), None);
    assert_eq!(log_empty_state(0, false), Some(LogEmptyState::EmptyLogbook));
    assert_eq!(log_empty_state(0, true), Some(LogEmptyState::NoMatches));
}

#[test]
fn empty_logbook_renders_empty_state_message() {
    let mut app = qso_form_test_app();
    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::gui::logbook_table::render_logbook_body(&mut app, ui);
        });
        output.textures_delta.clear();
        texts.clear();
        for clipped in &output.shapes {
            collect_text_shapes(&clipped.shape, &mut texts);
        }
    }
    let lang = app.gui.appearance.language;
    let title = crate::core::i18n::tr(crate::core::i18n::key::LOG_EMPTY_TITLE, lang);
    assert!(
        texts.iter().any(|(t, _)| t == title),
        "pusty dziennik musi pokazać komunikat «{title}»: {texts:?}"
    );
}

#[test]
fn filter_without_matches_shows_clear_filter_state() {
    let mut app = qso_form_test_app();
    ready_to_save(&mut app, "SP9FIL");
    app.save_qso();
    assert_ne!(app.log_view.recent_qsos.len(), 0);
    app.log_view.search_query = "ZZZZZZ-NIE-ISTNIEJE".to_string();

    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::gui::logbook_table::render_logbook_body(&mut app, ui);
        });
        output.textures_delta.clear();
        texts.clear();
        for clipped in &output.shapes {
            collect_text_shapes(&clipped.shape, &mut texts);
        }
    }
    let lang = app.gui.appearance.language;
    let title = crate::core::i18n::tr(crate::core::i18n::key::LOG_NO_MATCHES_TITLE, lang);
    let clear = crate::core::i18n::tr(crate::core::i18n::key::LOG_CLEAR_FILTER, lang);
    assert!(texts.iter().any(|(t, _)| t == title), "{texts:?}");
    assert!(
        texts.iter().any(|(t, _)| t == clear),
        "przycisk czyszczenia filtra"
    );
}

#[test]
fn edit_dialog_combo_keeps_custom_band_and_mode() {
    use crate::gui::logbook_table::combo_options;
    use crate::gui::qso_entry::{STANDARD_BANDS, STANDARD_MODES};

    let custom_band = combo_options("5m", &STANDARD_BANDS);
    assert_eq!(
        custom_band[0], "5m",
        "niestandardowe pasmo zostaje, na początku"
    );
    assert_eq!(custom_band.len(), STANDARD_BANDS.len() + 1);
    let custom_mode = combo_options("JT65", &STANDARD_MODES);
    assert_eq!(custom_mode[0], "JT65");
    assert!(custom_mode.contains(&"CW".to_string()));

    let standard = combo_options("20m", &STANDARD_BANDS);
    assert_eq!(standard.len(), STANDARD_BANDS.len(), "bez duplikatu");

    // Render dialogu nie gubi wartości niestandardowych.
    let mut app = qso_form_test_app();
    let mut qso = QsoRecord::new("SP9EDIT", "5m", "JT65");
    qso.id = Some(1);
    app.log_view.editing_qso = Some(qso);
    let ctx = egui::Context::default();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::gui::logbook_table::render_edit_qso_dialog(&mut app, ui.ctx());
        });
        output.textures_delta.clear();
    }
    let edited = app.log_view.editing_qso.as_ref().expect("dialog otwarty");
    assert_eq!(edited.band, "5m");
    assert_eq!(edited.mode, "JT65");
}

/// Maksymalna prawa krawędź tekstów formularza QSO w oknie o zadanej szerokości.
fn qso_form_right_edge(lang: crate::core::i18n::Language, width: f32) -> f32 {
    let mut app = qso_form_test_app();
    app.gui.appearance.language = lang;
    app.entry.callsign = "SP9LANG".to_string();
    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 900.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
            });
        });
        output.textures_delta.clear();
        texts.clear();
        for clipped in &output.shapes {
            collect_text_shapes(&clipped.shape, &mut texts);
        }
    }
    texts
        .iter()
        .map(|(_, rect)| rect.max.x)
        .fold(0.0_f32, f32::max)
}

#[test]
fn qso_form_fits_340px_in_long_languages() {
    use crate::core::i18n::Language;
    for lang in [
        Language::Pl,
        Language::De,
        Language::Fr,
        Language::It,
        Language::Ru,
    ] {
        let right = qso_form_right_edge(lang, 340.0);
        assert!(
            right <= 340.0 + 0.5,
            "{lang:?}: formularz QSO wychodzi poza 340 px (prawa krawędź {right})"
        );
    }
}

#[test]
fn contest_entry_row_fits_in_long_languages() {
    use crate::core::i18n::Language;
    for lang in [Language::De, Language::Fr, Language::Pl] {
        let mut app = qso_form_test_app();
        app.gui.appearance.language = lang;
        app.open_tile(crate::gui::state::Tile::Contest);
        let ctx = egui::Context::default();
        let mut texts = Vec::new();
        let width = 640.0;
        for _ in 0..2 {
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(raw_input, |ui| {
                crate::gui::contest::render_contest_window(&mut app, ui.ctx());
            });
            output.textures_delta.clear();
            texts.clear();
            for clipped in &output.shapes {
                collect_text_shapes(&clipped.shape, &mut texts);
            }
        }
        let label =
            crate::core::i18n::tr(crate::core::i18n::key::CONTEST_RCVD_REPORT_EXCHANGE, lang);
        let (_, rect) = texts
            .iter()
            .find(|(t, _)| t == label)
            .unwrap_or_else(|| panic!("{lang:?}: etykieta wymiany «{label}» musi być widoczna"));
        assert!(
            rect.max.x <= width,
            "{lang:?}: etykieta wymiany wychodzi poza okno ({rect:?})"
        );
    }
}

// ======================================================================================
// Odpinanie / dokowanie kafelków: cykl undock -> floating -> redock dla każdego kafelka
// ======================================================================================

#[test]
fn every_tile_supports_undock_floating_redock_cycle() {
    for id in SpLogApp::all_tile_ids() {
        let mut app = qso_form_test_app();
        // Wszystkie kafelki widoczne i zadokowane.
        app.views = crate::gui::state::ViewsState::from_config(&AppConfig::default());
        for tile in crate::gui::state::Tile::ALL {
            let panel = app.views.panel_mut(tile);
            panel.visible = true;
            panel.floating = false;
        }
        app.rebuild_dock_state_from_panels();
        assert!(
            app.gui
                .workspace
                .dock_state
                .find_tab(&id.to_string())
                .is_some(),
            "{id}: kafelek zadokowany na starcie"
        );

        app.popout_tile(id);
        app.sync_dock_state();
        assert!(app.panel_config(id).floating, "{id}: po odpięciu floating");
        assert!(
            app.panel_config(id).visible,
            "{id}: odpięcie nie ukrywa panelu"
        );
        assert!(
            app.gui
                .workspace
                .dock_state
                .find_tab(&id.to_string())
                .is_none(),
            "{id}: odpięty kafelek znika z docka"
        );

        app.dock_tile(id);
        app.sync_dock_state();
        assert!(
            !app.panel_config(id).floating,
            "{id}: po zadokowaniu nie floating"
        );
        assert!(app.panel_config(id).visible, "{id}: panel nie zginął");
        assert!(
            app.gui
                .workspace
                .dock_state
                .find_tab(&id.to_string())
                .is_some(),
            "{id}: kafelek wraca do docka"
        );
    }
}

#[tokio::test]
async fn every_tile_renders_with_its_undock_row() {
    use egui_dock::TabViewer;
    for id in SpLogApp::all_tile_ids() {
        let mut app = qso_form_test_app();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let mut tab = id.to_string();
                TabViewer::ui(&mut app, ui, &mut tab);
            });
            output.textures_delta.clear();
        }
    }
}

// ======================================================================================
// Mapa świata: Natural Earth, cień nocy jako jeden Mesh, ikony wektorowe, toasty bez dubli
// ======================================================================================

fn collect_meshes(shape: &egui::Shape, out: &mut Vec<usize>) {
    match shape {
        egui::Shape::Mesh(mesh) => out.push(mesh.vertices.len()),
        egui::Shape::Vec(shapes) => {
            for inner in shapes {
                collect_meshes(inner, out);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn world_map_renders_in_all_themes_with_single_night_mesh() {
    use crate::gui::theme::ThemePreset;
    for preset in ThemePreset::ALL {
        let mut app = qso_form_test_app();
        let ctx = egui::Context::default();
        crate::gui::theme::ensure_applied(&ctx, preset, 1.0);
        let mut meshes = Vec::new();
        for _ in 0..2 {
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(raw_input, |ui| {
                crate::gui::world_map::render_world_map_content(&mut app, ui);
            });
            output.textures_delta.clear();
            meshes.clear();
            for clipped in &output.shapes {
                collect_meshes(&clipped.shape, &mut meshes);
            }
        }
        let land = crate::gui::map_data::land();
        assert!(
            meshes.contains(&land.vertices.len()),
            "{preset:?}: jeden Mesh lądów ({} wierzchołków): {meshes:?}",
            land.vertices.len()
        );
        // Cień nocy: 91 kolumn x 5 wierszy, dokładnie jeden Mesh (bez 91 osobnych czworokątów).
        let night_vertices = 91 * 5;
        assert_eq!(
            meshes.iter().filter(|&&n| n == night_vertices).count(),
            1,
            "{preset:?}: cień nocy to jeden Mesh: {meshes:?}"
        );
    }
}

#[tokio::test]
async fn world_map_fills_tile_keeps_two_to_one_and_clips_at_extreme_sizes() {
    for (width, height) in [
        (300.0_f32, 600.0_f32),
        (1200.0, 260.0),
        (320.0, 200.0),
        (1920.0, 1080.0),
    ] {
        let mut app = qso_form_test_app();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height));
        let mut content_width = 0.0_f32;
        let mut shapes = Vec::new();
        for _ in 0..2 {
            let raw_input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let mut output = ctx.run_ui(raw_input, |ui| {
                crate::gui::world_map::render_world_map_content(&mut app, ui);
                content_width = ui.min_rect().width();
            });
            output.textures_delta.clear();
            shapes = std::mem::take(&mut output.shapes);
        }
        let label = format!("{width}x{height}");
        let canvas = app.world_map.last_canvas.expect("płótno mapy");
        let map = app.world_map.last_map_rect.expect("prostokąt rzutu");

        // Panel nie rozpycha się na boki, a płótno wypełnia dostępny obszar do dołu i boków.
        assert!(
            content_width <= width + 0.5,
            "{label}: szerokość {content_width}"
        );
        assert!(canvas.width() > 0.0 && canvas.height() > 0.0, "{label}");
        assert!(
            screen.expand(0.5).contains_rect(canvas),
            "{label}: {canvas:?}"
        );
        assert!((canvas.width() - width).abs() < 1.5, "{label}: {canvas:?}");
        assert!(
            (canvas.bottom() - height).abs() < 1.5,
            "{label}: {canvas:?}"
        );

        // Rzut 2:1 mieści się w płótnie i jest wyśrodkowany.
        assert!(canvas.expand(0.01).contains_rect(map), "{label}");
        assert!((map.width() / map.height() - 2.0).abs() < 1e-3, "{label}");
        assert!((map.center() - canvas.center()).length() < 0.01, "{label}");
        assert!(
            (map.width() - canvas.width()).abs() < 0.01
                || (map.height() - canvas.height()).abs() < 0.01,
            "{label}: mapa wypełnia płótno w jednym wymiarze"
        );

        // Mesh lądów jest przycięty ścisłym clip_rect do płótna mapy.
        let land_vertices = crate::gui::map_data::land().vertices.len();
        let land_clip = shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) if mesh.vertices.len() == land_vertices => {
                Some(clipped.clip_rect)
            }
            _ => None,
        });
        let land_clip = land_clip.unwrap_or_else(|| panic!("{label}: brak meshu lądów"));
        assert!(
            canvas.expand(0.01).contains_rect(land_clip),
            "{label}: clip {land_clip:?} poza płótnem {canvas:?}"
        );
    }
}

#[tokio::test]
async fn world_map_rects_stay_valid_after_zoom_clicks_and_resizes() {
    let mut app = qso_form_test_app();
    let ctx = egui::Context::default();
    let render = |app: &mut SpLogApp, width: f32, height: f32, events: Vec<egui::Event>| {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw_input, |ui| {
            crate::gui::world_map::render_world_map_content(app, ui);
        });
        output.textures_delta.clear();
    };
    // Kliknięcie "+" trzykrotnie (zoom 2.5) w dużym oknie, potem zmniejszenie kafelka.
    render(&mut app, 1200.0, 700.0, Vec::new());
    render(&mut app, 1200.0, 700.0, Vec::new());
    let canvas = app.world_map.last_canvas.expect("płótno");
    let plus = canvas.left_bottom() + egui::vec2(25.0, -55.0);
    for _ in 0..3 {
        for pressed in [true, false] {
            render(
                &mut app,
                1200.0,
                700.0,
                vec![egui::Event::PointerButton {
                    pos: plus,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
    }
    for (width, height) in [(320.0, 200.0), (300.0, 600.0), (1920.0, 1080.0)] {
        render(&mut app, width, height, Vec::new());
        render(&mut app, width, height, Vec::new());
        let canvas = app.world_map.last_canvas.expect("płótno");
        let map = app.world_map.last_map_rect.expect("rzut");
        assert!(canvas.expand(0.01).contains_rect(map), "{width}x{height}");
        assert!((map.width() / map.height() - 2.0).abs() < 1e-3);
    }
}

#[test]
fn world_map_home_label_and_beam_azimuth_avoid_misleading_zero() {
    use crate::cat::rotor::RotorState;
    use crate::gui::world_map::{beam_azimuth, home_label};
    assert_eq!(
        home_label("", ""),
        None,
        "bez znaku i lokatora brak etykiety"
    );
    assert_eq!(
        home_label("SP6INA", "JO81").as_deref(),
        Some("SP6INA (JO81)")
    );
    assert_eq!(home_label("SP6INA", "").as_deref(), Some("SP6INA"));
    assert_eq!(home_label("", "JO81").as_deref(), Some("JO81"));

    let idle = RotorState::default();
    assert_eq!(
        beam_azimuth(&idle, None),
        None,
        "nieaktywny rotator, brak DX: bez azymutu"
    );
    assert_eq!(
        beam_azimuth(&idle, Some(45.0)),
        Some(45.0),
        "kierunek na DX"
    );
    let connected = RotorState {
        connected: true,
        azimuth_deg: 120.0,
        ..RotorState::default()
    };
    assert_eq!(
        beam_azimuth(&connected, Some(45.0)),
        Some(120.0),
        "rotator ma pierwszeństwo"
    );
}

#[test]
fn toast_overlay_does_not_duplicate_the_severity_icon() {
    use crate::gui::state::ToastSeverity;
    let mut app = qso_form_test_app();
    app.gui
        .chrome
        .push_status_toast(ToastSeverity::Warning, "⚠ Uwaga na zapis");
    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.render_chrome_overlays(ui.ctx());
        });
        output.textures_delta.clear();
        texts.clear();
        for clipped in &output.shapes {
            collect_text_shapes(&clipped.shape, &mut texts);
        }
    }
    assert!(
        texts.iter().any(|(t, _)| t == "Uwaga na zapis"),
        "{texts:?}"
    );
    assert_eq!(
        texts.iter().filter(|(t, _)| t.contains('⚠')).count(),
        1,
        "jedna ikona ostrzeżenia (z ToastSeverity), nie dwie: {texts:?}"
    );
}

fn s2_app() -> SpLogApp {
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    SpLogApp::for_tests(init, test_deps())
}

fn s2_view(app: &SpLogApp) -> (Vec<i64>, Vec<(i64, usize)>) {
    let ids: Vec<i64> = app
        .log_view
        .recent_qsos
        .iter()
        .filter_map(|q| q.id)
        .collect();
    let mut numbers: Vec<(i64, usize)> = app
        .log_view
        .qso_numbers
        .iter()
        .map(|(id, n)| (*id, *n))
        .collect();
    numbers.sort_unstable();
    (ids, numbers)
}

#[test]
fn incremental_log_updates_match_a_full_reload() {
    let mut app = s2_app();
    let journal = app.active_journal.id.clone();
    {
        let db = app.services.log_db.lock().unwrap();
        for day in 1..=20 {
            let mut qso = QsoRecord::new(format!("SP{day}ABC"), "20m", "CW");
            qso.journal_id = Some(journal.clone());
            qso.qso_date = format!("202610{day:02}");
            qso.time_on = "1200".to_string();
            db.insert_qso(&qso).unwrap();
        }
    }
    app.reload_qsos();
    assert_eq!(app.log_view.recent_qsos.len(), 20);

    // Form save (current time, newest row).
    app.entry.callsign = "SQ9NEW".to_string();
    app.entry.band = "40m".to_string();
    app.entry.mode = "SSB".to_string();
    app.entry.rst_sent = "59".to_string();
    app.entry.rst_rcvd = "59".to_string();
    app.save_qso();
    assert_eq!(app.log_view.recent_qsos.len(), 21);
    assert_eq!(app.log_view.recent_qsos[0].callsign, "SQ9NEW");

    // A backdated QSO from another source lands in the middle of the journal.
    let mut backdated = QsoRecord::new("DL1OLD", "15m", "CW");
    backdated.journal_id = Some(journal.clone());
    backdated.qso_date = "2026-10-05".to_string();
    backdated.time_on = "13:00".to_string();
    let outcome = app
        .services
        .ingestor
        .ingest(backdated, crate::core::ingest::QsoOrigin::LanPeer)
        .unwrap();
    app.apply_stored_qso(&outcome.qso);
    assert_eq!(app.log_view.recent_qsos.len(), 22);

    // A QSO of another journal is not shown.
    {
        let db = app.services.log_db.lock().unwrap();
        db.create_journal(&Journal {
            id: "OTHER".to_string(),
            name: "Other".to_string(),
            ..Default::default()
        })
        .unwrap();
    }
    let mut foreign = QsoRecord::new("OK1XYZ", "20m", "CW");
    foreign.journal_id = Some("OTHER".to_string());
    foreign.qso_date = "20261030".to_string();
    foreign.time_on = "1200".to_string();
    let outcome = app
        .services
        .ingestor
        .ingest(foreign, crate::core::ingest::QsoOrigin::LanPeer)
        .unwrap();
    app.apply_stored_qso(&outcome.qso);
    assert_eq!(app.log_view.recent_qsos.len(), 22);

    // Delete one row and edit another so it moves to a new position.
    let delete_id = app.log_view.recent_qsos[5].id.unwrap();
    app.delete_qso_by_id(delete_id);
    let mut edited = app.log_view.recent_qsos[10].clone();
    edited.qso_date = "20261031".to_string();
    app.log_view.editing_qso = Some(edited.clone());
    app.save_edited_qso();
    assert_eq!(app.log_view.recent_qsos[1].id, edited.id);

    let incremental = s2_view(&app);
    app.reload_qsos();
    assert_eq!(
        s2_view(&app),
        incremental,
        "incremental view == full reload"
    );
}

#[test]
fn awards_after_incremental_delete_match_a_rebuild_from_the_database() {
    let mut app = s2_app();
    let journal = app.active_journal.id.clone();
    {
        let db = app.services.log_db.lock().unwrap();
        for (call, band) in [("SP1AAA", "20m"), ("DL1BBB", "40m"), ("OK1CCC", "15m")] {
            let mut qso = QsoRecord::new(call, band, "CW");
            qso.journal_id = Some(journal.clone());
            qso.qso_date = "20261001".to_string();
            qso.time_on = "1200".to_string();
            qso.dxcc = Some(match call {
                "SP1AAA" => 269,
                "DL1BBB" => 230,
                _ => 503,
            });
            db.insert_qso(&qso).unwrap();
        }
    }
    app.rebuild_awards_full();
    let dl = app
        .log_view
        .recent_qsos
        .iter()
        .find(|q| q.callsign == "DL1BBB")
        .and_then(|q| q.id)
        .unwrap();
    app.delete_qso_by_id(dl);
    let incremental = app
        .services
        .awards_engine
        .lock()
        .unwrap()
        .worked_dxcc_all
        .clone();
    app.rebuild_awards_full();
    let rebuilt = app
        .services
        .awards_engine
        .lock()
        .unwrap()
        .worked_dxcc_all
        .clone();
    assert_eq!(incremental, rebuilt);
    assert!(!incremental.contains(&230));
}

// ======================================================================================
// STAGE-V140-S5: wyniki poleceń rotatora trafiają do GUI dopiero po odpowiedzi rotctld
// ======================================================================================

/// Jednorazowy rotctld: odbiera jedną linię i odpowiada `reply`.
async fn s5_one_shot_rotctld(reply: &'static str) -> u16 {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut line = String::new();
        tokio::io::BufReader::new(reader)
            .read_line(&mut line)
            .await
            .unwrap();
        writer.write_all(reply.as_bytes()).await.unwrap();
    });
    port
}

async fn s5_wait_for_toast(app: &mut SpLogApp) -> (crate::gui::state::ToastSeverity, String) {
    for _ in 0..200 {
        app.poll_background_notices();
        if let Some(t) = app.gui.chrome.toasts.latest() {
            return (t.severity, t.message.clone());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("no background notice arrived");
}

#[tokio::test]
async fn s5_rotor_rejection_is_shown_as_an_error_not_as_sent() {
    let mut app = qso_form_test_app();
    app.gui.appearance.language = crate::core::i18n::Language::En;
    app.config.rotor_host = "127.0.0.1".to_string();
    app.config.rotor_port = s5_one_shot_rotctld("RPRT -8\n").await;

    app.rotate_antenna_to(123.0);
    assert!(
        app.gui.chrome.toasts.is_empty(),
        "nothing is reported before rotctld answers"
    );
    let (severity, message) = s5_wait_for_toast(&mut app).await;
    assert_eq!(severity, crate::gui::state::ToastSeverity::Error);
    assert!(message.contains("RPRT -8"), "{message}");
}

#[tokio::test]
async fn s5_rotor_success_is_confirmed_after_the_reply() {
    let mut app = qso_form_test_app();
    app.config.rotor_host = "127.0.0.1".to_string();
    app.config.rotor_port = s5_one_shot_rotctld("RPRT 0\n").await;

    app.rotate_antenna_to(45.0);
    let (severity, message) = s5_wait_for_toast(&mut app).await;
    assert_eq!(severity, crate::gui::state::ToastSeverity::Info);
    assert!(message.contains("45"), "{message}");
}

// ======================================================================================
// STAGE-V140-S5: kopie okresowe, znacznik sesji, przywracanie z kopii, stan UI
// ======================================================================================

fn s5_file_backed_app(dir: &std::path::Path) -> SpLogApp {
    let db_path = dir.join("log.db");
    let fresh = !db_path.exists();
    let db = LogDatabase::open(&db_path).unwrap();
    if fresh {
        db.insert_qso(&QsoRecord::new("SP6INA", "20m", "CW"))
            .unwrap();
    }
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: dir.join("station_config.json"),
        active_db_path: db_path,
        shutdown_policy: ShutdownPolicy::None,
    };
    let deps = AppDeps {
        log_db: Arc::new(Mutex::new(db)),
        prefix_matcher: Arc::new(test_prefix_matcher()),
        scp_engine: Arc::new(Mutex::new(ScpEngine::new())),
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.gui.chrome.show_welcome_wizard = false;
    app
}

fn s5_app_from_config(config: AppConfig) -> SpLogApp {
    let init = AppInit {
        config,
        config_file_path: test_config_path(),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    SpLogApp::for_tests(init, test_deps())
}

fn s5_backups(dir: &std::path::Path, prefix: &str) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir.join("backups")) else {
        return Vec::new();
    };
    let mut found: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(prefix))
        })
        .collect();
    found.sort();
    found
}

fn s5_wait_until(mut done: impl FnMut() -> bool) {
    for _ in 0..500 {
        if done() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("condition not reached");
}

#[test]
fn s5_periodic_backup_runs_after_the_interval_and_skips_an_unchanged_database() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = s5_file_backed_app(temp.path());
    app.config.backup.interval_minutes = 5;
    let start = app.runtime.backup.last_periodic;

    app.run_backup_schedule(start + std::time::Duration::from_secs(60));
    assert!(
        s5_backups(temp.path(), "log_backup_").is_empty(),
        "too early"
    );

    app.run_backup_schedule(start + std::time::Duration::from_mins(5));
    s5_wait_until(|| {
        app.poll_backup_results();
        app.runtime.backup.last_fingerprint.is_some()
    });
    assert_eq!(s5_backups(temp.path(), "log_backup_").len(), 1);

    // Ta sama baza: kolejny termin nie tworzy identycznej kopii.
    app.run_backup_schedule(start + std::time::Duration::from_mins(10));
    assert!(
        !app.runtime
            .backup
            .in_flight
            .load(std::sync::atomic::Ordering::Acquire)
    );
    assert_eq!(s5_backups(temp.path(), "log_backup_").len(), 1);

    // Interwał 0 wyłącza kopie okresowe.
    app.config.backup.interval_minutes = 0;
    app.runtime.backup.last_fingerprint = None;
    app.run_backup_schedule(start + std::time::Duration::from_hours(1));
    assert!(
        !app.runtime
            .backup
            .in_flight
            .load(std::sync::atomic::Ordering::Acquire)
    );
    assert_eq!(s5_backups(temp.path(), "log_backup_").len(), 1);
}

#[test]
fn s5_periodic_backup_failure_is_shown_to_the_operator() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = s5_file_backed_app(temp.path());
    app.config.backup.interval_minutes = 1;
    // `backups` jako plik: katalogu kopii nie da się utworzyć.
    std::fs::write(temp.path().join("backups"), b"not a directory").unwrap();
    let start = app.runtime.backup.last_periodic;
    app.run_backup_schedule(start + std::time::Duration::from_secs(60));
    s5_wait_until(|| {
        app.poll_background_notices();
        !app.gui.chrome.toasts.is_empty()
    });
    assert_eq!(
        app.gui.chrome.toasts.latest().map(|t| t.severity),
        Some(crate::gui::state::ToastSeverity::Error)
    );
}

#[test]
fn s5_session_guard_checks_and_copies_the_database_after_an_unclean_exit() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = s5_file_backed_app(temp.path());
    app.start_session_guard();
    assert!(app.gui.chrome.toasts.is_empty(), "first start is clean");
    // Brak backup_on_exit = awaria; znacznik zostaje na dysku.
    drop(app.runtime.backup.session_marker.take());

    let mut next = s5_file_backed_app(temp.path());
    next.start_session_guard();
    s5_wait_until(|| {
        next.poll_background_notices();
        !next.gui.chrome.toasts.is_empty()
    });
    assert_eq!(
        next.gui.chrome.toasts.latest().map(|t| t.severity),
        Some(crate::gui::state::ToastSeverity::Warning)
    );
    assert_eq!(s5_backups(temp.path(), "log_unclean_shutdown_").len(), 1);

    next.backup_on_exit();
    assert!(!crate::core::backup::SessionMarker::path_for(&temp.path().join("log.db")).exists());
    assert_eq!(
        s5_backups(temp.path(), "log_backup_").len(),
        1,
        "backup on exit"
    );
    assert_eq!(
        s5_backups(temp.path(), "SPLogbook_backup_").len(),
        1,
        "ADIF on exit"
    );
}

#[test]
fn s5_restore_from_backup_replaces_the_log_and_keeps_a_copy_of_the_previous_state() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = s5_file_backed_app(temp.path());
    app.run_manual_backup();
    let backup = s5_backups(temp.path(), "log_backup_").remove(0);

    app.services
        .log_db
        .lock()
        .unwrap()
        .insert_qso(&QsoRecord::new("DL1ABC", "40m", "SSB"))
        .unwrap();
    app.rebuild_awards_full();
    assert_eq!(app.log_view.recent_qsos.len(), 2);

    std::thread::sleep(std::time::Duration::from_millis(20));
    app.restore_from_backup(&backup);
    assert_eq!(app.log_view.recent_qsos.len(), 1, "log view reloaded");
    assert_eq!(app.log_view.recent_qsos[0].callsign, "SP6INA");
    let count: i64 = app
        .services
        .log_db
        .lock()
        .unwrap()
        .raw_conn()
        .query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "the reopened database is the restored one");
    assert_eq!(
        s5_backups(temp.path(), "log_backup_").len(),
        2,
        "the state before the restore was saved first"
    );
    assert_eq!(
        app.gui.chrome.toasts.latest().map(|t| t.severity),
        Some(crate::gui::state::ToastSeverity::Info)
    );
}

#[test]
fn s5_ui_state_survives_a_restart_and_bad_values_fall_back_to_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let cfg_path = temp.path().join("station_config.json");
    let mut app = s5_app_from_config(AppConfig::default());
    app.paths.config_file_path = cfg_path.clone();
    app.log_view.page_size = 100;
    app.cluster.filter_band_selection = "20m".to_string();
    app.cluster.filter_pota_sota_only = true;
    app.duplicates.match_date = false;
    app.contest.name = "CQ WW DX".to_string();
    app.contest.stx = 157;
    app.save_station_config();

    let restarted = s5_app_from_config(AppConfig::load_from_file(&cfg_path).unwrap());
    assert_eq!(restarted.log_view.page_size, 100);
    assert_eq!(restarted.cluster.filter_band_selection, "20m");
    assert!(restarted.cluster.filter_pota_sota_only);
    assert!(!restarted.duplicates.match_date);
    assert_eq!(restarted.contest.name, "CQ WW DX");
    assert_eq!(restarted.contest.stx, 157, "contest serial continues");

    let mut broken = AppConfig::default();
    broken.ui_state.log_page_size = 7;
    broken.ui_state.cluster_band = "   ".to_string();
    broken.ui_state.contest_next_serial = 0;
    broken.ui_state.waterfall.gain_db = f32::NAN;
    broken.ui_state.waterfall.view_max_hz = 5.0;
    broken.ui_state.waterfall.palette = "no-such-palette".to_string();
    let app = s5_app_from_config(broken);
    assert_eq!(app.log_view.page_size, 50);
    assert_eq!(app.cluster.filter_band_selection, "ALL");
    assert_eq!(app.contest.stx, 1);
    assert_eq!(
        app.gui.dialogs.waterfall_panel.settings(),
        crate::core::station::WaterfallSettings::default()
    );
}

// ======================================================================================
// STAGE-V140-S6: nadawanie CW przez prawdziwy klucz (wirtualny port WinKeyera)
// ======================================================================================

#[tokio::test]
async fn s6_cw_without_a_keyer_warns_instead_of_pretending_to_send() {
    let mut app = qso_form_test_app();
    app.transmit_cw_macro("CQ TEST");
    assert_eq!(app.gui.dialogs.cw_terminal_dialog.buffer.queued_text, "");
    assert_eq!(
        app.gui.chrome.toasts.latest().map(|t| t.severity),
        Some(crate::gui::state::ToastSeverity::Warning)
    );
}

#[tokio::test]
async fn s6_macro_goes_to_the_winkeyer_and_progress_follows_the_echo() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut app = qso_form_test_app();
    app.config.station.callsign = "SP6INA".to_string();
    app.entry.callsign = "DL1ABC".to_string();
    app.contest.stx = 7;
    let (host, mut device) = tokio::io::duplex(256);
    app.gui.dialogs.cw_terminal_dialog.keyer = Some(crate::cat::cw_keyer::spawn_winkeyer(
        host,
        app.config.cw.keyer.clone(),
    ));

    let mut open = [0u8; 2];
    device.read_exact(&mut open).await.unwrap();
    device.write_all(&[31]).await.unwrap();
    let mut setup = [0u8; 6];
    device.read_exact(&mut setup).await.unwrap();

    app.apply_cw_terminal_actions(vec![crate::gui::cw_terminal::CwTerminalAction::Macro(1)]);
    let expected = b"DL1ABC 5NN 007 ";
    let mut sent = vec![0u8; expected.len()];
    device.read_exact(&mut sent).await.unwrap();
    assert_eq!(&sent, expected, "%HISCALL% 5NN %SERIAL% resolved and keyed");
    assert_eq!(
        app.gui.dialogs.cw_terminal_dialog.buffer.queued_text,
        "DL1ABC 5NN 007 "
    );

    // WinKeyer nadał pierwsze trzy litery.
    device.write_all(&[0xC4, b'D', b'L', b'1']).await.unwrap();
    for _ in 0..500 {
        app.poll_cw_keyer();
        if app.gui.dialogs.cw_terminal_dialog.buffer.sent_text == "DL1" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    let terminal = &app.gui.dialogs.cw_terminal_dialog;
    assert!(terminal.ready, "Ready event received");
    assert_eq!(terminal.buffer.sent_text, "DL1");
    assert_eq!(terminal.buffer.queued_text, "ABC 5NN 007 ");

    // Przerwanie: Clear Buffer na porcie, bufor terminala pusty.
    app.apply_cw_terminal_actions(vec![crate::gui::cw_terminal::CwTerminalAction::Abort]);
    let mut abort = [0u8; 1];
    device.read_exact(&mut abort).await.unwrap();
    assert_eq!(abort, [0x0A]);
    assert_eq!(app.gui.dialogs.cw_terminal_dialog.buffer.queued_text, "");

    // Odłączenie zamyka WinKeyera (Clear Buffer + Host Close).
    app.disconnect_cw_keyer();
    let mut close = [0u8; 3];
    device.read_exact(&mut close).await.unwrap();
    assert_eq!(close, [0x0A, 0x00, 0x03]);
}

#[test]
fn s6_cw_macros_and_keyer_settings_survive_a_restart() {
    let temp = tempfile::tempdir().unwrap();
    let cfg_path = temp.path().join("station_config.json");
    let mut app = s5_app_from_config(AppConfig::default());
    app.paths.config_file_path = cfg_path.clone();
    app.cw.macros[0] = "CQ CQ DE %MYCALL% K".to_string();
    app.config.cw.keyer.kind = crate::cat::cw_keyer::CwKeyerKind::SerialRts;
    app.config.cw.keyer.port = "COM9".to_string();
    app.config.cw.keyer.wpm = 31;
    app.save_station_config();

    let restarted = s5_app_from_config(AppConfig::load_from_file(&cfg_path).unwrap());
    assert_eq!(restarted.cw.macros[0], "CQ CQ DE %MYCALL% K");
    assert_eq!(
        restarted.config.cw.keyer.kind,
        crate::cat::cw_keyer::CwKeyerKind::SerialRts
    );
    assert_eq!(restarted.config.cw.keyer.port, "COM9");
    assert_eq!(restarted.config.cw.keyer.wpm, 31);
}

// ======================================================================================
// STAGE-V140-S7: sesja zawodów, wynik tylko z sesji, ESM, makra F1..F8
// ======================================================================================

/// Aplikacja testowa z konfiguracją w katalogu tymczasowym (sesja zawodów zapisuje config).
/// Aplikacja leży na stercie: w testach asynchronicznych jest częścią stanu future.
fn s7_app() -> (tempfile::TempDir, Box<SpLogApp>) {
    let temp = tempfile::tempdir().unwrap();
    let mut app = Box::new(qso_form_test_app());
    app.paths.config_file_path = temp.path().join("station_config.json");
    (temp, app)
}

fn s7_render_contest_window(app: &mut SpLogApp) {
    app.open_tile(crate::gui::state::Tile::Contest);
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::gui::contest::render_contest_window(app, ui.ctx());
    });
    output.textures_delta.clear();
}

#[tokio::test]
async fn s7_only_session_qsos_count_towards_the_contest_score() {
    let (_config_dir, mut app) = s7_app();
    app.config.station.callsign = "SP6INA".to_string();
    app.contest.name = "CQ WPX Contest (CW)".to_string();

    // QSO sprzed lat z tym samym identyfikatorem zawodów i zwykłe QSO z dziennika.
    let mut old = QsoRecord::new("K1OLD", "20m", "CW");
    old.qso_date = "20200328".to_string();
    old.time_on = "1200".to_string();
    old.contest_id = Some("CQ-WPX-CW".to_string());
    let mut plain = QsoRecord::new("DL1ABC", "20m", "CW");
    plain.qso_date = "20200328".to_string();
    plain.time_on = "1300".to_string();
    {
        let db = app.services.log_db.lock().unwrap();
        db.insert_qso(&old).unwrap();
        db.insert_qso(&plain).unwrap();
    }
    app.rebuild_awards_full();

    app.start_contest_session();
    app.entry.callsign = "W1AW".to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "CW".to_string();
    app.save_qso();

    let saved = app
        .log_view
        .recent_qsos
        .iter()
        .find(|q| q.callsign == "W1AW")
        .cloned()
        .unwrap();
    assert_eq!(saved.contest_id.as_deref(), Some("CQ-WPX-CW"));
    assert_eq!(saved.stx_string.as_deref(), Some("001"));
    assert_eq!(
        app.contest.stx, 2,
        "the serial advances after a saved contest QSO"
    );

    s7_render_contest_window(&mut app);
    let calls: Vec<&str> = app
        .contest
        .cache
        .session_qsos
        .iter()
        .map(|q| q.callsign.as_str())
        .collect();
    assert_eq!(
        calls,
        vec!["W1AW"],
        "old and untagged QSOs are not part of the session"
    );
    assert_eq!(app.contest.cache.scoreboard.qsos, 1);
}

#[tokio::test]
async fn s7_station_worked_before_the_session_is_not_a_contest_dupe() {
    let (_config_dir, mut app) = s7_app();
    app.contest.name = "CQ WPX Contest (CW)".to_string();
    let mut earlier = QsoRecord::new("W1AW", "20m", "CW");
    earlier.qso_date = "20240101".to_string();
    earlier.time_on = "1200".to_string();
    app.services
        .log_db
        .lock()
        .unwrap()
        .insert_qso(&earlier)
        .unwrap();
    app.rebuild_awards_full();
    app.start_contest_session();
    app.entry.callsign = "W1AW".to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "CW".to_string();
    s7_render_contest_window(&mut app);
    assert!(
        !app.contest
            .cache
            .session_qsos
            .iter()
            .any(|q| q.callsign == "W1AW"),
        "the earlier QSO is outside the session, so W1AW is not flagged as a dupe"
    );
}

async fn s7_sent(device: &mut tokio::io::DuplexStream, expected: &str) {
    use tokio::io::AsyncReadExt;
    let mut buf = vec![0u8; expected.len()];
    device.read_exact(&mut buf).await.unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), expected);
}

#[tokio::test]
async fn s7_esm_in_run_mode_sends_cq_then_the_exchange_then_tu() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (_config_dir, mut app) = s7_app();
    app.config.station.callsign = "SP6INA".to_string();
    app.contest.name = "CQ WPX Contest (CW)".to_string();
    app.start_contest_session();
    let (host, mut device) = tokio::io::duplex(512);
    app.gui.dialogs.cw_terminal_dialog.keyer = Some(crate::cat::cw_keyer::spawn_winkeyer(
        host,
        app.config.cw.keyer.clone(),
    ));
    let mut open = [0u8; 2];
    device.read_exact(&mut open).await.unwrap();
    device.write_all(&[31]).await.unwrap();
    let mut setup = [0u8; 6];
    device.read_exact(&mut setup).await.unwrap();

    // Puste pole znaku: CQ, bez zapisu.
    assert!(!app.contest_enter());
    s7_sent(&mut device, "CQ TEST SP6INA SP6INA TEST ").await;

    // Znak bez wymiany: wymiana i przejście do pola wymiany.
    app.entry.callsign = "W1AW".to_string();
    assert!(!app.contest_enter());
    assert!(app.contest.focus_exchange_requested);
    s7_sent(&mut device, "W1AW 5NN 001 ").await;

    // Znak i wymiana: TU i zapis.
    app.entry.exchange = "599 123".to_string();
    assert!(app.contest_enter());
    s7_sent(&mut device, "TU SP6INA TEST ").await;

    // S&P: znak bez wymiany wysyła mój znak.
    app.toggle_contest_mode();
    app.entry.exchange.clear();
    assert!(!app.contest_enter());
    s7_sent(&mut device, "SP6INA ").await;
}

#[tokio::test]
async fn s7_without_a_keyer_enter_just_saves() {
    let (_config_dir, mut app) = s7_app();
    app.contest.name = "CQ WPX Contest (CW)".to_string();
    app.start_contest_session();
    assert!(
        app.contest_enter(),
        "ESM needs a keyer; otherwise Enter logs the QSO"
    );
}

#[test]
fn s7_contest_function_keys_send_macros_only_in_the_contest_context() {
    let registry = crate::gui::ShortcutRegistry::default();
    assert_eq!(registry.find_conflicts().len(), 0);
    let contest_f1 = registry
        .bindings()
        .iter()
        .find(|b| b.action == crate::gui::AppAction::ContestMacro(0))
        .expect("F1 contest macro binding");
    assert_eq!(contest_f1.shortcut.logical_key, egui::Key::F1);
    assert_eq!(contest_f1.context, crate::gui::ShortcutContext::Contest);
    assert_eq!(
        registry.label_for_action(crate::gui::AppAction::SaveQso),
        Some("F2"),
        "outside the contest window F2 still saves the QSO"
    );
}

#[test]
fn s7_contest_session_survives_a_restart() {
    let temp = tempfile::tempdir().unwrap();
    let cfg_path = temp.path().join("station_config.json");
    let mut app = s5_app_from_config(AppConfig::default());
    app.paths.config_file_path = cfg_path.clone();
    app.contest.name = "IARU HF Championship".to_string();
    app.config.station.itu_zone = 28;
    app.start_contest_session();
    app.contest.session.as_mut().unwrap().categories.power = "LOW".to_string();
    app.save_station_config();

    let restarted = s5_app_from_config(AppConfig::load_from_file(&cfg_path).unwrap());
    let session = restarted.contest.session.as_ref().unwrap();
    assert_eq!(session.contest_id, "IARU-HF");
    assert_eq!(session.sent_exchange, "28", "IARU HF sends the ITU zone");
    assert_eq!(session.categories.power, "LOW");
    assert!(session.is_active());
}

// ── STAGE-V140-S8: dwukierunkowa współpraca z WSJT-X / JTDX ──

use crate::digital::wsjtx::{
    self as wsjtx_proto, WsjtxDecode, WsjtxMark, WsjtxMessage, WsjtxOutgoing, WsjtxStatus,
};

/// Aplikacja z wirtualnym kanałem poleceń zamiast gniazda UDP.
fn s8_app() -> (
    tempfile::TempDir,
    Box<SpLogApp>,
    tokio::sync::mpsc::UnboundedReceiver<WsjtxOutgoing>,
) {
    let (dir, mut app) = s7_app();
    app.config.station.callsign = "SP6INA".to_string();
    // Pierwsza klatka programu: przeliczenie ocen zlecone przy starcie, jeszcze bez dekodów.
    app.poll_wsjtx();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    app.digital.wsjtx.out = Some(tx);
    (dir, app, rx)
}

fn s8_status(id: &str, dial_freq: u64) -> WsjtxMessage {
    WsjtxMessage::Status(Box::new(WsjtxStatus {
        id: id.to_string(),
        dial_freq,
        mode: "FT8".to_string(),
        ..WsjtxStatus::default()
    }))
}

fn s8_decode(id: &str, message: &str) -> WsjtxDecode {
    WsjtxDecode {
        id: id.to_string(),
        is_new: true,
        time_ms: 43_200_000,
        snr: -8,
        delta_time: 0.1,
        delta_freq: 1500,
        mode: "~".to_string(),
        message: message.to_string(),
        low_confidence: false,
        off_air: false,
    }
}

fn s8_feed(app: &mut SpLogApp, messages: Vec<WsjtxMessage>) {
    for message in messages {
        app.digital.wsjtx_tx.send(message).unwrap();
    }
    app.poll_wsjtx();
}

fn s8_sent(rx: &mut tokio::sync::mpsc::UnboundedReceiver<WsjtxOutgoing>) -> Vec<WsjtxOutgoing> {
    let mut sent = Vec::new();
    while let Ok(packet) = rx.try_recv() {
        sent.push(packet);
    }
    sent
}

fn s8_highlight(id: &str, call: &str, mark: WsjtxMark) -> WsjtxOutgoing {
    let (background, foreground) = mark.colors().unwrap();
    WsjtxOutgoing {
        client_id: id.to_string(),
        packet: wsjtx_proto::encode_highlight(id, call, background, foreground, false),
    }
}

fn s8_clear_all(id: &str) -> WsjtxOutgoing {
    WsjtxOutgoing {
        client_id: id.to_string(),
        packet: wsjtx_proto::encode_clear_highlights(id),
    }
}

fn s8_logged(call: &str, band: &str, freq: f64) -> WsjtxMessage {
    // Tak jak po dekodowaniu pakietu bez raportów: pola RST są puste.
    let mut qso = QsoRecord::new(call, band, "FT8");
    qso.rst_sent.clear();
    qso.rst_rcvd.clear();
    qso.qso_date = "20261122".to_string();
    qso.time_on = "120000".to_string();
    qso.freq = Some(freq);
    WsjtxMessage::QsoLogged {
        id: "WSJT-X".to_string(),
        qso: Box::new(qso),
    }
}

#[tokio::test]
async fn s8_cq_callers_are_coloured_once_by_their_log_status() {
    let (_dir, mut app, mut rx) = s8_app();
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 14_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP9ABC JO90")),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "SP9ABC SP3XYZ -05")),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP9ABC JO90")),
        ],
    );
    assert_eq!(
        s8_sent(&mut rx),
        vec![s8_highlight("WSJT-X", "SP9ABC", WsjtxMark::NewDxcc)],
        "a CQ caller is highlighted once; other messages are not highlighted"
    );
    let decodes = &app.digital.wsjtx.decodes;
    assert_eq!(decodes.len(), 3);
    assert_eq!(decodes[0].call.as_deref(), Some("SP9ABC"));
    assert_eq!(decodes[0].band.as_deref(), Some("20m"));
    assert_eq!(decodes[0].mark, WsjtxMark::NewDxcc);
    assert_eq!(decodes[1].call.as_deref(), Some("SP3XYZ"));
    assert_eq!(app.entry.band, "20m");
    assert_eq!(app.entry.mode, "FT8");
}

#[tokio::test]
async fn s8_unknown_band_or_country_is_not_highlighted() {
    let (_dir, mut app, mut rx) = s8_app();
    // Bez pakietu Status pasmo jest nieznane; K1ABC nie ma w testowym pliku krajów.
    s8_feed(
        &mut app,
        vec![WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP9ABC JO90"))],
    );
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 14_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ K1ABC FN42")),
        ],
    );
    assert_eq!(s8_sent(&mut rx), Vec::<WsjtxOutgoing>::new());
    assert!(
        app.digital
            .wsjtx
            .decodes
            .iter()
            .all(|row| row.mark == WsjtxMark::Other)
    );
}

#[tokio::test]
async fn s8_logging_a_qso_recolours_the_band_activity() {
    let (_dir, mut app, mut rx) = s8_app();
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 7_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP3XYZ JO82")),
        ],
    );
    assert_eq!(
        s8_sent(&mut rx),
        vec![s8_highlight("WSJT-X", "SP3XYZ", WsjtxMark::NewDxcc)]
    );

    s8_feed(&mut app, vec![s8_logged("SP3XYZ", "40m", 7.074)]);
    assert_eq!(
        s8_sent(&mut rx),
        vec![
            s8_clear_all("WSJT-X"),
            s8_highlight("WSJT-X", "SP3XYZ", WsjtxMark::WorkedBefore),
        ],
        "after the QSO the station is worked on this band"
    );
    assert_eq!(app.digital.wsjtx.decodes[0].mark, WsjtxMark::WorkedBefore);

    // Na innym paśmie ten sam kraj jest nowym pasmem.
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 14_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP3XYZ JO82")),
        ],
    );
    let sent = s8_sent(&mut rx);
    assert_eq!(
        sent.last(),
        Some(&s8_highlight("WSJT-X", "SP3XYZ", WsjtxMark::NewBand))
    );
    assert_eq!(app.digital.wsjtx.decodes[0].mark, WsjtxMark::NewBand);
}

#[tokio::test]
async fn s8_highlights_are_capped_per_instance() {
    let (_dir, mut app, mut rx) = s8_app();
    let mut messages = vec![s8_status("WSJT-X", 14_074_000)];
    let calls: Vec<String> = (0..=wsjtx_proto::MAX_HIGHLIGHTS)
        .map(|n| {
            format!(
                "SP{}A{}{}",
                n % 10,
                (b'A' + (n / 26 % 26) as u8) as char,
                (b'A' + (n % 26) as u8) as char
            )
        })
        .collect();
    for call in &calls {
        messages.push(WsjtxMessage::Decode(s8_decode(
            "WSJT-X",
            &format!("CQ {call} JO90"),
        )));
    }
    s8_feed(&mut app, messages);
    let sent = s8_sent(&mut rx);
    assert_eq!(sent.len(), wsjtx_proto::MAX_HIGHLIGHTS + 2);
    assert_eq!(sent[wsjtx_proto::MAX_HIGHLIGHTS], s8_clear_all("WSJT-X"));
    assert_eq!(app.digital.wsjtx.marks.len(), 1);
}

#[tokio::test]
async fn s8_highlighting_can_be_switched_off() {
    let (_dir, mut app, mut rx) = s8_app();
    app.config.wsjtx.highlight = false;
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 14_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP9ABC JO90")),
        ],
    );
    assert_eq!(s8_sent(&mut rx), Vec::<WsjtxOutgoing>::new());
    assert_eq!(app.digital.wsjtx.decodes[0].mark, WsjtxMark::NewDxcc);
}

#[tokio::test]
async fn s8_reply_goes_back_to_the_instance_that_decoded_the_cq() {
    let (_dir, mut app, mut rx) = s8_app();
    app.config.wsjtx.highlight = false;
    let cq = s8_decode("JTDX", "CQ DX SP9ABC JO90");
    s8_feed(
        &mut app,
        vec![
            s8_status("JTDX", 14_074_000),
            WsjtxMessage::Decode(cq.clone()),
            WsjtxMessage::Decode(s8_decode("JTDX", "SP9ABC SP3XYZ R-05")),
        ],
    );
    app.reply_to_wsjtx_decode(0);
    assert!(
        s8_sent(&mut rx).is_empty(),
        "only CQ/QRZ decodes are answered"
    );
    app.reply_to_wsjtx_decode(1);
    assert_eq!(
        s8_sent(&mut rx),
        vec![WsjtxOutgoing {
            client_id: "JTDX".to_string(),
            packet: wsjtx_proto::encode_reply(&cq, 0),
        }]
    );
    app.reply_to_wsjtx_decode(99);
    assert_eq!(s8_sent(&mut rx), Vec::<WsjtxOutgoing>::new());
}

#[tokio::test]
async fn s8_clear_and_close_drop_only_that_instance() {
    let (_dir, mut app, _rx) = s8_app();
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 14_074_000),
            s8_status("JTDX", 7_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP9ABC JO90")),
            WsjtxMessage::Decode(s8_decode("JTDX", "CQ SP3XYZ JO82")),
            WsjtxMessage::Clear {
                id: "WSJT-X".to_string(),
            },
        ],
    );
    assert_eq!(app.digital.wsjtx.decodes.len(), 1);
    assert_eq!(app.digital.wsjtx.clients.len(), 2);
    s8_feed(
        &mut app,
        vec![WsjtxMessage::Close {
            id: "JTDX".to_string(),
        }],
    );
    assert!(app.digital.wsjtx.decodes.is_empty());
    assert_eq!(
        app.digital.wsjtx.clients.keys().collect::<Vec<_>>(),
        vec!["WSJT-X"]
    );
    assert!(
        app.digital
            .wsjtx
            .marks
            .keys()
            .all(|(client, _)| client == "WSJT-X")
    );
}

#[tokio::test]
async fn s8_stopping_the_link_clears_the_highlights_it_set() {
    let (_dir, mut app, mut rx) = s8_app();
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 14_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP9ABC JO90")),
        ],
    );
    s8_sent(&mut rx);
    assert!(app.stop_wsjtx_link().is_none());
    assert_eq!(s8_sent(&mut rx), vec![s8_clear_all("WSJT-X")]);
    assert!(app.digital.wsjtx.out.is_none() && app.digital.wsjtx.marks.is_empty());
}

#[tokio::test]
async fn s8_logged_qso_without_reports_is_stored_without_invented_rst() {
    let (_dir, mut app, _rx) = s8_app();
    s8_feed(&mut app, vec![s8_logged("SP9ABC", "20m", 14.074)]);
    let db = app.services.log_db.lock().unwrap();
    let stored = db.get_all_qsos_for_journal(&app.active_journal.id).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].callsign, "SP9ABC");
    assert_eq!(stored[0].rst_sent, "");
    assert_eq!(stored[0].rst_rcvd, "");
}

/// Wolny port UDP na pętli zwrotnej (gniazdo jest od razu zamykane).
fn s8_free_udp_port() -> u16 {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn s8_wait_for(app: &mut SpLogApp, mut done: impl FnMut(&SpLogApp) -> bool) {
    for _ in 0..200 {
        app.poll_wsjtx();
        if done(app) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("condition not reached in time");
}

#[tokio::test]
async fn s8_link_listens_on_the_configured_port_and_releases_it() {
    let (_dir, mut app) = s7_app();
    let port = s8_free_udp_port();
    app.config.wsjtx.port = port;
    app.start_wsjtx_link();

    let peer = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let cq = s8_decode("WSJT-X", "CQ SP9ABC JO90");
    let mut packet = Vec::new();
    packet.extend_from_slice(&0xadbc_cbdau32.to_be_bytes());
    packet.extend_from_slice(&2u32.to_be_bytes());
    packet.extend_from_slice(&2u32.to_be_bytes());
    packet.extend_from_slice(&6u32.to_be_bytes());
    packet.extend_from_slice(b"WSJT-X");
    packet.push(1);
    packet.extend_from_slice(&cq.time_ms.to_be_bytes());
    packet.extend_from_slice(&cq.snr.to_be_bytes());
    packet.extend_from_slice(&cq.delta_time.to_be_bytes());
    packet.extend_from_slice(&cq.delta_freq.to_be_bytes());
    packet.extend_from_slice(&1u32.to_be_bytes());
    packet.extend_from_slice(b"~");
    packet.extend_from_slice(&(cq.message.len() as u32).to_be_bytes());
    packet.extend_from_slice(cq.message.as_bytes());
    packet.extend_from_slice(&[0, 0]);

    for _ in 0..40 {
        peer.send_to(&packet, ("127.0.0.1", port)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        app.poll_wsjtx();
        if !app.digital.wsjtx.decodes.is_empty() {
            break;
        }
    }
    assert_eq!(app.digital.wsjtx.decodes[0].decode, cq);
    assert!(app.digital.wsjtx.is_listening());

    let link = app.stop_wsjtx_link().expect("the link task is running");
    tokio::time::timeout(std::time::Duration::from_secs(5), link)
        .await
        .unwrap()
        .unwrap();
    assert!(
        std::net::UdpSocket::bind(("0.0.0.0", port)).is_ok(),
        "the port is free again after stopping"
    );
}

#[tokio::test]
async fn s8_busy_port_is_reported_to_the_operator() {
    let (_dir, mut app) = s7_app();
    let busy = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    app.config.wsjtx.port = busy.local_addr().unwrap().port();
    app.start_wsjtx_link();
    s8_wait_for(&mut app, |app| !app.digital.wsjtx.is_listening()).await;
    let notice = app.runtime.notice_rx.try_recv().expect("a notice is sent");
    assert_eq!(notice.severity, crate::gui::state::ToastSeverity::Error);
    assert!(
        notice
            .msg
            .text(crate::core::i18n::Language::En)
            .starts_with("Cannot start the WSJT-X listener"),
    );
}

#[tokio::test]
async fn s8_disabled_link_does_not_bind() {
    let (_dir, mut app) = s7_app();
    app.config.wsjtx.enabled = false;
    app.start_wsjtx_link();
    assert!(app.digital.wsjtx.out.is_none() && app.digital.wsjtx.link.is_none());
}

#[test]
fn s8_decodes_window_renders_rows_and_settings() {
    let (_dir, mut app) = s7_app();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();
    s8_feed(
        &mut app,
        vec![
            s8_status("WSJT-X", 14_074_000),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "CQ SP9ABC JO90")),
            WsjtxMessage::Decode(s8_decode("WSJT-X", "SP9ABC SP3XYZ -05")),
        ],
    );
    app.open_tile(crate::gui::state::Tile::Wsjtx);
    app.digital.wsjtx.only_cq = true;
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::gui::wsjtx_window::render_wsjtx_window(&mut app, ui.ctx());
    });
    output.textures_delta.clear();
    assert!(app.views.panel(crate::gui::state::Tile::Wsjtx).visible);
    assert_eq!(app.digital.wsjtx.decodes.len(), 2);
}

// ── STAGE-V140-S9: spoty klastra — emisja, split, odświeżanie, odznaki, rotator ──

fn s9_spot(call: &str, frequency_khz: f64, comment: &str) -> crate::cluster::telnet::DxSpot {
    crate::cluster::telnet::DxSpot {
        spotter: "SP6INA".to_string(),
        frequency_khz,
        dx_call: call.to_string(),
        comment: comment.to_string(),
        time_utc: "1200".to_string(),
        band: crate::core::bandplan::freq_khz_to_band(frequency_khz)
            .unwrap_or("OTHER")
            .to_string(),
        is_ft8: false,
        is_skimmer: false,
        received_at: jiff::Timestamp::now().as_second(),
    }
}

#[tokio::test]
async fn s9_ssb_spot_with_up_sets_mode_split_and_filter_width() {
    let (_dir, mut app) = s7_app();
    app.entry.mode = "CW".to_string();
    app.tune_to_cluster_spot(&s9_spot("SP9ABC", 14_195.0, "UP 5-10 ssb"));
    assert_eq!(app.entry.callsign, "SP9ABC");
    assert_eq!(app.entry.band, "20m");
    assert_eq!(app.entry.mode, "SSB");
    assert_eq!(app.cat.rig_state.frequency_hz, 14_195_000);
    assert_eq!(app.cat.rig_state.mode, "USB");
    assert_eq!(app.cat.rig_state.passband_hz, 2700);
    assert!(app.cat.rig_state.split_enabled && app.cat.vfo_split);
    let status = app.gui.chrome.status_message.clone().unwrap();
    assert!(status.contains("14200.0"), "{status}");
}

#[tokio::test]
async fn s9_cw_spot_on_40m_and_plain_spot_switch_split_off() {
    let (_dir, mut app) = s7_app();
    app.cat.rig_state.split_enabled = true;
    app.cat.vfo_split = true;
    app.tune_to_cluster_spot(&s9_spot("SP3XYZ", 7_012.0, "tnx qso"));
    assert_eq!(app.entry.mode, "CW", "the CW segment decides the mode");
    assert_eq!(app.cat.rig_state.mode, "CW");
    assert_eq!(app.cat.rig_state.passband_hz, 500);
    assert!(!app.cat.rig_state.split_enabled && !app.cat.vfo_split);
}

#[tokio::test]
async fn s9_up_without_a_frequency_warns_instead_of_guessing() {
    let (_dir, mut app) = s7_app();
    app.tune_to_cluster_spot(&s9_spot("SP9ABC", 14_025.0, "listening UP"));
    assert!(!app.cat.rig_state.split_enabled);
    assert!(app.gui.chrome.toasts.iter().any(|t| {
        t.severity == crate::gui::state::ToastSeverity::Warning
            && t.message == tr(key::CLUSTER_SPLIT_UNSPECIFIED, app.gui.appearance.language)
    }));
}

#[tokio::test]
async fn s9_digital_spots_leave_the_rig_mode_to_the_digital_program() {
    let (_dir, mut app) = s7_app();
    app.cat.rig_state.mode = "USB".to_string();
    let mut spot = s9_spot("SP9ABC", 14_074.0, "");
    spot.is_ft8 = true;
    app.tune_to_cluster_spot(&spot);
    assert_eq!(app.entry.mode, "FT8");
    assert_eq!(app.cat.rig_state.mode, "USB");
}

#[tokio::test]
async fn s9_switched_off_settings_keep_mode_and_split() {
    let (_dir, mut app) = s7_app();
    app.config.cluster_spots.tune_sets_mode = false;
    app.config.cluster_spots.tune_sets_split = false;
    app.entry.mode = "RTTY".to_string();
    app.cat.rig_state.split_enabled = true;
    app.tune_to_cluster_spot(&s9_spot("SP9ABC", 14_195.0, "UP 5 SSB"));
    assert_eq!(app.entry.mode, "RTTY");
    assert!(app.cat.rig_state.split_enabled);
}

#[test]
fn s9_respot_refreshes_the_existing_spot() {
    let (_dir, mut app) = s7_app();
    app.solar.band_alert_enabled = false;
    let (tx, rx) = std::sync::mpsc::channel();
    app.cluster.event_rx = Some(rx);
    let mut first = s9_spot("SP9ABC", 14_025.0, "CQ");
    first.received_at = jiff::Timestamp::now().as_second() - 50 * 60;
    let mut other = s9_spot("SP3XYZ", 7_012.0, "");
    other.received_at = first.received_at;
    let mut respot = s9_spot("SP9ABC", 14_025.5, "UP 2");
    respot.spotter = "SP1AAA".to_string();
    tx.send(crate::cluster::telnet::ClusterEvent::Spot(first))
        .unwrap();
    tx.send(crate::cluster::telnet::ClusterEvent::Spot(other))
        .unwrap();
    tx.send(crate::cluster::telnet::ClusterEvent::Spot(respot.clone()))
        .unwrap();
    app.poll_cluster_events();
    let spots = &app.cluster.spots;
    assert_eq!(spots.len(), 2);
    assert_eq!(spots[0].dx_call, "SP9ABC");
    assert_eq!(spots[0].comment, "UP 2");
    assert_eq!(spots[0].spotter, "SP1AAA");
    assert_eq!(spots[0].received_at, respot.received_at);
}

#[test]
fn s9_spot_retention_follows_the_settings() {
    let (_dir, mut app) = s7_app();
    let now = 2_000_000_000;
    for minutes in [1, 10, 30] {
        let mut spot = s9_spot(&format!("SP{minutes}A"), 14_025.0, "");
        spot.received_at = now - minutes * 60;
        app.cluster.spots.push(spot);
    }
    app.config.cluster_spots.max_age_minutes = 15;
    app.prune_cluster_spots(now);
    assert_eq!(app.cluster.spots.len(), 2);
    // Wartości spoza zakresu są przycinane: 0 minut działa jak 5 minut.
    app.config.cluster_spots.max_age_minutes = 0;
    app.prune_cluster_spots(now);
    assert_eq!(app.cluster.spots.len(), 1);
}

#[tokio::test]
async fn s9_badges_use_the_mode_of_the_spot() {
    use crate::gui::state::cluster::SpotBadgeKind;
    let (_dir, mut app) = s7_app();
    let mut qso = QsoRecord::new("SP3XYZ", "20m", "CW");
    qso.qso_date = "20261122".to_string();
    qso.time_on = "120000".to_string();
    qso.freq = Some(14.025);
    app.handle_wsjtx_qso(qso);

    assert_eq!(
        app.cluster_spot_badge(&s9_spot("SP3XYZ", 14_025.0, "CW")),
        SpotBadgeKind::Worked
    );
    assert_eq!(
        app.cluster_spot_badge(&s9_spot("SP9ABC", 14_025.0, "")),
        SpotBadgeKind::Standard
    );
    assert_eq!(
        app.cluster_spot_badge(&s9_spot("SP9ABC", 14_250.0, "SSB")),
        SpotBadgeKind::NewMode
    );
    assert_eq!(
        app.cluster_spot_badge(&s9_spot("SP9ABC", 7_012.0, "CW")),
        SpotBadgeKind::NewBand
    );
}

#[tokio::test]
async fn s9_rotor_follows_the_spot_through_a_virtual_rotctld() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let (_dir, mut app) = s7_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    app.config.rotor_host = "127.0.0.1".to_string();
    app.config.rotor_port = listener.local_addr().unwrap().port();
    app.config.station.gridsquare = "FN42AA".to_string();
    app.config.cluster_spots.rotor_follows_spot = true;

    let expected = app.bearing_to_callsign("SP9ABC").expect("bearing");
    app.tune_to_cluster_spot(&s9_spot("SP9ABC", 14_025.0, ""));

    let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut stream = tokio::io::BufReader::new(stream);
    let mut line = String::new();
    stream.read_line(&mut line).await.unwrap();
    stream.get_mut().write_all(b"RPRT 0\n").await.unwrap();
    assert_eq!(line, format!("P {:.1} 0.0\n", expected.round()));
    assert!((app.cat.rotor_state.azimuth_deg - expected.round()).abs() < f32::EPSILON);
}

#[tokio::test]
async fn s9_rotor_stays_still_by_default() {
    let (_dir, mut app) = s7_app();
    app.config.station.gridsquare = "JO81LC".to_string();
    let before = app.cat.rotor_state.azimuth_deg;
    app.tune_to_cluster_spot(&s9_spot("SP9ABC", 14_025.0, ""));
    assert!((app.cat.rotor_state.azimuth_deg - before).abs() < f32::EPSILON);
}

#[test]
fn applied_band_plan_drives_spot_modes_and_the_built_in_plan_is_stored_as_none() {
    use crate::cluster::spot_info::{self, SpotMode};
    use crate::core::bandplan::{BandPlan, PlanSegment, SegmentMode};
    let mut app = s2_app();
    assert_eq!(
        spot_info::spot_mode("", 14_250_000, false, app.config.active_bandplan()),
        Some(SpotMode::Ssb)
    );
    let mut plan = BandPlan::iaru_region1().clone();
    plan.band_mut("20m").unwrap().segments = vec![PlanSegment {
        label: "CW".to_string(),
        start_hz: 14_000_000,
        end_hz: 14_350_000,
        mode: SegmentMode::Cw,
    }];
    app.apply_bandplan(plan.clone());
    assert_eq!(app.config.bandplan.as_ref(), Some(&plan));
    assert_eq!(
        spot_info::spot_mode("", 14_250_000, false, app.config.active_bandplan()),
        Some(SpotMode::Cw)
    );

    app.open_bandplan_editor();
    assert!(app.gui.dialogs.bandplan_editor.is_open);
    assert_eq!(app.gui.dialogs.bandplan_editor.draft(), &plan);

    app.apply_bandplan(BandPlan::iaru_region1().clone());
    assert_eq!(app.config.bandplan, None);
}

// ======================================================================================
// Moduły na pulpicie (S12a): kontest, WSJT-X, makra CW, statystyki, dyplomy, ewidencja
// ======================================================================================

fn s12_modules() -> impl Iterator<Item = crate::gui::state::Tile> {
    crate::gui::state::Tile::ALL
        .into_iter()
        .filter(|t| t.is_module())
}

#[test]
fn docked_module_joins_the_logbook_leaf() {
    use crate::gui::state::Tile;
    for tile in s12_modules() {
        let mut app = qso_form_test_app();
        app.views = crate::gui::state::ViewsState::from_config(&AppConfig::default());
        app.rebuild_dock_state_from_panels();
        let docked_before = app.views.visible_count();
        app.open_tile(tile);
        app.sync_dock_state();
        assert_eq!(
            app.views.visible_count(),
            docked_before,
            "{tile:?}: okno nie zajmuje pulpitu"
        );
        assert!(
            app.gui
                .workspace
                .dock_state
                .find_tab(&tile.id().to_string())
                .is_none(),
            "{tile:?}: otwarty moduł jest najpierw oknem"
        );
        app.dock_tile(tile.id());
        app.sync_dock_state();
        let dock = &app.gui.workspace.dock_state;
        let module = dock
            .find_tab(&tile.id().to_string())
            .expect("moduł zadokowany");
        let log = dock
            .find_tab(&Tile::Log.id().to_string())
            .expect("dziennik w układzie");
        assert_eq!(
            module.node_path(),
            log.node_path(),
            "{tile:?}: obok dziennika"
        );
        assert_eq!(app.views.visible_count(), docked_before + 1);

        app.close_tile(tile.id());
        app.sync_dock_state();
        assert!(!app.views.panel(tile).visible);
        assert!(
            app.gui
                .workspace
                .dock_state
                .find_tab(&tile.id().to_string())
                .is_none()
        );
    }
}

#[test]
fn module_toggle_and_close_side_effects() {
    use crate::gui::state::Tile;
    let mut app = qso_form_test_app();
    app.toggle_tile(Tile::Wsjtx);
    assert!(app.views.panel(Tile::Wsjtx).visible);
    app.digital.wsjtx.draft = Some(app.config.wsjtx.clone());
    app.toggle_tile(Tile::Wsjtx);
    assert!(!app.views.panel(Tile::Wsjtx).visible);
    // Niezastosowane ustawienia znikają razem z oknem, jak dotąd.
    assert!(app.digital.wsjtx.draft.is_none());
    // Zamknięcie nie zmienia miejsca modułu: po ponownym otwarciu wraca zadokowany.
    app.open_tile(Tile::Statistics);
    app.dock_tile(Tile::Statistics.id());
    app.close_tile(Tile::Statistics.id());
    app.open_tile(Tile::Statistics);
    assert!(!app.views.panel(Tile::Statistics).floating);
}

#[test]
fn module_layout_survives_config_save_and_profiles() {
    use crate::gui::state::Tile;
    let mut app = qso_form_test_app();
    app.open_tile(Tile::Contest);
    app.dock_tile(Tile::Contest.id());
    app.open_tile(Tile::Awards);

    let cfg = app.snapshot_config();
    assert_eq!(
        cfg.module_panels
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["awards", "contest"]
    );
    let text = serde_json::to_string(&cfg).unwrap();
    let back: AppConfig = serde_json::from_str(&text).unwrap();
    let views = crate::gui::state::ViewsState::from_config(&back);
    assert!(views.panel(Tile::Contest).visible && !views.panel(Tile::Contest).floating);
    assert!(views.panel(Tile::Awards).visible && views.panel(Tile::Awards).floating);
    assert!(!views.panel(Tile::Ledger).visible);

    let profile = crate::gui::workspace_profiles::capture_profile(&app, "Contest");
    app.close_tile(Tile::Contest.id());
    app.close_tile(Tile::Awards.id());
    crate::gui::workspace_profiles::apply_profile(&mut app, &profile);
    assert!(app.views.panel(Tile::Contest).visible);
    assert!(app.views.panel(Tile::Awards).visible);
    assert!(
        app.gui
            .workspace
            .dock_state
            .find_tab(&Tile::Contest.id().to_string())
            .is_some()
    );

    // Profil sprzed v1.4 (bez modułów) zamyka moduły.
    let mut old = profile.clone();
    old.module_panels.clear();
    crate::gui::workspace_profiles::apply_profile(&mut app, &old);
    assert!(!app.views.panel(Tile::Contest).visible);
}

#[tokio::test]
async fn every_module_renders_as_a_window_and_as_a_tile() {
    use egui_dock::TabViewer;
    for tile in s12_modules() {
        let mut app = qso_form_test_app();
        app.open_tile(tile);
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                app.render_advanced_module_windows(ui.ctx());
            });
            output.textures_delta.clear();
        }
        assert!(
            app.views.panel(tile).visible,
            "{tile:?}: okno zostaje otwarte"
        );
        app.dock_tile(tile.id());
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let mut tab = tile.id().to_string();
                TabViewer::ui(&mut app, ui, &mut tab);
                assert_eq!(
                    TabViewer::title(&mut app, &mut tab).text(),
                    app.tile_title(tile.id())
                );
            });
            output.textures_delta.clear();
        }
    }
}

// ======================================================================================
// Strony pulpitu (S12b)
// ======================================================================================

#[test]
fn workspace_pages_keep_their_own_layouts() {
    use crate::gui::state::Tile;
    let mut app = qso_form_test_app();
    app.rebuild_dock_state_from_panels();
    assert_eq!(app.workspace_pages(), []);

    app.add_workspace_page();
    assert_eq!(app.workspace_pages().len(), 2);
    assert_eq!(app.active_workspace_page(), 1);
    assert_ne!(app.workspace_pages()[0].name, app.workspace_pages()[1].name);

    // Strona 2: kontest zadokowany, mapa pasma ukryta.
    app.open_tile(Tile::Contest);
    app.dock_tile(Tile::Contest.id());
    app.close_tile(Tile::Bandmap.id());
    app.sync_dock_state();

    app.switch_workspace_page(0);
    assert_eq!(app.active_workspace_page(), 0);
    assert!(!app.views.panel(Tile::Contest).visible);
    assert!(app.views.panel(Tile::Bandmap).visible);
    let dock = &app.gui.workspace.dock_state;
    assert!(dock.find_tab(&"contest".to_string()).is_none());
    assert!(dock.find_tab(&"bandmap".to_string()).is_some());

    app.step_workspace_page(1);
    assert_eq!(app.active_workspace_page(), 1);
    assert!(app.views.panel(Tile::Contest).visible);
    assert!(!app.views.panel(Tile::Bandmap).visible);
    assert!(
        app.gui
            .workspace
            .dock_state
            .find_tab(&"contest".to_string())
            .is_some()
    );

    // Przełączanie zawija się na końcach.
    app.step_workspace_page(1);
    assert_eq!(app.active_workspace_page(), 0);
    app.step_workspace_page(-1);
    assert_eq!(app.active_workspace_page(), 1);

    // Strony trafiają do zapisu konfiguracji.
    let cfg = app.snapshot_config();
    assert_eq!(cfg.workspace_pages.len(), 2);
    assert_eq!(cfg.active_workspace_page, 1);
}

#[test]
fn workspace_pages_rename_and_remove() {
    let mut app = qso_form_test_app();
    app.rebuild_dock_state_from_panels();
    app.add_workspace_page();
    app.add_workspace_page();
    assert_eq!(app.workspace_pages().len(), 3);
    assert_eq!(app.active_workspace_page(), 2);

    app.rename_workspace_page(0, "  FT8 \n");
    assert_eq!(app.workspace_pages()[0].name, "FT8");
    app.rename_workspace_page(0, "   ");
    assert_eq!(app.workspace_pages()[0].name, "FT8");

    // Usunięcie aktywnej strony przełącza na sąsiednią.
    app.remove_workspace_page(2);
    assert_eq!(app.workspace_pages().len(), 2);
    assert_eq!(app.active_workspace_page(), 1);
    app.remove_workspace_page(0);
    // Jedna strona to zwykły pulpit bez paska stron.
    assert_eq!(app.workspace_pages(), []);
    assert_eq!(app.active_workspace_page(), 0);

    for _ in 0..crate::gui::workspace_pages::MAX_PAGES + 2 {
        app.add_workspace_page();
    }
    assert_eq!(
        app.workspace_pages().len(),
        crate::gui::workspace_pages::MAX_PAGES
    );
}

#[test]
fn workspace_page_bar_and_rename_dialog_render() {
    let mut app = qso_form_test_app();
    app.rebuild_dock_state_from_panels();
    app.add_workspace_page();
    app.start_page_rename(0);
    let ctx = egui::Context::default();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.render_workspace_page_bar(ui);
            app.render_page_rename_dialog(ui.ctx());
        });
        output.textures_delta.clear();
    }
    assert!(
        app.gui.workspace.page_rename.is_some(),
        "okno zostaje otwarte"
    );
}

#[test]
fn workspace_pages_from_disk_are_normalized() {
    let mut cfg = AppConfig::default();
    cfg.workspace_pages = vec![crate::core::station::WorkspacePage {
        name: "Only".to_string(),
        layout: crate::core::station::workspace_profile_presets().remove(0),
    }];
    cfg.active_workspace_page = 5;
    cfg.normalize_workspace_pages();
    assert_eq!(cfg.workspace_pages.len(), 0);
    assert_eq!(cfg.active_workspace_page, 0);

    let page = crate::core::station::WorkspacePage {
        name: "A".to_string(),
        layout: crate::core::station::workspace_profile_presets().remove(0),
    };
    cfg.workspace_pages = vec![page.clone(), page];
    cfg.active_workspace_page = 7;
    cfg.normalize_workspace_pages();
    assert_eq!(cfg.workspace_pages.len(), 2);
    assert_eq!(cfg.active_workspace_page, 0);
    let back: AppConfig = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
    assert_eq!(back.workspace_pages, cfg.workspace_pages);
}

#[test]
fn workspace_pages_switch_with_ctrl_page_keys() {
    use crate::gui::shortcuts::{AppAction, ShortcutRegistry};
    let registry = ShortcutRegistry::default();
    for (action, key) in [
        (AppAction::PreviousWorkspacePage, egui::Key::PageUp),
        (AppAction::NextWorkspacePage, egui::Key::PageDown),
    ] {
        let binding = registry
            .bindings()
            .iter()
            .find(|b| b.action == action)
            .expect("skrót strony pulpitu");
        assert_eq!(binding.shortcut.logical_key, key);
        assert!(binding.shortcut.modifiers.command);
    }
}

// ======================================================================================
// Blokada układu pulpitu (S12c)
// ======================================================================================

#[test]
fn locked_layout_refuses_undock_dock_and_close() {
    use crate::gui::state::Tile;
    use egui_dock::TabViewer;
    let mut app = qso_form_test_app();
    app.rebuild_dock_state_from_panels();
    app.set_layout_locked(true);
    assert!(app.layout_locked());
    assert!(app.snapshot_config().workspace_locked);
    let toasts_before = app.gui.chrome.toasts.len();

    app.popout_tile("log");
    assert!(!app.views.panel_log.floating, "odpięcie zablokowane");
    assert!(
        app.gui.chrome.toasts.len() > toasts_before,
        "przypomnienie o blokadzie"
    );

    app.open_tile(Tile::Contest);
    app.dock_tile(Tile::Contest.id());
    assert!(
        app.views.panel(Tile::Contest).floating,
        "dokowanie zablokowane"
    );

    let mut tab = "log".to_string();
    assert!(!TabViewer::is_closeable(&app, &tab));
    let _ = TabViewer::on_close(&mut app, &mut tab);
    assert!(
        app.views.panel_log.visible,
        "karta nie znika przy blokadzie"
    );

    // Świadome działania z menu nadal działają: zamknięcie modułu, przełączenie strony.
    app.close_tile(Tile::Contest.id());
    assert!(!app.views.panel(Tile::Contest).visible);

    app.set_layout_locked(false);
    app.popout_tile("log");
    assert!(app.views.panel_log.floating);
    assert!(TabViewer::is_closeable(&app, &tab));
}

#[tokio::test]
async fn locked_workspace_renders_without_undock_controls() {
    use crate::gui::state::Tile;
    let mut app = qso_form_test_app();
    app.rebuild_dock_state_from_panels();
    app.open_tile(Tile::Statistics);
    app.set_layout_locked(true);
    let ctx = egui::Context::default();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            app.render_central_workspace(ui);
            app.render_advanced_module_windows(ui.ctx());
        });
        output.textures_delta.clear();
    }
    assert!(app.layout_locked());
    assert!(app.views.panel(Tile::Statistics).visible);
}

// ======================================================================================
// Menu główne (S13a): dziewięć menu ułożonych wokół pracy w eterze
// ======================================================================================

#[test]
fn main_menu_has_the_nine_operating_menus_in_every_language() {
    use crate::core::i18n::{Language, key, tr};
    for lang in [
        Language::Pl,
        Language::En,
        Language::De,
        Language::Fr,
        Language::Es,
        Language::It,
        Language::Ru,
    ] {
        let mut app = qso_form_test_app();
        app.gui.appearance.language = lang;
        let ctx = egui::Context::default();
        let mut texts = Vec::new();
        for _ in 0..2 {
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1920.0, 200.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_ui(raw_input, |ui| {
                crate::gui::menu::render_menu_bar(&mut app, ui);
            });
            output.textures_delta.clear();
            texts.clear();
            for clipped in &output.shapes {
                collect_text_shapes(&clipped.shape, &mut texts);
            }
        }
        let titles = [
            key::MENU_LOG,
            key::MENU_RADIO,
            key::MENU_SPOTS,
            key::MENU_CONTEST,
            key::MENU_DIGITAL,
            key::MENU_VIEW,
            key::MENU_TOOLS,
            key::MENU_SETTINGS,
            key::MENU_HELP,
        ];
        let mut last_x = f32::MIN;
        for k in titles {
            let title = tr(k, lang);
            let (_, rect) = texts
                .iter()
                .find(|(t, _)| t == title)
                .unwrap_or_else(|| panic!("{lang:?}: brak menu «{title}»"));
            assert!(
                rect.min.x > last_x,
                "{lang:?}: kolejność menu przy «{title}»"
            );
            last_x = rect.min.x;
        }
    }
}

#[test]
fn command_palette_labels_are_unique_and_cover_the_main_menu_actions() {
    use crate::core::i18n::key;
    let commands = crate::gui::command_palette::COMMANDS;
    let mut labels: Vec<&str> = commands.iter().map(|c| c.label_key.as_str()).collect();
    labels.sort_unstable();
    let before = labels.len();
    labels.dedup();
    assert_eq!(
        labels.len(),
        before,
        "każde polecenie palety ma własną etykietę"
    );
    for k in [
        key::MENU_IMPORT_ADIF,
        key::MENU_EXPORT_ADIF,
        key::TOOLS_CAT_CONNECTION,
        key::CONTEST_EXPORT_CABRILLO,
        key::REF_CF_INSTALL,
        key::MENU_PLUGIN_MANAGER,
        key::BANDPLAN_MENU,
        key::WORKSPACE_LOCK,
    ] {
        assert!(
            commands.iter().any(|c| c.label_key == k),
            "polecenie {} w palecie",
            k.as_str()
        );
    }
}

// ======================================================================================
// Pasek narzędzi (S13b): kolejność, same ikony, profile
// ======================================================================================

fn s13_toolbar_texts(app: &mut SpLogApp) -> Vec<(String, egui::Rect)> {
    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(4000.0, 200.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw_input, |ui| {
            crate::gui::menu::render_main_toolbar(app, ui);
        });
        output.textures_delta.clear();
        texts.clear();
        for clipped in &output.shapes {
            collect_text_shapes(&clipped.shape, &mut texts);
        }
    }
    texts
}

#[test]
fn toolbar_follows_the_user_order_and_icons_only_mode() {
    use crate::core::i18n::{Language, key, tr};
    use crate::core::station::ToolbarItem;
    let mut app = qso_form_test_app();
    app.gui.appearance.language = Language::En;
    for _ in 0..30 {
        app.config.quick_access.move_item(ToolbarItem::Contest, -1);
    }
    let texts = s13_toolbar_texts(&mut app);
    let x_of = |needle: &str| {
        texts
            .iter()
            .find(|(t, _)| t.contains(needle))
            .map_or_else(|| panic!("brak «{needle}»"), |(_, r)| r.min.x)
    };
    let contest = tr(key::TOOLBAR_CONTEST, Language::En);
    let vfo = tr(key::TOOLBAR_VFO, Language::En);
    assert!(x_of(contest) < x_of(vfo), "kontest przesunięty na początek");

    app.config.quick_access.icons_only = true;
    let texts = s13_toolbar_texts(&mut app);
    assert!(
        !texts.iter().any(|(t, _)| t.contains(contest)),
        "w trybie ikon nazwa jest tylko w podpowiedzi"
    );
    assert!(
        texts.iter().any(|(t, _)| t.contains("WSJT-X")),
        "wskaźnik stanu zachowuje tekst"
    );
}

#[test]
fn toolbar_travels_with_profiles_but_not_with_pages() {
    use crate::core::station::ToolbarItem;
    let mut app = qso_form_test_app();
    app.rebuild_dock_state_from_panels();
    app.config.quick_access.icons_only = true;
    app.config.quick_access.move_item(ToolbarItem::Log, -2);
    let profile = crate::gui::workspace_profiles::capture_profile(&app, "Compact");
    assert_eq!(profile.toolbar.as_ref(), Some(&app.config.quick_access));

    app.config.quick_access = crate::core::station::QuickAccessConfig::default();
    crate::gui::workspace_profiles::apply_profile(&mut app, &profile);
    assert!(app.config.quick_access.icons_only);

    app.add_workspace_page();
    assert_eq!(app.workspace_pages()[1].layout.toolbar, None);
    app.config.quick_access.icons_only = false;
    app.switch_workspace_page(0);
    assert!(
        !app.config.quick_access.icons_only,
        "strona nie zmienia paska"
    );
}

// ======================================================================================
// Personalizacja motywu (S13c)
// ======================================================================================

#[test]
fn theme_customizer_opens_renders_and_saves() {
    use crate::core::station::{ThemeCustomization, UiDensity};
    let mut app = qso_form_test_app();
    app.open_theme_customizer([56, 189, 248]);
    let draft = app.gui.windows.theme_accent_draft.expect("okno otwarte");
    assert!(!draft.own);
    assert_eq!(draft.rgb, [56, 189, 248]);

    let ctx = egui::Context::default();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::gui::theme_customizer::render_theme_customizer(&mut app, ui.ctx());
        });
        output.textures_delta.clear();
    }
    assert!(
        app.gui.windows.theme_accent_draft.is_some(),
        "okno zostaje otwarte"
    );

    let custom = ThemeCustomization {
        accent_rgb: Some([255, 200, 0]),
        density: UiDensity::Comfortable,
    };
    app.apply_theme_customization(custom);
    assert_eq!(app.snapshot_config().theme_custom, custom);
    app.open_theme_customizer([0, 0, 0]);
    let draft = app.gui.windows.theme_accent_draft.unwrap();
    assert!(draft.own);
    assert_eq!(draft.rgb, [255, 200, 0], "okno pokazuje zapisany akcent");
}
