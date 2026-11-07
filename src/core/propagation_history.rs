// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Historia prognoz propagacyjnych i ich weryfikacja (dokładność modelu).
//!
//! Pozwala mierzyć, jak trafne są prognozy otwarcia pasm w czasie. Prognozy są
//! zapisywane z parametrami wejściowymi (SFI, indeks K, godzina UTC), a następnie
//! weryfikowane przez obserwację rzeczywistą (np. udana łączność na danym paśmie
//! = potwierdzone otwarcie; brak łączności mimo wywołań = potwierdzone zamknięcie).

use super::propagation::BandOpeningStatus;
use serde::{Deserialize, Serialize};

/// Obserwowany wynik łączności, używany do weryfikacji prognozy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservedOutcome {
    /// Łączność została nawiązana na prognozowanym paśmie.
    ConfirmedOpen,
    /// Mimo wywołań nie nawiązano łączności.
    ConfirmedClosed,
}

impl ObservedOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConfirmedOpen => "Potwierdzone otwarcie",
            Self::ConfirmedClosed => "Potwierdzone zamknięcie",
        }
    }
}

/// Pojedynczy rekord prognozy wraz z (opcjonalną) weryfikacją.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropagationRecord {
    pub band: String,
    pub utc_hour: f64,
    pub sfi: u32,
    pub k_index: u8,
    pub forecast_reliability_pct: u8,
    pub forecast_status: BandOpeningStatus,
    pub observed: Option<ObservedOutcome>,
    pub timestamp_unix: i64,
}

fn default_max_records() -> usize {
    500
}

/// Pierścieniowy bufor prognoz z weryfikacją dokładności.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PropagationHistory {
    pub records: Vec<PropagationRecord>,
    /// Maksymalna liczba przechowywanych rekordów (nie jest serializowana).
    #[serde(skip, default = "default_max_records")]
    max_records: usize,
}

impl Default for PropagationHistory {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            max_records: default_max_records(),
        }
    }
}

impl PropagationHistory {
    /// Tworzy historię z limitem rekordów.
    pub fn with_capacity(max_records: usize) -> Self {
        Self {
            records: Vec::with_capacity(max_records.min(500)),
            max_records: max_records.min(500),
        }
    }

    /// Zapisuje prognozę i przycina bufor do limitu.
    pub fn record_forecast(
        &mut self,
        band: &str,
        utc_hour: f64,
        sfi: u32,
        k_index: u8,
        forecast_reliability_pct: u8,
        forecast_status: BandOpeningStatus,
    ) {
        if self.max_records == 0 {
            self.max_records = default_max_records();
        }
        self.records.push(PropagationRecord {
            band: band.to_string(),
            utc_hour,
            sfi,
            k_index,
            forecast_reliability_pct,
            forecast_status,
            observed: None,
            timestamp_unix: chrono::Utc::now().timestamp(),
        });
        if self.records.len() > self.max_records {
            let overflow = self.records.len() - self.max_records;
            self.records.drain(0..overflow);
        }
    }

    /// Weryfikuje najnowszą prognozę dla danego pasma rzeczywistym wynikiem.
    pub fn record_observation(&mut self, band: &str, outcome: ObservedOutcome) {
        if let Some(rec) = self
            .records
            .iter_mut()
            .rev()
            .find(|r| r.band == band && r.observed.is_none())
        {
            rec.observed = Some(outcome);
        }
    }

    /// Średnia dokładność prognoz w procentach (0-100) dla rekordów zweryfikowanych.
    ///
    /// Prognoza jest uznawana za trafną, gdy:
    /// - prognozowano `Open`, a obserwowano `ConfirmedOpen`, lub
    /// - prognozowano `Closed`/`Marginal`, a obserwowano `ConfirmedClosed`.
    pub fn accuracy_pct(&self) -> Option<f64> {
        let verified: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.observed.is_some())
            .collect();
        if verified.is_empty() {
            return None;
        }
        let correct = verified
            .iter()
            .filter(|r| {
                let observed_open = r.observed == Some(ObservedOutcome::ConfirmedOpen);
                let forecast_open = r.forecast_status == BandOpeningStatus::Open;
                // Trafne, gdy prognoza otwarcia pokrywa się z obserwacją.
                observed_open == forecast_open
            })
            .count();
        Some((correct as f64 / verified.len() as f64 * 100.0).clamp(0.0, 100.0))
    }

    /// Liczba zweryfikowanych rekordów.
    /// Dokładność prognoz w procentach dla wskazanego pasma (np. "20m").
    /// Zwraca `None` (N/A) w przypadku braku próbek obserwacji dla tego pasma.
    pub fn band_accuracy_pct(&self, band: &str) -> Option<f64> {
        let verified: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.band == band && r.observed.is_some())
            .collect();
        if verified.is_empty() {
            return None;
        }
        let correct = verified
            .iter()
            .filter(|r| {
                let observed_open = r.observed == Some(ObservedOutcome::ConfirmedOpen);
                let forecast_open = r.forecast_status == BandOpeningStatus::Open;
                observed_open == forecast_open
            })
            .count();
        Some((correct as f64 / verified.len() as f64 * 100.0).clamp(0.0, 100.0))
    }

    pub fn verified_count(&self) -> usize {
        self.records.iter().filter(|r| r.observed.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_trim_are_bounded() {
        let mut h = PropagationHistory::with_capacity(3);
        for _ in 0..5 {
            h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
            assert!(h.records.len() <= 3);
        }
        assert_eq!(h.records.len(), 3);
    }

    #[test]
    fn accuracy_counts_matching_outcomes() {
        let mut h = PropagationHistory::with_capacity(10);
        // Prognoza Open + potwierdzone otwarcie = trafna.
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        h.record_observation("20m", ObservedOutcome::ConfirmedOpen);
        // Prognoza Closed + potwierdzone zamknięcie = trafna.
        h.record_forecast("10m", 14.0, 70, 3, 5, BandOpeningStatus::Closed);
        h.record_observation("10m", ObservedOutcome::ConfirmedClosed);
        // Prognoza Open + potwierdzone zamknięcie = nietrafna.
        h.record_forecast("15m", 14.0, 150, 2, 70, BandOpeningStatus::Open);
        h.record_observation("15m", ObservedOutcome::ConfirmedClosed);

        let acc = h.accuracy_pct().unwrap();
        assert!(
            (acc - 66.666).abs() < 0.1,
            "2/3 trafne => ~66.7%, było {acc}"
        );
        assert_eq!(h.verified_count(), 3);
    }

    #[test]
    fn accuracy_none_when_no_verification() {
        let mut h = PropagationHistory::with_capacity(10);
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        assert_eq!(h.accuracy_pct(), None);
    }

    #[test]
    fn observation_attaches_to_latest_unverified() {
        let mut h = PropagationHistory::with_capacity(10);
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        h.record_forecast("20m", 15.0, 150, 2, 60, BandOpeningStatus::Marginal);
        h.record_observation("20m", ObservedOutcome::ConfirmedOpen);
        // Najnowsza (Marginal) powinna dostać obserwację, starsza pozostać None.
        assert_eq!(h.records[1].observed, Some(ObservedOutcome::ConfirmedOpen));
        assert_eq!(h.records[0].observed, None);
    }

    #[test]
    fn propagation_accuracy_is_na_without_samples() {
        let h = PropagationHistory::default();
        assert_eq!(h.accuracy_pct(), None);
        assert_eq!(h.verified_count(), 0);
    }

    #[test]
    fn propagation_accuracy_is_zero_with_samples_and_no_hits() {
        let mut h = PropagationHistory::with_capacity(5);
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        h.record_observation("20m", ObservedOutcome::ConfirmedClosed);
        assert_eq!(h.accuracy_pct(), Some(0.0));
        assert_eq!(h.verified_count(), 1);
    }

    #[test]
    fn propagation_accuracy_is_one_hundred_with_all_hits() {
        let mut h = PropagationHistory::with_capacity(5);
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        h.record_observation("20m", ObservedOutcome::ConfirmedOpen);
        assert_eq!(h.accuracy_pct(), Some(100.0));
        assert_eq!(h.verified_count(), 1);
    }

    #[test]
    fn propagation_accuracy_is_bounded() {
        let mut h = PropagationHistory::with_capacity(5);
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        h.record_observation("20m", ObservedOutcome::ConfirmedOpen);
        let acc = h.accuracy_pct().unwrap();
        assert!((0.0..=100.0).contains(&acc));
    }

    #[test]
    fn propagation_accuracy_does_not_divide_by_zero() {
        let mut h = PropagationHistory::with_capacity(5);
        assert_eq!(h.accuracy_pct(), None);
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        assert_eq!(h.accuracy_pct(), None);
        assert_eq!(h.band_accuracy_pct("20m"), None);
    }

    #[test]
    fn band_propagation_accuracy_is_na_without_band_samples() {
        let mut h = PropagationHistory::with_capacity(5);
        h.record_forecast("20m", 14.0, 150, 2, 80, BandOpeningStatus::Open);
        h.record_observation("20m", ObservedOutcome::ConfirmedOpen);
        assert_eq!(h.band_accuracy_pct("20m"), Some(100.0));
        assert_eq!(h.band_accuracy_pct("40m"), None);
    }
}
