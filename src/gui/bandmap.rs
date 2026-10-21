// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::bandplan::{AMATEUR_BANDS, SegmentMode, get_band_by_freq, get_band_by_name};
use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

pub fn render_bandmap_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.views.panel_bandmap.visible {
        return;
    }

    let lang = app.gui.appearance.language;

    if app.views.panel_bandmap.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let ((), captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("bandmap_viewport"),
            format!("📶 {} - SPLogbook", tr(key::BANDMAP_TITLE, lang)),
            [380.0, 360.0],
            [280.0, 220.0],
            app.views.panel_bandmap.saved_pos,
            app.views.panel_bandmap.saved_size,
            |app, ui| {
                egui::Panel::top("bandmap_vp_bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let lang = app.gui.appearance.language;
                        if ui
                            .button(format!("↙ {}", tr(key::WINDOW_DOCK, lang)))
                            .on_hover_text(tr(key::WINDOW_DOCK_TOOLTIP, lang))
                            .clicked()
                        {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    render_bandmap_content(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if let Some((pos, size)) = captured_geo {
            if app.views.panel_bandmap.saved_pos != Some(pos)
                || app.views.panel_bandmap.saved_size != Some(size)
            {
                app.views.panel_bandmap.saved_pos = Some(pos);
                app.views.panel_bandmap.saved_size = Some(size);
                app.save_station_config();
            }
        }
        if dock_back {
            app.views.panel_bandmap.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.views.panel_bandmap.visible = false;
            app.views.panel_bandmap.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.views.panel_bandmap.visible;
    let screen = ctx.content_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let right_x = screen.min.x + 488.0 + mid_w + 10.0;

    let default_pos = [right_x, screen.min.y + 8.0];
    let default_size = [right_w.max(340.0), 320.0];

    let mut win = egui::Window::new(
        egui::RichText::new(format!("   📶 {}", tr(key::BANDMAP_TITLE, lang)))
            .size(12.0)
            .strong(),
    )
    .open(&mut open)
    .min_size([280.0, 220.0])
    .resizable(true)
    .collapsible(true)
    .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.views.panel_bandmap.saved_pos {
        let sz = app.views.panel_bandmap.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_bandmap_content(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(
            ctx,
            "bandmap_popout_btn",
            res.response.layer_id,
            res.response.rect,
            &mut app.views.panel_bandmap.floating,
            true,
            app.gui.appearance.language,
        );
        if app.views.panel_bandmap.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let rect = res.response.rect;
            let new_pos = [rect.min.x, rect.min.y];
            let new_size = [rect.width(), rect.height()];
            if app.views.panel_bandmap.saved_pos != Some(new_pos)
                || app.views.panel_bandmap.saved_size != Some(new_size)
            {
                app.views.panel_bandmap.saved_pos = Some(new_pos);
                app.views.panel_bandmap.saved_size = Some(new_size);
            }
            if res.response.drag_stopped() {
                app.save_station_config();
            }
        }
    }

    if !open {
        app.views.panel_bandmap.visible = false;
        app.save_station_config();
    }
}

pub fn render_bandmap_content(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let palette = ui.palette();
    let lang = app.gui.appearance.language;

    // 1. Pasek sterowania pasmem
    ui.horizontal(|ui| {
        ui.checkbox(
            &mut app.bandmap_auto_track,
            tr(key::BANDMAP_AUTO_TRACK, lang),
        );
        ui.separator();

        if app.bandmap_auto_track {
            if let Some(current_band) = get_band_by_freq(app.rig_state.frequency_hz) {
                app.bandmap_selected_band = current_band.name.to_string();
            }
        }

        ui.label(format!("{}:", tr(key::QSO_BAND, lang)));
        for band in AMATEUR_BANDS {
            let is_selected = app.bandmap_selected_band == band.name;
            if ui.selectable_label(is_selected, band.name).clicked() {
                app.bandmap_selected_band = band.name.to_string();
                app.bandmap_auto_track = false;
            }
        }
    });

    let current_band_def =
        get_band_by_name(&app.bandmap_selected_band).unwrap_or(&AMATEUR_BANDS[5]); // Domyślnie 20m

    let min_freq = current_band_def.min_freq_hz;
    let max_freq = current_band_def.max_freq_hz;
    let freq_span = (max_freq - min_freq) as f32;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{} {}: {:.3} - {:.3} MHz",
                tr(key::QSO_BAND, lang),
                current_band_def.name,
                (min_freq as f64) / 1_000_000.0,
                (max_freq as f64) / 1_000_000.0
            ))
            .strong()
            .color(palette.accent),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(tr(key::BANDMAP_LEGEND_STATUS, lang))
                    .size(11.0)
                    .color(palette.text_muted),
            );
        });
    });

    // 2. Graficzna skala częstotliwości i panorama pasma
    let desired_size = egui::vec2(ui.available_width(), 100.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    let mut clicked_freq = None;
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let fraction = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
            let target_hz = min_freq + (fraction * freq_span) as u64;
            clicked_freq = Some(target_hz);
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Tło skali
        painter.rect_filled(rect, 4.0, palette.surface_bg);
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0_f32, palette.border),
            egui::StrokeKind::Inside,
        );

        // Wycinki pasma wg IARU
        for seg in current_band_def.segments {
            let seg_start_frac =
                ((seg.start_hz.saturating_sub(min_freq)) as f32 / freq_span).clamp(0.0, 1.0);
            let seg_end_frac =
                ((seg.end_hz.saturating_sub(min_freq)) as f32 / freq_span).clamp(0.0, 1.0);

            let seg_x1 = rect.min.x + seg_start_frac * rect.width();
            let seg_x2 = rect.min.x + seg_end_frac * rect.width();

            let seg_rect = egui::Rect::from_min_max(
                egui::pos2(seg_x1, rect.min.y + 4.0),
                egui::pos2(seg_x2, rect.min.y + 24.0),
            );

            let (color, text_color) = match seg.mode {
                SegmentMode::Cw => (palette.segments.cw, palette.segments.cw_text),
                SegmentMode::Data => (palette.segments.data, palette.segments.data_text),
                SegmentMode::Ssb => (palette.segments.phone, palette.segments.phone_text),
                SegmentMode::Fm => (palette.segments.fm, palette.segments.fm_text),
                SegmentMode::Beacon => (palette.segments.beacon, palette.segments.beacon_text),
            };

            painter.rect_filled(seg_rect, 2.0, color);
            painter.text(
                seg_rect.center(),
                egui::Align2::CENTER_CENTER,
                seg.label,
                egui::FontId::monospace(10.0),
                text_color,
            );
        }

        // Linie siatki częstotliwości (co 25 kHz lub 50 kHz)
        let step_hz = if freq_span > 1_000_000.0 {
            100_000
        } else if freq_span > 300_000.0 {
            50_000
        } else {
            10_000
        };
        let first_tick = min_freq.div_ceil(step_hz) * step_hz;

        let mut tick_hz = first_tick;
        while tick_hz < max_freq {
            let frac = ((tick_hz - min_freq) as f32 / freq_span).clamp(0.0, 1.0);
            let x_pos = rect.min.x + frac * rect.width();

            painter.line_segment(
                [
                    egui::pos2(x_pos, rect.min.y + 26.0),
                    egui::pos2(x_pos, rect.max.y - 18.0),
                ],
                egui::Stroke::new(1.0_f32, palette.border_subtle),
            );

            painter.text(
                egui::pos2(x_pos, rect.max.y - 10.0),
                egui::Align2::CENTER_CENTER,
                format!("{:.3}", (tick_hz as f64) / 1_000_000.0),
                egui::FontId::monospace(9.0),
                palette.text_muted,
            );

            tick_hz += step_hz;
        }

        // 3. Stacje z DX Cluster na skali
        let spots = app.cluster_spots.clone();

        // Zebranie spotów w paśmie i posortowanie wg częstotliwości,
        // a następnie rozłożenie etykiet w "liniach", aby się nie nakładały.
        let mut in_band: Vec<(f32, String)> = spots
            .iter()
            .filter_map(|spot| {
                let spot_hz = (spot.frequency_khz * 1000.0).round() as u64;
                if spot_hz >= min_freq && spot_hz <= max_freq {
                    let frac = ((spot_hz - min_freq) as f32 / freq_span).clamp(0.0, 1.0);
                    let x_pos = rect.min.x + frac * rect.width();
                    Some((x_pos, spot.dx_call.clone()))
                } else {
                    None
                }
            })
            .collect();
        in_band.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Greedy: każda etykieta trafia do pierwszej linii, w której nie koliduje z poprzednią.
        let lane_height = 15.0_f32;
        let gap = 6.0_f32;
        let mut lane_ends: Vec<f32> = Vec::new();

        for (spot_x, call) in &in_band {
            let half_width = (call.len() as f32) * 5.0_f32 + 4.0;
            let label_start = spot_x - half_width;
            let label_end = spot_x + half_width;

            let mut lane = 0;
            while lane < lane_ends.len() && lane_ends[lane] + gap > label_start {
                lane += 1;
            }
            if lane == lane_ends.len() {
                lane_ends.push(label_end);
            } else {
                lane_ends[lane] = lane_ends[lane].max(label_end);
            }

            let y_offset = 32.0 + (lane as f32) * lane_height;
            let x_pos = *spot_x;

            // Pionowa kreska spotu
            painter.line_segment(
                [
                    egui::pos2(x_pos, rect.min.y + 24.0),
                    egui::pos2(x_pos, rect.max.y - 20.0),
                ],
                egui::Stroke::new(1.5_f32, palette.warning),
            );

            // Etykieta znaku
            let badge_pos = egui::pos2(x_pos, rect.min.y + y_offset);
            painter.text(
                badge_pos,
                egui::Align2::CENTER_CENTER,
                call,
                egui::FontId::proportional(10.0),
                palette.warning,
            );
        }

        // 4. Kursor aktualnej częstotliwości VFO
        let vfo_hz = app.rig_state.frequency_hz;
        if vfo_hz >= min_freq && vfo_hz <= max_freq {
            let vfo_frac = ((vfo_hz - min_freq) as f32 / freq_span).clamp(0.0, 1.0);
            let vfo_x = rect.min.x + vfo_frac * rect.width();

            // Czerwona linia kursora VFO
            painter.line_segment(
                [egui::pos2(vfo_x, rect.min.y), egui::pos2(vfo_x, rect.max.y)],
                egui::Stroke::new(2.5_f32, palette.vfo_a),
            );

            // Wskaźnik VFO ze strzałką
            let tri_pts = vec![
                egui::pos2(vfo_x - 5.0, rect.min.y),
                egui::pos2(vfo_x + 5.0, rect.min.y),
                egui::pos2(vfo_x, rect.min.y + 8.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                tri_pts,
                palette.vfo_a,
                egui::Stroke::NONE,
            ));
        }
    }

    if let Some(target_hz) = clicked_freq {
        app.set_vfo_frequency(target_hz);
    }

    // 5. Lista aktywnych stacji na wybranym paśmie z funkcją natychmiastowego QSY
    ui.add_space(8.0);
    ui.label(egui::RichText::new(tr(key::BANDMAP_ACTIVE_STATIONS_HEADING, lang)).strong());

    let mut tune_target: Option<(u64, String)> = None;

    egui::ScrollArea::vertical()
        .max_height(140.0)
        .show(ui, |ui| {
            egui::Grid::new("band_spots_grid")
                .striped(true)
                .num_columns(5)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(tr(key::QSO_CALLSIGN, lang)).strong());
                    ui.label(egui::RichText::new(tr(key::CLUSTER_FREQ, lang)).strong());
                    ui.label(egui::RichText::new(tr(key::CLUSTER_COMMENT, lang)).strong());
                    ui.label(egui::RichText::new(tr(key::CLUSTER_TIME, lang)).strong());
                    ui.label(egui::RichText::new(tr(key::BANDMAP_COL_TUNE, lang)).strong());
                    ui.end_row();

                    for spot in &app.cluster_spots {
                        let spot_hz = (spot.frequency_khz * 1000.0).round() as u64;
                        if spot_hz >= min_freq && spot_hz <= max_freq {
                            ui.label(
                                egui::RichText::new(&spot.dx_call)
                                    .strong()
                                    .color(palette.accent),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.3} MHz",
                                    spot.frequency_khz / 1000.0
                                ))
                                .monospace(),
                            );
                            ui.label(&spot.comment);
                            ui.label(egui::RichText::new(&spot.time_utc).weak());

                            if ui
                                .button(egui::RichText::new("QSY 📻").color(palette.success))
                                .clicked()
                            {
                                tune_target = Some((spot_hz, spot.dx_call.clone()));
                            }
                            ui.end_row();
                        }
                    }
                });
        });

    if let Some((freq_hz, call)) = tune_target {
        app.set_vfo_frequency(freq_hz);
        app.entry_callsign = call;
        app.on_callsign_changed();
    }
}
