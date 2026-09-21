// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Lista 50 stanów USA w programie WAS (Worked All States)
pub const US_STATES: [&str; 50] = [
    "AK", "AL", "AR", "AZ", "CA", "CO", "CT", "DE", "FL", "GA",
    "HI", "IA", "ID", "IL", "IN", "KS", "KY", "LA", "MA", "MD",
    "ME", "MI", "MN", "MO", "MS", "MT", "NC", "ND", "NE", "NH",
    "NJ", "NM", "NV", "NY", "OH", "OK", "OR", "PA", "RI", "SC",
    "SD", "TN", "TX", "UT", "VA", "VT", "WA", "WI", "WV", "WY",
];

/// Kontynenty w programie WAC (Worked All Continents)
pub const CONTINENTS: [&str; 7] = ["AF", "AN", "AS", "EU", "NA", "OC", "SA"];

/// Status łączności względem dyplomów (do podświetleń i alarmów w locie)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QsoAwardStatus {
    pub is_worked_b4: bool,    // Czy ta sama stacja była już pracowana na tym samym paśmie i emisji
    pub is_new_dxcc: bool,     // Nowy kraj DXCC (All Time New One - ATNO)
    pub is_new_band: bool,     // Nowe pasmo dla danego kraju DXCC
    pub is_new_mode: bool,     // Nowa emisja dla danego kraju DXCC
    pub is_new_pga: bool,      // Nowa gmina polska w programie PGA
    pub is_new_waz: bool,      // Nowa strefa CQ (WAZ 1-40)
    pub is_new_was: bool,      // Nowy stan USA (WAS)
    pub is_new_wac: bool,      // Nowy kontynent (WAC)
    pub is_new_iota: bool,     // Nowa wyspa IOTA
}

/// Okręgi Polskiego Związku Krótkofalowców (PZK) i województwa
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishDistrictInfo {
    pub district: u8,
    pub voivodeships: Vec<String>,
}

/// Wyciąga prefiks WPX ze znaku (np. SP6INA -> SP6, DL/SP6INA -> DL0, K3LR -> K3)
pub fn extract_wpx_prefix(call: &str) -> String {
    let clean = call.trim().to_uppercase();
    if clean.is_empty() {
        return String::new();
    }

    let parts: Vec<&str> = clean.split('/').collect();
    let main_call = if parts.len() > 1 {
        if parts[0].len() <= 3 && !parts[0].chars().any(|c| c.is_ascii_digit()) {
            return format!("{}0", parts[0]);
        }
        if parts[1].len() == 1 && parts[1].chars().all(|c| c.is_ascii_digit()) {
            let pfx = extract_wpx_prefix(parts[0]);
            let alpha: String = pfx.chars().take_while(|c| !c.is_ascii_digit()).collect();
            return format!("{}{}", alpha, parts[1]);
        }
        parts[0]
    } else {
        &clean
    };

    if let Some(last_digit_pos) = main_call.rfind(|c: char| c.is_ascii_digit()) {
        main_call[..=last_digit_pos].to_string()
    } else {
        format!("{}0", main_call)
    }
}

pub const WAE_EUROPEAN_ENTITIES: &[u32] = &[
    1, 14, 15, 21, 22, 27, 29, 33, 40, 42, 45, 49, 54, 56, 61, 62, 63, 66, 70, 72, 74, 75, 76, 79, 82, 84, 86, 87, 88, 100, 104, 106, 110, 111, 113, 117, 118, 120, 123, 126, 128, 130, 134, 138, 146, 147, 150, 151, 152, 153, 154, 160, 163, 164, 169, 170, 175, 176, 177, 179, 230, 239, 248, 269, 279, 283, 496, 497, 221,
];

pub const SP_DISTRICTS: &[&str] = &[
    "SP1", "SP2", "SP3", "SP4", "SP5", "SP6", "SP7", "SP8", "SP9",
    "SO1", "SO2", "SO3", "SO4", "SO5", "SO6", "SO7", "SO8", "SO9",
    "SN1", "SN2", "SN3", "SN4", "SN5", "SN6", "SN7", "SN8", "SN9",
    "3Z1", "3Z2", "3Z3", "3Z4", "3Z5", "3Z6", "3Z7", "3Z8", "3Z9",
    "HF1", "HF2", "HF3", "HF4", "HF5", "HF6", "HF7", "HF8", "HF9",
    "SQ1", "SQ2", "SQ3", "SQ4", "SQ5", "SQ6", "SQ7", "SQ8", "SQ9",
    "SR1", "SR2", "SR3", "SR4", "SR5", "SR6", "SR7", "SR8", "SR9",
];

pub fn extract_sp_district(callsign: &str) -> Option<String> {
    let call = callsign.trim().to_uppercase();
    let base = call.split('/').next().unwrap_or(&call);
    for prefix in &["SP", "SO", "SN", "3Z", "HF", "SQ", "SR"] {
        if let Some(rest) = base.strip_prefix(prefix) {
            if let Some(digit) = rest.chars().next() {
                if digit.is_ascii_digit() {
                    return Some(format!("{}{}", prefix, digit));
                }
            }
        }
    }
    None
}

/// Silnik śledzenia postępu dyplomowego (krajowego i międzynarodowego)
pub struct AwardsEngine {
    pub worked_calls: HashSet<(String, String, String)>,
    pub worked_dxcc_all: HashSet<u32>,
    pub worked_dxcc_band: HashSet<(u32, String)>,
    pub worked_dxcc_mode: HashSet<(u32, String)>,
    pub confirmed_dxcc: HashSet<u32>,
    pub worked_waz: HashSet<u32>,
    pub confirmed_waz: HashSet<u32>,
    pub worked_was: HashSet<String>,
    pub confirmed_was: HashSet<String>,
    pub worked_wac: HashSet<String>,
    pub confirmed_wac: HashSet<String>,
    pub worked_wpx: HashSet<String>,
    pub worked_vucc: HashSet<String>,
    pub worked_iota: HashSet<String>,
    pub worked_sota: HashSet<String>,
    pub worked_pota: HashSet<String>,
    pub worked_pga: HashSet<String>,

    pub worked_sp_districts: HashSet<String>,
    pub confirmed_sp_districts: HashSet<String>,
    pub worked_wae: HashSet<u32>,
    pub confirmed_wae: HashSet<u32>,
    pub worked_wwff: HashSet<String>,
    pub confirmed_wwff: HashSet<String>,
    pub worked_rda: HashSet<String>,
    pub confirmed_rda: HashSet<String>,

    // Szczegółowe mapy łączności dla programów (podgląd w zakładce Inne)
    pub details_wpx: std::collections::HashMap<String, Vec<AwardWorkedRecord>>,
    pub details_iota: std::collections::HashMap<String, Vec<AwardWorkedRecord>>,
    pub details_vucc: std::collections::HashMap<String, Vec<AwardWorkedRecord>>,
    pub details_sota: std::collections::HashMap<String, Vec<AwardWorkedRecord>>,
    pub details_pota: std::collections::HashMap<String, Vec<AwardWorkedRecord>>,
    pub details_pga: std::collections::HashMap<String, Vec<AwardWorkedRecord>>,
}

/// Szczegółowy rekord zaliczonej łączności dla dyplomów z zakładki "Inne"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AwardWorkedRecord {
    pub key: String,         // np. "SP6", "EU-001", "JO81", "WR01", "SP/SZ-001", "SP-0001"
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub qso_date: String,
    pub is_confirmed: bool,
}

impl Default for AwardsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AwardsEngine {
    pub fn new() -> Self {
        Self {
            worked_calls: HashSet::new(),
            worked_dxcc_all: HashSet::new(),
            worked_dxcc_band: HashSet::new(),
            worked_dxcc_mode: HashSet::new(),
            confirmed_dxcc: HashSet::new(),
            worked_waz: HashSet::new(),
            confirmed_waz: HashSet::new(),
            worked_was: HashSet::new(),
            confirmed_was: HashSet::new(),
            worked_wac: HashSet::new(),
            confirmed_wac: HashSet::new(),
            worked_wpx: HashSet::new(),
            worked_vucc: HashSet::new(),
            worked_iota: HashSet::new(),
            worked_sota: HashSet::new(),
            worked_pota: HashSet::new(),
            worked_pga: HashSet::new(),
            worked_sp_districts: HashSet::new(),
            confirmed_sp_districts: HashSet::new(),
            worked_wae: HashSet::new(),
            confirmed_wae: HashSet::new(),
            worked_wwff: HashSet::new(),
            confirmed_wwff: HashSet::new(),
            worked_rda: HashSet::new(),
            confirmed_rda: HashSet::new(),
            details_wpx: std::collections::HashMap::new(),
            details_iota: std::collections::HashMap::new(),
            details_vucc: std::collections::HashMap::new(),
            details_sota: std::collections::HashMap::new(),
            details_pota: std::collections::HashMap::new(),
            details_pga: std::collections::HashMap::new(),
        }
    }

    pub fn rebuild_from_qsos(&mut self, qsos: &[QsoRecord]) {
        *self = Self::new();
        for q in qsos {
            self.register_qso_record(q);
        }
    }

    pub fn register_qso_record(&mut self, qso: &QsoRecord) {
        let is_confirmed = qso.qsl_rcvd == "Y" || qso.lotw_qsl_rcvd == "Y" || qso.eqsl_qsl_rcvd == "Y";
        
        let wpx = extract_wpx_prefix(&qso.callsign);
        if !wpx.is_empty() {
            self.worked_wpx.insert(wpx.clone());
            self.details_wpx.entry(wpx.clone()).or_default().push(AwardWorkedRecord {
                key: wpx,
                callsign: qso.callsign.clone(),
                band: qso.band.clone(),
                mode: qso.mode.clone(),
                qso_date: qso.qso_date.clone(),
                is_confirmed,
            });
        }

        if let Some(ref i) = qso.iota {
            let i_clean = i.trim().to_uppercase();
            if !i_clean.is_empty() {
                self.worked_iota.insert(i_clean.clone());
                self.details_iota.entry(i_clean.clone()).or_default().push(AwardWorkedRecord {
                    key: i_clean,
                    callsign: qso.callsign.clone(),
                    band: qso.band.clone(),
                    mode: qso.mode.clone(),
                    qso_date: qso.qso_date.clone(),
                    is_confirmed,
                });
            }
        }

        if let Some(ref g) = qso.gridsquare {
            let g_clean = g.trim().to_uppercase();
            if g_clean.len() >= 4 {
                let grid4 = g_clean[..4].to_string();
                self.worked_vucc.insert(grid4.clone());
                self.details_vucc.entry(grid4.clone()).or_default().push(AwardWorkedRecord {
                    key: grid4,
                    callsign: qso.callsign.clone(),
                    band: qso.band.clone(),
                    mode: qso.mode.clone(),
                    qso_date: qso.qso_date.clone(),
                    is_confirmed,
                });
            }
        }

        if let Some(ref s) = qso.sota_ref {
            let s_clean = s.trim().to_uppercase();
            if !s_clean.is_empty() {
                self.worked_sota.insert(s_clean.clone());
                self.details_sota.entry(s_clean.clone()).or_default().push(AwardWorkedRecord {
                    key: s_clean,
                    callsign: qso.callsign.clone(),
                    band: qso.band.clone(),
                    mode: qso.mode.clone(),
                    qso_date: qso.qso_date.clone(),
                    is_confirmed,
                });
            }
        }

        if let Some(ref p) = qso.pota_ref {
            let p_clean = p.trim().to_uppercase();
            if !p_clean.is_empty() {
                self.worked_pota.insert(p_clean.clone());
                self.details_pota.entry(p_clean.clone()).or_default().push(AwardWorkedRecord {
                    key: p_clean,
                    callsign: qso.callsign.clone(),
                    band: qso.band.clone(),
                    mode: qso.mode.clone(),
                    qso_date: qso.qso_date.clone(),
                    is_confirmed,
                });
            }
        }

        if let Some(ref pga) = qso.pga_ref {
            let pga_clean = pga.trim().to_uppercase();
            if !pga_clean.is_empty() {
                self.worked_pga.insert(pga_clean.clone());
                self.details_pga.entry(pga_clean.clone()).or_default().push(AwardWorkedRecord {
                    key: pga_clean,
                    callsign: qso.callsign.clone(),
                    band: qso.band.clone(),
                    mode: qso.mode.clone(),
                    qso_date: qso.qso_date.clone(),
                    is_confirmed,
                });
            }
        }

        self.register_qso_full(
            &qso.callsign,
            &qso.band,
            &qso.mode,
            qso.dxcc,
            qso.pga_ref.as_deref(),
            qso.cqz,
            qso.state.as_deref(),
            qso.continent.as_deref(),
            qso.iota.as_deref(),
            qso.gridsquare.as_deref(),
            qso.sota_ref.as_deref(),
            qso.pota_ref.as_deref(),
            is_confirmed,
        );
    }

    pub fn register_qso(&mut self, call: &str, band: &str, mode: &str, dxcc: Option<u32>, pga: Option<&str>) {
        self.register_qso_full(call, band, mode, dxcc, pga, None, None, None, None, None, None, None, false);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_qso_full(
        &mut self,
        call: &str,
        band: &str,
        mode: &str,
        dxcc: Option<u32>,
        pga: Option<&str>,
        cqz: Option<u32>,
        state: Option<&str>,
        continent: Option<&str>,
        iota: Option<&str>,
        gridsquare: Option<&str>,
        sota: Option<&str>,
        pota: Option<&str>,
        is_confirmed: bool,
    ) {
        let call = call.to_uppercase();
        let band = band.to_string();
        let mode = mode.to_uppercase();

        self.worked_calls.insert((call.clone(), band.clone(), mode.clone()));

        if let Some(dxcc_id) = dxcc {
            self.worked_dxcc_all.insert(dxcc_id);
            self.worked_dxcc_band.insert((dxcc_id, band));
            self.worked_dxcc_mode.insert((dxcc_id, mode));
            if is_confirmed {
                self.confirmed_dxcc.insert(dxcc_id);
            }
        }

        if let Some(z) = cqz {
            if (1..=40).contains(&z) {
                self.worked_waz.insert(z);
                if is_confirmed {
                    self.confirmed_waz.insert(z);
                }
            }
        }

        if let Some(st) = state {
            let st_clean = st.trim().to_uppercase();
            if US_STATES.contains(&st_clean.as_str()) {
                self.worked_was.insert(st_clean.clone());
                if is_confirmed {
                    self.confirmed_was.insert(st_clean);
                }
            }
        }

        if let Some(cont) = continent {
            let c_clean = cont.trim().to_uppercase();
            if CONTINENTS.contains(&c_clean.as_str()) {
                self.worked_wac.insert(c_clean.clone());
                if is_confirmed {
                    self.confirmed_wac.insert(c_clean);
                }
            }
        }

        if let Some(i) = iota {
            let i_clean = i.trim().to_uppercase();
            if !i_clean.is_empty() {
                self.worked_iota.insert(i_clean);
            }
        }

        if let Some(grid) = gridsquare {
            let g_clean = grid.trim().to_uppercase();
            if g_clean.len() >= 4 {
                let grid4 = g_clean[..4].to_string();
                self.worked_vucc.insert(grid4);
            }
        }

        if let Some(s) = sota {
            let s_clean = s.trim().to_uppercase();
            if !s_clean.is_empty() {
                self.worked_sota.insert(s_clean);
            }
        }

        if let Some(p) = pota {
            let p_clean = p.trim().to_uppercase();
            if !p_clean.is_empty() {
                self.worked_pota.insert(p_clean);
            }
        }

        if let Some(pga_code) = pga {
            let pga_clean = pga_code.trim().to_uppercase();
            if !pga_clean.is_empty() {
                self.worked_pga.insert(pga_clean);
            }
        }

        let pfx = extract_wpx_prefix(&call);
        if !pfx.is_empty() {
            self.worked_wpx.insert(pfx);
        }

        if let Some(district) = extract_sp_district(&call) {
            self.worked_sp_districts.insert(district.clone());
            if is_confirmed {
                self.confirmed_sp_districts.insert(district);
            }
        }

        if let Some(d) = dxcc {
            if WAE_EUROPEAN_ENTITIES.contains(&d) {
                self.worked_wae.insert(d);
                if is_confirmed {
                    self.confirmed_wae.insert(d);
                }
            }
        }

        if let Some(ref wwff) = sota {
            if wwff.contains('-') && !wwff.starts_with('W') {
                let _ = wwff;
            }
        }
    }

    pub fn check_status(&self, call: &str, band: &str, mode: &str, dxcc: Option<u32>, pga: Option<&str>) -> QsoAwardStatus {
        self.check_status_full(call, band, mode, dxcc, pga, None, None, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn check_status_full(
        &self,
        call: &str,
        band: &str,
        mode: &str,
        dxcc: Option<u32>,
        pga: Option<&str>,
        cqz: Option<u32>,
        state: Option<&str>,
        continent: Option<&str>,
        iota: Option<&str>,
    ) -> QsoAwardStatus {
        let call = call.to_uppercase();
        let band = band.to_string();
        let mode = mode.to_uppercase();

        let is_worked_b4 = self.worked_calls.contains(&(call, band.clone(), mode.clone()));

        let (is_new_dxcc, is_new_band, is_new_mode) = match dxcc {
            Some(dxcc_id) => {
                let new_dxcc = !self.worked_dxcc_all.contains(&dxcc_id);
                let new_band = !self.worked_dxcc_band.contains(&(dxcc_id, band));
                let new_mode = !self.worked_dxcc_mode.contains(&(dxcc_id, mode));
                (new_dxcc, new_band, new_mode)
            }
            None => (false, false, false),
        };

        let is_new_pga = match pga {
            Some(code) => {
                let clean = code.trim().to_uppercase();
                !clean.is_empty() && !self.worked_pga.contains(&clean)
            }
            None => false,
        };

        let is_new_waz = match cqz {
            Some(z) => (1..=40).contains(&z) && !self.worked_waz.contains(&z),
            None => false,
        };

        let is_new_was = match state {
            Some(st) => {
                let st_clean = st.trim().to_uppercase();
                US_STATES.contains(&st_clean.as_str()) && !self.worked_was.contains(&st_clean)
            }
            None => false,
        };

        let is_new_wac = match continent {
            Some(c) => {
                let c_clean = c.trim().to_uppercase();
                CONTINENTS.contains(&c_clean.as_str()) && !self.worked_wac.contains(&c_clean)
            }
            None => false,
        };

        let is_new_iota = match iota {
            Some(i) => {
                let i_clean = i.trim().to_uppercase();
                !i_clean.is_empty() && !self.worked_iota.contains(&i_clean)
            }
            None => false,
        };

        QsoAwardStatus {
            is_worked_b4,
            is_new_dxcc,
            is_new_band,
            is_new_mode,
            is_new_pga,
            is_new_waz,
            is_new_was,
            is_new_wac,
            is_new_iota,
        }
    }

    pub fn get_polish_district(call: &str) -> Option<PolishDistrictInfo> {
        let clean = call.trim().to_uppercase();
        if !clean.starts_with("SP") && !clean.starts_with("SQ") && !clean.starts_with("SO")
            && !clean.starts_with("SN") && !clean.starts_with("3Z") && !clean.starts_with("HF") {
            return None;
        }

        let parts: Vec<&str> = clean.split('/').collect();
        let district = if let Some(d_str) = parts.iter().rev().find(|p| p.len() == 1 && p.chars().all(|c| c.is_ascii_digit() && c != '0')) {
            d_str.chars().next()?.to_digit(10)? as u8
        } else {
            let num_char = clean.chars().find(|c| c.is_ascii_digit())?;
            num_char.to_digit(10)? as u8
        };

        let voivodeships: Vec<String> = match district {
            1 => vec!["Zachodniopomorskie"],
            2 => vec!["Kujawsko-Pomorskie", "Pomorskie"],
            3 => vec!["Wielkopolskie", "Lubuskie"],
            4 => vec!["Warmińsko-Mazurskie", "Podlaskie"],
            5 => vec!["Mazowieckie"],
            6 => vec!["Dolnośląskie", "Opolskie"],
            7 => vec!["Łódzkie", "Świętokrzyskie"],
            8 => vec!["Lubelskie", "Podkarpackie"],
            9 => vec!["Małopolskie", "Śląskie"],
            _ => vec!["Polska"],
        }.into_iter().map(|s| s.to_string()).collect();

        Some(PolishDistrictInfo {
            district,
            voivodeships,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_award_status_tracking() {
        let mut engine = AwardsEngine::new();

        let st = engine.check_status("SP6INA", "20m", "CW", Some(269), Some("WR01"));
        assert!(!st.is_worked_b4);
        assert!(st.is_new_dxcc);
        assert!(st.is_new_band);
        assert!(st.is_new_pga);

        engine.register_qso("SP6INA", "20m", "CW", Some(269), Some("WR01"));

        let st2 = engine.check_status("SP6INA", "20m", "CW", Some(269), Some("WR01"));
        assert!(st2.is_worked_b4);
        assert!(!st2.is_new_dxcc);
        assert!(!st2.is_new_pga);

        let st3 = engine.check_status("SP6INA", "40m", "CW", Some(269), None);
        assert!(!st3.is_worked_b4);
        assert!(!st3.is_new_dxcc);
        assert!(st3.is_new_band);
    }

    #[test]
    fn test_wpx_extraction() {
        assert_eq!(extract_wpx_prefix("SP6INA"), "SP6");
        assert_eq!(extract_wpx_prefix("W1AW"), "W1");
        assert_eq!(extract_wpx_prefix("K3LR"), "K3");
        assert_eq!(extract_wpx_prefix("3Z100POL"), "3Z100");
        assert_eq!(extract_wpx_prefix("DL/SP6INA"), "DL0");
        assert_eq!(extract_wpx_prefix("SP6INA/1"), "SP1");
    }

    #[test]
    fn test_polish_district() {
        let d6 = AwardsEngine::get_polish_district("SP6INA").unwrap();
        assert_eq!(d6.district, 6);
        assert!(d6.voivodeships.iter().any(|v| v == "Dolnośląskie"));

        let d5 = AwardsEngine::get_polish_district("SQ5ABC").unwrap();
        assert_eq!(d5.district, 5);
        assert!(d5.voivodeships.iter().any(|v| v == "Mazowieckie"));

        let d1 = AwardsEngine::get_polish_district("SP6INA/1").unwrap();
        assert_eq!(d1.district, 1);
        assert!(d1.voivodeships.iter().any(|v| v == "Zachodniopomorskie"));
    }
}
