// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Reference-data actions started by the user: country files (download,
//! import, rollback), regional rules and user overrides (import, export).
//! Every action keeps the installed data when it fails and reloads the
//! shared prefix matcher without a restart when it succeeds.

use super::SpLogApp;
use crate::core::i18n::{Language, Msg, key, tr};
use crate::core::reference::country_files::{self, CountryFilesError};
use crate::core::reference::install::SetOrigin;
use crate::core::reference::loader::{self, SourceStatus};
use crate::core::reference::tables::{self, TableImportError, TableKind};
use crate::core::reference::user_rules::{self, RulesImportError, RulesKind};
use std::path::PathBuf;

/// Translated one-line status of a source.
pub fn source_status_text(status: &SourceStatus, lang: Language) -> String {
    match status {
        SourceStatus::NotInstalled => tr(key::REF_STATUS_NOT_INSTALLED, lang).to_string(),
        SourceStatus::Failed(_) => tr(key::REF_STATUS_FAILED, lang).to_string(),
        SourceStatus::Loaded {
            version,
            date,
            items,
            ..
        } => Msg::new(key::REF_STATUS_INSTALLED)
            .arg("version", version)
            .arg("date", date)
            .arg("items", items.to_string())
            .text(lang),
    }
}

fn country_files_error_text(e: &CountryFilesError, lang: Language) -> String {
    let k = match e {
        CountryFilesError::Archive(_) => key::REF_CF_ERR_ARCHIVE,
        CountryFilesError::Data(_) | CountryFilesError::Inconsistent(_) => key::REF_CF_ERR_DATA,
        CountryFilesError::Install(_) => key::REF_CF_ERR_INSTALL,
    };
    tr(k, lang).to_string()
}

fn installed_text(
    manifest: &crate::core::reference::install::SetManifest,
    entities: usize,
    lang: Language,
) -> String {
    Msg::new(key::REF_CF_INSTALLED)
        .arg("version", &manifest.origin.source_version)
        .arg("date", &manifest.origin.source_date)
        .arg("entities", entities.to_string())
        .text(lang)
}

impl SpLogApp {
    /// Reference root and legacy compatibility database for this session.
    pub fn reference_paths(&self) -> (PathBuf, Option<PathBuf>) {
        loader::context().map_or_else(
            || (loader::reference_root_for(&self.paths.active_db_path), None),
            |ctx| (ctx.root.clone(), ctx.legacy_db.clone()),
        )
    }

    fn reload_reference_data(&self) {
        let (root, legacy) = self.reference_paths();
        let _ = loader::reload_into(&self.services.prefix_matcher, &root, legacy.as_deref());
    }

    /// Downloads the current Big CTY archive after the user confirmed the source.
    pub fn trigger_country_files_download(&mut self) {
        let lang = self.gui.appearance.language;
        let url = country_files::BIG_CTY_ARCHIVE_URL;
        let confirmed = rfd::MessageDialog::new()
            .set_title(tr(key::REF_CF_CONFIRM_TITLE, lang))
            .set_description(
                Msg::new(key::REF_CF_CONFIRM_BODY)
                    .arg("url", url)
                    .text(lang),
            )
            .set_level(rfd::MessageLevel::Info)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        if !matches!(confirmed, rfd::MessageDialogResult::Yes) {
            return;
        }
        let tx = self.online.sync_log_tx.clone();
        let _ = tx.send((tr(key::REF_CF_DOWNLOADING, lang).to_string(), false));
        let matcher = self.services.prefix_matcher.clone();
        let (root, legacy) = self.reference_paths();
        tokio::spawn(async move {
            let limit = country_files::ArchiveLimits::default().max_archive_bytes;
            let msg =
                match crate::cloud::updater::DatabaseUpdater::download_country_archive(url, limit)
                    .await
                {
                    Err(e) => {
                        log::warn!("country files download failed: {e}");
                        tr(key::REF_CF_ERR_DOWNLOAD, lang).to_string()
                    }
                    Ok(bytes) => {
                        let job = tokio::task::spawn_blocking(move || {
                            let origin = SetOrigin {
                                source: url.to_string(),
                                ..SetOrigin::default()
                            };
                            let result = country_files::install_from_archive(&root, &bytes, origin);
                            if result.is_ok() {
                                let _ = loader::reload_into(&matcher, &root, legacy.as_deref());
                            }
                            result
                        })
                        .await;
                        match job {
                            Ok(Ok((manifest, summary))) => {
                                installed_text(&manifest, summary.entities, lang)
                            }
                            Ok(Err(e)) => {
                                log::warn!("country files not installed: {e}");
                                country_files_error_text(&e, lang)
                            }
                            Err(_) => tr(key::REF_CF_ERR_INSTALL, lang).to_string(),
                        }
                    }
                };
            let _ = tx.send((msg, false));
        });
    }

    /// Imports country files from a ZIP archive or `cty.dat` + `cty.csv`.
    pub fn trigger_country_files_import(&mut self) {
        let lang = self.gui.appearance.language;
        let Some(path) = rfd::FileDialog::new()
            .add_filter(tr(key::REF_FILTER_COUNTRY_FILES, lang), &["zip", "dat"])
            .set_title(tr(key::REF_PICK_COUNTRY_FILES, lang))
            .pick_file()
        else {
            return;
        };
        let (root, _) = self.reference_paths();
        let msg = match country_files::install_from_user_file(&root, &path) {
            Ok((manifest, summary)) => {
                self.reload_reference_data();
                installed_text(&manifest, summary.entities, lang)
            }
            Err(e) => {
                log::warn!("country files not imported: {e}");
                country_files_error_text(&e, lang)
            }
        };
        let _ = self.online.sync_log_tx.send((msg, false));
    }

    /// Restores the previously installed country files.
    pub fn trigger_country_files_rollback(&mut self) {
        let lang = self.gui.appearance.language;
        let (root, _) = self.reference_paths();
        let msg = match country_files::rollback(&root) {
            Ok(manifest) => {
                self.reload_reference_data();
                Msg::new(key::REF_CF_ROLLED_BACK)
                    .arg("date", &manifest.origin.source_date)
                    .text(lang)
            }
            Err(e) => {
                log::info!("no country files rollback: {e}");
                tr(key::REF_CF_NO_PREVIOUS, lang).to_string()
            }
        };
        let _ = self.online.sync_log_tx.send((msg, false));
    }

    /// Imports a regional rules or user overrides file.
    pub fn trigger_rules_import(&mut self, kind: RulesKind) {
        let lang = self.gui.appearance.language;
        let Some(path) = rfd::FileDialog::new()
            .add_filter(tr(key::REF_FILTER_RULES, lang), &["csv"])
            .set_title(tr(key::REF_PICK_RULES, lang))
            .pick_file()
        else {
            return;
        };
        let (root, _) = self.reference_paths();
        let msg = match user_rules::import_rules(&root, kind, &path) {
            Ok(report) => {
                self.reload_reference_data();
                Msg::new(key::REF_RULES_IMPORTED)
                    .arg("count", report.rules.to_string())
                    .arg("warnings", report.warnings.len().to_string())
                    .text(lang)
            }
            Err(RulesImportError::Invalid(issues)) => Msg::new(key::REF_RULES_REJECTED)
                .arg("count", issues.len().to_string())
                .arg("line", issues.first().map_or(0, |i| i.line).to_string())
                .text(lang),
            Err(e) => {
                log::warn!("{} not imported: {e}", kind.set_name());
                tr(key::REF_RULES_FAILED, lang).to_string()
            }
        };
        let _ = self.online.sync_log_tx.send((msg, false));
    }

    /// Imports one user table (IOTA, states, QSL managers, callbook) and
    /// reloads the shared tables.
    pub fn trigger_table_import(&mut self, kind: TableKind) {
        let lang = self.gui.appearance.language;
        let Some(path) = rfd::FileDialog::new()
            .add_filter(tr(key::REF_FILTER_TABLE, lang), &["csv"])
            .set_title(tr(key::REF_PICK_TABLE, lang))
            .pick_file()
        else {
            return;
        };
        let (root, _) = self.reference_paths();
        let msg = match tables::import_table(&root, kind, &path) {
            Ok(report) => {
                let _ = loader::reload_user_tables(&root);
                Msg::new(key::REF_TABLE_IMPORTED)
                    .arg("count", report.rows.to_string())
                    .arg("warnings", report.warnings.len().to_string())
                    .text(lang)
            }
            Err(TableImportError::Invalid(issues)) => Msg::new(key::REF_TABLE_REJECTED)
                .arg("count", issues.len().to_string())
                .arg("line", issues.first().map_or(0, |i| i.line).to_string())
                .text(lang),
            Err(e) => {
                log::warn!("{} not imported: {e}", kind.set_name());
                tr(key::REF_TABLE_FAILED, lang).to_string()
            }
        };
        let _ = self.online.sync_log_tx.send((msg, false));
    }

    /// Exports one user table (backup).
    pub fn trigger_table_export(&mut self, kind: TableKind) {
        let lang = self.gui.appearance.language;
        let Some(dest) = rfd::FileDialog::new()
            .add_filter(tr(key::REF_FILTER_TABLE, lang), &["csv"])
            .set_title(tr(key::REF_SAVE_TABLE, lang))
            .set_file_name(format!("{}.csv", kind.set_name()))
            .save_file()
        else {
            return;
        };
        let (root, _) = self.reference_paths();
        let msg = match tables::export_table(&root, kind, &dest) {
            Ok(_) => tr(key::REF_TABLE_EXPORTED, lang).to_string(),
            Err(_) => tr(key::REF_TABLE_NOTHING_TO_EXPORT, lang).to_string(),
        };
        let _ = self.online.sync_log_tx.send((msg, false));
    }

    /// Exports the installed regional rules or user overrides (backup).
    pub fn trigger_rules_export(&mut self, kind: RulesKind) {
        let lang = self.gui.appearance.language;
        let Some(dest) = rfd::FileDialog::new()
            .add_filter(tr(key::REF_FILTER_RULES, lang), &["csv"])
            .set_title(tr(key::REF_SAVE_RULES, lang))
            .set_file_name(format!("{}.csv", kind.set_name()))
            .save_file()
        else {
            return;
        };
        let (root, _) = self.reference_paths();
        let msg = match user_rules::export_rules(&root, kind, &dest) {
            Ok(_) => tr(key::REF_RULES_EXPORTED, lang).to_string(),
            Err(_) => tr(key::REF_RULES_NOTHING_TO_EXPORT, lang).to_string(),
        };
        let _ = self.online.sync_log_tx.send((msg, false));
    }
}
