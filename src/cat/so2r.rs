// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Podstawy pracy SO2R (Single Operator, Two Radios): fokus, blokada TX, konflikty pasm.

/// Identyfikator jednego z dwóch radioodbiorników w konfiguracji SO2R.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RadioSlot {
    A,
    B,
}

impl RadioSlot {
    pub fn opposite(self) -> Self {
        match self {
            RadioSlot::A => RadioSlot::B,
            RadioSlot::B => RadioSlot::A,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RadioSlot::A => "Radio A",
            RadioSlot::B => "Radio B",
        }
    }
}

/// Błędy operacji SO2R.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum So2rError {
    /// Próba załączenia nadawania na drugim radiu, gdy pierwsze nadaje.
    TxAlreadyActive(RadioSlot),
}

impl std::fmt::Display for So2rError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            So2rError::TxAlreadyActive(slot) => write!(f, "{} nadaje — nie można nadać na drugim radiu", slot.label()),
        }
    }
}

impl std::error::Error for So2rError {}

/// Stan logiczny stanowiska SO2R (bez I/O — czysta logika decyzyjna).
#[derive(Debug, Clone, PartialEq)]
pub struct So2rState {
    pub focus: RadioSlot,
    pub tx: Option<RadioSlot>,
    pub radio_a_freq_hz: u64,
    pub radio_b_freq_hz: u64,
    pub radio_a_band: String,
    pub radio_b_band: String,
    /// Gdy `true`, po zakończeniu nadawania fokus nie wraca automatycznie
    /// na radio odbiorcze (ręczne przełączanie).
    pub manual_focus: bool,
}

impl Default for So2rState {
    fn default() -> Self {
        Self {
            focus: RadioSlot::A,
            tx: None,
            radio_a_freq_hz: 14_025_000,
            radio_b_freq_hz: 21_025_000,
            radio_a_band: "20m".to_string(),
            radio_b_band: "15m".to_string(),
            manual_focus: false,
        }
    }
}

impl So2rState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Radio aktualnie aktywne (fokus operatorski).
    pub fn active_radio(&self) -> RadioSlot {
        self.focus
    }

    /// Częstotliwość radia aktywnego (odbiór).
    pub fn active_rx_freq_hz(&self) -> u64 {
        match self.focus {
            RadioSlot::A => self.radio_a_freq_hz,
            RadioSlot::B => self.radio_b_freq_hz,
        }
    }

    /// Częstotliwość nadawania (jeśli którekolwiek radio nadaje).
    pub fn tx_freq_hz(&self) -> Option<u64> {
        self.tx.map(|slot| match slot {
            RadioSlot::A => self.radio_a_freq_hz,
            RadioSlot::B => self.radio_b_freq_hz,
        })
    }

    /// Przełącza fokus na drugie radio.
    pub fn switch_focus(&mut self) {
        self.focus = self.focus.opposite();
    }

    /// Ustawia fokus na wskazane radio.
    pub fn focus_on(&mut self, slot: RadioSlot) {
        self.focus = slot;
    }

    /// Załącza nadawanie na danym radiu, egzekwując blokadę pojedynczego TX.
    pub fn start_tx(&mut self, slot: RadioSlot) -> Result<(), So2rError> {
        if let Some(active) = self.tx {
            if active != slot {
                return Err(So2rError::TxAlreadyActive(active));
            }
        }
        self.tx = Some(slot);
        Ok(())
    }

    /// Kończy nadawanie na wskazanym radiu.
    pub fn stop_tx(&mut self, slot: RadioSlot) {
        if self.tx == Some(slot) {
            self.tx = None;
        }
    }

    /// Czy oba radia pracują na tym samym paśmie (ryzyko wzajemnych zakłóceń)?
    pub fn same_band(&self) -> bool {
        !self.radio_a_band.is_empty() && self.radio_a_band == self.radio_b_band
    }

    /// Czy nadawanie jest aktualnie zablokowane przez drugie radio?
    pub fn is_tx_locked_out(&self, slot: RadioSlot) -> bool {
        matches!(self.tx, Some(active) if active != slot)
    }

    /// Radio, które aktualnie nadaje.
    pub fn tx_radio(&self) -> Option<RadioSlot> {
        self.tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_switches_between_a_and_b() {
        let mut s = So2rState::new();
        assert_eq!(s.active_radio(), RadioSlot::A);
        s.switch_focus();
        assert_eq!(s.active_radio(), RadioSlot::B);
        s.switch_focus();
        assert_eq!(s.active_radio(), RadioSlot::A);
    }

    #[test]
    fn single_tx_enforced() {
        let mut s = So2rState::new();
        assert!(s.start_tx(RadioSlot::A).is_ok());
        assert_eq!(s.start_tx(RadioSlot::B), Err(So2rError::TxAlreadyActive(RadioSlot::A)));
        // Ponowne załączenie tego samego radia jest idempotentne.
        assert!(s.start_tx(RadioSlot::A).is_ok());
    }

    #[test]
    fn tx_lockout_and_release() {
        let mut s = So2rState::new();
        s.start_tx(RadioSlot::A).unwrap();
        assert!(s.is_tx_locked_out(RadioSlot::B));
        assert!(!s.is_tx_locked_out(RadioSlot::A));
        s.stop_tx(RadioSlot::A);
        assert_eq!(s.tx_radio(), None);
        assert!(s.start_tx(RadioSlot::B).is_ok());
    }

    #[test]
    fn tx_frequency_reports_active_tx() {
        let mut s = So2rState::new();
        s.start_tx(RadioSlot::B).unwrap();
        assert_eq!(s.tx_freq_hz(), Some(s.radio_b_freq_hz));
        assert_eq!(s.active_rx_freq_hz(), s.radio_a_freq_hz);
    }

    #[test]
    fn same_band_detection() {
        let mut s = So2rState::new();
        assert!(!s.same_band());
        s.radio_b_band = "20m".to_string();
        assert!(s.same_band());
        s.radio_b_band = String::new();
        assert!(!s.same_band());
    }

    #[test]
    fn labels_and_opposite() {
        assert_eq!(RadioSlot::A.opposite(), RadioSlot::B);
        assert_eq!(RadioSlot::B.opposite(), RadioSlot::A);
        assert_eq!(RadioSlot::A.label(), "Radio A");
    }
}
