// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Przeglądarka i wyszukiwarka grup wysp IOTA (Islands on the Air) z bazy serviceLOG.db

use crate::core::awards::AwardsEngine;
use crate::core::i18n::{Language, tr, tr_or};
use crate::core::service_db::{IotaRecord, ServiceDatabase};
use eframe::egui;

/// Status dyplomowy pojedynczej referencji IOTA względem dziennika.
enum RefStatus {
    Needed,
    Worked,
    Confirmed,
}

impl RefStatus {
    fn of_iota(engine: &AwardsEngine, iota: &str) -> Self {
        if engine
            .details_iota
            .get(iota)
            .is_some_and(|v| v.iter().any(|r| r.is_confirmed))
        {
            RefStatus::Confirmed
        } else if engine.worked_iota.contains(iota) {
            RefStatus::Worked
        } else {
            RefStatus::Needed
        }
    }

    fn label(&self, lang: Language) -> egui::RichText {
        match self {
            RefStatus::Confirmed => {
                egui::RichText::new(tr_or(lang, "✔ Potwierdzone", "✔ Confirmed"))
                    .color(egui::Color32::from_rgb(56, 189, 248))
            }
            RefStatus::Worked => egui::RichText::new(tr_or(lang, "✔ Zaliczona", "✔ Worked"))
                .color(egui::Color32::from_rgb(34, 197, 94)),
            RefStatus::Needed => egui::RichText::new(tr_or(lang, "— Potrzebna", "— Needed"))
                .color(egui::Color32::from_rgb(248, 113, 113)),
        }
    }
}

#[derive(Default)]
pub struct IotaBrowserDialog {
    pub is_open: bool,
    pub search_query: String,
    pub results: Vec<IotaRecord>,
    pub loaded_initial: bool,
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
            "🏝 Przeglądarka Wysp Świata IOTA (Islands On The Air)",
            "🏝 IOTA Browser (Islands On The Air)",
        ))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(580.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr_or(
                    lang,
                    "Szukaj IOTA (kod np. EU-001 lub nazwa wyspy):",
                    "Search IOTA (code e.g. EU-001 or island name):",
                ));
                let resp =
                    ui.add(egui::TextEdit::singleline(&mut self.search_query).desired_width(200.0));
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
                    egui::Grid::new("iota_results_grid")
                        .striped(true)
                        .spacing([15.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(tr_or(lang, "IOTA Kod", "IOTA Code")).strong(),
                            );
                            ui.label(
                                egui::RichText::new(tr_or(lang, "Prefiks", "Prefix")).strong(),
                            );
                            ui.label(
                                egui::RichText::new(tr_or(
                                    lang,
                                    "Nazwa grupy wysp",
                                    "Island Group Name",
                                ))
                                .strong(),
                            );
                            ui.label(egui::RichText::new("Status").strong());
                            ui.label(egui::RichText::new(tr_or(lang, "Akcja", "Action")).strong());
                            ui.end_row();

                            for item in &self.results {
                                ui.label(
                                    egui::RichText::new(&item.iota)
                                        .strong()
                                        .color(egui::Color32::from_rgb(56, 189, 248)),
                                );
                                ui.label(
                                    egui::RichText::new(&item.prefix)
                                        .color(egui::Color32::from_rgb(251, 191, 36)),
                                );
                                ui.label(&item.name);
                                ui.label(RefStatus::of_iota(awards, &item.iota).label(lang));

                                if ui
                                    .button(tr_or(lang, "Wybierz", "Select"))
                                    .on_hover_text(tr_or(
                                        lang,
                                        "Użyj tej referencji IOTA",
                                        "Use this IOTA reference",
                                    ))
                                    .clicked()
                                {
                                    chosen = Some(item.iota.clone());
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
                        "Baza zawiera 1 190 oficjalnych grup wysp programu RSGB IOTA.",
                        "Database contains 1,190 official RSGB IOTA island groups.",
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
