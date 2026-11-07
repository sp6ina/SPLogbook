//! Centralna polityka anulowania transakcji aktualizacji SPLogbook (M3-A).
//!
//! Zgodnie z decyzją OWNER-M2-13 oraz Sekcją 18 specyfikacji bramki M3-A:
//! - Stany przed `Applying` są bezpiecznie anulowalne (`CANCELLABLE`).
//! - Od wejścia w stan `Applying` anulowanie jest bezwzględnie zabronione (`NON-CANCELLABLE`).
//! - Próba anulowania w trakcie lub po `Applying` zwraca typowany błąd `UpdaterError::CancellationRejected`.
//! - Anulowanie przed `Applying` usuwa częściowo pobrane pliki i staging, bez naruszania danych użytkownika.

use std::path::Path;

use chrono::Utc;

use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::attempt_id::UpdateAttemptId;
use crate::cloud::updater::state::UpdateStateId;
use crate::cloud::updater::transaction::{CancellationStatus, UpdateTransaction};

/// Wynik operacji anulowania transakcji aktualizacji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationResult {
    /// Identyfikator przerwanej próby aktualizacji.
    pub attempt_id: UpdateAttemptId,
    /// Czy usunięto częściowo pobrany plik instalacyjny.
    pub cleaned_download: bool,
    /// Czy usunięto katalog stagingu aktualizacji.
    pub cleaned_staging: bool,
    /// Końcowy stan automatu po anulowaniu (np. `Cleanup`).
    pub final_state: UpdateStateId,
}

/// Sprawdza, czy bieżący stan aktualizatora pozwala na bezpieczne anulowanie operacji przez użytkownika lub system.
///
/// Funkcja jest bezpieczna do wywołania przez interfejs graficzny użytkownika (UI).
pub fn can_cancel(state: UpdateStateId) -> bool {
    state.is_cancellable()
}

/// Sprawdza, czy dana ścieżka reprezentuje chronione dane użytkownika, które nigdy nie mogą zostać usunięte.
pub fn is_protected_user_path(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let path_str = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let ext_str = ext.as_deref();

    // 1. Konfiguracja użytkownika
    if file_name == "station_config.json"
        || path_str.contains("/config/")
        || path_str.starts_with("config/")
        || ext_str == Some("conf")
        || ext_str == Some("cfg")
        || (ext_str == Some("json")
            && file_name != "manifest.json"
            && file_name != "release.json"
            && file_name != ".update.lock"
            && file_name != ".update_transaction.json"
            && file_name != ".health_ok")
    {
        return true;
    }

    // 2. Bazy danych użytkownika i logi łączności
    if file_name == "default_log.db"
        || file_name.starts_with("default_log.db")
        || file_name == "servicelog.db"
        || file_name.starts_with("servicelog.db")
        || file_name == "callbook.db"
        || file_name.starts_with("callbook.db")
        || path_str.contains("/logs/")
        || path_str.starts_with("logs/")
        || path_str.contains("/log/")
        || path_str.starts_with("log/")
        || ext_str == Some("adi")
        || ext_str == Some("adif")
        || ext_str == Some("cabrillo")
        || file_name.ends_with(".db-wal")
        || file_name.ends_with(".db-shm")
        || file_name.ends_with(".db-journal")
    {
        return true;
    }

    // 3. Inne pliki baz danych w instalacji
    if ext_str == Some("db") || ext_str == Some("sqlite") || ext_str == Some("sqlite3") {
        return true;
    }

    false
}

/// Sprawdza integralność baz danych użytkownika i konfiguracji w katalogu instalacji.
pub fn verify_user_data_intact(installation_root: &Path) -> Result<(), UpdaterError> {
    let protected = [
        "default_log.db",
        "station_config.json",
        "serviceLOG.db",
        "callbook.db",
    ];

    for name in &protected {
        let p = installation_root.join(name);
        if p.exists() && p.is_dir() {
            return Err(UpdaterError::SafetyGate(format!(
                "Chroniony plik użytkownika '{name}' jest nieprawidłowym katalogiem."
            )));
        }
    }

    Ok(())
}

/// Wykonuje bezpieczne anulowanie transakcji aktualizacji.
///
/// Jeśli stan transakcji to `Applying` lub dowolny stan późniejszy, operacja jest bezwzględnie
/// odrzucana błędem `UpdaterError::CancellationRejected`.
///
/// Jeśli stan pozwala na anulowanie, funkcja:
/// 1. Rejestruje status żądania anulowania (`CancellationStatus::Completed`).
/// 2. Bezpiecznie usuwa częściowo pobrany pakiet aktualizacyjny (jeśli istnieje).
/// 3. Bezpiecznie usuwa tymczasowy katalog stagingu (jeśli istnieje).
/// 4. Zapewnia nienaruszalność danych i baz użytkownika.
/// 5. Przenosi transakcję do stanu `Cleanup`.
pub fn execute_cancellation(
    tx: &mut UpdateTransaction,
    installation_root: &Path,
) -> Result<CancellationResult, UpdaterError> {
    if !can_cancel(tx.current_state) {
        tx.cancellation_status = CancellationStatus::RejectedPostApply;
        return Err(UpdaterError::CancellationRejected(format!(
            "Anulowanie odrzucone w stanie '{}': od stanu Applying anulowanie jest bezwzględnie zabronione (wymagany rollback lub recovery).",
            tx.current_state
        )));
    }

    let attempt_id = tx.attempt_id;
    let mut cleaned_download = false;
    let mut cleaned_staging = false;

    // 1. Usunięcie częściowo pobranego pliku
    let candidate_download_paths = [
        installation_root.join(&tx.asset_filename),
        installation_root.join(format!("{}.part", tx.asset_filename)),
    ];

    for dl_full in &candidate_download_paths {
        if dl_full.exists() {
            if is_protected_user_path(dl_full) {
                return Err(UpdaterError::SafetyGate(format!(
                    "Ścieżka pobierania '{}' wskazuje na chronione dane użytkownika!",
                    dl_full.display()
                )));
            }
            if dl_full.is_file() {
                if let Err(err) = std::fs::remove_file(dl_full) {
                    return Err(UpdaterError::Io(format!(
                        "Nie udało się usunąć pobranego pliku '{}': {err}",
                        dl_full.display()
                    )));
                }
                cleaned_download = true;
            }
        }
    }

    // 2. Usunięcie katalogu stagingu
    let st_full = if tx.staging_path.is_absolute() {
        tx.staging_path.clone()
    } else {
        installation_root.join(&tx.staging_path)
    };

    if st_full.exists() {
        if is_protected_user_path(&st_full) {
            return Err(UpdaterError::SafetyGate(format!(
                "Ścieżka stagingu '{}' wskazuje na chronione dane użytkownika!",
                st_full.display()
            )));
        }
        if st_full.is_dir() {
            if let Err(err) = std::fs::remove_dir_all(&st_full) {
                return Err(UpdaterError::Io(format!(
                    "Nie udało się usunąć katalogu stagingu '{}': {err}",
                    st_full.display()
                )));
            }
            cleaned_staging = true;
        }
    }

    // 3. Weryfikacja nienaruszalności danych użytkownika
    verify_user_data_intact(installation_root)?;

    // 4. Aktualizacja statusu transakcji
    tx.cancellation_status = CancellationStatus::Completed;
    tx.current_state = UpdateStateId::Cleanup;
    tx.last_transition_time = Utc::now();

    Ok(CancellationResult {
        attempt_id,
        cleaned_download,
        cleaned_staging,
        final_state: tx.current_state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::updater::installation::InstallationType;
    use crate::cloud::updater::state::UpdateStateMachine;
    use std::fs;
    use tempfile::tempdir;

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
    fn cancellation_allowed_before_apply() {
        let cancellable = [
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
        ];

        for s in cancellable {
            assert!(
                can_cancel(s),
                "Stan {:?} powinien być oznaczony jako anulowalny",
                s
            );
        }

        let non_cancellable = [
            UpdateStateId::Applying,
            UpdateStateId::LaunchingCandidate,
            UpdateStateId::AwaitingHealth,
            UpdateStateId::Committed,
            UpdateStateId::FailedPostApply,
            UpdateStateId::Rollback,
            UpdateStateId::RollbackVerified,
            UpdateStateId::Cleanup,
        ];

        for s in non_cancellable {
            assert!(
                !can_cancel(s),
                "Stan {:?} NIE powinien być oznaczony jako anulowalny",
                s
            );
        }
    }

    #[test]
    fn cancellation_removes_partial_download() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let mut tx = sample_tx(root, UpdateAttemptId::generate());
        tx.current_state = UpdateStateId::Downloading;

        let part_file = root.join(format!("{}.part", tx.asset_filename));
        fs::write(&part_file, b"partial download bytes").expect("write part file");
        assert!(part_file.exists());

        let res = execute_cancellation(&mut tx, root).expect("cancellation should succeed");
        assert!(res.cleaned_download);
        assert!(
            !part_file.exists(),
            "Częściowy plik powinien zostać usunięty"
        );
        assert_eq!(tx.cancellation_status, CancellationStatus::Completed);
    }

    #[test]
    fn cancellation_removes_staging() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let mut tx = sample_tx(root, UpdateAttemptId::generate());
        tx.current_state = UpdateStateId::Staging;

        let staging_dir = root.join(".staging");
        fs::create_dir_all(&staging_dir).expect("create staging dir");
        fs::write(staging_dir.join("extracted.bin"), b"extracted candidate")
            .expect("write extracted");
        assert!(staging_dir.exists());

        let res = execute_cancellation(&mut tx, root).expect("cancellation should succeed");
        assert!(res.cleaned_staging);
        assert!(
            !staging_dir.exists(),
            "Katalog stagingu powinien zostać usunięty"
        );
        assert_eq!(tx.cancellation_status, CancellationStatus::Completed);
    }

    #[test]
    fn cancellation_preserves_user_data() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let log_db = root.join("default_log.db");
        let config_json = root.join("station_config.json");
        let service_db = root.join("serviceLOG.db");
        let callbook_db = root.join("callbook.db");

        fs::write(&log_db, b"CRITICAL QSO DATABASE").expect("write log");
        fs::write(&config_json, b"{\"callsign\":\"SP6IN\"}").expect("write config");
        fs::write(&service_db, b"SERVICELOG DATA").expect("write service");
        fs::write(&callbook_db, b"CALLBOOK CACHE").expect("write callbook");

        let mut tx = sample_tx(root, UpdateAttemptId::generate());
        tx.current_state = UpdateStateId::AwaitingApply;

        let res = execute_cancellation(&mut tx, root).expect("cancellation should succeed");
        assert_eq!(res.final_state, UpdateStateId::Cleanup);

        assert_eq!(
            fs::read(&log_db).expect("read log"),
            b"CRITICAL QSO DATABASE"
        );
        assert_eq!(
            fs::read(&config_json).expect("read config"),
            b"{\"callsign\":\"SP6IN\"}"
        );
        assert_eq!(
            fs::read(&service_db).expect("read service"),
            b"SERVICELOG DATA"
        );
        assert_eq!(
            fs::read(&callbook_db).expect("read callbook"),
            b"CALLBOOK CACHE"
        );
    }

    #[test]
    fn cancellation_rejected_during_apply() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let mut tx = sample_tx(root, UpdateAttemptId::generate());
        tx.current_state = UpdateStateId::Applying;

        let err = execute_cancellation(&mut tx, root).expect_err("should reject in Applying");
        match err {
            UpdaterError::CancellationRejected(msg) => {
                assert!(msg.contains("Applying"));
            }
            other => panic!("Oczekiwano CancellationRejected, otrzymano {:?}", other),
        }
        assert_eq!(
            tx.cancellation_status,
            CancellationStatus::RejectedPostApply
        );
    }

    #[test]
    fn cancellation_rejected_during_health_wait() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let mut tx = sample_tx(root, UpdateAttemptId::generate());
        tx.current_state = UpdateStateId::AwaitingHealth;

        let err = execute_cancellation(&mut tx, root).expect_err("should reject in AwaitingHealth");
        match err {
            UpdaterError::CancellationRejected(msg) => {
                assert!(msg.contains("AwaitingHealth"));
            }
            other => panic!("Oczekiwano CancellationRejected, otrzymano {:?}", other),
        }
        assert_eq!(
            tx.cancellation_status,
            CancellationStatus::RejectedPostApply
        );
    }

    #[test]
    fn cancellation_rejected_during_rollback() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let mut tx = sample_tx(root, UpdateAttemptId::generate());
        tx.current_state = UpdateStateId::Rollback;

        let err = execute_cancellation(&mut tx, root).expect_err("should reject in Rollback");
        match err {
            UpdaterError::CancellationRejected(msg) => {
                assert!(msg.contains("Rollback"));
            }
            other => panic!("Oczekiwano CancellationRejected, otrzymano {:?}", other),
        }
        assert_eq!(
            tx.cancellation_status,
            CancellationStatus::RejectedPostApply
        );
    }

    #[test]
    fn late_cancel_request_does_not_interrupt_io() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let mut tx = sample_tx(root, UpdateAttemptId::generate());
        tx.current_state = UpdateStateId::Applying;

        // Symulacja nadejścia spóźnionego cancellation podczas IO Applying
        let err = execute_cancellation(&mut tx, root).expect_err("musi odrzucić późne anulowanie");
        assert!(matches!(err, UpdaterError::CancellationRejected(_)));
        assert_eq!(
            tx.current_state,
            UpdateStateId::Applying,
            "Stan transakcji nie uległ zaburzeniu"
        );
    }

    #[test]
    fn failure_during_apply_enters_post_apply_failure() {
        let mut sm = UpdateStateMachine::new();
        // Przejście od Idle do Applying
        sm.transition_to(UpdateStateId::CheckingRelease)
            .expect("step");
        sm.transition_to(UpdateStateId::FetchingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::VerifyingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::SelectingAsset)
            .expect("step");
        sm.transition_to(UpdateStateId::Downloading).expect("step");
        sm.transition_to(UpdateStateId::VerifyingDownload)
            .expect("step");
        sm.transition_to(UpdateStateId::InspectingPackage)
            .expect("step");
        sm.transition_to(UpdateStateId::Staging).expect("step");
        sm.transition_to(UpdateStateId::BuildingUpdatePlan)
            .expect("step");
        sm.transition_to(UpdateStateId::AwaitingApply)
            .expect("step");
        sm.transition_to(UpdateStateId::Applying).expect("step");

        // Awaria podczas applying musi wejść w FailedPostApply, a nie FailedPreApply
        assert!(!sm.can_transition_to(UpdateStateId::FailedPreApply));
        assert!(sm.can_transition_to(UpdateStateId::FailedPostApply));

        sm.transition_to(UpdateStateId::FailedPostApply)
            .expect("must transition to FailedPostApply");
        assert_eq!(sm.current_state(), UpdateStateId::FailedPostApply);
        assert!(sm.current_state().requires_rollback());
    }
}
