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
        if ui
            .add(
                egui::Button::new(icons::SEARCH_SM.label(tr("palette.title", lang)))
                    .shortcut_text("Ctrl+Shift+P"),
            )
            .clicked()
        {
            app.show_command_palette = true;
            app.command_palette_query.clear();
            app.command_palette_selected = 0;
            ui.close();
        }
        ui.separator();
        if ui
            .add(
                egui::Button::new(icons::MANUAL.label(tr("help.user_manual", lang)))
                    .shortcut_text("F12"),
            )
            .clicked()
        {
            app.show_user_manual = true;
            app.manual_section = None;
            ui.close();
        }
        if ui
            .add(
                egui::Button::new(icons::KEYBOARD.label(tr("help.shortcuts_title", lang)))
                    .shortcut_text("F1"),
            )
            .clicked()
        {
            app.show_shortcuts_window = true;
            ui.close();
        }
        if ui
            .button(icons::SETTINGS.label(tr("help.os_setup", lang)))
            .clicked()
        {
            app.show_user_manual = true;
            app.manual_section = Some("os_setup".to_string());
            ui.close();
        }
        if ui
            .button(icons::INFO_ICON.label(tr("help.troubleshooting", lang)))
            .clicked()
        {
            app.show_user_manual = true;
            app.manual_section = Some("troubleshooting".to_string());
            ui.close();
        }
        if ui
            .button(icons::THEME.label(tr("help.legend", lang)))
            .clicked()
        {
            app.show_legend_window = true;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::FOLDER.label(tr("help.open_data_dir", lang)))
            .clicked()
        {
            if let Some(parent) = app.config_file_path.parent() {
                let _ = open::that(parent);
            }
            ui.close();
        }
        if ui
            .button(icons::CHANGELOG.label(tr("help.changelog", lang)))
            .clicked()
        {
            app.show_changelog_window = true;
            ui.close();
        }
        if ui
            .button(icons::UPDATE.label(tr("help.check_updates", lang)))
            .clicked()
        {
            app.show_update_window = true;
            ui.close();
        }
        if ui
            .button(icons::WIZARD.label(tr("help.rerun_wizard", lang)))
            .clicked()
        {
            app.show_welcome_wizard = true;
            app.wizard_tab = 0;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::BUG.label(tr("help.report_bug", lang)))
            .clicked()
        {
            let _ = open::that("https://github.com/sp6ina/SPLogbook/issues/new");
            ui.close();
        }
        if ui.button(tr("help.buy_coffee", lang)).clicked() {
            let _ = open::that("https://buycoffee.to/sp6ina");
            ui.close();
        }
        ui.separator();
        if ui.button(tr("tab.about", lang)).clicked() {
            app.show_about_window = true;
            ui.close();
        }
    });
}
