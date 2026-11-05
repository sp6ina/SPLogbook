// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::contest_stats::MultKind;
use crate::core::exchange::ExchangeField;
use crate::core::exchange::ExchangeField::{
    Age, Category, District, Grid, Iota, Name, Power, Qth, Rst, Serial, State, Time, Year, Zone,
    ZoneOrHq,
};
use crate::core::qso::QsoRecord;
use std::collections::HashSet;

pub type ScoringFn = fn(&[QsoRecord], u32, u8) -> (u32, u32, u32);

pub const CQ_WPX_RTTY_NOT_IN_SUPPORTED_CONTEST_PROFILES: &str =
    "CQ WPX RTTY is not in supported contest profiles; CW/SSB rules cannot be applied to RTTY";

pub const CQ_WW_RTTY_NOT_IN_SUPPORTED_CONTEST_PROFILES: &str =
    "CQ WW RTTY is not in supported contest profiles; CW/SSB rules cannot be applied to RTTY";

/// Błędy domeny zawodów krótkofalarskich.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContestError {
    UnsupportedContest(String),
    UnsupportedContestVariant(String),
    UnsupportedContestCategory(String),
    UnsupportedContestMode {
        contest: &'static str,
        mode: String,
    },
    UnsupportedContestBand {
        contest: &'static str,
        band: String,
    },
    MissingContestExchange {
        contest: &'static str,
        detail: String,
    },
    InvalidContestExchange {
        contest: &'static str,
        detail: String,
    },
    MissingSerialNumber(&'static str),
    InvalidSerialNumber(String),
    MissingTransmitterId(String),
    ContestScoreRuleUnavailable(String),
}

impl std::fmt::Display for ContestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContestError::UnsupportedContest(s) => write!(f, "Unsupported contest: {s}"),
            ContestError::UnsupportedContestVariant(s) => {
                write!(f, "Unsupported contest variant: {s}")
            }
            ContestError::UnsupportedContestCategory(s) => {
                write!(f, "Unsupported contest category: {s}")
            }
            ContestError::UnsupportedContestMode { contest, mode } => {
                write!(f, "Unsupported contest mode '{mode}' for contest {contest}")
            }
            ContestError::UnsupportedContestBand { contest, band } => {
                write!(f, "Unsupported contest band '{band}' for contest {contest}")
            }
            ContestError::MissingContestExchange { contest, detail } => {
                write!(f, "Missing contest exchange for {contest}: {detail}")
            }
            ContestError::InvalidContestExchange { contest, detail } => {
                write!(f, "Invalid contest exchange for {contest}: {detail}")
            }
            ContestError::MissingSerialNumber(contest) => {
                write!(f, "Missing serial number for contest {contest}")
            }
            ContestError::InvalidSerialNumber(s) => write!(f, "Invalid serial number: {s}"),
            ContestError::MissingTransmitterId(s) => {
                write!(f, "Missing transmitter ID for category: {s}")
            }
            ContestError::ContestScoreRuleUnavailable(s) => {
                write!(f, "Contest scoring rule unavailable for: {s}")
            }
        }
    }
}

impl std::error::Error for ContestError {}

/// Ściśle typowany identyfikator wspieranych i rozróżnianych zawodów krótkofalarskich.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContestId {
    SpDx,
    CqWwCw,
    CqWwSsb,
    CqWwRtty,
    CqWpxCw,
    CqWpxSsb,
    CqWpxRtty,
}

impl ContestId {
    /// Rozpoznaje identyfikator konkursu na podstawie oficjalnej lub alternatywnej nazwy.
    pub fn parse_from_name(name: &str) -> Option<Self> {
        let n = name.trim();
        if n.eq_ignore_ascii_case("SP DX Contest") || n.eq_ignore_ascii_case("SP-DX-CONTEST") {
            Some(ContestId::SpDx)
        } else if n.eq_ignore_ascii_case("CQ World Wide DX Contest (CW)")
            || n.eq_ignore_ascii_case("CQ WW CW")
            || n.eq_ignore_ascii_case("CQ-WW-CW")
        {
            Some(ContestId::CqWwCw)
        } else if n.eq_ignore_ascii_case("CQ World Wide DX Contest (SSB)")
            || n.eq_ignore_ascii_case("CQ WW SSB")
            || n.eq_ignore_ascii_case("CQ-WW-SSB")
        {
            Some(ContestId::CqWwSsb)
        } else if n.eq_ignore_ascii_case("CQ World Wide DX Contest (RTTY)")
            || n.eq_ignore_ascii_case("CQ WW RTTY")
            || n.eq_ignore_ascii_case("CQ-WW-RTTY")
        {
            Some(ContestId::CqWwRtty)
        } else if n.eq_ignore_ascii_case("CQ WPX Contest (CW)")
            || n.eq_ignore_ascii_case("CQ WPX CW")
            || n.eq_ignore_ascii_case("CQ-WPX-CW")
        {
            Some(ContestId::CqWpxCw)
        } else if n.eq_ignore_ascii_case("CQ WPX Contest (SSB)")
            || n.eq_ignore_ascii_case("CQ WPX SSB")
            || n.eq_ignore_ascii_case("CQ-WPX-SSB")
        {
            Some(ContestId::CqWpxSsb)
        } else if n.eq_ignore_ascii_case("CQ WPX Contest") || n.eq_ignore_ascii_case("CQ-WPX") {
            Some(ContestId::CqWpxCw)
        } else if n.eq_ignore_ascii_case("CQ WPX RTTY Contest")
            || n.eq_ignore_ascii_case("CQ WPX RTTY")
            || n.eq_ignore_ascii_case("CQ-WPX-RTTY")
            || n.eq_ignore_ascii_case("WPX-RTTY")
        {
            Some(ContestId::CqWpxRtty)
        } else {
            None
        }
    }

    /// Kanoniczny identyfikator konkursu w nagłówku Cabrillo `CONTEST: <NAME>`.
    pub fn cabrillo_contest_name(&self) -> &'static str {
        match self {
            ContestId::SpDx => "SP-DX-CONTEST",
            ContestId::CqWwCw => "CQ-WW-CW",
            ContestId::CqWwSsb => "CQ-WW-SSB",
            ContestId::CqWwRtty => "CQ-WW-RTTY",
            ContestId::CqWpxCw => "CQ-WPX-CW",
            ContestId::CqWpxSsb => "CQ-WPX-SSB",
            ContestId::CqWpxRtty => "CQ-WPX-RTTY",
        }
    }

    /// Zwraca true, jeśli profil jest w pełni zintegrowany i gotowy do przeliczania punktów.
    pub fn is_supported(&self) -> bool {
        match self {
            ContestId::SpDx
            | ContestId::CqWwCw
            | ContestId::CqWwSsb
            | ContestId::CqWpxCw
            | ContestId::CqWpxSsb => true,
            ContestId::CqWwRtty | ContestId::CqWpxRtty => false,
        }
    }
}

/// Jawny profil zawodów definiujący dopuszczalne pasma, emisje, format wymiany i silnik punktacji.
#[derive(Debug, Clone)]
pub struct ContestProfile {
    pub id: ContestId,
    pub name: &'static str,
    pub cabrillo_name: &'static str,
    pub rules_edition: &'static str,
    pub allowed_modes: &'static [&'static str],
    pub allowed_bands: &'static [&'static str],
    pub exchange_format: &'static str,
    pub mult_kind: MultKind,
    pub scoring_fn: Option<ScoringFn>,
}

impl ContestProfile {
    /// Zwraca zweryfikowany profil dla danego identyfikatora zawodów.
    pub fn for_id(id: ContestId) -> Self {
        match id {
            ContestId::SpDx => Self {
                id,
                name: "SP DX Contest",
                cabrillo_name: "SP-DX-CONTEST",
                rules_edition: "PZK SP DX Contest Rules 2024/2025",
                allowed_modes: &["CW", "CW-R", "SSB", "USB", "LSB", "PH"],
                allowed_bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
                exchange_format: "RST + Voivodeship / Serial",
                mult_kind: MultKind::DxccPerBand,
                scoring_fn: Some(score_sp_dx),
            },
            ContestId::CqWwCw => Self {
                id,
                name: "CQ World Wide DX Contest (CW)",
                cabrillo_name: "CQ-WW-CW",
                rules_edition: "CQ WW DX Contest Rules 2024/2025 (CW)",
                allowed_modes: &["CW", "CW-R"],
                allowed_bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
                exchange_format: "RST + CQ Zone",
                mult_kind: MultKind::CqZone,
                scoring_fn: Some(score_cqww),
            },
            ContestId::CqWwSsb => Self {
                id,
                name: "CQ World Wide DX Contest (SSB)",
                cabrillo_name: "CQ-WW-SSB",
                rules_edition: "CQ WW DX Contest Rules 2024/2025 (SSB)",
                allowed_modes: &["SSB", "USB", "LSB", "PH", "AM"],
                allowed_bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
                exchange_format: "RS + CQ Zone",
                mult_kind: MultKind::CqZone,
                scoring_fn: Some(score_cqww),
            },
            ContestId::CqWpxCw => Self {
                id,
                name: "CQ WPX Contest (CW)",
                cabrillo_name: "CQ-WPX-CW",
                rules_edition: "CQ WPX Contest Rules 2024/2025 (CW)",
                allowed_modes: &["CW", "CW-R"],
                allowed_bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
                exchange_format: "RST + Serial",
                mult_kind: MultKind::Prefix,
                scoring_fn: Some(score_wpx),
            },
            ContestId::CqWpxSsb => Self {
                id,
                name: "CQ WPX Contest (SSB)",
                cabrillo_name: "CQ-WPX-SSB",
                rules_edition: "CQ WPX Contest Rules 2024/2025 (SSB)",
                allowed_modes: &["SSB", "USB", "LSB", "PH"],
                allowed_bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
                exchange_format: "RS + Serial",
                mult_kind: MultKind::Prefix,
                scoring_fn: Some(score_wpx),
            },
            ContestId::CqWpxRtty => Self {
                id,
                name: "CQ WPX RTTY Contest",
                cabrillo_name: "CQ-WPX-RTTY",
                rules_edition: "CQ WPX RTTY Contest Rules (Not Supported in Profile)",
                allowed_modes: &["RTTY", "RY"],
                allowed_bands: &["80m", "40m", "20m", "15m", "10m"],
                exchange_format: "RST + Serial",
                mult_kind: MultKind::Prefix,
                scoring_fn: None,
            },
            ContestId::CqWwRtty => Self {
                id,
                name: "CQ World Wide DX Contest (RTTY)",
                cabrillo_name: "CQ-WW-RTTY",
                rules_edition: "CQ WW RTTY Contest Rules (Not Supported in Profile)",
                allowed_modes: &["RTTY", "RY"],
                allowed_bands: &["80m", "40m", "20m", "15m", "10m"],
                exchange_format: "RST + CQ Zone + State/VE",
                mult_kind: MultKind::CqZone,
                scoring_fn: None,
            },
        }
    }

    /// Oblicza wynik na podstawie jawnego profilu zawodów.
    pub fn score_qsos(
        &self,
        qsos: &[QsoRecord],
        my_dxcc: u32,
        my_cqzone: u8,
    ) -> Result<(u32, u32, u32), ContestError> {
        match self.scoring_fn {
            Some(f) => Ok(f(qsos, my_dxcc, my_cqzone)),
            None => {
                if self.id == ContestId::CqWpxRtty {
                    Err(ContestError::UnsupportedContestVariant(
                        CQ_WPX_RTTY_NOT_IN_SUPPORTED_CONTEST_PROFILES.to_string(),
                    ))
                } else if self.id == ContestId::CqWwRtty {
                    Err(ContestError::UnsupportedContestVariant(
                        CQ_WW_RTTY_NOT_IN_SUPPORTED_CONTEST_PROFILES.to_string(),
                    ))
                } else {
                    Err(ContestError::ContestScoreRuleUnavailable(
                        self.name.to_string(),
                    ))
                }
            }
        }
    }
}

impl PartialEq for ContestProfile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ContestProfile {}

pub struct ContestRule {
    pub name: &'static str,
    /// Opis wymiany wyświetlany użytkownikowi.
    pub exchange_format: &'static str,
    /// Uporządkowana struktura wymiany używana przez parser (zob. `core::exchange`).
    pub exchange_fields: &'static [ExchangeField],
    pub bands: &'static [&'static str],
    /// Rodzaj mnożnika używanego do macierzy mnożników.
    pub mult_kind: MultKind,
    pub scoring_fn: ScoringFn,
}

pub fn calculate_score(
    rule: &ContestRule,
    qsos: &[QsoRecord],
    my_dxcc: u32,
    my_cqzone: u8,
) -> (u32, u32, u32) {
    (rule.scoring_fn)(qsos, my_dxcc, my_cqzone)
}

pub fn detect_duplicate(_rule: &ContestRule, qso: &QsoRecord, log: &[QsoRecord]) -> bool {
    for prev in log {
        if prev.callsign == qso.callsign && prev.band == qso.band && prev.mode == qso.mode {
            return true;
        }
    }
    false
}

/// Kanoniczne oznaczenia 16 polskich województw w SP DX Contest (PZK).
pub const SP_VOIVODESHIPS: &[char] = &[
    'B', 'C', 'D', 'F', 'G', 'J', 'K', 'L', 'M', 'O', 'P', 'R', 'S', 'U', 'W', 'Z',
];

/// Mapuje strefę CQ (1..40) na kod kontynentu: `NA`, `SA`, `EU`, `AS`, `OC`, `AF`.
pub fn cqzone_to_continent(z: u8) -> &'static str {
    match z {
        1..=8 => "NA",
        9..=13 => "SA",
        14..=16 => "EU",
        17..=19 => "AS",
        20 => "EU", // Obszar Bałkanów / Cypru / Turcji
        21..=26 => "AS",
        27..=32 => "OC",
        33..=39 => "AF",
        40 => "EU",
        _ => "EU",
    }
}

/// Sprawdza, czy dane pasmo zalicza się do pasm niskich w zawodach WPX (160m, 80m, 40m).
pub fn is_low_band(band: &str) -> bool {
    let b = band.trim().to_ascii_lowercase();
    b == "160m" || b == "80m" || b == "40m"
}

/// Sprawdza, czy dane pasmo jest dozwolone w zawodach CQ WPX (160m, 80m, 40m, 20m, 15m, 10m).
pub fn is_valid_wpx_band(band: &str) -> bool {
    let b = band.trim().to_ascii_lowercase();
    matches!(b.as_str(), "160m" | "80m" | "40m" | "20m" | "15m" | "10m")
}

/// Sprawdza, czy stacja jest stacją polską (DXCC 269 lub prefiks SP/SQ/SN/SO/3Z/HF).
pub fn is_polish_station(q: &QsoRecord) -> bool {
    if q.dxcc == Some(269) {
        return true;
    }
    let pfx = crate::core::prefix::extract_wpx_prefix(&q.callsign);
    pfx.starts_with("SP")
        || pfx.starts_with("SQ")
        || pfx.starts_with("SN")
        || pfx.starts_with("SO")
        || pfx.starts_with("3Z")
        || pfx.starts_with("HF")
}

/// Ekstrahuje literę polskiego województwa z odebranej grupy (`srx_string`) lub stanu (`state`).
pub fn extract_sp_voivodeship(q: &QsoRecord) -> Option<char> {
    let check_token = |token: &str| -> Option<char> {
        let t = token.trim().to_ascii_uppercase();
        if t.len() == 1 {
            let ch = t.chars().next()?;
            if SP_VOIVODESHIPS.contains(&ch) {
                return Some(ch);
            }
        }
        None
    };

    if let Some(srx_s) = &q.srx_string {
        for word in srx_s.split_whitespace() {
            if let Some(v) = check_token(word) {
                return Some(v);
            }
        }
        if let Some(v) = check_token(srx_s) {
            return Some(v);
        }
    }
    if let Some(state) = &q.state {
        for word in state.split_whitespace() {
            if let Some(v) = check_token(word) {
                return Some(v);
            }
        }
        if let Some(v) = check_token(state) {
            return Some(v);
        }
    }
    None
}

/// Punktacja i mnożniki SP DX Contest (PZK) — `FUNC-002`:
///
/// Dla stacji polskiej (`my_dxcc == 269`):
/// - Łączności SP - SP: 0 pkt, brak mnożnika.
/// - Łączności z Europą: 1 pkt, mnożnik podmiot DXCC per pasmo.
/// - Łączności poza Europą: 3 pkt, mnożnik podmiot DXCC per pasmo.
///
/// Dla stacji zagranicznej (`my_dxcc != 269`):
/// - Łączności tylko ze stacjami polskimi (SP): 3 pkt za QSO na dowolnym paśmie.
/// - Łączności ze stacjami zagranicznymi: 0 pkt, brak mnożnika.
/// - Mnożnik: 16 polskich województw (`B, C, D, F, G, J, K, L, M, O, P, R, S, U, W, Z`) per pasmo.
fn score_sp_dx(qsos: &[QsoRecord], my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let is_my_sp = my_dxcc == 269;
    let mut pts = 0;
    let mut mults = HashSet::new();

    for q in qsos {
        let their_is_sp = is_polish_station(q);

        if is_my_sp {
            if their_is_sp {
                continue; // Łączności SP-SP nie dają punktów ani mnożników w SP DX Contest
            }

            let continent = q
                .continent
                .as_deref()
                .unwrap_or_else(|| q.cqz.map(|z| cqzone_to_continent(z as u8)).unwrap_or("EU"));

            if continent == "EU" {
                pts += 1;
            } else {
                pts += 3;
            }

            if let Some(dxcc) = q.dxcc {
                if dxcc != 269 {
                    mults.insert(format!("{}:D{}", q.band, dxcc));
                }
            } else {
                let pfx = crate::core::prefix::extract_wpx_prefix(&q.callsign);
                if !pfx.is_empty() {
                    mults.insert(format!("{}:P_{}", q.band, pfx));
                }
            }
        } else {
            if !their_is_sp {
                continue; // Stacje zagraniczne pracują wyłącznie ze stacjami SP
            }

            pts += 3;

            if let Some(voiv) = extract_sp_voivodeship(q) {
                mults.insert(format!("{}:V_{}", q.band, voiv));
            }
        }
    }

    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

/// Punktacja i mnożniki CQ World Wide DX Contest — `FUNC-002`:
///
/// Punkty QSO:
/// - Własny kraj DXCC: 0 pkt (ale liczy się jako mnożnik).
/// - Ten sam kontynent, inny podmiot DXCC:
///   - Wewnątrz Ameryki Północnej (NA): 2 pkt (reguła IV.B CQ WW).
///   - Na pozostałych kontynentach (np. EU): 1 pkt.
/// - Inny kontynent: 3 pkt.
///
/// Mnożniki:
/// - Strefy CQ (1..40) per pasmo.
/// - Podmioty DXCC per pasmo.
fn score_cqww(qsos: &[QsoRecord], my_dxcc: u32, my_cqzone: u8) -> (u32, u32, u32) {
    let my_cont = if my_dxcc == 269 {
        "EU"
    } else if my_dxcc == 291 || my_dxcc == 1 {
        "NA"
    } else {
        cqzone_to_continent(my_cqzone)
    };

    let mut pts = 0;
    let mut mults = HashSet::new();

    for q in qsos {
        let their_cont = q
            .continent
            .as_deref()
            .unwrap_or_else(|| q.cqz.map(|z| cqzone_to_continent(z as u8)).unwrap_or("EU"));

        if q.dxcc == Some(my_dxcc) {
            pts += 0;
        } else if their_cont == my_cont {
            if my_cont == "NA" {
                pts += 2;
            } else {
                pts += 1;
            }
        } else {
            pts += 3;
        }

        if let Some(z) = q.cqz {
            if (1..=40).contains(&z) {
                mults.insert(format!("{}:Z{:02}", q.band, z));
            }
        }
        if let Some(d) = q.dxcc {
            mults.insert(format!("{}:D{}", q.band, d));
        }
    }

    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_arrl_dx(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3; // 3 pts per QSO
        if let Some(state) = &q.state {
            mults.insert(format!("{}:{}", q.band, state));
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

/// Punktacja i mnożniki CQ WPX Contest — `FUNC-002`:
///
/// Punkty QSO z uwzględnieniem podwojenia na pasmach niskich (160m, 80m, 40m):
/// - Własny kraj DXCC: 1 pkt na każdym paśmie (zarówno wysokie: 20m/15m/10m, jak i niskie: 160m/80m/40m; brak podwojenia).
/// - Ten sam kontynent, inny DXCC:
///   - Poza NA: 1 pkt (wysokie) / 2 pkt (niskie).
///   - Wewnątrz NA: 2 pkt (wysokie) / 4 pkt (niskie).
/// - Inny kontynent: 3 pkt (wysokie) / 6 pkt (niskie).
///
/// Mnożniki:
/// - Unikalne prefiksy WPX zaliczane raz na całe zawody (niezależnie od pasma).
fn score_wpx(qsos: &[QsoRecord], my_dxcc: u32, my_cqzone: u8) -> (u32, u32, u32) {
    let my_cont = if my_dxcc == 269 {
        "EU"
    } else if my_dxcc == 291 || my_dxcc == 1 {
        "NA"
    } else {
        cqzone_to_continent(my_cqzone)
    };

    let mut pts = 0;
    let mut mults = HashSet::new();

    for q in qsos {
        let low = is_low_band(&q.band);
        let their_cont = q
            .continent
            .as_deref()
            .unwrap_or_else(|| q.cqz.map(|z| cqzone_to_continent(z as u8)).unwrap_or("EU"));

        if !is_valid_wpx_band(&q.band) {
            continue;
        }

        if q.dxcc == Some(my_dxcc) {
            pts += 1;
        } else if their_cont == my_cont {
            if my_cont == "NA" {
                pts += if low { 4 } else { 2 };
            } else {
                pts += if low { 2 } else { 1 };
            }
        } else {
            pts += if low { 6 } else { 3 };
        }

        let prefix = crate::core::prefix::extract_wpx_prefix(&q.callsign);
        if !prefix.is_empty() {
            mults.insert(prefix);
        }
    }

    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_vhf(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 100; // 1pt/km approx
        if let Some(grid) = &q.gridsquare {
            mults.insert(grid.clone());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_iaru(qsos: &[QsoRecord], _my_dxcc: u32, my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        if q.cqz == Some(my_cqzone as u32) {
            pts += 1;
        } else {
            pts += 3;
        }
        if let Some(z) = q.cqz {
            mults.insert(z.to_string());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_1pt_dxcc(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 1;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_iota(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        if let Some(i) = &q.iota {
            pts += 15;
            mults.insert(i.clone());
        } else {
            pts += 3;
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_field_day(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += if q.mode == "CW" || q.mode == "DIGI" {
            2
        } else {
            1
        };
        if let Some(state) = &q.state {
            mults.insert(state.clone());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_eu_hf(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        if q.continent.as_deref() == Some("EU") {
            pts += 1;
        } else {
            pts += 2;
        }
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_3pt_dxcc(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3;
        if let Some(d) = q.dxcc {
            mults.insert(d.to_string());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_king_spain(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += if q.continent.as_deref() == Some("EU") {
            1
        } else {
            3
        };
        if let Some(s) = &q.state {
            mults.insert(s.clone());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_rda(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 2;
        if let Some(s) = &q.state {
            mults.insert(s.clone());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_bartg(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 6;
        if let Some(d) = q.dxcc {
            mults.insert(format!("{}-{}", d, q.band));
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

fn score_ukrainian(qsos: &[QsoRecord], _my_dxcc: u32, _my_cqzone: u8) -> (u32, u32, u32) {
    let mut pts = 0;
    let mut mults = HashSet::new();
    for q in qsos {
        pts += 3;
        if let Some(s) = &q.state {
            mults.insert(s.clone());
        }
    }
    let mult_count = mults.len() as u32;
    (pts, mult_count, pts * mult_count)
}

pub const RULES: &[ContestRule] = &[
    ContestRule {
        name: "SP DX Contest",
        exchange_format: "RST + Serial",
        exchange_fields: &[Rst, Serial],
        bands: &["80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::DxccPerBand,
        scoring_fn: score_sp_dx,
    },
    ContestRule {
        name: "CQ World Wide DX Contest (CW)",
        exchange_format: "RST + CQ Zone",
        exchange_fields: &[Rst, Zone],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::CqZone,
        scoring_fn: score_cqww,
    },
    ContestRule {
        name: "CQ World Wide DX Contest (SSB)",
        exchange_format: "RST + CQ Zone",
        exchange_fields: &[Rst, Zone],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::CqZone,
        scoring_fn: score_cqww,
    },
    ContestRule {
        name: "CQ WPX Contest",
        exchange_format: "RST + Serial",
        exchange_fields: &[Rst, Serial],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Prefix,
        scoring_fn: score_wpx,
    },
    ContestRule {
        name: "ARRL International DX",
        exchange_format: "RST + Power",
        exchange_fields: &[Rst, Power],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::State,
        scoring_fn: score_arrl_dx,
    },
    ContestRule {
        name: "VHFUHF (SP)",
        exchange_format: "RST + Grid",
        exchange_fields: &[Rst, Grid],
        bands: &["2m", "70cm"],
        mult_kind: MultKind::Grid,
        scoring_fn: score_vhf,
    },
    ContestRule {
        name: "IARU HF Championship",
        exchange_format: "RST + Zone/HQ",
        exchange_fields: &[Rst, ZoneOrHq],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::CqZone,
        scoring_fn: score_iaru,
    },
    ContestRule {
        name: "WAE DX Contest",
        exchange_format: "RST + Serial",
        exchange_fields: &[Rst, Serial],
        bands: &["80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_1pt_dxcc,
    },
    ContestRule {
        name: "RSGB IOTA Contest",
        exchange_format: "RST + Serial + IOTA",
        exchange_fields: &[Rst, Serial, Iota],
        bands: &["80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Iota,
        scoring_fn: score_iota,
    },
    ContestRule {
        name: "ARRL Field Day",
        exchange_format: "Category + ARRL Section",
        exchange_fields: &[Category, State],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::State,
        scoring_fn: score_field_day,
    },
    ContestRule {
        name: "Scandinavian Activity Contest",
        exchange_format: "RST + Serial",
        exchange_fields: &[Rst, Serial],
        bands: &["80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_1pt_dxcc,
    },
    ContestRule {
        name: "EU HF Championship",
        exchange_format: "RST + Year of 1st License",
        exchange_fields: &[Rst, Year],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_eu_hf,
    },
    ContestRule {
        name: "OK/OM DX Contest",
        exchange_format: "RST + District",
        exchange_fields: &[Rst, District],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_3pt_dxcc,
    },
    ContestRule {
        name: "King of Spain DX Contest",
        exchange_format: "RST + Province",
        exchange_fields: &[Rst, State],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::State,
        scoring_fn: score_king_spain,
    },
    ContestRule {
        name: "All Asian DX Contest",
        exchange_format: "RST + Age",
        exchange_fields: &[Rst, Age],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_1pt_dxcc,
    },
    ContestRule {
        name: "JIDX Contest",
        exchange_format: "RST + Prefecture",
        exchange_fields: &[Rst, District],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_1pt_dxcc,
    },
    ContestRule {
        name: "RDA Contest",
        exchange_format: "RST + District",
        exchange_fields: &[Rst, District],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::State,
        scoring_fn: score_rda,
    },
    ContestRule {
        name: "Marconi Memorial HF",
        exchange_format: "RST + Serial",
        exchange_fields: &[Rst, Serial],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_1pt_dxcc,
    },
    ContestRule {
        name: "BARTG HF RTTY",
        exchange_format: "RST + Serial + Time",
        exchange_fields: &[Rst, Serial, Time],
        bands: &["80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::DxccPerBand,
        scoring_fn: score_bartg,
    },
    ContestRule {
        name: "NA RTTY Sprint",
        exchange_format: "RST + Serial + Name + QTH",
        exchange_fields: &[Rst, Serial, Name, Qth],
        bands: &["80m", "40m", "20m"],
        mult_kind: MultKind::Dxcc,
        scoring_fn: score_3pt_dxcc,
    },
    ContestRule {
        name: "Ukrainian DX Contest",
        exchange_format: "RST + Oblast",
        exchange_fields: &[Rst, District],
        bands: &["160m", "80m", "40m", "20m", "15m", "10m"],
        mult_kind: MultKind::State,
        scoring_fn: score_ukrainian,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn make_qso(
        call: &str,
        band: &str,
        dxcc: Option<u32>,
        cqz: Option<u32>,
        continent: Option<&str>,
        srx_string: Option<&str>,
    ) -> QsoRecord {
        let mut q = QsoRecord::new(call, band, "CW");
        q.freq = None;
        q.dxcc = dxcc;
        q.cqz = cqz;
        q.continent = continent.map(str::to_string);
        q.srx_string = srx_string.map(str::to_string);
        q
    }

    #[test]
    fn sp_dx_contest_scoring_for_sp_and_non_sp_stations_matches_pzk_rules() {
        let rule_idx = RULES
            .iter()
            .position(|r| r.name == "SP DX Contest")
            .unwrap();
        let rule = &RULES[rule_idx];

        // --- Część 1: Stacja polska (my_dxcc = 269) ---
        let sp_qsos = vec![
            // 1. SP-SP łączność: 0 pkt, 0 mnożników
            make_qso(
                "SP1ABC",
                "20m",
                Some(269),
                Some(15),
                Some("EU"),
                Some("001"),
            ),
            // 2. EU non-SP: 1 pkt, mnożnik 20m:D230
            make_qso(
                "DL1XYZ",
                "20m",
                Some(230),
                Some(14),
                Some("EU"),
                Some("002"),
            ),
            // 3. Kolejne QSO z tym samym podmiotem na tym samym paśmie: 1 pkt, mnożnik już jest
            make_qso(
                "DL2BBB",
                "20m",
                Some(230),
                Some(14),
                Some("EU"),
                Some("003"),
            ),
            // 4. Ten sam podmiot na innym paśmie: 1 pkt, mnożnik 40m:D230
            make_qso(
                "DL1XYZ",
                "40m",
                Some(230),
                Some(14),
                Some("EU"),
                Some("004"),
            ),
            // 5. Stacja poza Europą (DX): 3 pkt, mnożnik 20m:D291
            make_qso("W1AW", "20m", Some(291), Some(5), Some("NA"), Some("005")),
        ];

        let (pts, mults, total) = calculate_score(rule, &sp_qsos, 269, 15);
        // Punkty: 0 + 1 + 1 + 1 + 3 = 6
        assert_eq!(pts, 6, "Punkty dla stacji SP");
        // Mnożniki: 20m:D230, 40m:D230, 20m:D291 = 3
        assert_eq!(mults, 3, "Liczba mnożników dla stacji SP");
        // Wynik łączny: 6 * 3 = 18
        assert_eq!(total, 18, "Wynik łączny dla stacji SP");

        // --- Część 2: Stacja zagraniczna (np. DL1XYZ, my_dxcc = 230) ---
        let foreign_qsos = vec![
            // 1. Łączność zagraniczna - zagraniczna: 0 pkt, 0 mnożników
            make_qso(
                "OK1KPA",
                "20m",
                Some(503),
                Some(15),
                Some("EU"),
                Some("001"),
            ),
            // 2. Łączność z SP na 80m z województwem D: 3 pkt, mnożnik 80m:V_D
            make_qso("SP6INA", "80m", Some(269), Some(15), Some("EU"), Some("D")),
            // 3. Łączność z SP na 80m z województwem Z: 3 pkt, mnożnik 80m:V_Z
            make_qso("SP1ABC", "80m", Some(269), Some(15), Some("EU"), Some("Z")),
            // 4. Łączność z SP na 80m z powtórzonym województwem D: 3 pkt, mnożnik powtórzony
            make_qso("SP2DEF", "80m", Some(269), Some(15), Some("EU"), Some("D")),
            // 5. Łączność z SP na 20m z województwem D: 3 pkt, nowy mnożnik dla pasma 20m
            make_qso("SP6INA", "20m", Some(269), Some(15), Some("EU"), Some("D")),
        ];

        let (pts_f, mults_f, total_f) = calculate_score(rule, &foreign_qsos, 230, 14);
        // Punkty: 0 + 3 + 3 + 3 + 3 = 12
        assert_eq!(pts_f, 12, "Punkty dla stacji zagranicznej");
        // Mnożniki: 80m:V_D, 80m:V_Z, 20m:V_D = 3
        assert_eq!(mults_f, 3, "Liczba mnożników dla stacji zagranicznej");
        // Wynik łączny: 12 * 3 = 36
        assert_eq!(total_f, 36, "Wynik łączny dla stacji zagranicznej");
    }

    #[test]
    fn cq_ww_contest_scoring_matches_cq_rules() {
        let rule_idx = RULES
            .iter()
            .position(|r| r.name.starts_with("CQ World Wide DX"))
            .unwrap();
        let rule = &RULES[rule_idx];

        // Stacja europejska (Polska: my_dxcc = 269, strefa 15)
        let eu_qsos = vec![
            // 1. Własny kraj DXCC: 0 pkt, ale mnożnik strefy i kraju
            make_qso("SP1ABC", "20m", Some(269), Some(15), Some("EU"), None),
            // 2. Ten sam kontynent, inny DXCC: 1 pkt, mnożniki strefy 14 i kraju 230
            make_qso("DL1XYZ", "20m", Some(230), Some(14), Some("EU"), None),
            // 3. Inny kontynent (USA): 3 pkt, mnożniki strefy 05 i kraju 291
            make_qso("W1AW", "20m", Some(291), Some(5), Some("NA"), None),
        ];

        let (pts, mults, total) = calculate_score(rule, &eu_qsos, 269, 15);
        // Punkty: 0 + 1 + 3 = 4
        assert_eq!(pts, 4);
        // Mnożniki: 20m:Z15, 20m:D269, 20m:Z14, 20m:D230, 20m:Z05, 20m:D291 = 6
        assert_eq!(mults, 6);
        assert_eq!(total, 24);

        // Stacja północnoamerykańska (USA: my_dxcc = 291, strefa 5)
        let na_qsos = vec![
            // 1. Własny kraj DXCC: 0 pkt
            make_qso("K1ABC", "20m", Some(291), Some(5), Some("NA"), None),
            // 2. Ten sam kontynent NA, inny kraj (Kanada): 2 pkt
            make_qso("VE3XYZ", "20m", Some(1), Some(4), Some("NA"), None),
            // 3. Inny kontynent (Europa): 3 pkt
            make_qso("SP6INA", "20m", Some(269), Some(15), Some("EU"), None),
        ];

        let (pts_na, _, _) = calculate_score(rule, &na_qsos, 291, 5);
        // Punkty: 0 + 2 + 3 = 5
        assert_eq!(pts_na, 5);
    }

    #[test]
    fn cq_wpx_contest_scoring_and_low_band_doubling() {
        let rule_idx = RULES
            .iter()
            .position(|r| r.name == "CQ WPX Contest")
            .unwrap();
        let rule = &RULES[rule_idx];

        // Stacja europejska (Polska: my_dxcc = 269)
        let qsos = vec![
            // 1. Własny kraj na pasmie wysokim (20m): 1 pkt, prefiks SP1
            make_qso("SP1ABC", "20m", Some(269), Some(15), Some("EU"), None),
            // 2. Własny kraj na pasmie niskim (80m): 1 pkt (brak podwojenia na pasmach niskich w tym samym kraju)
            make_qso("SP1ABC", "80m", Some(269), Some(15), Some("EU"), None),
            // 3. Ten sam kontynent EU, inny kraj na pasmie wysokim (20m): 1 pkt, prefiks DL1
            make_qso("DL1XYZ", "20m", Some(230), Some(14), Some("EU"), None),
            // 4. Ten sam kontynent EU, inny kraj na pasmie niskim (40m): 2 pkt (podwojenie)
            make_qso("DL1XYZ", "40m", Some(230), Some(14), Some("EU"), None),
            // 5. Inny kontynent na pasmie wysokim (20m): 3 pkt, prefiks W1
            make_qso("W1AW", "20m", Some(291), Some(5), Some("NA"), None),
            // 6. Inny kontynent na pasmie niskim (80m): 6 pkt (podwojenie)
            make_qso("W1AW", "80m", Some(291), Some(5), Some("NA"), None),
        ];

        let (pts, mults, total) = calculate_score(rule, &qsos, 269, 15);
        // Punkty: 1 + 1 + 1 + 2 + 3 + 6 = 14
        assert_eq!(
            pts, 14,
            "Punkty: same-country bez podwojenia na 80m, podwojenie dla innych krajów"
        );
        // Mnożniki: SP1, DL1, W1 = 3 prefiksy
        assert_eq!(mults, 3, "Liczba unikalnych prefiksów WPX");
        // Wynik łączny: 14 * 3 = 42
        assert_eq!(total, 42, "Wynik łączny WPX");
    }

    #[test]
    fn cqwpx_same_country_high_band_scores_one() {
        let q = make_qso(
            "SP1ABC",
            "20m",
            Some(269),
            Some(15),
            Some("EU"),
            Some("001"),
        );
        let (pts, mults, total) = score_wpx(&[q], 269, 15);
        assert_eq!(pts, 1);
        assert_eq!(mults, 1);
        assert_eq!(total, 1);
    }

    #[test]
    fn cqwpx_same_country_low_band_scores_one() {
        let q = make_qso(
            "SP1ABC",
            "80m",
            Some(269),
            Some(15),
            Some("EU"),
            Some("001"),
        );
        let (pts, mults, total) = score_wpx(&[q], 269, 15);
        assert_eq!(
            pts, 1,
            "Same country low band MUST score 1 point without doubling"
        );
        assert_eq!(mults, 1);
        assert_eq!(total, 1);
    }

    #[test]
    fn cqwpx_same_continent_high_band_scores_one() {
        let q = make_qso(
            "DL1XYZ",
            "20m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("001"),
        );
        let (pts, mults, total) = score_wpx(&[q], 269, 15);
        assert_eq!(pts, 1);
        assert_eq!(mults, 1);
        assert_eq!(total, 1);
    }

    #[test]
    fn cqwpx_same_continent_low_band_scores_two() {
        let q = make_qso(
            "DL1XYZ",
            "40m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("001"),
        );
        let (pts, mults, total) = score_wpx(&[q], 269, 15);
        assert_eq!(pts, 2);
        assert_eq!(mults, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn cqwpx_north_america_high_band_scores_two() {
        let q = make_qso("VE3XYZ", "20m", Some(1), Some(4), Some("NA"), Some("001"));
        let (pts, mults, total) = score_wpx(&[q], 291, 5);
        assert_eq!(pts, 2);
        assert_eq!(mults, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn cqwpx_north_america_low_band_scores_four() {
        let q = make_qso("VE3XYZ", "80m", Some(1), Some(4), Some("NA"), Some("001"));
        let (pts, mults, total) = score_wpx(&[q], 291, 5);
        assert_eq!(pts, 4);
        assert_eq!(mults, 1);
        assert_eq!(total, 4);
    }

    #[test]
    fn cqwpx_different_continent_high_band_scores_three() {
        let q = make_qso("W1AW", "20m", Some(291), Some(5), Some("NA"), Some("001"));
        let (pts, mults, total) = score_wpx(&[q], 269, 15);
        assert_eq!(pts, 3);
        assert_eq!(mults, 1);
        assert_eq!(total, 3);
    }

    #[test]
    fn cqwpx_different_continent_low_band_scores_six() {
        let q = make_qso("W1AW", "80m", Some(291), Some(5), Some("NA"), Some("001"));
        let (pts, mults, total) = score_wpx(&[q], 269, 15);
        assert_eq!(pts, 6);
        assert_eq!(mults, 1);
        assert_eq!(total, 6);
    }

    #[test]
    fn cqwpx_prefix_multiplier_counts_once_across_bands() {
        let q1 = make_qso(
            "DL1XYZ",
            "20m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("001"),
        );
        let q2 = make_qso(
            "DL1XYZ",
            "40m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("002"),
        );
        let (pts, mults, total) = score_wpx(&[q1, q2], 269, 15);
        assert_eq!(pts, 3);
        assert_eq!(mults, 1);
        assert_eq!(total, 3);
    }

    #[test]
    fn cqwpx_same_prefix_different_calls_counts_once() {
        let q1 = make_qso(
            "DL1XYZ",
            "20m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("001"),
        );
        let q2 = make_qso(
            "DL1ABC",
            "20m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("002"),
        );
        let (pts, mults, total) = score_wpx(&[q1, q2], 269, 15);
        assert_eq!(pts, 2);
        assert_eq!(mults, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn cqwpx_different_numbering_creates_distinct_prefix() {
        let q1 = make_qso(
            "SP1ABC",
            "20m",
            Some(269),
            Some(15),
            Some("EU"),
            Some("001"),
        );
        let q2 = make_qso(
            "SP2DEF",
            "20m",
            Some(269),
            Some(15),
            Some("EU"),
            Some("002"),
        );
        let (pts, mults, total) = score_wpx(&[q1, q2], 269, 15);
        assert_eq!(pts, 2);
        assert_eq!(mults, 2);
        assert_eq!(total, 4);
    }

    #[test]
    fn cqwpx_portable_prefix_follows_contract() {
        let q = make_qso(
            "SP6INA/1",
            "20m",
            Some(269),
            Some(15),
            Some("EU"),
            Some("001"),
        );
        let (_, mults, _) = score_wpx(&[q], 269, 15);
        assert_eq!(mults, 1);
        assert_eq!(crate::core::prefix::extract_wpx_prefix("SP6INA/1"), "SP1");
    }

    #[test]
    fn cqwpx_mobile_suffix_does_not_create_false_prefix() {
        let q = make_qso(
            "DL1ABC/M",
            "20m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("001"),
        );
        let (_, mults, _) = score_wpx(&[q], 269, 15);
        assert_eq!(mults, 1);
        assert_eq!(crate::core::prefix::extract_wpx_prefix("DL1ABC/M"), "DL1");
    }

    #[test]
    fn cqwpx_call_without_number_follows_contract() {
        let q = make_qso("RA", "20m", Some(54), Some(16), Some("EU"), Some("001"));
        let (_, mults, _) = score_wpx(&[q], 269, 15);
        assert_eq!(mults, 1);
        assert_eq!(crate::core::prefix::extract_wpx_prefix("RA"), "RA0");
    }

    #[test]
    fn contest_profile_supports_spdx_cqww_cqwpx() {
        let spdx = ContestProfile::for_id(ContestId::SpDx);
        assert_eq!(spdx.cabrillo_name, "SP-DX-CONTEST");
        assert!(spdx.id.is_supported());

        let cqww_cw = ContestProfile::for_id(ContestId::CqWwCw);
        assert_eq!(cqww_cw.cabrillo_name, "CQ-WW-CW");
        assert!(cqww_cw.id.is_supported());

        let cqww_ssb = ContestProfile::for_id(ContestId::CqWwSsb);
        assert_eq!(cqww_ssb.cabrillo_name, "CQ-WW-SSB");
        assert!(cqww_ssb.id.is_supported());

        let cqwpx_cw = ContestProfile::for_id(ContestId::CqWpxCw);
        assert_eq!(cqwpx_cw.cabrillo_name, "CQ-WPX-CW");
        assert!(cqwpx_cw.id.is_supported());

        let cqwpx_ssb = ContestProfile::for_id(ContestId::CqWpxSsb);
        assert_eq!(cqwpx_ssb.cabrillo_name, "CQ-WPX-SSB");
        assert!(cqwpx_ssb.id.is_supported());
    }

    #[test]
    fn cqwpx_rtty_does_not_use_cw_ssb_rules() {
        let rtty_profile = ContestProfile::for_id(ContestId::CqWpxRtty);
        assert_eq!(rtty_profile.cabrillo_name, "CQ-WPX-RTTY");
        assert!(!rtty_profile.id.is_supported());
        assert!(rtty_profile.scoring_fn.is_none());

        let qso = make_qso(
            "DL1XYZ",
            "20m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("001"),
        );
        let res = rtty_profile.score_qsos(&[qso], 269, 15);
        assert_eq!(
            res,
            Err(ContestError::UnsupportedContestVariant(
                CQ_WPX_RTTY_NOT_IN_SUPPORTED_CONTEST_PROFILES.to_string()
            ))
        );
    }

    #[test]
    fn cqww_rtty_does_not_use_cw_ssb_rules_if_supported() {
        let rtty_profile = ContestProfile::for_id(ContestId::CqWwRtty);
        assert_eq!(rtty_profile.cabrillo_name, "CQ-WW-RTTY");
        assert!(!rtty_profile.id.is_supported());
        assert!(rtty_profile.scoring_fn.is_none());

        let qso = make_qso("DL1XYZ", "20m", Some(230), Some(14), Some("EU"), Some("14"));
        let res = rtty_profile.score_qsos(&[qso], 269, 15);
        assert_eq!(
            res,
            Err(ContestError::UnsupportedContestVariant(
                CQ_WW_RTTY_NOT_IN_SUPPORTED_CONTEST_PROFILES.to_string()
            ))
        );
    }

    #[test]
    fn unsupported_contest_and_unsupported_variant_are_rejected() {
        assert_eq!(ContestId::parse_from_name("NonExistentContest2026"), None);

        let err = ContestError::UnsupportedContestVariant("CQ-WPX-RTTY".to_string());
        assert_eq!(err.to_string(), "Unsupported contest variant: CQ-WPX-RTTY");

        let err_cat = ContestError::UnsupportedContestCategory("MULTI-TWO".to_string());
        assert_eq!(
            err_cat.to_string(),
            "Unsupported contest category: MULTI-TWO"
        );
    }

    #[test]
    fn contest_profile_does_not_infer_profile_from_single_qso_mode() {
        let qso = make_qso(
            "DL1XYZ",
            "20m",
            Some(230),
            Some(14),
            Some("EU"),
            Some("001"),
        );
        assert_eq!(qso.mode, "CW");

        let parsed = ContestId::parse_from_name("CQ WPX Contest (SSB)");
        assert_eq!(parsed, Some(ContestId::CqWpxSsb));
        let profile = ContestProfile::for_id(parsed.unwrap());
        assert_eq!(profile.id, ContestId::CqWpxSsb);
    }
}
