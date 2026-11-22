// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::state::Tile;
use crate::gui::theme::ThemeExt;
use eframe::egui;

pub fn render_cw_macros_window(app: &mut SpLogApp, ctx: &egui::Context) {
    let lang = app.gui.appearance.language;
    let window = egui::Window::new(tr(key::CW_MACROS_TITLE, lang)).default_size([540.0, 400.0]);
    app.show_module_window(ctx, Tile::CwMacros, window, render_cw_macros_body);
}

/// Treść modułu makr CW: okno albo kafelek pulpitu.
pub fn render_cw_macros_body(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let mut close_req = false;
    let mut macro_to_transmit: Option<String> = None;

    let mut config_changed = false;
    let lang = app.gui.appearance.language;

    let palette = ui.palette();
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(tr(key::CW_SPEED_LABEL, lang));
            let mut wpm = app.config.cw.keyer.wpm;
            let s_resp = ui.add(egui::Slider::new(&mut wpm, 5..=60).text("WPM"));
            if s_resp.changed() {
                app.config.cw.keyer.wpm = wpm;
                if let Some(keyer) = app.gui.dialogs.cw_terminal_dialog.keyer.as_ref() {
                    keyer.send(crate::cat::cw_keyer::CwCommand::SetSpeed(wpm));
                }
            }
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

        let labels = [
            key::CW_MACRO_F1_LABEL,
            key::CW_MACRO_F2_LABEL,
            key::CW_MACRO_F3_LABEL,
            key::CW_MACRO_F4_LABEL,
            key::CW_MACRO_F5_LABEL,
            key::CW_MACRO_F6_LABEL,
            key::CW_MACRO_F7_LABEL,
            key::CW_MACRO_F8_LABEL,
        ];
        for (label, macro_text) in labels.iter().zip(app.cw.macros.iter_mut()) {
            render_macro_row(
                ui,
                tr(*label, lang),
                macro_text,
                &mut macro_to_transmit,
                &mut config_changed,
            );
        }

        ui.separator();
        ui.label(tr(key::CW_MACROS_LEGEND, lang));
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                close_req = true;
            }
        });
    });

    if config_changed {
        app.save_station_config();
    }

    if close_req {
        app.close_tile(Tile::CwMacros.id());
    }

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
        if resp.lost_focus() {
            *config_changed = true;
        }
    });
}
