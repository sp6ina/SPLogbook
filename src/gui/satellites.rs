// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::gui::app::SpLogApp;
use crate::core::i18n::tr;
use eframe::egui;

pub fn render_satellites_tile(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("🛰 {}", tr("sat.window_title", lang))).strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text(tr("window.hide_tooltip", lang)).clicked() {
                        app.panel_satellites.visible = false;
                        app.show_satellites_window = false;
                        app.save_station_config();
                    }
                    if ui.button("↗").on_hover_text(tr("window.popout_tooltip", lang)).clicked() {
                        app.panel_satellites.floating = true;
                        app.show_satellites_window = true;
                        app.save_station_config();
                    }
                });
            });
            ui.separator();
            render_satellites_content(app, ui);
        });
    });
}

pub fn render_satellites_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_satellites.visible {
        return;
    }
    let lang = app.current_language;

    if app.panel_satellites.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let (_, captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("satellites_viewport"),
            format!("🛰 {} - SPLogbook", tr("sat.window_title", lang)),
            [400.0, 340.0],
            [280.0, 200.0],
            app.panel_satellites.saved_pos,
            app.panel_satellites.saved_size,
            |app, ui| {
                egui::Panel::top("satellites_vp_bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(format!("↙ {}", tr("window.dock", lang))).on_hover_text(tr("window.dock_tooltip", lang)).clicked() {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    render_satellites_content(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if let Some((pos, size)) = captured_geo {
            if app.panel_satellites.saved_pos != Some(pos) || app.panel_satellites.saved_size != Some(size) {
                app.panel_satellites.saved_pos = Some(pos);
                app.panel_satellites.saved_size = Some(size);
                app.save_station_config();
            }
        }
        if dock_back {
            app.panel_satellites.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_satellites.visible = false;
            app.panel_satellites.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.panel_satellites.visible;
    let screen = ctx.content_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let right_x = screen.min.x + 488.0 + mid_w + 10.0;

    let default_pos = [right_x, screen.min.y + 588.0];
    let default_size = [right_w.max(340.0), 300.0];

    let mut win = egui::Window::new(egui::RichText::new(format!("   🛰 {}", tr("sat.window_title", lang))).size(12.0).strong())
        .open(&mut open)
        .min_size([280.0, 200.0])
        .resizable(true)
        .collapsible(true)
        .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_satellites.saved_pos {
        let sz = app.panel_satellites.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_satellites_content(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(ctx, "satellites_popout_btn", res.response.layer_id, res.response.rect, &mut app.panel_satellites.floating, true);
        if app.panel_satellites.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_satellites.saved_pos != Some(new_pos) || app.panel_satellites.saved_size != Some(new_size) {
                app.panel_satellites.saved_pos = Some(new_pos);
                app.panel_satellites.saved_size = Some(new_size);
                app.save_station_config();
            }
        }
    }

    if !open {
        app.panel_satellites.visible = false;
        app.show_satellites_window = false;
        app.save_station_config();
    }
}


pub fn render_satellites_content(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    let mut tune_clicked = false;
    let mut point_rotor_clicked = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("sat.select", lang));
                    egui::ComboBox::from_id_salt("sat_select")
                        .selected_text(&app.selected_satellite)
                        .show_ui(ui, |ui| {
                            for sat in &["ISS (ZARYA)", "AO-91 (RadFxSat)", "AO-92 (Fox-1D)", "SO-50 (Saudisat 1C)", "RS-44 (DOSAAF)", "CAS-4A", "JO-97 (JY1Sat)"] {
                                ui.selectable_value(&mut app.selected_satellite, sat.to_string(), *sat);
                            }
                        });
                });

                ui.separator();

                ui.columns(2, |cols| {
                    cols[0].group(|ui| {
                        ui.label(egui::RichText::new(tr("sat.position", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                        ui.label(format!("{}: {:.1}°", tr("sat.azimuth", lang), app.sat_azimuth));
                        ui.label(format!("{}: {:.1}°", tr("sat.elevation", lang), app.sat_elevation));
                        ui.label(format!("{}: {:.0} km", tr("sat.slant_range", lang), app.sat_range_km));
                        ui.label(format!("{}: {:.0} km", tr("sat.altitude", lang), app.sat_altitude_km));
                    });

                    cols[1].group(|ui| {
                        ui.label(egui::RichText::new(tr("sat.frequencies", lang)).strong().color(egui::Color32::from_rgb(251, 191, 36)));
                        ui.label(format!("{}: {:.4} MHz", tr("sat.downlink", lang), app.sat_downlink_mhz));
                        ui.label(format!("Doppler RX: {:+.2} kHz", app.sat_rx_doppler_khz));
                        ui.separator();
                        ui.label(format!("{}: {:.4} MHz", tr("sat.uplink", lang), app.sat_uplink_mhz));
                        ui.label(format!("Doppler TX: {:+.2} kHz", app.sat_tx_doppler_khz));
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut app.sat_auto_track_rotator, tr("sat.auto_rotor", lang));
                    ui.checkbox(&mut app.sat_auto_tune_radio, tr("sat.auto_doppler", lang));
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button(format!("📡 {}", tr("sat.set_frequencies", lang))).clicked() {
                        tune_clicked = true;
                    }
                    if ui.button(format!("🧭 {}", tr("sat.point_rotor", lang))).clicked() {
                        point_rotor_clicked = true;
                    }
                });

                ui.separator();

                // ——— Najbliższe przeloty + Doppler ———
                ui.label(egui::RichText::new("🕒 Najbliższe przeloty (24 h)").strong().size(11.0));
                let now = crate::cloud::scheduler::now_unix() as f64;
                if let Some(np) = crate::core::satellite::next_pass(&app.sat_passes, now) {
                    let secs = np.aos_unix - now;
                    if secs > 0.0 {
                        ui.label(format!("⏱ Następny przelot za {}", format_countdown(secs)));
                    } else {
                        ui.label(egui::RichText::new("● Satelita nad horyzontem").color(egui::Color32::from_rgb(34, 197, 94)));
                    }
                } else {
                    ui.label(egui::RichText::new("Brak przelotów w ciągu 24 h").color(egui::Color32::from_rgb(148, 163, 184)));
                }

                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .id_salt("sat_passes")
                    .show(ui, |ui| {
                        egui::Grid::new("sat_passes_grid")
                            .num_columns(4)
                            .spacing([10.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("AOS").strong().small());
                                ui.label(egui::RichText::new("Max el.").strong().small());
                                ui.label(egui::RichText::new("Azymut").strong().small());
                                ui.label(egui::RichText::new("Doppler AOS").strong().small());
                                ui.end_row();

                                for pass in app.sat_passes.iter().take(8) {
                                    ui.label(egui::RichText::new(format_local_time(pass.aos_unix)).small().monospace());
                                    ui.label(egui::RichText::new(format!("{:.0}°", pass.max_elevation_deg)).small());
                                    ui.label(egui::RichText::new(format!("{:.0}°", pass.aos_azimuth_deg)).small());
                                    ui.label(egui::RichText::new(format!("{:+.2} kHz", pass.aos_doppler_khz)).small()
                                        .color(if pass.aos_doppler_khz >= 0.0 {
                                            egui::Color32::from_rgb(34, 197, 94)
                                        } else {
                                            egui::Color32::from_rgb(239, 68, 68)
                                        }));
                                    ui.end_row();
                                }
                            });
                    });
            });

    if tune_clicked {
        app.tune_satellite_frequencies();
    }
    if point_rotor_clicked {
        app.point_rotor_to_satellite();
    }
}

fn format_countdown(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

fn format_local_time(unix: f64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(unix as i64, 0)
        .single()
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".to_string())
}
