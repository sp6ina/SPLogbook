// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::*;

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
    // Wszystkie kroki migracji zostały zarejestrowane.
    assert_eq!(db.current_schema_version().unwrap(), 4);

    // Kolumny z migracji 2 istnieją i ponowna inicjalizacja nie psuje schematu.
    let col_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('qso_records') WHERE name IN ('journal_id','my_pota_ref','my_sota_ref','vucc_grids','audio_file')",
                [],
                |r| r.get(0),
            )
            .unwrap();
    assert_eq!(col_count, 5);

    // Indeksy z migracji 4 istnieją.
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
    assert_eq!(db.current_schema_version().unwrap(), 4);
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
    qso.qso_date = "2026-01-01".into();
    qso.time_on = "12:00:00".into();
    let first = db.insert_qso(&qso).unwrap();
    qso.journal_id = Some("PORTABLE".into());
    let portable = db.insert_qso(&qso).unwrap();
    qso.journal_id = Some("DEFAULT".into());
    for _ in 0..501 {
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
        operator: "Mariusz Woźniak".to_string(),
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

    // Przełączenie na nieistniejący dziennik zwraca błąd.
    assert!(db.set_active_journal("GHOST").is_err());
    // Po nieudanym przełączeniu aktywny pozostaje DEFAULT.
    let active = db.get_active_journal().unwrap();
    assert_eq!(active.id, "DEFAULT");

    // Utworzenie drugiego dziennika i przełączenie na niego.
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

    // Usunięcie aktywnego dziennika jest blokowane.
    assert!(db.delete_journal("PORTABLE").is_err());

    // Powrót na DEFAULT i próba usunięcia DEFAULT — zablokowana.
    db.set_active_journal("DEFAULT").unwrap();
    assert!(db.delete_journal("DEFAULT").is_err());

    // Osierocone QSO są przepinane do DEFAULT przy usunięciu.
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
    let mut db = LogDatabase::open_in_memory().unwrap();
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
    let mut db = LogDatabase::open_in_memory().unwrap();
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
    updated.qth = Some("Wrocław".to_string());
    updated.gridsquare = Some("JO81WA".to_string());
    updated.pga_ref = Some("WR01".to_string());

    db.update_qso(id, &updated).unwrap();

    let list = db.find_previous_qsos("SP6INA").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].rst_sent, "579");
    assert_eq!(list[0].name.as_deref(), Some("Mariusz SP6INA"));
    assert_eq!(list[0].qth.as_deref(), Some("Wrocław"));
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
        "Wyszukiwanie z apostrofem nie powinno powodować błędu SQL"
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

    let dups = db.find_duplicate_qsos(false).unwrap();
    assert_eq!(
        dups.len(),
        1,
        "Powinna być dokładnie jedna grupa duplikatów (SP6INA / 20m / CW)"
    );
    assert_eq!(dups[0].len(), 2);

    let deleted = db.delete_multiple_qsos(&[id2]).unwrap();
    assert_eq!(deleted, 1);

    let dups_after = db.find_duplicate_qsos(false).unwrap();
    assert_eq!(dups_after.len(), 0, "Brak duplikatów po usunięciu");

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
        eqsl_qsl_sent: "N".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
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
        eqsl_qsl_sent: "Q".to_string(),
        eqsl_qsl_rcvd: "W".to_string(),
        eqsl_qslrdate: Some("20260405".to_string()),
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
        eqsl_qsl_sent: "Q".to_string(),
        eqsl_qsl_rcvd: "W".to_string(),
        eqsl_qslrdate: Some("20260405".to_string()),
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
        eqsl_qsl_sent: "N".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
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
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260509".to_string()),
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
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260509".to_string()),
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
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260605".to_string()),
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
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "Y".to_string(),
        eqsl_qslrdate: Some("20260605".to_string()),
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
    };

    assert_eq!(retrieved, expected);
}

#[test]
fn batch_insert_roundtrip() {
    let mut db = LogDatabase::open_in_memory().unwrap();
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
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
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
        eqsl_qsl_sent: "Y".to_string(),
        eqsl_qsl_rcvd: "N".to_string(),
        eqsl_qslrdate: None,
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
    ];

    assert_eq!(expected.len(), 54);
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
    assert_eq!(db.current_schema_version().unwrap(), 4);

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
            );
            INSERT INTO schema_version (version) VALUES (5);",
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
