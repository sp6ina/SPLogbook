// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Installing the contest country files (`cty.dat` + `cty.csv`) from an
//! official archive or from files the user selected.
//!
//! Archives are read in memory with explicit limits; only the two expected
//! entries are read and nothing is extracted to disk. The validated pair is
//! installed atomically as the `country-files` set; on any error the
//! previously installed set keeps working.

use super::cty::{self, CtyData, CtyError, EntityCodeReport};
use super::install::{self, InstallError, SetManifest, SetOrigin};
use crate::core::prefix::{CountryFileReport, PrefixMatcher};
use std::fmt;
use std::io::Read;
use std::path::Path;

pub const SET_NAME: &str = "country-files";
pub const DAT_FILE: &str = "cty.dat";
pub const CSV_FILE: &str = "cty.csv";

/// Default source for the user-triggered download: the "Big CTY" archive,
/// which lists all zone overrides needed for everyday logging.
pub const BIG_CTY_ARCHIVE_URL: &str = "https://www.country-files.com/bigcty/download/bigcty.zip";

/// Limits applied before and while reading an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub max_archive_bytes: u64,
    pub max_entries: usize,
    pub max_entry_bytes: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 16 * 1024 * 1024,
            max_entries: 256,
            max_entry_bytes: cty::MAX_COUNTRY_FILE_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    TooLarge(u64),
    NotZip(String),
    TooManyEntries(usize),
    UnsafeEntryName(String),
    DuplicateEntry(&'static str),
    MissingEntry(&'static str),
    Encrypted(&'static str),
    UnsupportedCompression(&'static str),
    EntryTooLarge { name: &'static str, size: u64 },
    Read { name: &'static str, error: String },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(n) => write!(f, "archive too large ({n} bytes)"),
            Self::NotZip(e) => write!(f, "not a ZIP archive: {e}"),
            Self::TooManyEntries(n) => write!(f, "archive has too many entries ({n})"),
            Self::UnsafeEntryName(n) => write!(f, "unsafe entry name '{n}'"),
            Self::DuplicateEntry(n) => write!(f, "entry '{n}' appears more than once"),
            Self::MissingEntry(n) => write!(f, "entry '{n}' missing"),
            Self::Encrypted(n) => write!(f, "entry '{n}' is encrypted"),
            Self::UnsupportedCompression(n) => {
                write!(f, "entry '{n}' uses an unsupported compression method")
            }
            Self::EntryTooLarge { name, size } => {
                write!(f, "entry '{name}' too large ({size} bytes)")
            }
            Self::Read { name, error } => write!(f, "cannot read '{name}': {error}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

/// The two files of a country-file release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountryFilesPayload {
    pub dat: Vec<u8>,
    pub csv: Option<Vec<u8>>,
}

/// `true` for names that would escape a target directory if extracted:
/// absolute paths, drive prefixes, `..` components, NUL or control bytes.
fn unsafe_entry_name(name: &str) -> bool {
    name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.as_bytes().get(1) == Some(&b':')
        || name.chars().any(char::is_control)
        || name.split(['/', '\\']).any(|part| part == "..")
}

/// Reads `cty.dat` (required) and `cty.csv` (required when `require_csv`)
/// from the top level of a ZIP archive held in memory.
pub fn read_country_archive(
    bytes: &[u8],
    limits: &ArchiveLimits,
    require_csv: bool,
) -> Result<CountryFilesPayload, ArchiveError> {
    if bytes.len() as u64 > limits.max_archive_bytes {
        return Err(ArchiveError::TooLarge(bytes.len() as u64));
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| ArchiveError::NotZip(e.to_string()))?;
    if archive.len() > limits.max_entries {
        return Err(ArchiveError::TooManyEntries(archive.len()));
    }

    // First pass: names only; refuse the whole archive on any unsafe name or
    // duplicated wanted entry.
    let mut dat_index = None;
    let mut csv_index = None;
    for i in 0..archive.len() {
        let entry = archive
            .by_index_raw(i)
            .map_err(|e| ArchiveError::NotZip(e.to_string()))?;
        let name = String::from_utf8_lossy(entry.name_raw()).into_owned();
        if unsafe_entry_name(&name) {
            return Err(ArchiveError::UnsafeEntryName(name));
        }
        let slot = if name.eq_ignore_ascii_case(DAT_FILE) {
            Some((&mut dat_index, DAT_FILE))
        } else if name.eq_ignore_ascii_case(CSV_FILE) {
            Some((&mut csv_index, CSV_FILE))
        } else {
            None
        };
        if let Some((slot, wanted)) = slot {
            if slot.is_some() {
                return Err(ArchiveError::DuplicateEntry(wanted));
            }
            *slot = Some(i);
        }
    }

    let dat_index = dat_index.ok_or(ArchiveError::MissingEntry(DAT_FILE))?;
    if require_csv && csv_index.is_none() {
        return Err(ArchiveError::MissingEntry(CSV_FILE));
    }

    let dat = read_entry(&mut archive, dat_index, DAT_FILE, limits)?;
    let csv = csv_index
        .map(|i| read_entry(&mut archive, i, CSV_FILE, limits))
        .transpose()?;
    Ok(CountryFilesPayload { dat, csv })
}

fn read_entry(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    index: usize,
    name: &'static str,
    limits: &ArchiveLimits,
) -> Result<Vec<u8>, ArchiveError> {
    {
        let raw = archive
            .by_index_raw(index)
            .map_err(|e| ArchiveError::Read {
                name,
                error: e.to_string(),
            })?;
        if raw.encrypted() {
            return Err(ArchiveError::Encrypted(name));
        }
        if !matches!(
            raw.compression(),
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        ) {
            return Err(ArchiveError::UnsupportedCompression(name));
        }
        if raw.size() > limits.max_entry_bytes {
            return Err(ArchiveError::EntryTooLarge {
                name,
                size: raw.size(),
            });
        }
    }
    let entry = archive.by_index(index).map_err(|e| ArchiveError::Read {
        name,
        error: e.to_string(),
    })?;
    // Never trust the declared size: read at most one byte past the limit.
    let mut out = Vec::new();
    entry
        .take(limits.max_entry_bytes + 1)
        .read_to_end(&mut out)
        .map_err(|e| ArchiveError::Read {
            name,
            error: e.to_string(),
        })?;
    if out.len() as u64 > limits.max_entry_bytes {
        return Err(ArchiveError::EntryTooLarge {
            name,
            size: out.len() as u64,
        });
    }
    Ok(out)
}

#[derive(Debug)]
pub enum CountryFilesError {
    Archive(ArchiveError),
    Data(CtyError),
    /// `cty.csv` does not describe the same entities as `cty.dat`.
    Inconsistent(EntityCodeReport),
    Install(InstallError),
}

impl fmt::Display for CountryFilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(e) => write!(f, "{e}"),
            Self::Data(e) => write!(f, "{e}"),
            Self::Inconsistent(r) => write!(
                f,
                "cty.csv does not match cty.dat ({} missing, {} unknown)",
                r.missing_in_csv.len(),
                r.unknown_in_csv.len()
            ),
            Self::Install(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CountryFilesError {}

impl From<ArchiveError> for CountryFilesError {
    fn from(e: ArchiveError) -> Self {
        Self::Archive(e)
    }
}
impl From<CtyError> for CountryFilesError {
    fn from(e: CtyError) -> Self {
        Self::Data(e)
    }
}
impl From<InstallError> for CountryFilesError {
    fn from(e: InstallError) -> Self {
        Self::Install(e)
    }
}

/// What a validated pair contains; shown to the user before and after install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountryFileSummary {
    pub entities: usize,
    pub prefixes: usize,
    pub exact_calls: usize,
    pub version: Option<String>,
    pub date: Option<String>,
    pub entity_codes: Option<EntityCodeReport>,
    pub dat_issues: usize,
    pub csv_issues: usize,
}

/// Parses both files and checks that `cty.csv`, when present, covers exactly
/// the entities of `cty.dat`.
pub fn validate_payload(
    payload: &CountryFilesPayload,
) -> Result<CountryFileSummary, CountryFilesError> {
    let dat_text = String::from_utf8_lossy(&payload.dat);
    let mut data = CtyData::parse(&dat_text)?;
    let mut summary = CountryFileSummary {
        entities: data.entities().len(),
        prefixes: data.prefix_count(),
        exact_calls: data.exact_count(),
        version: None,
        date: None,
        entity_codes: None,
        dat_issues: data.issues.len(),
        csv_issues: 0,
    };
    if let Some((v, d)) = data.version_marker() {
        summary.version = Some(v);
        summary.date = Some(d);
    }
    if let Some(csv) = &payload.csv {
        let (rows, issues) = cty::parse_cty_csv(&String::from_utf8_lossy(csv));
        if rows.is_empty() {
            return Err(CountryFilesError::Data(CtyError::NoEntities));
        }
        let report = data.apply_entity_codes(&rows);
        if !report.missing_in_csv.is_empty() || !report.unknown_in_csv.is_empty() {
            return Err(CountryFilesError::Inconsistent(report));
        }
        summary.csv_issues = issues.len();
        summary.entity_codes = Some(report);
    }
    Ok(summary)
}

/// Validates and atomically installs a country-file pair under `root`.
/// Version and date are taken from the data when `origin` leaves them empty.
pub fn install_country_files(
    root: &Path,
    payload: &CountryFilesPayload,
    mut origin: SetOrigin,
) -> Result<(SetManifest, CountryFileSummary), CountryFilesError> {
    let summary = validate_payload(payload)?;
    if origin.source_version.is_empty()
        && let Some(v) = &summary.version
    {
        origin.source_version.clone_from(v);
    }
    if origin.source_date.is_empty()
        && let Some(d) = &summary.date
    {
        origin.source_date.clone_from(d);
    }
    let mut files = vec![(DAT_FILE.to_string(), payload.dat.clone())];
    if let Some(csv) = &payload.csv {
        files.push((CSV_FILE.to_string(), csv.clone()));
    }
    let expected = summary.clone();
    let manifest = install::install_set(root, SET_NAME, &files, origin, |staged: &Path| {
        // Consistency check on exactly the bytes that were written.
        let staged_payload = CountryFilesPayload {
            dat: std::fs::read(staged.join(DAT_FILE))
                .map_err(|e| CountryFilesError::Install(InstallError::Io(e)))?,
            csv: match std::fs::read(staged.join(CSV_FILE)) {
                Ok(b) => Some(b),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(CountryFilesError::Install(InstallError::Io(e))),
            },
        };
        match validate_payload(&staged_payload) {
            Ok(s) if s == expected => Ok(()),
            Ok(_) => Err(CountryFilesError::Install(InstallError::Rejected(
                "staged files differ".into(),
            ))),
            Err(e) => Err(e),
        }
    })?;
    Ok((manifest, summary))
}

/// Loads the installed country files into a [`PrefixMatcher`].
/// `Ok(None)` means DATA_NOT_INSTALLED.
pub fn load_installed(
    root: &Path,
) -> Result<Option<(PrefixMatcher, CountryFileReport, SetManifest)>, CountryFilesError> {
    install::recover_set(root, SET_NAME)?;
    let Some((dir, manifest)) = install::current_set(root, SET_NAME)? else {
        return Ok(None);
    };
    let csv = dir.join(CSV_FILE);
    let csv = csv.exists().then_some(csv);
    let (matcher, report) =
        PrefixMatcher::load_from_country_files(&dir.join(DAT_FILE), csv.as_deref())?;
    Ok(Some((matcher, report, manifest)))
}

/// Restores the previously installed country files.
pub fn rollback(root: &Path) -> Result<SetManifest, CountryFilesError> {
    Ok(install::rollback_set(root, SET_NAME)?)
}

#[cfg(test)]
mod tests;
