// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno menedżera pluginów użytkownika (Rhai).
//! Umożliwia włączanie/wyłączanie silnika, wybór katalogu skryptów, ich
//! przeładowanie oraz podgląd logów i powiadomień generowanych przez pluginy.

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

/// Rysuje okno menedżera pluginów, jeśli jest otwarte.
pub fn render_plugin_manager(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.plugins_ui.show_manager {
        return;
    }

    let mut open = app.plugins_ui.show_manager;
    let lang = app.gui.appearance.language;
    egui::Window::new(tr(key::PLUGINS_WINDOW_TITLE, lang))
        .open(&mut open)
        .default_size([620.0, 480.0])
        .resizable(true)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.heading(
                egui::RichText::new(tr(key::PLUGINS_HEADING, lang))
                    .size(15.0)
                    .color(palette.accent),
            );
            ui.label(
                egui::RichText::new(tr(key::PLUGINS_DESCRIPTION, lang))
                    .small()
                    .color(palette.text_muted),
            );
            ui.add_space(6.0);

            let mut reload = false;

            ui.horizontal(|ui| {
                if ui
                    .checkbox(
                        &mut app.config.plugins_enabled,
                        tr(key::PLUGINS_ENABLED, lang),
                    )
                    .changed()
                {
                    app.services
                        .plugin_engine
                        .set_enabled(app.config.plugins_enabled);
                    app.save_station_config();
                }
                ui.label(tr(key::PLUGINS_DIRECTORY, lang));
                ui.add(
                    egui::TextEdit::singleline(&mut app.config.plugins_dir)
                        .hint_text("plugins")
                        .desired_width(240.0),
                );
                if ui.button(tr(key::PLUGINS_RELOAD, lang)).clicked() {
                    reload = true;
                }
            });

            if reload {
                let dir = std::path::PathBuf::from(app.config.plugins_dir.clone());
                app.services
                    .plugin_engine
                    .set_enabled(app.config.plugins_enabled);
                app.services.plugin_engine.load_dir(&dir);
                app.services.plugin_engine.run_startup();
                app.save_station_config();
            }

            ui.add_space(4.0);
            ui.separator();
            ui.label(
                egui::RichText::new(tr(key::PLUGINS_DETECTED_SCRIPTS, lang))
                    .strong()
                    .color(palette.success),
            );

            let infos = app.services.plugin_engine.infos().to_vec();
            if infos.is_empty() {
                ui.label(
                    egui::RichText::new(tr(key::PLUGINS_NO_SCRIPTS, lang))
                        .italics()
                        .color(palette.text_muted),
                );
            } else {
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for info in infos {
                            ui.horizontal(|ui| {
                                let (icon, color) = if info.loaded {
                                    ("✔", palette.success)
                                } else {
                                    ("✖", palette.danger)
                                };
                                ui.label(egui::RichText::new(icon).color(color));
                                ui.label(egui::RichText::new(&info.name).strong());
                                ui.label(
                                    egui::RichText::new(&info.path)
                                        .small()
                                        .color(palette.text_muted),
                                );
                                if let Some(err) = &info.error {
                                    ui.label(
                                        egui::RichText::new(err).small().color(palette.danger),
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
                    egui::RichText::new(tr(key::PLUGINS_LOG_HEADING, lang))
                        .strong()
                        .color(palette.warning),
                );
                if ui.small_button(tr(key::PLUGINS_REFRESH, lang)).clicked() {
                    let logs = app.services.plugin_engine.drain_log();
                    app.plugins_ui.log.extend(logs);
                    if app.plugins_ui.log.len() > 200 {
                        let overflow = app.plugins_ui.log.len() - 200;
                        app.plugins_ui.log.drain(0..overflow);
                    }
                }
                if ui.small_button(tr(key::PLUGINS_CLEAR, lang)).clicked() {
                    app.plugins_ui.log.clear();
                }
            });

            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    if app.plugins_ui.log.is_empty() {
                        ui.label(
                            egui::RichText::new(tr(key::PLUGINS_NO_LOGS, lang))
                                .italics()
                                .color(palette.text_muted),
                        );
                    } else {
                        for line in &app.plugins_ui.log {
                            ui.label(egui::RichText::new(line).monospace().small());
                        }
                    }
                });
        });

    app.plugins_ui.show_manager = open;
}
