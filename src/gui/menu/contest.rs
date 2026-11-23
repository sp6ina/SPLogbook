// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::state::Tile;
use eframe::egui;

/// Menu: Zawody (moduł zawodów, własne zawody, praca wielu operatorów, Cabrillo)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_CONTEST, lang), |ui| {
        if ui
            .button(icons::FLAG.label(tr(key::TOOLS_CONTEST_MODULE, lang)))
            .clicked()
        {
            app.open_tile(Tile::Contest);
            ui.close();
        }
        if ui
            .button(icons::EDIT.label(tr(key::CONTEST_CUSTOM_EDITOR_TITLE, lang)))
            .clicked()
        {
            app.contest.show_custom_editor = true;
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::TOOLS_MULTI_OP, lang)))
            .clicked()
        {
            app.gui.windows.show_multi_op_window = true;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::EXPORT.label(tr(key::CONTEST_EXPORT_CABRILLO, lang)))
            .clicked()
        {
            app.export_cabrillo();
            ui.close();
        }
    });
}
