// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use regex::Regex;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, RwLock};

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

/// Źródło reguł aktualnie załadowanych do [`PrefixMatcher`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrefixSource {
    /// Brak danych referencyjnych (stan początkowy).
    #[default]
    None,
    /// Starsza baza referencyjna SQLite.
    LegacyDatabase,
    /// Pliki krajów `cty.dat` (+ opcjonalnie `cty.csv` z kodami encji ADIF).
    CountryFiles { entity_codes: bool },
}

/// Podsumowanie wczytania plików krajów.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CountryFileReport {
    pub entities: usize,
    pub prefixes: usize,
    pub exact_calls: usize,
    pub issues: Vec<crate::core::reference::cty::CtyIssue>,
    pub csv_issues: Vec<crate::core::reference::cty::CtyIssue>,
    pub entity_codes: Option<crate::core::reference::cty::EntityCodeReport>,
}

#[derive(Default)]
struct RuleSet {
    unique_calls: HashMap<String, PrefixInfo>,
    province_rules: Vec<CompiledRule>,
    country_rules: Vec<CompiledRule>,
    cty: Option<crate::core::reference::cty::CtyData>,
    source: PrefixSource,
}

/// Silnik dopasowywania znaków do danych DXCC i stref
pub struct PrefixMatcher {
    /// Reguły za blokadą RW — umożliwia przeładowanie danych bez restartu
    /// (współdzielony `Arc<PrefixMatcher>` w GUI i API pozostaje ten sam).
    rules: RwLock<RuleSet>,
    /// Cache wyników `lookup()` — eliminuje wielokrotne skanowanie setek regexów
    /// przy renderowaniu spotów klastra i mapy świata w każdej klatce.
    cache: Mutex<HashMap<String, Option<PrefixInfo>>>,
}

fn cty_match_to_info(
    clean: &str,
    lookup_call: &str,
    m: crate::core::reference::cty::CtyMatch,
) -> PrefixInfo {
    PrefixInfo {
        callsign: clean.to_string(),
        country: m.entity_name,
        arrl_prefix: m.primary_prefix,
        dxcc: m.dxcc.unwrap_or(0),
        cqz: m.cq_zone,
        ituz: m.itu_zone,
        continent: m.continent,
        latitude: m.latitude,
        longitude: m.longitude,
        // Jak w ścieżce starszej bazy: dla znaku dokładnego WPX liczony z pełnego
        // znaku, dla prefiksu — ze znaku po normalizacji.
        wpx_prefix: extract_wpx_prefix(if m.exact { clean } else { lookup_call }),
        province: None,
    }
}

fn parse_coord(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }
    let is_south_or_west =
        s.ends_with('S') || s.ends_with('W') || s.ends_with('s') || s.ends_with('w');
    let num_str = s.trim_matches(|c: char| c.is_alphabetic());
    let val: f64 = num_str.parse().unwrap_or(0.0);
    if is_south_or_west { -val } else { val }
}

/// Ekstrahuje prefiks WPX (np. SP6INA -> SP6, 3Z100POL -> 3Z100, W1AW -> W1, SP6INA/1 -> SP1, DL/SP6INA -> DL0)
pub fn extract_wpx_prefix(call: &str) -> String {
    let clean = call.trim().to_uppercase();
    let is_operational_suffix = |s: &str| -> bool {
        matches!(
            s,
            "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J"
        )
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
            if let Some(pos) = prefix_chars.iter().rposition(char::is_ascii_digit) {
                if let Some(digit) = p1.chars().next() {
                    prefix_chars[pos] = digit;
                }
                return prefix_chars.into_iter().collect();
            }
            return format!("{prefix0}{p1}");
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
    let mut seen_alpha = false;
    let mut last_digit_after_alpha = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_alphabetic() {
            seen_alpha = true;
        } else if b.is_ascii_digit() && seen_alpha {
            last_digit_after_alpha = Some(i);
        }
    }

    if let Some(idx) = last_digit_after_alpha {
        call[..=idx].to_string()
    } else {
        let mut res = call.to_string();
        res.push('0');
        res
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

        for r in rows {
            let r = match r {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Ignorowanie błędu w prefix.rs: {e}");
                    continue;
                }
            };
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

        for row_res in rows {
            let (mut info, raw_regex) = match row_res {
                Ok(val) => val,
                Err(e) => {
                    eprintln!("Ignorowanie błędu w prefix.rs: {e}");
                    continue;
                }
            };
            let pattern = if raw_regex.starts_with('^') {
                raw_regex
            } else {
                format!("^(?:{raw_regex})")
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

        for row_res in rows {
            let (mut info, raw_regex) = match row_res {
                Ok(val) => val,
                Err(e) => {
                    eprintln!("Ignorowanie błędu w prefix.rs: {e}");
                    continue;
                }
            };
            let pattern = if raw_regex.starts_with('^') {
                raw_regex
            } else {
                format!("^(?:{raw_regex})")
            };

            if let Ok(reg) = Regex::new(&pattern) {
                info.wpx_prefix = extract_wpx_prefix(&info.arrl_prefix);
                province_rules.push(CompiledRule { regex: reg, info });
            }
        }

        Ok(Self::with_rules(RuleSet {
            unique_calls,
            province_rules,
            country_rules,
            cty: None,
            source: PrefixSource::LegacyDatabase,
        }))
    }

    fn with_rules(rules: RuleSet) -> Self {
        Self {
            rules: RwLock::new(rules),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Tworzy pusty silnik dopasowywania prefiksów (np. gdy brak bazy danych)
    pub fn empty() -> Self {
        Self::with_rules(RuleSet::default())
    }

    /// Ładuje reguły z plików krajów: `cty.dat` (prefiksy, znaki dokładne,
    /// strefy, kontynent, współrzędne) oraz opcjonalnie `cty.csv` (kody encji
    /// ADIF DXCC). Bez `cty.csv` pole `dxcc` ma wartość 0 (nieznane).
    pub fn load_from_country_files(
        dat_path: &Path,
        csv_path: Option<&Path>,
    ) -> std::result::Result<(Self, CountryFileReport), crate::core::reference::cty::CtyError> {
        use crate::core::reference::cty;
        let mut data = cty::CtyData::load(dat_path)?;
        let mut report = CountryFileReport::default();
        if let Some(csv) = csv_path {
            let (rows, csv_issues) = cty::load_cty_csv(csv)?;
            report.entity_codes = Some(data.apply_entity_codes(&rows));
            report.csv_issues = csv_issues;
        }
        report.entities = data.entities().len();
        report.prefixes = data.prefix_count();
        report.exact_calls = data.exact_count();
        report.issues.clone_from(&data.issues);
        let entity_codes = report.entity_codes.is_some();
        Ok((
            Self::with_rules(RuleSet {
                cty: Some(data),
                source: PrefixSource::CountryFiles { entity_codes },
                ..RuleSet::default()
            }),
            report,
        ))
    }

    /// Zastępuje reguły regułami z `other` i czyści cache. Pozwala przeładować
    /// dane referencyjne we współdzielonym `Arc<PrefixMatcher>` bez restartu.
    pub fn replace_rules_from(&self, other: Self) {
        let new_rules = other
            .rules
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *self
            .rules
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = new_rules;
        self.clear_cache();
    }

    /// Aktualne źródło reguł.
    pub fn source(&self) -> PrefixSource {
        self.rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .source
    }

    /// `true`, gdy załadowano jakiekolwiek dane referencyjne.
    pub fn has_data(&self) -> bool {
        self.source() != PrefixSource::None
    }

    /// Znaki dokładne (wyjątki pełnych znaków) z aktualnego źródła, posortowane.
    pub fn exception_calls(&self) -> Vec<PrefixInfo> {
        let rules = self
            .rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out: Vec<PrefixInfo> = if let Some(cty) = &rules.cty {
            cty.exact_calls()
                .into_iter()
                .map(|m| {
                    let call = m.matched.clone();
                    cty_match_to_info(&call, &call, m)
                })
                .collect()
        } else {
            rules.unique_calls.values().cloned().collect()
        };
        out.sort_by(|a, b| a.callsign.cmp(&b.callsign));
        out
    }

    /// Normalizuje znak wywoławczy dla potrzeb dopasowania prefiksu (obsługa /P, /M, prefiksów gościnnych itp.)
    pub fn normalize_call_for_prefix(clean: &str) -> &str {
        let is_operational_suffix = |s: &str| -> bool {
            matches!(
                s,
                "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J"
            )
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

        // 2b. Przenośne stacje rosyjskie zmieniające okręg/kontynent (np. UA3ABC/9 -> UA9, RA9AAA/3 -> UA3)
        if p1.len() == 1
            && p1.as_bytes()[0].is_ascii_digit()
            && (p0.starts_with('R') || p0.starts_with('U'))
        {
            match p1 {
                "8" | "9" | "0" => return "UA9",
                "2" => return "UA2",
                "1" | "3" | "4" | "5" | "6" | "7" => return "UA3",
                _ => {}
            }
        }

        // 3. Domyślnie bierzemy dłuższą część (znak główny)
        if p0.len() >= p1.len() { p0 } else { p1 }
    }

    /// Wyszukuje kraj DXCC, strefy CQ/ITU i dane dla podanego znaku.
    ///
    /// Wyniki są zapamiętywane w cache (klucz = znak po normalizacji wielkości
    /// liter), dzięki czemu powtarzane wywołania dla tych samych znaków (np. przy
    /// renderowaniu spotów klastra co klatkę) nie skanują ponownie setek regexów.
    pub fn lookup(&self, callsign: &str) -> Option<PrefixInfo> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return None;
        }

        if let Some(cached) = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&clean)
        {
            return cached.clone();
        }

        let result = self.lookup_uncached(&clean);

        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(clean, result.clone());

        result
    }

    /// Czyści cache wyników wyszukiwania (wywołaj po przeładowaniu reguł prefiksów).
    pub fn clear_cache(&self) {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    fn lookup_uncached(&self, clean: &str) -> Option<PrefixInfo> {
        let rules = self
            .rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // 0. Pliki krajów (cty.dat): znak dokładny, potem najdłuższy prefiks.
        if let Some(cty) = &rules.cty {
            let lookup_call = Self::normalize_call_for_prefix(clean);
            return cty
                .resolve(clean, lookup_call)
                .map(|m| cty_match_to_info(clean, lookup_call, m));
        }

        // 1. Sprawdź UniqueCalls (dokładne dopasowanie)
        if let Some(info) = rules.unique_calls.get(clean) {
            let mut res = info.clone();
            res.callsign = clean.to_string();
            return Some(res);
        }

        // Znormalizuj znak do rozpoznania prefiksu (np. SP6INA/P -> SP6INA, OE3/SP6INA -> OE3, DL/SP6INA/M -> DL)
        let lookup_call = Self::normalize_call_for_prefix(clean);

        // 2. Sprawdź Province (specyficzne regiony)
        for rule in &rules.province_rules {
            if rule.regex.is_match(lookup_call) {
                let mut res = rule.info.clone();
                res.callsign = clean.to_string();
                res.wpx_prefix = extract_wpx_prefix(lookup_call);
                return Some(res);
            }
        }

        // 3. Sprawdź CountryDataEx
        for rule in &rules.country_rules {
            if rule.regex.is_match(lookup_call) {
                let mut res = rule.info.clone();
                res.callsign = clean.to_string();
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
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("SP6INA/P"),
            "SP6INA"
        );
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("SP6INA/M"),
            "SP6INA"
        );
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("SP6INA/QRP"),
            "SP6INA"
        );
        assert_eq!(PrefixMatcher::normalize_call_for_prefix("DL/SP6INA"), "DL");
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("OE3/SP6INA"),
            "OE3"
        );
        assert_eq!(
            PrefixMatcher::normalize_call_for_prefix("3D2/SP6INA/P"),
            "3D2"
        );
    }

    // Syntetyczne pliki krajów (blok prefiksów Q nie jest przydzielony żadnemu krajowi).
    const SYNTH_DAT: &str = "\
Alpha Land:               14:  27:  EU:   50.00:   -20.00:    -1.0:  QA:
    QA,QA2(15)[28],=QA1EXACT{AS};
Beta Isles:               3:   6:   NA:   20.00:   100.00:     8.0:  QB:
    QB;
";
    const SYNTH_CSV: &str = "\
QA,Alpha Land,901,EU,14,27,50.00,-20.00,-1.0,QA QA2(15)[28] =QA1EXACT{AS};
QB,Beta Isles,902,NA,3,6,20.00,100.00,8.0,QB;
";

    fn write_country_files(dir: &Path, dat: &str, csv: Option<&str>) {
        std::fs::write(dir.join("cty.dat"), dat).unwrap();
        if let Some(csv) = csv {
            std::fs::write(dir.join("cty.csv"), csv).unwrap();
        }
    }

    #[test]
    fn country_files_resolve_through_call_normalisation() {
        let dir = tempfile::tempdir().unwrap();
        write_country_files(dir.path(), SYNTH_DAT, Some(SYNTH_CSV));
        let (m, report) = PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap();
        assert_eq!(report.entities, 2);
        assert_eq!(report.exact_calls, 1);
        assert_eq!(report.entity_codes.as_ref().unwrap().matched, 2);
        assert_eq!(
            m.source(),
            PrefixSource::CountryFiles { entity_codes: true }
        );

        let info = m.lookup("qa5xyz/p").unwrap();
        assert_eq!(info.callsign, "QA5XYZ/P");
        assert_eq!(info.country, "Alpha Land");
        assert_eq!(info.arrl_prefix, "QA");
        assert_eq!(info.dxcc, 901);
        assert_eq!((info.cqz, info.ituz), (14, 27));
        assert_eq!(info.continent, "EU");
        assert!((info.latitude - 50.0).abs() < 1e-9);
        assert!((info.longitude - 20.0).abs() < 1e-9);
        assert_eq!(info.wpx_prefix, "QA5");
        assert_eq!(info.province, None);

        // Prefiks gościnny: QB/QA5XYZ -> QB.
        assert_eq!(m.lookup("QB/QA5XYZ").unwrap().country, "Beta Isles");
        // Nadpisanie stref dla dłuższego prefiksu.
        let z = m.lookup("QA2AA").unwrap();
        assert_eq!((z.cqz, z.ituz), (15, 28));
        // Znak dokładny z nadpisaniem kontynentu.
        assert_eq!(m.lookup("QA1EXACT").unwrap().continent, "AS");
        assert_eq!(m.lookup("X1ABC"), None);

        let ex = m.exception_calls();
        assert_eq!(ex.len(), 1);
        assert_eq!(ex[0].callsign, "QA1EXACT");
        assert_eq!(ex[0].dxcc, 901);
    }

    #[test]
    fn country_files_without_csv_report_unknown_entity_code() {
        let dir = tempfile::tempdir().unwrap();
        write_country_files(dir.path(), SYNTH_DAT, None);
        let (m, report) =
            PrefixMatcher::load_from_country_files(&dir.path().join("cty.dat"), None).unwrap();
        assert!(report.entity_codes.is_none());
        assert_eq!(
            m.source(),
            PrefixSource::CountryFiles {
                entity_codes: false
            }
        );
        assert_eq!(m.lookup("QA5XYZ").unwrap().dxcc, 0);
    }

    #[test]
    fn missing_country_files_are_errors_and_matcher_stays_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(PrefixMatcher::load_from_country_files(&dir.path().join("cty.dat"), None).is_err());
        write_country_files(dir.path(), SYNTH_DAT, None);
        assert!(
            PrefixMatcher::load_from_country_files(
                &dir.path().join("cty.dat"),
                Some(&dir.path().join("cty.csv"))
            )
            .is_err()
        );
        let m = PrefixMatcher::empty();
        assert!(!m.has_data());
        assert_eq!(m.source(), PrefixSource::None);
        assert_eq!(m.exception_calls(), Vec::<PrefixInfo>::new());
    }

    #[test]
    fn replace_rules_reloads_shared_matcher_and_clears_cache() {
        let dir = tempfile::tempdir().unwrap();
        let shared = std::sync::Arc::new(PrefixMatcher::empty());
        assert_eq!(shared.lookup("QA5XYZ"), None); // trafia do cache jako None

        write_country_files(dir.path(), SYNTH_DAT, Some(SYNTH_CSV));
        let (fresh, _) = PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap();
        shared.replace_rules_from(fresh);
        assert!(shared.has_data());
        assert_eq!(shared.lookup("QA5XYZ").unwrap().dxcc, 901);

        // Kolejna wersja danych zastępuje poprzednią.
        write_country_files(dir.path(), &SYNTH_DAT.replace("QA,QA2", "QA,QA5,QA2"), None);
        let (v2, _) =
            PrefixMatcher::load_from_country_files(&dir.path().join("cty.dat"), None).unwrap();
        shared.replace_rules_from(v2);
        assert_eq!(shared.lookup("QA5XYZ").unwrap().dxcc, 0);
        assert_eq!(shared.lookup("QA5XYZ").unwrap().arrl_prefix, "QA");
    }

    #[test]
    fn test_lookup_caches_and_clear_cache() {
        let matcher = PrefixMatcher::empty();

        // Brak reguł -> None, ale wynik trafia do cache.
        assert_eq!(matcher.lookup("SP6INA"), None);
        assert_eq!(matcher.lookup("sp6ina"), None); // ścieżka cache (normalizacja wielkości liter)
        assert_eq!(matcher.lookup(""), None);

        // clear_cache() musi być bezpieczne i nie zmieniać poprawności wyników.
        matcher.clear_cache();
        assert_eq!(matcher.lookup("SP6INA"), None);
    }
}
