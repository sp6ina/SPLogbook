// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wielodziennikowość - Okno zarządzania profilami dzienników łączności

use crate::core::database::{Journal, LogDatabase};
use crate::core::i18n::{Language, Msg, key, tr};
use crate::gui::theme::ThemeExt;
use eframe::egui;

#[derive(Default)]
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
    pub status_message: Option<Msg>,
}

impl JournalManagerDialog {
    pub fn reload(&mut self, db: &LogDatabase) {
        if let Ok(list) = db.get_all_journals() {
            self.journals = list;
            self.selected_index = self.journals.iter().position(|j| j.is_default).unwrap_or(0);
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        db: &LogDatabase,
        lang: Language,
        on_journal_switched: &mut dyn FnMut(&Journal),
    ) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        egui::Window::new(tr(key::JOURNAL_TITLE, lang))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(tr(key::JOURNAL_DEFINED_PROFILES, lang));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(key::JOURNAL_NEW_PROFILE, lang)).clicked() {
                            self.show_create_form = !self.show_create_form;
                        }
                    });
                });

                ui.separator();

                if self.show_create_form {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(tr(key::JOURNAL_CREATE_TITLE, lang)).strong());
                        egui::Grid::new("new_journal_grid")
                            .num_columns(2)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(tr(key::JOURNAL_ID_CODE, lang));
                                ui.text_edit_singleline(&mut self.new_journal_id);
                                ui.end_row();

                                ui.label(tr(key::JOURNAL_PROFILE_NAME, lang));
                                ui.text_edit_singleline(&mut self.new_journal_name);
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::QSO_CALLSIGN, lang)));
                                ui.text_edit_singleline(&mut self.new_journal_callsign);
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::STATION_OPERATOR, lang)));
                                ui.text_edit_singleline(&mut self.new_journal_operator);
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::GEO_LOCATOR, lang)));
                                ui.text_edit_singleline(&mut self.new_journal_grid);
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::QSO_PGA, lang)));
                                ui.text_edit_singleline(&mut self.new_journal_pga);
                                ui.end_row();

                                ui.label(format!("{}:", tr(key::QSO_COMMENT, lang)));
                                ui.text_edit_singleline(&mut self.new_journal_desc);
                                ui.end_row();
                            });

                        ui.horizontal(|ui| {
                            if ui
                                .button(format!("💾 {}", tr(key::BTN_SAVE, lang)))
                                .clicked()
                                && !self.new_journal_id.trim().is_empty()
                                && !self.new_journal_name.trim().is_empty()
                            {
                                let journal = Journal {
                                    id: self.new_journal_id.trim().to_uppercase(),
                                    name: self.new_journal_name.trim().to_string(),
                                    station_callsign: self
                                        .new_journal_callsign
                                        .trim()
                                        .to_uppercase(),
                                    operator: self.new_journal_operator.trim().to_string(),
                                    my_gridsquare: self.new_journal_grid.trim().to_uppercase(),
                                    my_pga: self.new_journal_pga.trim().to_uppercase(),
                                    description: self.new_journal_desc.trim().to_string(),
                                    is_default: false,
                                };
                                let _ = db.create_journal(&journal);
                                self.show_create_form = false;
                                self.reload(db);
                            }
                            if ui.button(tr(key::BTN_CANCEL, lang)).clicked() {
                                self.show_create_form = false;
                            }
                        });
                    });
                    ui.separator();
                }

                // Lista istniejących dzienników
                let mut switched_journal: Option<Journal> = None;
                let mut reload_needed = false;
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for journal in &self.journals {
                            let is_active = journal.is_default;
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    let badge = if is_active {
                                        tr(key::JOURNAL_ACTIVE_BADGE, lang)
                                    } else {
                                        "📁 "
                                    };
                                    let header = format!(
                                        "{}{}: {}",
                                        badge, journal.station_callsign, journal.name
                                    );
                                    let palette = ui.palette();
                                    ui.label(egui::RichText::new(header).strong().color(
                                        if is_active {
                                            palette.success
                                        } else {
                                            palette.text_primary
                                        },
                                    ));

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if !is_active {
                                                if ui
                                                    .button(tr(key::JOURNAL_ACTIVATE, lang))
                                                    .clicked()
                                                {
                                                    match db.set_active_journal(&journal.id) {
                                                        Ok(()) => {
                                                            self.status_message = None;
                                                            switched_journal =
                                                                Some(journal.clone());
                                                        }
                                                        Err(e) => {
                                                            self.status_message = Some(e.msg())
                                                        }
                                                    }
                                                }
                                                if !journal.id.eq_ignore_ascii_case("default")
                                                    && ui
                                                        .button("🗑")
                                                        .on_hover_text(tr(
                                                            key::JOURNAL_DELETE_TOOLTIP,
                                                            lang,
                                                        ))
                                                        .clicked()
                                                {
                                                    match db.delete_journal(&journal.id) {
                                                        Ok(()) => {
                                                            self.status_message = None;
                                                            reload_needed = true;
                                                        }
                                                        Err(e) => {
                                                            self.status_message = Some(e.msg())
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                    );
                                });
                                ui.label(format!(
                                    "{}: {} | {}: {} | {}: {}",
                                    tr(key::STATION_OPERATOR, lang),
                                    journal.operator,
                                    tr(key::GEO_LOCATOR, lang),
                                    journal.my_gridsquare,
                                    tr(key::QSO_PGA, lang),
                                    journal.my_pga
                                ));
                                if !journal.description.is_empty() {
                                    ui.label(
                                        egui::RichText::new(&journal.description).italics().small(),
                                    );
                                }
                            });
                        }
                    });

                if let Some(ref j) = switched_journal {
                    on_journal_switched(j);
                    reload_needed = true;
                }
                if reload_needed {
                    self.reload(db);
                }

                if let Some(msg) = &self.status_message {
                    ui.label(
                        egui::RichText::new(msg.text(lang))
                            .color(ui.palette().danger)
                            .small(),
                    );
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
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
