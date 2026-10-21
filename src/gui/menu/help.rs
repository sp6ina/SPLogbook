// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Pomoc
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_HELP, lang), |ui| {
        if ui
            .add(
                egui::Button::new(icons::SEARCH_SM.label(tr(key::PALETTE_TITLE, lang)))
                    .shortcut_text("Ctrl+Shift+P"),
            )
            .clicked()
        {
            app.gui.chrome.show_command_palette = true;
            app.gui.chrome.command_palette_query.clear();
            app.gui.chrome.command_palette_selected = 0;
            ui.close();
        }
        ui.separator();
        if ui
            .add(
                egui::Button::new(icons::MANUAL.label(tr(key::HELP_USER_MANUAL, lang)))
                    .shortcut_text("F12"),
            )
            .clicked()
        {
            app.gui.chrome.show_user_manual = true;
            app.gui.chrome.manual_section = None;
            ui.close();
        }
        if ui
            .add(
                egui::Button::new(icons::KEYBOARD.label(tr(key::HELP_SHORTCUTS_TITLE, lang)))
                    .shortcut_text("F1"),
            )
            .clicked()
        {
            app.gui.chrome.show_shortcuts_window = true;
            ui.close();
        }
        if ui
            .button(icons::SETTINGS.label(tr(key::HELP_OS_SETUP, lang)))
            .clicked()
        {
            app.gui.chrome.show_user_manual = true;
            app.gui.chrome.manual_section = Some("os_setup".to_string());
            ui.close();
        }
        if ui
            .button(icons::INFO_ICON.label(tr(key::HELP_TROUBLESHOOTING, lang)))
            .clicked()
        {
            app.gui.chrome.show_user_manual = true;
            app.gui.chrome.manual_section = Some("troubleshooting".to_string());
            ui.close();
        }
        if ui
            .button(icons::THEME.label(tr(key::HELP_LEGEND, lang)))
            .clicked()
        {
            app.gui.chrome.show_legend_window = true;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::FOLDER.label(tr(key::HELP_OPEN_DATA_DIR, lang)))
            .clicked()
        {
            if let Some(parent) = app.paths.config_file_path.parent() {
                let _ = open::that(parent);
            }
            ui.close();
        }
        if ui
            .button(icons::CHANGELOG.label(tr(key::HELP_CHANGELOG, lang)))
            .clicked()
        {
            app.updates.show_changelog_window = true;
            ui.close();
        }
        if ui
            .button(icons::UPDATE.label(tr(key::HELP_CHECK_UPDATES, lang)))
            .clicked()
        {
            app.updates.show_update_window = true;
            ui.close();
        }
        if ui
            .button(icons::WIZARD.label(tr(key::HELP_RERUN_WIZARD, lang)))
            .clicked()
        {
            app.gui.chrome.show_welcome_wizard = true;
            app.gui.chrome.wizard_tab = 0;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::BUG.label(tr(key::HELP_REPORT_BUG, lang)))
            .clicked()
        {
            let _ = open::that("https://github.com/sp6ina/SPLogbook/issues/new");
            ui.close();
        }
        if ui.button(tr(key::HELP_BUY_COFFEE, lang)).clicked() {
            let _ = open::that("https://buycoffee.to/sp6ina");
            ui.close();
        }
        ui.separator();
        if ui.button(tr(key::TAB_ABOUT, lang)).clicked() {
            app.gui.chrome.show_about_window = true;
            ui.close();
        }
    });
}
