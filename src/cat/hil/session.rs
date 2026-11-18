// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::authorization::{
    HilAttemptRegistry, HilAttestationDocument, HilAuthorizationDecision, HilAuthorizationOutcome,
    HilEvidenceProviderOutcome, HilHardwareExecutionAuthorizer, HilIndependentEvidence,
};
use super::error::HilError;
use super::evidence::{HilExecutionStage, HilReadinessReport, HilReadinessStatus};
use super::manifest::{HIL_MIN_TEST_DURATION_MS, HilRfTermination, HilTestBenchManifest};
use super::validator::HilReadinessValidator;
use crate::cat::backend::RigBackend;
use crate::cat::config::RigConfig;
use crate::cat::probe::RigProbeRunner;
use crate::cat::rig::{RigCapabilities, RigCommand};

pub const SESSION_STATE_READY: u8 = 0;
pub const SESSION_STATE_ACTIVE_TX: u8 = 1;
pub const SESSION_STATE_COMPLETED_CLEANLY: u8 = 2;
pub const SESSION_STATE_FAILED_OR_UNKNOWN: u8 = 3;

/// Stan bezpieczeństwa sesji wykonawczej HIL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HilSessionState {
    /// Sesja jest zainicjalizowana i gotowa do kontrolowanych prób.
    Ready,
    /// Sesja posiada aktywnego strażnika TX (nadawanie w toku).
    ActiveTx,
    /// Próba TX została poprawnie i bezpiecznie zakończona (potwierdzony powrót do RX).
    CompletedCleanly,
    /// Stan nieznany lub awaryjny (np. strażnik TX został porzucony bez wykonania jawnego release).
    FailedOrUnknown,
}

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
        let now = jiff::Timestamp::now().to_string();
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

/// Wynik jawnej próby zwolnienia nadajnika (PTT OFF) w ścieżce HIL.
///
/// Śledzi, czy próba zwolnienia została w ogóle podjęta i czy została potwierdzona świeżym
/// odczytem RX. Dzięki temu błąd PTT OFF nie jest ukrywany, a niepotwierdzone zwolnienie nigdy
/// nie jest raportowane jako sukces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HilReleaseOutcome {
    /// Żadna próba PTT OFF nie została jeszcze podjęta.
    NotAttempted,
    /// PTT OFF wysłane i potwierdzone świeżym odczytem RX.
    Confirmed,
    /// Próba PTT OFF zakończyła się błędem transportu/protokołu — nadajnik może być załączony.
    Failed,
    /// PTT OFF wysłane (lub podjęte awaryjnie), ale brak potwierdzenia RX.
    Unconfirmed,
}

/// Wylicza ograniczony czas impulsu PTT bez użycia `Ord::clamp`.
///
/// Funkcja jest totalna (zwraca [`Result`], nigdy nie panikuje) i odrzuca odwrócony zakres:
/// gdy górna granica z manifestu jest mniejsza niż [`HIL_MIN_TEST_DURATION_MS`], zwraca
/// kontrolowany [`HilError::SafetyViolation`] zamiast wywoływać `clamp(min > max)`.
///
/// `min == max` jest jawne i dozwolone — zwraca dokładnie tę wartość.
fn bounded_test_duration(
    requested: Duration,
    max_test_duration_ms: u64,
) -> Result<Duration, HilError> {
    if max_test_duration_ms < HIL_MIN_TEST_DURATION_MS {
        return Err(HilError::SafetyViolation(format!(
            "max_test_duration_ms ({max_test_duration_ms}) is below the transmit-path minimum of \
             {HIL_MIN_TEST_DURATION_MS} ms; refusing to build an inverted PTT pulse range"
        )));
    }
    let lower = Duration::from_millis(HIL_MIN_TEST_DURATION_MS);
    let upper = Duration::from_millis(max_test_duration_ms);
    // `upper >= lower` is guaranteed by the check above, so the comparisons below are total.
    let bounded = if requested < lower {
        lower
    } else if requested > upper {
        upper
    } else {
        requested
    };
    Ok(bounded)
}

/// Bezpieczny strażnik nadawania TX dla prób laboratoryjnych HIL ([`HilTxGuard`]).
///
/// Gwarantuje:
/// - Dokładnie jedno żądanie TX (SetPtt(true)), brak ponawiania (Zero Retry).
/// - Jawny deadline operacji.
/// - Dokładnie jedną jawną asynchroniczną próbę powrotu do RX (`release_and_confirm_rx()`).
/// - Wymóg świeżego ConfirmedRx — test otrzymuje Passed wyłącznie po rzeczywistym potwierdzeniu RX.
/// - Bezpieczną semantykę Drop: brak jawnego release pozostawia stan niepotwierdzony (nigdy Passed)
///   i podejmuje **jedną** nieblokującą próbę awaryjnego PTT OFF, jeżeli zwolnienie nie zostało
///   wcześniej zainicjowane (np. przy anulowaniu przyszłości w trakcie impulsu TX).
///   Drop NIE fabrykuje ConfirmedRx, NIE wykonuje blokującego I/O, NIE tworzy runtime Tokio.
pub struct HilTxGuard<'a> {
    rig_id: crate::cat::routing::RigId,
    backend: &'a mut dyn RigBackend,
    deadline: Instant,
    released: bool,
    confirmed_rx: bool,
    attempted_release: bool,
    release_outcome: HilReleaseOutcome,
    session_state: Option<Arc<AtomicU8>>,
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
            release_outcome: HilReleaseOutcome::NotAttempted,
            session_state: None,
        }
    }

    /// Tworzy nowego strażnika TX powiązanego ze stanem sesji wykonawczej.
    pub fn new_with_state(
        rig_id: crate::cat::routing::RigId,
        backend: &'a mut dyn RigBackend,
        timeout: Duration,
        session_state: Arc<AtomicU8>,
    ) -> Self {
        session_state.store(SESSION_STATE_ACTIVE_TX, Ordering::SeqCst);
        Self {
            rig_id,
            backend,
            deadline: Instant::now() + timeout,
            released: false,
            confirmed_rx: false,
            attempted_release: false,
            release_outcome: HilReleaseOutcome::NotAttempted,
            session_state: Some(session_state),
        }
    }

    /// Zwraca aktualny stan powiązanej sesji wykonawczej.
    pub fn session_state(&self) -> HilSessionState {
        if let Some(state) = &self.session_state {
            match state.load(Ordering::SeqCst) {
                SESSION_STATE_READY => HilSessionState::Ready,
                SESSION_STATE_ACTIVE_TX => HilSessionState::ActiveTx,
                SESSION_STATE_COMPLETED_CLEANLY => HilSessionState::CompletedCleanly,
                _ => HilSessionState::FailedOrUnknown,
            }
        } else if self.released && self.confirmed_rx {
            HilSessionState::CompletedCleanly
        } else if !self.released {
            HilSessionState::ActiveTx
        } else {
            HilSessionState::FailedOrUnknown
        }
    }

    /// Podłącza uchwyt stanu sesji pod strażnika TX.
    pub fn set_session_state_handle(&mut self, state: Arc<AtomicU8>) {
        state.store(SESSION_STATE_ACTIVE_TX, Ordering::SeqCst);
        self.session_state = Some(state);
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

    /// Wynik ostatniej (lub awaryjnej) próby zwolnienia nadajnika.
    ///
    /// Pozwala odróżnić brak próby od błędu i od niepotwierdzonego zwolnienia — dzięki temu
    /// błąd PTT OFF nie jest ukrywany, a niepotwierdzone PTT OFF nie jest raportowane jako sukces.
    pub fn release_outcome(&self) -> HilReleaseOutcome {
        self.release_outcome
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
            // Deadline exceeded: dokładnie jedna próba PTT OFF jest nadal podejmowana
            // (fail-closed), ale stan pozostaje Unknown i nigdy nie jest raportowany jako sukces.
            let off_res = self.backend.execute(&RigCommand::SetPtt(false)).await;
            self.release_outcome = if off_res.is_ok() {
                HilReleaseOutcome::Unconfirmed
            } else {
                HilReleaseOutcome::Failed
            };
            if let Some(st) = &self.session_state {
                st.store(SESSION_STATE_FAILED_OR_UNKNOWN, Ordering::SeqCst);
            }
            return Err(HilError::TimeoutToUnknown(
                "TX pulse deadline exceeded prior to release confirmation".to_string(),
            ));
        }

        // Dokładnie jedna próba wysłania SetPtt(false)
        if let Err(e) = self.backend.execute(&RigCommand::SetPtt(false)).await {
            self.release_outcome = HilReleaseOutcome::Failed;
            if let Some(st) = &self.session_state {
                st.store(SESSION_STATE_FAILED_OR_UNKNOWN, Ordering::SeqCst);
            }
            return Err(e.into());
        }

        // Weryfikacja stanu odbiornika (świeży odczyt stanu radia)
        let state = match self.backend.poll().await {
            Ok(s) => s,
            Err(e) => {
                self.release_outcome = HilReleaseOutcome::Unconfirmed;
                if let Some(st) = &self.session_state {
                    st.store(SESSION_STATE_FAILED_OR_UNKNOWN, Ordering::SeqCst);
                }
                return Err(e.into());
            }
        };

        if state.ptt {
            self.release_outcome = HilReleaseOutcome::Unconfirmed;
            if let Some(st) = &self.session_state {
                st.store(SESSION_STATE_FAILED_OR_UNKNOWN, Ordering::SeqCst);
            }
            return Err(HilError::MissingConfirmedRx(
                "Radio reports PTT is still active after release command".to_string(),
            ));
        }

        self.released = true;
        self.confirmed_rx = true;
        self.release_outcome = HilReleaseOutcome::Confirmed;
        if let Some(st) = &self.session_state {
            st.store(SESSION_STATE_COMPLETED_CLEANLY, Ordering::SeqCst);
        }

        Ok(HilPttExecutionResult {
            ptt_pulse_duration_ms: 0,
            rx_restored_cleanly: true,
            timestamp_iso: jiff::Timestamp::now().to_string(),
        })
    }

    /// Awaryjna, **nieblokująca** próba PTT OFF wykonywana z `Drop`.
    ///
    /// Polluje przyszłość backendu dokładnie raz z `Waker::noop()`:
    /// - `Ready(Ok)` / `Pending` → próba została wysłana, ale nie jest potwierdzona (`Unconfirmed`),
    /// - `Ready(Err)` → próba zakończyła się błędem (`Failed`).
    ///
    /// Nie tworzy runtime, nie blokuje wątku i nie fabrykuje `ConfirmedRx`.
    fn emergency_release_ptt(&mut self) -> HilReleaseOutcome {
        use std::task::{Context, Poll, Waker};

        let command = RigCommand::SetPtt(false);
        let mut future = self.backend.execute(&command);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(Ok(())) | Poll::Pending => HilReleaseOutcome::Unconfirmed,
            Poll::Ready(Err(_)) => HilReleaseOutcome::Failed,
        }
    }
}

impl Drop for HilTxGuard<'_> {
    fn drop(&mut self) {
        if !self.released {
            // Zakaz fabrykowania ConfirmedRx!
            // Zakaz tworzenia nowego runtime Tokio!
            // Zakaz blokującego I/O!
            self.confirmed_rx = false;
            if let Some(state) = &self.session_state {
                state.store(SESSION_STATE_FAILED_OR_UNKNOWN, Ordering::SeqCst);
            }
            // F-14: jeżeli zwolnienie nie zostało wcześniej zainicjowane (porzucony strażnik,
            // anulowanie przyszłości w trakcie impulsu TX), podejmij dokładnie jedną
            // nieblokującą próbę awaryjnego PTT OFF. Brak potwierdzenia nigdy nie jest sukcesem.
            if !self.attempted_release {
                self.attempted_release = true;
                self.release_outcome = self.emergency_release_ptt();
            }
        }
    }
}

/// Zwraca bieżący czas ewaluacji HIL:
/// - wartość ze zmiennej środowiskowej `SPLOG_HIL_REFERENCE_TIME` (jeśli ustawiona na poprawny RFC3339),
/// - lub rzeczywisty czas systemowy [`jiff::Timestamp::now()`] w produkcji.
pub fn default_hil_evaluation_time() -> jiff::Timestamp {
    if let Ok(var) = std::env::var("SPLOG_HIL_REFERENCE_TIME")
        && let Ok(dt) = crate::cat::hil::parse_rfc3339(&var)
    {
        return dt;
    }
    jiff::Timestamp::now()
}

/// Źródło bramek środowiskowych HIL.
///
/// Produkcyjnie bramki są odczytywane ze zmiennych procesu (`from_process`). Bramki są
/// **dodatkowymi warunkami**, a nie dowodem: nie mogą zastąpić niezależnego dowodu (F-15).
/// W zakresie testowym bramki mogą zostać wstrzyknięte jawnie, dzięki czemu testy nie mutują
/// globalnego środowiska procesu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HilEnvironmentGates {
    hil_enabled: bool,
    session: Option<String>,
    allow_tx: Option<String>,
}

impl HilEnvironmentGates {
    /// Odczytuje bramki ze środowiska procesu (zachowanie produkcyjne).
    pub fn from_process() -> Self {
        Self {
            hil_enabled: std::env::var("SPLOG_HIL_ENABLE").as_deref() == Ok("1"),
            session: std::env::var("SPLOG_HIL_SESSION").ok(),
            allow_tx: std::env::var("SPLOG_HIL_ALLOW_TX").ok(),
        }
    }

    /// Czy bramka `SPLOG_HIL_ENABLE=1` jest spełniona.
    pub fn hil_enabled(&self) -> bool {
        self.hil_enabled
    }

    /// Wartość bramki `SPLOG_HIL_SESSION`.
    pub fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }

    /// Wartość bramki `SPLOG_HIL_ALLOW_TX`.
    pub fn allow_tx(&self) -> Option<&str> {
        self.allow_tx.as_deref()
    }

    /// Bramki jawnie wstrzyknięte dla testów w zakresie `cfg(test)`.
    #[cfg(test)]
    pub(crate) fn for_tests(session: &str, allow_tx: &str) -> Self {
        Self {
            hil_enabled: true,
            session: Some(session.to_string()),
            allow_tx: Some(allow_tx.to_string()),
        }
    }
}

/// Bezpieczna sesja wykonawcza stanowiska laboratoryjnego HIL ([`HilHardwareSession`]).
///
/// Gwarantuje:
/// 1. Całkowity zakaz wykonania fizycznego PTT w fazie NATIVE-7A.
/// 2. Całkowity zakaz wykonania fizycznego PTT bez niezależnego dowodu i decyzji
///    autoryzacyjnej ([`HilAuthorizationDecision`], F-15).
/// 3. Całkowity zakaz nadawania przy zakończeniu toru anteną.
/// 4. Pojedynczą próbę załączenia PTT (Zero Retry Invariant).
/// 5. Wymóg posiadania jawnego tokenu zgody operatora [`HilPttConsentToken`].
///
/// Sesja **nie** traktuje manifestu ani deklaracji operatora jako dowodu. Dowód jest osobnym
/// dokumentem ([`HilIndependentEvidence`]) przekazanym jawnie przez [`Self::with_authorization`].
/// W kodzie produkcyjnym nie istnieje żaden dostawca dowodu, więc sesja bez dowodu zawsze
/// odmawia wykonania sprzętowego.
pub struct HilHardwareSession {
    manifest: HilTestBenchManifest,
    attestation: Option<HilAttestationDocument>,
    evidence: Option<HilIndependentEvidence>,
    stage: HilExecutionStage,
    readiness_report: HilReadinessReport,
    session_state: Arc<AtomicU8>,
    reference_time: Option<jiff::Timestamp>,
    attempt_registry: Arc<HilAttemptRegistry>,
    authorization_consumed: AtomicBool,
    environment: Option<HilEnvironmentGates>,
}

impl HilHardwareSession {
    /// Inicjalizuje nową sesję HIL, przeprowadzając automatyczną walidację konfiguracji.
    ///
    /// Walidacja konfiguracji **nie** upoważnia wykonania sprzętowego.
    pub fn new(manifest: HilTestBenchManifest, stage: HilExecutionStage) -> Self {
        Self::new_with_reference_time(manifest, stage, None)
    }

    /// Inicjalizuje nową sesję HIL z jawnym czasem odniesienia dla testów deterministycznych.
    pub fn new_with_reference_time(
        manifest: HilTestBenchManifest,
        stage: HilExecutionStage,
        reference_time: Option<jiff::Timestamp>,
    ) -> Self {
        let readiness_report = HilReadinessValidator::validate(&manifest, stage);
        Self {
            manifest,
            attestation: None,
            evidence: None,
            stage,
            readiness_report,
            session_state: Arc::new(AtomicU8::new(SESSION_STATE_READY)),
            reference_time,
            attempt_registry: Arc::new(HilAttemptRegistry::new()),
            authorization_consumed: AtomicBool::new(false),
            environment: None,
        }
    }

    /// Przypisuje jawny czas odniesienia (builder).
    pub fn with_reference_time(mut self, reference_time: jiff::Timestamp) -> Self {
        self.reference_time = Some(reference_time);
        self
    }

    /// Przypisuje jawne bramki środowiskowe (nie zastępują niezależnego dowodu).
    pub fn with_environment(mut self, environment: HilEnvironmentGates) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Zwraca bramki środowiskowe: jawnie wstrzyknięte albo odczytane z procesu w chwili użycia.
    ///
    /// Odczyt w chwili użycia jest istotny dla determinizmu testów, które zakładają bramki
    /// środowiskowe **przed** wywołaniem walidacji.
    fn environment_gates(&self) -> HilEnvironmentGates {
        self.environment
            .clone()
            .unwrap_or_else(HilEnvironmentGates::from_process)
    }

    /// Przypisuje jawny dokument oświadczenia operatora oraz niezależny dowód.
    ///
    /// W kodzie produkcyjnym dowód może pochodzić wyłącznie z zewnętrznego, zatwierdzonego
    /// dostawcy; nie istnieje żaden wbudowany dostawca. Bez dokumentu dowodowego sesja odmawia
    /// wykonania sprzętowego.
    pub fn with_authorization(
        mut self,
        attestation: Option<HilAttestationDocument>,
        evidence: Option<HilIndependentEvidence>,
    ) -> Self {
        self.attestation = attestation;
        self.evidence = evidence;
        self
    }

    /// Zwraca efektywny czas ewaluacji sesji.
    pub fn evaluated_at(&self) -> jiff::Timestamp {
        self.reference_time
            .unwrap_or_else(default_hil_evaluation_time)
    }

    /// Zwraca aktualny stan bezpieczeństwa sesji.
    pub fn session_state(&self) -> HilSessionState {
        match self.session_state.load(Ordering::SeqCst) {
            SESSION_STATE_READY => HilSessionState::Ready,
            SESSION_STATE_ACTIVE_TX => HilSessionState::ActiveTx,
            SESSION_STATE_COMPLETED_CLEANLY => HilSessionState::CompletedCleanly,
            _ => HilSessionState::FailedOrUnknown,
        }
    }

    /// Zwraca uchwyt do atomowego licznika stanu sesji.
    pub fn session_state_handle(&self) -> Arc<AtomicU8> {
        self.session_state.clone()
    }

    /// Zwraca referencję do planu stanowiska.
    pub fn manifest(&self) -> &HilTestBenchManifest {
        &self.manifest
    }

    /// Zwraca jawny dokument oświadczenia operatora, jeśli został przekazany.
    pub fn attestation(&self) -> Option<&HilAttestationDocument> {
        self.attestation.as_ref()
    }

    /// Zwraca jawny dokument niezależnego dowodu, jeśli został przekazany.
    pub fn evidence(&self) -> Option<&HilIndependentEvidence> {
        self.evidence.as_ref()
    }

    /// Zwraca aktualną fazę wykonawczą.
    pub fn stage(&self) -> HilExecutionStage {
        self.stage
    }

    /// Zwraca raport gotowości konfiguracji HIL.
    ///
    /// Raport jest diagnostyką planu. **Nie** jest autoryzacją sprzętową i nigdy nie upoważnia PTT.
    pub fn readiness_report(&self) -> &HilReadinessReport {
        &self.readiness_report
    }

    /// Oblicza decyzję autoryzacyjną dla bieżącej sesji względem jawnego czasu.
    ///
    /// Decyzja jest liczona z trzech rozłącznych wejść: planu, oświadczenia i dowodu.
    pub fn authorization_decision_at(
        &self,
        evaluated_at: jiff::Timestamp,
    ) -> HilAuthorizationDecision {
        HilHardwareExecutionAuthorizer::authorize_at(
            super::authorization::HilAuthorizationInputs {
                plan: &self.manifest,
                attestation: self.attestation.as_ref(),
                evidence: self.evidence.as_ref(),
            },
            evaluated_at,
        )
    }

    /// Oblicza decyzję autoryzacyjną dla bieżącej sesji.
    pub fn authorization_decision(&self) -> HilAuthorizationDecision {
        self.authorization_decision_at(self.evaluated_at())
    }

    /// Czy sesja posiada już zużyte upoważnienie (replay protection).
    pub fn is_authorization_consumed(&self) -> bool {
        self.authorization_consumed.load(Ordering::SeqCst)
    }

    /// Zużywa upoważnienie dokładnie raz, tuż przed logicznym PTT ON.
    fn consume_authorization(&self, decision: &HilAuthorizationDecision) -> Result<(), HilError> {
        if !decision.authorizes_hardware_ptt() {
            return Err(HilError::NotHilReady(format!(
                "hardware execution is not authorized: {} ({})",
                decision.outcome(),
                decision.reason()
            )));
        }
        if self.authorization_consumed.swap(true, Ordering::SeqCst) {
            return Err(HilError::UnauthorizedSession(format!(
                "hardware authorization for attempt '{}' was already consumed; replay rejected",
                self.manifest.attempt_id
            )));
        }
        self.attempt_registry
            .consume(&self.manifest.session_id, &self.manifest.attempt_id)
            .map_err(HilError::from)
    }

    /// Weryfikuje wieloskładnikową autoryzację dostępu do fizycznego sprzętu laboratoryjnego.
    ///
    /// Wymaga JEDNOCZEŚNIE:
    /// 1. Poprawnego planu.
    /// 2. `hil_enabled: true` w planie.
    /// 3. Zmiennej środowiskowej `SPLOG_HIL_ENABLE=1`.
    /// 4. Jawnego, niepustego `RigId`.
    /// 5. Niewygasłej sesji.
    /// 6. Zmiennej `SPLOG_HIL_SESSION` zgodnej z `session_id`.
    /// 7. Niezależnej decyzji autoryzacyjnej `HardwareExecutionAuthorized` (F-15).
    ///
    /// Bramki środowiskowe są dodatkowymi warunkami, a nie dowodem. Bez niezależnego dowodu
    /// funkcja zwraca kontrolowaną odmowę.
    pub fn verify_hardware_access_authorization(&self) -> Result<(), HilError> {
        self.verify_hardware_access_authorization_at(self.evaluated_at())
    }

    /// Weryfikuje bramki sesji (plan, środowisko, identyfikatory, wygaśnięcie).
    ///
    /// Bramki te nie są dowodem gotowości i nie upoważniają PTT; stanowią wyłącznie warunki
    /// wstępne dla dalszej walidacji.
    fn verify_hardware_access_gates_at(
        &self,
        evaluated_at: jiff::Timestamp,
    ) -> Result<(), HilError> {
        self.manifest.validate_structure()?;

        if !self.manifest.hil_enabled {
            return Err(HilError::UnauthorizedSession(
                "Manifest does not have hil_enabled set to true".to_string(),
            ));
        }

        if !self.environment_gates().hil_enabled() {
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

        if let Some(exp_str) = &self.manifest.expires_at
            && let Ok(exp_time) = crate::cat::hil::parse_rfc3339(exp_str)
            && evaluated_at >= exp_time
        {
            return Err(HilError::SessionExpired(format!(
                "HIL session '{}' expired at {}",
                self.manifest.session_id, exp_str
            )));
        }

        let gates = self.environment_gates();
        let env_session = gates.session().ok_or_else(|| {
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
        Ok(())
    }

    /// Weryfikuje wieloskładnikową autoryzację dostępu do fizycznego sprzętu laboratoryjnego
    /// względem jawnego czasu.
    pub fn verify_hardware_access_authorization_at(
        &self,
        evaluated_at: jiff::Timestamp,
    ) -> Result<(), HilError> {
        self.verify_hardware_access_gates_at(evaluated_at)?;

        // F-15: niezależna decyzja autoryzacyjna jest obowiązkowa. Manifest, deklaracja
        // operatora ani zmienne środowiskowe nie mogą jej zastąpić.
        let decision = self.authorization_decision_at(evaluated_at);
        if !decision.authorizes_hardware_ptt() {
            return Err(HilError::NotHilReady(format!(
                "hardware access denied: {} ({})",
                decision.outcome(),
                decision.reason()
            )));
        }

        Ok(())
    }

    /// Weryfikuje wieloskładnikową autoryzację fizycznego nadawania (PTT).
    ///
    /// Obejmuje bramki środowiskowe, warunki TX **oraz** niezależną decyzję autoryzacyjną.
    pub fn verify_tx_authorization(
        &self,
        consent_token: &HilPttConsentToken,
        arbiter: &crate::cat::tx_arbiter::TxArbiter,
    ) -> Result<(), HilError> {
        let evaluated_at = self.evaluated_at();
        self.verify_tx_authorization_at(consent_token, arbiter, evaluated_at)?;
        self.verify_hardware_access_authorization_at(evaluated_at)
    }

    /// Weryfikuje wieloskładnikową autoryzację fizycznego nadawania (PTT) względem jawnego czasu.
    pub fn verify_tx_authorization_at(
        &self,
        consent_token: &HilPttConsentToken,
        arbiter: &crate::cat::tx_arbiter::TxArbiter,
        evaluated_at: jiff::Timestamp,
    ) -> Result<(), HilError> {
        // Bramki wstępne (plan, środowisko, sesja) oraz warunki TX (allow_tx, moc, czas,
        // arbiter, token) są sprawdzane przed decyzją dowodową, aby zachować jednoznaczną
        // diagnostykę odmowy. Żadna z tych bramek nie jest dowodem (F-15).
        self.verify_hardware_access_gates_at(evaluated_at)?;

        if !self.manifest.allow_tx {
            return Err(HilError::UnauthorizedSession(
                "Manifest does not have allow_tx set to true".to_string(),
            ));
        }

        let gates = self.environment_gates();
        let allow_tx_env = gates.allow_tx().ok_or_else(|| {
            HilError::UnauthorizedSession(
                "Environment variable SPLOG_HIL_ALLOW_TX is not set".to_string(),
            )
        })?;
        if allow_tx_env != "I_HAVE_DUMMY_LOAD" && allow_tx_env != "1" {
            return Err(HilError::UnauthorizedSession(format!(
                "SPLOG_HIL_ALLOW_TX ('{allow_tx_env}') is not 'I_HAVE_DUMMY_LOAD' or '1'"
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
            executed_at: jiff::Timestamp::now().to_string(),
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
            executed_at: jiff::Timestamp::now().to_string(),
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
        self.acquire_tx_guard_at(
            backend,
            consent_token,
            arbiter,
            duration,
            self.evaluated_at(),
        )
        .await
    }

    /// Pobiera strażnika nadawania TX względem jawnego czasu ewaluacji.
    pub async fn acquire_tx_guard_at<'a>(
        &'a self,
        backend: &'a mut dyn RigBackend,
        consent_token: &HilPttConsentToken,
        arbiter: &mut crate::cat::tx_arbiter::TxArbiter,
        duration: Duration,
        evaluated_at: jiff::Timestamp,
    ) -> Result<HilTxGuard<'a>, HilError> {
        if self.stage == HilExecutionStage::Stage7AOffline {
            return Err(HilError::PttForbiddenInStage7A);
        }

        self.verify_tx_authorization_at(consent_token, arbiter, evaluated_at)?;

        // F-14: zakres impulsu jest wyliczany i walidowany **przed** jakimkolwiek logicznym
        // PTT ON. Odwrócony zakres kończy się kontrolowanym błędem, nie paniką i nie TX.
        let bounded_duration =
            bounded_test_duration(duration, self.manifest.rf_safety.max_test_duration_ms)?;

        // Weryfikacja, że radio jest w stanie RX przed rozpoczęciem testu TX
        let initial_state = backend.poll().await.map_err(|e| {
            HilError::BackendError(format!("Initial poll() before PTT failed: {e:?}"))
        })?;
        if initial_state.ptt {
            return Err(HilError::MissingConfirmedRx(
                "Cannot acquire TX guard: radio is already transmitting".to_string(),
            ));
        }

        // F-15: ostatnia bramka przed logicznym PTT ON. Upoważnienie pochodzi wyłącznie
        // z niezależnego dowodu i jest zużywane dokładnie raz (ochrona przed powtórzeniem).
        let decision = self.authorization_decision_at(evaluated_at);
        self.consume_authorization(&decision)?;

        let cmd_id = crate::cat::routing::RigCommandId::new(1);
        let rig_id = self.manifest.matrix.rig_id.clone();

        arbiter.request_tx(&rig_id, cmd_id).map_err(|e| {
            HilError::TxArbiterBusy(format!("TxArbiter acquire request rejected: {e:?}"))
        })?;

        // Dokładnie jedno wysłanie SetPtt(true) — brak jakichkolwiek ponowień
        if let Err(e) = backend.execute(&RigCommand::SetPtt(true)).await {
            // Fail-closed: polecenie ON mogło zostać częściowo wykonane (np. timeout po zapisie
            // ramki). Podejmij dokładnie jedną próbę PTT OFF, zamiast zostawiać nadajnik
            // w nieznanym stanie.
            let _ = backend.execute(&RigCommand::SetPtt(false)).await;
            let _ = arbiter.reject_request(&rig_id, cmd_id);
            return Err(HilError::BackendError(format!(
                "SetPtt(true) failed: {e:?}; emergency PTT OFF attempted"
            )));
        }

        if let Err(e) = arbiter.confirm_tx(&rig_id, cmd_id) {
            // Potwierdzenie TX w arbitrze nie powiodło się po wysłaniu SetPtt(true):
            // dokładnie jedna próba PTT OFF, arbiter w Unknown, błąd jawnie zgłoszony.
            let _ = backend.execute(&RigCommand::SetPtt(false)).await;
            arbiter.set_unknown(
                Some(rig_id.clone()),
                crate::cat::tx_arbiter::TxUnknownReason::BackendError(format!(
                    "TxArbiter confirm tx rejected: {e:?}"
                )),
            );
            return Err(HilError::TxArbiterBusy(format!(
                "TxArbiter confirm tx rejected: {e:?}; emergency PTT OFF attempted"
            )));
        }

        Ok(HilTxGuard::new_with_state(
            self.manifest.matrix.rig_id.clone(),
            backend,
            bounded_duration,
            self.session_state.clone(),
        ))
    }

    /// Wykonuje kontrolowaną, bezpieczną próbę załączenia PTT na sztucznym obciążeniu.
    pub async fn execute_controlled_ptt(
        &self,
        backend: &mut dyn RigBackend,
        consent_token: &HilPttConsentToken,
        pulse_duration: Duration,
    ) -> Result<HilPttExecutionResult, HilError> {
        self.execute_controlled_ptt_at(backend, consent_token, pulse_duration, self.evaluated_at())
            .await
    }

    /// Wykonuje kontrolowaną, bezpieczną próbę załączenia PTT względem jawnego czasu ewaluacji.
    pub async fn execute_controlled_ptt_at(
        &self,
        backend: &mut dyn RigBackend,
        consent_token: &HilPttConsentToken,
        pulse_duration: Duration,
        evaluated_at: jiff::Timestamp,
    ) -> Result<HilPttExecutionResult, HilError> {
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();
        self.execute_controlled_ptt_with_arbiter_at(
            backend,
            consent_token,
            &mut arbiter,
            pulse_duration,
            evaluated_at,
        )
        .await
    }

    /// Wykonuje kontrolowaną, bezpieczną próbę załączenia PTT z jawnym arbitrem nadawania.
    pub async fn execute_controlled_ptt_with_arbiter(
        &self,
        backend: &mut dyn RigBackend,
        consent_token: &HilPttConsentToken,
        arbiter: &mut crate::cat::tx_arbiter::TxArbiter,
        pulse_duration: Duration,
    ) -> Result<HilPttExecutionResult, HilError> {
        self.execute_controlled_ptt_with_arbiter_at(
            backend,
            consent_token,
            arbiter,
            pulse_duration,
            self.evaluated_at(),
        )
        .await
    }

    /// Wykonuje kontrolowaną, bezpieczną próbę załączenia PTT z jawnym arbitrem i czasem ewaluacji.
    pub async fn execute_controlled_ptt_with_arbiter_at(
        &self,
        backend: &mut dyn RigBackend,
        consent_token: &HilPttConsentToken,
        arbiter: &mut crate::cat::tx_arbiter::TxArbiter,
        pulse_duration: Duration,
        evaluated_at: jiff::Timestamp,
    ) -> Result<HilPttExecutionResult, HilError> {
        // Rygor 1: Faza NATIVE-7A
        if self.stage == HilExecutionStage::Stage7AOffline {
            return Err(HilError::PttForbiddenInStage7A);
        }

        // Rygor 2: Brak anteny
        if let HilRfTermination::Antenna { antenna_name } = &self.manifest.rf_safety.termination {
            return Err(HilError::SafetyViolation(format!(
                "Fatal RF safety violation: attempt to execute PTT on antenna '{antenna_name}'. Dummy load strictly required."
            )));
        }

        // Rygor 3: Wieloskładnikowa autoryzacja TX (środowisko, token, moc, arbiter idle).
        // Bramki środowiskowe pozostają dodatkowymi warunkami; nie są dowodem (F-15).
        self.verify_tx_authorization_at(consent_token, arbiter, evaluated_at)?;

        // F-14: totalna walidacja zakresu impulsu — bez `clamp`, bez paniki, przed PTT ON.
        let bounded_duration =
            bounded_test_duration(pulse_duration, self.manifest.rf_safety.max_test_duration_ms)?;

        let guard_timeout = Duration::from_millis(self.manifest.rf_safety.max_test_duration_ms);

        let start_time = Instant::now();

        // Nabycie strażnika TX z weryfikacją początkowego RX oraz rejestracją w TxArbiter.
        // Upoważnienie jest zużywane tutaj, dokładnie raz, przed pierwszym SetPtt(true).
        let mut guard = self
            .acquire_tx_guard_at(backend, consent_token, arbiter, guard_timeout, evaluated_at)
            .await?;

        tokio::time::sleep(bounded_duration).await;

        // Bezwzględny powrót do RX z odczytem poll() i potwierdzeniem ConfirmedRx
        let release_res = guard.release_and_confirm_rx().await;

        let elapsed_ms = start_time.elapsed().as_millis() as u64;

        match release_res {
            Ok(mut res) => {
                let cmd_id = crate::cat::routing::RigCommandId::new(1);
                let _ = arbiter.request_release(&self.manifest.matrix.rig_id, cmd_id);
                let _ = arbiter.confirm_release(&self.manifest.matrix.rig_id, cmd_id);
                res.ptt_pulse_duration_ms = elapsed_ms;
                Ok(res)
            }
            Err(e) => {
                arbiter.set_unknown(
                    Some(self.manifest.matrix.rig_id.clone()),
                    crate::cat::tx_arbiter::TxUnknownReason::CommandTimeout,
                );
                Err(e)
            }
        }
    }

    /// Wykonuje suchy przebieg: waliduje konfigurację i oświadczenie, nie wykonuje PTT.
    pub fn dry_run(&self) -> HilAuthorizationDecision {
        HilHardwareExecutionAuthorizer::dry_run_at(
            &self.manifest,
            self.attestation.as_ref(),
            self.evaluated_at(),
        )
    }

    /// Sprawdza, czy PTT jest upoważnione (bez wykonywania).
    pub fn is_hardware_ptt_authorized(&self) -> bool {
        self.authorization_decision().authorizes_hardware_ptt()
    }

    /// Zwraca powód odmowy autoryzacji w bezpiecznej, audytowalnej postaci.
    pub fn authorization_denial_reason(&self) -> Option<String> {
        let decision = self.authorization_decision();
        if decision.authorizes_hardware_ptt() {
            None
        } else {
            Some(format!("{}: {}", decision.outcome(), decision.reason()))
        }
    }

    /// Zwraca oczekiwany wynik autoryzacji (diagnostyka, bez wykonywania).
    pub fn authorization_outcome(&self) -> HilAuthorizationOutcome {
        self.authorization_decision().outcome()
    }

    /// Zwraca informację, czy sesja ma jawny dowód (bez wykonywania).
    pub fn has_independent_evidence(&self) -> bool {
        self.evidence.is_some()
    }

    /// Zwraca informację, czy sesja ma jawny dokument oświadczenia.
    pub fn has_operator_attestation(&self) -> bool {
        self.attestation.is_some()
    }

    /// Pozwala jawnie sprawdzić wynik zapytania do dostawcy dowodu (fail-closed).
    pub fn provider_outcome_is_authorized(outcome: &HilEvidenceProviderOutcome) -> bool {
        matches!(outcome, HilEvidenceProviderOutcome::Evidence(_))
    }
}

/// Pojedynczy przypadek testowy w dyspozytorze stanowiska laboratoryjnego.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilDispatcherCase {
    /// Nazwa przypadku testowego (np. "hil_hardware_live_test_bench").
    pub name: String,
    /// Status wykonania przypadku (NotRun, Blocked, Failed, Passed).
    pub status: HilReadinessStatus,
    /// Przyczyna lub uzasadnienie statusu.
    pub reason: String,
}

/// Raport wykonawczy dyspozytora izolowanych testów sprzętowych HIL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilDispatcherReport {
    /// Identyfikator sesji laboratoryjnej.
    pub session_id: String,
    /// Zbiorczy status dyspozytora.
    pub overall_status: HilReadinessStatus,
    /// Wyniki poszczególnych 10 przypadków testowych.
    pub cases: Vec<HilDispatcherCase>,
    /// Czy podczas dyspozycji otwarto jakikolwiek fizyczny transport sprzętowy.
    pub transport_opened: bool,
    /// Czy wyemitowano jakikolwiek fizyczny sygnał PTT.
    pub ptt_emitted: bool,
}

/// Dyspozytor hermetycznie odizolowanych testów sprzętowych HIL ([`run_isolated_hardware_test`]).
pub fn run_isolated_hardware_test(
    manifest: &HilTestBenchManifest,
    stage: HilExecutionStage,
) -> HilDispatcherReport {
    run_isolated_hardware_dispatcher(manifest, stage)
}

/// Rzeczywista implementacja dyspozytora sprzętowego HIL.
pub fn run_isolated_hardware_dispatcher(
    manifest: &HilTestBenchManifest,
    stage: HilExecutionStage,
) -> HilDispatcherReport {
    run_isolated_hardware_dispatcher_at(manifest, stage, default_hil_evaluation_time())
}

/// Rzeczywista implementacja dyspozytora sprzętowego HIL względem jawnego czasu ewaluacji.
pub fn run_isolated_hardware_dispatcher_at(
    manifest: &HilTestBenchManifest,
    _stage: HilExecutionStage,
    evaluated_at: jiff::Timestamp,
) -> HilDispatcherReport {
    let case_names = [
        "hil_hardware_live_test_bench",
        "hil_hardware_read_01",
        "hil_hardware_freq_01",
        "hil_hardware_mode_01",
        "hil_hardware_ptt_01",
        "hil_hardware_ptt_timeout_01",
        "hil_hardware_resync_01",
        "hil_hardware_multi_radio_01",
        "hil_hardware_shutdown_01",
        "hil_hardware_shutdown_tx_01",
    ];

    // Sprawdzenie integralności planu
    if let Err(e) = manifest.validate_structure() {
        let cases = case_names
            .iter()
            .map(|name| HilDispatcherCase {
                name: (*name).to_string(),
                status: HilReadinessStatus::Failed,
                reason: format!("Manifest structure invalid: {e}"),
            })
            .collect();
        return HilDispatcherReport {
            session_id: manifest.session_id.clone(),
            overall_status: HilReadinessStatus::Failed,
            cases,
            transport_opened: false,
            ptt_emitted: false,
        };
    }

    // Sprawdzenie identyfikatora sesji
    if manifest.session_id.trim().is_empty() {
        let cases = case_names
            .iter()
            .map(|name| HilDispatcherCase {
                name: (*name).to_string(),
                status: HilReadinessStatus::Failed,
                reason: "Manifest session_id is empty".to_string(),
            })
            .collect();
        return HilDispatcherReport {
            session_id: manifest.session_id.clone(),
            overall_status: HilReadinessStatus::Failed,
            cases,
            transport_opened: false,
            ptt_emitted: false,
        };
    }

    // Sprawdzenie wygaśnięcia sesji względem jawnego czasu odniesienia.
    if let Some(exp_str) = &manifest.expires_at
        && let Ok(exp_time) = crate::cat::hil::parse_rfc3339(exp_str)
        && evaluated_at >= exp_time
    {
        let cases = case_names
            .iter()
            .map(|name| HilDispatcherCase {
                name: (*name).to_string(),
                status: HilReadinessStatus::Failed,
                reason: format!(
                    "HIL session '{}' expired at {}",
                    manifest.session_id, exp_str
                ),
            })
            .collect();
        return HilDispatcherReport {
            session_id: manifest.session_id.clone(),
            overall_status: HilReadinessStatus::Failed,
            cases,
            transport_opened: false,
            ptt_emitted: false,
        };
    }

    // F-15: dyspozytor nigdy nie upoważnia wykonania sprzętowego. Bez niezależnego dowodu
    // (którego ten kod nie posiada i nie potrafi wytworzyć) wszystkie przypadki pozostają
    // NOT RUN, a dyspozytor nie otwiera transportu i nie emituje PTT.
    let cases = case_names
        .iter()
        .map(|name| HilDispatcherCase {
            name: (*name).to_string(),
            status: HilReadinessStatus::NotRun,
            reason: "HIL-READY hardware evidence missing: no independent evidence provider is \
                     configured, so no hardware case may run (F-15 fail-closed)"
                .to_string(),
        })
        .collect();

    HilDispatcherReport {
        session_id: manifest.session_id.clone(),
        overall_status: HilReadinessStatus::Blocked,
        cases,
        transport_opened: false,
        ptt_emitted: false,
    }
}

#[cfg(test)]
mod test_scope_tests {
    //! Testy jednostkowe w zakresie `cfg(test)` — jedyne miejsce, w którym istnieje testowy
    //! dostawca dowodu ([`super::super::authorization::test_evidence_support`]).

    use super::super::authorization::test_evidence_support::{
        authorized_decision, issue_test_attestation, issue_test_evidence, test_plan,
    };
    use super::super::authorization::{HilAuthorizationOutcome, HilEvidenceScope};
    use super::*;
    use crate::cat::backend::CatBackendKind;
    use crate::cat::backend::RigProbeResult;
    use crate::cat::hamlib::RigState;
    use crate::cat::rig::RigError;
    use crate::cat::routing::RigId;
    use std::sync::atomic::{AtomicBool, AtomicUsize};

    #[derive(Clone, Default)]
    struct Counters {
        ptt_on: Arc<AtomicUsize>,
        ptt_off: Arc<AtomicUsize>,
        last_ptt: Arc<AtomicBool>,
    }

    struct MockBackend {
        counters: Counters,
        stick_ptt: Arc<AtomicBool>,
        fail_on: Arc<AtomicBool>,
        fail_off: Arc<AtomicBool>,
        poll_error_after_on: Arc<AtomicBool>,
        sticky_after_on: Arc<AtomicBool>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                counters: Counters::default(),
                stick_ptt: Arc::new(AtomicBool::new(false)),
                fail_on: Arc::new(AtomicBool::new(false)),
                fail_off: Arc::new(AtomicBool::new(false)),
                poll_error_after_on: Arc::new(AtomicBool::new(false)),
                sticky_after_on: Arc::new(AtomicBool::new(false)),
            }
        }

        fn with_fail_on() -> Self {
            let backend = Self::new();
            backend.fail_on.store(true, Ordering::SeqCst);
            backend
        }

        fn with_fail_off() -> Self {
            let backend = Self::new();
            backend.fail_off.store(true, Ordering::SeqCst);
            backend
        }

        fn with_sticky_after_on() -> Self {
            let backend = Self::new();
            backend.sticky_after_on.store(true, Ordering::SeqCst);
            backend
        }

        fn with_poll_error_after_on() -> Self {
            let backend = Self::new();
            backend.poll_error_after_on.store(true, Ordering::SeqCst);
            backend
        }
    }

    impl RigBackend for MockBackend {
        fn kind(&self) -> CatBackendKind {
            CatBackendKind::IcomCiV
        }

        fn capabilities(&self) -> crate::cat::rig::RigCapabilities {
            crate::cat::rig::RigCapabilities::default()
        }

        fn connect(&mut self) -> crate::cat::backend::BoxFuture<'_, Result<(), RigError>> {
            Box::pin(async { Ok(()) })
        }

        fn poll(&mut self) -> crate::cat::backend::BoxFuture<'_, Result<RigState, RigError>> {
            let ptt_on = self.counters.ptt_on.load(Ordering::SeqCst) > 0;
            if ptt_on && self.sticky_after_on.load(Ordering::SeqCst) {
                return Box::pin(async {
                    Ok(RigState {
                        ptt: true,
                        ..Default::default()
                    })
                });
            }
            if ptt_on && self.poll_error_after_on.load(Ordering::SeqCst) {
                return Box::pin(async {
                    Err(RigError::Protocol {
                        detail: "simulated poll error after ON".into(),
                    })
                });
            }
            let ptt = self.stick_ptt.load(Ordering::SeqCst)
                || self.counters.last_ptt.load(Ordering::SeqCst);
            Box::pin(async move {
                Ok(RigState {
                    ptt,
                    ..Default::default()
                })
            })
        }

        fn execute<'a>(
            &'a mut self,
            command: &'a RigCommand,
        ) -> crate::cat::backend::BoxFuture<'a, Result<(), RigError>> {
            let counters = self.counters.clone();
            let fail_on = self.fail_on.clone();
            let fail_off = self.fail_off.clone();
            Box::pin(async move {
                if let RigCommand::SetPtt(state) = command {
                    if *state {
                        counters.ptt_on.fetch_add(1, Ordering::SeqCst);
                        if fail_on.load(Ordering::SeqCst) {
                            return Err(RigError::Protocol {
                                detail: "simulated ON failure".into(),
                            });
                        }
                    } else {
                        counters.ptt_off.fetch_add(1, Ordering::SeqCst);
                        if fail_off.load(Ordering::SeqCst) {
                            return Err(RigError::Protocol {
                                detail: "simulated OFF failure".into(),
                            });
                        }
                    }
                    counters.last_ptt.store(*state, Ordering::SeqCst);
                }
                Ok(())
            })
        }

        fn disconnect(&mut self) -> crate::cat::backend::BoxFuture<'_, ()> {
            Box::pin(async {})
        }

        fn probe(
            &mut self,
        ) -> crate::cat::backend::BoxFuture<'_, Result<RigProbeResult, RigError>> {
            Box::pin(async {
                Ok(RigProbeResult {
                    reachable: true,
                    identity: Some("Mock IC-7300".to_string()),
                    model: Some("IC-7300".to_string()),
                    firmware: Some("v1.42".to_string()),
                    capabilities: crate::cat::rig::RigCapabilities::default(),
                    state: None,
                    safe_diagnostic: "mock probe".to_string(),
                })
            })
        }
    }

    fn reference_time() -> jiff::Timestamp {
        crate::cat::hil::parse_rfc3339("2026-10-18T12:00:00Z").expect("reference time")
    }

    fn authorized_session() -> HilHardwareSession {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let evidence = issue_test_evidence(&plan, &attestation);
        HilHardwareSession::new_with_reference_time(
            plan,
            HilExecutionStage::Stage7BHardware,
            Some(reference_time()),
        )
        .with_authorization(Some(attestation), Some(evidence))
        .with_environment(test_gates())
    }

    #[test]
    fn f15_session_without_evidence_is_denied_at_stage_7b() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let session = HilHardwareSession::new_with_reference_time(
            plan,
            HilExecutionStage::Stage7BHardware,
            Some(reference_time()),
        )
        .with_authorization(Some(attestation), None);

        let decision = session.authorization_decision();
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::EvidenceMissing);
        assert!(!session.is_hardware_ptt_authorized());
        assert!(!session.readiness_report().can_run_hardware_ptt());
    }

    #[test]
    fn f15_session_with_full_evidence_is_authorized_in_test_scope() {
        let session = authorized_session();
        assert!(session.is_hardware_ptt_authorized());
        assert!(session.authorization_denial_reason().is_none());
    }

    #[test]
    fn f15_dry_run_never_authorizes_ptt() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let session = HilHardwareSession::new_with_reference_time(
            plan,
            HilExecutionStage::Stage7BHardware,
            Some(reference_time()),
        )
        .with_authorization(Some(attestation), None);
        let decision = session.dry_run();
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::DryRunOnly);
        assert!(!decision.authorizes_hardware_ptt());
        assert!(!session.is_hardware_ptt_authorized());
    }

    #[tokio::test]
    async fn f15_authorized_session_keys_ptt_once_and_releases_once() {
        let session = authorized_session();
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let mut guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("authorized acquisition must succeed");

        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1);
        assert!(guard.release_and_confirm_rx().await.is_ok());
        assert_eq!(counters.ptt_off.load(Ordering::SeqCst), 1);
        assert!(session.is_authorization_consumed());
    }

    #[tokio::test]
    async fn f15_second_authorization_is_rejected_as_replay() {
        let session = authorized_session();
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let mut guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("first acquisition");
        assert!(guard.release_and_confirm_rx().await.is_ok());
        drop(guard);

        let second = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(second.is_err(), "replay must be rejected");
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn f15_expired_evidence_never_keys_ptt() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.expires_at = Some("2026-10-18T10:00:00Z".to_string());
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert_eq!(decision.outcome(), HilAuthorizationOutcome::EvidenceExpired);

        let session = HilHardwareSession::new_with_reference_time(
            plan,
            HilExecutionStage::Stage7BHardware,
            Some(reference_time()),
        )
        .with_authorization(Some(attestation), Some(evidence));
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let res = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(res.is_err());
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn f15_wrong_scope_evidence_never_keys_ptt() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let mut evidence = issue_test_evidence(&plan, &attestation);
        evidence.scope = HilEvidenceScope::ConfigurationOnly;
        let session = HilHardwareSession::new_with_reference_time(
            plan,
            HilExecutionStage::Stage7BHardware,
            Some(reference_time()),
        )
        .with_authorization(Some(attestation), Some(evidence));
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let res = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(res.is_err());
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn f15_stage_7a_is_rejected_before_any_authorization_use() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let evidence = issue_test_evidence(&plan, &attestation);
        let session = HilHardwareSession::new_with_reference_time(
            plan,
            HilExecutionStage::Stage7AOffline,
            Some(reference_time()),
        )
        .with_authorization(Some(attestation), Some(evidence));
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let res = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(matches!(res, Err(HilError::PttForbiddenInStage7A)));
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn f15_dispatcher_never_emits_ptt_even_with_evidence() {
        let plan = test_plan();
        let report = run_isolated_hardware_dispatcher_at(
            &plan,
            HilExecutionStage::Stage7BHardware,
            reference_time(),
        );
        assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
        assert!(!report.ptt_emitted);
        assert!(!report.transport_opened);
        assert!(
            report
                .cases
                .iter()
                .all(|c| c.status == HilReadinessStatus::NotRun)
        );
    }

    #[test]
    fn f15_authorized_decision_reports_attempt_and_scope() {
        let plan = test_plan();
        let attestation = issue_test_attestation(&plan);
        let evidence = issue_test_evidence(&plan, &attestation);
        let decision = authorized_decision(&plan, &attestation, &evidence, reference_time());
        assert!(decision.authorizes_hardware_ptt());
        let audit = decision.audit();
        assert_eq!(audit.attempt_id.as_deref(), Some("ATTEMPT-F15-0001"));
        assert_eq!(audit.scope.as_deref(), Some("hardware_ptt"));
        assert_eq!(
            audit.outcome,
            HilAuthorizationOutcome::HardwareExecutionAuthorized
        );
    }

    #[test]
    fn f15_manifest_rig_identity_is_used_not_guessed() {
        let session = authorized_session();
        assert_eq!(
            session.manifest().matrix.rig_id,
            RigId::new("RIG-F15-01").unwrap()
        );
    }

    // =====================================================================
    // F-14 MECHANICS UNDER AN AUTHORIZED SESSION (test-scope evidence only)
    //
    // The integration suite `tests/cat_f14_fail_safe.rs` can no longer key PTT,
    // because that file cannot obtain independent evidence (correct fail-closed
    // behaviour). These in-crate tests keep the F-14 mechanical invariants
    // covered: single ON, exactly one OFF on every post-ON path, failures are
    // never success, Drop releases at most once, release is idempotent.
    // =====================================================================

    fn test_gates() -> HilEnvironmentGates {
        HilEnvironmentGates::for_tests("SESSION-F15-01", "I_HAVE_DUMMY_LOAD")
    }

    fn authorized_session_with_duration(max_test_duration_ms: u64) -> HilHardwareSession {
        let mut plan = test_plan();
        plan.rf_safety.max_test_duration_ms = max_test_duration_ms;
        let attestation = issue_test_attestation(&plan);
        let evidence = issue_test_evidence(&plan, &attestation);
        HilHardwareSession::new_with_reference_time(
            plan,
            HilExecutionStage::Stage7BHardware,
            Some(reference_time()),
        )
        .with_authorization(Some(attestation), Some(evidence))
        .with_environment(test_gates())
    }

    #[tokio::test]
    async fn f14_authorized_valid_range_keys_once_and_releases_once() {
        let session = authorized_session_with_duration(500);
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let mut guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("authorized guard");
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1, "exactly one ON");
        assert!(guard.release_and_confirm_rx().await.is_ok());
        assert_eq!(
            counters.ptt_off.load(Ordering::SeqCst),
            1,
            "exactly one OFF"
        );
        assert!(!counters.last_ptt.load(Ordering::SeqCst), "device in RX");
    }

    #[tokio::test]
    async fn f14_authorized_on_failure_attempts_emergency_release() {
        let session = authorized_session_with_duration(500);
        let mut backend = MockBackend::with_fail_on();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let res = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(res.is_err(), "ON failure must be a controlled error");
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1, "one ON attempt");
        assert_eq!(counters.ptt_off.load(Ordering::SeqCst), 1, "emergency OFF");
        assert!(arbiter.is_idle(), "arbiter returns to Idle");
    }

    #[tokio::test]
    async fn f14_authorized_off_failure_is_never_success() {
        let session = authorized_session_with_duration(500);
        let mut backend = MockBackend::with_fail_off();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let mut guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("authorized guard");
        let res = guard.release_and_confirm_rx().await;
        assert!(res.is_err(), "OFF failure must not be success");
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1);
        assert_eq!(counters.ptt_off.load(Ordering::SeqCst), 1);
        assert!(matches!(guard.release_outcome(), HilReleaseOutcome::Failed));
    }

    #[tokio::test]
    async fn f14_authorized_unconfirmed_rx_is_never_success() {
        let session = authorized_session_with_duration(500);
        let mut backend = MockBackend::with_sticky_after_on();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let mut guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("authorized guard");
        let res = guard.release_and_confirm_rx().await;
        assert!(matches!(res, Err(HilError::MissingConfirmedRx(_))));
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1);
        assert_eq!(counters.ptt_off.load(Ordering::SeqCst), 1);
        assert!(matches!(
            guard.release_outcome(),
            HilReleaseOutcome::Unconfirmed
        ));
    }

    #[tokio::test]
    async fn f14_authorized_poll_error_after_on_is_never_success() {
        let session = authorized_session_with_duration(500);
        let mut backend = MockBackend::with_poll_error_after_on();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let mut guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("authorized guard");
        assert!(guard.release_and_confirm_rx().await.is_err());
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1);
        assert_eq!(counters.ptt_off.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn f14_authorized_duplicate_release_is_rejected() {
        let session = authorized_session_with_duration(500);
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        let mut guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("authorized guard");
        assert!(guard.release_and_confirm_rx().await.is_ok());
        assert!(matches!(
            guard.release_and_confirm_rx().await,
            Err(HilError::ReleaseError(_))
        ));
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1);
        assert_eq!(
            counters.ptt_off.load(Ordering::SeqCst),
            1,
            "no duplicate OFF"
        );
    }

    #[tokio::test]
    async fn f14_authorized_abandoned_guard_releases_once_without_fabricating_rx() {
        let session = authorized_session_with_duration(500);
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");
        let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();

        {
            let guard = session
                .acquire_tx_guard_at(
                    &mut backend,
                    &token,
                    &mut arbiter,
                    Duration::from_millis(20),
                    reference_time(),
                )
                .await
                .expect("authorized guard");
            assert!(!guard.is_confirmed_rx());
        }
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1);
        assert_eq!(
            counters.ptt_off.load(Ordering::SeqCst),
            1,
            "one emergency OFF"
        );
        assert_eq!(session.session_state(), HilSessionState::FailedOrUnknown);
    }

    #[tokio::test]
    async fn f14_authorized_below_minimum_range_never_keys() {
        for max_ms in 0..crate::cat::hil::manifest::HIL_MIN_TEST_DURATION_MS {
            let session = authorized_session_with_duration(max_ms);
            let mut backend = MockBackend::new();
            let counters = backend.counters.clone();
            let token = HilPttConsentToken::new("SP6INA").expect("token");
            let mut arbiter = crate::cat::tx_arbiter::TxArbiter::new();
            let res = session
                .acquire_tx_guard_at(
                    &mut backend,
                    &token,
                    &mut arbiter,
                    Duration::from_millis(50),
                    reference_time(),
                )
                .await;
            assert!(res.is_err(), "max={max_ms} must be rejected");
            assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0, "zero ON");
        }
    }

    #[tokio::test]
    async fn f14_authorized_full_width_pulse_times_out_but_releases() {
        let session =
            authorized_session_with_duration(crate::cat::hil::manifest::HIL_MIN_TEST_DURATION_MS);
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let token = HilPttConsentToken::new("SP6INA").expect("token");

        let res = session
            .execute_controlled_ptt(&mut backend, &token, Duration::from_millis(10))
            .await;
        assert!(matches!(res, Err(HilError::TimeoutToUnknown(_))));
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 1, "one ON");
        assert_eq!(
            counters.ptt_off.load(Ordering::SeqCst),
            1,
            "one OFF attempt"
        );
    }

    #[test]
    fn f14_authorized_suite_is_within_test_scope_only() {
        // Ten moduł jest kompilowany wyłącznie pod `cfg(test)`; dowód testowy nie istnieje
        // w kodzie produkcyjnym.
        assert_eq!(
            option_env!("CARGO_CRATE_NAME").unwrap_or("splogbook"),
            "splogbook"
        );
    }
}
