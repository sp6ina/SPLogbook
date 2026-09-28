// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Statystyki kontestowe: metr tempa QSO/h, historia tempa do wykresu oraz
//! macierz mnożników per pasmo z podświetleniem potrzebnych mnożników.

use crate::core::qso::QsoRecord;
use chrono::{NaiveDate, NaiveTime};

/// Konwertuje datę i czas QSO na uniksowy znacznik czasu (sekundy, UTC).
/// Obsługuje formaty ADIF: `YYYY-MM-DD`/`YYYYMMDD` oraz `HH:MM:SS`/`HHMMSS`.
pub fn qso_epoch_secs(q: &QsoRecord) -> Option<i64> {
    let d = q.qso_date.trim();
    let date = if d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit()) {
        NaiveDate::parse_from_str(d, "%Y%m%d").ok()
    } else {
        NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
    }?;

    let t = q.time_on.trim();
    let time = if t.len() == 6 && t.bytes().all(|b| b.is_ascii_digit()) {
        NaiveTime::parse_from_str(t, "%H%M%S").ok()
    } else {
        NaiveTime::parse_from_str(t, "%H:%M:%S").ok()
    }
    .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

    Some(date.and_time(time).and_utc().timestamp())
}

/// Wynik pomiaru tempa pracy w kontescie.
#[derive(Debug, Clone, Default)]
pub struct RateStats {
    pub now_secs: i64,
    /// Liczba QSO w ostatnich 1/5/10/60 minutach.
    pub last_1m: u32,
    pub last_5m: u32,
    pub last_10m: u32,
    pub last_60m: u32,
    /// Historia tempa: 60 kubełków po 1 minucie, od najstarszego do najnowszego.
    pub per_minute: Vec<u32>,
}

impl RateStats {
    /// Tempo ekstrapolowane na godzinę (QSO/h) dla okna 1 minuty.
    pub fn rate_1m(&self) -> u32 {
        self.last_1m.saturating_mul(60)
    }
    /// Tempo ekstrapolowane na godzinę (QSO/h) dla okna 5 minut.
    pub fn rate_5m(&self) -> u32 {
        self.last_5m.saturating_mul(12)
    }
    /// Tempo ekstrapolowane na godzinę (QSO/h) dla okna 10 minut.
    pub fn rate_10m(&self) -> u32 {
        self.last_10m.saturating_mul(6)
    }
    /// Tempo rzeczywiste (QSO/h) dla okna 60 minut.
    pub fn rate_60m(&self) -> u32 {
        self.last_60m
    }
}

/// Oblicza metr tempa dla podanych QSO względem `now_secs` (uniksowy czas UTC).
pub fn compute_rate(qsos: &[QsoRecord], now_secs: i64) -> RateStats {
    let mut stats = RateStats {
        now_secs,
        per_minute: vec![0u32; 60],
        ..Default::default()
    };

    for q in qsos {
        let Some(ts) = qso_epoch_secs(q) else { continue };
        let age = now_secs - ts;
        if age < 0 {
            continue;
        }
        if age <= 60 {
            stats.last_1m += 1;
        }
        if age <= 300 {
            stats.last_5m += 1;
        }
        if age <= 600 {
            stats.last_10m += 1;
        }
        if age <= 3600 {
            stats.last_60m += 1;
            let bucket = (age / 60) as usize;
            if bucket < 60 {
                stats.per_minute[59 - bucket] += 1;
            }
        }
    }

    stats
}

/// Rodzaj mnożnika w kontescie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultKind {
    /// Kraje DXCC (`dxcc`).
    Dxcc,
    /// Strefy CQ (`cqz`), pełny zbiór 1..=40.
    CqZone,
    /// Stany/prowincje/obwody (`state`).
    State,
    /// Lokatory (`gridsquare`, 4 znaki).
    Grid,
    /// Referencje IOTA (`iota`).
    Iota,
    /// Prefiksy znaków (WPX).
    Prefix,
    /// Kraj × pasmo (np. BARTG).
    DxccPerBand,
    /// Brak mnożnika (konkursy tylko punktowe).
    None,
}

impl MultKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MultKind::Dxcc => "DXCC",
            MultKind::CqZone => "CQ Zone",
            MultKind::State => "State/Prov",
            MultKind::Grid => "Grid",
            MultKind::Iota => "IOTA",
            MultKind::Prefix => "Prefix",
            MultKind::DxccPerBand => "DXCC/Band",
            MultKind::None => "None",
        }
    }
}

/// Wyciąga wartość mnożnika z rekordu QSO dla danego rodzaju.
pub fn mult_value(kind: MultKind, q: &QsoRecord) -> Option<String> {
    match kind {
        MultKind::Dxcc => q.dxcc.map(|d| d.to_string()),
        MultKind::CqZone => q.cqz.map(|z| format!("{z:02}")),
        MultKind::State => q.state.clone(),
        MultKind::Grid => q.gridsquare.as_ref().map(|g| g.chars().take(4).collect()),
        MultKind::Iota => q.iota.clone(),
        MultKind::Prefix => {
            let prefix: String = q
                .callsign
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(3)
                .collect();
            if prefix.is_empty() {
                None
            } else {
                Some(prefix)
            }
        }
        MultKind::DxccPerBand => q.dxcc.map(|d| format!("{}-{}", d, q.band)),
        MultKind::None => None,
    }
}

/// Macierz mnożników: wiersze = pasma, kolumny = mnożniki.
///
/// Każda komórka `worked` przyjmuje:
/// - `2` — mnożnik zaliczony na tym paśmie (zielony),
/// - `1` — mnożnik zaliczony gdzie indziej, dostępny na tym paśmie (żółty),
/// - `0` — mnożnik nigdy niezliczony (szary = potrzebny).
#[derive(Debug, Clone, Default)]
pub struct MultMatrix {
    pub bands: Vec<String>,
    pub columns: Vec<String>,
    pub worked: Vec<Vec<u8>>,
}

impl MultMatrix {
    /// Liczba mnożników nigdy niezliczonych (potrzebnych) w całej macierzy.
    pub fn needed_count(&self) -> usize {
        self.columns
            .iter()
            .enumerate()
            .filter(|(ci, _)| self.worked.iter().all(|row| row[*ci] == 0))
            .count()
    }
}

/// Buduje macierz mnożników dla pasm `bands` i listy QSO.
pub fn compute_mult_matrix(kind: MultKind, bands: &[&str], qsos: &[QsoRecord]) -> MultMatrix {
    let mut columns_set: Vec<String> = Vec::new();

    let push_col = |columns_set: &mut Vec<String>, value: String| {
        if !columns_set.contains(&value) {
            columns_set.push(value);
        }
    };

    // Zbiór mnożników z QSO + (dla stref CQ) pełny zestaw 1..=40.
    for q in qsos {
        if let Some(v) = mult_value(kind, q) {
            push_col(&mut columns_set, v);
        }
    }
    if kind == MultKind::CqZone {
        for z in 1..=40u32 {
            push_col(&mut columns_set, format!("{z:02}"));
        }
    }

    columns_set.sort();

    let band_rows: Vec<String> = bands.iter().map(std::string::ToString::to_string).collect();

    let mut worked = vec![vec![0u8; columns_set.len()]; band_rows.len()];

    for q in qsos {
        let Some(v) = mult_value(kind, q) else { continue };
        let Some(col) = columns_set.iter().position(|c| *c == v) else { continue };
        let Some(row) = band_rows.iter().position(|b| *b == q.band) else { continue };
        worked[row][col] = 2; // zaliczony na tym paśmie
    }

    // Oznacz mnożniki dostępne na danym paśmie, ale zaliczone gdzie indziej.
    for col in 0..columns_set.len() {
        let worked_somewhere = worked.iter().any(|row| row[col] == 2);
        if worked_somewhere {
            for row in worked.iter_mut().take(band_rows.len()) {
                if row[col] == 0 {
                    row[col] = 1;
                }
            }
        }
    }

    MultMatrix {
        bands: band_rows,
        columns: columns_set,
        worked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso(call: &str, band: &str, date: &str, time: &str, dxcc: Option<u32>, cqz: Option<u32>) -> QsoRecord {
        let mut q = QsoRecord::new(call, band, "CW");
        q.qso_date = date.to_string();
        q.time_on = time.to_string();
        q.dxcc = dxcc;
        q.cqz = cqz;
        q
    }

    #[test]
    fn test_qso_epoch_secs_both_formats() {
        let a = qso("SP6INA", "20m", "2026-01-01", "12:34:56", None, None);
        let b = qso("SP6INA", "20m", "20260101", "123456", None, None);
        assert_eq!(qso_epoch_secs(&a), qso_epoch_secs(&b));
        // 2026-01-01 12:34:56 UTC
        let expected = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_time(NaiveTime::from_hms_opt(12, 34, 56).unwrap())
            .and_utc()
            .timestamp();
        assert_eq!(qso_epoch_secs(&a), Some(expected));
    }

    #[test]
    fn test_compute_rate_windows() {
        let now = qso_epoch_secs(&qso("X", "20m", "2026-01-01", "12:00:00", None, None)).unwrap();
        // 2 QSO w ostatniej minucie, 1 w 5 min, 1 w 10 min, 1 w 60 min
        let qsos = vec![
            qso("A", "20m", "2026-01-01", "11:59:30", None, None),
            qso("B", "20m", "2026-01-01", "11:59:10", None, None),
            qso("C", "20m", "2026-01-01", "11:57:00", None, None),
            qso("D", "20m", "2026-01-01", "11:54:00", None, None),
            qso("E", "20m", "2026-01-01", "11:20:00", None, None),
            qso("F", "20m", "2026-01-01", "10:00:00", None, None), // poza oknem
        ];

        let stats = compute_rate(&qsos, now);
        assert_eq!(stats.last_1m, 2);
        assert_eq!(stats.last_5m, 3);
        assert_eq!(stats.last_10m, 4);
        assert_eq!(stats.last_60m, 5);
        assert_eq!(stats.rate_1m(), 120);
        assert_eq!(stats.rate_5m(), 36);
        assert_eq!(stats.rate_60m(), 5);
    }

    #[test]
    fn test_mult_matrix_dxcc() {
        let qsos = vec![
            qso("A", "20m", "2026-01-01", "12:00:00", Some(269), None),
            qso("B", "20m", "2026-01-01", "12:01:00", Some(291), None),
            qso("C", "40m", "2026-01-01", "12:02:00", Some(269), None),
        ];

        let m = compute_mult_matrix(MultKind::Dxcc, &["20m", "40m"], &qsos);
        assert_eq!(m.bands, vec!["20m", "40m"]);
        assert_eq!(m.columns, vec!["269", "291"]);
        // 269 na 20m = 2, na 40m = 2; 291 tylko na 20m = 2, na 40m = 1
        assert_eq!(m.worked[0], vec![2, 2]);
        assert_eq!(m.worked[1], vec![2, 1]);
        assert_eq!(m.needed_count(), 0);
    }

    #[test]
    fn test_mult_matrix_cqzone_full_set() {
        let qsos = vec![
            qso("A", "20m", "2026-01-01", "12:00:00", None, Some(15)),
        ];

        let m = compute_mult_matrix(MultKind::CqZone, &["20m"], &qsos);
        assert_eq!(m.columns.len(), 40);
        assert!(m.columns.contains(&"15".to_string()));
        // 39 stref nigdy niezaliczonych
        assert_eq!(m.needed_count(), 39);
    }
}
