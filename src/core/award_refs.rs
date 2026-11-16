// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Jawny model stron QSO dla referencji dyplomowych SOTA i POTA (F-19).
//!
//! Problem F-19: pole referencji było pojedyncze i dwuznaczne — referencja własnej
//! stacji (aktywacja) trafiała do pola korespondenta (`SOTA_REF`/`POTA_REF`),
//! a pole `MY_*` nie było zapisywane wcale. Ten moduł jest jedynym miejscem, które
//! zna strony QSO i tłumaczy je na pola ADIF.
//!
//! Semantyka (potwierdzona oficjalnymi dokumentami — patrz
//! `docs/ADIF_AWARD_REFERENCES.md`):
//!
//! - [`StationSide::Local`] oznacza **zawsze** stację SPLogbook (stację logującą),
//!   [`StationSide::Remote`] **zawsze** stację korespondenta;
//! - ADIF 3.1.8 definiuje `MY_SOTA_REF` jako "the logging station's International
//!   SOTA Reference" i `SOTA_REF` jako "the contacted station's International SOTA
//!   Reference" — prefiks `MY_` znaczy „moja stacja", a nie „mój kontakt";
//! - oficjalna dokumentacja POTA opisuje park aktywatora wyłącznie przez
//!   `MY_SIG=POTA` + `MY_SIG_INFO`, a park drugiej stacji przez `SIG=POTA` +
//!   `SIG_INFO`; bezpośrednie pola `MY_POTA_REF`/`POTA_REF` są polami ADIF 3.1.8
//!   (`POTARefList`, od 3.1.4), ale POTA ich nie dokumentuje, więc są wyłącznie
//!   reprezentacją wejściową i nośnikiem listy wielu parków;
//! - `SOTARef` jest pojedynczy (brak typu listy), `POTARefList` jest listą
//!   rozdzielaną przecinkami.
//!
//! Zasady wymuszone tutaj:
//!
//! 1. strona lokalna nigdy nie jest kopiowana do zdalnej i odwrotnie;
//! 2. brak referencji to jawne `None`, nie pusty napis i nie referencja drugiej strony;
//! 3. dokładnie jedna reprezentacja POTA na rekord (para SIG **albo** pola
//!    bezpośrednie), nigdy obie, bo mogłyby być sprzeczne;
//! 4. `MY_SIG` nigdy bez `MY_SIG_INFO`, `SIG` nigdy bez `SIG_INFO`;
//! 5. sprzeczne reprezentacje wejściowe kończą się jawnym, typowanym konfliktem —
//!    bez arbitralnego wyboru i bez zamiany stron.

use crate::core::qso::QsoRecord;
use std::collections::HashMap;

/// Wartość pola `SIG`/`MY_SIG` dla programu Parks on the Air.
pub const SIG_POTA: &str = "POTA";

/// Separator listy `POTARefList` (ADIF 3.1.8).
pub const POTA_REF_LIST_SEPARATOR: char = ',';

/// Strona QSO, której dotyczy referencja.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StationSide {
    /// Stacja SPLogbook (stacja logująca, „moja").
    Local,
    /// Stacja korespondenta.
    Remote,
}

impl StationSide {
    /// Wszystkie strony w deterministycznej kolejności.
    pub const ALL: [Self; 2] = [Self::Local, Self::Remote];

    /// Prefiks pól ADIF należących do tej strony.
    #[must_use]
    pub fn adif_prefix(self) -> &'static str {
        match self {
            Self::Local => "MY_",
            Self::Remote => "",
        }
    }

    /// Stabilna etykieta diagnostyczna (nie jest tekstem interfejsu).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }

    /// Para pól ADIF `(SIG, SIG_INFO)` dla tej strony.
    ///
    /// Nazwy pól nie zależą od programu: strona lokalna używa pary `MY_*`,
    /// strona zdalna pary bez prefiksu. Program wyraża **wartość** pola `SIG`
    /// (`POTA`), nie nazwę pola.
    #[must_use]
    pub fn sig_fields(self) -> (&'static str, &'static str) {
        match self {
            Self::Local => ("MY_SIG", "MY_SIG_INFO"),
            Self::Remote => ("SIG", "SIG_INFO"),
        }
    }
}

/// Program dyplomowy obsługiwany przez model referencji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AwardProgram {
    /// Summits on the Air.
    Sota,
    /// Parks on the Air.
    Pota,
}

impl AwardProgram {
    /// Wszystkie programy w deterministycznej kolejności.
    pub const ALL: [Self; 2] = [Self::Sota, Self::Pota];

    /// Stabilna etykieta diagnostyczna (nie jest tekstem interfejsu).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Sota => "SOTA",
            Self::Pota => "POTA",
        }
    }

    /// Bezpośrednie pole ADIF tego programu dla wskazanej strony.
    #[must_use]
    pub fn direct_field(self, side: StationSide) -> &'static str {
        match (self, side) {
            (Self::Sota, StationSide::Local) => "MY_SOTA_REF",
            (Self::Sota, StationSide::Remote) => "SOTA_REF",
            (Self::Pota, StationSide::Local) => "MY_POTA_REF",
            (Self::Pota, StationSide::Remote) => "POTA_REF",
        }
    }
}

/// Para referencji obu stron jednego programu.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwardRefPair {
    /// Referencja stacji SPLogbook.
    pub local: Option<String>,
    /// Referencja stacji korespondenta.
    pub remote: Option<String>,
}

impl AwardRefPair {
    /// Tworzy parę z jawnych wartości obu stron.
    #[must_use]
    pub fn new(local: Option<String>, remote: Option<String>) -> Self {
        Self {
            local: clean(local),
            remote: clean(remote),
        }
    }

    /// Referencja wskazanej strony.
    #[must_use]
    pub fn side(&self, side: StationSide) -> Option<&str> {
        match side {
            StationSide::Local => self.local.as_deref(),
            StationSide::Remote => self.remote.as_deref(),
        }
    }

    /// Ustawia referencję wskazanej strony (puste napisy stają się `None`).
    pub fn set_side(&mut self, side: StationSide, value: Option<String>) {
        let value = clean(value);
        match side {
            StationSide::Local => self.local = value,
            StationSide::Remote => self.remote = value,
        }
    }

    /// Czy obie strony są puste.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.local.is_none() && self.remote.is_none()
    }

    /// Czy któraś strona niesie listę `POTARefList` (więcej niż jedna referencja).
    #[must_use]
    pub fn any_multiple(&self) -> bool {
        self.local.as_deref().is_some_and(is_multi_reference)
            || self.remote.as_deref().is_some_and(is_multi_reference)
    }

    /// Liczba referencji po stronie (0, 1 lub więcej dla listy).
    #[must_use]
    pub fn count(&self, side: StationSide) -> usize {
        self.side(side).map_or(0, |v| split_reference_list(v).len())
    }
}

/// Pełny zestaw referencji rekordu QSO, rozdzielony na program i stronę.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwardReferences {
    /// Referencje SOTA.
    pub sota: AwardRefPair,
    /// Referencje POTA.
    pub pota: AwardRefPair,
}

/// Reprezentacja POTA wybrana dla konkretnego rekordu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotaRepresentation {
    /// Brak referencji POTA po obu stronach.
    Absent,
    /// Kontrakt produkcyjny POTA: `MY_SIG`/`MY_SIG_INFO` oraz `SIG`/`SIG_INFO`.
    Sig,
    /// Pola bezpośrednie ADIF 3.1.8: `MY_POTA_REF`/`POTA_REF`.
    DirectFields,
}

impl AwardReferences {
    /// Odczytuje referencje z rekordu QSO (bez zmiany wartości poza `trim`).
    #[must_use]
    pub fn from_qso(qso: &QsoRecord) -> Self {
        Self {
            sota: AwardRefPair::new(qso.my_sota_ref.clone(), qso.sota_ref.clone()),
            pota: AwardRefPair::new(qso.my_pota_ref.clone(), qso.pota_ref.clone()),
        }
    }

    /// Zapisuje referencje do rekordu QSO — wyłącznie na ich własne strony.
    pub fn apply_to_qso(&self, qso: &mut QsoRecord) {
        qso.my_sota_ref.clone_from(&self.sota.local);
        qso.sota_ref.clone_from(&self.sota.remote);
        qso.my_pota_ref.clone_from(&self.pota.local);
        qso.pota_ref.clone_from(&self.pota.remote);
    }

    /// Referencje wskazanego programu.
    #[must_use]
    pub fn program(&self, program: AwardProgram) -> &AwardRefPair {
        match program {
            AwardProgram::Sota => &self.sota,
            AwardProgram::Pota => &self.pota,
        }
    }

    /// Mutowalne referencje wskazanego programu.
    pub fn program_mut(&mut self, program: AwardProgram) -> &mut AwardRefPair {
        match program {
            AwardProgram::Sota => &mut self.sota,
            AwardProgram::Pota => &mut self.pota,
        }
    }

    /// Czy rekord nie niesie żadnej referencji.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sota.is_empty() && self.pota.is_empty()
    }

    /// Wybiera reprezentację POTA dla tego rekordu.
    ///
    /// Lista wielu parków (`POTARefList`) nie jest reprezentowalna w kontrakcie
    /// uploadu POTA (POTA dokumentuje jedną referencję na pole), dlatego taki
    /// rekord używa pól bezpośrednich ADIF — jedynego bezstratnego nośnika listy.
    #[must_use]
    pub fn pota_representation(&self) -> PotaRepresentation {
        if self.pota.is_empty() {
            PotaRepresentation::Absent
        } else if self.pota.any_multiple() {
            PotaRepresentation::DirectFields
        } else {
            PotaRepresentation::Sig
        }
    }

    /// Centralne mapowanie referencji na pola ADIF (eksport).
    ///
    /// Kolejność jest deterministyczna. Puste wartości nie są emitowane. Rekord
    /// nigdy nie zawiera dwóch reprezentacji POTA jednocześnie.
    #[must_use]
    pub fn adif_fields(&self) -> Vec<(&'static str, String)> {
        let mut fields: Vec<(&'static str, String)> = Vec::new();
        if let Some(value) = self.sota.local.as_deref() {
            fields.push((
                AwardProgram::Sota.direct_field(StationSide::Local),
                value.to_string(),
            ));
        }
        if let Some(value) = self.sota.remote.as_deref() {
            fields.push((
                AwardProgram::Sota.direct_field(StationSide::Remote),
                value.to_string(),
            ));
        }
        match self.pota_representation() {
            PotaRepresentation::Absent => {}
            PotaRepresentation::Sig => {
                for side in StationSide::ALL {
                    let Some(value) = self.pota.side(side) else {
                        continue;
                    };
                    let (sig_tag, info_tag) = side.sig_fields();
                    fields.push((sig_tag, SIG_POTA.to_string()));
                    fields.push((info_tag, value.to_string()));
                }
            }
            PotaRepresentation::DirectFields => {
                for side in StationSide::ALL {
                    let Some(value) = self.pota.side(side) else {
                        continue;
                    };
                    fields.push((AwardProgram::Pota.direct_field(side), value.to_string()));
                }
            }
        }
        fields
    }
}

impl QsoRecord {
    /// Zwraca jawny model referencji tego rekordu.
    #[must_use]
    pub fn award_references(&self) -> AwardReferences {
        AwardReferences::from_qso(self)
    }

    /// Zapisuje model referencji do rekordu.
    pub fn apply_award_references(&mut self, references: &AwardReferences) {
        references.apply_to_qso(self);
    }
}

/// Uwaga importera o referencji, której nie da się przypisać do programu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AwardRefNotice {
    /// `SIG`/`MY_SIG` wskazuje program inny niż POTA — referencja pozostaje nieinterpretowana.
    UnknownSig {
        /// Strona, której dotyczy pole.
        side: StationSide,
        /// Wartość pola `SIG`/`MY_SIG`.
        sig: String,
    },
    /// `SIG_INFO`/`MY_SIG_INFO` bez `SIG`/`MY_SIG` — nie wiadomo, jakiego programu dotyczy.
    ReferenceWithoutSig {
        /// Strona, której dotyczy pole.
        side: StationSide,
    },
    /// `SIG=POTA` bez `SIG_INFO` — brak referencji parku.
    PotaSigWithoutReference {
        /// Strona, której dotyczy pole.
        side: StationSide,
    },
}

impl AwardRefNotice {
    /// Czytelny opis problemu (komunikat raportu importu, nie tekst GUI).
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::UnknownSig { side, sig } => format!(
                "pole SIG={sig} po stronie {} nie jest obsługiwanym programem; referencja pozostaje nieinterpretowana",
                side.label()
            ),
            Self::ReferenceWithoutSig { side } => format!(
                "pole SIG_INFO po stronie {} bez pola SIG — nie można ustalić programu; wartość nie jest interpretowana",
                side.label()
            ),
            Self::PotaSigWithoutReference { side } => format!(
                "pole SIG=POTA po stronie {} bez SIG_INFO — brak referencji parku",
                side.label()
            ),
        }
    }
}

/// Konflikt dwóch reprezentacji tej samej referencji (ta sama strona, różne wartości).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwardRefConflict {
    /// Program, którego dotyczy konflikt.
    pub program: AwardProgram,
    /// Strona QSO.
    pub side: StationSide,
    /// Nazwa pola bezpośredniego.
    pub direct_field: &'static str,
    /// Wartość pola bezpośredniego.
    pub direct_value: String,
    /// Nazwa pola `SIG_INFO`/`MY_SIG_INFO`.
    pub sig_field: &'static str,
    /// Wartość pola `SIG_INFO`/`MY_SIG_INFO`.
    pub sig_value: String,
}

impl AwardRefConflict {
    /// Czytelny opis konfliktu (komunikat raportu importu, nie tekst GUI).
    #[must_use]
    pub fn message(&self) -> String {
        format!(
            "sprzeczne reprezentacje {} po stronie {}: {}={:?} vs {}={:?} — rekord niejednoznaczny, żadna wartość nie jest wybierana arbitralnie",
            self.program.label(),
            self.side.label(),
            self.direct_field,
            self.direct_value,
            self.sig_field,
            self.sig_value
        )
    }
}

impl std::fmt::Display for AwardRefConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for AwardRefConflict {}

/// Wynik odczytu referencji z pól ADIF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdifReferenceImport {
    /// Referencje rozdzielone na program i stronę.
    pub references: AwardReferences,
    /// Uwagi o danych, których nie da się przypisać do programu.
    pub notices: Vec<AwardRefNotice>,
}

/// Odczytuje i usuwa z mapy pól ADIF wszystkie pola referencji dyplomowych.
///
/// Funkcja jest centralnym importerem referencji: rozpoznaje pola bezpośrednie
/// (`MY_SOTA_REF`, `SOTA_REF`, `MY_POTA_REF`, `POTA_REF`) oraz reprezentację SIG
/// (`MY_SIG`, `MY_SIG_INFO`, `SIG`, `SIG_INFO`). Zgodne reprezentacje POTA są
/// scalane bez utraty wartości; sprzeczne kończą się [`AwardRefConflict`].
///
/// # Errors
///
/// Zwraca [`AwardRefConflict`], gdy ta sama strona ma różne wartości w polu
/// bezpośrednim i w polu `SIG_INFO`.
pub fn import_adif_references<S: std::hash::BuildHasher>(
    fields: &mut HashMap<String, String, S>,
) -> Result<AdifReferenceImport, AwardRefConflict> {
    let mut references = AwardReferences::default();
    let mut notices = Vec::new();

    references.sota.local = fields.remove("MY_SOTA_REF").and_then(|v| clean_value(&v));
    references.sota.remote = fields.remove("SOTA_REF").and_then(|v| clean_value(&v));

    let direct_local = fields.remove("MY_POTA_REF").and_then(|v| clean_value(&v));
    let direct_remote = fields.remove("POTA_REF").and_then(|v| clean_value(&v));

    let sig_local = read_sig_side(fields, StationSide::Local, &mut notices);
    let sig_remote = read_sig_side(fields, StationSide::Remote, &mut notices);

    references.pota.local = merge_pota_side(StationSide::Local, direct_local, sig_local)?;
    references.pota.remote = merge_pota_side(StationSide::Remote, direct_remote, sig_remote)?;

    Ok(AdifReferenceImport {
        references,
        notices,
    })
}

/// Odczytuje i usuwa z mapy parę `(MY_SIG, MY_SIG_INFO)` / `(SIG, SIG_INFO)`.
fn read_sig_side<S: std::hash::BuildHasher>(
    fields: &mut HashMap<String, String, S>,
    side: StationSide,
    notices: &mut Vec<AwardRefNotice>,
) -> Option<String> {
    let (sig_tag, info_tag) = side.sig_fields();
    let sig = fields.remove(sig_tag).and_then(|v| clean_value(&v));
    let info = fields.remove(info_tag).and_then(|v| clean_value(&v));

    match (sig, info) {
        (Some(sig), info) => {
            if !sig.eq_ignore_ascii_case(SIG_POTA) {
                notices.push(AwardRefNotice::UnknownSig { side, sig });
                return None;
            }
            if info.is_none() {
                notices.push(AwardRefNotice::PotaSigWithoutReference { side });
            }
            info
        }
        (None, Some(_)) => {
            notices.push(AwardRefNotice::ReferenceWithoutSig { side });
            None
        }
        (None, None) => None,
    }
}

/// Scala wartość pola bezpośredniego i wartość SIG po jednej stronie.
fn merge_pota_side(
    side: StationSide,
    direct: Option<String>,
    sig: Option<String>,
) -> Result<Option<String>, AwardRefConflict> {
    match (direct, sig) {
        (None, None) => Ok(None),
        (Some(value), None) | (None, Some(value)) => Ok(Some(value)),
        (Some(direct_value), Some(sig_value)) => {
            if reference_lists_equal(&direct_value, &sig_value) {
                Ok(Some(direct_value))
            } else {
                let (_, info_tag) = side.sig_fields();
                Err(AwardRefConflict {
                    program: AwardProgram::Pota,
                    side,
                    direct_field: AwardProgram::Pota.direct_field(side),
                    direct_value,
                    sig_field: info_tag,
                    sig_value,
                })
            }
        }
    }
}

/// Normalizuje pojedynczą wartość referencji: `trim`, a pusty napis staje się `None`.
///
/// Wielkość liter nie jest zmieniana — ADIF deklaruje `POTARef` jako
/// case-insensitive, a dla `SOTARef` nie definiuje konwencji, więc przepisywanie
/// wartości byłoby zgadywaniem.
fn clean_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Normalizuje opcjonalną wartość referencji (patrz [`clean_value`]).
fn clean(value: Option<String>) -> Option<String> {
    value.and_then(|v| clean_value(&v))
}

/// Dzieli `POTARefList` na pojedyncze referencje (bez zmiany ich treści).
#[must_use]
pub fn split_reference_list(value: &str) -> Vec<&str> {
    value
        .split(POTA_REF_LIST_SEPARATOR)
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

/// Czy wartość niesie więcej niż jedną referencję listy.
#[must_use]
pub fn is_multi_reference(value: &str) -> bool {
    split_reference_list(value).len() > 1
}

/// Porównuje dwie listy referencji bez względu na kolejność i wielkość liter.
#[must_use]
pub fn reference_lists_equal(left: &str, right: &str) -> bool {
    let mut a: Vec<String> = split_reference_list(left)
        .into_iter()
        .map(str::to_ascii_uppercase)
        .collect();
    let mut b: Vec<String> = split_reference_list(right)
        .into_iter()
        .map(str::to_ascii_uppercase)
        .collect();
    a.sort_unstable();
    b.sort_unstable();
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso_with(
        my_sota: Option<&str>,
        sota: Option<&str>,
        my_pota: Option<&str>,
        pota: Option<&str>,
    ) -> QsoRecord {
        let mut qso = QsoRecord::new("TSTREMOTE", "20m", "CW");
        qso.my_sota_ref = my_sota.map(str::to_string);
        qso.sota_ref = sota.map(str::to_string);
        qso.my_pota_ref = my_pota.map(str::to_string);
        qso.pota_ref = pota.map(str::to_string);
        qso
    }

    fn field_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn sota_mapping_keeps_sides() {
        let qso = qso_with(Some("AA/BB-001"), Some("CC/DD-999"), None, None);
        let fields = qso.award_references().adif_fields();
        assert_eq!(
            fields,
            vec![
                ("MY_SOTA_REF", "AA/BB-001".to_string()),
                ("SOTA_REF", "CC/DD-999".to_string())
            ]
        );
    }

    #[test]
    fn sota_local_only_never_reaches_remote_field() {
        let qso = qso_with(Some("AA/BB-001"), None, None, None);
        let fields = qso.award_references().adif_fields();
        assert_eq!(fields, vec![("MY_SOTA_REF", "AA/BB-001".to_string())]);
        assert!(!fields.iter().any(|(tag, _)| *tag == "SOTA_REF"));
    }

    #[test]
    fn sota_remote_only_never_reaches_local_field() {
        let qso = qso_with(None, Some("CC/DD-999"), None, None);
        let fields = qso.award_references().adif_fields();
        assert_eq!(fields, vec![("SOTA_REF", "CC/DD-999".to_string())]);
        assert!(!fields.iter().any(|(tag, _)| *tag == "MY_SOTA_REF"));
    }

    #[test]
    fn empty_references_produce_no_fields() {
        let qso = qso_with(None, None, None, None);
        assert_eq!(qso.award_references().adif_fields().len(), 0);
        assert_eq!(qso.award_references(), AwardReferences::default());
    }

    #[test]
    fn pota_single_reference_uses_sig_contract() {
        let qso = qso_with(None, None, Some("XX-0001"), Some("YY-9999"));
        let fields = qso.award_references().adif_fields();
        assert_eq!(
            fields,
            vec![
                ("MY_SIG", "POTA".to_string()),
                ("MY_SIG_INFO", "XX-0001".to_string()),
                ("SIG", "POTA".to_string()),
                ("SIG_INFO", "YY-9999".to_string()),
            ]
        );
    }

    #[test]
    fn pota_local_only_has_no_remote_sig_pair() {
        let qso = qso_with(None, None, Some("XX-0001"), None);
        let fields = qso.award_references().adif_fields();
        assert_eq!(
            fields,
            vec![
                ("MY_SIG", "POTA".to_string()),
                ("MY_SIG_INFO", "XX-0001".to_string()),
            ]
        );
        assert!(!fields.iter().any(|(tag, _)| *tag == "SIG"));
        assert!(!fields.iter().any(|(tag, _)| *tag == "SIG_INFO"));
    }

    #[test]
    fn pota_sig_is_never_emitted_without_reference() {
        for qso in [
            qso_with(None, None, None, None),
            qso_with(None, None, Some("XX-0001"), None),
            qso_with(None, None, None, Some("YY-9999")),
        ] {
            let pairs = qso.award_references().adif_fields();
            for (sig_tag, info_tag) in [("MY_SIG", "MY_SIG_INFO"), ("SIG", "SIG_INFO")] {
                let has_sig = pairs.iter().any(|(t, _)| *t == sig_tag);
                let has_info = pairs.iter().any(|(t, _)| *t == info_tag);
                assert_eq!(
                    has_sig, has_info,
                    "{sig_tag} must be paired with {info_tag}"
                );
            }
        }
    }

    #[test]
    fn pota_multi_reference_uses_direct_fields_only() {
        let qso = qso_with(None, None, Some("XX-0001,XX-0002"), Some("YY-9999"));
        let refs = qso.award_references();
        assert_eq!(refs.pota_representation(), PotaRepresentation::DirectFields);
        let fields = refs.adif_fields();
        assert_eq!(
            fields,
            vec![
                ("MY_POTA_REF", "XX-0001,XX-0002".to_string()),
                ("POTA_REF", "YY-9999".to_string()),
            ]
        );
        assert!(!fields.iter().any(|(tag, _)| tag.starts_with("SIG")));
        assert!(!fields.iter().any(|(tag, _)| *tag == "MY_SIG"));
    }

    #[test]
    fn exactly_one_pota_representation_is_emitted() {
        for (my, remote) in [
            (Some("XX-0001"), None),
            (None, Some("YY-9999")),
            (Some("XX-0001"), Some("YY-9999")),
            (Some("XX-0001,XX-0002"), Some("YY-9999")),
        ] {
            let qso = qso_with(None, None, my, remote);
            let refs = qso.award_references();
            let fields = refs.adif_fields();
            let direct = fields
                .iter()
                .any(|(tag, _)| *tag == "MY_POTA_REF" || *tag == "POTA_REF");
            let sig = fields
                .iter()
                .any(|(tag, _)| *tag == "MY_SIG" || *tag == "SIG");
            assert!(direct ^ sig, "exactly one representation, got {fields:?}");
        }
    }

    #[test]
    fn import_accepts_direct_fields() {
        let mut fields = field_map(&[
            ("MY_SOTA_REF", "AA/BB-001"),
            ("SOTA_REF", "CC/DD-999"),
            ("MY_POTA_REF", "XX-0001"),
            ("POTA_REF", "YY-9999"),
        ]);
        let imported = import_adif_references(&mut fields).expect("no conflict");
        assert_eq!(imported.references.sota.local.as_deref(), Some("AA/BB-001"));
        assert_eq!(
            imported.references.sota.remote.as_deref(),
            Some("CC/DD-999")
        );
        assert_eq!(imported.references.pota.local.as_deref(), Some("XX-0001"));
        assert_eq!(imported.references.pota.remote.as_deref(), Some("YY-9999"));
        assert_eq!(imported.notices.len(), 0);
        assert_eq!(fields.len(), 0, "all reference fields must be consumed");
    }

    #[test]
    fn import_accepts_sig_representation() {
        let mut fields = field_map(&[
            ("MY_SIG", "POTA"),
            ("MY_SIG_INFO", "XX-0001"),
            ("SIG", "POTA"),
            ("SIG_INFO", "YY-9999"),
        ]);
        let imported = import_adif_references(&mut fields).expect("no conflict");
        assert_eq!(imported.references.pota.local.as_deref(), Some("XX-0001"));
        assert_eq!(imported.references.pota.remote.as_deref(), Some("YY-9999"));
        assert_eq!(imported.notices.len(), 0);
        assert_eq!(fields.len(), 0);
    }

    #[test]
    fn import_merges_consistent_representations() {
        let mut fields = field_map(&[
            ("MY_POTA_REF", "XX-0001"),
            ("MY_SIG", "POTA"),
            ("MY_SIG_INFO", "xx-0001"),
        ]);
        let imported = import_adif_references(&mut fields).expect("consistent");
        assert_eq!(imported.references.pota.local.as_deref(), Some("XX-0001"));
        assert_eq!(imported.references.pota.remote, None);
        assert_eq!(imported.notices.len(), 0);
    }

    #[test]
    fn import_merges_consistent_lists_regardless_of_order() {
        let mut fields = field_map(&[
            ("POTA_REF", "YY-9999,YY-0001"),
            ("SIG", "POTA"),
            ("SIG_INFO", "yy-0001,YY-9999"),
        ]);
        let imported = import_adif_references(&mut fields).expect("consistent set");
        assert_eq!(
            imported.references.pota.remote.as_deref(),
            Some("YY-9999,YY-0001")
        );
    }

    #[test]
    fn import_rejects_contradictory_representations() {
        let mut fields = field_map(&[
            ("MY_POTA_REF", "XX-0001"),
            ("MY_SIG", "POTA"),
            ("MY_SIG_INFO", "XX-0002"),
        ]);
        let conflict = import_adif_references(&mut fields).expect_err("must conflict");
        assert_eq!(conflict.program, AwardProgram::Pota);
        assert_eq!(conflict.side, StationSide::Local);
        assert_eq!(conflict.direct_value, "XX-0001");
        assert_eq!(conflict.sig_value, "XX-0002");
        assert!(conflict.message().contains("XX-0001"));
        assert!(conflict.message().contains("XX-0002"));
    }

    #[test]
    fn import_reports_unknown_and_orphan_sig_without_guessing() {
        let mut fields = field_map(&[("SIG", "WWFF"), ("SIG_INFO", "SPFF-0001")]);
        let imported = import_adif_references(&mut fields).expect("no conflict");
        assert_eq!(imported.references.pota.remote, None);
        assert_eq!(
            imported.notices,
            vec![AwardRefNotice::UnknownSig {
                side: StationSide::Remote,
                sig: "WWFF".to_string()
            }]
        );

        let mut orphan = field_map(&[("MY_SIG_INFO", "XX-0001")]);
        let imported = import_adif_references(&mut orphan).expect("no conflict");
        assert_eq!(imported.references.pota.local, None);
        assert_eq!(
            imported.notices,
            vec![AwardRefNotice::ReferenceWithoutSig {
                side: StationSide::Local
            }]
        );

        let mut missing_info = field_map(&[("SIG", "POTA")]);
        let imported = import_adif_references(&mut missing_info).expect("no conflict");
        assert_eq!(imported.references.pota.remote, None);
        assert_eq!(
            imported.notices,
            vec![AwardRefNotice::PotaSigWithoutReference {
                side: StationSide::Remote
            }]
        );
    }

    #[test]
    fn import_is_case_insensitive_for_the_pota_sig_marker_only() {
        // The ADIF parser upper-cases field names, so the map keys are upper-case;
        // the SIG *value* comparison is case-insensitive.
        let mut fields = field_map(&[("MY_SIG", "pota"), ("MY_SIG_INFO", "XX-0001")]);
        let imported = import_adif_references(&mut fields).expect("no conflict");
        assert_eq!(imported.references.pota.local.as_deref(), Some("XX-0001"));

        // A lower-cased SIG value that is not POTA is not interpreted as a park.
        let mut other = field_map(&[("MY_SIG", "wwff"), ("MY_SIG_INFO", "SPFF-0001")]);
        let imported = import_adif_references(&mut other).expect("no conflict");
        assert_eq!(imported.references.pota.local, None);
        assert_eq!(imported.notices.len(), 1);
    }

    #[test]
    fn import_preserves_case_of_reference_values() {
        let mut fields = field_map(&[("MY_SOTA_REF", "aa/bb-001"), ("POTA_REF", "xx-0001")]);
        let imported = import_adif_references(&mut fields).expect("no conflict");
        assert_eq!(imported.references.sota.local.as_deref(), Some("aa/bb-001"));
        assert_eq!(imported.references.pota.remote.as_deref(), Some("xx-0001"));
    }

    #[test]
    fn import_treats_empty_values_as_missing() {
        let mut fields = field_map(&[("MY_SOTA_REF", "  "), ("SOTA_REF", ""), ("MY_POTA_REF", "")]);
        let imported = import_adif_references(&mut fields).expect("no conflict");
        assert_eq!(imported.references, AwardReferences::default());
    }

    #[test]
    fn apply_to_qso_never_crosses_sides() {
        let mut qso = QsoRecord::new("TSTREMOTE", "20m", "CW");
        let refs = AwardReferences {
            sota: AwardRefPair::new(Some("AA/BB-001".into()), Some("CC/DD-999".into())),
            pota: AwardRefPair::new(Some("XX-0001".into()), Some("YY-9999".into())),
        };
        refs.apply_to_qso(&mut qso);
        assert_eq!(qso.my_sota_ref.as_deref(), Some("AA/BB-001"));
        assert_eq!(qso.sota_ref.as_deref(), Some("CC/DD-999"));
        assert_eq!(qso.my_pota_ref.as_deref(), Some("XX-0001"));
        assert_eq!(qso.pota_ref.as_deref(), Some("YY-9999"));
    }

    #[test]
    fn round_trip_export_import_export_is_stable() {
        for (my_sota, sota, my_pota, pota) in [
            (None, None, None, None),
            (Some("AA/BB-001"), None, None, None),
            (None, Some("CC/DD-999"), None, None),
            (
                Some("AA/BB-001"),
                Some("CC/DD-999"),
                Some("XX-0001"),
                Some("YY-9999"),
            ),
            (
                Some("AA/BB-001"),
                Some("CC/DD-999"),
                Some("XX-0001,XX-0002"),
                Some("YY-9999"),
            ),
        ] {
            let qso = qso_with(my_sota, sota, my_pota, pota);
            let first = qso.award_references().adif_fields();
            let mut map: HashMap<String, String> = first
                .iter()
                .map(|(tag, value)| ((*tag).to_string(), value.clone()))
                .collect();
            let imported = import_adif_references(&mut map).expect("no conflict");
            let second = imported.references.adif_fields();
            assert_eq!(first, second, "export→import→export must be stable");
            assert_eq!(map.len(), 0);
        }
    }

    #[test]
    fn reference_list_helpers() {
        assert_eq!(split_reference_list("XX-0001"), vec!["XX-0001"]);
        assert_eq!(
            split_reference_list(" XX-0001 , XX-0002 ,"),
            vec!["XX-0001", "XX-0002"]
        );
        assert!(!is_multi_reference("XX-0001"));
        assert!(is_multi_reference("XX-0001,XX-0002"));
        assert!(reference_lists_equal("a,b", "B,A"));
        assert!(!reference_lists_equal("a,b", "a,c"));
        assert!(!reference_lists_equal("a", "a,b"));
    }

    #[test]
    fn pair_counts_and_multiple_detection() {
        let pair = AwardRefPair::new(Some("XX-0001,XX-0002".into()), Some("YY-9999".into()));
        assert_eq!(pair.count(StationSide::Local), 2);
        assert_eq!(pair.count(StationSide::Remote), 1);
        assert!(pair.any_multiple());

        let empty = AwardRefPair::default();
        assert_eq!(empty.count(StationSide::Local), 0);
        assert!(!empty.any_multiple());
        assert_eq!(empty, AwardRefPair::default());
    }

    #[test]
    fn program_and_side_accessors_are_symmetric() {
        let mut refs = AwardReferences::default();
        for program in AwardProgram::ALL {
            for side in StationSide::ALL {
                refs.program_mut(program).set_side(
                    side,
                    Some(format!("{}-{}-0001", program.label(), side.label())),
                );
            }
        }
        for program in AwardProgram::ALL {
            for side in StationSide::ALL {
                let expected = format!("{}-{}-0001", program.label(), side.label());
                assert_eq!(refs.program(program).side(side), Some(expected.as_str()));
            }
        }
    }
}
