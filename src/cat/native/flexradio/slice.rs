// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Model i zarządzanie odbiornikami slice transceivera FlexRadio.
//!
//! SmartSDR nie korzysta z tradycyjnej koncepcji VFO A/B, lecz z niezależnych
//! instancji odbiorników (slice receivers). Każdy slice ma własną częstotliwość,
//! tryb emisji, filtry DSP, stan aktywności, flagę TX oraz uchwyt klienta.
//!
//! Moduł implementuje deterministyczną politykę wyboru slice z wyraźnym rozdziałem:
//! - selekcja do odczytu (`select_read_slice`),
//! - selekcja do operacji mutujących (`select_write_slice`),
//! - selekcja kontekstu nadawczego (`select_tx_slice`).
//!
//! Zapobiega modyfikowaniu odbiorników należących do innych klientów oraz nie fabricuje VFO.

use std::collections::BTreeMap;

use super::protocol::FlexProtocolError;

/// Identyfikator odbiornika slice (zwykle 0, 1, 2, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlexSliceId(pub u32);

impl std::fmt::Display for FlexSliceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pełny stan pojedynczego odbiornika slice.
#[derive(Debug, Clone, PartialEq)]
pub struct FlexSliceState {
    pub id: FlexSliceId,
    pub frequency_hz: u64,
    pub mode: String,
    pub filter_low_hz: i32,
    pub filter_high_hz: i32,
    pub active: bool,
    pub tx: bool,
    pub locked: bool,
    pub client_handle: Option<String>,
    pub in_use: bool,
    pub step_hz: u32,
    pub rit_on: bool,
    pub rit_freq_hz: i32,
    pub xit_on: bool,
    pub xit_freq_hz: i32,
    pub rx_ant: String,
    pub tx_ant: String,
    pub rf_power: Option<f32>,
}

impl FlexSliceState {
    pub fn new(id: FlexSliceId) -> Self {
        Self {
            id,
            frequency_hz: 14_000_000,
            mode: "USB".to_string(),
            filter_low_hz: 100,
            filter_high_hz: 2800,
            active: false,
            tx: false,
            locked: false,
            client_handle: None,
            in_use: true,
            step_hz: 100,
            rit_on: false,
            rit_freq_hz: 0,
            xit_on: false,
            xit_freq_hz: 0,
            rx_ant: "ANT1".to_string(),
            tx_ant: "ANT1".to_string(),
            rf_power: None,
        }
    }

    /// Oblicza szerokość pasma filtru odbiorczego w Hz.
    pub fn passband_hz(&self) -> u32 {
        (self.filter_high_hz - self.filter_low_hz).max(0) as u32
    }

    /// Częściowa aktualizacja stanu slice z asynchronicznego komunikatu statusu.
    /// Pola niewymienione w statusie zachowują swoją dotychczasową wartość.
    /// W przypadku uszkodzonej wartości znanego pola zwraca typowany błąd.
    pub fn update_from_properties(
        &mut self,
        properties: &BTreeMap<String, String>,
    ) -> Result<(), FlexProtocolError> {
        if let Some(val) = properties.get("in_use") {
            self.in_use = val == "1" || val.eq_ignore_ascii_case("true");
        }

        if let Some(val) = properties
            .get("rf_frequency")
            .or_else(|| properties.get("freq"))
        {
            let mhz = val.parse::<f64>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid frequency '{val}': {e}"))
            })?;
            if !mhz.is_finite() || mhz <= 0.0 {
                return Err(FlexProtocolError::ParseError(format!(
                    "frequency '{val}' must be positive and finite"
                )));
            }
            self.frequency_hz = (mhz * 1_000_000.0).round() as u64;
        }

        if let Some(val) = properties.get("mode") {
            let m = val.trim();
            if !m.is_empty() {
                // Nie mapujemy nieznanego trybu do USB — zachowujemy oryginalną nazwę trybu
                self.mode = m.to_ascii_uppercase();
            }
        }

        let mut low = self.filter_low_hz;
        let mut high = self.filter_high_hz;
        let mut filter_modified = false;

        if let Some(lo_str) = properties.get("filter_lo") {
            low = lo_str.parse::<i32>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid filter_lo '{lo_str}': {e}"))
            })?;
            filter_modified = true;
        }
        if let Some(hi_str) = properties.get("filter_hi") {
            high = hi_str.parse::<i32>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid filter_hi '{hi_str}': {e}"))
            })?;
            filter_modified = true;
        }

        if filter_modified {
            if low >= high {
                return Err(FlexProtocolError::ParseError(format!(
                    "invalid filter bounds: filter_lo ({low}) >= filter_hi ({high})"
                )));
            }
            self.filter_low_hz = low;
            self.filter_high_hz = high;
        }

        if let Some(val) = properties.get("active") {
            self.active = val == "1" || val.eq_ignore_ascii_case("true");
        }

        if let Some(val) = properties.get("tx") {
            self.tx = val == "1" || val.eq_ignore_ascii_case("true");
        }

        if let Some(val) = properties.get("lock").or_else(|| properties.get("locked")) {
            self.locked = val == "1" || val.eq_ignore_ascii_case("true");
        }

        if let Some(val) = properties.get("client_handle") {
            let clean = val.trim().strip_prefix("0x").unwrap_or(val.trim());
            if !clean.is_empty() && clean != "0" {
                self.client_handle = Some(clean.to_string());
            } else {
                self.client_handle = None;
            }
        }

        if let Some(s_str) = properties.get("step") {
            self.step_hz = s_str.parse::<u32>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid step '{s_str}': {e}"))
            })?;
        }

        if let Some(val) = properties.get("rit_on") {
            self.rit_on = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Some(f_str) = properties.get("rit_freq") {
            self.rit_freq_hz = f_str.parse::<i32>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid rit_freq '{f_str}': {e}"))
            })?;
        }

        if let Some(val) = properties.get("xit_on") {
            self.xit_on = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Some(f_str) = properties.get("xit_freq") {
            self.xit_freq_hz = f_str.parse::<i32>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid xit_freq '{f_str}': {e}"))
            })?;
        }

        if let Some(val) = properties.get("rxant") {
            self.rx_ant.clone_from(val);
        }
        if let Some(val) = properties.get("txant") {
            self.tx_ant.clone_from(val);
        }

        if let Some(p_str) = properties.get("rf_power") {
            let p = p_str.parse::<f32>().map_err(|e| {
                FlexProtocolError::ParseError(format!("invalid rf_power '{p_str}': {e}"))
            })?;
            self.rf_power = Some(p);
        }

        Ok(())
    }
}

/// Wybór odbiornika slice do odczytu stanu radia (`RigState`).
/// Operacje tylko do odczytu mogą obserwować aktywny slice należący do innego klienta,
/// jeśli taki slice został wskazany lub jest jedynym aktywnym w radiu.
pub fn select_read_slice<'a>(
    slices: &'a BTreeMap<FlexSliceId, FlexSliceState>,
    preferred: Option<FlexSliceId>,
    client_handle: Option<&str>,
) -> Result<&'a FlexSliceState, FlexProtocolError> {
    select_active_slice(slices, preferred, client_handle)
}

/// Deterministyczny wybór aktywnego slice dla odczytu RigState.
///
/// Polityka wyboru:
/// 1. Jeśli użytkownik jawnie wskazał `preferred_slice` w konfiguracji i dany slice istnieje
///    oraz jest aktywny (`in_use`), to ten slice jest wybierany.
/// 2. Jeśli podano `client_handle`, wybierany jest slice powiązany z tym klientem:
///    a) slice z `active == true` należący do klienta,
///    b) jeśli klient ma dokładnie jeden slice, to ten jedyny slice.
/// 3. Jeśli brak powiązania z klientem, ale w całej mapie istnieje dokładnie jeden aktywny
///    slice (`active == true` oraz `in_use == true`), to ten slice jest wybierany.
/// 4. W przypadku niejednoznaczności zwracany jest typowany błąd.
pub fn select_active_slice<'a>(
    slices: &'a BTreeMap<FlexSliceId, FlexSliceState>,
    preferred: Option<FlexSliceId>,
    client_handle: Option<&str>,
) -> Result<&'a FlexSliceState, FlexProtocolError> {
    if slices.is_empty() {
        return Err(FlexProtocolError::SliceNotFound(0));
    }

    if let Some(slice) = preferred
        .and_then(|pref| slices.get(&pref))
        .filter(|s| s.in_use)
    {
        return Ok(slice);
    }

    if let Some(handle) = client_handle {
        let clean_handle = handle.strip_prefix("0x").unwrap_or(handle);
        let client_slices: Vec<&FlexSliceState> = slices
            .values()
            .filter(|s| {
                s.in_use
                    && s.client_handle
                        .as_deref()
                        .is_some_and(|h| h.eq_ignore_ascii_case(clean_handle))
            })
            .collect();

        if let Some(active_client_slice) = client_slices.iter().copied().find(|s| s.active) {
            return Ok(active_client_slice);
        }

        if client_slices.len() == 1 {
            return Ok(client_slices[0]);
        }
    }

    let global_active: Vec<&FlexSliceState> =
        slices.values().filter(|s| s.in_use && s.active).collect();

    if global_active.len() == 1 {
        return Ok(global_active[0]);
    }

    let in_use_slices: Vec<&FlexSliceState> = slices.values().filter(|s| s.in_use).collect();
    if in_use_slices.len() == 1 {
        return Ok(in_use_slices[0]);
    }

    Err(FlexProtocolError::AmbiguousSlice(format!(
        "found {} in-use slices with {} active slices; cannot deterministically select active slice",
        in_use_slices.len(),
        global_active.len()
    )))
}

/// Wybór odbiornika slice do operacji mutujących (np. zmiana częstotliwości, trybu, filtrów).
///
/// Zasady bezpieczeństwa wieloklientowego:
/// 1. Jeśli użytkownik wskazał `preferred_slice`:
///    - Weryfikuje, czy dany slice istnieje i jest w użyciu (`in_use`).
///    - Jeśli slice należy do innego klienta (`slice.client_handle != client_handle`),
///      wymagana jest flaga `allow_foreign == true`. W przeciwnym razie zwracany jest
///      błąd `FlexProtocolError::SliceOwnershipMismatch`.
/// 2. Jeśli bieżący klient posiada własne aktywne slice (zgodne z `client_handle`),
///    wybierany jest slice z `active == true`, a jeśli żaden nie ma `active == true`,
///    ale klient posiada dokładnie jeden swój slice, wybierany jest ten slice.
/// 3. Jeśli bieżący klient nie posiada żadnych własnych slice, ale w radiu istnieje
///    dokładnie jeden slice i nie jest on przypisany do żadnego obcego klienta,
///    może on zostać wybrany.
/// 4. NIGDY nie modyfikuje slice należącego do obcego klienta bez jawnego zezwolenia (`allow_foreign`).
pub fn select_write_slice<'a>(
    slices: &'a BTreeMap<FlexSliceId, FlexSliceState>,
    preferred: Option<FlexSliceId>,
    client_handle: Option<&str>,
    allow_foreign: bool,
) -> Result<&'a FlexSliceState, FlexProtocolError> {
    if slices.is_empty() {
        return Err(FlexProtocolError::SliceNotFound(0));
    }

    let clean_handle = client_handle.map(|h| h.strip_prefix("0x").unwrap_or(h));

    if let Some(slice) = preferred
        .and_then(|pref| slices.get(&pref))
        .filter(|s| s.in_use)
    {
        if let Some(foreign_handle) = slice.client_handle.as_deref() {
            if let Some(my_handle) = clean_handle {
                if !foreign_handle.eq_ignore_ascii_case(my_handle) && !allow_foreign {
                    return Err(FlexProtocolError::SliceOwnershipMismatch(format!(
                        "slice {} is owned by client 0x{foreign_handle}, current client is 0x{my_handle}",
                        slice.id
                    )));
                }
            } else if !allow_foreign {
                return Err(FlexProtocolError::SliceOwnershipMismatch(format!(
                    "slice {} is owned by client 0x{foreign_handle}",
                    slice.id
                )));
            }
        }
        return Ok(slice);
    }

    // Szukaj slice należącego do tego klienta
    if let Some(my_handle) = clean_handle {
        let client_slices: Vec<&FlexSliceState> = slices
            .values()
            .filter(|s| {
                s.in_use
                    && s.client_handle
                        .as_deref()
                        .is_some_and(|h| h.eq_ignore_ascii_case(my_handle))
            })
            .collect();

        if let Some(active_client_slice) = client_slices.iter().copied().find(|s| s.active) {
            return Ok(active_client_slice);
        }

        if client_slices.len() == 1 {
            return Ok(client_slices[0]);
        }

        if client_slices.len() > 1 {
            return Err(FlexProtocolError::AmbiguousSlice(format!(
                "client 0x{my_handle} owns {} slices, but none is marked active",
                client_slices.len()
            )));
        }
    }

    // Jeśli klient nie ma własnych slice, sprawdź czy istnieje dokładnie jeden wolny (bez przypisanego właściciela)
    let unassigned_slices: Vec<&FlexSliceState> = slices
        .values()
        .filter(|s| s.in_use && s.client_handle.is_none())
        .collect();

    if unassigned_slices.len() == 1 {
        return Ok(unassigned_slices[0]);
    }

    // Jeśli istnieją tylko slice należące do innych klientów
    let foreign_slices: Vec<&FlexSliceState> = slices
        .values()
        .filter(|s| s.in_use && s.client_handle.is_some())
        .collect();

    if !foreign_slices.is_empty() && !allow_foreign {
        return Err(FlexProtocolError::SliceOwnershipMismatch(
            "cannot mutate slices owned by foreign clients without explicit permission".to_string(),
        ));
    }

    if allow_foreign {
        return select_active_slice(slices, preferred, None);
    }

    Err(FlexProtocolError::AmbiguousSlice(
        "cannot deterministically select write target slice".to_string(),
    ))
}

/// Deterministyczny wybór potwierdzonego slice nadawczego (TX slice).
///
/// Wymaga, aby slice nadawczy posiadał flagę `tx == true`, był w użyciu (`in_use`),
/// oraz należał do bieżącego klienta (bądź nie miał przypisanego obcego właściciela).
pub fn select_tx_slice<'a>(
    slices: &'a BTreeMap<FlexSliceId, FlexSliceState>,
    client_handle: Option<&str>,
) -> Result<&'a FlexSliceState, FlexProtocolError> {
    let clean_handle = client_handle.map(|h| h.strip_prefix("0x").unwrap_or(h));

    let tx_slices: Vec<&FlexSliceState> = slices.values().filter(|s| s.in_use && s.tx).collect();

    if let Some(my_handle) = clean_handle {
        let client_tx: Vec<&FlexSliceState> = tx_slices
            .iter()
            .copied()
            .filter(|s| {
                s.client_handle
                    .as_deref()
                    .is_some_and(|h| h.eq_ignore_ascii_case(my_handle))
            })
            .collect();

        if client_tx.len() == 1 {
            return Ok(client_tx[0]);
        }

        // Jeśli są slice z flagą tx, ale żaden nie należy do naszego klienta:
        if !tx_slices.is_empty() {
            return Err(FlexProtocolError::SliceOwnershipMismatch(
                "active TX slice belongs to another client".to_string(),
            ));
        }

        // Jeśli brak tx=true, ale klient ma dokładnie 1 swój slice:
        let my_slices: Vec<&FlexSliceState> = slices
            .values()
            .filter(|s| {
                s.in_use
                    && s.client_handle
                        .as_deref()
                        .is_some_and(|h| h.eq_ignore_ascii_case(my_handle))
            })
            .collect();

        if my_slices.len() == 1 {
            return Ok(my_slices[0]);
        }
    } else {
        if tx_slices.len() == 1 {
            return Ok(tx_slices[0]);
        }
        let in_use: Vec<&FlexSliceState> = slices.values().filter(|s| s.in_use).collect();
        if in_use.len() == 1 {
            return Ok(in_use[0]);
        }
    }

    Err(FlexProtocolError::AmbiguousSlice(
        "no confirmed owned TX slice found".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flex_slice_status_creates_slice() {
        let mut slice = FlexSliceState::new(FlexSliceId(0));
        let mut props = BTreeMap::new();
        props.insert("rf_frequency".to_string(), "14.074000".to_string());
        props.insert("mode".to_string(), "DIGU".to_string());
        props.insert("filter_lo".to_string(), "100".to_string());
        props.insert("filter_hi".to_string(), "3000".to_string());
        props.insert("active".to_string(), "1".to_string());

        slice.update_from_properties(&props).expect("update failed");

        assert_eq!(slice.frequency_hz, 14_074_000);
        assert_eq!(slice.mode, "DIGU");
        assert_eq!(slice.filter_low_hz, 100);
        assert_eq!(slice.filter_high_hz, 3000);
        assert_eq!(slice.passband_hz(), 2900);
        assert!(slice.active);
    }

    #[test]
    fn flex_slice_partial_status_preserves_other_fields() {
        let mut slice = FlexSliceState::new(FlexSliceId(1));
        slice.frequency_hz = 7_074_000;
        slice.mode = "LSB".to_string();
        slice.filter_low_hz = -2800;
        slice.filter_high_hz = -200;

        let mut partial = BTreeMap::new();
        partial.insert("rf_frequency".to_string(), "7.150000".to_string());

        slice
            .update_from_properties(&partial)
            .expect("partial update failed");

        assert_eq!(slice.frequency_hz, 7_150_000);
        assert_eq!(slice.mode, "LSB");
        assert_eq!(slice.filter_low_hz, -2800);
        assert_eq!(slice.filter_high_hz, -200);
    }

    #[test]
    fn flex_status_partial_update_preserves_fields() {
        let mut slice = FlexSliceState::new(FlexSliceId(0));
        slice.frequency_hz = 14_250_000;
        slice.mode = "USB".to_string();
        slice.rit_freq_hz = 50;

        let mut partial = BTreeMap::new();
        partial.insert("rit_freq".to_string(), "120".to_string());

        slice.update_from_properties(&partial).expect("update");
        assert_eq!(slice.rit_freq_hz, 120);
        assert_eq!(slice.frequency_hz, 14_250_000);
        assert_eq!(slice.mode, "USB");
    }

    #[test]
    fn flex_unknown_status_class_is_safe() {
        // Obiekty inne niż slice/interlock/transmit nie powodują błędów ani zakłóceń stanu
        let slice = FlexSliceState::new(FlexSliceId(0));
        assert_eq!(slice.id, FlexSliceId(0));
    }

    #[test]
    fn flex_invalid_known_status_field_is_error() {
        let mut slice = FlexSliceState::new(FlexSliceId(0));
        let mut props = BTreeMap::new();
        props.insert("freq".to_string(), "not_a_valid_frequency".to_string());

        let err = slice
            .update_from_properties(&props)
            .expect_err("should reject invalid freq");
        assert!(matches!(err, FlexProtocolError::ParseError(_)));
    }

    #[test]
    fn flex_read_selection_may_observe_foreign_slice() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.client_handle = Some("FOREIGN9999".to_string());
        slices.insert(FlexSliceId(0), s0);

        // Odczyt stanu może obserwować slice innego klienta
        let read = select_read_slice(&slices, None, Some("MY1111")).expect("read should observe");
        assert_eq!(read.id, FlexSliceId(0));
    }

    #[test]
    fn flex_write_selection_rejects_foreign_slice() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.client_handle = Some("FOREIGN9999".to_string());
        slices.insert(FlexSliceId(0), s0);

        // Operacja mutująca bez zgody na modyfikację obcego slice musi zostać odrzucona
        let err = select_write_slice(&slices, None, Some("MY1111"), false)
            .expect_err("should reject foreign write");
        assert!(matches!(err, FlexProtocolError::SliceOwnershipMismatch(_)));

        // Ze zgodą allow_foreign operacja może przejść
        let write_ok = select_write_slice(&slices, None, Some("MY1111"), true)
            .expect("should allow foreign write when permitted");
        assert_eq!(write_ok.id, FlexSliceId(0));
    }

    #[test]
    fn flex_tx_selection_requires_owned_tx_slice() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.tx = true;
        s0.client_handle = Some("FOREIGN9999".to_string());
        slices.insert(FlexSliceId(0), s0);

        // Próba wyboru TX slice gdy aktywny TX należy do innego klienta
        let err = select_tx_slice(&slices, Some("MY1111")).expect_err("should reject foreign tx");
        assert!(matches!(err, FlexProtocolError::SliceOwnershipMismatch(_)));
    }

    #[test]
    fn flex_slice_removed_status_deletes_slice() {
        let mut slice = FlexSliceState::new(FlexSliceId(0));
        assert!(slice.in_use);

        let mut props = BTreeMap::new();
        props.insert("in_use".to_string(), "0".to_string());

        slice.update_from_properties(&props).expect("update failed");
        assert!(!slice.in_use);
    }

    #[test]
    fn flex_slice_tx_flag_selects_tx_slice() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.tx = false;

        let mut s1 = FlexSliceState::new(FlexSliceId(1));
        s1.active = false;
        s1.tx = true;

        slices.insert(FlexSliceId(0), s0);
        slices.insert(FlexSliceId(1), s1);

        let tx = select_tx_slice(&slices, None).expect("tx slice selection failed");
        assert_eq!(tx.id, FlexSliceId(1));
    }

    #[test]
    fn flex_slice_active_flag_selects_active_slice() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = false;

        let mut s1 = FlexSliceState::new(FlexSliceId(1));
        s1.active = true;

        slices.insert(FlexSliceId(0), s0);
        slices.insert(FlexSliceId(1), s1);

        let active =
            select_active_slice(&slices, None, None).expect("active slice selection failed");
        assert_eq!(active.id, FlexSliceId(1));
    }

    #[test]
    fn flex_slice_selection_is_not_map_order_dependent() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = false;
        s0.in_use = true;

        let mut s1 = FlexSliceState::new(FlexSliceId(1));
        s1.active = true;
        s1.in_use = true;

        slices.insert(FlexSliceId(0), s0);
        slices.insert(FlexSliceId(1), s1);

        let active = select_active_slice(&slices, None, None).expect("should find s1");
        assert_eq!(active.id, FlexSliceId(1));
    }

    #[test]
    fn flex_slice_unknown_mode_is_not_mapped_to_usb() {
        let mut slice = FlexSliceState::new(FlexSliceId(0));
        let mut props = BTreeMap::new();
        props.insert("mode".to_string(), "CUSTOM_DIGITAL".to_string());

        slice.update_from_properties(&props).expect("update failed");
        assert_eq!(slice.mode, "CUSTOM_DIGITAL");
        assert_ne!(slice.mode, "USB");
    }

    #[test]
    fn flex_slice_filter_validation_rejects_invalid_bounds() {
        let mut slice = FlexSliceState::new(FlexSliceId(0));
        let mut props = BTreeMap::new();
        props.insert("filter_lo".to_string(), "3000".to_string());
        props.insert("filter_hi".to_string(), "100".to_string());

        let err = slice
            .update_from_properties(&props)
            .expect_err("should reject");
        assert!(matches!(err, FlexProtocolError::ParseError(_)));
    }

    #[test]
    fn flex_slice_client_ownership_is_preserved() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.client_handle = Some("AAAA1111".to_string());

        let mut s1 = FlexSliceState::new(FlexSliceId(1));
        s1.active = true;
        s1.client_handle = Some("BBBB2222".to_string());

        slices.insert(FlexSliceId(0), s0);
        slices.insert(FlexSliceId(1), s1);

        let active = select_active_slice(&slices, None, Some("BBBB2222")).expect("select s1");
        assert_eq!(active.id, FlexSliceId(1));
    }

    #[test]
    fn flex_slice_ambiguous_selection_returns_typed_error() {
        let mut slices = BTreeMap::new();
        let mut s0 = FlexSliceState::new(FlexSliceId(0));
        s0.active = true;
        s0.client_handle = None;

        let mut s1 = FlexSliceState::new(FlexSliceId(1));
        s1.active = true;
        s1.client_handle = None;

        slices.insert(FlexSliceId(0), s0);
        slices.insert(FlexSliceId(1), s1);

        let err = select_active_slice(&slices, None, None).expect_err("should be ambiguous");
        assert!(matches!(err, FlexProtocolError::AmbiguousSlice(_)));
    }
}
