// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno bazy prefiksów i znaków specjalnych (UniqueCalls z serviceLOG.db - 4 310 rekordów)

use crate::core::i18n::{Language, Msg, key, tr};
use crate::core::service_db::{ServiceDatabase, UniqueCallRecord};
use crate::gui::theme::ThemeExt;
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

    pub fn show(&mut self, ctx: &egui::Context, db: &ServiceDatabase, lang: Language) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new(tr(key::PREFIX_BROWSER_TITLE, lang))
            .open(&mut open)
            .default_width(700.0)
            .default_height(450.0)
            .show(ctx, |ui| {
                let palette = ui.palette();
                ui.horizontal(|ui| {
                    ui.label(tr(key::PREFIX_SEARCH_LABEL, lang));
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(220.0)
                            .hint_text(tr(key::PREFIX_SEARCH_HINT, lang)),
                    );
                    if edit.changed()
                        || (ui.input(|i| i.key_pressed(egui::Key::Enter)) && edit.has_focus())
                    {
                        self.search_query = self.search_query.to_uppercase();
                        self.execute_search(db);
                    }

                    if ui
                        .button(format!("🔍 {}", tr(key::BTN_SEARCH, lang)))
                        .clicked()
                    {
                        self.execute_search(db);
                    }

                    if ui.button(tr(key::BTN_CLEAR, lang)).clicked() {
                        self.search_query.clear();
                        self.results.clear();
                    }
                });

                ui.separator();
                ui.label(
                    Msg::new(key::PREFIX_RESULTS_COUNT)
                        .arg("count", self.results.len())
                        .text(lang),
                );

                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        egui::Grid::new("prefix_mgr_grid")
                            .striped(true)
                            .spacing([15.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(tr(key::PREFIX_COL_CALLSIGN, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::PREFIX_COL_COUNTRY, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::PREFIX_COL_ARRL_PREFIX, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::GEO_CONTINENT, lang)).strong(),
                                );
                                ui.label(egui::RichText::new(tr(key::GEO_CQ_ZONE, lang)).strong());
                                ui.label(egui::RichText::new(tr(key::GEO_ITU_ZONE, lang)).strong());
                                ui.end_row();

                                for rec in &self.results {
                                    ui.label(
                                        egui::RichText::new(&rec.call)
                                            .strong()
                                            .color(palette.accent),
                                    );
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
                    ui.label(
                        egui::RichText::new(tr(key::PREFIX_DB_INFO, lang))
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
    }
}
