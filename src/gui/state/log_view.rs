// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use crate::gui::drill_filter::DrillFilter;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct LogViewState {
    pub recent_qsos: Vec<QsoRecord>,
    pub qso_numbers: HashMap<i64, usize>,
    pub search_query: String,
    pub drill_filter: Option<DrillFilter>,
    pub editing_qso: Option<QsoRecord>,
    pub sort_column: u8,
    pub sort_asc: bool,
    pub page: usize,
    pub page_size: usize,
    pub selected_qso_ids: Vec<i64>,
    pub confirm_bulk_delete: bool,
    pub show_statistics_window: bool,
    pub stats_tab: usize,
    pub undo_stack: VecDeque<QsoRecord>,
    pub redo_stack: VecDeque<QsoRecord>,
    pub lotw_users_cache: HashMap<String, bool>,
}

impl Default for LogViewState {
    fn default() -> Self {
        Self {
            recent_qsos: Vec::new(),
            qso_numbers: HashMap::new(),
            search_query: String::new(),
            drill_filter: None,
            editing_qso: None,
            sort_column: 0,
            sort_asc: false,
            page: 0,
            page_size: 50,
            selected_qso_ids: Vec::new(),
            confirm_bulk_delete: false,
            show_statistics_window: false,
            stats_tab: 0,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            lotw_users_cache: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_initial_values() {
        let state = LogViewState::default();
        assert!(state.recent_qsos.is_empty());
        assert!(state.qso_numbers.is_empty());
        assert_eq!(state.search_query, "");
        assert!(state.drill_filter.is_none());
        assert!(state.editing_qso.is_none());
        assert_eq!(state.sort_column, 0);
        assert!(!state.sort_asc);
        assert_eq!(state.page, 0);
        assert_eq!(state.page_size, 50);
        assert!(state.selected_qso_ids.is_empty());
        assert!(!state.confirm_bulk_delete);
        assert!(!state.show_statistics_window);
        assert_eq!(state.stats_tab, 0);
        assert!(state.undo_stack.is_empty());
        assert!(state.redo_stack.is_empty());
        assert!(state.lotw_users_cache.is_empty());
    }
}
