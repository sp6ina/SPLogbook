// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Wersjonowany kontrakt protokołu procesów pomocniczych aktualizatora Windows (M3-B).
//!
//! Zgodnie ze specyfikacją STAGE-M3-B oraz regułami bezpieczeństwa Windows:
//! - Wersjonowany rekord żądania (`WindowsHelperRequest`) jest lokalny, podpisany logicznie
//!   przez powiązanie z `UpdateAttemptId`, `UpdateTransaction` oraz `UpdatePlanIdentity`.
//! - Helper nigdy nie przyjmuje adresów URL, tokenów, poświadczeń ani kluczy prywatnych.
//! - Plan aktualizacji jest chroniony przez deterministyczny skrót SHA-256 (`plan_digest`).
//! - Ścieżki podlegają rygorystycznej walidacji: brak `..`, brak ADS, brak znaków wieloznacznych,
//!   brak nazw zarezerwowanych DOS, brak ucieczki poza katalog instalacji.
//! - Zwracane są stabilne, typowane kody wyjścia (`WindowsHelperExitCode`) oraz wersjonowany
//!   rekord wyniku (`WindowsHelperResult`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use crate::cloud::updater::UpdatePlan;
use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::attempt_id::UpdateAttemptId;
use crate::cloud::updater::installation::ManagementMode;
use crate::cloud::updater::is_protected_user_path;
use crate::cloud::updater::state::UpdateStateId;

/// Aktualna kanoniczna wersja rekordu żądania pomocnika Windows.
pub const CURRENT_HELPER_REQUEST_VERSION: u32 = 1;

/// Aktualna kanoniczna wersja rekordu wyniku pomocnika Windows.
pub const CURRENT_HELPER_RESULT_VERSION: u32 = 1;

/// Aktualna kanoniczna wersja tożsamości planu aktualizacji.
pub const CURRENT_PLAN_IDENTITY_VERSION: u32 = 1;

/// Domyślny limit czasu oczekiwania na zamknięcie procesu macierzystego w sekundach.
pub const DEFAULT_PARENT_PROCESS_TIMEOUT_SECS: u64 = 90;

/// Domyślny limit czasu oczekiwania na dowód zdrowia kandydata w sekundach.
pub const DEFAULT_CANDIDATE_HEALTH_TIMEOUT_SECS: u64 = 15;

/// Typowane kody wyjścia dla pomocnika Windows (`splogbook-updater.exe` oraz PowerShell fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum WindowsHelperExitCode {
    /// Sukces pełnego cyklu aktualizacji i weryfikacji zdrowia kandydata.
    Success = 0,
    /// Niepoprawne lub brakujące argumenty wiersza poleceń.
    InvalidArguments = 10,
    /// Nieobsługiwana wersja rekordu żądania pomocnika.
    UnsupportedRequestVersion = 11,
    /// Niepoprawny format lub tożsamość identyfikatora próby `UpdateAttemptId`.
    InvalidAttemptId = 12,
    /// Niezgodność tożsamości rekordu transakcji z żądaniem.
    TransactionMismatch = 13,
    /// Niezgodność skrótu SHA-256 planu aktualizacji (naruszenie integralności planu).
    PlanDigestMismatch = 14,
    /// Wykryto niebezpieczną ścieżkę (traversal, ADS, nazwa zarezerwowana DOS, ucieczka poza root).
    UnsafePath = 15,
    /// Upłynął limit czasu oczekiwania na zakończenie procesu macierzystego SPLogbook.
    ParentProcessTimeout = 16,
    /// Niezgodność tożsamości procesu macierzystego (np. PID został ponownie użyty przez inny proces).
    ParentProcessIdentityMismatch = 17,
    /// Błąd tworzenia kopii zapasowej podmienianych plików programu (`replace_files`).
    BackupFailed = 18,
    /// Błąd podczas podmiany plików (`apply`), rollback zakończony pełnym sukcesem.
    ApplyFailedRollbackSucceeded = 19,
    /// Błąd podczas podmiany plików (`apply`), rollback zakończony częściowo.
    ApplyFailedRollbackPartial = 20,
    /// Błąd podczas podmiany plików (`apply`), rollback nie powiódł się.
    ApplyFailedRollbackFailed = 21,
    /// Nie udało się uruchomić procesu kandydata, rollback zakończony pomyślnie.
    CandidateLaunchFailedRollbackSucceeded = 22,
    /// Upłynął limit czasu oczekiwania na dowód zdrowia kandydata, rollback zakończony pomyślnie.
    HealthTimeoutRollbackSucceeded = 23,
    /// Dowód zdrowia kandydata został odrzucony (niezgodny/stale/błędny), rollback pomyślny.
    HealthRejectedRollbackSucceeded = 24,
    /// Wykryto aktywną kolizję blokady aktualizatora (.update.lock).
    LockContended = 25,
    /// Wykryto obcą blokadę nienależącą do bieżącej próby aktualizacji.
    ForeignLock = 26,
    /// Błąd trwałego zapisu rekordu pomocnika lub dziennika operacji.
    PersistenceFailed = 27,
    /// Stan instalacji wymaga natychmiastowej ręcznej interwencji użytkownika.
    ManualInterventionRequired = 28,
    /// Wymagana cecha lub tryb zarządzania instalacją jest niedostępny.
    CapabilityUnavailable = 29,
}

impl WindowsHelperExitCode {
    /// Zwraca numeryczną wartość kodu wyjścia zgodną ze standardem procesu Windows.
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }

    /// Odtwarza kod wyjścia z wartości numerycznej.
    #[must_use]
    pub const fn from_i32(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Success),
            10 => Some(Self::InvalidArguments),
            11 => Some(Self::UnsupportedRequestVersion),
            12 => Some(Self::InvalidAttemptId),
            13 => Some(Self::TransactionMismatch),
            14 => Some(Self::PlanDigestMismatch),
            15 => Some(Self::UnsafePath),
            16 => Some(Self::ParentProcessTimeout),
            17 => Some(Self::ParentProcessIdentityMismatch),
            18 => Some(Self::BackupFailed),
            19 => Some(Self::ApplyFailedRollbackSucceeded),
            20 => Some(Self::ApplyFailedRollbackPartial),
            21 => Some(Self::ApplyFailedRollbackFailed),
            22 => Some(Self::CandidateLaunchFailedRollbackSucceeded),
            23 => Some(Self::HealthTimeoutRollbackSucceeded),
            24 => Some(Self::HealthRejectedRollbackSucceeded),
            25 => Some(Self::LockContended),
            26 => Some(Self::ForeignLock),
            27 => Some(Self::PersistenceFailed),
            28 => Some(Self::ManualInterventionRequired),
            29 => Some(Self::CapabilityUnavailable),
            _ => None,
        }
    }

    /// Sprawdza, czy kod wyjścia reprezentuje pełny sukces.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// Zwraca zwięzły opis semantyczny kodu wyjścia.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Success => "Operacja aktualizacji zakończona pełnym sukcesem",
            Self::InvalidArguments => "Niepoprawne argumenty wiersza poleceń",
            Self::UnsupportedRequestVersion => "Nieobsługiwana wersja rekordu żądania pomocnika",
            Self::InvalidAttemptId => "Niepoprawny identyfikator próby aktualizacji",
            Self::TransactionMismatch => "Niezgodność transakcji z żądaniem pomocnika",
            Self::PlanDigestMismatch => "Niezgodność skrótu SHA-256 planu aktualizacji",
            Self::UnsafePath => "Wykryto niebezpieczną ścieżkę w planie lub żądaniu",
            Self::ParentProcessTimeout => "Upłynął limit czasu oczekiwania na proces macierzysty",
            Self::ParentProcessIdentityMismatch => "Niezgodność tożsamości procesu macierzystego",
            Self::BackupFailed => "Błąd tworzenia kopii zapasowej przed podmianą",
            Self::ApplyFailedRollbackSucceeded => "Błąd podmiany plików; rollback udany",
            Self::ApplyFailedRollbackPartial => "Błąd podmiany plików; rollback częściowy",
            Self::ApplyFailedRollbackFailed => "Błąd podmiany plików; rollback nie powiódł się",
            Self::CandidateLaunchFailedRollbackSucceeded => "Błąd startu kandydata; rollback udany",
            Self::HealthTimeoutRollbackSucceeded => {
                "Limit czasu dowodu zdrowia kandydata; rollback udany"
            }
            Self::HealthRejectedRollbackSucceeded => {
                "Odrzucony dowód zdrowia kandydata; rollback udany"
            }
            Self::LockContended => "Kolizja blokady aktualizatora",
            Self::ForeignLock => "Wykryto obcą blokadę aktualizatora",
            Self::PersistenceFailed => "Błąd zapisu rekordu lub dziennika operacji",
            Self::ManualInterventionRequired => "Wymagana natychmiastowa interwencja ręczna",
            Self::CapabilityUnavailable => "Brak wymaganych możliwości platformowych",
        }
    }
}

/// Wynik procedury rollbacku wykonanej przez pomocnika.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RollbackResultKind {
    /// Pełny sukces: wszystkie pliki `create` usunięte, wszystkie `replace` przywrócone i zweryfikowane.
    Success,
    /// Częściowy rollback: część operacji przywracania nie powiodła się.
    Partial,
    /// Nieudany rollback: wystąpił błąd krytyczny podczas przywracania.
    Failed,
    /// Rollback nie był wymagany (operacja zakończona sukcesem lub błąd przed rozpoczęciem podmiany).
    None,
}

/// Wynik weryfikacji zdrowia kandydata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthResultKind {
    /// Kandydat wygenerował poprawny, zweryfikowany `HealthProof`.
    Passed,
    /// Upłynął limit czasu (15 sekund) na pojawienie się dowodu zdrowia.
    TimedOut,
    /// Dowód zdrowia istnieje, lecz nie przeszedł walidacji tożsamości, wersji lub wieku.
    Rejected,
    /// Proces kandydata zakończył się przedwcześnie awarią (crash/exit).
    Crash,
    /// Weryfikacja zdrowia nie została osiągnięta.
    None,
}

/// Kategoria wyniku końcowego pomocnika.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HelperResultCategory {
    /// Sukces pełnego cyklu aktualizacji.
    Success,
    /// Błąd w trakcie lub po podmianie; rollback powiódł się w 100%.
    FailedRollbackSucceeded,
    /// Błąd w trakcie lub po podmianie; rollback częściowy.
    FailedRollbackPartial,
    /// Błąd w trakcie lub po podmianie; rollback nie powiódł się.
    FailedRollbackFailed,
    /// Przerwanie operacji przed rozpoczęciem fazy `apply` (pliki programu nienaruszone).
    AbortedPreApply,
    /// Wymagana interwencja ręczna użytkownika.
    ManualInterventionRequired,
}

/// Deterministyczna tożsamość planu aktualizacji powiązana z unikalną próbą.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePlanIdentity {
    /// Wersja schematu tożsamości planu.
    pub plan_version: u32,
    /// Identyfikator próby aktualizacji.
    pub attempt_id: UpdateAttemptId,
    /// Znormalizowana tożsamość katalogu instalacji.
    pub installation_root_identity: String,
    /// Posortowane, znormalizowane względne ścieżki plików do podmiany (`replace`).
    pub ordered_replace_entries: Vec<String>,
    /// Posortowane, znormalizowane względne ścieżki nowych plików (`create`).
    pub ordered_create_entries: Vec<String>,
    /// Posortowane, znormalizowane względne ścieżki plików nienaruszanych (`preserve`).
    pub ordered_preserve_entries: Vec<String>,
    /// Posortowane, znormalizowane względne ścieżki plików odrzuconych (`reject`).
    pub ordered_reject_entries: Vec<String>,
    /// Nazwa pliku kandydata (np. "SPLogbook.exe").
    pub candidate_executable: String,
    /// Wersja docelowa SemVer.
    pub target_version: String,
    /// Commit docelowy Git.
    pub target_commit: String,
    /// Skrót SHA-256 zweryfikowanego assetu pobrania.
    pub asset_sha256: String,
}

impl UpdatePlanIdentity {
    /// Tworzy tożsamość planu z istniejącego `UpdatePlan` oraz parametrów transakcji.
    pub fn from_plan(
        attempt_id: UpdateAttemptId,
        installation_root: &Path,
        plan: &UpdatePlan,
        candidate_name: &str,
        target_version: &str,
        target_commit: &str,
        asset_sha256: &str,
    ) -> Result<Self, UpdaterError> {
        let root_identity = normalize_root_identity(installation_root);

        let mut replace_entries: Vec<String> = Vec::with_capacity(plan.replace_files.len());
        for (rel, _) in &plan.replace_files {
            let norm = validate_and_normalize_relative_path(rel)?;
            replace_entries.push(norm);
        }
        replace_entries.sort();
        replace_entries.dedup();

        let mut create_entries: Vec<String> = Vec::with_capacity(plan.create_files.len());
        for (rel, _) in &plan.create_files {
            let norm = validate_and_normalize_relative_path(rel)?;
            create_entries.push(norm);
        }
        create_entries.sort();
        create_entries.dedup();

        let mut preserve_entries: Vec<String> = Vec::with_capacity(plan.preserve_files.len());
        for (abs, _) in &plan.preserve_files {
            let rel = abs.strip_prefix(installation_root).map_err(|_| {
                UpdaterError::SafetyGate(format!(
                    "Plik preserve '{}' leży poza installation root",
                    abs.display()
                ))
            })?;
            let norm = validate_and_normalize_relative_path(rel)?;
            preserve_entries.push(norm);
        }
        preserve_entries.sort();
        preserve_entries.dedup();

        let mut reject_entries: Vec<String> = Vec::with_capacity(plan.rejected_files.len());
        for (rel, _) in &plan.rejected_files {
            let norm = validate_and_normalize_relative_path(rel)?;
            reject_entries.push(norm);
        }
        reject_entries.sort();
        reject_entries.dedup();

        let norm_candidate = validate_and_normalize_relative_path(Path::new(candidate_name))?;

        Ok(Self {
            plan_version: CURRENT_PLAN_IDENTITY_VERSION,
            attempt_id,
            installation_root_identity: root_identity,
            ordered_replace_entries: replace_entries,
            ordered_create_entries: create_entries,
            ordered_preserve_entries: preserve_entries,
            ordered_reject_entries: reject_entries,
            candidate_executable: norm_candidate,
            target_version: target_version.trim().to_string(),
            target_commit: target_commit.trim().to_string(),
            asset_sha256: asset_sha256.trim().to_ascii_lowercase(),
        })
    }

    /// Oblicza kryptograficzny skrót SHA-256 z kanonicznej postaci tożsamości planu.
    #[must_use]
    pub fn compute_digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"UpdatePlanIdentity:v1\n");
        hasher.update(format!("attempt_id:{}\n", self.attempt_id).as_bytes());
        hasher.update(format!("plan_version:{}\n", self.plan_version).as_bytes());
        hasher.update(format!("root:{}\n", self.installation_root_identity).as_bytes());
        hasher.update(format!("candidate:{}\n", self.candidate_executable).as_bytes());
        hasher.update(format!("version:{}\n", self.target_version).as_bytes());
        hasher.update(format!("commit:{}\n", self.target_commit).as_bytes());
        hasher.update(format!("asset_sha256:{}\n", self.asset_sha256).as_bytes());

        hasher.update(b"replace_files:\n");
        for entry in &self.ordered_replace_entries {
            hasher.update(format!("  {entry}\n").as_bytes());
        }

        hasher.update(b"create_files:\n");
        for entry in &self.ordered_create_entries {
            hasher.update(format!("  {entry}\n").as_bytes());
        }

        hasher.update(b"preserve_files:\n");
        for entry in &self.ordered_preserve_entries {
            hasher.update(format!("  {entry}\n").as_bytes());
        }

        hasher.update(b"reject_files:\n");
        for entry in &self.ordered_reject_entries {
            hasher.update(format!("  {entry}\n").as_bytes());
        }

        let digest = hasher.finalize();
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    }

    /// Weryfikuje tożsamość planu względem przekazanego `UpdatePlan`.
    pub fn validate_against_plan(
        &self,
        plan: &UpdatePlan,
        installation_root: &Path,
    ) -> Result<(), UpdaterError> {
        let root_identity = normalize_root_identity(installation_root);
        if self.installation_root_identity != root_identity {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność tożsamości katalogu instalacji: oczekiwano '{}', otrzymano '{}'",
                self.installation_root_identity, root_identity
            )));
        }

        // Sprawdzamy, czy w planie nie ma nieoczekiwanych plików odrzuconych
        if !self.ordered_reject_entries.is_empty() {
            return Err(UpdaterError::SafetyGate(format!(
                "Plan zawiera {} odrzuconych plików (odrzucenie fail-closed)",
                self.ordered_reject_entries.len()
            )));
        }

        // Weryfikacja liczności
        if plan.replace_files.len() != self.ordered_replace_entries.len() {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność liczby pozycji replace: plan ma {}, tożsamość ma {}",
                plan.replace_files.len(),
                self.ordered_replace_entries.len()
            )));
        }
        if plan.create_files.len() != self.ordered_create_entries.len() {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność liczby pozycji create: plan ma {}, tożsamość ma {}",
                plan.create_files.len(),
                self.ordered_create_entries.len()
            )));
        }

        // Sprawdzamy każdą pozycję replace
        for (rel, _) in &plan.replace_files {
            let norm = validate_and_normalize_relative_path(rel)?;
            if !self.ordered_replace_entries.contains(&norm) {
                return Err(UpdaterError::SafetyGate(format!(
                    "Plik replace '{norm}' nie występuje w tożsamości planu"
                )));
            }
        }

        // Sprawdzamy każdą pozycję create
        for (rel, _) in &plan.create_files {
            let norm = validate_and_normalize_relative_path(rel)?;
            if !self.ordered_create_entries.contains(&norm) {
                return Err(UpdaterError::SafetyGate(format!(
                    "Plik create '{norm}' nie występuje w tożsamości planu"
                )));
            }
        }

        Ok(())
    }
}

/// Wersjonowany kontrakt żądania uruchomienia pomocnika Windows (`splogbook-updater.exe` / fallback PowerShell).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsHelperRequest {
    /// Wersja schematu rekordu żądania.
    pub request_version: u32,
    /// Unikalny identyfikator próby aktualizacji (UUID v4).
    pub attempt_id: UpdateAttemptId,
    /// Bezwzględna ścieżka do trwałego rekordu transakcji `.update_transaction.json`.
    pub transaction_record_path: PathBuf,
    /// Bezwzględna ścieżka do katalogu instalacji aplikacji (installation root).
    pub installation_root: PathBuf,
    /// Bezwzględna ścieżka do bieżącego pliku wykonywalnego (`SPLogbook.exe`).
    pub current_executable: PathBuf,
    /// Bezwzględna ścieżka do kandydata aktualizacji w stagingu.
    pub candidate_executable: PathBuf,
    /// Oczekiwany identyfikator PID procesu macierzystego SPLogbook, na którego zamknięcie czeka helper.
    pub expected_parent_pid: u32,
    /// Opcjonalna tożsamość bieżącej binarki (np. skrót SHA-256 lub rozmiar) dla weryfikacji tożsamości.
    pub expected_current_binary_identity: Option<String>,
    /// Oczekiwana docelowa wersja SemVer.
    pub expected_target_version: String,
    /// Oczekiwany docelowy commit Git.
    pub expected_target_commit: String,
    /// Tryb zarządzania instalacją (np. SelfManaged).
    pub update_management_mode: ManagementMode,
    /// Tożsamość zatwierdzonego planu aktualizacji.
    pub update_plan_identity: UpdatePlanIdentity,
    /// Oczekiwany skrót SHA-256 planu aktualizacji.
    pub plan_digest: String,
    /// Bezwzględna ścieżka do pliku dowodu zdrowia kandydata (`.health_ok`).
    pub health_proof_path: PathBuf,
    /// Limit czasu oczekiwania na zamknięcie procesu macierzystego w sekundach.
    pub parent_process_timeout_seconds: u64,
    /// Limit czasu oczekiwania na dowód zdrowia kandydata w sekundach.
    pub candidate_health_timeout_seconds: u64,
    /// Ścieżka docelowa trwałego zapisu wyniku pomocnika (`.update_helper_result.json`).
    pub result_record_path: PathBuf,
    /// Ścieżka docelowa lokalnego zredagowanego logu diagnostycznego pomocnika.
    pub diagnostic_log_path: PathBuf,
}

impl WindowsHelperRequest {
    /// Dokonuje pełnej walidacji spójności i bezpieczeństwa żądania przed wykonaniem jakichkolwiek operacji I/O.
    pub fn validate(&self) -> Result<(), UpdaterError> {
        // 1. Weryfikacja wersji rekordu
        if self.request_version != CURRENT_HELPER_REQUEST_VERSION {
            return Err(UpdaterError::SafetyGate(format!(
                "Nieobsługiwana wersja rekordu żądania pomocnika: {} (oczekiwano {})",
                self.request_version, CURRENT_HELPER_REQUEST_VERSION
            )));
        }

        // 2. Weryfikacja spójności próby
        if self.attempt_id != self.update_plan_identity.attempt_id {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność attempt_id w żądaniu ({}) i w tożsamości planu ({})",
                self.attempt_id, self.update_plan_identity.attempt_id
            )));
        }

        // 3. Weryfikacja skrótu planu
        let computed_digest = self.update_plan_identity.compute_digest();
        if self.plan_digest != computed_digest {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność plan_digest w żądaniu: oczekiwano '{}', tożsamość planu daje '{}'",
                self.plan_digest, computed_digest
            )));
        }

        // 4. Weryfikacja parametrów procesu
        if self.expected_parent_pid == 0 {
            return Err(UpdaterError::SafetyGate(
                "Identyfikator PID procesu macierzystego nie może być równy 0".to_string(),
            ));
        }

        // 5. Weryfikacja limitów czasu
        if self.parent_process_timeout_seconds == 0 || self.parent_process_timeout_seconds > 600 {
            return Err(UpdaterError::SafetyGate(format!(
                "Limit czasu procesu macierzystego poza zakresem (1..600s): {}s",
                self.parent_process_timeout_seconds
            )));
        }
        if self.candidate_health_timeout_seconds == 0 || self.candidate_health_timeout_seconds > 120
        {
            return Err(UpdaterError::SafetyGate(format!(
                "Limit czasu testu zdrowia kandydata poza zakresem (1..120s): {}s",
                self.candidate_health_timeout_seconds
            )));
        }

        // 6. Weryfikacja ścieżki installation root
        validate_root_path(&self.installation_root)?;

        // 7. Weryfikacja containment wszystkich powiązanych ścieżek
        ensure_path_inside_root(&self.installation_root, &self.transaction_record_path)?;
        ensure_path_inside_root(&self.installation_root, &self.current_executable)?;
        ensure_path_inside_root(&self.installation_root, &self.candidate_executable)?;
        ensure_path_inside_root(&self.installation_root, &self.health_proof_path)?;
        ensure_path_inside_root(&self.installation_root, &self.result_record_path)?;
        ensure_path_inside_root(&self.installation_root, &self.diagnostic_log_path)?;

        // 8. Ochrona danych użytkownika: żadna ścieżka planu nie może być chronionym plikiem użytkownika
        for entry in &self.update_plan_identity.ordered_replace_entries {
            let p = Path::new(entry);
            if is_protected_user_path(p) {
                return Err(UpdaterError::SafetyGate(format!(
                    "Ścieżka replace '{entry}' narusza ochronę danych użytkownika"
                )));
            }
        }
        for entry in &self.update_plan_identity.ordered_create_entries {
            let p = Path::new(entry);
            if is_protected_user_path(p) {
                return Err(UpdaterError::SafetyGate(format!(
                    "Ścieżka create '{entry}' narusza ochronę danych użytkownika"
                )));
            }
        }

        Ok(())
    }

    /// Zapisuje żądanie atomowo do pliku JSON z zachowaniem gwarancji crash-safe.
    pub fn save_atomic(&self, path: &Path) -> Result<(), UpdaterError> {
        let parent = path.parent().ok_or_else(|| {
            UpdaterError::Io("Brak katalogu nadrzędnego dla pliku żądania".to_string())
        })?;

        let json = serde_json::to_string_pretty(self).map_err(|e| {
            UpdaterError::Io(format!("Błąd serializacji WindowsHelperRequest: {e}"))
        })?;

        let temp_filename = format!(
            ".helper_request_{}_{}.tmp",
            self.attempt_id.as_file_component(),
            std::process::id()
        );
        let temp_path = parent.join(temp_filename);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| {
                UpdaterError::Io(format!(
                    "Nie można utworzyć pliku tymczasowego żądania {}: {e}",
                    temp_path.display()
                ))
            })?;

        file.write_all(json.as_bytes()).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!("Błąd zapisu danych żądania pomocnika: {e}"))
        })?;

        file.sync_all().map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!("Błąd synchronizacji pliku żądania pomocnika: {e}"))
        })?;

        drop(file);

        fs::rename(&temp_path, path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!(
                "Błąd atomowego przeniesienia pliku żądania pomocnika do {}: {e}",
                path.display()
            ))
        })?;

        Ok(())
    }

    /// Odczytuje i waliduje żądanie z pliku JSON.
    pub fn load_from_path(path: &Path) -> Result<Self, UpdaterError> {
        if !path.exists() {
            return Err(UpdaterError::Io(format!(
                "Plik żądania pomocnika nie istnieje: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(path).map_err(|e| {
            UpdaterError::Io(format!(
                "Nie można odczytać pliku żądania pomocnika {}: {e}",
                path.display()
            ))
        })?;

        let req: Self = serde_json::from_str(&content).map_err(|e| {
            UpdaterError::Io(format!(
                "Błąd deserializacji rekordu żądania pomocnika {}: {e}",
                path.display()
            ))
        })?;

        req.validate()?;
        Ok(req)
    }
}

/// Wersjonowany kontrakt wyniku działania pomocnika Windows (`splogbook-updater.exe` / fallback PowerShell).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsHelperResult {
    /// Wersja schematu rekordu wyniku.
    pub result_version: u32,
    /// Unikalny identyfikator próby aktualizacji.
    pub attempt_id: UpdateAttemptId,
    /// Skrót SHA-256 zatwierdzonego planu aktualizacji.
    pub plan_digest: String,
    /// Stan końcowy automatu transakcji (np. Committed, RollbackVerified).
    pub final_update_state: UpdateStateId,
    /// Identyfikator silnika pomocnika ("NativeRust" lub "PowerShellFallback").
    pub helper_engine: String,
    /// Kategoria wyniku pomocnika.
    pub result_category: HelperResultCategory,
    /// Numeryczny kod wyjścia (zgodny z `WindowsHelperExitCode`).
    pub exit_code: i32,
    /// Wynik procedury rollbacku.
    pub rollback_result: Option<RollbackResultKind>,
    /// Wynik testu zdrowia kandydata.
    pub health_result: Option<HealthResultKind>,
    /// Liczba pomyślnie wykonanych operacji na plikach.
    pub completed_operation_count: usize,
    /// Całkowita liczba zaplanowanych operacji na plikach.
    pub total_operation_count: usize,
    /// Czy kopia zapasowa `.rollback` została zachowana na dysku.
    pub backup_retained: bool,
    /// Czy stan instalacji wymaga interwencji ręcznej.
    pub manual_recovery_required: bool,
    /// Zredagowane podsumowanie diagnostyczne bez sekretów i danych użytkownika.
    pub sanitized_diagnostic_summary: String,
    /// Znacznik czasu zakończenia działania pomocnika w UTC.
    pub completion_time: DateTime<Utc>,
}

impl WindowsHelperResult {
    /// Tworzy nowy rekord wyniku z bieżącym czasem zakończenia.
    pub fn new(
        attempt_id: UpdateAttemptId,
        plan_digest: String,
        final_update_state: UpdateStateId,
        helper_engine: impl Into<String>,
        result_category: HelperResultCategory,
        exit_code: WindowsHelperExitCode,
        rollback_result: Option<RollbackResultKind>,
        health_result: Option<HealthResultKind>,
        completed_operation_count: usize,
        total_operation_count: usize,
        backup_retained: bool,
        manual_recovery_required: bool,
        sanitized_diagnostic_summary: impl Into<String>,
    ) -> Self {
        Self {
            result_version: CURRENT_HELPER_RESULT_VERSION,
            attempt_id,
            plan_digest,
            final_update_state,
            helper_engine: helper_engine.into(),
            result_category,
            exit_code: exit_code.as_i32(),
            rollback_result,
            health_result,
            completed_operation_count,
            total_operation_count,
            backup_retained,
            manual_recovery_required,
            sanitized_diagnostic_summary: sanitized_diagnostic_summary.into(),
            completion_time: Utc::now(),
        }
    }

    /// Dokonuje walidacji spójności rekordu wyniku względem parametrów żądania.
    pub fn validate(
        &self,
        expected_attempt: &UpdateAttemptId,
        expected_plan_digest: &str,
    ) -> Result<(), UpdaterError> {
        if self.result_version != CURRENT_HELPER_RESULT_VERSION {
            return Err(UpdaterError::SafetyGate(format!(
                "Nieobsługiwana wersja rekordu wyniku pomocnika: {} (oczekiwano {})",
                self.result_version, CURRENT_HELPER_RESULT_VERSION
            )));
        }

        if self.attempt_id != *expected_attempt {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność attempt_id w wyniku pomocnika: oczekiwano '{expected_attempt}', otrzymano '{}'",
                self.attempt_id
            )));
        }

        if self.plan_digest != expected_plan_digest {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność plan_digest w wyniku pomocnika: oczekiwano '{}', otrzymano '{}'",
                expected_plan_digest, self.plan_digest
            )));
        }

        // Weryfikacja spójności kodu wyjścia i kategorii wyniku
        if self.exit_code == WindowsHelperExitCode::Success.as_i32() {
            if self.result_category != HelperResultCategory::Success {
                return Err(UpdaterError::SafetyGate(
                    "Kod wyjścia 0 wymaga kategorii wyniku Success".to_string(),
                ));
            }
            if self.final_update_state != UpdateStateId::Committed
                && self.final_update_state != UpdateStateId::Cleanup
            {
                return Err(UpdaterError::SafetyGate(format!(
                    "Sukces wymaga stanu Committed lub Cleanup, otrzymano {:?}",
                    self.final_update_state
                )));
            }
        }

        Ok(())
    }

    /// Zapisuje wynik pomocnika atomowo z gwarancją crash-safe.
    pub fn save_atomic(&self, path: &Path) -> Result<(), UpdaterError> {
        let parent = path.parent().ok_or_else(|| {
            UpdaterError::Io("Brak katalogu nadrzędnego dla rekordu wyniku pomocnika".to_string())
        })?;

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| UpdaterError::Io(format!("Błąd serializacji WindowsHelperResult: {e}")))?;

        let temp_filename = format!(
            ".helper_result_{}_{}.tmp",
            self.attempt_id.as_file_component(),
            std::process::id()
        );
        let temp_path = parent.join(temp_filename);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| {
                UpdaterError::Io(format!(
                    "Nie można utworzyć pliku tymczasowego wyniku {}: {e}",
                    temp_path.display()
                ))
            })?;

        file.write_all(json.as_bytes()).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!("Błąd zapisu rekordu wyniku pomocnika: {e}"))
        })?;

        file.sync_all().map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!(
                "Błąd synchronizacji pliku rekordu wyniku pomocnika: {e}"
            ))
        })?;

        drop(file);

        fs::rename(&temp_path, path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!(
                "Błąd atomowego przeniesienia rekordu wyniku pomocnika do {}: {e}",
                path.display()
            ))
        })?;

        Ok(())
    }

    /// Odczytuje rekord wyniku z pliku.
    pub fn load_from_path(path: &Path) -> Result<Self, UpdaterError> {
        if !path.exists() {
            return Err(UpdaterError::Io(format!(
                "Plik rekordu wyniku pomocnika nie istnieje: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(path).map_err(|e| {
            UpdaterError::Io(format!(
                "Nie można odczytać pliku wyniku pomocnika {}: {e}",
                path.display()
            ))
        })?;

        let res: Self = serde_json::from_str(&content).map_err(|e| {
            UpdaterError::Io(format!(
                "Błąd deserializacji rekordu wyniku pomocnika {}: {e}",
                path.display()
            ))
        })?;

        Ok(res)
    }
}

// ============================================================================
// Funkcje pomocnicze walidacji i normalizacji ścieżek
// ============================================================================

/// Normalizuje identyfikator katalogu bazowego dla deterministycznego skrótu planu.
#[must_use]
pub fn normalize_root_identity(root: &Path) -> String {
    let s = root.to_string_lossy().replace('\\', "/");
    let trimmed = s.trim_end_matches('/');
    trimmed.to_ascii_lowercase()
}

/// Dokonuje rygorystycznej walidacji i normalizacji ścieżki względnej.
///
/// Odrzuca:
/// - ścieżki bezwzględne;
/// - komponenty `..` oraz `.`;
/// - alternatywne strumienie danych NTFS (Alternate Data Streams `:` );
/// - znaki wieloznaczne (`*`, `?`);
/// - znaki kontrolne oraz NUL;
/// - nazwy zarezerwowane Windows DOS (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9`);
/// - dwukropki, separatory dyskowe, prefiksy `\\?\`;
/// - spacje lub kropki na końcu nazwy komponentu (ambiguity Windows).
pub fn validate_and_normalize_relative_path(path: &Path) -> Result<String, UpdaterError> {
    let raw = path.to_string_lossy();
    if raw.is_empty() {
        return Err(UpdaterError::SafetyGate(
            "Ścieżka względna w planie nie może być pusta".to_string(),
        ));
    }

    // Blokada znaków NUL
    if raw.contains('\0') {
        return Err(UpdaterError::SafetyGate(
            "Wykryto znak NUL w ścieżce planu".to_string(),
        ));
    }

    // Blokada znaków wieloznacznych
    if raw.contains('*') || raw.contains('?') {
        return Err(UpdaterError::SafetyGate(format!(
            "Zabronione znaki wieloznaczne w ścieżce planu: {raw}"
        )));
    }

    // Blokada alternatywnych strumieni danych NTFS (ADS) oraz prefiksów dyskowych
    if raw.contains(':') {
        return Err(UpdaterError::SafetyGate(format!(
            "Wykryto zabroniony dwukropek (ADS lub litera dysku) w ścieżce planu: {raw}"
        )));
    }

    let mut normalized_parts: Vec<String> = Vec::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part_str = part.to_string_lossy();

                // Sprawdzamy zarezerwowane nazwy DOS
                let stem = part_str
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .to_ascii_uppercase();
                if matches!(
                    stem.as_str(),
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                ) {
                    return Err(UpdaterError::SafetyGate(format!(
                        "Zarezerwowana nazwa urządzenia DOS w ścieżce planu: {part_str}"
                    )));
                }

                // Odrzucenie kończących kropek i spacji (Windows trailing dot/space ambiguity)
                if part_str.ends_with('.') || part_str.ends_with(' ') {
                    return Err(UpdaterError::SafetyGate(format!(
                        "Niejednoznaczna nazwa z kropką lub spacją na końcu: '{part_str}'"
                    )));
                }

                normalized_parts.push(part_str.to_string());
            }
            _ => {
                return Err(UpdaterError::SafetyGate(format!(
                    "Niedozwolony komponent ścieżki w planie ({component:?}): {raw}"
                )));
            }
        }
    }

    if normalized_parts.is_empty() {
        return Err(UpdaterError::SafetyGate(
            "Ścieżka zredukowana do pustej po normalizacji".to_string(),
        ));
    }

    // Łączymy kanonicznie przez standardowy separator ukośnika `/`
    Ok(normalized_parts.join("/"))
}

/// Sprawdza, czy ścieżka installation root jest poprawna i bezpieczna.
fn validate_root_path(root: &Path) -> Result<(), UpdaterError> {
    if !root.is_absolute() {
        return Err(UpdaterError::SafetyGate(format!(
            "Installation root musi być ścieżką bezwzględną: {}",
            root.display()
        )));
    }

    let root_str = root.to_string_lossy();
    if root_str.contains('\0') {
        return Err(UpdaterError::SafetyGate(
            "Installation root zawiera znak NUL".to_string(),
        ));
    }

    // Blokada niebezpiecznych ścieżek sieciowych UNC, o ile nie są jawnie zamierzone
    if root_str.starts_with(r"\\") || root_str.starts_with("//") {
        return Err(UpdaterError::SafetyGate(format!(
            "Installation root na ścieżce sieciowej UNC jest zabroniony: {}",
            root.display()
        )));
    }

    Ok(())
}

/// Weryfikuje, czy `target` znajduje się ściśle wewnątrz `root`.
fn ensure_path_inside_root(root: &Path, target: &Path) -> Result<(), UpdaterError> {
    if !target.is_absolute() {
        return Err(UpdaterError::SafetyGate(format!(
            "Ścieżka docelowa musi być bezwzględna: {}",
            target.display()
        )));
    }

    // Sprawdzamy brak komponentów nadrzędnych
    for c in target.components() {
        if matches!(c, Component::ParentDir) {
            return Err(UpdaterError::SafetyGate(format!(
                "Ścieżka docelowa zawiera komponent nadrzędny '..': {}",
                target.display()
            )));
        }
    }

    let norm_root = normalize_root_identity(root);
    let norm_target = normalize_root_identity(target);

    if norm_target != norm_root && !norm_target.starts_with(&format!("{norm_root}/")) {
        return Err(UpdaterError::SafetyGate(format!(
            "Ścieżka '{}' wykracza poza katalog instalacji '{}'",
            target.display(),
            root.display()
        )));
    }

    Ok(())
}

// ============================================================================
// Testy jednostkowe protokołu
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_attempt_id() -> UpdateAttemptId {
        UpdateAttemptId::generate()
    }

    fn make_dummy_plan(root: &Path) -> UpdatePlan {
        UpdatePlan {
            replace_files: vec![
                (PathBuf::from("SPLogbook.exe"), root.join("SPLogbook.exe")),
                (PathBuf::from("lib.dll"), root.join("lib.dll")),
            ],
            create_files: vec![(
                PathBuf::from("resources/new.dat"),
                root.join("resources/new.dat"),
            )],
            preserve_files: vec![(root.join("databases/serviceLOG.db"), "seed".to_string())],
            rejected_files: Vec::new(),
        }
    }

    #[test]
    fn exit_codes_round_trip_and_no_collisions() {
        let mut seen = std::collections::HashSet::new();
        let all_codes = [
            WindowsHelperExitCode::Success,
            WindowsHelperExitCode::InvalidArguments,
            WindowsHelperExitCode::UnsupportedRequestVersion,
            WindowsHelperExitCode::InvalidAttemptId,
            WindowsHelperExitCode::TransactionMismatch,
            WindowsHelperExitCode::PlanDigestMismatch,
            WindowsHelperExitCode::UnsafePath,
            WindowsHelperExitCode::ParentProcessTimeout,
            WindowsHelperExitCode::ParentProcessIdentityMismatch,
            WindowsHelperExitCode::BackupFailed,
            WindowsHelperExitCode::ApplyFailedRollbackSucceeded,
            WindowsHelperExitCode::ApplyFailedRollbackPartial,
            WindowsHelperExitCode::ApplyFailedRollbackFailed,
            WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded,
            WindowsHelperExitCode::HealthTimeoutRollbackSucceeded,
            WindowsHelperExitCode::HealthRejectedRollbackSucceeded,
            WindowsHelperExitCode::LockContended,
            WindowsHelperExitCode::ForeignLock,
            WindowsHelperExitCode::PersistenceFailed,
            WindowsHelperExitCode::ManualInterventionRequired,
            WindowsHelperExitCode::CapabilityUnavailable,
        ];

        for code in all_codes {
            let num = code.as_i32();
            assert!(seen.insert(num), "Kolizja numeryczna kodu wyjścia: {num}");
            let parsed = WindowsHelperExitCode::from_i32(num);
            assert_eq!(parsed, Some(code));
            assert_ne!(code.description(), "");
        }

        assert!(WindowsHelperExitCode::Success.is_success());
        assert!(!WindowsHelperExitCode::BackupFailed.is_success());
    }

    #[test]
    fn relative_path_validator_accepts_clean_paths() {
        assert_eq!(
            validate_and_normalize_relative_path(Path::new("SPLogbook.exe")).unwrap(),
            "SPLogbook.exe"
        );
        assert_eq!(
            validate_and_normalize_relative_path(Path::new("sub/dir/file.dll")).unwrap(),
            "sub/dir/file.dll"
        );
        assert_eq!(
            validate_and_normalize_relative_path(Path::new(r"sub\dir\file.dll")).unwrap(),
            "sub/dir/file.dll"
        );
    }

    #[test]
    fn relative_path_validator_rejects_dangerous_paths() {
        assert!(validate_and_normalize_relative_path(Path::new("")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("..")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("../escape.dll")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("sub/../../escape.dll")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("/abs/path.dll")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("C:/abs/path.dll")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("file:stream")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("wildcard*.dll")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("wildcard?.dll")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("NUL")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("CON.txt")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("aux.dat")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("trailing_dot.")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("trailing_space ")).is_err());
        assert!(validate_and_normalize_relative_path(Path::new("null\0byte")).is_err());
    }

    #[test]
    fn update_plan_identity_is_deterministic() {
        let attempt = make_test_attempt_id();
        let root = Path::new("C:/test");
        let plan = make_dummy_plan(root);

        let identity1 = UpdatePlanIdentity::from_plan(
            attempt,
            root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "abcdef123456",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

        let identity2 = UpdatePlanIdentity::from_plan(
            attempt,
            root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "abcdef123456",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

        assert_eq!(identity1.compute_digest(), identity2.compute_digest());

        // Modyfikacja asset_sha256 zmienia digest
        let mut tampered = identity1.clone();
        tampered.asset_sha256 =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
        assert_ne!(identity1.compute_digest(), tampered.compute_digest());

        // Modyfikacja wpisów replace zmienia digest
        let mut tampered2 = identity1.clone();
        tampered2
            .ordered_replace_entries
            .push("extra.dll".to_string());
        assert_ne!(identity1.compute_digest(), tampered2.compute_digest());
    }

    #[test]
    fn helper_request_atomic_roundtrip() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let attempt = make_test_attempt_id();

        let plan = make_dummy_plan(&root);
        let plan_identity = UpdatePlanIdentity::from_plan(
            attempt,
            &root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "commit123",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap();
        let plan_digest = plan_identity.compute_digest();

        let req_path = root.join("test_request.json");
        let req = WindowsHelperRequest {
            request_version: CURRENT_HELPER_REQUEST_VERSION,
            attempt_id: attempt,
            transaction_record_path: root.join(".update_transaction.json"),
            installation_root: root.clone(),
            current_executable: root.join("SPLogbook.exe"),
            candidate_executable: root.join(".staging/SPLogbook.exe"),
            expected_parent_pid: 12345,
            expected_current_binary_identity: Some("old_sha".to_string()),
            expected_target_version: "1.2.1".to_string(),
            expected_target_commit: "commit123".to_string(),
            update_management_mode: ManagementMode::SelfManaged,
            update_plan_identity: plan_identity,
            plan_digest,
            health_proof_path: root.join(".health_ok"),
            parent_process_timeout_seconds: 90,
            candidate_health_timeout_seconds: 15,
            result_record_path: root.join(".update_helper_result.json"),
            diagnostic_log_path: root.join("update_helper.log"),
        };

        req.save_atomic(&req_path).unwrap();
        assert!(req_path.exists());

        let loaded = WindowsHelperRequest::load_from_path(&req_path).unwrap();
        assert_eq!(req, loaded);
    }

    #[test]
    fn helper_request_rejects_plan_digest_tampering() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let attempt = make_test_attempt_id();

        let plan = make_dummy_plan(&root);
        let plan_identity = UpdatePlanIdentity::from_plan(
            attempt,
            &root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "commit123",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap();

        let req = WindowsHelperRequest {
            request_version: CURRENT_HELPER_REQUEST_VERSION,
            attempt_id: attempt,
            transaction_record_path: root.join(".update_transaction.json"),
            installation_root: root.clone(),
            current_executable: root.join("SPLogbook.exe"),
            candidate_executable: root.join(".staging/SPLogbook.exe"),
            expected_parent_pid: 12345,
            expected_current_binary_identity: None,
            expected_target_version: "1.2.1".to_string(),
            expected_target_commit: "commit123".to_string(),
            update_management_mode: ManagementMode::SelfManaged,
            update_plan_identity: plan_identity,
            plan_digest: "bad_digest_00000000000000000000000000000000".to_string(),
            health_proof_path: root.join(".health_ok"),
            parent_process_timeout_seconds: 90,
            candidate_health_timeout_seconds: 15,
            result_record_path: root.join(".update_helper_result.json"),
            diagnostic_log_path: root.join("update_helper.log"),
        };

        assert!(req.validate().is_err());
    }

    #[test]
    fn helper_result_atomic_roundtrip() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let attempt = make_test_attempt_id();
        let res_path = root.join("test_result.json");

        let res = WindowsHelperResult::new(
            attempt,
            "valid_plan_digest_123".to_string(),
            UpdateStateId::Committed,
            "NativeRust",
            HelperResultCategory::Success,
            WindowsHelperExitCode::Success,
            Some(RollbackResultKind::None),
            Some(HealthResultKind::Passed),
            5,
            5,
            true,
            false,
            "Wszystkie operacje zakonczone sukcesem",
        );

        res.save_atomic(&res_path).unwrap();
        assert!(res_path.exists());

        let loaded = WindowsHelperResult::load_from_path(&res_path).unwrap();
        assert_eq!(res.attempt_id, loaded.attempt_id);
        assert_eq!(res.plan_digest, loaded.plan_digest);
        assert_eq!(res.exit_code, loaded.exit_code);
        assert!(loaded.validate(&attempt, "valid_plan_digest_123").is_ok());
    }
}
