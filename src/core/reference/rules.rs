// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! User-supplied prefix rules: regional refinements and explicit overrides.
//!
//! # File format (`splogbook-rules`, version 1)
//!
//! UTF-8 text, comma-separated, RFC 4180 quoting (`"a, b"`, `""` for a quote).
//!
//! 1. The first line is the format marker: `#splogbook-rules,1`.
//! 2. Further lines starting with `#` are comments.
//! 3. The first other line is the header. Column order is free; columns are
//!    identified by name. Unknown columns are reported and ignored.
//! 4. Every following non-empty line is one rule.
//!
//! | column | required | meaning |
//! |---|---|---|
//! | `rule_type` | yes | `exact` (complete callsign), `prefix` (longest prefix), `pattern` (regular expression anchored at the start) |
//! | `callsign_or_prefix` | yes | callsign, prefix or pattern |
//! | `dxcc_entity_code` | no | ADIF DXCC entity code, 1–999 |
//! | `subdivision_code` | no | code of the region (≤ 32 characters) |
//! | `subdivision_name` | no | name of the region (≤ 128 characters) |
//! | `cq_zone` | no | 1–40 |
//! | `itu_zone` | no | 1–90 |
//! | `continent` | no | AF, AN, AS, EU, NA, OC, SA |
//! | `latitude`, `longitude` | no, both or neither | degrees, + North / + East |
//! | `priority` | no | integer −1000…1000, default 0; higher wins |
//! | `source`, `source_version`, `source_date`, `notes` | no | provenance; `source_date` is `YYYY-MM-DD` |
//!
//! A rule must set at least one of: entity code, subdivision, zone,
//! continent or coordinates.
//!
//! # Validation
//!
//! A file is accepted only as a whole: any invalid row rejects the file and
//! the previously installed rules stay in use. Two rules with the same type,
//! key and priority but different attributes are a conflict and reject the
//! file; an exact duplicate is reported and kept once.
//!
//! # Matching
//!
//! Within one rule book: `exact` (full call, then the call without
//! operational suffixes), then the longest `prefix`, then `pattern`. Among
//! rules for the same key the highest priority wins. When two patterns of the
//! same highest priority match, the result is a conflict and no pattern rule
//! is applied.

use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

pub const FORMAT_MARKER: &str = "#splogbook-rules";
pub const FORMAT_VERSION: u32 = 1;
pub const MAX_RULES_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PATTERN_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleType {
    Exact,
    Prefix,
    Pattern,
}

/// Attributes a rule may set. `None` keeps the value from lower layers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuleAttributes {
    pub dxcc: Option<u32>,
    pub subdivision_code: Option<String>,
    pub subdivision_name: Option<String>,
    pub cq_zone: Option<u32>,
    pub itu_zone: Option<u32>,
    pub continent: Option<String>,
    pub coordinates: Option<(f64, f64)>,
}

impl RuleAttributes {
    fn is_empty(&self) -> bool {
        self.dxcc.is_none()
            && self.subdivision_code.is_none()
            && self.subdivision_name.is_none()
            && self.cq_zone.is_none()
            && self.itu_zone.is_none()
            && self.continent.is_none()
            && self.coordinates.is_none()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleProvenance {
    pub source: String,
    pub source_version: String,
    pub source_date: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub rule_type: RuleType,
    /// Upper-case key (callsign, prefix or pattern text).
    pub key: String,
    pub priority: i32,
    pub attributes: RuleAttributes,
    pub provenance: RuleProvenance,
    /// 1-based line in the source file.
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleIssueKind {
    MissingFormatMarker,
    UnsupportedVersion(String),
    MissingHeader,
    MissingRequiredColumn(&'static str),
    DuplicateColumn(String),
    UnknownColumn(String),
    UnterminatedQuote,
    TooManyFields(usize),
    InvalidRuleType(String),
    InvalidKey(String),
    InvalidPattern(String),
    InvalidNumber { column: &'static str, value: String },
    OutOfRange { column: &'static str, value: String },
    InvalidContinent(String),
    IncompleteCoordinates,
    TextTooLong(&'static str),
    InvalidDate(String),
    EmptyRule,
    DuplicateRule(String),
    ConflictingRule { key: String, first_line: usize },
    NotUtf8,
    TooLarge(u64),
}

impl fmt::Display for RuleIssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFormatMarker => {
                write!(f, "first line must be '{FORMAT_MARKER},{FORMAT_VERSION}'")
            }
            Self::UnsupportedVersion(v) => write!(f, "unsupported format version '{v}'"),
            Self::MissingHeader => f.write_str("header line missing"),
            Self::MissingRequiredColumn(c) => write!(f, "required column '{c}' missing"),
            Self::DuplicateColumn(c) => write!(f, "duplicate column '{c}'"),
            Self::UnknownColumn(c) => write!(f, "unknown column '{c}' ignored"),
            Self::UnterminatedQuote => f.write_str("unterminated quoted field"),
            Self::TooManyFields(n) => write!(f, "{n} fields, more than the header"),
            Self::InvalidRuleType(v) => write!(f, "invalid rule_type '{v}'"),
            Self::InvalidKey(v) => write!(f, "invalid callsign_or_prefix '{v}'"),
            Self::InvalidPattern(v) => write!(f, "invalid pattern: {v}"),
            Self::InvalidNumber { column, value } => write!(f, "invalid {column} '{value}'"),
            Self::OutOfRange { column, value } => write!(f, "{column} '{value}' out of range"),
            Self::InvalidContinent(v) => write!(f, "invalid continent '{v}'"),
            Self::IncompleteCoordinates => f.write_str("latitude and longitude must both be set"),
            Self::TextTooLong(c) => write!(f, "{c} too long"),
            Self::InvalidDate(v) => write!(f, "invalid source_date '{v}' (expected YYYY-MM-DD)"),
            Self::EmptyRule => f.write_str("rule sets no attribute"),
            Self::DuplicateRule(k) => write!(f, "duplicate rule '{k}' ignored"),
            Self::ConflictingRule { key, first_line } => {
                write!(f, "rule '{key}' conflicts with line {first_line}")
            }
            Self::NotUtf8 => f.write_str("file is not valid UTF-8"),
            Self::TooLarge(n) => write!(f, "file too large ({n} bytes)"),
        }
    }
}

impl RuleIssueKind {
    /// Warnings do not reject the file.
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::UnknownColumn(_) | Self::DuplicateRule(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleIssue {
    pub line: usize,
    pub kind: RuleIssueKind,
}

/// Result of parsing a rules file.
#[derive(Debug, Clone, Default)]
pub struct RulesParse {
    pub rules: Vec<Rule>,
    pub issues: Vec<RuleIssue>,
}

impl RulesParse {
    pub fn errors(&self) -> impl Iterator<Item = &RuleIssue> {
        self.issues.iter().filter(|i| !i.kind.is_warning())
    }

    pub fn is_valid(&self) -> bool {
        self.errors().next().is_none()
    }
}

const COLUMNS: &[&str] = &[
    "rule_type",
    "callsign_or_prefix",
    "dxcc_entity_code",
    "subdivision_code",
    "subdivision_name",
    "cq_zone",
    "itu_zone",
    "continent",
    "latitude",
    "longitude",
    "priority",
    "source",
    "source_version",
    "source_date",
    "notes",
];

/// Splits one CSV line (RFC 4180 quoting, no embedded newlines).
fn split_csv_line(line: &str) -> Result<Vec<String>, RuleIssueKind> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    let mut at_field_start = true;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '"' if at_field_start => {
                quoted = true;
                at_field_start = false;
            }
            ',' => {
                fields.push(std::mem::take(&mut cur));
                at_field_start = true;
            }
            _ => {
                cur.push(c);
                at_field_start = false;
            }
        }
    }
    if quoted {
        return Err(RuleIssueKind::UnterminatedQuote);
    }
    fields.push(cur);
    Ok(fields)
}

fn is_call_text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().ok();
    match (num(0..4), num(5..7), num(8..10)) {
        (Some(_), Some(m), Some(d)) => (1..=12).contains(&m) && (1..=31).contains(&d),
        _ => false,
    }
}

/// Parses the text of a rules file. Never panics.
pub fn parse_rules(text: &str) -> RulesParse {
    let mut out = RulesParse::default();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')));

    let Some((_, first)) = lines.next() else {
        out.issues.push(RuleIssue {
            line: 1,
            kind: RuleIssueKind::MissingFormatMarker,
        });
        return out;
    };
    let first = first.trim();
    let Some((_, version)) = first
        .split_once(',')
        .filter(|(marker, _)| marker.trim() == FORMAT_MARKER)
    else {
        out.issues.push(RuleIssue {
            line: 1,
            kind: RuleIssueKind::MissingFormatMarker,
        });
        return out;
    };
    if version.trim() != FORMAT_VERSION.to_string() {
        out.issues.push(RuleIssue {
            line: 1,
            kind: RuleIssueKind::UnsupportedVersion(version.trim().to_string()),
        });
        return out;
    }

    // Header.
    let mut header: Option<(usize, Vec<Option<usize>>)> = None;
    let mut seen: HashMap<(RuleType, String, i32), usize> = HashMap::new();

    for (line_no, raw) in lines {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = match split_csv_line(raw) {
            Ok(f) => f,
            Err(kind) => {
                out.issues.push(RuleIssue {
                    line: line_no,
                    kind,
                });
                continue;
            }
        };

        let Some((width, ref map)) = header else {
            // map[i] = index into COLUMNS for field i, or None for unknown.
            let mut map = Vec::with_capacity(fields.len());
            let mut have = std::collections::HashSet::new();
            for name in &fields {
                let name = name.trim().to_ascii_lowercase();
                if let Some(idx) = COLUMNS.iter().position(|c| *c == name) {
                    if !have.insert(idx) {
                        out.issues.push(RuleIssue {
                            line: line_no,
                            kind: RuleIssueKind::DuplicateColumn(name),
                        });
                    }
                    map.push(Some(idx));
                } else {
                    out.issues.push(RuleIssue {
                        line: line_no,
                        kind: RuleIssueKind::UnknownColumn(name),
                    });
                    map.push(None);
                }
            }
            for required in [0usize, 1] {
                if !have.contains(&required) {
                    out.issues.push(RuleIssue {
                        line: line_no,
                        kind: RuleIssueKind::MissingRequiredColumn(COLUMNS[required]),
                    });
                }
            }
            if !out.is_valid() {
                return out;
            }
            header = Some((fields.len(), map));
            continue;
        };

        if fields.len() > width {
            out.issues.push(RuleIssue {
                line: line_no,
                kind: RuleIssueKind::TooManyFields(fields.len()),
            });
            continue;
        }
        let mut values: [&str; 15] = [""; 15];
        for (i, v) in fields.iter().enumerate() {
            if let Some(Some(col)) = map.get(i) {
                values[*col] = v.trim();
            }
        }
        match parse_row(&values, line_no) {
            Ok(rule) => {
                let key = (rule.rule_type, rule.key.clone(), rule.priority);
                if let Some(&first_idx) = seen.get(&key) {
                    let first: &Rule = &out.rules[first_idx];
                    let kind = if first.attributes == rule.attributes {
                        RuleIssueKind::DuplicateRule(rule.key.clone())
                    } else {
                        RuleIssueKind::ConflictingRule {
                            key: rule.key.clone(),
                            first_line: first.line,
                        }
                    };
                    out.issues.push(RuleIssue {
                        line: line_no,
                        kind,
                    });
                } else {
                    seen.insert(key, out.rules.len());
                    out.rules.push(rule);
                }
            }
            Err(kind) => out.issues.push(RuleIssue {
                line: line_no,
                kind,
            }),
        }
    }

    if header.is_none() {
        out.issues.push(RuleIssue {
            line: 1,
            kind: RuleIssueKind::MissingHeader,
        });
    }
    out
}

fn num<T: std::str::FromStr>(column: &'static str, s: &str) -> Result<Option<T>, RuleIssueKind> {
    if s.is_empty() {
        return Ok(None);
    }
    s.parse()
        .map(Some)
        .map_err(|_| RuleIssueKind::InvalidNumber {
            column,
            value: s.to_string(),
        })
}
fn ranged<T: PartialOrd + Copy>(
    column: &'static str,
    raw: &str,
    v: Option<T>,
    lo: T,
    hi: T,
) -> Result<Option<T>, RuleIssueKind> {
    match v {
        Some(x) if !(lo..=hi).contains(&x) => Err(RuleIssueKind::OutOfRange {
            column,
            value: raw.to_string(),
        }),
        other => Ok(other),
    }
}
fn text(column: &'static str, s: &str, max: usize) -> Result<Option<String>, RuleIssueKind> {
    if s.is_empty() {
        Ok(None)
    } else if s.chars().count() > max {
        Err(RuleIssueKind::TextTooLong(column))
    } else {
        Ok(Some(s.to_string()))
    }
}

fn parse_row(v: &[&str; 15], line: usize) -> Result<Rule, RuleIssueKind> {
    let rule_type = match v[0].to_ascii_lowercase().as_str() {
        "exact" => RuleType::Exact,
        "prefix" => RuleType::Prefix,
        "pattern" => RuleType::Pattern,
        other => return Err(RuleIssueKind::InvalidRuleType(other.to_string())),
    };
    let key = if rule_type == RuleType::Pattern {
        let k = v[1].to_string();
        if k.is_empty() || k.len() > MAX_PATTERN_LEN {
            return Err(RuleIssueKind::InvalidKey(k));
        }
        Regex::new(&anchor(&k)).map_err(|e| RuleIssueKind::InvalidPattern(e.to_string()))?;
        k
    } else {
        let k = v[1].to_ascii_uppercase();
        if !is_call_text(&k) {
            return Err(RuleIssueKind::InvalidKey(v[1].to_string()));
        }
        k
    };

    let dxcc = ranged(
        "dxcc_entity_code",
        v[2],
        num::<u32>("dxcc_entity_code", v[2])?,
        1,
        999,
    )?;
    let cq = ranged("cq_zone", v[5], num::<u32>("cq_zone", v[5])?, 1, 40)?;
    let itu = ranged("itu_zone", v[6], num::<u32>("itu_zone", v[6])?, 1, 90)?;
    let continent = if v[7].is_empty() {
        None
    } else {
        let c = v[7].to_ascii_uppercase();
        if !matches!(c.as_str(), "AF" | "AN" | "AS" | "EU" | "NA" | "OC" | "SA") {
            return Err(RuleIssueKind::InvalidContinent(v[7].to_string()));
        }
        Some(c)
    };
    let lat = num::<f64>("latitude", v[8])?.filter(|x| x.is_finite());
    let lon = num::<f64>("longitude", v[9])?.filter(|x| x.is_finite());
    let lat = ranged("latitude", v[8], lat, -90.0, 90.0)?;
    let lon = ranged("longitude", v[9], lon, -180.0, 180.0)?;
    let coordinates = match (lat, lon) {
        (Some(a), Some(b)) => Some((a, b)),
        (None, None) if v[8].is_empty() && v[9].is_empty() => None,
        _ => return Err(RuleIssueKind::IncompleteCoordinates),
    };
    let priority = ranged(
        "priority",
        v[10],
        num::<i32>("priority", v[10])?,
        -1000,
        1000,
    )?
    .unwrap_or(0);
    if !v[13].is_empty() && !valid_date(v[13]) {
        return Err(RuleIssueKind::InvalidDate(v[13].to_string()));
    }

    let attributes = RuleAttributes {
        dxcc,
        subdivision_code: text("subdivision_code", v[3], 32)?,
        subdivision_name: text("subdivision_name", v[4], 128)?,
        cq_zone: cq,
        itu_zone: itu,
        continent,
        coordinates,
    };
    if attributes.is_empty() {
        return Err(RuleIssueKind::EmptyRule);
    }
    Ok(Rule {
        rule_type,
        key,
        priority,
        attributes,
        provenance: RuleProvenance {
            source: text("source", v[11], 256)?.unwrap_or_default(),
            source_version: text("source_version", v[12], 64)?.unwrap_or_default(),
            source_date: v[13].to_string(),
            notes: text("notes", v[14], 1024)?.unwrap_or_default(),
        },
        line,
    })
}

fn anchor(p: &str) -> String {
    if p.starts_with('^') {
        p.to_string()
    } else {
        format!("^(?:{p})")
    }
}

/// Reads a rules file with size and encoding checks.
pub fn read_rules_file(path: &Path) -> std::io::Result<Result<String, RuleIssueKind>> {
    let len = std::fs::metadata(path)?.len();
    if len > MAX_RULES_FILE_BYTES {
        return Ok(Err(RuleIssueKind::TooLarge(len)));
    }
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8(bytes).map_err(|_| RuleIssueKind::NotUtf8))
}

/// Outcome of matching a rule book against one callsign.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleMatch<'a> {
    None,
    Matched(&'a Rule),
    /// Several rules of equal top priority matched; none is applied.
    Conflict(Vec<&'a Rule>),
}

/// Compiled, query-ready rules.
#[derive(Debug, Default)]
pub struct RuleBook {
    exact: HashMap<String, Rule>,
    prefix: HashMap<String, Rule>,
    max_prefix_len: usize,
    patterns: Vec<(Regex, Rule)>,
}

impl RuleBook {
    /// Builds a rule book from a valid parse. Among several rules with the
    /// same type and key the highest priority is kept.
    pub fn from_rules(rules: Vec<Rule>) -> Self {
        let mut book = Self::default();
        for rule in rules {
            match rule.rule_type {
                RuleType::Exact | RuleType::Prefix => {
                    let map = if rule.rule_type == RuleType::Exact {
                        &mut book.exact
                    } else {
                        book.max_prefix_len = book.max_prefix_len.max(rule.key.len());
                        &mut book.prefix
                    };
                    match map.get(&rule.key) {
                        Some(existing) if existing.priority >= rule.priority => {}
                        _ => {
                            map.insert(rule.key.clone(), rule);
                        }
                    }
                }
                RuleType::Pattern => {
                    if let Ok(re) = Regex::new(&anchor(&rule.key)) {
                        book.patterns.push((re, rule));
                    }
                }
            }
        }
        book
    }

    pub fn len(&self) -> usize {
        self.exact.len() + self.prefix.len() + self.patterns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Exact rule for the full call, else for `base` (call without
    /// operational suffixes).
    pub fn match_exact(&self, full: &str, base: &str) -> Option<&Rule> {
        self.exact.get(full).or_else(|| self.exact.get(base))
    }

    /// Longest prefix rule, else the highest-priority matching pattern.
    pub fn match_prefix(&self, call: &str) -> RuleMatch<'_> {
        let upper = call.len().min(self.max_prefix_len);
        if let Some(rule) = (1..=upper)
            .rev()
            .filter(|&n| call.is_char_boundary(n))
            .find_map(|n| self.prefix.get(&call[..n]))
        {
            return RuleMatch::Matched(rule);
        }
        let matching: Vec<&Rule> = self
            .patterns
            .iter()
            .filter(|(re, _)| re.is_match(call))
            .map(|(_, r)| r)
            .collect();
        let Some(top) = matching.iter().map(|r| r.priority).max() else {
            return RuleMatch::None;
        };
        let best: Vec<&Rule> = matching.into_iter().filter(|r| r.priority == top).collect();
        if best.len() == 1 {
            RuleMatch::Matched(best[0])
        } else {
            RuleMatch::Conflict(best)
        }
    }
}

#[cfg(test)]
mod tests;
