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

/// Tabela starszej bazy referencyjnej.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyTable {
    UniqueCalls,
    CountryDataEx,
    Province,
}

/// Rodzaj problemu z rekordem starszej bazy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyRuleIssueKind {
    /// Wiersz nie dał się odczytać (np. niezgodny typ kolumny).
    UnreadableRow(String),
    /// Wzorzec nie kompiluje się nawet po normalizacji kotwic.
    InvalidPattern(String),
}

/// Typowane ostrzeżenie: rekord pominięty podczas ładowania starszej bazy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyRuleIssue {
    pub table: LegacyTable,
    /// Indeks rekordu w wyniku zapytania (od 0).
    pub index: usize,
    /// Nazwa encji/regionu, jeśli dostępna.
    pub label: String,
    pub kind: LegacyRuleIssueKind,
}

impl std::fmt::Display for LegacyRuleIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = match &self.kind {
            LegacyRuleIssueKind::UnreadableRow(e) => format!("unreadable row: {e}"),
            LegacyRuleIssueKind::InvalidPattern(e) => format!("invalid pattern: {e}"),
        };
        write!(f, "{:?}[{}] {}: {what}", self.table, self.index, self.label)
    }
}

/// Liczniki i ostrzeżenia z ładowania starszej bazy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyLoadReport {
    pub exact_calls: usize,
    pub country_rules: usize,
    pub province_rules: usize,
    /// Reguły, w których przetłumaczono kotwicę końca tekstu `\Z` na `\z`.
    pub normalized_anchors: usize,
    pub issues: Vec<LegacyRuleIssue>,
}

/// Tłumaczy historyczną kotwicę końca tekstu `\Z` (składnia PCRE/Python) na
/// `\z` obsługiwaną przez crate `regex`. Zmienia wyłącznie niepoprzedzoną
/// innym ukośnikiem sekwencję `\Z` poza klasą znaków; `\\Z` (literalny
/// ukośnik i litera Z) oraz zawartość `[...]` pozostają bez zmian.
/// Zwraca wzorzec i liczbę przetłumaczonych kotwic.
pub fn normalize_legacy_end_anchors(pattern: &str) -> (String, usize) {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    let mut in_class = false;
    let mut class_start = false;
    let mut count = 0;
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('Z') if !in_class => {
                    out.push_str("\\z");
                    count += 1;
                }
                Some(next) => {
                    out.push('\\');
                    out.push(next);
                }
                None => out.push('\\'),
            },
            '[' if !in_class => {
                in_class = true;
                class_start = true;
                out.push(c);
                continue;
            }
            ']' if in_class && !class_start => {
                in_class = false;
                out.push(c);
            }
            '^' if in_class && class_start => {
                out.push(c);
                continue;
            }
            _ => {
                out.push(c);
            }
        }
        class_start = false;
    }
    (out, count)
}

fn compile_legacy_rule(
    table: LegacyTable,
    index: usize,
    mut info: PrefixInfo,
    raw_regex: &str,
    rules: &mut Vec<CompiledRule>,
    report: &mut LegacyLoadReport,
) {
    let (normalized, anchors) = normalize_legacy_end_anchors(raw_regex);
    let pattern = if normalized.starts_with('^') {
        normalized
    } else {
        format!("^(?:{normalized})")
    };
    match Regex::new(&pattern) {
        Ok(regex) => {
            if anchors > 0 {
                report.normalized_anchors += 1;
            }
            info.wpx_prefix = extract_wpx_prefix(&info.arrl_prefix);
            rules.push(CompiledRule { regex, info });
            match table {
                LegacyTable::Province => report.province_rules += 1,
                _ => report.country_rules += 1,
            }
        }
        Err(e) => report.issues.push(LegacyRuleIssue {
            table,
            index,
            label: info
                .province
                .clone()
                .unwrap_or_else(|| info.country.clone()),
            kind: LegacyRuleIssueKind::InvalidPattern(e.to_string()),
        }),
    }
}

fn is_operational_suffix(s: &str) -> bool {
    matches!(
        s,
        "P" | "M" | "MM" | "AM" | "QRP" | "LGT" | "LH" | "B" | "R" | "A" | "J"
    )
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
    /// Ładuje reguły prefiksów ze starszej bazy referencyjnej SQLite.
    /// Problemy z poszczególnymi regułami są logowane jako ostrzeżenia;
    /// pełny raport zwraca [`Self::load_from_db_with_report`].
    pub fn load_from_db(db_path: impl AsRef<Path>) -> Result<Self> {
        let (matcher, report) = Self::load_from_db_with_report(db_path)?;
        for issue in &report.issues {
            log::warn!("legacy prefix rule skipped: {issue}");
        }
        Ok(matcher)
    }

    /// Jak [`Self::load_from_db`], ale zwraca też liczniki reguł i typowane
    /// ostrzeżenia o pominiętych rekordach.
    pub fn load_from_db_with_report(db_path: impl AsRef<Path>) -> Result<(Self, LegacyLoadReport)> {
        let mut report = LegacyLoadReport::default();
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

        for (index, r) in rows.enumerate() {
            match r {
                Ok(r) => {
                    unique_calls.insert(r.callsign.to_uppercase(), r);
                }
                Err(e) => report.issues.push(LegacyRuleIssue {
                    table: LegacyTable::UniqueCalls,
                    index,
                    label: String::new(),
                    kind: LegacyRuleIssueKind::UnreadableRow(e.to_string()),
                }),
            }
        }
        report.exact_calls = unique_calls.len();

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

        for (index, row_res) in rows.enumerate() {
            match row_res {
                Ok((info, raw_regex)) => {
                    compile_legacy_rule(
                        LegacyTable::CountryDataEx,
                        index,
                        info,
                        &raw_regex,
                        &mut country_rules,
                        &mut report,
                    );
                }
                Err(e) => report.issues.push(LegacyRuleIssue {
                    table: LegacyTable::CountryDataEx,
                    index,
                    label: String::new(),
                    kind: LegacyRuleIssueKind::UnreadableRow(e.to_string()),
                }),
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

        for (index, row_res) in rows.enumerate() {
            match row_res {
                Ok((info, raw_regex)) => {
                    compile_legacy_rule(
                        LegacyTable::Province,
                        index,
                        info,
                        &raw_regex,
                        &mut province_rules,
                        &mut report,
                    );
                }
                Err(e) => report.issues.push(LegacyRuleIssue {
                    table: LegacyTable::Province,
                    index,
                    label: String::new(),
                    kind: LegacyRuleIssueKind::UnreadableRow(e.to_string()),
                }),
            }
        }

        Ok((
            Self::with_rules(RuleSet {
                unique_calls,
                province_rules,
                country_rules,
                cty: None,
                source: PrefixSource::LegacyDatabase,
            }),
            report,
        ))
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

    /// Usuwa wyłącznie końcowe sufiksy operacyjne (/P, /M, /QRP…), które nie
    /// zmieniają lokalizacji stacji. Prefiksy gościnne i zmiany okręgu (/9)
    /// pozostają bez zmian.
    pub fn strip_operational_suffixes(clean: &str) -> &str {
        let mut end = clean.len();
        while let Some(pos) = clean[..end].rfind('/') {
            if is_operational_suffix(&clean[pos + 1..end]) {
                end = pos;
            } else {
                break;
            }
        }
        &clean[..end]
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

        // 0. Pliki krajów (cty.dat): znak dokładny (pełny, potem bez sufiksów
        // operacyjnych typu /P), następnie najdłuższy prefiks.
        if let Some(cty) = &rules.cty {
            let lookup_call = Self::normalize_call_for_prefix(clean);
            let base = Self::strip_operational_suffixes(clean);
            return cty
                .resolve_exact(clean)
                .or_else(|| (base != clean).then(|| cty.resolve_exact(base)).flatten())
                .or_else(|| cty.resolve_prefix(lookup_call))
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
    fn legacy_end_anchor_is_normalized_only_where_it_is_an_anchor() {
        // Kotwica końca tekstu.
        assert_eq!(
            normalize_legacy_end_anchors(r"G\Z"),
            (r"G\z".to_string(), 1)
        );
        assert_eq!(
            normalize_legacy_end_anchors(r"Q\Z|QA[0-9].*|QB\Z"),
            (r"Q\z|QA[0-9].*|QB\z".to_string(), 2)
        );
        // Literalny ukośnik, po którym następuje litera Z — bez zmian.
        assert_eq!(
            normalize_legacy_end_anchors(r"Q\\Z"),
            (r"Q\\Z".to_string(), 0)
        );
        // Wewnątrz klasy znaków — bez zmian (także z ']' na początku i negacją).
        assert_eq!(
            normalize_legacy_end_anchors(r"[\Z]"),
            (r"[\Z]".to_string(), 0)
        );
        assert_eq!(
            normalize_legacy_end_anchors(r"[]\Z]"),
            (r"[]\Z]".to_string(), 0)
        );
        assert_eq!(
            normalize_legacy_end_anchors(r"[^]\Z]"),
            (r"[^]\Z]".to_string(), 0)
        );
        // Escapowany ']' nie zamyka klasy; kotwica po klasie jest tłumaczona.
        assert_eq!(
            normalize_legacy_end_anchors(r"[^\]]\Z"),
            (r"[^\]]\z".to_string(), 1)
        );
        // Inne sekwencje i wzorce bez kotwic pozostają identyczne.
        for p in [r"\d+", r"\bQ", r"2[BEGORQV].*", r"Q\.", r"Q\z", ""] {
            assert_eq!(normalize_legacy_end_anchors(p), (p.to_string(), 0));
        }
        // Samotny ukośnik na końcu nie powoduje paniki.
        assert_eq!(normalize_legacy_end_anchors("Q\\"), ("Q\\".to_string(), 0));
    }

    fn synthetic_legacy_db(rows_sql: &str) -> String {
        let uri = format!(
            "file:legacy_rules_{}_{}?mode=memory&cache=shared",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let conn = Connection::open_with_flags(
            &uri,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE UniqueCalls (
                Callsign TEXT, Country TEXT, ARRLPrefix TEXT, DXCC TEXT, Continent TEXT,
                CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT, Enable TEXT);
             CREATE TABLE CountryDataEx (
                Country TEXT, ARRLPrefix TEXT, DXCC INTEGER, Continent TEXT,
                CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT,
                PrefixList TEXT, EndDate TEXT);
             CREATE TABLE Province (
                Country TEXT, ARRLPrefix TEXT, DXCC TEXT, Continent TEXT,
                CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT,
                PrefixList TEXT, Comment TEXT, EndDate TEXT);
             {rows_sql}"
        ))
        .unwrap();
        // Połączenie musi żyć, aby baza w pamięci istniała podczas ładowania.
        std::mem::forget(conn);
        uri
    }

    #[test]
    fn legacy_loader_compiles_anchored_rules_and_reports_invalid_ones() {
        let uri = synthetic_legacy_db(
            r"INSERT INTO CountryDataEx VALUES
                ('Alpha Land', 'QA', 901, 'EU', 14, 27, '50.0N', '20.0E', 'QA\Z|QA[0-9].*', ''),
                ('Beta Isles', 'QB', 902, 'NA', 3, 6, '20.0N', '100.0W', 'QB(', ''),
                ('Gamma Rock', 'QG', 903, 'AF', 33, 37, '10.0S', '15.0E', 'QG\\Z.*', ''),
                ('Retired',    'QR', 904, 'EU', 14, 27, '50.0N', '20.0E', 'QR.*', '2001-01-01');
              INSERT INTO Province VALUES
                ('Alpha Land', 'QA', '901', 'EU', 15, 28, '51.0N', '21.0E', 'QA9[\Z]', 'Alpha North', ''),
                ('Alpha Land', 'QA', '901', 'EU', 16, 29, '49.0N', '19.0E', 'QA8.*', 'Alpha South', '');
              INSERT INTO UniqueCalls VALUES
                ('QA1X', 'Alpha Land', 'QA', '901', 'EU', 14, 27, '50.0N', '20.0E', '1'),
                ('QA2X', 'Alpha Land', 'QA', '901', 'EU', 14, 'bad', '50.0N', '20.0E', '1');",
        );
        let (m, report) = PrefixMatcher::load_from_db_with_report(&uri).unwrap();
        assert_eq!(report.country_rules, 2, "{report:?}");
        assert_eq!(report.province_rules, 1, "{report:?}");
        assert_eq!(report.exact_calls, 1, "{report:?}");
        assert_eq!(report.normalized_anchors, 1);
        let kinds: Vec<(LegacyTable, &str, bool)> = report
            .issues
            .iter()
            .map(|i| {
                (
                    i.table,
                    i.label.as_str(),
                    matches!(i.kind, LegacyRuleIssueKind::InvalidPattern(_)),
                )
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                (LegacyTable::UniqueCalls, "", false),
                (LegacyTable::CountryDataEx, "Beta Isles", true),
                (LegacyTable::Province, "Alpha North", true),
            ]
        );

        // Kotwica \Z działa jak koniec tekstu: sam prefiks "QA" pasuje.
        assert_eq!(m.lookup("QA").unwrap().country, "Alpha Land");
        assert_eq!(m.lookup("QA5ZZ").unwrap().country, "Alpha Land");
        assert_eq!(
            m.lookup("QA8ZZ").unwrap().province.as_deref(),
            Some("Alpha South")
        );
        // Literalny "\Z" w regule Gamma pozostał literalny: QG… nie pasuje.
        assert_eq!(m.lookup("QG1AB"), None);
        // Pominięta reguła Beta nie jest dopasowywana; wycofana reguła też nie.
        assert_eq!(m.lookup("QB1AB"), None);
        assert_eq!(m.lookup("QR1AB"), None);
    }

    #[test]
    fn strip_operational_suffixes_keeps_location_parts() {
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QA1AB/P"),
            "QA1AB"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QA1AB/QRP/M"),
            "QA1AB"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QA1AB/9"),
            "QA1AB/9"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QB/QA1AB"),
            "QB/QA1AB"
        );
        assert_eq!(
            PrefixMatcher::strip_operational_suffixes("QB/QA1AB/P"),
            "QB/QA1AB"
        );
        assert_eq!(PrefixMatcher::strip_operational_suffixes("QA1AB"), "QA1AB");
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
        // Znak dokładny z nadpisaniem kontynentu — także z sufiksem operacyjnym.
        assert_eq!(m.lookup("QA1EXACT").unwrap().continent, "AS");
        assert_eq!(m.lookup("QA1EXACT/P").unwrap().continent, "AS");
        assert_eq!(m.lookup("QA1EXACT/QRP/M").unwrap().continent, "AS");
        // Zmiana okręgu i prefiks gościnny nie dziedziczą znaku dokładnego.
        assert_eq!(m.lookup("QA1EXACT/2").unwrap().continent, "EU");
        assert_eq!(m.lookup("QB/QA1EXACT").unwrap().country, "Beta Isles");
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
