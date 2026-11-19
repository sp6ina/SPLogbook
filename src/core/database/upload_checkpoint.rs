// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Trwała persystencja checkpointów planu uploadu (F-20R, ryzyko **F20-R1**).
//!
//! Model stanów i dozwolone przejścia są zdefiniowane w
//! [`crate::cloud::upload_checkpoint`]; ten moduł wyłącznie je utrwala.
//!
//! Zasady, które ten moduł egzekwuje:
//!
//!  * **jedna transakcja na przejście** — stan paczki i stan planu nie mogą
//!    się rozjechać,
//!  * **`affected == expected`** — każde przejście sprawdza liczbę zmienionych
//!    wierszy; rozbieżność to [`CheckpointError::TransitionRejected`], nigdy
//!    cichy sukces,
//!  * **skład paczki jest niezmienny** — przy każdym commicie skład jest
//!    porównywany ze skrótem zapisanym przy tworzeniu planu,
//!  * **żadnych sekretów** — w tabelach nie ma URL-i, haseł, tokenów ani treści
//!    odpowiedzi; wyłącznie identyfikatory, liczby, znaczniki czasu i skróty,
//!  * **brak transakcji podczas sieci** — każda funkcja tutaj kończy się przed
//!    wywołaniem transportu i zaczyna po jego zakończeniu.

use super::LogDatabase;
use crate::cloud::upload_checkpoint::{
    BATCH_POLICY_VERSION, BatchCheckpoint, BatchState, CANONICAL_ORDER_VERSION,
    CHECKPOINT_SCHEMA_VERSION, ErrorClass, OutcomeSnapshot, PlanCheckpoint, PlanState,
    SELECTION_POLICY_VERSION, batch_identifier, compute_batch_digest, compute_evidence_digest,
    compute_plan_digest, derive_plan_state,
};
use crate::cloud::upload_plan::{JournalId, SelectionPolicy, UploadPlan, UploadTarget};
use rusqlite::{OptionalExtension, Result as SqlResult, params};

/// Błąd operacji na trwałym checkpoincie. Wszystkie warianty są kontrolowane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointError {
    /// Baza danych zwróciła błąd.
    Database(String),
    /// Plan o tym `plan_id` istnieje, ale ma inny skład lub inne metadane.
    PlanConflict { detail: String },
    /// Paczka o tym numerze nie istnieje w planie.
    MissingBatch { plan_id: String, ordinal: usize },
    /// Plan nie istnieje.
    MissingPlan { plan_id: String },
    /// Przejście stanu jest niedozwolone w obecnym stanie.
    TransitionRejected {
        plan_id: String,
        ordinal: usize,
        from: BatchState,
        to: BatchState,
    },
    /// Skład paczki nie odpowiada skrótowi zapisanemu w planie.
    MembershipMismatch {
        plan_id: String,
        ordinal: usize,
        expected: usize,
        actual: usize,
    },
    /// Zapis statusu QSO objął inną liczbę wierszy niż długość paczki.
    RowCountMismatch { expected: usize, affected: usize },
    /// Wersja schematu checkpointu jest niezgodna z tą wersją aplikacji.
    IncompatibleSchemaVersion { found: i64, expected: i64 },
    /// Brak trwałego dowodu akceptacji wymaganego do lokalnego commita.
    MissingEvidence { plan_id: String, ordinal: usize },
    /// Usługa nie ma kolumny statusu wysyłki, więc nie ma czego commitować.
    UnsupportedTarget,
}

impl std::fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(detail) => write!(f, "błąd bazy checkpointów: {detail}"),
            Self::PlanConflict { detail } => write!(f, "konflikt planu: {detail}"),
            Self::MissingBatch { plan_id, ordinal } => {
                write!(f, "brak paczki {ordinal} w planie {plan_id}")
            }
            Self::MissingPlan { plan_id } => write!(f, "brak planu {plan_id}"),
            Self::TransitionRejected {
                plan_id,
                ordinal,
                from,
                to,
            } => write!(
                f,
                "niedozwolone przejście paczki {ordinal} planu {plan_id}: {from} → {to}"
            ),
            Self::MembershipMismatch {
                plan_id,
                ordinal,
                expected,
                actual,
            } => write!(
                f,
                "skład paczki {ordinal} planu {plan_id} zmieniony: oczekiwano {expected}, jest {actual}"
            ),
            Self::RowCountMismatch { expected, affected } => write!(
                f,
                "zapis statusu objął {affected} z {expected} rekordów — transakcja wycofana"
            ),
            Self::IncompatibleSchemaVersion { found, expected } => write!(
                f,
                "niezgodna wersja schematu checkpointu: {found} (oczekiwana {expected})"
            ),
            Self::MissingEvidence { plan_id, ordinal } => write!(
                f,
                "brak trwałego dowodu akceptacji paczki {ordinal} planu {plan_id}"
            ),
            Self::UnsupportedTarget => write!(f, "usługa nie ma trwałego statusu wysyłki"),
        }
    }
}

impl std::error::Error for CheckpointError {}

impl From<rusqlite::Error> for CheckpointError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

/// Wynik recovery jednego planu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRecoveryReport {
    pub plan_id: String,
    pub target: UploadTarget,
    pub journal_id: String,
    pub previous_state: PlanState,
    pub state: PlanState,
    pub batches_recovered_to_unknown: usize,
    pub batches_committed_locally: usize,
    pub batches_requiring_manual_reconciliation: usize,
    pub batches_never_resent: usize,
}

impl PlanRecoveryReport {
    /// Czy plan wymaga decyzji człowieka.
    pub const fn needs_manual_reconciliation(&self) -> bool {
        matches!(self.state, PlanState::RecoveryRequired)
    }
}

/// Kolumna statusu wysyłki i kolumna daty wysłania dla usługi.
///
/// Zwraca wyłącznie stałe identyfikatory kolumn — nigdy tekst z zewnątrz.
fn sent_columns(target: UploadTarget) -> Option<(&'static str, &'static str)> {
    match target {
        UploadTarget::Lotw => Some(("lotw_qsl_sent", "lotw_qslsdate")),
        UploadTarget::Eqsl => Some(("eqsl_qsl_sent", "eqsl_qslsdate")),
        _ => None,
    }
}

fn parse_target(raw: &str) -> Result<UploadTarget, CheckpointError> {
    match raw {
        "lotw" => Ok(UploadTarget::Lotw),
        "eqsl" => Ok(UploadTarget::Eqsl),
        "clublog" => Ok(UploadTarget::ClubLog),
        "qrz" => Ok(UploadTarget::Qrz),
        "hamqth" => Ok(UploadTarget::HamQth),
        "cloudlog" => Ok(UploadTarget::Cloudlog),
        "hrdlog" => Ok(UploadTarget::HrdLog),
        other => Err(CheckpointError::Database(format!(
            "nieznana usługa w checkpoincie: {other}"
        ))),
    }
}

fn parse_policy(raw: &str) -> Result<SelectionPolicy, CheckpointError> {
    match raw {
        "pending_only" => Ok(SelectionPolicy::PendingOnly),
        "explicit_retry" => Ok(SelectionPolicy::ExplicitRetry),
        other => Err(CheckpointError::Database(format!(
            "nieznana polityka w checkpoincie: {other}"
        ))),
    }
}

fn state_parse(raw: &str) -> Result<BatchState, CheckpointError> {
    BatchState::parse(raw).ok_or_else(|| {
        CheckpointError::Database(format!("nieznany stan paczki w checkpoincie: {raw}"))
    })
}

impl LogDatabase {
    /// Tworzy trwały checkpoint planu **albo** otwiera istniejący, jeżeli
    /// skład jest identyczny.
    ///
    /// Idempotencja jest tu celowa: dwukrotne kliknięcie „wyślij” dla tego
    /// samego zbioru rekordów nie może utworzyć dwóch planów ani zmienić
    /// istniejącego. Konflikt składu jest błędem, nie cichym nadpisaniem.
    pub fn open_or_create_upload_checkpoint(
        &self,
        plan: &UploadPlan,
    ) -> Result<BatchCheckpointSet, CheckpointError> {
        let plan_digest = compute_plan_digest(plan);

        if let Some(existing) = self.load_upload_plan(plan.plan_id())? {
            Self::verify_existing_plan(plan, &existing, &plan_digest)?;
            return self.load_upload_checkpoint_set(plan.plan_id());
        }

        let batches = plan.batches();
        let tx = self.conn.unchecked_transaction()?;
        {
            tx.execute(
                "INSERT INTO upload_plans (
                    plan_id, schema_version, service, journal_id, selection_policy,
                    selection_policy_version, canonical_order_version, batch_policy_version,
                    plan_digest, total_qsos, total_batches, created_at, state, updated_at,
                    error_class
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    plan.plan_id(),
                    CHECKPOINT_SCHEMA_VERSION,
                    plan.target().as_str(),
                    plan.journal_id().as_str(),
                    plan.policy().as_str(),
                    SELECTION_POLICY_VERSION,
                    CANONICAL_ORDER_VERSION,
                    BATCH_POLICY_VERSION,
                    plan_digest,
                    plan.total_records() as i64,
                    batches.len() as i64,
                    plan.created_at() as i64,
                    PlanState::Planned.as_str(),
                    plan.created_at() as i64,
                    ErrorClass::None.as_str(),
                ],
            )?;

            let mut insert_batch = tx.prepare(
                "INSERT INTO upload_batches (
                    plan_id, ordinal, batch_id, service, journal_id, record_count,
                    batch_digest, attempt_id, state, updated_at, error_class
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            let mut insert_member = tx.prepare(
                "INSERT INTO upload_batch_members (plan_id, ordinal, position, qso_id)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;

            for batch in &batches {
                let ordinal = batch.index();
                let batch_id = batch_identifier(plan.plan_id(), ordinal);
                let digest = compute_batch_digest(
                    plan.target(),
                    plan.journal_id(),
                    plan.plan_id(),
                    ordinal,
                    batch.record_ids(),
                );
                insert_batch.execute(params![
                    plan.plan_id(),
                    ordinal as i64,
                    batch_id,
                    plan.target().as_str(),
                    plan.journal_id().as_str(),
                    batch.count() as i64,
                    digest,
                    plan.attempt_id(),
                    BatchState::Planned.as_str(),
                    plan.created_at() as i64,
                    ErrorClass::None.as_str(),
                ])?;
                for (position, id) in batch.record_ids().iter().enumerate() {
                    insert_member.execute(params![
                        plan.plan_id(),
                        ordinal as i64,
                        position as i64,
                        id,
                    ])?;
                }
            }
        }
        tx.commit()?;

        self.load_upload_checkpoint_set(plan.plan_id())
    }

    fn verify_existing_plan(
        plan: &UploadPlan,
        existing: &PlanCheckpoint,
        plan_digest: &str,
    ) -> Result<(), CheckpointError> {
        if existing.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::IncompatibleSchemaVersion {
                found: existing.schema_version,
                expected: CHECKPOINT_SCHEMA_VERSION,
            });
        }
        let same = existing.target == plan.target()
            && existing.journal_id.as_str() == plan.journal_id().as_str()
            && existing.policy == plan.policy()
            && existing.plan_digest == plan_digest
            && existing.total_qsos == plan.total_records()
            && existing.total_batches == plan.batch_count();
        if !same {
            return Err(CheckpointError::PlanConflict {
                detail: format!(
                    "plan {} istnieje z innym składem (service={}, journal={}, total_qsos={}, batches={})",
                    plan.plan_id(),
                    existing.target.as_str(),
                    existing.journal_id.as_str(),
                    existing.total_qsos,
                    existing.total_batches
                ),
            });
        }
        Ok(())
    }

    /// Odczytuje plan i wszystkie jego paczki wraz z uporządkowanym
    /// członkostwem, po czym **weryfikuje** skróty i rozmiary.
    pub fn load_upload_checkpoint_set(
        &self,
        plan_id: &str,
    ) -> Result<BatchCheckpointSet, CheckpointError> {
        let plan = self
            .load_upload_plan(plan_id)?
            .ok_or_else(|| CheckpointError::MissingPlan {
                plan_id: plan_id.to_string(),
            })?;
        if plan.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::IncompatibleSchemaVersion {
                found: plan.schema_version,
                expected: CHECKPOINT_SCHEMA_VERSION,
            });
        }
        let mut batches = self.load_upload_batches(plan_id)?;
        let mut total_members = 0usize;
        for batch in &mut batches {
            let ids = self.load_batch_members(plan_id, batch.ordinal)?;
            let digest = compute_batch_digest(
                batch.target,
                &batch.journal_id,
                plan_id,
                batch.ordinal,
                &ids,
            );
            if digest != batch.batch_digest || ids.len() != batch.record_count || ids.is_empty() {
                return Err(CheckpointError::MembershipMismatch {
                    plan_id: plan_id.to_string(),
                    ordinal: batch.ordinal,
                    expected: batch.record_count,
                    actual: ids.len(),
                });
            }
            total_members += ids.len();
        }
        if total_members != plan.total_qsos || batches.len() != plan.total_batches {
            return Err(CheckpointError::MembershipMismatch {
                plan_id: plan_id.to_string(),
                ordinal: 0,
                expected: plan.total_qsos,
                actual: total_members,
            });
        }
        Ok(BatchCheckpointSet { plan, batches })
    }

    /// Odczytuje pojedynczy plan (bez paczek).
    pub fn load_upload_plan(
        &self,
        plan_id: &str,
    ) -> Result<Option<PlanCheckpoint>, CheckpointError> {
        let row = self
            .conn
            .query_row(
                "SELECT plan_id, schema_version, service, journal_id, selection_policy,
                        selection_policy_version, canonical_order_version, batch_policy_version,
                        plan_digest, total_qsos, total_batches, created_at, state, error_class
                 FROM upload_plans WHERE plan_id = ?1",
                params![plan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, i64>(9)?,
                        row.get::<_, i64>(10)?,
                        row.get::<_, i64>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            plan_id,
            schema_version,
            service,
            journal_id,
            policy,
            selection_policy_version,
            canonical_order_version,
            batch_policy_version,
            plan_digest,
            total_qsos,
            total_batches,
            created_at,
            state,
            error_class,
        )) = row
        else {
            return Ok(None);
        };

        Ok(Some(PlanCheckpoint {
            plan_id,
            schema_version,
            target: parse_target(&service)?,
            journal_id: JournalId::new(&journal_id).map_err(|_| {
                CheckpointError::Database(format!(
                    "nieprawidłowy journal_id w checkpoincie: {journal_id}"
                ))
            })?,
            policy: parse_policy(&policy)?,
            selection_policy_version,
            canonical_order_version,
            batch_policy_version,
            plan_digest,
            total_qsos: total_qsos as usize,
            total_batches: total_batches as usize,
            created_at: created_at as u64,
            state: PlanState::parse(&state).ok_or_else(|| {
                CheckpointError::Database(format!("nieznany stan planu: {state}"))
            })?,
            error_class: ErrorClass::parse(&error_class).unwrap_or(ErrorClass::UnknownOutcome),
        }))
    }

    /// Odczytuje wszystkie paczki planu w kolejności numerów.
    pub fn load_upload_batches(
        &self,
        plan_id: &str,
    ) -> Result<Vec<BatchCheckpoint>, CheckpointError> {
        let mut stmt = self.conn.prepare(
            "SELECT plan_id, ordinal, batch_id, service, journal_id, record_count,
                    batch_digest, attempt_id, state, started_at, accepted_at, completed_at,
                    accepted_count, rejected_count, unknown_count, evidence_digest,
                    error_class, retry_eligible
             FROM upload_batches WHERE plan_id = ?1 ORDER BY ordinal ASC",
        )?;
        let rows = stmt.query_map(params![plan_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, i64>(12)?,
                row.get::<_, i64>(13)?,
                row.get::<_, i64>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, String>(16)?,
                row.get::<_, i64>(17)?,
            ))
        })?;

        let mut batches = Vec::new();
        for row in rows {
            let (
                plan_id,
                ordinal,
                batch_id,
                service,
                journal_id,
                record_count,
                batch_digest,
                attempt_id,
                state,
                started_at,
                accepted_at,
                completed_at,
                accepted_count,
                rejected_count,
                unknown_count,
                evidence_digest,
                error_class,
                retry_eligible,
            ) = row?;
            batches.push(BatchCheckpoint {
                plan_id,
                batch_id,
                ordinal: ordinal as usize,
                target: parse_target(&service)?,
                journal_id: JournalId::new(&journal_id).map_err(|_| {
                    CheckpointError::Database(format!(
                        "nieprawidłowy journal_id paczki: {journal_id}"
                    ))
                })?,
                record_count: record_count as usize,
                batch_digest,
                attempt_id,
                state: state_parse(&state)?,
                started_at: started_at.map(|value| value as u64),
                accepted_at: accepted_at.map(|value| value as u64),
                completed_at: completed_at.map(|value| value as u64),
                accepted_count: accepted_count as usize,
                rejected_count: rejected_count as usize,
                unknown_count: unknown_count as usize,
                evidence_digest,
                error_class: ErrorClass::parse(&error_class).unwrap_or(ErrorClass::UnknownOutcome),
                retry_eligible: retry_eligible != 0,
            });
        }
        Ok(batches)
    }

    /// Odczytuje **uporządkowane** identyfikatory QSO jednej paczki.
    pub fn load_batch_members(
        &self,
        plan_id: &str,
        ordinal: usize,
    ) -> Result<Vec<i64>, CheckpointError> {
        let mut stmt = self.conn.prepare(
            "SELECT qso_id FROM upload_batch_members
             WHERE plan_id = ?1 AND ordinal = ?2 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map(params![plan_id, ordinal as i64], |row| row.get::<_, i64>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }

    /// Oznacza paczkę jako przygotowaną (`Planned → Prepared`).
    ///
    /// `payload_digest` jest skrótem zmaterializowanego ładunku; nie zawiera
    /// treści rekordów ani sekretów, służy do wykrycia zmiany ładunku między
    /// przygotowaniem a wysyłką.
    pub fn prepare_upload_batch(
        &self,
        plan_id: &str,
        ordinal: usize,
        payload_digest: &str,
        now: u64,
    ) -> Result<BatchCheckpoint, CheckpointError> {
        self.advance_batch_state(
            plan_id,
            ordinal,
            BatchState::Planned,
            BatchState::Prepared,
            now,
            Some(payload_digest),
        )
    }

    /// Oznacza paczkę jako wysyłaną (`Prepared → InFlight`).
    ///
    /// Od tego momentu przerwanie procesu oznacza `Unknown` po restarcie —
    /// nigdy `Accepted`.
    pub fn start_upload_batch(
        &self,
        plan_id: &str,
        ordinal: usize,
        now: u64,
    ) -> Result<BatchCheckpoint, CheckpointError> {
        let tx = self.conn.unchecked_transaction()?;
        let affected = tx.execute(
            "UPDATE upload_batches
             SET state = ?3, started_at = ?4, updated_at = ?4
             WHERE plan_id = ?1 AND ordinal = ?2 AND state = ?5",
            params![
                plan_id,
                ordinal as i64,
                BatchState::InFlight.as_str(),
                now as i64,
                BatchState::Prepared.as_str(),
            ],
        )?;
        if affected != 1 {
            let current = Self::batch_state_within(&tx, plan_id, ordinal)?;
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: current,
                to: BatchState::InFlight,
            });
        }
        tx.execute(
            "UPDATE upload_plans SET state = ?2, updated_at = ?3
             WHERE plan_id = ?1 AND state = ?4",
            params![
                plan_id,
                PlanState::Running.as_str(),
                now as i64,
                PlanState::Planned.as_str(),
            ],
        )?;
        tx.commit()?;
        self.load_batch(plan_id, ordinal)
    }

    fn advance_batch_state(
        &self,
        plan_id: &str,
        ordinal: usize,
        from: BatchState,
        to: BatchState,
        now: u64,
        evidence_digest: Option<&str>,
    ) -> Result<BatchCheckpoint, CheckpointError> {
        let tx = self.conn.unchecked_transaction()?;
        let affected = tx.execute(
            "UPDATE upload_batches
             SET state = ?4, updated_at = ?5,
                 evidence_digest = COALESCE(?6, evidence_digest)
             WHERE plan_id = ?1 AND ordinal = ?2 AND state = ?3",
            params![
                plan_id,
                ordinal as i64,
                from.as_str(),
                to.as_str(),
                now as i64,
                evidence_digest,
            ],
        )?;
        if affected != 1 {
            let current = Self::batch_state_within(&tx, plan_id, ordinal)?;
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: current,
                to,
            });
        }
        tx.commit()?;
        self.load_batch(plan_id, ordinal)
    }

    fn batch_state_within(
        conn: &rusqlite::Connection,
        plan_id: &str,
        ordinal: usize,
    ) -> Result<BatchState, CheckpointError> {
        let raw: Option<String> = conn
            .query_row(
                "SELECT state FROM upload_batches WHERE plan_id = ?1 AND ordinal = ?2",
                params![plan_id, ordinal as i64],
                |row| row.get(0),
            )
            .optional()?;
        match raw {
            Some(raw) => state_parse(&raw),
            None => Err(CheckpointError::MissingBatch {
                plan_id: plan_id.to_string(),
                ordinal,
            }),
        }
    }

    /// Odczytuje pojedynczą paczkę.
    pub fn load_batch(
        &self,
        plan_id: &str,
        ordinal: usize,
    ) -> Result<BatchCheckpoint, CheckpointError> {
        self.load_upload_batches(plan_id)?
            .into_iter()
            .find(|batch| batch.ordinal == ordinal)
            .ok_or_else(|| CheckpointError::MissingBatch {
                plan_id: plan_id.to_string(),
                ordinal,
            })
    }

    /// Zapisuje wynik próby (`InFlight → Accepted | PartiallyAccepted |
    /// Rejected | Unknown | Cancelled`).
    ///
    /// Stan docelowy wynika **wyłącznie** z dowodu zwróconego przez transport.
    /// `Accepted` zapisuje `accepted_at`, czyli moment uzyskania trwałego
    /// dowodu — to z niego (a nie z chwili commita ani z `QSO_DATE`) powstanie
    /// data wysłania.
    pub fn record_upload_batch_outcome(
        &self,
        plan_id: &str,
        ordinal: usize,
        snapshot: OutcomeSnapshot,
        now: u64,
    ) -> Result<BatchCheckpoint, CheckpointError> {
        let batch = self.load_batch(plan_id, ordinal)?;
        if !batch.state.allows_transition_to(snapshot.state) {
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: batch.state,
                to: snapshot.state,
            });
        }
        let evidence = compute_evidence_digest(
            snapshot.state,
            snapshot.accepted_count,
            snapshot.rejected_count,
            snapshot.unknown_count,
        );
        let accepted_at = matches!(snapshot.state, BatchState::Accepted).then_some(now as i64);

        let tx = self.conn.unchecked_transaction()?;
        let affected = tx.execute(
            "UPDATE upload_batches
             SET state = ?3, accepted_at = ?4, accepted_count = ?5, rejected_count = ?6,
                 unknown_count = ?7, evidence_digest = ?8, error_class = ?9,
                 retry_eligible = 0, updated_at = ?10
             WHERE plan_id = ?1 AND ordinal = ?2 AND state = ?11",
            params![
                plan_id,
                ordinal as i64,
                snapshot.state.as_str(),
                accepted_at,
                snapshot.accepted_count as i64,
                snapshot.rejected_count as i64,
                snapshot.unknown_count as i64,
                evidence,
                snapshot.error_class.as_str(),
                now as i64,
                BatchState::InFlight.as_str(),
            ],
        )?;
        if affected != 1 {
            let current = Self::batch_state_within(&tx, plan_id, ordinal)?;
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: current,
                to: snapshot.state,
            });
        }
        tx.commit()?;
        self.load_batch(plan_id, ordinal)
    }

    /// Zatwierdza lokalny skutek zaakceptowanej paczki: status wysłania,
    /// datę wysłania i stan paczki — **w jednej transakcji**.
    ///
    /// Sekwencja jest zgodna z kontraktem F20-R1:
    /// 1. weryfikacja planu (usługa, dziennik, wersja schematu),
    /// 2. weryfikacja paczki (stan `Accepted`, skrót, rozmiar),
    /// 3. weryfikacja dziennika i pełnego uporządkowanego zbioru `QSO id`,
    /// 4. zapis statusu **i** daty (`affected == expected`),
    /// 5. przejście `Accepted → Committing → Completed` wewnątrz transakcji,
    /// 6. przeliczenie stanu planu,
    /// 7. commit.
    ///
    /// `Committing` jest zapisywane wewnątrz tej samej transakcji, więc po
    /// restarcie obserwowalne jest wyłącznie `Accepted` (rollback) albo
    /// `Completed` (commit). To jest dowód atomowości, a nie jej założenie.
    pub fn commit_batch_acceptance_with_dates(
        &self,
        plan_id: &str,
        ordinal: usize,
        now: u64,
    ) -> Result<BatchCheckpoint, CheckpointError> {
        let plan = self
            .load_upload_plan(plan_id)?
            .ok_or_else(|| CheckpointError::MissingPlan {
                plan_id: plan_id.to_string(),
            })?;
        let batch = self.load_batch(plan_id, ordinal)?;
        if batch.state != BatchState::Accepted {
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: batch.state,
                to: BatchState::Committing,
            });
        }
        if batch.target != plan.target || batch.journal_id != plan.journal_id {
            return Err(CheckpointError::PlanConflict {
                detail: format!(
                    "paczka {ordinal} planu {plan_id} należy do innej usługi lub dziennika"
                ),
            });
        }
        let ids = self.load_batch_members(plan_id, ordinal)?;
        if ids.is_empty() || ids.len() != batch.record_count {
            return Err(CheckpointError::MembershipMismatch {
                plan_id: plan_id.to_string(),
                ordinal,
                expected: batch.record_count,
                actual: ids.len(),
            });
        }
        let digest = compute_batch_digest(batch.target, &batch.journal_id, plan_id, ordinal, &ids);
        if digest != batch.batch_digest {
            return Err(CheckpointError::MembershipMismatch {
                plan_id: plan_id.to_string(),
                ordinal,
                expected: batch.record_count,
                actual: ids.len(),
            });
        }
        let Some(accepted_at) = batch.accepted_at else {
            return Err(CheckpointError::MissingEvidence {
                plan_id: plan_id.to_string(),
                ordinal,
            });
        };

        let tx = self.conn.unchecked_transaction()?;

        // 4. Zapis statusu i daty. `COALESCE` chroni pierwotną, potwierdzoną
        //    datę przed nadpisaniem przez późniejszą próbę.
        let sent_date = crate::cloud::upload_checkpoint::adif_date_from_unix(accepted_at as i64);
        if let Some((sent_column, date_column)) = sent_columns(batch.target) {
            let placeholders = (0..ids.len())
                .map(|index| format!("?{}", index + 3))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "UPDATE qso_records
                 SET {sent_column} = 'Y', {date_column} = COALESCE({date_column}, ?2)
                 WHERE journal_id = ?1 AND id IN ({placeholders})"
            );
            let mut bound: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 3);
            let journal = batch.journal_id.as_str();
            let date = sent_date.as_deref();
            bound.push(&journal);
            bound.push(&date);
            for id in &ids {
                bound.push(id);
            }
            let affected = tx.execute(&sql, bound.as_slice())?;
            if affected != ids.len() {
                // `tx` zostanie wycofana przy drop — status NIE został zapisany.
                return Err(CheckpointError::RowCountMismatch {
                    expected: ids.len(),
                    affected,
                });
            }
        }

        // 5. Przejście przez stan wewnątrztransakcyjny.
        let affected = tx.execute(
            "UPDATE upload_batches
             SET state = ?3, updated_at = ?4
             WHERE plan_id = ?1 AND ordinal = ?2 AND state = ?5",
            params![
                plan_id,
                ordinal as i64,
                BatchState::Committing.as_str(),
                now as i64,
                BatchState::Accepted.as_str(),
            ],
        )?;
        if affected != 1 {
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: BatchState::Accepted,
                to: BatchState::Committing,
            });
        }
        let affected = tx.execute(
            "UPDATE upload_batches
             SET state = ?3, completed_at = ?4, updated_at = ?4, accepted_count = record_count,
                 rejected_count = 0, unknown_count = 0, error_class = ?5
             WHERE plan_id = ?1 AND ordinal = ?2 AND state = ?6",
            params![
                plan_id,
                ordinal as i64,
                BatchState::Completed.as_str(),
                now as i64,
                ErrorClass::None.as_str(),
                BatchState::Committing.as_str(),
            ],
        )?;
        if affected != 1 {
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: BatchState::Committing,
                to: BatchState::Completed,
            });
        }

        // 6. Stan planu jest wyliczany, nigdy ustawiany „na wiarę”.
        Self::recompute_plan_state_within(&tx, plan_id, now, false)?;
        tx.commit()?;
        self.load_batch(plan_id, ordinal)
    }

    /// Oznacza paczkę jako wymagającą ręcznego rozstrzygnięcia.
    pub fn mark_batch_recovery_required(
        &self,
        plan_id: &str,
        ordinal: usize,
        error_class: ErrorClass,
        now: u64,
    ) -> Result<BatchCheckpoint, CheckpointError> {
        let tx = self.conn.unchecked_transaction()?;
        let affected = tx.execute(
            "UPDATE upload_batches
             SET state = ?3, error_class = ?4, retry_eligible = 0, updated_at = ?5
             WHERE plan_id = ?1 AND ordinal = ?2 AND state NOT IN (?6, ?7)",
            params![
                plan_id,
                ordinal as i64,
                BatchState::RecoveryRequired.as_str(),
                error_class.as_str(),
                now as i64,
                BatchState::Completed.as_str(),
                BatchState::Cancelled.as_str(),
            ],
        )?;
        if affected != 1 {
            return Err(CheckpointError::TransitionRejected {
                plan_id: plan_id.to_string(),
                ordinal,
                from: Self::batch_state_within(&tx, plan_id, ordinal)?,
                to: BatchState::RecoveryRequired,
            });
        }
        Self::recompute_plan_state_within(&tx, plan_id, now, false)?;
        tx.commit()?;
        self.load_batch(plan_id, ordinal)
    }

    /// Anuluje plan. Paczki nie są wysyłane ponownie.
    pub fn cancel_upload_plan(
        &self,
        plan_id: &str,
        now: u64,
    ) -> Result<PlanCheckpoint, CheckpointError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE upload_batches
             SET state = ?2, error_class = ?3, retry_eligible = 0, updated_at = ?4
             WHERE plan_id = ?1 AND state IN (?5, ?6)",
            params![
                plan_id,
                BatchState::Cancelled.as_str(),
                ErrorClass::Cancelled.as_str(),
                now as i64,
                BatchState::Planned.as_str(),
                BatchState::Prepared.as_str(),
            ],
        )?;
        let affected = tx.execute(
            "UPDATE upload_plans SET state = ?2, updated_at = ?3 WHERE plan_id = ?1 AND state NOT IN (?4, ?5)",
            params![
                plan_id,
                PlanState::Cancelled.as_str(),
                now as i64,
                PlanState::Completed.as_str(),
                PlanState::Cancelled.as_str(),
            ],
        )?;
        if affected != 1 {
            let existing =
                self.load_upload_plan(plan_id)?
                    .ok_or_else(|| CheckpointError::MissingPlan {
                        plan_id: plan_id.to_string(),
                    })?;
            if existing.state != PlanState::Cancelled {
                return Err(CheckpointError::PlanConflict {
                    detail: format!(
                        "plan {plan_id} jest w stanie {} i nie może zostać anulowany",
                        existing.state
                    ),
                });
            }
        }
        tx.commit()?;
        self.load_upload_plan(plan_id)?
            .ok_or_else(|| CheckpointError::MissingPlan {
                plan_id: plan_id.to_string(),
            })
    }

    /// Kończy plan, przeliczając jego stan z paczek.
    pub fn finish_upload_plan(
        &self,
        plan_id: &str,
        now: u64,
    ) -> Result<PlanCheckpoint, CheckpointError> {
        let tx = self.conn.unchecked_transaction()?;
        Self::recompute_plan_state_within(&tx, plan_id, now, false)?;
        tx.commit()?;
        self.load_upload_plan(plan_id)?
            .ok_or_else(|| CheckpointError::MissingPlan {
                plan_id: plan_id.to_string(),
            })
    }

    fn recompute_plan_state_within(
        conn: &rusqlite::Connection,
        plan_id: &str,
        now: u64,
        cancelled: bool,
    ) -> Result<PlanState, CheckpointError> {
        let current: Option<String> = conn
            .query_row(
                "SELECT state FROM upload_plans WHERE plan_id = ?1",
                params![plan_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(current) = current else {
            return Err(CheckpointError::MissingPlan {
                plan_id: plan_id.to_string(),
            });
        };
        let current = PlanState::parse(&current)
            .ok_or_else(|| CheckpointError::Database(format!("nieznany stan planu: {current}")))?;
        if current.is_terminal() {
            return Ok(current);
        }
        let mut stmt =
            conn.prepare("SELECT state FROM upload_batches WHERE plan_id = ?1 ORDER BY ordinal")?;
        let rows = stmt.query_map(params![plan_id], |row| row.get::<_, String>(0))?;
        let mut states = Vec::new();
        for row in rows {
            states.push(state_parse(&row?)?);
        }
        let next = derive_plan_state(&states, cancelled);
        let error_class = if matches!(next, PlanState::RecoveryRequired) {
            ErrorClass::UnknownOutcome.as_str()
        } else {
            ErrorClass::None.as_str()
        };
        conn.execute(
            "UPDATE upload_plans SET state = ?2, updated_at = ?3, error_class = ?4 WHERE plan_id = ?1",
            params![plan_id, next.as_str(), now as i64, error_class],
        )?;
        Ok(next)
    }

    /// Recovery po restarcie: **jednoznaczny** wynik dla każdego trwałego
    /// stanu.
    ///
    /// * `InFlight` ⇒ `Unknown` (fail-closed, **bez** ponowienia),
    /// * `Committing` ⇒ `RecoveryRequired` (stan wewnątrztransakcyjny nie ma
    ///   prawa być obserwowalny),
    /// * `Accepted` ⇒ lokalny commit **bez** ponownej wysyłki,
    /// * `Planned` / `Prepared` ⇒ pozostają dozwolone do rozpoczęcia,
    /// * `Completed` ⇒ nigdy nie wysyłane ponownie,
    /// * stan planu jest przeliczany z paczek.
    ///
    /// Funkcja nigdy nie wywołuje transportu i nigdy nie ustawia daty
    /// wysłania na „teraz”.
    pub fn recover_upload_checkpoints(
        &self,
        now: u64,
    ) -> Result<Vec<PlanRecoveryReport>, CheckpointError> {
        // 1. Stany przerwane w locie → Unknown / RecoveryRequired.
        {
            let tx = self.conn.unchecked_transaction()?;
            tx.execute(
                "UPDATE upload_batches
                 SET state = ?1, error_class = ?2, retry_eligible = 0, updated_at = ?3
                 WHERE state = ?4",
                params![
                    BatchState::Unknown.as_str(),
                    ErrorClass::InterruptedInFlight.as_str(),
                    now as i64,
                    BatchState::InFlight.as_str(),
                ],
            )?;
            tx.execute(
                "UPDATE upload_batches
                 SET state = ?1, error_class = ?2, retry_eligible = 0, updated_at = ?3
                 WHERE state = ?4",
                params![
                    BatchState::RecoveryRequired.as_str(),
                    ErrorClass::InterruptedInCommit.as_str(),
                    now as i64,
                    BatchState::Committing.as_str(),
                ],
            )?;
            tx.commit()?;
        }

        // 2. `Accepted` → dokończ lokalny commit (bez sieci).
        let accepted: Vec<(String, usize)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT plan_id, ordinal FROM upload_batches WHERE state = ?1")?;
            let rows = stmt.query_map(params![BatchState::Accepted.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            let mut collected = Vec::new();
            for row in rows {
                let (plan_id, ordinal) = row?;
                collected.push((plan_id, ordinal as usize));
            }
            collected
        };
        let mut committed = std::collections::BTreeMap::<String, usize>::new();
        for (plan_id, ordinal) in &accepted {
            match self.commit_batch_acceptance_with_dates(plan_id, *ordinal, now) {
                Ok(_) => *committed.entry(plan_id.clone()).or_default() += 1,
                Err(_) => {
                    let _ = self.mark_batch_recovery_required(
                        plan_id,
                        *ordinal,
                        ErrorClass::RowCountMismatch,
                        now,
                    );
                }
            }
        }

        // 3. Przelicz stan każdego planu i zbuduj raport.
        let plan_ids: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT plan_id FROM upload_plans ORDER BY created_at ASC, plan_id ASC")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            let mut collected = Vec::new();
            for row in rows {
                collected.push(row?);
            }
            collected
        };

        let mut reports = Vec::new();
        for plan_id in plan_ids {
            let Some(before) = self.load_upload_plan(&plan_id)? else {
                continue;
            };
            let batches = self.load_upload_batches(&plan_id)?;
            let tx = self.conn.unchecked_transaction()?;
            Self::recompute_plan_state_within(&tx, &plan_id, now, false)?;
            tx.commit()?;
            let after =
                self.load_upload_plan(&plan_id)?
                    .ok_or_else(|| CheckpointError::MissingPlan {
                        plan_id: plan_id.clone(),
                    })?;
            reports.push(PlanRecoveryReport {
                plan_id: plan_id.clone(),
                target: after.target,
                journal_id: after.journal_id.as_str().to_string(),
                previous_state: before.state,
                state: after.state,
                batches_recovered_to_unknown: batches
                    .iter()
                    .filter(|batch| {
                        batch.state == BatchState::Unknown
                            && batch.error_class == ErrorClass::InterruptedInFlight
                    })
                    .count(),
                batches_committed_locally: committed.get(&plan_id).copied().unwrap_or(0),
                batches_requiring_manual_reconciliation: batches
                    .iter()
                    .filter(|batch| batch.needs_manual_reconciliation())
                    .count(),
                batches_never_resent: batches
                    .iter()
                    .filter(|batch| {
                        matches!(batch.state, BatchState::Completed | BatchState::Cancelled)
                    })
                    .count(),
            });
        }
        Ok(reports)
    }

    /// Plany wymagające decyzji człowieka (do pokazania operatorowi).
    pub fn upload_plans_requiring_reconciliation(
        &self,
    ) -> Result<Vec<PlanCheckpoint>, CheckpointError> {
        let mut stmt = self.conn.prepare(
            "SELECT plan_id FROM upload_plans WHERE state IN (?1, ?2) ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(
            params![
                PlanState::RecoveryRequired.as_str(),
                PlanState::Running.as_str()
            ],
            |row| row.get::<_, String>(0),
        )?;
        let mut plans = Vec::new();
        for row in rows {
            let plan_id = row?;
            if let Some(plan) = self.load_upload_plan(&plan_id)? {
                plans.push(plan);
            }
        }
        Ok(plans)
    }

    /// Liczba trwałych planów (diagnostyka i testy).
    pub fn count_upload_plans(&self) -> SqlResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM upload_plans", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Liczba trwałych paczek (diagnostyka i testy).
    pub fn count_upload_batches(&self) -> SqlResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM upload_batches", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

/// Plan wraz z pełnym, zweryfikowanym zbiorem paczek.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCheckpointSet {
    pub plan: PlanCheckpoint,
    pub batches: Vec<BatchCheckpoint>,
}

impl BatchCheckpointSet {
    /// Czy wolno rozpocząć wysyłkę paczki bez decyzji człowieka.
    pub fn resumable_ordinals(&self) -> Vec<usize> {
        self.batches
            .iter()
            .filter(|batch| batch.state.is_resumable())
            .map(|batch| batch.ordinal)
            .collect()
    }

    /// Numery paczek wymagających decyzji człowieka.
    pub fn manual_reconciliation_ordinals(&self) -> Vec<usize> {
        self.batches
            .iter()
            .filter(|batch| batch.needs_manual_reconciliation())
            .map(|batch| batch.ordinal)
            .collect()
    }

    /// Czy cały plan jest zakończony z pełnym sukcesem.
    pub fn all_batches_completed(&self) -> bool {
        !self.batches.is_empty()
            && self
                .batches
                .iter()
                .all(|batch| batch.state == BatchState::Completed)
    }
}
