// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::hamlib::RigState;
use crate::cat::rotor::RotorState;
use crate::cluster::telnet::ClusterEvent;
use crate::cluster::telnet::DxSpot;
use crate::core::awards::{AwardsEngine, PolishDistrictInfo, QsoAwardStatus};
use crate::core::backup::BackupManager;
use crate::core::database::Journal;
use crate::core::i18n::{Msg, key, tr};
use crate::core::ingest::{IngestError, IngestOutcome, QsoOrigin};
use crate::core::prefix::PrefixInfo;
use crate::core::qso::QsoRecord;
use crate::core::service_db::{QslManagerRecord, ServiceDatabase};
use crate::core::station::{AppConfig, StationProfile, ViewPanelConfig, WorkspaceProfile};
use crate::gui::awards_matrix::render_awards_matrix_window;
use crate::gui::bandmap::render_bandmap_window;
use crate::gui::cat_settings::render_cat_settings_window;
use crate::gui::cluster_panel::render_cluster_window;
use crate::gui::contest::{render_contest_window, render_custom_contest_editor};
use crate::gui::cw_macros::render_cw_macros_window;
use crate::gui::icons;
use crate::gui::logbook_table::{
    render_column_settings, render_edit_qso_dialog, render_logbook_window,
};
use crate::gui::menu::{render_main_toolbar, render_menu_bar};
use crate::gui::online_sync::render_online_sync_window;
use crate::gui::qso_entry::render_qso_entry_window;
use crate::gui::satellites::render_satellites_window;
use crate::gui::solar_panel::render_solar_window;
use crate::gui::station_ledger::render_station_ledger_window;
use crate::gui::statistics::render_statistics_window;
use crate::gui::theme::ThemeExt;
use crate::gui::vfo_panel::render_vfo_window;
use crate::gui::voice_keyer::render_voice_keyer_window;
use crate::gui::welcome_wizard::render_welcome_wizard;
use crate::gui::world_map::render_world_map_window;
use eframe::egui;
use egui_dock::{DockArea, DockState, Style, TabViewer};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

pub use crate::gui::drill_filter::{DrillFilter, QslDrillStatus};

/// Główny stan aplikacji natywnego pulpitu SPLogbook
pub use crate::gui::state::ShutdownPolicy;

pub struct AppInit {
    pub config: AppConfig,
    pub config_file_path: std::path::PathBuf,
    pub active_db_path: std::path::PathBuf,
    pub shutdown_policy: ShutdownPolicy,
}

pub struct AppDeps {
    pub log_db: std::sync::Arc<std::sync::Mutex<crate::core::database::LogDatabase>>,
    pub prefix_matcher: std::sync::Arc<crate::core::prefix::PrefixMatcher>,
    pub scp_engine: std::sync::Arc<std::sync::Mutex<crate::core::scp::ScpEngine>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotBadgeKind {
    NewDxcc,
    NewBand,
    Standard,
}

impl SpotBadgeKind {
    pub fn suffix(self) -> &'static str {
        match self {
            SpotBadgeKind::NewDxcc => " ⭐",
            SpotBadgeKind::NewBand => " ✨",
            SpotBadgeKind::Standard => "",
        }
    }

    pub fn color(self, palette: &crate::gui::theme::Palette) -> egui::Color32 {
        match self {
            SpotBadgeKind::NewDxcc => palette.accent_alt,
            SpotBadgeKind::NewBand => palette.success,
            SpotBadgeKind::Standard => palette.accent,
        }
    }
}

pub struct SpLogApp {
    pub paths: crate::gui::state::AppPaths,
    pub services: crate::gui::state::AppServices,
    pub runtime: crate::gui::state::AppRuntime,
    pub digital: crate::gui::state::DigitalModesState,

    // Konfiguracja stacji i lokalizacja
    pub my_station: StationProfile,

    // Stany formularza wprowadzania QSO
    pub entry_callsign: String,
    pub entry_band: String,
    pub entry_mode: String,
    pub entry_rst_sent: String,
    pub entry_rst_rcvd: String,
    /// Pełna wymiana kontestowa (np. "599 001" lub "599 15 1234").
    pub entry_exchange: String,
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
    /// Aktywny filtr drill-down ze statystyk (kliknięty słupek/wiersz).
    pub log_drill_filter: Option<DrillFilter>,

    // DX Cluster i pogoda kosmiczna
    pub cluster_spots: Vec<DxSpot>,
    /// Cache kolorów/odznak spotów klastra (unika przeliczania prefiksu i statusu
    /// nagród w każdej klatce). Czyszczony przy zmianie danych nagród.
    pub cluster_badge_cache: std::collections::HashMap<String, SpotBadgeKind>,
    // WSPR Monitor
    pub wspr: crate::gui::state::WsprState,

    // Moduł Satelitów
    pub satellites: crate::gui::state::SatelliteState,

    // Moduł Pogody Kosmicznej i Propagacji Słonecznej
    pub solar: crate::gui::state::SolarState,

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
    pub cw: crate::gui::state::CwMacroState,

    // Księga sprzętowa i CRUD
    pub ledger: crate::gui::state::EquipmentLedgerState,

    // Motyw i konfiguracja stacji
    pub font_scale: f32,
    pub font_family: String,
    pub distance_unit: String,

    // Wizualna Panorama Pasma (Band Map)
    pub show_bandmap_window: bool,
    pub bandmap_selected_band: String,
    pub bandmap_auto_track: bool,

    // Okna dialogowe
    pub updates: crate::gui::state::UpdateState,
    pub show_vfo_panel: bool,
    pub show_cluster_panel: bool,

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

    // PSK Reporter
    pub psk_reporter_enabled: bool,

    // N1MM Logger+ UDP broadcast
    pub n1mm_broadcast_enabled: bool,
    pub n1mm_broadcast_host: String,
    pub n1mm_broadcast_port: u16,

    // Voice keyer (SSB)
    pub voice_keyer: crate::gui::state::VoiceKeyerState,

    // Profile układu operatorskiego (workspace)
    pub workspace_profiles: Vec<WorkspaceProfile>,
    pub show_workspace_profiles_window: bool,

    // Multi-Op LAN (SYNC-6)
    pub sync_ui: crate::gui::SyncUiState,
    pub lan_sync_port: u16,
    pub lan_sync_server_ip: String,
    pub lan_sync_secret: String,
    pub show_multi_op_window: bool,

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

    pub online: crate::gui::state::OnlineSyncUiState,

    // Edycja QSO i historia Worked Before
    pub editing_qso: Option<QsoRecord>,
    pub past_qsos_for_active_call: Vec<QsoRecord>,

    // Kanał asynchroniczny QRZ
    pub qrz_lookup_tx: std::sync::mpsc::Sender<crate::cloud::qrz::CallbookData>,
    pub qrz_lookup_rx: std::sync::mpsc::Receiver<crate::cloud::qrz::CallbookData>,

    // Okna modułów
    pub world_map: crate::gui::state::WorldMapState,
    pub show_awards_matrix_window: bool,
    pub awards_matrix_tab: usize,
    pub awards_other_subtab: usize,
    pub awards_other_search: String,
    pub callbook_priority: Vec<crate::core::callbook::CallbookSource>,
    pub callbook_cache_enabled: bool,
    pub callbook_cache_ttl_days: u32,
    pub quick_access: crate::core::station::QuickAccessConfig,
    pub show_quick_access_customizer: bool,

    // Hamlib i rigctld
    pub cat_rig_id: u32,
    pub cat_model_search: String,
    pub cat_auto_start_rigctld: bool,
    pub cat_hamlib_source: String,
    pub cat_custom_rigctld_path: String,
    pub rigctld_supervisor: Option<crate::cat::supervisor::RigctldSupervisor>,

    // Stan GUI (dialogi, chrome, widoki)
    pub gui: crate::gui::state::GuiState,
    pub active_journal: crate::core::database::Journal,

    // Alerty, Live Auto-Upload i Toast
    pub live_auto_upload_clublog: bool,
    pub live_auto_upload_qrz: bool,
    pub sync_log_tx: std::sync::mpsc::Sender<(String, bool)>,
    pub sync_log_rx: std::sync::mpsc::Receiver<(String, bool)>,

    // SPLogbook
    pub show_column_settings: bool,
    pub logbook_columns: Vec<crate::core::station::LogColumn>,
    pub logbook_column_presets: Vec<crate::core::station::ColumnPreset>,
    pub column_preset_name: String,

    // CAT Channels
    pub cat_state_tx: std::sync::mpsc::Sender<crate::cat::hamlib::RigState>,
    pub cat_state_rx: std::sync::mpsc::Receiver<crate::cat::hamlib::RigState>,

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
    pub cluster_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,

    // Modułowy układ kafelków i okien pływających
    pub left_column_width: f32,
    pub right_column_width: f32,
    pub views: crate::gui::state::ViewsState,

    /// Tryb zakładek w kolumnach dokowanego układu (zapis w konfiguracji).
    pub tabbed_columns: bool,

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
    pub log_sort_column: u8, // 0=data, 1=znak, 2=pasmo, 3=emisja, 4=kraj, 5..=13 pozostałe kolumny
    pub log_sort_asc: bool,
    pub log_page: usize,
    pub log_page_size: usize,       // 25, 50, 100, 0=wszystkie
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

    // REST API key
    pub rest_api_key: String,

    // System pluginów użytkownika (Rhai)
    pub plugins_enabled: bool,
    pub plugins_dir: String,
    pub plugins_ui: crate::gui::state::PluginUiState,

    // Marketplace pluginów (katalog + instalacja jednym kliknięciem)
    pub marketplace: crate::gui::state::MarketplaceState,

    // Zintegrowany pasek asystenta operatora
    pub operator_assistant_enabled: bool,
    pub show_operator_assistant: bool,

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
    pub cat_proxy_rx:
        Option<tokio::sync::broadcast::Receiver<crate::cat::server::RigServerCommand>>,
    pub cat_poll_abort: Option<tokio::task::AbortHandle>,
    /// Nowoczesna konfiguracja wieloradiowa CAT (NATIVE-6B).
    pub cat_station: Option<crate::cat::config::CatStationConfig>,
    /// Centralny komponent aplikacyjny zarządzający węzłem RigHub i sesjami RigService.
    pub cat_runtime: crate::cat::runtime::CatRuntime,
    /// Kanał odbioru wyników asynchronicznego probe CAT wraz z numerem generacji.
    pub cat_probe_rx: Option<
        std::sync::mpsc::Receiver<(
            u64,
            Result<crate::cat::backend::RigProbeResult, crate::cat::rig::RigError>,
        )>,
    >,
    /// Flaga trwającego testu probe CAT.
    pub cat_probe_running: bool,
    /// Ostatni wynik testu probe CAT.
    pub cat_probe_result:
        Option<Result<crate::cat::backend::RigProbeResult, crate::cat::rig::RigError>>,
    /// Asynchroniczny stan probe CAT skorelowany z generacją formularza.
    pub cat_probe_state: crate::cat::probe::ProbeState,
    /// Bieżąca generacja formularza CAT probe.
    pub cat_probe_generation: u64,
    /// Aktywny task asynchronicznego probe CAT.
    pub cat_probe_task: Option<tokio::task::JoinHandle<()>>,
    /// Kanał odbioru asynchronicznie uruchamianego CatRuntime.
    pub cat_start_rx: Option<
        std::sync::mpsc::Receiver<
            Result<crate::cat::runtime::CatRuntime, crate::cat::runtime::CatRuntimeError>,
        >,
    >,
    /// Indeks aktualnie edytowanego radia w oknie ustawień CAT.
    pub cat_selected_rig_idx: usize,
    /// Pamięć podręczna formularza konfiguracji backendów w UI.
    pub cat_form_cache: crate::gui::cat_settings::FormVariantCache,

    // VFO Konsola radiowa & DSP
    pub vfo_split_offset_khz: f64,
    pub vfo_filter_preset: String,
    pub vfo_preamp_att: String,
    pub vfo_nb_nr: bool,
    pub vfo_agc_speed: String,

    // Boczny panel filtrów klastra i POTA/SOTA
    pub cluster_show_filter_sidebar: bool,
    pub cluster_search_query: String,
    pub cluster_filter_band_selection: String,
    pub cluster_filter_mode_selection: String,
    pub cluster_filter_status: String,
    pub cluster_filter_source: String,
    pub cluster_filter_pota_sota_only: bool,

    // Wyszukiwanie i usuwanie duplikatów w logu
    pub duplicates: crate::gui::state::DuplicatesState,

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
        cc: &eframe::CreationContext<'_>,
        log_db: std::sync::Arc<std::sync::Mutex<crate::core::database::LogDatabase>>,
        prefix_matcher: std::sync::Arc<crate::core::prefix::PrefixMatcher>,
        scp_engine: std::sync::Arc<std::sync::Mutex<crate::core::scp::ScpEngine>>,
        app_config: AppConfig,
        config_file_path: std::path::PathBuf,
        active_db_path: std::path::PathBuf,
    ) -> Self {
        let init = AppInit {
            config: app_config,
            config_file_path,
            active_db_path,
            shutdown_policy: ShutdownPolicy::Backup,
        };
        let deps = AppDeps {
            log_db,
            prefix_matcher,
            scp_engine,
        };
        let mut app = Self::build(init, deps);
        app.configure_egui(&cc.egui_ctx);
        app.start_background();
        app
    }

    pub fn for_tests(init: AppInit, deps: AppDeps) -> Self {
        Self::build(init, deps)
    }

    pub fn build(mut init: AppInit, deps: AppDeps) -> Self {
        let (active_journal, recent_qsos, qso_numbers) = {
            let db = deps
                .log_db
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let journal = match db.get_active_journal() {
                Ok(journal) => journal,
                Err(e) => {
                    log::error!("Failed to fetch active logbook: {e}");
                    Journal::default()
                }
            };
            let qsos = db
                .get_recent_qsos_for_journal(&journal.id, 100)
                .unwrap_or_default();
            let numbers = db
                .qso_numbers_for_journal(&journal.id)
                .unwrap_or_else(|error| {
                    log::error!("Failed to calculate QSO serial numbers: {error}");
                    std::collections::HashMap::new()
                });
            (journal, qsos, numbers)
        };

        let (cat_state_tx, cat_state_rx) = std::sync::mpsc::channel();

        let cat_poll_abort = None;

        if crate::cluster::telnet::is_dead_legacy_cluster_host(&init.config.cluster_host) {
            init.config.cluster_host = "dxcluster.pl".to_string();
            init.config.cluster_port = 8000;
        }

        let sample_spots = Vec::new();

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

        let (qrz_lookup_tx, qrz_lookup_rx) = std::sync::mpsc::channel();
        let (sync_log_tx, sync_log_rx) = std::sync::mpsc::channel();

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .unwrap_or_else(|| {
                std::env::var("APPDATA").map_or_else(
                    |_| std::path::PathBuf::from("."),
                    |p| std::path::PathBuf::from(p).join("SPLogbook"),
                )
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

        // Cache offline wyników callbook — obok bazy dziennika (katalog zapisywalny).
        let callbook_cache_path = if init.config.callbook_cache_enabled {
            init.active_db_path
                .parent()
                .map(|p| p.join("callbook_cache.db"))
        } else {
            None
        };
        let callbook_cache_ttl_days = init.config.callbook_cache_ttl_days;
        let callbook_priority = init.config.callbook_priority.clone();

        let local_callbook = std::sync::Arc::new(
            crate::core::callbook::LocalCallbook::new(cb_path, srv_path)
                .with_cache(callbook_cache_path, callbook_cache_ttl_days),
        );

        let awards_engine = Arc::new(Mutex::new(AwardsEngine::new()));
        let event_bus = crate::core::events::EventBus::default();
        let qso_ingestor = crate::core::ingest::QsoIngestor::new(
            deps.log_db.clone(),
            awards_engine.clone(),
            deps.scp_engine.clone(),
            deps.prefix_matcher.clone(),
            event_bus.clone(),
        );

        let appearance = crate::gui::state::AppearanceState::from_config(&init.config);
        let lang = appearance.language;
        let show_wizard = !init.config.is_configured && init.config.station.callsign.is_empty();
        let mut chrome = crate::gui::state::ChromeState::default();
        chrome.show_welcome_wizard = show_wizard;
        let gui = crate::gui::state::GuiState {
            chrome,
            appearance,
            dialogs: crate::gui::state::DialogState::default(),
            workspace: crate::gui::state::DockWorkspace::default(),
        };
        let views = crate::gui::state::ViewsState::from_config(&init.config);
        let online = crate::gui::state::OnlineSyncUiState::from_config(&init.config);
        let paths = crate::gui::state::AppPaths::new(
            init.config_file_path,
            init.active_db_path,
            init.config.secret_store_id.clone(),
        );
        let services = crate::gui::state::AppServices::new(
            deps.log_db,
            deps.prefix_matcher,
            deps.scp_engine,
            awards_engine,
            Arc::new(ServiceDatabase::open()),
            local_callbook,
            Arc::new(Mutex::new(
                crate::cloud::scheduler::UploadScheduler::load_from_disk(),
            )),
            event_bus,
            crate::plugins::PluginEngine::new(),
            qso_ingestor,
        );
        let runtime =
            crate::gui::state::AppRuntime::new(init.shutdown_policy, init.config.cat_enabled);
        let digital = crate::gui::state::DigitalModesState::new();
        let mut app = Self {
            paths,
            services,
            runtime,
            digital,
            my_station: init.config.station.clone(),

            entry_callsign: String::new(),
            callsign_corrections: Vec::new(),
            entry_band: "20m".to_string(),
            entry_mode: "CW".to_string(),
            entry_rst_sent: "599".to_string(),
            entry_rst_rcvd: "599".to_string(),
            entry_exchange: String::new(),
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
            cat_connected: false,
            vfo_split: false,

            recent_qsos,
            qso_numbers,
            log_search_query: String::new(),
            log_drill_filter: None,

            cluster_spots: sample_spots,
            cluster_badge_cache: std::collections::HashMap::new(),
            wspr: crate::gui::state::WsprState::default(),

            satellites: crate::gui::state::SatelliteState::default(),

            solar: crate::gui::state::SolarState::with_propagation_history(
                init.config.propagation_history,
            ),

            show_contest_window: false,
            contest_name: "SP DX Contest".to_string(),
            contest_stx: 1,
            contest_qsos: 0,
            contest_points: 0,
            contest_mults: 0,
            show_custom_contest_editor: false,
            custom_contest_edit_idx: None,
            custom_contest_draft: crate::core::station::CustomContest::default(),
            custom_contests: init.config.custom_contests.clone(),

            cw: crate::gui::state::CwMacroState::default(),

            ledger: crate::gui::state::EquipmentLedgerState::with_items(init.config.equipment),

            font_scale: init.config.font_scale,
            font_family: init.config.font_family.clone(),
            distance_unit: init.config.distance_unit.clone(),

            show_bandmap_window: false,
            bandmap_selected_band: "20m".to_string(),
            bandmap_auto_track: true,

            show_vfo_panel: true,
            show_cluster_panel: true,

            cat_host: init.config.cat_host,
            cat_port: init.config.cat_port,
            cat_poll_rate_ms: init.config.cat_poll_rate_ms,
            cat_rig_model: init.config.cat_rig_model,
            cat_serial_port: init.config.cat_serial_port.clone(),
            cat_baud_rate: init.config.cat_baud_rate,
            cat_test_result: None,
            show_cat_settings_window: false,

            rotor_host: init.config.rotor_host,
            rotor_port: init.config.rotor_port,
            rotor_test_result: None,

            cat_backend: init.config.cat_backend.clone(),
            tci_host: init.config.tci_host,
            tci_port: init.config.tci_port,
            tci_test_result: None,

            fldigi_enabled: init.config.fldigi_enabled,
            fldigi_host: init.config.fldigi_host,
            fldigi_port: init.config.fldigi_port,

            psk_reporter_enabled: init.config.psk_reporter_enabled,

            n1mm_broadcast_enabled: init.config.n1mm_broadcast_enabled,
            n1mm_broadcast_host: init.config.n1mm_broadcast_host,
            n1mm_broadcast_port: init.config.n1mm_broadcast_port,

            voice_keyer: crate::gui::state::VoiceKeyerState::with_messages(
                init.config.voice_keyer_messages,
            ),

            workspace_profiles: init.config.workspace_profiles,
            show_workspace_profiles_window: false,
            sync_ui: crate::gui::SyncUiState::new(),

            lan_sync_port: init.config.lan_sync_port,
            lan_sync_server_ip: init.config.lan_sync_server_ip,
            lan_sync_secret: init.config.lan_sync_secret,
            show_multi_op_window: false,

            lotw_tqsl_path: init.config.lotw_tqsl_path,
            lotw_station_name: init.config.lotw_station_name,
            lotw_username: init.config.lotw_username,
            lotw_password: init.config.lotw_password,

            qrz_username: init.config.qrz_username,
            qrz_password: init.config.qrz_password,
            qrz_api_key: init.config.qrz_api_key,
            qrz_auto_lookup: init.config.qrz_auto_lookup,

            online,

            editing_qso: None,
            past_qsos_for_active_call: Vec::new(),

            qrz_lookup_tx,
            qrz_lookup_rx,

            world_map: crate::gui::state::WorldMapState::default(),
            show_awards_matrix_window: false,
            awards_matrix_tab: 0,
            awards_other_subtab: 0,
            awards_other_search: String::new(),
            callbook_priority,
            callbook_cache_enabled: init.config.callbook_cache_enabled,
            callbook_cache_ttl_days,
            quick_access: init.config.quick_access,
            show_quick_access_customizer: false,

            cat_rig_id: init.config.cat_rig_id,
            cat_model_search: String::new(),
            cat_auto_start_rigctld: init.config.cat_auto_start_rigctld,
            cat_hamlib_source: init.config.cat_hamlib_source,
            cat_custom_rigctld_path: init.config.cat_custom_rigctld_path,
            rigctld_supervisor: None,

            gui,
            active_journal,

            live_auto_upload_clublog: init.config.live_auto_upload_clublog,
            live_auto_upload_qrz: init.config.live_auto_upload_qrz,
            sync_log_tx,
            sync_log_rx,

            show_column_settings: false,
            updates: crate::gui::state::UpdateState::default(),
            logbook_columns: init.config.logbook_columns.clone(),
            logbook_column_presets: init.config.logbook_column_presets.clone(),
            column_preset_name: String::new(),

            compact_hud_mode: init.config.compact_hud_mode,
            tabbed_columns: init.config.tabbed_columns,
            hud_always_on_top: init.config.hud_always_on_top,
            hud_saved_pos: init.config.hud_saved_pos,
            hud_saved_size: init.config.hud_saved_size,
            hud_operating_bar: init.config.hud_operating_bar,
            main_window_pos: init.config.main_window_pos,
            main_window_size: init.config.main_window_size,
            main_window_maximized: init.config.main_window_maximized,

            entry_iota: String::new(),
            entry_state: String::new(),
            entry_sota: String::new(),
            entry_pota: String::new(),
            entry_qsl_manager: String::new(),
            active_qsl_manager_record: None,

            cloudlog_url: init.config.cloudlog_url,
            cloudlog_api_key: init.config.cloudlog_api_key,
            hrdlog_username: init.config.hrdlog_username,
            hrdlog_upload_code: init.config.hrdlog_upload_code,
            hamqth_username: init.config.hamqth_username,
            hamqth_password: init.config.hamqth_password,

            cat_state_tx,
            cat_state_rx,

            cluster_host: init.config.cluster_host,
            cluster_port: init.config.cluster_port,
            cluster_callsign: init.config.cluster_callsign,
            cluster_auto_connect: init.config.cluster_auto_connect,
            cluster_connected: false,
            cluster_connecting: false,
            cluster_status_text: tr(key::CLUSTER_DISCONNECTED, lang).to_string(),
            cluster_filter_current_band: init.config.cluster_filter_current_band,
            cluster_hide_ft8: init.config.cluster_hide_ft8,
            cluster_hide_skimmers: init.config.cluster_hide_skimmers,
            custom_clusters: init.config.custom_clusters,
            show_add_cluster_dialog: false,
            new_cluster_name: String::new(),
            new_cluster_host: String::new(),
            new_cluster_port: 7300,
            cluster_event_rx: None,
            cluster_stop_tx: None,
            cluster_cmd_tx: None,

            left_column_width: init.config.left_column_width,
            right_column_width: init.config.right_column_width,

            views,

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

            // REST API
            rest_api_key: init.config.rest_api_key.clone(),

            // System pluginów użytkownika (Rhai)
            plugins_enabled: init.config.plugins_enabled,
            plugins_dir: init.config.plugins_dir.clone(),
            plugins_ui: crate::gui::state::PluginUiState::default(),

            marketplace: crate::gui::state::MarketplaceState::default(),

            // Zintegrowany pasek asystenta operatora
            operator_assistant_enabled: init.config.operator_assistant_enabled,
            show_operator_assistant: init.config.operator_assistant_enabled,

            // PTT
            ptt_active: false,

            // Dwuetapowy CAT i udostępnianie (CAT Sharing)
            cat_mfg_selected: "Wszystkie".to_string(),
            cat_conn_type: if init.config.cat_backend == "tci" {
                "tci".to_string()
            } else if init.config.cat_auto_start_rigctld || !init.config.cat_serial_port.is_empty()
            {
                "serial".to_string()
            } else {
                "tcp".to_string()
            },
            cat_sharing_enabled: init.config.cat_sharing_enabled,
            cat_sharing_port: init.config.cat_sharing_port,
            cat_sharing_active: false,
            cat_shared_state: std::sync::Arc::new(std::sync::RwLock::new(
                crate::cat::hamlib::RigState::default(),
            )),
            cat_proxy_server: None,
            cat_proxy_rx: None,
            cat_poll_abort,
            cat_station: init.config.cat_station.clone(),
            cat_runtime: crate::cat::runtime::CatRuntime::new(),
            cat_probe_rx: None,
            cat_probe_running: false,
            cat_probe_result: None,
            cat_probe_state: crate::cat::probe::ProbeState::Idle,
            cat_probe_generation: 0,
            cat_probe_task: None,
            cat_start_rx: None,
            cat_selected_rig_idx: 0,
            cat_form_cache: crate::gui::cat_settings::FormVariantCache::default(),

            // VFO Konsola radiowa & DSP
            vfo_split_offset_khz: 1.0,
            vfo_filter_preset: "FIL2".to_string(),
            vfo_preamp_att: "OFF".to_string(),
            vfo_nb_nr: false,
            vfo_agc_speed: "MID".to_string(),

            // Boczny panel filtrów klastra
            cluster_show_filter_sidebar: false,
            cluster_search_query: String::new(),
            cluster_filter_band_selection: "ALL".to_string(),
            cluster_filter_mode_selection: "ALL".to_string(),
            cluster_filter_status: "ALL".to_string(),
            cluster_filter_source: "ALL".to_string(),
            cluster_filter_pota_sota_only: false,

            // Wyszukiwanie duplikatów
            duplicates: crate::gui::state::DuplicatesState::default(),

            // Profile stacji roboczej
            show_station_profiles_window: false,
            station_profiles: if init.config.station_profiles.is_empty() {
                let mut def = init.config.station.clone();
                if def.id.is_empty() {
                    def.id = "default".to_string();
                }
                if def.name.is_empty() {
                    def.name = "Main (Home QTH)".to_string();
                }
                vec![def]
            } else {
                init.config.station_profiles.clone()
            },
            active_profile_id: init.config.active_profile_id.clone(),
            focus_callsign_requested: false,
            lookup_debounce_until: None,
        };

        app.rebuild_awards_full();

        // Zbuduj (lub wczytaj z konfiguracji) układ dokowania paneli.
        app.build_dock_state(init.config.dock_layout.as_ref());
        app
    }

    pub fn configure_egui(&self, ctx: &egui::Context) {
        // Rejestracja czcionki symboli systemowych (seguisym na Windows, DejaVu/Noto na Linux)
        // dla pełnej obsługi znaków Unicode, symboli radiowych, strzałek ▲/▼, planet, satelitów i statusów
        let mut font_defs = egui::FontDefinitions::default();
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let font_candidates = [
            std::path::Path::new(&win_dir)
                .join("Fonts")
                .join("seguisym.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf"),
            std::path::PathBuf::from("/usr/share/fonts/truetype/freefont/FreeSans.ttf"),
        ];

        for path in &font_candidates {
            if let Ok(bytes) = std::fs::read(path) {
                font_defs.font_data.insert(
                    "symbol_font".to_owned(),
                    egui::FontData::from_owned(bytes).into(),
                );
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
        if !self.font_family.is_empty() {
            if let Some(path) = resolve_font_path(&self.font_family) {
                if let Ok(bytes) = std::fs::read(&path) {
                    font_defs.font_data.insert(
                        "custom_ui_font".to_owned(),
                        egui::FontData::from_owned(bytes).into(),
                    );
                    if let Some(prop) = font_defs.families.get_mut(&egui::FontFamily::Proportional)
                    {
                        prop.insert(0, "custom_ui_font".to_owned());
                    }
                    let mono_family = self.font_family == "Consolas";
                    if mono_family {
                        if let Some(mono) = font_defs.families.get_mut(&egui::FontFamily::Monospace)
                        {
                            mono.insert(0, "custom_ui_font".to_owned());
                        }
                    }
                }
            }
        }

        ctx.set_fonts(font_defs);
    }

    pub fn start_background(&mut self) {
        let ws_tx = self.digital.wsjtx_tx.clone();
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
        // Wczytanie pluginów użytkownika (Rhai) z katalogu i uruchomienie on_startup().
        {
            let dir = std::path::PathBuf::from(&self.plugins_dir);
            self.services
                .plugin_engine
                .set_enabled(self.plugins_enabled);
            self.services.plugin_engine.load_dir(&dir);
            let total = self.qso_numbers.len() as i64;
            self.services.plugin_engine.set_qso_count(total);
            self.services.plugin_engine.run_startup();
        }

        if self.runtime.initial_cat_enabled {
            self.start_cat_service();
        }

        if self.cat_sharing_enabled {
            self.toggle_cat_proxy_server();
        }

        if self.runtime.rest_api_enabled {
            self.start_rest_api_server();
        }

        if self.cluster_auto_connect {
            self.connect_dx_cluster();
        }
    }
}

impl TabViewer for SpLogApp {
    type Tab = String;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.as_str())
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        self.tile_title(tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let tile_id = tab.clone();
        let mut action_popout = false;
        let mut action_close = false;

        egui::Frame::group(ui.style())
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Własne widżety nagłówka (CAT, SPLIT, Szukaj, Spot)
                    self.render_tile_header_custom(&tile_id, ui);

                    // Przyciski odpinania i ukrywania (wyrównane do prawej)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let lang = self.gui.appearance.language;
                        if ui
                            .button("✕")
                            .on_hover_text(tr(key::WINDOW_HIDE_TOOLTIP, lang))
                            .clicked()
                        {
                            action_close = true;
                        }
                        if ui
                            .button("🗗")
                            .on_hover_text(tr(key::WINDOW_POPOUT_TOOLTIP, lang))
                            .clicked()
                        {
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
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

        // Wczesna faza asynchroniczna: wtyczki, WSPR, API, upload scheduler, satelity i solar
        self.poll_early_async_events();

        // Faza obsługi skrótów klawiaturowych, palety poleceń i wejść globalnych
        self.handle_input_and_shortcuts(ctx);

        // Debounced lookup Callbook/QRZ po wpisaniu znaku
        self.process_debounced_lookup();

        // Główna faza asynchroniczna: aktualizacje, CAT, REST API, proxy, klaster, callbook, sync, WSJT-X, JS8Call, Multi-Op
        self.poll_main_async_events(ctx);

        // Zastosowanie wybranego motywu, stylów i skali UI (apply-on-change przez ensure_applied).
        crate::gui::theme::ensure_applied(ctx, self.gui.appearance.theme_preset, self.font_scale);

        // Faza Chrome: paski menu i statusu stacji
        self.render_chrome_bars(ui);

        // Główny obszar roboczy - swobodny pulpit stacji SPLogbook (MDI Desktop)
        egui::CentralPanel::default().show(ui, |ui| {
            let lang = self.gui.appearance.language;
            let palette = ui.palette();
            if self.compact_hud_mode {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.heading(
                        egui::RichText::new(tr(key::APP_COMPACT_HUD_HEADING, lang))
                            .size(22.0)
                            .color(palette.accent),
                    );
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(tr(key::APP_COMPACT_HUD_DESC1, lang)).size(14.0));
                    ui.label(
                        egui::RichText::new(tr(key::APP_COMPACT_HUD_DESC2, lang))
                            .color(palette.text_muted),
                    );
                    ui.add_space(16.0);
                    if ui
                        .button(
                            egui::RichText::new(tr(key::APP_COMPACT_HUD_RETURN, lang))
                                .size(14.0)
                                .strong(),
                        )
                        .clicked()
                    {
                        self.compact_hud_mode = false;
                        self.save_station_config();
                    }
                });
            } else {
                let total_visible = self.views.visible_count();

                if total_visible == 0 {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.heading(
                            egui::RichText::new("📻 SPLogbook Workspace")
                                .size(26.0)
                                .color(palette.accent),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(tr(key::APP_EMPTY_TILES_TITLE, lang)).size(14.0),
                        );
                        ui.label(
                            egui::RichText::new(tr(key::APP_EMPTY_TILES_DESC, lang))
                                .color(palette.text_muted),
                        );
                        ui.add_space(16.0);
                        if ui
                            .button(
                                egui::RichText::new(tr(key::APP_EMPTY_TILES_RESTORE_BTN, lang))
                                    .size(15.0)
                                    .strong(),
                            )
                            .clicked()
                        {
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
                    let mut dock_state = std::mem::replace(
                        &mut self.gui.workspace.dock_state,
                        DockState::new(vec![]),
                    );
                    DockArea::new(&mut dock_state)
                        .style(Style::from_egui(ui.style().as_ref()))
                        .show_add_buttons(false)
                        .show_add_popup(false)
                        .show_close_buttons(false)
                        .show_inside(ui, self);
                    self.gui.workspace.dock_state = dock_state;

                    // Zastosuj zmiany widoczności/odpięcia wykonane przyciskami nagłówka.
                    self.sync_dock_state();

                    // Persystuj układ dokowania tylko po puszczeniu przycisku myszy (lub przy pierwszym zapisie),
                    // aby uniknąć alokacji drzewa JSON w każdej klatce (60 FPS).
                    let pointer_released = ui.ctx().input(|i| i.pointer.any_released());
                    if pointer_released || self.gui.workspace.last_saved_dock_layout.is_none() {
                        let current = self.serialize_dock_layout();
                        if current != self.gui.workspace.last_saved_dock_layout {
                            self.gui.workspace.last_saved_dock_layout = current;
                            self.save_station_config();
                        }
                    }

                    // Dyskretny pasek pomocy i szybkiego resetowania układu na dole pulpitu
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new(tr(key::APP_RESTORE_LAYOUT_BTN, lang))
                                        .size(11.0),
                                )
                                .on_hover_text(tr(key::APP_RESTORE_LAYOUT_TOOLTIP, lang))
                                .clicked()
                            {
                                self.reset_panel_layout();
                                self.save_station_config();
                            }
                            ui.label(
                                egui::RichText::new(tr(key::APP_DOCK_HINT, lang))
                                    .size(11.0)
                                    .color(palette.text_subtle),
                            );
                        });
                    });
                }
            }
        });

        // W zależności od trybu: Mini HUD (tryb kompaktowy) lub pełny pulpit roboczy
        if self.compact_hud_mode {
            crate::gui::mini_hud::render_mini_hud_bar(self, ui);
        } else {
            // Pływające okna modułów (pop-out windows) — renderowane tylko wtedy,
            // gdy panel jest „odpięty” (floating). Kafelki zadokowane rysuje
            // układ kolumnowy w CentralPanel poniżej.
            if self.views.panel_qso.floating {
                render_qso_entry_window(self, ctx);
            }
            if self.views.panel_vfo.floating {
                render_vfo_window(self, ctx);
            }
            if self.views.panel_log.floating {
                render_logbook_window(self, ctx);
            }
            if self.views.panel_cluster.floating {
                render_cluster_window(self, ctx);
            }
            if self.views.panel_solar.floating {
                render_solar_window(self, ctx);
            }
            if self.views.panel_bandmap.floating {
                render_bandmap_window(self, ctx);
            }
            if self.views.panel_satellites.floating {
                render_satellites_window(self, ctx);
            }
            if self.views.panel_world_map.floating {
                render_world_map_window(self, ctx);
            }
            if self.views.panel_waterfall.floating {
                crate::gui::waterfall_panel::render_waterfall_window(self, ctx);
            }
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
        crate::gui::multi_op_window::render_multi_op_window(self, ctx);
        crate::gui::find_duplicates::render_find_duplicates_window(self, ctx);
        crate::gui::station_profiles::render_station_profiles_window(self, ctx);
        render_voice_keyer_window(self, ctx);
        crate::gui::workspace_profiles::render_workspace_profiles_window(self, ctx);
        crate::gui::operator_assistant::render_operator_assistant(self, ctx);
        crate::gui::plugin_manager::render_plugin_manager(self, ctx);
        crate::gui::marketplace::render_marketplace(self, ctx);

        // Okna dialogowe i narzędzia pomocnicze
        if self.gui.dialogs.journal_dialog.is_open {
            let mut switched_journal = None;
            let lang = self.gui.appearance.language;
            if let Ok(db) = self.services.log_db.lock() {
                self.gui
                    .dialogs
                    .journal_dialog
                    .render(ctx, &db, lang, &mut |j| {
                        switched_journal = Some(j.clone());
                    });
            }
            if let Some(j) = switched_journal {
                self.active_journal = j.clone();
                self.my_station.callsign.clone_from(&j.station_callsign);
                self.my_station.operator.clone_from(&j.operator);
                self.my_station.gridsquare.clone_from(&j.my_gridsquare);
                self.my_station.pga_gmina = if j.my_pga.is_empty() {
                    None
                } else {
                    Some(j.my_pga.clone())
                };
                self.reload_qsos();
                self.gui.chrome.status_toast = Some((
                    Msg::new(key::PROFILE_SWITCHED_JOURNAL)
                        .arg("name", &j.name)
                        .text(self.gui.appearance.language),
                    std::time::Instant::now(),
                ));
            }
        }

        if self.gui.dialogs.advanced_filter_dialog.is_open {
            let mut filter_results = None;
            let mut clear_filter = false;
            let journal_id = self.active_journal.id.clone();
            let my_call = self.my_station.callsign.clone();
            let lang = self.gui.appearance.language;
            if let Ok(db) = self.services.log_db.lock() {
                self.gui.dialogs.advanced_filter_dialog.render(
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

        if self.gui.dialogs.cw_terminal_dialog.is_open {
            let my_call = self.my_station.callsign.clone();
            self.gui
                .dialogs
                .cw_terminal_dialog
                .render(ctx, &my_call, self.gui.appearance.language);
        }

        if self.gui.dialogs.qsl_designer_dialog.is_open {
            let my_call = self.my_station.callsign.clone();
            if let Ok(db) = self.services.log_db.lock() {
                self.gui.dialogs.qsl_designer_dialog.render(
                    ctx,
                    &db,
                    &my_call,
                    self.gui.appearance.language,
                );
            }
        }
        if self.wspr.show_window {
            crate::gui::wspr_panel::render_wspr_window(self, ctx);
        }

        // 1. Zdalne wybudzanie stacji Wake-on-LAN (WOL)
        self.gui
            .dialogs
            .wol_dialog
            .show(ctx, self.gui.appearance.language);

        // 2. Położenie Księżyca i Słońca (EME / Kalkulator astronomiczny)
        let my_grid = self.my_station.gridsquare.clone();
        self.gui.dialogs.astronomy_dialog.show(
            ctx,
            &my_grid,
            &mut self.rotor_state,
            self.gui.appearance.language,
        );

        // 3. Moduł SOTA / POTA
        self.gui
            .dialogs
            .sota_dialog
            .show(ctx, &self.recent_qsos, self.gui.appearance.language);

        // 3b. Konfigurowalny eksport CSV
        if self.gui.dialogs.csv_export_dialog.is_open {
            if let Some(req) = self
                .gui
                .dialogs
                .csv_export_dialog
                .render(ctx, self.gui.appearance.language)
            {
                self.perform_csv_export(&req);
            }
        }

        // 4. Przeglądarka wysp IOTA
        {
            let awards = self
                .services
                .awards_engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(iota) = self.gui.dialogs.iota_dialog.show(
                ctx,
                &self.services.service_db,
                &awards,
                self.gui.appearance.language,
            ) {
                self.entry_iota = iota;
            }

            // 5. Przeglądarka stanów USA (WAS)
            if let Some(state) = self.gui.dialogs.states_dialog.show(
                ctx,
                &self.services.service_db,
                &awards,
                self.gui.appearance.language,
            ) {
                self.entry_state = state;
            }
        }

        // 6. Baza menedżerów QSL
        if let Some(qslm) = self.gui.dialogs.qsl_manager_dialog.show(
            ctx,
            &self.services.service_db,
            self.gui.appearance.language,
        ) {
            self.entry_qsl_manager = qslm;
        }

        // 7. Przeglądarka fotografii i kart QSL korespondenta
        self.gui
            .dialogs
            .photo_viewer_dialog
            .show(ctx, self.gui.appearance.language);

        // 8. Menedżer prefiksów DXCC
        self.gui.dialogs.prefix_manager_dialog.show(
            ctx,
            &self.services.service_db,
            self.gui.appearance.language,
        );

        // 9. Formularz wysyłania własnego spotu do DX Cluster
        if let Some(sub) = self.gui.dialogs.send_spot_dialog.show(
            ctx,
            &self.my_station.callsign,
            self.gui.appearance.language,
        ) {
            if let Some(ref tx) = self.cluster_cmd_tx {
                let telnet_cmd = format!(
                    "DX {:.1} {} {}\r\n",
                    sub.freq_khz,
                    sub.dx_call.trim().to_uppercase(),
                    sub.comment.trim()
                );
                let _ = tx.send(telnet_cmd);
            }
            let band_str = crate::core::bandplan::get_band_by_freq((sub.freq_khz * 1000.0) as u64)
                .map_or_else(|| "HF".to_string(), |b| b.name.to_string());
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
            let toast_msg = Msg::new(key::CLUSTER_SPOT_SENT)
                .arg("call", &sub.dx_call)
                .arg("freq", format!("{:.1}", sub.freq_khz))
                .text(self.gui.appearance.language);
            self.gui.chrome.status_toast = Some((toast_msg, std::time::Instant::now()));
        }

        // Faza Chrome: toasty, okno o programie, paleta, manual, skróty, legenda
        self.render_chrome_overlays(ctx);

        // Resetowanie flagi po narysowaniu i spzycjonowaniu okien w danej klatce
        self.gui.workspace.reset_layout_requested = false;

        // Płynne odświeżanie zegara UTC, stanu radia CAT oraz spotów sieciowych bez obciążania procesora (4 Hz)
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cat_runtime
            .shutdown_blocking(std::time::Duration::from_millis(1500));
        if let Some(mut sup) = self.rigctld_supervisor.take() {
            sup.stop();
        }
        // Zapisanie geometrii okna głównego (i pozostałej konfiguracji) przy zamknięciu aplikacji.
        self.save_station_config();
    }
}

impl Drop for SpLogApp {
    fn drop(&mut self) {
        if self.runtime.shutdown_policy == ShutdownPolicy::None {
            return;
        }

        let backup_dir = self
            .paths
            .active_db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("backups");
        let _ = std::fs::create_dir_all(&backup_dir);
        let _ = BackupManager::backup_database(&self.paths.active_db_path, &backup_dir);

        let db = self
            .services
            .log_db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(qsos) = db.get_recent_qsos(5000) {
            let adif =
                crate::core::adif::export_adif(&qsos, "SPLogbook", &self.my_station.callsign);
            let _ = BackupManager::backup_adif(&adif, &backup_dir);
        }
    }
}

#[path = "app_async.rs"]
mod app_async;
#[path = "app_cat.rs"]
mod app_cat;
#[path = "app_config.rs"]
mod app_config;
#[path = "app_files.rs"]
mod app_files;
#[path = "app_lookup.rs"]
mod app_lookup;
#[path = "app_plugins.rs"]
mod app_plugins;
#[path = "app_qso.rs"]
mod app_qso;
#[path = "app_services.rs"]
mod app_services;
#[path = "app_solar.rs"]
mod app_solar;

#[path = "app_chrome.rs"]
mod app_chrome;
#[path = "app_input.rs"]
mod app_input;

#[path = "app_export.rs"]
mod app_export;

#[path = "app_layout.rs"]
mod app_layout;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::database::{Journal, LogDatabase};
    use crate::core::prefix::PrefixMatcher;
    use crate::core::qso::QsoRecord;
    use crate::core::scp::ScpEngine;
    use crate::core::station::AppConfig;
    use std::sync::{Arc, Mutex};

    fn test_deps() -> AppDeps {
        AppDeps {
            log_db: Arc::new(Mutex::new(LogDatabase::open_in_memory().unwrap())),
            prefix_matcher: Arc::new(PrefixMatcher::empty()),
            scp_engine: Arc::new(Mutex::new(ScpEngine::new())),
        }
    }

    #[test]
    fn build_loads_active_journal_and_qso_numbers() {
        let deps = test_deps();
        let (first_id, second_id) = {
            let db = deps.log_db.lock().unwrap();
            let j = Journal {
                id: "test-id".to_string(),
                name: "Test".to_string(),
                ..Default::default()
            };
            db.create_journal(&j).unwrap();
            db.set_active_journal("test-id").unwrap();

            let mut q1 = QsoRecord::new("SP1ABC", "20m", "CW");
            q1.journal_id = Some("test-id".to_string());
            q1.qso_date = "2026-01-01".to_string();
            q1.time_on = "10:00:00".to_string();
            let id1 = db.insert_qso(&q1).unwrap();

            let mut q2 = QsoRecord::new("SP2DEF", "40m", "SSB");
            q2.journal_id = Some("test-id".to_string());
            q2.qso_date = "2026-01-01".to_string();
            q2.time_on = "10:15:00".to_string();
            let id2 = db.insert_qso(&q2).unwrap();

            (id1, id2)
        };

        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };

        let app = SpLogApp::for_tests(init, deps);
        assert_eq!(app.active_journal.name, "Test");
        assert_eq!(app.active_journal.id, "test-id");
        assert_eq!(app.qso_numbers.len(), 2);
        assert_eq!(app.qso_numbers.get(&first_id), Some(&1));
        assert_eq!(app.qso_numbers.get(&second_id), Some(&2));
        assert_eq!(app.recent_qsos.len(), 2);
    }

    #[test]
    fn build_with_empty_database() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let app = SpLogApp::for_tests(init, deps);
        assert_eq!(app.active_journal.name, "Główny Dziennik");
    }

    #[test]
    fn build_does_not_start_background_services() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let _app = SpLogApp::for_tests(init, deps);

        // Ensure UDP 2237 is free
        let socket = std::net::UdpSocket::bind("0.0.0.0:2237");
        assert!(socket.is_ok(), "Port 2237 was occupied by build!");
    }

    #[test]
    fn drop_with_none_policy_does_not_touch_disk() {
        let temp = tempfile::tempdir().unwrap();
        let active_db = temp.path().join("db.sqlite");
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: temp.path().join("config.json"),
            active_db_path: active_db.clone(),
            shutdown_policy: ShutdownPolicy::None,
        };

        {
            let _app = SpLogApp::for_tests(init, deps);
        }

        assert!(!temp.path().join("backups").exists());
    }

    #[test]
    fn drop_with_backup_policy_creates_backup() {
        let temp = tempfile::tempdir().unwrap();
        let active_db = temp.path().join("db.sqlite");
        std::fs::write(&active_db, "dummy data").unwrap();

        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: temp.path().join("config.json"),
            active_db_path: active_db.clone(),
            shutdown_policy: ShutdownPolicy::Backup,
        };

        {
            let _app = SpLogApp::for_tests(init, deps);
        }

        assert!(temp.path().join("backups").exists());
    }

    #[test]
    fn gui_import_adif_content_success_duplicates_and_state_refresh() {
        let deps = test_deps();
        let scp = Arc::clone(&deps.scp_engine);
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);

        let adif = "<CALL:6>SP6INA <BAND:3>20m <MODE:2>CW <QSO_DATE:8>20261012 <TIME_ON:6>120000 <RST_SENT:3>599 <RST_RCVD:3>599 <EOR>\n\
                    <CALL:6>DL1ABC <BAND:3>40m <MODE:3>SSB <QSO_DATE:8>20261012 <TIME_ON:6>121500 <RST_SENT:2>59 <RST_RCVD:2>59 <EOR>";

        // 1. Pierwszy poprawny import
        app.import_adif_content(adif, "test_file.adi");

        assert_eq!(app.recent_qsos.len(), 2);
        assert_eq!(app.qso_numbers.len(), 2);
        assert!(
            app.gui
                .chrome
                .status_message
                .as_ref()
                .unwrap()
                .contains("Zaimportowano 2")
        );

        // Weryfikacja aktualizacji SCP
        {
            let scp_guard = scp.lock().unwrap();
            let matches = scp_guard.search("SP6INA", 5);
            assert!(matches.contains(&"SP6INA".to_string()));
            let matches_dl = scp_guard.search("DL1ABC", 5);
            assert!(matches_dl.contains(&"DL1ABC".to_string()));
        }

        // Weryfikacja aktualizacji AwardsEngine
        {
            let awards_guard = app.services.awards_engine.lock().unwrap();
            assert!(
                awards_guard
                    .worked_calls
                    .iter()
                    .any(|(call, _, _)| call == "SP6INA")
            );
            assert!(
                awards_guard
                    .worked_calls
                    .iter()
                    .any(|(call, _, _)| call == "DL1ABC")
            );
        }

        // 2. Import duplikatów
        app.import_adif_content(adif, "test_file.adi");

        // Liczba rekordów nie wzrosła
        assert_eq!(app.recent_qsos.len(), 2);
        assert_eq!(app.qso_numbers.len(), 2);
        assert!(
            app.gui
                .chrome
                .status_message
                .as_ref()
                .unwrap()
                .contains("Zaimportowano 0")
        );
        assert!(
            app.gui
                .chrome
                .status_message
                .as_ref()
                .unwrap()
                .contains("duplikatów pominiętych")
        );
    }

    #[test]
    fn form_qso_save_success_and_side_effects() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);
        let mut event_rx = app.services.event_bus.subscribe();

        app.entry_callsign = "SP6INA".to_string();
        app.entry_band = "20m".to_string();
        app.entry_mode = "CW".to_string();
        app.entry_rst_sent = "599".to_string();
        app.entry_rst_rcvd = "599".to_string();

        app.save_qso();

        // 1. Formularz wyczyszczony
        assert_eq!(app.entry_callsign, "");

        // 2. Rekord zapisany w bazie z aktywnym journal_id
        assert_eq!(app.recent_qsos.len(), 1);
        assert_eq!(app.recent_qsos[0].callsign, "SP6INA");
        assert_eq!(
            app.recent_qsos[0].journal_id,
            Some(app.active_journal.id.clone())
        );

        // 3. Awards i SCP zaktualizowane
        {
            let awards = app.services.awards_engine.lock().unwrap();
            assert!(awards.worked_calls.iter().any(|(c, _, _)| c == "SP6INA"));
        }
        {
            let scp = app.services.scp_engine.lock().unwrap();
            assert!(scp.search("SP6INA", 5).contains(&"SP6INA".to_string()));
        }

        // 4. AppEvent::QsoLogged opublikowany dokładnie raz
        let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
        assert!(
            matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "SP6INA")
        );
        assert!(
            event_rx.try_recv().is_err(),
            "Zdarzenie powinno być opublikowane dokładnie raz"
        );
    }

    #[test]
    fn form_qso_validation_error_aborts_without_side_effects() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);
        let mut event_rx = app.services.event_bus.subscribe();

        // Błąd walidacji: niedozwolony znak w znaku wywoławczym (QsoIssue::CallsignChars)
        app.entry_callsign = "SP6INA@".to_string();
        app.entry_band = "20m".to_string();
        app.entry_mode = "CW".to_string();

        app.save_qso();

        // Formularz NIE został wyczyszczony
        assert_eq!(app.entry_callsign, "SP6INA@");

        // Brak rekordu w bazie
        assert_eq!(app.recent_qsos.len(), 0);

        // Brak zdarzeń
        assert!(event_rx.try_recv().is_err());

        // Toast z błędem
        assert!(app.gui.chrome.status_toast.is_some());
    }

    #[test]
    fn contest_qso_ingest_success_and_stx_increment() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);
        let mut event_rx = app.services.event_bus.subscribe();
        let initial_stx = app.contest_stx;

        let mut new_qso = QsoRecord::new("SN0HQ", "40m", "CW");
        new_qso.stx = Some(app.contest_stx);
        new_qso.journal_id = Some(app.active_journal.id.clone());

        let outcome = app
            .services
            .ingestor
            .ingest(new_qso, QsoOrigin::Contest)
            .unwrap();
        app.apply_post_effects(&outcome);
        app.contest_stx += 1;
        app.reload_qsos();

        assert_eq!(app.contest_stx, initial_stx + 1);
        assert_eq!(app.recent_qsos.len(), 1);
        assert_eq!(app.recent_qsos[0].callsign, "SN0HQ");

        let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
        assert!(
            matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "SN0HQ")
        );
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn contest_qso_validation_error_does_not_increment_stx() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let app = SpLogApp::for_tests(init, deps);
        let initial_stx = app.contest_stx;

        let mut bad_qso = QsoRecord::new("SN0HQ", "", "CW");
        bad_qso.freq = None;

        let res = app.services.ingestor.ingest(bad_qso, QsoOrigin::Contest);
        assert!(matches!(res, Err(IngestError::Invalid(_))));
        assert_eq!(app.contest_stx, initial_stx);
    }

    #[test]
    fn wsjtx_qso_ingest_advisory_issue_persists_and_registers() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);
        let mut event_rx = app.services.event_bus.subscribe();

        let mut qso = QsoRecord::new("K1ABC", "20m", "FT8");
        qso.cqz = Some(99); // advisory issue
        qso.qso_date.clear(); // fallback w adapterze
        qso.time_on.clear(); // fallback w adapterze

        app.handle_wsjtx_qso(qso);

        assert_eq!(app.recent_qsos.len(), 1);
        let stored = &app.recent_qsos[0];
        assert_eq!(stored.callsign, "K1ABC");
        assert_ne!(stored.qso_date, "");
        assert_ne!(stored.time_on, "");

        {
            let awards = app.services.awards_engine.lock().unwrap();
            assert!(awards.worked_calls.iter().any(|(c, _, _)| c == "K1ABC"));
        }
        {
            let scp = app.services.scp_engine.lock().unwrap();
            assert!(scp.search("K1ABC", 5).contains(&"K1ABC".to_string()));
        }

        let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
        assert!(
            matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "K1ABC")
        );
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn js8call_qso_ingest_handles_batch_and_errors() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);
        let mut event_rx = app.services.event_bus.subscribe();

        let qso1 = QsoRecord::new("JS8ONE", "40m", "JS8");
        let qso2 = QsoRecord::new("", "40m", "JS8"); // pusty znak - błąd
        let qso3 = QsoRecord::new("JS8TWO", "40m", "JS8");

        app.handle_js8call_qso(qso1);
        app.handle_js8call_qso(qso2);
        app.handle_js8call_qso(qso3);

        assert_eq!(app.recent_qsos.len(), 2);
        assert!(app.recent_qsos.iter().any(|q| q.callsign == "JS8ONE"));
        assert!(app.recent_qsos.iter().any(|q| q.callsign == "JS8TWO"));

        let mut events_count = 0;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(event, crate::core::events::AppEvent::QsoLogged { .. }) {
                events_count += 1;
            }
        }
        assert_eq!(events_count, 2);
    }

    #[test]
    fn lan_peer_qso_ingest_no_broadcast_loop() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);
        let mut event_rx = app.services.event_bus.subscribe();

        assert!(
            !QsoOrigin::LanPeer.policy().effects.broadcast_lan,
            "LanPeer nie może broadcastować do LAN"
        );

        {
            let db = app.services.log_db.lock().unwrap();
            let j = Journal {
                id: "PEER_JOURNAL".to_string(),
                name: "Peer Journal".to_string(),
                ..Default::default()
            };
            db.create_journal(&j).unwrap();
        }

        let mut qso = QsoRecord::new("LANSTN", "80m", "SSB");
        qso.journal_id = Some("PEER_JOURNAL".to_string());

        app.handle_lan_qso(qso);

        let db_records = app
            .services
            .log_db
            .lock()
            .unwrap()
            .get_recent_qsos_for_journal("PEER_JOURNAL", 10)
            .unwrap();
        assert_eq!(db_records.len(), 1);
        assert_eq!(db_records[0].callsign, "LANSTN");
        assert_eq!(db_records[0].journal_id.as_deref(), Some("PEER_JOURNAL"));
        assert!(app.sync_ui.log.iter().any(|line| line.contains("LANSTN")));

        let event = event_rx.try_recv().expect("Oczekiwano zdarzenia AppEvent");
        assert!(
            matches!(event, crate::core::events::AppEvent::QsoLogged { ref callsign, .. } if callsign == "LANSTN")
        );
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn post_effects_execution_matches_flags() {
        let deps = test_deps();
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("config.json"),
            active_db_path: std::path::PathBuf::from("db.sqlite"),
            shutdown_policy: ShutdownPolicy::None,
        };
        let mut app = SpLogApp::for_tests(init, deps);

        let qso = QsoRecord::new("SP1TEST", "20m", "CW");
        let mut effects = crate::core::ingest::PostEffects::default();

        app.solar.propagation_history.record_forecast(
            "20m",
            12.0,
            120,
            2,
            85,
            crate::core::propagation::BandOpeningStatus::Open,
        );
        assert_eq!(app.solar.propagation_history.records.len(), 1);
        assert!(app.solar.propagation_history.records[0].observed.is_none());

        let outcome = IngestOutcome {
            origin: QsoOrigin::Form,
            qso: qso.clone(),
            warnings: Vec::new(),
            advisory_issue: None,
            effects,
        };

        // Bez flag - obserwacja nie jest zapisana
        app.apply_post_effects(&outcome);
        assert!(app.solar.propagation_history.records[0].observed.is_none());

        // Włączona flaga propagation_observation
        effects.propagation_observation = true;
        let outcome_with_prop = IngestOutcome {
            origin: QsoOrigin::Form,
            qso,
            warnings: Vec::new(),
            advisory_issue: None,
            effects,
        };
        app.apply_post_effects(&outcome_with_prop);
        assert_eq!(
            app.solar.propagation_history.records[0].observed,
            Some(crate::core::propagation_history::ObservedOutcome::ConfirmedOpen)
        );
    }
}
