// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Parser for the contest country files `cty.dat` and `cty.csv`.
//!
//! `cty.dat` layout (one entity per block, fields terminated by `:`):
//!
//! ```text
//! name: CQ: ITU: continent: latitude(+N): longitude(+W): UTC offset: [*]primary:
//!     alias, alias, =EXACTCALL(cq)[itu]<lat/lon>{cc}~offset~, ...;
//! ```
//!
//! * a leading `*` on the primary prefix marks an entity that is only on the
//!   DARC WAEDC list;
//! * an alias starting with `=` is a complete callsign that must match exactly;
//! * `(n)`, `[n]`, `<lat/lon>`, `{cc}` and `~n~` after an alias override the CQ
//!   zone, ITU zone, coordinates, continent and UTC offset of that alias;
//! * the alias list ends with `;`; line breaks are not significant;
//! * when a callsign or prefix appears twice, the first occurrence wins.
//!
//! `cty.csv` carries the same entities in the same order, one per line, with the
//! ADIF DXCC entity code as the third column:
//!
//! ```text
//! [*]primary,name,adif,continent,CQ,ITU,latitude(+N),longitude(+W),offset,alias alias ...;
//! ```
//!
//! Only the entity code is taken from `cty.csv`; everything else comes from
//! `cty.dat`. Longitudes are stored internally with the conventional sign
//! (+ East), i.e. negated with respect to both files.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// Upper bound for a country file accepted from disk (the largest published
/// variant is well below this).
pub const MAX_COUNTRY_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// One entity (country) from `cty.dat`.
#[derive(Debug, Clone, PartialEq)]
pub struct CtyEntity {
    pub name: String,
    pub cq_zone: u32,
    pub itu_zone: u32,
    pub continent: String,
    pub latitude: f64,
    /// Degrees, + East.
    pub longitude: f64,
    /// Hours, as written in the file (local time offset from GMT).
    pub utc_offset: f64,
    /// Primary prefix without the WAE marker.
    pub primary_prefix: String,
    /// `true` when the primary prefix carried the `*` WAE-only marker.
    pub wae_only: bool,
    /// ADIF DXCC entity code, available only after [`CtyData::apply_entity_codes`].
    pub dxcc: Option<u32>,
}

/// Per-alias overrides.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CtyOverrides {
    pub cq_zone: Option<u32>,
    pub itu_zone: Option<u32>,
    /// (latitude + North, longitude + East)
    pub coordinates: Option<(f64, f64)>,
    pub continent: Option<String>,
    pub utc_offset: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct AliasTarget {
    entity: usize,
    overrides: CtyOverrides,
}

/// Fully resolved attributes for one callsign.
#[derive(Debug, Clone, PartialEq)]
pub struct CtyMatch {
    pub entity_name: String,
    pub primary_prefix: String,
    pub wae_only: bool,
    pub dxcc: Option<u32>,
    pub cq_zone: u32,
    pub itu_zone: u32,
    pub continent: String,
    pub latitude: f64,
    pub longitude: f64,
    pub utc_offset: f64,
    /// The alias that matched (prefix or complete callsign, without `=`).
    pub matched: String,
    pub exact: bool,
    /// `true` when the alias carried a `<lat/lon>` override, i.e. the
    /// coordinates are more specific than the entity centre.
    pub coordinates_overridden: bool,
}

/// Kind of a non-fatal problem found while reading a country file.
/// Text fields carry the offending token for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtyIssueKind {
    EmptyAlias(String),
    UnknownModifier { modifier: char, token: String },
    UnterminatedModifier(String),
    InvalidModifierValue(String),
    IncompleteHeader,
    UnterminatedAliasList,
    EmptyEntityName,
    InvalidCqZone(String),
    InvalidItuZone(String),
    InvalidContinent(String),
    InvalidCoordinates,
    CoordinatesOutOfRange,
    InvalidUtcOffset(String),
    InvalidPrimaryPrefix(String),
    CsvColumnCount(usize),
    CsvInvalidEntityCode(String),
    CsvDuplicatePrimary(String),
}

impl fmt::Display for CtyIssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAlias(t) => write!(f, "empty alias in '{t}'"),
            Self::UnknownModifier { modifier, token } => {
                write!(f, "unknown modifier '{modifier}' in '{token}'")
            }
            Self::UnterminatedModifier(t) => write!(f, "unterminated modifier in '{t}'"),
            Self::InvalidModifierValue(t) => write!(f, "invalid modifier value in '{t}'"),
            Self::IncompleteHeader => f.write_str("incomplete entity header"),
            Self::UnterminatedAliasList => f.write_str("alias list not terminated by ';'"),
            Self::EmptyEntityName => f.write_str("empty entity name"),
            Self::InvalidCqZone(v) => write!(f, "invalid CQ zone '{v}'"),
            Self::InvalidItuZone(v) => write!(f, "invalid ITU zone '{v}'"),
            Self::InvalidContinent(v) => write!(f, "invalid continent '{v}'"),
            Self::InvalidCoordinates => f.write_str("invalid coordinates"),
            Self::CoordinatesOutOfRange => f.write_str("coordinates out of range"),
            Self::InvalidUtcOffset(v) => write!(f, "invalid UTC offset '{v}'"),
            Self::InvalidPrimaryPrefix(v) => write!(f, "invalid primary prefix '{v}'"),
            Self::CsvColumnCount(n) => write!(f, "expected 10 columns, found {n}"),
            Self::CsvInvalidEntityCode(v) => write!(f, "invalid entity code '{v}'"),
            Self::CsvDuplicatePrimary(v) => write!(f, "duplicate primary prefix '{v}' ignored"),
        }
    }
}

/// A non-fatal problem found while reading a file. The affected record or
/// alias is skipped; the rest of the file is still used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtyIssue {
    /// 1-based line where the problem starts.
    pub line: usize,
    pub kind: CtyIssueKind,
}

/// A fatal problem: nothing usable could be read.
#[derive(Debug)]
pub enum CtyError {
    Io(std::io::Error),
    TooLarge(u64),
    NotText,
    NoEntities,
}

impl fmt::Display for CtyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::TooLarge(n) => write!(f, "file too large ({n} bytes)"),
            Self::NotText => f.write_str("file is not a text file"),
            Self::NoEntities => f.write_str("no valid entity found"),
        }
    }
}

impl std::error::Error for CtyError {}

impl From<std::io::Error> for CtyError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Parsed `cty.dat`.
#[derive(Debug, Clone, Default)]
pub struct CtyData {
    entities: Vec<CtyEntity>,
    prefixes: HashMap<String, AliasTarget>,
    exact: HashMap<String, AliasTarget>,
    /// Longest prefix length present, bounds the longest-match scan.
    max_prefix_len: usize,
    pub issues: Vec<CtyIssue>,
}

/// Outcome of joining `cty.csv` entity codes onto `cty.dat` entities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityCodeReport {
    pub matched: usize,
    pub missing_in_csv: Vec<String>,
    pub unknown_in_csv: Vec<String>,
    pub name_mismatches: Vec<String>,
}

/// One row of `cty.csv`.
#[derive(Debug, Clone, PartialEq)]
pub struct CtyCsvRow {
    /// Primary prefix exactly as written (including a leading `*`).
    pub primary_prefix: String,
    pub name: String,
    pub dxcc: u32,
}

/// Reads a country file from disk with size and encoding checks.
pub fn read_country_file(path: &Path) -> Result<String, CtyError> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_COUNTRY_FILE_BYTES {
        return Err(CtyError::TooLarge(meta.len()));
    }
    let bytes = std::fs::read(path)?;
    decode_text(&bytes)
}

fn decode_text(bytes: &[u8]) -> Result<String, CtyError> {
    if bytes.contains(&0) {
        return Err(CtyError::NotText);
    }
    // The files are plain ASCII; accept UTF-8 and fall back to Latin-1 so a
    // stray accented character in a name never rejects the whole file.
    Ok(match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    })
}

fn parse_number<T: std::str::FromStr>(field: &str) -> Option<T> {
    field.trim().parse().ok()
}

fn parse_float(field: &str) -> Option<f64> {
    let v: f64 = field.trim().parse().ok()?;
    v.is_finite().then_some(v)
}

fn valid_zone(z: u32, max: u32) -> bool {
    (1..=max).contains(&z)
}

fn valid_continent(c: &str) -> bool {
    matches!(c, "AF" | "AN" | "AS" | "EU" | "NA" | "OC" | "SA")
}

fn valid_coordinates(lat: f64, lon: f64) -> bool {
    (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)
}

fn valid_offset(o: f64) -> bool {
    (-14.0..=14.0).contains(&o)
}

fn is_call_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '/'
}

/// Parses one alias token such as `=K1ABC(4)[7]` into text, exactness and overrides.
fn parse_alias(token: &str) -> Result<(String, bool, CtyOverrides), CtyIssueKind> {
    let mut chars = token.chars().peekable();
    let exact = chars.peek() == Some(&'=');
    if exact {
        chars.next();
    }
    let mut text = String::new();
    while let Some(&c) = chars.peek() {
        if is_call_char(c) {
            text.push(c.to_ascii_uppercase());
            chars.next();
        } else {
            break;
        }
    }
    if text.is_empty() {
        return Err(CtyIssueKind::EmptyAlias(token.to_string()));
    }

    let mut ov = CtyOverrides::default();
    while let Some(open) = chars.next() {
        let close = match open {
            '(' => ')',
            '[' => ']',
            '<' => '>',
            '{' => '}',
            '~' => '~',
            other => {
                return Err(CtyIssueKind::UnknownModifier {
                    modifier: other,
                    token: token.to_string(),
                });
            }
        };
        let mut body = String::new();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == close {
                closed = true;
                break;
            }
            body.push(c);
        }
        if !closed {
            return Err(CtyIssueKind::UnterminatedModifier(token.to_string()));
        }
        let bad = || CtyIssueKind::InvalidModifierValue(token.to_string());
        match open {
            '(' => {
                let z = parse_number::<u32>(&body).filter(|z| valid_zone(*z, 40));
                ov.cq_zone = Some(z.ok_or_else(bad)?);
            }
            '[' => {
                let z = parse_number::<u32>(&body).filter(|z| valid_zone(*z, 90));
                ov.itu_zone = Some(z.ok_or_else(bad)?);
            }
            '<' => {
                let (lat, lon) = body.split_once('/').ok_or_else(bad)?;
                let lat = parse_float(lat).ok_or_else(bad)?;
                let lon_w = parse_float(lon).ok_or_else(bad)?;
                if !valid_coordinates(lat, -lon_w) {
                    return Err(bad());
                }
                ov.coordinates = Some((lat, -lon_w));
            }
            '{' => {
                let c = body.trim().to_ascii_uppercase();
                if !valid_continent(&c) {
                    return Err(bad());
                }
                ov.continent = Some(c);
            }
            _ => {
                let o = parse_float(&body).filter(|o| valid_offset(*o));
                ov.utc_offset = Some(o.ok_or_else(bad)?);
            }
        }
    }
    Ok((text, exact, ov))
}

/// Character cursor that tracks line numbers.
struct Cursor<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
}

impl<'a> Cursor<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            chars: s.chars().peekable(),
            line: 1,
        }
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next();
        if c == Some('\n') {
            self.line += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while self.chars.peek().is_some_and(|c| c.is_whitespace()) {
            self.bump();
        }
    }

    /// Reads up to (and consumes) `stop`; returns `None` at end of input.
    fn read_until(&mut self, stop: char) -> Option<String> {
        let mut out = String::new();
        loop {
            match self.bump() {
                Some(c) if c == stop => return Some(out),
                Some(c) => out.push(c),
                None => return None,
            }
        }
    }
}

impl CtyData {
    /// Parses the text of a `cty.dat` file.
    pub fn parse(text: &str) -> Result<Self, CtyError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut data = Self::default();
        let mut cur = Cursor::new(text);

        loop {
            cur.skip_whitespace();
            if cur.chars.peek().is_none() {
                break;
            }
            let start_line = cur.line;

            // Header: eight ':'-terminated fields. A ';' inside the header means
            // the record is broken; resynchronise on it.
            let mut fields = Vec::with_capacity(8);
            let mut broken = false;
            while fields.len() < 8 {
                let mut f = String::new();
                loop {
                    match cur.bump() {
                        Some(':') => break,
                        Some(c) if c != ';' => f.push(c),
                        _ => {
                            broken = true;
                            break;
                        }
                    }
                }
                if broken {
                    break;
                }
                fields.push(f);
            }
            if broken {
                data.issue(start_line, CtyIssueKind::IncompleteHeader);
                if cur.chars.peek().is_none() {
                    break;
                }
                continue;
            }

            let Some(alias_block) = cur.read_until(';') else {
                data.issue(start_line, CtyIssueKind::UnterminatedAliasList);
                break;
            };

            match Self::parse_header(&fields) {
                Ok(entity) => data.add_entity(entity, &alias_block, start_line),
                Err(kind) => data.issue(start_line, kind),
            }
        }

        if data.entities.is_empty() {
            return Err(CtyError::NoEntities);
        }
        Ok(data)
    }

    /// Reads and parses a `cty.dat` file.
    pub fn load(path: &Path) -> Result<Self, CtyError> {
        Self::parse(&read_country_file(path)?)
    }

    fn issue(&mut self, line: usize, kind: CtyIssueKind) {
        self.issues.push(CtyIssue { line, kind });
    }

    fn parse_header(f: &[String]) -> Result<CtyEntity, CtyIssueKind> {
        let name = f[0].trim().to_string();
        if name.is_empty() {
            return Err(CtyIssueKind::EmptyEntityName);
        }
        let cq = parse_number::<u32>(&f[1])
            .filter(|z| valid_zone(*z, 40))
            .ok_or_else(|| CtyIssueKind::InvalidCqZone(f[1].trim().to_string()))?;
        let itu = parse_number::<u32>(&f[2])
            .filter(|z| valid_zone(*z, 90))
            .ok_or_else(|| CtyIssueKind::InvalidItuZone(f[2].trim().to_string()))?;
        let continent = f[3].trim().to_ascii_uppercase();
        if !valid_continent(&continent) {
            return Err(CtyIssueKind::InvalidContinent(continent));
        }
        let lat = parse_float(&f[4]);
        let lon_w = parse_float(&f[5]);
        let (Some(lat), Some(lon_w)) = (lat, lon_w) else {
            return Err(CtyIssueKind::InvalidCoordinates);
        };
        if !valid_coordinates(lat, -lon_w) {
            return Err(CtyIssueKind::CoordinatesOutOfRange);
        }
        let offset = parse_float(&f[6])
            .filter(|o| valid_offset(*o))
            .ok_or_else(|| CtyIssueKind::InvalidUtcOffset(f[6].trim().to_string()))?;
        let raw_primary = f[7].trim();
        let wae_only = raw_primary.starts_with('*');
        let primary = raw_primary.trim_start_matches('*').to_ascii_uppercase();
        if primary.is_empty() || !primary.chars().all(is_call_char) {
            return Err(CtyIssueKind::InvalidPrimaryPrefix(raw_primary.to_string()));
        }
        Ok(CtyEntity {
            name,
            cq_zone: cq,
            itu_zone: itu,
            continent,
            latitude: lat,
            longitude: -lon_w,
            utc_offset: offset,
            primary_prefix: primary,
            wae_only,
            dxcc: None,
        })
    }

    fn add_entity(&mut self, entity: CtyEntity, alias_block: &str, line: usize) {
        let idx = self.entities.len();
        self.entities.push(entity);
        for token in alias_block
            .split([',', '\n', '\r', ' ', '\t'])
            .filter(|t| !t.is_empty())
        {
            match parse_alias(token) {
                Ok((text, exact, overrides)) => {
                    let target = AliasTarget {
                        entity: idx,
                        overrides,
                    };
                    let map = if exact {
                        &mut self.exact
                    } else {
                        &mut self.prefixes
                    };
                    // First occurrence wins.
                    let len = text.len();
                    if let std::collections::hash_map::Entry::Vacant(slot) = map.entry(text) {
                        slot.insert(target);
                        if !exact {
                            self.max_prefix_len = self.max_prefix_len.max(len);
                        }
                    }
                }
                Err(kind) => self.issue(line, kind),
            }
        }
    }

    pub fn entities(&self) -> &[CtyEntity] {
        &self.entities
    }

    pub fn prefix_count(&self) -> usize {
        self.prefixes.len()
    }

    pub fn exact_count(&self) -> usize {
        self.exact.len()
    }

    /// Complete callsigns listed with `=`, sorted, each with its resolved attributes.
    pub fn exact_calls(&self) -> Vec<CtyMatch> {
        let mut out: Vec<CtyMatch> = self
            .exact
            .iter()
            .map(|(call, t)| self.build_match(call, t, true))
            .collect();
        out.sort_by(|a, b| a.matched.cmp(&b.matched));
        out
    }

    /// Exact-callsign match only.
    pub fn resolve_exact(&self, call: &str) -> Option<CtyMatch> {
        let call = call.trim().to_ascii_uppercase();
        self.exact
            .get(&call)
            .map(|t| self.build_match(&call, t, true))
    }

    /// Longest-prefix match on an already normalised callsign.
    pub fn resolve_prefix(&self, call: &str) -> Option<CtyMatch> {
        let call = call.trim().to_ascii_uppercase();
        let upper = call.len().min(self.max_prefix_len);
        (1..=upper)
            .rev()
            .filter(|&n| call.is_char_boundary(n))
            .find_map(|n| {
                let p = &call[..n];
                self.prefixes.get(p).map(|t| self.build_match(p, t, false))
            })
    }

    /// Exact callsign first, then the longest prefix of `prefix_call`.
    pub fn resolve(&self, full_call: &str, prefix_call: &str) -> Option<CtyMatch> {
        self.resolve_exact(full_call)
            .or_else(|| self.resolve_prefix(prefix_call))
    }

    fn build_match(&self, matched: &str, t: &AliasTarget, exact: bool) -> CtyMatch {
        let e = &self.entities[t.entity];
        let o = &t.overrides;
        let (lat, lon) = o.coordinates.unwrap_or((e.latitude, e.longitude));
        CtyMatch {
            entity_name: e.name.clone(),
            primary_prefix: e.primary_prefix.clone(),
            wae_only: e.wae_only,
            dxcc: e.dxcc,
            cq_zone: o.cq_zone.unwrap_or(e.cq_zone),
            itu_zone: o.itu_zone.unwrap_or(e.itu_zone),
            continent: o.continent.clone().unwrap_or_else(|| e.continent.clone()),
            latitude: lat,
            longitude: lon,
            utc_offset: o.utc_offset.unwrap_or(e.utc_offset),
            matched: matched.to_string(),
            exact,
            coordinates_overridden: o.coordinates.is_some(),
        }
    }

    /// Release marker listed in the file as the exact callsign `VERyyyymmdd`
    /// (see the country-files documentation). Returns the marker and the date
    /// as `YYYY-MM-DD`.
    pub fn version_marker(&self) -> Option<(String, String)> {
        let mut markers: Vec<&String> = self
            .exact
            .keys()
            .filter(|k| {
                k.len() == 11 && k.starts_with("VER") && k[3..].bytes().all(|b| b.is_ascii_digit())
            })
            .collect();
        markers.sort();
        let m = markers.last()?;
        Some((
            (*m).clone(),
            format!("{}-{}-{}", &m[3..7], &m[7..9], &m[9..11]),
        ))
    }

    /// First entity carrying ADIF entity code `dxcc`, preferring entities
    /// that are not WAE-only.
    pub fn entity_for_dxcc(&self, dxcc: u32) -> Option<&CtyEntity> {
        let mut with_code = self.entities.iter().filter(|e| e.dxcc == Some(dxcc));
        let first = with_code.clone().find(|e| !e.wae_only);
        first.or_else(|| with_code.next())
    }

    /// Joins ADIF entity codes from `cty.csv` rows onto the entities, keyed by
    /// the primary prefix including the WAE marker.
    pub fn apply_entity_codes(&mut self, rows: &[CtyCsvRow]) -> EntityCodeReport {
        let mut by_primary: HashMap<String, &CtyCsvRow> = HashMap::new();
        for r in rows {
            by_primary
                .entry(r.primary_prefix.to_ascii_uppercase())
                .or_insert(r);
        }
        let mut report = EntityCodeReport::default();
        let mut used = std::collections::HashSet::new();
        for e in &mut self.entities {
            let key = if e.wae_only {
                format!("*{}", e.primary_prefix)
            } else {
                e.primary_prefix.clone()
            };
            if let Some(r) = by_primary.get(&key) {
                e.dxcc = Some(r.dxcc);
                report.matched += 1;
                used.insert(key.clone());
                if !r.name.trim().eq_ignore_ascii_case(e.name.trim()) {
                    report.name_mismatches.push(key);
                }
            } else {
                e.dxcc = None;
                report.missing_in_csv.push(key);
            }
        }
        report.unknown_in_csv = by_primary
            .keys()
            .filter(|k| !used.contains(*k))
            .cloned()
            .collect();
        report.unknown_in_csv.sort();
        report
    }
}

/// Parses `cty.csv`, keeping only the columns SPLogbook needs. Malformed rows
/// are reported and skipped.
pub fn parse_cty_csv(text: &str) -> (Vec<CtyCsvRow>, Vec<CtyIssue>) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    let mut issues = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.splitn(10, ',').collect();
        if cols.len() < 10 {
            issues.push(CtyIssue {
                line: line_no,
                kind: CtyIssueKind::CsvColumnCount(cols.len()),
            });
            continue;
        }
        let primary = cols[0].trim().to_ascii_uppercase();
        let valid_primary = !primary.trim_start_matches('*').is_empty()
            && primary.trim_start_matches('*').chars().all(is_call_char);
        let dxcc = parse_number::<u32>(cols[2]).filter(|d| (1..=999).contains(d));
        match (valid_primary, dxcc) {
            (true, Some(dxcc)) => {
                if seen.insert(primary.clone()) {
                    rows.push(CtyCsvRow {
                        primary_prefix: primary,
                        name: cols[1].trim().to_string(),
                        dxcc,
                    });
                } else {
                    issues.push(CtyIssue {
                        line: line_no,
                        kind: CtyIssueKind::CsvDuplicatePrimary(primary),
                    });
                }
            }
            (false, _) => issues.push(CtyIssue {
                line: line_no,
                kind: CtyIssueKind::InvalidPrimaryPrefix(cols[0].trim().to_string()),
            }),
            (true, None) => issues.push(CtyIssue {
                line: line_no,
                kind: CtyIssueKind::CsvInvalidEntityCode(cols[2].trim().to_string()),
            }),
        }
    }
    (rows, issues)
}

/// Reads and parses a `cty.csv` file.
pub fn load_cty_csv(path: &Path) -> Result<(Vec<CtyCsvRow>, Vec<CtyIssue>), CtyError> {
    let text = read_country_file(path)?;
    let (rows, issues) = parse_cty_csv(&text);
    if rows.is_empty() {
        return Err(CtyError::NoEntities);
    }
    Ok((rows, issues))
}

#[cfg(test)]
mod tests;
