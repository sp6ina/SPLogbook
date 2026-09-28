// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Referencje (bazy danych, dyplomy, narzędzia pomocnicze)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(icons::BOOKS.label(tr("menu.references", lang)), |ui| {
        if ui.button(icons::IOTA.label(tr("ref.iota_db", lang))).clicked() {
            app.iota_dialog.open();
            ui.close();
        }
        if ui.button(icons::WORLD_MAP.label(tr("ref.states_db", lang))).clicked() {
            app.states_dialog.open();
            ui.close();
        }
        if ui.button(icons::LOGBOOK.label(tr("ref.qsl_managers", lang))).clicked() {
            app.qsl_manager_dialog.open();
            ui.close();
        }
        if ui.button(icons::GLOBE.label(tr("ref.prefix_manager", lang))).clicked() {
            app.prefix_manager_dialog.open();
            ui.close();
        }
        ui.separator();
        if ui.button(icons::MOON.label(tr("ref.moon_sun", lang))).clicked() {
            app.astronomy_dialog.open();
            ui.close();
        }
        if ui.button(icons::LIGHTNING.label(tr("ref.wol", lang))).clicked() {
            app.wol_dialog.open();
            ui.close();
        }
        if ui.button(icons::REFRESH.label(tr("ref.update_online_dbs", lang))).clicked() {
            app.trigger_database_update();
            ui.close();
        }
    });
}
