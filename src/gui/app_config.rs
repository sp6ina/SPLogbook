use super::SpLogApp;
use crate::core::i18n::{Msg, key, tr};
use crate::core::station::{AppConfig, EquipmentItem};

impl SpLogApp {
    /// Cyklicznie przełącza motyw kolorystyczny (Dark -> Daylight -> High-Contrast).
    pub fn cycle_theme(&mut self) {
        let all = crate::gui::theme::ThemePreset::ALL;
        let idx = all
            .iter()
            .position(|t| t.id() == self.gui.appearance.theme_preset.id())
            .unwrap_or(0);
        self.gui.appearance.theme_preset = all[(idx + 1) % all.len()];
        self.save_station_config();
        self.gui.chrome.status_toast = Some((
            format!(
                "Zmieniono motyw: {}",
                self.gui
                    .appearance
                    .theme_preset
                    .label(self.gui.appearance.language)
            ),
            std::time::Instant::now(),
        ));
    }

    /// Tworzy zaktualizowaną strukturę [`AppConfig`] odzwierciedlającą bieżący stan aplikacji.
    pub fn build_app_config(&self) -> AppConfig {
        let is_configured = if self.gui.chrome.show_welcome_wizard {
            false
        } else {
            !self.my_station.callsign.is_empty() && self.my_station.callsign != "N0CALL"
        };
        AppConfig {
            secret_store_id: self.secret_store_id.clone(),
            is_configured,
            station: self.my_station.clone(),
            equipment: self.ledger.items.clone(),
            dark_theme: self.gui.appearance.theme_preset.is_dark(),
            theme_preset: self.gui.appearance.theme_preset.id().to_string(),

            cat_host: self.cat_host.clone(),
            cat_port: self.cat_port,
            cat_enabled: self.cat_connected,
            cat_poll_rate_ms: self.cat_poll_rate_ms,
            cat_rig_model: self.cat_rig_model.clone(),
            cat_serial_port: self.cat_serial_port.clone(),
            cat_baud_rate: self.cat_baud_rate,
            cat_rig_id: self.cat_rig_id,
            cat_auto_start_rigctld: self.cat_auto_start_rigctld,
            cat_hamlib_source: self.cat_hamlib_source.clone(),
            cat_custom_rigctld_path: self.cat_custom_rigctld_path.clone(),

            station_profiles: self.station_profiles.clone(),
            active_profile_id: self.active_profile_id.clone(),
            cat_sharing_enabled: self.cat_sharing_enabled,
            cat_sharing_port: self.cat_sharing_port,

            rotor_host: self.rotor_host.clone(),
            rotor_port: self.rotor_port,

            cat_backend: self.cat_backend.clone(),
            tci_host: self.tci_host.clone(),
            tci_port: self.tci_port,
            cat_station: self.cat_station.clone(),

            fldigi_enabled: self.fldigi_enabled,
            fldigi_host: self.fldigi_host.clone(),
            fldigi_port: self.fldigi_port,

            psk_reporter_enabled: self.psk_reporter_enabled,

            n1mm_broadcast_enabled: self.n1mm_broadcast_enabled,
            n1mm_broadcast_host: self.n1mm_broadcast_host.clone(),
            n1mm_broadcast_port: self.n1mm_broadcast_port,

            voice_keyer_messages: self.voice_keyer.messages.clone(),

            workspace_profiles: self.workspace_profiles.clone(),

            plugins_enabled: self.plugins_enabled,
            plugins_dir: self.plugins_dir.clone(),
            operator_assistant_enabled: self.operator_assistant_enabled,
            propagation_history: self.solar.propagation_history.clone(),

            lan_sync_port: self.lan_sync_port,
            lan_sync_server_ip: self.lan_sync_server_ip.clone(),
            lan_sync_secret: self.lan_sync_secret.clone(),

            lotw_tqsl_path: self.lotw_tqsl_path.clone(),
            lotw_station_name: self.lotw_station_name.clone(),
            lotw_username: self.lotw_username.clone(),
            lotw_password: self.lotw_password.clone(),

            qrz_username: self.qrz_username.clone(),
            qrz_password: self.qrz_password.clone(),
            qrz_api_key: self.qrz_api_key.clone(),
            qrz_auto_lookup: self.qrz_auto_lookup,

            eqsl_username: self.online.eqsl_username.clone(),
            eqsl_password: self.online.eqsl_password.clone(),

            clublog_callsign: self.online.clublog_callsign.clone(),
            clublog_email: self.online.clublog_email.clone(),
            clublog_password: self.online.clublog_password.clone(),
            clublog_api_key: self.online.clublog_api_key.clone(),

            cloudlog_url: self.cloudlog_url.clone(),
            cloudlog_api_key: self.cloudlog_api_key.clone(),
            rest_api_key: self.rest_api_key.clone(),
            hrdlog_username: self.hrdlog_username.clone(),
            hrdlog_upload_code: self.hrdlog_upload_code.clone(),
            hamqth_username: self.hamqth_username.clone(),
            hamqth_password: self.hamqth_password.clone(),

            callbook_priority: self.callbook_priority.clone(),
            callbook_cache_enabled: self.callbook_cache_enabled,
            callbook_cache_ttl_days: self.callbook_cache_ttl_days,

            wol_mac: self.gui.dialogs.wol_dialog.mac_address.clone(),
            wol_ip: self.gui.dialogs.wol_dialog.broadcast_ip.clone(),
            wol_port: self.gui.dialogs.wol_dialog.port,

            current_language: self.gui.appearance.language.code().to_string(),
            compact_hud_mode: self.compact_hud_mode,
            tabbed_columns: self.tabbed_columns,
            dock_layout: self.serialize_dock_layout(),
            font_scale: self.font_scale,
            font_family: self.font_family.clone(),
            distance_unit: self.distance_unit.clone(),
            hud_always_on_top: self.hud_always_on_top,
            hud_saved_pos: self.hud_saved_pos,
            hud_saved_size: self.hud_saved_size,
            hud_operating_bar: self.hud_operating_bar,
            main_window_pos: self.main_window_pos,
            main_window_size: self.main_window_size,
            main_window_maximized: self.main_window_maximized,

            live_auto_upload_clublog: self.live_auto_upload_clublog,
            live_auto_upload_qrz: self.live_auto_upload_qrz,

            cluster_host: self.cluster_host.clone(),
            cluster_port: self.cluster_port,
            cluster_callsign: self.cluster_callsign.clone(),
            cluster_auto_connect: self.cluster_auto_connect,
            cluster_filter_current_band: self.cluster_filter_current_band,
            cluster_hide_ft8: self.cluster_hide_ft8,
            cluster_hide_skimmers: self.cluster_hide_skimmers,
            custom_clusters: self.custom_clusters.clone(),
            quick_access: self.quick_access.clone(),

            left_column_width: self.left_column_width,
            right_column_width: self.right_column_width,

            panel_vfo: self.views.panel_vfo.clone(),
            panel_qso: self.views.panel_qso.clone(),
            panel_log: self.views.panel_log.clone(),
            panel_cluster: self.views.panel_cluster.clone(),
            panel_solar: self.views.panel_solar.clone(),
            panel_bandmap: self.views.panel_bandmap.clone(),
            panel_satellites: self.views.panel_satellites.clone(),
            panel_world_map: self.views.panel_world_map.clone(),
            panel_waterfall: self.views.panel_waterfall.clone(),
            logbook_columns: self.logbook_columns.clone(),
            logbook_column_presets: self.logbook_column_presets.clone(),
            custom_contests: self.custom_contests.clone(),
        }
    }

    pub fn save_station_config(&mut self) {
        let mut cfg = self.build_app_config();
        if let Err(e) = cfg.save_to_file(&self.config_file_path) {
            self.report_error(
                Msg::new(key::CONFIG_SAVE_ERROR)
                    .arg("err", &e)
                    .text(self.gui.appearance.language),
            );
        } else {
            self.secret_store_id = cfg.secret_store_id;
        }
    }

    /// Scentralizowane zgłaszanie błędów: log + pływające powiadomienie w UI.
    pub fn report_error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::error!("{msg}");
        self.gui.chrome.status_toast = Some((msg, std::time::Instant::now()));
    }

    /// Scentralizowane zgłaszanie ostrzeżeń (log warning + pływające powiadomienie).
    pub fn report_warning(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::warn!("{msg}");
        self.gui.chrome.status_toast = Some((msg, std::time::Instant::now()));
    }

    /// Zlokalizowane zgłoszenie informacji/statusu (UI toast + log info).
    pub fn say(&mut self, msg: Msg) {
        let text = msg.text(self.gui.appearance.language);
        log::info!("{text}");
        self.gui.chrome.status_message = Some(text.clone());
        self.gui.chrome.status_toast = Some((text, std::time::Instant::now()));
    }

    /// Zlokalizowane zgłoszenie ostrzeżenia (UI toast + log warn).
    pub fn warn(&mut self, msg: Msg) {
        let text = msg.text(self.gui.appearance.language);
        log::warn!("{text}");
        self.gui.chrome.status_toast = Some((text, std::time::Instant::now()));
    }

    /// Zlokalizowane zgłoszenie błędu (UI toast + log error).
    pub fn fail(&mut self, msg: Msg) {
        let text = msg.text(self.gui.appearance.language);
        log::error!("{text}");
        self.gui.chrome.status_toast = Some((text, std::time::Instant::now()));
    }

    pub fn add_new_equipment(&mut self) {
        let next_num = self
            .ledger
            .items
            .iter()
            .filter_map(|e| {
                e.id.strip_prefix("eq-")
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(self.ledger.items.len())
            + 1;
        let item = EquipmentItem {
            id: format!("eq-{next_num}"),
            category: self.ledger.new_cat,
            manufacturer: self.ledger.new_mfr.trim().to_string(),
            model: self.ledger.new_model.trim().to_string(),
            serial_number: if self.ledger.new_sn.trim().is_empty() {
                None
            } else {
                Some(self.ledger.new_sn.trim().to_string())
            },
            purchase_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            notes: if self.ledger.new_notes.trim().is_empty() {
                None
            } else {
                Some(self.ledger.new_notes.trim().to_string())
            },
        };
        self.ledger.items.push(item);
        self.ledger.new_model.clear();
        self.ledger.new_sn.clear();
        self.ledger.new_notes.clear();
        self.save_station_config();
        self.gui.chrome.status_message =
            Some(tr(key::EQ_ADDED, self.gui.appearance.language).to_string());
    }

    pub fn save_edited_equipment(&mut self, id: &str) {
        if let Some(item) = self.ledger.items.iter_mut().find(|i| i.id == id) {
            item.category = self.ledger.edit_cat;
            item.manufacturer = self.ledger.edit_mfr.trim().to_string();
            item.model = self.ledger.edit_model.trim().to_string();
            item.serial_number = if self.ledger.edit_sn.trim().is_empty() {
                None
            } else {
                Some(self.ledger.edit_sn.trim().to_string())
            };
            item.purchase_date = if self.ledger.edit_date.trim().is_empty() {
                None
            } else {
                Some(self.ledger.edit_date.trim().to_string())
            };
            item.notes = if self.ledger.edit_notes.trim().is_empty() {
                None
            } else {
                Some(self.ledger.edit_notes.trim().to_string())
            };
        }
        self.ledger.editing_id = None;
        self.save_station_config();
        self.gui.chrome.status_message =
            Some(tr(key::EQ_UPDATED, self.gui.appearance.language).to_string());
    }

    pub fn delete_equipment_item(&mut self, id: &str) {
        self.ledger.items.retain(|i| i.id != id);
        if self.ledger.editing_id.as_deref() == Some(id) {
            self.ledger.editing_id = None;
        }
        self.save_station_config();
        self.gui.chrome.status_message =
            Some(tr(key::EQ_DELETED, self.gui.appearance.language).to_string());
    }

    pub fn activate_station_profile(&mut self, profile_id: &str) {
        if let Some(prof) = self
            .station_profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
        {
            self.active_profile_id.clone_from(&prof.id);
            self.my_station = prof.clone();
            self.entry_pota = prof.pota_ref.clone().unwrap_or_default();
            self.entry_sota = prof.sota_ref.clone().unwrap_or_default();
            self.save_station_config();
            let prof_name = if prof.name.is_empty() {
                &prof.callsign
            } else {
                &prof.name
            };
            self.gui.chrome.status_toast = Some((
                Msg::new(key::PROFILE_SWITCHED_STATION)
                    .arg("name", prof_name)
                    .text(self.gui.appearance.language),
                std::time::Instant::now(),
            ));
        }
    }
}
