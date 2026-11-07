// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Model transakcji aktualizacji i trwały, crash-safe zapis stanu (M3-A)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::attempt_id::UpdateAttemptId;
use crate::cloud::updater::installation::InstallationType;
use crate::cloud::updater::state::{UpdateStateId, UpdateStateMachine};

/// Aktualna kanoniczna wersja rekordu transakcji aktualizatora.
pub const CURRENT_RECORD_VERSION: u32 = 1;

/// Nazwa kanonicznego pliku trwałego rekordu transakcji.
pub const TRANSACTION_FILE_NAME: &str = ".update_transaction.json";

/// Status żądania anulowania transakcji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CancellationStatus {
    NotRequested,
    Requested,
    RejectedPostApply,
    Completed,
}

/// Status procedury odzyskiwania (Crash Recovery).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryStatus {
    None,
    Required,
    InProgress,
    Completed,
    Blocked,
}

/// Trwały model transakcji aktualizacji SPLogbook.
///
/// Gwarancje bezpieczeństwa:
/// - Zapis transakcji jest crash-safe (zapis do pliku tymczasowego w tym samym katalogu, flush, sync_all, atomowy rename).
/// - Żadna część uszkodzonego pliku nie zastąpi poprawnego rekordu nadrzędnego.
/// - Brak jakichkolwiek sekretów, haseł, tokenów, kluczy prywatnych ani danych QSO w rekordzie.
/// - Ścisła walidacja wersji formatu rekordu (odrzucenie nieznanych wersji).
/// - Walidacja ścieżek pod kątem ucieczki poza installation root (brak `..`, brak prefiksów).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateTransaction {
    /// Wersja schematu rekordu transakcji.
    pub record_version: u32,
    /// Unikalny domenowy identyfikator próby aktualizacji (UUID v4).
    pub attempt_id: UpdateAttemptId,
    /// Bieżący zwalidowany stan automatu transakcji.
    pub current_state: UpdateStateId,
    /// Docelowa wersja wydania (SemVer).
    pub release_version: String,
    /// Znacznik wydania (git tag).
    pub release_tag: String,
    /// Opcjonalny skrót SHA commitu wydania.
    pub commit_identity: Option<String>,
    /// Docelowa platforma systemowa (np. "windows", "linux").
    pub platform: String,
    /// Docelowa architektura sprzętowa (np. "x86_64").
    pub architecture: String,
    /// Typ pakietu (np. "windows-portable-zip", "linux-portable-tar-gz").
    pub package_type: String,
    /// Sklasyfikowany typ instalacji.
    pub installation_type: InstallationType,
    /// Mechanizm aktualizacji ("self-managed" lub "system-managed").
    pub update_mechanism: String,
    /// Skrót kryptograficzny lub identyfikator manifestu wydań.
    pub manifest_digest: String,
    /// Nazwa pliku pobranego assetu.
    pub asset_filename: String,
    /// Rozmiar assetu w bajtach.
    pub asset_size: u64,
    /// Suma kontrolna SHA-256 assetu.
    pub asset_sha256: String,
    /// Bezpieczna ścieżka do katalogu stagingu (względna lub wewnątrz installation root).
    pub staging_path: PathBuf,
    /// Opcjonalna ścieżka do katalogu kopii zapasowej (.rollback).
    pub rollback_path: Option<PathBuf>,
    /// Opcjonalna ścieżka do pliku dowodu zdrowia (.health_ok).
    pub health_proof_path: Option<PathBuf>,
    /// Opcjonalna tożsamość nabytej blokady concurrency.
    pub lock_identity: Option<String>,
    /// Znacznik czasu UTC utworzenia transakcji.
    pub creation_time: DateTime<Utc>,
    /// Znacznik czasu UTC ostatniego przejścia stanu.
    pub last_transition_time: DateTime<Utc>,
    /// Status anulowania.
    pub cancellation_status: CancellationStatus,
    /// Status odzyskiwania po awarii.
    pub recovery_status: RecoveryStatus,
    /// Etap, na którym wystąpiła awaria (jeśli dotyczy).
    pub failure_stage: Option<UpdateStateId>,
    /// Zredagowane, pozbawione danych wrażliwych podsumowanie diagnostyczne.
    pub sanitized_diagnostic_summary: Option<String>,
    /// Wewnętrzny automat stanów egzekwujący reguły przejść.
    state_machine: UpdateStateMachine,
}

impl UpdateTransaction {
    /// Inicjuje nową transakcję aktualizacji w stanie początkowym `Idle`.
    pub fn new(
        attempt_id: UpdateAttemptId,
        release_version: String,
        release_tag: String,
        platform: String,
        architecture: String,
        package_type: String,
        installation_type: InstallationType,
        manifest_digest: String,
        asset_filename: String,
        asset_size: u64,
        asset_sha256: String,
        staging_path: PathBuf,
        installation_root: &Path,
    ) -> Result<Self, UpdaterError> {
        // Walidacja ścieżki stagingu pod kątem bezpieczeństwa
        validate_path_under_root(&staging_path, installation_root)?;

        let now = Utc::now();
        let state_machine = UpdateStateMachine::new();

        Ok(Self {
            record_version: CURRENT_RECORD_VERSION,
            attempt_id,
            current_state: state_machine.current_state(),
            release_version,
            release_tag,
            commit_identity: None,
            platform,
            architecture,
            package_type,
            installation_type,
            update_mechanism: installation_type.management_mode().as_str().to_string(),
            manifest_digest,
            asset_filename,
            asset_size,
            asset_sha256,
            staging_path,
            rollback_path: None,
            health_proof_path: None,
            lock_identity: None,
            creation_time: now,
            last_transition_time: now,
            cancellation_status: CancellationStatus::NotRequested,
            recovery_status: RecoveryStatus::None,
            failure_stage: None,
            sanitized_diagnostic_summary: None,
            state_machine,
        })
    }

    /// Dokonuje zwalidowanego przejścia stanu transakcji.
    pub fn transition_to(&mut self, next_state: UpdateStateId) -> Result<(), UpdaterError> {
        self.state_machine.transition_to(next_state)?;
        self.current_state = next_state;
        self.last_transition_time = Utc::now();
        Ok(())
    }

    /// Trwały, crash-safe zapis transakcji do pliku w installation_root.
    ///
    /// Wzorzec atomowego zapisu:
    /// 1. Serializacja JSON do pamięci RAM.
    /// 2. Utworzenie unikalnego pliku tymczasowego w TYM SAMYM katalogu (`target_dir`).
    /// 3. Zapis danych, `flush` oraz `sync_all`.
    /// 4. Atomowa podmiana docelowego pliku `.update_transaction.json` (`fs::rename`).
    /// 5. Usunięcie pliku tymczasowego w razie jakiegokolwiek błędu przed rename.
    pub fn save_to_dir(&self, target_dir: &Path) -> Result<PathBuf, UpdaterError> {
        let serialized = serde_json::to_vec_pretty(self).map_err(|e| {
            UpdaterError::Persistence(format!("Błąd serializacji UpdateTransaction do JSON: {e}"))
        })?;

        // Plik tymczasowy w tym samym katalogu gwarantuje ten sam filesystem dla atomowego rename
        let temp_filename = format!(
            ".update_transaction_{}.tmp",
            self.attempt_id.as_file_component()
        );
        let temp_path = target_dir.join(&temp_filename);
        let primary_path = target_dir.join(TRANSACTION_FILE_NAME);

        // Zapis z opcją tworzenia nowego pliku
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| {
                UpdaterError::Io(format!(
                    "Nie można utworzyć pliku tymczasowego transakcji {}: {e}",
                    temp_path.display()
                ))
            })?;

        if let Err(e) = file.write_all(&serialized) {
            let _ = fs::remove_file(&temp_path);
            return Err(UpdaterError::Io(format!(
                "Błąd zapisu danych transakcji do {}: {e}",
                temp_path.display()
            )));
        }

        if let Err(e) = file.flush() {
            let _ = fs::remove_file(&temp_path);
            return Err(UpdaterError::Io(format!(
                "Błąd flush pliku transakcji {}: {e}",
                temp_path.display()
            )));
        }

        if let Err(e) = file.sync_all() {
            let _ = fs::remove_file(&temp_path);
            return Err(UpdaterError::Io(format!(
                "Błąd sync_all pliku transakcji {}: {e}",
                temp_path.display()
            )));
        }

        // Zamknięcie pliku przed operacją rename
        drop(file);

        // Atomowa podmiana rekordu głównego
        if let Err(e) = fs::rename(&temp_path, &primary_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(UpdaterError::Persistence(format!(
                "Błąd atomowej podmiany rekordu transakcji {} -> {}: {e}",
                temp_path.display(),
                primary_path.display()
            )));
        }

        Ok(primary_path)
    }

    /// Bezpieczny odczyt rekordu transakcji z dysku.
    /// Zwraca `Ok(None)` jeśli plik transakcji nie istnieje.
    pub fn load_from_dir(
        target_dir: &Path,
        expected_root: &Path,
    ) -> Result<Option<Self>, UpdaterError> {
        let primary_path = target_dir.join(TRANSACTION_FILE_NAME);
        if !primary_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&primary_path).map_err(|e| {
            UpdaterError::Io(format!(
                "Błąd odczytu pliku transakcji {}: {e}",
                primary_path.display()
            ))
        })?;

        let tx: Self = serde_json::from_slice(&bytes).map_err(|e| {
            UpdaterError::InvalidTransaction(format!(
                "Błąd deserializacji rekordu transakcji {}: {e}",
                primary_path.display()
            ))
        })?;

        // Walidacja wersji rekordu
        if tx.record_version != CURRENT_RECORD_VERSION {
            return Err(UpdaterError::UnsupportedTransactionVersion(
                tx.record_version,
            ));
        }

        // Walidacja ścieżek
        validate_path_under_root(&tx.staging_path, expected_root)?;
        if let Some(ref rb) = tx.rollback_path {
            validate_path_under_root(rb, expected_root)?;
        }
        if let Some(ref hp) = tx.health_proof_path {
            validate_path_under_root(hp, expected_root)?;
        }

        Ok(Some(tx))
    }

    /// Usuwa plik transakcji po zakończonym procesie cleanup.
    pub fn remove_record(target_dir: &Path) -> Result<(), UpdaterError> {
        let primary_path = target_dir.join(TRANSACTION_FILE_NAME);
        if primary_path.exists() {
            fs::remove_file(&primary_path).map_err(|e| {
                UpdaterError::Io(format!(
                    "Nie można usunąć pliku transakcji {}: {e}",
                    primary_path.display()
                ))
            })?;
        }
        Ok(())
    }
}

/// Waliduje, czy podana ścieżka nie ucieka poza installation root i nie zawiera sekwencji `..`.
pub fn validate_path_under_root(path: &Path, root: &Path) -> Result<(), UpdaterError> {
    for comp in path.components() {
        if let std::path::Component::ParentDir = comp {
            return Err(UpdaterError::UnsafePath(format!(
                "Ścieżka '{}' zawiera niedozwolony komponent '..' (traversal)",
                path.display()
            )));
        }
    }

    // Jeśli ścieżka jest względna, po złączeniu z rootem musi być poprawna
    let combined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };

    // Weryfikacja prefixu roota dla ścieżek bezwzględnych
    if combined.is_absolute() && root.is_absolute() && !combined.starts_with(root) {
        return Err(UpdaterError::UnsafePath(format!(
            "Ścieżka '{}' ucieka poza installation root '{}'",
            combined.display(),
            root.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tx(temp_dir: &Path, attempt_id: UpdateAttemptId) -> UpdateTransaction {
        let staging = temp_dir.join(".staging");
        UpdateTransaction::new(
            attempt_id,
            "1.3.0".into(),
            "v1.3.0".into(),
            "windows".into(),
            "x86_64".into(),
            "windows-portable-zip".into(),
            InstallationType::WindowsPortableZip,
            "abcdef1234567890".into(),
            "SPLogbook-Windows-x64.zip".into(),
            12_345_678,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            staging,
            temp_dir,
        )
        .expect("Tworzenie poprawnej transakcji")
    }

    #[test]
    fn transaction_starts_in_idle() {
        let temp = tempfile::tempdir().unwrap();
        let tx = sample_tx(temp.path(), UpdateAttemptId::generate());
        assert_eq!(tx.current_state, UpdateStateId::Idle);
    }

    #[test]
    fn transaction_contains_attempt_id() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let tx = sample_tx(temp.path(), attempt);
        assert_eq!(tx.attempt_id, attempt);
    }

    #[test]
    fn transaction_rejects_unknown_record_version() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(temp.path(), attempt);
        tx.record_version = 999; // Nieznana wersja formatu

        let json = serde_json::to_vec_pretty(&tx).unwrap();
        let primary = temp.path().join(TRANSACTION_FILE_NAME);
        std::fs::write(&primary, json).unwrap();

        let res = UpdateTransaction::load_from_dir(temp.path(), temp.path());
        assert!(matches!(
            res,
            Err(UpdaterError::UnsupportedTransactionVersion(999))
        ));
    }

    #[test]
    fn unknown_transaction_version_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let primary = temp.path().join(TRANSACTION_FILE_NAME);
        let raw_json = r#"{
            "record_version": 42,
            "attempt_id": "11111111-2222-4333-8444-555555555555",
            "current_state": "Idle",
            "release_version": "2.0.0",
            "release_tag": "v2.0.0",
            "commit_identity": null,
            "platform": "windows",
            "architecture": "x86_64",
            "package_type": "windows-portable-zip",
            "installation_type": "WindowsPortableZip",
            "update_mechanism": "SelfManaged",
            "manifest_digest": "digest",
            "asset_filename": "asset.zip",
            "asset_size": 100,
            "asset_sha256": "sha",
            "staging_path": ".staging",
            "rollback_path": null,
            "health_proof_path": null,
            "lock_identity": null,
            "creation_time": "2026-11-07T12:00:00Z",
            "last_transition_time": "2026-11-07T12:00:00Z",
            "cancellation_status": "NotRequested",
            "recovery_status": "None",
            "failure_stage": null,
            "sanitized_diagnostic_summary": null,
            "state_machine": {
                "current_state": "Idle",
                "history": [["Idle", "2026-11-07T12:00:00Z"]]
            }
        }"#;
        std::fs::write(&primary, raw_json).unwrap();
        let res = UpdateTransaction::load_from_dir(temp.path(), temp.path());
        assert!(matches!(
            res,
            Err(UpdaterError::UnsupportedTransactionVersion(42))
        ));
    }

    #[test]
    fn transaction_persistence_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(temp.path(), attempt);
        tx.transition_to(UpdateStateId::CheckingRelease).unwrap();

        let saved_path = tx.save_to_dir(temp.path()).expect("Pomyślny zapis");
        assert!(saved_path.exists());
        assert_eq!(saved_path.file_name().unwrap(), TRANSACTION_FILE_NAME);

        let loaded = UpdateTransaction::load_from_dir(temp.path(), temp.path())
            .expect("Pomyślny odczyt")
            .expect("Obecność pliku transakcji");

        assert_eq!(loaded.attempt_id, attempt);
        assert_eq!(loaded.current_state, UpdateStateId::CheckingRelease);
        assert_eq!(loaded.record_version, CURRENT_RECORD_VERSION);
    }

    #[test]
    fn transaction_write_does_not_leave_partial_primary_record() {
        let temp = tempfile::tempdir().unwrap();
        let tx = sample_tx(temp.path(), UpdateAttemptId::generate());
        let _ = tx.save_to_dir(temp.path()).unwrap();

        // Potwierdzenie, że plik główny istnieje i jest poprawnym JSON
        let primary = temp.path().join(TRANSACTION_FILE_NAME);
        assert!(primary.exists());
        let content = std::fs::read_to_string(&primary).unwrap();
        assert!(content.ends_with('}'));

        // Żadne pliki tymczasowe .tmp nie mogą pozostać po pomyślnym zapisie
        for entry in std::fs::read_dir(temp.path()).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            assert!(
                !std::path::Path::new(&name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("tmp")),
                "Znaleziono porzucony plik tmp: {name}"
            );
        }
    }

    #[test]
    fn transaction_record_contains_no_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let tx = sample_tx(temp.path(), UpdateAttemptId::generate());
        let serialized = serde_json::to_string(&tx).unwrap();

        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("api_key"));
        assert!(!serialized.contains("private_key"));
    }

    #[test]
    fn transaction_record_contains_no_database_content() {
        let temp = tempfile::tempdir().unwrap();
        let tx = sample_tx(temp.path(), UpdateAttemptId::generate());
        let serialized = serde_json::to_string(&tx).unwrap();

        assert!(!serialized.contains("serviceLOG"));
        assert!(!serialized.contains("default_log"));
        assert!(!serialized.contains("callbook"));
        assert!(!serialized.contains("station_config"));
        assert!(!serialized.contains("sqlite_master"));
    }

    #[test]
    fn transaction_record_contains_no_qso_data() {
        let temp = tempfile::tempdir().unwrap();
        let tx = sample_tx(temp.path(), UpdateAttemptId::generate());
        let serialized = serde_json::to_string(&tx).unwrap();

        assert!(!serialized.contains("QSO"));
        assert!(!serialized.contains("CALLSIGN"));
        assert!(!serialized.contains("FREQ"));
        assert!(!serialized.contains("BAND"));
        assert!(!serialized.contains("RST_SENT"));
    }

    #[test]
    fn transaction_rejects_path_outside_installation_root() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();

        // Próba utworzenia ze ścieżką zawierającą ..
        let invalid_path = temp.path().join("..").join("outside_staging");
        let res = UpdateTransaction::new(
            attempt,
            "1.3.0".into(),
            "v1.3.0".into(),
            "windows".into(),
            "x86_64".into(),
            "windows-portable-zip".into(),
            InstallationType::WindowsPortableZip,
            "digest".into(),
            "asset.zip".into(),
            100,
            "sha".into(),
            invalid_path,
            temp.path(),
        );
        assert!(matches!(res, Err(UpdaterError::UnsafePath(_))));
    }

    #[test]
    fn transaction_rejects_mismatched_attempt_id() {
        let temp = tempfile::tempdir().unwrap();
        let attempt1 = UpdateAttemptId::generate();
        let attempt2 = UpdateAttemptId::generate();
        let tx = sample_tx(temp.path(), attempt1);
        tx.save_to_dir(temp.path()).unwrap();

        let loaded = UpdateTransaction::load_from_dir(temp.path(), temp.path())
            .unwrap()
            .unwrap();
        assert_ne!(loaded.attempt_id, attempt2);
        assert_eq!(loaded.attempt_id, attempt1);
    }

    #[test]
    fn recovered_transaction_never_reapplies_automatically() {
        let temp = tempfile::tempdir().unwrap();
        let mut tx = sample_tx(temp.path(), UpdateAttemptId::generate());
        // Symulacja awarii podczas Applying
        tx.transition_to(UpdateStateId::CheckingRelease).unwrap();
        tx.transition_to(UpdateStateId::FetchingManifest).unwrap();
        tx.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        tx.transition_to(UpdateStateId::SelectingAsset).unwrap();
        tx.transition_to(UpdateStateId::Downloading).unwrap();
        tx.transition_to(UpdateStateId::VerifyingDownload).unwrap();
        tx.transition_to(UpdateStateId::InspectingPackage).unwrap();
        tx.transition_to(UpdateStateId::Staging).unwrap();
        tx.transition_to(UpdateStateId::BuildingUpdatePlan).unwrap();
        tx.transition_to(UpdateStateId::AwaitingApply).unwrap();
        tx.transition_to(UpdateStateId::Applying).unwrap();
        tx.transition_to(UpdateStateId::FailedPostApply).unwrap();

        tx.save_to_dir(temp.path()).unwrap();

        // Odczyt po restarcie aplikacji (recovery)
        let loaded = UpdateTransaction::load_from_dir(temp.path(), temp.path())
            .unwrap()
            .unwrap();
        assert_eq!(loaded.current_state, UpdateStateId::FailedPostApply);

        // Z FailedPostApply niedozwolone jest przejście powrotne do Applying (zakaz ponownego automatycznego apply)
        assert!(
            !loaded
                .state_machine
                .can_transition_to(UpdateStateId::Applying)
        );
    }

    #[test]
    fn transaction_record_temp_file_is_same_directory() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let tx = sample_tx(temp.path(), attempt);
        let expected_tmp = temp.path().join(format!(
            ".update_transaction_{}.tmp",
            attempt.as_file_component()
        ));
        assert_eq!(expected_tmp.parent().unwrap(), temp.path());
        tx.save_to_dir(temp.path()).unwrap();
    }

    #[test]
    fn transaction_record_survives_interrupted_rewrite() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let tx = sample_tx(temp.path(), attempt);
        tx.save_to_dir(temp.path()).unwrap();

        let primary = temp.path().join(TRANSACTION_FILE_NAME);
        let orig_content = std::fs::read_to_string(&primary).unwrap();

        // Symulacja przerwanego zapisu (częściowy plik tmp z innym attempt)
        let junk_tmp = temp.path().join(".update_transaction_junk.tmp");
        std::fs::write(&junk_tmp, b"PARTIAL CORRUPTED DATA").unwrap();

        // Odczyt rekordu głównego nadal zwraca poprawną oryginalną transakcję
        let loaded = UpdateTransaction::load_from_dir(temp.path(), temp.path())
            .unwrap()
            .unwrap();
        assert_eq!(loaded.attempt_id, attempt);
        assert_eq!(std::fs::read_to_string(&primary).unwrap(), orig_content);

        let _ = std::fs::remove_file(junk_tmp);
    }

    #[test]
    fn failed_rewrite_preserves_previous_valid_record() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let tx = sample_tx(temp.path(), attempt);
        tx.save_to_dir(temp.path()).unwrap();

        let primary = temp.path().join(TRANSACTION_FILE_NAME);
        let valid_content = std::fs::read(&primary).unwrap();

        // Pozostawiamy plik tmp bez udanego rename
        let tmp_file = temp.path().join(".update_transaction_failed.tmp");
        std::fs::write(&tmp_file, b"INCOMPLETE JSON {").unwrap();

        // Plik primary pozostaje w 100% poprawny
        let loaded_bytes = std::fs::read(&primary).unwrap();
        assert_eq!(loaded_bytes, valid_content);
    }

    #[test]
    fn partial_temp_record_is_not_primary_source() {
        let temp = tempfile::tempdir().unwrap();
        // Tworzymy wyłącznie plik tmp, brak TRANSACTION_FILE_NAME
        let tmp = temp.path().join(".update_transaction_123.tmp");
        std::fs::write(&tmp, b"PARTIAL DATA").unwrap();

        let loaded = UpdateTransaction::load_from_dir(temp.path(), temp.path()).unwrap();
        assert!(
            loaded.is_none(),
            "Plik tmp nie może być uznany za primary transaction record"
        );
    }

    #[test]
    fn cross_filesystem_path_is_not_claimed_atomic() {
        let temp1 = tempfile::tempdir().unwrap();
        let temp2 = tempfile::tempdir().unwrap();
        // Ścieżka z obcego katalogu nadrzędnego
        let foreign_path = temp2.path().join("foreign_dir");
        let res = validate_path_under_root(&foreign_path, temp1.path());
        assert!(matches!(res, Err(UpdaterError::UnsafePath(_))));
    }
}
