// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Wozniak (SP6INA)

use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::text_sparkline;
use crate::gui::theme::ThemeExt;
use eframe::egui;

pub fn render_wspr_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.wspr.show_window {
        return;
    }
    let lang = app.gui.appearance.language;
    let mut open = app.wspr.show_window;

    egui::Window::new(tr(key::WSPR_TITLE, lang))
        .id(egui::Id::new("splogbook_wspr_window"))
        .open(&mut open)
        .resizable(true)
        .default_size([680.0, 440.0])
        .show(ctx, |ui| {
            let palette = ui.palette();
            // Nagłówek z przyciskiem odświeżania
            ui.horizontal(|ui| {
                ui.heading(tr(key::WSPR_TITLE, lang));
                ui.label(
                    egui::RichText::new("| wspr.live API")
                        .color(palette.text_subtle)
                        .small(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.wspr.loading {
                        ui.spinner();
                        ui.label(tr(key::WSPR_FETCHING, lang));
                    } else {
                        let refresh_btn = ui.button(tr(key::WSPR_REFRESH, lang));
                        if refresh_btn.clicked() {
                            fetch_wspr_spots_async(app, ctx);
                        }
                    }
                    ui.label(format!(
                        "{}: {}",
                        tr(key::WIZARD_TAB_STATION, lang),
                        app.my_station.callsign
                    ));
                });
            });

            if let Some(ref err) = app.wspr.last_error.clone() {
                ui.colored_label(
                    palette.danger,
                    Msg::new(key::WSPR_FETCH_ERROR).arg("err", err).text(lang),
                );
            }

            ui.separator();

            // Kontrolki: jednostki odległości + trend SNR
            if !app.wspr.spots.is_empty() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut app.wspr.distance_miles, tr(key::WSPR_DIST_MILES, lang));
                    ui.label(
                        egui::RichText::new(tr(key::WSPR_TREND_SNR, lang))
                            .small()
                            .color(palette.text_muted),
                    );
                    let snrs: Vec<f64> = app.wspr.spots.iter().map(|s| s.snr as f64).collect();
                    if snrs.len() >= 2 {
                        let min = snrs.iter().copied().fold(f64::INFINITY, f64::min);
                        let max = snrs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        ui.monospace(
                            egui::RichText::new(text_sparkline(&snrs, min, max))
                                .color(palette.accent),
                        );
                    }
                });
            }

            // Tabela spotów
            if app.wspr.spots.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        egui::RichText::new(tr(key::WSPR_NO_SPOTS, lang))
                            .color(palette.text_subtle),
                    );
                    ui.label(
                        egui::RichText::new(tr(key::WSPR_REFRESH_HINT, lang))
                            .color(palette.text_subtle)
                            .small(),
                    );
                    ui.add_space(40.0);
                });
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("wspr_scroll")
                    .show(ui, |ui| {
                        egui::Grid::new("wspr_table")
                            .num_columns(6)
                            .spacing([12.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                // Nagłówki
                                ui.label(
                                    egui::RichText::new(tr(key::WSPR_COL_CALLSIGN, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::WSPR_COL_FREQUENCY, lang)).strong(),
                                );
                                ui.label(egui::RichText::new(tr(key::WSPR_COL_SNR, lang)).strong());
                                ui.label(
                                    egui::RichText::new(tr(key::WSPR_COL_LOCATOR, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::WSPR_COL_DISTANCE, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::WSPR_COL_AZIMUTH, lang)).strong(),
                                );
                                ui.end_row();

                                let my_coords = crate::core::geo::locator_to_coordinates(
                                    &app.my_station.gridsquare,
                                )
                                .ok();

                                for spot in &app.wspr.spots {
                                    ui.label(
                                        egui::RichText::new(&spot.callsign).color(palette.accent),
                                    );
                                    ui.label(format!("{:.6}", spot.frequency));
                                    let snr_color = if spot.snr >= 0 {
                                        palette.success
                                    } else if spot.snr >= -10 {
                                        palette.warning
                                    } else {
                                        palette.danger
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{:+}", spot.snr))
                                            .color(snr_color),
                                    );
                                    ui.label(&spot.gridsquare);

                                    // Odległość i azymut z mojego lokatora
                                    let spot_coords =
                                        crate::core::geo::locator_to_coordinates(&spot.gridsquare)
                                            .ok();
                                    if let (Some(m), Some(s)) = (my_coords, spot_coords) {
                                        let km = crate::core::geo::calculate_distance_km(m, s);
                                        let az = crate::core::geo::calculate_bearing_deg(m, s);
                                        if app.wspr.distance_miles {
                                            ui.label(format!("{:.0} mi", km * 0.621_371));
                                        } else {
                                            ui.label(format!("{km:.0} km"));
                                        }
                                        ui.label(format!("{az:.0}°"));
                                    } else {
                                        ui.label("—");
                                        ui.label("—");
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        Msg::new(key::WSPR_TOTAL_SPOTS_COUNT)
                            .arg("count", app.wspr.spots.len())
                            .text(lang),
                    )
                    .small()
                    .color(palette.text_subtle),
                );
            }
        });

    app.wspr.show_window = open;
}

/// Uruchamia asynchroniczne pobieranie spotów WSPR w tle Tokio.
/// Wynik zapisuje do app.wspr.spots przez kanał.
fn fetch_wspr_spots_async(app: &mut SpLogApp, ctx: &egui::Context) {
    if app.wspr.loading {
        return;
    }
    app.wspr.loading = true;
    app.wspr.last_error = None;

    let callsign = app.my_station.callsign.clone();
    let ctx_clone = ctx.clone();

    // Używamy tokio::spawn bo jesteśmy w środowisku tokio (eframe + runtime)
    // Wynik przekazujemy przez Arc<Mutex<Option<Result>>>
    let result_slot: std::sync::Arc<
        std::sync::Mutex<
            Option<Result<Vec<crate::cloud::wspr::WsprSpot>, crate::cloud::wspr::WsprError>>,
        >,
    > = std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot_clone = result_slot.clone();

    tokio::spawn(async move {
        let result = crate::cloud::wspr::fetch_wspr_spots(&callsign).await;
        *slot_clone
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(result);
        ctx_clone.request_repaint();
    });

    // Zapisujemy Arc w app żeby móc odczytać w następnej klatce
    app.wspr.fetch_slot = Some(result_slot);
}
