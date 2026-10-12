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
