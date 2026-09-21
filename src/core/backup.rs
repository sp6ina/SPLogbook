// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Automatyczna kopia zapasowa bazy danych SQLite i eksportu ADIF przy zamykaniu programu

use std::fs;
use std::path::{Path, PathBuf};

pub struct BackupManager;

impl BackupManager {
    /// Wykonuje kopię zapasową bazy danych SQLite
    pub fn backup_database(source_db_path: &Path, backup_dir: &Path) -> Result<PathBuf, String> {
        if !source_db_path.exists() {
            return Err("Plik bazy źródłowej nie istnieje".to_string());
        }

        fs::create_dir_all(backup_dir).map_err(|e| format!("Błąd tworzenia katalogu kopii: {}", e))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let file_stem = source_db_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let dest_filename = format!("{}_backup_{}.db", file_stem, timestamp);
        let dest_path = backup_dir.join(dest_filename);

        fs::copy(source_db_path, &dest_path).map_err(|e| format!("Błąd kopiowania bazy: {}", e))?;
        Self::prune_old_backups(backup_dir, ".db", 10).ok();

        Ok(dest_path)
    }

    /// Wykonuje kopię zapasową w formacie ADIF
    pub fn backup_adif(adif_content: &str, backup_dir: &Path) -> Result<PathBuf, String> {
        fs::create_dir_all(backup_dir).map_err(|e| format!("Błąd tworzenia katalogu kopii: {}", e))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let dest_filename = format!("SPLogbook_backup_{}.adi", timestamp);
        let dest_path = backup_dir.join(dest_filename);

        fs::write(&dest_path, adif_content).map_err(|e| format!("Błąd zapisu pliku ADIF: {}", e))?;
        Self::prune_old_backups(backup_dir, ".adi", 10).ok();

        Ok(dest_path)
    }

    /// Usuwa najstarsze kopie zapasowe, zachowując maksymalnie `keep_max` plików
    fn prune_old_backups(dir: &Path, extension: &str, keep_max: usize) -> std::io::Result<()> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.to_string_lossy().ends_with(extension) {
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
}
