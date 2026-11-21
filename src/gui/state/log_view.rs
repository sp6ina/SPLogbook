// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use crate::gui::drill_filter::DrillFilter;
use crate::gui::pagination::{PaginationInput, PaginationWindow, paginate};
use std::collections::{HashMap, HashSet, VecDeque};

/// Atomowa akcja w historii operacji Undo/Redo (DATA-004)
/// Rozmiary strony dziennika do wyboru; 0 = wszystkie wiersze.
pub const LOG_PAGE_SIZES: [usize; 4] = [25, 50, 100, 0];

#[derive(Debug, Clone, PartialEq)]
pub enum UndoAction {
    DeleteSingle {
        journal_id: String,
        qso: Box<QsoRecord>,
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

/// Bezpieczne okno bieżącej strony: znormalizowany zakres i skopiowane indeksy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSelection {
    pub window: PaginationWindow,
    pub indices: Vec<usize>,
    /// `true`, gdy uszkodzony cache został jednorazowo odbudowany.
    pub repaired: bool,
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
    /// `true` while `recent_qsos` holds an advanced-filter result instead of the whole journal.
    pub buffer_is_filtered: bool,
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
            buffer_is_filtered: false,
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

    /// Replaces the buffer with journal rows in `ORDER_CHRONO_DESC` order and numbers them.
    pub fn set_journal_rows(&mut self, rows: Vec<QsoRecord>) {
        self.recent_qsos = rows;
        self.buffer_is_filtered = false;
        self.renumber();
        self.bump_revision();
    }

    /// Replaces the buffer with an advanced-filter result; journal numbers are kept.
    pub fn set_filtered_rows(&mut self, rows: Vec<QsoRecord>) {
        self.recent_qsos = rows;
        self.buffer_is_filtered = true;
        self.bump_revision();
    }

    /// Chronological numbers derived from the buffer order: the oldest QSO is number 1.
    fn renumber(&mut self) {
        let total = self.recent_qsos.len();
        self.qso_numbers.clear();
        self.qso_numbers.reserve(total);
        for (index, qso) in self.recent_qsos.iter().enumerate() {
            if let Some(id) = qso.id {
                self.qso_numbers.insert(id, total - index);
            }
        }
    }

    /// Inserts or replaces one stored QSO at its chronological position without reloading the
    /// journal. Rows without an id are ignored.
    pub fn upsert_record(&mut self, qso: QsoRecord) {
        let Some(id) = qso.id else {
            return;
        };
        if let Some(existing) = self.recent_qsos.iter().position(|q| q.id == Some(id)) {
            self.recent_qsos.remove(existing);
        }
        let key = crate::core::database::schema::chrono_key(&qso);
        let position = self
            .recent_qsos
            .partition_point(|q| crate::core::database::schema::chrono_key(q) > key);
        self.recent_qsos.insert(position, qso);
        if position == 0 && !self.qso_numbers.contains_key(&id) {
            self.qso_numbers.insert(id, self.recent_qsos.len());
        } else {
            self.renumber();
        }
        self.bump_revision();
    }

    /// Removes the given QSOs from the buffer and the selection.
    pub fn remove_records(&mut self, ids: &[i64]) {
        let ids: HashSet<i64> = ids.iter().copied().collect();
        self.recent_qsos
            .retain(|q| q.id.is_none_or(|id| !ids.contains(&id)));
        self.selected_qso_ids.retain(|id| !ids.contains(id));
        self.renumber();
        self.bump_revision();
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
                    let Some(qso) = self.recent_qsos.get(i) else {
                        return false;
                    };
                    if let Some(ref d) = drill
                        && !d.matches(qso)
                    {
                        return false;
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
                let (Some(qa), Some(qb)) = (self.recent_qsos.get(a), self.recent_qsos.get(b))
                else {
                    // Nieosiągalne: `a` i `b` pochodzą z `0..recent_qsos.len()`.
                    // Deterministyczny porządek zamiast paniki.
                    return a.cmp(&b);
                };
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

    /// Normalizuje `page` względem bieżącego wyniku i zwraca okno paginacji.
    ///
    /// To jedyne miejsce, które zapisuje `self.page = window.normalized_page`.
    pub fn pagination_window(&mut self, total_items: usize) -> PaginationWindow {
        let window = paginate(PaginationInput {
            requested_page: self.page,
            page_size: self.page_size,
            total_items,
        });
        self.page = window.normalized_page;
        window
    }

    /// Buduje bezpieczną listę indeksów bieżącej strony.
    ///
    /// Cache jest budowany z `recent_qsos`, więc indeksy powinny zawsze być w
    /// granicach. Gdyby jednak stan był niespójny (np. po przyszłej zmianie
    /// ścieżki danych), cache jest jednorazowo odbudowywany, a dopiero trwałe
    /// naruszenie inwariantu daje kontrolowany pusty widok i wpis w logu.
    pub fn select_page_indices(&mut self, log_identity: &str, profile_id: &str) -> PageSelection {
        self.ensure_sorted_indices_cached(log_identity, profile_id);
        let mut window = self.pagination_window(self.cache.sorted_indices.len());
        let mut repaired = false;
        let indices = loop {
            let candidate = window.slice_of(&self.cache.sorted_indices);
            let is_valid = candidate
                .is_some_and(|slice| slice.iter().all(|&index| index < self.recent_qsos.len()));
            if is_valid {
                break candidate.map(<[usize]>::to_vec).unwrap_or_default();
            }
            if repaired {
                log::error!(
                    "logbook pagination invariant violated after cache repair (window {}..{} over {} cached indices)",
                    window.start,
                    window.end,
                    self.cache.sorted_indices.len()
                );
                break Vec::new();
            }
            repaired = true;
            self.invalidate_cache();
            self.ensure_sorted_indices_cached(log_identity, profile_id);
            window = self.pagination_window(self.cache.sorted_indices.len());
        };
        PageSelection {
            window,
            indices,
            repaired,
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
        assert_eq!(state.recent_qsos, [] as [QsoRecord; 0]);
        assert_eq!(state.qso_numbers.len(), 0);
        assert_eq!(state.search_query, "");
        assert!(state.drill_filter.is_none());
        assert!(state.editing_qso.is_none());
        assert_eq!(state.sort_column, 0);
        assert!(!state.sort_asc);
        assert_eq!(state.page, 0);
        assert_eq!(state.page_size, 50);
        assert_eq!(state.selected_qso_ids, [] as [i64; 0]);
        assert!(!state.confirm_bulk_delete);
        assert!(!state.show_statistics_window);
        assert_eq!(state.stats_tab, 0);
        assert_eq!(state.undo_stack.len(), 0);
        assert_eq!(state.redo_stack.len(), 0);
        assert_eq!(state.lotw_users_cache.len(), 0);
        assert_eq!(state.log_revision, 0);
        assert_eq!(state.cache.sorted_indices, [] as [usize; 0]);
        assert_eq!(state.cache.worked_callsigns.len(), 0);
    }

    fn synthetic_rows(count: usize) -> Vec<QsoRecord> {
        (0..count)
            .map(|index| {
                let mut qso = QsoRecord::new(format!("TST{index:04}"), "20m", "CW");
                qso.id = Some(index as i64 + 1);
                qso
            })
            .collect()
    }

    #[test]
    fn pagination_window_normalizes_stale_page() {
        let mut state = LogViewState::default();
        state.page = 9;
        state.page_size = 50;
        let window = state.pagination_window(10);
        assert_eq!(state.page, 0);
        assert_eq!(window.page_count, 1);
        assert_eq!((window.start, window.end), (0, 10));
        assert!(window.was_clamped);
    }

    #[test]
    fn pagination_window_empty_result_resets_page() {
        let mut state = LogViewState::default();
        state.page = 4;
        let window = state.pagination_window(0);
        assert_eq!(state.page, 0);
        assert_eq!((window.page_count, window.start, window.end), (0, 0, 0));
    }

    #[test]
    fn select_page_indices_returns_bounded_indices() {
        let mut state = LogViewState {
            recent_qsos: synthetic_rows(120),
            page: 2,
            page_size: 50,
            ..Default::default()
        };
        let selection = state.select_page_indices("journal", "profile");
        assert!(!selection.repaired);
        assert_eq!(selection.window.normalized_page, 2);
        assert_eq!(selection.window.page_count, 3);
        assert_eq!(selection.indices.len(), 20);
        assert!(selection.indices.iter().all(|&index| index < 120));
    }

    #[test]
    fn select_page_indices_repairs_an_out_of_range_cache() {
        let mut state = LogViewState {
            recent_qsos: synthetic_rows(120),
            page: 1,
            page_size: 50,
            ..Default::default()
        };
        let _ = state.select_page_indices("journal", "profile");
        // Symulacja naruszenia inwariantu: cache wskazuje poza bufor.
        state.cache.sorted_indices = vec![999, 1000];
        let selection = state.select_page_indices("journal", "profile");
        assert!(selection.repaired);
        assert_eq!(selection.indices.len(), 50);
        assert!(selection.indices.iter().all(|&index| index < 120));
        assert!(selection.window.start <= selection.window.end);
    }

    fn dated(id: i64, date: &str, time: &str) -> QsoRecord {
        let mut qso = QsoRecord::new(format!("SP{id}AA"), "20m", "CW");
        qso.id = Some(id);
        qso.qso_date = date.to_string();
        qso.time_on = time.to_string();
        qso
    }

    fn ids(state: &LogViewState) -> Vec<i64> {
        state.recent_qsos.iter().filter_map(|q| q.id).collect()
    }

    fn assert_numbers_follow_order(state: &LogViewState) {
        let total = state.recent_qsos.len();
        assert_eq!(state.qso_numbers.len(), total);
        for (index, qso) in state.recent_qsos.iter().enumerate() {
            let number = qso.id.and_then(|id| state.qso_numbers.get(&id).copied());
            assert_eq!(number, Some(total - index));
        }
    }

    #[test]
    fn upsert_record_keeps_chronological_order_and_numbers() {
        let mut state = LogViewState::default();
        state.set_journal_rows(vec![
            dated(3, "20261003", "1200"),
            dated(2, "20261002", "1200"),
            dated(1, "20261001", "1200"),
        ]);
        let revision = state.log_revision;

        state.upsert_record(dated(4, "20261004", "0000"));
        assert_eq!(ids(&state), [4, 3, 2, 1]);
        state.upsert_record(dated(5, "2026-10-02", "12:30"));
        assert_eq!(ids(&state), [4, 3, 5, 2, 1]);
        assert_numbers_follow_order(&state);
        assert!(state.log_revision > revision);

        // An edited row moves to its new position instead of being duplicated.
        state.upsert_record(dated(2, "20261005", "0100"));
        assert_eq!(ids(&state), [2, 4, 3, 5, 1]);
        assert_numbers_follow_order(&state);

        state.upsert_record(QsoRecord::new("NOID", "20m", "CW"));
        assert_eq!(state.recent_qsos.len(), 5, "rows without an id are ignored");
    }

    #[test]
    fn remove_records_renumbers_and_clears_the_selection() {
        let mut state = LogViewState::default();
        state.set_journal_rows(vec![
            dated(3, "20261003", "1200"),
            dated(2, "20261002", "1200"),
            dated(1, "20261001", "1200"),
        ]);
        state.selected_qso_ids = vec![2, 3];
        state.remove_records(&[2]);
        assert_eq!(ids(&state), [3, 1]);
        assert_eq!(state.selected_qso_ids, [3]);
        assert_numbers_follow_order(&state);
    }

    #[test]
    fn filtered_rows_are_flagged_and_keep_journal_numbers() {
        let mut state = LogViewState::default();
        state.set_journal_rows(vec![
            dated(2, "20261002", "1200"),
            dated(1, "20261001", "1200"),
        ]);
        state.set_filtered_rows(vec![dated(1, "20261001", "1200")]);
        assert!(state.buffer_is_filtered);
        assert_eq!(state.qso_numbers.get(&2), Some(&2));
        state.set_journal_rows(vec![dated(1, "20261001", "1200")]);
        assert!(!state.buffer_is_filtered);
    }

    #[test]
    fn select_page_indices_empty_dataset_is_safe() {
        let mut state = LogViewState {
            page: 7,
            ..Default::default()
        };
        let selection = state.select_page_indices("journal", "profile");
        assert_eq!(selection.indices, [] as [usize; 0]);
        assert_eq!((selection.window.start, selection.window.end), (0, 0));
        assert_eq!(state.page, 0);
    }
}
