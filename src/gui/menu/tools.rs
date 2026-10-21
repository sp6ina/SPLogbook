// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Narzędzia (analiza, synchronizacja, eksport, integracje)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr(key::MENU_TOOLS, lang), |ui| {
        if ui
            .button(icons::SEARCH.label(tr(key::MENU_FIND_DUPLICATES, lang)))
            .clicked()
        {
            app.duplicates.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::TOOLS_ONLINE_SYNC, lang)))
            .clicked()
        {
            app.show_online_sync_window = true;
            ui.close();
        }
        if ui
            .button(icons::WORLD_MAP.label(tr(key::TOOLS_WORLD_MAP, lang)))
            .clicked()
        {
            app.views.panel_world_map.visible = true;
            app.views.panel_world_map.floating = true;
            app.world_map.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::AWARDS.label(tr(key::TOOLS_AWARDS_MATRIX, lang)))
            .clicked()
        {
            app.show_awards_matrix_window = true;
            ui.close();
        }
        if ui
            .button(icons::STATS.label(tr(key::TOOLS_STATISTICS, lang)))
            .clicked()
        {
            app.show_statistics_window = true;
            ui.close();
        }
        if ui
            .button(icons::CLUSTER.label(tr(key::TOOLS_WSPR_MONITOR, lang)))
            .clicked()
        {
            app.wspr.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::SIGNAL_UP.label(tr(key::TOOLS_WATERFALL, lang)))
            .clicked()
        {
            app.views.panel_waterfall.visible = true;
            app.views.panel_waterfall.floating = false;
            app.save_station_config();
            ui.close();
        }
        ui.separator();
        ui.menu_button(
            icons::GLOBE.label(tr(key::TOOLS_REST_API_SERVER, lang)),
            |ui| {
                ui.label(tr(key::TOOLS_REST_API_HINT, lang));
                if ui
                    .checkbox(
                        &mut app.rest_api_enabled,
                        tr(key::TOOLS_REST_API_ENABLE, lang),
                    )
                    .changed()
                {
                    app.save_station_config();
                    if app.rest_api_enabled {
                        app.start_rest_api_server();
                    }
                }
                ui.label(format!(
                    "{} http://127.0.0.1:{}/api/v1/",
                    tr(key::TOOLS_REST_API_URL, lang),
                    app.rest_api_port
                ));
                if app.rest_api_enabled {
                    let key = app.ensure_rest_api_key();
                    ui.separator();
                    ui.label(icons::WARNING.label(tr(key::TOOLS_REST_API_HEADER, lang)));
                    ui.horizontal(|ui| {
                        ui.label(tr(key::TOOLS_REST_API_KEY, lang));
                        let mut key_display = key.clone();
                        ui.add(egui::TextEdit::singleline(&mut key_display).desired_width(220.0));
                        if ui
                            .button(icons::LOGBOOK.as_str())
                            .on_hover_text(tr(key::TOOLS_REST_API_COPY_KEY, lang))
                            .clicked()
                        {
                            ui.ctx().copy_text(key.clone());
                        }
                        if ui
                            .button(icons::REFRESH.as_str())
                            .on_hover_text(tr(key::TOOLS_REST_API_REGEN_KEY, lang))
                            .clicked()
                        {
                            app.rest_api_key = crate::api::server::generate_api_key();
                            app.save_station_config();
                        }
                    });
                }
                if ui
                    .button(icons::LINK.label(tr(key::TOOLS_REST_API_OPEN, lang)))
                    .clicked()
                {
                    let _ = open::that(format!(
                        "http://127.0.0.1:{}/api/v1/status",
                        app.rest_api_port
                    ));
                    ui.close();
                }
                if ui
                    .button(icons::BOOKS.label(tr(key::TOOLS_REST_API_DOCS, lang)))
                    .clicked()
                {
                    let _ = open::that(format!(
                        "http://127.0.0.1:{}/api/v1/endpoints",
                        app.rest_api_port
                    ));
                    ui.close();
                }
            },
        );
        ui.separator();
        if ui.button(tr(key::STATION_EQUIPMENT, lang)).clicked() {
            app.ledger.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::BADGE.label(tr(key::TOOLS_QSL_DESIGNER, lang)))
            .clicked()
        {
            app.qsl_designer_dialog.is_open = true;
            ui.close();
        }
    });
}
