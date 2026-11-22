// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Odczyt informacji ze spotu klastra: emisja i praca split (`UP 2`, `QSX 14025`).
//!
//! Komentarz jest dzielony na słowa, więc `UP` w `GROUP` ani `CW` w `CWOPS` niczego nie
//! oznaczają. Emisję bez podpowiedzi w komentarzu wyznacza segment planu pasma.

use crate::core::bandplan::BandPlan;

/// Emisja stacji ze spotu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpotMode {
    Cw,
    Ssb,
    Fm,
    Am,
    /// Emisja cyfrowa pod nazwą ADIF (`FT8`, `FT4`, `RTTY`, `PSK31`, `JS8`, `SSTV`…).
    Digital(String),
}

impl SpotMode {
    /// Nazwa emisji do dziennika (`SSB` rozdzielone na `LSB`/`USB` według pasma).
    pub fn log_mode(&self) -> &str {
        match self {
            Self::Cw => "CW",
            Self::Ssb => "SSB",
            Self::Fm => "FM",
            Self::Am => "AM",
            Self::Digital(mode) => mode,
        }
    }

    /// Emisja radia i typowa szerokość filtra (Hz). Emisje cyfrowe ustawia program cyfrowy
    /// (WSJT-X, fldigi), nie logger, więc dla nich radio nie jest przestawiane.
    pub fn rig_setting(&self, freq_hz: u64) -> Option<(&'static str, u32)> {
        match self {
            Self::Cw => Some(("CW", 500)),
            // Konwencja: poniżej 10 MHz LSB, wyżej USB (z wyjątkiem 60 m, gdzie zawsze USB).
            Self::Ssb if freq_hz < 10_000_000 && !(5_000_000..5_500_000).contains(&freq_hz) => {
                Some(("LSB", 2700))
            }
            Self::Ssb => Some(("USB", 2700)),
            Self::Fm => Some(("FM", 12_000)),
            Self::Am => Some(("AM", 6000)),
            Self::Digital(_) => None,
        }
    }
}

/// Kolory spotów na panoramie SDR (ARGB): nowe DXCC, nowe pasmo, nowa emisja,
/// stacja zrobiona i pozostałe.
pub const SDR_COLOR_NEW_DXCC: u32 = 0xFFFF_3B30;
pub const SDR_COLOR_NEW_BAND: u32 = 0xFFFF_A500;
pub const SDR_COLOR_NEW_MODE: u32 = 0xFF4A_A3FF;
pub const SDR_COLOR_WORKED: u32 = 0xFF8C_8C8C;
pub const SDR_COLOR_OTHER: u32 = 0xFFFF_FFFF;

/// Częstotliwość spotu (kHz) w Hz.
pub fn spot_freq_hz(frequency_khz: f64) -> u64 {
    if frequency_khz.is_finite() && frequency_khz > 0.0 {
        (frequency_khz * 1000.0).round() as u64
    } else {
        0
    }
}

/// Kategoria filtra listy spotów.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotCategory {
    Cw,
    Phone,
    Digital,
}

/// Kategoria spotu; w segmencie danych planu pasma spot jest cyfrowy, nawet gdy nie
/// wiadomo, jaką emisją.
pub fn spot_category(
    comment: &str,
    freq_hz: u64,
    is_ft8: bool,
    plan: &BandPlan,
) -> Option<SpotCategory> {
    match spot_mode(comment, freq_hz, is_ft8, plan) {
        Some(SpotMode::Cw) => Some(SpotCategory::Cw),
        Some(SpotMode::Ssb | SpotMode::Fm | SpotMode::Am) => Some(SpotCategory::Phone),
        Some(SpotMode::Digital(_)) => Some(SpotCategory::Digital),
        None => (plan.suggested_mode(freq_hz) == Some("FT8")).then_some(SpotCategory::Digital),
    }
}

/// Słowa komentarza bez znaków interpunkcyjnych, wielkimi literami.
fn words(comment: &str) -> Vec<String> {
    comment
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '!' | '(' | ')'))
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_uppercase)
        .collect()
}

fn mode_word(word: &str) -> Option<SpotMode> {
    Some(match word {
        "CW" => SpotMode::Cw,
        "SSB" | "USB" | "LSB" | "PHONE" | "PH" => SpotMode::Ssb,
        "FM" => SpotMode::Fm,
        "AM" => SpotMode::Am,
        "FT8" | "FT4" | "RTTY" | "JS8" | "SSTV" | "Q65" | "JT65" | "FST4" | "MSK144" | "PSK31"
        | "PSK63" | "OLIVIA" | "VARA" => SpotMode::Digital(word.to_string()),
        "PSK" | "BPSK" => SpotMode::Digital("PSK31".to_string()),
        _ => return None,
    })
}

/// Emisja spotu: najpierw słowo w komentarzu, potem segment planu pasma.
pub fn spot_mode(comment: &str, freq_hz: u64, is_ft8: bool, plan: &BandPlan) -> Option<SpotMode> {
    if is_ft8 {
        return Some(SpotMode::Digital("FT8".to_string()));
    }
    if let Some(mode) = words(comment).iter().find_map(|w| mode_word(w)) {
        return Some(mode);
    }
    // Segment danych nie mówi, która to emisja cyfrowa — wtedy emisja pozostaje nieznana.
    match plan.suggested_mode(freq_hz)? {
        "CW" => Some(SpotMode::Cw),
        "USB" | "LSB" => Some(SpotMode::Ssb),
        "FM" => Some(SpotMode::Fm),
        _ => None,
    }
}

/// Wskazówka pracy split ze spotu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitHint {
    /// Częstotliwość nadawania podana w komentarzu (Hz).
    Tx(u64),
    /// Samo `UP` bez wartości — operator musi ustawić nadawanie sam.
    UpUnspecified,
}

/// Liczba kHz z tekstu (`2`, `1.5`); odrzuca wartości ujemne i nieskończone.
fn khz(text: &str) -> Option<f64> {
    let value: f64 = text.parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

/// Pierwsza wartość zakresu `5-10` albo cała liczba.
fn first_of_range(text: &str) -> Option<f64> {
    khz(text.split('-').next()?)
}

/// Częstotliwość z `QSX`: pełne kHz (`14025`), MHz (`14.025`) albo końcówka kHz (`025`, `195`)
/// odniesiona do częstotliwości stacji DX — wybierana jest najbliższa zgodna wartość.
fn qsx_hz(value: &str, dx_hz: u64) -> Option<u64> {
    let number = khz(value)?;
    let dx_khz = dx_hz as f64 / 1000.0;
    let tx_khz = if number >= 1000.0 {
        number
    } else if value.contains('.') && number * 1000.0 > dx_khz / 2.0 {
        number * 1000.0
    } else {
        // Końcówka: zastąp ostatnie cyfry kHz częstotliwości DX.
        let digits = value.split('.').next()?.len() as u32;
        let modulus = f64::from(10u32.checked_pow(digits)?);
        let base = (dx_khz / modulus).floor() * modulus;
        [base - modulus, base, base + modulus]
            .into_iter()
            .map(|b| b + number)
            .min_by(|a, b| (a - dx_khz).abs().total_cmp(&(b - dx_khz).abs()))?
    };
    // Split poza pasmem stacji DX to błąd odczytu, nie wskazówka.
    let tx_hz = (tx_khz * 1000.0).round() as u64;
    let same_band = crate::core::bandplan::get_band_by_freq(tx_hz).map(|b| b.name)
        == crate::core::bandplan::get_band_by_freq(dx_hz).map(|b| b.name);
    same_band.then_some(tx_hz)
}

/// Odczytuje split z komentarza: `UP 2`, `UP2`, `UP 5-10`, `DWN 3`, `DOWN 1`, `QSX 14025`.
pub fn split_hint(comment: &str, dx_hz: u64) -> Option<SplitHint> {
    let words = words(comment);
    for (i, word) in words.iter().enumerate() {
        let next = words.get(i + 1).map(String::as_str);
        if word == "QSX" {
            if let Some(tx) = next.and_then(|v| qsx_hz(v, dx_hz)) {
                return Some(SplitHint::Tx(tx));
            }
            continue;
        }
        let (direction, inline) = if let Some(rest) = word.strip_prefix("UP") {
            (1.0, rest)
        } else if let Some(rest) = word
            .strip_prefix("DOWN")
            .or_else(|| word.strip_prefix("DWN"))
            .or_else(|| word.strip_prefix("DN"))
        {
            (-1.0, rest)
        } else {
            continue;
        };
        let offset = if inline.is_empty() {
            next.and_then(first_of_range)
        } else {
            match first_of_range(inline) {
                Some(v) => Some(v),
                // `UPDATE`, `UPSIDE` — nie słowo split.
                None => continue,
            }
        };
        let Some(offset) = offset.filter(|v| *v > 0.0 && *v <= 500.0) else {
            if direction > 0.0 && inline.is_empty() {
                return Some(SplitHint::UpUnspecified);
            }
            continue;
        };
        let tx = dx_hz as f64 + direction * offset * 1000.0;
        if tx > 0.0 {
            return Some(SplitHint::Tx(tx.round() as u64));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_comes_from_whole_words_then_from_the_band_plan() {
        assert_eq!(
            spot_mode("CW 25 WPM", 14_200_000, false, BandPlan::iaru_region1()),
            Some(SpotMode::Cw)
        );
        assert_eq!(
            spot_mode("up 5 ssb", 7_020_000, false, BandPlan::iaru_region1()),
            Some(SpotMode::Ssb)
        );
        assert_eq!(
            spot_mode("", 14_074_000, true, BandPlan::iaru_region1()),
            Some(SpotMode::Digital("FT8".into()))
        );
        assert_eq!(
            spot_mode("rtty test", 14_085_000, false, BandPlan::iaru_region1()),
            Some(SpotMode::Digital("RTTY".into()))
        );
        // „CWOPS”, „GROUP” i „UP” to nie emisje: decyduje segment pasma.
        assert_eq!(
            spot_mode(
                "CWOPS member UP 2",
                14_025_000,
                false,
                BandPlan::iaru_region1()
            ),
            Some(SpotMode::Cw)
        );
        assert_eq!(
            spot_mode("GROUP", 14_250_000, false, BandPlan::iaru_region1()),
            Some(SpotMode::Ssb)
        );
        assert_eq!(
            spot_mode("", 14_080_000, false, BandPlan::iaru_region1()),
            None
        );
    }

    #[test]
    fn rig_setting_follows_the_sideband_convention() {
        assert_eq!(SpotMode::Ssb.rig_setting(7_150_000), Some(("LSB", 2700)));
        assert_eq!(SpotMode::Ssb.rig_setting(14_250_000), Some(("USB", 2700)));
        assert_eq!(SpotMode::Ssb.rig_setting(5_357_000), Some(("USB", 2700)));
        assert_eq!(
            SpotMode::Digital("FT8".into()).rig_setting(14_074_000),
            None
        );
        assert_eq!(SpotMode::Cw.rig_setting(3_520_000), Some(("CW", 500)));
    }

    #[test]
    fn categories_for_the_spot_filter() {
        assert_eq!(
            spot_category("UP 2", 14_025_000, false, BandPlan::iaru_region1()),
            Some(SpotCategory::Cw)
        );
        assert_eq!(
            spot_category("5/9", 14_250_000, false, BandPlan::iaru_region1()),
            Some(SpotCategory::Phone)
        );
        assert_eq!(
            spot_category("", 14_080_000, false, BandPlan::iaru_region1()),
            Some(SpotCategory::Digital)
        );
        assert_eq!(
            spot_category("FT4", 14_250_000, false, BandPlan::iaru_region1()),
            Some(SpotCategory::Digital)
        );
        assert_eq!(spot_freq_hz(14_025.5), 14_025_500);
        assert_eq!(spot_freq_hz(f64::NAN), 0);
        assert_eq!(spot_freq_hz(-1.0), 0);
    }

    #[test]
    fn up_and_down_offsets() {
        let dx = 14_025_000;
        assert_eq!(split_hint("UP 2", dx), Some(SplitHint::Tx(14_027_000)));
        assert_eq!(split_hint("up2", dx), Some(SplitHint::Tx(14_027_000)));
        assert_eq!(split_hint("UP 1.5", dx), Some(SplitHint::Tx(14_026_500)));
        assert_eq!(
            split_hint("UP 5-10", 14_195_000),
            Some(SplitHint::Tx(14_200_000))
        );
        assert_eq!(split_hint("DWN 3", dx), Some(SplitHint::Tx(14_022_000)));
        assert_eq!(
            split_hint("pse DOWN 1", dx),
            Some(SplitHint::Tx(14_024_000))
        );
        assert_eq!(
            split_hint("listening UP", dx),
            Some(SplitHint::UpUnspecified)
        );
        assert_eq!(
            split_hint("TNX QSO, UP!", dx),
            Some(SplitHint::UpUnspecified)
        );
    }

    #[test]
    fn words_that_only_start_like_split_are_ignored() {
        let dx = 14_025_000;
        assert_eq!(split_hint("UPDATE log", dx), None);
        assert_eq!(split_hint("GROUP 5", dx), None);
        assert_eq!(split_hint("SUPER signal", dx), None);
        assert_eq!(split_hint("", dx), None);
        assert_eq!(split_hint("UP 9000", dx), Some(SplitHint::UpUnspecified));
    }

    #[test]
    fn qsx_in_khz_mhz_and_trailing_digits() {
        assert_eq!(
            split_hint("QSX 7150", 7_025_000),
            Some(SplitHint::Tx(7_150_000))
        );
        assert_eq!(
            split_hint("QSX 14.195", 14_160_000),
            Some(SplitHint::Tx(14_195_000))
        );
        assert_eq!(
            split_hint("qsx 025", 14_023_000),
            Some(SplitHint::Tx(14_025_000))
        );
        assert_eq!(
            split_hint("QSX 195", 14_160_000),
            Some(SplitHint::Tx(14_195_000))
        );
        // QSX na innym paśmie to błąd odczytu, nie split.
        assert_eq!(split_hint("QSX 21025", 14_025_000), None);
        assert_eq!(
            split_hint("DX de SP6INA QSX 7150 UP", 7_025_000),
            Some(SplitHint::Tx(7_150_000))
        );
    }
}
