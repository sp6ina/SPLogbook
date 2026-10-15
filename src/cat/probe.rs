// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Bezpieczne, ściśle read-only próbkowanie transceivera ([`RigProbeRunner`]) (NATIVE-6B).
//!
//! Gwarantuje:
//! - Całkowity zakaz załączania PTT podczas testu,
//! - Brak wysyłania komend modyfikujących nastawy radia (brak strojenia, zmiany modulacji itp.),
//! - Zamknięcie transportu i zwolnienie zasobów natychmiast po zakończeniu,
//! - Zwracanie wyłącznie potwierdzonych danych diagnostycznych,
//! - Redagowanie poufnych informacji w komunikatach diagnostycznych.

use std::time::Duration;

use crate::cat::backend::RigProbeResult;
use crate::cat::config::RigConfig;
use crate::cat::factory::{FactoryError, RigBackendFactory};
use crate::cat::rig::RigError;
use crate::cat::tx_arbiter::redact_sensitive_info;

/// Bezpieczny mechanizm uruchamiania read-only probe.
pub struct RigProbeRunner;

impl RigProbeRunner {
    /// Przeprowadza ściśle read-only próbkowanie transceivera na podstawie konfiguracji [`RigConfig`].
    pub async fn probe(config: &RigConfig) -> Result<RigProbeResult, RigError> {
        let mut backend = RigBackendFactory::create(config).map_err(|e| match e {
            FactoryError::InvalidConfig(cfg_err) => RigError::Protocol {
                detail: format!("nieprawidłowa konfiguracja: {cfg_err}"),
            },
            FactoryError::ProfileNotFound { model } => RigError::Protocol {
                detail: format!("nie odnaleziono profilu modelu '{model}'"),
            },
            FactoryError::UnsupportedBackend(b) => RigError::Protocol {
                detail: format!("backend '{b}' nie jest obsługiwany"),
            },
        })?;

        let timeout_dur = Duration::from_millis(config.command_timeout_ms.clamp(500, 10000));

        let probe_fut = backend.probe();
        match tokio::time::timeout(timeout_dur, probe_fut).await {
            Ok(Ok(mut res)) => {
                res.safe_diagnostic = redact_sensitive_info(&res.safe_diagnostic);
                Ok(res)
            }
            Ok(Err(e)) => {
                let safe_err = redact_sensitive_info(&e.to_string());
                log::warn!("[Probe] Błąd próbkowania: {safe_err}");
                Err(e)
            }
            Err(_) => Err(RigError::Timeout {
                endpoint: config.id.to_string(),
                operation: "probe",
            }),
        }
    }
}

/// Stan asynchronicznego procesu próbkowania w GUI.
#[derive(Debug, Clone, PartialEq)]
pub enum ProbeState {
    /// Oczekiwanie na akcję użytkownika.
    Idle,
    /// Trwa sprawdzanie poprawności formularza przed uruchomieniem testu.
    Validating,
    /// Trwa asynchroniczny test (z powiązanym numerem generacji formularza).
    Running {
        /// Numer generacji formularza skorelowany z zadaniem testowym.
        generation: u64,
    },
    /// Test zakończony sukcesem dla wskazanej generacji.
    Succeeded {
        /// Numer generacji formularza.
        generation: u64,
        /// Potwierdzony wynik próbkowania.
        result: RigProbeResult,
    },
    /// Test zakończony błędem dla wskazanej generacji.
    Failed {
        /// Numer generacji formularza.
        generation: u64,
        /// Zredagowany komunikat błędu.
        error: String,
    },
    /// Test został anulowany (np. przez zamknięcie okna lub modyfikację konfiguracji).
    Cancelled,
}

impl Default for ProbeState {
    fn default() -> Self {
        Self::Idle
    }
}
