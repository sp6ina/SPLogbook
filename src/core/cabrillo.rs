// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Generator plików w formacie Cabrillo v3.0 dla zawodów krótkofalarskich.

use crate::core::qso::QsoRecord;
use std::fmt::Write;

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

/// Czysta funkcja generująca tekst raportu Cabrillo 3.0.
pub fn build(input: &CabrilloInput) -> String {
    let mut out = String::new();
    out.push_str("START-OF-LOG: 3.0\n");
    let _ = writeln!(out, "CONTEST: {}", input.contest_name);
    let _ = writeln!(out, "CALLSIGN: {}", input.callsign);
    out.push_str("CATEGORY-OPERATOR: SINGLE-OP\n");
    out.push_str("CATEGORY-TRANSMITTER: ONE\n");
    out.push_str("CATEGORY-POWER: HIGH\n");
    out.push_str("CATEGORY-BAND: ALL\n");
    out.push_str("CATEGORY-MODE: MIXED\n");
    out.push_str("CATEGORY-STATION: FIXED\n");
    let _ = writeln!(out, "CLAIMED-SCORE: {}", input.claimed_score);
    let _ = writeln!(out, "OPERATORS: {}", input.callsign);
    let _ = writeln!(out, "NAME: {}", input.operator);
    let _ = writeln!(out, "ADDRESS: {}, {}", input.city, input.country);
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

        let default_rst = if mode_cab == "CW" || mode_cab == "RY" || mode_cab == "DG" {
            "599"
        } else {
            "59"
        };

        let my_rst = if !qso.rst_sent.trim().is_empty() {
            qso.rst_sent.trim()
        } else {
            default_rst
        };

        let his_rst = if !qso.rst_rcvd.trim().is_empty() {
            qso.rst_rcvd.trim()
        } else {
            default_rst
        };

        let my_exch = if let Some(stx_s) = &qso.stx_string {
            let trimmed = stx_s.trim();
            if !trimmed.is_empty() {
                trimmed.to_string()
            } else if let Some(stx) = qso.stx {
                format!("{stx:03}")
            } else {
                format!("{:03}", idx + 1)
            }
        } else if let Some(stx) = qso.stx {
            format!("{stx:03}")
        } else {
            format!("{:03}", idx + 1)
        };

        let his_exch = if let Some(srx_s) = &qso.srx_string {
            let trimmed = srx_s.trim();
            if !trimmed.is_empty() {
                trimmed.to_string()
            } else if let Some(srx) = qso.srx {
                format!("{srx:03}")
            } else {
                "001".to_string()
            }
        } else if let Some(srx) = qso.srx {
            format!("{srx:03}")
        } else {
            "001".to_string()
        };

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
    out
}

#[cfg(test)]
mod tests {
    use super::*;
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
        };

        let cabrillo = build(&input);
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
        };

        let text = build(&input);

        // Nagłówek
        assert!(text.starts_with("START-OF-LOG: 3.0\n"));
        assert!(text.contains("CONTEST: CQ WW DX\n"));
        assert!(text.contains("CALLSIGN: SP6INA\n"));
        assert!(text.contains("CATEGORY-OPERATOR: SINGLE-OP\n"));
        assert!(text.contains("CLAIMED-SCORE: 1234\n"));
        assert!(text.contains("OPERATORS: SP6INA\n"));
        assert!(text.contains("NAME: Mariusz\n"));
        assert!(text.contains("ADDRESS: Opole, Poland\n"));
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
        };

        let text = build(&input);
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
        };

        let text = build(&input);
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
}
