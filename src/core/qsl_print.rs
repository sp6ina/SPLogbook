// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use serde::{Deserialize, Serialize};

/// Etykieta na papierową kartę QSL (wzorem Log4OM / QLog)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QslLabel {
    pub to_call: String,
    pub date: String,
    pub time_utc: String,
    pub band: String,
    pub mode: String,
    pub rst_sent: String,
    pub qsl_msg: String,
    pub my_call: String,
    pub my_grid: String,
}

impl QslLabel {
    pub fn from_qso(qso: &QsoRecord, my_call: &str, my_grid: &str) -> Self {
        Self {
            to_call: qso.callsign.clone(),
            date: qso.qso_date.clone(),
            time_utc: qso.time_on.clone(),
            band: qso.band.clone(),
            mode: qso.mode.clone(),
            rst_sent: qso.rst_sent.clone(),
            qsl_msg: qso.comment.clone().unwrap_or_else(|| "TNX 73!".to_string()),
            my_call: my_call.to_string(),
            my_grid: my_grid.to_string(),
        }
    }

    /// Generuje sformatowaną etykietę tekstową (dla drukarek etykiet termicznych lub podglądu)
    pub fn format_text(&self) -> String {
        format!(
            "----------------------------------------\n\
             Confirming QSO with: {}\n\
             Date: {}  Time: {} UTC\n\
             Band: {:<6} Mode: {:<6} RST: {}\n\
             From: {} (QTH: {})\n\
             Msg:  {}\n\
             ----------------------------------------",
            self.to_call, self.date, self.time_utc, self.band, self.mode, self.rst_sent,
            self.my_call, self.my_grid, self.qsl_msg
        )
    }

    /// Generuje kod HTML arkusza etykiet (A4, 24 etykiety 70x37mm lub 16 etykiet) gotowy do wydruku
    pub fn generate_html_sheet(labels: &[QslLabel]) -> String {
        let mut html = String::from(
            "<!DOCTYPE html><html><head><meta charset='utf-8'>\n\
             <style>\n\
             body { font-family: 'Segoe UI', Arial, sans-serif; margin: 0; padding: 10mm; }\n\
             .grid { display: grid; grid-template-columns: repeat(3, 70mm); grid-gap: 2mm; }\n\
             .label { border: 1px dashed #bbb; padding: 4mm; height: 35mm; box-sizing: border-box; font-size: 11px; }\n\
             .call { font-size: 14px; font-weight: bold; color: #004488; }\n\
             .meta { font-size: 9px; color: #555; }\n\
             </style></head><body><div class='grid'>\n"
        );

        for l in labels {
            html.push_str(&format!(
                "<div class='label'>\n\
                 <div>Confirming QSO: <span class='call'>{}</span></div>\n\
                 <div><b>{}</b> {}z | <b>{}</b> {}</div>\n\
                 <div>2-way RST: <b>{}</b></div>\n\
                 <div class='meta'>Op: {} ({}) | {}</div>\n\
                 </div>\n",
                Self::escape_html(&l.to_call),
                Self::escape_html(&l.date),
                Self::escape_html(&l.time_utc),
                Self::escape_html(&l.band),
                Self::escape_html(&l.mode),
                Self::escape_html(&l.rst_sent),
                Self::escape_html(&l.my_call),
                Self::escape_html(&l.my_grid),
                Self::escape_html(&l.qsl_msg),
            ));
        }

        html.push_str("</div></body></html>");
        html
    }

    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qsl_label_generation() {
        let qso = QsoRecord::new("W1AW", "20m", "CW");
        let label = QslLabel::from_qso(&qso, "SP6INA", "JO81WA");
        assert_eq!(label.to_call, "W1AW");
        assert_eq!(label.my_call, "SP6INA");

        let txt = label.format_text();
        assert!(txt.contains("Confirming QSO with: W1AW"));

        let html = QslLabel::generate_html_sheet(&[label]);
        assert!(html.contains("SP6INA"));
        assert!(html.contains("W1AW"));
    }
}
