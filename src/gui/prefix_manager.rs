// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno bazy prefiksów i znaków specjalnych (UniqueCalls z serviceLOG.db - 4 310 rekordów)

use crate::core::service_db::{ServiceDatabase, UniqueCallRecord};
use eframe::egui;

pub struct PrefixManagerDialog {
    pub is_open: bool,
    pub search_query: String,
    pub results: Vec<UniqueCallRecord>,
}

impl Default for PrefixManagerDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl PrefixManagerDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            search_query: String::new(),
            results: Vec::new(),
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn execute_search(&mut self, db: &ServiceDatabase) {
        if self.search_query.trim().is_empty() {
            self.results.clear();
        } else {
            self.results = db.search_unique_calls(&self.search_query);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, db: &ServiceDatabase) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new("🌐 Menedżer Prefiksów i Znaków Specjalnych (serviceLOG)")
            .open(&mut open)
            .default_width(700.0)
            .default_height(450.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Szukaj znaku lub kraju:");
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(220.0)
                            .hint_text("np. 3Y0, VP8, DP0...")
                    );
                    if edit.changed() || (ui.input(|i| i.key_pressed(egui::Key::Enter)) && edit.has_focus()) {
                        self.search_query = self.search_query.to_uppercase();
                        self.execute_search(db);
                    }

                    if ui.button("🔍 Szukaj").clicked() {
                        self.execute_search(db);
                    }

                    if ui.button("Wyczyść").clicked() {
                        self.search_query.clear();
                        self.results.clear();
                    }
                });

                ui.separator();
                ui.label(format!("Znaleziono unikalnych wpisów: {}", self.results.len()));

                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    egui::Grid::new("prefix_mgr_grid").striped(true).spacing([15.0, 6.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("Znak / Prefiks").strong());
                        ui.label(egui::RichText::new("Kraj / DXCC").strong());
                        ui.label(egui::RichText::new("Prefiks ARRL").strong());
                        ui.label(egui::RichText::new("Kontynent").strong());
                        ui.label(egui::RichText::new("CQ Strefa").strong());
                        ui.label(egui::RichText::new("ITU Strefa").strong());
                        ui.end_row();

                        for rec in &self.results {
                            ui.label(egui::RichText::new(&rec.call).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                            ui.label(format!("{} (DXCC #{})", rec.country, rec.dxcc));
                            ui.label(&rec.arrl_prefix);
                            ui.label(&rec.continent);
                            ui.label(rec.cq_zone.to_string());
                            ui.label(rec.itu_zone.to_string());
                            ui.end_row();
                        }
                    });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Baza zawiera 4 310 unikalnych reguł DXCC dla znaków nieregularnych i ekspedycji.").small().color(egui::Color32::GRAY));
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
    }
}
