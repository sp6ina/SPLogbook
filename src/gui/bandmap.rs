// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::bandplan::{get_band_by_freq, get_band_by_name, SegmentMode, AMATEUR_BANDS};
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

pub fn render_bandmap_tile(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("📶 {}", tr("bandmap.title", lang))).strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text("Ukryj ten kafelek").clicked() {
                        app.panel_bandmap.visible = false;
                        app.show_bandmap_window = false;
                        app.save_station_config();
                    }
                    if ui.button("↗").on_hover_text("Odepnij do osobnego okna pływającego").clicked() {
                        app.panel_bandmap.floating = true;
                        app.show_bandmap_window = true;
                        app.save_station_config();
                    }
                });
            });
            ui.separator();
            render_bandmap_content(app, ui);
        });
    });
}

pub fn render_bandmap_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_bandmap.visible {
        return;
    }

    let lang = app.current_language;

    if app.panel_bandmap.floating {
        let mut still_open = true;
        let mut dock_back = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("bandmap_viewport"),
            egui::ViewportBuilder::default()
                .with_title(format!("📶 {} - SPLogbook", tr("bandmap.title", lang)))
                .with_inner_size([380.0, 360.0])
                .with_min_inner_size([280.0, 220.0]),
            |ctx, _class| {
                egui::TopBottomPanel::top("bandmap_vp_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("↙ Przypnij do pulpitu").on_hover_text("Przenieś okno z powrotem na główny pulpit SPLogbook").clicked() {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_bandmap_content(app, ui);
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if dock_back {
            app.panel_bandmap.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_bandmap.visible = false;
            app.panel_bandmap.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.panel_bandmap.visible;
    let screen = ctx.available_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let right_x = screen.min.x + 488.0 + mid_w + 10.0;

    let default_pos = [right_x, screen.min.y + 8.0];
    let default_size = [right_w.max(340.0), 320.0];

    let mut win = egui::Window::new(egui::RichText::new(format!("   📶 {}", tr("bandmap.title", lang))).size(12.0).strong())
        .open(&mut open)
        .min_size([280.0, 220.0])
        .resizable(true)
        .collapsible(true)
        .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_bandmap.saved_pos {
        let sz = app.panel_bandmap.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_bandmap_content(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(ctx, "bandmap_popout_btn", res.response.layer_id, res.response.rect, &mut app.panel_bandmap.floating, true);
        if app.panel_bandmap.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_bandmap.saved_pos != Some(new_pos) || app.panel_bandmap.saved_size != Some(new_size) {
                app.panel_bandmap.saved_pos = Some(new_pos);
                app.panel_bandmap.saved_size = Some(new_size);
                app.save_station_config();
            }
        }
    }

    if !open {
        app.panel_bandmap.visible = false;
        app.save_station_config();
    }
}


pub fn render_bandmap_content(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    // 1. Pasek sterowania pasmem
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.bandmap_auto_track, tr("bandmap.auto_track", lang));
        ui.separator();

        if app.bandmap_auto_track {
            if let Some(current_band) = get_band_by_freq(app.rig_state.frequency_hz) {
                app.bandmap_selected_band = current_band.name.to_string();
            }
        }

        ui.label("Pasmo:");
        for band in AMATEUR_BANDS {
            let is_selected = app.bandmap_selected_band == band.name;
            if ui.selectable_label(is_selected, band.name).clicked() {
                app.bandmap_selected_band = band.name.to_string();
                app.bandmap_auto_track = false;
            }
        }
    });

    let current_band_def = get_band_by_name(&app.bandmap_selected_band)
        .unwrap_or(&AMATEUR_BANDS[5]); // Domyślnie 20m

    let min_freq = current_band_def.min_freq_hz;
    let max_freq = current_band_def.max_freq_hz;
    let freq_span = (max_freq - min_freq) as f32;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "Pasmo {}: {:.3} - {:.3} MHz",
                current_band_def.name,
                (min_freq as f64) / 1_000_000.0,
                (max_freq as f64) / 1_000_000.0
            ))
            .strong()
            .color(egui::Color32::from_rgb(56, 189, 248))
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("🔴 ATNO  🟠 Nowe Pasmo  🟢 Zaliczony")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(148, 163, 184))
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
        painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(15, 23, 42));
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(51, 65, 85)));

        // Wycinki pasma wg IARU
        for seg in current_band_def.segments {
            let seg_start_frac = ((seg.start_hz.saturating_sub(min_freq)) as f32 / freq_span).clamp(0.0, 1.0);
            let seg_end_frac = ((seg.end_hz.saturating_sub(min_freq)) as f32 / freq_span).clamp(0.0, 1.0);

            let seg_x1 = rect.min.x + seg_start_frac * rect.width();
            let seg_x2 = rect.min.x + seg_end_frac * rect.width();

            let seg_rect = egui::Rect::from_min_max(
                egui::pos2(seg_x1, rect.min.y + 4.0),
                egui::pos2(seg_x2, rect.min.y + 24.0),
            );

            let (color, text_color) = match seg.mode {
                SegmentMode::Cw => (egui::Color32::from_rgb(14, 116, 144), egui::Color32::WHITE),
                SegmentMode::Data => (egui::Color32::from_rgb(109, 40, 217), egui::Color32::WHITE),
                SegmentMode::Ssb => (egui::Color32::from_rgb(21, 128, 61), egui::Color32::WHITE),
                SegmentMode::Fm => (egui::Color32::from_rgb(180, 83, 9), egui::Color32::WHITE),
                SegmentMode::Beacon => (egui::Color32::from_rgb(161, 98, 7), egui::Color32::WHITE),
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
        let step_hz = if freq_span > 1_000_000.0 { 100_000 } else if freq_span > 300_000.0 { 50_000 } else { 10_000 };
        let first_tick = min_freq.div_ceil(step_hz) * step_hz;

        let mut tick_hz = first_tick;
        while tick_hz < max_freq {
            let frac = ((tick_hz - min_freq) as f32 / freq_span).clamp(0.0, 1.0);
            let x = rect.min.x + frac * rect.width();

            painter.line_segment(
                [egui::pos2(x, rect.min.y + 26.0), egui::pos2(x, rect.max.y - 18.0)],
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(51, 65, 85)),
            );

            painter.text(
                egui::pos2(x, rect.max.y - 10.0),
                egui::Align2::CENTER_CENTER,
                format!("{:.3}", (tick_hz as f64) / 1_000_000.0),
                egui::FontId::monospace(9.0),
                egui::Color32::from_rgb(148, 163, 184),
            );

            tick_hz += step_hz;
        }

        // 3. Stacje z DX Cluster na skali
        let spots = app.cluster_spots.clone();
        let mut spot_idx = 0;

        for spot in spots.iter() {
            let spot_hz = (spot.frequency_khz * 1000.0) as u64;
            if spot_hz >= min_freq && spot_hz <= max_freq {
                let frac = ((spot_hz - min_freq) as f32 / freq_span).clamp(0.0, 1.0);
                let x = rect.min.x + frac * rect.width();

                // Rozrzut wysokości stacji, aby napisy nie nakładały się
                let y_offset = 32.0 + ((spot_idx % 3) as f32) * 16.0;
                spot_idx += 1;

                // Pionowa kreska spotu
                painter.line_segment(
                    [egui::pos2(x, rect.min.y + 24.0), egui::pos2(x, rect.max.y - 20.0)],
                    egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(250, 204, 21)),
                );

                // Etykieta znaku
                let badge_pos = egui::pos2(x, rect.min.y + y_offset);
                painter.text(
                    badge_pos,
                    egui::Align2::CENTER_CENTER,
                    &spot.dx_call,
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(254, 240, 138),
                );
            }
        }

        // 4. Kursor aktualnej częstotliwości VFO
        let vfo_hz = app.rig_state.frequency_hz;
        if vfo_hz >= min_freq && vfo_hz <= max_freq {
            let vfo_frac = ((vfo_hz - min_freq) as f32 / freq_span).clamp(0.0, 1.0);
            let vfo_x = rect.min.x + vfo_frac * rect.width();

            // Czerwona linia kursora VFO
            painter.line_segment(
                [egui::pos2(vfo_x, rect.min.y), egui::pos2(vfo_x, rect.max.y)],
                egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(239, 68, 68)),
            );

            // Wskaźnik VFO ze strzałką
            let tri_pts = vec![
                egui::pos2(vfo_x - 5.0, rect.min.y),
                egui::pos2(vfo_x + 5.0, rect.min.y),
                egui::pos2(vfo_x, rect.min.y + 8.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                tri_pts,
                egui::Color32::from_rgb(239, 68, 68),
                egui::Stroke::NONE,
            ));
        }
    }

    if let Some(target_hz) = clicked_freq {
        app.set_vfo_frequency(target_hz);
    }

    // 5. Lista aktywnych stacji na wybranym paśmie z funkcją natychmiastowego QSY
    ui.add_space(8.0);
    ui.label(egui::RichText::new("🎯 Aktywne stacje na paśmie (Kliknij, aby nastroić VFO):").strong());

    let mut tune_target: Option<(u64, String)> = None;

    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
        egui::Grid::new("band_spots_grid")
            .striped(true)
            .num_columns(5)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Znak").strong());
                ui.label(egui::RichText::new("Częstotliwość").strong());
                ui.label(egui::RichText::new("Komentarz / Info").strong());
                ui.label(egui::RichText::new("Czas").strong());
                ui.label(egui::RichText::new("Dostrój").strong());
                ui.end_row();

                for spot in app.cluster_spots.iter() {
                    let spot_hz = (spot.frequency_khz * 1000.0) as u64;
                    if spot_hz >= min_freq && spot_hz <= max_freq {
                        ui.label(egui::RichText::new(&spot.dx_call).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                        ui.label(egui::RichText::new(format!("{:.3} MHz", spot.frequency_khz / 1000.0)).monospace());
                        ui.label(&spot.comment);
                        ui.label(egui::RichText::new(&spot.time_utc).weak());

                        if ui.button(egui::RichText::new("QSY 📻").color(egui::Color32::from_rgb(34, 197, 94))).clicked() {
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
