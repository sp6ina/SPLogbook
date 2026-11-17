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

#[cfg(test)]
mod country_name_tests {
    //! A change of entity naming between reference sources must never rewrite
    //! stored QSOs: the ADIF DXCC entity code is the identifier and an
    //! existing COUNTRY value is kept as written.
    use super::*;

    fn info(country: &str, dxcc: u32, cqz: u32, ituz: u32, continent: &str) -> PrefixInfo {
        PrefixInfo {
            callsign: "QA1AA".into(),
            country: country.into(),
            arrl_prefix: String::new(),
            dxcc,
            cqz,
            ituz,
            continent: continent.into(),
            latitude: 0.0,
            longitude: 0.0,
            wpx_prefix: String::new(),
            province: None,
            subdivision_code: None,
            location_precision: crate::core::prefix::LocationPrecision::EntityCentroid,
        }
    }

    const RENAMES: [(&str, &str, u32, u32, u32, &str); 3] = [
        ("USA (CA)", "United States", 291, 3, 6, "NA"),
        ("China, Ji Lin", "China", 318, 24, 33, "AS"),
        ("Russia (Asiatic)", "Asiatic Russia", 15, 17, 30, "AS"),
    ];

    #[test]
    fn existing_country_and_codes_are_never_rewritten() {
        for (old, new, dxcc, cqz, ituz, cont) in RENAMES {
            let mut qso = QsoRecord {
                callsign: "QA1AA".into(),
                country: Some(old.into()),
                dxcc: Some(dxcc),
                cqz: Some(cqz),
                ituz: Some(ituz),
                continent: Some(cont.into()),
                ..QsoRecord::default()
            };
            let before = qso.clone();
            // Even a source reporting other zones must not touch a stored QSO.
            apply_prefix_info(&mut qso, &info(new, dxcc, cqz + 1, ituz + 1, cont));
            assert_eq!(qso, before, "{old}");
        }
    }

    #[test]
    fn new_qso_gets_code_and_current_source_name() {
        for (_, new, dxcc, cqz, ituz, cont) in RENAMES {
            let mut qso = QsoRecord {
                callsign: "QA1AA".into(),
                ..QsoRecord::default()
            };
            apply_prefix_info(&mut qso, &info(new, dxcc, cqz, ituz, cont));
            assert_eq!(qso.dxcc, Some(dxcc));
            assert_eq!(qso.country.as_deref(), Some(new));
            assert_eq!((qso.cqz, qso.ituz), (Some(cqz), Some(ituz)));
            assert_eq!(qso.continent.as_deref(), Some(cont));
        }
    }

    #[test]
    fn stored_names_survive_adif_export_and_import() {
        let qsos: Vec<QsoRecord> = RENAMES
            .iter()
            .map(|(old, _, dxcc, cqz, ituz, cont)| QsoRecord {
                callsign: "QA1AA".into(),
                qso_date: "20261101".into(),
                time_on: "1200".into(),
                band: "20m".into(),
                mode: "CW".into(),
                country: Some((*old).into()),
                dxcc: Some(*dxcc),
                cqz: Some(*cqz),
                ituz: Some(*ituz),
                continent: Some((*cont).into()),
                ..QsoRecord::default()
            })
            .collect();
        let text = crate::core::adif::export_adif(&qsos, "SPLogbook", "QA1AA");
        let back = crate::core::adif::parse_adif(&text);
        assert_eq!(back.len(), qsos.len());
        for (a, b) in qsos.iter().zip(&back) {
            assert_eq!(a.country, b.country);
            assert_eq!(a.dxcc, b.dxcc);
            assert_eq!((a.cqz, a.ituz), (b.cqz, b.ituz));
            assert_eq!(a.continent, b.continent);
        }
        // Re-enriching imported QSOs keeps their names.
        let mut again = back.clone();
        for (q, (_, new, dxcc, cqz, ituz, cont)) in again.iter_mut().zip(RENAMES) {
            apply_prefix_info(q, &info(new, dxcc, cqz, ituz, cont));
        }
        assert_eq!(again, back);
    }
}
