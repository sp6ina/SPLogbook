// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Parser i walidacja wymiany (exchange) w zawodach krótkofalarskich.
//!
//! Zamiast wolnego tekstu, każda reguła kontestu deklaruje uporządkowaną listę
//! pól wymiany ([`ExchangeField`]). Parser zamienia wpisany przez operatora
//! ciąg znaków (np. `599 001 EU-115`) na ustrukturyzowane pola
//! ([`ParsedExchange`]), które można bezpośrednio przenieść do rekordu QSO
//! oraz zwalidować przed zapisem.

use crate::core::qso::QsoRecord;

/// Składowa wymiany w zawodach. Kolejność pól w regule odpowiada kolejności,
/// w jakiej operator je odbiera/nadaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeField {
    /// Raport odbioru: `599`, `59`, `579` (2–3 cyfry).
    Rst,
    /// Numer kolejny (serial), np. `001`.
    Serial,
    /// Strefa CQ (1–40).
    Zone,
    /// Strefa CQ (1–40) LUB skrót centrali HQ (IARU HF).
    ZoneOrHq,
    /// Stan/prowincja US/VE (2 litery) LUB moc nadajnika (ARRL DX: DX wysyła moc).
    StateOrPower,
    /// Strefa ITU (1–90).
    ItuZone,
    /// Lokator Maidenhead (4/6/8 znaków), np. `JO80`.
    Grid,
    /// Stan/prowincja (2 litery), np. `NY`, `ON`, `DX`.
    State,
    /// Moc nadajnika, np. `100W`, `1KW`, `KW`, `5`.
    Power,
    /// Wiek operatora.
    Age,
    /// Rok uzyskania pierwszej licencji (2 lub 4 cyfry).
    Year,
    /// Referencja IOTA, np. `EU-115`.
    Iota,
    /// Imię operatora.
    Name,
    /// QTH / miejscowość.
    Qth,
    /// Czas UTC `HHMM` (BARTG).
    Time,
    /// Skrót centrali HQ, np. `ARRL`, `REF`.
    Hq,
    /// Kategoria (Field Day), np. `2A`.
    Category,
    /// Okręg / prowincja / oblast / prefektura / sekcja ARRL (token ogólny).
    District,
}

/// Wynik parsowania wymiany — pola ustrukturyzowane.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedExchange {
    pub rst: Option<String>,
    pub serial: Option<u32>,
    pub zone: Option<u32>,
    pub ituz: Option<u32>,
    pub grid: Option<String>,
    pub state: Option<String>,
    pub power: Option<String>,
    pub age: Option<u32>,
    pub year: Option<u32>,
    pub iota: Option<String>,
    pub name: Option<String>,
    pub qth: Option<String>,
    pub time: Option<String>,
    pub hq: Option<String>,
    pub category: Option<String>,
    pub district: Option<String>,
    /// Oryginalny, nieprzetworzony tekst wymiany.
    pub raw: String,
}

/// Błąd parsowania wymiany z czytelnym komunikatem dla operatora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeError {
    pub message: String,
}

impl ExchangeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ExchangeError {}

/// Parsuje wymianę wg uporządkowanej listy pól reguły kontestu.
///
/// Zwraca ustrukturyzowaną wymianę lub [`ExchangeError`] z opisem problemu.
pub fn parse_exchange(fields: &[ExchangeField], text: &str) -> Result<ParsedExchange, ExchangeError> {
    let raw = text.trim().to_string();
    if fields.is_empty() {
        // Brak zdefiniowanej struktury wymiany — nic nie walidujemy.
        return Ok(ParsedExchange { raw, ..Default::default() });
    }
    if raw.is_empty() {
        return Err(ExchangeError::new("Pusta wymiana — wpisz raport i wymianę"));
    }

    let tokens = tokenize(&raw, fields);
    let mut parsed = ParsedExchange { raw, ..Default::default() };

    let mut ti = 0usize;
    for field in fields.iter().copied() {
        if ti >= tokens.len() {
            if field_is_optional(field) {
                continue;
            }
            return Err(ExchangeError::new(format!(
                "Brak elementu wymiany: {}",
                field_label(field)
            )));
        }
        let token = &tokens[ti];
        if match_token(field, token, &mut parsed) {
            ti += 1;
        } else {
            return Err(ExchangeError::new(format!(
                "Nieprawidłowa wartość „{}” dla pola: {}",
                token,
                field_label(field)
            )));
        }
    }

    if ti < tokens.len() {
        let extra: Vec<&str> = tokens[ti..].iter().map(std::string::String::as_str).collect();
        return Err(ExchangeError::new(format!(
            "Nadmiarowe elementy wymiany: {}",
            extra.join(" ")
        )));
    }

    Ok(parsed)
}

/// Zgaduje strukturę wymiany na podstawie opisu tekstowego (format niestandardowy).
///
/// Używane dla własnych kontestów, gdzie operator podaje np. `RST + Serial`.
pub fn fields_from_format_string(format: &str) -> Vec<ExchangeField> {
    let upper = format.to_uppercase();
    let mut out = Vec::new();
    for part in upper.split('+') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        out.push(field_from_token(p));
    }
    out
}

fn field_from_token(p: &str) -> ExchangeField {
    if p.contains("RST") {
        return ExchangeField::Rst;
    }
    if p.contains("SERIAL") || p.contains("NR") || p.contains("NUM") {
        return ExchangeField::Serial;
    }
    if p.contains("IOTA") || p.contains("ISLAND") {
        return ExchangeField::Iota;
    }
    if p.contains("ITU") {
        return ExchangeField::ItuZone;
    }
    if p.contains("ZONE") || p.contains("HQ") {
        if p.contains("HQ") && p.contains("ZONE") {
            return ExchangeField::ZoneOrHq;
        }
        if p.contains("HQ") {
            return ExchangeField::Hq;
        }
        return ExchangeField::Zone;
    }
    if p.contains("GRID") || p.contains("LOC") || p.contains("MAIDENHEAD") || p.contains("SQUARE") {
        return ExchangeField::Grid;
    }
    if p.contains("STATE") {
        return ExchangeField::State;
    }
    if p.contains("POWER") {
        return ExchangeField::Power;
    }
    if p.contains("AGE") {
        return ExchangeField::Age;
    }
    if p.contains("YEAR") || p.contains("LICENSE") || p.contains("LICENCE") {
        return ExchangeField::Year;
    }
    if p.contains("NAME") {
        return ExchangeField::Name;
    }
    if p.contains("QTH") || p.contains("CITY") {
        return ExchangeField::Qth;
    }
    if p.contains("TIME") {
        return ExchangeField::Time;
    }
    if p.contains("CATEGORY") || p.contains("CLASS") {
        return ExchangeField::Category;
    }
    if p.contains("PROVINCE") || p.contains("DISTRICT") || p.contains("OBLAST")
        || p.contains("PREFECTURE") || p.contains("SECTION")
    {
        return ExchangeField::District;
    }
    // Domyślnie traktujemy jako ogólny token (okręg/prowincja/…).
    ExchangeField::District
}

/// Przenosi sparsowaną wymianę do pól rekordu QSO (w tym pól mnożnikowych).
pub fn apply_to_qso(parsed: &ParsedExchange, qso: &mut QsoRecord) {
    if let Some(rst) = &parsed.rst {
        qso.rst_rcvd.clone_from(rst);
    }
    if let Some(serial) = parsed.serial {
        qso.srx = Some(serial);
        qso.srx_string = Some(serial.to_string());
    }
    if let Some(zone) = parsed.zone {
        qso.cqz = Some(zone);
    }
    if let Some(ituz) = parsed.ituz {
        qso.ituz = Some(ituz);
    }
    if let Some(grid) = &parsed.grid {
        qso.gridsquare = Some(grid.clone());
    }
    if let Some(state) = &parsed.state {
        qso.state = Some(state.clone());
    }
    if let Some(district) = &parsed.district {
        // Prowincje/oblasty/okręgi/prefektury/sekcyjne ARRL pełnią rolę mnożnika
        // w tych samych polach, w których punkty liczy się po `state`.
        if qso.state.is_none() {
            qso.state = Some(district.clone());
        }
    }
    if let Some(iota) = &parsed.iota {
        qso.iota = Some(iota.clone());
    }
    if let Some(name) = &parsed.name {
        qso.name = Some(name.clone());
    }
    if let Some(qth) = &parsed.qth {
        qso.qth = Some(qth.clone());
    }
    // Pełna, oryginalna wymiana trafia do SRX_STRING (ADIF) jako zapas.
    if !parsed.raw.is_empty() {
        qso.srx_string = Some(parsed.raw.clone());
    }
    // Dodatkowe, nie-ADIF pola (moc/wiek/rok/czas/HQ/kategoria) — do komentarza.
    let extras = extra_fields_summary(parsed);
    if !extras.is_empty() {
        qso.comment = Some(match &qso.comment {
            Some(existing) if !existing.is_empty() => format!("{existing} | {extras}"),
            _ => extras,
        });
    }
}

/// Czytelne podsumowanie sparsowanej wymiany (do podglądu w UI).
pub fn exchange_summary(parsed: &ParsedExchange) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = &parsed.rst {
        parts.push(format!("RST {v}"));
    }
    if let Some(v) = parsed.serial {
        parts.push(format!("Numer {v:03}"));
    }
    if let Some(v) = parsed.zone {
        parts.push(format!("Strefa {v}"));
    }
    if let Some(v) = parsed.ituz {
        parts.push(format!("ITU {v}"));
    }
    if let Some(v) = &parsed.grid {
        parts.push(format!("Lokator {v}"));
    }
    if let Some(v) = &parsed.state {
        parts.push(format!("Stan {v}"));
    }
    if let Some(v) = &parsed.power {
        parts.push(format!("Moc {v}"));
    }
    if let Some(v) = parsed.age {
        parts.push(format!("Wiek {v}"));
    }
    if let Some(v) = parsed.year {
        parts.push(format!("Rok {v}"));
    }
    if let Some(v) = &parsed.iota {
        parts.push(format!("IOTA {v}"));
    }
    if let Some(v) = &parsed.name {
        parts.push(format!("Imię {v}"));
    }
    if let Some(v) = &parsed.qth {
        parts.push(format!("QTH {v}"));
    }
    if let Some(v) = &parsed.time {
        parts.push(format!("Czas {v}"));
    }
    if let Some(v) = &parsed.hq {
        parts.push(format!("HQ {v}"));
    }
    if let Some(v) = &parsed.category {
        parts.push(format!("Kategoria {v}"));
    }
    if let Some(v) = &parsed.district {
        parts.push(format!("Okręg {v}"));
    }
    parts.join(" · ")
}

/// Buduje tekst nadawanej wymiany (RST + numer) dla danej reguły.
///
/// Dla większości zawodów nadawana wymiana to `RST + Serial`. W razie potrzeby
/// zwraca pola nieznane jako ich etykiety (do ręcznego uzupełnienia).
pub fn format_sent_exchange(fields: &[ExchangeField], rst: &str, serial: u32) -> String {
    let mut parts: Vec<String> = Vec::new();
    for field in fields {
        match field {
            ExchangeField::Rst => parts.push(rst.to_string()),
            ExchangeField::Serial => parts.push(format!("{serial:03}")),
            ExchangeField::Zone
            | ExchangeField::ZoneOrHq
            | ExchangeField::StateOrPower
            | ExchangeField::ItuZone
            | ExchangeField::Grid
            | ExchangeField::State
            | ExchangeField::Power
            | ExchangeField::Age
            | ExchangeField::Year
            | ExchangeField::Iota
            | ExchangeField::Name
            | ExchangeField::Qth
            | ExchangeField::Time
            | ExchangeField::Hq
            | ExchangeField::Category
            | ExchangeField::District => parts.push("?".to_string()),
        }
    }
    parts.join(" ")
}

fn extra_fields_summary(parsed: &ParsedExchange) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = &parsed.power {
        parts.push(format!("Power={v}"));
    }
    if let Some(v) = parsed.age {
        parts.push(format!("Age={v}"));
    }
    if let Some(v) = parsed.year {
        parts.push(format!("Year={v}"));
    }
    if let Some(v) = &parsed.time {
        parts.push(format!("Time={v}"));
    }
    if let Some(v) = &parsed.hq {
        parts.push(format!("HQ={v}"));
    }
    if let Some(v) = &parsed.category {
        parts.push(format!("Category={v}"));
    }
    parts.join(", ")
}

impl ExchangeField {
    /// Czytelna etykieta pola (używana w GUI do podpowiedzi formatu wymiany).
    pub fn label(self) -> &'static str {
        field_label(self)
    }
}

fn field_is_optional(field: ExchangeField) -> bool {
    matches!(field, ExchangeField::Iota)
}

fn field_label(field: ExchangeField) -> &'static str {
    match field {
        ExchangeField::Rst => "RST",
        ExchangeField::Serial => "numer (serial)",
        ExchangeField::Zone => "strefa CQ",
        ExchangeField::ZoneOrHq => "strefa CQ / skrót HQ",
        ExchangeField::StateOrPower => "stan/prowincja lub moc",
        ExchangeField::ItuZone => "strefa ITU",
        ExchangeField::Grid => "lokator (grid)",
        ExchangeField::State => "stan/prowincja",
        ExchangeField::Power => "moc",
        ExchangeField::Age => "wiek",
        ExchangeField::Year => "rok licencji",
        ExchangeField::Iota => "numer IOTA",
        ExchangeField::Name => "imię",
        ExchangeField::Qth => "QTH (miejscowość)",
        ExchangeField::Time => "czas",
        ExchangeField::Hq => "skrót HQ",
        ExchangeField::Category => "kategoria",
        ExchangeField::District => "okręg/prowincja/oblast",
    }
}

/// Dzieli tekst na tokeny, obsługując sklejoną formę `RST+numer` (np. `599001`).
fn tokenize(text: &str, fields: &[ExchangeField]) -> Vec<String> {
    if fields.first() == Some(&ExchangeField::Rst) {
        let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = compact.as_bytes();
        if compact.len() >= 5
            && compact.len() <= 6
            && bytes.iter().all(u8::is_ascii_digit)
        {
            let rst_len = if compact.starts_with("599") { 3 } else { 2 };
            let rst = &compact[..rst_len];
            let rest = &compact[rst_len..];
            if is_rst(rst) && !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()) {
                return vec![rst.to_string(), rest.to_string()];
            }
        }
    }
    text.split_whitespace().map(str::to_uppercase).collect()
}

fn match_token(field: ExchangeField, token: &str, out: &mut ParsedExchange) -> bool {
    match field {
        ExchangeField::Rst => {
            if is_rst(token) {
                out.rst = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::Serial => {
            if is_digits(token) && token.len() <= 6 {
                out.serial = token.parse::<u32>().ok();
                true
            } else {
                false
            }
        }
        ExchangeField::Zone => {
            if is_zone(token) {
                out.zone = token.parse::<u32>().ok();
                true
            } else {
                false
            }
        }
        ExchangeField::ZoneOrHq => {
            if is_zone(token) {
                out.zone = token.parse::<u32>().ok();
                true
            } else if is_hq(token) {
                out.hq = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::StateOrPower => {
            if is_state(token) {
                out.state = Some(token.to_string());
                true
            } else if is_power(token) {
                out.power = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::ItuZone => {
            if is_itu_zone(token) {
                out.ituz = token.parse::<u32>().ok();
                true
            } else {
                false
            }
        }
        ExchangeField::Grid => {
            if is_grid(token) {
                out.grid = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::State => {
            if is_state(token) {
                out.state = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::Power => {
            if is_power(token) {
                out.power = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::Age => {
            if is_age(token) {
                out.age = token.parse::<u32>().ok();
                true
            } else {
                false
            }
        }
        ExchangeField::Year => {
            if is_year(token) {
                out.year = token.parse::<u32>().ok();
                true
            } else {
                false
            }
        }
        ExchangeField::Iota => {
            if is_iota(token) {
                out.iota = Some(normalize_iota(token));
                true
            } else {
                false
            }
        }
        ExchangeField::Name => {
            if is_word(token) {
                out.name = Some(title_case(token));
                true
            } else {
                false
            }
        }
        ExchangeField::Qth => {
            if is_word(token) {
                out.qth = Some(title_case(token));
                true
            } else {
                false
            }
        }
        ExchangeField::Time => {
            if is_time(token) {
                out.time = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::Hq => {
            if is_hq(token) {
                out.hq = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::Category => {
            if is_category(token) {
                out.category = Some(token.to_string());
                true
            } else {
                false
            }
        }
        ExchangeField::District => {
            if is_token(token) {
                out.district = Some(token.to_string());
                true
            } else {
                false
            }
        }
    }
}

fn is_rst(token: &str) -> bool {
    (token.len() == 2 || token.len() == 3) && is_digits(token)
}

fn is_digits(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit())
}

fn is_zone(token: &str) -> bool {
    if token.is_empty() || token.len() > 2 || !is_digits(token) {
        return false;
    }
    token.parse::<u32>().is_ok_and(|z| (1..=40).contains(&z))
}

fn is_itu_zone(token: &str) -> bool {
    if token.is_empty() || token.len() > 2 || !is_digits(token) {
        return false;
    }
    token.parse::<u32>().is_ok_and(|z| (1..=90).contains(&z))
}

fn is_grid(token: &str) -> bool {
    let b = token.as_bytes();
    let alpha2 = |i: usize| b[i].is_ascii_alphabetic();
    let digit2 = |i: usize| b[i].is_ascii_digit();
    match b.len() {
        4 => alpha2(0) && alpha2(1) && digit2(2) && digit2(3),
        6 => alpha2(0) && alpha2(1) && digit2(2) && digit2(3) && alpha2(4) && alpha2(5),
        8 => {
            alpha2(0)
                && alpha2(1)
                && digit2(2)
                && digit2(3)
                && alpha2(4)
                && alpha2(5)
                && digit2(6)
                && digit2(7)
        }
        _ => false,
    }
}

fn is_state(token: &str) -> bool {
    token.len() == 2 && token.bytes().all(|b| b.is_ascii_alphabetic())
}

fn is_power(token: &str) -> bool {
    if matches!(token, "KW" | "QRP" | "LP" | "HP" | "LOW" | "HIGH") {
        return true;
    }
    let (num, _suffix) = if let Some(stripped) = token.strip_suffix("KW") {
        (stripped, "KW")
    } else if let Some(stripped) = token.strip_suffix('W') {
        (stripped, "W")
    } else if let Some(stripped) = token.strip_suffix('K') {
        (stripped, "K")
    } else {
        (token, "")
    };
    !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())
}

fn is_age(token: &str) -> bool {
    if token.is_empty() || token.len() > 3 || !is_digits(token) {
        return false;
    }
    token.parse::<u32>().is_ok_and(|a| (1..=130).contains(&a))
}

fn is_year(token: &str) -> bool {
    (token.len() == 2 || token.len() == 4) && is_digits(token)
}

fn is_iota(token: &str) -> bool {
    // Formy: EU-115, EU115, 115.
    if is_digits(token) {
        return token.len() == 3;
    }
    let b = token.as_bytes();
    // "EU-115" (6 znaków, z myślnikiem na pozycji 2).
    if b.len() == 6 && b[2] == b'-' {
        return b[0].is_ascii_alphabetic()
            && b[1].is_ascii_alphabetic()
            && b[3].is_ascii_digit()
            && b[4].is_ascii_digit()
            && b[5].is_ascii_digit();
    }
    // "EU115" (5 znaków, bez myślnika).
    if b.len() == 5 {
        return b[0].is_ascii_alphabetic()
            && b[1].is_ascii_alphabetic()
            && b[2].is_ascii_digit()
            && b[3].is_ascii_digit()
            && b[4].is_ascii_digit();
    }
    false
}

fn normalize_iota(token: &str) -> String {
    let t = token.to_uppercase();
    if t.len() == 5 && !t.contains('-') {
        format!("{}-{}", &t[..2], &t[2..])
    } else {
        t
    }
}

fn is_word(token: &str) -> bool {
    token.len() >= 2 && token.bytes().all(|b| b.is_ascii_alphabetic())
}

fn is_time(token: &str) -> bool {
    if token.len() != 4 || !is_digits(token) {
        return false;
    }
    let hh: u32 = token[..2].parse().unwrap_or(99);
    let mm: u32 = token[2..].parse().unwrap_or(99);
    hh < 24 && mm < 60
}

fn is_hq(token: &str) -> bool {
    token.len() >= 2 && token.len() <= 6 && token.bytes().all(|b| b.is_ascii_alphabetic())
}

fn is_category(token: &str) -> bool {
    if token.len() == 2 {
        let b = token.as_bytes();
        return b[0].is_ascii_digit() && b[1].is_ascii_alphabetic();
    }
    is_word(token) && token.len() <= 3
}

fn is_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = first.to_uppercase().collect::<String>();
            out.push_str(&chars.as_str().to_lowercase());
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields_serial() -> Vec<ExchangeField> {
        vec![ExchangeField::Rst, ExchangeField::Serial]
    }

    #[test]
    fn parses_rst_serial() {
        let p = parse_exchange(&fields_serial(), "599 001").unwrap();
        assert_eq!(p.rst.as_deref(), Some("599"));
        assert_eq!(p.serial, Some(1));
    }

    #[test]
    fn parses_concatenated_rst_serial() {
        let p = parse_exchange(&fields_serial(), "599001").unwrap();
        assert_eq!(p.rst.as_deref(), Some("599"));
        assert_eq!(p.serial, Some(1));
    }

    #[test]
    fn parses_two_digit_rst() {
        let p = parse_exchange(&fields_serial(), "59 1234").unwrap();
        assert_eq!(p.rst.as_deref(), Some("59"));
        assert_eq!(p.serial, Some(1234));
    }

    #[test]
    fn rejects_missing_serial() {
        let err = parse_exchange(&fields_serial(), "599").unwrap_err();
        assert!(err.message.contains("numer"));
    }

    #[test]
    fn rejects_invalid_serial() {
        let err = parse_exchange(&fields_serial(), "599 ABC").unwrap_err();
        assert!(err.message.contains("ABC"));
    }

    #[test]
    fn parses_zone() {
        let p = parse_exchange(&[ExchangeField::Rst, ExchangeField::Zone], "599 15").unwrap();
        assert_eq!(p.zone, Some(15));
    }

    #[test]
    fn rejects_zone_out_of_range() {
        let err = parse_exchange(&[ExchangeField::Rst, ExchangeField::Zone], "599 99").unwrap_err();
        assert!(err.message.contains("strefa CQ"));
    }

    #[test]
    fn parses_grid() {
        let p = parse_exchange(&[ExchangeField::Rst, ExchangeField::Grid], "59 JO80").unwrap();
        assert_eq!(p.grid.as_deref(), Some("JO80"));
    }

    #[test]
    fn parses_iota_with_optional_missing() {
        let p = parse_exchange(
            &[ExchangeField::Rst, ExchangeField::Serial, ExchangeField::Iota],
            "599 007 EU-115",
        )
        .unwrap();
        assert_eq!(p.serial, Some(7));
        assert_eq!(p.iota.as_deref(), Some("EU-115"));

        let p = parse_exchange(
            &[ExchangeField::Rst, ExchangeField::Serial, ExchangeField::Iota],
            "599 007",
        )
        .unwrap();
        assert_eq!(p.serial, Some(7));
        assert_eq!(p.iota, None);
    }

    #[test]
    fn parses_zone_or_hq() {
        let p = parse_exchange(&[ExchangeField::Rst, ExchangeField::ZoneOrHq], "599 ARRL").unwrap();
        assert_eq!(p.hq.as_deref(), Some("ARRL"));
        let p = parse_exchange(&[ExchangeField::Rst, ExchangeField::ZoneOrHq], "599 33").unwrap();
        assert_eq!(p.zone, Some(33));
    }

    #[test]
    fn parses_field_day_category_section() {
        let p = parse_exchange(&[ExchangeField::Category, ExchangeField::District], "2A EPA").unwrap();
        assert_eq!(p.category.as_deref(), Some("2A"));
        assert_eq!(p.district.as_deref(), Some("EPA"));
    }

    #[test]
    fn parses_name_qth() {
        let p = parse_exchange(
            &[ExchangeField::Rst, ExchangeField::Serial, ExchangeField::Name, ExchangeField::Qth],
            "599 001 JOHN NYC",
        )
        .unwrap();
        assert_eq!(p.name.as_deref(), Some("John"));
        assert_eq!(p.qth.as_deref(), Some("Nyc"));
    }

    #[test]
    fn parses_time() {
        let p = parse_exchange(
            &[ExchangeField::Rst, ExchangeField::Serial, ExchangeField::Time],
            "599 001 1452",
        )
        .unwrap();
        assert_eq!(p.time.as_deref(), Some("1452"));
    }

    #[test]
    fn applies_to_qso() {
        let p = parse_exchange(&fields_serial(), "599 042").unwrap();
        let mut q = QsoRecord::default();
        apply_to_qso(&p, &mut q);
        assert_eq!(q.rst_rcvd, "599");
        assert_eq!(q.srx, Some(42));
        assert_eq!(q.srx_string.as_deref(), Some("599 042"));
    }

    #[test]
    fn fields_from_format() {
        assert_eq!(
            fields_from_format_string("RST + Serial"),
            vec![ExchangeField::Rst, ExchangeField::Serial]
        );
        assert_eq!(
            fields_from_format_string("RST + CQ Zone"),
            vec![ExchangeField::Rst, ExchangeField::Zone]
        );
    }

    #[test]
    fn normalizes_iota_without_dash() {
        let p = parse_exchange(
            &[ExchangeField::Rst, ExchangeField::Serial, ExchangeField::Iota],
            "599 001 EU115",
        )
        .unwrap();
        assert_eq!(p.iota.as_deref(), Some("EU-115"));
    }
}
