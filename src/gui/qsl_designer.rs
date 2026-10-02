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
    // Kalibracja drukarki (marginesy w mm)
    pub margin_left_mm: f32,
    pub margin_top_mm: f32,
    // Reguły masowego wyboru kart do wydruku
    pub only_unprinted: bool,
    pub batch_band: String,
    pub batch_mode: String,
    pub batch_date_from: String,
    pub batch_date_to: String,
    pub batch_limit: usize,
}

impl Default for QslDesignerDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            sheet_format: LabelSheetFormat::Avery3x8,
            queued_qsos: Vec::new(),
            status_message: None,
            margin_left_mm: 10.0,
            margin_top_mm: 10.0,
            only_unprinted: true,
            batch_band: String::new(),
            batch_mode: String::new(),
            batch_date_from: String::new(),
            batch_date_to: String::new(),
            batch_limit: 48,
        }
    }
}

impl QslDesignerDialog {
    pub fn queue_by_rules(&mut self, db: &LogDatabase) {
        if let Ok(all) = db.get_all_qsos() {
            let from = self.batch_date_from.trim();
            let to = self.batch_date_to.trim();
            let band = self.batch_band.trim().to_uppercase();
            let mode = self.batch_mode.trim().to_uppercase();
            let limit = if self.batch_limit == 0 {
                48
            } else {
                self.batch_limit
            };

            self.queued_qsos = all
                .into_iter()
                .filter(|q| {
                    if self.only_unprinted && q.qsl_sent != "N" && q.qsl_sent != "R" {
                        return false;
                    }
                    if !band.is_empty() && q.band.to_uppercase() != band {
                        return false;
                    }
                    if !mode.is_empty() && q.mode.to_uppercase() != mode {
                        return false;
                    }
                    if !from.is_empty() && q.qso_date.as_str() < from {
                        return false;
                    }
                    if !to.is_empty() && q.qso_date.as_str() > to {
                        return false;
                    }
                    true
                })
                .take(limit)
                .collect();
            self.status_message = Some(format!(
                "Dodano {} łączności do kolejki druku.",
                self.queued_qsos.len()
            ));
        }
    }

    /// Walidacja przed generacją PDF. Zwraca komunikat błędu, jeśli nie można drukować.
    fn validate(&self, my_callsign: &str) -> Result<(), String> {
        if self.queued_qsos.is_empty() {
            return Err("Kolejka druku jest pusta. Najpierw dodaj łączności.".to_string());
        }
        if my_callsign.trim().is_empty() {
            return Err("Brak znaku stacji (callsign) do wydruku na etykietach.".to_string());
        }
        Ok(())
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
                            ui.selectable_value(
                                &mut self.sheet_format,
                                LabelSheetFormat::Avery3x8,
                                "Avery 3x8 (24 naklejki)",
                            );
                            ui.selectable_value(
                                &mut self.sheet_format,
                                LabelSheetFormat::Avery3x7,
                                "Avery 3x7 (21 naklejek)",
                            );
                            ui.selectable_value(
                                &mut self.sheet_format,
                                LabelSheetFormat::Avery2x8,
                                "Avery 2x8 (16 naklejek)",
                            );
                            ui.selectable_value(
                                &mut self.sheet_format,
                                LabelSheetFormat::Avery2x7,
                                "Avery 2x7 (14 naklejek)",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    if ui.button("📥 Dodaj z bazy wg reguł").clicked() {
                        self.queue_by_rules(db);
                    }
                    if ui.button("🗑 Wyczyść kolejkę").clicked() {
                        self.queued_qsos.clear();
                        self.status_message = Some("Wyczyszczono kolejkę druku.".to_string());
                    }
                });

                // Reguły masowego wyboru kart do wydruku
                ui.separator();
                ui.collapsing("🎯 Reguły wyboru kart do wydruku", |ui| {
                    ui.checkbox(
                        &mut self.only_unprinted,
                        "Tylko niepotwierdzone (QSL wysłane: N/R)",
                    );
                    ui.horizontal(|ui| {
                        ui.label("Pasmo:");
                        ui.text_edit_singleline(&mut self.batch_band);
                        ui.label("Tryb:");
                        ui.text_edit_singleline(&mut self.batch_mode);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Data od:");
                        ui.text_edit_singleline(&mut self.batch_date_from);
                        ui.label("do:");
                        ui.text_edit_singleline(&mut self.batch_date_to);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Limit:");
                        ui.add(egui::DragValue::new(&mut self.batch_limit).range(1..=1000));
                        ui.label("(0 = domyślnie 48)");
                    });
                });

                // Kalibracja drukarki
                ui.separator();
                ui.collapsing("🖨 Kalibracja wydruku (marginesy A4)", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Margines lewy (mm):");
                        ui.add(egui::Slider::new(&mut self.margin_left_mm, 0.0..=30.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Margines górny (mm):");
                        ui.add(egui::Slider::new(&mut self.margin_top_mm, 0.0..=30.0));
                    });
                    ui.label(
                        egui::RichText::new(
                            "Dopasuj marginesy, aby naklejki idealnie pokrywały pola arkusza.",
                        )
                        .small()
                        .color(egui::Color32::DARK_GRAY),
                    );
                });

                if let Some(ref msg) = self.status_message {
                    ui.label(egui::RichText::new(msg).color(egui::Color32::LIGHT_BLUE));
                }

                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "Łączności w kolejce druku ({} szt.):",
                        self.queued_qsos.len()
                    ))
                    .strong(),
                );

                let (cols, rows_per_page) = match self.sheet_format {
                    LabelSheetFormat::Avery3x8 => (3, 8),
                    LabelSheetFormat::Avery3x7 => (3, 7),
                    LabelSheetFormat::Avery2x8 => (2, 8),
                    LabelSheetFormat::Avery2x7 => (2, 7),
                };

                // Podgląd siatki etykiet
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        let total_cells = cols * rows_per_page;
                        egui::Grid::new("qsl_sheet_grid")
                            .num_columns(cols)
                            .spacing([10.0, 8.0])
                            .show(ui, |ui| {
                                for i in 0..total_cells {
                                    ui.group(|ui| {
                                        ui.set_min_size(egui::vec2(160.0, 70.0));
                                        if i < self.queued_qsos.len() {
                                            let qso = &self.queued_qsos[i];
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "TO: {}",
                                                    qso.callsign
                                                ))
                                                .strong()
                                                .color(egui::Color32::YELLOW),
                                            );
                                            ui.label(format!(
                                                "CFM QSO: {} {}",
                                                qso.qso_date, qso.time_on
                                            ));
                                            ui.label(format!(
                                                "{} | {} | RST {}",
                                                qso.band, qso.mode, qso.rst_sent
                                            ));
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "TNX QSL PSE! de {my_callsign}"
                                                ))
                                                .small()
                                                .italics(),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new(format!("[Puste #{}]", i + 1))
                                                    .color(egui::Color32::DARK_GRAY),
                                            );
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
                        if let Err(err) = self.validate(my_callsign) {
                            self.status_message = Some(err);
                            return;
                        }
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF Document", &["pdf"])
                            .set_file_name("qsl_labels_sheet.pdf")
                            .save_file()
                        {
                            use printpdf::*;

                            fn text_ops(
                                text: impl Into<String>,
                                size_pt: f32,
                                x_mm: f32,
                                y_mm: f32,
                                bold: bool,
                            ) -> Vec<Op> {
                                let safe_text = crate::core::text::transliterate_pl(&text.into());
                                vec![
                                    Op::StartTextSection,
                                    Op::SetFont {
                                        font: PdfFontHandle::Builtin(if bold {
                                            BuiltinFont::HelveticaBold
                                        } else {
                                            BuiltinFont::Helvetica
                                        }),
                                        size: Pt(size_pt),
                                    },
                                    Op::SetFillColor {
                                        col: Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)),
                                    },
                                    Op::SetTextCursor {
                                        pos: Point::new(Mm(x_mm), Mm(y_mm)),
                                    },
                                    Op::ShowText {
                                        items: vec![TextItem::Text(safe_text)],
                                    },
                                    Op::EndTextSection,
                                ]
                            }

                            let (cols, rows_per_page) = match self.sheet_format {
                                LabelSheetFormat::Avery3x8 => (3, 8),
                                LabelSheetFormat::Avery3x7 => (3, 7),
                                LabelSheetFormat::Avery2x8 => (2, 8),
                                LabelSheetFormat::Avery2x7 => (2, 7),
                            };

                            let content_w = 210.0 - 2.0 * self.margin_left_mm;
                            let content_h = 297.0 - 2.0 * self.margin_top_mm;
                            let col_w_mm = content_w / (cols as f32);
                            let row_h_mm = content_h / (rows_per_page as f32);

                            let labels_per_page = (cols * rows_per_page).max(1);
                            let mut doc = PdfDocument::new("QSL Labels Sheet");

                            for chunk in self.queued_qsos.chunks(labels_per_page) {
                                let mut ops: Vec<Op> = Vec::new();
                                for (i, q) in chunk.iter().enumerate() {
                                    let c = i % cols;
                                    let r = i / cols;
                                    let x = self.margin_left_mm + (c as f32) * col_w_mm;
                                    let y = 297.0 - self.margin_top_mm - (r as f32) * row_h_mm;

                                    ops.extend(text_ops(
                                        format!("TO: {}", q.callsign),
                                        11.0,
                                        x + 2.0,
                                        y - 5.0,
                                        true,
                                    ));
                                    ops.extend(text_ops(
                                        format!("QSO: {} {}", q.qso_date, q.time_on),
                                        9.0,
                                        x + 2.0,
                                        y - 11.0,
                                        false,
                                    ));
                                    ops.extend(text_ops(
                                        format!("{} | {} | RST {}", q.band, q.mode, q.rst_sent),
                                        9.0,
                                        x + 2.0,
                                        y - 17.0,
                                        false,
                                    ));
                                    ops.extend(text_ops(
                                        format!("TNX QSL! 73 de {my_callsign}"),
                                        8.0,
                                        x + 2.0,
                                        y - 23.0,
                                        false,
                                    ));
                                }
                                doc.pages.push(PdfPage::new(Mm(210.0), Mm(297.0), ops));
                            }

                            let mut warnings = Vec::new();
                            let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
                            if std::fs::write(&path, bytes).is_ok() {
                                self.status_message = Some(format!(
                                    "Zapisano arkusz naklejek PDF ({} str.): {}",
                                    doc.pages.len(),
                                    path.display()
                                ));
                                let _ = open::that(&path);
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
