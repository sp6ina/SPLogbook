// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno wysyłania własnego spotu do DX Clustera (Send Telnet Spot)

use eframe::egui;

#[derive(Debug, Clone)]
pub struct SpotSubmission {
    pub dx_call: String,
    pub freq_khz: f64,
    pub comment: String,
}

pub struct SendSpotDialog {
    pub is_open: bool,
    pub dx_call: String,
    pub freq_khz: f64,
    pub comment: String,
    pub status_message: Option<String>,
}

impl Default for SendSpotDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            dx_call: String::new(),
            freq_khz: 14025.0,
            comment: "TNX for QSO! 73!".to_string(),
            status_message: None,
        }
    }
}

pub static QUICK_SPOT_COMMENTS: &[&str] = &[
    "TNX for QSO! 73!",
    "CQ DX UP 2 kHz",
    "599 in JO81 (Poland)",
    "SPLIT 1.5 kHz UP",
    "Loud signal 59+ in Europe",
    "NEW ONE! TNX fer new slot!",
    "Listening UP 1",
    "Strong signal on dipol",
];

impl SendSpotDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.status_message = None;
    }

    pub fn open_with(&mut self, call: &str, freq_khz: f64) {
        self.dx_call = call.trim().to_uppercase();
        self.freq_khz = freq_khz;
        self.is_open = true;
        self.status_message = None;
    }

    pub fn open_for_station(&mut self, call: &str, freq_hz: u64) {
        self.dx_call = call.trim().to_uppercase();
        self.freq_khz = (freq_hz as f64) / 1000.0;
        self.is_open = true;
        self.status_message = None;
    }

    pub fn show(&mut self, ctx: &egui::Context, _my_call: &str, lang: crate::core::i18n::Language) -> Option<SpotSubmission> {
        if !self.is_open {
            return None;
        }

        use crate::core::i18n::tr;
        let mut open = self.is_open;
        let mut close_requested = false;
        let mut submission = None;

        egui::Window::new(tr("cluster.send_spot_title", lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.heading(tr("cluster.spot_dialog_sub", lang));
                ui.separator();

                egui::Grid::new("send_spot_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(tr("cluster.spot_dx_call", lang));
                        ui.add(egui::TextEdit::singleline(&mut self.dx_call).desired_width(160.0));
                        ui.end_row();

                        ui.label(tr("cluster.spot_freq", lang));
                        ui.add(egui::DragValue::new(&mut self.freq_khz).speed(0.5).suffix(" kHz"));
                        ui.end_row();

                        ui.label(tr("cluster.spot_comment", lang));
                        ui.add(egui::TextEdit::singleline(&mut self.comment).desired_width(260.0));
                        ui.end_row();
                    });

                ui.add_space(4.0);
                ui.label(tr("cluster.quick_templates", lang));
                ui.horizontal_wrapped(|ui| {
                    for &tmpl in QUICK_SPOT_COMMENTS {
                        if ui.button(tmpl).clicked() {
                            self.comment = tmpl.to_string();
                        }
                    }
                });

                if let Some(ref msg) = self.status_message {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(msg).small().color(egui::Color32::from_rgb(52, 211, 153)));
                }

                ui.separator();
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(egui::RichText::new(tr("cluster.send_btn", lang)).strong().color(egui::Color32::from_rgb(15, 23, 42)))
                        .fill(egui::Color32::from_rgb(56, 189, 248));

                    if ui.add(btn).clicked()
                        && !self.dx_call.trim().is_empty() {
                            submission = Some(SpotSubmission {
                                dx_call: self.dx_call.clone(),
                                freq_khz: self.freq_khz,
                                comment: self.comment.clone(),
                            });
                            self.status_message = Some(format!("Spot dla {} ({:.1} kHz) OK!", self.dx_call, self.freq_khz));
                            close_requested = true;
                        }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr("btn.cancel", lang)).clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

        if close_requested || !open {
            self.is_open = false;
        }

        submission
    }
}
