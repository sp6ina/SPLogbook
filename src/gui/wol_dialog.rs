// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno zdalnego włączania komputera stacji (Wake-on-LAN)

use crate::core::i18n::{Language, key, tr};
use crate::gui::theme::ThemeExt;
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

        egui::Window::new(tr(key::WOL_WINDOW_TITLE, lang))
            .open(&mut open)
            .default_width(450.0)
            .default_height(240.0)
            .show(ctx, |ui| {
                let palette = ui.palette();
                ui.label(tr(key::WOL_DESCRIPTION, lang));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(tr(key::WOL_MAC_LABEL, lang));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.mac_address)
                            .hint_text(tr(key::WOL_MAC_HINT, lang))
                            .desired_width(180.0),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(tr(key::WOL_BROADCAST_LABEL, lang));
                    ui.add(egui::TextEdit::singleline(&mut self.broadcast_ip).desired_width(140.0));
                    ui.label(tr(key::WOL_PORT_LABEL, lang));
                    ui.add(egui::DragValue::new(&mut self.port));
                });

                if let Some(ref st) = self.status {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(st).color(palette.success).strong());
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new(tr(key::WOL_SEND_BTN, lang))
                            .strong()
                            .color(palette.selection_text),
                    )
                    .fill(palette.accent);

                    if ui.add(btn).clicked() {
                        let target = format!("{}:{}", self.broadcast_ip, self.port);
                        match WolClient::send_wol(&self.mac_address, Some(&target)) {
                            Ok(()) => {
                                self.status = Some(format!(
                                    "{} -> {}",
                                    tr(key::WOL_SUCCESS, lang),
                                    self.mac_address
                                ));
                            }
                            Err(e) => {
                                self.status = Some(format!("{} {}", tr(key::WOL_ERROR, lang), e));
                            }
                        }
                    }

                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_requested = true;
                    }
                });
            });

        if close_requested || !open {
            self.is_open = false;
        }
    }
}
