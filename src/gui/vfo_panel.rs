// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::{ColorExt, ThemeExt};
use eframe::egui;

pub fn render_vfo_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.views.panel_vfo.visible {
        return;
    }

    if app.views.panel_vfo.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let ((), captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("vfo_viewport"),
            format!(
                "📻 {} - SPLogbook",
                tr(key::VIEW_PANEL_VFO, app.gui.appearance.language)
            ),
            [520.0, 320.0],
            [400.0, 240.0],
            app.views.panel_vfo.saved_pos,
            app.views.panel_vfo.saved_size,
            |app, ui| {
                egui::Panel::top("vfo_vp_bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let lang = app.gui.appearance.language;
                        if ui
                            .button(format!("↙ {}", tr(key::WINDOW_DOCK, lang)))
                            .on_hover_text(tr(key::WINDOW_DOCK_TOOLTIP, lang))
                            .clicked()
                        {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    render_vfo_body(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if let Some((pos, size)) = captured_geo {
            if app.views.panel_vfo.saved_pos != Some(pos)
                || app.views.panel_vfo.saved_size != Some(size)
            {
                app.views.panel_vfo.saved_pos = Some(pos);
                app.views.panel_vfo.saved_size = Some(size);
                app.save_station_config();
            }
        }
        if dock_back {
            app.views.panel_vfo.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.views.panel_vfo.visible = false;
            app.views.panel_vfo.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.views.panel_vfo.visible;
    let screen = ctx.content_rect();
    let default_pos = [screen.min.x + 8.0, screen.min.y + 8.0];
    let default_size = [510.0, 310.0];

    let mut win = egui::Window::new(
        egui::RichText::new(format!(
            "   📻 {}",
            tr(key::VIEW_PANEL_VFO, app.gui.appearance.language)
        ))
        .size(12.0)
        .strong(),
    )
    .open(&mut open)
    .min_size([400.0, 240.0])
    .resizable(true)
    .collapsible(true)
    .constrain_to(screen);

    if app.gui.workspace.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.views.panel_vfo.saved_pos {
        let sz = app.views.panel_vfo.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_vfo_body(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(
            ctx,
            "vfo_popout_btn",
            res.response.layer_id,
            res.response.rect,
            &mut app.views.panel_vfo.floating,
            true,
            app.gui.appearance.language,
        );
        if app.views.panel_vfo.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let rect = res.response.rect;
            let new_pos = [rect.min.x, rect.min.y];
            let new_size = [rect.width(), rect.height()];
            if app.views.panel_vfo.saved_pos != Some(new_pos)
                || app.views.panel_vfo.saved_size != Some(new_size)
            {
                app.views.panel_vfo.saved_pos = Some(new_pos);
                app.views.panel_vfo.saved_size = Some(new_size);
            }
            if res.response.drag_stopped() {
                app.save_station_config();
            }
        }
    }

    if !open {
        app.views.panel_vfo.visible = false;
        app.save_station_config();
    }
}

fn render_tuning_digit(
    ui: &mut egui::Ui,
    digit: u64,
    weight_hz: i64,
    dim: bool,
    unit_label: &str,
    app: &mut SpLogApp,
) {
    let palette = ui.palette();
    let lang = app.gui.appearance.language;
    let text = format!("{digit}");
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(17.0, 36.0), egui::Sense::click());
    let hovered = resp.hovered();

    let color = if hovered {
        palette.warning
    } else if dim {
        palette.text_subtle
    } else {
        palette.accent
    };

    if hovered {
        let underline_y = rect.bottom() - 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), underline_y),
                egui::pos2(rect.right(), underline_y),
            ],
            egui::Stroke::new(2.5_f32, palette.warning),
        );
        ui.painter()
            .rect_filled(rect, 2.0, palette.warning.alpha(35));
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        &text,
        egui::FontId::monospace(28.0),
        color,
    );

    let resp = resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);
    resp.clone().on_hover_text(
        Msg::new(key::VFO_TUNING_STEP_TOOLTIP)
            .arg("unit_label", unit_label)
            .text(lang),
    );

    if hovered {
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y > 0.0 {
            app.step_vfo(weight_hz);
        } else if scroll_y < 0.0 {
            app.step_vfo(-weight_hz);
        } else if resp.clicked() {
            app.step_vfo(weight_hz);
        } else if resp.secondary_clicked() {
            app.step_vfo(-weight_hz);
        }
    }
}

fn render_tuning_dot(ui: &mut egui::Ui) {
    let palette = ui.palette();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 36.0), egui::Sense::hover());
    let dot_pos = egui::pos2(rect.center().x, rect.bottom() - 8.0);
    ui.painter().circle_filled(dot_pos, 2.2, palette.accent);
}

pub fn render_vfo_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let palette = ui.palette();
    let lang = app.gui.appearance.language;
    let frequency_hz = app.cat.rig_state.frequency_hz;

    // Nagłówek konsoli radiowej: Model radia + PTT + Ustawienia
    ui.horizontal(|ui| {
        let rig_badge = if app.config.cat_rig_model.is_empty() {
            "Manual Transceiver".to_string()
        } else {
            app.config.cat_rig_model.clone()
        };

        if app.cat.connected {
            ui.label(
                egui::RichText::new(format!("📻 {rig_badge}"))
                    .color(palette.accent)
                    .strong()
                    .size(12.0),
            );
            ui.label(
                egui::RichText::new("● ONLINE")
                    .color(palette.success)
                    .size(10.0),
            );
        } else {
            ui.label(
                egui::RichText::new(format!("📻 {rig_badge}"))
                    .color(palette.text_muted)
                    .size(12.0),
            );
            ui.label(
                egui::RichText::new("○ OFFLINE")
                    .color(palette.danger)
                    .size(10.0),
            );
        }

        ui.separator();

        // Przycisk PTT (nadawanie)
        let ptt_color = if app.cat.ptt_active {
            palette.tx
        } else {
            palette.text_subtle
        };
        let ptt_label = if app.cat.ptt_active {
            "🔴 TX ON"
        } else {
            "⚫ RX"
        };
        if ui
            .button(egui::RichText::new(ptt_label).color(ptt_color).strong())
            .on_hover_text(tr(key::VFO_PTT_TOOLTIP, lang))
            .clicked()
        {
            app.toggle_ptt();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("⚙")
                .on_hover_text(tr(key::CAT_SETTINGS_TOOLTIP, app.gui.appearance.language))
                .clicked()
            {
                app.gui.windows.show_cat_settings_window = true;
            }
        });
    });

    ui.separator();

    // Twin VFO / Split Status
    ui.horizontal(|ui| {
        let vfo_a_active = app.cat.rig_state.vfo != "VFOB";
        let color_a = if vfo_a_active {
            palette.vfo_a
        } else {
            palette.text_muted
        };
        ui.label(egui::RichText::new("VFO A:").strong().color(color_a));
        ui.label(
            egui::RichText::new(format!("{:.3} MHz", (frequency_hz as f64) / 1_000_000.0)).strong(),
        );

        ui.separator();
        let split_freq = if app.cat.vfo_split {
            frequency_hz + (app.cat.vfo.split_offset_khz * 1000.0) as u64
        } else {
            frequency_hz
        };
        let color_b = if app.cat.vfo_split {
            palette.split
        } else {
            palette.text_muted
        };
        ui.label(egui::RichText::new("VFO B (TX):").strong().color(color_b));
        ui.label(
            egui::RichText::new(format!("{:.3} MHz", (split_freq as f64) / 1_000_000.0)).strong(),
        );

        if ui.selectable_label(app.cat.vfo_split, "SPLIT").clicked() {
            let enabled = !app.cat.vfo_split;
            app.set_vfo_split(enabled);
        }

        if app.cat.vfo_split {
            if ui.button("+1k").clicked() {
                app.cat.vfo.split_offset_khz = 1.0;
            }
            if ui.button("+5k").clicked() {
                app.cat.vfo.split_offset_khz = 5.0;
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("⇄ A/B")
                .on_hover_text(tr(key::VFO_SWAP_TOOLTIP, lang))
                .clicked()
            {
                app.swap_vfo_ab();
            }
        });
    });

    ui.separator();

    // Rozbicie częstotliwości na poszczególne cyfry
    let d_100m = (frequency_hz / 100_000_000) % 10;
    let d_10m = (frequency_hz / 10_000_000) % 10;
    let d_1m = (frequency_hz / 1_000_000) % 10;
    let d_100k = (frequency_hz / 100_000) % 10;
    let d_10k = (frequency_hz / 10_000) % 10;
    let d_1k = (frequency_hz / 1_000) % 10;
    let d_100h = (frequency_hz / 100) % 10;
    let d_10h = (frequency_hz / 10) % 10;
    let d_1h = frequency_hz % 10;

    let dim_100m = frequency_hz < 100_000_000;
    let dim_10m = frequency_hz < 10_000_000;

    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);

            // MHz
            render_tuning_digit(ui, d_100m, 100_000_000, dim_100m, "100 MHz", app);
            render_tuning_digit(ui, d_10m, 10_000_000, dim_10m, "10 MHz", app);
            render_tuning_digit(ui, d_1m, 1_000_000, false, "1 MHz", app);

            render_tuning_dot(ui);

            // kHz
            render_tuning_digit(ui, d_100k, 100_000, false, "100 kHz", app);
            render_tuning_digit(ui, d_10k, 10_000, false, "10 kHz", app);
            render_tuning_digit(ui, d_1k, 1_000, false, "1 kHz", app);

            render_tuning_dot(ui);

            // Hz
            render_tuning_digit(ui, d_100h, 100, false, "100 Hz", app);
            render_tuning_digit(ui, d_10h, 10, false, "10 Hz", app);
            render_tuning_digit(ui, d_1h, 1, false, "1 Hz", app);

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("MHz")
                    .size(15.0)
                    .monospace()
                    .color(palette.text_muted),
            );

            ui.add_space(8.0);

            // Bezpośrednie wpisanie częstotliwości (kHz) z zachowaniem bufora edycji podczas fokusu
            ui.label(
                egui::RichText::new("kHz:")
                    .size(11.0)
                    .color(palette.text_muted),
            );
            let edit_id = egui::Id::new("vfo_freq_khz_input");
            let is_focused = ui.ctx().memory(|m| m.has_focus(edit_id));
            let mut freq_khz_str = if is_focused {
                ui.ctx()
                    .data_mut(|d| d.get_temp::<String>(edit_id))
                    .unwrap_or_else(|| format!("{:.3}", (frequency_hz as f64) / 1000.0))
            } else {
                format!("{:.3}", (frequency_hz as f64) / 1000.0)
            };
            let resp_txt = ui.add(
                egui::TextEdit::singleline(&mut freq_khz_str)
                    .id(edit_id)
                    .desired_width(75.0),
            );
            if resp_txt.has_focus() {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(edit_id, freq_khz_str.clone()));
            }
            if resp_txt.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let normalized = freq_khz_str.trim().replace(',', ".");
                if let Ok(khz) = normalized.parse::<f64>() {
                    if khz > 0.0 {
                        let new_hz = (khz * 1000.0).round() as u64;
                        app.set_vfo_frequency(new_hz);
                    }
                }
                ui.ctx().data_mut(|d| d.remove::<String>(edit_id));
            }
        });

        // Przyciski krokowe VFO (+/-) i tryby emisji
        ui.horizontal(|ui| {
            if ui.button("-10k").clicked() {
                app.step_vfo(-10_000);
            }
            if ui.button("-1k").clicked() {
                app.step_vfo(-1_000);
            }
            if ui.button("-100").clicked() {
                app.step_vfo(-100);
            }
            if ui.button("+100").clicked() {
                app.step_vfo(100);
            }
            if ui.button("+1k").clicked() {
                app.step_vfo(1_000);
            }
            if ui.button("+10k").clicked() {
                app.step_vfo(10_000);
            }

            ui.separator();
            for mode in &["CW", "LSB", "USB", "FT8", "FT4", "RTTY", "AM", "FM"] {
                let is_sel = app.cat.rig_state.mode == *mode;
                if ui.selectable_label(is_sel, *mode).clicked() {
                    app.set_vfo_mode(mode);
                }
            }
        });

        // Szybki wybór pasm KF/VHF
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(tr(key::VFO_BAND_LABEL, app.gui.appearance.language))
                    .size(11.0)
                    .strong()
                    .color(palette.text_muted),
            );
            let quick_bands = [
                ("160m", 1_840_000, "LSB"),
                ("80m", 3_710_000, "LSB"),
                ("40m", 7_150_000, "LSB"),
                ("30m", 10_136_000, "USB"),
                ("20m", 14_195_000, "USB"),
                ("17m", 18_130_000, "USB"),
                ("15m", 21_250_000, "USB"),
                ("12m", 24_950_000, "USB"),
                ("10m", 28_500_000, "USB"),
                ("6m", 50_150_000, "USB"),
                ("2m", 144_300_000, "USB"),
                ("70cm", 432_200_000, "USB"),
            ];
            for (b_name, b_freq, b_mode) in quick_bands {
                if ui.button(egui::RichText::new(b_name).size(10.0)).clicked() {
                    app.set_vfo_frequency(b_freq);
                    app.set_vfo_mode(b_mode);
                    app.gui.windows.bandmap_selected_band = b_name.to_string();
                }
            }
        });

        // Pasek DSP / Filtrów / AGC
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(tr(key::VFO_FILTERS, app.gui.appearance.language))
                    .size(10.0)
                    .strong()
                    .color(palette.text_muted),
            );
            if ui
                .selectable_label(app.cat.vfo.filter_preset == "FIL1", "FIL1 (3k)")
                .clicked()
            {
                app.set_vfo_filter("FIL1", 3000);
            }
            if ui
                .selectable_label(app.cat.vfo.filter_preset == "FIL2", "FIL2 (2.4k)")
                .clicked()
            {
                app.set_vfo_filter("FIL2", 2400);
            }
            if ui
                .selectable_label(app.cat.vfo.filter_preset == "FIL3", "FIL3 (500Hz)")
                .clicked()
            {
                app.set_vfo_filter("FIL3", 500);
            }

            ui.separator();
            ui.label(
                egui::RichText::new("RF:")
                    .size(10.0)
                    .strong()
                    .color(palette.text_muted),
            );
            for pre in &["OFF", "PRE1", "PRE2", "ATT"] {
                if ui
                    .selectable_label(app.cat.vfo.preamp_att == *pre, *pre)
                    .clicked()
                {
                    app.cat.vfo.preamp_att = pre.to_string();
                }
            }

            ui.separator();
            if ui.selectable_label(app.cat.vfo.nb_nr, "NB/NR").clicked() {
                app.cat.vfo.nb_nr = !app.cat.vfo.nb_nr;
            }

            ui.separator();
            ui.label(
                egui::RichText::new("AGC:")
                    .size(10.0)
                    .strong()
                    .color(palette.text_muted),
            );
            for agc in &["FAST", "MID", "SLOW"] {
                if ui
                    .selectable_label(app.cat.vfo.agc_speed == *agc, *agc)
                    .clicked()
                {
                    app.cat.vfo.agc_speed = agc.to_string();
                }
            }
        });
    });

    ui.add_space(4.0);

    // Klastrowe wskaźniki Mierników (S-Meter + Moc + SWR)
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("S-METER:")
                .size(11.0)
                .strong()
                .color(palette.text_muted),
        );
        ui.label(
            egui::RichText::new(&app.cat.rig_state.s_meter_unit)
                .size(12.0)
                .strong()
                .color(palette.warning),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let pwr = if app.cat.ptt_active {
                app.config.station.power_watts as f32
            } else {
                0.0
            };
            let swr = if app.cat.ptt_active { 1.2 } else { 1.0 };
            ui.label(
                egui::RichText::new(format!("PWR: {pwr:.0} W | SWR: {swr:.1}"))
                    .size(11.0)
                    .monospace()
                    .color(palette.accent),
            );
        });
    });

    let raw_db = app.cat.rig_state.s_meter_dbm;
    let meter_fraction = ((raw_db + 54.0) / 114.0).clamp(0.0, 1.0);

    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, palette.extreme_bg);
        let filled_width = rect.width() * meter_fraction;
        let fill_rect =
            egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
        let fill_color = if meter_fraction > 0.65 {
            palette.danger // Czerwony dla > S9
        } else {
            palette.success // Zielony dla S1..S9
        };
        painter.rect_filled(fill_rect, 3.0, fill_color);
    }

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("S1     S3     S5     S7     S9    +20   +40   +60dB")
                .size(9.0)
                .monospace()
                .color(palette.text_subtle),
        );
    });

    ui.separator();

    // Sterownik Rotora Anteny
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("🧭 ROTOR: {:.0}°", app.cat.rotor_state.azimuth_deg))
                .size(13.0)
                .strong()
                .color(palette.warning),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(egui::RichText::new("STOP").color(palette.danger).strong())
                .clicked()
            {
                app.stop_rotor();
            }
            if ui.button("LP").clicked() {
                app.turn_rotor_long_path();
            }
            if ui.button("SP").clicked() {
                app.turn_rotor_short_path();
            }
        });
    });
}
