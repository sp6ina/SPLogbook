// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Atomowa blokada współbieżności aktualizatora SPLogbook (M3-A)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::attempt_id::UpdateAttemptId;

/// Nazwa kanonicznego pliku blokady współbieżności aktualizacji.
pub const UPDATE_LOCK_FILENAME: &str = ".update.lock";

/// Wersja schematu rekordu blokady aktualizatora.
pub const UPDATE_LOCK_RECORD_VERSION: u32 = 1;

/// Rekord tożsamości właściciela blokady zapisywany w pliku `.update.lock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateLockRecord {
    pub record_version: u32,
    pub attempt_id: UpdateAttemptId,
    pub pid: u32,
    pub process_start_identity: Option<String>,
    pub installation_identity: String,
    pub creation_time: DateTime<Utc>,
    pub sanitized_executable_identity: String,
}

impl UpdateLockRecord {
    pub fn new(attempt_id: UpdateAttemptId, installation_identity: String) -> Self {
        let exe_name = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "SPLogbook".into());

        Self {
            record_version: UPDATE_LOCK_RECORD_VERSION,
            attempt_id,
            pid: std::process::id(),
            process_start_identity: None,
            installation_identity,
            creation_time: Utc::now(),
            sanitized_executable_identity: exe_name,
        }
    }
}

/// Strażnik nabytej blokady aktualizatora.
///
/// Gwarantuje zwolnienie blokady przy normalnym zakończeniu lub dropie,
/// ale pod warunkiem, że blokada nadal należy do tej samej próby aktualizacji (Attempt ID).
/// Obca blokada nigdy nie zostanie usunięta przez strażnika innej próby.
#[derive(Debug)]
pub struct UpdateLockGuard {
    lock_path: PathBuf,
    attempt_id: UpdateAttemptId,
    is_released: bool,
}

impl UpdateLockGuard {
    /// Nabywa wyłączną blokadę pojedynczej aktualizacji w podanym installation_root.
    ///
    /// Semantyka atomowości:
    /// - Użycie `OpenOptions::new().write(true).create_new(true)` (`O_CREAT | O_EXCL` na POSIX, `CREATE_NEW` na Windows).
    /// - Wyklucza podatności TOCTOU obecne we wzorcach `if !exists() { create() }`.
    /// - Jeśli plik już istnieje, natychmiast zwraca `UpdaterError::LockContended`.
    pub fn acquire(
        installation_root: &Path,
        attempt_id: UpdateAttemptId,
    ) -> Result<Self, UpdaterError> {
        let lock_path = installation_root.join(UPDATE_LOCK_FILENAME);

        let record = UpdateLockRecord::new(attempt_id, installation_root.to_string_lossy().into());
        let record_bytes = serde_json::to_vec_pretty(&record).map_err(|e| {
            UpdaterError::LockCorrupt(format!("Błąd serializacji UpdateLockRecord: {e}"))
        })?;

        // Atomowa próba utworzenia nowego pliku
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Plik blokady istnieje — badamy tożsamość właściciela do celów diagnostycznych
                if let Ok(existing_bytes) = fs::read(&lock_path) {
                    if let Ok(existing_record) =
                        serde_json::from_slice::<UpdateLockRecord>(&existing_bytes)
                    {
                        return Err(UpdaterError::LockContended(format!(
                            "Inna aktualizacja jest aktywna (PID {}, próba {}, rozpoczęta {})",
                            existing_record.pid,
                            existing_record.attempt_id,
                            existing_record.creation_time
                        )));
                    }
                    // Uszkodzony plik blokady — fail-closed
                    return Err(UpdaterError::LockCorrupt(format!(
                        "Plik blokady {} istnieje, lecz zawiera uszkodzone dane",
                        lock_path.display()
                    )));
                }
                return Err(UpdaterError::LockContended(format!(
                    "Plik blokady {} jest zablokowany przez inny proces",
                    lock_path.display()
                )));
            }
            Err(e) => {
                return Err(UpdaterError::Io(format!(
                    "Nie można utworzyć pliku blokady {}: {e}",
                    lock_path.display()
                )));
            }
        };

        if let Err(e) = file.write_all(&record_bytes) {
            let _ = fs::remove_file(&lock_path);
            return Err(UpdaterError::Io(format!(
                "Błąd zapisu rekordu blokady do {}: {e}",
                lock_path.display()
            )));
        }

        if let Err(e) = file.flush() {
            let _ = fs::remove_file(&lock_path);
            return Err(UpdaterError::Io(format!(
                "Błąd flush pliku blokady {}: {e}",
                lock_path.display()
            )));
        }

        if let Err(e) = file.sync_all() {
            let _ = fs::remove_file(&lock_path);
            return Err(UpdaterError::Io(format!(
                "Błąd sync_all pliku blokady {}: {e}",
                lock_path.display()
            )));
        }

        Ok(Self {
            lock_path,
            attempt_id,
            is_released: false,
        })
    }

    /// Jawne, kontrolowane zwolnienie blokady.
    /// Weryfikuje zgodność attempt_id przed usunięciem pliku.
    pub fn release(&mut self) -> Result<(), UpdaterError> {
        if self.is_released {
            return Ok(());
        }

        if self.lock_path.exists() {
            // Weryfikacja, czy plik blokady nadal należy do tej próby
            if let Ok(bytes) = fs::read(&self.lock_path)
                && let Ok(record) = serde_json::from_slice::<UpdateLockRecord>(&bytes)
                && record.attempt_id != self.attempt_id
            {
                return Err(UpdaterError::ForeignLock(format!(
                    "Próba zwolnienia obcej blokady: oczekiwano {}, na dysku {}",
                    self.attempt_id, record.attempt_id
                )));
            }

            fs::remove_file(&self.lock_path).map_err(|e| {
                UpdaterError::Io(format!(
                    "Nie można usunąć pliku blokady {}: {e}",
                    self.lock_path.display()
                ))
            })?;
        }

        self.is_released = true;
        Ok(())
    }

    /// Identyfikator próby aktualizacji będącej właścicielem blokady.
    pub fn attempt_id(&self) -> UpdateAttemptId {
        self.attempt_id
    }

    /// Ścieżka do pliku blokady.
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for UpdateLockGuard {
    fn drop(&mut self) {
        if !self.is_released {
            let _ = self.release();
        }
    }
}

/// Sprawdza, czy dany plik blokady może zostać uznany za osierocony (Stale Lock).
///
/// Zasada bezpieczeństwa:
/// Sam brak procesu o danym PID (`stale_pid_alone_does_not_authorize_lock_removal`)
/// NIE upoważnia do automatycznego usunięcia blokady, ponieważ PID mógł zostać zrecyklingowany
/// lub proces mógł zawisnąć. Usunięcie wymaga wielu spójnych dowodów.
pub fn evaluate_lock_status(
    installation_root: &Path,
    active_attempt_id: Option<UpdateAttemptId>,
) -> Result<Option<UpdateLockRecord>, UpdaterError> {
    let lock_path = installation_root.join(UPDATE_LOCK_FILENAME);
    if !lock_path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&lock_path).map_err(|e| {
        UpdaterError::Io(format!(
            "Nie można odczytać pliku blokady {}: {e}",
            lock_path.display()
        ))
    })?;

    let record = serde_json::from_slice::<UpdateLockRecord>(&bytes).map_err(|_| {
        UpdaterError::LockCorrupt(format!(
            "Plik blokady {} zawiera uszkodzony rekord",
            lock_path.display()
        ))
    })?;

    if let Some(expected_attempt) = active_attempt_id
        && record.attempt_id != expected_attempt
    {
        return Err(UpdaterError::ForeignLock(format!(
            "Wykryto obcą blokadę próby {}, oczekiwano {}",
            record.attempt_id, expected_attempt
        )));
    }

    Ok(Some(record))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[test]
    fn first_update_attempt_acquires_lock() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let guard = UpdateLockGuard::acquire(temp.path(), attempt).expect("Pomyślne nabycie locka");
        assert!(guard.lock_path().exists());
        assert_eq!(guard.attempt_id(), attempt);
    }

    #[test]
    fn second_update_attempt_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let attempt1 = UpdateAttemptId::generate();
        let attempt2 = UpdateAttemptId::generate();

        let _guard1 = UpdateLockGuard::acquire(temp.path(), attempt1).expect("Nabycie 1");
        let res2 = UpdateLockGuard::acquire(temp.path(), attempt2);
        assert!(matches!(res2, Err(UpdaterError::LockContended(_))));
    }

    #[test]
    fn lock_creation_is_atomic() {
        let temp = tempfile::tempdir().unwrap();
        let lock_file = temp.path().join(UPDATE_LOCK_FILENAME);
        assert!(!lock_file.exists());

        let attempt = UpdateAttemptId::generate();
        let guard = UpdateLockGuard::acquire(temp.path(), attempt).unwrap();
        assert!(lock_file.exists());
        drop(guard);
        assert!(!lock_file.exists());
    }

    #[test]
    fn lock_guard_releases_own_lock() {
        let temp = tempfile::tempdir().unwrap();
        let attempt = UpdateAttemptId::generate();
        let mut guard = UpdateLockGuard::acquire(temp.path(), attempt).unwrap();
        assert!(temp.path().join(UPDATE_LOCK_FILENAME).exists());
        guard.release().unwrap();
        assert!(!temp.path().join(UPDATE_LOCK_FILENAME).exists());
    }

    #[test]
    fn lock_guard_does_not_remove_foreign_lock() {
        let temp = tempfile::tempdir().unwrap();
        let attempt1 = UpdateAttemptId::generate();
        let attempt2 = UpdateAttemptId::generate();

        let mut guard1 = UpdateLockGuard::acquire(temp.path(), attempt1).unwrap();

        // Symulacja podmienienia rekordu blokady na obcą próbę (attempt2)
        let foreign_record = UpdateLockRecord::new(attempt2, "test_root".into());
        let foreign_bytes = serde_json::to_vec_pretty(&foreign_record).unwrap();
        std::fs::write(guard1.lock_path(), foreign_bytes).unwrap();

        // Próba zwolnienia przez guard1 nie może usunąć pliku obcej blokady
        let res = guard1.release();
        assert!(matches!(res, Err(UpdaterError::ForeignLock(_))));
        assert!(temp.path().join(UPDATE_LOCK_FILENAME).exists());
    }

    #[test]
    fn corrupt_lock_is_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let lock_file = temp.path().join(UPDATE_LOCK_FILENAME);
        std::fs::write(&lock_file, b"NOT A VALID JSON LOCK RECORD").unwrap();

        let attempt = UpdateAttemptId::generate();
        let res = UpdateLockGuard::acquire(temp.path(), attempt);
        assert!(matches!(res, Err(UpdaterError::LockCorrupt(_))));
    }

    #[test]
    fn mismatched_attempt_cannot_release_lock() {
        let temp = tempfile::tempdir().unwrap();
        let attempt1 = UpdateAttemptId::generate();
        let attempt2 = UpdateAttemptId::generate();

        let _guard = UpdateLockGuard::acquire(temp.path(), attempt1).unwrap();
        let eval_res = evaluate_lock_status(temp.path(), Some(attempt2));
        assert!(matches!(eval_res, Err(UpdaterError::ForeignLock(_))));
    }

    #[test]
    fn stale_pid_alone_does_not_authorize_lock_removal() {
        let temp = tempfile::tempdir().unwrap();
        let lock_file = temp.path().join(UPDATE_LOCK_FILENAME);

        // Rekord blokady z martwym PID
        let record = UpdateLockRecord {
            record_version: UPDATE_LOCK_RECORD_VERSION,
            attempt_id: UpdateAttemptId::generate(),
            pid: 999_999, // Fikcyjny martwy PID
            process_start_identity: None,
            installation_identity: "test_root".into(),
            creation_time: Utc::now(),
            sanitized_executable_identity: "SPLogbook".into(),
        };
        std::fs::write(&lock_file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        // Próba nabycia nowej blokady musi zwrócić LockContended, a nie usunąć starą blokadę
        let new_attempt = UpdateAttemptId::generate();
        let res = UpdateLockGuard::acquire(temp.path(), new_attempt);
        assert!(matches!(res, Err(UpdaterError::LockContended(_))));
        assert!(lock_file.exists());
    }

    #[test]
    fn lock_record_contains_no_secret() {
        let record = UpdateLockRecord::new(UpdateAttemptId::generate(), "C:\\SPLogbook".into());
        let serialized = serde_json::to_string(&record).unwrap();
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("password"));
        assert!(!serialized.contains("token"));
        assert!(!serialized.contains("key"));
        assert!(!serialized.contains("callsign"));
    }

    #[test]
    fn concurrent_lock_stress_has_single_winner() {
        let temp = tempfile::tempdir().unwrap();
        let temp_path = Arc::new(temp.path().to_path_buf());
        let winners = Arc::new(AtomicUsize::new(0));
        let contenders = Arc::new(AtomicUsize::new(0));
        let num_threads = 20;

        let mut handles = Vec::new();
        for _ in 0..num_threads {
            let t_path = Arc::clone(&temp_path);
            let t_winners = Arc::clone(&winners);
            let t_contenders = Arc::clone(&contenders);

            handles.push(thread::spawn(move || {
                let attempt = UpdateAttemptId::generate();
                match UpdateLockGuard::acquire(&t_path, attempt) {
                    Ok(_guard) => {
                        t_winners.fetch_add(1, Ordering::SeqCst);
                        // Trzymaj blokadę przez krótki czas, aby wymusić rywalizację
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(UpdaterError::LockContended(_)) => {
                        t_contenders.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => panic!("Nieoczekiwany błąd rywalizacji blokady: {e:?}"),
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Dokładnie jeden wątek musiał wygrać blokadę
        assert_eq!(winners.load(Ordering::SeqCst), 1);
        assert_eq!(contenders.load(Ordering::SeqCst), num_threads - 1);
    }
}
