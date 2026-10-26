// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Ergonomiczny, kompaktowy pasek/okno operacyjne (Mini HUD) dla stacji SPLogbook

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

pub fn render_mini_hud_bar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    if !app.config.compact_hud_mode {
        return;
    }
    let ctx = ui.ctx().clone();
    let lang = app.gui.appearance.language;

    // Wariant "pasek operacyjny" dokowany do dolnej krawędzi okna.
    if app.config.hud_operating_bar {
        render_operating_bar(app, ui);
        return;
    }

    // Wariant "zawsze na wierzchu" — osobny, niezależny viewport systemowy.
    if app.config.hud_always_on_top {
        let mut still_open = true;
        let mut exit_compact = false;
        let mut captured_geo: Option<([f32; 2], [f32; 2])> = None;
        let mut builder = egui::ViewportBuilder::default()
            .with_title(tr(key::HUD_WINDOW_TITLE, lang))
            .with_decorations(false)
            .with_always_on_top()
            .with_inner_size(app.config.hud_saved_size.unwrap_or([460.0, 260.0]));
        if let Some([x, y]) = app.config.hud_saved_pos {
            builder = builder.with_position(egui::pos2(x, y));
        }
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("mini_hud_aot_viewport"),
            builder,
            |ui, _class| {
                egui::CentralPanel::default().show(ui, |ui| {
                    render_hud_body(app, ui, &mut exit_compact);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
                captured_geo = ui.ctx().input(|i| {
                    i.viewport()
                        .outer_rect
                        .map(|r| ([r.min.x, r.min.y], [r.width(), r.height()]))
                });
            },
        );
        if let Some((pos, size)) = captured_geo {
            if app.config.hud_saved_pos != Some(pos) || app.config.hud_saved_size != Some(size) {
                app.config.hud_saved_pos = Some(pos);
                app.config.hud_saved_size = Some(size);
                app.save_station_config();
            }
        }
        if exit_compact {
            app.config.compact_hud_mode = false;
            app.config.hud_always_on_top = false;
            app.save_station_config();
        }
        if !still_open {
            app.config.hud_always_on_top = false;
            app.save_station_config();
        }
        return;
    }

    // Domyślny wariant: swobodne, skalowalne okno wewnątrz pulpitu.
    let mut exit_compact = false;
    let mut win = egui::Window::new(
        egui::RichText::new(tr(key::HUD_WINDOW_TITLE, lang))
            .strong()
            .size(12.0),
    )
    .collapsible(false)
    .resizable(true)
    .default_width(app.config.hud_saved_size.map_or(460.0, |s| s[0]))
    .default_height(app.config.hud_saved_size.map_or(260.0, |s| s[1]));
    if let Some(pos) = app.config.hud_saved_pos {
        win = win.default_pos(pos);
    }

    let resp = win.show(&ctx, |ui| {
        render_hud_body(app, ui, &mut exit_compact);
    });

    if let Some(inner) = resp {
        let rect = inner.response.rect;
        app.config.hud_saved_pos = Some([rect.min.x, rect.min.y]);
        app.config.hud_saved_size = Some([rect.width(), rect.height()]);
    }

    if exit_compact {
        app.config.compact_hud_mode = false;
        app.save_station_config();
    }
}

/// Wspólna treść okna Mini HUD (używana przez wariant okna oraz viewport always-on-top).
fn render_hud_body(app: &mut SpLogApp, ui: &mut egui::Ui, exit_compact: &mut bool) {
    let lang = app.gui.appearance.language;
    let palette = ui.palette();
    ui.vertical(|ui| {
        // 1. Pasek VFO i Strojenia
        ui.horizontal(|ui| {
            let freq_mhz = app.cat.rig_state.frequency_hz as f64 / 1_000_000.0;
            ui.label(
                egui::RichText::new(format!("{freq_mhz:.3} MHz"))
                    .strong()
                    .size(20.0)
                    .monospace()
                    .color(palette.rx),
            );

            if ui
                .button("◀ -1k")
                .on_hover_text(tr(key::HUD_TUNE_DOWN_HINT, lang))
                .clicked()
            {
                let new_freq = app.cat.rig_state.frequency_hz.saturating_sub(1000);
                app.set_vfo_frequency(new_freq);
            }
            if ui
                .button("▶ +1k")
                .on_hover_text(tr(key::HUD_TUNE_UP_HINT, lang))
                .clicked()
            {
                let new_freq = app.cat.rig_state.frequency_hz.saturating_add(1000);
                app.set_vfo_frequency(new_freq);
            }

            egui::ComboBox::from_id_salt("hud_band")
                .selected_text(&app.entry.band)
                .width(55.0)
                .show_ui(ui, |ui| {
                    for band in &[
                        "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m",
                        "70cm",
                    ] {
                        ui.selectable_value(&mut app.entry.band, band.to_string(), *band);
                    }
                });

            egui::ComboBox::from_id_salt("hud_mode")
                .selected_text(&app.entry.mode)
                .width(50.0)
                .show_ui(ui, |ui| {
                    for mode in &["CW", "SSB", "FT8", "FT4", "RTTY", "AM", "FM"] {
                        ui.selectable_value(&mut app.entry.mode, mode.to_string(), *mode);
                    }
                });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new(tr(key::HUD_FULL_DESKTOP, lang)).strong())
                    .on_hover_text(tr(key::HUD_FULL_DESKTOP_HINT, lang))
                    .clicked()
                {
                    *exit_compact = true;
                }
            });
        });

        ui.separator();

        // 2. Wprowadzanie QSO: Znak + RST
        ui.horizontal(|ui| {
            let call_edit = ui.add(
                egui::TextEdit::singleline(&mut app.entry.callsign)
                    .font(egui::TextStyle::Heading)
                    .desired_width(130.0)
                    .hint_text(tr(key::HUD_DX_CALL_HINT, lang)),
            );
            if call_edit.changed() {
                app.entry.callsign = app.entry.callsign.to_uppercase();
                app.on_callsign_changed();
            }

            if ui
                .button(egui::RichText::new("🔍").strong())
                .on_hover_text(tr(key::HUD_LOOKUP_HINT, lang))
                .clicked()
            {
                app.lookup_active_callsign_online();
            }

            ui.label("RST:");
            ui.add(egui::TextEdit::singleline(&mut app.entry.rst_sent).desired_width(38.0));
            ui.add(egui::TextEdit::singleline(&mut app.entry.rst_rcvd).desired_width(38.0));

            if ui
                .button(
                    egui::RichText::new(tr(key::HUD_SAVE_QSO, lang))
                        .strong()
                        .color(palette.success),
                )
                .clicked()
            {
                app.save_qso();
            }
        });

        // 3. Dodatkowe dane korespondenta: Imię, QTH, Grid
        ui.horizontal(|ui| {
            ui.label(tr(key::HUD_NAME_LABEL, lang));
            ui.add(egui::TextEdit::singleline(&mut app.entry.name).desired_width(85.0));

            ui.label("QTH:");
            ui.add(egui::TextEdit::singleline(&mut app.entry.qth).desired_width(90.0));

            ui.label("Grid:");
            let grid_edit =
                ui.add(egui::TextEdit::singleline(&mut app.entry.grid).desired_width(60.0));
            if grid_edit.changed() {
                app.entry.grid = app.entry.grid.to_uppercase();
                app.recalculate_distance_from_grid();
            }
        });

        // 4. Etykiety DXCC i azymutu
        if let Some(ref info) = app.entry.active_prefix_info {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    palette.accent,
                    format!("📍 {} ({})", info.country, info.wpx_prefix),
                );
                if app.entry.active_distance_km > 0.0 {
                    ui.colored_label(
                        palette.warning,
                        format!(
                            "🧭 {} | {:.0}°",
                            app.active_distance_display(),
                            app.entry.active_bearing_deg
                        ),
                    );
                }
            });
        }

        ui.separator();

        // 5. Ustawienia widoku Mini HUD
        ui.horizontal(|ui| {
            let mut changed = false;
            if ui
                .checkbox(
                    &mut app.config.hud_operating_bar,
                    tr(key::HUD_OPERATING_BAR, lang),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .checkbox(
                    &mut app.config.hud_always_on_top,
                    tr(key::HUD_ALWAYS_ON_TOP, lang),
                )
                .on_hover_text(tr(key::HUD_ALWAYS_ON_TOP_HINT, lang))
                .changed()
            {
                changed = true;
            }
            if changed {
                app.save_station_config();
            }
        });
    });
}

/// Smukły, dokowany pasek operacyjny u dołu okna (wariant "operating bar").
fn render_operating_bar(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;
    let palette = ui.palette();
    let mut exit_compact = false;
    egui::Panel::bottom("mini_hud_operating_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            let freq_mhz = app.cat.rig_state.frequency_hz as f64 / 1_000_000.0;
            ui.label(
                egui::RichText::new(format!("{freq_mhz:.3} MHz"))
                    .strong()
                    .size(16.0)
                    .monospace()
                    .color(palette.rx),
            );

            ui.separator();

            egui::ComboBox::from_id_salt("opbar_band")
                .selected_text(&app.entry.band)
                .width(52.0)
                .show_ui(ui, |ui| {
                    for band in &[
                        "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m",
                        "70cm",
                    ] {
                        ui.selectable_value(&mut app.entry.band, band.to_string(), *band);
                    }
                });

            egui::ComboBox::from_id_salt("opbar_mode")
                .selected_text(&app.entry.mode)
                .width(48.0)
                .show_ui(ui, |ui| {
                    for mode in &["CW", "SSB", "FT8", "FT4", "RTTY", "AM", "FM"] {
                        ui.selectable_value(&mut app.entry.mode, mode.to_string(), *mode);
                    }
                });

            ui.separator();

            let call_edit = ui.add(
                egui::TextEdit::singleline(&mut app.entry.callsign)
                    .font(egui::TextStyle::Heading)
                    .desired_width(120.0)
                    .hint_text(tr(key::HUD_DX_CALL_HINT, lang)),
            );
            if call_edit.changed() {
                app.entry.callsign = app.entry.callsign.to_uppercase();
                app.on_callsign_changed();
            }

            ui.label("RST:");
            ui.add(egui::TextEdit::singleline(&mut app.entry.rst_sent).desired_width(34.0));
            ui.add(egui::TextEdit::singleline(&mut app.entry.rst_rcvd).desired_width(34.0));

            if ui
                .button(
                    egui::RichText::new(tr(key::HUD_SAVE_QSO, lang))
                        .strong()
                        .color(palette.success),
                )
                .clicked()
            {
                app.save_qso();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(tr(key::HUD_FULL_DESKTOP, lang))
                    .on_hover_text(tr(key::HUD_FULL_DESKTOP_HINT, lang))
                    .clicked()
                {
                    exit_compact = true;
                }
                if ui
                    .button(tr(key::HUD_WINDOW_MODE, lang))
                    .on_hover_text(tr(key::HUD_WINDOW_MODE_HINT, lang))
                    .clicked()
                {
                    app.config.hud_operating_bar = false;
                    app.save_station_config();
                }
            });
        });
    });

    if exit_compact {
        app.config.compact_hud_mode = false;
        app.save_station_config();
    }
}
