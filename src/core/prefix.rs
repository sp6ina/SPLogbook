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
    /// Nazwa jednostki administracyjnej (prowincji, stanu, oblastu), jeśli znana.
    pub province: Option<String>,
    /// Kod jednostki administracyjnej z reguły użytkownika, jeśli znany.
    #[serde(default)]
    pub subdivision_code: Option<String>,
    /// Dokładność współrzędnych — GUI i API nie mogą jej zawyżać.
    #[serde(default)]
    pub location_precision: LocationPrecision,
}

/// Źródło i dokładność współrzędnych w [`PrefixInfo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LocationPrecision {
    /// Brak wiarygodnych współrzędnych.
    #[default]
    Unknown,
    /// Środek encji DXCC (pliki krajów).
    EntityCentroid,
    /// Współrzędne z nadpisania `<lat/lon>` w pliku krajów.
    CountryFileOverride,
    /// Współrzędne ze starszej bazy na poziomie encji.
    LegacyEntity,
    /// Współrzędne regionu ze starszej bazy (warstwa zgodności).
    LegacyRegionalRule,
    /// Współrzędne z reguły regionalnej zaimportowanej przez użytkownika.
    UserRegionalRule,
    /// Współrzędne z jawnego nadpisania użytkownika.
    UserOverride,
    /// Dokładna lokalizacja z danych QSO (lokator / współrzędne). Ustawiana
    /// przez warstwę QSO, nigdy przez dopasowanie prefiksu.
    ExactQsoLocation,
}

/// Warstwa reguł użytkownika, której dotyczy konflikt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleLayer {
    UserOverride,
    UserRegional,
}

/// Konflikt wykryty podczas rozstrzygania znaku: reguła nie została użyta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerConflict {
    /// Kilka reguł wzorca o tym samym najwyższym priorytecie.
    AmbiguousPatterns { layer: RuleLayer, lines: Vec<usize> },
    /// Reguła regionalna wskazuje inną encję niż plik krajów (źródło nadrzędne).
    EntityMismatch {
        layer: RuleLayer,
        line: usize,
        rule_dxcc: u32,
        resolved_dxcc: u32,
    },
}

/// Wynik rozstrzygnięcia znaku wraz z konfliktami reguł użytkownika.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resolution {
    pub info: Option<PrefixInfo>,
    pub conflicts: Vec<LayerConflict>,
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
    /// Prefiksy główne encji dodane jako niejawne aliasy (I6).
    pub implicit_primary_prefixes: usize,
    /// Domyślne atrybuty encji zastąpione wartościami z oficjalnej listy DXCC (D-ENT).
    pub official_attribute_changes: Vec<crate::core::reference::cty::OfficialAttributeChange>,
}

#[derive(Default)]
struct RuleSet {
    unique_calls: HashMap<String, PrefixInfo>,
    province_rules: Vec<CompiledRule>,
    country_rules: Vec<CompiledRule>,
    cty: Option<crate::core::reference::cty::CtyData>,
    regional: Option<crate::core::reference::rules::RuleBook>,
    overrides: Option<crate::core::reference::rules::RuleBook>,
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
        subdivision_code: None,
        location_precision: if m.coordinates_overridden {
            LocationPrecision::CountryFileOverride
        } else {
            LocationPrecision::EntityCentroid
        },
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
    /// Znak w `UniqueCalls` pusty po przycięciu albo z niedozwolonymi
    /// znakami (dozwolone: A–Z, 0–9, `/`); wartość jak w bazie.
    InvalidCallsign(String),
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
            LegacyRuleIssueKind::InvalidCallsign(c) => format!("invalid callsign: {c:?}"),
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
    /// Wiersze `UniqueCalls` powtarzające już wczytany znak (rozstrzygnięte
    /// deterministycznie, zob. [`PrefixMatcher::load_from_db_with_report`]).
    pub duplicate_calls: usize,
    pub issues: Vec<LegacyRuleIssue>,
}

/// Klucz rozstrzygania powtórzonych znaków w `UniqueCalls`: najpóźniejsza
/// znana data działania (`YYYY-MM-DD`, pusta gdy brak dat), potem `rowid`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LegacyActivity {
    last_date: String,
    rowid: i64,
}

/// Data `YYYY-MM-DD` z początku pola daty starszej bazy (`YYYY-MM-DD HH:MM:SS`)
/// albo `None`, gdy pole jest puste lub ma inny format.
fn legacy_date(field: Option<&str>) -> Option<&str> {
    let date = field?.trim().get(..10)?;
    let b = date.as_bytes();
    let shape = b.iter().enumerate().all(|(i, c)| match i {
        4 | 7 => *c == b'-',
        _ => c.is_ascii_digit(),
    });
    shape.then_some(date)
}

/// Najpóźniejsza z dat początku i końca (porównanie leksykalne `YYYY-MM-DD`);
/// pusty napis, gdy wiersz nie ma żadnej poprawnej daty.
fn legacy_activity_date(start: Option<&str>, end: Option<&str>) -> String {
    legacy_date(start)
        .max(legacy_date(end))
        .unwrap_or_default()
        .to_string()
}

fn is_valid_legacy_callsign(call: &str) -> bool {
    !call.is_empty() && call.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
        [table, column],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
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

        // 1. UniqueCalls. Wiersze mogą mieć okres działania (StartDate/EndDate);
        // starsze układy tabeli tych kolumn nie mają. `rowid` jest aliasem `_id`.
        let date_column = |name: &'static str| -> Result<&'static str> {
            Ok(if table_has_column(&conn, "UniqueCalls", name)? {
                name
            } else {
                "NULL"
            })
        };
        let sql = format!(
            "SELECT Callsign, Country, ARRLPrefix, DXCC, Continent, CQZone, ITUZone, Latitude, Longitude,
                    rowid, {}, {}
             FROM UniqueCalls WHERE Enable != '0' OR Enable IS NULL ORDER BY rowid",
            date_column("StartDate")?,
            date_column("EndDate")?
        );
        let mut unique_calls: HashMap<String, (PrefixInfo, LegacyActivity)> = HashMap::new();
        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map([], |row| {
            let call: String = row.get(0)?;
            let dxcc_str: String = row.get(3)?;
            let dxcc: u32 = dxcc_str.parse().unwrap_or(0);
            let lat_str: String = row.get(7)?;
            let lon_str: String = row.get(8)?;
            let start: Option<String> = row.get(10)?;
            let end: Option<String> = row.get(11)?;

            Ok((
                PrefixInfo {
                    callsign: call,
                    country: row.get(1)?,
                    arrl_prefix: row.get(2)?,
                    dxcc,
                    continent: row.get(4)?,
                    cqz: row.get(5)?,
                    ituz: row.get(6)?,
                    latitude: parse_coord(&lat_str),
                    longitude: parse_coord(&lon_str),
                    wpx_prefix: String::new(),
                    province: None,
                    subdivision_code: None,
                    location_precision: LocationPrecision::LegacyEntity,
                },
                LegacyActivity {
                    last_date: legacy_activity_date(start.as_deref(), end.as_deref()),
                    rowid: row.get(9)?,
                },
            ))
        })?;

        for (index, r) in rows.enumerate() {
            match r {
                Ok((mut info, activity)) => {
                    // I1: klucz bez otaczających białych znaków; pusty lub z
                    // niedozwolonymi znakami (np. `?`) jest odrzucany i zgłaszany.
                    let key = info.callsign.trim().to_uppercase();
                    if !is_valid_legacy_callsign(&key) {
                        report.issues.push(LegacyRuleIssue {
                            table: LegacyTable::UniqueCalls,
                            index,
                            label: info.callsign.clone(),
                            kind: LegacyRuleIssueKind::InvalidCallsign(info.callsign),
                        });
                        continue;
                    }
                    info.wpx_prefix = extract_wpx_prefix(&key);
                    info.callsign.clone_from(&key);
                    // I2: przy powtórzonym znaku wygrywa wiersz o najpóźniejszej
                    // znanej dacie działania, przy remisie — późniejszy `rowid`.
                    match unique_calls.entry(key) {
                        std::collections::hash_map::Entry::Vacant(slot) => {
                            slot.insert((info, activity));
                        }
                        std::collections::hash_map::Entry::Occupied(mut slot) => {
                            report.duplicate_calls += 1;
                            if activity > slot.get().1 {
                                slot.insert((info, activity));
                            }
                        }
                    }
                }
                Err(e) => report.issues.push(LegacyRuleIssue {
                    table: LegacyTable::UniqueCalls,
                    index,
                    label: String::new(),
                    kind: LegacyRuleIssueKind::UnreadableRow(e.to_string()),
                }),
            }
        }
        let unique_calls: HashMap<String, PrefixInfo> = unique_calls
            .into_iter()
            .map(|(k, (info, _))| (k, info))
            .collect();
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
                    subdivision_code: None,
                    location_precision: LocationPrecision::LegacyEntity,
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

        // 3. Province (Oblasty, stany itp.). Nazwa regionu jest zapisana w
        // kolumnie `Country` (np. „Russia, Chechen Rep.”); `Comment` jest pusta
        // we wszystkich wierszach dostarczanej bazy (I3).
        let mut province_rules = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT Country, ARRLPrefix, DXCC, Continent, CQZone, ITUZone, Latitude, Longitude, PrefixList
             FROM Province WHERE EndDate = '' OR EndDate IS NULL"
        )?;

        let rows = stmt.query_map([], |row| {
            let lat_str: String = row.get(6)?;
            let lon_str: String = row.get(7)?;
            let prefix_list: String = row.get(8)?;
            let country: String = row.get(0)?;
            let region = country.trim();
            let province = (!region.is_empty()).then(|| region.to_string());

            Ok((
                PrefixInfo {
                    callsign: String::new(),
                    country,
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
                    province,
                    subdivision_code: None,
                    location_precision: LocationPrecision::LegacyRegionalRule,
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
                regional: None,
                overrides: None,
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
            // D-ENT: oficjalne atrybuty encji (lista ARRL DXCC) mają pierwszeństwo.
            report.official_attribute_changes = data.apply_official_entity_attributes();
        }
        report.implicit_primary_prefixes = data.implicit_primary_count();
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
        self.resolve_clean(clean).info
    }

    /// Rozstrzyga znak bez użycia cache i zwraca też konflikty reguł
    /// użytkownika (reguły, które z powodu konfliktu nie zostały zastosowane).
    pub fn lookup_detailed(&self, callsign: &str) -> Resolution {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return Resolution::default();
        }
        self.resolve_clean(&clean)
    }

    /// Kolejność warstw:
    /// 1. jawne nadpisanie użytkownika;
    /// 2. znak dokładny z plików krajów;
    /// 3. dokładna reguła regionalna użytkownika;
    /// 4. najdłuższy prefiks z plików krajów, doprecyzowany regułą regionalną
    ///    użytkownika albo (w okresie przejściowym) regułą regionalną starszej bazy;
    /// 5. starsza baza jako warstwa zgodności, gdy pliki krajów nic nie dają;
    /// 6. sama reguła użytkownika z kodem encji;
    /// 7. brak wyniku.
    fn resolve_clean(&self, clean: &str) -> Resolution {
        let rules = self
            .rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let lookup_call = Self::normalize_call_for_prefix(clean);
        let base = Self::strip_operational_suffixes(clean);
        let mut conflicts = Vec::new();

        let mut info =
            Self::resolve_without_overrides(&rules, clean, lookup_call, base, &mut conflicts);

        if let Some(book) = &rules.overrides {
            let rule = book.match_exact(clean, base).or_else(|| {
                pick_rule(
                    book.match_prefix(lookup_call),
                    RuleLayer::UserOverride,
                    &mut conflicts,
                )
            });
            if let Some(rule) = rule {
                info = apply_rule(
                    info,
                    rule,
                    rules.cty.as_ref(),
                    clean,
                    lookup_call,
                    LocationPrecision::UserOverride,
                );
            }
        }

        Resolution { info, conflicts }
    }

    fn resolve_without_overrides(
        rules: &RuleSet,
        clean: &str,
        lookup_call: &str,
        base: &str,
        conflicts: &mut Vec<LayerConflict>,
    ) -> Option<PrefixInfo> {
        let regional = rules.regional.as_ref();

        if let Some(cty) = &rules.cty {
            // 2. Znak dokładny z plików krajów.
            let exact = cty
                .resolve_exact(clean)
                .or_else(|| (base != clean).then(|| cty.resolve_exact(base)).flatten());
            if let Some(m) = exact {
                return Some(cty_match_to_info(clean, lookup_call, m));
            }
            let prefix_info = cty
                .resolve_prefix(lookup_call)
                .map(|m| cty_match_to_info(clean, lookup_call, m));

            // 3. Dokładna reguła regionalna użytkownika (może zmienić encję).
            if let Some(rule) = regional.and_then(|b| b.match_exact(clean, base)) {
                return apply_rule(
                    prefix_info,
                    rule,
                    Some(cty),
                    clean,
                    lookup_call,
                    LocationPrecision::UserRegionalRule,
                );
            }

            // 4. Najdłuższy prefiks + doprecyzowanie regionalne.
            if let Some(info) = prefix_info {
                if let Some(book) = regional
                    && let Some(rule) = pick_rule(
                        book.match_prefix(lookup_call),
                        RuleLayer::UserRegional,
                        conflicts,
                    )
                {
                    match rule.attributes.dxcc {
                        Some(d) if d != info.dxcc => {
                            conflicts.push(LayerConflict::EntityMismatch {
                                layer: RuleLayer::UserRegional,
                                line: rule.line,
                                rule_dxcc: d,
                                resolved_dxcc: info.dxcc,
                            });
                        }
                        _ => {
                            return apply_rule(
                                Some(info),
                                rule,
                                Some(cty),
                                clean,
                                lookup_call,
                                LocationPrecision::UserRegionalRule,
                            );
                        }
                    }
                }
                if let Some(refined) = legacy_regional_refinement(rules, &info, lookup_call) {
                    return Some(refined);
                }
                return Some(info);
            }

            // 5. Warstwa zgodności: starsza baza.
            if let Some(info) = legacy_lookup(rules, clean, lookup_call) {
                return Some(info);
            }

            // 6. Sama reguła regionalna z kodem encji.
            if let Some(book) = regional
                && let Some(rule) = pick_rule(
                    book.match_prefix(lookup_call),
                    RuleLayer::UserRegional,
                    conflicts,
                )
            {
                return apply_rule(
                    None,
                    rule,
                    Some(cty),
                    clean,
                    lookup_call,
                    LocationPrecision::UserRegionalRule,
                );
            }
            return None;
        }

        // Bez plików krajów: dotychczasowa ścieżka starszej bazy, potem reguły użytkownika.
        let legacy = legacy_lookup(rules, clean, lookup_call);
        let Some(book) = regional else {
            return legacy;
        };
        if let Some(rule) = book.match_exact(clean, base) {
            return apply_rule(
                legacy,
                rule,
                None,
                clean,
                lookup_call,
                LocationPrecision::UserRegionalRule,
            );
        }
        let Some(rule) = pick_rule(
            book.match_prefix(lookup_call),
            RuleLayer::UserRegional,
            conflicts,
        ) else {
            return legacy;
        };
        match (rule.attributes.dxcc, &legacy) {
            (Some(d), Some(l)) if d != l.dxcc => {
                conflicts.push(LayerConflict::EntityMismatch {
                    layer: RuleLayer::UserRegional,
                    line: rule.line,
                    rule_dxcc: d,
                    resolved_dxcc: l.dxcc,
                });
                legacy
            }
            _ => apply_rule(
                legacy,
                rule,
                None,
                clean,
                lookup_call,
                LocationPrecision::UserRegionalRule,
            ),
        }
    }

    /// Dołącza reguły regionalne użytkownika (zastępuje poprzednie).
    #[must_use]
    pub fn with_regional_rules(self, book: crate::core::reference::rules::RuleBook) -> Self {
        self.rules
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .regional = Some(book);
        self.clear_cache();
        self
    }

    /// Dołącza jawne nadpisania użytkownika (zastępuje poprzednie).
    #[must_use]
    pub fn with_overrides(self, book: crate::core::reference::rules::RuleBook) -> Self {
        self.rules
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .overrides = Some(book);
        self.clear_cache();
        self
    }

    /// Dołącza reguły starszej bazy jako warstwę zgodności (okres przejściowy).
    #[must_use]
    pub fn with_legacy_compatibility(self, legacy: Self) -> Self {
        let legacy = legacy
            .rules
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        {
            let mut rules = self
                .rules
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            rules.unique_calls = legacy.unique_calls;
            rules.province_rules = legacy.province_rules;
            rules.country_rules = legacy.country_rules;
            if rules.source == PrefixSource::None {
                rules.source = legacy.source;
            }
        }
        self.clear_cache();
        self
    }

    /// Opis załadowanych warstw.
    pub fn layers(&self) -> LayerSummary {
        let rules = self
            .rules
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        LayerSummary {
            source: rules.source,
            country_files: rules.cty.is_some(),
            legacy_rules: rules.unique_calls.len()
                + rules.province_rules.len()
                + rules.country_rules.len(),
            regional_rules: rules
                .regional
                .as_ref()
                .map_or(0, crate::core::reference::rules::RuleBook::len),
            override_rules: rules
                .overrides
                .as_ref()
                .map_or(0, crate::core::reference::rules::RuleBook::len),
        }
    }
}

/// Liczba reguł w poszczególnych warstwach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayerSummary {
    pub source: PrefixSource,
    pub country_files: bool,
    pub legacy_rules: usize,
    pub regional_rules: usize,
    pub override_rules: usize,
}

fn pick_rule<'a>(
    m: crate::core::reference::rules::RuleMatch<'a>,
    layer: RuleLayer,
    conflicts: &mut Vec<LayerConflict>,
) -> Option<&'a crate::core::reference::rules::Rule> {
    use crate::core::reference::rules::RuleMatch;
    match m {
        RuleMatch::None => None,
        RuleMatch::Matched(r) => Some(r),
        RuleMatch::Conflict(rs) => {
            conflicts.push(LayerConflict::AmbiguousPatterns {
                layer,
                lines: rs.iter().map(|r| r.line).collect(),
            });
            None
        }
    }
}

/// Nakłada atrybuty reguły na wynik niższych warstw. Kod encji z reguły
/// zmienia encję (nazwa, prefiks i wartości domyślne z pliku krajów, jeśli
/// dostępny); dokładność współrzędnych rośnie tylko, gdy reguła je podaje.
fn apply_rule(
    base: Option<PrefixInfo>,
    rule: &crate::core::reference::rules::Rule,
    cty: Option<&crate::core::reference::cty::CtyData>,
    clean: &str,
    lookup_call: &str,
    precision: LocationPrecision,
) -> Option<PrefixInfo> {
    let a = &rule.attributes;
    let mut info = match (base, a.dxcc) {
        (Some(b), Some(d)) if b.dxcc != d => entity_info(cty, d, clean, lookup_call),
        (Some(b), _) => b,
        (None, Some(d)) => entity_info(cty, d, clean, lookup_call),
        (None, None) => return None,
    };
    if let Some(z) = a.cq_zone {
        info.cqz = z;
    }
    if let Some(z) = a.itu_zone {
        info.ituz = z;
    }
    if let Some(c) = &a.continent {
        info.continent.clone_from(c);
    }
    if let Some((lat, lon)) = a.coordinates {
        info.latitude = lat;
        info.longitude = lon;
        info.location_precision = precision;
    }
    if a.subdivision_name.is_some() {
        info.province.clone_from(&a.subdivision_name);
    }
    if a.subdivision_code.is_some() {
        info.subdivision_code.clone_from(&a.subdivision_code);
    }
    Some(info)
}

/// Dane encji o podanym kodzie z plików krajów; bez nich — tylko kod, bez
/// współrzędnych (dokładność `Unknown`).
fn entity_info(
    cty: Option<&crate::core::reference::cty::CtyData>,
    dxcc: u32,
    clean: &str,
    lookup_call: &str,
) -> PrefixInfo {
    let entity = cty.and_then(|c| c.entity_for_dxcc(dxcc));
    match entity {
        Some(e) => PrefixInfo {
            callsign: clean.to_string(),
            country: e.name.clone(),
            arrl_prefix: e.primary_prefix.clone(),
            dxcc,
            cqz: e.cq_zone,
            ituz: e.itu_zone,
            continent: e.continent.clone(),
            latitude: e.latitude,
            longitude: e.longitude,
            wpx_prefix: extract_wpx_prefix(lookup_call),
            province: None,
            subdivision_code: None,
            location_precision: LocationPrecision::EntityCentroid,
        },
        None => PrefixInfo {
            callsign: clean.to_string(),
            country: String::new(),
            arrl_prefix: String::new(),
            dxcc,
            cqz: 0,
            ituz: 0,
            continent: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            wpx_prefix: extract_wpx_prefix(lookup_call),
            province: None,
            subdivision_code: None,
            location_precision: LocationPrecision::Unknown,
        },
    }
}

/// Dotychczasowa ścieżka starszej bazy (UniqueCalls → Province → CountryDataEx).
fn legacy_lookup(rules: &RuleSet, clean: &str, lookup_call: &str) -> Option<PrefixInfo> {
    if let Some(info) = rules.unique_calls.get(clean) {
        let mut res = info.clone();
        res.callsign = clean.to_string();
        return Some(res);
    }
    for rule in rules.province_rules.iter().chain(&rules.country_rules) {
        if rule.regex.is_match(lookup_call) {
            let mut res = rule.info.clone();
            res.callsign = clean.to_string();
            res.wpx_prefix = extract_wpx_prefix(lookup_call);
            return Some(res);
        }
    }
    None
}

/// Doprecyzowanie wyniku z plików krajów regułą regionalną starszej bazy tej
/// samej encji (strefy, współrzędne, nazwa regionu) — warstwa zgodności.
fn legacy_regional_refinement(
    rules: &RuleSet,
    info: &PrefixInfo,
    lookup_call: &str,
) -> Option<PrefixInfo> {
    let rule = rules
        .province_rules
        .iter()
        .find(|r| r.info.dxcc == info.dxcc && info.dxcc != 0 && r.regex.is_match(lookup_call))?;
    let mut refined = info.clone();
    refined.cqz = rule.info.cqz;
    refined.ituz = rule.info.ituz;
    refined.continent.clone_from(&rule.info.continent);
    refined.latitude = rule.info.latitude;
    refined.longitude = rule.info.longitude;
    refined.province.clone_from(&rule.info.province);
    refined.location_precision = LocationPrecision::LegacyRegionalRule;
    Some(refined)
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
                ('Alpha Land, North', 'QA', '901', 'EU', 15, 28, '51.0N', '21.0E', 'QA9[\Z]', '', ''),
                ('Alpha Land, South', 'QA', '901', 'EU', 16, 29, '49.0N', '19.0E', 'QA8.*', '', '');
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
                (LegacyTable::Province, "Alpha Land, North", true),
            ]
        );

        // Kotwica \Z działa jak koniec tekstu: sam prefiks "QA" pasuje.
        assert_eq!(m.lookup("QA").unwrap().country, "Alpha Land");
        assert_eq!(m.lookup("QA5ZZ").unwrap().country, "Alpha Land");
        assert_eq!(
            m.lookup("QA8ZZ").unwrap().province.as_deref(),
            Some("Alpha Land, South")
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

    fn rule_book(body: &str) -> crate::core::reference::rules::RuleBook {
        let text = format!(
            "#splogbook-rules,1\nrule_type,callsign_or_prefix,dxcc_entity_code,subdivision_code,subdivision_name,cq_zone,itu_zone,continent,latitude,longitude,priority\n{body}"
        );
        let parsed = crate::core::reference::rules::parse_rules(&text);
        assert!(parsed.is_valid(), "{:?}", parsed.issues);
        crate::core::reference::rules::RuleBook::from_rules(parsed.rules)
    }

    fn cty_matcher() -> PrefixMatcher {
        let dir = tempfile::tempdir().unwrap();
        write_country_files(dir.path(), SYNTH_DAT, Some(SYNTH_CSV));
        PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap()
        .0
    }

    #[test]
    fn country_file_results_report_entity_or_override_precision() {
        let m = cty_matcher();
        assert_eq!(
            m.lookup("QA5XYZ").unwrap().location_precision,
            LocationPrecision::EntityCentroid
        );
        let empty = PrefixMatcher::empty();
        assert_eq!(empty.lookup("QA5XYZ"), None);
    }

    #[test]
    fn regional_prefix_rule_refines_zones_coordinates_and_region() {
        let m = cty_matcher().with_regional_rules(rule_book(
            "prefix,QA7,901,QA-N,Alpha North,16,29,,55.5,22.5,0\n\
             prefix,QA8,,,Alpha Zone Only,17,,,,,0\n",
        ));
        let r = m.lookup("QA7AB/P").unwrap();
        assert_eq!(r.dxcc, 901);
        assert_eq!(r.country, "Alpha Land");
        assert_eq!((r.cqz, r.ituz), (16, 29));
        assert_eq!(r.province.as_deref(), Some("Alpha North"));
        assert_eq!(r.subdivision_code.as_deref(), Some("QA-N"));
        assert_eq!((r.latitude, r.longitude), (55.5, 22.5));
        assert_eq!(r.location_precision, LocationPrecision::UserRegionalRule);

        // Reguła bez współrzędnych nie podnosi deklarowanej dokładności.
        let z = m.lookup("QA8AB").unwrap();
        assert_eq!(z.cqz, 17);
        assert_eq!(z.province.as_deref(), Some("Alpha Zone Only"));
        assert_eq!(z.location_precision, LocationPrecision::EntityCentroid);
        assert_eq!((z.latitude, z.longitude), (50.0, 20.0));
    }

    #[test]
    fn regional_rule_cannot_change_entity_resolved_by_country_files() {
        let m = cty_matcher().with_regional_rules(rule_book("prefix,QA7,902,,,16,,,,,0\n"));
        let res = m.lookup_detailed("QA7AB");
        let info = res.info.unwrap();
        assert_eq!(info.dxcc, 901);
        assert_eq!(info.cqz, 14);
        assert_eq!(
            res.conflicts,
            vec![LayerConflict::EntityMismatch {
                layer: RuleLayer::UserRegional,
                line: 3,
                rule_dxcc: 902,
                resolved_dxcc: 901,
            }]
        );
    }

    #[test]
    fn country_file_exact_call_ranks_above_regional_exact_rule() {
        let m = cty_matcher().with_regional_rules(rule_book(
            "exact,QA1EXACT,,,,20,,,,,0\nexact,QA5XYZ,902,,,21,,,,,0\n",
        ));
        // Znak dokładny z pliku krajów wygrywa z dokładną regułą regionalną.
        assert_eq!(m.lookup("QA1EXACT").unwrap().cqz, 14);
        // Dokładna reguła regionalna wygrywa z prefiksem i może zmienić encję.
        let r = m.lookup("QA5XYZ").unwrap();
        assert_eq!(r.dxcc, 902);
        assert_eq!(r.country, "Beta Isles");
        assert_eq!(r.cqz, 21);
        assert_eq!(r.location_precision, LocationPrecision::EntityCentroid);
    }

    #[test]
    fn user_override_ranks_above_every_other_layer() {
        let m = cty_matcher()
            .with_regional_rules(rule_book("prefix,QA,,,,16,,,,,0\n"))
            .with_overrides(rule_book(
                "exact,QA1EXACT,902,,,7,,,1.5,2.5,0\nprefix,QA9,,,Override Region,,,,,,0\n",
            ));
        let r = m.lookup("QA1EXACT/P").unwrap();
        assert_eq!(r.dxcc, 902);
        assert_eq!(r.country, "Beta Isles");
        assert_eq!(r.cqz, 7);
        assert_eq!((r.latitude, r.longitude), (1.5, 2.5));
        assert_eq!(r.location_precision, LocationPrecision::UserOverride);
        let p = m.lookup("QA9ZZ").unwrap();
        assert_eq!(p.province.as_deref(), Some("Override Region"));
        assert_eq!(p.cqz, 16); // strefa z reguły regionalnej pozostaje
        assert_eq!(m.layers().override_rules, 2);
        assert_eq!(m.layers().regional_rules, 1);
    }

    #[test]
    fn override_can_resolve_a_call_no_source_knows() {
        let m = cty_matcher().with_overrides(rule_book("prefix,QZ,902,,,,,,,,0\n"));
        let r = m.lookup("QZ1AB").unwrap();
        assert_eq!(r.country, "Beta Isles");
        assert_eq!(r.location_precision, LocationPrecision::EntityCentroid);
        // Bez plików krajów encja jest znana tylko z kodu — bez współrzędnych.
        let bare = PrefixMatcher::empty().with_overrides(rule_book("prefix,QZ,902,,,,,,,,0\n"));
        let b = bare.lookup("QZ1AB").unwrap();
        assert_eq!(b.dxcc, 902);
        assert_eq!(b.location_precision, LocationPrecision::Unknown);
    }

    #[test]
    fn ambiguous_patterns_are_reported_and_not_applied() {
        let m = cty_matcher().with_regional_rules(rule_book(
            "pattern,QA[0-9].*,,,,30,,,,,1\npattern,QA5.*,,,,31,,,,,1\n",
        ));
        let res = m.lookup_detailed("QA5AB");
        assert_eq!(res.info.unwrap().cqz, 14);
        assert_eq!(
            res.conflicts,
            vec![LayerConflict::AmbiguousPatterns {
                layer: RuleLayer::UserRegional,
                lines: vec![3, 4],
            }]
        );
    }

    #[test]
    fn legacy_regional_rules_refine_country_file_results_during_transition() {
        let uri = synthetic_legacy_db(
            r"INSERT INTO Province VALUES
                ('Alpha Land, North', 'QA', '901', 'EU', 15, 28, '51.0N', '21.0E', 'QA6.*', '', ''),
                ('Other, Wrong Entity', 'QB', '902', 'NA', 4, 7, '21.0N', '99.0W', 'QA4.*', '', '');
              INSERT INTO CountryDataEx VALUES
                ('Legacy Only', 'QL', 903, 'AF', 33, 37, '1.0N', '1.0E', 'QL.*', '');",
        );
        let legacy = PrefixMatcher::load_from_db(&uri).unwrap();
        let m = cty_matcher().with_legacy_compatibility(legacy);
        let r = m.lookup("QA6AB").unwrap();
        assert_eq!(r.country, "Alpha Land");
        assert_eq!((r.cqz, r.ituz), (15, 28));
        assert_eq!(r.province.as_deref(), Some("Alpha Land, North"));
        assert_eq!(r.location_precision, LocationPrecision::LegacyRegionalRule);
        // Reguła starszej bazy innej encji nie zmienia wyniku plików krajów.
        let w = m.lookup("QA4AB").unwrap();
        assert_eq!((w.dxcc, w.province), (901, None));
        // Prefiks nieznany plikom krajów: warstwa zgodności.
        let l = m.lookup("QL1AB").unwrap();
        assert_eq!(l.country, "Legacy Only");
        assert_eq!(l.location_precision, LocationPrecision::LegacyEntity);
        assert!(m.layers().country_files);
        assert_eq!(m.layers().legacy_rules, 3);
    }

    #[test]
    fn legacy_only_matcher_keeps_its_previous_results() {
        let uri = synthetic_legacy_db(
            r"INSERT INTO Province VALUES
                ('Alpha Land, North', 'QA', '901', 'EU', 15, 28, '51.0N', '21.0E', 'QA6.*', '', '');
              INSERT INTO CountryDataEx VALUES
                ('Alpha Land', 'QA', 901, 'EU', 14, 27, '50.0N', '20.0E', 'QA.*', '');",
        );
        let m = PrefixMatcher::load_from_db(&uri).unwrap();
        let r = m.lookup("QA6AB").unwrap();
        assert_eq!(
            (r.cqz, r.province.as_deref()),
            (15, Some("Alpha Land, North"))
        );
        assert_eq!(r.location_precision, LocationPrecision::LegacyRegionalRule);
        let e = m.lookup("QA1AB").unwrap();
        assert_eq!((e.cqz, e.province), (14, None));
        assert_eq!(e.location_precision, LocationPrecision::LegacyEntity);
        // Reguły użytkownika działają także bez plików krajów.
        let m = m.with_regional_rules(rule_book("prefix,QA1,,,Alpha One,18,,,,,0\n"));
        assert_eq!(m.lookup("QA1AB").unwrap().cqz, 18);
    }

    /// Baza w pamięci w układzie dostarczanej bazy: `UniqueCalls` z `_id`,
    /// `StartDate`, `EndDate`; `Province` z pustą kolumną `Comment`. Nazwa bez
    /// zegara: licznik w procesie.
    fn real_layout_legacy_db(rows_sql: &str) -> String {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let uri = format!(
            "file:legacy_real_{}_{n}?mode=memory&cache=shared",
            std::process::id()
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
                _id INTEGER PRIMARY KEY AUTOINCREMENT, Callsign TEXT, Country TEXT, QSL TEXT,
                DXCC TEXT, ARRLPrefix TEXT, Prefix TEXT, Continent TEXT, CQZone INTEGER,
                ITUZone INTEGER, Latitude TEXT, Longitude TEXT, Enable TEXT, EndDate TEXT,
                StartDate TEXT);
             CREATE TABLE CountryDataEx (
                _id INTEGER PRIMARY KEY AUTOINCREMENT, ARRLPrefix TEXT, Comment TEXT,
                Continent TEXT, Country TEXT, CQZone INTEGER, DXCC INTEGER, ITUZone INTEGER,
                Latitude TEXT, Longitude TEXT, PrefixList TEXT, Status TEXT, StartDate TEXT,
                TimeDiff INTEGER, EndDate TEXT);
             CREATE TABLE Province (
                _id INTEGER PRIMARY KEY AUTOINCREMENT, ARRLPrefix TEXT, Prefix TEXT,
                Comment TEXT, Continent TEXT, Country TEXT, CQZone INTEGER, DXCC TEXT,
                ITUZone INTEGER, Latitude TEXT, Longitude TEXT, PrefixList TEXT,
                EndDate TEXT, TimeDiff INTEGER, StartDate TEXT);
             {rows_sql}"
        ))
        .unwrap();
        std::mem::forget(conn);
        uri
    }

    /// Wiersz `UniqueCalls`: (_id, znak, encja, kod, CQ, ITU, Enable, start, koniec).
    #[allow(clippy::too_many_arguments)]
    fn uc_row(
        id: u32,
        call: &str,
        country: &str,
        dxcc: &str,
        cq: u32,
        itu: u32,
        enable: &str,
        start: &str,
        end: &str,
    ) -> String {
        format!(
            "INSERT INTO UniqueCalls (_id, Callsign, Country, QSL, DXCC, ARRLPrefix, Prefix,
               Continent, CQZone, ITUZone, Latitude, Longitude, Enable, EndDate, StartDate)
             VALUES ({id}, '{call}', '{country}', '', '{dxcc}', 'QA', 'QA', 'EU', {cq}, {itu},
               '50.0N', '20.0E', '{enable}', '{end}', '{start}');"
        )
    }

    #[test]
    fn unique_call_keys_are_trimmed_and_invalid_keys_rejected() {
        let rows = [
            uc_row(1, "QA1X ", "Alpha Land", "901", 14, 27, "", "", ""),
            uc_row(2, " qa2y", "Alpha Land", "901", 15, 28, "", "", ""),
            uc_row(3, "QA3??P", "Alpha Land", "901", 14, 27, "", "", ""),
            uc_row(4, "   ", "Alpha Land", "901", 14, 27, "", "", ""),
            uc_row(5, "QA4Z", "Alpha Land", "901", 14, 27, "0", "", ""),
        ]
        .concat();
        let (m, report) =
            PrefixMatcher::load_from_db_with_report(real_layout_legacy_db(&rows)).unwrap();
        assert_eq!(report.exact_calls, 2, "{report:?}");
        let invalid: Vec<&str> = report
            .issues
            .iter()
            .filter_map(|i| match &i.kind {
                LegacyRuleIssueKind::InvalidCallsign(c) => Some(c.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(invalid, vec!["QA3??P", "   "]);
        // Wcześniej klucz "QA1X " nie pasował do żadnego zapytania.
        let a = m.lookup("QA1X").unwrap();
        assert_eq!(
            (a.callsign.as_str(), a.dxcc, a.wpx_prefix.as_str()),
            ("QA1X", 901, "QA1")
        );
        assert_eq!(m.lookup("qa2y").unwrap().cqz, 15);
        // Wiersz wyłączony nadal nie jest wczytywany.
        assert_eq!(m.lookup("QA4Z"), None);
        let listed: Vec<String> = m
            .exception_calls()
            .into_iter()
            .map(|p| p.callsign)
            .collect();
        assert_eq!(listed, vec!["QA1X".to_string(), "QA2Y".to_string()]);
    }

    #[test]
    fn duplicate_unique_calls_resolve_to_the_latest_activity_then_rowid() {
        // (_id, znak, Alpha=true / Beta=false, StartDate, EndDate)
        let table: &[(u32, &str, bool, &str, &str)] = &[
            // Nowszy okres w wierszu o niższym _id wygrywa ze starszym w wyższym.
            (
                1,
                "QA5D",
                false,
                "2019-08-10 00:00:00",
                "2019-08-18 00:00:00",
            ),
            (
                2,
                "QA5D",
                true,
                "2004-10-23 00:00:00",
                "2004-11-15 00:00:00",
            ),
            // Bez dat: decyduje _id (jak dotychczasowa kolejność wierszy).
            (3, "QA6D", true, "", ""),
            (4, "QA6D", false, "", ""),
            // Wiersz z datą wygrywa z wierszem bez dat.
            (
                5,
                "QA7D",
                true,
                "2015-09-19 00:00:00",
                "2015-10-31 00:00:00",
            ),
            (6, "QA7D", false, "", ""),
            // Otwarty koniec: liczy się data początku.
            (7, "QA8D", true, "2013-07-14 00:00:00", ""),
            (8, "QA8D", false, "", "2013-07-13 00:00:00"),
            // Uszkodzona data jest traktowana jak brak daty.
            (9, "QA9D", true, "", "2020-01-01 00:00:00"),
            (10, "QA9D", false, "2030-13", "not a date"),
            // Ten sam dzień: decyduje _id.
            (11, "QB1D", true, "", "2011-03-01 00:00:00"),
            (12, "QB1D", false, "", "2011-03-01 12:00:00"),
        ];
        let rows: String = table
            .iter()
            .map(|&(id, call, alpha, start, end)| {
                let (country, dxcc, cq, itu) = if alpha {
                    ("Alpha Land", "901", 14, 27)
                } else {
                    ("Beta Isles", "902", 3, 6)
                };
                uc_row(id, call, country, dxcc, cq, itu, "", start, end)
            })
            .collect();
        let uri = real_layout_legacy_db(&rows);
        let (m, report) = PrefixMatcher::load_from_db_with_report(&uri).unwrap();
        assert_eq!(report.exact_calls, 6);
        assert_eq!(report.duplicate_calls, 6);
        let entity = |c: &str| m.lookup(c).unwrap().dxcc;
        assert_eq!(entity("QA5D"), 902);
        assert_eq!(entity("QA6D"), 902);
        assert_eq!(entity("QA7D"), 901);
        assert_eq!(entity("QA8D"), 901);
        assert_eq!(entity("QA9D"), 901);
        assert_eq!(entity("QB1D"), 902);
        // Deterministycznie: ponowne wczytanie daje te same wyniki.
        for _ in 0..20 {
            let (again, _) = PrefixMatcher::load_from_db_with_report(&uri).unwrap();
            assert_eq!(again.exception_calls(), m.exception_calls());
        }
    }

    #[test]
    fn legacy_activity_date_uses_the_latest_valid_date() {
        assert_eq!(
            legacy_activity_date(Some("2019-08-10 00:00:00"), Some("2019-08-18 00:00:00")),
            "2019-08-18"
        );
        assert_eq!(
            legacy_activity_date(Some("2013-07-14 00:00:00"), Some("")),
            "2013-07-14"
        );
        assert_eq!(
            legacy_activity_date(None, Some(" 2001-02-03")),
            "2001-02-03"
        );
        assert_eq!(
            legacy_activity_date(Some("2030-13"), Some("bad-date-x")),
            ""
        );
        assert_eq!(legacy_activity_date(Some("ąęść-óż-źń"), None), "");
        assert_eq!(legacy_activity_date(None, None), "");
    }

    #[test]
    fn region_name_comes_from_the_country_column() {
        let rows = r"
            INSERT INTO Province (ARRLPrefix, Prefix, Comment, Continent, Country, CQZone, DXCC,
                ITUZone, Latitude, Longitude, PrefixList, EndDate, TimeDiff, StartDate) VALUES
              ('QA', 'QA6', '', 'EU', '  Alpha Land, Alpha North ', 15, '901', 28, '51.0N', '21.0E', 'QA6.*', '', 1, ''),
              ('QA', 'QA7', '', 'EU', '   ', 16, '901', 29, '49.0N', '19.0E', 'QA7.*', '', 1, '');
            INSERT INTO CountryDataEx (ARRLPrefix, Comment, Continent, Country, CQZone, DXCC,
                ITUZone, Latitude, Longitude, PrefixList, Status, StartDate, TimeDiff, EndDate) VALUES
              ('QA', '', 'EU', 'Alpha Land', 14, 901, 27, '50.0N', '20.0E', 'QA.*', 'Active', '', 1, '');";
        let m = PrefixMatcher::load_from_db(real_layout_legacy_db(rows)).unwrap();
        let north = m.lookup("QA6AB").unwrap();
        assert_eq!(north.province.as_deref(), Some("Alpha Land, Alpha North"));
        assert_eq!(
            north.location_precision,
            LocationPrecision::LegacyRegionalRule
        );
        // Pusta nazwa regionu: None zamiast Some("").
        let blank = m.lookup("QA7AB").unwrap();
        assert_eq!((blank.cqz, blank.province), (16, None));
        // Reguła encji nie ma nazwy regionu.
        assert_eq!(m.lookup("QA1AB").unwrap().province, None);
    }

    #[test]
    fn country_file_load_applies_implicit_primaries_and_official_attributes() {
        let dir = tempfile::tempdir().unwrap();
        let dat = "\
Ant Synth:                13:  74:  SA:  -90.00:     0.00:     0.0:  QY:
    QY0,QY1;
";
        let csv = "QY,Ant Synth,13,SA,13,74,-90.00,0.00,0.0,QY0 QY1;\n";
        write_country_files(dir.path(), dat, Some(csv));
        let (m, report) = PrefixMatcher::load_from_country_files(
            &dir.path().join("cty.dat"),
            Some(&dir.path().join("cty.csv")),
        )
        .unwrap();
        assert_eq!(report.implicit_primary_prefixes, 1);
        assert_eq!(report.official_attribute_changes.len(), 1);
        // Prefiks gościnny równy prefiksowi głównemu (`QY/SP1AA` → `QY`).
        let g = m.lookup("QY/QA1AA").unwrap();
        assert_eq!((g.dxcc, g.continent.as_str()), (13, "AN"));
        assert_eq!(m.lookup("QY5AB").unwrap().arrl_prefix, "QY");
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
