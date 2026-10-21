// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;

/// Kategoria statusu QSL używana w drill-down z zakładki QSL statystyk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QslDrillStatus {
    Lotw,
    Eqsl,
    Paper,
    #[default]
    None,
}

/// Filtr „drill-down”: kliknięcie słupka/wiersza w statystykach przenosi do
/// listy QSO spełniających kryterium. Przechowywany na `SpLogApp` i stosowany
/// w tabeli logu (obok wyszukiwarki tekstowej).
#[derive(Clone, Debug, PartialEq)]
pub enum DrillFilter {
    Band(String),
    Mode(String),
    Country(String),
    Month(String), // "YYYY-MM"
    Hour(u32),     // 0..=23
    Qsl(QslDrillStatus),
}

impl DrillFilter {
    /// Czy dany rekord QSO spełnia filtr drill-down.
    pub fn matches(&self, q: &QsoRecord) -> bool {
        match self {
            DrillFilter::Band(b) => q.band.eq_ignore_ascii_case(b),
            DrillFilter::Mode(m) => q.mode.eq_ignore_ascii_case(m),
            DrillFilter::Country(c) => q.country.as_deref().unwrap_or("").eq_ignore_ascii_case(c),
            DrillFilter::Month(m) => {
                // Akceptuje zarówno "YYYY-MM-DD", jak i "YYYYMMDD".
                q.qso_date.replace('-', "").starts_with(&m.replace('-', ""))
            }
            DrillFilter::Hour(h) => {
                // Pierwsze dwie cyfry czasu to godzina (dla "HH:MM:SS" i "HHMMSS").
                let hh = format!("{h:02}");
                q.time_on.chars().take(2).collect::<String>() == hh
            }
            DrillFilter::Qsl(st) => match st {
                QslDrillStatus::Lotw => q.lotw_qsl_rcvd == "Y",
                QslDrillStatus::Eqsl => q.eqsl_qsl_rcvd == "Y",
                QslDrillStatus::Paper => q.qsl_rcvd == "Y",
                QslDrillStatus::None => {
                    q.lotw_qsl_rcvd != "Y" && q.eqsl_qsl_rcvd != "Y" && q.qsl_rcvd != "Y"
                }
            },
        }
    }

    /// Krótka, czytelna etykieta aktywnego filtra (dla paska w tabeli logu).
    pub fn label(&self) -> String {
        match self {
            DrillFilter::Band(b) => format!("📻 {b}"),
            DrillFilter::Mode(m) => format!("📡 {m}"),
            DrillFilter::Country(c) => format!("🌍 {c}"),
            DrillFilter::Month(m) => format!("📅 {m}"),
            DrillFilter::Hour(h) => format!("⏰ {h:02}:00"),
            DrillFilter::Qsl(QslDrillStatus::Lotw) => "📬 LoTW".to_string(),
            DrillFilter::Qsl(QslDrillStatus::Eqsl) => "📬 eQSL".to_string(),
            DrillFilter::Qsl(QslDrillStatus::Paper) => "📬 QSL".to_string(),
            DrillFilter::Qsl(QslDrillStatus::None) => "📬 Brak".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_qso() -> QsoRecord {
        QsoRecord {
            id: Some(1),
            callsign: "SP6INA".to_string(),
            qso_date: "2026-10-21".to_string(),
            time_on: "12:34:56".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            rst_sent: "599".to_string(),
            rst_rcvd: "599".to_string(),
            country: Some("Poland".to_string()),
            lotw_qsl_rcvd: "N".to_string(),
            eqsl_qsl_rcvd: "N".to_string(),
            qsl_rcvd: "N".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_qsl_drill_status_default() {
        assert_eq!(QslDrillStatus::default(), QslDrillStatus::None);
    }

    #[test]
    fn test_drill_filter_matches_band() {
        let q = sample_qso();
        assert!(DrillFilter::Band("20m".to_string()).matches(&q));
        assert!(DrillFilter::Band("20M".to_string()).matches(&q));
        assert!(!DrillFilter::Band("40m".to_string()).matches(&q));
    }

    #[test]
    fn test_drill_filter_matches_mode() {
        let q = sample_qso();
        assert!(DrillFilter::Mode("CW".to_string()).matches(&q));
        assert!(DrillFilter::Mode("cw".to_string()).matches(&q));
        assert!(!DrillFilter::Mode("SSB".to_string()).matches(&q));
    }

    #[test]
    fn test_drill_filter_matches_country() {
        let q = sample_qso();
        assert!(DrillFilter::Country("Poland".to_string()).matches(&q));
        assert!(DrillFilter::Country("POLAND".to_string()).matches(&q));
        assert!(!DrillFilter::Country("Germany".to_string()).matches(&q));
    }

    #[test]
    fn test_drill_filter_matches_month() {
        let q = sample_qso();
        assert!(DrillFilter::Month("2026-10".to_string()).matches(&q));
        assert!(DrillFilter::Month("202610".to_string()).matches(&q));
        assert!(!DrillFilter::Month("2026-09".to_string()).matches(&q));
    }

    #[test]
    fn test_drill_filter_matches_hour() {
        let q = sample_qso();
        assert!(DrillFilter::Hour(12).matches(&q));
        assert!(!DrillFilter::Hour(13).matches(&q));
    }

    #[test]
    fn test_drill_filter_matches_qsl() {
        let mut q = sample_qso();
        assert!(DrillFilter::Qsl(QslDrillStatus::None).matches(&q));
        assert!(!DrillFilter::Qsl(QslDrillStatus::Lotw).matches(&q));

        q.lotw_qsl_rcvd = "Y".to_string();
        assert!(DrillFilter::Qsl(QslDrillStatus::Lotw).matches(&q));
        assert!(!DrillFilter::Qsl(QslDrillStatus::None).matches(&q));

        q.lotw_qsl_rcvd = "N".to_string();
        q.eqsl_qsl_rcvd = "Y".to_string();
        assert!(DrillFilter::Qsl(QslDrillStatus::Eqsl).matches(&q));

        q.eqsl_qsl_rcvd = "N".to_string();
        q.qsl_rcvd = "Y".to_string();
        assert!(DrillFilter::Qsl(QslDrillStatus::Paper).matches(&q));
    }

    #[test]
    fn test_drill_filter_labels() {
        assert_eq!(DrillFilter::Band("20m".to_string()).label(), "📻 20m");
        assert_eq!(DrillFilter::Mode("CW".to_string()).label(), "📡 CW");
        assert_eq!(
            DrillFilter::Country("Poland".to_string()).label(),
            "🌍 Poland"
        );
        assert_eq!(
            DrillFilter::Month("2026-10".to_string()).label(),
            "📅 2026-10"
        );
        assert_eq!(DrillFilter::Hour(9).label(), "⏰ 09:00");
        assert_eq!(DrillFilter::Qsl(QslDrillStatus::Lotw).label(), "📬 LoTW");
        assert_eq!(DrillFilter::Qsl(QslDrillStatus::Eqsl).label(), "📬 eQSL");
        assert_eq!(DrillFilter::Qsl(QslDrillStatus::Paper).label(), "📬 QSL");
        assert_eq!(DrillFilter::Qsl(QslDrillStatus::None).label(), "📬 Brak");
    }
}
