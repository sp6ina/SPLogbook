// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use crate::gui::drill_filter::DrillFilter;
use std::collections::{HashMap, HashSet, VecDeque};

/// Atomowa akcja w historii operacji Undo/Redo (DATA-004)
#[derive(Debug, Clone, PartialEq)]
pub enum UndoAction {
    DeleteSingle {
        journal_id: String,
        qso: QsoRecord,
    },
    DeleteBatch {
        journal_id: String,
        qsos: Vec<QsoRecord>,
    },
}

/// Klucz bufora widoku tabeli łączności (PERF-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogViewCacheKey {
    pub profile_id: String,
    pub log_identity: String,
    pub log_revision: u64,
    pub search_query: String,
    pub drill_filter: Option<DrillFilter>,
    pub sort_column: u8,
    pub sort_asc: bool,
}

/// Pamięć podręczna posortowanych i przefiltrowanych indeksów oraz unikalnych znaków (PERF-002).
#[derive(Debug, Clone, Default)]
pub struct LogViewCache {
    pub key: Option<LogViewCacheKey>,
    pub sorted_indices: Vec<usize>,
    pub worked_callsigns_revision: u64,
    pub worked_callsigns: HashSet<String>,
}

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
    pub undo_stack: VecDeque<UndoAction>,
    pub redo_stack: VecDeque<UndoAction>,
    pub lotw_users_cache: HashMap<String, bool>,
    pub log_revision: u64,
    pub cache: LogViewCache,
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
            log_revision: 0,
            cache: LogViewCache::default(),
        }
    }
}

impl LogViewState {
    pub fn invalidate_cache(&mut self) {
        self.cache.key = None;
        self.cache.sorted_indices.clear();
        self.cache.worked_callsigns.clear();
        self.cache.worked_callsigns_revision = 0;
        self.log_revision = self.log_revision.wrapping_add(1);
    }

    pub fn bump_revision(&mut self) {
        self.log_revision = self.log_revision.wrapping_add(1);
    }

    pub fn is_cache_valid(&self, log_identity: &str, profile_id: &str) -> bool {
        if let Some(ref k) = self.cache.key {
            k.log_identity == log_identity
                && k.profile_id == profile_id
                && k.log_revision == self.log_revision
                && k.search_query == self.search_query
                && k.drill_filter == self.drill_filter
                && k.sort_column == self.sort_column
                && k.sort_asc == self.sort_asc
        } else {
            false
        }
    }

    pub fn ensure_sorted_indices_cached(&mut self, log_identity: &str, profile_id: &str) {
        let current_key = LogViewCacheKey {
            profile_id: profile_id.to_string(),
            log_identity: log_identity.to_string(),
            log_revision: self.log_revision,
            search_query: self.search_query.clone(),
            drill_filter: self.drill_filter.clone(),
            sort_column: self.sort_column,
            sort_asc: self.sort_asc,
        };

        if self.cache.key.as_ref() != Some(&current_key) {
            let query = self.search_query.trim().to_uppercase();
            let drill = self.drill_filter.clone();
            let mut sorted: Vec<usize> = (0..self.recent_qsos.len())
                .filter(|&i| {
                    let qso = &self.recent_qsos[i];
                    if let Some(ref d) = drill {
                        if !d.matches(qso) {
                            return false;
                        }
                    }
                    if query.is_empty() {
                        return true;
                    }
                    crate::gui::logbook_table::contains_case_insensitive(&qso.callsign, &query)
                        || crate::gui::logbook_table::contains_case_insensitive(&qso.band, &query)
                        || crate::gui::logbook_table::contains_case_insensitive(&qso.mode, &query)
                        || crate::gui::logbook_table::contains_case_insensitive(
                            qso.country.as_deref().unwrap_or(""),
                            &query,
                        )
                        || crate::gui::logbook_table::contains_case_insensitive(
                            qso.name.as_deref().unwrap_or(""),
                            &query,
                        )
                        || crate::gui::logbook_table::contains_case_insensitive(
                            qso.qth.as_deref().unwrap_or(""),
                            &query,
                        )
                        || crate::gui::logbook_table::contains_case_insensitive(
                            qso.gridsquare.as_deref().unwrap_or(""),
                            &query,
                        )
                        || crate::gui::logbook_table::contains_case_insensitive(
                            qso.comment.as_deref().unwrap_or(""),
                            &query,
                        )
                })
                .collect();

            let sc = self.sort_column;
            let sa = self.sort_asc;
            sorted.sort_by(|&a, &b| {
                let qa = &self.recent_qsos[a];
                let qb = &self.recent_qsos[b];
                let ord = match sc {
                    1 => qa.callsign.cmp(&qb.callsign),
                    2 => qa.band.cmp(&qb.band),
                    3 => qa.mode.cmp(&qb.mode),
                    4 => qa
                        .country
                        .as_deref()
                        .unwrap_or("")
                        .cmp(qb.country.as_deref().unwrap_or("")),
                    _ => {
                        let da =
                            crate::gui::logbook_table::normalize_digits_stack::<8>(&qa.qso_date);
                        let db =
                            crate::gui::logbook_table::normalize_digits_stack::<8>(&qb.qso_date);
                        da.cmp(&db)
                            .then_with(|| {
                                let ta = crate::gui::logbook_table::normalize_digits_stack::<6>(
                                    &qa.time_on,
                                );
                                let tb = crate::gui::logbook_table::normalize_digits_stack::<6>(
                                    &qb.time_on,
                                );
                                ta.cmp(&tb)
                            })
                            .then_with(|| qa.id.cmp(&qb.id))
                    }
                };
                if sa { ord } else { ord.reverse() }
            });

            self.cache.sorted_indices = sorted;
            self.cache.key = Some(current_key);
        }
    }

    pub fn get_or_compute_sorted_indices<'a>(
        &'a mut self,
        log_identity: &str,
        profile_id: &str,
    ) -> &'a [usize] {
        self.ensure_sorted_indices_cached(log_identity, profile_id);
        &self.cache.sorted_indices
    }

    pub fn get_or_compute_worked_callsigns(&mut self) -> &HashSet<String> {
        if self.cache.worked_callsigns_revision != self.log_revision
            || (self.cache.worked_callsigns.is_empty() && !self.recent_qsos.is_empty())
        {
            let mut set = HashSet::with_capacity(self.recent_qsos.len());
            for q in &self.recent_qsos {
                set.insert(q.callsign.to_ascii_uppercase());
            }
            self.cache.worked_callsigns = set;
            self.cache.worked_callsigns_revision = self.log_revision;
        }
        &self.cache.worked_callsigns
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
        assert_eq!(state.log_revision, 0);
        assert!(state.cache.sorted_indices.is_empty());
        assert!(state.cache.worked_callsigns.is_empty());
    }
}
