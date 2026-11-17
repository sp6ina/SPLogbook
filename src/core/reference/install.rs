// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Atomic installation of user reference-data sets.
//!
//! A set is a small group of files (for example `cty.dat` + `cty.csv`) plus a
//! `manifest.json` describing where the data came from. Layout under the
//! reference-data root:
//!
//! ```text
//! <root>/<set>/            current, verified set
//! <root>/<set>.prev/       previous verified set (kept for rollback)
//! <root>/<set>.staging/    temporary, never read as data
//! ```
//!
//! Installation writes and verifies the staging directory first. Only then
//! the current set becomes `.prev` and staging becomes current. Every step
//! is a rename inside one directory, and [`recover_set`] completes or rolls
//! back an interrupted swap, so a failure never leaves the user without the
//! last valid data.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILE: &str = "manifest.json";
pub const MANIFEST_FORMAT: u32 = 1;

/// Where the data of a set came from. All fields are informational and are
/// shown to the user; none of them is trusted for security decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetOrigin {
    /// Human-readable source, e.g. a URL or "local file".
    pub source: String,
    /// Version string announced by the source, if any.
    #[serde(default)]
    pub source_version: String,
    /// Date of the data as announced by the source (`YYYY-MM-DD`), if any.
    #[serde(default)]
    pub source_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetManifest {
    pub format: u32,
    pub set: String,
    pub origin: SetOrigin,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug)]
pub enum InstallError {
    Io(std::io::Error),
    InvalidSetName(String),
    InvalidFileName(String),
    DuplicateFile(String),
    EmptySet,
    /// The caller's validation of the staged files failed; nothing changed.
    Rejected(String),
    ManifestMissing,
    ManifestInvalid(String),
    HashMismatch(String),
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::InvalidSetName(n) => write!(f, "invalid set name '{n}'"),
            Self::InvalidFileName(n) => write!(f, "invalid file name '{n}'"),
            Self::DuplicateFile(n) => write!(f, "duplicate file '{n}'"),
            Self::EmptySet => f.write_str("empty data set"),
            Self::Rejected(why) => write!(f, "validation failed: {why}"),
            Self::ManifestMissing => f.write_str("manifest missing"),
            Self::ManifestInvalid(e) => write!(f, "manifest invalid: {e}"),
            Self::HashMismatch(n) => write!(f, "checksum mismatch for '{n}'"),
        }
    }
}

impl std::error::Error for InstallError {}

impl From<std::io::Error> for InstallError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// What [`recover_set`] found and did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryOutcome {
    /// Nothing to do.
    Clean,
    /// A finished staging set replaced a missing current set.
    CompletedInstall,
    /// The previous set was restored because the current set was missing.
    RestoredPrevious,
    /// Leftover staging data was discarded.
    DiscardedStaging,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !name.starts_with('.')
        && !name.rsplit_once('.').is_some_and(|(_, ext)| {
            ext.eq_ignore_ascii_case("prev") || ext.eq_ignore_ascii_case("staging")
        })
}

fn sibling(root: &Path, set: &str, suffix: &str) -> PathBuf {
    root.join(format!("{set}{suffix}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::with_capacity(64), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    f.flush()?;
    f.sync_all()
}

fn remove_dir_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Reads and verifies the manifest and file hashes of a set directory.
pub fn verify_set_dir(dir: &Path) -> Result<SetManifest, InstallError> {
    let raw = match std::fs::read(dir.join(MANIFEST_FILE)) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(InstallError::ManifestMissing);
        }
        Err(e) => return Err(e.into()),
    };
    let manifest: SetManifest =
        serde_json::from_slice(&raw).map_err(|e| InstallError::ManifestInvalid(e.to_string()))?;
    if manifest.format != MANIFEST_FORMAT {
        return Err(InstallError::ManifestInvalid(format!(
            "unsupported format {}",
            manifest.format
        )));
    }
    for f in &manifest.files {
        if !valid_name(&f.name) || f.name == MANIFEST_FILE {
            return Err(InstallError::InvalidFileName(f.name.clone()));
        }
        let bytes = std::fs::read(dir.join(&f.name))?;
        if bytes.len() as u64 != f.size || sha256_hex(&bytes) != f.sha256 {
            return Err(InstallError::HashMismatch(f.name.clone()));
        }
    }
    Ok(manifest)
}

/// Installs `files` as set `set` under `root`.
///
/// `validate` receives the staging directory after every file has been
/// written and must return `Err(reason)` to reject the set; in that case the
/// current and previous sets are left untouched.
pub fn install_set<V, E>(
    root: &Path,
    set: &str,
    files: &[(String, Vec<u8>)],
    origin: SetOrigin,
    validate: V,
) -> Result<SetManifest, InstallError>
where
    V: FnOnce(&Path) -> Result<(), E>,
    E: fmt::Display,
{
    if !valid_name(set) {
        return Err(InstallError::InvalidSetName(set.to_string()));
    }
    if files.is_empty() {
        return Err(InstallError::EmptySet);
    }
    let mut seen = std::collections::HashSet::new();
    for (name, _) in files {
        if !valid_name(name) || name == MANIFEST_FILE {
            return Err(InstallError::InvalidFileName(name.clone()));
        }
        if !seen.insert(name.as_str()) {
            return Err(InstallError::DuplicateFile(name.clone()));
        }
    }

    std::fs::create_dir_all(root)?;
    // Finish or undo any earlier interrupted swap before touching anything.
    recover_set(root, set)?;

    let current = root.join(set);
    let prev = sibling(root, set, ".prev");
    let staging = sibling(root, set, ".staging");

    remove_dir_if_exists(&staging)?;
    std::fs::create_dir_all(&staging)?;

    let staged = (|| -> Result<SetManifest, InstallError> {
        let mut entries = Vec::with_capacity(files.len());
        for (name, bytes) in files {
            write_synced(&staging.join(name), bytes)?;
            entries.push(ManifestFile {
                name: name.clone(),
                size: bytes.len() as u64,
                sha256: sha256_hex(bytes),
            });
        }
        let manifest = SetManifest {
            format: MANIFEST_FORMAT,
            set: set.to_string(),
            origin,
            files: entries,
        };
        let json = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| InstallError::ManifestInvalid(e.to_string()))?;
        write_synced(&staging.join(MANIFEST_FILE), &json)?;
        verify_set_dir(&staging)?;
        validate(&staging).map_err(|e| InstallError::Rejected(e.to_string()))?;
        Ok(manifest)
    })();

    let manifest = match staged {
        Ok(m) => m,
        Err(e) => {
            let _ = remove_dir_if_exists(&staging);
            return Err(e);
        }
    };

    // Swap: current -> prev, staging -> current.
    if current.exists() {
        remove_dir_if_exists(&prev)?;
        std::fs::rename(&current, &prev)?;
    }
    if let Err(e) = std::fs::rename(&staging, &current) {
        // Put the previous set back so the user keeps working data.
        if prev.exists() && !current.exists() {
            let _ = std::fs::rename(&prev, &current);
        }
        return Err(e.into());
    }
    Ok(manifest)
}

/// Completes or rolls back an interrupted installation of `set`.
pub fn recover_set(root: &Path, set: &str) -> Result<RecoveryOutcome, InstallError> {
    if !valid_name(set) {
        return Err(InstallError::InvalidSetName(set.to_string()));
    }
    let current = root.join(set);
    let prev = sibling(root, set, ".prev");
    let staging = sibling(root, set, ".staging");

    if current.exists() {
        if staging.exists() {
            remove_dir_if_exists(&staging)?;
            return Ok(RecoveryOutcome::DiscardedStaging);
        }
        return Ok(RecoveryOutcome::Clean);
    }
    if staging.exists() && verify_set_dir(&staging).is_ok() {
        std::fs::rename(&staging, &current)?;
        return Ok(RecoveryOutcome::CompletedInstall);
    }
    if staging.exists() {
        remove_dir_if_exists(&staging)?;
    }
    if prev.exists() && verify_set_dir(&prev).is_ok() {
        std::fs::rename(&prev, &current)?;
        return Ok(RecoveryOutcome::RestoredPrevious);
    }
    Ok(RecoveryOutcome::Clean)
}

/// Directory of the current verified set, or `None` when the set is not
/// installed or fails verification (in which case nothing is loaded and the
/// caller reports DATA_NOT_INSTALLED or the verification error).
pub fn current_set(root: &Path, set: &str) -> Result<Option<(PathBuf, SetManifest)>, InstallError> {
    if !valid_name(set) {
        return Err(InstallError::InvalidSetName(set.to_string()));
    }
    let dir = root.join(set);
    if !dir.exists() {
        return Ok(None);
    }
    let manifest = verify_set_dir(&dir)?;
    Ok(Some((dir, manifest)))
}

/// Restores the previous set as current; the replaced set becomes `.prev`.
pub fn rollback_set(root: &Path, set: &str) -> Result<SetManifest, InstallError> {
    if !valid_name(set) {
        return Err(InstallError::InvalidSetName(set.to_string()));
    }
    let current = root.join(set);
    let prev = sibling(root, set, ".prev");
    let manifest = verify_set_dir(&prev)?;
    let swap = sibling(root, set, ".staging");
    remove_dir_if_exists(&swap)?;
    if current.exists() {
        std::fs::rename(&current, &swap)?;
    }
    std::fs::rename(&prev, &current)?;
    if swap.exists() {
        std::fs::rename(&swap, &prev)?;
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(v: &str) -> SetOrigin {
        SetOrigin {
            source: "local file".into(),
            source_version: v.into(),
            source_date: "2026-11-01".into(),
        }
    }

    fn files(tag: &str) -> Vec<(String, Vec<u8>)> {
        vec![
            ("a.txt".into(), format!("A {tag}").into_bytes()),
            ("b.txt".into(), format!("B {tag}").into_bytes()),
        ]
    }

    fn ok(staging: &Path) -> Result<(), &'static str> {
        if staging.join(MANIFEST_FILE).is_file() {
            Ok(())
        } else {
            Err("manifest not staged")
        }
    }

    #[test]
    fn install_creates_verified_current_set() {
        let root = tempfile::tempdir().unwrap();
        let m = install_set(root.path(), "demo", &files("v1"), origin("v1"), ok).unwrap();
        assert_eq!(m.files.len(), 2);
        let (dir, read) = current_set(root.path(), "demo").unwrap().unwrap();
        assert_eq!(read, m);
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "A v1");
        assert!(!root.path().join("demo.staging").exists());
    }

    #[test]
    fn missing_set_is_not_installed_rather_than_an_error() {
        let root = tempfile::tempdir().unwrap();
        assert!(current_set(root.path(), "demo").unwrap().is_none());
    }

    #[test]
    fn second_install_keeps_previous_and_rollback_swaps() {
        let root = tempfile::tempdir().unwrap();
        install_set(root.path(), "demo", &files("v1"), origin("v1"), ok).unwrap();
        install_set(root.path(), "demo", &files("v2"), origin("v2"), ok).unwrap();
        let (dir, m) = current_set(root.path(), "demo").unwrap().unwrap();
        assert_eq!(m.origin.source_version, "v2");
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "A v2");
        let prev = verify_set_dir(&root.path().join("demo.prev")).unwrap();
        assert_eq!(prev.origin.source_version, "v1");

        let restored = rollback_set(root.path(), "demo").unwrap();
        assert_eq!(restored.origin.source_version, "v1");
        let (_, now) = current_set(root.path(), "demo").unwrap().unwrap();
        assert_eq!(now.origin.source_version, "v1");
        let prev = verify_set_dir(&root.path().join("demo.prev")).unwrap();
        assert_eq!(prev.origin.source_version, "v2");
    }

    #[test]
    fn rejected_validation_leaves_current_set_untouched() {
        let root = tempfile::tempdir().unwrap();
        install_set(root.path(), "demo", &files("v1"), origin("v1"), ok).unwrap();
        let err = install_set(root.path(), "demo", &files("bad"), origin("bad"), |_| {
            Err::<(), _>("incomplete data")
        })
        .unwrap_err();
        assert!(matches!(err, InstallError::Rejected(_)));
        let (_, m) = current_set(root.path(), "demo").unwrap().unwrap();
        assert_eq!(m.origin.source_version, "v1");
        assert!(!root.path().join("demo.staging").exists());
        assert!(!root.path().join("demo.prev").exists());
    }

    #[test]
    fn invalid_names_and_duplicates_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        for bad in ["", "../x", "a/b", ".hidden", "x.prev", "x.staging", "a\\b"] {
            assert!(matches!(
                install_set(root.path(), bad, &files("v"), origin("v"), ok),
                Err(InstallError::InvalidSetName(_))
            ));
        }
        let traversal = vec![("../evil".to_string(), b"x".to_vec())];
        assert!(matches!(
            install_set(root.path(), "demo", &traversal, origin("v"), ok),
            Err(InstallError::InvalidFileName(_))
        ));
        let manifest_name = vec![(MANIFEST_FILE.to_string(), b"{}".to_vec())];
        assert!(matches!(
            install_set(root.path(), "demo", &manifest_name, origin("v"), ok),
            Err(InstallError::InvalidFileName(_))
        ));
        let dup = vec![
            ("a.txt".to_string(), b"1".to_vec()),
            ("a.txt".to_string(), b"2".to_vec()),
        ];
        assert!(matches!(
            install_set(root.path(), "demo", &dup, origin("v"), ok),
            Err(InstallError::DuplicateFile(_))
        ));
        assert!(matches!(
            install_set(root.path(), "demo", &[], origin("v"), ok),
            Err(InstallError::EmptySet)
        ));
        assert!(!root.path().join("demo").exists());
    }

    #[test]
    fn tampered_set_fails_verification() {
        let root = tempfile::tempdir().unwrap();
        install_set(root.path(), "demo", &files("v1"), origin("v1"), ok).unwrap();
        std::fs::write(root.path().join("demo").join("a.txt"), "changed").unwrap();
        assert!(matches!(
            current_set(root.path(), "demo"),
            Err(InstallError::HashMismatch(_))
        ));
    }

    #[test]
    fn recovery_completes_interrupted_swap() {
        let root = tempfile::tempdir().unwrap();
        install_set(root.path(), "demo", &files("v1"), origin("v1"), ok).unwrap();
        install_set(root.path(), "demo", &files("v2"), origin("v2"), ok).unwrap();
        // Simulate a crash after "current -> prev" but before "staging -> current".
        let current = root.path().join("demo");
        let staging = root.path().join("demo.staging");
        std::fs::rename(&current, &staging).unwrap();
        assert_eq!(
            recover_set(root.path(), "demo").unwrap(),
            RecoveryOutcome::CompletedInstall
        );
        let (_, m) = current_set(root.path(), "demo").unwrap().unwrap();
        assert_eq!(m.origin.source_version, "v2");
    }

    #[test]
    fn recovery_restores_previous_when_staging_is_broken() {
        let root = tempfile::tempdir().unwrap();
        install_set(root.path(), "demo", &files("v1"), origin("v1"), ok).unwrap();
        install_set(root.path(), "demo", &files("v2"), origin("v2"), ok).unwrap();
        let current = root.path().join("demo");
        let staging = root.path().join("demo.staging");
        std::fs::rename(&current, &staging).unwrap();
        std::fs::write(staging.join("a.txt"), "torn write").unwrap();
        assert_eq!(
            recover_set(root.path(), "demo").unwrap(),
            RecoveryOutcome::RestoredPrevious
        );
        let (_, m) = current_set(root.path(), "demo").unwrap().unwrap();
        assert_eq!(m.origin.source_version, "v1");
        assert!(!staging.exists());
    }

    #[test]
    fn recovery_discards_leftover_staging() {
        let root = tempfile::tempdir().unwrap();
        install_set(root.path(), "demo", &files("v1"), origin("v1"), ok).unwrap();
        std::fs::create_dir_all(root.path().join("demo.staging")).unwrap();
        assert_eq!(
            recover_set(root.path(), "demo").unwrap(),
            RecoveryOutcome::DiscardedStaging
        );
        assert_eq!(
            recover_set(root.path(), "demo").unwrap(),
            RecoveryOutcome::Clean
        );
    }
}
