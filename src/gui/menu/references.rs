// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{Msg, key, tr};
use crate::core::reference::user_rules::RulesKind;
use crate::gui::app::SpLogApp;
use crate::gui::app::app_reference::source_status_text;
use crate::gui::icons;
use eframe::egui;

/// Menu: Referencje (bazy danych, dyplomy, narzędzia pomocnicze)
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
        ui.separator();
        if ui
            .button(icons::MOON.label(tr(key::REF_MOON_SUN, lang)))
            .clicked()
        {
            app.gui.dialogs.astronomy_dialog.open();
            ui.close();
        }
        if ui
            .button(icons::LIGHTNING.label(tr(key::REF_WOL, lang)))
            .clicked()
        {
            app.gui.dialogs.wol_dialog.open();
            ui.close();
        }
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
    });
}
