// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Wozniak (SP6INA)

use eframe::egui;
use crate::gui::app::SpLogApp;

pub fn render_wspr_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_wspr_window { return; }
    let mut open = app.show_wspr_window;

    egui::Window::new("WSPR Monitor")
        .id(egui::Id::new("splogbook_wspr_window"))
        .open(&mut open)
        .resizable(true)
        .default_size([680.0, 440.0])
        .show(ctx, |ui| {
            // --- Naglowek z przyciskiem odswiezania ---
            ui.horizontal(|ui| {
                ui.heading("Monitor WSPR");
                ui.label(egui::RichText::new("| wspr.live API")
                    .color(egui::Color32::from_rgb(100, 116, 139)).small());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.wspr_loading {
                        ui.spinner();
                        ui.label("Pobieranie...");
                    } else {
                        let refresh_btn = ui.button("Odswiez spoty");
                        if refresh_btn.clicked() {
                            fetch_wspr_spots_async(app, ctx);
                        }
                    }
                    ui.label(format!("Stacja: {}", app.my_station.callsign));
                });
            });

            if let Some(ref err) = app.wspr_last_error.clone() {
                ui.colored_label(egui::Color32::from_rgb(239, 68, 68),
                    format!("Blad pobierania: {}", err));
            }

            ui.separator();

            // ——— Kontrolki: jednostki odległości + trend SNR ———
            if !app.wspr_spots.is_empty() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut app.wspr_distance_miles, "Odległość w milach");
                    ui.label(egui::RichText::new("Trend SNR:").small().color(egui::Color32::from_rgb(148, 163, 184)));
                    let snrs: Vec<f64> = app.wspr_spots.iter().map(|s| s.snr as f64).collect();
                    if snrs.len() >= 2 {
                        let min = snrs.iter().cloned().fold(f64::INFINITY, f64::min);
                        let max = snrs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                        ui.monospace(egui::RichText::new(snr_sparkline(&snrs, min, max)).color(egui::Color32::from_rgb(56, 189, 248)));
                    }
                });
            }

            // --- Tabela spotow ---
            if app.wspr_spots.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new("Brak spotow WSPR.")
                        .color(egui::Color32::from_rgb(100, 116, 139)));
                    ui.label(egui::RichText::new("Kliknij 'Odswiez spoty' aby pobrac dane z wspr.live")
                        .color(egui::Color32::from_rgb(100, 116, 139)).small());
                    ui.add_space(40.0);
                });
            } else {
                egui::ScrollArea::vertical().id_salt("wspr_scroll").show(ui, |ui| {
                    egui::Grid::new("wspr_table")
                        .num_columns(6)
                        .spacing([12.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Naglowki
                            ui.label(egui::RichText::new("Znak").strong());
                            ui.label(egui::RichText::new("Czestotliwosc (MHz)").strong());
                            ui.label(egui::RichText::new("SNR (dB)").strong());
                            ui.label(egui::RichText::new("Lokator").strong());
                            ui.label(egui::RichText::new("Odleglosc").strong());
                            ui.label(egui::RichText::new("Azymut").strong());
                            ui.end_row();

                            let my_coords = crate::core::geo::locator_to_coordinates(&app.my_station.gridsquare).ok();

                            for spot in &app.wspr_spots {
                                ui.label(egui::RichText::new(&spot.callsign)
                                    .color(egui::Color32::from_rgb(56, 189, 248)));
                                ui.label(format!("{:.6}", spot.frequency));
                                let snr_color = if spot.snr >= 0 {
                                    egui::Color32::from_rgb(34, 197, 94)
                                } else if spot.snr >= -10 {
                                    egui::Color32::from_rgb(250, 204, 21)
                                } else {
                                    egui::Color32::from_rgb(239, 68, 68)
                                };
                                ui.label(egui::RichText::new(format!("{:+}", spot.snr)).color(snr_color));
                                ui.label(&spot.gridsquare);

                                // Odległość i azymut z mojego lokatora
                                let spot_coords = crate::core::geo::locator_to_coordinates(&spot.gridsquare).ok();
                                match (my_coords, spot_coords) {
                                    (Some(m), Some(s)) => {
                                        let km = crate::core::geo::calculate_distance_km(m, s);
                                        let az = crate::core::geo::calculate_bearing_deg(m, s);
                                        if app.wspr_distance_miles {
                                            ui.label(format!("{:.0} mi", km * 0.621371));
                                        } else {
                                            ui.label(format!("{:.0} km", km));
                                        }
                                        ui.label(format!("{:.0}°", az));
                                    }
                                    _ => {
                                        ui.label("—");
                                        ui.label("—");
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new(
                    format!("Lacznie {} spotow z wspr.live", app.wspr_spots.len())
                ).small().color(egui::Color32::from_rgb(100, 116, 139)));
            }
        });

    app.show_wspr_window = open;
}

/// Uruchamia asynchroniczne pobieranie spotow WSPR w tle Tokio.
/// Wynik zapisuje do app.wspr_spots przez kanal.
fn fetch_wspr_spots_async(app: &mut SpLogApp, ctx: &egui::Context) {
    if app.wspr_loading { return; }
    app.wspr_loading = true;
    app.wspr_last_error = None;

    let callsign = app.my_station.callsign.clone();
    let ctx_clone = ctx.clone();

    // Uzywamy tokio::spawn bo jestesmy w srodowisku tokio (eframe + runtime)
    // Wynik przekazujemy przez Arc<Mutex<Option<Result>>>
    let result_slot: std::sync::Arc<std::sync::Mutex<Option<Result<Vec<crate::cloud::wspr::WsprSpot>, String>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot_clone = result_slot.clone();

    tokio::spawn(async move {
        let result = crate::cloud::wspr::fetch_wspr_spots(&callsign).await;
        *slot_clone.lock().unwrap_or_else(|p| p.into_inner()) = Some(result);
        ctx_clone.request_repaint();
    });

    // Zapisujemy Arc w app zeby moc odczytac w nastepnej klatce
    app.wspr_fetch_slot = Some(result_slot);
}

/// Buduje tekstowy sparkline (▁▂▃▄▅▆▇█) z listy wartości SNR.
fn snr_sparkline(values: &[f64], min: f64, max: f64) -> String {
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let range = (max - min).max(1e-9);
    values
        .iter()
        .map(|v| {
            let idx = (((v - min) / range) * 7.0).round().clamp(0.0, 7.0) as usize;
            BLOCKS[idx]
        })
        .collect()
}