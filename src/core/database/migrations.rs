// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use log::error;
use rusqlite::{Connection, Result, params};
use std::path::{Path, PathBuf};

/// Sygnatura pojedynczej migracji schematu.
pub(super) type MigrationFn = fn(&Connection) -> rusqlite::Result<()>;

/// Jedyne źródło prawdy o migracjach schematu (STAGE-V140-S1): wersje rosną od 1 bez luk.
///
/// Każda nowa migracja jest dopisywana na końcu; wersja schematu znana programowi
/// ([`LATEST_SCHEMA_VERSION`]) jest wyprowadzana z ostatniego wpisu, a nie wpisywana ręcznie
/// w kilku miejscach.
pub(super) const MIGRATIONS: &[(i64, MigrationFn)] = &[
    (1, LogDatabase::migration_1_base_schema),
    (2, LogDatabase::migration_2_add_columns),
    (3, LogDatabase::migration_3_foreign_keys),
    (4, LogDatabase::migration_4_indexes),
    (5, LogDatabase::migration_5_identity_index),
    (6, LogDatabase::migration_6_identity_unique),
    (7, LogDatabase::migration_7_refined_qso_identity),
    (8, LogDatabase::migration_8_upload_checkpoints),
    (9, LogDatabase::migration_9_upload_sent_dates),
    (10, LogDatabase::migration_10_chrono_and_qsl_indexes),
];

/// Najwyższa wersja schematu znana tej wersji programu — wyprowadzona z [`MIGRATIONS`].
pub const LATEST_SCHEMA_VERSION: i64 = MIGRATIONS[MIGRATIONS.len() - 1].0;

/// Podkatalog (względem katalogu bazy) z kopiami wykonywanymi przed migracją schematu.
pub const PRE_MIGRATION_BACKUP_DIR: [&str; 2] = ["backups", "pre-migration"];

/// Katalog kopii przedmigracyjnych dla bazy pod wskazaną ścieżką.
#[must_use]
pub fn pre_migration_backup_dir(db_path: &Path) -> PathBuf {
    let parent = match db_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    PRE_MIGRATION_BACKUP_DIR
        .iter()
        .fold(parent.to_path_buf(), |dir, part| dir.join(part))
}

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

pub const MIGRATION_8_SQL: &str = "CREATE TABLE IF NOT EXISTS upload_plans (
                plan_id TEXT PRIMARY KEY,
                schema_version INTEGER NOT NULL,
                service TEXT NOT NULL,
                journal_id TEXT NOT NULL,
                selection_policy TEXT NOT NULL,
                selection_policy_version INTEGER NOT NULL,
                canonical_order_version INTEGER NOT NULL,
                batch_policy_version INTEGER NOT NULL,
                plan_digest TEXT NOT NULL,
                total_qsos INTEGER NOT NULL,
                total_batches INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                state TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                error_class TEXT NOT NULL DEFAULT 'none'
            );

            CREATE INDEX IF NOT EXISTS idx_upload_plans_journal_service
            ON upload_plans(journal_id, service, state);

            CREATE TABLE IF NOT EXISTS upload_batches (
                plan_id TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                batch_id TEXT NOT NULL,
                service TEXT NOT NULL,
                journal_id TEXT NOT NULL,
                record_count INTEGER NOT NULL,
                batch_digest TEXT NOT NULL,
                attempt_id TEXT NOT NULL,
                state TEXT NOT NULL,
                started_at INTEGER,
                accepted_at INTEGER,
                completed_at INTEGER,
                accepted_count INTEGER NOT NULL DEFAULT 0,
                rejected_count INTEGER NOT NULL DEFAULT 0,
                unknown_count INTEGER NOT NULL DEFAULT 0,
                evidence_digest TEXT,
                error_class TEXT NOT NULL DEFAULT 'none',
                retry_eligible INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (plan_id, ordinal),
                UNIQUE (plan_id, batch_id)
            );

            CREATE INDEX IF NOT EXISTS idx_upload_batches_state
            ON upload_batches(state);

            CREATE TABLE IF NOT EXISTS upload_batch_members (
                plan_id TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                position INTEGER NOT NULL,
                qso_id INTEGER NOT NULL,
                PRIMARY KEY (plan_id, ordinal, position)
            );

            CREATE INDEX IF NOT EXISTS idx_upload_batch_members_qso
            ON upload_batch_members(plan_id, qso_id);";

/// Migracja 10 (STAGE-V140-S1): indeksy na wyrażeniach **tekstowo identycznych** z
/// `ORDER_CHRONO_ASC`/`ORDER_CHRONO_DESC` (`schema.rs`) oraz indeks dopasowań QSL.
///
/// Planista SQLite używa indeksu na wyrażeniu tylko wtedy, gdy wyrażenie w zapytaniu jest
/// takie samo jak w indeksie — dlatego porządek chronologiczny i ścieżki zapisu pozostają bez
/// zmian, a znika jedynie sortowanie w pamięci (`USE TEMP B-TREE FOR ORDER BY`). Treść jest
/// zamrożona: zmiana porządku chronologicznego wymaga nowej migracji (pilnuje tego test
/// `migration_10_index_expressions_match_order_chrono`).
pub const MIGRATION_10_SQL: &str = "CREATE INDEX IF NOT EXISTS idx_qso_journal_chrono
                ON qso_records(journal_id, REPLACE(qso_date, '-', ''), SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6), id);
            CREATE INDEX IF NOT EXISTS idx_qso_journal_call_chrono
                ON qso_records(journal_id, callsign, REPLACE(qso_date, '-', ''), SUBSTR(REPLACE(time_on, ':', '') || '000000', 1, 6), id);
            CREATE INDEX IF NOT EXISTS idx_qso_qsl_match
                ON qso_records(journal_id, callsign, band, REPLACE(qso_date, '-', ''));
            ANALYZE;";

impl LogDatabase {
    /// Inicjalizuje schemat tabeli łączności oraz niezbędne indeksy.
    /// Stosuje wersjonowane migracje rejestrowane w tabeli `schema_version`,
    /// dzięki czemu starsze bazy są uaktualniane krok po kroku, a błędy
    /// `ALTER TABLE` nie są już maskowane.
    ///
    /// Każda migracja jest wykonywana w transakcji `BEGIN IMMEDIATE`: oficjalna
    /// dokumentacja SQLite (`lang_transaction.html`) gwarantuje, że po
    /// `BEGIN IMMEDIATE` żadna kolejna operacja nie zwróci `SQLITE_BUSY`, co
    /// usuwa okno częściowo wykonanej migracji przy równoległym pisarzu.
    pub(super) fn init_schema(&mut self) -> Result<()> {
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

        // Migracje z jednego źródła prawdy (`MIGRATIONS`) w kolejności rosnącej; każda jest
        // wykonywana w transakcji i rejestrowana w `schema_version` dopiero po pełnym powodzeniu.
        for (version, migrate) in MIGRATIONS {
            if self.current_schema_version()? < *version {
                let tx = self
                    .conn
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
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
        let version = self.current_schema_version()?;
        if version > LATEST_SCHEMA_VERSION {
            error!(
                "Baza ma nieznaną wersję schematu {version} (znana maksymalna: {LATEST_SCHEMA_VERSION})"
            );
            return Err(rusqlite::Error::InvalidQuery);
        }

        let required_tables = [
            "journals",
            "qso_records",
            "upload_queue",
            "schema_version",
            "upload_plans",
            "upload_batches",
            "upload_batch_members",
        ];
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
            ("qso_records", "lotw_qslsdate"),
            ("qso_records", "eqsl_qslsdate"),
            ("upload_queue", "id"),
            ("upload_queue", "qso_id"),
            ("upload_queue", "service"),
            ("upload_plans", "plan_id"),
            ("upload_plans", "journal_id"),
            ("upload_plans", "service"),
            ("upload_plans", "schema_version"),
            ("upload_plans", "state"),
            ("upload_batches", "plan_id"),
            ("upload_batches", "ordinal"),
            ("upload_batches", "batch_id"),
            ("upload_batches", "journal_id"),
            ("upload_batches", "service"),
            ("upload_batches", "state"),
            ("upload_batches", "batch_digest"),
            ("upload_batch_members", "plan_id"),
            ("upload_batch_members", "position"),
            ("upload_batch_members", "qso_id"),
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

    /// Wersja 8 (F-20R / F20-R1): trwałe checkpointy planu uploadu i jego paczek.
    ///
    /// Migracja jest **wyłącznie addytywna** — nie modyfikuje żadnego istniejącego
    /// wiersza i nie tworzy żadnej kolumny w `qso_records`. Trzy nowe tabele
    /// przechowują tożsamość planu, stan paczek i uporządkowane członkostwo
    /// (identyfikatory QSO), ale **żadnych sekretów**: bez URL-i, haseł, tokenów
    /// i treści odpowiedzi usług.
    fn migration_8_upload_checkpoints(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(MIGRATION_8_SQL)
    }

    /// Wersja 9 (F-20R / F20-R2): pola ADIF `LOTW_QSLSDATE` i `EQSL_QSLSDATE`.
    ///
    /// Typ `Date` wg oficjalnej specyfikacji ADIF 3.1.8 (8 cyfr, UTC,
    /// `YYYYMMDD`). Kolumny są `NULL`-owalne i **nie są backfillowane**:
    /// historyczny wiersz `Y` bez daty pozostaje `Y + NULL`, bo nie istnieje
    /// udokumentowane źródło tej daty. Data migracji nigdy nie staje się datą
    /// wysłania.
    fn migration_9_upload_sent_dates(conn: &Connection) -> rusqlite::Result<()> {
        Self::add_column_if_missing(conn, "qso_records", "lotw_qslsdate", "TEXT")?;
        Self::add_column_if_missing(conn, "qso_records", "eqsl_qslsdate", "TEXT")?;
        Ok(())
    }

    /// Wycofuje migrację 9 (test scope): usuwa wyłącznie kolumny dat wysłania.
    ///
    /// `ALTER TABLE … DROP COLUMN` jest dostępne od SQLite 3.35; projekt używa
    /// bundled SQLite 3.54.1. Rollback nigdy nie dotyka statusów `Y`, więc nie
    /// może zgubić dowodu wysłania — usuwa tylko datę.
    #[cfg(test)]
    pub(super) fn rollback_migration_9(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "ALTER TABLE qso_records DROP COLUMN lotw_qslsdate;
             ALTER TABLE qso_records DROP COLUMN eqsl_qslsdate;
             DELETE FROM schema_version WHERE version = 9;",
        )
    }

    /// Wycofuje migrację 8 (test scope): usuwa wyłącznie tabele checkpointów.
    #[cfg(test)]
    pub(super) fn rollback_migration_8(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "DROP TABLE IF EXISTS upload_batch_members;
             DROP TABLE IF EXISTS upload_batches;
             DROP TABLE IF EXISTS upload_plans;
             DELETE FROM schema_version WHERE version = 8;",
        )
    }

    /// Wersja 10 (STAGE-V140-S1): indeksy chronologiczne i indeks dopasowań QSL.
    ///
    /// Migracja jest **wyłącznie addytywna** — tworzy trzy indeksy i odświeża statystyki
    /// planisty (`ANALYZE`); nie modyfikuje żadnego wiersza.
    fn migration_10_chrono_and_qsl_indexes(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(MIGRATION_10_SQL)
    }

    /// Wycofuje migrację 10 (test scope): usuwa wyłącznie jej indeksy.
    #[cfg(test)]
    pub(super) fn rollback_migration_10(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_qso_journal_chrono;
             DROP INDEX IF EXISTS idx_qso_journal_call_chrono;
             DROP INDEX IF EXISTS idx_qso_qsl_match;
             DELETE FROM schema_version WHERE version = 10;",
        )
    }

    /// Kopia przedmigracyjna (STAGE-V140-S1, fail-closed).
    ///
    /// Wywoływana przy otwarciu bazy plikowej **przed** jakąkolwiek zmianą pliku (także przed
    /// przełączeniem `journal_mode`). Gdy baza ma zaległe migracje (`wersja < LATEST`) i zawiera
    /// co najmniej jedno QSO, tworzy zweryfikowaną kopię w `<katalog bazy>/backups/pre-migration/`.
    /// Niepowodzenie kopii oznacza brak migracji: zwracany jest błąd z przyczyną, a plik bazy
    /// pozostaje nietknięty. Baza pusta lub nowa nie wymaga kopii.
    pub(super) fn backup_before_pending_migrations(
        conn: &Connection,
        db_path: &Path,
    ) -> Result<Option<PathBuf>> {
        let from = Self::probe_schema_version(conn)?;
        if from >= LATEST_SCHEMA_VERSION || !Self::probe_has_qsos(conn)? {
            return Ok(None);
        }
        let backup_dir = pre_migration_backup_dir(db_path);
        match crate::core::backup::BackupManager::backup_database_pre_migration(
            db_path,
            &backup_dir,
            from,
            LATEST_SCHEMA_VERSION,
        ) {
            Ok(path) => {
                log::info!(
                    "Kopia przedmigracyjna schematu v{from} -> v{LATEST_SCHEMA_VERSION}: {}",
                    path.display()
                );
                Ok(Some(path))
            }
            Err(e) => {
                error!(
                    "Kopia przedmigracyjna nie powiodła się — migracja v{from} -> v{LATEST_SCHEMA_VERSION} nie została rozpoczęta: {e}"
                );
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(format!(
                        "pre-migration backup failed (schema v{from} -> v{LATEST_SCHEMA_VERSION}); \
                         the migration was not started and the database is unchanged: {e}"
                    )),
                ))
            }
        }
    }

    /// Wersja schematu odczytana bez modyfikowania pliku (0, gdy brak tabeli wersji).
    fn probe_schema_version(conn: &Connection) -> Result<i64> {
        let has_table = conn
            .prepare(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_version'",
            )?
            .exists([])?;
        if !has_table {
            return Ok(0);
        }
        let version: Option<i64> =
            conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))?;
        Ok(version.unwrap_or(0))
    }

    /// Czy baza zawiera co najmniej jedno QSO (bez modyfikowania pliku).
    fn probe_has_qsos(conn: &Connection) -> Result<bool> {
        let has_table = conn
            .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'qso_records'")?
            .exists([])?;
        if !has_table {
            return Ok(false);
        }
        conn.prepare("SELECT 1 FROM qso_records LIMIT 1")?
            .exists([])
    }

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
