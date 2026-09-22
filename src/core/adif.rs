// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use std::collections::HashMap;
use std::io::{BufRead, Write};

/// Parser i generator formatu ADIF (Amateur Data Interchange Format) 3.1.4
pub struct AdifEngine;

impl AdifEngine {
    /// Parsuje strumień tekstowy ADIF do wektora rekordów QSO
    pub fn parse_reader<R: BufRead>(mut reader: R) -> Vec<QsoRecord> {
        let mut content = String::new();
        if reader.read_to_string(&mut content).is_err() {
            return Vec::new();
        }

        // Pomiń nagłówek (do znacznika <EOH>)
        let body = if let Some(pos) = content.to_ascii_uppercase().find("<EOH>") {
            &content[pos + 5..]
        } else {
            &content
        };

        let mut qsos = Vec::new();

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
                    if let Some(qso) = Self::fields_to_qso(&current_fields) {
                        qsos.push(qso);
                    }
                    current_fields.clear();
                    continue;
                }

                // Format tagu: NAZWA:DŁUGOŚĆ[:TYP]
                let parts: Vec<&str> = tag_content.split(':').collect();
                if parts.len() >= 2 {
                    let field_name = parts[0].trim().to_uppercase();
                    if let Ok(length) = parts[1].trim().parse::<usize>() {
                        let mut val = String::with_capacity(length);
                        for _ in 0..length {
                            if let Some(&(_, c)) = chars.peek() {
                                chars.next();
                                val.push(c);
                            }
                        }
                        current_fields.insert(field_name, val.trim().to_string());
                    }
                }
            } else {
                chars.next();
            }
        }

        // Jeśli na końcu pliku pozostały niezatwierdzone pola bez <EOR>
        if !current_fields.is_empty() {
            if let Some(qso) = Self::fields_to_qso(&current_fields) {
                qsos.push(qso);
            }
        }

        qsos
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
        if let Some(frx) = fields.get("FREQ_RX") {
            qso.freq_rx = frx.parse().ok();
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
            Self::write_field(&mut writer, "CALL", &q.callsign)?;
            Self::write_field(&mut writer, "BAND", &q.band)?;
            Self::write_field(&mut writer, "MODE", &q.mode)?;
            if let Some(ref sub) = q.submode {
                Self::write_field(&mut writer, "SUBMODE", sub)?;
            }
            Self::write_field(&mut writer, "QSO_DATE", &q.adif_date())?;
            Self::write_field(&mut writer, "TIME_ON", &q.adif_time())?;
            if let Some(ref toff) = q.time_off {
                Self::write_field(&mut writer, "TIME_OFF", toff)?;
            }
            if let Some(freq) = q.freq {
                Self::write_field(&mut writer, "FREQ", &format!("{:.6}", freq))?;
            }
            if let Some(frx) = q.freq_rx {
                Self::write_field(&mut writer, "FREQ_RX", &format!("{:.6}", frx))?;
            }
            Self::write_field(&mut writer, "RST_SENT", &q.rst_sent)?;
            Self::write_field(&mut writer, "RST_RCVD", &q.rst_rcvd)?;
            if let Some(ref name) = q.name {
                Self::write_field(&mut writer, "NAME", name)?;
            }
            if let Some(ref qth) = q.qth {
                Self::write_field(&mut writer, "QTH", qth)?;
            }
            if let Some(ref grid) = q.gridsquare {
                Self::write_field(&mut writer, "GRIDSQUARE", grid)?;
            }
            if let Some(ref st) = q.state {
                Self::write_field(&mut writer, "STATE", st)?;
            }
            if let Some(ref iota) = q.iota {
                Self::write_field(&mut writer, "IOTA", iota)?;
            }
            if let Some(ref sota) = q.sota_ref {
                Self::write_field(&mut writer, "SOTA_REF", sota)?;
            }
            if let Some(ref pota) = q.pota_ref {
                Self::write_field(&mut writer, "POTA_REF", pota)?;
            }
            if let Some(ref my_pota) = q.my_pota_ref {
                Self::write_field(&mut writer, "MY_POTA_REF", my_pota)?;
            }
            if let Some(ref my_sota) = q.my_sota_ref {
                Self::write_field(&mut writer, "MY_SOTA_REF", my_sota)?;
            }
            if let Some(ref vucc) = q.vucc_grids {
                Self::write_field(&mut writer, "VUCC_GRIDS", vucc)?;
            }
            if let Some(ref pga) = q.pga_ref {
                Self::write_field(&mut writer, "PGA_REF", pga)?;
            }
            if let Some(dxcc) = q.dxcc {
                Self::write_field(&mut writer, "DXCC", &dxcc.to_string())?;
            }
            if let Some(ref country) = q.country {
                Self::write_field(&mut writer, "COUNTRY", country)?;
            }
            if let Some(ref cont) = q.continent {
                Self::write_field(&mut writer, "CONT", cont)?;
            }
            if let Some(cq) = q.cqz {
                Self::write_field(&mut writer, "CQZ", &cq.to_string())?;
            }
            if let Some(itu) = q.ituz {
                Self::write_field(&mut writer, "ITUZ", &itu.to_string())?;
            }
            if let Some(ref c) = q.comment {
                Self::write_field(&mut writer, "COMMENT", c)?;
            }
            if let Some(ref qv) = q.qsl_via {
                Self::write_field(&mut writer, "QSL_VIA", qv)?;
            }
            if let Some(ref qm) = q.qsl_manager {
                Self::write_field(&mut writer, "QSL_VIA_MANAGER", qm)?;
            }
            if let Some(ref sn) = q.sat_name {
                Self::write_field(&mut writer, "SAT_NAME", sn)?;
            }
            if let Some(ref sm) = q.sat_mode {
                Self::write_field(&mut writer, "SAT_MODE", sm)?;
            }
            if let Some(ref pm) = q.prop_mode {
                Self::write_field(&mut writer, "PROP_MODE", pm)?;
            }
            if let Some(ref mg) = q.my_gridsquare {
                Self::write_field(&mut writer, "MY_GRIDSQUARE", mg)?;
            }
            if let Some(ref ms) = q.my_state {
                Self::write_field(&mut writer, "MY_STATE", ms)?;
            }
            Self::write_field(&mut writer, "QSL_SENT", &q.qsl_sent)?;
            Self::write_field(&mut writer, "QSL_RCVD", &q.qsl_rcvd)?;
            Self::write_field(&mut writer, "LOTW_QSL_SENT", &q.lotw_qsl_sent)?;
            Self::write_field(&mut writer, "LOTW_QSL_RCVD", &q.lotw_qsl_rcvd)?;
            Self::write_field(&mut writer, "EQSL_QSL_SENT", &q.eqsl_qsl_sent)?;
            Self::write_field(&mut writer, "EQSL_QSL_RCVD", &q.eqsl_qsl_rcvd)?;

            writeln!(writer, "<EOR>")?;
        }

        Ok(())
    }

    fn write_field<W: Write>(writer: &mut W, tag: &str, val: &str) -> std::io::Result<()> {
        if !val.is_empty() {
            write!(writer, "<{}:{}>{}", tag, val.chars().count(), val)?;
        }
        Ok(())
    }
}

pub fn parse_adif(content: &str) -> Vec<QsoRecord> {
    AdifEngine::parse_reader(std::io::Cursor::new(content.as_bytes()))
}

pub fn export_adif(qsos: &[QsoRecord], _prog: &str, _call: &str) -> String {
    let mut buffer = Vec::new();
    let _ = AdifEngine::export_to_writer(qsos, &mut buffer);
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
}
