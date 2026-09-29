// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Dedykowany eksport łączności do oficjalnego formatu SOTA V2 CSV (Summits on the Air)

use crate::core::qso::QsoRecord;
use std::fmt::Write as _;

/// Eksportuje wektor łączności do oficjalnego formatu SOTA Database V2 CSV
/// Specyfikacja formatu:
/// V2,[MyCall],[MySOTA],[Date:DD/MM/YY],[Time:HH:MM],[Band:e.g. 14MHz],[Mode],[HisCall],[HisSOTA],[Notes]
pub fn export_sota_csv(qsos: &[QsoRecord], my_call: &str, my_sota: &str) -> Result<String, String> {
    let mut out = String::new();

    for q in qsos {
        let clean_my_call = my_call.trim().to_uppercase();
        let clean_my_sota = my_sota.trim().to_uppercase();
        let date_formatted = format_sota_date(&q.qso_date)
            .map_err(|_| format!("Nieprawidłowa data w łączności z {}", q.callsign))?;
        let time_formatted = format_sota_time(&q.time_on)
            .map_err(|_| format!("Nieprawidłowy czas w łączności z {}", q.callsign))?;
        let band_formatted = format_sota_band(&q.band);
        let mode_formatted = q.mode.trim().to_uppercase();
        let his_call = q.callsign.trim().to_uppercase();
        let his_sota = q.sota_ref.as_deref().unwrap_or("").trim().to_uppercase();
        let notes = q.comment.as_deref().unwrap_or("").trim().replace(',', ";");

        let _ = writeln!(
            out,
            "V2,{clean_my_call},{clean_my_sota},{date_formatted},{time_formatted},{band_formatted},{mode_formatted},{his_call},{his_sota},{notes}"
        );
    }

    Ok(out)
}

fn format_sota_date(adif_date: &str) -> Result<String, ()> {
    let digits: String = adif_date.chars().filter(char::is_ascii_digit).collect();
    if digits.len() == 8 {
        // YYYYMMDD -> DD/MM/YY
        let y = &digits[2..4];
        let m = &digits[4..6];
        let d = &digits[6..8];
        Ok(format!("{d}/{m}/{y}"))
    } else {
        Err(())
    }
}

fn format_sota_time(adif_time: &str) -> Result<String, ()> {
    let digits: String = adif_time.chars().filter(char::is_ascii_digit).collect();
    if digits.len() >= 4 {
        // HHMM -> HH:MM
        Ok(format!("{}:{}", &digits[0..2], &digits[2..4]))
    } else {
        Err(())
    }
}

fn format_sota_band(band: &str) -> String {
    let clean = band.trim().to_lowercase();
    match clean.as_str() {
        "160m" => "1.8MHz".to_string(),
        "80m" => "3.5MHz".to_string(),
        "60m" => "5.3MHz".to_string(),
        "40m" => "7MHz".to_string(),
        "30m" => "10MHz".to_string(),
        "20m" => "14MHz".to_string(),
        "17m" => "18MHz".to_string(),
        "15m" => "21MHz".to_string(),
        "12m" => "24MHz".to_string(),
        "10m" => "28MHz".to_string(),
        "6m" => "50MHz".to_string(),
        "4m" => "70MHz".to_string(),
        "2m" => "144MHz".to_string(),
        "1.25m" => "220MHz".to_string(),
        "70cm" => "432MHz".to_string(),
        "33cm" => "900MHz".to_string(),
        "23cm" => "1240MHz".to_string(),
        "13cm" => "2300MHz".to_string(),
        "9cm" => "3400MHz".to_string(),
        "6cm" => "5650MHz".to_string(),
        "3cm" => "10000MHz".to_string(),
        "1.25cm" => "24000MHz".to_string(),
        other => {
            if other.ends_with("mhz") {
                other.to_uppercase()
            } else {
                format!("{}MHz", other.replace('m', ""))
            }
        }
    }
}

pub struct SotaExporter;

impl SotaExporter {
    pub fn export_csv_v2(
        qsos: &[QsoRecord],
        my_call: &str,
        my_sota: &str,
    ) -> Result<String, String> {
        export_sota_csv(qsos, my_call, my_sota)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sota_csv_export() {
        let mut q = QsoRecord::new("OE/SP6INA/P", "20m", "CW");
        q.qso_date = "20260920".to_string();
        q.time_on = "1430".to_string();
        q.sota_ref = Some("OE/TI-123".to_string());
        q.comment = Some("Peak summit activation".to_string());

        let csv = export_sota_csv(&[q], "SP6INA", "SP/SS-001").unwrap();
        assert!(csv.starts_with("V2,SP6INA,SP/SS-001,20/09/26,14:30,14MHz,CW,OE/SP6INA/P,OE/TI-123,Peak summit activation"));
    }
}
