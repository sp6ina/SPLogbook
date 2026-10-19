// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Theme Guard: audytor zakazu bezpośrednich literałów i konstruktorów kolorów w `src/gui/`.
//! Wszystkie kolory UI muszą pochodzić ze scentralizowanej palety semantycznej (`ctx.palette()` / `ui.palette()`).

use std::fs;
use std::path::Path;

/// Pliki oczekujące na migrację w kolejnych krokach THEME-3 do THEME-7.
/// Każdy oczyszczony plik jest usuwany z tej listy w commicie danej migracji.
/// Po ukończeniu THEME-8 lista musi być pusta.
const PENDING_FILES: &[&str] = &[
    "src/gui/advanced_filter.rs",
    "src/gui/awards_matrix.rs",
    "src/gui/iota_browser.rs",
    "src/gui/online_sync.rs",
    "src/gui/qsl_designer.rs",
    "src/gui/solar_panel.rs",
    "src/gui/states_browser.rs",
    "src/gui/statistics.rs",
    "src/gui/world_map.rs",
];

const FORBIDDEN_CONSTRUCTORS: &[&str] = &[
    "Color32::from_rgb",
    "Color32::from_rgba",
    "Color32::from_gray",
    "Color32::from_black_alpha",
    "Color32::from_white_alpha",
];

const FORBIDDEN_NAMED_COLORS: &[&str] = &[
    "Color32::WHITE",
    "Color32::BLACK",
    "Color32::RED",
    "Color32::GREEN",
    "Color32::BLUE",
    "Color32::YELLOW",
    "Color32::LIGHT_BLUE",
    "Color32::LIGHT_RED",
    "Color32::LIGHT_GREEN",
    "Color32::LIGHT_YELLOW",
    "Color32::LIGHT_GRAY",
    "Color32::DARK_GRAY",
    "Color32::GRAY",
    "Color32::BROWN",
    "Color32::GOLD",
    "Color32::KHAKI",
    "Color32::DEBUG_COLOR",
];

fn scan_dir(dir: &Path, files: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, files);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let norm = path.to_string_lossy().replace('\\', "/");
                files.push(norm);
            }
        }
    }
}

#[test]
fn test_theme_guard_no_forbidden_colors() {
    let mut files = Vec::new();
    let root = Path::new("src/gui");
    assert!(root.exists(), "src/gui must exist");
    scan_dir(root, &mut files);

    let mut violations = Vec::new();

    for file_path in &files {
        // Dozwolony katalog centralnego motywu
        if file_path.contains("/theme/") || file_path.ends_with("/theme.rs") {
            continue;
        }

        // Pliki oczekujące na migrację w kolejnych fazach
        let rel_path = file_path.trim_start_matches("./");
        if PENDING_FILES.contains(&rel_path) {
            continue;
        }

        let content = fs::read_to_string(file_path).unwrap_or_else(|e| {
            panic!("Nie można odczytać pliku {file_path}: {e}");
        });

        let is_qsl_designer = file_path.ends_with("qsl_designer.rs");
        let mut in_qsl_colors_module = false;
        let mut qsl_colors_brace_depth: i32 = 0;

        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();

            // Ignoruj komentarze jednoliniowe
            if trimmed.starts_with("//") {
                continue;
            }

            // Obsługa strukturalnego wyjątku qsl_colors wewnątrz qsl_designer.rs
            if is_qsl_designer {
                if trimmed.contains("mod qsl_colors") {
                    in_qsl_colors_module = true;
                }
                if in_qsl_colors_module {
                    qsl_colors_brace_depth += trimmed.chars().filter(|c| *c == '{').count() as i32;
                    qsl_colors_brace_depth -= trimmed.chars().filter(|c| *c == '}').count() as i32;
                    if qsl_colors_brace_depth <= 0 && trimmed.contains('}') {
                        in_qsl_colors_module = false;
                    }
                    continue; // Dozwolony moduł kolorów projektu karty QSL
                }
            }

            // Sprawdź niedozwolone konstruktory
            for constructor in FORBIDDEN_CONSTRUCTORS {
                if trimmed.contains(constructor) {
                    violations.push(format!(
                        "{file_path}:{line_num}: znaleziono konstruktor {constructor} -> {trimmed}"
                    ));
                }
            }

            // Sprawdź nazwane stałe kolorów
            for named in FORBIDDEN_NAMED_COLORS {
                if trimmed.contains(named) {
                    violations.push(format!(
                        "{file_path}:{line_num}: znaleziono stałą {named} -> {trimmed}"
                    ));
                }
            }

            // Sprawdź ukryte stałe LIGHT_* lub DARK_* typu Color32
            if (trimmed.starts_with("const LIGHT_") || trimmed.starts_with("const DARK_"))
                && trimmed.contains("Color32")
            {
                violations.push(format!(
                    "{file_path}:{line_num}: zdefiniowano lokalną stałą koloru -> {trimmed}"
                ));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("\n=== THEME GUARD VIOLATIONS ({}): ===", violations.len());
        for v in &violations {
            eprintln!("  {}", v);
        }
        panic!(
            "Znaleziono {} naruszeń centralnej palety kolorów! Skonwertuj użycia na tokeny z ctx.palette() lub ui.palette().",
            violations.len()
        );
    }
}

#[test]
fn test_theme_guard_pending_files_exist() {
    for pending in PENDING_FILES {
        let p = Path::new(pending);
        assert!(
            p.exists(),
            "Plik na liście PENDING_FILES nie istnieje: {pending}"
        );
    }
}
