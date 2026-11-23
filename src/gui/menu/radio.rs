// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::state::Tile;
use eframe::egui;

/// Menu: Radio (CAT, stacja i sprzęt, kluczowanie, wodospad, satelity, zdalne włączanie)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_RADIO, lang), |ui| {
        if ui
            .button(icons::RADIO.label(tr(key::TOOLS_CAT_CONNECTION, lang)))
            .clicked()
        {
            app.gui.windows.show_cat_settings_window = true;
            ui.close();
        }
        if ui
            .button(icons::BADGE.label(tr(key::MENU_STATION_PROFILES, lang)))
            .clicked()
        {
            app.gui.windows.show_station_profiles_window = true;
            ui.close();
        }
        if ui
            .button(icons::ANTENNA_TUNER.label(tr(key::STATION_EQUIPMENT, lang)))
            .clicked()
        {
            app.open_tile(Tile::Ledger);
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::CW_TERMINAL.label(tr(key::TOOLS_CW_TERMINAL, lang)))
            .clicked()
        {
            app.gui.dialogs.cw_terminal_dialog.is_open = true;
            ui.close();
        }
        if ui
            .button(icons::CW_KEYER.label(tr(key::TOOLS_CW_MACROS, lang)))
            .clicked()
        {
            app.open_tile(Tile::CwMacros);
            ui.close();
        }
        if ui
            .button(icons::VOICE_KEYER.label(tr(key::MENU_VOICE_KEYER, lang)))
            .clicked()
        {
            app.voice_keyer.show_window = true;
            ui.close();
        }
        ui.separator();
        if ui
            .button(icons::SIGNAL_UP.label(tr(key::TOOLS_WATERFALL, lang)))
            .clicked()
        {
            app.views.panel_waterfall.visible = true;
            app.views.panel_waterfall.floating = false;
            app.save_station_config();
            ui.close();
        }
        if ui
            .button(icons::SATELLITE.label(tr(key::TOOLS_SATELLITES, lang)))
            .clicked()
        {
            app.views.panel_satellites.visible = true;
            app.views.panel_satellites.floating = true;
            app.satellites.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::LIGHTNING.label(tr(key::REF_WOL, lang)))
            .clicked()
        {
            app.gui.dialogs.wol_dialog.open();
            ui.close();
        }
    });
}
