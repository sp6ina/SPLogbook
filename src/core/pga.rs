// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Kompletna baza gmin programu Polska Gmina Award (PGA) wg spga.pl / PZK

use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgaGmina {
    pub code: &'static str,
    pub name: &'static str,
    pub powiat: &'static str,
}

pub static ALL_PGA_GMINAS: &[PgaGmina] = include!("pga_data.rs");

/// Wielkie litery nazw i powiatów, wyliczane raz (bez alokacji per zapytanie).
static ALL_PGA_UPPER: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    ALL_PGA_GMINAS
        .iter()
        .map(|g| (g.name.to_uppercase(), g.powiat.to_uppercase()))
        .collect()
});

/// Wyszukuje gminę po kodzie PGA (dokładne dopasowanie, np. "BE01")
pub fn find_by_code(code: &str) -> Option<&'static PgaGmina> {
    let clean = code.trim().to_uppercase();
    ALL_PGA_GMINAS.iter().find(|g| g.code == clean)
}

/// Wyszukuje gminy po nazwie miejscowości, powiatu lub fragmencie kodu PGA
pub fn search_pga(query: &str) -> Vec<&'static PgaGmina> {
    let query_upper = query.trim().to_uppercase();
    if query_upper.is_empty() {
        return Vec::new();
    }
    ALL_PGA_GMINAS
        .iter()
        .zip(ALL_PGA_UPPER.iter())
        .filter(|(g, (name_upper, powiat_upper))| {
            g.code.contains(&query_upper)
                || name_upper.contains(&query_upper)
                || powiat_upper.contains(&query_upper)
        })
        .map(|(g, _)| g)
        .collect()
}

/// Podpowiada kod PGA na podstawie podanego QTH/miasta
pub fn suggest_pga_for_qth(qth: &str) -> Option<&'static PgaGmina> {
    let qth_upper = qth.trim().to_uppercase();
    if qth_upper.len() < 3 {
        return None;
    }
    ALL_PGA_GMINAS
        .iter()
        .zip(ALL_PGA_UPPER.iter())
        .find(|(_, (name_upper, _))| {
            name_upper.contains(&qth_upper) || qth_upper.contains(name_upper)
        })
        .map(|(g, _)| g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pga_lookup() {
        assert!(ALL_PGA_GMINAS.len() > 2400, "Powinno być ponad 2400 gmin");

        let be01 = find_by_code("BE01").expect("Powinien znaleźć BE01");
        assert!(be01.name.contains("Bolesławiec"));
        assert!(be01.powiat.contains("Bolesławiec"));

        let results = search_pga("Wrocław");
        assert!(!results.is_empty(), "Powinno znaleźć gminy we Wrocławiu");

        let sug = suggest_pga_for_qth("Bolesławiec");
        assert!(sug.is_some());
    }
}
