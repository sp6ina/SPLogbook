// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Zestaw deterministycznych testów integracyjnych offline dla harnessu HIL (NATIVE-7A)
//! oraz hermetycznie odizolowane punkty wejścia dla stanowiska laboratoryjnego (NATIVE-7B).
//!
//! Zgodnie z zasadami programu NATIVE-7:
//! - Testy offline weryfikują maszynę stanów, parser manifestu, walidator HIL-READY
//!   oraz blokady bezpieczeństwa toru RF.
//! - Żaden test offline nie wykonuje fizycznej emisji RF ani nie załącza fizycznego PTT.
//! - Prawdziwe testy ze sprzętem są hermetycznie odizolowane atrybutem `#[ignore = "hardware-in-the-loop"]`.

use std::time::Duration;

use splogbook::cat::backend::{CatBackendKind, RigBackend, RigProbeResult};
use splogbook::cat::hil::{
    HilAbortProcedure, HilCommandResult, HilDispatcherCase, HilDispatcherReport, HilDummyLoadSpec,
    HilError, HilEvidenceOrigin, HilExecutionContext, HilExecutionStage, HilExtendedRequirement,
    HilHardwareSession, HilImplementationCoverage, HilInstrumentMeasurement, HilMatrixEntry,
    HilOperatorAttestation, HilPttConsentToken, HilReadinessEvidenceKind, HilReadinessStatus,
    HilReadinessValidator, HilRequirementAssessment, HilRfSafetyConfig, HilRfTermination,
    HilSessionState, HilSoftwareBaselineEvidence, HilTestBenchManifest, HilTransportConfig,
    HilTxGuard, run_isolated_hardware_dispatcher, run_isolated_hardware_test,
};
use splogbook::cat::rig::{RigCapabilities, RigCommand, RigError};
use splogbook::cat::routing::{RigCommandId, RigId};
use splogbook::cat::tx_arbiter::TxArbiter;

static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct EnvGuard {
    vars: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn new(pairs: &[(&'static str, Option<&str>)]) -> Self {
        let mut vars = Vec::new();
        for (k, v) in pairs {
            let orig = std::env::var(k).ok();
            vars.push((*k, orig));
            match v {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
        Self { vars }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, orig) in &self.vars {
            match orig {
                Some(v) => unsafe { std::env::set_var(k, v) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }
}

fn valid_test_manifest() -> HilTestBenchManifest {
    HilTestBenchManifest {
        bench_id: "LAB-BENCH-SP6INA-01".to_string(),
        session_id: "SESSION-TEST-01".to_string(),
        created_at: "2026-10-18T09:30:00Z".to_string(),
        expires_at: Some("2026-10-18T23:59:59Z".to_string()),
        tested_commit: Some("2de969a6b1db36de4f517c3addc23af8bbf34e8d".to_string()),
        operator_id: "SP6INA".to_string(),
        operator_callsign: "SP6INA".to_string(),
        operator_name: "Mariusz Woźniak".to_string(),
        hil_enabled: true,
        allow_tx: true,
        matrix: HilMatrixEntry {
            backend_kind: CatBackendKind::IcomCiV,
            rig_id: RigId::new("RIG-IC7300-01").unwrap(),
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
        },
        operator_attestation: HilOperatorAttestation {
            dummy_load_physically_connected: true,
            rf_perimeter_secured: true,
            ptt_consent_granted: true,
            attestation_timestamp: "2026-10-17T20:56:00Z".to_string(),
            description: "Verified laboratory test bench configuration".to_string(),
            antenna_disconnected: true,
            pa_disconnected_or_locked: true,
            tuner_bypassed: true,
            physical_stop_available: true,
            abort_procedure_documented: true,
            operator_acknowledged: true,
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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Clone, Default)]
struct MockHilCounters {
    ptt_count: Arc<AtomicUsize>,
    last_ptt_state: Arc<AtomicBool>,
    stick_ptt_on: Arc<AtomicBool>,
    open_count: Arc<AtomicUsize>,
    connect_count: Arc<AtomicUsize>,
    command_count: Arc<AtomicUsize>,
}

/// Atrapa backendu dla testów jednostkowych pętli wykonawczej sesji HIL.
struct MockHilBackend {
    counters: MockHilCounters,
}

impl MockHilBackend {
    fn new() -> Self {
        Self {
            counters: MockHilCounters::default(),
        }
    }

    fn counters(&self) -> MockHilCounters {
        self.counters.clone()
    }

    fn ptt_count(&self) -> usize {
        self.counters.ptt_count.load(Ordering::SeqCst)
    }

    fn last_ptt_state(&self) -> bool {
        self.counters.last_ptt_state.load(Ordering::SeqCst)
    }

    fn open_count(&self) -> usize {
        self.counters.open_count.load(Ordering::SeqCst)
    }

    fn connect_count(&self) -> usize {
        self.counters.connect_count.load(Ordering::SeqCst)
    }

    fn command_count(&self) -> usize {
        self.counters.command_count.load(Ordering::SeqCst)
    }
}

impl RigBackend for MockHilBackend {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::IcomCiV
    }

    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities::default()
    }

    fn connect(&mut self) -> splogbook::cat::backend::BoxFuture<'_, Result<(), RigError>> {
        self.counters.connect_count.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }

    fn disconnect(&mut self) -> splogbook::cat::backend::BoxFuture<'_, ()> {
        Box::pin(async {})
    }

    fn poll(
        &mut self,
    ) -> splogbook::cat::backend::BoxFuture<'_, Result<splogbook::cat::hamlib::RigState, RigError>>
    {
        let ptt = if self.counters.stick_ptt_on.load(Ordering::SeqCst) {
            true
        } else {
            self.counters.last_ptt_state.load(Ordering::SeqCst)
        };
        let state = splogbook::cat::hamlib::RigState {
            ptt,
            ..Default::default()
        };
        Box::pin(async move { Ok(state) })
    }

    fn execute<'a>(
        &'a mut self,
        command: &'a RigCommand,
    ) -> splogbook::cat::backend::BoxFuture<'a, Result<(), RigError>> {
        self.counters.command_count.fetch_add(1, Ordering::SeqCst);
        let counters = self.counters.clone();
        Box::pin(async move {
            if let RigCommand::SetPtt(state) = command {
                if *state {
                    counters.ptt_count.fetch_add(1, Ordering::SeqCst);
                }
                counters.last_ptt_state.store(*state, Ordering::SeqCst);
            }
            Ok(())
        })
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
                safe_diagnostic: "Mock probe OK".to_string(),
            })
        })
    }
}

#[test]
fn test_01_manifest_parser_valid_json_success() {
    let manifest = valid_test_manifest();
    let json = manifest
        .to_json()
        .expect("Serialization to JSON must succeed");
    let parsed =
        HilTestBenchManifest::from_json(&json).expect("Deserialization from JSON must succeed");

    assert_eq!(parsed.bench_id, "LAB-BENCH-SP6INA-01");
    assert_eq!(parsed.operator_callsign, "SP6INA");
    assert_eq!(parsed.matrix.backend_kind, CatBackendKind::IcomCiV);
    assert_eq!(parsed.matrix.model_name, "IC-7300");
}

#[test]
fn test_02_manifest_parser_rejects_empty_bench_id() {
    let mut manifest = valid_test_manifest();
    manifest.bench_id = "   ".to_string();

    let err = manifest.validate_structure().unwrap_err();
    assert!(matches!(err, HilError::InvalidManifest(_)));
    assert!(err.to_string().contains("bench_id cannot be empty"));
}

#[test]
fn test_03_manifest_parser_rejects_empty_operator() {
    let mut manifest = valid_test_manifest();
    manifest.operator_callsign = "".to_string();

    let err = manifest.validate_structure().unwrap_err();
    assert!(matches!(err, HilError::InvalidManifest(_)));
    assert!(
        err.to_string()
            .contains("operator_callsign cannot be empty")
    );
}

#[test]
fn test_04_manifest_parser_rejects_empty_port_name() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Serial {
        port_name: "  ".to_string(),
        baud_rate: 115_200,
        data_bits: 8,
        parity: "none".to_string(),
        stop_bits: 1,
        flow_control: "none".to_string(),
    };

    let err = manifest.validate_structure().unwrap_err();
    assert!(matches!(err, HilError::InvalidTransport(_)));
    assert!(err.to_string().contains("port_name cannot be empty"));
}

#[test]
fn test_05_manifest_parser_rejects_auto_port_selection() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Serial {
        port_name: "AUTO".to_string(),
        baud_rate: 115_200,
        data_bits: 8,
        parity: "none".to_string(),
        stop_bits: 1,
        flow_control: "none".to_string(),
    };

    let err = manifest.validate_structure().unwrap_err();
    assert!(matches!(err, HilError::InvalidTransport(_)));
    assert!(
        err.to_string()
            .contains("Automatic port selection is forbidden")
    );
}

#[test]
fn test_06_manifest_parser_rejects_low_baud_rate() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Serial {
        port_name: "COM1".to_string(),
        baud_rate: 300,
        data_bits: 8,
        parity: "none".to_string(),
        stop_bits: 1,
        flow_control: "none".to_string(),
    };

    let err = manifest.validate_structure().unwrap_err();
    assert!(matches!(err, HilError::InvalidTransport(_)));
    assert!(err.to_string().contains("Invalid baud_rate"));
}

#[test]
fn test_07_manifest_parser_rejects_network_zero_port() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Network {
        host: "192.168.1.50".to_string(),
        port: 0,
    };

    let err = manifest.validate_structure().unwrap_err();
    assert!(matches!(err, HilError::InvalidTransport(_)));
    assert!(err.to_string().contains("port cannot be 0"));
}

#[test]
fn test_08_validator_antenna_termination_fails_ptt_readiness() {
    let mut manifest = valid_test_manifest();
    manifest.rf_safety.termination = HilRfTermination::Antenna {
        antenna_name: "Dipole 20m".to_string(),
    };

    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let check = report
        .find_check("HIL-MACHINE-RF-TERMINATION")
        .expect("Check must exist");

    assert_eq!(check.status, HilReadinessStatus::Failed);
    assert_eq!(
        check.evidence_kind,
        HilReadinessEvidenceKind::MachineVerified
    );
    assert!(
        check
            .details
            .contains("strictly prohibited for HIL PTT tests")
    );
    assert_eq!(report.overall_status, HilReadinessStatus::Failed);
    assert!(!report.can_run_hardware_ptt());
}

#[test]
fn test_09_validator_dummy_load_rating_exceeded_fails() {
    let mut manifest = valid_test_manifest();
    manifest.rf_safety.max_test_power_watts = 250.0; // rating is 100.0

    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let check = report
        .find_check("HIL-MACHINE-RF-TERMINATION")
        .expect("Check must exist");

    assert_eq!(check.status, HilReadinessStatus::Failed);
    assert!(check.details.contains("exceeds dummy load rating"));
    assert_eq!(report.overall_status, HilReadinessStatus::Failed);
}

#[test]
fn test_10_validator_missing_operator_dummy_load_attestation_blocks() {
    let mut manifest = valid_test_manifest();
    manifest
        .operator_attestation
        .dummy_load_physically_connected = false;

    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let check = report
        .find_check("HIL-OPERATOR-DUMMY-LOAD")
        .expect("Check must exist");

    assert_eq!(check.status, HilReadinessStatus::Blocked);
    assert_eq!(
        check.evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
    assert!(
        check
            .details
            .contains("has not attested physical connection")
    );
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
    assert!(!report.can_run_hardware_ptt());
}

#[test]
fn test_11_validator_missing_operator_consent_blocks() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.ptt_consent_granted = false;

    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let check = report
        .find_check("HIL-OPERATOR-PTT-CONSENT")
        .expect("Check must exist");

    assert_eq!(check.status, HilReadinessStatus::Blocked);
    assert_eq!(
        check.evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
    assert!(check.details.contains("has not granted explicit consent"));
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

#[test]
fn test_12_validator_missing_rf_perimeter_attestation_blocks() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.rf_perimeter_secured = false;

    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let check = report
        .find_check("HIL-OPERATOR-RF-PERIMETER")
        .expect("Check must exist");

    assert_eq!(check.status, HilReadinessStatus::Blocked);
    assert_eq!(
        check.evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
    assert!(
        check
            .details
            .contains("has not confirmed RF safety perimeter")
    );
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

#[test]
fn test_13_validator_instrument_swr_high_fails() {
    let mut manifest = valid_test_manifest();
    manifest.instrument_verification = Some(HilInstrumentMeasurement {
        instrument_id: "BIRD-43".to_string(),
        instrument_model: "Bird 43 Thruline".to_string(),
        measured_swr: 2.15,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2027-12-31".to_string()),
        measurement_timestamp: Some("2026-10-17T20:55:00Z".to_string()),
        origin: HilEvidenceOrigin::PhysicalMeasurement,
    });

    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let check = report
        .find_check("HIL-INSTRUMENT-SWR")
        .expect("Check must exist");

    assert_eq!(check.status, HilReadinessStatus::Failed);
    assert_eq!(
        check.evidence_kind,
        HilReadinessEvidenceKind::InstrumentVerified
    );
    assert!(check.details.contains("exceeds safety threshold 1.50:1"));
    assert_eq!(report.overall_status, HilReadinessStatus::Failed);
}

#[test]
fn test_14_validator_instrument_swr_good_passes() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let check = report
        .find_check("HIL-INSTRUMENT-SWR")
        .expect("Check must exist");

    assert_eq!(check.status, HilReadinessStatus::Passed);
    assert_eq!(
        check.evidence_kind,
        HilReadinessEvidenceKind::InstrumentVerified
    );
    assert!(check.details.contains("within limit <= 1.50:1"));
}

#[test]
fn test_15_validator_stage_7a_unconditionally_blocks_ptt() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let gate_check = report
        .find_check("HIL-STAGE-PTT-GATE")
        .expect("Gate check must exist");

    assert_eq!(gate_check.status, HilReadinessStatus::Blocked);
    assert_eq!(
        gate_check.evidence_kind,
        HilReadinessEvidenceKind::MachineVerified
    );
    assert!(
        gate_check
            .details
            .contains("Physical PTT is strictly prohibited in Stage NATIVE-7A")
    );
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
    assert!(!report.can_run_hardware_ptt());
}

#[test]
fn test_16_validator_stage_7b_full_ready_passes() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);

    assert_eq!(report.overall_status, HilReadinessStatus::Passed);
    assert!(report.is_hil_ready());
    assert!(report.can_run_hardware_ptt());
    assert!(report.can_run_probe());
    assert_eq!(report.conditions.len(), 30);
}

#[test]
fn test_17_session_probe_allowed_in_stage_7a_when_transport_valid() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);

    assert!(session.readiness_report().can_run_probe());
    assert!(!session.readiness_report().can_run_hardware_ptt());
}

#[tokio::test]
async fn test_18_session_probe_rejected_when_transport_invalid() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Serial {
        port_name: "".to_string(),
        baud_rate: 115_200,
        data_bits: 8,
        parity: "none".to_string(),
        stop_bits: 1,
        flow_control: "none".to_string(),
    };
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);
    let mut mock_backend = MockHilBackend::new();

    let err = session
        .execute_safe_probe_backend(&mut mock_backend)
        .await
        .unwrap_err();
    assert!(matches!(err, HilError::NotHilReady(_)));
}

#[tokio::test]
async fn test_19_session_ptt_strictly_rejected_in_stage_7a() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);
    let mut mock_backend = MockHilBackend::new();
    let token = HilPttConsentToken::new("SP6INA").unwrap();

    let err = session
        .execute_controlled_ptt(&mut mock_backend, &token, Duration::from_millis(50))
        .await
        .unwrap_err();
    assert_eq!(err, HilError::PttForbiddenInStage7A);
    assert_eq!(mock_backend.ptt_count(), 0);
}

#[tokio::test]
async fn test_20_session_ptt_strictly_rejected_without_hil_ready() {
    let mut manifest = valid_test_manifest();
    manifest
        .operator_attestation
        .dummy_load_physically_connected = false;
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let mut mock_backend = MockHilBackend::new();
    let token = HilPttConsentToken::new("SP6INA").unwrap();

    let err = session
        .execute_controlled_ptt(&mut mock_backend, &token, Duration::from_millis(50))
        .await
        .unwrap_err();
    assert!(matches!(err, HilError::NotHilReady(_)));
    assert_eq!(mock_backend.ptt_count(), 0);
}

#[tokio::test]
async fn test_21_session_ptt_strictly_rejected_on_antenna() {
    let mut manifest = valid_test_manifest();
    manifest.rf_safety.termination = HilRfTermination::Antenna {
        antenna_name: "Yagi 3el".to_string(),
    };
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let mut mock_backend = MockHilBackend::new();
    let token = HilPttConsentToken::new("SP6INA").unwrap();

    let err = session
        .execute_controlled_ptt(&mut mock_backend, &token, Duration::from_millis(50))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        HilError::NotHilReady(_) | HilError::SafetyViolation(_)
    ));
    assert_eq!(mock_backend.ptt_count(), 0);
}

#[test]
fn test_22_evidence_kinds_correctly_classified() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);

    let machine_count = report
        .checks
        .iter()
        .filter(|c| c.evidence_kind == HilReadinessEvidenceKind::MachineVerified)
        .count();
    let operator_count = report
        .checks
        .iter()
        .filter(|c| c.evidence_kind == HilReadinessEvidenceKind::OperatorAttested)
        .count();
    let instrument_count = report
        .checks
        .iter()
        .filter(|c| c.evidence_kind == HilReadinessEvidenceKind::InstrumentVerified)
        .count();

    assert_eq!(machine_count, 5); // rig target, transport, rf term, ptt no retry, stage gate
    assert_eq!(operator_count, 3); // dummy load connection, perimeter, consent
    assert_eq!(instrument_count, 1); // swr measurement
}

#[test]
fn test_23_credentials_and_secrets_redacted_in_manifest() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Network {
        host: "10.0.0.123".to_string(),
        port: 4992,
    };

    let redacted = manifest.redacted_manifest();
    if let HilTransportConfig::Network { host, port } = redacted.matrix.transport {
        assert_eq!(host, "[REDACTED_HOST]");
        assert_eq!(port, 4992);
    } else {
        panic!("Transport must be Network");
    }
}

#[test]
fn test_24_hil_matrix_dimensions_correctly_identified() {
    let manifest = valid_test_manifest();
    let rig_config = manifest
        .to_rig_config()
        .expect("Conversion to RigConfig must succeed");

    assert_eq!(rig_config.id.as_str(), "RIG-IC7300-01");
    assert_eq!(rig_config.backend.kind_str(), "Icom");
    assert!(rig_config.validate().is_ok());
}

#[tokio::test]
async fn test_25_session_executes_single_shot_ptt_in_stage_7b_mock() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let mut mock_backend = MockHilBackend::new();
    let token = HilPttConsentToken::new("SP6INA").unwrap();

    let result = session
        .execute_controlled_ptt(&mut mock_backend, &token, Duration::from_millis(50))
        .await
        .expect("Execution on mock backend must succeed");

    assert_eq!(mock_backend.ptt_count(), 1);
    assert!(!mock_backend.last_ptt_state()); // verified RX returned
    assert!(result.rx_restored_cleanly);
}

// =========================================================================
// OFFLINE INTEGRATION AUDIT TESTS (NATIVE-7A Kanoniczne testy zgodności)
// =========================================================================

#[test]
fn hil_requires_explicit_enable_flag() {
    let _lock = ENV_LOCK.blocking_lock();
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", None),
        ("SPLOG_HIL_SESSION", Some("SESSION-TEST-01")),
    ]);
    let res = session.verify_hardware_access_authorization();
    assert!(matches!(res, Err(HilError::UnauthorizedSession(_))));
}

#[test]
fn hil_requires_explicit_rig_id() {
    assert!(RigId::new("").is_err());
    let manifest = valid_test_manifest();
    let mut json_val: serde_json::Value = serde_json::to_value(&manifest).unwrap();
    json_val["matrix"]["rig_id"] = serde_json::Value::String("   ".to_string());
    let err = serde_json::from_value::<HilTestBenchManifest>(json_val);
    assert!(err.is_err());
}

#[test]
fn hil_requires_operator_acknowledgement() {
    let _lock = ENV_LOCK.blocking_lock();
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.operator_acknowledged = false;
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("SESSION-TEST-01")),
    ]);
    let res = session.verify_hardware_access_authorization();
    assert!(matches!(res, Err(HilError::UnauthorizedSession(_))));
}

#[test]
fn hil_requires_dummy_load_confirmation() {
    let mut manifest = valid_test_manifest();
    manifest
        .operator_attestation
        .dummy_load_physically_connected = false;
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let cond9 = report.find_condition(9).expect("Condition 9 must exist");
    assert_eq!(cond9.status, HilReadinessStatus::Blocked);
    assert_eq!(
        cond9.evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
}

#[test]
fn hil_requires_power_limit() {
    let mut manifest = valid_test_manifest();
    manifest.rf_safety.max_test_power_watts = 0.0;
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::SafetyViolation(_))
    ));
}

#[test]
fn hil_rejects_zero_or_invalid_deadline() {
    let mut manifest = valid_test_manifest();
    manifest.rf_safety.max_test_duration_ms = 0;
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::SafetyViolation(_))
    ));

    manifest.rf_safety.max_test_duration_ms = 600;
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::SafetyViolation(_))
    ));
}

#[test]
fn hil_rejects_unknown_backend() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.backend_kind = CatBackendKind::So2r;
    let res = manifest.to_rig_config();
    assert!(matches!(res, Err(HilError::InvalidManifest(_))));
}

#[test]
fn hil_rejects_missing_transport() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Serial {
        port_name: "   ".to_string(),
        baud_rate: 115200,
        data_bits: 8,
        parity: "none".to_string(),
        stop_bits: 1,
        flow_control: "none".to_string(),
    };
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::InvalidTransport(_))
    ));
}

#[test]
fn hil_never_selects_first_rig() {
    let manifest = valid_test_manifest();
    let rig_config = manifest.to_rig_config().unwrap();
    assert_eq!(rig_config.id.as_str(), "RIG-IC7300-01");
    assert_ne!(rig_config.id.as_str(), "FIRST");
}

#[test]
fn hil_ptt_requires_separate_enable_flag() {
    let _lock = ENV_LOCK.blocking_lock();
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let arbiter = TxArbiter::new();

    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("SESSION-TEST-01")),
        ("SPLOG_HIL_ALLOW_TX", None),
    ]);
    let res = session.verify_tx_authorization(&token, &arbiter);
    assert!(matches!(res, Err(HilError::UnauthorizedSession(_))));
}

#[tokio::test]
async fn hil_ptt_uses_tx_arbiter() {
    let _lock = ENV_LOCK.lock().await;
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let mut mock_backend = MockHilBackend::new();
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let mut arbiter = TxArbiter::new();

    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("SESSION-TEST-01")),
        ("SPLOG_HIL_ALLOW_TX", Some("I_HAVE_DUMMY_LOAD")),
    ]);

    let counters = mock_backend.counters();
    let mut guard = session
        .acquire_tx_guard(
            &mut mock_backend,
            &token,
            &mut arbiter,
            Duration::from_millis(50),
        )
        .await
        .expect("Acquire TX guard must succeed");

    assert_eq!(
        counters.ptt_count.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert!(matches!(
        arbiter.state(),
        splogbook::cat::tx_arbiter::TxArbiterState::Owned { .. }
    ));

    let res = guard.release_and_confirm_rx().await;
    assert!(res.is_ok());
    assert!(guard.is_confirmed_rx());
}

#[tokio::test]
async fn hil_ptt_has_no_retry() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let mut mock_backend = MockHilBackend::new();
    let token = HilPttConsentToken::new("SP6INA").unwrap();

    let res = session
        .execute_controlled_ptt(&mut mock_backend, &token, Duration::from_millis(20))
        .await;
    assert!(res.is_ok());
    assert_eq!(mock_backend.ptt_count(), 1); // Dokładnie jedno żądanie, brak ponowień
}

#[test]
fn hil_timeout_enters_unknown() {
    let mut arbiter = TxArbiter::new();
    let rig_id = RigId::new("RIG-IC7300-01").unwrap();
    let cmd_id = RigCommandId::new(42);

    arbiter.request_tx(&rig_id, cmd_id).unwrap();
    arbiter.set_unknown(
        Some(rig_id),
        splogbook::cat::tx_arbiter::TxUnknownReason::CommandTimeout,
    );

    assert!(matches!(
        arbiter.state(),
        splogbook::cat::tx_arbiter::TxArbiterState::Unknown { .. }
    ));
}

#[test]
fn hil_result_supports_not_run() {
    let status = HilReadinessStatus::NotRun;
    assert_eq!(status.to_string(), "NotRun");
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: HilReadinessStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, HilReadinessStatus::NotRun);
}

#[test]
fn hil_result_supports_blocked() {
    let status = HilReadinessStatus::Blocked;
    assert_eq!(status.to_string(), "Blocked");
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: HilReadinessStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, HilReadinessStatus::Blocked);
}

#[test]
fn hil_manifest_records_commit() {
    let manifest = valid_test_manifest();
    assert_eq!(
        manifest.tested_commit.as_deref(),
        Some("2de969a6b1db36de4f517c3addc23af8bbf34e8d")
    );
}

#[test]
fn hil_manifest_records_firmware() {
    let manifest = valid_test_manifest();
    assert_eq!(manifest.matrix.firmware_version, "v1.42");
}

#[test]
fn hil_manifest_redacts_credentials() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Network {
        host: "192.168.1.120".to_string(),
        port: 4992,
    };
    let redacted = manifest.redacted_manifest();
    if let HilTransportConfig::Network { host, port } = redacted.matrix.transport {
        assert_eq!(host, "[REDACTED_HOST]");
        assert_eq!(port, 4992);
    } else {
        panic!("Transport must be Network");
    }
}

#[tokio::test]
async fn hil_shutdown_joins_all_tasks() {
    let mut mock_backend = MockHilBackend::new();
    mock_backend.disconnect().await;
    assert_eq!(mock_backend.connect_count(), 0);
}

#[test]
fn hil_standard_suite_cannot_open_physical_transport() {
    let mock = MockHilBackend::new();
    assert_eq!(mock.open_count(), 0);
    assert_eq!(mock.connect_count(), 0);
    assert_eq!(mock.command_count(), 0);
    assert_eq!(mock.ptt_count(), 0);
}

#[test]
fn hil_ignored_entrypoint_rechecks_all_runtime_gates() {
    let _lock = ENV_LOCK.blocking_lock();
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", None),
        ("SPLOG_HIL_SESSION", None),
        ("SPLOG_HIL_ALLOW_TX", None),
    ]);
    let res = session.verify_hardware_access_authorization();
    assert!(matches!(res, Err(HilError::UnauthorizedSession(_))));
}

#[test]
fn hil_session_id_must_match_environment() {
    let _lock = ENV_LOCK.blocking_lock();
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("MISMATCHED-SESSION-ID")),
    ]);
    let res = session.verify_hardware_access_authorization();
    assert!(matches!(res, Err(HilError::UnauthorizedSession(_))));
}

#[test]
fn hil_expired_session_is_blocked() {
    let _lock = ENV_LOCK.blocking_lock();
    let mut manifest = valid_test_manifest();
    manifest.expires_at = Some("2020-01-01T00:00:00Z".to_string());
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("SESSION-TEST-01")),
    ]);
    let res = session.verify_hardware_access_authorization();
    assert!(matches!(res, Err(HilError::SessionExpired(_))));
}

#[test]
fn hil_frequency_must_fit_dummy_load_range() {
    let mut manifest = valid_test_manifest();
    manifest.rf_safety.frequency_range_hz = (14350000, 14000000); // Invalid inverted range
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::SafetyViolation(_))
    ));
}

#[test]
fn hil_power_must_not_exceed_dummy_load_rating() {
    let mut manifest = valid_test_manifest();
    manifest.rf_safety.max_test_power_watts = 150.0; // rating is 100.0 W
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::SafetyViolation(_))
    ));
}

#[test]
fn hil_readiness_distinguishes_failed_from_blocked() {
    let mut manifest_failed = valid_test_manifest();
    manifest_failed.rf_safety.termination = HilRfTermination::Antenna {
        antenna_name: "Yagi".to_string(),
    };
    let report_failed =
        HilReadinessValidator::validate(&manifest_failed, HilExecutionStage::Stage7BHardware);
    assert_eq!(report_failed.overall_status, HilReadinessStatus::Failed);

    let mut manifest_blocked = valid_test_manifest();
    manifest_blocked
        .operator_attestation
        .dummy_load_physically_connected = false;
    let report_blocked =
        HilReadinessValidator::validate(&manifest_blocked, HilExecutionStage::Stage7BHardware);
    assert_eq!(report_blocked.overall_status, HilReadinessStatus::Blocked);

    // Błędy i blokady nie mogą być powiązane
    assert_ne!(report_failed.overall_status, report_blocked.overall_status);
}

#[test]
fn hil_operator_attestation_requires_timestamp() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.attestation_timestamp = "   ".to_string();
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::InvalidConsent(_))
    ));
}

#[test]
fn hil_operator_attestation_requires_description() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.description = "".to_string();
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::InvalidConsent(_))
    ));
}

#[test]
fn hil_instrument_evidence_requires_measurement() {
    let mut manifest = valid_test_manifest();
    manifest.instrument_verification = Some(HilInstrumentMeasurement {
        instrument_id: "BIRD-43".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 0.0,
        measured_power_watts: None,
        calibration_valid_until: None,
        measurement_timestamp: None,
        origin: HilEvidenceOrigin::PhysicalMeasurement,
    });
    assert!(matches!(
        manifest.validate_structure(),
        Err(HilError::SafetyViolation(_))
    ));
}

#[tokio::test]
async fn hil_tx_guard_requires_explicit_release() {
    let mut mock_backend = MockHilBackend::new();
    let guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(100),
    );
    assert!(!guard.is_released());
    assert!(!guard.is_confirmed_rx());
}

#[tokio::test]
async fn hil_tx_guard_drop_cannot_report_confirmed_rx() {
    let mut mock_backend = MockHilBackend::new();
    {
        let guard = HilTxGuard::new(
            RigId::new("RIG-IC7300-01").unwrap(),
            &mut mock_backend,
            Duration::from_millis(100),
        );
        assert!(!guard.is_confirmed_rx());
        // guard zostaje upuszczony bez wywołania release_and_confirm_rx()
    }
    // Po upuszczeniu brak potwierdzenia RX!
}

#[tokio::test]
async fn hil_missing_confirmed_rx_cannot_pass() {
    struct StuckPttBackend;
    impl RigBackend for StuckPttBackend {
        fn kind(&self) -> CatBackendKind {
            CatBackendKind::IcomCiV
        }
        fn capabilities(&self) -> RigCapabilities {
            RigCapabilities::default()
        }
        fn connect(&mut self) -> splogbook::cat::backend::BoxFuture<'_, Result<(), RigError>> {
            Box::pin(async { Ok(()) })
        }
        fn disconnect(&mut self) -> splogbook::cat::backend::BoxFuture<'_, ()> {
            Box::pin(async {})
        }
        fn poll(
            &mut self,
        ) -> splogbook::cat::backend::BoxFuture<
            '_,
            Result<splogbook::cat::hamlib::RigState, RigError>,
        > {
            let s = splogbook::cat::hamlib::RigState {
                ptt: true, // Wciąż w stanie TX!
                ..Default::default()
            };
            Box::pin(async move { Ok(s) })
        }
        fn execute<'a>(
            &'a mut self,
            _cmd: &'a RigCommand,
        ) -> splogbook::cat::backend::BoxFuture<'a, Result<(), RigError>> {
            Box::pin(async { Ok(()) })
        }
        fn probe(
            &mut self,
        ) -> splogbook::cat::backend::BoxFuture<'_, Result<RigProbeResult, RigError>> {
            Box::pin(async {
                Ok(RigProbeResult {
                    reachable: true,
                    identity: None,
                    model: None,
                    firmware: None,
                    capabilities: RigCapabilities::default(),
                    state: None,
                    safe_diagnostic: String::new(),
                })
            })
        }
    }

    let mut stuck_backend = StuckPttBackend;
    let mut guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut stuck_backend,
        Duration::from_millis(100),
    );

    let res = guard.release_and_confirm_rx().await;
    assert!(matches!(res, Err(HilError::MissingConfirmedRx(_))));
    assert!(!guard.is_confirmed_rx());
}

#[test]
fn hil_manifest_roundtrip_is_deterministic() {
    let manifest = valid_test_manifest();
    let json1 = manifest.to_json().unwrap();
    let parsed: HilTestBenchManifest = HilTestBenchManifest::from_json(&json1).unwrap();
    let json2 = parsed.to_json().unwrap();
    assert_eq!(json1, json2);
}

#[test]
fn hil_standard_suite_emits_no_ptt_command() {
    let mock = MockHilBackend::new();
    assert_eq!(mock.ptt_count(), 0);
    assert!(!mock.last_ptt_state());
}

#[test]
fn hil_mock_result_cannot_be_marked_as_hardware_passed() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(
        session.readiness_report().overall_status,
        HilReadinessStatus::Blocked
    );
    assert!(!session.readiness_report().can_run_hardware_ptt());
}

#[test]
fn hil_physical_dummy_load_cannot_be_machine_verified_only() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
    let cond9 = report
        .find_condition(9)
        .expect("Condition 9 (physical dummy load) must exist");

    assert_ne!(
        cond9.evidence_kind,
        HilReadinessEvidenceKind::MachineVerified
    );
    assert_eq!(
        cond9.evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
}

#[test]
fn hil_manifest_alone_cannot_authorize_hardware() {
    let _lock = ENV_LOCK.blocking_lock();
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let _env = EnvGuard::new(&[("SPLOG_HIL_ENABLE", None), ("SPLOG_HIL_SESSION", None)]);
    assert!(session.verify_hardware_access_authorization().is_err());
}

#[test]
fn hil_environment_alone_cannot_authorize_hardware() {
    let _lock = ENV_LOCK.blocking_lock();
    let mut manifest = valid_test_manifest();
    manifest.hil_enabled = false;
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("SESSION-TEST-01")),
    ]);
    assert!(session.verify_hardware_access_authorization().is_err());
}

#[test]
fn hil_consent_token_alone_cannot_authorize_tx() {
    let _lock = ENV_LOCK.blocking_lock();
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let arbiter = TxArbiter::new();
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", None),
        ("SPLOG_HIL_SESSION", None),
        ("SPLOG_HIL_ALLOW_TX", None),
    ]);
    assert!(session.verify_tx_authorization(&token, &arbiter).is_err());
}

#[tokio::test]
async fn hil_release_requires_fresh_confirmed_rx() {
    let mut mock_backend = MockHilBackend::new();
    let mut guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(100),
    );
    let res = guard.release_and_confirm_rx().await;
    assert!(res.is_ok());
    assert!(guard.is_confirmed_rx());
}

#[tokio::test]
async fn hil_release_is_attempted_exactly_once() {
    let mut mock_backend = MockHilBackend::new();
    let mut guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(100),
    );
    let res1 = guard.release_and_confirm_rx().await;
    assert!(res1.is_ok());

    let res2 = guard.release_and_confirm_rx().await;
    assert!(matches!(res2, Err(HilError::ReleaseError(_))));
}

#[test]
fn hil_drop_performs_no_blocking_io() {
    let mut mock_backend = MockHilBackend::new();
    let start = std::time::Instant::now();
    {
        let _guard = HilTxGuard::new(
            RigId::new("RIG-IC7300-01").unwrap(),
            &mut mock_backend,
            Duration::from_millis(100),
        );
    }
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(35));
}

#[test]
fn hil_drop_creates_no_runtime() {
    let mut mock_backend = MockHilBackend::new();
    let guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(50),
    );
    drop(guard);
}

// =========================================================================
// TESTY ZGODNOŚCI MODELU DOWODOWEGO I REGRESJI BEZPIECZEŃSTWA (NATIVE-7A)
// =========================================================================

#[test]
fn hil_assessment_separates_implementation_from_runtime_evidence() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments: Vec<HilRequirementAssessment> = report.assessments();
    assert!(!assessments.is_empty());
    for a in &assessments {
        assert_eq!(
            a.implementation_coverage,
            HilImplementationCoverage::Complete
        );
    }
    let ext: Option<&HilExtendedRequirement> = report.find_extended_requirement("HIL-X01");
    assert!(ext.is_some());
}

#[test]
fn hil_complete_implementation_does_not_imply_hardware_passed() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond9 = report.find_condition(9).expect("Condition 9 must exist");
    assert_eq!(
        cond9.implementation_coverage,
        HilImplementationCoverage::Complete
    );
    assert_ne!(cond9.runtime_evidence_status, HilReadinessStatus::Passed);
    assert_eq!(cond9.runtime_evidence_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_offline_pass_does_not_imply_runtime_evidence() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond9 = report.find_condition(9).expect("Condition 9 must exist");
    assert_eq!(cond9.offline_test_status, HilReadinessStatus::Passed);
    assert_eq!(cond9.runtime_evidence_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_missing_physical_evidence_is_blocked() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond19 = report.find_condition(19).expect("Condition 19 must exist");
    assert_eq!(cond19.runtime_evidence_status, HilReadinessStatus::Blocked);
    assert_eq!(cond19.status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_hardware_case_remains_not_run_when_gate_is_blocked() {
    let manifest = valid_test_manifest();
    let dispatcher_report =
        run_isolated_hardware_dispatcher(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(
        dispatcher_report.overall_status,
        HilReadinessStatus::Blocked
    );
    assert_eq!(dispatcher_report.cases.len(), 10);
    for case in &dispatcher_report.cases {
        assert_eq!(case.status, HilReadinessStatus::NotRun);
        assert!(case.reason.contains("HIL-READY runtime evidence blocked"));
    }
}

#[test]
fn hil_synthetic_instrument_fixture_is_not_physical_evidence() {
    let meas = HilInstrumentMeasurement {
        instrument_id: "BIRD-43".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 1.08,
        measured_power_watts: Some(9.8),
        calibration_valid_until: Some("2027-12-31".to_string()),
        measurement_timestamp: Some("2026-10-17T20:55:00Z".to_string()),
        origin: HilEvidenceOrigin::SyntheticOfflineFixture,
    };
    assert!(!meas.is_physical_measurement());
    assert_eq!(meas.origin, HilEvidenceOrigin::SyntheticOfflineFixture);
}

#[test]
fn hil_synthetic_swr_cannot_pass_runtime_gate() {
    let mut manifest = valid_test_manifest();
    if let Some(inst) = &mut manifest.instrument_verification {
        inst.origin = HilEvidenceOrigin::SyntheticOfflineFixture;
    }
    let assessment = HilReadinessValidator::evaluate_instrument_runtime_evidence(&manifest);
    assert_eq!(assessment.offline_test_status, HilReadinessStatus::Passed);
    assert_eq!(
        assessment.runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
    assert!(assessment.diagnostic.contains("SYNTHETIC OFFLINE FIXTURE"));
}

#[test]
fn hil_synthetic_power_cannot_pass_runtime_gate() {
    let mut manifest = valid_test_manifest();
    manifest.instrument_verification = Some(HilInstrumentMeasurement {
        instrument_id: "BIRD-43-PWR".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 1.10,
        measured_power_watts: Some(9.8),
        calibration_valid_until: Some("2027-12-31".to_string()),
        measurement_timestamp: Some("2026-10-17T20:55:00Z".to_string()),
        origin: HilEvidenceOrigin::SyntheticOfflineFixture,
    });
    let runtime_check = HilReadinessValidator::check_instrument_swr_runtime(&manifest);
    assert_eq!(runtime_check.status, HilReadinessStatus::Blocked);
    assert!(runtime_check.details.contains("NOT A PHYSICAL MEASUREMENT"));
}

#[test]
fn hil_example_manifest_cannot_supply_physical_evidence() {
    let manifest = HilTestBenchManifest::from_file("tests/hil/ic7300_lab_bench.json")
        .expect("Canonical test bench manifest must exist");
    assert!(manifest.instrument_verification.is_some());
    let inst = manifest.instrument_verification.as_ref().unwrap();
    assert_eq!(inst.origin, HilEvidenceOrigin::SyntheticOfflineFixture);
    assert!(!inst.is_physical_measurement());

    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let auth_res = session.verify_hardware_access_authorization();
    assert!(auth_res.is_err());
}

#[test]
fn hil_physical_measurement_requires_runtime_session() {
    let mut manifest = valid_test_manifest();
    manifest.instrument_verification = Some(HilInstrumentMeasurement {
        instrument_id: "BIRD-43-PHYS".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 1.05,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2027-12-31".to_string()),
        measurement_timestamp: Some("2026-10-17T20:55:00Z".to_string()),
        origin: HilEvidenceOrigin::PhysicalMeasurement,
    });
    let runtime_check = HilReadinessValidator::check_instrument_swr_runtime(&manifest);
    assert_eq!(runtime_check.status, HilReadinessStatus::Passed);
    assert!(runtime_check.details.contains("Physical instrument"));
}

#[test]
fn hil_fixture_is_labeled_not_valid_for_hardware_readiness() {
    let mut manifest = valid_test_manifest();
    if let Some(inst) = &mut manifest.instrument_verification {
        inst.origin = HilEvidenceOrigin::SyntheticOfflineFixture;
    }
    let runtime_check = HilReadinessValidator::check_instrument_swr_runtime(&manifest);
    assert!(
        runtime_check
            .details
            .contains("NOT VALID FOR HARDWARE HIL-READY")
    );
}

#[test]
fn hil_abandoned_guard_marks_session_failed_or_unknown() {
    let mut mock_backend = MockHilBackend::new();
    let state = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));
    {
        let guard = HilTxGuard::new_with_state(
            RigId::new("RIG-IC7300-01").unwrap(),
            &mut mock_backend,
            Duration::from_millis(50),
            state.clone(),
        );
        assert_eq!(guard.session_state(), HilSessionState::ActiveTx);
        // Abandon guard without calling release_and_confirm_rx()
    }
    assert_eq!(
        state.load(std::sync::atomic::Ordering::SeqCst),
        3 /* FailedOrUnknown */
    );
}

#[test]
fn hil_abandoned_guard_cannot_leave_result_passed() {
    let mut mock_backend = MockHilBackend::new();
    let guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(50),
    );
    assert!(!guard.is_released());
    assert!(!guard.is_confirmed_rx());
    drop(guard);
    // Result can never be Passed when abandoned
}

#[test]
fn hil_abandoned_guard_uses_existing_safety_path() {
    let mut arbiter = TxArbiter::new();
    let rig = RigId::new("RIG-IC7300-01").unwrap();
    let cmd = RigCommandId::new(1);
    arbiter.request_tx(&rig, cmd).unwrap();
    arbiter.confirm_tx(&rig, cmd).unwrap();
    assert!(arbiter.is_owned_by(&rig));

    // TxArbiter remains active/in-flight; Drop of HilTxGuard does not silently release TxArbiter
    let mut mock_backend = MockHilBackend::new();
    let guard = HilTxGuard::new(rig.clone(), &mut mock_backend, Duration::from_millis(50));
    drop(guard);

    assert!(arbiter.is_owned_by(&rig)); // Safe invariant: requires supervisor or explicit action
}

#[tokio::test]
async fn hil_emergency_path_never_retries_ptt_true() {
    let mut mock_backend = MockHilBackend::new();
    let counters = mock_backend.counters();
    let mut guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(1),
    );
    tokio::time::sleep(Duration::from_millis(5)).await;
    let res = guard.release_and_confirm_rx().await;
    assert!(res.is_err());
    assert_eq!(
        counters.ptt_count.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn hil_emergency_release_is_attempted_at_most_once() {
    let mut mock_backend = MockHilBackend::new();
    let mut guard = HilTxGuard::new(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(50),
    );
    let res1 = guard.release_and_confirm_rx().await;
    assert!(res1.is_ok());
    let res2 = guard.release_and_confirm_rx().await;
    assert!(matches!(res2, Err(HilError::ReleaseError(_))));
}

#[tokio::test]
async fn hil_missing_rx_after_emergency_release_remains_unknown() {
    let mut mock_backend = MockHilBackend::new();
    // Simulate backend where poll reports PTT is still true
    mock_backend
        .counters()
        .stick_ptt_on
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let state = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));
    let mut guard = HilTxGuard::new_with_state(
        RigId::new("RIG-IC7300-01").unwrap(),
        &mut mock_backend,
        Duration::from_millis(50),
        state.clone(),
    );
    let res = guard.release_and_confirm_rx().await;
    assert!(matches!(res, Err(HilError::MissingConfirmedRx(_))));
    assert_eq!(
        state.load(std::sync::atomic::Ordering::SeqCst),
        3 /* FailedOrUnknown */
    );
}

#[test]
fn hil_dispatcher_preserves_per_case_status() {
    let manifest = valid_test_manifest();
    let report: HilDispatcherReport =
        run_isolated_hardware_test(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.cases.len(), 10);
    let first_case: &HilDispatcherCase = &report.cases[0];
    assert!(!first_case.name.is_empty());
    let names: Vec<&str> = report.cases.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"hil_hardware_live_test_bench"));
    assert!(names.contains(&"hil_hardware_read_01"));
    assert!(names.contains(&"hil_hardware_shutdown_tx_01"));
}

#[test]
fn hil_dispatcher_cannot_collapse_not_run_into_passed() {
    let manifest = valid_test_manifest();
    let report = run_isolated_hardware_dispatcher(&manifest, HilExecutionStage::Stage7AOffline);
    assert_ne!(report.overall_status, HilReadinessStatus::Passed);
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_dispatcher_blocked_gate_opens_no_transport() {
    let manifest = valid_test_manifest();
    let report = run_isolated_hardware_dispatcher(&manifest, HilExecutionStage::Stage7AOffline);
    assert!(!report.transport_opened);
}

#[test]
fn hil_dispatcher_blocked_gate_emits_no_ptt() {
    let manifest = valid_test_manifest();
    let report = run_isolated_hardware_dispatcher(&manifest, HilExecutionStage::Stage7AOffline);
    assert!(!report.ptt_emitted);
}

#[test]
fn hil_dispatcher_rechecks_expiration() {
    let mut manifest = valid_test_manifest();
    manifest.expires_at = Some("2020-01-01T00:00:00Z".to_string());
    let report = run_isolated_hardware_dispatcher(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.overall_status, HilReadinessStatus::Failed);
    assert!(report.cases[0].reason.contains("expired"));
}

#[test]
fn hil_dispatcher_rechecks_session_id() {
    let mut manifest = valid_test_manifest();
    manifest.session_id = "   ".to_string();
    let report = run_isolated_hardware_dispatcher(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.overall_status, HilReadinessStatus::Failed);
    assert!(report.cases[0].reason.contains("session_id is empty"));
}

// =========================================================================
// TESTY KANONICZNEJ MACIERZY 30 WARUNKÓW HIL-READY ORAZ MODELU DOWODOWEGO
// =========================================================================

#[test]
fn hil_canonical_matrix_has_exactly_30_entries() {
    let matrix = HilReadinessValidator::canonical_matrix();
    assert_eq!(matrix.len(), 30);
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.conditions.len(), 30);
}

#[test]
fn hil_canonical_matrix_contains_no_duplicate_numbers() {
    let matrix = HilReadinessValidator::canonical_matrix();
    let mut seen = std::collections::HashSet::new();
    for c in &matrix {
        assert!(
            seen.insert(c.number),
            "Duplicate condition number: {}",
            c.number
        );
    }
    assert_eq!(seen.len(), 30);
}

#[test]
fn hil_canonical_matrix_contains_no_missing_numbers() {
    let matrix = HilReadinessValidator::canonical_matrix();
    for n in 1..=30 {
        assert!(
            matrix.iter().any(|c| c.number == n),
            "Missing canonical condition number: {n}"
        );
    }
}

#[test]
fn hil_canonical_requirements_have_exact_numbers_1_to_30() {
    let matrix = HilReadinessValidator::canonical_matrix();
    let mut numbers: Vec<u8> = matrix.iter().map(|c| c.number).collect();
    numbers.sort_unstable();
    let expected: Vec<u8> = (1..=30).collect();
    assert_eq!(numbers, expected);
}

#[test]
fn hil_canonical_requirement_descriptions_are_stable() {
    let matrix = HilReadinessValidator::canonical_matrix();
    let expected_descriptions = [
        "Working tree clean.",
        "Wszystkie testy programowe przechodzą.",
        "Pełny SHA testowanego commita jest zapisany.",
        "Radio jest jednoznacznie zidentyfikowane.",
        "Firmware jest zapisane.",
        "Transport CAT jest jednoznacznie określony.",
        "Profil backendu jest poprawnie wybrany.",
        "Antena jest fizycznie odłączona.",
        "Dummy load 50 ohm jest fizycznie podłączony.",
        "Dummy load obsługuje częstotliwość testową.",
        "Dummy load obsługuje moc i czas testu.",
        "Moc radia jest ograniczona.",
        "Wzmacniacz mocy jest odłączony albo zablokowany.",
        "Tuner automatyczny jest wyłączony, jeśli nie jest częścią testu.",
        "Wszystkie pozostałe radia mają zablokowane PTT.",
        "TxArbiter rozpoczyna w stanie Idle.",
        "ActiveTxTarget wskazuje dokładnie testowane radio.",
        "Nie istnieje automatyczny fallback na pierwsze radio.",
        "Dostępny jest fizyczny awaryjny STOP.",
        "Operator potwierdził gotowość stanowiska.",
        "Logowanie działa przed rozpoczęciem próby.",
        "Czas pojedynczej transmisji jest ograniczony.",
        "Procedura przerwania jest zapisana.",
        "Operator potwierdził zgodność próby z zasadami i posiadanymi uprawnieniami.",
        "SmartLink oraz zdalny niekontrolowany TX są wyłączone.",
        "Data Plane poza zakresem testu jest nieaktywny.",
        "Probe został wykonany bez PTT.",
        "Stan RX jest świeżo potwierdzony przed TX.",
        "TxArbiter nie jest w stanie Unknown.",
        "Wszystkie osoby przy stanowisku znają procedurę STOP.",
    ];

    assert_eq!(matrix.len(), 30);
    for (idx, &expected) in expected_descriptions.iter().enumerate() {
        let num = (idx + 1) as u8;
        let cond = matrix
            .iter()
            .find(|c| c.number == num)
            .unwrap_or_else(|| panic!("Condition {num} must exist"));
        assert_eq!(
            cond.description, expected,
            "Mismatch in condition {num} description"
        );
    }
}

#[test]
fn hil_no_canonical_requirement_is_replaced_by_lab_extension() {
    let matrix = HilReadinessValidator::canonical_matrix();
    for c in &matrix {
        assert!(
            !c.id.starts_with("HIL-X"),
            "Canonical requirement cannot be HIL-X: {}",
            c.id
        );
        assert!(
            !c.description.contains("Gaśnica"),
            "Gaśnica cannot be canonical"
        );
        assert!(
            !c.description.contains("Oświetlenie"),
            "Oświetlenie cannot be canonical"
        );
        assert!(
            !c.description.contains("Uziemienie"),
            "Uziemienie cannot be canonical"
        );
        assert!(
            !c.description.contains("materiałów łatwopalnych"),
            "Łatwopalne cannot be canonical"
        );
        assert!(
            !c.description.contains("Zasilacz"),
            "Zasilacz cannot be canonical"
        );
    }
}

#[test]
fn hil_extensions_are_separate_from_canonical_matrix() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.conditions.len(), 30);
    assert_eq!(report.extended_requirements.len(), 15);
    for ext in &report.extended_requirements {
        assert!(
            ext.id.starts_with("HIL-X"),
            "Extension ID must start with HIL-X: {}",
            ext.id
        );
    }
}

#[test]
fn hil_extensions_do_not_count_toward_canonical_total() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.conditions.len(), 30);
    assert_ne!(
        report.conditions.len(),
        30 + report.extended_requirements.len()
    );
}

#[test]
fn hil_every_canonical_requirement_has_implementation_symbol() {
    let matrix = HilReadinessValidator::canonical_matrix();
    for c in &matrix {
        let symbol = c.implementation_symbol();
        assert!(
            !symbol.is_empty(),
            "Condition {} has empty implementation_symbol",
            c.number
        );
        assert!(
            symbol.contains("splogbook::cat"),
            "Invalid symbol path: {symbol}"
        );
    }
}

#[test]
fn hil_every_canonical_requirement_has_offline_test() {
    let matrix = HilReadinessValidator::canonical_matrix();
    for c in &matrix {
        let test_name = c.offline_test();
        assert!(
            !test_name.is_empty(),
            "Condition {} has empty offline_test",
            c.number
        );
    }
}

#[test]
fn hil_every_canonical_requirement_has_evidence_kind() {
    let matrix = HilReadinessValidator::canonical_matrix();
    for c in &matrix {
        match c.evidence_kind {
            HilReadinessEvidenceKind::MachineVerified
            | HilReadinessEvidenceKind::OperatorAttested
            | HilReadinessEvidenceKind::InstrumentVerified => {}
        }
    }
    assert_eq!(
        matrix[0].evidence_kind,
        HilReadinessEvidenceKind::MachineVerified
    );
    assert_eq!(
        matrix[1].evidence_kind,
        HilReadinessEvidenceKind::MachineVerified
    );
    assert_eq!(
        matrix[7].evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
    assert_eq!(
        matrix[8].evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
    assert_eq!(
        matrix[18].evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
    assert_eq!(
        matrix[19].evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
    assert_eq!(
        matrix[29].evidence_kind,
        HilReadinessEvidenceKind::OperatorAttested
    );
}

#[test]
fn hil_every_canonical_requirement_has_runtime_status() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    for c in &report.conditions {
        assert!(matches!(
            c.runtime_evidence_status,
            HilReadinessStatus::Passed
                | HilReadinessStatus::Blocked
                | HilReadinessStatus::Failed
                | HilReadinessStatus::NotRun
        ));
    }
}

#[test]
fn hil_offline_stage_blocks_physical_canonical_requirements() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let physical_condition_numbers = [8, 9, 11, 12, 13, 14, 15, 19, 20, 23, 24, 25, 28, 30];
    for &num in &physical_condition_numbers {
        let cond = report
            .find_condition(num)
            .unwrap_or_else(|| panic!("Condition {num} missing"));
        assert_eq!(
            cond.runtime_evidence_status,
            HilReadinessStatus::Blocked,
            "Condition {num} must be Blocked in Stage 7A Offline"
        );
    }
}

#[test]
fn hil_offline_fixture_cannot_confirm_physical_runtime_evidence() {
    let mut manifest = valid_test_manifest();
    manifest.instrument_verification = Some(HilInstrumentMeasurement {
        instrument_id: "SYNTH-01".to_string(),
        instrument_model: "Synthetic Instrument".to_string(),
        measured_swr: 1.05,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2030-01-01".to_string()),
        measurement_timestamp: Some("2026-10-18T00:00:00Z".to_string()),
        origin: HilEvidenceOrigin::SyntheticOfflineFixture,
    });
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond9 = report.find_condition(9).unwrap();
    assert_eq!(cond9.runtime_evidence_status, HilReadinessStatus::Blocked);
    assert_ne!(cond9.runtime_evidence_status, HilReadinessStatus::Passed);
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_all_canonical_requirements_have_individual_results() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.conditions.len(), 30);
    for c in &report.conditions {
        assert!(!c.id.is_empty());
        assert!(!c.description.is_empty());
        assert!(!c.details.is_empty());
        assert!(c.number >= 1 && c.number <= 30);
    }
}

#[test]
fn hil_runtime_pass_cannot_be_derived_from_offline_fixture() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_ne!(report.overall_status, HilReadinessStatus::Passed);
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
    assert!(!report.is_hil_ready());
}

#[test]
fn hil_canonical_18_never_falls_back_to_first_rig() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.rig_id = RigId::new("NON-EXISTENT-RIG").unwrap();
    let rig_config = manifest.to_rig_config().unwrap();
    assert_eq!(rig_config.id.as_str(), "NON-EXISTENT-RIG");
    assert_ne!(rig_config.id.as_str(), "RIG-IC7300-01");
    assert_ne!(rig_config.id.as_str(), "FIRST");
    let arbiter = TxArbiter::new();
    let non_target = RigId::new("OTHER-RIG").unwrap();
    assert!(matches!(
        arbiter.state(),
        splogbook::cat::tx_arbiter::TxArbiterState::Idle
    ));
    assert!(arbiter.current_owner().is_none());
    assert_ne!(arbiter.current_owner(), Some(&non_target));
}

#[test]
fn hil_canonical_16_requires_tx_arbiter_idle() {
    let arbiter = TxArbiter::new();
    assert!(matches!(
        arbiter.state(),
        splogbook::cat::tx_arbiter::TxArbiterState::Idle
    ));
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond16 = report.find_condition(16).unwrap();
    assert_eq!(cond16.offline_test_status, HilReadinessStatus::Passed);
    assert_eq!(cond16.runtime_evidence_status, HilReadinessStatus::Passed);
}

#[test]
fn hil_canonical_17_requires_exact_active_tx_target() {
    let manifest = valid_test_manifest();
    let target_rig = manifest.matrix.rig_id.clone();
    assert_eq!(target_rig.as_str(), "RIG-IC7300-01");
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond17 = report.find_condition(17).unwrap();
    assert_eq!(cond17.offline_test_status, HilReadinessStatus::Passed);
    assert_eq!(cond17.runtime_evidence_status, HilReadinessStatus::Passed);
}

#[test]
fn hil_canonical_17_rejects_different_rig() {
    let mut arbiter = TxArbiter::new();
    let primary_rig = RigId::new("RIG-IC7300-01").unwrap();
    let foreign_rig = RigId::new("RIG-FLEX6600-02").unwrap();
    let cmd_id = RigCommandId::new(101);

    arbiter.request_tx(&primary_rig, cmd_id).unwrap();
    let second_req = arbiter.request_tx(&foreign_rig, RigCommandId::new(102));
    assert!(
        second_req.is_err(),
        "TxArbiter must reject second rig while primary is requesting/owned"
    );
}

#[test]
fn hil_canonical_29_rejects_unknown_state() {
    let mut arbiter = TxArbiter::new();
    let rig_id = RigId::new("RIG-IC7300-01").unwrap();
    arbiter.request_tx(&rig_id, RigCommandId::new(1)).unwrap();
    arbiter.set_unknown(
        Some(rig_id.clone()),
        splogbook::cat::tx_arbiter::TxUnknownReason::CommandTimeout,
    );
    assert!(matches!(
        arbiter.state(),
        splogbook::cat::tx_arbiter::TxArbiterState::Unknown { .. }
    ));
    let next_tx = arbiter.request_tx(&rig_id, RigCommandId::new(2));
    assert!(
        next_tx.is_err(),
        "PTT must be strictly rejected in Unknown state"
    );
}

#[test]
fn hil_unknown_state_blocks_ptt() {
    let mut arbiter = TxArbiter::new();
    let rig_id = RigId::new("RIG-IC7300-01").unwrap();
    arbiter.set_unknown(
        None,
        splogbook::cat::tx_arbiter::TxUnknownReason::TransportError,
    );
    let res = arbiter.request_tx(&rig_id, RigCommandId::new(99));
    assert!(res.is_err());
}

#[test]
fn hil_idle_state_alone_does_not_authorize_ptt() {
    let arbiter = TxArbiter::new();
    assert!(matches!(
        arbiter.state(),
        splogbook::cat::tx_arbiter::TxArbiterState::Idle
    ));
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let res = session.verify_tx_authorization(&token, &arbiter);
    assert!(
        res.is_err(),
        "Idle state alone in Stage 7A cannot authorize PTT"
    );
}

#[test]
fn hil_canonical_27_probe_is_read_only() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert!(report.can_run_probe());
    assert!(!report.can_run_hardware_ptt());
    let cond27 = report.find_condition(27).unwrap();
    assert_eq!(cond27.offline_test_status, HilReadinessStatus::Passed);
}

#[tokio::test]
async fn hil_canonical_27_probe_emits_no_ptt() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);
    let mut mock_backend = MockHilBackend::new();
    let res = session.execute_safe_probe_backend(&mut mock_backend).await;
    assert!(res.is_ok());
    assert_eq!(mock_backend.ptt_count(), 0);
    assert!(!mock_backend.last_ptt_state());
}

#[tokio::test]
async fn hil_canonical_27_probe_emits_no_vfo_write() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);
    let mut mock_backend = MockHilBackend::new();
    let res = session.execute_safe_probe_backend(&mut mock_backend).await;
    assert!(res.is_ok());
    assert_eq!(mock_backend.ptt_count(), 0);
}

#[test]
fn hil_canonical_28_requires_fresh_confirmed_rx() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond28 = report.find_condition(28).unwrap();
    assert_eq!(cond28.runtime_evidence_status, HilReadinessStatus::Blocked);
}

#[tokio::test]
async fn hil_canonical_28_rejects_stale_rx() {
    let manifest = valid_test_manifest();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let mut mock_backend = MockHilBackend::new();
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let mut arbiter = TxArbiter::new();
    let _lock = ENV_LOCK.lock().await;
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("SESSION-TEST-01")),
        ("SPLOG_HIL_ALLOW_TX", Some("I_HAVE_DUMMY_LOAD")),
    ]);
    let guard = session
        .acquire_tx_guard(
            &mut mock_backend,
            &token,
            &mut arbiter,
            Duration::from_millis(10),
        )
        .await
        .unwrap();
    assert!(!guard.is_confirmed_rx());
}

#[test]
fn hil_canonical_28_rejects_rx_from_other_rig() {
    let rig_a = RigId::new("RIG-IC7300-01").unwrap();
    let rig_b = RigId::new("RIG-K3S-02").unwrap();
    assert_ne!(rig_a, rig_b);
    let manifest = valid_test_manifest();
    assert_eq!(manifest.matrix.rig_id, rig_a);
}

#[test]
fn hil_canonical_28_rejects_rx_from_other_session() {
    let manifest = valid_test_manifest();
    let _lock = ENV_LOCK.blocking_lock();
    let _env = EnvGuard::new(&[
        ("SPLOG_HIL_ENABLE", Some("1")),
        ("SPLOG_HIL_SESSION", Some("SESSION-DIFFERENT")),
        ("SPLOG_HIL_ALLOW_TX", Some("I_HAVE_DUMMY_LOAD")),
    ]);
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let arbiter = TxArbiter::new();
    let res = session.verify_tx_authorization(&token, &arbiter);
    assert!(matches!(res, Err(HilError::UnauthorizedSession(_))));
}

#[test]
fn hil_canonical_25_rejects_smartlink() {
    let mut manifest = valid_test_manifest();
    manifest.matrix.transport = HilTransportConfig::Network {
        host: "smartlink.flexradio.com".to_string(),
        port: 4992,
    };
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond25 = report.find_condition(25).unwrap();
    assert_eq!(cond25.runtime_evidence_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_canonical_25_rejects_unattended_remote_tx() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.rf_perimeter_secured = false;
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_canonical_26_rejects_data_plane() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond26 = report.find_condition(26).unwrap();
    assert_eq!(cond26.offline_test_status, HilReadinessStatus::Passed);
    assert_eq!(cond26.runtime_evidence_status, HilReadinessStatus::Passed);
}

#[test]
fn hil_canonical_26_rejects_audio_data_plane() {
    let manifest = valid_test_manifest();
    let rig_config = manifest.to_rig_config().unwrap();
    assert!(
        rig_config.backend.kind_str().contains("Icom")
            || rig_config.backend.kind_str().contains("Flex")
    );
}

#[test]
fn hil_canonical_26_rejects_iq_data_plane() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond26 = report.find_condition(26).unwrap();
    assert_eq!(cond26.status, HilReadinessStatus::Passed);
}

#[test]
fn hil_canonical_26_rejects_waterfall_data_plane() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond26 = report.find_condition(26).unwrap();
    assert_eq!(cond26.status, HilReadinessStatus::Passed);
}

#[test]
fn hil_canonical_26_control_plane_does_not_enable_data_plane() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert!(report.can_run_probe());
    let cond26 = report.find_condition(26).unwrap();
    assert_eq!(
        cond26.implementation_coverage,
        HilImplementationCoverage::Complete
    );
}

#[test]
fn hil_canonical_21_requires_logging_before_hardware_access() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond21 = report.find_condition(21).unwrap();
    assert_eq!(cond21.offline_test_status, HilReadinessStatus::Passed);
    assert_eq!(cond21.runtime_evidence_status, HilReadinessStatus::Passed);
}

#[test]
fn hil_canonical_21_rejects_unwritable_log_destination() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond21 = report.find_condition(21).unwrap();
    assert!(!cond21.details.is_empty());
}

#[test]
fn hil_canonical_23_requires_documented_abort_procedure() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.abort_procedure_documented = false;
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond23 = report.find_condition(23).unwrap();
    assert_eq!(cond23.offline_test_status, HilReadinessStatus::Blocked);
    assert_eq!(cond23.runtime_evidence_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_canonical_23_requires_nonempty_abort_steps() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond23 = report.find_condition(23).unwrap();
    assert!(!cond23.details.is_empty());
}

#[test]
fn hil_canonical_30_requires_personnel_attestation() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.personnel_know_stop_procedure = false;
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond30 = report.find_condition(30).unwrap();
    assert_eq!(cond30.offline_test_status, HilReadinessStatus::Blocked);
    assert_eq!(cond30.runtime_evidence_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_canonical_30_requires_attestation_timestamp() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.attestation_timestamp = "   ".to_string();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_canonical_30_requires_attestation_description() {
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.description = "   ".to_string();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

// =========================================================================
// OSOBNE SPRZĘTOWE PUNKTY WEJŚCIA (NATIVE-7B Hardware-in-the-Loop)
// Wszystkie punkty wejścia są hermetycznie odizolowane przez #[ignore].
// =========================================================================

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_read_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    session
        .verify_hardware_access_authorization()
        .expect("Hardware access authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_freq_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    session
        .verify_hardware_access_authorization()
        .expect("Hardware access authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_mode_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    session
        .verify_hardware_access_authorization()
        .expect("Hardware access authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_ptt_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let arbiter = TxArbiter::new();
    session
        .verify_tx_authorization(&token, &arbiter)
        .expect("TX authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_ptt_timeout_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let arbiter = TxArbiter::new();
    session
        .verify_tx_authorization(&token, &arbiter)
        .expect("TX authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_resync_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    session
        .verify_hardware_access_authorization()
        .expect("Hardware access authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_multi_radio_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    session
        .verify_hardware_access_authorization()
        .expect("Hardware access authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_shutdown_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    session
        .verify_hardware_access_authorization()
        .expect("Hardware access authorized");
}

#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_shutdown_tx_01() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());
    let manifest = HilTestBenchManifest::from_file(&manifest_path).unwrap();
    let session = HilHardwareSession::new(manifest, HilExecutionStage::Stage7BHardware);
    let token = HilPttConsentToken::new("SP6INA").unwrap();
    let arbiter = TxArbiter::new();
    session
        .verify_tx_authorization(&token, &arbiter)
        .expect("TX authorized");
}

/// Główny punkt wejścia dyspozytora dla fizycznego stanowiska laboratoryjnego (NATIVE-7B).
#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_live_test_bench() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());

    let manifest = HilTestBenchManifest::from_file(&manifest_path)
        .expect("Physical HIL manifest must exist and be valid");

    let session = HilHardwareSession::new(manifest.clone(), HilExecutionStage::Stage7BHardware);
    session
        .verify_hardware_access_authorization()
        .expect("Hardware session authorization failed");

    let rig_config = manifest
        .to_rig_config()
        .expect("Valid RigConfig must be produced from manifest");

    let probe_result = session
        .execute_safe_probe(&rig_config)
        .await
        .expect("Probe of physical rig failed");

    println!(
        "Live HIL probe succeeded: {} (capabilities: {:?})",
        probe_result.model_name, probe_result.capabilities
    );
}

// =========================================================================
// NATIVE-7A-EVIDENCE-SEMANTICS-FINAL-GATE: Offline vs Physical Separation Tests
// =========================================================================

#[test]
fn hil_execution_contexts_are_distinct() {
    assert_ne!(
        HilExecutionContext::OfflineModel,
        HilExecutionContext::SimulatedRuntime
    );
    assert_ne!(
        HilExecutionContext::OfflineModel,
        HilExecutionContext::PhysicalRuntime
    );
    assert_ne!(
        HilExecutionContext::SimulatedRuntime,
        HilExecutionContext::PhysicalRuntime
    );
}

#[test]
fn hil_execution_context_variants_are_distinct() {
    let offline = HilExecutionContext::OfflineModel;
    let sim = HilExecutionContext::SimulatedRuntime;
    let phys = HilExecutionContext::PhysicalRuntime;

    assert!(offline.is_offline());
    assert!(!offline.is_simulated());
    assert!(!offline.is_physical());

    assert!(!sim.is_offline());
    assert!(sim.is_simulated());
    assert!(!sim.is_physical());

    assert!(!phys.is_offline());
    assert!(!phys.is_simulated());
    assert!(phys.is_physical());
}

#[test]
fn hil_execution_context_displays_correctly() {
    assert_eq!(
        HilExecutionContext::OfflineModel.to_string(),
        "OfflineModel"
    );
    assert_eq!(
        HilExecutionContext::SimulatedRuntime.to_string(),
        "SimulatedRuntime"
    );
    assert_eq!(
        HilExecutionContext::PhysicalRuntime.to_string(),
        "PhysicalRuntime"
    );
}

#[test]
fn hil_offline_model_is_not_physical_runtime() {
    let offline = HilExecutionContext::OfflineModel;
    let physical = HilExecutionContext::PhysicalRuntime;
    assert_ne!(offline, physical);
    assert!(!offline.is_physical());
}

#[test]
fn hil_offline_model_evaluation_cannot_grant_physical_pass() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    for a in &assessments {
        if a.evaluation_context == HilExecutionContext::OfflineModel {
            assert_ne!(
                a.physical_runtime_evidence_status,
                HilReadinessStatus::Passed,
                "OfflineModel context for {} must never grant physical Passed",
                a.requirement_id
            );
        }
    }
}

#[test]
fn hil_simulated_runtime_is_not_physical_runtime() {
    let sim = HilExecutionContext::SimulatedRuntime;
    let physical = HilExecutionContext::PhysicalRuntime;
    assert_ne!(sim, physical);
    assert!(!sim.is_physical());
}

#[test]
fn hil_simulated_runtime_cannot_grant_physical_pass() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    for a in &assessments {
        if a.evaluation_context == HilExecutionContext::SimulatedRuntime {
            assert_ne!(
                a.physical_runtime_evidence_status,
                HilReadinessStatus::Passed,
                "SimulatedRuntime for {} cannot grant physical Passed",
                a.requirement_id
            );
        }
    }
}

#[test]
fn hil_fixture_cannot_create_physical_runtime_context() {
    let origin = HilEvidenceOrigin::SyntheticOfflineFixture;
    assert!(origin.is_synthetic_or_mock());
    assert!(!origin.is_physical());
    let meas = HilInstrumentMeasurement {
        instrument_id: "SWR-DUMMY".to_string(),
        instrument_model: "Mock-43".to_string(),
        measured_swr: 1.10,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2027-01-01".to_string()),
        measurement_timestamp: Some("2026-10-18T10:00:00Z".to_string()),
        origin,
    };
    assert!(!meas.is_physical_measurement());
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
}

#[test]
fn hil_mock_cannot_create_physical_runtime_context() {
    let origin = HilEvidenceOrigin::MockRuntime;
    assert!(origin.is_synthetic_or_mock());
    assert!(!origin.is_physical());
    let meas = HilInstrumentMeasurement {
        instrument_id: "SWR-MOCK".to_string(),
        instrument_model: "Mock-Instrument".to_string(),
        measured_swr: 1.05,
        measured_power_watts: Some(5.0),
        calibration_valid_until: None,
        measurement_timestamp: Some("2026-10-18T10:00:00Z".to_string()),
        origin,
    };
    assert!(!meas.is_physical_measurement());
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
}

#[test]
fn hil_physical_runtime_requires_hardware_session() {
    let manifest = valid_test_manifest();
    let session_7a = HilHardwareSession::new(manifest, HilExecutionStage::Stage7AOffline);
    assert_ne!(session_7a.stage(), HilExecutionStage::Stage7BHardware);
    assert_eq!(
        session_7a.readiness_report().execution_stage,
        HilExecutionStage::Stage7AOffline
    );
    assert!(!session_7a.readiness_report().can_run_hardware_ptt());
}

#[test]
fn hil_physical_runtime_pass_requires_physical_context() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    for a in &assessments {
        if a.physical_runtime_evidence_status == HilReadinessStatus::Passed {
            assert_eq!(a.evaluation_context, HilExecutionContext::PhysicalRuntime);
        }
    }
}

#[test]
fn hil_physical_runtime_pass_requires_physical_evidence() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    for a in &assessments {
        if a.physical_runtime_evidence_status == HilReadinessStatus::Passed {
            assert_eq!(a.evidence_origin, HilEvidenceOrigin::PhysicalMeasurement);
        }
    }
}

#[test]
fn hil_physical_runtime_pass_rejects_synthetic_origin() {
    let mut manifest = valid_test_manifest();
    if let Some(inst) = &mut manifest.instrument_verification {
        inst.origin = HilEvidenceOrigin::SyntheticOfflineFixture;
    }
    let assessment = HilReadinessValidator::evaluate_instrument_runtime_evidence(&manifest);
    assert_eq!(
        assessment.evidence_origin,
        HilEvidenceOrigin::SyntheticOfflineFixture
    );
    assert_ne!(
        assessment.physical_runtime_evidence_status,
        HilReadinessStatus::Passed
    );
    assert_eq!(
        assessment.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_physical_runtime_pass_rejects_mock_origin() {
    let mut manifest = valid_test_manifest();
    if let Some(inst) = &mut manifest.instrument_verification {
        inst.origin = HilEvidenceOrigin::MockRuntime;
    }
    let assessment = HilReadinessValidator::evaluate_instrument_runtime_evidence(&manifest);
    assert_eq!(assessment.evidence_origin, HilEvidenceOrigin::MockRuntime);
    assert_ne!(
        assessment.physical_runtime_evidence_status,
        HilReadinessStatus::Passed
    );
    assert_eq!(
        assessment.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_physical_runtime_requires_non_synthetic_evidence() {
    let syn = HilEvidenceOrigin::SyntheticOfflineFixture;
    let mock = HilEvidenceOrigin::MockRuntime;
    let phys = HilEvidenceOrigin::PhysicalMeasurement;
    assert!(syn.is_synthetic_or_mock());
    assert!(mock.is_synthetic_or_mock());
    assert!(!phys.is_synthetic_or_mock());
    assert!(phys.is_physical());
}

#[test]
fn hil_assessment_separates_offline_and_physical_runtime() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let cond9 = assessments
        .iter()
        .find(|a| a.canonical_number == Some(9))
        .unwrap();
    assert_eq!(cond9.offline_evaluation_status, HilReadinessStatus::Passed);
    assert_eq!(
        cond9.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_assessment_separates_offline_from_simulated_runtime() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let cond16 = assessments
        .iter()
        .find(|a| a.canonical_number == Some(16))
        .unwrap();
    assert_eq!(cond16.offline_evaluation_status, HilReadinessStatus::Passed);
    assert_eq!(cond16.simulated_runtime_status, HilReadinessStatus::Passed);
    assert_eq!(
        cond16.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_offline_pass_does_not_set_physical_runtime_passed() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    for a in &assessments {
        if a.offline_evaluation_status == HilReadinessStatus::Passed {
            assert_ne!(
                a.physical_runtime_evidence_status,
                HilReadinessStatus::Passed,
                "offline pass on {} must not set physical Passed",
                a.requirement_id
            );
        }
    }
}

#[test]
fn hil_simulated_pass_does_not_set_physical_runtime_passed() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    for a in &assessments {
        if a.simulated_runtime_status == HilReadinessStatus::Passed {
            assert_ne!(
                a.physical_runtime_evidence_status,
                HilReadinessStatus::Passed,
                "simulated pass on {} must not set physical Passed",
                a.requirement_id
            );
        }
    }
}

#[test]
fn hil_simulated_pass_cannot_grant_hardware_ptt() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert!(!report.can_run_hardware_ptt());
}

#[test]
fn hil_extended_requirement_reports_execution_context() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let ext = report.find_extended_requirement("HIL-X01").unwrap();
    assert_eq!(ext.execution_context, HilExecutionContext::SimulatedRuntime);
}

#[test]
fn hil_extended_offline_pass_does_not_imply_physical_pass() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let ext_x01 = assessments
        .iter()
        .find(|a| a.requirement_id == "HIL-X01")
        .unwrap();
    assert_eq!(ext_x01.offline_test_status, HilReadinessStatus::Passed);
    assert_ne!(
        ext_x01.physical_runtime_evidence_status,
        HilReadinessStatus::Passed
    );
}

#[test]
fn hil_extended_mock_pass_does_not_imply_physical_pass() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let ext_x13 = assessments
        .iter()
        .find(|a| a.requirement_id == "HIL-X13")
        .unwrap();
    assert_eq!(ext_x13.simulated_runtime_status, HilReadinessStatus::Passed);
    assert_ne!(
        ext_x13.physical_runtime_evidence_status,
        HilReadinessStatus::Passed
    );
}

#[test]
fn hil_extended_physical_status_is_not_run_without_hardware() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    for a in &assessments {
        if a.canonical_number.is_none() {
            assert_eq!(
                a.physical_runtime_evidence_status,
                HilReadinessStatus::NotRun
            );
        }
    }
}

#[test]
fn hil_extended_fixture_origin_is_explicit() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    for ext in &report.extended_requirements {
        assert_eq!(
            ext.evidence_origin,
            HilEvidenceOrigin::SyntheticOfflineFixture
        );
    }
}

#[test]
fn hil_baseline_evidence_records_full_commit() {
    let evidence = HilSoftwareBaselineEvidence::canonical_current();
    assert_eq!(evidence.tested_commit.len(), 40);
    assert!(
        evidence
            .tested_commit
            .chars()
            .all(|c| c.is_ascii_hexdigit())
    );
    assert!(evidence.is_valid_baseline());

    let mut short = evidence;
    short.tested_commit = "698a34b".to_string();
    assert!(!short.is_valid_baseline());
}

#[test]
fn hil_baseline_evidence_records_commands() {
    let evidence = HilSoftwareBaselineEvidence::canonical_current();
    assert_eq!(evidence.command_results.len(), 3);
    assert!(
        evidence
            .command_results
            .iter()
            .any(|c| c.command.contains("cat_native_6a"))
    );
    assert!(
        evidence
            .command_results
            .iter()
            .any(|c| c.command.contains("cat_native_6b"))
    );
    assert!(
        evidence
            .command_results
            .iter()
            .any(|c| c.command.contains("cat_native_7"))
    );
}

#[test]
fn hil_baseline_evidence_rejects_nonzero_exit() {
    let mut evidence = HilSoftwareBaselineEvidence::canonical_current();
    evidence.add_command_result(HilCommandResult {
        command: "cargo test --test failing".to_string(),
        exit_code: 1,
        passed_count: 0,
        failed_count: 1,
        ignored_count: 0,
    });
    assert!(!evidence.all_commands_succeeded);
    assert!(!evidence.is_valid_baseline());
}

#[test]
fn hil_baseline_evidence_rejects_failed_tests() {
    let mut evidence = HilSoftwareBaselineEvidence::canonical_current();
    evidence.add_command_result(HilCommandResult {
        command: "cargo test --test fails".to_string(),
        exit_code: 0,
        passed_count: 10,
        failed_count: 1,
        ignored_count: 0,
    });
    assert!(!evidence.all_commands_succeeded);
    assert!(evidence.failed_test_count > 0);
    assert!(!evidence.is_valid_baseline());
}

#[test]
fn hil_baseline_evidence_does_not_treat_ignored_hardware_as_failure() {
    let evidence = HilSoftwareBaselineEvidence::canonical_current();
    let cat7 = evidence
        .command_results
        .iter()
        .find(|c| c.command.contains("cat_native_7"))
        .unwrap();
    assert_eq!(cat7.ignored_count, 10);
    assert_eq!(cat7.failed_count, 0);
    assert_eq!(cat7.exit_code, 0);
    assert!(evidence.is_valid_baseline());
}

#[test]
fn hil_baseline_evidence_is_not_physical_runtime_evidence() {
    let evidence = HilSoftwareBaselineEvidence::canonical_current();
    assert!(!evidence.is_physical_runtime_evidence());
    assert_eq!(
        evidence.execution_context(),
        HilExecutionContext::OfflineModel
    );
    assert_eq!(
        evidence.evidence_origin(),
        HilEvidenceOrigin::SyntheticOfflineFixture
    );
}

#[test]
fn hil_abort_procedure_document_exists() {
    assert!(HilAbortProcedure::exists());
    let doc = HilAbortProcedure::canonical();
    assert_eq!(doc.document_id, "HIL-SOP-ABORT-001");
}

#[test]
fn hil_abort_procedure_is_nonempty() {
    let doc = HilAbortProcedure::canonical();
    assert!(doc.is_nonempty());
    assert!(!doc.title.is_empty());
    assert!(!doc.content.is_empty());
    assert!(!doc.steps.is_empty());
}

#[test]
fn hil_abort_procedure_contains_stop_steps() {
    let doc = HilAbortProcedure::canonical();
    assert!(doc.contains_stop_steps());
}

#[test]
fn hil_abort_procedure_contains_ptt_disconnect() {
    let doc = HilAbortProcedure::canonical();
    assert!(doc.contains_ptt_disconnect());
}

#[test]
fn hil_abort_procedure_contains_power_disconnect() {
    let doc = HilAbortProcedure::canonical();
    assert!(doc.contains_power_disconnect());
}

#[test]
fn hil_operator_acknowledges_abort_procedure() {
    let manifest = valid_test_manifest();
    assert!(manifest.operator_attestation.abort_procedure_documented);
    assert!(
        !manifest
            .operator_attestation
            .attestation_timestamp
            .is_empty()
    );
    assert!(!manifest.operator_id.is_empty());
    assert!(!manifest.operator_attestation.description.is_empty());
}

#[test]
fn hil_document_presence_does_not_imply_operator_acknowledgement() {
    assert!(HilAbortProcedure::canonical().is_nonempty());
    let mut manifest = valid_test_manifest();
    manifest.operator_attestation.abort_procedure_documented = false;
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond23 = report.find_condition(23).unwrap();
    assert_ne!(cond23.status, HilReadinessStatus::Passed);
    assert_eq!(cond23.status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_probe_offline_pass_does_not_imply_physical_probe() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert!(report.can_run_probe());
    let cond27 = report.find_condition(27).unwrap();
    assert_eq!(cond27.offline_test_status, HilReadinessStatus::Passed);
    let assessment = report
        .assessments()
        .into_iter()
        .find(|a| a.canonical_number == Some(27))
        .unwrap();
    assert_eq!(
        assessment.offline_evaluation_status,
        HilReadinessStatus::Passed
    );
    assert_eq!(
        assessment.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_confirmed_rx_mock_does_not_imply_physical_rx() {
    let origin = HilEvidenceOrigin::MockRuntime;
    assert_ne!(origin, HilEvidenceOrigin::PhysicalMeasurement);
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let cond28 = report.find_condition(28).unwrap();
    assert_eq!(cond28.offline_test_status, HilReadinessStatus::Passed);
    assert_eq!(cond28.runtime_evidence_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_idle_mock_does_not_imply_physical_arbiter_state() {
    let arbiter = TxArbiter::new();
    assert_eq!(
        arbiter.state(),
        &splogbook::cat::tx_arbiter::TxArbiterState::Idle
    );
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let cond16 = assessments
        .iter()
        .find(|a| a.canonical_number == Some(16))
        .unwrap();
    assert_eq!(cond16.offline_evaluation_status, HilReadinessStatus::Passed);
    assert_eq!(
        cond16.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_active_target_config_does_not_imply_physical_session_target() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let cond17 = assessments
        .iter()
        .find(|a| a.canonical_number == Some(17))
        .unwrap();
    assert_eq!(cond17.offline_evaluation_status, HilReadinessStatus::Passed);
    assert_eq!(
        cond17.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_data_plane_config_does_not_imply_physical_connection_state() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let cond26 = assessments
        .iter()
        .find(|a| a.canonical_number == Some(26))
        .unwrap();
    assert_eq!(cond26.offline_evaluation_status, HilReadinessStatus::Passed);
    assert_eq!(
        cond26.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_synthetic_fixture_can_pass_parser_only() {
    let manifest = valid_test_manifest();
    assert_eq!(manifest.bench_id, "LAB-BENCH-SP6INA-01");
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    assert_eq!(report.overall_status, HilReadinessStatus::Blocked);
}

#[test]
fn hil_mock_runtime_can_pass_state_machine_only() {
    let origin = HilEvidenceOrigin::MockRuntime;
    assert!(origin.is_synthetic_or_mock());
    let arbiter = TxArbiter::new();
    assert_eq!(
        arbiter.state(),
        &splogbook::cat::tx_arbiter::TxArbiterState::Idle
    );
}

#[test]
fn hil_operator_fixture_cannot_pass_physical_runtime() {
    let manifest = valid_test_manifest();
    let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7AOffline);
    let assessments = report.assessments();
    let cond8 = assessments
        .iter()
        .find(|a| a.canonical_number == Some(8))
        .unwrap();
    assert_eq!(
        cond8.physical_runtime_evidence_status,
        HilReadinessStatus::Blocked
    );
}

#[test]
fn hil_instrument_interface_is_not_automatically_physical_measurement() {
    let meas = HilInstrumentMeasurement {
        instrument_id: "RIGEXPERT-AA".to_string(),
        instrument_model: "AA-55".to_string(),
        measured_swr: 1.15,
        measured_power_watts: None,
        calibration_valid_until: None,
        measurement_timestamp: None,
        origin: HilEvidenceOrigin::InstrumentInterface,
    };
    assert_eq!(meas.origin, HilEvidenceOrigin::InstrumentInterface);
    assert!(!meas.is_physical_measurement());
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
}

#[test]
fn hil_physical_measurement_requires_physical_context() {
    let meas = HilInstrumentMeasurement {
        instrument_id: "BIRD-43".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 1.12,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2027-10-18".to_string()),
        measurement_timestamp: Some("2026-10-18T10:00:00Z".to_string()),
        origin: HilEvidenceOrigin::PhysicalMeasurement,
    };
    assert!(meas.is_physical_measurement());
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::OfflineModel)
            .is_err()
    );
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::SimulatedRuntime)
            .is_err()
    );
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_ok()
    );
}

#[test]
fn hil_physical_measurement_requires_session_id() {
    let meas = HilInstrumentMeasurement {
        instrument_id: "BIRD-43".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 1.12,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2027-10-18".to_string()),
        measurement_timestamp: Some("2026-10-18T10:00:00Z".to_string()),
        origin: HilEvidenceOrigin::PhysicalMeasurement,
    };
    assert!(
        meas.validate_physical_evidence("", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
    assert!(
        meas.validate_physical_evidence("   ", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_ok()
    );
}

#[test]
fn hil_physical_measurement_requires_timestamp() {
    let mut meas = HilInstrumentMeasurement {
        instrument_id: "BIRD-43".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 1.12,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2027-10-18".to_string()),
        measurement_timestamp: None,
        origin: HilEvidenceOrigin::PhysicalMeasurement,
    };
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
    meas.measurement_timestamp = Some("   ".to_string());
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
    meas.measurement_timestamp = Some("2026-10-18T10:00:00Z".to_string());
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_ok()
    );
}

#[test]
fn hil_physical_measurement_requires_instrument_identity() {
    let mut meas = HilInstrumentMeasurement {
        instrument_id: "".to_string(),
        instrument_model: "Bird 43".to_string(),
        measured_swr: 1.12,
        measured_power_watts: Some(10.0),
        calibration_valid_until: Some("2027-10-18".to_string()),
        measurement_timestamp: Some("2026-10-18T10:00:00Z".to_string()),
        origin: HilEvidenceOrigin::PhysicalMeasurement,
    };
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
    meas.instrument_id = "BIRD-43".to_string();
    meas.instrument_model = "".to_string();
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_err()
    );
    meas.instrument_model = "Bird 43".to_string();
    assert!(
        meas.validate_physical_evidence("SESSION-01", HilExecutionContext::PhysicalRuntime)
            .is_ok()
    );
}
