// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno eksportu łączności do oficjalnego formatu SOTA CSV V2

use crate::core::qso::QsoRecord;
use crate::core::sota_export::SotaExporter;
use eframe::egui;

pub struct SotaDialog {
    pub is_open: bool,
    pub activator_call: String,
    pub summit_ref: String,
    pub filter_today_only: bool,
    pub status: Option<String>,
}

impl Default for SotaDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl SotaDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            activator_call: String::new(),
            summit_ref: "SP/BZ-001".to_string(),
            filter_today_only: false,
            status: None,
        }
    }

    pub fn open(&mut self, default_call: &str) {
        if self.activator_call.is_empty() {
            self.activator_call = default_call.to_uppercase();
        }
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, qsos: &[QsoRecord]) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new("🏔 Eksport Łączności SOTA (Summits on the Air V2 CSV)")
            .open(&mut open)
            .default_width(480.0)
            .default_height(260.0)
            .show(ctx, |ui| {
                ui.label("Eksportuj łączności do oficjalnego formatu bazy danych SOTA (sotadata.org.uk) w formacie CSV V2.");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Twój znak aktywatora:");
                    ui.add(egui::TextEdit::singleline(&mut self.activator_call).desired_width(120.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Referencja szczytu (My Summit):");
                    ui.add(egui::TextEdit::singleline(&mut self.summit_ref).hint_text("np. SP/BZ-001, OE/TI-001").desired_width(140.0));
                });

                ui.checkbox(&mut self.filter_today_only, "Eksportuj tylko łączności z dzisiejszego dnia");

                if let Some(ref st) = self.status {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(st).color(egui::Color32::from_rgb(56, 189, 248)).strong());
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("💾 Eksportuj do pliku CSV...").strong()).clicked() {
                        let filtered: Vec<_> = if self.filter_today_only {
                            let today = chrono::Utc::now().format("%Y%m%d").to_string();
                            let today_dash = chrono::Utc::now().format("%Y-%m-%d").to_string();
                            qsos.iter().filter(|q| q.qso_date == today || q.qso_date == today_dash).cloned().collect()
                        } else {
                            qsos.to_vec()
                        };

                        let csv_content = SotaExporter::export_csv_v2(&filtered, &self.activator_call, &self.summit_ref);

                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(format!("SOTA_{}_{}.csv", self.activator_call, self.summit_ref.replace('/', "_")))
                            .add_filter("SOTA CSV File", &["csv"])
                            .save_file()
                        {
                            match std::fs::write(&path, csv_content) {
                                Ok(_) => {
                                    self.status = Some(format!("Zapisano {} łączności w: {}", filtered.len(), path.display()));
                                }
                                Err(e) => {
                                    self.status = Some(format!("Błąd zapisu pliku: {}", e));
                                }
                            }
                        }
                    }

                    if ui.button("Zamknij").clicked() {
                        close_requested = true;
                    }
                });
            });

        if close_requested || !open {
            self.is_open = false;
        }
    }
}
