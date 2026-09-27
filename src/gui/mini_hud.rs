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

        // Wariant "pasek operacyjny" dokowany do dolnej krawędzi okna.
        if app.hud_operating_bar {
            render_operating_bar(app, ctx);
            return;
        }

        // Wariant "zawsze na wierzchu" — osobny, niezależny viewport systemowy.
        if app.hud_always_on_top {
            let mut still_open = true;
            let mut exit_compact = false;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("mini_hud_aot_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("📻 SPLogbook Mini HUD")
                    .with_decorations(false)
                    .with_always_on_top()
                    .with_inner_size(app.hud_saved_size.unwrap_or([460.0, 260.0])),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        render_hud_body(app, ui, &mut exit_compact);
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        still_open = false;
                    }
                },
            );
            if exit_compact {
                app.compact_hud_mode = false;
                app.hud_always_on_top = false;
                app.save_station_config();
            }
            if !still_open {
                app.hud_always_on_top = false;
                app.save_station_config();
            }
            return;
        }

        // Domyślny wariant: swobodne, skalowalne okno wewnątrz pulpitu.
        let mut exit_compact = false;
        let mut win = egui::Window::new(egui::RichText::new("📻 SPLogbook Mini HUD").strong().size(12.0))
            .collapsible(false)
            .resizable(true)
            .default_width(app.hud_saved_size.map(|s| s[0]).unwrap_or(460.0))
            .default_height(app.hud_saved_size.map(|s| s[1]).unwrap_or(260.0));
        if let Some(pos) = app.hud_saved_pos {
            win = win.default_pos(pos);
        }

        let resp = win.show(ctx, |ui| {
            render_hud_body(app, ui, &mut exit_compact);
        });

        if let Some(inner) = resp {
            let rect = inner.response.rect;
            app.hud_saved_pos = Some([rect.min.x, rect.min.y]);
            app.hud_saved_size = Some([rect.width(), rect.height()]);
        }

        if exit_compact {
            app.compact_hud_mode = false;
            app.save_station_config();
        }
    }
}

/// Wspólna treść okna Mini HUD (używana przez wariant okna oraz viewport always-on-top).
fn render_hud_body(app: &mut SpLogApp, ui: &mut egui::Ui, exit_compact: &mut bool) {
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
                    *exit_compact = true;
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
                    ui.colored_label(egui::Color32::from_rgb(253, 224, 71), format!("🧭 {} | {:.0}°", app.active_distance_display(), app.active_bearing_deg));
                }
            });
        }

        ui.separator();

        // 5. Ustawienia widoku Mini HUD
        ui.horizontal(|ui| {
            let mut changed = false;
            if ui.checkbox(&mut app.hud_operating_bar, "📏 Pasek operacyjny").changed() {
                changed = true;
            }
            if ui.checkbox(&mut app.hud_always_on_top, "📌 Zawsze na wierzchu").on_hover_text("Otwórz Mini HUD jako zawsze-widoczne okno systemowe").changed() {
                changed = true;
            }
            if changed {
                app.save_station_config();
            }
        });
    });
}

/// Smukły, dokowany pasek operacyjny u dołu okna (wariant "operating bar").
fn render_operating_bar(app: &mut SpLogApp, ctx: &egui::Context) {
    let mut exit_compact = false;
    egui::TopBottomPanel::bottom("mini_hud_operating_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let freq_mhz = app.rig_state.frequency_hz as f64 / 1_000_000.0;
            ui.label(
                egui::RichText::new(format!("{:.3} MHz", freq_mhz))
                    .strong()
                    .size(16.0)
                    .monospace()
                    .color(egui::Color32::from_rgb(34, 197, 94))
            );

            ui.separator();

            egui::ComboBox::from_id_salt("opbar_band")
                .selected_text(&app.entry_band)
                .width(52.0)
                .show_ui(ui, |ui| {
                    for b in &["160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"] {
                        ui.selectable_value(&mut app.entry_band, b.to_string(), *b);
                    }
                });

            egui::ComboBox::from_id_salt("opbar_mode")
                .selected_text(&app.entry_mode)
                .width(48.0)
                .show_ui(ui, |ui| {
                    for m in &["CW", "SSB", "FT8", "FT4", "RTTY", "AM", "FM"] {
                        ui.selectable_value(&mut app.entry_mode, m.to_string(), *m);
                    }
                });

            ui.separator();

            let call_edit = ui.add(
                egui::TextEdit::singleline(&mut app.entry_callsign)
                    .font(egui::TextStyle::Heading)
                    .desired_width(120.0)
                    .hint_text("ZNAK DX")
            );
            if call_edit.changed() {
                app.entry_callsign = app.entry_callsign.to_uppercase();
                app.on_callsign_changed();
            }

            ui.label("RST:");
            ui.add(egui::TextEdit::singleline(&mut app.entry_rst_sent).desired_width(34.0));
            ui.add(egui::TextEdit::singleline(&mut app.entry_rst_rcvd).desired_width(34.0));

            if ui.button(egui::RichText::new("💾 ZAPISZ").strong().color(egui::Color32::from_rgb(34, 197, 94))).clicked() {
                app.save_qso();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗗 Pełny pulpit").on_hover_text("Powróć do pełnego pulpitu roboczego SPLogbook").clicked() {
                    exit_compact = true;
                }
                if ui.button("🪟 Okno HUD").on_hover_text("Przełącz na pływające okno Mini HUD").clicked() {
                    app.hud_operating_bar = false;
                    app.save_station_config();
                }
            });
        });
    });

    if exit_compact {
        app.compact_hud_mode = false;
        app.save_station_config();
    }
}
