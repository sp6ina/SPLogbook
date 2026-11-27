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
                        delimiter_label(CsvDelimiter::Comma, lang),
                    );
                    ui.radio_value(
                        &mut self.delimiter,
                        CsvDelimiter::Semicolon,
                        delimiter_label(CsvDelimiter::Semicolon, lang),
                    );
                    ui.radio_value(
                        &mut self.delimiter,
                        CsvDelimiter::Tab,
                        delimiter_label(CsvDelimiter::Tab, lang),
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
                                if ui
                                    .checkbox(&mut selected, column_label(*col, lang))
                                    .changed()
                                {
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

/// Nazwa separatora w języku interfejsu.
fn delimiter_label(delimiter: CsvDelimiter, lang: Language) -> &'static str {
    match delimiter {
        CsvDelimiter::Comma => tr(key::CSV_DELIM_COMMA, lang),
        CsvDelimiter::Semicolon => tr(key::CSV_DELIM_SEMICOLON, lang),
        CsvDelimiter::Tab => tr(key::CSV_DELIM_TAB, lang),
    }
}

/// Nazwa kolumny w języku interfejsu. Nagłówek w pliku pozostaje nazwą pola ADIF.
fn column_label(column: CsvColumn, lang: Language) -> &'static str {
    let k = match column {
        CsvColumn::Date => key::QSO_DATE,
        CsvColumn::Time => key::QSO_TIME,
        CsvColumn::Callsign => key::QSO_CALLSIGN,
        CsvColumn::Band => key::QSO_BAND,
        CsvColumn::Mode => key::QSO_MODE,
        CsvColumn::Submode => key::CSV_COL_SUBMODE,
        CsvColumn::Freq => key::CSV_COL_FREQ_TX,
        CsvColumn::FreqRx => key::CSV_COL_FREQ_RX,
        CsvColumn::RstSent => key::QSO_RST_SENT,
        CsvColumn::RstRcvd => key::QSO_RST_RCVD,
        CsvColumn::Name => key::QSO_NAME,
        CsvColumn::Qth => key::QSO_QTH,
        CsvColumn::Grid => key::QSO_LOCATOR,
        CsvColumn::State => key::QSO_STATE,
        CsvColumn::Iota => key::QSO_IOTA,
        CsvColumn::SotaRef => key::QSO_SOTA,
        CsvColumn::PotaRef => key::QSO_POTA,
        CsvColumn::PgaRef => key::QSO_PGA,
        CsvColumn::Dxcc => key::GEO_DXCC,
        CsvColumn::Country => key::GEO_COUNTRY,
        CsvColumn::Continent => key::GEO_CONTINENT,
        CsvColumn::Cqz => key::GEO_CQ_ZONE,
        CsvColumn::Ituz => key::GEO_ITU_ZONE,
        CsvColumn::Comment => key::QSO_COMMENT,
        CsvColumn::QslVia => key::QSO_QSL_VIA,
        CsvColumn::QslSent => key::CSV_COL_QSL_SENT,
        CsvColumn::QslRcvd => key::CSV_COL_QSL_RCVD,
        CsvColumn::LotwQslRcvd => key::CSV_COL_LOTW_RCVD,
        CsvColumn::EqslQslRcvd => key::CSV_COL_EQSL_RCVD,
        CsvColumn::SatName => key::CSV_COL_SAT_NAME,
        CsvColumn::SatMode => key::CSV_COL_SAT_MODE,
        CsvColumn::PropMode => key::CSV_COL_PROP_MODE,
        CsvColumn::Srx => key::CSV_COL_SRX,
        CsvColumn::Stx => key::CSV_COL_STX,
        CsvColumn::SrxString => key::CSV_COL_SRX_STRING,
        CsvColumn::StxString => key::CSV_COL_STX_STRING,
        CsvColumn::MyGrid => key::CSV_COL_MY_GRID,
        CsvColumn::MyState => key::CSV_COL_MY_STATE,
        CsvColumn::MyPotaRef => key::QSO_MY_POTA,
        CsvColumn::MySotaRef => key::QSO_MY_SOTA,
        // Nazwa własna programu dyplomowego.
        CsvColumn::VuccGrids => return column.label(),
    };
    tr(k, lang)
}
