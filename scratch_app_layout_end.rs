    pub fn get_tiles_in_column(&self, col: usize) -> Vec<String> {
        let mut items = Vec::new();
        if self.panel_vfo.column == col && self.panel_vfo.visible && !self.panel_vfo.floating {
            items.push(("vfo".to_string(), self.panel_vfo.order));
        }
        if self.panel_qso.column == col && self.panel_qso.visible && !self.panel_qso.floating {
            items.push(("qso".to_string(), self.panel_qso.order));
        }
        if self.panel_log.column == col && self.panel_log.visible && !self.panel_log.floating {
            items.push(("log".to_string(), self.panel_log.order));
        }
        if self.panel_cluster.column == col
            && self.panel_cluster.visible
            && !self.panel_cluster.floating
        {
            items.push(("cluster".to_string(), self.panel_cluster.order));
        }
        if self.panel_bandmap.column == col
            && self.panel_bandmap.visible
            && !self.panel_bandmap.floating
        {
            items.push(("bandmap".to_string(), self.panel_bandmap.order));
        }
        if self.panel_solar.column == col && self.panel_solar.visible && !self.panel_solar.floating
        {
            items.push(("solar".to_string(), self.panel_solar.order));
        }
        if self.panel_satellites.column == col
            && self.panel_satellites.visible
            && !self.panel_satellites.floating
        {
            items.push(("satellites".to_string(), self.panel_satellites.order));
        }
        if self.panel_world_map.column == col
            && self.panel_world_map.visible
            && !self.panel_world_map.floating
        {
            items.push(("world_map".to_string(), self.panel_world_map.order));
        }
        if self.panel_waterfall.column == col
            && self.panel_waterfall.visible
            && !self.panel_waterfall.floating
        {
            items.push(("waterfall".to_string(), self.panel_waterfall.order));
        }
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
            "waterfall" => self.panel_waterfall.order = order,
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
            "waterfall" => self.panel_waterfall.column,
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
            "waterfall" => self.panel_waterfall.column,
            _ => return,
        };
        let mut tiles = self.get_tiles_in_column(col);
        if let Some(pos) = tiles.iter().position(|id| id == tile_id) {
            let new_pos =
                (pos as i32 + delta).clamp(0, (tiles.len().saturating_sub(1)) as i32) as usize;
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
            "waterfall" => self.panel_waterfall.column = col,
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
            if self.panel_vfo.column == col && self.panel_vfo.visible && !self.panel_vfo.floating {
                items.push(("vfo", self.panel_vfo.order));
            }
            if self.panel_qso.column == col && self.panel_qso.visible && !self.panel_qso.floating {
                items.push(("qso", self.panel_qso.order));
            }
            if self.panel_log.column == col && self.panel_log.visible && !self.panel_log.floating {
                items.push(("log", self.panel_log.order));
            }
            if self.panel_cluster.column == col
                && self.panel_cluster.visible
                && !self.panel_cluster.floating
            {
                items.push(("cluster", self.panel_cluster.order));
            }
            if self.panel_bandmap.column == col
                && self.panel_bandmap.visible
                && !self.panel_bandmap.floating
            {
                items.push(("bandmap", self.panel_bandmap.order));
            }
            if self.panel_solar.column == col
                && self.panel_solar.visible
                && !self.panel_solar.floating
            {
                items.push(("solar", self.panel_solar.order));
            }
            if self.panel_satellites.column == col
                && self.panel_satellites.visible
                && !self.panel_satellites.floating
            {
                items.push(("satellites", self.panel_satellites.order));
            }
            if self.panel_world_map.column == col
                && self.panel_world_map.visible
                && !self.panel_world_map.floating
            {
                items.push(("world_map", self.panel_world_map.order));
            }
            if self.panel_waterfall.column == col
                && self.panel_waterfall.visible
                && !self.panel_waterfall.floating
            {
                items.push(("waterfall", self.panel_waterfall.order));
            }

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

    pub fn render_tiles_in_column(&mut self, ui: &mut egui::Ui, col_idx: usize, tiles: &[String]) {
        let lang = self.current_language;
        let is_dragging = self.dragging_tile.is_some();
        let col_name = match col_idx {
            0 => crate::core::i18n::tr_or(lang, "Lewa", "Left"),
            1 => crate::core::i18n::tr_or(lang, "Środek", "Center"),
            _ => crate::core::i18n::tr_or(lang, "Prawa", "Right"),
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
                ui.painter().rect_stroke(
                    rect,
                    8.0,
                    egui::Stroke::new(if hovered { 2.5_f32 } else { 1.5_f32 }, border_color),
                    egui::StrokeKind::Inside,
                );
                let fill = if hovered {
                    egui::Color32::from_rgba_unmultiplied(56, 189, 248, 30)
                } else {
                    egui::Color32::from_rgba_unmultiplied(30, 41, 59, 120)
                };
                ui.painter().rect_filled(rect, 8.0, fill);
                let drop_col_label = if lang == crate::core::i18n::Language::Pl {
                    format!("➕ Upuść kafelek tutaj\n(Kolumna: {col_name})")
                } else {
                    format!("➕ Drop panel here\n(Column: {col_name})")
                };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    drop_col_label,
                    egui::FontId::proportional(12.0),
                    if hovered {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(148, 163, 184)
                    },
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
                        crate::core::i18n::tr_or(lang, "⬇ Upuść tutaj", "⬇ Drop here"),
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
                        [
                            egui::pos2(slot_rect.left() + 20.0, slot_rect.center().y),
                            egui::pos2(slot_rect.right() - 20.0, slot_rect.center().y),
                        ],
                        egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgba_unmultiplied(56, 189, 248, 40),
                        ),
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
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(8))
                .stroke(border_stroke)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Nagłówek kafelka z uchwytem przeciągania i przyciskami
                        ui.horizontal(|ui| {
                            // Uchwyt przeciągania myszką (Drag & Drop handle)
                            let handle_label = egui::RichText::new("⠿").size(16.0).color(egui::Color32::from_rgb(148, 163, 184)).strong();
                            let handle_resp = ui.add(egui::Label::new(handle_label).sense(egui::Sense::drag()))
                                .on_hover_cursor(egui::CursorIcon::Grab)
                                .on_hover_text(crate::core::i18n::tr_or(
                                    lang,
                                    "Przeciągnij myszą, aby przenieść ten kafelek do innej kolumny lub pozycji",
                                    "Drag to move this panel to another column or position",
                                ));

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
                                ui.label(
                                    egui::RichText::new(crate::core::i18n::tr_or(
                                        lang,
                                        "[Przenoszenie...]",
                                        "[Moving...]",
                                    ))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(56, 189, 248)),
                                );
                            }

                            // Własne widżety nagłówka (CAT, SPLIT, Szukaj, Spot)
                            self.render_tile_header_custom(tile_id, ui);

                            // Przyciski przestawiania i zamykania (wyrównane do prawej)
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("✕").on_hover_text(crate::core::i18n::tr_or(lang, "Ukryj ten kafelek", "Hide this panel")).clicked() {
                                    action_close = Some(tile_id.clone());
                                }
                                if ui.button("↗").on_hover_text(crate::core::i18n::tr_or(lang, "Odepnij do osobnego okna pływającego", "Undock into a floating window")).clicked() {
                                    action_popout = Some(tile_id.clone());
                                }

                                if idx + 1 < tiles.len()
                                    && ui.button("▼").on_hover_text(crate::core::i18n::tr_or(lang, "Przesuń niżej", "Move down")).clicked() {
                                        action_move_order = Some((tile_id.clone(), 1));
                                    }
                                if idx > 0
                                    && ui.button("▲").on_hover_text(crate::core::i18n::tr_or(lang, "Przesuń wyżej", "Move up")).clicked() {
                                        action_move_order = Some((tile_id.clone(), -1));
                                    }

                                if col_idx < 2
                                    && ui.button("▶").on_hover_text(crate::core::i18n::tr_or(lang, "Przenieś do kolumny po prawej", "Move to right column")).clicked() {
                                        action_move_col = Some((tile_id.clone(), 1));
                                    }
                                if col_idx > 0
                                    && ui.button("◀").on_hover_text(crate::core::i18n::tr_or(lang, "Przenieś do kolumny po lewej", "Move to left column")).clicked() {
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
            let (slot_rect, slot_resp) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
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
                    crate::core::i18n::tr_or(
                        lang,
                        "⬇ Upuść tutaj (na końcu)",
                        "⬇ Drop here (at end)",
                    ),
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
                    [
                        egui::pos2(slot_rect.left() + 20.0, slot_rect.center().y),
                        egui::pos2(slot_rect.right() - 20.0, slot_rect.center().y),
                    ],
                    egui::Stroke::new(
                        1.0_f32,
                        egui::Color32::from_rgba_unmultiplied(56, 189, 248, 40),
                    ),
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
    pub fn render_tiles_in_column_tabbed(
        &mut self,
        ui: &mut egui::Ui,
        col_idx: usize,
        tiles: &[String],
    ) {
        if tiles.is_empty() {
            return;
        }
        let lang = self.current_language;

        let active = self.active_tab[col_idx].min(tiles.len() - 1);
        self.active_tab[col_idx] = active;

        // Pasek zakładek
        ui.horizontal_wrapped(|ui| {
            for (idx, tile_id) in tiles.iter().enumerate() {
                let selected = idx == active;
                let title = egui::RichText::new(self.tile_title(tile_id))
                    .size(12.0)
                    .strong();
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
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(8))
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(51, 65, 85),
            ))
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
                            if ui
                                .button("✕")
                                .on_hover_text(crate::core::i18n::tr_or(
                                    lang,
                                    "Ukryj ten kafelek",
                                    "Hide this panel",
                                ))
                                .clicked()
                            {
                                action_close = Some(tile_id.clone());
                            }
                            if ui
                                .button("↗")
                                .on_hover_text(crate::core::i18n::tr_or(
                                    lang,
                                    "Odepnij do osobnego okna pływającego",
                                    "Undock into a floating window",
                                ))
                                .clicked()
                            {
                                action_popout = Some(tile_id.clone());
                            }
                            if active + 1 < tiles.len()
                                && ui
                                    .button("▼")
                                    .on_hover_text(crate::core::i18n::tr_or(
                                        lang,
                                        "Przesuń niżej",
                                        "Move down",
                                    ))
                                    .clicked()
                            {
                                action_move_order = Some((tile_id.clone(), 1));
                            }
                            if active > 0
                                && ui
                                    .button("▲")
                                    .on_hover_text(crate::core::i18n::tr_or(
                                        lang,
                                        "Przesuń wyżej",
                                        "Move up",
                                    ))
                                    .clicked()
                            {
                                action_move_order = Some((tile_id.clone(), -1));
                            }
                            if col_idx < 2
                                && ui
                                    .button("▶")
                                    .on_hover_text(crate::core::i18n::tr_or(
                                        lang,
                                        "Przenieś do kolumny po prawej",
                                        "Move to right column",
                                    ))
                                    .clicked()
                            {
                                action_move_col = Some((tile_id.clone(), 1));
                            }
                            if col_idx > 0
                                && ui
                                    .button("◀")
                                    .on_hover_text(crate::core::i18n::tr_or(
                                        lang,
                                        "Przenieś do kolumny po lewej",
                                        "Move to left column",
                                    ))
                                    .clicked()
                            {
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
}
