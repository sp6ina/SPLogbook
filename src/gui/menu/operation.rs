// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cluster::telnet::CLUSTER_PRESETS;
use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Operacja (radio, DX Cluster, CAT, satelity, contesty, multi-op)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    use crate::gui::theme::ThemeExt;
    let palette = ui.palette();
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_OPERATION, lang), |ui| {
        // Sekcja DX Cluster
        ui.label(
            egui::RichText::new(icons::CLUSTER.label(tr(key::MENU_DX_CLUSTER, lang)))
                .strong()
                .color(palette.accent),
        );
        ui.separator();
        if app.cluster_connected {
            if ui
                .button(icons::DOT_RED.label(tr(key::CLUSTER_DISCONNECT_BTN, lang)))
                .clicked()
            {
                app.disconnect_dx_cluster();
                ui.close();
            }
        } else {
            let txt = if app.cluster_connecting {
                format!("● {}", tr(key::CLUSTER_CONNECTING_BTN, lang))
            } else {
                format!("● {}", tr(key::CLUSTER_CONNECT_BTN, lang))
            };
            if ui.button(txt).clicked() {
                app.connect_dx_cluster();
                ui.close();
            }
        }

        if ui
            .button(icons::SPOT.label(tr(key::CLUSTER_ANNOUNCE_SPOT, lang)))
            .clicked()
        {
            let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
            app.gui
                .dialogs
                .send_spot_dialog
                .open_with(&app.entry_callsign, freq_khz);
            ui.close();
        }

        ui.separator();
        ui.label(egui::RichText::new(tr(key::CLUSTER_QUICK_SERVER, lang)).strong());
        for (name, host, port) in CLUSTER_PRESETS {
            let is_current = app.config.cluster_host == *host && app.config.cluster_port == *port;
            if ui.selectable_label(is_current, *name).clicked() {
                app.config.cluster_host = host.to_string();
                app.config.cluster_port = *port;
                app.save_station_config();
                if app.cluster_connected {
                    app.connect_dx_cluster();
                }
                ui.close();
            }
        }

        ui.separator();
        ui.label(egui::RichText::new(tr(key::CLUSTER_FILTERS_OPTIONS, lang)).strong());
        if ui
            .checkbox(
                &mut app.config.cluster_auto_connect,
                tr(key::CLUSTER_AUTO_CONNECT_STARTUP, lang),
            )
            .changed()
        {
            app.save_station_config();
        }
        if ui
            .checkbox(
                &mut app.config.cluster_filter_current_band,
                tr(key::CLUSTER_FILTER_VFO_BAND, lang),
            )
            .changed()
        {
            app.save_station_config();
        }
        if ui
            .checkbox(
                &mut app.config.cluster_hide_ft8,
                tr(key::CLUSTER_HIDE_FT8, lang),
            )
            .changed()
        {
            app.save_station_config();
        }
        if ui
            .checkbox(
                &mut app.config.cluster_hide_skimmers,
                tr(key::CLUSTER_HIDE_SKIMMERS, lang),
            )
            .changed()
        {
            app.save_station_config();
        }

        ui.separator();
        if ui
            .button(icons::DELETE.label(tr(key::CLUSTER_CLEAR_SPOTS, lang)))
            .clicked()
        {
            app.cluster_spots.clear();
            ui.close();
        }

        // Sekcja radio / CAT / praca
        ui.separator();
        ui.label(
            egui::RichText::new(tr(key::TOOLS_CAT_CONNECTION, lang))
                .strong()
                .color(palette.accent),
        );
        if ui
            .button(icons::RADIO.label(tr(key::TOOLS_CAT_CONNECTION, lang)))
            .clicked()
        {
            app.show_cat_settings_window = true;
            ui.close();
        }
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
            app.cw.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::VOICE_KEYER.label(tr(key::MENU_VOICE_KEYER, lang)))
            .clicked()
        {
            app.voice_keyer.show_window = true;
            ui.close();
        }
        if ui
            .button(icons::DESKTOP.label(tr(key::MENU_WORKSPACE_PROFILES, lang)))
            .clicked()
        {
            app.show_workspace_profiles_window = true;
            ui.close();
        }
        if ui
            .button(icons::AI_ASSISTANT.label(tr(key::MENU_OPERATOR_ASSISTANT, lang)))
            .clicked()
        {
            app.show_operator_assistant = true;
            ui.close();
        }
        if ui
            .button(icons::PLUGIN.label(tr(key::MENU_PLUGIN_MANAGER, lang)))
            .clicked()
        {
            app.plugins_ui.show_manager = true;
            ui.close();
        }
        if ui
            .button(icons::STORE.label(tr(key::MENU_PLUGIN_MARKETPLACE, lang)))
            .clicked()
        {
            app.marketplace.show_window = true;
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
            .button(icons::FLAG.label(tr(key::TOOLS_CONTEST_MODULE, lang)))
            .clicked()
        {
            app.show_contest_window = true;
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr(key::TOOLS_MULTI_OP, lang)))
            .clicked()
        {
            app.show_multi_op_window = true;
            ui.close();
        }
    });
}
