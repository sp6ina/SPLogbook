// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Silnik wykonawczy pomocnika aktualizacji Windows (M3-B).
//!
//! Realizuje deterministyczną procedurę aktualizacji dla natywnego pomocnika Rust:
//! 1. Walidacja żądania `WindowsHelperRequest` i tożsamości planu;
//! 2. Bezpieczne oczekiwanie na zakończenie procesu macierzystego SPLogbook;
//! 3. Kopia zapasowa (`backup`) wyłącznie dla plików `replace_files`;
//! 4. Prowadzenie dziennika operacji (`HelperOperationJournal`);
//! 5. Zastosowanie zmian (`apply`) w trybie pojedynczych, literalnych operacji;
//! 6. Uruchomienie kandydata bez powłoki (no-shell) z parametrami `--check-health-startup` oraz `--update-attempt-id`;
//! 7. Weryfikacja dowodu zdrowia `HealthProof`;
//! 8. Deterministyczny i idempotentny `rollback` w razie niepowodzenia;
//! 9. Sprzątanie (`cleanup`) artefaktów z zachowaniem kopii zapasowej do czasu decyzji retencyjnej;
//! 10. Zapis wersjonowanego rekordu wyniku `WindowsHelperResult` i typowany kod wyjścia.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::attempt_id::UpdateAttemptId;
use crate::cloud::updater::health::{DEFAULT_HEALTH_PROOF_MAX_AGE_SECS, HealthProof};
use crate::cloud::updater::helper_protocol::{
    HealthResultKind, HelperResultCategory, RollbackResultKind, WindowsHelperExitCode,
    WindowsHelperRequest, WindowsHelperResult,
};
use crate::cloud::updater::is_protected_user_path;
use crate::cloud::updater::sha256_hex;
use crate::cloud::updater::state::UpdateStateId;
use crate::cloud::updater::transaction::UpdateTransaction;

/// Nazwa podkatalogu kopii zapasowej wewnątrz installation root.
pub const ROLLBACK_DIR_NAME: &str = ".rollback";

/// Nazwa podkatalogu stagingu wewnątrz installation root.
pub const STAGING_DIR_NAME: &str = ".staging";

/// Nazwa pliku dziennika operacji pomocnika.
pub const OPERATION_JOURNAL_FILENAME: &str = ".update_journal.json";

/// Typ operacji na plikach w planie aktualizacji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    /// Podmiana istniejącego pliku programu.
    Replace,
    /// Utworzenie nowego pliku programu / zasobu.
    Create,
}

/// Wpis w dzienniku operacji pomocnika.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationJournalEntry {
    /// Indeks kolejności operacji.
    pub index: usize,
    /// Rodzaj operacji.
    pub op_type: OperationType,
    /// Znormalizowana ścieżka względna pliku.
    pub relative_path: String,
    /// Czy utworzono kopię zapasową pliku oryginalnego.
    pub backup_performed: bool,
    /// Suma kontrolna SHA-256 kopii zapasowej (jeśli dotyczy).
    pub backup_sha256: Option<String>,
    /// Czy operacja podmiany/utworzenia została pomyślnie wykonana.
    pub apply_performed: bool,
    /// Czy operacja została wycofana w ramach procedury rollbacku.
    pub rollback_performed: bool,
    /// Znacznik czasu operacji.
    pub timestamp: chrono::DateTime<Utc>,
}

/// Dziennik operacji pomocnika umożliwiający precyzyjny i idempotentny rollback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelperOperationJournal {
    /// Identyfikator próby aktualizacji.
    pub attempt_id: UpdateAttemptId,
    /// Skrót SHA-256 planu aktualizacji.
    pub plan_digest: String,
    /// Zarejestrowane operacje w kolejności wykonania.
    pub entries: Vec<OperationJournalEntry>,
}

impl HelperOperationJournal {
    /// Tworzy nowy pusty dziennik powiązany z próbą.
    #[must_use]
    pub fn new(attempt_id: UpdateAttemptId, plan_digest: String) -> Self {
        Self {
            attempt_id,
            plan_digest,
            entries: Vec::new(),
        }
    }

    /// Rejestruje wykonanie kopii zapasowej pliku do podmienienia.
    pub fn record_backup(&mut self, rel_path: &str, sha: &str) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.relative_path == rel_path)
        {
            existing.backup_performed = true;
            existing.backup_sha256 = Some(sha.to_string());
        } else {
            let idx = self.entries.len();
            self.entries.push(OperationJournalEntry {
                index: idx,
                op_type: OperationType::Replace,
                relative_path: rel_path.to_string(),
                backup_performed: true,
                backup_sha256: Some(sha.to_string()),
                apply_performed: false,
                rollback_performed: false,
                timestamp: Utc::now(),
            });
        }
    }

    /// Rejestruje wykonanie operacji apply (podmiany lub utworzenia pliku).
    pub fn record_apply(&mut self, rel_path: &str, op_type: OperationType) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.relative_path == rel_path)
        {
            existing.apply_performed = true;
            existing.timestamp = Utc::now();
        } else {
            let idx = self.entries.len();
            self.entries.push(OperationJournalEntry {
                index: idx,
                op_type,
                relative_path: rel_path.to_string(),
                backup_performed: false,
                backup_sha256: None,
                apply_performed: true,
                rollback_performed: false,
                timestamp: Utc::now(),
            });
        }
    }

    /// Rejestruje pomyślne wycofanie operacji.
    pub fn record_rollback(&mut self, rel_path: &str) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.relative_path == rel_path)
        {
            existing.rollback_performed = true;
            existing.timestamp = Utc::now();
        }
    }

    /// Zapisuje dziennik operacji atomowo do pliku JSON.
    pub fn save_atomic(&self, path: &Path) -> Result<(), UpdaterError> {
        let parent = path.parent().ok_or_else(|| {
            UpdaterError::Io("Brak katalogu nadrzędnego dla dziennika operacji".to_string())
        })?;

        let json = serde_json::to_string_pretty(self).map_err(|e| {
            UpdaterError::Io(format!("Błąd serializacji HelperOperationJournal: {e}"))
        })?;

        let temp_filename = format!(
            ".journal_{}_{}.tmp",
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
                    "Nie można utworzyć pliku tymczasowego dziennika {}: {e}",
                    temp_path.display()
                ))
            })?;

        file.write_all(json.as_bytes()).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!("Błąd zapisu pliku dziennika pomocnika: {e}"))
        })?;

        file.sync_all().map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!(
                "Błąd synchronizacji pliku dziennika pomocnika: {e}"
            ))
        })?;

        drop(file);

        fs::rename(&temp_path, path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            UpdaterError::Io(format!(
                "Błąd atomowego przeniesienia pliku dziennika do {}: {e}",
                path.display()
            ))
        })?;

        Ok(())
    }

    /// Odczytuje dziennik z pliku JSON.
    pub fn load_from_path(path: &Path) -> Result<Self, UpdaterError> {
        if !path.exists() {
            return Err(UpdaterError::Io(format!(
                "Plik dziennika operacji nie istnieje: {}",
                path.display()
            )));
        }

        let content = fs::read_to_string(path).map_err(|e| {
            UpdaterError::Io(format!(
                "Nie można odczytać dziennika operacji {}: {e}",
                path.display()
            ))
        })?;

        let journal: Self = serde_json::from_str(&content).map_err(|e| {
            UpdaterError::Io(format!(
                "Błąd deserializacji dziennika operacji {}: {e}",
                path.display()
            ))
        })?;

        Ok(journal)
    }
}

/// Abstrakcja monitora procesów pozwalająca na niezależne testowanie i izolację.
pub trait ProcessMonitor: Send + Sync {
    /// Sprawdza, czy proces o danym PID nadal działa i czy odpowiada oczekiwanej tożsamości.
    fn is_process_running(
        &self,
        pid: u32,
        expected_image: &str,
    ) -> Result<bool, WindowsHelperExitCode>;
}

/// Standardowy monitor procesów Windows oparty na `tasklist.exe` bez użycia powłoki.
#[derive(Debug, Default)]
pub struct SystemProcessMonitor;

impl ProcessMonitor for SystemProcessMonitor {
    fn is_process_running(
        &self,
        pid: u32,
        expected_image: &str,
    ) -> Result<bool, WindowsHelperExitCode> {
        #[cfg(target_os = "windows")]
        {
            // tasklist /FI "PID eq <pid>" /FO CSV /NH
            let output = Command::new("tasklist.exe")
                .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
                .output()
                .map_err(|_| WindowsHelperExitCode::PersistenceFailed)?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let trimmed = stdout.trim();

            if trimmed.is_empty() || trimmed.contains("INFO:") {
                return Ok(false);
            }

            // Parsowanie CSV: "image_name","pid","session_name","session_num","mem_usage"
            let expected_lower = expected_image.to_ascii_lowercase();
            for line in trimmed.lines() {
                let parts: Vec<&str> = line.split(',').collect();
                if let Some(image_part) = parts.first() {
                    let image_clean = image_part.trim_matches('"').to_ascii_lowercase();
                    if image_clean == expected_lower {
                        return Ok(true);
                    } else if !image_clean.is_empty() {
                        // PID został ponownie użyty przez obcy proces
                        return Err(WindowsHelperExitCode::ParentProcessIdentityMismatch);
                    }
                }
            }

            Ok(false)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let proc_path = PathBuf::from(format!("/proc/{pid}"));
            if !proc_path.exists() {
                return Ok(false);
            }

            // Opcjonalne sprawdzenie nazwy procesu w comm
            let comm_path = proc_path.join("comm");
            if let Ok(comm) = fs::read_to_string(comm_path) {
                let comm_trimmed = comm.trim().to_ascii_lowercase();
                let exp_trimmed = expected_image.to_ascii_lowercase();
                if !comm_trimmed.contains(&exp_trimmed) && !exp_trimmed.contains(&comm_trimmed) {
                    return Err(WindowsHelperExitCode::ParentProcessIdentityMismatch);
                }
            }

            Ok(true)
        }
    }
}

/// Abstrakcja uchwytu do uruchomionego kandydata.
pub trait HelperCandidateChild: Send + Sync {
    /// Sprawdza, czy proces kandydata nadal działa.
    fn is_alive(&mut self) -> Result<bool, WindowsHelperExitCode>;
    /// Zatrzymuje proces kandydata (force stop przy awarii testu zdrowia).
    fn terminate(&mut self) -> Result<(), WindowsHelperExitCode>;
}

/// Natywny uchwyt procesu kandydata `std::process::Child`.
pub struct SystemCandidateChild(pub Child);

impl HelperCandidateChild for SystemCandidateChild {
    fn is_alive(&mut self) -> Result<bool, WindowsHelperExitCode> {
        match self.0.try_wait() {
            Ok(None) => Ok(true),
            Ok(Some(_)) => Ok(false),
            Err(_) => Err(WindowsHelperExitCode::PersistenceFailed),
        }
    }

    fn terminate(&mut self) -> Result<(), WindowsHelperExitCode> {
        let _ = self.0.kill();
        let _ = self.0.wait();
        Ok(())
    }
}

/// Abstrakcja mechanizmu uruchamiania kandydata aktualizacji.
pub trait CandidateLauncher: Send + Sync {
    /// Uruchamia kandydata bez powłoki (no-shell) z parametrami diagnostycznymi.
    fn launch_candidate(
        &self,
        candidate_exe: &Path,
        attempt_id: &UpdateAttemptId,
        health_proof_path: &Path,
        expected_version: &str,
        expected_commit: &str,
    ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode>;
}

/// Standardowy launcher procesów kandydata.
#[derive(Debug, Default)]
pub struct SystemCandidateLauncher;

impl CandidateLauncher for SystemCandidateLauncher {
    fn launch_candidate(
        &self,
        candidate_exe: &Path,
        attempt_id: &UpdateAttemptId,
        health_proof_path: &Path,
        expected_version: &str,
        expected_commit: &str,
    ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode> {
        if !candidate_exe.exists() {
            return Err(WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded);
        }

        let child = Command::new(candidate_exe)
            .arg("--check-health-startup")
            .arg("--update-attempt-id")
            .arg(attempt_id.to_string())
            .arg("--health-proof-path")
            .arg(health_proof_path)
            .arg("--expected-version")
            .arg(expected_version)
            .arg("--expected-commit")
            .arg(expected_commit)
            .spawn()
            .map_err(|_| WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded)?;

        Ok(Box::new(SystemCandidateChild(child)))
    }
}

/// Główny silnik wykonawczy pomocnika Windows (`WindowsHelperEngine`).
pub struct WindowsHelperEngine<'a> {
    request: &'a WindowsHelperRequest,
    process_monitor: &'a dyn ProcessMonitor,
    candidate_launcher: &'a dyn CandidateLauncher,
    journal: HelperOperationJournal,
}

impl<'a> WindowsHelperEngine<'a> {
    /// Inicjuje silnik z podanym żądaniem i zależnościami monitora i launchera.
    pub fn new(
        request: &'a WindowsHelperRequest,
        process_monitor: &'a dyn ProcessMonitor,
        candidate_launcher: &'a dyn CandidateLauncher,
    ) -> Self {
        let journal = HelperOperationJournal::new(request.attempt_id, request.plan_digest.clone());
        Self {
            request,
            process_monitor,
            candidate_launcher,
            journal,
        }
    }

    /// Wykonuje pełny bezpieczny cykl aktualizacji pomocnika Windows.
    pub fn execute(&mut self) -> (WindowsHelperExitCode, WindowsHelperResult) {
        self.log_diagnostic("Rozpoczęcie procedury pomocnika Windows M3-B");

        // 1. Walidacja żądania
        if let Err(err) = self.request.validate() {
            let msg = format!("Błąd walidacji żądania pomocnika: {err}");
            self.log_diagnostic(&msg);
            return self.build_result(
                UpdateStateId::FailedPreApply,
                HelperResultCategory::AbortedPreApply,
                WindowsHelperExitCode::InvalidArguments,
                None,
                None,
                0,
                0,
                false,
                false,
                msg,
            );
        }

        // 2. Weryfikacja powiązanej transakcji
        if let Err(code) = self.verify_transaction_binding() {
            return self.build_result(
                UpdateStateId::FailedPreApply,
                HelperResultCategory::AbortedPreApply,
                code,
                None,
                None,
                0,
                0,
                false,
                false,
                format!("Błąd tożsamości transakcji: {}", code.description()),
            );
        }

        // 3. Oczekiwanie na normalne zakończenie procesu macierzystego
        self.log_diagnostic("Oczekiwanie na zamknięcie procesu macierzystego");
        if let Err(code) = self.wait_for_parent_exit() {
            self.log_diagnostic(&format!(
                "Awaria oczekiwania na proces macierzysty: {code:?}"
            ));
            return self.build_result(
                UpdateStateId::FailedPreApply,
                HelperResultCategory::AbortedPreApply,
                code,
                None,
                None,
                0,
                0,
                false,
                false,
                format!(
                    "Proces macierzysty nie zakończył się poprawnie: {}",
                    code.description()
                ),
            );
        }

        // 4. Kopia zapasowa plików replace_files przed modyfikacją
        self.log_diagnostic("Tworzenie kopii zapasowej podmienianych plików programu w .rollback");
        if let Err(code) = self.create_backup() {
            self.log_diagnostic(&format!("Błąd tworzenia kopii zapasowej: {code:?}"));
            return self.build_result(
                UpdateStateId::FailedPreApply,
                HelperResultCategory::AbortedPreApply,
                code,
                None,
                None,
                0,
                0,
                false,
                false,
                "Nie udało się utworzyć kopii zapasowej podmienianych plików".to_string(),
            );
        }

        // 5. Zastosowanie zmian (apply)
        self.log_diagnostic("Rozpoczęcie podmiany plików (Applying)");
        let total_ops = self
            .request
            .update_plan_identity
            .ordered_replace_entries
            .len()
            + self
                .request
                .update_plan_identity
                .ordered_create_entries
                .len();

        let mut completed_ops = 0;
        if let Err(err_code) = self.apply_plan_operations(&mut completed_ops) {
            self.log_diagnostic(&format!(
                "Awaria podczas fazy apply: {err_code:?}. Rozpoczęcie procedury rollback."
            ));
            let rollback_kind = self.execute_rollback();
            let exit_code = match rollback_kind {
                RollbackResultKind::Success => WindowsHelperExitCode::ApplyFailedRollbackSucceeded,
                RollbackResultKind::Partial => WindowsHelperExitCode::ApplyFailedRollbackPartial,
                _ => WindowsHelperExitCode::ApplyFailedRollbackFailed,
            };
            let category = match rollback_kind {
                RollbackResultKind::Success => HelperResultCategory::FailedRollbackSucceeded,
                RollbackResultKind::Partial => HelperResultCategory::FailedRollbackPartial,
                _ => HelperResultCategory::FailedRollbackFailed,
            };
            return self.build_result(
                UpdateStateId::FailedPostApply,
                category,
                exit_code,
                Some(rollback_kind),
                None,
                completed_ops,
                total_ops,
                true,
                rollback_kind != RollbackResultKind::Success,
                "Błąd podczas podmiany plików; wykonano rollback".to_string(),
            );
        }

        // 6. Uruchomienie kandydata i weryfikacja zdrowia (Health Check)
        self.log_diagnostic("Uruchomienie kandydata aktualizacji (LaunchingCandidate)");
        let (health_result, exit_code) = self.verify_candidate_health();
        if health_result != HealthResultKind::Passed {
            self.log_diagnostic(&format!(
                "Awaria testu zdrowia kandydata: {health_result:?}. Rozpoczęcie procedury rollback."
            ));
            let rollback_kind = self.execute_rollback();
            let final_exit_code = match exit_code {
                WindowsHelperExitCode::HealthTimeoutRollbackSucceeded => {
                    if rollback_kind == RollbackResultKind::Success {
                        WindowsHelperExitCode::HealthTimeoutRollbackSucceeded
                    } else {
                        WindowsHelperExitCode::ApplyFailedRollbackFailed
                    }
                }
                WindowsHelperExitCode::HealthRejectedRollbackSucceeded => {
                    if rollback_kind == RollbackResultKind::Success {
                        WindowsHelperExitCode::HealthRejectedRollbackSucceeded
                    } else {
                        WindowsHelperExitCode::ApplyFailedRollbackFailed
                    }
                }
                _ => {
                    if rollback_kind == RollbackResultKind::Success {
                        WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded
                    } else {
                        WindowsHelperExitCode::ApplyFailedRollbackFailed
                    }
                }
            };
            let category = if rollback_kind == RollbackResultKind::Success {
                HelperResultCategory::FailedRollbackSucceeded
            } else {
                HelperResultCategory::FailedRollbackFailed
            };

            return self.build_result(
                UpdateStateId::FailedPostApply,
                category,
                final_exit_code,
                Some(rollback_kind),
                Some(health_result),
                completed_ops,
                total_ops,
                true,
                rollback_kind != RollbackResultKind::Success,
                format!(
                    "Niepowodzenie testu zdrowia kandydata ({health_result:?}); wykonano rollback"
                ),
            );
        }

        // 7. Sukces i czyszczenie
        self.log_diagnostic("Weryfikacja zdrowia kandydata zakończona sukcesem (Committed)");
        self.cleanup_on_success();

        self.build_result(
            UpdateStateId::Committed,
            HelperResultCategory::Success,
            WindowsHelperExitCode::Success,
            Some(RollbackResultKind::None),
            Some(HealthResultKind::Passed),
            completed_ops,
            total_ops,
            true, // Backup zachowany zgodnie z polityką retencji OWNER-M2-08/09
            false,
            "Aktualizacja i weryfikacja zdrowia kandydata zakończona pełnym sukcesem".to_string(),
        )
    }

    /// Weryfikuje zgodność powiązania z rekordem transakcji.
    fn verify_transaction_binding(&self) -> Result<(), WindowsHelperExitCode> {
        let trans_path = &self.request.transaction_record_path;
        if !trans_path.exists() {
            return Err(WindowsHelperExitCode::TransactionMismatch);
        }

        let bytes = fs::read(trans_path).map_err(|_| WindowsHelperExitCode::TransactionMismatch)?;
        let trans: UpdateTransaction = serde_json::from_slice(&bytes)
            .map_err(|_| WindowsHelperExitCode::TransactionMismatch)?;

        if trans.attempt_id != self.request.attempt_id {
            return Err(WindowsHelperExitCode::TransactionMismatch);
        }

        if let Some(ref trans_digest) = trans.plan_digest
            && trans_digest != &self.request.plan_digest
        {
            return Err(WindowsHelperExitCode::PlanDigestMismatch);
        }

        Ok(())
    }

    /// Oczekuje na normalne zamknięcie procesu macierzystego z limitem czasu.
    fn wait_for_parent_exit(&self) -> Result<(), WindowsHelperExitCode> {
        let pid = self.request.expected_parent_pid;
        let expected_image = self
            .request
            .current_executable
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("SPLogbook.exe");

        let timeout = Duration::from_secs(self.request.parent_process_timeout_seconds);
        let start = Instant::now();

        while start.elapsed() < timeout {
            match self.process_monitor.is_process_running(pid, expected_image) {
                Ok(true) => {
                    // Proces nadal działa - krótki sen bez busy loop
                    sleep(Duration::from_millis(250));
                }
                Ok(false) => {
                    // Proces zakończony! Krótka pauza na zwolnienie blokad plików NTFS
                    sleep(Duration::from_millis(100));
                    return Ok(());
                }
                Err(code) => {
                    return Err(code);
                }
            }
        }

        Err(WindowsHelperExitCode::ParentProcessTimeout)
    }

    /// Tworzy kopię zapasową wyłącznie dla plików `replace_files` w podkatalogu `.rollback`.
    fn create_backup(&mut self) -> Result<(), WindowsHelperExitCode> {
        let root = &self.request.installation_root;
        let rollback_dir = root.join(ROLLBACK_DIR_NAME);

        if rollback_dir.exists() {
            let _ = fs::remove_dir_all(&rollback_dir);
        }
        fs::create_dir_all(&rollback_dir).map_err(|_| WindowsHelperExitCode::BackupFailed)?;

        for rel in &self.request.update_plan_identity.ordered_replace_entries {
            let src = root.join(rel);
            let dst = rollback_dir.join(rel);

            if !src.exists() {
                // Plik do podmiany musi istnieć w aktywnej instalacji
                return Err(WindowsHelperExitCode::BackupFailed);
            }

            if let Some(parent) = dst.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent).map_err(|_| WindowsHelperExitCode::BackupFailed)?;
            }

            fs::copy(&src, &dst).map_err(|_| WindowsHelperExitCode::BackupFailed)?;

            // Weryfikacja rozmiaru i SHA-256 kopii zapasowej
            let src_data = fs::read(&src).map_err(|_| WindowsHelperExitCode::BackupFailed)?;
            let dst_data = fs::read(&dst).map_err(|_| WindowsHelperExitCode::BackupFailed)?;

            if src_data.len() != dst_data.len() {
                return Err(WindowsHelperExitCode::BackupFailed);
            }

            let sha_src = sha256_hex(&src_data);
            let sha_dst = sha256_hex(&dst_data);

            if sha_src != sha_dst {
                return Err(WindowsHelperExitCode::BackupFailed);
            }

            self.journal.record_backup(rel, &sha_src);
        }

        // Zapis stanu dziennika po wykonaniu kopii
        let journal_path = root.join(OPERATION_JOURNAL_FILENAME);
        let _ = self.journal.save_atomic(&journal_path);

        Ok(())
    }

    /// Wykonuje operacje podmiany (`replace`) oraz utworzenia (`create`) plików.
    fn apply_plan_operations(
        &mut self,
        completed_ops: &mut usize,
    ) -> Result<(), WindowsHelperExitCode> {
        let root = &self.request.installation_root;
        let staging_dir = root.join(STAGING_DIR_NAME);
        let journal_path = root.join(OPERATION_JOURNAL_FILENAME);

        // 1. Podmiana plików `replace_files`
        for rel in &self.request.update_plan_identity.ordered_replace_entries {
            let src = staging_dir.join(rel);
            let dst = root.join(rel);

            if !src.exists() {
                return Err(WindowsHelperExitCode::ApplyFailedRollbackSucceeded);
            }

            if is_protected_user_path(&dst) {
                return Err(WindowsHelperExitCode::UnsafePath);
            }

            fs::copy(&src, &dst)
                .map_err(|_| WindowsHelperExitCode::ApplyFailedRollbackSucceeded)?;

            self.journal.record_apply(rel, OperationType::Replace);
            *completed_ops += 1;
            let _ = self.journal.save_atomic(&journal_path);
        }

        // 2. Utworzenie nowych plików `create_files`
        for rel in &self.request.update_plan_identity.ordered_create_entries {
            let src = staging_dir.join(rel);
            let dst = root.join(rel);

            if !src.exists() {
                return Err(WindowsHelperExitCode::ApplyFailedRollbackSucceeded);
            }

            if dst.exists() {
                // Plik create nie może istnieć przed utworzeniem (zasada fail-closed)
                return Err(WindowsHelperExitCode::UnsafePath);
            }

            if is_protected_user_path(&dst) {
                return Err(WindowsHelperExitCode::UnsafePath);
            }

            if let Some(parent) = dst.parent()
                && !parent.exists()
            {
                let _ = fs::create_dir_all(parent);
            }

            fs::copy(&src, &dst)
                .map_err(|_| WindowsHelperExitCode::ApplyFailedRollbackSucceeded)?;

            self.journal.record_apply(rel, OperationType::Create);
            *completed_ops += 1;
            let _ = self.journal.save_atomic(&journal_path);
        }

        Ok(())
    }

    /// Uruchamia kandydata i czeka na zweryfikowany dowód zdrowia `HealthProof`.
    fn verify_candidate_health(&mut self) -> (HealthResultKind, WindowsHelperExitCode) {
        let root = &self.request.installation_root;
        let candidate_exe = root.join(&self.request.update_plan_identity.candidate_executable);
        let health_proof_path = &self.request.health_proof_path;

        // Przed startem kandydata usuwamy wyłącznie własny stary dowód zdrowia
        if health_proof_path.exists() {
            let _ = fs::remove_file(health_proof_path);
        }

        let mut child_handle = match self.candidate_launcher.launch_candidate(
            &candidate_exe,
            &self.request.attempt_id,
            health_proof_path,
            &self.request.expected_target_version,
            &self.request.expected_target_commit,
        ) {
            Ok(handle) => handle,
            Err(code) => return (HealthResultKind::Crash, code),
        };

        let timeout = Duration::from_secs(self.request.candidate_health_timeout_seconds);
        let start = Instant::now();

        while start.elapsed() < timeout {
            // Sprawdzamy czy plik dowodu zdrowia powstał
            if health_proof_path.exists() {
                if let Ok(bytes) = fs::read(health_proof_path)
                    && let Ok(proof) = serde_json::from_slice::<HealthProof>(&bytes)
                {
                    // Weryfikacja tożsamości dowodu
                    if proof
                        .validate(
                            &self.request.attempt_id,
                            &self.request.expected_target_version,
                            &self.request.expected_target_commit,
                            DEFAULT_HEALTH_PROOF_MAX_AGE_SECS,
                        )
                        .is_ok()
                    {
                        // Pomyślna weryfikacja zdrowia!
                        return (HealthResultKind::Passed, WindowsHelperExitCode::Success);
                    }
                    // Dowód odrzucony
                    let _ = child_handle.terminate();
                    return (
                        HealthResultKind::Rejected,
                        WindowsHelperExitCode::HealthRejectedRollbackSucceeded,
                    );
                }
                // Plik w trakcie zapisu lub uszkodzony - krótka pauza
                sleep(Duration::from_millis(100));
            }

            // Sprawdzamy czy kandydat nie uległ awarii (crash/exit)
            match child_handle.is_alive() {
                Ok(false) => {
                    // Kandydat zakończył się przed zapisem poprawnego dowodu zdrowia
                    return (
                        HealthResultKind::Crash,
                        WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded,
                    );
                }
                Err(code) => {
                    let _ = child_handle.terminate();
                    return (HealthResultKind::Crash, code);
                }
                Ok(true) => {}
            }

            sleep(Duration::from_millis(200));
        }

        // Limit czasu upłynął
        let _ = child_handle.terminate();
        (
            HealthResultKind::TimedOut,
            WindowsHelperExitCode::HealthTimeoutRollbackSucceeded,
        )
    }

    /// Wykonuje deterministyczną i idempotentną procedurę rollbacku.
    pub fn execute_rollback(&mut self) -> RollbackResultKind {
        self.log_diagnostic("Rozpoczęcie procedury wycofywania zmian (Rollback)");
        let root = &self.request.installation_root;
        let rollback_dir = root.join(ROLLBACK_DIR_NAME);
        let journal_path = root.join(OPERATION_JOURNAL_FILENAME);

        let mut errors_encountered = 0;

        // 1. Usunięcie wszystkich utworzonych plików `create_files`
        for entry in self.journal.entries.iter_mut().filter(|e| {
            e.op_type == OperationType::Create && e.apply_performed && !e.rollback_performed
        }) {
            let target = root.join(&entry.relative_path);
            if target.exists() {
                if is_protected_user_path(&target) {
                    errors_encountered += 1;
                    continue;
                }
                if fs::remove_file(&target).is_ok() {
                    entry.rollback_performed = true;
                } else {
                    errors_encountered += 1;
                }
            } else {
                entry.rollback_performed = true;
            }
        }

        // 2. Przywrócenie podmienionych plików `replace_files` z katalogu `.rollback`
        for entry in self.journal.entries.iter_mut().filter(|e| {
            e.op_type == OperationType::Replace && e.apply_performed && !e.rollback_performed
        }) {
            let bkp = rollback_dir.join(&entry.relative_path);
            let dst = root.join(&entry.relative_path);

            if !bkp.exists() {
                errors_encountered += 1;
                continue;
            }

            if is_protected_user_path(&dst) {
                errors_encountered += 1;
                continue;
            }

            if fs::copy(&bkp, &dst).is_ok() {
                // Weryfikacja przywróconego pliku
                if let Ok(data) = fs::read(&dst) {
                    let sha = sha256_hex(&data);
                    if let Some(ref expected_sha) = entry.backup_sha256 {
                        if sha == *expected_sha {
                            entry.rollback_performed = true;
                        } else {
                            errors_encountered += 1;
                        }
                    } else {
                        entry.rollback_performed = true;
                    }
                } else {
                    errors_encountered += 1;
                }
            } else {
                errors_encountered += 1;
            }
        }

        let _ = self.journal.save_atomic(&journal_path);

        if errors_encountered == 0 {
            self.log_diagnostic("Rollback zakończony pełnym sukcesem");
            RollbackResultKind::Success
        } else {
            self.log_diagnostic(&format!(
                "Rollback częściowy: wystąpiło {errors_encountered} błędów"
            ));
            RollbackResultKind::Partial
        }
    }

    /// Bezpieczne czyszczenie po pomyślnej aktualizacji.
    fn cleanup_on_success(&self) {
        let root = &self.request.installation_root;
        let staging_dir = root.join(STAGING_DIR_NAME);
        let health_proof_path = &self.request.health_proof_path;

        // Usuwamy staging
        if staging_dir.exists() {
            let _ = fs::remove_dir_all(&staging_dir);
        }

        // Usuwamy plik dowodu zdrowia
        if health_proof_path.exists() {
            let _ = fs::remove_file(health_proof_path);
        }

        // Zgodnie z decyzjami OWNER-M2-08/09 katalog `.rollback` jest zachowywany do czasu zatwierdzenia polityki retencji
        self.log_diagnostic("Katalog .rollback zachowany zgodnie z otwartą polityką retencji");
    }

    /// Buduje, trwale zapisuje i zwraca rekord wyniku `WindowsHelperResult`.
    #[allow(clippy::too_many_arguments)]
    fn build_result(
        &self,
        final_state: UpdateStateId,
        category: HelperResultCategory,
        exit_code: WindowsHelperExitCode,
        rollback: Option<RollbackResultKind>,
        health: Option<HealthResultKind>,
        completed_ops: usize,
        total_ops: usize,
        backup_retained: bool,
        manual_recovery_required: bool,
        summary: String,
    ) -> (WindowsHelperExitCode, WindowsHelperResult) {
        let res = WindowsHelperResult::new(
            self.request.attempt_id,
            self.request.plan_digest.clone(),
            final_state,
            "NativeRust",
            category,
            exit_code,
            rollback,
            health,
            completed_ops,
            total_ops,
            backup_retained,
            manual_recovery_required,
            summary,
        );

        let _ = res.save_atomic(&self.request.result_record_path);
        (exit_code, res)
    }

    /// Zapisuje zredagowany log diagnostyczny do wskazanego pliku.
    fn log_diagnostic(&self, message: &str) {
        let log_path = &self.request.diagnostic_log_path;
        let now = Utc::now().to_rfc3339();
        let attempt = self.request.attempt_id;
        let line = format!("[{now}] [{attempt}] {message}\n");

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

// ============================================================================
// Testy jednostkowe silnika pomocnika
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::updater::ManagementMode;
    use crate::cloud::updater::UpdatePlan;
    use crate::cloud::updater::helper_protocol::{
        CURRENT_HELPER_REQUEST_VERSION, UpdatePlanIdentity,
    };
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tempfile::tempdir;

    struct MockMonitor {
        call_count: Arc<AtomicU32>,
        terminate_after_polls: u32,
    }

    impl ProcessMonitor for MockMonitor {
        fn is_process_running(
            &self,
            _pid: u32,
            _expected_image: &str,
        ) -> Result<bool, WindowsHelperExitCode> {
            let cnt = self.call_count.fetch_add(1, Ordering::SeqCst);
            if cnt >= self.terminate_after_polls {
                Ok(false)
            } else {
                Ok(true)
            }
        }
    }

    struct MockCandidateChild;
    impl HelperCandidateChild for MockCandidateChild {
        fn is_alive(&mut self) -> Result<bool, WindowsHelperExitCode> {
            Ok(true)
        }
        fn terminate(&mut self) -> Result<(), WindowsHelperExitCode> {
            Ok(())
        }
    }

    struct MockLauncher {
        create_valid_proof: bool,
    }

    impl CandidateLauncher for MockLauncher {
        fn launch_candidate(
            &self,
            _candidate_exe: &Path,
            attempt_id: &UpdateAttemptId,
            health_proof_path: &Path,
            expected_version: &str,
            expected_commit: &str,
        ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode> {
            if self.create_valid_proof {
                let proof = HealthProof::new(
                    *attempt_id,
                    expected_version,
                    expected_commit,
                    "windows-x86_64",
                    crate::cloud::updater::InstallationType::WindowsPortableZip,
                );
                let json = serde_json::to_string_pretty(&proof).unwrap();
                fs::write(health_proof_path, json).unwrap();
            }
            Ok(Box::new(MockCandidateChild))
        }
    }

    fn setup_test_environment() -> (tempfile::TempDir, WindowsHelperRequest) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let staging = root.join(STAGING_DIR_NAME);
        fs::create_dir_all(&staging).unwrap();

        // Aktywne pliki
        fs::write(root.join("SPLogbook.exe"), b"OLD_BIN").unwrap();
        fs::write(root.join("lib.dll"), b"OLD_LIB").unwrap();

        // Pliki w stagingu
        fs::write(staging.join("SPLogbook.exe"), b"NEW_BIN").unwrap();
        fs::write(staging.join("lib.dll"), b"NEW_LIB").unwrap();
        fs::create_dir_all(staging.join("resources")).unwrap();
        fs::write(staging.join("resources/new.dat"), b"NEW_RES").unwrap();

        let attempt = UpdateAttemptId::generate();
        let plan = UpdatePlan {
            replace_files: vec![
                (PathBuf::from("SPLogbook.exe"), root.join("SPLogbook.exe")),
                (PathBuf::from("lib.dll"), root.join("lib.dll")),
            ],
            create_files: vec![(
                PathBuf::from("resources/new.dat"),
                root.join("resources/new.dat"),
            )],
            preserve_files: Vec::new(),
            rejected_files: Vec::new(),
        };

        let plan_identity = UpdatePlanIdentity::from_plan(
            attempt,
            &root,
            &plan,
            "SPLogbook.exe",
            "1.2.1",
            "commit123",
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();

        let plan_digest = plan_identity.compute_digest();

        // Trwały rekord transakcji
        let mut trans = UpdateTransaction::new(
            attempt,
            "1.2.1".to_string(),
            "v1.2.1".to_string(),
            "windows".to_string(),
            "x86_64".to_string(),
            "windows-portable-zip".to_string(),
            crate::cloud::updater::InstallationType::WindowsPortableZip,
            "manifest_digest".to_string(),
            "SPLogbook-Windows-x64.zip".to_string(),
            1000,
            "asset_sha".to_string(),
            staging.clone(),
            &root,
        )
        .unwrap();
        trans.set_plan_digest(&plan_digest);
        trans.save_to_dir(&root).unwrap();

        let req = WindowsHelperRequest {
            request_version: CURRENT_HELPER_REQUEST_VERSION,
            attempt_id: attempt,
            transaction_record_path: root.join(".update_transaction.json"),
            installation_root: root.clone(),
            current_executable: root.join("SPLogbook.exe"),
            candidate_executable: root.join("SPLogbook.exe"),
            expected_parent_pid: 12345,
            expected_current_binary_identity: None,
            expected_target_version: "1.2.1".to_string(),
            expected_target_commit: "commit123".to_string(),
            update_management_mode: ManagementMode::SelfManaged,
            update_plan_identity: plan_identity,
            plan_digest,
            health_proof_path: root.join(".health_ok"),
            parent_process_timeout_seconds: 5,
            candidate_health_timeout_seconds: 5,
            result_record_path: root.join(".update_helper_result.json"),
            diagnostic_log_path: root.join("helper_test.log"),
        };

        (dir, req)
    }

    #[test]
    fn helper_engine_full_success_flow() {
        let (dir, req) = setup_test_environment();
        let root = dir.path();

        let monitor = MockMonitor {
            call_count: Arc::new(AtomicU32::new(0)),
            terminate_after_polls: 1,
        };
        let launcher = MockLauncher {
            create_valid_proof: true,
        };

        let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
        let (exit_code, result) = engine.execute();

        assert_eq!(exit_code, WindowsHelperExitCode::Success);
        assert_eq!(result.result_category, HelperResultCategory::Success);

        // Weryfikacja podmiany
        assert_eq!(fs::read(root.join("SPLogbook.exe")).unwrap(), b"NEW_BIN");
        assert_eq!(fs::read(root.join("lib.dll")).unwrap(), b"NEW_LIB");
        assert_eq!(
            fs::read(root.join("resources/new.dat")).unwrap(),
            b"NEW_RES"
        );

        // Kopia zapasowa zachowana zgodnie z polityką OWNER-M2-08/09
        assert!(root.join(ROLLBACK_DIR_NAME).exists());
        assert_eq!(
            fs::read(root.join(ROLLBACK_DIR_NAME).join("SPLogbook.exe")).unwrap(),
            b"OLD_BIN"
        );
        assert_eq!(
            fs::read(root.join(ROLLBACK_DIR_NAME).join("lib.dll")).unwrap(),
            b"OLD_LIB"
        );

        // Log diagnostyczny powstał
        assert!(root.join("helper_test.log").exists());
    }

    #[test]
    fn helper_engine_health_timeout_triggers_deterministic_rollback() {
        let (dir, req) = setup_test_environment();
        let root = dir.path();

        let monitor = MockMonitor {
            call_count: Arc::new(AtomicU32::new(0)),
            terminate_after_polls: 1,
        };
        // Nie tworzymy dowodu zdrowia, symulując timeout
        let launcher = MockLauncher {
            create_valid_proof: false,
        };

        let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
        let (exit_code, result) = engine.execute();

        assert_eq!(
            exit_code,
            WindowsHelperExitCode::HealthTimeoutRollbackSucceeded
        );
        assert_eq!(
            result.result_category,
            HelperResultCategory::FailedRollbackSucceeded
        );
        assert_eq!(result.rollback_result, Some(RollbackResultKind::Success));

        // Pliki oryginalne zostały przywrócone!
        assert_eq!(fs::read(root.join("SPLogbook.exe")).unwrap(), b"OLD_BIN");
        assert_eq!(fs::read(root.join("lib.dll")).unwrap(), b"OLD_LIB");

        // Nowo utworzony plik create_files został usunięty!
        assert!(!root.join("resources/new.dat").exists());
    }

    #[test]
    fn helper_rollback_is_idempotent() {
        let (dir, req) = setup_test_environment();
        let root = dir.path();

        let monitor = MockMonitor {
            call_count: Arc::new(AtomicU32::new(0)),
            terminate_after_polls: 1,
        };
        let launcher = MockLauncher {
            create_valid_proof: false,
        };

        let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
        let _ = engine.execute();

        // Pierwszy rollback przywrócił pliki. Powtórny rollback na tym samym dzienniku jest bezpieczny:
        let second_rollback = engine.execute_rollback();
        assert_eq!(second_rollback, RollbackResultKind::Success);

        assert_eq!(fs::read(root.join("SPLogbook.exe")).unwrap(), b"OLD_BIN");
        assert_eq!(fs::read(root.join("lib.dll")).unwrap(), b"OLD_LIB");
    }
}
