// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::*;

impl SpLogApp {
    pub fn save_qso(&mut self) {
        if self.entry_callsign.trim().is_empty() {
            return;
        }

        let now = chrono::Utc::now();
        let date_str = now.format("%Y%m%d").to_string();
        let time_str = now.format("%H%M").to_string();

        let mut qso = QsoRecord::new(&self.entry_callsign, &self.entry_band, &self.entry_mode);
        qso.journal_id = Some(self.active_journal.id.clone());
        if !self.my_station.gridsquare.trim().is_empty() {
            qso.my_gridsquare = Some(self.my_station.gridsquare.clone());
        }
        if self.rig_state.frequency_hz > 0 {
            qso.freq = Some(self.rig_state.frequency_hz as f64 / 1_000_000.0);
        }
        qso.qso_date.clone_from(&date_str);
        qso.time_on.clone_from(&time_str);
        qso.rst_sent = self.entry_rst_sent.clone();
        qso.rst_rcvd = self.entry_rst_rcvd.clone();
        qso.name = if self.entry_name.is_empty() {
            None
        } else {
            Some(self.entry_name.clone())
        };
        qso.qth = if self.entry_qth.is_empty() {
            None
        } else {
            Some(self.entry_qth.clone())
        };
        qso.gridsquare = if self.entry_grid.is_empty() {
            None
        } else {
            Some(self.entry_grid.clone())
        };
        qso.pga_ref = if self.entry_pga.is_empty() {
            None
        } else {
            Some(self.entry_pga.clone())
        };
        qso.comment = if self.entry_comment.is_empty() {
            None
        } else {
            Some(self.entry_comment.clone())
        };
        qso.iota = if self.entry_iota.is_empty() {
            None
        } else {
            Some(self.entry_iota.clone())
        };
        qso.state = if self.entry_state.is_empty() {
            None
        } else {
            Some(self.entry_state.clone())
        };
        qso.sota_ref = if self.entry_sota.is_empty() {
            None
        } else {
            Some(self.entry_sota.clone())
        };
        qso.pota_ref = if self.entry_pota.is_empty() {
            None
        } else {
            Some(self.entry_pota.clone())
        };
        qso.qsl_via = if self.entry_qsl_manager.is_empty() {
            None
        } else {
            Some(self.entry_qsl_manager.clone())
        };
        qso.qsl_manager = qso.qsl_via.clone();

        // Sprawdzenie czy nagrywano audio łączności (Audio Memo) — zapis obok aktywnej bazy danych
        if crate::media::audio_recorder::AudioRecorder::is_recording() {
            let rec_dir = self.paths.active_db_path.parent().map_or_else(
                || std::path::PathBuf::from("recordings"),
                |p| p.join("recordings"),
            );
            let clean_call = self.entry_callsign.trim().to_uppercase().replace('/', "_");
            let filename = format!("QSO_{clean_call}_{date_str}_{time_str}.wav");
            let path = rec_dir.join(filename);
            if let Ok(saved) = crate::media::audio_recorder::AudioRecorder::stop_and_save(&path) {
                qso.audio_file = Some(saved.to_string_lossy().to_string());
            }
        }

        let outcome = match self.services.ingestor.ingest(qso, QsoOrigin::Form) {
            Ok(outcome) => outcome,
            Err(e) => {
                let msg = match e {
                    IngestError::EmptyCallsign => {
                        Msg::new(key::QSO_ISSUE_EMPTY_CALLSIGN).text(self.gui.appearance.language)
                    }
                    IngestError::Invalid(issue) => Msg::new(key::MSG_QSO_NOT_SAVED)
                        .arg("reason", issue.msg().text(self.gui.appearance.language))
                        .text(self.gui.appearance.language),
                    IngestError::Duplicate => {
                        Msg::new(key::MSG_QSO_DUPLICATE).text(self.gui.appearance.language)
                    }
                    IngestError::Storage(err) => Msg::new(key::MSG_QSO_SAVE_FAILED)
                        .arg("error", err)
                        .text(self.gui.appearance.language),
                };
                self.gui.chrome.status_toast = Some((msg, std::time::Instant::now()));
                return;
            }
        };

        self.invalidate_cluster_badges();
        self.apply_post_effects(&outcome);

        let mut success_msg = Msg::new(key::MSG_QSO_SAVED)
            .arg("callsign", &outcome.qso.callsign)
            .arg("band", &outcome.qso.band)
            .arg("mode", &outcome.qso.mode)
            .text(self.gui.appearance.language);
        if !outcome.warnings.is_empty() {
            success_msg.push_str("\n⚠ ");
            let warning_texts: Vec<String> = outcome
                .warnings
                .iter()
                .map(|w| w.msg().text(self.gui.appearance.language))
                .collect();
            success_msg.push_str(&warning_texts.join("; "));
        }
        self.gui.chrome.status_toast = Some((success_msg, std::time::Instant::now()));

        self.clear_qso_form();
        self.reload_qsos();
    }

    /// Wykonuje centralne efekty uboczne po pomyślnym zapisie QSO w potoku `core::ingest` (PIPE-4B.3).
    ///
    /// Każdy efekt jest wykonywany tylko wtedy, gdy flaga w `outcome.effects` ma wartość `true`.
    pub fn apply_post_effects(&mut self, outcome: &IngestOutcome) {
        let qso = &outcome.qso;
        let effects = &outcome.effects;

        // 1. Dźwięki i alerty (alert_sound)
        if effects.alert_sound && self.solar.band_alert_enabled {
            if let Some(status) = &self.active_award_status {
                if status.is_new_dxcc {
                    crate::media::sounds::play_new_dxcc_alert();
                } else if status.is_new_iota {
                    crate::media::sounds::play_new_iota_alert();
                } else if status.is_worked_b4 {
                    crate::media::sounds::play_duplicate_alert();
                } else {
                    crate::media::sounds::play_qso_saved_alert();
                }
            } else {
                crate::media::sounds::play_qso_saved_alert();
            }
        }

        // 2. Weryfikacja prognozy propagacyjnej (propagation_observation)
        if effects.propagation_observation {
            self.solar.propagation_history.record_observation(
                &qso.band,
                crate::core::propagation_history::ObservedOutcome::ConfirmedOpen,
            );
        }

        // 3. Hook pluginów użytkownika (plugin_hook)
        if effects.plugin_hook {
            self.refresh_plugin_snapshot();
            self.services
                .plugin_engine
                .run_on_qso_logged(&crate::plugins::QsoHookContext {
                    callsign: qso.callsign.clone(),
                    band: qso.band.clone(),
                    mode: qso.mode.clone(),
                    freq_mhz: qso.freq.unwrap_or(0.0),
                    is_atno: self
                        .active_award_status
                        .as_ref()
                        .is_some_and(|s| s.is_new_dxcc),
                });
        }

        // 4. Kolejka uploadów chmurowych (enqueue_uploads)
        if effects.enqueue_uploads {
            // Automatyczny przesył na żywo do Club Log — przez wspólną kolejkę wysyłki
            if self.live_auto_upload_clublog
                && !self.online.clublog_callsign.is_empty()
                && !self.online.clublog_password.is_empty()
            {
                let adif_record = crate::core::adif::export_adif(
                    std::slice::from_ref(qso),
                    "SPLogbook",
                    &self.my_station.callsign,
                );
                let now = crate::cloud::scheduler::now_unix();
                let mut sched = self
                    .services
                    .upload_scheduler
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                sched.enqueue(
                    crate::cloud::scheduler::UploadService::ClubLog,
                    adif_record,
                    now,
                );
                sched.save_to_disk();
            }

            // Automatyczny przesył na żywo do logbooka QRZ.com — przez wspólną kolejkę wysyłki
            if self.live_auto_upload_qrz && !self.qrz_api_key.is_empty() {
                let adif_record = crate::core::adif::export_adif(
                    std::slice::from_ref(qso),
                    "SPLogbook",
                    &self.my_station.callsign,
                );
                let now = crate::cloud::scheduler::now_unix();
                let mut sched = self
                    .services
                    .upload_scheduler
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                sched.enqueue(
                    crate::cloud::scheduler::UploadService::Qrz,
                    adif_record,
                    now,
                );
                sched.save_to_disk();
            }
        }

        // 5. Automatyczne raportowanie do PSK Reporter (psk_report)
        if effects.psk_report
            && self.psk_reporter_enabled
            && !self.my_station.callsign.is_empty()
            && !self.my_station.gridsquare.is_empty()
        {
            let client = crate::cloud::psk_reporter::PskReporterClient::new(
                &self.my_station.callsign,
                &self.my_station.gridsquare,
            );
            let qso_clone = qso.clone();
            tokio::spawn(async move {
                let _ = client.submit_reception_report(&qso_clone).await;
            });
        }

        // 6. Rozgłaszanie w sieci lokalnej Multi-Op LAN (broadcast_lan)
        if effects.broadcast_lan {
            if let Some(ref srv) = self.sync_ui.service {
                srv.broadcast(qso);
            }
        }

        // 7. Emisja zdarzenia w formacie N1MM Logger+ UDP broadcast (broadcast_n1mm)
        if effects.broadcast_n1mm && self.n1mm_broadcast_enabled {
            let host = self.n1mm_broadcast_host.clone();
            let port = self.n1mm_broadcast_port;
            let my_call = self.my_station.callsign.clone();
            let xml = crate::digital::n1mm::contactinfo_xml(qso, &my_call, 1);
            tokio::spawn(async move {
                if let Err(e) = crate::digital::n1mm::send_broadcast(&host, port, &xml).await {
                    log::warn!("N1MM broadcast nieudany ({host}:{port}): {e}");
                }
            });
        }
    }

    /// Przetwarza i loguje QSO odebrane z WSJT-X (PIPE-4B.3).
    pub fn handle_wsjtx_qso(&mut self, mut qso: QsoRecord) {
        let now = chrono::Utc::now();
        if qso.qso_date.is_empty() {
            qso.qso_date = now.format("%Y%m%d").to_string();
        }
        if qso.time_on.is_empty() {
            qso.time_on = now.format("%H%M").to_string();
        }
        qso.journal_id = Some(self.active_journal.id.clone());

        match self.services.ingestor.ingest(qso, QsoOrigin::WsjtX) {
            Ok(outcome) => {
                self.invalidate_cluster_badges();
                self.apply_post_effects(&outcome);
                if let Some(ref issue) = outcome.advisory_issue {
                    log::warn!(
                        "WSJT-X QSO advisory issue: {}",
                        issue.msg().text(self.gui.appearance.language)
                    );
                }
                self.gui.chrome.status_toast = Some((
                    format!(
                        "WSJT-X: Automatycznie dodano QSO z {}!",
                        outcome.qso.callsign
                    ),
                    std::time::Instant::now(),
                ));
                self.reload_qsos();
            }
            Err(e) => {
                self.report_error(
                    Msg::new(key::WSJTX_AUTO_SAVE_ERROR)
                        .arg("e", &e)
                        .text(self.gui.appearance.language),
                );
            }
        }
    }

    /// Przetwarza i loguje QSO odebrane z JS8Call (PIPE-4B.3).
    pub fn handle_js8call_qso(&mut self, mut qso: QsoRecord) {
        qso.journal_id = Some(self.active_journal.id.clone());

        match self.services.ingestor.ingest(qso, QsoOrigin::Js8Call) {
            Ok(outcome) => {
                self.invalidate_cluster_badges();
                self.apply_post_effects(&outcome);
                if let Some(ref issue) = outcome.advisory_issue {
                    log::warn!(
                        "JS8Call QSO advisory issue: {}",
                        issue.msg().text(self.gui.appearance.language)
                    );
                }
                self.gui.chrome.status_toast = Some((
                    format!(
                        "JS8Call: Automatycznie dodano QSO z {}!",
                        outcome.qso.callsign
                    ),
                    std::time::Instant::now(),
                ));
                self.reload_qsos();
            }
            Err(e) => {
                self.gui.chrome.status_toast = Some((
                    Msg::new(key::JS8CALL_AUTO_SAVE_ERROR)
                        .arg("e", &e)
                        .text(self.gui.appearance.language),
                    std::time::Instant::now(),
                ));
            }
        }
    }

    /// Przetwarza i loguje QSO odebrane z sieci LAN Multi-Op (PIPE-4B.3, SYNC-6).
    pub fn handle_lan_qso(&mut self, qso: QsoRecord) {
        match self.services.ingestor.ingest(qso, QsoOrigin::LanPeer) {
            Ok(outcome) => {
                self.invalidate_cluster_badges();
                self.apply_post_effects(&outcome);
                if let Some(ref issue) = outcome.advisory_issue {
                    log::warn!(
                        "Multi-Op LAN QSO advisory issue: {}",
                        issue.msg().text(self.gui.appearance.language)
                    );
                }
                let log_entry = format!(
                    "Odebrano z LAN: {} ({} {})",
                    outcome.qso.callsign, outcome.qso.band, outcome.qso.mode
                );
                self.sync_ui.push_log(log_entry);
                self.gui.chrome.status_toast = Some((
                    format!("🌐 Multi-Op LAN: Dodano QSO z {}!", outcome.qso.callsign),
                    std::time::Instant::now(),
                ));
                self.reload_qsos();
            }
            Err(crate::core::ingest::IngestError::Duplicate) => {
                // Duplicate jest ignorowany bez ponownego zapisu
            }
            Err(e) => {
                log::warn!("Multi-Op LAN auto-save error: {e}");
            }
        }
    }

    /// Odpytuje odbiornik zdarzeń podsystemu synchronizacji Multi-Op i przetwarza je (SYNC-6).
    pub fn poll_sync_events(&mut self) {
        let mut incoming_qsos = Vec::new();
        let mut catchup_applied = 0;

        while let Ok(event) = self.sync_ui.events_rx.try_recv() {
            match event {
                crate::sync::SyncEvent::Status(status) => {
                    let peers = status.peers;
                    let state_str = match status.state {
                        crate::sync::SyncState::Starting => "Starting...".to_string(),
                        crate::sync::SyncState::Listening => {
                            Msg::new(key::MULTI_OP_STATUS_LISTENING)
                                .arg("peers", peers)
                                .text(self.gui.appearance.language)
                        }
                        crate::sync::SyncState::Connected => {
                            tr(key::MULTI_OP_STATUS_CONNECTED, self.gui.appearance.language)
                                .to_string()
                        }
                        crate::sync::SyncState::Reconnecting => tr(
                            key::MULTI_OP_STATUS_RECONNECTING,
                            self.gui.appearance.language,
                        )
                        .to_string(),
                        crate::sync::SyncState::Failed => {
                            tr(key::MULTI_OP_STATUS_FAILED, self.gui.appearance.language)
                                .to_string()
                        }
                    };
                    self.sync_ui.status = state_str;
                }
                crate::sync::SyncEvent::IncomingQso(qso) => {
                    incoming_qsos.push(*qso);
                }
                crate::sync::SyncEvent::CatchUpApplied { inserted } => {
                    if inserted > 0 {
                        catchup_applied += inserted;
                        let msg = Msg::new(key::MULTI_OP_BACKWARD_SYNC)
                            .arg("inserted", inserted)
                            .text(self.gui.appearance.language);
                        self.sync_ui.push_log(msg);
                    }
                }
                crate::sync::SyncEvent::Log(msg) => {
                    let text = msg.text(self.gui.appearance.language);
                    self.sync_ui.push_log(text);
                }
                crate::sync::SyncEvent::Error(err) => {
                    let err_msg = err.to_string();
                    self.sync_ui.push_log(format!("Error: {err_msg}"));
                    self.sync_ui.status = format!("Error: {err_msg}");
                    self.gui.chrome.status_toast =
                        Some((format!("⚠ Multi-Op: {err_msg}"), std::time::Instant::now()));
                }
            }
        }

        for qso in incoming_qsos {
            self.handle_lan_qso(qso);
        }

        if catchup_applied > 0 {
            self.rebuild_awards_full();
            self.reload_qsos();
            self.gui.chrome.status_toast = Some((
                Msg::new(key::MULTI_OP_CATCHUP_TOAST)
                    .arg("count", catchup_applied)
                    .text(self.gui.appearance.language),
                std::time::Instant::now(),
            ));
        }
    }

    pub fn clear_qso_form(&mut self) {
        self.entry_callsign.clear();
        self.entry_name.clear();
        self.entry_qth.clear();
        self.entry_grid.clear();
        self.entry_pga.clear();
        self.entry_comment.clear();
        self.entry_iota.clear();
        self.entry_state.clear();
        self.entry_sota.clear();
        self.entry_pota.clear();
        self.entry_qsl_manager.clear();
        self.active_qsl_manager_record = None;
        self.scp_suggestions.clear();
        self.active_prefix_info = None;
        self.active_polish_district = None;
        self.active_award_status = None;
        self.active_distance_km = 0.0;
        self.active_bearing_deg = 0.0;
        self.active_propagation = None;
        self.active_clubs.clear();
        self.past_qsos_for_active_call.clear();
        self.focus_callsign_requested = true;
    }

    /// Zwraca odległość do korespondenta sformatowaną wg wybranej jednostki (km/mi/nmi).
    pub fn active_distance_display(&self) -> String {
        let km = self.active_distance_km;
        match self.distance_unit.as_str() {
            "mi" => format!("{:.0} mi", km * 0.621_371),
            "nmi" => format!("{:.0} NM", km * 0.539_957),
            _ => format!("{km:.0} km"),
        }
    }

    /// Czyści formularz QSO i natychmiast przenosi kursor/fokus na pole znaku (Wipe).
    pub fn wipe_qso_form(&mut self) {
        self.clear_qso_form();
        self.views.panel_qso.visible = true;
        self.focus_callsign_requested = true;
    }

    pub fn reload_qsos(&mut self) {
        // Pobierz duży bufor żeby paginacja w tabeli miała z czego czerpać.
        // Gdy dziennik jest większy niż bufor, ładujemy całość — w przeciwnym
        // razie sortowanie, filtrowanie i eksport zakresu „Filtered" operowałyby
        // na niepełnych danych.
        if let Ok(db) = self.services.log_db.lock() {
            let total = db
                .count_qsos_for_journal(&self.active_journal.id)
                .unwrap_or(0);
            let requested = if self.log_page_size == 0 {
                10000
            } else {
                (self.log_page_size * 20).max(500)
            };
            let limit = requested.max(total);
            self.recent_qsos = db
                .get_recent_qsos_for_journal(&self.active_journal.id, limit)
                .unwrap_or_default();
            match db.qso_numbers_for_journal(&self.active_journal.id) {
                Ok(numbers) => self.qso_numbers = numbers,
                Err(error) => {
                    log::error!("Failed to calculate QSO serial numbers: {error}");
                    self.qso_numbers.clear();
                    self.gui.chrome.status_toast = Some((
                        Msg::new(key::QSO_SERIAL_ERROR)
                            .arg("err", &error)
                            .text(self.gui.appearance.language),
                        std::time::Instant::now(),
                    ));
                }
            }
        }
        // Zresetuj stronę jeśli wyszła poza zakres
        if self.log_page_size > 0 {
            let pages =
                (self.recent_qsos.len() + self.log_page_size - 1) / self.log_page_size.max(1);
            if self.log_page >= pages && pages > 0 {
                self.log_page = pages - 1;
            }
        }
    }

    /// Ustawia filtr drill-down i przenosi widok do tabeli logu, aby pokazać
    /// listę QSO pasujących do klikniętej pozycji w statystykach.
    pub fn drill_down(&mut self, filter: DrillFilter) {
        self.log_drill_filter = Some(filter);
        self.log_search_query.clear();
        self.log_page = 0;
        self.focus_logbook();
    }

    /// Czyści aktywny filtr drill-down (wraca do pełnego widoku logu).
    pub fn clear_drill_down(&mut self) {
        self.log_drill_filter = None;
        self.log_page = 0;
    }

    /// Pokazuje panel logu i ustawia go jako aktywną zakładkę w swojej kolumnie.
    pub fn focus_logbook(&mut self) {
        self.views.panel_log.visible = true;
    }

    pub fn rebuild_awards_full(&mut self) {
        let active_id = self.active_journal.id.clone();
        let journal_qsos = if let Ok(db) = self.services.log_db.lock() {
            db.get_all_qsos().ok().map(|all_qsos| {
                all_qsos
                    .into_iter()
                    .filter(|q| {
                        q.journal_id
                            .as_deref()
                            .unwrap_or("DEFAULT")
                            .eq_ignore_ascii_case(&active_id)
                    })
                    .collect::<Vec<_>>()
            })
        } else {
            None
        };
        if let Some(qsos) = journal_qsos {
            if let Ok(mut awards) = self.services.awards_engine.lock() {
                awards.rebuild_from_qsos(&qsos);
            }
        }
        self.reload_qsos();
        // Stan nagród się zmienił — odznaki spotów klastra są nieaktualne.
        self.invalidate_cluster_badges();
    }

    pub fn delete_selected_qso(&mut self) {
        let ids: Vec<i64> = std::mem::take(&mut self.selected_qso_ids);
        if ids.is_empty() {
            return;
        }
        let mut first_err: Option<String> = None;
        for id in &ids {
            let result = {
                let db = self
                    .services
                    .log_db
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                db.delete_qso(*id)
            };
            if let Err(e) = result {
                if first_err.is_none() {
                    first_err = Some(
                        Msg::new(key::MSG_QSO_DELETE_ERROR_ID)
                            .arg("id", id)
                            .arg("err", &e.to_string())
                            .text(self.gui.appearance.language),
                    );
                }
            }
        }
        if let Some(msg) = first_err {
            self.report_error(msg);
        } else {
            self.gui.chrome.status_message = Some(
                Msg::new(key::MSG_QSO_DELETED_MULTIPLE)
                    .arg("count", ids.len())
                    .text(self.gui.appearance.language),
            );
        }
        self.rebuild_awards_full();
        self.reload_qsos();
    }

    pub fn delete_qso_by_id(&mut self, id: i64) {
        let result = {
            let db = self
                .services
                .log_db
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            db.delete_qso(id)
        };
        if let Err(e) = result {
            self.report_error(
                Msg::new(key::MSG_QSO_DELETE_ERROR_ID)
                    .arg("id", id)
                    .arg("err", &e)
                    .text(self.gui.appearance.language),
            );
            return;
        }
        self.rebuild_awards_full();
        self.reload_qsos();
        self.gui.chrome.status_message = Some(
            Msg::new(key::MSG_QSO_DELETED_ID)
                .arg("id", id)
                .text(self.gui.appearance.language),
        );
    }

    /// Cofa ostatnie usunięte QSO (Undo)
    pub fn perform_undo(&mut self) {
        if let Some(qso) = self.undo_stack.pop_front() {
            let callsign = qso.callsign.clone();
            // Zachowujemy oryginalny identyfikator rekordu (o ile istniał), aby
            // przywrócona łączność zachowała tożsamość dla redo i referencji.
            let restored = qso.clone();
            let result = {
                let db = self
                    .services
                    .log_db
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                db.restore_qso(&restored)
            };
            match result {
                Ok(id) => {
                    // Zapisujemy rzeczywiste ID przywróconego rekordu,
                    // aby Redo usuwał dokładnie ten rekord, a nie dopasowany po znaku/dacie.
                    let mut redo_entry = restored;
                    redo_entry.id = Some(id);
                    self.redo_stack.push_front(redo_entry);
                }
                Err(e) => {
                    self.report_error(
                        Msg::new(key::MSG_QSO_RESTORE_ERROR)
                            .arg("err", &e.to_string())
                            .text(self.gui.appearance.language),
                    );
                    return;
                }
            }
            self.rebuild_awards_full();
            self.reload_qsos();
            self.gui.chrome.status_message = Some(
                Msg::new(key::MSG_QSO_RESTORED_CALL)
                    .arg("call", &callsign)
                    .text(self.gui.appearance.language),
            );
        }
    }

    /// Ponawia cofnięte QSO (Redo — usuwa ponownie)
    pub fn perform_redo(&mut self) {
        if let Some(qso) = self.redo_stack.pop_front() {
            let callsign = qso.callsign.clone();
            let Some(id) = qso.id else {
                self.report_error(
                    Msg::new(key::MSG_QSO_REDO_NO_ID)
                        .arg("callsign", &callsign)
                        .text(self.gui.appearance.language),
                );
                return;
            };
            let result = {
                let db = self
                    .services
                    .log_db
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                db.delete_qso(id)
            };
            if let Err(e) = result {
                self.report_error(
                    Msg::new(key::MSG_QSO_REDO_DELETE_ERROR)
                        .arg("id", id)
                        .arg("err", &e)
                        .text(self.gui.appearance.language),
                );
                return;
            }
            self.undo_stack.push_front(qso);
            self.rebuild_awards_full();
            self.reload_qsos();
            self.gui.chrome.status_message = Some(
                Msg::new(key::MSG_QSO_REDO_DELETED_CALL)
                    .arg("call", &callsign)
                    .text(self.gui.appearance.language),
            );
        }
    }

    pub fn save_edited_qso(&mut self) {
        let mut warnings_toast: Option<String> = None;
        if let Some(ref qso) = self.editing_qso {
            if let Err(e) = qso.validate() {
                self.report_error(
                    Msg::new(key::MSG_QSO_CHANGES_NOT_SAVED)
                        .arg("reason", e.msg().text(self.gui.appearance.language))
                        .text(self.gui.appearance.language),
                );
                return;
            }
            let warnings = qso.warnings();
            if !warnings.is_empty() {
                let warning_texts: Vec<String> = warnings
                    .iter()
                    .map(|w| w.msg().text(self.gui.appearance.language))
                    .collect();
                warnings_toast = Some(format!(
                    "{}: {}",
                    tr(key::LOG_WARNINGS_LABEL, self.gui.appearance.language),
                    warning_texts.join("; ")
                ));
            }
            if let Some(id) = qso.id {
                let result = {
                    let db = self
                        .services
                        .log_db
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    db.update_qso(id, qso)
                };
                if let Err(e) = result {
                    self.report_error(
                        Msg::new(key::MSG_QSO_UPDATE_ERROR_ID)
                            .arg("id", id)
                            .arg("err", &e.to_string())
                            .text(self.gui.appearance.language),
                    );
                    return;
                }
                self.gui.chrome.status_message = Some(
                    Msg::new(key::MSG_QSO_UPDATED_CALL)
                        .arg("call", &qso.callsign)
                        .text(self.gui.appearance.language),
                );
            }
        }
        if let Some(msg) = warnings_toast {
            self.report_warning(msg);
        }
        self.editing_qso = None;
        self.rebuild_awards_full();
    }
}
