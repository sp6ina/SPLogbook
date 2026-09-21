// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Przeglądarka i wyszukiwarka grup wysp IOTA (Islands on the Air) z bazy serviceLOG.db

use crate::core::service_db::{IotaRecord, ServiceDatabase};
use eframe::egui;

#[derive(Default)]
pub struct IotaBrowserDialog {
    pub is_open: bool,
    pub search_query: String,
    pub results: Vec<IotaRecord>,
}


impl IotaBrowserDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn reload(&mut self, sdb: &ServiceDatabase) {
        self.results = sdb.search_iota(&self.search_query);
    }

    pub fn show(&mut self, ctx: &egui::Context, sdb: &ServiceDatabase) -> Option<String> {
        if !self.is_open {
            return None;
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        let mut chosen = None;

        egui::Window::new("🏝 Przeglądarka Wysp Świata IOTA (Islands On The Air)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(580.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Szukaj IOTA (kod np. EU-001 lub nazwa wyspy):");
                    let resp = ui.add(egui::TextEdit::singleline(&mut self.search_query).desired_width(200.0));
                    if resp.changed() || (self.results.is_empty() && self.search_query.is_empty()) {
                        self.reload(sdb);
                    }
                    if ui.button("Wyczyść").clicked() {
                        self.search_query.clear();
                        self.reload(sdb);
                    }
                });

                ui.separator();

                ui.label(format!("Wyniki ({}) - baza serviceLOG:", self.results.len()));

                egui::ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                    egui::Grid::new("iota_results_grid")
                        .striped(true)
                        .spacing([15.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("IOTA Kod").strong());
                            ui.label(egui::RichText::new("Prefiks").strong());
                            ui.label(egui::RichText::new("Nazwa grupy wysp").strong());
                            ui.label(egui::RichText::new("Akcja").strong());
                            ui.end_row();

                            for item in &self.results {
                                ui.label(egui::RichText::new(&item.iota).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                                ui.label(egui::RichText::new(&item.prefix).color(egui::Color32::from_rgb(251, 191, 36)));
                                ui.label(&item.name);

                                if ui.button("Wybierz").on_hover_text("Użyj tej referencji IOTA").clicked() {
                                    chosen = Some(item.iota.clone());
                                    close_requested = true;
                                }
                                ui.end_row();
                            }
                        });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Baza zawiera 1 190 oficjalnych grup wysp programu RSGB IOTA.").small().color(egui::Color32::GRAY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Zamknij").clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

        if close_requested || !open {
            self.is_open = false;
        }

        chosen
    }
}
