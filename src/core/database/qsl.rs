// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Dopasowanie potwierdzeń LoTW / eQSL do łączności w dzienniku (STAGE-V140-S1).
//!
//! Kontrakt (`docs/LOTW_EQSL_CONTRACT.md` §6):
//! - wejście jest normalizowane w Rust (znak `trim` + wielkie litery, data `YYYYMMDD`), a
//!   kandydaci są wyszukiwani po indeksie `idx_qso_qsl_match` — bez `UPPER(...)` na kolumnach;
//! - gdy raport podaje `TIME_ON`, potwierdzane jest wyłącznie QSO w oknie ±30 min UTC (także
//!   przez północ UTC — dzień sąsiedni jest dołączany przez `IN`), najbliższe w czasie;
//! - gdy raport nie podaje czasu, potwierdzane jest tylko jedyne QSO danego dnia; kilku
//!   kandydatów to „niejednoznaczne” (nic nie jest oznaczane) — bez zgadywania;
//! - przetwarzanie odbywa się w paczkach po [`QSL_MATCH_CHUNK_SIZE`] w osobnych transakcjach,
//!   a blokada bazy jest oddawana między paczkami;
//! - QSO już potwierdzone z tą samą datą nie jest przepisywane (brak zbędnych zapisów WAL).

use super::LogDatabase;
use super::schema::QslChannel;
use rusqlite::{Result, params};
use std::sync::{Mutex, PoisonError};

/// Okno dopasowania czasu potwierdzenia do QSO: ±30 minut UTC.
pub const QSL_MATCH_WINDOW_SECS: i64 = 30 * 60;

/// Maksymalna liczba potwierdzeń w jednej transakcji (i jednym trzymaniu blokady bazy).
///
/// Pomiar na 200 000 QSO (STAGE-V140-S1): koszt dominujący to zapisy WAL, nie wyszukiwanie,
/// więc paczka jest mała, aby wątek UI nigdy nie czekał długo na blokadę bazy.
pub const QSL_MATCH_CHUNK_SIZE: usize = 100;

const SECS_PER_DAY: i64 = 86_400;

/// Zapytanie kandydatów: równości na `journal_id` i `callsign`, `IN` na pasmo (wariant małych
/// i wielkich liter) oraz na datę (dzień potwierdzenia i ewentualnie dni sąsiednie). Wyrażenie
/// daty jest identyczne z indeksem `idx_qso_qsl_match` (migracja 10).
pub(super) const QSL_CANDIDATES_SQL: &str = "SELECT id, mode, qso_date, time_on FROM qso_records \
     WHERE journal_id = ?1 AND callsign = ?2 AND band IN (?3, ?4) \
     AND REPLACE(qso_date, '-', '') IN (?5, ?6, ?7)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QslConfirmationRecord<'a> {
    pub callsign: &'a str,
    pub band: &'a str,
    pub mode: &'a str,
    pub qso_date: &'a str,
    /// UTC `TIME_ON` z raportu usługi (`HHMM` lub `HHMMSS`); `None` — raport bez czasu.
    pub time_on: Option<&'a str>,
    /// Received date; `None` (or blank) keeps the date already stored (F20-R4).
    pub rdate: Option<&'a str>,
}

/// Wynik dopasowania partii potwierdzeń.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QslMatchReport {
    /// Liczba przetworzonych potwierdzeń.
    pub confirmations: usize,
    /// Potwierdzenia, które oznaczyły dokładnie jedno QSO.
    pub matched: usize,
    /// Potwierdzenia bez czasu z kilkoma kandydatami danego dnia — nic nie oznaczono.
    pub ambiguous: usize,
    /// Potwierdzenia bez pasującego QSO (także poza oknem ±30 min).
    pub unmatched: usize,
}

impl QslMatchReport {
    fn absorb(&mut self, other: Self) {
        self.confirmations += other.confirmations;
        self.matched += other.matched;
        self.ambiguous += other.ambiguous;
        self.unmatched += other.unmatched;
    }
}

/// Rozstrzygnięcie pojedynczego potwierdzenia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    Matched(i64),
    Ambiguous,
    Unmatched,
}

/// Numer dnia od 1970-01-01 dla daty `YYYYMMDD` (myślniki są pomijane); `None` dla daty
/// niepoprawnej.
fn epoch_day(date: &str) -> Option<i64> {
    let digits: String = date.trim().chars().filter(|c| *c != '-').collect();
    if digits.len() != 8 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year: i64 = digits[0..4].parse().ok()?;
    let month: i64 = digits[4..6].parse().ok()?;
    let day: i64 = digits[6..8].parse().ok()?;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=month_days).contains(&day) {
        return None;
    }
    // Algorytm „days from civil” (kalendarz gregoriański proleptyczny).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Data `YYYYMMDD` dla numeru dnia od 1970-01-01.
fn date_from_epoch_day(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}")
}

/// Sekundy od północy UTC dla `HHMM` / `HHMMSS` (dwukropki są pomijane).
fn seconds_of_day(time: &str) -> Option<i64> {
    let digits: String = time.trim().chars().filter(|c| *c != ':').collect();
    if !(digits.len() == 4 || digits.len() == 6) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hh: i64 = digits[0..2].parse().ok()?;
    let mm: i64 = digits[2..4].parse().ok()?;
    let ss: i64 = if digits.len() == 6 {
        digits[4..6].parse().ok()?
    } else {
        0
    };
    if hh > 23 || mm > 59 || ss > 59 {
        return None;
    }
    Some(hh * 3600 + mm * 60 + ss)
}

/// Zgodność emisji jak w dotychczasowym SQL: równość bez względu na wielkość liter oraz
/// wspólna grupa SSB/USB/LSB.
fn modes_match(stored: &str, reported: &str) -> bool {
    const PHONE: [&str; 3] = ["SSB", "USB", "LSB"];
    let stored = stored.trim().to_ascii_uppercase();
    let reported = reported.trim().to_ascii_uppercase();
    stored == reported || (PHONE.contains(&reported.as_str()) && PHONE.contains(&stored.as_str()))
}

fn resolve(
    select: &mut rusqlite::Statement<'_>,
    journal_id: &str,
    rec: &QslConfirmationRecord<'_>,
) -> Result<Resolution> {
    let callsign = rec.callsign.trim().to_uppercase();
    let band = rec.band.trim();
    let Some(day) = epoch_day(rec.qso_date) else {
        return Ok(Resolution::Unmatched);
    };
    let reported_secs = rec.time_on.and_then(seconds_of_day);

    let today = date_from_epoch_day(day);
    let (before, after) = match reported_secs {
        Some(secs) => (
            if secs < QSL_MATCH_WINDOW_SECS {
                date_from_epoch_day(day - 1)
            } else {
                today.clone()
            },
            if secs + QSL_MATCH_WINDOW_SECS >= SECS_PER_DAY {
                date_from_epoch_day(day + 1)
            } else {
                today.clone()
            },
        ),
        None => (today.clone(), today.clone()),
    };

    let rows = select.query_map(
        params![
            journal_id,
            callsign,
            band.to_lowercase(),
            band.to_uppercase(),
            today,
            before,
            after
        ],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;
    let mut candidates = Vec::new();
    for row in rows {
        let (id, mode, qso_date, time_on) = row?;
        if modes_match(&mode, rec.mode) {
            candidates.push((id, qso_date, time_on));
        }
    }

    match reported_secs {
        Some(secs) => {
            let reported_at = day * SECS_PER_DAY + secs;
            let nearest = candidates
                .iter()
                .filter_map(|(id, qso_date, time_on)| {
                    let at = epoch_day(qso_date)? * SECS_PER_DAY + seconds_of_day(time_on)?;
                    let distance = (at - reported_at).abs();
                    (distance <= QSL_MATCH_WINDOW_SECS).then_some((distance, *id))
                })
                .min();
            Ok(nearest.map_or(Resolution::Unmatched, |(_, id)| Resolution::Matched(id)))
        }
        None => match candidates.as_slice() {
            [] => Ok(Resolution::Unmatched),
            [(id, _, _)] => Ok(Resolution::Matched(*id)),
            _ => Ok(Resolution::Ambiguous),
        },
    }
}

impl LogDatabase {
    fn mark_confirmed_batch_report(
        &self,
        journal_id: &str,
        channel: QslChannel,
        records: &[QslConfirmationRecord<'_>],
    ) -> Result<QslMatchReport> {
        let mut report = QslMatchReport::default();
        if records.is_empty() {
            return Ok(report);
        }
        let rcvd_col = channel.rcvd_column();
        let rdate_col = channel.rdate_column();
        // Zapis tylko wtedy, gdy coś się zmienia: QSO już potwierdzone z tą samą datą (lub
        // raport bez daty) nie jest przepisywane — pełne pobranie raportu nie generuje wtedy
        // zapisów WAL, a potwierdzenie nadal liczy się jako dopasowane.
        let update_sql = format!(
            "UPDATE qso_records
             SET {rcvd_col} = 'Y', {rdate_col} = COALESCE(?1, {rdate_col})
             WHERE id = ?2 AND journal_id = ?3
               AND ({rcvd_col} IS NOT 'Y' OR (?1 IS NOT NULL AND {rdate_col} IS NOT ?1))"
        );
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut select = tx.prepare(QSL_CANDIDATES_SQL)?;
            let mut update = tx.prepare(&update_sql)?;
            for rec in records {
                report.confirmations += 1;
                match resolve(&mut select, journal_id, rec)? {
                    Resolution::Matched(id) => {
                        update.execute(params![
                            rec.rdate.map(str::trim).filter(|d| !d.is_empty()),
                            id,
                            journal_id,
                        ])?;
                        report.matched += 1;
                    }
                    Resolution::Ambiguous => report.ambiguous += 1,
                    Resolution::Unmatched => report.unmatched += 1,
                }
            }
        }
        tx.commit()?;
        Ok(report)
    }

    fn mark_confirmed_scoped(
        &self,
        journal_id: &str,
        channel: QslChannel,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        let rec = QslConfirmationRecord {
            callsign,
            band,
            mode,
            qso_date,
            time_on: None,
            rdate: Some(rdate),
        };
        Ok(self
            .mark_confirmed_batch_report(journal_id, channel, &[rec])?
            .matched)
    }

    /// Oznacza potwierdzenie LoTW dla dopasowanej łączności w ramach konkretnego dziennika
    pub fn mark_lotw_confirmed_scoped(
        &self,
        journal_id: &str,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        self.mark_confirmed_scoped(
            journal_id,
            QslChannel::Lotw,
            callsign,
            band,
            mode,
            qso_date,
            rdate,
        )
    }

    /// Oznacza potwierdzenie eQSL dla dopasowanej łączności w ramach konkretnego dziennika
    pub fn mark_eqsl_confirmed_scoped(
        &self,
        journal_id: &str,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        self.mark_confirmed_scoped(
            journal_id,
            QslChannel::Eqsl,
            callsign,
            band,
            mode,
            qso_date,
            rdate,
        )
    }

    /// Wsadowe oznaczanie potwierdzeń LoTW w ramach konkretnego dziennika w pojedynczej
    /// transakcji; zwraca liczbę potwierdzeń, które oznaczyły QSO.
    pub fn mark_lotw_confirmed_batch_for_journal(
        &self,
        journal_id: &str,
        confirmations: &[crate::cloud::lotw::LotwConfirmation],
    ) -> Result<usize> {
        Ok(self
            .mark_lotw_confirmed_batch_report(journal_id, confirmations)?
            .matched)
    }

    /// Jak [`Self::mark_lotw_confirmed_batch_for_journal`], z pełnym raportem dopasowania.
    pub fn mark_lotw_confirmed_batch_report(
        &self,
        journal_id: &str,
        confirmations: &[crate::cloud::lotw::LotwConfirmation],
    ) -> Result<QslMatchReport> {
        let records: Vec<QslConfirmationRecord<'_>> = confirmations
            .iter()
            .map(|c| QslConfirmationRecord {
                callsign: &c.callsign,
                band: &c.band,
                mode: &c.mode,
                qso_date: &c.qso_date,
                time_on: c.qso_time.as_deref(),
                rdate: c.qsl_rdate.as_deref(),
            })
            .collect();
        self.mark_confirmed_batch_report(journal_id, QslChannel::Lotw, &records)
    }

    /// Wsadowe oznaczanie potwierdzeń eQSL w ramach konkretnego dziennika w pojedynczej
    /// transakcji; zwraca liczbę potwierdzeń, które oznaczyły QSO.
    pub fn mark_eqsl_confirmed_batch_for_journal(
        &self,
        journal_id: &str,
        qsos: &[crate::core::qso::QsoRecord],
    ) -> Result<usize> {
        Ok(self
            .mark_eqsl_confirmed_batch_report(journal_id, qsos)?
            .matched)
    }

    /// Jak [`Self::mark_eqsl_confirmed_batch_for_journal`], z pełnym raportem dopasowania.
    pub fn mark_eqsl_confirmed_batch_report(
        &self,
        journal_id: &str,
        qsos: &[crate::core::qso::QsoRecord],
    ) -> Result<QslMatchReport> {
        let resolved: Vec<(&crate::core::qso::QsoRecord, Option<String>)> = qsos
            .iter()
            .map(|q| {
                let rdate = q
                    .eqsl_qslrdate
                    .as_deref()
                    .and_then(crate::core::adif::adif_date_part)
                    .or_else(|| {
                        if q.eqsl_qsl_rcvd.trim().eq_ignore_ascii_case("Y")
                            || q.qsl_rcvd.trim().eq_ignore_ascii_case("Y")
                        {
                            q.qsl_rcvd_date
                                .as_deref()
                                .and_then(crate::core::adif::adif_date_part)
                        } else {
                            None
                        }
                    });
                (q, rdate)
            })
            .collect();
        let records: Vec<QslConfirmationRecord<'_>> = resolved
            .iter()
            .map(|(q, rdate)| QslConfirmationRecord {
                callsign: &q.callsign,
                band: &q.band,
                mode: &q.mode,
                qso_date: &q.qso_date,
                time_on: Some(q.time_on.as_str()),
                rdate: rdate.as_deref(),
            })
            .collect();
        self.mark_confirmed_batch_report(journal_id, QslChannel::Eqsl, &records)
    }

    /// Oznacza potwierdzenie LoTW dla dopasowanej łączności (domyślny dziennik)
    pub fn mark_lotw_confirmed(
        &self,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        self.mark_lotw_confirmed_scoped("DEFAULT", callsign, band, mode, qso_date, rdate)
    }

    /// Oznacza potwierdzenie eQSL dla dopasowanej łączności (domyślny dziennik)
    pub fn mark_eqsl_confirmed(
        &self,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        self.mark_eqsl_confirmed_scoped("DEFAULT", callsign, band, mode, qso_date, rdate)
    }
}

/// Dopasowuje potwierdzenia LoTW paczkami po [`QSL_MATCH_CHUNK_SIZE`]: każda paczka to osobna
/// transakcja pod krótko trzymaną blokadą `db`, zwalnianą przed wywołaniem `between_chunks`
/// (np. raport postępu). Wątek UI nigdy nie czeka na całą partię.
pub fn apply_lotw_confirmations_chunked(
    db: &Mutex<LogDatabase>,
    journal_id: &str,
    confirmations: &[crate::cloud::lotw::LotwConfirmation],
    mut between_chunks: impl FnMut(&QslMatchReport),
) -> Result<QslMatchReport> {
    let mut total = QslMatchReport::default();
    for chunk in confirmations.chunks(QSL_MATCH_CHUNK_SIZE) {
        let part = {
            let guard = db.lock().unwrap_or_else(PoisonError::into_inner);
            guard.mark_lotw_confirmed_batch_report(journal_id, chunk)?
        };
        total.absorb(part);
        between_chunks(&total);
    }
    Ok(total)
}

/// Dopasowuje potwierdzenia eQSL paczkami — kontrakt jak [`apply_lotw_confirmations_chunked`].
pub fn apply_eqsl_confirmations_chunked(
    db: &Mutex<LogDatabase>,
    journal_id: &str,
    qsos: &[crate::core::qso::QsoRecord],
    mut between_chunks: impl FnMut(&QslMatchReport),
) -> Result<QslMatchReport> {
    let mut total = QslMatchReport::default();
    for chunk in qsos.chunks(QSL_MATCH_CHUNK_SIZE) {
        let part = {
            let guard = db.lock().unwrap_or_else(PoisonError::into_inner);
            guard.mark_eqsl_confirmed_batch_report(journal_id, chunk)?
        };
        total.absorb(part);
        between_chunks(&total);
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_day_round_trips_and_rejects_invalid_dates() {
        for date in ["19700101", "20000229", "20261231", "20270101", "19301231"] {
            let day = epoch_day(date).expect("valid date");
            assert_eq!(date_from_epoch_day(day), date);
        }
        assert_eq!(epoch_day("1970-01-02"), Some(1));
        assert_eq!(
            epoch_day("20270101").unwrap() - epoch_day("20261231").unwrap(),
            1
        );
        for bad in ["", "2026", "20261301", "20260230", "2026-02-29", "2026x101"] {
            assert_eq!(epoch_day(bad), None, "{bad}");
        }
    }

    #[test]
    fn seconds_of_day_accepts_adif_and_colon_forms() {
        assert_eq!(seconds_of_day("0000"), Some(0));
        assert_eq!(seconds_of_day("2359"), Some(86_340));
        assert_eq!(seconds_of_day("235959"), Some(86_399));
        assert_eq!(seconds_of_day("12:34:56"), Some(45_296));
        for bad in ["", "12", "12345", "2400", "1260", "126060", "ab12"] {
            assert_eq!(seconds_of_day(bad), None, "{bad}");
        }
    }

    #[test]
    fn mode_matching_keeps_the_phone_group() {
        assert!(modes_match("SSB", "usb"));
        assert!(modes_match("LSB", "SSB"));
        assert!(modes_match("cw", "CW"));
        assert!(!modes_match("FT8", "FT4"));
        assert!(!modes_match("CW", "SSB"));
    }
}
