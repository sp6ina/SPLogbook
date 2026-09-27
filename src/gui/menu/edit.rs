// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Menu: Edycja
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr("menu.edit", lang), |ui| {
        if ui.button(tr("qso.clear", lang)).clicked() {
            app.clear_qso_form();
            ui.close_menu();
        }
        if ui.button(tr("qso.delete", lang)).clicked() {
            app.delete_selected_qso();
            ui.close_menu();
        }
    });
}
