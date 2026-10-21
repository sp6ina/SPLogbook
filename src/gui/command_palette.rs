// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wyszukiwarka poleceń (Command Palette) — szybkie uruchamianie akcji i okien
// przez Ctrl+Shift+P, na wzór edytorów kodu i nowoczesnych aplikacji.

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Pojedyncza akcja dostępna w palecie poleceń.
struct Command {
    label_key: crate::core::i18n::Key,
    keywords: &'static str,
    run: fn(&mut SpLogApp),
}

const COMMANDS: &[Command] = &[
    Command {
        label_key: key::CMD_NEW_QSO,
        keywords: "new qso clear form",
        run: |app| {
            app.wipe_qso_form();
        },
    },
    Command {
        label_key: key::CMD_WIPE_QSO,
        keywords: "wipe clear form qso",
        run: |app| {
            app.wipe_qso_form();
        },
    },
    Command {
        label_key: key::CMD_SAVE_QSO,
        keywords: "save qso log",
        run: |app| {
            app.save_qso();
        },
    },
    Command {
        label_key: key::CMD_TOGGLE_PTT,
        keywords: "ptt tx rx transmit",
        run: |app| {
            app.toggle_ptt();
        },
    },
    Command {
        label_key: key::CMD_REFRESH_LOG,
        keywords: "refresh reload log",
        run: |app| {
            app.reload_qsos();
        },
    },
    Command {
        label_key: key::CMD_SEND_SPOT,
        keywords: "spot send dx cluster",
        run: |app| {
            let freq_khz = (app.rig_state.frequency_hz as f64) / 1000.0;
            app.send_spot_dialog
                .open_with(&app.entry_callsign, freq_khz);
        },
    },
    Command {
        label_key: key::CMD_VOICE_KEYER,
        keywords: "voice keyer ssb cq audio",
        run: |app| {
            app.show_voice_keyer_window = true;
        },
    },
    Command {
        label_key: key::CMD_MANUAL,
        keywords: "help manual docs instrukcja",
        run: |app| {
            app.show_user_manual = true;
            app.manual_section = None;
        },
    },
    Command {
        label_key: key::CMD_CONNECT_CLUSTER,
        keywords: "connect dx cluster telnet",
        run: |app| {
            app.connect_dx_cluster();
        },
    },
    Command {
        label_key: key::CMD_DISCONNECT_CLUSTER,
        keywords: "disconnect dx cluster",
        run: |app| {
            app.disconnect_dx_cluster();
        },
    },
    Command {
        label_key: key::CMD_AWARDS_MATRIX,
        keywords: "awards dxcc waz was matrix",
        run: |app| {
            app.show_awards_matrix_window = true;
        },
    },
    Command {
        label_key: key::CMD_STATISTICS,
        keywords: "statistics stats charts",
        run: |app| {
            app.show_statistics_window = true;
        },
    },
    Command {
        label_key: key::CMD_WORLD_MAP,
        keywords: "world map",
        run: |app| {
            app.panel_world_map.visible = true;
            app.show_world_map_window = true;
        },
    },
    Command {
        label_key: key::CMD_WATERFALL,
        keywords: "waterfall spectrum sdr fft",
        run: |app| {
            app.panel_waterfall.visible = true;
        },
    },
    Command {
        label_key: key::CMD_SATELLITES,
        keywords: "satellite pass",
        run: |app| {
            app.panel_satellites.visible = true;
            app.show_satellites_window = true;
        },
    },
    Command {
        label_key: key::CMD_SOLAR_PANEL,
        keywords: "solar propagation sfi",
        run: |app| {
            app.panel_solar.visible = true;
            app.show_solar_panel = true;
        },
    },
    Command {
        label_key: key::CMD_BANDMAP,
        keywords: "bandmap spots",
        run: |app| {
            app.panel_bandmap.visible = true;
            app.show_bandmap_window = true;
        },
    },
    Command {
        label_key: key::CMD_CLUSTER,
        keywords: "cluster dx spots",
        run: |app| {
            app.panel_cluster.visible = true;
            app.show_cluster_panel = true;
        },
    },
    Command {
        label_key: key::CMD_VFO,
        keywords: "vfo radio cat",
        run: |app| {
            app.panel_vfo.visible = true;
            app.show_vfo_panel = true;
        },
    },
    Command {
        label_key: key::CMD_QSO_ENTRY,
        keywords: "qso entry form",
        run: |app| {
            app.panel_qso.visible = true;
        },
    },
    Command {
        label_key: key::CMD_LOGBOOK,
        keywords: "logbook table log",
        run: |app| {
            app.panel_log.visible = true;
        },
    },
    Command {
        label_key: key::CMD_CONTEST_MODULE,
        keywords: "contest cabrillo",
        run: |app| {
            app.show_contest_window = true;
        },
    },
    Command {
        label_key: key::CMD_CW_TERMINAL,
        keywords: "cw terminal macros keyer",
        run: |app| {
            app.show_cw_window = true;
        },
    },
    Command {
        label_key: key::CMD_ONLINE_SYNC,
        keywords: "online sync lotw eqsl clublog",
        run: |app| {
            app.show_online_sync_window = true;
        },
    },
    Command {
        label_key: key::CMD_QSL_DESIGNER,
        keywords: "qsl label designer print",
        run: |app| {
            app.qsl_designer_dialog.is_open = true;
        },
    },
    Command {
        label_key: key::CMD_FIND_DUPLICATES,
        keywords: "duplicates find",
        run: |app| {
            app.duplicates.show_window = true;
        },
    },
    Command {
        label_key: key::CMD_STATION_PROFILES,
        keywords: "station profiles",
        run: |app| {
            app.show_station_profiles_window = true;
        },
    },
    Command {
        label_key: key::CMD_COLUMN_SETTINGS,
        keywords: "columns logbook",
        run: |app| {
            app.show_column_settings = true;
        },
    },
    Command {
        label_key: key::CMD_ADVANCED_FILTER,
        keywords: "advanced filter",
        run: |app| {
            app.advanced_filter_dialog.is_open = true;
        },
    },
    Command {
        label_key: key::CMD_MULTI_OP,
        keywords: "multi op network",
        run: |app| {
            app.show_multi_op_window = true;
        },
    },
    Command {
        label_key: key::CMD_PSK_REPORTER,
        keywords: "psk reporter spots",
        run: |app| {
            app.show_pskreporter_window = true;
        },
    },
    Command {
        label_key: key::CMD_WSPR_MONITOR,
        keywords: "wspr monitor",
        run: |app| {
            app.show_wspr_window = true;
        },
    },
    Command {
        label_key: key::CMD_ABOUT,
        keywords: "about",
        run: |app| {
            app.show_about_window = true;
        },
    },
    Command {
        label_key: key::CMD_SHORTCUTS,
        keywords: "shortcuts keyboard",
        run: |app| {
            app.show_shortcuts_window = true;
        },
    },
    Command {
        label_key: key::CMD_COLOR_LEGEND,
        keywords: "legend colors",
        run: |app| {
            app.show_legend_window = true;
        },
    },
    Command {
        label_key: key::CMD_WIZARD,
        keywords: "wizard setup",
        run: |app| {
            app.show_welcome_wizard = true;
            app.wizard_tab = 0;
        },
    },
    Command {
        label_key: key::CMD_TOGGLE_THEME,
        keywords: "theme cycle colors",
        run: |app| {
            app.cycle_theme();
        },
    },
];

/// Renderuje okno palety poleceń. Wywoływane co klatkę z `app.update()`.
pub fn render_command_palette(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_command_palette {
        return;
    }

    let lang = app.current_language;
    let mut open = app.show_command_palette;
    let mut run_idx: Option<usize> = None;

    egui::Window::new(egui::RichText::new(format!("🔎 {}", tr(key::PALETTE_TITLE, lang))).strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
        .default_width(460.0)
        .show(ctx, |ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.command_palette_query)
                    .hint_text(tr(key::PALETTE_SEARCH_HINT, lang))
                    .desired_width(f32::INFINITY),
            );
            if resp.changed() {
                app.command_palette_selected = 0;
            }

            ui.separator();

            let query = app.command_palette_query.trim().to_lowercase();
            let mut matches: Vec<usize> = Vec::new();
            for (i, cmd) in COMMANDS.iter().enumerate() {
                let lbl = tr(cmd.label_key, lang);
                if query.is_empty()
                    || lbl.to_lowercase().contains(&query)
                    || cmd.keywords.to_lowercase().contains(&query)
                {
                    matches.push(i);
                }
            }

            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    use crate::gui::theme::ThemeExt;
                    let palette = ui.palette();
                    if matches.is_empty() {
                        ui.label(
                            egui::RichText::new(tr(key::PALETTE_NO_RESULTS, lang))
                                .color(palette.text_muted),
                        );
                    }
                    for (row, &cmd_idx) in matches.iter().enumerate() {
                        let cmd = &COMMANDS[cmd_idx];
                        let lbl = tr(cmd.label_key, lang);
                        let selected = row == app.command_palette_selected;
                        let text = if selected {
                            egui::RichText::new(format!("▶ {}", lbl))
                                .strong()
                                .color(palette.accent)
                        } else {
                            egui::RichText::new(lbl).color(palette.text_primary)
                        };
                        if ui
                            .add_sized([f32::INFINITY, 24.0], egui::Button::new(text))
                            .clicked()
                        {
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
                app.command_palette_selected =
                    (app.command_palette_selected + 1).min(matches.len() - 1);
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
