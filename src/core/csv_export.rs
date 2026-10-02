// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Konfigurowalny eksport dziennika łączności do formatu CSV.
//!
//! Eksporter oferuje pełny katalog pól `QsoRecord`, wybór separatora,
//! opcjonalny nagłówek oraz poprawną obsługę znaków specjalnych (cudzysłów,
//! separator i znaki nowej linii w polach).

use crate::core::qso::QsoRecord;

/// Separator pól w pliku CSV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvDelimiter {
    Comma,
    Semicolon,
    Tab,
}

impl CsvDelimiter {
    /// Zwraca bajt separatora używany przy generowaniu i wykrywaniu znaków specjalnych.
    pub fn as_byte(self) -> u8 {
        match self {
            CsvDelimiter::Comma => b',',
            CsvDelimiter::Semicolon => b';',
            CsvDelimiter::Tab => b'\t',
        }
    }

    /// Etykieta separatora do wyświetlenia w interfejsie.
    pub fn display_name(self) -> &'static str {
        match self {
            CsvDelimiter::Comma => "Przecinek (,)",
            CsvDelimiter::Semicolon => "Średnik (;)",
            CsvDelimiter::Tab => "Tabulator",
        }
    }
}

/// Kolumna możliwa do wyeksportowania do CSV (podzbiór pól `QsoRecord`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvColumn {
    Date,
    Time,
    Callsign,
    Band,
    Mode,
    Submode,
    Freq,
    FreqRx,
    RstSent,
    RstRcvd,
    Name,
    Qth,
    Grid,
    State,
    Iota,
    SotaRef,
    PotaRef,
    PgaRef,
    Dxcc,
    Country,
    Continent,
    Cqz,
    Ituz,
    Comment,
    QslVia,
    QslSent,
    QslRcvd,
    LotwQslRcvd,
    EqslQslRcvd,
    SatName,
    SatMode,
    PropMode,
    Srx,
    Stx,
    SrxString,
    StxString,
    MyGrid,
    MyState,
    MyPotaRef,
    MySotaRef,
    VuccGrids,
}

impl CsvColumn {
    /// Wszystkie kolumny w kolejności prezentacji.
    pub const ALL: &'static [CsvColumn] = &[
        CsvColumn::Date,
        CsvColumn::Time,
        CsvColumn::Callsign,
        CsvColumn::Band,
        CsvColumn::Mode,
        CsvColumn::Submode,
        CsvColumn::Freq,
        CsvColumn::FreqRx,
        CsvColumn::RstSent,
        CsvColumn::RstRcvd,
        CsvColumn::Name,
        CsvColumn::Qth,
        CsvColumn::Grid,
        CsvColumn::State,
        CsvColumn::Iota,
        CsvColumn::SotaRef,
        CsvColumn::PotaRef,
        CsvColumn::PgaRef,
        CsvColumn::Dxcc,
        CsvColumn::Country,
        CsvColumn::Continent,
        CsvColumn::Cqz,
        CsvColumn::Ituz,
        CsvColumn::Comment,
        CsvColumn::QslVia,
        CsvColumn::QslSent,
        CsvColumn::QslRcvd,
        CsvColumn::LotwQslRcvd,
        CsvColumn::EqslQslRcvd,
        CsvColumn::SatName,
        CsvColumn::SatMode,
        CsvColumn::PropMode,
        CsvColumn::Srx,
        CsvColumn::Stx,
        CsvColumn::SrxString,
        CsvColumn::StxString,
        CsvColumn::MyGrid,
        CsvColumn::MyState,
        CsvColumn::MyPotaRef,
        CsvColumn::MySotaRef,
        CsvColumn::VuccGrids,
    ];

    /// Stabilny identyfikator kolumny (używany m.in. przy zapisie ustawień).
    pub fn id(self) -> &'static str {
        match self {
            CsvColumn::Date => "date",
            CsvColumn::Time => "time",
            CsvColumn::Callsign => "callsign",
            CsvColumn::Band => "band",
            CsvColumn::Mode => "mode",
            CsvColumn::Submode => "submode",
            CsvColumn::Freq => "freq",
            CsvColumn::FreqRx => "freq_rx",
            CsvColumn::RstSent => "rst_sent",
            CsvColumn::RstRcvd => "rst_rcvd",
            CsvColumn::Name => "name",
            CsvColumn::Qth => "qth",
            CsvColumn::Grid => "gridsquare",
            CsvColumn::State => "state",
            CsvColumn::Iota => "iota",
            CsvColumn::SotaRef => "sota_ref",
            CsvColumn::PotaRef => "pota_ref",
            CsvColumn::PgaRef => "pga_ref",
            CsvColumn::Dxcc => "dxcc",
            CsvColumn::Country => "country",
            CsvColumn::Continent => "continent",
            CsvColumn::Cqz => "cqz",
            CsvColumn::Ituz => "ituz",
            CsvColumn::Comment => "comment",
            CsvColumn::QslVia => "qsl_via",
            CsvColumn::QslSent => "qsl_sent",
            CsvColumn::QslRcvd => "qsl_rcvd",
            CsvColumn::LotwQslRcvd => "lotw_qsl_rcvd",
            CsvColumn::EqslQslRcvd => "eqsl_qsl_rcvd",
            CsvColumn::SatName => "sat_name",
            CsvColumn::SatMode => "sat_mode",
            CsvColumn::PropMode => "prop_mode",
            CsvColumn::Srx => "srx",
            CsvColumn::Stx => "stx",
            CsvColumn::SrxString => "srx_string",
            CsvColumn::StxString => "stx_string",
            CsvColumn::MyGrid => "my_gridsquare",
            CsvColumn::MyState => "my_state",
            CsvColumn::MyPotaRef => "my_pota_ref",
            CsvColumn::MySotaRef => "my_sota_ref",
            CsvColumn::VuccGrids => "vucc_grids",
        }
    }

    /// Etykieta wyświetlana w interfejsie (polski skrót).
    pub fn label(self) -> &'static str {
        match self {
            CsvColumn::Date => "Data",
            CsvColumn::Time => "Czas",
            CsvColumn::Callsign => "Znak",
            CsvColumn::Band => "Pasmo",
            CsvColumn::Mode => "Emisja",
            CsvColumn::Submode => "Subemisja",
            CsvColumn::Freq => "Częstotliwość TX (MHz)",
            CsvColumn::FreqRx => "Częstotliwość RX (MHz)",
            CsvColumn::RstSent => "RST nadane",
            CsvColumn::RstRcvd => "RST odebrane",
            CsvColumn::Name => "Imię",
            CsvColumn::Qth => "QTH (miasto)",
            CsvColumn::Grid => "Lokator",
            CsvColumn::State => "Stan/prowincja",
            CsvColumn::Iota => "IOTA",
            CsvColumn::SotaRef => "SOTA Ref",
            CsvColumn::PotaRef => "POTA Ref",
            CsvColumn::PgaRef => "PGA Ref",
            CsvColumn::Dxcc => "DXCC",
            CsvColumn::Country => "Kraj",
            CsvColumn::Continent => "Kontynent",
            CsvColumn::Cqz => "Strefa CQ",
            CsvColumn::Ituz => "Strefa ITU",
            CsvColumn::Comment => "Uwagi",
            CsvColumn::QslVia => "QSL via",
            CsvColumn::QslSent => "QSL wysłana",
            CsvColumn::QslRcvd => "QSL odebrana",
            CsvColumn::LotwQslRcvd => "LoTW odebrane",
            CsvColumn::EqslQslRcvd => "eQSL odebrane",
            CsvColumn::SatName => "Satelita",
            CsvColumn::SatMode => "Tryb satelity",
            CsvColumn::PropMode => "Propagacja",
            CsvColumn::Srx => "SRX (numer)",
            CsvColumn::Stx => "STX (numer)",
            CsvColumn::SrxString => "SRX (tekst)",
            CsvColumn::StxString => "STX (tekst)",
            CsvColumn::MyGrid => "Mój lokator",
            CsvColumn::MyState => "Mój stan",
            CsvColumn::MyPotaRef => "Mój POTA Ref",
            CsvColumn::MySotaRef => "Mój SOTA Ref",
            CsvColumn::VuccGrids => "VUCC Grids",
        }
    }

    /// Nazwa pola ADIF używana jako nagłówek kolumny (kompatybilna z innymi programami).
    pub fn header(self) -> &'static str {
        match self {
            CsvColumn::Date => "QSO_DATE",
            CsvColumn::Time => "TIME_ON",
            CsvColumn::Callsign => "CALL",
            CsvColumn::Band => "BAND",
            CsvColumn::Mode => "MODE",
            CsvColumn::Submode => "SUBMODE",
            CsvColumn::Freq => "FREQ",
            CsvColumn::FreqRx => "FREQ_RX",
            CsvColumn::RstSent => "RST_SENT",
            CsvColumn::RstRcvd => "RST_RCVD",
            CsvColumn::Name => "NAME",
            CsvColumn::Qth => "QTH",
            CsvColumn::Grid => "GRIDSQUARE",
            CsvColumn::State => "STATE",
            CsvColumn::Iota => "IOTA",
            CsvColumn::SotaRef => "SOTA_REF",
            CsvColumn::PotaRef => "POTA_REF",
            CsvColumn::PgaRef => "PGA_REF",
            CsvColumn::Dxcc => "DXCC",
            CsvColumn::Country => "COUNTRY",
            CsvColumn::Continent => "CONTINENT",
            CsvColumn::Cqz => "CQZ",
            CsvColumn::Ituz => "ITUZ",
            CsvColumn::Comment => "COMMENT",
            CsvColumn::QslVia => "QSL_VIA",
            CsvColumn::QslSent => "QSL_SENT",
            CsvColumn::QslRcvd => "QSL_RCVD",
            CsvColumn::LotwQslRcvd => "LOTW_QSL_RCVD",
            CsvColumn::EqslQslRcvd => "EQSL_QSL_RCVD",
            CsvColumn::SatName => "SAT_NAME",
            CsvColumn::SatMode => "SAT_MODE",
            CsvColumn::PropMode => "PROP_MODE",
            CsvColumn::Srx => "SRX",
            CsvColumn::Stx => "STX",
            CsvColumn::SrxString => "SRX_STRING",
            CsvColumn::StxString => "STX_STRING",
            CsvColumn::MyGrid => "MY_GRIDSQUARE",
            CsvColumn::MyState => "MY_STATE",
            CsvColumn::MyPotaRef => "MY_POTA_REF",
            CsvColumn::MySotaRef => "MY_SOTA_REF",
            CsvColumn::VuccGrids => "VUCC_GRIDS",
        }
    }

    /// Wartość pola dla danego rekordu łączności.
    pub fn value(self, qso: &QsoRecord) -> String {
        match self {
            CsvColumn::Date => qso.qso_date.clone(),
            CsvColumn::Time => qso.time_on.clone(),
            CsvColumn::Callsign => qso.callsign.clone(),
            CsvColumn::Band => qso.band.clone(),
            CsvColumn::Mode => qso.mode.clone(),
            CsvColumn::Submode => qso.submode.clone().unwrap_or_default(),
            CsvColumn::Freq => qso.freq.map(fmt_mhz).unwrap_or_default(),
            CsvColumn::FreqRx => qso.freq_rx.map(fmt_mhz).unwrap_or_default(),
            CsvColumn::RstSent => qso.rst_sent.clone(),
            CsvColumn::RstRcvd => qso.rst_rcvd.clone(),
            CsvColumn::Name => qso.name.clone().unwrap_or_default(),
            CsvColumn::Qth => qso.qth.clone().unwrap_or_default(),
            CsvColumn::Grid => qso.gridsquare.clone().unwrap_or_default(),
            CsvColumn::State => qso.state.clone().unwrap_or_default(),
            CsvColumn::Iota => qso.iota.clone().unwrap_or_default(),
            CsvColumn::SotaRef => qso.sota_ref.clone().unwrap_or_default(),
            CsvColumn::PotaRef => qso.pota_ref.clone().unwrap_or_default(),
            CsvColumn::PgaRef => qso.pga_ref.clone().unwrap_or_default(),
            CsvColumn::Dxcc => qso.dxcc.map(|v| v.to_string()).unwrap_or_default(),
            CsvColumn::Country => qso.country.clone().unwrap_or_default(),
            CsvColumn::Continent => qso.continent.clone().unwrap_or_default(),
            CsvColumn::Cqz => qso.cqz.map(|v| v.to_string()).unwrap_or_default(),
            CsvColumn::Ituz => qso.ituz.map(|v| v.to_string()).unwrap_or_default(),
            CsvColumn::Comment => qso.comment.clone().unwrap_or_default(),
            CsvColumn::QslVia => qso.qsl_via.clone().unwrap_or_default(),
            CsvColumn::QslSent => qso.qsl_sent.clone(),
            CsvColumn::QslRcvd => qso.qsl_rcvd.clone(),
            CsvColumn::LotwQslRcvd => qso.lotw_qsl_rcvd.clone(),
            CsvColumn::EqslQslRcvd => qso.eqsl_qsl_rcvd.clone(),
            CsvColumn::SatName => qso.sat_name.clone().unwrap_or_default(),
            CsvColumn::SatMode => qso.sat_mode.clone().unwrap_or_default(),
            CsvColumn::PropMode => qso.prop_mode.clone().unwrap_or_default(),
            CsvColumn::Srx => qso.srx.map(|v| v.to_string()).unwrap_or_default(),
            CsvColumn::Stx => qso.stx.map(|v| v.to_string()).unwrap_or_default(),
            CsvColumn::SrxString => qso.srx_string.clone().unwrap_or_default(),
            CsvColumn::StxString => qso.stx_string.clone().unwrap_or_default(),
            CsvColumn::MyGrid => qso.my_gridsquare.clone().unwrap_or_default(),
            CsvColumn::MyState => qso.my_state.clone().unwrap_or_default(),
            CsvColumn::MyPotaRef => qso.my_pota_ref.clone().unwrap_or_default(),
            CsvColumn::MySotaRef => qso.my_sota_ref.clone().unwrap_or_default(),
            CsvColumn::VuccGrids => qso.vucc_grids.clone().unwrap_or_default(),
        }
    }
}

/// Formatuje częstotliwość w MHz z maks. 6 miejscami po przecinku,
/// usuwając zbędne zera na końcu.
fn fmt_mhz(mhz: f64) -> String {
    let mut text = format!("{mhz:.6}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// Zabezpiecza pojedyncze pole CSV: cudzysłowia są podwajane, a pola zawierające
/// separator, cudzysłów lub znak nowej linii są otaczane cudzysłowami.
pub fn escape_field(value: &str, delimiter: u8) -> String {
    let delim = delimiter as char;
    let needs_quotes = value.contains(delim)
        || value.contains('"')
        || value.contains('\n')
        || value.contains('\r');
    if !needs_quotes {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        if c == '"' {
            out.push('"');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Generuje kompletny dokument CSV dla wybranych rekordów, kolumn i separatora.
///
/// Wiersze kończone są sekwencją `\r\n` (zgodnie z RFC 4180).
pub fn export_csv(
    qsos: &[QsoRecord],
    columns: &[CsvColumn],
    delimiter: CsvDelimiter,
    include_header: bool,
) -> String {
    let delim = delimiter.as_byte() as char;
    let mut out = String::new();

    if include_header {
        let headers: Vec<&str> = columns.iter().map(|c| c.header()).collect();
        out.push_str(&headers.join(&delim.to_string()));
        out.push_str("\r\n");
    }

    for qso in qsos {
        let row: Vec<String> = columns
            .iter()
            .map(|c| escape_field(&c.value(qso), delimiter.as_byte()))
            .collect();
        out.push_str(&row.join(&delim.to_string()));
        out.push_str("\r\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_qso() -> QsoRecord {
        QsoRecord {
            callsign: "SP6XYZ".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            qso_date: "2026-01-15".to_string(),
            time_on: "12:34:56".to_string(),
            freq: Some(14.025),
            country: Some("Poland".to_string()),
            comment: Some("Test \"cytat\", średnik; i\nnowa linia".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn escape_field_leaves_plain_values_untouched() {
        assert_eq!(escape_field("SP6XYZ", b','), "SP6XYZ");
    }

    #[test]
    fn escape_field_quotes_fields_with_delimiter() {
        assert_eq!(escape_field("a,b", b','), "\"a,b\"");
    }

    #[test]
    fn escape_field_doubles_quotes() {
        assert_eq!(escape_field("a\"b", b','), "\"a\"\"b\"");
    }

    #[test]
    fn escape_field_quotes_newlines() {
        assert_eq!(escape_field("a\nb", b','), "\"a\nb\"");
    }

    #[test]
    fn escape_field_respects_tab_delimiter() {
        assert_eq!(escape_field("a\tb", b'\t'), "\"a\tb\"");
        assert_eq!(escape_field("a,b", b'\t'), "a,b");
    }

    #[test]
    fn export_csv_builds_header_and_rows() {
        let qso = QsoRecord {
            callsign: "SP6XYZ".to_string(),
            band: "20m".to_string(),
            mode: "CW".to_string(),
            comment: Some("Test \"cytat\", średnik;".to_string()),
            ..Default::default()
        };
        let cols = vec![CsvColumn::Callsign, CsvColumn::Band, CsvColumn::Comment];
        let out = export_csv(&[qso], &cols, CsvDelimiter::Comma, true);
        let expected = "CALL,BAND,COMMENT\r\nSP6XYZ,20m,\"Test \"\"cytat\"\", średnik;\"\r\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn export_csv_embeds_newline_inside_quoted_field() {
        let qso = sample_qso();
        let cols = vec![CsvColumn::Callsign, CsvColumn::Comment];
        let out = export_csv(&[qso], &cols, CsvDelimiter::Comma, true);
        let expected = "CALL,COMMENT\r\nSP6XYZ,\"Test \"\"cytat\"\", średnik; i\nnowa linia\"\r\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn export_csv_omits_header_when_disabled() {
        let qsos = vec![sample_qso()];
        let cols = vec![CsvColumn::Callsign];
        let out = export_csv(&qsos, &cols, CsvDelimiter::Semicolon, false);
        assert_eq!(out, "SP6XYZ\r\n");
    }

    #[test]
    fn value_returns_expected_fields() {
        let qso = sample_qso();
        assert_eq!(CsvColumn::Callsign.value(&qso), "SP6XYZ");
        assert_eq!(CsvColumn::Country.value(&qso), "Poland");
        assert_eq!(CsvColumn::Dxcc.value(&qso), "");
    }

    #[test]
    fn fmt_mhz_trims_trailing_zeros() {
        assert_eq!(fmt_mhz(14.025), "14.025");
        assert_eq!(fmt_mhz(7.0), "7");
        assert_eq!(fmt_mhz(14.025_000_001), "14.025");
    }
}
