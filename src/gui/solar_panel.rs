// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::solar::BandCondition;
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

pub fn render_solar_panel(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("☀ {}", tr("solar.title", lang))).strong().size(13.0).color(egui::Color32::from_rgb(56, 189, 248)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text(tr("window.hide_tooltip", lang)).clicked() {
                        app.panel_solar.visible = false;
                        app.save_station_config();
                    }
                    if ui.button("↗").on_hover_text(tr("window.popout_tooltip", lang)).clicked() {
                        app.panel_solar.floating = true;
                        app.save_station_config();
                    }
                });
            });
            ui.separator();

            render_solar_body(app, ui);
        });
    });
}

pub fn render_solar_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_solar.visible {
        return;
    }

    let lang = app.current_language;

    if app.panel_solar.floating {
        let mut still_open = true;
        let mut dock_back = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("solar_viewport"),
            egui::ViewportBuilder::default()
                .with_title(format!("☀ {} - SPLogbook", tr("solar.title", lang)))
                .with_inner_size([380.0, 260.0])
                .with_min_inner_size([280.0, 180.0]),
            |ctx, _class| {
                egui::TopBottomPanel::top("solar_vp_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(format!("↙ {}", tr("window.dock", lang))).on_hover_text(tr("window.dock_tooltip", lang)).clicked() {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_solar_body(app, ui);
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if dock_back {
            app.panel_solar.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_solar.visible = false;
            app.panel_solar.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.panel_solar.visible;
    let screen = ctx.available_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let right_x = screen.min.x + 488.0 + mid_w + 10.0;

    let default_pos = [right_x, screen.min.y + 338.0];
    let default_size = [right_w.max(340.0), 240.0];

    let mut win = egui::Window::new(egui::RichText::new(format!("   ☀ {}", tr("solar.title", lang))).size(12.0).strong())
        .open(&mut open)
        .min_size([280.0, 180.0])
        .resizable(true)
        .collapsible(true)
        .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_solar.saved_pos {
        let sz = app.panel_solar.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_solar_body(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(ctx, "solar_popout_btn", res.response.layer_id, res.response.rect, &mut app.panel_solar.floating, true);
        if app.panel_solar.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_solar.saved_pos != Some(new_pos) || app.panel_solar.saved_size != Some(new_size) {
                app.panel_solar.saved_pos = Some(new_pos);
                app.panel_solar.saved_size = Some(new_size);
                app.save_station_config();
            }
        }
    }

    if !open {
        app.panel_solar.visible = false;
        app.save_station_config();
    }
}


pub fn render_solar_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    let weather = &app.space_weather;

            ui.columns(4, |cols| {
                cols[0].vertical_centered(|ui| {
                    ui.label(egui::RichText::new(weather.sfi.to_string()).strong().size(16.0).color(egui::Color32::from_rgb(251, 191, 36)));
                    ui.label(egui::RichText::new("SFI").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                });
                cols[1].vertical_centered(|ui| {
                    ui.label(egui::RichText::new(weather.ssn.to_string()).strong().size(16.0).color(egui::Color32::from_rgb(251, 191, 36)));
                    ui.label(egui::RichText::new("SSN").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                });
                cols[2].vertical_centered(|ui| {
                    let k_color = if weather.k_index <= 2 {
                        egui::Color32::from_rgb(34, 197, 94)
                    } else if weather.k_index <= 4 {
                        egui::Color32::from_rgb(250, 204, 21)
                    } else {
                        egui::Color32::from_rgb(239, 68, 68)
                    };
                    ui.label(egui::RichText::new(weather.k_index.to_string()).strong().size(16.0).color(k_color));
                    ui.label(egui::RichText::new("K-Index").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                });
                cols[3].vertical_centered(|ui| {
                    ui.label(egui::RichText::new(weather.a_index.to_string()).strong().size(16.0).color(egui::Color32::from_rgb(147, 197, 253)));
                    ui.label(egui::RichText::new("A-Index").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                });
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let hf_str = match weather.hf_day_condition() {
                    BandCondition::Good => (tr("solar.condition_good", lang), egui::Color32::from_rgb(34, 197, 94)),
                    BandCondition::Fair => (tr("solar.condition_fair", lang), egui::Color32::from_rgb(250, 204, 21)),
                    BandCondition::Poor => (tr("solar.condition_poor", lang), egui::Color32::from_rgb(239, 68, 68)),
                };
                let lf_str = match weather.lf_night_condition() {
                    BandCondition::Good => (tr("solar.condition_good", lang), egui::Color32::from_rgb(34, 197, 94)),
                    BandCondition::Fair => (tr("solar.condition_fair", lang), egui::Color32::from_rgb(250, 204, 21)),
                    BandCondition::Poor => (tr("solar.condition_poor", lang), egui::Color32::from_rgb(239, 68, 68)),
                };

                ui.label(format!("{}:", tr("solar.hf_day", lang)));
                ui.label(egui::RichText::new(hf_str.0).color(hf_str.1).strong());

                ui.add_space(8.0);

                ui.label(format!("{}:", tr("solar.lf_night", lang)));
                ui.label(egui::RichText::new(lf_str.0).color(lf_str.1).strong());
            });

            ui.add_space(6.0);
            ui.separator();
            ui.label(egui::RichText::new("📊 Przewidywane otwarcie pasm DX (VOACAP-lite):").small().color(egui::Color32::from_rgb(148, 163, 184)));
            ui.horizontal_wrapped(|ui| {
                let sfi = if weather.sfi > 0 { weather.sfi } else { 140 };
                let k = weather.k_index;
                let now = chrono::Utc::now();
                use chrono::{Datelike, Timelike};
                let utc_h = now.hour() as f64 + (now.minute() as f64) / 60.0;
                let doy = now.ordinal();
                let sp = crate::core::geo::Coordinates::new(51.1, 17.0);
                let dx = crate::core::geo::Coordinates::new(40.7, -74.0);

                for &(b_name, freq) in crate::core::propagation::HF_BANDS {
                    let forecast = crate::core::propagation::PropagationEngine::calculate(sp, dx, freq, sfi, k as u8, utc_h, doy);
                    let (st_color, sym) = match forecast.status {
                        crate::core::propagation::BandOpeningStatus::Open => (egui::Color32::from_rgb(34, 197, 94), "●"),
                        crate::core::propagation::BandOpeningStatus::Marginal => (egui::Color32::from_rgb(250, 204, 21), "◐"),
                        crate::core::propagation::BandOpeningStatus::Closed => (egui::Color32::from_rgb(100, 116, 139), "○"),
                    };
                    ui.label(egui::RichText::new(format!("{} {}", sym, b_name)).color(st_color).small())
                        .on_hover_text(format!("{}: {}% REL, {}\nMUF: {:.1} MHz", b_name, forecast.reliability_pct, forecast.status.as_str(), forecast.muf_mhz));
                }
            });
}
