// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Menu: Edycja
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr(key::MENU_EDIT, lang), |ui| {
        if ui.button(tr(key::QSO_CLEAR, lang)).clicked() {
            app.clear_qso_form();
            ui.close();
        }
        if ui.button(tr(key::QSO_DELETE, lang)).clicked() {
            app.delete_selected_qso();
            ui.close();
        }
    });
}
