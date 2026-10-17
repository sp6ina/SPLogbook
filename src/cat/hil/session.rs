// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::time::{Duration, Instant};

use super::error::HilError;
use super::evidence::{HilExecutionStage, HilReadinessReport};
use super::manifest::{HilRfTermination, HilTestBenchManifest};
use super::validator::HilReadinessValidator;
use crate::cat::backend::RigBackend;
use crate::cat::config::RigConfig;
use crate::cat::probe::RigProbeRunner;
use crate::cat::rig::{RigCapabilities, RigCommand};

/// Bezpieczny token jawnej zgody operatora na wykonanie nadawania (PTT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HilPttConsentToken {
    operator_callsign: String,
    timestamp_iso: String,
}

impl HilPttConsentToken {
    /// Tworzy nowy token zgody operatora po weryfikacji tożsamości.
    pub fn new(operator_callsign: &str) -> Result<Self, HilError> {
        let trimmed = operator_callsign.trim();
        if trimmed.is_empty() {
            return Err(HilError::InvalidConsent(
                "Operator callsign cannot be empty".to_string(),
            ));
        }
        let now = chrono::Utc::now().to_rfc3339();
        Ok(Self {
            operator_callsign: trimmed.to_ascii_uppercase(),
            timestamp_iso: now,
        })
    }

    /// Zwraca znak wywoławczy operatora, który autoryzował próbę.
    pub fn operator_callsign(&self) -> &str {
        &self.operator_callsign
    }

    /// Zwraca sygnaturę czasową autoryzacji.
    pub fn timestamp_iso(&self) -> &str {
        &self.timestamp_iso
    }
}

/// Wynik bezpiecznego, pasywnego testu połączenia (probe).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HilProbeResult {
    /// Identyfikator badanego radia.
    pub rig_id: String,
    /// Nazwa modelu.
    pub model_name: String,
    /// Wykryte możliwości transceivera.
    pub capabilities: RigCapabilities,
    /// Bezpieczny, zredagowany komunikat diagnostyczny.
    pub safe_diagnostic: String,
    /// Sygnatura czasowa wykonania testu.
    pub executed_at: String,
}

/// Wynik kontrolowanej próby nadawania w warunkach laboratoryjnych HIL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HilPttExecutionResult {
    /// Czas trwania impulsu nadawania (w milisekundach).
    pub ptt_pulse_duration_ms: u64,
    /// Czy potwierdzono bezpieczny powrót do trybu odbioru (RX).
    pub rx_restored_cleanly: bool,
    /// Sygnatura czasowa wykonania.
    pub timestamp_iso: String,
}

/// Bezpieczna sesja wykonawcza stanowiska laboratoryjnego HIL ([`HilHardwareSession`]).
///
/// Gwarantuje:
/// 1. Całkowity zakaz wykonania fizycznego PTT w fazie NATIVE-7A.
/// 2. Całkowity zakaz wykonania fizycznego PTT bez pełnego statusu HIL-READY (Passed).
/// 3. Całkowity zakaz nadawania przy zakończeniu toru anteną.
/// 4. Pojedynczą próbę załączenia PTT (Zero Retry Invariant).
/// 5. Wymóg posiadania jawnego tokenu zgody operatora [`HilPttConsentToken`].
pub struct HilHardwareSession {
    manifest: HilTestBenchManifest,
    stage: HilExecutionStage,
    readiness_report: HilReadinessReport,
}

impl HilHardwareSession {
    /// Inicjalizuje nową sesję HIL, przeprowadzając automatyczną ewaluację gotowości.
    pub fn new(manifest: HilTestBenchManifest, stage: HilExecutionStage) -> Self {
        let readiness_report = HilReadinessValidator::validate(&manifest, stage);
        Self {
            manifest,
            stage,
            readiness_report,
        }
    }

    /// Zwraca referencję do manifestu stanowiska.
    pub fn manifest(&self) -> &HilTestBenchManifest {
        &self.manifest
    }

    /// Zwraca aktualną fazę wykonawczą.
    pub fn stage(&self) -> HilExecutionStage {
        self.stage
    }

    /// Zwraca raport gotowości HIL-READY.
    pub fn readiness_report(&self) -> &HilReadinessReport {
        &self.readiness_report
    }

    /// Wykonuje bezpieczny, wyłącznie odczytujący test połączenia (probe) na konfiguracji sesji.
    ///
    /// Wymaga: spełnienia warunków poprawności transportu i identyfikacji radia (`can_run_probe`).
    /// Nie wymaga i nie wykonuje operacji PTT ani modyfikacji VFO.
    pub async fn execute_safe_probe(&self, config: &RigConfig) -> Result<HilProbeResult, HilError> {
        if !self.readiness_report.can_run_probe() {
            return Err(HilError::NotHilReady(
                "Cannot execute probe: transport or radio identification parameters invalid"
                    .to_string(),
            ));
        }

        let probe_res = RigProbeRunner::probe(config).await?;

        Ok(HilProbeResult {
            rig_id: self.manifest.matrix.rig_id.as_str().to_string(),
            model_name: self.manifest.matrix.model_name.clone(),
            capabilities: probe_res.capabilities,
            safe_diagnostic: probe_res.safe_diagnostic,
            executed_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Wykonuje bezpieczny, wyłącznie odczytujący test połączenia bezpośrednio na instancji backendu.
    pub async fn execute_safe_probe_backend(
        &self,
        backend: &mut dyn RigBackend,
    ) -> Result<HilProbeResult, HilError> {
        if !self.readiness_report.can_run_probe() {
            return Err(HilError::NotHilReady(
                "Cannot execute probe: transport or radio identification parameters invalid"
                    .to_string(),
            ));
        }

        let probe_res = backend.probe().await?;

        Ok(HilProbeResult {
            rig_id: self.manifest.matrix.rig_id.as_str().to_string(),
            model_name: self.manifest.matrix.model_name.clone(),
            capabilities: probe_res.capabilities,
            safe_diagnostic: probe_res.safe_diagnostic,
            executed_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Wykonuje kontrolowaną, bezpieczną próbę załączenia PTT na sztucznym obciążeniu.
    ///
    /// Rygory wykonawcze:
    /// - Bezwzględny zakaz wykonania w fazie NATIVE-7A (`Err(HilError::PttForbiddenInStage7A)`).
    /// - Bezwzględny wymóg pełnego statusu HIL-READY (`Passed`).
    /// - Bezwzględny zakaz nadawania na antenie (`Err(HilError::SafetyViolation)`).
    /// - Bezwzględny zakaz ponawiania polecenia `SetPtt(true)` (dokładnie jedno wysłanie).
    /// - Natychmiastowe przywrócenie stanu `SetPtt(false)`.
    pub async fn execute_controlled_ptt(
        &self,
        backend: &mut dyn RigBackend,
        consent_token: &HilPttConsentToken,
        pulse_duration: Duration,
    ) -> Result<HilPttExecutionResult, HilError> {
        // Rygor 1: Faza NATIVE-7A
        if self.stage == HilExecutionStage::Stage7AOffline {
            return Err(HilError::PttForbiddenInStage7A);
        }

        // Rygor 2: Pełne HIL-READY
        if !self.readiness_report.can_run_hardware_ptt() {
            return Err(HilError::NotHilReady(
                "Hardware PTT blocked: test bench is not fully HIL-READY (readiness check failed or blocked)".to_string(),
            ));
        }

        // Rygor 3: Brak anteny
        if let HilRfTermination::Antenna { antenna_name } = &self.manifest.rf_safety.termination {
            return Err(HilError::SafetyViolation(format!(
                "Fatal RF safety violation: attempt to execute PTT on antenna '{antenna_name}'. Dummy load strictly required."
            )));
        }

        // Rygor 4: Ważność tokenu zgody
        if consent_token.operator_callsign().is_empty() {
            return Err(HilError::InvalidConsent(
                "Operator consent token has empty callsign".to_string(),
            ));
        }

        // Limit czasu impulsu testowego w laboratorium: od 10 ms do maksymalnie 500 ms
        let bounded_duration =
            pulse_duration.clamp(Duration::from_millis(10), Duration::from_millis(500));

        let start_time = Instant::now();

        // Pojedyncze wysłanie SetPtt(true) — brak jakichkolwiek prób retry
        backend.execute(&RigCommand::SetPtt(true)).await?;

        tokio::time::sleep(bounded_duration).await;

        // Bezwzględny powrót do RX
        let rx_result = backend.execute(&RigCommand::SetPtt(false)).await;

        let elapsed_ms = start_time.elapsed().as_millis() as u64;

        rx_result?;

        Ok(HilPttExecutionResult {
            ptt_pulse_duration_ms: elapsed_ms,
            rx_restored_cleanly: true,
            timestamp_iso: chrono::Utc::now().to_rfc3339(),
        })
    }
}
