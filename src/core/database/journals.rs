// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use rusqlite::{Result, params};
use serde::{Deserialize, Serialize};

/// Reprezentacja profilu / dziennika łączności (Wielodziennikowość)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Journal {
    pub id: String,
    pub name: String,
    pub station_callsign: String,
    pub operator: String,
    pub my_gridsquare: String,
    pub my_pga: String,
    pub description: String,
    pub is_default: bool,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            id: "DEFAULT".to_string(),
            name: "Główny Dziennik".to_string(),
            station_callsign: String::new(),
            operator: String::new(),
            my_gridsquare: String::new(),
            my_pga: String::new(),
            description: String::new(),
            is_default: true,
        }
    }
}

impl LogDatabase {
    /// Pobiera listę wszystkich zdefiniowanych dzienników (profili)
    pub fn get_all_journals(&self) -> Result<Vec<Journal>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, station_callsign, operator, my_gridsquare, my_pga, description, is_default
             FROM journals ORDER BY is_default DESC, name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Journal {
                id: row.get(0)?,
                name: row.get(1)?,
                station_callsign: row.get(2)?,
                operator: row.get(3)?,
                my_gridsquare: row.get(4)?,
                my_pga: row.get(5)?,
                description: row.get(6)?,
                is_default: row.get::<_, i32>(7)? == 1,
            })
        })?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    /// Tworzy nowy dziennik (profil)
    pub fn create_journal(&self, j: &Journal) -> Result<()> {
        self.conn.execute(
            "INSERT INTO journals (id, name, station_callsign, operator, my_gridsquare, my_pga, description, is_default)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![j.id, j.name, j.station_callsign, j.operator, j.my_gridsquare, j.my_pga, j.description, i32::from(j.is_default)],
        )?;
        Ok(())
    }

    /// Aktualizuje dane profilu dziennika
    pub fn update_journal(&self, j: &Journal) -> Result<()> {
        self.conn.execute(
            "UPDATE journals SET name = ?2, station_callsign = ?3, operator = ?4,
             my_gridsquare = ?5, my_pga = ?6, description = ?7, is_default = ?8
             WHERE id = ?1",
            params![
                j.id,
                j.name,
                j.station_callsign,
                j.operator,
                j.my_gridsquare,
                j.my_pga,
                j.description,
                i32::from(j.is_default)
            ],
        )?;
        Ok(())
    }

    /// Usuwa profil dziennika.
    ///
    /// Blokuje usunięcie dziennika `DEFAULT` oraz aktywnego (domyślnego) profilu;
    /// osierocone QSO są przepinane do `DEFAULT` w tej samej transakcji, więc baza
    /// nigdy nie zostaje z rekordami wskazującymi na nieistniejący dziennik.
    pub fn delete_journal(&self, id: &str) -> Result<(), String> {
        if id == "DEFAULT" {
            return Err("Nie można usunąć dziennika domyślnego.".to_string());
        }

        let is_active: bool = self
            .conn
            .query_row(
                "SELECT is_default FROM journals WHERE id = ?1",
                params![id],
                |row| row.get::<_, i32>(0),
            )
            .map_err(|e| e.to_string())?
            != 0;
        if is_active {
            return Err(
                "Nie można usunąć aktywnego dziennika. Najpierw przełącz aktywny dziennik."
                    .to_string(),
            );
        }

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE qso_records SET journal_id = 'DEFAULT' WHERE journal_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM journals WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Pobiera aktywny / domyślny dziennik. Propaguje błędy SQL zamiast je maskować.
    pub fn get_active_journal(&self) -> Result<Journal> {
        self.conn.query_row(
            "SELECT id, name, station_callsign, operator, my_gridsquare, my_pga, description, is_default
             FROM journals WHERE is_default = 1 LIMIT 1",
            [],
            |row| {
                Ok(Journal {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    station_callsign: row.get(2)?,
                    operator: row.get(3)?,
                    my_gridsquare: row.get(4)?,
                    my_pga: row.get(5)?,
                    description: row.get(6)?,
                    is_default: row.get::<_, i32>(7)? == 1,
                })
            },
        )
    }

    /// Ustawia wybrany dziennik jako domyślny (aktywny). Waliduje istnienie
    /// dziennika i przełącza atomowo, aby nigdy nie zostawić bazy bez aktywnego
    /// dziennika.
    pub fn set_active_journal(&self, id: &str) -> Result<(), String> {
        let exists: bool = self
            .conn
            .prepare("SELECT 1 FROM journals WHERE id = ?1")
            .map_err(|e| e.to_string())?
            .exists(params![id])
            .map_err(|e| e.to_string())?;
        if !exists {
            return Err(format!("Dziennik „{id}” nie istnieje."));
        }

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute("UPDATE journals SET is_default = 0", [])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE journals SET is_default = 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}
