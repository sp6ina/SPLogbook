// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use super::schema::QslChannel;
use rusqlite::{Result, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QslConfirmationRecord<'a> {
    pub callsign: &'a str,
    pub band: &'a str,
    pub mode: &'a str,
    pub qso_date: &'a str,
    pub rdate: &'a str,
}

impl LogDatabase {
    fn mark_confirmed_batch(
        &self,
        journal_id: &str,
        channel: QslChannel,
        records: &[QslConfirmationRecord<'_>],
    ) -> Result<usize> {
        if records.is_empty() {
            return Ok(0);
        }
        let rcvd_col = channel.rcvd_column();
        let rdate_col = channel.rdate_column();
        let sql = format!(
            "UPDATE qso_records
             SET {rcvd_col} = 'Y', {rdate_col} = ?1
             WHERE UPPER(callsign) = UPPER(?2)
               AND UPPER(band) = UPPER(?3)
               AND (UPPER(mode) = UPPER(?4) OR (UPPER(?4) IN ('SSB', 'USB', 'LSB') AND UPPER(mode) IN ('SSB', 'USB', 'LSB')))
               AND REPLACE(qso_date, '-', '') = REPLACE(?5, '-', '')
               AND journal_id = ?6"
        );
        let mut total = 0;
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(&sql)?;
            for rec in records {
                total += stmt.execute(params![
                    rec.rdate.trim(),
                    rec.callsign.trim(),
                    rec.band.trim(),
                    rec.mode.trim(),
                    rec.qso_date.trim(),
                    journal_id,
                ])?;
            }
        }
        tx.commit()?;
        Ok(total)
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
            rdate,
        };
        self.mark_confirmed_batch(journal_id, channel, &[rec])
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

    /// Wsadowe oznaczanie potwierdzeń LoTW w ramach konkretnego dziennika w pojedynczej transakcji
    pub fn mark_lotw_confirmed_batch_for_journal(
        &self,
        journal_id: &str,
        confirmations: &[crate::cloud::lotw::LotwConfirmation],
    ) -> Result<usize> {
        let records: Vec<QslConfirmationRecord<'_>> = confirmations
            .iter()
            .map(|c| QslConfirmationRecord {
                callsign: &c.callsign,
                band: &c.band,
                mode: &c.mode,
                qso_date: &c.qso_date,
                rdate: &c.qsl_rdate,
            })
            .collect();
        self.mark_confirmed_batch(journal_id, QslChannel::Lotw, &records)
    }

    /// Wsadowe oznaczanie potwierdzeń eQSL w ramach konkretnego dziennika w pojedynczej transakcji
    pub fn mark_eqsl_confirmed_batch_for_journal(
        &self,
        journal_id: &str,
        qsos: &[crate::core::qso::QsoRecord],
    ) -> Result<usize> {
        let records: Vec<QslConfirmationRecord<'_>> = qsos
            .iter()
            .map(|q| QslConfirmationRecord {
                callsign: &q.callsign,
                band: &q.band,
                mode: &q.mode,
                qso_date: &q.qso_date,
                rdate: q.eqsl_qslrdate.as_deref().unwrap_or(&q.qso_date),
            })
            .collect();
        self.mark_confirmed_batch(journal_id, QslChannel::Eqsl, &records)
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
