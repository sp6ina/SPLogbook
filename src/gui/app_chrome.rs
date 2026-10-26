// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::*;
use crate::core::i18n::{key, tr};
use crate::gui::theme::ThemeExt;
use eframe::egui;

impl SpLogApp {
    pub fn render_chrome_bars(&mut self, ui: &mut egui::Ui) {
        // Górny pasek menu systemowego oraz pasek szybkiego dostępu (bez kolizji!)
        egui::Panel::top("top_menu_bar").show(ui, |ui| {
            render_menu_bar(self, ui);
            ui.separator();
            render_main_toolbar(self, ui);
        });

        // Dolny pasek statusu stacji
        egui::Panel::bottom("bottom_status_bar").show(ui, |ui| {
            let lang = self.gui.appearance.language;
            let palette = ui.palette();
            ui.horizontal(|ui| {
                let fallback_status = tr(key::STATUS_READY_ALL_ACTIVE, lang);
                let status_txt = self
                    .gui
                    .chrome
                    .status_message
                    .as_deref()
                    .unwrap_or(fallback_status);
                ui.label(
                    egui::RichText::new(status_txt)
                        .size(11.0)
                        .color(palette.text_muted),
                );

                ui.separator();

                // Pasek operacyjny: częstotliwość, pasmo, tryb, split i RST
                let freq_mhz = self.rig_state.frequency_hz as f64 / 1_000_000.0;
                let mut op_txt = format!(
                    "📻 {:.3} MHz | {} | {}",
                    freq_mhz, self.entry_band, self.entry_mode
                );
                if self.vfo_split {
                    op_txt.push_str(" | SPLIT");
                }
                ui.label(
                    egui::RichText::new(op_txt)
                        .size(11.0)
                        .monospace()
                        .color(palette.accent),
                );

                ui.separator();

                let rst_txt = format!("RST {} / {}", self.entry_rst_sent, self.entry_rst_rcvd);
                ui.label(
                    egui::RichText::new(rst_txt)
                        .size(11.0)
                        .monospace()
                        .color(palette.warning),
                );

                ui.separator();

                let prop_txt = format!(
                    "🌞 SFI {} | K {}",
                    self.solar.space_weather.sfi, self.solar.space_weather.k_index
                );
                ui.label(
                    egui::RichText::new(prop_txt)
                        .size(11.0)
                        .color(palette.success),
                );

                ui.separator();

                let journal_str = format!(
                    "📁 {}: {} ({} QSO)",
                    tr(key::STATUSBAR_LOG, lang),
                    self.active_journal.name,
                    self.recent_qsos.len()
                );
                ui.label(
                    egui::RichText::new(journal_str)
                        .size(11.0)
                        .color(palette.success),
                );

                ui.separator();

                let prof_name = if self.config.station.name.is_empty() {
                    &self.config.station.callsign
                } else {
                    &self.config.station.name
                };
                let profile_str = format!("🏷 {}: {}", tr(key::STATUSBAR_PROFILE, lang), prof_name);
                ui.label(
                    egui::RichText::new(profile_str)
                        .size(11.0)
                        .color(palette.warning),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "SPLogbook v{} | SP6INA | GPLv3",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .size(10.0)
                        .color(palette.text_subtle),
                    );
                    ui.separator();

                    let utc_str = format!(
                        "⏱ {}: {}",
                        tr(key::STATUSBAR_UTC, lang),
                        chrono::Utc::now().format("%H:%M:%S")
                    );
                    ui.label(
                        egui::RichText::new(utc_str)
                            .size(11.0)
                            .monospace()
                            .color(palette.accent),
                    );
                    ui.separator();

                    if self.cat_connected {
                        ui.label(
                            egui::RichText::new("● CAT")
                                .size(11.0)
                                .strong()
                                .color(palette.success),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("○ CAT")
                                .size(11.0)
                                .color(palette.text_muted),
                        );
                    }
                    ui.separator();

                    if self.cluster_connected {
                        ui.label(
                            egui::RichText::new("● CLUSTER")
                                .size(11.0)
                                .strong()
                                .color(palette.success),
                        );
                    } else if self.cluster_connecting {
                        ui.label(
                            egui::RichText::new("● CLUSTER")
                                .size(11.0)
                                .color(palette.warning),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("○ CLUSTER")
                                .size(11.0)
                                .color(palette.text_muted),
                        );
                    }
                });
            });
        });
    }

    pub fn render_chrome_overlays(&mut self, ctx: &egui::Context) {
        // Pływające powiadomienia Toast
        if let Some((ref msg, time)) = self.gui.chrome.status_toast {
            if time.elapsed().as_secs() < 5 {
                egui::Area::new(egui::Id::new("status_toast_area"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -40.0))
                    .show(ctx, |ui| {
                        let palette = ui.palette();
                        egui::Frame::popup(ui.style())
                            .fill(palette.surface_bg)
                            .stroke(egui::Stroke::new(1.0_f32, palette.accent))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("🔔 {msg}"))
                                        .strong()
                                        .color(palette.text_primary),
                                );
                            });
                    });
            } else {
                self.gui.chrome.status_toast = None;
            }
        }

        // Okno "O programie SPLogbook"
        if self.gui.chrome.show_about_window {
            let lang = self.gui.appearance.language;
            let mut is_open = self.gui.chrome.show_about_window;
            let mut close_req = false;
            egui::Window::new(tr(key::TAB_ABOUT, lang))
                .open(&mut is_open)
                .default_size([480.0, 340.0])
                .show(ctx, |ui| {
                    let palette = ui.palette();
                    ui.vertical_centered(|ui| {
                        ui.heading(
                            egui::RichText::new("📻 SPLogbook")
                                .size(24.0)
                                .color(palette.accent),
                        );
                        ui.label(
                            egui::RichText::new(tr(key::APP_ABOUT_SUBTITLE, lang))
                                .size(14.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(tr(key::APP_ABOUT_AUTHOR, lang))
                                .size(13.0)
                                .strong()
                                .color(palette.warning),
                        );
                        ui.label(
                            egui::RichText::new("E-mail: contact@splogbook.org")
                                .size(12.0)
                                .color(palette.accent),
                        );
                        ui.label(tr(key::APP_LICENSE, lang));
                        ui.add_space(12.0);
                        ui.label(tr(key::APP_ABOUT_DESC1, lang));
                        ui.label(tr(key::APP_ABOUT_DESC2, lang));
                        ui.add_space(10.0);
                        let diag_label = tr(key::APP_ABOUT_DIAGNOSTICS, lang);
                        egui::CollapsingHeader::new(diag_label).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(tr(key::APP_DIAG_VERSION, lang));
                                ui.monospace(env!("CARGO_PKG_VERSION"));
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr(key::APP_DIAG_PLATFORM, lang));
                                ui.monospace(std::env::consts::OS);
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr(key::APP_DIAG_ARCH, lang));
                                ui.monospace(std::env::consts::ARCH);
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr(key::APP_DIAG_THEME, lang));
                                ui.monospace(self.gui.appearance.theme_preset.id());
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr(key::APP_DIAG_DATA_DIR, lang));
                                ui.monospace(
                                    self.paths
                                        .config_file_path
                                        .parent()
                                        .map(|p| p.display().to_string())
                                        .unwrap_or_default(),
                                );
                            });
                            let copy_diag = tr(key::APP_DIAG_COPY_BTN, lang);
                            if ui.button(copy_diag).clicked() {
                                let info = format!(
                                    "SPLogbook {} | OS: {} {} | Theme: {} | Config: {}",
                                    env!("CARGO_PKG_VERSION"),
                                    std::env::consts::OS,
                                    std::env::consts::ARCH,
                                    self.gui.appearance.theme_preset.id(),
                                    self.paths.config_file_path.display()
                                );
                                ui.ctx().copy_text(info);
                            }
                        });
                        ui.add_space(8.0);
                        if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                            close_req = true;
                        }
                    });
                });
            if close_req || !is_open {
                self.gui.chrome.show_about_window = false;
            }
        }

        // Paleta poleceń (Ctrl+Shift+P)
        crate::gui::command_palette::render_command_palette(self, ctx);

        // Dziennik zmian i sprawdzanie aktualizacji
        crate::gui::changelog::render_changelog_window(self, ctx);
        crate::gui::changelog::render_update_check_window(self, ctx);
        crate::gui::user_manual::render_user_manual_window(self, ctx);

        if self.gui.chrome.show_shortcuts_window {
            let lang = self.gui.appearance.language;
            let mut is_open = self.gui.chrome.show_shortcuts_window;
            egui::Window::new(format!("⌨ {}", tr(key::HELP_SHORTCUTS_TITLE, lang)))
                .open(&mut is_open)
                .default_size([600.0, 520.0])
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let sections: [(&str, &[(&str, &str)]); 3] = [
                            (
                                tr(key::SHORTCUTS_SEC_LOGGING, lang),
                                &[
                                    ("Enter / F2", tr(key::SHORTCUTS_SAVE_QSO, lang)),
                                    ("Esc / F3", tr(key::SHORTCUTS_WIPE_FORM, lang)),
                                    ("Ctrl+N", tr(key::SHORTCUTS_NEW_QSO, lang)),
                                    ("Ctrl+W", tr(key::SHORTCUTS_WIPE_EXCHANGE, lang)),
                                    ("F4", tr(key::SHORTCUTS_CALLBOOK_LOOKUP, lang)),
                                    ("Ctrl+S", tr(key::SHORTCUTS_SAVE_CONFIG, lang)),
                                    ("F5", tr(key::SHORTCUTS_REFRESH_LOG, lang)),
                                    ("Ctrl+F", tr(key::SHORTCUTS_SEARCH_FILTERS, lang)),
                                    ("Ctrl+L", tr(key::SHORTCUTS_FOCUS_TABLE, lang)),
                                    ("Ctrl+Z", tr(key::SHORTCUTS_UNDO, lang)),
                                    ("Ctrl+Y / Ctrl+Shift+Z", tr(key::SHORTCUTS_REDO, lang)),
                                ],
                            ),
                            (
                                tr(key::SHORTCUTS_SEC_TRANSCEIVER, lang),
                                &[
                                    ("F6", tr(key::SHORTCUTS_SPOT_DX, lang)),
                                    ("F7", tr(key::SHORTCUTS_TOGGLE_PTT, lang)),
                                    ("F8", tr(key::SHORTCUTS_VOICE_KEYER, lang)),
                                    ("Ctrl+K", tr(key::SHORTCUTS_CW_TERMINAL, lang)),
                                    ("Ctrl+D", tr(key::SHORTCUTS_DX_CLUSTER, lang)),
                                    ("Ctrl+B", tr(key::SHORTCUTS_BANDMAP, lang)),
                                    ("Ctrl+M", tr(key::SHORTCUTS_GREYLINE_MAP, lang)),
                                    ("Ctrl+P", tr(key::SHORTCUTS_WORKSPACE_PROFILES, lang)),
                                ],
                            ),
                            (
                                tr(key::SHORTCUTS_SEC_TOOLS, lang),
                                &[
                                    ("Ctrl+Shift+P", tr(key::SHORTCUTS_COMMAND_PALETTE, lang)),
                                    ("Ctrl+Shift+S", tr(key::SHORTCUTS_STATS, lang)),
                                    ("Ctrl+E", tr(key::SHORTCUTS_EXPORT_CSV, lang)),
                                    ("Ctrl+I", tr(key::SHORTCUTS_IMPORT_ADIF, lang)),
                                    ("Ctrl+T", tr(key::SHORTCUTS_TOGGLE_THEME, lang)),
                                    ("F11", tr(key::SHORTCUTS_FULLSCREEN, lang)),
                                    ("F12 / Ctrl+H", tr(key::SHORTCUTS_USER_MANUAL, lang)),
                                    ("F1", tr(key::SHORTCUTS_SHORTCUTS_HELP, lang)),
                                    ("Ctrl+Q", tr(key::SHORTCUTS_QUIT, lang)),
                                ],
                            ),
                        ];

                        let palette = ui.palette();
                        for (cat_title, rows) in sections {
                            ui.add_space(6.0);
                            ui.heading(
                                egui::RichText::new(cat_title)
                                    .size(14.0)
                                    .strong()
                                    .color(palette.accent),
                            );
                            ui.separator();
                            egui::Grid::new(format!("shortcuts_grid_{cat_title}"))
                                .num_columns(2)
                                .spacing([20.0, 6.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for (key, desc) in rows {
                                        ui.monospace(
                                            egui::RichText::new(*key)
                                                .strong()
                                                .color(palette.warning),
                                        );
                                        ui.label(*desc);
                                        ui.end_row();
                                    }
                                });
                        }
                    });
                });
            if !is_open {
                self.gui.chrome.show_shortcuts_window = false;
            }
        }

        // Legenda kolorów i statusów (niezależna od samego koloru — symbole + tekst)
        if self.gui.chrome.show_legend_window {
            let lang = self.gui.appearance.language;
            let mut is_open = self.gui.chrome.show_legend_window;
            egui::Window::new(tr(key::LEGEND_TITLE, lang))
                .open(&mut is_open)
                .default_size([480.0, 420.0])
                .show(ctx, |ui| {
                    let palette = ui.palette();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(tr(key::APP_COLORBLIND_NOTE, lang))
                                .italics()
                                .color(palette.text_muted),
                        );

                        ui.add_space(8.0);
                        let rows: [(egui::Color32, &str, &str, &str); 6] = [
                            (
                                palette.success,
                                "●",
                                tr(key::LEGEND_ROW_GREEN_NAME, lang),
                                tr(key::LEGEND_ROW_GREEN_DESC, lang),
                            ),
                            (
                                palette.warning,
                                "◐",
                                tr(key::LEGEND_ROW_AMBER_NAME, lang),
                                tr(key::LEGEND_ROW_AMBER_DESC, lang),
                            ),
                            (
                                palette.text_muted,
                                "○",
                                tr(key::LEGEND_ROW_GRAY_NAME, lang),
                                tr(key::LEGEND_ROW_GRAY_DESC, lang),
                            ),
                            (
                                palette.danger,
                                "⚠",
                                tr(key::LEGEND_ROW_RED_NAME, lang),
                                tr(key::LEGEND_ROW_RED_DESC, lang),
                            ),
                            (
                                palette.accent,
                                "ℹ",
                                tr(key::LEGEND_ROW_BLUE_NAME, lang),
                                tr(key::LEGEND_ROW_BLUE_DESC, lang),
                            ),
                            (
                                palette.accent_alt,
                                "✦",
                                tr(key::LEGEND_ROW_CYAN_NAME, lang),
                                tr(key::LEGEND_ROW_CYAN_DESC, lang),
                            ),
                        ];
                        egui::Grid::new("legend_grid")
                            .num_columns(4)
                            .spacing([12.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (color, shape, name, desc) in rows {
                                    ui.label(egui::RichText::new(shape).color(color).size(18.0));
                                    ui.label(egui::RichText::new(name).strong().color(color));
                                    ui.label(egui::RichText::new("■").color(color).size(14.0));
                                    ui.label(egui::RichText::new(desc).color(palette.text_muted));
                                    ui.end_row();
                                }
                            });
                    });
                });
            if !is_open {
                self.gui.chrome.show_legend_window = false;
            }
        }
    }
}
