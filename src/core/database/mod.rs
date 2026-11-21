// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł bazy danych SQLite dla logu łączności SPLogbook.
//!
//! Podział na wyspecjalizowane moduły:
//! - `journals`: definicja `Journal` i operacje na profilach dzienników
//! - `migrations`: tablica migracji schematu (`MIGRATIONS`, jedno źródło `LATEST_SCHEMA_VERSION`),
//!   kopia przedmigracyjna i walidacja schematu
//! - `qsl`: obsługa potwierdzeń LoTW i eQSL
//! - `qso_store`: operacje CRUD na rekordach QSO
//! - `schema`: centralna definicja kolumn tabeli, mapowanie wierszy i zapytania SQL
//! - `search`: zaawansowane filtrowanie i wyszukiwanie duplikatów
//! - `stats`: statystyki bazy danych
//! - `upload_checkpoint`: trwałe checkpointy planu uploadu (F-20R / F20-R1)
//! - `upload_selection`: dziennikowo ograniczony wybór i zapis statusu uploadu (F-20)

use rusqlite::{Connection, Result};
use std::path::Path;

pub mod journals;
pub mod migrations;
pub mod qsl;
pub mod qso_store;
pub mod schema;
pub mod search;
mod stats;
pub mod upload_checkpoint;
pub mod upload_selection;

#[cfg(test)]
mod tests;

pub use journals::Journal;
pub use migrations::{LATEST_SCHEMA_VERSION, pre_migration_backup_dir};
pub use search::AdvancedQsoFilter;

/// Czy otwarcie bazy plikowej ma wykonać kopię przedmigracyjną (STAGE-V140-S1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreMigrationBackup {
    /// Domyślnie: kopia przed każdą zaległą migracją bazy zawierającej QSO.
    Required,
    /// Wyłącznie plik stagingowy przywracania: jest kopią pliku kopii zapasowej, który
    /// pozostaje nienaruszony, więc stan sprzed migracji już istnieje.
    SkipForStagedRestore,
}

/// Menedżer bazy danych SQLite dla logu łączności
pub struct LogDatabase {
    conn: Connection,
}

/// Nazwa podkatalogu danych użytkownika z bazami.
pub const DATABASES_SUBDIR: &str = "databases";
/// Nazwa pliku domyślnego dziennika.
pub const DEFAULT_LOG_DB_FILE: &str = "default_log.db";

/// Ścieżka domyślnego dziennika w katalogu danych użytkownika, budowana wyłącznie przez
/// `join` (bez mieszania `\` i `/` w jednym łańcuchu na Windowsie).
#[must_use]
pub fn default_log_db_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir
        .join(DATABASES_SUBDIR)
        .join(DEFAULT_LOG_DB_FILE)
}

/// Tworzy katalog nadrzędny pliku (rekurencyjnie). Na czystym profilu katalog `databases`
/// nie istnieje — instalatory nie dostarczają już gotowych baz.
pub fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => std::fs::create_dir_all(parent),
        _ => Ok(()),
    }
}

impl LogDatabase {
    /// Otwiera bazę danych SQLite (lub tworzy nową) i inicjalizuje schemat tabel.
    ///
    /// Katalog nadrzędny jest tworzony, jeśli nie istnieje (czysty profil); niepowodzenie
    /// tworzenia katalogu zwraca czytelny błąd `CANTOPEN` z przyczyną, a nie ciche „unable
    /// to open database file”.
    ///
    /// Gdy baza zawiera QSO i ma zaległe migracje, najpierw powstaje zweryfikowana kopia
    /// przedmigracyjna (`backups/pre-migration/`); jej niepowodzenie zwraca błąd bez migracji.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_policy(db_path.as_ref(), PreMigrationBackup::Required)
    }

    /// Otwiera plik stagingowy przywracanej kopii (`BackupManager::restore_database`) bez kopii
    /// przedmigracyjnej — źródłowy plik kopii zapasowej pozostaje nienaruszony.
    pub(crate) fn open_staged_restore(db_path: &Path) -> Result<Self> {
        Self::open_with_policy(db_path, PreMigrationBackup::SkipForStagedRestore)
    }

    fn open_with_policy(db_path: &Path, policy: PreMigrationBackup) -> Result<Self> {
        ensure_parent_dir(db_path).map_err(|e| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                Some(format!(
                    "cannot create directory for {}: {e}",
                    db_path.display()
                )),
            )
        })?;
        let conn = Connection::open(db_path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        // Kopia przedmigracyjna przed jakąkolwiek zmianą pliku (także przed przełączeniem WAL).
        if policy == PreMigrationBackup::Required {
            Self::backup_before_pending_migrations(&conn, db_path)?;
        }

        // Włącz tryb WAL (Write-Ahead Logging) dla najwyższej współbieżności i wydajności
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        let mut db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Otwiera bazę danych w pamięci RAM (np. do testów jednostkowych)
    #[cfg(test)]
    pub fn raw_conn(&self) -> &Connection {
        &self.conn
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let mut db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }
}
