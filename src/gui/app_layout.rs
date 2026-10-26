// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

// Układ dokowania i kafelki.
use super::*;

impl SpLogApp {
    /// Kolejność paneli (identyfikatorów kafelków) używana przy budowaniu
    /// układu dokowania oraz przy synchronizacji widoczności.
    pub fn all_tile_ids() -> [&'static str; 9] {
        crate::gui::state::DockWorkspace::all_tile_ids()
    }

    pub fn panel_config(&self, id: &str) -> &ViewPanelConfig {
        crate::gui::state::DockWorkspace::panel_config(&self.views, id)
    }

    /// Buduje początkowy `DockState` z konfiguracji. Preferuje zapisany układ
    /// (`dock_layout`); w przeciwnym razie odtwarza układ kolumnowy
    /// na podstawie pól `column`/`order` poszczególnych paneli.
    pub fn build_dock_state(&mut self, dock_layout: Option<&serde_json::Value>) {
        self.gui.workspace.build(dock_layout, &self.views);
    }

    /// Odtwarza dokowany układ z pól `column`/`order` paneli (3 kolumny
    /// ułożone poziomo). Używane przy pierwszym uruchomieniu i po resecie.
    pub fn rebuild_dock_state_from_panels(&mut self) {
        self.gui.workspace.rebuild_from_views(&self.views);
    }

    /// Uzgadnia zawartość `dock_state` ze stanem konfiguracji paneli:
    /// usuwa karty paneli ukrytych/odpiętych i dodaje brakujące karty paneli
    /// widocznych i zadokowanych.
    pub fn sync_dock_state(&mut self) {
        self.gui.workspace.sync_with_views(&self.views);
    }

    /// Serializuje bieżący układ dokowania do surowego JSON (do zapisu w konfiguracji).
    pub fn serialize_dock_layout(&self) -> Option<serde_json::Value> {
        self.gui.workspace.serialize_layout()
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
        mut body: impl FnMut(&mut Self, &mut egui::Ui) -> T,
    ) -> (T, Option<([f32; 2], [f32; 2])>) {
        let mut captured_geo: Option<([f32; 2], [f32; 2])> = None;

        let init_cache_id = egui::Id::new(("floating_vp_init_geo", viewport_id));
        let (init_pos, init_size) = ctx.data_mut(|d| {
            *d.get_temp_mut_or_insert_with(init_cache_id, || {
                let sanitized_size = saved_size
                    .map(|[w, h]| [w.clamp(min_size[0], 1600.0), h.clamp(min_size[1], 1000.0)])
                    .unwrap_or(default_size);
                let sanitized_pos =
                    saved_pos.map(|[x, y]| [x.clamp(-1920.0, 3840.0), y.clamp(0.0, 2000.0)]);
                (sanitized_pos, sanitized_size)
            })
        });

        let mut builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(init_size)
            .with_min_inner_size(min_size);
        if let Some([x, y]) = init_pos {
            builder = builder.with_position(egui::pos2(x, y));
        }

        let result = ctx.show_viewport_immediate(viewport_id, builder, |vp_ui, _class| {
            let out = body(self, vp_ui);
            captured_geo = vp_ui.ctx().input(|i| {
                let vp = i.viewport();
                let pos = vp
                    .outer_rect
                    .or(vp.inner_rect)
                    .map(|r| [r.min.x.clamp(-1920.0, 3840.0), r.min.y.clamp(0.0, 2000.0)]);
                let size = vp.inner_rect.map(|r| {
                    [
                        r.width().clamp(min_size[0], 1600.0),
                        r.height().clamp(min_size[1], 1000.0),
                    ]
                });
                match (pos, size) {
                    (Some(p), Some(s)) => {
                        let pos_changed = saved_pos
                            .map(|sp| (sp[0] - p[0]).abs() > 2.0 || (sp[1] - p[1]).abs() > 2.0)
                            .unwrap_or(true);
                        let size_changed = saved_size
                            .map(|ss| (ss[0] - s[0]).abs() > 2.0 || (ss[1] - s[1]).abs() > 2.0)
                            .unwrap_or(true);
                        if (pos_changed || size_changed) && !i.pointer.any_down() {
                            Some((p, s))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            });
            out
        });
        (result, captured_geo)
    }

    pub fn reset_panel_layout(&mut self) {
        use crate::core::station::ViewPanelConfig;
        self.config.left_column_width = 350.0;
        self.config.right_column_width = 360.0;

        self.views.panel_vfo = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 0,
            order: 0,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_qso = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 0,
            order: 1,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_log = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 1,
            order: 0,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_cluster = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 1,
            order: 1,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_bandmap = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 0,
            order: 2,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_solar = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 2,
            order: 1,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_satellites = ViewPanelConfig {
            visible: false,
            floating: false,
            column: 2,
            order: 2,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_world_map = ViewPanelConfig {
            visible: false,
            floating: false,
            column: 2,
            order: 0,
            saved_pos: None,
            saved_size: None,
        };
        self.views.panel_waterfall = ViewPanelConfig {
            visible: false,
            floating: false,
            column: 2,
            order: 3,
            saved_pos: None,
            saved_size: None,
        };

        self.gui.windows.show_bandmap_window = false;
        self.satellites.show_window = false;
        self.world_map.show_window = false;
        self.gui.workspace.reset_layout_requested = true;
        // Natychmiast odtwórz dokowany układ z domyślnej konfiguracji paneli.
        self.rebuild_dock_state_from_panels();
        self.gui.workspace.last_saved_dock_layout = self.serialize_dock_layout();
    }

    pub fn popout_tile(&mut self, tile_id: &str) {
        match tile_id {
            "vfo" => self.views.panel_vfo.floating = true,
            "qso" => self.views.panel_qso.floating = true,
            "log" => self.views.panel_log.floating = true,
            "cluster" => self.views.panel_cluster.floating = true,
            "bandmap" => {
                self.views.panel_bandmap.floating = true;
                self.gui.windows.show_bandmap_window = true;
            }
            "solar" => self.views.panel_solar.floating = true,
            "satellites" => {
                self.views.panel_satellites.floating = true;
                self.satellites.show_window = true;
            }
            "world_map" => {
                self.views.panel_world_map.floating = true;
                self.world_map.show_window = true;
            }
            "waterfall" => {
                self.views.panel_waterfall.floating = true;
            }
            _ => {}
        }
    }

    pub fn close_tile(&mut self, tile_id: &str) {
        match tile_id {
            "vfo" => self.views.panel_vfo.visible = false,
            "qso" => self.views.panel_qso.visible = false,
            "log" => self.views.panel_log.visible = false,
            "cluster" => self.views.panel_cluster.visible = false,
            "bandmap" => {
                self.views.panel_bandmap.visible = false;
                self.gui.windows.show_bandmap_window = false;
            }
            "solar" => self.views.panel_solar.visible = false,
            "satellites" => {
                self.views.panel_satellites.visible = false;
                self.satellites.show_window = false;
            }
            "world_map" => {
                self.views.panel_world_map.visible = false;
                self.world_map.show_window = false;
            }
            "waterfall" => {
                self.views.panel_waterfall.visible = false;
            }
            _ => {}
        }
    }

    pub fn tile_title(&self, tile_id: &str) -> String {
        let lang = self.gui.appearance.language;
        match tile_id {
            "vfo" => icons::RADIO.label("TRANSCEIVER VFO"),
            "qso" => icons::NEW_QSO.label(tr(key::TAB_NEW_QSO, lang)),
            "log" => icons::LOGBOOK.label(tr(key::TAB_LOGBOOK, lang)),
            "cluster" => icons::CLUSTER.label(tr(key::CLUSTER_TITLE, lang)),
            "bandmap" => icons::BANDMAP.label(tr(key::BANDMAP_TITLE, lang)),
            "solar" => icons::SOLAR.label(tr(key::SOLAR_TITLE, lang)),
            "satellites" => icons::SATELLITE.label(tr(key::SAT_TITLE, lang)),
            "world_map" => icons::WORLD_MAP.label(tr(key::MAP_WORLD_TITLE, lang)),
            "waterfall" => icons::SIGNAL_UP.label(tr(key::VIEW_PANEL_WATERFALL, lang)),
            _ => tile_id.to_string(),
        }
    }

    pub fn render_tile_header_custom(&mut self, tile_id: &str, ui: &mut egui::Ui) {
        use crate::gui::theme::ThemeExt;
        let palette = ui.palette();
        let lang = self.gui.appearance.language;
        match tile_id {
            "vfo" => {
                if self.cat.vfo_split {
                    ui.label(
                        egui::RichText::new("SPLIT ON")
                            .color(palette.danger)
                            .strong()
                            .size(11.0),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("SPLIT OFF")
                            .color(palette.text_subtle)
                            .size(11.0),
                    );
                }
            }
            "qso" => {
                if self.cat.connected {
                    if ui
                        .button(
                            egui::RichText::new("● CAT ONLINE")
                                .color(palette.success)
                                .size(11.0)
                                .strong(),
                        )
                        .clicked()
                    {
                        self.gui.windows.show_cat_settings_window = true;
                    }
                } else if ui
                    .button(
                        egui::RichText::new("○ CAT OFFLINE")
                            .color(palette.text_muted)
                            .size(11.0),
                    )
                    .clicked()
                {
                    self.gui.windows.show_cat_settings_window = true;
                }
            }
            "log" => {
                ui.label(
                    egui::RichText::new(format!("({} QSO)", self.log_view.recent_qsos.len()))
                        .size(11.0)
                        .color(palette.text_muted),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.log_view.search_query)
                        .hint_text(tr(key::QSO_SEARCH, lang))
                        .desired_width(110.0),
                );
                if ui
                    .button("🔄")
                    .on_hover_text(tr(key::BTN_REFRESH, lang))
                    .clicked()
                {
                    self.reload_qsos();
                }
            }
            "cluster" => {
                if self.cluster.connected {
                    ui.label(
                        egui::RichText::new("● ONLINE")
                            .color(palette.success)
                            .size(10.0)
                            .strong(),
                    );
                } else if self.cluster.connecting {
                    ui.label(
                        egui::RichText::new(tr(key::CLUSTER_STATUS_CONNECTING, lang))
                            .color(palette.warning)
                            .size(10.0)
                            .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("○ OFFLINE")
                            .color(palette.text_muted)
                            .size(10.0),
                    );
                }
                if ui
                    .button(
                        egui::RichText::new("📢 Spot")
                            .strong()
                            .color(palette.accent),
                    )
                    .on_hover_text(tr(key::CLUSTER_SEND_SPOT_TOOLTIP, lang))
                    .clicked()
                {
                    let freq_khz = self.cat.rig_state.frequency_hz as f64 / 1000.0;
                    self.gui
                        .dialogs
                        .send_spot_dialog
                        .open_with(&self.entry.callsign, freq_khz);
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
            "waterfall" => crate::gui::waterfall_panel::render_waterfall_body(self, ui),
            _ => {}
        }
    }

    /// Śledzenie geometrii okna głównego (zapisywane przy wyjściu — obsługa multi-monitor)
    pub fn track_window_geometry(&mut self, ctx: &egui::Context) {
        let (win_pos, win_size, win_max) = ctx.input(|i| {
            let vp = i.viewport();
            let pos = vp.outer_rect.map(|r| [r.min.x, r.min.y]);
            let size = vp.outer_rect.map(|r| [r.width(), r.height()]);
            (pos, size, vp.maximized.unwrap_or(false))
        });
        if let Some(pos) = win_pos {
            self.config.main_window_pos = Some(pos);
        }
        if let Some(size) = win_size {
            self.config.main_window_size = Some(size);
        }
        self.config.main_window_maximized = win_max;
    }

    /// Główny obszar roboczy — swobodny pulpit stacji SPLogbook (MDI Desktop)
    pub fn render_central_workspace(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            let lang = self.gui.appearance.language;
            let palette = ui.palette();
            if self.config.compact_hud_mode {
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
                        self.config.compact_hud_mode = false;
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
    }

    /// Pływające okna modułów (pop-out windows) — renderowane tylko wtedy,
    /// gdy panel jest „odpięty” (floating).
    pub fn render_floating_module_windows(&mut self, ctx: &egui::Context) {
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

    /// Pozostałe okna modułów zaawansowanych
    pub fn render_advanced_module_windows(&mut self, ctx: &egui::Context) {
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
    }

    /// Okna dialogowe i narzędzia pomocnicze
    pub fn render_dialogs(&mut self, ctx: &egui::Context) {
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
                self.config.station.callsign.clone_from(&j.station_callsign);
                self.config.station.operator.clone_from(&j.operator);
                self.config.station.gridsquare.clone_from(&j.my_gridsquare);
                self.config.station.pga_gmina = if j.my_pga.is_empty() {
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
            let my_call = self.config.station.callsign.clone();
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
                self.log_view.recent_qsos = filtered;
            }
            if clear_filter {
                self.reload_qsos();
            }
        }

        if self.gui.dialogs.cw_terminal_dialog.is_open {
            let my_call = self.config.station.callsign.clone();
            self.gui
                .dialogs
                .cw_terminal_dialog
                .render(ctx, &my_call, self.gui.appearance.language);
        }

        if self.gui.dialogs.qsl_designer_dialog.is_open {
            let my_call = self.config.station.callsign.clone();
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
        let my_grid = self.config.station.gridsquare.clone();
        self.gui.dialogs.astronomy_dialog.show(
            ctx,
            &my_grid,
            &mut self.cat.rotor_state,
            self.gui.appearance.language,
        );

        // 3. Moduł SOTA / POTA
        self.gui.dialogs.sota_dialog.show(
            ctx,
            &self.log_view.recent_qsos,
            self.gui.appearance.language,
        );

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
                self.entry.iota = iota;
            }

            // 5. Przeglądarka stanów USA (WAS)
            if let Some(state) = self.gui.dialogs.states_dialog.show(
                ctx,
                &self.services.service_db,
                &awards,
                self.gui.appearance.language,
            ) {
                self.entry.state = state;
            }
        }

        // 6. Baza menedżerów QSL
        if let Some(qslm) = self.gui.dialogs.qsl_manager_dialog.show(
            ctx,
            &self.services.service_db,
            self.gui.appearance.language,
        ) {
            self.entry.qsl_manager = qslm;
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
            &self.config.station.callsign,
            self.gui.appearance.language,
        ) {
            if let Some(ref tx) = self.cluster.cmd_tx {
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
                spotter: self.config.station.callsign.clone(),
                comment: sub.comment.clone(),
                time_utc: chrono::Utc::now().format("%H%M").to_string(),
                band: band_str,
                is_ft8,
                is_skimmer: false,
                received_at: chrono::Utc::now().timestamp(),
            };
            self.cluster.spots.insert(0, spot);
            let toast_msg = Msg::new(key::CLUSTER_SPOT_SENT)
                .arg("call", &sub.dx_call)
                .arg("freq", format!("{:.1}", sub.freq_khz))
                .text(self.gui.appearance.language);
            self.gui.chrome.status_toast = Some((toast_msg, std::time::Instant::now()));
        }
    }
}
