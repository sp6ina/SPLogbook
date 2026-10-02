// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

// Eksport logu (Cabrillo, PDF, GPX) — wydzielone z app.rs.
use super::*;

impl SpLogApp {
    pub fn export_cabrillo(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pliki Cabrillo (*.cbr, *.log)", &["cbr", "log"])
            .set_file_name(format!(
                "{}_{}.cbr",
                self.my_station.callsign,
                self.contest_name.replace(' ', "_")
            ))
            .set_title("Zapisz dziennik zawodów Cabrillo 3.0")
            .save_file()
        {
            let qsos = match self.log_db.lock() {
                Ok(db) => match db.get_recent_qsos_for_journal(&self.active_journal.id, 10000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.status_message = Some(format!("Błąd odczytu dziennika: {e}"));
                        return;
                    }
                },
                Err(e) => {
                    self.status_message = Some(format!("Nie można otworzyć dziennika: {e}"));
                    return;
                }
            };

            let mut out = String::new();
            out.push_str("START-OF-LOG: 3.0\n");
            let _ = writeln!(out, "CONTEST: {}", self.contest_name);
            let _ = writeln!(out, "CALLSIGN: {}", self.my_station.callsign);
            out.push_str("CATEGORY-OPERATOR: SINGLE-OP\n");
            out.push_str("CATEGORY-TRANSMITTER: ONE\n");
            out.push_str("CATEGORY-POWER: HIGH\n");
            out.push_str("CATEGORY-BAND: ALL\n");
            out.push_str("CATEGORY-MODE: MIXED\n");
            out.push_str("CATEGORY-STATION: FIXED\n");
            let total_score = self.contest_points * self.contest_mults.max(1);
            let _ = writeln!(out, "CLAIMED-SCORE: {total_score}");
            let _ = writeln!(out, "OPERATORS: {}", self.my_station.callsign);
            let _ = writeln!(out, "NAME: {}", self.my_station.operator);
            let _ = writeln!(
                out,
                "ADDRESS: {}, {}",
                self.my_station.city, self.my_station.country
            );
            out.push_str("SOAPBOX: Created with SPLogbook by SP6INA (GPLv3)\n");

            for (idx, q) in qsos.iter().rev().enumerate() {
                let freq_khz = q.freq.map_or_else(
                    || match q.band.as_str() {
                        "160m" => 1840,
                        "80m" => 3700,
                        "40m" => 7100,
                        "20m" => 14200,
                        "15m" => 21200,
                        "10m" => 28500,
                        _ => 14000,
                    },
                    |f| (f * 1000.0) as u64,
                );
                let date_str = if q.qso_date.len() == 8 && q.qso_date.is_ascii() {
                    format!(
                        "{}-{}-{}",
                        &q.qso_date[0..4],
                        &q.qso_date[4..6],
                        &q.qso_date[6..8]
                    )
                } else {
                    chrono::Utc::now().format("%Y-%m-%d").to_string()
                };
                let time_str = if q.time_on.len() >= 4 && q.time_on.is_ascii() {
                    q.time_on[0..4].to_string()
                } else {
                    chrono::Utc::now().format("%H%M").to_string()
                };

                let my_rst = if q.mode == "CW" { "599" } else { "59" };
                let his_rst = &q.rst_rcvd;
                let my_serial = idx + 1;
                let his_serial = q.srx.unwrap_or(1);

                let _ = writeln!(
                    out,
                    "QSO: {:5} {:2} {} {} {:10} {:3} {:03} {:10} {:3} {:03}",
                    freq_khz,
                    if q.mode == "CW" { "CW" } else { "PH" },
                    date_str,
                    time_str,
                    self.my_station.callsign,
                    my_rst,
                    my_serial,
                    q.callsign,
                    his_rst,
                    his_serial
                );
            }
            out.push_str("END-OF-LOG:\n");

            match std::fs::write(&path, out) {
                Ok(()) => {
                    self.status_message = Some(format!(
                        "Plik zawodów zapisany pomyślnie: {}",
                        path.display()
                    ));
                    self.status_toast = Some((
                        format!(
                            "Zapisano Cabrillo 3.0: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    self.status_message = Some(format!("Błąd zapisu pliku Cabrillo: {e}"));
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

            let qsos = match self.log_db.lock() {
                Ok(db) => match db.get_recent_qsos_for_journal(&self.active_journal.id, 10000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.status_message = Some(format!("Błąd odczytu dziennika: {e}"));
                        return;
                    }
                },
                Err(e) => {
                    self.status_message = Some(format!("Nie można otworzyć dziennika: {e}"));
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
                format!("Operator: {}", self.my_station.callsign),
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

            let mut y = 250.0f32;
            for (i, h) in headers.iter().enumerate() {
                ops.extend(text_ops(*h, 10.0, x_positions[i], y, true));
            }
            y -= 5.0;

            let mut row_count = 0usize;
            let mut page_num = 1usize;

            for qso in &qsos {
                if y < 20.0 {
                    doc.pages
                        .push(PdfPage::new(Mm(210.0), Mm(297.0), std::mem::take(&mut ops)));
                    y = 280.0;
                    page_num += 1;
                }
                ops.extend(text_ops(
                    qso.qso_date.as_str(),
                    10.0,
                    x_positions[0],
                    y,
                    false,
                ));
                ops.extend(text_ops(
                    qso.time_on.as_str(),
                    10.0,
                    x_positions[1],
                    y,
                    false,
                ));
                ops.extend(text_ops(
                    qso.callsign.as_str(),
                    10.0,
                    x_positions[2],
                    y,
                    false,
                ));
                ops.extend(text_ops(qso.band.as_str(), 10.0, x_positions[3], y, false));
                ops.extend(text_ops(qso.mode.as_str(), 10.0, x_positions[4], y, false));
                ops.extend(text_ops(
                    qso.rst_sent.as_str(),
                    10.0,
                    x_positions[5],
                    y,
                    false,
                ));
                ops.extend(text_ops(
                    qso.rst_rcvd.as_str(),
                    10.0,
                    x_positions[6],
                    y,
                    false,
                ));
                ops.extend(text_ops(
                    qso.country.as_deref().unwrap_or(""),
                    10.0,
                    x_positions[7],
                    y,
                    false,
                ));
                ops.extend(text_ops(
                    format!("{}/{}", qso.qsl_sent, qso.qsl_rcvd),
                    10.0,
                    x_positions[8],
                    y,
                    false,
                ));
                y -= 5.0;
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
                    self.status_toast = Some((
                        format!("Wyeksportowano {row_count} QSO do PDF: {filename}"),
                        std::time::Instant::now(),
                    ));
                    let _ = open::that(&path);
                }
                Err(e) => {
                    self.status_message = Some(format!("Błąd zapisu pliku PDF: {e}"));
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

            let qsos = match self.log_db.lock() {
                Ok(db) => match db.get_recent_qsos_for_journal(&self.active_journal.id, 10000) {
                    Ok(list) => list,
                    Err(e) => {
                        self.status_message = Some(format!("Błąd odczytu dziennika: {e}"));
                        return;
                    }
                },
                Err(e) => {
                    self.status_message = Some(format!("Nie można otworzyć dziennika: {e}"));
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
                    self.status_message = Some(format!("Błąd tworzenia pliku GPX: {e}"));
                    return;
                }
            };
            match gpx::write(&gpx, file) {
                Ok(()) => {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    self.status_toast = Some((
                        format!("Wyeksportowano {row_count} QSO do GPX: {filename}"),
                        std::time::Instant::now(),
                    ));
                    let _ = open::that(&path);
                }
                Err(e) => {
                    self.status_message = Some(format!("Błąd zapisu pliku GPX: {e}"));
                }
            }
        }
    }
}
