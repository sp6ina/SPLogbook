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

    for (idx, qso) in input.qsos.iter().rev().enumerate() {
        let freq_khz = qso.freq.map_or_else(
            || match qso.band.as_str() {
                "160m" => 1840,
                "80m" => 3700,
                "40m" => 7100,
                "20m" => 14200,
                "15m" => 21200,
                "10m" => 28500,
                _ => 14000,
            },
            |f| (f * 1000.0) as u64,
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

        let my_rst = if qso.mode == "CW" { "599" } else { "59" };
        let his_rst = &qso.rst_rcvd;
        let my_serial = idx + 1;
        let his_serial = qso.srx.unwrap_or(1);

        let _ = writeln!(
            out,
            "QSO: {:5} {:2} {} {} {:10} {:3} {:03} {:10} {:3} {:03}",
            freq_khz,
            if qso.mode == "CW" { "CW" } else { "PH" },
            date_str,
            time_str,
            input.callsign,
            my_rst,
            my_serial,
            qso.callsign,
            his_rst,
            his_serial
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
        let mut qso1 = sample_qso("DL1XYZ", "20m", "CW", "599", "599001");
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

        // Linia QSO: 14025 CW 2026-04-04 1200 SP6INA     599 001 DL1XYZ     599001 042
        assert!(
            text.contains(
                "QSO: 14025 CW 2026-04-04 1200 SP6INA     599 001 DL1XYZ     599001 042\n"
            )
        );

        // Zakończenie
        assert!(text.ends_with("END-OF-LOG:\n"));
    }
}
