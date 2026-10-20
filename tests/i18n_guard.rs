// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const LANGUAGES: [&str; 7] = ["pl", "en", "de", "fr", "es", "it", "ru"];

fn extract_params(file: &str, key: &str, text: &str) -> BTreeSet<String> {
    let mut params = BTreeSet::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut param_name = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '}' {
                    closed = true;
                    break;
                }
                assert!(
                    inner != '{',
                    "File '{file}', key '{key}': unescaped or nested open brace '{{' in value: {text}"
                );
                param_name.push(inner);
            }
            assert!(
                closed,
                "File '{file}', key '{key}': unbalanced brace in value: {text}"
            );
            assert!(
                !param_name.is_empty(),
                "File '{file}', key '{key}': empty parameter placeholder '{{}}' in value: {text}"
            );
            let mut chars_iter = param_name.chars();
            let Some(first) = chars_iter.next() else {
                panic!("File '{file}', key '{key}': parameter name is empty");
            };
            assert!(
                first.is_ascii_lowercase(),
                "File '{file}', key '{key}': invalid parameter name '{{{param_name}}}', must start with [a-z]"
            );
            assert!(
                chars_iter.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'),
                "File '{file}', key '{key}': invalid parameter name '{{{param_name}}}', must match [a-z][a-z0-9_]*"
            );
            params.insert(param_name);
        } else {
            assert!(
                c != '}',
                "File '{file}', key '{key}': unexpected closing brace '}}' in value: {text}"
            );
        }
    }
    params
}

#[test]
fn catalog_languages_have_identical_keys() {
    let locales_dir = Path::new("locales");
    assert!(
        locales_dir.is_dir(),
        "Katalog locales/ nie istnieje w bieżącym katalogu roboczym"
    );

    let mut key_sets: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    let mut param_maps: BTreeMap<&'static str, BTreeMap<String, BTreeSet<String>>> =
        BTreeMap::new();

    for &lang in &LANGUAGES {
        let file_path = locales_dir.join(format!("{lang}.json"));
        assert!(
            file_path.is_file(),
            "Brak wymaganego pliku katalogu: {}",
            file_path.display()
        );

        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|e| panic!("Nie można odczytać pliku {}: {e}", file_path.display()));

        let json_val: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
            panic!(
                "Plik {} zawiera niepoprawny format JSON: {e}",
                file_path.display()
            )
        });

        let serde_json::Value::Object(map) = json_val else {
            panic!(
                "Plik {} musi być obiektem JSON (string -> string)",
                file_path.display()
            );
        };

        let mut keys = BTreeSet::new();
        let mut params = BTreeMap::new();

        for (k, v) in map {
            let serde_json::Value::String(text) = v else {
                panic!(
                    "Plik {}, klucz '{}': wartość musi być typu string",
                    file_path.display(),
                    k
                );
            };

            assert!(
                !text.is_empty(),
                "Plik {}, klucz '{}': wartość nie może być pusta",
                file_path.display(),
                k
            );

            let parts: Vec<&str> = k.split('.').collect();
            assert!(
                parts.len() >= 2
                    && parts.iter().all(|part| {
                        !part.is_empty()
                            && part
                                .chars()
                                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                    }),
                "Plik {}, klucz '{}': klucz musi spełniać wzorzec ^[a-z0-9_]+(\\.[a-z0-9_]+)+$",
                file_path.display(),
                k
            );

            let key_params = extract_params(&format!("{lang}.json"), &k, &text);
            params.insert(k.clone(), key_params);
            keys.insert(k);
        }

        key_sets.insert(lang, keys);
        param_maps.insert(lang, params);
    }

    let ref_lang = "pl";
    let ref_keys = &key_sets[ref_lang];

    for &lang in &LANGUAGES {
        if lang == ref_lang {
            continue;
        }
        let current_keys = &key_sets[lang];

        let missing_in_current: Vec<&String> = ref_keys.difference(current_keys).collect();
        assert!(
            missing_in_current.is_empty(),
            "W pliku '{lang}.json' brakuje kluczy obecnych w '{ref_lang}.json': {:?}",
            missing_in_current
        );

        let extra_in_current: Vec<&String> = current_keys.difference(ref_keys).collect();
        assert!(
            extra_in_current.is_empty(),
            "Plik '{lang}.json' zawiera nadmiarowe klucze nieobecne w '{ref_lang}.json': {:?}",
            extra_in_current
        );
    }

    for key in ref_keys {
        let ref_params = &param_maps[ref_lang][key];
        for &lang in &LANGUAGES {
            if lang == ref_lang {
                continue;
            }
            let current_params = &param_maps[lang][key];
            assert_eq!(
                ref_params, current_params,
                "Klucz '{key}' ma rozbieżne parametry między '{ref_lang}' ({ref_params:?}) a '{lang}' ({current_params:?})"
            );
        }
    }
}

/// Pliki oczekujące na migrację twardo zakodowanych tekstów w etapie I18N-B2.
/// Każdy kolejny commit I18N-B2A do I18N-B2E musi zmniejszać tę listę.
/// W commicie I18N-B6 lista ta zostanie bezwzględnie usunięta.
const PENDING_FILES: &[&str] = &[
    "src/gui/app.rs",
    "src/gui/cat_settings.rs",
    "src/gui/changelog.rs",
    "src/gui/cw_terminal.rs",
    "src/gui/marketplace.rs",
    "src/gui/mini_hud.rs",
    "src/gui/multi_op_window.rs",
    "src/gui/online_sync.rs",
    "src/gui/plugin_manager.rs",
    "src/gui/qsl_designer.rs",
    "src/gui/qso_entry.rs",
    "src/gui/vfo_panel.rs",
    "src/gui/voice_keyer.rs",
    "src/gui/waterfall_panel.rs",
    "src/gui/welcome_wizard.rs",
    "src/gui/workspace_profiles.rs",
];

const ALLOWED_EXACT_TOKENS: &[&str] = &[
    "",
    " ",
    "-",
    "+",
    "...",
    "?",
    "✓",
    "●",
    "○",
    "■",
    "▶",
    "◀",
    "✕",
    "|",
    "·",
    "CW",
    "SSB",
    "AM",
    "FM",
    "RTTY",
    "FT8",
    "FT4",
    "PSK31",
    "JT65",
    "JT9",
    "Q65",
    "MSK144",
    "160m",
    "80m",
    "60m",
    "40m",
    "30m",
    "20m",
    "17m",
    "15m",
    "12m",
    "10m",
    "6m",
    "4m",
    "2m",
    "70cm",
    "23cm",
    "ONLINE",
    "OFFLINE",
    "SPLIT ON",
    "SPLIT OFF",
    "TX",
    "RX",
    "VFO A",
    "VFO B",
    "MAIN",
    "SUB",
    "QSY 📻",
    "🗑",
    "✨",
    "📋",
    "↗",
    "—",
    "RST S/R",
    "Home QTH, /P, SOTA",
    "SP/BZ-001",
    "SP-0123",
    "| wspr.live API",
    "DUPE!",
    "◐",
    "🔍",
    "•",
    "◦",
    "TRANSCEIVER VFO",
    "● CAT ONLINE",
    "○ CAT OFFLINE",
    "● ONLINE",
    "○ OFFLINE",
    "🔄",
    "📢 Spot",
    "📢 Spot DX",
    "➕",
    "Host:",
    "Port:",
    "SP, W1, POTA",
    "Program / Ref",
    "np. SP7PKI DX Spider",
    "np. dxc.sp7pki.pl",
    "np. SP/BZ-001, OE/TI-001",
    "↩ Undo",
    "↪ Redo",
    "✏",
    "🔊",
    "⬆",
    "⬇",
    "↩",
    "Rel.",
    "Status",
    "NOAA / HamQTH",
    "K-Index",
    "A-Index",
    "✓ C",
    "● W",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "0",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "SP",
    "US",
    "VE",
    "DL",
    "G",
    "JA",
    "VK",
];

const POLISH_DIACRITICS: &[char] = &[
    'ą', 'ć', 'ę', 'ł', 'ń', 'ó', 'ś', 'ź', 'ż', 'Ą', 'Ć', 'Ę', 'Ł', 'Ń', 'Ó', 'Ś', 'Ź', 'Ż',
];

fn is_allowed_ui_literal(s: &str) -> bool {
    let trimmed = s.trim();
    if ALLOWED_EXACT_TOKENS.contains(&trimmed) {
        return true;
    }
    // Czyste znaki interpunkcyjne / formaty / liczby / symbole niealfabetyczne
    if trimmed.chars().all(|c| {
        c.is_ascii_digit() || c.is_ascii_punctuation() || c.is_whitespace() || !c.is_alphabetic()
    }) {
        return true;
    }
    // Krótkie kody / formaty wielkimi literami
    if trimmed.len() <= 4
        && trimmed
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c.is_ascii_punctuation())
    {
        return true;
    }
    false
}

fn collect_rs_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, files);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
}

fn check_file_for_hardcoded_text(
    _path_str: &str,
    content: &str,
) -> Vec<(usize, &'static str, String)> {
    let mut violations = Vec::new();
    let mut in_test = false;

    let widget_prefixes = [
        ".label(",
        ".heading(",
        ".button(",
        ".small_button(",
        ".on_hover_text(",
        ".hint_text(",
        ".checkbox(",
        ".colored_label(",
        "Button::new(",
        "RichText::new(",
        "Window::new(",
    ];

    for (line_idx, line) in content.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();

        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        if trimmed.contains("#[cfg(test)]") || trimmed.contains("mod tests {") {
            in_test = true;
        }
        if in_test {
            continue;
        }

        // 1. Sprawdź obecność polskich znaków w literałach stringowych
        let mut rest = trimmed;
        while let Some(start) = rest.find('"') {
            let after_quote = &rest[start + 1..];
            let mut end_idx = None;
            let mut escaped = false;
            for (idx, ch) in after_quote.char_indices() {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    end_idx = Some(idx);
                    break;
                }
            }
            let Some(end) = end_idx else {
                break;
            };
            let lit = &after_quote[..end];
            if lit.chars().any(|c| POLISH_DIACRITICS.contains(&c)) {
                violations.push((
                    line_no,
                    "Polski znak w literale stringowym",
                    lit.to_string(),
                ));
                break;
            }
            rest = &after_quote[end + 1..];
        }

        // 2. Sprawdź niezlokalizowane wywołania metod widżetowych egui
        for prefix in &widget_prefixes {
            if let Some(pos) = line.find(prefix) {
                let call_tail = &line[pos + prefix.len()..];
                if let Some(q_start) = call_tail.find('"') {
                    // upewnij się, że cudzysłów jest w argumentach tego wywołania
                    let between = call_tail[..q_start].trim();
                    if between.is_empty() || between.ends_with(',') {
                        let after_quote = &call_tail[q_start + 1..];
                        if let Some(q_end) = after_quote.find('"') {
                            let lit = &after_quote[..q_end];
                            if !is_allowed_ui_literal(lit) {
                                violations.push((
                                    line_no,
                                    "Niezlokalizowany literał w wywołaniu widżetu",
                                    lit.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    violations
}

#[test]
fn gui_has_no_hardcoded_text() {
    let gui_dir = Path::new("src/gui");
    assert!(gui_dir.is_dir(), "Katalog src/gui/ nie istnieje");

    let mut files = Vec::new();
    collect_rs_files(gui_dir, &mut files);
    files.sort();

    let mut total_violations = Vec::new();

    for file_path in &files {
        let rel_path = file_path.to_string_lossy().replace('\\', "/");
        if PENDING_FILES.contains(&rel_path.as_str()) {
            continue;
        }

        let content = fs::read_to_string(file_path).unwrap_or_else(|e| {
            panic!("Nie można odczytać pliku {rel_path}: {e}");
        });

        let viols = check_file_for_hardcoded_text(&rel_path, &content);
        if !viols.is_empty() {
            total_violations.push((rel_path, viols));
        }
    }

    if !total_violations.is_empty() {
        let mut msg =
            String::from("Wykryto niezlokalizowane teksty UI w plikach poza PENDING_FILES:\n");
        for (f, vs) in total_violations {
            msg.push_str(&format!("Plik: {f}\n"));
            for (line, desc, lit) in vs {
                msg.push_str(&format!("  L{line}: {desc} -> {lit:?}\n"));
            }
        }
        panic!("{msg}");
    }
}

#[test]
fn test_i18n_pending_files_exist() {
    for pending in PENDING_FILES {
        let p = Path::new(pending);
        assert!(
            p.exists(),
            "Plik na liście PENDING_FILES nie istnieje: {pending}"
        );
    }
}
