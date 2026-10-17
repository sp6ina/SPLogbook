// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::fmt;

/// Silnie typowane błędy podsystemu Hardware-In-The-Loop (HIL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HilError {
    /// Stanowisko nie spełnia kryteriów HIL-READY.
    NotHilReady(String),
    /// Wykonanie fizycznego PTT jest bezwzględnie zablokowane w etapie NATIVE-7A.
    PttForbiddenInStage7A,
    /// Błąd walidacji lub struktury manifestu stanowiska HIL.
    InvalidManifest(String),
    /// Nieprawidłowa lub nieobsługiwana konfiguracja transportu.
    InvalidTransport(String),
    /// Brak ważnej zgody operatora lub nieprawidłowy token autoryzacji PTT.
    InvalidConsent(String),
    /// Naruszenie zasad bezpieczeństwa radiowego (np. próba nadawania na antenie).
    SafetyViolation(String),
    /// Błąd operacji na backendzie radiowym.
    BackendError(String),
    /// Błąd fabryki backendów podczas tworzenia instancji.
    FactoryError(String),
    /// Błąd wejścia/wyjścia lub systemu plików.
    IoError(String),
}

impl fmt::Display for HilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHilReady(msg) => write!(f, "HIL test bench is not HIL-READY: {msg}"),
            Self::PttForbiddenInStage7A => write!(
                f,
                "Physical PTT execution is strictly prohibited in Stage NATIVE-7A (permitted only in NATIVE-7B with hardware bench)"
            ),
            Self::InvalidManifest(msg) => write!(f, "Invalid HIL manifest: {msg}"),
            Self::InvalidTransport(msg) => write!(f, "Invalid transport configuration: {msg}"),
            Self::InvalidConsent(msg) => write!(f, "Invalid operator consent: {msg}"),
            Self::SafetyViolation(msg) => write!(f, "RF safety violation: {msg}"),
            Self::BackendError(msg) => write!(f, "CAT backend error during HIL operation: {msg}"),
            Self::FactoryError(msg) => write!(f, "Backend factory error: {msg}"),
            Self::IoError(msg) => write!(f, "I/O error during HIL operation: {msg}"),
        }
    }
}

impl std::error::Error for HilError {}

impl From<crate::cat::factory::FactoryError> for HilError {
    fn from(err: crate::cat::factory::FactoryError) -> Self {
        Self::FactoryError(err.to_string())
    }
}

impl From<crate::cat::rig::RigError> for HilError {
    fn from(err: crate::cat::rig::RigError) -> Self {
        Self::BackendError(err.to_string())
    }
}
