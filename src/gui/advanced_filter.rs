// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Zaawansowane okno wyszukiwania i filtrowania łączności z eksportem ADIF

use crate::core::adif::export_adif;
use crate::core::database::{AdvancedQsoFilter, LogDatabase};
use crate::core::i18n::{tr, Language};
use crate::core::qso::QsoRecord;
use eframe::egui;
use std::fs::File;
use std::io::Write;

#[derive(Default)]
pub struct AdvancedFilterDialog {
    pub is_open: bool,
    pub date_from: String,
    pub date_to: String,
    pub selected_bands: Vec<String>,
    pub selected_modes: Vec<String>,
    pub lotw_only: bool,
    pub eqsl_only: bool,
    pub qsl_rcvd_only: bool,
    pub call_query: String,
    pub is_filtered_active: bool,
    pub filtered_count: usize,
    pub status_message: Option<String>,
}

impl AdvancedFilterDialog {
    pub fn build_filter(&self, journal_id: Option<&str>) -> AdvancedQsoFilter {
        AdvancedQsoFilter {
            date_from: if self.date_from.is_empty() { None } else { Some(self.date_from.clone()) },
            date_to: if self.date_to.is_empty() { None } else { Some(self.date_to.clone()) },
            bands: self.selected_bands.clone(),
            modes: self.selected_modes.clone(),
            lotw_confirmed: if self.lotw_only { Some(true) } else { None },
            eqsl_confirmed: if self.eqsl_only { Some(true) } else { None },
            qsl_rcvd: if self.qsl_rcvd_only { Some(true) } else { None },
            callsign_query: if self.call_query.is_empty() { None } else { Some(self.call_query.clone()) },
            journal_id: journal_id.map(|s| s.to_string()),
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        db: &LogDatabase,
        active_journal_id: Option<&str>,
        my_callsign: &str,
        lang: Language,
        on_filter_applied: &mut dyn FnMut(Vec<QsoRecord>),
        on_filter_cleared: &mut dyn FnMut(),
    ) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        egui::Window::new(tr("filter.dialog_title", lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.heading(tr("filter.criteria_heading", lang));
                ui.separator();

                egui::Grid::new("filter_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label(tr("filter.date_from", lang));
                    ui.text_edit_singleline(&mut self.date_from);
                    ui.end_row();

                    ui.label(tr("filter.date_to", lang));
                    ui.text_edit_singleline(&mut self.date_to);
                    ui.end_row();

                    ui.label(tr("filter.call_query", lang));
                    ui.text_edit_singleline(&mut self.call_query);
                    ui.end_row();
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new(tr("filter.bands_selection", lang)).strong());
                let bands = ["160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"];
                ui.horizontal_wrapped(|ui| {
                    for b in &bands {
                        let mut is_sel = self.selected_bands.contains(&b.to_string());
                        if ui.checkbox(&mut is_sel, *b).changed() {
                            if is_sel {
                                self.selected_bands.push(b.to_string());
                            } else {
                                self.selected_bands.retain(|x| x != *b);
                            }
                        }
                    }
                    if ui.button(tr("filter.all_btn", lang)).clicked() {
                        self.selected_bands.clear();
                    }
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new(tr("filter.modes_selection", lang)).strong());
                let modes = ["CW", "SSB", "FT8", "FT4", "RTTY", "PSK", "FM", "AM"];
                ui.horizontal_wrapped(|ui| {
                    for m in &modes {
                        let mut is_sel = self.selected_modes.contains(&m.to_string());
                        if ui.checkbox(&mut is_sel, *m).changed() {
                            if is_sel {
                                self.selected_modes.push(m.to_string());
                            } else {
                                self.selected_modes.retain(|x| x != *m);
                            }
                        }
                    }
                    if ui.button(tr("filter.all_btn", lang)).clicked() {
                        self.selected_modes.clear();
                    }
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new(tr("filter.qsl_status_heading", lang)).strong());
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.lotw_only, tr("filter.lotw_only", lang));
                    ui.checkbox(&mut self.eqsl_only, tr("filter.eqsl_only", lang));
                    ui.checkbox(&mut self.qsl_rcvd_only, tr("filter.paper_only", lang));
                });

                ui.separator();

                if let Some(ref msg) = self.status_message {
                    ui.label(egui::RichText::new(msg).color(egui::Color32::LIGHT_BLUE));
                }

                ui.horizontal(|ui| {
                    if ui.button(format!("🔎 {}", tr("filter.apply_btn", lang))).clicked() {
                        let filter = self.build_filter(active_journal_id);
                        if let Ok(qsos) = db.search_qsos_advanced(&filter) {
                            self.filtered_count = qsos.len();
                            self.is_filtered_active = true;
                            self.status_message = Some(format!("Znaleziono {} łączności spełniających kryteria.", self.filtered_count));
                            on_filter_applied(qsos);
                        }
                    }

                    if ui.button(format!("❌ {}", tr("filter.clear_btn", lang))).clicked() {
                        self.date_from.clear();
                        self.date_to.clear();
                        self.selected_bands.clear();
                        self.selected_modes.clear();
                        self.lotw_only = false;
                        self.eqsl_only = false;
                        self.qsl_rcvd_only = false;
                        self.call_query.clear();
                        self.is_filtered_active = false;
                        self.status_message = Some("Filtry zresetowane - wyświetlanie pełnego logu.".to_string());
                        on_filter_cleared();
                    }

                    if ui.button(format!("📤 {}", tr("filter.export_adif_btn", lang))).clicked() {
                        let filter = self.build_filter(active_journal_id);
                        if let Ok(qsos) = db.search_qsos_advanced(&filter) {
                            if qsos.is_empty() {
                                self.status_message = Some("Brak łączności do wyeksportowania!".to_string());
                            } else if let Some(path) = rfd::FileDialog::new()
                                .add_filter("ADIF Log Files", &["adi", "adif"])
                                .set_file_name("SPLogbook-filtered.adi")
                                .save_file()
                            {
                                let author_call = if my_callsign.trim().is_empty() { "N0CALL" } else { my_callsign };
                                let content = export_adif(&qsos, "SPLogbook", author_call);
                                if let Ok(mut f) = File::create(&path) {
                                    let _ = f.write_all(content.as_bytes());
                                    self.status_message = Some(format!("Pomyślnie wyeksportowano do: {:?}", path));
                                }
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr("btn.close", lang)).clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

        if close_requested {
            open = false;
        }
        self.is_open = open;
    }
}
