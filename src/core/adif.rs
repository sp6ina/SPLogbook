// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use std::collections::HashMap;
use std::io::{BufRead, Write};

/// Parser i generator formatu ADIF (Amateur Data Interchange Format) 3.1.4
pub struct AdifEngine;

/// Wynik importu ADIF wraz z pełnym raportem odrzuconych rekordów i błędów.
pub struct AdifImportResult {
    /// Poprawnie sparsowane rekordy QSO.
    pub qsos: Vec<QsoRecord>,
    /// Liczba pomyślnie zaimportowanych rekordów (równa `qsos.len()`).
    pub imported: usize,
    /// Liczba rekordów odrzuconych (np. brak pola CALL).
    pub rejected: usize,
    /// Czytelne komunikaty o problemach napotkanych podczas parsowania.
    pub errors: Vec<String>,
}

impl AdifEngine {
    /// Parsuje strumień tekstowy ADIF do wektora rekordów QSO.
    /// Błędy odczytu są maskowane — użyj [`AdifEngine::parse_reader_with_report`],
    /// aby otrzymać pełny raport.
    pub fn parse_reader<R: BufRead>(reader: R) -> Vec<QsoRecord> {
        Self::parse_reader_with_report(reader)
            .map(|r| r.qsos)
            .unwrap_or_default()
    }

    /// Parsuje strumień ADIF i zwraca rekordy wraz z raportem błędów.
    pub fn parse_reader_with_report<R: BufRead>(
        mut reader: R,
    ) -> Result<AdifImportResult, String> {
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| format!("Błąd odczytu danych ADIF: {e}"))?;

        // Pomiń nagłówek (do znacznika <EOH>)
        let body = if let Some(pos) = content.to_ascii_uppercase().find("<EOH>") {
            &content[pos + 5..]
        } else {
            &content
        };

        let mut qsos = Vec::new();
        let mut errors = Vec::new();
        let mut rejected = 0usize;
        let mut record_index = 0usize;
        // Czy bieżący rekord zawiera błąd składni (np. nieprawidłowa długość pola).
        let mut record_has_error = false;

        // Bezpieczny stan maszyny parsowania
        let mut current_fields: HashMap<String, String> = HashMap::new();
        let mut chars = body.char_indices().peekable();

        while let Some(&(_, ch)) = chars.peek() {
            if ch == '<' {
                chars.next(); // pomiń '<'
                let mut tag_content = String::new();
                while let Some(&(_, c)) = chars.peek() {
                    chars.next();
                    if c == '>' {
                        break;
                    }
                    tag_content.push(c);
                }

                let tag_upper = tag_content.trim().to_uppercase();
                if tag_upper == "EOR" {
                    record_index += 1;
                    if record_has_error {
                        // Rekord z błędem składni jest odrzucany; komunikat już zapisano.
                        rejected += 1;
                    } else {
                        match Self::fields_to_qso(&current_fields) {
                            Some(qso) => qsos.push(qso),
                            None => {
                                rejected += 1;
                                errors.push(format!(
                                    "Rekord {} odrzucony: brak wymaganego pola CALL.",
                                    record_index
                                ));
                            }
                        }
                    }
                    current_fields.clear();
                    record_has_error = false;
                    continue;
                }

                // Format tagu: NAZWA:DŁUGOŚĆ[:TYP]
                let parts: Vec<&str> = tag_content.split(':').collect();
                if parts.len() >= 2 {
                    let field_name = parts[0].trim().to_uppercase();
                    match parts[1].trim().parse::<usize>() {
                        Ok(length) => {
                            let mut val = String::with_capacity(length);
                            for _ in 0..length {
                                if let Some(&(_, c)) = chars.peek() {
                                    chars.next();
                                    val.push(c);
                                }
                            }
                            current_fields.insert(field_name, val.trim().to_string());
                        }
                        Err(_) => {
                            errors.push(format!(
                                "Nieprawidłowa długość pola „{}”.",
                                field_name
                            ));
                            record_has_error = true;
                        }
                    }
                }
            } else {
                chars.next();
            }
        }

        // Jeśli na końcu pliku pozostały niezatwierdzone pola bez <EOR>
        if !current_fields.is_empty() {
            record_index += 1;
            if record_has_error {
                rejected += 1;
            } else {
                match Self::fields_to_qso(&current_fields) {
                    Some(qso) => qsos.push(qso),
                    None => {
                        rejected += 1;
                        errors.push(format!(
                            "Rekord {} odrzucony: brak wymaganego pola CALL.",
                            record_index
                        ));
                    }
                }
            }
        }

        Ok(AdifImportResult {
            imported: qsos.len(),
            qsos,
            rejected,
            errors,
        })
    }

    /// Konwertuje mapę pól ADIF na rekord QsoRecord
    fn fields_to_qso(fields: &HashMap<String, String>) -> Option<QsoRecord> {
        let call = fields.get("CALL")?;
        if call.is_empty() {
            return None;
        }

        let band = fields.get("BAND").cloned().unwrap_or_else(|| "20m".to_string());
        let mode = fields.get("MODE").cloned().unwrap_or_else(|| "CW".to_string());

        let mut qso = QsoRecord::new(call, band, mode);

        if let Some(sub) = fields.get("SUBMODE") {
            qso.submode = Some(sub.clone());
        }
        if let Some(d) = fields.get("QSO_DATE") {
            qso.qso_date = d.clone();
        }
        if let Some(t) = fields.get("TIME_ON") {
            qso.time_on = t.clone();
        }
        if let Some(t) = fields.get("TIME_OFF") {
            qso.time_off = Some(t.clone());
        }
        if let Some(f) = fields.get("FREQ") {
            qso.freq = f.parse().ok();
        }
        if let Some(f) = fields.get("FREQ_RX") {
            qso.freq_rx = f.parse().ok();
        }
        if let Some(rst) = fields.get("RST_SENT") {
            qso.rst_sent = rst.clone();
        }
        if let Some(rst) = fields.get("RST_RCVD") {
            qso.rst_rcvd = rst.clone();
        }
        if let Some(name) = fields.get("NAME") {
            qso.name = Some(name.clone());
        }
        if let Some(qth) = fields.get("QTH") {
            qso.qth = Some(qth.clone());
        }
        if let Some(grid) = fields.get("GRIDSQUARE") {
            qso.gridsquare = Some(grid.clone());
        }
        if let Some(st) = fields.get("STATE") {
            qso.state = Some(st.clone());
        }
        if let Some(iota) = fields.get("IOTA") {
            qso.iota = Some(iota.clone());
        }
        if let Some(sota) = fields.get("SOTA_REF") {
            qso.sota_ref = Some(sota.clone());
        }
        if let Some(pota) = fields.get("POTA_REF") {
            qso.pota_ref = Some(pota.clone());
        }
        if let Some(my_pota) = fields.get("MY_POTA_REF") {
            qso.my_pota_ref = Some(my_pota.clone());
        }
        if let Some(my_sota) = fields.get("MY_SOTA_REF") {
            qso.my_sota_ref = Some(my_sota.clone());
        }
        if let Some(vucc) = fields.get("VUCC_GRIDS") {
            qso.vucc_grids = Some(vucc.clone());
        }
        if let Some(pga) = fields.get("PGA_REF").or_else(|| fields.get("PGA")) {
            qso.pga_ref = Some(pga.clone());
        }
        if let Some(dxcc) = fields.get("DXCC") {
            qso.dxcc = dxcc.parse().ok();
        }
        if let Some(cnt) = fields.get("COUNTRY") {
            qso.country = Some(cnt.clone());
        }
        if let Some(cont) = fields.get("CONT") {
            qso.continent = Some(cont.clone());
        }
        if let Some(cq) = fields.get("CQZ") {
            qso.cqz = cq.parse().ok();
        }
        if let Some(itu) = fields.get("ITUZ") {
            qso.ituz = itu.parse().ok();
        }
        if let Some(c) = fields.get("COMMENT") {
            qso.comment = Some(c.clone());
        }
        if let Some(q) = fields.get("QSL_SENT") {
            qso.qsl_sent = q.clone();
        }
        if let Some(q) = fields.get("QSL_RCVD") {
            qso.qsl_rcvd = q.clone();
        }
        if let Some(q) = fields.get("LOTW_QSL_SENT") {
            qso.lotw_qsl_sent = q.clone();
        }
        if let Some(q) = fields.get("LOTW_QSL_RCVD") {
            qso.lotw_qsl_rcvd = q.clone();
        }
        if let Some(q) = fields.get("EQSL_QSL_SENT") {
            qso.eqsl_qsl_sent = q.clone();
        }
        if let Some(q) = fields.get("EQSL_QSL_RCVD") {
            qso.eqsl_qsl_rcvd = q.clone();
        }
        if let Some(sn) = fields.get("SAT_NAME") {
            qso.sat_name = Some(sn.clone());
        }
        if let Some(sm) = fields.get("SAT_MODE") {
            qso.sat_mode = Some(sm.clone());
        }
        if let Some(pm) = fields.get("PROP_MODE") {
            qso.prop_mode = Some(pm.clone());
        }
        if let Some(mg) = fields.get("MY_GRIDSQUARE") {
            qso.my_gridsquare = Some(mg.clone());
        }
        if let Some(ms) = fields.get("MY_STATE") {
            qso.my_state = Some(ms.clone());
        }
        if let Some(qv) = fields.get("QSL_VIA") {
            qso.qsl_via = Some(qv.clone());
        }
        if let Some(qm) = fields.get("QSL_VIA_MANAGER").or_else(|| fields.get("QSL_MANAGER")) {
            qso.qsl_manager = Some(qm.clone());
        }

        Some(qso)
    }

    /// Eksportuje listę łączności do formatu ADIF 3.1.4
    pub fn export_to_writer<W: Write>(qsos: &[QsoRecord], mut writer: W) -> std::io::Result<()> {
        writeln!(writer, "SPLogbook ADIF 3.1.5 Export")?;
        writeln!(writer, "Author: Mariusz Wozniak (SP6INA)")?;
        writeln!(writer, "<ADIF_VER:5>3.1.5")?;
        writeln!(writer, "<PROGRAMID:9>SPLogbook")?;
        writeln!(writer, "<PROGRAMVERSION:5>1.0.3")?;
        writeln!(writer, "<EOH>")?;

        for q in qsos {
            for (tag, val) in Self::qso_fields(q) {
                Self::write_field(&mut writer, tag, &val)?;
            }
            writeln!(writer, "<EOR>")?;
        }

        Ok(())
    }

    /// Zwraca uporządkowaną listę pól ADIF rekordu QSO jako pary `(nazwa, wartość)`.
    fn qso_fields(q: &QsoRecord) -> Vec<(&'static str, String)> {
        let mut f: Vec<(&'static str, String)> = Vec::new();
        f.push(("CALL", q.callsign.clone()));
        f.push(("BAND", q.band.clone()));
        f.push(("MODE", q.mode.clone()));
        if let Some(ref v) = q.submode {
            f.push(("SUBMODE", v.clone()));
        }
        f.push(("QSO_DATE", q.adif_date()));
        f.push(("TIME_ON", q.adif_time()));
        if let Some(ref v) = q.time_off {
            f.push(("TIME_OFF", v.clone()));
        }
        if let Some(v) = q.freq {
            f.push(("FREQ", format!("{:.6}", v)));
        }
        if let Some(v) = q.freq_rx {
            f.push(("FREQ_RX", format!("{:.6}", v)));
        }
        f.push(("RST_SENT", q.rst_sent.clone()));
        f.push(("RST_RCVD", q.rst_rcvd.clone()));
        if let Some(ref v) = q.name {
            f.push(("NAME", v.clone()));
        }
        if let Some(ref v) = q.qth {
            f.push(("QTH", v.clone()));
        }
        if let Some(ref v) = q.gridsquare {
            f.push(("GRIDSQUARE", v.clone()));
        }
        if let Some(ref v) = q.state {
            f.push(("STATE", v.clone()));
        }
        if let Some(ref v) = q.iota {
            f.push(("IOTA", v.clone()));
        }
        if let Some(ref v) = q.sota_ref {
            f.push(("SOTA_REF", v.clone()));
        }
        if let Some(ref v) = q.pota_ref {
            f.push(("POTA_REF", v.clone()));
        }
        if let Some(ref v) = q.my_pota_ref {
            f.push(("MY_POTA_REF", v.clone()));
        }
        if let Some(ref v) = q.my_sota_ref {
            f.push(("MY_SOTA_REF", v.clone()));
        }
        if let Some(ref v) = q.vucc_grids {
            f.push(("VUCC_GRIDS", v.clone()));
        }
        if let Some(ref v) = q.pga_ref {
            f.push(("PGA_REF", v.clone()));
        }
        if let Some(v) = q.dxcc {
            f.push(("DXCC", v.to_string()));
        }
        if let Some(ref v) = q.country {
            f.push(("COUNTRY", v.clone()));
        }
        if let Some(ref v) = q.continent {
            f.push(("CONT", v.clone()));
        }
        if let Some(v) = q.cqz {
            f.push(("CQZ", v.to_string()));
        }
        if let Some(v) = q.ituz {
            f.push(("ITUZ", v.to_string()));
        }
        if let Some(ref v) = q.comment {
            f.push(("COMMENT", v.clone()));
        }
        if let Some(ref v) = q.qsl_via {
            f.push(("QSL_VIA", v.clone()));
        }
        if let Some(ref v) = q.qsl_manager {
            f.push(("QSL_VIA_MANAGER", v.clone()));
        }
        if let Some(ref v) = q.sat_name {
            f.push(("SAT_NAME", v.clone()));
        }
        if let Some(ref v) = q.sat_mode {
            f.push(("SAT_MODE", v.clone()));
        }
        if let Some(ref v) = q.prop_mode {
            f.push(("PROP_MODE", v.clone()));
        }
        if let Some(ref v) = q.my_gridsquare {
            f.push(("MY_GRIDSQUARE", v.clone()));
        }
        if let Some(ref v) = q.my_state {
            f.push(("MY_STATE", v.clone()));
        }
        f.push(("QSL_SENT", q.qsl_sent.clone()));
        f.push(("QSL_RCVD", q.qsl_rcvd.clone()));
        f.push(("LOTW_QSL_SENT", q.lotw_qsl_sent.clone()));
        f.push(("LOTW_QSL_RCVD", q.lotw_qsl_rcvd.clone()));
        f.push(("EQSL_QSL_SENT", q.eqsl_qsl_sent.clone()));
        f.push(("EQSL_QSL_RCVD", q.eqsl_qsl_rcvd.clone()));
        f
    }

    fn write_field<W: Write>(writer: &mut W, tag: &str, val: &str) -> std::io::Result<()> {
        if !val.is_empty() {
            write!(writer, "<{}:{}>{}", tag, val.chars().count(), val)?;
        }
        Ok(())
    }

    fn write_xml_field<W: Write>(writer: &mut W, tag: &str, val: &str) -> std::io::Result<()> {
        if !val.is_empty() {
            writeln!(writer, "      <{}>{}</{}>", tag, xml_escape(val), tag)?;
        }
        Ok(())
    }

    /// Eksportuje rekordy do formatu ADX (XML ADIF).
    pub fn export_adx_to_writer<W: Write>(qsos: &[QsoRecord], mut writer: W) -> std::io::Result<()> {
        writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(writer, "<ADX>")?;
        writeln!(writer, "  <HEADER>")?;
        writeln!(writer, "    <ADIF_VER>3.1.5</ADIF_VER>")?;
        writeln!(writer, "    <PROGRAMID>SPLogbook</PROGRAMID>")?;
        writeln!(writer, "    <PROGRAMVERSION>1.0.3</PROGRAMVERSION>")?;
        writeln!(writer, "  </HEADER>")?;
        writeln!(writer, "  <RECORDS>")?;

        for q in qsos {
            writeln!(writer, "    <RECORD>")?;
            for (tag, val) in Self::qso_fields(q) {
                Self::write_xml_field(&mut writer, tag, &val)?;
            }
            writeln!(writer, "    </RECORD>")?;
        }

        writeln!(writer, "  </RECORDS>")?;
        writeln!(writer, "</ADX>")?;

        Ok(())
    }
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn parse_adif(content: &str) -> Vec<QsoRecord> {
    AdifEngine::parse_reader(std::io::Cursor::new(content.as_bytes()))
}

pub fn parse_adif_with_report(content: &str) -> AdifImportResult {
    AdifEngine::parse_reader_with_report(std::io::Cursor::new(content.as_bytes())).unwrap_or_else(
        |e| AdifImportResult {
            qsos: Vec::new(),
            imported: 0,
            rejected: 0,
            errors: vec![e],
        },
    )
}

pub fn export_adif(qsos: &[QsoRecord], _prog: &str, _call: &str) -> String {
    let mut buffer = Vec::new();
    let _ = AdifEngine::export_to_writer(qsos, &mut buffer);
    String::from_utf8_lossy(&buffer).to_string()
}

pub fn export_adx(qsos: &[QsoRecord]) -> String {
    let mut buffer = Vec::new();
    let _ = AdifEngine::export_adx_to_writer(qsos, &mut buffer);
    String::from_utf8_lossy(&buffer).to_string()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adif_roundtrip() {
        let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
        qso.name = Some("Mariusz".to_string());
        qso.gridsquare = Some("JO81WA".to_string());
        qso.pga_ref = Some("WR01".to_string());

        let mut buffer = Vec::new();
        AdifEngine::export_to_writer(&[qso.clone()], &mut buffer).unwrap();

        let parsed = AdifEngine::parse_reader(buffer.as_slice());
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].callsign, "SP6INA");
        assert_eq!(parsed[0].band, "20m");
        assert_eq!(parsed[0].mode, "CW");
        assert_eq!(parsed[0].name.as_deref(), Some("Mariusz"));
        assert_eq!(parsed[0].gridsquare.as_deref(), Some("JO81WA"));
        assert_eq!(parsed[0].pga_ref.as_deref(), Some("WR01"));
    }

    #[test]
    fn test_adif_utf8_and_award_fields_roundtrip() {
        let mut qso = QsoRecord::new("SP6INA/P", "40m", "SSB");
        qso.name = Some("Stanisław".to_string());
        qso.qth = Some("Kraków".to_string());
        qso.comment = Some("Łączność terenowa z żółtym namiotem".to_string());
        qso.state = Some("CA".to_string());
        qso.iota = Some("EU-132".to_string());
        qso.sota_ref = Some("SP/BZ-001".to_string());
        qso.pota_ref = Some("PL-0042".to_string());
        qso.sat_name = Some("AO-91".to_string());
        qso.sat_mode = Some("V/U".to_string());
        qso.prop_mode = Some("SAT".to_string());
        qso.qsl_via = Some("DIRECT".to_string());
        qso.qsl_manager = Some("SP6IXU".to_string());

        let mut buffer = Vec::new();
        AdifEngine::export_to_writer(&[qso.clone()], &mut buffer).unwrap();

        let parsed = AdifEngine::parse_reader(buffer.as_slice());
        assert_eq!(parsed.len(), 1);
        let p = &parsed[0];
        assert_eq!(p.callsign, "SP6INA/P");
        assert_eq!(p.name.as_deref(), Some("Stanisław"));
        assert_eq!(p.qth.as_deref(), Some("Kraków"));
        assert_eq!(p.comment.as_deref(), Some("Łączność terenowa z żółtym namiotem"));
        assert_eq!(p.state.as_deref(), Some("CA"));
        assert_eq!(p.iota.as_deref(), Some("EU-132"));
        assert_eq!(p.sota_ref.as_deref(), Some("SP/BZ-001"));
        assert_eq!(p.pota_ref.as_deref(), Some("PL-0042"));
        assert_eq!(p.sat_name.as_deref(), Some("AO-91"));
        assert_eq!(p.sat_mode.as_deref(), Some("V/U"));
        assert_eq!(p.prop_mode.as_deref(), Some("SAT"));
        assert_eq!(p.qsl_via.as_deref(), Some("DIRECT"));
        assert_eq!(p.qsl_manager.as_deref(), Some("SP6IXU"));
    }

    #[test]
    fn test_adif_import_report_tracks_rejected_and_errors() {
        // Nagłówek + jeden poprawny rekord + jeden bez CALL + zła długość pola.
        let content = concat!(
            "SPLogbook test export\n",
            "<EOH>\n",
            "<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<EOR>\n",
            "<BAND:3>40m<MODE:3>SSB<EOR>\n",
            "<CALL:6>DL1ABC<BAD:xyz>??<EOR>\n",
        );

        let report = parse_adif_with_report(content);
        assert_eq!(report.imported, 1);
        assert_eq!(report.rejected, 2);
        assert_eq!(report.qsos.len(), 1);
        assert_eq!(report.qsos[0].callsign, "SP6INA");
        assert!(report.errors.iter().any(|e| e.contains("brak wymaganego pola CALL")));
        assert!(report.errors.iter().any(|e| e.contains("Nieprawidłowa długość pola")));
    }

    #[test]
    fn test_adif_import_trailing_record_without_eor() {
        let content = "<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW";
        let report = parse_adif_with_report(content);
        assert_eq!(report.imported, 1);
        assert_eq!(report.qsos[0].callsign, "SP6INA");
    }

    #[test]
    fn test_adx_export_well_formed_and_escaped() {
        let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
        qso.name = Some("A&B <test>".to_string());
        qso.comment = Some("cudzysłów \" i apostrof '".to_string());

        let adx = export_adx(&[qso]);
        assert!(adx.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(adx.contains("<ADX>"));
        assert!(adx.contains("<ADIF_VER>3.1.5</ADIF_VER>"));
        assert!(adx.contains("<RECORD>"));
        assert!(adx.contains("<CALL>SP6INA</CALL>"));
        assert!(adx.contains("<NAME>A&amp;B &lt;test&gt;</NAME>"));
        assert!(adx.contains("cudzysłów &quot; i apostrof &apos;"));
        assert!(adx.trim_end().ends_with("</ADX>"));
    }
}
