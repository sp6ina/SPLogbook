// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::state::Tile;
use eframe::egui;

/// Menu: Emisje cyfrowe (dekody WSJT-X/JTDX, monitor WSPR)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_DIGITAL, lang), |ui| {
        if ui
            .button(icons::ANTENNA_TUNER.label(tr(key::WSJTX_OPEN_WINDOW, lang)))
            .clicked()
        {
            app.open_tile(Tile::Wsjtx);
            ui.close();
        }
        if ui
            .button(icons::CLUSTER.label(tr(key::TOOLS_WSPR_MONITOR, lang)))
            .clicked()
        {
            app.wspr.show_window = true;
            ui.close();
        }
    });
}
