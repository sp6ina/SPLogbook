// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł automatycznego rozpoznawania przynależności stacji do klubów krótkofalarskich:
//! SP-OTC (SP Old Timers Club), SPCWC (SP CW Club), CWOPS (CW Operators' Club),
//! FOC (First Class CW Operators' Club), HSC (High Speed Club) oraz SKCC (Straight Key Century Club).
//!
//! Członkostwo i numery członkowskie są ładowane dynamicznie z pliku
//! referencyjnego użytkownika w katalogu danych aplikacji.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Informacja o odznace klubowej
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClubAffiliation {
    pub code: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub number: Option<Cow<'static, str>>,
    pub badge_color: (u8, u8, u8), // RGB
}

impl ClubAffiliation {
    pub fn new(
        code: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        number: Option<impl Into<Cow<'static, str>>>,
        badge_color: (u8, u8, u8),
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            number: number.map(Into::into),
            badge_color,
        }
    }
}

/// Rejestr członków prestiżowych klubów krótkofalarskich
pub struct ClubRegistry;

type Registry = HashMap<String, Vec<ClubAffiliation>>;

static GLOBAL_REGISTRY: RwLock<Option<Arc<Registry>>> = RwLock::new(None);

impl ClubRegistry {
    /// Bieżący rejestr: plik użytkownika w katalogu danych lub pusty rejestr.
    pub fn load_or_default() -> Arc<Registry> {
        if let Some(reg) = GLOBAL_REGISTRY
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return reg.clone();
        }
        let root = crate::core::reference::loader::context().map(|c| c.root.clone());
        Self::reload(root.as_deref())
    }

    /// Wczytuje rejestr ponownie (np. po imporcie lub zapisie) i zwraca go.
    pub fn reload(reference_root: Option<&Path>) -> Arc<Registry> {
        let reg = Arc::new(Self::resolve_sources(reference_root));
        *GLOBAL_REGISTRY
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(reg.clone());
        reg
    }

    pub(crate) fn resolve_sources(reference_root: Option<&Path>) -> Registry {
        if let Some(root) = reference_root {
            match load_user_clubs(root) {
                Ok(Some((rows, _))) => return registry_from_rows(&rows),
                Ok(None) => {}
                Err(e) => log::warn!("user clubs file not loaded: {e}"),
            }
        }
        Self::default_registry()
    }

    /// Wczytuje powiązania ze wskazanego pliku CSV z kluczowaniem relacji i walidacją unikalności.
    pub fn load_from_csv(
        path: &Path,
    ) -> Result<HashMap<String, Vec<ClubAffiliation>>, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let mut map: HashMap<String, Vec<ClubAffiliation>> = HashMap::new();
        let mut seen_memberships: HashMap<(String, String), Option<String>> = HashMap::new();
        let mut club_numbers: HashMap<(String, String), String> = HashMap::new();

        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || (idx == 0 && line.starts_with("callsign")) {
                continue;
            }

            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            if parts.len() < 7 {
                continue;
            }

            let callsign = parts[0].to_uppercase();
            let code = parts[1].to_string();
            let name = parts[2].to_string();
            if callsign.is_empty() || code.is_empty() {
                continue;
            }

            let number = if parts[3].is_empty() || parts[3] == "-" {
                None
            } else {
                Some(parts[3].to_string())
            };

            // PRIMARY MEMBERSHIP KEY: (club_id, normalized_callsign)
            let membership_key = (code.clone(), callsign.clone());
            if let Some(prev_num) = seen_memberships.get(&membership_key) {
                // Rekord dla tej tożsamości członkostwa już istnieje:
                // - jeśli ten sam numer: idempotentny duplikat (pomijany bez błędu)
                // - jeśli inny numer: sprzeczność/konflikt członkostwa (odrzucany)
                let _ = prev_num;
                continue;
            }

            // MEMBER NUMBER CONFLICT KEY: (club_id, normalized_member_number)
            if let Some(ref num) = number {
                let club_num_key = (code.clone(), num.clone());
                if let Some(existing_call) = club_numbers.get(&club_num_key)
                    && existing_call != &callsign
                {
                    // Ten sam numer w tym samym klubie należy już do innego znaku -> konflikt
                    continue;
                }
                club_numbers.insert(club_num_key, callsign.clone());
            }

            seen_memberships.insert(membership_key, number.clone());

            let r = parts[4].parse::<u8>().unwrap_or(59);
            let g = parts[5].parse::<u8>().unwrap_or(130);
            let b = parts[6].parse::<u8>().unwrap_or(246);

            let affiliation = ClubAffiliation {
                code: Cow::Owned(code),
                name: Cow::Owned(name),
                number: number.map(Cow::Owned),
                badge_color: (r, g, b),
            };

            map.entry(callsign).or_default().push(affiliation);
        }

        Ok(map)
    }

    /// Zwraca pusty domyślny rejestr klubów (brak zahardkodowanych znaków).
    pub fn default_registry() -> HashMap<String, Vec<ClubAffiliation>> {
        HashMap::new()
    }

    /// Sprawdza przynależność znaku do klubów krótkofalarskich.
    pub fn check(callsign: &str) -> Vec<ClubAffiliation> {
        let clean = callsign.trim().to_uppercase();
        let registry = Self::load_or_default();
        registry.get(&clean).cloned().unwrap_or_default()
    }
}

/// One membership row in the clubs file (same columns as before:
/// `callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubRow {
    pub callsign: String,
    pub code: String,
    pub name: String,
    pub number: Option<String>,
    pub color: (u8, u8, u8),
}

pub const CLUBS_SET: &str = "clubs";
pub const CLUBS_FILE: &str = "clubs.csv";
pub const CLUBS_HEADER: &str = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClubIssueKind {
    ColumnCount(usize),
    EmptyField(&'static str),
    InvalidCallsign(String),
    InvalidColor(String),
    TextTooLong(&'static str),
    /// The same membership twice with the same number (kept once).
    DuplicateMembership {
        callsign: String,
        code: String,
    },
    /// The same membership with a different number.
    ConflictingMembership {
        callsign: String,
        code: String,
        first_line: usize,
    },
    /// A member number already used by another callsign in the same club.
    NumberTaken {
        number: String,
        code: String,
        first_line: usize,
    },
    NotUtf8,
}

impl std::fmt::Display for ClubIssueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ColumnCount(n) => write!(f, "expected 7 columns, found {n}"),
            Self::EmptyField(c) => write!(f, "{c} is empty"),
            Self::InvalidCallsign(c) => write!(f, "invalid callsign '{c}'"),
            Self::InvalidColor(c) => write!(f, "invalid colour component '{c}'"),
            Self::TextTooLong(c) => write!(f, "{c} too long"),
            Self::DuplicateMembership { callsign, code } => {
                write!(f, "duplicate membership {callsign}/{code} ignored")
            }
            Self::ConflictingMembership {
                callsign,
                code,
                first_line,
            } => write!(
                f,
                "membership {callsign}/{code} conflicts with line {first_line}"
            ),
            Self::NumberTaken {
                number,
                code,
                first_line,
            } => write!(
                f,
                "number {number} in {code} already used on line {first_line}"
            ),
            Self::NotUtf8 => f.write_str("file is not valid UTF-8"),
        }
    }
}

impl ClubIssueKind {
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::DuplicateMembership { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubIssue {
    pub line: usize,
    pub kind: ClubIssueKind,
}

#[derive(Debug, Clone, Default)]
pub struct ClubsParse {
    pub rows: Vec<ClubRow>,
    pub issues: Vec<ClubIssue>,
}

impl ClubsParse {
    pub fn errors(&self) -> impl Iterator<Item = &ClubIssue> {
        self.issues.iter().filter(|i| !i.kind.is_warning())
    }
    pub fn is_valid(&self) -> bool {
        self.errors().next().is_none()
    }
}

/// Strict parser used for imports and saves: every problem is reported with
/// its line; nothing is skipped silently.
pub fn parse_clubs_csv(text: &str) -> ClubsParse {
    let mut out = ClubsParse::default();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut memberships: HashMap<(String, String), (usize, Option<String>)> = HashMap::new();
    let mut numbers: HashMap<(String, String), (usize, String)> = HashMap::new();
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || (idx == 0 && line.starts_with("callsign")) {
            continue;
        }
        let issue = |kind| ClubIssue {
            line: line_no,
            kind,
        };
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() != 7 {
            out.issues
                .push(issue(ClubIssueKind::ColumnCount(parts.len())));
            continue;
        }
        let callsign = parts[0].to_uppercase();
        if callsign.is_empty() {
            out.issues
                .push(issue(ClubIssueKind::EmptyField("callsign")));
            continue;
        }
        if callsign.len() > 32
            || !callsign
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '/')
        {
            out.issues
                .push(issue(ClubIssueKind::InvalidCallsign(parts[0].to_string())));
            continue;
        }
        if parts[1].is_empty() {
            out.issues
                .push(issue(ClubIssueKind::EmptyField("club_code")));
            continue;
        }
        if parts[2].is_empty() {
            out.issues
                .push(issue(ClubIssueKind::EmptyField("club_name")));
            continue;
        }
        if parts[1].chars().count() > 32
            || parts[2].chars().count() > 128
            || parts[3].chars().count() > 32
        {
            out.issues.push(issue(ClubIssueKind::TextTooLong("club")));
            continue;
        }
        let mut color = [0u8; 3];
        let mut bad_color = None;
        for (i, c) in color.iter_mut().enumerate() {
            match parts[4 + i].parse::<u8>() {
                Ok(v) => *c = v,
                Err(_) => bad_color = Some(parts[4 + i].to_string()),
            }
        }
        if let Some(v) = bad_color {
            out.issues.push(issue(ClubIssueKind::InvalidColor(v)));
            continue;
        }
        let number = (!parts[3].is_empty() && parts[3] != "-").then(|| parts[3].to_string());
        let code = parts[1].to_string();
        let key = (code.clone(), callsign.clone());
        if let Some((first_line, prev)) = memberships.get(&key) {
            let kind = if *prev == number {
                ClubIssueKind::DuplicateMembership { callsign, code }
            } else {
                ClubIssueKind::ConflictingMembership {
                    callsign,
                    code,
                    first_line: *first_line,
                }
            };
            out.issues.push(issue(kind));
            continue;
        }
        if let Some(n) = &number {
            if let Some((first_line, other)) = numbers.get(&(code.clone(), n.clone()))
                && *other != callsign
            {
                out.issues.push(issue(ClubIssueKind::NumberTaken {
                    number: n.clone(),
                    code,
                    first_line: *first_line,
                }));
                continue;
            }
            numbers.insert((code.clone(), n.clone()), (line_no, callsign.clone()));
        }
        memberships.insert(key, (line_no, number.clone()));
        out.rows.push(ClubRow {
            callsign,
            code,
            name: parts[2].to_string(),
            number,
            color: (color[0], color[1], color[2]),
        });
    }
    out
}

/// Serialises rows in the clubs file format (header + one row per line).
pub fn rows_to_csv(rows: &[ClubRow]) -> String {
    let mut out = String::from(CLUBS_HEADER);
    out.push('\n');
    for r in rows {
        use std::fmt::Write as _;
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{}",
            r.callsign,
            r.code,
            r.name,
            r.number.as_deref().unwrap_or(""),
            r.color.0,
            r.color.1,
            r.color.2
        );
    }
    out
}

/// Builds the lookup registry from rows.
pub fn registry_from_rows(rows: &[ClubRow]) -> Registry {
    let mut map: Registry = HashMap::new();
    for r in rows {
        map.entry(r.callsign.clone())
            .or_default()
            .push(ClubAffiliation {
                code: Cow::Owned(r.code.clone()),
                name: Cow::Owned(r.name.clone()),
                number: r.number.clone().map(Cow::Owned),
                badge_color: r.color,
            });
    }
    map
}

#[derive(Debug)]
pub enum ClubsStoreError {
    Io(std::io::Error),
    Invalid(Vec<ClubIssue>),
    Install(crate::core::reference::install::InstallError),
}

impl std::fmt::Display for ClubsStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid(i) => write!(f, "{} invalid row(s)", i.len()),
            Self::Install(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ClubsStoreError {}

impl From<std::io::Error> for ClubsStoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<crate::core::reference::install::InstallError> for ClubsStoreError {
    fn from(e: crate::core::reference::install::InstallError) -> Self {
        Self::Install(e)
    }
}

fn install_clubs_text(
    root: &Path,
    text: String,
    source: String,
) -> Result<(Vec<ClubRow>, Vec<ClubIssue>), ClubsStoreError> {
    let parsed = parse_clubs_csv(&text);
    if !parsed.is_valid() {
        return Err(ClubsStoreError::Invalid(parsed.errors().cloned().collect()));
    }
    let count = parsed.rows.len();
    crate::core::reference::install::install_set(
        root,
        CLUBS_SET,
        &[(CLUBS_FILE.to_string(), text.into_bytes())],
        crate::core::reference::install::SetOrigin {
            source,
            ..Default::default()
        },
        |staged: &Path| -> Result<(), &'static str> {
            let t = std::fs::read_to_string(staged.join(CLUBS_FILE))
                .map_err(|_| "staged clubs unreadable")?;
            let p = parse_clubs_csv(&t);
            if p.is_valid() && p.rows.len() == count {
                Ok(())
            } else {
                Err("staged clubs differ from the validated file")
            }
        },
    )?;
    let warnings = parsed
        .issues
        .into_iter()
        .filter(|i| i.kind.is_warning())
        .collect();
    Ok((parsed.rows, warnings))
}

/// Imports a clubs file chosen by the user (same format as before).
pub fn import_clubs(
    root: &Path,
    source: &Path,
) -> Result<(Vec<ClubRow>, Vec<ClubIssue>), ClubsStoreError> {
    let bytes = std::fs::read(source)?;
    let text = String::from_utf8(bytes).map_err(|_| {
        ClubsStoreError::Invalid(vec![ClubIssue {
            line: 1,
            kind: ClubIssueKind::NotUtf8,
        }])
    })?;
    let name = source
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    install_clubs_text(root, text, name)
}

/// Saves rows edited by the user.
pub fn save_clubs(root: &Path, rows: &[ClubRow]) -> Result<(), ClubsStoreError> {
    install_clubs_text(root, rows_to_csv(rows), "edited in SPLogbook".to_string()).map(|_| ())
}

/// Rows of the user clubs file; `Ok(None)` when none is installed.
pub fn load_user_clubs(
    root: &Path,
) -> Result<Option<(Vec<ClubRow>, crate::core::reference::install::SetManifest)>, ClubsStoreError> {
    crate::core::reference::install::recover_set(root, CLUBS_SET)?;
    let Some((dir, manifest)) = crate::core::reference::install::current_set(root, CLUBS_SET)?
    else {
        return Ok(None);
    };
    let parsed = parse_clubs_csv(&std::fs::read_to_string(dir.join(CLUBS_FILE))?);
    if !parsed.is_valid() {
        return Err(ClubsStoreError::Invalid(parsed.errors().cloned().collect()));
    }
    Ok(Some((parsed.rows, manifest)))
}

/// Copies the user clubs file to `dest` (export / backup).
pub fn export_clubs(root: &Path, dest: &Path) -> Result<u64, ClubsStoreError> {
    match crate::core::reference::install::current_set(root, CLUBS_SET)? {
        Some((dir, _)) => Ok(std::fs::copy(dir.join(CLUBS_FILE), dest)?),
        None => Err(ClubsStoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "clubs not installed",
        ))),
    }
}

/// Rows currently in effect, for the editor: the user file when installed,
/// otherwise the active registry flattened into rows.
pub fn current_rows(reference_root: Option<&Path>) -> Vec<ClubRow> {
    if let Some(root) = reference_root
        && let Ok(Some((rows, _))) = load_user_clubs(root)
    {
        return rows;
    }
    let reg = ClubRegistry::load_or_default();
    let mut rows: Vec<ClubRow> = reg
        .iter()
        .flat_map(|(call, affs)| {
            affs.iter().map(move |a| ClubRow {
                callsign: call.clone(),
                code: a.code.to_string(),
                name: a.name.to_string(),
                number: a.number.as_ref().map(ToString::to_string),
                color: a.badge_color,
            })
        })
        .collect();
    rows.sort_by(|a, b| (&a.code, &a.callsign).cmp(&(&b.code, &b.callsign)));
    rows
}

#[cfg(test)]
mod store_tests {
    use super::*;

    const CSV: &str = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\nqa1aa,QCLUB,Test Club,#1,10,20,30\nQA2AA,QCLUB,Test Club,,10,20,30\nQA1AA,QOTHER,Other Club,#1,1,2,3\n";

    #[test]
    fn strict_parser_accepts_the_existing_format() {
        let p = parse_clubs_csv(CSV);
        assert!(p.is_valid(), "{:?}", p.issues);
        assert_eq!(p.rows.len(), 3);
        assert_eq!(p.rows[0].callsign, "QA1AA");
        assert_eq!(p.rows[1].number, None);
        let reg = registry_from_rows(&p.rows);
        assert_eq!(reg["QA1AA"].len(), 2);
        // Round trip through the same format.
        let again = parse_clubs_csv(&rows_to_csv(&p.rows));
        assert_eq!(again.rows, p.rows);
    }

    #[test]
    fn every_problem_is_reported_with_its_line() {
        let text = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
QA1AA,QCLUB,Test Club,#1,10,20,30\n\
QA2AA,QCLUB,Test Club,#1,10,20,30\n\
QA1AA,QCLUB,Test Club,#9,10,20,30\n\
QA1AA,QCLUB,Test Club,#1,10,20,30\n\
QA3AA,QCLUB,Test Club,#3,300,20,30\n\
QA 4,QCLUB,Test Club,#4,1,2,3\n\
QA5AA,,Test Club,#5,1,2,3\n\
QA6AA,QCLUB,Test Club\n";
        let p = parse_clubs_csv(text);
        let kinds: Vec<(usize, bool)> = p
            .issues
            .iter()
            .map(|i| (i.line, i.kind.is_warning()))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (3, false),
                (4, false),
                (5, true),
                (6, false),
                (7, false),
                (8, false),
                (9, false)
            ]
        );
        assert!(!p.is_valid());
        assert_eq!(p.rows.len(), 1);
    }

    #[test]
    fn import_edit_save_reload_and_export() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        assert!(load_user_clubs(root.path()).unwrap().is_none());
        let file = src.path().join("clubs.csv");
        std::fs::write(&file, CSV).unwrap();
        let (rows, warnings) = import_clubs(root.path(), &file).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(warnings, Vec::<ClubIssue>::new());

        let mut edited = rows;
        edited.retain(|r| r.code != "QOTHER");
        edited.push(ClubRow {
            callsign: "QA9ZZ".into(),
            code: "QCLUB".into(),
            name: "Test Club".into(),
            number: Some("#9".into()),
            color: (1, 1, 1),
        });
        save_clubs(root.path(), &edited).unwrap();
        let (loaded, m) = load_user_clubs(root.path()).unwrap().unwrap();
        assert_eq!(loaded, edited);
        assert_eq!(m.origin.source, "edited in SPLogbook");
        assert_eq!(current_rows(Some(root.path())), edited);

        // An invalid save is rejected and the stored file is unchanged.
        let mut broken = edited.clone();
        broken.push(broken[0].clone());
        broken.last_mut().unwrap().number = Some("#77".into());
        assert!(matches!(
            save_clubs(root.path(), &broken),
            Err(ClubsStoreError::Invalid(_))
        ));
        assert_eq!(load_user_clubs(root.path()).unwrap().unwrap().0, edited);

        let out = src.path().join("backup.csv");
        export_clubs(root.path(), &out).unwrap();
        assert_eq!(
            parse_clubs_csv(&std::fs::read_to_string(out).unwrap()).rows,
            edited
        );
    }

    #[test]
    fn registry_prefers_the_user_file_and_reloads() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let file = src.path().join("c.csv");
        std::fs::write(&file, CSV).unwrap();
        import_clubs(root.path(), &file).unwrap();
        // Pure resolution: the process-wide registry is not touched.
        let reg = ClubRegistry::resolve_sources(Some(root.path()));
        assert_eq!(reg["QA2AA"][0].code, "QCLUB");
        // An empty user file gives an empty, valid registry.
        save_clubs(root.path(), &[]).unwrap();
        assert!(ClubRegistry::resolve_sources(Some(root.path())).is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const SYNTH_CLUBS_CSV: &str = "\
callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b
qa1aa,SPCWC,SP CW Club,#480,59,130,246
QA2BB,CWOPS,CW Operators' Club,#001,245,158,11
QA2BB,FOC,First Class CW Operators' Club,#1850,147,51,234
";

    #[test]
    fn club_registry_loads_synthetic_csv_in_memory() {
        let parsed = parse_clubs_csv(SYNTH_CLUBS_CSV);
        assert!(parsed.is_valid(), "{:?}", parsed.issues);
        let reg = registry_from_rows(&parsed.rows);
        assert!(!reg.is_empty());
        assert!(reg.contains_key("QA1AA"));
        assert!(reg.contains_key("QA2BB"));
    }

    #[test]
    fn club_registry_default_is_empty_without_hardcoded_callsigns() {
        assert!(ClubRegistry::default_registry().is_empty());
        assert!(ClubRegistry::resolve_sources(None).is_empty());
    }

    #[test]
    fn club_registry_normalizes_callsign() {
        let parsed = parse_clubs_csv(
            "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n  qa1aa  ,QCLUB,Test Club,#1,10,20,30\n",
        );
        assert!(parsed.is_valid(), "{:?}", parsed.issues);
        let reg = registry_from_rows(&parsed.rows);
        assert_eq!(reg.get("QA1AA").map(Vec::len), Some(1));
        assert_eq!(
            ClubRegistry::check("QA1AA"),
            ClubRegistry::check("  qa1aa  ")
        );
    }

    #[test]
    fn club_registry_lookup_uses_normalized_callsign() {
        let parsed = parse_clubs_csv(SYNTH_CLUBS_CSV);
        let reg = registry_from_rows(&parsed.rows);
        let lookup = |call: &str| -> Vec<ClubAffiliation> {
            reg.get(&call.trim().to_uppercase())
                .cloned()
                .unwrap_or_default()
        };
        let affs_upper = lookup("QA1AA");
        let affs_mixed = lookup("  qA1aA  ");
        assert_eq!(affs_upper, affs_mixed);
        assert_ne!(affs_mixed, []);
    }

    #[test]
    fn club_membership_primary_key_is_club_and_callsign() {
        let parsed = parse_clubs_csv(SYNTH_CLUBS_CSV);
        let reg = registry_from_rows(&parsed.rows);
        let affs = &reg["QA2BB"];
        let codes: Vec<_> = affs.iter().map(|a| a.code.as_ref()).collect();
        assert!(codes.contains(&"CWOPS"));
        assert!(codes.contains(&"FOC"));
    }

    #[test]
    fn club_registry_allows_one_callsign_in_multiple_clubs() {
        let parsed = parse_clubs_csv(SYNTH_CLUBS_CSV);
        let reg = registry_from_rows(&parsed.rows);
        let affs = &reg["QA2BB"];
        assert!(affs.len() >= 2);
        let mut codes = HashSet::new();
        for a in affs {
            assert!(codes.insert(a.code.as_ref()));
        }
    }

    #[test]
    fn club_registry_keys_membership_by_club_and_callsign() {
        let parsed = parse_clubs_csv(SYNTH_CLUBS_CSV);
        let reg = registry_from_rows(&parsed.rows);
        let affs = &reg["QA2BB"];
        let codes: Vec<_> = affs.iter().map(|a| a.code.as_ref()).collect();
        assert!(codes.contains(&"CWOPS"));
        assert!(codes.contains(&"FOC"));
    }

    #[test]
    fn club_registry_allows_same_member_number_in_different_clubs() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_diff_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SPCWC,SP CW Club,#104,59,130,246\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        assert_eq!(loaded["SP2BBB"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_member_number_conflict_key_is_scoped_to_club() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_scope_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,CLUB_A,Alpha Club,#42,239,68,68\n\
                           SP2BBB,CLUB_B,Beta Club,#42,59,130,246\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#42"));
        assert_eq!(loaded["SP2BBB"][0].number.as_deref(), Some("#42"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_same_number_for_different_callsigns_in_same_club() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_diffcall_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SP-OTC,SP Old Timers Club,#104,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert!(!loaded.contains_key("SP2BBB"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_conflicting_duplicate_within_same_club() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_conf_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SP-OTC,SP Old Timers Club,#104,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert!(!loaded.contains_key("SP2BBB"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_accepts_idempotent_duplicate() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_idem_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_same_membership_with_conflicting_number() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_same_conf_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#999,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_rejects_conflicting_membership_for_same_key() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_key_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#999,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded["SP1AAA"][0].number.as_deref(), Some("#104"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_handles_missing_reference_file_safely() {
        let res = ClubRegistry::load_from_csv(Path::new("definitely_missing_file_xyz.csv"));
        assert!(res.is_err());
    }

    #[test]
    fn club_registry_handles_malformed_record_without_crash() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_malf_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           INVALID_LINE_WITHOUT_COMMAS\n\
                           TOO,FEW,COLUMNS\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,not_a_num,not_a_num,246\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded.get("SP1AAA").unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_does_not_invent_member_number() {
        let affs = ClubRegistry::check("ZZ0NONEXISTENT");
        assert_eq!(affs, []);
    }

    #[test]
    fn club_registry_duplicate_policy_is_deterministic() {
        let temp_dir =
            std::env::temp_dir().join(format!("splog_clubs_deter_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let csv_file = temp_dir.join("test_clubs.csv");
        let csv_content = "callsign,club_code,club_name,member_number,badge_r,badge_g,badge_b\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP2BBB,SP-OTC,SP Old Timers Club,#104,239,68,68\n\
                           SP1AAA,SP-OTC,SP Old Timers Club,#999,239,68,68\n";
        std::fs::write(&csv_file, csv_content).unwrap();

        let loaded1 = ClubRegistry::load_from_csv(&csv_file).unwrap();
        let loaded2 = ClubRegistry::load_from_csv(&csv_file).unwrap();
        assert_eq!(loaded1, loaded2);
        assert_eq!(loaded1.get("SP1AAA").unwrap().len(), 1);
        assert_eq!(loaded1["SP1AAA"][0].number.as_deref(), Some("#104"));
        assert!(!loaded1.contains_key("SP2BBB"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn club_registry_lookup_is_deterministic() {
        let parsed = parse_clubs_csv(SYNTH_CLUBS_CSV);
        let reg1 = registry_from_rows(&parsed.rows);
        let reg2 = registry_from_rows(&parsed.rows);
        assert_eq!(reg1.get("QA1AA"), reg2.get("QA1AA"));
    }
}
