// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Sesja zawodów (STAGE-V140-S7): które QSO należą do zawodów, jak je oznaczać
//! i z jakimi kategoriami Cabrillo zgłosić log.
//!
//! Do sesji należą QSO z polem `CONTEST_ID` równym identyfikatorowi sesji i czasem
//! w oknie od startu do zakończenia sesji. Dzięki temu punktacja, mnożniki, tempo
//! i eksport Cabrillo nie obejmują łączności sprzed lat z tego samego dziennika.

use crate::core::qso::{QsoRecord, utc_epoch_seconds};
use serde::{Deserialize, Serialize};

/// Sposób pracy w zawodach (wpływa na ESM: co wysyła klawisz Enter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OperatingMode {
    /// Wołanie CQ na własnej częstotliwości.
    #[default]
    Run,
    /// Szukanie i wołanie stacji (search and pounce).
    SearchAndPounce,
}

/// Kategorie i dane nagłówka Cabrillo 3.0 (wartości według specyfikacji WWROF).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CabrilloCategories {
    /// `SINGLE-OP`, `MULTI-OP`, `CHECKLOG`.
    pub operator: String,
    /// `ONE`, `TWO`, `LIMITED`, `UNLIMITED`, `SWL`.
    pub transmitter: String,
    /// `HIGH`, `LOW`, `QRP`.
    pub power: String,
    /// `ALL`, `160M` … `10M`, `6M`, `2M` …
    pub band: String,
    /// `CW`, `DIGI`, `FM`, `RTTY`, `SSB`, `MIXED`.
    pub mode: String,
    /// `FIXED`, `PORTABLE`, `MOBILE`, …
    pub station: String,
    /// `ASSISTED`, `NON-ASSISTED`.
    pub assisted: String,
    /// Opcjonalna nakładka: `CLASSIC`, `ROOKIE`, `TB-WIRES`, `YOUTH`, `NOVICE-TECH`, `YL`.
    pub overlay: String,
    /// `LOCATION:` — sekcja ARRL albo `DX` (wymagane w IARU HF, ARRL i CQ).
    pub location: String,
    pub club: String,
    /// Znaki operatorów; puste = znak stacji.
    pub operators: String,
    pub soapbox: String,
}

impl Default for CabrilloCategories {
    fn default() -> Self {
        Self {
            operator: "SINGLE-OP".to_string(),
            transmitter: "ONE".to_string(),
            power: "HIGH".to_string(),
            band: "ALL".to_string(),
            mode: "MIXED".to_string(),
            station: "FIXED".to_string(),
            assisted: "NON-ASSISTED".to_string(),
            overlay: String::new(),
            location: "DX".to_string(),
            club: String::new(),
            operators: String::new(),
            soapbox: String::new(),
        }
    }
}

/// Dozwolone wartości kategorii Cabrillo 3.0 (wwrof.org, nagłówek v3, 2025-06-05).
pub mod allowed {
    pub const OPERATOR: &[&str] = &["SINGLE-OP", "MULTI-OP", "CHECKLOG"];
    pub const TRANSMITTER: &[&str] = &["ONE", "TWO", "LIMITED", "UNLIMITED", "SWL"];
    pub const POWER: &[&str] = &["HIGH", "LOW", "QRP"];
    pub const BAND: &[&str] = &[
        "ALL", "160M", "80M", "40M", "20M", "15M", "10M", "6M", "4M", "2M", "222", "432",
    ];
    pub const MODE: &[&str] = &["CW", "DIGI", "FM", "RTTY", "SSB", "MIXED"];
    pub const STATION: &[&str] = &[
        "FIXED",
        "PORTABLE",
        "MOBILE",
        "EXPEDITION",
        "HQ",
        "SCHOOL",
        "DISTRIBUTED",
        "ROVER",
        "ROVER-LIMITED",
        "ROVER-UNLIMITED",
        "EXPLORER",
    ];
    pub const ASSISTED: &[&str] = &["NON-ASSISTED", "ASSISTED"];
    pub const OVERLAY: &[&str] = &[
        "",
        "CLASSIC",
        "ROOKIE",
        "TB-WIRES",
        "YOUTH",
        "NOVICE-TECH",
        "YL",
    ];
}

/// Aktywna albo zakończona sesja zawodów.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContestSession {
    /// Nazwa zawodów z listy reguł albo własnych zawodów.
    pub contest_name: String,
    /// Identyfikator Cabrillo zapisywany w polu `CONTEST_ID` każdego QSO (A-Z, 0-9, `-`).
    pub contest_id: String,
    /// Początek sesji (sekundy UTC od epoki).
    pub started_at: i64,
    /// Koniec sesji; `None` = trwa.
    pub ended_at: Option<i64>,
    pub mode: OperatingMode,
    /// Wymiana nadawana; `#` zastępuje numer kolejny (np. `#` w WPX, `15` w CQ WW).
    pub sent_exchange: String,
    /// Enter wysyła makro właściwe dla etapu łączności (Enter Sends Message).
    pub esm: bool,
    pub categories: CabrilloCategories,
}

impl Default for ContestSession {
    fn default() -> Self {
        Self {
            contest_name: String::new(),
            contest_id: String::new(),
            started_at: 0,
            ended_at: None,
            mode: OperatingMode::Run,
            sent_exchange: "#".to_string(),
            esm: true,
            categories: CabrilloCategories::default(),
        }
    }
}

/// Identyfikator Cabrillo dla nazwy zawodów: znane nazwy mają ustalone identyfikatory,
/// pozostałe są sprowadzane do dozwolonych znaków (A-Z, 0-9, `-`, najwyżej 32 znaki).
pub fn cabrillo_id_for(contest_name: &str) -> String {
    if let Some(id) = crate::core::contest_rules::ContestId::parse_from_name(contest_name) {
        return id.cabrillo_contest_name().to_string();
    }
    let known = [
        ("IARU HF Championship", "IARU-HF"),
        ("RSGB IOTA Contest", "RSGB-IOTA"),
        ("ARRL Field Day", "ARRL-FD"),
    ];
    if let Some((_, id)) = known
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(contest_name.trim()))
    {
        return (*id).to_string();
    }
    sanitize_contest_id(contest_name)
}

/// Sprowadza tekst do znaków dozwolonych w `CONTEST:` (A-Z, 0-9, `-`, najwyżej 32 znaki).
pub fn sanitize_contest_id(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().chars() {
        let c = c.to_ascii_uppercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let mut id: String = out.trim_end_matches('-').chars().take(32).collect();
    while id.ends_with('-') {
        id.pop();
    }
    id
}

/// Domyślna wymiana nadawana dla zawodów.
pub fn default_sent_exchange(contest_name: &str, my_cq_zone: u32, my_itu_zone: u32) -> String {
    let name = contest_name.to_ascii_uppercase();
    if name.contains("CQ WORLD WIDE") || name.contains("CQ WW") {
        if my_cq_zone > 0 {
            return my_cq_zone.to_string();
        }
    } else if name.contains("IARU") && my_itu_zone > 0 {
        return my_itu_zone.to_string();
    }
    "#".to_string()
}

impl ContestSession {
    /// Nowa sesja rozpoczęta w chwili `now`.
    pub fn start(contest_name: &str, sent_exchange: String, now: i64) -> Self {
        Self {
            contest_name: contest_name.to_string(),
            contest_id: cabrillo_id_for(contest_name),
            started_at: now,
            sent_exchange,
            ..Self::default()
        }
    }

    pub fn is_active(&self) -> bool {
        self.ended_at.is_none() && !self.contest_id.is_empty()
    }

    /// Czy QSO należy do tej sesji (identyfikator zawodów i czas w oknie sesji).
    pub fn contains(&self, qso: &QsoRecord) -> bool {
        if self.contest_id.is_empty()
            || !qso
                .contest_id
                .as_deref()
                .is_some_and(|id| id.trim().eq_ignore_ascii_case(&self.contest_id))
        {
            return false;
        }
        let Some(at) = utc_epoch_seconds(&qso.qso_date, &qso.time_on) else {
            return false;
        };
        // Czas QSO z formularza bywa zaokrąglony do minuty — minuta tolerancji na starcie.
        at >= self.started_at - 60 && self.ended_at.is_none_or(|end| at <= end)
    }

    /// QSO sesji z dziennika (dowolna kolejność wejściowa, wynik w tej samej kolejności).
    pub fn qsos(&self, journal: &[QsoRecord]) -> Vec<QsoRecord> {
        journal
            .iter()
            .filter(|q| self.contains(q))
            .cloned()
            .collect()
    }

    /// Oznacza QSO zapisywane w trakcie sesji: identyfikator zawodów i wymiana nadawana
    /// z numerem kolejnym `serial` (jeśli wymiana go zawiera).
    pub fn tag(&self, qso: &mut QsoRecord, serial: u32) {
        if !self.is_active() {
            return;
        }
        qso.contest_id = Some(self.contest_id.clone());
        let sent = self.sent_exchange.trim();
        if sent.contains('#') {
            qso.stx = Some(serial);
            qso.stx_string = Some(sent.replace('#', &format!("{serial:03}")));
        } else if !sent.is_empty() {
            qso.stx_string = Some(sent.to_string());
        }
    }

    /// Czy wymiana nadawana zawiera numer kolejny (po każdym QSO rośnie licznik).
    pub fn uses_serial(&self) -> bool {
        self.sent_exchange.contains('#')
    }
}

/// Co robi Enter w zawodach przy włączonym ESM (Enter Sends Message), zależnie od trybu
/// pracy i tego, co jest już wpisane. Makra: F1 = CQ, F2 = wymiana, F3 = TU, F4 = mój znak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EsmStep {
    /// Nic do zrobienia (S&P bez znaku).
    Nothing,
    /// Nadaj makro o podanym indeksie i przejdź do pola wymiany.
    SendThenExchange(usize),
    /// Nadaj makro i zapisz QSO.
    SendThenLog(usize),
    /// Nadaj makro, pole znaku zostaje aktywne.
    Send(usize),
}

pub const MACRO_CQ: usize = 0;
pub const MACRO_EXCHANGE: usize = 1;
pub const MACRO_TU: usize = 2;
pub const MACRO_MY_CALL: usize = 3;

pub fn esm_step(mode: OperatingMode, has_call: bool, has_exchange: bool) -> EsmStep {
    match (mode, has_call, has_exchange) {
        (OperatingMode::Run, false, _) => EsmStep::Send(MACRO_CQ),
        (OperatingMode::Run, true, false) => EsmStep::SendThenExchange(MACRO_EXCHANGE),
        (OperatingMode::Run, true, true) => EsmStep::SendThenLog(MACRO_TU),
        (OperatingMode::SearchAndPounce, false, _) => EsmStep::Nothing,
        (OperatingMode::SearchAndPounce, true, false) => EsmStep::SendThenExchange(MACRO_MY_CALL),
        (OperatingMode::SearchAndPounce, true, true) => EsmStep::SendThenLog(MACRO_EXCHANGE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esm_follows_the_run_and_search_and_pounce_sequences() {
        use OperatingMode::{Run, SearchAndPounce};
        assert_eq!(esm_step(Run, false, false), EsmStep::Send(MACRO_CQ));
        assert_eq!(
            esm_step(Run, true, false),
            EsmStep::SendThenExchange(MACRO_EXCHANGE)
        );
        assert_eq!(esm_step(Run, true, true), EsmStep::SendThenLog(MACRO_TU));
        assert_eq!(esm_step(SearchAndPounce, false, false), EsmStep::Nothing);
        assert_eq!(
            esm_step(SearchAndPounce, true, false),
            EsmStep::SendThenExchange(MACRO_MY_CALL)
        );
        assert_eq!(
            esm_step(SearchAndPounce, true, true),
            EsmStep::SendThenLog(MACRO_EXCHANGE)
        );
    }

    fn qso(call: &str, date: &str, time: &str, contest: Option<&str>) -> QsoRecord {
        let mut q = QsoRecord::new(call, "20m", "CW");
        q.qso_date = date.to_string();
        q.time_on = time.to_string();
        q.contest_id = contest.map(str::to_string);
        q
    }

    #[test]
    fn known_contests_get_their_cabrillo_ids_and_others_are_sanitised() {
        assert_eq!(cabrillo_id_for("SP DX Contest"), "SP-DX-CONTEST");
        assert_eq!(cabrillo_id_for("CQ World Wide DX Contest (CW)"), "CQ-WW-CW");
        assert_eq!(cabrillo_id_for("IARU HF Championship"), "IARU-HF");
        assert_eq!(
            cabrillo_id_for("  Moje zawody / klubowe #3 "),
            "MOJE-ZAWODY-KLUBOWE-3"
        );
        assert!(sanitize_contest_id(&"A".repeat(50)).len() <= 32);
        assert_eq!(sanitize_contest_id("--x--"), "X");
    }

    #[test]
    fn only_tagged_qsos_inside_the_session_window_belong_to_it() {
        let start = utc_epoch_seconds("20261121", "120000").unwrap();
        let end = utc_epoch_seconds("20261122", "120000").unwrap();
        let mut session = ContestSession::start("SP DX Contest", "#".into(), start);
        session.ended_at = Some(end);
        let journal = vec![
            qso("A1", "20261121", "1300", Some("SP-DX-CONTEST")),
            qso("A2", "20251121", "1300", Some("SP-DX-CONTEST")), // rok wcześniej
            qso("A3", "20261121", "1300", Some("CQ-WW-CW")),      // inne zawody
            qso("A4", "20261121", "1300", None),                  // zwykłe QSO
            qso("A5", "20261122", "1201", Some("sp-dx-contest")), // po końcu sesji
            qso("A6", "20261121", "1159", Some("SP-DX-CONTEST")), // minuta tolerancji
        ];
        let calls: Vec<String> = session
            .qsos(&journal)
            .into_iter()
            .map(|q| q.callsign)
            .collect();
        assert_eq!(calls, vec!["A1", "A6"]);
    }

    #[test]
    fn tagging_writes_the_contest_id_and_the_sent_exchange() {
        let session = ContestSession::start("CQ WPX Contest (CW)", "#".into(), 0);
        let mut q = QsoRecord::new("K1ABC", "20m", "CW");
        session.tag(&mut q, 7);
        assert_eq!(q.contest_id.as_deref(), Some("CQ-WPX-CW"));
        assert_eq!(q.stx, Some(7));
        assert_eq!(q.stx_string.as_deref(), Some("007"));

        let zone = ContestSession::start("CQ World Wide DX Contest (CW)", "15".into(), 0);
        let mut q = QsoRecord::new("K1ABC", "20m", "CW");
        zone.tag(&mut q, 7);
        assert_eq!(q.stx, None, "CQ WW sends the zone, not a serial");
        assert_eq!(q.stx_string.as_deref(), Some("15"));
        assert!(!zone.uses_serial());

        let mut ended = session.clone();
        ended.ended_at = Some(1);
        let mut q = QsoRecord::new("K1ABC", "20m", "CW");
        ended.tag(&mut q, 1);
        assert_eq!(
            q.contest_id, None,
            "a finished session does not tag new QSOs"
        );
    }

    #[test]
    fn default_sent_exchange_depends_on_the_contest() {
        assert_eq!(
            default_sent_exchange("CQ World Wide DX Contest (CW)", 15, 28),
            "15"
        );
        assert_eq!(default_sent_exchange("IARU HF Championship", 15, 28), "28");
        assert_eq!(default_sent_exchange("CQ WPX Contest (CW)", 15, 28), "#");
        assert_eq!(default_sent_exchange("CQ WW SSB", 0, 0), "#");
    }
}
