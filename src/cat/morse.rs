// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Kod Morse'a i taktowanie znaków dla kluczowania linią portu (DTR/RTS).
//!
//! Taktowanie według wzorca PARIS: kropka = 1 jednostka, kreska = 3, przerwa wewnątrz
//! znaku = 1, między znakami = 3, między słowami = 7; jednostka trwa `1200 / WPM` ms.
//! Prosigny zapisuje się w nawiasach ostrych (`<SK>`, `<BT>`, `<AR>`, `<KN>`, `<AS>`);
//! ich litery są nadawane bez przerwy międzyznakowej.

use std::time::Duration;

/// Najmniejsza i największa obsługiwana prędkość kluczowania.
pub const MIN_WPM: u8 = 5;
pub const MAX_WPM: u8 = 99;

/// Kod Morse'a znaku (kropki i kreski); `None` dla znaków bez kodu.
pub fn code(c: char) -> Option<&'static str> {
    Some(match c.to_ascii_uppercase() {
        'A' => ".-",
        'B' => "-...",
        'C' => "-.-.",
        'D' => "-..",
        'E' => ".",
        'F' => "..-.",
        'G' => "--.",
        'H' => "....",
        'I' => "..",
        'J' => ".---",
        'K' => "-.-",
        'L' => ".-..",
        'M' => "--",
        'N' => "-.",
        'O' => "---",
        'P' => ".--.",
        'Q' => "--.-",
        'R' => ".-.",
        'S' => "...",
        'T' => "-",
        'U' => "..-",
        'V' => "...-",
        'W' => ".--",
        'X' => "-..-",
        'Y' => "-.--",
        'Z' => "--..",
        '0' => "-----",
        '1' => ".----",
        '2' => "..---",
        '3' => "...--",
        '4' => "....-",
        '5' => ".....",
        '6' => "-....",
        '7' => "--...",
        '8' => "---..",
        '9' => "----.",
        '.' => ".-.-.-",
        ',' => "--..--",
        '?' => "..--..",
        '/' => "-..-.",
        '=' => "-...-",
        '+' => ".-.-.",
        '-' => "-....-",
        '(' => "-.--.",
        ')' => "-.--.-",
        ':' => "---...",
        ';' => "-.-.-.",
        '\'' => ".----.",
        '"' => ".-..-.",
        '@' => ".--.-.",
        _ => return None,
    })
}

/// Czas trwania jednej jednostki (kropki) przy danej prędkości.
pub fn unit(wpm: u8) -> Duration {
    Duration::from_micros(1_200_000 / u64::from(wpm.clamp(MIN_WPM, MAX_WPM)))
}

/// Jeden krok kluczowania: klucz wciśnięty albo zwolniony przez `units` jednostek.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyStep {
    pub key_down: bool,
    pub units: u32,
}

/// Fragment tekstu: zwykły znak, prosign (litery nadawane razem) albo odstęp słów.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Letters(Vec<&'static str>),
    WordGap,
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            if tokens.last().is_some_and(|t| *t != Token::WordGap) {
                tokens.push(Token::WordGap);
            }
            continue;
        }
        if c == '<' {
            let mut inner = String::new();
            let mut closed = false;
            for n in chars.by_ref() {
                if n == '>' {
                    closed = true;
                    break;
                }
                inner.push(n);
            }
            let codes: Vec<&'static str> = inner.chars().filter_map(code).collect();
            if closed && !codes.is_empty() {
                tokens.push(Token::Letters(codes));
            }
            continue;
        }
        if let Some(code) = code(c) {
            tokens.push(Token::Letters(vec![code]));
        }
    }
    while tokens.last() == Some(&Token::WordGap) {
        tokens.pop();
    }
    tokens
}

/// Kroki kluczowania tekstu. Znaki bez kodu Morse'a są pomijane.
pub fn keying_steps(text: &str) -> Vec<KeyStep> {
    let mut steps: Vec<KeyStep> = Vec::new();
    let gap = |steps: &mut Vec<KeyStep>, units: u32| {
        if steps.is_empty() {
            return;
        }
        match steps.last_mut() {
            Some(last) if !last.key_down => last.units = last.units.max(units),
            _ => steps.push(KeyStep {
                key_down: false,
                units,
            }),
        }
    };
    for token in tokenize(text) {
        match token {
            Token::WordGap => gap(&mut steps, 7),
            Token::Letters(codes) => {
                gap(&mut steps, 3);
                for (i, element) in codes.iter().flat_map(|c| c.chars()).enumerate() {
                    if i > 0 {
                        gap(&mut steps, 1);
                    }
                    steps.push(KeyStep {
                        key_down: true,
                        units: if element == '-' { 3 } else { 1 },
                    });
                }
            }
        }
    }
    steps
}

/// Liczba znaków tekstu, które zostaną nadane (do śledzenia postępu w terminalu CW).
pub fn sendable_len(text: &str) -> usize {
    tokenize(text)
        .iter()
        .map(|t| match t {
            Token::Letters(codes) => codes.len(),
            Token::WordGap => 1,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(steps: &[KeyStep]) -> String {
        steps
            .iter()
            .map(|s| {
                let c = if s.key_down { '#' } else { '_' };
                std::iter::repeat_n(c, s.units as usize).collect::<String>()
            })
            .collect()
    }

    #[test]
    fn paris_is_fifty_units_including_the_word_gap() {
        let steps = keying_steps("PARIS PARIS");
        let one_word: u32 = keying_steps("PARIS").iter().map(|s| s.units).sum();
        assert_eq!(one_word + 7, 50, "PARIS + word gap = 50 units");
        let total: u32 = steps.iter().map(|s| s.units).sum();
        assert_eq!(total, 2 * one_word + 7);
    }

    #[test]
    fn letter_and_word_spacing_follow_the_standard() {
        assert_eq!(pattern(&keying_steps("E")), "#");
        assert_eq!(pattern(&keying_steps("A")), "#_###");
        assert_eq!(pattern(&keying_steps("EE")), "#___#");
        assert_eq!(pattern(&keying_steps("E E")), "#_______#");
        assert_eq!(pattern(&keying_steps("  E   E  ")), "#_______#");
    }

    #[test]
    fn prosigns_are_sent_without_a_letter_gap() {
        // SK = ...-.- ; SK jako dwie litery miałoby przerwę 3 jednostek.
        assert_eq!(pattern(&keying_steps("<SK>")), "#_#_#_###_#_###");
        assert_eq!(pattern(&keying_steps("SK")), "#_#_#___###_#_###");
        assert_eq!(sendable_len("TU <SK>"), 5);
    }

    #[test]
    fn unknown_characters_and_unclosed_prosigns_are_skipped() {
        assert_eq!(keying_steps("Ł#%"), Vec::new());
        assert_eq!(pattern(&keying_steps("E<SK")), "#");
        assert_eq!(pattern(&keying_steps("e")), "#");
    }

    #[test]
    fn unit_length_follows_wpm_and_is_clamped() {
        assert_eq!(unit(20), Duration::from_millis(60));
        assert_eq!(unit(12), Duration::from_millis(100));
        assert_eq!(unit(1), unit(MIN_WPM));
        assert_eq!(unit(250), unit(MAX_WPM));
    }
}
