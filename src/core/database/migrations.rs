// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use log::error;
use rusqlite::{Connection, Result, params};

pub const MIGRATION_1_SQL: &str = "CREATE TABLE IF NOT EXISTS journals (
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
            CREATE UNIQUE INDEX IF NOT EXISTS idx_upload_queue_qso_service ON upload_queue(qso_id, service);

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
            CREATE INDEX IF NOT EXISTS idx_qso_composite ON qso_records(callsign, band, mode);";

pub const MIGRATION_3_SQL: &str = "CREATE TABLE qso_records (
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
            DROP TABLE qso_records_old;";

impl LogDatabase {
    /// Inicjalizuje schemat tabeli łączności oraz niezbędne indeksy.
    /// Stosuje wersjonowane migracje rejestrowane w tabeli `schema_version`,
    /// dzięki czemu starsze bazy są uaktualniane krok po kroku, a błędy
    /// `ALTER TABLE` nie są już maskowane.
    pub(super) fn init_schema(&mut self) -> Result<()> {
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
            (4, Self::migration_4_indexes),
            (5, Self::migration_5_identity_index),
            (6, Self::migration_6_identity_unique),
            (7, Self::migration_7_refined_qso_identity),
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
    pub(super) fn validate_schema(&self) -> Result<()> {
        const KNOWN_MAX_VERSION: i64 = 7;

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
            ("qso_records", "identity_key"),
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
    pub fn current_schema_version(&self) -> Result<i64> {
        let version: Option<i64> =
            self.conn
                .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))?;
        Ok(version.unwrap_or(0))
    }

    /// Wersja 1: bazowe tabele (dzienniki, QSO, kolejka uploadów) i indeksy.
    pub fn migration_1_base_schema(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(MIGRATION_1_SQL)
    }

    /// Wersja 2: kolumny dodane w kolejnych wydaniach (z kontrolą istnienia).
    pub(super) fn migration_2_add_columns(conn: &Connection) -> rusqlite::Result<()> {
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
            "DELETE FROM upload_queue WHERE id NOT IN (SELECT MIN(id) FROM upload_queue GROUP BY qso_id, service)",
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

        conn.execute_batch(MIGRATION_3_SQL)?;

        // Odtwórz indeksy po odbudowie tabel.
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_upload_queue_service ON upload_queue(service, retry_count);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_upload_queue_qso_service ON upload_queue(qso_id, service);
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

    /// Wersja 4: indeksy pomocnicze dla filtrów po kraju i częstotliwości.
    fn migration_4_indexes(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_qso_country ON qso_records(country);
             CREATE INDEX IF NOT EXISTS idx_qso_freq ON qso_records(freq);",
        )
    }

    /// Wersja 5: normalizacja istniejących wierszy i złożony indeks tożsamości dla szybkiej deduplikacji.
    fn migration_5_identity_index(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "UPDATE qso_records
             SET callsign = UPPER(TRIM(callsign)),
                 qso_date = REPLACE(qso_date, '-', '')
             WHERE callsign IS NOT NULL;

             CREATE INDEX IF NOT EXISTS idx_qso_identity
             ON qso_records(journal_id, callsign, qso_date, band, mode);",
        )
    }

    /// Wersja 6: bazodanowe wymuszanie unikalności QSO (DATA-003).
    /// Tworzy dedykowaną kolumnę identity_key oraz unikalny indeks idx_qso_identity_unique.
    /// Przed utworzeniem ograniczenia UNIQUE migracja wykonuje audyt istniejących rekordów.
    /// W razie wykrycia nierozstrzygniętych duplikatów migracja NIE usuwa ich ani nie scala,
    /// lecz przerywa działanie zwracając błąd, co powoduje wycofanie (ROLLBACK) całej transakcji.
    fn migration_6_identity_unique(conn: &Connection) -> rusqlite::Result<()> {
        Self::add_column_if_missing(conn, "qso_records", "identity_key", "TEXT")?;

        // 1. Oblicz kanoniczny identity_key dla wszystkich istniejących rekordów w bazie
        let mut update_stmt =
            conn.prepare_cached("UPDATE qso_records SET identity_key = ?1 WHERE id = ?2")?;
        let mut select_stmt = conn.prepare(
            "SELECT id, callsign, band, mode, qso_date, time_on FROM qso_records WHERE identity_key IS NULL",
        )?;
        let rows = select_stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let callsign: String = row.get(1)?;
            let band: String = row.get(2)?;
            let mode: String = row.get(3)?;
            let qso_date: String = row.get(4)?;
            let time_on: String = row.get(5)?;
            Ok((id, callsign, band, mode, qso_date, time_on))
        })?;

        for r in rows {
            let (id, callsign, band, mode, qso_date, time_on) = r?;
            let identity = crate::core::qso::QsoIdentity::from_fields(
                &callsign, &band, &mode, &qso_date, &time_on,
            );
            let key = identity.canonical_key();
            update_stmt.execute(rusqlite::params![key, id])?;
        }

        // 2. Audyt istniejących duplikatów w bazie danych
        let mut check_stmt = conn.prepare(
            "SELECT journal_id, identity_key, COUNT(*) as cnt \
             FROM qso_records \
             WHERE identity_key IS NOT NULL \
             GROUP BY journal_id, identity_key \
             HAVING cnt > 1",
        )?;
        let duplicate_exists = check_stmt.exists([])?;
        if duplicate_exists {
            error!(
                "Wykryto istniejące duplikaty tożsamości QSO w bazie danych. \
                 Migracja unikalności przerwana — baza wymaga ręcznego rozstrzygnięcia konfliktów."
            );
            return Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "Nierozstrzygnięte duplikaty tożsamości QSO uniemożliwiają utworzenie indeksu UNIQUE",
                )),
            ));
        }

        // 3. Utwórz unikalny indeks bazodanowy i usuń stary indeks pomocniczy
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_qso_identity;
             CREATE UNIQUE INDEX IF NOT EXISTS idx_qso_identity_unique
             ON qso_records(journal_id, identity_key);",
        )?;

        Ok(())
    }

    /// Wersja 7: precyzyjna tożsamość QSO v2 (DATA-003).
    /// Aktualizuje wszystkie klucze tożsamości do formatu v2:
    /// `v2|{CALL}|{BAND}|{FREQ_HZ}|{MODE}|{SUBMODE}|{DATE}|{TIME_TAG}`
    /// z uwzględnieniem sekund (Second vs Minute), częstotliwości w Hz oraz normalizacji SUBMODE.
    /// Rekordy niepełne otrzymują identity_key = NULL (MODEL B).
    /// Przed odtworzeniem indeksu UNIQUE migracja wykonuje audyt wykrywający rzeczywiste duplikaty.
    /// W razie wykrycia konfliktu migracja NIE usuwa ani nie scala rekordów, lecz zwraca błąd,
    /// co powoduje wycofanie transakcji (ROLLBACK) i zachowanie schema_version bez zmian.
    fn migration_7_refined_qso_identity(conn: &Connection) -> rusqlite::Result<()> {
        Self::add_column_if_missing(conn, "qso_records", "identity_key", "TEXT")?;

        // 1. Przelicz kanoniczny identity_key (v2) dla wszystkich rekordów w bazie danych
        let mut update_stmt =
            conn.prepare_cached("UPDATE qso_records SET identity_key = ?1 WHERE id = ?2")?;
        let mut select_stmt = conn.prepare(
            "SELECT id, callsign, band, mode, submode, qso_date, time_on, freq FROM qso_records",
        )?;
        let rows = select_stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let callsign: String = row.get(1)?;
            let band: String = row.get(2)?;
            let mode: String = row.get(3)?;
            let submode: Option<String> = row.get(4)?;
            let qso_date: String = row.get(5)?;
            let time_on: String = row.get(6)?;
            let freq: Option<f64> = row.get(7)?;
            Ok((id, callsign, band, mode, submode, qso_date, time_on, freq))
        })?;

        for r in rows {
            let (id, callsign, band, mode, submode, qso_date, time_on, freq) = r?;
            let identity = crate::core::qso::QsoIdentity::from_detailed_fields(
                &callsign,
                &band,
                freq,
                &mode,
                submode.as_deref(),
                &qso_date,
                &time_on,
            );
            let key = identity.canonical_key();
            update_stmt.execute(rusqlite::params![key, id])?;
        }

        // 2. Audyt istniejących duplikatów w formacie v2
        let mut check_stmt = conn.prepare(
            "SELECT journal_id, identity_key, COUNT(*) as cnt              FROM qso_records              WHERE identity_key IS NOT NULL              GROUP BY journal_id, identity_key              HAVING cnt > 1",
        )?;
        let duplicate_exists = check_stmt.exists([])?;
        if duplicate_exists {
            error!(
                "Wykryto nierozstrzygnięte duplikaty tożsamości QSO w bazie danych.                  Migracja v7 przerwana — baza wymaga ręcznego rozstrzygnięcia konfliktów."
            );
            return Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "Nierozstrzygnięte duplikaty tożsamości QSO uniemożliwiają utworzenie indeksu UNIQUE (v2)",
                )),
            ));
        }

        // 3. Utwórz unikalny indeks bazodanowy
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_qso_identity;
             DROP INDEX IF EXISTS idx_qso_identity_unique;
             CREATE UNIQUE INDEX IF NOT EXISTS idx_qso_identity_unique
             ON qso_records(journal_id, identity_key);",
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
}
