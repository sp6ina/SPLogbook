// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::collections::HashSet;
use std::io::BufRead;

/// Silnik podpowiadania znaków Super Check Partial (SCP)
pub struct ScpEngine {
    callsigns: HashSet<String>,
}

impl Default for ScpEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScpEngine {
    pub fn new() -> Self {
        Self {
            callsigns: HashSet::new(),
        }
    }

    /// Ładuje bazę znaków z pliku MASTER.SCP
    pub fn load_from_reader<R: BufRead>(&mut self, reader: R) {
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim().to_uppercase();
            // Pomiń komentarze w plikach SCP
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                self.callsigns.insert(trimmed);
            }
        }
    }

    /// Dodaje pojedynczy znak do bazy podpowiedzi (np. z bazy lokalnego logu)
    pub fn insert(&mut self, callsign: &str) {
        let clean = callsign.trim().to_uppercase();
        if !clean.is_empty() {
            self.callsigns.insert(clean);
        }
    }

    /// Wyszukuje znaki pasujące do wpisanego prefiksu lub fragmentu (Super Check Partial)
    pub fn search(&self, query: &str, limit: usize) -> Vec<String> {
        let q = query.trim().to_uppercase();
        if q.is_empty() {
            return Vec::new();
        }

        let mut prefix_matches: Vec<String> = self
            .callsigns
            .iter()
            .filter(|call| call.starts_with(&q))
            .cloned()
            .collect();
        prefix_matches.sort();

        if prefix_matches.len() >= limit {
            prefix_matches.truncate(limit);
            return prefix_matches;
        }

        let mut contains_matches: Vec<String> = self
            .callsigns
            .iter()
            .filter(|call| !call.starts_with(&q) && call.contains(&q))
            .cloned()
            .collect();
        contains_matches.sort();

        let needed = limit - prefix_matches.len();
        prefix_matches.extend(contains_matches.into_iter().take(needed));
        prefix_matches
    }

    pub fn len(&self) -> usize {
        self.callsigns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.callsigns.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scp_search() {
        let mut scp = ScpEngine::new();
        scp.insert("SP6INA");
        scp.insert("SP6ZDA");
        scp.insert("SQ6ABC");
        scp.insert("W1AW");

        let res = scp.search("SP6", 5);
        assert_eq!(res.len(), 2);
        assert!(res.contains(&"SP6INA".to_string()));
        assert!(res.contains(&"SP6ZDA".to_string()));

        let partial = scp.search("6IN", 5);
        assert_eq!(partial, vec!["SP6INA"]);
    }
}
