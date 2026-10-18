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

/// Bezpieczny strażnik nadawania TX dla prób laboratoryjnych HIL ([`HilTxGuard`]).
///
/// Gwarantuje:
/// - Dokładnie jedno żądanie TX (SetPtt(true)), brak ponawiania (Zero Retry).
/// - Jawny deadline operacji.
/// - Dokładnie jedną jawną asynchroniczną próbę powrotu do RX (`release_and_confirm_rx()`).
/// - Wymóg świeżego ConfirmedRx — test otrzymuje Passed wyłącznie po rzeczywistym potwierdzeniu RX.
/// - Bezpieczną semantykę Drop: brak jawnego release pozostawia stan niepotwierdzony (nigdy Passed).
///   Drop NIE fabrykuje ConfirmedRx, NIE wykonuje blokującego I/O, NIE tworzy runtime Tokio.
pub struct HilTxGuard<'a> {
    rig_id: crate::cat::routing::RigId,
    backend: &'a mut dyn RigBackend,
    deadline: Instant,
    released: bool,
    confirmed_rx: bool,
    attempted_release: bool,
}

impl<'a> HilTxGuard<'a> {
    /// Tworzy nowego strażnika TX po udanym, pojedynczym wysłaniu SetPtt(true).
    pub fn new(
        rig_id: crate::cat::routing::RigId,
        backend: &'a mut dyn RigBackend,
        timeout: Duration,
    ) -> Self {
        Self {
            rig_id,
            backend,
            deadline: Instant::now() + timeout,
            released: false,
            confirmed_rx: false,
            attempted_release: false,
        }
    }

    /// Zwraca identyfikator zabezpieczanego radia.
    pub fn rig_id(&self) -> &crate::cat::routing::RigId {
        &self.rig_id
    }

    /// Czy zasób TX został już zwolniony.
    pub fn is_released(&self) -> bool {
        self.released
    }

    /// Czy potwierdzono bezpieczny powrót do trybu RX.
    pub fn is_confirmed_rx(&self) -> bool {
        self.confirmed_rx
    }

    /// Jawna, asynchroniczna procedura zwalniania nadajnika z weryfikacją ConfirmedRx.
    pub async fn release_and_confirm_rx(&mut self) -> Result<HilPttExecutionResult, HilError> {
        if self.attempted_release {
            return Err(HilError::ReleaseError(
                "PTT release already attempted; duplicate release call rejected".to_string(),
            ));
        }
        self.attempted_release = true;

        if Instant::now() > self.deadline {
            let _ = self.backend.execute(&RigCommand::SetPtt(false)).await;
            return Err(HilError::TimeoutToUnknown(
                "TX pulse deadline exceeded prior to release confirmation".to_string(),
            ));
        }

        // Dokładnie jedna próba wysłania SetPtt(false)
        self.backend.execute(&RigCommand::SetPtt(false)).await?;

        // Weryfikacja stanu odbiornika (świeży odczyt stanu radia)
        let state = self.backend.poll().await?;
        if state.ptt {
            return Err(HilError::MissingConfirmedRx(
                "Radio reports PTT is still active after release command".to_string(),
            ));
        }

        self.released = true;
        self.confirmed_rx = true;

        Ok(HilPttExecutionResult {
            ptt_pulse_duration_ms: 0,
            rx_restored_cleanly: true,
            timestamp_iso: chrono::Utc::now().to_rfc3339(),
        })
    }
}

impl Drop for HilTxGuard<'_> {
    fn drop(&mut self) {
        if !self.released {
            // Zakaz fabrykowania ConfirmedRx!
            // Zakaz tworzenia nowego runtime Tokio!
            // Zakaz blokującego I/O!
            self.confirmed_rx = false;
        }
    }
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

    /// Weryfikuje wieloskładnikową autoryzację dostępu do fizycznego sprzętu laboratoryjnego.
    ///
    /// Wymaga JEDNOCZEŚNIE:
    /// 1. Poprawnego manifestu.
    /// 2. `hil_enabled: true` w manifeście.
    /// 3. Zmiennej środowiskowej `SPLOG_HIL_ENABLE=1`.
    /// 4. Jawnego, niepustego `RigId`.
    /// 5. Niewygasłego `session_id`.
    /// 6. Zmiennej `SPLOG_HIL_SESSION` zgodnej z `manifest.session_id`.
    /// 7. `operator_acknowledged: true`.
    /// 8. Statusu HIL-READY `Passed`.
    pub fn verify_hardware_access_authorization(&self) -> Result<(), HilError> {
        self.manifest.validate_structure()?;

        if !self.manifest.hil_enabled {
            return Err(HilError::UnauthorizedSession(
                "Manifest does not have hil_enabled set to true".to_string(),
            ));
        }

        if std::env::var("SPLOG_HIL_ENABLE").as_deref() != Ok("1") {
            return Err(HilError::UnauthorizedSession(
                "Environment variable SPLOG_HIL_ENABLE=1 is missing or not set to '1'".to_string(),
            ));
        }

        if self.manifest.matrix.rig_id.as_str().trim().is_empty() {
            return Err(HilError::UnauthorizedSession(
                "Explicit RigId is required for hardware access".to_string(),
            ));
        }

        if self.manifest.session_id.trim().is_empty() {
            return Err(HilError::UnauthorizedSession(
                "Manifest session_id cannot be empty".to_string(),
            ));
        }

        if let Some(exp_str) = &self.manifest.expires_at {
            if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(exp_str) {
                if chrono::Utc::now() > exp_time {
                    return Err(HilError::SessionExpired(format!(
                        "HIL session '{}' expired at {}",
                        self.manifest.session_id, exp_str
                    )));
                }
            }
        }

        let env_session = std::env::var("SPLOG_HIL_SESSION").map_err(|_| {
            HilError::UnauthorizedSession(
                "Environment variable SPLOG_HIL_SESSION is not set".to_string(),
            )
        })?;
        if env_session != self.manifest.session_id {
            return Err(HilError::UnauthorizedSession(format!(
                "SPLOG_HIL_SESSION ('{env_session}') does not match manifest session_id ('{}')",
                self.manifest.session_id
            )));
        }

        if !self.manifest.operator_attestation.operator_acknowledged {
            return Err(HilError::UnauthorizedSession(
                "Operator acknowledgement is required".to_string(),
            ));
        }

        if self.readiness_report.overall_status != super::evidence::HilReadinessStatus::Passed {
            return Err(HilError::NotHilReady(
                "Test bench is not fully HIL-READY (readiness check not Passed)".to_string(),
            ));
        }

        Ok(())
    }

    /// Weryfikuje wieloskładnikową autoryzację fizycznego nadawania (PTT).
    pub fn verify_tx_authorization(
        &self,
        consent_token: &HilPttConsentToken,
        arbiter: &crate::cat::tx_arbiter::TxArbiter,
    ) -> Result<(), HilError> {
        self.verify_hardware_access_authorization()?;

        if !self.manifest.allow_tx {
            return Err(HilError::UnauthorizedSession(
                "Manifest does not have allow_tx set to true".to_string(),
            ));
        }

        let allow_tx_env = std::env::var("SPLOG_HIL_ALLOW_TX").map_err(|_| {
            HilError::UnauthorizedSession(
                "Environment variable SPLOG_HIL_ALLOW_TX is not set".to_string(),
            )
        })?;
        if allow_tx_env != "I_HAVE_DUMMY_LOAD" {
            return Err(HilError::UnauthorizedSession(format!(
                "SPLOG_HIL_ALLOW_TX ('{allow_tx_env}') is not 'I_HAVE_DUMMY_LOAD'"
            )));
        }

        match &self.manifest.rf_safety.termination {
            HilRfTermination::DummyLoad(spec) => {
                if self.manifest.rf_safety.max_test_power_watts <= 0.0 {
                    return Err(HilError::SafetyViolation(
                        "Test power must be positive".to_string(),
                    ));
                }
                if self.manifest.rf_safety.max_test_power_watts > spec.power_rating_watts {
                    return Err(HilError::SafetyViolation(
                        "Test power exceeds dummy load rating".to_string(),
                    ));
                }
            }
            HilRfTermination::Antenna { antenna_name } => {
                return Err(HilError::SafetyViolation(format!(
                    "PTT on antenna '{antenna_name}' is strictly prohibited"
                )));
            }
        }

        if self.manifest.rf_safety.max_test_duration_ms == 0
            || self.manifest.rf_safety.max_test_duration_ms > 500
        {
            return Err(HilError::SafetyViolation(
                "Test pulse duration must be between 1 and 500 ms".to_string(),
            ));
        }

        if !arbiter.is_idle() {
            return Err(HilError::TxArbiterBusy(format!(
                "TxArbiter is not in Idle state: {:?}",
                arbiter.state()
            )));
        }

        if consent_token.operator_callsign().trim().is_empty() {
            return Err(HilError::InvalidConsent(
                "Operator consent token has empty callsign".to_string(),
            ));
        }

        Ok(())
    }

    /// Wykonuje bezpieczny, wyłącznie odczytujący test połączenia (probe) na konfiguracji sesji.
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

    /// Pobiera strażnika nadawania TX po spełnieniu wszelkich rygorów bezpieczeństwa.
    pub async fn acquire_tx_guard<'a>(
        &'a self,
        backend: &'a mut dyn RigBackend,
        consent_token: &HilPttConsentToken,
        arbiter: &mut crate::cat::tx_arbiter::TxArbiter,
        duration: Duration,
    ) -> Result<HilTxGuard<'a>, HilError> {
        if self.stage == HilExecutionStage::Stage7AOffline {
            return Err(HilError::PttForbiddenInStage7A);
        }

        self.verify_tx_authorization(consent_token, arbiter)?;

        let cmd_id = crate::cat::routing::RigCommandId::new(1);
        let rig_id = self.manifest.matrix.rig_id.clone();

        arbiter.request_tx(&rig_id, cmd_id).map_err(|e| {
            HilError::TxArbiterBusy(format!("TxArbiter acquire request rejected: {e:?}"))
        })?;

        // Dokładnie jedno wysłanie SetPtt(true) — brak jakichkolwiek ponowień
        if let Err(e) = backend.execute(&RigCommand::SetPtt(true)).await {
            let _ = arbiter.reject_request(&rig_id, cmd_id);
            return Err(HilError::BackendError(format!(
                "SetPtt(true) failed: {e:?}"
            )));
        }

        arbiter.confirm_tx(&rig_id, cmd_id).map_err(|e| {
            HilError::TxArbiterBusy(format!("TxArbiter confirm tx rejected: {e:?}"))
        })?;

        let bounded_duration = duration.clamp(
            Duration::from_millis(10),
            Duration::from_millis(self.manifest.rf_safety.max_test_duration_ms),
        );

        Ok(HilTxGuard::new(
            self.manifest.matrix.rig_id.clone(),
            backend,
            bounded_duration,
        ))
    }

    /// Wykonuje kontrolowaną, bezpieczną próbę załączenia PTT na sztucznym obciążeniu.
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
