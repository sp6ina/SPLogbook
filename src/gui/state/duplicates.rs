// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct DuplicatesState {
    pub show_window: bool,
    pub match_date: bool,
    pub groups: Vec<Vec<QsoRecord>>,
    pub selected_ids: HashSet<i64>,
    pub status: Option<String>,
}

impl Default for DuplicatesState {
    /// Domyślnie duplikat to ten sam znak, pasmo i emisja tego samego dnia; bez daty
    /// wyszukiwarka łączyłaby w grupy zwykłe łączności z różnych lat.
    fn default() -> Self {
        Self {
            show_window: false,
            match_date: true,
            groups: Vec::new(),
            selected_ids: HashSet::new(),
            status: None,
        }
    }
}

impl DuplicatesState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = DuplicatesState::default();
        assert!(!state.show_window);
        assert!(state.match_date);
        assert_eq!(state.groups.len(), 0);
        assert_eq!(state.selected_ids.len(), 0);
        assert!(state.status.is_none());

        let new_state = DuplicatesState::new();
        assert!(!new_state.show_window);
        assert!(new_state.match_date);
    }
}
