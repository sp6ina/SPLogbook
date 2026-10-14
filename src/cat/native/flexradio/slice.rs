// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Model i zarządzanie odbiornikami slice transceivera FlexRadio.
//!
//! SmartSDR nie korzysta z tradycyjnej koncepcji VFO A/B, lecz z niezależnych
//! instancji odbiorników (slice receivers). Każdy slice ma własną częstotliwość,
//! tryb emisji, filtry DSP, stan aktywności, flagę TX oraz uchwyt klienta.
//!
//! Moduł implementuje deterministyczną politykę wyboru aktywnego slice dla RigState,
//! zapobiega arbitralnemu wybieraniu pierwszego elementu mapy oraz nie fabricuje VFO.

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
    pub fn update_from_properties(
        &mut self,
        properties: &BTreeMap<String, String>,
    ) -> Result<(), FlexProtocolError> {
        if let Some(val) = properties.get("in_use") {
            self.in_use = val == "1" || val.eq_ignore_ascii_case("true");
        }

        if let Some(mhz) = properties
            .get("rf_frequency")
            .or_else(|| properties.get("freq"))
            .and_then(|val| val.parse::<f64>().ok())
            .filter(|&mhz| mhz.is_finite() && mhz > 0.0)
        {
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

        if let Some(lo) = properties
            .get("filter_lo")
            .and_then(|val| val.parse::<i32>().ok())
        {
            low = lo;
            filter_modified = true;
        }
        if let Some(hi) = properties
            .get("filter_hi")
            .and_then(|val| val.parse::<i32>().ok())
        {
            high = hi;
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

        if let Some(s) = properties
            .get("step")
            .and_then(|val| val.parse::<u32>().ok())
        {
            self.step_hz = s;
        }

        if let Some(val) = properties.get("rit_on") {
            self.rit_on = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Some(f) = properties
            .get("rit_freq")
            .and_then(|val| val.parse::<i32>().ok())
        {
            self.rit_freq_hz = f;
        }

        if let Some(val) = properties.get("xit_on") {
            self.xit_on = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Some(f) = properties
            .get("xit_freq")
            .and_then(|val| val.parse::<i32>().ok())
        {
            self.xit_freq_hz = f;
        }

        if let Some(val) = properties.get("rxant") {
            self.rx_ant.clone_from(val);
        }
        if let Some(val) = properties.get("txant") {
            self.tx_ant.clone_from(val);
        }

        if let Some(p) = properties
            .get("rf_power")
            .and_then(|val| val.parse::<f32>().ok())
        {
            self.rf_power = Some(p);
        }

        Ok(())
    }
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
/// 4. W przypadku niejednoznaczności (wiele aktywnych slice bez wskazania właściciela)
///    lub braku jakiegokolwiek slice zwracany jest typowany błąd.
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

/// Deterministyczny wybór potwierdzonego slice nadawczego (TX slice).
///
/// Polityka wyboru:
/// 1. Slice posiadający potwierdzoną flagę `tx == true` oraz `in_use == true`.
/// 2. Jeśli żaden slice nie ma `tx == true`, ale istnieje dokładnie jeden slice dla bieżącego klienta,
///    to ten slice jest traktowany jako domyślny kontekst TX.
/// 3. W pozostałych przypadkach zwracany jest błąd niejednoznaczności.
pub fn select_tx_slice<'a>(
    slices: &'a BTreeMap<FlexSliceId, FlexSliceState>,
    client_handle: Option<&str>,
) -> Result<&'a FlexSliceState, FlexProtocolError> {
    let tx_slices: Vec<&FlexSliceState> = slices.values().filter(|s| s.in_use && s.tx).collect();

    if tx_slices.len() == 1 {
        return Ok(tx_slices[0]);
    }

    if let Some(handle) = client_handle {
        let clean_handle = handle.strip_prefix("0x").unwrap_or(handle);
        let client_tx: Vec<&FlexSliceState> = tx_slices
            .into_iter()
            .filter(|s| {
                s.client_handle
                    .as_deref()
                    .is_some_and(|h| h.eq_ignore_ascii_case(clean_handle))
            })
            .collect();

        if client_tx.len() == 1 {
            return Ok(client_tx[0]);
        }
    }

    // Jeśli brak slice z flagą tx=true, ale jest tylko jeden slice w użyciu:
    let in_use: Vec<&FlexSliceState> = slices.values().filter(|s| s.in_use).collect();
    if in_use.len() == 1 {
        return Ok(in_use[0]);
    }

    Err(FlexProtocolError::AmbiguousSlice(
        "no confirmed TX slice found".to_string(),
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
        // Niewymienione pola zostały zachowane:
        assert_eq!(slice.mode, "LSB");
        assert_eq!(slice.filter_low_hz, -2800);
        assert_eq!(slice.filter_high_hz, -200);
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
        // Sprawdzenie, że slice 1 jest wybierany, gdy jest jedynym aktywnym,
        // mimo że slice 0 występuje w mapie jako pierwszy
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
        props.insert("filter_hi".to_string(), "100".to_string()); // low > high!

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

        // Zapytanie o aktywny slice dla klienta BBBB2222
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

        // Dwa aktywne slice bez wskazania preferencji i bez powiązania z klientem
        let err = select_active_slice(&slices, None, None).expect_err("should be ambiguous");
        assert!(matches!(err, FlexProtocolError::AmbiguousSlice(_)));
    }
}
