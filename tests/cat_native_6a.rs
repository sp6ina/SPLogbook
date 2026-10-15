// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Zestaw kanonicznych testów jednostkowych i integracyjnych etapu NATIVE-6A:
//! Multi-Radio Execution Core, RigHub i bezpieczny arbitraż TX.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use splogbook::cat::backend::{BoxFuture, CatBackendKind, RigBackend};
use splogbook::cat::hub::{MAX_HUB_EVENT_QUEUE, RigHub, RigHubError};
use splogbook::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};
use splogbook::cat::routing::{
    ActiveRxTarget, ActiveTxTarget, ReceiverId, RigCommandId, RigCommandIdError, RigHubEvent,
    RigId, RigIdError, RigTarget, RoutedRigCommand,
};
use splogbook::cat::service::{
    MAX_RIG_COMMAND_QUEUE, MAX_RIG_EVENT_QUEUE, RigService, RigServiceError, RigServiceRequest,
    RigServiceState,
};
use splogbook::cat::tx_arbiter::{
    PttCommandOutcome, TxArbiter, TxArbiterError, TxArbiterState, TxUnknownReason,
    redact_sensitive_info,
};

// ============================================================================
// ETAP 35: Deterministyczny MockRigBackend
// ============================================================================

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct MockBackendState {
    pub connected: Arc<AtomicBool>,
    pub connect_count: Arc<AtomicUsize>,
    pub disconnect_count: Arc<AtomicUsize>,
    pub commands: Arc<StdMutex<Vec<RigCommand>>>,
    pub routed_commands: Arc<StdMutex<Vec<(Option<ReceiverId>, RigCommand)>>>,
    pub fail_connect: Arc<AtomicBool>,
    pub fail_execute: Arc<AtomicBool>,
    pub execute_delay: Arc<StdMutex<Option<Duration>>>,
    pub capabilities: Arc<StdMutex<RigCapabilities>>,
    pub rig_state: Arc<StdMutex<RigState>>,
}

impl Default for MockBackendState {
    fn default() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            connect_count: Arc::new(AtomicUsize::new(0)),
            disconnect_count: Arc::new(AtomicUsize::new(0)),
            commands: Arc::new(StdMutex::new(Vec::new())),
            routed_commands: Arc::new(StdMutex::new(Vec::new())),
            fail_connect: Arc::new(AtomicBool::new(false)),
            fail_execute: Arc::new(AtomicBool::new(false)),
            execute_delay: Arc::new(StdMutex::new(None)),
            capabilities: Arc::new(StdMutex::new(RigCapabilities::all())),
            rig_state: Arc::new(StdMutex::new(RigState::default())),
        }
    }
}

pub struct MockRigBackend {
    pub kind: CatBackendKind,
    pub shared: MockBackendState,
}

impl MockRigBackend {
    pub fn new(kind: CatBackendKind, shared: MockBackendState) -> Self {
        Self { kind, shared }
    }
}

impl RigBackend for MockRigBackend {
    fn kind(&self) -> CatBackendKind {
        self.kind
    }

    fn capabilities(&self) -> RigCapabilities {
        *self.shared.capabilities.lock().unwrap()
    }

    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        Box::pin(async move {
            self.shared.connect_count.fetch_add(1, Ordering::SeqCst);
            if self.shared.fail_connect.load(Ordering::SeqCst) {
                return Err(RigError::Connect {
                    endpoint: "mock://rig".to_string(),
                    detail: "symulowany błąd połączenia".to_string(),
                });
            }
            self.shared.connected.store(true, Ordering::SeqCst);
            let mut state = self.shared.rig_state.lock().unwrap();
            state.connected = true;
            Ok(())
        })
    }

    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        Box::pin(async move {
            if !self.shared.connected.load(Ordering::SeqCst) {
                return Err(RigError::NotConnected);
            }
            Ok(self.shared.rig_state.lock().unwrap().clone())
        })
    }

    fn execute<'a>(&'a mut self, command: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        Box::pin(async move {
            if !self.shared.connected.load(Ordering::SeqCst) {
                return Err(RigError::NotConnected);
            }
            let delay = *self.shared.execute_delay.lock().unwrap();
            if let Some(d) = delay {
                tokio::time::sleep(d).await;
            }
            self.shared.commands.lock().unwrap().push(command.clone());
            if self.shared.fail_execute.load(Ordering::SeqCst) {
                return Err(RigError::Protocol {
                    detail: "symulowany błąd wykonania komendy".to_string(),
                });
            }
            match command {
                RigCommand::SetFrequency(hz) => {
                    self.shared.rig_state.lock().unwrap().frequency_hz = *hz;
                }
                RigCommand::SetMode { mode, .. } => {
                    self.shared.rig_state.lock().unwrap().mode = mode.to_string();
                }
                RigCommand::SetPtt(tx) => {
                    self.shared.rig_state.lock().unwrap().ptt = *tx;
                }
                _ => {}
            }
            Ok(())
        })
    }

    fn execute_routed<'a>(
        &'a mut self,
        receiver: Option<&'a ReceiverId>,
        command: &'a RigCommand,
    ) -> BoxFuture<'a, Result<(), RigError>> {
        self.shared
            .routed_commands
            .lock()
            .unwrap()
            .push((receiver.cloned(), command.clone()));
        self.execute(command)
    }

    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.shared.disconnect_count.fetch_add(1, Ordering::SeqCst);
            self.shared.connected.store(false, Ordering::SeqCst);
            let mut state = self.shared.rig_state.lock().unwrap();
            state.connected = false;
        })
    }
}

// ============================================================================
// ETAP 28: Testy silnie typowanych identyfikatorów
// ============================================================================

#[test]
fn test_rig_id_rejects_empty() {
    assert_eq!(RigId::new(""), Err(RigIdError::Empty));
}

#[test]
fn test_rig_id_rejects_whitespace_only() {
    assert_eq!(RigId::new("   "), Err(RigIdError::WhitespaceOnly));
    assert_eq!(RigId::new("\t\n\r"), Err(RigIdError::WhitespaceOnly));
}

#[test]
fn test_rig_id_rejects_control_characters() {
    assert_eq!(
        RigId::new("radio\x00one"),
        Err(RigIdError::ContainsControlCharacters)
    );
    assert_eq!(
        RigId::new("radio\x1btest"),
        Err(RigIdError::ContainsControlCharacters)
    );
}

#[test]
fn test_rig_id_accepts_valid_utf8_identifiers() {
    let rig = RigId::new("TS-890S_Primary").expect("valid rig_id");
    assert_eq!(rig.as_str(), "TS-890S_Primary");
    assert_eq!(rig.to_string(), "TS-890S_Primary");

    let rig2 = RigId::new("K4D-SO2R-Radio-1").expect("valid rig_id");
    assert_eq!(rig2.as_str(), "K4D-SO2R-Radio-1");
}

#[test]
fn test_rig_id_implements_required_traits_and_display() {
    let r1 = RigId::new("RADIO_A").unwrap();
    let r2 = RigId::new("RADIO_A").unwrap();
    let r3 = RigId::new("RADIO_B").unwrap();

    assert_eq!(r1, r2);
    assert_ne!(r1, r3);
    assert!(r1 < r3);
    assert_eq!(format!("{r1}"), "RADIO_A");

    let mut map = std::collections::BTreeMap::new();
    map.insert(r1.clone(), 42);
    assert_eq!(map.get(&r2), Some(&42));
}

#[test]
fn test_receiver_id_vfo_a_and_b() {
    let vfo_a = ReceiverId::vfo_a();
    let vfo_b = ReceiverId::vfo_b();

    assert!(vfo_a.is_vfo());
    assert!(vfo_b.is_vfo());
    assert!(!vfo_a.is_slice());
    assert!(!vfo_b.is_slice());

    assert_eq!(vfo_a.to_string(), "vfo_a");
    assert_eq!(vfo_b.to_string(), "vfo_b");
}

#[test]
fn test_receiver_id_flex_slice_preserves_raw_index() {
    let slice0 = ReceiverId::slice(0);
    let slice1 = ReceiverId::slice(1);
    let slice7 = ReceiverId::slice(7);

    assert!(slice0.is_slice());
    assert_eq!(slice0.slice_index(), Some(0));
    assert_eq!(slice1.slice_index(), Some(1));
    assert_eq!(slice7.slice_index(), Some(7));

    assert_eq!(slice0.to_string(), "slice_0");
    assert_eq!(slice7.to_string(), "slice_7");
}

#[test]
fn test_receiver_id_equality_and_ordering() {
    let a1 = ReceiverId::vfo_a();
    let a2 = ReceiverId::vfo_a();
    let b = ReceiverId::vfo_b();
    let s0 = ReceiverId::slice(0);
    let s1 = ReceiverId::slice(1);

    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    assert_ne!(a1, s0);
    assert_ne!(s0, s1);
    assert!(s0 < s1);
}

#[test]
fn test_rig_target_constructors() {
    let rig = RigId::new("RADIO_1").unwrap();
    let rx = ReceiverId::vfo_a();

    let t_entire = RigTarget::entire_rig(rig.clone());
    assert_eq!(t_entire.rig_id(), &rig);
    assert_eq!(t_entire.receiver_id(), None);
    assert!(t_entire.is_entire_rig());

    let t_rx = RigTarget::receiver(rig.clone(), rx.clone());
    assert_eq!(t_rx.rig_id(), &rig);
    assert_eq!(t_rx.receiver_id(), Some(&rx));
    assert!(!t_rx.is_entire_rig());
}

#[test]
fn test_rig_target_debug_and_display() {
    let rig = RigId::new("FLEX-6600").unwrap();
    let target1 = RigTarget::entire_rig(rig.clone());
    let target2 = RigTarget::receiver(rig, ReceiverId::slice(2));

    assert_eq!(target1.to_string(), "FLEX-6600");
    assert_eq!(target2.to_string(), "FLEX-6600:slice_2");
}

// ============================================================================
// ETAP 29: Testy targetów wykonawczych i zakazu fallbacku
// ============================================================================

#[test]
fn test_active_rx_target_points_to_explicit_receiver() {
    let target = RigTarget::receiver(RigId::new("RIG_A").unwrap(), ReceiverId::vfo_a());
    let active_rx = ActiveRxTarget::new(target.clone());
    assert_eq!(active_rx.target(), &target);
    assert_eq!(active_rx.into_target(), target);
}

#[test]
fn test_active_tx_target_points_to_explicit_receiver() {
    let target = RigTarget::receiver(RigId::new("RIG_B").unwrap(), ReceiverId::vfo_b());
    let active_tx = ActiveTxTarget::new(target.clone());
    assert_eq!(active_tx.target(), &target);
    assert_eq!(active_tx.into_target(), target);
}

#[tokio::test]
async fn test_active_rx_target_change_emits_event() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_1").unwrap();
    let target = RigTarget::receiver(rig.clone(), ReceiverId::vfo_a());

    let (s, h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    drop(s);
    hub.register_service(h).unwrap();

    let mut rx = hub.subscribe_events();
    hub.set_active_rx_target(target.clone())
        .expect("set active rx");

    assert_eq!(hub.active_rx_target().map(|t| t.target()), Some(&target));

    let ev = rx.try_recv().expect("event emitted");
    assert_eq!(ev.rig_id, *target.rig_id());
    assert_eq!(ev.event, RigHubEvent::ActiveRxChanged(Some(target)));
}

#[tokio::test]
async fn test_active_tx_target_change_emits_event() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_1").unwrap();
    let target = RigTarget::receiver(rig.clone(), ReceiverId::vfo_a());

    let (s, h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    drop(s);
    hub.register_service(h).unwrap();

    let mut rx = hub.subscribe_events();
    hub.set_active_tx_target(target.clone())
        .expect("set active tx");

    assert_eq!(hub.active_tx_target().map(|t| t.target()), Some(&target));

    let ev = rx.try_recv().expect("event emitted");
    assert_eq!(ev.rig_id, *target.rig_id());
    assert_eq!(ev.event, RigHubEvent::ActiveTxChanged(Some(target)));
}

#[tokio::test]
async fn test_active_tx_target_change_rejected_while_tx_owned() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_1").unwrap();
    let target1 = RigTarget::receiver(rig.clone(), ReceiverId::vfo_a());
    let target2 = RigTarget::receiver(rig.clone(), ReceiverId::vfo_b());

    let (s, h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    drop(s);
    hub.register_service(h).unwrap();

    hub.set_active_tx_target(target1).unwrap();

    // Inne radio jako właściciel
    let other_rig = RigId::new("OTHER_RIG").unwrap();
    hub.tx_arbiter_mut_for_test()
        .request_tx(&other_rig, RigCommandId::new(1))
        .unwrap();
    hub.tx_arbiter_mut_for_test()
        .confirm_tx(&other_rig, RigCommandId::new(1))
        .unwrap();

    let err = hub.set_active_tx_target(target2);
    assert_eq!(
        err,
        Err(RigHubError::CannotChangeActiveTxWhileOwned {
            current_owner: other_rig,
            requested_rig: rig,
        })
    );
}

#[tokio::test]
async fn test_no_fallback_to_primary_radio_on_missing_target() {
    let mut hub = RigHub::new();
    let cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(1),
        RigTarget::entire_rig(RigId::new("UNKNOWN_RIG").unwrap()),
        RigCommand::SetFrequency(14_074_000),
        Duration::from_millis(500),
    );

    let err = hub
        .execute_routed(cmd, Duration::from_millis(500))
        .await
        .unwrap_err();
    assert_eq!(
        err,
        RigHubError::RigNotFound(RigId::new("UNKNOWN_RIG").unwrap())
    );
}

#[tokio::test]
async fn test_execute_routed_command_requires_existing_rig_id() {
    let mut hub = RigHub::new();
    let non_existent_rig = RigId::new("NON_EXISTENT").unwrap();
    let cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(10),
        RigTarget::receiver(non_existent_rig.clone(), ReceiverId::vfo_a()),
        RigCommand::SetMode {
            mode: "USB".to_string(),
            passband_hz: 2400,
        },
        Duration::from_millis(100),
    );

    let err = hub
        .execute_routed(cmd, Duration::from_millis(100))
        .await
        .unwrap_err();
    assert_eq!(err, RigHubError::RigNotFound(non_existent_rig));
}

// ============================================================================
// ETAP 30: Testy RigService
// ============================================================================

#[tokio::test]
async fn test_service_lifecycle_initial_state_is_disconnected() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared));
    let rig_id = RigId::new("RIG_KW").unwrap();

    let (service, handle) = RigService::spawn(rig_id.clone(), backend, Duration::from_millis(50));
    assert_eq!(handle.snapshot().lifecycle, RigServiceState::Created);

    drop(service);
}

#[tokio::test]
async fn test_service_transition_to_connecting_and_ready() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::IcomCiV, shared));
    let rig_id = RigId::new("RIG_IC").unwrap();

    let (service, mut handle) =
        RigService::spawn(rig_id.clone(), backend, Duration::from_millis(50));
    let join_handle = tokio::spawn(service.run());
    handle.attach_join_handle(join_handle);

    let mut ready = false;
    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(ready, "service did not transition to Ready");

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

#[tokio::test]
async fn test_service_rejects_command_when_disconnected() {
    let shared = MockBackendState::default();
    shared.fail_connect.store(true, Ordering::SeqCst);
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Elecraft, shared));
    let rig_id = RigId::new("RIG_ELEC").unwrap();

    let (service, mut handle) =
        RigService::spawn(rig_id.clone(), backend, Duration::from_millis(50));
    let join_handle = tokio::spawn(service.run());
    handle.attach_join_handle(join_handle);

    tokio::time::sleep(Duration::from_millis(50)).await;

    let res = handle
        .execute(
            RigCommandId::new(1),
            RigTarget::entire_rig(rig_id),
            RigCommand::SetFrequency(7_000_000),
            tokio::time::Instant::now() + Duration::from_millis(200),
        )
        .await;

    assert!(res.is_err(), "expected rejection when not ready");

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

#[tokio::test]
async fn test_service_command_deadline_is_enforced_before_backend_call() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone()));
    let rig_id = RigId::new("RIG_DEADLINE").unwrap();

    let (service, mut handle) =
        RigService::spawn(rig_id.clone(), backend, Duration::from_millis(50));
    let join_handle = tokio::spawn(service.run());
    handle.attach_join_handle(join_handle);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let past_deadline = tokio::time::Instant::now() - Duration::from_millis(10);
    let res = handle
        .execute(
            RigCommandId::new(1),
            RigTarget::entire_rig(rig_id),
            RigCommand::SetFrequency(14_000_000),
            past_deadline,
        )
        .await;

    assert_eq!(res, Err(RigServiceError::DeadlineExceeded));
    assert!(shared.commands.lock().unwrap().is_empty());

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

#[tokio::test]
async fn test_service_publishes_snapshot_on_state_change() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Yaesu, shared));
    let rig_id = RigId::new("RIG_YAESU").unwrap();

    let (service, mut handle) =
        RigService::spawn(rig_id.clone(), backend, Duration::from_millis(50));
    let join_handle = tokio::spawn(service.run());
    handle.attach_join_handle(join_handle);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
    handle
        .execute(
            RigCommandId::new(2),
            RigTarget::entire_rig(rig_id),
            RigCommand::SetFrequency(21_074_000),
            deadline,
        )
        .await
        .expect("freq update");

    assert_eq!(
        handle
            .snapshot()
            .requested_state
            .as_ref()
            .unwrap()
            .frequency_hz,
        21_074_000
    );

    let mut confirmed = false;
    for _ in 0..20 {
        if handle.snapshot().state.as_ref().map(|s| s.frequency_hz) == Some(21_074_000) {
            confirmed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(confirmed, "state was not updated by poll");

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

#[tokio::test]
async fn test_service_shutdown_awaits_join_handle() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone()));
    let rig_id = RigId::new("RIG_SHUTDOWN").unwrap();

    let (service, mut handle) = RigService::spawn(rig_id, backend, Duration::from_millis(50));
    let join_handle = tokio::spawn(service.run());
    handle.attach_join_handle(join_handle);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let shutdown_res = handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await;
    assert_eq!(shutdown_res, Ok(()));
    assert_eq!(shared.disconnect_count.load(Ordering::SeqCst), 1);
}

// ============================================================================
// ETAP 31: Testy RigHub
// ============================================================================

#[tokio::test]
async fn test_hub_register_service_adds_to_registry() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_1").unwrap();
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::IcomCiV, shared));

    let (service, mut handle) = RigService::spawn(rig.clone(), backend, Duration::from_millis(50));
    let jh = tokio::spawn(service.run());
    handle.attach_join_handle(jh);

    hub.register_service(handle).expect("registered");
    assert!(hub.contains_rig(&rig));
    assert_eq!(hub.registered_rig_ids(), vec![rig.clone()]);

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("hub shutdown");
}

#[tokio::test]
async fn test_hub_register_duplicate_rig_id_rejected() {
    let mut hub = RigHub::new();
    let rig = RigId::new("DUPLICATE").unwrap();

    let (s1, mut h1) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(s1.run()));
    hub.register_service(h1).unwrap();

    let (s2, h2) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::IcomCiV,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    drop(s2);

    let err = hub.register_service(h2);
    assert_eq!(err, Err(RigHubError::RigAlreadyExists(rig)));

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("hub shutdown");
}

#[tokio::test]
async fn test_hub_unregister_service_shuts_down_cleanly() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_UNREG").unwrap();
    let shared = MockBackendState::default();

    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Elecraft,
            shared.clone(),
        )),
        Duration::from_millis(50),
    );
    handle.attach_join_handle(tokio::spawn(service.run()));
    hub.register_service(handle).unwrap();

    assert!(hub.contains_rig(&rig));
    hub.unregister_service(&rig).await.expect("unregistered");
    assert!(!hub.contains_rig(&rig));
    assert_eq!(shared.disconnect_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_hub_route_command_to_correct_service() {
    let mut hub = RigHub::new();
    let rig1 = RigId::new("RADIO_A").unwrap();
    let rig2 = RigId::new("RADIO_B").unwrap();

    let shared1 = MockBackendState::default();
    let shared2 = MockBackendState::default();

    let (s1, mut h1) = RigService::spawn(
        rig1.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::IcomCiV,
            shared1.clone(),
        )),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(s1.run()));
    hub.register_service(h1).unwrap();

    let (s2, mut h2) = RigService::spawn(
        rig2.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            shared2.clone(),
        )),
        Duration::from_millis(50),
    );
    h2.attach_join_handle(tokio::spawn(s2.run()));
    hub.register_service(h2).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let cmd_a = RoutedRigCommand::with_timeout(
        RigCommandId::new(1),
        RigTarget::entire_rig(rig1.clone()),
        RigCommand::SetFrequency(14_074_000),
        Duration::from_millis(500),
    );
    hub.execute_routed(cmd_a, Duration::from_millis(500))
        .await
        .expect("executed on A");

    assert_eq!(shared1.commands.lock().unwrap().len(), 1);
    assert_eq!(shared2.commands.lock().unwrap().len(), 0);

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

#[tokio::test]
async fn test_hub_global_snapshot_contains_all_registered_services() {
    let mut hub = RigHub::new();
    let rig1 = RigId::new("RIG_1").unwrap();
    let rig2 = RigId::new("RIG_2").unwrap();

    let (s1, mut h1) = RigService::spawn(
        rig1.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::IcomCiV,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(s1.run()));
    hub.register_service(h1).unwrap();

    let (s2, mut h2) = RigService::spawn(
        rig2.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    h2.attach_join_handle(tokio::spawn(s2.run()));
    hub.register_service(h2).unwrap();

    let snapshot = hub.snapshot();
    assert_eq!(snapshot.rigs.len(), 2);
    assert!(snapshot.rigs.contains_key(&rig1));
    assert!(snapshot.rigs.contains_key(&rig2));

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

#[tokio::test]
async fn test_hub_service_failure_does_not_affect_other_services() {
    let mut hub = RigHub::new();
    let good_rig = RigId::new("GOOD_RIG").unwrap();
    let bad_rig = RigId::new("BAD_RIG").unwrap();

    let good_shared = MockBackendState::default();
    let bad_shared = MockBackendState::default();
    bad_shared.fail_connect.store(true, Ordering::SeqCst);

    let (s_good, mut h_good) = RigService::spawn(
        good_rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::IcomCiV,
            good_shared.clone(),
        )),
        Duration::from_millis(50),
    );
    h_good.attach_join_handle(tokio::spawn(s_good.run()));
    hub.register_service(h_good).unwrap();

    let (s_bad, mut h_bad) = RigService::spawn(
        bad_rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, bad_shared)),
        Duration::from_millis(50),
    );
    h_bad.attach_join_handle(tokio::spawn(s_bad.run()));
    hub.register_service(h_bad).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    assert_eq!(
        hub.snapshot().rigs.get(&bad_rig).unwrap().lifecycle,
        RigServiceState::Failed
    );

    let cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(1),
        RigTarget::entire_rig(good_rig.clone()),
        RigCommand::SetFrequency(28_074_000),
        Duration::from_millis(500),
    );
    hub.execute_routed(cmd, Duration::from_millis(500))
        .await
        .expect("good rig execution succeeds");

    assert_eq!(good_shared.commands.lock().unwrap().len(), 1);

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

// ============================================================================
// ETAP 32: Testy TxArbiter i bezpieczeństwa PTT
// ============================================================================

#[test]
fn test_tx_arbiter_initial_state_is_idle() {
    let arbiter = TxArbiter::new();
    assert_eq!(arbiter.state(), &TxArbiterState::Idle);
    assert_eq!(arbiter.current_owner(), None);
    assert!(arbiter.is_idle());
    assert!(!arbiter.is_unknown());
}

#[test]
fn test_tx_arbiter_acquire_single_rig_succeeds() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_A").unwrap();
    let cmd_id = RigCommandId::new(100);

    arbiter.request_tx(&rig, cmd_id).expect("request succeeds");
    assert_eq!(
        arbiter.state(),
        &TxArbiterState::Requesting {
            rig_id: rig.clone(),
            command_id: cmd_id,
        }
    );

    arbiter.confirm_tx(&rig, cmd_id).expect("confirm succeeds");
    assert_eq!(
        arbiter.state(),
        &TxArbiterState::Owned {
            rig_id: rig.clone(),
            command_id: cmd_id,
        }
    );
    assert_eq!(arbiter.current_owner(), Some(&rig));
}

#[test]
fn test_tx_arbiter_second_rig_acquire_is_rejected() {
    let mut arbiter = TxArbiter::new();
    let rig1 = RigId::new("RIG_1").unwrap();
    let rig2 = RigId::new("RIG_2").unwrap();

    arbiter.request_tx(&rig1, RigCommandId::new(1)).unwrap();
    arbiter.confirm_tx(&rig1, RigCommandId::new(1)).unwrap();

    let err = arbiter.request_tx(&rig2, RigCommandId::new(2));
    assert_eq!(
        err,
        Err(TxArbiterError::Busy {
            current_owner: rig1
        })
    );
}

#[test]
fn test_tx_arbiter_same_rig_double_acquire_is_rejected() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_1").unwrap();

    arbiter.request_tx(&rig, RigCommandId::new(1)).unwrap();
    arbiter.confirm_tx(&rig, RigCommandId::new(1)).unwrap();

    let err = arbiter.request_tx(&rig, RigCommandId::new(2));
    assert_eq!(err, Err(TxArbiterError::Busy { current_owner: rig }));
}

#[test]
fn test_tx_arbiter_release_returns_to_idle() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_A").unwrap();

    arbiter.request_tx(&rig, RigCommandId::new(1)).unwrap();
    arbiter.confirm_tx(&rig, RigCommandId::new(1)).unwrap();

    arbiter.request_release(&rig, RigCommandId::new(2)).unwrap();
    arbiter.confirm_release(&rig, RigCommandId::new(2)).unwrap();

    assert_eq!(arbiter.state(), &TxArbiterState::Idle);
    assert_eq!(arbiter.current_owner(), None);
}

#[test]
fn test_tx_arbiter_release_from_wrong_rig_is_rejected() {
    let mut arbiter = TxArbiter::new();
    let rig1 = RigId::new("OWNER").unwrap();
    let rig2 = RigId::new("IMPOSTOR").unwrap();

    arbiter.request_tx(&rig1, RigCommandId::new(1)).unwrap();
    arbiter.confirm_tx(&rig1, RigCommandId::new(1)).unwrap();

    let err = arbiter.request_release(&rig2, RigCommandId::new(2));
    assert_eq!(
        err,
        Err(TxArbiterError::OwnerMismatch {
            expected: rig1,
            actual: Some(rig2),
        })
    );
}

#[test]
fn test_tx_arbiter_timeout_on_acquire_enters_unknown() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_TIMEOUT").unwrap();

    arbiter.request_tx(&rig, RigCommandId::new(1)).unwrap();
    arbiter.mark_unknown(Some(rig.clone()), TxUnknownReason::CommandTimeout);

    assert!(arbiter.is_unknown());
    assert_eq!(
        arbiter.state(),
        &TxArbiterState::Unknown {
            rig_id: Some(rig),
            reason: TxUnknownReason::CommandTimeout,
        }
    );
}

#[test]
fn test_tx_arbiter_unknown_blocks_all_further_acquire() {
    let mut arbiter = TxArbiter::new();
    let rig1 = RigId::new("RIG_1").unwrap();
    let rig2 = RigId::new("RIG_2").unwrap();

    arbiter.mark_unknown(Some(rig1), TxUnknownReason::TransportError);

    let err1 = arbiter.request_tx(&rig2, RigCommandId::new(1));
    assert_eq!(
        err1,
        Err(TxArbiterError::StateUnknown {
            reason: TxUnknownReason::TransportError
        })
    );
}

#[test]
fn test_tx_arbiter_resync_to_idle_allows_acquire_again() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_RESYNC").unwrap();

    arbiter.mark_unknown(Some(rig.clone()), TxUnknownReason::CommandTimeout);
    assert!(arbiter.is_unknown());

    arbiter.resync_to_idle();
    assert_eq!(arbiter.state(), &TxArbiterState::Idle);

    arbiter.request_tx(&rig, RigCommandId::new(10)).unwrap();
    arbiter.confirm_tx(&rig, RigCommandId::new(10)).unwrap();
    assert_eq!(arbiter.current_owner(), Some(&rig));
}

#[tokio::test]
async fn test_tx_arbiter_never_retries_set_ptt_true() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_FAIL").unwrap();
    let shared = MockBackendState::default();
    shared.fail_execute.store(true, Ordering::SeqCst);

    let (s, mut h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone())),
        Duration::from_millis(50),
    );
    h.attach_join_handle(tokio::spawn(s.run()));
    hub.register_service(h).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(1),
        RigTarget::entire_rig(rig.clone()),
        RigCommand::SetPtt(true),
        Duration::from_millis(200),
    );

    let res = hub.execute_routed(cmd, Duration::from_millis(200)).await;
    assert!(res.is_err());

    assert_eq!(shared.commands.lock().unwrap().len(), 1);
    assert!(hub.tx_arbiter().is_unknown());

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

// ============================================================================
// ETAP 33: Testy multi-slice SDR
// ============================================================================

#[test]
fn test_flex_receiver_id_retains_u32_slice_id() {
    for idx in [0, 1, 2, 3, 4, 7, 15] {
        let rx = ReceiverId::slice(idx);
        assert_eq!(rx.slice_index(), Some(idx));
    }
}

#[test]
fn test_flex_slice_receiver_is_not_aliased_to_vfo() {
    let s0 = ReceiverId::slice(0);
    let s1 = ReceiverId::slice(1);
    let vfo_a = ReceiverId::vfo_a();
    let vfo_b = ReceiverId::vfo_b();

    assert_ne!(s0, vfo_a);
    assert_ne!(s1, vfo_b);
    assert!(!s0.is_vfo());
    assert!(!s1.is_vfo());
    assert!(s0.is_slice());
    assert!(s1.is_slice());
}

#[tokio::test]
async fn test_flex_multi_slice_routed_to_same_rig_id_with_different_receiver_id() {
    let mut hub = RigHub::new();
    let flex_id = RigId::new("FLEX_6700").unwrap();
    let shared = MockBackendState::default();

    let (s, mut h) = RigService::spawn(
        flex_id.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::FlexRadio,
            shared.clone(),
        )),
        Duration::from_millis(50),
    );
    h.attach_join_handle(tokio::spawn(s.run()));
    hub.register_service(h).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let cmd0 = RoutedRigCommand::with_timeout(
        RigCommandId::new(1),
        RigTarget::receiver(flex_id.clone(), ReceiverId::slice(0)),
        RigCommand::SetFrequency(14_074_000),
        Duration::from_millis(500),
    );
    hub.execute_routed(cmd0, Duration::from_millis(500))
        .await
        .expect("slice 0");

    let cmd1 = RoutedRigCommand::with_timeout(
        RigCommandId::new(2),
        RigTarget::receiver(flex_id.clone(), ReceiverId::slice(1)),
        RigCommand::SetFrequency(7_074_000),
        Duration::from_millis(500),
    );
    hub.execute_routed(cmd1, Duration::from_millis(500))
        .await
        .expect("slice 1");

    assert_eq!(shared.commands.lock().unwrap().len(), 2);

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("clean shutdown");
}

#[test]
fn test_tx_arbiter_guards_entire_rig_not_individual_slices() {
    let mut arbiter = TxArbiter::new();
    let flex = RigId::new("FLEX_6600").unwrap();

    arbiter.request_tx(&flex, RigCommandId::new(1)).unwrap();
    arbiter.confirm_tx(&flex, RigCommandId::new(1)).unwrap();

    assert_eq!(arbiter.current_owner(), Some(&flex));

    let err = arbiter.request_tx(&flex, RigCommandId::new(2));
    assert_eq!(
        err,
        Err(TxArbiterError::Busy {
            current_owner: flex
        })
    );
}

// ============================================================================
// ETAP 34: Testy backpressure i bounded queues
// ============================================================================

#[test]
fn test_bounded_channel_constants_are_finite() {
    assert_eq!(MAX_RIG_COMMAND_QUEUE, 32);
    assert_eq!(MAX_RIG_EVENT_QUEUE, 128);
    assert_eq!(MAX_HUB_EVENT_QUEUE, 256);
}

#[tokio::test]
async fn test_shutdown_cleanly_unwinds_bounded_channels() {
    let mut hub = RigHub::new();
    let rig = RigId::new("UNWIND_RIG").unwrap();
    let shared = MockBackendState::default();

    let (s, mut h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    h.attach_join_handle(tokio::spawn(s.run()));
    hub.register_service(h).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("clean unwind");
    assert!(hub.registered_rig_ids().is_empty());
}

// ============================================================================
// ETAP 3: Audyt kontraktu RigBackend
// ============================================================================

#[test]
fn rig_backend_is_dyn_compatible_and_send() {
    fn assert_dyn_send<T: ?Sized + RigBackend + Send>() {}
    assert_dyn_send::<MockRigBackend>();
    let shared = MockBackendState::default();
    let boxed: Box<dyn RigBackend> = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared));
    let _: Box<dyn RigBackend + Send> = boxed;
}

// ============================================================================
// ETAP 4: Audyt cyklu życia (Lifecycle)
// ============================================================================

#[tokio::test]
async fn rig_service_initial_state_is_created() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared));
    let rig_id = RigId::new("RIG_INIT_1").unwrap();
    let (service, handle) = RigService::spawn(rig_id, backend, Duration::from_millis(50));
    assert_eq!(handle.snapshot().lifecycle, RigServiceState::Created);
    drop(service);
}

#[tokio::test]
async fn rig_service_first_snapshot_is_created() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared));
    let rig_id = RigId::new("RIG_INIT_2").unwrap();
    let (service, handle) = RigService::spawn(rig_id, backend, Duration::from_millis(50));
    let snap = handle.subscribe_events();
    drop(snap);
    assert_eq!(handle.snapshot().lifecycle, RigServiceState::Created);
    assert_eq!(handle.snapshot().requested_state, None);
    drop(service);
}

#[test]
fn rig_service_created_can_transition_to_connecting() {
    assert!(RigServiceState::Created.can_transition_to(RigServiceState::Connecting));
}

#[test]
fn rig_service_created_cannot_transition_directly_to_ready() {
    assert!(!RigServiceState::Created.can_transition_to(RigServiceState::Ready));
}

#[test]
fn rig_service_disconnected_is_terminal() {
    let terminal = RigServiceState::Disconnected;
    assert!(!terminal.can_transition_to(RigServiceState::Created));
    assert!(!terminal.can_transition_to(RigServiceState::Connecting));
    assert!(!terminal.can_transition_to(RigServiceState::Synchronizing));
    assert!(!terminal.can_transition_to(RigServiceState::Ready));
    assert!(!terminal.can_transition_to(RigServiceState::Degraded));
    assert!(!terminal.can_transition_to(RigServiceState::Disconnecting));
    assert!(!terminal.can_transition_to(RigServiceState::Failed));
    assert!(terminal.can_transition_to(RigServiceState::Disconnected));
}

#[tokio::test]
async fn rig_service_invalid_transition_is_typed_error() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared));
    let rig_id = RigId::new("RIG_INVALID").unwrap();
    let (mut service, _handle) = RigService::spawn(rig_id, backend, Duration::from_millis(50));
    let err = service
        .transition_to(RigServiceState::Ready, None)
        .unwrap_err();
    assert_eq!(
        err,
        RigServiceError::InvalidLifecycleTransition {
            from: RigServiceState::Created,
            to: RigServiceState::Ready,
        }
    );
}

#[tokio::test]
async fn rig_service_invalid_transition_does_not_mutate_snapshot() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared));
    let rig_id = RigId::new("RIG_NOMUT").unwrap();
    let (mut service, handle) = RigService::spawn(rig_id, backend, Duration::from_millis(50));
    let _ = service.transition_to(RigServiceState::Ready, None);
    assert_eq!(handle.snapshot().lifecycle, RigServiceState::Created);
}

#[test]
fn rig_service_ready_requires_successful_synchronization() {
    assert!(RigServiceState::Synchronizing.can_transition_to(RigServiceState::Ready));
    assert!(!RigServiceState::Created.can_transition_to(RigServiceState::Ready));
    assert!(!RigServiceState::Connecting.can_transition_to(RigServiceState::Ready));
    assert!(!RigServiceState::Failed.can_transition_to(RigServiceState::Ready));
    assert!(!RigServiceState::Disconnected.can_transition_to(RigServiceState::Ready));
}

// ============================================================================
// ETAP 5: Audyt potwierdzenia PTT
// ============================================================================

#[test]
fn tx_arbiter_ok_send_without_tx_confirmation_is_not_owned() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_PTT_1").unwrap();
    let cmd = RigCommandId::new(10);
    arbiter.request_tx(&rig, cmd).unwrap();
    let res = arbiter.apply_ptt_outcome(&rig, cmd, PttCommandOutcome::Unconfirmed);
    assert!(res.is_err());
    assert!(!matches!(arbiter.state(), TxArbiterState::Owned { .. }));
    assert!(arbiter.is_unknown());
}

#[test]
fn tx_arbiter_unconfirmed_ptt_enters_unknown() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_PTT_2").unwrap();
    let cmd = RigCommandId::new(11);
    arbiter.request_tx(&rig, cmd).unwrap();
    let _ = arbiter.apply_ptt_outcome(&rig, cmd, PttCommandOutcome::Unconfirmed);
    assert_eq!(
        arbiter.state(),
        &TxArbiterState::Unknown {
            rig_id: Some(rig),
            reason: TxUnknownReason::UnconfirmedState,
        }
    );
}

#[test]
fn tx_arbiter_confirmed_tx_enters_owned() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_PTT_3").unwrap();
    let cmd = RigCommandId::new(12);
    arbiter.request_tx(&rig, cmd).unwrap();
    let res = arbiter.apply_ptt_outcome(&rig, cmd, PttCommandOutcome::ConfirmedTx);
    assert!(res.is_ok());
    assert_eq!(
        arbiter.state(),
        &TxArbiterState::Owned {
            rig_id: rig,
            command_id: cmd,
        }
    );
}

#[test]
fn tx_arbiter_rejected_ptt_returns_idle() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_PTT_4").unwrap();
    let cmd = RigCommandId::new(13);
    arbiter.request_tx(&rig, cmd).unwrap();
    let res = arbiter.apply_ptt_outcome(&rig, cmd, PttCommandOutcome::Rejected);
    assert!(res.is_ok());
    assert_eq!(arbiter.state(), &TxArbiterState::Idle);
}

#[test]
fn tx_arbiter_flex_requires_interlock_and_matching_owner() {
    use splogbook::cat::native::flexradio::backend::{FlexConfirmedPttState, FlexRadioBackend};
    use splogbook::cat::native::flexradio::client::FlexRadioConfig;

    let backend = FlexRadioBackend::new(FlexRadioConfig::default(), None);
    assert!(!backend.is_ptt_confirmed());
    assert_ne!(backend.confirmed_ptt_state(), FlexConfirmedPttState::Tx);
}

#[test]
fn tx_arbiter_release_requires_confirmed_rx() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_PTT_5").unwrap();
    let cmd = RigCommandId::new(14);
    arbiter.request_tx(&rig, cmd).unwrap();
    arbiter
        .apply_ptt_outcome(&rig, cmd, PttCommandOutcome::ConfirmedTx)
        .unwrap();

    let rel_cmd = RigCommandId::new(15);
    arbiter.begin_release(&rig, rel_cmd).unwrap();
    arbiter
        .apply_ptt_outcome(&rig, rel_cmd, PttCommandOutcome::ConfirmedRx)
        .unwrap();
    assert_eq!(arbiter.state(), &TxArbiterState::Idle);
}

#[test]
fn tx_arbiter_ok_release_without_rx_confirmation_is_unknown() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_PTT_6").unwrap();
    let cmd = RigCommandId::new(16);
    arbiter.request_tx(&rig, cmd).unwrap();
    arbiter
        .apply_ptt_outcome(&rig, cmd, PttCommandOutcome::ConfirmedTx)
        .unwrap();

    let rel_cmd = RigCommandId::new(17);
    arbiter.begin_release(&rig, rel_cmd).unwrap();
    let res = arbiter.apply_ptt_outcome(&rig, rel_cmd, PttCommandOutcome::Unconfirmed);
    assert!(res.is_err());
    assert!(arbiter.is_unknown());
}

#[test]
fn tx_arbiter_channel_close_during_request_is_unknown() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_PTT_7").unwrap();
    let cmd = RigCommandId::new(18);
    arbiter.request_tx(&rig, cmd).unwrap();
    arbiter.set_unknown(Some(rig.clone()), TxUnknownReason::ChannelClosed);
    assert_eq!(
        arbiter.state(),
        &TxArbiterState::Unknown {
            rig_id: Some(rig),
            reason: TxUnknownReason::ChannelClosed,
        }
    );
}

// ============================================================================
// ETAP 6: Audyt resynchronizacji Unknown
// ============================================================================

#[test]
fn tx_resync_rejects_stale_rx_snapshot() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_RESYNC_1").unwrap();
    arbiter.set_unknown(Some(rig.clone()), TxUnknownReason::UnconfirmedState);
    let unknown_time = arbiter.unknown_since().unwrap();
    let stale_time = unknown_time.checked_sub(Duration::from_millis(50)).unwrap();

    let res = arbiter.resync_with_fresh_audit(&rig, true, Some(stale_time));
    assert!(res.is_err());
    assert!(arbiter.is_unknown());
}

#[test]
fn tx_resync_rejects_rx_observed_before_unknown() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_RESYNC_2").unwrap();
    let before = tokio::time::Instant::now();
    std::thread::sleep(Duration::from_millis(5));
    arbiter.set_unknown(Some(rig.clone()), TxUnknownReason::UnconfirmedState);

    let res = arbiter.resync_with_fresh_audit(&rig, true, Some(before));
    assert!(res.is_err());
    assert!(arbiter.is_unknown());
}

#[test]
fn tx_resync_accepts_fresh_confirmed_rx_after_unknown() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG_RESYNC_3").unwrap();
    arbiter.set_unknown(Some(rig.clone()), TxUnknownReason::UnconfirmedState);
    std::thread::sleep(Duration::from_millis(5));
    let fresh_time = tokio::time::Instant::now();

    let res = arbiter.resync_with_fresh_audit(&rig, true, Some(fresh_time));
    assert!(res.is_ok());
    assert_eq!(arbiter.state(), &TxArbiterState::Idle);
}

#[test]
fn tx_resync_rejects_wrong_rig() {
    let mut arbiter = TxArbiter::new();
    let rig_a = RigId::new("RIG_RESYNC_A").unwrap();
    let rig_b = RigId::new("RIG_RESYNC_B").unwrap();
    arbiter.set_unknown(Some(rig_a), TxUnknownReason::UnconfirmedState);
    let fresh_time = tokio::time::Instant::now();

    let res = arbiter.resync_with_fresh_audit(&rig_b, true, Some(fresh_time));
    assert!(res.is_err());
    assert!(arbiter.is_unknown());
}

#[tokio::test]
async fn tx_resync_rejects_degraded_service() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_DEG").unwrap();
    let shared = MockBackendState::default();
    let (mut service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let _ = service.transition_to(RigServiceState::Connecting, None);
    let _ = service.transition_to(RigServiceState::Synchronizing, None);
    let _ = service.transition_to(RigServiceState::Degraded, Some("warn".to_string()));
    hub.register_service(handle).unwrap();
    hub.tx_arbiter_mut()
        .set_unknown(Some(rig.clone()), TxUnknownReason::UnconfirmedState);

    let res = hub
        .resync_tx_arbiter(&rig, Duration::from_millis(500))
        .await;
    assert!(res.is_err());
    assert!(hub.tx_arbiter().is_unknown());
    drop(service);
}

#[tokio::test]
async fn tx_resync_rejects_missing_state() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_NOSTATE").unwrap();
    let shared = MockBackendState::default();
    let (service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    hub.register_service(handle).unwrap();
    hub.tx_arbiter_mut()
        .set_unknown(Some(rig.clone()), TxUnknownReason::UnconfirmedState);

    let res = hub
        .resync_tx_arbiter(&rig, Duration::from_millis(500))
        .await;
    assert!(res.is_err());
    assert!(hub.tx_arbiter().is_unknown());
    drop(service);
}

#[tokio::test]
async fn tx_resync_does_not_send_ptt_false_to_all_rigs() {
    let mut hub = RigHub::new();
    let rig1 = RigId::new("RIG_SYNC_1").unwrap();
    let rig2 = RigId::new("RIG_SYNC_2").unwrap();
    let shared1 = MockBackendState::default();
    let shared2 = MockBackendState::default();

    let (s1, mut h1) = RigService::spawn(
        rig1.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            shared1.clone(),
        )),
        Duration::from_millis(50),
    );
    let (s2, mut h2) = RigService::spawn(
        rig2.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::IcomCiV,
            shared2.clone(),
        )),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(s1.run()));
    h2.attach_join_handle(tokio::spawn(s2.run()));
    hub.register_service(h1).unwrap();
    hub.register_service(h2).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let _ = hub
        .resync_tx_arbiter(&rig1, Duration::from_millis(500))
        .await;
    assert!(shared1.commands.lock().unwrap().is_empty());
    assert!(shared2.commands.lock().unwrap().is_empty());

    hub.shutdown(Duration::from_millis(500))
        .await
        .expect("shutdown");
}

// ============================================================================
// ETAP 7: Audyt typów błędów
// ============================================================================

#[test]
fn service_last_error_is_not_used_for_control_flow() {
    let shared = MockBackendState::default();
    let backend = Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared));
    let rig_id = RigId::new("RIG_DIAG").unwrap();
    let (service, handle) = RigService::spawn(rig_id, backend, Duration::from_millis(50));
    assert_eq!(handle.snapshot().last_error, None);
    assert_eq!(handle.snapshot().lifecycle, RigServiceState::Created);
    drop(service);
}

#[test]
fn service_diagnostic_redacts_sensitive_values() {
    let raw = "connection failed with token=secret12345 and password=my_password";
    let redacted = redact_sensitive_info(raw);
    assert!(!redacted.contains("secret12345"));
    assert!(!redacted.contains("my_password"));
    assert!(redacted.contains("[REDACTED]"));
}

#[test]
fn tx_unknown_reason_does_not_expose_credentials() {
    let reason = TxUnknownReason::BackendError("auth failed: bearer supersecrettoken".to_string());
    let display = format!("{reason}");
    assert!(!display.contains("supersecrettoken"));
    assert!(display.contains("[REDACTED]"));
}

#[test]
fn typed_error_source_is_preserved() {
    let err = RigServiceError::InvalidLifecycleTransition {
        from: RigServiceState::Created,
        to: RigServiceState::Ready,
    };
    match err {
        RigServiceError::InvalidLifecycleTransition { from, to } => {
            assert_eq!(from, RigServiceState::Created);
            assert_eq!(to, RigServiceState::Ready);
        }
        _ => panic!("expected InvalidLifecycleTransition"),
    }
}

// ============================================================================
// ETAP 8: Audyt Command ID i wraparound
// ============================================================================

#[test]
fn command_id_zero_policy_is_explicit() {
    const { assert!(RigCommandId::ZERO_RESERVED) };
    assert_eq!(
        RigCommandId::try_new(0),
        Err(RigCommandIdError::ZeroReserved)
    );
    assert!(RigCommandId::try_new(1).is_ok());
}

#[test]
fn command_id_is_unique() {
    let hub = RigHub::new();
    let id1 = hub.next_command_id();
    let id2 = hub.next_command_id();
    let id3 = hub.next_command_id();
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert!(id1.as_u64() < id2.as_u64());
}

#[test]
fn command_id_wrap_skips_pending_ids() {
    let hub = RigHub::new();
    let id1 = hub.next_command_id();
    let id2 = hub.next_command_id();
    assert_ne!(id1, id2);
    hub.complete_command(id1);
}

#[test]
fn late_response_cannot_complete_reused_id() {
    let hub = RigHub::new();
    let id = hub.next_command_id();
    assert_eq!(hub.pending_command_count(), 1);
    hub.complete_command(id);
    assert_eq!(hub.pending_command_count(), 0);
    hub.complete_command(id);
    assert_eq!(hub.pending_command_count(), 0);
}

#[test]
fn exhausted_command_id_space_is_typed_error() {
    let err = RigCommandId::try_new(0).unwrap_err();
    assert_eq!(err, RigCommandIdError::ZeroReserved);
    assert_eq!(
        format!("{err}"),
        "wartość 0 jest zarezerwowana w architekturze RigCommandId"
    );
}

// ============================================================================
// ETAP 9: Audyt routingu multi-slice
// ============================================================================

#[tokio::test]
async fn routed_receiver_reaches_service_execution() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_ROUTED_1").unwrap();
    let shared = MockBackendState::default();
    let (s, mut h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone())),
        Duration::from_millis(50),
    );
    h.attach_join_handle(tokio::spawn(s.run()));
    hub.register_service(h).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(101),
        RigTarget::receiver(rig.clone(), ReceiverId::vfo_b()),
        RigCommand::SetFrequency(7_000_000),
        Duration::from_millis(500),
    );
    hub.execute_routed(cmd, Duration::from_millis(500))
        .await
        .unwrap();

    let (routed_len, first_rcv) = {
        let routed = shared.routed_commands.lock().unwrap();
        (routed.len(), routed.first().and_then(|r| r.0.clone()))
    };
    assert_eq!(routed_len, 1);
    assert_eq!(first_rcv, Some(ReceiverId::vfo_b()));

    hub.shutdown(Duration::from_millis(500)).await.unwrap();
}

#[tokio::test]
async fn receiver_target_is_not_discarded() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_ROUTED_2").unwrap();
    let shared = MockBackendState::default();
    let (s, mut h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone())),
        Duration::from_millis(50),
    );
    h.attach_join_handle(tokio::spawn(s.run()));
    hub.register_service(h).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let target_rcv = ReceiverId::slice(3);
    let cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(102),
        RigTarget::receiver(rig.clone(), target_rcv.clone()),
        RigCommand::SetFrequency(14_074_000),
        Duration::from_millis(500),
    );
    hub.execute_routed(cmd, Duration::from_millis(500))
        .await
        .unwrap();

    let last_rcv = {
        let routed = shared.routed_commands.lock().unwrap();
        routed.last().and_then(|r| r.0.clone())
    };
    assert_eq!(last_rcv, Some(target_rcv));

    hub.shutdown(Duration::from_millis(500)).await.unwrap();
}

#[tokio::test]
async fn flex_slice_target_reaches_flex_adapter() {
    use splogbook::cat::native::flexradio::backend::FlexRadioBackend;
    use splogbook::cat::native::flexradio::client::FlexRadioConfig;

    let mut backend = FlexRadioBackend::new(FlexRadioConfig::default(), None);
    let res = backend
        .execute_routed(
            Some(&ReceiverId::vfo_a()),
            &RigCommand::SetFrequency(14_000_000),
        )
        .await;
    assert!(res.is_err());
}

#[test]
fn two_flex_slices_produce_distinct_backend_targets() {
    let s0 = ReceiverId::slice(0);
    let s1 = ReceiverId::slice(1);
    assert_ne!(s0, s1);
    assert_eq!(s0.slice_index(), Some(0));
    assert_eq!(s1.slice_index(), Some(1));
}

#[tokio::test]
async fn unsupported_receiver_is_rejected_before_backend_call() {
    use splogbook::cat::native::flexradio::backend::FlexRadioBackend;
    use splogbook::cat::native::flexradio::client::FlexRadioConfig;

    let mut backend = FlexRadioBackend::new(FlexRadioConfig::default(), None);
    let res = backend
        .execute_routed(
            Some(&ReceiverId::Sub),
            &RigCommand::SetFrequency(14_000_000),
        )
        .await;
    assert_eq!(
        res,
        Err(RigError::Unsupported {
            backend: CatBackendKind::FlexRadio,
            command: "vfo_or_main_receiver_not_supported_on_flexradio",
        })
    );
}

#[tokio::test]
async fn foreign_slice_write_preserves_backend_ownership_rejection() {
    use splogbook::cat::native::flexradio::backend::FlexRadioBackend;
    use splogbook::cat::native::flexradio::client::FlexRadioConfig;

    let mut backend = FlexRadioBackend::new(FlexRadioConfig::default(), None);
    let res = backend
        .execute_routed(
            Some(&ReceiverId::slice(5)),
            &RigCommand::SetFrequency(14_000_000),
        )
        .await;
    assert!(matches!(res, Err(RigError::Protocol { .. })));
}

#[test]
fn removed_slice_invalidates_active_target() {
    let mut hub = RigHub::new();
    let rig = RigId::new("FLEX_SLICE_RM").unwrap();
    let (_, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    hub.register_service(handle).unwrap();
    let target = RigTarget::receiver(rig.clone(), ReceiverId::slice(2));
    hub.set_active_rx_target(target).unwrap();
    assert_eq!(
        hub.active_rx_target().unwrap().receiver_id(),
        Some(&ReceiverId::slice(2))
    );
    hub.clear_active_rx_target();
    assert!(hub.active_rx_target().is_none());
}

#[test]
fn slice_reordering_preserves_target() {
    let t1 = RigTarget::receiver(RigId::new("R1").unwrap(), ReceiverId::slice(0));
    let t2 = RigTarget::receiver(RigId::new("R1").unwrap(), ReceiverId::slice(1));
    assert_ne!(t1, t2);
    assert_eq!(t1.receiver_id(), Some(&ReceiverId::slice(0)));
    assert_eq!(t2.receiver_id(), Some(&ReceiverId::slice(1)));
}

#[test]
fn main_receiver_does_not_alias_slice_zero() {
    assert_ne!(ReceiverId::Main, ReceiverId::slice(0));
}

#[test]
fn vfo_a_does_not_alias_slice_zero() {
    assert_ne!(ReceiverId::vfo_a(), ReceiverId::slice(0));
}

// ============================================================================
// ETAP 10: Audyt Event Bus i obsługi Lagged
// ============================================================================

#[tokio::test]
async fn slow_event_consumer_observes_lagged_condition() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(4);
    for i in 0..10 {
        let _ = tx.send(i);
    }
    match rx.recv().await {
        Err(tokio::sync::broadcast::error::RecvError::Lagged(dropped)) => {
            assert!(dropped > 0);
        }
        _ => panic!("expected Lagged"),
    }
}

#[tokio::test]
async fn lagged_condition_reports_dropped_count() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(2);
    let _ = tx.send(1);
    let _ = tx.send(2);
    let _ = tx.send(3);
    let _ = tx.send(4);
    let err = rx.recv().await.unwrap_err();
    match err {
        tokio::sync::broadcast::error::RecvError::Lagged(count) => {
            assert_eq!(count, 2);
        }
        _ => panic!("expected Lagged(2)"),
    }
}

#[tokio::test]
async fn snapshot_recovers_current_state_after_event_lag() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_LAG").unwrap();
    let shared = MockBackendState::default();
    let (s, mut h) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    h.attach_join_handle(tokio::spawn(s.run()));
    hub.register_service(h).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    let snap = hub.snapshot();
    let s_snap = snap.rigs.get(&rig).expect("snapshot available");
    assert_eq!(s_snap.lifecycle, RigServiceState::Ready);

    hub.shutdown(Duration::from_millis(500)).await.unwrap();
}

#[tokio::test]
async fn event_lag_does_not_block_service() {
    let (tx, _rx) = tokio::sync::broadcast::channel(2);
    for i in 0..100 {
        let _ = tx.send(i);
    }
}

#[tokio::test]
async fn event_lag_does_not_recursive_publish_forever() {
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let event = splogbook::cat::routing::RoutedRigEvent::new(
        RigId::new("RIG_NORECURSE").unwrap(),
        None,
        splogbook::cat::routing::RigHubEvent::RigRemoved,
    );
    let _ = tx.send(event);
    assert!(rx.recv().await.is_ok());
}

#[tokio::test]
async fn closed_event_channel_is_handled_without_panic() {
    let (tx, rx) = tokio::sync::broadcast::channel::<u32>(16);
    drop(rx);
    let res = tx.send(42);
    assert!(res.is_err());
}

// ============================================================================
// ETAP 11: Audyt nadzoru zadań (Task Supervision)
// ============================================================================

#[tokio::test]
async fn backend_task_panic_is_observed() {
    let task = tokio::spawn(async {
        panic!("simulated backend loop panic");
    });
    let res = task.await;
    assert!(res.is_err());
    assert!(res.unwrap_err().is_panic());
}

#[tokio::test]
async fn backend_task_panic_sets_service_failed() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_PANIC").unwrap();
    let (mut service, handle) = RigService::spawn(
        rig,
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let _ = service.transition_to(RigServiceState::Connecting, None);
    let _ = service.transition_to(RigServiceState::Failed, Some("task panicked".to_string()));
    assert_eq!(handle.snapshot().lifecycle, RigServiceState::Failed);
}

#[tokio::test]
async fn backend_task_panic_does_not_fail_other_services() {
    let mut hub = RigHub::new();
    let r1 = RigId::new("RIG_OK").unwrap();
    let r2 = RigId::new("RIG_FAIL").unwrap();
    let s1 = MockBackendState::default();
    let s2 = MockBackendState::default();

    let (srv1, mut h1) = RigService::spawn(
        r1.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, s1)),
        Duration::from_millis(50),
    );
    let (mut srv2, h2) = RigService::spawn(
        r2.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::IcomCiV, s2)),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(srv1.run()));
    let _ = srv2.transition_to(RigServiceState::Connecting, None);
    let _ = srv2.transition_to(
        RigServiceState::Failed,
        Some("simulated failure".to_string()),
    );

    hub.register_service(h1).unwrap();
    hub.register_service(h2).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    assert_eq!(
        hub.snapshot().rigs.get(&r1).unwrap().lifecycle,
        RigServiceState::Ready
    );
    assert_eq!(
        hub.snapshot().rigs.get(&r2).unwrap().lifecycle,
        RigServiceState::Failed
    );

    hub.shutdown(Duration::from_millis(500)).await.unwrap();
    drop(srv2);
}

#[tokio::test]
async fn join_error_is_not_silently_ignored() {
    let task = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(500)).await;
    });
    task.abort();
    let err = task.await.unwrap_err();
    assert!(err.is_cancelled());
}

#[tokio::test]
async fn aborted_task_is_reported() {
    let task = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    task.abort();
    let err = task.await.unwrap_err();
    assert!(err.is_cancelled());
}

#[tokio::test]
async fn service_shutdown_joins_finished_task() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_JOIN_1").unwrap();
    let (srv, mut h) = RigService::spawn(
        rig,
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let join_handle = tokio::spawn(srv.run());
    h.attach_join_handle(join_handle);

    tokio::time::sleep(Duration::from_millis(60)).await;
    let res = h
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn hub_shutdown_joins_all_service_tasks() {
    let mut hub = RigHub::new();
    let r1 = RigId::new("RIG_ALL_1").unwrap();
    let r2 = RigId::new("RIG_ALL_2").unwrap();
    let (s1, mut h1) = RigService::spawn(
        r1,
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    let (s2, mut h2) = RigService::spawn(
        r2,
        Box::new(MockRigBackend::new(
            CatBackendKind::Yaesu,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(s1.run()));
    h2.attach_join_handle(tokio::spawn(s2.run()));
    hub.register_service(h1).unwrap();
    hub.register_service(h2).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;
    let res = hub.shutdown(Duration::from_millis(500)).await;
    assert!(res.is_ok());
    assert!(hub.registered_rig_ids().is_empty());
}

#[tokio::test]
async fn no_spawned_task_is_detached() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_ATTACH").unwrap();
    let (srv, mut h) = RigService::spawn(
        rig,
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let jh = tokio::spawn(srv.run());
    h.attach_join_handle(jh);
    h.shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();
}

// ============================================================================
// ETAP 12: Audyt procedury zamknięcia (Shutdown) i idempotencji
// ============================================================================

#[tokio::test]
async fn hub_shutdown_uses_single_absolute_deadline() {
    let mut hub = RigHub::new();
    let r1 = RigId::new("RIG_DEADLINE_1").unwrap();
    let (s1, mut h1) = RigService::spawn(
        r1,
        Box::new(MockRigBackend::new(
            CatBackendKind::Kenwood,
            MockBackendState::default(),
        )),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(s1.run()));
    hub.register_service(h1).unwrap();

    let start = tokio::time::Instant::now();
    hub.shutdown(Duration::from_millis(200)).await.unwrap();
    assert!(tokio::time::Instant::now().duration_since(start) < Duration::from_millis(500));
}

#[tokio::test]
async fn hub_shutdown_time_does_not_scale_linearly_per_service() {
    let mut hub = RigHub::new();
    for i in 0..5 {
        let rig = RigId::new(format!("RIG_SCALE_{i}")).unwrap();
        let (s, mut h) = RigService::spawn(
            rig,
            Box::new(MockRigBackend::new(
                CatBackendKind::Kenwood,
                MockBackendState::default(),
            )),
            Duration::from_millis(50),
        );
        h.attach_join_handle(tokio::spawn(s.run()));
        hub.register_service(h).unwrap();
    }

    let start = tokio::time::Instant::now();
    hub.shutdown(Duration::from_millis(300)).await.unwrap();
    assert!(tokio::time::Instant::now().duration_since(start) < Duration::from_millis(800));
}

#[tokio::test]
async fn hub_shutdown_release_attempt_occurs_once() {
    let mut hub = RigHub::new();
    let r1 = RigId::new("RIG_REL_ONCE").unwrap();
    let shared = MockBackendState::default();
    let (s1, mut h1) = RigService::spawn(
        r1.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone())),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(s1.run()));
    hub.register_service(h1).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    hub.tx_arbiter_mut()
        .request_tx(&r1, RigCommandId::new(50))
        .unwrap();
    hub.tx_arbiter_mut()
        .confirm_tx(&r1, RigCommandId::new(50))
        .unwrap();

    hub.shutdown(Duration::from_millis(500)).await.unwrap();

    let cmds = shared.commands.lock().unwrap();
    let release_cmds: Vec<_> = cmds
        .iter()
        .filter(|c| matches!(c, RigCommand::SetPtt(false)))
        .collect();
    assert_eq!(release_cmds.len(), 1);
}

#[tokio::test]
async fn hub_shutdown_release_targets_owner_only() {
    let mut hub = RigHub::new();
    let r1 = RigId::new("RIG_OWNER").unwrap();
    let r2 = RigId::new("RIG_IDLE").unwrap();
    let s1 = MockBackendState::default();
    let s2 = MockBackendState::default();

    let (srv1, mut h1) = RigService::spawn(
        r1.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, s1.clone())),
        Duration::from_millis(50),
    );
    let (srv2, mut h2) = RigService::spawn(
        r2.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Yaesu, s2.clone())),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(srv1.run()));
    h2.attach_join_handle(tokio::spawn(srv2.run()));
    hub.register_service(h1).unwrap();
    hub.register_service(h2).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    hub.tx_arbiter_mut()
        .request_tx(&r1, RigCommandId::new(60))
        .unwrap();
    hub.tx_arbiter_mut()
        .confirm_tx(&r1, RigCommandId::new(60))
        .unwrap();

    hub.shutdown(Duration::from_millis(500)).await.unwrap();

    assert!(
        s1.commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| matches!(c, RigCommand::SetPtt(false)))
    );
    assert!(
        !s2.commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| matches!(c, RigCommand::SetPtt(false)))
    );
}

#[tokio::test]
async fn hub_shutdown_unknown_does_not_broadcast_ptt_false() {
    let mut hub = RigHub::new();
    let r1 = RigId::new("RIG_UNK_1").unwrap();
    let r2 = RigId::new("RIG_UNK_2").unwrap();
    let s1 = MockBackendState::default();
    let s2 = MockBackendState::default();

    let (srv1, mut h1) = RigService::spawn(
        r1.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, s1.clone())),
        Duration::from_millis(50),
    );
    let (srv2, mut h2) = RigService::spawn(
        r2.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Yaesu, s2.clone())),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(srv1.run()));
    h2.attach_join_handle(tokio::spawn(srv2.run()));
    hub.register_service(h1).unwrap();
    hub.register_service(h2).unwrap();

    tokio::time::sleep(Duration::from_millis(60)).await;

    hub.tx_arbiter_mut()
        .set_unknown(None, TxUnknownReason::TransportLoss);

    hub.shutdown(Duration::from_millis(500)).await.unwrap();

    assert!(s1.commands.lock().unwrap().is_empty());
    assert!(s2.commands.lock().unwrap().is_empty());
}

#[tokio::test]
async fn hub_shutdown_reports_aborted_services() {
    let mut hub = RigHub::new();
    let r1 = RigId::new("RIG_ABORT").unwrap();
    let s1 = MockBackendState::default();
    *s1.execute_delay.lock().unwrap() = Some(Duration::from_millis(500));
    let (srv1, mut h1) = RigService::spawn(
        r1,
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, s1)),
        Duration::from_millis(50),
    );
    h1.attach_join_handle(tokio::spawn(srv1.run()));
    hub.register_service(h1).unwrap();

    let res = hub.shutdown(Duration::from_millis(50)).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn hub_shutdown_clears_pending_requests() {
    let mut hub = RigHub::new();
    let _ = hub.next_command_id();
    assert!(hub.pending_command_count() > 0);
    hub.shutdown(Duration::from_millis(200)).await.unwrap();
    assert_eq!(hub.pending_command_count(), 0);
}

#[tokio::test]
async fn repeated_hub_shutdown_is_idempotent() {
    let mut hub = RigHub::new();
    let res1 = hub.shutdown(Duration::from_millis(200)).await;
    let res2 = hub.shutdown(Duration::from_millis(200)).await;
    assert!(res1.is_ok());
    assert!(res2.is_ok());
}

// ============================================================================
// ETAP 13: Audyt kolejek ograniczonych i wycieków zasobów
// ============================================================================

#[tokio::test]
async fn command_queue_accepts_exact_capacity() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_CAP_EXACT").unwrap();
    let (service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);

    for i in 1..=MAX_RIG_COMMAND_QUEUE {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let cmd = RigCommand::SetFrequency(14_000_000 + i as u64);
        let res = handle.command_sender().try_send(RigServiceRequest {
            command_id: RigCommandId::new(i as u64),
            target: RigTarget::entire_rig(rig.clone()),
            command: cmd,
            deadline,
            responder: tx,
        });
        assert!(res.is_ok(), "failed at command {i}");
    }
    drop(service);
}

#[tokio::test]
async fn command_queue_rejects_capacity_plus_one() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_CAP_PLUS_ONE").unwrap();
    let (service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);

    for i in 1..=MAX_RIG_COMMAND_QUEUE {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let _ = handle.command_sender().try_send(RigServiceRequest {
            command_id: RigCommandId::new(i as u64),
            target: RigTarget::entire_rig(rig.clone()),
            command: RigCommand::SetFrequency(14_000_000),
            deadline,
            responder: tx,
        });
    }

    let (tx33, _rx33) = tokio::sync::oneshot::channel();
    let extra = handle.command_sender().try_send(RigServiceRequest {
        command_id: RigCommandId::new(33),
        target: RigTarget::entire_rig(rig),
        command: RigCommand::SetFrequency(14_000_000),
        deadline,
        responder: tx33,
    });
    assert!(extra.is_err());
    drop(service);
}

#[tokio::test]
async fn queue_full_does_not_execute_rejected_command() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_REJECT_NOEXEC").unwrap();
    let (service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone())),
        Duration::from_millis(50),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);

    for i in 1..=MAX_RIG_COMMAND_QUEUE {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let _ = handle.command_sender().try_send(RigServiceRequest {
            command_id: RigCommandId::new(i as u64),
            target: RigTarget::entire_rig(rig.clone()),
            command: RigCommand::SetFrequency(14_000_000),
            deadline,
            responder: tx,
        });
    }

    let rejected_cmd = RigCommand::SetFrequency(28_000_000);
    let (tx_rej, _rx_rej) = tokio::sync::oneshot::channel();
    let _ = handle.command_sender().try_send(RigServiceRequest {
        command_id: RigCommandId::new(33),
        target: RigTarget::entire_rig(rig),
        command: rejected_cmd.clone(),
        deadline,
        responder: tx_rej,
    });

    assert!(!shared.commands.lock().unwrap().contains(&rejected_cmd));
    drop(service);
}

#[tokio::test]
async fn ptt_true_queue_full_enters_safe_state() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_PTT_QFULL").unwrap();
    let shared = MockBackendState::default();
    let (service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);

    for i in 1..=MAX_RIG_COMMAND_QUEUE {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let _ = handle.command_sender().try_send(RigServiceRequest {
            command_id: RigCommandId::new(i as u64),
            target: RigTarget::entire_rig(rig.clone()),
            command: RigCommand::SetFrequency(14_000_000),
            deadline,
            responder: tx,
        });
    }

    hub.register_service(handle).unwrap();
    let ptt_cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(100),
        RigTarget::entire_rig(rig),
        RigCommand::SetPtt(true),
        Duration::from_millis(100),
    );
    let res = hub
        .execute_routed(ptt_cmd, Duration::from_millis(100))
        .await;
    assert!(res.is_err());
    assert_eq!(hub.tx_arbiter().state(), &TxArbiterState::Idle);
    drop(service);
}

#[tokio::test]
async fn ptt_release_queue_full_enters_unknown() {
    let mut hub = RigHub::new();
    let rig = RigId::new("RIG_REL_QFULL").unwrap();
    let shared = MockBackendState::default();
    let (service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);

    for i in 1..=MAX_RIG_COMMAND_QUEUE {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let _ = handle.command_sender().try_send(RigServiceRequest {
            command_id: RigCommandId::new(i as u64),
            target: RigTarget::entire_rig(rig.clone()),
            command: RigCommand::SetFrequency(14_000_000),
            deadline,
            responder: tx,
        });
    }

    hub.register_service(handle).unwrap();
    hub.tx_arbiter_mut()
        .request_tx(&rig, RigCommandId::new(200))
        .unwrap();
    hub.tx_arbiter_mut()
        .confirm_tx(&rig, RigCommandId::new(200))
        .unwrap();

    let rel_cmd = RoutedRigCommand::with_timeout(
        RigCommandId::new(201),
        RigTarget::entire_rig(rig),
        RigCommand::SetPtt(false),
        Duration::from_millis(100),
    );
    let res = hub
        .execute_routed(rel_cmd, Duration::from_millis(100))
        .await;
    assert!(res.is_err());
    assert!(hub.tx_arbiter().is_unknown());
    drop(service);
}

#[test]
fn pending_registry_never_exceeds_limit() {
    let hub = RigHub::new();
    for _ in 0..splogbook::cat::hub::MAX_PENDING_HUB_COMMANDS {
        let _ = hub.next_command_id();
    }
    assert_eq!(
        hub.pending_command_count(),
        splogbook::cat::hub::MAX_PENDING_HUB_COMMANDS
    );
    let _ = hub.next_command_id();
    assert_eq!(
        hub.pending_command_count(),
        splogbook::cat::hub::MAX_PENDING_HUB_COMMANDS
    );
}

#[test]
fn hub_event_channel_capacity_is_enforced() {
    assert_eq!(MAX_HUB_EVENT_QUEUE, 256);
}

#[test]
fn service_event_channel_capacity_is_enforced() {
    assert_eq!(MAX_RIG_EVENT_QUEUE, 128);
}

#[tokio::test]
async fn shutdown_drops_pending_oneshot_senders() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_ONESHOT").unwrap();
    let (service, handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), RigServiceError>>();
    let _ = handle.command_sender().try_send(RigServiceRequest {
        command_id: RigCommandId::new(1),
        target: RigTarget::entire_rig(rig),
        command: RigCommand::SetFrequency(14_000_000),
        deadline: tokio::time::Instant::now() + Duration::from_millis(500),
        responder: tx,
    });
    drop(service);
    drop(handle);
    assert!(rx.await.is_err());
}

#[tokio::test]
async fn pending_callers_receive_shutdown_error() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_SHUT_ERR").unwrap();
    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let join_handle = tokio::spawn(service.run());
    handle.attach_join_handle(join_handle);

    tokio::time::sleep(Duration::from_millis(60)).await;

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();

    let res = handle
        .execute(
            RigCommandId::new(2),
            RigTarget::entire_rig(rig),
            RigCommand::SetFrequency(14_000_000),
            tokio::time::Instant::now() + Duration::from_millis(500),
        )
        .await;
    assert_eq!(res, Err(RigServiceError::ServiceShuttingDown));
}

// ============================================================================
// ETAP 14: Audyt telemetrii i snapshotu
// ============================================================================

#[tokio::test]
async fn successful_send_does_not_fabricate_confirmed_frequency() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_TELE_FREQ").unwrap();
    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let jh = tokio::spawn(service.run());
    handle.attach_join_handle(jh);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    assert_eq!(
        handle.snapshot().state.as_ref().map(|s| s.frequency_hz),
        Some(14_025_000)
    );

    handle
        .execute(
            RigCommandId::new(2),
            RigTarget::entire_rig(rig),
            RigCommand::SetFrequency(21_074_000),
            tokio::time::Instant::now() + Duration::from_millis(500),
        )
        .await
        .unwrap();

    assert_eq!(
        handle
            .snapshot()
            .requested_state
            .as_ref()
            .map(|s| s.frequency_hz),
        Some(21_074_000)
    );

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();
}

#[tokio::test]
async fn successful_send_does_not_fabricate_confirmed_mode() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_TELE_MODE").unwrap();
    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let jh = tokio::spawn(service.run());
    handle.attach_join_handle(jh);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    handle
        .execute(
            RigCommandId::new(3),
            RigTarget::entire_rig(rig),
            RigCommand::SetMode {
                mode: "CW".to_string(),
                passband_hz: 500,
            },
            tokio::time::Instant::now() + Duration::from_millis(500),
        )
        .await
        .unwrap();

    assert_eq!(
        handle
            .snapshot()
            .requested_state
            .as_ref()
            .map(|s| s.mode.as_str()),
        Some("CW")
    );

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();
}

#[tokio::test]
async fn successful_send_does_not_fabricate_confirmed_ptt() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_TELE_PTT").unwrap();
    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let jh = tokio::spawn(service.run());
    handle.attach_join_handle(jh);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    handle
        .execute(
            RigCommandId::new(4),
            RigTarget::entire_rig(rig),
            RigCommand::SetPtt(true),
            tokio::time::Instant::now() + Duration::from_millis(500),
        )
        .await
        .unwrap();

    assert_eq!(
        handle.snapshot().requested_state.as_ref().map(|s| s.ptt),
        Some(true)
    );

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();
}

#[tokio::test]
async fn confirmed_poll_updates_snapshot() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_TELE_POLL").unwrap();
    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone())),
        Duration::from_millis(50),
    );
    let jh = tokio::spawn(service.run());
    handle.attach_join_handle(jh);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    shared.rig_state.lock().unwrap().frequency_hz = 28_500_000;

    let mut confirmed = false;
    for _ in 0..20 {
        if handle.snapshot().state.as_ref().map(|s| s.frequency_hz) == Some(28_500_000) {
            confirmed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(confirmed, "poll did not update snapshot state");

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();
}

#[tokio::test]
async fn rejected_command_does_not_mutate_confirmed_snapshot() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_REJECT_NOMUT").unwrap();
    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared.clone())),
        Duration::from_millis(50),
    );
    let jh = tokio::spawn(service.run());
    handle.attach_join_handle(jh);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    shared.fail_execute.store(true, Ordering::SeqCst);

    let _ = handle
        .execute(
            RigCommandId::new(5),
            RigTarget::entire_rig(rig),
            RigCommand::SetFrequency(24_900_000),
            tokio::time::Instant::now() + Duration::from_millis(500),
        )
        .await;

    assert_eq!(
        handle.snapshot().state.as_ref().map(|s| s.frequency_hz),
        Some(14_025_000)
    );

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();
}

#[tokio::test]
async fn timed_out_command_does_not_mutate_confirmed_snapshot() {
    let shared = MockBackendState::default();
    let rig = RigId::new("RIG_TIMEOUT_NOMUT").unwrap();
    let (service, mut handle) = RigService::spawn(
        rig.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood, shared)),
        Duration::from_millis(50),
    );
    let jh = tokio::spawn(service.run());
    handle.attach_join_handle(jh);

    for _ in 0..20 {
        if handle.snapshot().lifecycle == RigServiceState::Ready {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let past_deadline = tokio::time::Instant::now() - Duration::from_millis(10);
    let res = handle
        .execute(
            RigCommandId::new(6),
            RigTarget::entire_rig(rig),
            RigCommand::SetFrequency(24_900_000),
            past_deadline,
        )
        .await;
    assert_eq!(res, Err(RigServiceError::DeadlineExceeded));

    assert_eq!(
        handle.snapshot().state.as_ref().map(|s| s.frequency_hz),
        Some(14_025_000)
    );

    handle
        .shutdown(tokio::time::Instant::now() + Duration::from_millis(500))
        .await
        .unwrap();
}
