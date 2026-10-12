// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::LogDatabase;
use rusqlite::{Result, params};

impl LogDatabase {
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
             WHERE UPPER(callsign) = UPPER(?2)
               AND UPPER(band) = UPPER(?3)
               AND (UPPER(mode) = UPPER(?4) OR (UPPER(?4) IN ('SSB', 'USB', 'LSB') AND UPPER(mode) IN ('SSB', 'USB', 'LSB')))
               AND REPLACE(qso_date, '-', '') = REPLACE(?5, '-', '')",
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
             WHERE UPPER(callsign) = UPPER(?2)
               AND UPPER(band) = UPPER(?3)
               AND (UPPER(mode) = UPPER(?4) OR (UPPER(?4) IN ('SSB', 'USB', 'LSB') AND UPPER(mode) IN ('SSB', 'USB', 'LSB')))
               AND REPLACE(qso_date, '-', '') = REPLACE(?5, '-', '')",
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
}
