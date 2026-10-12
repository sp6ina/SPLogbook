// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Referencje (bazy danych, dyplomy, narzędzia pomocnicze)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(icons::BOOKS.label(tr(key::MENU_REFERENCES, lang)), |ui| {
        if ui
            .button(icons::IOTA.label(tr(key::REF_IOTA_DB, lang)))
            .clicked()
        {
            app.iota_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::WORLD_MAP.label(tr(key::REF_STATES_DB, lang)))
            .clicked()
        {
            app.states_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::LOGBOOK.label(tr(key::REF_QSL_MANAGERS, lang)))
            .clicked()
        {
            app.qsl_manager_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::REF_PREFIX_MANAGER, lang)))
            .clicked()
        {
            app.prefix_manager_dialog.open();
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::MOON.label(tr(key::REF_MOON_SUN, lang)))
            .clicked()
        {
            app.astronomy_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::LIGHTNING.label(tr(key::REF_WOL, lang)))
            .clicked()
        {
            app.wol_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::REFRESH.label(tr(key::REF_UPDATE_ONLINE_DBS, lang)))
            .clicked()
        {
            app.trigger_database_update();
            ui.close();
        }
    });
}
