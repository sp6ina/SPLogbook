// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use super::schema::QslChannel;
use rusqlite::{Result, params};

impl LogDatabase {
    fn mark_confirmed(
        &self,
        channel: QslChannel,
        callsign: &str,
        band: &str,
        mode: &str,
        qso_date: &str,
        rdate: &str,
    ) -> Result<usize> {
        let rcvd_col = channel.rcvd_column();
        let rdate_col = channel.rdate_column();
        let sql = format!(
            "UPDATE qso_records
             SET {rcvd_col} = 'Y', {rdate_col} = ?1
             WHERE UPPER(callsign) = UPPER(?2)
               AND UPPER(band) = UPPER(?3)
               AND (UPPER(mode) = UPPER(?4) OR (UPPER(?4) IN ('SSB', 'USB', 'LSB') AND UPPER(mode) IN ('SSB', 'USB', 'LSB')))
               AND REPLACE(qso_date, '-', '') = REPLACE(?5, '-', '')"
        );
        let count = self.conn.execute(
            &sql,
            params![
                rdate,
                callsign.trim(),
                band.trim(),
                mode.trim(),
                qso_date.trim()
            ],
        )?;
        Ok(count)
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
        self.mark_confirmed(QslChannel::Lotw, callsign, band, mode, qso_date, rdate)
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
        self.mark_confirmed(QslChannel::Eqsl, callsign, band, mode, qso_date, rdate)
    }
}
