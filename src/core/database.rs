// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use rusqlite::{params, Connection, Result, Row};
use std::path::Path;

/// Reprezentacja profilu / dziennika łączności (Wielodziennikowość)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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

/// Zaawansowany filtr wyszukiwania łączności
#[derive(Debug, Clone, Default)]
pub struct AdvancedQsoFilter {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub bands: Vec<String>,
    pub modes: Vec<String>,
    pub lotw_confirmed: Option<bool>,
    pub eqsl_confirmed: Option<bool>,
    pub qsl_rcvd: Option<bool>,
    pub callsign_query: Option<String>,
    pub journal_id: Option<String>,
}

const QSO_COLUMNS: &str = "id, callsign, band, mode, submode, qso_date, time_on, time_off,
    freq, freq_rx, rst_sent, rst_rcvd, name, qth, gridsquare,
    state, iota, sota_ref, pota_ref, pga_ref, dxcc, country,
    continent, cqz, ituz, comment, qsl_via, qsl_manager,
    qsl_sent, qsl_rcvd, qsl_sent_date, qsl_rcvd_date,
    lotw_qsl_sent, lotw_qsl_rcvd, lotw_qslrdate,
    eqsl_qsl_sent, eqsl_qsl_rcvd, eqsl_qslrdate,
    clublog_upload_status, qrzcom_upload_status,
    sat_name, sat_mode, prop_mode, srx, stx, srx_string, stx_string,
    my_gridsquare, my_state, my_pota_ref, my_sota_ref, vucc_grids, audio_file, journal_id";

fn row_to_qso(row: &Row) -> Result<QsoRecord> {
    Ok(QsoRecord {
        id: Some(row.get(0)?),
        callsign: row.get(1)?,
        band: row.get(2)?,
        mode: row.get(3)?,
        submode: row.get(4)?,
        qso_date: row.get(5)?,
        time_on: row.get(6)?,
        time_off: row.get(7)?,
        freq: row.get(8)?,
        freq_rx: row.get(9)?,
        rst_sent: row.get(10)?,
        rst_rcvd: row.get(11)?,
        name: row.get(12)?,
        qth: row.get(13)?,
        gridsquare: row.get(14)?,
        state: row.get(15)?,
        iota: row.get(16)?,
        sota_ref: row.get(17)?,
        pota_ref: row.get(18)?,
        pga_ref: row.get(19)?,
        dxcc: row.get(20)?,
        country: row.get(21)?,
        continent: row.get(22)?,
        cqz: row.get(23)?,
        ituz: row.get(24)?,
        comment: row.get(25)?,
        qsl_via: row.get(26)?,
        qsl_manager: row.get(27)?,
        qsl_sent: row.get(28)?,
        qsl_rcvd: row.get(29)?,
        qsl_sent_date: row.get(30)?,
        qsl_rcvd_date: row.get(31)?,
        lotw_qsl_sent: row.get(32)?,
        lotw_qsl_rcvd: row.get(33)?,
        lotw_qslrdate: row.get(34)?,
        eqsl_qsl_sent: row.get(35)?,
        eqsl_qsl_rcvd: row.get(36)?,
        eqsl_qslrdate: row.get(37)?,
        clublog_upload_status: row.get(38)?,
        qrzcom_upload_status: row.get(39)?,
        sat_name: row.get(40)?,
        sat_mode: row.get(41)?,
        prop_mode: row.get(42)?,
        srx: row.get(43)?,
        stx: row.get(44)?,
        srx_string: row.get(45)?,
        stx_string: row.get(46)?,
        my_gridsquare: row.get(47)?,
        my_state: row.get(48)?,
        my_pota_ref: row.get(49).ok(),
        my_sota_ref: row.get(50).ok(),
        vucc_grids: row.get(51).ok(),
        audio_file: row.get(52)?,
        journal_id: row.get(53).ok(),
    })
}

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

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Otwiera bazę danych w pamięci RAM (np. do testów jednostkowych)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Inicjalizuje schemat tabeli łączności oraz niezbędne indeksy
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=10000;
            PRAGMA temp_store=MEMORY;

            CREATE TABLE IF NOT EXISTS journals (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                station_callsign TEXT NOT NULL,
                operator TEXT NOT NULL,
                my_gridsquare TEXT,
                my_pga TEXT,
                description TEXT,
                is_default INTEGER DEFAULT 0
            );

            INSERT OR IGNORE INTO journals (id, name, station_callsign, operator, my_gridsquare, my_pga, description, is_default)
            VALUES ('DEFAULT', 'Główny Dziennik', '', '', '', '', '', 1);

            CREATE TABLE IF NOT EXISTS qso_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                callsign TEXT NOT NULL,
                band TEXT NOT NULL,
                mode TEXT NOT NULL,
                submode TEXT,
                qso_date TEXT NOT NULL,
                time_on TEXT NOT NULL,
                time_off TEXT,
                freq REAL,
                freq_rx REAL,
                rst_sent TEXT NOT NULL,
                rst_rcvd TEXT NOT NULL,
                name TEXT,
                qth TEXT,
                gridsquare TEXT,
                state TEXT,
                iota TEXT,
                sota_ref TEXT,
                pota_ref TEXT,
                pga_ref TEXT,
                dxcc INTEGER,
                country TEXT,
                continent TEXT,
                cqz INTEGER,
                ituz INTEGER,
                comment TEXT,
                qsl_via TEXT,
                qsl_manager TEXT,
                qsl_sent TEXT DEFAULT 'N',
                qsl_rcvd TEXT DEFAULT 'N',
                qsl_sent_date TEXT,
                qsl_rcvd_date TEXT,
                lotw_qsl_sent TEXT DEFAULT 'N',
                lotw_qsl_rcvd TEXT DEFAULT 'N',
                lotw_qslrdate TEXT,
                eqsl_qsl_sent TEXT DEFAULT 'N',
                eqsl_qsl_rcvd TEXT DEFAULT 'N',
                eqsl_qslrdate TEXT,
                clublog_upload_status TEXT,
                qrzcom_upload_status TEXT,
                sat_name TEXT,
                sat_mode TEXT,
                prop_mode TEXT,
                srx INTEGER,
                stx INTEGER,
                srx_string TEXT,
                stx_string TEXT,
                my_gridsquare TEXT,
                my_state TEXT,
                my_pota_ref TEXT,
                my_sota_ref TEXT,
                vucc_grids TEXT,
                audio_file TEXT,
                journal_id TEXT DEFAULT 'DEFAULT'
            );

            CREATE TABLE IF NOT EXISTS upload_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                qso_id INTEGER NOT NULL,
                service TEXT NOT NULL,
                adif_data TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                retry_count INTEGER NOT NULL DEFAULT 0,
                last_error TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_upload_queue_service ON upload_queue(service, retry_count);

            CREATE INDEX IF NOT EXISTS idx_qso_call ON qso_records(callsign);
            CREATE INDEX IF NOT EXISTS idx_qso_callsign ON qso_records(callsign);
            CREATE INDEX IF NOT EXISTS idx_qso_date ON qso_records(qso_date);
            CREATE INDEX IF NOT EXISTS idx_qso_date_time ON qso_records(qso_date, time_on);
            CREATE INDEX IF NOT EXISTS idx_qso_band ON qso_records(band);
            CREATE INDEX IF NOT EXISTS idx_qso_mode ON qso_records(mode);
            CREATE INDEX IF NOT EXISTS idx_qso_dxcc ON qso_records(dxcc);
            CREATE INDEX IF NOT EXISTS idx_qso_pga ON qso_records(pga_ref);
            CREATE INDEX IF NOT EXISTS idx_qso_journal ON qso_records(journal_id);
            CREATE INDEX IF NOT EXISTS idx_qso_lotw ON qso_records(lotw_qsl_rcvd);
            CREATE INDEX IF NOT EXISTS idx_qso_eqsl ON qso_records(eqsl_qsl_rcvd);
            CREATE INDEX IF NOT EXISTS idx_qso_cqz ON qso_records(cqz);
            CREATE INDEX IF NOT EXISTS idx_qso_composite ON qso_records(callsign, band, mode);"
        )?;

        // Bezpieczna migracja dla istniejących baz bez kolumny journal_id
        let _ = self.conn.execute("ALTER TABLE qso_records ADD COLUMN journal_id TEXT DEFAULT 'DEFAULT'", []);
        let _ = self.conn.execute("ALTER TABLE qso_records ADD COLUMN my_pota_ref TEXT", []);
        let _ = self.conn.execute("ALTER TABLE qso_records ADD COLUMN my_sota_ref TEXT", []);
        let _ = self.conn.execute("ALTER TABLE qso_records ADD COLUMN vucc_grids TEXT", []);
        let _ = self.conn.execute("ALTER TABLE qso_records ADD COLUMN audio_file TEXT", []);

        Ok(())
    }

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
            params![j.id, j.name, j.station_callsign, j.operator, j.my_gridsquare, j.my_pga, j.description, if j.is_default { 1 } else { 0 }],
        )?;
        Ok(())
    }

    /// Aktualizuje dane profilu dziennika
    pub fn update_journal(&self, j: &Journal) -> Result<()> {
        self.conn.execute(
            "UPDATE journals SET name = ?2, station_callsign = ?3, operator = ?4,
             my_gridsquare = ?5, my_pga = ?6, description = ?7, is_default = ?8
             WHERE id = ?1",
            params![j.id, j.name, j.station_callsign, j.operator, j.my_gridsquare, j.my_pga, j.description, if j.is_default { 1 } else { 0 }],
        )?;
        Ok(())
    }

    /// Usuwa profil dziennika (o ile nie jest jedynym domyślnym)
    pub fn delete_journal(&self, id: &str) -> Result<()> {
        if id == "DEFAULT" {
            return Ok(()); // Nie pozwalamy usunąć domyślnego
        }
        self.conn.execute("DELETE FROM journals WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Pobiera aktywny / domyślny dziennik
    pub fn get_active_journal(&self) -> Result<Journal> {
        let j = self.conn.query_row(
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
                    is_default: true,
                })
            },
        ).unwrap_or_default();
        Ok(j)
    }

    /// Ustawia wybrany dziennik jako domyślny (aktywny)
    pub fn set_active_journal(&self, id: &str) -> Result<()> {
        self.conn.execute("UPDATE journals SET is_default = 0", [])?;
        self.conn.execute("UPDATE journals SET is_default = 1 WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Pobiera QSO po ID
    pub fn get_qso_by_id(&self, id: i64) -> Result<Option<QsoRecord>> {
        let sql = format!("SELECT {} FROM qso_records WHERE id = ?1", QSO_COLUMNS);
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
        let sql = format!("SELECT {} FROM qso_records WHERE callsign = ?1 ORDER BY qso_date DESC, time_on DESC", QSO_COLUMNS);
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
        self.conn.execute("DELETE FROM qso_records WHERE id = ?1", params![id])?;
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

    /// Wyszukuje duplikaty łączności w całym logu
    /// Grupuje po znaku, paśmie i emisji (opcjonalnie również po dacie)
    pub fn find_duplicate_qsos(&self, match_same_day: bool) -> Result<Vec<Vec<QsoRecord>>> {
        let group_by = if match_same_day {
            "callsign, band, mode, qso_date"
        } else {
            "callsign, band, mode"
        };

        let dup_keys_sql = format!(
            "SELECT callsign, band, mode{} FROM qso_records GROUP BY {} HAVING COUNT(*) > 1 ORDER BY callsign ASC, band ASC",
            if match_same_day { ", qso_date" } else { "" },
            group_by
        );

        let mut key_stmt = self.conn.prepare(&dup_keys_sql)?;
        let mut groups = Vec::new();

        if match_same_day {
            let keys = key_stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?;

            let query_sql = format!(
                "SELECT {} FROM qso_records WHERE callsign = ?1 AND band = ?2 AND mode = ?3 AND qso_date = ?4 ORDER BY time_on ASC, id ASC",
                QSO_COLUMNS
            );
            let mut q_stmt = self.conn.prepare(&query_sql)?;
            for k in keys {
                let (c, b, m, d) = k?;
                let rows = q_stmt.query_map(params![c, b, m, d], row_to_qso)?;
                let mut cluster = Vec::new();
                for row in rows {
                    cluster.push(row?);
                }
                if cluster.len() > 1 {
                    groups.push(cluster);
                }
            }
        } else {
            let keys = key_stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;

            let query_sql = format!(
                "SELECT {} FROM qso_records WHERE callsign = ?1 AND band = ?2 AND mode = ?3 ORDER BY qso_date ASC, time_on ASC, id ASC",
                QSO_COLUMNS
            );
            let mut q_stmt = self.conn.prepare(&query_sql)?;
            for k in keys {
                let (c, b, m) = k?;
                let rows = q_stmt.query_map(params![c, b, m], row_to_qso)?;
                let mut cluster = Vec::new();
                for row in rows {
                    cluster.push(row?);
                }
                if cluster.len() > 1 {
                    groups.push(cluster);
                }
            }
        }

        Ok(groups)
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

    /// Oznacza potwierdzenie LoTW dla dopasowanej łączności
    pub fn mark_lotw_confirmed(&self, callsign: &str, band: &str, mode: &str, qso_date: &str, rdate: &str) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE qso_records
             SET lotw_qsl_rcvd = 'Y', lotw_qslrdate = ?1
             WHERE callsign = ?2 AND band = ?3 AND mode = ?4 AND qso_date = ?5",
            params![rdate, callsign.to_uppercase(), band, mode.to_uppercase(), qso_date],
        )?;
        Ok(count)
    }

    /// Oznacza potwierdzenie eQSL dla dopasowanej łączności
    pub fn mark_eqsl_confirmed(&self, callsign: &str, band: &str, mode: &str, qso_date: &str, rdate: &str) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE qso_records
             SET eqsl_qsl_rcvd = 'Y', eqsl_qslrdate = ?1
             WHERE callsign = ?2 AND band = ?3 AND mode = ?4 AND qso_date = ?5",
            params![rdate, callsign.to_uppercase(), band, mode.to_uppercase(), qso_date],
        )?;
        Ok(count)
    }

    /// Pobiera ostatnio zarejestrowane łączności dla wybranego profilu/dziennika
    pub fn get_recent_qsos_for_journal(&self, journal_id: &str, limit: usize) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records WHERE journal_id = ?1 ORDER BY qso_date DESC, time_on DESC LIMIT ?2",
            QSO_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![journal_id, limit as i64], row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    /// Pobiera ostatnio zarejestrowane łączności (domyślnie)
    pub fn get_recent_qsos(&self, limit: usize) -> Result<Vec<QsoRecord>> {
        let sql = format!(
            "SELECT {} FROM qso_records ORDER BY qso_date DESC, time_on DESC LIMIT ?1",
            QSO_COLUMNS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64], row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    /// Wyszukiwanie łączności z wieloma kryteriami (zaawansowane filtrowanie)
    pub fn search_qsos_advanced(&self, filter: &AdvancedQsoFilter) -> Result<Vec<QsoRecord>> {
        let mut sql = format!("SELECT {} FROM qso_records WHERE 1=1", QSO_COLUMNS);
        let mut conditions = Vec::new();

        if let Some(ref jid) = filter.journal_id {
            conditions.push(format!("journal_id = '{}'", jid.replace('\'', "''")));
        }
        if let Some(ref from) = filter.date_from {
            if !from.is_empty() {
                conditions.push(format!("qso_date >= '{}'", from.replace('\'', "''")));
            }
        }
        if let Some(ref to) = filter.date_to {
            if !to.is_empty() {
                conditions.push(format!("qso_date <= '{}'", to.replace('\'', "''")));
            }
        }
        if !filter.bands.is_empty() {
            let b_list = filter.bands.iter().map(|b| format!("'{}'", b.replace('\'', "''"))).collect::<Vec<_>>().join(",");
            conditions.push(format!("band IN ({})", b_list));
        }
        if !filter.modes.is_empty() {
            let m_list = filter.modes.iter().map(|m| format!("'{}'", m.replace('\'', "''"))).collect::<Vec<_>>().join(",");
            conditions.push(format!("mode IN ({})", m_list));
        }
        if let Some(lotw) = filter.lotw_confirmed {
            if lotw {
                conditions.push("lotw_qsl_rcvd = 'Y'".to_string());
            }
        }
        if let Some(eqsl) = filter.eqsl_confirmed {
            if eqsl {
                conditions.push("eqsl_qsl_rcvd = 'Y'".to_string());
            }
        }
        if let Some(qsl) = filter.qsl_rcvd {
            if qsl {
                conditions.push("qsl_rcvd = 'Y'".to_string());
            }
        }
        if let Some(ref query) = filter.callsign_query {
            let q = query.trim().to_uppercase().replace('\'', "''");
            if !q.is_empty() {
                conditions.push(format!("(callsign LIKE '%{}%' OR name LIKE '%{}%' OR comment LIKE '%{}%')", q, q, q));
            }
        }

        for c in conditions {
            sql.push_str(" AND ");
            sql.push_str(&c);
        }

        sql.push_str(" ORDER BY qso_date DESC, time_on DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    /// Pobiera wszystkie łączności z logu (np. do eksportu całego dziennika)
    pub fn get_all_qsos(&self) -> Result<Vec<QsoRecord>> {
        let sql = format!("SELECT {} FROM qso_records ORDER BY qso_date DESC, time_on DESC", QSO_COLUMNS);
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
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn vacuum_if_needed(&self) -> rusqlite::Result<()> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))?;
        if count > 0 && count % 1000 == 0 {
            self.conn.execute_batch("VACUUM;")?;
        }
        Ok(())
    }

    /// Dodaje QSO do kolejki ponownego przesyłania
    pub fn queue_upload(&self, qso_id: i64, service: &str, adif_data: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO upload_queue (qso_id, service, adif_data) VALUES (?1, ?2, ?3)",
            params![qso_id, service, adif_data],
        )?;
        Ok(())
    }

    /// Pobiera oczekujące wpisy z kolejki (max 50, retry < 5)
    pub fn get_pending_uploads(&self, service: &str) -> rusqlite::Result<Vec<(i64, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, qso_id, adif_data FROM upload_queue WHERE service = ?1 AND retry_count < 5 ORDER BY created_at LIMIT 50"
        )?;
        let rows = stmt.query_map([service], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1).map(|_| service.to_string()).unwrap_or_default(), row.get::<_, String>(2)?))
        })?;
        let mut items = Vec::new();
        for r in rows.flatten() { items.push(r); }
        Ok(items)
    }

    /// Usuwa wpis z kolejki po udanym przesłaniu
    pub fn remove_from_queue(&self, queue_id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM upload_queue WHERE id = ?1", params![queue_id])?;
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

    /// Zwraca liczbę QSO pogrupowaną według miesiąca (YYYY-MM)
    pub fn stats_qso_per_month(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT substr(qso_date, 1, 7) as month, COUNT(*) as cnt FROM qso_records GROUP BY month ORDER BY month DESC LIMIT 24"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca liczbę QSO pogrupowaną według pasma
    pub fn stats_qso_per_band(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT band, COUNT(*) as cnt FROM qso_records GROUP BY band ORDER BY cnt DESC"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca liczbę QSO pogrupowaną według emisji
    pub fn stats_qso_per_mode(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT mode, COUNT(*) as cnt FROM qso_records GROUP BY mode ORDER BY cnt DESC"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca histogram aktywności wg godziny UTC (0-23)
    pub fn stats_activity_by_hour(&self) -> rusqlite::Result<Vec<(u32, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(substr(time_on, 1, 2) AS INTEGER) as hour, COUNT(*) as cnt FROM qso_records GROUP BY hour ORDER BY hour"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca top N krajów wg liczby QSO
    pub fn stats_top_countries(&self, limit: usize) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(country, 'Unknown') as cty, COUNT(*) as cnt FROM qso_records GROUP BY cty ORDER BY cnt DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit as i64], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }

    /// Zwraca statystyki QSL (total, lotw_confirmed, eqsl_confirmed, paper_confirmed)
    pub fn stats_qsl_summary(&self) -> rusqlite::Result<(i64, i64, i64, i64)> {
        let total: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))?;
        let lotw: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records WHERE lotw_qsl_rcvd = 'Y'", [], |r| r.get(0))?;
        let eqsl: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records WHERE eqsl_qsl_rcvd = 'Y'", [], |r| r.get(0))?;
        let paper: i64 = self.conn.query_row("SELECT COUNT(*) FROM qso_records WHERE qsl_rcvd = 'Y'", [], |r| r.get(0))?;
        Ok((total, lotw, eqsl, paper))
    }

    /// Zwraca top N stref CQ wg liczby QSO
    pub fn stats_top_cq_zones(&self, limit: usize) -> rusqlite::Result<Vec<(u32, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT cqz, COUNT(*) as cnt FROM qso_records WHERE cqz IS NOT NULL GROUP BY cqz ORDER BY cnt DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit as i64], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)))?;
        let mut v = Vec::new();
        for x in rows.flatten() { v.push(x); }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_find_qso() {
        let db = LogDatabase::open_in_memory().unwrap();
        let qso = QsoRecord::new("SP6INA", "20m", "CW");
        let id = db.insert_qso(&qso).unwrap();
        assert!(id > 0);

        let prev = db.find_previous_qsos("SP6INA").unwrap();
        assert_eq!(prev.len(), 1);
        assert_eq!(prev[0].callsign, "SP6INA");
        assert_eq!(prev[0].band, "20m");
        assert_eq!(prev[0].mode, "CW");
    }

    #[test]
    fn test_multi_journal_operations() {
        let db = LogDatabase::open_in_memory().unwrap();
        let journals = db.get_all_journals().unwrap();
        assert_eq!(journals.len(), 1);
        assert_eq!(journals[0].id, "DEFAULT");

        let new_j = Journal {
            id: "PORTABLE".to_string(),
            name: "Aktywacje SPFF / PGA".to_string(),
            station_callsign: "SP6INA/P".to_string(),
            operator: "Mariusz Woźniak".to_string(),
            my_gridsquare: "JO80".to_string(),
            my_pga: "KL01".to_string(),
            description: "Praca w terenie".to_string(),
            is_default: false,
        };
        db.create_journal(&new_j).unwrap();

        let updated_journals = db.get_all_journals().unwrap();
        assert_eq!(updated_journals.len(), 2);

        // Dodanie QSO do profilu PORTABLE
        let mut qso_p = QsoRecord::new("DL1ABC", "40m", "SSB");
        qso_p.journal_id = Some("PORTABLE".to_string());
        db.insert_qso(&qso_p).unwrap();

        let p_qsos = db.get_recent_qsos_for_journal("PORTABLE", 50).unwrap();
        assert_eq!(p_qsos.len(), 1);
        assert_eq!(p_qsos[0].callsign, "DL1ABC");

        let def_qsos = db.get_recent_qsos_for_journal("DEFAULT", 50).unwrap();
        assert_eq!(def_qsos.len(), 0);
    }

    #[test]
    fn test_advanced_search_filtering() {
        let mut db = LogDatabase::open_in_memory().unwrap();
        let mut q1 = QsoRecord::new("SP6INA", "20m", "CW");
        q1.qso_date = "2026-09-01".to_string();
        q1.lotw_qsl_rcvd = "Y".to_string();

        let mut q2 = QsoRecord::new("W1AW", "40m", "SSB");
        q2.qso_date = "2026-09-15".to_string();

        let mut q3 = QsoRecord::new("JA1ABC", "20m", "FT8");
        q3.qso_date = "2026-09-20".to_string();

        db.batch_insert_qsos(&[q1, q2, q3]).unwrap();

        // Filtruj tylko 20m
        let mut filter = AdvancedQsoFilter::default();
        filter.bands = vec!["20m".to_string()];
        let res = db.search_qsos_advanced(&filter).unwrap();
        assert_eq!(res.len(), 2);

        // Filtruj tylko potwierdzone LoTW
        filter.bands.clear();
        filter.lotw_confirmed = Some(true);
        let res_lotw = db.search_qsos_advanced(&filter).unwrap();
        assert_eq!(res_lotw.len(), 1);
        assert_eq!(res_lotw[0].callsign, "SP6INA");
    }

    #[test]
    fn test_batch_insert() {
        let mut db = LogDatabase::open_in_memory().unwrap();
        let qsos = vec![
            QsoRecord::new("SP6INA", "20m", "CW"),
            QsoRecord::new("W1AW", "40m", "SSB"),
            QsoRecord::new("JA1ABC", "15m", "FT8"),
        ];
        let inserted = db.batch_insert_qsos(&qsos).unwrap();
        assert_eq!(inserted, 3);
        assert_eq!(db.count_all().unwrap(), 3);
    }

    #[test]
    fn test_update_qso() {
        let db = LogDatabase::open_in_memory().unwrap();
        let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
        qso.rst_sent = "599".to_string();
        qso.name = Some("Mariusz".to_string());
        let id = db.insert_qso(&qso).unwrap();

        let mut updated = qso.clone();
        updated.rst_sent = "579".to_string();
        updated.name = Some("Mariusz SP6INA".to_string());
        updated.qth = Some("Wrocław".to_string());
        updated.gridsquare = Some("JO81WA".to_string());
        updated.pga_ref = Some("WR01".to_string());

        db.update_qso(id, &updated).unwrap();

        let list = db.find_previous_qsos("SP6INA").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].rst_sent, "579");
        assert_eq!(list[0].name.as_deref(), Some("Mariusz SP6INA"));
        assert_eq!(list[0].qth.as_deref(), Some("Wrocław"));
        assert_eq!(list[0].gridsquare.as_deref(), Some("JO81WA"));
        assert_eq!(list[0].pga_ref.as_deref(), Some("WR01"));
    }

    #[test]
    fn test_mark_lotw_and_eqsl() {
        let db = LogDatabase::open_in_memory().unwrap();
        let mut qso = QsoRecord::new("W1AW", "20m", "CW");
        qso.qso_date = "20260920".to_string();
        db.insert_qso(&qso).unwrap();

        let updated_lotw = db.mark_lotw_confirmed("W1AW", "20m", "CW", "20260920", "20260920").unwrap();
        assert_eq!(updated_lotw, 1);

        let updated_eqsl = db.mark_eqsl_confirmed("W1AW", "20m", "CW", "20260920", "20260920").unwrap();
        assert_eq!(updated_eqsl, 1);

        let list = db.find_previous_qsos("W1AW").unwrap();
        assert_eq!(list[0].lotw_qsl_rcvd, "Y");
        assert_eq!(list[0].eqsl_qsl_rcvd, "Y");
    }

    #[test]
    fn test_search_with_apostrophe() {
        let db = LogDatabase::open_in_memory().unwrap();
        let mut qso = QsoRecord::new("EI2O", "20m", "SSB");
        qso.name = Some("O'Connor".to_string());
        db.insert_qso(&qso).unwrap();

        let mut filter = AdvancedQsoFilter::default();
        filter.callsign_query = Some("O'Connor".to_string());
        let res = db.search_qsos_advanced(&filter);
        assert!(res.is_ok(), "Wyszukiwanie z apostrofem nie powinno powodować błędu SQL");
        assert_eq!(res.unwrap().len(), 1);
    }

    #[test]
    fn test_find_and_delete_duplicates() {
        let db = LogDatabase::open_in_memory().unwrap();

        let mut q1 = QsoRecord::new("SP6INA", "20m", "CW");
        q1.qso_date = "20260920".to_string();
        q1.time_on = "120000".to_string();
        let id1 = db.insert_qso(&q1).unwrap();

        let mut q2 = QsoRecord::new("SP6INA", "20m", "CW");
        q2.qso_date = "20260920".to_string();
        q2.time_on = "120500".to_string();
        let id2 = db.insert_qso(&q2).unwrap();

        let mut q3 = QsoRecord::new("SP6INA", "40m", "CW");
        q3.qso_date = "20260920".to_string();
        q3.time_on = "121000".to_string();
        db.insert_qso(&q3).unwrap();

        let dups = db.find_duplicate_qsos(false).unwrap();
        assert_eq!(dups.len(), 1, "Powinna być dokładnie jedna grupa duplikatów (SP6INA / 20m / CW)");
        assert_eq!(dups[0].len(), 2);

        let deleted = db.delete_multiple_qsos(&[id2]).unwrap();
        assert_eq!(deleted, 1);

        let dups_after = db.find_duplicate_qsos(false).unwrap();
        assert_eq!(dups_after.len(), 0, "Brak duplikatów po usunięciu");

        let remaining = db.find_previous_qsos("SP6INA").unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|q| q.id == Some(id1)));
    }
}
