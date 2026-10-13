// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Wzbogacanie i bezpieczna normalizacja rekordu QSO w potoku `core::ingest` (PIPE-2, PIPE-4B.1).

use crate::core::bandplan::get_band_by_freq;
use crate::core::prefix::PrefixInfo;
use crate::core::qso::QsoRecord;

/// Bezpieczna normalizacja domenowa pól rekordu QSO przed walidacją i zapisem.
///
/// Zasady normalizacji (PIPE-4B.1):
/// - `callsign`: obcięcie białych znaków (`trim`) oraz konwersja na wielkie litery (`uppercase`),
/// - `mode`: obcięcie białych znaków (`trim`) oraz konwersja na wielkie litery (`uppercase`),
/// - `band`: jeśli pole jest puste lub zawiera same białe znaki i dostępna jest poprawna
///   częstotliwość (`freq`), wyznaczenie nazwy pasma z planu pasm (`get_band_by_freq`);
///   w przeciwnym razie obcięcie białych znaków,
/// - `qso_date`: normalizacja formatu istniejącej daty poprzez usunięcie separatorów `-`, `.`, `/`
///   oraz `trim` — BEZ fabrykowania nowej daty (np. przez `Utc::now()`),
/// - `time_on` oraz opcjonalnie `time_off`: normalizacja formatu istniejącego czasu poprzez usunięcie `:`
///   oraz `trim` — BEZ fabrykowania nowego czasu.
pub(super) fn normalize_qso_fields(qso: &mut QsoRecord) {
    qso.callsign = qso.callsign.trim().to_uppercase();
    qso.mode = qso.mode.trim().to_uppercase();

    let band_trimmed = qso.band.trim();
    if band_trimmed.is_empty() {
        if let Some(freq_mhz) = qso.freq {
            if freq_mhz.is_finite() && freq_mhz > 0.0 {
                let freq_hz = (freq_mhz * 1_000_000.0).round() as u64;
                if let Some(band_def) = get_band_by_freq(freq_hz) {
                    qso.band = band_def.name.to_string();
                } else {
                    qso.band = String::new();
                }
            } else {
                qso.band = String::new();
            }
        } else {
            qso.band = String::new();
        }
    } else {
        qso.band = band_trimmed.to_string();
    }

    qso.qso_date = qso.qso_date.trim().replace(['-', '.', '/'], "");
    qso.time_on = qso.time_on.trim().replace(':', "");
    if let Some(ref toff) = qso.time_off {
        qso.time_off = Some(toff.trim().replace(':', ""));
    }
}

/// Uzupełnia brakujące pola `QsoRecord` poprawnymi wartościami z wyniku `PrefixMatcher::lookup`.
///
/// Istniejące poprawne wartości w rekordzie nie są nadpisywane. Wartości zerowe (`0`) dla pól
/// liczbowych (`dxcc`, `cqz`, `ituz`) oraz puste napisy dla `country` i `continent` są pomijane.
pub(super) fn apply_prefix_info(qso: &mut QsoRecord, info: &PrefixInfo) {
    if qso.dxcc.is_none() && info.dxcc > 0 {
        qso.dxcc = Some(info.dxcc);
    }
    if qso.cqz.is_none() && info.cqz > 0 {
        qso.cqz = Some(info.cqz);
    }
    if qso.ituz.is_none() && info.ituz > 0 {
        qso.ituz = Some(info.ituz);
    }
    if qso.country.as_deref().is_none_or(|s| s.trim().is_empty()) && !info.country.trim().is_empty()
    {
        qso.country = Some(info.country.clone());
    }
    if qso.continent.as_deref().is_none_or(|s| s.trim().is_empty())
        && !info.continent.trim().is_empty()
    {
        qso.continent = Some(info.continent.clone());
    }
}
