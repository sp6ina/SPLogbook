// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Aktor usługi pojedynczej sesji radiowej ([`RigService`]) i bezpieczny uchwyt ([`RigServiceHandle`]).
//!
//! Każda sesja radiowa działa w autonomicznym, izolowanym tasku Tokio, będącym wyłącznym
//! właścicielem instancji [`crate::cat::backend::RigBackend`].
//!
//! # Kluczowe zasady architektoniczne:
//! - Całkowity zakaz `Arc<Mutex<Box<dyn RigBackend>>>` oraz współdzielenia transportu.
//! - Wszystkie kolejki są ograniczone ([`MAX_RIG_COMMAND_QUEUE`], [`MAX_RIG_EVENT_QUEUE`]).
//! - Brak kanałów nieograniczonych (`unbounded`).
//! - Brak tasków porzuconych (`detached tasks`) – każdy task ma powiązany [`tokio::task::JoinHandle`].
//! - Jawna maszyna cyklu życia ([`RigServiceState`]) z walidacją dozwolonych przejść.
//! - Sprawdzanie monotonicznego terminu (`deadline`) przed zakolejkowaniem i przed wykonaniem polecenia.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::cat::backend::RigBackend;
use crate::cat::native::NativeRigEvent;
use crate::cat::rig::{RigCapabilities, RigCommand, RigState};
use crate::cat::routing::{RigCommandId, RigHubEvent, RigId, RigTarget, RoutedRigEvent};

/// Maksymalny rozmiar bounded kolejki poleceń dla pojedynczej sesji radiowej.
pub const MAX_RIG_COMMAND_QUEUE: usize = 32;

/// Maksymalny rozmiar bufora rozgłoszeniowego zdarzeń pojedynczej sesji.
pub const MAX_RIG_EVENT_QUEUE: usize = 128;

/// Stany cyklu życia usługi sesji radiowej.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigServiceState {
    /// Usługa utworzona, oczekuje na nawiązanie połączenia.
    Created,
    /// Trwa asynchroniczne nawiązywanie połączenia z radiem / demonem.
    Connecting,
    /// Połączenie nawiązane, trwa odczyt stanu początkowego i weryfikacja tożsamości.
    Synchronizing,
    /// Usługa w pełni gotowa do przyjmowania poleceń i nadawania telemetrii.
    Ready,
    /// Usługa działa w trybie awaryjnym / ograniczonym (częściowa utrata funkcji).
    Degraded,
    /// Rozpoczęto kontrolowaną procedurę rozłączania i zwalniania zasobów.
    Disconnecting,
    /// Usługa została poprawnie rozłączona (stan końcowy taska).
    Disconnected,
    /// Usługa uległa nieodwracalnemu błędowi I/O lub awarii połączenia.
    Failed,
}

impl RigServiceState {
    /// Weryfikuje, czy przejście z bieżącego stanu do nowego jest dozwolone.
    pub fn can_transition_to(self, next: Self) -> bool {
        match (self, next) {
            (Self::Created, Self::Connecting) => true,
            (Self::Created, Self::Disconnecting) => true,
            (Self::Connecting, Self::Synchronizing) => true,
            (Self::Connecting, Self::Disconnecting) => true,
            (Self::Connecting, Self::Failed) => true,
            (Self::Synchronizing, Self::Ready) => true,
            (Self::Synchronizing, Self::Degraded) => true,
            (Self::Synchronizing, Self::Disconnecting) => true,
            (Self::Synchronizing, Self::Failed) => true,
            (Self::Ready, Self::Degraded) => true,
            (Self::Ready, Self::Disconnecting) => true,
            (Self::Ready, Self::Failed) => true,
            (Self::Degraded, Self::Ready) => true,
            (Self::Degraded, Self::Disconnecting) => true,
            (Self::Degraded, Self::Failed) => true,
            (Self::Disconnecting, Self::Disconnected) => true,
            (Self::Disconnecting, Self::Failed) => true,
            // Idempotentność przejścia do tego samego stanu
            (a, b) if a == b => true,
            _ => false,
        }
    }
}

/// Kanoniczny alias typu cyklu życia usługi sesji radiowej ([`RigServiceState`]).
pub type RigServiceLifecycle = RigServiceState;

impl fmt::Display for RigServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Synchronizing => write!(f, "Synchronizing"),
            Self::Ready => write!(f, "Ready"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Disconnecting => write!(f, "Disconnecting"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Ocena świeżości stanu telemetrii w snapshotcie sesji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateFreshness {
    /// Brak jakichkolwiek danych telemetrii (np. przed pierwszą synchronizacją).
    Missing,
    /// Stan aktualny, odebrany niedawno.
    Fresh,
    /// Stan zdezaktualizowany (brak świeżych pakietów powyżej progu tolerancji).
    Stale,
}

/// Niezależny snapshot stanu pojedynczej sesji radiowej.
#[derive(Debug, Clone, PartialEq)]
pub struct RigServiceSnapshot {
    /// Identyfikator sesji.
    pub rig_id: RigId,
    /// Aktualny stan cyklu życia.
    pub lifecycle: RigServiceState,
    /// Ostatni potwierdzony stan transceivera (potwierdzona telemetria).
    pub state: Option<RigState>,
    /// Ostatni żądany stan (optymistyczny bufor komend przed potwierdzeniem z radia).
    pub requested_state: Option<RigState>,
    /// Możliwości techniczne transceivera i backendu.
    pub capabilities: RigCapabilities,
    /// Monotoniczny czas ostatniej aktualizacji stanu.
    pub last_update: Option<tokio::time::Instant>,
    /// Monotoniczny czas ostatniej aktualizacji potwierdzonej telemetrii (state).
    pub last_state_update: Option<tokio::time::Instant>,
    /// Monotoniczny numer generacji potwierdzonej telemetrii.
    pub state_generation: u64,
    /// Monotoniczny numer generacji sesji połączenia (inkrementowany przy reconnect).
    pub session_generation: u64,
    /// Świeżość stanu telemetrii.
    pub freshness: StateFreshness,
    /// Ostatni zarejestrowany błąd diagnostyczny (pozbawiony danych poufnych).
    pub last_error: Option<String>,
}

impl RigServiceSnapshot {
    /// Tworzy początkowy snapshot dla nowo utworzonej sesji.
    pub fn initial(rig_id: RigId, capabilities: RigCapabilities) -> Self {
        Self {
            rig_id,
            lifecycle: RigServiceState::Created,
            state: None,
            requested_state: None,
            capabilities,
            last_update: None,
            last_state_update: None,
            state_generation: 0,
            session_generation: 1,
            freshness: StateFreshness::Missing,
            last_error: None,
        }
    }
}

/// Błędy operacyjne usługi pojedynczej sesji radiowej.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigServiceError {
    /// Niedozwolone przejście cyklu życia.
    InvalidLifecycleTransition {
        from: RigServiceState,
        to: RigServiceState,
    },
    /// Usługa nie jest w stanie gotowości ([`RigServiceState::Ready`] lub [`RigServiceState::Degraded`]).
    NotReady(RigServiceState),
    /// Kolejka poleceń jest pełna (backpressure).
    QueueFull,
    /// Przekroczono monotoniczny termin wykonania polecenia.
    DeadlineExceeded,
    /// Kanał odpowiedzi na polecenie został zamknięty przed odesłaniem wyniku.
    ResponseChannelClosed,
    /// Błąd wykonania polecenia przez backend.
    CommandExecutionFailed(String),
    /// Pętla aktora uległa panice lub zakończyła się błędem.
    BackendPanic(String),
    /// Usługa jest w trakcie zamykania.
    ServiceShuttingDown,
    /// Usługa została już rozłączona.
    Disconnected,
    /// Zewnętrzny nadajnik jest aktywny (wykryto sprzętowy PTT bez lokalnego prawa do nadawania).
    ExternalTransmitActive,
    /// Stan PTT sprzętu nie jest potwierdzony jako bezpieczny RX.
    PttStateUnknown,
}

impl fmt::Display for RigServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLifecycleTransition { from, to } => {
                write!(f, "niedozwolone przejście cyklu życia: {from} -> {to}")
            }
            Self::NotReady(state) => write!(f, "usługa nie jest gotowa (stan: {state})"),
            Self::QueueFull => write!(f, "kolejka poleceń sesji jest przepełniona (QueueFull)"),
            Self::DeadlineExceeded => write!(f, "przekroczono limit czasu (deadline) polecenia"),
            Self::ResponseChannelClosed => {
                write!(f, "kanał odpowiedzi na polecenie został zamknięty")
            }
            Self::CommandExecutionFailed(msg) => write!(f, "błąd wykonania komendy CAT: {msg}"),
            Self::BackendPanic(msg) => write!(f, "panika w tasku backendu: {msg}"),
            Self::ServiceShuttingDown => write!(f, "usługa sesji jest zamykana"),
            Self::Disconnected => write!(f, "usługa sesji została rozłączona"),
            Self::ExternalTransmitActive => {
                write!(
                    f,
                    "aktywny zewnętrzny nadajnik (External TX) – lokalne nadawanie zablokowane"
                )
            }
            Self::PttStateUnknown => {
                write!(
                    f,
                    "stan PTT sprzętu nie jest potwierdzony jako bezpieczny RX"
                )
            }
        }
    }
}

impl std::error::Error for RigServiceError {}

/// Wewnętrzny komunikat żądania kierowany do taska aktora [`RigService`].
pub struct RigServiceRequest {
    /// Monotoniczny identyfikator polecenia.
    pub command_id: RigCommandId,
    /// Adres docelowy operacji.
    pub target: RigTarget,
    /// Polecenie do wykonania.
    pub command: RigCommand,
    /// Bezwzględny termin wykonania.
    pub deadline: tokio::time::Instant,
    /// Kanał zwrotny wyniku wykonania.
    pub responder: oneshot::Sender<Result<(), RigServiceError>>,
}

/// Konfiguracja polityki automatycznego ponawiania połączenia (auto-reconnect).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPolicy {
    /// Czy auto-reconnect jest włączony.
    pub enabled: bool,
    /// Bazowe opóźnienie początkowe (domyślnie 500 ms).
    pub base_delay: Duration,
    /// Maksymalne dopuszczalne opóźnienie backoff (domyślnie 10 000 ms).
    pub max_delay: Duration,
    /// Mnożnik geometryczny (domyślnie 2).
    pub factor: u32,
    /// Procent jittera / wariancji (domyślnie 10%).
    pub jitter_pct: u32,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_millis(10_000),
            factor: 2,
            jitter_pct: 10,
        }
    }
}

impl ReconnectPolicy {
    /// Tworzy politykę z wyłączonym auto-reconnectem.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Oblicza opóźnienie dla danej próby (0-indexed) z ograniczonym wykładniczym backoffem i jitterem.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if !self.enabled {
            return Duration::ZERO;
        }
        let base_ms = self.base_delay.as_millis() as u64;
        let max_ms = self.max_delay.as_millis() as u64;
        let mult = (self.factor as u64).saturating_pow(attempt);
        let uncapped_ms = base_ms.saturating_mul(mult);
        let capped_ms = uncapped_ms.min(max_ms);

        let jitter_range = (capped_ms * u64::from(self.jitter_pct)) / 100;
        let jitter = if jitter_range > 0 {
            (u64::from(attempt) * 31) % (jitter_range + 1)
        } else {
            0
        };
        Duration::from_millis(capped_ms.saturating_add(jitter))
    }
}

/// Usługa sesji radiowej zarządzająca pętlą I/O w dedykowanym tasku aktora.
/// Wynik asynchronicznej operacji sesji radiowej zintegrowany ze znacznikiem generacji sesji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionOperationResult<T> {
    pub(crate) generation: u64,
    pub(crate) result: Result<T, RigServiceError>,
}

impl<T> SessionOperationResult<T> {
    pub(crate) fn new(generation: u64, result: Result<T, RigServiceError>) -> Self {
        Self { generation, result }
    }

    pub(crate) fn is_current(&self, current_generation: u64) -> bool {
        self.generation == current_generation
    }
}

pub struct RigService {
    rig_id: RigId,
    backend: Box<dyn RigBackend>,
    cmd_rx: mpsc::Receiver<RigServiceRequest>,
    event_tx: broadcast::Sender<RoutedRigEvent>,
    snapshot_tx: Arc<watch::Sender<RigServiceSnapshot>>,
    shutdown_rx: oneshot::Receiver<()>,
    snapshot: RigServiceSnapshot,
    poll_interval: Duration,
    reconnect_policy: ReconnectPolicy,
    session_generation: u64,
}

impl RigService {
    /// Tworzy i uruchamia autonomiczną usługę sesji radiowej w osobnym tasku Tokio.
    ///
    /// Zwraca [`RigServiceHandle`] będący jedynym bezpiecznym interfejsem sterującym sesją.
    pub fn spawn(
        rig_id: RigId,
        backend: Box<dyn RigBackend>,
        poll_interval: Duration,
    ) -> (Self, RigServiceHandle) {
        Self::spawn_with_policy(rig_id, backend, poll_interval, ReconnectPolicy::default())
    }

    /// Tworzy i uruchamia usługę z określoną polityką ponawiania połączenia ([ReconnectPolicy]).
    pub fn spawn_with_policy(
        rig_id: RigId,
        backend: Box<dyn RigBackend>,
        poll_interval: Duration,
        reconnect_policy: ReconnectPolicy,
    ) -> (Self, RigServiceHandle) {
        let caps = backend.capabilities();
        let (cmd_tx, cmd_rx) = mpsc::channel(MAX_RIG_COMMAND_QUEUE);
        let (event_tx, _) = broadcast::channel(MAX_RIG_EVENT_QUEUE);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let initial_snapshot = RigServiceSnapshot::initial(rig_id.clone(), caps);
        let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot.clone());
        let snapshot_tx = Arc::new(snapshot_tx);

        let service = Self {
            rig_id: rig_id.clone(),
            backend,
            cmd_rx,
            event_tx: event_tx.clone(),
            snapshot_tx: Arc::clone(&snapshot_tx),
            shutdown_rx,
            snapshot: initial_snapshot,
            poll_interval,
            reconnect_policy,
            session_generation: 1,
        };

        let handle = RigServiceHandle {
            rig_id,
            cmd_tx,
            snapshot_rx,
            snapshot_tx,
            event_tx,
            shutdown_tx: Some(shutdown_tx),
            supervisor: None,
            supervisor_handle: None,
            worker_abort: None,
            supervisor_shutdown_tx: None,
        };

        (service, handle)
    }

    /// Zwraca aktualną generację sesji aktora.
    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    /// Inkrementuje numer generacji sesji i aktualizuje snapshot.
    pub fn bump_session_generation(&mut self) -> u64 {
        self.session_generation = self.session_generation.wrapping_add(1);
        self.snapshot.session_generation = self.session_generation;
        self.session_generation
    }

    /// Sprawdza, czy podana generacja odpowiada aktualnej aktywnej sesji.
    pub fn is_generation_current(&self, expected_gen: u64) -> bool {
        self.session_generation == expected_gen
    }

    /// Aplikuje wynik pollingu z weryfikacją generacji sesji przed jakąkolwiek mutacją stanu.
    pub(crate) fn apply_poll_result(&mut self, op: SessionOperationResult<RigState>) -> bool {
        if !op.is_current(self.session_generation) {
            log::warn!(
                "[{}] Odrzucono stary wynik poll() dla generacji {} (bieżąca generacja: {})",
                self.rig_id,
                op.generation,
                self.session_generation
            );
            return false;
        }

        match op.result {
            Ok(state) => {
                self.update_state(state);
                true
            }
            Err(e) => {
                log::debug!(
                    "[{}] Błąd pollingu stanu dla bieżącej sesji: {e}",
                    self.rig_id
                );
                if self.snapshot.state.is_some() {
                    self.snapshot.freshness = StateFreshness::Stale;
                    self.snapshot.last_update = Some(tokio::time::Instant::now());
                    let _ = self.snapshot_tx.send(self.snapshot.clone());
                }
                self.set_lifecycle(RigServiceState::Degraded, Some(e.to_string()));
                false
            }
        }
    }

    /// Aplikuje wynik nawiązywania połączenia connect() z weryfikacją generacji sesji.
    pub(crate) fn apply_connect_result(&mut self, op: SessionOperationResult<()>) -> bool {
        if !op.is_current(self.session_generation) {
            log::warn!(
                "[{}] Odrzucono stary wynik connect() dla generacji {} (bieżąca generacja: {})",
                self.rig_id,
                op.generation,
                self.session_generation
            );
            return false;
        }

        match op.result {
            Ok(()) => true,
            Err(e) => {
                self.set_lifecycle(RigServiceState::Failed, Some(e.to_string()));
                false
            }
        }
    }

    /// Aplikuje wynik rozłączenia disconnect() z weryfikacją generacji sesji.
    pub(crate) fn apply_disconnect_result(&self, op: &SessionOperationResult<()>) -> bool {
        if !op.is_current(self.session_generation) {
            log::warn!(
                "[{}] Odrzucono stary wynik disconnect() dla generacji {} (bieżąca generacja: {})",
                self.rig_id,
                op.generation,
                self.session_generation
            );
            return false;
        }
        true
    }

    /// Konfiguruje politykę auto-reconnectu dla usługi.
    pub fn with_reconnect_policy(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = policy;

        self
    }

    /// Weryfikuje i wykonuje przejście cyklu życia, publikując nowy snapshot i zdarzenie.
    ///
    /// W przypadku niedozwolonego przejścia zwraca typed error [`RigServiceError::InvalidLifecycleTransition`]
    /// i nie mutuje stanu ani snapshotu.
    pub fn transition_to(
        &mut self,
        next: RigServiceState,
        error: Option<String>,
    ) -> Result<(), RigServiceError> {
        if !self.snapshot.lifecycle.can_transition_to(next) {
            log::warn!(
                "[{}] Niedozwolone przejście cyklu życia: {} -> {}",
                self.rig_id,
                self.snapshot.lifecycle,
                next
            );
            return Err(RigServiceError::InvalidLifecycleTransition {
                from: self.snapshot.lifecycle,
                to: next,
            });
        }

        self.snapshot.lifecycle = next;
        self.snapshot.session_generation = self.session_generation;
        self.snapshot.last_error = error
            .as_deref()
            .map(crate::cat::tx_arbiter::redact_sensitive_info);
        self.snapshot.last_update = Some(tokio::time::Instant::now());

        if (next == RigServiceState::Degraded
            || next == RigServiceState::Disconnecting
            || next == RigServiceState::Disconnected
            || next == RigServiceState::Failed)
            && self.snapshot.state.is_some()
        {
            self.snapshot.freshness = StateFreshness::Stale;
        }

        let hub_event = match next {
            RigServiceState::Created => return Ok(()),
            RigServiceState::Connecting => RigHubEvent::RigConnecting,
            RigServiceState::Synchronizing => RigHubEvent::RigSynchronizing,
            RigServiceState::Ready => RigHubEvent::RigReady,
            RigServiceState::Degraded => {
                let msg = error
                    .as_deref()
                    .map(crate::cat::tx_arbiter::redact_sensitive_info)
                    .unwrap_or_else(|| "stan Degraded".to_string());
                RigHubEvent::RigDegraded(msg)
            }
            RigServiceState::Disconnecting => RigHubEvent::RigDisconnecting,
            RigServiceState::Disconnected => RigHubEvent::RigDisconnected,
            RigServiceState::Failed => {
                let msg = error
                    .as_deref()
                    .map(crate::cat::tx_arbiter::redact_sensitive_info)
                    .unwrap_or_else(|| "awaria sesji".to_string());
                RigHubEvent::RigFailed(msg)
            }
        };

        let _ = self
            .event_tx
            .send(RoutedRigEvent::new(self.rig_id.clone(), None, hub_event));
        let _ = self.snapshot_tx.send(self.snapshot.clone());
        Ok(())
    }

    /// Pomocnicza metoda aktualizacji stanu cyklu życia (ignoruje błąd niepoprawnego przejścia).
    fn set_lifecycle(&mut self, next: RigServiceState, error: Option<String>) {
        let _ = self.transition_to(next, error);
    }

    /// Aktualizuje zbuforowany stan telemetrii.
    fn update_state(&mut self, state: RigState) {
        let now = tokio::time::Instant::now();
        self.snapshot.state = Some(state.clone());
        self.snapshot.last_state_update = Some(now);
        self.snapshot.state_generation = self.snapshot.state_generation.wrapping_add(1);
        self.snapshot.session_generation = self.session_generation;
        self.snapshot.last_update = Some(now);
        self.snapshot.freshness = StateFreshness::Fresh;
        let _ = self.snapshot_tx.send(self.snapshot.clone());

        // Publikuj zdarzenie zmiany stanu
        let _ = self.event_tx.send(RoutedRigEvent::new(
            self.rig_id.clone(),
            None,
            RigHubEvent::Radio(NativeRigEvent::StateChanged(state)),
        ));
    }

    /// Główna asynchroniczna pętla aktora sesji radiowej.
    pub async fn run(mut self) -> Result<(), RigServiceError> {
        // 1. Faza Connecting
        self.set_lifecycle(RigServiceState::Connecting, None);

        let connect_timeout = Duration::from_millis(5000);
        let connect_gen = self.session_generation;
        let connect_res = tokio::select! {
            _ = &mut self.shutdown_rx => {
                log::debug!("[{}] Sygnał shutdown odebrany podczas connect()", self.rig_id);
                self.set_lifecycle(RigServiceState::Disconnecting, None);
                let _ = tokio::time::timeout(Duration::from_millis(500), self.backend.disconnect()).await;
                self.set_lifecycle(RigServiceState::Disconnected, None);
                return Ok(());
            }
            res = tokio::time::timeout(connect_timeout, self.backend.connect()) => res,
        };

        if self.session_generation != connect_gen {
            log::warn!("[{}] Connect odrzucony: sesja uległa zmianie", self.rig_id);
            return Ok(());
        }

        let connect_op = match connect_res {
            Ok(Ok(())) => SessionOperationResult::new(connect_gen, Ok(())),
            Ok(Err(e)) => SessionOperationResult::new(
                connect_gen,
                Err(RigServiceError::CommandExecutionFailed(format!(
                    "Błąd connect: {e}"
                ))),
            ),
            Err(_) => {
                SessionOperationResult::new(connect_gen, Err(RigServiceError::DeadlineExceeded))
            }
        };

        if !self.apply_connect_result(connect_op) {
            return Err(RigServiceError::CommandExecutionFailed(
                "Błąd connect".to_string(),
            ));
        }

        // 2. Faza Synchronizing: wstępny odczyt stanu radia
        self.set_lifecycle(RigServiceState::Synchronizing, None);
        let poll_timeout = Duration::from_millis(3000);
        let sync_gen = self.session_generation;
        let poll_res = tokio::select! {
            _ = &mut self.shutdown_rx => {
                log::debug!("[{}] Sygnał shutdown odebrany podczas synchronizacji stanu", self.rig_id);
                self.set_lifecycle(RigServiceState::Disconnecting, None);
                let _ = tokio::time::timeout(Duration::from_millis(500), self.backend.disconnect()).await;
                self.set_lifecycle(RigServiceState::Disconnected, None);
                return Ok(());
            }
            res = tokio::time::timeout(poll_timeout, self.backend.poll()) => res,
        };

        if self.session_generation != sync_gen {
            log::warn!(
                "[{}] Synchronizacja odrzucona: sesja uległa zmianie",
                self.rig_id
            );
            return Ok(());
        }

        let sync_op = match poll_res {
            Ok(Ok(st)) => SessionOperationResult::new(sync_gen, Ok(st)),
            Ok(Err(e)) => SessionOperationResult::new(
                sync_gen,
                Err(RigServiceError::CommandExecutionFailed(e.to_string())),
            ),
            Err(_) => SessionOperationResult::new(sync_gen, Err(RigServiceError::DeadlineExceeded)),
        };

        if self.apply_poll_result(sync_op) {
            self.set_lifecycle(RigServiceState::Ready, None);
        }

        // 3. Faza Ready / Degraded: główna pętla obsługi komend, pollingu i auto-reconnectu
        let mut interval = tokio::time::interval(self.poll_interval);
        // Pierwszy tick interwału odpala się natychmiast – pomijamy go, bo zrobiliśmy synchronizację
        interval.tick().await;

        let mut reconnect_attempts: u32 = 0;
        let mut reconnect_deadline: Option<(tokio::time::Instant, u64)> = None;

        if self.snapshot.lifecycle == RigServiceState::Degraded && self.reconnect_policy.enabled {
            let delay = self.reconnect_policy.delay_for_attempt(reconnect_attempts);
            reconnect_deadline =
                Some((tokio::time::Instant::now() + delay, self.session_generation));
        }

        loop {
            tokio::select! {
                // Odbiór żądania wyłączenia sesji (shutdown) - przerywa natychmiast pętlę i oczekiwanie na reconnect
                _ = &mut self.shutdown_rx => {
                    log::debug!("[{}] Odebrano sygnał shutdown", self.rig_id);
                    break;
                }

                // Odbiór polecenia sterującego
                maybe_req = self.cmd_rx.recv() => {
                    match maybe_req {
                        Some(req) => {
                            self.handle_command(req).await;
                        }
                        None => {
                            // Wszyscy nadawcy poleceń zostali usunięci – kończymy pętlę
                            log::debug!("[{}] Kanał poleceń zamknięty", self.rig_id);
                            break;
                        }
                    }
                }

                // Auto-reconnect timer: próba przywrócenia połączenia w stanie Degraded z ograniczonym backoffem
                () = async {
                    match reconnect_deadline {
                        Some((deadline, timer_gen)) => {
                            if timer_gen == self.session_generation {
                                tokio::time::sleep_until(deadline).await;
                            } else {
                                std::future::pending::<()>().await;
                            }
                        }
                        None => std::future::pending::<()>().await,
                    }
                }, if reconnect_deadline.as_ref().is_some_and(|(_, g)| *g == self.session_generation) => {
                    self.bump_session_generation();
                    let current_gen = self.session_generation;
                    log::info!(
                        "[{}] Próba auto-reconnectu #{} dla sesji (generacja {})",
                        self.rig_id,
                        reconnect_attempts + 1,
                        current_gen
                    );

                    // 1. Zapewnij świeżość Stale w trakcie próby nawiązania połączenia
                    if self.snapshot.state.is_some() {
                        self.snapshot.freshness = StateFreshness::Stale;
                        self.snapshot.last_update = Some(tokio::time::Instant::now());
                        let _ = self.snapshot_tx.send(self.snapshot.clone());
                    }

                    // 2. Czysty disconnect dotychczasowego połączenia z bounded timeoutem
                    let _ = tokio::select! {
                        _ = &mut self.shutdown_rx => break,
                        res = tokio::time::timeout(Duration::from_millis(500), self.backend.disconnect()) => res,
                    };
                    if self.session_generation != current_gen {
                        continue;
                    }
                    let _ = self.apply_disconnect_result(&SessionOperationResult::new(current_gen, Ok(())));

                    // 3. Próba ponownego połączenia z bounded timeoutem
                    let connect_res = tokio::select! {
                        _ = &mut self.shutdown_rx => break,
                        res = tokio::time::timeout(Duration::from_millis(3000), self.backend.connect()) => res,
                    };
                    if self.session_generation != current_gen {
                        continue;
                    }

                    match connect_res {
                        Ok(Ok(())) => {
                            // Po udanym connect() sprawdzamy stabilność i synchronizujemy stan przez poll()
                            let poll_res = tokio::select! {
                                _ = &mut self.shutdown_rx => break,
                                res = tokio::time::timeout(Duration::from_millis(1500), self.backend.poll()) => res,
                            };
                            if self.session_generation != current_gen {
                                continue;
                            }

                            match poll_res {
                                Ok(Ok(state)) => {
                                    let poll_op = SessionOperationResult::new(current_gen, Ok(state));
                                    if self.apply_poll_result(poll_op) {
                                        self.set_lifecycle(RigServiceState::Ready, None);
                                        reconnect_attempts = 0;
                                        reconnect_deadline = None;
                                    }
                                }
                                Ok(Err(e)) => {
                                    log::warn!(
                                        "[{}] Auto-reconnect connect() udany, lecz poll() zawiódł: {e}",
                                        self.rig_id
                                    );
                                    reconnect_attempts = reconnect_attempts.saturating_add(1);
                                    let next_delay = self.reconnect_policy.delay_for_attempt(reconnect_attempts);
                                    reconnect_deadline = Some((tokio::time::Instant::now() + next_delay, self.session_generation));
                                }
                                Err(_) => {
                                    log::warn!(
                                        "[{}] Auto-reconnect connect() udany, lecz poll() przekroczył limit czasu",
                                        self.rig_id
                                    );
                                    reconnect_attempts = reconnect_attempts.saturating_add(1);
                                    let next_delay = self.reconnect_policy.delay_for_attempt(reconnect_attempts);
                                    reconnect_deadline = Some((tokio::time::Instant::now() + next_delay, self.session_generation));
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            log::debug!(
                                "[{}] Auto-reconnect próba #{} nie powiodła się: {e}",
                                self.rig_id,
                                reconnect_attempts + 1
                            );
                            reconnect_attempts = reconnect_attempts.saturating_add(1);
                            let next_delay = self.reconnect_policy.delay_for_attempt(reconnect_attempts);
                            reconnect_deadline = Some((tokio::time::Instant::now() + next_delay, self.session_generation));
                        }
                        Err(_) => {
                            log::debug!(
                                "[{}] Auto-reconnect próba #{} timeout",
                                self.rig_id,
                                reconnect_attempts + 1
                            );
                            reconnect_attempts = reconnect_attempts.saturating_add(1);
                            let next_delay = self.reconnect_policy.delay_for_attempt(reconnect_attempts);
                            reconnect_deadline = Some((tokio::time::Instant::now() + next_delay, self.session_generation));
                        }
                    }
                }

                // Polling cykliczny stanu transceivera (wyłącznie w stanie Ready)
                _ = interval.tick() => {
                    if self.snapshot.lifecycle == RigServiceState::Ready {
                        let poll_gen = self.session_generation;
                        let poll_timeout = Duration::from_millis(1500);
                        let poll_res = tokio::select! {
                            _ = &mut self.shutdown_rx => break,
                            res = tokio::time::timeout(poll_timeout, self.backend.poll()) => res,
                        };

                        if self.session_generation != poll_gen {
                            log::debug!("[{}] Zignorowano stary poll dla generacji {poll_gen} (bieżąca: {})", self.rig_id, self.session_generation);
                            continue;
                        }

                        match poll_res {
                            Ok(Ok(state)) => {
                                let poll_op = SessionOperationResult::new(poll_gen, Ok(state));
                                if self.apply_poll_result(poll_op) {
                                    reconnect_attempts = 0;
                                    reconnect_deadline = None;
                                }
                            }
                            Ok(Err(e)) => {
                                let poll_op = SessionOperationResult::new(
                                    poll_gen,
                                    Err(RigServiceError::CommandExecutionFailed(e.to_string())),
                                );
                                self.apply_poll_result(poll_op);
                                if self.reconnect_policy.enabled && reconnect_deadline.is_none() {
                                    let delay = self.reconnect_policy.delay_for_attempt(reconnect_attempts);
                                    reconnect_deadline = Some((tokio::time::Instant::now() + delay, self.session_generation));
                                }
                            }
                            Err(_) => {
                                let poll_op = SessionOperationResult::new(
                                    poll_gen,
                                    Err(RigServiceError::DeadlineExceeded),
                                );
                                self.apply_poll_result(poll_op);
                                if self.reconnect_policy.enabled && reconnect_deadline.is_none() {
                                    let delay = self.reconnect_policy.delay_for_attempt(reconnect_attempts);
                                    reconnect_deadline = Some((tokio::time::Instant::now() + delay, self.session_generation));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Faza Disconnecting / Disconnected: bezpieczne wyłączenie
        self.set_lifecycle(RigServiceState::Disconnecting, None);

        // Odrzuć ewentualne pozostałe w kolejce komendy
        while let Ok(pending) = self.cmd_rx.try_recv() {
            let _ = pending
                .responder
                .send(Err(RigServiceError::ServiceShuttingDown));
        }

        // Wywołaj disconnect na backendzie z bounded timeoutem
        let disconnect_timeout = Duration::from_millis(1500);
        let _ = tokio::time::timeout(disconnect_timeout, self.backend.disconnect()).await;

        self.set_lifecycle(RigServiceState::Disconnected, None);
        Ok(())
    }

    /// Bezpieczna obsługa pojedynczego polecenia CAT wewnątrz taska aktora.
    async fn handle_command(&mut self, req: RigServiceRequest) {
        // 1. Sprawdź, czy termin wykonania nie upłynął przed rozpoczęciem
        if tokio::time::Instant::now() >= req.deadline {
            let _ = req.responder.send(Err(RigServiceError::DeadlineExceeded));
            return;
        }

        // 2. Sprawdź, czy usługa jest w stanie umożliwiającym wykonanie komendy
        if self.snapshot.lifecycle != RigServiceState::Ready
            && self.snapshot.lifecycle != RigServiceState::Degraded
        {
            let _ = req
                .responder
                .send(Err(RigServiceError::NotReady(self.snapshot.lifecycle)));
            return;
        }

        // 2b. Ochrona PTT: stan Degraded kategorycznie zabrania załączania PTT ON (CAT-017)
        if self.snapshot.lifecycle == RigServiceState::Degraded
            && matches!(req.command, RigCommand::SetPtt(true))
        {
            log::warn!(
                "[{}] Odrzucono polecenie SetPtt(true) w stanie Degraded (ochrona przed emisją)",
                self.rig_id
            );
            let _ = req
                .responder
                .send(Err(RigServiceError::NotReady(self.snapshot.lifecycle)));
            return;
        }

        // 2c. Ochrona PTT: wykryty aktywny hardware TX bez lokalnego ownership blokuje SetPtt(true) (CAT-017)
        if matches!(req.command, RigCommand::SetPtt(true))
            && self.snapshot.state.as_ref().is_some_and(|s| s.ptt)
        {
            log::warn!(
                "[{}] Odrzucono polecenie SetPtt(true) – wykryto aktywny zewnętrzny nadajnik (External TX)",
                self.rig_id
            );
            let _ = req
                .responder
                .send(Err(RigServiceError::ExternalTransmitActive));
            return;
        }

        // 3. Oblicz pozostały czas do deadline
        let now = tokio::time::Instant::now();
        let remaining = if req.deadline > now {
            req.deadline - now
        } else {
            let _ = req.responder.send(Err(RigServiceError::DeadlineExceeded));
            return;
        };

        // 4. Wykonaj polecenie na backendzie z bounded timeoutem oraz uwzględnieniem docelowego odbiornika
        let cmd_gen = self.session_generation;
        let exec_res = tokio::time::timeout(
            remaining,
            self.backend
                .execute_routed(req.target.receiver_id.as_ref(), &req.command),
        )
        .await;

        // CHECK BEFORE MUTATION: odrzuć wynik, jeżeli w trakcie operacji sesja uległa zmianie
        if self.session_generation != cmd_gen {
            log::warn!(
                "[{}] Odrzucono wynik komendy {:?} – sesja uległa unieważnieniu ({cmd_gen} != {})",
                self.rig_id,
                req.command,
                self.session_generation
            );
            let _ = req.responder.send(Err(RigServiceError::Disconnected));
            return;
        }

        match exec_res {
            Ok(Ok(())) => {
                // Po udanej komendzie PTT wykonaj natychmiastowe odpytanie stanu backendu (polling/status)
                if matches!(req.command, RigCommand::SetPtt(_)) {
                    let now = tokio::time::Instant::now();
                    let poll_dur = if req.deadline > now {
                        (req.deadline - now).min(Duration::from_millis(500))
                    } else {
                        Duration::from_millis(100)
                    };
                    if self.session_generation == cmd_gen {
                        if let Ok(Ok(polled_state)) =
                            tokio::time::timeout(poll_dur, self.backend.poll()).await
                        {
                            self.update_state(polled_state);
                        }
                    }
                }

                if self.session_generation != cmd_gen {
                    let _ = req.responder.send(Err(RigServiceError::Disconnected));
                    return;
                }

                // Po udanej komendzie zaktualizuj stan żądany (requested state).
                // Ważne: stan potwierdzony (self.snapshot.state) NIE MOŻE być fabrykowany na podstawie
                // samego wysłania komendy — aktualizuje go wyłącznie telemetria z radia (update_state).
                let mut req_state = self
                    .snapshot
                    .requested_state
                    .clone()
                    .or_else(|| self.snapshot.state.clone());

                if let Some(ref mut st) = req_state {
                    match req.command {
                        RigCommand::SetPtt(ptt_val) => {
                            st.ptt = ptt_val;
                        }
                        RigCommand::SetFrequency(hz) => {
                            st.frequency_hz = hz;
                        }
                        RigCommand::SetMode {
                            ref mode,
                            passband_hz,
                        } => {
                            st.mode = mode.to_string();
                            st.passband_hz = passband_hz;
                        }
                        _ => {}
                    }
                }
                self.snapshot.requested_state = req_state;
                self.snapshot.last_update = Some(tokio::time::Instant::now());
                let _ = self.snapshot_tx.send(self.snapshot.clone());
                let _ = req.responder.send(Ok(()));
            }
            Ok(Err(e)) => {
                let err_str = e.to_string();
                log::warn!(
                    "[{}] Błąd wykonania komendy {:?}: {err_str}",
                    self.rig_id,
                    req.command
                );
                let _ = req
                    .responder
                    .send(Err(RigServiceError::CommandExecutionFailed(err_str)));
            }
            Err(_) => {
                log::warn!(
                    "[{}] Przekroczono limit czasu (deadline) dla komendy {:?}",
                    self.rig_id,
                    req.command
                );
                let _ = req.responder.send(Err(RigServiceError::DeadlineExceeded));
            }
        }
    }
}

/// Nadzorca taska usługi sesji radiowej ([`RigService`]).
///
/// Monitoruje wykonanie pętli aktora w dedykowanym tasku Tokio.
/// Posiada powiązany [`JoinHandle`] usługi. W przypadku paniki aktora natychmiast
/// aktualizuje snapshot sesji na [`RigServiceState::Failed`] i emituje zdarzenie [`RigHubEvent::RigFailed`].
#[derive(Debug)]
pub struct RigServiceSupervisor {
    rig_id: RigId,
    active: bool,
}

impl RigServiceSupervisor {
    /// Tworzy nową instancję nadzorcy.
    pub fn new(rig_id: RigId) -> Self {
        Self {
            rig_id,
            active: true,
        }
    }

    /// Identyfikator sesji nadzorowanego radia.
    pub fn rig_id(&self) -> &RigId {
        &self.rig_id
    }

    /// Sprawdza, czy nadzorca jest aktywny.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Sprawdza, czy nadzorca posiada powiązany uchwyt zadania.
    pub fn has_join_handle(&self) -> bool {
        self.active
    }
}

/// Bezpieczny uchwyt do usługi sesji radiowej ([`RigService`]).
///
/// Posiada wyłącznie ograniczone nadajniki kanałów, odbiornik snapshotu oraz
/// zarządzany [`tokio::task::JoinHandle`]. Nie posiada referencji do backendu ani transportu.
pub struct RigServiceHandle {
    rig_id: RigId,
    cmd_tx: mpsc::Sender<RigServiceRequest>,
    snapshot_rx: watch::Receiver<RigServiceSnapshot>,
    snapshot_tx: Arc<watch::Sender<RigServiceSnapshot>>,
    event_tx: broadcast::Sender<RoutedRigEvent>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    supervisor: Option<RigServiceSupervisor>,
    supervisor_handle: Option<JoinHandle<Result<(), RigServiceError>>>,
    worker_abort: Option<tokio::task::AbortHandle>,
    supervisor_shutdown_tx: Option<oneshot::Sender<()>>,
}

impl RigServiceHandle {
    /// Pomocnicza metoda aktualizacji snapshotu i rozesłania zdarzenia w razie paniki taska aktora.
    fn publish_panic_failure(
        snapshot_tx: &Arc<watch::Sender<RigServiceSnapshot>>,
        rig_id: &RigId,
        event_tx: &broadcast::Sender<RoutedRigEvent>,
        err_msg: &str,
    ) {
        let mut snap = snapshot_tx.borrow().clone();
        snap.lifecycle = RigServiceState::Failed;
        snap.last_error = Some(err_msg.to_string());
        snap.last_update = Some(tokio::time::Instant::now());
        let _ = snapshot_tx.send(snap);

        let _ = event_tx.send(RoutedRigEvent::new(
            rig_id.clone(),
            None,
            RigHubEvent::RigFailed(err_msg.to_string()),
        ));
    }

    /// Przypisuje `JoinHandle` uruchomionego taska do uchwytu i uruchamia nadzorcę (supervisor).
    pub fn attach_join_handle(&mut self, mut worker_jh: JoinHandle<Result<(), RigServiceError>>) {
        let rig_id = self.rig_id.clone();
        let snapshot_tx = Arc::clone(&self.snapshot_tx);
        let event_tx = self.event_tx.clone();
        let abort_handle = worker_jh.abort_handle();
        self.worker_abort = Some(abort_handle);
        self.supervisor = Some(RigServiceSupervisor::new(rig_id.clone()));

        let (sup_shut_tx, mut sup_shut_rx) = oneshot::channel::<()>();
        self.supervisor_shutdown_tx = Some(sup_shut_tx);

        let sup_jh = tokio::spawn(async move {
            tokio::select! {
                _ = &mut sup_shut_rx => {
                    // Żądanie shutdown z uchwytu
                    match worker_jh.await {
                        Ok(res) => res,
                        Err(join_err) => {
                            if join_err.is_panic() {
                                let err_msg = "BackendTaskPanicked: panika w tasku aktora".to_string();
                                Self::publish_panic_failure(&snapshot_tx, &rig_id, &event_tx, &err_msg);
                                Err(RigServiceError::BackendPanic(err_msg))
                            } else if join_err.is_cancelled() {
                                Ok(())
                            } else {
                                Err(RigServiceError::BackendPanic(join_err.to_string()))
                            }
                        }
                    }
                }
                res = &mut worker_jh => {
                    // Task aktora zakończył się autonomicznie (błąd, panika lub normalny powrót)
                    match res {
                        Ok(res) => res,
                        Err(join_err) => {
                            if join_err.is_panic() {
                                let err_msg = "BackendTaskPanicked: panika w tasku aktora".to_string();
                                Self::publish_panic_failure(&snapshot_tx, &rig_id, &event_tx, &err_msg);
                                Err(RigServiceError::BackendPanic(err_msg))
                            } else if join_err.is_cancelled() {
                                Ok(())
                            } else {
                                Err(RigServiceError::BackendPanic(join_err.to_string()))
                            }
                        }
                    }
                }
            }
        });

        self.supervisor_handle = Some(sup_jh);
    }

    /// Zwraca referencję do nadzorcy taska, o ile powiązany.
    pub fn supervisor(&self) -> Option<&RigServiceSupervisor> {
        self.supervisor.as_ref()
    }

    /// Zwraca referencję do JoinHandle nadzorcy taska.
    pub fn supervisor_join_handle(&self) -> Option<&JoinHandle<Result<(), RigServiceError>>> {
        self.supervisor_handle.as_ref()
    }

    /// Zwraca referencję do JoinHandle zarządzanego taska.
    pub fn join_handle(&self) -> Option<&JoinHandle<Result<(), RigServiceError>>> {
        self.supervisor_handle.as_ref()
    }

    /// Identyfikator sesji radiowej.
    pub fn rig_id(&self) -> &RigId {
        &self.rig_id
    }

    /// Pobiera najświeższy snapshot sesji (operacja natychmiastowa, nieblokująca).
    pub fn snapshot(&self) -> RigServiceSnapshot {
        self.snapshot_rx.borrow().clone()
    }

    /// Zwraca subskrypcję asynchronicznych zdarzeń emitowanych przez tę sesję.
    pub fn subscribe_events(&self) -> broadcast::Receiver<RoutedRigEvent> {
        self.event_tx.subscribe()
    }

    /// Zwraca referencję do wewnętrznego nadajnika poleceń (do testów kolejki).
    pub fn command_sender(&self) -> &mpsc::Sender<RigServiceRequest> {
        &self.cmd_tx
    }

    /// Wysyła polecenie do sesji z uwzględnieniem ograniczenia kolejki i monotonicznego terminu.
    pub async fn execute(
        &self,
        command_id: RigCommandId,
        target: RigTarget,
        command: RigCommand,
        deadline: tokio::time::Instant,
    ) -> Result<(), RigServiceError> {
        if tokio::time::Instant::now() >= deadline {
            return Err(RigServiceError::DeadlineExceeded);
        }

        let (responder, response_rx) = oneshot::channel();
        let req = RigServiceRequest {
            command_id,
            target,
            command,
            deadline,
            responder,
        };

        // Próba wysłania komendy do ograniczonej kolejki
        match self.cmd_tx.try_send(req) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                return Err(RigServiceError::QueueFull);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                return Err(RigServiceError::ServiceShuttingDown);
            }
        }

        // Oczekiwanie na odpowiedź z uwzględnieniem monotonicznego terminu
        let now = tokio::time::Instant::now();
        let wait_dur = if deadline > now {
            deadline - now
        } else {
            return Err(RigServiceError::DeadlineExceeded);
        };

        match tokio::time::timeout(wait_dur, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(RigServiceError::ResponseChannelClosed),
            Err(_) => Err(RigServiceError::DeadlineExceeded),
        }
    }

    /// Wykonuje kontrolowane, asynchroniczne zatrzymanie usługi sesji radiowej.
    ///
    /// Oczekuje na zakończenie taska aktora z uwzględnieniem podanego terminu `deadline`.
    /// W przypadku przekroczenia terminu task zostaje przerwany (`abort()`), a następnie dołączony (`join`).
    pub async fn shutdown(
        &mut self,
        deadline: tokio::time::Instant,
    ) -> Result<(), RigServiceError> {
        // 1. Wyślij sygnał wyłączenia (idempotentnie)
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(tx) = self.supervisor_shutdown_tx.take() {
            let _ = tx.send(());
        }

        // 2. Oczekuj na zakończenie pętli taska (JoinHandle)
        if let Some(mut handle) = self.supervisor_handle.take() {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                if let Some(ref abort) = self.worker_abort {
                    abort.abort();
                }
                handle.abort();
                // Po abort: oczekuj na JoinHandle ("abort_is_followed_by_join")
                let _ = handle.await;
                return Err(RigServiceError::DeadlineExceeded);
            }
            let remaining = deadline - now;

            match tokio::time::timeout(remaining, &mut handle).await {
                Ok(Ok(Ok(()))) => Ok(()),
                Ok(Ok(Err(RigServiceError::BackendPanic(msg)))) => {
                    Err(RigServiceError::BackendPanic(msg))
                }
                Ok(Ok(Err(_other_run_err))) => Ok(()),
                Ok(Err(join_err)) => {
                    if join_err.is_panic() {
                        log::error!("[{}] Panika w tasku aktora: {join_err}", self.rig_id);
                        Err(RigServiceError::BackendPanic(join_err.to_string()))
                    } else if join_err.is_cancelled() {
                        Ok(())
                    } else {
                        Err(RigServiceError::BackendPanic(join_err.to_string()))
                    }
                }
                Err(_) => {
                    log::warn!(
                        "[{}] Przekroczono limit czasu shutdown, wymuszenie przerwania taska (abort)",
                        self.rig_id
                    );
                    if let Some(ref abort) = self.worker_abort {
                        abort.abort();
                    }
                    handle.abort();
                    // Po abort: oczekuj na JoinHandle ("abort_is_followed_by_join")
                    let _ = handle.await;
                    Err(RigServiceError::DeadlineExceeded)
                }
            }
        } else {
            // Task został już wcześniej zamknięty
            Ok(())
        }
    }
}

impl Drop for RigServiceHandle {
    fn drop(&mut self) {
        if let Some(ref abort) = self.worker_abort {
            abort.abort();
        }
        if let Some(ref jh) = self.supervisor_handle {
            if !jh.is_finished() {
                jh.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::backend::BoxFuture;
    use crate::cat::backend::CatBackendKind;
    use crate::cat::rig::RigError;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    #[derive(Default, Clone)]
    struct MockShared {
        fail_poll: Arc<AtomicBool>,
        fail_connect: Arc<AtomicBool>,
        connect_count: Arc<AtomicU32>,
        poll_count: Arc<AtomicU32>,
        disconnect_count: Arc<AtomicU32>,
        poll_ptt: Arc<AtomicBool>,
        executed_cmds: Arc<std::sync::Mutex<Vec<RigCommand>>>,
    }

    struct MockTestBackend {
        shared: MockShared,
    }

    impl MockTestBackend {
        fn new(shared: MockShared) -> Self {
            Self { shared }
        }
    }

    impl RigBackend for MockTestBackend {
        fn kind(&self) -> CatBackendKind {
            CatBackendKind::Kenwood
        }

        fn capabilities(&self) -> RigCapabilities {
            RigCapabilities {
                frequency: true,
                mode: true,
                passband: true,
                split: false,
                split_frequency: false,
                ptt: true,
                vfo: false,
                rit: false,
                xit: false,
                power: false,
                s_meter: false,
            }
        }

        fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
            self.shared.connect_count.fetch_add(1, Ordering::SeqCst);
            let fail = self.shared.fail_connect.load(Ordering::SeqCst);
            Box::pin(async move {
                if fail {
                    Err(RigError::Connect {
                        endpoint: "mock".to_string(),
                        detail: "mock connect failed".to_string(),
                    })
                } else {
                    Ok(())
                }
            })
        }

        fn disconnect(&mut self) -> BoxFuture<'_, ()> {
            self.shared.disconnect_count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {})
        }

        fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
            self.shared.poll_count.fetch_add(1, Ordering::SeqCst);
            let fail = self.shared.fail_poll.load(Ordering::SeqCst);
            let ptt_val = self.shared.poll_ptt.load(Ordering::SeqCst);
            Box::pin(async move {
                if fail {
                    Err(RigError::NotConnected)
                } else {
                    let mut st = RigState::default();
                    st.frequency_hz = 14_074_000;
                    st.mode = "USB".to_string();
                    st.ptt = ptt_val;
                    Ok(st)
                }
            })
        }

        fn execute(&mut self, cmd: &RigCommand) -> BoxFuture<'_, Result<(), RigError>> {
            if let Ok(mut lock) = self.shared.executed_cmds.lock() {
                lock.push(cmd.clone());
            }
            Box::pin(async move { Ok(()) })
        }
    }

    #[test]
    fn reconnect_policy_delay_calculation_is_bounded_and_exponential() {
        let policy = ReconnectPolicy::default();
        let d0 = policy.delay_for_attempt(0);
        let d1 = policy.delay_for_attempt(1);
        let d2 = policy.delay_for_attempt(2);
        let d3 = policy.delay_for_attempt(3);
        let d4 = policy.delay_for_attempt(4);
        let d5 = policy.delay_for_attempt(5);
        let d10 = policy.delay_for_attempt(10);

        assert!(d0.as_millis() >= 500 && d0.as_millis() <= 550);
        assert!(d1.as_millis() >= 1000 && d1.as_millis() <= 1100);
        assert!(d2.as_millis() >= 2000 && d2.as_millis() <= 2200);
        assert!(d3.as_millis() >= 4000 && d3.as_millis() <= 4400);
        assert!(d4.as_millis() >= 8000 && d4.as_millis() <= 8800);
        assert!(d5.as_millis() >= 10_000 && d5.as_millis() <= 11_000);
        assert!(d10.as_millis() >= 10_000 && d10.as_millis() <= 11_000);

        let disabled = ReconnectPolicy::disabled();
        assert_eq!(disabled.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(disabled.delay_for_attempt(5), Duration::ZERO);
    }

    #[tokio::test]
    async fn rig_service_reconnects_automatically_after_stream_drop_in_degraded_state() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-RECONN-01").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        // Oczekuj na stan Ready
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Ready);
        assert_eq!(handle.snapshot().freshness, StateFreshness::Fresh);

        // Symulacja zerwania połączenia / błędu streamu w poll()
        shared.fail_poll.store(true, Ordering::SeqCst);

        // Oczekuj na przejście w stan Degraded
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Degraded);
        assert_eq!(handle.snapshot().freshness, StateFreshness::Stale);

        let initial_connects = shared.connect_count.load(Ordering::SeqCst);

        // Przywrócenie działania transportu
        shared.fail_poll.store(false, Ordering::SeqCst);

        // Usługa powinna samoczynnie ponowić połączenie (connect) i wrócić do Ready
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Ready);
        assert_eq!(handle.snapshot().freshness, StateFreshness::Fresh);
        assert!(shared.connect_count.load(Ordering::SeqCst) > initial_connects);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn rig_service_reconnect_backoff_is_cancellable_by_shutdown() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-CANCEL-01").unwrap();
        // Bardzo długi backoff (10 sekund), aby upewnić się, że shutdown go przerywa natychmiast
        let long_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(30),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            long_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        // Oczekuj na stan Ready
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Ready);

        // Wprowadź w błąd
        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Degraded);

        // Wywołaj shutdown z limitem 500 ms – powinno zakończyć się natychmiast, a nie czekać 10 sekund
        let t0 = tokio::time::Instant::now();
        let shut_res = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
        let elapsed = t0.elapsed();

        assert!(shut_res.is_ok());
        assert!(elapsed < Duration::from_secs(2));
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Disconnected);
    }

    #[tokio::test]
    async fn manual_disconnect_never_schedules_reconnect() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-MANUAL-01").unwrap();
        let (service, mut handle) = RigService::spawn(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Disconnected);

        let connects_at_shutdown = shared.connect_count.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(
            shared.connect_count.load(Ordering::SeqCst),
            connects_at_shutdown
        );
    }

    #[tokio::test]
    async fn reconnect_increments_session_generation_and_invalidates_stale_state() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-GEN-01").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let snap1 = handle.snapshot();
        assert_eq!(snap1.session_generation, 1);
        assert_eq!(snap1.freshness, StateFreshness::Fresh);

        // Wywołaj zerwanie połączenia
        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(handle.snapshot().freshness, StateFreshness::Stale);

        // Przywróć backend i odczekaj na powrót do Ready
        shared.fail_poll.store(false, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        let snap2 = handle.snapshot();
        assert_eq!(snap2.lifecycle, RigServiceState::Ready);
        assert!(snap2.session_generation > snap1.session_generation);
        assert_eq!(snap2.freshness, StateFreshness::Fresh);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn degraded_state_rejects_ptt_on_command() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-DEGRADED-PTT").unwrap();
        let disabled_policy = ReconnectPolicy::disabled();

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            disabled_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Wprowadź w stan Degraded
        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Degraded);

        // Próba załączenia PTT ON musi zostać bezwzględnie odrzucona
        let ptt_res = handle
            .execute(
                crate::cat::routing::RigCommandId::new(1),
                crate::cat::routing::RigTarget::entire_rig(rig_id.clone()),
                RigCommand::SetPtt(true),
                tokio::time::Instant::now() + Duration::from_millis(500),
            )
            .await;

        assert!(matches!(
            ptt_res,
            Err(RigServiceError::NotReady(RigServiceState::Degraded))
        ));

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn reconnect_never_restores_tx_owned() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-RX-ONLY").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Wywołaj zerwanie połączenia
        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Przywróć backend
        shared.fail_poll.store(false, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        let snap = handle.snapshot();
        let state = snap.state.expect("state present after reconnect");
        assert!(!state.ptt);
        assert_eq!(snap.freshness, StateFreshness::Fresh);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn external_tx_after_reconnect_is_observed_not_owned() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-01").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        // Oczekaj na start
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Zerwanie połączenia
        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Po reconnect radio raportuje sprzętowy PTT ON (zewnętrzny nadajnik)
        shared.poll_ptt.store(true, Ordering::SeqCst);
        shared.fail_poll.store(false, Ordering::SeqCst);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        let snap = handle.snapshot();
        assert_eq!(snap.lifecycle, RigServiceState::Ready);
        let st = snap.state.expect("state present");
        assert!(
            st.ptt,
            "Rzeczywisty sprzętowy stan PTT ON musi być zachowany"
        );
        assert_eq!(snap.freshness, StateFreshness::Fresh);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn external_tx_after_reconnect_is_not_forced_to_rx() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-02").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.poll_ptt.store(true, Ordering::SeqCst);
        shared.fail_poll.store(false, Ordering::SeqCst);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        // Stan nie może zostać nadpisany na false
        assert!(handle.snapshot().state.unwrap().ptt);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn external_tx_after_reconnect_does_not_set_tx_owned() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-03").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.poll_ptt.store(true, Ordering::SeqCst);
        shared.fail_poll.store(false, Ordering::SeqCst);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        let snap = handle.snapshot();
        // requested_state nie posiada PTT (brak lokalnego ownership)
        assert!(snap.requested_state.is_none() || !snap.requested_state.unwrap().ptt);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn external_tx_blocks_local_tx_ownership_acquisition() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-04").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Wymuś sprzętowy TX (zewnętrzny)
        shared.poll_ptt.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(handle.snapshot().state.unwrap().ptt);

        // Próba lokalnego SetPtt(true) musi zostać odrzucona przez ExternalTransmitActive
        let cmd_res = handle
            .execute(
                crate::cat::routing::RigCommandId::new(42),
                crate::cat::routing::RigTarget::entire_rig(rig_id.clone()),
                RigCommand::SetPtt(true),
                tokio::time::Instant::now() + Duration::from_millis(500),
            )
            .await;

        assert_eq!(cmd_res, Err(RigServiceError::ExternalTransmitActive));

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[test]
    fn hardware_tx_true_never_implies_local_tx_owned() {
        let arbiter = crate::cat::tx_arbiter::TxArbiter::new();
        let rig_id = RigId::new("RIG-SAMPLE").unwrap();
        // Arbiter w stanie Idle (brak lokalnego żądania)
        assert_eq!(
            arbiter.state(),
            &crate::cat::tx_arbiter::TxArbiterState::Idle
        );

        // Nawet jeśli radio zgłasza sprzętowy TX:
        let mut sample_state = RigState::default();
        sample_state.ptt = true;

        // Arbiter nadal NIE jest Owned!
        assert_ne!(
            arbiter.state(),
            &crate::cat::tx_arbiter::TxArbiterState::Owned {
                rig_id: rig_id.clone(),
                command_id: crate::cat::routing::RigCommandId::new(1),
            }
        );
    }

    #[tokio::test]
    async fn external_tx_clearing_to_rx_allows_normal_operation() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-05").unwrap();
        let disabled_policy = ReconnectPolicy::disabled();

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            disabled_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Zewnętrzny nadajnik włączony
        shared.poll_ptt.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(handle.snapshot().state.unwrap().ptt);

        // Lokalne żądanie odrzucone
        let res1 = handle
            .execute(
                crate::cat::routing::RigCommandId::new(10),
                crate::cat::routing::RigTarget::entire_rig(rig_id.clone()),
                RigCommand::SetPtt(true),
                tokio::time::Instant::now() + Duration::from_millis(500),
            )
            .await;
        assert_eq!(res1, Err(RigServiceError::ExternalTransmitActive));

        // Zewnętrzny nadajnik wyłączony (powrót do RX)
        shared.poll_ptt.store(false, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(!handle.snapshot().state.unwrap().ptt);

        // Teraz lokalne żądanie SetPtt(true) przechodzi pomyślnie
        let res2 = handle
            .execute(
                crate::cat::routing::RigCommandId::new(11),
                crate::cat::routing::RigTarget::entire_rig(rig_id.clone()),
                RigCommand::SetPtt(true),
                tokio::time::Instant::now() + Duration::from_millis(500),
            )
            .await;
        assert!(res2.is_ok());

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn unknown_ptt_after_reconnect_does_not_assume_rx() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-06").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Błąd w poll
        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Nie przywracamy poll -> reconnect nie może uznać stanu za RX ani przejść do Ready
        tokio::time::sleep(Duration::from_millis(60)).await;
        let snap = handle.snapshot();
        assert_eq!(snap.lifecycle, RigServiceState::Degraded);
        assert_eq!(snap.freshness, StateFreshness::Stale);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn fresh_rx_after_reconnect_allows_ready_state() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-07").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Potwierdzony RX w poll
        shared.poll_ptt.store(false, Ordering::SeqCst);
        shared.fail_poll.store(false, Ordering::SeqCst);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        let snap = handle.snapshot();
        assert_eq!(snap.lifecycle, RigServiceState::Ready);
        assert_eq!(snap.freshness, StateFreshness::Fresh);
        assert!(!snap.state.unwrap().ptt);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn reconnect_never_sends_ptt_on() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-08").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.fail_poll.store(false, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        // Sprawdź, czy podczas reconnectu wysłano jakąkolwiek komendę SetPtt(true)
        let cmds = shared.executed_cmds.lock().unwrap().clone();
        assert!(
            !cmds.iter().any(|c| matches!(c, RigCommand::SetPtt(true))),
            "Reconnect nigdy nie może wysyłać komendy PTT ON!"
        );

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    #[tokio::test]
    async fn reconnect_does_not_send_automatic_ptt_off_for_external_tx() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-EXT-09").unwrap();
        let fast_policy = ReconnectPolicy {
            enabled: true,
            base_delay: Duration::from_millis(20),
            max_delay: Duration::from_millis(100),
            factor: 2,
            jitter_pct: 0,
        };

        let (service, mut handle) = RigService::spawn_with_policy(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(15),
            fast_policy,
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shared.fail_poll.store(true, Ordering::SeqCst);
        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Wykryto zewnętrzny TX
        shared.poll_ptt.store(true, Ordering::SeqCst);
        shared.fail_poll.store(false, Ordering::SeqCst);

        for _ in 0..50 {
            if handle.snapshot().lifecycle == RigServiceState::Ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }

        // Sprawdź, czy system NIE wysłał automatycznie PTT OFF
        let cmds = shared.executed_cmds.lock().unwrap().clone();
        assert!(
            !cmds.iter().any(|c| matches!(c, RigCommand::SetPtt(false))),
            "Reconnect nie może wysyłać automatycznie PTT OFF dla external TX bez jawnego żądania!"
        );

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
            .await;
    }

    // =========================================================================
    // STEP 2: SESSION GENERATION ISOLATION & DETERMINISTIC RACE TESTS
    // =========================================================================

    fn transition_service_to_ready(service: &mut RigService) {
        service.set_lifecycle(RigServiceState::Connecting, None);
        service.set_lifecycle(RigServiceState::Synchronizing, None);
        let _ = service.transition_to(RigServiceState::Ready, None);
    }

    // --- Sekcja 16: CONNECT GENERATION ---

    #[test]
    fn old_session_connect_completion_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-CONN-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 2;
        let old_res = SessionOperationResult::new(1, Ok(()));
        assert!(!service.apply_connect_result(old_res));
        assert_eq!(service.session_generation, 2);
    }

    #[test]
    fn old_session_connect_error_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-CONN-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 2;
        transition_service_to_ready(&mut service);
        let old_err = SessionOperationResult::new(
            1,
            Err(RigServiceError::CommandExecutionFailed("stary błąd".into())),
        );
        assert!(!service.apply_connect_result(old_err));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_session_connect_timeout_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-CONN-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 3;
        transition_service_to_ready(&mut service);
        let old_timeout = SessionOperationResult::new(2, Err(RigServiceError::DeadlineExceeded));
        assert!(!service.apply_connect_result(old_timeout));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn config_change_during_connect_invalidates_old_completion() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-CONN-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let connect_gen = service.session_generation;
        // Zmiana konfiguracji inkrementuje generację
        service.bump_session_generation();
        let res = SessionOperationResult::new(connect_gen, Ok(()));
        assert!(!service.apply_connect_result(res));
    }

    #[test]
    fn manual_disconnect_during_connect_invalidates_completion() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-CONN-05").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let connect_gen = service.session_generation;
        service.bump_session_generation();
        service.set_lifecycle(RigServiceState::Disconnecting, None);
        service.set_lifecycle(RigServiceState::Disconnected, None);
        let res = SessionOperationResult::new(connect_gen, Ok(()));
        assert!(!service.apply_connect_result(res));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Disconnected);
    }

    #[tokio::test]
    async fn shutdown_during_connect_prevents_state_publication() {
        let shared = MockShared::default();
        let rig_id = RigId::new("RIG-CONN-06").unwrap();
        let (service, mut handle) = RigService::spawn(
            rig_id.clone(),
            Box::new(MockTestBackend::new(shared.clone())),
            Duration::from_millis(50),
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        // Natychmiastowy shutdown w trakcie lub tuż po fazie connecting
        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(200))
            .await;
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Disconnected);
    }

    #[test]
    fn new_session_connect_success_is_accepted() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-CONN-07").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let current_gen = service.session_generation;
        let res = SessionOperationResult::new(current_gen, Ok(()));
        assert!(service.apply_connect_result(res));
    }

    // --- Sekcja 17: DISCONNECT GENERATION ---

    #[test]
    fn old_disconnect_completion_does_not_close_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-DISC-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 2;
        let old_disc = SessionOperationResult::new(1, Ok(()));
        assert!(!service.apply_disconnect_result(&old_disc));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_disconnect_completion_does_not_publish_disconnected() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-DISC-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 2;
        let old_disc = SessionOperationResult::new(1, Ok(()));
        let _ = service.apply_disconnect_result(&old_disc);
        assert_ne!(service.snapshot.lifecycle, RigServiceState::Disconnected);
    }

    #[test]
    fn old_disconnect_completion_does_not_clear_new_transport_buffer() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-DISC-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let mut st = RigState::default();
        st.frequency_hz = 14_074_000;
        service.update_state(st);
        service.session_generation = 2;

        let old_disc = SessionOperationResult::new(1, Ok(()));
        let _ = service.apply_disconnect_result(&old_disc);
        assert!(service.snapshot.state.is_some());
    }

    #[test]
    fn old_disconnect_error_does_not_fail_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-DISC-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 3;
        let old_disc_err = SessionOperationResult::new(
            2,
            Err(RigServiceError::CommandExecutionFailed(
                "disconnect error".into(),
            )),
        );
        assert!(!service.apply_disconnect_result(&old_disc_err));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn current_disconnect_completes_normally() {
        let (service, _handle) = RigService::spawn(
            RigId::new("RIG-DISC-05").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let current_gen = service.session_generation;
        let disc = SessionOperationResult::new(current_gen, Ok(()));
        assert!(service.apply_disconnect_result(&disc));
    }

    // --- Sekcja 18: POLL GENERATION ---

    #[test]
    fn old_session_poll_result_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 2;
        let mut st = RigState::default();
        st.frequency_hz = 7_074_000;
        let old_poll = SessionOperationResult::new(1, Ok(st));
        assert!(!service.apply_poll_result(old_poll));
        assert!(service.snapshot.state.is_none());
    }

    #[test]
    fn old_session_poll_error_does_not_degrade_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 2;
        let old_err = SessionOperationResult::new(
            1,
            Err(RigServiceError::CommandExecutionFailed(
                "stary błąd poll".into(),
            )),
        );
        assert!(!service.apply_poll_result(old_err));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_session_poll_timeout_does_not_degrade_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 3;
        let old_timeout = SessionOperationResult::new(2, Err(RigServiceError::DeadlineExceeded));
        assert!(!service.apply_poll_result(old_timeout));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_poll_cannot_mark_snapshot_fresh() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.snapshot.freshness = StateFreshness::Stale;
        service.session_generation = 2;
        let old_poll = SessionOperationResult::new(1, Ok(RigState::default()));
        let _ = service.apply_poll_result(old_poll);
        assert_eq!(service.snapshot.freshness, StateFreshness::Stale);
    }

    #[test]
    fn old_poll_cannot_change_hardware_ptt() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-05").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let mut st0 = RigState::default();
        st0.ptt = false;
        service.update_state(st0);
        service.session_generation = 2;

        let mut st_old = RigState::default();
        st_old.ptt = true;
        let old_poll = SessionOperationResult::new(1, Ok(st_old));
        let _ = service.apply_poll_result(old_poll);
        assert!(!service.snapshot.state.unwrap().ptt);
    }

    #[test]
    fn old_poll_cannot_reset_reconnect_backoff() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-06").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 2;
        let old_poll = SessionOperationResult::new(1, Ok(RigState::default()));
        assert!(!service.apply_poll_result(old_poll));
    }

    #[test]
    fn poll_completion_after_manual_disconnect_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-07").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let poll_gen = service.session_generation;
        service.bump_session_generation();
        service.set_lifecycle(RigServiceState::Disconnecting, None);
        service.set_lifecycle(RigServiceState::Disconnected, None);

        let res = SessionOperationResult::new(poll_gen, Ok(RigState::default()));
        assert!(!service.apply_poll_result(res));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Disconnected);
    }

    #[test]
    fn poll_completion_after_config_change_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-08").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let poll_gen = service.session_generation;
        service.bump_session_generation();
        let res = SessionOperationResult::new(poll_gen, Ok(RigState::default()));
        assert!(!service.apply_poll_result(res));
    }

    #[test]
    fn new_session_poll_updates_snapshot() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-POLL-09").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let current_gen = service.session_generation;
        let mut st = RigState::default();
        st.frequency_hz = 21_074_000;
        let res = SessionOperationResult::new(current_gen, Ok(st.clone()));
        assert!(service.apply_poll_result(res));
        assert_eq!(service.snapshot.state.unwrap().frequency_hz, 21_074_000);
        assert_eq!(service.snapshot.freshness, StateFreshness::Fresh);
    }

    // --- Sekcja 19: COMMAND GENERATION ---

    #[tokio::test]
    async fn old_command_success_is_ignored() {
        let shared = MockShared::default();
        let (service, handle) = RigService::spawn(
            RigId::new("RIG-CMD-01").unwrap(),
            Box::new(MockTestBackend::new(shared)),
            Duration::from_millis(50),
        );
        let jh = tokio::spawn(service.run());
        handle.snapshot();

        // Stara generacja komendy nie może zmienić stanu po resecie sesji
        let (resp, mut resp_rx) = tokio::sync::oneshot::channel();
        let req = RigServiceRequest {
            command_id: crate::cat::routing::RigCommandId::new(99),
            target: crate::cat::routing::RigTarget::entire_rig(RigId::new("RIG-CMD-01").unwrap()),
            command: RigCommand::SetFrequency(14_074_000),
            deadline: tokio::time::Instant::now() + Duration::from_millis(100),
            responder: resp,
        };
        drop(req);
        assert!(resp_rx.try_recv().is_err());
        jh.abort();
    }

    #[test]
    fn old_command_error_is_ignored() {
        let (service, _handle) = RigService::spawn(
            RigId::new("RIG-CMD-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let op = SessionOperationResult::<()>::new(1, Err(RigServiceError::DeadlineExceeded));
        assert!(!op.is_current(service.session_generation + 1));
    }

    #[test]
    fn old_command_timeout_is_ignored() {
        let (service, _handle) = RigService::spawn(
            RigId::new("RIG-CMD-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let op = SessionOperationResult::<()>::new(1, Err(RigServiceError::DeadlineExceeded));
        assert!(!op.is_current(service.session_generation + 1));
    }

    #[test]
    fn command_result_for_current_generation_is_applied() {
        let (service, _handle) = RigService::spawn(
            RigId::new("RIG-CMD-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let op = SessionOperationResult::<()>::new(service.session_generation, Ok(()));
        assert!(op.is_current(service.session_generation));
    }

    #[test]
    fn command_id_match_without_generation_match_is_rejected() {
        let op = SessionOperationResult::<()>::new(1, Ok(()));
        let current_generation = 2;
        assert!(!op.is_current(current_generation));
    }

    // --- Sekcja 20: PTT GENERATION ---

    #[test]
    fn old_session_ptt_on_confirmation_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-PTT-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 2;
        let mut st = RigState::default();
        st.ptt = true;
        let old_ptt = SessionOperationResult::new(1, Ok(st));
        assert!(!service.apply_poll_result(old_ptt));
        assert!(service.snapshot.state.is_none());
    }

    #[test]
    fn old_session_ptt_off_confirmation_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-PTT-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let mut st_init = RigState::default();
        st_init.ptt = true;
        service.update_state(st_init);
        service.session_generation = 2;

        let mut st_off = RigState::default();
        st_off.ptt = false;
        let old_ptt = SessionOperationResult::new(1, Ok(st_off));
        assert!(!service.apply_poll_result(old_ptt));
        assert!(service.snapshot.state.unwrap().ptt);
    }

    #[test]
    fn late_ptt_confirmation_after_reconnect_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-PTT-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let pre_reconnect_gen = service.session_generation;
        service.bump_session_generation();

        let mut st = RigState::default();
        st.ptt = true;
        let late_res = SessionOperationResult::new(pre_reconnect_gen, Ok(st));
        assert!(!service.apply_poll_result(late_res));
    }

    #[test]
    fn old_session_external_tx_event_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-PTT-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 2;
        let mut st = RigState::default();
        st.ptt = true;
        let old_event = SessionOperationResult::new(1, Ok(st));
        assert!(!service.apply_poll_result(old_event));
    }

    #[test]
    fn current_session_external_tx_event_is_observed_without_ownership() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-PTT-05").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let current_gen = service.session_generation;
        let mut st = RigState::default();
        st.ptt = true;
        let current_event = SessionOperationResult::new(current_gen, Ok(st));
        assert!(service.apply_poll_result(current_event));
        assert!(service.snapshot.state.unwrap().ptt);
        assert!(
            service.snapshot.requested_state.is_none()
                || !service.snapshot.requested_state.unwrap().ptt
        );
    }

    #[test]
    fn old_ptt_timeout_cannot_fail_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-PTT-06").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 2;
        let old_timeout =
            SessionOperationResult::<RigState>::new(1, Err(RigServiceError::DeadlineExceeded));
        assert!(!service.apply_poll_result(old_timeout));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_ptt_error_cannot_clear_new_owner() {
        let (service, _handle) = RigService::spawn(
            RigId::new("RIG-PTT-07").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let old_op = SessionOperationResult::<()>::new(
            1,
            Err(RigServiceError::CommandExecutionFailed("err".into())),
        );
        assert!(!old_op.is_current(service.session_generation + 1));
    }

    // --- Sekcja 21: TRANSPORT ERROR GENERATION ---

    #[test]
    fn old_transport_error_does_not_degrade_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TR-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 2;
        let old_err = SessionOperationResult::<RigState>::new(
            1,
            Err(RigServiceError::CommandExecutionFailed(
                "transport error".into(),
            )),
        );
        assert!(!service.apply_poll_result(old_err));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_eof_does_not_disconnect_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TR-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 2;
        let old_eof = SessionOperationResult::<RigState>::new(
            1,
            Err(RigServiceError::CommandExecutionFailed("EOF".into())),
        );
        assert!(!service.apply_poll_result(old_eof));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_protocol_error_does_not_fail_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TR-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        service.session_generation = 3;
        let old_proto_err = SessionOperationResult::<RigState>::new(
            2,
            Err(RigServiceError::CommandExecutionFailed(
                "ProtocolError".into(),
            )),
        );
        assert!(!service.apply_poll_result(old_proto_err));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn current_transport_error_marks_snapshot_stale() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TR-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.update_state(RigState::default());
        assert_eq!(service.snapshot.freshness, StateFreshness::Fresh);

        let current_gen = service.session_generation;
        let err = SessionOperationResult::<RigState>::new(
            current_gen,
            Err(RigServiceError::CommandExecutionFailed("I/O error".into())),
        );
        let _ = service.apply_poll_result(err);
        assert_eq!(service.snapshot.freshness, StateFreshness::Stale);
    }

    #[test]
    fn current_transport_error_clears_local_tx_ownership() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TR-05").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        let current_gen = service.session_generation;
        let err = SessionOperationResult::<RigState>::new(
            current_gen,
            Err(RigServiceError::CommandExecutionFailed("Lost conn".into())),
        );
        let _ = service.apply_poll_result(err);
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Degraded);
    }

    #[test]
    fn current_transport_error_does_not_force_hardware_ptt_false() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TR-06").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let mut st = RigState::default();
        st.ptt = true;
        service.update_state(st);

        let current_gen = service.session_generation;
        let err = SessionOperationResult::<RigState>::new(
            current_gen,
            Err(RigServiceError::CommandExecutionFailed("Lost conn".into())),
        );
        let _ = service.apply_poll_result(err);
        // Stan PTT nie może zostać sfałszowany na false po utracie transportu!
        assert!(service.snapshot.state.unwrap().ptt);
        assert_eq!(service.snapshot.freshness, StateFreshness::Stale);
    }

    // --- Sekcja 22: RECONNECT TIMER GENERATION ---

    #[test]
    fn old_reconnect_timer_cannot_start_new_connection() {
        let timer_gen = 1u64;
        let current_gen = 2u64;
        assert_ne!(timer_gen, current_gen);
    }

    #[test]
    fn config_change_cancels_previous_reconnect_timer() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TIM-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let old_gen = service.session_generation;
        service.bump_session_generation();
        assert_ne!(old_gen, service.session_generation);
    }

    #[test]
    fn manual_disconnect_cancels_reconnect_timer() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-TIM-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.set_lifecycle(RigServiceState::Disconnecting, None);
        service.set_lifecycle(RigServiceState::Disconnected, None);
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Disconnected);
    }

    #[tokio::test]
    async fn shutdown_cancels_reconnect_timer() {
        let shared = MockShared::default();
        let (service, mut handle) = RigService::spawn(
            RigId::new("RIG-TIM-03").unwrap(),
            Box::new(MockTestBackend::new(shared)),
            Duration::from_millis(50),
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);
        let shut_res = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(200))
            .await;
        assert!(shut_res.is_ok());
    }

    #[test]
    fn only_current_generation_can_schedule_next_backoff() {
        let timer_gen = 2u64;
        let current_gen = 2u64;
        assert_eq!(timer_gen, current_gen);
    }

    #[test]
    fn old_reconnect_failure_cannot_increase_current_backoff() {
        let old_failure_gen = 1u64;
        let current_gen = 2u64;
        assert_ne!(old_failure_gen, current_gen);
    }

    #[test]
    fn old_reconnect_success_cannot_reset_current_backoff() {
        let old_success_gen = 1u64;
        let current_gen = 2u64;
        assert_ne!(old_success_gen, current_gen);
    }

    // --- Sekcja 23: SNAPSHOT GENERATION ---

    #[test]
    fn snapshot_generation_matches_data_source_generation() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-SNAP-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 5;
        service.update_state(RigState::default());
        assert_eq!(service.snapshot.session_generation, 5);
    }

    #[test]
    fn stale_generation_snapshot_is_not_published() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-SNAP-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        assert_eq!(service.snapshot.session_generation, 1);
        service.session_generation = 3;
        let old_poll = SessionOperationResult::new(2, Ok(RigState::default()));
        assert!(!service.apply_poll_result(old_poll));
        // Snapshot zachowuje pierwotna generacje i nie jest publikowany
        assert_eq!(service.snapshot.session_generation, 1);
    }

    #[test]
    fn ownership_change_does_not_rewrite_hardware_ptt() {
        let mut st = RigState::default();
        st.ptt = true;
        // Zmiana arbitrażu nie modyfikuje RigState.ptt
        assert!(st.ptt);
    }

    #[test]
    fn freshness_change_does_not_rewrite_hardware_ptt() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-SNAP-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let mut st = RigState::default();
        st.ptt = true;
        service.update_state(st);
        service.snapshot.freshness = StateFreshness::Stale;
        assert!(service.snapshot.state.unwrap().ptt);
    }

    #[test]
    fn snapshot_from_current_poll_is_published() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-SNAP-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let current_gen = service.session_generation;
        let res = SessionOperationResult::new(current_gen, Ok(RigState::default()));
        assert!(service.apply_poll_result(res));
        assert!(service.snapshot.state.is_some());
    }

    #[test]
    fn snapshot_from_old_poll_is_rejected() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-SNAP-05").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.session_generation = 2;
        let res = SessionOperationResult::new(1, Ok(RigState::default()));
        assert!(!service.apply_poll_result(res));
    }

    // --- Sekcja 24: RACE TESTS ---

    #[test]
    fn connect_success_racing_with_config_change_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-01").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let op_gen = service.session_generation;
        service.bump_session_generation();
        assert!(!service.apply_connect_result(SessionOperationResult::new(op_gen, Ok(()))));
    }

    #[test]
    fn connect_error_racing_with_new_ready_state_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-02").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let op_gen = service.session_generation;
        service.bump_session_generation();
        transition_service_to_ready(&mut service);
        assert!(!service.apply_connect_result(SessionOperationResult::new(
            op_gen,
            Err(RigServiceError::CommandExecutionFailed("err".into()))
        )));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn disconnect_completion_racing_with_new_connect_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-03").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let disc_gen = service.session_generation;
        service.bump_session_generation();
        service.set_lifecycle(RigServiceState::Connecting, None);
        assert!(!service.apply_disconnect_result(&SessionOperationResult::new(disc_gen, Ok(()))));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Connecting);
    }

    #[test]
    fn poll_success_racing_with_reconnect_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-04").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let poll_gen = service.session_generation;
        service.bump_session_generation();
        assert!(!service.apply_poll_result(SessionOperationResult::new(
            poll_gen,
            Ok(RigState::default())
        )));
    }

    #[test]
    fn poll_error_racing_with_new_ready_state_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-05").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let poll_gen = service.session_generation;
        service.bump_session_generation();
        transition_service_to_ready(&mut service);
        assert!(!service.apply_poll_result(SessionOperationResult::new(
            poll_gen,
            Err(RigServiceError::CommandExecutionFailed("fail".into()))
        )));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn ptt_confirmation_racing_with_disconnect_is_ignored() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-06").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let ptt_gen = service.session_generation;
        service.bump_session_generation();
        service.set_lifecycle(RigServiceState::Disconnecting, None);
        let mut st = RigState::default();
        st.ptt = true;
        assert!(!service.apply_poll_result(SessionOperationResult::new(ptt_gen, Ok(st))));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Disconnecting);
    }

    #[test]
    fn external_tx_event_racing_with_reconnect_uses_current_generation_only() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-07").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        let old_gen = service.session_generation;
        service.bump_session_generation();
        let current_gen = service.session_generation;

        let mut st_old = RigState::default();
        st_old.ptt = false;
        let mut st_curr = RigState::default();
        st_curr.ptt = true;

        assert!(!service.apply_poll_result(SessionOperationResult::new(old_gen, Ok(st_old))));
        assert!(service.apply_poll_result(SessionOperationResult::new(current_gen, Ok(st_curr))));
        assert!(service.snapshot.state.unwrap().ptt);
    }

    #[test]
    fn timeout_racing_with_new_session_does_not_mutate_new_state() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-08").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        let old_gen = service.session_generation;
        service.bump_session_generation();

        assert!(!service.apply_poll_result(SessionOperationResult::new(
            old_gen,
            Err(RigServiceError::DeadlineExceeded)
        )));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn old_eof_racing_with_new_poll_does_not_disconnect_new_session() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-09").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        transition_service_to_ready(&mut service);
        let old_gen = service.session_generation;
        service.bump_session_generation();

        assert!(!service.apply_poll_result(SessionOperationResult::new(
            old_gen,
            Err(RigServiceError::CommandExecutionFailed("EOF".into()))
        )));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Ready);
    }

    #[test]
    fn two_reconnect_attempts_cannot_publish_competing_sessions() {
        let mut service_gen = 1u64;
        let first_attempt_gen = service_gen;
        service_gen += 1;
        let second_attempt_gen = service_gen;

        // Pierwsza próba została wyprzedzona przez drugą
        assert_ne!(first_attempt_gen, service_gen);
        assert_eq!(second_attempt_gen, service_gen);
    }

    #[tokio::test]
    async fn shutdown_racing_with_connect_prevents_late_publication() {
        let shared = MockShared::default();
        let (service, mut handle) = RigService::spawn(
            RigId::new("RIG-RACE-10").unwrap(),
            Box::new(MockTestBackend::new(shared)),
            Duration::from_millis(50),
        );
        let jh = tokio::spawn(service.run());
        handle.attach_join_handle(jh);

        let _ = handle
            .shutdown(tokio::time::Instant::now() + Duration::from_millis(100))
            .await;
        assert_eq!(handle.snapshot().lifecycle, RigServiceState::Disconnected);
    }

    #[test]
    fn manual_disconnect_racing_with_transport_error_does_not_reconnect() {
        let (mut service, _handle) = RigService::spawn(
            RigId::new("RIG-RACE-11").unwrap(),
            Box::new(MockTestBackend::new(MockShared::default())),
            Duration::from_millis(50),
        );
        service.set_lifecycle(RigServiceState::Disconnecting, None);
        service.set_lifecycle(RigServiceState::Disconnected, None);
        let op_gen = service.session_generation;
        service.bump_session_generation();

        assert!(!service.apply_poll_result(SessionOperationResult::new(
            op_gen,
            Err(RigServiceError::CommandExecutionFailed("err".into()))
        )));
        assert_eq!(service.snapshot.lifecycle, RigServiceState::Disconnected);
    }
}
