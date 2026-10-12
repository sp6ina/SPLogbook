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

/// Zwraca tekst polski dla `Language::Pl`, a angielski (uniwersalny międzynarodowy) dla pozostałych języków.
pub fn tr_or<'a>(lang: Language, pl: &'a str, en: &'a str) -> &'a str {
    match lang {
        Language::Pl => pl,
        _ => en,
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
}
