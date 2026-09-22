// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cluster::telnet::CLUSTER_PRESETS;
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Wyodrębnia referencję POTA (np. SP-0123) oraz SOTA (np. SP/BZ-001) z komentarza spotu
pub fn extract_pota_sota(comment: &str) -> (Option<String>, Option<String>) {
    let mut pota = None;
    let mut sota = None;
    let upper = comment.to_uppercase();

    for word in upper.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '/');
        // POTA: [1-4 litery/cyfry]-[3-5 cyfr]
        if let Some(dash_pos) = clean.find('-') {
            let prefix = &clean[..dash_pos];
            let suffix = &clean[dash_pos + 1..];
            if !prefix.is_empty() && prefix.len() <= 4 && suffix.len() >= 3 && suffix.len() <= 5 && suffix.chars().all(|c| c.is_ascii_digit()) && !prefix.contains('/') {
                pota = Some(clean.to_string());
            }
        }
        // SOTA: [1-3 znaki]/[2 znaki]-[3-4 cyfry]
        if clean.contains('/') && clean.contains('-') {
            let parts: Vec<&str> = clean.split('/').collect();
            if parts.len() == 2 {
                if let Some(dash) = parts[1].find('-') {
                    let reg = &parts[1][..dash];
                    let num = &parts[1][dash + 1..];
                    if reg.len() == 2 && num.len() >= 3 && num.chars().all(|c| c.is_ascii_digit()) {
                        sota = Some(clean.to_string());
                    }
                }
            }
        }
    }
    (pota, sota)
}

pub fn render_cluster_panel(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("📡 {}", tr("cluster.title", lang)))
                        .strong()
                        .size(12.0)
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text(tr("window.hide_tooltip", lang)).clicked() {
                        app.panel_cluster.visible = false;
                        app.save_station_config();
                    }
                    if ui.button("↗").on_hover_text(tr("window.popout_tooltip", lang)).clicked() {
                        app.panel_cluster.floating = true;
                        app.save_station_config();
                    }
                    if ui.button(egui::RichText::new("📢 Spot").strong().color(egui::Color32::from_rgb(56, 189, 248)))
                        .on_hover_text(tr("cluster.send_spot_tooltip", lang))
                        .clicked()
                    {
                        let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
                        app.send_spot_dialog.open_with(&app.entry_callsign, freq_khz);
                    }

                    if app.cluster_connected {
                        ui.label(egui::RichText::new(tr("cluster.status_online", lang)).color(egui::Color32::from_rgb(34, 197, 94)).size(10.0).strong());
                    } else if app.cluster_connecting {
                        ui.label(egui::RichText::new(tr("cluster.status_connecting", lang)).color(egui::Color32::from_rgb(250, 204, 21)).size(10.0).strong());
                    } else {
                        ui.label(egui::RichText::new(tr("cluster.status_offline", lang)).color(egui::Color32::from_rgb(148, 163, 184)).size(10.0));
                    }
                });
            });

            render_cluster_body(app, ui);
        });
    });
}

pub fn render_cluster_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_cluster.visible {
        return;
    }

    let lang = app.current_language;

    // Obsługa okna w trybie Multi-Viewport (niezależne okno OS dla wielu monitorów)
    if app.panel_cluster.floating {
        let mut still_open = true;
        let mut dock_back = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("cluster_viewport"),
            egui::ViewportBuilder::default()
                .with_title(format!("📡 {} - SPLogbook", tr("cluster.title", lang)))
                .with_inner_size([720.0, 520.0])
                .with_min_inner_size([420.0, 280.0]),
            |ctx, _class| {
                egui::TopBottomPanel::top("cluster_vp_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(format!("↙ {}", tr("window.dock", lang))).on_hover_text(tr("window.dock_tooltip", lang)).clicked() {
                            dock_back = true;
                        }
                        ui.separator();
                        if ui.button(egui::RichText::new("📢 Spot DX").strong().color(egui::Color32::from_rgb(56, 189, 248))).clicked() {
                            let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
                            app.send_spot_dialog.open_with(&app.entry_callsign, freq_khz);
                        }
                        if app.cluster_connected {
                            ui.label(egui::RichText::new(tr("cluster.status_online", lang)).color(egui::Color32::from_rgb(34, 197, 94)).size(11.0).strong());
                        } else if app.cluster_connecting {
                            ui.label(egui::RichText::new(tr("cluster.status_connecting", lang)).color(egui::Color32::from_rgb(250, 204, 21)).size(11.0).strong());
                        } else {
                            ui.label(egui::RichText::new(tr("cluster.status_offline", lang)).color(egui::Color32::from_rgb(148, 163, 184)).size(11.0));
                        }
                    });
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_cluster_body(app, ui);
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if dock_back {
            app.panel_cluster.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_cluster.visible = false;
            app.panel_cluster.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.panel_cluster.visible;
    let screen = ctx.available_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let mid_h = (screen.height() * 0.56).max(340.0);
    let cluster_y = screen.min.y + 8.0 + mid_h + 10.0;
    let cluster_h = (screen.height() - (cluster_y - screen.min.y) - 10.0).max(220.0);

    let default_pos = [screen.min.x + 488.0, cluster_y];
    let default_size = [mid_w, cluster_h];

    // Minimalistyczny pasek górny okna
    let mut win = egui::Window::new(egui::RichText::new(format!("   📡 {}", tr("cluster.title", lang))).size(12.0).strong())
        .open(&mut open)
        .min_size([400.0, 200.0])
        .resizable(true)
        .collapsible(true)
        .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_cluster.saved_pos {
        let sz = app.panel_cluster.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new("📢 Spot DX").strong().color(egui::Color32::from_rgb(56, 189, 248))).clicked() {
                let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
                app.send_spot_dialog.open_with(&app.entry_callsign, freq_khz);
            }
            if app.cluster_connected {
                ui.label(egui::RichText::new(tr("cluster.status_online", lang)).color(egui::Color32::from_rgb(34, 197, 94)).size(11.0).strong());
            } else if app.cluster_connecting {
                ui.label(egui::RichText::new(tr("cluster.status_connecting", lang)).color(egui::Color32::from_rgb(250, 204, 21)).size(11.0).strong());
            } else {
                ui.label(egui::RichText::new(tr("cluster.status_offline", lang)).color(egui::Color32::from_rgb(148, 163, 184)).size(11.0));
            }
        });
        ui.separator();
        render_cluster_body(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(ctx, "cluster_popout_btn", res.response.layer_id, res.response.rect, &mut app.panel_cluster.floating, true);
        if app.panel_cluster.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_cluster.saved_pos != Some(new_pos) || app.panel_cluster.saved_size != Some(new_size) {
                app.panel_cluster.saved_pos = Some(new_pos);
                app.panel_cluster.saved_size = Some(new_size);
                app.save_station_config();
            }
        }
    }

    if !open {
        app.panel_cluster.visible = false;
        app.save_station_config();
    }
}


pub fn render_cluster_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    // 1. Sekcja połączenia: Presety, Serwery użytkownika, Adres, Port i Znak logowania
    ui.horizontal(|ui| {
        ui.label(tr("cluster.server_label", lang));
        let mut selected_preset_name = tr("cluster.select_server", lang).to_string();
        let mut is_custom = false;
        let mut custom_idx = None;

        for (name, host, port) in CLUSTER_PRESETS {
            if app.cluster_host == *host && app.cluster_port == *port {
                selected_preset_name = name.to_string();
                break;
            }
        }
        for (i, s) in app.custom_clusters.iter().enumerate() {
            if app.cluster_host == s.host && app.cluster_port == s.port {
                selected_preset_name = format!("⭐ {}", s.name);
                is_custom = true;
                custom_idx = Some(i);
                break;
            }
        }

        let mut open_add_dialog = false;

        egui::ComboBox::from_id_salt("cluster_preset_combo")
            .selected_text(&selected_preset_name)
            .width(170.0)
            .show_ui(ui, |ui| {
                ui.label(egui::RichText::new(tr("cluster.std_servers", lang)).size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                for (name, host, port) in CLUSTER_PRESETS {
                    let is_sel = app.cluster_host == *host && app.cluster_port == *port;
                    if ui.selectable_label(is_sel, *name).clicked() {
                        app.cluster_host = host.to_string();
                        app.cluster_port = *port;
                        app.save_station_config();
                    }
                }

                let mut sel_custom = None;
                if !app.custom_clusters.is_empty() {
                    ui.separator();
                    ui.label(egui::RichText::new(tr("cluster.user_servers", lang)).size(10.0).color(egui::Color32::from_rgb(250, 204, 21)));
                    for s in &app.custom_clusters {
                        let is_sel = app.cluster_host == s.host && app.cluster_port == s.port;
                        if ui.selectable_label(is_sel, format!("⭐ {}", s.name)).clicked() {
                            sel_custom = Some((s.host.clone(), s.port));
                        }
                    }
                }

                if let Some((h, p)) = sel_custom {
                    app.cluster_host = h;
                    app.cluster_port = p;
                    app.save_station_config();
                }

                ui.separator();
                if ui.selectable_label(false, tr("cluster.add_new_server", lang)).clicked() {
                    open_add_dialog = true;
                }
            });

        if open_add_dialog {
            app.show_add_cluster_dialog = true;
        }

        if ui.button("➕").on_hover_text(tr("cluster.add_server_tooltip", lang)).clicked() {
            app.show_add_cluster_dialog = true;
        }

        if is_custom {
            if let Some(idx) = custom_idx {
                if ui.button("🗑").on_hover_text(tr("cluster.delete_server_tooltip", lang)).clicked() {
                    app.custom_clusters.remove(idx);
                    app.save_station_config();
                }
            }
        }

        if app.cluster_connected {
            if ui.add(egui::Button::new(egui::RichText::new(tr("cluster.disconnect", lang)).color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(239, 68, 68))).clicked() {
                app.disconnect_dx_cluster();
            }
        } else {
            let conn_text = if app.cluster_connecting { tr("toolbar.cluster_connecting", lang) } else { tr("cluster.connect", lang) };
            if ui.add(egui::Button::new(egui::RichText::new(conn_text).color(egui::Color32::from_rgb(15, 23, 42))).fill(egui::Color32::from_rgb(52, 211, 153))).clicked() {
                app.connect_dx_cluster();
            }
        }
    });

    // Wiersz parametrów: Host, Port, Login
    ui.horizontal(|ui| {
        ui.label("Host:");
        let h_resp = ui.add(egui::TextEdit::singleline(&mut app.cluster_host).desired_width(120.0));
        ui.label("Port:");
        let p_resp = ui.add(egui::DragValue::new(&mut app.cluster_port).range(1..=65535));
        ui.label(format!("{}:", tr("qso.callsign", lang)));
        let c_resp = ui.add(egui::TextEdit::singleline(&mut app.cluster_callsign).hint_text(&app.my_station.callsign).desired_width(70.0));

        if h_resp.changed() || p_resp.changed() || c_resp.changed() {
            app.save_station_config();
        }
    });

    // Pasek statusu klastra
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&app.cluster_status_text).size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.checkbox(&mut app.cluster_auto_connect, tr("cluster.auto_connect_startup", lang)).on_hover_text(tr("cluster.auto_connect_tooltip", lang)).changed() {
                app.save_station_config();
            }
        });
    });

    ui.separator();

    // Pasek narzędziowy: Przełącznik bocznego panelu filtrów, Alerty, Czyszczenie
    ui.horizontal(|ui| {
        let filter_btn_text = if app.cluster_show_filter_sidebar { "◀ Ukryj Filtry" } else { "🔍 Panel Filtrów" };
        if ui.selectable_label(app.cluster_show_filter_sidebar, filter_btn_text).clicked() {
            app.cluster_show_filter_sidebar = !app.cluster_show_filter_sidebar;
            app.save_station_config();
        }

        // Alert dźwiękowy dla nowych DXCC
        ui.separator();
        if ui.button(if app.band_alert_enabled { "🔔 Alert ON" } else { "🔕 Alert OFF" })
            .on_hover_text(format!("Alert przy nowym DXCC/IOTA (K-index ≥ {}). Kliknij aby przełączyć.", app.band_alert_k_index_threshold))
            .clicked()
        {
            app.band_alert_enabled = !app.band_alert_enabled;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(format!("🗑 {}", tr("cluster.clear_spots", lang))).on_hover_text(tr("cluster.clear_spots_tooltip", lang)).clicked() {
                app.cluster_spots.clear();
            }
        });
    });

    ui.separator();

    let current_band = app.entry_band.clone();
    let search_q = app.cluster_search_query.trim().to_uppercase();
    let filter_band_sel = app.cluster_filter_band_selection.clone();
    let filter_mode_sel = app.cluster_filter_mode_selection.clone();
    let filter_source_sel = app.cluster_filter_source.clone();
    let filter_pota_only = app.cluster_filter_pota_sota_only;

    let filtered_spots: Vec<_> = app.cluster_spots.iter().filter(|s| {
        let (pota, sota) = extract_pota_sota(&s.comment);

        if filter_pota_only && pota.is_none() && sota.is_none() {
            return false;
        }

        if !search_q.is_empty() {
            let match_call = s.dx_call.to_uppercase().contains(&search_q);
            let match_comment = s.comment.to_uppercase().contains(&search_q);
            let match_spotter = s.spotter.to_uppercase().contains(&search_q);
            if !match_call && !match_comment && !match_spotter {
                return false;
            }
        }

        if filter_band_sel == "VFO" && s.band != current_band {
            return false;
        } else if filter_band_sel != "ALL" && filter_band_sel != "VFO" && s.band != filter_band_sel {
            return false;
        }

        if filter_mode_sel == "CW" && (s.is_ft8 || s.comment.to_uppercase().contains("FT8") || s.comment.to_uppercase().contains("SSB")) {
            return false;
        } else if filter_mode_sel == "SSB" && (s.is_ft8 || s.comment.to_uppercase().contains("CW")) {
            return false;
        } else if filter_mode_sel == "DIGI" && !s.is_ft8 {
            return false;
        }

        if filter_source_sel == "HUMAN" && s.is_skimmer {
            return false;
        } else if filter_source_sel == "RBN" && !s.is_skimmer {
            return false;
        }

        true
    }).cloned().collect();

    let total_spots = app.cluster_spots.len();
    let shown_spots = filtered_spots.len();
    let mut tune_target: Option<(String, f64, String, Option<String>, Option<String>)> = None;

    ui.horizontal(|ui| {
        // Boczny panel filtrów
        if app.cluster_show_filter_sidebar {
            ui.group(|ui| {
                ui.set_width(170.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(tr("cluster_filter.title", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                    ui.add_space(2.0);

                    ui.label(egui::RichText::new(tr("cluster_filter.search", lang)).size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    ui.add(egui::TextEdit::singleline(&mut app.cluster_search_query).hint_text("SP, W1, POTA").desired_width(155.0));

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("{}:", tr("qso.band", lang))).size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    egui::ComboBox::from_id_salt("sidebar_band_combo")
                        .selected_text(&app.cluster_filter_band_selection)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            for b in &["ALL", "VFO", "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"] {
                                ui.selectable_value(&mut app.cluster_filter_band_selection, b.to_string(), *b);
                            }
                        });

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("{}:", tr("qso.mode", lang))).size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    egui::ComboBox::from_id_salt("sidebar_mode_combo")
                        .selected_text(&app.cluster_filter_mode_selection)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            for m in &["ALL", "CW", "SSB", "DIGI"] {
                                ui.selectable_value(&mut app.cluster_filter_mode_selection, m.to_string(), *m);
                            }
                        });

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(tr("cluster_filter.source", lang)).size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    egui::ComboBox::from_id_salt("sidebar_source_combo")
                        .selected_text(&app.cluster_filter_source)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.cluster_filter_source, "ALL".to_string(), tr("cluster_filter.source_all", lang));
                            ui.selectable_value(&mut app.cluster_filter_source, "HUMAN".to_string(), tr("cluster_filter.source_human", lang));
                            ui.selectable_value(&mut app.cluster_filter_source, "RBN".to_string(), tr("cluster_filter.source_skimmer", lang));
                        });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.checkbox(&mut app.cluster_filter_pota_sota_only, tr("cluster_filter.pota_sota_only", lang));

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(format!("{}\n{}/{}", tr("cluster_filter.shown_label", lang), shown_spots, total_spots)).size(10.0).color(egui::Color32::GRAY));

                    if ui.button(tr("cluster_filter.clear_btn", lang)).clicked() {
                        app.cluster_search_query.clear();
                        app.cluster_filter_band_selection = "ALL".to_string();
                        app.cluster_filter_mode_selection = "ALL".to_string();
                        app.cluster_filter_source = "ALL".to_string();
                        app.cluster_filter_pota_sota_only = false;
                    }
                });
            });
            ui.separator();
        }

        // Tabela spotów klastra
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("cluster_spots_grid")
                    .striped(true)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(tr("cluster.dx", lang)).strong());
                        ui.label(egui::RichText::new(tr("cluster.freq", lang)).strong());
                        ui.label(egui::RichText::new(tr("qso.band", lang)).strong());
                        ui.label(egui::RichText::new("Program / Ref").strong());
                        ui.label(egui::RichText::new(tr("cluster.spotter", lang)).strong());
                        ui.label(egui::RichText::new(tr("cluster.comment", lang)).strong());
                        ui.label(egui::RichText::new(tr("cluster.time", lang)).strong());
                        ui.end_row();

                        for spot in filtered_spots {
                            let (pota_ref, sota_ref) = extract_pota_sota(&spot.comment);

                            let (call_color, badge) = if let Some(info) = app.prefix_matcher.lookup(&spot.dx_call) {
                                let awards = app.awards_engine.lock().unwrap_or_else(|p| p.into_inner());
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

                            if ui.button(egui::RichText::new(format!("{}{}", spot.dx_call, badge)).strong().color(call_color))
                                .on_hover_text(tr("cluster.tune_tooltip", lang))
                                .clicked() 
                            {
                                tune_target = Some((spot.dx_call.clone(), spot.frequency_khz, spot.band.clone(), pota_ref.clone(), sota_ref.clone()));
                            }

                            ui.label(format!("{:.1} kHz", spot.frequency_khz));
                            ui.label(egui::RichText::new(&spot.band).color(egui::Color32::from_rgb(251, 191, 36)));

                            // Kolumna referencji POTA / SOTA
                            if let Some(ref p) = pota_ref {
                                ui.label(egui::RichText::new(format!("🌲 {}", p)).color(egui::Color32::from_rgb(34, 197, 94)).strong().size(11.0));
                            } else if let Some(ref s) = sota_ref {
                                ui.label(egui::RichText::new(format!("⛰️ {}", s)).color(egui::Color32::from_rgb(250, 204, 21)).strong().size(11.0));
                            } else {
                                ui.label("-");
                            }

                            ui.label(egui::RichText::new(&spot.spotter).color(egui::Color32::from_rgb(148, 163, 184)));
                            ui.label(egui::RichText::new(&spot.comment).size(11.0));
                            ui.label(&spot.time_utc);
                            ui.end_row();
                        }
                    });
            });
    });

    if let Some((dx_call, freq_khz, band, pota, sota)) = tune_target {
        app.tune_to_spot(&dx_call, freq_khz, &band);
        if let Some(p) = pota {
            app.entry_pota = p;
        }
        if let Some(s) = sota {
            app.entry_sota = s;
        }
    }
}

pub fn render_add_cluster_dialog(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_add_cluster_dialog {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_add_cluster_dialog;
    egui::Window::new(egui::RichText::new(tr("cluster.add_dialog_title", lang)).size(12.0).strong())
        .open(&mut is_open)
        .default_size([340.0, 160.0])
        .resizable(false)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(tr("cluster.add_dialog_desc", lang));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(tr("cluster.add_name", lang));
                    ui.add(egui::TextEdit::singleline(&mut app.new_cluster_name).hint_text("np. SP7PKI DX Spider"));
                });
                ui.horizontal(|ui| {
                    ui.label(tr("cluster.add_host", lang));
                    ui.add(egui::TextEdit::singleline(&mut app.new_cluster_host).hint_text("np. dxc.sp7pki.pl"));
                });
                ui.horizontal(|ui| {
                    ui.label(tr("cluster.add_port", lang));
                    ui.add(egui::DragValue::new(&mut app.new_cluster_port).range(1..=65535));
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(tr("cluster.add_and_connect", lang)).strong().color(egui::Color32::from_rgb(34, 197, 94))).clicked() {
                        let name = if app.new_cluster_name.trim().is_empty() {
                            app.new_cluster_host.clone()
                        } else {
                            app.new_cluster_name.clone()
                        };
                        let host = app.new_cluster_host.trim().to_string();
                        let port = app.new_cluster_port;
                        if !host.is_empty() {
                            app.custom_clusters.push(crate::core::station::CustomClusterServer {
                                name,
                                host: host.clone(),
                                port,
                            });
                            app.cluster_host = host;
                            app.cluster_port = port;
                            app.save_station_config();
                            app.connect_dx_cluster();
                            app.show_add_cluster_dialog = false;
                        }
                    }

                    if ui.button(tr("btn.add", lang)).clicked() {
                        let name = if app.new_cluster_name.trim().is_empty() {
                            app.new_cluster_host.clone()
                        } else {
                            app.new_cluster_name.clone()
                        };
                        let host = app.new_cluster_host.trim().to_string();
                        let port = app.new_cluster_port;
                        if !host.is_empty() {
                            app.custom_clusters.push(crate::core::station::CustomClusterServer {
                                name,
                                host: host.clone(),
                                port,
                            });
                            app.cluster_host = host;
                            app.cluster_port = port;
                            app.save_station_config();
                            app.show_add_cluster_dialog = false;
                        }
                    }

                    if ui.button(tr("btn.cancel", lang)).clicked() {
                        app.show_add_cluster_dialog = false;
                    }
                });
            });
        });

    app.show_add_cluster_dialog = is_open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pota_sota() {
        let (pota, sota) = extract_pota_sota("CQ POTA SP-0123 from park");
        assert_eq!(pota, Some("SP-0123".to_string()));
        assert_eq!(sota, None);

        let (pota, sota) = extract_pota_sota("QRV SOTA SP/BZ-001 atop peak");
        assert_eq!(pota, None);
        assert_eq!(sota, Some("SP/BZ-001".to_string()));

        let (pota, sota) = extract_pota_sota("Dual act: POTA K-1234 & SOTA W6/NC-421 good sig");
        assert_eq!(pota, Some("K-1234".to_string()));
        assert_eq!(sota, Some("W6/NC-421".to_string()));

        let (pota, sota) = extract_pota_sota("Just regular 599 tu 73");
        assert_eq!(pota, None);
        assert_eq!(sota, None);
    }
}
