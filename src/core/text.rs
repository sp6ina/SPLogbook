// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

/// Escapuje znaki specjalne SQL `LIKE` (`%`, `_` oraz sam znak ucieczki `\`), aby
/// tekst wyszukiwany przez użytkownika nie był traktowany jako wzorzec wildcard.
/// Używać zawsze razem z klauzulą `ESCAPE '\'` w zapytaniu `LIKE`.
pub fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Transliteruje polskie znaki diakrytyczne na odpowiedniki ASCII (np. `Ł` → `L`).
/// Używane przy eksporcie do formatów, które wymagają czystego ASCII (np. PDF).
pub fn transliterate_pl(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'ą' => 'a',
            'ć' => 'c',
            'ę' => 'e',
            'ł' => 'l',
            'ń' => 'n',
            'ó' => 'o',
            'ś' => 's',
            'ź' | 'ż' => 'z',
            'Ą' => 'A',
            'Ć' => 'C',
            'Ę' => 'E',
            'Ł' => 'L',
            'Ń' => 'N',
            'Ó' => 'O',
            'Ś' => 'S',
            'Ź' | 'Ż' => 'Z',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_like_escapes_wildcards_and_escape_char() {
        assert_eq!(escape_like("50%"), "50\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("c:\\d"), "c:\\\\d");
    }

    #[test]
    fn transliterate_pl_maps_polish_diacritics() {
        assert_eq!(transliterate_pl("Zażółć gęślą jaźń"), "Zazolc gesla jazn");
        assert_eq!(transliterate_pl("ŁÓDŹ"), "LODZ");
        assert_eq!(transliterate_pl("abc 123"), "abc 123");
    }
}
