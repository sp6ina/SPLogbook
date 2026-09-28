// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cluster::telnet::CLUSTER_PRESETS;
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Operacja (radio, DX Cluster, CAT, satelity, contesty, multi-op)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr("menu.operation", lang), |ui| {
        // Sekcja DX Cluster
        ui.label(
            egui::RichText::new(icons::CLUSTER.label(tr("menu.dx_cluster", lang)))
                .strong()
                .color(egui::Color32::from_rgb(56, 189, 248)),
        );
        ui.separator();
        if app.cluster_connected {
            if ui
                .button(icons::DOT_RED.label(tr("cluster.disconnect_btn", lang)))
                .clicked()
            {
                app.disconnect_dx_cluster();
                ui.close();
            }
        } else {
            let txt = if app.cluster_connecting {
                format!("● {}", tr("cluster.connecting_btn", lang))
            } else {
                format!("● {}", tr("cluster.connect_btn", lang))
            };
            if ui.button(txt).clicked() {
                app.connect_dx_cluster();
                ui.close();
            }
        }

        if ui
            .button(icons::SPOT.label(tr("cluster.announce_spot", lang)))
            .clicked()
        {
            let freq_khz = app.rig_state.frequency_hz as f64 / 1000.0;
            app.send_spot_dialog
                .open_with(&app.entry_callsign, freq_khz);
            ui.close();
        }

        ui.separator();
        ui.label(egui::RichText::new(tr("cluster.quick_server", lang)).strong());
        for (name, host, port) in CLUSTER_PRESETS {
            let is_current = app.cluster_host == *host && app.cluster_port == *port;
            if ui.selectable_label(is_current, *name).clicked() {
                app.cluster_host = host.to_string();
                app.cluster_port = *port;
                app.save_station_config();
                if app.cluster_connected {
                    app.connect_dx_cluster();
                }
                ui.close();
            }
        }

        ui.separator();
        ui.label(egui::RichText::new(tr("cluster.filters_options", lang)).strong());
        if ui
            .checkbox(
                &mut app.cluster_auto_connect,
                tr("cluster.auto_connect_startup", lang),
            )
            .changed()
        {
            app.save_station_config();
        }
        if ui
            .checkbox(
                &mut app.cluster_filter_current_band,
                tr("cluster.filter_vfo_band", lang),
            )
            .changed()
        {
            app.save_station_config();
        }
        if ui
            .checkbox(&mut app.cluster_hide_ft8, tr("cluster.hide_ft8", lang))
            .changed()
        {
            app.save_station_config();
        }
        if ui
            .checkbox(
                &mut app.cluster_hide_skimmers,
                tr("cluster.hide_skimmers", lang),
            )
            .changed()
        {
            app.save_station_config();
        }

        ui.separator();
        if ui
            .button(icons::DELETE.label(tr("cluster.clear_spots", lang)))
            .clicked()
        {
            app.cluster_spots.clear();
            ui.close();
        }

        // Sekcja radio / CAT / praca
        ui.separator();
        ui.label(
            egui::RichText::new(tr("tools.cat_connection", lang))
                .strong()
                .color(egui::Color32::from_rgb(56, 189, 248)),
        );
        if ui
            .button(icons::RADIO.label(tr("tools.cat_connection", lang)))
            .clicked()
        {
            app.show_cat_settings_window = true;
            ui.close();
        }
        if ui
            .button(icons::CW_TERMINAL.label(tr("tools.cw_terminal", lang)))
            .clicked()
        {
            app.cw_terminal_dialog.is_open = true;
            ui.close();
        }
        if ui
            .button(icons::CW_KEYER.label(tr("tools.cw_macros", lang)))
            .clicked()
        {
            app.show_cw_window = true;
            ui.close();
        }
        if ui
            .button(icons::VOICE_KEYER.label("Voice Keyer (SSB)"))
            .clicked()
        {
            app.show_voice_keyer_window = true;
            ui.close();
        }
        if ui
            .button(icons::DESKTOP.label("Profile układu (workspace)"))
            .clicked()
        {
            app.show_workspace_profiles_window = true;
            ui.close();
        }
        if ui
            .button(icons::AI_ASSISTANT.label("Asystent operatora"))
            .clicked()
        {
            app.show_operator_assistant = true;
            ui.close();
        }
        if ui
            .button(icons::PLUGIN.label("Menedżer pluginów (Rhai)"))
            .clicked()
        {
            app.show_plugin_manager = true;
            ui.close();
        }
        if ui
            .button(icons::STORE.label("Marketplace pluginów"))
            .clicked()
        {
            app.show_marketplace = true;
            ui.close();
        }
        if ui
            .button(icons::SATELLITE.label(tr("tools.satellites", lang)))
            .clicked()
        {
            app.panel_satellites.visible = true;
            app.panel_satellites.floating = true;
            app.show_satellites_window = true;
            ui.close();
        }
        if ui
            .button(icons::FLAG.label(tr("tools.contest_module", lang)))
            .clicked()
        {
            app.show_contest_window = true;
            ui.close();
        }
        if ui
            .button(icons::GLOBE.label(tr("tools.multi_op", lang)))
            .clicked()
        {
            app.show_multi_op_window = true;
            ui.close();
        }
    });
}
