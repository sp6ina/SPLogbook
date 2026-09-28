// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use std::collections::HashMap;
use std::io::{BufRead, Write};

/// Aktualna wspierana wersja specyfikacji ADIF (marzec 2026).
pub const ADIF_VERSION: &str = "3.1.7";

/// Parser i generator formatu ADIF (Amateur Data Interchange Format) 3.1.7
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
    pub fn parse_reader_with_report<R: BufRead>(mut reader: R) -> Result<AdifImportResult, String> {
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| format!("Błąd odczytu danych ADIF: {e}"))?;

        // Pomiń nagłówek (do znacznika <EOH>) bez alokacji kopii całego pliku
        let body = if let Some(end_pos) = find_tag_end_ci(content.as_bytes(), 0, b"<EOH>") {
            &content[end_pos..]
        } else {
            &content
        };

        let mut qsos = Vec::new();
        let mut errors = Vec::new();
        let mut rejected = 0usize;
        let mut record_index = 0usize;
        // Czy bieżący rekord zawiera błąd składni (np. nieprawidłowa długość pola).
        let mut record_has_error = false;

        // Bezpieczny stan maszyny parsowania operującej na bajtach UTF-8 (zgodnie ze specyfikacją ADIF)
        let mut current_fields: HashMap<String, String> = HashMap::new();
        let bytes = body.as_bytes();
        let mut idx = 0;

        while idx < bytes.len() {
            if bytes[idx] == b'<' {
                idx += 1; // pomiń '<'
                let start_tag = idx;
                while idx < bytes.len() && bytes[idx] != b'>' {
                    idx += 1;
                }
                if idx >= bytes.len() {
                    break;
                }
                let tag_raw = &bytes[start_tag..idx];
                idx += 1; // pomiń '>'

                let tag_str = String::from_utf8_lossy(tag_raw);
                let tag_upper = tag_str.trim().to_uppercase();
                if tag_upper == "EOR" {
                    record_index += 1;
                    if record_has_error {
                        // Rekord z błędem składni jest odrzucany; komunikat już zapisano.
                        rejected += 1;
                    } else if let Some(qso) = Self::fields_to_qso(&current_fields) {
                        qsos.push(qso);
                    } else {
                        rejected += 1;
                        errors.push(format!(
                            "Rekord {record_index} odrzucony: brak wymaganego pola CALL."
                        ));
                    }
                    current_fields.clear();
                    record_has_error = false;
                    continue;
                }

                // Format tagu: NAZWA:DŁUGOŚĆ[:TYP]
                let parts: Vec<&str> = tag_str.split(':').collect();
                if parts.len() >= 2 {
                    let field_name = parts[0].trim().to_uppercase();
                    if let Ok(length) = parts[1].trim().parse::<usize>() {
                        let Some(end_idx) = idx.checked_add(length) else {
                            errors.push(format!(
                                "Rekord {record_index}: pole „{field_name}” deklaruje przepełnienie długości ({length})."
                            ));
                            record_has_error = true;
                            break;
                        };
                        // Zadeklarowana długość musi mieścić się w buforze; w przeciwnym
                        // razie plik jest ucięty/uszkodzony i rekord należy odrzucić.
                        if end_idx > bytes.len() {
                            errors.push(format!(
                                "Rekord {record_index}: pole „{field_name}” deklaruje {length} bajtów, ale dostępnych jest tylko {}.",
                                bytes.len().saturating_sub(idx)
                            ));
                            record_has_error = true;
                            idx = bytes.len();
                            continue;
                        }
                        let val_bytes = &bytes[idx..end_idx];
                        let val_str = String::from_utf8_lossy(val_bytes);
                        // Pola tekstowe, w których spacje brzegowe mogą być celowe,
                        // zachowujemy w całości; pozostałe (tokeny, daty, referencje)
                        // są przycinane zgodnie ze specyfikacją ADIF.
                        let preserve_whitespace = matches!(
                            field_name.as_str(),
                            "COMMENT" | "NAME" | "QTH" | "NOTES" | "ADDRESS"
                        );
                        let value = if preserve_whitespace {
                            val_str.into_owned()
                        } else {
                            val_str.trim().to_string()
                        };
                        current_fields.insert(field_name, value);
                        idx = end_idx;
                    } else {
                        errors.push(format!("Nieprawidłowa długość pola „{field_name}”."));
                        record_has_error = true;
                    }
                }
            } else {
                idx += 1;
            }
        }

        // Jeśli na końcu pliku pozostały niezatwierdzone pola bez <EOR>
        // (lub rekord z błędem składni, który nie został zamknięty znacznikiem)
        if !current_fields.is_empty() || record_has_error {
            record_index += 1;
            if record_has_error {
                rejected += 1;
            } else if let Some(qso) = Self::fields_to_qso(&current_fields) {
                qsos.push(qso);
            } else {
                rejected += 1;
                errors.push(format!(
                    "Rekord {record_index} odrzucony: brak wymaganego pola CALL."
                ));
            }
        }

        Ok(AdifImportResult {
            imported: qsos.len(),
            qsos,
            rejected,
            errors,
        })
    }

    /// Mapuje nazwę emisji używaną w radiostacji/aplikacji na oficjalną parę `(MODE, Option<SUBMODE>)`
    /// zgodną ze specyfikacją ADIF 3.1.7 (marzec 2026).
    pub fn normalize_mode_submode(
        mode: &str,
        submode: Option<&str>,
    ) -> (&'static str, Option<&'static str>) {
        let m = mode.trim().to_uppercase();
        let sub = submode.map(|s| s.trim().to_uppercase());
        let effective = sub.as_deref().unwrap_or(m.as_str());

        match effective {
            // ADIF 3.1.7: MFSK submodes (w tym nowe FT2)
            "FT4" => ("MFSK", Some("FT4")),
            "FT2" => ("MFSK", Some("FT2")),
            "JS8" => ("MFSK", Some("JS8")),
            "Q65" => ("MFSK", Some("Q65")),
            "FST4" => ("MFSK", Some("FST4")),
            "FST4W" => ("MFSK", Some("FST4W")),
            "MFSK4" => ("MFSK", Some("MFSK4")),
            "MFSK8" => ("MFSK", Some("MFSK8")),
            "MFSK16" => ("MFSK", Some("MFSK16")),
            "MFSK32" => ("MFSK", Some("MFSK32")),
            "FSQCALL" => ("MFSK", Some("FSQCALL")),
            // ADIF 3.1.7: DYNAMIC submodes (w tym nowe FREEDATA)
            "FREEDATA" => ("DYNAMIC", Some("FREEDATA")),
            "VARA HF" | "VARA_HF" => ("DYNAMIC", Some("VARA HF")),
            "VARA FM" | "VARA_FM" => ("DYNAMIC", Some("VARA FM")),
            "VARA SATELLITE" => ("DYNAMIC", Some("VARA SATELLITE")),
            // ADIF 3.1.7: nowa emisja główna OFDM i jej submode'y RIBBIT
            "RIBBIT_PIX" => ("OFDM", Some("RIBBIT_PIX")),
            "RIBBIT_SMS" => ("OFDM", Some("RIBBIT_SMS")),
            "OFDM" => ("OFDM", None),
            // ADIF 3.1.6: SCAMP submodes dla FSK oraz MTONE
            "SCAMP_FAST" => ("FSK", Some("SCAMP_FAST")),
            "SCAMP_SLOW" => ("FSK", Some("SCAMP_SLOW")),
            "SCAMP_VSLOW" => ("FSK", Some("SCAMP_VSLOW")),
            "SCAMP_OO" => ("MTONE", Some("SCAMP_OO")),
            "SCAMP_OO_SLW" => ("MTONE", Some("SCAMP_OO_SLW")),
            // SSB submodes
            "USB" => ("SSB", Some("USB")),
            "LSB" => ("SSB", Some("LSB")),
            // PSK submodes
            "PSK31" | "BPSK31" => ("PSK", Some("PSK31")),
            "PSK63" | "BPSK63" => ("PSK", Some("PSK63")),
            "PSK125" | "BPSK125" => ("PSK", Some("PSK125")),
            "QPSK31" => ("PSK", Some("QPSK31")),
            "QPSK63" => ("PSK", Some("QPSK63")),
            "QPSK125" => ("PSK", Some("QPSK125")),
            // DIGITALVOICE submodes
            "DMR" => ("DIGITALVOICE", Some("DMR")),
            "C4FM" => ("DIGITALVOICE", Some("C4FM")),
            "DSTAR" | "D-STAR" => ("DIGITALVOICE", Some("DSTAR")),
            "FREEDV" => ("DIGITALVOICE", Some("FREEDV")),
            "M17" => ("DIGITALVOICE", Some("M17")),
            // Standardowe emisje główne
            "CW" => ("CW", None),
            "SSB" => ("SSB", None),
            "AM" => ("AM", None),
            "FM" => ("FM", None),
            "FT8" => ("FT8", None),
            "RTTY" => ("RTTY", None),
            "SSTV" => ("SSTV", None),
            "WSPR" => ("WSPR", None),
            "JT65" => ("JT65", None),
            "JT9" => ("JT9", None),
            "MSK144" => ("MSK144", None),
            "OLIVIA" => ("OLIVIA", None),
            "CONTESTIA" => ("CONTESTIA", None),
            "HELL" => ("HELL", None),
            "PKT" => ("PKT", None),
            "ATV" => ("ATV", None),
            _ => ("OTHER", None),
        }
    }

    /// Konwertuje mapę pól ADIF na rekord QsoRecord
    fn fields_to_qso(fields: &HashMap<String, String>) -> Option<QsoRecord> {
        let call = fields.get("CALL")?;
        if call.is_empty() {
            return None;
        }

        let parsed_freq: Option<f64> = fields.get("FREQ").and_then(|f| f.parse().ok());
        let band = fields
            .get("BAND")
            .filter(|b| !b.is_empty())
            .cloned()
            .or_else(|| {
                parsed_freq
                    .filter(|f| f.is_finite() && *f > 0.0)
                    .and_then(|f_mhz| {
                        crate::core::bandplan::get_band_by_freq(
                            (f_mhz * 1_000_000.0).round() as u64,
                        )
                    })
                    .map(|b| b.name.to_string())
            })
            .unwrap_or_else(|| "20m".to_string());
        let mode = fields
            .get("MODE")
            .cloned()
            .unwrap_or_else(|| "CW".to_string());

        let mut qso = QsoRecord::new(call, band, mode);

        if let Some(sub) = fields.get("SUBMODE").filter(|s| !s.is_empty()) {
            let (norm_mode, norm_sub) = Self::normalize_mode_submode(&qso.mode, Some(sub));
            if norm_mode != "OTHER" {
                qso.mode = norm_mode.to_string();
                qso.submode = norm_sub.map(str::to_string).or_else(|| Some(sub.clone()));
            } else {
                qso.submode = Some(sub.clone());
            }
        }
        if let Some(d) = fields.get("QSO_DATE") {
            qso.qso_date.clone_from(d);
        }
        if let Some(t) = fields.get("TIME_ON") {
            qso.time_on.clone_from(t);
        }
        if let Some(t) = fields.get("TIME_OFF") {
            qso.time_off = Some(t.clone());
        }
        if parsed_freq.is_some() {
            qso.freq = parsed_freq;
        }
        if let Some(f) = fields.get("FREQ_RX") {
            qso.freq_rx = f.parse().ok();
        }
        if let Some(rst) = fields.get("RST_SENT") {
            qso.rst_sent.clone_from(rst);
        }
        if let Some(rst) = fields.get("RST_RCVD") {
            qso.rst_rcvd.clone_from(rst);
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
            qso.qsl_sent.clone_from(q);
        }
        if let Some(q) = fields.get("QSL_RCVD") {
            qso.qsl_rcvd.clone_from(q);
        }
        if let Some(qd) = fields.get("QSLSDATE") {
            qso.qsl_sent_date = Some(qd.clone());
        }
        if let Some(qd) = fields.get("QSLRDATE") {
            qso.qsl_rcvd_date = Some(qd.clone());
        }
        if let Some(q) = fields.get("LOTW_QSL_SENT") {
            qso.lotw_qsl_sent.clone_from(q);
        }
        if let Some(q) = fields.get("LOTW_QSL_RCVD") {
            qso.lotw_qsl_rcvd.clone_from(q);
        }
        if let Some(qd) = fields.get("LOTW_QSLRDATE") {
            qso.lotw_qslrdate = Some(qd.clone());
        }
        if let Some(q) = fields.get("EQSL_QSL_SENT") {
            qso.eqsl_qsl_sent.clone_from(q);
        }
        if let Some(q) = fields.get("EQSL_QSL_RCVD") {
            qso.eqsl_qsl_rcvd.clone_from(q);
        }
        if let Some(qd) = fields.get("EQSL_QSLRDATE") {
            qso.eqsl_qslrdate = Some(qd.clone());
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
        if let Some(srx) = fields.get("SRX") {
            qso.srx = srx.parse().ok();
        }
        if let Some(stx) = fields.get("STX") {
            qso.stx = stx.parse().ok();
        }
        if let Some(srx_s) = fields.get("SRX_STRING") {
            qso.srx_string = Some(srx_s.clone());
        }
        if let Some(stx_s) = fields.get("STX_STRING") {
            qso.stx_string = Some(stx_s.clone());
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
        if let Some(qm) = fields
            .get("QSL_VIA_MANAGER")
            .or_else(|| fields.get("QSL_MANAGER"))
        {
            qso.qsl_manager = Some(qm.clone());
        }

        Some(qso)
    }

    /// Eksportuje listę łączności do formatu ADIF 3.1.7
    pub fn export_to_writer<W: Write>(qsos: &[QsoRecord], mut writer: W) -> std::io::Result<()> {
        writeln!(writer, "SPLogbook ADIF {ADIF_VERSION} Export")?;
        writeln!(writer, "Author: Mariusz Wozniak (SP6INA)")?;
        writeln!(writer, "<ADIF_VER:{}>{}", ADIF_VERSION.len(), ADIF_VERSION)?;
        writeln!(writer, "<PROGRAMID:9>SPLogbook")?;
        writeln!(
            writer,
            "<PROGRAMVERSION:{}>{}",
            env!("CARGO_PKG_VERSION").len(),
            env!("CARGO_PKG_VERSION")
        )?;
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
            f.push(("TIME_OFF", v.replace(':', "")));
        }
        if let Some(v) = q.freq {
            f.push(("FREQ", format!("{v:.6}")));
        }
        if let Some(v) = q.freq_rx {
            f.push(("FREQ_RX", format!("{v:.6}")));
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
        if let Some(v) = q.srx {
            f.push(("SRX", v.to_string()));
        }
        if let Some(v) = q.stx {
            f.push(("STX", v.to_string()));
        }
        if let Some(ref v) = q.srx_string {
            f.push(("SRX_STRING", v.clone()));
        }
        if let Some(ref v) = q.stx_string {
            f.push(("STX_STRING", v.clone()));
        }
        if let Some(ref v) = q.my_gridsquare {
            f.push(("MY_GRIDSQUARE", v.clone()));
        }
        if let Some(ref v) = q.my_state {
            f.push(("MY_STATE", v.clone()));
        }
        f.push(("QSL_SENT", q.qsl_sent.clone()));
        f.push(("QSL_RCVD", q.qsl_rcvd.clone()));
        if let Some(ref v) = q.qsl_sent_date {
            f.push(("QSLSDATE", v.replace(['-', '.', '/'], "")));
        }
        if let Some(ref v) = q.qsl_rcvd_date {
            f.push(("QSLRDATE", v.replace(['-', '.', '/'], "")));
        }
        f.push(("LOTW_QSL_SENT", q.lotw_qsl_sent.clone()));
        f.push(("LOTW_QSL_RCVD", q.lotw_qsl_rcvd.clone()));
        if let Some(ref v) = q.lotw_qslrdate {
            f.push(("LOTW_QSLRDATE", v.replace(['-', '.', '/'], "")));
        }
        f.push(("EQSL_QSL_SENT", q.eqsl_qsl_sent.clone()));
        f.push(("EQSL_QSL_RCVD", q.eqsl_qsl_rcvd.clone()));
        if let Some(ref v) = q.eqsl_qslrdate {
            f.push(("EQSL_QSLRDATE", v.replace(['-', '.', '/'], "")));
        }
        f
    }

    fn write_field<W: Write>(writer: &mut W, tag: &str, val: &str) -> std::io::Result<()> {
        if !val.is_empty() {
            write!(writer, "<{}:{}>{}", tag, val.len(), val)?;
        }
        Ok(())
    }

    fn write_xml_field<W: Write>(writer: &mut W, tag: &str, val: &str) -> std::io::Result<()> {
        if !val.is_empty() {
            writeln!(writer, "      <{}>{}</{}>", tag, xml_escape(val), tag)?;
        }
        Ok(())
    }

    /// Eksportuje rekordy do formatu ADX (XML ADIF 3.1.7).
    pub fn export_adx_to_writer<W: Write>(
        qsos: &[QsoRecord],
        mut writer: W,
    ) -> std::io::Result<()> {
        writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        writeln!(writer, "<ADX>")?;
        writeln!(writer, "  <HEADER>")?;
        writeln!(writer, "    <ADIF_VER>{ADIF_VERSION}</ADIF_VER>")?;
        writeln!(writer, "    <PROGRAMID>SPLogbook</PROGRAMID>")?;
        writeln!(
            writer,
            "    <PROGRAMVERSION>{}</PROGRAMVERSION>",
            env!("CARGO_PKG_VERSION")
        )?;
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

fn find_tag_end_ci(bytes: &[u8], start: usize, tag: &[u8]) -> Option<usize> {
    bytes[start..]
        .windows(tag.len())
        .position(|w| w.eq_ignore_ascii_case(tag))
        .map(|pos| start + pos + tag.len())
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
        qso.srx = Some(42);
        qso.stx = Some(7);
        qso.srx_string = Some("15".to_string());
        qso.stx_string = Some("SP".to_string());

        let mut buffer = Vec::new();
        AdifEngine::export_to_writer(&[qso.clone()], &mut buffer).unwrap();

        let parsed = AdifEngine::parse_reader(buffer.as_slice());
        assert_eq!(parsed.len(), 1);
        let p = &parsed[0];
        assert_eq!(p.callsign, "SP6INA/P");
        assert_eq!(p.name.as_deref(), Some("Stanisław"));
        assert_eq!(p.qth.as_deref(), Some("Kraków"));
        assert_eq!(
            p.comment.as_deref(),
            Some("Łączność terenowa z żółtym namiotem")
        );
        assert_eq!(p.state.as_deref(), Some("CA"));
        assert_eq!(p.iota.as_deref(), Some("EU-132"));
        assert_eq!(p.sota_ref.as_deref(), Some("SP/BZ-001"));
        assert_eq!(p.pota_ref.as_deref(), Some("PL-0042"));
        assert_eq!(p.sat_name.as_deref(), Some("AO-91"));
        assert_eq!(p.sat_mode.as_deref(), Some("V/U"));
        assert_eq!(p.prop_mode.as_deref(), Some("SAT"));
        assert_eq!(p.qsl_via.as_deref(), Some("DIRECT"));
        assert_eq!(p.qsl_manager.as_deref(), Some("SP6IXU"));
        assert_eq!(p.srx, Some(42));
        assert_eq!(p.stx, Some(7));
        assert_eq!(p.srx_string.as_deref(), Some("15"));
        assert_eq!(p.stx_string.as_deref(), Some("SP"));
    }

    #[test]
    fn test_adif_3_1_7_mode_submode_normalization() {
        assert_eq!(
            AdifEngine::normalize_mode_submode("FT2", None),
            ("MFSK", Some("FT2"))
        );
        assert_eq!(
            AdifEngine::normalize_mode_submode("FREEDATA", None),
            ("DYNAMIC", Some("FREEDATA"))
        );
        assert_eq!(
            AdifEngine::normalize_mode_submode("RIBBIT_SMS", None),
            ("OFDM", Some("RIBBIT_SMS"))
        );
        assert_eq!(
            AdifEngine::normalize_mode_submode("RIBBIT_PIX", None),
            ("OFDM", Some("RIBBIT_PIX"))
        );
        assert_eq!(
            AdifEngine::normalize_mode_submode("SCAMP_FAST", None),
            ("FSK", Some("SCAMP_FAST"))
        );
        assert_eq!(
            AdifEngine::normalize_mode_submode("SCAMP_OO", None),
            ("MTONE", Some("SCAMP_OO"))
        );
        assert_eq!(
            AdifEngine::normalize_mode_submode("USB", None),
            ("SSB", Some("USB"))
        );
        assert_eq!(AdifEngine::normalize_mode_submode("CW", None), ("CW", None));
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
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("brak wymaganego pola CALL"))
        );
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("Nieprawidłowa długość pola"))
        );
    }

    #[test]
    fn test_adif_import_trailing_record_without_eor() {
        let content = "<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW";
        let report = parse_adif_with_report(content);
        assert_eq!(report.imported, 1);
        assert_eq!(report.qsos[0].callsign, "SP6INA");
    }

    #[test]
    fn test_adif_rejects_field_longer_than_buffer() {
        // CALL deklaruje 10 bajtów, ale po znaczniku jest tylko 6 ("SP6INA").
        let content = "<CALL:10>SP6INA";
        let report = parse_adif_with_report(content);
        assert_eq!(report.imported, 0);
        assert_eq!(report.rejected, 1);
        assert!(report.errors.iter().any(|e| e.contains("deklaruje")));
    }

    #[test]
    fn test_adif_export_uses_byte_length_for_unicode() {
        // Specyfikacja ADIF liczy długość pola w bajtach (UTF-8), nie w znakach.
        // Test dokumentuje, że eksporter używa `str::len()` (bajty), dzięki czemu
        // wielobajtowe znaki nie są mylnie zliczane jako jeden bajt.
        let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
        qso.comment = Some("żółć łąka".to_string());

        let mut buffer = Vec::new();
        AdifEngine::export_to_writer(&[qso], &mut buffer).unwrap();
        let adif = String::from_utf8_lossy(&buffer);

        let expected_bytes = "żółć łąka".len();
        assert!(adif.contains(&format!("<COMMENT:{expected_bytes}>żółć łąka")));

        // Round-trip zachowuje wartość i długość po stronie bajtowej.
        let parsed = AdifEngine::parse_reader(buffer.as_slice());
        assert_eq!(parsed[0].comment.as_deref(), Some("żółć łąka"));
    }

    #[test]
    fn test_adif_preserves_intentional_whitespace_in_free_text() {
        // COMMENT jest polem tekstowym — spacje brzegowe mogą być celowe.
        let content = "<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<COMMENT:10>  z lewej <EOR>";
        let report = parse_adif_with_report(content);
        assert_eq!(report.imported, 1);
        assert_eq!(report.qsos[0].comment.as_deref(), Some("  z lewej "));
    }

    #[test]
    fn test_adx_export_well_formed_and_escaped() {
        let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
        qso.name = Some("A&B <test>".to_string());
        qso.comment = Some("cudzysłów \" i apostrof '".to_string());

        let adx = export_adx(&[qso]);
        assert!(adx.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(adx.contains("<ADX>"));
        assert!(adx.contains("<ADIF_VER>3.1.7</ADIF_VER>"));
        assert!(adx.contains("<RECORD>"));
        assert!(adx.contains("<CALL>SP6INA</CALL>"));
        assert!(adx.contains("<NAME>A&amp;B &lt;test&gt;</NAME>"));
        assert!(adx.contains("cudzysłów &quot; i apostrof &apos;"));
        assert!(adx.trim_end().ends_with("</ADX>"));
    }
}
