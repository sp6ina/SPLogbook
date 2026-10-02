// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::sync::OnceLock;

use chrono::Utc;
use regex::Regex;
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

/// Waliduje pojedyncze pole czasu w formacie HHMM lub HHMMSS.
/// Sprawdza format (długość 4/6, same cyfry) oraz zakresy: godzina 00–23,
/// minuty 00–59, sekundy 00–59 (tylko dla HHMMSS).
fn validate_time_field(raw: &str, field_name: &str) -> Result<(), String> {
    let normalized = raw.trim().replace(':', "");
    let valid_len = normalized.len() == 4 || normalized.len() == 6;
    let valid_digits = normalized.chars().all(|c| c.is_ascii_digit());
    if !valid_len || !valid_digits {
        return Err(format!(
            "błędny {field_name} (oczekiwano HHMM lub HHMMSS): {raw}"
        ));
    }

    let hh: u32 = normalized[0..2].parse().unwrap();
    let mm: u32 = normalized[2..4].parse().unwrap();
    if hh > 23 {
        return Err(format!(
            "błędny {field_name} — godzina poza zakresem 00–23: {raw}"
        ));
    }
    if mm > 59 {
        return Err(format!(
            "błędny {field_name} — minuty poza zakresem 00–59: {raw}"
        ));
    }
    if normalized.len() == 6 {
        let ss: u32 = normalized[4..6].parse().unwrap();
        if ss > 59 {
            return Err(format!(
                "błędny {field_name} — sekundy poza zakresem 00–59: {raw}"
            ));
        }
    }
    Ok(())
}

/// Zwraca ostrzeżenie, jeśli niepusty napis nie pasuje do formatu IOTA (XX-NNN,
/// np. EU-001). Walidacja miękka — nie blokuje zapisu.
fn validate_iota_format(value: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z]{2}-\d{3}$").unwrap());
    let trimmed = value.trim();
    if trimmed.is_empty() || re.is_match(trimmed) {
        None
    } else {
        Some(format!(
            "nieprawidłowy format IOTA (oczekiwano XX-NNN, np. EU-001): {value}"
        ))
    }
}

/// Zwraca ostrzeżenie, jeśli niepusty napis nie pasuje do formatu SOTA
/// (XX/YY-NNN, np. SP/TA-001). Walidacja miękka — nie blokuje zapisu.
fn validate_sota_format(value: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9]{1,3}/[A-Za-z0-9]{2,3}-\d{3}$").unwrap());
    let trimmed = value.trim();
    if trimmed.is_empty() || re.is_match(trimmed) {
        None
    } else {
        Some(format!(
            "nieprawidłowy format SOTA (oczekiwano XX/YY-NNN, np. SP/TA-001): {value}"
        ))
    }
}

/// Zwraca ostrzeżenie, jeśli niepusty napis nie pasuje do formatu POTA
/// (XX-NNNN, np. SP-0001). Walidacja miękka — nie blokuje zapisu.
fn validate_pota_format(value: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9]{1,4}-\d{4}$").unwrap());
    let trimmed = value.trim();
    if trimmed.is_empty() || re.is_match(trimmed) {
        None
    } else {
        Some(format!(
            "nieprawidłowy format POTA (oczekiwano XX-NNNN, np. SP-0001): {value}"
        ))
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

    /// Zwraca klucz jednoznaczności QSO (`ZNAK|PASMO|EMISJA|DATA|CZAS`), używany do
    /// idempotentnego importu ADIF: rekordy o tym samym kluczu są pomijane jako
    /// duplikaty. Format jest zgodny z `LogDatabase::existing_qso_keys`.
    pub fn dedup_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.callsign.to_uppercase(),
            self.band.to_uppercase(),
            self.mode.to_uppercase(),
            self.qso_date.replace('-', ""),
            self.time_on.replace(':', "")
        )
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

        validate_time_field(&self.time_on, "czas rozpoczęcia")?;

        if let Some(ref toff) = self.time_off {
            validate_time_field(toff, "czas zakończenia")?;
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

    /// Zwraca miękkie ostrzeżenia walidacyjne (nie blokują zapisu). Służą do
    /// sygnalizowania wartości o podejrzanym formacie (IOTA/SOTA/POTA), które
    /// warto skorygować, ale które nie powinny uniemożliwiać zapisu rekordu.
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if let Some(ref value) = self.iota {
            if let Some(w) = validate_iota_format(value) {
                warnings.push(w);
            }
        }
        if let Some(ref value) = self.sota_ref {
            if let Some(w) = validate_sota_format(value) {
                warnings.push(w);
            }
        }
        if let Some(ref value) = self.pota_ref {
            if let Some(w) = validate_pota_format(value) {
                warnings.push(w);
            }
        }
        warnings
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
    fn validate_rejects_hour_out_of_range() {
        let mut q = valid_qso();
        q.time_on = "2460".to_string();
        assert!(q.validate().is_err());
    }

    #[test]
    fn validate_rejects_minute_out_of_range() {
        let mut q = valid_qso();
        q.time_on = "1860".to_string();
        assert!(q.validate().is_err());
    }

    #[test]
    fn validate_rejects_second_out_of_range() {
        let mut q = valid_qso();
        q.time_on = "183060".to_string();
        assert!(q.validate().is_err());
    }

    #[test]
    fn validate_accepts_edge_times() {
        let mut q = valid_qso();
        q.time_on = "0000".to_string();
        q.time_off = Some("235959".to_string());
        assert!(q.validate().is_ok());
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

    #[test]
    fn warnings_accepts_valid_refs() {
        let mut q = valid_qso();
        q.iota = Some("EU-001".to_string());
        q.sota_ref = Some("SP/TA-001".to_string());
        q.pota_ref = Some("SP-0001".to_string());
        assert!(q.warnings().is_empty());
    }

    #[test]
    fn warnings_flags_invalid_iota() {
        let mut q = valid_qso();
        q.iota = Some("EU12".to_string());
        let w = q.warnings();
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("IOTA"));
    }

    #[test]
    fn warnings_flags_invalid_sota() {
        let mut q = valid_qso();
        q.sota_ref = Some("SPTA-001".to_string());
        assert_eq!(q.warnings().len(), 1);
    }

    #[test]
    fn warnings_flags_invalid_pota() {
        let mut q = valid_qso();
        q.pota_ref = Some("SP-001".to_string());
        assert_eq!(q.warnings().len(), 1);
    }

    #[test]
    fn warnings_ignore_empty_and_none() {
        let mut q = valid_qso();
        q.iota = Some("   ".to_string());
        q.sota_ref = None;
        q.pota_ref = Some(String::new());
        assert!(q.warnings().is_empty());
    }
}
