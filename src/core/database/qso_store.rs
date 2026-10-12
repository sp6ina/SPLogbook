// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use super::schema::{
    ORDER_CHRONO_ASC, ORDER_CHRONO_DESC, QSO_DATA_COLUMN_COUNT, QSO_SQL, bind_params, collect_qsos,
    for_storage, qso_from_row,
};
use crate::core::qso::{QsoIdentity, QsoRecord};
use rusqlite::{OptionalExtension, Result, params};
use std::collections::{HashMap, HashSet};

impl LogDatabase {
    /// Pobiera QSO po ID
    pub fn get_qso_by_id(&self, id: i64) -> Result<Option<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE id = ?1",
            QSO_SQL.select_columns
        );
        self.conn
            .query_row(&sql, params![id], qso_from_row)
            .optional()
    }

    /// Wstawia nowy rekord QSO do bazy i zwraca nadane ID
    pub fn insert_qso(&self, qso: &QsoRecord) -> Result<i64> {
        let stored = for_storage(qso);
        let params = bind_params(&stored);
        let mut stmt = self.conn.prepare_cached(&QSO_SQL.insert)?;
        stmt.execute(params.as_slice())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Przywraca usunięte QSO zachowując jego oryginalny identyfikator.
    /// Używane przez Undo, aby redo oraz ewentualne referencje zachowały
    /// tożsamość rekordu zamiast tworzyć nowe ID.
    pub fn restore_qso(&self, qso: &QsoRecord) -> Result<i64> {
        let Some(id) = qso.id else {
            return self.insert_qso(qso);
        };
        let stored = for_storage(qso);
        let data_params = bind_params(&stored);
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 1);
        params.extend_from_slice(&data_params);
        params.push(&id);
        let mut stmt = self.conn.prepare_cached(&QSO_SQL.insert_with_id)?;
        stmt.execute(params.as_slice())?;
        Ok(id)
    }

    /// Masowe wstawianie łączności w pojedynczej transakcji (bardzo szybki import ADIF)
    pub fn batch_insert_qsos(&self, qsos: &[QsoRecord]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let mut count = 0;
        {
            let mut stmt = tx.prepare_cached(&QSO_SQL.insert)?;
            for qso in qsos {
                let stored = for_storage(qso);
                let params = bind_params(&stored);
                stmt.execute(params.as_slice())?;
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    /// Pobiera listę poprzednich łączności z daną stacją (do podglądu w locie)
    pub fn find_previous_qsos(&self, callsign: &str) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE callsign = ?1 ORDER BY {ORDER_CHRONO_DESC}",
            QSO_SQL.select_columns
        );
        let mut stmt = self.conn.prepare(&sql)?;
        collect_qsos(&mut stmt, params![callsign.to_uppercase()])
    }

    /// Usuwa łączność z bazy po ID
    pub fn delete_qso(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM qso_records WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Bezpieczne masowe usuwanie wielu rekordów QSO po ID
    pub fn delete_multiple_qsos(&self, ids: &[i64]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare("DELETE FROM qso_records WHERE id = ?1")?;
            for id in ids {
                stmt.execute(params![id])?;
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    /// Aktualizuje istniejący rekord QSO w bazie po ID
    pub fn update_qso(&self, id: i64, qso: &QsoRecord) -> Result<()> {
        let stored = for_storage(qso);
        let data_params = bind_params(&stored);
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 1);
        params.extend_from_slice(&data_params);
        params.push(&id);
        let mut stmt = self.conn.prepare_cached(&QSO_SQL.update)?;
        stmt.execute(params.as_slice())?;
        Ok(())
    }

    /// Pobiera ostatnio zarejestrowane łączności dla wybranego profilu/dziennika
    pub fn get_recent_qsos_for_journal(
        &self,
        journal_id: &str,
        limit: usize,
    ) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE journal_id = ?1 ORDER BY {ORDER_CHRONO_DESC} LIMIT ?2",
            QSO_SQL.select_columns
        );
        let mut stmt = self.conn.prepare(&sql)?;
        collect_qsos(&mut stmt, params![journal_id, limit as i64])
    }

    /// Chronological position of every QSO within its journal, independent of
    /// the logbook's pagination, search and display sort.
    pub fn qso_numbers_for_journal(&self, journal_id: &str) -> Result<HashMap<i64, usize>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, ROW_NUMBER() OVER (ORDER BY {ORDER_CHRONO_ASC}) \
             FROM qso_records WHERE journal_id = ?1"
        ))?;
        let rows = stmt.query_map(params![journal_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as usize))
        })?;
        rows.collect()
    }

    /// Pobiera ostatnio zarejestrowane łączności (domyślnie)
    pub fn get_recent_qsos(&self, limit: usize) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records ORDER BY {ORDER_CHRONO_DESC} LIMIT ?1",
            QSO_SQL.select_columns
        );
        let mut stmt = self.conn.prepare(&sql)?;
        collect_qsos(&mut stmt, params![limit as i64])
    }

    /// Pobiera stronicowaną listę łączności bezpośrednio w SQLite (`LIMIT ?1 OFFSET ?2`).
    pub fn get_qsos_paginated(&self, limit: usize, offset: usize) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records ORDER BY {ORDER_CHRONO_DESC} LIMIT ?1 OFFSET ?2",
            QSO_SQL.select_columns
        );
        let mut stmt = self.conn.prepare(&sql)?;
        collect_qsos(&mut stmt, params![limit as i64, offset as i64])
    }

    /// Pobiera unikalne, niepuste znaki wywoławcze z całego dziennika (do
    /// lokalnego podpowiadania SCP i korekty rozmytej). Ograniczone limitem.
    pub fn get_distinct_callsigns(&self, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT UPPER(callsign) FROM qso_records \
             WHERE callsign IS NOT NULL AND TRIM(callsign) <> '' \
             ORDER BY callsign LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get::<_, String>(0))?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    /// Zwraca zbiór tożsamości łączności (`QsoIdentity`) już zapisanych w wybranym dzienniku.
    /// Używane do idempotentnego importu ADIF: ponowny import identycznego pliku nie tworzy
    /// duplikatów, bo rekordy o tej samej tożsamości są pomijane.
    pub fn existing_identities(&self, journal_id: &str) -> Result<HashSet<QsoIdentity>> {
        let mut stmt = self.conn.prepare(
            "SELECT callsign, band, mode, qso_date, time_on \
             FROM qso_records WHERE journal_id = ?1",
        )?;
        let rows = stmt.query_map(params![journal_id], |row| {
            let callsign: String = row.get(0)?;
            let band: String = row.get(1)?;
            let mode: String = row.get(2)?;
            let qso_date: String = row.get(3)?;
            let time_on: String = row.get(4)?;
            Ok(QsoIdentity::from_fields(
                &callsign, &band, &mode, &qso_date, &time_on,
            ))
        })?;
        rows.collect()
    }

    /// Pobiera wszystkie łączności z logu (np. do eksportu całego dziennika)
    pub fn get_all_qsos(&self) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records ORDER BY {ORDER_CHRONO_DESC}",
            QSO_SQL.select_columns
        );
        let mut stmt = self.conn.prepare(&sql)?;
        collect_qsos(&mut stmt, [])
    }

    /// Liczba wszystkich łączności w logu
    pub fn count_all(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM qso_records", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Liczba łączności w pojedynczym dzienniku.
    pub fn count_qsos_for_journal(&self, journal_id: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM qso_records WHERE journal_id = ?1",
            [journal_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn checkpoint_and_optimize_if_needed(&self) -> rusqlite::Result<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))?;
        if count > 0 && count % 1000 == 0 {
            self.conn
                .execute_batch("PRAGMA wal_checkpoint(PASSIVE); PRAGMA optimize;")?;
        }
        Ok(())
    }

    /// Dodaje QSO do kolejki ponownego przesyłania
    pub fn queue_upload(
        &self,
        qso_id: i64,
        service: &str,
        adif_data: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO upload_queue (qso_id, service, adif_data) VALUES (?1, ?2, ?3)",
            params![qso_id, service, adif_data],
        )?;
        Ok(())
    }

    /// Pobiera oczekujące wpisy z kolejki (max 50, retry < 5)
    pub fn get_pending_uploads(
        &self,
        service: &str,
    ) -> rusqlite::Result<Vec<(i64, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, qso_id, adif_data FROM upload_queue WHERE service = ?1 AND retry_count < 5 ORDER BY created_at LIMIT 50"
        )?;
        let rows = stmt.query_map([service], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    /// Usuwa wpis z kolejki po udanym przesłaniu
    pub fn remove_from_queue(&self, queue_id: i64) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM upload_queue WHERE id = ?1", params![queue_id])?;
        Ok(())
    }

    /// Zwiększa licznik prób i zapisuje błąd
    pub fn mark_upload_failed(&self, queue_id: i64, error: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE upload_queue SET retry_count = retry_count + 1, last_error = ?1 WHERE id = ?2",
            params![error, queue_id],
        )?;
        Ok(())
    }
}
