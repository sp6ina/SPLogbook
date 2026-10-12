// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Główny pasek menu, pasek narzędziowy i menu szybkiego dostępu.
//! Logika poszczególnych grup menu jest podzielona na podmoduły.

mod edit;
mod file;
mod help;
mod operation;
mod references;
mod settings;
mod tools;
mod view;
mod workspace;

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use eframe::egui;

/// Główny pasek menu (Menu Bar) z rozwijanymi kategoriami
pub fn render_menu_bar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        file::render(app, ui);
        edit::render(app, ui);
        view::render(app, ui);
        operation::render(app, ui);
        workspace::render(app, ui);
        references::render(app, ui);
        tools::render(app, ui);
        settings::render(app, ui);
        help::render(app, ui);
        render_right_side(app, ui);
    });
}

/// Prawa strona paska menu: wyłącznie znak OP, aktywny profil stacji i zegar UTC
/// (nie koliduje z lewym menu).
fn render_right_side(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let utc_now = chrono::Utc::now().format("%H:%M:%S UTC").to_string();
        ui.label(
            egui::RichText::new(icons::TIMER.label(utc_now))
                .color(egui::Color32::from_rgb(148, 163, 184))
                .monospace()
                .strong(),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "OP: {} ({})",
                app.my_station.callsign, app.my_station.gridsquare
            ))
            .color(egui::Color32::from_rgb(56, 189, 248))
            .strong(),
        );
        ui.separator();
        let profile_label = if app.my_station.name.is_empty() {
            icons::BADGE.label(&app.my_station.callsign)
        } else {
            icons::BADGE.label(&app.my_station.name)
        };
        if ui
            .button(
                egui::RichText::new(profile_label)
                    .color(egui::Color32::from_rgb(250, 204, 21))
                    .size(11.0),
            )
            .on_hover_text(tr(key::PROFILES_TITLE, lang))
            .clicked()
        {
            app.show_station_profiles_window = true;
        }
    });
}

/// Drugi rząd paska narzędziowego z automatycznym zawijaniem (horizontal_wrapped) i pełną personalizacją (PPM / ikona zębatki)
pub fn render_main_toolbar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    let mut show_customize_popup = false;

    let toolbar_resp = ui.horizontal_wrapped(|ui| {
        // 1. Aktywny dziennik
        if app.quick_access.show_journal
            && ui
                .button(
                    egui::RichText::new(icons::FOLDER.label(format!(
                        "{}: {} ({})",
                        tr(key::TOOLBAR_JOURNAL, lang),
                        app.active_journal.name,
                        app.active_journal.station_callsign
                    )))
                    .strong()
                    .color(egui::Color32::from_rgb(100, 220, 100)),
                )
                .on_hover_text(tr(key::TOOLBAR_JOURNAL_TOOLTIP, lang))
                .clicked()
        {
            if let Ok(db) = app.log_db.lock() {
                app.journal_dialog.reload(&db);
            }
            app.journal_dialog.is_open = true;
        }

        // 2. Sygnalizator WSJT-X
        if app.quick_access.show_wsjtx {
            ui.label(
                egui::RichText::new(format!("● WSJT-X ({})", app.wsjtx_packets_count))
                    .small()
                    .color(egui::Color32::from_rgb(52, 211, 153)),
            );
        }

        // 3. Sygnalizator aktywnego filtra
        if app.advanced_filter_dialog.is_filtered_active
            && ui
                .button(
                    egui::RichText::new(icons::SEARCH.label(tr(key::TOOLBAR_FILTER, lang)))
                        .strong()
                        .color(egui::Color32::YELLOW),
                )
                .clicked()
        {
            app.advanced_filter_dialog.is_open = true;
        }

        // 4. Status DX Cluster
        if app.quick_access.show_cluster_status {
            let (cl_color, cl_text) = if app.cluster_connected {
                (
                    egui::Color32::from_rgb(34, 197, 94),
                    format!("● Cluster ({})", app.cluster_host),
                )
            } else if app.cluster_connecting {
                (
                    egui::Color32::from_rgb(250, 204, 21),
                    format!("● Cluster: {}", tr(key::TOOLBAR_CLUSTER_CONNECTING, lang)),
                )
            } else {
                (
                    egui::Color32::from_rgb(148, 163, 184),
                    format!("○ Cluster: {}", tr(key::TOOLBAR_CLUSTER_DISCONNECTED, lang)),
                )
            };

            if ui
                .button(egui::RichText::new(cl_text).small().color(cl_color))
                .on_hover_text(tr(key::TOOLBAR_CLUSTER_TOGGLE_TOOLTIP, lang))
                .clicked()
            {
                if app.cluster_connected {
                    app.disconnect_dx_cluster();
                } else {
                    app.connect_dx_cluster();
                }
            }
        }

        ui.separator();

        // 5. Szybkie przyciski modułów
        if app.quick_access.show_vfo
            && ui
                .button(icons::RADIO.label(tr(key::TOOLBAR_VFO, lang)))
                .on_hover_text(tr(key::TOOLBAR_VFO_TOOLTIP, lang))
                .clicked()
        {
            app.panel_vfo.visible = !app.panel_vfo.visible;
            app.save_station_config();
        }

        if app.quick_access.show_qso
            && ui
                .button(icons::NEW_QSO.label(tr(key::TOOLBAR_QSO, lang)))
                .on_hover_text(tr(key::TOOLBAR_QSO_TOOLTIP, lang))
                .clicked()
        {
            app.panel_qso.visible = !app.panel_qso.visible;
            app.save_station_config();
        }

        if app.quick_access.show_log
            && ui
                .button(icons::LOGBOOK.label(tr(key::TOOLBAR_LOG, lang)))
                .on_hover_text(tr(key::TOOLBAR_LOG_TOOLTIP, lang))
                .clicked()
        {
            app.panel_log.visible = !app.panel_log.visible;
            app.save_station_config();
        }

        if app.quick_access.show_cat
            && ui
                .button(icons::RADIO.label(tr(key::TOOLBAR_CAT, lang)))
                .on_hover_text(tr(key::TOOLBAR_CAT_TOOLTIP, lang))
                .clicked()
        {
            app.show_cat_settings_window = !app.show_cat_settings_window;
        }

        if app.quick_access.show_cluster
            && ui
                .button(icons::CLUSTER.label(tr(key::TOOLBAR_CLUSTER, lang)))
                .on_hover_text(tr(key::TOOLBAR_CLUSTER_TOOLTIP, lang))
                .clicked()
        {
            app.panel_cluster.visible = !app.panel_cluster.visible;
            app.save_station_config();
        }

        if app.quick_access.show_bandmap
            && ui
                .button(icons::BANDMAP.label(tr(key::TOOLBAR_BANDMAP, lang)))
                .on_hover_text(tr(key::TOOLBAR_BANDMAP_TOOLTIP, lang))
                .clicked()
        {
            app.panel_bandmap.visible = !app.panel_bandmap.visible;
            app.show_bandmap_window = app.panel_bandmap.visible;
            app.save_station_config();
        }

        if app.quick_access.show_solar
            && ui
                .button(icons::SOLAR.label(tr(key::TOOLBAR_SOLAR, lang)))
                .on_hover_text(tr(key::TOOLBAR_SOLAR_TOOLTIP, lang))
                .clicked()
        {
            app.panel_solar.visible = !app.panel_solar.visible;
            app.save_station_config();
        }

        if app.quick_access.show_lotw
            && ui
                .button(icons::GLOBE.label(tr(key::TOOLBAR_LOTW, lang)))
                .on_hover_text(tr(key::TOOLBAR_SYNC_TOOLTIP, lang))
                .clicked()
        {
            app.show_online_sync_window = !app.show_online_sync_window;
        }

        if app.quick_access.show_map
            && ui
                .button(icons::WORLD_MAP.label(tr(key::TOOLBAR_MAP, lang)))
                .on_hover_text(tr(key::TOOLBAR_MAP_TOOLTIP, lang))
                .clicked()
        {
            app.panel_world_map.visible = !app.panel_world_map.visible;
            app.show_world_map_window = app.panel_world_map.visible;
            app.save_station_config();
        }

        if app.quick_access.show_awards
            && ui
                .button(icons::AWARDS.label(tr(key::TOOLBAR_AWARDS, lang)))
                .on_hover_text(tr(key::TOOLBAR_AWARDS_TOOLTIP, lang))
                .clicked()
        {
            app.show_awards_matrix_window = !app.show_awards_matrix_window;
        }

        if app.quick_access.show_cw
            && ui
                .button(icons::CW_TERMINAL.label(tr(key::TOOLBAR_CW, lang)))
                .on_hover_text(tr(key::TOOLBAR_CW_TOOLTIP, lang))
                .clicked()
        {
            app.cw_terminal_dialog.is_open = !app.cw_terminal_dialog.is_open;
        }

        if app.quick_access.show_satellites
            && ui
                .button(icons::SATELLITE.label(tr(key::TOOLBAR_SATELLITES, lang)))
                .on_hover_text(tr(key::TOOLBAR_SATELLITES_TOOLTIP, lang))
                .clicked()
        {
            app.panel_satellites.visible = !app.panel_satellites.visible;
            app.show_satellites_window = app.panel_satellites.visible;
            app.save_station_config();
        }

        if app.quick_access.show_contest
            && ui
                .button(icons::FLAG.label(tr(key::TOOLBAR_CONTEST, lang)))
                .on_hover_text(tr(key::TOOLBAR_CONTEST_TOOLTIP, lang))
                .clicked()
        {
            app.show_contest_window = !app.show_contest_window;
        }

        if app.quick_access.show_equipment
            && ui
                .button(icons::PACKAGE.label(tr(key::TOOLBAR_EQUIPMENT, lang)))
                .on_hover_text(tr(key::TOOLBAR_EQUIPMENT_TOOLTIP, lang))
                .clicked()
        {
            app.show_ledger_window = !app.show_ledger_window;
        }

        if app.quick_access.show_eme
            && ui
                .button(icons::MOON.label("EME"))
                .on_hover_text(tr(key::TOOLBAR_EME_TOOLTIP, lang))
                .clicked()
        {
            app.astronomy_dialog.open();
        }

        if app.quick_access.show_wol
            && ui
                .button(icons::LIGHTNING.label("WOL"))
                .on_hover_text(tr(key::TOOLBAR_WOL_TOOLTIP, lang))
                .clicked()
        {
            app.wol_dialog.open();
        }

        if app.quick_access.show_theme {
            let theme_btn_text = icons::THEME.label(app.theme_preset.label(lang));
            if ui
                .button(theme_btn_text)
                .on_hover_text(tr(key::TOOLBAR_THEME_CYCLE_HINT, lang))
                .clicked()
            {
                let all = crate::gui::theme::ThemePreset::ALL;
                let idx = all.iter().position(|p| *p == app.theme_preset).unwrap_or(0);
                app.theme_preset = all[(idx + 1) % all.len()];
                app.save_station_config();
            }
        }

        // Przycisk personalizacji paska (oprócz PPM)
        if ui
            .button(icons::SETTINGS.rich(12.0))
            .on_hover_text(tr(key::TOOLBAR_CUSTOMIZE_TOOLTIP, lang))
            .clicked()
        {
            show_customize_popup = true;
        }
    });

    // Menu kontekstowe pod prawym przyciskiem myszy na pasku narzędziowym
    toolbar_resp.response.context_menu(|ui| {
        render_quick_access_menu(app, ui);
    });

    if show_customize_popup {
        app.show_quick_access_customizer = true;
    }

    if app.show_quick_access_customizer {
        let mut open = true;
        egui::Window::new(icons::SETTINGS.label(tr(key::TOOLBAR_CUSTOMIZE_TITLE, lang)))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(360.0)
            .show(ui.ctx(), |ui| {
                render_quick_access_menu(app, ui);
            });
        if !open {
            app.show_quick_access_customizer = false;
        }
    }
}

fn render_quick_access_menu(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.heading(tr(key::TOOLBAR_VISIBLE_ELEMENTS, lang));
    ui.label(
        egui::RichText::new(tr(key::TOOLBAR_VISIBLE_ELEMENTS_HINT, lang))
            .size(11.0)
            .color(egui::Color32::from_rgb(148, 163, 184)),
    );
    ui.separator();

    let mut changed = false;
    egui::Grid::new("qa_customizer_grid")
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_journal,
                    icons::FOLDER.label(tr(key::TOOLBAR_JOURNAL, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_wsjtx,
                    format!("● {}", tr(key::TOOLBAR_WSJTX_STATUS, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_cluster_status,
                    icons::CLUSTER.label(tr(key::TOOLBAR_CLUSTER_STATUS, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_vfo,
                    icons::RADIO.label(tr(key::TOOLBAR_VFO, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_qso,
                    icons::NEW_QSO.label(tr(key::TOOLBAR_QSO, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_log,
                    icons::LOGBOOK.label(tr(key::TOOLBAR_LOG, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_cat,
                    icons::RADIO.label(tr(key::TOOLBAR_CAT, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_cluster,
                    icons::CLUSTER.label(tr(key::TOOLBAR_CLUSTER, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_bandmap,
                    icons::BANDMAP.label(tr(key::TOOLBAR_BANDMAP, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_solar,
                    icons::SOLAR.label(tr(key::TOOLBAR_SOLAR, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_lotw,
                    icons::GLOBE.label(tr(key::TOOLBAR_LOTW, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_map,
                    icons::WORLD_MAP.label(tr(key::TOOLBAR_MAP, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_awards,
                    icons::AWARDS.label(tr(key::TOOLBAR_AWARDS, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_cw,
                    icons::CW_TERMINAL.label(tr(key::TOOLBAR_CW, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_satellites,
                    icons::SATELLITE.label(tr(key::TOOLBAR_SATELLITES, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_contest,
                    icons::FLAG.label(tr(key::TOOLBAR_CONTEST, lang)),
                )
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_equipment,
                    icons::PACKAGE.label(tr(key::TOOLBAR_EQUIPMENT, lang)),
                )
                .changed();
            changed |= ui
                .checkbox(&mut app.quick_access.show_eme, icons::MOON.label("EME"))
                .changed();
            ui.end_row();

            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_wol,
                    icons::LIGHTNING.label("WOL"),
                )
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.quick_access.show_theme,
                    format!(
                        "{}/{} {}",
                        icons::MOON.as_str(),
                        icons::SOLAR.as_str(),
                        tr(key::THEME_MENU, lang)
                    ),
                )
                .changed();
            ui.end_row();
        });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(tr(key::TOOLBAR_RESTORE_DEFAULTS, lang)).clicked() {
            app.quick_access = crate::core::station::QuickAccessConfig::default();
            changed = true;
        }
        if ui.button(tr(key::TOOLBAR_MINIMAL_TOOLBAR, lang)).clicked() {
            app.quick_access.show_journal = true;
            app.quick_access.show_wsjtx = true;
            app.quick_access.show_cluster_status = true;
            app.quick_access.show_vfo = true;
            app.quick_access.show_qso = true;
            app.quick_access.show_log = true;
            app.quick_access.show_cat = false;
            app.quick_access.show_cluster = false;
            app.quick_access.show_bandmap = false;
            app.quick_access.show_solar = false;
            app.quick_access.show_lotw = false;
            app.quick_access.show_map = false;
            app.quick_access.show_awards = false;
            app.quick_access.show_cw = false;
            app.quick_access.show_satellites = false;
            app.quick_access.show_contest = false;
            app.quick_access.show_equipment = false;
            app.quick_access.show_eme = false;
            app.quick_access.show_wol = false;
            app.quick_access.show_theme = true;
            changed = true;
        }
    });

    if changed {
        app.save_station_config();
    }
}
