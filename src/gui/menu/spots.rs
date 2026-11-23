// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cluster::telnet::CLUSTER_PRESETS;
use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Spoty DX (połączenie z klastrem, ogłaszanie spotu, serwery, filtry, mapa)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_SPOTS, lang), |ui| {
        if app.cluster.connected {
            if ui
                .button(icons::DOT_RED.label(tr(key::CLUSTER_DISCONNECT_BTN, lang)))
                .clicked()
            {
                app.disconnect_dx_cluster();
                ui.close();
            }
        } else {
            let label = if app.cluster.connecting {
                tr(key::CLUSTER_CONNECTING_BTN, lang)
            } else {
                tr(key::CLUSTER_CONNECT_BTN, lang)
            };
            if ui.button(icons::DOT_GREEN.label(label)).clicked() {
                app.connect_dx_cluster();
                ui.close();
            }
        }
        if ui
            .button(icons::SPOT.label(tr(key::CLUSTER_ANNOUNCE_SPOT, lang)))
            .clicked()
        {
            let freq_khz = app.cat.rig_state.frequency_hz as f64 / 1000.0;
            app.gui
                .dialogs
                .send_spot_dialog
                .open_with(&app.entry.callsign, freq_khz);
            ui.close();
        }
        if ui
            .button(icons::WORLD_MAP.label(tr(key::TOOLS_WORLD_MAP, lang)))
            .clicked()
        {
            app.views.panel_world_map.visible = true;
            app.views.panel_world_map.floating = true;
            app.world_map.show_window = true;
            ui.close();
        }

        ui.separator();
        ui.menu_button(
            icons::CLUSTER.label(tr(key::CLUSTER_QUICK_SERVER, lang)),
            |ui| {
                for (name, host, port) in CLUSTER_PRESETS {
                    let is_current =
                        app.config.cluster_host == *host && app.config.cluster_port == *port;
                    if ui.selectable_label(is_current, *name).clicked() {
                        app.config.cluster_host = host.to_string();
                        app.config.cluster_port = *port;
                        app.save_station_config();
                        if app.cluster.connected {
                            app.connect_dx_cluster();
                        }
                        ui.close();
                    }
                }
            },
        );
        ui.menu_button(
            icons::FILTER.label(tr(key::CLUSTER_FILTERS_OPTIONS, lang)),
            |ui| {
                let mut changed = false;
                changed |= ui
                    .checkbox(
                        &mut app.config.cluster_auto_connect,
                        tr(key::CLUSTER_AUTO_CONNECT_STARTUP, lang),
                    )
                    .changed();
                changed |= ui
                    .checkbox(
                        &mut app.config.cluster_filter_current_band,
                        tr(key::CLUSTER_FILTER_VFO_BAND, lang),
                    )
                    .changed();
                changed |= ui
                    .checkbox(
                        &mut app.config.cluster_hide_ft8,
                        tr(key::CLUSTER_HIDE_FT8, lang),
                    )
                    .changed();
                changed |= ui
                    .checkbox(
                        &mut app.config.cluster_hide_skimmers,
                        tr(key::CLUSTER_HIDE_SKIMMERS, lang),
                    )
                    .changed();
                if changed {
                    app.save_station_config();
                }
            },
        );

        ui.separator();
        if ui
            .button(icons::DELETE.label(tr(key::CLUSTER_CLEAR_SPOTS, lang)))
            .clicked()
        {
            app.cluster.spots.clear();
            app.clear_sdr_spots();
            ui.close();
        }
    });
}
