// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::*;
use eframe::egui;

/// Pomocnik sprawdzający czy wciśnięto klawisz modyfikatora Ctrl (Windows/Linux) lub Command (macOS).
#[inline]
pub fn is_ctrl_modifier(modifiers: &egui::Modifiers) -> bool {
    modifiers.ctrl || modifiers.command
}

impl SpLogApp {
    /// Faza obsługi skrótów klawiaturowych, palety poleceń i wejść globalnych.
    pub fn handle_input_and_shortcuts(&mut self, ctx: &egui::Context) {
        // Globalne skróty klawiszowe
        // Sprawdzanie modyfikatora: Ctrl (Windows/Linux) oraz Command (macOS)
        let is_ctrl = |m: &egui::Modifiers| is_ctrl_modifier(m);

        // Klawisze funkcyjne (F1–F12) — działają globalnie, niezależnie od fokusu pól tekstowych
        let open_shortcuts_f1 = ctx.input(|i| i.key_pressed(egui::Key::F1));
        let save_qso_f2 = ctx.input(|i| i.key_pressed(egui::Key::F2));
        let wipe_qso_f3 = ctx.input(|i| i.key_pressed(egui::Key::F3));
        let lookup_qrz_f4 = ctx.input(|i| i.key_pressed(egui::Key::F4));
        let refresh_log_f5 = ctx.input(|i| i.key_pressed(egui::Key::F5));
        let send_spot_f6 = ctx.input(|i| i.key_pressed(egui::Key::F6));
        let toggle_ptt_f7 = ctx.input(|i| i.key_pressed(egui::Key::F7));
        let voice_keyer_f8 = ctx.input(|i| i.key_pressed(egui::Key::F8));
        let toggle_fullscreen_f11 = ctx.input(|i| i.key_pressed(egui::Key::F11));
        let open_manual_f12 = ctx.input(|i| i.key_pressed(egui::Key::F12));

        // Skróty z klawiszem Ctrl / Cmd
        let do_undo = ctx
            .input(|i| i.key_pressed(egui::Key::Z) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let do_redo = ctx.input(|i| {
            (i.key_pressed(egui::Key::Z) && is_ctrl(&i.modifiers) && i.modifiers.shift)
                || (i.key_pressed(egui::Key::Y) && is_ctrl(&i.modifiers))
        });
        let open_stats = ctx
            .input(|i| i.key_pressed(egui::Key::S) && is_ctrl(&i.modifiers) && i.modifiers.shift);
        let save_now = ctx
            .input(|i| i.key_pressed(egui::Key::S) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let open_palette = ctx
            .input(|i| i.key_pressed(egui::Key::P) && is_ctrl(&i.modifiers) && i.modifiers.shift);
        let open_manual_ctrl_h = ctx
            .input(|i| i.key_pressed(egui::Key::H) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let quit_app_ctrl_q = ctx
            .input(|i| i.key_pressed(egui::Key::Q) && is_ctrl(&i.modifiers) && !i.modifiers.shift);

        // Skróty edycyjne i nawigacyjne (tylko gdy użytkownik nie pisze w polu tekstowym)
        let wants_text = ctx.egui_wants_keyboard_input();
        let focus_filter = ctx
            .input(|i| i.key_pressed(egui::Key::F) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let new_qso = ctx
            .input(|i| i.key_pressed(egui::Key::N) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let wipe_qso_ctrl_w = ctx
            .input(|i| i.key_pressed(egui::Key::W) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let toggle_cluster = ctx
            .input(|i| i.key_pressed(egui::Key::D) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let toggle_bandmap = ctx
            .input(|i| i.key_pressed(egui::Key::B) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let toggle_map = ctx
            .input(|i| i.key_pressed(egui::Key::M) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let toggle_cw = ctx
            .input(|i| i.key_pressed(egui::Key::K) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let open_profiles = ctx
            .input(|i| i.key_pressed(egui::Key::P) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let export_csv = ctx
            .input(|i| i.key_pressed(egui::Key::E) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let import_adif = ctx
            .input(|i| i.key_pressed(egui::Key::I) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let cycle_theme_ctrl_t = ctx
            .input(|i| i.key_pressed(egui::Key::T) && is_ctrl(&i.modifiers) && !i.modifiers.shift);
        let focus_log_ctrl_l = ctx
            .input(|i| i.key_pressed(egui::Key::L) && is_ctrl(&i.modifiers) && !i.modifiers.shift);

        // Wykonanie akcji
        if do_undo {
            self.perform_undo();
        }
        if do_redo {
            self.perform_redo();
        }
        if open_stats {
            self.show_statistics_window = true;
        }
        if open_shortcuts_f1 {
            self.show_shortcuts_window = !self.show_shortcuts_window;
        }
        if open_manual_f12 || open_manual_ctrl_h {
            self.show_user_manual = !self.show_user_manual;
            self.manual_section = None;
        }
        if save_qso_f2 {
            self.save_qso();
        }
        if wipe_qso_f3 {
            self.wipe_qso_form();
        }
        if lookup_qrz_f4 {
            let clean = self.entry_callsign.trim().to_uppercase();
            if clean.len() >= 3 {
                self.lookup_active_callsign_online();
            }
        }
        if refresh_log_f5 {
            self.reload_qsos();
        }
        if send_spot_f6 {
            let freq_khz = (self.rig_state.frequency_hz as f64) / 1000.0;
            self.send_spot_dialog
                .open_with(&self.entry_callsign, freq_khz);
        }
        if toggle_ptt_f7 {
            self.toggle_ptt();
        }
        if voice_keyer_f8 {
            self.voice_keyer.show_window = !self.voice_keyer.show_window;
        }
        if toggle_fullscreen_f11 {
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
        }
        if save_now {
            self.save_station_config();
        }
        if open_palette {
            self.show_command_palette = !self.show_command_palette;
            self.command_palette_query.clear();
            self.command_palette_selected = 0;
        }
        if quit_app_ctrl_q {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if !wants_text {
            if focus_filter {
                self.advanced_filter_dialog.is_open = true;
            }
            if new_qso || wipe_qso_ctrl_w {
                self.wipe_qso_form();
            }
            if toggle_cluster {
                self.panel_cluster.visible = true;
                self.show_cluster_panel = true;
            }
            if toggle_bandmap {
                self.panel_bandmap.visible = true;
                self.show_bandmap_window = true;
            }
            if toggle_map {
                self.panel_world_map.visible = true;
                self.world_map.show_window = true;
            }
            if toggle_cw {
                self.cw.show_window = !self.cw.show_window;
            }
            if open_profiles {
                self.show_station_profiles_window = !self.show_station_profiles_window;
            }
            if export_csv {
                self.csv_export_dialog.open();
            }
            if import_adif {
                self.trigger_import_adif();
            }
            if cycle_theme_ctrl_t {
                self.cycle_theme();
            }
            if focus_log_ctrl_l {
                self.panel_log.visible = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
