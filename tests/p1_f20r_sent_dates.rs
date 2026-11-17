// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! STAGE-P1-F20R-SOFTWARE — ADIF `LOTW_QSLSDATE` / `EQSL_QSLSDATE` (ryzyko **F20-R2**).
//!
//! Dane są syntetyczne (`TST…`), baza jest w pamięci lub w katalogu tymczasowym,
//! nie ma sieci ani prawdziwych poświadczeń. Zegar jest **wstrzykiwany** — każdy
//! znacznik czasu jest jawną stałą, żaden test nie czyta zegara ściennego.

use splogbook::cloud::upload_checkpoint::{
    BatchState, OutcomeSnapshot, adif_date_from_unix, compute_payload_digest,
};
use splogbook::cloud::upload_plan::{
    BatchOutcome, JournalId, SelectionPolicy, UploadPlan, UploadTarget,
};
use splogbook::core::adif::{export_adif, parse_adif};
use splogbook::core::database::upload_checkpoint::CheckpointError;
use splogbook::core::database::{Journal, LogDatabase};
use splogbook::core::qso::QsoRecord;

const JOURNAL: &str = "F20R_DATES";
const BATCH: usize = 8;
/// 2026-11-17T12:00:00Z — deterministyczna „chwila akceptacji”.
const ACCEPTED_AT: u64 = 1_794_916_800;

fn journal() -> Journal {
    Journal {
        id: JOURNAL.to_string(),
        name: "dates journal".to_string(),
        station_callsign: "TSTSTN".to_string(),
        operator: "TSTOP".to_string(),
        my_gridsquare: "JO90AA".to_string(),
        my_pga: String::new(),
        description: "synthetic F-20R dates".to_string(),
        is_default: true,
    }
}

fn seeded(count: usize) -> LogDatabase {
    let db = LogDatabase::open_in_memory().expect("db");
    db.create_journal(&journal()).expect("journal");
    let qsos: Vec<QsoRecord> = (0..count)
        .map(|i| {
            let mut qso = QsoRecord::new(format!("TST{i:05}"), "20m", "CW");
            qso.journal_id = Some(JOURNAL.to_string());
            // QSO_DATE celowo różni się od daty wysłania.
            qso.qso_date = "20250101".to_string();
            qso.time_on = format!("{:04}00", i % 2400);
            qso
        })
        .collect();
    db.batch_insert_qsos(&qsos).expect("insert");
    db
}

fn plan_for(db: &LogDatabase, target: UploadTarget) -> UploadPlan {
    let journal_id = JournalId::new(JOURNAL).expect("journal id");
    let records = db
        .select_upload_candidates_for_journal(&journal_id, target)
        .expect("candidates");
    UploadPlan::build(
        target,
        journal_id,
        SelectionPolicy::PendingOnly,
        "attempt-dates",
        1_700_000_000,
        records,
        BATCH,
    )
    .expect("plan")
}

fn drive(
    db: &LogDatabase,
    plan_id: &str,
    ordinal: usize,
    outcome: &BatchOutcome,
    count: usize,
    accepted_at: u64,
) {
    db.prepare_upload_batch(plan_id, ordinal, &compute_payload_digest(b"payload"), 1_000)
        .expect("prepare");
    db.start_upload_batch(plan_id, ordinal, 1_001)
        .expect("start");
    db.record_upload_batch_outcome(
        plan_id,
        ordinal,
        OutcomeSnapshot::from_outcome(outcome, count),
        accepted_at,
    )
    .expect("outcome");
}

fn snapshots(db: &LogDatabase) -> Vec<QsoRecord> {
    db.get_all_qsos_for_journal(JOURNAL).expect("qsos")
}

// ---------------------------------------------------------------------------
// Accepted: status i data w jednej transakcji
// ---------------------------------------------------------------------------

#[test]
fn lotw_accepted_sets_status_and_sent_date_atomically() {
    let db = seeded(BATCH);
    let plan = plan_for(&db, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let count = db.load_batch(plan.plan_id(), 0).unwrap().record_count;
    drive(
        &db,
        plan.plan_id(),
        0,
        &BatchOutcome::Accepted,
        count,
        ACCEPTED_AT,
    );
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, ACCEPTED_AT + 60)
        .expect("commit");

    let expected = adif_date_from_unix(ACCEPTED_AT as i64).expect("date");
    assert_eq!(expected, "20261117");
    for qso in snapshots(&db) {
        assert_eq!(qso.lotw_qsl_sent, "Y");
        assert_eq!(qso.lotw_qslsdate.as_deref(), Some(expected.as_str()));
        // eQSL pozostaje nietknięty.
        assert_eq!(qso.eqsl_qsl_sent, "N");
        assert!(qso.eqsl_qslsdate.is_none());
        // Data wysłania nie pochodzi z QSO_DATE.
        assert_ne!(qso.qso_date, expected);
    }
    assert_eq!(
        db.load_batch(plan.plan_id(), 0).unwrap().state,
        BatchState::Completed
    );
}

#[test]
fn eqsl_accepted_sets_status_and_sent_date_atomically() {
    let db = seeded(BATCH);
    let plan = plan_for(&db, UploadTarget::Eqsl);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let count = db.load_batch(plan.plan_id(), 0).unwrap().record_count;
    drive(
        &db,
        plan.plan_id(),
        0,
        &BatchOutcome::Accepted,
        count,
        ACCEPTED_AT,
    );
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, ACCEPTED_AT + 60)
        .expect("commit");

    let expected = adif_date_from_unix(ACCEPTED_AT as i64).expect("date");
    for qso in snapshots(&db) {
        assert_eq!(qso.eqsl_qsl_sent, "Y");
        assert_eq!(qso.eqsl_qslsdate.as_deref(), Some(expected.as_str()));
        assert_eq!(qso.lotw_qsl_sent, "N");
        assert!(qso.lotw_qslsdate.is_none());
    }
}

#[test]
fn a_retry_cannot_overwrite_the_originally_accepted_sent_date() {
    let db = seeded(BATCH);
    let plan = plan_for(&db, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let count = set.batches[0].record_count;

    // Jeden z rekordów paczki ma już potwierdzoną, wcześniejszą datę wysłania.
    // `COALESCE(<kolumna>, ?data)` musi ją zachować, a nie nadpisać nowszą.
    let ids = db.load_batch_members(plan.plan_id(), 0).expect("members");
    let existing = db.get_qso_by_id(ids[0]).expect("qso").expect("exists");
    let mut qso = existing.clone();
    qso.lotw_qsl_sent = "Y".to_string();
    qso.lotw_qslsdate = Some("20260101".to_string());
    db.update_qso(ids[0], &qso).expect("pre-set date");

    drive(
        &db,
        plan.plan_id(),
        0,
        &BatchOutcome::Accepted,
        count,
        ACCEPTED_AT,
    );
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, ACCEPTED_AT + 60)
        .expect("commit");

    let original = db.get_qso_by_id(ids[0]).expect("qso").expect("exists");
    assert_eq!(
        original.lotw_qslsdate.as_deref(),
        Some("20260101"),
        "pierwotna data akceptacji nie może zostać nadpisana"
    );
    let fresh = db.get_qso_by_id(ids[1]).expect("qso").expect("exists");
    assert_eq!(
        fresh.lotw_qslsdate.as_deref(),
        Some(adif_date_from_unix(ACCEPTED_AT as i64).unwrap().as_str())
    );
}

// ---------------------------------------------------------------------------
// Wyniki nie będące akceptacją nie zapisują daty
// ---------------------------------------------------------------------------

#[test]
fn rejected_timeout_cancelled_partial_and_unknown_never_write_a_sent_date() {
    let outcomes = [
        BatchOutcome::Rejected {
            detail: "TEST_REJECTED".to_string(),
        },
        BatchOutcome::TimedOut {
            detail: "TEST_TIMEOUT".to_string(),
        },
        BatchOutcome::Cancelled,
        BatchOutcome::PartiallyAccepted {
            rejected: 2,
            detail: "TEST_PARTIAL".to_string(),
        },
        BatchOutcome::NoRecordsProcessed {
            detail: "TEST_NONE".to_string(),
        },
        BatchOutcome::Unknown {
            detail: "TEST_UNKNOWN".to_string(),
        },
        BatchOutcome::Failed {
            detail: "TEST_FAILED".to_string(),
        },
    ];
    for outcome in outcomes {
        let db = seeded(BATCH);
        let plan = plan_for(&db, UploadTarget::Lotw);
        db.open_or_create_upload_checkpoint(&plan)
            .expect("checkpoint");
        let count = db.load_batch(plan.plan_id(), 0).unwrap().record_count;
        drive(&db, plan.plan_id(), 0, &outcome, count, ACCEPTED_AT);

        let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
        assert_ne!(batch.state, BatchState::Accepted, "{outcome:?}");
        assert!(!batch.state.awaits_local_commit(), "{outcome:?}");

        // Commit jest niemożliwy — nie ma dowodu akceptacji.
        let err = db
            .commit_batch_acceptance_with_dates(plan.plan_id(), 0, ACCEPTED_AT + 60)
            .expect_err("commit without evidence must fail");
        assert!(
            matches!(err, CheckpointError::TransitionRejected { .. }),
            "{outcome:?}: {err:?}"
        );

        for qso in snapshots(&db) {
            assert_ne!(qso.lotw_qsl_sent, "Y", "{outcome:?}");
            assert!(qso.lotw_qslsdate.is_none(), "{outcome:?}");
        }
    }
}

#[test]
fn cancelled_plan_writes_no_sent_date() {
    let db = seeded(BATCH);
    let plan = plan_for(&db, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    db.cancel_upload_plan(plan.plan_id(), 5_000)
        .expect("cancel");
    for qso in snapshots(&db) {
        assert_eq!(qso.lotw_qsl_sent, "N");
        assert!(qso.lotw_qslsdate.is_none());
    }
}

// ---------------------------------------------------------------------------
// Atomowość i rollback
// ---------------------------------------------------------------------------

#[test]
fn a_failed_row_count_rolls_back_both_status_and_date() {
    let db = seeded(BATCH);
    let plan = plan_for(&db, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let ids = db.load_batch_members(plan.plan_id(), 0).expect("members");
    let count = ids.len();
    drive(
        &db,
        plan.plan_id(),
        0,
        &BatchOutcome::Accepted,
        count,
        ACCEPTED_AT,
    );

    // Usuwamy jeden rekord paczki: `affected != expected`, więc transakcja
    // musi zostać wycofana w całości — ani status, ani data.
    db.delete_qso(ids[count - 1]).expect("delete");

    let err = db
        .commit_batch_acceptance_with_dates(plan.plan_id(), 0, ACCEPTED_AT + 60)
        .expect_err("row count mismatch");
    assert!(matches!(
        err,
        CheckpointError::RowCountMismatch { expected, affected }
            if expected == count && affected == count - 1
    ));

    let qsos = snapshots(&db);
    assert_eq!(qsos.len(), count - 1);
    for qso in qsos {
        assert_ne!(qso.lotw_qsl_sent, "Y");
        assert!(qso.lotw_qslsdate.is_none());
    }
    assert_eq!(
        db.load_batch(plan.plan_id(), 0).unwrap().state,
        BatchState::Accepted,
        "po rollbacku paczka nadal czeka na lokalny commit"
    );
}

#[test]
fn legacy_accepted_rows_keep_status_without_a_sent_date() {
    let db = seeded(3);
    let qsos = snapshots(&db);
    let id = qsos[0].id.expect("id");
    let mut qso = qsos[0].clone();
    qso.lotw_qsl_sent = "Y".to_string();
    qso.lotw_qslsdate = None;
    qso.eqsl_qsl_sent = "I".to_string();
    qso.eqsl_qslsdate = None;
    db.update_qso(id, &qso).expect("update");

    let reloaded = db.get_qso_by_id(id).expect("qso").expect("exists");
    assert_eq!(reloaded.lotw_qsl_sent, "Y");
    assert!(reloaded.lotw_qslsdate.is_none());

    // `Y` jest wykluczone z kolejnego wyboru — historia nie jest przepisywana.
    let journal_id = JournalId::new(JOURNAL).expect("journal id");
    let candidates = db
        .select_upload_candidates_for_journal(&journal_id, UploadTarget::Lotw)
        .expect("candidates");
    assert!(!candidates.iter().any(|r| r.id == id));
}

#[test]
fn status_n_and_i_without_dates_remain_untouched() {
    let db = seeded(3);
    for qso in snapshots(&db) {
        assert_eq!(qso.lotw_qsl_sent, "N");
        assert!(qso.lotw_qslsdate.is_none());
        assert_eq!(qso.eqsl_qsl_sent, "N");
        assert!(qso.eqsl_qslsdate.is_none());
    }
}

// ---------------------------------------------------------------------------
// ADIF import / export / round-trip
// ---------------------------------------------------------------------------

fn with_dates() -> QsoRecord {
    let mut qso = QsoRecord::new("TST1AA", "20m", "CW");
    qso.journal_id = Some(JOURNAL.to_string());
    qso.qso_date = "20250101".to_string();
    qso.time_on = "120000".to_string();
    qso.lotw_qsl_sent = "Y".to_string();
    qso.lotw_qslsdate = Some("20261117".to_string());
    qso.eqsl_qsl_sent = "Y".to_string();
    qso.eqsl_qslsdate = Some("20261118".to_string());
    qso
}

#[test]
fn export_emits_both_sent_dates_with_canonical_tags() {
    let adif = export_adif(&[with_dates()], "SPLogbook", "TSTSTN");
    assert!(adif.contains("<LOTW_QSLSDATE:8>20261117"), "{adif}");
    assert!(adif.contains("<EQSL_QSLSDATE:8>20261118"), "{adif}");
    assert!(adif.contains("<LOTW_QSL_SENT:1>Y"), "{adif}");
    assert!(adif.contains("<EQSL_QSL_SENT:1>Y"), "{adif}");
}

#[test]
fn export_omits_the_date_when_the_status_makes_it_invalid() {
    let mut qso = with_dates();
    qso.lotw_qsl_sent = "N".to_string();
    qso.eqsl_qsl_sent = "R".to_string();
    let adif = export_adif(&[qso], "SPLogbook", "TSTSTN");
    assert!(!adif.contains("LOTW_QSLSDATE"), "{adif}");
    assert!(!adif.contains("EQSL_QSLSDATE"), "{adif}");
}

#[test]
fn export_keeps_the_date_for_the_queued_and_ignored_statuses() {
    for status in ["Q", "I", "Y", "q", "i", "y"] {
        let mut qso = with_dates();
        qso.lotw_qsl_sent = status.to_string();
        let adif = export_adif(&[qso], "SPLogbook", "TSTSTN");
        assert!(
            adif.contains("<LOTW_QSLSDATE:8>20261117"),
            "status {status} is valid per ADIF: {adif}"
        );
    }
}

#[test]
fn import_reads_both_sent_dates() {
    let adif = "<CALL:6>TST1AA<BAND:3>20M<MODE:2>CW<QSO_DATE:8>20250101<TIME_ON:6>120000\
                <LOTW_QSL_SENT:1>Y<LOTW_QSLSDATE:8>20261117\
                <EQSL_QSL_SENT:1>Y<EQSL_QSLSDATE:8>20261118<EOR>";
    let parsed = parse_adif(adif);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].lotw_qslsdate.as_deref(), Some("20261117"));
    assert_eq!(parsed[0].eqsl_qslsdate.as_deref(), Some("20261118"));
}

#[test]
fn missing_sent_date_imports_as_none_and_is_never_invented() {
    let adif = "<CALL:6>TST1AA<BAND:3>20M<MODE:2>CW<QSO_DATE:8>20250101<TIME_ON:6>120000\
                <LOTW_QSL_SENT:1>Y<EOR>";
    let parsed = parse_adif(adif);
    assert_eq!(parsed[0].lotw_qsl_sent, "Y");
    assert!(parsed[0].lotw_qslsdate.is_none());
    assert!(parsed[0].eqsl_qslsdate.is_none());
}

#[test]
fn export_import_export_is_a_fixed_point_for_sent_dates() {
    let original = with_dates();
    let first = export_adif(std::slice::from_ref(&original), "SPLogbook", "TSTSTN");
    let parsed = parse_adif(&first);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].lotw_qslsdate, original.lotw_qslsdate);
    assert_eq!(parsed[0].eqsl_qslsdate, original.eqsl_qslsdate);
    let second = export_adif(&parsed, "SPLogbook", "TSTSTN");
    assert_eq!(first, second, "round-trip musi być punktem stałym");
}

#[test]
fn imported_date_with_separators_is_normalised_on_export() {
    let adif = "<CALL:6>TST1AA<BAND:3>20M<MODE:2>CW<QSO_DATE:8>20250101<TIME_ON:6>120000\
                <LOTW_QSL_SENT:1>Y<LOTW_QSLSDATE:10>2026-11-17<EOR>";
    let parsed = parse_adif(adif);
    assert_eq!(parsed[0].lotw_qslsdate.as_deref(), Some("2026-11-17"));
    let exported = export_adif(&parsed, "SPLogbook", "TSTSTN");
    assert!(exported.contains("<LOTW_QSLSDATE:8>20261117"), "{exported}");
}

#[test]
fn an_unusable_sent_date_is_omitted_instead_of_being_invented() {
    for invalid in ["2026", "2026-1-1", "202611", "", "20261301x", "not-a-date"] {
        let mut qso = with_dates();
        qso.lotw_qslsdate = Some(invalid.to_string());
        let adif = export_adif(&[qso], "SPLogbook", "TSTSTN");
        assert!(
            !adif.contains("LOTW_QSLSDATE"),
            "unusable date `{invalid}` must not be emitted: {adif}"
        );
    }
}

// ---------------------------------------------------------------------------
// Zegar, granice UTC, rok przestępny, koniec roku
// ---------------------------------------------------------------------------

#[test]
fn deterministic_clock_produces_the_same_date_for_the_same_instant() {
    let a = adif_date_from_unix(ACCEPTED_AT as i64).expect("a");
    let b = adif_date_from_unix(ACCEPTED_AT as i64).expect("b");
    assert_eq!(a, b);
    assert_eq!(a, "20261117");
}

#[test]
fn utc_day_boundary_is_respected() {
    // 2026-11-17T23:59:59Z
    assert_eq!(
        adif_date_from_unix(1_794_959_999),
        Some("20261117".to_string())
    );
    // 2026-11-18T00:00:00Z
    assert_eq!(
        adif_date_from_unix(1_794_960_000),
        Some("20261118".to_string())
    );
}

#[test]
fn end_of_year_and_leap_day_are_exact() {
    assert_eq!(
        adif_date_from_unix(1_798_761_599),
        Some("20261231".to_string())
    );
    assert_eq!(
        adif_date_from_unix(1_798_761_600),
        Some("20270101".to_string())
    );
    assert_eq!(
        adif_date_from_unix(1_835_438_400),
        Some("20280229".to_string())
    );
}

#[test]
fn the_sent_date_is_never_the_qso_date() {
    let db = seeded(BATCH);
    let plan = plan_for(&db, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let count = db.load_batch(plan.plan_id(), 0).unwrap().record_count;
    drive(
        &db,
        plan.plan_id(),
        0,
        &BatchOutcome::Accepted,
        count,
        ACCEPTED_AT,
    );
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, ACCEPTED_AT + 60)
        .expect("commit");
    for qso in snapshots(&db) {
        assert_eq!(qso.qso_date, "20250101");
        assert_ne!(qso.lotw_qslsdate.as_deref(), Some(qso.qso_date.as_str()));
    }
}

#[test]
fn accepted_at_is_always_a_valid_adif_date_because_the_clock_is_unsigned() {
    // Znacznik czasu jest `u64`, więc `accepted_at` nie może wypaść przed 1970,
    // a dolna granica typu ADIF `Date` (1930) jest nieosiągalna z produkcyjnego
    // zegara. Przypadek „status Y bez daty” dotyczy wyłącznie wierszy
    // historycznych i jest pokryty testem `legacy_accepted_rows_…`.
    for timestamp in [0u64, ACCEPTED_AT, 4_102_444_800] {
        let date = adif_date_from_unix(timestamp as i64).expect("date");
        assert_eq!(date.len(), 8);
        assert!(date.chars().all(|c| c.is_ascii_digit()));
    }
}
