// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Komponent aplikacyjny zarządzający węzłem wieloradiowym ([`CatRuntime`]) (NATIVE-6B).
//!
//! Odpowiada za:
//! - Koordynację cyklu życia sesji radiowych w aplikacji,
//! - Powoływanie backendów przez [`crate::cat::factory::RigBackendFactory`],
//! - Nadzór nad taskami aktorów [`crate::cat::service::RigService`],
//! - Bieżące publikowanie spójnego snapshotu ([`RigHubSnapshot`]) dla GUI bez blokowania renderera,
//! - Bezpieczne i asynchroniczne delegowanie poleceń domenowych,
//! - Kontrolowany, deterministyczny i idempotentny shutdown.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::broadcast;

use crate::cat::config::{CatConfigValidationError, CatStationConfig};
use crate::cat::factory::{FactoryError, RigBackendFactory};
use crate::cat::hub::{RigHub, RigHubError, RigHubSnapshot};
use crate::cat::rig::RigCommand;
use crate::cat::routing::{RigId, RigTarget, RoutedRigEvent};
use crate::cat::service::RigService;

/// Błędy środowiska wykonawczego CAT ([`CatRuntime`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatRuntimeError {
    /// Błąd walidacji konfiguracji stacji CAT.
    Validation(CatConfigValidationError),
    /// Błąd fabryki przy tworzeniu backendu.
    Factory(FactoryError),
    /// Błąd węzła [`RigHub`].
    Hub(RigHubError),
    /// Środowisko wykonawcze jest w trakcie zamykania lub zostało już zamknięte.
    ShuttingDown,
    /// Przekroczono limit czasu operacji runtime.
    Timeout,
}

impl fmt::Display for CatRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "błąd konfiguracji CAT: {e}"),
            Self::Factory(e) => write!(f, "błąd fabryki backendów: {e}"),
            Self::Hub(e) => write!(f, "błąd węzła RigHub: {e}"),
            Self::ShuttingDown => write!(f, "runtime CAT jest w trakcie wyłączania"),
            Self::Timeout => write!(f, "przekroczono limit czasu operacji CAT"),
        }
    }
}

impl std::error::Error for CatRuntimeError {}

impl From<CatConfigValidationError> for CatRuntimeError {
    fn from(e: CatConfigValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<FactoryError> for CatRuntimeError {
    fn from(e: FactoryError) -> Self {
        Self::Factory(e)
    }
}

impl From<RigHubError> for CatRuntimeError {
    fn from(e: RigHubError) -> Self {
        Self::Hub(e)
    }
}

/// Centralny komponent aplikacyjny zarządzający warstwą wykonawczą CAT.
#[derive(Clone)]
pub struct CatRuntime {
    hub: Arc<TokioMutex<RigHub>>,
    snapshot: Arc<StdRwLock<RigHubSnapshot>>,
    event_tx: broadcast::Sender<RoutedRigEvent>,
    shutting_down: Arc<AtomicBool>,
}

impl Default for CatRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl CatRuntime {
    /// Tworzy nowe, puste środowisko wykonawcze CAT z pustym węzłem [`RigHub`].
    pub fn new() -> Self {
        let hub = RigHub::new();
        let initial_snapshot = hub.snapshot();
        let (event_tx, _) = broadcast::channel(crate::cat::hub::MAX_HUB_EVENT_QUEUE);

        Self {
            hub: Arc::new(TokioMutex::new(hub)),
            snapshot: Arc::new(StdRwLock::new(initial_snapshot)),
            event_tx,
            shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Tworzy środowisko wykonawcze z istniejącym węzłem [`RigHub`].
    pub fn with_hub(hub: RigHub) -> Self {
        let initial_snapshot = hub.snapshot();
        let event_rx = hub.subscribe_events();
        let (event_tx, _) = broadcast::channel(crate::cat::hub::MAX_HUB_EVENT_QUEUE);

        let runtime = Self {
            hub: Arc::new(TokioMutex::new(hub)),
            snapshot: Arc::new(StdRwLock::new(initial_snapshot)),
            event_tx,
            shutting_down: Arc::new(AtomicBool::new(false)),
        };

        runtime.spawn_sync_loop(event_rx);
        runtime
    }

    /// Tworzy i inicjalizuje środowisko wykonawcze na podstawie konfiguracji stacji [`CatStationConfig`].
    ///
    /// # Procedura startowa:
    /// 1. Weryfikuje konfigurację.
    /// 2. Tworzy instancję [`RigHub`].
    /// 3. Dla każdego aktywnego radia tworzy backend przez [`RigBackendFactory`].
    /// 4. Uruchamia autonomiczną pętlę aktora [`RigService`] i wiąże `JoinHandle`.
    /// 5. Rejestruje usługi w hubie.
    /// 6. Odtwarza Active RX.
    /// 7. Odtwarza Active TX wyłącznie po walidacji i w stanie `Idle`.
    /// 8. Publikuje początkowy snapshot.
    pub async fn start(config: &CatStationConfig) -> Result<Self, CatRuntimeError> {
        config.validate()?;

        let mut hub = RigHub::new();

        for rig_cfg in &config.rigs {
            if !rig_cfg.enabled {
                continue;
            }

            let backend = RigBackendFactory::create(rig_cfg)?;
            let poll_dur = Duration::from_millis(rig_cfg.poll_interval_ms);
            let (service, mut handle) = RigService::spawn(rig_cfg.id.clone(), backend, poll_dur);

            let worker_jh = tokio::spawn(async move { service.run().await });
            handle.attach_join_handle(worker_jh);

            hub.register_service(handle)?;
        }

        // Odtwarzanie celów
        if let Some(ref rx) = config.active_rx {
            if hub.contains_rig(&rx.rig_id) {
                let target = RigTarget {
                    rig_id: rx.rig_id.clone(),
                    receiver_id: rx.receiver_id.clone(),
                };
                let _ = hub.set_active_rx_target(target);
            }
        }

        if let Some(ref tx) = config.active_tx {
            if hub.contains_rig(&tx.rig_id) {
                let target = RigTarget {
                    rig_id: tx.rig_id.clone(),
                    receiver_id: tx.receiver_id.clone(),
                };
                let _ = hub.set_active_tx_target(target);
            }
        }

        let initial_snapshot = hub.snapshot();
        let event_rx = hub.subscribe_events();
        let (event_tx, _) = broadcast::channel(crate::cat::hub::MAX_HUB_EVENT_QUEUE);

        let runtime = Self {
            hub: Arc::new(TokioMutex::new(hub)),
            snapshot: Arc::new(StdRwLock::new(initial_snapshot)),
            event_tx: event_tx.clone(),
            shutting_down: Arc::new(AtomicBool::new(false)),
        };

        // Uruchomienie pętli synchronizującej snapshot i zdarzenia
        runtime.spawn_sync_loop(event_rx);

        Ok(runtime)
    }

    /// Pobiera najświeższy snapshot węzła [`RigHub`] i wszystkich sesji.
    ///
    /// Jest to operacja natychmiastowa i nieblokująca, bezpieczna do wywołania
    /// w każdej klatce renderera GUI (np. w `ui()`).
    pub fn snapshot(&self) -> RigHubSnapshot {
        self.snapshot
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Zwraca subskrypcję zdarzeń emitowanych przez sesje radiowe i hub.
    pub fn subscribe_events(&self) -> broadcast::Receiver<RoutedRigEvent> {
        self.event_tx.subscribe()
    }

    /// Zwraca referencję do współdzielonego muteksu [`RigHub`] (dla zaawansowanych operacji).
    pub fn hub(&self) -> &Arc<TokioMutex<RigHub>> {
        &self.hub
    }

    /// Uruchamia pętlę synchronizacji stanu snapshotu i retransmisji zdarzeń.
    fn spawn_sync_loop(&self, mut hub_event_rx: broadcast::Receiver<RoutedRigEvent>) {
        let snapshot_cache = Arc::clone(&self.snapshot);
        let hub_ref = Arc::clone(&self.hub);
        let event_tx = self.event_tx.clone();
        let shutting_down = Arc::clone(&self.shutting_down);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(100));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            while !shutting_down.load(Ordering::Relaxed) {
                tokio::select! {
                    event_res = hub_event_rx.recv() => {
                        match event_res {
                            Ok(event) => {
                                // Retransmituj zdarzenie do subskrybentów GUI
                                let _ = event_tx.send(event);
                                // Odśwież snapshot
                                let snap = {
                                    let hub_lock = hub_ref.lock().await;
                                    hub_lock.snapshot()
                                };
                                *snapshot_cache.write().unwrap_or_else(|e| e.into_inner()) = snap;
                            }
                            Err(broadcast::error::RecvError::Lagged(dropped)) => {
                                log::warn!("[CatRuntime] Wykryto opóźnienie w kolejce zdarzeń (utracono {dropped}), pobieranie pełnego snapshotu");
                                let snap = {
                                    let hub_lock = hub_ref.lock().await;
                                    hub_lock.snapshot()
                                };
                                *snapshot_cache.write().unwrap_or_else(|e| e.into_inner()) = snap;
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                break;
                            }
                        }
                    }
                    _ = ticker.tick() => {
                        let snap = {
                            let mut hub_lock = hub_ref.lock().await;
                            hub_lock.poll_service_health();
                            hub_lock.snapshot()
                        };
                        *snapshot_cache.write().unwrap_or_else(|e| e.into_inner()) = snap;
                    }
                }
            }
        });
    }

    /// Asynchronicznie przesyła polecenie zmiany częstotliwości do aktywnego celu odbiorczego.
    pub fn set_frequency(&self, freq_hz: u64) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            let cmd = RigCommand::SetFrequency(freq_hz);
            if let Err(e) = hub
                .execute_on_active_rx(cmd, Duration::from_millis(1500))
                .await
            {
                log::warn!("[CatRuntime] Błąd przestawienia częstotliwości: {e}");
            }
        });
    }

    /// Asynchronicznie przesyła polecenie zmiany emisji i szerokości pasma do aktywnego celu odbiorczego.
    pub fn set_mode(&self, mode: String, passband_hz: u32) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            let cmd = RigCommand::SetMode { mode, passband_hz };
            if let Err(e) = hub
                .execute_on_active_rx(cmd, Duration::from_millis(1500))
                .await
            {
                log::warn!("[CatRuntime] Błąd przestawienia modulacji: {e}");
            }
        });
    }

    /// Asynchronicznie przesyła polecenie trybu Split do aktywnego celu odbiorczego.
    pub fn set_split(&self, enabled: bool, tx_vfo: String) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            let cmd = RigCommand::SetSplit { enabled, tx_vfo };
            if let Err(e) = hub
                .execute_on_active_rx(cmd, Duration::from_millis(1500))
                .await
            {
                log::warn!("[CatRuntime] Błąd przestawienia split: {e}");
            }
        });
    }

    /// Asynchronicznie przesyła polecenie wyboru VFO do aktywnego celu odbiorczego.
    pub fn set_vfo(&self, vfo: String) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            let cmd = RigCommand::SetVfo(vfo);
            if let Err(e) = hub
                .execute_on_active_rx(cmd, Duration::from_millis(1500))
                .await
            {
                log::warn!("[CatRuntime] Błąd przestawienia VFO: {e}");
            }
        });
    }

    /// Asynchronicznie przesyła dowolne polecenie [`RigCommand`] do aktywnego celu odbiorczego.
    pub fn execute_cmd(&self, cmd: RigCommand) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            if let Err(e) = hub
                .execute_on_active_rx(cmd, Duration::from_millis(1500))
                .await
            {
                log::warn!("[CatRuntime] Błąd wykonania polecenia CAT: {e}");
            }
        });
    }

    /// Bezpiecznie żąda załączenia nadawania (PTT on) z pełną weryfikacją w [`crate::cat::tx_arbiter::TxArbiter`].
    pub fn request_tx(&self, target_rig: RigId, timeout: Duration) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            if let Err(e) = hub.request_tx(&target_rig, timeout).await {
                log::warn!("[CatRuntime] Odrzucono załączenie TX na '{target_rig}': {e}");
            }
        });
    }

    /// Bezpiecznie żąda wyłączenia nadawania (PTT off) ze zwolnieniem blokady w arbitralnym zarządcy TX.
    pub fn release_tx(&self, target_rig: RigId, timeout: Duration) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            if let Err(e) = hub.release_tx(&target_rig, timeout).await {
                log::warn!("[CatRuntime] Błąd wyłączenia TX na '{target_rig}': {e}");
            }
        });
    }

    /// Bezpieczna resynchronizacja arbitra nadawania po wejściu w stan `Unknown`.
    pub fn resync_tx(&self, rig_id: RigId, timeout: Duration) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            if let Err(e) = hub.resync_tx_arbiter(&rig_id, timeout).await {
                log::warn!("[CatRuntime] Resynchronizacja TX dla '{rig_id}' nie powiodła się: {e}");
            }
        });
    }

    /// Ustawia aktywny cel odbiorczy (Active RX).
    pub fn set_active_rx(&self, target: RigTarget) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            if let Err(e) = hub.set_active_rx_target(target) {
                log::warn!("[CatRuntime] Błąd ustawienia Active RX: {e}");
            }
        });
    }

    /// Ustawia aktywny cel nadawczy (Active TX).
    pub fn set_active_tx(&self, target: RigTarget) {
        let hub_ref = Arc::clone(&self.hub);
        tokio::spawn(async move {
            let mut hub = hub_ref.lock().await;
            if let Err(e) = hub.set_active_tx_target(target) {
                log::warn!("[CatRuntime] Błąd ustawienia Active TX: {e}");
            }
        });
    }

    /// Aktualizuje działającą stację na podstawie nowej konfiguracji [`CatStationConfig`].
    pub async fn apply_config(&mut self, config: &CatStationConfig) -> Result<(), CatRuntimeError> {
        config.validate()?;

        // 1. Zatrzymanie bieżących sesji w hubie
        self.shutdown(Duration::from_millis(2000)).await?;

        // 2. Start nowego huba i usług
        let new_runtime = Self::start(config).await?;
        *self = new_runtime;
        Ok(())
    }

    /// Kontrolowany, asynchroniczny shutdown całego środowiska wykonawczego.
    pub async fn shutdown(&mut self, timeout: Duration) -> Result<(), CatRuntimeError> {
        self.shutting_down.store(true, Ordering::SeqCst);

        let mut hub = self.hub.lock().await;
        hub.shutdown(timeout).await?;

        // Zaktualizuj snapshot na stan końcowy
        *self.snapshot.write().unwrap_or_else(|e| e.into_inner()) = hub.snapshot();
        Ok(())
    }

    /// Synchroniczne opakowanie metody shutdown do wywołania podczas zamykania aplikacji (np. `on_exit`).
    pub fn shutdown_blocking(&mut self, timeout: Duration) {
        self.shutting_down.store(true, Ordering::SeqCst);
        let mut self_clone = self.clone();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let _ = tokio::task::block_in_place(|| {
                handle.block_on(async move {
                    let _ = self_clone.shutdown(timeout).await;
                })
            });
        }
    }
}
