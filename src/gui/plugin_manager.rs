// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno menedżera pluginów użytkownika (Rhai).
//! Umożliwia włączanie/wyłączanie silnika, wybór katalogu skryptów, ich
//! przeładowanie oraz podgląd logów i powiadomień generowanych przez pluginy.

use crate::gui::app::SpLogApp;
use eframe::egui;

/// Rysuje okno menedżera pluginów, jeśli jest otwarte.
pub fn render_plugin_manager(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_plugin_manager {
        return;
    }

    let mut open = app.show_plugin_manager;
    egui::Window::new("🧩 Menedżer pluginów (Rhai)")
        .open(&mut open)
        .default_size([620.0, 480.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading(
                egui::RichText::new("🧩 System pluginów użytkownika — Rhai")
                    .size(15.0)
                    .color(egui::Color32::from_rgb(56, 189, 248)),
            );
            ui.label(
                egui::RichText::new(
                    "Bezpieczny, osadzony język Rhai. Skrypty definiują haki: on_startup(), on_qso_logged(call_sign, band, mode, freq_mhz, is_atno) oraz on_band_opened(band). Dostępne funkcje API: log(text), notify(text), qso_count().",
                )
                .small()
                .color(egui::Color32::GRAY),
            );
            ui.add_space(6.0);

            let mut reload = false;

            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut app.plugins_enabled, "Włączony")
                    .changed()
                {
                    app.plugin_engine.set_enabled(app.plugins_enabled);
                    app.save_station_config();
                }
                ui.label("Katalog:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.plugins_dir)
                        .hint_text("plugins")
                        .desired_width(240.0),
                );
                if ui.button("🔄 Przeładuj").clicked() {
                    reload = true;
                }
            });

            if reload {
                let dir = std::path::PathBuf::from(app.plugins_dir.clone());
                app.plugin_engine.set_enabled(app.plugins_enabled);
                app.plugin_engine.load_dir(&dir);
                app.plugin_engine.run_startup();
                app.save_station_config();
            }

            ui.add_space(4.0);
            ui.separator();
            ui.label(
                egui::RichText::new("📦 Wykryte skrypty")
                    .strong()
                    .color(egui::Color32::from_rgb(134, 239, 172)),
            );

            let infos = app.plugin_engine.infos().to_vec();
            if infos.is_empty() {
                ui.label(
                    egui::RichText::new(
                        "Brak plików .rhai w wybranym katalogu. Utwórz np. plugins/moje_reguly.rhai",
                    )
                    .italics()
                    .color(egui::Color32::from_rgb(148, 163, 184)),
                );
            } else {
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for info in infos {
                            ui.horizontal(|ui| {
                                let (icon, color) = if info.loaded {
                                    ("✔", egui::Color32::from_rgb(34, 197, 94))
                                } else {
                                    ("✖", egui::Color32::from_rgb(239, 68, 68))
                                };
                                ui.label(egui::RichText::new(icon).color(color));
                                ui.label(egui::RichText::new(&info.name).strong());
                                ui.label(
                                    egui::RichText::new(&info.path)
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                                if let Some(err) = &info.error {
                                    ui.label(
                                        egui::RichText::new(err)
                                            .small()
                                            .color(egui::Color32::from_rgb(248, 113, 113)),
                                    )
                                    .on_hover_text(err);
                                }
                            });
                        }
                    });
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("📜 Log")
                        .strong()
                        .color(egui::Color32::from_rgb(250, 204, 21)),
                );
                if ui.small_button("Odśwież").clicked() {
                    let logs = app.plugin_engine.drain_log();
                    app.plugin_log.extend(logs);
                    if app.plugin_log.len() > 200 {
                        let overflow = app.plugin_log.len() - 200;
                        app.plugin_log.drain(0..overflow);
                    }
                }
                if ui.small_button("Wyczyść").clicked() {
                    app.plugin_log.clear();
                }
            });

            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    if app.plugin_log.is_empty() {
                        ui.label(
                            egui::RichText::new("(brak komunikatów log)")
                                .italics()
                                .color(egui::Color32::from_rgb(148, 163, 184)),
                        );
                    } else {
                        for line in &app.plugin_log {
                            ui.label(egui::RichText::new(line).monospace().small());
                        }
                    }
                });
        });

    app.show_plugin_manager = open;
}
