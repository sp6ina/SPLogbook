// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Ergonomiczny, kompaktowy pasek/okno operacyjne (Mini HUD) dla stacji SPLogbook

use crate::gui::app::SpLogApp;
use eframe::egui;

pub struct MiniHudBar;

impl MiniHudBar {
    pub fn render(app: &mut SpLogApp, ctx: &egui::Context) {
        if !app.compact_hud_mode {
            return;
        }

        let mut exit_compact = false;

        egui::Window::new(egui::RichText::new("📻 SPLogbook Mini HUD").strong().size(12.0))
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // 1. Pasek VFO i Strojenia
                    ui.horizontal(|ui| {
                        let freq_mhz = app.rig_state.frequency_hz as f64 / 1_000_000.0;
                        ui.label(
                            egui::RichText::new(format!("{:.3} MHz", freq_mhz))
                                .strong()
                                .size(20.0)
                                .monospace()
                                .color(egui::Color32::from_rgb(34, 197, 94))
                        );

                        if ui.button("◀ -1k").on_hover_text("Dostrój -1 kHz").clicked() {
                            app.rig_state.frequency_hz = app.rig_state.frequency_hz.saturating_sub(1000);
                        }
                        if ui.button("▶ +1k").on_hover_text("Dostrój +1 kHz").clicked() {
                            app.rig_state.frequency_hz = app.rig_state.frequency_hz.saturating_add(1000);
                        }

                        egui::ComboBox::from_id_salt("hud_band")
                            .selected_text(&app.entry_band)
                            .width(55.0)
                            .show_ui(ui, |ui| {
                                for b in &["160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"] {
                                    ui.selectable_value(&mut app.entry_band, b.to_string(), *b);
                                }
                            });

                        egui::ComboBox::from_id_salt("hud_mode")
                            .selected_text(&app.entry_mode)
                            .width(50.0)
                            .show_ui(ui, |ui| {
                                for m in &["CW", "SSB", "FT8", "FT4", "RTTY", "AM", "FM"] {
                                    ui.selectable_value(&mut app.entry_mode, m.to_string(), *m);
                                }
                            });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("🗗 Pełny pulpit").strong()).on_hover_text("Powróć do pełnego pulpitu roboczego SPLogbook").clicked() {
                                exit_compact = true;
                            }
                        });
                    });

                    ui.separator();

                    // 2. Wprowadzanie QSO: Znak + RST
                    ui.horizontal(|ui| {
                        let call_edit = ui.add(
                            egui::TextEdit::singleline(&mut app.entry_callsign)
                                .font(egui::TextStyle::Heading)
                                .desired_width(130.0)
                                .hint_text("ZNAK DX")
                        );
                        if call_edit.changed() {
                            app.entry_callsign = app.entry_callsign.to_uppercase();
                            app.on_callsign_changed();
                        }

                        if ui.button(egui::RichText::new("🔍").strong()).on_hover_text("Pobierz dane z Callbook / HamQTH / QRZ").clicked() {
                            app.lookup_active_callsign_online();
                        }

                        ui.label("RST:");
                        ui.add(egui::TextEdit::singleline(&mut app.entry_rst_sent).desired_width(38.0));
                        ui.add(egui::TextEdit::singleline(&mut app.entry_rst_rcvd).desired_width(38.0));

                        if ui.button(egui::RichText::new("💾 ZAPISZ").strong().color(egui::Color32::from_rgb(34, 197, 94))).clicked() {
                            app.save_qso();
                        }
                    });

                    // 3. Dodatkowe dane korespondenta: Imię, QTH, Grid
                    ui.horizontal(|ui| {
                        ui.label("Imię:");
                        ui.add(egui::TextEdit::singleline(&mut app.entry_name).desired_width(85.0));

                        ui.label("QTH:");
                        ui.add(egui::TextEdit::singleline(&mut app.entry_qth).desired_width(90.0));

                        ui.label("Grid:");
                        let grid_edit = ui.add(egui::TextEdit::singleline(&mut app.entry_grid).desired_width(60.0));
                        if grid_edit.changed() {
                            app.entry_grid = app.entry_grid.to_uppercase();
                            app.recalculate_distance_from_grid();
                        }
                    });

                    // 4. Etykiety DXCC i azymutu
                    if let Some(ref info) = app.active_prefix_info {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(147, 197, 253), format!("📍 {} ({})", info.country, info.wpx_prefix));
                            if app.active_distance_km > 0.0 {
                                ui.colored_label(egui::Color32::from_rgb(253, 224, 71), format!("🧭 {:.0} km | {:.0}°", app.active_distance_km, app.active_bearing_deg));
                            }
                        });
                    }
                });
            });

        if exit_compact {
            app.compact_hud_mode = false;
            app.save_station_config();
        }
    }
}
