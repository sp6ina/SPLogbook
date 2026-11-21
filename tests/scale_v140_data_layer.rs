// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! STAGE-V140-S1 — opt-in pomiar skali warstwy danych na 200 000 QSO.
//!
//! Standardowy baseline NIE wykonuje pomiaru: bez `SPLOG_RUN_SCALE_E2E=1` test wypisuje jawny
//! komunikat `SCALE BENCHMARK NOT REQUESTED` (wzorzec natywnych E2E instalatorów). Z opt-in
//! pomiar jest FAIL-CLOSED: przekroczenie budżetu kończy test błędem z metryką.
//!
//! Dane są syntetyczne i deterministyczne (stały seed); baza powstaje w katalogu
//! tymczasowym (`tempfile`), nigdy w profilu użytkownika.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use splogbook::cloud::lotw::LotwConfirmation;
use splogbook::core::database::qsl::{QSL_MATCH_CHUNK_SIZE, apply_lotw_confirmations_chunked};
use splogbook::core::database::{LATEST_SCHEMA_VERSION, LogDatabase};
use splogbook::core::qso::QsoRecord;

const OPT_IN_ENV: &str = "SPLOG_RUN_SCALE_E2E";
const QSO_COUNT: usize = 200_000;
const DISTINCT_CALLS: usize = 55_000;
const JOURNAL: &str = "DEFAULT";

/// Budżety akceptacji STAGE-V140-S1 (host referencyjny opisany w raporcie bramki).
const BUDGET_RECENT_1000: Duration = Duration::from_millis(10);
const BUDGET_PAGE_FIRST: Duration = Duration::from_millis(2);
const BUDGET_PAGE_DEEP: Duration = Duration::from_millis(20);
const BUDGET_PREVIOUS: Duration = Duration::from_millis(30);
const BUDGET_NUMBERS: Duration = Duration::from_millis(150);
const BUDGET_QSL_20K: Duration = Duration::from_secs(5);
const BUDGET_QSL_LOCK_HOLD: Duration = Duration::from_millis(250);
const BUDGET_MIGRATION_WITH_BACKUP: Duration = Duration::from_secs(15);

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe as i64 + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

const PREFIXES: &[&str] = &[
    "SP", "SP", "SP", "SQ", "DL", "DL", "G", "F", "I", "EA", "OK", "UA", "UA", "UR", "HA", "K",
    "W", "N", "VE", "PY", "JA", "VK", "ZS",
];
const BANDS: &[(&str, f64)] = &[
    ("160m", 1.83),
    ("80m", 3.55),
    ("40m", 7.03),
    ("30m", 10.12),
    ("20m", 14.03),
    ("17m", 18.08),
    ("15m", 21.03),
    ("12m", 24.90),
    ("10m", 28.03),
    ("6m", 50.10),
    ("2m", 144.30),
];
const MODES: &[&str] = &["FT8", "FT8", "FT8", "CW", "CW", "SSB", "SSB", "FT4", "RTTY"];

fn callsign_pool(rng: &mut Rng) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut calls = Vec::with_capacity(DISTINCT_CALLS);
    while calls.len() < DISTINCT_CALLS {
        let mut call = PREFIXES[rng.below(PREFIXES.len() as u64) as usize].to_string();
        call.push(char::from(b'0' + rng.below(10) as u8));
        for _ in 0..=rng.below(3) {
            call.push(char::from(b'A' + rng.below(26) as u8));
        }
        if seen.insert(call.clone()) {
            calls.push(call);
        }
    }
    calls
}

fn synthetic_qso(rng: &mut Rng, pool: &[String]) -> QsoRecord {
    let u = rng.unit();
    let call = &pool[(((pool.len() as f64) * u * u * u) as usize).min(pool.len() - 1)];
    let (band, base) = BANDS[rng.below(BANDS.len() as u64) as usize];
    let mode = MODES[rng.below(MODES.len() as u64) as usize];
    let (year, month, day) = civil_from_days(14_975 + rng.below(5_800) as i64);
    let (hh, mm, ss) = (rng.below(24), rng.below(60), rng.below(60));
    let mut qso = QsoRecord::new(call.clone(), band, mode);
    qso.qso_date = format!("{year:04}{month:02}{day:02}");
    qso.time_on = if rng.unit() < 0.7 {
        format!("{hh:02}{mm:02}{ss:02}")
    } else {
        format!("{hh:02}{mm:02}")
    };
    qso.freq = Some(((base + rng.unit() * 0.03) * 1_000_000.0).round() / 1_000_000.0);
    qso.journal_id = Some(JOURNAL.to_string());
    qso
}

fn best_of<T>(runs: usize, mut f: impl FnMut() -> T) -> (T, Duration) {
    let mut best = Duration::MAX;
    let mut last = None;
    for _ in 0..runs {
        let started = Instant::now();
        let value = f();
        best = best.min(started.elapsed());
        last = Some(value);
    }
    (last.expect("at least one run"), best)
}

fn check(failures: &mut Vec<String>, name: &str, measured: Duration, budget: Duration) {
    let verdict = if measured <= budget {
        "OK"
    } else {
        "OVER BUDGET"
    };
    eprintln!(
        "SCALE {name:<52} {:>10.2} ms  (budżet {:>8.2} ms)  {verdict}",
        measured.as_secs_f64() * 1000.0,
        budget.as_secs_f64() * 1000.0
    );
    if measured > budget {
        failures.push(format!(
            "{name}: {:.2} ms > {:.2} ms",
            measured.as_secs_f64() * 1000.0,
            budget.as_secs_f64() * 1000.0
        ));
    }
}

#[test]
fn scale_v140_data_layer_budgets_e2e() {
    if std::env::var(OPT_IN_ENV).ok().as_deref() != Some("1") {
        eprintln!(
            "SCALE BENCHMARK NOT REQUESTED: ustaw {OPT_IN_ENV}=1, aby wykonać pomiar skali \
             {QSO_COUNT} QSO warstwy danych (STAGE-V140-S1)"
        );
        return;
    }

    let dir = tempfile::tempdir().expect("SCALE FAIL-CLOSED: tempdir");
    let db_path = dir.path().join("scale.db");
    let mut failures = Vec::new();

    // 1. Syntetyczny dziennik 200 000 QSO.
    let mut rng = Rng(0x5350_4C4F_4742_4F4B);
    let pool = callsign_pool(&mut rng);
    let mut identities = std::collections::HashSet::new();
    let data: Vec<QsoRecord> = (0..QSO_COUNT)
        .map(|_| synthetic_qso(&mut rng, &pool))
        .filter(|qso| identities.insert(qso.identity().canonical_key()))
        .collect();
    {
        let db = LogDatabase::open(&db_path).expect("SCALE FAIL-CLOSED: open");
        let inserted = db
            .batch_insert_qsos(&data)
            .expect("SCALE FAIL-CLOSED: insert");
        eprintln!("SCALE inserted {inserted} QSO");
        assert_eq!(inserted, data.len());
        assert!(
            inserted > QSO_COUNT * 99 / 100,
            "SCALE FAIL-CLOSED: too few rows"
        );
    }

    // 2. Migracja 9 -> 10 razem z kopią przedmigracyjną.
    {
        let conn = rusqlite::Connection::open(&db_path).expect("SCALE FAIL-CLOSED: raw open");
        conn.execute_batch(
            "DROP INDEX idx_qso_journal_chrono;
             DROP INDEX idx_qso_journal_call_chrono;
             DROP INDEX idx_qso_qsl_match;
             DROP TABLE qso_extra;",
        )
        .expect("SCALE FAIL-CLOSED: rewind to schema 9");
        for column in [
            "tx_pwr",
            "operator",
            "station_callsign",
            "contest_id",
            "my_rig",
            "my_antenna",
            "cnty",
            "sig",
            "sig_info",
            "my_sig",
            "my_sig_info",
            "wwff_ref",
            "my_wwff_ref",
            "email",
        ] {
            conn.execute_batch(&format!("ALTER TABLE qso_records DROP COLUMN {column};"))
                .expect("SCALE FAIL-CLOSED: rewind to schema 9");
        }
        conn.execute_batch(
            "DELETE FROM schema_version WHERE version >= 10;
             PRAGMA wal_checkpoint(TRUNCATE);",
        )
        .expect("SCALE FAIL-CLOSED: rewind to schema 9");
    }
    let started = Instant::now();
    let db = LogDatabase::open(&db_path).expect("SCALE FAIL-CLOSED: migrate 9 -> 10");
    check(
        &mut failures,
        "migration 9->latest incl. pre-migration backup",
        started.elapsed(),
        BUDGET_MIGRATION_WITH_BACKUP,
    );
    assert_eq!(db.current_schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    let backups: Vec<_> = std::fs::read_dir(dir.path().join("backups").join("pre-migration"))
        .expect("SCALE FAIL-CLOSED: pre-migration backup dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(
        backups.len(),
        1,
        "SCALE FAIL-CLOSED: exactly one pre-migration backup"
    );

    // 3. Zapytania odczytu.
    let (_, t) = best_of(5, || {
        db.get_recent_qsos_for_journal(JOURNAL, 1000).unwrap().len()
    });
    check(
        &mut failures,
        "get_recent_qsos_for_journal(1000)",
        t,
        BUDGET_RECENT_1000,
    );
    let (_, t) = best_of(5, || {
        db.get_qsos_paginated_for_journal(JOURNAL, 50, 0)
            .unwrap()
            .len()
    });
    check(
        &mut failures,
        "get_qsos_paginated_for_journal(50, 0)",
        t,
        BUDGET_PAGE_FIRST,
    );
    let (_, t) = best_of(5, || {
        db.get_qsos_paginated_for_journal(JOURNAL, 50, 150_000)
            .unwrap()
            .len()
    });
    check(
        &mut failures,
        "get_qsos_paginated_for_journal(50, 150000)",
        t,
        BUDGET_PAGE_DEEP,
    );

    let mut counts = std::collections::HashMap::new();
    for qso in &data {
        *counts.entry(qso.callsign.as_str()).or_insert(0usize) += 1;
    }
    let (top_call, top_count) = counts
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(call, count)| ((*call).to_string(), *count))
        .expect("non-empty");
    assert!(
        top_count >= 5_000,
        "SCALE FAIL-CLOSED: top call must have 5000+ QSOs ({top_count})"
    );
    let (rows, t) = best_of(5, || {
        db.find_previous_qsos_for_journal(JOURNAL, &top_call)
            .unwrap()
            .len()
    });
    assert_eq!(rows, top_count);
    check(
        &mut failures,
        &format!("find_previous_qsos_for_journal ({top_count} QSO)"),
        t,
        BUDGET_PREVIOUS,
    );
    let (numbers, t) = best_of(3, || db.qso_numbers_for_journal(JOURNAL).unwrap().len());
    assert!(numbers > QSO_COUNT * 99 / 100);
    check(&mut failures, "qso_numbers_for_journal", t, BUDGET_NUMBERS);

    // 4. SCP: wszystkie unikalne znaki dziennika (bez limitu i alfabetycznego ucięcia).
    let started = Instant::now();
    let calls = db.get_all_distinct_callsigns_for_journal(JOURNAL).unwrap();
    let scp_elapsed = started.elapsed();
    let sp_in_log = counts.keys().filter(|call| call.starts_with("SP")).count();
    let sp_in_scp = calls.iter().filter(|call| call.starts_with("SP")).count();
    eprintln!(
        "SCALE SCP distinct calls {} / {} (SP {} / {}) in {:.2} ms",
        calls.len(),
        counts.len(),
        sp_in_scp,
        sp_in_log,
        scp_elapsed.as_secs_f64() * 1000.0
    );
    assert_eq!(
        calls.len(),
        counts.len(),
        "SCALE FAIL-CLOSED: SCP must know every call"
    );
    assert_eq!(
        sp_in_scp, sp_in_log,
        "SCALE FAIL-CLOSED: SCP must know every SP call"
    );

    // 5. Dopasowanie 20 000 potwierdzeń LoTW w paczkach, z oddawaniem blokady.
    let all = db.get_all_qsos_for_journal(JOURNAL).unwrap();
    drop(db);
    let confirmations: Vec<LotwConfirmation> = (0..20_000)
        .map(|i| {
            let qso = &all[(i * 397) % all.len()];
            LotwConfirmation {
                callsign: qso.callsign.clone(),
                band: qso.band.clone(),
                mode: qso.mode.clone(),
                qso_date: qso.qso_date.clone(),
                qso_time: Some(qso.time_on.clone()),
                qsl_rdate: Some("20261120".to_string()),
            }
        })
        .collect();
    let shared = Arc::new(Mutex::new(LogDatabase::open(&db_path).unwrap()));
    let started = Instant::now();
    let mut last_release = started;
    let mut max_hold = Duration::ZERO;
    let mut chunks = 0usize;
    let report = apply_lotw_confirmations_chunked(&shared, JOURNAL, &confirmations, |_| {
        let now = Instant::now();
        max_hold = max_hold.max(now - last_release);
        last_release = now;
        chunks += 1;
        assert!(
            shared.try_lock().is_ok(),
            "SCALE FAIL-CLOSED: lock must be free between chunks"
        );
    })
    .expect("SCALE FAIL-CLOSED: QSL matching");
    let total = started.elapsed();
    eprintln!("SCALE QSL report {report:?}, chunks {chunks}");
    assert_eq!(chunks, confirmations.len().div_ceil(QSL_MATCH_CHUNK_SIZE));
    assert_eq!(report.confirmations, confirmations.len());
    check(
        &mut failures,
        "QSL matching 20000 confirmations (total)",
        total,
        BUDGET_QSL_20K,
    );
    check(
        &mut failures,
        "QSL matching longest continuous lock hold",
        max_hold,
        BUDGET_QSL_LOCK_HOLD,
    );

    assert!(
        failures.is_empty(),
        "SCALE FAIL-CLOSED: budgets exceeded:\n{}",
        failures.join("\n")
    );
}
