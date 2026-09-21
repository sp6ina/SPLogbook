// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Interaktywny terminal telegraficzny CW z buforem i obsługą Winkeyera / portu COM

use crate::cat::winkeyer::{CwTerminalBuffer, WinkeyerConfig};
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
            status_text: "Gotowy do nadawania. Wpisz tekst i naciśnij Enter.".to_string(),
        }
    }
}

impl CwTerminalDialog {
    pub fn render(&mut self, ctx: &egui::Context, my_callsign: &str) {
        if !self.is_open {
            return;
        }

        // Obsługa klawisza ESC do natychmiastowego przerwania (Abort)
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.buffer.abort();
            self.status_text = "⚠ Nadawanie przerwane (ESC / Abort)!".to_string();
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        egui::Window::new("📟 Terminal Nadawania CW (Klawiatura)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Prędkość CW:");
                    let mut wpm = self.buffer.current_wpm;
                    if ui.add(egui::Slider::new(&mut wpm, 5..=50).text("WPM")).changed() {
                        self.buffer.current_wpm = wpm;
                        self.config.speed_wpm = wpm;
                    }

                    ui.separator();
                    ui.label("Port klucza:");
                    ui.text_edit_singleline(&mut self.config.port);

                    if ui.button(if self.is_connected { "Odłącz" } else { "Połącz" }).clicked() {
                        self.is_connected = !self.is_connected;
                        self.status_text = if self.is_connected {
                            format!("Połączono z kluczem na porcie {}", self.config.port)
                        } else {
                            "Rozłączono z portem klucza".to_string()
                        };
                    }
                });

                ui.separator();

                // Makra telegraficzne F1..F8
                ui.horizontal_wrapped(|ui| {
                    if ui.button("F1: CQ").clicked() {
                        let msg = format!("CQ CQ DE {} K ", my_callsign);
                        self.buffer.append_to_queue(&msg);
                    }
                    if ui.button("F2: TEST").clicked() {
                        let msg = format!("TEST {} ", my_callsign);
                        self.buffer.append_to_queue(&msg);
                    }
                    if ui.button("F3: 599 TU").clicked() {
                        self.buffer.append_to_queue("5NN TU ");
                    }
                    if ui.button("F4: 73").clicked() {
                        let msg = format!("73 EE {} ", my_callsign);
                        self.buffer.append_to_queue(&msg);
                    }
                    if ui.button("F5: ZNAK").clicked() {
                        let msg = format!("{} ", my_callsign);
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
                ui.label(egui::RichText::new("Podgląd bufora transmisji:").strong());
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        if !self.buffer.sent_text.is_empty() {
                            ui.label(egui::RichText::new(&self.buffer.sent_text).color(egui::Color32::from_rgb(120, 220, 120)));
                        }
                        if !self.buffer.queued_text.is_empty() {
                            ui.label(egui::RichText::new(&self.buffer.queued_text).color(egui::Color32::from_rgb(255, 200, 50)).strong());
                        }
                        if self.buffer.sent_text.is_empty() && self.buffer.queued_text.is_empty() {
                            ui.label(egui::RichText::new("[Bufor pusty]").color(egui::Color32::GRAY).italics());
                        }
                    });
                });

                ui.add_space(6.0);
                ui.label("Pole wpisywania (Enter wysyła do bufora):");
                let response = ui.text_edit_singleline(&mut self.input_text);
                if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.input_text.trim().is_empty() {
                        let to_send = format!("{} ", self.input_text.trim().to_uppercase());
                        self.buffer.append_to_queue(&to_send);
                        self.status_text = format!("Dodano do bufora: {}", to_send);
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
                        if ui.button("Wyczyść historię").clicked() {
                            self.buffer.sent_text.clear();
                        }
                        if ui.button("Zamknij").clicked() {
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
