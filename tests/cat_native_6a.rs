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
    ActiveRxTarget, ActiveTxTarget, ReceiverId, RigCommandId, RigHubEvent, RigId, RigIdError,
    RigTarget, RoutedRigCommand,
};
use splogbook::cat::service::{
    MAX_RIG_COMMAND_QUEUE, MAX_RIG_EVENT_QUEUE, RigService, RigServiceError, RigServiceState,
};
use splogbook::cat::tx_arbiter::{TxArbiter, TxArbiterError, TxArbiterState, TxUnknownReason};

// ============================================================================
// ETAP 35: Deterministyczny MockRigBackend
// ============================================================================

#[derive(Clone)]
pub struct MockBackendState {
    pub connected: Arc<AtomicBool>,
    pub connect_count: Arc<AtomicUsize>,
    pub disconnect_count: Arc<AtomicUsize>,
    pub commands: Arc<StdMutex<Vec<RigCommand>>>,
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
        handle.snapshot().state.as_ref().unwrap().frequency_hz,
        21_074_000
    );

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
