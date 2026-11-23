// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::state::Tile;
use eframe::egui;

/// Menu: Dziennik (formularz QSO, dzienniki, analiza, import i eksport, QSL, kopie)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_LOG, lang), |ui| {
        if ui
            .button(icons::CLEAR.label(tr(key::QSO_CLEAR, lang)))
            .clicked()
        {
            app.clear_qso_form();
            ui.close();
        }
        if ui
            .button(icons::DELETE.label(tr(key::QSO_DELETE, lang)))
            .clicked()
        {
            app.delete_selected_qso();
            ui.close();
        }
        ui.separator();
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
            .button(icons::DUPLICATES.label(tr(key::MENU_FIND_DUPLICATES, lang)))
            .clicked()
        {
            app.duplicates.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::STATS.label(tr(key::TOOLS_STATISTICS, lang)))
            .clicked()
        {
            app.open_tile(Tile::Statistics);
            ui.close();
        }
        if ui
            .button(icons::AWARDS.label(tr(key::TOOLS_AWARDS_MATRIX, lang)))
            .clicked()
        {
            app.open_tile(Tile::Awards);
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::IMPORT.label(tr(key::MENU_IMPORT_ADIF, lang)))
            .clicked()
        {
            app.trigger_import_adif();
            ui.close();
        }
        ui.menu_button(
            icons::EXPORT.label(tr(key::MENU_EXPORT_SUBMENU, lang)),
            |ui| {
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
                    let call = app.config.station.callsign.clone();
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
            },
        );
        if ui
            .button(icons::BADGE.label(tr(key::MENU_QSL_PRINT, lang)))
            .clicked()
        {
            app.gui.dialogs.qsl_designer_dialog.is_open = true;
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::MENU_ONLINE_SYNC, lang)))
            .clicked()
        {
            app.online.show_online_sync_window = true;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::SAVE.label(tr(key::MENU_BACKUP, lang)))
            .clicked()
        {
            app.run_manual_backup();
            ui.close();
        }
        if ui
            .button(icons::RETURN.label(tr(key::MENU_RESTORE_BACKUP, lang)))
            .clicked()
        {
            ui.close();
            app.restore_from_backup_dialog();
        }
        ui.separator();
        if ui.button(tr(key::MENU_EXIT, lang)).clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}
