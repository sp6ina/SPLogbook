// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// Lokalna, w pełni offline korekta błędnie odebranych/ wpisanych znaków
// wywoławczych (fuzzy matching). Nie wysyła żadnych danych na zewnątrz —
// kandydatów dostarczają wyłącznie lokalny dziennik łączności i baza SCP.

/// Klasyczna odległość Levenshteina (edycyjna) między dwoma napisami.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];

    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b.len()]
}

/// Proponuje poprawki dla podanego znaku na podstawie listy kandydatów
/// (np. znaków z lokalnego logu i bazy SCP).
///
/// Zwraca pary `(znak, odległość)` posortowane rosnąco według odległości.
/// Znaki identyczne z wejściem są pomijane (to nie jest „poprawka").
pub fn suggest_corrections(
    input: &str,
    candidates: &[&str],
    max_distance: usize,
    limit: usize,
) -> Vec<(String, usize)> {
    let query = input.trim().to_uppercase();
    let mut results: Vec<(String, usize)> = Vec::new();

    if query.len() < 3 {
        return results;
    }

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for candidate in candidates {
        let cand = candidate.trim().to_uppercase();
        if cand.is_empty() || cand == query {
            continue;
        }

        // Wstępny filtr długości — różnica większa niż `max_distance`
        // wyklucza możliwość dopasowania w granicach progu.
        let len_diff = (cand.chars().count() as isize - query.chars().count() as isize).unsigned_abs();
        if len_diff > max_distance {
            continue;
        }

        if !seen.insert(cand.clone()) {
            continue;
        }

        let distance = levenshtein_distance(&query, &cand);
        if distance <= max_distance {
            results.push((cand, distance));
        }
    }

    results.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    results.truncate(limit);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("SP6INA", "SP6INA"), 0);
        assert_eq!(levenshtein_distance("SP6INA", "SP6INB"), 1);
        assert_eq!(levenshtein_distance("SP6INA", "SP6IN"), 1);
        assert_eq!(levenshtein_distance("", "AB"), 2);
    }

    #[test]
    fn test_suggest_corrections() {
        let candidates = vec!["SP6INA", "SP6ZDA", "W1AW", "DL1ABC", "K1TTT"];
        let res = suggest_corrections("SP6INB", &candidates, 2, 4);
        assert_eq!(res.first().map(|r| r.0.as_str()), Some("SP6INA"));
        assert_eq!(res.first().map(|r| r.1), Some(1));
    }

    #[test]
    fn test_suggest_ignores_exact_and_short() {
        let candidates = vec!["SP6INA", "AB"];
        assert!(suggest_corrections("SP6INA", &candidates, 2, 4).is_empty());
        assert!(suggest_corrections("AB", &candidates, 2, 4).is_empty());
    }
}
