// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use serde::{Deserialize, Serialize};

/// Profil stacji roboczej (Domowa, Terenowa /P, Mobilna /M, Aktywacja SOTA/POTA)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StationProfile {
    pub id: String,
    pub name: String,
    pub callsign: String,
    pub operator: String,
    pub gridsquare: String,
    pub city: String,
    pub country: String,
    pub itu_zone: u32,
    pub cq_zone: u32,
    pub pga_gmina: Option<String>,
    pub sota_ref: Option<String>,
    pub pota_ref: Option<String>,
    pub power_watts: u32,
    pub default_rig: Option<String>,
    pub default_antenna: Option<String>,
}

impl Default for StationProfile {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Home QTH".to_string(),
            callsign: String::new(),
            operator: String::new(),
            gridsquare: String::new(),
            city: String::new(),
            country: String::new(),
            itu_zone: 0,
            cq_zone: 0,
            pga_gmina: None,
            sota_ref: None,
            pota_ref: None,
            power_watts: 100,
            default_rig: None,
            default_antenna: None,
        }
    }
}

/// Sprzęt radiowy w stacji (Księga sprzętowa - Equipment Ledger)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentItem {
    pub id: String,
    pub category: EquipmentCategory,
    pub model: String,
    pub manufacturer: String,
    pub serial_number: Option<String>,
    pub purchase_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EquipmentCategory {
    Transceiver,
    Antenna,
    Amplifier,
    Tuner,
    PowerSupply,
    Keyer,
    Other,
}

impl EquipmentCategory {
    pub fn all() -> &'static [EquipmentCategory] {
        &[
            EquipmentCategory::Transceiver,
            EquipmentCategory::Antenna,
            EquipmentCategory::Amplifier,
            EquipmentCategory::Tuner,
            EquipmentCategory::PowerSupply,
            EquipmentCategory::Keyer,
            EquipmentCategory::Other,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EquipmentCategory::Transceiver => "Transceiver",
            EquipmentCategory::Antenna => "Antenna",
            EquipmentCategory::Amplifier => "Amplifier",
            EquipmentCategory::Tuner => "Tuner",
            EquipmentCategory::PowerSupply => "Power Supply",
            EquipmentCategory::Keyer => "CW Keyer",
            EquipmentCategory::Other => "Other",
        }
    }
}

fn default_left_col_width() -> f32 {
    350.0
}
fn default_right_col_width() -> f32 {
    360.0
}
fn default_rotor_host() -> String { "127.0.0.1".to_string() }
fn default_rotor_port() -> u16 { 4533 }
fn default_cat_backend() -> String { "hamlib".to_string() }
fn default_tci_host() -> String { "127.0.0.1".to_string() }
fn default_tci_port() -> u16 { 40001 }
fn default_fldigi_host() -> String { "127.0.0.1".to_string() }
fn default_fldigi_port() -> u16 { 7362 }
fn default_lan_sync_port() -> u16 { 7373 }
fn default_lan_sync_server_ip() -> String { "127.0.0.1".to_string() }
fn default_profile_id() -> String { "default".to_string() }
fn default_cat_sharing_port() -> u16 { 4534 }

/// Konfiguracja pojedynczej kolumny w tabeli dziennika łączności
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LogColumn {
    pub id: String,
    pub label: String,
    pub visible: bool,
    pub width: f32,
}

impl LogColumn {
    fn new(id: &str, label: &str, visible: bool, width: f32) -> Self {
        Self { id: id.to_string(), label: label.to_string(), visible, width }
    }
}

pub fn default_logbook_columns() -> Vec<LogColumn> {
    vec![
        LogColumn::new("nr",       "#",        true,  40.0),
        LogColumn::new("date",     "Data",     true,  85.0),
        LogColumn::new("time",     "Czas",     true,  55.0),
        LogColumn::new("callsign", "Znak",     true,  105.0),
        LogColumn::new("band",     "Pasmo",    true,  60.0),
        LogColumn::new("mode",     "Emisja",   true,  60.0),
        LogColumn::new("rst_s",    "RST S",    true,  55.0),
        LogColumn::new("rst_r",    "RST R",    true,  55.0),
        LogColumn::new("country",  "Kraj",     true,  130.0),
        LogColumn::new("name",     "Imie",     false, 90.0),
        LogColumn::new("qsl",      "QSL",      true,  55.0),
        LogColumn::new("freq",     "Freq",     false, 80.0),
        LogColumn::new("cqz",      "CQZ",      false, 45.0),
        LogColumn::new("iota",     "IOTA",     false, 70.0),
        LogColumn::new("comment",  "Uwagi",    false, 160.0),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub is_configured: bool,
    pub station: StationProfile,
    pub equipment: Vec<EquipmentItem>,
    pub dark_theme: bool,

    // Konfiguracja Hamlib CAT
    pub cat_host: String,
    pub cat_port: u16,
    pub cat_enabled: bool,
    pub cat_poll_rate_ms: u64,
    pub cat_rig_model: String,
    pub cat_serial_port: String,
    pub cat_baud_rate: u32,
    pub cat_rig_id: u32,
    pub cat_auto_start_rigctld: bool,

    // Profile stacji roboczej (Wieloprofilowość)
    #[serde(default)]
    pub station_profiles: Vec<StationProfile>,
    #[serde(default = "default_profile_id")]
    pub active_profile_id: String,

    // Udostępnianie CAT (Hamlib proxy server dla WSJT-X / JTDX)
    #[serde(default)]
    pub cat_sharing_enabled: bool,
    #[serde(default = "default_cat_sharing_port")]
    pub cat_sharing_port: u16,

    // Konfiguracja Rotora
    #[serde(default = "default_rotor_host")]
    pub rotor_host: String,
    #[serde(default = "default_rotor_port")]
    pub rotor_port: u16,

    // Konfiguracja TCI
    #[serde(default = "default_cat_backend")]
    pub cat_backend: String,
    #[serde(default = "default_tci_host")]
    pub tci_host: String,
    #[serde(default = "default_tci_port")]
    pub tci_port: u16,

    // Konfiguracja FLDigi
    #[serde(default)]
    pub fldigi_enabled: bool,
    #[serde(default = "default_fldigi_host")]
    pub fldigi_host: String,
    #[serde(default = "default_fldigi_port")]
    pub fldigi_port: u16,

    // Konfiguracja PSK Reporter
    #[serde(default)]
    pub psk_reporter_enabled: bool,

    // Konfiguracja Multi-Op LAN
    #[serde(default = "default_lan_sync_port")]
    pub lan_sync_port: u16,
    #[serde(default)]
    pub lan_sync_auto_start: bool,
    #[serde(default = "default_lan_sync_server_ip")]
    pub lan_sync_server_ip: String,

    // Konfiguracja ARRL LoTW
    pub lotw_tqsl_path: String,
    pub lotw_station_name: String,
    pub lotw_username: String,
    pub lotw_password: String,

    // Konfiguracja QRZ.com XML Callbook
    pub qrz_username: String,
    pub qrz_password: String,
    pub qrz_api_key: String,
    pub qrz_auto_lookup: bool,

    // Konfiguracja eQSL.cc
    pub eqsl_username: String,
    pub eqsl_password: String,

    // Konfiguracja Club Log
    pub clublog_callsign: String,
    pub clublog_email: String,
    pub clublog_password: String,
    pub clublog_api_key: String,

    // Konfiguracja Cloudlog REST API
    pub cloudlog_url: String,
    pub cloudlog_api_key: String,

    // Konfiguracja HRDLog.net
    pub hrdlog_username: String,
    pub hrdlog_upload_code: String,

    // Konfiguracja HamQTH
    pub hamqth_username: String,
    pub hamqth_password: String,

    // Wake-on-LAN
    pub wol_mac: String,
    pub wol_ip: String,
    pub wol_port: u16,

    // Interfejs & Język
    pub current_language: String,
    pub compact_hud_mode: bool,

    // Live Auto-Upload
    pub live_auto_upload_clublog: bool,
    pub live_auto_upload_qrz: bool,

    // DX Cluster Telnet
    pub cluster_host: String,
    pub cluster_port: u16,
    pub cluster_callsign: String,
    pub cluster_auto_connect: bool,
    pub cluster_filter_current_band: bool,
    pub cluster_hide_ft8: bool,
    pub cluster_hide_skimmers: bool,
    #[serde(default)]
    pub custom_clusters: Vec<CustomClusterServer>,
    #[serde(default)]
    pub quick_access: QuickAccessConfig,

    // Konfiguracja kafelków i okien widoku
    #[serde(default = "default_left_col_width")]
    pub left_column_width: f32,
    #[serde(default = "default_right_col_width")]
    pub right_column_width: f32,

    pub panel_vfo: ViewPanelConfig,
    pub panel_qso: ViewPanelConfig,
    pub panel_log: ViewPanelConfig,
    pub panel_cluster: ViewPanelConfig,
    pub panel_bandmap: ViewPanelConfig,
    pub panel_solar: ViewPanelConfig,
    pub panel_satellites: ViewPanelConfig,
    pub panel_world_map: ViewPanelConfig,
    
    #[serde(default = "default_logbook_columns")]
    pub logbook_columns: Vec<LogColumn>,
    
    #[serde(default)]
    pub custom_contests: Vec<CustomContest>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct CustomContest {
    pub name: String,
    pub exchange_format: String,
    pub bands: Vec<String>,
    pub points_per_qso: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickAccessConfig {
    #[serde(default = "default_true_field")]
    pub show_journal: bool,
    #[serde(default = "default_true_field")]
    pub show_wsjtx: bool,
    #[serde(default = "default_true_field")]
    pub show_cluster_status: bool,
    #[serde(default = "default_true_field")]
    pub show_vfo: bool,
    #[serde(default = "default_true_field")]
    pub show_qso: bool,
    #[serde(default = "default_true_field")]
    pub show_log: bool,
    #[serde(default = "default_true_field")]
    pub show_cat: bool,
    #[serde(default = "default_true_field")]
    pub show_cluster: bool,
    #[serde(default = "default_true_field")]
    pub show_bandmap: bool,
    #[serde(default = "default_true_field")]
    pub show_solar: bool,
    #[serde(default = "default_true_field")]
    pub show_lotw: bool,
    #[serde(default = "default_true_field")]
    pub show_map: bool,
    #[serde(default = "default_true_field")]
    pub show_awards: bool,
    #[serde(default = "default_true_field")]
    pub show_cw: bool,
    #[serde(default = "default_true_field")]
    pub show_satellites: bool,
    #[serde(default = "default_true_field")]
    pub show_contest: bool,
    #[serde(default = "default_true_field")]
    pub show_equipment: bool,
    #[serde(default = "default_true_field")]
    pub show_eme: bool,
    #[serde(default = "default_true_field")]
    pub show_wol: bool,
    #[serde(default = "default_true_field")]
    pub show_theme: bool,
}

fn default_true_field() -> bool {
    true
}

impl Default for QuickAccessConfig {
    fn default() -> Self {
        Self {
            show_journal: true,
            show_wsjtx: true,
            show_cluster_status: true,
            show_vfo: true,
            show_qso: true,
            show_log: true,
            show_cat: true,
            show_cluster: true,
            show_bandmap: true,
            show_solar: true,
            show_lotw: true,
            show_map: true,
            show_awards: true,
            show_cw: true,
            show_satellites: true,
            show_contest: true,
            show_equipment: true,
            show_eme: true,
            show_wol: true,
            show_theme: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomClusterServer {
    pub name: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewPanelConfig {
    pub visible: bool,
    pub floating: bool,
    #[serde(default)]
    pub column: usize, // 0 = Lewa, 1 = Srodek, 2 = Prawa
    #[serde(default)]
    pub order: usize,  // Kolejnosc pionowa w kolumnie (0, 1, 2...)
    /// Ostatnia zapisana pozycja okna pływającego [x, y]
    #[serde(default)]
    pub saved_pos: Option<[f32; 2]>,
    /// Ostatni zapisany rozmiar okna pływającego [szerokość, wysokość]
    #[serde(default)]
    pub saved_size: Option<[f32; 2]>,
}

impl Default for ViewPanelConfig {
    fn default() -> Self {
        Self {
            visible: true,
            floating: false,
            column: 0,
            order: 0,
            saved_pos: None,
            saved_size: None,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            is_configured: false,
            station: StationProfile::default(),
            equipment: Vec::new(),
            dark_theme: true,

            cat_host: "127.0.0.1".to_string(),
            cat_port: 4532,
            cat_enabled: true,
            cat_poll_rate_ms: 200,
            cat_rig_model: String::new(),
            cat_serial_port: String::new(),
            cat_baud_rate: 19200,
            cat_rig_id: 1,
            cat_auto_start_rigctld: false,

            station_profiles: Vec::new(),
            active_profile_id: "default".to_string(),
            cat_sharing_enabled: false,
            cat_sharing_port: 4534,

            rotor_host: "127.0.0.1".to_string(),
            rotor_port: 4533,

            cat_backend: "hamlib".to_string(),
            tci_host: "127.0.0.1".to_string(),
            tci_port: 40001,

            fldigi_enabled: false,
            fldigi_host: "127.0.0.1".to_string(),
            fldigi_port: 7362,

            psk_reporter_enabled: false,

            lan_sync_port: 7373,
            lan_sync_auto_start: false,
            lan_sync_server_ip: "127.0.0.1".to_string(),

            lotw_tqsl_path: "C:\\Program Files (x86)\\Trusted QSL\\tqsl.exe".to_string(),
            lotw_station_name: String::new(),
            lotw_username: String::new(),
            lotw_password: String::new(),

            qrz_username: String::new(),
            qrz_password: String::new(),
            qrz_api_key: String::new(),
            qrz_auto_lookup: true,

            eqsl_username: String::new(),
            eqsl_password: String::new(),

            clublog_callsign: String::new(),
            clublog_email: String::new(),
            clublog_password: String::new(),
            clublog_api_key: String::new(),

            cloudlog_url: String::new(),
            cloudlog_api_key: String::new(),
            hrdlog_username: String::new(),
            hrdlog_upload_code: String::new(),
            hamqth_username: String::new(),
            hamqth_password: String::new(),

            wol_mac: String::new(),
            wol_ip: "255.255.255.255".to_string(),
            wol_port: 9,

            current_language: "pl".to_string(),
            compact_hud_mode: false,

            live_auto_upload_clublog: false,
            live_auto_upload_qrz: false,

            cluster_host: "cluster.sp7pka.ampr.org".to_string(),
            cluster_port: 8000,
            cluster_callsign: "".to_string(),
            cluster_auto_connect: false,
            cluster_filter_current_band: false,
            cluster_hide_ft8: false,
            cluster_hide_skimmers: false,
            custom_clusters: Vec::new(),
            quick_access: QuickAccessConfig::default(),

            left_column_width: 350.0,
            right_column_width: 360.0,

            // Kolumna 0 (Lewa): VFO -> QSO Entry -> Band Map
            panel_vfo:     ViewPanelConfig { visible: true, floating: false, column: 0, order: 0, saved_pos: None, saved_size: None },
            panel_qso:     ViewPanelConfig { visible: true, floating: false, column: 0, order: 1, saved_pos: None, saved_size: None },
            panel_bandmap: ViewPanelConfig { visible: true, floating: false, column: 0, order: 2, saved_pos: None, saved_size: None },

            // Kolumna 1 (Srodek): Tabela Dziennika -> DX Cluster
            panel_log:     ViewPanelConfig { visible: true, floating: false, column: 1, order: 0, saved_pos: None, saved_size: None },
            panel_cluster: ViewPanelConfig { visible: true, floating: false, column: 1, order: 1, saved_pos: None, saved_size: None },

            // Kolumna 2 (Prawa): Mapa Swiata -> Pogoda Solarna -> Satelity
            panel_world_map:  ViewPanelConfig { visible: true, floating: false, column: 2, order: 0, saved_pos: None, saved_size: None },
            panel_solar:      ViewPanelConfig { visible: true, floating: false, column: 2, order: 1, saved_pos: None, saved_size: None },
            panel_satellites: ViewPanelConfig { visible: true, floating: false, column: 2, order: 2, saved_pos: None, saved_size: None },
            logbook_columns: default_logbook_columns(),
            custom_contests: vec![],
        }
    }
}

impl AppConfig {
    pub fn load_from_file(path: &std::path::Path) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                return cfg;
            }
        }
        AppConfig::default()
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(path, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_file = temp_dir.path().join("station_config.json");

        let mut cfg = AppConfig::default();
        cfg.is_configured = true;
        cfg.station.callsign = "SP6INA".to_string();
        cfg.station.operator = "Mariusz Woźniak".to_string();
        cfg.dark_theme = true;

        cfg.save_to_file(&config_file).unwrap();

        let loaded = AppConfig::load_from_file(&config_file);
        assert!(loaded.is_configured);
        assert_eq!(loaded.station.callsign, "SP6INA");
        assert_eq!(loaded.station.operator, "Mariusz Woźniak");
        assert_eq!(loaded.equipment.len(), 0);
        assert!(loaded.dark_theme);
    }

    #[test]
    fn test_station_profiles_and_cat_sharing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_file = temp_dir.path().join("station_config_profiles.json");

        let mut cfg = AppConfig::default();
        cfg.cat_sharing_enabled = true;
        cfg.cat_sharing_port = 4534;

        let mut p1 = StationProfile::default();
        p1.id = "p1".to_string();
        p1.name = "Home QTH".to_string();
        p1.callsign = "SP6INA".to_string();

        let mut p2 = StationProfile::default();
        p2.id = "p2".to_string();
        p2.name = "Portable SOTA".to_string();
        p2.callsign = "SP6INA/P".to_string();
        p2.sota_ref = Some("SP/BZ-001".to_string());

        cfg.station_profiles = vec![p1, p2];
        cfg.active_profile_id = "p2".to_string();

        cfg.save_to_file(&config_file).unwrap();

        let loaded = AppConfig::load_from_file(&config_file);
        assert!(loaded.cat_sharing_enabled);
        assert_eq!(loaded.cat_sharing_port, 4534);
        assert_eq!(loaded.station_profiles.len(), 2);
        assert_eq!(loaded.active_profile_id, "p2");
        assert_eq!(loaded.station_profiles[1].callsign, "SP6INA/P");
        assert_eq!(loaded.station_profiles[1].sota_ref.as_deref(), Some("SP/BZ-001"));
    }
}


