// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Builds the [`PrefixMatcher`] from every installed source and reports the
//! state of each one, so the GUI can show DATA_NOT_INSTALLED, the installed
//! version or a load problem without ever failing the application.

use super::country_files;
use super::install::SetManifest;
use super::user_rules::{self, RulesKind};
use crate::core::prefix::PrefixMatcher;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

/// Directory name of the user reference-data root inside the data directory.
pub const REFERENCE_DIR: &str = "reference";

/// Where reference data lives for this process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceContext {
    /// Root of the installed sets (country files, regional rules, overrides).
    pub root: PathBuf,
    /// Legacy reference database used as compatibility layer, if present.
    pub legacy_db: Option<PathBuf>,
}

static CONTEXT: OnceLock<ReferenceContext> = OnceLock::new();
static LAST_STATUS: Mutex<Option<ReferenceStatus>> = Mutex::new(None);

static USER_TABLES: OnceLock<Arc<RwLock<super::tables::UserTables>>> = OnceLock::new();
static TABLE_STATUS: Mutex<Vec<(super::tables::TableKind, SourceStatus)>> = Mutex::new(Vec::new());

/// Shared user tables (IOTA, states, QSL managers, callbook) for this process.
/// Empty until [`reload_user_tables`] loads installed data.
pub fn user_tables() -> Arc<RwLock<super::tables::UserTables>> {
    USER_TABLES.get_or_init(Arc::default).clone()
}

/// Loads the installed user tables from `root` into the shared instance and
/// returns the status of each table. A broken table is reported and left empty.
pub fn reload_user_tables(root: &Path) -> Vec<(super::tables::TableKind, SourceStatus)> {
    use super::tables::{TableKind, UserTables};
    let (tables, failures) = UserTables::load(root);
    let statuses: Vec<(TableKind, SourceStatus)> = TableKind::ALL
        .iter()
        .map(|&kind| {
            let status = if let Some((_, e)) = failures.iter().find(|(k, _)| *k == kind) {
                log::warn!("{} not loaded: {e}", kind.set_name());
                SourceStatus::failed(e)
            } else if let Some(m) = tables.manifests.get(&kind) {
                SourceStatus::loaded(m, tables.row_count(kind))
            } else {
                SourceStatus::NotInstalled
            };
            (kind, status)
        })
        .collect();
    *user_tables()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = tables;
    TABLE_STATUS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone_from(&statuses);
    statuses
}

/// Status of a user table from the most recent [`reload_user_tables`].
pub fn table_status(kind: super::tables::TableKind) -> SourceStatus {
    TABLE_STATUS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, s)| s.clone())
        .unwrap_or_default()
}

/// Status from the most recent [`build_prefix_matcher`] call in this process
/// (cheap to read; used for menu labels).
pub fn last_status() -> ReferenceStatus {
    LAST_STATUS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .unwrap_or_default()
}

/// Registers the context once at start-up. Later calls are ignored.
pub fn set_context(ctx: ReferenceContext) {
    let _ = CONTEXT.set(ctx);
}

/// The registered context, if any.
pub fn context() -> Option<&'static ReferenceContext> {
    CONTEXT.get()
}

/// Reference root next to a log database stored as
/// `<data dir>/databases/<log>.db` → `<data dir>/reference`.
pub fn reference_root_for(log_db_path: &Path) -> PathBuf {
    let databases_dir = log_db_path.parent().unwrap_or_else(|| Path::new("."));
    databases_dir
        .parent()
        .unwrap_or(databases_dir)
        .join(REFERENCE_DIR)
}

/// Kind of a load problem; the GUI maps it to a translated message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadFailure {
    /// Installed data failed verification or parsing; `detail` is technical.
    Invalid { detail: String },
}

/// State of one source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SourceStatus {
    /// DATA_NOT_INSTALLED: a valid, empty state.
    #[default]
    NotInstalled,
    Loaded {
        source: String,
        version: String,
        date: String,
        items: usize,
    },
    Failed(LoadFailure),
}

impl SourceStatus {
    fn loaded(manifest: &SetManifest, items: usize) -> Self {
        Self::Loaded {
            source: manifest.origin.source.clone(),
            version: manifest.origin.source_version.clone(),
            date: manifest.origin.source_date.clone(),
            items,
        }
    }

    fn failed(e: &impl std::fmt::Display) -> Self {
        Self::Failed(LoadFailure::Invalid {
            detail: e.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReferenceStatus {
    pub country_files: SourceStatus,
    pub regional_rules: SourceStatus,
    pub user_overrides: SourceStatus,
    /// Legacy database loaded as compatibility layer.
    pub legacy_rules: usize,
}

/// Loads every available source. Never panics and never fails as a whole:
/// a broken source is reported and skipped, the others are still used.
pub fn build_prefix_matcher(
    root: &Path,
    legacy_db: Option<&Path>,
) -> (PrefixMatcher, ReferenceStatus) {
    let mut status = ReferenceStatus::default();

    let legacy =
        legacy_db
            .filter(|p| p.exists())
            .and_then(|p| match PrefixMatcher::load_from_db(p) {
                Ok(m) => Some(m),
                Err(e) => {
                    log::warn!("legacy reference database not loaded: {e}");
                    None
                }
            });

    let mut matcher = match country_files::load_installed(root) {
        Ok(Some((m, report, manifest))) => {
            status.country_files = SourceStatus::loaded(&manifest, report.entities);
            match legacy {
                Some(l) => m.with_legacy_compatibility(l),
                None => m,
            }
        }
        Ok(None) => legacy.unwrap_or_else(PrefixMatcher::empty),
        Err(e) => {
            log::warn!("country files not loaded: {e}");
            status.country_files = SourceStatus::failed(&e);
            legacy.unwrap_or_else(PrefixMatcher::empty)
        }
    };

    for (kind, slot) in [
        (RulesKind::Regional, &mut status.regional_rules),
        (RulesKind::Overrides, &mut status.user_overrides),
    ] {
        match user_rules::load_rules(root, kind) {
            Ok(Some((book, manifest))) => {
                *slot = SourceStatus::loaded(&manifest, book.len());
                matcher = match kind {
                    RulesKind::Regional => matcher.with_regional_rules(book),
                    RulesKind::Overrides => matcher.with_overrides(book),
                };
            }
            Ok(None) => {}
            Err(e) => {
                log::warn!("{} not loaded: {e}", kind.set_name());
                *slot = SourceStatus::failed(&e);
            }
        }
    }

    status.legacy_rules = matcher.layers().legacy_rules;
    *LAST_STATUS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(status.clone());
    (matcher, status)
}

/// Rebuilds the matcher from disk and swaps its rules into `shared`.
pub fn reload_into(
    shared: &PrefixMatcher,
    root: &Path,
    legacy_db: Option<&Path>,
) -> ReferenceStatus {
    let (fresh, status) = build_prefix_matcher(root, legacy_db);
    shared.replace_rules_from(fresh);
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::reference::country_files::{CountryFilesPayload, install_country_files};
    use crate::core::reference::install::SetOrigin;

    const DAT: &str = "Alpha Land: 14: 27: EU: 50.0: -20.0: -1.0: QA:\n    QA,=VER20261101;\n";
    const CSV: &str = "QA,Alpha Land,901,EU,14,27,50.0,-20.0,-1.0,QA =VER20261101;\n";

    #[test]
    fn root_is_next_to_the_databases_directory() {
        assert_eq!(
            reference_root_for(Path::new("/data/splogbook/databases/default_log.db")),
            PathBuf::from("/data/splogbook/reference")
        );
    }

    #[test]
    fn nothing_installed_gives_an_empty_valid_matcher() {
        let root = tempfile::tempdir().unwrap();
        let (m, status) = build_prefix_matcher(root.path(), None);
        assert!(!m.has_data());
        assert_eq!(status, ReferenceStatus::default());
        assert_eq!(m.lookup("QA1AB"), None);
        // A missing legacy file is not an error either.
        let (m, _) = build_prefix_matcher(root.path(), Some(&root.path().join("missing.db")));
        assert!(!m.has_data());
    }

    #[test]
    fn installed_sources_are_loaded_and_reloaded_without_restart() {
        let root = tempfile::tempdir().unwrap();
        let (shared, _) = build_prefix_matcher(root.path(), None);
        assert_eq!(shared.lookup("QA1AB"), None);

        install_country_files(
            root.path(),
            &CountryFilesPayload {
                dat: DAT.into(),
                csv: Some(CSV.into()),
            },
            SetOrigin {
                source: "test".into(),
                ..SetOrigin::default()
            },
        )
        .unwrap();
        let rules = root.path().join("r.csv");
        std::fs::write(
            &rules,
            "#splogbook-rules,1\nrule_type,callsign_or_prefix,cq_zone\nprefix,QA9,33\n",
        )
        .unwrap();
        user_rules::import_rules(root.path(), RulesKind::Regional, &rules).unwrap();

        let status = reload_into(&shared, root.path(), None);
        assert_eq!(
            status.country_files,
            SourceStatus::Loaded {
                source: "test".into(),
                version: "VER20261101".into(),
                date: "2026-11-01".into(),
                items: 1
            }
        );
        assert!(matches!(
            status.regional_rules,
            SourceStatus::Loaded { items: 1, .. }
        ));
        assert_eq!(status.user_overrides, SourceStatus::NotInstalled);
        assert_eq!(shared.lookup("QA1AB").unwrap().dxcc, 901);
        assert_eq!(shared.lookup("QA9AB").unwrap().cqz, 33);
    }

    #[test]
    fn a_broken_source_is_reported_and_the_others_still_load() {
        let root = tempfile::tempdir().unwrap();
        install_country_files(
            root.path(),
            &CountryFilesPayload {
                dat: DAT.into(),
                csv: None,
            },
            SetOrigin::default(),
        )
        .unwrap();
        let rules = root.path().join("r.csv");
        std::fs::write(
            &rules,
            "#splogbook-rules,1\nrule_type,callsign_or_prefix,cq_zone\nprefix,QA,3\n",
        )
        .unwrap();
        user_rules::import_rules(root.path(), RulesKind::Overrides, &rules).unwrap();
        // Corrupt the installed country files after installation.
        std::fs::write(
            root.path().join("country-files").join("cty.dat"),
            "tampered",
        )
        .unwrap();

        let (m, status) = build_prefix_matcher(root.path(), None);
        assert!(matches!(status.country_files, SourceStatus::Failed(_)));
        assert!(matches!(status.user_overrides, SourceStatus::Loaded { .. }));
        // The override still resolves zones but has no entity to attach to.
        assert_eq!(m.lookup("QA1AB"), None);
    }
}
