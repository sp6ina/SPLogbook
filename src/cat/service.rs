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

/// Usługa sesji radiowej zarządzająca pętlą I/O w dedykowanym tasku aktora.
pub struct RigService {
    rig_id: RigId,
    backend: Box<dyn RigBackend>,
    cmd_rx: mpsc::Receiver<RigServiceRequest>,
    event_tx: broadcast::Sender<RoutedRigEvent>,
    snapshot_tx: watch::Sender<RigServiceSnapshot>,
    shutdown_rx: oneshot::Receiver<()>,
    snapshot: RigServiceSnapshot,
    poll_interval: Duration,
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
        let caps = backend.capabilities();
        let (cmd_tx, cmd_rx) = mpsc::channel(MAX_RIG_COMMAND_QUEUE);
        let (event_tx, _) = broadcast::channel(MAX_RIG_EVENT_QUEUE);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let initial_snapshot = RigServiceSnapshot::initial(rig_id.clone(), caps);
        let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot.clone());

        let service = Self {
            rig_id: rig_id.clone(),
            backend,
            cmd_rx,
            event_tx: event_tx.clone(),
            snapshot_tx,
            shutdown_rx,
            snapshot: initial_snapshot,
            poll_interval,
        };

        let handle = RigServiceHandle {
            rig_id,
            cmd_tx,
            snapshot_rx,
            event_tx,
            shutdown_tx: Some(shutdown_tx),
            join_handle: None,
        };

        (service, handle)
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
        self.snapshot.last_error = error
            .as_deref()
            .map(crate::cat::tx_arbiter::redact_sensitive_info);
        self.snapshot.last_update = Some(tokio::time::Instant::now());

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
        self.snapshot.state = Some(state.clone());
        self.snapshot.last_update = Some(tokio::time::Instant::now());
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
        let connect_res = tokio::time::timeout(connect_timeout, self.backend.connect()).await;

        match connect_res {
            Ok(Ok(())) => {
                // 2. Faza Synchronizing: wstępny odczyt stanu radia
                self.set_lifecycle(RigServiceState::Synchronizing, None);
                let poll_timeout = Duration::from_millis(3000);
                match tokio::time::timeout(poll_timeout, self.backend.poll()).await {
                    Ok(Ok(initial_state)) => {
                        self.update_state(initial_state);
                        self.set_lifecycle(RigServiceState::Ready, None);
                    }
                    Ok(Err(e)) => {
                        log::warn!(
                            "[{}] Początkowy odczyt stanu nie powiódł się: {e}, przejście do Degraded",
                            self.rig_id
                        );
                        self.set_lifecycle(RigServiceState::Degraded, Some(e.to_string()));
                    }
                    Err(_) => {
                        log::warn!(
                            "[{}] Timeout początkowego odczytu stanu, przejście do Degraded",
                            self.rig_id
                        );
                        self.set_lifecycle(
                            RigServiceState::Degraded,
                            Some("Timeout synchronizacji stanu".to_string()),
                        );
                    }
                }
            }
            Ok(Err(e)) => {
                let err_msg = format!("Błąd connect: {e}");
                self.set_lifecycle(RigServiceState::Failed, Some(err_msg.clone()));
                return Err(RigServiceError::CommandExecutionFailed(err_msg));
            }
            Err(_) => {
                let err_msg = "Timeout nawiązywania połączenia".to_string();
                self.set_lifecycle(RigServiceState::Failed, Some(err_msg.clone()));
                return Err(RigServiceError::DeadlineExceeded);
            }
        }

        // 3. Faza Ready / Degraded: główna pętla obsługi komend i pollingu
        let mut interval = tokio::time::interval(self.poll_interval);
        // Pierwszy tick interwału odpala się natychmiast – pomijamy go, bo zrobiliśmy synchronizację
        interval.tick().await;

        loop {
            tokio::select! {
                // Odbiór żądania wyłączenia sesji (shutdown)
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

                // Polling cykliczny stanu transceivera
                _ = interval.tick() => {
                    if self.snapshot.lifecycle == RigServiceState::Ready || self.snapshot.lifecycle == RigServiceState::Degraded {
                        let poll_timeout = Duration::from_millis(1500);
                        match tokio::time::timeout(poll_timeout, self.backend.poll()).await {
                            Ok(Ok(state)) => {
                                self.update_state(state);
                                if self.snapshot.lifecycle == RigServiceState::Degraded {
                                    self.set_lifecycle(RigServiceState::Ready, None);
                                }
                            }
                            Ok(Err(e)) => {
                                log::debug!("[{}] Błąd pollingu stanu: {e}", self.rig_id);
                                if self.snapshot.lifecycle == RigServiceState::Ready {
                                    self.set_lifecycle(RigServiceState::Degraded, Some(e.to_string()));
                                }
                            }
                            Err(_) => {
                                log::debug!("[{}] Timeout pollingu stanu", self.rig_id);
                                if self.snapshot.lifecycle == RigServiceState::Ready {
                                    self.set_lifecycle(RigServiceState::Degraded, Some("Timeout pollingu".to_string()));
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

        // 3. Oblicz pozostały czas do deadline
        let now = tokio::time::Instant::now();
        let remaining = if req.deadline > now {
            req.deadline - now
        } else {
            let _ = req.responder.send(Err(RigServiceError::DeadlineExceeded));
            return;
        };

        // 4. Wykonaj polecenie na backendzie z bounded timeoutem oraz uwzględnieniem docelowego odbiornika
        let exec_res = tokio::time::timeout(
            remaining,
            self.backend
                .execute_routed(req.target.receiver_id.as_ref(), &req.command),
        )
        .await;

        match exec_res {
            Ok(Ok(())) => {
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

/// Bezpieczny uchwyt do usługi sesji radiowej ([`RigService`]).
///
/// Posiada wyłącznie ograniczone nadajniki kanałów, odbiornik snapshotu oraz
/// zarządzany [`tokio::task::JoinHandle`]. Nie posiada referencji do backendu ani transportu.
pub struct RigServiceHandle {
    rig_id: RigId,
    cmd_tx: mpsc::Sender<RigServiceRequest>,
    snapshot_rx: watch::Receiver<RigServiceSnapshot>,
    event_tx: broadcast::Sender<RoutedRigEvent>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<Result<(), RigServiceError>>>,
}

impl RigServiceHandle {
    /// Przypisuje `JoinHandle` uruchomionego taska do uchwytu.
    pub fn attach_join_handle(&mut self, handle: JoinHandle<Result<(), RigServiceError>>) {
        self.join_handle = Some(handle);
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
    /// W przypadku przekroczenia terminu task zostaje przerwany (`abort()`).
    pub async fn shutdown(
        &mut self,
        deadline: tokio::time::Instant,
    ) -> Result<(), RigServiceError> {
        // 1. Wyślij sygnał wyłączenia (idempotentnie)
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // 2. Oczekuj na zakończenie pętli taska (JoinHandle)
        if let Some(mut handle) = self.join_handle.take() {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                handle.abort();
                return Err(RigServiceError::DeadlineExceeded);
            }
            let remaining = deadline - now;

            match tokio::time::timeout(remaining, &mut handle).await {
                Ok(Ok(_run_res)) => Ok(()),
                Ok(Err(join_err)) => {
                    log::error!(
                        "[{}] Błąd łączenia taska aktora (JoinError): {join_err}",
                        self.rig_id
                    );
                    Err(RigServiceError::BackendPanic(join_err.to_string()))
                }
                Err(_) => {
                    log::warn!(
                        "[{}] Przekroczono limit czasu shutdown, wymuszenie przerwania taska (abort)",
                        self.rig_id
                    );
                    handle.abort();
                    Err(RigServiceError::DeadlineExceeded)
                }
            }
        } else {
            // Task został już wcześniej zamknięty
            Ok(())
        }
    }
}
