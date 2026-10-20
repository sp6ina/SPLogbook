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
    "plugins",
    "W1AW, SP6INA, DL1ABC",
    "🔍 QRZ",
    "⚠ DUPE!",
    "⚠ WORKED BEFORE (B4)",
    "⭐ NEW DXCC (ATNO)!",
    "✨ NEW BAND!",
    "🔹 NEW MODE!",
    "🌐 NEW WAZ (ZONE)!",
    "🇺🇸 NEW STATE (WAS)!",
    "🌍 NEW CONTINENT (WAC)!",
    "🏝 NEW IOTA!",
    "EU-001",
    "Manager",
    "⏹ STOP",
    "● REC",
    "⚙",
    "VFO A:",
    "VFO B (TX):",
    "SPLIT",
    "+1k",
    "+5k",
    "-1k",
    "-10k",
    "+10k",
    "⇄ A/B",
    "MHz",
    "kHz:",
    "FIL1 (3k)",
    "FIL2 (2.4k)",
    "FIL3 (500Hz)",
    "NB/NR",
    "S-METER:",
    "S1     S3     S5     S7     S9    +20   +40   +60dB",
    "⟳",
    "F1: CQ",
    "F2: TEST",
    "F3: 599 TU",
    "F4: 73",
    "F5: ZNAK",
    "F6: AGN?",
    "F7: CFM",
    "⛔ ESC: PRZERWIJ (ABORT)",
    "…",
    "📻 TX",
    "Start",
    "Zatrzymaj",
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
    "Radio:",
    "🎧 Active RX",
    "🎙 Active TX",
    "(50 - 2000 ms, default 250 ms)",
    "Requesting...",
    "Releasing...",
    "Host TCP:",
    "Port TCP:",
    "7300, FT-891, TS-590, G90, K4",
    "FLRig Host XML-RPC:",
    "(default :12345)",
    "TCI Host WebSocket:",
    "(default :40001)",
    "Model Icom:",
    "KNS Host:",
    "Model Kenwood:",
    "Model Yaesu:",
    "K4 Host TCP:",
    "Model Elecraft:",
    "Model:",
    "COM3, /dev/ttyUSB0",
    "◀ -1k",
    "▶ +1k",
    "Grid:",
    "SPLogbook — Setup Wizard",
    "SPLogbook",
    "np. SP6INA, W1AW, DL1ABC",
    "np. Jan Kowalski",
    "np. JO81WA, KO02AA",
    "np. Wroclaw, Krakow",
    "np. Poland, Germany",
    "np. WR01",
    "np. Icom IC-7300, Kenwood TS-890S",
    "np. Dipol 80m, Yagi 6el 20m, GP 40m",
    "CAT / Hamlib (rigctld):",
    "JS8Call:",
    "(port 2237)",
    "LoTW (ARRL):",
    "Login:",
    "callsign LoTW",
    "Haslo:",
    "QRZ.com XML API:",
    "Club Log:",
    "Email:",
    "Lokalny REST API:",
    "LoTW",
    "np. DX-pileup 20m",
    "v{}",
    "({})",
    "https://cloudlog.twojadomena.pl",
    "Upload Code:",
    "🏆 LoTW",
    "📨 eQSL.cc",
    "📡 Club Log",
    "📖 QRZ & HamQTH",
    "☁ Cloudlog / HRDLog",
    "💻 FLDigi",
    "📡 PSK Reporter",
    "📻 N1MM Broadcast",
    "Host FLDigi XML-RPC:",
    "Port XML-RPC:",
    "Login HamQTH:",
    "● CAT",
    "○ CAT",
    "● CLUSTER",
    "○ CLUSTER",
    "📻 SPLogbook Workspace",
    "📻 SPLogbook",
    "E-mail: contact@splogbook.org",
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

        let content = fs::read_to_string(file_path).unwrap_or_else(|e| {
            panic!("Nie można odczytać pliku {rel_path}: {e}");
        });

        let viols = check_file_for_hardcoded_text(&rel_path, &content);
        if !viols.is_empty() {
            total_violations.push((rel_path, viols));
        }
    }

    if !total_violations.is_empty() {
        let mut msg = String::from("Wykryto niezlokalizowane teksty UI w plikach src/gui:\n");
        for (f, vs) in total_violations {
            msg.push_str(&format!("Plik: {f}\n"));
            for (line, desc, lit) in vs {
                msg.push_str(&format!("  L{line}: {desc} -> {lit:?}\n"));
            }
        }
        panic!("{msg}");
    }
}

fn check_file_for_string_errors(content: &str) -> Vec<(usize, String)> {
    let mut violations = Vec::new();
    let mut in_test = false;

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

        let mut idx = 0;
        while let Some(pos) = line[idx..].find("Result<") {
            let actual_pos = idx + pos;
            let sub = &line[actual_pos + 7..];
            let mut depth_angle = 1;
            let mut depth_paren = 0;
            let mut depth_bracket = 0;
            let mut err_type = String::new();
            let mut found_comma = false;

            for ch in sub.chars() {
                match ch {
                    '<' => depth_angle += 1,
                    '>' => {
                        depth_angle -= 1;
                        if depth_angle == 0 {
                            break;
                        }
                    }
                    '(' => depth_paren += 1,
                    ')' => depth_paren -= 1,
                    '[' => depth_bracket += 1,
                    ']' => depth_bracket += 1,
                    ',' if depth_angle == 1 && depth_paren == 0 && depth_bracket == 0 => {
                        found_comma = true;
                        continue;
                    }
                    _ => {}
                }

                if found_comma {
                    err_type.push(ch);
                }
            }

            let err = err_type.trim();
            if err == "String" || err == "std::string::String" {
                violations.push((line_no, line.trim().to_string()));
            }

            idx = actual_pos + 7;
        }
    }

    violations
}

#[test]
fn no_string_errors_in_domain() {
    let src_dir = Path::new("src");
    assert!(src_dir.is_dir(), "Katalog src/ nie istnieje");

    let mut files = Vec::new();
    collect_rs_files(src_dir, &mut files);
    files.sort();

    let mut violations_found = Vec::new();

    for file_path in &files {
        let rel_path = file_path.to_string_lossy().replace('\\', "/");
        if rel_path.starts_with("src/bin/") {
            continue;
        }

        let content = fs::read_to_string(file_path).unwrap_or_else(|e| {
            panic!("Nie można odczytać pliku {rel_path}: {e}");
        });

        let viols = check_file_for_string_errors(&content);
        if !viols.is_empty() {
            violations_found.push((rel_path, viols));
        }
    }

    if !violations_found.is_empty() {
        let mut failure_msg = String::from("Wykryto Result<..., String> w modułach domenowych:\n");
        for (m, viols) in violations_found {
            failure_msg.push_str(&format!("Plik: {m}\n"));
            for (line, txt) in viols {
                failure_msg.push_str(&format!("  L{line}: {txt}\n"));
            }
        }
        panic!("{failure_msg}");
    }
}

/// Dopuszczalne wyjątki dla tekstów identycznych między językiem angielskim a pozostałymi.
/// Ograniczone do maksymalnie 40 pozycji, wyłącznie dla nazw własnych, skrótów technicznych i jednostek.
const IDENTICAL_ALLOWED: &[(&str, &str)] = &[
    (
        "app.author",
        "Nazwisko autora programu i znak krótkofalarski",
    ),
    ("app.name", "Nazwa własna programu komputerowego"),
    (
        "awards.tab_dxcc",
        "Międzynarodowa nazwa własna dyplomu DX Century Club",
    ),
    (
        "awards.total_qsos_count",
        "Zmienna z uniwersalnym skrótem łączności krótkofalarskiej",
    ),
    (
        "cat_settings.backend_flrig",
        "Nazwa oprogramowania zewnętrznego i protokołu XML-RPC",
    ),
    (
        "cat_settings.backend_hamlib",
        "Nazwa biblioteki zewnętrznej i demona rigctld",
    ),
    (
        "cat_settings.backend_tci",
        "Nazwa protokołu sieciowego i linii urządzeń SDR",
    ),
    (
        "cat_settings.cap_rit",
        "Uniwersalne międzynarodowe skróty radiowe RIT/XIT",
    ),
    (
        "cmd.psk_reporter",
        "Nazwa własna globalnego serwisu PSK Reporter",
    ),
    (
        "cw.macro_f1_label",
        "Klawisz funkcyjny i międzynarodowe wywołanie ogólne CQ",
    ),
    (
        "cw.macro_f3_label",
        "Klawisz funkcyjny i międzynarodowe telegraficzne kody pożegnalne TU/73",
    ),
    (
        "hud.window_title",
        "Nazwa programu i skrót techniczny Head-Up Display",
    ),
    (
        "statusbar.utc",
        "Uniwersalny standard czasu koordynowanego UTC",
    ),
    (
        "sync.tab_cloudlog",
        "Nazwa własna usługi Cloudlog i protokół REST API",
    ),
    (
        "sync.tab_clublog",
        "Nazwa własna serwisu Club Log i terminologia klastra",
    ),
    (
        "sync.tab_hamqth",
        "Nazwa własna serwisu HamQTH i znak założyciela OK1RR",
    ),
    ("sync.tab_hrdlog", "Nazwa domenowa serwisu HRDLog.net"),
    (
        "sync.tab_lotw",
        "Nazwa własna usługi ARRL Logbook of the World",
    ),
    (
        "sync.tab_psk_reporter",
        "Nazwa własna serwisu PSK Reporter i adres URL",
    ),
    (
        "sync.tab_qrz",
        "Nazwa własna i domena internetowa serwisu QRZ.COM",
    ),
    ("tab.vfo", "Międzynarodowe skróty radiotechniczne VFO / CAT"),
    (
        "toolbar.lotw",
        "Międzynarodowy skrót usługi Logbook of the World",
    ),
    (
        "toolbar.vfo",
        "Uniwersalny skrót Variable Frequency Oscillator",
    ),
    (
        "wspr.col_snr",
        "Naukowe oznaczenie stosunku sygnału do szumu SNR i jednostka decybeli dB",
    ),
];

#[test]
fn identical_allowed_is_bounded() {
    assert!(
        IDENTICAL_ALLOWED.len() <= 40,
        "IDENTICAL_ALLOWED przekracza dopuszczalny limit 40 pozycji: {}",
        IDENTICAL_ALLOWED.len()
    );
}

#[test]
fn identical_allowed_entries_are_used() {
    let locales_dir = Path::new("locales");
    let en_path = locales_dir.join("en.json");
    let en_content = fs::read_to_string(&en_path).unwrap();
    let en_json: serde_json::Value = serde_json::from_str(&en_content).unwrap();
    let en_map = en_json.as_object().unwrap();

    let mut non_en_maps = Vec::new();
    for &lang in &["de", "fr", "es", "it", "ru"] {
        let p = locales_dir.join(format!("{lang}.json"));
        let c = fs::read_to_string(&p).unwrap();
        let j: serde_json::Value = serde_json::from_str(&c).unwrap();
        non_en_maps.push((lang, j.as_object().unwrap().clone()));
    }

    for &(key, reason) in IDENTICAL_ALLOWED {
        assert!(
            !reason.trim().is_empty(),
            "Wpis '{key}' w IDENTICAL_ALLOWED musi posiadać uzasadnienie"
        );
        let en_val = en_map.get(key).unwrap_or_else(|| {
            panic!("Wpis '{key}' z IDENTICAL_ALLOWED nie istnieje w en.json");
        });

        let mut matches_any = false;
        for (_lang, map) in &non_en_maps {
            if map.get(key) == Some(en_val) {
                matches_any = true;
                break;
            }
        }
        assert!(
            matches_any,
            "Wpis '{key}' w IDENTICAL_ALLOWED nie jest identyczny z EN w żadnym z języków nieangielskich"
        );
    }
}

#[test]
fn translations_are_not_english_copies() {
    let locales_dir = Path::new("locales");
    let en_path = locales_dir.join("en.json");
    let en_content = fs::read_to_string(&en_path).unwrap();
    let en_json: serde_json::Value = serde_json::from_str(&en_content).unwrap();
    let en_map = en_json.as_object().unwrap();

    let allowed_keys: BTreeSet<&str> = IDENTICAL_ALLOWED.iter().map(|(k, _)| *k).collect();

    let mut violations = Vec::new();

    for &lang in &["de", "fr", "es", "it", "ru"] {
        let p = locales_dir.join(format!("{lang}.json"));
        let c = fs::read_to_string(&p).unwrap();
        let j: serde_json::Value = serde_json::from_str(&c).unwrap();
        let map = j.as_object().unwrap();

        for (k, en_val) in en_map {
            if allowed_keys.contains(k.as_str()) {
                continue;
            }
            if map.get(k) == Some(en_val) {
                violations.push((lang, k.clone(), en_val.as_str().unwrap().to_string()));
            }
        }
    }

    if !violations.is_empty() {
        let mut msg = format!(
            "Wykryto {} nieprzetłumaczonych kopii języka angielskiego poza IDENTICAL_ALLOWED:\n",
            violations.len()
        );
        for (lang, k, val) in violations {
            msg.push_str(&format!("  [{lang}] '{k}' == {val:?}\n"));
        }
        panic!("{msg}");
    }
}

#[test]
fn all_locales_have_identical_parameter_sets() {
    let locales_dir = Path::new("locales");
    let ref_path = locales_dir.join("pl.json");
    let ref_content = fs::read_to_string(&ref_path).unwrap();
    let ref_json: serde_json::Value = serde_json::from_str(&ref_content).unwrap();
    let ref_map = ref_json.as_object().unwrap();

    for &lang in &LANGUAGES {
        if lang == "pl" {
            continue;
        }
        let p = locales_dir.join(format!("{lang}.json"));
        let c = fs::read_to_string(&p).unwrap();
        let j: serde_json::Value = serde_json::from_str(&c).unwrap();
        let map = j.as_object().unwrap();

        for (k, pl_val) in ref_map {
            let pl_text = pl_val.as_str().unwrap();
            let pl_params = extract_params("pl.json", k, pl_text);

            let other_val = map.get(k).unwrap_or_else(|| {
                panic!("Brak klucza '{k}' w {lang}.json");
            });
            let other_text = other_val.as_str().unwrap();
            let other_params = extract_params(&format!("{lang}.json"), k, other_text);

            assert_eq!(
                pl_params, other_params,
                "Rozbieżność parametrów dla klucza '{k}' między pl ({pl_params:?}) a {lang} ({other_params:?})"
            );
        }
    }
}

#[test]
fn catalog_has_no_dead_keys() {
    let locales_dir = Path::new("locales");
    let pl_path = locales_dir.join("pl.json");
    let pl_content = fs::read_to_string(&pl_path).unwrap();
    let pl_json: serde_json::Value = serde_json::from_str(&pl_content).unwrap();
    let pl_map = pl_json.as_object().unwrap();

    let mut code_files = Vec::new();
    collect_rs_files(Path::new("src"), &mut code_files);
    collect_rs_files(Path::new("tests"), &mut code_files);

    let mut combined_sources = String::new();
    for f in &code_files {
        if let Ok(c) = fs::read_to_string(f) {
            combined_sources.push_str(&c);
            combined_sources.push('\n');
        }
    }
    if let Ok(c) = fs::read_to_string("build.rs") {
        combined_sources.push_str(&c);
        combined_sources.push('\n');
    }
    if let Ok(c) = fs::read_to_string("README.md") {
        combined_sources.push_str(&c);
        combined_sources.push('\n');
    }
    if let Ok(c) = fs::read_to_string("locales/README.md") {
        combined_sources.push_str(&c);
        combined_sources.push('\n');
    }

    let mut dead_keys = Vec::new();

    for key in pl_map.keys() {
        let const_ident = key.replace('.', "_").to_uppercase();
        let key_literal = format!("\"{key}\"");
        let const_pattern = format!("::{const_ident}");
        let const_pattern2 = format!("key::{const_ident}");

        let found = combined_sources.contains(&key_literal)
            || combined_sources.contains(&const_pattern)
            || combined_sources.contains(&const_pattern2)
            || combined_sources.contains(&const_ident);

        if !found {
            dead_keys.push(key.clone());
        }
    }

    if !dead_keys.is_empty() {
        panic!(
            "Wykryto {} martwych kluczy w katalogu lokalizacji:\n{:#?}",
            dead_keys.len(),
            dead_keys
        );
    }
}

#[test]
fn manual_languages_have_identical_section_ids() {
    let mut ids_per_lang: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    for &lang in &LANGUAGES {
        let path = format!("locales/manual/{lang}.json");
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
        let list: Vec<serde_json::Value> = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("File '{path}' is not valid JSON: {e}"));
        let ids: BTreeSet<String> = list
            .into_iter()
            .enumerate()
            .map(|(idx, val)| {
                val.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| panic!("File '{path}', section #{idx}: missing id"))
                    .to_string()
            })
            .collect();
        ids_per_lang.insert(lang, ids);
    }

    let ref_ids = &ids_per_lang["pl"];
    assert!(!ref_ids.is_empty(), "Reference manual must not be empty");
    for &lang in &LANGUAGES {
        assert_eq!(
            &ids_per_lang[lang], ref_ids,
            "Manual section IDs in '{lang}' differ from 'pl'"
        );
    }
}

#[test]
fn manual_languages_have_identical_order() {
    let mut order_per_lang: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for &lang in &LANGUAGES {
        let path = format!("locales/manual/{lang}.json");
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
        let list: Vec<serde_json::Value> = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("File '{path}' is not valid JSON: {e}"));
        let ids: Vec<String> = list
            .into_iter()
            .enumerate()
            .map(|(idx, val)| {
                val.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| panic!("File '{path}', section #{idx}: missing id"))
                    .to_string()
            })
            .collect();
        order_per_lang.insert(lang, ids);
    }

    let ref_order = &order_per_lang["pl"];
    for &lang in &LANGUAGES {
        assert_eq!(
            &order_per_lang[lang], ref_order,
            "Manual section order in '{lang}' differs from 'pl'"
        );
    }
}

#[test]
fn manual_has_no_empty_titles_or_bodies() {
    for &lang in &LANGUAGES {
        let path = format!("locales/manual/{lang}.json");
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {path}: {e}"));
        let list: Vec<serde_json::Value> = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("File '{path}' is not valid JSON: {e}"));
        for (idx, sec) in list.iter().enumerate() {
            let id = sec.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let title = sec.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let body = sec.get("body").and_then(|v| v.as_str()).unwrap_or("");

            assert!(!id.trim().is_empty(), "Empty id in {path} section #{idx}");
            assert!(
                !title.trim().is_empty(),
                "Empty title in {path} section '{id}'"
            );
            assert!(
                !body.trim().is_empty(),
                "Empty body in {path} section '{id}'"
            );
        }
    }
}

#[test]
fn manual_returns_every_language() {
    use splogbook::core::i18n::{Language, manual};

    for lang in Language::ALL {
        let sections = manual(lang);
        assert_eq!(
            sections.len(),
            17,
            "manual({:?}) should return exactly 17 sections",
            lang
        );
        for sec in sections {
            assert!(
                !sec.id.trim().is_empty(),
                "manual({:?}) section id must not be empty",
                lang
            );
            assert!(
                !sec.title.trim().is_empty(),
                "manual({:?}) section title must not be empty",
                lang
            );
            assert!(
                !sec.body.trim().is_empty(),
                "manual({:?}) section body must not be empty",
                lang
            );
        }
    }
}

#[test]
fn user_manual_renderer_has_no_embedded_translatable_sections() {
    let content = fs::read_to_string("src/gui/user_manual.rs")
        .expect("Failed to read src/gui/user_manual.rs");
    assert!(
        !content.contains("MANUAL_SECTIONS"),
        "src/gui/user_manual.rs must not define MANUAL_SECTIONS static array"
    );
    assert!(
        !content.contains("MANUAL_SECTIONS_EN"),
        "src/gui/user_manual.rs must not define MANUAL_SECTIONS_EN static array"
    );
    assert!(
        !content.contains("Pierwsze kroki"),
        "src/gui/user_manual.rs must not contain hardcoded manual section text"
    );
    assert!(
        !content.contains("Getting Started"),
        "src/gui/user_manual.rs must not contain hardcoded manual section text"
    );
}

#[test]
fn guards_have_no_pending_whitelists() {
    let content =
        fs::read_to_string("tests/i18n_guard.rs").expect("Failed to read tests/i18n_guard.rs");
    let p_files = concat!("PENDING_", "FILES");
    let p_mods = concat!("PENDING_", "MODULES");
    assert!(
        !content.contains(p_files),
        "tests/i18n_guard.rs must not contain any pending whitelist constant or check"
    );
    assert!(
        !content.contains(p_mods),
        "tests/i18n_guard.rs must not contain any pending whitelist constant or check"
    );
}

#[test]
fn domain_errors_substitute_all_parameters() {
    use splogbook::core::i18n::Language;

    let sample_messages = vec![
        splogbook::cat::rig::RigError::NotConnected.msg(),
        splogbook::cat::rig::RigError::Connect {
            endpoint: "127.0.0.1".into(),
            detail: "connection refused".into(),
        }
        .msg(),
        splogbook::cloud::cloudlog::CloudlogError::NotConfigured.msg(),
        splogbook::cloud::cloudlog::CloudlogError::Http("connection reset".into()).msg(),
        splogbook::cloud::hamqth::HamQthError::NotConfigured.msg(),
        splogbook::cloud::hrdlog::HrdlogError::NotConfigured.msg(),
        splogbook::cloud::psk_reporter::PskReporterError::MissingFrequency.msg(),
        splogbook::cloud::wspr::WsprError::InvalidCallsign.msg(),
        splogbook::core::adif::AdifError::Read("unexpected EOF".into()).msg(),
        splogbook::core::backup::BackupError::SourceNotFound.msg(),
        splogbook::core::callbook::CallbookError::NotFound.msg(),
        splogbook::core::database::journals::JournalError::CannotDeleteDefault.msg(),
        splogbook::core::ingest::IngestError::EmptyCallsign.msg(),
        splogbook::core::qso::QsoIssue::EmptyCallsign.msg(),
        splogbook::core::qso::QsoWarning::Iota("EU-140".into()).msg(),
        splogbook::core::sota_export::SotaExportError::InvalidDate {
            callsign: "SP6INA".into(),
        }
        .msg(),
        splogbook::sync::error::SyncError::SecretRequired.msg(),
    ];

    for lang in Language::ALL {
        for msg in &sample_messages {
            let rendered = msg.text(lang);
            assert!(
                !rendered.contains('{') && !rendered.contains('}'),
                "Unsubstituted placeholder in message for {lang:?}: {rendered}"
            );
            assert!(
                !rendered.trim().is_empty(),
                "Empty rendered message for {lang:?}"
            );
        }
    }
}
