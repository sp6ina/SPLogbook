// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Narzędzia (analiza, synchronizacja, eksport, integracje)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr("menu.tools", lang), |ui| {
        if ui.button(icons::SEARCH.label(tr("menu.find_duplicates", lang))).clicked() {
            app.show_find_duplicates_window = true;
            ui.close_menu();
        }
        if ui.button(icons::GLOBE.label(tr("tools.online_sync", lang))).clicked() {
            app.show_online_sync_window = true;
            ui.close_menu();
        }
        if ui.button(icons::WORLD_MAP.label(tr("tools.world_map", lang))).clicked() {
            app.panel_world_map.visible = true;
            app.panel_world_map.floating = true;
            app.show_world_map_window = true;
            ui.close_menu();
        }
        if ui.button(icons::AWARDS.label(tr("tools.awards_matrix", lang))).clicked() {
            app.show_awards_matrix_window = true;
            ui.close_menu();
        }
        if ui.button(icons::STATS.label(tr("tools.statistics", lang))).clicked() {
            app.show_statistics_window = true;
            ui.close_menu();
        }
        if ui.button(icons::CLUSTER.label(tr("tools.wspr_monitor", lang))).clicked() {
            app.show_wspr_window = true;
            ui.close_menu();
        }
        if ui.button(icons::SIGNAL_UP.label("Widmo / Waterfall (SDR)")).clicked() {
            app.waterfall_panel.open();
            ui.close_menu();
        }
        ui.separator();
        ui.menu_button(icons::GLOBE.label(tr("tools.rest_api_server", lang)), |ui| {
            ui.label("Włącz serwer na porcie 8080 (wymaga restartu aplikacji dla zmiany portu)");
            if ui.checkbox(&mut app.rest_api_enabled, tr("tools.rest_api_enable", lang)).changed() {
                app.save_station_config();
                if app.rest_api_enabled {
                    app.start_rest_api_server();
                }
            }
            ui.label(format!("API dostępne pod: http://127.0.0.1:{}/api/v1/", app.rest_api_port));
            if app.rest_api_enabled {
                let key = app.ensure_rest_api_key();
                ui.separator();
                ui.label(icons::WARNING.label("Wymagany nagłówek uwierzytelniający: X-Api-Key"));
                ui.horizontal(|ui| {
                    ui.label("Klucz API:");
                    let mut key_display = key.clone();
                    ui.add(egui::TextEdit::singleline(&mut key_display).desired_width(220.0));
                    if ui.button(icons::LOGBOOK.as_str()).on_hover_text("Kopiuj klucz").clicked() {
                        ui.output_mut(|o| o.copied_text = key.clone());
                    }
                    if ui.button(icons::REFRESH.as_str()).on_hover_text("Wygeneruj nowy klucz (wymaga wyłączenia i włączenia serwera, aby zaczął obowiązywać)").clicked() {
                        app.rest_api_key = crate::api::server::generate_api_key();
                        app.save_station_config();
                    }
                });
            }
            if ui.button(icons::LINK.label(tr("tools.rest_api_open", lang))).clicked() {
                let _ = open::that(format!("http://127.0.0.1:{}/api/v1/status", app.rest_api_port));
                ui.close_menu();
            }
        });
        ui.separator();
        if ui.button(tr("station.equipment", lang)).clicked() {
            app.show_ledger_window = true;
            ui.close_menu();
        }
        if ui.button(icons::BADGE.label(tr("tools.qsl_designer", lang))).clicked() {
            app.qsl_designer_dialog.is_open = true;
            ui.close_menu();
        }
    });
}
