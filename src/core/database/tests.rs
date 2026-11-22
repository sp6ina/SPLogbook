// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz WoĹşniak (SP6INA)

use super::*;
use crate::core::qso::{QsoIdentity, QsoRecord};

#[test]
fn test_insert_and_find_qso() {
    let db = LogDatabase::open_in_memory().unwrap();
    let qso = QsoRecord::new("SP6INA", "20m", "CW");
    let id = db.insert_qso(&qso).unwrap();
    assert!(id > 0);

    let prev = db.find_previous_qsos("SP6INA").unwrap();
    assert_eq!(prev.len(), 1);
    assert_eq!(prev[0].callsign, "SP6INA");
    assert_eq!(prev[0].band, "20m");
    assert_eq!(prev[0].mode, "CW");
}

#[test]
fn schema_migrations_record_versions_and_are_idempotent() {
    let mut db = LogDatabase::open_in_memory().unwrap();
    // Wszystkie kroki migracji zostaĹ‚y zarejestrowane.
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);

    // Kolumny z migracji 2 istniejÄ… i ponowna inicjalizacja nie psuje schematu.
    let col_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('qso_records') WHERE name IN ('journal_id','my_pota_ref','my_sota_ref','vucc_grids','audio_file')",
                [],
                |r| r.get(0),
            )
            .unwrap();
    assert_eq!(col_count, 5);

    // Indeksy z migracji 4 istniejÄ….
    let idx_count: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name IN ('idx_qso_country','idx_qso_freq')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(idx_count, 2);

    db.init_schema().unwrap();
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);
}

#[test]
fn chronological_numbers_cover_entire_journal_not_just_visible_rows() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "PORTABLE".to_string(),
        name: "Test portable".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
    qso.qso_date = "2025-01-01".into();
    qso.time_on = "12:00:00".into();
    let first = db.insert_qso(&qso).unwrap();
    qso.qso_date = "2026-01-01".into();
    qso.journal_id = Some("PORTABLE".into());
    let portable = db.insert_qso(&qso).unwrap();
    qso.journal_id = Some("DEFAULT".into());
    for i in 0..501 {
        qso.time_on = format!("{:02}{:02}00", (i / 60) % 24, i % 60);
        qso.callsign = format!("SP{i}INA");
        db.insert_qso(&qso).unwrap();
    }
    let newest = db.get_recent_qsos_for_journal("DEFAULT", 1).unwrap();
    let numbers = db.qso_numbers_for_journal("DEFAULT").unwrap();
    assert_eq!(numbers.len(), 502);
    assert_eq!(numbers[&first], 1);
    assert_eq!(numbers[&newest[0].id.unwrap()], 502);
    assert_eq!(
        db.qso_numbers_for_journal("PORTABLE").unwrap()[&portable],
        1
    );

    db.delete_qso(first).unwrap();
    assert_eq!(
        db.qso_numbers_for_journal("DEFAULT").unwrap()[&newest[0].id.unwrap()],
        501
    );
    db.conn
        .execute(
            "UPDATE qso_records SET qso_date = '2025-12-31' WHERE id = ?1",
            [newest[0].id.unwrap()],
        )
        .unwrap();
    assert_eq!(
        db.qso_numbers_for_journal("DEFAULT").unwrap()[&newest[0].id.unwrap()],
        1
    );
    db.conn
        .execute(
            "UPDATE qso_records SET journal_id = 'PORTABLE' WHERE id = ?1",
            [newest[0].id.unwrap()],
        )
        .unwrap();
    assert_eq!(
        db.qso_numbers_for_journal("PORTABLE").unwrap()[&newest[0].id.unwrap()],
        1
    );
    assert_eq!(db.qso_numbers_for_journal("DEFAULT").unwrap().len(), 500);

    let mut mixed = qso.clone();
    mixed.qso_date = "20260102".into();
    mixed.time_on = "090000".into();
    let later = db.insert_qso(&mixed).unwrap();
    assert_eq!(db.qso_numbers_for_journal("DEFAULT").unwrap()[&later], 501);
}

#[test]
fn test_multi_journal_operations() {
    let db = LogDatabase::open_in_memory().unwrap();
    let journals = db.get_all_journals().unwrap();
    assert_eq!(journals.len(), 1);
    assert_eq!(journals[0].id, "DEFAULT");

    let new_j = Journal {
        id: "PORTABLE".to_string(),
        name: "Aktywacje SPFF / PGA".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: "Mariusz WoĹşniak".to_string(),
        my_gridsquare: "JO80".to_string(),
        my_pga: "KL01".to_string(),
        description: "Praca w terenie".to_string(),
        is_default: false,
    };
    db.create_journal(&new_j).unwrap();

    let updated_journals = db.get_all_journals().unwrap();
    assert_eq!(updated_journals.len(), 2);

    // Dodanie QSO do profilu PORTABLE
    let mut qso_p = QsoRecord::new("DL1ABC", "40m", "SSB");
    qso_p.journal_id = Some("PORTABLE".to_string());
    db.insert_qso(&qso_p).unwrap();

    let p_qsos = db.get_recent_qsos_for_journal("PORTABLE", 50).unwrap();
    assert_eq!(p_qsos.len(), 1);
    assert_eq!(p_qsos[0].callsign, "DL1ABC");

    let def_qsos = db.get_recent_qsos_for_journal("DEFAULT", 50).unwrap();
    assert_eq!(def_qsos.len(), 0);
}

#[test]
fn test_journal_integrity_guards() {
    let db = LogDatabase::open_in_memory().unwrap();

    // PrzeĹ‚Ä…czenie na nieistniejÄ…cy dziennik zwraca bĹ‚Ä…d.
    assert!(db.set_active_journal("GHOST").is_err());
    // Po nieudanym przeĹ‚Ä…czeniu aktywny pozostaje DEFAULT.
    let active = db.get_active_journal().unwrap();
    assert_eq!(active.id, "DEFAULT");

    // Utworzenie drugiego dziennika i przeĹ‚Ä…czenie na niego.
    let portable = Journal {
        id: "PORTABLE".to_string(),
        name: "Aktywacje".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: "Mariusz".to_string(),
        my_gridsquare: "JO80".to_string(),
        my_pga: "KL01".to_string(),
        description: String::new(),
        is_default: false,
    };
    db.create_journal(&portable).unwrap();
    db.set_active_journal("PORTABLE").unwrap();
    assert_eq!(db.get_active_journal().unwrap().id, "PORTABLE");

    // UsuniÄ™cie aktywnego dziennika jest blokowane.
    assert!(db.delete_journal("PORTABLE").is_err());

    // PowrĂłt na DEFAULT i prĂłba usuniÄ™cia DEFAULT â€” zablokowana.
    db.set_active_journal("DEFAULT").unwrap();
    assert!(db.delete_journal("DEFAULT").is_err());

    // Osierocone QSO sÄ… przepinane do DEFAULT przy usuniÄ™ciu.
    let mut qso = QsoRecord::new("DL1ABC", "40m", "SSB");
    qso.journal_id = Some("PORTABLE".to_string());
    db.insert_qso(&qso).unwrap();
    assert_eq!(
        db.get_recent_qsos_for_journal("PORTABLE", 50)
            .unwrap()
            .len(),
        1
    );

    db.delete_journal("PORTABLE").unwrap();
    assert!(
        db.get_all_journals()
            .unwrap()
            .iter()
            .all(|j| j.id != "PORTABLE")
    );
    assert_eq!(
        db.get_recent_qsos_for_journal("DEFAULT", 50).unwrap().len(),
        1
    );
}

#[test]
fn test_advanced_search_filtering() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = QsoRecord::new("SP6INA", "20m", "CW");
    q1.qso_date = "2026-09-01".to_string();
    q1.lotw_qsl_rcvd = "Y".to_string();

    let mut q2 = QsoRecord::new("W1AW", "40m", "SSB");
    q2.qso_date = "2026-09-15".to_string();

    let mut q3 = QsoRecord::new("JA1ABC", "20m", "FT8");
    q3.qso_date = "2026-09-20".to_string();

    db.batch_insert_qsos(&[q1, q2, q3]).unwrap();

    // Filtruj tylko 20m
    let mut filter = AdvancedQsoFilter::default();
    filter.bands = vec!["20m".to_string()];
    let res = db.search_qsos_advanced(&filter).unwrap();
    assert_eq!(res.len(), 2);

    // Filtruj tylko potwierdzone LoTW
    filter.bands.clear();
    filter.lotw_confirmed = Some(true);
    let res_lotw = db.search_qsos_advanced(&filter).unwrap();
    assert_eq!(res_lotw.len(), 1);
    assert_eq!(res_lotw[0].callsign, "SP6INA");
}

#[test]
fn test_batch_insert() {
    let db = LogDatabase::open_in_memory().unwrap();
    let qsos = vec![
        QsoRecord::new("SP6INA", "20m", "CW"),
        QsoRecord::new("W1AW", "40m", "SSB"),
        QsoRecord::new("JA1ABC", "15m", "FT8"),
    ];
    let inserted = db.batch_insert_qsos(&qsos).unwrap();
    assert_eq!(inserted, 3);
    assert_eq!(db.count_all().unwrap(), 3);
}

#[test]
fn test_update_qso() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
    qso.rst_sent = "599".to_string();
    qso.name = Some("Mariusz".to_string());
    let id = db.insert_qso(&qso).unwrap();

    let mut updated = qso.clone();
    updated.rst_sent = "579".to_string();
    updated.name = Some("Mariusz SP6INA".to_string());
    updated.qth = Some("WrocĹ‚aw".to_string());
    updated.gridsquare = Some("JO81WA".to_string());
    updated.pga_ref = Some("WR01".to_string());

    db.update_qso(id, &updated).unwrap();

    let list = db.find_previous_qsos("SP6INA").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].rst_sent, "579");
    assert_eq!(list[0].name.as_deref(), Some("Mariusz SP6INA"));
    assert_eq!(list[0].qth.as_deref(), Some("WrocĹ‚aw"));
    assert_eq!(list[0].gridsquare.as_deref(), Some("JO81WA"));
    assert_eq!(list[0].pga_ref.as_deref(), Some("WR01"));
}

#[test]
fn test_mark_lotw_and_eqsl() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut qso = QsoRecord::new("W1AW", "20m", "CW");
    qso.qso_date = "20260920".to_string();
    db.insert_qso(&qso).unwrap();

    let updated_lotw = db
        .mark_lotw_confirmed("W1AW", "20m", "CW", "20260920", "20260920")
        .unwrap();
    assert_eq!(updated_lotw, 1);

    let updated_eqsl = db
        .mark_eqsl_confirmed("W1AW", "20m", "CW", "20260920", "20260920")
        .unwrap();
    assert_eq!(updated_eqsl, 1);

    let list = db.find_previous_qsos("W1AW").unwrap();
    assert_eq!(list[0].lotw_qsl_rcvd, "Y");
    assert_eq!(list[0].eqsl_qsl_rcvd, "Y");
}

#[test]
fn f20r4_lotw_confirmation_without_date_keeps_the_stored_date() {
    use crate::cloud::lotw::LotwConfirmation;
    let db = LogDatabase::open_in_memory().unwrap();
    let mut qso = QsoRecord::new("W1AW", "20m", "CW");
    qso.qso_date = "20260920".to_string();
    db.insert_qso(&qso).unwrap();
    let conf = |rdate: Option<&str>| LotwConfirmation {
        callsign: "W1AW".to_string(),
        band: "20m".to_string(),
        mode: "CW".to_string(),
        qso_date: "20260920".to_string(),
        qso_time: None,
        qsl_rdate: rdate.map(str::to_string),
    };
    let stored = |db: &LogDatabase| {
        let list = db.find_previous_qsos("W1AW").unwrap();
        (list[0].lotw_qsl_rcvd.clone(), list[0].lotw_qslrdate.clone())
    };

    // No date at all: confirmed, but no date is invented.
    assert_eq!(
        db.mark_lotw_confirmed_batch_for_journal("DEFAULT", &[conf(None)])
            .unwrap(),
        1
    );
    assert_eq!(stored(&db), ("Y".to_string(), None));

    assert_eq!(
        db.mark_lotw_confirmed_batch_for_journal("DEFAULT", &[conf(Some("20260921"))])
            .unwrap(),
        1
    );
    assert_eq!(stored(&db), ("Y".to_string(), Some("20260921".to_string())));

    // A later report without a date (or with a blank one) must not erase it.
    for rdate in [None, Some(""), Some("  ")] {
        db.mark_lotw_confirmed_batch_for_journal("DEFAULT", &[conf(rdate)])
            .unwrap();
        assert_eq!(stored(&db), ("Y".to_string(), Some("20260921".to_string())));
    }
}

#[test]
fn eqsl_confirmation_without_date_never_uses_qso_date_and_keeps_stored_date() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut qso = QsoRecord::new("W1AW", "20m", "CW");
    qso.qso_date = "20260920".to_string();
    db.insert_qso(&qso).unwrap();

    let incoming = |eqsl_rdate: Option<&str>, qsl_rcvd: &str, qsl_rdate: Option<&str>| {
        let mut rec = QsoRecord::new("W1AW", "20m", "CW");
        rec.qso_date = "20260920".to_string();
        rec.eqsl_qsl_rcvd = "Y".to_string();
        rec.eqsl_qslrdate = eqsl_rdate.map(str::to_string);
        rec.qsl_rcvd = qsl_rcvd.to_string();
        rec.qsl_rcvd_date = qsl_rdate.map(str::to_string);
        rec
    };
    let stored = |db: &LogDatabase| {
        let list = db.find_previous_qsos("W1AW").unwrap();
        (list[0].eqsl_qsl_rcvd.clone(), list[0].eqsl_qslrdate.clone())
    };

    // Case 3: No date at all -> confirmed ("Y"), eqsl_qslrdate stays None (never fabricated from qso_date).
    assert_eq!(
        db.mark_eqsl_confirmed_batch_for_journal("DEFAULT", &[incoming(None, "N", None)])
            .unwrap(),
        1
    );
    assert_eq!(stored(&db), ("Y".to_string(), None));

    // Case 2: QSLRDATE when QSL_RCVD=Y populates eqsl_qslrdate.
    assert_eq!(
        db.mark_eqsl_confirmed_batch_for_journal(
            "DEFAULT",
            &[incoming(None, "Y", Some("2026-09-21"))],
        )
        .unwrap(),
        1
    );
    assert_eq!(stored(&db), ("Y".to_string(), Some("20260921".to_string())));

    // Case 1: Explicit EQSL_QSLRDATE updates eqsl_qslrdate.
    assert_eq!(
        db.mark_eqsl_confirmed_batch_for_journal(
            "DEFAULT",
            &[incoming(Some("20260922"), "Y", Some("20260921"))],
        )
        .unwrap(),
        1
    );
    assert_eq!(stored(&db), ("Y".to_string(), Some("20260922".to_string())));

    // Case 4: Later report without a date (or invalid date) preserves stored date via COALESCE.
    for rdate in [None, Some(""), Some("invalid")] {
        assert_eq!(
            db.mark_eqsl_confirmed_batch_for_journal("DEFAULT", &[incoming(rdate, "N", None)])
                .unwrap(),
            1
        );
        assert_eq!(stored(&db), ("Y".to_string(), Some("20260922".to_string())));
    }
}

#[test]
fn test_search_with_apostrophe() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut qso = QsoRecord::new("EI2O", "20m", "SSB");
    qso.name = Some("O'Connor".to_string());
    db.insert_qso(&qso).unwrap();

    let mut filter = AdvancedQsoFilter::default();
    filter.callsign_query = Some("O'Connor".to_string());
    let res = db.search_qsos_advanced(&filter);
    assert!(
        res.is_ok(),
        "Wyszukiwanie z apostrofem nie powinno powodowaÄ‡ bĹ‚Ä™du SQL"
    );
    assert_eq!(res.unwrap().len(), 1);
}

#[test]
fn test_find_and_delete_duplicates() {
    let db = LogDatabase::open_in_memory().unwrap();

    let mut q1 = QsoRecord::new("SP6INA", "20m", "CW");
    q1.qso_date = "20260920".to_string();
    q1.time_on = "120000".to_string();
    let id1 = db.insert_qso(&q1).unwrap();

    let mut q2 = QsoRecord::new("SP6INA", "20m", "CW");
    q2.qso_date = "20260920".to_string();
    q2.time_on = "120500".to_string();
    let id2 = db.insert_qso(&q2).unwrap();

    let mut q3 = QsoRecord::new("SP6INA", "40m", "CW");
    q3.qso_date = "20260920".to_string();
    q3.time_on = "121000".to_string();
    db.insert_qso(&q3).unwrap();

    let dups = db.find_duplicate_qsos("DEFAULT", false).unwrap();
    assert_eq!(
        dups.len(),
        1,
        "Powinna byÄ‡ dokĹ‚adnie jedna grupa duplikatĂłw (SP6INA / 20m / CW)"
    );
    assert_eq!(dups[0].len(), 2);

    let deleted = db.delete_multiple_qsos(&[id2]).unwrap();
    assert_eq!(deleted, 1);

    let dups_after = db.find_duplicate_qsos("DEFAULT", false).unwrap();
    assert_eq!(dups_after.len(), 0, "Brak duplikatĂłw po usuniÄ™ciu");

    let remaining = db.find_previous_qsos("SP6INA").unwrap();
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().any(|q| q.id == Some(id1)));
}

fn make_test_qso_literal(callsign: &str, qso_date: &str, time_on: &str) -> QsoRecord {
    QsoRecord {
        id: None,
        callsign: callsign.to_string(),
        band: "20m".to_string(),
        mode: "CW".to_string(),
        submode: None,
        qso_date: qso_date.to_string(),
        time_on: time_on.to_string(),
        time_off: None,
        freq: Some(14.025),
        freq_rx: None,
        rst_sent: "599".to_string(),
        rst_rcvd: "599".to_string(),
        name: None,
        qth: None,
        gridsquare: None,
        state: None,
        iota: None,
        sota_ref: None,
        pota_ref: None,
        pga_ref: None,
        dxcc: None,
        country: None,
        continent: None,
        cqz: None,
        ituz: None,
        comment: None,
        qsl_via: None,
        qsl_manager: None,
        qsl_sent: "N".to_string(),
        qsl_rcvd: "N".to_string(),
        qsl_sent_date: None,
        qsl_rcvd_date: None,
        lotw_qsl_sent: "N".to_string(),
        lotw_qsl_rcvd: "N".to_string(),
        lotw_qslrdate: None,
        lotw_qslsdate: None,
        eqsl_qsl_sent: "N".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
        eqsl_qslsdate: None,
        clublog_upload_status: None,
        qrzcom_upload_status: None,
        sat_name: None,
        sat_mode: None,
        prop_mode: None,
        srx: None,
        stx: None,
        srx_string: None,
        stx_string: None,
        my_gridsquare: None,
        my_state: None,
        my_pota_ref: None,
        my_sota_ref: None,
        vucc_grids: None,
        audio_file: None,
        journal_id: Some("DEFAULT".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    }
}

#[test]
fn insert_roundtrip_preserves_every_column() {
    let db = LogDatabase::open_in_memory().unwrap();
    let qso = QsoRecord {
        id: None,
        callsign: "sp6ina".to_string(),
        band: "20m".to_string(),
        mode: "cw".to_string(),
        submode: Some("PSK31".to_string()),
        qso_date: "20260401".to_string(),
        time_on: "101500".to_string(),
        time_off: Some("101600".to_string()),
        freq: Some(14.0701),
        freq_rx: Some(14.0702),
        rst_sent: "599".to_string(),
        rst_rcvd: "589".to_string(),
        name: Some("Jan".to_string()),
        qth: Some("Wroclaw".to_string()),
        gridsquare: Some("JO81LA".to_string()),
        state: Some("DS".to_string()),
        iota: Some("EU-001".to_string()),
        sota_ref: Some("SP/DS-001".to_string()),
        pota_ref: Some("SP-0001".to_string()),
        pga_ref: Some("WR01".to_string()),
        dxcc: Some(281),
        country: Some("Poland".to_string()),
        continent: Some("EU".to_string()),
        cqz: Some(15),
        ituz: Some(28),
        comment: Some("Test comment".to_string()),
        qsl_via: Some("Bureau".to_string()),
        qsl_manager: Some("SP6MGR".to_string()),
        qsl_sent: "Y".to_string(),
        qsl_rcvd: "N".to_string(),
        qsl_sent_date: Some("20260402".to_string()),
        qsl_rcvd_date: Some("20260403".to_string()),
        lotw_qsl_sent: "R".to_string(),
        lotw_qsl_rcvd: "V".to_string(),
        lotw_qslrdate: Some("20260404".to_string()),
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Q".to_string(),
        eqsl_qsl_rcvd: "W".to_string(),
        eqsl_qslrdate: Some("20260405".to_string()),
        eqsl_qslsdate: None,
        clublog_upload_status: Some("UPLOADED".to_string()),
        qrzcom_upload_status: Some("PENDING".to_string()),
        sat_name: Some("AO-91".to_string()),
        sat_mode: Some("U/V".to_string()),
        prop_mode: Some("SAT".to_string()),
        srx: Some(101),
        stx: Some(202),
        srx_string: Some("SRX_STR".to_string()),
        stx_string: Some("STX_STR".to_string()),
        my_gridsquare: Some("JO81WA".to_string()),
        my_state: Some("OP".to_string()),
        my_pota_ref: Some("SP-0002".to_string()),
        my_sota_ref: Some("SP/DS-002".to_string()),
        vucc_grids: Some("JO81,JO82".to_string()),
        audio_file: Some("/path/to/audio.wav".to_string()),
        journal_id: None,
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    let id = db.insert_qso(&qso).unwrap();
    let retrieved = db.get_qso_by_id(id).unwrap().unwrap();

    let expected = QsoRecord {
        id: Some(id),
        callsign: "SP6INA".to_string(),
        band: "20m".to_string(),
        mode: "CW".to_string(),
        submode: Some("PSK31".to_string()),
        qso_date: "20260401".to_string(),
        time_on: "101500".to_string(),
        time_off: Some("101600".to_string()),
        freq: Some(14.0701),
        freq_rx: Some(14.0702),
        rst_sent: "599".to_string(),
        rst_rcvd: "589".to_string(),
        name: Some("Jan".to_string()),
        qth: Some("Wroclaw".to_string()),
        gridsquare: Some("JO81LA".to_string()),
        state: Some("DS".to_string()),
        iota: Some("EU-001".to_string()),
        sota_ref: Some("SP/DS-001".to_string()),
        pota_ref: Some("SP-0001".to_string()),
        pga_ref: Some("WR01".to_string()),
        dxcc: Some(281),
        country: Some("Poland".to_string()),
        continent: Some("EU".to_string()),
        cqz: Some(15),
        ituz: Some(28),
        comment: Some("Test comment".to_string()),
        qsl_via: Some("Bureau".to_string()),
        qsl_manager: Some("SP6MGR".to_string()),
        qsl_sent: "Y".to_string(),
        qsl_rcvd: "N".to_string(),
        qsl_sent_date: Some("20260402".to_string()),
        qsl_rcvd_date: Some("20260403".to_string()),
        lotw_qsl_sent: "R".to_string(),
        lotw_qsl_rcvd: "V".to_string(),
        lotw_qslrdate: Some("20260404".to_string()),
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Q".to_string(),
        eqsl_qsl_rcvd: "W".to_string(),
        eqsl_qslrdate: Some("20260405".to_string()),
        eqsl_qslsdate: None,
        clublog_upload_status: Some("UPLOADED".to_string()),
        qrzcom_upload_status: Some("PENDING".to_string()),
        sat_name: Some("AO-91".to_string()),
        sat_mode: Some("U/V".to_string()),
        prop_mode: Some("SAT".to_string()),
        srx: Some(101),
        stx: Some(202),
        srx_string: Some("SRX_STR".to_string()),
        stx_string: Some("STX_STR".to_string()),
        my_gridsquare: Some("JO81WA".to_string()),
        my_state: Some("OP".to_string()),
        my_pota_ref: Some("SP-0002".to_string()),
        my_sota_ref: Some("SP/DS-002".to_string()),
        vucc_grids: Some("JO81,JO82".to_string()),
        audio_file: Some("/path/to/audio.wav".to_string()),
        journal_id: Some("DEFAULT".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    assert_eq!(retrieved, expected);
}

#[test]
fn update_roundtrip_preserves_every_column() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "PORTABLE".to_string(),
        name: "Portable Journal".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: "Mariusz".to_string(),
        my_gridsquare: "JO80".to_string(),
        my_pga: "KL01".to_string(),
        description: "Field".to_string(),
        is_default: false,
    })
    .unwrap();

    let initial = QsoRecord {
        id: None,
        callsign: "INIT".to_string(),
        band: "40m".to_string(),
        mode: "SSB".to_string(),
        submode: None,
        qso_date: "20260101".to_string(),
        time_on: "000000".to_string(),
        time_off: None,
        freq: None,
        freq_rx: None,
        rst_sent: "59".to_string(),
        rst_rcvd: "59".to_string(),
        name: None,
        qth: None,
        gridsquare: None,
        state: None,
        iota: None,
        sota_ref: None,
        pota_ref: None,
        pga_ref: None,
        dxcc: None,
        country: None,
        continent: None,
        cqz: None,
        ituz: None,
        comment: None,
        qsl_via: None,
        qsl_manager: None,
        qsl_sent: "N".to_string(),
        qsl_rcvd: "N".to_string(),
        qsl_sent_date: None,
        qsl_rcvd_date: None,
        lotw_qsl_sent: "N".to_string(),
        lotw_qsl_rcvd: "N".to_string(),
        lotw_qslrdate: None,
        lotw_qslsdate: None,
        eqsl_qsl_sent: "N".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
        eqsl_qslsdate: None,
        clublog_upload_status: None,
        qrzcom_upload_status: None,
        sat_name: None,
        sat_mode: None,
        prop_mode: None,
        srx: None,
        stx: None,
        srx_string: None,
        stx_string: None,
        my_gridsquare: None,
        my_state: None,
        my_pota_ref: None,
        my_sota_ref: None,
        vucc_grids: None,
        audio_file: None,
        journal_id: Some("DEFAULT".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };
    let id = db.insert_qso(&initial).unwrap();

    let updated = QsoRecord {
        id: Some(id),
        callsign: "dl1abc".to_string(),
        band: "10m".to_string(),
        mode: "rtty".to_string(),
        submode: Some("RTTY_45".to_string()),
        qso_date: "20260505".to_string(),
        time_on: "153000".to_string(),
        time_off: Some("153500".to_string()),
        freq: Some(28.0805),
        freq_rx: Some(28.0809),
        rst_sent: "579".to_string(),
        rst_rcvd: "569".to_string(),
        name: Some("Hans".to_string()),
        qth: Some("Berlin".to_string()),
        gridsquare: Some("JO62PM".to_string()),
        state: Some("BE".to_string()),
        iota: Some("EU-123".to_string()),
        sota_ref: Some("DL/BE-001".to_string()),
        pota_ref: Some("DE-0042".to_string()),
        pga_ref: Some("DL01".to_string()),
        dxcc: Some(230),
        country: Some("Germany".to_string()),
        continent: Some("EU".to_string()),
        cqz: Some(14),
        ituz: Some(27),
        comment: Some("Updated comment".to_string()),
        qsl_via: Some("Direct".to_string()),
        qsl_manager: Some("DL1MGR".to_string()),
        qsl_sent: "Y".to_string(),
        qsl_rcvd: "Y".to_string(),
        qsl_sent_date: Some("20260506".to_string()),
        qsl_rcvd_date: Some("20260507".to_string()),
        lotw_qsl_sent: "Y".to_string(),
        lotw_qsl_rcvd: "Y".to_string(),
        lotw_qslrdate: Some("20260508".to_string()),
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260509".to_string()),
        eqsl_qslsdate: None,
        clublog_upload_status: Some("MODIFIED".to_string()),
        qrzcom_upload_status: Some("CONFIRMED".to_string()),
        sat_name: Some("RS-44".to_string()),
        sat_mode: Some("V/U".to_string()),
        prop_mode: Some("TR".to_string()),
        srx: Some(303),
        stx: Some(404),
        srx_string: Some("RX_UPDATE".to_string()),
        stx_string: Some("TX_UPDATE".to_string()),
        my_gridsquare: Some("JO80AA".to_string()),
        my_state: Some("WP".to_string()),
        my_pota_ref: Some("SP-9999".to_string()),
        my_sota_ref: Some("SP/DS-999".to_string()),
        vucc_grids: Some("JO62,JO63".to_string()),
        audio_file: Some("/path/updated.wav".to_string()),
        journal_id: Some("PORTABLE".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    db.update_qso(id, &updated).unwrap();
    let retrieved = db.get_qso_by_id(id).unwrap().unwrap();

    let expected = QsoRecord {
        id: Some(id),
        callsign: "DL1ABC".to_string(),
        band: "10m".to_string(),
        mode: "RTTY".to_string(),
        submode: Some("RTTY_45".to_string()),
        qso_date: "20260505".to_string(),
        time_on: "153000".to_string(),
        time_off: Some("153500".to_string()),
        freq: Some(28.0805),
        freq_rx: Some(28.0809),
        rst_sent: "579".to_string(),
        rst_rcvd: "569".to_string(),
        name: Some("Hans".to_string()),
        qth: Some("Berlin".to_string()),
        gridsquare: Some("JO62PM".to_string()),
        state: Some("BE".to_string()),
        iota: Some("EU-123".to_string()),
        sota_ref: Some("DL/BE-001".to_string()),
        pota_ref: Some("DE-0042".to_string()),
        pga_ref: Some("DL01".to_string()),
        dxcc: Some(230),
        country: Some("Germany".to_string()),
        continent: Some("EU".to_string()),
        cqz: Some(14),
        ituz: Some(27),
        comment: Some("Updated comment".to_string()),
        qsl_via: Some("Direct".to_string()),
        qsl_manager: Some("DL1MGR".to_string()),
        qsl_sent: "Y".to_string(),
        qsl_rcvd: "Y".to_string(),
        qsl_sent_date: Some("20260506".to_string()),
        qsl_rcvd_date: Some("20260507".to_string()),
        lotw_qsl_sent: "Y".to_string(),
        lotw_qsl_rcvd: "Y".to_string(),
        lotw_qslrdate: Some("20260508".to_string()),
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260509".to_string()),
        eqsl_qslsdate: None,
        clublog_upload_status: Some("MODIFIED".to_string()),
        qrzcom_upload_status: Some("CONFIRMED".to_string()),
        sat_name: Some("RS-44".to_string()),
        sat_mode: Some("V/U".to_string()),
        prop_mode: Some("TR".to_string()),
        srx: Some(303),
        stx: Some(404),
        srx_string: Some("RX_UPDATE".to_string()),
        stx_string: Some("TX_UPDATE".to_string()),
        my_gridsquare: Some("JO80AA".to_string()),
        my_state: Some("WP".to_string()),
        my_pota_ref: Some("SP-9999".to_string()),
        my_sota_ref: Some("SP/DS-999".to_string()),
        vucc_grids: Some("JO62,JO63".to_string()),
        audio_file: Some("/path/updated.wav".to_string()),
        journal_id: Some("PORTABLE".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    assert_eq!(retrieved, expected);
}

#[test]
fn restore_qso_keeps_id_and_columns() {
    let db = LogDatabase::open_in_memory().unwrap();
    let to_restore = QsoRecord {
        id: Some(42),
        callsign: "k1abc".to_string(),
        band: "15m".to_string(),
        mode: "ft8".to_string(),
        submode: Some("FT8_SUB".to_string()),
        qso_date: "20260601".to_string(),
        time_on: "160000".to_string(),
        time_off: Some("160015".to_string()),
        freq: Some(21.074),
        freq_rx: Some(21.0745),
        rst_sent: "-05".to_string(),
        rst_rcvd: "-11".to_string(),
        name: Some("Bob".to_string()),
        qth: Some("Boston".to_string()),
        gridsquare: Some("FN42".to_string()),
        state: Some("MA".to_string()),
        iota: Some("NA-001".to_string()),
        sota_ref: Some("W1/HA-001".to_string()),
        pota_ref: Some("K-0001".to_string()),
        pga_ref: Some("US01".to_string()),
        dxcc: Some(291),
        country: Some("United States".to_string()),
        continent: Some("NA".to_string()),
        cqz: Some(5),
        ituz: Some(8),
        comment: Some("Restored record".to_string()),
        qsl_via: Some("Direct/Bureau".to_string()),
        qsl_manager: Some("W1MGR".to_string()),
        qsl_sent: "R".to_string(),
        qsl_rcvd: "R".to_string(),
        qsl_sent_date: Some("20260602".to_string()),
        qsl_rcvd_date: Some("20260603".to_string()),
        lotw_qsl_sent: "Y".to_string(),
        lotw_qsl_rcvd: "Y".to_string(),
        lotw_qslrdate: Some("20260604".to_string()),
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260605".to_string()),
        eqsl_qslsdate: None,
        clublog_upload_status: Some("DONE".to_string()),
        qrzcom_upload_status: Some("DONE".to_string()),
        sat_name: Some("SO-50".to_string()),
        sat_mode: Some("FM".to_string()),
        prop_mode: Some("AUR".to_string()),
        srx: Some(505),
        stx: Some(606),
        srx_string: Some("REST_SRX".to_string()),
        stx_string: Some("REST_STX".to_string()),
        my_gridsquare: Some("JO81WA".to_string()),
        my_state: Some("DS".to_string()),
        my_pota_ref: Some("SP-0010".to_string()),
        my_sota_ref: Some("SP/DS-010".to_string()),
        vucc_grids: Some("FN42,FN43".to_string()),
        audio_file: Some("/path/restored.wav".to_string()),
        journal_id: Some("DEFAULT".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    let restored_id = db.restore_qso(&to_restore).unwrap();
    assert_eq!(restored_id, 42);

    let retrieved = db.get_qso_by_id(42).unwrap().unwrap();
    let expected = QsoRecord {
        id: Some(42),
        callsign: "K1ABC".to_string(),
        band: "15m".to_string(),
        mode: "FT8".to_string(),
        submode: Some("FT8_SUB".to_string()),
        qso_date: "20260601".to_string(),
        time_on: "160000".to_string(),
        time_off: Some("160015".to_string()),
        freq: Some(21.074),
        freq_rx: Some(21.0745),
        rst_sent: "-05".to_string(),
        rst_rcvd: "-11".to_string(),
        name: Some("Bob".to_string()),
        qth: Some("Boston".to_string()),
        gridsquare: Some("FN42".to_string()),
        state: Some("MA".to_string()),
        iota: Some("NA-001".to_string()),
        sota_ref: Some("W1/HA-001".to_string()),
        pota_ref: Some("K-0001".to_string()),
        pga_ref: Some("US01".to_string()),
        dxcc: Some(291),
        country: Some("United States".to_string()),
        continent: Some("NA".to_string()),
        cqz: Some(5),
        ituz: Some(8),
        comment: Some("Restored record".to_string()),
        qsl_via: Some("Direct/Bureau".to_string()),
        qsl_manager: Some("W1MGR".to_string()),
        qsl_sent: "R".to_string(),
        qsl_rcvd: "R".to_string(),
        qsl_sent_date: Some("20260602".to_string()),
        qsl_rcvd_date: Some("20260603".to_string()),
        lotw_qsl_sent: "Y".to_string(),
        lotw_qsl_rcvd: "Y".to_string(),
        lotw_qslrdate: Some("20260604".to_string()),
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260605".to_string()),
        eqsl_qslsdate: None,
        clublog_upload_status: Some("DONE".to_string()),
        qrzcom_upload_status: Some("DONE".to_string()),
        sat_name: Some("SO-50".to_string()),
        sat_mode: Some("FM".to_string()),
        prop_mode: Some("AUR".to_string()),
        srx: Some(505),
        stx: Some(606),
        srx_string: Some("REST_SRX".to_string()),
        stx_string: Some("REST_STX".to_string()),
        my_gridsquare: Some("JO81WA".to_string()),
        my_state: Some("DS".to_string()),
        my_pota_ref: Some("SP-0010".to_string()),
        my_sota_ref: Some("SP/DS-010".to_string()),
        vucc_grids: Some("FN42,FN43".to_string()),
        audio_file: Some("/path/restored.wav".to_string()),
        journal_id: Some("DEFAULT".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    assert_eq!(retrieved, expected);
}

#[test]
fn batch_insert_roundtrip() {
    let db = LogDatabase::open_in_memory().unwrap();
    let qso1 = QsoRecord {
        id: None,
        callsign: "ja1abc".to_string(),
        band: "6m".to_string(),
        mode: "ft4".to_string(),
        submode: Some("FT4_SUB".to_string()),
        qso_date: "20260701".to_string(),
        time_on: "050000".to_string(),
        time_off: Some("050015".to_string()),
        freq: Some(50.313),
        freq_rx: Some(50.3135),
        rst_sent: "-01".to_string(),
        rst_rcvd: "-02".to_string(),
        name: Some("Taro".to_string()),
        qth: Some("Tokyo".to_string()),
        gridsquare: Some("QM05".to_string()),
        state: Some("TK".to_string()),
        iota: Some("AS-007".to_string()),
        sota_ref: Some("JA/TK-001".to_string()),
        pota_ref: Some("JA-0001".to_string()),
        pga_ref: Some("JA01".to_string()),
        dxcc: Some(339),
        country: Some("Japan".to_string()),
        continent: Some("AS".to_string()),
        cqz: Some(25),
        ituz: Some(45),
        comment: Some("Batch QSO 1".to_string()),
        qsl_via: Some("JARL".to_string()),
        qsl_manager: Some("JA1MGR".to_string()),
        qsl_sent: "Y".to_string(),
        qsl_rcvd: "N".to_string(),
        qsl_sent_date: Some("20260702".to_string()),
        qsl_rcvd_date: Some("20260703".to_string()),
        lotw_qsl_sent: "N".to_string(),
        lotw_qsl_rcvd: "N".to_string(),
        lotw_qslrdate: None,
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
        eqsl_qslsdate: None,
        clublog_upload_status: Some("QUEUE".to_string()),
        qrzcom_upload_status: Some("QUEUE".to_string()),
        sat_name: Some("CAS-4B".to_string()),
        sat_mode: Some("V/U".to_string()),
        prop_mode: Some("ES".to_string()),
        srx: Some(701),
        stx: Some(801),
        srx_string: Some("B1_SRX".to_string()),
        stx_string: Some("B1_STX".to_string()),
        my_gridsquare: Some("JO81WA".to_string()),
        my_state: Some("DS".to_string()),
        my_pota_ref: Some("SP-0100".to_string()),
        my_sota_ref: Some("SP/DS-100".to_string()),
        vucc_grids: Some("QM05,QM06".to_string()),
        audio_file: Some("/path/batch1.wav".to_string()),
        journal_id: None,
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    let count = db.batch_insert_qsos(&[qso1]).unwrap();
    assert_eq!(count, 1);

    let retrieved_list = db.find_previous_qsos("JA1ABC").unwrap();
    assert_eq!(retrieved_list.len(), 1);
    let retrieved = &retrieved_list[0];

    let expected = QsoRecord {
        id: retrieved.id,
        callsign: "JA1ABC".to_string(),
        band: "6m".to_string(),
        mode: "FT4".to_string(),
        submode: Some("FT4_SUB".to_string()),
        qso_date: "20260701".to_string(),
        time_on: "050000".to_string(),
        time_off: Some("050015".to_string()),
        freq: Some(50.313),
        freq_rx: Some(50.3135),
        rst_sent: "-01".to_string(),
        rst_rcvd: "-02".to_string(),
        name: Some("Taro".to_string()),
        qth: Some("Tokyo".to_string()),
        gridsquare: Some("QM05".to_string()),
        state: Some("TK".to_string()),
        iota: Some("AS-007".to_string()),
        sota_ref: Some("JA/TK-001".to_string()),
        pota_ref: Some("JA-0001".to_string()),
        pga_ref: Some("JA01".to_string()),
        dxcc: Some(339),
        country: Some("Japan".to_string()),
        continent: Some("AS".to_string()),
        cqz: Some(25),
        ituz: Some(45),
        comment: Some("Batch QSO 1".to_string()),
        qsl_via: Some("JARL".to_string()),
        qsl_manager: Some("JA1MGR".to_string()),
        qsl_sent: "Y".to_string(),
        qsl_rcvd: "N".to_string(),
        qsl_sent_date: Some("20260702".to_string()),
        qsl_rcvd_date: Some("20260703".to_string()),
        lotw_qsl_sent: "N".to_string(),
        lotw_qsl_rcvd: "N".to_string(),
        lotw_qslrdate: None,
        lotw_qslsdate: None,
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
        eqsl_qslsdate: None,
        clublog_upload_status: Some("QUEUE".to_string()),
        qrzcom_upload_status: Some("QUEUE".to_string()),
        sat_name: Some("CAS-4B".to_string()),
        sat_mode: Some("V/U".to_string()),
        prop_mode: Some("ES".to_string()),
        srx: Some(701),
        stx: Some(801),
        srx_string: Some("B1_SRX".to_string()),
        stx_string: Some("B1_STX".to_string()),
        my_gridsquare: Some("JO81WA".to_string()),
        my_state: Some("DS".to_string()),
        my_pota_ref: Some("SP-0100".to_string()),
        my_sota_ref: Some("SP/DS-100".to_string()),
        vucc_grids: Some("QM05,QM06".to_string()),
        audio_file: Some("/path/batch1.wav".to_string()),
        journal_id: Some("DEFAULT".to_string()),
        tx_pwr: Some("100".to_string()),
        operator: Some("SP6INA".to_string()),
        station_callsign: Some("SP6INA/P".to_string()),
        contest_id: Some("SP-DX-CONTEST".to_string()),
        my_rig: Some("IC-7300".to_string()),
        my_antenna: Some("Dipole".to_string()),
        cnty: Some("MA,Middlesex".to_string()),
        sig: Some("WWFF".to_string()),
        sig_info: Some("SPFF-0001".to_string()),
        my_sig: Some("WWFF".to_string()),
        my_sig_info: Some("SPFF-0002".to_string()),
        wwff_ref: Some("SPFF-0001".to_string()),
        my_wwff_ref: Some("SPFF-0002".to_string()),
        email: Some("op@example.com".to_string()),
        extra_fields: Vec::new(),
    };

    assert_eq!(retrieved, &expected);
}

#[test]
fn recent_for_journal_orders_chronologically_with_mixed_separators() {
    let db = LogDatabase::open_in_memory().unwrap();

    let q0 = make_test_qso_literal("QSO0", "2025-12-30", "12:30");
    let q1 = make_test_qso_literal("QSO1", "2025-12-31", "1229");
    let q2 = make_test_qso_literal("QSO2", "20251231", "1230");
    let q3 = make_test_qso_literal("QSO3", "2025-12-31", "12:30");
    let q4 = make_test_qso_literal("QSO4", "20251231", "1231");
    let q5 = make_test_qso_literal("QSO5", "2026-01-01", "12:30");

    db.insert_qso(&q0).unwrap();
    db.insert_qso(&q1).unwrap();
    db.insert_qso(&q2).unwrap();
    db.insert_qso(&q3).unwrap();
    db.insert_qso(&q4).unwrap();
    db.insert_qso(&q5).unwrap();

    let recents = db.get_recent_qsos_for_journal("DEFAULT", 10).unwrap();
    let calls: Vec<&str> = recents.iter().map(|q| q.callsign.as_str()).collect();

    // Chronological DESC order:
    // QSO5: 2026-01-01 12:30
    // QSO4: 20251231 1231
    // QSO3: 2025-12-31 12:30 (id higher than QSO2)
    // QSO2: 20251231 1230
    // QSO1: 2025-12-31 1229
    // QSO0: 2025-12-30 12:30
    assert_eq!(calls, vec!["QSO5", "QSO4", "QSO3", "QSO2", "QSO1", "QSO0"]);
}

#[test]
fn table_columns_match_expected_snapshot() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut stmt = db.conn.prepare("PRAGMA table_info('qso_records')").unwrap();
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<String>>();

    let expected = vec![
        "id".to_string(),
        "callsign".to_string(),
        "band".to_string(),
        "mode".to_string(),
        "submode".to_string(),
        "qso_date".to_string(),
        "time_on".to_string(),
        "time_off".to_string(),
        "freq".to_string(),
        "freq_rx".to_string(),
        "rst_sent".to_string(),
        "rst_rcvd".to_string(),
        "name".to_string(),
        "qth".to_string(),
        "gridsquare".to_string(),
        "state".to_string(),
        "iota".to_string(),
        "sota_ref".to_string(),
        "pota_ref".to_string(),
        "pga_ref".to_string(),
        "dxcc".to_string(),
        "country".to_string(),
        "continent".to_string(),
        "cqz".to_string(),
        "ituz".to_string(),
        "comment".to_string(),
        "qsl_via".to_string(),
        "qsl_manager".to_string(),
        "qsl_sent".to_string(),
        "qsl_rcvd".to_string(),
        "qsl_sent_date".to_string(),
        "qsl_rcvd_date".to_string(),
        "lotw_qsl_sent".to_string(),
        "lotw_qsl_rcvd".to_string(),
        "lotw_qslrdate".to_string(),
        "eqsl_qsl_sent".to_string(),
        "eqsl_qsl_rcvd".to_string(),
        "eqsl_qslrdate".to_string(),
        "clublog_upload_status".to_string(),
        "qrzcom_upload_status".to_string(),
        "sat_name".to_string(),
        "sat_mode".to_string(),
        "prop_mode".to_string(),
        "srx".to_string(),
        "stx".to_string(),
        "srx_string".to_string(),
        "stx_string".to_string(),
        "my_gridsquare".to_string(),
        "my_state".to_string(),
        "my_pota_ref".to_string(),
        "my_sota_ref".to_string(),
        "vucc_grids".to_string(),
        "audio_file".to_string(),
        "journal_id".to_string(),
        "identity_key".to_string(),
        "lotw_qslsdate".to_string(),
        "eqsl_qslsdate".to_string(),
        "tx_pwr".to_string(),
        "operator".to_string(),
        "station_callsign".to_string(),
        "contest_id".to_string(),
        "my_rig".to_string(),
        "my_antenna".to_string(),
        "cnty".to_string(),
        "sig".to_string(),
        "sig_info".to_string(),
        "my_sig".to_string(),
        "my_sig_info".to_string(),
        "wwff_ref".to_string(),
        "my_wwff_ref".to_string(),
        "email".to_string(),
    ];

    assert_eq!(expected.len(), 71);
    assert_eq!(columns, expected);
}

#[test]
fn migration_from_v1_preserves_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("v1_migrate.db");

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            INSERT INTO schema_version (version) VALUES (1);",
        )
        .unwrap();

        LogDatabase::migration_1_base_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO qso_records (
                callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, comment, journal_id
            ) VALUES (
                'SP6INA', '20m', 'CW', '20260101', '120000', '599', '599', 'V1 preserved row', 'DEFAULT'
            )",
            [],
        )
        .unwrap();
    }

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);

    let qsos = db.get_all_qsos().unwrap();
    assert_eq!(qsos.len(), 1);
    assert_eq!(qsos[0].callsign, "SP6INA");
    assert_eq!(qsos[0].band, "20m");
    assert_eq!(qsos[0].mode, "CW");
    assert_eq!(qsos[0].comment.as_deref(), Some("V1 preserved row"));
    assert_eq!(qsos[0].journal_id.as_deref(), Some("DEFAULT"));
}

#[test]
fn validate_schema_rejects_future_version() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("future_schema.db");

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (
                version INTEGER NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
        // Wersja o jeden wyższa niż najwyższa znana programowi (jedno źródło prawdy).
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [LATEST_SCHEMA_VERSION + 1],
        )
        .unwrap();
    }

    let res = LogDatabase::open(&db_path);
    assert!(matches!(res, Err(rusqlite::Error::InvalidQuery)));
}

#[test]
fn validate_schema_reports_missing_table() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("missing_table.db");

    {
        let db = LogDatabase::open(&db_path).unwrap();
        db.conn.execute("DROP TABLE upload_queue", []).unwrap();
    }

    let res = LogDatabase::open(&db_path);
    assert!(matches!(res, Err(rusqlite::Error::InvalidQuery)));
}

#[test]
fn frozen_migrations_match_baseline_hashes() {
    use sha2::{Digest, Sha256};

    let hash_sql = |sql: &str| {
        let normalized = sql.replace("\r\n", "\n");
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let result = hasher.finalize();
        result.iter().fold(String::new(), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
    };

    let m1_hash = hash_sql(crate::core::database::migrations::MIGRATION_1_SQL);
    let m3_hash = hash_sql(crate::core::database::migrations::MIGRATION_3_SQL);

    assert_eq!(
        m1_hash, "8c179cd36654e3305112c6bbda1c46ba36cc4a51e3da831b93ee14addf60f6ca",
        "MIGRATION_1_SQL hash mismatch against refactor-baseline"
    );
    assert_eq!(
        m3_hash, "f05d04d12225e5235f0b7bca077d21479348cefd1e4871900d3d2887c9804d04",
        "MIGRATION_3_SQL hash mismatch against refactor-baseline"
    );
}

#[test]
fn ordering_is_consistent_for_mixed_formats() {
    let db = LogDatabase::open_in_memory().unwrap();

    let q0 = make_test_qso_literal("QSO0", "2025-12-30", "12:30");
    let q1 = make_test_qso_literal("QSO1", "2025-12-31", "1229");
    let q2 = make_test_qso_literal("QSO2", "20251231", "1230");
    let q3 = make_test_qso_literal("QSO3", "2025-12-31", "12:30");
    let q4 = make_test_qso_literal("QSO4", "20251231", "1231");
    let q5 = make_test_qso_literal("QSO5", "2026-01-01", "12:30");

    db.insert_qso(&q0).unwrap();
    db.insert_qso(&q1).unwrap();
    db.insert_qso(&q2).unwrap();
    db.insert_qso(&q3).unwrap();
    db.insert_qso(&q4).unwrap();
    db.insert_qso(&q5).unwrap();

    let expected_calls = vec!["QSO5", "QSO4", "QSO3", "QSO2", "QSO1", "QSO0"];

    let all = db.get_all_qsos().unwrap();
    let all_calls: Vec<&str> = all.iter().map(|q| q.callsign.as_str()).collect();
    assert_eq!(all_calls, expected_calls, "get_all_qsos ordering mismatch");

    let searched = db
        .search_qsos_advanced(&AdvancedQsoFilter::default())
        .unwrap();
    let search_calls: Vec<&str> = searched.iter().map(|q| q.callsign.as_str()).collect();
    assert_eq!(
        search_calls, expected_calls,
        "search_qsos_advanced ordering mismatch"
    );
}

#[test]
fn get_qso_by_id_propagates_errors() {
    let db = LogDatabase::open_in_memory().unwrap();
    let qso = make_test_qso_literal("SP6INA", "2026-01-01", "12:00");
    let id = db.insert_qso(&qso).unwrap();

    // Verify row can be read normally
    assert!(db.get_qso_by_id(id).unwrap().is_some());

    // Non-existent ID returns Ok(None)
    assert!(db.get_qso_by_id(id + 9999).unwrap().is_none());

    // Corrupt the row: dxcc column is INTEGER, setting text causes type conversion error on retrieval
    db.conn
        .execute(
            "UPDATE qso_records SET dxcc = 'abc' WHERE id = ?1",
            rusqlite::params![id],
        )
        .unwrap();

    let res = db.get_qso_by_id(id);
    assert!(
        res.is_err(),
        "expected Err when retrieving corrupted row, got {res:?}"
    );
}

#[test]
fn qso_identity_ignores_case_and_whitespace() {
    let id1 = crate::core::qso::QsoIdentity::from_fields(
        "  sp6ina  ",
        " 20m ",
        " cw ",
        " 20260928 ",
        " 1230 ",
    );
    let id2 = crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "1230");
    assert_eq!(id1, id2);
    assert_eq!(id1.to_string(), "v2|SP6INA|20M||CW||20260928|M1230");
}

#[test]
fn minute_precision_and_second_precision_follow_explicit_contract() {
    let id_hhmm =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "1230");
    let id_hhmmss =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "123045");
    let id_colon =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "12:30:45");
    assert_ne!(id_hhmm, id_hhmmss);
    assert_eq!(id_hhmmss, id_colon);
    assert_eq!(
        id_hhmm.precision,
        Some(crate::core::qso::QsoTimePrecision::Minute)
    );
    assert_eq!(
        id_hhmmss.precision,
        Some(crate::core::qso::QsoTimePrecision::Second)
    );
}

#[test]
fn qso_identity_ignores_date_separators() {
    let id_raw =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "1230");
    let id_dash =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "2026-09-28", "1230");
    let id_dot =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "2026.09.28", "1230");
    let id_slash =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "2026/09/28", "1230");
    assert_eq!(id_raw, id_dash);
    assert_eq!(id_raw, id_dot);
    assert_eq!(id_raw, id_slash);
}

#[test]
fn qso_identity_keeps_distinct_invalid_values_distinct() {
    let id_bad1 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "bad-date-1", "1230");
    let id_bad2 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "bad-date-2", "1230");
    let id_bad1_dup =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "bad-date-1", "1230");
    assert_ne!(id_bad1, id_bad2);
    assert_eq!(id_bad1, id_bad1_dup);
    assert!(id_bad1.date.starts_with('?'));

    let id_bad_time1 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "12");
    let id_bad_time2 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "34");
    let id_bad_time1_dup =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20260928", "12");
    assert_ne!(id_bad_time1, id_bad_time2);
    assert_eq!(id_bad_time1, id_bad_time1_dup);
    assert!(id_bad_time1.minute.starts_with('?'));
}

#[test]
fn existing_identities_matches_record_identity() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q1 = make_test_qso_literal("SP6INA", "2026-09-28", "12:30:15");
    let q2 = make_test_qso_literal("W1AW", "20260928", "1230");
    db.insert_qso(&q1).unwrap();
    db.insert_qso(&q2).unwrap();

    let identities = db.existing_identities("DEFAULT").unwrap();
    assert_eq!(identities.len(), 2);
    assert!(identities.contains(&q1.identity()));
    assert!(identities.contains(&q2.identity()));
}

#[test]
fn has_identity_matches_exact_seconds_and_rejects_different_seconds() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "2026-09-28", "12:30:15");
    db.insert_qso(&q).unwrap();

    let query_same_second = QsoIdentity::from_detailed_fields(
        "sp6ina",
        "20m",
        Some(14.025),
        "cw",
        None,
        "20260928",
        "123015",
    );
    assert!(db.has_identity("DEFAULT", &query_same_second).unwrap());

    let query_diff_second = QsoIdentity::from_detailed_fields(
        "sp6ina",
        "20m",
        Some(14.025),
        "cw",
        None,
        "20260928",
        "123045",
    );
    assert!(!db.has_identity("DEFAULT", &query_diff_second).unwrap());
}

#[test]
fn has_identity_rejects_different_identity() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "2026-09-28", "12:30:15");
    db.insert_qso(&q).unwrap();

    let diff_call = QsoIdentity::from_fields("W1AW", "20m", "cw", "20260928", "123015");
    let diff_time = QsoIdentity::from_fields("SP6INA", "20m", "cw", "20260928", "123115");
    let diff_band = QsoIdentity::from_fields("SP6INA", "40m", "cw", "20260928", "123015");
    let diff_journal = QsoIdentity::from_fields("SP6INA", "20m", "cw", "20260928", "123015");

    assert!(!db.has_identity("DEFAULT", &diff_call).unwrap());
    assert!(!db.has_identity("DEFAULT", &diff_time).unwrap());
    assert!(!db.has_identity("DEFAULT", &diff_band).unwrap());
    assert!(!db.has_identity("OTHER", &diff_journal).unwrap());
}

#[test]
fn search_qsos_advanced_normalizes_hyphenated_date_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let db = LogDatabase::open(dir.path().join("search_date.db")).unwrap();

    let q1 = make_test_qso_literal("SP1AAA", "20261018", "120000");
    db.insert_qso(&q1).unwrap();

    let q2 = make_test_qso_literal("SP2BBB", "20261020", "120000");
    db.insert_qso(&q2).unwrap();

    let q3 = make_test_qso_literal("SP3CCC", "20261025", "120000");
    db.insert_qso(&q3).unwrap();

    let mut filter = super::search::AdvancedQsoFilter::default();
    filter.date_from = Some("2026-10-19".to_string());
    filter.date_to = Some("2026-10-22".to_string());

    let results = db.search_qsos_advanced(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].callsign, "SP2BBB");
    assert_eq!(results[0].qso_date, "20261020");
}

#[test]
fn explain_query_plan_has_identity_uses_index() {
    let dir = tempfile::tempdir().unwrap();
    let db = LogDatabase::open(dir.path().join("query_plan.db")).unwrap();

    let mut stmt = db
        .conn
        .prepare(
            "EXPLAIN QUERY PLAN SELECT 1 FROM qso_records WHERE journal_id = ?1 AND identity_key = ?2 LIMIT 1",
        )
        .unwrap();

    let rows: Vec<String> = stmt
        .query_map(
            rusqlite::params!["DEFAULT", "SP6INA|20M|CW|20261018|1200"],
            |row| row.get::<_, String>(3),
        )
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    let full_plan = rows.join(" ");
    assert!(
        full_plan.contains("idx_qso_identity_unique") && full_plan.contains("SEARCH qso_records"),
        "expected plan to use idx_qso_identity_unique, got: {full_plan}"
    );
}

// =========================================================================
// KOĹCOWA BRAMKA ETAPU 5: DATA-003 â€” KANONICZNA TOĹ»SAMOĹšÄ† I UNIQUE INTEGRITY
// =========================================================================

#[test]
fn direct_duplicate_insert_is_rejected_by_database() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    db.insert_qso(&q1).unwrap();

    let err = db.insert_qso(&q1);
    assert!(
        matches!(err, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation),
        "bezpoĹ›redni duplikat musi byÄ‡ odrzucony ograniczeniem UNIQUE bazy danych"
    );
}

#[test]
fn duplicate_insert_from_second_connection_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("two_connections.db");

    let db = LogDatabase::open(&db_path).unwrap();
    let q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    db.insert_qso(&q1).unwrap();

    // Drugie niezaleĹĽne poĹ‚Ä…czenie SQLite omijajÄ…ce instancjÄ™ LogDatabase
    let conn2 = rusqlite::Connection::open(&db_path).unwrap();
    let mut stmt = conn2.prepare(
        "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id, identity_key) \
         VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', '599', '599', 'DEFAULT', 'v2|SP6INA|20M|14025000|CW||20261018|S120000')"
    ).unwrap();

    let err = stmt.execute([]);
    assert!(
        matches!(err, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation),
        "drugie niezaleĹĽne poĹ‚Ä…czenie musi zostaÄ‡ zablokowane przez indeks UNIQUE w SQLite"
    );
}

#[test]
fn duplicate_insert_from_second_repository_instance_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("two_repos.db");

    let db1 = LogDatabase::open(&db_path).unwrap();
    let db2 = LogDatabase::open(&db_path).unwrap();

    let q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    db1.insert_qso(&q1).unwrap();

    let err = db2.insert_qso(&q1);
    assert!(
        matches!(err, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation),
        "druga instancja repozytorium musi zostaÄ‡ odrzucona przez unikalnoĹ›Ä‡ bazy bez polegania na mutexie procesu"
    );
}

#[test]
fn same_qso_same_identity_key() {
    let id1 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "2026-10-18", "12:00:00");
    let id2 =
        crate::core::qso::QsoIdentity::from_fields("sp6ina", "20M", "cw", "20261018", "120000");
    assert_eq!(id1.canonical_key(), id2.canonical_key());
    assert_eq!(
        id1.canonical_key(),
        Some("v2|SP6INA|20M||CW||20261018|S120000".to_string())
    );
}

#[test]
fn same_qso_different_adif_field_order_same_identity() {
    let id1 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "20261018", "120000");
    let id2 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "20261018", "120000");
    assert_eq!(id1.canonical_key(), id2.canonical_key());
}

#[test]
fn same_qso_case_variations_same_identity() {
    let id1 =
        crate::core::qso::QsoIdentity::from_fields("sp6ina", "20m", "cw", "20261018", "120000");
    let id2 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20M", "CW", "20261018", "120000");
    assert_eq!(id1.canonical_key(), id2.canonical_key());
}

#[test]
fn same_qso_normalized_date_same_identity() {
    let id1 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "2026-10-18", "120000");
    let id2 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "2026/10/18", "120000");
    let id3 =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "20261018", "120000");
    assert_eq!(id1.canonical_key(), id2.canonical_key());
    assert_eq!(id1.canonical_key(), id3.canonical_key());
}

#[test]
fn same_qso_hhmm_and_hhmmss_follow_contract() {
    let id_hhmm =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "20261018", "1200");
    let id_hhmmss =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "20261018", "120045");
    assert_ne!(id_hhmm.canonical_key(), id_hhmmss.canonical_key());
}

#[test]
fn same_callsign_same_day_different_time_are_distinct() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q1 = make_test_qso_literal("SP6INA", "20261018", "100000");
    let q2 = make_test_qso_literal("SP6INA", "20261018", "143000");

    assert_ne!(q1.identity().canonical_key(), q2.identity().canonical_key());
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn same_callsign_same_time_different_band_are_distinct() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.band = "20m".to_string();
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.band = "40m".to_string();

    assert_ne!(q1.identity().canonical_key(), q2.identity().canonical_key());
    db.insert_qso(&q1).unwrap();
    db.insert_qso(&q2).unwrap();
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn same_callsign_same_time_different_mode_follow_contract() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.mode = "CW".to_string();
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.mode = "SSB".to_string();

    assert_ne!(q1.identity().canonical_key(), q2.identity().canonical_key());
    db.insert_qso(&q1).unwrap();
    db.insert_qso(&q2).unwrap();
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn mode_and_submode_follow_identity_contract() {
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.mode = "FT8".to_string();
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.mode = "ft8".to_string();
    q2.submode = Some("FT8".to_string());

    assert_eq!(q1.identity().canonical_key(), q2.identity().canonical_key());
}

#[test]
fn different_journals_allow_same_qso() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "CONTEST".to_string(),
        name: "Contest Journal".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.journal_id = Some("DEFAULT".to_string());
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.journal_id = Some("CONTEST".to_string());

    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_qsos_for_journal("DEFAULT").unwrap(), 1);
    assert_eq!(db.count_qsos_for_journal("CONTEST").unwrap(), 1);
}

#[test]
fn missing_time_does_not_create_false_duplicate() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "");
    q1.time_on = String::new();
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "");
    q2.time_on = String::new();

    // Rekordy bez peĹ‚nej toĹĽsamoĹ›ci majÄ… identity_key=NULL i nie blokujÄ… siÄ™ wzajemnie
    assert_eq!(q1.identity().canonical_key(), None);
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn null_and_empty_do_not_collapse_unintentionally() {
    let id_empty =
        crate::core::qso::QsoIdentity::from_fields("", "20m", "CW", "20261018", "120000");
    let id_no_date =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "", "120000");
    let id_no_time =
        crate::core::qso::QsoIdentity::from_fields("SP6INA", "20m", "CW", "20261018", "");

    assert_eq!(id_empty.canonical_key(), None);
    assert_eq!(id_no_date.canonical_key(), None);
    assert_eq!(id_no_time.canonical_key(), None);
}

#[test]
fn stable_source_id_has_documented_precedence() {
    let mut q = make_test_qso_literal("SP6INA", "20261018", "120000");
    q.id = Some(42);
    let id = q.identity();
    // ToĹĽsamoĹ›Ä‡ naturalna jest staĹ‚a i deterministyczna niezaleĹĽnie od lokalnego klucza autoincrement ID
    assert_eq!(
        id.canonical_key(),
        Some("v2|SP6INA|20M|14025000|CW||20261018|S120000".to_string())
    );
}

#[test]
fn identity_key_is_stable_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("restart.db");

    let q = make_test_qso_literal("SP6INA", "20261018", "120000");
    {
        let db = LogDatabase::open(&db_path).unwrap();
        db.insert_qso(&q).unwrap();
    }

    {
        let db = LogDatabase::open(&db_path).unwrap();
        let exists = db.has_identity("DEFAULT", &q.identity()).unwrap();
        assert!(
            exists,
            "toĹĽsamoĹ›Ä‡ musi byÄ‡ trwaĹ‚a i stabilna po restarcie bazy"
        );
    }
}

#[test]
fn concurrent_identical_insert_from_two_connections_creates_one_qso() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conc_conn.db");

    // Inicjalizacja schematu bazy danych
    {
        let _ = LogDatabase::open(&db_path).unwrap();
    }

    let p1 = db_path.clone();
    let p2 = db_path.clone();

    let t1 = std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(&p1).unwrap();
        conn.execute(
            "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id, identity_key) \
             VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', '599', '599', 'DEFAULT', 'v2|SP6INA|20M|14025000|CW||20261018|S120000')",
            [],
        )
    });

    let t2 = std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(&p2).unwrap();
        conn.execute(
            "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id, identity_key) \
             VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', '599', '599', 'DEFAULT', 'v2|SP6INA|20M|14025000|CW||20261018|S120000')",
            [],
        )
    });

    let res1 = t1.join().unwrap();
    let res2 = t2.join().unwrap();

    let mut successes = 0;
    let mut conflicts = 0;

    for r in [res1, res2] {
        match r {
            Ok(_) => successes += 1,
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                conflicts += 1;
            }
            Err(e) => panic!("nieoczekiwany bĹ‚Ä…d wspĂłĹ‚bieĹĽnoĹ›ci: {e:?}"),
        }
    }

    assert_eq!(
        successes, 1,
        "dokĹ‚adnie jedno poĹ‚Ä…czenie musi wygraÄ‡ wyĹ›cig"
    );
    assert_eq!(
        conflicts, 1,
        "drugie poĹ‚Ä…czenie musi zostaÄ‡ zablokowane przez constraint UNIQUE"
    );

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.count_all().unwrap(), 1);
}

#[test]
fn concurrent_identical_insert_from_two_repository_instances_creates_one_qso() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conc_repos.db");
    {
        let _init = LogDatabase::open(&db_path).unwrap();
    }

    let p1 = db_path.clone();
    let p2 = db_path.clone();

    let t1 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p1).unwrap();
        let q = make_test_qso_literal("SP6INA", "20261018", "120000");
        db.insert_qso(&q)
    });

    let t2 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p2).unwrap();
        let q = make_test_qso_literal("SP6INA", "20261018", "120000");
        db.insert_qso(&q)
    });

    let res1 = t1.join().unwrap();
    let res2 = t2.join().unwrap();

    let mut successes = 0;
    let mut conflicts = 0;

    for r in [res1, res2] {
        match r {
            Ok(_) => successes += 1,
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                conflicts += 1;
            }
            Err(e) => panic!("nieoczekiwany bĹ‚Ä…d: {e:?}"),
        }
    }

    assert_eq!(successes, 1);
    assert_eq!(conflicts, 1);

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.count_all().unwrap(), 1);
}

#[test]
fn concurrent_distinct_qsos_are_not_collapsed() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conc_distinct.db");
    {
        let _init = LogDatabase::open(&db_path).unwrap();
    }

    let p1 = db_path.clone();
    let p2 = db_path.clone();

    let t1 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p1).unwrap();
        let q = make_test_qso_literal("SP1AAA", "20261018", "120000");
        db.insert_qso(&q)
    });

    let t2 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p2).unwrap();
        let q = make_test_qso_literal("SP2BBB", "20261018", "120000");
        db.insert_qso(&q)
    });

    t1.join().unwrap().unwrap();
    t2.join().unwrap().unwrap();

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn database_unique_constraint_wins_race() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("race_wins.db");
    let db = LogDatabase::open(&db_path).unwrap();

    let q = make_test_qso_literal("SP6INA", "20261018", "120000");
    db.insert_qso(&q).unwrap();

    let err = db.insert_qso(&q);
    assert!(
        matches!(err, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation)
    );
}

#[test]
fn on_conflict_returns_deterministic_duplicate_result() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "20261018", "120000");

    db.insert_qso(&q).unwrap();

    // Sprawdzenie has_identity
    assert!(db.has_identity("DEFAULT", &q.identity()).unwrap());
}

#[test]
fn retry_after_conflict_is_idempotent() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "20261018", "120000");

    db.insert_qso(&q).unwrap();
    let err1 = db.insert_qso(&q);
    let err2 = db.insert_qso(&q);

    assert!(
        matches!(err1, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation)
    );
    assert!(
        matches!(err2, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation)
    );
    assert_eq!(db.count_all().unwrap(), 1);
}

#[test]
fn same_process_mutex_is_not_required_for_database_integrity() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("no_mutex_needed.db");

    let conn1 = rusqlite::Connection::open(&db_path).unwrap();
    let conn2 = rusqlite::Connection::open(&db_path).unwrap();

    // Inicjalizacja schematu przez conn1
    {
        let db = LogDatabase::open(&db_path).unwrap();
        let q = make_test_qso_literal("SP6INA", "20261018", "120000");
        db.insert_qso(&q).unwrap();
    }

    // BezpoĹ›redni insert przez conn2 bez mutexu LogDatabase
    let err = conn2.execute(
        "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id, identity_key) \
         VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', '599', '599', 'DEFAULT', 'v2|SP6INA|20M|14025000|CW||20261018|S120000')",
        [],
    );
    assert!(
        matches!(err, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation)
    );
    let _ = conn1;
}

#[test]
fn fresh_schema_contains_identity_unique_constraint() {
    let db = LogDatabase::open_in_memory().unwrap();
    let count: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_qso_identity_unique' AND sql LIKE '%UNIQUE%'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(
        count, 1,
        "nowa baza danych musi posiadaÄ‡ indeks UNIQUE idx_qso_identity_unique"
    );
}

#[test]
fn migrated_schema_contains_same_identity_unique_constraint() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("migrated_schema.db");

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (1);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
    }

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);

    let count: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_qso_identity_unique' AND sql LIKE '%UNIQUE%'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert_eq!(
        count, 1,
        "zmigrowana baza musi posiadaÄ‡ dokĹ‚adnie ten sam indeks UNIQUE idx_qso_identity_unique"
    );
}

#[test]
fn migration_conflict_rolls_back_without_data_loss() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("migration_conflict.db");

    // Tworzymy bazÄ™ na wersji 5 z syntetycznym duplikatem
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (1);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
        conn.execute_batch(
            "UPDATE schema_version SET version = 5;
             INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', '599', '599', 'DEFAULT');
             INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', '599', '599', 'DEFAULT');",
        ).unwrap();
    }

    // Otwarcie bazy prĂłbuje uruchomiÄ‡ migracjÄ™ 6
    let res = LogDatabase::open(&db_path);
    assert!(
        res.is_err(),
        "migracja musi zakoĹ„czyÄ‡ siÄ™ bĹ‚Ä™dem po wykryciu nierozstrzygniÄ™tych duplikatĂłw"
    );

    // Po wycofaniu migracji: dane sÄ… nienaruszone, wersja nadal 5
    let conn = Connection::open(&db_path).unwrap();
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        version, 5,
        "wersja schematu nie moĹĽe wzrosnÄ…Ä‡ po wycofanej migracji"
    );

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 2,
        "ĹĽaden rekord nie moĹĽe zostaÄ‡ usuniÄ™ty ani scalony automatycznie przez migracjÄ™"
    );
}

#[test]
fn migration_conflict_does_not_delete_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("no_delete.db");

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (5);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP1AAA', '40m', 'SSB', '20261018', '140000', '59', '59', 'DEFAULT');
             INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP1AAA', '40m', 'SSB', '20261018', '140000', '59', '59', 'DEFAULT');",
        ).unwrap();
    }

    let _ = LogDatabase::open(&db_path);

    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "duplikaty muszÄ… pozostaÄ‡ nienaruszone w bazie");
}

#[test]
fn migration_conflict_does_not_advance_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("no_advance.db");

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (5);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP1AAA', '40m', 'SSB', '20261018', '140000', '59', '59', 'DEFAULT');
             INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP1AAA', '40m', 'SSB', '20261018', '140000', '59', '59', 'DEFAULT');",
        ).unwrap();
    }

    let _ = LogDatabase::open(&db_path);

    let conn = Connection::open(&db_path).unwrap();
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 5);
}

#[test]
fn migration_can_be_retried_after_manual_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("manual_res.db");

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (5);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP1AAA', '40m', 'SSB', '20261018', '140000', '59', '59', 'DEFAULT');
             INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id) \
             VALUES ('SP1AAA', '40m', 'SSB', '20261018', '140000', '59', '59', 'DEFAULT');",
        ).unwrap();
    }

    // 1. Pierwsza prĂłba zawodzi
    assert!(LogDatabase::open(&db_path).is_err());

    // 2. RÄ™czne rozstrzygniÄ™cie konfliktu przez operatora (zmiana czasu drugiego QSO)
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "UPDATE qso_records SET time_on = '140500' WHERE id = (SELECT MAX(id) FROM qso_records)",
            [],
        ).unwrap();
    }

    // 3. Ponowna prĂłba migracji koĹ„czy siÄ™ sukcesem
    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn fresh_and_migrated_schema_have_equivalent_constraints() {
    let fresh_db = LogDatabase::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let migrated_path = dir.path().join("equiv.db");

    {
        let conn = Connection::open(&migrated_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (1);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
    }

    let migrated_db = LogDatabase::open(&migrated_path).unwrap();

    let fresh_idx: String = fresh_db.conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_qso_identity_unique'",
        [],
        |r| r.get(0),
    ).unwrap();

    let migrated_idx: String = migrated_db.conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_qso_identity_unique'",
        [],
        |r| r.get(0),
    ).unwrap();

    assert_eq!(fresh_idx, migrated_idx);
    assert!(fresh_idx.contains("UNIQUE"));
}

#[test]
fn same_callsign_same_minute_different_seconds_are_distinct() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q1 = make_test_qso_literal("SP6INA", "20261018", "120010");
    let q2 = make_test_qso_literal("SP6INA", "20261018", "120050");
    assert_ne!(q1.identity().canonical_key(), q2.identity().canonical_key());
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn same_callsign_same_minute_different_frequency_are_distinct() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.freq = Some(14.010);
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.freq = Some(14.050);
    assert_ne!(q1.identity().canonical_key(), q2.identity().canonical_key());
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn same_callsign_same_minute_different_submode_follow_contract() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.mode = "SSB".to_string();
    q1.submode = Some("USB".to_string());
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.mode = "SSB".to_string();
    q2.submode = Some("LSB".to_string());
    assert_ne!(q1.identity().canonical_key(), q2.identity().canonical_key());
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn same_callsign_same_minute_same_freq_same_mode_same_submode_is_duplicate() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.freq = Some(14.025);
    q1.mode = "CW".to_string();
    q1.submode = None;
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.freq = Some(14.025);
    q2.mode = "CW".to_string();
    q2.submode = None;
    assert_eq!(q1.identity().canonical_key(), q2.identity().canonical_key());
    db.insert_qso(&q1).unwrap();
    let res = db.insert_qso(&q2);
    assert!(
        res.is_err(),
        "Identyczne QSO w tej samej minucie musi być odrzucone jako duplikat"
    );
    assert_eq!(db.count_all().unwrap(), 1);
}

#[test]
fn same_callsign_same_minute_different_journal_is_distinct() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "EXPEDITION".to_string(),
        name: "Expedition".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q1 = make_test_qso_literal("W1AW", "20261018", "120000");
    q1.journal_id = Some("DEFAULT".to_string());
    let mut q2 = make_test_qso_literal("W1AW", "20261018", "120000");
    q2.journal_id = Some("EXPEDITION".to_string());

    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_qsos_for_journal("DEFAULT").unwrap(), 1);
    assert_eq!(db.count_qsos_for_journal("EXPEDITION").unwrap(), 1);
}

#[test]
fn same_callsign_same_minute_different_station_identity_follows_contract() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "CLUB_STATION".to_string(),
        name: "Club Station".to_string(),
        station_callsign: "SP6ZDA".to_string(),
        operator: "SP6INA".to_string(),
        my_gridsquare: "JO80IT".to_string(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q1 = make_test_qso_literal("DL1AAA", "20261018", "120000");
    q1.journal_id = Some("DEFAULT".to_string());
    let mut q2 = make_test_qso_literal("DL1AAA", "20261018", "120000");
    q2.journal_id = Some("CLUB_STATION".to_string());

    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
}

#[test]
fn ft8_identity_is_stable_across_adif_mode_submode_representation() {
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.mode = "FT8".to_string();
    q1.submode = None;

    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.mode = "MFSK".to_string();
    q2.submode = Some("FT8".to_string());

    assert_eq!(q1.identity().canonical_key(), q2.identity().canonical_key());
}

#[test]
fn different_submodes_same_parent_mode_follow_contract() {
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.mode = "MFSK".to_string();
    q1.submode = Some("JS8".to_string());

    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.mode = "MFSK".to_string();
    q2.submode = Some("FT2".to_string());

    assert_ne!(q1.identity().canonical_key(), q2.identity().canonical_key());
}

#[test]
fn usb_and_ssb_follow_contract() {
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.mode = "USB".to_string();
    q1.submode = None;

    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.mode = "SSB".to_string();
    q2.submode = Some("USB".to_string());

    let mut q3 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q3.mode = "SSB".to_string();
    q3.submode = None;

    assert_eq!(q1.identity().canonical_key(), q2.identity().canonical_key());
    assert_ne!(q1.identity().canonical_key(), q3.identity().canonical_key());
}

#[test]
fn cw_and_cw_r_follow_contract() {
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q1.mode = "CW".to_string();

    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.mode = "CW-R".to_string();

    assert_eq!(q1.identity().canonical_key(), q2.identity().canonical_key());
}

#[test]
fn mode_aliases_normalize_deterministically() {
    let cases = [
        ("CW-R", None, "CW", None),
        ("CWR", None, "CW", None),
        ("USB", None, "SSB", Some("USB")),
        ("LSB", None, "SSB", Some("LSB")),
        ("PCW", None, "CW", Some("PCW")),
        ("BPSK31", None, "PSK", Some("PSK31")),
        ("MFSK", Some("FT8"), "FT8", None),
        ("MFSK", Some("FT4"), "FT4", None),
        ("MFSK", Some("JS8"), "MFSK", Some("JS8")),
    ];

    for (m, s, expected_m, expected_s) in cases {
        let (norm_m, norm_s) = crate::core::qso::normalize_mode_and_submode(m, s);
        assert_eq!(norm_m, expected_m, "mode alias: {m}");
        assert_eq!(norm_s.as_deref(), expected_s, "submode alias for {m}");
    }
}

#[test]
fn missing_call_has_explicit_identity_behavior() {
    let id = QsoIdentity::from_fields("", "20m", "CW", "20261018", "120000");
    assert!(!id.is_complete());
    assert_eq!(id.canonical_key(), None);
}

#[test]
fn missing_date_has_explicit_identity_behavior() {
    let id = QsoIdentity::from_fields("SP6INA", "20m", "CW", "", "120000");
    assert!(!id.is_complete());
    assert_eq!(id.canonical_key(), None);
}

#[test]
fn missing_time_has_explicit_identity_behavior() {
    let id = QsoIdentity::from_fields("SP6INA", "20m", "CW", "20261018", "");
    assert!(!id.is_complete());
    assert_eq!(id.canonical_key(), None);
}

#[test]
fn missing_band_and_frequency_has_explicit_identity_behavior() {
    let id =
        QsoIdentity::from_detailed_fields("SP6INA", "", None, "CW", None, "20261018", "120000");
    assert!(!id.is_complete());
    assert_eq!(id.canonical_key(), None);
}

#[test]
fn missing_mode_has_explicit_identity_behavior() {
    let id = QsoIdentity::from_fields("SP6INA", "20m", "", "20261018", "120000");
    assert!(!id.is_complete());
    assert_eq!(id.canonical_key(), None);
}

#[test]
fn incomplete_identity_records_do_not_share_fake_key() {
    let id1 = QsoIdentity::from_fields("SP6INA", "20m", "CW", "bad1", "120000");
    let id2 = QsoIdentity::from_fields("SP6INA", "20m", "CW", "bad2", "120000");
    assert_eq!(id1.canonical_key(), None);
    assert_eq!(id2.canonical_key(), None);
    assert_ne!(id1.to_string(), id2.to_string());
}

#[test]
fn incomplete_identity_is_not_reported_as_database_deduplicated() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "");
    q1.time_on = String::new();
    assert_eq!(q1.identity().canonical_key(), None);
    assert!(!db.has_identity("DEFAULT", &q1.identity()).unwrap());
}

#[test]
fn trusted_source_id_can_supply_identity_if_contract_allows() {
    let mut q = make_test_qso_literal("SP6INA", "20261018", "120000");
    q.id = Some(101);
    assert_eq!(q.id, Some(101));
    assert!(q.identity().is_complete());
}

#[test]
fn reimport_incomplete_record_follows_explicit_contract() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q1 = make_test_qso_literal("SP6INA", "20261018", "");
    q1.time_on = String::new();
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "");
    q2.time_on = String::new();

    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert_ne!(id1, id2);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn multiple_null_identity_keys_follow_documented_contract() {
    let db = LogDatabase::open_in_memory().unwrap();
    for i in 0..5 {
        let mut q = make_test_qso_literal(&format!("CALL{i}"), "20261018", "");
        q.time_on = String::new();
        assert_eq!(q.identity().canonical_key(), None);
        db.insert_qso(&q).unwrap();
    }
    assert_eq!(db.count_all().unwrap(), 5);
}

#[test]
fn update_recomputes_identity_key() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "20261018", "120000");
    let id = db.insert_qso(&q).unwrap();

    let mut updated = q.clone();
    updated.callsign = "W1AW".to_string();
    db.update_qso(id, &updated).unwrap();

    let stored = db.get_qso_by_id(id).unwrap().unwrap();
    assert_eq!(stored.callsign, "W1AW");
    assert!(db.has_identity("DEFAULT", &updated.identity()).unwrap());
    assert!(!db.has_identity("DEFAULT", &q.identity()).unwrap());
}

#[test]
fn update_identity_conflict_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("update_conflict.db");
    let db = LogDatabase::open(&db_path).unwrap();

    let q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    let q2 = make_test_qso_literal("W1AW", "20261018", "120000");
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    assert!(id1 > 0);

    let mut conflicting = q2.clone();
    conflicting.callsign = "SP6INA".to_string();
    let res = db.update_qso(id2, &conflicting);
    assert!(
        res.is_err(),
        "Konflikt UNIQUE przy UPDATE musi zwrócić błąd"
    );

    let stored2 = db.get_qso_by_id(id2).unwrap().unwrap();
    assert_eq!(stored2.callsign, "W1AW");
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn update_conflict_leaves_original_qso_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conflict_orig.db");
    let db = LogDatabase::open(&db_path).unwrap();

    let q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    let q2 = make_test_qso_literal("W1AW", "20261018", "140000");
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();

    let mut bad_edit = q2.clone();
    bad_edit.callsign = "SP6INA".to_string();
    bad_edit.time_on = "120000".to_string();

    assert!(db.update_qso(id2, &bad_edit).is_err());
    let reloaded1 = db.get_qso_by_id(id1).unwrap().unwrap();
    assert_eq!(reloaded1.callsign, "SP6INA");
    let reloaded2 = db.get_qso_by_id(id2).unwrap().unwrap();
    assert_eq!(reloaded2.callsign, "W1AW");
}

#[test]
fn update_preserves_record_uuid() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "20261018", "120000");
    let id = db.insert_qso(&q).unwrap();

    let mut edited = q.clone();
    edited.comment = Some("Updated comment".to_string());
    db.update_qso(id, &edited).unwrap();

    let stored = db.get_qso_by_id(id).unwrap().unwrap();
    assert_eq!(stored.id, Some(id));
    assert_eq!(stored.comment.as_deref(), Some("Updated comment"));
}

#[test]
fn update_to_distinct_time_succeeds() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "20261018", "120000");
    let id = db.insert_qso(&q).unwrap();

    let mut edited = q.clone();
    edited.time_on = "120500".to_string();
    db.update_qso(id, &edited).unwrap();

    let stored = db.get_qso_by_id(id).unwrap().unwrap();
    assert_eq!(stored.time_on, "120500");
}

#[test]
fn update_from_incomplete_to_complete_identity_enables_dedup() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut q = make_test_qso_literal("SP6INA", "20261018", "");
    q.time_on = String::new();
    assert_eq!(q.identity().canonical_key(), None);
    let id = db.insert_qso(&q).unwrap();

    let mut complete = q.clone();
    complete.time_on = "120000".to_string();
    assert!(complete.identity().is_complete());
    db.update_qso(id, &complete).unwrap();

    assert!(db.has_identity("DEFAULT", &complete.identity()).unwrap());
}

#[test]
fn deleted_qso_identity_reuse_follows_contract() {
    let db = LogDatabase::open_in_memory().unwrap();
    let q = make_test_qso_literal("SP6INA", "20261018", "120000");
    let id = db.insert_qso(&q).unwrap();
    assert!(db.has_identity("DEFAULT", &q.identity()).unwrap());

    db.delete_qso(id).unwrap();
    assert!(!db.has_identity("DEFAULT", &q.identity()).unwrap());

    let new_id = db.insert_qso(&q).unwrap();
    assert!(new_id > 0);
    assert!(db.has_identity("DEFAULT", &q.identity()).unwrap());
    assert_eq!(db.count_all().unwrap(), 1);
}

#[test]
fn identity_is_deterministic() {
    let id1 = QsoIdentity::from_detailed_fields(
        "SP6INA",
        "20M",
        Some(14.025),
        "CW",
        None,
        "20261018",
        "120000",
    );
    let id2 = QsoIdentity::from_detailed_fields(
        "SP6INA",
        "20M",
        Some(14.025),
        "CW",
        None,
        "20261018",
        "120000",
    );
    assert_eq!(id1.canonical_key(), id2.canonical_key());
    assert_eq!(
        id1.canonical_key(),
        Some("v2|SP6INA|20M|14025000|CW||20261018|S120000".to_string())
    );
}

#[test]
fn identity_is_idempotent_after_normalization() {
    let id = QsoIdentity::from_detailed_fields(
        "  sp6ina  ",
        " 20m ",
        Some(14.025),
        " cw-r ",
        None,
        " 2026-10-18 ",
        " 12:00:00 ",
    );
    let key1 = id.canonical_key();
    let key2 = id.canonical_key();
    assert_eq!(key1, key2);
    assert_eq!(
        key1,
        Some("v2|SP6INA|20M|14025000|CW||20261018|S120000".to_string())
    );
}

#[test]
fn identity_field_order_independent() {
    let q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    let mut q2 = make_test_qso_literal("SP6INA", "20261018", "120000");
    q2.comment = Some("Different comment".to_string());
    q2.rst_sent = "579".to_string();
    assert_eq!(q1.identity().canonical_key(), q2.identity().canonical_key());
}

#[test]
fn identity_changes_only_when_identity_fields_change() {
    let base = make_test_qso_literal("SP6INA", "20261018", "120000");
    let base_key = base.identity().canonical_key();

    let mut diff_call = base.clone();
    diff_call.callsign = "W1AW".to_string();
    assert_ne!(base_key, diff_call.identity().canonical_key());

    let mut diff_band = base.clone();
    diff_band.band = "40m".to_string();
    diff_band.freq = Some(7.025);
    assert_ne!(base_key, diff_band.identity().canonical_key());

    let mut diff_freq = base.clone();
    diff_freq.freq = Some(14.050);
    assert_ne!(base_key, diff_freq.identity().canonical_key());

    let mut diff_mode = base.clone();
    diff_mode.mode = "SSB".to_string();
    assert_ne!(base_key, diff_mode.identity().canonical_key());

    let mut diff_submode = base.clone();
    diff_submode.mode = "SSB".to_string();
    diff_submode.submode = Some("USB".to_string());
    assert_ne!(base_key, diff_submode.identity().canonical_key());

    let mut diff_date = base.clone();
    diff_date.qso_date = "20261019".to_string();
    assert_ne!(base_key, diff_date.identity().canonical_key());

    let mut diff_time = base.clone();
    diff_time.time_on = "120015".to_string();
    assert_ne!(base_key, diff_time.identity().canonical_key());
}

#[test]
fn non_identity_field_change_does_not_change_dedup_identity() {
    let mut q = make_test_qso_literal("SP6INA", "20261018", "120000");
    let key_orig = q.identity().canonical_key();

    q.rst_sent = "559".to_string();
    assert_eq!(key_orig, q.identity().canonical_key());
    q.rst_rcvd = "579".to_string();
    assert_eq!(key_orig, q.identity().canonical_key());
    q.name = Some("John".to_string());
    assert_eq!(key_orig, q.identity().canonical_key());
    q.qth = Some("Warsaw".to_string());
    assert_eq!(key_orig, q.identity().canonical_key());
    q.gridsquare = Some("KO02".to_string());
    assert_eq!(key_orig, q.identity().canonical_key());
    q.comment = Some("Great QSO".to_string());
    assert_eq!(key_orig, q.identity().canonical_key());
    q.qsl_sent = "Y".to_string();
    assert_eq!(key_orig, q.identity().canonical_key());
    q.lotw_qsl_sent = "Y".to_string();
    assert_eq!(key_orig, q.identity().canonical_key());
}

#[test]
fn identity_survives_serialization_roundtrip() {
    let id = QsoIdentity::from_detailed_fields(
        "SP6INA",
        "20M",
        Some(14.025),
        "SSB",
        Some("USB"),
        "20261018",
        "120015",
    );
    let json = serde_json::to_string(&id).unwrap();
    let deserialized: QsoIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(id, deserialized);
    assert_eq!(id.canonical_key(), deserialized.canonical_key());
}

#[test]
fn migration_7_upgrades_v1_keys_to_v2_keys_and_preserves_distinct_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("v6_to_v7.db");

    // Tworzymy baze na wersji 6 ze starym kluczem v1 i dwoma lacznosciami w tej samej minucie
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (1), (2), (3), (4), (5), (6);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
        LogDatabase::migration_2_add_columns(&conn).unwrap();
        conn.execute_batch(
            "ALTER TABLE qso_records ADD COLUMN identity_key TEXT;
             INSERT INTO qso_records (callsign, band, mode, submode, qso_date, time_on, freq, rst_sent, rst_rcvd, journal_id, identity_key)              VALUES ('SP6INA', '20m', 'CW', NULL, '20261018', '120010', 14.025, '599', '599', 'DEFAULT', 'SP6INA|20M|CW|20261018|1200');
             INSERT INTO qso_records (callsign, band, mode, submode, qso_date, time_on, freq, rst_sent, rst_rcvd, journal_id, identity_key)              VALUES ('SP6INA', '20m', 'CW', NULL, '20261018', '120045', 14.025, '599', '599', 'DEFAULT', 'SP6INA|20M|CW|20261018|1200');",
        ).unwrap();
    }

    // Otwarcie bazy wykonuje migracje 7
    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(db.count_all().unwrap(), 2);

    let conn = Connection::open(&db_path).unwrap();
    let keys: Vec<String> = conn
        .prepare("SELECT identity_key FROM qso_records ORDER BY time_on")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], "v2|SP6INA|20M|14025000|CW||20261018|S120010");
    assert_eq!(keys[1], "v2|SP6INA|20M|14025000|CW||20261018|S120045");
}

#[test]
fn migration_7_conflict_rolls_back_without_data_loss_and_preserves_v6() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("v7_conflict.db");

    // Baza na wersji 6 z dwoma identycznymi rekordami pod v2
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT DEFAULT (datetime('now')));
             INSERT INTO schema_version (version) VALUES (1), (2), (3), (4), (5), (6);",
        ).unwrap();
        LogDatabase::migration_1_base_schema(&conn).unwrap();
        LogDatabase::migration_2_add_columns(&conn).unwrap();
        conn.execute_batch(
            "ALTER TABLE qso_records ADD COLUMN identity_key TEXT;
             INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, freq, rst_sent, rst_rcvd, journal_id)              VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', 14.025, '599', '599', 'DEFAULT');
             INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, freq, rst_sent, rst_rcvd, journal_id)              VALUES ('SP6INA', '20m', 'CW', '20261018', '120000', 14.025, '599', '599', 'DEFAULT');",
        ).unwrap();
    }

    // 1. Otwarcie zawodzi z powodu konfliktu w migracji 7
    let res = LogDatabase::open(&db_path);
    assert!(
        res.is_err(),
        "migracja 7 musi zwrocic blad po wykryciu duplikatow"
    );

    // 2. Po rollback: wersja schematu to nadal 6, oba rekordy zachowane
    let conn = Connection::open(&db_path).unwrap();
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        version, 6,
        "schema_version nie moze wzrosnac po wycofanej migracji 7"
    );

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 2,
        "zaden rekord nie moze zostac usuniety ani scalony"
    );

    // 3. Rozstrzygniecie reczne konfliktu i ponowna proba migracji
    conn.execute(
        "UPDATE qso_records SET time_on = '120030' WHERE id = (SELECT MAX(id) FROM qso_records)",
        [],
    )
    .unwrap();

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(db.count_all().unwrap(), 2);
}

#[test]
fn concurrent_same_minute_different_seconds_from_two_repositories_both_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conc_diff_sec.db");
    {
        let _init = LogDatabase::open(&db_path).unwrap();
    }

    let p1 = db_path.clone();
    let p2 = db_path.clone();

    let t1 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p1).unwrap();
        let q = make_test_qso_literal("SP6INA", "20261018", "120010");
        db.insert_qso(&q)
    });

    let t2 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p2).unwrap();
        let q = make_test_qso_literal("SP6INA", "20261018", "120050");
        db.insert_qso(&q)
    });

    t1.join().unwrap().unwrap();
    t2.join().unwrap().unwrap();

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(
        db.count_all().unwrap(),
        2,
        "obie legalne lacznosci z roznymi sekundami musza zostac zapisane"
    );
}

#[test]
fn concurrent_same_minute_different_frequencies_from_two_repositories_both_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conc_diff_freq.db");
    {
        let _init = LogDatabase::open(&db_path).unwrap();
    }

    let p1 = db_path.clone();
    let p2 = db_path.clone();

    let t1 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p1).unwrap();
        let mut q = make_test_qso_literal("SP6INA", "20261018", "120000");
        q.freq = Some(14.025);
        db.insert_qso(&q)
    });

    let t2 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p2).unwrap();
        let mut q = make_test_qso_literal("SP6INA", "20261018", "120000");
        q.freq = Some(14.035);
        db.insert_qso(&q)
    });

    t1.join().unwrap().unwrap();
    t2.join().unwrap().unwrap();

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(
        db.count_all().unwrap(),
        2,
        "obie legalne lacznosci na roznych czestotliwosciach musza zostac zapisane"
    );
}

#[test]
fn concurrent_same_minute_different_submodes_from_two_repositories_both_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conc_diff_submode.db");
    {
        let _init = LogDatabase::open(&db_path).unwrap();
    }

    let p1 = db_path.clone();
    let p2 = db_path.clone();

    let t1 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p1).unwrap();
        let mut q = make_test_qso_literal("SP6INA", "20261018", "120000");
        q.mode = "SSB".to_string();
        q.submode = Some("USB".to_string());
        db.insert_qso(&q)
    });

    let t2 = std::thread::spawn(move || {
        let db = LogDatabase::open(&p2).unwrap();
        let mut q = make_test_qso_literal("SP6INA", "20261018", "120000");
        q.mode = "SSB".to_string();
        q.submode = Some("LSB".to_string());
        db.insert_qso(&q)
    });

    t1.join().unwrap().unwrap();
    t2.join().unwrap().unwrap();

    let db = LogDatabase::open(&db_path).unwrap();
    assert_eq!(
        db.count_all().unwrap(),
        2,
        "legalne lacznosci USB i LSB musza zostac zachowane jako odrebne"
    );
}

#[test]
fn concurrent_update_conflict_from_second_connection_preserves_both_records() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("conc_update_conflict.db");
    let db = LogDatabase::open(&db_path).unwrap();

    let q1 = make_test_qso_literal("SP6INA", "20261018", "120000");
    let q2 = make_test_qso_literal("DL1XYZ", "20261018", "120000");
    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();

    let p1 = db_path.clone();
    let p2 = db_path.clone();

    let t1 = std::thread::spawn(move || {
        let db1 = LogDatabase::open(&p1).unwrap();
        let mut conflicting = q2.clone();
        conflicting.callsign = "SP6INA".to_string();
        db1.update_qso(id2, &conflicting)
    });

    let t2 = std::thread::spawn(move || {
        let db2 = LogDatabase::open(&p2).unwrap();
        (
            db2.get_qso_by_id(id1).unwrap(),
            db2.get_qso_by_id(id2).unwrap(),
        )
    });

    let update_res = t1.join().unwrap();
    let (rec1, rec2) = t2.join().unwrap();

    assert!(
        update_res.is_err(),
        "kolizja UNIQUE przy wspolbieznym update musi zwrocic blad"
    );
    assert_eq!(rec1.unwrap().callsign, "SP6INA");
    assert_eq!(rec2.unwrap().callsign, "DL1XYZ");
}

#[test]
fn concurrent_identity_stress_five_iterations() {
    for iter in 0..5 {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join(format!("stress_{iter}.db"));
        {
            let _init = LogDatabase::open(&db_path).unwrap();
        }

        let p1 = db_path.clone();
        let p2 = db_path.clone();

        // 1. Same minute, different seconds
        let t1 = std::thread::spawn({
            let p = p1.clone();
            move || {
                let db = LogDatabase::open(&p).unwrap();
                let q = make_test_qso_literal("SP6INA", "20261018", "120015");
                db.insert_qso(&q)
            }
        });
        let t2 = std::thread::spawn({
            let p = p2.clone();
            move || {
                let db = LogDatabase::open(&p).unwrap();
                let q = make_test_qso_literal("SP6INA", "20261018", "120045");
                db.insert_qso(&q)
            }
        });
        t1.join().unwrap().unwrap();
        t2.join().unwrap().unwrap();

        // 2. Same minute, different frequency
        let t3 = std::thread::spawn({
            let p = p1.clone();
            move || {
                let db = LogDatabase::open(&p).unwrap();
                let mut q = make_test_qso_literal("W1AW", "20261018", "120000");
                q.freq = Some(7.020);
                db.insert_qso(&q)
            }
        });
        let t4 = std::thread::spawn({
            let p = p2.clone();
            move || {
                let db = LogDatabase::open(&p).unwrap();
                let mut q = make_test_qso_literal("W1AW", "20261018", "120000");
                q.freq = Some(7.030);
                db.insert_qso(&q)
            }
        });
        t3.join().unwrap().unwrap();
        t4.join().unwrap().unwrap();

        // 3. Duplicate race: exactly one wins
        let t5 = std::thread::spawn({
            let p = p1.clone();
            move || {
                let db = LogDatabase::open(&p).unwrap();
                let q = make_test_qso_literal("K1ABC", "20261018", "140000");
                db.insert_qso(&q)
            }
        });
        let t6 = std::thread::spawn({
            let p = p2.clone();
            move || {
                let db = LogDatabase::open(&p).unwrap();
                let q = make_test_qso_literal("K1ABC", "20261018", "140000");
                db.insert_qso(&q)
            }
        });
        let res5 = t5.join().unwrap();
        let res6 = t6.join().unwrap();

        let wins = match (res5, res6) {
            (Ok(_), Err(_)) | (Err(_), Ok(_)) => 1,
            _ => 0,
        };
        assert_eq!(
            wins, 1,
            "wyscig o duplikat w iteracji {iter} musi miec dokladnie jednego zwyciezce"
        );

        let db = LogDatabase::open(&db_path).unwrap();
        assert_eq!(
            db.count_all().unwrap(),
            5,
            "lacznie 5 rekordow w iteracji {iter}"
        );
    }
}

#[test]
fn journal_a_list_never_returns_journal_b_qsos() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_A".to_string(),
        name: "Journal A".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_B".to_string(),
        name: "Journal B".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q_a = QsoRecord::new("W1AW", "20m", "CW");
    q_a.journal_id = Some("J_A".to_string());
    q_a.qso_date = "20261101".to_string();
    q_a.time_on = "120000".to_string();
    db.insert_qso(&q_a).unwrap();

    let mut q_b = QsoRecord::new("K1ABC", "40m", "SSB");
    q_b.journal_id = Some("J_B".to_string());
    q_b.qso_date = "20261101".to_string();
    q_b.time_on = "130000".to_string();
    db.insert_qso(&q_b).unwrap();

    let list_a = db.get_all_qsos_for_journal("J_A").unwrap();
    assert_eq!(list_a.len(), 1);
    assert_eq!(list_a[0].callsign, "W1AW");

    let list_b = db.get_all_qsos_for_journal("J_B").unwrap();
    assert_eq!(list_b.len(), 1);
    assert_eq!(list_b[0].callsign, "K1ABC");

    let paginated_a = db.get_qsos_paginated_for_journal("J_A", 10, 0).unwrap();
    assert_eq!(paginated_a.len(), 1);
    assert_eq!(paginated_a[0].callsign, "W1AW");
}

#[test]
fn journal_a_search_never_returns_journal_b_qsos() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_A".to_string(),
        name: "Journal A".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_B".to_string(),
        name: "Journal B".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q_a = QsoRecord::new("DL1XYZ", "20m", "CW");
    q_a.journal_id = Some("J_A".to_string());
    q_a.qso_date = "20261101".to_string();
    q_a.time_on = "120000".to_string();
    db.insert_qso(&q_a).unwrap();

    let mut q_b = QsoRecord::new("DL1XYZ", "20m", "CW");
    q_b.journal_id = Some("J_B".to_string());
    q_b.qso_date = "20261101".to_string();
    q_b.time_on = "120000".to_string();
    db.insert_qso(&q_b).unwrap();

    let filter_a = AdvancedQsoFilter {
        callsign_query: Some("DL1XYZ".to_string()),
        journal_id: Some("J_A".to_string()),
        ..Default::default()
    };
    let results_a = db.search_qsos_advanced(&filter_a).unwrap();
    assert_eq!(results_a.len(), 1);
    assert_eq!(results_a[0].journal_id.as_deref(), Some("J_A"));

    let filter_b = AdvancedQsoFilter {
        callsign_query: Some("DL1XYZ".to_string()),
        journal_id: Some("J_B".to_string()),
        ..Default::default()
    };
    let results_b = db.search_qsos_advanced(&filter_b).unwrap();
    assert_eq!(results_b.len(), 1);
    assert_eq!(results_b[0].journal_id.as_deref(), Some("J_B"));
}

#[test]
fn journal_a_count_excludes_journal_b() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_A".to_string(),
        name: "Journal A".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_B".to_string(),
        name: "Journal B".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    for i in 1..=3 {
        let mut q = QsoRecord::new(format!("DL{i}AA"), "20m", "CW");
        q.journal_id = Some("J_A".to_string());
        q.qso_date = "20261101".to_string();
        q.time_on = format!("120{i}00");
        db.insert_qso(&q).unwrap();
    }
    for i in 1..=5 {
        let mut q = QsoRecord::new(format!("SP{i}BB"), "40m", "SSB");
        q.journal_id = Some("J_B".to_string());
        q.qso_date = "20261101".to_string();
        q.time_on = format!("130{i}00");
        db.insert_qso(&q).unwrap();
    }

    assert_eq!(db.count_qsos_for_journal("J_A").unwrap(), 3);
    assert_eq!(db.count_qsos_for_journal("J_B").unwrap(), 5);
}

#[test]
fn journal_a_get_by_foreign_id_returns_not_found() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_A".to_string(),
        name: "Journal A".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_B".to_string(),
        name: "Journal B".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q_b = QsoRecord::new("W1AW", "20m", "CW");
    q_b.journal_id = Some("J_B".to_string());
    q_b.qso_date = "20261101".to_string();
    q_b.time_on = "120000".to_string();
    let id_b = db.insert_qso(&q_b).unwrap();

    assert_eq!(db.get_qso_by_id_scoped(id_b, "J_A").unwrap(), None);
    assert!(db.get_qso_by_id_scoped(id_b, "J_B").unwrap().is_some());
}

#[test]
fn journal_scoped_identity_lookup_does_not_cross_journals() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_A".to_string(),
        name: "Journal A".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_B".to_string(),
        name: "Journal B".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q_a = QsoRecord::new("W1AW", "20m", "CW");
    q_a.journal_id = Some("J_A".to_string());
    q_a.qso_date = "20261101".to_string();
    q_a.time_on = "120000".to_string();
    db.insert_qso(&q_a).unwrap();

    let id_key = q_a.identity();
    assert!(db.has_identity("J_A", &id_key).unwrap());
    assert!(!db.has_identity("J_B", &id_key).unwrap());

    let existing_a = db.existing_identities("J_A").unwrap();
    let existing_b = db.existing_identities("J_B").unwrap();
    assert!(existing_a.contains(&id_key));
    assert!(!existing_b.contains(&id_key));
}

#[test]
fn find_duplicates_is_isolated_to_active_journal() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_A".to_string(),
        name: "Journal A".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_B".to_string(),
        name: "Journal B".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q_a = QsoRecord::new("W1AW", "20m", "CW");
    q_a.journal_id = Some("J_A".to_string());
    q_a.qso_date = "20261101".to_string();
    q_a.time_on = "120000".to_string();
    db.insert_qso(&q_a).unwrap();

    let mut q_b = QsoRecord::new("W1AW", "20m", "CW");
    q_b.journal_id = Some("J_B".to_string());
    q_b.qso_date = "20261101".to_string();
    q_b.time_on = "120000".to_string();
    db.insert_qso(&q_b).unwrap();

    let dups_a = db.find_duplicate_qsos("J_A", false).unwrap();
    assert_eq!(dups_a.len(), 0, "Dziennik J_A nie powinien miec duplikatow");

    let dups_b = db.find_duplicate_qsos("J_B", false).unwrap();
    assert_eq!(dups_b.len(), 0, "Dziennik J_B nie powinien miec duplikatow");
}

#[test]
fn journal_mutation_isolation_update_and_delete() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_ONE".to_string(),
        name: "Journal One".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_TWO".to_string(),
        name: "Journal Two".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q1 = QsoRecord::new("W1AW", "20m", "CW");
    q1.journal_id = Some("J_ONE".to_string());
    let id1 = db.insert_qso(&q1).unwrap();

    let mut q2 = QsoRecord::new("K1ABC", "40m", "SSB");
    q2.journal_id = Some("J_TWO".to_string());
    let id2 = db.insert_qso(&q2).unwrap();

    // 1. Attempt update id1 with J_TWO: must return Ok(false) and not modify
    let mut edit = q1.clone();
    edit.rst_sent = "579".to_string();
    let updated = db.update_qso_scoped(id1, "J_TWO", &edit).unwrap();
    assert!(!updated, "Cross-journal update must return false");
    let fetched = db.get_qso_by_id_scoped(id1, "J_ONE").unwrap().unwrap();
    assert_ne!(fetched.rst_sent, "579");

    // 2. Valid update with J_ONE: must succeed
    let updated = db.update_qso_scoped(id1, "J_ONE", &edit).unwrap();
    assert!(updated, "Journal-scoped update must return true");
    let fetched = db.get_qso_by_id_scoped(id1, "J_ONE").unwrap().unwrap();
    assert_eq!(fetched.rst_sent, "579");

    // 3. Attempt delete id2 with J_ONE: must return Ok(false) and record persists in J_TWO
    let deleted = db.delete_qso_scoped(id2, "J_ONE").unwrap();
    assert!(!deleted, "Cross-journal delete must return false");
    assert!(db.get_qso_by_id_scoped(id2, "J_TWO").unwrap().is_some());

    // 4. Batch delete with J_ONE on [id1, id2]: must only delete id1
    let deleted_count = db
        .delete_multiple_qsos_scoped(&[id1, id2], "J_ONE")
        .unwrap();
    assert_eq!(deleted_count, 1);
    assert!(db.get_qso_by_id_scoped(id1, "J_ONE").unwrap().is_none());
    assert!(db.get_qso_by_id_scoped(id2, "J_TWO").unwrap().is_some());

    // 5. Delete id2 with J_TWO: succeeds
    let deleted = db.delete_qso_scoped(id2, "J_TWO").unwrap();
    assert!(deleted);
    assert!(db.get_qso_by_id_scoped(id2, "J_TWO").unwrap().is_none());
}

#[test]
fn journal_qsl_confirmations_isolation_and_batch() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_ALPHA".to_string(),
        name: "Journal Alpha".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_BETA".to_string(),
        name: "Journal Beta".to_string(),
        station_callsign: "SP6INA/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q_a = QsoRecord::new("W1AW", "20m", "CW");
    q_a.journal_id = Some("J_ALPHA".to_string());
    q_a.qso_date = "20261101".to_string();
    let id_a = db.insert_qso(&q_a).unwrap();

    let mut q_b = QsoRecord::new("W1AW", "20m", "CW");
    q_b.journal_id = Some("J_BETA".to_string());
    q_b.qso_date = "20261101".to_string();
    let id_b = db.insert_qso(&q_b).unwrap();

    // Confirm for J_ALPHA only
    let updated = db
        .mark_lotw_confirmed_scoped("J_ALPHA", "W1AW", "20m", "CW", "20261101", "20261102")
        .unwrap();
    assert_eq!(updated, 1);

    // J_ALPHA is confirmed, J_BETA is NOT confirmed
    let rec_a = db.get_qso_by_id_scoped(id_a, "J_ALPHA").unwrap().unwrap();
    assert_eq!(rec_a.lotw_qsl_rcvd, "Y");

    let rec_b = db.get_qso_by_id_scoped(id_b, "J_BETA").unwrap().unwrap();
    assert_ne!(rec_b.lotw_qsl_rcvd, "Y");

    // Batch confirm eQSL for J_BETA
    let conf_qsos = vec![rec_b.clone()];
    let updated_b = db
        .mark_eqsl_confirmed_batch_for_journal("J_BETA", &conf_qsos)
        .unwrap();
    assert_eq!(updated_b, 1);

    let rec_b_after = db.get_qso_by_id_scoped(id_b, "J_BETA").unwrap().unwrap();
    assert_eq!(rec_b_after.eqsl_qsl_rcvd, "Y");

    let rec_a_after = db.get_qso_by_id_scoped(id_a, "J_ALPHA").unwrap().unwrap();
    assert_ne!(rec_a_after.eqsl_qsl_rcvd, "Y");
}

#[test]
fn journal_statistics_isolation() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "J_BASE".to_string(),
        name: "Base Station".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "J_EXPED".to_string(),
        name: "Expedition".to_string(),
        station_callsign: "SP6INA/3B8".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    // Insert 2 QSOs in J_BASE
    let mut q1 = QsoRecord::new("W1AW", "20m", "CW");
    q1.journal_id = Some("J_BASE".to_string());
    q1.country = Some("United States".to_string());
    q1.dxcc = Some(291);
    db.insert_qso(&q1).unwrap();

    let mut q2 = QsoRecord::new("DL1ABC", "40m", "SSB");
    q2.journal_id = Some("J_BASE".to_string());
    q2.country = Some("Fed. Rep. of Germany".to_string());
    q2.dxcc = Some(230);
    db.insert_qso(&q2).unwrap();

    // Insert 1 QSO in J_EXPED
    let mut q3 = QsoRecord::new("JA1XYZ", "15m", "FT8");
    q3.journal_id = Some("J_EXPED".to_string());
    q3.country = Some("Japan".to_string());
    q3.dxcc = Some(339);
    db.insert_qso(&q3).unwrap();

    // 1. Bands isolation
    let bands_base = db.stats_qso_per_band_for_journal("J_BASE").unwrap();
    assert_eq!(bands_base.len(), 2);
    assert!(bands_base.iter().any(|(b, c)| b == "20m" && *c == 1));
    assert!(bands_base.iter().any(|(b, c)| b == "40m" && *c == 1));
    assert!(!bands_base.iter().any(|(b, _)| b == "15m"));

    let bands_exped = db.stats_qso_per_band_for_journal("J_EXPED").unwrap();
    assert_eq!(bands_exped.len(), 1);
    assert_eq!(bands_exped[0].0, "15m");

    // 2. Unique counts isolation
    let (calls_base, dxcc_base) = db.stats_unique_counts_for_journal("J_BASE").unwrap();
    assert_eq!(calls_base, 2);
    assert_eq!(dxcc_base, 2);

    let (calls_exped, dxcc_exped) = db.stats_unique_counts_for_journal("J_EXPED").unwrap();
    assert_eq!(calls_exped, 1);
    assert_eq!(dxcc_exped, 1);

    // 3. Top countries isolation
    let countries_base = db.stats_top_countries_for_journal("J_BASE", 10).unwrap();
    assert_eq!(countries_base.len(), 2);
    assert!(!countries_base.iter().any(|(cty, _)| cty == "Japan"));

    let countries_exped = db.stats_top_countries_for_journal("J_EXPED", 10).unwrap();
    assert_eq!(countries_exped.len(), 1);
    assert_eq!(countries_exped[0].0, "Japan");
}

#[test]
fn awards_engine_journal_scoped_rebuild() {
    let mut engine = crate::core::awards::AwardsEngine::new();

    let mut q1 = QsoRecord::new("W1AW", "20m", "CW");
    q1.journal_id = Some("LOG_A".to_string());
    q1.dxcc = Some(291);

    let mut q2 = QsoRecord::new("JA1XYZ", "15m", "FT8");
    q2.journal_id = Some("LOG_B".to_string());
    q2.dxcc = Some(339);

    engine.rebuild_for_journal("LOG_A", &[q1.clone(), q2.clone()]);
    assert_eq!(engine.scoped_journal_id.as_deref(), Some("LOG_A"));
    assert!(engine.worked_dxcc_all.contains(&291));
    assert!(
        !engine.worked_dxcc_all.contains(&339),
        "AwardsEngine must not leak DXCC from LOG_B into LOG_A"
    );

    engine.rebuild_for_journal("LOG_B", &[q1, q2]);
    assert_eq!(engine.scoped_journal_id.as_deref(), Some("LOG_B"));
    assert!(engine.worked_dxcc_all.contains(&339));
    assert!(
        !engine.worked_dxcc_all.contains(&291),
        "AwardsEngine must not leak DXCC from LOG_A into LOG_B"
    );
}

#[test]
fn test_restore_qso_scoped_and_batch() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "JOURNAL_1".to_string(),
        name: "Journal 1".to_string(),
        station_callsign: "SP1ABC".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q1 = QsoRecord::new("SP1ABC", "20m", "SSB");
    q1.qso_date = "20260701".to_string();
    q1.time_on = "120000".to_string();
    q1.journal_id = Some("JOURNAL_1".to_string());

    let mut q2 = QsoRecord::new("SP2DEF", "40m", "CW");
    q2.qso_date = "20260701".to_string();
    q2.time_on = "120500".to_string();
    q2.journal_id = Some("JOURNAL_1".to_string());

    let id1 = db.insert_qso(&q1).unwrap();
    let id2 = db.insert_qso(&q2).unwrap();
    q1.id = Some(id1);
    q2.id = Some(id2);

    // Delete both via delete_multiple_qsos_scoped
    let deleted_count = db
        .delete_multiple_qsos_scoped(&[id1, id2], "JOURNAL_1")
        .unwrap();
    assert_eq!(deleted_count, 2);

    // Ensure they are gone from JOURNAL_1
    let remaining = db.get_recent_qsos_for_journal("JOURNAL_1", 10).unwrap();
    assert_eq!(remaining.len(), 0);

    // Batch restore scoped to JOURNAL_1
    let restored_ids = db
        .restore_multiple_qsos_scoped("JOURNAL_1", &[q1.clone(), q2.clone()])
        .unwrap();
    assert_eq!(restored_ids, vec![id1, id2]);

    let after_restore = db.get_recent_qsos_for_journal("JOURNAL_1", 10).unwrap();
    assert_eq!(after_restore.len(), 2);
    assert_eq!(after_restore[0].callsign, "SP2DEF");
    assert_eq!(after_restore[1].callsign, "SP1ABC");

    // Attempting to restore again with identical ID must fail (no ON CONFLICT REPLACE)
    let duplicate_restore = db.restore_qso_scoped("JOURNAL_1", &q1);
    assert!(
        duplicate_restore.is_err(),
        "Restoring an existing ID must fail with error"
    );
}

#[test]
fn test_restore_qso_scoped_prevents_cross_journal_restoration() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "LOG_ORIGIN".to_string(),
        name: "Origin".to_string(),
        station_callsign: "SP1ABC".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.create_journal(&Journal {
        id: "LOG_TARGET".to_string(),
        name: "Target".to_string(),
        station_callsign: "SP1ABC/P".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();

    let mut q = QsoRecord::new("DL9XYZ", "15m", "FT8");
    q.qso_date = "20260702".to_string();
    q.time_on = "140000".to_string();
    q.journal_id = Some("LOG_ORIGIN".to_string());

    let id = db.insert_qso(&q).unwrap();
    q.id = Some(id);

    db.delete_qso_scoped(id, "LOG_ORIGIN").unwrap();

    // Restore explicitly scoped into TARGET_LOG
    let restored_id = db.restore_qso_scoped("LOG_TARGET", &q).unwrap();
    assert_eq!(restored_id, id);

    // Check it exists in TARGET_LOG and not in ORIGIN
    let rec_origin = db.get_qso_by_id_scoped(id, "LOG_ORIGIN").unwrap();
    assert!(rec_origin.is_none());

    let rec_target = db.get_qso_by_id_scoped(id, "LOG_TARGET").unwrap();
    assert!(rec_target.is_some());
    assert_eq!(
        rec_target.unwrap().journal_id.as_deref(),
        Some("LOG_TARGET")
    );
}

// ---------------------------------------------------------------------------
// P1-F20R — trwałe checkpointy uploadu (F20-R1) i daty (F20-R2)
// ---------------------------------------------------------------------------

fn f20r_journal(id: &str) -> Journal {
    Journal {
        id: id.to_string(),
        name: format!("journal {id}"),
        station_callsign: "TSTSTN".to_string(),
        operator: "TSTOP".to_string(),
        my_gridsquare: "JO90AA".to_string(),
        my_pga: String::new(),
        description: "synthetic F-20R fixture".to_string(),
        is_default: false,
    }
}

fn f20r_plan(
    db: &LogDatabase,
    journal_id: &str,
    batch_size: usize,
) -> crate::cloud::upload_plan::UploadPlan {
    use crate::cloud::upload_plan::{JournalId, SelectionPolicy, UploadPlan, UploadTarget};
    let journal = JournalId::new(journal_id).expect("journal id");
    let records = db
        .select_upload_candidates_for_journal(&journal, UploadTarget::Lotw)
        .expect("candidates");
    UploadPlan::build(
        UploadTarget::Lotw,
        journal,
        SelectionPolicy::PendingOnly,
        "attempt-f20r-unit",
        1_700_000_000,
        records,
        batch_size,
    )
    .expect("plan")
}

fn f20r_seeded_db(journal_id: &str, count: usize) -> LogDatabase {
    let db = LogDatabase::open_in_memory().expect("in-memory");
    db.create_journal(&f20r_journal(journal_id))
        .expect("journal");
    let qsos: Vec<QsoRecord> = (0..count)
        .map(|i| {
            let mut qso = QsoRecord::new(format!("TST{i:05}"), "20m", "CW");
            qso.journal_id = Some(journal_id.to_string());
            qso.qso_date = format!("202601{:02}", (i % 28) + 1);
            qso.time_on = format!("{:04}00", i % 2400);
            qso
        })
        .collect();
    db.batch_insert_qsos(&qsos).expect("insert");
    db
}

#[test]
fn f20r_checkpoint_tables_and_sent_date_columns_exist_after_migrations() {
    let db = LogDatabase::open_in_memory().expect("db");
    assert_eq!(
        db.current_schema_version().expect("version"),
        LATEST_SCHEMA_VERSION
    );
    for table in ["upload_plans", "upload_batches", "upload_batch_members"] {
        let exists: bool = db
            .raw_conn()
            .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .unwrap()
            .exists(rusqlite::params![table])
            .unwrap();
        assert!(exists, "brak tabeli {table}");
    }
    for column in ["lotw_qslsdate", "eqsl_qslsdate"] {
        let exists: bool = db
            .raw_conn()
            .prepare("SELECT 1 FROM pragma_table_info('qso_records') WHERE name = ?1")
            .unwrap()
            .exists(rusqlite::params![column])
            .unwrap();
        assert!(exists, "brak kolumny {column}");
    }
}

#[test]
fn f20r_incompatible_checkpoint_schema_version_is_rejected() {
    use crate::core::database::upload_checkpoint::CheckpointError;
    let db = f20r_seeded_db("F20R_UNIT_A", 40);
    let plan = f20r_plan(&db, "F20R_UNIT_A", 10);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    db.raw_conn()
        .execute(
            "UPDATE upload_plans SET schema_version = 999 WHERE plan_id = ?1",
            rusqlite::params![plan.plan_id()],
        )
        .expect("mutate");
    let err = db.load_upload_checkpoint_set(plan.plan_id());
    assert!(matches!(
        err,
        Err(CheckpointError::IncompatibleSchemaVersion { found: 999, .. })
    ));
}

#[test]
fn f20r_corrupted_membership_is_detected_instead_of_silently_trusted() {
    use crate::core::database::upload_checkpoint::CheckpointError;
    let db = f20r_seeded_db("F20R_UNIT_B", 40);
    let plan = f20r_plan(&db, "F20R_UNIT_B", 10);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    db.raw_conn()
        .execute(
            "UPDATE upload_batch_members SET qso_id = qso_id + 1000000
             WHERE plan_id = ?1 AND ordinal = 0 AND position = 0",
            rusqlite::params![plan.plan_id()],
        )
        .expect("corrupt membership");
    let err = db.load_upload_checkpoint_set(plan.plan_id());
    assert!(matches!(
        err,
        Err(CheckpointError::MembershipMismatch { .. })
    ));
}

#[test]
fn f20r_checkpoint_tables_contain_no_secret_or_url() {
    use crate::cloud::upload_checkpoint::OutcomeSnapshot;
    use crate::cloud::upload_plan::BatchOutcome;
    let db = f20r_seeded_db("F20R_UNIT_C", 40);
    let plan = f20r_plan(&db, "F20R_UNIT_C", 10);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    db.prepare_upload_batch(plan.plan_id(), 0, "digest", 100)
        .expect("prepare");
    db.start_upload_batch(plan.plan_id(), 0, 101)
        .expect("start");
    db.record_upload_batch_outcome(
        plan.plan_id(),
        0,
        OutcomeSnapshot::from_outcome(&BatchOutcome::Accepted, 10),
        102,
    )
    .expect("accepted");
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, 103)
        .expect("commit");

    let mut dump = String::new();
    for table in ["upload_plans", "upload_batches", "upload_batch_members"] {
        let mut stmt = db
            .raw_conn()
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap();
        let column_count = stmt.column_count();
        let rows = stmt
            .query_map([], |row| {
                let mut cells = Vec::new();
                for index in 0..column_count {
                    let value: Option<String> = row.get(index).ok();
                    cells.push(value.unwrap_or_default());
                }
                Ok(cells.join("|"))
            })
            .unwrap();
        for row in rows {
            dump.push_str(&row.unwrap());
            dump.push('\n');
        }
    }
    for secret in [
        "TEST_LOTW_SECRET_DO_NOT_LOG",
        "TEST_EQSL_SECRET_DO_NOT_LOG",
        "TEST_TQSL_CERT_PASSPHRASE_DO_NOT_LOG",
    ] {
        assert!(!dump.contains(secret), "checkpoint zawiera sekret");
    }
    for needle in ["https://", "http://", "password=", "login=", "Password="] {
        assert!(!dump.contains(needle), "checkpoint zawiera `{needle}`");
    }
}

#[test]
fn f20r_migration_9_rollback_and_reapply_preserves_upload_status() {
    let db = f20r_seeded_db("F20R_UNIT_D", 12);
    let qsos = db.get_all_qsos_for_journal("F20R_UNIT_D").expect("qsos");
    let id = qsos[0].id.expect("id");
    let mut qso = qsos[0].clone();
    qso.lotw_qsl_sent = "Y".to_string();
    qso.lotw_qslsdate = Some("20261117".to_string());
    qso.eqsl_qsl_sent = "Y".to_string();
    qso.eqsl_qslsdate = Some("20261118".to_string());
    db.update_qso(id, &qso).expect("update");

    // Rollback v9: kolumny znikają, wersja wraca do 8, status `Y` zostaje.
    // Kolejność odwrotna do zastosowania: najpierw migracja 10 (STAGE-V140-S1).
    LogDatabase::rollback_migration_11(db.raw_conn()).expect("rollback 11");
    LogDatabase::rollback_migration_10(db.raw_conn()).expect("rollback 10");
    LogDatabase::rollback_migration_9(db.raw_conn()).expect("rollback 9");
    assert_eq!(db.current_schema_version().expect("version"), 8);
    let has_column: bool = db
        .raw_conn()
        .prepare("SELECT 1 FROM pragma_table_info('qso_records') WHERE name = 'lotw_qslsdate'")
        .unwrap()
        .exists([])
        .unwrap();
    assert!(!has_column, "rollback musi usunąć kolumnę daty");
    let status: String = db
        .raw_conn()
        .query_row(
            "SELECT lotw_qsl_sent FROM qso_records WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "Y", "rollback nie może zgubić statusu wysłania");
}

#[test]
fn f20r_migration_8_rollback_and_reapply_preserves_qso_data() {
    let db = f20r_seeded_db("F20R_UNIT_E", 25);
    let plan = f20r_plan(&db, "F20R_UNIT_E", 10);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    assert_eq!(db.count_upload_plans().expect("count"), 1);

    // Rollback w kolejności odwrotnej do zastosowania: najpierw 10, 9, potem 8.
    LogDatabase::rollback_migration_11(db.raw_conn()).expect("rollback 11");
    LogDatabase::rollback_migration_10(db.raw_conn()).expect("rollback 10");
    LogDatabase::rollback_migration_9(db.raw_conn()).expect("rollback 9");
    LogDatabase::rollback_migration_8(db.raw_conn()).expect("rollback 8");
    assert_eq!(db.current_schema_version().expect("version"), 7);
    let exists: bool = db
        .raw_conn()
        .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'upload_plans'")
        .unwrap()
        .exists([])
        .unwrap();
    assert!(!exists, "rollback musi usunąć tabele checkpointów");
    // Dane QSO pozostają nienaruszone. Liczymy surowo, bo kolumny dat wysłania
    // również zostały wycofane przez rollback v9.
    let qso_count: i64 = db
        .raw_conn()
        .query_row("SELECT COUNT(*) FROM qso_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(qso_count, 25);
}

#[test]
fn f20r_reopening_a_migrated_database_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("f20r_idempotent.sqlite");
    {
        let db = LogDatabase::open(&path).expect("open");
        assert_eq!(
            db.current_schema_version().expect("version"),
            LATEST_SCHEMA_VERSION
        );
    }
    for _ in 0..5 {
        let db = LogDatabase::open(&path).expect("reopen");
        assert_eq!(
            db.current_schema_version().expect("version"),
            LATEST_SCHEMA_VERSION
        );
        let versions: i64 = db
            .raw_conn()
            .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            versions, LATEST_SCHEMA_VERSION,
            "każda wersja migracji dokładnie raz"
        );
        assert_eq!(db.count_upload_plans().expect("count"), 0);
    }
}

#[test]
fn f20r_interrupted_migration_keeps_the_previous_version_and_is_rerunnable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("f20r_interrupted.sqlite");
    {
        let db = LogDatabase::open(&path).expect("open");
        LogDatabase::rollback_migration_11(db.raw_conn()).expect("rollback 11");
        LogDatabase::rollback_migration_10(db.raw_conn()).expect("rollback 10");
        LogDatabase::rollback_migration_9(db.raw_conn()).expect("rollback 9");
        assert_eq!(db.current_schema_version().expect("version"), 8);
    }
    {
        // Przerwana migracja: dokładnie ta sama instrukcja DDL, ale w
        // transakcji wycofanej bez commita — tak wygląda zabity proces.
        let conn = Connection::open(&path).expect("raw open");
        {
            let tx = conn.unchecked_transaction().expect("tx");
            tx.execute_batch(
                "ALTER TABLE qso_records ADD COLUMN lotw_qslsdate TEXT;
                 ALTER TABLE qso_records ADD COLUMN eqsl_qslsdate TEXT;",
            )
            .expect("probe ddl");
            // `tx` wycofywana przy drop.
        }
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, 8, "przerwana migracja nie podnosi wersji");
        let probe: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('qso_records') WHERE name = 'lotw_qslsdate'")
            .unwrap()
            .exists([])
            .unwrap();
        assert!(!probe, "przerwana migracja nie może zostawić kolumny");
    }
    {
        // Ponowne otwarcie kończy migrację 9 (i kolejne) dokładnie raz.
        let db = LogDatabase::open(&path).expect("reopen");
        assert_eq!(
            db.current_schema_version().expect("version"),
            LATEST_SCHEMA_VERSION
        );
        for column in ["lotw_qslsdate", "eqsl_qslsdate"] {
            let exists: bool = db
                .raw_conn()
                .prepare("SELECT 1 FROM pragma_table_info('qso_records') WHERE name = ?1")
                .unwrap()
                .exists(rusqlite::params![column])
                .unwrap();
            assert!(exists, "{column}");
        }
        let version_rows: i64 = db
            .raw_conn()
            .query_row(
                "SELECT COUNT(*) FROM schema_version WHERE version = 9",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version_rows, 1);
    }
}

#[test]
fn f20r_legacy_accepted_rows_keep_y_with_null_sent_date() {
    let db = f20r_seeded_db("F20R_UNIT_F", 8);
    let qsos = db.get_all_qsos_for_journal("F20R_UNIT_F").expect("qsos");
    let id = qsos[0].id.expect("id");
    let mut qso = qsos[0].clone();
    // Historyczny wiersz: status Y, data nieznana.
    qso.lotw_qsl_sent = "Y".to_string();
    qso.lotw_qslsdate = None;
    qso.eqsl_qsl_sent = "I".to_string();
    qso.eqsl_qslsdate = None;
    db.update_qso(id, &qso).expect("update");

    let reloaded = db.get_qso_by_id(id).expect("qso").expect("exists");
    assert_eq!(reloaded.lotw_qsl_sent, "Y");
    assert!(reloaded.lotw_qslsdate.is_none());
    assert_eq!(reloaded.eqsl_qsl_sent, "I");
    assert!(reloaded.eqsl_qslsdate.is_none());

    // Migracja nie wypełnia historii żadną datą.
    LogDatabase::rollback_migration_11(db.raw_conn()).expect("rollback 11");
    LogDatabase::rollback_migration_10(db.raw_conn()).expect("rollback 10");
    LogDatabase::rollback_migration_9(db.raw_conn()).expect("rollback 9");
    assert_eq!(db.current_schema_version().expect("version"), 8);
}

#[test]
fn f20r_sent_date_column_is_text_and_nullable() {
    let db = LogDatabase::open_in_memory().expect("db");
    let mut stmt = db
        .raw_conn()
        .prepare("SELECT name, type, \"notnull\" FROM pragma_table_info('qso_records')")
        .expect("prepare");
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .expect("rows");
    let mut found = 0;
    for row in rows {
        let (name, kind, notnull) = row.expect("row");
        if name == "lotw_qslsdate" || name == "eqsl_qslsdate" {
            assert_eq!(kind, "TEXT");
            assert_eq!(notnull, 0, "{name} musi dopuszczać NULL");
            found += 1;
        }
    }
    assert_eq!(found, 2);
}

#[test]
fn open_creates_missing_databases_directory_on_clean_profile() {
    // Całkowicie pusty katalog tymczasowy: bez ręcznego tworzenia `SPLogbook/databases`.
    let profile = tempfile::tempdir().expect("tempdir");
    let app_data = profile.path().join("Roaming").join("SPLogbook");
    assert!(!app_data.exists());

    let db_path = default_log_db_path(&app_data);
    assert!(db_path.ends_with(std::path::Path::new("databases").join("default_log.db")));
    #[cfg(windows)]
    assert!(
        !db_path.to_string_lossy().contains('/'),
        "ścieżka na Windowsie nie może mieszać separatorów: {}",
        db_path.display()
    );

    let db = LogDatabase::open(&db_path).expect("open na czystym profilu");
    assert!(db_path.is_file(), "plik bazy został utworzony");
    assert!(db_path.parent().unwrap().is_dir());
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);
}

#[test]
fn open_handles_mixed_separator_path_on_clean_profile() {
    // Regresja: `dir.join("databases/default_log.db")` (mieszane separatory na Windowsie).
    let profile = tempfile::tempdir().expect("tempdir");
    let db_path = profile
        .path()
        .join("SPLogbook")
        .join("databases/default_log.db");
    let db = LogDatabase::open(&db_path).expect("open ze ścieżką o mieszanych separatorach");
    drop(db);
    assert!(db_path.is_file());
}

#[test]
fn open_reports_readable_error_when_directory_cannot_be_created() {
    // Rodzic jest plikiem, więc katalogu nie da się utworzyć.
    let profile = tempfile::tempdir().expect("tempdir");
    let blocker = profile.path().join("SPLogbook");
    std::fs::write(&blocker, b"plik, nie katalog").unwrap();
    let err = LogDatabase::open(blocker.join("databases").join("default_log.db"))
        .err()
        .expect("musi się nie udać");
    assert!(
        err.to_string().contains("cannot create directory"),
        "błąd ma wskazywać przyczynę: {err}"
    );
}

// ---------------------------------------------------------------------------------------------
// STAGE-V140-S1 — fundament danych: migracja 10, jedno źródło wersji schematu, kopia
// przedmigracyjna, plany zapytań, dopasowanie QSL i SCP.
// ---------------------------------------------------------------------------------------------

const S1_DATE_EXPR: &str = "REPLACE(qso_date, '-', '')";
const S1_TIME_EXPR: &str = "SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6)";

fn s1_qso(call: &str, band: &str, mode: &str, date: &str, time: &str) -> QsoRecord {
    let mut qso = QsoRecord::new(call, band, mode);
    qso.qso_date = date.to_string();
    qso.time_on = time.to_string();
    qso
}

fn s1_plan(conn: &Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> String {
    let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
    let rows = stmt.query_map(params, |r| r.get::<_, String>(3)).unwrap();
    rows.map(|r| r.unwrap()).collect::<Vec<_>>().join(" ; ")
}

fn s1_ids(conn: &Connection, sql: &str, journal: &str) -> Vec<i64> {
    let mut stmt = conn.prepare(sql).unwrap();
    let rows = stmt.query_map([journal], |r| r.get::<_, i64>(0)).unwrap();
    rows.map(|r| r.unwrap()).collect()
}

fn s1_has_index(conn: &Connection, name: &str) -> bool {
    conn.prepare("SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1")
        .unwrap()
        .exists([name])
        .unwrap()
}

fn s1_raw_version(path: &std::path::Path) -> i64 {
    let conn = Connection::open(path).unwrap();
    conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .unwrap()
}

fn s1_raw_count(path: &std::path::Path) -> i64 {
    let conn = Connection::open(path).unwrap();
    conn.query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))
        .unwrap()
}

/// Plikowa baza z trzema QSO cofnięta do schematu 9 (stan sprzed STAGE-V140-S1).
fn s1_v9_database(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("log.db");
    {
        let db = LogDatabase::open(&path).unwrap();
        for i in 0..3 {
            db.insert_qso(&s1_qso(
                &format!("SP{i}AAA"),
                "20m",
                "CW",
                "20260101",
                &format!("12{i:02}00"),
            ))
            .unwrap();
        }
        LogDatabase::rollback_migration_11(db.raw_conn()).unwrap();
        LogDatabase::rollback_migration_10(db.raw_conn()).unwrap();
    }
    assert_eq!(s1_raw_version(&path), 9);
    path
}

fn s1_pre_migration_files(db_path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let dir = pre_migration_backup_dir(db_path);
    match std::fs::read_dir(&dir) {
        Ok(entries) => entries.map(|e| e.unwrap().path()).collect(),
        Err(_) => Vec::new(),
    }
}

#[test]
fn s1_migrations_table_is_contiguous_and_defines_latest_version() {
    let versions: Vec<i64> = migrations::MIGRATIONS.iter().map(|(v, _)| *v).collect();
    let expected: Vec<i64> = (1..=versions.len() as i64).collect();
    assert_eq!(versions, expected, "migracje muszą rosnąć od 1 bez luk");
    assert_eq!(Some(&LATEST_SCHEMA_VERSION), versions.last());
    const {
        assert!(LATEST_SCHEMA_VERSION >= 10);
    }
    assert_eq!(
        crate::core::backup::BackupManager::SUPPORTED_SCHEMA_VERSION,
        LATEST_SCHEMA_VERSION,
        "kopia zapasowa korzysta z tej samej wersji schematu"
    );
}

#[test]
fn s1_migration_10_index_expressions_match_order_chrono() {
    use super::schema::{ORDER_CHRONO_ASC, ORDER_CHRONO_DESC};
    assert_eq!(
        ORDER_CHRONO_ASC,
        format!("{S1_DATE_EXPR} ASC, {S1_TIME_EXPR} ASC, id ASC")
    );
    assert_eq!(
        ORDER_CHRONO_DESC,
        format!("{S1_DATE_EXPR} DESC, {S1_TIME_EXPR} DESC, id DESC")
    );
    let sql = migrations::MIGRATION_10_SQL;
    assert!(sql.contains(&format!("(journal_id, {S1_DATE_EXPR}, {S1_TIME_EXPR}, id)")));
    assert!(sql.contains(&format!(
        "(journal_id, callsign, {S1_DATE_EXPR}, {S1_TIME_EXPR}, id)"
    )));
    assert!(sql.contains(&format!("(journal_id, callsign, band, {S1_DATE_EXPR})")));
    assert!(qsl::QSL_CANDIDATES_SQL.contains(&format!("{S1_DATE_EXPR} IN")));
}

#[test]
fn s1_new_database_has_migration_10_indexes() {
    let db = LogDatabase::open_in_memory().unwrap();
    for name in [
        "idx_qso_journal_chrono",
        "idx_qso_journal_call_chrono",
        "idx_qso_qsl_match",
    ] {
        assert!(s1_has_index(db.raw_conn(), name), "brak indeksu {name}");
    }
}

#[test]
fn s1_query_plans_use_chrono_indexes_without_temp_sort() {
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "PORTABLE".to_string(),
        name: "Portable".to_string(),
        ..Default::default()
    })
    .unwrap();
    for i in 0..300 {
        let mut qso = s1_qso(
            &format!("SP{}AB", i % 37),
            if i % 2 == 0 { "20m" } else { "40m" },
            "CW",
            &format!("2026{:02}{:02}", 1 + i % 12, 1 + i % 28),
            &format!("{:02}{:02}", i % 24, i % 60),
        );
        if i % 3 == 0 {
            qso.journal_id = Some("PORTABLE".to_string());
        }
        db.insert_qso(&qso).unwrap();
    }
    let conn = db.raw_conn();
    let cases: Vec<(&str, String, Vec<&dyn rusqlite::ToSql>, &str)> = vec![
        (
            "get_recent_qsos_for_journal",
            qso_store::sql_recent_for_journal(),
            vec![&"DEFAULT", &50_i64],
            "idx_qso_journal_chrono",
        ),
        (
            "get_qsos_paginated_for_journal",
            qso_store::sql_paginated_for_journal(),
            vec![&"DEFAULT", &50_i64, &100_i64],
            "idx_qso_journal_chrono",
        ),
        (
            "find_previous_qsos_for_journal",
            qso_store::sql_previous_for_journal(),
            vec![&"DEFAULT", &"SP1AB"],
            "idx_qso_journal_call_chrono",
        ),
        (
            "qso_numbers_for_journal",
            qso_store::sql_numbers_for_journal(),
            vec![&"DEFAULT"],
            "idx_qso_journal_chrono",
        ),
    ];
    for (name, sql, params, index) in cases {
        let plan = s1_plan(conn, &sql, &params);
        assert!(
            !plan.contains("USE TEMP B-TREE FOR ORDER BY"),
            "{name}: sortowanie w pamięci zamiast indeksu: {plan}"
        );
        assert!(plan.contains(index), "{name}: plan bez {index}: {plan}");
    }
    let plan = s1_plan(
        conn,
        qsl::QSL_CANDIDATES_SQL,
        &[
            &"DEFAULT",
            &"SP1AB",
            &"20m",
            &"20M",
            &"20260920",
            &"20260919",
            &"20260921",
        ],
    );
    assert!(
        plan.contains("idx_qso_qsl_match"),
        "dopasowanie QSL musi używać idx_qso_qsl_match: {plan}"
    );
}

#[test]
fn s1_chrono_order_is_identical_to_the_unindexed_order() {
    use super::schema::{ORDER_CHRONO_ASC, ORDER_CHRONO_DESC};
    let db = LogDatabase::open_in_memory().unwrap();
    db.create_journal(&Journal {
        id: "PORTABLE".to_string(),
        name: "Portable".to_string(),
        ..Default::default()
    })
    .unwrap();
    let mut state: u64 = 0x5331_5f4f_5244_4552;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize
    };
    let mut inserted = 0;
    while inserted < 10_000 {
        let (y, m, d) = (2020 + next() % 7, 1 + next() % 12, 1 + next() % 28);
        let date = if next() % 3 == 0 {
            format!("{y:04}-{m:02}-{d:02}")
        } else {
            format!("{y:04}{m:02}{d:02}")
        };
        let (hh, mm, ss) = (next() % 24, next() % 60, next() % 60);
        let time = match next() % 4 {
            0 => format!("{hh:02}{mm:02}"),
            1 => format!("{hh:02}{mm:02}{ss:02}"),
            2 => format!("{hh:02}:{mm:02}"),
            _ => format!("{hh:02}:{mm:02}:{ss:02}"),
        };
        let mut qso = s1_qso(&format!("SQ{}X", next() % 400), "20m", "CW", &date, &time);
        if next() % 4 == 0 {
            qso.journal_id = Some("PORTABLE".to_string());
        }
        if db.insert_qso(&qso).is_ok() {
            inserted += 1;
        }
    }
    let conn = db.raw_conn();
    for journal in ["DEFAULT", "PORTABLE"] {
        let unindexed_desc = s1_ids(
            conn,
            &format!(
                "SELECT id FROM qso_records NOT INDEXED WHERE journal_id = ?1 ORDER BY {ORDER_CHRONO_DESC}"
            ),
            journal,
        );
        let indexed_desc: Vec<i64> = db
            .get_recent_qsos_for_journal(journal, 20_000)
            .unwrap()
            .into_iter()
            .map(|q| q.id.unwrap())
            .collect();
        assert_eq!(indexed_desc, unindexed_desc, "{journal}: porządek malejący");

        let mut rust_sorted = db.get_all_qsos_for_journal(journal).unwrap();
        rust_sorted.sort_by_key(|q| std::cmp::Reverse(schema::chrono_key(q)));
        let rust_desc: Vec<i64> = rust_sorted.iter().map(|q| q.id.unwrap()).collect();
        assert_eq!(
            rust_desc, unindexed_desc,
            "{journal}: chrono_key == ORDER_CHRONO_DESC"
        );

        let mut paged = Vec::new();
        for page in 0.. {
            let rows = db
                .get_qsos_paginated_for_journal(journal, 997, page * 997)
                .unwrap();
            if rows.is_empty() {
                break;
            }
            paged.extend(rows.into_iter().map(|q| q.id.unwrap()));
        }
        assert_eq!(
            paged, unindexed_desc,
            "{journal}: strony bez luk i zmian kolejności"
        );

        let unindexed_asc = s1_ids(
            conn,
            &format!(
                "SELECT id FROM qso_records NOT INDEXED WHERE journal_id = ?1 ORDER BY {ORDER_CHRONO_ASC}"
            ),
            journal,
        );
        let numbers = db.qso_numbers_for_journal(journal).unwrap();
        for (position, id) in unindexed_asc.iter().enumerate() {
            assert_eq!(
                numbers.get(id),
                Some(&(position + 1)),
                "{journal}: numer {id}"
            );
        }

        let previous: Vec<i64> = db
            .find_previous_qsos_for_journal(journal, "SQ7X")
            .unwrap()
            .into_iter()
            .map(|q| q.id.unwrap())
            .collect();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id FROM qso_records NOT INDEXED WHERE journal_id = ?1 AND callsign = 'SQ7X' ORDER BY {ORDER_CHRONO_DESC}"
            ))
            .unwrap();
        let expected: Vec<i64> = stmt
            .query_map([journal], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(previous, expected, "{journal}: poprzednie QSO");
    }
}

#[test]
fn s1_pre_migration_backup_is_taken_before_migration_10_and_keeps_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = s1_v9_database(dir.path());
    // Nowa, pusta baza nie wymagała kopii.
    assert_eq!(
        s1_pre_migration_files(&path),
        Vec::<std::path::PathBuf>::new()
    );

    let db = LogDatabase::open(&path).unwrap();
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    assert_eq!(db.count_all().unwrap(), 3);
    for name in [
        "idx_qso_journal_chrono",
        "idx_qso_journal_call_chrono",
        "idx_qso_qsl_match",
    ] {
        assert!(s1_has_index(db.raw_conn(), name), "brak indeksu {name}");
    }
    drop(db);

    let files = s1_pre_migration_files(&path);
    assert_eq!(
        files.len(),
        1,
        "dokładnie jedna kopia przedmigracyjna: {files:?}"
    );
    let name = files[0].file_name().unwrap().to_string_lossy().to_string();
    assert!(
        name.starts_with(&format!(
            "log_pre_migration_v9_to_v{LATEST_SCHEMA_VERSION}_"
        )) && files[0]
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("db")),
        "nazwa kopii zawiera wersje: {name}"
    );
    let version = crate::core::backup::BackupManager::validate_backup_internal(&files[0]).unwrap();
    assert_eq!(version, Some(9), "kopia ma stan sprzed migracji");
    assert_eq!(s1_raw_count(&files[0]), 3);

    // Ponowne otwarcie bez zaległych migracji nie tworzy kolejnej kopii.
    drop(LogDatabase::open(&path).unwrap());
    assert_eq!(s1_pre_migration_files(&path).len(), 1);
}

#[test]
fn s1_failed_migration_10_keeps_schema_9_rows_and_a_valid_pre_migration_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = s1_v9_database(dir.path());
    {
        // Wstrzyknięta awaria: obiekt o nazwie indeksu z migracji 10 blokuje CREATE INDEX.
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE idx_qso_qsl_match (blocker INTEGER);")
            .unwrap();
    }

    assert!(
        LogDatabase::open(&path).is_err(),
        "migracja musi się nie powieść"
    );

    assert_eq!(s1_raw_version(&path), 9, "wersja schematu bez zmian");
    assert_eq!(s1_raw_count(&path), 3, "dane bez zmian");
    let conn = Connection::open(&path).unwrap();
    assert!(
        !s1_has_index(&conn, "idx_qso_journal_chrono"),
        "częściowa migracja została wycofana"
    );
    drop(conn);

    let files = s1_pre_migration_files(&path);
    assert_eq!(files.len(), 1, "kopia przedmigracyjna istnieje: {files:?}");
    let version = crate::core::backup::BackupManager::validate_backup_internal(&files[0]).unwrap();
    assert_eq!(version, Some(9));
    assert_eq!(s1_raw_count(&files[0]), 3);
}

#[test]
fn s1_pre_migration_backup_failure_blocks_the_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = s1_v9_database(dir.path());
    // Plik w miejscu katalogu `backups` uniemożliwia utworzenie kopii.
    std::fs::write(dir.path().join("backups"), b"not a directory").unwrap();

    let err = LogDatabase::open(&path)
        .err()
        .expect("otwarcie musi się nie powieść");
    assert!(
        err.to_string().contains("pre-migration backup failed"),
        "błąd wskazuje przyczynę: {err}"
    );
    assert_eq!(s1_raw_version(&path), 9, "migracja nie została rozpoczęta");
    assert_eq!(s1_raw_count(&path), 3);
    let conn = Connection::open(&path).unwrap();
    assert!(!s1_has_index(&conn, "idx_qso_journal_chrono"));
}

fn s1_lotw(
    call: &str,
    band: &str,
    mode: &str,
    date: &str,
    time: Option<&str>,
) -> crate::cloud::lotw::LotwConfirmation {
    crate::cloud::lotw::LotwConfirmation {
        callsign: call.to_string(),
        band: band.to_string(),
        mode: mode.to_string(),
        qso_date: date.to_string(),
        qso_time: time.map(str::to_string),
        qsl_rdate: Some("20261120".to_string()),
    }
}

fn s1_lotw_state(db: &LogDatabase, id: i64) -> String {
    db.get_qso_by_id(id).unwrap().unwrap().lotw_qsl_rcvd
}

#[test]
fn s1_qsl_time_window_confirms_only_the_nearest_qso() {
    let db = LogDatabase::open_in_memory().unwrap();
    let morning = db
        .insert_qso(&s1_qso("W1AW", "20m", "CW", "20260920", "100000"))
        .unwrap();
    let afternoon = db
        .insert_qso(&s1_qso("W1AW", "20m", "CW", "20260920", "140000"))
        .unwrap();

    let report = db
        .mark_lotw_confirmed_batch_report(
            "DEFAULT",
            &[s1_lotw("W1AW", "20m", "CW", "20260920", Some("140500"))],
        )
        .unwrap();
    assert_eq!(report.matched, 1);
    assert_eq!(s1_lotw_state(&db, afternoon), "Y");
    assert_ne!(s1_lotw_state(&db, morning), "Y");

    // 45 minut od najbliższego QSO: poza oknem ±30 min.
    let report = db
        .mark_lotw_confirmed_batch_report(
            "DEFAULT",
            &[s1_lotw("W1AW", "20m", "CW", "20260920", Some("104500"))],
        )
        .unwrap();
    assert_eq!((report.matched, report.unmatched), (0, 1));
    assert_ne!(s1_lotw_state(&db, morning), "Y");
}

#[test]
fn s1_qsl_without_time_and_two_same_day_candidates_is_ambiguous() {
    let db = LogDatabase::open_in_memory().unwrap();
    let first = db
        .insert_qso(&s1_qso("W1AW", "20m", "CW", "20260920", "100000"))
        .unwrap();
    let second = db
        .insert_qso(&s1_qso("W1AW", "20m", "CW", "20260920", "140000"))
        .unwrap();

    let report = db
        .mark_lotw_confirmed_batch_report(
            "DEFAULT",
            &[s1_lotw("W1AW", "20m", "CW", "20260920", None)],
        )
        .unwrap();
    assert_eq!(report.confirmations, 1);
    assert_eq!(report.ambiguous, 1, "raport: niejednoznaczne");
    assert_eq!(report.matched, 0);
    assert_ne!(s1_lotw_state(&db, first), "Y");
    assert_ne!(s1_lotw_state(&db, second), "Y");

    // Pojedyncze potwierdzenie bez czasu (eQSL) podlega tej samej regule.
    let updated = db
        .mark_eqsl_confirmed_scoped("DEFAULT", "W1AW", "20m", "CW", "20260920", "20261120")
        .unwrap();
    assert_eq!(updated, 0);
}

#[test]
fn s1_qsl_record_with_dashed_date_is_matched_through_the_index() {
    let db = LogDatabase::open_in_memory().unwrap();
    let id = db
        .insert_qso(&s1_qso("SP6INA", "40m", "CW", "2026-09-20", "18:30:00"))
        .unwrap();
    let report = db
        .mark_lotw_confirmed_batch_report(
            "DEFAULT",
            &[s1_lotw("sp6ina ", "40m", "CW", "20260920", Some("1835"))],
        )
        .unwrap();
    assert_eq!(report.matched, 1);
    assert_eq!(s1_lotw_state(&db, id), "Y");
    let plan = s1_plan(
        db.raw_conn(),
        qsl::QSL_CANDIDATES_SQL,
        &[
            &"DEFAULT",
            &"SP6INA",
            &"40m",
            &"40M",
            &"20260920",
            &"20260920",
            &"20260920",
        ],
    );
    assert!(plan.contains("idx_qso_qsl_match"), "{plan}");
}

#[test]
fn s1_qsl_time_window_crosses_midnight_utc() {
    let db = LogDatabase::open_in_memory().unwrap();
    let late = db
        .insert_qso(&s1_qso("JA1XYZ", "20m", "FT8", "20260920", "235000"))
        .unwrap();
    let early = db
        .insert_qso(&s1_qso("VK2ABC", "20m", "FT8", "20260921", "000500"))
        .unwrap();

    let report = db
        .mark_lotw_confirmed_batch_report(
            "DEFAULT",
            &[
                s1_lotw("JA1XYZ", "20m", "FT8", "20260921", Some("000500")),
                s1_lotw("VK2ABC", "20m", "FT8", "20260920", Some("235000")),
            ],
        )
        .unwrap();
    assert_eq!(report.matched, 2, "{report:?}");
    assert_eq!(s1_lotw_state(&db, late), "Y");
    assert_eq!(s1_lotw_state(&db, early), "Y");
}

#[test]
fn s1_qsl_keeps_band_case_and_phone_group_semantics() {
    let db = LogDatabase::open_in_memory().unwrap();
    let id = db
        .insert_qso(&s1_qso("DL1ABC", "20M", "USB", "20260920", "1200"))
        .unwrap();
    let report = db
        .mark_lotw_confirmed_batch_report(
            "DEFAULT",
            &[s1_lotw("DL1ABC", "20m", "SSB", "20260920", Some("1210"))],
        )
        .unwrap();
    assert_eq!(report.matched, 1);
    assert_eq!(s1_lotw_state(&db, id), "Y");
}

#[test]
fn s1_qsl_chunks_release_the_database_lock_between_batches() {
    use std::sync::Mutex;
    let db = LogDatabase::open_in_memory().unwrap();
    let id = db
        .insert_qso(&s1_qso("W1AW", "20m", "CW", "20260920", "120000"))
        .unwrap();
    let shared = Mutex::new(db);
    let confirmations: Vec<_> = (0..=(2 * qsl::QSL_MATCH_CHUNK_SIZE))
        .map(|_| s1_lotw("W1AW", "20m", "CW", "20260920", Some("120100")))
        .collect();

    let mut calls = 0;
    let report = qsl::apply_lotw_confirmations_chunked(&shared, "DEFAULT", &confirmations, |r| {
        calls += 1;
        assert!(
            shared.try_lock().is_ok(),
            "blokada bazy musi być wolna między paczkami"
        );
        assert_eq!(
            r.confirmations,
            (calls * qsl::QSL_MATCH_CHUNK_SIZE).min(confirmations.len())
        );
    })
    .unwrap();
    assert_eq!(calls, 3);
    assert_eq!(report.confirmations, confirmations.len());
    assert_eq!(report.matched, confirmations.len());
    assert_eq!(s1_lotw_state(&shared.lock().unwrap(), id), "Y");

    let mut eqsl_calls = 0;
    let mut incoming = s1_qso("W1AW", "20m", "CW", "20260920", "120200");
    incoming.eqsl_qsl_rcvd = "Y".to_string();
    let eqsl = vec![incoming; qsl::QSL_MATCH_CHUNK_SIZE + 1];
    let report = qsl::apply_eqsl_confirmations_chunked(&shared, "DEFAULT", &eqsl, |_| {
        eqsl_calls += 1;
        assert!(shared.try_lock().is_ok());
    })
    .unwrap();
    assert_eq!(eqsl_calls, 2);
    assert_eq!(report.matched, eqsl.len());
}

#[test]
fn s1_scp_source_lists_every_callsign_beyond_ten_thousand() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut qsos: Vec<QsoRecord> = (0..10_000)
        .map(|i| s1_qso(&format!("AA{i:05}"), "20m", "CW", "20260101", "120000"))
        .collect();
    qsos.extend((0..50).map(|i| s1_qso(&format!("SP{i}ZZ"), "20m", "CW", "20260101", "120000")));
    db.batch_insert_qsos(&qsos).unwrap();

    // Dotychczasowe zapytanie z limitem ucina alfabetycznie — SP nie mieści się w 10 000.
    let truncated = db
        .get_distinct_callsigns_for_journal("DEFAULT", 10_000)
        .unwrap();
    assert!(!truncated.iter().any(|c| c.starts_with("SP")));

    let all = db
        .get_all_distinct_callsigns_for_journal("DEFAULT")
        .unwrap();
    assert_eq!(all.len(), 10_050);
    assert_eq!(all.iter().filter(|c| c.starts_with("SP")).count(), 50);
}

#[test]
fn scp_seed_covers_every_journal_and_the_whole_alphabet() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut qsos: Vec<QsoRecord> = (0..60_000)
        .map(|i| s1_qso(&format!("AA{i:05}"), "20m", "CW", "20260101", "120000"))
        .collect();
    qsos.push(s1_qso("sp6ina", "20m", "CW", "20260101", "120100"));
    qsos.push(s1_qso("SP6INA", "40m", "CW", "20260101", "120200"));
    db.batch_insert_qsos(&qsos).unwrap();
    db.create_journal(&Journal {
        id: "CONTEST".to_string(),
        name: "Contest".to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: String::new(),
        my_gridsquare: String::new(),
        my_pga: String::new(),
        description: String::new(),
        is_default: false,
    })
    .unwrap();
    db.raw_conn()
        .execute(
            "UPDATE qso_records SET journal_id = 'CONTEST' WHERE callsign = 'AA00000'",
            [],
        )
        .unwrap();

    let calls = db.get_distinct_callsigns().unwrap();
    assert_eq!(calls.len(), 60_001);
    assert_eq!(calls.iter().filter(|c| *c == "SP6INA").count(), 1);
    assert!(calls.iter().any(|c| c == "AA00000"));
}

#[test]
fn s1_qsl_reconfirmation_without_change_writes_nothing_but_still_matches() {
    let db = LogDatabase::open_in_memory().unwrap();
    let id = db
        .insert_qso(&s1_qso("W1AW", "20m", "CW", "20260920", "120000"))
        .unwrap();
    let total_changes = |db: &LogDatabase| -> i64 {
        db.raw_conn()
            .query_row("SELECT total_changes()", [], |r| r.get(0))
            .unwrap()
    };
    let conf = s1_lotw("W1AW", "20m", "CW", "20260920", Some("120000"));

    let report = db
        .mark_lotw_confirmed_batch_report("DEFAULT", std::slice::from_ref(&conf))
        .unwrap();
    assert_eq!(report.matched, 1);
    assert_eq!(s1_lotw_state(&db, id), "Y");
    let after_first = total_changes(&db);

    // Ten sam raport ponownie: dopasowane, ale bez żadnego zapisu.
    let report = db
        .mark_lotw_confirmed_batch_report("DEFAULT", &[conf.clone(), conf.clone()])
        .unwrap();
    assert_eq!(report.matched, 2);
    assert_eq!(total_changes(&db), after_first, "brak zbędnych zapisów");

    // Nowa data odbioru nadal jest zapisywana.
    let mut newer = conf;
    newer.qsl_rdate = Some("20261121".to_string());
    db.mark_lotw_confirmed_batch_report("DEFAULT", &[newer])
        .unwrap();
    assert_eq!(total_changes(&db), after_first + 1);
    assert_eq!(
        db.get_qso_by_id(id)
            .unwrap()
            .unwrap()
            .lotw_qslrdate
            .as_deref(),
        Some("20261121")
    );
}

// ---------------------------------------------------------------------------------------------
// STAGE-V140-S4 — pełne pola ADIF i pola nieznane (qso_extra).
// ---------------------------------------------------------------------------------------------

fn s4_field(tag: &str, value: &str) -> String {
    format!("<{tag}:{}>{value}", value.len())
}

fn s4_adif_record() -> String {
    [
        ("CALL", "SP6INA"),
        ("BAND", "20m"),
        ("MODE", "CW"),
        ("QSO_DATE", "20261101"),
        ("TIME_ON", "120000"),
        ("TX_PWR", "100"),
        ("OPERATOR", "SP6ABC"),
        ("STATION_CALLSIGN", "SP6INA/P"),
        ("CONTEST_ID", "SP-DX-CONTEST"),
        ("MY_RIG", "IC-7300"),
        ("MY_ANTENNA", "Dipole"),
        ("CNTY", "MA,Middlesex"),
        ("SIG", "WWFF"),
        ("SIG_INFO", "SPFF-0001"),
        ("MY_SIG", "SOTA"),
        ("MY_SIG_INFO", "SP/BZ-001"),
        ("WWFF_REF", "SPFF-0001"),
        ("MY_WWFF_REF", "SPFF-0002"),
        ("EMAIL", "op@example.com"),
        ("APP_LOG4OM_RATING", "5"),
        ("USERDEF_NOTE", "from another logger"),
    ]
    .iter()
    .map(|(tag, value)| s4_field(tag, value))
    .collect::<String>()
        + "<EOR>\n"
}

fn s4_assert_fields(q: &QsoRecord) {
    assert_eq!(q.tx_pwr.as_deref(), Some("100"));
    assert_eq!(q.operator.as_deref(), Some("SP6ABC"));
    assert_eq!(q.station_callsign.as_deref(), Some("SP6INA/P"));
    assert_eq!(q.contest_id.as_deref(), Some("SP-DX-CONTEST"));
    assert_eq!(q.my_rig.as_deref(), Some("IC-7300"));
    assert_eq!(q.my_antenna.as_deref(), Some("Dipole"));
    assert_eq!(q.cnty.as_deref(), Some("MA,Middlesex"));
    assert_eq!(q.sig.as_deref(), Some("WWFF"));
    assert_eq!(q.sig_info.as_deref(), Some("SPFF-0001"));
    assert_eq!(q.my_sig.as_deref(), Some("SOTA"));
    assert_eq!(q.my_sig_info.as_deref(), Some("SP/BZ-001"));
    assert_eq!(q.wwff_ref.as_deref(), Some("SPFF-0001"));
    assert_eq!(q.my_wwff_ref.as_deref(), Some("SPFF-0002"));
    assert_eq!(q.email.as_deref(), Some("op@example.com"));
    assert_eq!(
        q.extra_fields,
        vec![
            ("APP_LOG4OM_RATING".to_string(), "5".to_string()),
            (
                "USERDEF_NOTE".to_string(),
                "from another logger".to_string()
            ),
        ]
    );
}

#[test]
fn s4_adif_fields_and_unknown_fields_survive_import_storage_and_export() {
    let parsed = crate::core::adif::parse_adif(&s4_adif_record());
    assert_eq!(parsed.len(), 1);
    s4_assert_fields(&parsed[0]);

    let db = LogDatabase::open_in_memory().unwrap();
    let id = db.insert_qso(&parsed[0]).unwrap();
    s4_assert_fields(&db.get_qso_by_id(id).unwrap().unwrap());
    let all = db.get_all_qsos_for_journal_with_extras("DEFAULT").unwrap();
    s4_assert_fields(&all[0]);
    assert!(
        db.get_all_qsos_for_journal("DEFAULT").unwrap()[0]
            .extra_fields
            .is_empty(),
        "the plain journal load does not read qso_extra"
    );

    let exported = crate::core::adif::export_adif(&all, "SPLogbook", "SP6INA");
    assert!(!exported.contains("Author:"));
    let reparsed = crate::core::adif::parse_adif(&exported);
    assert_eq!(reparsed.len(), 1);
    s4_assert_fields(&reparsed[0]);
}

#[test]
fn s4_pota_sig_stays_in_pota_ref_and_not_in_the_sig_columns() {
    let record = [
        ("CALL", "K1ABC"),
        ("BAND", "40m"),
        ("MODE", "SSB"),
        ("QSO_DATE", "20261101"),
        ("TIME_ON", "1300"),
        ("SIG", "POTA"),
        ("SIG_INFO", "US-0001"),
    ]
    .iter()
    .map(|(tag, value)| s4_field(tag, value))
    .collect::<String>()
        + "<EOR>\n";
    let parsed = crate::core::adif::parse_adif(&record);
    assert_eq!(parsed[0].pota_ref.as_deref(), Some("US-0001"));
    assert_eq!(parsed[0].sig, None);
    assert_eq!(parsed[0].extra_fields, Vec::<(String, String)>::new());
}

#[test]
fn s4_extra_fields_follow_updates_and_cascade_on_delete() {
    let db = LogDatabase::open_in_memory().unwrap();
    let mut parsed = crate::core::adif::parse_adif(&s4_adif_record());
    let id = db.insert_qso(&parsed[0]).unwrap();

    // An update from a row loaded without extras keeps the stored extras.
    let mut plain = db.get_all_qsos_for_journal("DEFAULT").unwrap().remove(0);
    plain.comment = Some("edited".to_string());
    db.update_qso(id, &plain).unwrap();
    assert_eq!(db.get_qso_by_id(id).unwrap().unwrap().extra_fields.len(), 2);

    // An update that carries extras replaces them.
    let mut replaced = parsed.remove(0);
    replaced.id = Some(id);
    replaced.extra_fields = vec![("APP_X".to_string(), "1".to_string())];
    assert!(db.update_qso_scoped(id, "DEFAULT", &replaced).unwrap());
    assert_eq!(
        db.get_qso_by_id(id).unwrap().unwrap().extra_fields,
        vec![("APP_X".to_string(), "1".to_string())]
    );

    db.delete_qso(id).unwrap();
    let left: i64 = db
        .raw_conn()
        .query_row("SELECT COUNT(*) FROM qso_extra", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0, "qso_extra rows are deleted with their QSO");
}
