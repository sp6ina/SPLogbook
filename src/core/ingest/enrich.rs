// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Wzbogacanie rekordu QSO danymi DXCC, kraju, kontynentu oraz stref CQ/ITU (PIPE-2).

use crate::core::prefix::PrefixInfo;
use crate::core::qso::QsoRecord;

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
