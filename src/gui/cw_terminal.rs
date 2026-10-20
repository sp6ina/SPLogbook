// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Interaktywny terminal telegraficzny CW z buforem i obsługą Winkeyera / portu COM

use crate::cat::winkeyer::{CwTerminalBuffer, WinkeyerConfig};
use crate::core::i18n::{Msg, key, tr};
use crate::gui::theme::ThemeExt;
use eframe::egui;

pub struct CwTerminalDialog {
    pub is_open: bool,
    pub buffer: CwTerminalBuffer,
    pub input_text: String,
    pub config: WinkeyerConfig,
    pub is_connected: bool,
    pub status_text: String,
}

impl Default for CwTerminalDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            buffer: CwTerminalBuffer::new(26),
            input_text: String::new(),
            config: WinkeyerConfig::default(),
            is_connected: false,
            status_text: "Ready to transmit. Type text and press Enter.".to_string(),
        }
    }
}

impl CwTerminalDialog {
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        my_callsign: &str,
        lang: crate::core::i18n::Language,
    ) {
        if !self.is_open {
            return;
        }

        // Obsługa klawisza ESC do natychmiastowego przerwania (Abort)
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.buffer.abort();
            self.status_text = "⚠ Transmission aborted (ESC / Abort)!".to_string();
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        egui::Window::new(tr(key::CW_TERM_WINDOW_TITLE, lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr(key::CW_TERM_SPEED, lang));
                    let mut wpm = self.buffer.current_wpm;
                    if ui
                        .add(egui::Slider::new(&mut wpm, 5..=50).text("WPM"))
                        .changed()
                    {
                        self.buffer.current_wpm = wpm;
                        self.config.speed_wpm = wpm;
                    }

                    ui.separator();
                    ui.label(tr(key::CW_TERM_KEY_PORT, lang));
                    ui.text_edit_singleline(&mut self.config.port);

                    if ui
                        .button(if self.is_connected {
                            tr(key::CW_TERM_DISCONNECT, lang)
                        } else {
                            tr(key::CW_TERM_CONNECT, lang)
                        })
                        .clicked()
                    {
                        self.is_connected = !self.is_connected;
                        self.status_text = if self.is_connected {
                            Msg::new(key::CW_TERM_CONNECTED)
                                .arg("port", &self.config.port)
                                .text(lang)
                        } else {
                            tr(key::CW_TERM_DISCONNECTED, lang).to_string()
                        };
                    }
                });

                ui.separator();

                // Makra telegraficzne F1..F8
                ui.horizontal_wrapped(|ui| {
                    if ui.button("F1: CQ").clicked() {
                        let msg = format!("CQ CQ DE {my_callsign} K ");
                        self.buffer.append_to_queue(&msg);
                    }
                    if ui.button("F2: TEST").clicked() {
                        let msg = format!("TEST {my_callsign} ");
                        self.buffer.append_to_queue(&msg);
                    }
                    if ui.button("F3: 599 TU").clicked() {
                        self.buffer.append_to_queue("5NN TU ");
                    }
                    if ui.button("F4: 73").clicked() {
                        let msg = format!("73 EE {my_callsign} ");
                        self.buffer.append_to_queue(&msg);
                    }
                    if ui.button("F5: ZNAK").clicked() {
                        let msg = format!("{my_callsign} ");
                        self.buffer.append_to_queue(&msg);
                    }
                    if ui.button("F6: AGN?").clicked() {
                        self.buffer.append_to_queue("AGN? ");
                    }
                    if ui.button("F7: CFM").clicked() {
                        self.buffer.append_to_queue("CFM TU ");
                    }
                    if ui.button("⛔ ESC: PRZERWIJ (ABORT)").clicked() {
                        self.buffer.abort();
                        self.status_text = "⚠ Przerwano bufor nadawania!".to_string();
                    }
                });

                ui.separator();

                // Okno podglądu bufora nadawania (wieloliniowe)
                let palette = ui.palette();
                ui.label(egui::RichText::new(tr(key::CW_TERM_BUFFER_PREVIEW, lang)).strong());
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        if !self.buffer.sent_text.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.buffer.sent_text).color(palette.success),
                            );
                        }
                        if !self.buffer.queued_text.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.buffer.queued_text)
                                    .color(palette.warning)
                                    .strong(),
                            );
                        }
                        if self.buffer.sent_text.is_empty() && self.buffer.queued_text.is_empty() {
                            ui.label(
                                egui::RichText::new(tr(key::CW_TERM_BUFFER_EMPTY, lang))
                                    .color(palette.text_muted)
                                    .italics(),
                            );
                        }
                    });
                });

                ui.add_space(6.0);
                ui.label(tr(key::CW_TERM_INPUT_PROMPT, lang));
                let response = ui.text_edit_singleline(&mut self.input_text);
                if response.lost_focus()
                    && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.input_text.trim().is_empty()
                {
                    let to_send = format!("{} ", self.input_text.trim().to_uppercase());
                    self.buffer.append_to_queue(&to_send);
                    self.status_text = format!("Dodano do bufora: {to_send}");
                    self.input_text.clear();
                    response.request_focus();
                }

                // W tle symuluj stopniowe wysyłanie znaków
                if !self.buffer.queued_text.is_empty() {
                    // W tempie WPM przesuwaj bufor
                    self.buffer.advance_sent(1);
                    ctx.request_repaint_after(std::time::Duration::from_millis(150));
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.status_text).small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(key::CW_TERM_CLEAR_HISTORY, lang)).clicked() {
                            self.buffer.sent_text.clear();
                        }
                        if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

        if close_requested {
            open = false;
        }
        self.is_open = open;
    }
}
