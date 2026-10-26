// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Menu: Widok (Zarządzanie kafelkami modułów i układem pulpitu)
pub(super) fn render(app: &mut SpLogApp, ui: &mut egui::Ui) {
    use crate::gui::theme::ThemeExt;
    let palette = ui.palette();
    let lang = app.gui.appearance.language;

    ui.menu_button(tr(key::MENU_VIEW, lang), |ui| {
        ui.label(
            egui::RichText::new(tr(key::VIEW_PANELS_TITLE, lang))
                .strong()
                .color(palette.accent),
        );
        ui.separator();

        let mut changed = false;

        if ui
            .checkbox(
                &mut app.views.panel_vfo.visible,
                icons::RADIO.label(tr(key::VIEW_PANEL_VFO, lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_qso.visible,
                icons::NEW_QSO.label(tr(key::VIEW_PANEL_QSO, lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_log.visible,
                icons::LOGBOOK.label(tr(key::VIEW_PANEL_LOG, lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_cluster.visible,
                icons::CLUSTER.label(tr(key::VIEW_PANEL_CLUSTER, lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_bandmap.visible,
                icons::BANDMAP.label(tr(key::VIEW_PANEL_BANDMAP, lang)),
            )
            .changed()
        {
            app.gui.windows.show_bandmap_window = app.views.panel_bandmap.visible;
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_solar.visible,
                icons::SOLAR.label(tr(key::VIEW_PANEL_SOLAR, lang)),
            )
            .changed()
        {
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_satellites.visible,
                icons::SATELLITE.label(tr(key::VIEW_PANEL_SATELLITES, lang)),
            )
            .changed()
        {
            app.satellites.show_window = app.views.panel_satellites.visible;
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_world_map.visible,
                icons::WORLD_MAP.label(tr(key::VIEW_PANEL_WORLD_MAP, lang)),
            )
            .changed()
        {
            app.world_map.show_window = app.views.panel_world_map.visible;
            changed = true;
        }
        if ui
            .checkbox(
                &mut app.views.panel_waterfall.visible,
                icons::SIGNAL_UP.label(tr(key::VIEW_PANEL_WATERFALL, lang)),
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
                egui::RichText::new(icons::REFRESH.label(tr(key::VIEW_RESET_LAYOUT, lang)))
                    .strong(),
            )
            .clicked()
        {
            app.reset_panel_layout();
            app.save_station_config();
            ui.close();
        }
        ui.separator();
        ui.checkbox(
            &mut app.gui.windows.show_awards_matrix_window,
            icons::AWARDS.label(tr(key::VIEW_AWARDS_MATRIX, lang)),
        );
        if ui
            .checkbox(
                &mut app.config.compact_hud_mode,
                icons::COMPACT.label(tr(key::VIEW_COMPACT_HUD, lang)),
            )
            .changed()
        {
            app.save_station_config();
        }
    });
}
