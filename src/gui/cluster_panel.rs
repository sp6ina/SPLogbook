// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cluster::telnet::{CLUSTER_PRESETS, DxSpot};
use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

/// Wyodrębnia referencję POTA (np. SP-0123) oraz SOTA (np. SP/BZ-001) z komentarza spotu
pub fn extract_pota_sota(comment: &str) -> (Option<String>, Option<String>) {
    let mut pota = None;
    let mut sota = None;
    let upper = comment.to_uppercase();

    for word in upper.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '/');
        // POTA: [1-2 litery]-[3-5 cyfr] (z pominięciem prefiksów modeli transceiverów np. IC-7300, FT-891)
        if let Some(dash_pos) = clean.find('-') {
            let prefix = &clean[..dash_pos];
            let suffix = &clean[dash_pos + 1..];
            if !prefix.is_empty()
                && prefix.len() <= 2
                && prefix.chars().all(|c| c.is_ascii_alphabetic())
                && !matches!(prefix, "IC" | "FT" | "TS" | "KX" | "TM")
                && suffix.len() >= 3
                && suffix.len() <= 5
                && suffix.chars().all(|c| c.is_ascii_digit())
            {
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

pub fn render_cluster_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.views.panel_cluster.visible {
        return;
    }

    let lang = app.gui.appearance.language;

    // Obsługa okna w trybie Multi-Viewport (niezależne okno OS dla wielu monitorów)
    if app.views.panel_cluster.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let ((), captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("cluster_viewport"),
            format!("📡 {} - SPLogbook", tr(key::CLUSTER_TITLE, lang)),
            [720.0, 520.0],
            [420.0, 280.0],
            app.views.panel_cluster.saved_pos,
            app.views.panel_cluster.saved_size,
            |app, ui| {
                egui::Panel::top("cluster_vp_bar").show(ui, |ui| {
                    let palette = ui.palette();
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("↙ {}", tr(key::WINDOW_DOCK, lang)))
                            .on_hover_text(tr(key::WINDOW_DOCK_TOOLTIP, lang))
                            .clicked()
                        {
                            dock_back = true;
                        }
                        ui.separator();
                        if ui
                            .button(
                                egui::RichText::new("📢 Spot DX")
                                    .strong()
                                    .color(palette.accent),
                            )
                            .clicked()
                        {
                            let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
                            app.gui
                                .dialogs
                                .send_spot_dialog
                                .open_with(&app.entry_callsign, freq_khz);
                        }
                        if app.cluster_connected {
                            ui.label(
                                egui::RichText::new(tr(key::CLUSTER_STATUS_ONLINE, lang))
                                    .color(palette.success)
                                    .size(11.0)
                                    .strong(),
                            );
                        } else if app.cluster_connecting {
                            ui.label(
                                egui::RichText::new(tr(key::CLUSTER_STATUS_CONNECTING, lang))
                                    .color(palette.warning)
                                    .size(11.0)
                                    .strong(),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(tr(key::CLUSTER_STATUS_OFFLINE, lang))
                                    .color(palette.text_muted)
                                    .size(11.0),
                            );
                        }
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    render_cluster_body(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if let Some((pos, size)) = captured_geo {
            if app.views.panel_cluster.saved_pos != Some(pos)
                || app.views.panel_cluster.saved_size != Some(size)
            {
                app.views.panel_cluster.saved_pos = Some(pos);
                app.views.panel_cluster.saved_size = Some(size);
                app.save_station_config();
            }
        }
        if dock_back {
            app.views.panel_cluster.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.views.panel_cluster.visible = false;
            app.views.panel_cluster.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.views.panel_cluster.visible;
    let screen = ctx.content_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let mid_h = (screen.height() * 0.56).max(340.0);
    let cluster_y = screen.min.y + 8.0 + mid_h + 10.0;
    let cluster_h = (screen.height() - (cluster_y - screen.min.y) - 10.0).max(220.0);

    let default_pos = [screen.min.x + 488.0, cluster_y];
    let default_size = [mid_w, cluster_h];

    // Minimalistyczny pasek górny okna
    let mut win = egui::Window::new(
        egui::RichText::new(format!("   📡 {}", tr(key::CLUSTER_TITLE, lang)))
            .size(12.0)
            .strong(),
    )
    .open(&mut open)
    .min_size([400.0, 200.0])
    .resizable(true)
    .collapsible(true)
    .constrain_to(screen);

    if app.gui.workspace.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.views.panel_cluster.saved_pos {
        let sz = app.views.panel_cluster.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        let palette = ui.palette();
        ui.horizontal(|ui| {
            if ui
                .button(
                    egui::RichText::new("📢 Spot DX")
                        .strong()
                        .color(palette.accent),
                )
                .clicked()
            {
                let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
                app.gui
                    .dialogs
                    .send_spot_dialog
                    .open_with(&app.entry_callsign, freq_khz);
            }
            if app.cluster_connected {
                ui.label(
                    egui::RichText::new(tr(key::CLUSTER_STATUS_ONLINE, lang))
                        .color(palette.success)
                        .size(11.0)
                        .strong(),
                );
            } else if app.cluster_connecting {
                ui.label(
                    egui::RichText::new(tr(key::CLUSTER_STATUS_CONNECTING, lang))
                        .color(palette.warning)
                        .size(11.0)
                        .strong(),
                );
            } else {
                ui.label(
                    egui::RichText::new(tr(key::CLUSTER_STATUS_OFFLINE, lang))
                        .color(palette.text_muted)
                        .size(11.0),
                );
            }
        });
        ui.separator();
        render_cluster_body(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(
            ctx,
            "cluster_popout_btn",
            res.response.layer_id,
            res.response.rect,
            &mut app.views.panel_cluster.floating,
            true,
            app.gui.appearance.language,
        );
        if app.views.panel_cluster.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let rect = res.response.rect;
            let new_pos = [rect.min.x, rect.min.y];
            let new_size = [rect.width(), rect.height()];
            app.views.panel_cluster.saved_pos = Some(new_pos);
            app.views.panel_cluster.saved_size = Some(new_size);
            if res.response.drag_stopped() {
                app.save_station_config();
            }
        }
    }

    if !open {
        app.views.panel_cluster.visible = false;
        app.save_station_config();
    }
}

pub fn render_cluster_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let palette = ui.palette();
    let lang = app.gui.appearance.language;
    // 1. Sekcja połączenia: Presety, Serwery użytkownika, Adres, Port i Znak logowania
    ui.horizontal(|ui| {
        ui.label(tr(key::CLUSTER_SERVER_LABEL, lang));
        let mut selected_preset_name = tr(key::CLUSTER_SELECT_SERVER, lang).to_string();
        let mut is_custom = false;
        let mut custom_idx = None;

        for (name, host, port) in CLUSTER_PRESETS {
            if app.cluster_host == *host && app.cluster_port == *port {
                selected_preset_name = name.to_string();
                break;
            }
        }
        for (i, cluster) in app.custom_clusters.iter().enumerate() {
            if app.cluster_host == cluster.host && app.cluster_port == cluster.port {
                selected_preset_name = format!("⭐ {}", cluster.name);
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
                ui.label(
                    egui::RichText::new(tr(key::CLUSTER_STD_SERVERS, lang))
                        .size(10.0)
                        .color(palette.text_muted),
                );
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
                    ui.label(
                        egui::RichText::new(tr(key::CLUSTER_USER_SERVERS, lang))
                            .size(10.0)
                            .color(palette.warning),
                    );
                    for cluster in &app.custom_clusters {
                        let is_sel =
                            app.cluster_host == cluster.host && app.cluster_port == cluster.port;
                        if ui
                            .selectable_label(is_sel, format!("⭐ {}", cluster.name))
                            .clicked()
                        {
                            sel_custom = Some((cluster.host.clone(), cluster.port));
                        }
                    }
                }

                if let Some((h, p)) = sel_custom {
                    app.cluster_host = h;
                    app.cluster_port = p;
                    app.save_station_config();
                }

                ui.separator();
                if ui
                    .selectable_label(false, tr(key::CLUSTER_ADD_NEW_SERVER, lang))
                    .clicked()
                {
                    open_add_dialog = true;
                }
            });

        if open_add_dialog {
            app.show_add_cluster_dialog = true;
        }

        if ui
            .button("➕")
            .on_hover_text(tr(key::CLUSTER_ADD_SERVER_TOOLTIP, lang))
            .clicked()
        {
            app.show_add_cluster_dialog = true;
        }

        if is_custom {
            if let Some(idx) = custom_idx {
                if ui
                    .button("🗑")
                    .on_hover_text(tr(key::CLUSTER_DELETE_SERVER_TOOLTIP, lang))
                    .clicked()
                {
                    app.custom_clusters.remove(idx);
                    app.save_station_config();
                }
            }
        }

        if app.cluster_connected {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(tr(key::CLUSTER_DISCONNECT, lang))
                            .color(palette.selection_text),
                    )
                    .fill(palette.danger),
                )
                .clicked()
            {
                app.disconnect_dx_cluster();
            }
        } else {
            let conn_text = if app.cluster_connecting {
                tr(key::TOOLBAR_CLUSTER_CONNECTING, lang)
            } else {
                tr(key::CLUSTER_CONNECT, lang)
            };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(conn_text).color(palette.selection_text))
                        .fill(palette.success),
                )
                .clicked()
            {
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
        ui.label(format!("{}:", tr(key::QSO_CALLSIGN, lang)));
        let c_resp = ui.add(
            egui::TextEdit::singleline(&mut app.cluster_callsign)
                .hint_text(&app.my_station.callsign)
                .desired_width(70.0),
        );

        if h_resp.changed() || p_resp.changed() || c_resp.changed() {
            app.save_station_config();
        }
    });

    // Pasek statusu klastra
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(&app.cluster_status_text)
                .size(10.0)
                .color(palette.text_muted),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .checkbox(
                    &mut app.cluster_auto_connect,
                    tr(key::CLUSTER_AUTO_CONNECT_STARTUP, lang),
                )
                .on_hover_text(tr(key::CLUSTER_AUTO_CONNECT_TOOLTIP, lang))
                .changed()
            {
                app.save_station_config();
            }
        });
    });

    ui.separator();

    // Pasek narzędziowy: Przełącznik bocznego panelu filtrów, Alerty, Czyszczenie
    ui.horizontal(|ui| {
        let filter_btn_text = if app.cluster_show_filter_sidebar {
            tr(key::CLUSTER_HIDE_FILTERS, lang)
        } else {
            tr(key::CLUSTER_SHOW_FILTERS, lang)
        };
        if ui
            .selectable_label(app.cluster_show_filter_sidebar, filter_btn_text)
            .clicked()
        {
            app.cluster_show_filter_sidebar = !app.cluster_show_filter_sidebar;
            app.save_station_config();
        }

        // Alert dźwiękowy dla nowych DXCC
        ui.separator();
        if ui
            .button(if app.solar.band_alert_enabled {
                "🔔 Alert ON"
            } else {
                "🔕 Alert OFF"
            })
            .on_hover_text(
                Msg::new(key::CLUSTER_ALERT_TOOLTIP)
                    .arg("threshold", app.solar.band_alert_k_index_threshold)
                    .text(lang),
            )
            .clicked()
        {
            app.solar.band_alert_enabled = !app.solar.band_alert_enabled;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(format!("🗑 {}", tr(key::CLUSTER_CLEAR_SPOTS, lang)))
                .on_hover_text(tr(key::CLUSTER_CLEAR_SPOTS_TOOLTIP, lang))
                .clicked()
            {
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

    // Pojedynczy przebieg: tanie filtry najpierw, parsowanie POTA/SOTA tylko raz na spot.
    let mut filtered_spots: Vec<(DxSpot, Option<String>, Option<String>)> =
        Vec::with_capacity(app.cluster_spots.len());
    for spot in &app.cluster_spots {
        if filter_band_sel == "VFO" {
            if spot.band != current_band {
                continue;
            }
        } else if filter_band_sel != "ALL" && spot.band != filter_band_sel {
            continue;
        }

        let comment_upper = spot.comment.to_uppercase();
        let freq_hz = (spot.frequency_khz * 1000.0).round() as u64;
        let inferred_mode = crate::core::bandplan::get_suggested_mode(freq_hz);
        let is_digi = spot.is_ft8
            || comment_upper.contains("FT8")
            || comment_upper.contains("FT4")
            || comment_upper.contains("RTTY")
            || comment_upper.contains("PSK")
            || inferred_mode == Some("FT8")
            || inferred_mode == Some("RTTY");
        let is_cw = !is_digi
            && (comment_upper.contains("CW")
                || comment_upper.contains("UP")
                || (!comment_upper.contains("SSB")
                    && !comment_upper.contains("USB")
                    && !comment_upper.contains("LSB")
                    && inferred_mode == Some("CW")));
        let is_ssb = !is_digi
            && (comment_upper.contains("SSB")
                || comment_upper.contains("USB")
                || comment_upper.contains("LSB")
                || (!comment_upper.contains("CW")
                    && matches!(inferred_mode, Some("USB" | "LSB" | "SSB" | "FM"))));

        let mode_skip = match filter_mode_sel.as_str() {
            "CW" => !is_cw,
            "SSB" => !is_ssb,
            "DIGI" => !is_digi,
            _ => false,
        };
        if mode_skip {
            continue;
        }

        let source_skip = match filter_source_sel.as_str() {
            "HUMAN" => spot.is_skimmer,
            "RBN" => !spot.is_skimmer,
            _ => false,
        };
        if source_skip {
            continue;
        }

        let (pota, sota) = extract_pota_sota(&spot.comment);

        if filter_pota_only && pota.is_none() && sota.is_none() {
            continue;
        }

        if !search_q.is_empty() {
            let match_call = spot.dx_call.to_uppercase().contains(&search_q);
            let match_comment = spot.comment.to_uppercase().contains(&search_q);
            let match_spotter = spot.spotter.to_uppercase().contains(&search_q);
            if !match_call && !match_comment && !match_spotter {
                continue;
            }
        }

        filtered_spots.push((spot.clone(), pota, sota));
    }

    let total_spots = app.cluster_spots.len();
    let shown_spots = filtered_spots.len();
    let mut tune_target: Option<(String, f64, String, Option<String>, Option<String>)> = None;

    ui.horizontal(|ui| {
        // Boczny panel filtrów
        if app.cluster_show_filter_sidebar {
            ui.group(|ui| {
                ui.set_width(170.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(tr(key::CLUSTER_FILTER_TITLE, lang))
                            .strong()
                            .color(palette.accent),
                    );
                    ui.add_space(2.0);

                    ui.label(
                        egui::RichText::new(tr(key::CLUSTER_FILTER_SEARCH, lang))
                            .size(10.0)
                            .color(palette.text_muted),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.cluster_search_query)
                            .hint_text("SP, W1, POTA")
                            .desired_width(155.0),
                    );

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("{}:", tr(key::QSO_BAND, lang)))
                            .size(10.0)
                            .color(palette.text_muted),
                    );
                    egui::ComboBox::from_id_salt("sidebar_band_combo")
                        .selected_text(&app.cluster_filter_band_selection)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            for band in &[
                                "ALL", "VFO", "160m", "80m", "40m", "30m", "20m", "17m", "15m",
                                "12m", "10m", "6m", "2m", "70cm",
                            ] {
                                ui.selectable_value(
                                    &mut app.cluster_filter_band_selection,
                                    band.to_string(),
                                    *band,
                                );
                            }
                        });

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("{}:", tr(key::QSO_MODE, lang)))
                            .size(10.0)
                            .color(palette.text_muted),
                    );
                    egui::ComboBox::from_id_salt("sidebar_mode_combo")
                        .selected_text(&app.cluster_filter_mode_selection)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            for mode in &["ALL", "CW", "SSB", "DIGI"] {
                                ui.selectable_value(
                                    &mut app.cluster_filter_mode_selection,
                                    mode.to_string(),
                                    *mode,
                                );
                            }
                        });

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(tr(key::CLUSTER_FILTER_SOURCE, lang))
                            .size(10.0)
                            .color(palette.text_muted),
                    );
                    egui::ComboBox::from_id_salt("sidebar_source_combo")
                        .selected_text(&app.cluster_filter_source)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.cluster_filter_source,
                                "ALL".to_string(),
                                tr(key::CLUSTER_FILTER_SOURCE_ALL, lang),
                            );
                            ui.selectable_value(
                                &mut app.cluster_filter_source,
                                "HUMAN".to_string(),
                                tr(key::CLUSTER_FILTER_SOURCE_HUMAN, lang),
                            );
                            ui.selectable_value(
                                &mut app.cluster_filter_source,
                                "RBN".to_string(),
                                tr(key::CLUSTER_FILTER_SOURCE_SKIMMER, lang),
                            );
                        });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.checkbox(
                        &mut app.cluster_filter_pota_sota_only,
                        tr(key::CLUSTER_FILTER_POTA_SOTA_ONLY, lang),
                    );

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{}\n{}/{}",
                            tr(key::CLUSTER_FILTER_SHOWN_LABEL, lang),
                            shown_spots,
                            total_spots
                        ))
                        .size(10.0)
                        .color(palette.text_muted),
                    );

                    if ui.button(tr(key::CLUSTER_FILTER_CLEAR_BTN, lang)).clicked() {
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
                        ui.label(egui::RichText::new(tr(key::CLUSTER_DX, lang)).strong());
                        ui.label(egui::RichText::new(tr(key::CLUSTER_FREQ, lang)).strong());
                        ui.label(egui::RichText::new(tr(key::QSO_BAND, lang)).strong());
                        ui.label(egui::RichText::new("Program / Ref").strong());
                        ui.label(egui::RichText::new(tr(key::CLUSTER_SPOTTER, lang)).strong());
                        ui.label(egui::RichText::new(tr(key::CLUSTER_COMMENT, lang)).strong());
                        ui.label(egui::RichText::new(tr(key::CLUSTER_TIME, lang)).strong());
                        ui.end_row();

                        for (spot, pota_ref, sota_ref) in &filtered_spots {
                            let badge =
                                app.cluster_spot_badge(&spot.dx_call, &spot.band, spot.is_ft8);
                            let palette = ui.palette();
                            let call_color = badge.color(&palette);

                            if ui
                                .button(
                                    egui::RichText::new(format!(
                                        "{}{}",
                                        spot.dx_call,
                                        badge.suffix()
                                    ))
                                    .strong()
                                    .color(call_color),
                                )
                                .on_hover_text(tr(key::CLUSTER_TUNE_TOOLTIP, lang))
                                .clicked()
                            {
                                tune_target = Some((
                                    spot.dx_call.clone(),
                                    spot.frequency_khz,
                                    spot.band.clone(),
                                    pota_ref.clone(),
                                    sota_ref.clone(),
                                ));
                            }

                            ui.label(format!("{:.1} kHz", spot.frequency_khz));
                            ui.label(egui::RichText::new(&spot.band).color(palette.warning));

                            // Kolumna referencji POTA / SOTA
                            if let Some(p) = pota_ref {
                                ui.label(
                                    egui::RichText::new(format!("🌲 {p}"))
                                        .color(palette.success)
                                        .strong()
                                        .size(11.0),
                                );
                            } else if let Some(s) = sota_ref {
                                ui.label(
                                    egui::RichText::new(format!("⛰️ {s}"))
                                        .color(palette.warning)
                                        .strong()
                                        .size(11.0),
                                );
                            } else {
                                ui.label("-");
                            }

                            ui.label(egui::RichText::new(&spot.spotter).color(palette.text_muted));
                            ui.label(egui::RichText::new(&spot.comment).size(11.0));

                            // Czas spotu + wiek (minuty/godziny od odebrania)
                            let age_secs =
                                (chrono::Utc::now().timestamp() - spot.received_at).max(0);
                            let age_text = if age_secs < 60 {
                                format!("{age_secs}s")
                            } else if age_secs < 3600 {
                                format!("{}m", age_secs / 60)
                            } else {
                                format!("{}h", age_secs / 3600)
                            };
                            ui.label(
                                egui::RichText::new(format!("{} ({})", spot.time_utc, age_text))
                                    .size(10.0)
                                    .color(palette.text_muted),
                            );
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

    let lang = app.gui.appearance.language;
    let mut is_open = app.show_add_cluster_dialog;
    egui::Window::new(
        egui::RichText::new(tr(key::CLUSTER_ADD_DIALOG_TITLE, lang))
            .size(12.0)
            .strong(),
    )
    .open(&mut is_open)
    .default_size([340.0, 160.0])
    .resizable(false)
    .show(ctx, |ui| {
        let palette = ui.palette();
        ui.vertical(|ui| {
            ui.label(tr(key::CLUSTER_ADD_DIALOG_DESC, lang));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(tr(key::CLUSTER_ADD_NAME, lang));
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_cluster_name)
                        .hint_text("np. SP7PKI DX Spider"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(tr(key::CLUSTER_ADD_HOST, lang));
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_cluster_host)
                        .hint_text("np. dxc.sp7pki.pl"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(tr(key::CLUSTER_ADD_PORT, lang));
                ui.add(egui::DragValue::new(&mut app.new_cluster_port).range(1..=65535));
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(
                        egui::RichText::new(tr(key::CLUSTER_ADD_AND_CONNECT, lang))
                            .strong()
                            .color(palette.success),
                    )
                    .clicked()
                {
                    let name = if app.new_cluster_name.trim().is_empty() {
                        app.new_cluster_host.clone()
                    } else {
                        app.new_cluster_name.clone()
                    };
                    let host = app.new_cluster_host.trim().to_string();
                    let port = app.new_cluster_port;
                    if !host.is_empty() {
                        app.custom_clusters
                            .push(crate::core::station::CustomClusterServer {
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

                if ui.button(tr(key::BTN_ADD, lang)).clicked() {
                    let name = if app.new_cluster_name.trim().is_empty() {
                        app.new_cluster_host.clone()
                    } else {
                        app.new_cluster_name.clone()
                    };
                    let host = app.new_cluster_host.trim().to_string();
                    let port = app.new_cluster_port;
                    if !host.is_empty() {
                        app.custom_clusters
                            .push(crate::core::station::CustomClusterServer {
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

                if ui.button(tr(key::BTN_CANCEL, lang)).clicked() {
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

        // Rig model names must not be misidentified as POTA references
        let (pota, sota) = extract_pota_sota("RIG IC-7300 FT-891 TS-590 100W");
        assert_eq!(pota, None);
        assert_eq!(sota, None);
    }
}
