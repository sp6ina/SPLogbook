// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wyszukiwarka poleceń (Command Palette) — szybkie uruchamianie akcji i okien
// przez Ctrl+Shift+P, na wzór edytorów kodu i nowoczesnych aplikacji.

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Pojedyncza akcja dostępna w palecie poleceń.
pub(crate) struct Command {
    pub(crate) label_key: crate::core::i18n::Key,
    pub(crate) keywords: &'static str,
    pub(crate) run: fn(&mut SpLogApp),
}

pub(crate) const COMMANDS: &[Command] = &[
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
        label_key: key::CMD_PTT_ON,
        keywords: "ptt tx on transmit nadawanie",
        run: |app| {
            let _ = app.dispatch_ptt(
                crate::gui::ptt_input::PttActionSource::CommandPalette,
                crate::gui::ptt_input::PttIntent::RequestOn,
            );
        },
    },
    Command {
        label_key: key::CMD_PTT_OFF,
        keywords: "ptt rx off receive odbior",
        run: |app| {
            let _ = app.dispatch_ptt(
                crate::gui::ptt_input::PttActionSource::CommandPalette,
                crate::gui::ptt_input::PttIntent::RequestOff,
            );
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
            let freq_khz = (app.cat.rig_state.frequency_hz as f64) / 1000.0;
            app.gui
                .dialogs
                .send_spot_dialog
                .open_with(&app.entry.callsign, freq_khz);
        },
    },
    Command {
        label_key: key::CMD_VOICE_KEYER,
        keywords: "voice keyer ssb cq audio",
        run: |app| {
            app.voice_keyer.show_window = true;
        },
    },
    Command {
        label_key: key::CMD_MANUAL,
        keywords: "help manual docs instrukcja",
        run: |app| {
            app.gui.chrome.show_user_manual = true;
            app.gui.chrome.manual_section = None;
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
            app.open_tile(crate::gui::state::Tile::Awards);
        },
    },
    Command {
        label_key: key::CMD_STATISTICS,
        keywords: "statistics stats charts",
        run: |app| {
            app.open_tile(crate::gui::state::Tile::Statistics);
        },
    },
    Command {
        label_key: key::CMD_WORLD_MAP,
        keywords: "world map",
        run: |app| {
            app.views.panel_world_map.visible = true;
            app.world_map.show_window = true;
        },
    },
    Command {
        label_key: key::CMD_WATERFALL,
        keywords: "waterfall spectrum sdr fft",
        run: |app| {
            app.views.panel_waterfall.visible = true;
        },
    },
    Command {
        label_key: key::CMD_SATELLITES,
        keywords: "satellite pass",
        run: |app| {
            app.views.panel_satellites.visible = true;
            app.satellites.show_window = true;
        },
    },
    Command {
        label_key: key::CMD_SOLAR_PANEL,
        keywords: "solar propagation sfi",
        run: |app| {
            app.views.panel_solar.visible = true;
            app.solar.show_panel = true;
        },
    },
    Command {
        label_key: key::CMD_BANDMAP,
        keywords: "bandmap spots",
        run: |app| {
            app.views.panel_bandmap.visible = true;
            app.gui.windows.show_bandmap_window = true;
        },
    },
    Command {
        label_key: key::CMD_CLUSTER,
        keywords: "cluster dx spots",
        run: |app| {
            app.views.panel_cluster.visible = true;
            app.gui.windows.show_cluster_panel = true;
        },
    },
    Command {
        label_key: key::CMD_VFO,
        keywords: "vfo radio cat",
        run: |app| {
            app.views.panel_vfo.visible = true;
            app.gui.windows.show_vfo_panel = true;
        },
    },
    Command {
        label_key: key::CMD_QSO_ENTRY,
        keywords: "qso entry form",
        run: |app| {
            app.views.panel_qso.visible = true;
        },
    },
    Command {
        label_key: key::CMD_LOGBOOK,
        keywords: "logbook table log",
        run: |app| {
            app.views.panel_log.visible = true;
        },
    },
    Command {
        label_key: key::CMD_CONTEST_MODULE,
        keywords: "contest cabrillo",
        run: |app| {
            app.open_tile(crate::gui::state::Tile::Contest);
        },
    },
    Command {
        label_key: key::CMD_CW_TERMINAL,
        keywords: "cw terminal macros keyer",
        run: |app| {
            app.open_tile(crate::gui::state::Tile::CwMacros);
        },
    },
    Command {
        label_key: key::CMD_ONLINE_SYNC,
        keywords: "online sync lotw eqsl clublog",
        run: |app| {
            app.online.show_online_sync_window = true;
        },
    },
    Command {
        label_key: key::CMD_QSL_DESIGNER,
        keywords: "qsl label designer print",
        run: |app| {
            app.gui.dialogs.qsl_designer_dialog.is_open = true;
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
            app.gui.windows.show_station_profiles_window = true;
        },
    },
    Command {
        label_key: key::CMD_COLUMN_SETTINGS,
        keywords: "columns logbook",
        run: |app| {
            app.gui.windows.show_column_settings = true;
        },
    },
    Command {
        label_key: key::CMD_ADVANCED_FILTER,
        keywords: "advanced filter",
        run: |app| {
            app.gui.dialogs.advanced_filter_dialog.is_open = true;
        },
    },
    Command {
        label_key: key::CMD_MULTI_OP,
        keywords: "multi op network",
        run: |app| {
            app.gui.windows.show_multi_op_window = true;
        },
    },
    Command {
        label_key: key::CMD_PSK_REPORTER,
        keywords: "psk reporter spots",
        run: |app| {
            app.digital.show_pskreporter_window = true;
        },
    },
    Command {
        label_key: key::CMD_WSPR_MONITOR,
        keywords: "wspr monitor",
        run: |app| {
            app.wspr.show_window = true;
        },
    },
    Command {
        label_key: key::CMD_ABOUT,
        keywords: "about",
        run: |app| {
            app.gui.chrome.show_about_window = true;
        },
    },
    Command {
        label_key: key::CMD_SHORTCUTS,
        keywords: "shortcuts keyboard",
        run: |app| {
            app.gui.chrome.show_shortcuts_window = true;
        },
    },
    Command {
        label_key: key::CMD_COLOR_LEGEND,
        keywords: "legend colors",
        run: |app| {
            app.gui.chrome.show_legend_window = true;
        },
    },
    Command {
        label_key: key::THEME_CUSTOM_MENU,
        keywords: "theme accent colour color density personalize",
        run: |app| {
            let accent = app.gui.appearance.theme_preset.palette().accent;
            app.open_theme_customizer([accent.r(), accent.g(), accent.b()]);
        },
    },
    Command {
        label_key: key::STATUSBAR_CUSTOMIZE_MENU,
        keywords: "status bar items order",
        run: |app| {
            app.gui.windows.show_status_bar_settings = true;
        },
    },
    Command {
        label_key: key::WORKSPACE_LOCK,
        keywords: "lock layout workspace unlock",
        run: |app| {
            let locked = app.layout_locked();
            app.set_layout_locked(!locked);
        },
    },
    Command {
        label_key: key::BANDPLAN_MENU,
        keywords: "band plan segments bandplan",
        run: |app| {
            app.open_bandplan_editor();
        },
    },
    Command {
        label_key: key::CMD_WIZARD,
        keywords: "wizard setup",
        run: |app| {
            app.gui.chrome.show_welcome_wizard = true;
            app.gui.chrome.wizard_tab = 0;
        },
    },
    Command {
        label_key: key::CMD_TOGGLE_THEME,
        keywords: "theme cycle colors",
        run: |app| {
            app.cycle_theme();
        },
    },
    Command {
        label_key: key::QSO_CLEAR,
        keywords: "clear qso form wipe",
        run: |app| {
            app.clear_qso_form();
        },
    },
    Command {
        label_key: key::QSO_DELETE,
        keywords: "delete qso remove",
        run: |app| {
            app.delete_selected_qso();
        },
    },
    Command {
        label_key: key::MENU_JOURNAL_MGMT,
        keywords: "journal logbook manage",
        run: |app| {
            if let Ok(db) = app.services.log_db.lock() {
                app.gui.dialogs.journal_dialog.reload(&db);
            }
            app.gui.dialogs.journal_dialog.is_open = true;
        },
    },
    Command {
        label_key: key::MENU_IMPORT_ADIF,
        keywords: "import adif adi",
        run: |app| {
            app.trigger_import_adif();
        },
    },
    Command {
        label_key: key::MENU_EXPORT_ADIF,
        keywords: "export adif adi",
        run: |app| {
            app.trigger_export_adif();
        },
    },
    Command {
        label_key: key::MENU_EXPORT_ADX,
        keywords: "export adx xml",
        run: |app| {
            app.trigger_export_adx();
        },
    },
    Command {
        label_key: key::MENU_EXPORT_CSV,
        keywords: "export csv spreadsheet",
        run: |app| {
            app.gui.dialogs.csv_export_dialog.open();
        },
    },
    Command {
        label_key: key::MENU_EXPORT_SOTA,
        keywords: "export sota summit csv",
        run: |app| {
            let call = app.config.station.callsign.clone();
            app.gui.dialogs.sota_dialog.open(&call);
        },
    },
    Command {
        label_key: key::MENU_EXPORT_PDF,
        keywords: "export pdf print",
        run: |app| {
            app.export_pdf_log();
        },
    },
    Command {
        label_key: key::MENU_EXPORT_GPX,
        keywords: "export gpx track map",
        run: |app| {
            app.export_gpx_log();
        },
    },
    Command {
        label_key: key::MENU_BACKUP,
        keywords: "backup copy save",
        run: |app| {
            app.run_manual_backup();
        },
    },
    Command {
        label_key: key::TOOLS_CAT_CONNECTION,
        keywords: "cat hamlib rig radio connection",
        run: |app| {
            app.gui.windows.show_cat_settings_window = true;
        },
    },
    Command {
        label_key: key::TOOLS_CW_TERMINAL,
        keywords: "cw terminal keyer morse",
        run: |app| {
            app.gui.dialogs.cw_terminal_dialog.is_open = true;
        },
    },
    Command {
        label_key: key::REF_WOL,
        keywords: "wake on lan wol power",
        run: |app| {
            app.gui.dialogs.wol_dialog.open();
        },
    },
    Command {
        label_key: key::REF_IOTA_DB,
        keywords: "iota islands",
        run: |app| {
            app.gui.dialogs.iota_dialog.open();
        },
    },
    Command {
        label_key: key::REF_STATES_DB,
        keywords: "states provinces",
        run: |app| {
            app.gui.dialogs.states_dialog.open();
        },
    },
    Command {
        label_key: key::REF_QSL_MANAGERS,
        keywords: "qsl managers via",
        run: |app| {
            app.gui.dialogs.qsl_manager_dialog.open();
        },
    },
    Command {
        label_key: key::REF_PREFIX_MANAGER,
        keywords: "prefix dxcc",
        run: |app| {
            app.gui.dialogs.prefix_manager_dialog.open();
        },
    },
    Command {
        label_key: key::REF_CLUBS_EDITOR,
        keywords: "clubs membership",
        run: |app| {
            app.open_clubs_editor();
        },
    },
    Command {
        label_key: key::REF_UPDATE_ONLINE_DBS,
        keywords: "update databases online",
        run: |app| {
            app.trigger_database_update();
        },
    },
    Command {
        label_key: key::REF_CF_INSTALL,
        keywords: "country files cty download",
        run: |app| {
            app.trigger_country_files_download();
        },
    },
    Command {
        label_key: key::REF_MOON_SUN,
        keywords: "moon sun astronomy eme",
        run: |app| {
            app.gui.dialogs.astronomy_dialog.open();
        },
    },
    Command {
        label_key: key::MENU_OPERATOR_ASSISTANT,
        keywords: "assistant operator ai",
        run: |app| {
            app.gui.windows.show_operator_assistant = true;
        },
    },
    Command {
        label_key: key::MENU_PLUGIN_MANAGER,
        keywords: "plugins rhai scripts",
        run: |app| {
            app.plugins_ui.show_manager = true;
        },
    },
    Command {
        label_key: key::MENU_PLUGIN_MARKETPLACE,
        keywords: "plugins marketplace store",
        run: |app| {
            app.marketplace.show_window = true;
        },
    },
    Command {
        label_key: key::CONTEST_CUSTOM_EDITOR_TITLE,
        keywords: "contest custom editor",
        run: |app| {
            app.contest.show_custom_editor = true;
        },
    },
    Command {
        label_key: key::CONTEST_EXPORT_CABRILLO,
        keywords: "cabrillo contest export",
        run: |app| {
            app.export_cabrillo();
        },
    },
    Command {
        label_key: key::WORKSPACE_MANAGE_PROFILES,
        keywords: "workspace layout profiles",
        run: |app| {
            app.gui.windows.show_workspace_profiles_window = true;
        },
    },
    Command {
        label_key: key::HELP_CHANGELOG,
        keywords: "changelog changes news",
        run: |app| {
            app.updates.show_changelog_window = true;
        },
    },
    Command {
        label_key: key::HELP_CHECK_UPDATES,
        keywords: "update check version",
        run: |app| {
            app.updates.show_update_window = true;
        },
    },
];

/// Renderuje okno palety poleceń. Wywoływane co klatkę z `app.update()`.
pub fn render_command_palette(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.gui.chrome.show_command_palette {
        return;
    }

    let lang = app.gui.appearance.language;
    let mut open = app.gui.chrome.show_command_palette;
    let mut run_idx: Option<usize> = None;

    egui::Window::new(egui::RichText::new(format!("🔎 {}", tr(key::PALETTE_TITLE, lang))).strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
        .default_width(460.0)
        .show(ctx, |ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.gui.chrome.command_palette_query)
                    .hint_text(tr(key::PALETTE_SEARCH_HINT, lang))
                    .desired_width(f32::INFINITY),
            );
            if resp.changed() {
                app.gui.chrome.command_palette_selected = 0;
            }

            ui.separator();

            let query = app.gui.chrome.command_palette_query.trim().to_lowercase();
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
                        let selected = row == app.gui.chrome.command_palette_selected;
                        let text = if selected {
                            egui::RichText::new(format!("▶ {lbl}"))
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
            if app.gui.chrome.command_palette_selected >= matches.len() {
                app.gui.chrome.command_palette_selected = matches.len().saturating_sub(1);
            }
            let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if down && !matches.is_empty() {
                app.gui.chrome.command_palette_selected =
                    (app.gui.chrome.command_palette_selected + 1).min(matches.len() - 1);
            }
            if up && !matches.is_empty() {
                app.gui.chrome.command_palette_selected =
                    app.gui.chrome.command_palette_selected.saturating_sub(1);
            }
            if enter && !matches.is_empty() {
                run_idx = matches
                    .get(app.gui.chrome.command_palette_selected)
                    .copied();
            }
        });

    if let Some(idx) = run_idx {
        (COMMANDS[idx].run)(app);
        app.gui.chrome.show_command_palette = false;
        app.gui.chrome.command_palette_query.clear();
    }

    app.gui.chrome.show_command_palette = open;
}
