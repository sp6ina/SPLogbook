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

        self.show_bandmap_window = false;
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
                self.show_bandmap_window = true;
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
                self.show_bandmap_window = false;
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
                if self.vfo_split {
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
                if self.cat_connected {
                    if ui
                        .button(
                            egui::RichText::new("● CAT ONLINE")
                                .color(palette.success)
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
                            .color(palette.text_muted)
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
                        .color(palette.text_muted),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.log_search_query)
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
                if self.cluster_connected {
                    ui.label(
                        egui::RichText::new("● ONLINE")
                            .color(palette.success)
                            .size(10.0)
                            .strong(),
                    );
                } else if self.cluster_connecting {
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
                    let freq_khz = self.rig_state.frequency_hz as f64 / 1000.0;
                    self.gui
                        .dialogs
                        .send_spot_dialog
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
