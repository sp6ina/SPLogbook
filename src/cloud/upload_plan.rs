// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Centralny, czysty (bez I/O) model planu uploadu i bezstratnego batchowania.
//!
//! Ten moduł jest jedynym miejscem, w którym powstaje **zakres** uploadu:
//! jawny `journal_id`, zmaterializowany zbiór tożsamości QSO, deterministyczna
//! kolejność i podział na paczki. Transport (TQSL, HTTP, pętla per-QSO) jest
//! wstrzykiwany przez [`run_plan`], dzięki czemu cała logika decyzyjna jest
//! testowalna bez sieci, bez procesów, bez czasu ściennego i bez `sleep`.
//!
//! Niezmienniki (pilnowane konstruktorami i testami):
//!  * każdy plan ma niepusty `journal_id` (tożsamość dziennika, nigdy nazwa),
//!  * każdy rekord należy do dokładnie jednej paczki,
//!  * suma długości paczek == liczba rekordów planu,
//!  * paczka nigdy nie jest pusta,
//!  * kolejność jest stabilna i jawnie zdefiniowana (rosnąco, `id` jako tie-breaker),
//!  * `batch_size == 0` jest kontrolowanym błędem,
//!  * brak cichego obcinania: plan nie ma pojemności ani limitu rekordów.

use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};

/// Rozmiar paczki LoTW/TQSL.
///
/// Oficjalna dokumentacja LoTW/TQSL **nie określa** limitu liczby QSO w jednym
/// pliku — ograniczenia dotyczą treści pojedynczego QSO, nie ich liczby. Wartość
/// jest więc jawną polityką transportową klienta (jeden proces TQSL i jeden plik
/// tymczasowy na paczkę), a nie limitem usługi.
pub const LOTW_BATCH_SIZE: usize = 500;

/// Rozmiar paczki eQSL (`ImportADIF.cfm`).
///
/// Oficjalna dokumentacja eQSL **nie określa** maksymalnej liczby rekordów dla
/// interfejsu czasu rzeczywistego. Liczba ~1000 dotyczy ręcznego uploadu przez
/// formularz w przeglądarce i nie jest przenoszona na API. Wartość jest jawną
/// polityką transportową klienta; kompletność planu jest niezależna od rozmiaru
/// paczki.
pub const EQSL_BATCH_SIZE: usize = 250;

/// Rozmiar paczki Club Log — zachowuje dotychczasowy rozmiar pojedynczego
/// żądania; powyżej tej liczby powstaje kolejna paczka zamiast obcinania.
pub const CLUBLOG_BATCH_SIZE: usize = 5000;

/// Rozmiar paczki QRZ.com — jak wyżej.
pub const QRZ_BATCH_SIZE: usize = 1000;

/// Rozmiar porcji dla usług wysyłających pojedyncze QSO (HamQTH, Cloudlog,
/// HRDLog): ogranicza liczbę rekordów pobieranych z bazy naraz, nie zakres.
pub const PER_QSO_CHUNK_SIZE: usize = 100;

/// Usługa docelowa planu uploadu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UploadTarget {
    Lotw,
    Eqsl,
    ClubLog,
    Qrz,
    HamQth,
    Cloudlog,
    HrdLog,
}

impl UploadTarget {
    /// Stabilny identyfikator tekstowy używany w `plan_id` (nigdy nie tłumaczony).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lotw => "lotw",
            Self::Eqsl => "eqsl",
            Self::ClubLog => "clublog",
            Self::Qrz => "qrz",
            Self::HamQth => "hamqth",
            Self::Cloudlog => "cloudlog",
            Self::HrdLog => "hrdlog",
        }
    }

    /// Etykieta do logu/GUI (nazwa własna usługi, nie tłumaczona).
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lotw => "LoTW",
            Self::Eqsl => "eQSL.cc",
            Self::ClubLog => "Club Log",
            Self::Qrz => "QRZ.com",
            Self::HamQth => "HamQTH",
            Self::Cloudlog => "Cloudlog",
            Self::HrdLog => "HRDLog",
        }
    }

    /// Domyślny rozmiar paczki dla usługi.
    pub const fn batch_size(self) -> usize {
        match self {
            Self::Lotw => LOTW_BATCH_SIZE,
            Self::Eqsl => EQSL_BATCH_SIZE,
            Self::ClubLog => CLUBLOG_BATCH_SIZE,
            Self::Qrz => QRZ_BATCH_SIZE,
            Self::HamQth | Self::Cloudlog | Self::HrdLog => PER_QSO_CHUNK_SIZE,
        }
    }

    /// Czy usługa ma kolumnę statusu wysyłki, której wolno użyć jako predykatu
    /// kwalifikowalności (`LOTW_QSL_SENT` / `EQSL_QSL_SENT`).
    pub const fn has_sent_status(self) -> bool {
        matches!(self, Self::Lotw | Self::Eqsl)
    }
}

/// Silna tożsamość dziennika. Opakowuje istniejące `journals.id` (TEXT).
///
/// Celowo **nie** przyjmuje nazwy dziennika ani indeksu z GUI: tożsamość musi
/// pochodzić z trwałego identyfikatora bazy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JournalId(String);

impl JournalId {
    /// Maksymalna długość akceptowanego identyfikatora dziennika.
    pub const MAX_LEN: usize = 64;

    /// Waliduje surowy identyfikator dziennika.
    ///
    /// Odrzuca pusty/biały identyfikator, identyfikator z wiodącymi lub
    /// końcowymi białymi znakami (identyfikator nie może być po cichu zmieniany),
    /// znaki sterujące oraz zbyt długie wartości.
    pub fn new(raw: &str) -> Result<Self, PlanError> {
        if raw.trim().is_empty() {
            return Err(PlanError::MissingJournal);
        }
        if raw != raw.trim() {
            return Err(PlanError::InvalidJournalId);
        }
        if raw.len() > Self::MAX_LEN {
            return Err(PlanError::InvalidJournalId);
        }
        if raw.chars().any(char::is_control) {
            return Err(PlanError::InvalidJournalId);
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for JournalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Polityka wyboru rekordów do planu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPolicy {
    /// Normalna ścieżka: rekordy kwalifikujące się (nie wysłane/nie zignorowane).
    PendingOnly,
    /// Jawne ponowienie zmaterializowanego wcześniej zbioru tożsamości.
    ExplicitRetry,
}

impl SelectionPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PendingOnly => "pending_only",
            Self::ExplicitRetry => "explicit_retry",
        }
    }
}

/// Błędy budowy planu — wszystkie kontrolowane, żaden nie panikuje.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// Brak użytecznego identyfikatora dziennika.
    MissingJournal,
    /// Identyfikator dziennika nie spełnia walidacji.
    InvalidJournalId,
    /// Brak identyfikatora próby.
    MissingAttemptId,
    /// Rozmiar paczki 0 jest błędem konfiguracji, nie „brak paczek”.
    ZeroBatchSize,
    /// Ten sam identyfikator QSO wystąpił więcej niż raz w jednym planie.
    DuplicateRecord(i64),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingJournal => write!(f, "brak aktywnego dziennika"),
            Self::InvalidJournalId => write!(f, "nieprawidłowy identyfikator dziennika"),
            Self::MissingAttemptId => write!(f, "brak identyfikatora próby"),
            Self::ZeroBatchSize => write!(f, "rozmiar paczki musi być większy od zera"),
            Self::DuplicateRecord(id) => write!(f, "zduplikowany rekord w planie: {id}"),
        }
    }
}

impl std::error::Error for PlanError {}

/// Pojedynczy rekord planu: tożsamość QSO + klucz kolejności.
///
/// Wartości `qso_date`/`time_on` są snapshotem użytym wyłącznie do ustalenia
/// deterministycznej kolejności; nie są ponownie odczytywane z bazy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRecord {
    pub id: i64,
    pub qso_date: String,
    pub time_on: String,
}

impl PlanRecord {
    pub fn new(id: i64, qso_date: impl Into<String>, time_on: impl Into<String>) -> Self {
        Self {
            id,
            qso_date: qso_date.into(),
            time_on: time_on.into(),
        }
    }

    /// Kanoniczny klucz kolejności: (data, czas, id) — rosnąco, totalny.
    fn order_key(&self) -> (String, String, i64) {
        (
            normalize_date(&self.qso_date),
            normalize_time(&self.time_on),
            self.id,
        )
    }
}

/// Normalizuje datę ADIF do porównywalnego klucza (usuwa separatory).
fn normalize_date(raw: &str) -> String {
    raw.chars()
        .filter(|c| !matches!(c, '-' | '.' | '/'))
        .collect()
}

/// Normalizuje czas ADIF do 6 cyfr (`HHMMSS`), dopełniając zerami z prawej.
fn normalize_time(raw: &str) -> String {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let trimmed = if digits.len() > 6 {
        &digits[..6]
    } else {
        &digits
    };
    format!("{trimmed:0<6}")
}

/// Zmaterializowany plan uploadu jednej usługi w jednym dzienniku.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPlan {
    target: UploadTarget,
    journal_id: JournalId,
    policy: SelectionPolicy,
    plan_id: String,
    attempt_id: String,
    created_at: u64,
    records: Vec<PlanRecord>,
    batch_size: usize,
}

impl UploadPlan {
    /// Buduje plan: waliduje dziennik i paczkę, sortuje kanonicznie, wykrywa
    /// duplikaty i wylicza deterministyczny `plan_id`.
    ///
    /// Sortowanie odbywa się tutaj, a nie w SQL — plan jest więc identyczny
    /// niezależnie od tego, w jakiej kolejności rekordy przyszły z bazy, z GUI
    /// albo z serializacji.
    pub fn build(
        target: UploadTarget,
        journal_id: JournalId,
        policy: SelectionPolicy,
        attempt_id: impl Into<String>,
        created_at: u64,
        mut records: Vec<PlanRecord>,
        batch_size: usize,
    ) -> Result<Self, PlanError> {
        if batch_size == 0 {
            return Err(PlanError::ZeroBatchSize);
        }
        let attempt_id = attempt_id.into();
        if attempt_id.trim().is_empty() {
            return Err(PlanError::MissingAttemptId);
        }

        records.sort_by_key(PlanRecord::order_key);

        let mut previous: Option<i64> = None;
        for record in &records {
            if previous == Some(record.id) {
                return Err(PlanError::DuplicateRecord(record.id));
            }
            previous = Some(record.id);
        }

        let plan_id = compute_plan_id(target, &journal_id, policy, &records);

        Ok(Self {
            target,
            journal_id,
            policy,
            plan_id,
            attempt_id,
            created_at,
            records,
            batch_size,
        })
    }

    pub fn target(&self) -> UploadTarget {
        self.target
    }

    pub fn journal_id(&self) -> &JournalId {
        &self.journal_id
    }

    pub fn policy(&self) -> SelectionPolicy {
        self.policy
    }

    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    pub fn records(&self) -> &[PlanRecord] {
        &self.records
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn total_records(&self) -> usize {
        self.records.len()
    }

    /// Identyfikatory QSO w kolejności kanonicznej.
    pub fn record_ids(&self) -> Vec<i64> {
        self.records.iter().map(|r| r.id).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Liczba paczek bez przepełnienia (pusty plan → 0 paczek).
    pub fn batch_count(&self) -> usize {
        if self.records.is_empty() {
            0
        } else {
            self.records.len().div_ceil(self.batch_size)
        }
    }

    /// Dzieli plan na paczki: rozłączne, niepuste, pokrywające całość,
    /// w kolejności planu.
    pub fn batches(&self) -> Vec<PlanBatch> {
        let total = self.batch_count();
        if total == 0 {
            return Vec::new();
        }
        self.records
            .chunks(self.batch_size)
            .enumerate()
            .map(|(index, chunk)| PlanBatch {
                target: self.target,
                journal_id: self.journal_id.clone(),
                plan_id: self.plan_id.clone(),
                attempt_id: self.attempt_id.clone(),
                index,
                total_batches: total,
                record_ids: chunk.iter().map(|r| r.id).collect(),
            })
            .collect()
    }
}

/// Jedna paczka planu. Należy do dokładnie jednego planu i jednego dziennika.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanBatch {
    target: UploadTarget,
    journal_id: JournalId,
    plan_id: String,
    attempt_id: String,
    index: usize,
    total_batches: usize,
    record_ids: Vec<i64>,
}

impl PlanBatch {
    pub fn target(&self) -> UploadTarget {
        self.target
    }

    pub fn journal_id(&self) -> &JournalId {
        &self.journal_id
    }

    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    /// Indeks paczki liczony od 0.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Indeks paczki liczony od 1 (do komunikatów dla użytkownika).
    pub fn number(&self) -> usize {
        self.index + 1
    }

    pub fn total_batches(&self) -> usize {
        self.total_batches
    }

    pub fn record_ids(&self) -> &[i64] {
        &self.record_ids
    }

    pub fn count(&self) -> usize {
        self.record_ids.len()
    }
}

/// Empiryczny wynik jednej paczki — jedyny nośnik dowodu, na podstawie którego
/// wolno zmienić status QSO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchOutcome {
    /// Usługa potwierdziła całą paczkę.
    Accepted,
    /// Usługa przetworzyła część paczki (np. TQSL kod 9, eQSL `Warning:`).
    PartiallyAccepted { rejected: usize, detail: String },
    /// Usługa nie przetworzyła żadnego rekordu (np. TQSL kod 8: duplikaty /
    /// poza zakresem dat — kody te są nierozróżnialne).
    NoRecordsProcessed { detail: String },
    /// Usługa odrzuciła paczkę.
    Rejected { detail: String },
    /// Wyniku nie da się ustalić.
    Unknown { detail: String },
    /// Przekroczono limit czasu — brak dowodu sukcesu.
    TimedOut { detail: String },
    /// Błąd transportu lub protokołu.
    Failed { detail: String },
    /// Operacja anulowana.
    Cancelled,
}

impl BatchOutcome {
    /// Czy wynik jest dowodem pełnego sukcesu paczki.
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Status pojedynczego rekordu wynikający z dowodu dla całej paczki.
    ///
    /// Tylko `Accepted` daje `Accepted`; częściowy wynik nie pozwala przypisać
    /// akceptacji do konkretnego rekordu, więc rekordy pozostają `Unknown`.
    pub fn record_status(&self) -> RecordUploadStatus {
        match self {
            Self::Accepted => RecordUploadStatus::Accepted,
            Self::Rejected { .. } => RecordUploadStatus::Rejected,
            Self::PartiallyAccepted { .. } | Self::NoRecordsProcessed { .. } => {
                RecordUploadStatus::Unknown
            }
            Self::Unknown { .. } | Self::TimedOut { .. } => RecordUploadStatus::Unknown,
            Self::Failed { .. } => RecordUploadStatus::Failed,
            Self::Cancelled => RecordUploadStatus::NotAttempted,
        }
    }

    /// Krótki opis do logu/raportu.
    pub fn detail(&self) -> &str {
        match self {
            Self::Accepted | Self::Cancelled => "",
            Self::PartiallyAccepted { detail, .. }
            | Self::NoRecordsProcessed { detail }
            | Self::Rejected { detail }
            | Self::Unknown { detail }
            | Self::TimedOut { detail }
            | Self::Failed { detail } => detail,
        }
    }
}

/// Status pojedynczego rekordu QSO w ramach uruchomienia planu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordUploadStatus {
    Accepted,
    Rejected,
    Unknown,
    Failed,
    NotAttempted,
}

/// Zbiorczy status uruchomienia planu. `Complete` jest jedynym sukcesem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Empty,
    Complete,
    Partial,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub const fn is_success(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Wykonanie pojedynczej paczki w raporcie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRunRecord {
    pub index: usize,
    pub total_batches: usize,
    pub record_ids: Vec<i64>,
    pub outcome: BatchOutcome,
}

impl BatchRunRecord {
    pub fn count(&self) -> usize {
        self.record_ids.len()
    }
}

/// Pełny raport uruchomienia planu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadRunReport {
    pub target: UploadTarget,
    pub journal_id: JournalId,
    pub plan_id: String,
    pub attempt_id: String,
    pub total_records: usize,
    pub batch_count: usize,
    pub batches: Vec<BatchRunRecord>,
    pub accepted_records: usize,
    pub rejected_records: usize,
    pub unknown_records: usize,
    pub failed_records: usize,
    pub not_attempted_records: usize,
    pub cancelled: bool,
    pub status: RunStatus,
}

impl UploadRunReport {
    /// Identyfikatory QSO, dla których istnieje dowód akceptacji.
    pub fn accepted_record_ids(&self) -> Vec<i64> {
        self.batches
            .iter()
            .filter(|b| b.outcome.is_accepted())
            .flat_map(|b| b.record_ids.iter().copied())
            .collect()
    }

    /// Identyfikatory QSO z paczek, które nie zostały w pełni zaakceptowane.
    pub fn non_accepted_record_ids(&self) -> Vec<i64> {
        self.batches
            .iter()
            .filter(|b| !b.outcome.is_accepted())
            .flat_map(|b| b.record_ids.iter().copied())
            .collect()
    }

    /// Kontrola spójności rachunku: suma paczek == liczba rekordów planu.
    pub fn accounting_consistent(&self) -> bool {
        let summed: usize = self.batches.iter().map(BatchRunRecord::count).sum();
        summed == self.total_records
            && self.accepted_records
                + self.rejected_records
                + self.unknown_records
                + self.failed_records
                + self.not_attempted_records
                == self.total_records
    }
}

/// Uruchamia plan paczka po paczce, przekazując każdą paczkę do wstrzykniętego
/// transportu.
///
/// * `cancel` jest sprawdzany **przed** i **po** każdej paczce; anulowanie nigdy
///   nie zamienia braku dowodu w sukces.
/// * Pierwszy błąd nie przerywa pętli — pozostałe paczki są nadal próbowane
///   (błąd jednej paczki nie unieważnia pozostałych), chyba że nastąpiło
///   anulowanie.
/// * Nie ma automatycznego ponawiania: transport jest wywoływany dokładnie raz
///   na paczkę.
pub async fn run_plan<F, Fut>(
    plan: &UploadPlan,
    cancel: &AtomicBool,
    mut send: F,
) -> UploadRunReport
where
    F: FnMut(PlanBatch) -> Fut,
    Fut: std::future::Future<Output = BatchOutcome>,
{
    let batches = plan.batches();
    let mut batch_records: Vec<BatchRunRecord> = Vec::with_capacity(batches.len());
    let mut cancelled = false;

    for batch in batches {
        let index = batch.index();
        let total_batches = batch.total_batches();
        let record_ids = batch.record_ids().to_vec();

        let outcome = if cancelled || cancel.load(Ordering::SeqCst) {
            cancelled = true;
            BatchOutcome::Cancelled
        } else {
            let outcome = send(batch).await;
            if matches!(outcome, BatchOutcome::Cancelled) || cancel.load(Ordering::SeqCst) {
                cancelled = true;
            }
            outcome
        };

        batch_records.push(BatchRunRecord {
            index,
            total_batches,
            record_ids,
            outcome,
        });
    }

    build_report(plan, batch_records, cancelled)
}

fn build_report(
    plan: &UploadPlan,
    batches: Vec<BatchRunRecord>,
    cancelled: bool,
) -> UploadRunReport {
    let mut accepted_records = 0usize;
    let mut rejected_records = 0usize;
    let mut unknown_records = 0usize;
    let mut failed_records = 0usize;
    let mut not_attempted_records = 0usize;

    for batch in &batches {
        let count = batch.count();
        match batch.outcome.record_status() {
            RecordUploadStatus::Accepted => accepted_records += count,
            RecordUploadStatus::Rejected => rejected_records += count,
            RecordUploadStatus::Unknown => unknown_records += count,
            RecordUploadStatus::Failed => failed_records += count,
            RecordUploadStatus::NotAttempted => not_attempted_records += count,
        }
    }

    let total_records = plan.total_records();
    let status = if total_records == 0 {
        RunStatus::Empty
    } else if cancelled {
        RunStatus::Cancelled
    } else if accepted_records == total_records {
        RunStatus::Complete
    } else if accepted_records > 0 {
        RunStatus::Partial
    } else {
        RunStatus::Failed
    };

    UploadRunReport {
        target: plan.target(),
        journal_id: plan.journal_id().clone(),
        plan_id: plan.plan_id().to_string(),
        attempt_id: plan.attempt_id().to_string(),
        total_records,
        batch_count: plan.batch_count(),
        batches,
        accepted_records,
        rejected_records,
        unknown_records,
        failed_records,
        not_attempted_records,
        cancelled,
        status,
    }
}

/// Determinizm planu: `plan_id` zależy wyłącznie od (usługa, dziennik, polityka,
/// uporządkowane identyfikatory). Bez RNG, bez czasu, bez stanu procesu.
fn compute_plan_id(
    target: UploadTarget,
    journal_id: &JournalId,
    policy: SelectionPolicy,
    records: &[PlanRecord],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(target.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(journal_id.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(policy.as_str().as_bytes());
    for record in records {
        hasher.update(b"|");
        hasher.update(record.id.to_string().as_bytes());
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(16);
    for byte in &digest[..8] {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Determinizm kolejności — wydzielone, aby testy mogły porównać klucze.
pub fn canonical_order_key(record: &PlanRecord) -> (String, String, i64) {
    record.order_key()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal() -> JournalId {
        JournalId::new("JOURNAL_A").unwrap()
    }

    fn records(count: usize) -> Vec<PlanRecord> {
        (1..=count)
            .map(|i| PlanRecord::new(i as i64, "20260101", format!("{i:06}")))
            .collect()
    }

    fn plan_with(records: Vec<PlanRecord>, batch_size: usize) -> UploadPlan {
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

    #[test]
    fn journal_id_rejects_empty_and_whitespace() {
        assert_eq!(JournalId::new(""), Err(PlanError::MissingJournal));
        assert_eq!(JournalId::new("   "), Err(PlanError::MissingJournal));
        assert_eq!(JournalId::new("\t"), Err(PlanError::MissingJournal));
        assert_eq!(JournalId::new(" DEFAULT"), Err(PlanError::InvalidJournalId));
        assert_eq!(JournalId::new("DEFAULT "), Err(PlanError::InvalidJournalId));
    }

    #[test]
    fn journal_id_rejects_control_characters_and_overlong_values() {
        assert_eq!(JournalId::new("A\u{0}B"), Err(PlanError::InvalidJournalId));
        let long = "J".repeat(JournalId::MAX_LEN + 1);
        assert_eq!(JournalId::new(&long), Err(PlanError::InvalidJournalId));
        let max = "J".repeat(JournalId::MAX_LEN);
        assert!(JournalId::new(&max).is_ok());
    }

    #[test]
    fn journal_id_keeps_exact_identity() {
        let id = JournalId::new("PORTABLE").unwrap();
        assert_eq!(id.as_str(), "PORTABLE");
        assert_eq!(id.to_string(), "PORTABLE");
        // Nazwa dziennika nie jest tożsamością: identyfikator jest case-sensitive.
        assert_ne!(JournalId::new("portable").unwrap(), id);
    }

    #[test]
    fn zero_batch_size_is_a_controlled_error() {
        let err = UploadPlan::build(
            UploadTarget::Eqsl,
            journal(),
            SelectionPolicy::PendingOnly,
            "a",
            0,
            records(3),
            0,
        );
        assert_eq!(err, Err(PlanError::ZeroBatchSize));
    }

    #[test]
    fn empty_attempt_id_is_rejected() {
        let err = UploadPlan::build(
            UploadTarget::Eqsl,
            journal(),
            SelectionPolicy::PendingOnly,
            "  ",
            0,
            records(1),
            10,
        );
        assert_eq!(err, Err(PlanError::MissingAttemptId));
    }

    #[test]
    fn duplicate_record_is_rejected_not_silently_deduplicated() {
        let mut input = records(3);
        input.push(PlanRecord::new(2, "20260101", "000002"));
        let err = UploadPlan::build(
            UploadTarget::Lotw,
            journal(),
            SelectionPolicy::PendingOnly,
            "a",
            0,
            input,
            10,
        );
        assert_eq!(err, Err(PlanError::DuplicateRecord(2)));
    }

    #[test]
    fn plan_sorts_input_into_canonical_order() {
        let input = vec![
            PlanRecord::new(3, "2026-01-02", "10:00"),
            PlanRecord::new(1, "20260101", "2359"),
            PlanRecord::new(2, "2026-01-01", "00:01"),
        ];
        let plan = plan_with(input, 10);
        assert_eq!(plan.record_ids(), vec![2, 1, 3]);
        let keys: Vec<_> = plan.records().iter().map(canonical_order_key).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn identical_timestamps_are_ordered_by_id() {
        let input = vec![
            PlanRecord::new(30, "20260101", "120000"),
            PlanRecord::new(10, "20260101", "120000"),
            PlanRecord::new(20, "20260101", "120000"),
        ];
        let plan = plan_with(input, 10);
        assert_eq!(plan.record_ids(), vec![10, 20, 30]);
    }

    #[test]
    fn time_is_padded_on_the_right_like_the_sql_order_key() {
        // Klucz kolejności jest identyczny z ORDER BY w SQL: TIME_ON bez
        // separatorów, dopełniane zerami z prawej do 6 znaków. Dlatego "10"
        // (100000) sortuje się przed "9" (900000).
        let input = vec![
            PlanRecord::new(1, "20260101", "10"),
            PlanRecord::new(2, "20260101", "9"),
        ];
        let plan = plan_with(input, 10);
        assert_eq!(plan.record_ids(), vec![1, 2]);
    }

    #[test]
    fn equal_time_keys_are_broken_by_id() {
        // "1200" i "120000" dają ten sam klucz 120000 — decyduje id.
        let input = vec![
            PlanRecord::new(5, "20260101", "120000"),
            PlanRecord::new(3, "20260101", "1200"),
        ];
        let plan = plan_with(input, 10);
        assert_eq!(plan.record_ids(), vec![3, 5]);
    }

    #[test]
    fn date_separators_are_normalised() {
        let input = vec![
            PlanRecord::new(2, "2026-01-02", "0000"),
            PlanRecord::new(1, "20260101", "0000"),
        ];
        let plan = plan_with(input, 10);
        assert_eq!(plan.record_ids(), vec![1, 2]);
    }

    #[test]
    fn plan_is_deterministic_regardless_of_input_order() {
        let mut a = records(50);
        let mut b = a.clone();
        b.reverse();
        a.rotate_left(17);
        let plan_a = plan_with(a, 7);
        let plan_b = plan_with(b, 7);
        assert_eq!(plan_a.plan_id(), plan_b.plan_id());
        assert_eq!(plan_a.record_ids(), plan_b.record_ids());
    }

    #[test]
    fn plan_id_depends_on_service_journal_and_policy() {
        let base = plan_with(records(5), 2);
        let other_service = UploadPlan::build(
            UploadTarget::Eqsl,
            journal(),
            SelectionPolicy::PendingOnly,
            "attempt-1",
            1_700_000_000,
            records(5),
            2,
        )
        .unwrap();
        let other_journal = UploadPlan::build(
            UploadTarget::Lotw,
            JournalId::new("JOURNAL_B").unwrap(),
            SelectionPolicy::PendingOnly,
            "attempt-1",
            1_700_000_000,
            records(5),
            2,
        )
        .unwrap();
        let other_policy = UploadPlan::build(
            UploadTarget::Lotw,
            journal(),
            SelectionPolicy::ExplicitRetry,
            "attempt-1",
            1_700_000_000,
            records(5),
            2,
        )
        .unwrap();
        assert_ne!(base.plan_id(), other_service.plan_id());
        assert_ne!(base.plan_id(), other_journal.plan_id());
        assert_ne!(base.plan_id(), other_policy.plan_id());
        assert_eq!(base.plan_id().len(), 16);
    }

    #[test]
    fn attempt_id_and_created_at_do_not_change_plan_id() {
        let a = UploadPlan::build(
            UploadTarget::Lotw,
            journal(),
            SelectionPolicy::PendingOnly,
            "attempt-a",
            10,
            records(4),
            2,
        )
        .unwrap();
        let b = UploadPlan::build(
            UploadTarget::Lotw,
            journal(),
            SelectionPolicy::PendingOnly,
            "attempt-b",
            99,
            records(4),
            2,
        )
        .unwrap();
        assert_eq!(a.plan_id(), b.plan_id());
        assert_ne!(a.attempt_id(), b.attempt_id());
    }

    #[test]
    fn empty_plan_has_no_batches() {
        let plan = plan_with(Vec::new(), 10);
        assert_eq!(plan.total_records(), 0);
        assert_eq!(plan.batch_count(), 0);
        assert_eq!(plan.batches().len(), 0);
    }

    #[test]
    fn batches_are_never_empty_and_cover_everything() {
        for count in [1usize, 2, 9, 10, 11, 999, 1000, 1001, 1505, 10_000] {
            for size in [
                1usize,
                2,
                3,
                250,
                500,
                999,
                1000,
                1001,
                1505,
                10_000,
                usize::MAX,
            ] {
                let plan = plan_with(records(count), size);
                let batches = plan.batches();
                assert!(!batches.is_empty(), "count={count} size={size}");
                for batch in &batches {
                    assert_ne!(batch.record_ids().len(), 0);
                }
                let summed: usize = batches.iter().map(PlanBatch::count).sum();
                assert_eq!(summed, count, "count={count} size={size}");
                assert_eq!(batches.len(), plan.batch_count());
            }
        }
    }

    #[test]
    fn batches_are_disjoint_and_ordered() {
        let plan = plan_with(records(1505), 100);
        let batches = plan.batches();
        let mut seen = std::collections::HashSet::new();
        let mut flat = Vec::new();
        for batch in &batches {
            for id in batch.record_ids() {
                assert!(seen.insert(*id), "duplicate id {id}");
                flat.push(*id);
            }
        }
        assert_eq!(flat, plan.record_ids());
        assert_eq!(seen.len(), plan.total_records());
    }

    #[test]
    fn batch_size_larger_than_set_makes_one_batch() {
        let plan = plan_with(records(10), usize::MAX);
        let batches = plan.batches();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].count(), 10);
    }

    #[test]
    fn exact_multiple_has_no_partial_tail() {
        let plan = plan_with(records(20), 5);
        let batches = plan.batches();
        assert_eq!(batches.len(), 4);
        assert!(batches.iter().all(|b| b.count() == 5));
    }

    #[test]
    fn partial_tail_batch_is_kept() {
        let plan = plan_with(records(1505), 500);
        let batches = plan.batches();
        assert_eq!(batches.len(), 4);
        assert_eq!(batches[3].count(), 5);
        assert_eq!(batches[0].count(), 500);
    }

    #[test]
    fn batch_carries_plan_and_journal_identity() {
        let plan = plan_with(records(5), 2);
        for batch in plan.batches() {
            assert_eq!(batch.plan_id(), plan.plan_id());
            assert_eq!(batch.journal_id().as_str(), "JOURNAL_A");
            assert_eq!(batch.target(), UploadTarget::Lotw);
            assert_eq!(batch.attempt_id(), plan.attempt_id());
        }
    }

    #[test]
    fn batch_number_is_one_based_and_index_zero_based() {
        let plan = plan_with(records(5), 2);
        let batches = plan.batches();
        assert_eq!(batches[0].index(), 0);
        assert_eq!(batches[0].number(), 1);
        assert_eq!(batches[2].index(), 2);
        assert_eq!(batches[2].number(), 3);
        assert_eq!(batches[0].total_batches(), 3);
    }

    #[test]
    fn batch_outcome_record_status_mapping() {
        assert_eq!(
            BatchOutcome::Accepted.record_status(),
            RecordUploadStatus::Accepted
        );
        assert_eq!(
            BatchOutcome::PartiallyAccepted {
                rejected: 1,
                detail: String::new()
            }
            .record_status(),
            RecordUploadStatus::Unknown
        );
        assert_eq!(
            BatchOutcome::NoRecordsProcessed {
                detail: String::new()
            }
            .record_status(),
            RecordUploadStatus::Unknown
        );
        assert_eq!(
            BatchOutcome::Rejected {
                detail: String::new()
            }
            .record_status(),
            RecordUploadStatus::Rejected
        );
        assert_eq!(
            BatchOutcome::Unknown {
                detail: String::new()
            }
            .record_status(),
            RecordUploadStatus::Unknown
        );
        assert_eq!(
            BatchOutcome::TimedOut {
                detail: String::new()
            }
            .record_status(),
            RecordUploadStatus::Unknown
        );
        assert_eq!(
            BatchOutcome::Failed {
                detail: String::new()
            }
            .record_status(),
            RecordUploadStatus::Failed
        );
        assert_eq!(
            BatchOutcome::Cancelled.record_status(),
            RecordUploadStatus::NotAttempted
        );
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(future)
    }

    fn run_with(plan: &UploadPlan, outcomes: Vec<BatchOutcome>) -> UploadRunReport {
        let cancel = AtomicBool::new(false);
        let mut queue = std::collections::VecDeque::from(outcomes);
        block_on(run_plan(plan, &cancel, move |_batch| {
            let next = queue.pop_front().unwrap_or(BatchOutcome::Accepted);
            async move { next }
        }))
    }

    #[test]
    fn all_batches_accepted_is_complete() {
        let plan = plan_with(records(10), 3);
        let report = run_with(
            &plan,
            vec![
                BatchOutcome::Accepted,
                BatchOutcome::Accepted,
                BatchOutcome::Accepted,
                BatchOutcome::Accepted,
            ],
        );
        assert_eq!(report.status, RunStatus::Complete);
        assert_eq!(report.accepted_records, 10);
        assert_eq!(report.accepted_record_ids().len(), 10);
        assert!(report.accounting_consistent());
        assert!(!report.cancelled);
    }

    #[test]
    fn partial_success_is_not_complete() {
        let plan = plan_with(records(10), 3);
        let report = run_with(
            &plan,
            vec![
                BatchOutcome::Accepted,
                BatchOutcome::PartiallyAccepted {
                    rejected: 1,
                    detail: "some ignored".into(),
                },
                BatchOutcome::Accepted,
                BatchOutcome::Accepted,
            ],
        );
        assert_eq!(report.status, RunStatus::Partial);
        assert!(!report.status.is_success());
        assert!(report.accepted_records < report.total_records);
        assert_eq!(report.accepted_records, 7);
        assert_eq!(report.unknown_records, 3);
        assert!(report.accounting_consistent());
    }

    #[test]
    fn no_records_processed_is_not_success() {
        let plan = plan_with(records(4), 4);
        let report = run_with(
            &plan,
            vec![BatchOutcome::NoRecordsProcessed {
                detail: "duplicates/out of range".into(),
            }],
        );
        assert_eq!(report.status, RunStatus::Failed);
        assert_eq!(report.accepted_records, 0);
        assert_eq!(report.unknown_records, 4);
        assert_eq!(report.accepted_record_ids().len(), 0);
    }

    #[test]
    fn timeout_is_never_success() {
        let plan = plan_with(records(3), 2);
        let report = run_with(
            &plan,
            vec![
                BatchOutcome::TimedOut {
                    detail: "timeout".into(),
                },
                BatchOutcome::TimedOut {
                    detail: "timeout".into(),
                },
            ],
        );
        assert_eq!(report.status, RunStatus::Failed);
        assert_eq!(report.accepted_records, 0);
        assert_eq!(report.unknown_records, 3);
    }

    #[test]
    fn failed_batch_does_not_stop_later_batches() {
        let plan = plan_with(records(9), 3);
        let report = run_with(
            &plan,
            vec![
                BatchOutcome::Failed {
                    detail: "boom".into(),
                },
                BatchOutcome::Accepted,
                BatchOutcome::Accepted,
            ],
        );
        assert_eq!(report.batches.len(), 3);
        assert_eq!(report.accepted_records, 6);
        assert_eq!(report.failed_records, 3);
        assert_eq!(report.status, RunStatus::Partial);
        assert!(report.accounting_consistent());
    }

    #[test]
    fn cancellation_marks_remaining_batches_not_attempted() {
        let plan = plan_with(records(9), 3);
        let cancel = AtomicBool::new(false);
        let mut calls = 0usize;
        let report = block_on(run_plan(&plan, &cancel, |_batch| {
            calls += 1;
            async move { BatchOutcome::Accepted }
        }));
        // Bez anulowania: 3 wywołania, pełny sukces.
        assert_eq!(calls, 3);
        assert_eq!(report.status, RunStatus::Complete);

        let plan = plan_with(records(9), 3);
        let cancel = AtomicBool::new(true);
        let mut calls = 0usize;
        let report = block_on(run_plan(&plan, &cancel, |_batch| {
            calls += 1;
            async move { BatchOutcome::Accepted }
        }));
        assert_eq!(calls, 0, "anulowanie przed pierwszą paczką nie wysyła nic");
        assert_eq!(report.status, RunStatus::Cancelled);
        assert_eq!(report.not_attempted_records, 9);
        assert_eq!(report.accepted_records, 0);
        assert!(report.cancelled);
    }

    #[test]
    fn cancellation_after_first_batch_keeps_accepted_records() {
        let plan = plan_with(records(9), 3);
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let cancel_inner = cancel.clone();
        let report = block_on(run_plan(&plan, &cancel, move |_batch| {
            let flag = cancel_inner.clone();
            async move {
                flag.store(true, Ordering::SeqCst);
                BatchOutcome::Accepted
            }
        }));
        assert_eq!(report.status, RunStatus::Cancelled);
        assert_eq!(report.accepted_records, 3);
        assert_eq!(report.not_attempted_records, 6);
        assert!(report.accounting_consistent());
        assert!(!report.status.is_success());
    }

    #[test]
    fn transport_cancelled_outcome_is_not_success() {
        let plan = plan_with(records(4), 4);
        let report = run_with(&plan, vec![BatchOutcome::Cancelled]);
        assert_eq!(report.status, RunStatus::Cancelled);
        assert_eq!(report.accepted_records, 0);
        assert_eq!(report.not_attempted_records, 4);
    }

    #[test]
    fn run_status_empty_for_empty_plan() {
        let plan = plan_with(Vec::new(), 5);
        let report = run_with(&plan, Vec::new());
        assert_eq!(report.status, RunStatus::Empty);
        assert_eq!(report.total_records, 0);
        assert!(report.accounting_consistent());
        assert!(!report.status.is_success());
    }

    #[test]
    fn every_batch_is_sent_exactly_once() {
        let plan = plan_with(records(100), 7);
        let mut seen: Vec<usize> = Vec::new();
        let cancel = AtomicBool::new(false);
        let report = block_on(run_plan(&plan, &cancel, |batch| {
            seen.push(batch.index());
            async move { BatchOutcome::Accepted }
        }));
        assert_eq!(seen, (0..plan.batch_count()).collect::<Vec<_>>());
        assert_eq!(report.accepted_records, 100);
    }

    #[test]
    fn target_batch_sizes_are_documented_policies() {
        assert_eq!(UploadTarget::Lotw.batch_size(), LOTW_BATCH_SIZE);
        assert_eq!(UploadTarget::Eqsl.batch_size(), EQSL_BATCH_SIZE);
        assert!(UploadTarget::Lotw.has_sent_status());
        assert!(UploadTarget::Eqsl.has_sent_status());
        assert!(!UploadTarget::ClubLog.has_sent_status());
        assert!(!UploadTarget::HrdLog.has_sent_status());
    }

    #[test]
    fn target_labels_and_ids_are_distinct() {
        let all = [
            UploadTarget::Lotw,
            UploadTarget::Eqsl,
            UploadTarget::ClubLog,
            UploadTarget::Qrz,
            UploadTarget::HamQth,
            UploadTarget::Cloudlog,
            UploadTarget::HrdLog,
        ];
        let ids: std::collections::HashSet<_> = all.iter().map(|t| t.as_str()).collect();
        assert_eq!(ids.len(), all.len());
        let labels: std::collections::HashSet<_> = all.iter().map(|t| t.label()).collect();
        assert_eq!(labels.len(), all.len());
    }
}
