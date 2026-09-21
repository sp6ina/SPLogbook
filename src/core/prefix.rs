// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use regex::Regex;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Informacje o kraju, strefach i prefiksie dla danego znaku wywoławczego
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrefixInfo {
    pub callsign: String,
    pub country: String,
    pub arrl_prefix: String,
    pub dxcc: u32,
    pub cqz: u32,
    pub ituz: u32,
    pub continent: String,
    pub latitude: f64,
    pub longitude: f64,
    pub wpx_prefix: String,
    pub province: Option<String>,
}

struct CompiledRule {
    pub regex: Regex,
    pub info: PrefixInfo,
}

/// Silnik dopasowywania znaków do danych DXCC i stref
pub struct PrefixMatcher {
    unique_calls: HashMap<String, PrefixInfo>,
    province_rules: Vec<CompiledRule>,
    country_rules: Vec<CompiledRule>,
}

fn parse_coord(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }
    let is_south_or_west = s.ends_with('S') || s.ends_with('W') || s.ends_with('s') || s.ends_with('w');
    let num_str = s.trim_matches(|c: char| c.is_alphabetic());
    let val: f64 = num_str.parse().unwrap_or(0.0);
    if is_south_or_west {
        -val
    } else {
        val
    }
}

/// Ekstrahuje prefiks WPX (np. SP6INA -> SP6, 3Z100POL -> 3Z100, W1AW -> W1, SP6INA/1 -> SP1, DL/SP6INA -> DL0)
pub fn extract_wpx_prefix(call: &str) -> String {
    let clean = call.trim().to_uppercase();
    let is_operational_suffix = |s: &str| -> bool {
        matches!(s, "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J")
    };

    let parts: Vec<&str> = clean
        .split('/')
        .filter(|p| !is_operational_suffix(p) && !p.is_empty())
        .collect();

    if parts.is_empty() {
        return clean;
    }

    let base_call = if parts.len() >= 2 {
        let p0 = parts[0];
        let p1 = parts[1];

        if p0.len() <= 4 && p0.len() < p1.len() && !p0.chars().all(|c| c.is_ascii_digit()) {
            p0
        } else if p1.len() == 1 && p1.chars().all(|c| c.is_ascii_digit()) {
            let prefix0 = extract_wpx_base(p0);
            let mut prefix_chars: Vec<char> = prefix0.chars().collect();
            if let Some(pos) = prefix_chars.iter().rposition(|c| c.is_ascii_digit()) {
                prefix_chars[pos] = p1.chars().next().unwrap();
                return prefix_chars.into_iter().collect();
            } else {
                return format!("{}{}", prefix0, p1);
            }
        } else if p1.len() <= 4 && p1.len() < p0.len() && !p1.chars().all(|c| c.is_ascii_digit()) {
            p1
        } else if p0.len() >= p1.len() {
            p0
        } else {
            p1
        }
    } else {
        parts[0]
    };

    extract_wpx_base(base_call)
}

fn extract_wpx_base(call: &str) -> String {
    let bytes = call.as_bytes();
    let mut last_digit_idx = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            last_digit_idx = Some(i);
        }
    }

    match last_digit_idx {
        Some(idx) => call[..=idx].to_string(),
        None => {
            let mut res = call.to_string();
            res.push('0');
            res
        }
    }
}

impl PrefixMatcher {
    /// Ładuje reguły prefiksów z bazy danych serviceLOG.db
    pub fn load_from_db(db_path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?;

        // 1. UniqueCalls
        let mut unique_calls = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT Callsign, Country, ARRLPrefix, DXCC, Continent, CQZone, ITUZone, Latitude, Longitude 
             FROM UniqueCalls WHERE Enable != '0' OR Enable IS NULL"
        )?;

        let rows = stmt.query_map([], |row| {
            let call: String = row.get(0)?;
            let dxcc_str: String = row.get(3)?;
            let dxcc: u32 = dxcc_str.parse().unwrap_or(0);
            let lat_str: String = row.get(7)?;
            let lon_str: String = row.get(8)?;

            Ok(PrefixInfo {
                callsign: call.clone(),
                country: row.get(1)?,
                arrl_prefix: row.get(2)?,
                dxcc,
                continent: row.get(4)?,
                cqz: row.get(5)?,
                ituz: row.get(6)?,
                latitude: parse_coord(&lat_str),
                longitude: parse_coord(&lon_str),
                wpx_prefix: extract_wpx_prefix(&call),
                province: None,
            })
        })?;

        for r in rows.flatten() {
            unique_calls.insert(r.callsign.to_uppercase(), r);
        }

        // 2. CountryDataEx
        let mut country_rules = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT Country, ARRLPrefix, DXCC, Continent, CQZone, ITUZone, Latitude, Longitude, PrefixList 
             FROM CountryDataEx WHERE EndDate = '' OR EndDate IS NULL"
        )?;

        let rows = stmt.query_map([], |row| {
            let lat_str: String = row.get(6)?;
            let lon_str: String = row.get(7)?;
            let prefix_list: String = row.get(8)?;

            Ok((
                PrefixInfo {
                    callsign: String::new(),
                    country: row.get(0)?,
                    arrl_prefix: row.get(1)?,
                    dxcc: row.get(2)?,
                    continent: row.get(3)?,
                    cqz: row.get(4)?,
                    ituz: row.get(5)?,
                    latitude: parse_coord(&lat_str),
                    longitude: parse_coord(&lon_str),
                    wpx_prefix: String::new(),
                    province: None,
                },
                prefix_list,
            ))
        })?;

        for (mut info, raw_regex) in rows.flatten() {
            let pattern = if raw_regex.starts_with('^') {
                raw_regex
            } else {
                format!("^(?:{})", raw_regex)
            };

            if let Ok(reg) = Regex::new(&pattern) {
                info.wpx_prefix = extract_wpx_prefix(&info.arrl_prefix);
                country_rules.push(CompiledRule { regex: reg, info });
            }
        }

        // 3. Province (Oblasty, stany itp.)
        let mut province_rules = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT Country, ARRLPrefix, DXCC, Continent, CQZone, ITUZone, Latitude, Longitude, PrefixList, Comment 
             FROM Province WHERE EndDate = '' OR EndDate IS NULL"
        )?;

        let rows = stmt.query_map([], |row| {
            let lat_str: String = row.get(6)?;
            let lon_str: String = row.get(7)?;
            let prefix_list: String = row.get(8)?;
            let province: String = row.get(9)?;

            Ok((
                PrefixInfo {
                    callsign: String::new(),
                    country: row.get(0)?,
                    arrl_prefix: row.get(1)?,
                    dxcc: {
                        let d: String = row.get(2)?;
                        d.parse().unwrap_or(0)
                    },
                    continent: row.get(3)?,
                    cqz: row.get(4)?,
                    ituz: row.get(5)?,
                    latitude: parse_coord(&lat_str),
                    longitude: parse_coord(&lon_str),
                    wpx_prefix: String::new(),
                    province: Some(province),
                },
                prefix_list,
            ))
        })?;

        for (mut info, raw_regex) in rows.flatten() {
            let pattern = if raw_regex.starts_with('^') {
                raw_regex
            } else {
                format!("^(?:{})", raw_regex)
            };

            if let Ok(reg) = Regex::new(&pattern) {
                info.wpx_prefix = extract_wpx_prefix(&info.arrl_prefix);
                province_rules.push(CompiledRule { regex: reg, info });
            }
        }

        Ok(Self {
            unique_calls,
            province_rules,
            country_rules,
        })
    }

    /// Tworzy pusty silnik dopasowywania prefiksów (np. gdy brak bazy danych)
    pub fn empty() -> Self {
        Self {
            unique_calls: HashMap::new(),
            province_rules: Vec::new(),
            country_rules: Vec::new(),
        }
    }

    /// Normalizuje znak wywoławczy dla potrzeb dopasowania prefiksu (obsługa /P, /M, prefiksów gościnnych itp.)
    pub fn normalize_call_for_prefix(clean: &str) -> &str {
        let is_operational_suffix = |s: &str| -> bool {
            matches!(s, "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J")
        };

        let mut parts: Vec<&str> = clean.split('/').collect();
        if parts.len() == 1 {
            return clean;
        }

        // Usuń końcowe sufiksy operacyjne (/P, /M itp.)
        while parts.len() > 1 && is_operational_suffix(parts.last().unwrap()) {
            parts.pop();
        }

        if parts.len() == 1 {
            return parts[0];
        }

        let p0 = parts[0];
        let p1 = parts[1];

        // 1. Przypadek stacji gościnnej na początku: DL/SP6INA, OE3/SP6INA, 3D2/SP6INA -> prefiks gościnny (DL, OE3)
        if p0.len() <= 4 && p0.len() < p1.len() && !p0.chars().all(|c| c.is_ascii_digit()) {
            return p0;
        }

        // 2. Przypadek stacji gościnnej na końcu: SP6INA/W6, SP6INA/DL
        if p1.len() <= 4 && p1.len() < p0.len() && !p1.chars().all(|c| c.is_ascii_digit()) {
            return p1;
        }

        // 3. Domyślnie bierzemy dłuższą część (znak główny)
        if p0.len() >= p1.len() {
            p0
        } else {
            p1
        }
    }

    /// Wyszukuje kraj DXCC, strefy CQ/ITU i dane dla podanego znaku
    pub fn lookup(&self, callsign: &str) -> Option<PrefixInfo> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return None;
        }

        // 1. Sprawdź UniqueCalls (dokładne dopasowanie)
        if let Some(info) = self.unique_calls.get(&clean) {
            let mut res = info.clone();
            res.callsign = clean;
            return Some(res);
        }

        // Znormalizuj znak do rozpoznania prefiksu (np. SP6INA/P -> SP6INA, OE3/SP6INA -> OE3, DL/SP6INA/M -> DL)
        let lookup_call = Self::normalize_call_for_prefix(&clean);

        // 2. Sprawdź Province (specyficzne regiony)
        for rule in &self.province_rules {
            if rule.regex.is_match(lookup_call) {
                let mut res = rule.info.clone();
                res.callsign = clean.clone();
                res.wpx_prefix = extract_wpx_prefix(lookup_call);
                return Some(res);
            }
        }

        // 3. Sprawdź CountryDataEx
        for rule in &self.country_rules {
            if rule.regex.is_match(lookup_call) {
                let mut res = rule.info.clone();
                res.callsign = clean.clone();
                res.wpx_prefix = extract_wpx_prefix(lookup_call);
                return Some(res);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wpx() {
        assert_eq!(extract_wpx_prefix("SP6INA"), "SP6");
        assert_eq!(extract_wpx_prefix("3Z100POL"), "3Z100");
        assert_eq!(extract_wpx_prefix("W1AW"), "W1");
        assert_eq!(extract_wpx_prefix("DL1ABC/P"), "DL1");
        assert_eq!(extract_wpx_prefix("SP6INA/1"), "SP1");
        assert_eq!(extract_wpx_prefix("SP6INA/W6"), "W6");
        assert_eq!(extract_wpx_prefix("DL/SP6INA/P"), "DL0");
    }

    #[test]
    fn test_normalize_call_for_prefix() {
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("SP6INA/P"), "SP6INA");
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("SP6INA/M"), "SP6INA");
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("SP6INA/QRP"), "SP6INA");
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("DL/SP6INA"), "DL");
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("OE3/SP6INA"), "OE3");
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("3D2/SP6INA/P"), "3D2");
    }
}
