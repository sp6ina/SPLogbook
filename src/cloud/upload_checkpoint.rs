// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Trwały model checkpointów planu uploadu (F-20R, ryzyko **F20-R1**).
//!
//! Ten moduł jest **czysty** (bez I/O): zawiera wyłącznie tożsamości, stany,
//! dozwolone przejścia, klasy błędów i deterministyczne skróty. Persystencja
//! znajduje się w [`crate::core::database::upload_checkpoint`], a orkiestracja
//! w [`crate::gui::online_sync_upload`].
//!
//! Dlaczego osobny model: `BatchOutcome` z F-20 opisuje **wynik** jednej próby
//! transportu, a nie **stan** trwałego obiektu. Bez rozróżnienia wyniku próby od
//! stanu checkpointu nie da się odpowiedzieć na pytanie, czy po restarcie wolno
//! wysłać tę paczkę ponownie. Odpowiedź na to pytanie jest jedynym celem tego
//! modułu.
//!
//! Niezmienniki:
//!  * każdy plan i każda paczka mają jawny `journal_id` i `service` (nigdy
//!    nazwę dziennika, nigdy indeks GUI),
//!  * każdy plan i każda paczka mają `schema_version`,
//!  * `Completed` nigdy nie jest ponownie wysyłane,
//!  * `Unknown` nigdy nie jest automatycznie ponawiane,
//!  * `InFlight` po restarcie staje się `Unknown` — nigdy `Accepted`,
//!  * `Committing` jest stanem wewnątrztransakcyjnym i nie jest obserwowalny po
//!    restarcie; jego obserwacja oznacza `RecoveryRequired`,
//!  * żaden skrót nie zawiera sekretów — wyłącznie identyfikatory i wersje.

use crate::cloud::upload_plan::{
    BatchOutcome, JournalId, SelectionPolicy, UploadPlan, UploadTarget,
};
use sha2::{Digest, Sha256};

/// Wersja schematu trwałego checkpointu (nie mylić z wersją schematu bazy).
pub const CHECKPOINT_SCHEMA_VERSION: i64 = 1;

/// Wersja polityki wyboru rekordów do planu.
pub const SELECTION_POLICY_VERSION: i64 = 1;

/// Wersja kanonicznej kolejności (`QSO_DATE`, `TIME_ON`, `id`).
pub const CANONICAL_ORDER_VERSION: i64 = 1;

/// Wersja polityki podziału na paczki (rozmiary z [`crate::cloud::upload_plan`]).
pub const BATCH_POLICY_VERSION: i64 = 1;

/// Stan trwałego planu uploadu.
///
/// Zbiór jest celowo mały: plan nie jest maszyną stanów per rekord, tylko
/// kontenerem na paczki. Stan planu jest wyliczany z paczek, nigdy ustawiany
/// bez dowodu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanState {
    /// Plan utworzony; żadna paczka nie została rozpoczęta.
    Planned,
    /// Co najmniej jedna paczka jest w toku lub zakończona, żadna nie wymaga
    /// rozstrzygnięcia.
    Running,
    /// Plan zakończony: wszystkie paczki w stanie terminalnym, brak
    /// niejednoznaczności.
    Completed,
    /// Plan anulowany przez operatora; nic nie jest wysyłane automatycznie.
    Cancelled,
    /// Plan wymaga ręcznego rozstrzygnięcia.
    RecoveryRequired,
}

impl PlanState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::RecoveryRequired => "recovery_required",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "planned" => Some(Self::Planned),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            "recovery_required" => Some(Self::RecoveryRequired),
            _ => None,
        }
    }

    /// Czy plan jest w stanie terminalnym (nie wymaga dalszej pracy).
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

impl std::fmt::Display for PlanState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stan trwałej paczki uploadu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BatchState {
    /// Członkostwo zapisane, paczka nigdy nie rozpoczęta. Wolno rozpocząć.
    Planned,
    /// Ładunek zmaterializowany i zweryfikowany, `attempt_id` przypisany.
    /// Wolno rozpocząć.
    Prepared,
    /// Wywołanie transportu rozpoczęte. Po restarcie **nie** jest sukcesem.
    InFlight,
    /// Usługa potwierdziła całą paczkę; trwały dowód istnieje, ale wiersze QSO
    /// nie zostały jeszcze zatwierdzone.
    Accepted,
    /// Usługa przetworzyła część paczki — brak możliwości przypisania
    /// akceptacji do konkretnego rekordu.
    PartiallyAccepted,
    /// Usługa odrzuciła paczkę (jednoznaczny wynik negatywny).
    Rejected,
    /// Wyniku nie da się ustalić.
    Unknown,
    /// Operator anulował; paczka nie jest wysyłana automatycznie.
    Cancelled,
    /// Stan wewnątrztransakcyjny: trwa lokalny zapis statusu i daty.
    /// Po restarcie nieobserwowalny; obserwacja oznacza `RecoveryRequired`.
    Committing,
    /// Status i data wysłania zatwierdzone atomowo. Nigdy nie wysyłane ponownie.
    Completed,
    /// Niezmiennik naruszony — wymaga ręcznego rozstrzygnięcia.
    RecoveryRequired,
}

impl BatchState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Prepared => "prepared",
            Self::InFlight => "in_flight",
            Self::Accepted => "accepted",
            Self::PartiallyAccepted => "partially_accepted",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
            Self::Cancelled => "cancelled",
            Self::Committing => "committing",
            Self::Completed => "completed",
            Self::RecoveryRequired => "recovery_required",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "planned" => Some(Self::Planned),
            "prepared" => Some(Self::Prepared),
            "in_flight" => Some(Self::InFlight),
            "accepted" => Some(Self::Accepted),
            "partially_accepted" => Some(Self::PartiallyAccepted),
            "rejected" => Some(Self::Rejected),
            "unknown" => Some(Self::Unknown),
            "cancelled" => Some(Self::Cancelled),
            "committing" => Some(Self::Committing),
            "completed" => Some(Self::Completed),
            "recovery_required" => Some(Self::RecoveryRequired),
            _ => None,
        }
    }

    /// Czy paczka jest w stanie terminalnym — żadna automatyczna akcja nie jest
    /// dozwolona.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Rejected | Self::RecoveryRequired
        )
    }

    /// Czy stan jest niejednoznaczny i wymaga decyzji człowieka.
    pub const fn is_ambiguous(self) -> bool {
        matches!(
            self,
            Self::InFlight
                | Self::PartiallyAccepted
                | Self::Unknown
                | Self::Committing
                | Self::RecoveryRequired
        )
    }

    /// Czy paczkę wolno rozpocząć bez decyzji człowieka.
    pub const fn is_resumable(self) -> bool {
        matches!(self, Self::Planned | Self::Prepared)
    }

    /// Czy paczka ma trwały dowód akceptacji oczekujący na lokalny commit.
    pub const fn awaits_local_commit(self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Dozwolone przejścia. Wszystko poza tą tabelą jest naruszeniem
    /// niezmiennika, nie miękkim błędem.
    pub const fn allows_transition_to(self, next: Self) -> bool {
        match self {
            Self::Planned => matches!(next, Self::Prepared | Self::Cancelled),
            Self::Prepared => matches!(next, Self::InFlight | Self::Cancelled),
            Self::InFlight => matches!(
                next,
                Self::Accepted
                    | Self::PartiallyAccepted
                    | Self::Rejected
                    | Self::Unknown
                    | Self::Cancelled
            ),
            Self::Accepted => matches!(next, Self::Committing | Self::RecoveryRequired),
            Self::Committing => matches!(next, Self::Completed | Self::RecoveryRequired),
            Self::PartiallyAccepted | Self::Unknown | Self::Rejected | Self::RecoveryRequired => {
                matches!(next, Self::RecoveryRequired)
            }
            Self::Completed | Self::Cancelled => false,
        }
    }
}

impl std::fmt::Display for BatchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Co wolno zrobić z paczką po restarcie. Odpowiedź zależy **wyłącznie** od
/// trwałego stanu, nigdy od czasu, kolejności ani konfiguracji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Wolno rozpocząć wysyłkę.
    Resume,
    /// Wolno dokończyć lokalny zapis statusu i daty; **bez** ponownej wysyłki.
    CommitLocalOnly,
    /// Stan `InFlight` po restarcie: przejdź w `Unknown`, nie wysyłaj.
    FailClosedUnknown,
    /// Wymaga decyzji człowieka; żadnej automatycznej akcji.
    ManualReconciliation,
    /// Stan terminalny: nigdy nie wysyłaj ponownie.
    NeverResend,
}

impl RecoveryAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resume => "resume",
            Self::CommitLocalOnly => "commit_local_only",
            Self::FailClosedUnknown => "fail_closed_unknown",
            Self::ManualReconciliation => "manual_reconciliation",
            Self::NeverResend => "never_resend",
        }
    }
}

/// Wylicza dozwoloną akcję recovery dla trwałego stanu paczki.
pub const fn recovery_action(state: BatchState) -> RecoveryAction {
    match state {
        BatchState::Planned | BatchState::Prepared => RecoveryAction::Resume,
        BatchState::Accepted => RecoveryAction::CommitLocalOnly,
        BatchState::InFlight => RecoveryAction::FailClosedUnknown,
        BatchState::Completed | BatchState::Cancelled => RecoveryAction::NeverResend,
        BatchState::PartiallyAccepted
        | BatchState::Rejected
        | BatchState::Unknown
        | BatchState::Committing
        | BatchState::RecoveryRequired => RecoveryAction::ManualReconciliation,
    }
}

/// Klasa błędu zapisana przy paczce. Tekst diagnostyczny usługi **nigdy** nie
/// jest utrwalany — tylko ta klasa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    None,
    Transport,
    Timeout,
    Protocol,
    Database,
    Cancelled,
    RowCountMismatch,
    EvidenceInvalid,
    InterruptedInFlight,
    InterruptedInCommit,
    UnknownOutcome,
}

impl ErrorClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Transport => "transport",
            Self::Timeout => "timeout",
            Self::Protocol => "protocol",
            Self::Database => "database",
            Self::Cancelled => "cancelled",
            Self::RowCountMismatch => "row_count_mismatch",
            Self::EvidenceInvalid => "evidence_invalid",
            Self::InterruptedInFlight => "interrupted_in_flight",
            Self::InterruptedInCommit => "interrupted_in_commit",
            Self::UnknownOutcome => "unknown_outcome",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "none" => Some(Self::None),
            "transport" => Some(Self::Transport),
            "timeout" => Some(Self::Timeout),
            "protocol" => Some(Self::Protocol),
            "database" => Some(Self::Database),
            "cancelled" => Some(Self::Cancelled),
            "row_count_mismatch" => Some(Self::RowCountMismatch),
            "evidence_invalid" => Some(Self::EvidenceInvalid),
            "interrupted_in_flight" => Some(Self::InterruptedInFlight),
            "interrupted_in_commit" => Some(Self::InterruptedInCommit),
            "unknown_outcome" => Some(Self::UnknownOutcome),
            _ => None,
        }
    }
}

impl std::fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Mapowanie wyniku transportu (F-20) na trwały stan paczki.
///
/// Kluczowa decyzja: `BatchOutcome::Failed` **nie** jest `Rejected`. Błąd
/// transportu nie dowodzi, że żądanie nie dotarło do usługi, więc trwały stan to
/// `Unknown` — nigdy automatyczne ponowienie.
pub const fn state_for_outcome(outcome: &BatchOutcome) -> BatchState {
    match outcome {
        BatchOutcome::Accepted => BatchState::Accepted,
        BatchOutcome::PartiallyAccepted { .. } => BatchState::PartiallyAccepted,
        BatchOutcome::Rejected { .. } => BatchState::Rejected,
        BatchOutcome::Cancelled => BatchState::Cancelled,
        BatchOutcome::NoRecordsProcessed { .. }
        | BatchOutcome::Unknown { .. }
        | BatchOutcome::TimedOut { .. }
        | BatchOutcome::Failed { .. } => BatchState::Unknown,
    }
}

/// Mapowanie wyniku transportu na klasę błędu.
pub const fn error_class_for_outcome(outcome: &BatchOutcome) -> ErrorClass {
    match outcome {
        BatchOutcome::Accepted | BatchOutcome::PartiallyAccepted { .. } => ErrorClass::None,
        BatchOutcome::Rejected { .. } => ErrorClass::Protocol,
        BatchOutcome::Unknown { .. } | BatchOutcome::NoRecordsProcessed { .. } => {
            ErrorClass::UnknownOutcome
        }
        BatchOutcome::TimedOut { .. } => ErrorClass::Timeout,
        BatchOutcome::Failed { .. } => ErrorClass::Transport,
        BatchOutcome::Cancelled => ErrorClass::Cancelled,
    }
}

/// Wynik jednej próby zapisany w checkpoincie: stan + liczby + klasa błędu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutcomeSnapshot {
    pub state: BatchState,
    pub error_class: ErrorClass,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub unknown_count: usize,
}

impl OutcomeSnapshot {
    /// Buduje snapshot dla paczki o znanej liczbie rekordów.
    pub fn from_outcome(outcome: &BatchOutcome, record_count: usize) -> Self {
        let state = state_for_outcome(outcome);
        let (accepted, rejected, unknown) = match outcome {
            BatchOutcome::Accepted => (record_count, 0, 0),
            BatchOutcome::PartiallyAccepted { rejected, .. } => {
                let rejected = (*rejected).min(record_count);
                (record_count - rejected, rejected, 0)
            }
            BatchOutcome::Rejected { .. } => (0, record_count, 0),
            // Anulowana paczka nie ma przypisania do rekordów, a błąd
            // transportu nie dowodzi, że żądanie nie dotarło do usługi: wynik
            // każdego rekordu pozostaje nieustalony, nigdy sukces.
            BatchOutcome::Cancelled
            | BatchOutcome::NoRecordsProcessed { .. }
            | BatchOutcome::Unknown { .. }
            | BatchOutcome::TimedOut { .. }
            | BatchOutcome::Failed { .. } => (0, 0, record_count),
        };
        Self {
            state,
            error_class: error_class_for_outcome(outcome),
            accepted_count: accepted,
            rejected_count: rejected,
            unknown_count: unknown,
        }
    }

    /// Czy rachunek rekordów jest spójny z rozmiarem paczki.
    pub fn accounting_consistent(&self, record_count: usize) -> bool {
        self.accepted_count + self.rejected_count + self.unknown_count == record_count
    }
}

/// Odczytany trwały checkpoint planu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanCheckpoint {
    pub plan_id: String,
    pub schema_version: i64,
    pub target: UploadTarget,
    pub journal_id: JournalId,
    pub policy: SelectionPolicy,
    pub selection_policy_version: i64,
    pub canonical_order_version: i64,
    pub batch_policy_version: i64,
    pub plan_digest: String,
    pub total_qsos: usize,
    pub total_batches: usize,
    pub created_at: u64,
    pub state: PlanState,
    pub error_class: ErrorClass,
}

/// Odczytany trwały checkpoint paczki.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCheckpoint {
    pub plan_id: String,
    pub batch_id: String,
    pub ordinal: usize,
    pub target: UploadTarget,
    pub journal_id: JournalId,
    pub record_count: usize,
    pub batch_digest: String,
    pub attempt_id: String,
    pub state: BatchState,
    pub started_at: Option<u64>,
    pub accepted_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub unknown_count: usize,
    pub evidence_digest: Option<String>,
    pub error_class: ErrorClass,
    pub retry_eligible: bool,
}

impl BatchCheckpoint {
    /// Czy paczka wymaga ręcznego rozstrzygnięcia.
    pub const fn needs_manual_reconciliation(&self) -> bool {
        matches!(
            recovery_action(self.state),
            RecoveryAction::ManualReconciliation
        )
    }
}

/// Stabilny identyfikator paczki w ramach planu — deterministyczny, bez RNG.
pub fn batch_identifier(plan_id: &str, ordinal: usize) -> String {
    format!("{plan_id}-b{ordinal:04}")
}

fn hex16(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(16);
    for byte in &bytes[..8] {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Skrót całego planu: usługa, dziennik, polityka, kolejność i pełne
/// uporządkowane członkostwo. Bez sekretów i bez czasu.
pub fn compute_plan_digest(plan: &UploadPlan) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"plan\x1f");
    hasher.update(plan.target().as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(plan.journal_id().as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(plan.policy().as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(plan.total_records().to_string().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(plan.batch_size().to_string().as_bytes());
    for id in plan.record_ids() {
        hasher.update(b"\x1f");
        hasher.update(id.to_string().as_bytes());
    }
    hex16(&hasher.finalize())
}

/// Skrót jednej paczki: tożsamość planu, usługa, dziennik, numer i pełne
/// uporządkowane `QSO id`. Pozwala wykryć zmianę składu lub rozmiaru paczki.
pub fn compute_batch_digest(
    target: UploadTarget,
    journal_id: &JournalId,
    plan_id: &str,
    ordinal: usize,
    record_ids: &[i64],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"batch\x1f");
    hasher.update(target.as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(journal_id.as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(plan_id.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(ordinal.to_string().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(record_ids.len().to_string().as_bytes());
    for id in record_ids {
        hasher.update(b"\x1f");
        hasher.update(id.to_string().as_bytes());
    }
    hex16(&hasher.finalize())
}

/// Skrót zmaterializowanego ładunku ADIF.
///
/// Przechowywany jest wyłącznie skrót — nigdy treść. Pozwala wykryć, że ładunek
/// zmienił się między przygotowaniem a wysyłką paczki.
pub fn compute_payload_digest(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"payload\x1f");
    hasher.update(payload);
    hex16(&hasher.finalize())
}

/// Skrót dowodu z odpowiedzi usługi — wyłącznie bezpieczne metadane (klasa
/// wyniku, liczby). Nigdy treść odpowiedzi, nigdy URL, nigdy sekret.
pub fn compute_evidence_digest(
    state: BatchState,
    accepted_count: usize,
    rejected_count: usize,
    unknown_count: usize,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"evidence\x1f");
    hasher.update(state.as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(accepted_count.to_string().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(rejected_count.to_string().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(unknown_count.to_string().as_bytes());
    hex16(&hasher.finalize())
}

/// Minimalny dopuszczalny rok w typie ADIF `Date` (oficjalna specyfikacja:
/// `1930 <= YYYY`).
pub const ADIF_DATE_MIN_YEAR: i32 = 1930;

/// Konwertuje znacznik czasu UNIX (UTC) na wartość ADIF `Date` (`YYYYMMDD`).
///
/// Zwraca `None`, gdy data jest poza dopuszczalnym zakresem typu `Date`.
/// Funkcja jest czysta i deterministyczna — dzięki temu testy używają zegara
/// wstrzykiwanego, a nie czasu ściennego.
pub fn adif_date_from_unix(timestamp: i64) -> Option<String> {
    let instant = jiff::Timestamp::from_second(timestamp).ok()?;
    let date = jiff::tz::TimeZone::UTC.to_datetime(instant).date();
    if i32::from(date.year()) < ADIF_DATE_MIN_YEAR {
        return None;
    }
    Some(date.strftime("%Y%m%d").to_string())
}

/// Wylicza stan planu na podstawie stanów jego paczek.
///
/// * dowolna paczka niejednoznaczna ⇒ `RecoveryRequired`,
/// * paczka `Accepted` (dowód bez commita) ⇒ `RecoveryRequired`,
/// * jakakolwiek paczka `Planned`/`Prepared` ⇒ `Running` (lub `Planned`, gdy nic
///   jeszcze nie ruszyło),
/// * wszystkie paczki terminalne i brak niejednoznaczności ⇒ `Completed`.
pub fn derive_plan_state(batches: &[BatchState], cancelled: bool) -> PlanState {
    if cancelled {
        return PlanState::Cancelled;
    }
    if batches
        .iter()
        .any(|state| state.is_ambiguous() || state.awaits_local_commit())
    {
        return PlanState::RecoveryRequired;
    }
    if batches.iter().any(|state| state.is_resumable()) {
        if batches.iter().any(|state| !state.is_resumable()) {
            return PlanState::Running;
        }
        return PlanState::Planned;
    }
    PlanState::Completed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::upload_plan::{PlanRecord, UploadPlan};

    fn journal() -> JournalId {
        JournalId::new("CHECKPOINT_JOURNAL").expect("journal")
    }

    fn plan(count: usize, batch_size: usize) -> UploadPlan {
        let records: Vec<PlanRecord> = (1..=count as i64)
            .map(|id| PlanRecord::new(id, "20260101", format!("{id:06}")))
            .collect();
        UploadPlan::build(
            UploadTarget::Lotw,
            journal(),
            SelectionPolicy::PendingOnly,
            "attempt-1",
            1_700_000_000,
            records,
            batch_size,
        )
        .expect("plan")
    }

    fn all_batch_states() -> [BatchState; 11] {
        [
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
        ]
    }

    #[test]
    fn every_state_round_trips_through_its_text_form() {
        for state in [
            PlanState::Planned,
            PlanState::Running,
            PlanState::Completed,
            PlanState::Cancelled,
            PlanState::RecoveryRequired,
        ] {
            assert_eq!(PlanState::parse(state.as_str()), Some(state));
        }
        for state in all_batch_states() {
            assert_eq!(BatchState::parse(state.as_str()), Some(state));
        }
        for class in [
            ErrorClass::None,
            ErrorClass::Transport,
            ErrorClass::Timeout,
            ErrorClass::Protocol,
            ErrorClass::Database,
            ErrorClass::Cancelled,
            ErrorClass::RowCountMismatch,
            ErrorClass::EvidenceInvalid,
            ErrorClass::InterruptedInFlight,
            ErrorClass::InterruptedInCommit,
            ErrorClass::UnknownOutcome,
        ] {
            assert_eq!(ErrorClass::parse(class.as_str()), Some(class));
        }
    }

    #[test]
    fn unknown_text_never_parses_to_a_state() {
        assert_eq!(PlanState::parse("Accepted"), None);
        assert_eq!(BatchState::parse("complete"), None);
        assert_eq!(ErrorClass::parse(""), None);
    }

    #[test]
    fn completed_and_cancelled_are_terminal_and_never_transition() {
        for state in [BatchState::Completed, BatchState::Cancelled] {
            assert!(state.is_terminal());
            assert!(!state.is_resumable());
            for next in all_batch_states() {
                assert!(
                    !state.allows_transition_to(next),
                    "{state} must not transition to {next}"
                );
            }
            assert_eq!(recovery_action(state), RecoveryAction::NeverResend);
        }
    }

    #[test]
    fn only_planned_and_prepared_are_resumable() {
        for state in all_batch_states() {
            assert_eq!(
                state.is_resumable(),
                matches!(state, BatchState::Planned | BatchState::Prepared),
                "{state}"
            );
        }
    }

    #[test]
    fn in_flight_recovery_is_fail_closed_never_accepted() {
        assert_eq!(
            recovery_action(BatchState::InFlight),
            RecoveryAction::FailClosedUnknown
        );
        assert_eq!(
            state_for_outcome(&BatchOutcome::TimedOut {
                detail: String::new()
            }),
            BatchState::Unknown
        );
        assert_eq!(
            state_for_outcome(&BatchOutcome::Unknown {
                detail: String::new()
            }),
            BatchState::Unknown
        );
    }

    #[test]
    fn transport_failure_is_unknown_not_rejected() {
        assert_eq!(
            state_for_outcome(&BatchOutcome::Failed {
                detail: String::new()
            }),
            BatchState::Unknown
        );
        assert_ne!(
            state_for_outcome(&BatchOutcome::Failed {
                detail: String::new()
            }),
            BatchState::Rejected
        );
    }

    #[test]
    fn only_an_accepted_outcome_awaits_local_commit() {
        assert!(state_for_outcome(&BatchOutcome::Accepted).awaits_local_commit());
        for outcome in [
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
        ] {
            assert!(
                !state_for_outcome(&outcome).awaits_local_commit(),
                "{outcome:?}"
            );
        }
    }

    #[test]
    fn outcome_snapshot_accounting_holds_for_every_outcome() {
        for count in [1usize, 7, 250, 500] {
            for outcome in [
                BatchOutcome::Accepted,
                BatchOutcome::PartiallyAccepted {
                    rejected: 0,
                    detail: String::new(),
                },
                BatchOutcome::PartiallyAccepted {
                    rejected: count,
                    detail: String::new(),
                },
                BatchOutcome::PartiallyAccepted {
                    rejected: count + 999,
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
            ] {
                let snapshot = OutcomeSnapshot::from_outcome(&outcome, count);
                assert!(
                    snapshot.accounting_consistent(count),
                    "count={count} outcome={outcome:?}"
                );
            }
        }
    }

    #[test]
    fn rejected_count_is_clamped_to_the_batch_size() {
        let snapshot = OutcomeSnapshot::from_outcome(
            &BatchOutcome::PartiallyAccepted {
                rejected: 999,
                detail: String::new(),
            },
            10,
        );
        assert_eq!(snapshot.rejected_count, 10);
        assert_eq!(snapshot.accepted_count, 0);
        assert!(snapshot.accounting_consistent(10));
    }

    #[test]
    fn plan_digest_is_stable_and_content_sensitive() {
        let a = plan(20, 5);
        let b = plan(20, 5);
        assert_eq!(compute_plan_digest(&a), compute_plan_digest(&b));
        assert_eq!(compute_plan_digest(&a).len(), 16);

        assert_ne!(
            compute_plan_digest(&a),
            compute_plan_digest(&plan(19, 5)),
            "a different record set must produce a different plan digest"
        );
        assert_ne!(
            compute_plan_digest(&a),
            compute_plan_digest(&plan(20, 4)),
            "the batch policy is part of the plan identity"
        );

        let other_service = UploadPlan::build(
            UploadTarget::Eqsl,
            journal(),
            SelectionPolicy::PendingOnly,
            "attempt-1",
            1_700_000_000,
            (1..=20i64)
                .map(|id| PlanRecord::new(id, "20260101", format!("{id:06}")))
                .collect(),
            5,
        )
        .expect("plan");
        assert_ne!(
            compute_plan_digest(&a),
            compute_plan_digest(&other_service),
            "the service is part of the plan identity"
        );
    }

    #[test]
    fn batch_digest_binds_plan_ordinal_and_membership() {
        let ids = vec![1i64, 2, 3];
        let base = compute_batch_digest(UploadTarget::Lotw, &journal(), "PLAN", 0, &ids);
        assert_eq!(
            base,
            compute_batch_digest(UploadTarget::Lotw, &journal(), "PLAN", 0, &ids)
        );
        assert_ne!(
            base,
            compute_batch_digest(UploadTarget::Lotw, &journal(), "PLAN", 1, &ids),
            "the ordinal is part of the batch identity"
        );
        assert_ne!(
            base,
            compute_batch_digest(UploadTarget::Lotw, &journal(), "OTHER", 0, &ids),
            "the plan is part of the batch identity"
        );
        assert_ne!(
            base,
            compute_batch_digest(UploadTarget::Eqsl, &journal(), "PLAN", 0, &ids),
            "the service is part of the batch identity"
        );
        assert_ne!(
            base,
            compute_batch_digest(UploadTarget::Lotw, &journal(), "PLAN", 0, &[1, 2, 4]),
            "membership is part of the batch identity"
        );
        assert_ne!(
            base,
            compute_batch_digest(UploadTarget::Lotw, &journal(), "PLAN", 0, &[3, 2, 1]),
            "order is part of the batch identity"
        );
        assert_ne!(
            base,
            compute_batch_digest(UploadTarget::Lotw, &journal(), "PLAN", 0, &[1, 2]),
            "size is part of the batch identity"
        );
        let other_journal = JournalId::new("OTHER_JOURNAL").expect("journal");
        assert_ne!(
            base,
            compute_batch_digest(UploadTarget::Lotw, &other_journal, "PLAN", 0, &ids),
            "the journal is part of the batch identity"
        );
    }

    #[test]
    fn batch_identifiers_are_unique_and_stable() {
        assert_eq!(batch_identifier("PLAN", 0), "PLAN-b0000");
        assert_eq!(batch_identifier("PLAN", 7), "PLAN-b0007");
        assert_ne!(batch_identifier("PLAN", 7), batch_identifier("PLAN", 8));
    }

    #[test]
    fn evidence_digest_never_contains_a_payload() {
        let digest = compute_evidence_digest(BatchState::Accepted, 500, 0, 0);
        assert_eq!(digest.len(), 16);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(
            digest,
            compute_evidence_digest(BatchState::PartiallyAccepted, 400, 100, 0)
        );
    }

    #[test]
    fn payload_digest_is_stable_and_content_sensitive() {
        assert_eq!(
            compute_payload_digest(b"<EOR>"),
            compute_payload_digest(b"<EOR>")
        );
        assert_ne!(
            compute_payload_digest(b"<EOR>"),
            compute_payload_digest(b"<EOR> ")
        );
        assert_eq!(compute_payload_digest(b"").len(), 16);
    }

    #[test]
    fn adif_date_format_is_eight_digits_and_utc() {
        // 2026-11-17T00:00:00Z
        assert_eq!(
            adif_date_from_unix(1_794_873_600),
            Some("20261117".to_string())
        );
        // 2026-12-31T23:59:59Z
        assert_eq!(
            adif_date_from_unix(1_798_761_599),
            Some("20261231".to_string())
        );
        // 1970-01-01T00:00:00Z — the earliest representable instant.
        assert_eq!(adif_date_from_unix(0), Some("19700101".to_string()));
        for timestamp in [0i64, 1_794_873_600, 1_798_761_599, 4_102_444_800] {
            let date = adif_date_from_unix(timestamp).expect("date");
            assert_eq!(date.len(), 8, "{date}");
            assert!(date.chars().all(|c| c.is_ascii_digit()), "{date}");
        }
    }

    #[test]
    fn adif_date_rejects_years_below_the_documented_floor() {
        // 1930-01-01T00:00:00Z — the documented floor `1930 <= YYYY`.
        assert_eq!(
            adif_date_from_unix(-1_262_304_000),
            Some("19300101".to_string())
        );
        // 1929-12-31T23:59:59Z — below the floor, never invented.
        assert_eq!(adif_date_from_unix(-1_262_304_001), None);
        // 1900-01-01T00:00:00Z — far below the floor.
        assert_eq!(adif_date_from_unix(-2_208_988_800), None);
    }

    #[test]
    fn leap_day_and_year_end_are_represented_without_arithmetic_shortcuts() {
        // 2028-02-29T12:00:00Z
        assert_eq!(
            adif_date_from_unix(1_835_438_400),
            Some("20280229".to_string())
        );
        // 2027-12-31T23:59:59Z
        assert_eq!(
            adif_date_from_unix(1_830_297_599),
            Some("20271231".to_string())
        );
        // 2100-01-01T00:00:00Z
        assert_eq!(
            adif_date_from_unix(4_102_444_800),
            Some("21000101".to_string())
        );
    }

    #[test]
    fn plan_state_is_derived_from_batches_never_assumed() {
        assert_eq!(derive_plan_state(&[], false), PlanState::Completed);
        assert_eq!(
            derive_plan_state(&[BatchState::Planned, BatchState::Planned], false),
            PlanState::Planned
        );
        assert_eq!(
            derive_plan_state(&[BatchState::Completed, BatchState::Planned], false),
            PlanState::Running
        );
        assert_eq!(
            derive_plan_state(&[BatchState::Completed, BatchState::Completed], false),
            PlanState::Completed
        );
        assert_eq!(
            derive_plan_state(&[BatchState::Completed, BatchState::Unknown], false),
            PlanState::RecoveryRequired
        );
        assert_eq!(
            derive_plan_state(&[BatchState::Accepted], false),
            PlanState::RecoveryRequired
        );
        assert_eq!(
            derive_plan_state(&[BatchState::Completed, BatchState::Rejected], false),
            PlanState::Completed
        );
        assert_eq!(
            derive_plan_state(&[BatchState::Completed], true),
            PlanState::Cancelled
        );
    }

    #[test]
    fn every_non_terminal_batch_state_is_reachable_from_a_documented_transition() {
        let all = all_batch_states();
        for target in all {
            if target == BatchState::Planned {
                continue;
            }
            assert!(
                all.iter().any(|source| source.allows_transition_to(target)),
                "{target} has no documented predecessor"
            );
        }
    }
}
