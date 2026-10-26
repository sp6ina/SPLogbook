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
    pub fn snapshot_config(&self) -> AppConfig {
        let is_configured = if self.gui.chrome.show_welcome_wizard {
            false
        } else {
            !self.config.station.callsign.is_empty() && self.config.station.callsign != "N0CALL"
        };
        let mut cfg = self.config.clone();
        cfg.secret_store_id = self.paths.secret_store_id.clone();
        cfg.is_configured = is_configured;
        cfg.dark_theme = self.gui.appearance.theme_preset.is_dark();
        cfg.theme_preset = self.gui.appearance.theme_preset.id().to_string();
        cfg.cat_enabled = self.cat_connected;

        cfg.plugins_enabled = self.plugins_enabled;
        cfg.plugins_dir = self.plugins_dir.clone();
        cfg.operator_assistant_enabled = self.operator_assistant_enabled;
        cfg.propagation_history = self.solar.propagation_history.clone();

        cfg.wol_mac = self.gui.dialogs.wol_dialog.mac_address.clone();
        cfg.wol_ip = self.gui.dialogs.wol_dialog.broadcast_ip.clone();
        cfg.wol_port = self.gui.dialogs.wol_dialog.port;

        cfg.current_language = self.gui.appearance.language.code().to_string();
        cfg.compact_hud_mode = self.compact_hud_mode;
        cfg.tabbed_columns = self.tabbed_columns;
        cfg.dock_layout = self.serialize_dock_layout();
        cfg.font_scale = self.font_scale;
        cfg.font_family = self.font_family.clone();
        cfg.distance_unit = self.distance_unit.clone();
        cfg.hud_always_on_top = self.hud_always_on_top;
        cfg.hud_saved_pos = self.hud_saved_pos;
        cfg.hud_saved_size = self.hud_saved_size;
        cfg.hud_operating_bar = self.hud_operating_bar;
        cfg.main_window_pos = self.main_window_pos;
        cfg.main_window_size = self.main_window_size;
        cfg.main_window_maximized = self.main_window_maximized;

        cfg.left_column_width = self.left_column_width;
        cfg.right_column_width = self.right_column_width;

        cfg.panel_vfo = self.views.panel_vfo.clone();
        cfg.panel_qso = self.views.panel_qso.clone();
        cfg.panel_log = self.views.panel_log.clone();
        cfg.panel_cluster = self.views.panel_cluster.clone();
        cfg.panel_solar = self.views.panel_solar.clone();
        cfg.panel_bandmap = self.views.panel_bandmap.clone();
        cfg.panel_satellites = self.views.panel_satellites.clone();
        cfg.panel_world_map = self.views.panel_world_map.clone();
        cfg.panel_waterfall = self.views.panel_waterfall.clone();
        cfg
    }

    /// Tworzy zaktualizowaną strukturę [`AppConfig`] odzwierciedlającą bieżący stan aplikacji.
    pub fn build_app_config(&self) -> AppConfig {
        self.snapshot_config()
    }

    pub fn save_station_config(&mut self) {
        let mut cfg = self.snapshot_config();
        if let Err(e) = cfg.save_to_file(&self.paths.config_file_path) {
            self.report_error(
                Msg::new(key::CONFIG_SAVE_ERROR)
                    .arg("err", &e)
                    .text(self.gui.appearance.language),
            );
        } else {
            self.paths.secret_store_id.clone_from(&cfg.secret_store_id);
            self.config = cfg;
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
            .config
            .equipment
            .iter()
            .filter_map(|e| {
                e.id.strip_prefix("eq-")
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(self.config.equipment.len())
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
        self.config.equipment.push(item);
        self.ledger.new_model.clear();
        self.ledger.new_sn.clear();
        self.ledger.new_notes.clear();
        self.save_station_config();
        self.gui.chrome.status_message =
            Some(tr(key::EQ_ADDED, self.gui.appearance.language).to_string());
    }

    pub fn save_edited_equipment(&mut self, id: &str) {
        if let Some(item) = self.config.equipment.iter_mut().find(|i| i.id == id) {
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
        self.config.equipment.retain(|i| i.id != id);
        if self.ledger.editing_id.as_deref() == Some(id) {
            self.ledger.editing_id = None;
        }
        self.save_station_config();
        self.gui.chrome.status_message =
            Some(tr(key::EQ_DELETED, self.gui.appearance.language).to_string());
    }

    pub fn activate_station_profile(&mut self, profile_id: &str) {
        if let Some(prof) = self
            .config
            .station_profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
        {
            self.config.active_profile_id.clone_from(&prof.id);
            self.config.station = prof.clone();
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
