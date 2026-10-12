// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use rusqlite::{Connection, Result};
use std::path::Path;

pub mod journals;
pub mod migrations;
pub mod qsl;
pub mod qso_store;
pub mod schema;
pub mod search;
mod stats;

#[cfg(test)]
mod tests;

pub use journals::Journal;
pub use search::AdvancedQsoFilter;

/// Menedżer bazy danych SQLite dla logu łączności
pub struct LogDatabase {
    conn: Connection,
}

impl LogDatabase {
    /// Otwiera bazę danych SQLite (lub tworzy nową) i inicjalizuje schemat tabel
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Włącz tryb WAL (Write-Ahead Logging) dla najwyższej współbieżności i wydajności
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        let mut db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Otwiera bazę danych w pamięci RAM (np. do testów jednostkowych)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let mut db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }
}
