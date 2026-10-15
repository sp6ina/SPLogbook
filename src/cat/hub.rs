// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Główny dyspozytor i koordynator stacji wieloradiowej ([`RigHub`]) (NATIVE-6A).
//!
//! Zarządza rejestrem sesji radiowych ([`BTreeMap<RigId, RigServiceHandle>`]),
//! deterministycznym routingiem poleceń ([`RoutedRigCommand`]), fokusami odbioru
//! i nadawania ([`ActiveRxTarget`], [`ActiveTxTarget`]), bezpiecznym arbitrem TX
//! ([`TxArbiter`]) oraz kontrolowanym, idempotentnym zamykaniem wszystkich sesji.
//!
//! # Kluczowe zasady:
//! - Brak `Box<dyn RigBackend>`, socketów czy transportów w `RigHub`.
//! - Brak niejawnego fallbacku routingu na pierwsze radio w rejestrze.
//! - Brak aliasowania VFO A/B ze slice FlexRadio.
//! - Całkowita izolacja awarii pojedynczej sesji radiowej.
//! - Gwarancja oczekiwania na wszystkie [`tokio::task::JoinHandle`] podczas shutdown.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::broadcast;

use crate::cat::rig::RigCommand;
use crate::cat::routing::{
    ActiveRxTarget, ActiveTxTarget, RigCommandId, RigHubEvent, RigId, RigTarget, RoutedRigCommand,
    RoutedRigEvent, RoutingError,
};
use crate::cat::service::{RigServiceError, RigServiceHandle, RigServiceSnapshot, RigServiceState};
use crate::cat::tx_arbiter::{
    PttCommandOutcome, TxArbiter, TxArbiterError, TxArbiterState, TxUnknownReason,
};

/// Maksymalna liczba zdarzeń buforowanych w globalnej magistrali huba.
pub const MAX_HUB_EVENT_QUEUE: usize = 256;

/// Maksymalna dopuszczalna liczba oczekujących asynchronicznych żądań w hubie.
pub const MAX_PENDING_HUB_COMMANDS: usize = 64;

/// Niezależny snapshot stanu całego [`RigHub`] i wszystkich zarejestrowanych sesji.
#[derive(Debug, Clone, PartialEq)]
pub struct RigHubSnapshot {
    /// Snapshoty poszczególnych sesji radiowych w stabilnej kolejności kluczy [`RigId`].
    pub rigs: BTreeMap<RigId, RigServiceSnapshot>,
    /// Aktualny cel odbiorczy (Active RX), o ile skonfigurowany.
    pub active_rx: Option<RigTarget>,
    /// Aktualny cel nadawczy (Active TX), o ile skonfigurowany.
    pub active_tx: Option<RigTarget>,
    /// Stan arbitra nadawania TX.
    pub tx_state: TxArbiterState,
    /// Czy hub znajduje się w trakcie procedury shutdown.
    pub shutting_down: bool,
}

/// Błędy operacyjne węzła sterowania wieloradiowego [`RigHub`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigHubError {
    /// Błąd routingu lub adresowania celu.
    Routing(RoutingError),
    /// Błąd wykonania zwrócony przez usługę sesji radiowej.
    Service(RigServiceError),
    /// Błąd arbitrażu nadawania TX.
    TxArbiter(TxArbiterError),
    /// Sesja o podanym identyfikatorze już istnieje w hubie.
    RigAlreadyExists(RigId),
    /// Wskazana sesja radiowa nie została odnaleziona w rejestrze.
    RigNotFound(RigId),
    /// Próba wykonania operacji na aktywnym RX, gdy cel nie został ustawiony.
    ActiveRxNotConfigured,
    /// Próba wykonania operacji na aktywnym TX, gdy cel nie został ustawiony.
    ActiveTxNotConfigured,
    /// Zmiana celu Active TX na inne radio jest zablokowana w trakcie trwania nadawania (stan Owned).
    CannotChangeActiveTxWhileOwned {
        /// Radio aktualnie posiadające nadajnik.
        current_owner: RigId,
        /// Żądane nowe radio docelowe.
        requested_rig: RigId,
    },
    /// Przekroczono dopuszczalny limit czasu operacji w hubie.
    Timeout,
    /// Przepełnienie kolejki żądań huba.
    QueueFull,
    /// Hub jest w trakcie zamykania (shutdown) i odrzuca nowe polecenia.
    HubShuttingDown,
    /// Błąd w trakcie procedury shutdown.
    ShutdownError(String),
}

impl fmt::Display for RigHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Routing(e) => write!(f, "błąd routingu: {e}"),
            Self::Service(e) => write!(f, "błąd usługi sesji: {e}"),
            Self::TxArbiter(e) => write!(f, "błąd arbitrażu TX: {e}"),
            Self::RigAlreadyExists(id) => write!(f, "radio '{id}' jest już zarejestrowane w hubie"),
            Self::RigNotFound(id) => write!(f, "radio '{id}' nie zostało odnalezione w hubie"),
            Self::ActiveRxNotConfigured => {
                write!(f, "aktywny cel odbiorczy (Active RX) nie jest ustawiony")
            }
            Self::ActiveTxNotConfigured => {
                write!(f, "aktywny cel nadawczy (Active TX) nie jest ustawiony")
            }
            Self::CannotChangeActiveTxWhileOwned {
                current_owner,
                requested_rig,
            } => write!(
                f,
                "nie można zmienić celu TX na '{requested_rig}': radio '{current_owner}' aktualnie nadaje (Owned)"
            ),
            Self::Timeout => write!(f, "przekroczono limit czasu operacji w hubie"),
            Self::QueueFull => write!(f, "kolejka operacji huba jest przepełniona"),
            Self::HubShuttingDown => {
                write!(f, "RigHub jest w trakcie wyłączania (HubShuttingDown)")
            }
            Self::ShutdownError(msg) => write!(f, "błąd zamykania huba: {msg}"),
        }
    }
}

impl std::error::Error for RigHubError {}

impl From<RoutingError> for RigHubError {
    fn from(e: RoutingError) -> Self {
        Self::Routing(e)
    }
}

impl From<RigServiceError> for RigHubError {
    fn from(e: RigServiceError) -> Self {
        Self::Service(e)
    }
}

impl From<TxArbiterError> for RigHubError {
    fn from(e: TxArbiterError) -> Self {
        Self::TxArbiter(e)
    }
}

/// Centralny zarządca sesji radiowych i arbitrażu nadawania.
pub struct RigHub {
    services: BTreeMap<RigId, RigServiceHandle>,
    active_rx: Option<ActiveRxTarget>,
    active_tx: Option<ActiveTxTarget>,
    tx_arbiter: TxArbiter,
    event_tx: broadcast::Sender<RoutedRigEvent>,
    next_cmd_counter: AtomicU64,
    pending_commands: StdMutex<BTreeSet<RigCommandId>>,
    shutting_down: bool,
}

impl Default for RigHub {
    fn default() -> Self {
        Self::new()
    }
}

impl RigHub {
    /// Tworzy nowy pusty węzeł [`RigHub`].
    pub fn new() -> Self {
        let (event_tx, _event_rx) = broadcast::channel(MAX_HUB_EVENT_QUEUE);
        Self {
            services: BTreeMap::new(),
            active_rx: None,
            active_tx: None,
            tx_arbiter: TxArbiter::new(),
            event_tx,
            next_cmd_counter: AtomicU64::new(1),
            pending_commands: StdMutex::new(BTreeSet::new()),
            shutting_down: false,
        }
    }

    /// Generuje kolejny unikalny, monotoniczny identyfikator polecenia [`RigCommandId`],
    /// rejestrując go w puli oczekujących żądań oraz pomijając aktywne ID przy wraparound (ETAP 8).
    pub fn next_command_id(&self) -> RigCommandId {
        let mut pending = self
            .pending_commands
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        loop {
            let val = self.next_cmd_counter.fetch_add(1, Ordering::Relaxed);
            if let Some(id) = RigCommandId::from_u64(val) {
                if !pending.contains(&id) {
                    if pending.len() < MAX_PENDING_HUB_COMMANDS {
                        pending.insert(id);
                    }
                    return id;
                }
            }
            // Wraparound 0 pomijany automatycznie
        }
    }

    /// Zwraca liczbę aktualnie oczekujących poleceń w rejestrze huba.
    pub fn pending_command_count(&self) -> usize {
        self.pending_commands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Oznacza polecenie jako ukończone, zwalniając identyfikator z rejestru oczekujących.
    pub fn complete_command(&self, id: RigCommandId) {
        let mut pending = self
            .pending_commands
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        pending.remove(&id);
    }

    /// Zwraca liczbę zarejestrowanych sesji radiowych.
    pub fn rig_count(&self) -> usize {
        self.services.len()
    }

    /// Sprawdza, czy radio o danym identyfikatorze jest zarejestrowane.
    pub fn contains_rig(&self, rig_id: &RigId) -> bool {
        self.services.contains_key(rig_id)
    }

    /// Rejestruje nową sesję radiową w hubie.
    ///
    /// # Błędy
    /// Zwraca [`RigHubError::RigAlreadyExists`], jeśli radio o danym `RigId` jest już zarejestrowane.
    pub fn register_rig(
        &mut self,
        rig_id: RigId,
        handle: RigServiceHandle,
    ) -> Result<(), RigHubError> {
        if self.shutting_down {
            return Err(RigHubError::HubShuttingDown);
        }

        if self.services.contains_key(&rig_id) {
            return Err(RigHubError::RigAlreadyExists(rig_id));
        }

        let _ = self.event_tx.send(RoutedRigEvent::new(
            rig_id.clone(),
            None,
            RigHubEvent::RigRegistered,
        ));

        self.services.insert(rig_id, handle);
        Ok(())
    }

    /// Rejestruje usługę sesji radiowej pobierając `RigId` bezpośrednio z uchwytu.
    pub fn register_service(&mut self, handle: RigServiceHandle) -> Result<(), RigHubError> {
        let rig_id = handle.rig_id().clone();
        self.register_rig(rig_id, handle)
    }

    /// Wyrejestrowuje i zatrzymuje usługę sesji radiowej o danym `RigId`.
    pub async fn unregister_service(&mut self, rig_id: &RigId) -> Result<(), RigHubError> {
        self.remove_rig(rig_id, Duration::from_secs(5)).await
    }

    /// Zwraca listę wszystkich zarejestrowanych identyfikatorów radiowych.
    pub fn registered_rig_ids(&self) -> Vec<RigId> {
        self.services.keys().cloned().collect()
    }

    /// Usuwa sesję radiową z huba z wykonaniem jej kontrolowanego rozłączenia.
    ///
    /// Automatycznie unieważnia cele Active RX / Active TX wskazujące usuwane radio
    /// oraz przełącza [`TxArbiter`] w stan `Unknown`, jeżeli usuwana sesja brała udział w nadawaniu.
    pub async fn remove_rig(
        &mut self,
        rig_id: &RigId,
        timeout: Duration,
    ) -> Result<(), RigHubError> {
        let mut handle = self
            .services
            .remove(rig_id)
            .ok_or_else(|| RigHubError::RigNotFound(rig_id.clone()))?;

        // 1. Unieważnij cele, jeśli wskazywały usuwane radio
        if let Some(ref rx) = self.active_rx {
            if rx.rig_id() == rig_id {
                self.active_rx = None;
                let _ = self.event_tx.send(RoutedRigEvent::new(
                    rig_id.clone(),
                    None,
                    RigHubEvent::ActiveRxChanged(None),
                ));
            }
        }

        if let Some(ref tx) = self.active_tx {
            if tx.rig_id() == rig_id {
                self.active_tx = None;
                let _ = self.event_tx.send(RoutedRigEvent::new(
                    rig_id.clone(),
                    None,
                    RigHubEvent::ActiveTxChanged(None),
                ));
            }
        }

        // 2. Jeśli usunięte radio brało udział w procedurze TX, ustaw Unknown
        self.tx_arbiter
            .handle_session_failure(rig_id, "sesja radiowa została usunięta z RigHub");

        // 3. Kontrolowane zatrzymanie taska sesji
        let deadline = tokio::time::Instant::now() + timeout;
        let shutdown_res = handle.shutdown(deadline).await;

        let _ = self.event_tx.send(RoutedRigEvent::new(
            rig_id.clone(),
            None,
            RigHubEvent::RigRemoved,
        ));

        shutdown_res.map_err(RigHubError::Service)
    }

    /// Ustawia aktywny cel odbiorczy (Active RX).
    ///
    /// Weryfikuje istnienie radia w rejestrze. Nie wysyła żadnych komend sterujących do radia.
    pub fn set_active_rx_target(&mut self, target: RigTarget) -> Result<(), RigHubError> {
        if self.shutting_down {
            return Err(RigHubError::HubShuttingDown);
        }

        if !self.services.contains_key(&target.rig_id) {
            return Err(RigHubError::RigNotFound(target.rig_id));
        }

        let new_target = target.clone();
        self.active_rx = Some(ActiveRxTarget::new(target.clone()));
        let _ = self.event_tx.send(RoutedRigEvent::new(
            target.rig_id,
            target.receiver_id,
            RigHubEvent::ActiveRxChanged(Some(new_target)),
        ));

        Ok(())
    }

    /// Ustawia aktywny cel nadawczy (Active TX).
    ///
    /// # Zasady bezpieczeństwa:
    /// - Jeżeli arbiter nadawania znajduje się w stanie `Owned` lub `Requesting` z innym radiem,
    ///   zmiana celu jest natychmiast odrzucana z błędem [`RigHubError::CannotChangeActiveTxWhileOwned`].
    /// - Zmiana celu nadawczego nigdy samoczynnie nie załącza PTT ani nie wysyła komend do radia.
    pub fn set_active_tx_target(&mut self, target: RigTarget) -> Result<(), RigHubError> {
        if self.shutting_down {
            return Err(RigHubError::HubShuttingDown);
        }

        if !self.services.contains_key(&target.rig_id) {
            return Err(RigHubError::RigNotFound(target.rig_id));
        }

        // Blokada zmiany celu TX w trakcie trwającego nadawania
        if let Some(owner) = self.tx_arbiter.current_owner() {
            if owner != &target.rig_id {
                return Err(RigHubError::CannotChangeActiveTxWhileOwned {
                    current_owner: owner.clone(),
                    requested_rig: target.rig_id,
                });
            }
        }

        let new_target = target.clone();
        self.active_tx = Some(ActiveTxTarget::new(target.clone()));
        let _ = self.event_tx.send(RoutedRigEvent::new(
            target.rig_id,
            target.receiver_id,
            RigHubEvent::ActiveTxChanged(Some(new_target)),
        ));

        Ok(())
    }

    /// Czyści aktywny cel odbiorczy (Active RX).
    pub fn clear_active_rx_target(&mut self) {
        self.active_rx = None;
        let _ = self.event_tx.send(RoutedRigEvent::new(
            RigId::hub(),
            None,
            RigHubEvent::ActiveRxChanged(None),
        ));
    }

    /// Czyści aktywny cel nadawczy (Active TX).
    ///
    /// # Błędy
    /// Zwraca błąd, jeżeli nadajnik jest aktualnie w stanie `Owned`.
    pub fn clear_active_tx_target(&mut self) -> Result<(), RigHubError> {
        if let Some(owner) = self.tx_arbiter.current_owner() {
            return Err(RigHubError::CannotChangeActiveTxWhileOwned {
                current_owner: owner.clone(),
                requested_rig: RigId::none(),
            });
        }
        self.active_tx = None;
        let _ = self.event_tx.send(RoutedRigEvent::new(
            RigId::hub(),
            None,
            RigHubEvent::ActiveTxChanged(None),
        ));
        Ok(())
    }

    /// Referencja do aktualnego celu Active RX.
    pub fn active_rx_target(&self) -> Option<&ActiveRxTarget> {
        self.active_rx.as_ref()
    }

    /// Referencja do aktualnego celu Active TX.
    pub fn active_tx_target(&self) -> Option<&ActiveTxTarget> {
        self.active_tx.as_ref()
    }

    /// Referencja do aktualnego stanu arbitra TX.
    pub fn tx_arbiter(&self) -> &TxArbiter {
        &self.tx_arbiter
    }

    /// Mutowalna referencja do arbitra TX (np. dla zaawansowanego sterowania lub testów).
    pub fn tx_arbiter_mut(&mut self) -> &mut TxArbiter {
        &mut self.tx_arbiter
    }

    /// Mutowalna referencja do arbitra TX w celach testowych.
    pub fn tx_arbiter_mut_for_test(&mut self) -> &mut TxArbiter {
        &mut self.tx_arbiter
    }

    /// Zwraca subskrypcję globalnego strumienia zdarzeń huba.
    pub fn subscribe_events(&self) -> broadcast::Receiver<RoutedRigEvent> {
        self.event_tx.subscribe()
    }

    /// Deterministyczne wykonanie polecenia zaadresowanego z uwzględnieniem podanego timeoutu.
    pub async fn execute_routed(
        &mut self,
        cmd: RoutedRigCommand,
        timeout: Duration,
    ) -> Result<(), RigHubError> {
        let mut cmd = cmd;
        cmd.deadline = tokio::time::Instant::now() + timeout;
        self.route_command(cmd).await
    }

    /// Wykonuje polecenie na radiu wskazanym przez [`ActiveRxTarget`].
    pub async fn execute_on_active_rx(
        &mut self,
        command: RigCommand,
        timeout: Duration,
    ) -> Result<(), RigHubError> {
        let target = self
            .active_rx
            .as_ref()
            .map(|t| t.target().clone())
            .ok_or(RigHubError::ActiveRxNotConfigured)?;

        let command_id = self.next_command_id();
        let deadline = tokio::time::Instant::now() + timeout;
        let routed = RoutedRigCommand::new(command_id, target, command, deadline);
        self.route_command(routed).await
    }

    /// Wykonuje polecenie na radiu wskazanym przez [`ActiveTxTarget`].
    pub async fn execute_on_active_tx(
        &mut self,
        command: RigCommand,
        timeout: Duration,
    ) -> Result<(), RigHubError> {
        let target = self
            .active_tx
            .as_ref()
            .map(|t| t.target().clone())
            .ok_or(RigHubError::ActiveTxNotConfigured)?;

        let command_id = self.next_command_id();
        let deadline = tokio::time::Instant::now() + timeout;
        let routed = RoutedRigCommand::new(command_id, target, command, deadline);
        self.route_command(routed).await
    }

    /// Deterministyczny routing polecenia do wskazanej sesji radiowej z uwzględnieniem arbitrażu TX.
    ///
    /// # Zasady routingu:
    /// - Brak niejawnego fallbacku na pierwsze radio w mapie.
    /// - Brak kierowania komend nadawczych na cel odbiorczy i na odwrót.
    /// - Komenda `SetPtt(true)` przechodzi dwufazową autoryzację w [`TxArbiter`].
    /// - W przypadku błędu lub timeoutu PTT arbiter przechodzi w [`TxArbiterState::Unknown`] bez automatycznego retry.
    pub async fn route_command(&mut self, cmd: RoutedRigCommand) -> Result<(), RigHubError> {
        if self.shutting_down {
            return Err(RigHubError::HubShuttingDown);
        }

        let target_rig = cmd.target.rig_id.clone();
        let handle = self
            .services
            .get(&target_rig)
            .ok_or_else(|| RigHubError::RigNotFound(target_rig.clone()))?;

        // 1. Walidacja odbiornika i unikanie błędnych aliasów VFO/Slice
        if let Some(ref rx) = cmd.target.receiver_id {
            let snap = handle.snapshot();
            if rx.is_slice() && !snap.capabilities.split && snap.capabilities.vfo {
                // Tradycyjny backend VFO jednokanałowy nie obsługuje slice
                return Err(RigHubError::Routing(RoutingError::Unsupported {
                    rig_id: target_rig,
                    operation: format!(
                        "odbiornik slice '{rx}' nie jest obsługiwany przez tradycyjny backend"
                    ),
                }));
            }
        }

        // 2. Obsługa arbitrażu nadawania PTT
        match cmd.command {
            RigCommand::SetPtt(true) => {
                // Krok 1: Weryfikacja arbitrażu (Idle -> Requesting)
                self.tx_arbiter
                    .begin_request(target_rig.clone(), cmd.command_id)?;
                let _ = self.event_tx.send(RoutedRigEvent::new(
                    target_rig.clone(),
                    cmd.target.receiver_id.clone(),
                    RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                ));

                // Krok 2: Wysłanie dokładnie jednej komendy do wybranego backendu
                let exec_res = handle
                    .execute(
                        cmd.command_id,
                        cmd.target.clone(),
                        RigCommand::SetPtt(true),
                        cmd.deadline,
                    )
                    .await;

                match exec_res {
                    Ok(()) => {
                        // Krok 3a: Weryfikacja czy stan po komendzie to fizycznie potwierdzony stan TX
                        let snap = handle.snapshot();
                        let confirmed_tx = snap.state.as_ref().map_or(false, |s| s.ptt);
                        if confirmed_tx {
                            self.tx_arbiter.apply_ptt_outcome(
                                &target_rig,
                                cmd.command_id,
                                PttCommandOutcome::ConfirmedTx,
                            )?;
                            let _ = self.event_tx.send(RoutedRigEvent::new(
                                target_rig.clone(),
                                cmd.target.receiver_id,
                                RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                            ));
                            Ok(())
                        } else {
                            // Wysłanie komendy bez fizycznego potwierdzenia TX (Unconfirmed) prowadzi do stanu Unknown!
                            let _ = self.tx_arbiter.apply_ptt_outcome(
                                &target_rig,
                                cmd.command_id,
                                PttCommandOutcome::Unconfirmed,
                            );
                            let _ = self.event_tx.send(RoutedRigEvent::new(
                                target_rig.clone(),
                                cmd.target.receiver_id,
                                RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                            ));
                            Err(RigHubError::TxArbiter(TxArbiterError::StateUnknown {
                                reason: TxUnknownReason::UnconfirmedState,
                            }))
                        }
                    }
                    Err(RigServiceError::QueueFull) => {
                        // Przepełnienie kolejki podczas próby SetPtt(true) cofa żądanie i wraca bezpiecznie do Idle
                        let _ = self.tx_arbiter.apply_ptt_outcome(
                            &target_rig,
                            cmd.command_id,
                            PttCommandOutcome::Rejected,
                        );
                        let _ = self.event_tx.send(RoutedRigEvent::new(
                            target_rig,
                            cmd.target.receiver_id,
                            RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                        ));
                        Err(RigHubError::Service(RigServiceError::QueueFull))
                    }
                    Err(RigServiceError::DeadlineExceeded) => {
                        // Krok 3b: Timeout prowadzi do bezpiecznego stanu Unknown (brak retry!)
                        self.tx_arbiter
                            .set_unknown(Some(target_rig.clone()), TxUnknownReason::Timeout);
                        let _ = self.event_tx.send(RoutedRigEvent::new(
                            target_rig,
                            cmd.target.receiver_id,
                            RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                        ));
                        Err(RigHubError::Service(RigServiceError::DeadlineExceeded))
                    }
                    Err(e) => {
                        // Krok 3c: Błąd wykonania PTT pozostawia niejednoznaczny stan nadajnika -> Unknown (brak retry!)
                        self.tx_arbiter.set_unknown(
                            Some(target_rig.clone()),
                            TxUnknownReason::BackendError(e.to_string()),
                        );
                        let _ = self.event_tx.send(RoutedRigEvent::new(
                            target_rig,
                            cmd.target.receiver_id,
                            RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                        ));
                        Err(RigHubError::Service(e))
                    }
                }
            }
            RigCommand::SetPtt(false) => {
                // Krok 1: Weryfikacja zwalniania (Owned -> Releasing)
                self.tx_arbiter.begin_release(&target_rig, cmd.command_id)?;
                let _ = self.event_tx.send(RoutedRigEvent::new(
                    target_rig.clone(),
                    cmd.target.receiver_id.clone(),
                    RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                ));

                // Krok 2: Wysłanie dokładnie jednej komendy wyłączenia PTT do właściciela
                let exec_res = handle
                    .execute(
                        cmd.command_id,
                        cmd.target.clone(),
                        RigCommand::SetPtt(false),
                        cmd.deadline,
                    )
                    .await;

                match exec_res {
                    Ok(()) => {
                        // Krok 3a: Weryfikacja czy stan po wyłączeniu to potwierdzony stan RX
                        let snap = handle.snapshot();
                        let confirmed_rx = snap.state.as_ref().map_or(false, |s| !s.ptt);
                        if confirmed_rx {
                            self.tx_arbiter.apply_ptt_outcome(
                                &target_rig,
                                cmd.command_id,
                                PttCommandOutcome::ConfirmedRx,
                            )?;
                            let _ = self.event_tx.send(RoutedRigEvent::new(
                                target_rig.clone(),
                                cmd.target.receiver_id,
                                RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                            ));
                            Ok(())
                        } else {
                            // Zwolnienie bez potwierdzonego RX prowadzi do Unknown!
                            let _ = self.tx_arbiter.apply_ptt_outcome(
                                &target_rig,
                                cmd.command_id,
                                PttCommandOutcome::Unconfirmed,
                            );
                            let _ = self.event_tx.send(RoutedRigEvent::new(
                                target_rig.clone(),
                                cmd.target.receiver_id,
                                RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                            ));
                            Err(RigHubError::TxArbiter(TxArbiterError::StateUnknown {
                                reason: TxUnknownReason::UnconfirmedState,
                            }))
                        }
                    }
                    Err(RigServiceError::QueueFull) => {
                        // Przepełnienie kolejki przy wyłączaniu PTT uniemożliwia gwarancję wyłączenia -> Unknown!
                        self.tx_arbiter.set_unknown(
                            Some(target_rig.clone()),
                            TxUnknownReason::BackendError(
                                "Kolejka przepełniona podczas zwalniania PTT".to_string(),
                            ),
                        );
                        let _ = self.event_tx.send(RoutedRigEvent::new(
                            target_rig,
                            cmd.target.receiver_id,
                            RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                        ));
                        Err(RigHubError::Service(RigServiceError::QueueFull))
                    }
                    Err(e) => {
                        // Krok 3b: Błąd podczas zwalniania prowadzi do Unknown
                        self.tx_arbiter
                            .set_unknown(Some(target_rig.clone()), TxUnknownReason::ReleaseTimeout);
                        let _ = self.event_tx.send(RoutedRigEvent::new(
                            target_rig,
                            cmd.target.receiver_id,
                            RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
                        ));
                        Err(RigHubError::Service(e))
                    }
                }
            }
            other_cmd => {
                // Komendy niemutujące PTT są przesyłane bezpośrednio do sesji
                handle
                    .execute(cmd.command_id, cmd.target, other_cmd, cmd.deadline)
                    .await
                    .map_err(RigHubError::Service)
            }
        }
    }

    /// Bezpieczna resynchronizacja arbitra TX ze stanu [`TxArbiterState::Unknown`] do [`TxArbiterState::Idle`].
    ///
    /// Weryfikuje bieżącą telemetrię wskazanego radia. Jeśli radio potwierdza stan RX (`ptt == false`)
    /// ze świeżym znacznikiem czasu wykonanym po wejściu w stan Unknown, stan `Unknown` zostaje skasowany.
    /// W przeciwnym razie blokada pozostaje nienaruszona.
    pub async fn resync_tx_arbiter(
        &mut self,
        rig_id: &RigId,
        _timeout: Duration,
    ) -> Result<(), RigHubError> {
        let handle = self
            .services
            .get(rig_id)
            .ok_or_else(|| RigHubError::RigNotFound(rig_id.clone()))?;

        let snapshot = handle.snapshot();
        if snapshot.lifecycle != RigServiceState::Ready {
            return Err(RigHubError::TxArbiter(TxArbiterError::ResyncFailed(
                format!("radio '{rig_id}' nie jest w stanie Ready"),
            )));
        }

        if snapshot.freshness != crate::cat::service::StateFreshness::Fresh {
            return Err(RigHubError::TxArbiter(TxArbiterError::ResyncFailed(
                format!(
                    "telemetria radia '{rig_id}' nie jest świeża (freshness: {:?})",
                    snapshot.freshness
                ),
            )));
        }

        let is_rx = snapshot.state.as_ref().map(|s| !s.ptt).unwrap_or(false);
        if !is_rx {
            return Err(RigHubError::TxArbiter(TxArbiterError::ResyncFailed(
                format!("telemetria radia '{rig_id}' nie potwierdza stanu RX"),
            )));
        }

        let update_time = snapshot.last_update.ok_or_else(|| {
            RigHubError::TxArbiter(TxArbiterError::ResyncFailed(
                "brak znacznika czasu aktualizacji telemetrii".to_string(),
            ))
        })?;

        // Asynchroniczne ustąpienie wykonania usuwa ostrzeżenie clippy unused_async
        tokio::task::yield_now().await;

        self.tx_arbiter
            .resync_with_fresh_audit(rig_id, true, Some(update_time))?;
        let _ = self.event_tx.send(RoutedRigEvent::new(
            rig_id.clone(),
            None,
            RigHubEvent::TxArbiterStateChanged(self.tx_arbiter.state().clone()),
        ));

        Ok(())
    }

    /// Pobiera spójny logicznie snapshot całego węzła [`RigHub`] i wszystkich sesji.
    pub fn snapshot(&self) -> RigHubSnapshot {
        let mut rigs = BTreeMap::new();
        for (id, handle) in &self.services {
            rigs.insert(id.clone(), handle.snapshot());
        }

        RigHubSnapshot {
            rigs,
            active_rx: self.active_rx.as_ref().map(|t| t.target().clone()),
            active_tx: self.active_tx.as_ref().map(|t| t.target().clone()),
            tx_state: self.tx_arbiter.state().clone(),
            shutting_down: self.shutting_down,
        }
    }

    /// Kontrolowane, asynchroniczne i idempotentne zatrzymanie całego [`RigHub`].
    ///
    /// # Procedura:
    /// 1. Ustawia flagę `shutting_down` i odrzuca nowe polecenia.
    /// 2. Sprawdza, czy którekolwiek radio nadaje (`Owned`). Jeśli tak, wykonuje JEDNĄ bounded próbę wyłączenia PTT.
    /// 3. Wysyła sygnał shutdown do wszystkich zarejestrowanych sesji radiowych.
    /// 4. Oczekuje na zakończenie wszystkich tasków (`JoinHandle`) z zachowaniem jednego bezwzględnego `deadline`.
    /// 5. Czyści rejestr sesji, cele focusu i oczekujące żądania.
    pub async fn shutdown(&mut self, timeout: Duration) -> Result<(), RigHubError> {
        if self.shutting_down && self.services.is_empty() {
            return Ok(());
        }

        self.shutting_down = true;
        let deadline = tokio::time::Instant::now() + timeout;

        // 1. Bezpieczne zwolnienie PTT wyłącznie dla właściciela (jeśli istnieje)
        if let Some(owner) = self.tx_arbiter.shutdown_release_target() {
            if let Some(handle) = self.services.get(&owner) {
                let now = tokio::time::Instant::now();
                let release_deadline = if deadline > now {
                    now + (deadline - now).min(Duration::from_millis(1000))
                } else {
                    now
                };
                let cmd_id = self.next_command_id();
                let _ = handle
                    .execute(
                        cmd_id,
                        RigTarget::rig_only(owner.clone()),
                        RigCommand::SetPtt(false),
                        release_deadline,
                    )
                    .await;
            }
        }

        // 2. Wyślij shutdown i dołącz (join) wszystkie taski sesji z zachowaniem jednego bezwzględnego deadline
        let mut shutdown_errors = Vec::new();
        let service_ids: Vec<RigId> = self.services.keys().cloned().collect();

        for id in service_ids {
            if let Some(mut handle) = self.services.remove(&id) {
                if let Err(e) = handle.shutdown(deadline).await {
                    log::warn!("[{id}] Błąd podczas shutdown sesji: {e}");
                    shutdown_errors.push(format!("{id}: {e}"));
                }
            }
        }

        // 3. Wyczyść cele, rejestr poleceń i stan
        self.active_rx = None;
        self.active_tx = None;
        self.pending_commands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();

        if shutdown_errors.is_empty() {
            Ok(())
        } else {
            Err(RigHubError::ShutdownError(shutdown_errors.join("; ")))
        }
    }
}
