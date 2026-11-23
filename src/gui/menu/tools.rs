// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Narzędzia (dane referencyjne, astronomia, asystent, wtyczki, REST API)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_TOOLS, lang), |ui| {
        super::references::render(app, ui);
        if ui
            .button(icons::MOON.label(tr(key::REF_MOON_SUN, lang)))
            .clicked()
        {
            app.gui.dialogs.astronomy_dialog.open();
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::AI_ASSISTANT.label(tr(key::MENU_OPERATOR_ASSISTANT, lang)))
            .clicked()
        {
            app.gui.windows.show_operator_assistant = true;
            ui.close();
        }
        if ui
            .button(icons::PLUGIN.label(tr(key::MENU_PLUGIN_MANAGER, lang)))
            .clicked()
        {
            app.plugins_ui.show_manager = true;
            ui.close();
        }
        if ui
            .button(icons::STORE.label(tr(key::MENU_PLUGIN_MARKETPLACE, lang)))
            .clicked()
        {
            app.marketplace.show_window = true;
            ui.close();
        }
        ui.separator();
        ui.menu_button(
            icons::GLOBE.label(tr(key::TOOLS_REST_API_SERVER, lang)),
            |ui| {
                ui.label(tr(key::TOOLS_REST_API_HINT, lang));
                if ui
                    .checkbox(
                        &mut app.runtime.rest_api_enabled,
                        tr(key::TOOLS_REST_API_ENABLE, lang),
                    )
                    .changed()
                {
                    app.save_station_config();
                    if app.runtime.rest_api_enabled {
                        app.start_rest_api_server();
                    }
                }
                ui.label(format!(
                    "{} http://127.0.0.1:{}/api/v1/",
                    tr(key::TOOLS_REST_API_URL, lang),
                    app.runtime.rest_api_port
                ));
                if app.runtime.rest_api_enabled {
                    let key = app.ensure_rest_api_key();
                    ui.separator();
                    ui.label(icons::WARNING.label(tr(key::TOOLS_REST_API_HEADER, lang)));
                    ui.horizontal(|ui| {
                        ui.label(tr(key::TOOLS_REST_API_KEY, lang));
                        let mut key_display = key.clone();
                        ui.add(
                            egui::TextEdit::singleline(&mut key_display)
                                .password(true)
                                .desired_width(220.0),
                        );
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
                            app.config.rest_api_key = crate::api::server::generate_api_key();
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
                        app.runtime.rest_api_port
                    ));
                    ui.close();
                }
                if ui
                    .button(icons::BOOKS.label(tr(key::TOOLS_REST_API_DOCS, lang)))
                    .clicked()
                {
                    let _ = open::that(format!(
                        "http://127.0.0.1:{}/api/v1/endpoints",
                        app.runtime.rest_api_port
                    ));
                    ui.close();
                }
            },
        );
    });
}
