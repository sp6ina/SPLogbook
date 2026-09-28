// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Plik
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr("menu.file", lang), |ui| {
        if ui.button(icons::FOLDER.label(tr("menu.journal_mgmt", lang))).clicked() {
            if let Ok(db) = app.log_db.lock() {
                app.journal_dialog.reload(&db);
            }
            app.journal_dialog.is_open = true;
            ui.close();
        }
        if ui.button(icons::SEARCH.label(tr("menu.advanced_filter", lang))).clicked() {
            app.advanced_filter_dialog.is_open = true;
            ui.close();
        }
        if ui.button(icons::BADGE.label(tr("menu.qsl_print", lang))).clicked() {
            app.qsl_designer_dialog.is_open = true;
            ui.close();
        }
        if ui.button(icons::BADGE.label(tr("menu.station_profiles", lang))).clicked() {
            app.show_station_profiles_window = true;
            ui.close();
        }
        ui.separator();
        if ui.button(icons::STAR.label(tr("wizard.setup_station", lang))).clicked() {
            app.show_welcome_wizard = true;
            ui.close();
        }
        if ui.button(icons::GLOBE.label(tr("menu.online_sync", lang))).clicked() {
            app.show_online_sync_window = true;
            ui.close();
        }
        if ui.button(tr("menu.import_adif", lang)).clicked() {
            app.trigger_import_adif();
            ui.close();
        }
        if ui.button(tr("menu.export_adif", lang)).clicked() {
            app.trigger_export_adif();
            ui.close();
        }
        if ui.button(tr("menu.export_adx", lang)).clicked() {
            app.trigger_export_adx();
            ui.close();
        }
        if ui.button("Eksport CSV (konfigurowalny)…").clicked() {
            app.csv_export_dialog.open();
            ui.close();
        }
        if ui.button(icons::SOTA_MOUNTAIN.label(tr("menu.export_sota", lang))).clicked() {
            let call = app.my_station.callsign.clone();
            app.sota_dialog.open(&call);
            ui.close();
        }
        if ui.button(icons::DOCUMENT.label(tr("menu.export_pdf", lang))).clicked() {
            app.export_pdf_log();
            ui.close();
        }
        if ui.button(icons::WORLD_MAP.label(tr("menu.export_gpx", lang))).clicked() {
            app.export_gpx_log();
            ui.close();
        }
        ui.separator();
        if ui.button(tr("menu.backup", lang)).clicked() {
            app.run_manual_backup();
            ui.close();
        }
        ui.separator();
        if ui.button(tr("menu.exit", lang)).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}
