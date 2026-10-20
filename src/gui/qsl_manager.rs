// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno przeglądarki managerów QSL (z bazy serviceLOG.db - 29 102 rekordów)

use crate::core::i18n::{Language, Msg, key, tr};
use crate::core::service_db::{QslManagerRecord, ServiceDatabase};
use crate::gui::theme::ThemeExt;
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
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        db: &ServiceDatabase,
        lang: Language,
    ) -> Option<String> {
        if !self.is_open {
            return None;
        }

        let mut chosen = None;
        let mut open = self.is_open;
        let mut close_requested = false;

        egui::Window::new(tr(key::QSL_MGR_BROWSER_TITLE, lang))
            .open(&mut open)
            .default_width(650.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                let palette = ui.palette();
                ui.horizontal(|ui| {
                    ui.label(tr(key::QSL_MGR_SEARCH_LABEL, lang));
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(220.0)
                            .hint_text(tr(key::QSL_MGR_SEARCH_HINT, lang)),
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
                    Msg::new(key::QSL_MGR_RESULTS_COUNT)
                        .arg("count", self.results.len())
                        .text(lang),
                );

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        egui::Grid::new("qsl_mgr_grid")
                            .striped(true)
                            .spacing([15.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(tr(key::QSL_MGR_COL_CALLSIGN, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::QSL_MGR_COL_MANAGER, lang))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::QSL_MGR_COL_PERIOD, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::QSL_MGR_COL_NOTES, lang)).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(tr(key::QSL_MGR_COL_ACTION, lang)).strong(),
                                );
                                ui.end_row();

                                for rec in &self.results {
                                    ui.label(
                                        egui::RichText::new(&rec.call)
                                            .strong()
                                            .color(palette.accent),
                                    );
                                    ui.label(
                                        egui::RichText::new(&rec.manager)
                                            .strong()
                                            .color(palette.success),
                                    );
                                    ui.label(&rec.years);
                                    ui.label(&rec.notes);

                                    if ui
                                        .button(tr(key::QSL_MGR_BTN_SELECT, lang))
                                        .on_hover_text(tr(key::QSL_MGR_SELECT_TOOLTIP, lang))
                                        .clicked()
                                    {
                                        chosen = Some(rec.manager.clone());
                                        close_requested = true;
                                    }
                                    ui.end_row();
                                }
                            });
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(tr(key::QSL_MGR_DB_INFO, lang))
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
