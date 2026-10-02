// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use crate::core::xml::escape_xml as xml_escape;
use std::collections::HashMap;
use std::io::{BufRead, Write};

/// Wspierana wersja specyfikacji ADIF.
pub const ADIF_VERSION: &str = "3.1.8";

/// Parser i generator formatu ADIF.
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

        // Pola są gromadzone do znacznika <EOR>, który zamyka rekord.
        let mut current_fields: HashMap<String, String> = HashMap::new();
        let bytes = body.as_bytes();
        let mut idx = 0;

        while idx < bytes.len() {
            if bytes[idx] == b'<' {
                idx += 1;
                let start_tag = idx;
                while idx < bytes.len() && bytes[idx] != b'>' {
                    idx += 1;
                }
                if idx >= bytes.len() {
                    break;
                }
                let tag_raw = &bytes[start_tag..idx];
                idx += 1;

                let tag_str = String::from_utf8_lossy(tag_raw);
                let tag_upper = tag_str.trim().to_uppercase();
                if tag_upper == "EOR" {
                    record_index += 1;
                    if record_has_error {
                        // Rekord z błędem składni jest odrzucany; komunikat już zapisano.
                        rejected += 1;
                    } else {
                        match Self::fields_to_qso(&mut current_fields) {
                            Ok(qso) => {
                                qsos.push(qso);
                                if let Some(msg) =
                                    Self::missing_mode_error(&current_fields, record_index)
                                {
                                    errors.push(msg);
                                }
                            }
                            Err(reason) => {
                                rejected += 1;
                                errors.push(format!("Rekord {record_index} odrzucony: {reason}"));
                            }
                        }
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
            } else {
                match Self::fields_to_qso(&mut current_fields) {
                    Ok(qso) => {
                        qsos.push(qso);
                        if let Some(msg) = Self::missing_mode_error(&current_fields, record_index) {
                            errors.push(msg);
                        }
                    }
                    Err(reason) => {
                        rejected += 1;
                        errors.push(format!("Rekord {record_index} odrzucony: {reason}"));
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

    /// Mapuje nazwę emisji na kanoniczną parę `(MODE, Option<SUBMODE>)`.
    ///
    /// Normalizuje rozpoznane emisje cyfrowe oraz wartości dozwolone
    /// wyłącznie podczas importu. Nieznane pary MODE/SUBMODE pozostawia
    /// bez zmian, aby nie tracić danych z zewnętrznych logów.
    pub fn normalize_mode_submode(
        mode: &str,
        submode: Option<&str>,
    ) -> Option<(&'static str, Option<&'static str>)> {
        let mode_upper = mode.trim().to_uppercase();
        let sub = submode.map(|s| s.trim().to_uppercase());

        // MODE = MFSK: o emisji decyduje SUBMODE.
        if mode_upper == "MFSK" {
            if let Some(s) = sub.as_deref() {
                match s {
                    "FT4" => return Some(("FT4", None)),
                    "Q65" => return Some(("Q65", None)),
                    "FT8" => return Some(("FT8", None)),
                    "WSPR" => return Some(("WSPR", None)),
                    "MSK144" => return Some(("MSK144", None)),
                    "JT65" => return Some(("JT65", None)),
                    "JT9" => return Some(("JT9", None)),
                    _ => {
                        if let Some(v) = Self::jt65_submode(s) {
                            return Some(("JT65", Some(v)));
                        }
                        if let Some(v) = Self::jt9_submode(s) {
                            return Some(("JT9", Some(v)));
                        }
                    }
                }
            }
            // MFSK bez rozpoznanego SUBMODE — zachowaj bez zmian.
            return None;
        }

        // Emisje cyfrowe traktowane jako MODE.
        match mode_upper.as_str() {
            "FT8" => return Some(("FT8", None)),
            "FT4" => return Some(("FT4", None)),
            "Q65" => return Some(("Q65", None)),
            "WSPR" => return Some(("WSPR", None)),
            "MSK144" => return Some(("MSK144", None)),
            "JT65" => return Some(("JT65", sub.as_deref().and_then(Self::jt65_submode))),
            "JT9" => return Some(("JT9", sub.as_deref().and_then(Self::jt9_submode))),
            _ => {}
        }

        // Wartości import-only jako MODE, np. MODE=JT65A → JT65/JT65A.
        if let Some(v) = Self::jt65_submode(mode_upper.as_str()) {
            return Some(("JT65", Some(v)));
        }
        if let Some(v) = Self::jt9_submode(mode_upper.as_str()) {
            return Some(("JT9", Some(v)));
        }

        // Import-only MODE=PCW → CW/PCW.
        if mode_upper == "PCW" {
            return Some(("CW", Some("PCW")));
        }

        // Pozostałe emisje mapowane wyłącznie, gdy podano SUBMODE.
        // Sam MODE spoza powyższej listy nie jest normalizowany.
        if let Some(s) = sub.as_deref() {
            match s {
                // ADIF 3.1.7: submode'y MFSK.
                "FT2" => return Some(("MFSK", Some("FT2"))),
                "JS8" => return Some(("MFSK", Some("JS8"))),
                "FST4" => return Some(("MFSK", Some("FST4"))),
                "FST4W" => return Some(("MFSK", Some("FST4W"))),
                "MFSK4" => return Some(("MFSK", Some("MFSK4"))),
                "MFSK8" => return Some(("MFSK", Some("MFSK8"))),
                "MFSK16" => return Some(("MFSK", Some("MFSK16"))),
                "MFSK32" => return Some(("MFSK", Some("MFSK32"))),
                "FSQCALL" => return Some(("MFSK", Some("FSQCALL"))),
                // DYNAMIC submodes
                "FREEDATA" => return Some(("DYNAMIC", Some("FREEDATA"))),
                "VARA HF" | "VARA_HF" => return Some(("DYNAMIC", Some("VARA HF"))),
                "VARA FM" | "VARA_FM" => return Some(("DYNAMIC", Some("VARA FM"))),
                "VARA SATELLITE" => return Some(("DYNAMIC", Some("VARA SATELLITE"))),
                // OFDM i submode'y RIBBIT
                "RIBBIT_PIX" => return Some(("OFDM", Some("RIBBIT_PIX"))),
                "RIBBIT_SMS" => return Some(("OFDM", Some("RIBBIT_SMS"))),
                // SCAMP submodes dla FSK oraz MTONE
                "SCAMP_FAST" => return Some(("FSK", Some("SCAMP_FAST"))),
                "SCAMP_SLOW" => return Some(("FSK", Some("SCAMP_SLOW"))),
                "SCAMP_VSLOW" => return Some(("FSK", Some("SCAMP_VSLOW"))),
                "SCAMP_OO" => return Some(("MTONE", Some("SCAMP_OO"))),
                "SCAMP_OO_SLW" => return Some(("MTONE", Some("SCAMP_OO_SLW"))),
                // SSB submodes
                "USB" => return Some(("SSB", Some("USB"))),
                "LSB" => return Some(("SSB", Some("LSB"))),
                // PSK submodes
                "PSK31" | "BPSK31" => return Some(("PSK", Some("PSK31"))),
                "PSK63" | "BPSK63" => return Some(("PSK", Some("PSK63"))),
                "PSK125" | "BPSK125" => return Some(("PSK", Some("PSK125"))),
                "QPSK31" => return Some(("PSK", Some("QPSK31"))),
                "QPSK63" => return Some(("PSK", Some("QPSK63"))),
                "QPSK125" => return Some(("PSK", Some("QPSK125"))),
                // DIGITALVOICE submodes
                "DMR" => return Some(("DIGITALVOICE", Some("DMR"))),
                "C4FM" => return Some(("DIGITALVOICE", Some("C4FM"))),
                "DSTAR" | "D-STAR" => return Some(("DIGITALVOICE", Some("DSTAR"))),
                "FREEDV" => return Some(("DIGITALVOICE", Some("FREEDV"))),
                "M17" => return Some(("DIGITALVOICE", Some("M17"))),
                _ => {}
            }
        }

        None
    }

    /// Zwraca kanoniczną nazwę submode'u JT65 lub `None`.
    fn jt65_submode(value: &str) -> Option<&'static str> {
        match value {
            "JT65A" => Some("JT65A"),
            "JT65B" => Some("JT65B"),
            "JT65B2" => Some("JT65B2"),
            "JT65C" => Some("JT65C"),
            "JT65C2" => Some("JT65C2"),
            _ => None,
        }
    }

    /// Zwraca kanoniczną nazwę submode'u JT9 lub `None`.
    fn jt9_submode(value: &str) -> Option<&'static str> {
        match value {
            "JT9-1" => Some("JT9-1"),
            "JT9-2" => Some("JT9-2"),
            "JT9-5" => Some("JT9-5"),
            "JT9-10" => Some("JT9-10"),
            "JT9-30" => Some("JT9-30"),
            "JT9A" => Some("JT9A"),
            "JT9B" => Some("JT9B"),
            "JT9C" => Some("JT9C"),
            "JT9D" => Some("JT9D"),
            "JT9E" => Some("JT9E"),
            "JT9F" => Some("JT9F"),
            "JT9G" => Some("JT9G"),
            "JT9H" => Some("JT9H"),
            "JT9E FAST" => Some("JT9E FAST"),
            "JT9F FAST" => Some("JT9F FAST"),
            "JT9G FAST" => Some("JT9G FAST"),
            "JT9H FAST" => Some("JT9H FAST"),
            _ => None,
        }
    }

    /// Zwraca komunikat błędu, gdy rekord nie zawiera ani MODE, ani APP_LoTW_MODE.
    fn missing_mode_error(fields: &HashMap<String, String>, record_index: usize) -> Option<String> {
        let has_mode = fields.get("MODE").is_some_and(|m| !m.trim().is_empty());
        let has_lotw = fields
            .get("APP_LOTW_MODE")
            .is_some_and(|m| !m.trim().is_empty());
        if !has_mode && !has_lotw {
            Some(format!(
                "Rekord {record_index}: brak pola MODE i APP_LoTW_MODE — emisja pozostaje pusta."
            ))
        } else {
            None
        }
    }

    /// Konwertuje mapę pól ADIF na rekord QsoRecord.
    ///
    /// Zwraca `Err` z komunikatem odrzucenia, gdy brakuje wymaganego pola CALL
    /// lub gdy nie da się ustalić pasma (brak BAND i rozpoznawalnej FREQ).
    fn fields_to_qso(fields: &mut HashMap<String, String>) -> Result<QsoRecord, String> {
        let call = match fields.remove("CALL") {
            Some(c) if !c.is_empty() => c,
            _ => return Err("brak wymaganego pola CALL.".to_string()),
        };

        let parsed_freq: Option<f64> = fields.get("FREQ").and_then(|f| f.parse().ok());
        let band = fields
            .remove("BAND")
            .filter(|b| !b.is_empty())
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
            .ok_or_else(|| "brak pola BAND ani rozpoznawalnej FREQ.".to_string())?;
        // MODE → APP_LoTW_MODE → puste (rekord bez emisji).
        let mode = fields
            .get("MODE")
            .filter(|m| !m.trim().is_empty())
            .cloned()
            .or_else(|| {
                fields
                    .get("APP_LOTW_MODE")
                    .filter(|m| !m.trim().is_empty())
                    .cloned()
            })
            .unwrap_or_default();

        let mut qso = QsoRecord::new(call, band, mode);

        if let Some(sub) = fields.remove("SUBMODE").filter(|s| !s.is_empty()) {
            if let Some((norm_mode, norm_sub)) = Self::normalize_mode_submode(&qso.mode, Some(&sub))
            {
                qso.mode = norm_mode.to_string();
                qso.submode = norm_sub.map(str::to_string);
            } else {
                qso.submode = Some(sub);
            }
        } else if let Some((norm_mode, norm_sub)) = Self::normalize_mode_submode(&qso.mode, None) {
            qso.mode = norm_mode.to_string();
            qso.submode = norm_sub.map(str::to_string);
        }
        if let Some(d) = fields.remove("QSO_DATE") {
            qso.qso_date = d;
        }
        if let Some(t) = fields.remove("TIME_ON") {
            qso.time_on = t;
        }
        qso.time_off = fields.remove("TIME_OFF");
        if parsed_freq.is_some() {
            qso.freq = parsed_freq;
        }
        if let Some(f) = fields.get("FREQ_RX") {
            qso.freq_rx = f.parse().ok();
        }
        if let Some(rst) = fields.remove("RST_SENT") {
            qso.rst_sent = rst;
        }
        if let Some(rst) = fields.remove("RST_RCVD") {
            qso.rst_rcvd = rst;
        }
        qso.name = fields.remove("NAME");
        qso.qth = fields.remove("QTH");
        qso.gridsquare = fields.remove("GRIDSQUARE");
        qso.state = fields.remove("STATE");
        qso.iota = fields.remove("IOTA");
        qso.sota_ref = fields.remove("SOTA_REF");
        qso.pota_ref = fields.remove("POTA_REF");
        qso.my_pota_ref = fields.remove("MY_POTA_REF");
        qso.my_sota_ref = fields.remove("MY_SOTA_REF");
        qso.vucc_grids = fields.remove("VUCC_GRIDS");
        qso.pga_ref = fields.remove("PGA_REF").or_else(|| fields.remove("PGA"));
        if let Some(dxcc) = fields.get("DXCC") {
            qso.dxcc = dxcc.parse().ok();
        }
        qso.country = fields.remove("COUNTRY");
        qso.continent = fields.remove("CONT");
        if let Some(cq) = fields.get("CQZ") {
            qso.cqz = cq.parse().ok();
        }
        if let Some(itu) = fields.get("ITUZ") {
            qso.ituz = itu.parse().ok();
        }
        qso.comment = fields.remove("COMMENT");
        if let Some(q) = fields.remove("QSL_SENT") {
            qso.qsl_sent = q;
        }
        if let Some(q) = fields.remove("QSL_RCVD") {
            qso.qsl_rcvd = q;
        }
        qso.qsl_sent_date = fields.remove("QSLSDATE");
        qso.qsl_rcvd_date = fields.remove("QSLRDATE");
        if let Some(q) = fields.remove("LOTW_QSL_SENT") {
            qso.lotw_qsl_sent = q;
        }
        if let Some(q) = fields.remove("LOTW_QSL_RCVD") {
            qso.lotw_qsl_rcvd = q;
        }
        qso.lotw_qslrdate = fields.remove("LOTW_QSLRDATE");
        if let Some(q) = fields.remove("EQSL_QSL_SENT") {
            qso.eqsl_qsl_sent = q;
        }
        if let Some(q) = fields.remove("EQSL_QSL_RCVD") {
            qso.eqsl_qsl_rcvd = q;
        }
        qso.eqsl_qslrdate = fields.remove("EQSL_QSLRDATE");
        qso.sat_name = fields.remove("SAT_NAME");
        qso.sat_mode = fields.remove("SAT_MODE");
        qso.prop_mode = fields.remove("PROP_MODE");
        if let Some(srx) = fields.get("SRX") {
            qso.srx = srx.parse().ok();
        }
        if let Some(stx) = fields.get("STX") {
            qso.stx = stx.parse().ok();
        }
        qso.srx_string = fields.remove("SRX_STRING");
        qso.stx_string = fields.remove("STX_STRING");
        qso.my_gridsquare = fields.remove("MY_GRIDSQUARE");
        qso.my_state = fields.remove("MY_STATE");
        qso.qsl_via = fields.remove("QSL_VIA");
        qso.qsl_manager = fields
            .remove("QSL_VIA_MANAGER")
            .or_else(|| fields.remove("QSL_MANAGER"));

        Ok(qso)
    }

    /// Eksportuje listę łączności do formatu ADIF.
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
        // FT4/Q65 eksportowane jako MODE=MFSK + SUBMODE (ADIF 3.1.7);
        // JT65/JT9 z wariantem oraz pozostałe emisje przechodzą bez zmian.
        let (export_mode, export_submode) = match (q.mode.as_str(), q.submode.as_deref()) {
            ("FT4", _) => ("MFSK", Some("FT4")),
            ("Q65", _) => ("MFSK", Some("Q65")),
            _ => (q.mode.as_str(), q.submode.as_deref()),
        };
        f.push(("MODE", export_mode.to_string()));
        if let Some(v) = export_submode {
            f.push(("SUBMODE", v.to_string()));
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

    /// Eksportuje rekordy do formatu ADX, czyli XML-owej odmiany ADIF.
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
    fn test_adif_mode_submode_normalization() {
        let norm = |m: &str, s: Option<&str>| AdifEngine::normalize_mode_submode(m, s);

        // Standardowe pary (w tym MFSK + SUBMODE dla FT4/Q65).
        assert_eq!(norm("FT8", None), Some(("FT8", None)));
        assert_eq!(norm("FT4", None), Some(("FT4", None)));
        assert_eq!(norm("MFSK", Some("FT4")), Some(("FT4", None)));
        assert_eq!(norm("Q65", None), Some(("Q65", None)));
        assert_eq!(norm("MFSK", Some("Q65")), Some(("Q65", None)));
        assert_eq!(norm("WSPR", None), Some(("WSPR", None)));
        assert_eq!(norm("MSK144", None), Some(("MSK144", None)));
        assert_eq!(norm("JT65", None), Some(("JT65", None)));
        assert_eq!(norm("JT65", Some("JT65B")), Some(("JT65", Some("JT65B"))));
        assert_eq!(norm("JT9", None), Some(("JT9", None)));
        assert_eq!(
            norm("JT9", Some("JT9E FAST")),
            Some(("JT9", Some("JT9E FAST")))
        );

        // Postacie niestandardowe sprowadzane do kanonicznej.
        assert_eq!(norm("MFSK", Some("FT8")), Some(("FT8", None)));
        assert_eq!(norm("MFSK", Some("WSPR")), Some(("WSPR", None)));
        assert_eq!(norm("MFSK", Some("MSK144")), Some(("MSK144", None)));
        // Rozpoznany MODE wygrywa nad nieznanym SUBMODE.
        assert_eq!(norm("FT8", Some("FT4")), Some(("FT8", None)));

        // Wartości import-only jako MODE.
        assert_eq!(norm("JT65A", None), Some(("JT65", Some("JT65A"))));
        assert_eq!(norm("JT9E FAST", None), Some(("JT9", Some("JT9E FAST"))));

        // Nieznane wartości zachowane bez zmian (None).
        assert_eq!(norm("NOT_A_MODE", None), None);

        // Mapowanie z SUBMODE dla pozostałych emisji nadal działa.
        assert_eq!(norm("SSB", Some("USB")), Some(("SSB", Some("USB"))));
    }

    #[test]
    fn test_adif_export_import_mode_submode_roundtrip() {
        let cases: &[(&str, Option<&str>)] = &[
            ("FT8", None),
            ("FT4", None),
            ("Q65", None),
            ("JT65", None),
            ("JT65", Some("JT65B")),
            ("JT9", None),
            ("JT9", Some("JT9E FAST")),
            ("WSPR", None),
            ("MSK144", None),
        ];

        for &(mode, submode) in cases {
            let mut qso = QsoRecord::new("SP6INA", "20m", mode);
            qso.submode = submode.map(str::to_string);

            let mut buffer = Vec::new();
            AdifEngine::export_to_writer(&[qso], &mut buffer).unwrap();

            let parsed = AdifEngine::parse_reader(buffer.as_slice());
            assert_eq!(parsed.len(), 1, "mode={mode} submode={submode:?}");
            assert_eq!(parsed[0].mode, mode, "mode={mode} submode={submode:?}");
            assert_eq!(
                parsed[0].submode.as_deref(),
                submode,
                "mode={mode} submode={submode:?}"
            );
        }
    }

    #[test]
    fn test_adif_import_uses_app_lotw_mode_fallback() {
        let content = "<CALL:6>SP6INA<BAND:3>20m<APP_LOTW_MODE:3>FT4<EOR>";
        let report = parse_adif_with_report(content);
        assert_eq!(report.imported, 1);
        assert_eq!(report.qsos[0].mode, "FT4");
        assert_eq!(report.qsos[0].submode, None);
    }

    #[test]
    fn test_adif_import_without_mode_is_empty_and_reported() {
        let content = "<CALL:6>SP6INA<BAND:3>20m<EOR>";
        let report = parse_adif_with_report(content);
        assert_eq!(report.imported, 1);
        assert_eq!(report.qsos[0].mode, "");
        assert!(report.errors.iter().any(|e| e.contains("MODE")));
    }

    #[test]
    fn test_adif_import_cw_and_pcw_modes() {
        // MODE=CW → mode="CW", submode=None.
        let report = parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<EOR>");
        assert_eq!(report.qsos[0].mode, "CW");
        assert_eq!(report.qsos[0].submode, None);

        // MODE=CW + SUBMODE=PCW → mode="CW", submode=Some("PCW").
        let report =
            parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<SUBMODE:3>PCW<EOR>");
        assert_eq!(report.qsos[0].mode, "CW");
        assert_eq!(report.qsos[0].submode.as_deref(), Some("PCW"));

        // Tolerancyjny import MODE=PCW → mode="CW", submode=Some("PCW").
        let report = parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<MODE:3>PCW<EOR>");
        assert_eq!(report.qsos[0].mode, "CW");
        assert_eq!(report.qsos[0].submode.as_deref(), Some("PCW"));
    }

    #[test]
    fn test_adif_export_cw_and_pcw() {
        // Zwykły CW → MODE=CW, bez SUBMODE.
        let qso = QsoRecord::new("SP6INA", "20m", "CW");
        let adif = export_adif(&[qso], "", "");
        assert!(adif.contains("<MODE:2>CW"), "{adif}");
        assert!(!adif.contains("<SUBMODE:"), "{adif}");

        // PCW → MODE=CW, SUBMODE=PCW.
        let mut qso = QsoRecord::new("SP6INA", "20m", "CW");
        qso.submode = Some("PCW".to_string());
        let adif = export_adif(&[qso], "", "");
        assert!(adif.contains("<MODE:2>CW"), "{adif}");
        assert!(adif.contains("<SUBMODE:3>PCW"), "{adif}");
    }

    #[test]
    fn test_adif_cw_pcw_roundtrip() {
        // Round-trip CW i PCW.
        let cases: &[(&str, Option<&str>)] = &[("CW", None), ("CW", Some("PCW"))];
        for &(mode, submode) in cases {
            let mut qso = QsoRecord::new("SP6INA", "20m", mode);
            qso.submode = submode.map(str::to_string);
            let adif = export_adif(&[qso], "", "");
            let report = parse_adif_with_report(&adif);
            assert_eq!(report.qsos[0].mode, "CW", "mode={mode} submode={submode:?}");
            assert_eq!(
                report.qsos[0].submode.as_deref(),
                submode,
                "mode={mode} submode={submode:?}"
            );
        }
    }

    #[test]
    fn test_adif_mfsk_ft8_import_normalizes_and_reexports_as_ft8() {
        // MODE=MFSK + SUBMODE=FT8 → FT8/None; re-eksport → MODE=FT8 bez SUBMODE.
        let report =
            parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<MODE:4>MFSK<SUBMODE:3>FT8<EOR>");
        assert_eq!(report.qsos[0].mode, "FT8");
        assert_eq!(report.qsos[0].submode, None);

        let adif = export_adif(&report.qsos, "", "");
        assert!(adif.contains("<MODE:3>FT8"), "{adif}");
        assert!(!adif.contains("<SUBMODE:"), "{adif}");
    }

    #[test]
    fn test_adif_app_lotw_mode_fallback_rules() {
        // APP_LOTW_MODE użyty wyłącznie przy braku MODE.
        let report = parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<APP_LOTW_MODE:3>FT4<EOR>");
        assert_eq!(report.qsos[0].mode, "FT4");

        // MODE ma pierwszeństwo nad APP_LOTW_MODE.
        let report =
            parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<APP_LOTW_MODE:3>FT4<EOR>");
        assert_eq!(report.qsos[0].mode, "CW");

        // Pusta wartość APP_LOTW_MODE nie jest akceptowana → pusta emisja + błąd.
        let report = parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<APP_LOTW_MODE:0><EOR>");
        assert_eq!(report.qsos[0].mode, "");
        assert!(report.errors.iter().any(|e| e.contains("MODE")));
    }

    #[test]
    fn test_adif_import_band_fallback_is_rejected() {
        // Rekord z BAND jest importowany.
        let report = parse_adif_with_report("<CALL:6>SP6INA<BAND:3>20m<MODE:2>CW<EOR>");
        assert_eq!(report.imported, 1);
        assert_eq!(report.rejected, 0);
        assert_eq!(report.qsos[0].band, "20m");

        // Rekord z poprawną FREQ (bez BAND) jest importowany, pasmo wyznaczone z FREQ.
        let report = parse_adif_with_report("<CALL:6>SP6INA<FREQ:6>14.074<MODE:2>CW<EOR>");
        assert_eq!(report.imported, 1);
        assert_eq!(report.rejected, 0);
        assert_eq!(report.qsos[0].band, "20m");

        // Rekord bez BAND i bez rozpoznawalnej FREQ jest odrzucany z komunikatem.
        let report = parse_adif_with_report("<CALL:6>SP6INA<MODE:2>CW<EOR>");
        assert_eq!(report.imported, 0);
        assert_eq!(report.rejected, 1);
        assert!(report.qsos.is_empty());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("BAND") && e.contains("FREQ"))
        );

        // FREQ spoza pasm amatorskich (brak dopasowania) też jest odrzucany.
        let report = parse_adif_with_report("<CALL:6>SP6INA<FREQ:5>1.000<MODE:2>CW<EOR>");
        assert_eq!(report.imported, 0);
        assert_eq!(report.rejected, 1);
        assert!(report.errors.iter().any(|e| e.contains("BAND")));
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
        assert!(adx.contains("<ADIF_VER>3.1.8</ADIF_VER>"));
        assert!(adx.contains("<RECORD>"));
        assert!(adx.contains("<CALL>SP6INA</CALL>"));
        assert!(adx.contains("<NAME>A&amp;B &lt;test&gt;</NAME>"));
        assert!(adx.contains("cudzysłów &quot; i apostrof &apos;"));
        assert!(adx.trim_end().ends_with("</ADX>"));
    }
}
