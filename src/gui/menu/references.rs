// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{Msg, key, tr};
use crate::core::reference::tables::TableKind;
use crate::core::reference::user_rules::RulesKind;
use crate::gui::app::SpLogApp;
use crate::gui::app::app_reference::source_status_text;
use crate::gui::icons;
use eframe::egui;

/// Podmenu Narzędzia ▸ Dane referencyjne (bazy, pliki krajów, reguły, tabele)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(icons::BOOKS.label(tr(key::MENU_REFERENCES, lang)), |ui| {
        if ui
            .button(icons::IOTA.label(tr(key::REF_IOTA_DB, lang)))
            .clicked()
        {
            app.gui.dialogs.iota_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::WORLD_MAP.label(tr(key::REF_STATES_DB, lang)))
            .clicked()
        {
            app.gui.dialogs.states_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::LOGBOOK.label(tr(key::REF_QSL_MANAGERS, lang)))
            .clicked()
        {
            app.gui.dialogs.qsl_manager_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::REF_PREFIX_MANAGER, lang)))
            .clicked()
        {
            app.gui.dialogs.prefix_manager_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::BOOKS.label(tr(key::REF_CLUBS_EDITOR, lang)))
            .clicked()
        {
            app.open_clubs_editor();
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::REFRESH.label(tr(key::REF_UPDATE_ONLINE_DBS, lang)))
            .clicked()
        {
            app.trigger_database_update();
            ui.close();
        }
        ui.separator();
        let status = crate::core::reference::loader::last_status();
        let status_line = |k, s: &crate::core::reference::loader::SourceStatus| {
            Msg::new(k)
                .arg("status", source_status_text(s, lang))
                .text(lang)
        };
        ui.label(status_line(key::REF_CF_STATUS, &status.country_files));
        if ui
            .button(icons::REFRESH.label(tr(key::REF_CF_INSTALL, lang)))
            .clicked()
        {
            ui.close();
            app.trigger_country_files_download();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::REF_CF_IMPORT, lang)))
            .clicked()
        {
            ui.close();
            app.trigger_country_files_import();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::REF_CF_ROLLBACK, lang)))
            .clicked()
        {
            ui.close();
            app.trigger_country_files_rollback();
        }
        ui.separator();
        ui.label(status_line(
            key::REF_REGIONAL_STATUS,
            &status.regional_rules,
        ));
        if ui
            .button(icons::WORLD_MAP.label(tr(key::REF_REGIONAL_IMPORT, lang)))
            .clicked()
        {
            ui.close();
            app.trigger_rules_import(RulesKind::Regional);
        }
        if ui
            .button(icons::WORLD_MAP.label(tr(key::REF_REGIONAL_EXPORT, lang)))
            .clicked()
        {
            ui.close();
            app.trigger_rules_export(RulesKind::Regional);
        }
        ui.label(status_line(
            key::REF_OVERRIDES_STATUS,
            &status.user_overrides,
        ));
        if ui
            .button(icons::GLOBE.label(tr(key::REF_OVERRIDES_IMPORT, lang)))
            .clicked()
        {
            ui.close();
            app.trigger_rules_import(RulesKind::Overrides);
        }
        if ui
            .button(icons::GLOBE.label(tr(key::REF_OVERRIDES_EXPORT, lang)))
            .clicked()
        {
            ui.close();
            app.trigger_rules_export(RulesKind::Overrides);
        }
        for (kind, status_key, import_key, export_key) in [
            (
                TableKind::Iota,
                key::REF_IOTA_STATUS,
                key::REF_IOTA_IMPORT,
                key::REF_IOTA_EXPORT,
            ),
            (
                TableKind::States,
                key::REF_STATES_STATUS,
                key::REF_STATES_IMPORT,
                key::REF_STATES_EXPORT,
            ),
            (
                TableKind::QslManagers,
                key::REF_MANAGERS_STATUS,
                key::REF_MANAGERS_IMPORT,
                key::REF_MANAGERS_EXPORT,
            ),
            (
                TableKind::Callbook,
                key::REF_CALLBOOK_STATUS,
                key::REF_CALLBOOK_IMPORT,
                key::REF_CALLBOOK_EXPORT,
            ),
        ] {
            ui.separator();
            ui.label(status_line(
                status_key,
                &crate::core::reference::loader::table_status(kind),
            ));
            if ui
                .button(icons::BOOKS.label(tr(import_key, lang)))
                .clicked()
            {
                ui.close();
                app.trigger_table_import(kind);
            }
            if ui
                .button(icons::BOOKS.label(tr(export_key, lang)))
                .clicked()
            {
                ui.close();
                app.trigger_table_export(kind);
            }
        }
    });
}
