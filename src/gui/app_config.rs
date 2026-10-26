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
        cfg.cat_enabled = self.cat.connected;
        cfg.propagation_history = self.solar.propagation_history.clone();

        cfg.wol_mac = self.gui.dialogs.wol_dialog.mac_address.clone();
        cfg.wol_ip = self.gui.dialogs.wol_dialog.broadcast_ip.clone();
        cfg.wol_port = self.gui.dialogs.wol_dialog.port;

        cfg.current_language = self.gui.appearance.language.code().to_string();
        cfg.dock_layout = self.serialize_dock_layout();

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
            self.entry.pota = prof.pota_ref.clone().unwrap_or_default();
            self.entry.sota = prof.sota_ref.clone().unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::LogDatabase;
    use crate::core::i18n::Language;
    use crate::core::prefix::PrefixMatcher;
    use crate::core::scp::ScpEngine;
    use crate::gui::app::{AppDeps, AppInit, ShutdownPolicy, SpLogApp};
    use crate::gui::theme::ThemePreset;
    use std::sync::{Arc, Mutex};

    fn build_test_app(config_file: std::path::PathBuf) -> SpLogApp {
        let deps = AppDeps {
            log_db: Arc::new(Mutex::new(LogDatabase::open_in_memory().unwrap())),
            prefix_matcher: Arc::new(PrefixMatcher::empty()),
            scp_engine: Arc::new(Mutex::new(ScpEngine::new())),
        };
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: config_file,
            active_db_path: std::path::PathBuf::from(":memory:"),
            shutdown_policy: ShutdownPolicy::None,
        };
        SpLogApp::for_tests(init, deps)
    }

    #[test]
    fn snapshot_config_roundtrips_through_file() {
        let temp = tempfile::tempdir().unwrap();
        let cfg_path = temp.path().join("station_config.json");
        let mut app = build_test_app(cfg_path.clone());

        app.config.station.callsign = "SP6TEST".to_string();
        app.config.font_scale = 1.15;
        app.config.distance_unit = "nmi".to_string();
        app.save_station_config();

        let loaded = AppConfig::load_from_file(&cfg_path).unwrap();
        let snapshot = app.snapshot_config();

        assert_eq!(loaded.station.callsign, "SP6TEST");
        assert_eq!(loaded.font_scale, snapshot.font_scale);
        assert_eq!(loaded.distance_unit, snapshot.distance_unit);
        assert_eq!(loaded.station.callsign, snapshot.station.callsign);
    }

    #[test]
    fn snapshot_config_overrides_derived_fields() {
        let temp = tempfile::tempdir().unwrap();
        let cfg_path = temp.path().join("station_config.json");
        let mut app = build_test_app(cfg_path);

        // is_configured derived field
        app.gui.chrome.show_welcome_wizard = false;
        app.config.station.callsign = "SP6TEST".to_string();
        assert!(app.snapshot_config().is_configured);

        app.gui.chrome.show_welcome_wizard = true;
        assert!(!app.snapshot_config().is_configured);

        app.gui.chrome.show_welcome_wizard = false;
        app.config.station.callsign = "N0CALL".to_string();
        assert!(!app.snapshot_config().is_configured);

        // theme & dark_theme derived fields
        app.gui.appearance.theme_preset = ThemePreset::Daylight;
        let snap_daylight = app.snapshot_config();
        assert_eq!(snap_daylight.theme_preset, "daylight");
        assert!(!snap_daylight.dark_theme);

        app.gui.appearance.theme_preset = ThemePreset::OperatorDark;
        let snap_dark = app.snapshot_config();
        assert_eq!(snap_dark.theme_preset, "operator_dark");
        assert!(snap_dark.dark_theme);

        // language derived field
        app.gui.appearance.language = Language::En;
        assert_eq!(app.snapshot_config().current_language, "en");

        // CAT enabled derived field
        app.cat.connected = true;
        assert!(app.snapshot_config().cat_enabled);
        app.cat.connected = false;
        assert!(!app.snapshot_config().cat_enabled);

        // WOL derived fields
        app.gui.dialogs.wol_dialog.mac_address = "AA:BB:CC:DD:EE:FF".to_string();
        app.gui.dialogs.wol_dialog.broadcast_ip = "192.168.1.255".to_string();
        app.gui.dialogs.wol_dialog.port = 9;
        let snap_wol = app.snapshot_config();
        assert_eq!(snap_wol.wol_mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(snap_wol.wol_ip, "192.168.1.255");
        assert_eq!(snap_wol.wol_port, 9);

        // secret_store_id derived field
        app.paths.secret_store_id = "test-store-123".to_string();
        assert_eq!(app.snapshot_config().secret_store_id, "test-store-123");
    }

    #[test]
    fn legacy_config_file_loads() {
        let temp = tempfile::tempdir().unwrap();
        let cfg_path = temp.path().join("legacy_station_config.json");
        let legacy_json = r#"{
            "station": {
                "id": "default",
                "name": "Main",
                "callsign": "SP1LEGACY",
                "operator": "Jan",
                "gridsquare": "JO73GO",
                "city": "Szczecin",
                "country": "Poland",
                "itu_zone": 28,
                "cq_zone": 14,
                "power_watts": 100
            },
            "font_scale": 1.2,
            "lan_sync_auto_start": true
        }"#;
        std::fs::write(&cfg_path, legacy_json).unwrap();

        let loaded = AppConfig::load_without_secrets(&cfg_path).unwrap();
        assert_eq!(loaded.station.callsign, "SP1LEGACY");
        assert_eq!(loaded.station.operator, "Jan");
        assert_eq!(loaded.station.gridsquare, "JO73GO");
        assert!((loaded.font_scale - 1.2).abs() < 1e-4);
    }
}
