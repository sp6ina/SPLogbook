// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Zestaw deterministycznych testów integracyjnych offline dla harnessu HIL (NATIVE-7A).
//!
//! Zgodnie z zasadami programu NATIVE-7:
//! - Testy offline weryfikują maszynę stanów, parser manifestu, walidator HIL-READY
//!   oraz blokady bezpieczeństwa toru RF.
//! - Żaden test offline nie wykonuje fizycznej emisji RF ani nie załącza fizycznego PTT.
//! - Prawdziwe testy ze sprzętem są hermetycznie odizolowane atrybutem `#[ignore = "hardware-in-the-loop"]`.

use std::time::Duration;

use splogbook::cat::backend::{CatBackendKind, RigBackend, RigProbeResult};
use splogbook::cat::hil::{
    HilDummyLoadSpec, HilError, HilExecutionStage, HilHardwareSession, HilInstrumentMeasurement,
    HilMatrixEntry, HilOperatorAttestation, HilPttConsentToken, HilReadinessEvidenceKind,
    HilReadinessStatus, HilReadinessValidator, HilRfSafetyConfig, HilRfTermination,
    HilTestBenchManifest, HilTransportConfig,
};
use splogbook::cat::rig::{RigCapabilities, RigCommand, RigError};
use splogbook::cat::routing::RigId;

fn valid_test_manifest() -> HilTestBenchManifest {
    HilTestBenchManifest {
        bench_id: "LAB-BENCH-SP6INA-01".to_string(),
        operator_callsign: "SP6INA".to_string(),
        operator_name: "Mariusz Woźniak".to_string(),
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
        },
        operator_attestation: HilOperatorAttestation {
            dummy_load_physically_connected: true,
            rf_perimeter_secured: true,
            ptt_consent_granted: true,
            attestation_timestamp: "2026-10-17T20:56:00Z".to_string(),
        },
        instrument_verification: Some(HilInstrumentMeasurement {
            instrument_id: "BIRD-43".to_string(),
            instrument_model: "Bird 43 Thruline".to_string(),
            measured_swr: 1.08,
            measured_power_watts: Some(9.8),
            calibration_valid_until: Some("2027-12-31".to_string()),
        }),
    }
}

/// Atrapa backendu dla testów jednostkowych pętli wykonawczej sesji HIL.
struct MockHilBackend {
    ptt_count: usize,
    last_ptt_state: bool,
}

impl MockHilBackend {
    fn new() -> Self {
        Self {
            ptt_count: 0,
            last_ptt_state: false,
        }
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
        Box::pin(async { Ok(()) })
    }

    fn disconnect(&mut self) -> splogbook::cat::backend::BoxFuture<'_, ()> {
        Box::pin(async {})
    }

    fn poll(
        &mut self,
    ) -> splogbook::cat::backend::BoxFuture<'_, Result<splogbook::cat::hamlib::RigState, RigError>>
    {
        Box::pin(async { Ok(splogbook::cat::hamlib::RigState::default()) })
    }

    fn execute<'a>(
        &'a mut self,
        command: &'a RigCommand,
    ) -> splogbook::cat::backend::BoxFuture<'a, Result<(), RigError>> {
        Box::pin(async move {
            if let RigCommand::SetPtt(state) = command {
                if *state {
                    self.ptt_count += 1;
                }
                self.last_ptt_state = *state;
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
    assert_eq!(mock_backend.ptt_count, 0);
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
    assert_eq!(mock_backend.ptt_count, 0);
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
    assert_eq!(mock_backend.ptt_count, 0);
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

    assert_eq!(mock_backend.ptt_count, 1);
    assert!(!mock_backend.last_ptt_state); // verified RX returned
    assert!(result.rx_restored_cleanly);
}

/// Bezpieczny punkt wejścia do testów na fizycznym sprzęcie laboratoryjnym (NATIVE-7B).
///
/// Ten test jest celowo wyłączony (`#[ignore = "hardware-in-the-loop"]`) z automatycznych
/// przebiegów CI oraz standardowego `cargo test`. Może zostać uruchomiony wyłącznie
/// przez operatora w laboratorium z podłączonym fizycznym radiem i sztucznym obciążeniem:
///
/// `cargo test --test cat_native_7 -- --ignored hil_hardware_live_test_bench`
#[tokio::test]
#[ignore = "hardware-in-the-loop"]
async fn hil_hardware_live_test_bench() {
    let manifest_path = std::env::var("SPLOG_HIL_MANIFEST")
        .unwrap_or_else(|_| "tests/hil/ic7300_lab_bench.json".to_string());

    let manifest = HilTestBenchManifest::from_file(&manifest_path)
        .expect("Physical HIL manifest must exist and be valid");

    // Weryfikacja HIL-READY dla fazy 7B
    let session = HilHardwareSession::new(manifest.clone(), HilExecutionStage::Stage7BHardware);
    let report = session.readiness_report();

    if !report.is_hil_ready() {
        panic!("Laboratory test bench is NOT HIL-READY: {report:#?}");
    }

    let rig_config = manifest
        .to_rig_config()
        .expect("Valid RigConfig must be produced from manifest");

    // 1. Bezpieczny odczyt parametrów (probe)
    let probe_result = session
        .execute_safe_probe(&rig_config)
        .await
        .expect("Probe of physical rig failed");

    println!(
        "Live HIL probe succeeded: {} (capabilities: {:?})",
        probe_result.model_name, probe_result.capabilities
    );

    // 2. Jeśli operator jawnie zezwolił na PTT w zmiennej środowiskowej
    if std::env::var("SPLOG_HIL_ALLOW_LIVE_PTT")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        let operator_call =
            std::env::var("SPLOG_HIL_OPERATOR").unwrap_or_else(|_| "SP6INA".to_string());
        let token = HilPttConsentToken::new(&operator_call).expect("Valid token");

        let mut backend = splogbook::cat::factory::RigBackendFactory::create(&rig_config)
            .expect("Backend creation failed");

        backend.connect().await.expect("Physical connect failed");

        let ptt_result = session
            .execute_controlled_ptt(backend.as_mut(), &token, Duration::from_millis(50))
            .await
            .expect("Hardware PTT execution failed");

        backend.disconnect().await;

        println!(
            "Live HIL PTT pulse completed: {} ms (RX restored: {})",
            ptt_result.ptt_pulse_duration_ms, ptt_result.rx_restored_cleanly
        );
    }
}
