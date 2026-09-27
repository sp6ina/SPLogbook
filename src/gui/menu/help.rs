// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Pomoc
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr("menu.help", lang), |ui| {
        if ui.button(icons::SEARCH_SM.label(tr("palette.title", lang))).clicked() {
            app.show_command_palette = true;
            app.command_palette_query.clear();
            app.command_palette_selected = 0;
            ui.close_menu();
        }
        ui.separator();
        if ui.button(tr("tab.about", lang)).clicked() {
            app.show_about_window = true;
            ui.close_menu();
        }
        if ui.button(icons::MANUAL.label("Instrukcja obsługi")).clicked() {
            app.show_user_manual = true;
            app.manual_section = None;
            ui.close_menu();
        }
        if ui.button(icons::CHANGELOG.label("Dziennik zmian")).clicked() {
            app.show_changelog_window = true;
            ui.close_menu();
        }
        if ui.button(icons::UPDATE.label("Sprawdź aktualizacje")).clicked() {
            app.show_update_window = true;
            ui.close_menu();
        }
        if ui.button(icons::KEYBOARD.label(tr("help.shortcuts_title", lang))).clicked() {
            app.show_shortcuts_window = true;
            ui.close_menu();
        }
        if ui.button(icons::THEME.label(tr("help.legend", lang))).clicked() {
            app.show_legend_window = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button(icons::WIZARD.label(tr("help.rerun_wizard", lang))).clicked() {
            app.show_welcome_wizard = true;
            app.wizard_tab = 0;
            ui.close_menu();
        }
        ui.separator();
        if ui.button(icons::BUG.label(tr("help.report_bug", lang))).clicked() {
            let _ = open::that("https://github.com/sp6ina/SPLogbook/issues/new");
            ui.close_menu();
        }
    });
}
