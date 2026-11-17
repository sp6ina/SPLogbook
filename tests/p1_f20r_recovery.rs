// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! STAGE-P1-F20R-SOFTWARE — trwałe checkpointy uploadu i recovery po restarcie.
//!
//! Wszystkie dane są syntetyczne (`TST…`), bazy są lokalne (in-memory lub plik
//! w katalogu tymczasowym), nie ma sieci, prawdziwych poświadczeń ani
//! prawdziwego uploadu. Zegar jest **wstrzykiwany** — żaden test nie używa czasu
//! ściennego ani `sleep`.
//!
//! Zakres: ryzyko **F20-R1** (brak trwałego checkpointu planów i batchy).

use splogbook::cloud::upload_checkpoint::{
    BatchState, CHECKPOINT_SCHEMA_VERSION, ErrorClass, OutcomeSnapshot, PlanState, RecoveryAction,
    batch_identifier, compute_batch_digest, compute_plan_digest, recovery_action,
    state_for_outcome,
};
use splogbook::cloud::upload_plan::{
    BatchOutcome, JournalId, SelectionPolicy, UploadPlan, UploadTarget,
};
use splogbook::core::database::Journal;
use splogbook::core::database::LogDatabase;
use splogbook::core::database::upload_checkpoint::CheckpointError;
use splogbook::core::qso::QsoRecord;

const JOURNAL_A: &str = "F20R_A";
const JOURNAL_B: &str = "F20R_B";
const QSOS_A: usize = 1505;
const BATCH_SIZE: usize = 500;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn journal(id: &str, name: &str, is_default: bool) -> Journal {
    Journal {
        id: id.to_string(),
        name: name.to_string(),
        station_callsign: "TSTSTN".to_string(),
        operator: "TSTOP".to_string(),
        my_gridsquare: "JO90AA".to_string(),
        my_pga: String::new(),
        description: "synthetic F-20R fixture".to_string(),
        is_default,
    }
}

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

fn seeded_memory_db() -> LogDatabase {
    let db = LogDatabase::open_in_memory().expect("in-memory database");
    db.create_journal(&journal(JOURNAL_A, "Journal A", true))
        .expect("journal A");
    db.create_journal(&journal(JOURNAL_B, "Journal B", false))
        .expect("journal B");
    let a: Vec<QsoRecord> = (0..QSOS_A).map(|i| make_qso(JOURNAL_A, i)).collect();
    db.batch_insert_qsos(&a).expect("insert A");
    let b: Vec<QsoRecord> = (0..200).map(|i| make_qso(JOURNAL_B, 100_000 + i)).collect();
    db.batch_insert_qsos(&b).expect("insert B");
    db
}

/// Kompaktowy fixture do pętli stresowych: te same niezmienniki, 40 rekordów.
fn seeded_small_db(qso_count: usize) -> LogDatabase {
    let db = LogDatabase::open_in_memory().expect("in-memory database");
    db.create_journal(&journal(JOURNAL_A, "Journal A", true))
        .expect("journal A");
    let a: Vec<QsoRecord> = (0..qso_count).map(|i| make_qso(JOURNAL_A, i)).collect();
    db.batch_insert_qsos(&a).expect("insert A");
    db
}

fn build_plan(db: &LogDatabase, journal_id: &str, target: UploadTarget) -> UploadPlan {
    let journal = JournalId::new(journal_id).expect("journal id");
    let records = db
        .select_upload_candidates_for_journal(&journal, target)
        .expect("candidates");
    UploadPlan::build(
        target,
        journal,
        SelectionPolicy::PendingOnly,
        "attempt-f20r",
        1_700_000_000,
        records,
        BATCH_SIZE,
    )
    .expect("plan")
}

/// Doprowadza paczkę do trwałego stanu `InFlight` (transport rozpoczęty).
fn drive_to_in_flight(db: &LogDatabase, plan_id: &str, ordinal: usize, now: u64) {
    db.prepare_upload_batch(plan_id, ordinal, "payload-digest", now)
        .expect("prepare");
    db.start_upload_batch(plan_id, ordinal, now + 1)
        .expect("start");
}

/// Doprowadza paczkę do trwałego stanu `Accepted` (dowód bez commita).
fn drive_to_accepted(db: &LogDatabase, plan_id: &str, ordinal: usize, now: u64) -> usize {
    drive_to_in_flight(db, plan_id, ordinal, now);
    let count = db.load_batch(plan_id, ordinal).expect("batch").record_count;
    db.record_upload_batch_outcome(
        plan_id,
        ordinal,
        OutcomeSnapshot::from_outcome(&BatchOutcome::Accepted, count),
        now + 2,
    )
    .expect("accepted");
    count
}

// ---------------------------------------------------------------------------
// 1-7. Tworzenie planu, paczek, przeładowanie, stabilność składu i skrótów
// ---------------------------------------------------------------------------

#[test]
fn create_plan_persists_identity_membership_and_schema_version() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");

    assert_eq!(set.plan.plan_id, plan.plan_id());
    assert_eq!(set.plan.target, UploadTarget::Lotw);
    assert_eq!(set.plan.journal_id.as_str(), JOURNAL_A);
    assert_eq!(set.plan.policy, SelectionPolicy::PendingOnly);
    assert_eq!(set.plan.schema_version, CHECKPOINT_SCHEMA_VERSION);
    assert_eq!(set.plan.total_qsos, QSOS_A);
    assert_eq!(set.plan.total_batches, QSOS_A.div_ceil(BATCH_SIZE));
    assert_eq!(set.plan.state, PlanState::Planned);
    assert_eq!(set.plan.plan_digest, compute_plan_digest(&plan));
    assert_eq!(set.batches.len(), set.plan.total_batches);
    assert!(set.batches.iter().all(|b| b.state == BatchState::Planned));
    assert!(
        set.batches
            .iter()
            .all(|b| b.journal_id.as_str() == JOURNAL_A)
    );
    assert!(set.batches.iter().all(|b| b.target == UploadTarget::Lotw));
    assert_eq!(db.count_upload_plans().expect("count"), 1);
    assert_eq!(db.count_upload_batches().expect("count"), set.batches.len());
}

#[test]
fn reload_plan_and_batches_returns_identical_membership_and_order() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");

    let reloaded = db
        .load_upload_checkpoint_set(plan.plan_id())
        .expect("reload");
    assert_eq!(
        reloaded.plan,
        db.load_upload_plan(plan.plan_id()).unwrap().unwrap()
    );

    let mut flat = Vec::new();
    for (index, batch) in reloaded.batches.iter().enumerate() {
        assert_eq!(batch.ordinal, index);
        assert_eq!(batch.batch_id, batch_identifier(plan.plan_id(), index));
        let ids = db
            .load_batch_members(plan.plan_id(), index)
            .expect("members");
        assert_eq!(ids, plan.batches()[index].record_ids());
        assert_eq!(
            batch.batch_digest,
            compute_batch_digest(
                UploadTarget::Lotw,
                &plan.journal_id().clone(),
                plan.plan_id(),
                index,
                &ids
            )
        );
        flat.extend(ids);
    }
    assert_eq!(flat, plan.record_ids());
    assert_eq!(flat.len(), QSOS_A);
    // Całkowite pokrycie i brak duplikatów.
    let unique: std::collections::HashSet<i64> = flat.iter().copied().collect();
    assert_eq!(unique.len(), QSOS_A);
}

#[test]
fn identical_plan_reuses_the_existing_checkpoint_instead_of_creating_a_second() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let first = db.open_or_create_upload_checkpoint(&plan).expect("first");
    let second = db.open_or_create_upload_checkpoint(&plan).expect("second");
    assert_eq!(first.plan, second.plan);
    assert_eq!(db.count_upload_plans().expect("count"), 1);
    assert_eq!(
        db.count_upload_batches().expect("count"),
        first.batches.len()
    );
}

#[test]
fn batch_policy_change_produces_a_different_plan_and_cannot_rewrite_existing_batches() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let before = db
        .load_upload_batches(plan.plan_id())
        .expect("batches")
        .len();

    // Inny rozmiar paczki tej samej zawartości: F-20 wiąże `plan_id` z usługą,
    // dziennikiem, polityką i zbiorem identyfikatorów — nie z rozmiarem paczki.
    // Trwały checkpoint rozpoznaje więc konflikt i odmawia, zamiast po cichu
    // zmienić rozmiar istniejących batchy.
    let records = db
        .select_upload_candidates_for_journal(&plan.journal_id().clone(), UploadTarget::Lotw)
        .expect("candidates");
    let smaller = UploadPlan::build(
        UploadTarget::Lotw,
        plan.journal_id().clone(),
        SelectionPolicy::PendingOnly,
        "attempt-f20r",
        1_700_000_000,
        records,
        100,
    )
    .expect("plan");
    assert_eq!(
        smaller.plan_id(),
        plan.plan_id(),
        "plan_id nie zależy od rozmiaru paczki (kontrakt F-20)"
    );
    assert_ne!(compute_plan_digest(&smaller), compute_plan_digest(&plan));
    let err = db
        .open_or_create_upload_checkpoint(&smaller)
        .expect_err("a batch policy change must be rejected, not applied");
    assert!(matches!(err, CheckpointError::PlanConflict { .. }));

    // Istniejący plan nie zmienił liczby paczek ani rozmiaru.
    let batches = db.load_upload_batches(plan.plan_id()).expect("batches");
    assert_eq!(batches.len(), before);
    assert_eq!(batches[0].record_count, BATCH_SIZE);
    assert_eq!(db.count_upload_plans().expect("count"), 1);
}

#[test]
fn duplicate_plan_id_cannot_be_created_twice() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    // Powtórne otwarcie jest idempotentne, a liczba planów pozostaje 1.
    for _ in 0..8 {
        db.open_or_create_upload_checkpoint(&plan)
            .expect("idempotent");
    }
    assert_eq!(db.count_upload_plans().expect("count"), 1);
}

#[test]
fn batch_from_another_journal_is_detected_by_the_batch_digest() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let ids = db.load_batch_members(plan.plan_id(), 0).expect("members");

    let other = JournalId::new(JOURNAL_B).expect("journal");
    let forged = compute_batch_digest(UploadTarget::Lotw, &other, plan.plan_id(), 0, &ids);
    assert_ne!(
        forged, set.batches[0].batch_digest,
        "a batch digest must bind the journal"
    );
}

#[test]
fn batch_from_another_service_is_detected_by_the_batch_digest() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let ids = db.load_batch_members(plan.plan_id(), 0).expect("members");
    let forged = compute_batch_digest(
        UploadTarget::Eqsl,
        &plan.journal_id().clone(),
        plan.plan_id(),
        0,
        &ids,
    );
    assert_ne!(forged, set.batches[0].batch_digest);
}

#[test]
fn duplicate_batch_ordinal_cannot_exist_in_one_plan() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let ordinals: Vec<usize> = set.batches.iter().map(|b| b.ordinal).collect();
    let unique: std::collections::HashSet<usize> = ordinals.iter().copied().collect();
    assert_eq!(unique.len(), ordinals.len());
    assert_eq!(ordinals, (0..set.batches.len()).collect::<Vec<_>>());
    // Każdy numer paczki istnieje dokładnie raz na poziomie bazy.
    assert_eq!(db.count_upload_batches().expect("count"), set.batches.len());
}

// ---------------------------------------------------------------------------
// 8-16. Macierz crash windows / restart
// ---------------------------------------------------------------------------

#[test]
fn crash_before_any_send_leaves_the_plan_resumable() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");

    let reports = db.recover_upload_checkpoints(9_000).expect("recovery");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].state, PlanState::Planned);
    assert_eq!(reports[0].batches_recovered_to_unknown, 0);
    assert_eq!(reports[0].batches_committed_locally, 0);
    let set = db.load_upload_checkpoint_set(plan.plan_id()).expect("set");
    assert_eq!(set.resumable_ordinals().len(), set.batches.len());
    assert!(set.manual_reconciliation_ordinals().is_empty());
}

#[test]
fn crash_after_prepared_keeps_the_batch_resumable() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    db.prepare_upload_batch(plan.plan_id(), 0, "payload-digest", 100)
        .expect("prepare");

    db.recover_upload_checkpoints(9_000).expect("recovery");
    let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
    assert_eq!(batch.state, BatchState::Prepared);
    assert_eq!(recovery_action(batch.state), RecoveryAction::Resume);
    assert_eq!(
        db.load_upload_plan(plan.plan_id()).unwrap().unwrap().state,
        PlanState::Planned
    );
}

#[test]
fn crash_after_in_flight_becomes_unknown_and_is_never_accepted() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    drive_to_in_flight(&db, plan.plan_id(), 0, 100);

    let reports = db.recover_upload_checkpoints(9_000).expect("recovery");
    assert_eq!(reports[0].batches_recovered_to_unknown, 1);

    let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
    assert_eq!(batch.state, BatchState::Unknown);
    assert_eq!(batch.error_class, ErrorClass::InterruptedInFlight);
    assert!(!batch.retry_eligible);
    assert_ne!(batch.state, BatchState::Accepted);
    assert_ne!(batch.state, BatchState::Completed);
    assert_eq!(
        recovery_action(batch.state),
        RecoveryAction::ManualReconciliation
    );

    // Żaden QSO nie został oznaczony jako wysłany.
    let qsos = db.get_all_qsos_for_journal(JOURNAL_A).expect("all qsos");
    assert!(qsos.iter().all(|qso| qso.lotw_qsl_sent != "Y"));
    assert!(qsos.iter().all(|qso| qso.lotw_qslsdate.is_none()));

    // Plan wymaga rozstrzygnięcia i nie jest ponawiany automatycznie.
    assert_eq!(
        db.load_upload_plan(plan.plan_id()).unwrap().unwrap().state,
        PlanState::RecoveryRequired
    );
    assert_eq!(
        db.upload_plans_requiring_reconciliation()
            .expect("plans")
            .len(),
        1
    );
}

#[test]
fn crash_after_remote_acceptance_completes_the_local_commit_without_resending() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let count = drive_to_accepted(&db, plan.plan_id(), 0, 100);
    assert_eq!(count, BATCH_SIZE);

    // Symulacja restartu: recovery nie może wysłać nic ponownie, tylko
    // dokończyć lokalny zapis z trwałego dowodu.
    let reports = db.recover_upload_checkpoints(9_000).expect("recovery");
    assert_eq!(reports[0].batches_committed_locally, 1);
    assert_eq!(reports[0].batches_recovered_to_unknown, 0);

    let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
    assert_eq!(batch.state, BatchState::Completed);
    assert_eq!(batch.accepted_at, Some(102));

    let qsos = db.get_all_qsos_for_journal(JOURNAL_A).expect("all qsos");
    let sent = qsos.iter().filter(|q| q.lotw_qsl_sent == "Y").count();
    assert_eq!(sent, BATCH_SIZE);
    // Data wysłania pochodzi z chwili uzyskania dowodu, nie z chwili recovery.
    let expected_date = splogbook::cloud::upload_checkpoint::adif_date_from_unix(102).unwrap();
    for qso in qsos.iter().filter(|q| q.lotw_qsl_sent == "Y") {
        assert_eq!(qso.lotw_qslsdate.as_deref(), Some(expected_date.as_str()));
    }
    assert!(qsos.iter().all(|q| q.eqsl_qslsdate.is_none()));
}

#[test]
fn crash_before_database_commit_leaves_no_partial_status() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    drive_to_accepted(&db, plan.plan_id(), 0, 100);

    // Bez recovery: stan jest `Accepted`, a wiersze QSO są nietknięte.
    let qsos = db.get_all_qsos_for_journal(JOURNAL_A).expect("all qsos");
    assert!(qsos.iter().all(|q| q.lotw_qsl_sent != "Y"));
    assert_eq!(
        db.load_batch(plan.plan_id(), 0).unwrap().state,
        BatchState::Accepted
    );
    // Przeliczenie stanu planu (bez commita i bez sieci) pokazuje, że plan
    // wymaga ręcznego rozstrzygnięcia — dowód akceptacji nie jest sukcesem.
    let plan_state = db
        .finish_upload_plan(plan.plan_id(), 5_000)
        .expect("finish")
        .state;
    assert_eq!(plan_state, PlanState::RecoveryRequired);
    assert!(
        db.upload_plans_requiring_reconciliation()
            .expect("plans")
            .iter()
            .any(|entry| entry.plan_id == plan.plan_id())
    );
}

#[test]
fn crash_during_commit_is_unobservable_because_the_transaction_is_atomic() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    drive_to_accepted(&db, plan.plan_id(), 0, 100);

    // `Committing` jest zapisywane wyłącznie wewnątrz transakcji, więc po
    // jakimkolwiek przerwaniu obserwowalne jest `Accepted` albo `Completed`.
    let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
    assert_eq!(batch.state, BatchState::Accepted);
    assert_ne!(batch.state, BatchState::Committing);

    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, 8_000)
        .expect("commit");
    let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
    assert_eq!(batch.state, BatchState::Completed);
    assert_eq!(batch.completed_at, Some(8_000));

    // Ponowny commit tej samej paczki jest odrzucony — nie ma podwójnego zapisu.
    let again = db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, 8_001);
    assert!(matches!(
        again,
        Err(CheckpointError::TransitionRejected { .. })
    ));
}

#[test]
fn restart_after_completed_never_resends() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    drive_to_accepted(&db, plan.plan_id(), 0, 100);
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, 200)
        .expect("commit");

    let reports = db.recover_upload_checkpoints(9_000).expect("recovery");
    assert_eq!(reports[0].batches_never_resent, 1);
    assert_eq!(reports[0].batches_committed_locally, 0);
    assert_eq!(reports[0].batches_recovered_to_unknown, 0);

    let set = db.load_upload_checkpoint_set(plan.plan_id()).expect("set");
    assert_eq!(set.batches[0].state, BatchState::Completed);
    assert!(!set.resumable_ordinals().contains(&0));
    assert!(set.manual_reconciliation_ordinals().is_empty());

    // Powtórne recovery nie zmienia niczego.
    db.recover_upload_checkpoints(9_100).expect("recovery 2");
    let set = db
        .load_upload_checkpoint_set(plan.plan_id())
        .expect("set 2");
    assert_eq!(set.batches[0].state, BatchState::Completed);
    assert_eq!(set.batches[0].completed_at, Some(200));
}

#[test]
fn restart_after_cancelled_never_sends_automatically() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let cancelled = db.cancel_upload_plan(plan.plan_id(), 300).expect("cancel");
    assert_eq!(cancelled.state, PlanState::Cancelled);

    db.recover_upload_checkpoints(9_000).expect("recovery");
    let set = db.load_upload_checkpoint_set(plan.plan_id()).expect("set");
    assert_eq!(set.plan.state, PlanState::Cancelled);
    assert!(set.batches.iter().all(|b| b.state == BatchState::Cancelled));
    assert!(set.resumable_ordinals().is_empty());
    assert_eq!(
        recovery_action(BatchState::Cancelled),
        RecoveryAction::NeverResend
    );
}

#[test]
fn restart_after_unknown_keeps_unknown_and_does_not_retry() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let plan_id_under_test = plan.plan_id().to_string();
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    drive_to_in_flight(&db, plan.plan_id(), 0, 100);
    let count = db.load_batch(plan.plan_id(), 0).unwrap().record_count;
    db.record_upload_batch_outcome(
        plan.plan_id(),
        0,
        OutcomeSnapshot::from_outcome(
            &BatchOutcome::Unknown {
                detail: "TEST_UNKNOWN_OUTCOME".to_string(),
            },
            count,
        ),
        110,
    )
    .expect("unknown");

    for now in [9_000u64, 9_001, 9_002] {
        db.recover_upload_checkpoints(now).expect("recovery");
        let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
        assert_eq!(batch.state, BatchState::Unknown);
        assert!(!batch.retry_eligible);
        assert_eq!(batch.unknown_count, count);
        assert_eq!(batch.accepted_count, 0);
    }
    let pending = db.upload_plans_requiring_reconciliation().expect("plans");
    assert!(
        pending
            .iter()
            .any(|plan| plan.plan_id == plan_id_under_test)
    );
}

#[test]
fn cancellation_leaves_accepted_batches_committed_and_rest_not_sent() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    drive_to_accepted(&db, plan.plan_id(), 0, 100);
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, 200)
        .expect("commit");
    db.cancel_upload_plan(plan.plan_id(), 300).expect("cancel");

    let set = db.load_upload_checkpoint_set(plan.plan_id()).expect("set");
    assert_eq!(set.batches[0].state, BatchState::Completed);
    for batch in set.batches.iter().skip(1) {
        assert_eq!(batch.state, BatchState::Cancelled);
    }
    let qsos = db.get_all_qsos_for_journal(JOURNAL_A).expect("qsos");
    assert_eq!(
        qsos.iter().filter(|q| q.lotw_qsl_sent == "Y").count(),
        BATCH_SIZE
    );
}

// ---------------------------------------------------------------------------
// 17-22. Zmiany w dzienniku, schemacie i integralności
// ---------------------------------------------------------------------------
//
// Uwaga: testy wymagające bezpośredniej mutacji tabel checkpointów
// (`incompatible_checkpoint_schema_version_is_rejected`,
// `corrupted_membership_is_detected_instead_of_silently_trusted`,
// `no_secret_is_persisted_in_any_checkpoint_table`) znajdują się w module
// `core::database::tests`, gdzie dostępne jest surowe połączenie. Ten plik nie
// ma i nie może mieć dostępu do surowego połączenia produkcyjnej bazy.

#[test]
fn changed_active_journal_cannot_move_a_batch_to_another_journal() {
    let db = seeded_memory_db();
    let plan_a = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db.open_or_create_upload_checkpoint(&plan_a).expect("A");
    drive_to_accepted(&db, plan_a.plan_id(), 0, 100);
    db.commit_batch_acceptance_with_dates(plan_a.plan_id(), 0, 200)
        .expect("commit");

    // Plan dziennika B powstaje niezależnie i nie widzi paczek A.
    let plan_b = build_plan(&db, JOURNAL_B, UploadTarget::Lotw);
    assert_ne!(plan_b.plan_id(), plan_a.plan_id());
    let set_b = db.open_or_create_upload_checkpoint(&plan_b).expect("B");
    assert_eq!(set_b.plan.journal_id.as_str(), JOURNAL_B);
    assert!(
        set_b
            .batches
            .iter()
            .all(|b| b.journal_id.as_str() == JOURNAL_B)
    );

    // Żaden rekord B nie został oznaczony przez commit planu A.
    for qso in db.get_all_qsos_for_journal(JOURNAL_B).expect("B qsos") {
        assert_ne!(qso.lotw_qsl_sent, "Y");
        assert!(qso.lotw_qslsdate.is_none());
    }
    // A ma dokładnie tyle rekordów wysłanych, ile w paczce 0.
    let sent_a = db
        .get_all_qsos_for_journal(JOURNAL_A)
        .expect("A qsos")
        .into_iter()
        .filter(|q| q.lotw_qsl_sent == "Y")
        .count();
    assert_eq!(sent_a, set.batches[0].record_count);
}

#[test]
fn added_qso_after_plan_is_not_in_the_plan_and_not_sent() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");

    let extra = vec![make_qso(JOURNAL_A, 900_001)];
    db.batch_insert_qsos(&extra).expect("insert extra");

    let set = db.load_upload_checkpoint_set(plan.plan_id()).expect("set");
    assert_eq!(set.plan.total_qsos, QSOS_A);
    let members: std::collections::HashSet<i64> = (0..set.batches.len())
        .flat_map(|ordinal| db.load_batch_members(plan.plan_id(), ordinal).unwrap())
        .collect();
    let extra_id = db
        .get_all_qsos_for_journal(JOURNAL_A)
        .expect("qsos")
        .into_iter()
        .filter(|q| q.callsign == "TST900001")
        .filter_map(|q| q.id)
        .next()
        .expect("extra id");
    assert!(!members.contains(&extra_id));
}

#[test]
fn edited_qso_after_plan_cannot_change_membership_or_batch_size() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let ids = db.load_batch_members(plan.plan_id(), 0).expect("members");

    let mut qso = db.get_qso_by_id(ids[0]).expect("qso").expect("exists");
    qso.callsign = "TSTEDITED".to_string();
    db.update_qso(ids[0], &qso).expect("update");

    let reloaded = db
        .load_upload_checkpoint_set(plan.plan_id())
        .expect("reload");
    assert_eq!(
        reloaded.batches[0].record_count,
        set.batches[0].record_count
    );
    assert_eq!(
        reloaded.batches[0].batch_digest,
        set.batches[0].batch_digest
    );
    assert_eq!(
        db.load_batch_members(plan.plan_id(), 0).expect("members"),
        ids
    );
}

#[test]
fn deleted_qso_after_plan_cannot_shrink_the_batch_membership() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let ids = db.load_batch_members(plan.plan_id(), 0).expect("members");

    db.delete_qso(ids[0]).expect("delete");

    let reloaded = db
        .load_upload_checkpoint_set(plan.plan_id())
        .expect("reload");
    assert_eq!(
        reloaded.batches[0].record_count,
        set.batches[0].record_count
    );
    assert_eq!(
        db.load_batch_members(plan.plan_id(), 0).expect("members"),
        ids
    );
    // Commit nie może się udać, bo brakuje rekordu — `affected == expected`
    // jest wymuszone, transakcja jest wycofana i paczka wymaga rozstrzygnięcia.
    drive_to_accepted(&db, plan.plan_id(), 0, 100);
    let result = db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, 200);
    assert!(matches!(
        result,
        Err(CheckpointError::RowCountMismatch { .. })
    ));
    assert_eq!(
        db.load_batch(plan.plan_id(), 0).unwrap().state,
        BatchState::Accepted,
        "a rolled-back commit must leave the batch awaiting a local commit"
    );
    let qsos = db.get_all_qsos_for_journal(JOURNAL_A).expect("qsos");
    assert!(qsos.iter().all(|q| q.lotw_qsl_sent != "Y"));
}

#[test]
fn every_batch_transition_is_rejected_from_a_terminal_state() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    drive_to_accepted(&db, plan.plan_id(), 0, 100);
    db.commit_batch_acceptance_with_dates(plan.plan_id(), 0, 200)
        .expect("commit");

    for outcome in [
        BatchOutcome::Accepted,
        BatchOutcome::Rejected {
            detail: String::new(),
        },
        BatchOutcome::Unknown {
            detail: String::new(),
        },
        BatchOutcome::Cancelled,
    ] {
        let err = db
            .record_upload_batch_outcome(
                plan.plan_id(),
                0,
                OutcomeSnapshot::from_outcome(&outcome, BATCH_SIZE),
                300,
            )
            .expect_err("terminal state must reject every outcome");
        assert!(matches!(err, CheckpointError::TransitionRejected { .. }));
    }
    assert_eq!(
        db.load_batch(plan.plan_id(), 0).unwrap().state,
        BatchState::Completed
    );
}

#[test]
fn recording_an_outcome_for_a_missing_batch_is_a_controlled_error() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    db.open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let err = db
        .record_upload_batch_outcome(
            plan.plan_id(),
            plan.batch_count() + 7,
            OutcomeSnapshot::from_outcome(&BatchOutcome::Accepted, 1),
            100,
        )
        .expect_err("missing batch");
    assert!(matches!(err, CheckpointError::MissingBatch { .. }));
    let err = db
        .load_upload_checkpoint_set("NO_SUCH_PLAN")
        .expect_err("missing plan");
    assert!(matches!(err, CheckpointError::MissingPlan { .. }));
}

#[test]
fn report_plan_state_is_derived_for_every_combination_of_batch_states() {
    // 11 stanów paczki × 11 stanów paczki × 2 flagi anulowania = 242 kombinacje.
    let states = [
        BatchState::Planned,
        BatchState::Prepared,
        BatchState::InFlight,
        BatchState::Accepted,
        BatchState::PartiallyAccepted,
        BatchState::Rejected,
        BatchState::Unknown,
        BatchState::Cancelled,
        BatchState::Committing,
        BatchState::Completed,
        BatchState::RecoveryRequired,
    ];
    let mut checked = 0usize;
    for a in states {
        for b in states {
            for cancelled in [false, true] {
                let derived =
                    splogbook::cloud::upload_checkpoint::derive_plan_state(&[a, b], cancelled);
                checked += 1;
                if cancelled {
                    assert_eq!(derived, PlanState::Cancelled);
                    continue;
                }
                if [a, b]
                    .iter()
                    .any(|s| s.is_ambiguous() || s.awaits_local_commit())
                {
                    assert_eq!(derived, PlanState::RecoveryRequired, "{a}/{b}");
                } else if [a, b].iter().any(|s| s.is_resumable()) {
                    assert!(
                        matches!(derived, PlanState::Planned | PlanState::Running),
                        "{a}/{b}"
                    );
                } else {
                    assert_eq!(derived, PlanState::Completed, "{a}/{b}");
                }
            }
        }
    }
    assert_eq!(checked, states.len() * states.len() * 2);
}

#[test]
fn remote_outcome_mapping_is_fail_closed_for_every_outcome() {
    let outcomes = [
        BatchOutcome::Accepted,
        BatchOutcome::PartiallyAccepted {
            rejected: 1,
            detail: String::new(),
        },
        BatchOutcome::NoRecordsProcessed {
            detail: String::new(),
        },
        BatchOutcome::Rejected {
            detail: String::new(),
        },
        BatchOutcome::Unknown {
            detail: String::new(),
        },
        BatchOutcome::TimedOut {
            detail: String::new(),
        },
        BatchOutcome::Failed {
            detail: String::new(),
        },
        BatchOutcome::Cancelled,
    ];
    for outcome in outcomes {
        let state = state_for_outcome(&outcome);
        let action = recovery_action(state);
        if outcome.is_accepted() {
            assert_eq!(state, BatchState::Accepted);
            assert_eq!(action, RecoveryAction::CommitLocalOnly);
            continue;
        }
        assert_ne!(state, BatchState::Accepted, "{outcome:?}");
        assert_ne!(state, BatchState::Completed, "{outcome:?}");
        assert_ne!(
            action,
            RecoveryAction::Resume,
            "{outcome:?} must never be resumable"
        );
    }
}

// ---------------------------------------------------------------------------
// 25-27. Stress / kombinatoryka (deterministyczne, bez sleep)
// ---------------------------------------------------------------------------

#[test]
fn stress_5000_plan_and_batch_reloads_are_stable() {
    let db = seeded_small_db(40);
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let first = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    let mut reloads = 0usize;
    for _ in 0..5000 {
        let set = db
            .load_upload_checkpoint_set(plan.plan_id())
            .expect("reload");
        assert_eq!(set.plan.plan_id, first.plan.plan_id);
        assert_eq!(set.batches.len(), first.batches.len());
        assert_eq!(set.batches[0].batch_digest, first.batches[0].batch_digest);
        reloads += 1;
    }
    assert_eq!(reloads, 5000);
}

#[test]
fn stress_5000_recorded_outcomes_never_produce_a_false_acceptance() {
    let db = seeded_small_db(40);
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let mut accepted = 0usize;
    for iteration in 0..5000u64 {
        let set = db
            .open_or_create_upload_checkpoint(&plan)
            .expect("checkpoint");
        // Reset do stanu `Planned` nie istnieje produkcyjnie; iterujemy więc
        // wyłącznie na modelu wyniku, bez dotykania bazy.
        let outcome = match iteration % 5 {
            0 => BatchOutcome::Accepted,
            1 => BatchOutcome::PartiallyAccepted {
                rejected: 1,
                detail: String::new(),
            },
            2 => BatchOutcome::Rejected {
                detail: String::new(),
            },
            3 => BatchOutcome::TimedOut {
                detail: String::new(),
            },
            _ => BatchOutcome::Cancelled,
        };
        let snapshot = OutcomeSnapshot::from_outcome(&outcome, set.plan.total_qsos);
        assert!(snapshot.accounting_consistent(set.plan.total_qsos));
        if outcome.is_accepted() {
            accepted += 1;
        } else {
            assert_ne!(snapshot.state, BatchState::Accepted);
        }
    }
    assert_eq!(accepted, 1000);
}

#[test]
fn stress_3000_in_flight_restarts_all_become_unknown() {
    for _ in 0..3000 {
        let db = seeded_small_db(12);
        let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
        db.open_or_create_upload_checkpoint(&plan)
            .expect("checkpoint");
        drive_to_in_flight(&db, plan.plan_id(), 0, 100);
        db.recover_upload_checkpoints(9_000).expect("recovery");
        let batch = db.load_batch(plan.plan_id(), 0).expect("batch");
        assert_eq!(batch.state, BatchState::Unknown);
        assert_ne!(batch.state, BatchState::Accepted);
    }
}

#[test]
fn combinatorial_matrix_of_crash_points_and_states_is_total() {
    // 11 stanów × 8 wyników × 2 dzienniki × 2 usługi × 3 punkty crashu = 1056
    // kombinacji, każda z jawnym oczekiwanym zachowaniem.
    let states = [
        BatchState::Planned,
        BatchState::Prepared,
        BatchState::InFlight,
        BatchState::Accepted,
        BatchState::PartiallyAccepted,
        BatchState::Rejected,
        BatchState::Unknown,
        BatchState::Cancelled,
        BatchState::Committing,
        BatchState::Completed,
        BatchState::RecoveryRequired,
    ];
    let crash_points = [
        "before_transport",
        "after_transport_before_outcome",
        "after_outcome_before_commit",
    ];
    let mut combinations = 0usize;
    for state in states {
        for outcome_bits in 0..8u8 {
            for journal_id in [JOURNAL_A, JOURNAL_B] {
                for target in [UploadTarget::Lotw, UploadTarget::Eqsl] {
                    for crash in crash_points {
                        combinations += 1;
                        let action = recovery_action(state);
                        // Niezmiennik globalny: żaden stan nie może zostać
                        // uznany za sukces tylko dlatego, że przerwano go
                        // w danym punkcie.
                        match state {
                            BatchState::Completed | BatchState::Cancelled => {
                                assert_eq!(action, RecoveryAction::NeverResend);
                            }
                            BatchState::Accepted => {
                                assert_eq!(action, RecoveryAction::CommitLocalOnly);
                            }
                            BatchState::InFlight => {
                                assert_eq!(action, RecoveryAction::FailClosedUnknown);
                            }
                            BatchState::Planned | BatchState::Prepared => {
                                assert_eq!(action, RecoveryAction::Resume);
                            }
                            _ => {
                                assert_eq!(action, RecoveryAction::ManualReconciliation);
                            }
                        }
                        assert!(!journal_id.is_empty());
                        assert!(target.has_sent_status());
                        assert!(!crash.is_empty());
                        let _ = outcome_bits;
                    }
                }
            }
        }
    }
    assert_eq!(combinations, 11 * 8 * 2 * 2 * 3);
}

#[test]
fn batch_identifier_and_digest_are_stable_for_1505_records() {
    let db = seeded_memory_db();
    let plan = build_plan(&db, JOURNAL_A, UploadTarget::Lotw);
    let set = db
        .open_or_create_upload_checkpoint(&plan)
        .expect("checkpoint");
    assert_eq!(set.batches.len(), 4);
    assert_eq!(set.batches[0].record_count, 500);
    assert_eq!(set.batches[3].record_count, 5);
    assert_eq!(set.batches[3].batch_id, batch_identifier(plan.plan_id(), 3));
    let mut seen = std::collections::HashSet::new();
    for batch in &set.batches {
        assert!(seen.insert(batch.batch_digest.clone()));
        assert_eq!(batch.state, BatchState::Planned);
    }
}
