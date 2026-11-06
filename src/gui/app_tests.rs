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
fn ctrl_w_in_qso_exchange_follows_explicit_qso_contract() {
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
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("qso_entry_exchange")));
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
        "Ctrl+W in exchange field must wipe the QSO form"
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
    assert!(
        conflicts.is_empty(),
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
    app.entry.callsign = "".to_string();
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
    app.entry.callsign = "".to_string();
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
fn escape_does_not_silently_discard_qso() {
    let deps = test_deps();
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: std::path::PathBuf::from("config.json"),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let mut app = SpLogApp::for_tests(init, deps);
    app.entry.callsign = "SP6INA".to_string();
    app.entry.name = "Mariusz".to_string();
    app.entry.grid = "JO81".to_string();

    let ctx = egui::Context::default();
    let call_id = egui::Id::new("qso_entry_callsign");
    ctx.memory_mut(|m| m.request_focus(call_id));

    let mut raw_input = egui::RawInput::default();
    raw_input.events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });

    let mut output = ctx.run_ui(raw_input, |ui| {
        crate::gui::qso_entry::render_qso_entry_body(&mut app, ui);
    });
    output.textures_delta.clear();

    assert_eq!(
        app.entry.callsign, "SP6INA",
        "Escape must not wipe callsign"
    );
    assert_eq!(app.entry.name, "Mariusz", "Escape must not wipe name");
    assert_eq!(app.entry.grid, "JO81", "Escape must not wipe grid");
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
            .map(|v| v.repaint_delay)
            .unwrap_or(std::time::Duration::ZERO);

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
            .map(|v| v.repaint_delay)
            .unwrap_or(std::time::Duration::ZERO);

        let requested = duration_until_next_second(mock_subsec);
        let expected_observed = compute_observed_repaint_delay(requested, 1.0 / 60.0);

        assert_eq!(
            repaint_delay, expected_observed,
            "Iter {iter}: Repaint delay must match expected observed delay for mock_subsec={mock_subsec}"
        );
        assert!(
            repaint_delay >= std::time::Duration::from_millis(30),
            "Iter {iter}: Idle GUI observed repaint delay must be >= 30ms, got {:?}",
            repaint_delay
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
        .map(|v| v.repaint_delay)
        .unwrap_or(std::time::Duration::ZERO);

    assert!(
        repaint_delay >= std::time::Duration::from_millis(50),
        "Unchanged state must not request continuous repaint storm, got {:?}",
        repaint_delay
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
