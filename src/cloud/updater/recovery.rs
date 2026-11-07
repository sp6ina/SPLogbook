//! Silnik decyzji recovery dla przerwanych transakcji aktualizacji SPLogbook (M3-A).
//!
//! Zgodnie z decyzjami OWNER-M2-14, OWNER-M2-15 oraz Sekcją 22 specyfikacji bramki:
//! - Silnik decyzji (`RecoveryDecisionEngine`) jest czystą logiką biznesową (zero niekontrolowanego I/O).
//! - Bezwzględny zakaz automatycznego ponawiania `apply` lub `download` (Zero Automatic Re-apply).
//! - Przed `Applying` następuje wyłącznie bezpieczne sprzątanie własnych artefaktów (`CleanupPreApplyArtifacts`).
//! - Od `Applying` wymagany jest rollback lub delegacja do zarządcy pakietów (`RequireRollback`, `DelegateToManagedInstallerRecovery`).
//! - Bazy danych użytkownika (`default_log.db`, `station_config.json`, `serviceLOG.db`, `callbook.db`) są bezwzględnie chronione.
//! - Obce próby aktualizacji (foreign attempt) nie są przejmowane.
//! - Całość ewaluacji jest w pełni idempotentna i deterministyczna.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::attempt_id::UpdateAttemptId;
use crate::cloud::updater::cancellation::is_protected_user_path;
use crate::cloud::updater::installation::{InstallationType, ManagementMode};
use crate::cloud::updater::state::UpdateStateId;
use crate::cloud::updater::transaction::UpdateTransaction;

/// Możliwe decyzje mechanizmu recovery zgodnie z Sekcją 22 specyfikacji M3-A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryDecision {
    /// Brak konieczności podejmowania jakichkolwiek działań recovery.
    NoRecoveryNeeded,
    /// Usunięcie artefaktów tymczasowych pobierania i stagingu przed fazą podmiany plików.
    CleanupPreApplyArtifacts,
    /// Bezwzględny wymóg przywrócenia oryginalnych plików aplikacji z kopii zapasowej.
    RequireRollback,
    /// Wznowienie niedokończonej procedury wycofywania zmian (Rollback).
    ResumeRollback,
    /// Weryfikacja spójności przywróconych plików programu po rollbacku.
    VerifyRollback,
    /// Usunięcie plików stagingu i blokad po pomyślnym zatwierdzeniu wydania (`Committed`).
    CleanupAfterCommitted,
    /// Wymagana ręczna interwencja użytkownika (błąd krytyczny / podejrzenie sabotażu / uszkodzenie blokady).
    ManualInterventionRequired,
    /// Przekazanie procedury recovery do zewnętrznego zarządcy pakietów (SystemManaged).
    DelegateToManagedInstallerRecovery,
}

/// Szczegółowy plan naprawczy wygenerowany przez silnik decyzji.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// Główna decyzja naprawcza.
    pub decision: RecoveryDecision,
    /// Czytelne uzasadnienie audytowe podjętej decyzji.
    pub reason: String,
    /// Czy decyzja wymaga zgłoszenia użytkownikowi lub interwencji ręcznej.
    pub requires_user_intervention: bool,
    /// Docelowy stan automatu, do którego transakcja powinna zostać sprowadzona.
    pub target_state: Option<UpdateStateId>,
}

/// Kontekst operacyjny przekazywany do silnika decyzji recovery.
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    /// Oczekiwany identyfikator aktywnej próby (jeśli istnieje).
    pub expected_attempt_id: Option<UpdateAttemptId>,
    /// Odczytana utrwalona transakcja z dysku (jeśli istnieje).
    pub transaction: Option<UpdateTransaction>,
    /// Identyfikator próby zapisany w pliku blokady `.update.lock` (jeśli plik istnieje).
    pub lock_owner: Option<UpdateAttemptId>,
    /// Czy plik blokady jest uszkodzony (np. niepoprawny JSON).
    pub lock_is_corrupt: bool,
    /// Czy katalog stagingu istnieje na dysku.
    pub staging_exists: bool,
    /// Czy katalog kopii zapasowej (`.rollback`) istnieje na dysku.
    pub backup_exists: bool,
    /// Czy dowód poprawności działania (.health_ok) został pomyślnie zweryfikowany.
    pub health_proof_valid: bool,
    /// Wykryty rodzaj instalacji.
    pub installation_type: InstallationType,
}

/// Deterministyczny silnik podejmowania decyzji recovery.
pub struct RecoveryDecisionEngine;

impl RecoveryDecisionEngine {
    /// Dokonuje deterministycznej ewaluacji kontekstu i zwraca plan naprawczy.
    ///
    /// Ta metoda nie wykonuje żadnych operacji wejścia/wyjścia (I/O).
    pub fn evaluate(ctx: &RecoveryContext) -> RecoveryPlan {
        // 1. Uszkodzona blokada wymaga interwencji ręcznej (fail-closed)
        if ctx.lock_is_corrupt {
            return RecoveryPlan {
                decision: RecoveryDecision::ManualInterventionRequired,
                reason: "Plik blokady współbieżności jest uszkodzony lub nieczytelny — wymagana interwencja w celu uniknięcia wyścigu.".to_string(),
                requires_user_intervention: true,
                target_state: None,
            };
        }

        // 2. Nieznany typ instalacji jest traktowany bezpiecznie (Fail-closed)
        if ctx.installation_type == InstallationType::UnknownInstallation {
            return RecoveryPlan {
                decision: RecoveryDecision::ManualInterventionRequired,
                reason: "Nieznany format instalacji — brak bezpiecznej procedury automatycznego recovery.".to_string(),
                requires_user_intervention: true,
                target_state: None,
            };
        }

        // 3. Brak transakcji lub stan Idle
        let Some(tx) = &ctx.transaction else {
            return RecoveryPlan {
                decision: RecoveryDecision::NoRecoveryNeeded,
                reason: "Brak zapisanej transakcji aktualizacji na dysku.".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Idle),
            };
        };

        if tx.current_state == UpdateStateId::Idle {
            return RecoveryPlan {
                decision: RecoveryDecision::NoRecoveryNeeded,
                reason: "Zapisana transakcja znajduje się w stanie spoczynku (Idle).".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Idle),
            };
        }

        // 3. Obca transakcja nie jest przejmowana (Foreign Attempt Protection)
        if let Some(expected) = ctx.expected_attempt_id
            && tx.attempt_id != expected
        {
            return RecoveryPlan {
                decision: RecoveryDecision::ManualInterventionRequired,
                reason: format!(
                    "Transakcja na dysku należy do innej próby ({}) niż bieżąca ({}). Obca transakcja nie jest przejmowana.",
                    tx.attempt_id, expected
                ),
                requires_user_intervention: true,
                target_state: None,
            };
        }

        // Sprawdzenie zgodności właściciela blokady
        if let Some(lock_owner) = ctx.lock_owner
            && lock_owner != tx.attempt_id
        {
            return RecoveryPlan {
                decision: RecoveryDecision::ManualInterventionRequired,
                reason: format!(
                    "Plik blokady należy do innej próby ({}) niż transakcja ({}). Obca blokada nie może zostać zwolniona.",
                    lock_owner, tx.attempt_id
                ),
                requires_user_intervention: true,
                target_state: None,
            };
        }

        // 4. Obsługa instalacji zarządzanych systemowo (SystemManaged)
        if ctx.installation_type.management_mode() == ManagementMode::SystemManaged {
            if tx.current_state.is_post_apply() || tx.current_state == UpdateStateId::Applying {
                return RecoveryPlan {
                    decision: RecoveryDecision::DelegateToManagedInstallerRecovery,
                    reason: format!(
                        "Format instalacji '{}' jest zarządzany systemowo — recovery po uruchomieniu instalatora delegowane do menedżera pakietów.",
                        ctx.installation_type
                    ),
                    requires_user_intervention: true,
                    target_state: Some(UpdateStateId::Cleanup),
                };
            }
            return RecoveryPlan {
                decision: RecoveryDecision::CleanupPreApplyArtifacts,
                reason: "Anulowanie lub awaria przed uruchomieniem systemowego instalatora — sprzątanie artefaktów tymczasowych.".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Cleanup),
            };
        }

        // 5. Ewaluacja stanów SelfManaged w oparciu o fazę cyklu życia
        match tx.current_state {
            // Faza 1: Przed rozpoczęciem podmiany plików programu (Pre-Apply)
            UpdateStateId::CheckingRelease
            | UpdateStateId::FetchingManifest
            | UpdateStateId::VerifyingManifest
            | UpdateStateId::SelectingAsset
            | UpdateStateId::Downloading
            | UpdateStateId::VerifyingDownload
            | UpdateStateId::InspectingPackage
            | UpdateStateId::Staging
            | UpdateStateId::BuildingUpdatePlan
            | UpdateStateId::AwaitingApply
            | UpdateStateId::FailedPreApply => RecoveryPlan {
                decision: RecoveryDecision::CleanupPreApplyArtifacts,
                reason: format!(
                    "Przerwanie w stanie pre-apply '{}'. Pliki programu są nienaruszone — sprzątanie wyłącznie artefaktów pobierania i stagingu.",
                    tx.current_state
                ),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Cleanup),
            },

            // Faza 2: W trakcie lub bezpośrednio po podmianie plików programu
            UpdateStateId::Applying | UpdateStateId::FailedPostApply => RecoveryPlan {
                decision: RecoveryDecision::RequireRollback,
                reason: format!(
                    "Przerwanie w stanie '{}'. Pliki programu mogły ulec częściowej podmianie — bezwzględnie wymagany Rollback.",
                    tx.current_state
                ),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Rollback),
            },

            // Faza 3: Test kandydata i oczekiwanie na dowód zdrowia (Health Proof)
            UpdateStateId::LaunchingCandidate | UpdateStateId::AwaitingHealth => {
                if ctx.health_proof_valid {
                    RecoveryPlan {
                        decision: RecoveryDecision::CleanupAfterCommitted,
                        reason: "Potwierdzono poprawny dowód zdrowia kandydata — transakcja może zostać bezpiecznie sfinalizowana.".to_string(),
                        requires_user_intervention: false,
                        target_state: Some(UpdateStateId::Committed),
                    }
                } else {
                    RecoveryPlan {
                        decision: RecoveryDecision::RequireRollback,
                        reason: "Brak poprawnego dowodu zdrowia nowego procesu po przekroczeniu limitu czasu — wymagany Rollback do poprzedniej wersji.".to_string(),
                        requires_user_intervention: false,
                        target_state: Some(UpdateStateId::Rollback),
                    }
                }
            }

            // Faza 4: Wznawianie i weryfikacja rollbacku
            UpdateStateId::Rollback => RecoveryPlan {
                decision: RecoveryDecision::ResumeRollback,
                reason: "Przerwanie podczas procedury rollbacku — wznowienie przywracania oryginalnych plików.".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Rollback),
            },

            UpdateStateId::RollbackVerified => RecoveryPlan {
                decision: RecoveryDecision::VerifyRollback,
                reason: "Rollback został pomyślnie ukończony i zweryfikowany — przejście do usunięcia plików tymczasowych.".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Cleanup),
            },

            // Faza 5: Końcowe sprzątanie po sukcesie
            UpdateStateId::Committed => RecoveryPlan {
                decision: RecoveryDecision::CleanupAfterCommitted,
                reason: "Wydanie zostało zatwierdzone — usunięcie plików stagingu i blokad bez ruszania plików binarnych i baz danych.".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Cleanup),
            },

            UpdateStateId::Cleanup => RecoveryPlan {
                decision: RecoveryDecision::CleanupPreApplyArtifacts,
                reason: "Dokończenie sprzątania pozostałych artefaktów transakcji.".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Idle),
            },

            UpdateStateId::Idle => RecoveryPlan {
                decision: RecoveryDecision::NoRecoveryNeeded,
                reason: "Transakcja w stanie spoczynku (Idle).".to_string(),
                requires_user_intervention: false,
                target_state: Some(UpdateStateId::Idle),
            },
        }
    }

    /// Bezpiecznie wykonuje procedurę oczyszczania artefaktów transakcji, gwarantując
    /// nienaruszalność baz danych i konfiguracji użytkownika.
    pub fn execute_cleanup(
        installation_root: &Path,
        tx: &UpdateTransaction,
    ) -> Result<Vec<PathBuf>, UpdaterError> {
        let mut cleaned_items = Vec::new();

        // 1. Sprzątanie plików pobierania
        let candidate_download_paths = [
            installation_root.join(&tx.asset_filename),
            installation_root.join(format!("{}.part", tx.asset_filename)),
        ];

        for p in &candidate_download_paths {
            if p.exists() {
                if is_protected_user_path(p) {
                    return Err(UpdaterError::SafetyGate(format!(
                        "Próba usunięcia chronionego pliku użytkownika w trakcie recovery: {}",
                        p.display()
                    )));
                }
                if p.is_file() {
                    let _ = std::fs::remove_file(p);
                    cleaned_items.push(p.clone());
                }
            }
        }

        // 2. Sprzątanie katalogu stagingu
        let st_full = if tx.staging_path.is_absolute() {
            tx.staging_path.clone()
        } else {
            installation_root.join(&tx.staging_path)
        };

        if st_full.exists() {
            if is_protected_user_path(&st_full) {
                return Err(UpdaterError::SafetyGate(format!(
                    "Próba usunięcia chronionego katalogu użytkownika w trakcie recovery: {}",
                    st_full.display()
                )));
            }
            if st_full.is_dir() {
                let _ = std::fs::remove_dir_all(&st_full);
                cleaned_items.push(st_full);
            }
        }

        Ok(cleaned_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn sample_tx(
        temp_dir: &Path,
        attempt_id: UpdateAttemptId,
        installation_type: InstallationType,
    ) -> UpdateTransaction {
        let staging = temp_dir.join(".staging");
        UpdateTransaction::new(
            attempt_id,
            "1.3.0".into(),
            "v1.3.0".into(),
            "windows".into(),
            "x86_64".into(),
            "windows-portable-zip".into(),
            installation_type,
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
    fn idle_transaction_needs_no_recovery() {
        let ctx = RecoveryContext {
            expected_attempt_id: None,
            transaction: None,
            lock_owner: None,
            lock_is_corrupt: false,
            staging_exists: false,
            backup_exists: false,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::NoRecoveryNeeded);
        assert!(!plan.requires_user_intervention);
    }

    #[test]
    fn pre_apply_failure_requires_cleanup_only() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::WindowsPortableZip);
        tx.current_state = UpdateStateId::FailedPreApply;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: false,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::CleanupPreApplyArtifacts);
        assert!(!plan.requires_user_intervention);
        assert_eq!(plan.target_state, Some(UpdateStateId::Cleanup));
    }

    #[test]
    fn applying_state_requires_rollback() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::WindowsPortableZip);
        tx.current_state = UpdateStateId::Applying;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: true,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::RequireRollback);
        assert_eq!(plan.target_state, Some(UpdateStateId::Rollback));
    }

    #[test]
    fn health_timeout_requires_rollback() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::LinuxPortableTarGz);
        tx.current_state = UpdateStateId::AwaitingHealth;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: true,
            health_proof_valid: false, // Timeout / dowód nie został wygenerowany
            installation_type: InstallationType::LinuxPortableTarGz,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::RequireRollback);
        assert_eq!(plan.target_state, Some(UpdateStateId::Rollback));
    }

    #[test]
    fn rollback_state_resumes_rollback() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::WindowsPortableZip);
        tx.current_state = UpdateStateId::Rollback;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: true,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::ResumeRollback);
        assert_eq!(plan.target_state, Some(UpdateStateId::Rollback));
    }

    #[test]
    fn rollback_verified_requires_cleanup() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::WindowsPortableZip);
        tx.current_state = UpdateStateId::RollbackVerified;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: true,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::VerifyRollback);
        assert_eq!(plan.target_state, Some(UpdateStateId::Cleanup));
    }

    #[test]
    fn committed_state_cleans_only_transaction_artifacts() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::WindowsPortableZip);
        tx.current_state = UpdateStateId::Committed;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: false,
            health_proof_valid: true,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::CleanupAfterCommitted);
        assert_eq!(plan.target_state, Some(UpdateStateId::Cleanup));
    }

    #[test]
    fn unknown_state_is_fail_closed() {
        let attempt = UpdateAttemptId::generate();
        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: None,
            lock_owner: None,
            lock_is_corrupt: true, // Uszkodzona blokada
            staging_exists: false,
            backup_exists: false,
            health_proof_valid: false,
            installation_type: InstallationType::UnknownInstallation,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::ManualInterventionRequired);
        assert!(plan.requires_user_intervention);
    }

    #[test]
    fn managed_installation_delegates_recovery() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::LinuxDeb);
        tx.current_state = UpdateStateId::Applying;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: false,
            backup_exists: false,
            health_proof_valid: false,
            installation_type: InstallationType::LinuxDeb,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(
            plan.decision,
            RecoveryDecision::DelegateToManagedInstallerRecovery
        );
        assert!(plan.requires_user_intervention);
    }

    #[test]
    fn recovery_never_reapplies_update() {
        let dir = tempdir().expect("tempdir");
        let all_states = [
            UpdateStateId::Idle,
            UpdateStateId::CheckingRelease,
            UpdateStateId::FetchingManifest,
            UpdateStateId::VerifyingManifest,
            UpdateStateId::SelectingAsset,
            UpdateStateId::Downloading,
            UpdateStateId::VerifyingDownload,
            UpdateStateId::InspectingPackage,
            UpdateStateId::Staging,
            UpdateStateId::BuildingUpdatePlan,
            UpdateStateId::AwaitingApply,
            UpdateStateId::Applying,
            UpdateStateId::LaunchingCandidate,
            UpdateStateId::AwaitingHealth,
            UpdateStateId::Committed,
            UpdateStateId::FailedPreApply,
            UpdateStateId::FailedPostApply,
            UpdateStateId::Rollback,
            UpdateStateId::RollbackVerified,
            UpdateStateId::Cleanup,
        ];

        for state in all_states {
            let attempt = UpdateAttemptId::generate();
            let mut tx = sample_tx(dir.path(), attempt, InstallationType::WindowsPortableZip);
            tx.current_state = state;

            let ctx = RecoveryContext {
                expected_attempt_id: Some(attempt),
                transaction: Some(tx),
                lock_owner: Some(attempt),
                lock_is_corrupt: false,
                staging_exists: true,
                backup_exists: true,
                health_proof_valid: false,
                installation_type: InstallationType::WindowsPortableZip,
            };

            let plan = RecoveryDecisionEngine::evaluate(&ctx);
            // Decyzja nigdy nie może być re-apply / download
            assert_ne!(
                plan.target_state,
                Some(UpdateStateId::Applying),
                "Stan {:?} nie może skutkować automatycznym ponowieniem Applying",
                state
            );
            assert_ne!(
                plan.target_state,
                Some(UpdateStateId::Downloading),
                "Stan {:?} nie może skutkować automatycznym ponowieniem Downloading",
                state
            );
        }
    }

    #[test]
    fn recovery_preserves_user_databases() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let log_db = root.join("default_log.db");
        let service_db = root.join("serviceLOG.db");
        let callbook_db = root.join("callbook.db");
        fs::write(&log_db, b"IMPORTANT LOG").expect("write log");
        fs::write(&service_db, b"IMPORTANT SERVICE").expect("write service");
        fs::write(&callbook_db, b"IMPORTANT CALLBOOK").expect("write callbook");

        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(root, attempt, InstallationType::WindowsPortableZip);
        // Próba złośliwego wskazania logu jako pliku assetu do usunięcia
        tx.asset_filename = "default_log.db".to_string();

        let err = RecoveryDecisionEngine::execute_cleanup(root, &tx)
            .expect_err("musi zablokować usunięcie bazy");
        assert!(matches!(err, UpdaterError::SafetyGate(_)));

        assert!(log_db.exists());
        assert!(service_db.exists());
        assert!(callbook_db.exists());
    }

    #[test]
    fn recovery_preserves_user_configuration() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let cfg = root.join("station_config.json");
        fs::write(&cfg, b"{\"my_call\":\"SP6IN\"}").expect("write cfg");

        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(root, attempt, InstallationType::WindowsPortableZip);
        // Próba złośliwego wskazania konfiguracji jako staging_path
        tx.staging_path = PathBuf::from("station_config.json");

        let err = RecoveryDecisionEngine::execute_cleanup(root, &tx)
            .expect_err("musi zablokować usunięcie konfiguracji");
        assert!(matches!(err, UpdaterError::SafetyGate(_)));

        assert!(cfg.exists());
        assert_eq!(
            fs::read(&cfg).expect("read cfg"),
            b"{\"my_call\":\"SP6IN\"}"
        );
    }

    #[test]
    fn recovery_is_idempotent() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let mut tx = sample_tx(dir.path(), attempt, InstallationType::WindowsPortableZip);
        tx.current_state = UpdateStateId::FailedPostApply;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(attempt),
            transaction: Some(tx),
            lock_owner: Some(attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: true,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let initial_plan = RecoveryDecisionEngine::evaluate(&ctx);
        for _ in 0..10 {
            let next_plan = RecoveryDecisionEngine::evaluate(&ctx);
            assert_eq!(
                initial_plan, next_plan,
                "Recovery engine musi być ściśle idempotentny"
            );
        }
    }

    #[test]
    fn foreign_attempt_is_not_recovered() {
        let dir = tempdir().expect("tempdir");
        let local_attempt = UpdateAttemptId::generate();
        let foreign_attempt = UpdateAttemptId::generate();

        let mut tx = sample_tx(
            dir.path(),
            foreign_attempt,
            InstallationType::WindowsPortableZip,
        );
        tx.current_state = UpdateStateId::Downloading;

        let ctx = RecoveryContext {
            expected_attempt_id: Some(local_attempt),
            transaction: Some(tx),
            lock_owner: Some(foreign_attempt),
            lock_is_corrupt: false,
            staging_exists: true,
            backup_exists: false,
            health_proof_valid: false,
            installation_type: InstallationType::WindowsPortableZip,
        };

        let plan = RecoveryDecisionEngine::evaluate(&ctx);
        assert_eq!(plan.decision, RecoveryDecision::ManualInterventionRequired);
        assert!(plan.requires_user_intervention);
        assert!(plan.reason.contains("inna próba") || plan.reason.contains("innej próby"));
    }
}
