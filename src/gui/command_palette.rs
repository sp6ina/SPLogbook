// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wyszukiwarka poleceń (Command Palette) — szybkie uruchamianie akcji i okien
// przez Ctrl+Shift+P, na wzór edytorów kodu i nowoczesnych aplikacji.

use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Pojedyncza akcja dostępna w palecie poleceń.
struct Command {
    label: &'static str,
    keywords: &'static str,
    run: fn(&mut SpLogApp),
}

fn toggle_awards(app: &mut SpLogApp) { app.show_awards_matrix_window = true; }
fn toggle_stats(app: &mut SpLogApp) { app.show_statistics_window = true; }
fn toggle_world_map(app: &mut SpLogApp) { app.panel_world_map.visible = true; app.show_world_map_window = true; }
fn toggle_satellites(app: &mut SpLogApp) { app.panel_satellites.visible = true; app.show_satellites_window = true; }
fn toggle_solar(app: &mut SpLogApp) { app.panel_solar.visible = true; app.show_solar_panel = true; }
fn toggle_bandmap(app: &mut SpLogApp) { app.panel_bandmap.visible = true; app.show_bandmap_window = true; }
fn toggle_cluster(app: &mut SpLogApp) { app.panel_cluster.visible = true; app.show_cluster_panel = true; }
fn toggle_vfo(app: &mut SpLogApp) { app.panel_vfo.visible = true; app.show_vfo_panel = true; }
fn toggle_qso(app: &mut SpLogApp) { app.panel_qso.visible = true; }
fn toggle_log(app: &mut SpLogApp) { app.panel_log.visible = true; }
fn toggle_contest(app: &mut SpLogApp) { app.show_contest_window = true; }
fn toggle_cw(app: &mut SpLogApp) { app.show_cw_window = true; }
fn toggle_sync(app: &mut SpLogApp) { app.show_online_sync_window = true; }
fn toggle_qsl_designer(app: &mut SpLogApp) { app.qsl_designer_dialog.is_open = true; }
fn toggle_duplicates(app: &mut SpLogApp) { app.show_find_duplicates_window = true; }
fn toggle_profiles(app: &mut SpLogApp) { app.show_station_profiles_window = true; }
fn toggle_columns(app: &mut SpLogApp) { app.show_column_settings = true; }
fn toggle_advanced_filter(app: &mut SpLogApp) { app.advanced_filter_dialog.is_open = true; }
fn toggle_about(app: &mut SpLogApp) { app.show_about_window = true; }
fn toggle_shortcuts(app: &mut SpLogApp) { app.show_shortcuts_window = true; }
fn toggle_legend(app: &mut SpLogApp) { app.show_legend_window = true; }
fn toggle_wizard(app: &mut SpLogApp) { app.show_welcome_wizard = true; app.wizard_tab = 0; }
fn toggle_multi_op(app: &mut SpLogApp) { app.show_multi_op_window = true; }
fn toggle_pskreporter(app: &mut SpLogApp) { app.show_pskreporter_window = true; }
fn toggle_wspr(app: &mut SpLogApp) { app.show_wspr_window = true; }

fn save_qso(app: &mut SpLogApp) { app.save_qso(); }
fn clear_qso(app: &mut SpLogApp) {
    app.clear_qso_form();
    app.panel_qso.visible = true;
    app.focus_callsign_requested = true;
}
fn connect_cluster(app: &mut SpLogApp) { app.connect_dx_cluster(); }
fn disconnect_cluster(app: &mut SpLogApp) { app.disconnect_dx_cluster(); }
fn refresh_log(app: &mut SpLogApp) { app.reload_qsos(); }

fn cycle_theme(app: &mut SpLogApp) {
    let all = crate::gui::theme::ThemePreset::ALL;
    let idx = all.iter().position(|t| t.id() == app.theme_preset.id()).unwrap_or(0);
    app.theme_preset = all[(idx + 1) % all.len()];
    app.dark_theme = app.theme_preset.is_dark();
    app.save_station_config();
}

const COMMANDS: &[Command] = &[
    Command { label: "Nowe QSO (wyczyść formularz)", keywords: "new qso clear form", run: clear_qso },
    Command { label: "Zapisz QSO", keywords: "save qso log", run: save_qso },
    Command { label: "Odśwież dziennik (log)", keywords: "refresh reload log", run: refresh_log },
    Command { label: "Połącz z DX Cluster", keywords: "connect dx cluster telnet", run: connect_cluster },
    Command { label: "Rozłącz DX Cluster", keywords: "disconnect dx cluster", run: disconnect_cluster },
    Command { label: "Macierz nagród (Awards)", keywords: "awards dxcc waz was matrix", run: toggle_awards },
    Command { label: "Statystyki", keywords: "statistics stats charts", run: toggle_stats },
    Command { label: "Mapa świata", keywords: "world map", run: toggle_world_map },
    Command { label: "Satelity", keywords: "satellite pass", run: toggle_satellites },
    Command { label: "Panel słoneczny / propagacja", keywords: "solar propagation sfi", run: toggle_solar },
    Command { label: "Bandmapa (panorama pasma)", keywords: "bandmap spots", run: toggle_bandmap },
    Command { label: "Klaster DX", keywords: "cluster dx spots", run: toggle_cluster },
    Command { label: "Panel VFO / radio", keywords: "vfo radio cat", run: toggle_vfo },
    Command { label: "Formularz QSO", keywords: "qso entry form", run: toggle_qso },
    Command { label: "Tabela logbooka", keywords: "logbook table log", run: toggle_log },
    Command { label: "Moduł kontestowy", keywords: "contest cabrillo", run: toggle_contest },
    Command { label: "Terminal / makra CW", keywords: "cw terminal macros keyer", run: toggle_cw },
    Command { label: "Synchronizacja online", keywords: "online sync lotw eqsl clublog", run: toggle_sync },
    Command { label: "Projektant naklejek QSL", keywords: "qsl label designer print", run: toggle_qsl_designer },
    Command { label: "Znajdź duplikaty", keywords: "duplicates find", run: toggle_duplicates },
    Command { label: "Profile stacji", keywords: "station profiles", run: toggle_profiles },
    Command { label: "Ustawienia kolumn logbooka", keywords: "columns logbook", run: toggle_columns },
    Command { label: "Filtr zaawansowany", keywords: "advanced filter", run: toggle_advanced_filter },
    Command { label: "Multi-op / log sieciowy", keywords: "multi op network", run: toggle_multi_op },
    Command { label: "PSK Reporter", keywords: "psk reporter spots", run: toggle_pskreporter },
    Command { label: "Monitor WSPR", keywords: "wspr monitor", run: toggle_wspr },
    Command { label: "O programie", keywords: "about", run: toggle_about },
    Command { label: "Skróty klawiszowe", keywords: "shortcuts keyboard", run: toggle_shortcuts },
    Command { label: "Legenda kolorów", keywords: "legend colors", run: toggle_legend },
    Command { label: "Kreator konfiguracji", keywords: "wizard setup", run: toggle_wizard },
    Command { label: "Przełącz motyw kolorystyczny", keywords: "theme cycle colors", run: cycle_theme },
];

/// Renderuje okno palety poleceń. Wywoływane co klatkę z `app.update()`.
pub fn render_command_palette(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_command_palette {
        return;
    }

    let lang = app.current_language;
    let mut open = app.show_command_palette;
    let mut run_idx: Option<usize> = None;

    egui::Window::new(egui::RichText::new(format!("🔎 {}", tr("palette.title", lang))).strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
        .default_width(460.0)
        .show(ctx, |ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.command_palette_query)
                    .hint_text(tr("palette.search_hint", lang))
                    .desired_width(f32::INFINITY),
            );
            if resp.changed() {
                app.command_palette_selected = 0;
            }

            ui.separator();

            let query = app.command_palette_query.trim().to_lowercase();
            let mut matches: Vec<usize> = Vec::new();
            for (i, cmd) in COMMANDS.iter().enumerate() {
                if query.is_empty()
                    || cmd.label.to_lowercase().contains(&query)
                    || cmd.keywords.to_lowercase().contains(&query)
                {
                    matches.push(i);
                }
            }

            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                if matches.is_empty() {
                    ui.label(egui::RichText::new(tr("palette.no_results", lang)).color(egui::Color32::from_rgb(148, 163, 184)));
                }
                for (row, &cmd_idx) in matches.iter().enumerate() {
                    let cmd = &COMMANDS[cmd_idx];
                    let selected = row == app.command_palette_selected;
                    let text = if selected {
                        egui::RichText::new(format!("▶ {}", cmd.label)).strong().color(egui::Color32::from_rgb(56, 189, 248))
                    } else {
                        egui::RichText::new(cmd.label).color(egui::Color32::from_rgb(203, 213, 225))
                    };
                    if ui.add_sized([f32::INFINITY, 24.0], egui::Button::new(text)).clicked() {
                        run_idx = Some(cmd_idx);
                    }
                }
            });

            // Nawigacja strzałkami i zatwierdzenie Enterem
            if app.command_palette_selected >= matches.len() {
                app.command_palette_selected = matches.len().saturating_sub(1);
            }
            let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if down && !matches.is_empty() {
                app.command_palette_selected = (app.command_palette_selected + 1).min(matches.len() - 1);
            }
            if up && !matches.is_empty() {
                app.command_palette_selected = app.command_palette_selected.saturating_sub(1);
            }
            if enter && !matches.is_empty() {
                run_idx = matches.get(app.command_palette_selected).copied();
            }
        });

    if let Some(idx) = run_idx {
        (COMMANDS[idx].run)(app);
        app.show_command_palette = false;
        app.command_palette_query.clear();
    }

    app.show_command_palette = open;
}
