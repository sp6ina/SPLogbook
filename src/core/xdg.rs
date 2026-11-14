// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Obsługa specyfikacji XDG Base Directory oraz ochrona danych użytkownika Linux (M3-F)

use std::path::{Path, PathBuf};

/// Błąd rozwiązywania ścieżek XDG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XdgError {
    MissingHome,
    RelativeHomePath(String),
}

impl std::fmt::Display for XdgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHome => write!(f, "Brak zmiennej środowiskowej HOME lub pusta ścieżka"),
            Self::RelativeHomePath(p) => write!(f, "Ścieżka HOME nie jest bezwzględna: {p}"),
        }
    }
}

impl std::error::Error for XdgError {}

/// Rozwiązane kanoniczne ścieżki XDG Base Directory dla SPLogbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgPaths {
    pub config_home: PathBuf,
    pub data_home: PathBuf,
    pub state_home: PathBuf,
    pub cache_home: PathBuf,
}

impl XdgPaths {
    /// Rozwiązuje ścieżki XDG z pełnym egzekwowaniem bezpieczeństwa ścieżek bezwzględnych.
    ///
    /// Zgodnie ze specyfikacją XDG Base Directory oraz decyzją OWNER-M3-F-XDG-DATA-01:
    /// - Zmienne XDG muszą zawierać ścieżki bezwzględne;
    /// - Wszelkie ścieżki względne są odrzucane jako niepoprawne i zastępowane bezpiecznym fallbackiem w `$HOME`;
    /// - W razie braku `$HOME` lub gdy `$HOME` nie jest ścieżką bezwzględną funkcja zwraca błąd fail-closed.
    pub fn resolve() -> Result<Self, XdgError> {
        let home_str = std::env::var("HOME").map_err(|_| XdgError::MissingHome)?;
        let home_trimmed = home_str.trim();
        if home_trimmed.is_empty() {
            return Err(XdgError::MissingHome);
        }
        let home_path = PathBuf::from(home_trimmed);
        if !home_path.is_absolute() {
            return Err(XdgError::RelativeHomePath(home_trimmed.to_string()));
        }

        let config_home =
            Self::resolve_single_xdg_var("XDG_CONFIG_HOME", &home_path.join(".config"))
                .join("splogbook");

        let data_home =
            Self::resolve_single_xdg_var("XDG_DATA_HOME", &home_path.join(".local").join("share"))
                .join("splogbook");

        let state_home =
            Self::resolve_single_xdg_var("XDG_STATE_HOME", &home_path.join(".local").join("state"))
                .join("splogbook");

        let cache_home = Self::resolve_single_xdg_var("XDG_CACHE_HOME", &home_path.join(".cache"))
            .join("splogbook");

        Ok(Self {
            config_home,
            data_home,
            state_home,
            cache_home,
        })
    }

    fn resolve_single_xdg_var(var_name: &str, fallback_dir: &Path) -> PathBuf {
        if let Ok(val) = std::env::var(var_name) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if p.is_absolute() {
                    return p;
                }
            }
        }
        fallback_dir.to_path_buf()
    }
}

/// Raport z wykrycia danych użytkownika w starszych lokalizacjach (Legacy Data Detection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyDataFinding {
    pub path: PathBuf,
    pub description: String,
    pub category: String,
}

/// Wykrywa istniejące pliki baz lub konfiguracji w starych lokalizacjach (np. bezpośrednio w korzeniu portable).
///
/// Gwarancja architektoniczna (OWNER-M3-F-XDG-DATA-01):
/// - Funkcja ma charakter wyłącznie detekcyjno-informacyjny.
/// - Bezwzględny ZAKAZ automatycznej, cichej migracji plików bez wiedzy i zgody użytkownika.
pub fn detect_legacy_portable_user_data(portable_root: &Path) -> Vec<LegacyDataFinding> {
    let mut findings = Vec::new();

    let legacy_candidates = [
        ("default_log.db", "Baza danych QSO (SQLite)", "qso_database"),
        (
            "default_log.db-wal",
            "Dziennik zapisu WAL bazy danych QSO",
            "qso_database_wal",
        ),
        (
            "default_log.db-shm",
            "Pamięć dzielona bazy danych QSO",
            "qso_database_shm",
        ),
        (
            "station_config.json",
            "Główna konfiguracja stacji",
            "configuration",
        ),
    ];

    for (name, desc, category) in legacy_candidates {
        let p = portable_root.join(name);
        if p.exists() {
            findings.push(LegacyDataFinding {
                path: p,
                description: desc.to_string(),
                category: category.to_string(),
            });
        }
    }

    // Wykrywanie logów ADIF i Cabrillo
    if let Ok(entries) = std::fs::read_dir(portable_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|s| s.to_str())
            {
                let ext_lower = ext.to_ascii_lowercase();
                if ext_lower == "adi" || ext_lower == "adif" {
                    findings.push(LegacyDataFinding {
                        path: path.clone(),
                        description: "Archiwalny plik eksportu ADIF".to_string(),
                        category: "adif_export".to_string(),
                    });
                } else if ext_lower == "cbr" || ext_lower == "cabrillo" {
                    findings.push(LegacyDataFinding {
                        path: path.clone(),
                        description: "Archiwalny plik eksportu Cabrillo".to_string(),
                        category: "cabrillo_export".to_string(),
                    });
                }
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    // Testy mutujące procesowe HOME/XDG_* zostały przeniesione do
    // `tests/linux_xdg_data_preservation_test.rs`, gdzie korzystają ze wspólnego,
    // serializowanego guarda środowiska (`tests/support/process_env.rs`). Binarka lib
    // nie mutuje już środowiska procesu, więc odczyty w innych testach lib są bezpieczne.

    #[test]
    fn legacy_portable_user_data_detection() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        std::fs::write(root.join("default_log.db"), b"QSO DB").unwrap();
        std::fs::write(root.join("station_config.json"), b"CFG").unwrap();
        std::fs::write(root.join("log_2026.adi"), b"ADIF LOG").unwrap();
        std::fs::write(root.join("contest.cbr"), b"CABRILLO").unwrap();
        std::fs::write(root.join("SPLogbook"), b"ELF").unwrap(); // Program file nie jest legacy user data

        let findings = detect_legacy_portable_user_data(root);
        assert_eq!(findings.len(), 4);

        let paths: Vec<_> = findings.iter().map(|f| f.path.as_path()).collect();
        assert!(paths.contains(&root.join("default_log.db").as_path()));
        assert!(paths.contains(&root.join("station_config.json").as_path()));
        assert!(paths.contains(&root.join("log_2026.adi").as_path()));
        assert!(paths.contains(&root.join("contest.cbr").as_path()));
    }
}
