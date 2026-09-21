// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Projektant i kolejka drukowania etykiet na karty QSL (Avery A4)

use crate::core::database::LogDatabase;
use crate::core::qso::QsoRecord;
use eframe::egui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelSheetFormat {
    Avery2x7, // 14 etykiet (99.1 x 38.1 mm)
    Avery2x8, // 16 etykiet (99.1 x 33.8 mm)
    Avery3x7, // 21 etykiet (63.5 x 38.1 mm)
    Avery3x8, // 24 etykiety (70.0 x 36.0 mm)
}

pub struct QslDesignerDialog {
    pub is_open: bool,
    pub sheet_format: LabelSheetFormat,
    pub queued_qsos: Vec<QsoRecord>,
    pub status_message: Option<String>,
}

impl Default for QslDesignerDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            sheet_format: LabelSheetFormat::Avery3x8,
            queued_qsos: Vec::new(),
            status_message: None,
        }
    }
}

impl QslDesignerDialog {
    pub fn queue_unprinted(&mut self, db: &LogDatabase) {
        if let Ok(all) = db.get_all_qsos() {
            self.queued_qsos = all.into_iter().filter(|q| q.qsl_sent == "N" || q.qsl_sent == "R").take(48).collect();
            self.status_message = Some(format!("Dodano {} łączności do kolejki druku.", self.queued_qsos.len()));
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, db: &LogDatabase, my_callsign: &str) {
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;
        egui::Window::new("🏷 Projektant i Wydruk Naklejek QSL")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .show(ctx, |ui| {
                ui.heading("Kolejka i format naklejek na karty QSL");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Format arkusza samoprzylepnego A4:");
                    egui::ComboBox::from_id_salt("sheet_fmt_combo")
                        .selected_text(match self.sheet_format {
                            LabelSheetFormat::Avery3x8 => "Avery 3x8 (24 naklejki, 70x36 mm)",
                            LabelSheetFormat::Avery3x7 => "Avery 3x7 (21 naklejek, 63.5x38.1 mm)",
                            LabelSheetFormat::Avery2x8 => "Avery 2x8 (16 naklejek, 99.1x33.8 mm)",
                            LabelSheetFormat::Avery2x7 => "Avery 2x7 (14 naklejek, 99.1x38.1 mm)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.sheet_format, LabelSheetFormat::Avery3x8, "Avery 3x8 (24 naklejki)");
                            ui.selectable_value(&mut self.sheet_format, LabelSheetFormat::Avery3x7, "Avery 3x7 (21 naklejek)");
                            ui.selectable_value(&mut self.sheet_format, LabelSheetFormat::Avery2x8, "Avery 2x8 (16 naklejek)");
                            ui.selectable_value(&mut self.sheet_format, LabelSheetFormat::Avery2x7, "Avery 2x7 (14 naklejek)");
                        });
                });

                ui.horizontal(|ui| {
                    if ui.button("📥 Dodaj niepotwierdzone z bazy").clicked() {
                        self.queue_unprinted(db);
                    }
                    if ui.button("🗑 Wyczyść kolejkę").clicked() {
                        self.queued_qsos.clear();
                        self.status_message = Some("Wyczyszczono kolejkę druku.".to_string());
                    }
                });

                if let Some(ref msg) = self.status_message {
                    ui.label(egui::RichText::new(msg).color(egui::Color32::LIGHT_BLUE));
                }

                ui.separator();
                ui.label(egui::RichText::new(format!("Łączności w kolejce druku ({} szt.):", self.queued_qsos.len())).strong());

                let (cols, rows_per_page) = match self.sheet_format {
                    LabelSheetFormat::Avery3x8 => (3, 8),
                    LabelSheetFormat::Avery3x7 => (3, 7),
                    LabelSheetFormat::Avery2x8 => (2, 8),
                    LabelSheetFormat::Avery2x7 => (2, 7),
                };

                // Podgląd siatki etykiet
                egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                    let total_cells = cols * rows_per_page;
                    egui::Grid::new("qsl_sheet_grid").num_columns(cols).spacing([10.0, 8.0]).show(ui, |ui| {
                        for i in 0..total_cells {
                            ui.group(|ui| {
                                ui.set_min_size(egui::vec2(160.0, 70.0));
                                if i < self.queued_qsos.len() {
                                    let qso = &self.queued_qsos[i];
                                    ui.label(egui::RichText::new(format!("TO: {}", qso.callsign)).strong().color(egui::Color32::YELLOW));
                                    ui.label(format!("CFM QSO: {} {}", qso.qso_date, qso.time_on));
                                    ui.label(format!("{} | {} | RST {}", qso.band, qso.mode, qso.rst_sent));
                                    ui.label(egui::RichText::new(format!("TNX QSL PSE! de {}", my_callsign)).small().italics());
                                } else {
                                    ui.label(egui::RichText::new(format!("[Puste #{}]", i + 1)).color(egui::Color32::DARK_GRAY));
                                }
                            });
                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("📄 Eksport Naklejek do PDF (A4)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF Document", &["pdf"])
                            .set_file_name("qsl_labels_sheet.pdf")
                            .save_file()
                        {
                            use printpdf::*;
                            let (doc, page1, layer1) = PdfDocument::new("QSL Labels Sheet", Mm(210.0), Mm(297.0), "Labels Layer");
                            let current_layer = doc.get_page(page1).get_layer(layer1);
                            let font = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();
                            let font_reg = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();

                            let (cols, rows_per_page) = match self.sheet_format {
                                LabelSheetFormat::Avery3x8 => (3, 8),
                                LabelSheetFormat::Avery3x7 => (3, 7),
                                LabelSheetFormat::Avery2x8 => (2, 8),
                                LabelSheetFormat::Avery2x7 => (2, 7),
                            };

                            let col_w_mm = 190.0 / (cols as f32);
                            let row_h_mm = 277.0 / (rows_per_page as f32);

                            for (i, q) in self.queued_qsos.iter().enumerate().take(cols * rows_per_page) {
                                let c = i % cols;
                                let r = i / cols;
                                let x = 10.0 + (c as f32) * col_w_mm;
                                let y = 285.0 - (r as f32) * row_h_mm;

                                current_layer.use_text(format!("TO: {}", q.callsign), 11.0, Mm(x + 2.0), Mm(y - 5.0), &font);
                                current_layer.use_text(format!("QSO: {} {}", q.qso_date, q.time_on), 9.0, Mm(x + 2.0), Mm(y - 11.0), &font_reg);
                                current_layer.use_text(format!("{} | {} | RST {}", q.band, q.mode, q.rst_sent), 9.0, Mm(x + 2.0), Mm(y - 17.0), &font_reg);
                                current_layer.use_text(format!("TNX QSL! 73 de {}", my_callsign), 8.0, Mm(x + 2.0), Mm(y - 23.0), &font_reg);
                            }

                            if let Ok(file) = std::fs::File::create(&path) {
                                if doc.save(&mut std::io::BufWriter::new(file)).is_ok() {
                                    self.status_message = Some(format!("Zapisano arkusz naklejek PDF: {:?}", path));
                                    let _ = open::that(&path);
                                }
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Zamknij").clicked() {
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
