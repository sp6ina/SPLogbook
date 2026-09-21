// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wielodziennikowość - Okno zarządzania profilami dzienników łączności

use crate::core::database::{Journal, LogDatabase};
use crate::core::i18n::{tr, Language};
use eframe::egui;

pub struct JournalManagerDialog {
    pub is_open: bool,
    pub journals: Vec<Journal>,
    pub selected_index: usize,
    pub new_journal_id: String,
    pub new_journal_name: String,
    pub new_journal_callsign: String,
    pub new_journal_operator: String,
    pub new_journal_grid: String,
    pub new_journal_pga: String,
    pub new_journal_desc: String,
    pub show_create_form: bool,
}

impl Default for JournalManagerDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            journals: Vec::new(),
            selected_index: 0,
            new_journal_id: String::new(),
            new_journal_name: String::new(),
            new_journal_callsign: String::new(),
            new_journal_operator: String::new(),
            new_journal_grid: String::new(),
            new_journal_pga: String::new(),
            new_journal_desc: String::new(),
            show_create_form: false,
        }
    }
}

impl JournalManagerDialog {
    pub fn reload(&mut self, db: &LogDatabase) {
        if let Ok(list) = db.get_all_journals() {
            self.journals = list;
            self.selected_index = self.journals.iter().position(|j| j.is_default).unwrap_or(0);
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, db: &LogDatabase, lang: Language, on_journal_switched: &mut dyn FnMut(&Journal)) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        egui::Window::new(tr("journal.title", lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(tr("journal.defined_profiles", lang));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr("journal.new_profile", lang)).clicked() {
                            self.show_create_form = !self.show_create_form;
                        }
                    });
                });

                ui.separator();

                if self.show_create_form {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(tr("journal.create_title", lang)).strong());
                        egui::Grid::new("new_journal_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label(tr("journal.id_code", lang));
                            ui.text_edit_singleline(&mut self.new_journal_id);
                            ui.end_row();

                            ui.label(tr("journal.profile_name", lang));
                            ui.text_edit_singleline(&mut self.new_journal_name);
                            ui.end_row();

                            ui.label(format!("{}:", tr("qso.callsign", lang)));
                            ui.text_edit_singleline(&mut self.new_journal_callsign);
                            ui.end_row();

                            ui.label(format!("{}:", tr("station.operator", lang)));
                            ui.text_edit_singleline(&mut self.new_journal_operator);
                            ui.end_row();

                            ui.label(format!("{}:", tr("geo.locator", lang)));
                            ui.text_edit_singleline(&mut self.new_journal_grid);
                            ui.end_row();

                            ui.label(format!("{}:", tr("qso.pga", lang)));
                            ui.text_edit_singleline(&mut self.new_journal_pga);
                            ui.end_row();

                            ui.label(format!("{}:", tr("qso.comment", lang)));
                            ui.text_edit_singleline(&mut self.new_journal_desc);
                            ui.end_row();
                        });

                        ui.horizontal(|ui| {
                            if ui.button(format!("💾 {}", tr("btn.save", lang))).clicked()
                                && !self.new_journal_id.trim().is_empty() && !self.new_journal_name.trim().is_empty() {
                                    let j = Journal {
                                        id: self.new_journal_id.trim().to_uppercase(),
                                        name: self.new_journal_name.trim().to_string(),
                                        station_callsign: self.new_journal_callsign.trim().to_uppercase(),
                                        operator: self.new_journal_operator.trim().to_string(),
                                        my_gridsquare: self.new_journal_grid.trim().to_uppercase(),
                                        my_pga: self.new_journal_pga.trim().to_uppercase(),
                                        description: self.new_journal_desc.trim().to_string(),
                                        is_default: false,
                                    };
                                    let _ = db.create_journal(&j);
                                    self.show_create_form = false;
                                    self.reload(db);
                                }
                            if ui.button(tr("btn.cancel", lang)).clicked() {
                                self.show_create_form = false;
                            }
                        });
                    });
                    ui.separator();
                }

                // Lista istniejących dzienników
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    for journal in self.journals.iter() {
                        let is_active = journal.is_default;
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let badge = if is_active { tr("journal.active_badge", lang) } else { "📁 " };
                                let header = format!("{}{}: {}", badge, journal.station_callsign, journal.name);
                                ui.label(egui::RichText::new(header).strong().color(if is_active {
                                    egui::Color32::from_rgb(100, 220, 100)
                                } else {
                                    egui::Color32::from_rgb(200, 200, 200)
                                }));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if !is_active {
                                        if ui.button(tr("journal.activate", lang)).clicked() {
                                            let _ = db.set_active_journal(&journal.id);
                                            on_journal_switched(journal);
                                        }
                                        if journal.id != "DEFAULT" && ui.button("🗑").on_hover_text(tr("journal.delete_tooltip", lang)).clicked() {
                                            let _ = db.delete_journal(&journal.id);
                                        }
                                    }
                                });
                            });
                            ui.label(format!("{}: {} | {}: {} | {}: {}", tr("station.operator", lang), journal.operator, tr("geo.locator", lang), journal.my_gridsquare, tr("qso.pga", lang), journal.my_pga));
                            if !journal.description.is_empty() {
                                ui.label(egui::RichText::new(&journal.description).italics().small());
                            }
                        });
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(tr("btn.close", lang)).clicked() {
                        close_requested = true;
                    }
                });
            });

        if close_requested {
            open = false;
        }
        self.is_open = open;
    }
}
