// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Przeglądarka i wyszukiwarka grup wysp IOTA (Islands on the Air) z bazy serviceLOG.db

use crate::core::awards::AwardsEngine;
use crate::core::i18n::{Language, Msg, key, tr};
use crate::core::service_db::{IotaRecord, ServiceDatabase};
use crate::gui::theme::{Palette, ThemeExt};
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

    fn label(&self, lang: Language, palette: &Palette) -> egui::RichText {
        match self {
            RefStatus::Confirmed => {
                egui::RichText::new(tr(key::IOTA_STATUS_CONFIRMED, lang)).color(palette.accent)
            }
            RefStatus::Worked => {
                egui::RichText::new(tr(key::IOTA_STATUS_WORKED, lang)).color(palette.success)
            }
            RefStatus::Needed => {
                egui::RichText::new(tr(key::IOTA_STATUS_NEEDED, lang)).color(palette.danger)
            }
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

        egui::Window::new(tr(key::IOTA_BROWSER_TITLE, lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(580.0)
            .show(ctx, |ui| {
                let palette = ui.palette();
                ui.horizontal(|ui| {
                    ui.label(tr(key::IOTA_SEARCH_LABEL, lang));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query).desired_width(200.0),
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
                    Msg::new(key::IOTA_RESULTS_COUNT)
                        .arg("count", self.results.len())
                        .text(lang),
                );

                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        egui::Grid::new("iota_results_grid")
                            .striped(true)
                            .spacing([15.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(tr(key::IOTA_COL_CODE, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::IOTA_COL_PREFIX, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::IOTA_COL_GROUP_NAME, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::AWARDS_COL_STATUS, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::IOTA_COL_ACTION, lang)).strong(),
                                );
                                ui.end_row();

                                for item in &self.results {
                                    ui.label(
                                        egui::RichText::new(&item.iota)
                                            .strong()
                                            .color(palette.accent),
                                    );
                                    ui.label(
                                        egui::RichText::new(&item.prefix).color(palette.warning),
                                    );
                                    ui.label(&item.name);
                                    ui.label(
                                        RefStatus::of_iota(awards, &item.iota)
                                            .label(lang, &palette),
                                    );

                                    if ui
                                        .button(tr(key::IOTA_BTN_SELECT, lang))
                                        .on_hover_text(tr(key::IOTA_SELECT_TOOLTIP, lang))
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
                        egui::RichText::new(tr(key::IOTA_DB_INFO, lang))
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
