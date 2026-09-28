// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Reprezentacja pojedynczego rekordu łączności (QSO) w standardzie ADIF 3.1.7
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct QsoRecord {
    pub id: Option<i64>,
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub submode: Option<String>,
    pub qso_date: String, // YYYYMMDD (standard ADIF)
    pub time_on: String,  // HHMMSS lub HHMM (standard ADIF)
    pub time_off: Option<String>,
    pub freq: Option<f64>,    // Częstotliwość TX w MHz
    pub freq_rx: Option<f64>, // Częstotliwość RX w MHz (np. split/satelity)
    pub rst_sent: String,
    pub rst_rcvd: String,
    pub name: Option<String>,
    pub qth: Option<String>,
    pub gridsquare: Option<String>,
    pub state: Option<String>,
    pub iota: Option<String>,
    pub sota_ref: Option<String>,
    pub pota_ref: Option<String>,
    pub pga_ref: Option<String>, // Polska Gmina Award (np. WR01)
    pub dxcc: Option<u32>,
    pub country: Option<String>,
    pub continent: Option<String>,
    pub cqz: Option<u32>,
    pub ituz: Option<u32>,
    pub comment: Option<String>,
    pub qsl_via: Option<String>,
    pub qsl_manager: Option<String>,
    pub qsl_sent: String,
    pub qsl_rcvd: String,
    pub qsl_sent_date: Option<String>,
    pub qsl_rcvd_date: Option<String>,
    pub lotw_qsl_sent: String,
    pub lotw_qsl_rcvd: String,
    pub lotw_qslrdate: Option<String>,
    pub eqsl_qsl_sent: String,
    pub eqsl_qsl_rcvd: String,
    pub eqsl_qslrdate: Option<String>,
    pub clublog_upload_status: Option<String>,
    pub qrzcom_upload_status: Option<String>,
    pub sat_name: Option<String>,
    pub sat_mode: Option<String>,
    pub prop_mode: Option<String>,
    pub srx: Option<u32>,
    pub stx: Option<u32>,
    pub srx_string: Option<String>,
    pub stx_string: Option<String>,
    pub my_gridsquare: Option<String>,
    pub my_state: Option<String>,
    pub my_pota_ref: Option<String>, // ADIF: MY_POTA_REF
    pub my_sota_ref: Option<String>, // ADIF: MY_SOTA_REF
    pub vucc_grids: Option<String>,  // ADIF: VUCC_GRIDS (np. dla satelitów/VHF)
    pub audio_file: Option<String>,  // Powiązany plik audio nagrania łączności
    pub journal_id: Option<String>, // Identyfikator profilu/dziennika (np. DEFAULT, PORTABLE, CONTEST)
}

impl Default for QsoRecord {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: None,
            callsign: String::new(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            submode: None,
            qso_date: now.format("%Y%m%d").to_string(),
            time_on: now.format("%H%M%S").to_string(),
            time_off: None,
            freq: Some(14.025),
            freq_rx: None,
            rst_sent: "599".to_string(),
            rst_rcvd: "599".to_string(),
            name: None,
            qth: None,
            gridsquare: None,
            state: None,
            iota: None,
            sota_ref: None,
            pota_ref: None,
            pga_ref: None,
            dxcc: None,
            country: None,
            continent: None,
            cqz: None,
            ituz: None,
            comment: None,
            qsl_via: None,
            qsl_manager: None,
            qsl_sent: "N".to_string(),
            qsl_rcvd: "N".to_string(),
            qsl_sent_date: None,
            qsl_rcvd_date: None,
            lotw_qsl_sent: "N".to_string(),
            lotw_qsl_rcvd: "N".to_string(),
            lotw_qslrdate: None,
            eqsl_qsl_sent: "N".to_string(),
            eqsl_qsl_rcvd: "N".to_string(),
            eqsl_qslrdate: None,
            clublog_upload_status: None,
            qrzcom_upload_status: None,
            sat_name: None,
            sat_mode: None,
            prop_mode: None,
            srx: None,
            stx: None,
            srx_string: None,
            stx_string: None,
            my_pota_ref: None,
            my_sota_ref: None,
            vucc_grids: None,
            my_gridsquare: None,
            my_state: None,
            audio_file: None,
            journal_id: Some("DEFAULT".to_string()),
        }
    }
}

impl QsoRecord {
    /// Tworzy nowy rekord QSO z podstawowymi parametrami
    pub fn new(
        callsign: impl Into<String>,
        band: impl Into<String>,
        mode: impl Into<String>,
    ) -> Self {
        let mode_str = mode.into().to_uppercase();
        let (rst_sent, rst_rcvd) = if matches!(
            mode_str.as_str(),
            "SSB" | "LSB" | "USB" | "FM" | "AM" | "DV" | "DMR" | "C4FM" | "DSTAR"
        ) {
            ("59".to_string(), "59".to_string())
        } else if matches!(
            mode_str.as_str(),
            "FT8"
                | "FT4"
                | "FT2"
                | "JS8"
                | "Q65"
                | "JT65"
                | "JT9"
                | "FST4"
                | "FST4W"
                | "WSPR"
                | "FREEDATA"
        ) {
            ("-10".to_string(), "-10".to_string())
        } else {
            ("599".to_string(), "599".to_string())
        };
        Self {
            callsign: callsign.into().to_uppercase(),
            band: band.into(),
            mode: mode_str,
            rst_sent,
            rst_rcvd,
            ..Default::default()
        }
    }

    /// Zwraca znormalizowany format daty YYYYMMDD dla ADIF
    pub fn adif_date(&self) -> String {
        self.qso_date.replace(['-', '.', '/'], "")
    }

    /// Zwraca znormalizowany format czasu HHMMSS lub HHMM dla ADIF
    pub fn adif_time(&self) -> String {
        self.time_on.replace(':', "")
    }

    /// Centralna walidacja domenowa rekordu QSO. Zwraca `Ok(())` gdy rekord
    /// nadaje się do zapisania, lub `Err` z opisem pierwszego wykrytego błędu.
    /// Wywoływana przy zapisie z formularza, edycji i imporcie, aby uniemożliwić
    /// zapis błędnej daty, czasu, pasma, częstotliwości i stref.
    pub fn validate(&self) -> Result<(), String> {
        let callsign = self.callsign.trim();
        if callsign.is_empty() {
            return Err("znak wywoławczy nie może być pusty".to_string());
        }
        if !callsign
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '/')
        {
            return Err(format!(
                "znak wywoławczy zawiera niedozwolone znaki: {callsign}"
            ));
        }

        if self.band.trim().is_empty() {
            return Err("pasmo nie może być puste".to_string());
        }
        if self.mode.trim().is_empty() {
            return Err("rodzaj emisji nie może być pusty".to_string());
        }

        let date = self.qso_date.trim().replace(['-', '.', '/'], "");
        if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!(
                "błędna data (oczekiwano YYYYMMDD): {}",
                self.qso_date
            ));
        }
        if chrono::NaiveDate::parse_from_str(&date, "%Y%m%d").is_err() {
            return Err(format!("nieprawidłowa data: {}", self.qso_date));
        }

        let time_on = self.time_on.trim().replace(':', "");
        let time_on_ok = (time_on.len() == 4 || time_on.len() == 6)
            && time_on.chars().all(|c| c.is_ascii_digit());
        if !time_on_ok {
            return Err(format!(
                "błędny czas rozpoczęcia (oczekiwano HHMM lub HHMMSS): {}",
                self.time_on
            ));
        }
        if let Some(ref toff) = self.time_off {
            let toff = toff.trim().replace(':', "");
            let toff_ok =
                (toff.len() == 4 || toff.len() == 6) && toff.chars().all(|c| c.is_ascii_digit());
            if !toff_ok {
                return Err(format!(
                    "błędny czas zakończenia (oczekiwano HHMM lub HHMMSS): {toff}"
                ));
            }
        }

        if let Some(f) = self.freq {
            if !f.is_finite() || f <= 0.0 {
                return Err(format!(
                    "częstotliwość TX musi być skończona i dodatnia: {f}"
                ));
            }
        }
        if let Some(f) = self.freq_rx {
            if !f.is_finite() || f < 0.0 {
                return Err(format!(
                    "częstotliwość RX musi być skończona i nieujemna: {f}"
                ));
            }
        }

        if let Some(cqz) = self.cqz {
            if !(1..=40).contains(&cqz) {
                return Err(format!("strefa CQ musi być z zakresu 1–40: {cqz}"));
            }
        }
        if let Some(ituz) = self.ituz {
            if !(1..=90).contains(&ituz) {
                return Err(format!("strefa ITU musi być z zakresu 1–90: {ituz}"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_qso() -> QsoRecord {
        let mut q = QsoRecord::new("SP6INA", "20m", "CW");
        q.qso_date = "20260928".to_string();
        q.time_on = "1830".to_string();
        q.freq = Some(14.025);
        q
    }

    #[test]
    fn validate_accepts_valid_qso() {
        assert!(valid_qso().validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_callsign() {
        let mut q = valid_qso();
        q.callsign = "   ".to_string();
        assert!(q.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_date() {
        let mut q = valid_qso();
        q.qso_date = "20261340".to_string();
        assert!(q.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_time() {
        let mut q = valid_qso();
        q.time_on = "12:3".to_string();
        assert!(q.validate().is_err());
    }

    #[test]
    fn validate_rejects_nan_freq() {
        let mut q = valid_qso();
        q.freq = Some(f64::NAN);
        assert!(q.validate().is_err());
    }

    #[test]
    fn validate_rejects_out_of_range_zone() {
        let mut q = valid_qso();
        q.cqz = Some(99);
        assert!(q.validate().is_err());
    }
}
