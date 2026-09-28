// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Widok (Zarządzanie kafelkami modułów i układem pulpitu)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.menu_button(tr("menu.view", lang), |ui| {
        ui.label(
            egui::RichText::new(tr("view.panels_title", lang))
                .strong()
                .color(egui::Color32::from_rgb(56, 189, 248)),
        );
        ui.separator();

        let mut changed = false;

        if ui
            .checkbox(
                &mut app.panel_vfo.visible,
                icons::RADIO.label(tr("view.panel_vfo", lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_qso.visible,
                icons::NEW_QSO.label(tr("view.panel_qso", lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_log.visible,
                icons::LOGBOOK.label(tr("view.panel_log", lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_cluster.visible,
                icons::CLUSTER.label(tr("view.panel_cluster", lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_bandmap.visible,
                icons::BANDMAP.label(tr("view.panel_bandmap", lang)),
            )
            .changed()
        {
            app.show_bandmap_window = app.panel_bandmap.visible;
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_solar.visible,
                icons::SOLAR.label(tr("view.panel_solar", lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_satellites.visible,
                icons::SATELLITE.label(tr("view.panel_satellites", lang)),
            )
            .changed()
        {
            app.show_satellites_window = app.panel_satellites.visible;
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_world_map.visible,
                icons::WORLD_MAP.label(tr("view.panel_world_map", lang)),
            )
            .changed()
        {
            app.show_world_map_window = app.panel_world_map.visible;
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.panel_waterfall.visible,
                icons::SIGNAL_UP.label("Widmo / Waterfall (SDR)"),
            )
            .changed()
        {
            changed = true;
        }

        if changed {
            app.save_station_config();
        }

        ui.separator();
        if ui
            .button(
                egui::RichText::new(icons::REFRESH.label(tr("view.reset_layout", lang))).strong(),
            )
            .clicked()
        {
            app.reset_panel_layout();
            app.save_station_config();
            ui.close();
        }
        ui.separator();
        ui.checkbox(
            &mut app.show_awards_matrix_window,
            icons::AWARDS.label(tr("view.awards_matrix", lang)),
        );
        if ui
            .checkbox(
                &mut app.compact_hud_mode,
                icons::COMPACT.label(tr("view.compact_hud", lang)),
            )
            .changed()
        {
            app.save_station_config();
        }
    });
}
