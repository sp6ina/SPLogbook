// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno zdalnego włączania komputera stacji (Wake-on-LAN)

use crate::core::i18n::{tr, Language};
use crate::network::wol::WolClient;
use eframe::egui;

pub struct WolDialog {
    pub is_open: bool,
    pub mac_address: String,
    pub broadcast_ip: String,
    pub port: u16,
    pub status: Option<String>,
}

impl Default for WolDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl WolDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            mac_address: String::new(),
            broadcast_ip: "255.255.255.255".to_string(),
            port: 9,
            status: None,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, lang: Language) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new(tr("wol.window_title", lang))
            .open(&mut open)
            .default_width(450.0)
            .default_height(240.0)
            .show(ctx, |ui| {
                ui.label(tr("wol.description", lang));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(tr("wol.mac_label", lang));
                    ui.add(egui::TextEdit::singleline(&mut self.mac_address).hint_text("np. AA:BB:CC:DD:EE:FF").desired_width(180.0));
                });

                ui.horizontal(|ui| {
                    ui.label(tr("wol.broadcast_label", lang));
                    ui.add(egui::TextEdit::singleline(&mut self.broadcast_ip).desired_width(140.0));
                    ui.label(tr("wol.port_label", lang));
                    ui.add(egui::DragValue::new(&mut self.port));
                });

                if let Some(ref st) = self.status {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(st).color(egui::Color32::from_rgb(52, 211, 153)).strong());
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(egui::RichText::new(tr("wol.send_btn", lang)).strong().color(egui::Color32::BLACK))
                        .fill(egui::Color32::from_rgb(250, 204, 21));

                    if ui.add(btn).clicked() {
                        let target = format!("{}:{}", self.broadcast_ip, self.port);
                        match WolClient::send_wol(&self.mac_address, Some(&target)) {
                            Ok(_) => {
                                self.status = Some(format!("{} -> {}", tr("wol.success", lang), self.mac_address));
                            }
                            Err(e) => {
                                self.status = Some(format!("{} {}", tr("wol.error", lang), e));
                            }
                        }
                    }

                    if ui.button(tr("btn.close", lang)).clicked() {
                        close_requested = true;
                    }
                });
            });

        if close_requested || !open {
            self.is_open = false;
        }
    }
}
