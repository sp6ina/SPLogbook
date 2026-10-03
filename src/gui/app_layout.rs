// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

// Układ dokowania i kafelki.
use super::*;

impl SpLogApp {
    /// Kolejność paneli (identyfikatorów kafelków) używana przy budowaniu
    /// układu dokowania oraz przy synchronizacji widoczności.
    pub fn all_tile_ids() -> [&'static str; 9] {
        [
            "vfo",
            "qso",
            "log",
            "cluster",
            "bandmap",
            "solar",
            "satellites",
            "world_map",
            "waterfall",
        ]
    }

    pub fn panel_config(&self, id: &str) -> &ViewPanelConfig {
        match id {
            "qso" => &self.panel_qso,
            "log" => &self.panel_log,
            "cluster" => &self.panel_cluster,
            "bandmap" => &self.panel_bandmap,
            "solar" => &self.panel_solar,
            "satellites" => &self.panel_satellites,
            "world_map" => &self.panel_world_map,
            "waterfall" => &self.panel_waterfall,
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
        for col in &mut columns {
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
        for col_items in columns.iter().skip(first + 1) {
            if col_items.is_empty() {
                continue;
            }
            let [_, new_node] =
                self.dock_state
                    .main_surface_mut()
                    .split_right(rightmost, 0.5, col_items.clone());
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
        self.left_column_width = 350.0;
        self.right_column_width = 360.0;

        self.panel_vfo = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 0,
            order: 0,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_qso = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 0,
            order: 1,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_log = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 1,
            order: 0,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_cluster = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 1,
            order: 1,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_bandmap = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 0,
            order: 2,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_solar = ViewPanelConfig {
            visible: true,
            floating: false,
            column: 2,
            order: 1,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_satellites = ViewPanelConfig {
            visible: false,
            floating: false,
            column: 2,
            order: 2,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_world_map = ViewPanelConfig {
            visible: false,
            floating: false,
            column: 2,
            order: 0,
            saved_pos: None,
            saved_size: None,
        };
        self.panel_waterfall = ViewPanelConfig {
            visible: false,
            floating: false,
            column: 2,
            order: 3,
            saved_pos: None,
            saved_size: None,
        };

        self.show_bandmap_window = false;
        self.show_satellites_window = false;
        self.show_world_map_window = false;
        self.reset_layout_requested = true;
        // Natychmiast odtwórz dokowany układ z domyślnej konfiguracji paneli.
        self.rebuild_dock_state_from_panels();
        self.last_saved_dock_layout = self.serialize_dock_layout();
    }

    pub fn popout_tile(&mut self, tile_id: &str) {
        match tile_id {
            "vfo" => self.panel_vfo.floating = true,
            "qso" => self.panel_qso.floating = true,
            "log" => self.panel_log.floating = true,
            "cluster" => self.panel_cluster.floating = true,
            "bandmap" => {
                self.panel_bandmap.floating = true;
                self.show_bandmap_window = true;
            }
            "solar" => self.panel_solar.floating = true,
            "satellites" => {
                self.panel_satellites.floating = true;
                self.show_satellites_window = true;
            }
            "world_map" => {
                self.panel_world_map.floating = true;
                self.show_world_map_window = true;
            }
            "waterfall" => {
                self.panel_waterfall.floating = true;
            }
            _ => {}
        }
    }

    pub fn close_tile(&mut self, tile_id: &str) {
        match tile_id {
            "vfo" => self.panel_vfo.visible = false,
            "qso" => self.panel_qso.visible = false,
            "log" => self.panel_log.visible = false,
            "cluster" => self.panel_cluster.visible = false,
            "bandmap" => {
                self.panel_bandmap.visible = false;
                self.show_bandmap_window = false;
            }
            "solar" => self.panel_solar.visible = false,
            "satellites" => {
                self.panel_satellites.visible = false;
                self.show_satellites_window = false;
            }
            "world_map" => {
                self.panel_world_map.visible = false;
                self.show_world_map_window = false;
            }
            "waterfall" => {
                self.panel_waterfall.visible = false;
            }
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
            "satellites" => icons::SATELLITE.label(tr("sat.title", lang)),
            "world_map" => icons::WORLD_MAP.label(tr("map.world_title", lang)),
            "waterfall" => icons::SIGNAL_UP.label(crate::core::i18n::tr_or(
                lang,
                "WIDMO / WATERFALL (SDR)",
                "SPECTRUM / WATERFALL (SDR)",
            )),
            _ => tile_id.to_string(),
        }
    }

    pub fn render_tile_header_custom(&mut self, tile_id: &str, ui: &mut egui::Ui) {
        let lang = self.current_language;
        match tile_id {
            "vfo" => {
                if self.vfo_split {
                    ui.label(
                        egui::RichText::new("SPLIT ON")
                            .color(egui::Color32::from_rgb(239, 68, 68))
                            .strong()
                            .size(11.0),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("SPLIT OFF")
                            .color(egui::Color32::from_rgb(100, 116, 139))
                            .size(11.0),
                    );
                }
            }
            "qso" => {
                if self.cat_connected {
                    if ui
                        .button(
                            egui::RichText::new("● CAT ONLINE")
                                .color(egui::Color32::from_rgb(34, 197, 94))
                                .size(11.0)
                                .strong(),
                        )
                        .clicked()
                    {
                        self.show_cat_settings_window = true;
                    }
                } else if ui
                    .button(
                        egui::RichText::new("○ CAT OFFLINE")
                            .color(egui::Color32::from_rgb(148, 163, 184))
                            .size(11.0),
                    )
                    .clicked()
                {
                    self.show_cat_settings_window = true;
                }
            }
            "log" => {
                ui.label(
                    egui::RichText::new(format!("({} QSO)", self.recent_qsos.len()))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.log_search_query)
                        .hint_text(tr("qso.search", lang))
                        .desired_width(110.0),
                );
                if ui
                    .button("🔄")
                    .on_hover_text(tr("btn.refresh", lang))
                    .clicked()
                {
                    self.reload_qsos();
                }
            }
            "cluster" => {
                if self.cluster_connected {
                    ui.label(
                        egui::RichText::new("● ONLINE")
                            .color(egui::Color32::from_rgb(34, 197, 94))
                            .size(10.0)
                            .strong(),
                    );
                } else if self.cluster_connecting {
                    ui.label(
                        egui::RichText::new(crate::core::i18n::tr_or(
                            lang,
                            "● ŁĄCZENIE...",
                            "● CONNECTING...",
                        ))
                        .color(egui::Color32::from_rgb(250, 204, 21))
                        .size(10.0)
                        .strong(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("○ OFFLINE")
                            .color(egui::Color32::from_rgb(148, 163, 184))
                            .size(10.0),
                    );
                }
                if ui
                    .button(
                        egui::RichText::new("📢 Spot")
                            .strong()
                            .color(egui::Color32::from_rgb(56, 189, 248)),
                    )
                    .on_hover_text(crate::core::i18n::tr_or(
                        lang,
                        "Wyślij spot DX do klastra Telnet",
                        "Send DX spot to Telnet cluster",
                    ))
                    .clicked()
                {
                    let freq_khz = self.rig_state.frequency_hz as f64 / 1000.0;
                    self.send_spot_dialog
                        .open_with(&self.entry_callsign, freq_khz);
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
}
