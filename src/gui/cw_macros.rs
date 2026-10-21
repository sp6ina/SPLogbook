// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

pub fn render_cw_macros_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.cw.show_window {
        return;
    }

    let mut open = app.cw.show_window;
    let mut close_req = false;
    let mut macro_to_transmit: Option<String> = None;

    let mut config_changed = false;
    let lang = app.current_language;

    egui::Window::new(tr(key::CW_MACROS_TITLE, lang))
        .open(&mut open)
        .default_size([540.0, 400.0])
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr(key::CW_SPEED_LABEL, lang));
                    let s_resp = ui.add(egui::Slider::new(&mut app.cw.wpm, 10..=50).text("WPM"));
                    if s_resp.drag_stopped() || (s_resp.changed() && !s_resp.dragged()) {
                        config_changed = true;
                    }
                });

                ui.separator();
                ui.label(
                    egui::RichText::new(tr(key::CW_FN_KEYS_HEADING, lang))
                        .strong()
                        .color(palette.accent),
                );

                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F1_LABEL, lang),
                    &mut app.cw.macro_f1,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );
                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F2_LABEL, lang),
                    &mut app.cw.macro_f2,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );
                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F3_LABEL, lang),
                    &mut app.cw.macro_f3,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );
                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F4_LABEL, lang),
                    &mut app.cw.macro_f4,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );
                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F5_LABEL, lang),
                    &mut app.cw.macro_f5,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );
                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F6_LABEL, lang),
                    &mut app.cw.macro_f6,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );
                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F7_LABEL, lang),
                    &mut app.cw.macro_f7,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );
                render_macro_row(
                    ui,
                    tr(key::CW_MACRO_F8_LABEL, lang),
                    &mut app.cw.macro_f8,
                    &mut macro_to_transmit,
                    &mut config_changed,
                );

                ui.separator();
                ui.label(tr(key::CW_MACROS_LEGEND, lang));
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_req = true;
                    }
                });
            });
        });

    if config_changed {
        app.save_station_config();
    }

    if close_req {
        open = false;
    }
    app.cw.show_window = open;

    if let Some(text) = macro_to_transmit {
        app.transmit_cw_macro(&text);
    }
}

fn render_macro_row(
    ui: &mut egui::Ui,
    label: &str,
    macro_text: &mut String,
    macro_to_transmit: &mut Option<String>,
    config_changed: &mut bool,
) {
    ui.horizontal(|ui| {
        let btn = ui.button(
            egui::RichText::new(label)
                .strong()
                .color(ui.palette().warning),
        );
        if btn.clicked() {
            *macro_to_transmit = Some(macro_text.clone());
        }
        let resp =
            ui.add(egui::TextEdit::singleline(macro_text).desired_width(ui.available_width()));
        if resp.lost_focus() || resp.changed() {
            *config_changed = true;
        }
    });
}
