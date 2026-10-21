use super::SpLogApp;
use crate::core::backup::BackupManager;
use crate::core::i18n::{Msg, key, tr};
use crate::core::qso::QsoRecord;
use crate::gui::csv_export_dialog::{CsvExportRequest, CsvExportScope};
use std::fmt::Write as _;

impl SpLogApp {
    pub fn import_adif_content(&mut self, content: &str, source_name: &str) {
        let report = crate::core::adif::parse_adif_with_report(content);
        let parsed = report.qsos;
        let rejected = report.rejected;
        let errors = report.errors;

        let active_journal_id = self.active_journal.id.clone();
        let options = crate::core::ingest::ImportOptions {
            journal_id: active_journal_id,
            enrich: true,
        };

        match self.qso_ingestor.import_batch(parsed, &options) {
            Ok(import_report) => {
                self.reload_qsos();
                self.invalidate_cluster_badges();
                let inserted = import_report.inserted;
                let mut msg = Msg::new(key::IMPORT_SUCCESS_SOURCE)
                    .arg("count", inserted)
                    .arg("source", &source_name)
                    .text(self.current_language);
                let mut details: Vec<String> = Vec::new();
                if rejected > 0 {
                    details.push(
                        Msg::new(key::IMPORT_REJECTED_DAMAGED)
                            .arg("count", rejected)
                            .text(self.current_language),
                    );
                }
                if import_report.rejected_invalid > 0 {
                    details.push(
                        Msg::new(key::IMPORT_REJECTED_VALIDATION)
                            .arg("count", import_report.rejected_invalid)
                            .text(self.current_language),
                    );
                }
                if import_report.skipped_duplicates > 0 {
                    details.push(
                        Msg::new(key::IMPORT_SKIPPED_DUPLICATES)
                            .arg("count", import_report.skipped_duplicates)
                            .text(self.current_language),
                    );
                }
                if !details.is_empty() {
                    let _ = write!(msg, " ({})", details.join(", "));
                }
                if let Some(first_error) = errors.first() {
                    if rejected > 0 || !errors.is_empty() {
                        let _ = write!(msg, " | {first_error}");
                    }
                }
                if let Some(issue) = import_report.first_issue {
                    let reason = issue.msg().text(self.current_language);
                    let first_err_prefix =
                        tr(key::IMPORT_FIRST_VALIDATION_ERROR, self.current_language);
                    let _ = write!(msg, " | {first_err_prefix}: {reason}");
                }
                self.status_message = Some(msg);
                self.status_toast = Some((
                    format!("Zaimportowano {inserted} QSO"),
                    std::time::Instant::now(),
                ));
            }
            Err(e) => {
                let err_text = Msg::new(key::IMPORT_DB_SAVE_ERROR)
                    .arg("err", &e)
                    .text(self.current_language);
                self.status_message = Some(err_text.clone());
                self.report_error(err_text);
            }
        }
    }

    pub fn trigger_import_adif(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pliki ADIF (*.adi, *.adif)", &["adi", "adif"])
            .set_title("Wybierz plik ADIF do zaimportowania")
            .pick_file()
        {
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let content = String::from_utf8_lossy(&bytes);
                    self.import_adif_content(&content, &path.display().to_string());
                }
                Err(e) => {
                    self.status_message = Some(
                        Msg::new(key::FILE_READ_ERROR)
                            .arg("path", path.display().to_string())
                            .arg("err", &e.to_string())
                            .text(self.current_language),
                    );
                }
            }
        }
    }

    pub fn trigger_export_adif(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pliki ADIF (*.adi, *.adif)", &["adi", "adif"])
            .set_file_name("SPLogbook_export.adi")
            .set_title("Zapisz eksport bazy do pliku ADIF")
            .save_file()
        {
            let qsos = {
                let db = self
                    .log_db
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match db.get_recent_qsos(100_000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.status_message = Some(
                            Msg::new(key::EXPORT_ERR_READ_LOG)
                                .arg("err", &e.to_string())
                                .text(self.current_language),
                        );
                        return;
                    }
                }
            };
            let count = qsos.len();
            let adif_text =
                crate::core::adif::export_adif(&qsos, "SPLogbook", &self.my_station.callsign);
            match std::fs::write(&path, adif_text) {
                Ok(()) => {
                    self.status_message = Some(
                        Msg::new(key::EXPORT_SUCCESS)
                            .arg("count", count)
                            .arg("path", path.display().to_string())
                            .text(self.current_language),
                    );
                }
                Err(e) => {
                    self.status_message = Some(
                        Msg::new(key::FILE_WRITE_ERROR)
                            .arg("path", path.display().to_string())
                            .arg("err", &e.to_string())
                            .text(self.current_language),
                    );
                }
            }
        }
    }

    pub fn trigger_export_adx(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pliki ADX (*.adx, *.xml)", &["adx", "xml"])
            .set_file_name("SPLogbook_export.adx")
            .set_title("Zapisz eksport bazy do pliku ADX (XML)")
            .save_file()
        {
            let qsos = {
                let db = self
                    .log_db
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match db.get_recent_qsos(100_000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.status_message = Some(
                            Msg::new(key::EXPORT_ERR_READ_LOG)
                                .arg("err", &e.to_string())
                                .text(self.current_language),
                        );
                        return;
                    }
                }
            };
            let count = qsos.len();
            let adx_text = crate::core::adif::export_adx(&qsos);
            match std::fs::write(&path, adx_text) {
                Ok(()) => {
                    self.status_message = Some(
                        Msg::new(key::EXPORT_SUCCESS)
                            .arg("count", count)
                            .arg("path", path.display().to_string())
                            .text(self.current_language),
                    );
                }
                Err(e) => {
                    self.status_message = Some(
                        Msg::new(key::FILE_WRITE_ERROR)
                            .arg("path", path.display().to_string())
                            .arg("err", &e.to_string())
                            .text(self.current_language),
                    );
                }
            }
        }
    }

    pub fn perform_csv_export(&mut self, req: &CsvExportRequest) {
        let qsos: Vec<QsoRecord> = match req.scope {
            CsvExportScope::All => {
                let db = self
                    .log_db
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match db.get_recent_qsos(100_000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.status_message = Some(
                            Msg::new(key::EXPORT_ERR_READ_LOG)
                                .arg("err", &e.to_string())
                                .text(self.current_language),
                        );
                        return;
                    }
                }
            }
            CsvExportScope::Filtered => self.recent_qsos.clone(),
            CsvExportScope::Selected => self
                .recent_qsos
                .iter()
                .filter(|q| q.id.is_some_and(|id| self.selected_qso_ids.contains(&id)))
                .cloned()
                .collect(),
        };

        if qsos.is_empty() {
            self.status_message =
                Some(tr(key::EXPORT_NO_QSOS_IN_RANGE, self.current_language).to_string());
            return;
        }

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pliki CSV (*.csv, *.tsv)", &["csv", "tsv"])
            .add_filter("Wszystkie pliki (*.*)", &["*"])
            .set_file_name("SPLogbook_export.csv")
            .set_title("Zapisz eksport dziennika do pliku CSV")
            .save_file()
        {
            let csv_text = crate::core::csv_export::export_csv(
                &qsos,
                &req.columns,
                req.delimiter,
                req.include_header,
            );
            match std::fs::write(&path, csv_text) {
                Ok(()) => {
                    self.status_message = Some(
                        Msg::new(key::EXPORT_SUCCESS)
                            .arg("count", qsos.len())
                            .arg("path", path.display().to_string())
                            .text(self.current_language),
                    );
                }
                Err(e) => {
                    self.status_message = Some(
                        Msg::new(key::FILE_WRITE_ERROR)
                            .arg("path", path.display().to_string())
                            .arg("err", &e.to_string())
                            .text(self.current_language),
                    );
                }
            }
        }
    }

    pub fn run_manual_backup(&mut self) {
        let backup_dir = self.active_db_path.parent().map_or_else(
            || std::path::PathBuf::from("backups"),
            |p| p.join("backups"),
        );
        let _ = std::fs::create_dir_all(&backup_dir);
        let backup_res = BackupManager::backup_database(&self.active_db_path, &backup_dir);

        let qsos = {
            let db = self
                .log_db
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            db.get_recent_qsos(10000).unwrap_or_default()
        };
        let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &self.my_station.callsign);
        let _ = BackupManager::backup_adif(&adif, &backup_dir);

        match backup_res {
            Ok(backed_path) => {
                let name = backed_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                self.say(Msg::new(key::BACKUP_CREATED_TOAST).arg("name", &name));
            }
            Err(e) => {
                self.fail(e.msg());
            }
        }
    }
}
