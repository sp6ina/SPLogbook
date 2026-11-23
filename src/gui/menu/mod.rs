// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Główny pasek menu, pasek narzędziowy i menu szybkiego dostępu.
//! Logika poszczególnych grup menu jest podzielona na podmoduły.

mod contest;
mod digital;
mod help;
mod log;
mod radio;
mod references;
mod settings;
mod spots;
mod tools;
mod view;
mod workspace;

use crate::core::i18n::{key, tr};
use crate::core::station::{QuickAccessConfig, ToolbarItem};
use crate::gui::app::SpLogApp;
use crate::gui::icons;
use crate::gui::state::Tile;
use eframe::egui;

/// Główny pasek menu (Menu Bar) z rozwijanymi kategoriami
pub fn render_menu_bar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        log::render(app, ui);
        radio::render(app, ui);
        spots::render(app, ui);
        contest::render(app, ui);
        digital::render(app, ui);
        view::render(app, ui);
        tools::render(app, ui);
        settings::render(app, ui);
        help::render(app, ui);
        render_right_side(app, ui);
    });
}

/// Prawa strona paska menu: wyłącznie znak OP, aktywny profil stacji i zegar UTC
/// (nie koliduje z lewym menu).
fn render_right_side(app: &mut SpLogApp, ui: &mut egui::Ui) {
    use crate::gui::theme::ThemeExt;
    let palette = ui.palette();
    let lang = app.gui.appearance.language;

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let utc_now = jiff::Timestamp::now().strftime("%H:%M:%S UTC").to_string();
        ui.label(
            egui::RichText::new(icons::TIMER.label(utc_now))
                .color(palette.text_muted)
                .monospace()
                .strong(),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "OP: {} ({})",
                app.config.station.callsign, app.config.station.gridsquare
            ))
            .color(palette.accent)
            .strong(),
        );
        ui.separator();
        let profile_label = if app.config.station.name.is_empty() {
            icons::BADGE.label(&app.config.station.callsign)
        } else {
            icons::BADGE.label(&app.config.station.name)
        };
        if ui
            .button(
                egui::RichText::new(profile_label)
                    .color(palette.warning)
                    .size(11.0),
            )
            .on_hover_text(tr(key::PROFILES_TITLE, lang))
            .clicked()
        {
            app.gui.windows.show_station_profiles_window = true;
        }
    });
}

/// Ikona i nazwa elementu paska (do przycisku i do okna personalizacji).
fn toolbar_item_text(app: &SpLogApp, item: ToolbarItem) -> (String, &'static str) {
    let lang = app.gui.appearance.language;
    let (icon, label): (icons::Icon, &str) = match item {
        ToolbarItem::Journal => (icons::FOLDER, tr(key::TOOLBAR_JOURNAL, lang)),
        ToolbarItem::Wsjtx => (icons::DOT_GREEN, tr(key::TOOLBAR_WSJTX_STATUS, lang)),
        ToolbarItem::ClusterStatus => (icons::CLUSTER, tr(key::TOOLBAR_CLUSTER_STATUS, lang)),
        ToolbarItem::Vfo => (icons::RADIO, tr(key::TOOLBAR_VFO, lang)),
        ToolbarItem::Qso => (icons::NEW_QSO, tr(key::TOOLBAR_QSO, lang)),
        ToolbarItem::Log => (icons::LOGBOOK, tr(key::TOOLBAR_LOG, lang)),
        ToolbarItem::Cat => (icons::CAT_LINK, tr(key::TOOLBAR_CAT, lang)),
        ToolbarItem::Cluster => (icons::CLUSTER, tr(key::TOOLBAR_CLUSTER, lang)),
        ToolbarItem::Bandmap => (icons::BANDMAP, tr(key::TOOLBAR_BANDMAP, lang)),
        ToolbarItem::Solar => (icons::SOLAR, tr(key::TOOLBAR_SOLAR, lang)),
        ToolbarItem::Lotw => (icons::GLOBE, tr(key::TOOLBAR_LOTW, lang)),
        ToolbarItem::Map => (icons::WORLD_MAP, tr(key::TOOLBAR_MAP, lang)),
        ToolbarItem::Awards => (icons::AWARDS, tr(key::TOOLBAR_AWARDS, lang)),
        ToolbarItem::Cw => (icons::CW_TERMINAL, tr(key::TOOLBAR_CW, lang)),
        ToolbarItem::Satellites => (icons::SATELLITE, tr(key::TOOLBAR_SATELLITES, lang)),
        ToolbarItem::Contest => (icons::FLAG, tr(key::TOOLBAR_CONTEST, lang)),
        ToolbarItem::Equipment => (icons::PACKAGE, tr(key::TOOLBAR_EQUIPMENT, lang)),
        ToolbarItem::Eme => (icons::MOON, "EME"),
        ToolbarItem::Wol => (icons::LIGHTNING, "WOL"),
        ToolbarItem::Theme => (icons::THEME, app.gui.appearance.theme_preset.label(lang)),
        ToolbarItem::Lock => (icons::PIN, tr(key::TOOLBAR_LOCK, lang)),
    };
    (icon.as_str().to_string(), label)
}

/// Podpowiedź przycisku paska.
fn toolbar_item_tooltip(item: ToolbarItem, lang: crate::core::i18n::Language) -> &'static str {
    let k = match item {
        ToolbarItem::Journal => key::TOOLBAR_JOURNAL_TOOLTIP,
        ToolbarItem::Wsjtx => key::WSJTX_OPEN_WINDOW,
        ToolbarItem::ClusterStatus => key::TOOLBAR_CLUSTER_TOGGLE_TOOLTIP,
        ToolbarItem::Vfo => key::TOOLBAR_VFO_TOOLTIP,
        ToolbarItem::Qso => key::TOOLBAR_QSO_TOOLTIP,
        ToolbarItem::Log => key::TOOLBAR_LOG_TOOLTIP,
        ToolbarItem::Cat => key::TOOLBAR_CAT_TOOLTIP,
        ToolbarItem::Cluster => key::TOOLBAR_CLUSTER_TOOLTIP,
        ToolbarItem::Bandmap => key::TOOLBAR_BANDMAP_TOOLTIP,
        ToolbarItem::Solar => key::TOOLBAR_SOLAR_TOOLTIP,
        ToolbarItem::Lotw => key::TOOLBAR_SYNC_TOOLTIP,
        ToolbarItem::Map => key::TOOLBAR_MAP_TOOLTIP,
        ToolbarItem::Awards => key::TOOLBAR_AWARDS_TOOLTIP,
        ToolbarItem::Cw => key::TOOLBAR_CW_TOOLTIP,
        ToolbarItem::Satellites => key::TOOLBAR_SATELLITES_TOOLTIP,
        ToolbarItem::Contest => key::TOOLBAR_CONTEST_TOOLTIP,
        ToolbarItem::Equipment => key::TOOLBAR_EQUIPMENT_TOOLTIP,
        ToolbarItem::Eme => key::TOOLBAR_EME_TOOLTIP,
        ToolbarItem::Wol => key::TOOLBAR_WOL_TOOLTIP,
        ToolbarItem::Theme => key::TOOLBAR_THEME_CYCLE_HINT,
        ToolbarItem::Lock => key::WORKSPACE_LOCK_HINT,
    };
    tr(k, lang)
}

/// Pasek narzędzi: elementy w kolejności użytkownika, opcjonalnie same ikony.
pub fn render_main_toolbar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;
    let mut show_customize_popup = false;

    let toolbar_resp = ui.horizontal_wrapped(|ui| {
        use crate::gui::theme::ThemeExt;
        let palette = ui.palette();

        // Wskaźnik aktywnego filtra dziennika pojawia się zawsze, gdy filtr działa.
        if app.gui.dialogs.advanced_filter_dialog.is_filtered_active
            && ui
                .button(
                    egui::RichText::new(icons::SEARCH.label(tr(key::TOOLBAR_FILTER, lang)))
                        .strong()
                        .color(palette.warning),
                )
                .clicked()
        {
            app.gui.dialogs.advanced_filter_dialog.is_open = true;
        }

        let mut previous_status = None;
        for item in app.config.quick_access.ordered() {
            if !app.config.quick_access.is_visible(item) {
                continue;
            }
            if previous_status == Some(true) && !item.is_status() {
                ui.separator();
            }
            previous_status = Some(item.is_status());
            render_toolbar_item(app, ui, item, &palette);
        }

        if ui
            .button(icons::SETTINGS.rich(12.0))
            .on_hover_text(tr(key::TOOLBAR_CUSTOMIZE_TOOLTIP, lang))
            .clicked()
        {
            show_customize_popup = true;
        }
    });

    toolbar_resp.response.context_menu(|ui| {
        render_quick_access_menu(app, ui);
    });

    if show_customize_popup {
        app.gui.windows.show_quick_access_customizer = true;
    }

    if app.gui.windows.show_quick_access_customizer {
        let mut open = true;
        egui::Window::new(icons::SETTINGS.label(tr(key::TOOLBAR_CUSTOMIZE_TITLE, lang)))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(380.0)
            .show(ui.ctx(), |ui| {
                render_quick_access_menu(app, ui);
            });
        if !open {
            app.gui.windows.show_quick_access_customizer = false;
        }
    }
}

fn render_toolbar_item(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    item: ToolbarItem,
    palette: &crate::gui::theme::Palette,
) {
    let lang = app.gui.appearance.language;
    let tooltip = toolbar_item_tooltip(item, lang);
    match item {
        ToolbarItem::Journal => {
            if ui
                .button(
                    egui::RichText::new(icons::FOLDER.label(format!(
                        "{}: {} ({})",
                        tr(key::TOOLBAR_JOURNAL, lang),
                        app.active_journal.name,
                        app.active_journal.station_callsign
                    )))
                    .strong()
                    .color(palette.success),
                )
                .on_hover_text(tooltip)
                .clicked()
            {
                if let Ok(db) = app.services.log_db.lock() {
                    app.gui.dialogs.journal_dialog.reload(&db);
                }
                app.gui.dialogs.journal_dialog.is_open = true;
            }
        }
        ToolbarItem::Wsjtx => {
            let color = if app.digital.wsjtx.is_listening() {
                palette.success
            } else {
                palette.text_muted
            };
            let indicator = ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(format!(
                            "● WSJT-X ({})",
                            app.digital.wsjtx_packets_count
                        ))
                        .small()
                        .color(color),
                    )
                    .sense(egui::Sense::click()),
                )
                .on_hover_text(tooltip);
            if indicator.clicked() {
                app.open_tile(Tile::Wsjtx);
            }
        }
        ToolbarItem::ClusterStatus => {
            let (color, text) = if app.cluster.connected {
                (
                    palette.success,
                    format!("● Cluster ({})", app.config.cluster_host),
                )
            } else if app.cluster.connecting {
                (
                    palette.warning,
                    format!("● Cluster: {}", tr(key::TOOLBAR_CLUSTER_CONNECTING, lang)),
                )
            } else {
                (
                    palette.text_muted,
                    format!("○ Cluster: {}", tr(key::TOOLBAR_CLUSTER_DISCONNECTED, lang)),
                )
            };
            if ui
                .button(egui::RichText::new(text).small().color(color))
                .on_hover_text(tooltip)
                .clicked()
            {
                if app.cluster.connected {
                    app.disconnect_dx_cluster();
                } else {
                    app.connect_dx_cluster();
                }
            }
        }
        _ => {
            let (icon, label) = toolbar_item_text(app, item);
            let icon = if item == ToolbarItem::Lock {
                if app.layout_locked() { "🔒" } else { "🔓" }.to_string()
            } else {
                icon
            };
            let (text, hover) = if app.config.quick_access.icons_only {
                (icon, format!("{label}\n{tooltip}"))
            } else {
                (format!("{icon} {label}"), tooltip.to_string())
            };
            if ui.button(text).on_hover_text(hover).clicked() {
                run_toolbar_button(app, item);
            }
        }
    }
}

/// Działanie przycisku paska (bez wskaźników stanu).
fn run_toolbar_button(app: &mut SpLogApp, item: ToolbarItem) {
    match item {
        ToolbarItem::Vfo => {
            app.views.panel_vfo.visible = !app.views.panel_vfo.visible;
            app.save_station_config();
        }
        ToolbarItem::Qso => {
            app.views.panel_qso.visible = !app.views.panel_qso.visible;
            app.save_station_config();
        }
        ToolbarItem::Log => {
            app.views.panel_log.visible = !app.views.panel_log.visible;
            app.save_station_config();
        }
        ToolbarItem::Cat => {
            app.gui.windows.show_cat_settings_window = !app.gui.windows.show_cat_settings_window;
        }
        ToolbarItem::Cluster => {
            app.views.panel_cluster.visible = !app.views.panel_cluster.visible;
            app.save_station_config();
        }
        ToolbarItem::Bandmap => {
            app.views.panel_bandmap.visible = !app.views.panel_bandmap.visible;
            app.gui.windows.show_bandmap_window = app.views.panel_bandmap.visible;
            app.save_station_config();
        }
        ToolbarItem::Solar => {
            app.views.panel_solar.visible = !app.views.panel_solar.visible;
            app.save_station_config();
        }
        ToolbarItem::Lotw => {
            app.online.show_online_sync_window = !app.online.show_online_sync_window;
        }
        ToolbarItem::Map => {
            app.views.panel_world_map.visible = !app.views.panel_world_map.visible;
            app.world_map.show_window = app.views.panel_world_map.visible;
            app.save_station_config();
        }
        ToolbarItem::Awards => app.toggle_tile(Tile::Awards),
        ToolbarItem::Cw => {
            app.gui.dialogs.cw_terminal_dialog.is_open =
                !app.gui.dialogs.cw_terminal_dialog.is_open;
        }
        ToolbarItem::Satellites => {
            app.views.panel_satellites.visible = !app.views.panel_satellites.visible;
            app.satellites.show_window = app.views.panel_satellites.visible;
            app.save_station_config();
        }
        ToolbarItem::Contest => app.toggle_tile(Tile::Contest),
        ToolbarItem::Equipment => app.toggle_tile(Tile::Ledger),
        ToolbarItem::Eme => app.gui.dialogs.astronomy_dialog.open(),
        ToolbarItem::Wol => app.gui.dialogs.wol_dialog.open(),
        ToolbarItem::Theme => {
            app.cycle_theme();
        }
        ToolbarItem::Lock => {
            let locked = app.layout_locked();
            app.set_layout_locked(!locked);
        }
        ToolbarItem::Journal | ToolbarItem::Wsjtx | ToolbarItem::ClusterStatus => {}
    }
}

/// Personalizacja paska: widoczność, kolejność i tryb samych ikon.
fn render_quick_access_menu(app: &mut SpLogApp, ui: &mut egui::Ui) {
    use crate::gui::theme::ThemeExt;
    let palette = ui.palette();
    let lang = app.gui.appearance.language;
    ui.heading(tr(key::TOOLBAR_VISIBLE_ELEMENTS, lang));
    ui.label(
        egui::RichText::new(tr(key::TOOLBAR_VISIBLE_ELEMENTS_HINT, lang))
            .size(11.0)
            .color(palette.text_muted),
    );
    ui.separator();

    let mut changed = false;
    changed |= ui
        .checkbox(
            &mut app.config.quick_access.icons_only,
            tr(key::TOOLBAR_ICONS_ONLY, lang),
        )
        .on_hover_text(tr(key::TOOLBAR_ICONS_ONLY_HINT, lang))
        .changed();
    ui.separator();

    let items = app.config.quick_access.ordered();
    let last = items.len().saturating_sub(1);
    let mut moved = None;
    egui::ScrollArea::vertical()
        .id_salt("qa_customizer_items")
        .max_height(360.0)
        .show(ui, |ui| {
            egui::Grid::new("qa_customizer_grid")
                .num_columns(3)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for (index, item) in items.iter().copied().enumerate() {
                        let (icon, label) = toolbar_item_text(app, item);
                        changed |= ui
                            .checkbox(
                                app.config.quick_access.visibility_mut(item),
                                format!("{icon} {label}"),
                            )
                            .changed();
                        if ui
                            .add_enabled(index > 0, egui::Button::new("▲").small())
                            .on_hover_text(tr(key::TOOLBAR_MOVE_LEFT, lang))
                            .clicked()
                        {
                            moved = Some((item, -1));
                        }
                        if ui
                            .add_enabled(index < last, egui::Button::new("▼").small())
                            .on_hover_text(tr(key::TOOLBAR_MOVE_RIGHT, lang))
                            .clicked()
                        {
                            moved = Some((item, 1));
                        }
                        ui.end_row();
                    }
                });
        });
    if let Some((item, step)) = moved {
        app.config.quick_access.move_item(item, step);
        changed = true;
    }

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button(tr(key::TOOLBAR_RESTORE_DEFAULTS, lang)).clicked() {
            app.config.quick_access = QuickAccessConfig::default();
            changed = true;
        }
        if ui.button(tr(key::TOOLBAR_MINIMAL_TOOLBAR, lang)).clicked() {
            let current = &app.config.quick_access;
            let mut minimal = QuickAccessConfig::minimal();
            minimal.order.clone_from(&current.order);
            minimal.icons_only = current.icons_only;
            app.config.quick_access = minimal;
            changed = true;
        }
    });

    if changed {
        app.save_station_config();
    }
}
