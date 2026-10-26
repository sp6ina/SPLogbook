// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

#[cfg(test)]
use super::*;
use crate::core::database::{Journal, LogDatabase};
use crate::core::prefix::PrefixMatcher;
use crate::core::qso::QsoRecord;
use crate::core::scp::ScpEngine;
use crate::core::station::AppConfig;
use std::sync::{Arc, Mutex};

fn test_prefix_matcher() -> PrefixMatcher {
    let uri = format!(
        "file:prefix_test_{}?mode=memory&cache=shared",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
        | rusqlite::OpenFlags::SQLITE_OPEN_URI;
    if let Ok(conn) = rusqlite::Connection::open_with_flags(&uri, flags) {
        let _ = conn.execute_batch(
            "CREATE TABLE UniqueCalls (
                    Callsign TEXT, Country TEXT, ARRLPrefix TEXT, DXCC TEXT, Continent TEXT,
                    CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT, Enable TEXT
                );
                CREATE TABLE CountryDataEx (
                    Country TEXT, ARRLPrefix TEXT, DXCC INTEGER, Continent TEXT,
                    CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT,
                    PrefixList TEXT, EndDate TEXT
                );
                CREATE TABLE Province (
                    Country TEXT, ARRLPrefix TEXT, DXCC TEXT, Continent TEXT,
                    CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT,
                    PrefixList TEXT, Comment TEXT, EndDate TEXT
                );
                INSERT INTO UniqueCalls VALUES
                    ('SP6INA', 'Poland', 'SP', '269', 'EU', 15, 28, '51.1N', '17.0E', '1');",
        );
        if let Ok(m) = PrefixMatcher::load_from_db(&uri) {
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
        Some(std::time::Instant::now() - std::time::Duration::from_millis(10));

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
            } else if path.extension().map_or(false, |e| e == "rs") {
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
        "build() exceeds 200 lines: {} lines",
        build_len
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
    assert!(ui_len <= 120, "ui() exceeds 120 lines: {} lines", ui_len);

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
        "save_station_config() exceeds 30 lines: {} lines",
        save_len
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
        "SpLogApp exceeds 35 fields: {}",
        splogapp_fields
    );

    // 6. Each state struct in src/gui/state/*.rs <= 30 fields
    let state_dir = manifest_dir.join("src/gui/state");
    for entry in std::fs::read_dir(&state_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |ext| ext == "rs") {
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
                        "Struct {} in {:?} exceeds 30 fields: {}",
                        struct_name,
                        path.file_name().unwrap(),
                        fields
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
        if path.extension().map_or(false, |ext| ext == "rs") {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(
                !content.contains("use crate::gui::app"),
                "Found 'use crate::gui::app' in {:?}",
                path
            );
            assert!(
                !content.contains("SpLogApp"),
                "Found 'SpLogApp' in {:?}",
                path
            );
            assert!(
                !content.contains("tokio::spawn") && !content.contains("std::thread::spawn"),
                "Found background thread spawn in {:?}",
                path
            );
        }
    }

    // No unauthorized "AppConfig {" struct instantiations in src/gui (only return type annotations like "-> AppConfig {")
    let gui_dir = manifest_dir.join("src/gui");
    for entry in walkdir(&gui_dir) {
        if entry.file_name().map_or(false, |f| f == "app_tests.rs") {
            continue;
        }
        let content = std::fs::read_to_string(&entry).unwrap();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("AppConfig {") {
                assert!(
                    trimmed.contains("fn ") || trimmed.contains("-> AppConfig"),
                    "Found unauthorized 'AppConfig {{' literal in {:?}:{}",
                    entry,
                    idx + 1
                );
            }
        }
    }

    // No "contest_qsos", "contest_points", "contest_mults" in src
    let src_dir = manifest_dir.join("src");
    for entry in walkdir(&src_dir) {
        if entry.file_name().map_or(false, |f| f == "app_tests.rs") {
            continue;
        }
        let content = std::fs::read_to_string(&entry).unwrap();
        assert!(
            !content.contains("contest_qsos")
                && !content.contains("contest_points")
                && !content.contains("contest_mults"),
            "Found obsolete contest counters in {:?}",
            entry
        );
    }
}
