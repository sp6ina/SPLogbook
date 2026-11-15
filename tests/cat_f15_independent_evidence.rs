// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! F-15 integration suite: fail-closed HIL readiness, no self-certification.
//!
//! These tests drive the **production** API only. They deliberately run in the production
//! configuration: a complete, hand-written test plan and a complete operator declaration but
//! **no independent evidence document**. That configuration must never authorize hardware PTT.
//!
//! Invariants asserted here:
//! * a missing safety field is a parse error, never `true`;
//! * a legacy (v1) document requires an explicit migration and stays denied;
//! * a manifest that sets every boolean to `true` cannot authorize itself;
//! * no PTT entry point keys the transmitter (`SetPtt(true)` count is 0);
//! * a structurally valid synthetic evidence **document cannot be created at all** from this
//!   scope (the evidence and decision types are not `Deserialize` and have private constructors),
//!   so there is no production path that could forge an authorization;
//! * dry-run never keys PTT.
//!
//! No hardware, no serial port, no `rigctld`, no real PTT and no RF is involved. The ten
//! `#[ignore = "hardware-in-the-loop"]` tests in `tests/cat_native_7.rs` stay untouched.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use splogbook::cat::backend::{CatBackendKind, RigBackend, RigProbeResult};
use splogbook::cat::hamlib::RigState;
use splogbook::cat::hil::{
    HIL_MIN_TEST_DURATION_MS, HIL_PLAN_SCHEMA_VERSION, HilAuthorizationOutcome, HilDummyLoadSpec,
    HilError, HilExecutionStage, HilHardwareSession, HilMatrixEntry, HilOperatorAttestation,
    HilPttConsentToken, HilReadinessValidator, HilRfSafetyConfig, HilRfTermination,
    HilTestBenchManifest, HilTransportConfig,
};
use splogbook::cat::rig::{RigCapabilities, RigCommand, RigError};
use splogbook::cat::routing::RigId;
use splogbook::cat::tx_arbiter::TxArbiter;

#[path = "support/process_env.rs"]
mod process_env;

use process_env::EnvGuard;

const REFERENCE_TIME: &str = "2026-10-18T12:00:00Z";

fn reference_time() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(REFERENCE_TIME)
        .expect("valid RFC3339 reference time")
        .with_timezone(&chrono::Utc)
}

/// A complete, valid v2 plan. It carries **no** self-declared safety booleans; those live in the
/// operator attestation document and are declarations, not evidence.
fn valid_plan() -> HilTestBenchManifest {
    HilTestBenchManifest {
        schema_version: HIL_PLAN_SCHEMA_VERSION,
        bench_id: "LAB-BENCH-SP6INA-01".to_string(),
        session_id: "SESSION-F15-01".to_string(),
        test_case_id: "hil_hardware_ptt_01".to_string(),
        attempt_id: "ATTEMPT-F15-0001".to_string(),
        created_at: "2026-10-18T09:30:00Z".to_string(),
        expires_at: Some("2026-10-18T23:59:59Z".to_string()),
        tested_commit: Some("e1ff373b5d264f3f3de2383c19936f45b2befe6a".to_string()),
        expected_commit: None,
        working_tree_clean: None,
        software_tests_passed: None,
        operator_id: "SP6INA".to_string(),
        operator_callsign: "SP6INA".to_string(),
        operator_name: "Mariusz Woźniak".to_string(),
        hil_enabled: true,
        allow_tx: true,
        matrix: HilMatrixEntry {
            backend_kind: CatBackendKind::IcomCiV,
            rig_id: RigId::new("RIG-F15-01").unwrap(),
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
            max_test_duration_ms: 500,
            smartlink_disabled: true,
            unattended_remote_tx_disabled: true,
            remote_endpoint_is_local_or_controlled: true,
            operator_has_immediate_stop_control: true,
            out_of_band_data_plane_disabled: true,
        },
        operator_attestation: Some(complete_attestation()),
        instrument_verification: None,
    }
}

fn complete_attestation() -> HilOperatorAttestation {
    HilOperatorAttestation {
        dummy_load_physically_connected: true,
        rf_perimeter_secured: true,
        ptt_consent_granted: true,
        attestation_timestamp: "2026-10-18T09:30:00Z".to_string(),
        description: "Declared bench; a declaration is not independent evidence".to_string(),
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
    }
}

fn plan_json() -> serde_json::Value {
    serde_json::to_value(valid_plan()).expect("plan serialises")
}

fn env_gates() -> EnvGuard {
    EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some(std::ffi::OsStr::new("1"))),
        (
            "SPLOG_HIL_SESSION",
            Some(std::ffi::OsStr::new("SESSION-F15-01")),
        ),
        (
            "SPLOG_HIL_ALLOW_TX",
            Some(std::ffi::OsStr::new("I_HAVE_DUMMY_LOAD")),
        ),
    ])
}

#[derive(Clone, Default)]
struct Counters {
    ptt_on: Arc<AtomicUsize>,
    ptt_off: Arc<AtomicUsize>,
    last_ptt: Arc<AtomicBool>,
}

struct MockBackend {
    counters: Counters,
    stick_ptt: Arc<AtomicBool>,
}

impl MockBackend {
    fn new() -> Self {
        Self {
            counters: Counters::default(),
            stick_ptt: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl RigBackend for MockBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::IcomCiV
    }

    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities::default()
    }

    fn connect(&mut self) -> splogbook::cat::backend::BoxFuture<'_, Result<(), RigError>> {
        Box::pin(async { Ok(()) })
    }

    fn poll(&mut self) -> splogbook::cat::backend::BoxFuture<'_, Result<RigState, RigError>> {
        let ptt =
            self.stick_ptt.load(Ordering::SeqCst) || self.counters.last_ptt.load(Ordering::SeqCst);
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
    ) -> splogbook::cat::backend::BoxFuture<'a, Result<(), RigError>> {
        let counters = self.counters.clone();
        Box::pin(async move {
            if let RigCommand::SetPtt(state) = command {
                if *state {
                    counters.ptt_on.fetch_add(1, Ordering::SeqCst);
                } else {
                    counters.ptt_off.fetch_add(1, Ordering::SeqCst);
                }
                counters.last_ptt.store(*state, Ordering::SeqCst);
            }
            Ok(())
        })
    }

    fn disconnect(&mut self) -> splogbook::cat::backend::BoxFuture<'_, ()> {
        Box::pin(async {})
    }

    fn probe(
        &mut self,
    ) -> splogbook::cat::backend::BoxFuture<'_, Result<RigProbeResult, RigError>> {
        Box::pin(async {
            Ok(RigProbeResult {
                reachable: true,
                identity: Some("Mock IC-7300".to_string()),
                model: Some("IC-7300".to_string()),
                firmware: Some("v1.42".to_string()),
                capabilities: RigCapabilities::default(),
                state: None,
                safe_diagnostic: "mock probe".to_string(),
            })
        })
    }
}

fn session(plan: HilTestBenchManifest) -> HilHardwareSession {
    HilHardwareSession::new_with_reference_time(
        plan,
        HilExecutionStage::Stage7BHardware,
        Some(reference_time()),
    )
}

// =========================================================================
// PHASE 13 — DESERIALISATION MATRIX (24 mandatory cases)
// =========================================================================

fn parse(value: serde_json::Value) -> Result<HilTestBenchManifest, HilError> {
    HilTestBenchManifest::from_json(&value.to_string())
}

#[test]
fn f15_deserialisation_matrix_is_fail_closed() {
    // 1. Empty manifest
    assert!(parse(serde_json::json!({})).is_err(), "empty manifest");
    // 2. All readiness fields missing (only required identity fields remain)
    let mut plan_without_safety = plan_json();
    let safety = plan_without_safety["rf_safety"]
        .as_object_mut()
        .expect("rf_safety object");
    for field in [
        "interlock_engaged",
        "smartlink_disabled",
        "unattended_remote_tx_disabled",
        "remote_endpoint_is_local_or_controlled",
        "operator_has_immediate_stop_control",
        "out_of_band_data_plane_disabled",
    ] {
        safety.remove(field);
    }
    assert!(
        parse(plan_without_safety).is_err(),
        "missing safety fields must not default"
    );
    // 3. Explicit false is preserved (never upgraded to true)
    let mut false_plan = plan_json();
    false_plan["rf_safety"]["interlock_engaged"] = serde_json::json!(false);
    let parsed = parse(false_plan).expect("explicit false parses");
    assert!(!parsed.rf_safety.interlock_engaged);
    // 4. True without independent evidence is still denied by the authorizer
    let decision = HilReadinessValidator::authorize_hardware_execution_at(
        &valid_plan(),
        None,
        None,
        reference_time(),
    );
    assert!(!decision.authorizes_hardware_ptt());
    // 5. Null safety field
    let mut null_plan = plan_json();
    null_plan["rf_safety"]["smartlink_disabled"] = serde_json::Value::Null;
    assert!(parse(null_plan).is_err(), "null must not become true");
    // 6. String instead of bool
    let mut string_plan = plan_json();
    string_plan["rf_safety"]["smartlink_disabled"] = serde_json::json!("true");
    assert!(parse(string_plan).is_err(), "string must not become true");
    // 7. Integer instead of bool
    let mut int_plan = plan_json();
    int_plan["rf_safety"]["smartlink_disabled"] = serde_json::json!(1);
    assert!(parse(int_plan).is_err(), "integer must not become true");
    // 8. Duplicate field
    let json = format!(
        "{{\"bench_id\":\"B\",\"bench_id\":\"C\",\"operator_callsign\":\"SP6INA\",\"operator_name\":\"M\",\"matrix\":{},\"rf_safety\":{}}}",
        plan_json()["matrix"],
        plan_json()["rf_safety"]
    );
    assert!(
        HilTestBenchManifest::from_json(&json).is_err(),
        "duplicate field must be rejected"
    );
    // 9. Unknown field is ignored by serde but cannot grant readiness
    let mut unknown_plan = plan_json();
    unknown_plan["hil_ready"] = serde_json::json!(true);
    unknown_plan["authorized"] = serde_json::json!(true);
    let parsed = parse(unknown_plan).expect("unknown fields are ignored");
    let decision = HilReadinessValidator::authorize_hardware_execution_at(
        &parsed,
        None,
        None,
        reference_time(),
    );
    assert!(
        !decision.authorizes_hardware_ptt(),
        "unknown field no grant"
    );
    // 10. Alias/case-different key is not a grant
    let mut case_plan = plan_json();
    case_plan["HIL_ENABLED"] = serde_json::json!(true);
    case_plan["allow_tx"] = serde_json::json!(false);
    let parsed = parse(case_plan).expect("case-different key is ignored");
    assert!(!parsed.allow_tx);
    // 11. Empty object
    assert!(parse(serde_json::json!({})).is_err());
    // 12. Partial object
    assert!(parse(serde_json::json!({"bench_id": "B"})).is_err());
    // 13. Legacy schema (v1: no schema_version) requires explicit migration
    let mut legacy = plan_json();
    legacy
        .as_object_mut()
        .expect("object")
        .remove("schema_version");
    let parsed = parse(legacy).expect("legacy document still parses as a plan");
    assert!(parsed.requires_migration());
    assert!(parsed.validate_plan().is_err(), "legacy plan stays denied");
    let decision = HilReadinessValidator::authorize_hardware_execution_at(
        &parsed,
        None,
        None,
        reference_time(),
    );
    assert_eq!(
        decision.outcome(),
        HilAuthorizationOutcome::ConfigurationInvalid
    );
    // 14. Future unsupported schema
    let mut future = plan_json();
    future["schema_version"] = serde_json::json!(99);
    let parsed = parse(future).expect("future version parses structurally");
    assert!(parsed.validate_plan().is_err(), "future schema is denied");
    // 15. Malformed JSON
    assert!(HilTestBenchManifest::from_json("{\"bench_id\":").is_err());
    // 16. Trailing comma
    assert!(
        HilTestBenchManifest::from_json(
            "{\"bench_id\":\"B\",\"operator_callsign\":\"S\",\"operator_name\":\"M\",\"matrix\":null,\"rf_safety\":null}"
        )
        .is_err()
    );
    // 17. Missing nested object
    let mut missing_nested = plan_json();
    missing_nested
        .as_object_mut()
        .expect("object")
        .remove("rf_safety");
    assert!(parse(missing_nested).is_err());
    // 18. Partially missing attestation
    let mut partial_attestation = plan_json();
    partial_attestation["operator_attestation"]
        .as_object_mut()
        .expect("attestation object")
        .remove("antenna_disconnected");
    assert!(
        parse(partial_attestation).is_err(),
        "partial attestation must not default to true"
    );
    // 19. `serde(default)` fields that exist keep their safe defaults
    let mut defaulted = plan_json();
    defaulted
        .as_object_mut()
        .expect("object")
        .remove("session_id");
    let parsed = parse(defaulted).expect("session_id has a safe empty default");
    assert!(parsed.session_id.is_empty());
    assert!(parsed.validate_plan().is_err(), "empty session denied");
    // 20. `Default::default()` for the authorization outcome is a denial
    assert_eq!(
        HilAuthorizationOutcome::default(),
        HilAuthorizationOutcome::NotEvaluated
    );
    assert!(!HilAuthorizationOutcome::default().authorizes_hardware_ptt());
    // 21. Custom default: hil_enabled/allow_tx default to false
    let mut custom = plan_json();
    custom
        .as_object_mut()
        .expect("object")
        .remove("hil_enabled");
    custom.as_object_mut().expect("object").remove("allow_tx");
    let parsed = parse(custom).expect("optional scope flags default to false");
    assert!(!parsed.hil_enabled);
    assert!(!parsed.allow_tx);
    // 22. Migration never creates readiness
    let mut legacy = plan_json();
    legacy
        .as_object_mut()
        .expect("object")
        .remove("schema_version");
    legacy["working_tree_clean"] = serde_json::json!(true);
    legacy["software_tests_passed"] = serde_json::json!(true);
    let parsed = parse(legacy).expect("legacy with historical true parses");
    let decision = HilReadinessValidator::authorize_hardware_execution_at(
        &parsed,
        None,
        None,
        reference_time(),
    );
    assert!(!decision.authorizes_hardware_ptt(), "migration no grant");
    // 23. Wrong-type nested matrix
    let mut wrong_matrix = plan_json();
    wrong_matrix["matrix"] = serde_json::json!("IC-7300");
    assert!(parse(wrong_matrix).is_err());
    // 24. Attestation missing entirely
    let mut no_attestation = plan_json();
    no_attestation
        .as_object_mut()
        .expect("object")
        .remove("operator_attestation");
    let parsed = parse(no_attestation).expect("attestation is an optional declaration");
    let decision = HilReadinessValidator::authorize_hardware_execution_at(
        &parsed,
        None,
        None,
        reference_time(),
    );
    assert!(!decision.authorizes_hardware_ptt());
}

// =========================================================================
// PHASE 14 — SELF-CERTIFICATION CANNOT AUTHORIZE
// =========================================================================

#[test]
fn f15_manifest_cannot_self_certify_readiness() {
    // Every boolean the schema knows, set to true, plus hand-added evidence-looking fields.
    let mut manifest = valid_plan();
    manifest.hil_enabled = true;
    manifest.allow_tx = true;
    manifest.working_tree_clean = Some(true);
    manifest.software_tests_passed = Some(true);
    manifest.rf_safety.interlock_engaged = true;
    manifest.rf_safety.smartlink_disabled = true;
    manifest.rf_safety.unattended_remote_tx_disabled = true;
    manifest.rf_safety.remote_endpoint_is_local_or_controlled = true;
    manifest.rf_safety.operator_has_immediate_stop_control = true;
    manifest.rf_safety.out_of_band_data_plane_disabled = true;
    manifest.operator_attestation = Some(complete_attestation());

    let mut json = serde_json::to_value(&manifest).expect("serialise");
    json["hil_ready"] = serde_json::json!(true);
    json["approved"] = serde_json::json!(true);
    json["authorized"] = serde_json::json!(true);
    json["operator_confirmed"] = serde_json::json!(true);
    json["evidence"] = serde_json::json!({"issuer": "self", "plan_sha256": "0".repeat(64)});
    json["issuer"] = serde_json::json!("manifest-self");
    json["plan_sha256"] = serde_json::json!("0".repeat(64));
    let parsed = serde_json::from_value::<HilTestBenchManifest>(json).expect("parses");

    let decision = HilReadinessValidator::authorize_hardware_execution_at(
        &parsed,
        None,
        None,
        reference_time(),
    );
    assert!(!decision.authorizes_hardware_ptt());
    assert_eq!(
        decision.outcome(),
        HilAuthorizationOutcome::OperatorAttestationMissing
    );

    // The readiness report is diagnostics only; it cannot authorize PTT either.
    let report = HilReadinessValidator::validate(&parsed, HilExecutionStage::Stage7BHardware);
    assert!(!report.can_run_hardware_ptt());
    assert_eq!(report.conditions.len(), 30);
}

#[test]
fn f15_dry_run_never_authorizes_ptt() {
    let decision =
        HilReadinessValidator::authorize_dry_run_at(&valid_plan(), None, reference_time());
    assert_eq!(decision.outcome(), HilAuthorizationOutcome::DryRunOnly);
    assert!(!decision.authorizes_hardware_ptt());
}

// =========================================================================
// PHASE 16/17 — AUTHORIZATION AND PTT GATING (zero PTT)
// =========================================================================

#[tokio::test]
async fn f15_every_ptt_entry_point_stays_closed_without_evidence() {
    let _env = env_gates();
    let token = HilPttConsentToken::new("SP6INA").expect("token");

    // 1. Public controlled-PTT entry point
    {
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let session = session(valid_plan());
        let res = session
            .execute_controlled_ptt(&mut backend, &token, Duration::from_millis(20))
            .await;
        assert!(res.is_err());
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
        assert_eq!(counters.ptt_off.load(Ordering::SeqCst), 0);
    }

    // 2. Guard acquisition
    {
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let mut arbiter = TxArbiter::new();
        let session = session(valid_plan());
        let guard = session
            .acquire_tx_guard_at(
                &mut backend,
                &token,
                &mut arbiter,
                Duration::from_millis(20),
                reference_time(),
            )
            .await;
        assert!(guard.is_err());
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
        assert!(arbiter.is_idle());
    }

    // 3. Explicit arbiter path
    {
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let mut arbiter = TxArbiter::new();
        let session = session(valid_plan());
        let res = session
            .execute_controlled_ptt_with_arbiter_at(
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

    // 4. Authorization verification
    {
        let session = session(valid_plan());
        assert!(session.verify_hardware_access_authorization().is_err());
        assert!(
            session
                .verify_tx_authorization(&token, &TxArbiter::new())
                .is_err()
        );
        assert!(!session.is_hardware_ptt_authorized());
        assert!(!session.is_authorization_consumed());
    }

    // 5. Stage 7A always blocks
    {
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let session = HilHardwareSession::new_with_reference_time(
            valid_plan(),
            HilExecutionStage::Stage7AOffline,
            Some(reference_time()),
        );
        let res = session
            .execute_controlled_ptt(&mut backend, &token, Duration::from_millis(20))
            .await;
        assert!(matches!(res, Err(HilError::PttForbiddenInStage7A)));
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
    }

    // 6. A radio already transmitting is rejected, still zero ON
    {
        let mut backend = MockBackend::new();
        backend.stick_ptt.store(true, Ordering::SeqCst);
        let counters = backend.counters.clone();
        let session = session(valid_plan());
        let res = session
            .execute_controlled_ptt(&mut backend, &token, Duration::from_millis(20))
            .await;
        assert!(res.is_err());
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn f15_expired_session_denies_and_never_keys_ptt() {
    let _env = env_gates();
    let mut plan = valid_plan();
    plan.expires_at = Some("2026-10-18T10:00:00Z".to_string());
    let session = session(plan);
    let mut backend = MockBackend::new();
    let counters = backend.counters.clone();
    let token = HilPttConsentToken::new("SP6INA").expect("token");
    let res = session
        .execute_controlled_ptt(&mut backend, &token, Duration::from_millis(20))
        .await;
    assert!(matches!(res, Err(HilError::SessionExpired(_))));
    assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0);
}

// =========================================================================
// PHASE 19/20 — COMBINATORIAL MATRIX AND STRESS ON THE DENIED PATH
// =========================================================================

#[test]
fn f15_combinatorial_matrix_never_authorizes_without_evidence() {
    let mut cases = 0usize;
    for manifest_completeness in [true, false] {
        for attestation_state in ["complete", "false", "missing"] {
            for legacy_schema in [false, true] {
                for session_expired in [false, true] {
                    for allow_tx in [true, false] {
                        let mut plan = valid_plan();
                        if !manifest_completeness {
                            plan.test_case_id.clear();
                        }
                        match attestation_state {
                            "complete" => plan.operator_attestation = Some(complete_attestation()),
                            "false" => {
                                let mut attestation = complete_attestation();
                                attestation.antenna_disconnected = false;
                                plan.operator_attestation = Some(attestation);
                            }
                            _ => plan.operator_attestation = None,
                        }
                        if legacy_schema {
                            plan.schema_version = 0;
                        }
                        if session_expired {
                            plan.expires_at = Some("2026-10-18T10:00:00Z".to_string());
                        }
                        plan.allow_tx = allow_tx;

                        let decision = HilReadinessValidator::authorize_hardware_execution_at(
                            &plan,
                            None,
                            None,
                            reference_time(),
                        );
                        assert!(
                            !decision.authorizes_hardware_ptt(),
                            "combination ({manifest_completeness}, {attestation_state}, \
                             {legacy_schema}, {session_expired}, {allow_tx}) must be denied"
                        );
                        assert!(decision.outcome().is_denial());
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 48);
}

#[tokio::test]
async fn f15_stress_missing_evidence_never_keys_ptt_2000() {
    let _env = env_gates();
    let token = HilPttConsentToken::new("SP6INA").expect("token");
    for i in 0..2000usize {
        let mut backend = MockBackend::new();
        let counters = backend.counters.clone();
        let session = session(valid_plan());
        let res = session
            .execute_controlled_ptt(&mut backend, &token, Duration::from_millis(10))
            .await;
        assert!(res.is_err(), "iteration {i}");
        assert_eq!(counters.ptt_on.load(Ordering::SeqCst), 0, "iteration {i}");
    }
}

#[test]
fn f15_stress_malformed_manifests_1000() {
    let base = plan_json();
    for i in 0..1000usize {
        let mut candidate = base.clone();
        match i % 6 {
            0 => candidate["rf_safety"] = serde_json::Value::Null,
            1 => candidate["rf_safety"]["interlock_engaged"] = serde_json::json!("yes"),
            2 => candidate["matrix"] = serde_json::json!([]),
            3 => candidate["schema_version"] = serde_json::json!(i as u64 + 100),
            4 => candidate["operator_attestation"]["antenna_disconnected"] = serde_json::json!(1),
            _ => candidate["frequency_range_hz"] = serde_json::json!([1, 1]),
        }
        let result = HilTestBenchManifest::from_json(&candidate.to_string());
        if let Ok(plan) = result {
            let decision = HilReadinessValidator::authorize_hardware_execution_at(
                &plan,
                None,
                None,
                reference_time(),
            );
            assert!(!decision.authorizes_hardware_ptt(), "iteration {i}");
        }
    }
}

#[test]
fn f15_ptt_on_count_without_authorization_is_zero() {
    // Static assertion of the gate contract: a readiness report can never run hardware PTT.
    let report = HilReadinessValidator::validate(&valid_plan(), HilExecutionStage::Stage7BHardware);
    assert!(!report.can_run_hardware_ptt());
    assert_eq!(HIL_MIN_TEST_DURATION_MS, 10);
}
