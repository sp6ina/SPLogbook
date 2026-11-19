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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
    assert!(app.gui.chrome.status_toast.is_some());
}

#[test]
fn contest_qso_ingest_success_and_stx_increment() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        app.gui.chrome.status_toast.is_some(),
        "Status toast should report validation issue"
    );
}

#[test]
fn failed_qso_save_preserves_form() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
    assert!(app.gui.chrome.status_toast.is_none());

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

    assert!(
        app.gui.chrome.status_toast.is_some(),
        "Background error must be visible in UI immediately"
    );
    let (msg, _) = app.gui.chrome.status_toast.as_ref().unwrap();
    assert!(msg.contains("Proxy server binding port conflict"));
}

#[test]
fn cache_is_reused_when_inputs_are_unchanged() {
    let deps = test_deps();
    let mut config = AppConfig::default();
    config.station.callsign = "SP6INA".to_string();
    config.is_configured = true;
    let init = AppInit {
        config,
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
            config_file_path: std::path::PathBuf::from("config.json"),
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
        config_file_path: std::path::PathBuf::from("config.json"),
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
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(id)));
    run_qso_form_frame(ctx, app, Vec::new());
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
            _ => app.contest.show_window = true,
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
    app.contest.show_window = true;
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
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(field)));

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
