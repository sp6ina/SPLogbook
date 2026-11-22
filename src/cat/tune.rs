// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Strojenie na spot jako jedna uporządkowana sekwencja poleceń CAT (STAGE-V140-S9).
//!
//! Osobne zadania dla częstotliwości, emisji i split mogą wykonać się w dowolnej kolejności.
//! Tu kolejność jest stała, a split jest włączany wyłącznie po udanym ustawieniu
//! częstotliwości nadawania: radio nigdy nie nadaje z VFO B ustawionego na przypadkową
//! częstotliwość.

use crate::cat::rig::RigCommand;
use std::future::Future;

/// VFO nadawczy przy pracy split (ten sam, którego używa przełącznik split w GUI).
pub const SPLIT_TX_VFO: &str = "VFOB";

/// Co zrobić z pracą split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitRequest {
    /// Nie zmieniać.
    Keep,
    /// Wyłączyć split.
    Off,
    /// Nadawać na podanej częstotliwości (Hz).
    On(u64),
}

/// Strojenie na stację: częstotliwość odbioru, opcjonalnie emisja i split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuneRequest {
    pub frequency_hz: u64,
    /// Emisja radia i szerokość pasma.
    pub mode: Option<(String, u32)>,
    pub split: SplitRequest,
}

/// Wynik strojenia: które części się udały.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TuneOutcome {
    pub frequency: bool,
    pub mode: bool,
    /// Stan split po strojeniu, jeśli został zmieniony.
    pub split: Option<bool>,
}

/// Cel poleceń CAT (aktywny odbiornik w hubie albo atrapa w testach).
pub trait CommandSink {
    type Error: std::fmt::Display;

    fn execute(
        &mut self,
        command: RigCommand,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Wykonuje strojenie w ustalonej kolejności. Nieudane ustawienie częstotliwości przerywa
/// sekwencję; nieudana emisja nie blokuje split.
pub async fn apply_tune<S: CommandSink>(sink: &mut S, request: TuneRequest) -> TuneOutcome {
    let mut outcome = TuneOutcome::default();
    if let Err(e) = sink
        .execute(RigCommand::SetFrequency(request.frequency_hz))
        .await
    {
        log::warn!("CAT: cannot tune to {} Hz: {e}", request.frequency_hz);
        return outcome;
    }
    outcome.frequency = true;

    if let Some((mode, passband_hz)) = request.mode {
        match sink
            .execute(RigCommand::SetMode {
                mode: mode.clone(),
                passband_hz,
            })
            .await
        {
            Ok(()) => outcome.mode = true,
            Err(e) => log::warn!("CAT: cannot set mode {mode}: {e}"),
        }
    }

    let split_off = RigCommand::SetSplit {
        enabled: false,
        tx_vfo: SPLIT_TX_VFO.to_string(),
    };
    match request.split {
        SplitRequest::Keep => {}
        SplitRequest::Off => match sink.execute(split_off).await {
            Ok(()) => outcome.split = Some(false),
            Err(e) => log::warn!("CAT: cannot switch split off: {e}"),
        },
        SplitRequest::On(tx_hz) => {
            if let Err(e) = sink.execute(RigCommand::SetSplitFrequency(tx_hz)).await {
                log::warn!("CAT: split not enabled, TX frequency {tx_hz} Hz rejected: {e}");
                return outcome;
            }
            match sink
                .execute(RigCommand::SetSplit {
                    enabled: true,
                    tx_vfo: SPLIT_TX_VFO.to_string(),
                })
                .await
            {
                Ok(()) => outcome.split = Some(true),
                Err(e) => log::warn!("CAT: cannot enable split: {e}"),
            }
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Atrapa radia: zapisuje polecenia i odrzuca wskazane rodzaje.
    #[derive(Default)]
    struct FakeRig {
        sent: Vec<RigCommand>,
        reject: Vec<fn(&RigCommand) -> bool>,
    }

    impl CommandSink for FakeRig {
        type Error = &'static str;

        fn execute(
            &mut self,
            command: RigCommand,
        ) -> impl Future<Output = Result<(), &'static str>> + Send {
            let rejected = self.reject.iter().any(|f| f(&command));
            self.sent.push(command);
            std::future::ready(if rejected { Err("unsupported") } else { Ok(()) })
        }
    }

    fn request(split: SplitRequest) -> TuneRequest {
        TuneRequest {
            frequency_hz: 14_025_000,
            mode: Some(("CW".to_string(), 500)),
            split,
        }
    }

    fn split(enabled: bool) -> RigCommand {
        RigCommand::SetSplit {
            enabled,
            tx_vfo: SPLIT_TX_VFO.to_string(),
        }
    }

    #[tokio::test]
    async fn split_is_enabled_after_the_tx_frequency_in_a_fixed_order() {
        let mut rig = FakeRig::default();
        let outcome = apply_tune(&mut rig, request(SplitRequest::On(14_027_000))).await;
        assert_eq!(
            rig.sent,
            vec![
                RigCommand::SetFrequency(14_025_000),
                RigCommand::SetMode {
                    mode: "CW".to_string(),
                    passband_hz: 500
                },
                RigCommand::SetSplitFrequency(14_027_000),
                split(true),
            ]
        );
        assert_eq!(
            outcome,
            TuneOutcome {
                frequency: true,
                mode: true,
                split: Some(true)
            }
        );
    }

    #[tokio::test]
    async fn rejected_tx_frequency_never_enables_split() {
        let mut rig = FakeRig {
            reject: vec![|c| matches!(c, RigCommand::SetSplitFrequency(_))],
            ..FakeRig::default()
        };
        let outcome = apply_tune(&mut rig, request(SplitRequest::On(14_027_000))).await;
        assert!(!rig.sent.contains(&split(true)));
        assert_eq!(outcome.split, None);
    }

    #[tokio::test]
    async fn failed_frequency_stops_the_sequence_and_mode_failure_does_not() {
        let mut rig = FakeRig {
            reject: vec![|c| matches!(c, RigCommand::SetFrequency(_))],
            ..FakeRig::default()
        };
        assert_eq!(
            apply_tune(&mut rig, request(SplitRequest::Off)).await,
            TuneOutcome::default()
        );
        assert_eq!(rig.sent.len(), 1);

        let mut rig = FakeRig {
            reject: vec![|c| matches!(c, RigCommand::SetMode { .. })],
            ..FakeRig::default()
        };
        let outcome = apply_tune(&mut rig, request(SplitRequest::Off)).await;
        assert_eq!(rig.sent.last(), Some(&split(false)));
        assert!(outcome.frequency && !outcome.mode);
        assert_eq!(outcome.split, Some(false));
    }

    #[tokio::test]
    async fn keep_leaves_split_alone() {
        let mut rig = FakeRig::default();
        let mut req = request(SplitRequest::Keep);
        req.mode = None;
        apply_tune(&mut rig, req).await;
        assert_eq!(rig.sent, vec![RigCommand::SetFrequency(14_025_000)]);
    }
}
