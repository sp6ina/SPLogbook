// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use log::error;
use rusqlite::{Connection, Result, Row, params};
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

    /// Inicjalizuje schemat tabeli łączności oraz niezbędne indeksy.
    /// Stosuje wersjonowane migracje rejestrowane w tabeli `schema_version`,
    /// dzięki czemu starsze bazy są uaktualniane krok po kroku, a błędy
    /// `ALTER TABLE` nie są już maskowane.
    fn init_schema(&mut self) -> Result<()> {
        type MigrationFn = fn(&Connection) -> rusqlite::Result<()>;
        self.conn.execute_batch(
            "PRAGMA foreign_keys=ON;
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA cache_size=10000;
            PRAGMA temp_store=MEMORY;

            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

        // Lista migracji w kolejności rosnącej; każda jest wykonywana w transakcji
        // i rejestrowana w `schema_version` dopiero po pełnym powodzeniu.
        let migrations: &[(i64, MigrationFn)] = &[
            (1, Self::migration_1_base_schema),
            (2, Self::migration_2_add_columns),
            (3, Self::migration_3_foreign_keys),
        ];

        for (version, migrate) in migrations {
            if self.current_schema_version()? < *version {
                let tx = self.conn.transaction()?;
                migrate(&tx)?;
                tx.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    params![version],
                )?;
                tx.commit()?;
            }
        }

        self.validate_schema()
    }

    /// Weryfikuje integralność schematu po migracjach. Wykrywa zawyżoną wersję
    /// `schema_version`, brak kluczowych tabel lub kolumn, zamiast pozwolić
    /// aplikacji działać na uszkodzonej bazie.
    fn validate_schema(&self) -> Result<()> {
        const KNOWN_MAX_VERSION: i64 = 3;

        let version = self.current_schema_version()?;
        if version > KNOWN_MAX_VERSION {
            error!(
                "Baza ma nieznaną wersję schematu {version} (znana maksymalna: {KNOWN_MAX_VERSION})"
            );
            return Err(rusqlite::Error::InvalidQuery);
        }

        let required_tables = ["journals", "qso_records", "upload_queue", "schema_version"];
        for table in required_tables {
            let exists: bool = self
                .conn
                .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1")?
                .exists(params![table])?;
            if !exists {
                error!("Brak wymaganej tabeli `{table}` w bazie danych");
                return Err(rusqlite::Error::InvalidQuery);
            }
        }

        let required_columns: &[(&str, &str)] = &[
            ("journals", "id"),
            ("journals", "name"),
            ("qso_records", "id"),
            ("qso_records", "callsign"),
            ("qso_records", "qso_date"),
            ("qso_records", "time_on"),
            ("qso_records", "journal_id"),
            ("upload_queue", "id"),
            ("upload_queue", "qso_id"),
            ("upload_queue", "service"),
        ];
        for (table, column) in required_columns {
            let exists: bool = self
                .conn
                .prepare("SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2")?
                .exists(params![table, column])?;
            if !exists {
                error!("Brak wymaganej kolumny `{table}.{column}` w bazie danych");
                return Err(rusqlite::Error::InvalidQuery);
            }
        }

        Ok(())
    }

    /// Najwyższa zarejestrowana wersja schematu (0, gdy brak wpisów).
    fn current_schema_version(&self) -> Result<i64> {
        let v: Option<i64> =
            self.conn
                .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))?;
        Ok(v.unwrap_or(0))
    }

    /// Wersja 1: bazowe tabele (dzienniki, QSO, kolejka uploadów) i indeksy.
    fn migration_1_base_schema(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS journals (
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

            CREATE INDEX IF NOT EXISTS idx_qso_callsign ON qso_records(callsign);
            DROP INDEX IF EXISTS idx_qso_call;
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
        )
    }

    /// Wersja 2: kolumny dodane w kolejnych wydaniach (z kontrolą istnienia).
    fn migration_2_add_columns(conn: &Connection) -> rusqlite::Result<()> {
        Self::add_column_if_missing(conn, "qso_records", "journal_id", "TEXT DEFAULT 'DEFAULT'")?;
        Self::add_column_if_missing(conn, "qso_records", "my_pota_ref", "TEXT")?;
        Self::add_column_if_missing(conn, "qso_records", "my_sota_ref", "TEXT")?;
        Self::add_column_if_missing(conn, "qso_records", "vucc_grids", "TEXT")?;
        Self::add_column_if_missing(conn, "qso_records", "audio_file", "TEXT")?;
        Ok(())
    }

    /// Wersja 3: dodaje klucze obce (`journal_id` → `journals.id`,
    /// `upload_queue.qso_id` → `qso_records.id`) przez odbudowę tabel, wraz
    /// z usunięciem ewentualnych osieroconych wierszy. SQLite nie wspiera
    /// `ALTER TABLE ... ADD CONSTRAINT`, dlatego tabele są tworzone od nowa.
    fn migration_3_foreign_keys(conn: &Connection) -> rusqlite::Result<()> {
        // Usuń osierocone wiersze, aby odbudowa z kluczami obcymi nie zawiodła.
        conn.execute(
            "DELETE FROM upload_queue WHERE qso_id NOT IN (SELECT id FROM qso_records)",
            [],
        )?;
        conn.execute(
            "UPDATE qso_records SET journal_id = 'DEFAULT' WHERE journal_id NOT IN (SELECT id FROM journals)",
            [],
        )?;

        // Zmień nazwy starych tabel (bez kluczy obcych), aby utworzyć nowe.
        conn.execute_batch(
            "ALTER TABLE qso_records RENAME TO qso_records_old;
             ALTER TABLE upload_queue RENAME TO upload_queue_old;",
        )?;

        conn.execute_batch(
            "CREATE TABLE qso_records (
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
                journal_id TEXT DEFAULT 'DEFAULT' REFERENCES journals(id) ON DELETE SET DEFAULT
            );

            INSERT INTO qso_records (
                id, callsign, band, mode, submode, qso_date, time_on, time_off,
                freq, freq_rx, rst_sent, rst_rcvd, name, qth, gridsquare,
                state, iota, sota_ref, pota_ref, pga_ref, dxcc, country,
                continent, cqz, ituz, comment, qsl_via, qsl_manager,
                qsl_sent, qsl_rcvd, qsl_sent_date, qsl_rcvd_date,
                lotw_qsl_sent, lotw_qsl_rcvd, lotw_qslrdate,
                eqsl_qsl_sent, eqsl_qsl_rcvd, eqsl_qslrdate,
                clublog_upload_status, qrzcom_upload_status,
                sat_name, sat_mode, prop_mode, srx, stx, srx_string, stx_string,
                my_gridsquare, my_state, my_pota_ref, my_sota_ref, vucc_grids, audio_file, journal_id
            )
            SELECT
                id, callsign, band, mode, submode, qso_date, time_on, time_off,
                freq, freq_rx, rst_sent, rst_rcvd, name, qth, gridsquare,
                state, iota, sota_ref, pota_ref, pga_ref, dxcc, country,
                continent, cqz, ituz, comment, qsl_via, qsl_manager,
                qsl_sent, qsl_rcvd, qsl_sent_date, qsl_rcvd_date,
                lotw_qsl_sent, lotw_qsl_rcvd, lotw_qslrdate,
                eqsl_qsl_sent, eqsl_qsl_rcvd, eqsl_qslrdate,
                clublog_upload_status, qrzcom_upload_status,
                sat_name, sat_mode, prop_mode, srx, stx, srx_string, stx_string,
                my_gridsquare, my_state, my_pota_ref, my_sota_ref, vucc_grids, audio_file, journal_id
            FROM qso_records_old;

            CREATE TABLE upload_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                qso_id INTEGER NOT NULL REFERENCES qso_records(id) ON DELETE CASCADE,
                service TEXT NOT NULL,
                adif_data TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                retry_count INTEGER NOT NULL DEFAULT 0,
                last_error TEXT
            );
            INSERT INTO upload_queue (id, qso_id, service, adif_data, created_at, retry_count, last_error)
            SELECT id, qso_id, service, adif_data, created_at, retry_count, last_error FROM upload_queue_old;

            DROP TABLE upload_queue_old;
            DROP TABLE qso_records_old;",
        )?;

        // Odtwórz indeksy po odbudowie tabel.
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_upload_queue_service ON upload_queue(service, retry_count);
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
            CREATE INDEX IF NOT EXISTS idx_qso_composite ON qso_records(callsign, band, mode);",
        )?;

        Ok(())
    }

    /// Dodaje kolumnę tylko wtedy, gdy jeszcze nie istnieje (bez maskowania błędów).
    fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        decl: &str,
    ) -> rusqlite::Result<()> {
        let exists: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2")?
            .exists(params![table, column])?;
        if !exists {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
                [],
            )?;
        }
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
                "SELECT {QSO_COLUMNS} FROM qso_records WHERE callsign = ?1 AND band = ?2 AND mode = ?3 AND qso_date = ?4 ORDER BY time_on ASC, id ASC"
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
                "SELECT {QSO_COLUMNS} FROM qso_records WHERE callsign = ?1 AND band = ?2 AND mode = ?3 ORDER BY qso_date ASC, time_on ASC, id ASC"
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
    pub fn mark_lotw_confirmed(
        &self,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE qso_records
             SET lotw_qsl_rcvd = 'Y', lotw_qslrdate = ?1
             WHERE callsign = ?2 AND band = ?3 AND mode = ?4 AND qso_date = ?5",
            params![
                rdate,
                callsign.to_uppercase(),
                band,
                mode.to_uppercase(),
                qso_date
            ],
        )?;
        Ok(count)
    }

    /// Oznacza potwierdzenie eQSL dla dopasowanej łączności
    pub fn mark_eqsl_confirmed(
        &self,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE qso_records
             SET eqsl_qsl_rcvd = 'Y', eqsl_qslrdate = ?1
             WHERE callsign = ?2 AND band = ?3 AND mode = ?4 AND qso_date = ?5",
            params![
                rdate,
                callsign.to_uppercase(),
                band,
                mode.to_uppercase(),
                qso_date
            ],
        )?;
        Ok(count)
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
    pub fn qso_numbers_for_journal(
        &self,
        journal_id: &str,
    ) -> Result<std::collections::HashMap<i64, usize>> {
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
    pub fn existing_qso_keys(&self, journal_id: &str) -> Result<std::collections::HashSet<String>> {
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

    /// Wyszukiwanie łączności z wieloma kryteriami (zaawansowane filtrowanie).
    /// Wszystkie wartości pochodzące od użytkownika są przekazywane jako parametry
    /// wiązane (nie string-concat), a wzorce LIKE mają escapowane znaki wieloznaczne
    /// `%`/`_`, żeby wyszukiwane teksty użytkownika nie działały jak wildcardy.
    pub fn search_qsos_advanced(&self, filter: &AdvancedQsoFilter) -> Result<Vec<QsoRecord>> {
        let mut sql = format!("SELECT {QSO_COLUMNS} FROM qso_records WHERE 1=1");
        let mut conditions: Vec<String> = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref jid) = filter.journal_id {
            conditions.push(format!("journal_id = ?{}", values.len() + 1));
            values.push(Box::new(jid.clone()));
        }
        if let Some(ref from) = filter.date_from {
            if !from.is_empty() {
                conditions.push(format!("qso_date >= ?{}", values.len() + 1));
                values.push(Box::new(from.clone()));
            }
        }
        if let Some(ref to) = filter.date_to {
            if !to.is_empty() {
                conditions.push(format!("qso_date <= ?{}", values.len() + 1));
                values.push(Box::new(to.clone()));
            }
        }
        if !filter.bands.is_empty() {
            let placeholders: Vec<String> = filter
                .bands
                .iter()
                .map(|b| {
                    values.push(Box::new(b.clone()));
                    format!("?{}", values.len())
                })
                .collect();
            conditions.push(format!("band IN ({})", placeholders.join(",")));
        }
        if !filter.modes.is_empty() {
            let placeholders: Vec<String> = filter
                .modes
                .iter()
                .map(|m| {
                    values.push(Box::new(m.clone()));
                    format!("?{}", values.len())
                })
                .collect();
            conditions.push(format!("mode IN ({})", placeholders.join(",")));
        }
        if let Some(lotw) = filter.lotw_confirmed {
            conditions.push(if lotw {
                "lotw_qsl_rcvd = 'Y'".to_string()
            } else {
                "(lotw_qsl_rcvd IS NULL OR lotw_qsl_rcvd <> 'Y')".to_string()
            });
        }
        if let Some(eqsl) = filter.eqsl_confirmed {
            conditions.push(if eqsl {
                "eqsl_qsl_rcvd = 'Y'".to_string()
            } else {
                "(eqsl_qsl_rcvd IS NULL OR eqsl_qsl_rcvd <> 'Y')".to_string()
            });
        }
        if let Some(qsl) = filter.qsl_rcvd {
            conditions.push(if qsl {
                "qsl_rcvd = 'Y'".to_string()
            } else {
                "(qsl_rcvd IS NULL OR qsl_rcvd <> 'Y')".to_string()
            });
        }
        if let Some(ref query) = filter.callsign_query {
            let q = query.trim().to_uppercase();
            if !q.is_empty() {
                // Escapuje % i _ (znaki specjalne LIKE) znakiem ucieczki '\', żeby wpisany
                // przez użytkownika tekst nie działał jak wzorzec wildcard.
                let escaped = q
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_");
                let like_pattern = format!("%{escaped}%");
                values.push(Box::new(like_pattern.clone()));
                let p1 = values.len();
                values.push(Box::new(like_pattern.clone()));
                let p2 = values.len();
                values.push(Box::new(like_pattern));
                let p3 = values.len();
                conditions.push(format!(
                    "(callsign LIKE ?{p1} ESCAPE '\\' OR name LIKE ?{p2} ESCAPE '\\' OR comment LIKE ?{p3} ESCAPE '\\')"
                ));
            }
        }

        for c in conditions {
            sql.push_str(" AND ");
            sql.push_str(&c);
        }

        sql.push_str(" ORDER BY qso_date DESC, time_on DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            values.iter().map(std::convert::AsRef::as_ref).collect();
        let rows = stmt.query_map(param_refs.as_slice(), row_to_qso)?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
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

    pub fn vacuum_if_needed(&self) -> rusqlite::Result<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM qso_records", [], |r| r.get(0))?;
        if count > 0 && count % 1000 == 0 {
            self.conn.execute_batch("VACUUM;")?;
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
                    .map(|_| service.to_string())
                    .unwrap_or_default(),
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut items = Vec::new();
        for r in rows.flatten() {
            items.push(r);
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

#[path = "database_stats.rs"]
mod database_stats;

#[cfg(test)]
#[path = "database_tests.rs"]
mod tests;
