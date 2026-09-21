// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno przeglądarki managerów QSL (z bazy serviceLOG.db - 29 102 rekordów)

use crate::core::service_db::{QslManagerRecord, ServiceDatabase};
use eframe::egui;

pub struct QslManagerDialog {
    pub is_open: bool,
    pub search_query: String,
    pub results: Vec<QslManagerRecord>,
    pub selected_manager: Option<String>,
}

impl Default for QslManagerDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl QslManagerDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            search_query: String::new(),
            results: Vec::new(),
            selected_manager: None,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn open_for_call(&mut self, call: &str, db: &ServiceDatabase) {
        self.search_query = call.trim().to_uppercase();
        self.is_open = true;
        self.execute_search(db);
    }

    pub fn execute_search(&mut self, db: &ServiceDatabase) {
        if self.search_query.trim().is_empty() {
            self.results.clear();
        } else {
            self.results = db.search_managers(&self.search_query);
        }
    }

    /// Rysuje okno wyszukiwarki managerów QSL.
    /// Zwraca Some(manager) jeśli użytkownik kliknął "Wybierz / Użyj".
    pub fn show(&mut self, ctx: &egui::Context, db: &ServiceDatabase) -> Option<String> {
        if !self.is_open {
            return None;
        }

        let mut chosen = None;
        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new("📋 Baza Managerów QSL (serviceLOG)")
            .open(&mut open)
            .default_width(650.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Szukaj znaku lub managera:");
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(220.0)
                            .hint_text("np. 3B8, VP8, SP6...")
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

                ui.label(format!("Znaleziono rekordów: {}", self.results.len()));

                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    egui::Grid::new("qsl_mgr_grid").striped(true).spacing([15.0, 6.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("Znak stacji").strong());
                        ui.label(egui::RichText::new("Manager QSL (Via)").strong());
                        ui.label(egui::RichText::new("Lata / Okres").strong());
                        ui.label(egui::RichText::new("Uwagi").strong());
                        ui.label(egui::RichText::new("Akcja").strong());
                        ui.end_row();

                        for rec in &self.results {
                            ui.label(egui::RichText::new(&rec.call).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                            ui.label(egui::RichText::new(&rec.manager).strong().color(egui::Color32::from_rgb(34, 197, 94)));
                            ui.label(&rec.years);
                            ui.label(&rec.notes);

                            if ui.button("Wybierz").on_hover_text("Użyj tego managera w formularzu QSO").clicked() {
                                chosen = Some(rec.manager.clone());
                                close_requested = true;
                            }
                            ui.end_row();
                        }
                    });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Baza serviceLOG zawiera ponad 29 000 powiązań stacji z managerami QSL.").small().color(egui::Color32::GRAY));
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
