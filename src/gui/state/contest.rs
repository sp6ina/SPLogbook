// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::station::CustomContest;

/// Klucz bufora obliczeń zawodów (PERF-002 / F-21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContestCacheKey {
    pub profile_id: String,
    pub log_identity: String,
    pub log_revision: u64,
    pub journal_qso_count: usize,
    pub contest_name: String,
    pub custom_contests_len: usize,
    pub my_dxcc: u32,
    pub my_cqzone: u8,
    /// Identyfikator, początek i koniec sesji zawodów, z której liczony jest wynik.
    pub session: Option<(String, i64, Option<i64>)>,
}

/// Pamięć podręczna punktacji, mnożników i tempa zawodów (PERF-002).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ContestCache {
    pub key: Option<ContestCacheKey>,
    /// QSO bieżącej sesji (od najnowszego), podstawa wyniku, tempa i kontroli duplikatów.
    pub session_qsos: Vec<crate::core::qso::QsoRecord>,
    pub scoreboard: crate::core::contest_stats::Scoreboard,
    pub mult_matrix: crate::core::contest_stats::MultMatrix,
    pub band_breakdown: Vec<(String, usize)>,
    pub rate_cached_at_sec: i64,
    pub rate: crate::core::contest_stats::RateStats,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContestState {
    pub name: String,
    pub stx: u32,
    pub show_custom_editor: bool,
    pub custom_edit_idx: Option<usize>,
    pub custom_draft: CustomContest,
    pub cache: ContestCache,
    /// Skrót `Enter` (`AppAction::ContestSaveQso`) prosi okno Contest o zapis QSO.
    pub save_requested: bool,
    /// Bieżąca albo ostatnia sesja zawodów.
    pub session: Option<crate::core::contest_session::ContestSession>,
    /// ESM: po nadaniu makra kursor przechodzi do pola wymiany.
    pub focus_exchange_requested: bool,
}

impl Default for ContestState {
    fn default() -> Self {
        Self {
            name: "SP DX Contest".to_string(),
            stx: 1,
            show_custom_editor: false,
            custom_edit_idx: None,
            custom_draft: CustomContest::default(),
            cache: ContestCache::default(),
            save_requested: false,
            session: None,
            focus_exchange_requested: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = ContestState::default();
        assert_eq!(state.name, "SP DX Contest");
        assert_eq!(state.stx, 1);
        assert!(!state.show_custom_editor);
        assert!(state.custom_edit_idx.is_none());
        assert_eq!(state.custom_draft, CustomContest::default());
        assert!(state.cache.key.is_none());
    }
}
