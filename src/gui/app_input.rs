// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::*;
use crate::gui::shortcuts::{AppAction, ShortcutContext, ShortcutRegistry};
use eframe::egui;

impl SpLogApp {
    /// Faza obsługi skrótów klawiaturowych, palety poleceń i wejść globalnych (GUI-001).
    pub fn handle_input_and_shortcuts(&mut self, ctx: &egui::Context) {
        let registry = ShortcutRegistry::default();
        let wants_text = ctx.egui_wants_keyboard_input();

        let current_context = if self.contest.show_window {
            ShortcutContext::Contest
        } else if self.gui.chrome.show_command_palette
            || self.gui.dialogs.advanced_filter_dialog.is_open
            || self.gui.dialogs.send_spot_dialog.is_open
            || self.gui.chrome.show_shortcuts_window
            || self.gui.chrome.show_user_manual
        {
            ShortcutContext::ModalDialog
        } else if wants_text {
            ShortcutContext::QsoEntry
        } else {
            ShortcutContext::Global
        };

        if let Some(action) = registry.resolve(ctx, current_context, wants_text) {
            match action {
                AppAction::Undo => self.perform_undo(),
                AppAction::Redo => self.perform_redo(),
                AppAction::OpenStats => self.log_view.show_statistics_window = true,
                AppAction::ToggleShortcutsHelp => {
                    self.gui.chrome.show_shortcuts_window = !self.gui.chrome.show_shortcuts_window;
                }
                AppAction::SaveQso => self.save_qso(),
                AppAction::WipeQso | AppAction::NewQso => self.wipe_qso_form(),
                AppAction::LookupQrz => {
                    let clean = self.entry.callsign.trim().to_uppercase();
                    if clean.len() >= 3 {
                        self.lookup_active_callsign_online();
                    }
                }
                AppAction::RefreshLog => self.reload_qsos(),
                AppAction::SendSpot => {
                    let freq_khz = (self.cat.rig_state.frequency_hz as f64) / 1000.0;
                    self.gui
                        .dialogs
                        .send_spot_dialog
                        .open_with(&self.entry.callsign, freq_khz);
                }
                AppAction::TogglePtt => self.toggle_ptt(),
                AppAction::ToggleVoiceKeyer => {
                    self.voice_keyer.show_window = !self.voice_keyer.show_window;
                }
                AppAction::ToggleFullscreen => {
                    self.gui.appearance.is_fullscreen = !self.gui.appearance.is_fullscreen;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                        self.gui.appearance.is_fullscreen,
                    ));
                }
                AppAction::OpenManual => {
                    self.gui.chrome.show_user_manual = !self.gui.chrome.show_user_manual;
                    self.gui.chrome.manual_section = None;
                }
                AppAction::OpenCommandPalette => {
                    self.gui.chrome.show_command_palette = !self.gui.chrome.show_command_palette;
                    self.gui.chrome.command_palette_query.clear();
                    self.gui.chrome.command_palette_selected = 0;
                }
                AppAction::SaveConfig => self.save_station_config(),
                AppAction::QuitApp => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                AppAction::FocusFilter => {
                    self.gui.dialogs.advanced_filter_dialog.is_open = true;
                }
                AppAction::ToggleCluster => {
                    self.views.panel_cluster.visible = true;
                    self.gui.windows.show_cluster_panel = true;
                }
                AppAction::ToggleBandmap => {
                    self.views.panel_bandmap.visible = true;
                    self.gui.windows.show_bandmap_window = true;
                }
                AppAction::ToggleWorldMap => {
                    self.views.panel_world_map.visible = true;
                    self.world_map.show_window = true;
                }
                AppAction::ToggleCwTerminal => {
                    self.cw.show_window = !self.cw.show_window;
                }
                AppAction::OpenProfiles => {
                    self.gui.windows.show_station_profiles_window =
                        !self.gui.windows.show_station_profiles_window;
                }
                AppAction::ExportCsv => self.gui.dialogs.csv_export_dialog.open(),
                AppAction::ImportAdif => self.trigger_import_adif(),
                AppAction::CycleTheme => self.cycle_theme(),
                AppAction::FocusLog => self.views.panel_log.visible = true,
                AppAction::ContestSaveQso => {}
                AppAction::ContestWipeQso => {
                    self.entry.callsign.clear();
                    self.entry.rst_rcvd.clear();
                    self.entry.exchange.clear();
                }
                AppAction::EscapeCloseContext => {
                    if self.gui.chrome.show_command_palette {
                        self.gui.chrome.show_command_palette = false;
                    } else if self.gui.dialogs.advanced_filter_dialog.is_open {
                        self.gui.dialogs.advanced_filter_dialog.is_open = false;
                    } else if self.gui.dialogs.send_spot_dialog.is_open {
                        self.gui.dialogs.send_spot_dialog.is_open = false;
                    } else if self.gui.chrome.show_shortcuts_window {
                        self.gui.chrome.show_shortcuts_window = false;
                    } else if self.gui.chrome.show_user_manual {
                        self.gui.chrome.show_user_manual = false;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::shortcuts::is_ctrl_modifier;

    #[test]
    fn test_is_ctrl_modifier() {
        let empty = egui::Modifiers::NONE;
        assert!(!is_ctrl_modifier(&empty));

        let ctrl_only = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        assert!(is_ctrl_modifier(&ctrl_only));

        let mac_cmd = egui::Modifiers {
            command: true,
            ..Default::default()
        };
        assert!(is_ctrl_modifier(&mac_cmd));

        let alt_only = egui::Modifiers {
            alt: true,
            ..Default::default()
        };
        assert!(!is_ctrl_modifier(&alt_only));
    }
}
