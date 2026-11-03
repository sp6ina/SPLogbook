pub type DistinctCallsigns = Vec<String>;

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

    /// Pobiera QSO po ID w kontekście konkretnego profilu/dziennika.
    /// Zwraca None, jeśli rekord o podanym ID nie istnieje w tym dzienniku.
    pub fn get_qso_by_id_scoped(&self, id: i64, journal_id: &str) -> Result<Option<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE id = ?1 AND journal_id = ?2",
            QSO_SQL.select_columns
        );

        self.conn
            .query_row(&sql, params![id, journal_id], qso_from_row)
            .optional()
    }

    /// Wstawia nowy rekord QSO do bazy i zwraca nadane ID
    pub fn insert_qso(&self, qso: &QsoRecord) -> Result<i64> {
        let stored = for_storage(qso);
        let data_params = bind_params(&stored);
        let identity_key = qso.identity().canonical_key();

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 1);
        params.extend_from_slice(&data_params);
        params.push(&identity_key);

        let mut stmt = self.conn.prepare_cached(&QSO_SQL.insert)?;
        stmt.execute(params.as_slice())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Przywraca usunięte QSO zachowując jego oryginalny identyfikator pod warunkiem zgodności z journal_id
    pub fn restore_qso_scoped(&self, journal_id: &str, qso: &QsoRecord) -> Result<i64> {
        let mut owned = qso.clone();
        owned.journal_id = Some(journal_id.to_string());
        let Some(id) = owned.id else {
            return self.insert_qso(&owned);
        };
        let stored = for_storage(&owned);
        let data_params = bind_params(&stored);
        let identity_key = owned.identity().canonical_key();

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 2);
        params.extend_from_slice(&data_params);
        params.push(&id);
        params.push(&identity_key);

        let mut stmt = self.conn.prepare_cached(&QSO_SQL.insert_with_id)?;
        stmt.execute(params.as_slice())?;
        Ok(id)
    }

    /// Bezpieczne atomowe przywracanie wielu łączności w pojedynczej transakcji
    pub fn restore_multiple_qsos_scoped(
        &self,
        journal_id: &str,
        qsos: &[QsoRecord],
    ) -> Result<Vec<i64>> {
        if qsos.is_empty() {
            return Ok(Vec::new());
        }
        let tx = self.conn.unchecked_transaction()?;
        let mut restored_ids = Vec::with_capacity(qsos.len());
        {
            let mut stmt_with_id = tx.prepare_cached(&QSO_SQL.insert_with_id)?;
            let mut stmt_no_id = tx.prepare_cached(&QSO_SQL.insert)?;
            for q in qsos {
                let mut owned = q.clone();
                owned.journal_id = Some(journal_id.to_string());
                let stored = for_storage(&owned);
                let data_params = bind_params(&stored);
                let identity_key = owned.identity().canonical_key();

                if let Some(id) = owned.id {
                    let mut params: Vec<&dyn rusqlite::ToSql> =
                        Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 2);
                    params.extend_from_slice(&data_params);
                    params.push(&id);
                    params.push(&identity_key);
                    stmt_with_id.execute(params.as_slice())?;
                    restored_ids.push(id);
                } else {
                    let mut params: Vec<&dyn rusqlite::ToSql> =
                        Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 1);
                    params.extend_from_slice(&data_params);
                    params.push(&identity_key);
                    stmt_no_id.execute(params.as_slice())?;
                    let new_id = tx.last_insert_rowid();
                    restored_ids.push(new_id);
                }
            }
        }
        tx.commit()?;
        Ok(restored_ids)
    }

    /// Przywraca usunięte QSO zachowując jego oryginalny identyfikator
    pub fn restore_qso(&self, qso: &QsoRecord) -> Result<i64> {
        let jid = qso.journal_id.as_deref().unwrap_or("DEFAULT");
        self.restore_qso_scoped(jid, qso)
    }

    /// Masowe wstawianie łączności w pojedynczej transakcji (bardzo szybki import ADIF)
    pub fn batch_insert_qsos(&self, qsos: &[QsoRecord]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let mut count = 0;
        {
            let mut stmt = tx.prepare_cached(&QSO_SQL.insert)?;
            for qso in qsos {
                let stored = for_storage(qso);
                let data_params = bind_params(&stored);
                let identity_key = qso.identity().canonical_key();

                let mut params: Vec<&dyn rusqlite::ToSql> =
                    Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 1);
                params.extend_from_slice(&data_params);
                params.push(&identity_key);

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

    /// Pobiera listę poprzednich łączności z daną stacją w wybranym profilu/dzienniku.
    pub fn find_previous_qsos_for_journal(
        &self,
        journal_id: &str,
        callsign: &str,
    ) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE journal_id = ?1 AND callsign = ?2 ORDER BY {ORDER_CHRONO_DESC}",
            QSO_SQL.select_columns
        );

        let mut stmt = self.conn.prepare(&sql)?;

        collect_qsos(&mut stmt, params![journal_id, callsign.to_uppercase()])
    }

    /// Usuwa łączność z bazy po ID z wymuszeniem przynależności do journal_id
    pub fn delete_qso_scoped(&self, id: i64, journal_id: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM qso_records WHERE id = ?1 AND journal_id = ?2",
            params![id, journal_id],
        )?;
        Ok(rows > 0)
    }

    /// Bezpieczne masowe usuwanie wielu rekordów QSO po ID z wymuszeniem przynależności do journal_id
    pub fn delete_multiple_qsos_scoped(&self, ids: &[i64], journal_id: &str) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }

        let mut count = 0;
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt =
                tx.prepare("DELETE FROM qso_records WHERE id = ?1 AND journal_id = ?2")?;
            for id in ids {
                count += stmt.execute(params![id, journal_id])?;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    /// Usuwa łączność z bazy po ID (domyślny dziennik lub bez sprawdzania)
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
                count += stmt.execute(params![id])?;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    /// Aktualizuje istniejący rekord QSO w bazie po ID pod warunkiem zgodności journal_id
    pub fn update_qso_scoped(&self, id: i64, journal_id: &str, qso: &QsoRecord) -> Result<bool> {
        let mut owned = qso.clone();
        owned.journal_id = Some(journal_id.to_string());
        let stored = for_storage(&owned);
        let data_params = bind_params(&stored);
        let identity_key = qso.identity().canonical_key();

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 3);
        params.extend_from_slice(&data_params);
        params.push(&identity_key);
        params.push(&id);
        params.push(&journal_id);

        let mut stmt = self.conn.prepare_cached(&QSO_SQL.update_scoped)?;
        let rows = stmt.execute(params.as_slice())?;
        Ok(rows > 0)
    }

    /// Aktualizuje istniejący rekord QSO w bazie po ID
    pub fn update_qso(&self, id: i64, qso: &QsoRecord) -> Result<()> {
        let stored = for_storage(qso);
        let data_params = bind_params(&stored);
        let identity_key = qso.identity().canonical_key();

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(QSO_DATA_COLUMN_COUNT + 2);
        params.extend_from_slice(&data_params);
        params.push(&identity_key);
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

    /// Pobiera wszystkie łączności dla wybranego profilu/dziennika

    pub fn get_all_qsos_for_journal(&self, journal_id: &str) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE journal_id = ?1 ORDER BY {ORDER_CHRONO_ASC}",
            QSO_SQL.select_columns
        );

        let mut stmt = self.conn.prepare(&sql)?;

        collect_qsos(&mut stmt, params![journal_id])
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

    /// Pobiera stronicowaną listę łączności dla wybranego profilu/dziennika (`LIMIT ?2 OFFSET ?3`).
    pub fn get_qsos_paginated_for_journal(
        &self,
        journal_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE journal_id = ?1 ORDER BY {ORDER_CHRONO_DESC} LIMIT ?2 OFFSET ?3",
            QSO_SQL.select_columns
        );

        let mut stmt = self.conn.prepare(&sql)?;

        collect_qsos(&mut stmt, params![journal_id, limit as i64, offset as i64])
    }

    /// Pobiera unikalne, niepuste znaki wywoławcze z całego dziennika (do

    /// lokalnego podpowiadania SCP i korekty rozmytej). Ograniczone limitem.

    pub fn get_distinct_callsigns(&self, limit: usize) -> Result<DistinctCallsigns> {
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

    /// Pobiera unikalne, niepuste znaki wywoławcze z wybranego profilu/dziennika.
    pub fn get_distinct_callsigns_for_journal(
        &self,
        journal_id: &str,
        limit: usize,
    ) -> Result<DistinctCallsigns> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT UPPER(callsign) FROM qso_records \
             WHERE journal_id = ?1 AND callsign IS NOT NULL AND TRIM(callsign) <> '' \
             ORDER BY callsign LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![journal_id, limit as i64], |row| {
            row.get::<_, String>(0)
        })?;

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
            "SELECT callsign, band, mode, submode, qso_date, time_on, freq \
             FROM qso_records WHERE journal_id = ?1",
        )?;

        let rows = stmt.query_map(params![journal_id], |row| {
            let callsign: String = row.get(0)?;
            let band: String = row.get(1)?;
            let mode: String = row.get(2)?;
            let submode: Option<String> = row.get(3)?;
            let qso_date: String = row.get(4)?;
            let time_on: String = row.get(5)?;
            let freq: Option<f64> = row.get(6)?;

            Ok(QsoIdentity::from_detailed_fields(
                &callsign,
                &band,
                freq,
                &mode,
                submode.as_deref(),
                &qso_date,
                &time_on,
            ))
        })?;

        rows.collect()
    }

    /// Sprawdza, czy w danym dzienniku istnieje już rekord o podanej tożsamości `QsoIdentity`.

    /// Ogranicza kandydatów przez UPPER(callsign), a ostateczne porównanie wykonuje przez

    /// `QsoIdentity` w Rust z zachowaniem precyzji sekund, częstotliwości i subemisji.

    pub fn has_identity(&self, journal_id: &str, identity: &QsoIdentity) -> Result<bool> {
        let Some(canonical_key) = identity.canonical_key() else {
            return Ok(false);
        };
        let mut stmt = self.conn.prepare_cached(
            "SELECT 1 FROM qso_records WHERE journal_id = ?1 AND identity_key = ?2 LIMIT 1",
        )?;
        stmt.exists(params![journal_id, canonical_key])
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
}
