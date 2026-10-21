// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Plik
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_FILE, lang), |ui| {
        if ui
            .button(icons::FOLDER.label(tr(key::MENU_JOURNAL_MGMT, lang)))
            .clicked()
        {
            if let Ok(db) = app.services.log_db.lock() {
                app.gui.dialogs.journal_dialog.reload(&db);
            }
            app.gui.dialogs.journal_dialog.is_open = true;
            ui.close();
        }
        if ui
            .button(icons::SEARCH.label(tr(key::MENU_ADVANCED_FILTER, lang)))
            .clicked()
        {
            app.gui.dialogs.advanced_filter_dialog.is_open = true;
            ui.close();
        }
        if ui
            .button(icons::BADGE.label(tr(key::MENU_QSL_PRINT, lang)))
            .clicked()
        {
            app.gui.dialogs.qsl_designer_dialog.is_open = true;
            ui.close();
        }
        if ui
            .button(icons::BADGE.label(tr(key::MENU_STATION_PROFILES, lang)))
            .clicked()
        {
            app.show_station_profiles_window = true;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::STAR.label(tr(key::WIZARD_SETUP_STATION, lang)))
            .clicked()
        {
            app.gui.chrome.show_welcome_wizard = true;
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::MENU_ONLINE_SYNC, lang)))
            .clicked()
        {
            app.online.show_online_sync_window = true;
            ui.close();
        }
        if ui.button(tr(key::MENU_IMPORT_ADIF, lang)).clicked() {
            app.trigger_import_adif();
            ui.close();
        }
        if ui.button(tr(key::MENU_EXPORT_ADIF, lang)).clicked() {
            app.trigger_export_adif();
            ui.close();
        }
        if ui.button(tr(key::MENU_EXPORT_ADX, lang)).clicked() {
            app.trigger_export_adx();
            ui.close();
        }
        if ui.button(tr(key::MENU_EXPORT_CSV, lang)).clicked() {
            app.gui.dialogs.csv_export_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::SOTA_MOUNTAIN.label(tr(key::MENU_EXPORT_SOTA, lang)))
            .clicked()
        {
            let call = app.my_station.callsign.clone();
            app.gui.dialogs.sota_dialog.open(&call);
            ui.close();
        }
        if ui
            .button(icons::DOCUMENT.label(tr(key::MENU_EXPORT_PDF, lang)))
            .clicked()
        {
            app.export_pdf_log();
            ui.close();
        }
        if ui
            .button(icons::WORLD_MAP.label(tr(key::MENU_EXPORT_GPX, lang)))
            .clicked()
        {
            app.export_gpx_log();
            ui.close();
        }
        ui.separator();
        if ui.button(tr(key::MENU_BACKUP, lang)).clicked() {
            app.run_manual_backup();
            ui.close();
        }
        ui.separator();
        if ui.button(tr(key::MENU_EXIT, lang)).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}
