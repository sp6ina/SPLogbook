// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Kopie zapasowe w trakcie pracy programu (STAGE-V140-S5): okresowe, ręczne, przy
//! zamknięciu oraz kontrola bazy po nieoczekiwanym zakończeniu poprzedniej sesji.

use super::SpLogApp;
use crate::core::backup::{BackupError, BackupManager, DbFingerprint, SessionMarker};
use crate::core::i18n::{Msg, key};
use crate::gui::state::ToastSeverity;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Katalog kopii zapasowych: `backups/` obok pliku bazy.
pub fn backup_dir_for(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

impl SpLogApp {
    /// Zakłada znacznik sesji. Jeśli poprzednia sesja go nie usunęła, w tle sprawdza
    /// spójność bazy i zapisuje jej kopię, a wynik pokazuje operatorowi.
    pub fn start_session_guard(&mut self) {
        let db_path = self.paths.active_db_path.clone();
        let left_open = match SessionMarker::acquire(&db_path) {
            Ok((marker, left_open)) => {
                self.runtime.backup.session_marker = Some(marker);
                left_open
            }
            Err(e) => {
                log::warn!("session marker for {}: {e}", db_path.display());
                return;
            }
        };
        if !left_open || !db_path.exists() {
            return;
        }
        let notices = self.runtime.notice_sender();
        std::thread::spawn(move || {
            let notice = match BackupManager::backup_after_unclean_shutdown(
                &db_path,
                &backup_dir_for(&db_path),
            ) {
                Ok(copy) => (
                    ToastSeverity::Warning,
                    Msg::new(key::BACKUP_UNCLEAN_SHUTDOWN_OK).arg("name", file_name(&copy)),
                ),
                Err(e) => (
                    ToastSeverity::Error,
                    Msg::new(key::BACKUP_UNCLEAN_SHUTDOWN_FAILED).arg("err", e),
                ),
            };
            notices.send(notice.0, notice.1);
        });
    }

    /// Uruchamia kopię okresową, gdy minął ustawiony odstęp, poprzednia kopia się
    /// zakończyła, a baza zmieniła się od ostatniej kopii. Kopia działa w osobnym wątku
    /// na własnym połączeniu SQLite i nie blokuje GUI ani zapisu QSO.
    pub fn run_backup_schedule(&mut self, now: Instant) {
        let Some(interval) = self.config.backup.interval() else {
            return;
        };
        let state = &mut self.runtime.backup;
        if now.saturating_duration_since(state.last_periodic) < interval
            || state.in_flight.load(Ordering::Acquire)
        {
            return;
        }
        state.last_periodic = now;

        let db_path = self.paths.active_db_path.clone();
        let Some(fingerprint) = DbFingerprint::of(&db_path) else {
            return;
        };
        if state.last_fingerprint.as_ref() == Some(&fingerprint) {
            return;
        }
        state.in_flight.store(true, Ordering::Release);

        let in_flight = state.in_flight.clone();
        let done = state.done_tx.clone();
        let keep = self.config.backup.keep_count();
        let notices = self.runtime.notice_sender();
        std::thread::spawn(move || {
            match BackupManager::backup_database_keeping(&db_path, &backup_dir_for(&db_path), keep)
            {
                Ok(copy) => {
                    log::info!("periodic backup: {}", copy.display());
                    let _ = done.send(fingerprint);
                }
                Err(e) => notices.send(
                    ToastSeverity::Error,
                    Msg::new(key::BACKUP_PERIODIC_FAILED).arg("err", e),
                ),
            }
            in_flight.store(false, Ordering::Release);
        });
    }

    /// Odbiera odciski baz, z których powstała udana kopia okresowa.
    pub fn poll_backup_results(&mut self) {
        while let Ok(fingerprint) = self.runtime.backup.done_rx.try_recv() {
            self.runtime.backup.last_fingerprint = Some(fingerprint);
        }
    }

    /// Kopia ADIF aktywnego dziennika. Błąd odczytu bazy jest zwracany, a nie
    /// zamieniany na pustą kopię.
    fn write_adif_backup(&self, backup_dir: &Path) -> Result<PathBuf, BackupError> {
        let qsos = {
            let db = self
                .services
                .log_db
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            db.get_all_qsos_for_journal_with_extras(&self.active_journal.id)
                .map_err(|e| BackupError::WriteAdif(e.to_string()))?
        };
        let adif =
            crate::core::adif::export_adif(&qsos, "SPLogbook", &self.config.station.callsign);
        BackupManager::backup_adif_keeping(&adif, backup_dir, self.config.backup.keep_count())
    }

    /// Kopia ręczna (menu): baza i — zgodnie z ustawieniami — ADIF. Każdy błąd jest
    /// pokazywany operatorowi.
    pub fn run_manual_backup(&mut self) {
        let backup_dir = backup_dir_for(&self.paths.active_db_path);
        match BackupManager::backup_database_keeping(
            &self.paths.active_db_path,
            &backup_dir,
            self.config.backup.keep_count(),
        ) {
            Ok(copy) => self.say(Msg::new(key::BACKUP_CREATED_TOAST).arg("name", file_name(&copy))),
            Err(e) => self.fail(e.msg()),
        }
        if self.config.backup.with_adif
            && let Err(e) = self.write_adif_backup(&backup_dir)
        {
            self.fail(Msg::new(key::BACKUP_ADIF_FAILED).arg("err", e));
        }
    }

    /// Kopia przy zamknięciu programu i zdjęcie znacznika sesji. GUI już nie istnieje,
    /// więc błędy trafiają do logu.
    pub fn backup_on_exit(&mut self) {
        if self.config.backup.on_exit {
            let backup_dir = backup_dir_for(&self.paths.active_db_path);
            if let Err(e) = BackupManager::backup_database_keeping(
                &self.paths.active_db_path,
                &backup_dir,
                self.config.backup.keep_count(),
            ) {
                log::error!("backup on exit: {e}");
            }
            if self.config.backup.with_adif
                && let Err(e) = self.write_adif_backup(&backup_dir)
            {
                log::error!("ADIF backup on exit: {e}");
            }
        }
        if let Some(marker) = self.runtime.backup.session_marker.take()
            && let Err(e) = marker.release()
        {
            log::warn!("session marker: {e}");
        }
    }

    /// Menu Plik: wybór kopii, potwierdzenie i przywrócenie bazy.
    pub fn restore_from_backup_dialog(&mut self) {
        let lang = self.gui.appearance.language;
        let Some(backup) = rfd::FileDialog::new()
            .set_directory(backup_dir_for(&self.paths.active_db_path))
            .add_filter("SQLite", &["db"])
            .pick_file()
        else {
            return;
        };
        let confirmed = rfd::MessageDialog::new()
            .set_title(crate::core::i18n::tr(
                key::BACKUP_RESTORE_CONFIRM_TITLE,
                lang,
            ))
            .set_description(
                Msg::new(key::BACKUP_RESTORE_CONFIRM_BODY)
                    .arg("name", file_name(&backup))
                    .text(lang),
            )
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        if matches!(confirmed, rfd::MessageDialogResult::Yes) {
            self.restore_from_backup(&backup);
        }
    }

    /// Przywraca bazę z kopii `backup`. Najpierw zapisuje kopię bieżącej bazy, potem
    /// zamyka połączenie, podmienia plik (z walidacją i wycofaniem w razie błędu),
    /// otwiera bazę ponownie i przeładowuje aktywny dziennik, dyplomy i SCP.
    pub fn restore_from_backup(&mut self, backup: &Path) {
        if self.runtime.backup.in_flight.load(Ordering::Acquire) {
            self.warn(Msg::new(key::BACKUP_RESTORE_BUSY));
            return;
        }
        let db_path = self.paths.active_db_path.clone();
        let safety = match BackupManager::backup_database_keeping(
            &db_path,
            &backup_dir_for(&db_path),
            self.config.backup.keep_count(),
        ) {
            Ok(copy) => copy,
            Err(e) => {
                self.fail(Msg::new(key::BACKUP_RESTORE_FAILED).arg("err", e));
                return;
            }
        };

        let result = {
            let mut db = self
                .services
                .log_db
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match crate::core::database::LogDatabase::open_in_memory() {
                Ok(placeholder) => {
                    drop(std::mem::replace(&mut *db, placeholder));
                    let restored = BackupManager::restore_database(backup, &db_path);
                    match crate::core::database::LogDatabase::open(&db_path) {
                        Ok(reopened) => {
                            *db = reopened;
                            restored.map_err(|e| e.to_string())
                        }
                        Err(e) => Err(e.to_string()),
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        };

        match result {
            Ok(()) => {
                let journal = self
                    .services
                    .log_db
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get_active_journal();
                match journal {
                    Ok(journal) => self.active_journal = journal,
                    Err(e) => log::error!("active journal after restore: {e}"),
                }
                self.rebuild_awards_full();
                self.rebuild_scp();
                self.say(
                    Msg::new(key::BACKUP_RESTORED)
                        .arg("name", file_name(backup))
                        .arg("safety", file_name(&safety)),
                );
            }
            Err(e) => self.fail(Msg::new(key::BACKUP_RESTORE_FAILED).arg("err", e)),
        }
    }
}
