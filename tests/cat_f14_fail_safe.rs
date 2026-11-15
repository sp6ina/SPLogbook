// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz WoĹşniak (SP6INA)
//! F-14 software remediation tests â€” PTT panic, fail-closed range handling and release ordering.
//!
//! These tests drive the **real** production `HilHardwareSession` logic against TEST-ONLY
//! protocol doubles. No hardware, no serial port, no USB device, no `rigctld`, no real PTT and
//! no RF is involved. The ten `#[ignore = "hardware-in-the-loop"]` tests stay untouched.
//!
//! Safety invariants asserted throughout:
//! * an invalid/reversed range never reaches the device (`PTT ON == 0`);
//! * `PTT ON >= 1` implies `PTT OFF >= 1` (every controlled post-ON path attempts a release);
//! * a failed or unconfirmed release is never reported as success;
//! * no analysed input causes a panic.

use std::ffi::OsStr;
use std::task::{Context, Waker};
use std::time::Duration;

use splogbook::cat::backend::{CatBackendKind, RigBackend};
use splogbook::cat::hil::{
    HIL_MAX_TEST_DURATION_MS, HIL_MIN_TEST_DURATION_MS, HilDummyLoadSpec, HilError,
    HilEvidenceOrigin, HilExecutionStage, HilHardwareSession, HilInstrumentMeasurement,
    HilMatrixEntry, HilOperatorAttestation, HilPttConsentToken, HilReleaseOutcome,
    HilRfSafetyConfig, HilRfTermination, HilTestBenchManifest, HilTransportConfig,
};
use splogbook::cat::rig::RigCommand;
use splogbook::cat::routing::RigId;
use splogbook::cat::tx_arbiter::TxArbiter;

#[path = "support/process_env.rs"]
mod process_env;
#[path = "support/protocol_double.rs"]
mod protocol_double;

use process_env::EnvGuard;
use protocol_double::{DocumentedPttProtocol, DoubleFaults, ProtocolDouble};

const REFERENCE_TIME: &str = "2026-10-18T12:00:00Z";

fn reference_time() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(REFERENCE_TIME)
        .expect("valid RFC3339 reference time")
        .with_timezone(&chrono::Utc)
}

fn manifest_with_duration(max_test_duration_ms: u64) -> HilTestBenchManifest {
    HilTestBenchManifest {
        bench_id: "LAB-BENCH-SP6INA-01".to_string(),
        session_id: "SESSION-F14-01".to_string(),
        created_at: "2026-10-18T09:30:00Z".to_string(),
        expires_at: Some("2026-10-18T23:59:59Z".to_string()),
        tested_commit: Some("e1ff373b5d264f3f3de2383c19936f45b2befe6a".to_string()),
        expected_commit: Some("e1ff373b5d264f3f3de2383c19936f45b2befe6a".to_string()),
        working_tree_clean: true,
        software_tests_passed: true,
        operator_id: "SP6INA".to_string(),
        operator_callsign: "SP6INA".to_string(),
        operator_name: "Mariusz WoĹşniak".to_string(),
        hil_enabled: true,
        allow_tx: true,
        matrix: HilMatrixEntry {
            backend_kind: CatBackendKind::IcomCiV,
            rig_id: RigId::new("RIG-F14-01").unwrap(),
            model_name: "IC-7300".to_string(),
            transport: HilTransportConfig::Serial {
                port_name: "COM3".to_string(),
                baud_rate: 115_200,
                data_bits: 8,
                parity: "none".to_string(),
                stop_bits: 1,
                flow_control: "none".to_string(),
            },
            firmware_version: "v1.42".to_string(),
        },
        rf_safety: HilRfSafetyConfig {
            termination: HilRfTermination::DummyLoad(HilDummyLoadSpec {
                power_rating_watts: 100.0,
                max_swr: 1.15,
                serial_number: "DL-50W-001".to_string(),
                impedance_ohms: 50.0,
            }),
            max_test_power_watts: 10.0,
            frequency_range_hz: (14_000_000, 14_350_000),
            interlock_engaged: true,
            max_test_duration_ms,
            smartlink_disabled: true,
            unattended_remote_tx_disabled: true,
            remote_endpoint_is_local_or_controlled: true,
            operator_has_immediate_stop_control: true,
            out_of_band_data_plane_disabled: true,
        },
        operator_attestation: HilOperatorAttestation {
            dummy_load_physically_connected: true,
            rf_perimeter_secured: true,
            ptt_consent_granted: true,
            attestation_timestamp: "2026-10-17T20:56:00Z".to_string(),
            description: "Software-only F-14 verification bench".to_string(),
            antenna_disconnected: true,
            pa_disconnected_or_locked: true,
            tuner_bypassed: true,
            physical_stop_available: true,
            abort_procedure_documented: true,
            operator_acknowledged: true,
            bench_readiness_confirmed: true,
            operator_present: true,
            supervised_trial_confirmed: true,
            rules_compliance_attested: true,
            operator_privileges_attested: true,
            allowed_band_attested: true,
            allowed_mode_attested: true,
            no_unauthorized_emission_attested: true,
            personnel_know_stop_procedure: true,
        },
        instrument_verification: Some(HilInstrumentMeasurement {
            instrument_id: "BIRD-43".to_string(),
            instrument_model: "Bird 43 Thruline".to_string(),
            measured_swr: 1.08,
            measured_power_watts: Some(9.8),
            calibration_valid_until: Some("2027-12-31".to_string()),
            measurement_timestamp: Some("2026-10-17T20:55:00Z".to_string()),
            origin: HilEvidenceOrigin::PhysicalMeasurement,
        }),
    }
}

fn session(max_test_duration_ms: u64) -> HilHardwareSession {
    HilHardwareSession::new_with_reference_time(
        manifest_with_duration(max_test_duration_ms),
        HilExecutionStage::Stage7BHardware,
        Some(reference_time()),
    )
}

fn token() -> HilPttConsentToken {
    HilPttConsentToken::new("SP6INA").expect("consent token")
}

/// Environment gates required by the real `verify_hardware_access_authorization_at` and
/// `verify_tx_authorization_at` paths. The shared `tests/support/process_env.rs` guard is the
/// only sanctioned way to mutate process environment in this repository.
fn env_gates() -> EnvGuard {
    EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some(OsStr::new("1"))),
        ("SPLOG_HIL_SESSION", Some(OsStr::new("SESSION-F14-01"))),
        ("SPLOG_HIL_ALLOW_TX", Some(OsStr::new("I_HAVE_DUMMY_LOAD"))),
    ])
}

/// Acquires the real TX guard and immediately performs the real explicit release.
async fn acquire_and_release(
    session: &HilHardwareSession,
    double: &mut ProtocolDouble,
    requested: Duration,
) -> Result<splogbook::cat::hil::HilPttExecutionResult, HilError> {
    let mut arbiter = TxArbiter::new();
    let mut guard = session
        .acquire_tx_guard_at(double, &token(), &mut arbiter, requested, reference_time())
        .await?;
    guard.release_and_confirm_rx().await
}

// =========================================================================
// PHASE 7 â€” SOFTWARE REPRODUCTION OF F-14
// =========================================================================

/// F-14 reproduction: `max_test_duration_ms` (5 ms) below the transmit path's lower bound must
/// never reach a clamp and must never key PTT. Before the fix this test panicked inside
/// `acquire_tx_guard_at` **after** `SetPtt(true)`.
#[tokio::test]
async fn f14_reversed_range_is_rejected_before_any_ptt_on() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let mut arbiter = TxArbiter::new();

    let session = session(5);
    let res = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(50),
            reference_time(),
        )
        .await;

    assert!(
        res.is_err(),
        "F-14: reversed range must be a controlled error"
    );
    assert_eq!(
        counters.ptt_on_count(),
        0,
        "F-14: no PTT ON may reach the device for an invalid range"
    );
    assert!(!counters.last_ptt_state(), "F-14: device must stay in RX");
    assert!(arbiter.is_idle(), "F-14: arbiter must stay Idle");
}

/// F-14 reproduction through the public controlled-PTT entry point.
#[tokio::test]
async fn f14_reversed_range_is_rejected_in_controlled_ptt_path() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();

    let session = session(5);
    let res = session
        .execute_controlled_ptt(&mut double, &token(), Duration::from_millis(5))
        .await;

    assert!(
        res.is_err(),
        "F-14: reversed range must be a controlled error"
    );
    assert_eq!(counters.ptt_on_count(), 0, "F-14: zero PTT ON");
    assert!(!counters.last_ptt_state(), "F-14: device must stay in RX");
}

// =========================================================================
// PHASE 8/11 â€” RANGE VALIDATION
// =========================================================================

/// Empty (0) and every representable sub-minimum range (1..=9) are rejected before PTT ON.
#[tokio::test]
async fn f14_empty_and_below_min_ranges_never_key_ptt() {
    let _env = env_gates();
    for max_ms in 0..HIL_MIN_TEST_DURATION_MS {
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let session = session(max_ms);
        let res = session
            .acquire_tx_guard_at(
                &mut double,
                &token(),
                &mut arbiter,
                Duration::from_millis(50),
                reference_time(),
            )
            .await;
        assert!(res.is_err(), "max={max_ms} must be rejected");
        assert_eq!(counters.ptt_on_count(), 0, "max={max_ms}: zero PTT ON");
        match res {
            Err(HilError::SafetyViolation(_)) => {}
            Err(other) => panic!("max={max_ms}: unexpected error {other:?}"),
            Ok(_) => panic!("max={max_ms}: guard must not be created"),
        }
    }
}

/// `min == max` is explicit: a maximum equal to the clamp floor is legal, bounded exactly, and
/// the pulse deadline expires first, which must still attempt a release and never report success.
#[tokio::test]
async fn f14_min_equals_max_range_is_explicit_and_releases() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let session = session(HIL_MIN_TEST_DURATION_MS);
    let res = session
        .execute_controlled_ptt(
            &mut double,
            &token(),
            Duration::from_millis(HIL_MIN_TEST_DURATION_MS),
        )
        .await;

    match res {
        Err(HilError::TimeoutToUnknown(_)) => {}
        other => panic!("expected TimeoutToUnknown for a full-width pulse, got {other:?}"),
    }
    assert_eq!(counters.ptt_on_count(), 1, "exactly one ON");
    assert_eq!(counters.ptt_off_count(), 1, "exactly one OFF attempt");
}

/// A valid range keeps working: exactly one ON and one OFF, in the documented frame order.
#[tokio::test]
async fn f14_valid_range_transmits_once_and_releases_once() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let session = session(500);

    let res = acquire_and_release(&session, &mut double, Duration::from_millis(50))
        .await
        .expect("valid range must succeed");

    assert!(res.rx_restored_cleanly);
    assert_eq!(counters.ptt_on_count(), 1, "exactly one ON");
    assert_eq!(counters.ptt_off_count(), 1, "exactly one OFF");
    assert!(!counters.last_ptt_state(), "device returned to RX");
    assert_eq!(
        counters.sequence(),
        vec![
            DocumentedPttProtocol::ICOM_CI_V.ptt_on.to_string(),
            DocumentedPttProtocol::ICOM_CI_V.ptt_off.to_string()
        ],
        "documented ON then OFF frames, in that order"
    );
}

/// A zero pulse duration is bounded up to the documented minimum and never panics.
#[tokio::test]
async fn f14_zero_duration_is_bounded_to_minimum() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let session = session(500);
    let res = acquire_and_release(&session, &mut double, Duration::ZERO).await;
    assert!(res.is_ok(), "zero pulse must be bounded, not rejected");
    assert_eq!(counters.ptt_on_count(), 1);
    assert_eq!(counters.ptt_off_count(), 1);
}

/// A pulse above the manifest maximum is bounded down; `Duration::MAX` is representable and
/// must not overflow or panic.
#[tokio::test]
async fn f14_oversized_and_type_bound_durations_are_bounded() {
    let _env = env_gates();
    for requested in [
        Duration::from_millis(99_999),
        Duration::from_secs(u64::MAX / 2),
        Duration::MAX,
    ] {
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let session = session(HIL_MAX_TEST_DURATION_MS);
        let res = session
            .execute_controlled_ptt(&mut double, &token(), requested)
            .await;
        assert!(res.is_err(), "full-width pulse must expire, not panic");
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }
}

/// Manifest validation rejects a duration above the laboratory limit and non-finite power/SWR.
#[test]
fn f14_manifest_rejects_out_of_range_and_non_finite_limits() {
    for bad in [HIL_MAX_TEST_DURATION_MS + 1, 600, u64::MAX] {
        let manifest = manifest_with_duration(bad);
        assert!(
            matches!(
                manifest.validate_structure(),
                Err(HilError::SafetyViolation(_))
            ),
            "max_test_duration_ms={bad} must be rejected"
        );
    }

    for bad_power in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut manifest = manifest_with_duration(500);
        manifest.rf_safety.max_test_power_watts = bad_power;
        assert!(
            matches!(
                manifest.validate_structure(),
                Err(HilError::SafetyViolation(_))
            ),
            "max_test_power_watts={bad_power} must be rejected"
        );
    }

    for bad_swr in [f64::NAN, f64::INFINITY] {
        let mut manifest = manifest_with_duration(500);
        if let Some(inst) = &mut manifest.instrument_verification {
            inst.measured_swr = bad_swr;
        }
        assert!(
            matches!(
                manifest.validate_structure(),
                Err(HilError::SafetyViolation(_))
            ),
            "measured_swr={bad_swr} must be rejected"
        );
    }

    assert!(
        manifest_with_duration(HIL_MIN_TEST_DURATION_MS)
            .validate_structure()
            .is_ok()
    );
    assert!(
        manifest_with_duration(HIL_MAX_TEST_DURATION_MS)
            .validate_structure()
            .is_ok()
    );
}

// =========================================================================
// PHASE 8/11 â€” POST-PTT-ON PATHS AND CLEANUP
// =========================================================================

/// A measurement error before PTT ON never keys the transmitter.
#[tokio::test]
async fn f14_measurement_error_before_on_never_keys() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            poll_error: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let res = acquire_and_release(&session, &mut double, Duration::from_millis(20)).await;
    assert!(res.is_err());
    assert_eq!(counters.ptt_on_count(), 0);
}

/// A measurement error **after** PTT ON must attempt PTT OFF and report failure.
#[tokio::test]
async fn f14_measurement_error_after_on_attempts_release() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            poll_error_after_on: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();
    let mut guard = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await
        .expect("ON path is valid");
    assert_eq!(counters.ptt_on_count(), 1);
    let res = guard.release_and_confirm_rx().await;
    assert!(res.is_err(), "unconfirmed RX must not be success");
    assert_eq!(counters.ptt_off_count(), 1, "release attempted");
    assert!(matches!(
        guard.release_outcome(),
        HilReleaseOutcome::Unconfirmed
    ));
}

/// A device error on the ON command still triggers exactly one emergency PTT OFF.
#[tokio::test]
async fn f14_on_failure_attempts_emergency_release() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            fail_on: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let mut arbiter = TxArbiter::new();
    let session = session(500);
    let res = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await;
    assert!(res.is_err());
    assert_eq!(counters.ptt_on_count(), 1);
    assert_eq!(
        counters.ptt_off_count(),
        1,
        "fail-closed: the ON error path must attempt a release"
    );
    assert!(arbiter.is_idle(), "arbiter returns to Idle");
}

/// A rejected ON command never keys the device and still performs the safety release attempt.
#[tokio::test]
async fn f14_rejected_on_never_keys_and_is_fail_closed() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            reject_on: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let mut arbiter = TxArbiter::new();
    let session = session(500);
    let res = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await;
    assert!(res.is_err());
    assert_eq!(counters.ptt_on_count(), 1);
    assert_eq!(counters.ptt_off_count(), 1);
    assert!(!counters.last_ptt_state());
}

/// A failed OFF is an error, never success, and the release outcome records the failure.
#[tokio::test]
async fn f14_off_failure_is_not_reported_as_success() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            fail_off: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();
    let mut guard = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await
        .expect("ON path is valid");
    let res = guard.release_and_confirm_rx().await;
    assert!(
        res.is_err(),
        "F-14: OFF failure must be an error, never success"
    );
    assert_eq!(counters.ptt_on_count(), 1);
    assert_eq!(counters.ptt_off_count(), 1, "one release attempt is made");
    assert!(matches!(guard.release_outcome(), HilReleaseOutcome::Failed));
    assert!(
        counters.last_ptt_state(),
        "the double still holds TX; the result must not claim RX"
    );
}

/// A release timeout is an error, never success.
#[tokio::test]
async fn f14_off_timeout_is_not_reported_as_success() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            silent_off: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();
    let mut guard = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await
        .expect("ON path is valid");
    assert!(guard.release_and_confirm_rx().await.is_err());
    assert_eq!(counters.ptt_off_count(), 1);
    assert!(matches!(guard.release_outcome(), HilReleaseOutcome::Failed));
}

/// A malformed/truncated OFF answer is an error, never success.
#[tokio::test]
async fn f14_malformed_off_response_is_not_reported_as_success() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            malformed_off: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();
    let mut guard = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await
        .expect("ON path is valid");
    assert!(guard.release_and_confirm_rx().await.is_err());
    assert_eq!(counters.ptt_off_count(), 1);
}

/// A device that keeps reporting TX after the OFF command must produce
/// `MissingConfirmedRx`, not a clean release.
#[tokio::test]
async fn f14_unconfirmed_rx_is_not_success() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            sticky_after_on: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let res = acquire_and_release(&session, &mut double, Duration::from_millis(20)).await;
    match res {
        Err(HilError::MissingConfirmedRx(_)) => {}
        other => panic!("expected MissingConfirmedRx, got {other:?}"),
    }
    assert_eq!(counters.ptt_on_count(), 1, "the ON command was sent");
    assert_eq!(counters.ptt_off_count(), 1, "the OFF command was sent");
}

/// Transport loss immediately after PTT ON: the release attempt fails and the result is an
/// error that names the failure; it is never a success.
#[tokio::test]
async fn f14_disconnect_after_on_reports_release_failure() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            disconnect_after_on: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();
    let mut guard = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await
        .expect("ON path is valid");
    let res = guard.release_and_confirm_rx().await;
    assert!(res.is_err());
    assert_eq!(counters.ptt_on_count(), 1);
    assert_eq!(counters.ptt_off_count(), 1, "release still attempted");
    assert!(matches!(guard.release_outcome(), HilReleaseOutcome::Failed));
}

/// Duplicate cleanup is rejected explicitly and the first outcome is preserved.
#[tokio::test]
async fn f14_duplicate_release_is_rejected() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();
    let mut guard = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await
        .expect("ON path is valid");
    assert!(guard.release_and_confirm_rx().await.is_ok());
    let second = guard.release_and_confirm_rx().await;
    assert!(matches!(second, Err(HilError::ReleaseError(_))));
    assert_eq!(counters.ptt_on_count(), 1, "no retry of ON");
    assert_eq!(counters.ptt_off_count(), 1, "no duplicate OFF");
    assert!(matches!(
        guard.release_outcome(),
        HilReleaseOutcome::Confirmed
    ));
}

// =========================================================================
// PHASE 11/12 â€” CANCELLATION AND DROP
// =========================================================================

/// Deterministic cancellation: the future is polled exactly once, which drives the real path up
/// to the pulse await, and then dropped. The guard's Drop must attempt exactly one PTT OFF.
#[tokio::test]
async fn f14_cancellation_after_ptt_on_attempts_release() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();
    let token = token();

    let mut future = Box::pin(session.execute_controlled_ptt_with_arbiter_at(
        &mut double,
        &token,
        &mut arbiter,
        Duration::from_millis(50),
        reference_time(),
    ));
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    assert!(
        future.as_mut().poll(&mut cx).is_pending(),
        "the pulse await must park the future"
    );
    assert_eq!(counters.ptt_on_count(), 1, "PTT ON happened before parking");
    drop(future);

    assert_eq!(counters.ptt_on_count(), 1, "no retry of ON");
    assert_eq!(
        counters.ptt_off_count(),
        1,
        "cancellation must attempt exactly one PTT OFF"
    );
    assert!(!counters.last_ptt_state(), "device returned to RX");
    assert!(
        !arbiter.is_idle(),
        "the arbiter is left in a non-Idle safety state after cancellation"
    );
}

/// Dropping an unreleased guard performs exactly one best-effort OFF and never fabricates
/// a confirmed RX.
#[tokio::test]
async fn f14_abandoned_guard_attempts_one_release_without_fabricating_rx() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let state = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));
    {
        let guard = splogbook::cat::hil::HilTxGuard::new_with_state(
            RigId::new("RIG-F14-01").unwrap(),
            &mut double,
            Duration::from_millis(50),
            state.clone(),
        );
        assert!(!guard.is_confirmed_rx());
        assert_eq!(guard.release_outcome(), HilReleaseOutcome::NotAttempted);
    }
    assert_eq!(counters.ptt_off_count(), 1, "one emergency attempt");
    assert_eq!(
        state.load(std::sync::atomic::Ordering::SeqCst),
        3,
        "session state is FailedOrUnknown, never CompletedCleanly"
    );
}

/// The emergency attempt must not block on an unresponsive device future.
#[test]
fn f14_emergency_release_does_not_block_on_pending_future() {
    let double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            pending_off: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let mut double = double;
    {
        let guard = splogbook::cat::hil::HilTxGuard::new(
            RigId::new("RIG-F14-01").unwrap(),
            &mut double,
            Duration::from_millis(50),
        );
        drop(guard);
    }
    assert_eq!(counters.ptt_off_count(), 1);
}

// =========================================================================
// PHASE 12 â€” FAULT INJECTION MATRIX
// =========================================================================

/// Every injectable fault preserves the safety invariants:
/// invalid preflight â†’ 0 ON; ON â‰Ą 1 â†’ OFF â‰Ą 1; OFF failure/timeout/disconnect â†’ not success.
#[tokio::test]
async fn f14_fault_injection_matrix_keeps_safety_invariants() {
    let _env = env_gates();
    // (label, faults, max_test_duration_ms, expected PTT ON count)
    let faults = [
        ("success", DoubleFaults::default(), 500, 1),
        (
            "fail_on",
            DoubleFaults {
                fail_on: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "silent_on",
            DoubleFaults {
                silent_on: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "reject_on",
            DoubleFaults {
                reject_on: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "fail_off",
            DoubleFaults {
                fail_off: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "silent_off",
            DoubleFaults {
                silent_off: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "malformed_off",
            DoubleFaults {
                malformed_off: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "sticky_ptt_preflight",
            DoubleFaults {
                sticky_ptt: true,
                ..Default::default()
            },
            500,
            0,
        ),
        (
            "sticky_after_on",
            DoubleFaults {
                sticky_after_on: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "poll_error",
            DoubleFaults {
                poll_error: true,
                ..Default::default()
            },
            500,
            0,
        ),
        (
            "poll_error_after_on",
            DoubleFaults {
                poll_error_after_on: true,
                ..Default::default()
            },
            500,
            1,
        ),
        (
            "disconnect_after_on",
            DoubleFaults {
                disconnect_after_on: true,
                ..Default::default()
            },
            500,
            1,
        ),
        ("reversed_range", DoubleFaults::default(), 5, 0),
    ];

    for (label, fault, max_ms, expected_on) in faults {
        let mut double = ProtocolDouble::with_faults(DocumentedPttProtocol::ICOM_CI_V, fault);
        let counters = double.counters();
        let session = session(max_ms);
        let result = session
            .execute_controlled_ptt(&mut double, &token(), Duration::from_millis(20))
            .await;

        assert_eq!(
            counters.ptt_on_count(),
            expected_on,
            "{label}: unexpected PTT ON count"
        );
        if counters.ptt_on_count() >= 1 {
            assert_eq!(
                counters.ptt_off_count(),
                1,
                "{label}: exactly one release attempt after any ON"
            );
        } else {
            assert_eq!(counters.ptt_off_count(), 0, "{label}: zero OFF without ON");
        }
        if let Ok(res) = result {
            assert!(
                res.rx_restored_cleanly,
                "{label}: success implies confirmed RX"
            );
            assert!(!counters.last_ptt_state(), "{label}: success implies RX");
        }
    }
}

// =========================================================================
// PHASE 10 â€” TEST-ONLY PROTOCOL DOUBLES (documented behaviour)
// =========================================================================

/// Each documented protocol double drives the same real HIL session to a clean, single ON/OFF
/// sequence using exactly the documented ON and OFF frames.
#[tokio::test]
async fn f14_protocol_doubles_round_trip_documented_frames() {
    let _env = env_gates();
    let protocols = [
        DocumentedPttProtocol::ICOM_CI_V,
        DocumentedPttProtocol::YAESU_CAT,
        DocumentedPttProtocol::KENWOOD_CAT,
        DocumentedPttProtocol::ELECRAFT_CAT,
        DocumentedPttProtocol::FLEX_SMARTSDR,
        DocumentedPttProtocol::TCI,
        DocumentedPttProtocol::HAMLIB_RIGCTLD,
        DocumentedPttProtocol::FLRIG,
    ];
    for protocol in protocols {
        let mut double = ProtocolDouble::new(protocol);
        let counters = double.counters();
        let session = session(500);
        let res = acquire_and_release(&session, &mut double, Duration::from_millis(20)).await;
        assert!(
            res.is_ok(),
            "{}: documented double must succeed",
            protocol.source
        );
        assert_eq!(counters.ptt_on_count(), 1, "{}: one ON", protocol.source);
        assert_eq!(counters.ptt_off_count(), 1, "{}: one OFF", protocol.source);
        assert_eq!(
            counters.sequence(),
            vec![protocol.ptt_on.to_string(), protocol.ptt_off.to_string()],
            "{}: documented frame order",
            protocol.source
        );
    }
}

/// Non-PTT commands are ignored by the doubles and never produce a PTT frame.
#[tokio::test]
async fn f14_protocol_double_ignores_non_ptt_commands() {
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    double
        .execute(&RigCommand::SetFrequency(14_074_000))
        .await
        .expect("frequency command is a no-op for the double");
    assert_eq!(counters.ptt_on_count(), 0);
    assert_eq!(counters.ptt_off_count(), 0);
}

// =========================================================================
// PHASE 13 â€” DETERMINISTIC STRESS
// =========================================================================

/// 2000 reversed-range attempts and 2000 valid-range attempts, deterministic, no sleep.
#[tokio::test]
async fn f14_stress_range_validation_2000_plus_2000() {
    let _env = env_gates();
    let mut rejected = 0usize;
    for i in 0..2000u64 {
        let max_ms = i % HIL_MIN_TEST_DURATION_MS;
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let session = session(max_ms);
        let res = session
            .acquire_tx_guard_at(
                &mut double,
                &token(),
                &mut arbiter,
                Duration::from_millis(50),
                reference_time(),
            )
            .await;
        assert!(res.is_err(), "iteration {i}: must be rejected");
        assert_eq!(counters.ptt_on_count(), 0, "iteration {i}: zero ON");
        rejected += 1;
    }
    assert_eq!(rejected, 2000);

    let session = session(500);
    let mut accepted = 0usize;
    for _ in 0..2000 {
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let guard = session
            .acquire_tx_guard_at(
                &mut double,
                &token(),
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await
            .expect("valid range must be accepted");
        drop(guard);
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
        accepted += 1;
    }
    assert_eq!(accepted, 2000);
}

/// 1000 iterations of every post-ON / cleanup path, deterministic, no sleep.
#[tokio::test]
async fn f14_stress_post_on_paths_1000_each() {
    let _env = env_gates();

    // cleanup success
    for _ in 0..1000 {
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let session = session(500);
        assert!(
            acquire_and_release(&session, &mut double, Duration::from_millis(20))
                .await
                .is_ok()
        );
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }

    // cleanup error
    for _ in 0..1000 {
        let mut double = ProtocolDouble::with_faults(
            DocumentedPttProtocol::ICOM_CI_V,
            DoubleFaults {
                fail_off: true,
                ..Default::default()
            },
        );
        let counters = double.counters();
        let session = session(500);
        assert!(
            acquire_and_release(&session, &mut double, Duration::from_millis(20))
                .await
                .is_err()
        );
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }

    // cleanup timeout
    for _ in 0..1000 {
        let mut double = ProtocolDouble::with_faults(
            DocumentedPttProtocol::ICOM_CI_V,
            DoubleFaults {
                silent_off: true,
                ..Default::default()
            },
        );
        let counters = double.counters();
        let session = session(500);
        assert!(
            acquire_and_release(&session, &mut double, Duration::from_millis(20))
                .await
                .is_err()
        );
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }

    // post-ON failure (ON acknowledgement lost)
    for _ in 0..1000 {
        let mut double = ProtocolDouble::with_faults(
            DocumentedPttProtocol::ICOM_CI_V,
            DoubleFaults {
                fail_on: true,
                ..Default::default()
            },
        );
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let session = session(500);
        let res = session
            .acquire_tx_guard_at(
                &mut double,
                &token(),
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(res.is_err());
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }

    // preflight failure: zero ON
    for _ in 0..1000 {
        let mut double = ProtocolDouble::with_faults(
            DocumentedPttProtocol::ICOM_CI_V,
            DoubleFaults {
                poll_error: true,
                ..Default::default()
            },
        );
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let session = session(500);
        let res = session
            .acquire_tx_guard_at(
                &mut double,
                &token(),
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(res.is_err());
        assert_eq!(counters.ptt_on_count(), 0);
    }

    // disconnect after ON
    for _ in 0..1000 {
        let mut double = ProtocolDouble::with_faults(
            DocumentedPttProtocol::ICOM_CI_V,
            DoubleFaults {
                disconnect_after_on: true,
                ..Default::default()
            },
        );
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let session = session(500);
        let res = session
            .acquire_tx_guard_at(
                &mut double,
                &token(),
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        let mut guard = res.expect("ON path is valid");
        assert!(guard.release_and_confirm_rx().await.is_err());
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }

    // cancellation
    for _ in 0..1000 {
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let session = session(500);
        let token = token();
        let mut future = Box::pin(session.execute_controlled_ptt_with_arbiter_at(
            &mut double,
            &token,
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        ));
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        assert!(future.as_mut().poll(&mut cx).is_pending());
        drop(future);
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }
}

/// 500 documented protocol-double round trips, 50 CAT ordering rounds and 200 full HIL software
/// scenarios â€” all deterministic and hardware-free.
#[tokio::test]
async fn f14_stress_protocol_doubles_500_cat_50_hil_200() {
    let _env = env_gates();
    let protocols = [
        DocumentedPttProtocol::ICOM_CI_V,
        DocumentedPttProtocol::YAESU_CAT,
        DocumentedPttProtocol::KENWOOD_CAT,
        DocumentedPttProtocol::ELECRAFT_CAT,
        DocumentedPttProtocol::FLEX_SMARTSDR,
        DocumentedPttProtocol::TCI,
        DocumentedPttProtocol::HAMLIB_RIGCTLD,
        DocumentedPttProtocol::FLRIG,
    ];
    let session = session(500);

    for i in 0..500usize {
        let protocol = protocols[i % protocols.len()];
        let mut double = ProtocolDouble::new(protocol);
        let counters = double.counters();
        assert!(
            acquire_and_release(&session, &mut double, Duration::from_millis(10))
                .await
                .is_ok(),
            "iteration {i} ({})",
            protocol.source
        );
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }

    for _ in 0..50 {
        for protocol in protocols {
            let mut double = ProtocolDouble::new(protocol);
            let counters = double.counters();
            assert!(
                acquire_and_release(&session, &mut double, Duration::from_millis(10))
                    .await
                    .is_ok()
            );
            assert_eq!(
                counters.sequence(),
                vec![protocol.ptt_on.to_string(), protocol.ptt_off.to_string()]
            );
        }
    }

    for _ in 0..200 {
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let probe = session.execute_safe_probe_backend(&mut double).await;
        assert!(probe.is_ok(), "software-only probe path");
        assert_eq!(counters.ptt_on_count(), 0, "probe never keys PTT");
        assert!(
            acquire_and_release(&session, &mut double, Duration::from_millis(10))
                .await
                .is_ok()
        );
        assert_eq!(counters.ptt_on_count(), 1);
        assert_eq!(counters.ptt_off_count(), 1);
    }
}

/// The production backend kind enumeration never exposes a test double.
#[test]
fn f14_production_backend_kinds_exclude_test_doubles() {
    for kind in CatBackendKind::ALL {
        let text = format!("{kind}").to_ascii_lowercase();
        for forbidden in ["mock", "simulator", "fake", "dummy", "test", "virtual"] {
            assert!(
                !text.contains(forbidden),
                "production backend kind '{text}' must not expose a test double"
            );
        }
    }
    assert_eq!(CatBackendKind::ALL.len(), 9);
}
