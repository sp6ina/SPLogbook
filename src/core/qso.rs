// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::sync::OnceLock;

use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::core::i18n::{Language, Msg, key};

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

/// Pole czasu podlegające walidacji
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeField {
    On,
    Off,
}

/// Składnik pola czasu podlegający walidacji
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimePart {
    Format,
    Hour,
    Minute,
    Second,
}

/// Błędy walidacji rekordu QSO blokujące zapis
#[derive(Debug, Clone, PartialEq)]
pub enum QsoIssue {
    EmptyCallsign,
    CallsignChars(String),
    EmptyBand,
    EmptyMode,
    DateFormat(String),
    DateInvalid(String),
    Time {
        field: TimeField,
        part: TimePart,
        raw: String,
    },
    FreqTx(f64),
    FreqRx(f64),
    CqZone(u32),
    ItuZone(u32),
}

impl QsoIssue {
    pub fn msg(&self) -> Msg {
        match self {
            Self::EmptyCallsign => Msg::new(key::QSO_ISSUE_EMPTY_CALLSIGN),
            Self::CallsignChars(v) => Msg::new(key::QSO_ISSUE_CALLSIGN_CHARS).arg("value", v),
            Self::EmptyBand => Msg::new(key::QSO_ISSUE_EMPTY_BAND),
            Self::EmptyMode => Msg::new(key::QSO_ISSUE_EMPTY_MODE),
            Self::DateFormat(v) => Msg::new(key::QSO_ISSUE_DATE_FORMAT).arg("value", v),
            Self::DateInvalid(v) => Msg::new(key::QSO_ISSUE_DATE_INVALID).arg("value", v),
            Self::Time { field, part, raw } => {
                let k = match (field, part) {
                    (TimeField::On, TimePart::Format) => key::QSO_ISSUE_TIME_ON_FORMAT,
                    (TimeField::On, TimePart::Hour) => key::QSO_ISSUE_TIME_ON_HOUR,
                    (TimeField::On, TimePart::Minute) => key::QSO_ISSUE_TIME_ON_MINUTE,
                    (TimeField::On, TimePart::Second) => key::QSO_ISSUE_TIME_ON_SECOND,
                    (TimeField::Off, TimePart::Format) => key::QSO_ISSUE_TIME_OFF_FORMAT,
                    (TimeField::Off, TimePart::Hour) => key::QSO_ISSUE_TIME_OFF_HOUR,
                    (TimeField::Off, TimePart::Minute) => key::QSO_ISSUE_TIME_OFF_MINUTE,
                    (TimeField::Off, TimePart::Second) => key::QSO_ISSUE_TIME_OFF_SECOND,
                };
                Msg::new(k).arg("value", raw)
            }
            Self::FreqTx(v) => Msg::new(key::QSO_ISSUE_FREQ_TX).arg("value", v),
            Self::FreqRx(v) => Msg::new(key::QSO_ISSUE_FREQ_RX).arg("value", v),
            Self::CqZone(v) => Msg::new(key::QSO_ISSUE_CQ_ZONE).arg("value", v),
            Self::ItuZone(v) => Msg::new(key::QSO_ISSUE_ITU_ZONE).arg("value", v),
        }
    }
}

impl std::fmt::Display for QsoIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg().text(Language::En))
    }
}

impl std::error::Error for QsoIssue {}

/// Miękkie ostrzeżenia walidacyjne rekordu QSO (nie blokują zapisu)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QsoWarning {
    Iota(String),
    Sota(String),
    Pota(String),
}

impl QsoWarning {
    pub fn msg(&self) -> Msg {
        match self {
            Self::Iota(v) => Msg::new(key::QSO_WARNING_IOTA).arg("value", v),
            Self::Sota(v) => Msg::new(key::QSO_WARNING_SOTA).arg("value", v),
            Self::Pota(v) => Msg::new(key::QSO_WARNING_POTA).arg("value", v),
        }
    }
}

impl std::fmt::Display for QsoWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg().text(Language::En))
    }
}

/// Waliduje pojedyncze pole czasu w formacie HHMM lub HHMMSS.
/// Sprawdza format (długość 4/6, same cyfry) oraz zakresy: godzina 00–23,
/// minuty 00–59, sekundy 00–59 (tylko dla HHMMSS).
fn validate_time_field(raw: &str, field: TimeField) -> Result<(), QsoIssue> {
    let normalized = raw.trim().replace(':', "");
    let valid_len = normalized.len() == 4 || normalized.len() == 6;
    let valid_digits = normalized.chars().all(|c| c.is_ascii_digit());
    if !valid_len || !valid_digits {
        return Err(QsoIssue::Time {
            field,
            part: TimePart::Format,
            raw: raw.to_string(),
        });
    }

    let hh: u32 = normalized[0..2].parse().unwrap();
    let mm: u32 = normalized[2..4].parse().unwrap();
    if hh > 23 {
        return Err(QsoIssue::Time {
            field,
            part: TimePart::Hour,
            raw: raw.to_string(),
        });
    }
    if mm > 59 {
        return Err(QsoIssue::Time {
            field,
            part: TimePart::Minute,
            raw: raw.to_string(),
        });
    }
    if normalized.len() == 6 {
        let ss: u32 = normalized[4..6].parse().unwrap();
        if ss > 59 {
            return Err(QsoIssue::Time {
                field,
                part: TimePart::Second,
                raw: raw.to_string(),
            });
        }
    }
    Ok(())
}

/// Zwraca ostrzeżenie, jeśli niepusty napis nie pasuje do formatu IOTA (XX-NNN,
/// np. EU-001). Walidacja miękka — nie blokuje zapisu.
fn validate_iota_format(value: &str) -> Option<QsoWarning> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z]{2}-\d{3}$").unwrap());
    let trimmed = value.trim();
    if trimmed.is_empty() || re.is_match(trimmed) {
        None
    } else {
        Some(QsoWarning::Iota(value.to_string()))
    }
}

/// Zwraca ostrzeżenie, jeśli niepusty napis nie pasuje do formatu SOTA
/// (XX/YY-NNN, np. SP/TA-001). Walidacja miękka — nie blokuje zapisu.
fn validate_sota_format(value: &str) -> Option<QsoWarning> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9]{1,3}/[A-Za-z0-9]{2,3}-\d{3}$").unwrap());
    let trimmed = value.trim();
    if trimmed.is_empty() || re.is_match(trimmed) {
        None
    } else {
        Some(QsoWarning::Sota(value.to_string()))
    }
}

/// Zwraca ostrzeżenie, jeśli niepusty napis nie pasuje do formatu POTA
/// (XX-NNNN, np. SP-0001). Walidacja miękka — nie blokuje zapisu.
fn validate_pota_format(value: &str) -> Option<QsoWarning> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9]{1,4}-\d{4}$").unwrap());
    let trimmed = value.trim();
    if trimmed.is_empty() || re.is_match(trimmed) {
        None
    } else {
        Some(QsoWarning::Pota(value.to_string()))
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

    /// Zwraca znormalizowaną tożsamość QSO (`QsoIdentity`).
    pub fn identity(&self) -> QsoIdentity {
        QsoIdentity::from_fields(
            &self.callsign,
            &self.band,
            &self.mode,
            &self.qso_date,
            &self.time_on,
        )
    }
}

/// Klucz jednoznaczności łączności (znormalizowana tożsamość QSO)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QsoIdentity {
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub date: String,
    pub minute: String,
}

impl QsoIdentity {
    pub fn from_fields(callsign: &str, band: &str, mode: &str, date: &str, time: &str) -> Self {
        let callsign = callsign.trim().to_uppercase();
        let band = band.trim().to_uppercase();
        let mode = mode.trim().to_uppercase();

        let date_digits: String = date.chars().filter(char::is_ascii_digit).collect();
        let date = if date_digits.len() >= 8 {
            date_digits[..8].to_string()
        } else {
            format!("?{}", date.trim())
        };

        let time_digits: String = time.chars().filter(char::is_ascii_digit).collect();
        let minute = if time_digits.len() >= 4 {
            time_digits[..4].to_string()
        } else {
            format!("?{}", time.trim())
        };

        Self {
            callsign,
            band,
            mode,
            date,
            minute,
        }
    }
}

impl std::fmt::Display for QsoIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}|{}|{}|{}|{}",
            self.callsign, self.band, self.mode, self.date, self.minute
        )
    }
}

impl QsoRecord {
    /// Centralna walidacja domenowa rekordu QSO. Zwraca `Ok(())` gdy rekord
    /// nadaje się do zapisania, lub `Err` z opisem pierwszego wykrytego błędu.
    /// Wywoływana przy zapisie z formularza, edycji i imporcie, aby uniemożliwić
    /// zapis błędnej daty, czasu, pasma, częstotliwości i stref.
    pub fn validate(&self) -> Result<(), QsoIssue> {
        let callsign = self.callsign.trim();
        if callsign.is_empty() {
            return Err(QsoIssue::EmptyCallsign);
        }
        if !callsign
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '/')
        {
            return Err(QsoIssue::CallsignChars(callsign.to_string()));
        }

        if self.band.trim().is_empty() {
            return Err(QsoIssue::EmptyBand);
        }
        if self.mode.trim().is_empty() {
            return Err(QsoIssue::EmptyMode);
        }

        let date = self.qso_date.trim().replace(['-', '.', '/'], "");
        if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
            return Err(QsoIssue::DateFormat(self.qso_date.clone()));
        }
        if chrono::NaiveDate::parse_from_str(&date, "%Y%m%d").is_err() {
            return Err(QsoIssue::DateInvalid(self.qso_date.clone()));
        }

        validate_time_field(&self.time_on, TimeField::On)?;

        if let Some(ref toff) = self.time_off {
            validate_time_field(toff, TimeField::Off)?;
        }

        if let Some(f) = self.freq {
            if !f.is_finite() || f <= 0.0 {
                return Err(QsoIssue::FreqTx(f));
            }
        }
        if let Some(f) = self.freq_rx {
            if !f.is_finite() || f < 0.0 {
                return Err(QsoIssue::FreqRx(f));
            }
        }

        if let Some(cqz) = self.cqz {
            if !(1..=40).contains(&cqz) {
                return Err(QsoIssue::CqZone(cqz));
            }
        }
        if let Some(ituz) = self.ituz {
            if !(1..=90).contains(&ituz) {
                return Err(QsoIssue::ItuZone(ituz));
            }
        }

        Ok(())
    }

    /// Zwraca miękkie ostrzeżenia walidacyjne (nie blokują zapisu). Służą do
    /// sygnalizowania wartości o podejrzanym formacie (IOTA/SOTA/POTA), które
    /// warto skorygować, ale które nie powinny uniemożliwiać zapisu rekordu.
    pub fn warnings(&self) -> Vec<QsoWarning> {
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
        assert_eq!(q.validate(), Err(QsoIssue::EmptyCallsign));
    }

    #[test]
    fn validate_rejects_bad_date() {
        let mut q = valid_qso();
        q.qso_date = "20261340".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::DateInvalid("20261340".to_string()))
        );
    }

    #[test]
    fn validate_rejects_bad_time() {
        let mut q = valid_qso();
        q.time_on = "12:3".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Format,
                raw: "12:3".to_string(),
            })
        );
    }

    #[test]
    fn validate_rejects_hour_out_of_range() {
        let mut q = valid_qso();
        q.time_on = "2460".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Hour,
                raw: "2460".to_string(),
            })
        );
    }

    #[test]
    fn validate_rejects_minute_out_of_range() {
        let mut q = valid_qso();
        q.time_on = "1860".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Minute,
                raw: "1860".to_string(),
            })
        );
    }

    #[test]
    fn validate_rejects_second_out_of_range() {
        let mut q = valid_qso();
        q.time_on = "183060".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Second,
                raw: "183060".to_string(),
            })
        );
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
        assert!(matches!(q.validate(), Err(QsoIssue::FreqTx(f)) if f.is_nan()));
    }

    #[test]
    fn validate_rejects_out_of_range_zone() {
        let mut q = valid_qso();
        q.cqz = Some(99);
        assert_eq!(q.validate(), Err(QsoIssue::CqZone(99)));
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
        assert_eq!(w, vec![QsoWarning::Iota("EU12".to_string())]);
        assert!(w[0].to_string().contains("IOTA"));
    }

    #[test]
    fn warnings_flags_invalid_sota() {
        let mut q = valid_qso();
        q.sota_ref = Some("SPTA-001".to_string());
        assert_eq!(q.warnings(), vec![QsoWarning::Sota("SPTA-001".to_string())]);
    }

    #[test]
    fn warnings_flags_invalid_pota() {
        let mut q = valid_qso();
        q.pota_ref = Some("SP-001".to_string());
        assert_eq!(q.warnings(), vec![QsoWarning::Pota("SP-001".to_string())]);
    }

    #[test]
    fn warnings_ignore_empty_and_none() {
        let mut q = valid_qso();
        q.iota = Some("   ".to_string());
        q.sota_ref = None;
        q.pota_ref = Some(String::new());
        assert!(q.warnings().is_empty());
    }

    #[test]
    fn validate_reports_empty_callsign() {
        let mut q = valid_qso();
        q.callsign = String::new();
        assert_eq!(q.validate(), Err(QsoIssue::EmptyCallsign));
    }

    #[test]
    fn validate_reports_callsign_chars() {
        let mut q = valid_qso();
        q.callsign = "SP6INA@".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::CallsignChars("SP6INA@".to_string()))
        );
    }

    #[test]
    fn validate_reports_empty_band() {
        let mut q = valid_qso();
        q.band = "   ".to_string();
        assert_eq!(q.validate(), Err(QsoIssue::EmptyBand));
    }

    #[test]
    fn validate_reports_empty_mode() {
        let mut q = valid_qso();
        q.mode = String::new();
        assert_eq!(q.validate(), Err(QsoIssue::EmptyMode));
    }

    #[test]
    fn validate_reports_date_format() {
        let mut q = valid_qso();
        q.qso_date = "2026-09".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::DateFormat("2026-09".to_string()))
        );
    }

    #[test]
    fn validate_reports_date_invalid() {
        let mut q = valid_qso();
        q.qso_date = "20260230".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::DateInvalid("20260230".to_string()))
        );
    }

    #[test]
    fn validate_reports_time_on_format() {
        let mut q = valid_qso();
        q.time_on = "12:3".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Format,
                raw: "12:3".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_time_on_hour() {
        let mut q = valid_qso();
        q.time_on = "2400".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Hour,
                raw: "2400".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_time_on_minute() {
        let mut q = valid_qso();
        q.time_on = "1260".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Minute,
                raw: "1260".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_time_on_second() {
        let mut q = valid_qso();
        q.time_on = "123060".to_string();
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Second,
                raw: "123060".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_time_off_format() {
        let mut q = valid_qso();
        q.time_off = Some("99".to_string());
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Format,
                raw: "99".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_time_off_hour() {
        let mut q = valid_qso();
        q.time_off = Some("2500".to_string());
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Hour,
                raw: "2500".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_time_off_minute() {
        let mut q = valid_qso();
        q.time_off = Some("1265".to_string());
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Minute,
                raw: "1265".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_time_off_second() {
        let mut q = valid_qso();
        q.time_off = Some("123075".to_string());
        assert_eq!(
            q.validate(),
            Err(QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Second,
                raw: "123075".to_string(),
            })
        );
    }

    #[test]
    fn validate_reports_freq_tx() {
        let mut q = valid_qso();
        q.freq = Some(0.0);
        assert_eq!(q.validate(), Err(QsoIssue::FreqTx(0.0)));
    }

    #[test]
    fn validate_reports_freq_rx() {
        let mut q = valid_qso();
        q.freq_rx = Some(-1.0);
        assert_eq!(q.validate(), Err(QsoIssue::FreqRx(-1.0)));
    }

    #[test]
    fn validate_reports_cq_zone() {
        let mut q = valid_qso();
        q.cqz = Some(41);
        assert_eq!(q.validate(), Err(QsoIssue::CqZone(41)));
    }

    #[test]
    fn validate_reports_itu_zone() {
        let mut q = valid_qso();
        q.ituz = Some(91);
        assert_eq!(q.validate(), Err(QsoIssue::ItuZone(91)));
    }

    #[test]
    fn issue_messages_have_no_unsubstituted_placeholders() {
        let sample_issues = [
            QsoIssue::EmptyCallsign,
            QsoIssue::CallsignChars("SP6INA/P".to_string()),
            QsoIssue::EmptyBand,
            QsoIssue::EmptyMode,
            QsoIssue::DateFormat("2026-09".to_string()),
            QsoIssue::DateInvalid("20260230".to_string()),
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Format,
                raw: "12:3".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Hour,
                raw: "2500".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Minute,
                raw: "1260".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Second,
                raw: "123060".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Format,
                raw: "99".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Hour,
                raw: "2500".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Minute,
                raw: "1260".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Second,
                raw: "123060".to_string(),
            },
            QsoIssue::FreqTx(0.0),
            QsoIssue::FreqRx(-1.0),
            QsoIssue::CqZone(45),
            QsoIssue::ItuZone(95),
        ];

        let sample_warnings = [
            QsoWarning::Iota("EU-9999".to_string()),
            QsoWarning::Sota("SP/TA-9999".to_string()),
            QsoWarning::Pota("SP-9999".to_string()),
        ];

        for lang in Language::ALL {
            for issue in &sample_issues {
                let text = issue.msg().text(lang);
                assert!(
                    !text.contains('{') && !text.contains('}'),
                    "Unsubstituted placeholder in issue {issue:?} for language {lang:?}: '{text}'"
                );
            }
            for warning in &sample_warnings {
                let text = warning.msg().text(lang);
                assert!(
                    !text.contains('{') && !text.contains('}'),
                    "Unsubstituted placeholder in warning {warning:?} for language {lang:?}: '{text}'"
                );
            }
        }
    }

    #[test]
    fn qso_issue_display_is_english() {
        let sample_issues = [
            QsoIssue::EmptyCallsign,
            QsoIssue::CallsignChars("SP6INA/P".to_string()),
            QsoIssue::EmptyBand,
            QsoIssue::EmptyMode,
            QsoIssue::DateFormat("2026-09".to_string()),
            QsoIssue::DateInvalid("20260230".to_string()),
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Format,
                raw: "12:3".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Hour,
                raw: "2500".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Minute,
                raw: "1260".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::On,
                part: TimePart::Second,
                raw: "123060".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Format,
                raw: "99".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Hour,
                raw: "2500".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Minute,
                raw: "1260".to_string(),
            },
            QsoIssue::Time {
                field: TimeField::Off,
                part: TimePart::Second,
                raw: "123060".to_string(),
            },
            QsoIssue::FreqTx(0.0),
            QsoIssue::FreqRx(-1.0),
            QsoIssue::CqZone(45),
            QsoIssue::ItuZone(95),
        ];

        for issue in &sample_issues {
            let display_text = issue.to_string();
            assert_eq!(display_text, issue.msg().text(Language::En));
            assert!(
                !display_text.contains('{') && !display_text.contains('}'),
                "Display for {issue:?} contains unformatted placeholder: '{display_text}'"
            );
        }

        let sample_warnings = [
            QsoWarning::Iota("EU-9999".to_string()),
            QsoWarning::Sota("SP/TA-9999".to_string()),
            QsoWarning::Pota("SP-9999".to_string()),
        ];
        for warning in &sample_warnings {
            let display_text = warning.to_string();
            assert_eq!(display_text, warning.msg().text(Language::En));
            assert!(
                !display_text.contains('{') && !display_text.contains('}'),
                "Display for {warning:?} contains unformatted placeholder: '{display_text}'"
            );
        }
    }

    #[test]
    fn qso_issue_implements_error() {
        let issue = QsoIssue::EmptyCallsign;
        let err: &dyn std::error::Error = &issue;
        assert!(err.source().is_none());
        let boxed: Box<dyn std::error::Error> = Box::new(issue);
        assert_eq!(boxed.to_string(), "callsign cannot be empty");
    }
}
