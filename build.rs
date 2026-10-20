// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed=locales/manual");

    generate_i18n_catalog();

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "SPLogbook");
        res.set(
            "FileDescription",
            "SPLogbook - Zaawansowany Dziennik Krotkofalarski",
        );
        res.set("CompanyName", "Mariusz Wozniak (SP6INA)");
        res.set(
            "LegalCopyright",
            "Copyright (C) 2026 Mariusz Wozniak (SP6INA)",
        );
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {e}");
        }
    }
}

const LANGUAGES: [&str; 7] = ["pl", "en", "de", "fr", "es", "it", "ru"];

struct RawManualSection {
    id: String,
    title: String,
    body: String,
}

fn generate_i18n_catalog() {
    let mut catalog: BTreeMap<&'static str, BTreeMap<String, String>> = BTreeMap::new();
    let mut params_per_lang: BTreeMap<&'static str, HashMap<String, BTreeSet<String>>> =
        BTreeMap::new();

    for &lang in &LANGUAGES {
        let file_path = format!("locales/{lang}.json");
        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|e| panic!("Failed to read {file_path}: {e}"));

        let value: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("File '{file_path}' is not valid JSON: {e}"));

        let serde_json::Value::Object(map) = value else {
            panic!("File '{file_path}' must be a JSON object");
        };

        let mut lang_map = BTreeMap::new();
        let mut lang_params = HashMap::new();

        for (key, val) in map {
            let serde_json::Value::String(text) = val else {
                panic!("File '{file_path}', key '{key}': value must be a string");
            };

            if text.is_empty() {
                panic!("File '{file_path}', key '{key}': value cannot be empty");
            }

            if !is_valid_key(&key) {
                panic!(
                    "File '{file_path}', key '{key}': key must match pattern ^[a-z0-9_]+(\\.[a-z0-9_]+)+$"
                );
            }

            let params = extract_params(&file_path, &key, &text);
            lang_params.insert(key.clone(), params);
            lang_map.insert(key, text);
        }

        params_per_lang.insert(lang, lang_params);
        catalog.insert(lang, lang_map);
    }

    // 1. Zbiory kluczy muszą być identyczne we wszystkich językach
    let reference_lang = "pl";
    let reference_keys: BTreeSet<&String> = catalog[reference_lang].keys().collect();

    for &lang in &LANGUAGES {
        if lang == reference_lang {
            continue;
        }
        let current_keys: BTreeSet<&String> = catalog[lang].keys().collect();
        if let Some(&k) = reference_keys.difference(&current_keys).next() {
            panic!("Key '{k}' present in '{reference_lang}.json' but missing in '{lang}.json'");
        }
        if let Some(&k) = current_keys.difference(&reference_keys).next() {
            panic!("Key '{k}' present in '{lang}.json' but missing in '{reference_lang}.json'");
        }
    }

    // 2. Walidacja kolizji nazw stałych oraz zgodności parametrów
    let mut const_map: HashMap<String, String> = HashMap::new();
    for key in catalog[reference_lang].keys() {
        let const_name = key.replace('.', "_").to_uppercase();
        if let Some(existing) = const_map.insert(const_name.clone(), key.clone()) {
            panic!(
                "Constant collision: keys '{key}' and '{existing}' both produce constant '{const_name}'"
            );
        }

        // Porównaj zbiory parametrów
        let ref_params = &params_per_lang[reference_lang][key];
        for &lang in &LANGUAGES {
            if lang == reference_lang {
                continue;
            }
            let lang_params = &params_per_lang[lang][key];
            if ref_params != lang_params {
                panic!(
                    "Key '{key}' has differing parameter placeholders: '{reference_lang}' has {ref_params:?}, but '{lang}' has {lang_params:?}"
                );
            }
        }
    }

    // 3. Walidacja podręcznika użytkownika (locales/manual/*.json)
    let mut manual_catalog: BTreeMap<&'static str, Vec<RawManualSection>> = BTreeMap::new();
    for &lang in &LANGUAGES {
        let file_path = format!("locales/manual/{lang}.json");
        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|e| panic!("Failed to read {file_path}: {e}"));

        let value: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("File '{file_path}' is not valid JSON: {e}"));

        let serde_json::Value::Array(items) = value else {
            panic!("File '{file_path}' must be a JSON array of sections");
        };

        if items.is_empty() {
            panic!("File '{file_path}' cannot be empty");
        }

        let mut sections = Vec::with_capacity(items.len());
        for (idx, item) in items.into_iter().enumerate() {
            let serde_json::Value::Object(map) = item else {
                panic!("File '{file_path}', section #{idx}: must be a JSON object");
            };

            let id = match map.get("id") {
                Some(serde_json::Value::String(s)) if !s.trim().is_empty() => s.clone(),
                _ => panic!("File '{file_path}', section #{idx}: missing or empty 'id'"),
            };
            let title = match map.get("title") {
                Some(serde_json::Value::String(s)) if !s.trim().is_empty() => s.clone(),
                _ => panic!("File '{file_path}', section '{id}': missing or empty 'title'"),
            };
            let body = match map.get("body") {
                Some(serde_json::Value::String(s)) if !s.trim().is_empty() => s.clone(),
                _ => panic!("File '{file_path}', section '{id}': missing or empty 'body'"),
            };

            sections.push(RawManualSection { id, title, body });
        }
        manual_catalog.insert(lang, sections);
    }

    let ref_manual_sections = &manual_catalog[reference_lang];
    let ref_ids: Vec<&str> = ref_manual_sections.iter().map(|s| s.id.as_str()).collect();

    for &lang in &LANGUAGES {
        if lang == reference_lang {
            continue;
        }
        let current_ids: Vec<&str> = manual_catalog[lang].iter().map(|s| s.id.as_str()).collect();
        if current_ids != ref_ids {
            panic!(
                "Manual sections mismatch in '{lang}.json': expected order {ref_ids:?}, got {current_ids:?}"
            );
        }
    }

    // 4. Stabilne sortowanie kluczy (BTreeMap.keys() już jest posortowane leksykograficznie)
    let sorted_keys: Vec<&String> = catalog[reference_lang].keys().collect();
    let key_count = sorted_keys.len();

    let mut out = String::with_capacity(384 * 1024);
    out.push_str("// Auto-generated by build.rs — DO NOT EDIT\n\n");
    out.push_str(&format!("pub const KEY_COUNT: usize = {key_count};\n\n"));

    out.push_str("pub mod key {\n");
    out.push_str("    use super::Key;\n\n");
    for (idx, key) in sorted_keys.iter().enumerate() {
        let const_name = key.replace('.', "_").to_uppercase();
        out.push_str(&format!(
            "    pub const {const_name}: Key = Key::from_index({idx});\n"
        ));
    }
    out.push_str("}\n\n");

    out.push_str("pub static KEY_NAMES: [&str; KEY_COUNT] = [\n");
    for key in &sorted_keys {
        out.push_str(&format!("    {key:?},\n"));
    }
    out.push_str("];\n\n");

    for &lang in &LANGUAGES {
        let table_name = format!("TEXT_{}", lang.to_uppercase());
        out.push_str(&format!("pub static {table_name}: [&str; KEY_COUNT] = [\n"));
        let lang_map = &catalog[lang];
        for key in &sorted_keys {
            let text = &lang_map[*key];
            out.push_str(&format!("    {text:?},\n"));
        }
        out.push_str("];\n\n");
    }

    // 5. Kod generatora podręcznika użytkownika
    out.push_str(
        "/// Struktura sekcji podręcznika użytkownika generowana z katalogu locales/manual\n",
    );
    out.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    out.push_str("pub struct ManualSection {\n");
    out.push_str("    pub id: &'static str,\n");
    out.push_str("    pub title: &'static str,\n");
    out.push_str("    pub body: &'static str,\n");
    out.push_str("}\n\n");

    for &lang in &LANGUAGES {
        let table_name = format!("MANUAL_{}", lang.to_uppercase());
        let sections = &manual_catalog[lang];
        out.push_str(&format!(
            "pub static {table_name}: [ManualSection; {}] = [\n",
            sections.len()
        ));
        for sec in sections {
            out.push_str("    ManualSection {\n");
            out.push_str(&format!("        id: {:?},\n", sec.id));
            out.push_str(&format!("        title: {:?},\n", sec.title));
            out.push_str(&format!("        body: {:?},\n", sec.body));
            out.push_str("    },\n");
        }
        out.push_str("];\n\n");
    }

    out.push_str("/// Zwraca listę sekcji podręcznika użytkownika dla wybranego języka.\n");
    out.push_str("pub fn manual(lang: Language) -> &'static [ManualSection] {\n");
    out.push_str("    match lang {\n");
    for &lang in &LANGUAGES {
        let table_name = format!("MANUAL_{}", lang.to_uppercase());
        let variant = match lang {
            "pl" => "Language::Pl",
            "en" => "Language::En",
            "de" => "Language::De",
            "fr" => "Language::Fr",
            "es" => "Language::Es",
            "it" => "Language::It",
            "ru" => "Language::Ru",
            _ => unreachable!(),
        };
        out.push_str(&format!("        {variant} => &{table_name},\n"));
    }
    out.push_str("    }\n");
    out.push_str("}\n");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set");
    let dest_path = Path::new(&out_dir).join("i18n_catalog.rs");
    fs::write(&dest_path, out)
        .unwrap_or_else(|e| panic!("Failed to write to {}: {e}", dest_path.display()));
}

fn is_valid_key(key: &str) -> bool {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    })
}

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
                if inner == '{' {
                    panic!(
                        "File '{file}', key '{key}': unescaped or nested open brace '{{' in value: {text:?}"
                    );
                }
                param_name.push(inner);
            }
            if !closed {
                panic!("File '{file}', key '{key}': unbalanced brace in value: {text:?}");
            }
            if param_name.is_empty() {
                panic!(
                    "File '{file}', key '{key}': empty parameter placeholder '{{}}' in value: {text:?}"
                );
            }
            let mut chars_iter = param_name.chars();
            let first = chars_iter.next().unwrap();
            if !first.is_ascii_lowercase() {
                panic!(
                    "File '{file}', key '{key}': invalid parameter name '{{{param_name}}}', must start with [a-z]"
                );
            }
            if !chars_iter.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') {
                panic!(
                    "File '{file}', key '{key}': invalid parameter name '{{{param_name}}}', must match [a-z][a-z0-9_]*"
                );
            }
            params.insert(param_name);
        } else if c == '}' {
            panic!("File '{file}', key '{key}': unexpected closing brace '}}' in value: {text:?}");
        }
    }
    params
}
