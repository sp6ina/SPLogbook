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
                        .num_columns(4)
                        .spacing([12.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Naglowki
                            ui.label(egui::RichText::new("Znak").strong());
                            ui.label(egui::RichText::new("Czestotliwosc (MHz)").strong());
                            ui.label(egui::RichText::new("SNR (dB)").strong());
                            ui.label(egui::RichText::new("Lokator").strong());
                            ui.end_row();

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
        *slot_clone.lock().unwrap() = Some(result);
        ctx_clone.request_repaint();
    });

    // Zapisujemy Arc w app zeby moc odczytac w nastepnej klatce
    app.wspr_fetch_slot = Some(result_slot);
}