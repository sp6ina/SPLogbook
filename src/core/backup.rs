// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Automatyczna kopia zapasowa bazy danych SQLite i eksportu ADIF przy zamykaniu programu

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, backup::Backup};

pub struct BackupManager;

impl BackupManager {
    /// Wykonuje spójną kopię zapasową bazy danych SQLite.
    ///
    /// Zamiast `fs::copy` na żywej bazie WAL (co grozi niespójną kopią) używamy
    /// API `Backup` SQLite, które wykonuje online-backup i daje spójny snapshot
    /// nawet przy współbieżnych zapisach. Nazwa pliku ma precyzję do milisekund,
    /// aby kolejne kopie w tej samej sekundzie nie nadpisywały się nawzajem.
    pub fn backup_database(source_db_path: &Path, backup_dir: &Path) -> Result<PathBuf, String> {
        if !source_db_path.exists() {
            return Err("Plik bazy źródłowej nie istnieje".to_string());
        }

        fs::create_dir_all(backup_dir)
            .map_err(|e| format!("Błąd tworzenia katalogu kopii: {e}"))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string();
        let file_stem = source_db_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let dest_filename = format!("{file_stem}_backup_{timestamp}.db");
        let dest_path = backup_dir.join(&dest_filename);
        // Zapis do pliku tymczasowego w tym samym katalogu, a następnie atomowe
        // `rename` — nagłe przerwanie nie pozostawi częściowej kopii pod docelową nazwą.
        let tmp_path = backup_dir.join(format!(".{dest_filename}.tmp"));

        {
            let source = Connection::open(source_db_path)
                .map_err(|e| format!("Błąd otwarcia bazy źródłowej: {e}"))?;
            let mut dest = Connection::open(&tmp_path)
                .map_err(|e| format!("Błąd otwarcia pliku kopii: {e}"))?;
            let backup = Backup::new(&source, &mut dest)
                .map_err(|e| format!("Błąd inicjalizacji kopii zapasowej: {e}"))?;
            backup
                .run_to_completion(64, Duration::from_millis(50), None)
                .map_err(|e| format!("Błąd wykonywania kopii zapasowej: {e}"))?;
        }

        fs::rename(&tmp_path, &dest_path)
            .map_err(|e| format!("Błąd finalizacji kopii zapasowej: {e}"))?;

        Self::prune_old_backups(backup_dir, ".db", 10).ok();

        Ok(dest_path)
    }

    /// Wykonuje kopię zapasową w formacie ADIF
    pub fn backup_adif(adif_content: &str, backup_dir: &Path) -> Result<PathBuf, String> {
        fs::create_dir_all(backup_dir)
            .map_err(|e| format!("Błąd tworzenia katalogu kopii: {e}"))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f").to_string();
        let dest_filename = format!("SPLogbook_backup_{timestamp}.adi");
        let dest_path = backup_dir.join(&dest_filename);
        let tmp_path = backup_dir.join(format!(".{dest_filename}.tmp"));

        fs::write(&tmp_path, adif_content).map_err(|e| format!("Błąd zapisu pliku ADIF: {e}"))?;
        fs::rename(&tmp_path, &dest_path)
            .map_err(|e| format!("Błąd finalizacji kopii ADIF: {e}"))?;
        Self::prune_old_backups(backup_dir, ".adi", 10).ok();

        Ok(dest_path)
    }

    /// Usuwa najstarsze kopie zapasowe, zachowując maksymalnie `keep_max` plików.
    /// Ogranicza się wyłącznie do nazw generowanych przez tę aplikację, aby nie
    /// usuwać obcych plików o tym samym rozszerzeniu.
    fn prune_old_backups(dir: &Path, extension: &str, keep_max: usize) -> std::io::Result<()> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let is_app_backup = if extension == ".db" {
                name.contains("_backup_") && name.ends_with(extension)
            } else {
                name.starts_with("SPLogbook_backup_") && name.ends_with(extension)
            };
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
        let temp_dir = std::env::temp_dir().join("splog_backup_test");
        let res = BackupManager::backup_adif("ADIF test content", &temp_dir);
        assert!(res.is_ok());
        let path = res.unwrap();
        assert!(path.exists());
        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(temp_dir);
    }

    #[test]
    fn test_backup_database_consistent() {
        let temp_dir = std::env::temp_dir().join("splog_backup_db_test");
        let _ = fs::create_dir_all(&temp_dir);
        let source = temp_dir.join("source.db");

        // Przygotuj źródłową bazę z danymi.
        {
            let conn = Connection::open(&source).unwrap();
            conn.execute_batch(
                "CREATE TABLE qso_records (id INTEGER PRIMARY KEY, callsign TEXT NOT NULL);
                 INSERT INTO qso_records (callsign) VALUES ('SP6INA'), ('SQ6ABC');",
            )
            .unwrap();
        }

        let dest = BackupManager::backup_database(&source, &temp_dir).unwrap();
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

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
