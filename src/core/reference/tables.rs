// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! User-supplied reference tables: IOTA groups, states and administrative
//! subdivisions, QSL managers and a local callbook.
//!
//! # File formats (version 1)
//!
//! UTF-8 CSV with RFC 4180 quoting, the same conventions as the rules format:
//! the first line is a marker, `#` lines are comments, the first other line is
//! the header (columns by name, any order, unknown columns are warnings).
//!
//! | table | marker | columns (required in bold) |
//! |---|---|---|
//! | IOTA | `#splogbook-iota,1` | **`reference`**, **`name`**, `prefix` |
//! | states | `#splogbook-states,1` | **`code`**, **`name`**, **`country`** |
//! | QSL managers | `#splogbook-qsl-managers,1` | **`callsign`**, **`manager`**, `years`, `notes` |
//! | callbook | `#splogbook-callbook,1` | **`callsign`**, `name`, `qth`, `grid`, `state`, `manager`, `notes` |
//!
//! A file is accepted only as a whole; two rows with the same key and
//! different content are a conflict, identical rows are kept once with a
//! warning. Nothing is bundled: every table starts empty (DATA_NOT_INSTALLED)
//! until the user imports a file.

use super::install::{self, InstallError, SetManifest, SetOrigin};
use super::rules::{RuleIssueKind, split_csv_line};
use crate::core::service_db::{IotaRecord, QslManagerRecord, StateRecord};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

pub const TABLE_FILE: &str = "table.csv";
pub const MAX_TABLE_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FIELD_CHARS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableKind {
    Iota,
    States,
    QslManagers,
    Callbook,
}

impl TableKind {
    pub const ALL: [Self; 4] = [Self::Iota, Self::States, Self::QslManagers, Self::Callbook];

    pub fn set_name(self) -> &'static str {
        match self {
            Self::Iota => "iota",
            Self::States => "states",
            Self::QslManagers => "qsl-managers",
            Self::Callbook => "callbook",
        }
    }

    pub fn marker(self) -> &'static str {
        match self {
            Self::Iota => "#splogbook-iota",
            Self::States => "#splogbook-states",
            Self::QslManagers => "#splogbook-qsl-managers",
            Self::Callbook => "#splogbook-callbook",
        }
    }

    fn columns(self) -> &'static [&'static str] {
        match self {
            Self::Iota => &["reference", "name", "prefix"],
            Self::States => &["code", "name", "country"],
            Self::QslManagers => &["callsign", "manager", "years", "notes"],
            Self::Callbook => &[
                "callsign", "name", "qth", "grid", "state", "manager", "notes",
            ],
        }
    }

    fn required(self) -> &'static [usize] {
        match self {
            Self::Iota | Self::QslManagers => &[0, 1],
            Self::States => &[0, 1, 2],
            Self::Callbook => &[0],
        }
    }
}

/// One entry of the user callbook.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallbookEntry {
    pub callsign: String,
    pub name: Option<String>,
    pub qth: Option<String>,
    pub grid: Option<String>,
    pub state: Option<String>,
    pub manager: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableIssueKind {
    Format(RuleIssueKind),
    InvalidValue { column: &'static str, value: String },
    EmptyRow,
    DuplicateRow(String),
    ConflictingRow { key: String, first_line: usize },
}

impl fmt::Display for TableIssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(k) => write!(f, "{k}"),
            Self::InvalidValue { column, value } => write!(f, "invalid {column} '{value}'"),
            Self::EmptyRow => f.write_str("row has no data besides its key"),
            Self::DuplicateRow(k) => write!(f, "duplicate row '{k}' ignored"),
            Self::ConflictingRow { key, first_line } => {
                write!(f, "row '{key}' conflicts with line {first_line}")
            }
        }
    }
}

impl TableIssueKind {
    pub fn is_warning(&self) -> bool {
        matches!(
            self,
            Self::DuplicateRow(_) | Self::Format(RuleIssueKind::UnknownColumn(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableIssue {
    pub line: usize,
    pub kind: TableIssueKind,
}

/// Parsed rows of one table, keyed and validated.
#[derive(Debug, Clone, Default)]
pub struct TableParse {
    pub rows: Vec<Vec<String>>,
    pub issues: Vec<TableIssue>,
}

impl TableParse {
    pub fn errors(&self) -> impl Iterator<Item = &TableIssue> {
        self.issues.iter().filter(|i| !i.kind.is_warning())
    }
    pub fn is_valid(&self) -> bool {
        self.errors().next().is_none()
    }
}

fn is_callsign(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
}

fn is_iota_ref(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 6
        && matches!(&s[..2], "AF" | "AN" | "AS" | "EU" | "NA" | "OC" | "SA")
        && b[2] == b'-'
        && b[3..].iter().all(u8::is_ascii_digit)
}

fn is_locator(s: &str) -> bool {
    let b = s.as_bytes();
    if !matches!(b.len(), 4 | 6 | 8) {
        return false;
    }
    let pair = |i: usize, f: fn(&u8) -> bool| f(&b[i]) && f(&b[i + 1]);
    pair(0, |c| (b'A'..=b'R').contains(c))
        && pair(2, u8::is_ascii_digit)
        && (b.len() < 6 || pair(4, |c| (b'A'..=b'X').contains(c)))
        && (b.len() < 8 || pair(6, u8::is_ascii_digit))
}

/// Normalises and validates one row in column order.
fn validate_row(kind: TableKind, row: &mut [String]) -> Result<(), TableIssueKind> {
    let cols = kind.columns();
    for (i, v) in row.iter_mut().enumerate() {
        *v = v.trim().to_string();
        if v.chars().count() > MAX_FIELD_CHARS {
            return Err(TableIssueKind::InvalidValue {
                column: cols[i],
                value: v.chars().take(32).collect(),
            });
        }
    }
    let bad = |i: usize, row: &[String]| TableIssueKind::InvalidValue {
        column: cols[i],
        value: row[i].clone(),
    };
    for &i in kind.required() {
        if row[i].is_empty() {
            return Err(bad(i, row));
        }
    }
    match kind {
        TableKind::Iota => {
            row[0] = row[0].to_ascii_uppercase();
            if !is_iota_ref(&row[0]) {
                return Err(bad(0, row));
            }
        }
        TableKind::States => {
            row[0] = row[0].to_ascii_uppercase();
        }
        TableKind::QslManagers => {
            row[0] = row[0].to_ascii_uppercase();
            row[1] = row[1].to_ascii_uppercase();
            for i in [0, 1] {
                if !is_callsign(&row[i]) {
                    return Err(bad(i, row));
                }
            }
        }
        TableKind::Callbook => {
            row[0] = row[0].to_ascii_uppercase();
            if !is_callsign(&row[0]) {
                return Err(bad(0, row));
            }
            if !row[3].is_empty() {
                let g = &row[3];
                let normalized = format!(
                    "{}{}",
                    g.get(..2).unwrap_or("").to_ascii_uppercase(),
                    g.get(2..).unwrap_or("").to_ascii_uppercase()
                );
                if !is_locator(&normalized) {
                    return Err(bad(3, row));
                }
                // Keep the conventional mixed case (JO81wa).
                row[3] = match normalized.len() {
                    4 => normalized,
                    _ => format!(
                        "{}{}",
                        &normalized[..4],
                        normalized[4..].to_ascii_lowercase()
                    ),
                };
            }
            if !row[5].is_empty() {
                row[5] = row[5].to_ascii_uppercase();
                if !is_callsign(&row[5]) {
                    return Err(bad(5, row));
                }
            }
            if row[1..].iter().all(String::is_empty) {
                return Err(TableIssueKind::EmptyRow);
            }
        }
    }
    Ok(())
}

/// Row key used for duplicate and conflict detection.
fn row_key(kind: TableKind, row: &[String]) -> String {
    match kind {
        // A state code is unique within its country.
        TableKind::States => format!("{}|{}", row[2].to_uppercase(), row[0]),
        _ => row[0].clone(),
    }
}

/// Parses a table file. Never panics.
pub fn parse_table(kind: TableKind, text: &str) -> TableParse {
    let mut out = TableParse::default();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')));
    let issue = |line, kind| TableIssue { line, kind };

    let first = lines
        .next()
        .map(|(_, l)| l.trim().to_string())
        .unwrap_or_default();
    match first.split_once(',') {
        Some((m, v)) if m.trim() == kind.marker() && v.trim() == "1" => {}
        Some((m, v)) if m.trim() == kind.marker() => {
            out.issues.push(issue(
                1,
                TableIssueKind::Format(RuleIssueKind::UnsupportedVersion(v.trim().to_string())),
            ));
            return out;
        }
        _ => {
            out.issues.push(issue(
                1,
                TableIssueKind::Format(RuleIssueKind::MissingFormatMarker),
            ));
            return out;
        }
    }

    let cols = kind.columns();
    let mut map: Option<Vec<Option<usize>>> = None;
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut lines_of: Vec<usize> = Vec::new();

    for (line_no, raw) in lines {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields = match split_csv_line(raw) {
            Ok(f) => f,
            Err(k) => {
                out.issues.push(issue(line_no, TableIssueKind::Format(k)));
                continue;
            }
        };
        let Some(m) = &map else {
            let mut m = Vec::with_capacity(fields.len());
            let mut have = std::collections::HashSet::new();
            for name in &fields {
                let name = name.trim().to_ascii_lowercase();
                if let Some(idx) = cols.iter().position(|c| *c == name) {
                    if !have.insert(idx) {
                        out.issues.push(issue(
                            line_no,
                            TableIssueKind::Format(RuleIssueKind::DuplicateColumn(name)),
                        ));
                    }
                    m.push(Some(idx));
                } else {
                    out.issues.push(issue(
                        line_no,
                        TableIssueKind::Format(RuleIssueKind::UnknownColumn(name)),
                    ));
                    m.push(None);
                }
            }
            for &r in kind.required() {
                if !have.contains(&r) {
                    out.issues.push(issue(
                        line_no,
                        TableIssueKind::Format(RuleIssueKind::MissingRequiredColumn(cols[r])),
                    ));
                }
            }
            if !out.is_valid() {
                return out;
            }
            map = Some(m);
            continue;
        };
        if fields.len() > m.len() {
            out.issues.push(issue(
                line_no,
                TableIssueKind::Format(RuleIssueKind::TooManyFields(fields.len())),
            ));
            continue;
        }
        let mut row = vec![String::new(); cols.len()];
        for (i, v) in fields.into_iter().enumerate() {
            if let Some(Some(c)) = m.get(i) {
                row[*c] = v;
            }
        }
        if let Err(k) = validate_row(kind, &mut row) {
            out.issues.push(issue(line_no, k));
            continue;
        }
        let key = row_key(kind, &row);
        if let Some(&idx) = seen.get(&key) {
            let k = if out.rows[idx] == row {
                TableIssueKind::DuplicateRow(key)
            } else {
                TableIssueKind::ConflictingRow {
                    key,
                    first_line: lines_of[idx],
                }
            };
            out.issues.push(issue(line_no, k));
        } else {
            seen.insert(key, out.rows.len());
            lines_of.push(line_no);
            out.rows.push(row);
        }
    }
    if map.is_none() {
        out.issues.push(issue(
            1,
            TableIssueKind::Format(RuleIssueKind::MissingHeader),
        ));
    }
    out
}

#[derive(Debug)]
pub enum TableImportError {
    Io(std::io::Error),
    Invalid(Vec<TableIssue>),
    Install(InstallError),
}

impl fmt::Display for TableImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid(i) => write!(f, "{} invalid row(s)", i.len()),
            Self::Install(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TableImportError {}

impl From<std::io::Error> for TableImportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<InstallError> for TableImportError {
    fn from(e: InstallError) -> Self {
        Self::Install(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableImportReport {
    pub rows: usize,
    pub warnings: Vec<TableIssue>,
    pub manifest: SetManifest,
}

fn read_table_text(path: &Path) -> Result<String, TableImportError> {
    let len = std::fs::metadata(path)?.len();
    if len > MAX_TABLE_FILE_BYTES {
        return Err(TableImportError::Invalid(vec![TableIssue {
            line: 1,
            kind: TableIssueKind::Format(RuleIssueKind::TooLarge(len)),
        }]));
    }
    String::from_utf8(std::fs::read(path)?).map_err(|_| {
        TableImportError::Invalid(vec![TableIssue {
            line: 1,
            kind: TableIssueKind::Format(RuleIssueKind::NotUtf8),
        }])
    })
}

/// Validates `source` as a whole and installs it as the `kind` table.
pub fn import_table(
    root: &Path,
    kind: TableKind,
    source: &Path,
) -> Result<TableImportReport, TableImportError> {
    let text = read_table_text(source)?;
    let parsed = parse_table(kind, &text);
    if !parsed.is_valid() {
        return Err(TableImportError::Invalid(
            parsed.errors().cloned().collect(),
        ));
    }
    let rows = parsed.rows.len();
    let origin = SetOrigin {
        source: source
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        ..SetOrigin::default()
    };
    let manifest = install::install_set(
        root,
        kind.set_name(),
        &[(TABLE_FILE.to_string(), text.into_bytes())],
        origin,
        |staged: &Path| -> Result<(), &'static str> {
            let text =
                read_table_text(&staged.join(TABLE_FILE)).map_err(|_| "staged table unreadable")?;
            let p = parse_table(kind, &text);
            if p.is_valid() && p.rows.len() == rows {
                Ok(())
            } else {
                Err("staged table differs from the validated file")
            }
        },
    )?;
    Ok(TableImportReport {
        rows,
        warnings: parsed
            .issues
            .into_iter()
            .filter(|i| i.kind.is_warning())
            .collect(),
        manifest,
    })
}

/// Rows of the installed `kind` table; `Ok(None)` means DATA_NOT_INSTALLED.
pub fn load_table(
    root: &Path,
    kind: TableKind,
) -> Result<Option<(Vec<Vec<String>>, SetManifest)>, TableImportError> {
    install::recover_set(root, kind.set_name())?;
    let Some((dir, manifest)) = install::current_set(root, kind.set_name())? else {
        return Ok(None);
    };
    let parsed = parse_table(kind, &read_table_text(&dir.join(TABLE_FILE))?);
    if !parsed.is_valid() {
        return Err(TableImportError::Invalid(
            parsed.errors().cloned().collect(),
        ));
    }
    Ok(Some((parsed.rows, manifest)))
}

/// Copies the installed `kind` table to `dest` (export / backup).
pub fn export_table(root: &Path, kind: TableKind, dest: &Path) -> Result<u64, TableImportError> {
    match install::current_set(root, kind.set_name())? {
        Some((dir, _)) => Ok(std::fs::copy(dir.join(TABLE_FILE), dest)?),
        None => Err(TableImportError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "table not installed",
        ))),
    }
}

fn opt(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// In-memory, query-ready user tables.
#[derive(Debug, Default)]
pub struct UserTables {
    iota: Vec<IotaRecord>,
    states: Vec<StateRecord>,
    managers: Vec<QslManagerRecord>,
    manager_index: HashMap<String, usize>,
    callbook: HashMap<String, CallbookEntry>,
    pub manifests: HashMap<TableKind, SetManifest>,
}

impl UserTables {
    /// Loads every installed table; a broken table is reported and skipped.
    pub fn load(root: &Path) -> (Self, Vec<(TableKind, TableImportError)>) {
        let mut t = Self::default();
        let mut failures = Vec::new();
        for kind in TableKind::ALL {
            match load_table(root, kind) {
                Ok(Some((rows, manifest))) => {
                    t.set_rows(kind, rows);
                    t.manifests.insert(kind, manifest);
                }
                Ok(None) => {}
                Err(e) => failures.push((kind, e)),
            }
        }
        (t, failures)
    }

    fn set_rows(&mut self, kind: TableKind, rows: Vec<Vec<String>>) {
        match kind {
            TableKind::Iota => {
                self.iota = rows
                    .into_iter()
                    .map(|r| IotaRecord {
                        iota: r[0].clone(),
                        name: r[1].clone(),
                        prefix: r[2].clone(),
                    })
                    .collect();
                self.iota.sort_by(|a, b| a.iota.cmp(&b.iota));
            }
            TableKind::States => {
                self.states = rows
                    .into_iter()
                    .map(|r| StateRecord {
                        code: r[0].clone(),
                        name: r[1].clone(),
                        country: r[2].clone(),
                    })
                    .collect();
            }
            TableKind::QslManagers => {
                self.managers = rows
                    .into_iter()
                    .map(|r| QslManagerRecord {
                        call: r[0].clone(),
                        manager: r[1].clone(),
                        years: r[2].clone(),
                        notes: r[3].clone(),
                    })
                    .collect();
                self.managers.sort_by(|a, b| a.call.cmp(&b.call));
                self.manager_index = self
                    .managers
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (m.call.clone(), i))
                    .collect();
            }
            TableKind::Callbook => {
                self.callbook = rows
                    .into_iter()
                    .map(|r| {
                        (
                            r[0].clone(),
                            CallbookEntry {
                                callsign: r[0].clone(),
                                name: opt(&r[1]),
                                qth: opt(&r[2]),
                                grid: opt(&r[3]),
                                state: opt(&r[4]),
                                manager: opt(&r[5]),
                                notes: opt(&r[6]),
                            },
                        )
                    })
                    .collect();
            }
        }
    }

    pub fn is_installed(&self, kind: TableKind) -> bool {
        self.manifests.contains_key(&kind)
    }

    pub fn row_count(&self, kind: TableKind) -> usize {
        match kind {
            TableKind::Iota => self.iota.len(),
            TableKind::States => self.states.len(),
            TableKind::QslManagers => self.managers.len(),
            TableKind::Callbook => self.callbook.len(),
        }
    }

    /// Same semantics as the legacy query: code or name contains the
    /// (upper-cased) text, ordered by code, at most 100 rows.
    pub fn search_iota(&self, upper_query: &str) -> Vec<IotaRecord> {
        self.iota
            .iter()
            .filter(|r| r.iota.contains(upper_query) || r.name.to_uppercase().contains(upper_query))
            .take(100)
            .cloned()
            .collect()
    }

    pub fn find_iota(&self, upper_code: &str) -> Option<IotaRecord> {
        self.iota.iter().find(|r| r.iota == upper_code).cloned()
    }

    pub fn search_states(&self, upper_query: &str) -> Vec<StateRecord> {
        self.states
            .iter()
            .filter(|r| r.code.contains(upper_query) || r.name.to_uppercase().contains(upper_query))
            .take(100)
            .cloned()
            .collect()
    }

    pub fn find_manager(&self, upper_call: &str) -> Option<QslManagerRecord> {
        self.manager_index
            .get(upper_call)
            .map(|&i| self.managers[i].clone())
    }

    pub fn search_managers(&self, upper_query: &str) -> Vec<QslManagerRecord> {
        self.managers
            .iter()
            .filter(|r| r.call.contains(upper_query) || r.manager.contains(upper_query))
            .take(100)
            .cloned()
            .collect()
    }

    pub fn find_callbook(&self, upper_call: &str) -> Option<&CallbookEntry> {
        self.callbook.get(upper_call)
    }
}

#[cfg(test)]
mod tests;
