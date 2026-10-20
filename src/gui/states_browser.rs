// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Przeglądarka stanów USA (WAS - Worked All States) i prowincji z bazy serviceLOG.db

use crate::core::awards::AwardsEngine;
use crate::core::i18n::{Language, Msg, key, tr};
use crate::core::service_db::{ServiceDatabase, StateRecord};
use crate::gui::theme::{Palette, ThemeExt};
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

    fn label(&self, lang: Language, palette: &Palette) -> egui::RichText {
        match self {
            RefStatus::Confirmed => {
                egui::RichText::new(tr(key::STATES_STATUS_CONFIRMED, lang)).color(palette.accent)
            }
            RefStatus::Worked => {
                egui::RichText::new(tr(key::STATES_STATUS_WORKED, lang)).color(palette.success)
            }
            RefStatus::Needed => {
                egui::RichText::new(tr(key::STATES_STATUS_NEEDED, lang)).color(palette.danger)
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

        egui::Window::new(tr(key::STATES_BROWSER_TITLE, lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let palette = ui.palette();
                ui.horizontal(|ui| {
                    ui.label(tr(key::STATES_SEARCH_LABEL, lang));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query).desired_width(180.0),
                    );
                    if resp.changed() || !self.loaded_initial {
                        self.reload(sdb);
                    }
                    if ui.button(tr(key::BTN_CLEAR, lang)).clicked() {
                        self.search_query.clear();
                        self.reload(sdb);
                    }
                });

                ui.separator();

                ui.label(
                    Msg::new(key::STATES_RESULTS_COUNT)
                        .arg("count", self.results.len())
                        .text(lang),
                );

                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        egui::Grid::new("states_results_grid")
                            .striped(true)
                            .spacing([15.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(tr(key::STATES_COL_CODE, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::STATES_COL_NAME, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::STATES_COL_COUNTRY, lang)).strong(),
                                );
                                ui.label(egui::RichText::new("Status").strong());
                                ui.label(
                                    egui::RichText::new(tr(key::STATES_COL_ACTION, lang)).strong(),
                                );
                                ui.end_row();

                                for item in &self.results {
                                    ui.label(
                                        egui::RichText::new(&item.code)
                                            .strong()
                                            .color(palette.accent),
                                    );
                                    ui.label(&item.name);
                                    ui.label(
                                        egui::RichText::new(&item.country).color(palette.warning),
                                    );
                                    ui.label(
                                        RefStatus::of_state(awards, &item.code)
                                            .label(lang, &palette),
                                    );

                                    if ui
                                        .button(tr(key::STATES_BTN_SELECT, lang))
                                        .on_hover_text(tr(key::STATES_SELECT_TOOLTIP, lang))
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
                        egui::RichText::new(tr(key::STATES_DB_INFO, lang))
                            .small()
                            .color(palette.text_muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
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
