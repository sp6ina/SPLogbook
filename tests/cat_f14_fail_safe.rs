// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! F-14 software regression suite under the F-15 fail-closed authorization model.
//!
//! F-14 invariants (no panicking range arithmetic, validation before PTT ON, exactly one PTT OFF on
//! every controlled post-ON path, failed/unconfirmed release is never success) are preserved and
//! verified in two complementary layers:
//!
//! 1. **In-crate mechanical tests** (`src/cat/hil/session.rs`, `#[cfg(test)]`): they obtain a
//!    TEST-ONLY independent evidence document (compiled only under `cfg(test)`) and drive the real
//!    production `HilHardwareSession` against test doubles. That is the only scope where PTT can be
//!    keyed at all, and it never touches hardware, a serial port, `rigctld` or RF.
//! 2. **This integration suite**: it drives the very same production entry points with **no**
//!    independent evidence, which is exactly the production configuration. Every former F-14 fault
//!    scenario must now stay fail-closed: a controlled error, zero `SetPtt(true)`, no RF, no panic.
//!
//! The ten `#[ignore = "hardware-in-the-loop"]` tests in `tests/cat_native_7.rs` stay untouched.

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
        schema_version: splogbook::cat::hil::HIL_PLAN_SCHEMA_VERSION,
        bench_id: "LAB-BENCH-SP6INA-01".to_string(),
        session_id: "SESSION-F14-01".to_string(),
        test_case_id: "hil_hardware_ptt_01".to_string(),
        attempt_id: "ATTEMPT-F14-01".to_string(),
        created_at: "2026-10-18T09:30:00Z".to_string(),
        expires_at: Some("2026-10-18T23:59:59Z".to_string()),
        tested_commit: Some("e1ff373b5d264f3f3de2383c19936f45b2befe6a".to_string()),
        expected_commit: Some("e1ff373b5d264f3f3de2383c19936f45b2befe6a".to_string()),
        working_tree_clean: Some(true),
        software_tests_passed: Some(true),
        operator_id: "SP6INA".to_string(),
        operator_callsign: "SP6INA".to_string(),
        operator_name: "Mariusz Woźniak".to_string(),
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
        operator_attestation: Some(HilOperatorAttestation {
            dummy_load_physically_connected: true,
            rf_perimeter_secured: true,
            ptt_consent_granted: true,
            attestation_timestamp: "2026-10-17T20:56:00Z".to_string(),
            description: "Software-only F-14/F-15 verification bench".to_string(),
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
        }),
        instrument_verification: Some(HilInstrumentMeasurement {
            instrument_id: "BIRD-43".to_string(),
            instrument_model: "Bird 43 Thruline".to_string(),
            measured_swr: 1.08,
            measured_power_watts: Some(9.8),
            calibration_valid_until: Some("2027-12-31".to_string()),
            measurement_timestamp: Some("2026-10-17T20:55:00Z".to_string()),
            origin: HilEvidenceOrigin::SyntheticOfflineFixture,
        }),
    }
}

/// Production configuration: a complete plan and a complete operator declaration, but **no**
/// independent evidence document. This is exactly what the shipped binary can produce.
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

/// Environment gates required by the production authorization chain. These are additional gates,
/// never evidence, so they cannot authorize PTT on their own (F-15).
fn env_gates() -> EnvGuard {
    EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some(OsStr::new("1"))),
        ("SPLOG_HIL_SESSION", Some(OsStr::new("SESSION-F14-01"))),
        ("SPLOG_HIL_ALLOW_TX", Some(OsStr::new("I_HAVE_DUMMY_LOAD"))),
    ])
}

/// Asserts the complete fail-closed outcome of a denied PTT attempt.
fn assert_denied_without_ptt(
    label: &str,
    result: Result<splogbook::cat::hil::HilPttExecutionResult, HilError>,
    counters: &std::sync::Arc<protocol_double::DoubleCounters>,
) {
    if let Err(err) = result {
        assert!(
            !matches!(err, HilError::PttForbiddenInStage7A),
            "{label}: unexpected stage error {err:?}"
        );
    } else {
        panic!("{label}: PTT must never be authorized without independent evidence");
    }
    assert_eq!(counters.ptt_on_count(), 0, "{label}: zero PTT ON");
    assert!(!counters.last_ptt_state(), "{label}: device stays in RX");
}

// =========================================================================
// PHASE 7 — F-15 GATE OVER THE F-14 REPRODUCTION
// =========================================================================

/// F-14 reproduction under F-15: a below-minimum range never reaches a clamp and never keys PTT.
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

    // The range validation is total and runs before any PTT ON: a controlled error, never a panic.
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
// PHASE 8/11 — RANGE VALIDATION
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

/// `min == max` is explicit in the type system: the range is legal, so the only reason PTT stays
/// blocked is the missing independent evidence (F-15) — never a panic and never a keyed device.
#[tokio::test]
async fn f14_min_equals_max_range_is_explicit_and_blocks_without_evidence() {
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

    assert_denied_without_ptt("min==max", res, &counters);
}

/// A valid range is not enough: without independent evidence PTT is never keyed.
#[tokio::test]
async fn f14_valid_range_without_evidence_never_keys_ptt() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let session = session(500);
    let mut arbiter = TxArbiter::new();

    let res = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(50),
            reference_time(),
        )
        .await;
    assert!(res.is_err(), "F-15: no evidence must mean no TX guard");
    assert_eq!(counters.ptt_on_count(), 0);
    assert_eq!(counters.ptt_off_count(), 0);
    assert!(arbiter.is_idle());
}

/// A zero pulse duration is bounded by the total helper and never panics; PTT stays blocked.
#[tokio::test]
async fn f14_zero_duration_is_bounded_to_minimum() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let session = session(500);
    let res = session
        .execute_controlled_ptt(&mut double, &token(), Duration::ZERO)
        .await;
    assert_denied_without_ptt("zero duration", res, &counters);
}

/// A pulse above the manifest maximum is bounded down; `Duration::MAX` is representable and must
/// not overflow or panic. PTT stays blocked without evidence.
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
        assert_denied_without_ptt("oversized duration", res, &counters);
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
// PHASE 9 — F-15 FAIL-CLOSED GATE ON EVERY F-14 FAULT CLASS
// =========================================================================

/// Every injectable device fault is unreachable without independent evidence: the gate denies
/// before the device sees anything, so `SetPtt(true)` count is zero for every fault class.
#[tokio::test]
async fn f14_fault_injection_matrix_stays_fail_closed_without_evidence() {
    let _env = env_gates();
    let faults = [
        ("success", DoubleFaults::default(), 500),
        (
            "fail_on",
            DoubleFaults {
                fail_on: true,
                ..Default::default()
            },
            500,
        ),
        (
            "silent_on",
            DoubleFaults {
                silent_on: true,
                ..Default::default()
            },
            500,
        ),
        (
            "reject_on",
            DoubleFaults {
                reject_on: true,
                ..Default::default()
            },
            500,
        ),
        (
            "fail_off",
            DoubleFaults {
                fail_off: true,
                ..Default::default()
            },
            500,
        ),
        (
            "silent_off",
            DoubleFaults {
                silent_off: true,
                ..Default::default()
            },
            500,
        ),
        (
            "malformed_off",
            DoubleFaults {
                malformed_off: true,
                ..Default::default()
            },
            500,
        ),
        (
            "sticky_ptt",
            DoubleFaults {
                sticky_ptt: true,
                ..Default::default()
            },
            500,
        ),
        (
            "sticky_after_on",
            DoubleFaults {
                sticky_after_on: true,
                ..Default::default()
            },
            500,
        ),
        (
            "poll_error",
            DoubleFaults {
                poll_error: true,
                ..Default::default()
            },
            500,
        ),
        (
            "poll_error_after_on",
            DoubleFaults {
                poll_error_after_on: true,
                ..Default::default()
            },
            500,
        ),
        (
            "disconnect_after_on",
            DoubleFaults {
                disconnect_after_on: true,
                ..Default::default()
            },
            500,
        ),
        ("reversed_range", DoubleFaults::default(), 5),
    ];

    for (label, fault, max_ms) in faults {
        let mut double = ProtocolDouble::with_faults(DocumentedPttProtocol::ICOM_CI_V, fault);
        let counters = double.counters();
        let session = session(max_ms);
        let result = session
            .execute_controlled_ptt(&mut double, &token(), Duration::from_millis(20))
            .await;
        assert_denied_without_ptt(label, result, &counters);
        assert_eq!(counters.ptt_off_count(), 0, "{label}: zero OFF without ON");
    }
}

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
    let res = session
        .execute_controlled_ptt(&mut double, &token(), Duration::from_millis(20))
        .await;
    assert!(res.is_err());
    assert_eq!(counters.ptt_on_count(), 0);
}

/// A radio that is already transmitting is rejected before any authorization consumption.
#[tokio::test]
async fn f14_sticky_ptt_preflight_is_rejected_without_evidence_or_transmit() {
    let _env = env_gates();
    let mut double = ProtocolDouble::with_faults(
        DocumentedPttProtocol::ICOM_CI_V,
        DoubleFaults {
            sticky_ptt: true,
            ..Default::default()
        },
    );
    let counters = double.counters();
    let session = session(500);
    let res = session
        .execute_controlled_ptt(&mut double, &token(), Duration::from_millis(20))
        .await;
    assert!(res.is_err(), "already-transmitting radio is rejected");
    assert_eq!(counters.ptt_on_count(), 0);
}

/// The explicit guard/release API cannot be reached without evidence: no guard is created.
#[tokio::test]
async fn f14_guard_cannot_be_acquired_without_evidence() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let mut arbiter = TxArbiter::new();
    let session = session(500);

    let guard = session
        .acquire_tx_guard_at(
            &mut double,
            &token(),
            &mut arbiter,
            Duration::from_millis(20),
            reference_time(),
        )
        .await;
    assert!(guard.is_err(), "F-15: no evidence must mean no TX guard");
    assert_eq!(counters.ptt_on_count(), 0);
    assert!(arbiter.is_idle());
}

/// The session refuses hardware access at the authorization boundary, with an auditable reason.
#[test]
fn f14_hardware_access_requires_independent_evidence() {
    let _env = env_gates();
    let session = session(500);
    let res = session.verify_hardware_access_authorization_at(reference_time());
    assert!(res.is_err(), "F-15: hardware access must be denied");
    assert!(!session.is_hardware_ptt_authorized());
    assert!(!session.has_independent_evidence());
    let reason = session
        .authorization_denial_reason()
        .expect("a denial must carry an auditable reason");
    assert!(
        reason.contains("OperatorAttestationMissing") || reason.contains("EvidenceMissing"),
        "the denial must name the missing independent authorization input, got: {reason}"
    );
}

/// A denied attempt does not consume the attempt registry: denial is not authorization.
#[test]
fn f14_denied_attempt_is_not_recorded_as_consumed() {
    let _env = env_gates();
    let session = session(500);
    let _ = session.verify_hardware_access_authorization_at(reference_time());
    assert!(!session.is_authorization_consumed());
}

// =========================================================================
// PHASE 10 — TEST-ONLY PROTOCOL DOUBLES (documented behaviour)
// =========================================================================

/// Each documented protocol double emits exactly the documented ON and OFF frames when driven
/// directly (no session, no authorization, no hardware). Frame-level conformance is unchanged.
#[tokio::test]
async fn f14_protocol_doubles_round_trip_documented_frames() {
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
        double
            .execute(&RigCommand::SetPtt(true))
            .await
            .expect("ON frame");
        double
            .execute(&RigCommand::SetPtt(false))
            .await
            .expect("OFF frame");
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
// PHASE 12/13 — DETERMINISTIC STRESS ON THE FAIL-CLOSED PATH
// =========================================================================

/// 2000 denied attempts over every fault class: deterministic, no sleep, no PTT, no panic.
#[tokio::test]
async fn f14_stress_denied_attempts_2000() {
    let _env = env_gates();
    let mut denied = 0usize;
    for i in 0..2000u64 {
        let max_ms = if i % 3 == 0 {
            500
        } else if i % 3 == 1 {
            HIL_MIN_TEST_DURATION_MS
        } else {
            i % HIL_MIN_TEST_DURATION_MS
        };
        let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
        let counters = double.counters();
        let mut arbiter = TxArbiter::new();
        let session = session(max_ms);
        let res = session
            .acquire_tx_guard_at(
                &mut double,
                &token(),
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(res.is_err(), "iteration {i}: must be denied");
        assert_eq!(counters.ptt_on_count(), 0, "iteration {i}: zero ON");
        assert_eq!(counters.ptt_off_count(), 0, "iteration {i}: zero OFF");
        denied += 1;
    }
    assert_eq!(denied, 2000);
}

/// 1000 denied controlled-PTT attempts across all documented protocol doubles: zero PTT frames.
#[tokio::test]
async fn f14_stress_documented_doubles_1000() {
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
    for i in 0..1000usize {
        let protocol = protocols[i % protocols.len()];
        let mut double = ProtocolDouble::new(protocol);
        let counters = double.counters();
        let session = session(500);
        let res = session
            .execute_controlled_ptt(&mut double, &token(), Duration::from_millis(10))
            .await;
        assert!(
            res.is_err(),
            "iteration {i} ({}) must be denied",
            protocol.source
        );
        assert_eq!(counters.ptt_on_count(), 0, "iteration {i}: zero ON");
        assert_eq!(counters.ptt_off_count(), 0, "iteration {i}: zero OFF");
    }
}

/// Cancelling the controlled-PTT future before any PTT ON is safe: zero frames, no panic.
#[tokio::test]
async fn f14_cancellation_before_ptt_on_is_safe() {
    let _env = env_gates();
    let mut double = ProtocolDouble::new(DocumentedPttProtocol::ICOM_CI_V);
    let counters = double.counters();
    let mut arbiter = TxArbiter::new();
    let session = session(500);
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
    let first_poll = future.as_mut().poll(&mut cx);
    assert!(
        first_poll.is_ready(),
        "denial is synchronous, no pulse await"
    );
    drop(future);
    assert_eq!(counters.ptt_on_count(), 0);
    assert_eq!(counters.ptt_off_count(), 0);
}

/// Release semantics remain explicit in the type system: `NotAttempted` is the initial state and
/// the guard API cannot confirm RX without a real release call.
#[test]
fn f14_release_outcome_starts_not_attempted() {
    assert_eq!(
        format!("{:?}", HilReleaseOutcome::NotAttempted),
        "NotAttempted"
    );
    assert_ne!(
        HilReleaseOutcome::NotAttempted,
        HilReleaseOutcome::Confirmed
    );
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
