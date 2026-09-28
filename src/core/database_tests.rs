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
    assert_eq!(db.current_schema_version().unwrap(), 3);

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

    db.init_schema().unwrap();
    assert_eq!(db.current_schema_version().unwrap(), 3);
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
