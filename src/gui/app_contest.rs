// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Sesja zawodów w aplikacji (STAGE-V140-S7): start i koniec, oznaczanie QSO,
//! punktacja ograniczona do sesji, ESM i makra F1..F8 w oknie zawodów.

use super::SpLogApp;
use crate::core::contest_session::{
    ContestSession, EsmStep, OperatingMode, default_sent_exchange, esm_step,
};
use crate::core::contest_stats::Scoreboard;
use crate::core::i18n::{Msg, key};
use crate::core::qso::QsoRecord;

fn now_utc_secs() -> i64 {
    jiff::Timestamp::now().as_second()
}

impl SpLogApp {
    /// Aktywna (niezakończona) sesja zawodów.
    pub fn active_contest_session(&self) -> Option<&ContestSession> {
        self.contest.session.as_ref().filter(|s| s.is_active())
    }

    /// Rozpoczyna sesję wybranych zawodów. Numeracja zaczyna się od 001.
    pub fn start_contest_session(&mut self) {
        let sent = default_sent_exchange(
            &self.contest.name,
            self.config.station.cq_zone,
            self.config.station.itu_zone,
        );
        let mut session = ContestSession::start(&self.contest.name, sent, now_utc_secs());
        if let Some(previous) = &self.contest.session {
            // Kategorie zgłoszenia zwykle się nie zmieniają między zawodami.
            session.categories = previous.categories.clone();
            session.mode = previous.mode;
            session.esm = previous.esm;
        }
        self.contest.stx = 1;
        let name = session.contest_name.clone();
        self.contest.session = Some(session);
        self.contest.cache.key = None;
        self.say(Msg::new(key::CONTEST_SESSION_STARTED).arg("name", name));
        self.save_station_config();
    }

    /// Kończy sesję; jej QSO dalej liczą się do wyniku i eksportu.
    pub fn end_contest_session(&mut self) {
        if let Some(session) = self.contest.session.as_mut()
            && session.ended_at.is_none()
        {
            session.ended_at = Some(now_utc_secs());
            self.contest.cache.key = None;
            self.save_station_config();
        }
    }

    /// Wznawia zakończoną sesję (np. po przerwie w zawodach wielodniowych).
    pub fn resume_contest_session(&mut self) {
        if let Some(session) = self.contest.session.as_mut() {
            session.ended_at = None;
            self.contest.cache.key = None;
            self.save_station_config();
        }
    }

    /// Oznacza zapisywane QSO identyfikatorem zawodów i wymianą nadaną, jeśli trwa sesja.
    /// Zwraca `true`, gdy QSO zużywa numer kolejny.
    pub fn tag_contest_qso(&self, qso: &mut QsoRecord) -> bool {
        match self.active_contest_session() {
            Some(session) => {
                session.tag(qso, self.contest.stx);
                session.uses_serial()
            }
            None => false,
        }
    }

    /// Wynik sesji. IARU HF liczy się według strefy ITU i kontynentu operatora, z
    /// kontynentami korespondentów z bazy prefiksów; pozostałe zawody — według reguł.
    pub fn contest_score(&self, qsos: &[QsoRecord], my_dxcc: u32, my_cqzone: u8) -> Scoreboard {
        if self
            .contest
            .name
            .eq_ignore_ascii_case("IARU HF Championship")
        {
            let matcher = &self.services.prefix_matcher;
            let my_continent = matcher
                .lookup(&self.config.station.callsign)
                .map(|info| info.continent)
                .unwrap_or_default();
            let (points, mults, total) = crate::core::contest_rules::score_iaru_hf(
                qsos,
                self.config.station.itu_zone,
                &my_continent,
                |q| matcher.lookup(&q.callsign).map(|info| info.continent),
            );
            return Scoreboard {
                qsos: qsos.len() as u32,
                points,
                mults,
                total,
            };
        }
        crate::core::contest_stats::compute_scoreboard(
            &self.contest.name,
            &self.config.custom_contests,
            qsos,
            my_dxcc,
            my_cqzone,
        )
    }

    /// Makro F1..F8 z okna zawodów.
    pub fn contest_macro(&mut self, index: usize) {
        if let Some(text) = self.cw.macros.get(index).cloned() {
            self.transmit_cw_macro(&text);
        }
    }

    /// Enter w oknie zawodów. Z ESM i podłączonym kluczem nadaje makro właściwe dla etapu
    /// łączności; zwraca `true`, gdy QSO ma zostać zapisane.
    pub fn contest_enter(&mut self) -> bool {
        let Some(session) = self.active_contest_session() else {
            return true;
        };
        if !session.esm || !self.gui.dialogs.cw_terminal_dialog.is_connected() {
            return true;
        }
        let mode = session.mode;
        let has_call = !self.entry.callsign.trim().is_empty();
        let has_exchange = !self.entry.exchange.trim().is_empty();
        match esm_step(mode, has_call, has_exchange) {
            EsmStep::Nothing => false,
            EsmStep::Send(index) => {
                self.contest_macro(index);
                false
            }
            EsmStep::SendThenExchange(index) => {
                self.contest_macro(index);
                self.contest.focus_exchange_requested = true;
                false
            }
            EsmStep::SendThenLog(index) => {
                self.contest_macro(index);
                true
            }
        }
    }

    /// Przełącza tryb pracy Run ↔ S&P.
    pub fn toggle_contest_mode(&mut self) {
        if let Some(session) = self.contest.session.as_mut() {
            session.mode = match session.mode {
                OperatingMode::Run => OperatingMode::SearchAndPounce,
                OperatingMode::SearchAndPounce => OperatingMode::Run,
            };
        }
    }
}
