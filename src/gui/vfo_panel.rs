// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::gui::app::SpLogApp;
use crate::core::i18n::tr;
use eframe::egui;

pub fn render_vfo_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_vfo.visible {
        return;
    }

    if app.panel_vfo.floating {
        let mut still_open = true;
        let mut dock_back = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("vfo_viewport"),
            egui::ViewportBuilder::default()
                .with_title(format!("📻 {} - SPLogbook", tr("view.panel_vfo", app.current_language)))
                .with_inner_size([520.0, 320.0])
                .with_min_inner_size([400.0, 240.0]),
            |ctx, _class| {
                egui::TopBottomPanel::top("vfo_vp_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let lang = app.current_language;
                        if ui.button(format!("↙ {}", tr("window.dock", lang))).on_hover_text(tr("window.dock_tooltip", lang)).clicked() {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_vfo_body(app, ui);
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if dock_back {
            app.panel_vfo.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_vfo.visible = false;
            app.panel_vfo.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut open = app.panel_vfo.visible;
    let screen = ctx.available_rect();
    let default_pos = [screen.min.x + 8.0, screen.min.y + 8.0];
    let default_size = [510.0, 310.0];

    let mut win = egui::Window::new(egui::RichText::new(format!("   📻 {}", tr("view.panel_vfo", app.current_language))).size(12.0).strong())
        .open(&mut open)
        .min_size([400.0, 240.0])
        .resizable(true)
        .collapsible(true)
        .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_vfo.saved_pos {
        let sz = app.panel_vfo.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_vfo_body(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(ctx, "vfo_popout_btn", res.response.layer_id, res.response.rect, &mut app.panel_vfo.floating, true);
        if app.panel_vfo.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_vfo.saved_pos != Some(new_pos) || app.panel_vfo.saved_size != Some(new_size) {
                app.panel_vfo.saved_pos = Some(new_pos);
                app.panel_vfo.saved_size = Some(new_size);
                app.save_station_config();
            }
        }
    }

    if !open {
        app.panel_vfo.visible = false;
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
    let text = format!("{}", digit);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(17.0, 36.0), egui::Sense::click());
    let hovered = resp.hovered();

    let color = if hovered {
        egui::Color32::from_rgb(250, 204, 21) // Bright gold
    } else if dim {
        egui::Color32::from_rgb(71, 85, 105) // Dim slate
    } else {
        egui::Color32::from_rgb(56, 189, 248) // Electric cyan
    };

    if hovered {
        let underline_y = rect.bottom() - 1.0;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), underline_y), egui::pos2(rect.right(), underline_y)],
            egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(250, 204, 21)),
        );
        ui.painter().rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgba_unmultiplied(250, 204, 21, 35),
        );
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        &text,
        egui::FontId::monospace(28.0),
        color,
    );

    let resp = resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);
    resp.clone().on_hover_text(format!(
        "🎛 Krok strojenia: {}\n• Kółko myszy w górę: +{}\n• Kółko myszy w dół: -{}\n• Lewy klik: +{}, Prawy klik: -{}",
        unit_label, unit_label, unit_label, unit_label, unit_label
    ));

    if hovered {
        let scroll_y = ui.input(|i| {
            if i.raw_scroll_delta.y.abs() > 0.0 {
                i.raw_scroll_delta.y
            } else {
                i.smooth_scroll_delta.y
            }
        });
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
    let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 36.0), egui::Sense::hover());
    let dot_pos = egui::pos2(rect.center().x, rect.bottom() - 8.0);
    ui.painter().circle_filled(dot_pos, 2.2, egui::Color32::from_rgb(56, 189, 248));
}

pub fn render_vfo_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let f = app.rig_state.frequency_hz;

    // Nagłówek konsoli radiowej: Model radia + PTT + Ustawienia
    ui.horizontal(|ui| {
        let rig_badge = if !app.cat_rig_model.is_empty() {
            app.cat_rig_model.clone()
        } else {
            "Manual Transceiver".to_string()
        };

        if app.cat_connected {
            ui.label(egui::RichText::new(format!("📻 {}", rig_badge)).color(egui::Color32::from_rgb(56, 189, 248)).strong().size(12.0));
            ui.label(egui::RichText::new("● ONLINE").color(egui::Color32::from_rgb(34, 197, 94)).size(10.0));
        } else {
            ui.label(egui::RichText::new(format!("📻 {}", rig_badge)).color(egui::Color32::from_rgb(148, 163, 184)).size(12.0));
            ui.label(egui::RichText::new("○ OFFLINE").color(egui::Color32::from_rgb(239, 68, 68)).size(10.0));
        }

        ui.separator();

        // Przycisk PTT (nadawanie)
        let ptt_color = if app.ptt_active {
            egui::Color32::from_rgb(239, 68, 68)
        } else {
            egui::Color32::from_rgb(100, 116, 139)
        };
        let ptt_label = if app.ptt_active { "🔴 TX ON" } else { "⚫ RX" };
        if ui.button(egui::RichText::new(ptt_label).color(ptt_color).strong())
            .on_hover_text("PTT — Przełącz nadawanie/odbiór (wymaga CAT)")
            .clicked()
        {
            app.ptt_active = !app.ptt_active;
            if app.cat_connected {
                let host = app.cat_host.clone();
                let port = app.cat_port;
                let tx = app.ptt_active;
                tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    if let Ok(mut stream) = tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await {
                        let cmd = if tx { "T 1\n" } else { "T 0\n" };
                        let _ = stream.write_all(cmd.as_bytes()).await;
                    }
                });
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("⚙").on_hover_text(tr("cat.settings_tooltip", app.current_language)).clicked() {
                app.show_cat_settings_window = true;
            }
        });
    });

    ui.separator();

    // Twin VFO / Split Status
    ui.horizontal(|ui| {
        let vfo_a_active = app.rig_state.vfo != "VFOB";
        let color_a = if vfo_a_active { egui::Color32::from_rgb(34, 197, 94) } else { egui::Color32::GRAY };
        ui.label(egui::RichText::new("VFO A:").strong().color(color_a));
        ui.label(egui::RichText::new(format!("{:.3} MHz", (f as f64) / 1_000_000.0)).strong());

        ui.separator();
        let split_freq = if app.vfo_split {
            f + (app.vfo_split_offset_khz * 1000.0) as u64
        } else {
            f
        };
        let color_b = if app.vfo_split { egui::Color32::from_rgb(239, 68, 68) } else { egui::Color32::GRAY };
        ui.label(egui::RichText::new("VFO B (TX):").strong().color(color_b));
        ui.label(egui::RichText::new(format!("{:.3} MHz", (split_freq as f64) / 1_000_000.0)).strong());

        if ui.selectable_label(app.vfo_split, "SPLIT").clicked() {
            app.vfo_split = !app.vfo_split;
            app.rig_state.split_enabled = app.vfo_split;
        }

        if app.vfo_split {
            if ui.button("+1k").clicked() { app.vfo_split_offset_khz = 1.0; }
            if ui.button("+5k").clicked() { app.vfo_split_offset_khz = 5.0; }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("⇄ A/B").on_hover_text("Zamień VFO A i VFO B").clicked() {
                // Zamiana VFO
                let current_vfo = if app.rig_state.vfo == "VFOA" { "VFOB" } else { "VFOA" };
                app.rig_state.vfo = current_vfo.to_string();
            }
        });
    });

    ui.separator();

    // Rozbicie częstotliwości na poszczególne cyfry
    let d_100m = (f / 100_000_000) % 10;
    let d_10m = (f / 10_000_000) % 10;
    let d_1m = (f / 1_000_000) % 10;
    let d_100k = (f / 100_000) % 10;
    let d_10k = (f / 10_000) % 10;
    let d_1k = (f / 1_000) % 10;
    let d_100h = (f / 100) % 10;
    let d_10h = (f / 10) % 10;
    let d_1h = f % 10;

    let dim_100m = f < 100_000_000;
    let dim_10m = f < 10_000_000;

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
            ui.label(egui::RichText::new("MHz").size(15.0).monospace().color(egui::Color32::from_rgb(148, 163, 184)));

            ui.add_space(8.0);

            // Bezpośrednie wpisanie częstotliwości (kHz)
            ui.label(egui::RichText::new("kHz:").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
            let mut freq_khz_str = format!("{:.3}", (f as f64) / 1000.0);
            let resp_txt = ui.add(egui::TextEdit::singleline(&mut freq_khz_str).desired_width(75.0));
            if resp_txt.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Ok(khz) = freq_khz_str.parse::<f64>() {
                    let new_hz = (khz * 1000.0) as u64;
                    app.set_vfo_frequency(new_hz);
                }
            }
        });

        // Przyciski krokowe VFO (+/-) i tryby emisji
        ui.horizontal(|ui| {
            if ui.button("-10k").clicked() { app.step_vfo(-10_000); }
            if ui.button("-1k").clicked() { app.step_vfo(-1_000); }
            if ui.button("-100").clicked() { app.step_vfo(-100); }
            if ui.button("+100").clicked() { app.step_vfo(100); }
            if ui.button("+1k").clicked() { app.step_vfo(1_000); }
            if ui.button("+10k").clicked() { app.step_vfo(10_000); }

            ui.separator();
            for m in &["CW", "LSB", "USB", "FT8", "FT4", "RTTY", "AM", "FM"] {
                let is_sel = app.rig_state.mode == *m;
                if ui.selectable_label(is_sel, *m).clicked() {
                    app.set_vfo_mode(m);
                }
            }
        });

        // Szybki wybór pasm KF/VHF
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(tr("vfo.band_label", app.current_language)).size(11.0).strong().color(egui::Color32::from_rgb(148, 163, 184)));
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
                    app.bandmap_selected_band = b_name.to_string();
                }
            }
        });

        // Pasek DSP / Filtrów / AGC
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tr("vfo.filters", app.current_language)).size(10.0).strong().color(egui::Color32::from_rgb(148, 163, 184)));
            if ui.selectable_label(app.vfo_filter_preset == "FIL1", "FIL1 (3k)").clicked() {
                app.vfo_filter_preset = "FIL1".to_string();
                app.rig_state.passband_hz = 3000;
            }
            if ui.selectable_label(app.vfo_filter_preset == "FIL2", "FIL2 (2.4k)").clicked() {
                app.vfo_filter_preset = "FIL2".to_string();
                app.rig_state.passband_hz = 2400;
            }
            if ui.selectable_label(app.vfo_filter_preset == "FIL3", "FIL3 (500Hz)").clicked() {
                app.vfo_filter_preset = "FIL3".to_string();
                app.rig_state.passband_hz = 500;
            }

            ui.separator();
            ui.label(egui::RichText::new("RF:").size(10.0).strong().color(egui::Color32::from_rgb(148, 163, 184)));
            for pre in &["OFF", "PRE1", "PRE2", "ATT"] {
                if ui.selectable_label(app.vfo_preamp_att == *pre, *pre).clicked() {
                    app.vfo_preamp_att = pre.to_string();
                }
            }

            ui.separator();
            if ui.selectable_label(app.vfo_nb_nr, "NB/NR").clicked() {
                app.vfo_nb_nr = !app.vfo_nb_nr;
            }

            ui.separator();
            ui.label(egui::RichText::new("AGC:").size(10.0).strong().color(egui::Color32::from_rgb(148, 163, 184)));
            for agc in &["FAST", "MID", "SLOW"] {
                if ui.selectable_label(app.vfo_agc_speed == *agc, *agc).clicked() {
                    app.vfo_agc_speed = agc.to_string();
                }
            }
        });
    });

    ui.add_space(4.0);

    // Klastrowe wskaźniki Mierników (S-Meter + Moc + SWR)
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("S-METER:").size(11.0).strong().color(egui::Color32::from_rgb(148, 163, 184)));
        ui.label(egui::RichText::new(&app.rig_state.s_meter_unit).size(12.0).strong().color(egui::Color32::from_rgb(250, 204, 21)));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let pwr = if app.ptt_active { app.my_station.power_watts as f32 } else { 0.0 };
            let swr = if app.ptt_active { 1.2 } else { 1.0 };
            ui.label(egui::RichText::new(format!("PWR: {:.0} W | SWR: {:.1}", pwr, swr)).size(11.0).monospace().color(egui::Color32::from_rgb(56, 189, 248)));
        });
    });

    let raw_db = app.rig_state.s_meter_dbm;
    let meter_fraction = ((raw_db + 54.0) / 114.0).clamp(0.0, 1.0);

    let (rect, _response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 12.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(15, 23, 42));
        let filled_width = rect.width() * meter_fraction;
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
        let fill_color = if meter_fraction > 0.65 {
            egui::Color32::from_rgb(239, 68, 68) // Czerwony dla > S9
        } else {
            egui::Color32::from_rgb(34, 197, 94)  // Zielony dla S1..S9
        };
        painter.rect_filled(fill_rect, 3.0, fill_color);
    }

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("S1     S3     S5     S7     S9    +20   +40   +60dB").size(9.0).monospace().color(egui::Color32::from_rgb(100, 116, 139)));
    });

    ui.separator();

    // Sterownik Rotora Anteny
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("🧭 ROTOR: {:.0}°", app.rotor_state.azimuth_deg)).size(13.0).strong().color(egui::Color32::from_rgb(251, 191, 36)));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(egui::RichText::new("STOP").color(egui::Color32::from_rgb(239, 68, 68)).strong()).clicked() {
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
