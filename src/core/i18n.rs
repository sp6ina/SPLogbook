// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Key(u16);

impl Key {
    pub const fn from_index(index: usize) -> Self {
        Self(index as u16)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub fn as_str(self) -> &'static str {
        KEY_NAMES[self.index()]
    }
}

include!(concat!(env!("OUT_DIR"), "/i18n_catalog.rs"));

/// Zwraca przetłumaczony tekst dla danego typowanego klucza i języka.
/// Dostęp w czasie O(1) bezpośrednio z wygenerowanej w build.rs tablicy.
pub fn tr(key: Key, lang: Language) -> &'static str {
    let idx = key.index();
    match lang {
        Language::Pl => TEXT_PL[idx],
        Language::En => TEXT_EN[idx],
        Language::De => TEXT_DE[idx],
        Language::Fr => TEXT_FR[idx],
        Language::Es => TEXT_ES[idx],
        Language::It => TEXT_IT[idx],
        Language::Ru => TEXT_RU[idx],
    }
}

/// Bezpieczna struktura do budowania komunikatów z nazwanymi parametrami `{param}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Msg {
    key: Key,
    args: Vec<(&'static str, String)>,
}

impl Msg {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, name: &'static str, value: impl ToString) -> Self {
        let val_str = value.to_string();
        drop(value);
        self.args.push((name, val_str));
        self
    }

    pub fn text(&self, lang: Language) -> String {
        let template = tr(self.key, lang);
        let mut result = String::with_capacity(template.len() + 32);
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut name = String::new();
                let mut closed = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                    name.push(inner);
                }

                if closed {
                    if let Some((_, val)) =
                        self.args.iter().rev().find(|(k, _)| *k == name.as_str())
                    {
                        result.push_str(val);
                    } else {
                        log::error!(
                            "Missing argument '{name}' for i18n key {:?} ({})",
                            self.key,
                            self.key.as_str()
                        );
                        result.push('{');
                        result.push_str(&name);
                        result.push('}');
                    }
                } else {
                    result.push('{');
                    result.push_str(&name);
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}

impl std::fmt::Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text(Language::En))
    }
}

/// Obsługiwane języki w programie SPLogbook
/// Obsługa języków: Polski (PL), Angielski (EN), Niemiecki (DE), Francuski (FR), Hiszpański (ES), Włoski (IT), Rosyjski (RU)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[serde(rename = "pl")]
    #[default]
    Pl,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "de")]
    De,
    #[serde(rename = "fr")]
    Fr,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "it")]
    It,
    #[serde(rename = "ru")]
    Ru,
}

impl Language {
    pub const ALL: [Language; 7] = [
        Language::Pl,
        Language::En,
        Language::De,
        Language::Fr,
        Language::Es,
        Language::It,
        Language::Ru,
    ];

    pub fn code(&self) -> &'static str {
        match self {
            Language::Pl => "pl",
            Language::En => "en",
            Language::De => "de",
            Language::Fr => "fr",
            Language::Es => "es",
            Language::It => "it",
            Language::Ru => "ru",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Pl => "Polski",
            Language::En => "English",
            Language::De => "Deutsch",
            Language::Fr => "Français",
            Language::Es => "Español",
            Language::It => "Italiano",
            Language::Ru => "Русский",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "en" | "en_us" | "en_gb" => Language::En,
            "de" | "de_de" | "de_at" | "de_ch" => Language::De,
            "fr" | "fr_fr" | "fr_be" | "fr_ca" => Language::Fr,
            "es" | "es_es" | "es_mx" => Language::Es,
            "it" | "it_it" | "it_ch" => Language::It,
            "ru" | "ru_ru" => Language::Ru,
            _ => Language::Pl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_returns_translation_for_every_language() {
        assert_eq!(tr(key::QSO_CALLSIGN, Language::Pl), "Znak korespondenta");
        assert_eq!(tr(key::QSO_CALLSIGN, Language::En), "Callsign");
        assert_eq!(tr(key::QSO_CALLSIGN, Language::De), "Rufzeichen");
        assert_eq!(tr(key::QSO_CALLSIGN, Language::Fr), "Indicatif");
        assert_eq!(tr(key::QSO_CALLSIGN, Language::Es), "Indicativo");
        assert_eq!(tr(key::QSO_CALLSIGN, Language::It), "Indicativo");
        assert_eq!(
            tr(key::QSO_CALLSIGN, Language::Ru),
            "Позывной корреспондента"
        );
    }

    #[test]
    fn language_codes_roundtrip() {
        for lang in Language::ALL {
            assert_eq!(Language::from_code(lang.code()), lang);
        }
    }

    #[test]
    fn msg_substitutes_named_arguments() {
        let msg = Msg::new(key::MSG_QSO_SAVED)
            .arg("callsign", "SP6INA")
            .arg("band", "20m")
            .arg("mode", "CW");

        let text_pl = msg.text(Language::Pl);
        assert_eq!(text_pl, "Zapisano łączność z: SP6INA (20m, CW)");
        assert!(!text_pl.contains('{'));
        assert!(!text_pl.contains('}'));

        let text_en = msg.text(Language::En);
        assert_eq!(text_en, "QSO saved with: SP6INA (20m, CW)");
        assert!(!text_en.contains('{'));
        assert!(!text_en.contains('}'));
    }

    #[test]
    fn msg_with_missing_argument_keeps_placeholder() {
        let msg = Msg::new(key::MSG_QSO_SAVED)
            .arg("callsign", "SP6INA")
            .arg("mode", "CW");

        let text_pl = msg.text(Language::Pl);
        assert_eq!(text_pl, "Zapisano łączność z: SP6INA ({band}, CW)");
        assert!(text_pl.contains("{band}"));
    }

    #[test]
    fn msg_display_uses_english() {
        let msg = Msg::new(key::MSG_QSO_NOT_SAVED).arg("reason", "duplicate callsign");

        assert_eq!(format!("{msg}"), msg.text(Language::En));
        assert_eq!(format!("{msg}"), "QSO not saved: duplicate callsign");
    }

    #[test]
    fn manual_returns_sections_for_all_languages() {
        for lang in Language::ALL {
            let sections = manual(lang);
            assert_eq!(sections.len(), 17);
            assert_eq!(sections[0].id, "start");
            assert!(!sections[0].title.is_empty());
            assert!(!sections[0].body.is_empty());
        }
    }
}
