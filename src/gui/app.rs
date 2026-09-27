// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::hamlib::RigState;
use crate::cat::rotor::RotorState;
use crate::cloud::solar::SpaceWeather;
use crate::cluster::telnet::DxSpot;
use crate::core::awards::{AwardsEngine, PolishDistrictInfo, QsoAwardStatus};
use crate::core::database::{Journal, LogDatabase};
use crate::core::geo::{calculate_bearing_deg, calculate_distance_km, locator_to_coordinates};
use crate::core::i18n::{tr, Language};
use crate::core::prefix::{PrefixInfo, PrefixMatcher};
use crate::core::qso::QsoRecord;
use crate::core::scp::ScpEngine;
use crate::core::station::{AppConfig, EquipmentCategory, EquipmentItem, StationProfile, ViewPanelConfig, VoiceKeyerMessage, WorkspaceProfile};
use crate::cluster::telnet::ClusterEvent;
use crate::gui::awards_matrix::render_awards_matrix_window;
use crate::gui::bandmap::render_bandmap_window;
use crate::gui::cat_settings::render_cat_settings_window;
use crate::gui::cluster_panel::render_cluster_window;
use crate::gui::contest::{render_contest_window, render_custom_contest_editor};
use crate::gui::cw_macros::render_cw_macros_window;
use crate::gui::logbook_table::{render_edit_qso_dialog, render_logbook_window, render_column_settings};
use crate::gui::menu::{render_main_toolbar, render_menu_bar};
use crate::gui::online_sync::render_online_sync_window;
use crate::gui::qso_entry::render_qso_entry_window;
use crate::gui::satellites::render_satellites_window;
use crate::gui::solar_panel::render_solar_window;
use crate::gui::station_ledger::render_station_ledger_window;
use crate::gui::statistics::render_statistics_window;
use crate::gui::vfo_panel::render_vfo_window;
use crate::gui::voice_keyer::render_voice_keyer_window;
use crate::gui::welcome_wizard::render_welcome_wizard;
use crate::gui::world_map::render_world_map_window;
use crate::core::service_db::{QslManagerRecord, ServiceDatabase};
use crate::gui::icons;
use crate::gui::astronomy_dialog::AstronomyDialog;
use crate::gui::iota_browser::IotaBrowserDialog;
use crate::gui::photo_viewer::PhotoViewerDialog;
use crate::gui::prefix_manager::PrefixManagerDialog;
use crate::gui::qsl_manager::QslManagerDialog;
use crate::gui::send_spot::SendSpotDialog;
use crate::gui::sota_dialog::SotaDialog;
use crate::gui::states_browser::StatesBrowserDialog;
use crate::gui::wol_dialog::WolDialog;
use crate::core::backup::BackupManager;
use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use std::sync::{Arc, Mutex};

/// Główny stan aplikacji natywnego pulpitu SPLogbook
pub struct SpLogApp {
    // Podsystemy bazodanowe i analityczne
    pub log_db: Arc<Mutex<LogDatabase>>,
    pub prefix_matcher: Arc<PrefixMatcher>,
    pub scp_engine: Arc<Mutex<ScpEngine>>,
    pub awards_engine: Arc<Mutex<AwardsEngine>>,

    // Konfiguracja stacji i lokalizacja
    pub my_station: StationProfile,
    pub current_language: Language,
    pub status_message: Option<String>,

    // Stany formularza wprowadzania QSO
    pub entry_callsign: String,
    pub entry_band: String,
    pub entry_mode: String,
    pub entry_rst_sent: String,
    pub entry_rst_rcvd: String,
    pub entry_name: String,
    pub entry_qth: String,
    pub entry_grid: String,
    pub entry_pga: String,
    pub entry_comment: String,

    // Dynamicznie wyliczane dane dla wpisanego znaku
    pub scp_suggestions: Vec<String>,
    /// Lokalne, rozmyte poprawki znaku (fuzzy matching) — `(znak, odległość)`.
    pub callsign_corrections: Vec<(String, usize)>,
    pub active_prefix_info: Option<PrefixInfo>,
    pub active_polish_district: Option<PolishDistrictInfo>,
    pub active_award_status: Option<QsoAwardStatus>,
    pub active_distance_km: f64,
    pub active_bearing_deg: f64,
    pub active_propagation: Option<crate::core::propagation::PropagationForecast>,
    pub active_clubs: Vec<crate::core::clubs::ClubAffiliation>,

    // Stan transceivera CAT, S-Metra i rotora
    pub rig_state: RigState,
    pub rotor_state: RotorState,
    pub cat_connected: bool,
    pub vfo_split: bool,

    // Tabela ostatnich łączności i wyszukiwarka
    pub recent_qsos: Vec<QsoRecord>,
    pub qso_numbers: std::collections::HashMap<i64, usize>,
    pub log_search_query: String,

    // DX Cluster i pogoda kosmiczna
    pub cluster_spots: Vec<DxSpot>,
    pub cluster_spots_api: Option<Arc<Mutex<Vec<DxSpot>>>>,  // wspoldzielone z REST API
    /// Cache kolorów/odznak spotów klastra (unika przeliczania prefiksu i statusu
    /// nagród w każdej klatce). Czyszczony przy zmianie danych nagród.
    pub cluster_badge_cache: std::collections::HashMap<String, (egui::Color32, &'static str)>,
    pub space_weather: SpaceWeather,
    /// Cache prognoz propagacyjnych panelu słonecznego (VOACAP-lite).
    pub solar_propagation_cache: Option<(
        (u32, u32, u32, u32, u32),
        Vec<(&'static str, crate::core::propagation::PropagationForecast)>,
    )>,

    // WSPR Monitor
    pub wspr_spots: Vec<crate::cloud::wspr::WsprSpot>,
    pub wspr_loading: bool,
    pub wspr_last_error: Option<String>,
    pub wspr_fetch_slot: Option<std::sync::Arc<std::sync::Mutex<Option<Result<Vec<crate::cloud::wspr::WsprSpot>, String>>>>>,

    // Moduł Satelitów
    pub show_satellites_window: bool,
    pub selected_satellite: String,
    pub sat_azimuth: f32,
    pub sat_elevation: f32,
    pub sat_range_km: f32,
    pub sat_altitude_km: f32,
    pub sat_downlink_mhz: f64,
    pub sat_uplink_mhz: f64,
    pub sat_rx_doppler_khz: f32,
    pub sat_tx_doppler_khz: f32,
    pub sat_auto_track_rotator: bool,
    pub sat_auto_tune_radio: bool,
    pub sat_passes: Vec<crate::core::satellite::SatellitePass>,
    pub sat_passes_key: String,

    // Historia warunków solarnych (dla wykresu + alertu)
    pub solar_history: Vec<SpaceWeather>,
    pub solar_loading: bool,
    pub solar_fetch_slot: Option<std::sync::Arc<std::sync::Mutex<Option<Result<SpaceWeather, String>>>>>,
    pub solar_last_alert: Option<String>,

    // Jednostki odległości w monitorze WSPR
    pub wspr_distance_miles: bool,

    // Moduł Zawodów
    pub show_contest_window: bool,
    pub contest_name: String,
    pub contest_stx: u32,
    pub contest_qsos: u32,
    pub contest_points: u32,
    pub contest_mults: u32,
    pub show_custom_contest_editor: bool,
    pub custom_contest_edit_idx: Option<usize>,
    pub custom_contest_draft: crate::core::station::CustomContest,
    pub custom_contests: Vec<crate::core::station::CustomContest>,

    // Makra Telegraficzne CW
    pub show_cw_window: bool,
    pub cw_wpm: u32,
    pub cw_macro_f1: String,
    pub cw_macro_f2: String,
    pub cw_macro_f3: String,
    pub cw_macro_f4: String,
    pub cw_macro_f5: String,
    pub cw_macro_f6: String,
    pub cw_macro_f7: String,
    pub cw_macro_f8: String,

    // Księga sprzętowa i CRUD
    pub show_ledger_window: bool,
    pub equipment_items: Vec<EquipmentItem>,
    pub new_eq_model: String,
    pub editing_equipment_id: Option<String>,
    pub edit_eq_cat: EquipmentCategory,
    pub edit_eq_mfr: String,
    pub edit_eq_model: String,
    pub edit_eq_sn: String,
    pub edit_eq_date: String,
    pub edit_eq_notes: String,
    pub new_eq_cat: EquipmentCategory,
    pub new_eq_mfr: String,
    pub new_eq_sn: String,
    pub new_eq_notes: String,

    // Motyw i konfiguracja stacji
    pub dark_theme: bool,
    pub theme_preset: crate::gui::theme::ThemePreset,
    pub font_scale: f32,
    pub font_family: String,
    pub distance_unit: String,
    pub show_welcome_wizard: bool,
    pub wizard_tab: u8,
    pub config_file_path: std::path::PathBuf,
    pub secret_store_id: String,
    pub active_db_path: std::path::PathBuf,

    // Wizualna Panorama Pasma (Band Map)
    pub show_bandmap_window: bool,
    pub bandmap_selected_band: String,
    pub bandmap_auto_track: bool,

    // Okna dialogowe
    pub show_about_window: bool,
    pub show_shortcuts_window: bool,
    pub show_legend_window: bool,
    pub show_command_palette: bool,
    pub command_palette_query: String,
    pub command_palette_selected: usize,
    pub show_changelog_window: bool,
    pub show_update_window: bool,
    pub show_user_manual: bool,
    pub manual_section: Option<String>,
    pub manual_selected: usize,
    pub update_check_status: Option<String>,
    pub update_check_rx: Option<std::sync::mpsc::Receiver<crate::cloud::updater::UpdateCheckOutcome>>,
    pub update_available: Option<crate::cloud::updater::LatestRelease>,
    pub update_install_status: Option<String>,
    pub update_install_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub pending_restart: bool,
    pub show_vfo_panel: bool,
    pub show_cluster_panel: bool,
    pub show_solar_panel: bool,

    // CAT / Hamlib konfiguracja
    pub cat_host: String,
    pub cat_port: u16,
    pub cat_poll_rate_ms: u64,
    pub cat_rig_model: String,
    pub cat_serial_port: String,
    pub cat_baud_rate: u32,
    pub cat_test_result: Option<String>,
    pub show_cat_settings_window: bool,

    // Rotor konfiguracja
    pub rotor_host: String,
    pub rotor_port: u16,
    pub rotor_test_result: Option<String>,

    // TCI konfiguracja
    pub cat_backend: String,
    pub tci_host: String,
    pub tci_port: u16,
    pub tci_test_result: Option<String>,

    // FLDigi konfiguracja
    pub fldigi_enabled: bool,
    pub fldigi_host: String,
    pub fldigi_port: u16,
    pub fldigi_test_result: Option<String>,

    // PSK Reporter
    pub psk_reporter_enabled: bool,

    // N1MM Logger+ UDP broadcast
    pub n1mm_broadcast_enabled: bool,
    pub n1mm_broadcast_host: String,
    pub n1mm_broadcast_port: u16,

    // Voice keyer (SSB)
    pub voice_keyer_messages: Vec<VoiceKeyerMessage>,
    pub voice_keyer_active: Option<usize>,
    pub voice_keyer_recording: Option<usize>,
    pub show_voice_keyer_window: bool,

    // Profile układu operatorskiego (workspace)
    pub workspace_profiles: Vec<WorkspaceProfile>,
    pub show_workspace_profiles_window: bool,

    // Multi-Op LAN
    pub lan_sync_port: u16,
    pub lan_sync_auto_start: bool,
    pub lan_sync_server_ip: String,
    pub lan_sync_secret: String,
    pub show_multi_op_window: bool,
    pub multi_op_server: Option<std::sync::Arc<crate::cluster::lan_sync::MultiOpServer>>,
    pub multi_op_incoming_rx: Option<tokio::sync::mpsc::UnboundedReceiver<crate::core::qso::QsoRecord>>,
    pub multi_op_is_server: bool,
    pub multi_op_status: String,
    pub multi_op_connected_count: usize,
    pub multi_op_log: Vec<String>,

    // LoTW konfiguracja
    pub lotw_tqsl_path: String,
    pub lotw_station_name: String,
    pub lotw_username: String,
    pub lotw_password: String,

    // QRZ / Callbook
    pub qrz_username: String,
    pub qrz_password: String,
    pub qrz_api_key: String,
    pub qrz_auto_lookup: bool,

    // eQSL & Club Log
    pub eqsl_username: String,
    pub eqsl_password: String,
    pub clublog_callsign: String,
    pub clublog_email: String,
    pub clublog_password: String,
    pub clublog_api_key: String,

    // Synchronizacja Online
    pub show_online_sync_window: bool,
    pub online_sync_logs: Vec<String>,

    // Edycja QSO i historia Worked Before
    pub editing_qso: Option<QsoRecord>,
    pub past_qsos_for_active_call: Vec<QsoRecord>,

    // Kanał asynchroniczny QRZ
    pub qrz_lookup_tx: std::sync::mpsc::Sender<crate::cloud::qrz::CallbookData>,
    pub qrz_lookup_rx: std::sync::mpsc::Receiver<crate::cloud::qrz::CallbookData>,

    // Okna modułów
    pub show_world_map_window: bool,
    pub show_awards_matrix_window: bool,
    pub awards_matrix_tab: usize,
    pub awards_other_subtab: usize,
    pub awards_other_search: String,
    pub local_callbook: std::sync::Arc<crate::core::callbook::LocalCallbook>,
    pub quick_access: crate::core::station::QuickAccessConfig,
    pub show_quick_access_customizer: bool,

    // Hamlib i rigctld
    pub cat_rig_id: u32,
    pub cat_model_search: String,
    pub cat_auto_start_rigctld: bool,
    pub cat_hamlib_source: String,
    pub cat_custom_rigctld_path: String,
    pub rigctld_supervisor: Option<crate::cat::supervisor::RigctldSupervisor>,

    // Wielodziennikowość i dialogi pomocnicze
    pub journal_dialog: crate::gui::journal_manager::JournalManagerDialog,
    pub advanced_filter_dialog: crate::gui::advanced_filter::AdvancedFilterDialog,
    pub cw_terminal_dialog: crate::gui::cw_terminal::CwTerminalDialog,
    pub qsl_designer_dialog: crate::gui::qsl_designer::QslDesignerDialog,
    pub active_journal: crate::core::database::Journal,

    // WSJT-X nasłuch UDP w tle
    pub wsjtx_tx: std::sync::mpsc::Sender<crate::digital::wsjtx::WsjtxMessage>,
    pub wsjtx_rx: std::sync::mpsc::Receiver<crate::digital::wsjtx::WsjtxMessage>,
    pub wsjtx_packets_count: u64,
    pub wsjtx_last_call: Option<String>,

    // JS8Call integracja przez TCP API (port 2237)
    pub js8call_enabled: bool,
    pub js8call_host: String,
    pub js8call_port: u16,
    pub js8call_state: crate::digital::js8call::Js8CallState,
    pub js8call_state_rx: Option<std::sync::mpsc::Receiver<crate::digital::js8call::Js8CallState>>,
    pub js8call_qso_rx: Option<std::sync::mpsc::Receiver<crate::core::qso::QsoRecord>>,

    // Alerty, Live Auto-Upload i Toast
    pub live_auto_upload_clublog: bool,
    pub live_auto_upload_qrz: bool,
    pub upload_scheduler: Arc<Mutex<crate::cloud::scheduler::UploadScheduler>>,
    pub status_toast: Option<(String, std::time::Instant)>,
    pub sync_log_tx: std::sync::mpsc::Sender<(String, bool)>,
    pub sync_log_rx: std::sync::mpsc::Receiver<(String, bool)>,

    // SPLogbook
    pub service_db: Arc<ServiceDatabase>,
    pub send_spot_dialog: SendSpotDialog,
    pub show_column_settings: bool,
    pub logbook_columns: Vec<crate::core::station::LogColumn>,
    pub logbook_column_presets: Vec<crate::core::station::ColumnPreset>,
    pub column_preset_name: String,
    pub iota_dialog: IotaBrowserDialog,
    pub states_dialog: StatesBrowserDialog,
    pub qsl_manager_dialog: QslManagerDialog,
    pub photo_viewer_dialog: PhotoViewerDialog,
    pub prefix_manager_dialog: PrefixManagerDialog,
    pub astronomy_dialog: AstronomyDialog,
    pub wol_dialog: WolDialog,
    pub sota_dialog: SotaDialog,

    // CAT Channels
    pub cat_state_tx: std::sync::mpsc::Sender<crate::cat::hamlib::RigState>,
    pub cat_state_rx: std::sync::mpsc::Receiver<crate::cat::hamlib::RigState>,

    /// Centralna magistrala zdarzeń aplikacji (QSO, spoty, CAT, chmura).
    pub event_bus: crate::core::events::EventBus,

    // DX Cluster Telnet & Filtry
    pub cluster_host: String,
    pub cluster_port: u16,
    pub cluster_callsign: String,
    pub cluster_auto_connect: bool,
    pub cluster_connected: bool,
    pub cluster_connecting: bool,
    pub cluster_status_text: String,
    pub cluster_filter_current_band: bool,
    pub cluster_hide_ft8: bool,
    pub cluster_hide_skimmers: bool,
    pub custom_clusters: Vec<crate::core::station::CustomClusterServer>,
    pub show_add_cluster_dialog: bool,
    pub new_cluster_name: String,
    pub new_cluster_host: String,
    pub new_cluster_port: u16,
    pub cluster_event_rx: Option<std::sync::mpsc::Receiver<ClusterEvent>>,
    pub cluster_stop_tx: Option<tokio::sync::watch::Sender<bool>>,

    // Modułowy układ kafelków i okien pływających
    pub reset_layout_requested: bool,
    pub left_column_width: f32,
    pub right_column_width: f32,
    pub dragging_tile: Option<String>,
    pub panel_vfo: ViewPanelConfig,
    pub panel_qso: ViewPanelConfig,
    pub panel_log: ViewPanelConfig,
    pub panel_cluster: ViewPanelConfig,
    pub panel_solar: ViewPanelConfig,
    pub panel_bandmap: ViewPanelConfig,
    pub panel_satellites: ViewPanelConfig,
    pub panel_world_map: ViewPanelConfig,

    /// Tryb zakładek w kolumnach dokowanego układu (zapis w konfiguracji).
    pub tabbed_columns: bool,
    /// Indeks aktywnej zakładki w każdej z trzech kolumn (stan sesji).
    pub active_tab: [usize; 3],

    /// Rzeczywisty układ dokowania paneli (egui_dock): podziały, grupy
    /// zakładek i aktywne karty. Uzupełniany/synchronizowany z konfiguracją
    /// paneli przy każdej klatce.
    pub dock_state: DockState<String>,
    /// Ostatnio zapisany (lub zbudowany) układ dokowania — używany do
    /// wykrywania zmian i unikania zbędnych zapisów konfiguracji.
    pub last_saved_dock_layout: Option<serde_json::Value>,

    // Tryb kompaktowy (Mini HUD)
    pub compact_hud_mode: bool,
    pub hud_always_on_top: bool,
    pub hud_saved_pos: Option<[f32; 2]>,
    pub hud_saved_size: Option<[f32; 2]>,
    pub hud_operating_bar: bool,

    // Geometria okna głównego — odczytana z ekranu w trakcie działania
    // (zapisywana przy wyjściu w AppConfig::main_window_*).
    pub main_window_pos: Option<[f32; 2]>,
    pub main_window_size: Option<[f32; 2]>,
    pub main_window_maximized: bool,

    // Pola dodatkowe formularza QSO
    pub entry_iota: String,
    pub entry_state: String,
    pub entry_sota: String,
    pub entry_pota: String,
    pub entry_qsl_manager: String,
    pub active_qsl_manager_record: Option<QslManagerRecord>,

    // Cloudlog, HRDLog, HamQTH
    pub cloudlog_url: String,
    pub cloudlog_api_key: String,
    pub hrdlog_username: String,
    pub hrdlog_upload_code: String,
    pub hamqth_username: String,
    pub hamqth_password: String,

    // Sortowanie i paginacja tabeli logu
    pub log_sort_column: u8,      // 0=data, 1=znak, 2=pasmo, 3=emisja, 4=kraj, 5..=13 pozostałe kolumny
    pub log_sort_asc: bool,
    pub log_page: usize,
    pub log_page_size: usize,     // 25, 50, 100, 0=wszystkie
    pub selected_qso_ids: Vec<i64>, // Zaznaczenie wielokrotne w logbooku (bulk delete)
    pub confirm_bulk_delete: bool,

    // Moduł statystyk
    pub show_statistics_window: bool,
    pub stats_tab: usize,

    // Bufor Undo/Redo (ostatnie 20 operacji na logu)
    pub undo_stack: std::collections::VecDeque<crate::core::qso::QsoRecord>,
    pub redo_stack: std::collections::VecDeque<crate::core::qso::QsoRecord>,

    // LoTW Users lookup cache (callsign -> uses_lotw)
    pub lotw_users_cache: std::collections::HashMap<String, bool>,

    // PSK Reporter / WSPR monitoring
    pub show_pskreporter_window: bool,
    pub show_wspr_window: bool,

    // REST API server flag
    pub rest_api_enabled: bool,
    pub rest_api_port: u16,
    pub rest_api_key: String,

    // Band opening alerts (K-index threshold)
    pub band_alert_k_index_threshold: u8,
    pub band_alert_enabled: bool,

    // System pluginów użytkownika (Rhai)
    pub plugins_enabled: bool,
    pub plugins_dir: String,
    pub show_plugin_manager: bool,
    pub plugin_engine: crate::plugins::PluginEngine,
    pub plugin_log: Vec<String>,
    pub plugin_notifications: Vec<String>,

    // Zintegrowany pasek asystenta operatora
    pub operator_assistant_enabled: bool,
    pub show_operator_assistant: bool,
    pub show_propagation_explain: bool,

    // Historia i weryfikacja dokładności prognoz propagacyjnych
    pub propagation_history: crate::core::propagation_history::PropagationHistory,
    /// Poprzednie statusy otwarcia pasm (do wykrywania przejść otwarcia/zamknięcia).
    pub previous_band_status:
        std::collections::HashMap<String, crate::core::propagation::BandOpeningStatus>,

    // PTT control
    pub ptt_active: bool,

    // Dwuetapowy CAT i udostępnianie (CAT Sharing dla WSJT-X / JTDX / FLDigi)
    pub cat_mfg_selected: String,
    pub cat_conn_type: String,
    pub cat_sharing_enabled: bool,
    pub cat_sharing_port: u16,
    pub cat_sharing_active: bool,
    pub cat_shared_state: std::sync::Arc<std::sync::RwLock<crate::cat::hamlib::RigState>>,
    pub cat_proxy_server: Option<std::sync::Arc<crate::cat::server::HamlibProxyServer>>,
    pub cat_proxy_rx: Option<tokio::sync::broadcast::Receiver<crate::cat::server::RigServerCommand>>,

    // VFO Konsola radiowa & DSP
    pub vfo_split_offset_khz: f64,
    pub vfo_filter_preset: String,
    pub vfo_preamp_att: String,
    pub vfo_nb_nr: bool,
    pub vfo_agc_speed: String,

    // Kompas antenowy i wiązka na mapie (Beam Lobe)
    pub map_show_beam_lobe: bool,
    pub map_show_compass: bool,
    pub map_beamwidth_deg: f32,

    // Boczny panel filtrów klastra i POTA/SOTA
    pub cluster_show_filter_sidebar: bool,
    pub cluster_search_query: String,
    pub cluster_filter_band_selection: String,
    pub cluster_filter_mode_selection: String,
    pub cluster_filter_status: String,
    pub cluster_filter_source: String,
    pub cluster_filter_pota_sota_only: bool,

    // Wyszukiwanie i usuwanie duplikatów w logu
    pub show_find_duplicates_window: bool,
    pub duplicates_match_date: bool,
    pub duplicates_groups: Vec<Vec<crate::core::qso::QsoRecord>>,
    pub duplicates_selected_ids: std::collections::HashSet<i64>,
    pub duplicates_status: Option<String>,

    // Profile stacji roboczej (Wieloprofilowość)
    pub show_station_profiles_window: bool,
    pub station_profiles: Vec<crate::core::station::StationProfile>,
    pub active_profile_id: String,

    pub focus_callsign_requested: bool,

    // Debounce automatycznego pobierania danych z Callbook/QRZ po wpisaniu znaku
    pub lookup_debounce_until: Option<std::time::Instant>,
}

/// Zwraca ścieżkę do pliku czcionki dla wybranej rodziny (o ile istnieje w systemie).
fn resolve_font_path(family: &str) -> Option<std::path::PathBuf> {
    let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let win_fonts = std::path::Path::new(&win_dir).join("Fonts");
    let candidates: Vec<std::path::PathBuf> = match family {
        "Segoe UI" => vec![win_fonts.join("segoeui.ttf")],
        "Arial" => vec![win_fonts.join("arial.ttf")],
        "Consolas" => vec![win_fonts.join("consola.ttf")],
        "DejaVu Sans" => vec![
            std::path::PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"),
        ],
        "Noto Sans" => vec![
            std::path::PathBuf::from("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/noto/NotoSans-Regular.ttf"),
        ],
        _ => return None,
    };
    candidates.into_iter().find(|p| p.exists())
}

impl SpLogApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        log_db: Arc<Mutex<LogDatabase>>,
        prefix_matcher: Arc<PrefixMatcher>,
        scp_engine: Arc<Mutex<ScpEngine>>,
        app_config: AppConfig,
        config_file_path: std::path::PathBuf,
        active_db_path: std::path::PathBuf,
    ) -> Self {
        // Rejestracja czcionki symboli systemowych (seguisym na Windows, DejaVu/Noto na Linux)
        // dla pełnej obsługi znaków Unicode, symboli radiowych, strzałek ▲/▼, planet, satelitów i statusów
        let mut font_defs = egui::FontDefinitions::default();
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let font_candidates = [
            std::path::Path::new(&win_dir).join("Fonts").join("seguisym.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/truetype/freefont/FreeSans.ttf"),
        ];

        for path in &font_candidates {
            if let Ok(bytes) = std::fs::read(path) {
                font_defs.font_data.insert("symbol_font".to_owned(), egui::FontData::from_owned(bytes).into());
                if let Some(prop) = font_defs.families.get_mut(&egui::FontFamily::Proportional) {
                    prop.push("symbol_font".to_owned());
                }
                if let Some(mono) = font_defs.families.get_mut(&egui::FontFamily::Monospace) {
                    mono.push("symbol_font".to_owned());
                }
                break;
            }
        }

        // Wybrana przez użytkownika rodzina czcionek (zmiana wymaga ponownego uruchomienia).
        if !app_config.font_family.is_empty() {
            if let Some(path) = resolve_font_path(&app_config.font_family) {
                if let Ok(bytes) = std::fs::read(&path) {
                    font_defs.font_data.insert("custom_ui_font".to_owned(), egui::FontData::from_owned(bytes).into());
                    if let Some(prop) = font_defs.families.get_mut(&egui::FontFamily::Proportional) {
                        prop.insert(0, "custom_ui_font".to_owned());
                    }
                    let mono_family = app_config.font_family == "Consolas";
                    if mono_family {
                        if let Some(mono) = font_defs.families.get_mut(&egui::FontFamily::Monospace) {
                            mono.insert(0, "custom_ui_font".to_owned());
                        }
                    }
                }
            }
        }

        _cc.egui_ctx.set_fonts(font_defs);

        let (active_journal, recent_qsos, qso_numbers) = {
            let db = log_db.lock().unwrap_or_else(|p| p.into_inner());
            let j = match db.get_active_journal() {
                Ok(j) => j,
                Err(e) => {
                    log::error!("Nie udało się pobrać aktywnego dziennika: {e}");
                    Journal::default()
                }
            };
            let qsos = db.get_recent_qsos_for_journal(&j.id, 100).unwrap_or_default();
            let numbers = db.qso_numbers_for_journal(&j.id).unwrap_or_else(|error| {
                log::error!("Nie udało się wyliczyć numerów QSO: {error}");
                std::collections::HashMap::new()
            });
            (j, qsos, numbers)
        };

        let (cat_state_tx, cat_state_rx) = std::sync::mpsc::channel();
        let cat_sender_init = cat_state_tx.clone();
        if app_config.cat_enabled {
            let host = app_config.cat_host.clone();
            let port = app_config.cat_port;
            let poll_rate = app_config.cat_poll_rate_ms;
            tokio::spawn(async move {
                let (client, mut rx) = crate::cat::hamlib::HamlibClient::new(&host, port);
                tokio::spawn(async move {
                    client.run_poll_loop(poll_rate).await;
                });
                while let Ok(st) = rx.recv().await {
                    let _ = cat_sender_init.send(st);
                }
            });
        }

        let (wsjtx_tx, wsjtx_rx) = std::sync::mpsc::channel();
        let ws_tx = wsjtx_tx.clone();
        tokio::spawn(async move {
            let receiver = crate::digital::wsjtx::WsjtxReceiver::new(2237);
            let (tokio_tx, mut tokio_rx) = tokio::sync::mpsc::channel(100);
            tokio::spawn(async move {
                let _ = receiver.run(tokio_tx).await;
            });
            while let Some(msg) = tokio_rx.recv().await {
                let _ = ws_tx.send(msg);
            }
        });

        let sample_spots = Vec::new();
        let equipment_items = app_config.equipment;

        let rig = RigState {
            frequency_hz: 14_025_000,
            mode: "CW".to_string(),
            passband_hz: 500,
            s_meter_dbm: 20.0,
            s_meter_unit: "S9+20dB".to_string(),
            rf_power_watts: 100.0,
            ..Default::default()
        };

        let rotor = RotorState {
            azimuth_deg: 275.0,
            ..Default::default()
        };

        let show_wizard = !app_config.is_configured;
        let (qrz_lookup_tx, qrz_lookup_rx) = std::sync::mpsc::channel();
        let (sync_log_tx, sync_log_rx) = std::sync::mpsc::channel();

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| {
                std::env::var("APPDATA")
                    .map(|p| std::path::PathBuf::from(p).join("SPLogbook"))
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
            });

        let cb_candidates = [
            std::path::PathBuf::from("databases/callbook.db"),
            exe_dir.join("databases/callbook.db"),
            exe_dir.join("../databases/callbook.db"),
            exe_dir.join("../../databases/callbook.db"),
        ];
        let cb_path = cb_candidates.into_iter().find(|p| p.exists());

        let srv_candidates = [
            std::path::PathBuf::from("databases/serviceLOG.db"),
            exe_dir.join("databases/serviceLOG.db"),
            exe_dir.join("../databases/serviceLOG.db"),
            exe_dir.join("../../databases/serviceLOG.db"),
        ];
        let srv_path = srv_candidates.into_iter().find(|p| p.exists());

        let local_callbook = std::sync::Arc::new(crate::core::callbook::LocalCallbook::new(cb_path, srv_path));

        let mut app = Self {
            log_db,
            prefix_matcher,
            scp_engine,
            awards_engine: Arc::new(Mutex::new(AwardsEngine::new())),
            my_station: app_config.station.clone(),
            // Wczytanie zapisanego języka z konfiguracji stacji
            current_language: Language::from_code(&app_config.current_language),
            status_message: None,

            entry_callsign: String::new(),
            callsign_corrections: Vec::new(),
            entry_band: "20m".to_string(),
            entry_mode: "CW".to_string(),
            entry_rst_sent: "599".to_string(),
            entry_rst_rcvd: "599".to_string(),
            entry_name: String::new(),
            entry_qth: String::new(),
            entry_grid: String::new(),
            entry_pga: String::new(),
            entry_comment: String::new(),

            scp_suggestions: Vec::new(),
            active_prefix_info: None,
            active_polish_district: None,
            active_award_status: None,
            active_distance_km: 0.0,
            active_bearing_deg: 0.0,
            active_propagation: None,
            active_clubs: Vec::new(),

            rig_state: rig,
            rotor_state: rotor,
            cat_connected: true,
            vfo_split: false,

            recent_qsos,
            qso_numbers,
            log_search_query: String::new(),

            cluster_spots: sample_spots,
            cluster_spots_api: None,
            cluster_badge_cache: std::collections::HashMap::new(),
            space_weather: SpaceWeather::default(),
            solar_propagation_cache: None,

            wspr_spots: Vec::new(),
            wspr_loading: false,
            wspr_last_error: None,
            wspr_fetch_slot: None,

            show_satellites_window: false,
            selected_satellite: "ISS (ZARYA)".to_string(),
            sat_azimuth: 142.5,
            sat_elevation: 38.2,
            sat_range_km: 685.0,
            sat_altitude_km: 418.0,
            sat_downlink_mhz: 145.8000,
            sat_uplink_mhz: 437.8000,
            sat_rx_doppler_khz: -2.85,
            sat_tx_doppler_khz: 8.52,
            sat_auto_track_rotator: true,
            sat_auto_tune_radio: true,
            sat_passes: Vec::new(),
            sat_passes_key: String::new(),

            solar_history: vec![SpaceWeather::default()],
            solar_loading: false,
            solar_fetch_slot: None,
            solar_last_alert: None,

            wspr_distance_miles: false,

            show_contest_window: false,
            contest_name: "SP DX Contest".to_string(),
            contest_stx: 1,
            contest_qsos: 0,
            contest_points: 0,
            contest_mults: 0,
            show_custom_contest_editor: false,
            custom_contest_edit_idx: None,
            custom_contest_draft: Default::default(),
            custom_contests: app_config.custom_contests.clone(),

            show_cw_window: false,
            cw_wpm: 28,
            cw_macro_f1: "CQ TEST %MYCALL% %MYCALL% TEST".to_string(),
            cw_macro_f2: "%HISCALL% 5NN %SERIAL%".to_string(),
            cw_macro_f3: "TU %MYCALL% TEST".to_string(),
            cw_macro_f4: "%MYCALL%".to_string(),
            cw_macro_f5: "%HISCALL%".to_string(),
            cw_macro_f6: "QTH %MYQTH% LOC %MYLOC%".to_string(),
            cw_macro_f7: "%SERIAL%".to_string(),
            cw_macro_f8: "?".to_string(),

            show_ledger_window: false,
            equipment_items,
            new_eq_model: String::new(),
            editing_equipment_id: None,
            edit_eq_cat: EquipmentCategory::Transceiver,
            edit_eq_mfr: String::new(),
            edit_eq_model: String::new(),
            edit_eq_sn: String::new(),
            edit_eq_date: String::new(),
            edit_eq_notes: String::new(),
            new_eq_cat: EquipmentCategory::Transceiver,
            new_eq_mfr: String::new(),
            new_eq_sn: String::new(),
            new_eq_notes: String::new(),

            dark_theme: app_config.dark_theme,
            theme_preset: crate::gui::theme::ThemePreset::from_id(&app_config.theme_preset),
            font_scale: app_config.font_scale,
            font_family: app_config.font_family.clone(),
            distance_unit: app_config.distance_unit.clone(),
            show_welcome_wizard: show_wizard,
            wizard_tab: 0,
            config_file_path,
            secret_store_id: app_config.secret_store_id.clone(),
            active_db_path,

            show_bandmap_window: false,
            bandmap_selected_band: "20m".to_string(),
            bandmap_auto_track: true,

            show_about_window: false,
            show_shortcuts_window: false,
            show_legend_window: false,
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
            show_vfo_panel: true,
            show_cluster_panel: true,
            show_solar_panel: true,

            cat_host: app_config.cat_host,
            cat_port: app_config.cat_port,
            cat_poll_rate_ms: app_config.cat_poll_rate_ms,
            cat_rig_model: app_config.cat_rig_model,
            cat_serial_port: app_config.cat_serial_port,
            cat_baud_rate: app_config.cat_baud_rate,
            cat_test_result: None,
            show_cat_settings_window: false,

            rotor_host: app_config.rotor_host,
            rotor_port: app_config.rotor_port,
            rotor_test_result: None,

            cat_backend: app_config.cat_backend.clone(),
            tci_host: app_config.tci_host,
            tci_port: app_config.tci_port,
            tci_test_result: None,

            fldigi_enabled: app_config.fldigi_enabled,
            fldigi_host: app_config.fldigi_host,
            fldigi_port: app_config.fldigi_port,
            fldigi_test_result: None,

            psk_reporter_enabled: app_config.psk_reporter_enabled,

            n1mm_broadcast_enabled: app_config.n1mm_broadcast_enabled,
            n1mm_broadcast_host: app_config.n1mm_broadcast_host,
            n1mm_broadcast_port: app_config.n1mm_broadcast_port,

            voice_keyer_messages: app_config.voice_keyer_messages,
            voice_keyer_active: None,
            voice_keyer_recording: None,
            show_voice_keyer_window: false,

            workspace_profiles: app_config.workspace_profiles,
            show_workspace_profiles_window: false,

            lan_sync_port: app_config.lan_sync_port,
            lan_sync_auto_start: app_config.lan_sync_auto_start,
            lan_sync_server_ip: app_config.lan_sync_server_ip,
            lan_sync_secret: app_config.lan_sync_secret,
            show_multi_op_window: false,
            multi_op_server: None,
            multi_op_incoming_rx: None,
            multi_op_is_server: true,
            multi_op_status: "Serwer wyłączony".to_string(),
            multi_op_connected_count: 0,
            multi_op_log: Vec::new(),

            lotw_tqsl_path: app_config.lotw_tqsl_path,
            lotw_station_name: app_config.lotw_station_name,
            lotw_username: app_config.lotw_username,
            lotw_password: app_config.lotw_password,

            qrz_username: app_config.qrz_username,
            qrz_password: app_config.qrz_password,
            qrz_api_key: app_config.qrz_api_key,
            qrz_auto_lookup: app_config.qrz_auto_lookup,

            eqsl_username: app_config.eqsl_username,
            eqsl_password: app_config.eqsl_password,

            clublog_callsign: app_config.clublog_callsign,
            clublog_email: app_config.clublog_email,
            clublog_password: app_config.clublog_password,
            clublog_api_key: app_config.clublog_api_key,

            show_online_sync_window: false,
            online_sync_logs: Vec::new(),

            editing_qso: None,
            past_qsos_for_active_call: Vec::new(),

            qrz_lookup_tx,
            qrz_lookup_rx,

            show_world_map_window: false,
            show_awards_matrix_window: false,
            awards_matrix_tab: 0,
            awards_other_subtab: 0,
            awards_other_search: String::new(),
            local_callbook,
            quick_access: app_config.quick_access,
            show_quick_access_customizer: false,

            cat_rig_id: app_config.cat_rig_id,
            cat_model_search: String::new(),
            cat_auto_start_rigctld: app_config.cat_auto_start_rigctld,
            cat_hamlib_source: app_config.cat_hamlib_source,
            cat_custom_rigctld_path: app_config.cat_custom_rigctld_path,
            rigctld_supervisor: None,

            journal_dialog: crate::gui::journal_manager::JournalManagerDialog::default(),
            advanced_filter_dialog: crate::gui::advanced_filter::AdvancedFilterDialog::default(),
            cw_terminal_dialog: crate::gui::cw_terminal::CwTerminalDialog::default(),
            qsl_designer_dialog: crate::gui::qsl_designer::QslDesignerDialog::default(),
            active_journal,

            wsjtx_tx,
            wsjtx_rx,
            wsjtx_packets_count: 0,
            wsjtx_last_call: None,

            // JS8Call — domyślnie wyłączony, bez aktywnych kanałów
            js8call_enabled: false,
            js8call_host: "127.0.0.1".to_string(),
            js8call_port: 2237,
            js8call_state: crate::digital::js8call::Js8CallState::default(),
            js8call_state_rx: None,
            js8call_qso_rx: None,

            live_auto_upload_clublog: app_config.live_auto_upload_clublog,
            live_auto_upload_qrz: app_config.live_auto_upload_qrz,
            upload_scheduler: Arc::new(Mutex::new(crate::cloud::scheduler::UploadScheduler::load_from_disk())),
            status_toast: None,
            sync_log_tx,
            sync_log_rx,

            service_db: Arc::new(ServiceDatabase::open()),
            send_spot_dialog: SendSpotDialog::new(),
            show_column_settings: false,
            show_changelog_window: false,
            show_update_window: false,
            show_user_manual: false,
            manual_section: None,
            manual_selected: 0,
            update_check_status: None,
            update_check_rx: None,
            update_available: None,
            update_install_status: None,
            update_install_rx: None,
            pending_restart: false,
            logbook_columns: app_config.logbook_columns.clone(),
            logbook_column_presets: app_config.logbook_column_presets.clone(),
            column_preset_name: String::new(),
            iota_dialog: IotaBrowserDialog::new(),
            states_dialog: StatesBrowserDialog::new(),
            qsl_manager_dialog: QslManagerDialog::new(),
            photo_viewer_dialog: PhotoViewerDialog::new(),
            prefix_manager_dialog: PrefixManagerDialog::new(),
            astronomy_dialog: AstronomyDialog::new(),
            wol_dialog: WolDialog::new(),
            sota_dialog: SotaDialog::new(),

            compact_hud_mode: app_config.compact_hud_mode,
            tabbed_columns: app_config.tabbed_columns,
            active_tab: [0; 3],
            dock_state: DockState::new(vec![]),
            last_saved_dock_layout: None,
            hud_always_on_top: app_config.hud_always_on_top,
            hud_saved_pos: app_config.hud_saved_pos,
            hud_saved_size: app_config.hud_saved_size,
            hud_operating_bar: app_config.hud_operating_bar,
            main_window_pos: app_config.main_window_pos,
            main_window_size: app_config.main_window_size,
            main_window_maximized: app_config.main_window_maximized,

            entry_iota: String::new(),
            entry_state: String::new(),
            entry_sota: String::new(),
            entry_pota: String::new(),
            entry_qsl_manager: String::new(),
            active_qsl_manager_record: None,

            cloudlog_url: app_config.cloudlog_url,
            cloudlog_api_key: app_config.cloudlog_api_key,
            hrdlog_username: app_config.hrdlog_username,
            hrdlog_upload_code: app_config.hrdlog_upload_code,
            hamqth_username: app_config.hamqth_username,
            hamqth_password: app_config.hamqth_password,

            cat_state_tx,
            cat_state_rx,

            event_bus: crate::core::events::EventBus::default(),

            cluster_host: app_config.cluster_host,
            cluster_port: app_config.cluster_port,
            cluster_callsign: app_config.cluster_callsign,
            cluster_auto_connect: app_config.cluster_auto_connect,
            cluster_connected: false,
            cluster_connecting: false,
            cluster_status_text: "Rozłączono z klastrem DX.".to_string(),
            cluster_filter_current_band: app_config.cluster_filter_current_band,
            cluster_hide_ft8: app_config.cluster_hide_ft8,
            cluster_hide_skimmers: app_config.cluster_hide_skimmers,
            custom_clusters: app_config.custom_clusters,
            show_add_cluster_dialog: false,
            new_cluster_name: String::new(),
            new_cluster_host: String::new(),
            new_cluster_port: 7300,
            cluster_event_rx: None,
            cluster_stop_tx: None,

            reset_layout_requested: false,
            left_column_width: app_config.left_column_width,
            right_column_width: app_config.right_column_width,
            dragging_tile: None,

            panel_vfo: app_config.panel_vfo,
            panel_qso: app_config.panel_qso,
            panel_log: app_config.panel_log,
            panel_cluster: app_config.panel_cluster,
            panel_solar: app_config.panel_solar,
            panel_bandmap: app_config.panel_bandmap.clone(),
            panel_satellites: app_config.panel_satellites.clone(),
            panel_world_map: app_config.panel_world_map.clone(),

            // Sortowanie i paginacja logu
            log_sort_column: 0,
            log_sort_asc: false,
            log_page: 0,
            log_page_size: 50,
            selected_qso_ids: Vec::new(),
            confirm_bulk_delete: false,

            // Statystyki
            show_statistics_window: false,
            stats_tab: 0,

            // Undo/Redo
            undo_stack: std::collections::VecDeque::new(),
            redo_stack: std::collections::VecDeque::new(),

            // LoTW Users cache
            lotw_users_cache: std::collections::HashMap::new(),

            // PSK Reporter / WSPR
            show_pskreporter_window: false,
            show_wspr_window: false,

            // REST API
            rest_api_enabled: false,
            rest_api_port: 8080,
            rest_api_key: app_config.rest_api_key.clone(),

            // Band opening alerts
            band_alert_k_index_threshold: 4,
            band_alert_enabled: true,

            // System pluginów użytkownika (Rhai)
            plugins_enabled: app_config.plugins_enabled,
            plugins_dir: app_config.plugins_dir.clone(),
            show_plugin_manager: false,
            plugin_engine: crate::plugins::PluginEngine::new(),
            plugin_log: Vec::new(),
            plugin_notifications: Vec::new(),

            // Zintegrowany pasek asystenta operatora
            operator_assistant_enabled: app_config.operator_assistant_enabled,
            show_operator_assistant: app_config.operator_assistant_enabled,
            show_propagation_explain: false,

            // Historia i weryfikacja prognoz propagacyjnych
            propagation_history: app_config.propagation_history.clone(),
            previous_band_status: std::collections::HashMap::new(),

            // PTT
            ptt_active: false,

            // Dwuetapowy CAT i udostępnianie (CAT Sharing)
            cat_mfg_selected: "Wszystkie".to_string(),
            cat_conn_type: if app_config.cat_backend == "tci" { "tci".to_string() } else { "serial".to_string() },
            cat_sharing_enabled: app_config.cat_sharing_enabled,
            cat_sharing_port: app_config.cat_sharing_port,
            cat_sharing_active: false,
            cat_shared_state: std::sync::Arc::new(std::sync::RwLock::new(crate::cat::hamlib::RigState::default())),
            cat_proxy_server: None,
            cat_proxy_rx: None,

            // VFO Konsola radiowa & DSP
            vfo_split_offset_khz: 1.0,
            vfo_filter_preset: "FIL2".to_string(),
            vfo_preamp_att: "OFF".to_string(),
            vfo_nb_nr: false,
            vfo_agc_speed: "MID".to_string(),

            // Kompas antenowy i wiązka na mapie
            map_show_beam_lobe: true,
            map_show_compass: true,
            map_beamwidth_deg: 50.0,

            // Boczny panel filtrów klastra
            cluster_show_filter_sidebar: false,
            cluster_search_query: String::new(),
            cluster_filter_band_selection: "ALL".to_string(),
            cluster_filter_mode_selection: "ALL".to_string(),
            cluster_filter_status: "ALL".to_string(),
            cluster_filter_source: "ALL".to_string(),
            cluster_filter_pota_sota_only: false,

            // Wyszukiwanie duplikatów
            show_find_duplicates_window: false,
            duplicates_match_date: false,
            duplicates_groups: Vec::new(),
            duplicates_selected_ids: std::collections::HashSet::new(),
            duplicates_status: None,

            // Profile stacji roboczej
            show_station_profiles_window: false,
            station_profiles: if app_config.station_profiles.is_empty() {
                let mut def = app_config.station.clone();
                if def.id.is_empty() { def.id = "default".to_string(); }
                if def.name.is_empty() { def.name = "Główny (Dom QTH)".to_string(); }
                vec![def]
            } else {
                app_config.station_profiles.clone()
            },
            active_profile_id: app_config.active_profile_id.clone(),
            focus_callsign_requested: false,
            lookup_debounce_until: None,
        };

        app.rebuild_awards_full();

        // Zbuduj (lub wczytaj z konfiguracji) układ dokowania paneli.
        app.build_dock_state(app_config.dock_layout.as_ref());

        // Wczytanie pluginów użytkownika (Rhai) z katalogu i uruchomienie on_startup().
        {
            let dir = std::path::PathBuf::from(&app.plugins_dir);
            app.plugin_engine.set_enabled(app.plugins_enabled);
            app.plugin_engine.load_dir(&dir);
            let total = app.qso_numbers.len() as i64;
            app.plugin_engine.set_qso_count(total);
            app.plugin_engine.run_startup();
        }

        if app.cat_sharing_enabled {
            app.toggle_cat_proxy_server();
        }

        if app.rest_api_enabled {
            app.start_rest_api_server();
        }

        if app.cluster_auto_connect {
            app.connect_dx_cluster();
        }

        app
    }

    /// Aktualizuje podpowiedzi SCP, rozpoznanie DXCC, azymut i dystans po wpisaniu znaku
    pub fn on_callsign_changed(&mut self) {
        let clean = self.entry_callsign.trim().to_uppercase();
        if clean.is_empty() {
            self.scp_suggestions.clear();
            self.callsign_corrections.clear();
            self.active_prefix_info = None;
            self.active_polish_district = None;
            self.active_award_status = None;
            self.active_distance_km = 0.0;
            self.active_bearing_deg = 0.0;
            self.active_propagation = None;
            self.active_clubs.clear();
            self.past_qsos_for_active_call.clear();
            return;
        }

        // 1. Podpowiedzi SCP
        {
            let scp = self.scp_engine.lock().unwrap_or_else(|p| p.into_inner());
            self.scp_suggestions = scp.search(&clean, 6);
        }

        // 1b. Lokalna korekta rozmyta (fuzzy) błędnie wpisanego/odebranego znaku
        {
            let scp = self.scp_engine.lock().unwrap_or_else(|p| p.into_inner());
            let candidates = scp.candidates();
            self.callsign_corrections = crate::core::callsign_correction::suggest_corrections(
                &clean,
                &candidates,
                2,
                4,
            );
        }

        // 2. Kluby krótkofalarskie (SP-OTC, PGA, SKCC, CWOPS, FOC)
        self.active_clubs = crate::core::clubs::ClubRegistry::check(&clean);

        // 3. Rozpoznanie DXCC, stref i estymacja propagacji
        if let Some(info) = self.prefix_matcher.lookup(&clean) {
            let dx_coords = crate::core::geo::Coordinates::new(info.latitude, info.longitude);
            if let Ok(my_coords) = locator_to_coordinates(&self.my_station.gridsquare) {
                self.active_distance_km = calculate_distance_km(my_coords, dx_coords);
                self.active_bearing_deg = calculate_bearing_deg(my_coords, dx_coords);
                self.rotor_state.azimuth_deg = self.active_bearing_deg as f32;

                use chrono::{Datelike, Timelike};
                let now = chrono::Utc::now();
                let utc_hour = now.hour() as f64 + (now.minute() as f64) / 60.0;
                let day_of_year = now.ordinal();
                let sfi = if self.space_weather.sfi > 0 { self.space_weather.sfi } else { 140 };
                let k_index = self.space_weather.k_index;
                let freq_mhz = crate::core::propagation::band_to_center_mhz(&self.entry_band).unwrap_or(14.175);
                self.active_propagation = Some(crate::core::propagation::PropagationEngine::calculate(
                    my_coords,
                    dx_coords,
                    freq_mhz,
                    sfi,
                    k_index as u8,
                    utc_hour,
                    day_of_year,
                ));
            }

            self.active_polish_district = AwardsEngine::get_polish_district(&clean);

            let awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
            self.active_award_status = Some(awards.check_status_full(
                &clean,
                &self.entry_band,
                &self.entry_mode,
                Some(info.dxcc),
                self.my_station.pga_gmina.as_deref(),
                Some(info.cqz),
                if self.entry_state.trim().is_empty() { None } else { Some(self.entry_state.trim()) },
                Some(&info.continent),
                if self.entry_iota.trim().is_empty() { None } else { Some(self.entry_iota.trim()) },
            ));

            self.active_prefix_info = Some(info);
        }

        // 3. Sprawdzenie poprzednich łączności z tą stacją w lokalnej bazie SQLite
        let prevs_opt = if let Ok(db) = self.log_db.lock() {
            db.find_previous_qsos(&clean).ok()
        } else {
            None
        };

        if let Some(prevs) = prevs_opt {
            if let Some(last) = prevs.first() {
                if self.entry_name.is_empty() {
                    if let Some(ref n) = last.name {
                        self.entry_name = n.clone();
                    }
                }
                if self.entry_qth.is_empty() {
                    if let Some(ref q) = last.qth {
                        self.entry_qth = q.clone();
                    }
                }
                if self.entry_grid.is_empty() {
                    if let Some(ref g) = last.gridsquare {
                        self.entry_grid = g.clone();
                        self.recalculate_distance_from_grid();
                    }
                }
                if self.entry_pga.is_empty() {
                    if let Some(ref p) = last.pga_ref {
                        self.entry_pga = p.clone();
                    }
                }
            }
            self.past_qsos_for_active_call = prevs;
        }

        // 4. Baza managerów QSL (serviceLOG)
        if let Some(mgr_rec) = self.service_db.find_manager(&clean) {
            if self.entry_qsl_manager.is_empty() {
                self.entry_qsl_manager = mgr_rec.manager.clone();
            }
            self.active_qsl_manager_record = Some(mgr_rec);
        } else {
            self.active_qsl_manager_record = None;
        }

        // 5. Baza UniqueCalls
        if let Some(uniq) = self.service_db.find_unique_call(&clean) {
            if let Some(ref mut pfx) = self.active_prefix_info {
                pfx.country = uniq.country;
                pfx.dxcc = uniq.dxcc;
                pfx.continent = uniq.continent;
                pfx.cqz = uniq.cq_zone;
                pfx.ituz = uniq.itu_zone;
            }
        }

        // 6. Baza Callbook (databases/callbook.db - 52k rekordów)
        if clean.len() >= 3 {
            if let Some(cb_data) = self.local_callbook.lookup(&clean) {
                if let Some(n) = cb_data.name {
                    if self.entry_name.is_empty() {
                        self.entry_name = n;
                    }
                }
                if let Some(q) = cb_data.qth {
                    if self.entry_qth.is_empty() {
                        self.entry_qth = q;
                    }
                }
                if let Some(g) = cb_data.gridsquare {
                    if self.entry_grid.is_empty() {
                        self.entry_grid = g;
                        self.recalculate_distance_from_grid();
                    }
                }
                if let Some(s) = cb_data.state {
                    if self.entry_state.is_empty() {
                        self.entry_state = s;
                    }
                }
                if let Some(mgr) = cb_data.qsl_manager {
                    if self.entry_qsl_manager.is_empty() {
                        self.entry_qsl_manager = mgr;
                    }
                }
            }
        }

        // 7. Jeśli włączono automatyczne pobieranie z serwisów online (HamQTH / Callook / QRZ)
        //    i znak ma min. 3 znaki — opóźnij o 600 ms (debounce), aby nie spamować zapytań przy szybkim pisaniu.
        if self.qrz_auto_lookup && clean.len() >= 3 {
            self.lookup_debounce_until = Some(std::time::Instant::now() + std::time::Duration::from_millis(600));
        }
    }

    /// Wywołuje automatyczny lookup online po upływie debounce'a (wołane co klatkę w update()).
    pub fn process_debounced_lookup(&mut self) {
        if let Some(deadline) = self.lookup_debounce_until {
            if std::time::Instant::now() >= deadline {
                self.lookup_debounce_until = None;
                let clean = self.entry_callsign.trim().to_uppercase();
                if self.qrz_auto_lookup && clean.len() >= 3 {
                    self.lookup_active_callsign_online();
                }
            }
        }
    }

    pub fn recalculate_distance_from_grid(&mut self) {
        if self.entry_grid.len() >= 4 {
            if let (Ok(p1), Ok(p2)) = (locator_to_coordinates(&self.my_station.gridsquare), locator_to_coordinates(&self.entry_grid)) {
                self.active_distance_km = calculate_distance_km(p1, p2);
                self.active_bearing_deg = calculate_bearing_deg(p1, p2);
                self.rotor_state.azimuth_deg = self.active_bearing_deg as f32;

                use chrono::{Datelike, Timelike};
                let now = chrono::Utc::now();
                let utc_hour = now.hour() as f64 + (now.minute() as f64) / 60.0;
                let day_of_year = now.ordinal();
                let sfi = if self.space_weather.sfi > 0 { self.space_weather.sfi } else { 140 };
                let k_index = self.space_weather.k_index;
                let freq_mhz = crate::core::propagation::band_to_center_mhz(&self.entry_band).unwrap_or(14.175);
                self.active_propagation = Some(crate::core::propagation::PropagationEngine::calculate(
                    p1,
                    p2,
                    freq_mhz,
                    sfi,
                    k_index as u8,
                    utc_hour,
                    day_of_year,
                ));
            }
        }
    }

    /// Obraca antenę na podany azymut przez protokół rotctld (Hamlib)
    pub fn rotate_antenna_to(&mut self, azimuth_deg: f32) {
        self.rotor_state.azimuth_deg = azimuth_deg;
        let host = self.rotor_host.clone();
        let port = self.rotor_port;
        tokio::spawn(async move {
            let _ = crate::cat::rotor::RotorClient::set_position(&host, port, azimuth_deg, 0.0).await;
        });
        self.status_toast = Some((
            format!("Wysłano polecenie obrotu anteny na azymut {:.0}° ({}:{})", azimuth_deg, self.rotor_host, self.rotor_port),
            std::time::Instant::now(),
        ));
    }

    pub fn save_qso(&mut self) {
        if self.entry_callsign.trim().is_empty() {
            return;
        }

        let now = chrono::Utc::now();
        let date_str = now.format("%Y%m%d").to_string();
        let time_str = now.format("%H%M").to_string();

        let mut qso = QsoRecord::new(&self.entry_callsign, &self.entry_band, &self.entry_mode);
        qso.journal_id = Some(self.active_journal.id.clone());
        qso.qso_date = date_str.clone();
        qso.time_on = time_str.clone();
        qso.rst_sent = self.entry_rst_sent.clone();
        qso.rst_rcvd = self.entry_rst_rcvd.clone();
        qso.name = if !self.entry_name.is_empty() { Some(self.entry_name.clone()) } else { None };
        qso.qth = if !self.entry_qth.is_empty() { Some(self.entry_qth.clone()) } else { None };
        qso.gridsquare = if !self.entry_grid.is_empty() { Some(self.entry_grid.clone()) } else { None };
        qso.pga_ref = if !self.entry_pga.is_empty() { Some(self.entry_pga.clone()) } else { None };
        qso.comment = if !self.entry_comment.is_empty() { Some(self.entry_comment.clone()) } else { None };
        qso.iota = if !self.entry_iota.is_empty() { Some(self.entry_iota.clone()) } else { None };
        qso.state = if !self.entry_state.is_empty() { Some(self.entry_state.clone()) } else { None };
        qso.sota_ref = if !self.entry_sota.is_empty() { Some(self.entry_sota.clone()) } else { None };
        qso.pota_ref = if !self.entry_pota.is_empty() { Some(self.entry_pota.clone()) } else { None };
        qso.qsl_via = if !self.entry_qsl_manager.is_empty() { Some(self.entry_qsl_manager.clone()) } else { None };
        qso.qsl_manager = qso.qsl_via.clone();

        // Sprawdzenie czy nagrywano audio łączności (Audio Memo)
        if crate::media::audio_recorder::AudioRecorder::is_recording() {
            let rec_dir = std::path::Path::new("recordings");
            let clean_call = self.entry_callsign.trim().to_uppercase().replace('/', "_");
            let filename = format!("QSO_{}_{}_{}.wav", clean_call, date_str, time_str);
            let path = rec_dir.join(filename);
            if let Ok(saved) = crate::media::audio_recorder::AudioRecorder::stop_and_save(&path) {
                qso.audio_file = Some(saved.to_string_lossy().to_string());
            }
        }

        if let Some(ref info) = self.active_prefix_info {
            qso.country = Some(info.country.clone());
            qso.dxcc = Some(info.dxcc);
            qso.continent = Some(info.continent.clone());
            qso.cqz = Some(info.cqz);
            qso.ituz = Some(info.ituz);
        }

        let insert_res = {
            let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
            db.insert_qso(&qso)
        };

        match insert_res {
            Ok(id) => {
                qso.id = Some(id);
                // Opublikuj zdarzenie na centralnej magistrali (WebSocket, plugin, toast).
                self.event_bus.publish(crate::core::events::AppEvent::QsoLogged {
                    callsign: qso.callsign.clone(),
                    band: qso.band.clone(),
                    mode: qso.mode.clone(),
                    frequency_hz: (qso.freq.unwrap_or(0.0) * 1_000_000.0) as u64,
                    time_utc: qso.time_on.clone(),
                });
                {
                    let mut awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
                    awards.register_qso_record(&qso);
                }
                self.invalidate_cluster_badges();
                {
                    let mut scp = self.scp_engine.lock().unwrap_or_else(|p| p.into_inner());
                    scp.insert(&qso.callsign);
                }
            }
            Err(e) => {
                self.status_toast = Some((
                    format!("Błąd zapisu łączności do bazy danych: {}", e),
                    std::time::Instant::now(),
                ));
            }
        }

        self.contest_qsos += 1;
        self.contest_points += 3;
        self.contest_mults += 1;
        self.contest_stx += 1;

        if self.band_alert_enabled {
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

        self.status_toast = Some((
            format!("Zapisano łączność z: {} ({}, {})", qso.callsign, qso.band, qso.mode),
            std::time::Instant::now(),
        ));

        // Weryfikacja prognozy propagacyjnej: udana łączność = potwierdzone otwarcie pasma.
        self.propagation_history
            .record_observation(&qso.band, crate::core::propagation_history::ObservedOutcome::ConfirmedOpen);

        // Hook pluginów użytkownika (Rhai) po zapisaniu łączności.
        self.plugin_engine.run_on_qso_logged(&crate::plugins::QsoHookContext {
            callsign: qso.callsign.clone(),
            band: qso.band.clone(),
            mode: qso.mode.clone(),
            freq_mhz: qso.freq.unwrap_or(0.0),
            is_atno: self
                .active_award_status
                .as_ref()
                .map(|s| s.is_new_dxcc)
                .unwrap_or(false),
        });

        // Automatyczny przesył na żywo do Club Log — przez wspólną kolejkę wysyłki
        if self.live_auto_upload_clublog && !self.clublog_callsign.is_empty() && !self.clublog_password.is_empty() {
            let adif_record = crate::core::adif::export_adif(std::slice::from_ref(&qso), "SPLogbook", &self.my_station.callsign);
            let now = crate::cloud::scheduler::now_unix();
            let mut sched = self.upload_scheduler.lock().unwrap();
            sched.enqueue(crate::cloud::scheduler::UploadService::ClubLog, adif_record, now);
            sched.save_to_disk();
        }

        // Automatyczny przesył na żywo do logbooka QRZ.com — przez wspólną kolejkę wysyłki
        if self.live_auto_upload_qrz && !self.qrz_api_key.is_empty() {
            let adif_record = crate::core::adif::export_adif(std::slice::from_ref(&qso), "SPLogbook", &self.my_station.callsign);
            let now = crate::cloud::scheduler::now_unix();
            let mut sched = self.upload_scheduler.lock().unwrap();
            sched.enqueue(crate::cloud::scheduler::UploadService::Qrz, adif_record, now);
            sched.save_to_disk();
        }

        // Automatyczne raportowanie do PSK Reporter
        if self.psk_reporter_enabled && !self.my_station.callsign.is_empty() && !self.my_station.gridsquare.is_empty() {
            let client = crate::cloud::psk_reporter::PskReporterClient::new(&self.my_station.callsign, &self.my_station.gridsquare);
            let qso_clone = qso.clone();
            tokio::spawn(async move {
                let _ = client.submit_reception_report(&qso_clone).await;
            });
        }

        // Rozgłaszanie w sieci lokalnej Multi-Op LAN
        if let Some(ref srv) = self.multi_op_server {
            srv.broadcast_qso(&qso);
        }

        // Emisja zdarzenia w formacie N1MM Logger+ UDP broadcast (GridTracker itp.)
        if self.n1mm_broadcast_enabled {
            let host = self.n1mm_broadcast_host.clone();
            let port = self.n1mm_broadcast_port;
            let my_call = self.my_station.callsign.clone();
            let xml = crate::digital::n1mm::contactinfo_xml(&qso, &my_call, 1);
            tokio::spawn(async move {
                if let Err(e) = crate::digital::n1mm::send_broadcast(&host, port, &xml).await {
                    log::warn!("N1MM broadcast nieudany ({}:{}): {}", host, port, e);
                }
            });
        }

        self.clear_qso_form();
        self.reload_qsos();
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
            "mi" => format!("{:.0} mi", km * 0.621371),
            "nmi" => format!("{:.0} NM", km * 0.539957),
            _ => format!("{:.0} km", km),
        }
    }

    pub fn reload_qsos(&mut self) {
        // Pobierz duży bufor żeby paginacja w tabeli miała z czego czerpać
        let limit = if self.log_page_size == 0 { 10000 } else { (self.log_page_size * 20).max(500) };
        if let Ok(db) = self.log_db.lock() {
            self.recent_qsos = db.get_recent_qsos_for_journal(&self.active_journal.id, limit).unwrap_or_default();
            match db.qso_numbers_for_journal(&self.active_journal.id) {
                Ok(numbers) => self.qso_numbers = numbers,
                Err(error) => {
                    log::error!("Nie udało się wyliczyć numerów QSO: {error}");
                    self.qso_numbers.clear();
                    self.status_toast = Some((
                        format!("Błąd numeracji QSO: {error}"),
                        std::time::Instant::now(),
                    ));
                }
            }
        }
        // Zresetuj stronę jeśli wyszła poza zakres
        if self.log_page_size > 0 {
            let pages = (self.recent_qsos.len() + self.log_page_size - 1) / self.log_page_size.max(1);
            if self.log_page >= pages && pages > 0 {
                self.log_page = pages - 1;
            }
        }
    }

    /// Zwraca kolor i odznakę (⭐ nowe DXCC, ✨ nowe pasmo) dla spotu klastra,
    /// korzystając z cache, aby nie przeliczać prefiksu i statusu nagród co klatkę.
    pub fn cluster_spot_badge(&mut self, dx_call: &str, band: &str, is_ft8: bool) -> (egui::Color32, &'static str) {
        let key = format!("{}|{}|{}", dx_call, band, is_ft8);
        if let Some(&badge) = self.cluster_badge_cache.get(&key) {
            return badge;
        }

        let badge = if let Some(info) = self.prefix_matcher.lookup(dx_call) {
            let awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
            let st = awards.check_status_full(
                dx_call,
                band,
                if is_ft8 { "FT8" } else { "CW" },
                Some(info.dxcc),
                None,
                Some(info.cqz),
                None,
                Some(info.continent.as_str()),
                None,
            );
            if st.is_new_dxcc {
                (egui::Color32::from_rgb(217, 70, 239), " ⭐")
            } else if st.is_new_band {
                (egui::Color32::from_rgb(34, 197, 94), " ✨")
            } else {
                (egui::Color32::from_rgb(56, 189, 248), "")
            }
        } else {
            (egui::Color32::from_rgb(56, 189, 248), "")
        };

        self.cluster_badge_cache.insert(key, badge);
        badge
    }

    /// Czyści cache odznak spotów klastra po zmianie stanu nagród (nowe/usunięte QSO).
    pub fn invalidate_cluster_badges(&mut self) {
        self.cluster_badge_cache.clear();
    }

    /// Zwraca prognozy otwarcia pasm dla panelu słonecznego z cache'em kluczowanym
    /// po (SFI, K, godzina, minuta, dzień roku), aby nie przeliczać 11 pasm co klatkę.
    pub fn solar_band_forecasts(&mut self) -> Vec<(&'static str, crate::core::propagation::PropagationForecast)> {
        use chrono::{Datelike, Timelike};

        let sfi = if self.space_weather.sfi > 0 { self.space_weather.sfi } else { 140 };
        let k = self.space_weather.k_index;
        let now = chrono::Utc::now();
        let hour = now.hour();
        let minute = now.minute();
        let doy = now.ordinal();
        let key = (sfi, k, hour, minute, doy);

        if let Some((cached_key, cached)) = &self.solar_propagation_cache {
            if *cached_key == key {
                return cached.clone();
            }
        }

        let sp = crate::core::geo::Coordinates::new(51.1, 17.0);
        let dx = crate::core::geo::Coordinates::new(40.7, -74.0);
        let utc_h = hour as f64 + (minute as f64) / 60.0;
        let forecasts: Vec<(&'static str, crate::core::propagation::PropagationForecast)> = crate::core::propagation::HF_BANDS
            .iter()
            .map(|&(name, freq)| {
                (
                    name,
                    crate::core::propagation::PropagationEngine::calculate(sp, dx, freq, sfi, k as u8, utc_h, doy),
                )
            })
            .collect();

        // Zapis prognoz do historii dokładności oraz wykrywanie przejść
        // otwarcia/zamknięcia pasm (dla alertów dźwiękowych i pluginów).
        let mut newly_opened: Vec<&'static str> = Vec::new();
        for (name, fc) in forecasts.iter() {
            let band = *name;
            self.propagation_history.record_forecast(
                band,
                utc_h,
                sfi,
                k as u8,
                fc.reliability_pct,
                fc.status,
            );
            if let Some(prev) = self.previous_band_status.get(band).copied() {
                let was_open = prev == crate::core::propagation::BandOpeningStatus::Open;
                let is_open = fc.status == crate::core::propagation::BandOpeningStatus::Open;
                if !was_open && is_open {
                    newly_opened.push(band);
                }
            }
            self.previous_band_status.insert(band.to_string(), fc.status);
        }

        if !newly_opened.is_empty() && self.band_alert_enabled {
            let names = newly_opened.join(", ");
            crate::media::sounds::play_band_opened_alert();
            self.status_toast = Some((
                format!("📡 Otwarcie pasma: {names}"),
                std::time::Instant::now(),
            ));
            for band in newly_opened {
                self.plugin_engine.run_on_band_opened(band);
            }
        }

        self.solar_propagation_cache = Some((key, forecasts.clone()));
        forecasts
    }

    pub fn rebuild_awards_full(&mut self) {
        if let Ok(db) = self.log_db.lock() {
            self.recent_qsos = db.get_recent_qsos_for_journal(&self.active_journal.id, 100).unwrap_or_default();
            if let Ok(all_qsos) = db.get_all_qsos() {
                drop(db);
                if let Ok(mut awards) = self.awards_engine.lock() {
                    awards.rebuild_from_qsos(&all_qsos);
                }
            }
        }
        // Stan nagród się zmienił — odznaki spotów klastra są nieaktualne.
        self.invalidate_cluster_badges();
    }

    pub fn delete_selected_qso(&mut self) {
        let ids: Vec<i64> = self.selected_qso_ids.drain(..).collect();
        if ids.is_empty() {
            return;
        }
        let mut first_err: Option<String> = None;
        for id in &ids {
            let result = {
                let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                db.delete_qso(*id)
            };
            if let Err(e) = result {
                if first_err.is_none() {
                    first_err = Some(format!("Błąd usuwania łączności #{}: {}", id, e));
                }
            }
        }
        if let Some(msg) = first_err {
            self.report_error(msg);
        } else {
            self.status_message = Some(format!("Usunięto {} łączności z dziennika.", ids.len()));
        }
        self.rebuild_awards_full();
        self.reload_qsos();
    }

    pub fn delete_qso_by_id(&mut self, id: i64) {
        let result = {
            let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
            db.delete_qso(id)
        };
        if let Err(e) = result {
            self.report_error(format!("Błąd usuwania łączności #{}: {}", id, e));
            return;
        }
        self.rebuild_awards_full();
        self.reload_qsos();
        self.status_message = Some(format!("Usunięto łączność #{} z dziennika.", id));
    }

    /// Cofa ostatnie usunięte QSO (Undo)
    pub fn perform_undo(&mut self) {
        if let Some(qso) = self.undo_stack.pop_front() {
            let callsign = qso.callsign.clone();
            let mut restored = qso.clone();
            restored.id = None; // nowe ID przy przywracaniu
            let result = {
                let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                db.insert_qso(&restored)
            };
            if let Err(e) = result {
                self.report_error(format!("Błąd przywracania łączności: {}", e));
                return;
            }
            self.redo_stack.push_front(qso);
            self.rebuild_awards_full();
            self.reload_qsos();
            self.status_message = Some(format!("Przywrócono łączność z: {}", callsign));
        }
    }

    /// Ponawia cofnięte QSO (Redo — usuwa ponownie)
    pub fn perform_redo(&mut self) {
        if let Some(qso) = self.redo_stack.pop_front() {
            let callsign = qso.callsign.clone();
            // Szukamy QSO w bazie po znaku i dacie żeby je usunąć
            let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
            if let Ok(all) = db.get_recent_qsos_for_journal(&self.active_journal.id, 10) {
                for q in all {
                    if q.callsign == qso.callsign && q.qso_date == qso.qso_date && q.time_on == qso.time_on {
                        if let Some(id) = q.id {
                            let _ = db.delete_qso(id);
                            break;
                        }
                    }
                }
            }
            drop(db);
            self.undo_stack.push_front(qso);
            self.rebuild_awards_full();
            self.reload_qsos();
            self.status_message = Some(format!("Redo: usunięto łączność z: {}", callsign));
        }
    }

    pub fn save_edited_qso(&mut self) {
        if let Some(ref qso) = self.editing_qso {
            if let Some(id) = qso.id {
                let result = {
                    let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                    db.update_qso(id, qso)
                };
                if let Err(e) = result {
                    self.report_error(format!("Błąd aktualizacji łączności #{}: {}", id, e));
                    return;
                }
                self.status_message = Some(format!("Zaktualizowano rekord łączności z: {}", qso.callsign));
            }
        }
        self.editing_qso = None;
        self.reload_qsos();
    }

    pub fn lookup_active_callsign_online(&mut self) {
        let call = self.entry_callsign.trim().to_uppercase();
        if call.is_empty() {
            return;
        }

        // Natychmiastowe sprawdzenie lokalnej bazy (callbook.db + serviceLOG.db)
        if let Some(cb_data) = self.local_callbook.lookup(&call) {
            if let Some(n) = cb_data.name.clone() { self.entry_name = n; }
            if let Some(q) = cb_data.qth.clone() { self.entry_qth = q; }
            if let Some(g) = cb_data.gridsquare.clone() { 
                self.entry_grid = g; 
                self.recalculate_distance_from_grid();
            }
            if let Some(s) = cb_data.state.clone() { self.entry_state = s; }
            if let Some(mgr) = cb_data.qsl_manager.clone() { self.entry_qsl_manager = mgr; }
            self.status_message = Some(format!("Znaleziono w bazie Callbook: {}", call));
        }

        let lc = self.local_callbook.clone();
        let hamqth_u = self.hamqth_username.clone();
        let hamqth_p = self.hamqth_password.clone();
        let qrz_u = self.qrz_username.clone();
        let qrz_p = self.qrz_password.clone();
        let tx = self.qrz_lookup_tx.clone();

        tokio::spawn(async move {
            if let Some(data) = crate::core::callbook::fetch_callsign_data(
                call, lc, hamqth_u, hamqth_p, qrz_u, qrz_p
            ).await {
                let _ = tx.send(data);
            }
        });
    }

    pub fn tune_to_spot(&mut self, dx_call: &str, freq_khz: f64, band: &str) {
        // Ustaw znak, pasmo i częstotliwość lokalnie
        self.entry_callsign = dx_call.to_string();
        self.entry_band = band.to_string();
        let freq_hz = (freq_khz * 1000.0) as u64;
        self.rig_state.frequency_hz = freq_hz;
        self.on_callsign_changed();

        // Wyślij komendę do transceivera przez Hamlib TCP (rigctld) jeśli połączony
        if self.cat_connected {
            let host = self.cat_host.clone();
            let port = self.cat_port;
            // Hamlib: "F <freq_hz>\n" ustawia częstotliwość VFO A
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                if let Ok(mut stream) = tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await {
                    let cmd = format!("F {}\n", freq_hz);
                    let _ = stream.write_all(cmd.as_bytes()).await;
                }
            });
        }
        self.status_message = Some(format!("Auto-tune → {} na {:.1} kHz", dx_call, freq_khz));
    }

    pub fn stop_rotor(&mut self) {
        self.rotor_state.moving = false;
    }

    pub fn turn_rotor_short_path(&mut self) {
        if self.active_bearing_deg > 0.0 {
            self.rotor_state.azimuth_deg = self.active_bearing_deg as f32;
        }
    }

    pub fn turn_rotor_long_path(&mut self) {
        if self.active_bearing_deg > 0.0 {
            self.rotor_state.azimuth_deg = ((self.active_bearing_deg + 180.0) % 360.0) as f32;
        }
    }

    pub fn tune_satellite_frequencies(&mut self) {
        self.rig_state.frequency_hz = (self.sat_downlink_mhz * 1_000_000.0) as u64;
    }

    pub fn point_rotor_to_satellite(&mut self) {
        self.rotor_state.azimuth_deg = self.sat_azimuth;
        self.rotor_state.elevation_deg = self.sat_elevation;
    }

    pub fn transmit_cw_macro(&mut self, text: &str) {
        let resolved = text
            .replace("%MYCALL%", &self.my_station.callsign)
            .replace("%HISCALL%", &self.entry_callsign)
            .replace("%RST%", &self.entry_rst_sent)
            .replace("%SERIAL%", &format!("{:03}", self.contest_stx));
        self.status_message = Some(format!("Nadawanie CW ({} WPM): {}", self.cw_wpm, resolved));
    }

    pub fn export_cabrillo(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pliki Cabrillo (*.cbr, *.log)", &["cbr", "log"])
            .set_file_name(format!("{}_{}.cbr", self.my_station.callsign, self.contest_name.replace(' ', "_")))
            .set_title("Zapisz dziennik zawodów Cabrillo 3.0")
            .save_file()
        {
            let qsos = if let Ok(db) = self.log_db.lock() {
                db.get_recent_qsos_for_journal(&self.active_journal.id, 10000).unwrap_or_default()
            } else {
                vec![]
            };

            let mut out = String::new();
            out.push_str("START-OF-LOG: 3.0\n");
            out.push_str(&format!("CONTEST: {}\n", self.contest_name));
            out.push_str(&format!("CALLSIGN: {}\n", self.my_station.callsign));
            out.push_str("CATEGORY-OPERATOR: SINGLE-OP\n");
            out.push_str("CATEGORY-TRANSMITTER: ONE\n");
            out.push_str("CATEGORY-POWER: HIGH\n");
            out.push_str("CATEGORY-BAND: ALL\n");
            out.push_str("CATEGORY-MODE: MIXED\n");
            out.push_str("CATEGORY-STATION: FIXED\n");
            let total_score = self.contest_points * self.contest_mults.max(1);
            out.push_str(&format!("CLAIMED-SCORE: {}\n", total_score));
            out.push_str(&format!("OPERATORS: {}\n", self.my_station.callsign));
            out.push_str(&format!("NAME: {}\n", self.my_station.operator));
            out.push_str(&format!("ADDRESS: {}, {}\n", self.my_station.city, self.my_station.country));
            out.push_str("SOAPBOX: Created with SPLogbook by SP6INA (GPLv3)\n");

            for (idx, q) in qsos.iter().rev().enumerate() {
                let freq_khz = q.freq.map(|f| (f * 1000.0) as u64).unwrap_or_else(|| {
                    match q.band.as_str() {
                        "160m" => 1840,
                        "80m" => 3700,
                        "40m" => 7100,
                        "20m" => 14200,
                        "15m" => 21200,
                        "10m" => 28500,
                        _ => 14000,
                    }
                });
                let date_str = if q.qso_date.len() == 8 {
                    format!("{}-{}-{}", &q.qso_date[0..4], &q.qso_date[4..6], &q.qso_date[6..8])
                } else {
                    chrono::Utc::now().format("%Y-%m-%d").to_string()
                };
                let time_str = if q.time_on.len() >= 4 {
                    q.time_on[0..4].to_string()
                } else {
                    chrono::Utc::now().format("%H%M").to_string()
                };

                let my_rst = if q.mode == "CW" { "599" } else { "59" };
                let his_rst = &q.rst_rcvd;
                let my_serial = idx + 1;
                let his_serial = q.srx.unwrap_or(1);

                out.push_str(&format!(
                    "QSO: {:5} {:2} {} {} {:10} {:3} {:03} {:10} {:3} {:03}\n",
                    freq_khz,
                    if q.mode == "CW" { "CW" } else { "PH" },
                    date_str,
                    time_str,
                    self.my_station.callsign,
                    my_rst,
                    my_serial,
                    q.callsign,
                    his_rst,
                    his_serial
                ));
            }
            out.push_str("END-OF-LOG:\n");

            match std::fs::write(&path, out) {
                Ok(_) => {
                    self.status_message = Some(format!("Plik zawodów zapisany pomyślnie: {}", path.display()));
                    self.status_toast = Some((format!("Zapisano Cabrillo 3.0: {}", path.file_name().unwrap_or_default().to_string_lossy()), std::time::Instant::now()));
                }
                Err(e) => {
                    self.status_message = Some(format!("Błąd zapisu pliku Cabrillo: {}", e));
                }
            }
        }
    }

    pub fn export_pdf_log(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("SPLogbook.pdf")
            .save_file()
        {
            let qsos = if let Ok(db) = self.log_db.lock() {
                db.get_recent_qsos_for_journal(&self.active_journal.id, 10000).unwrap_or_default()
            } else {
                vec![]
            };

            use printpdf::*;
            let (doc, page1, layer1) = PdfDocument::new("SPLogbook Log", Mm(210.0), Mm(297.0), "Layer 1");
            let mut current_layer = doc.get_page(page1).get_layer(layer1);
            let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();

            current_layer.use_text("SPLogbook Log", 24.0, Mm(10.0), Mm(280.0), &font);
            current_layer.use_text(format!("Operator: {}", self.my_station.callsign), 12.0, Mm(10.0), Mm(270.0), &font);
            current_layer.use_text(format!("Date: {}", chrono::Local::now().format("%Y-%m-%d")), 12.0, Mm(10.0), Mm(265.0), &font);

            // Table headers
            let headers = ["Date", "Time", "Callsign", "Band", "Mode", "RST S", "RST R", "Country", "QSL"];
            let mut y = Mm(250.0);
            let x_positions = [10.0, 35.0, 55.0, 85.0, 105.0, 125.0, 140.0, 155.0, 185.0];

            for (i, h) in headers.iter().enumerate() {
                current_layer.use_text(*h, 10.0, Mm(x_positions[i]), y, &font);
            }
            y -= Mm(5.0);

            let mut row_count = 0;
            let mut page_num = 1;

            for qso in qsos.iter() {
                if y.0 < 20.0 {
                    let (page, layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                    current_layer = doc.get_page(page).get_layer(layer);
                    y = Mm(280.0);
                    page_num += 1;
                }
                current_layer.use_text(&qso.qso_date, 10.0, Mm(x_positions[0]), y, &font);
                current_layer.use_text(&qso.time_on, 10.0, Mm(x_positions[1]), y, &font);
                current_layer.use_text(&qso.callsign, 10.0, Mm(x_positions[2]), y, &font);
                current_layer.use_text(&qso.band, 10.0, Mm(x_positions[3]), y, &font);
                current_layer.use_text(&qso.mode, 10.0, Mm(x_positions[4]), y, &font);
                current_layer.use_text(&qso.rst_sent, 10.0, Mm(x_positions[5]), y, &font);
                current_layer.use_text(&qso.rst_rcvd, 10.0, Mm(x_positions[6]), y, &font);
                current_layer.use_text(qso.country.as_deref().unwrap_or(""), 10.0, Mm(x_positions[7]), y, &font);
                current_layer.use_text(format!("{}/{}", qso.qsl_sent, qso.qsl_rcvd), 10.0, Mm(x_positions[8]), y, &font);
                y -= Mm(5.0);
                row_count += 1;
            }

            // Footer
            current_layer.use_text(format!("Total QSOs: {}", row_count), 12.0, Mm(10.0), Mm(10.0), &font);
            current_layer.use_text(format!("Page {}", page_num), 12.0, Mm(180.0), Mm(10.0), &font);

            if let Ok(file) = std::fs::File::create(&path) {
                if doc.save(&mut std::io::BufWriter::new(file)).is_ok() {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    self.status_toast = Some((format!("Wyeksportowano {} QSO do PDF: {}", row_count, filename), std::time::Instant::now()));
                    let _ = open::that(&path);
                }
            }
        }
    }

    pub fn export_gpx_log(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GPX", &["gpx"])
            .set_file_name("SPLogbook.gpx")
            .save_file()
        {
            let qsos = if let Ok(db) = self.log_db.lock() {
                db.get_recent_qsos_for_journal(&self.active_journal.id, 10000).unwrap_or_default()
            } else {
                vec![]
            };

            use gpx::{Gpx, GpxVersion, Waypoint};
            use geo_types::Point;
            let mut gpx = Gpx {
                version: GpxVersion::Gpx11,
                ..Gpx::default()
            };

            fn gridsquare_to_latlon(grid: &str) -> Option<(f64, f64)> {
                // Współdzielona, zwalidowana logika lokatora Maidenhead (core::geo)
                // zamiast lokalnej duplikacji bez walidacji formatu wejścia.
                crate::core::geo::locator_to_coordinates(grid)
                    .ok()
                    .map(|c| (c.latitude, c.longitude))
            }

            let mut row_count = 0;
            for qso in qsos.iter() {
                if let Some(grid) = &qso.gridsquare {
                    if let Some((lat, lon)) = gridsquare_to_latlon(grid) {
                        let mut waypoint = Waypoint::new(Point::new(lon, lat));
                        waypoint.name = Some(qso.callsign.clone());
                        waypoint.description = Some(format!("{} {}", qso.band, qso.mode));
                        // waypoint.sym = Some("Radio".to_string());
                        gpx.waypoints.push(waypoint);
                        row_count += 1;
                    }
                }
            }

            if let Ok(file) = std::fs::File::create(&path) {
                if gpx::write(&gpx, file).is_ok() {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    self.status_toast = Some((format!("Wyeksportowano {} QSO do GPX: {}", row_count, filename), std::time::Instant::now()));
                    let _ = open::that(&path);
                }
            }
        }
    }

    /// Zwraca istniejący klucz uwierzytelniający REST API lub generuje nowy (i zapisuje
    /// konfigurację), jeśli jeszcze nie istnieje. Klucz jest wymagany przez klientów
    /// w nagłówku `X-Api-Key`, aby zapobiec dostępowi z dowolnej strony/skryptu
    /// działającego lokalnie (serwer nasłuchuje na 127.0.0.1, ale CORS jest otwarty).
    pub fn ensure_rest_api_key(&mut self) -> String {
        if self.rest_api_key.is_empty() {
            self.rest_api_key = crate::api::server::generate_api_key();
            self.save_station_config();
        }
        self.rest_api_key.clone()
    }

    /// Uruchamia wbudowany serwer REST API w osobnym zadaniu tokio, wykorzystując
    /// bieżący stan aplikacji (baza, znak wywoławczy, port, klucz API, spoty klastra).
    pub fn start_rest_api_server(&mut self) {
        let db = self.log_db.clone();
        let cs = self.my_station.callsign.clone();
        let port = self.rest_api_port;
        let key = self.ensure_rest_api_key();
        let spots_for_api = Arc::new(Mutex::new(self.cluster_spots.clone()));
        self.cluster_spots_api = Some(spots_for_api.clone());
        let events = self.event_bus.clone();
        tokio::spawn(async move {
            crate::api::server::start_api_server(db, cs, port, spots_for_api, key, events).await;
        });
    }

    pub fn save_station_config(&mut self) {
        let is_configured = if self.show_welcome_wizard {
            false
        } else {
            !self.my_station.callsign.is_empty() && self.my_station.callsign != "N0CALL"
        };
        let mut cfg = AppConfig {
            secret_store_id: self.secret_store_id.clone(),
            is_configured,
            station: self.my_station.clone(),
            equipment: self.equipment_items.clone(),
            dark_theme: self.dark_theme,
            theme_preset: self.theme_preset.id().to_string(),

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

            fldigi_enabled: self.fldigi_enabled,
            fldigi_host: self.fldigi_host.clone(),
            fldigi_port: self.fldigi_port,

            psk_reporter_enabled: self.psk_reporter_enabled,

            n1mm_broadcast_enabled: self.n1mm_broadcast_enabled,
            n1mm_broadcast_host: self.n1mm_broadcast_host.clone(),
            n1mm_broadcast_port: self.n1mm_broadcast_port,

            voice_keyer_messages: self.voice_keyer_messages.clone(),

            workspace_profiles: self.workspace_profiles.clone(),

            plugins_enabled: self.plugins_enabled,
            plugins_dir: self.plugins_dir.clone(),
            operator_assistant_enabled: self.operator_assistant_enabled,
            propagation_history: self.propagation_history.clone(),

            lan_sync_port: self.lan_sync_port,
            lan_sync_auto_start: self.lan_sync_auto_start,
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

            eqsl_username: self.eqsl_username.clone(),
            eqsl_password: self.eqsl_password.clone(),

            clublog_callsign: self.clublog_callsign.clone(),
            clublog_email: self.clublog_email.clone(),
            clublog_password: self.clublog_password.clone(),
            clublog_api_key: self.clublog_api_key.clone(),

            cloudlog_url: self.cloudlog_url.clone(),
            cloudlog_api_key: self.cloudlog_api_key.clone(),
            rest_api_key: self.rest_api_key.clone(),
            hrdlog_username: self.hrdlog_username.clone(),
            hrdlog_upload_code: self.hrdlog_upload_code.clone(),
            hamqth_username: self.hamqth_username.clone(),
            hamqth_password: self.hamqth_password.clone(),

            wol_mac: self.wol_dialog.mac_address.clone(),
            wol_ip: self.wol_dialog.broadcast_ip.clone(),
            wol_port: self.wol_dialog.port,

            current_language: self.current_language.code().to_string(),
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

            panel_vfo: self.panel_vfo.clone(),
            panel_qso: self.panel_qso.clone(),
            panel_log: self.panel_log.clone(),
            panel_cluster: self.panel_cluster.clone(),
            panel_solar: self.panel_solar.clone(),
            panel_bandmap: self.panel_bandmap.clone(),
            panel_satellites: self.panel_satellites.clone(),
            panel_world_map: self.panel_world_map.clone(),
            logbook_columns: self.logbook_columns.clone(),
            logbook_column_presets: self.logbook_column_presets.clone(),
            custom_contests: self.custom_contests.clone(),
        };
        if let Err(e) = cfg.save_to_file(&self.config_file_path) {
            self.report_error(format!("⚠ Błąd zapisu konfiguracji: {}", e));
        } else {
            self.secret_store_id = cfg.secret_store_id;
        }
    }

    /// Scentralizowane zgłaszanie błędów: log + pływające powiadomienie w UI.
    pub fn report_error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::error!("{}", msg);
        self.status_toast = Some((msg, std::time::Instant::now()));
    }

    /// Scentralizowane zgłaszanie informacji/statusu (bez wpisu do logu błędów).
    pub fn report_info(&mut self, msg: impl Into<String>) {
        self.status_toast = Some((msg.into(), std::time::Instant::now()));
    }

    pub fn connect_dx_cluster(&mut self) {
        self.disconnect_dx_cluster();

        let host = self.cluster_host.clone();
        let port = self.cluster_port;
        let call = if !self.cluster_callsign.trim().is_empty() {
            self.cluster_callsign.trim().to_uppercase()
        } else {
            self.my_station.callsign.clone()
        };

        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

        self.cluster_event_rx = Some(event_rx);
        self.cluster_stop_tx = Some(stop_tx);
        self.cluster_connecting = true;
        self.cluster_connected = false;
        self.cluster_status_text = format!("Łączenie z {}:{}...", host, port);

        tokio::spawn(async move {
            crate::cluster::telnet::DxClusterClient::run_with_events(host, port, call, event_tx, stop_rx).await;
        });
    }

    pub fn disconnect_dx_cluster(&mut self) {
        if let Some(stop_tx) = self.cluster_stop_tx.take() {
            let _ = stop_tx.send(true);
        }
        self.cluster_event_rx = None;
        self.cluster_connected = false;
        self.cluster_connecting = false;
        self.cluster_status_text = "Rozłączono z klastrem DX.".to_string();
    }

    /// Uruchamia nasłuch TCP JS8Call — tworzy kanały mpsc i odpala wątek klienta
    pub fn connect_js8call(&mut self) {
        let (state_tx, state_rx) = std::sync::mpsc::channel::<crate::digital::js8call::Js8CallState>();
        let (qso_tx, qso_rx) = std::sync::mpsc::channel::<crate::core::qso::QsoRecord>();

        let client = crate::digital::js8call::Js8CallClient::new(
            self.js8call_host.clone(),
            self.js8call_port,
        );
        client.start_listener(state_tx, qso_tx);

        self.js8call_state_rx = Some(state_rx);
        self.js8call_qso_rx = Some(qso_rx);
        self.js8call_enabled = true;
        self.status_toast = Some((
            format!("JS8Call: Uruchomiono nasłuch TCP na {}:{}", self.js8call_host, self.js8call_port),
            std::time::Instant::now(),
        ));
    }

    /// Zatrzymuje integrację JS8Call (zamyka kanały przez upuszczenie receiverów)
    pub fn disconnect_js8call(&mut self) {
        self.js8call_state_rx = None;
        self.js8call_qso_rx = None;
        self.js8call_enabled = false;
        self.js8call_state = crate::digital::js8call::Js8CallState::default();
        self.status_toast = Some(("JS8Call: Integracja wyłączona.".to_string(), std::time::Instant::now()));
    }

    // ------------------------------------------------------------------
    // egui_dock: rzeczywisty układ dokowania paneli
    // ------------------------------------------------------------------

    /// Kolejność paneli (identyfikatorów kafelków) używana przy budowaniu
    /// układu dokowania oraz przy synchronizacji widoczności.
    pub fn all_tile_ids() -> [&'static str; 8] {
        [
            "vfo", "qso", "log", "cluster", "bandmap", "solar", "satellites", "world_map",
        ]
    }

    pub fn panel_config(&self, id: &str) -> &ViewPanelConfig {
        match id {
            "vfo" => &self.panel_vfo,
            "qso" => &self.panel_qso,
            "log" => &self.panel_log,
            "cluster" => &self.panel_cluster,
            "bandmap" => &self.panel_bandmap,
            "solar" => &self.panel_solar,
            "satellites" => &self.panel_satellites,
            "world_map" => &self.panel_world_map,
            _ => &self.panel_vfo,
        }
    }

    /// Buduje początkowy `DockState` z konfiguracji. Preferuje zapisany układ
    /// (`dock_layout`); w przeciwnym razie odtwarza układ kolumnowy
    /// na podstawie pól `column`/`order` poszczególnych paneli.
    pub fn build_dock_state(&mut self, dock_layout: Option<&serde_json::Value>) {
        // 1. Spróbuj wczytać zapisany układ egui_dock.
        if let Some(json) = dock_layout {
            if let Ok(state) = serde_json::from_value::<DockState<String>>(json.clone()) {
                self.dock_state = state;
                self.sync_dock_state();
                self.last_saved_dock_layout = self.serialize_dock_layout();
                return;
            }
        }

        // 2. Brak zapisanego układu — zbuduj z kolumn paneli.
        self.rebuild_dock_state_from_panels();
        self.last_saved_dock_layout = self.serialize_dock_layout();
    }

    /// Odtwarza dokowany układ z pól `column`/`order` paneli (3 kolumny
    /// ułożone poziomo). Używane przy pierwszym uruchomieniu i po resecie.
    pub fn rebuild_dock_state_from_panels(&mut self) {
        let mut columns: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for id in Self::all_tile_ids() {
            let (visible, floating, column) = {
                let cfg = self.panel_config(id);
                (cfg.visible, cfg.floating, cfg.column.clamp(0, 2))
            };
            if visible && !floating {
                columns[column].push(id.to_string());
            }
        }
        for col in columns.iter_mut() {
            col.sort_by_key(|id| self.panel_config(id).order);
        }

        // Pierwsza niepusta kolumna staje się korzeniem drzewa.
        let first = if !columns[0].is_empty() {
            0
        } else if !columns[1].is_empty() {
            1
        } else {
            2
        };

        self.dock_state = DockState::new(columns[first].clone());

        // Dołóż pozostałe kolumny jako podziały po prawej stronie ostatniego liścia.
        let mut rightmost = NodeIndex::root();
        for col in (first + 1)..3 {
            if columns[col].is_empty() {
                continue;
            }
            let [_, new_node] = self
                .dock_state
                .main_surface_mut()
                .split_right(rightmost, 0.5, columns[col].clone());
            rightmost = new_node;
        }
    }

    /// Uzgadnia zawartość `dock_state` ze stanem konfiguracji paneli:
    /// usuwa karty paneli ukrytych/odpiętych i dodaje brakujące karty paneli
    /// widocznych i zadokowanych.
    pub fn sync_dock_state(&mut self) {
        for id in Self::all_tile_ids() {
            let (visible, floating) = {
                let cfg = self.panel_config(id);
                (cfg.visible, cfg.floating)
            };
            let should_be_docked = visible && !floating;
            let slot = self.dock_state.find_tab(&id.to_string());
            match (should_be_docked, slot) {
                (false, Some(s)) => {
                    self.dock_state.remove_tab(s);
                }
                (true, None) => {
                    self.dock_state.push_to_first_leaf(id.to_string());
                }
                _ => {}
            }
        }
    }

    /// Serializuje bieżący układ dokowania do surowego JSON (do zapisu w konfiguracji).
    pub fn serialize_dock_layout(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.dock_state).ok()
    }

    /// Renders a floating panel body inside a native multi-viewport OS window.
    /// Restores the window's saved position/size and captures any user
    /// move/resize so the geometry survives detach/dock cycles and can be
    /// moved to another monitor.
    pub fn show_floating_viewport<T>(
        &mut self,
        ctx: &egui::Context,
        viewport_id: egui::ViewportId,
        title: String,
        default_size: [f32; 2],
        min_size: [f32; 2],
        saved_pos: Option<[f32; 2]>,
        saved_size: Option<[f32; 2]>,
        mut body: impl FnMut(&mut Self, &egui::Context) -> T,
    ) -> (T, Option<([f32; 2], [f32; 2])>) {
        let mut captured_geo: Option<([f32; 2], [f32; 2])> = None;
        let mut builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(saved_size.unwrap_or(default_size))
            .with_min_inner_size(min_size);
        if let Some([x, y]) = saved_pos {
            builder = builder.with_position(egui::pos2(x, y));
        }
        let result = ctx.show_viewport_immediate(viewport_id, builder, |vp_ctx, _class| {
            let out = body(self, vp_ctx);
            captured_geo = vp_ctx.input(|i| {
                i.viewport().outer_rect.map(|r| ([r.min.x, r.min.y], [r.width(), r.height()]))
            });
            out
        });
        (result, captured_geo)
    }

    pub fn reset_panel_layout(&mut self) {
        use crate::core::station::ViewPanelConfig;
        self.left_column_width = 350.0;
        self.right_column_width = 360.0;

        self.panel_vfo      = ViewPanelConfig { visible: true,  floating: false, column: 0, order: 0, saved_pos: None, saved_size: None };
        self.panel_qso      = ViewPanelConfig { visible: true,  floating: false, column: 0, order: 1, saved_pos: None, saved_size: None };
        self.panel_log      = ViewPanelConfig { visible: true,  floating: false, column: 1, order: 0, saved_pos: None, saved_size: None };
        self.panel_cluster  = ViewPanelConfig { visible: true,  floating: false, column: 1, order: 1, saved_pos: None, saved_size: None };
        self.panel_bandmap  = ViewPanelConfig { visible: true,  floating: false, column: 0, order: 2, saved_pos: None, saved_size: None };
        self.panel_solar    = ViewPanelConfig { visible: true,  floating: false, column: 2, order: 1, saved_pos: None, saved_size: None };
        self.panel_satellites = ViewPanelConfig { visible: false, floating: false, column: 2, order: 2, saved_pos: None, saved_size: None };
        self.panel_world_map  = ViewPanelConfig { visible: false, floating: false, column: 2, order: 0, saved_pos: None, saved_size: None };

        self.show_bandmap_window = false;
        self.show_satellites_window = false;
        self.show_world_map_window = false;
        self.reset_layout_requested = true;
        // Natychmiast odtwórz dokowany układ z domyślnej konfiguracji paneli.
        self.rebuild_dock_state_from_panels();
        self.last_saved_dock_layout = self.serialize_dock_layout();
    }

    pub fn get_tiles_in_column(&self, col: usize) -> Vec<String> {
        let mut items = Vec::new();
        if self.panel_vfo.column == col && self.panel_vfo.visible && !self.panel_vfo.floating { items.push(("vfo".to_string(), self.panel_vfo.order)); }
        if self.panel_qso.column == col && self.panel_qso.visible && !self.panel_qso.floating { items.push(("qso".to_string(), self.panel_qso.order)); }
        if self.panel_log.column == col && self.panel_log.visible && !self.panel_log.floating { items.push(("log".to_string(), self.panel_log.order)); }
        if self.panel_cluster.column == col && self.panel_cluster.visible && !self.panel_cluster.floating { items.push(("cluster".to_string(), self.panel_cluster.order)); }
        if self.panel_bandmap.column == col && self.panel_bandmap.visible && !self.panel_bandmap.floating { items.push(("bandmap".to_string(), self.panel_bandmap.order)); }
        if self.panel_solar.column == col && self.panel_solar.visible && !self.panel_solar.floating { items.push(("solar".to_string(), self.panel_solar.order)); }
        if self.panel_satellites.column == col && self.panel_satellites.visible && !self.panel_satellites.floating { items.push(("satellites".to_string(), self.panel_satellites.order)); }
        if self.panel_world_map.column == col && self.panel_world_map.visible && !self.panel_world_map.floating { items.push(("world_map".to_string(), self.panel_world_map.order)); }
        items.sort_by_key(|(_, ord)| *ord);
        items.into_iter().map(|(id, _)| id).collect()
    }

    pub fn set_tile_order(&mut self, tile_id: &str, order: usize) {
        match tile_id {
            "vfo" => self.panel_vfo.order = order,
            "qso" => self.panel_qso.order = order,
            "log" => self.panel_log.order = order,
            "cluster" => self.panel_cluster.order = order,
            "bandmap" => self.panel_bandmap.order = order,
            "solar" => self.panel_solar.order = order,
            "satellites" => self.panel_satellites.order = order,
            "world_map" => self.panel_world_map.order = order,
            _ => {}
        }
    }

    pub fn move_tile_column(&mut self, tile_id: &str, delta: i32) {
        let cur_col = match tile_id {
            "vfo" => self.panel_vfo.column,
            "qso" => self.panel_qso.column,
            "log" => self.panel_log.column,
            "cluster" => self.panel_cluster.column,
            "bandmap" => self.panel_bandmap.column,
            "solar" => self.panel_solar.column,
            "satellites" => self.panel_satellites.column,
            "world_map" => self.panel_world_map.column,
            _ => return,
        };
        let new_col = (cur_col as i32 + delta).clamp(0, 2) as usize;
        if new_col != cur_col {
            self.move_tile_to(tile_id, new_col, 999);
        }
    }

    pub fn move_tile_order(&mut self, tile_id: &str, delta: i32) {
        let col = match tile_id {
            "vfo" => self.panel_vfo.column,
            "qso" => self.panel_qso.column,
            "log" => self.panel_log.column,
            "cluster" => self.panel_cluster.column,
            "bandmap" => self.panel_bandmap.column,
            "solar" => self.panel_solar.column,
            "satellites" => self.panel_satellites.column,
            "world_map" => self.panel_world_map.column,
            _ => return,
        };
        let mut tiles = self.get_tiles_in_column(col);
        if let Some(pos) = tiles.iter().position(|id| id == tile_id) {
            let new_pos = (pos as i32 + delta).clamp(0, (tiles.len().saturating_sub(1)) as i32) as usize;
            if new_pos != pos {
                tiles.swap(pos, new_pos);
                for (idx, id) in tiles.into_iter().enumerate() {
                    self.set_tile_order(&id, idx);
                }
                self.save_station_config();
            }
        }
    }

    pub fn move_tile_to(&mut self, tile_id: &str, target_col: usize, target_order: usize) {
        let mut dest_tiles = self.get_tiles_in_column(target_col);
        dest_tiles.retain(|id| id != tile_id);
        let insert_idx = target_order.min(dest_tiles.len());
        dest_tiles.insert(insert_idx, tile_id.to_string());

        let col = target_col.min(2);
        match tile_id {
            "vfo" => self.panel_vfo.column = col,
            "qso" => self.panel_qso.column = col,
            "log" => self.panel_log.column = col,
            "cluster" => self.panel_cluster.column = col,
            "bandmap" => self.panel_bandmap.column = col,
            "solar" => self.panel_solar.column = col,
            "satellites" => self.panel_satellites.column = col,
            "world_map" => self.panel_world_map.column = col,
            _ => {}
        }

        for (idx, id) in dest_tiles.into_iter().enumerate() {
            self.set_tile_order(&id, idx);
        }
        self.normalize_tile_orders();
        self.save_station_config();
    }

    pub fn normalize_tile_orders(&mut self) {
        for col in 0..=2 {
            let mut items: Vec<(&str, usize)> = Vec::new();
            if self.panel_vfo.column == col && self.panel_vfo.visible && !self.panel_vfo.floating { items.push(("vfo", self.panel_vfo.order)); }
            if self.panel_qso.column == col && self.panel_qso.visible && !self.panel_qso.floating { items.push(("qso", self.panel_qso.order)); }
            if self.panel_log.column == col && self.panel_log.visible && !self.panel_log.floating { items.push(("log", self.panel_log.order)); }
            if self.panel_cluster.column == col && self.panel_cluster.visible && !self.panel_cluster.floating { items.push(("cluster", self.panel_cluster.order)); }
            if self.panel_bandmap.column == col && self.panel_bandmap.visible && !self.panel_bandmap.floating { items.push(("bandmap", self.panel_bandmap.order)); }
            if self.panel_solar.column == col && self.panel_solar.visible && !self.panel_solar.floating { items.push(("solar", self.panel_solar.order)); }
            if self.panel_satellites.column == col && self.panel_satellites.visible && !self.panel_satellites.floating { items.push(("satellites", self.panel_satellites.order)); }
            if self.panel_world_map.column == col && self.panel_world_map.visible && !self.panel_world_map.floating { items.push(("world_map", self.panel_world_map.order)); }

            items.sort_by_key(|(_, order)| *order);
            for (new_order, (id, _)) in items.into_iter().enumerate() {
                self.set_tile_order(id, new_order);
            }
        }
    }

    pub fn popout_tile(&mut self, tile_id: &str) {
        match tile_id {
            "vfo" => self.panel_vfo.floating = true,
            "qso" => self.panel_qso.floating = true,
            "log" => self.panel_log.floating = true,
            "cluster" => self.panel_cluster.floating = true,
            "bandmap" => { self.panel_bandmap.floating = true; self.show_bandmap_window = true; }
            "solar" => self.panel_solar.floating = true,
            "satellites" => { self.panel_satellites.floating = true; self.show_satellites_window = true; }
            "world_map" => { self.panel_world_map.floating = true; self.show_world_map_window = true; }
            _ => {}
        }
    }

    pub fn close_tile(&mut self, tile_id: &str) {
        match tile_id {
            "vfo" => self.panel_vfo.visible = false,
            "qso" => self.panel_qso.visible = false,
            "log" => self.panel_log.visible = false,
            "cluster" => self.panel_cluster.visible = false,
            "bandmap" => { self.panel_bandmap.visible = false; self.show_bandmap_window = false; }
            "solar" => self.panel_solar.visible = false,
            "satellites" => { self.panel_satellites.visible = false; self.show_satellites_window = false; }
            "world_map" => { self.panel_world_map.visible = false; self.show_world_map_window = false; }
            _ => {}
        }
    }

    pub fn tile_title(&self, tile_id: &str) -> String {
        let lang = self.current_language;
        match tile_id {
            "vfo" => icons::RADIO.label("TRANSCEIVER VFO"),
            "qso" => icons::NEW_QSO.label(tr("tab.new_qso", lang)),
            "log" => icons::LOGBOOK.label(tr("tab.logbook", lang)),
            "cluster" => icons::CLUSTER.label(tr("cluster.title", lang)),
            "bandmap" => icons::BANDMAP.label(tr("bandmap.title", lang)),
            "solar" => icons::SOLAR.label(tr("solar.title", lang)),
            "satellites" => icons::SATELLITE.label("ŚLEDZENIE SATELITÓW"),
            "world_map" => icons::WORLD_MAP.label(tr("map.world_title", lang)),
            _ => tile_id.to_string(),
        }
    }

    pub fn render_tile_header_custom(&mut self, tile_id: &str, ui: &mut egui::Ui) {
        match tile_id {
            "vfo" => {
                if self.vfo_split {
                    ui.label(egui::RichText::new("SPLIT ON").color(egui::Color32::from_rgb(239, 68, 68)).strong().size(11.0));
                } else {
                    ui.label(egui::RichText::new("SPLIT OFF").color(egui::Color32::from_rgb(100, 116, 139)).size(11.0));
                }
            }
            "qso" => {
                if self.cat_connected {
                    if ui.button(egui::RichText::new("● CAT ONLINE").color(egui::Color32::from_rgb(34, 197, 94)).size(11.0).strong()).clicked() {
                        self.show_cat_settings_window = true;
                    }
                } else {
                    if ui.button(egui::RichText::new("○ CAT OFFLINE").color(egui::Color32::from_rgb(148, 163, 184)).size(11.0)).clicked() {
                        self.show_cat_settings_window = true;
                    }
                }
            }
            "log" => {
                ui.label(egui::RichText::new(format!("({} QSO)", self.recent_qsos.len())).size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                let lang = self.current_language;
                ui.add(egui::TextEdit::singleline(&mut self.log_search_query).hint_text(tr("qso.search", lang)).desired_width(110.0));
                if ui.button("🔄").on_hover_text(tr("btn.refresh", lang)).clicked() {
                    self.reload_qsos();
                }
            }
            "cluster" => {
                if self.cluster_connected {
                    ui.label(egui::RichText::new("● ONLINE").color(egui::Color32::from_rgb(34, 197, 94)).size(10.0).strong());
                } else if self.cluster_connecting {
                    ui.label(egui::RichText::new("● ŁĄCZENIE...").color(egui::Color32::from_rgb(250, 204, 21)).size(10.0).strong());
                } else {
                    ui.label(egui::RichText::new("○ OFFLINE").color(egui::Color32::from_rgb(148, 163, 184)).size(10.0));
                }
                if ui.button(egui::RichText::new("📢 Spot").strong().color(egui::Color32::from_rgb(56, 189, 248)))
                    .on_hover_text("Wyślij spot DX do klastra Telnet")
                    .clicked()
                {
                    let freq_khz = self.rig_state.frequency_hz as f64 / 1000.0;
                    self.send_spot_dialog.open_with(&self.entry_callsign, freq_khz);
                }
            }
            _ => {}
        }
    }

    pub fn render_tile_body(&mut self, tile_id: &str, ui: &mut egui::Ui) {
        match tile_id {
            "vfo" => crate::gui::vfo_panel::render_vfo_body(self, ui),
            "qso" => crate::gui::qso_entry::render_qso_entry_body(self, ui),
            "log" => crate::gui::logbook_table::render_logbook_body(self, ui),
            "cluster" => crate::gui::cluster_panel::render_cluster_body(self, ui),
            "bandmap" => crate::gui::bandmap::render_bandmap_content(self, ui),
            "solar" => crate::gui::solar_panel::render_solar_body(self, ui),
            "satellites" => crate::gui::satellites::render_satellites_content(self, ui),
            "world_map" => crate::gui::world_map::render_world_map_content(self, ui),
            _ => {}
        }
    }

    pub fn render_tiles_in_column(&mut self, ui: &mut egui::Ui, col_idx: usize, tiles: &[String]) {
        let is_dragging = self.dragging_tile.is_some();
        let col_name = match col_idx {
            0 => "Lewa",
            1 => "Środek",
            _ => "Prawa",
        };

        if tiles.is_empty() {
            if is_dragging {
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().max(40.0), 160.0),
                    egui::Sense::hover(),
                );
                let hovered = resp.hovered();
                let border_color = if hovered {
                    egui::Color32::from_rgb(56, 189, 248)
                } else {
                    egui::Color32::from_rgba_unmultiplied(56, 189, 248, 80)
                };
                ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(if hovered { 2.5_f32 } else { 1.5_f32 }, border_color));
                let fill = if hovered {
                    egui::Color32::from_rgba_unmultiplied(56, 189, 248, 30)
                } else {
                    egui::Color32::from_rgba_unmultiplied(30, 41, 59, 120)
                };
                ui.painter().rect_filled(rect, 8.0, fill);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("➕ Upuść kafelek tutaj\n(Kolumna: {})", col_name),
                    egui::FontId::proportional(12.0),
                    if hovered { egui::Color32::WHITE } else { egui::Color32::from_rgb(148, 163, 184) },
                );
                if hovered && ui.input(|i| i.pointer.any_released()) {
                    if let Some(dragged) = self.dragging_tile.take() {
                        self.move_tile_to(&dragged, col_idx, 0);
                    }
                }
            }
            return;
        }

        let mut action_move_col: Option<(String, i32)> = None;
        let mut action_move_order: Option<(String, i32)> = None;
        let mut action_popout: Option<String> = None;
        let mut action_close: Option<String> = None;
        let mut start_drag: Option<String> = None;

        for (idx, tile_id) in tiles.iter().enumerate() {
            // Drop slot przed kafelkiem w trakcie przeciągania myszą
            if is_dragging {
                let (slot_rect, slot_resp) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 16.0),
                    egui::Sense::hover(),
                );
                let hovered = slot_resp.hovered();
                if hovered {
                    ui.painter().rect_filled(
                        slot_rect,
                        3.0,
                        egui::Color32::from_rgba_unmultiplied(56, 189, 248, 180),
                    );
                    ui.painter().text(
                        slot_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "⬇ Upuść tutaj",
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );
                    if ui.input(|i| i.pointer.any_released()) {
                        if let Some(dragged) = self.dragging_tile.take() {
                            self.move_tile_to(&dragged, col_idx, idx);
                        }
                    }
                } else {
                    ui.painter().line_segment(
                        [egui::pos2(slot_rect.left() + 20.0, slot_rect.center().y), egui::pos2(slot_rect.right() - 20.0, slot_rect.center().y)],
                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(56, 189, 248, 40)),
                    );
                }
            }

            // Karta kafelka z obramowaniem i nagłówkiem
            let is_this_dragged = self.dragging_tile.as_deref() == Some(tile_id.as_str());
            let border_stroke = if is_this_dragged {
                egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(56, 189, 248))
            } else {
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(51, 65, 85))
            };

            egui::Frame::group(ui.style())
                .rounding(6.0)
                .inner_margin(egui::Margin::same(8.0))
                .stroke(border_stroke)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Nagłówek kafelka z uchwytem przeciągania i przyciskami
                        ui.horizontal(|ui| {
                            // Uchwyt przeciągania myszką (Drag & Drop handle)
                            let handle_label = egui::RichText::new("⠿").size(16.0).color(egui::Color32::from_rgb(148, 163, 184)).strong();
                            let handle_resp = ui.add(egui::Label::new(handle_label).sense(egui::Sense::drag()))
                                .on_hover_cursor(egui::CursorIcon::Grab)
                                .on_hover_text("Przeciągnij myszą, aby przenieść ten kafelek do innej kolumny lub pozycji");

                            if handle_resp.drag_started() || handle_resp.dragged() {
                                start_drag = Some(tile_id.clone());
                            }

                            // Tytuł kafelka
                            let title_text = egui::RichText::new(self.tile_title(tile_id))
                                .color(egui::Color32::from_rgb(56, 189, 248))
                                .strong()
                                .size(13.0);
                            ui.label(title_text);

                            if is_this_dragged {
                                ui.label(egui::RichText::new("[Przenoszenie...]").size(10.0).color(egui::Color32::from_rgb(56, 189, 248)));
                            }

                            // Własne widżety nagłówka (CAT, SPLIT, Szukaj, Spot)
                            self.render_tile_header_custom(tile_id, ui);

                            // Przyciski przestawiania i zamykania (wyrównane do prawej)
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✕").on_hover_text("Ukryj ten kafelek").clicked() {
                                    action_close = Some(tile_id.clone());
                                }
                                if ui.button("↗").on_hover_text("Odepnij do osobnego okna pływającego").clicked() {
                                    action_popout = Some(tile_id.clone());
                                }

                                if idx + 1 < tiles.len()
                                    && ui.button("▼").on_hover_text("Przesuń niżej").clicked() {
                                        action_move_order = Some((tile_id.clone(), 1));
                                    }
                                if idx > 0
                                    && ui.button("▲").on_hover_text("Przesuń wyżej").clicked() {
                                        action_move_order = Some((tile_id.clone(), -1));
                                    }

                                if col_idx < 2
                                    && ui.button("▶").on_hover_text("Przenieś do kolumny po prawej").clicked() {
                                        action_move_col = Some((tile_id.clone(), 1));
                                    }
                                if col_idx > 0
                                    && ui.button("◀").on_hover_text("Przenieś do kolumny po lewej").clicked() {
                                        action_move_col = Some((tile_id.clone(), -1));
                                    }
                            });
                        });

                        ui.separator();

                        // Ciało / zawartość modułu
                        self.render_tile_body(tile_id, ui);
                    });
                });

            ui.add_space(8.0);
        }

        // Drop slot na samym końcu kolumny
        if is_dragging {
            let (slot_rect, slot_resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 20.0),
                egui::Sense::hover(),
            );
            let hovered = slot_resp.hovered();
            if hovered {
                ui.painter().rect_filled(
                    slot_rect,
                    3.0,
                    egui::Color32::from_rgba_unmultiplied(56, 189, 248, 180),
                );
                ui.painter().text(
                    slot_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⬇ Upuść tutaj (na końcu)",
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
                if ui.input(|i| i.pointer.any_released()) {
                    if let Some(dragged) = self.dragging_tile.take() {
                        self.move_tile_to(&dragged, col_idx, tiles.len());
                    }
                }
            } else {
                ui.painter().line_segment(
                    [egui::pos2(slot_rect.left() + 20.0, slot_rect.center().y), egui::pos2(slot_rect.right() - 20.0, slot_rect.center().y)],
                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(56, 189, 248, 40)),
                );
            }
        }

        // Wykonanie odłożonych akcji (unikanie konfliktów borrow-checkera w trakcie pętli)
        if let Some(id) = start_drag {
            self.dragging_tile = Some(id);
        }
        if let Some((id, delta)) = action_move_col {
            self.move_tile_column(&id, delta);
        }
        if let Some((id, delta)) = action_move_order {
            self.move_tile_order(&id, delta);
        }
        if let Some(id) = action_popout {
            self.popout_tile(&id);
        }
        if let Some(id) = action_close {
            self.close_tile(&id);
        }
    }

    /// Tryb zakładek: każda kolumna pokazuje pasek zakładek, a pod nim tylko
    /// jedną aktywną kartę (nagłówek + ciało). Zachowuje akcje odpinania,
    /// zamykania i przenoszenia między kolumnami.
    pub fn render_tiles_in_column_tabbed(&mut self, ui: &mut egui::Ui, col_idx: usize, tiles: &[String]) {
        if tiles.is_empty() {
            return;
        }

        let active = self.active_tab[col_idx].min(tiles.len() - 1);
        self.active_tab[col_idx] = active;

        // Pasek zakładek
        ui.horizontal_wrapped(|ui| {
            for (idx, tile_id) in tiles.iter().enumerate() {
                let selected = idx == active;
                let title = egui::RichText::new(self.tile_title(tile_id)).size(12.0).strong();
                if ui.selectable_label(selected, title).clicked() {
                    self.active_tab[col_idx] = idx;
                }
            }
        });
        ui.add_space(4.0);

        let tile_id = tiles[active].clone();
        let mut action_popout: Option<String> = None;
        let mut action_close: Option<String> = None;
        let mut action_move_col: Option<(String, i32)> = None;
        let mut action_move_order: Option<(String, i32)> = None;

        egui::Frame::group(ui.style())
            .rounding(6.0)
            .inner_margin(egui::Margin::same(8.0))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(51, 65, 85)))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let title_text = egui::RichText::new(self.tile_title(&tile_id))
                            .color(egui::Color32::from_rgb(56, 189, 248))
                            .strong()
                            .size(13.0);
                        ui.label(title_text);
                        self.render_tile_header_custom(&tile_id, ui);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕").on_hover_text("Ukryj ten kafelek").clicked() {
                                action_close = Some(tile_id.clone());
                            }
                            if ui.button("↗").on_hover_text("Odepnij do osobnego okna pływającego").clicked() {
                                action_popout = Some(tile_id.clone());
                            }
                            if active + 1 < tiles.len()
                                && ui.button("▼").on_hover_text("Przesuń niżej").clicked() {
                                action_move_order = Some((tile_id.clone(), 1));
                            }
                            if active > 0
                                && ui.button("▲").on_hover_text("Przesuń wyżej").clicked() {
                                action_move_order = Some((tile_id.clone(), -1));
                            }
                            if col_idx < 2
                                && ui.button("▶").on_hover_text("Przenieś do kolumny po prawej").clicked() {
                                action_move_col = Some((tile_id.clone(), 1));
                            }
                            if col_idx > 0
                                && ui.button("◀").on_hover_text("Przenieś do kolumny po lewej").clicked() {
                                action_move_col = Some((tile_id.clone(), -1));
                            }
                        });
                    });

                    ui.separator();
                    self.render_tile_body(&tile_id, ui);
                });
            });

        if let Some((id, delta)) = action_move_col {
            self.move_tile_column(&id, delta);
        }
        if let Some((id, delta)) = action_move_order {
            self.move_tile_order(&id, delta);
        }
        if let Some(id) = action_popout {
            self.popout_tile(&id);
        }
        if let Some(id) = action_close {
            self.close_tile(&id);
        }
    }

    /// Typ backendu CAT wybrany w konfiguracji (mapowanie `cat_backend`).
    pub fn cat_backend_kind(&self) -> crate::cat::backend::CatBackendKind {
        crate::cat::backend::CatBackendKind::from_str(&self.cat_backend)
    }

    pub fn start_cat_service(&mut self) {
        if self.cat_auto_start_rigctld {
            let mut sup = crate::cat::supervisor::RigctldSupervisor::new(
                self.cat_port,
                self.cat_rig_id,
                &self.cat_serial_port,
                self.cat_baud_rate,
                &self.cat_hamlib_source,
                &self.cat_custom_rigctld_path,
            );
            match sup.start() {
                Ok(_) => {
                    self.rigctld_supervisor = Some(sup);
                    self.cat_test_result = Some(format!("Uruchomiono rigctld w tle dla {}!", self.cat_rig_model));
                }
                Err(e) => {
                    self.cat_test_result = Some(format!("Błąd uruchomienia rigctld: {}", e));
                }
            }
        }

        let host = self.cat_host.clone();
        let port = self.cat_port;
        let poll_rate = self.cat_poll_rate_ms;
        let cat_sender = self.cat_state_tx.clone();
        let backend = self.cat_backend_kind();

        match backend {
            crate::cat::backend::CatBackendKind::Hamlib => {
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    let (client, mut rx) = crate::cat::hamlib::HamlibClient::new(&host, port);
                    tokio::spawn(async move {
                        client.run_poll_loop(poll_rate).await;
                    });
                    while let Ok(st) = rx.recv().await {
                        let _ = cat_sender.send(st);
                    }
                });
            }
            crate::cat::backend::CatBackendKind::Flrig => {
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    let mut client = crate::cat::flrig::FlrigClient::new(&host, port);
                    loop {
                        let state = match crate::cat::backend::CatBackend::poll_state(&mut client).await {
                            Ok(st) => st,
                            Err(_) => crate::cat::hamlib::RigState { connected: false, ..Default::default() },
                        };
                        if cat_sender.send(state).is_err() {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(poll_rate)).await;
                    }
                });
            }
            other => {
                self.cat_test_result = Some(format!(
                    "Backend {} nie jest jeszcze podłączony do pętli odpytywania.",
                    other.label()
                ));
            }
        }
    }

    pub fn stop_cat_service(&mut self) {
        self.cat_connected = false;
        self.rig_state.connected = false;
        if let Some(mut sup) = self.rigctld_supervisor.take() {
            sup.stop();
        }
        self.cat_test_result = Some("Zatrzymano CAT i wyłączono proces rigctld.".to_string());
    }

    pub fn set_vfo_frequency(&mut self, freq_hz: u64) {
        self.rig_state.frequency_hz = freq_hz;
        if let Some(band_def) = crate::core::bandplan::get_band_by_freq(freq_hz) {
            self.entry_band = band_def.name.to_string();
        }
        if self.cat_connected {
            let host = self.cat_host.clone();
            let port = self.cat_port;
            tokio::spawn(async move {
                let _ = crate::cat::hamlib::HamlibClient::set_frequency(&host, port, freq_hz).await;
            });
        }
        self.status_message = Some(format!("VFO dostrojone do: {:.3} kHz", (freq_hz as f64) / 1000.0));
    }

    pub fn step_vfo(&mut self, step_hz: i64) {
        let current = self.rig_state.frequency_hz as i64;
        let next = (current + step_hz).max(100_000) as u64;
        self.set_vfo_frequency(next);
    }

    pub fn set_vfo_mode(&mut self, mode: &str) {
        self.rig_state.mode = mode.to_string();
        self.entry_mode = mode.to_string();
        if self.cat_connected {
            let host = self.cat_host.clone();
            let port = self.cat_port;
            let m = mode.to_string();
            tokio::spawn(async move {
                let _ = crate::cat::hamlib::HamlibClient::set_mode(&host, port, &m, 0).await;
            });
        }
        self.status_message = Some(format!("Emisja zmieniona na: {}", mode));
    }

    pub fn add_new_equipment(&mut self) {
        let item = EquipmentItem {
            id: format!("eq-{}", self.equipment_items.len() + 1),
            category: self.new_eq_cat,
            manufacturer: self.new_eq_mfr.trim().to_string(),
            model: self.new_eq_model.trim().to_string(),
            serial_number: if self.new_eq_sn.trim().is_empty() { None } else { Some(self.new_eq_sn.trim().to_string()) },
            purchase_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            notes: if self.new_eq_notes.trim().is_empty() { None } else { Some(self.new_eq_notes.trim().to_string()) },
        };
        self.equipment_items.push(item);
        self.new_eq_model.clear();
        self.new_eq_sn.clear();
        self.new_eq_notes.clear();
        self.save_station_config();
        self.status_message = Some("Dodano nowy sprzęt do ewidencji.".to_string());
    }

    pub fn save_edited_equipment(&mut self, id: &str) {
        if let Some(item) = self.equipment_items.iter_mut().find(|i| i.id == id) {
            item.category = self.edit_eq_cat;
            item.manufacturer = self.edit_eq_mfr.trim().to_string();
            item.model = self.edit_eq_model.trim().to_string();
            item.serial_number = if self.edit_eq_sn.trim().is_empty() { None } else { Some(self.edit_eq_sn.trim().to_string()) };
            item.purchase_date = if self.edit_eq_date.trim().is_empty() { None } else { Some(self.edit_eq_date.trim().to_string()) };
            item.notes = if self.edit_eq_notes.trim().is_empty() { None } else { Some(self.edit_eq_notes.trim().to_string()) };
        }
        self.editing_equipment_id = None;
        self.save_station_config();
        self.status_message = Some("Zaktualizowano dane sprzętu w ewidencji.".to_string());
    }

    pub fn delete_equipment_item(&mut self, id: &str) {
        self.equipment_items.retain(|i| i.id != id);
        if self.editing_equipment_id.as_deref() == Some(id) {
            self.editing_equipment_id = None;
        }
        self.save_station_config();
        self.status_message = Some("Pozycja sprzętu została usunięta z ewidencji.".to_string());
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
                    let report = crate::core::adif::parse_adif_with_report(&content);
                    let qsos = report.qsos;
                    let count = qsos.len();
                    let rejected = report.rejected;
                    let errors = report.errors;
                    let insert_res = {
                        let mut db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                        db.batch_insert_qsos(&qsos)
                    };
                    match insert_res {
                        Ok(_) => {
                            self.rebuild_awards_full();
                            self.reload_qsos();
                            let mut msg = format!(
                                "Zaimportowano {} łączności, odrzucono {} z pliku: {}",
                                count,
                                rejected,
                                path.display()
                            );
                            if let Some(first_error) = errors.first() {
                                if rejected > 0 || !errors.is_empty() {
                                    msg.push_str(&format!(" | {}", first_error));
                                }
                            }
                            self.status_message = Some(msg);
                            self.status_toast = Some((format!("Zaimportowano {} QSO (odrzucono {})", count, rejected), std::time::Instant::now()));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Błąd zapisu łączności do bazy: {}", e));
                            self.report_error(format!("Błąd importu do bazy: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.status_message = Some(format!("Błąd odczytu pliku {}: {}", path.display(), e));
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
                let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                db.get_recent_qsos(100000).unwrap_or_default()
            };
            let count = qsos.len();
            let adif_text = crate::core::adif::export_adif(&qsos, "SPLogbook", &self.my_station.callsign);
            match std::fs::write(&path, adif_text) {
                Ok(_) => {
                    self.status_message = Some(format!("Wyeksportowano pomyślnie {} łączności do pliku: {}", count, path.display()));
                }
                Err(e) => {
                    self.status_message = Some(format!("Błąd zapisu pliku {}: {}", path.display(), e));
                }
            }
        }
    }

    pub fn run_manual_backup(&mut self) {
        let backup_dir = std::path::Path::new("backups");
        let _ = std::fs::create_dir_all(backup_dir);
        let backup_res = BackupManager::backup_database(&self.active_db_path, backup_dir);
        
        let qsos = {
            let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
            db.get_recent_qsos(10000).unwrap_or_default()
        };
        let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &self.my_station.callsign);
        let _ = BackupManager::backup_adif(&adif, backup_dir);
        
        match backup_res {
            Ok(backed_path) => {
                let name = backed_path.file_name().unwrap_or_default().to_string_lossy();
                self.status_toast = Some((format!("Wykonano kopię zapasową: {} w folderze backups/!", name), std::time::Instant::now()));
            }
            Err(e) => {
                self.report_error(format!("Błąd tworzenia kopii zapasowej: {}", e));
            }
        }
    }

    pub fn trigger_database_update(&mut self) {
        tokio::spawn(async move {
            let res = crate::cloud::updater::DatabaseUpdater::update_all(std::path::Path::new("databases")).await;
            println!("Database update completed: {:?}", res);
        });
        self.status_toast = Some(("Rozpoczęto pobieranie aktualizacji baz danych (cty.dat, SCP, LoTW) w tle...".to_string(), std::time::Instant::now()));
    }

    /// Sprawdza najnowsze wydanie na GitHub w tle i zapisuje wynik do odbiornika.
    pub fn trigger_update_check(&mut self) {
        self.update_check_status = Some("Sprawdzanie najnowszej wersji na GitHub…".to_string());
        self.update_available = None;
        let (tx, rx) = std::sync::mpsc::channel::<crate::cloud::updater::UpdateCheckOutcome>();
        self.update_check_rx = Some(rx);
        tokio::spawn(async move {
            let outcome = match crate::cloud::updater::latest_release().await {
                Ok(release) => {
                    let local = env!("CARGO_PKG_VERSION");
                    if release.tag == local {
                        crate::cloud::updater::UpdateCheckOutcome::UpToDate { local: local.to_string() }
                    } else {
                        crate::cloud::updater::UpdateCheckOutcome::NewVersion(release)
                    }
                }
                Err(e) => crate::cloud::updater::UpdateCheckOutcome::Error(e),
            };
            let _ = tx.send(outcome);
        });
    }

    /// Pobiera i instaluje dostępną aktualizację w tle (self-replace + restart).
    pub fn trigger_update_install(&mut self) {
        let Some(release) = self.update_available.clone() else {
            return;
        };
        let Some(asset) = crate::cloud::updater::select_asset_for_platform(&release.assets).cloned() else {
            self.update_install_status =
                Some("❌ Wydanie nie zawiera pliku instalacyjnego dla tego systemu.".to_string());
            return;
        };

        self.update_install_status = Some(format!(
            "Pobieranie aktualizacji do v{} ({}: {})…",
            release.tag,
            asset.name,
            if asset.size > 0 {
                format!("{:.1} MB", asset.size as f64 / 1_048_576.0)
            } else {
                "?".to_string()
            }
        ));

        let (tx, rx) = std::sync::mpsc::channel::<String>();
        self.update_install_rx = Some(rx);
        tokio::spawn(async move {
            let msg = match crate::cloud::updater::install_update(&asset).await {
                Ok(()) => "✅ Aktualizacja pobrana i zweryfikowana. Aplikacja zostanie zamknięta i uruchomiona ponownie…".to_string(),
                Err(e) => format!("❌ Instalacja nie powiodła się: {}", e),
            };
            let _ = tx.send(msg);
        });
    }

    pub fn generate_qsl_sheet(&mut self) {
        self.status_message = Some("Arkusz etykiet QSL (A4) wygenerowany gotowy do wydruku.".to_string());
    }

    pub fn render_column1(&mut self, ui: &mut egui::Ui) {
        let tiles = self.get_tiles_in_column(0);
        self.render_tiles_in_column(ui, 0, &tiles);
    }

    pub fn render_column2(&mut self, ui: &mut egui::Ui) {
        let tiles = self.get_tiles_in_column(1);
        self.render_tiles_in_column(ui, 1, &tiles);
    }

    pub fn render_column3(&mut self, ui: &mut egui::Ui) {
        let tiles = self.get_tiles_in_column(2);
        self.render_tiles_in_column(ui, 2, &tiles);
    }

    pub fn toggle_cat_proxy_server(&mut self) {
        if self.cat_sharing_enabled {
            if let Some(srv) = self.cat_proxy_server.take() {
                srv.stop();
            }
            let (srv, rx) = crate::cat::server::HamlibProxyServer::new(self.cat_sharing_port, self.cat_shared_state.clone());
            let srv_arc = std::sync::Arc::new(srv);
            self.cat_proxy_server = Some(srv_arc.clone());
            self.cat_proxy_rx = Some(rx);
            self.cat_sharing_active = true;

            tokio::spawn(async move {
                if let Err(e) = srv_arc.run().await {
                    eprintln!("Hamlib proxy server error: {}", e);
                }
            });
        } else {
            if let Some(srv) = self.cat_proxy_server.take() {
                srv.stop();
            }
            self.cat_proxy_rx = None;
            self.cat_sharing_active = false;
        }
    }

    pub fn activate_station_profile(&mut self, profile_id: &str) {
        if let Some(prof) = self.station_profiles.iter().find(|p| p.id == profile_id).cloned() {
            self.active_profile_id = prof.id.clone();
            self.my_station = prof.clone();
            self.entry_pota = prof.pota_ref.clone().unwrap_or_default();
            self.entry_sota = prof.sota_ref.clone().unwrap_or_default();
            self.save_station_config();
            self.status_toast = Some((
                format!("Przełączono aktywny profil stacji na: {}", if prof.name.is_empty() { &prof.callsign } else { &prof.name }),
                std::time::Instant::now(),
            ));
        }
    }

    pub fn refresh_qso_list(&mut self) {
        self.reload_qsos();
    }

    /// Obsługuje wspólną kolejkę wysyłki: pobiera gotowe zadania i uruchamia
    /// ich wysyłkę w tle, a po zakończeniu zapisuje wynik z powrotem do kolejki.
    pub fn poll_upload_scheduler(&mut self) {
        use crate::cloud::scheduler::UploadService;

        let now = crate::cloud::scheduler::now_unix();
        let creds = crate::cloud::scheduler::UploadCredentials {
            clublog_callsign: self.clublog_callsign.clone(),
            clublog_email: self.clublog_email.clone(),
            clublog_password: self.clublog_password.clone(),
            clublog_api_key: self.clublog_api_key.clone(),
            qrz_api_key: self.qrz_api_key.clone(),
            eqsl_username: self.eqsl_username.clone(),
            eqsl_password: self.eqsl_password.clone(),
        };

        // Pobierz gotowe zadania (maks. jedno na serwis, z uwzględnieniem rate-limit).
        let ready: Vec<(u64, UploadService, String)> = {
            let mut sched = self.upload_scheduler.lock().unwrap();
            let ids = sched.ready_jobs(now);
            ids.into_iter()
                .filter_map(|id| {
                    sched.mark_in_flight(id, now);
                    let job = sched.job(id)?;
                    Some((id, job.service, job.adif.clone()))
                })
                .collect()
        };

        if ready.is_empty() {
            return;
        }

        let sched = self.upload_scheduler.clone();
        tokio::spawn(async move {
            for (id, service, adif) in ready {
                let result = crate::cloud::scheduler::execute_upload(service, &creds, &adif).await;
                let now = crate::cloud::scheduler::now_unix();
                let mut sched = sched.lock().unwrap();
                match result {
                    Ok(_) => sched.mark_success(id, now),
                    Err(e) => {
                        sched.mark_failure(id, e, now);
                    }
                }
                sched.save_to_disk();
            }
            // Po zakończeniu serii utrzymaj kolejkę w rozsądnym rozmiarze.
            let mut sched = sched.lock().unwrap();
            if sched.purge_done() > 0 {
                sched.save_to_disk();
            }
        });
    }

    /// Odświeża pozycję satelity i listę najbliższych przelotów (przybliżenie keplerowskie).
    /// Wywoływane co klatkę, ale przeliczane co najwyżej raz na minutę.
    pub fn refresh_satellite_tracking(&mut self) {
        let now = crate::cloud::scheduler::now_unix() as f64;
        let minute = (now / 60.0) as u64;
        let key = format!(
            "{}|{}|{}",
            self.selected_satellite, self.my_station.gridsquare, minute
        );
        if self.sat_passes_key == key {
            return;
        }
        self.sat_passes_key = key;

        let def = crate::core::satellite::default_satellites()
            .into_iter()
            .find(|d| d.name == self.selected_satellite);

        let coords = locator_to_coordinates(&self.my_station.gridsquare)
            .unwrap_or(crate::core::geo::Coordinates::new(52.2297, 21.0122));
        let obs = crate::core::satellite::Observer {
            lat_deg: coords.latitude,
            lon_deg: coords.longitude,
        };

        if let Some(def) = def {
            self.sat_passes = crate::core::satellite::predict_passes(&def, &obs, now, 10.0, 24.0);
            self.sat_azimuth = crate::core::satellite::azimuth_deg(&def, &obs, now) as f32;
            self.sat_elevation = crate::core::satellite::elevation_deg(&def, &obs, now) as f32;
            self.sat_range_km = crate::core::satellite::range_km(&def, &obs, now) as f32;
            self.sat_altitude_km = def.altitude_km as f32;
            self.sat_downlink_mhz = def.downlink_mhz;
            self.sat_uplink_mhz = def.uplink_mhz;
            self.sat_rx_doppler_khz =
                crate::core::satellite::doppler_khz(&def, &obs, now, def.downlink_mhz) as f32;
            self.sat_tx_doppler_khz =
                crate::core::satellite::doppler_khz(&def, &obs, now, def.uplink_mhz) as f32;
        } else {
            self.sat_passes.clear();
        }
    }

    /// Uruchamia asynchroniczne pobranie warunków solarnych (HamQTH).
    pub fn refresh_solar_weather(&mut self, ctx: &egui::Context) {
        if self.solar_loading {
            return;
        }
        self.solar_loading = true;

        let slot: std::sync::Arc<std::sync::Mutex<Option<Result<SpaceWeather, String>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let slot_clone = slot.clone();
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            let result = crate::cloud::solar::SpaceWeatherClient::new()
                .fetch_hamqth_solar()
                .await
                .map_err(|e| e.to_string());
            *slot_clone.lock().unwrap_or_else(|p| p.into_inner()) = Some(result);
            ctx_clone.request_repaint();
        });

        self.solar_fetch_slot = Some(slot);
    }

    /// Odbiera wynik asynchronicznego pobrania warunków solarnych.
    pub fn poll_solar_fetch(&mut self) {
        if !self.solar_loading {
            return;
        }
        if let Some(ref slot) = self.solar_fetch_slot.clone() {
            if let Ok(mut guard) = slot.try_lock() {
                if let Some(result) = guard.take() {
                    self.solar_loading = false;
                    self.solar_fetch_slot = None;
                    match result {
                        Ok(weather) => self.record_solar_sample(weather),
                        Err(e) => {
                            self.solar_last_alert = Some(format!(
                                "Błąd pobierania danych solarnych: {}",
                                e
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Zapisuje próbkę warunków solarnych do historii i wykrywa istotne zmiany.
    pub fn record_solar_sample(&mut self, weather: SpaceWeather) {
        if let Some(prev) = self.solar_history.last() {
            let k_delta = weather.k_index as i64 - prev.k_index as i64;
            let sfi_delta = weather.sfi as i64 - prev.sfi as i64;
            if k_delta.abs() >= 2 {
                self.solar_last_alert = Some(format!(
                    "⚠ Zmiana indeksu K: {} → {} — możliwa burza geomagnetyczna",
                    prev.k_index, weather.k_index
                ));
            } else if sfi_delta.abs() >= 20 {
                self.solar_last_alert =
                    Some(format!("ℹ Zmiana SFI: {} → {}", prev.sfi, weather.sfi));
            } else {
                self.solar_last_alert = None;
            }
        }

        self.space_weather = weather.clone();

        let differs = match self.solar_history.last() {
            Some(p) => {
                p.sfi != weather.sfi
                    || p.ssn != weather.ssn
                    || p.k_index != weather.k_index
                    || p.a_index != weather.a_index
            }
            None => true,
        };
        if differs {
            self.solar_history.push(weather);
            if self.solar_history.len() > 96 {
                self.solar_history.remove(0);
            }
        }
    }
}

impl TabViewer for SpLogApp {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        self.tile_title(tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let tile_id = tab.clone();
        let mut action_popout = false;
        let mut action_close = false;

        egui::Frame::group(ui.style())
            .rounding(4.0)
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Własne widżety nagłówka (CAT, SPLIT, Szukaj, Spot)
                    self.render_tile_header_custom(&tile_id, ui);

                    // Przyciski odpinania i ukrywania (wyrównane do prawej)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").on_hover_text("Ukryj ten panel").clicked() {
                            action_close = true;
                        }
                        if ui.button("🗗").on_hover_text("Odepnij do osobnego okna pływającego").clicked() {
                            action_popout = true;
                        }
                    });
                });
                ui.separator();
                self.render_tile_body(&tile_id, ui);
            });

        if action_popout {
            self.popout_tile(&tile_id);
        }
        if action_close {
            self.close_tile(&tile_id);
        }
    }

    fn clear_background(&self, _tab: &Self::Tab) -> bool {
        true
    }
}

impl eframe::App for SpLogApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Śledzenie geometrii okna głównego (zapisywane przy wyjściu — obsługa multi-monitor)
        let (win_pos, win_size, win_max) = ctx.input(|i| {
            let vp = i.viewport();
            let pos = vp.outer_rect.map(|r| [r.min.x, r.min.y]);
            let size = vp.outer_rect.map(|r| [r.width(), r.height()]);
            (pos, size, vp.maximized.unwrap_or(false))
        });
        if let Some(pos) = win_pos {
            self.main_window_pos = Some(pos);
        }
        if let Some(size) = win_size {
            self.main_window_size = Some(size);
        }
        self.main_window_maximized = win_max;

        // ——— Odbieranie wynikow asynchronicznych operacji ———

        // WSPR: sprawdz czy pobieranie zakonczylo sie
        if self.wspr_loading {
            if let Some(ref slot) = self.wspr_fetch_slot.clone() {
                if let Ok(mut guard) = slot.try_lock() {
                    if let Some(result) = guard.take() {
                        self.wspr_loading = false;
                        self.wspr_fetch_slot = None;
                        match result {
                            Ok(spots) => {
                                self.wspr_spots = spots;
                                self.wspr_last_error = None;
                            }
                            Err(e) => {
                                self.wspr_last_error = Some(e);
                            }
                        }
                    }
                }
            }
        }

        // Synchronizuj cluster_spots z watkiem REST API (na kazda klatke gdy sie rozni rozmiar)
        if let Some(ref api_slot) = self.cluster_spots_api.clone() {
            if let Ok(mut guard) = api_slot.try_lock() {
                if guard.len() != self.cluster_spots.len() {
                    *guard = self.cluster_spots.clone();
                }
            }
        }

        // Wspólna kolejka wysyłki do serwisów online (Club Log / QRZ / eQSL)
        self.poll_upload_scheduler();

        // Śledzenie satelitów (przeloty + Doppler) i warunki solarne
        self.refresh_satellite_tracking();
        self.poll_solar_fetch();

        // ——— Globalne skróty klawiszowe ———
        ctx.input(|i| {
            // Ctrl+Z — Undo (cofnij ostatnie usunięcie QSO)
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && !i.modifiers.shift {
                // Undo będzie wykonane poniżej (borrow checker — nie można wywołać &mut self wewnątrz closure)
                // Używamy flagi żeby wywołać po wyjściu z closure
            }
        });

        // Undo/Redo przez skróty klawiszowe
        let do_undo = ctx.input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && !i.modifiers.shift);
        let do_redo = ctx.input(|i| {
            (i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && i.modifiers.shift)
            || (i.key_pressed(egui::Key::Y) && i.modifiers.ctrl)
        });
        let open_stats = ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl && i.modifiers.shift);

        // Rozszerzone skróty klawiszowe (F1/F5, Ctrl+S, Ctrl+F, Ctrl+N)
        let open_shortcuts = ctx.input(|i| i.key_pressed(egui::Key::F1));
        let refresh_log = ctx.input(|i| i.key_pressed(egui::Key::F5));
        let save_now = ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl && !i.modifiers.shift);
        let focus_filter = ctx.input(|i| i.key_pressed(egui::Key::F) && i.modifiers.ctrl);
        let new_qso = ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.ctrl);
        let open_palette = ctx.input(|i| i.key_pressed(egui::Key::P) && i.modifiers.ctrl && i.modifiers.shift);
        let wants_text = ctx.wants_keyboard_input();

        if do_undo { self.perform_undo(); }
        if do_redo { self.perform_redo(); }
        if open_stats { self.show_statistics_window = true; }

        if open_shortcuts { self.show_shortcuts_window = !self.show_shortcuts_window; }
        if refresh_log { self.reload_qsos(); }
        if save_now { self.save_station_config(); }

        // Skróty kolidujące z pisaniem tekstu działają tylko, gdy żadne pole nie ma fokusu.
        if !wants_text {
            if focus_filter { self.advanced_filter_dialog.is_open = true; }
            if new_qso {
                self.clear_qso_form();
                self.panel_qso.visible = true;
                self.focus_callsign_requested = true;
            }
        }

        if open_palette && !wants_text {
            self.show_command_palette = !self.show_command_palette;
            self.command_palette_query.clear();
            self.command_palette_selected = 0;
        }

        // Debounced lookup Callbook/QRZ po wpisaniu znaku
        self.process_debounced_lookup();

        // Wynik sprawdzania aktualizacji
        if let Some(rx) = &self.update_check_rx {
            if let Ok(outcome) = rx.try_recv() {
                match outcome {
                    crate::cloud::updater::UpdateCheckOutcome::UpToDate { local } => {
                        self.update_check_status = Some(format!("✅ Masz najnowszą wersję (v{}).", local));
                    }
                    crate::cloud::updater::UpdateCheckOutcome::NewVersion(release) => {
                        self.update_check_status = Some(format!(
                            "🆕 Dostępna jest nowa wersja: v{} (masz v{}).",
                            release.tag,
                            env!("CARGO_PKG_VERSION")
                        ));
                        self.update_available = Some(release);
                    }
                    crate::cloud::updater::UpdateCheckOutcome::Error(e) => {
                        self.update_check_status =
                            Some(format!("❌ Nie udało się sprawdzić aktualizacji: {}", e));
                    }
                }
                self.update_check_rx = None;
            }
        }

        // Wynik instalowania aktualizacji (self-replace)
        if let Some(rx) = &self.update_install_rx {
            if let Ok(msg) = rx.try_recv() {
                let installed_ok = msg.starts_with("✅");
                self.update_install_status = Some(msg);
                self.update_install_rx = None;
                if installed_ok {
                    // Skrypt podmiany czeka na zakończenie tego procesu — zamknij aplikację.
                    self.pending_restart = true;
                }
            }
        }

        if self.pending_restart {
            self.update_install_status = Some(
                "Trwa instalowanie aktualizacji — aplikacja zostanie zamknięta i uruchomiona ponownie."
                    .to_string(),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Odbiór asynchronicznego stanu radia z pętli Hamlib CAT (bi-directional sync)
        while let Ok(st) = self.cat_state_rx.try_recv() {
            self.cat_connected = st.connected;
            self.event_bus.publish(crate::core::events::AppEvent::RigState {
                frequency_hz: st.frequency_hz,
                mode: st.mode.clone(),
                connected: st.connected,
            });
            if st.connected {
                self.rig_state = st;
                if self.rig_state.frequency_hz > 0 {
                    if let Some(band_def) = crate::core::bandplan::get_band_by_freq(self.rig_state.frequency_hz) {
                        self.entry_band = band_def.name.to_string();
                    }
                    if !self.rig_state.mode.is_empty() {
                        self.entry_mode = self.rig_state.mode.clone();
                    }
                }
            }
        }

        // Synchronizuj stan radia do współdzielonego stanu serwera proxy (CAT Sharing)
        if self.cat_sharing_enabled {
            if let Ok(mut shared) = self.cat_shared_state.write() {
                *shared = self.rig_state.clone();
            }
        }

        // Odbiór poleceń z serwera Hamlib proxy (np. zmiana częstotliwości/emisji/PTT przez WSJT-X / FLDigi)
        let mut proxy_cmds = Vec::new();
        if let Some(ref mut rx) = self.cat_proxy_rx {
            while let Ok(cmd) = rx.try_recv() {
                proxy_cmds.push(cmd);
            }
        }
        for cmd in proxy_cmds {
            match cmd {
                crate::cat::server::RigServerCommand::SetFrequency(freq) => {
                    self.set_vfo_frequency(freq);
                }
                crate::cat::server::RigServerCommand::SetMode(mode) => {
                    self.set_vfo_mode(&mode);
                }
                crate::cat::server::RigServerCommand::SetPtt(ptt) => {
                    self.ptt_active = ptt;
                    if self.cat_connected {
                        let host = self.cat_host.clone();
                        let port = self.cat_port;
                        tokio::spawn(async move {
                            let _ = crate::cat::hamlib::HamlibClient::set_ptt(&host, port, ptt).await;
                        });
                    }
                }
                crate::cat::server::RigServerCommand::SetVfo(vfo) => {
                    self.rig_state.vfo = vfo.clone();
                    if self.cat_connected {
                        let host = self.cat_host.clone();
                        let port = self.cat_port;
                        tokio::spawn(async move {
                            let _ = crate::cat::hamlib::HamlibClient::set_vfo(&host, port, &vfo).await;
                        });
                    }
                }
                crate::cat::server::RigServerCommand::SetSplit { enabled, tx_vfo } => {
                    self.vfo_split = enabled;
                    self.rig_state.split_enabled = enabled;
                    if self.cat_connected {
                        let host = self.cat_host.clone();
                        let port = self.cat_port;
                        tokio::spawn(async move {
                            let _ = crate::cat::hamlib::HamlibClient::set_split(&host, port, enabled, &tx_vfo).await;
                        });
                    }
                }
                crate::cat::server::RigServerCommand::SetRit(rit) => {
                    self.rig_state.rit_hz = rit;
                    if self.cat_connected {
                        let host = self.cat_host.clone();
                        let port = self.cat_port;
                        tokio::spawn(async move {
                            let _ = crate::cat::hamlib::HamlibClient::set_rit(&host, port, rit).await;
                        });
                    }
                }
                crate::cat::server::RigServerCommand::SetXit(xit) => {
                    self.rig_state.xit_hz = xit;
                    if self.cat_connected {
                        let host = self.cat_host.clone();
                        let port = self.cat_port;
                        tokio::spawn(async move {
                            let _ = crate::cat::hamlib::HamlibClient::set_xit(&host, port, xit).await;
                        });
                    }
                }
                crate::cat::server::RigServerCommand::SetPower(watts) => {
                    self.rig_state.rf_power_watts = watts;
                    if self.cat_connected {
                        let host = self.cat_host.clone();
                        let port = self.cat_port;
                        tokio::spawn(async move {
                            let _ = crate::cat::hamlib::HamlibClient::set_power(&host, port, watts).await;
                        });
                    }
                }
            }
        }

        // Odbiór zdarzeń i spotów z klastra DX Telnet
        if let Some(ref rx) = self.cluster_event_rx {
            while let Ok(evt) = rx.try_recv() {
                match evt {
                    ClusterEvent::Connected(msg) => {
                        self.cluster_connected = true;
                        self.cluster_connecting = false;
                        self.cluster_status_text = msg.clone();
                        self.status_toast = Some((msg, std::time::Instant::now()));
                        self.event_bus.publish(crate::core::events::AppEvent::ClusterStatus { connected: true });
                    }
                    ClusterEvent::Disconnected(msg) => {
                        self.cluster_connected = false;
                        self.cluster_connecting = false;
                        self.cluster_status_text = msg;
                        self.event_bus.publish(crate::core::events::AppEvent::ClusterStatus { connected: false });
                    }
                    ClusterEvent::Spot(spot) => {
                        let is_duplicate = self.cluster_spots.iter().take(30).any(|s| {
                            s.dx_call == spot.dx_call && s.band == spot.band && (s.frequency_khz - spot.frequency_khz).abs() < 2.0
                        });
                        if !is_duplicate {
                            let is_atno = if let Some(info) = self.prefix_matcher.lookup(&spot.dx_call) {
                                let awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
                                let st = awards.check_status_full(
                                    &spot.dx_call,
                                    &spot.band,
                                    if spot.is_ft8 { "FT8" } else { "CW" },
                                    Some(info.dxcc),
                                    None,
                                    Some(info.cqz),
                                    None,
                                    Some(&info.continent),
                                    None,
                                );
                                st.is_new_dxcc
                            } else {
                                false
                            };

                            if is_atno && self.band_alert_enabled {
                                crate::media::sounds::play_new_dxcc_alert();
                                self.status_toast = Some((
                                    format!("⭐ ATNO DXCC: {} na pasmie {} ({:.1} kHz)!", spot.dx_call, spot.band, spot.frequency_khz),
                                    std::time::Instant::now(),
                                ));
                            }

                            self.event_bus.publish(crate::core::events::AppEvent::DxSpot {
                                spotter: spot.spotter.clone(),
                                dx_call: spot.dx_call.clone(),
                                frequency_khz: spot.frequency_khz,
                                band: spot.band.clone(),
                                comment: spot.comment.clone(),
                                time_utc: spot.time_utc.clone(),
                                is_ft8: spot.is_ft8,
                                is_skimmer: spot.is_skimmer,
                            });
                            self.cluster_spots.insert(0, spot);
                        }
                    }
                    ClusterEvent::RawLine(_) => {}
                }
            }
        }

        // Usuń przestarzałe spoty (starsze niż 60 minut), a następnie ogranicz bufor
        // do 200 najnowszych pozycji (nowe spoty są wstawiane na indeksie 0, więc ucinamy najstarsze od końca).
        let now_ts = chrono::Utc::now().timestamp();
        const SPOT_MAX_AGE_SECS: i64 = 60 * 60;
        if self.cluster_spots.iter().any(|s| now_ts - s.received_at > SPOT_MAX_AGE_SECS) {
            self.cluster_spots.retain(|s| now_ts - s.received_at <= SPOT_MAX_AGE_SECS);
        }
        if self.cluster_spots.len() > 200 {
            self.cluster_spots.truncate(200);
        }

        // Odbiór asynchronicznych danych korespondenta z Callbook / HamQTH / Callook / QRZ
        while let Ok(data) = self.qrz_lookup_rx.try_recv() {
            if data.callsign == self.entry_callsign.trim().to_uppercase() {
                if let Some(n) = data.name {
                    if self.entry_name.is_empty() || self.entry_name != n {
                        self.entry_name = n;
                    }
                }
                if let Some(q) = data.qth {
                    if self.entry_qth.is_empty() || self.entry_qth != q {
                        self.entry_qth = q;
                    }
                }
                if let Some(g) = data.gridsquare {
                    if self.entry_grid.is_empty() || self.entry_grid != g {
                        self.entry_grid = g;
                        self.recalculate_distance_from_grid();
                    }
                }
                if let Some(st) = data.state {
                    if self.entry_state.is_empty() || self.entry_state != st {
                        self.entry_state = st;
                    }
                }
                if let Some(mgr) = data.qsl_manager {
                    if self.entry_qsl_manager.is_empty() || self.entry_qsl_manager != mgr {
                        self.entry_qsl_manager = mgr;
                    }
                }
                if let Some(img) = data.image_url {
                    self.photo_viewer_dialog.photo_url = Some(img);
                }
                self.status_message = Some(format!("Pobrano dane Callbook dla: {} ({})", data.callsign, self.entry_name));
                self.status_toast = Some((format!("Pobrano dane korespondenta {}!", data.callsign), std::time::Instant::now()));
            }
        }

        // Odbiór asynchronicznych komunikatów z synchronizacji online (LoTW, eQSL, Club Log, QRZ)
        while let Ok((msg, rebuild_awards)) = self.sync_log_rx.try_recv() {
            self.online_sync_logs.push(msg.clone());
            self.status_toast = Some((msg, std::time::Instant::now()));
            if rebuild_awards {
                self.rebuild_awards_full();
            }
        }

        // Odbiór pakietów cyfrowych WSJT-X (UDP 2237)
        while let Ok(msg) = self.wsjtx_rx.try_recv() {
            self.wsjtx_packets_count += 1;
            match msg {
                crate::digital::wsjtx::WsjtxMessage::Status { dial_freq, mode, dx_call, .. } => {
                    if !dx_call.is_empty() {
                        self.wsjtx_last_call = Some(dx_call);
                    }
                    if dial_freq > 0 {
                        self.rig_state.frequency_hz = dial_freq;
                        if let Some(band_def) = crate::core::bandplan::get_band_by_freq(dial_freq) {
                            self.entry_band = band_def.name.to_string();
                        }
                    }
                    if !mode.is_empty() {
                        self.entry_mode = mode;
                    }
                }
                crate::digital::wsjtx::WsjtxMessage::QsoLogged(mut qso) => {
                    let now = chrono::Utc::now();
                    if qso.qso_date.is_empty() {
                        qso.qso_date = now.format("%Y%m%d").to_string();
                    }
                    if qso.time_on.is_empty() {
                        qso.time_on = now.format("%H%M").to_string();
                    }
                    qso.journal_id = Some(self.active_journal.id.clone());

                    if let Some(info) = self.prefix_matcher.lookup(&qso.callsign) {
                        qso.country = Some(info.country.clone());
                        qso.dxcc = Some(info.dxcc);
                        qso.continent = Some(info.continent.clone());
                        qso.cqz = Some(info.cqz);
                        qso.ituz = Some(info.ituz);
                    }

                    let insert_res = {
                        let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                        db.insert_qso(&qso)
                    };
                    match insert_res {
                        Ok(id) => {
                            qso.id = Some(id);
                            {
                                let mut awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
                                awards.register_qso_record(&qso);
                            }
                            self.invalidate_cluster_badges();
                            {
                                let mut scp = self.scp_engine.lock().unwrap_or_else(|p| p.into_inner());
                                scp.insert(&qso.callsign);
                            }
                            self.status_toast = Some((format!("WSJT-X: Automatycznie dodano QSO z {}!", qso.callsign), std::time::Instant::now()));
                            self.reload_qsos();
                        }
                        Err(e) => {
                            self.report_error(format!("Błąd auto-zapisu WSJT-X: {}", e));
                        }
                    }
                }
                _ => {}
            }
        }

        // Odbiór aktualizacji stanu z modułu JS8Call przez TCP API
        if let Some(ref rx) = self.js8call_state_rx {
            while let Ok(state) = rx.try_recv() {
                // Aktualizacja stanu połączenia i informacji o stacji JS8Call
                if state.dial_freq_hz > 0 && state.dial_freq_hz != self.js8call_state.dial_freq_hz {
                    self.rig_state.frequency_hz = state.dial_freq_hz;
                    if let Some(band_def) = crate::core::bandplan::get_band_by_freq(state.dial_freq_hz) {
                        self.entry_band = band_def.name.to_string();
                    }
                }
                // Aktualizacja trybu pracy na JS8 gdy JS8Call jest podłączony
                if state.connected && self.entry_mode != "JS8" {
                    self.entry_mode = "JS8".to_string();
                }
                self.js8call_state = state;
            }
        }

        // Odbiór QSO zalogowanych przez JS8Call (wiadomość LOG.QSO z API)
        let mut js8_new_qsos = Vec::new();
        if let Some(ref rx) = self.js8call_qso_rx {
            while let Ok(qso) = rx.try_recv() {
                js8_new_qsos.push(qso);
            }
        }
        for mut qso in js8_new_qsos {
            // Uzupełnij identyfikator aktywnego dziennika
            qso.journal_id = Some(self.active_journal.id.clone());

            // Wzbogać QSO o dane DXCC z lokalnej bazy prefiksów
            if let Some(info) = self.prefix_matcher.lookup(&qso.callsign) {
                qso.country = Some(info.country.clone());
                qso.dxcc = Some(info.dxcc);
                qso.continent = Some(info.continent.clone());
                qso.cqz = Some(info.cqz);
                qso.ituz = Some(info.ituz);
            }

            // Zapisz QSO do bazy SQLite
            let insert_res = {
                let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                db.insert_qso(&qso)
            };
            match insert_res {
                Ok(id) => {
                    qso.id = Some(id);
                    {
                        let mut awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
                        awards.register_qso_record(&qso);
                    }
                    self.invalidate_cluster_badges();
                    {
                        let mut scp = self.scp_engine.lock().unwrap_or_else(|p| p.into_inner());
                        scp.insert(&qso.callsign);
                    }
                    self.status_toast = Some((
                        format!("JS8Call: Automatycznie dodano QSO z {}!", qso.callsign),
                        std::time::Instant::now(),
                    ));
                    self.reload_qsos();
                }
                Err(e) => {
                    self.status_toast = Some((
                        format!("Błąd auto-zapisu JS8Call: {}", e),
                        std::time::Instant::now(),
                    ));
                }
            }
        }

        // Odbiór QSO ze stacji w sieci LAN Multi-Op
        let mut lan_new_qsos = Vec::new();
        if let Some(ref mut rx) = self.multi_op_incoming_rx {
            while let Ok(qso) = rx.try_recv() {
                lan_new_qsos.push(qso);
            }
        }
        for mut qso in lan_new_qsos {
            qso.journal_id = Some(self.active_journal.id.clone());
            if let Some(info) = self.prefix_matcher.lookup(&qso.callsign) {
                qso.country = Some(info.country.clone());
                qso.dxcc = Some(info.dxcc);
                qso.continent = Some(info.continent.clone());
                qso.cqz = Some(info.cqz);
                qso.ituz = Some(info.ituz);
            }
            let insert_res = {
                let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
                db.insert_qso(&qso)
            };
            if let Ok(id) = insert_res {
                qso.id = Some(id);
                {
                    let mut awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
                    awards.register_qso_record(&qso);
                }
                self.invalidate_cluster_badges();
                {
                    let mut scp = self.scp_engine.lock().unwrap_or_else(|p| p.into_inner());
                    scp.insert(&qso.callsign);
                }
                self.multi_op_log.push(format!("Odebrano z LAN: {} ({} {})", qso.callsign, qso.band, qso.mode));
                self.status_toast = Some((
                    format!("🌐 Multi-Op LAN: Dodano QSO z {}!", qso.callsign),
                    std::time::Instant::now(),
                ));
                self.reload_qsos();
            }
        }

        // Zastosowanie wybranego motywu kolorystycznego (patrz gui::theme::ThemePreset).
        // Zachowujemy `dark_theme` zsynchronizowany z jasnością aktywnego presetu,
        // aby ewentualny inny kod odczytujący ten flag nadal działał poprawnie.
        self.dark_theme = self.theme_preset.is_dark();
        self.theme_preset.apply(ctx);

        // Skala czcionki / UI (80–150%). Stosowana globalnie jako mnożnik zoom.
        ctx.set_zoom_factor(self.font_scale.clamp(0.8, 1.5));

        ctx.style_mut(|s| {
            s.spacing.window_margin = egui::Margin::symmetric(6.0, 4.0);
            s.spacing.button_padding = egui::vec2(4.0, 2.0);
        });

        // Górny pasek menu systemowego oraz pasek szybkiego dostępu (bez kolizji!)
        egui::TopBottomPanel::top("top_menu_bar").show(ctx, |ui| {
            render_menu_bar(self, ui);
            ui.separator();
            render_main_toolbar(self, ui);
        });

        // Dolny pasek statusu stacji
        egui::TopBottomPanel::bottom("bottom_status_bar").show(ctx, |ui| {
            let lang = self.current_language;
            ui.horizontal(|ui| {
                let fallback_status = tr("status.ready_all_active", lang);
                let status_txt = self.status_message.as_deref().unwrap_or(fallback_status);
                ui.label(egui::RichText::new(status_txt).size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));

                ui.separator();

                // Pasek operacyjny: częstotliwość, pasmo, tryb, split i RST
                let freq_mhz = self.rig_state.frequency_hz as f64 / 1_000_000.0;
                let mut op_txt = format!("📻 {:.3} MHz | {} | {}", freq_mhz, self.entry_band, self.entry_mode);
                if self.vfo_split {
                    op_txt.push_str(" | SPLIT");
                }
                ui.label(egui::RichText::new(op_txt).size(11.0).monospace().color(egui::Color32::from_rgb(56, 189, 248)));

                ui.separator();

                let rst_txt = format!("RST {} / {}", self.entry_rst_sent, self.entry_rst_rcvd);
                ui.label(egui::RichText::new(rst_txt).size(11.0).monospace().color(egui::Color32::from_rgb(250, 204, 21)));

                ui.separator();

                let prop_txt = format!("🌞 SFI {} | K {}", self.space_weather.sfi, self.space_weather.k_index);
                ui.label(egui::RichText::new(prop_txt).size(11.0).color(egui::Color32::from_rgb(52, 211, 153)));

                ui.separator();

                let journal_str = format!("📁 {}: {} ({} QSO)", tr("statusbar.log", lang), self.active_journal.name, self.recent_qsos.len());
                ui.label(egui::RichText::new(journal_str).size(11.0).color(egui::Color32::from_rgb(52, 211, 153)));

                ui.separator();

                let prof_name = if self.my_station.name.is_empty() { &self.my_station.callsign } else { &self.my_station.name };
                let profile_str = format!("🏷 {}: {}", tr("statusbar.profile", lang), prof_name);
                ui.label(egui::RichText::new(profile_str).size(11.0).color(egui::Color32::from_rgb(250, 204, 21)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("SPLogbook v1.0.3 | SP6INA | GPLv3").size(10.0).color(egui::Color32::from_rgb(100, 116, 139)));
                    ui.separator();

                    let utc_str = format!("⏱ {}: {}", tr("statusbar.utc", lang), chrono::Utc::now().format("%H:%M:%S"));
                    ui.label(egui::RichText::new(utc_str).size(11.0).monospace().color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.separator();

                    if self.cat_connected {
                        ui.label(egui::RichText::new("● CAT").size(11.0).strong().color(egui::Color32::from_rgb(34, 197, 94)));
                    } else {
                        ui.label(egui::RichText::new("○ CAT").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    }
                    ui.separator();

                    if self.cluster_connected {
                        ui.label(egui::RichText::new("● CLUSTER").size(11.0).strong().color(egui::Color32::from_rgb(34, 197, 94)));
                    } else if self.cluster_connecting {
                        ui.label(egui::RichText::new("● CLUSTER").size(11.0).color(egui::Color32::from_rgb(250, 204, 21)));
                    } else {
                        ui.label(egui::RichText::new("○ CLUSTER").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    }
                });
            });
        });

        // Główny obszar roboczy - swobodny pulpit stacji SPLogbook (MDI Desktop)
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.compact_hud_mode {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.heading(egui::RichText::new("📻 Tryb Kompaktowy (Mini HUD)").size(22.0).color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Kompaktowe okno operacyjne VFO + QSO jest aktywne na pulpicie.").size(14.0));
                    ui.label(egui::RichText::new("Możesz swobodnie pracować w innych programach (np. FT8/JTDX/przeglądarka) z minimalistycznym oknem SPLogbook.").color(egui::Color32::from_rgb(148, 163, 184)));
                    ui.add_space(16.0);
                    if ui.button(egui::RichText::new("🗗 Powrót do pełnego pulpitu roboczego").size(14.0).strong()).clicked() {
                        self.compact_hud_mode = false;
                        self.save_station_config();
                    }
                });
            } else {
                let total_visible = (self.panel_vfo.visible as usize)
                    + (self.panel_qso.visible as usize)
                    + (self.panel_log.visible as usize)
                    + (self.panel_cluster.visible as usize)
                    + (self.panel_bandmap.visible as usize)
                    + (self.panel_solar.visible as usize)
                    + (self.panel_satellites.visible as usize)
                    + (self.panel_world_map.visible as usize);

                if total_visible == 0 {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.heading(egui::RichText::new("📻 SPLogbook Workspace").size(26.0).color(egui::Color32::from_rgb(56, 189, 248)));
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Wszystkie kafelki modułów są obecnie zamknięte.").size(14.0));
                        ui.label(egui::RichText::new("Możesz w każdej chwili przywrócić domyślny układ lub włączyć wybrane moduły z menu 'Widok'.").color(egui::Color32::from_rgb(148, 163, 184)));
                        ui.add_space(16.0);
                        if ui.button(egui::RichText::new("🔄 Przywróć optymalny układ kafelków").size(15.0).strong()).clicked() {
                            self.reset_panel_layout();
                            self.save_station_config();
                        }
                    });
                } else {
                    // Uzgodnij `dock_state` z konfiguracją paneli (widoczność/odpięcie).
                    self.sync_dock_state();

                    // Tymczasowo wyjmij `dock_state`, aby uniknąć konfliktu pożyczek:
                    // `DockArea` potrzebuje `&mut dock_state`, a `TabViewer` (self)
                    // potrzebuje `&mut self`. Po renderze przywracamy stan.
                    let mut dock_state = std::mem::replace(&mut self.dock_state, DockState::new(vec![]));
                    DockArea::new(&mut dock_state)
                        .style(Style::from_egui(ui.style().as_ref()))
                        .show_add_buttons(false)
                        .show_add_popup(false)
                        .show_close_buttons(false)
                        .show_inside(ui, self);
                    self.dock_state = dock_state;

                    // Zastosuj zmiany widoczności/odpięcia wykonane przyciskami nagłówka.
                    self.sync_dock_state();

                    // Persystuj układ dokowania tylko wtedy, gdy faktycznie się zmienił.
                    let current = self.serialize_dock_layout();
                    if current != self.last_saved_dock_layout {
                        self.last_saved_dock_layout = current;
                        self.save_station_config();
                    }

                    // Dyskretny pasek pomocy i szybkiego resetowania układu na dole pulpitu
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("🔄 Przywróć optymalny układ").size(11.0)).on_hover_text("Ustawia domyślne, ergonomiczne rozmieszczenie wszystkich otwartych kafelków").clicked() {
                                self.reset_panel_layout();
                                self.save_station_config();
                            }
                            ui.label(egui::RichText::new("SPLogbook • Przeciągaj karty za nagłówek • Zmieniaj rozmiar za krawędzie • Podział prawym przyciskiem").size(11.0).color(egui::Color32::from_rgb(100, 116, 139)));
                        });
                    });
                }
            }
        });

        // W zależności od trybu: Mini HUD (tryb kompaktowy) lub pełny pulpit roboczy
        if self.compact_hud_mode {
            crate::gui::mini_hud::MiniHudBar::render(self, ctx);
        } else {
            // Pływające okna modułów (pop-out windows) — renderowane tylko wtedy,
            // gdy panel jest „odpięty” (floating). Kafelki zadokowane rysuje
            // układ kolumnowy w CentralPanel poniżej.
            if self.panel_qso.floating { render_qso_entry_window(self, ctx); }
            if self.panel_vfo.floating { render_vfo_window(self, ctx); }
            if self.panel_log.floating { render_logbook_window(self, ctx); }
            if self.panel_cluster.floating { render_cluster_window(self, ctx); }
            if self.panel_solar.floating { render_solar_window(self, ctx); }
            if self.panel_bandmap.floating { render_bandmap_window(self, ctx); }
            if self.panel_satellites.floating { render_satellites_window(self, ctx); }
            if self.panel_world_map.floating { render_world_map_window(self, ctx); }
        }

        // Pozostałe okna modułów zaawansowanych
        render_contest_window(self, ctx);
        render_custom_contest_editor(self, ctx);
        render_cw_macros_window(self, ctx);
        render_station_ledger_window(self, ctx);
        render_welcome_wizard(self, ctx);
        render_cat_settings_window(self, ctx);
        render_online_sync_window(self, ctx);
        render_awards_matrix_window(self, ctx);
        render_edit_qso_dialog(self, ctx);
        render_column_settings(self, ctx);
        render_statistics_window(self, ctx);
        crate::gui::cluster_panel::render_add_cluster_dialog(self, ctx);
        crate::gui::contest::render_multi_op_window(self, ctx);
        crate::gui::find_duplicates::render_find_duplicates_window(self, ctx);
        crate::gui::station_profiles::render_station_profiles_window(self, ctx);
        render_voice_keyer_window(self, ctx);
        crate::gui::workspace_profiles::render_workspace_profiles_window(self, ctx);
        crate::gui::operator_assistant::render_operator_assistant(self, ctx);
        crate::gui::plugin_manager::render_plugin_manager(self, ctx);

        // Okna dialogowe i narzędzia pomocnicze
        if self.journal_dialog.is_open {
            let mut switched_journal = None;
            let lang = self.current_language;
            if let Ok(db) = self.log_db.lock() {
                self.journal_dialog.render(ctx, &db, lang, &mut |j| {
                    switched_journal = Some(j.clone());
                });
            }
            if let Some(j) = switched_journal {
                self.active_journal = j.clone();
                self.my_station.callsign = j.station_callsign.clone();
                self.my_station.operator = j.operator.clone();
                self.my_station.gridsquare = j.my_gridsquare.clone();
                self.my_station.pga_gmina = if j.my_pga.is_empty() { None } else { Some(j.my_pga.clone()) };
                self.reload_qsos();
                self.status_toast = Some((format!("Przełączono aktywny profil dziennika na: {}", j.name), std::time::Instant::now()));
            }
        }

        if self.advanced_filter_dialog.is_open {
            let mut filter_results = None;
            let mut clear_filter = false;
            let journal_id = self.active_journal.id.clone();
            let my_call = self.my_station.callsign.clone();
            let lang = self.current_language;
            if let Ok(db) = self.log_db.lock() {
                self.advanced_filter_dialog.render(
                    ctx,
                    &db,
                    Some(&journal_id),
                    &my_call,
                    lang,
                    &mut |res| filter_results = Some(res),
                    &mut || clear_filter = true,
                );
            }
            if let Some(filtered) = filter_results {
                self.recent_qsos = filtered;
            }
            if clear_filter {
                self.reload_qsos();
            }
        }

        if self.cw_terminal_dialog.is_open {
            let my_call = self.my_station.callsign.clone();
            self.cw_terminal_dialog.render(ctx, &my_call);
        }

        if self.qsl_designer_dialog.is_open {
            let my_call = self.my_station.callsign.clone();
            if let Ok(db) = self.log_db.lock() {
                self.qsl_designer_dialog.render(ctx, &db, &my_call);
            }
        }
        if self.show_wspr_window {
            crate::gui::wspr_panel::render_wspr_window(self, ctx);
        }

        // 1. Zdalne wybudzanie stacji Wake-on-LAN (WOL)
        self.wol_dialog.show(ctx, self.current_language);

        // 2. Położenie Księżyca i Słońca (EME / Kalkulator astronomiczny)
        let my_grid = self.my_station.gridsquare.clone();
        self.astronomy_dialog.show(ctx, &my_grid, &mut self.rotor_state, self.current_language);

        // 3. Moduł SOTA / POTA
        self.sota_dialog.show(ctx, &self.recent_qsos);

        // 4. Przeglądarka wysp IOTA
        {
            let awards = self.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(iota) = self.iota_dialog.show(ctx, &self.service_db, &awards) {
                self.entry_iota = iota;
            }

            // 5. Przeglądarka stanów USA (WAS)
            if let Some(state) = self.states_dialog.show(ctx, &self.service_db, &awards) {
                self.entry_state = state;
            }
        }

        // 6. Baza menedżerów QSL
        if let Some(qslm) = self.qsl_manager_dialog.show(ctx, &self.service_db) {
            self.entry_qsl_manager = qslm;
        }

        // 7. Przeglądarka fotografii i kart QSL korespondenta
        self.photo_viewer_dialog.show(ctx);

        // 8. Menedżer prefiksów DXCC
        self.prefix_manager_dialog.show(ctx, &self.service_db);

        // 9. Formularz wysyłania własnego spotu do DX Cluster
        if let Some(sub) = self.send_spot_dialog.show(ctx, &self.my_station.callsign, self.current_language) {
            let band_str = crate::core::bandplan::get_band_by_freq((sub.freq_khz * 1000.0) as u64)
                .map(|b| b.name.to_string())
                .unwrap_or_else(|| "HF".to_string());
            let is_ft8 = sub.comment.to_uppercase().contains("FT8");
            let spot = crate::cluster::telnet::DxSpot {
                frequency_khz: sub.freq_khz,
                dx_call: sub.dx_call.clone(),
                spotter: self.my_station.callsign.clone(),
                comment: sub.comment.clone(),
                time_utc: chrono::Utc::now().format("%H%M").to_string(),
                band: band_str,
                is_ft8,
                is_skimmer: false,
                received_at: chrono::Utc::now().timestamp(),
            };
            self.cluster_spots.insert(0, spot);
            self.status_toast = Some((format!("Wysłano spot dla {} ({:.1} kHz)", sub.dx_call, sub.freq_khz), std::time::Instant::now()));
        }

        // Pływające powiadomienia Toast
        if let Some((ref msg, time)) = self.status_toast {
            if time.elapsed().as_secs() < 5 {
                egui::Area::new(egui::Id::new("status_toast_area"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -40.0))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(egui::Color32::from_rgb(15, 23, 42))
                            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(56, 189, 248)))
                            .inner_margin(egui::Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(format!("🔔 {}", msg)).strong().color(egui::Color32::WHITE));
                            });
                    });
            } else {
                self.status_toast = None;
            }
        }

        // Okno "O programie SPLogbook"
        if self.show_about_window {
            let mut is_open = self.show_about_window;
            let mut close_req = false;
            egui::Window::new(tr("tab.about", self.current_language))
                .open(&mut is_open)
                .default_size([480.0, 340.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(egui::RichText::new("📻 SPLogbook").size(24.0).color(egui::Color32::from_rgb(56, 189, 248)));
                        ui.label(egui::RichText::new("Zaawansowany Dziennik Krótkofalarski").size(14.0).strong());
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Autor: Mariusz Woźniak (SP6INA)").size(13.0).strong().color(egui::Color32::from_rgb(250, 204, 21)));
                        ui.label("Licencja: GNU General Public License v3.0 (GPL-3.0-or-later)");
                        ui.add_space(12.0);
                        ui.label("Natywna, nowoczesna aplikacja okienkowa dla stacji krótkofalarskich.");
                        ui.label("Pełna integracja z Hamlib 4.7+, TCI, rotctld, WSJT-X, NOAA, QRZ, HamQTH, eQSL, LoTW i Club Log.");
                        ui.add_space(10.0);
                        egui::CollapsingHeader::new("🩺 Diagnostyka").show(ui, |ui| {
                            ui.horizontal(|ui| { ui.label("Wersja:"); ui.monospace(env!("CARGO_PKG_VERSION")); });
                            ui.horizontal(|ui| { ui.label("Platforma:"); ui.monospace(std::env::consts::OS); });
                            ui.horizontal(|ui| { ui.label("Architektura:"); ui.monospace(std::env::consts::ARCH); });
                            ui.horizontal(|ui| { ui.label("Motyw:"); ui.monospace(self.theme_preset.label_pl()); });
                            ui.horizontal(|ui| {
                                ui.label("Repozytorium bazy danych:");
                                ui.monospace(self.config_file_path.parent().map(|p| p.display().to_string()).unwrap_or_default());
                            });
                            if ui.button("📋 Kopiuj dane diagnostyczne").clicked() {
                                let info = format!(
                                    "SPLogbook {} | OS: {} {} | Motyw: {} | Konfiguracja: {}",
                                    env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH,
                                    self.theme_preset.id(),
                                    self.config_file_path.display()
                                );
                                ui.output_mut(|o| o.copied_text = info);
                            }
                        });
                        ui.add_space(8.0);
                        if ui.button(tr("btn.close", self.current_language)).clicked() {
                            close_req = true;
                        }
                    });
                });
            if close_req || !is_open {
                self.show_about_window = false;
            }
        }

        // Paleta poleceń (Ctrl+Shift+P)
        crate::gui::command_palette::render_command_palette(self, ctx);

        // Dziennik zmian i sprawdzanie aktualizacji
        crate::gui::changelog::render_changelog_window(self, ctx);
        crate::gui::changelog::render_update_check_window(self, ctx);
        crate::gui::user_manual::render_user_manual_window(self, ctx);

        if self.show_shortcuts_window {
            let mut is_open = self.show_shortcuts_window;
            egui::Window::new(format!("⌨ {}", tr("help.shortcuts_title", self.current_language)))
                .open(&mut is_open)
                .default_size([420.0, 420.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let rows: &[(&str, &str)] = &[
                            ("Enter", "Zapisz QSO w panelu QSO Entry (gdy pole ma fokus)"),
                            ("Esc", "Wyczyść formularz QSO Entry (gdy pole ma fokus)"),
                            ("Ctrl+N", "Nowe QSO — wyczyść formularz i ustaw kursor na znaku"),
                            ("Ctrl+S", "Zapisz konfigurację / dziennik"),
                            ("Ctrl+Shift+S", "Otwórz okno statystyk"),
                            ("Ctrl+F", "Zaawansowane wyszukiwanie w logbooku"),
                            ("Ctrl+Z", "Cofnij (przywróć ostatnio usunięte QSO)"),
                            ("Ctrl+Y / Ctrl+Shift+Z", "Ponów (redo)"),
                            ("F1", "Skróty klawiszowe (to okno)"),
                            ("F5", "Odśwież dziennik (listę QSO)"),
                        ];
                        egui::Grid::new("shortcuts_grid").num_columns(2).spacing([16.0, 6.0]).striped(true).show(ui, |ui| {
                            for (key, desc) in rows {
                                ui.monospace(*key);
                                ui.label(*desc);
                                ui.end_row();
                            }
                        });
                    });
                });
            if !is_open {
                self.show_shortcuts_window = false;
            }
        }

        // Legenda kolorów i statusów (niezależna od samego koloru — symbole + tekst)
        if self.show_legend_window {
            let mut is_open = self.show_legend_window;
            egui::Window::new(tr("legend.title", self.current_language))
                .open(&mut is_open)
                .default_size([460.0, 420.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(egui::RichText::new(
                            "Każdy status jest oznaczony kolorem ORAZ symbolem/kształtem, aby był czytelny także dla osób z zaburzeniami widzenia barw."
                        ).italics().color(egui::Color32::from_rgb(148, 163, 184)));

                        ui.add_space(8.0);
                        let rows: &[(egui::Color32, &str, &str, &str)] = &[
                            (egui::Color32::from_rgb(34, 197, 94), "●", "Zielony", "Połączono / aktywny — DX Cluster połączony, WSJT-X aktywny, QSO zapisane poprawnie."),
                            (egui::Color32::from_rgb(250, 204, 21), "◐", "Żółty / bursztyn", "Łączenie / ostrzeżenie — trwa łączenie z klastrem, aktywny filtr, status oczekujący."),
                            (egui::Color32::from_rgb(148, 163, 184), "○", "Szary", "Rozłączono / nieaktywny — klaster rozłączony, moduł wyłączony."),
                            (egui::Color32::from_rgb(239, 68, 68), "⚠", "Czerwony", "Błąd / akcja niszcząca — błąd operacji, przycisk rozłączenia, usuwanie wpisu."),
                            (egui::Color32::from_rgb(56, 189, 248), "ℹ", "Niebieski", "Informacja / wartości aktywne — znak OP, nagłówki sekcji, wartości pomiarowe."),
                            (egui::Color32::from_rgb(52, 211, 153), "✦", "Cyjan / zielony", "Sukces / potwierdzenie — QSO potwierdzone (LoTW/eQSL), potwierdzony zapis."),
                        ];
                        egui::Grid::new("legend_grid")
                            .num_columns(4)
                            .spacing([12.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (color, shape, name, desc) in rows {
                                    ui.label(egui::RichText::new(*shape).color(*color).size(18.0));
                                    ui.label(egui::RichText::new(*name).strong().color(*color));
                                    ui.label(egui::RichText::new("■").color(*color).size(14.0));
                                    ui.label(egui::RichText::new(*desc).color(egui::Color32::from_rgb(203, 213, 225)));
                                    ui.end_row();
                                }
                            });
                    });
                });
            if !is_open {
                self.show_legend_window = false;
            }
        }

        // Resetowanie flagi po narysowaniu i spzycjonowaniu okien w danej klatce
        self.reset_layout_requested = false;

        // Płynne odświeżanie zegara UTC, stanu radia CAT oraz spotów sieciowych bez obciążania procesora (4 Hz)
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Zapisanie geometrii okna głównego (i pozostałej konfiguracji) przy zamknięciu aplikacji.
        self.save_station_config();
    }
}

impl Drop for SpLogApp {
    fn drop(&mut self) {
        let backup_dir = std::path::Path::new("backups");
        let _ = std::fs::create_dir_all(backup_dir);
        let _ = BackupManager::backup_database(&self.active_db_path, backup_dir);
        
        let db = self.log_db.lock().unwrap_or_else(|p| p.into_inner());
        if let Ok(qsos) = db.get_recent_qsos(5000) {
            let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &self.my_station.callsign);
            let _ = BackupManager::backup_adif(&adif, backup_dir);
        }
    }
}
