// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Automatyczna kopia zapasowa bazy danych SQLite i eksportu ADIF przy zamykaniu programu

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, backup::Backup};

use crate::core::database::LogDatabase;
use crate::core::i18n::{Msg, key};

#[derive(Debug)]
pub enum BackupError {
    SourceNotFound,
    SamePath,
    CreateDir(String),
    SqliteBackup(String),
    Validation(String),
    Finalize(String),
    WriteAdif(String),
    Restore(String),
    SchemaMismatch { backup: i64, current: i64 },
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceNotFound => write!(f, "Source database file does not exist"),
            Self::SamePath => {
                write!(
                    f,
                    "Backup destination cannot be identical to source database"
                )
            }
            Self::CreateDir(detail) => write!(f, "Failed to create backup directory: {detail}"),
            Self::SqliteBackup(detail) => write!(f, "SQLite backup failed: {detail}"),
            Self::Validation(detail) => write!(f, "Backup validation failed: {detail}"),
            Self::Finalize(detail) => write!(f, "Failed to finalize backup: {detail}"),
            Self::WriteAdif(detail) => write!(f, "Failed to write ADIF backup: {detail}"),
            Self::Restore(detail) => write!(f, "Restore failed: {detail}"),
            Self::SchemaMismatch { backup, current } => write!(
                f,
                "Backup schema version ({backup}) is newer than supported application schema ({current})"
            ),
        }
    }
}

impl std::error::Error for BackupError {}

impl BackupError {
    pub fn msg(&self) -> Msg {
        Msg::new(key::BACKUP_CREATE_ERROR).arg("err", self.to_string())
    }
}

pub struct BackupManager;

impl BackupManager {
    pub const SUPPORTED_SCHEMA_VERSION: i64 = 7;

    /// Weryfikuje integralność pliku bazy SQLite za pomocą `PRAGMA quick_check(1)`
    /// oraz odczytuje maksymalną wersję schematu.
    pub fn validate_backup_internal(path: &Path) -> Result<Option<i64>, BackupError> {
        if !path.exists() {
            return Err(BackupError::Validation(
                "Backup file does not exist".to_string(),
            ));
        }
        let meta = fs::metadata(path)
            .map_err(|e| BackupError::Validation(format!("Cannot read backup metadata: {e}")))?;
        if meta.len() == 0 {
            return Err(BackupError::Validation(
                "Backup file is empty (0 bytes)".to_string(),
            ));
        }

        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| {
                BackupError::Validation(format!("Cannot open backup for verification: {e}"))
            })?;

        let quick_check: String = conn
            .query_row("PRAGMA quick_check(1);", [], |r| r.get(0))
            .map_err(|e| BackupError::Validation(format!("PRAGMA quick_check failed: {e}")))?;

        if quick_check != "ok" {
            return Err(BackupError::Validation(format!(
                "Integrity check failed: {quick_check}"
            )));
        }

        let schema_ver: Option<i64> = conn
            .query_row("SELECT MAX(version) FROM schema_version;", [], |r| r.get(0))
            .optional()
            .unwrap_or(None);

        Ok(schema_ver)
    }

    /// Wykonuje spójną kopię zapasową bazy danych SQLite.
    ///
    /// Używa wyłącznie API `rusqlite::backup::Backup` z `busy_timeout(Duration::from_secs(5))`.
    /// Zgodnie z DATA-006, nigdy nie stosuje niebezpiecznego fallbacku `fs::copy` na żywym pliku WAL.
    /// Kopia jest tworzona w pliku `.tmp`, weryfikowana przez `validate_backup_internal`,
    /// a następnie atomowo przenoszona do nazwy docelowej.
    pub fn backup_database(
        source_db_path: &Path,
        backup_dir: &Path,
    ) -> Result<PathBuf, BackupError> {
        if !source_db_path.exists() {
            return Err(BackupError::SourceNotFound);
        }

        fs::create_dir_all(backup_dir).map_err(|e| BackupError::CreateDir(e.to_string()))?;

        // Zabezpieczenie przed zapisaniem kopii do tej samej ścieżki co aktywna baza
        if let (Ok(source_can), Ok(bdir_can)) =
            (source_db_path.canonicalize(), backup_dir.canonicalize())
        {
            if source_can == bdir_can {
                return Err(BackupError::SamePath);
            }
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string();
        let file_stem = source_db_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let dest_filename = format!("{file_stem}_backup_{timestamp}.db");
        let dest_path = backup_dir.join(&dest_filename);
        let tmp_path = backup_dir.join(format!(".{dest_filename}.tmp"));

        let _ = fs::remove_file(&tmp_path);

        // Wykonanie backupu online z limitem czasu na blokadę
        {
            let source = Connection::open_with_flags(
                source_db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .map_err(|e| BackupError::SqliteBackup(format!("Cannot open source database: {e}")))?;

            source.busy_timeout(Duration::from_secs(5)).map_err(|e| {
                BackupError::SqliteBackup(format!("Cannot set busy timeout on source: {e}"))
            })?;

            let mut dest = Connection::open(&tmp_path).map_err(|e| {
                BackupError::SqliteBackup(format!("Cannot create temp backup database: {e}"))
            })?;

            dest.busy_timeout(Duration::from_secs(5)).map_err(|e| {
                BackupError::SqliteBackup(format!("Cannot set busy timeout on destination: {e}"))
            })?;

            let backup = Backup::new(&source, &mut dest).map_err(|e| {
                BackupError::SqliteBackup(format!("Cannot initialize SQLite backup: {e}"))
            })?;

            backup
                .run_to_completion(100, Duration::from_millis(25), None)
                .map_err(|e| {
                    let _ = fs::remove_file(&tmp_path);
                    BackupError::SqliteBackup(format!("SQLite backup step failed: {e}"))
                })?;
        }

        // Walidacja spójności kopii przed zatwierdzeniem
        if let Err(val_err) = Self::validate_backup_internal(&tmp_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(val_err);
        }

        // Atomowe przeniesienie z pliku tymczasowego
        fs::rename(&tmp_path, &dest_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            BackupError::Finalize(e.to_string())
        })?;

        // Rotacja starych kopii z filtrowaniem po file_stem (DATA-006)
        Self::prune_old_backups(backup_dir, file_stem, ".db", 10).ok();

        Ok(dest_path)
    }

    /// Przywraca bazę danych z kopii zapasowej do docelowej ścieżki aktywnej bazy.
    ///
    /// Bezpieczny proces przywracania z walidacją i automatycznym rollbackiem:
    /// 1. Weryfikuje integralność kopii (`validate_backup_internal`).
    /// 2. Odrzuca kopie o wersji schematu nowszej niż `SUPPORTED_SCHEMA_VERSION` (DATA-006).
    /// 3. Tworzy plik stagingowy w katalogu docelowym i — jeśli wersja schematu jest starsza —
    ///    uruchamia migracje na pliku stagingowym przez `LogDatabase::open`.
    /// 4. Zabezpiecza aktywną bazę w pliku `.pre_restore_bak` (rollback snapshot).
    /// 5. Podmienia plik docelowy, usuwa stare pliki `-wal` i `-shm`, po czym weryfikuje bazę.
    /// 6. W razie jakiegokolwiek błędu na etapie podmiany, natychmiast przywraca aktywną bazę
    ///    ze snapshotu rollbackowego.
    pub fn restore_database(backup_path: &Path, target_db_path: &Path) -> Result<(), BackupError> {
        // 1. Walidacja pliku kopii zapasowej
        let backup_ver = Self::validate_backup_internal(backup_path)?;
        let ver = backup_ver.unwrap_or(0);

        // 2. Weryfikacja wersji schematu (odrzucenie nowszej)
        if ver > Self::SUPPORTED_SCHEMA_VERSION {
            return Err(BackupError::SchemaMismatch {
                backup: ver,
                current: Self::SUPPORTED_SCHEMA_VERSION,
            });
        }

        let target_dir = target_db_path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(target_dir).map_err(|e| BackupError::CreateDir(e.to_string()))?;

        let target_filename = target_db_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("log.db");

        // 3. Staging
        let staging_path = target_dir.join(format!(".{target_filename}.staging_restore.tmp"));
        let _ = fs::remove_file(&staging_path);

        fs::copy(backup_path, &staging_path)
            .map_err(|e| BackupError::Restore(format!("Failed to copy backup to staging: {e}")))?;

        // Jeśli wersja schematu jest starsza, przeprowadzamy migrację na pliku stagingowym
        if ver < Self::SUPPORTED_SCHEMA_VERSION {
            {
                let _db = LogDatabase::open(&staging_path).map_err(|e| {
                    let _ = fs::remove_file(&staging_path);
                    BackupError::Restore(format!("Staged migration failed: {e}"))
                })?;
            }
            // Zrzucamy WAL stagingu do pojedynczego pliku i usuwamy pliki pomocnicze
            if let Ok(conn) = Connection::open(&staging_path) {
                let _ = conn.pragma_update(None, "wal_checkpoint", "TRUNCATE");
            }
            let staging_wal = PathBuf::from(format!("{}-wal", staging_path.to_string_lossy()));
            let staging_shm = PathBuf::from(format!("{}-shm", staging_path.to_string_lossy()));
            let _ = fs::remove_file(&staging_wal);
            let _ = fs::remove_file(&staging_shm);

            // Sprawdzamy integralność po migracji
            if let Err(val_err) = Self::validate_backup_internal(&staging_path) {
                let _ = fs::remove_file(&staging_path);
                return Err(val_err);
            }
        }

        // 4. Kopia bezpieczeństwa do rollbacku
        let rollback_path = target_dir.join(format!(".{target_filename}.pre_restore_bak"));
        let _ = fs::remove_file(&rollback_path);
        let mut has_rollback = false;

        if target_db_path.exists() {
            fs::copy(target_db_path, &rollback_path).map_err(|e| {
                let _ = fs::remove_file(&staging_path);
                BackupError::Restore(format!("Failed to create safety rollback snapshot: {e}"))
            })?;
            has_rollback = true;
        }

        // 5. Atomowa podmiana docelowej bazy i czyszczenie
        let wal_path = PathBuf::from(format!("{}-wal", target_db_path.to_string_lossy()));
        let shm_path = PathBuf::from(format!("{}-shm", target_db_path.to_string_lossy()));

        let replace_result = (|| -> Result<(), BackupError> {
            if fs::rename(&staging_path, target_db_path).is_err() {
                // Na Windows przy istniejącym pliku docelowym może być konieczne usunięcie go przed podmianą
                let _ = fs::remove_file(target_db_path);
                fs::rename(&staging_path, target_db_path)
                    .or_else(|_| fs::copy(&staging_path, target_db_path).map(|_| ()))
                    .map_err(|e| {
                        BackupError::Restore(format!("Failed to overwrite active database: {e}"))
                    })?;
            }
            let _ = fs::remove_file(&staging_path);

            // Usunięcie starych plików WAL i SHM, aby SQLite nie próbował łączyć ich z nową bazą
            let _ = fs::remove_file(&wal_path);
            let _ = fs::remove_file(&shm_path);

            // Końcowa weryfikacja podmienionej bazy
            Self::validate_backup_internal(target_db_path)?;

            Ok(())
        })();

        // 6. Rollback w przypadku niepowodzenia
        if let Err(err) = replace_result {
            if has_rollback && rollback_path.exists() {
                let _ = fs::copy(&rollback_path, target_db_path);
            }
            let _ = fs::remove_file(&staging_path);
            let _ = fs::remove_file(&rollback_path);
            return Err(err);
        }

        // Czyszczenie pliku rollbacku po udanym przywróceniu
        if has_rollback {
            let _ = fs::remove_file(&rollback_path);
        }

        Ok(())
    }

    /// Wykonuje kopię zapasową w formacie ADIF
    pub fn backup_adif(adif_content: &str, backup_dir: &Path) -> Result<PathBuf, BackupError> {
        fs::create_dir_all(backup_dir).map_err(|e| BackupError::CreateDir(e.to_string()))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string();
        let dest_filename = format!("SPLogbook_backup_{timestamp}.adi");
        let dest_path = backup_dir.join(&dest_filename);
        let tmp_path = backup_dir.join(format!(".{dest_filename}.tmp"));

        fs::write(&tmp_path, adif_content).map_err(|e| BackupError::WriteAdif(e.to_string()))?;
        fs::rename(&tmp_path, &dest_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            BackupError::Finalize(e.to_string())
        })?;
        Self::prune_old_backups(backup_dir, "SPLogbook", ".adi", 10).ok();

        Ok(dest_path)
    }

    /// Usuwa najstarsze kopie zapasowe, zachowując maksymalnie `keep_max` plików.
    /// Filtruje ściśle według prefiksu `{stem}_backup_` (DATA-006), dzięki czemu nie usuwa
    /// kopii innych baz danych ani plików o obcej nazwie.
    fn prune_old_backups(
        dir: &Path,
        stem: &str,
        extension: &str,
        keep_max: usize,
    ) -> std::io::Result<()> {
        let expected_prefix = format!("{stem}_backup_");
        let mut entries = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let is_app_backup = name.starts_with(&expected_prefix) && name.ends_with(extension);
            if path.is_file() && is_app_backup {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        entries.push((path, modified));
                    }
                }
            }
        }

        // Sortuj od najstarszego do najnowszego
        entries.sort_by_key(|&(_, time)| time);

        if entries.len() > keep_max {
            let to_remove = entries.len() - keep_max;
            for (path, _) in entries.into_iter().take(to_remove) {
                let _ = fs::remove_file(path);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_adif() {
        let temp_dir = std::env::temp_dir().join("splog_backup_test_adif");
        let res = BackupManager::backup_adif("ADIF test content", &temp_dir);
        assert!(res.is_ok());
        let path = res.unwrap();
        assert!(path.exists());
        let _ = fs::remove_file(path);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_backup_database_consistent() {
        let temp_dir = std::env::temp_dir().join("splog_backup_db_test_consistent");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let source = temp_dir.join("source.db");
        let backup_dir = temp_dir.join("backups");

        // Przygotuj źródłową bazę z danymi w trybie WAL
        {
            let conn = Connection::open(&source).unwrap();
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);
                 INSERT OR REPLACE INTO schema_version (version) VALUES (7);
                 CREATE TABLE IF NOT EXISTS qso_records (id INTEGER PRIMARY KEY, callsign TEXT NOT NULL);
                 INSERT INTO qso_records (callsign) VALUES ('SP6INA'), ('SQ6ABC');",
            )
            .unwrap();
        }

        let dest = BackupManager::backup_database(&source, &backup_dir).unwrap();
        assert!(dest.exists());
        assert!(
            dest.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("source_backup_")
        );

        // Kopia musi być spójną, otwieralną bazą z tymi samymi danymi.
        let conn = Connection::open(&dest).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let schema_ver: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(schema_ver, 7);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backup_database_rejects_nonexistent_source() {
        let temp_dir = std::env::temp_dir().join("splog_backup_test_nonexistent");
        let non_existent = temp_dir.join("ghost.db");
        let res = BackupManager::backup_database(&non_existent, &temp_dir);
        assert!(matches!(res, Err(BackupError::SourceNotFound)));
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_prune_old_backups_only_removes_matching_database_stem() {
        let temp_dir = std::env::temp_dir().join("splog_backup_test_prune");
        let _ = fs::create_dir_all(&temp_dir);

        // Tworzymy pliki dla bazy A oraz dla bazy B
        let file_a1 = temp_dir.join("dbA_backup_20260101_100000.000.db");
        let file_a2 = temp_dir.join("dbA_backup_20260101_110000.000.db");
        let file_a3 = temp_dir.join("dbA_backup_20260101_120000.000.db");
        let file_b1 = temp_dir.join("dbB_backup_20260101_100000.000.db");
        let file_b2 = temp_dir.join("dbB_backup_20260101_110000.000.db");

        fs::write(&file_a1, "a1").unwrap();
        std::thread::sleep(Duration::from_millis(20));
        fs::write(&file_a2, "a2").unwrap();
        std::thread::sleep(Duration::from_millis(20));
        fs::write(&file_a3, "a3").unwrap();

        fs::write(&file_b1, "b1").unwrap();
        fs::write(&file_b2, "b2").unwrap();

        // Rotujemy dbA z limitem keep_max = 2
        BackupManager::prune_old_backups(&temp_dir, "dbA", ".db", 2).unwrap();

        // Najstarszy plik dbA powinien zostać usunięty
        assert!(!file_a1.exists(), "Oldest dbA backup should be pruned");
        assert!(file_a2.exists(), "dbA backup 2 should be preserved");
        assert!(file_a3.exists(), "dbA backup 3 should be preserved");

        // Żaden plik dbB nie może zostać usunięty!
        assert!(file_b1.exists(), "dbB backup 1 must NOT be pruned");
        assert!(file_b2.exists(), "dbB backup 2 must NOT be pruned");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backup_validation_detects_corrupt_file() {
        let temp_dir = std::env::temp_dir().join("splog_backup_test_corrupt");
        let _ = fs::create_dir_all(&temp_dir);
        let corrupt_path = temp_dir.join("corrupt.db");

        fs::write(
            &corrupt_path,
            b"This is not a SQLite database file, just random garbage text",
        )
        .unwrap();

        let val_res = BackupManager::validate_backup_internal(&corrupt_path);
        assert!(matches!(val_res, Err(BackupError::Validation(_))));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_backup_database_concurrent_writer() {
        let temp_dir = std::env::temp_dir().join("splog_backup_test_concurrent");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let source = temp_dir.join("concurrent.db");
        let backup_dir = temp_dir.join("backups");

        // Przygotuj bazę WAL z tabelą
        {
            let conn = Connection::open(&source).unwrap();
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);
                 INSERT OR REPLACE INTO schema_version (version) VALUES (7);
                 CREATE TABLE IF NOT EXISTS logs (id INTEGER PRIMARY KEY, msg TEXT NOT NULL);",
            )
            .unwrap();
        }

        // Uruchom wątek zapisujący w tle
        let source_clone = source.clone();
        let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        let writer_thread = std::thread::spawn(move || {
            let conn = Connection::open(&source_clone).unwrap();
            conn.busy_timeout(Duration::from_secs(5)).unwrap();
            let mut i = 0;
            while !stop_flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = conn.execute(
                    "INSERT INTO logs (msg) VALUES (?1)",
                    rusqlite::params![format!("msg_{i}")],
                );
                i += 1;
                std::thread::sleep(Duration::from_millis(5));
            }
        });

        // W czasie trwania zapisów wykonaj backup
        std::thread::sleep(Duration::from_millis(30));
        let backup_path = BackupManager::backup_database(&source, &backup_dir).unwrap();
        assert!(backup_path.exists());

        // Zatrzymaj wątek zapisujący
        stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        writer_thread.join().unwrap();

        // Walidacja backupu
        let val = BackupManager::validate_backup_internal(&backup_path).unwrap();
        assert_eq!(val, Some(7));

        let backup_conn = Connection::open(&backup_path).unwrap();
        let count: i64 = backup_conn
            .query_row("SELECT COUNT(*) FROM logs", [], |r| r.get(0))
            .unwrap();
        assert!(
            count > 0,
            "Backup must contain rows written before snapshot finalized"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_restore_valid_backup_success() {
        let temp_dir = std::env::temp_dir().join("splog_restore_test_valid");
        let _ = fs::create_dir_all(&temp_dir);
        let active_db = temp_dir.join("active.db");
        let backup_dir = temp_dir.join("backups");

        // 1. Tworzymy aktywną bazę ze schematem i rekordem
        {
            let db = LogDatabase::open(&active_db).unwrap();
            let mut qso = crate::core::qso::QsoRecord::new("SP6INA", "20m", "SSB");
            qso.qso_date = "20260801".to_string();
            qso.time_on = "100000".to_string();
            qso.journal_id = Some("DEFAULT".to_string());
            db.insert_qso(&qso).unwrap();
        }

        // 2. Tworzymy backup
        let backup_path = BackupManager::backup_database(&active_db, &backup_dir).unwrap();

        // 3. Mutujemy aktywną bazę (dodajemy drugi rekord)
        {
            let db = LogDatabase::open(&active_db).unwrap();
            let mut qso2 = crate::core::qso::QsoRecord::new("SQ6ABC", "40m", "CW");
            qso2.qso_date = "20260801".to_string();
            qso2.time_on = "101500".to_string();
            qso2.journal_id = Some("DEFAULT".to_string());
            db.insert_qso(&qso2).unwrap();
            assert_eq!(db.count_all().unwrap(), 2);
        }

        // 4. Przywracamy bazę z backupu
        BackupManager::restore_database(&backup_path, &active_db).unwrap();

        // 5. Weryfikujemy, że aktywna baza ma z powrotem dokładnie 1 rekord z backupu
        {
            let db = LogDatabase::open(&active_db).unwrap();
            assert_eq!(db.count_all().unwrap(), 1);
            let qsos = db.get_recent_qsos(10).unwrap();
            assert_eq!(qsos[0].callsign, "SP6INA");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_restore_rejects_corrupted_backup_without_touching_target() {
        let temp_dir = std::env::temp_dir().join("splog_restore_test_corrupt");
        let _ = fs::create_dir_all(&temp_dir);
        let active_db = temp_dir.join("active.db");
        let corrupt_backup = temp_dir.join("corrupt_backup.db");

        // 1. Tworzymy aktywną bazę
        {
            let db = LogDatabase::open(&active_db).unwrap();
            let mut qso = crate::core::qso::QsoRecord::new("SP1INTACT", "15m", "FT8");
            qso.qso_date = "20260802".to_string();
            qso.time_on = "120000".to_string();
            qso.journal_id = Some("DEFAULT".to_string());
            db.insert_qso(&qso).unwrap();
        }

        // 2. Tworzymy uszkodzony plik backupu
        fs::write(
            &corrupt_backup,
            b"THIS IS TOTALLY CORRUPTED AND NOT SQLITE DATA",
        )
        .unwrap();

        // 3. Próba restore musi zwrócić błąd walidacji
        let res = BackupManager::restore_database(&corrupt_backup, &active_db);
        assert!(matches!(res, Err(BackupError::Validation(_))));

        // 4. Aktywna baza musi być nienaruszona!
        {
            let db = LogDatabase::open(&active_db).unwrap();
            let qsos = db.get_recent_qsos(10).unwrap();
            assert_eq!(qsos.len(), 1);
            assert_eq!(qsos[0].callsign, "SP1INTACT");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_restore_rejects_newer_schema_version() {
        let temp_dir = std::env::temp_dir().join("splog_restore_test_future_schema");
        let _ = fs::create_dir_all(&temp_dir);
        let active_db = temp_dir.join("active.db");
        let future_backup = temp_dir.join("future_backup.db");

        // Tworzymy poprawną bazę SQLite z wersją schematu 99
        {
            let conn = Connection::open(&future_backup).unwrap();
            conn.execute_batch(
                "CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
                 INSERT INTO schema_version (version) VALUES (99);
                 CREATE TABLE dummy (id INTEGER);",
            )
            .unwrap();
        }

        // Restore musi odrzucić ze SchemaMismatch
        let res = BackupManager::restore_database(&future_backup, &active_db);
        match res {
            Err(BackupError::SchemaMismatch { backup, current }) => {
                assert_eq!(backup, 99);
                assert_eq!(current, 7);
            }
            other => panic!("Expected SchemaMismatch, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_restore_older_schema_migrates_to_current() {
        let temp_dir = std::env::temp_dir().join("splog_restore_test_older_schema");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);
        let active_db = temp_dir.join("active.db");
        let older_backup = temp_dir.join("v1_backup.db");

        // Tworzymy bazę w schemacie wersji 1 z jednym QSO
        {
            let conn = Connection::open(&older_backup).unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);
                 INSERT OR REPLACE INTO schema_version (version) VALUES (1);",
            )
            .unwrap();
            LogDatabase::migration_1_base_schema(&conn).unwrap();
            conn.execute(
                "INSERT INTO qso_records (callsign, band, mode, qso_date, time_on, rst_sent, rst_rcvd, journal_id)
                 VALUES ('SP5OLD', '20m', 'CW', '20260101', '120000', '599', '599', 'DEFAULT');",
                [],
            )
            .unwrap();
        }

        // Restore bazy w wersji 1 do active_db
        BackupManager::restore_database(&older_backup, &active_db).unwrap();

        // Aktywna baza po restore musi mieć schemat podniesiony do 7 i zachowane dane
        {
            let db = LogDatabase::open(&active_db).unwrap();
            let ver = db.current_schema_version().unwrap();
            assert_eq!(
                ver, 7,
                "Restored database must be migrated to latest schema version 7"
            );
            let qsos = db.get_recent_qsos(10).unwrap();
            assert_eq!(qsos.len(), 1);
            assert_eq!(qsos[0].callsign, "SP5OLD");
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
