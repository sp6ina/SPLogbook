// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno przeglądarki zdjęć stacji (z QRZ.com / HamQTH / bazy lokalnej)

use crate::gui::theme::ThemeExt;
use eframe::egui;

pub struct PhotoViewerDialog {
    pub is_open: bool,
    pub callsign: String,
    pub photo_url: Option<String>,
    pub error_message: Option<String>,
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
            error_message: None,
        }
    }

    pub fn open(&mut self, callsign: &str, photo_url: Option<String>) {
        let clean_call = callsign.trim().to_uppercase();
        let is_url_empty = photo_url.as_ref().map_or(true, |u| u.trim().is_empty());

        if clean_call.is_empty() && is_url_empty {
            self.error_message = Some("Empty photo source or callsign rejected".to_string());
            self.is_open = false;
            return;
        }

        if let Some(ref url) = photo_url {
            let u = url.trim();
            if u.is_empty() {
                self.error_message = Some("Empty photo source or callsign rejected".to_string());
                self.is_open = false;
                return;
            }
            let is_web = u.starts_with("http://") || u.starts_with("https://");
            let is_file_uri = u.starts_with("file://");
            let has_image_ext = u.ends_with(".jpg")
                || u.ends_with(".jpeg")
                || u.ends_with(".png")
                || u.ends_with(".webp")
                || u.ends_with(".gif");

            if !is_web && !is_file_uri && !has_image_ext {
                self.callsign = clean_call;
                self.photo_url = None;
                self.error_message = Some(format!("Invalid photo source format: {u}"));
                self.is_open = true;
                return;
            }
        }

        self.callsign = clean_call;
        self.photo_url = photo_url.map(|u| u.trim().to_string());
        self.error_message = None;
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.callsign.clear();
        self.photo_url = None;
        self.error_message = None;
    }

    pub fn show(&mut self, ctx: &egui::Context, lang: crate::core::i18n::Language) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new(format!(
            "📷 {}: {}",
            crate::core::i18n::tr(crate::core::i18n::key::QSO_CALLSIGN, lang),
            self.callsign
        ))
        .open(&mut open)
        .default_width(450.0)
        .default_height(350.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(format!(
                    "{}: {}",
                    crate::core::i18n::tr(crate::core::i18n::key::QSO_CALLSIGN, lang),
                    self.callsign
                ));

                if let Some(ref err) = self.error_message {
                    ui.add_space(10.0);
                    ui.colored_label(ui.palette().danger, err);
                } else if let Some(ref url) = self.photo_url {
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
                        if ui
                            .button(crate::core::i18n::tr(
                                crate::core::i18n::key::PHOTO_OPEN_BROWSER,
                                lang,
                            ))
                            .clicked()
                        {
                            ctx.open_url(egui::OpenUrl::new_tab(url));
                        }
                        if ui
                            .button(crate::core::i18n::tr(
                                crate::core::i18n::key::PHOTO_COPY_LINK,
                                lang,
                            ))
                            .clicked()
                        {
                            ui.ctx().copy_text(url.clone());
                        }
                    });
                } else {
                    ui.add_space(30.0);
                    ui.label(
                        egui::RichText::new(crate::core::i18n::tr(
                            crate::core::i18n::key::PHOTO_NO_PHOTO_FOUND,
                            lang,
                        ))
                        .italics()
                        .color(ui.palette().text_muted),
                    );
                    ui.add_space(20.0);
                    let qrz_link = format!("https://www.qrz.com/db/{}", self.callsign);
                    if ui
                        .button(crate::core::i18n::tr(
                            crate::core::i18n::key::PHOTO_CHECK_QRZ,
                            lang,
                        ))
                        .clicked()
                    {
                        ctx.open_url(egui::OpenUrl::new_tab(qrz_link));
                    }
                }

                ui.add_space(10.0);
                if ui
                    .button(crate::core::i18n::tr(
                        crate::core::i18n::key::BTN_CLOSE,
                        lang,
                    ))
                    .clicked()
                {
                    close_requested = true;
                }
            });
        });

        if close_requested || !open {
            self.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_viewer_open_preserves_selected_source() {
        let mut dialog = PhotoViewerDialog::new();
        dialog.open("SP6INA", Some("https://example.com/sp6ina.jpg".to_string()));
        assert!(dialog.is_open);
        assert_eq!(dialog.callsign, "SP6INA");
        assert_eq!(
            dialog.photo_url.as_deref(),
            Some("https://example.com/sp6ina.jpg")
        );
        assert!(dialog.error_message.is_none());
    }

    #[test]
    fn photo_viewer_rejects_empty_source() {
        let mut dialog = PhotoViewerDialog::new();
        dialog.open("", None);
        assert!(!dialog.is_open);
        assert!(dialog.error_message.is_some());

        dialog.open("   ", Some("   ".to_string()));
        assert!(!dialog.is_open);
        assert!(dialog.error_message.is_some());
    }

    #[test]
    fn photo_viewer_reports_invalid_source() {
        let mut dialog = PhotoViewerDialog::new();
        dialog.open("SP6INA", Some("invalid_protocol://bad-data".to_string()));
        assert!(dialog.is_open);
        assert!(dialog.photo_url.is_none());
        assert!(dialog.error_message.is_some());
        assert!(
            dialog
                .error_message
                .unwrap()
                .contains("Invalid photo source format")
        );
    }

    #[test]
    fn photo_viewer_close_clears_transient_state_by_contract() {
        let mut dialog = PhotoViewerDialog::new();
        dialog.open("SP6INA", Some("https://example.com/qsl.png".to_string()));
        assert!(dialog.is_open);

        dialog.close();
        assert!(!dialog.is_open);
        assert!(dialog.callsign.is_empty());
        assert!(dialog.photo_url.is_none());
        assert!(dialog.error_message.is_none());
    }

    #[test]
    fn photo_viewer_test_does_not_access_user_files() {
        let dialog = PhotoViewerDialog::new();
        assert!(!dialog.is_open);
        assert!(dialog.photo_url.is_none());
    }
}
