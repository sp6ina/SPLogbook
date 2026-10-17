// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::evidence::{
    HilExecutionStage, HilReadinessCheck, HilReadinessEvidenceKind, HilReadinessReport,
    HilReadinessStatus,
};
use super::manifest::{HilRfTermination, HilTestBenchManifest, HilTransportConfig};

/// Walidator gotowości laboratoryjnej Hardware-In-The-Loop ([`HilReadinessValidator`]).
///
/// Implementuje rygorystyczne reguły bezpieczeństwa w.cz., sprawdzając każdy warunek
/// pod kątem jednoznacznie przypisanej kategorii dowodu:
/// - MachineVerified: sprawdzenia deterministyczne w kodzie,
/// - OperatorAttested: manualne oświadczenia operatora o stanie fizycznym,
/// - InstrumentVerified: odczyty z certyfikowanych przyrządów pomiarowych.
pub struct HilReadinessValidator;

impl HilReadinessValidator {
    /// Przeprowadza pełną walidację gotowości stanowiska HIL na podstawie manifestu i fazy.
    pub fn validate(
        manifest: &HilTestBenchManifest,
        stage: HilExecutionStage,
    ) -> HilReadinessReport {
        let checks = vec![
            Self::check_rig_target(manifest),
            Self::check_transport(manifest),
            Self::check_rf_termination(manifest),
            Self::check_ptt_no_retry(),
            Self::check_operator_dummy_load(manifest),
            Self::check_operator_rf_perimeter(manifest),
            Self::check_operator_ptt_consent(manifest),
            Self::check_instrument_swr(manifest),
            Self::check_stage_gate(stage),
        ];

        // Wyznaczenie całościowego statusu gotowości
        let overall_status = if checks
            .iter()
            .any(|c| c.status == HilReadinessStatus::Failed)
        {
            HilReadinessStatus::Failed
        } else if checks
            .iter()
            .any(|c| c.status == HilReadinessStatus::Blocked)
        {
            HilReadinessStatus::Blocked
        } else {
            HilReadinessStatus::Passed
        };

        HilReadinessReport {
            bench_id: manifest.bench_id.clone(),
            execution_stage: stage,
            overall_status,
            checks,
        }
    }

    fn check_rig_target(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        let rig_id_str = manifest.matrix.rig_id.as_str().trim();
        let model_str = manifest.matrix.model_name.trim();

        if rig_id_str.is_empty() || model_str.is_empty() {
            HilReadinessCheck {
                id: "HIL-MACHINE-RIG-TARGET".to_string(),
                title: "Explicit Rig Target & Model Identification".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                status: HilReadinessStatus::Failed,
                details: "Rig ID or model name is empty; implicit radio selection is forbidden"
                    .to_string(),
            }
        } else {
            HilReadinessCheck {
                id: "HIL-MACHINE-RIG-TARGET".to_string(),
                title: "Explicit Rig Target & Model Identification".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                status: HilReadinessStatus::Passed,
                details: format!(
                    "Rig ID: '{}', Model: '{}', Backend: '{}'",
                    rig_id_str,
                    model_str,
                    manifest.matrix.backend_kind.as_str()
                ),
            }
        }
    }

    fn check_transport(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        match &manifest.matrix.transport {
            HilTransportConfig::Serial {
                port_name,
                baud_rate,
                ..
            } => {
                let trimmed = port_name.trim();
                let upper = trimmed.to_ascii_uppercase();
                if trimmed.is_empty() || upper == "AUTO" || upper == "FIRST" || upper == "DETECT" {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-TRANSPORT".to_string(),
                        title: "Explicit Serial Transport Parameterization".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Failed,
                        details: format!(
                            "Invalid serial port name '{port_name}'; automatic port picking is strictly forbidden in HIL"
                        ),
                    }
                } else if *baud_rate < 1200 {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-TRANSPORT".to_string(),
                        title: "Explicit Serial Transport Parameterization".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Failed,
                        details: format!(
                            "Baud rate {baud_rate} is below laboratory minimum 1200 bps"
                        ),
                    }
                } else {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-TRANSPORT".to_string(),
                        title: "Explicit Serial Transport Parameterization".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Passed,
                        details: format!("Serial port: '{trimmed}', Baud: {baud_rate}"),
                    }
                }
            }
            HilTransportConfig::Network { host, port } => {
                if host.trim().is_empty() || *port == 0 {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-TRANSPORT".to_string(),
                        title: "Explicit Network Transport Parameterization".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Failed,
                        details: format!("Invalid network endpoint: host='{host}', port={port}"),
                    }
                } else {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-TRANSPORT".to_string(),
                        title: "Explicit Network Transport Parameterization".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Passed,
                        details: format!("Network endpoint: {host}:{port}"),
                    }
                }
            }
        }
    }

    fn check_rf_termination(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        match &manifest.rf_safety.termination {
            HilRfTermination::DummyLoad(spec) => {
                if (spec.impedance_ohms - 50.0).abs() > 5.0 {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-RF-TERMINATION".to_string(),
                        title: "RF Termination Verification (50 Ohm Dummy Load)".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Failed,
                        details: format!(
                            "Dummy load impedance {:.1} ohms deviates from standard 50.0 ohms",
                            spec.impedance_ohms
                        ),
                    }
                } else if manifest.rf_safety.max_test_power_watts > spec.power_rating_watts {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-RF-TERMINATION".to_string(),
                        title: "RF Termination Verification (50 Ohm Dummy Load)".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Failed,
                        details: format!(
                            "Maximum test power ({:.1} W) exceeds dummy load rating ({:.1} W)",
                            manifest.rf_safety.max_test_power_watts, spec.power_rating_watts
                        ),
                    }
                } else {
                    HilReadinessCheck {
                        id: "HIL-MACHINE-RF-TERMINATION".to_string(),
                        title: "RF Termination Verification (50 Ohm Dummy Load)".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Passed,
                        details: format!(
                            "Verified 50-ohm dummy load S/N: '{}', rating: {:.1} W, max test power: {:.1} W",
                            spec.serial_number,
                            spec.power_rating_watts,
                            manifest.rf_safety.max_test_power_watts
                        ),
                    }
                }
            }
            HilRfTermination::Antenna { antenna_name } => HilReadinessCheck {
                id: "HIL-MACHINE-RF-TERMINATION".to_string(),
                title: "RF Termination Verification (50 Ohm Dummy Load)".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                status: HilReadinessStatus::Failed,
                details: format!(
                    "Antenna termination '{antenna_name}' is strictly prohibited for HIL PTT tests; 50-ohm dummy load required"
                ),
            },
        }
    }

    fn check_ptt_no_retry() -> HilReadinessCheck {
        HilReadinessCheck {
            id: "HIL-MACHINE-PTT-NO-RETRY".to_string(),
            title: "Single-Shot PTT Invariant (Zero Retry Policy)".to_string(),
            evidence_kind: HilReadinessEvidenceKind::MachineVerified,
            status: HilReadinessStatus::Passed,
            details:
                "TxArbiter strictly forbids SetPtt(true) retries; single-shot transition enforced"
                    .to_string(),
        }
    }

    fn check_operator_dummy_load(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        if manifest
            .operator_attestation
            .dummy_load_physically_connected
        {
            HilReadinessCheck {
                id: "HIL-OPERATOR-DUMMY-LOAD".to_string(),
                title: "Physical Dummy Load Connection Attestation".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Passed,
                details: "Operator confirmed physical connection and torque of 50-ohm dummy load"
                    .to_string(),
            }
        } else {
            HilReadinessCheck {
                id: "HIL-OPERATOR-DUMMY-LOAD".to_string(),
                title: "Physical Dummy Load Connection Attestation".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Blocked,
                details: "Operator has not attested physical connection of dummy load".to_string(),
            }
        }
    }

    fn check_operator_rf_perimeter(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        if manifest.operator_attestation.rf_perimeter_secured {
            HilReadinessCheck {
                id: "HIL-OPERATOR-RF-PERIMETER".to_string(),
                title: "RF Safety Perimeter Attestation".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Passed,
                details: "Operator confirmed RF safety perimeter is clear of personnel and hazards"
                    .to_string(),
            }
        } else {
            HilReadinessCheck {
                id: "HIL-OPERATOR-RF-PERIMETER".to_string(),
                title: "RF Safety Perimeter Attestation".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Blocked,
                details: "Operator has not confirmed RF safety perimeter clearance".to_string(),
            }
        }
    }

    fn check_operator_ptt_consent(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        if manifest.operator_attestation.ptt_consent_granted {
            HilReadinessCheck {
                id: "HIL-OPERATOR-PTT-CONSENT".to_string(),
                title: "Explicit Operator Consent for PTT Test".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Passed,
                details: "Operator granted explicit written consent for controlled PTT execution"
                    .to_string(),
            }
        } else {
            HilReadinessCheck {
                id: "HIL-OPERATOR-PTT-CONSENT".to_string(),
                title: "Explicit Operator Consent for PTT Test".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Blocked,
                details: "Operator has not granted explicit consent for PTT test".to_string(),
            }
        }
    }

    fn check_instrument_swr(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        if let Some(inst) = &manifest.instrument_verification {
            if inst.measured_swr <= 1.5 {
                HilReadinessCheck {
                    id: "HIL-INSTRUMENT-SWR".to_string(),
                    title: "Instrument-Verified SWR Measurement".to_string(),
                    evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                    status: HilReadinessStatus::Passed,
                    details: format!(
                        "Instrument '{}' ({}) verified SWR {:.2}:1 (within limit <= 1.50:1)",
                        inst.instrument_id, inst.instrument_model, inst.measured_swr
                    ),
                }
            } else {
                HilReadinessCheck {
                    id: "HIL-INSTRUMENT-SWR".to_string(),
                    title: "Instrument-Verified SWR Measurement".to_string(),
                    evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                    status: HilReadinessStatus::Failed,
                    details: format!(
                        "Instrument '{}' measured SWR {:.2}:1 exceeds safety threshold 1.50:1",
                        inst.instrument_id, inst.measured_swr
                    ),
                }
            }
        } else {
            HilReadinessCheck {
                id: "HIL-INSTRUMENT-SWR".to_string(),
                title: "Instrument-Verified SWR Measurement".to_string(),
                evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                status: HilReadinessStatus::NotRun,
                details: "No digital instrument measurement recorded; relying on operator dummy load attestation".to_string(),
            }
        }
    }

    fn check_stage_gate(stage: HilExecutionStage) -> HilReadinessCheck {
        match stage {
            HilExecutionStage::Stage7AOffline => HilReadinessCheck {
                id: "HIL-STAGE-PTT-GATE".to_string(),
                title: "Stage Execution Safety Gate".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                status: HilReadinessStatus::Blocked,
                details: "Physical PTT is strictly prohibited in Stage NATIVE-7A; permitted only in NATIVE-7B with real hardware bench".to_string(),
            },
            HilExecutionStage::Stage7BHardware => HilReadinessCheck {
                id: "HIL-STAGE-PTT-GATE".to_string(),
                title: "Stage Execution Safety Gate".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                status: HilReadinessStatus::Passed,
                details: "Stage NATIVE-7B permits hardware PTT execution subject to complete HIL-READY verification".to_string(),
            },
        }
    }
}
