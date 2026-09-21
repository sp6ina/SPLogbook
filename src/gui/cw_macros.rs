// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::gui::app::SpLogApp;
use eframe::egui;

pub fn render_cw_macros_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_cw_window {
        return;
    }

    let mut open = app.show_cw_window;
    let mut close_req = false;
    let mut macro_to_transmit: Option<String> = None;

    egui::Window::new("⚡ Makra Telegraficzne CW & Kluczowanie (WinKeyer / COM)")
        .open(&mut open)
        .default_size([540.0, 400.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Prędkość nadawania (WPM):");
                    ui.add(egui::Slider::new(&mut app.cw_wpm, 10..=50).text("WPM"));
                });

                ui.separator();
                ui.label(egui::RichText::new("KLAWISZE FUNKCYJNE (F1 - F8):").strong().color(egui::Color32::from_rgb(56, 189, 248)));

                render_macro_row(ui, "F1 (CQ)", &mut app.cw_macro_f1, &mut macro_to_transmit);
                render_macro_row(ui, "F2 (Raport)", &mut app.cw_macro_f2, &mut macro_to_transmit);
                render_macro_row(ui, "F3 (TU/73)", &mut app.cw_macro_f3, &mut macro_to_transmit);
                render_macro_row(ui, "F4 (Mój znak)", &mut app.cw_macro_f4, &mut macro_to_transmit);
                render_macro_row(ui, "F5 (Jego znak)", &mut app.cw_macro_f5, &mut macro_to_transmit);
                render_macro_row(ui, "F6 (QTH/Lokator)", &mut app.cw_macro_f6, &mut macro_to_transmit);
                render_macro_row(ui, "F7 (Numer STX)", &mut app.cw_macro_f7, &mut macro_to_transmit);
                render_macro_row(ui, "F8 (Znak zapytania)", &mut app.cw_macro_f8, &mut macro_to_transmit);

                ui.separator();
                ui.label("Zmienne w makrach: %MYCALL% = Znak stacji, %HISCALL% = Znak korespondenta, %RST% = Raport, %SERIAL% = Nr STX");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Zamknij").clicked() {
                        close_req = true;
                    }
                });
            });
        });

    if close_req {
        open = false;
    }
    app.show_cw_window = open;

    if let Some(text) = macro_to_transmit {
        app.transmit_cw_macro(&text);
    }
}

fn render_macro_row(ui: &mut egui::Ui, label: &str, macro_text: &mut String, macro_to_transmit: &mut Option<String>) {
    ui.horizontal(|ui| {
        let btn = ui.button(egui::RichText::new(label).strong().color(egui::Color32::from_rgb(251, 191, 36)));
        if btn.clicked() {
            *macro_to_transmit = Some(macro_text.clone());
        }
        ui.add(egui::TextEdit::singleline(macro_text).desired_width(ui.available_width()));
    });
}
