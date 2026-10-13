// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywna warstwa protokołów radiotelefonów (Native-CAT).
//!
//! Zapewnia bezpośrednią, asynchroniczną komunikację ze sprzętem
//! bez pośrednictwa zewnętrznych demonów, pełną obsługę możliwości
//! producentów oraz bezpieczny transport szeregowy i sieciowy.

pub mod icom;
pub mod kenwood;
pub mod profile;
pub mod registry;
pub mod transport;
pub mod yaesu;

use crate::cat::rig::{RigCommand, RigState};

/// Rozszerzone polecenia sterowania radiem (poza wspólnym przenośnym kontraktem RigCommand).
#[derive(Debug, Clone, PartialEq)]
pub enum NativeRigCommand {
    /// Podstawowe polecenie wspólnego kontraktu CAT.
    Common(RigCommand),
    /// Wymuszenie wyłączenia PTT w sytuacji awaryjnej.
    EmergencyRx,
    /// Polecenie pobrania rozszerzonych mierników (np. SWR, ALC, Temp, Vdd).
    PollMeters,
}

/// Asynchroniczne zdarzenia rozgłaszane przez natywne protokoły (np. Transceive, Auto Information).
#[derive(Debug, Clone, PartialEq)]
pub enum NativeRigEvent {
    /// Pełna aktualizacja stanu radia.
    StateChanged(RigState),
    /// Asynchroniczna zmiana częstotliwości VFO (np. obrót gałką transceivera).
    TransceiveFrequencyChanged { vfo: String, frequency_hz: u64 },
    /// Asynchroniczna zmiana emisji lub filtru.
    TransceiveModeChanged { mode: String, filter: Option<u8> },
    /// Surowa ramka diagnostyczna protokołu.
    RawFrameReceived(Vec<u8>),
}
