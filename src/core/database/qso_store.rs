// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use super::schema::{QSO_COLUMNS, row_to_qso};
use crate::core::qso::QsoRecord;
use rusqlite::{Result, params};
use std::collections::{HashMap, HashSet};

impl LogDatabase {
    /// Pobiera QSO po ID
    pub fn get_qso_by_id(&self, id: i64) -> Result<Option<QsoRecord>> {
        let sql = format!("SELECT {QSO_COLUMNS} FROM qso_records WHERE id = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![id], row_to_qso)?;
        if let Some(Ok(qso)) = rows.next() {
            Ok(Some(qso))
        } else {
            Ok(None)
        }
    }

    /// Wstawia nowy rekord QSO do bazy i zwraca nadane ID
    pub fn insert_qso(&self, qso: &QsoRecord) -> Result<i64> {
        let journal = qso.journal_id.as_deref().unwrap_or("DEFAULT");
        self.conn.execute(
            "INSERT INTO qso_records (
                callsign, band, mode, submode, qso_date, time_on, time_off,
                freq, freq_rx, rst_sent, rst_rcvd, name, qth, gridsquare,
                state, iota, sota_ref, pota_ref, pga_ref, dxcc, country,
                continent, cqz, ituz, comment, qsl_via, qsl_manager,
                qsl_sent, qsl_rcvd, qsl_sent_date, qsl_rcvd_date,
                lotw_qsl_sent, lotw_qsl_rcvd, lotw_qslrdate,
                eqsl_qsl_sent, eqsl_qsl_rcvd, eqsl_qslrdate,
                clublog_upload_status, qrzcom_upload_status,
                sat_name, sat_mode, prop_mode, srx, stx, srx_string, stx_string,
                my_gridsquare, my_state, my_pota_ref, my_sota_ref, vucc_grids, audio_file, journal_id
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18, ?19, ?20, ?21,
                ?22, ?23, ?24, ?25, ?26, ?27,
                ?28, ?29, ?30, ?31,
                ?32, ?33, ?34,
                ?35, ?36, ?37,
                ?38, ?39,
                ?40, ?41, ?42, ?43, ?44, ?45, ?46,
                ?47, ?48, ?49, ?50, ?51, ?52, ?53
            )",
            params![
                qso.callsign.to_uppercase(),
                qso.band,
                qso.mode.to_uppercase(),
                qso.submode,
                qso.qso_date,
                qso.time_on,
                qso.time_off,
                qso.freq,
                qso.freq_rx,
                qso.rst_sent,
                qso.rst_rcvd,
                qso.name,
                qso.qth,
                qso.gridsquare,
                qso.state,
                qso.iota,
                qso.sota_ref,
                qso.pota_ref,
                qso.pga_ref,
                qso.dxcc,
                qso.country,
                qso.continent,
                qso.cqz,
                qso.ituz,
                qso.comment,
                qso.qsl_via,
                qso.qsl_manager,
                qso.qsl_sent,
                qso.qsl_rcvd,
                qso.qsl_sent_date,
                qso.qsl_rcvd_date,
                qso.lotw_qsl_sent,
                qso.lotw_qsl_rcvd,
                qso.lotw_qslrdate,
                qso.eqsl_qsl_sent,
                qso.eqsl_qsl_rcvd,
                qso.eqsl_qslrdate,
                qso.clublog_upload_status,
                qso.qrzcom_upload_status,
                qso.sat_name,
                qso.sat_mode,
                qso.prop_mode,
                qso.srx,
                qso.stx,
                qso.srx_string,
                qso.stx_string,
                qso.my_gridsquare,
                qso.my_state,
                qso.my_pota_ref,
                qso.my_sota_ref,
                qso.vucc_grids,
                qso.audio_file,
                journal,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Przywraca usunięte QSO zachowując jego oryginalny identyfikator.
    /// Używane przez Undo, aby redo oraz ewentualne referencje zachowały
    /// tożsamość rekordu zamiast tworzyć nowe ID.
    pub fn restore_qso(&self, qso: &QsoRecord) -> Result<i64> {
        let Some(id) = qso.id else {
            return self.insert_qso(qso);
        };
        let journal = qso.journal_id.as_deref().unwrap_or("DEFAULT");
        self.conn.execute(
            "INSERT INTO qso_records (
                id,
                callsign, band, mode, submode, qso_date, time_on, time_off,
                freq, freq_rx, rst_sent, rst_rcvd, name, qth, gridsquare,
                state, iota, sota_ref, pota_ref, pga_ref, dxcc, country,
                continent, cqz, ituz, comment, qsl_via, qsl_manager,
                qsl_sent, qsl_rcvd, qsl_sent_date, qsl_rcvd_date,
                lotw_qsl_sent, lotw_qsl_rcvd, lotw_qslrdate,
                eqsl_qsl_sent, eqsl_qsl_rcvd, eqsl_qslrdate,
                clublog_upload_status, qrzcom_upload_status,
                sat_name, sat_mode, prop_mode, srx, stx, srx_string, stx_string,
                my_gridsquare, my_state, my_pota_ref, my_sota_ref, vucc_grids, audio_file, journal_id
            ) VALUES (
                ?54,
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                ?15, ?16, ?17, ?18, ?19, ?20, ?21,
                ?22, ?23, ?24, ?25, ?26, ?27,
                ?28, ?29, ?30, ?31,
                ?32, ?33, ?34,
                ?35, ?36, ?37,
                ?38, ?39,
                ?40, ?41, ?42, ?43, ?44, ?45, ?46,
                ?47, ?48, ?49, ?50, ?51, ?52, ?53
            )",
            params![
                qso.callsign.to_uppercase(),
                qso.band,
                qso.mode.to_uppercase(),
                qso.submode,
                qso.qso_date,
                qso.time_on,
                qso.time_off,
                qso.freq,
                qso.freq_rx,
                qso.rst_sent,
                qso.rst_rcvd,
                qso.name,
                qso.qth,
                qso.gridsquare,
                qso.state,
                qso.iota,
                qso.sota_ref,
                qso.pota_ref,
                qso.pga_ref,
                qso.dxcc,
                qso.country,
                qso.continent,
                qso.cqz,
                qso.ituz,
                qso.comment,
                qso.qsl_via,
                qso.qsl_manager,
                qso.qsl_sent,
                qso.qsl_rcvd,
                qso.qsl_sent_date,
                qso.qsl_rcvd_date,
                qso.lotw_qsl_sent,
                qso.lotw_qsl_rcvd,
                qso.lotw_qslrdate,
                qso.eqsl_qsl_sent,
                qso.eqsl_qsl_rcvd,
                qso.eqsl_qslrdate,
                qso.clublog_upload_status,
                qso.qrzcom_upload_status,
                qso.sat_name,
                qso.sat_mode,
                qso.prop_mode,
                qso.srx,
                qso.stx,
                qso.srx_string,
                qso.stx_string,
                qso.my_gridsquare,
                qso.my_state,
                qso.my_pota_ref,
                qso.my_sota_ref,
                qso.vucc_grids,
                qso.audio_file,
                journal,
                id,
            ],
        )?;

        Ok(id)
    }

    /// Masowe wstawianie łączności w pojedynczej transakcji (bardzo szybki import ADIF)
    pub fn batch_insert_qsos(&mut self, qsos: &[QsoRecord]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut count = 0;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO qso_records (
                    callsign, band, mode, submode, qso_date, time_on, time_off,
                    freq, freq_rx, rst_sent, rst_rcvd, name, qth, gridsquare,
                    state, iota, sota_ref, pota_ref, pga_ref, dxcc, country,
                    continent, cqz, ituz, comment, qsl_via, qsl_manager,
                    qsl_sent, qsl_rcvd, qsl_sent_date, qsl_rcvd_date,
                    lotw_qsl_sent, lotw_qsl_rcvd, lotw_qslrdate,
                    eqsl_qsl_sent, eqsl_qsl_rcvd, eqsl_qslrdate,
                    clublog_upload_status, qrzcom_upload_status,
                    sat_name, sat_mode, prop_mode, srx, stx, srx_string, stx_string,
                    my_gridsquare, my_state, my_pota_ref, my_sota_ref, vucc_grids, audio_file, journal_id
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                    ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                    ?15, ?16, ?17, ?18, ?19, ?20, ?21,
                    ?22, ?23, ?24, ?25, ?26, ?27,
                    ?28, ?29, ?30, ?31,
                    ?32, ?33, ?34,
                    ?35, ?36, ?37,
                    ?38, ?39,
                    ?40, ?41, ?42, ?43, ?44, ?45, ?46,
                    ?47, ?48, ?49, ?50, ?51, ?52, ?53
                )",
            )?;

            for qso in qsos {
                let journal = qso.journal_id.as_deref().unwrap_or("DEFAULT");
                stmt.execute(params![
                    qso.callsign.to_uppercase(),
                    qso.band,
                    qso.mode.to_uppercase(),
                    qso.submode,
                    qso.qso_date,
                    qso.time_on,
                    qso.time_off,
                    qso.freq,
                    qso.freq_rx,
                    qso.rst_sent,
                    qso.rst_rcvd,
                    qso.name,
                    qso.qth,
                    qso.gridsquare,
                    qso.state,
                    qso.iota,
                    qso.sota_ref,
                    qso.pota_ref,
                    qso.pga_ref,
                    qso.dxcc,
                    qso.country,
                    qso.continent,
                    qso.cqz,
                    qso.ituz,
                    qso.comment,
                    qso.qsl_via,
                    qso.qsl_manager,
                    qso.qsl_sent,
                    qso.qsl_rcvd,
                    qso.qsl_sent_date,
                    qso.qsl_rcvd_date,
                    qso.lotw_qsl_sent,
                    qso.lotw_qsl_rcvd,
                    qso.lotw_qslrdate,
                    qso.eqsl_qsl_sent,
                    qso.eqsl_qsl_rcvd,
                    qso.eqsl_qslrdate,
                    qso.clublog_upload_status,
                    qso.qrzcom_upload_status,
                    qso.sat_name,
                    qso.sat_mode,
                    qso.prop_mode,
                    qso.srx,
                    qso.stx,
                    qso.srx_string,
                    qso.stx_string,
                    qso.my_gridsquare,
                    qso.my_state,
                    qso.my_pota_ref,
                    qso.my_sota_ref,
                    qso.vucc_grids,
                    qso.audio_file,
                    journal,
                ])?;
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    /// Pobiera listę poprzednich łączności z daną stacją (do podglądu w locie)
    pub fn find_previous_qsos(&self, callsign: &str) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {QSO_COLUMNS} FROM qso_records WHERE callsign = ?1 ORDER BY qso_date DESC, time_on DESC"
        );
        let mut stmt = self.conn.prepare(&sql)?;

        let rows = stmt.query_map(params![callsign.to_uppercase()], row_to_qso)?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
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
        let journal = qso.journal_id.as_deref().unwrap_or("DEFAULT");
        self.conn.execute(
            "UPDATE qso_records SET
                callsign = ?1, band = ?2, mode = ?3, submode = ?4, qso_date = ?5,
                time_on = ?6, time_off = ?7, freq = ?8, freq_rx = ?9, rst_sent = ?10,
                rst_rcvd = ?11, name = ?12, qth = ?13, gridsquare = ?14, state = ?15,
                iota = ?16, sota_ref = ?17, pota_ref = ?18, pga_ref = ?19, dxcc = ?20,
                country = ?21, continent = ?22, cqz = ?23, ituz = ?24, comment = ?25,
                qsl_via = ?26, qsl_manager = ?27, qsl_sent = ?28, qsl_rcvd = ?29,
                qsl_sent_date = ?30, qsl_rcvd_date = ?31, lotw_qsl_sent = ?32,
                lotw_qsl_rcvd = ?33, lotw_qslrdate = ?34, eqsl_qsl_sent = ?35,
                eqsl_qsl_rcvd = ?36, eqsl_qslrdate = ?37, clublog_upload_status = ?38,
                qrzcom_upload_status = ?39, sat_name = ?40, sat_mode = ?41, prop_mode = ?42,
                srx = ?43, stx = ?44, srx_string = ?45, stx_string = ?46,
                my_gridsquare = ?47, my_state = ?48, my_pota_ref = ?49, my_sota_ref = ?50,
                vucc_grids = ?51, audio_file = ?52, journal_id = ?53
             WHERE id = ?54",
            params![
                qso.callsign.to_uppercase(),
                qso.band,
                qso.mode.to_uppercase(),
                qso.submode,
                qso.qso_date,
                qso.time_on,
                qso.time_off,
                qso.freq,
                qso.freq_rx,
                qso.rst_sent,
                qso.rst_rcvd,
                qso.name,
                qso.qth,
                qso.gridsquare,
                qso.state,
                qso.iota,
                qso.sota_ref,
                qso.pota_ref,
                qso.pga_ref,
                qso.dxcc,
                qso.country,
                qso.continent,
                qso.cqz,
                qso.ituz,
                qso.comment,
                qso.qsl_via,
                qso.qsl_manager,
                qso.qsl_sent,
                qso.qsl_rcvd,
                qso.qsl_sent_date,
                qso.qsl_rcvd_date,
                qso.lotw_qsl_sent,
                qso.lotw_qsl_rcvd,
                qso.lotw_qslrdate,
                qso.eqsl_qsl_sent,
                qso.eqsl_qsl_rcvd,
                qso.eqsl_qslrdate,
                qso.clublog_upload_status,
                qso.qrzcom_upload_status,
                qso.sat_name,
                qso.sat_mode,
                qso.prop_mode,
                qso.srx,
                qso.stx,
                qso.srx_string,
                qso.stx_string,
                qso.my_gridsquare,
                qso.my_state,
                qso.my_pota_ref,
                qso.my_sota_ref,
                qso.vucc_grids,
                qso.audio_file,
                journal,
                id
            ],
        )?;
        Ok(())
    }

    /// Pobiera ostatnio zarejestrowane łączności dla wybranego profilu/dziennika
    pub fn get_recent_qsos_for_journal(
        &self,
        journal_id: &str,
        limit: usize,
    ) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {QSO_COLUMNS} FROM qso_records WHERE journal_id = ?1 ORDER BY REPLACE(qso_date, '-', '') DESC, SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6) DESC, id DESC LIMIT ?2"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![journal_id, limit as i64], row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    /// Chronological position of every QSO within its journal, independent of
    /// the logbook's pagination, search and display sort.
    pub fn qso_numbers_for_journal(&self, journal_id: &str) -> Result<HashMap<i64, usize>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ROW_NUMBER() OVER (ORDER BY REPLACE(qso_date, '-', ''), SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6), id)
             FROM qso_records WHERE journal_id = ?1"
        )?;
        let rows = stmt.query_map(params![journal_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as usize))
        })?;
        rows.collect()
    }

    /// Pobiera ostatnio zarejestrowane łączności (domyślnie)
    pub fn get_recent_qsos(&self, limit: usize) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {QSO_COLUMNS} FROM qso_records ORDER BY REPLACE(qso_date, '-', '') DESC, SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6) DESC, id DESC LIMIT ?1"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64], row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    /// Pobiera stronicowaną listę łączności bezpośrednio w SQLite (`LIMIT ?1 OFFSET ?2`).
    pub fn get_qsos_paginated(&self, limit: usize, offset: usize) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {QSO_COLUMNS} FROM qso_records ORDER BY REPLACE(qso_date, '-', '') DESC, SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6) DESC, id DESC LIMIT ?1 OFFSET ?2"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64, offset as i64], row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
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

    /// Zwraca zbiór kluczy jednoznaczności już zapisanych w wybranym dzienniku.
    /// Klucz to znormalizowane `callsign|band|mode|data|czas`, używane do
    /// idempotentnego importu ADIF: ponowny import identycznego pliku nie tworzy
    /// duplikatów, bo rekordy o tym samym kluczu są pomijane.
    pub fn existing_qso_keys(&self, journal_id: &str) -> Result<HashSet<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT UPPER(callsign), UPPER(band), UPPER(mode),
                    REPLACE(COALESCE(qso_date, ''), '-', ''),
                    REPLACE(COALESCE(time_on, ''), ':', '')
             FROM qso_records WHERE journal_id = ?1",
        )?;
        let rows = stmt.query_map(params![journal_id], |row| {
            Ok(format!(
                "{}|{}|{}|{}|{}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?
            ))
        })?;
        rows.collect()
    }

    /// Pobiera wszystkie łączności z logu (np. do eksportu całego dziennika)
    pub fn get_all_qsos(&self) -> Result<Vec<QsoRecord>> {
        let sql =
            format!("SELECT {QSO_COLUMNS} FROM qso_records ORDER BY qso_date DESC, time_on DESC");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
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
