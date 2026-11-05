// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Generator plików w formacie Cabrillo v3.0 dla zawodów krótkofalarskich.

use crate::core::contest_rules::ContestError;
use crate::core::qso::QsoRecord;
use std::fmt::Write;

/// Kategoria nadajnika dla zawodów krótkofalarskich w formacie Cabrillo 3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CabrilloTransmitterCategory {
    One,
    Two,
    Multi,
    Unlimited,
}

impl CabrilloTransmitterCategory {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::One => "ONE",
            Self::Two => "TWO",
            Self::Multi => "MULTI",
            Self::Unlimited => "UNLIMITED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim();
        if t.eq_ignore_ascii_case("ONE") || t.eq_ignore_ascii_case("SINGLE") {
            Some(Self::One)
        } else if t.eq_ignore_ascii_case("TWO") || t.eq_ignore_ascii_case("MULTI-TWO") {
            Some(Self::Two)
        } else if t.eq_ignore_ascii_case("MULTI") || t.eq_ignore_ascii_case("MULTI-MULTI") {
            Some(Self::Multi)
        } else if t.eq_ignore_ascii_case("UNLIMITED") {
            Some(Self::Unlimited)
        } else {
            None
        }
    }
}

/// Waliduje kategorię nadajnika dla aktualnego wydania SPLogbook.
/// SPLogbook w wersji 1.x wspiera wyłącznie kategorię Single-Op / One (jeden nadajnik).
/// Kategorie wielonadajnikowe (Multi-Two, Multi-Unlimited) wymagają modelu wielu nadajników (Etap 8)
/// i są jawnie odrzucane z błędem `ContestError::UnsupportedContestCategory`.
pub fn validate_transmitter_category(cat: CabrilloTransmitterCategory) -> Result<(), ContestError> {
    match cat {
        CabrilloTransmitterCategory::One => Ok(()),
        CabrilloTransmitterCategory::Two => Err(ContestError::UnsupportedContestCategory(
            "Multi-Two category requires independent transmitter/band sequences and is not supported in current release".to_string(),
        )),
        CabrilloTransmitterCategory::Multi => Err(ContestError::UnsupportedContestCategory(
            "Multi-Multi category requires multi-transmitter tracking and is not supported in current release".to_string(),
        )),
        CabrilloTransmitterCategory::Unlimited => Err(ContestError::UnsupportedContestCategory(
            "Multi-Unlimited category requires multi-transmitter tracking and is not supported in current release".to_string(),
        )),
    }
}

/// Sprawdza, czy kolumna identyfikatora nadajnika (transmitter ID) powinna być emitowana w wierszu Cabrillo.
/// W kategorii ONE (Single-Op / Multi-One) kolumna nadajnika NIE jest emitowana.
pub const fn cabrillo_transmitter_id_is_emitted(cat: CabrilloTransmitterCategory) -> bool {
    match cat {
        CabrilloTransmitterCategory::One => false,
        CabrilloTransmitterCategory::Two
        | CabrilloTransmitterCategory::Multi
        | CabrilloTransmitterCategory::Unlimited => true,
    }
}

/// Sprawdza politykę numeracji seryjnej dla kategorii zawodów CQ WPX.
/// - Single-Op / One: jedna globalna sekwencja numerów rosnących od 001.
/// - Multi-One: jedna sekwencja dla całej stacji (zgodnie z regulaminem CQ WPX).
/// - Multi-Two: wymaga niezależnych sekwencji per pasmo oraz identyfikatora nadajnika.
pub fn validate_cqwpx_serial_policy(
    cat: CabrilloTransmitterCategory,
    qsos: &[QsoRecord],
) -> Result<(), ContestError> {
    validate_transmitter_category(cat)?;

    let mut prev_serial: Option<u32> = None;
    for qso in qsos {
        let serial_val = if let Some(ref s) = qso.stx_string {
            s.trim().parse::<u32>().ok()
        } else {
            qso.stx
        };

        if let Some(ser) = serial_val {
            if let Some(prev) = prev_serial {
                if ser <= prev {
                    return Err(ContestError::InvalidSerialNumber(format!(
                        "Non-monotonic serial sequence: previous {prev}, current {ser}"
                    )));
                }
            }
            prev_serial = Some(ser);
        }
    }
    Ok(())
}

/// Formatuje i waliduje raport RST dla logu Cabrillo bez fałszowania wartości użytkownika.
/// - Jeśli pole RST jest puste, stosowany jest domyślny raport (599 dla CW/RY/DG, 59 dla PH/FM).
/// - Jeśli pole zawiera wartość użytkownika (np. 57, 579, 559, 51), jest ona zachowywana.
/// - Jeśli wartość zawiera niedozwolone znaki (nie-cyfry) lub nieprawidłową długość, zwracany jest błąd.
pub fn format_and_validate_rst(raw: &str, cab_mode: &str) -> Result<String, ContestError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(match cab_mode {
            "CW" | "RY" | "DG" => "599".to_string(),
            _ => "59".to_string(),
        });
    }

    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err(ContestError::InvalidContestExchange {
            contest: "Cabrillo",
            detail: format!("Malformed RST '{trimmed}': contains non-digit characters"),
        });
    }

    match cab_mode {
        "CW" | "RY" | "DG" => {
            if trimmed.len() < 2 || trimmed.len() > 3 {
                return Err(ContestError::InvalidContestExchange {
                    contest: "Cabrillo",
                    detail: format!(
                        "Malformed RST '{trimmed}': expected 2 or 3 digits for {cab_mode}"
                    ),
                });
            }
        }
        _ => {
            if trimmed.len() != 2 {
                return Err(ContestError::InvalidContestExchange {
                    contest: "Cabrillo",
                    detail: format!("Malformed RS '{trimmed}': expected 2 digits for {cab_mode}"),
                });
            }
        }
    }

    Ok(trimmed.to_string())
}

/// Ustala wymianę nadawaną (STX) z zachowaniem ścisłego priorytetu:
/// 1. `stx_string` (jeśli niepusty po trim);
/// 2. `stx` (np. `{stx:03}`);
/// 3. `fallback_index` (np. `format!("{fallback_index:03}")`).
pub fn resolve_sent_exchange(qso: &QsoRecord, fallback_index: usize) -> String {
    if let Some(ref s) = qso.stx_string {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(stx) = qso.stx {
        return format!("{stx:03}");
    }
    format!("{fallback_index:03}")
}

/// Ustala wymianę odebraną (SRX) z zachowaniem priorytetu:
/// 1. `srx_string` (jeśli niepusty po trim);
/// 2. `srx` (np. `{srx:03}`);
/// 3. fallback (domyślnie "001").
pub fn resolve_rcvd_exchange(qso: &QsoRecord, default_fallback: &str) -> String {
    if let Some(ref s) = qso.srx_string {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(srx) = qso.srx {
        return format!("{srx:03}");
    }
    default_fallback.to_string()
}

/// Dane wejściowe do wygenerowania raportu zawodów w formacie Cabrillo 3.0.
#[derive(Debug, Clone)]
pub struct CabrilloInput<'a> {
    pub contest_name: &'a str,
    pub callsign: &'a str,
    pub operator: &'a str,
    pub city: &'a str,
    pub country: &'a str,
    pub claimed_score: u32,
    pub qsos: &'a [QsoRecord],
    pub transmitter_category: Option<CabrilloTransmitterCategory>,
}

/// Mapuje emisję QSO na kanoniczny kod Cabrillo 3.0: `CW`, `PH`, `FM`, `RY`, `DG`.
pub fn cabrillo_mode(mode: &str) -> &'static str {
    let m = mode.trim().to_ascii_uppercase();
    match m.as_str() {
        "CW" | "CW-R" => "CW",
        "SSB" | "USB" | "LSB" | "AM" => "PH",
        "FM" => "FM",
        "RTTY" | "RY" => "RY",
        "FT8" | "FT4" | "PSK" | "PSK31" | "PSK63" | "JT65" | "JT9" | "MFSK" | "JS8" | "DIGI" => {
            "DG"
        }
        _ => {
            if m.contains("SSB") || m.contains("PHONE") {
                "PH"
            } else if m.contains("RTTY") {
                "RY"
            } else if m.contains("CW") {
                "CW"
            } else if m.contains("FM") {
                "FM"
            } else {
                "PH"
            }
        }
    }
}

/// Zwraca domyślną częstotliwość w kHz (lub MHz dla pasm VHF/UHF) dla danego pasma w Cabrillo.
pub fn band_to_cabrillo_freq(band: &str) -> u64 {
    let b = band.trim().to_ascii_lowercase();
    match b.as_str() {
        "160m" => 1800,
        "80m" => 3500,
        "60m" => 5350,
        "40m" => 7000,
        "30m" => 10100,
        "20m" => 14000,
        "17m" => 18068,
        "15m" => 21000,
        "12m" => 24890,
        "10m" => 28000,
        "6m" => 50,
        "4m" => 70,
        "2m" => 144,
        "1.25m" => 222,
        "70cm" => 432,
        "33cm" => 902,
        "23cm" => 1296,
        "13cm" => 2300,
        _ => 14000,
    }
}

/// Czysta funkcja generująca tekst raportu Cabrillo 3.0 z pełną walidacją kategorii i raportów RST.
pub fn build(input: &CabrilloInput) -> Result<String, ContestError> {
    let tx_category = input
        .transmitter_category
        .unwrap_or(CabrilloTransmitterCategory::One);
    validate_transmitter_category(tx_category)?;

    let mut out = String::new();
    out.push_str("START-OF-LOG: 3.0\n");
    let _ = writeln!(out, "CONTEST: {}", input.contest_name);
    let _ = writeln!(out, "CALLSIGN: {}", input.callsign);
    out.push_str("CATEGORY-OPERATOR: SINGLE-OP\n");
    let _ = writeln!(out, "CATEGORY-TRANSMITTER: {}", tx_category.as_str());
    out.push_str("CATEGORY-POWER: HIGH\n");
    out.push_str("CATEGORY-BAND: ALL\n");
    out.push_str("CATEGORY-MODE: MIXED\n");
    out.push_str("CATEGORY-STATION: FIXED\n");
    let _ = writeln!(out, "CLAIMED-SCORE: {}", input.claimed_score);
    let _ = writeln!(out, "OPERATORS: {}", input.callsign);
    let _ = writeln!(out, "NAME: {}", input.operator);
    let _ = writeln!(out, "ADDRESS: {}, {}", input.city, input.country);
    out.push_str("CREATED-BY: SPLogbook by SP6INA (GPLv3)\n");
    out.push_str("SOAPBOX: Created with SPLogbook by SP6INA (GPLv3)\n");

    // Kopiujemy i sortujemy łączności chronologicznie rosnąco (DATA-001 / FUNC-001)
    let mut sorted_qsos = input.qsos.to_vec();
    sorted_qsos.sort_by(|a, b| {
        a.qso_date
            .cmp(&b.qso_date)
            .then_with(|| a.time_on.cmp(&b.time_on))
            .then_with(|| a.id.cmp(&b.id))
    });

    for (idx, qso) in sorted_qsos.iter().enumerate() {
        let freq_khz = qso.freq.map_or_else(
            || band_to_cabrillo_freq(&qso.band),
            |f| (f * 1000.0).round() as u64,
        );

        let date_str = if qso.qso_date.len() == 8 && qso.qso_date.is_ascii() {
            format!(
                "{}-{}-{}",
                &qso.qso_date[0..4],
                &qso.qso_date[4..6],
                &qso.qso_date[6..8]
            )
        } else {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        };

        let time_str = if qso.time_on.len() >= 4 && qso.time_on.is_ascii() {
            qso.time_on[0..4].to_string()
        } else {
            chrono::Utc::now().format("%H%M").to_string()
        };

        let mode_cab = cabrillo_mode(&qso.mode);
        let my_rst = format_and_validate_rst(&qso.rst_sent, mode_cab)?;
        let his_rst = format_and_validate_rst(&qso.rst_rcvd, mode_cab)?;
        let my_exch = resolve_sent_exchange(qso, idx + 1);
        let his_exch = resolve_rcvd_exchange(qso, "001");

        let _ = writeln!(
            out,
            "QSO: {:5} {:2} {} {} {:10} {:3} {:4} {:10} {:3} {}",
            freq_khz,
            mode_cab,
            date_str,
            time_str,
            input.callsign,
            my_rst,
            my_exch,
            qso.callsign,
            his_rst,
            his_exch
        );
    }
    out.push_str("END-OF-LOG:\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::contest_rules::{score_cqww, score_sp_dx};
    use crate::core::contest_stats::compute_scoreboard;

    fn sample_qso(call: &str, band: &str, mode: &str, rst_sent: &str, rst_rcvd: &str) -> QsoRecord {
        let mut q = QsoRecord::new(call, band, mode);
        q.freq = None;
        q.rst_sent = rst_sent.to_string();
        q.rst_rcvd = rst_rcvd.to_string();
        q.qso_date = "20260404".to_string();
        q.time_on = "120000".to_string();
        q.dxcc = Some(269);
        q.cqz = Some(15);
        q
    }

    #[test]
    fn cabrillo_claimed_score_equals_scoreboard_total() {
        let qsos = vec![
            sample_qso("SP1ABC", "20m", "CW", "599", "599"),
            sample_qso("DL1XYZ", "20m", "CW", "599", "599"),
        ];
        let sb = compute_scoreboard("SP DX Contest", &[], &qsos, 269, 15);
        let input = CabrilloInput {
            contest_name: "SP DX Contest",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: sb.total,
            qsos: &qsos,
            transmitter_category: None,
        };

        let cabrillo = build(&input).expect("valid cabrillo");
        let expected_line = format!("CLAIMED-SCORE: {}\n", sb.total);
        assert!(
            cabrillo.contains(&expected_line),
            "Cabrillo must contain claimed score matching scoreboard total: {expected_line}"
        );
    }

    #[test]
    fn cabrillo_header_and_qso_lines_format() {
        let mut qso1 = sample_qso("DL1XYZ", "20m", "CW", "599", "599");
        qso1.freq = Some(14.025);
        qso1.srx = Some(42);

        let qsos = vec![qso1];
        let input = CabrilloInput {
            contest_name: "CQ WW DX",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Opole",
            country: "Poland",
            claimed_score: 1234,
            qsos: &qsos,
            transmitter_category: Some(CabrilloTransmitterCategory::One),
        };

        let text = build(&input).expect("valid cabrillo");

        // Nagłówek
        assert!(text.starts_with("START-OF-LOG: 3.0\n"));
        assert!(text.contains("CONTEST: CQ WW DX\n"));
        assert!(text.contains("CALLSIGN: SP6INA\n"));
        assert!(text.contains("CATEGORY-OPERATOR: SINGLE-OP\n"));
        assert!(text.contains("CATEGORY-TRANSMITTER: ONE\n"));
        assert!(text.contains("CLAIMED-SCORE: 1234\n"));
        assert!(text.contains("OPERATORS: SP6INA\n"));
        assert!(text.contains("NAME: Mariusz\n"));
        assert!(text.contains("ADDRESS: Opole, Poland\n"));
        assert!(text.contains("CREATED-BY: SPLogbook by SP6INA (GPLv3)\n"));
        assert!(text.contains("SOAPBOX: Created with SPLogbook by SP6INA (GPLv3)\n"));

        // Linia QSO: 14025 CW 2026-04-04 1200 SP6INA     599 001  DL1XYZ     599 042
        assert!(
            text.contains("QSO: 14025 CW 2026-04-04 1200 SP6INA     599 001  DL1XYZ     599 042\n"),
            "Generated text did not contain expected QSO line:\n{text}"
        );

        // Zakończenie
        assert!(text.ends_with("END-OF-LOG:\n"));
    }

    #[test]
    fn cabrillo_chronological_sorting_and_serial_assignment() {
        let mut q1 = sample_qso("DL1AAA", "20m", "CW", "599", "599");
        q1.qso_date = "20260404".to_string();
        q1.time_on = "120500".to_string();

        let mut q2 = sample_qso("DL2BBB", "40m", "SSB", "59", "59");
        q2.qso_date = "20260404".to_string();
        q2.time_on = "120000".to_string(); // Wcześniejsza niż q1!

        let mut q3 = sample_qso("DL3CCC", "15m", "FT8", "599", "599");
        q3.qso_date = "20260404".to_string();
        q3.time_on = "121000".to_string();

        // Przekazujemy w losowej kolejności [q3, q1, q2]
        let qsos = vec![q3, q1, q2];
        let input = CabrilloInput {
            contest_name: "SP DX Contest",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 100,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("QSO:")).collect();
        assert_eq!(lines.len(), 3);

        // Pierwsza linia powinna być q2 (12:00) z serialem 001 i emisją PH
        assert!(
            lines[0].contains("DL2BBB"),
            "Expected DL2BBB in lines[0], got: '{}'",
            lines[0]
        );
        assert!(
            lines[0].contains("7000 PH 2026-04-04 1200"),
            "Expected 7000 PH 2026-04-04 1200 in lines[0], got: '{}'",
            lines[0]
        );
        // Druga linia powinna być q1 (12:05) z serialem 002 i emisją CW
        assert!(lines[1].contains("14000 CW 2026-04-04 1205 SP6INA"));
        assert!(lines[1].contains("DL1AAA"));
        assert!(lines[1].contains("002"));
        // Trzecia linia powinna być q3 (12:10) z serialem 003 i emisją DG
        assert!(lines[2].contains("21000 DG 2026-04-04 1210 SP6INA"));
        assert!(lines[2].contains("DL3CCC"));
        assert!(lines[2].contains("003"));
    }

    #[test]
    fn cabrillo_export_preserves_rst_digital_modes_and_custom_contest_exchanges() {
        let mut qso_sp_dx = sample_qso("SP6INA", "80m", "CW", "579", "589");
        qso_sp_dx.stx_string = Some("D".to_string()); // Dolnośląskie
        qso_sp_dx.srx_string = Some("001".to_string());

        let mut qso_cqww = sample_qso("W1AW", "20m", "RTTY", "599", "599");
        qso_cqww.stx_string = Some("15".to_string()); // Strefa 15
        qso_cqww.srx_string = Some("05".to_string()); // Strefa 5

        let mut qso_vhf = sample_qso("OK1KPA", "2m", "FM", "59", "59");
        qso_vhf.freq = Some(144.200);
        qso_vhf.stx_string = Some("JO80".to_string());
        qso_vhf.srx_string = Some("JN79".to_string());

        let qsos = vec![qso_sp_dx, qso_cqww, qso_vhf];
        let input = CabrilloInput {
            contest_name: "Mixed Contest",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 500,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("QSO:")).collect();
        assert_eq!(lines.len(), 3);

        // 1. SP DX: CW, rst 579 / 589, wymiana "D" i "001"
        assert!(
            lines[0].contains("3500 CW 2026-04-04 1200 SP6INA")
                && lines[0].contains("579")
                && lines[0].contains("D")
                && lines[0].contains("589")
                && lines[0].contains("001"),
            "lines[0] unexpected: '{}'",
            lines[0]
        );
        // 2. CQ WW: RTTY zmapowane na "RY", rst 599 / 599, strefy 15 i 05
        assert!(
            lines[1].contains("14000 RY 2026-04-04 1200 SP6INA")
                && lines[1].contains("15")
                && lines[1].contains("W1AW")
                && lines[1].contains("05"),
            "lines[1] unexpected: '{}'",
            lines[1]
        );
        // 3. VHF: 2m zmapowane na "144200 FM", rst 59 / 59, lokatory JO80 i JN79
        assert!(
            lines[2].contains("144200 FM 2026-04-04 1200 SP6INA")
                && lines[2].contains("JO80")
                && lines[2].contains("OK1KPA")
                && lines[2].contains("JN79"),
            "lines[2] unexpected: '{}'",
            lines[2]
        );
    }

    #[test]
    fn cqwpx_single_transmitter_uses_single_serial_sequence() {
        let mut q1 = sample_qso("W1AW", "20m", "CW", "599", "599");
        q1.stx = Some(1);
        let mut q2 = sample_qso("DL1XYZ", "40m", "CW", "599", "599");
        q2.stx = Some(2);
        let mut q3 = sample_qso("JA1ABC", "15m", "CW", "599", "599");
        q3.stx = Some(3);

        let qsos = vec![q1, q2, q3];
        assert!(validate_cqwpx_serial_policy(CabrilloTransmitterCategory::One, &qsos).is_ok());

        // Nie-monotoniczna sekwencja zwraca błąd InvalidSerialNumber
        let mut q_first = sample_qso("W1AW", "20m", "CW", "599", "599");
        q_first.stx = Some(2);
        let mut q_second = sample_qso("DL1XYZ", "20m", "CW", "599", "599");
        q_second.stx = Some(1);
        let res =
            validate_cqwpx_serial_policy(CabrilloTransmitterCategory::One, &[q_first, q_second]);
        assert!(matches!(res, Err(ContestError::InvalidSerialNumber(_))));
    }

    #[test]
    fn cqwpx_multi_one_uses_contract_serial_sequence() {
        // Multi-One stosuje jedną wspólną sekwencję numerów seryjnych
        let mut q1 = sample_qso("W1AW", "20m", "CW", "599", "599");
        q1.stx_string = Some("001".to_string());
        let mut q2 = sample_qso("DL1XYZ", "80m", "SSB", "59", "59");
        q2.stx_string = Some("002".to_string());

        let res = validate_cqwpx_serial_policy(CabrilloTransmitterCategory::One, &[q1, q2]);
        assert!(res.is_ok());
    }

    #[test]
    fn cqwpx_multi_two_requires_transmitter_and_band_sequences() {
        let q1 = sample_qso("W1AW", "20m", "CW", "599", "599");
        let res = validate_cqwpx_serial_policy(CabrilloTransmitterCategory::Two, &[q1]);
        match res {
            Err(ContestError::UnsupportedContestCategory(msg)) => {
                assert!(msg.contains("Multi-Two"));
            }
            other => panic!("Expected UnsupportedContestCategory for Multi-Two, got {other:?}"),
        }
    }

    #[test]
    fn cqwpx_unsupported_multi_category_is_rejected() {
        assert!(matches!(
            validate_transmitter_category(CabrilloTransmitterCategory::Two),
            Err(ContestError::UnsupportedContestCategory(_))
        ));
        assert!(matches!(
            validate_transmitter_category(CabrilloTransmitterCategory::Multi),
            Err(ContestError::UnsupportedContestCategory(_))
        ));
        assert!(matches!(
            validate_transmitter_category(CabrilloTransmitterCategory::Unlimited),
            Err(ContestError::UnsupportedContestCategory(_))
        ));

        let qsos = vec![sample_qso("W1AW", "20m", "CW", "599", "599")];
        let input = CabrilloInput {
            contest_name: "CQ WPX CW",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 100,
            qsos: &qsos,
            transmitter_category: Some(CabrilloTransmitterCategory::Unlimited),
        };
        let build_res = build(&input);
        assert!(matches!(
            build_res,
            Err(ContestError::UnsupportedContestCategory(_))
        ));
    }

    #[test]
    fn cabrillo_transmitter_id_is_emitted_only_when_required() {
        assert!(!cabrillo_transmitter_id_is_emitted(
            CabrilloTransmitterCategory::One
        ));
        assert!(cabrillo_transmitter_id_is_emitted(
            CabrilloTransmitterCategory::Two
        ));
        assert!(cabrillo_transmitter_id_is_emitted(
            CabrilloTransmitterCategory::Multi
        ));
        assert!(cabrillo_transmitter_id_is_emitted(
            CabrilloTransmitterCategory::Unlimited
        ));

        let qsos = vec![sample_qso("W1AW", "20m", "CW", "599", "599")];
        let input = CabrilloInput {
            contest_name: "CQ WPX CW",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 10,
            qsos: &qsos,
            transmitter_category: Some(CabrilloTransmitterCategory::One),
        };
        let text = build(&input).expect("valid cabrillo");
        let qso_line = text.lines().find(|l| l.starts_with("QSO:")).unwrap();
        // Dla Single-Op / One wiersz zawiera 10 pól (brak pola transmitter ID na końcu)
        let cols: Vec<&str> = qso_line.split_whitespace().collect();
        assert_eq!(cols.len(), 11); // "QSO:" + 10 pól = 11 tokenów
    }

    #[test]
    fn cabrillo_preserves_non_default_rst() {
        let qso_cw = sample_qso("W1AW", "20m", "CW", "579", "589");
        let qso_ph = sample_qso("DL1XYZ", "40m", "SSB", "57", "51");
        let qsos = vec![qso_cw, qso_ph];

        let input = CabrilloInput {
            contest_name: "CQ WW DX",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 100,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("QSO:")).collect();
        assert_eq!(lines.len(), 2);
        // Sprawdź zachowanie 579 i 589 dla CW
        assert!(lines[0].contains("579") && lines[0].contains("589"));
        assert!(!lines[0].contains("599"));
        // Sprawdź zachowanie 57 i 51 dla SSB
        assert!(
            lines[1].contains(" 57 ")
                && (lines[1].contains(" 51\n")
                    || lines[1].ends_with(" 51")
                    || lines[1].contains(" 51 "))
        );
        assert!(!lines[1].contains(" 59 "));
    }

    #[test]
    fn cabrillo_uses_default_rst_only_when_missing() {
        let qso_cw = sample_qso("W1AW", "20m", "CW", "", "");
        let qso_ph = sample_qso("DL1XYZ", "40m", "SSB", "  ", "  ");
        let qsos = vec![qso_cw, qso_ph];

        let input = CabrilloInput {
            contest_name: "CQ WW DX",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 100,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("QSO:")).collect();
        assert_eq!(lines.len(), 2);
        // CW z pustym RST otrzymuje 599
        assert!(lines[0].contains(" 599 ") && lines[0].contains(" 599 "));
        // SSB z pustym RST otrzymuje 59
        assert!(
            lines[1].contains(" 59 ") && (lines[1].ends_with(" 59") || lines[1].contains(" 59 "))
        );
    }

    #[test]
    fn cabrillo_rejects_or_reports_malformed_rst_by_contract() {
        // Nie-cyfrowe znaki w RST
        let bad_cw = sample_qso("W1AW", "20m", "CW", "59A", "599");
        let input1 = CabrilloInput {
            contest_name: "CQ WW DX",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 10,
            qsos: &[bad_cw],
            transmitter_category: None,
        };
        assert!(matches!(
            build(&input1),
            Err(ContestError::InvalidContestExchange { .. })
        ));

        // Za długi raport w SSB (np. 599 zamiast 2 cyfr)
        let bad_ssb = sample_qso("DL1XYZ", "40m", "SSB", "599", "59");
        let input2 = CabrilloInput {
            contest_name: "CQ WW DX",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 10,
            qsos: &[bad_ssb],
            transmitter_category: None,
        };
        assert!(matches!(
            build(&input2),
            Err(ContestError::InvalidContestExchange { .. })
        ));
    }

    #[test]
    fn duplicate_qso_remains_in_cabrillo_log() {
        let q1 = sample_qso("DL1XYZ", "20m", "CW", "599", "599");
        let q2 = sample_qso("DL1XYZ", "20m", "CW", "599", "599"); // duplikat

        let qsos = vec![q1, q2];
        let input = CabrilloInput {
            contest_name: "SP DX Contest",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 10,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        let qso_lines: Vec<&str> = text
            .lines()
            .filter(|l| l.starts_with("QSO:") && l.contains("DL1XYZ"))
            .collect();
        assert_eq!(
            qso_lines.len(),
            2,
            "Both duplicate QSO lines must remain in Cabrillo log"
        );
    }

    #[test]
    fn duplicate_qso_scores_according_to_contest_profile() {
        let mut q1 = sample_qso("DL1XYZ", "20m", "CW", "599", "599");
        q1.dxcc = Some(230); // Niemcy (Europa)
        let mut q2 = sample_qso("DL1XYZ", "20m", "CW", "599", "599"); // duplikat
        q2.dxcc = Some(230);

        let qsos = vec![q1, q2];
        let (pts, mults, final_score) = score_sp_dx(&qsos, 269, 15);
        // Pierwsze QSO z DL daje 1 pkt dla stacji SP, drugie to duplikat dający 0 pkt
        assert_eq!(pts, 1);
        assert_eq!(mults, 1);
        assert_eq!(final_score, 1);

        let input = CabrilloInput {
            contest_name: "SP DX Contest",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: final_score,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        assert!(text.contains("CLAIMED-SCORE: 1\n"));
        let count = text.lines().filter(|l| l.contains("DL1XYZ")).count();
        assert_eq!(count, 2, "Duplicate remains in exported Cabrillo log");
    }

    #[test]
    fn zero_point_qso_remains_in_log_when_required() {
        // W CQ WW łączność w tym samym kraju daje 0 pkt, ale jest dozwolona i zostaje w logu
        let mut q = sample_qso("SP1ABC", "20m", "CW", "599", "599");
        q.dxcc = Some(269); // SP
        q.cqz = Some(15);
        q.stx_string = Some("15".to_string());
        q.srx_string = Some("15".to_string());

        let qsos = vec![q];
        let (pts, _mults, final_score) = score_cqww(&qsos, 269, 15);
        assert_eq!(pts, 0);

        let input = CabrilloInput {
            contest_name: "CQ WW CW",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: final_score,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        assert!(text.contains("CLAIMED-SCORE: 0\n"));
        assert!(
            text.lines()
                .any(|l| l.starts_with("QSO:") && l.contains("SP1ABC")),
            "Zero-point QSO must remain in Cabrillo log"
        );
    }

    #[test]
    fn unscored_qso_is_not_silently_deleted() {
        let q = sample_qso("SP1ABC", "20m", "CW", "599", "599");
        let qsos = vec![q];
        let input = CabrilloInput {
            contest_name: "CQ WW CW",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 0,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        assert!(
            text.contains("SP1ABC"),
            "Unscored QSO must not be silently deleted from log"
        );
    }

    #[test]
    fn spdx_duplicate_remains_in_cabrillo() {
        let mut q1 = sample_qso("DL1XYZ", "80m", "CW", "599", "599");
        q1.stx_string = Some("D".to_string());
        q1.srx_string = Some("001".to_string());
        let mut q2 = sample_qso("DL1XYZ", "80m", "CW", "599", "599");
        q2.stx_string = Some("D".to_string());
        q2.srx_string = Some("002".to_string());

        let qsos = vec![q1, q2];
        let input = CabrilloInput {
            contest_name: "SP DX Contest",
            callsign: "SP6INA",
            operator: "Mariusz",
            city: "Wroclaw",
            country: "Poland",
            claimed_score: 1,
            qsos: &qsos,
            transmitter_category: None,
        };

        let text = build(&input).expect("valid cabrillo");
        let lines: Vec<&str> = text
            .lines()
            .filter(|l| l.starts_with("QSO:") && l.contains("DL1XYZ"))
            .collect();
        assert_eq!(
            lines.len(),
            2,
            "SP DX duplicate must remain in Cabrillo log"
        );
    }
}
