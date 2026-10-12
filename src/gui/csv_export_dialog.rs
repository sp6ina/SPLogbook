// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno dialogowe konfigurowalnego eksportu dziennika do CSV.
//!
//! Pozwala wybrać zakres (wszystkie / widok / zaznaczone), zestaw kolumn,
//! separator oraz to, czy dołączyć nagłówek. Właściwy zapis do pliku wykonuje
//! `SpLogApp::perform_csv_export` na podstawie zwróconego żądania.

use crate::core::csv_export::{CsvColumn, CsvDelimiter};
use crate::core::i18n::{Language, key, tr};
use eframe::egui;

/// Zakres danych podlegających eksportowi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvExportScope {
    All,
    Filtered,
    Selected,
}

impl CsvExportScope {
    fn label(self, lang: Language) -> &'static str {
        match self {
            CsvExportScope::All => tr(key::CSV_SCOPE_ALL, lang),
            CsvExportScope::Filtered => tr(key::CSV_SCOPE_FILTERED, lang),
            CsvExportScope::Selected => tr(key::CSV_SCOPE_SELECTED, lang),
        }
    }
}

/// Kompletne żądanie eksportu wygenerowane przez okno dialogowe.
#[derive(Debug, Clone)]
pub struct CsvExportRequest {
    pub scope: CsvExportScope,
    pub columns: Vec<CsvColumn>,
    pub delimiter: CsvDelimiter,
    pub include_header: bool,
}

pub struct CsvExportDialog {
    pub is_open: bool,
    selected_columns: Vec<CsvColumn>,
    delimiter: CsvDelimiter,
    include_header: bool,
    scope: CsvExportScope,
}

impl Default for CsvExportDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvExportDialog {
    /// Domyślny, sensowny zestaw kolumn przy pierwszym otwarciu.
    fn default_columns() -> Vec<CsvColumn> {
        vec![
            CsvColumn::Callsign,
            CsvColumn::Date,
            CsvColumn::Time,
            CsvColumn::Band,
            CsvColumn::Mode,
            CsvColumn::Freq,
            CsvColumn::RstSent,
            CsvColumn::RstRcvd,
            CsvColumn::Name,
            CsvColumn::Country,
            CsvColumn::Comment,
        ]
    }

    pub fn new() -> Self {
        Self {
            is_open: false,
            selected_columns: Self::default_columns(),
            delimiter: CsvDelimiter::Comma,
            include_header: true,
            scope: CsvExportScope::All,
        }
    }

    /// Otwiera okno (przywraca domyślne kolumny przy pierwszym użyciu).
    pub fn open(&mut self) {
        if self.selected_columns.is_empty() {
            self.selected_columns = Self::default_columns();
        }
        self.is_open = true;
    }

    /// Kolumny w stabilnej kolejności katalogu, ograniczone do zaznaczonych.
    fn ordered_selection(&self) -> Vec<CsvColumn> {
        CsvColumn::ALL
            .iter()
            .copied()
            .filter(|c| self.selected_columns.contains(c))
            .collect()
    }

    fn is_selected(&self, col: CsvColumn) -> bool {
        self.selected_columns.contains(&col)
    }

    fn set_selected(&mut self, col: CsvColumn, selected: bool) {
        if selected {
            if !self.selected_columns.contains(&col) {
                self.selected_columns.push(col);
            }
        } else {
            self.selected_columns.retain(|c| *c != col);
        }
    }

    /// Rysuje okno dialogowe. Zwraca `Some(request)`, gdy użytkownik kliknął „Eksportuj".
    pub fn render(&mut self, ctx: &egui::Context, lang: Language) -> Option<CsvExportRequest> {
        if !self.is_open {
            return None;
        }

        let mut open = self.is_open;
        let mut export_requested = false;
        let mut close_requested = false;

        egui::Window::new(tr(key::CSV_TITLE, lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.label(tr(key::CSV_SCOPE_LABEL, lang));
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut self.scope,
                        CsvExportScope::All,
                        CsvExportScope::All.label(lang),
                    );
                    ui.radio_value(
                        &mut self.scope,
                        CsvExportScope::Filtered,
                        CsvExportScope::Filtered.label(lang),
                    );
                    ui.radio_value(
                        &mut self.scope,
                        CsvExportScope::Selected,
                        CsvExportScope::Selected.label(lang),
                    );
                });

                ui.add_space(8.0);
                ui.label(tr(key::CSV_DELIMITER_LABEL, lang));
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut self.delimiter,
                        CsvDelimiter::Comma,
                        CsvDelimiter::Comma.display_name(),
                    );
                    ui.radio_value(
                        &mut self.delimiter,
                        CsvDelimiter::Semicolon,
                        CsvDelimiter::Semicolon.display_name(),
                    );
                    ui.radio_value(
                        &mut self.delimiter,
                        CsvDelimiter::Tab,
                        CsvDelimiter::Tab.display_name(),
                    );
                });

                ui.add_space(4.0);
                ui.checkbox(&mut self.include_header, tr(key::CSV_INCLUDE_HEADER, lang));

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tr(key::CSV_COLUMNS_LABEL, lang)).strong());
                    if ui.small_button(tr(key::CSV_SELECT_ALL, lang)).clicked() {
                        self.selected_columns = CsvColumn::ALL.to_vec();
                    }
                    if ui.small_button(tr(key::CSV_CLEAR, lang)).clicked() {
                        self.selected_columns.clear();
                    }
                    if ui
                        .small_button(tr(key::CSV_RESTORE_DEFAULTS, lang))
                        .clicked()
                    {
                        self.selected_columns = Self::default_columns();
                    }
                });

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for col in CsvColumn::ALL {
                                let mut selected = self.is_selected(*col);
                                if ui.checkbox(&mut selected, col.label()).changed() {
                                    self.set_selected(*col, selected);
                                }
                            }
                        });
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    let count = self.ordered_selection().len();
                    if ui.button(tr(key::CSV_EXPORT_BTN, lang)).clicked() && count > 0 {
                        export_requested = true;
                    }
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_requested = true;
                    }
                    ui.label(format!("{}: {count}", tr(key::CSV_SELECTED_COUNT, lang)));
                });
            });

        if close_requested {
            open = false;
        }
        self.is_open = open;
        if !self.is_open {
            return None;
        }
        if export_requested {
            Some(CsvExportRequest {
                scope: self.scope,
                columns: self.ordered_selection(),
                delimiter: self.delimiter,
                include_header: self.include_header,
            })
        } else {
            None
        }
    }
}
