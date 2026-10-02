// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Kompletna baza gmin programu Polska Gmina Award (PGA) wg spga.pl / PZK

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgaGmina {
    pub code: &'static str,
    pub name: &'static str,
    pub powiat: &'static str,
}

pub static ALL_PGA_GMINAS: &[PgaGmina] = include!("pga_data.rs");

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
        .filter(|g| {
            g.code.contains(&query_upper)
                || g.name.to_uppercase().contains(&query_upper)
                || g.powiat.to_uppercase().contains(&query_upper)
        })
        .collect()
}

/// Podpowiada kod PGA na podstawie podanego QTH/miasta
pub fn suggest_pga_for_qth(qth: &str) -> Option<&'static PgaGmina> {
    let qth_upper = qth.trim().to_uppercase();
    if qth_upper.len() < 3 {
        return None;
    }
    ALL_PGA_GMINAS.iter().find(|g| {
        g.name.to_uppercase().contains(&qth_upper) || qth_upper.contains(&g.name.to_uppercase())
    })
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
