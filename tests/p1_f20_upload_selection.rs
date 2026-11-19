// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! P1-F20 — journal-scoped, complete and lossless upload selection.
//!
//! Wszystkie dane są syntetyczne (`TST…`), baza jest w pamięci, nie ma sieci,
//! prawdziwych poświadczeń ani prawdziwego uploadu. Testy pilnują invariántów
//! F-20: jawny dziennik, brak globalnego wyboru, brak cichego obcinania,
//! kompletność i rozłączność paczek, deterministyczna kolejność oraz zapis
//! statusu wyłącznie po dowodzie akceptacji.

use splogbook::cloud::upload_plan::{
    BatchOutcome, JournalId, PlanError, PlanRecord, RecordUploadStatus, RunStatus, SelectionPolicy,
    UploadPlan, UploadTarget, run_plan,
};
use splogbook::core::database::{Journal, LogDatabase};
use splogbook::core::qso::QsoRecord;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

const JOURNAL_A: &str = "F20_A";
const JOURNAL_B: &str = "F20_B";
const JOURNAL_C: &str = "F20_C";

const QSOS_A: usize = 1505;
const QSOS_B: usize = 1200;
const QSOS_C: usize = 25;

fn journal(id: &str, name: &str, is_default: bool) -> Journal {
    Journal {
        id: id.to_string(),
        name: name.to_string(),
        station_callsign: "TSTSTN".to_string(),
        operator: "TSTOP".to_string(),
        my_gridsquare: "JO90AA".to_string(),
        my_pga: String::new(),
        description: "synthetic F-20 fixture".to_string(),
        is_default,
    }
}

/// Syntetyczny rekord QSO. Część rekordów celowo ma identyczne znaczniki czasu.
fn make_qso(journal_id: &str, index: usize) -> QsoRecord {
    let mut qso = QsoRecord::new(format!("TST{index:05}"), "20m", "CW");
    qso.journal_id = Some(journal_id.to_string());
    let day = (index % 28) + 1;
    let hour = index % 24;
    let minute = (index / 24) % 60;
    qso.qso_date = format!("202601{day:02}");
    qso.time_on = format!("{hour:02}{minute:02}00");
    qso
}

fn seeded_db() -> LogDatabase {
    let db = LogDatabase::open_in_memory().expect("in-memory database");
    db.create_journal(&journal(JOURNAL_A, "Journal A", true))
        .expect("journal A");
    db.create_journal(&journal(JOURNAL_B, "Journal B", false))
        .expect("journal B");
    db.create_journal(&journal(JOURNAL_C, "Journal C", false))
        .expect("journal C");

    let a: Vec<QsoRecord> = (0..QSOS_A).map(|i| make_qso(JOURNAL_A, i)).collect();
    db.batch_insert_qsos(&a).expect("insert A");

    let b: Vec<QsoRecord> = (0..QSOS_B)
        .map(|i| make_qso(JOURNAL_B, 100_000 + i))
        .collect();
    db.batch_insert_qsos(&b).expect("insert B");

    // Journal C: 25 rekordów, które NIE kwalifikują się do żadnej z usług.
    let c: Vec<QsoRecord> = (0..QSOS_C)
        .map(|i| {
            let mut qso = make_qso(JOURNAL_C, 200_000 + i);
            qso.lotw_qsl_sent = "Y".to_string();
            qso.eqsl_qsl_sent = if i % 2 == 0 { "Y" } else { "I" }.to_string();
            qso
        })
        .collect();
    db.batch_insert_qsos(&c).expect("insert C");

    db
}

fn ids_of(db: &LogDatabase, journal_id: &str) -> HashSet<i64> {
    db.get_all_qsos_for_journal(journal_id)
        .expect("all qsos")
        .into_iter()
        .filter_map(|qso| qso.id)
        .collect()
}

fn candidates(db: &LogDatabase, journal_id: &str, target: UploadTarget) -> Vec<PlanRecord> {
    let journal = JournalId::new(journal_id).expect("journal id");
    db.select_upload_candidates_for_journal(&journal, target)
        .expect("candidates")
}

fn build_plan(
    journal_id: &str,
    target: UploadTarget,
    records: Vec<PlanRecord>,
    batch_size: usize,
) -> UploadPlan {
    UploadPlan::build(
        target,
        JournalId::new(journal_id).expect("journal id"),
        SelectionPolicy::PendingOnly,
        "attempt-1",
        1_700_000_000,
        records,
        batch_size,
    )
    .expect("plan")
}

// ---------------------------------------------------------------------------
// Reprodukcja F-20: dziennik A 1505, dziennik B 1200, dziennik C 25 niekwalif.
// ---------------------------------------------------------------------------

#[test]
fn journal_a_plan_contains_exactly_its_own_1505_eligible_qsos() {
    let db = seeded_db();
    let records = candidates(&db, JOURNAL_A, UploadTarget::Lotw);
    assert_eq!(
        records.len(),
        QSOS_A,
        "wszystkie kwalifikujące się QSO dziennika A muszą być wybrane"
    );

    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        records,
        UploadTarget::Lotw.batch_size(),
    );
    assert_eq!(plan.total_records(), QSOS_A);
    assert_eq!(plan.batch_count(), 4, "1505 rekordów po 500 to 4 paczki");

    let batches = plan.batches();
    assert_eq!(batches.iter().map(|b| b.count()).sum::<usize>(), QSOS_A);
    assert_eq!(batches[0].count(), 500);
    assert_eq!(
        batches[3].count(),
        5,
        "ostatnia paczka nie może zostać obcięta"
    );
}

#[test]
fn journal_b_plan_contains_exactly_its_own_1200_eligible_qsos() {
    let db = seeded_db();
    let records = candidates(&db, JOURNAL_B, UploadTarget::Eqsl);
    assert_eq!(records.len(), QSOS_B);
    let plan = build_plan(
        JOURNAL_B,
        UploadTarget::Eqsl,
        records,
        UploadTarget::Eqsl.batch_size(),
    );
    assert_eq!(plan.total_records(), QSOS_B);
    assert_eq!(
        plan.batch_count(),
        QSOS_B.div_ceil(UploadTarget::Eqsl.batch_size())
    );
}

#[test]
fn journal_c_has_no_eligible_qsos_for_either_service() {
    let db = seeded_db();
    assert!(candidates(&db, JOURNAL_C, UploadTarget::Lotw).is_empty());
    assert!(candidates(&db, JOURNAL_C, UploadTarget::Eqsl).is_empty());
}

#[test]
fn no_foreign_journal_record_is_ever_selected() {
    let db = seeded_db();
    let ids_a = ids_of(&db, JOURNAL_A);
    let ids_b = ids_of(&db, JOURNAL_B);
    let ids_c = ids_of(&db, JOURNAL_C);
    assert_eq!(ids_a.len(), QSOS_A);
    assert_eq!(ids_b.len(), QSOS_B);
    assert_eq!(ids_c.len(), QSOS_C);
    // Rozłączne zakresy identyfikatorów.
    assert!(ids_a.is_disjoint(&ids_b));
    assert!(ids_a.is_disjoint(&ids_c));
    assert!(ids_b.is_disjoint(&ids_c));

    for target in [UploadTarget::Lotw, UploadTarget::Eqsl] {
        for (journal_id, own) in [
            (JOURNAL_A, &ids_a),
            (JOURNAL_B, &ids_b),
            (JOURNAL_C, &ids_c),
        ] {
            let selected: HashSet<i64> = candidates(&db, journal_id, target)
                .into_iter()
                .map(|record| record.id)
                .collect();
            assert!(
                selected.is_subset(own),
                "wybór dla {journal_id}/{target:?} zawiera rekord obcego dziennika"
            );
        }
    }
}

#[test]
fn selection_covers_the_complete_eligible_set_without_truncation() {
    let db = seeded_db();
    for target in [UploadTarget::Lotw, UploadTarget::Eqsl] {
        for (journal_id, expected) in [(JOURNAL_A, QSOS_A), (JOURNAL_B, QSOS_B)] {
            let records = candidates(&db, journal_id, target);
            assert_eq!(
                records.len(),
                expected,
                "brak cichego obcinania dla {journal_id}/{target:?}"
            );
            let plan = build_plan(journal_id, target, records, target.batch_size());
            let union: HashSet<i64> = plan
                .batches()
                .into_iter()
                .flat_map(|b| b.record_ids().to_vec())
                .collect();
            assert_eq!(union.len(), expected, "suma paczek == pełny zbiór");
        }
    }
}

// ---------------------------------------------------------------------------
// Kwalifikowalność (ADIF `QSL Sent`: Y = zaakceptowane, I = ignoruj).
// ---------------------------------------------------------------------------

#[test]
fn already_accepted_and_ignored_records_are_not_selected() {
    let db = LogDatabase::open_in_memory().expect("db");
    db.create_journal(&journal(JOURNAL_A, "A", true))
        .expect("journal");

    let mut qsos = Vec::new();
    for (index, status) in ["N", "Y", "R", "Q", "I", "N"].iter().enumerate() {
        let mut qso = make_qso(JOURNAL_A, index);
        qso.lotw_qsl_sent = (*status).to_string();
        qso.eqsl_qsl_sent = (*status).to_string();
        qsos.push(qso);
    }
    db.batch_insert_qsos(&qsos).expect("insert");

    // Y i I wykluczone; N, R, Q kwalifikują się.
    let lotw = candidates(&db, JOURNAL_A, UploadTarget::Lotw);
    assert_eq!(lotw.len(), 4, "N + R + Q + N");
    let eqsl = candidates(&db, JOURNAL_A, UploadTarget::Eqsl);
    assert_eq!(eqsl.len(), 4);
}

#[test]
fn lotw_status_never_changes_eqsl_selection_and_vice_versa() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let lotw_batch = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    let first = &lotw_batch.batches()[0];
    db.commit_batch_acceptance(&a, UploadTarget::Lotw, first.record_ids())
        .expect("commit LoTW batch");

    assert_eq!(
        candidates(&db, JOURNAL_A, UploadTarget::Lotw).len(),
        QSOS_A - first.count()
    );
    assert_eq!(
        candidates(&db, JOURNAL_A, UploadTarget::Eqsl).len(),
        QSOS_A,
        "status LoTW nie może zmienić wyboru eQSL"
    );

    // I odwrotnie.
    db.commit_batch_acceptance(&a, UploadTarget::Eqsl, &first.record_ids()[..10])
        .expect("commit eQSL batch");
    assert_eq!(
        candidates(&db, JOURNAL_A, UploadTarget::Eqsl).len(),
        QSOS_A - 10
    );
    assert_eq!(
        candidates(&db, JOURNAL_A, UploadTarget::Lotw).len(),
        QSOS_A - first.count(),
        "status eQSL nie może zmienić wyboru LoTW"
    );
}

#[test]
fn committing_a_batch_never_touches_another_journal() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    for batch in plan.batches() {
        db.commit_batch_acceptance(&a, UploadTarget::Lotw, batch.record_ids())
            .expect("commit");
    }

    assert!(candidates(&db, JOURNAL_A, UploadTarget::Lotw).is_empty());
    for qso in db.get_all_qsos_for_journal(JOURNAL_B).expect("B") {
        assert_eq!(
            qso.lotw_qsl_sent, "N",
            "rekord dziennika B nie może zostać oznaczony"
        );
    }
    for qso in db.get_all_qsos_for_journal(JOURNAL_C).expect("C") {
        assert_ne!(qso.lotw_qsl_sent, "N");
    }
    assert_eq!(candidates(&db, JOURNAL_B, UploadTarget::Lotw).len(), QSOS_B);
}

#[test]
fn committing_foreign_record_ids_affects_nothing() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let foreign: Vec<i64> = db
        .get_all_qsos_for_journal(JOURNAL_B)
        .expect("B")
        .into_iter()
        .filter_map(|qso| qso.id)
        .take(5)
        .collect();
    assert_eq!(foreign.len(), 5);
    // Rekordy B podane z dziennikiem A: predykat journal_id nie pasuje.
    let outcome = db.commit_batch_acceptance(&a, UploadTarget::Lotw, &foreign);
    assert!(outcome.is_err(), "obcy dziennik nie może zostać zapisany");
    for qso in db.get_all_qsos_for_journal(JOURNAL_B).expect("B") {
        assert_eq!(qso.lotw_qsl_sent, "N");
    }
}

// ---------------------------------------------------------------------------
// Kolejność i determinizm.
// ---------------------------------------------------------------------------

#[test]
fn selection_order_is_canonical_ascending_and_stable() {
    let db = seeded_db();
    let records = candidates(&db, JOURNAL_A, UploadTarget::Lotw);
    let plan = build_plan(JOURNAL_A, UploadTarget::Lotw, records.clone(), 500);
    let ordered = plan.record_ids();

    let mut keys: Vec<(String, String, i64)> = plan
        .records()
        .iter()
        .map(|record| {
            (
                record.qso_date.replace('-', ""),
                format!("{:0<6}", record.time_on.replace(':', "")),
                record.id,
            )
        })
        .collect();
    let sorted = {
        let mut clone = keys.clone();
        clone.sort();
        clone
    };
    assert_eq!(keys, sorted, "plan musi być w kolejności kanonicznej");
    keys.truncate(1);

    // Ta sama odpowiedź przy odwróconej kolejności wejścia.
    let mut reversed = records;
    reversed.reverse();
    let plan_reversed = build_plan(JOURNAL_A, UploadTarget::Lotw, reversed, 500);
    assert_eq!(plan_reversed.record_ids(), ordered);
    assert_eq!(plan_reversed.plan_id(), plan.plan_id());
}

#[test]
fn identical_timestamps_are_totally_ordered_by_id() {
    let db = LogDatabase::open_in_memory().expect("db");
    db.create_journal(&journal(JOURNAL_A, "A", true))
        .expect("journal");
    let qsos: Vec<QsoRecord> = (0..10)
        .map(|index| {
            let mut qso = make_qso(JOURNAL_A, index);
            qso.qso_date = "20260101".to_string();
            qso.time_on = "120000".to_string();
            qso
        })
        .collect();
    db.batch_insert_qsos(&qsos).expect("insert");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        3,
    );
    let ids = plan.record_ids();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "identyczne znaczniki czasu → kolejność po id");
}

#[test]
fn plan_id_is_deterministic_and_order_independent() {
    let records: Vec<PlanRecord> = (1..=50)
        .map(|index| PlanRecord::new(index, "20260101", format!("{index:06}")))
        .collect();
    let plan_a = build_plan(JOURNAL_A, UploadTarget::Lotw, records.clone(), 7);
    let mut shuffled = records;
    shuffled.rotate_left(13);
    shuffled.reverse();
    let plan_b = build_plan(JOURNAL_A, UploadTarget::Lotw, shuffled, 7);
    assert_eq!(plan_a.plan_id(), plan_b.plan_id());
    assert_eq!(plan_a.record_ids(), plan_b.record_ids());
}

// ---------------------------------------------------------------------------
// Odmowy i błędy kontrolowane.
// ---------------------------------------------------------------------------

#[test]
fn missing_journal_is_a_controlled_refusal() {
    assert_eq!(JournalId::new(""), Err(PlanError::MissingJournal));
    assert_eq!(JournalId::new("   "), Err(PlanError::MissingJournal));
    assert_eq!(JournalId::new(" LEADING"), Err(PlanError::InvalidJournalId));
}

#[test]
fn empty_journal_yields_an_empty_lossless_plan() {
    let db = LogDatabase::open_in_memory().expect("db");
    db.create_journal(&journal("F20_EMPTY", "Empty", true))
        .expect("journal");
    let records = candidates(&db, "F20_EMPTY", UploadTarget::Lotw);
    assert!(records.is_empty());
    let plan = build_plan("F20_EMPTY", UploadTarget::Lotw, records, 500);
    assert_eq!(plan.total_records(), 0);
    assert_eq!(plan.batch_count(), 0);
    assert!(plan.batches().is_empty());
}

#[test]
fn zero_batch_size_is_rejected() {
    let error = UploadPlan::build(
        UploadTarget::Lotw,
        JournalId::new(JOURNAL_A).expect("journal"),
        SelectionPolicy::PendingOnly,
        "attempt",
        0,
        vec![PlanRecord::new(1, "20260101", "000000")],
        0,
    );
    assert_eq!(error, Err(PlanError::ZeroBatchSize));
}

#[test]
fn duplicate_record_ids_are_rejected_not_deduplicated() {
    let records = vec![
        PlanRecord::new(7, "20260101", "000000"),
        PlanRecord::new(7, "20260102", "000000"),
    ];
    let error = UploadPlan::build(
        UploadTarget::Eqsl,
        JournalId::new(JOURNAL_B).expect("journal"),
        SelectionPolicy::PendingOnly,
        "attempt",
        0,
        records,
        10,
    );
    assert_eq!(error, Err(PlanError::DuplicateRecord(7)));
}

// ---------------------------------------------------------------------------
// Batchowanie: kompletność, rozłączność, limity.
// ---------------------------------------------------------------------------

#[test]
fn batching_matrix_is_complete_disjoint_and_ordered() {
    for count in [0usize, 1, 2, 999, 1000, 1001, 1505, 10_000] {
        let records: Vec<PlanRecord> = (1..=count as i64)
            .map(|index| PlanRecord::new(index, "20260101", format!("{index:06}")))
            .collect();
        for size in [
            1usize,
            2,
            250,
            500,
            999,
            1000,
            1001,
            1505,
            10_000,
            usize::MAX,
        ] {
            let plan = build_plan(JOURNAL_A, UploadTarget::Lotw, records.clone(), size);
            assert_eq!(plan.total_records(), count);

            let batches = plan.batches();
            if count == 0 {
                assert!(batches.is_empty());
                assert_eq!(plan.batch_count(), 0);
                continue;
            }
            assert!(!batches.is_empty());
            assert_eq!(batches.len(), plan.batch_count());

            let mut seen = HashSet::new();
            let mut flat = Vec::new();
            for batch in &batches {
                assert!(!batch.record_ids().is_empty(), "pusta paczka");
                for id in batch.record_ids() {
                    assert!(seen.insert(*id), "duplikat rekordu w paczkach");
                    flat.push(*id);
                }
            }
            assert_eq!(seen.len(), count, "suma paczek == liczba rekordów");
            assert_eq!(
                flat,
                plan.record_ids(),
                "kolejność paczek == kolejność planu"
            );

            if size == usize::MAX {
                assert_eq!(batches.len(), 1);
            }
            if count % size == 0 && size <= count {
                assert!(batches.iter().all(|b| b.count() == size));
            }
        }
    }
}

#[test]
fn ten_thousand_record_plan_is_lossless() {
    let records: Vec<PlanRecord> = (1..=10_000i64)
        .map(|index| PlanRecord::new(index, "20260101", format!("{index:06}")))
        .collect();
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Eqsl,
        records,
        UploadTarget::Eqsl.batch_size(),
    );
    assert_eq!(plan.total_records(), 10_000);
    assert_eq!(
        plan.batch_count(),
        10_000usize.div_ceil(UploadTarget::Eqsl.batch_size())
    );
    let total: usize = plan.batches().iter().map(|batch| batch.count()).sum();
    assert_eq!(total, 10_000);
}

// ---------------------------------------------------------------------------
// Częściowy sukces, timeout, anulowanie.
// ---------------------------------------------------------------------------

fn run_plan_with(
    plan: &UploadPlan,
    outcomes: Vec<BatchOutcome>,
) -> splogbook::cloud::upload_plan::UploadRunReport {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let cancel = AtomicBool::new(false);
    let mut queue = std::collections::VecDeque::from(outcomes);
    runtime.block_on(run_plan(plan, &cancel, move |_batch| {
        let next = queue.pop_front().unwrap_or(BatchOutcome::Cancelled);
        async move { next }
    }))
}

#[test]
fn partial_success_is_reported_as_partial_and_marks_only_accepted_batches() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    let batch_count = plan.batch_count();
    let mut outcomes = vec![BatchOutcome::Accepted; batch_count];
    outcomes[1] = BatchOutcome::PartiallyAccepted {
        rejected: 1,
        detail: "część pominięta".to_string(),
    };

    let report = run_plan_with(&plan, outcomes);
    assert_eq!(report.status, RunStatus::Partial);
    assert!(!report.status.is_success());
    assert!(report.accounting_consistent());
    assert_eq!(report.accepted_records, 1505 - 500);
    assert_eq!(report.unknown_records, 500);

    let marked = db
        .apply_accepted_batches(&a, UploadTarget::Lotw, &report)
        .expect("apply");
    assert_eq!(
        marked, report.accepted_records,
        "tylko potwierdzone rekordy"
    );
    assert_eq!(
        candidates(&db, JOURNAL_A, UploadTarget::Lotw).len(),
        500,
        "rekordy częściowej paczki pozostają kwalifikowalne"
    );
}

#[test]
fn timeout_and_cancellation_never_mark_records_as_sent() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Eqsl,
        candidates(&db, JOURNAL_A, UploadTarget::Eqsl),
        500,
    );

    let report = run_plan_with(
        &plan,
        vec![
            BatchOutcome::TimedOut {
                detail: "timeout".to_string(),
            },
            BatchOutcome::Cancelled,
            BatchOutcome::Cancelled,
            BatchOutcome::Cancelled,
        ],
    );
    assert!(!report.status.is_success());
    assert_eq!(report.status, RunStatus::Cancelled);
    assert_eq!(report.accepted_records, 0);
    let marked = db
        .apply_accepted_batches(&a, UploadTarget::Eqsl, &report)
        .expect("apply");
    assert_eq!(marked, 0);
    assert_eq!(candidates(&db, JOURNAL_A, UploadTarget::Eqsl).len(), QSOS_A);
}

#[test]
fn failed_batches_are_isolated_from_accepted_ones() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    let report = run_plan_with(
        &plan,
        vec![
            BatchOutcome::Failed {
                detail: "błąd transportu".to_string(),
            },
            BatchOutcome::Accepted,
            BatchOutcome::Rejected {
                detail: "odrzucone".to_string(),
            },
            BatchOutcome::Accepted,
        ],
    );
    assert_eq!(report.status, RunStatus::Partial);
    assert_eq!(report.failed_records, 500);
    assert_eq!(report.rejected_records, 500);
    assert_eq!(report.accepted_records, 505);
    let marked = db
        .apply_accepted_batches(&a, UploadTarget::Lotw, &report)
        .expect("apply");
    assert_eq!(marked, 505);
}

#[test]
fn cancellation_before_first_batch_sends_nothing() {
    let db = seeded_db();
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let cancel = AtomicBool::new(true);
    let mut calls = 0usize;
    let report = runtime.block_on(run_plan(&plan, &cancel, |_batch| {
        calls += 1;
        async move { BatchOutcome::Accepted }
    }));
    assert_eq!(calls, 0);
    assert_eq!(report.status, RunStatus::Cancelled);
    assert_eq!(report.not_attempted_records, QSOS_A);
}

#[test]
fn cancellation_after_first_batch_keeps_only_first_batch_accepted() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let report = runtime.block_on(run_plan(&plan, &cancel, move |batch| {
        let flag = flag.clone();
        async move {
            if batch.index() == 0 {
                flag.store(true, Ordering::SeqCst);
                BatchOutcome::Accepted
            } else {
                BatchOutcome::Accepted
            }
        }
    }));
    assert_eq!(report.status, RunStatus::Cancelled);
    assert_eq!(report.accepted_records, 500);
    assert_eq!(report.not_attempted_records, QSOS_A - 500);
    let marked = db
        .apply_accepted_batches(&a, UploadTarget::Lotw, &report)
        .expect("apply");
    assert_eq!(marked, 500);
}

// ---------------------------------------------------------------------------
// Fault injection: zmiany zbioru po utworzeniu planu.
// ---------------------------------------------------------------------------

#[test]
fn deleting_a_qso_after_plan_creation_prevents_partial_commit() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    let mut all_batches = plan.batches();
    let first = all_batches.remove(0);

    // Usunięcie jednego rekordu z paczki po materializacji planu.
    let victim = first.record_ids()[0];
    assert!(db.delete_qso_scoped(victim, JOURNAL_A).expect("delete"));

    let error = db
        .commit_batch_acceptance(&a, UploadTarget::Lotw, first.record_ids())
        .expect_err("commit nie może się udać dla niepełnego zbioru");
    assert!(
        matches!(
            error,
            splogbook::core::database::upload_selection::BatchCommitError::RowCountMismatch { .. }
        ),
        "oczekiwano RowCountMismatch, jest {error:?}"
    );

    // Transakcja wycofana: pozostałe rekordy paczki nadal nieoznaczone.
    let still_pending = candidates(&db, JOURNAL_A, UploadTarget::Lotw).len();
    assert_eq!(still_pending, QSOS_A - 1);
}

#[test]
fn payload_materialization_returns_only_the_planned_journal_records() {
    let db = seeded_db();
    let a = JournalId::new(JOURNAL_A).expect("journal");
    let plan = build_plan(
        JOURNAL_A,
        UploadTarget::Lotw,
        candidates(&db, JOURNAL_A, UploadTarget::Lotw),
        500,
    );
    let mut all_batches = plan.batches();
    let first = all_batches.remove(0);
    let qsos = db
        .get_qsos_by_ids_for_journal(&a, first.record_ids())
        .expect("payload");
    assert_eq!(qsos.len(), first.count());
    let ids_a = ids_of(&db, JOURNAL_A);
    for qso in &qsos {
        assert!(ids_a.contains(&qso.id.expect("id")));
        assert_eq!(qso.journal_id.as_deref(), Some(JOURNAL_A));
    }

    // Nieistniejący identyfikator nie jest po cichu pomijany w liczeniu.
    let mut with_extra = first.record_ids().to_vec();
    with_extra.push(i64::MAX);
    let qsos = db
        .get_qsos_by_ids_for_journal(&a, &with_extra)
        .expect("payload");
    assert_eq!(qsos.len(), first.count());
    assert_ne!(qsos.len(), with_extra.len());
}

// ---------------------------------------------------------------------------
// Macierz kombinatoryczna — co najmniej 100 000 przypadków.
// ---------------------------------------------------------------------------

/// Buduje syntetyczny zbiór rekordów planu. Rekordy o statusie `Y`/`I` nie
/// kwalifikują się, więc nie trafiają do planu — dokładnie tak, jak robi to
/// predykat SQL.
fn synthetic_records(count: usize, status: &str) -> Vec<PlanRecord> {
    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let id = index as i64 + 1;
        let day = (index % 28) + 1;
        let hour = index % 24;
        let minute = (index / 24) % 60;
        let sent = match status {
            "sent" => "Y",
            "mixed" => {
                if index % 3 == 0 {
                    "Y"
                } else if index % 7 == 0 {
                    "I"
                } else {
                    "N"
                }
            }
            _ => "N",
        };
        if sent == "N" {
            records.push(PlanRecord::new(
                id,
                format!("202601{day:02}"),
                format!("{hour:02}{minute:02}00"),
            ));
        }
    }
    records
}

#[test]
fn combinatorial_matrix_holds_selection_and_outcome_invariants() {
    let services = [UploadTarget::Lotw, UploadTarget::Eqsl];
    let journal_counts = [1usize, 2, 3];
    let record_counts = [0usize, 1, 5, 7, 13, 31];
    let statuses = ["pending", "mixed", "sent"];
    let batch_sizes = [1usize, 2, 5, 7, 64, usize::MAX];
    let policies = [SelectionPolicy::PendingOnly, SelectionPolicy::ExplicitRetry];
    let outcomes = [
        BatchOutcome::Accepted,
        BatchOutcome::PartiallyAccepted {
            rejected: 1,
            detail: "x".to_string(),
        },
        BatchOutcome::NoRecordsProcessed {
            detail: "x".to_string(),
        },
        BatchOutcome::Rejected {
            detail: "x".to_string(),
        },
        BatchOutcome::Unknown {
            detail: "x".to_string(),
        },
        BatchOutcome::TimedOut {
            detail: "x".to_string(),
        },
        BatchOutcome::Failed {
            detail: "x".to_string(),
        },
        BatchOutcome::Cancelled,
    ];
    let response_kinds = ["result", "warning", "error", "noresult"];
    let cancel_points = [0u8, 1, 2];

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let mut cases = 0usize;
    for &service in &services {
        for &journals in &journal_counts {
            let journal_id = match journals {
                1 => JOURNAL_A,
                2 => JOURNAL_B,
                _ => JOURNAL_C,
            };
            for &count in &record_counts {
                for &status in &statuses {
                    for &batch_size in &batch_sizes {
                        for &policy in &policies {
                            for outcome in &outcomes {
                                for &response in &response_kinds {
                                    for &cancel_at in &cancel_points {
                                        cases += 1;
                                        let records = synthetic_records(count, status);
                                        let expected = records.len();

                                        let plan = UploadPlan::build(
                                            service,
                                            JournalId::new(journal_id).expect("journal"),
                                            policy,
                                            "attempt",
                                            0,
                                            records,
                                            batch_size,
                                        )
                                        .expect("plan");

                                        assert_eq!(plan.total_records(), expected);
                                        assert_eq!(plan.target(), service);
                                        assert_eq!(plan.journal_id().as_str(), journal_id);
                                        assert_eq!(plan.policy(), policy);

                                        // Paczki: kompletne, rozłączne, puste tylko dla pustego planu.
                                        let batches = plan.batches();
                                        let mut seen = HashSet::new();
                                        for batch in &batches {
                                            assert!(!batch.record_ids().is_empty());
                                            for id in batch.record_ids() {
                                                assert!(seen.insert(*id));
                                            }
                                        }
                                        assert_eq!(seen.len(), plan.total_records());
                                        assert_eq!(
                                            batches.iter().map(|b| b.count()).sum::<usize>(),
                                            plan.total_records()
                                        );

                                        // Klasyfikacja odpowiedzi nigdy nie daje fałszywego sukcesu.
                                        let classification = match response {
                                            "result" => splogbook::cloud::eqsl::EqslClassification::RecordsResult {
                                                added: plan.total_records(),
                                                total: plan.total_records(),
                                            },
                                            "warning" => splogbook::cloud::eqsl::EqslClassification::WarningsOnly {
                                                rejected: 1,
                                                detail: "w".to_string(),
                                            },
                                            "error" => splogbook::cloud::eqsl::EqslClassification::FatalNothingImported {
                                                detail: "e".to_string(),
                                            },
                                            _ => splogbook::cloud::eqsl::EqslClassification::NoResult {
                                                detail: "n".to_string(),
                                            },
                                        };
                                        let classified = splogbook::cloud::eqsl::eqsl_batch_outcome(
                                            &classification,
                                            plan.total_records(),
                                        );
                                        if response != "result" {
                                            assert!(!classified.is_accepted());
                                        }
                                        if response == "result" && plan.total_records() == 0 {
                                            assert!(!classified.is_accepted());
                                        }

                                        // Runner: anulowanie i porażki nigdy nie dają Complete.
                                        if plan.total_records() == 0 {
                                            continue;
                                        }
                                        let cancel =
                                            std::sync::Arc::new(AtomicBool::new(cancel_at == 1));
                                        let cancel_inner = cancel.clone();
                                        let planned_outcome = outcome.clone();
                                        let report = runtime.block_on(run_plan(
                                            &plan,
                                            &cancel,
                                            move |batch| {
                                                let flag = cancel_inner.clone();
                                                let planned = planned_outcome.clone();
                                                async move {
                                                    if batch.index() == 0 && cancel_at == 2 {
                                                        flag.store(true, Ordering::SeqCst);
                                                    }
                                                    planned
                                                }
                                            },
                                        ));

                                        assert!(report.accounting_consistent());
                                        assert_eq!(report.total_records, plan.total_records());
                                        assert_eq!(report.batch_count, plan.batch_count());
                                        if cancel_at != 0
                                            || !matches!(outcome, BatchOutcome::Accepted)
                                        {
                                            assert_ne!(report.status, RunStatus::Complete);
                                        }
                                        if matches!(outcome, BatchOutcome::Accepted)
                                            && cancel_at == 0
                                        {
                                            assert_eq!(report.status, RunStatus::Complete);
                                            assert_eq!(
                                                report.accepted_records,
                                                plan.total_records()
                                            );
                                        }
                                        if report.cancelled {
                                            assert!(!report.status.is_success());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(
        cases >= 100_000,
        "macierz kombinatoryczna musi mieć >= 100 000 przypadków, jest {cases}"
    );
}

#[test]
fn stress_cross_journal_selection_has_zero_contamination() {
    let db = LogDatabase::open_in_memory().expect("db");
    db.create_journal(&journal("F20_S1", "S1", true))
        .expect("S1");
    db.create_journal(&journal("F20_S2", "S2", false))
        .expect("S2");
    let s1: Vec<QsoRecord> = (0..60).map(|i| make_qso("F20_S1", i)).collect();
    let s2: Vec<QsoRecord> = (0..60).map(|i| make_qso("F20_S2", 1000 + i)).collect();
    db.batch_insert_qsos(&s1).expect("insert S1");
    db.batch_insert_qsos(&s2).expect("insert S2");
    let ids_s1 = ids_of(&db, "F20_S1");
    let ids_s2 = ids_of(&db, "F20_S2");

    // 5000 iteracji na małym dzienniku: żaden rekord obcego dziennika.
    for _ in 0..5000 {
        let selected = candidates(&db, "F20_S1", UploadTarget::Lotw);
        assert_eq!(selected.len(), 60);
        for record in &selected {
            assert!(ids_s1.contains(&record.id));
            assert!(!ids_s2.contains(&record.id));
        }
    }

    // Pełnowymiarowy dziennik A (1505) — powtarzalny wybór bez kontaminacji.
    let full = seeded_db();
    let ids_a = ids_of(&full, JOURNAL_A);
    let ids_b = ids_of(&full, JOURNAL_B);
    for _ in 0..20 {
        let selected = candidates(&full, JOURNAL_A, UploadTarget::Eqsl);
        assert_eq!(selected.len(), QSOS_A);
        assert!(selected.iter().all(|record| ids_a.contains(&record.id)));
        assert!(selected.iter().all(|record| !ids_b.contains(&record.id)));
    }
}

#[test]
fn stress_batching_is_lossless_for_ten_thousand_records() {
    let records: Vec<PlanRecord> = (1..=10_000i64)
        .map(|index| PlanRecord::new(index, "20260101", format!("{index:06}")))
        .collect();
    let plan = build_plan(JOURNAL_A, UploadTarget::Lotw, records.clone(), 500);
    assert_eq!(plan.batch_count(), 20);

    // 2000 iteracji podziału na paczki na tym samym planie.
    for _ in 0..2000 {
        let batches = plan.batches();
        let total: usize = batches.iter().map(|batch| batch.count()).sum();
        assert_eq!(total, 10_000);
        assert_eq!(batches.len(), 20);
        assert!(batches.iter().all(|batch| batch.count() == 500));
    }

    // 50 pełnych przebudów planu: identyczny wynik i identyczny `plan_id`.
    for _ in 0..50 {
        let rebuilt = build_plan(JOURNAL_A, UploadTarget::Lotw, records.clone(), 500);
        assert_eq!(rebuilt.plan_id(), plan.plan_id());
        assert_eq!(rebuilt.record_ids(), plan.record_ids());
    }
}

#[test]
fn record_status_mapping_is_conservative() {
    assert_eq!(
        BatchOutcome::Accepted.record_status(),
        RecordUploadStatus::Accepted
    );
    for outcome in [
        BatchOutcome::PartiallyAccepted {
            rejected: 1,
            detail: String::new(),
        },
        BatchOutcome::NoRecordsProcessed {
            detail: String::new(),
        },
        BatchOutcome::Unknown {
            detail: String::new(),
        },
        BatchOutcome::TimedOut {
            detail: String::new(),
        },
    ] {
        assert_eq!(outcome.record_status(), RecordUploadStatus::Unknown);
    }
    assert_eq!(
        BatchOutcome::Cancelled.record_status(),
        RecordUploadStatus::NotAttempted
    );
    assert_eq!(
        BatchOutcome::Failed {
            detail: String::new()
        }
        .record_status(),
        RecordUploadStatus::Failed
    );
}
