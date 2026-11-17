// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Import, load and export of the user's rule files (regional rules and
//! explicit overrides) stored in the user data directory.
//!
//! Each kind lives in its own installed set, so updating country files never
//! touches user rules and vice versa.

use super::install::{self, InstallError, SetManifest, SetOrigin};
use super::rules::{self, RuleBook, RuleIssue};
use std::fmt;
use std::path::Path;

pub const RULES_FILE: &str = "rules.csv";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesKind {
    /// Regional refinements (subdivisions, regional zones and coordinates).
    Regional,
    /// Explicit user overrides, highest priority.
    Overrides,
}

impl RulesKind {
    pub fn set_name(self) -> &'static str {
        match self {
            Self::Regional => "regional-rules",
            Self::Overrides => "user-overrides",
        }
    }
}

#[derive(Debug)]
pub enum RulesImportError {
    Io(std::io::Error),
    /// The file was rejected; installed rules are unchanged.
    Invalid(Vec<RuleIssue>),
    Install(InstallError),
}

impl fmt::Display for RulesImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid(issues) => write!(f, "{} invalid row(s)", issues.len()),
            Self::Install(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RulesImportError {}

impl From<std::io::Error> for RulesImportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<InstallError> for RulesImportError {
    fn from(e: InstallError) -> Self {
        Self::Install(e)
    }
}

/// Summary shown to the user after a successful import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesImportReport {
    pub rules: usize,
    pub warnings: Vec<RuleIssue>,
    pub manifest: SetManifest,
}

fn file_origin(source: &Path) -> SetOrigin {
    // Only the file name is recorded: the full path may reveal private details.
    SetOrigin {
        source: source
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        source_version: String::new(),
        source_date: String::new(),
    }
}

fn parse_file(path: &Path) -> Result<rules::RulesParse, RulesImportError> {
    let text = match rules::read_rules_file(path)? {
        Ok(t) => t,
        Err(kind) => return Err(RulesImportError::Invalid(vec![RuleIssue { line: 1, kind }])),
    };
    Ok(rules::parse_rules(&text))
}

/// Validates `source` as a whole and installs it as the `kind` rule set.
/// On any error the installed rules are left as they were.
pub fn import_rules(
    root: &Path,
    kind: RulesKind,
    source: &Path,
) -> Result<RulesImportReport, RulesImportError> {
    let parsed = parse_file(source)?;
    if !parsed.is_valid() {
        return Err(RulesImportError::Invalid(
            parsed.errors().cloned().collect(),
        ));
    }
    let bytes = std::fs::read(source)?;
    let rule_count = parsed.rules.len();
    let manifest = install::install_set(
        root,
        kind.set_name(),
        &[(RULES_FILE.to_string(), bytes)],
        file_origin(source),
        |staged: &Path| -> Result<(), &'static str> {
            // Re-validate exactly the bytes that were written.
            match parse_file(&staged.join(RULES_FILE)) {
                Ok(p) if p.is_valid() && p.rules.len() == rule_count => Ok(()),
                Ok(_) | Err(_) => Err("staged rules differ from the validated file"),
            }
        },
    )?;
    Ok(RulesImportReport {
        rules: rule_count,
        warnings: parsed
            .issues
            .into_iter()
            .filter(|i| i.kind.is_warning())
            .collect(),
        manifest,
    })
}

/// Loads the installed `kind` rules. `Ok(None)` means DATA_NOT_INSTALLED.
pub fn load_rules(
    root: &Path,
    kind: RulesKind,
) -> Result<Option<(RuleBook, SetManifest)>, RulesImportError> {
    install::recover_set(root, kind.set_name())?;
    let Some((dir, manifest)) = install::current_set(root, kind.set_name())? else {
        return Ok(None);
    };
    let parsed = parse_file(&dir.join(RULES_FILE))?;
    if !parsed.is_valid() {
        return Err(RulesImportError::Invalid(
            parsed.errors().cloned().collect(),
        ));
    }
    Ok(Some((RuleBook::from_rules(parsed.rules), manifest)))
}

/// Copies the installed `kind` rules to `dest` (export / backup).
pub fn export_rules(root: &Path, kind: RulesKind, dest: &Path) -> Result<u64, RulesImportError> {
    let Some((dir, _)) = install::current_set(root, kind.set_name())? else {
        return Err(RulesImportError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "rules not installed",
        )));
    };
    Ok(std::fs::copy(dir.join(RULES_FILE), dest)?)
}

/// Restores the previously installed `kind` rules.
pub fn rollback_rules(root: &Path, kind: RulesKind) -> Result<SetManifest, RulesImportError> {
    Ok(install::rollback_set(root, kind.set_name())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "#splogbook-rules,1\nrule_type,callsign_or_prefix,cq_zone,notes\nprefix,QA,3,first\nprefix,QB,4,\n";
    const VALID2: &str = "#splogbook-rules,1\nrule_type,callsign_or_prefix,cq_zone\nprefix,QC,5\n";
    const INVALID: &str =
        "#splogbook-rules,1\nrule_type,callsign_or_prefix,cq_zone\nprefix,QC,99\n";

    fn write(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn missing_rules_are_not_installed_not_an_error() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            load_rules(root.path(), RulesKind::Regional)
                .unwrap()
                .is_none()
        );
        assert!(
            load_rules(root.path(), RulesKind::Overrides)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn import_then_load_and_reload() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let report = import_rules(
            root.path(),
            RulesKind::Regional,
            &write(src.path(), "mine.csv", VALID),
        )
        .unwrap();
        assert_eq!(report.rules, 2);
        assert_eq!(report.manifest.origin.source, "mine.csv");
        let (book, _) = load_rules(root.path(), RulesKind::Regional)
            .unwrap()
            .unwrap();
        assert_eq!(book.len(), 2);
        // Kinds are independent.
        assert!(
            load_rules(root.path(), RulesKind::Overrides)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn invalid_file_keeps_previous_rules() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        import_rules(
            root.path(),
            RulesKind::Overrides,
            &write(src.path(), "a.csv", VALID),
        )
        .unwrap();
        let err = import_rules(
            root.path(),
            RulesKind::Overrides,
            &write(src.path(), "b.csv", INVALID),
        )
        .unwrap_err();
        match err {
            RulesImportError::Invalid(issues) => {
                assert_eq!(issues.len(), 1);
                assert_eq!(issues[0].line, 3);
            }
            other => panic!("{other:?}"),
        }
        let (book, m) = load_rules(root.path(), RulesKind::Overrides)
            .unwrap()
            .unwrap();
        assert_eq!(book.len(), 2);
        assert_eq!(m.origin.source, "a.csv");
        // Not UTF-8 is rejected the same way.
        let bad = src.path().join("c.csv");
        std::fs::write(&bad, b"#splogbook-rules,1\n\xff").unwrap();
        assert!(matches!(
            import_rules(root.path(), RulesKind::Overrides, &bad),
            Err(RulesImportError::Invalid(_))
        ));
    }

    #[test]
    fn export_and_rollback() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        import_rules(
            root.path(),
            RulesKind::Regional,
            &write(src.path(), "a.csv", VALID),
        )
        .unwrap();
        import_rules(
            root.path(),
            RulesKind::Regional,
            &write(src.path(), "b.csv", VALID2),
        )
        .unwrap();
        let out = src.path().join("backup.csv");
        export_rules(root.path(), RulesKind::Regional, &out).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), VALID2);
        let restored = rollback_rules(root.path(), RulesKind::Regional).unwrap();
        assert_eq!(restored.origin.source, "a.csv");
        let (book, _) = load_rules(root.path(), RulesKind::Regional)
            .unwrap()
            .unwrap();
        assert_eq!(book.len(), 2);
        assert!(export_rules(root.path(), RulesKind::Overrides, &out).is_err());
    }

    #[test]
    fn warnings_are_reported_but_do_not_block_import() {
        let root = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();
        let text = "#splogbook-rules,1\nrule_type,callsign_or_prefix,cq_zone,shade\nprefix,QA,3,x\nprefix,QA,3,x\n";
        let r = import_rules(
            root.path(),
            RulesKind::Regional,
            &write(src.path(), "w.csv", text),
        )
        .unwrap();
        assert_eq!(r.rules, 1);
        assert_eq!(r.warnings.len(), 2);
    }
}
