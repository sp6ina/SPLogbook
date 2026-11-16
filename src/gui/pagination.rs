// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! P1-F16 — centralny, totalny model paginacji logbooka.
//!
//! Moduł jest czystą funkcją matematyczną: nie zależy od egui, I/O, czasu,
//! losowości ani od danych użytkownika. Jedno miejsce oblicza liczbę stron,
//! znormalizowany indeks strony i bezpieczny zakres `start..end`.
//!
//! Reguły (patrz `docs/DEVELOPMENT.md` §P1-F16):
//! - indeks strony jest liczony od zera;
//! - pusty wynik ma jawny stan `0` stron i zakres `0..0`;
//! - `page_size == 0` to istniejący tryb UI „wszystkie rekordy” (jedna strona logiczna);
//! - nieaktualny indeks strony jest normalizowany do ostatniej istniejącej strony;
//! - filtr rozszerzający zachowuje bieżącą stronę, jeśli nadal jest prawidłowa;
//! - mnożenie i dodawanie są kontrolowane (`checked_*`), bez panic i bez zawijania.

/// Tryb pracy okna paginacji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMode {
    /// Stronicowanie z jawnym rozmiarem strony (`page_size >= 1`).
    Paged,
    /// Istniejący tryb UI „wszystkie rekordy” (`page_size == 0`).
    AllRows,
}

/// Wejście modelu paginacji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationInput {
    /// Żądany indeks strony (0-based); może być nieaktualny lub `usize::MAX`.
    pub requested_page: usize,
    /// Rozmiar strony; `0` oznacza tryb „wszystkie rekordy”.
    pub page_size: usize,
    /// Liczba elementów w bieżącym (przefiltrowanym i posortowanym) wyniku.
    pub total_items: usize,
}

/// Wyjście modelu paginacji — zawsze prawidłowe okno `start..end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationWindow {
    /// Liczba elementów, względem której policzono okno.
    pub total_items: usize,
    /// Znormalizowany indeks strony (0-based, zawsze `< page_count` gdy `page_count > 0`).
    pub normalized_page: usize,
    /// Liczba stron; `0` wyłącznie dla pustego wyniku.
    pub page_count: usize,
    /// Początek zakresu (włącznie).
    pub start: usize,
    /// Koniec zakresu (wyłącznie).
    pub end: usize,
    /// Rozmiar strony użyty w obliczeniu; nigdy `0`.
    pub effective_page_size: usize,
    /// Tryb okna.
    pub mode: PageMode,
    /// `true`, gdy żądany indeks strony został skorygowany.
    pub was_clamped: bool,
    /// `true` wyłącznie wtedy, gdy defensywna gałąź `checked_mul` została użyta.
    pub arithmetic_guard_triggered: bool,
}

impl PaginationWindow {
    /// Czy okno jest puste (`start == end`).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Numer pierwszego wiersza widocznego w oknie, liczony od 1.
    #[must_use]
    pub const fn first_row_number(&self) -> Option<usize> {
        if self.is_empty() {
            None
        } else {
            self.start.checked_add(1)
        }
    }

    /// Numer bieżącej strony liczony od 1; `None` dla pustego wyniku.
    #[must_use]
    pub const fn one_based_page(&self) -> Option<usize> {
        if self.page_count == 0 {
            None
        } else {
            self.normalized_page.checked_add(1)
        }
    }

    /// Poprzednia strona, jeśli istnieje.
    #[must_use]
    pub const fn previous_page(&self) -> Option<usize> {
        if self.normalized_page == 0 {
            None
        } else {
            Some(self.normalized_page - 1)
        }
    }

    /// Następna strona, jeśli istnieje.
    #[must_use]
    pub const fn next_page(&self) -> Option<usize> {
        match self.normalized_page.checked_add(1) {
            Some(next) if next < self.page_count => Some(next),
            _ => None,
        }
    }

    /// Bezpieczny dostęp do zakresu: `items.get(start..end)`.
    ///
    /// Zgodnie z dokumentacją Rust 1.99.0 `slice::get` zwraca `Option` i nie
    /// panikuje dla zakresu poza granicami ani dla zakresu odwróconego.
    #[must_use]
    pub fn slice_of<'a, T>(&self, items: &'a [T]) -> Option<&'a [T]> {
        items.get(self.start..self.end)
    }
}

/// Oblicza okno paginacji. Funkcja jest totalna: każde wejście daje prawidłowe okno.
#[must_use]
pub fn paginate(input: PaginationInput) -> PaginationWindow {
    let mode = if input.page_size == 0 {
        PageMode::AllRows
    } else {
        PageMode::Paged
    };

    // Pusty wynik: jawny stan 0/0 i zakres 0..0.
    if input.total_items == 0 {
        return PaginationWindow {
            total_items: 0,
            normalized_page: 0,
            page_count: 0,
            start: 0,
            end: 0,
            effective_page_size: input.page_size.max(1),
            mode,
            was_clamped: input.requested_page != 0,
            arithmetic_guard_triggered: false,
        };
    }

    // Tryb „wszystkie rekordy”: jedna strona logiczna, bez dzielenia przez zero
    // i bez wywołania `div_ceil` z dzielnikiem 0.
    if mode == PageMode::AllRows {
        return PaginationWindow {
            total_items: input.total_items,
            normalized_page: 0,
            page_count: 1,
            start: 0,
            end: input.total_items,
            effective_page_size: input.total_items,
            mode,
            was_clamped: input.requested_page != 0,
            arithmetic_guard_triggered: false,
        };
    }

    // `page_size >= 1` i `total_items >= 1`, więc `div_ceil` nie dzieli przez zero
    // i `page_count >= 1` (odejmowanie poniżej jest bezpieczne).
    let page_count = input.total_items.div_ceil(input.page_size);
    let normalized_page = input.requested_page.min(page_count - 1);

    let (start, arithmetic_guard_triggered) = match normalized_page.checked_mul(input.page_size) {
        Some(start) => (start, false),
        // Gałąź nieosiągalna dla poprawnych typów (dowód w raporcie modelu):
        // `normalized_page * page_size < total_items <= usize::MAX`.
        // Fallback jest bezpieczny i jawnie sygnalizowany, nigdy panic ani wrap.
        None => (0, true),
    };
    let end = start
        .checked_add(input.page_size)
        .map_or(input.total_items, |candidate| {
            candidate.min(input.total_items)
        });

    PaginationWindow {
        total_items: input.total_items,
        normalized_page,
        page_count,
        start,
        end,
        effective_page_size: input.page_size,
        mode,
        was_clamped: normalized_page != input.requested_page,
        arithmetic_guard_triggered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(page: usize, page_size: usize, total: usize) -> PaginationWindow {
        paginate(PaginationInput {
            requested_page: page,
            page_size,
            total_items: total,
        })
    }

    fn assert_invariants(w: &PaginationWindow) {
        assert!(
            w.effective_page_size >= 1,
            "effective page size must never be zero"
        );
        assert!(w.start <= w.end, "start {} > end {}", w.start, w.end);
        assert!(
            w.end <= w.total_items,
            "end {} > total {}",
            w.end,
            w.total_items
        );
        assert!(w.start <= w.total_items);
        if w.page_count > 0 {
            assert!(w.normalized_page < w.page_count);
        } else {
            assert_eq!(w.normalized_page, 0);
            assert_eq!(w.start, 0);
            assert_eq!(w.end, 0);
        }
        if w.total_items == 0 {
            assert_eq!(w.page_count, 0);
            assert_eq!(w.start, 0);
            assert_eq!(w.end, 0);
        }
        if w.mode == PageMode::Paged && w.page_count > 0 {
            // `start / page_size == normalized_page` dla poprawnego okna; dzielenie
            // przez `effective_page_size >= 1` nie może panikować.
            assert_eq!(w.start / w.effective_page_size, w.normalized_page);
            assert_eq!(
                w.end,
                w.start
                    .checked_add(w.effective_page_size)
                    .map_or(w.total_items, |v| v.min(w.total_items))
            );
        }
    }

    #[test]
    fn empty_dataset_has_explicit_zero_window() {
        let w = window(0, 50, 0);
        assert_eq!(w.page_count, 0);
        assert_eq!(w.normalized_page, 0);
        assert_eq!((w.start, w.end), (0, 0));
        assert!(w.is_empty());
        assert!(!w.was_clamped);
        assert_eq!(w.one_based_page(), None);
        assert_eq!(w.first_row_number(), None);
        assert_eq!(w.previous_page(), None);
        assert_eq!(w.next_page(), None);
        assert_invariants(&w);
    }

    #[test]
    fn empty_dataset_with_stale_page_is_clamped() {
        let w = window(7, 50, 0);
        assert_eq!(w.page_count, 0);
        assert_eq!((w.start, w.end), (0, 0));
        assert!(w.was_clamped);
        assert_invariants(&w);
    }

    #[test]
    fn single_row_is_one_page() {
        let w = window(0, 50, 1);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (0, 1, 0, 1)
        );
        assert_eq!(w.first_row_number(), Some(1));
        assert_eq!(w.one_based_page(), Some(1));
        assert_invariants(&w);
    }

    #[test]
    fn stale_single_row_page_is_normalized() {
        let w = window(9, 50, 1);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (0, 1, 0, 1)
        );
        assert!(w.was_clamped);
        assert_invariants(&w);
    }

    #[test]
    fn exact_full_page_has_no_extra_page() {
        let w = window(0, 50, 50);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (0, 1, 0, 50)
        );
        assert_eq!(w.next_page(), None);
        assert_invariants(&w);
    }

    #[test]
    fn last_partial_page_is_complete_and_bounded() {
        let w = window(1, 50, 51);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (1, 2, 50, 51)
        );
        assert_eq!(w.previous_page(), Some(0));
        assert_eq!(w.next_page(), None);
        assert_invariants(&w);
    }

    #[test]
    fn two_pages_cover_the_result_exactly() {
        let total = 120;
        let ps = 50;
        let mut union = Vec::new();
        for page in 0..window(0, ps, total).page_count {
            let w = window(page, ps, total);
            let items: Vec<usize> = (0..total).collect();
            let slice = w.slice_of(&items).expect("window must be in bounds");
            union.extend_from_slice(slice);
        }
        assert_eq!(union, (0..total).collect::<Vec<_>>());
    }

    #[test]
    fn stale_page_normalizes_to_last_existing_page() {
        let w = window(2, 50, 30);
        assert_eq!(w.normalized_page, 0);
        assert_eq!(w.page_count, 1);
        assert_eq!((w.start, w.end), (0, 30));
        assert!(w.was_clamped);
        assert_invariants(&w);
    }

    #[test]
    fn filter_expansion_keeps_valid_page() {
        let before = window(1, 50, 80);
        let after = window(1, 50, 200);
        assert_eq!(before.normalized_page, 1);
        assert_eq!(after.normalized_page, 1);
        assert!(!after.was_clamped);
        assert_invariants(&after);
    }

    #[test]
    fn page_index_usize_max_is_normalized_without_overflow() {
        for &total in &[0usize, 1, 49, 50, 51, 120, usize::MAX - 1, usize::MAX] {
            let w = window(usize::MAX, 50, total);
            assert!(w.normalized_page < w.page_count.max(1));
            assert!(w.start <= w.end);
            assert!(w.end <= total);
            assert!(!w.arithmetic_guard_triggered);
            assert_invariants(&w);
        }
    }

    #[test]
    fn page_size_zero_means_all_rows() {
        let w = window(0, 0, 120);
        assert_eq!(w.mode, PageMode::AllRows);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (0, 1, 0, 120)
        );
        assert_eq!(w.effective_page_size, 120);
        assert_eq!(w.next_page(), None);
        assert_eq!(w.previous_page(), None);
        assert_invariants(&w);
    }

    #[test]
    fn page_size_zero_with_stale_page_is_clamped() {
        let w = window(5, 0, 120);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (0, 1, 0, 120)
        );
        assert!(w.was_clamped);
        assert_invariants(&w);
    }

    #[test]
    fn page_size_zero_empty_dataset_is_empty() {
        let w = window(0, 0, 0);
        assert_eq!(w.mode, PageMode::AllRows);
        assert_eq!(w.page_count, 0);
        assert_eq!((w.start, w.end), (0, 0));
        assert_eq!(w.effective_page_size, 1);
        assert_invariants(&w);
    }

    #[test]
    fn page_size_one_makes_every_row_a_page() {
        let w = window(2, 1, 5);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (2, 5, 2, 3)
        );
        assert_eq!(w.next_page(), Some(3));
        assert_invariants(&w);
    }

    #[test]
    fn page_size_usize_max_is_a_single_page() {
        let w = window(0, usize::MAX, 100);
        assert_eq!(
            (w.normalized_page, w.page_count, w.start, w.end),
            (0, 1, 0, 100)
        );
        assert_invariants(&w);

        let stale = window(usize::MAX, usize::MAX, 100);
        assert_eq!((stale.normalized_page, stale.page_count), (0, 1));
        assert!(stale.was_clamped);
        assert_invariants(&stale);
    }

    #[test]
    fn total_usize_max_is_computed_without_allocation() {
        for &ps in &[
            1usize,
            2,
            3,
            25,
            50,
            100,
            1_000_003,
            usize::MAX - 1,
            usize::MAX,
        ] {
            let w = window(usize::MAX - 1, ps, usize::MAX);
            assert!(w.page_count >= 1);
            assert!(w.normalized_page < w.page_count);
            assert!(w.start <= w.end);
            assert_eq!(w.end, usize::MAX);
            assert!(!w.arithmetic_guard_triggered);
            assert_invariants(&w);
        }
    }

    #[test]
    fn multiplication_overflow_is_impossible_and_guarded() {
        // `normalized_page * page_size < total_items`, więc dla wszystkich wejść
        // reprezentowalnych `checked_mul` zwraca `Some`; flaga pozostaje false.
        for total in 0..=300usize {
            for ps in 0..=64usize {
                for page in [0usize, 1, 2, 9, usize::MAX - 1, usize::MAX] {
                    let w = window(page, ps, total);
                    assert!(
                        !w.arithmetic_guard_triggered,
                        "unexpected arithmetic guard for total={total} ps={ps} page={page}"
                    );
                    assert_invariants(&w);
                }
            }
        }
    }

    #[test]
    fn addition_overflow_falls_back_to_total() {
        // start = page_size, start + page_size przepełnia usize.
        let ps = usize::MAX / 2 + 1;
        let w = window(1, ps, usize::MAX);
        assert_eq!(w.page_count, 2);
        assert_eq!(w.normalized_page, 1);
        assert_eq!(w.start, ps);
        assert_eq!(w.end, usize::MAX);
        assert_invariants(&w);
    }

    #[test]
    fn slice_of_returns_none_for_inconsistent_window() {
        let items: Vec<usize> = (0..10).collect();
        let mut w = window(0, 5, 10);
        assert_eq!(w.slice_of(&items), Some(&items[0..5][..]));

        w.start = 7;
        w.end = 3;
        assert_eq!(w.slice_of(&items), None, "reversed range must be rejected");

        w.start = 0;
        w.end = 11;
        assert_eq!(w.slice_of(&items), None, "end above len must be rejected");

        w.start = 11;
        w.end = 11;
        assert_eq!(w.slice_of(&items), None, "start above len must be rejected");

        w.start = 10;
        w.end = 10;
        assert_eq!(w.slice_of(&items), Some(&items[10..10][..]));
    }

    #[test]
    fn helper_navigation_is_total() {
        let w = window(1, 50, 120);
        assert_eq!(w.first_row_number(), Some(51));
        assert_eq!(w.one_based_page(), Some(2));
        assert_eq!(w.previous_page(), Some(0));
        assert_eq!(w.next_page(), Some(2));

        let first = window(0, 50, 120);
        assert_eq!(first.previous_page(), None);
        assert_eq!(first.first_row_number(), Some(1));

        let last = window(2, 50, 120);
        assert_eq!(last.next_page(), None);
        assert_eq!(last.one_based_page(), Some(3));
    }

    #[test]
    fn page_ranges_partition_the_result_without_gaps_or_duplicates() {
        for total in 0..=200usize {
            for ps in 1..=60usize {
                let page_count = window(0, ps, total).page_count;
                let mut expected = 0usize;
                for page in 0..page_count {
                    let w = window(page, ps, total);
                    assert_eq!(w.start, expected, "gap/overlap at page {page}");
                    assert!(w.end >= w.start);
                    expected = w.end;
                    assert_invariants(&w);
                }
                assert_eq!(expected, total, "last page must end at total");
            }
        }
    }

    #[test]
    fn deterministic_result_for_repeated_input() {
        let input = PaginationInput {
            requested_page: 7,
            page_size: 25,
            total_items: 123,
        };
        let first = paginate(input);
        for _ in 0..100 {
            assert_eq!(paginate(input), first);
        }
    }

    #[test]
    fn combinatorial_model_matrix_holds_every_invariant() {
        let mut combinations = 0usize;
        for total in 0..=200usize {
            for ps in 0..=100usize {
                for page in [0usize, 1, 2, 3, 4, 7, 19, usize::MAX] {
                    let w = window(page, ps, total);
                    assert_invariants(&w);
                    combinations += 1;
                }
            }
        }
        assert!(
            combinations >= 100_000,
            "combinatorial matrix must reach at least 100000 combinations, got {combinations}"
        );
    }
}
