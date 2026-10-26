// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cat::hamlib::RigState;
use crate::cat::rotor::RotorState;
use crate::core::awards::AwardsEngine;
use crate::core::backup::BackupManager;
use crate::core::database::Journal;
use crate::core::i18n::{Msg, key, tr};
use crate::core::ingest::{IngestError, IngestOutcome, QsoOrigin};
use crate::core::qso::QsoRecord;
use crate::core::service_db::ServiceDatabase;
use crate::core::station::{AppConfig, ViewPanelConfig};
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

pub use crate::gui::state::{
    CatConnection, CatState, ClusterState, ContestState, SpotBadgeKind, VfoState,
};

pub struct SpLogApp {
    pub config: AppConfig,
    pub paths: crate::gui::state::AppPaths,
    pub services: crate::gui::state::AppServices,
    pub runtime: crate::gui::state::AppRuntime,
    pub digital: crate::gui::state::DigitalModesState,
    pub entry: crate::gui::state::QsoEntryState,
    pub log_view: crate::gui::state::LogViewState,

    // Stan transceivera CAT, S-Metra, rotora i VFO
    pub cat: crate::gui::state::CatState,

    // DX Cluster Telnet i filtry
    pub cluster: crate::gui::state::ClusterState,
    // WSPR Monitor
    pub wspr: crate::gui::state::WsprState,

    // Moduł Satelitów
    pub satellites: crate::gui::state::SatelliteState,

    // Moduł Pogody Kosmicznej i Propagacji Słonecznej
    pub solar: crate::gui::state::SolarState,

    // Moduł Zawodów
    pub contest: crate::gui::state::ContestState,

    // Makra Telegraficzne CW
    pub cw: crate::gui::state::CwMacroState,

    // Księga sprzętowa i CRUD
    pub ledger: crate::gui::state::EquipmentLedgerState,

    // Okna dialogowe / aktualizacje
    pub updates: crate::gui::state::UpdateState,

    // Voice keyer (SSB)
    pub voice_keyer: crate::gui::state::VoiceKeyerState,

    // Multi-Op LAN (SYNC-6)
    pub sync_ui: crate::gui::SyncUiState,

    pub online: crate::gui::state::OnlineSyncUiState,

    // Okna modułów
    pub world_map: crate::gui::state::WorldMapState,

    // Stan GUI (dialogi, chrome, widoki, okna)
    pub gui: crate::gui::state::GuiState,
    pub active_journal: crate::core::database::Journal,

    // Modułowy układ kafelków i okien pływających
    pub views: crate::gui::state::ViewsState,

    // System pluginów użytkownika (Rhai)
    pub plugins_ui: crate::gui::state::PluginUiState,

    // Marketplace pluginów (katalog + instalacja jednym kliknięciem)
    pub marketplace: crate::gui::state::MarketplaceState,

    // Wyszukiwanie i usuwanie duplikatów w logu
    pub duplicates: crate::gui::state::DuplicatesState,
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

fn resolve_callbook_paths() -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
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
    (cb_path, srv_path)
}

fn fetch_initial_journal_data(
    log_db: &std::sync::Mutex<crate::core::database::LogDatabase>,
) -> (
    Journal,
    Vec<QsoRecord>,
    std::collections::HashMap<i64, usize>,
) {
    let db = log_db
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
        let (active_journal, recent_qsos, qso_numbers) = fetch_initial_journal_data(&deps.log_db);

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

        let (cb_path, srv_path) = resolve_callbook_paths();

        // Cache offline wyników callbook — obok bazy dziennika (katalog zapisywalny).
        let callbook_cache_path = if init.config.callbook_cache_enabled {
            init.active_db_path
                .parent()
                .map(|p| p.join("callbook_cache.db"))
        } else {
            None
        };
        let callbook_cache_ttl_days = init.config.callbook_cache_ttl_days;

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
        let mut gui = crate::gui::state::GuiState {
            chrome,
            appearance,
            dialogs: crate::gui::state::DialogState::default(),
            workspace: crate::gui::state::DockWorkspace::default(),
            windows: crate::gui::state::WindowState::default(),
        };
        gui.windows.show_operator_assistant = init.config.operator_assistant_enabled;

        let views = crate::gui::state::ViewsState::from_config(&init.config);
        let online = crate::gui::state::OnlineSyncUiState::new();
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
        if init.config.station_profiles.is_empty() {
            let mut def = init.config.station.clone();
            if def.id.is_empty() {
                def.id = "default".to_string();
            }
            if def.name.is_empty() {
                def.name = "Main (Home QTH)".to_string();
            }
            init.config.station_profiles = vec![def];
        }
        let config = init.config.clone();
        let mut app = Self {
            config,
            paths,
            services,
            runtime,
            digital,
            entry: crate::gui::state::QsoEntryState::new(&init.config),
            log_view: crate::gui::state::LogViewState {
                recent_qsos,
                qso_numbers,
                ..Default::default()
            },

            cat: crate::gui::state::CatState {
                rig_state: rig,
                rotor_state: rotor,
                conn_type: if init.config.cat_auto_start_rigctld
                    || !init.config.cat_serial_port.is_empty()
                {
                    crate::gui::state::CatConnection::Serial
                } else {
                    crate::gui::state::CatConnection::Tcp
                },
                mfg_selected: "Wszystkie".to_string(),
                vfo: crate::gui::state::VfoState {
                    split_offset_khz: 1.0,
                    filter_preset: "FIL2".to_string(),
                    preamp_att: "OFF".to_string(),
                    nb_nr: false,
                    agc_speed: "MID".to_string(),
                },
                ..Default::default()
            },

            cluster: crate::gui::state::ClusterState {
                spots: sample_spots,
                status_text: tr(key::CLUSTER_DISCONNECTED, lang).to_string(),
                ..Default::default()
            },
            wspr: crate::gui::state::WsprState::default(),

            satellites: crate::gui::state::SatelliteState::default(),

            solar: crate::gui::state::SolarState::with_propagation_history(
                init.config.propagation_history,
            ),

            contest: crate::gui::state::ContestState::default(),

            cw: crate::gui::state::CwMacroState::default(),

            ledger: crate::gui::state::EquipmentLedgerState::default(),

            updates: crate::gui::state::UpdateState::default(),

            voice_keyer: crate::gui::state::VoiceKeyerState::default(),

            sync_ui: crate::gui::SyncUiState::new(),

            online,

            world_map: crate::gui::state::WorldMapState::default(),

            gui,
            active_journal,

            views,

            // System pluginów użytkownika (Rhai)
            plugins_ui: crate::gui::state::PluginUiState::default(),

            marketplace: crate::gui::state::MarketplaceState::default(),

            // Wyszukiwanie duplikatów
            duplicates: crate::gui::state::DuplicatesState::default(),
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
        if !self.config.font_family.is_empty() {
            if let Some(path) = resolve_font_path(&self.config.font_family) {
                if let Ok(bytes) = std::fs::read(&path) {
                    font_defs.font_data.insert(
                        "custom_ui_font".to_owned(),
                        egui::FontData::from_owned(bytes).into(),
                    );
                    if let Some(prop) = font_defs.families.get_mut(&egui::FontFamily::Proportional)
                    {
                        prop.insert(0, "custom_ui_font".to_owned());
                    }
                    let mono_family = self.config.font_family == "Consolas";
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
            let dir = std::path::PathBuf::from(&self.config.plugins_dir);
            self.services
                .plugin_engine
                .set_enabled(self.config.plugins_enabled);
            self.services.plugin_engine.load_dir(&dir);
            let total = self.log_view.qso_numbers.len() as i64;
            self.services.plugin_engine.set_qso_count(total);
            self.services.plugin_engine.run_startup();
        }

        if self.runtime.initial_cat_enabled {
            self.start_cat_service();
        }

        if self.config.cat_sharing_enabled {
            self.toggle_cat_proxy_server();
        }

        if self.runtime.rest_api_enabled {
            self.start_rest_api_server();
        }

        if self.config.cluster_auto_connect {
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
        self.track_window_geometry(ctx);

        // Wczesna faza asynchroniczna: wtyczki, WSPR, API, upload scheduler, satelity i solar
        self.poll_early_async_events();

        // Faza obsługi skrótów klawiaturowych, palety poleceń i wejść globalnych
        self.handle_input_and_shortcuts(ctx);

        // Debounced lookup Callbook/QRZ po wpisaniu znaku
        self.process_debounced_lookup();

        // Główna faza asynchroniczna: aktualizacje, CAT, REST API, proxy, klaster, callbook, sync, WSJT-X, JS8Call, Multi-Op
        self.poll_main_async_events(ctx);

        // Zastosowanie wybranego motywu, stylów i skali UI (apply-on-change przez ensure_applied).
        crate::gui::theme::ensure_applied(
            ctx,
            self.gui.appearance.theme_preset,
            self.config.font_scale,
        );

        // Faza Chrome: paski menu i statusu stacji
        self.render_chrome_bars(ui);

        // Główny obszar roboczy - swobodny pulpit stacji SPLogbook (MDI Desktop)
        self.render_central_workspace(ui);

        // W zależności od trybu: Mini HUD (tryb kompaktowy) lub pełny pulpit roboczy
        if self.config.compact_hud_mode {
            crate::gui::mini_hud::render_mini_hud_bar(self, ui);
        } else {
            self.render_floating_module_windows(ctx);
        }

        // Pozostałe okna modułów zaawansowanych
        self.render_advanced_module_windows(ctx);

        // Okna dialogowe i narzędzia pomocnicze
        self.render_dialogs(ctx);

        // Faza Chrome: toasty, okno o programie, paleta, manual, skróty, legenda
        self.render_chrome_overlays(ctx);

        // Resetowanie flagi po narysowaniu i spzycjonowaniu okien w danej klatce
        self.gui.workspace.reset_layout_requested = false;

        // Płynne odświeżanie zegara UTC, stanu radia CAT oraz spotów sieciowych bez obciążania procesora (4 Hz)
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cat
            .runtime
            .shutdown_blocking(std::time::Duration::from_millis(1500));
        if let Some(mut sup) = self.cat.rigctld_supervisor.take() {
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
                crate::core::adif::export_adif(&qsos, "SPLogbook", &self.config.station.callsign);
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
#[path = "app_tests.rs"]
mod tests;
