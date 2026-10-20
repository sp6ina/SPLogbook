// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::solar::BandCondition;
use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::text_sparkline;
use crate::gui::theme::ThemeExt;
use eframe::egui;

pub fn render_solar_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_solar.visible {
        return;
    }

    let lang = app.current_language;

    if app.panel_solar.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let ((), captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("solar_viewport"),
            format!("☀ {} - SPLogbook", tr(key::SOLAR_TITLE, lang)),
            [380.0, 260.0],
            [280.0, 180.0],
            app.panel_solar.saved_pos,
            app.panel_solar.saved_size,
            |app, ui| {
                egui::Panel::top("solar_vp_bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
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
                    render_solar_body(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if let Some((pos, size)) = captured_geo {
            if app.panel_solar.saved_pos != Some(pos) || app.panel_solar.saved_size != Some(size) {
                app.panel_solar.saved_pos = Some(pos);
                app.panel_solar.saved_size = Some(size);
                app.save_station_config();
            }
        }
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
    let screen = ctx.content_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);
    let right_x = screen.min.x + 488.0 + mid_w + 10.0;

    let default_pos = [right_x, screen.min.y + 338.0];
    let default_size = [right_w.max(340.0), 240.0];

    let mut win = egui::Window::new(
        egui::RichText::new(format!("   ☀ {}", tr(key::SOLAR_TITLE, lang)))
            .size(12.0)
            .strong(),
    )
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
        crate::gui::render_titlebar_popout_button_if(
            ctx,
            "solar_popout_btn",
            res.response.layer_id,
            res.response.rect,
            &mut app.panel_solar.floating,
            true,
        );
        if app.panel_solar.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let rect = res.response.rect;
            let new_pos = [rect.min.x, rect.min.y];
            let new_size = [rect.width(), rect.height()];
            if app.panel_solar.saved_pos != Some(new_pos)
                || app.panel_solar.saved_size != Some(new_size)
            {
                app.panel_solar.saved_pos = Some(new_pos);
                app.panel_solar.saved_size = Some(new_size);
            }
            if res.response.drag_stopped() {
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
    let palette = ui.palette();

    // Pasek odświeżania danych solarnych
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("NOAA / HamQTH")
                .small()
                .color(palette.text_muted),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.solar_loading {
                ui.spinner();
                ui.label(tr(key::SOLAR_FETCHING, lang));
            } else {
                let ctx = ui.ctx().clone();
                if ui.button(tr(key::SOLAR_BTN_REFRESH, lang)).clicked() {
                    app.refresh_solar_weather(&ctx);
                }
            }
        });
    });

    // Alert o istotnej zmianie propagacji
    if let Some(alert) = app.solar_last_alert.as_ref() {
        let color = if alert.starts_with('⚠') {
            palette.warning
        } else {
            palette.accent
        };
        ui.colored_label(color, alert);
    }

    // Historia SFI (sparkline)
    let sfi_values: Vec<f64> = app.solar_history.iter().map(|w| w.sfi as f64).collect();
    if sfi_values.len() >= 2 {
        let min = sfi_values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = sfi_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SFI:")
                    .small()
                    .color(palette.text_muted),
            );
            ui.monospace(
                egui::RichText::new(text_sparkline(&sfi_values, min, max)).color(palette.warning),
            );
            ui.label(
                egui::RichText::new(format!("min {min:.0} / max {max:.0}"))
                    .small()
                    .color(palette.text_muted),
            );
        });
    }

    let weather = app.space_weather.clone();

    ui.columns(4, |cols| {
        cols[0].vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(weather.sfi.to_string())
                    .strong()
                    .size(16.0)
                    .color(palette.warning),
            );
            ui.label(
                egui::RichText::new("SFI")
                    .size(10.0)
                    .color(palette.text_muted),
            );
        });
        cols[1].vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(weather.ssn.to_string())
                    .strong()
                    .size(16.0)
                    .color(palette.warning),
            );
            ui.label(
                egui::RichText::new("SSN")
                    .size(10.0)
                    .color(palette.text_muted),
            );
        });
        cols[2].vertical_centered(|ui| {
            let k_color = if weather.k_index <= 2 {
                palette.success
            } else if weather.k_index <= 4 {
                palette.warning
            } else {
                palette.danger
            };
            ui.label(
                egui::RichText::new(weather.k_index.to_string())
                    .strong()
                    .size(16.0)
                    .color(k_color),
            );
            ui.label(
                egui::RichText::new("K-Index")
                    .size(10.0)
                    .color(palette.text_muted),
            );
        });
        cols[3].vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(weather.a_index.to_string())
                    .strong()
                    .size(16.0)
                    .color(palette.accent),
            );
            ui.label(
                egui::RichText::new("A-Index")
                    .size(10.0)
                    .color(palette.text_muted),
            );
        });
    });

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        let hf_str = match weather.hf_day_condition() {
            BandCondition::Good => (tr(key::SOLAR_CONDITION_GOOD, lang), palette.success),
            BandCondition::Fair => (tr(key::SOLAR_CONDITION_FAIR, lang), palette.warning),
            BandCondition::Poor => (tr(key::SOLAR_CONDITION_POOR, lang), palette.danger),
        };
        let lf_str = match weather.lf_night_condition() {
            BandCondition::Good => (tr(key::SOLAR_CONDITION_GOOD, lang), palette.success),
            BandCondition::Fair => (tr(key::SOLAR_CONDITION_FAIR, lang), palette.warning),
            BandCondition::Poor => (tr(key::SOLAR_CONDITION_POOR, lang), palette.danger),
        };

        ui.label(format!("{}:", tr(key::SOLAR_HF_DAY, lang)));
        ui.label(egui::RichText::new(hf_str.0).color(hf_str.1).strong());

        ui.add_space(8.0);

        ui.label(format!("{}:", tr(key::SOLAR_LF_NIGHT, lang)));
        ui.label(egui::RichText::new(lf_str.0).color(lf_str.1).strong());
    });

    ui.add_space(6.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(key::SOLAR_PREDICTED_OPENINGS, lang))
                .small()
                .color(palette.text_muted),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            crate::gui::user_manual::help_button(app, ui, "propagation");
        });
    });
    let forecasts = app.solar_band_forecasts();
    ui.horizontal_wrapped(|ui| {
        for (b_name, forecast) in &forecasts {
            let (st_color, sym) = match forecast.status {
                crate::core::propagation::BandOpeningStatus::Open => (palette.success, "●"),
                crate::core::propagation::BandOpeningStatus::Marginal => (palette.warning, "◐"),
                crate::core::propagation::BandOpeningStatus::Closed => (palette.text_muted, "○"),
            };
            ui.label(
                egui::RichText::new(format!("{sym} {b_name}"))
                    .color(st_color)
                    .small(),
            )
            .on_hover_text(format!(
                "{}: {}% REL, {}\nMUF: {:.1} MHz",
                b_name,
                forecast.reliability_pct,
                forecast.status.as_str(),
                forecast.muf_mhz
            ));
        }
    });
}
