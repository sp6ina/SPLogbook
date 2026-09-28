// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno przeglądarki zdjęć stacji (z QRZ.com / HamQTH / bazy lokalnej)

use eframe::egui;

pub struct PhotoViewerDialog {
    pub is_open: bool,
    pub callsign: String,
    pub photo_url: Option<String>,
}

impl Default for PhotoViewerDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl PhotoViewerDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            callsign: String::new(),
            photo_url: None,
        }
    }

    pub fn open(&mut self, callsign: &str, photo_url: Option<String>) {
        self.callsign = callsign.to_uppercase();
        self.photo_url = photo_url;
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, lang: crate::core::i18n::Language) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new(format!("📷 {}: {}", crate::core::i18n::tr("qso.callsign", lang), self.callsign))
            .open(&mut open)
            .default_width(450.0)
            .default_height(350.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(format!("{}: {}", crate::core::i18n::tr("qso.callsign", lang), self.callsign));

                    if let Some(ref url) = self.photo_url {
                        ui.label(format!("Adres grafiki: {url}"));
                        ui.add_space(8.0);

                        ui.add(
                            egui::Image::new(url)
                                .max_width(400.0)
                                .max_height(300.0)
                                .corner_radius(6.0),
                        );

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button(crate::core::i18n::tr("photo.open_browser", lang)).clicked() {
                                ctx.open_url(egui::OpenUrl::new_tab(url));
                            }
                            if ui.button(crate::core::i18n::tr("photo.copy_link", lang)).clicked() {
                                ui.ctx().copy_text(url.clone());
                            }
                        });
                    } else {
                        ui.add_space(30.0);
                        ui.label(
                            egui::RichText::new(
                                "Brak powiązanego zdjęcia dla tej stacji w QRZ / HamQTH.",
                            )
                            .italics()
                            .color(egui::Color32::GRAY),
                        );
                        ui.add_space(20.0);
                        let qrz_link = format!("https://www.qrz.com/db/{}", self.callsign);
                        if ui.button(crate::core::i18n::tr("photo.check_qrz", lang)).clicked() {
                            ctx.open_url(egui::OpenUrl::new_tab(qrz_link));
                        }
                    }

                    ui.add_space(10.0);
                    if ui.button(crate::core::i18n::tr("btn.close", lang)).clicked() {
                        close_requested = true;
                    }
                });
            });

        if close_requested || !open {
            self.is_open = false;
        }
    }
}
