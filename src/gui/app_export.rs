// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

// Eksport logu (Cabrillo, PDF, GPX).
use super::*;
use crate::core::i18n::{Msg, key, tr};

impl SpLogApp {
    pub fn export_cabrillo(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pliki Cabrillo (*.cbr, *.log)", &["cbr", "log"])
            .set_file_name(format!(
                "{}_{}.cbr",
                self.config.station.callsign,
                self.contest.name.replace(' ', "_")
            ))
            .set_title(tr(
                key::EXPORT_CABRILLO_SAVE_TITLE,
                self.gui.appearance.language,
            ))
            .save_file()
        {
            let qsos = match self.services.log_db.lock() {
                Ok(db) => match db.get_recent_qsos_for_journal(&self.active_journal.id, 10000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.gui.chrome.status_message = Some(
                            Msg::new(key::EXPORT_ERR_READ_LOG)
                                .arg("err", &e)
                                .text(self.gui.appearance.language),
                        );
                        return;
                    }
                },
                Err(e) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_ERR_OPEN_LOG)
                            .arg("err", &e)
                            .text(self.gui.appearance.language),
                    );
                    return;
                }
            };

            let my_dxcc: u32 = self
                .services
                .prefix_matcher
                .lookup(&self.config.station.callsign)
                .and_then(|m| if m.dxcc > 0 { Some(m.dxcc) } else { None })
                .unwrap_or(269);
            let my_cqzone = self.config.station.cq_zone as u8;

            let scoreboard = crate::core::contest_stats::compute_scoreboard(
                &self.contest.name,
                &self.config.custom_contests,
                &qsos,
                my_dxcc,
                my_cqzone,
            );

            let input = crate::core::cabrillo::CabrilloInput {
                contest_name: &self.contest.name,
                callsign: &self.config.station.callsign,
                operator: &self.config.station.operator,
                city: &self.config.station.city,
                country: &self.config.station.country,
                claimed_score: scoreboard.total,
                qsos: &qsos,
            };
            let out = crate::core::cabrillo::build(&input);

            match std::fs::write(&path, out) {
                Ok(()) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_CABRILLO_SUCCESS)
                            .arg("path", path.display().to_string())
                            .text(self.gui.appearance.language),
                    );
                    self.gui.chrome.status_toast = Some((
                        Msg::new(key::EXPORT_CABRILLO_TOAST)
                            .arg(
                                "file",
                                path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            )
                            .text(self.gui.appearance.language),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_ERR_SAVE_CABRILLO)
                            .arg("err", &e)
                            .text(self.gui.appearance.language),
                    );
                }
            }
        }
    }

    pub fn export_pdf_log(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("SPLogbook.pdf")
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

            let qsos = match self.services.log_db.lock() {
                Ok(db) => match db.get_recent_qsos_for_journal(&self.active_journal.id, 10000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.gui.chrome.status_message = Some(
                            Msg::new(key::EXPORT_ERR_READ_LOG)
                                .arg("err", &e)
                                .text(self.gui.appearance.language),
                        );
                        return;
                    }
                },
                Err(e) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_ERR_OPEN_LOG)
                            .arg("err", &e)
                            .text(self.gui.appearance.language),
                    );
                    return;
                }
            };

            let mut doc = PdfDocument::new("SPLogbook Log");

            // Table headers
            let headers = [
                "Date", "Time", "Callsign", "Band", "Mode", "RST S", "RST R", "Country", "QSL",
            ];
            let x_positions = [10.0f32, 35.0, 55.0, 85.0, 105.0, 125.0, 140.0, 155.0, 185.0];

            let mut ops: Vec<Op> = Vec::new();
            ops.extend(text_ops("SPLogbook Log", 24.0, 10.0, 280.0, true));
            ops.extend(text_ops(
                format!("Operator: {}", self.config.station.callsign),
                12.0,
                10.0,
                270.0,
                false,
            ));
            ops.extend(text_ops(
                format!("Date: {}", chrono::Local::now().format("%Y-%m-%d")),
                12.0,
                10.0,
                265.0,
                false,
            ));

            let mut y_pos = 250.0f32;
            for (i, header) in headers.iter().enumerate() {
                ops.extend(text_ops(*header, 10.0, x_positions[i], y_pos, true));
            }
            y_pos -= 5.0;

            let mut row_count = 0usize;
            let mut page_num = 1usize;

            for qso in &qsos {
                if y_pos < 20.0 {
                    doc.pages
                        .push(PdfPage::new(Mm(210.0), Mm(297.0), std::mem::take(&mut ops)));
                    y_pos = 280.0;
                    page_num += 1;
                }
                ops.extend(text_ops(
                    qso.qso_date.as_str(),
                    10.0,
                    x_positions[0],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    qso.time_on.as_str(),
                    10.0,
                    x_positions[1],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    qso.callsign.as_str(),
                    10.0,
                    x_positions[2],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    qso.band.as_str(),
                    10.0,
                    x_positions[3],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    qso.mode.as_str(),
                    10.0,
                    x_positions[4],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    qso.rst_sent.as_str(),
                    10.0,
                    x_positions[5],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    qso.rst_rcvd.as_str(),
                    10.0,
                    x_positions[6],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    qso.country.as_deref().unwrap_or(""),
                    10.0,
                    x_positions[7],
                    y_pos,
                    false,
                ));
                ops.extend(text_ops(
                    format!("{}/{}", qso.qsl_sent, qso.qsl_rcvd),
                    10.0,
                    x_positions[8],
                    y_pos,
                    false,
                ));
                y_pos -= 5.0;
                row_count += 1;
            }

            // Footer
            ops.extend(text_ops(
                format!("Total QSOs: {row_count}"),
                12.0,
                10.0,
                10.0,
                false,
            ));
            ops.extend(text_ops(
                format!("Page {page_num}"),
                12.0,
                180.0,
                10.0,
                false,
            ));

            doc.pages.push(PdfPage::new(Mm(210.0), Mm(297.0), ops));

            let mut warnings = Vec::new();
            let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
            match std::fs::write(&path, bytes) {
                Ok(()) => {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    self.gui.chrome.status_toast = Some((
                        format!("Wyeksportowano {row_count} QSO do PDF: {filename}"),
                        std::time::Instant::now(),
                    ));
                    let _ = open::that(&path);
                }
                Err(e) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_ERR_SAVE_PDF)
                            .arg("err", &e)
                            .text(self.gui.appearance.language),
                    );
                }
            }
        }
    }

    pub fn export_gpx_log(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GPX", &["gpx"])
            .set_file_name("SPLogbook.gpx")
            .save_file()
        {
            use geo_types::Point;
            use gpx::{Gpx, GpxVersion, Waypoint};

            fn gridsquare_to_latlon(grid: &str) -> Option<(f64, f64)> {
                // Współdzielona, zwalidowana logika lokatora Maidenhead (core::geo)
                // zamiast lokalnej duplikacji bez walidacji formatu wejścia.
                crate::core::geo::locator_to_coordinates(grid)
                    .ok()
                    .map(|c| (c.latitude, c.longitude))
            }

            let qsos = match self.services.log_db.lock() {
                Ok(db) => match db.get_recent_qsos_for_journal(&self.active_journal.id, 10000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.gui.chrome.status_message = Some(
                            Msg::new(key::EXPORT_ERR_READ_LOG)
                                .arg("err", &e)
                                .text(self.gui.appearance.language),
                        );
                        return;
                    }
                },
                Err(e) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_ERR_OPEN_LOG)
                            .arg("err", &e)
                            .text(self.gui.appearance.language),
                    );
                    return;
                }
            };

            let mut gpx = Gpx {
                version: GpxVersion::Gpx11,
                ..Gpx::default()
            };

            let mut row_count = 0;
            for qso in &qsos {
                if let Some(grid) = &qso.gridsquare {
                    if let Some((lat, lon)) = gridsquare_to_latlon(grid) {
                        let mut waypoint = Waypoint::new(Point::new(lon, lat));
                        waypoint.name = Some(qso.callsign.clone());
                        waypoint.description = Some(format!("{} {}", qso.band, qso.mode));
                        // waypoint.sym = Some("Radio".to_string());
                        gpx.waypoints.push(waypoint);
                        row_count += 1;
                    }
                }
            }

            let file = match std::fs::File::create(&path) {
                Ok(file) => file,
                Err(e) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_ERR_CREATE_GPX)
                            .arg("err", &e)
                            .text(self.gui.appearance.language),
                    );
                    return;
                }
            };
            match gpx::write(&gpx, file) {
                Ok(()) => {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    self.gui.chrome.status_toast = Some((
                        format!("Wyeksportowano {row_count} QSO do GPX: {filename}"),
                        std::time::Instant::now(),
                    ));
                    let _ = open::that(&path);
                }
                Err(e) => {
                    self.gui.chrome.status_message = Some(
                        Msg::new(key::EXPORT_ERR_SAVE_GPX)
                            .arg("err", &e)
                            .text(self.gui.appearance.language),
                    );
                }
            }
        }
    }
}
