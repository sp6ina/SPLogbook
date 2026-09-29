// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Przeglądarka stanów USA (WAS - Worked All States) i prowincji z bazy serviceLOG.db

use crate::core::awards::AwardsEngine;
use crate::core::i18n::{tr, tr_or, Language};
use crate::core::service_db::{ServiceDatabase, StateRecord};
use eframe::egui;

/// Status dyplomowy stanu/provincji (WAS) względem dziennika.
enum RefStatus {
    Needed,
    Worked,
    Confirmed,
}

impl RefStatus {
    fn of_state(engine: &AwardsEngine, code: &str) -> Self {
        if engine.confirmed_was.contains(code) {
            RefStatus::Confirmed
        } else if engine.worked_was.contains(code) {
            RefStatus::Worked
        } else {
            RefStatus::Needed
        }
    }

    fn label(&self, lang: Language) -> egui::RichText {
        match self {
            RefStatus::Confirmed => {
                egui::RichText::new(tr_or(lang, "✔ Potwierdzone", "✔ Confirmed")).color(egui::Color32::from_rgb(56, 189, 248))
            }
            RefStatus::Worked => {
                egui::RichText::new(tr_or(lang, "✔ Zaliczony", "✔ Worked")).color(egui::Color32::from_rgb(34, 197, 94))
            }
            RefStatus::Needed => {
                egui::RichText::new(tr_or(lang, "— Potrzebny", "— Needed")).color(egui::Color32::from_rgb(248, 113, 113))
            }
        }
    }
}

#[derive(Default)]
pub struct StatesBrowserDialog {
    pub is_open: bool,
    pub search_query: String,
    pub results: Vec<StateRecord>,
    pub loaded_initial: bool,
}

impl StatesBrowserDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn reload(&mut self, sdb: &ServiceDatabase) {
        self.results = sdb.search_states(&self.search_query);
        self.loaded_initial = true;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        sdb: &ServiceDatabase,
        awards: &AwardsEngine,
        lang: Language,
    ) -> Option<String> {
        if !self.is_open {
            return None;
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        let mut chosen = None;

        egui::Window::new(tr_or(
            lang,
            "🗺 Stany USA (WAS) i Prowincje",
            "🗺 US States (WAS) & Provinces",
        ))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr_or(
                        lang,
                        "Szukaj stanu (kod np. CA, NY lub nazwa):",
                        "Search state (code e.g. CA, NY or name):",
                    ));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query).desired_width(180.0),
                    );
                    if resp.changed() || !self.loaded_initial {
                        self.reload(sdb);
                    }
                    if ui.button(tr("btn.clear", lang)).clicked() {
                        self.search_query.clear();
                        self.reload(sdb);
                    }
                });

                ui.separator();

                ui.label(format!(
                    "{} ({}) - serviceLOG:",
                    tr_or(lang, "Wyniki", "Results"),
                    self.results.len()
                ));

                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        egui::Grid::new("states_results_grid")
                            .striped(true)
                            .spacing([15.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(tr_or(lang, "Kod", "Code")).strong());
                                ui.label(egui::RichText::new(tr_or(lang, "Nazwa Stanu / Prowincji", "State / Province Name")).strong());
                                ui.label(egui::RichText::new(tr_or(lang, "Kraj", "Country")).strong());
                                ui.label(egui::RichText::new("Status").strong());
                                ui.label(egui::RichText::new(tr_or(lang, "Akcja", "Action")).strong());
                                ui.end_row();

                                for item in &self.results {
                                    ui.label(
                                        egui::RichText::new(&item.code)
                                            .strong()
                                            .color(egui::Color32::from_rgb(56, 189, 248)),
                                    );
                                    ui.label(&item.name);
                                    ui.label(
                                        egui::RichText::new(&item.country)
                                            .color(egui::Color32::from_rgb(251, 191, 36)),
                                    );
                                    ui.label(RefStatus::of_state(awards, &item.code).label(lang));

                                    if ui
                                        .button(tr_or(lang, "Wybierz", "Select"))
                                        .on_hover_text(tr_or(lang, "Użyj tego kodu stanu", "Use this state code"))
                                        .clicked()
                                    {
                                        chosen = Some(item.code.clone());
                                        close_requested = true;
                                    }
                                    ui.end_row();
                                }
                            });
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(tr_or(
                            lang,
                            "Zawiera 50 stanów USA (WAS) oraz 3 500+ okręgów z serviceLOG.",
                            "Contains 50 US states (WAS) and 3,500+ subdivisions from serviceLOG.",
                        ))
                        .small()
                        .color(egui::Color32::GRAY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr("btn.close", lang)).clicked() {
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
