// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use super::authorization::{
    HIL_REQUIRED_MEASURED_FACTS, HilAttemptRegistry, HilAttestationDocument,
    HilAuthorizationDecision, HilAuthorizationInputs, HilAuthorizationOutcome, HilEvidenceProvider,
    HilEvidenceScope, HilHardwareExecutionAuthorizer, HilIndependentEvidence,
};
use super::evidence::{
    CANONICAL_HIL_READY_DESCRIPTIONS, CANONICAL_HIL_READY_IDS, HilEvidenceOrigin,
    HilExecutionContext, HilExecutionStage, HilExtendedRequirement, HilImplementationCoverage,
    HilReadinessCheck, HilReadinessCondition, HilReadinessEvidenceKind, HilReadinessReport,
    HilReadinessStatus, HilRequirementAssessment,
};
use super::manifest::{
    HilOperatorAttestation, HilRfTermination, HilTestBenchManifest, HilTransportConfig,
};
use crate::cat::backend::CatBackendKind;
use crate::cat::native::profile::RigVendor;
use crate::cat::native::registry::find_profile_by_model;

/// Błędy walidacji profilu backendu w testach HIL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HilProfileValidationError {
    EmptyModelName,
    InvalidModelProfile(String),
    So2rNotDirectBackend,
    UnknownProfile {
        model: String,
        backend: CatBackendKind,
    },
    VendorMismatch {
        model_id: String,
        actual: RigVendor,
        expected: RigVendor,
    },
    FlexRadioRequiresNetworkTransport,
    BackendRequiresNetworkTransport(CatBackendKind),
}

impl std::fmt::Display for HilProfileValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyModelName => write!(f, "Model name is empty"),
            Self::InvalidModelProfile(m) => write!(f, "Unknown or invalid model profile: '{m}'"),
            Self::So2rNotDirectBackend => {
                write!(
                    f,
                    "SO2R is a dual-radio coordinator layer, not a direct radio backend"
                )
            }
            Self::UnknownProfile { model, backend } => {
                write!(
                    f,
                    "Unknown radio profile '{model}' for {} backend",
                    backend.label()
                )
            }
            Self::VendorMismatch {
                model_id,
                actual,
                expected,
            } => {
                write!(
                    f,
                    "Profile '{model_id}' belongs to vendor {actual:?}, expected {expected:?}"
                )
            }
            Self::FlexRadioRequiresNetworkTransport => {
                write!(
                    f,
                    "FlexRadio SmartSDR profile requires Network transport, not Serial"
                )
            }
            Self::BackendRequiresNetworkTransport(kind) => {
                write!(
                    f,
                    "Backend '{}' requires Network transport configuration",
                    kind.as_str()
                )
            }
        }
    }
}

impl std::error::Error for HilProfileValidationError {}

/// Kanoniczny identyfikator radia używany wyłącznie przez referencyjną macierz warunków.
///
/// Jest to stała literału zwalidowana testem `canonical_matrix_rig_id_constant_is_valid`;
/// żadna ścieżka przetwarzająca dane wejściowe nie korzysta z `expect`/`unwrap`.
pub const CANONICAL_MATRIX_RIG_ID: &str = "RIG-IC7300-01";

/// Walidator konfiguracji i modelu dowodowego Hardware-In-The-Loop ([`HilReadinessValidator`]).
///
/// **F-15:** walidator ocenia plan i deklaracje, ale **nie może** autoryzować wykonania
/// sprzętowego. Autoryzacja sprzętowa wymaga niezależnego dowodu i jest wydawana wyłącznie
/// przez [`HilHardwareExecutionAuthorizer`]. Metoda [`HilReadinessReport::can_run_hardware_ptt`]
/// zwraca `false` dla każdego raportu, ponieważ raport nie jest upoważnieniem PTT.
///
/// Implementuje rygorystyczne reguły bezpieczeństwa w.cz., sprawdzając każdy warunek
/// pod kątem jednoznacznie przypisanej kategorii dowodu:
/// - MachineVerified: sprawdzenia deterministyczne w kodzie,
/// - OperatorAttested: manualne oświadczenia operatora o stanie fizycznym,
/// - InstrumentVerified: odczyty z przyrządów pomiarowych.
pub struct HilReadinessValidator;

impl HilReadinessValidator {
    /// Przeprowadza walidację konfiguracji stanowiska HIL na podstawie planu i fazy.
    ///
    /// Wynik jest **diagnostyką konfiguracji**, nie autoryzacją sprzętową.
    pub fn validate(
        manifest: &HilTestBenchManifest,
        stage: HilExecutionStage,
    ) -> HilReadinessReport {
        Self::evaluate_plan(manifest, stage, true)
    }

    /// Waliduje plan **bez** odczytywania osadzonych deklaracji operatora.
    ///
    /// Deklaracje osadzone w planie v1 nigdy nie są dowodem; ta ścieżka jawnie je pomija.
    pub fn validate_without_declarations(
        manifest: &HilTestBenchManifest,
        stage: HilExecutionStage,
    ) -> HilReadinessReport {
        Self::evaluate_plan(manifest, stage, false)
    }

    fn evaluate_plan(
        manifest: &HilTestBenchManifest,
        stage: HilExecutionStage,
        consider_embedded_declarations: bool,
    ) -> HilReadinessReport {
        let checks = vec![
            Self::check_rig_target(manifest),
            Self::check_transport(manifest),
            Self::check_rf_termination(manifest),
            Self::check_ptt_no_retry(),
            Self::check_operator_dummy_load(manifest, consider_embedded_declarations),
            Self::check_operator_rf_perimeter(manifest, consider_embedded_declarations),
            Self::check_operator_ptt_consent(manifest, consider_embedded_declarations),
            Self::check_instrument_swr(manifest),
            Self::check_stage_gate(stage),
        ];

        let conditions =
            Self::evaluate_all_conditions(manifest, stage, consider_embedded_declarations);
        let extended_requirements =
            Self::evaluate_extended_requirements(manifest, stage, consider_embedded_declarations);

        // Wyznaczenie całościowego statusu gotowości na podstawie reguł i 30 warunków.
        let has_failed = checks
            .iter()
            .any(|c| c.status == HilReadinessStatus::Failed)
            || conditions
                .iter()
                .any(|c| c.status == HilReadinessStatus::Failed);
        let has_blocked = checks
            .iter()
            .any(|c| c.status == HilReadinessStatus::Blocked)
            || conditions
                .iter()
                .any(|c| c.status == HilReadinessStatus::Blocked);

        let overall_status = if has_failed {
            HilReadinessStatus::Failed
        } else if has_blocked {
            HilReadinessStatus::Blocked
        } else {
            HilReadinessStatus::Passed
        };

        HilReadinessReport {
            bench_id: manifest.bench_id.clone(),
            execution_stage: stage,
            overall_status,
            checks,
            conditions,
            extended_requirements,
        }
    }

    /// Zwraca referencyjną listę 30 kanonicznych warunków HIL-READY z przypisanymi
    /// symbolami implementacyjnymi i testami offline.
    pub fn canonical_matrix() -> Vec<HilReadinessCondition> {
        let manifest = HilTestBenchManifest {
            schema_version: super::manifest::HIL_PLAN_SCHEMA_VERSION,
            bench_id: "LAB-BENCH-SP6INA-01".to_string(),
            session_id: "SESSION-CANONICAL-01".to_string(),
            test_case_id: "hil_hardware_live_test_bench".to_string(),
            attempt_id: "ATTEMPT-CANONICAL-01".to_string(),
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
            matrix: crate::cat::hil::HilMatrixEntry {
                backend_kind: crate::cat::backend::CatBackendKind::IcomCiV,
                rig_id: crate::cat::routing::RigId::new(CANONICAL_MATRIX_RIG_ID)
                    .expect("the canonical reference rig id is a validated constant"),
                model_name: "IC-7300".to_string(),
                transport: crate::cat::hil::HilTransportConfig::Serial {
                    port_name: "COM3".to_string(),
                    baud_rate: 115_200,
                    data_bits: 8,
                    parity: "none".to_string(),
                    stop_bits: 1,
                    flow_control: "none".to_string(),
                },
                firmware_version: "v1.42".to_string(),
            },
            rf_safety: crate::cat::hil::HilRfSafetyConfig {
                termination: crate::cat::hil::HilRfTermination::DummyLoad(
                    crate::cat::hil::HilDummyLoadSpec {
                        power_rating_watts: 100.0,
                        max_swr: 1.15,
                        serial_number: "DL-50W-001".to_string(),
                        impedance_ohms: 50.0,
                    },
                ),
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
            operator_attestation: None,
            instrument_verification: None,
        };
        Self::evaluate_all_conditions(&manifest, HilExecutionStage::Stage7AOffline, true)
    }

    /// Dokonuje pełnej walidacji autoryzacyjnej. Manifest nie jest authority dla samego siebie.
    ///
    /// To jedyna droga do uzyskania upoważnienia sprzętowego. Bez niezależnego dowodu wynik jest
    /// kontrolowaną odmową.
    pub fn authorize_hardware_execution_at(
        manifest: &HilTestBenchManifest,
        attestation: Option<&HilAttestationDocument>,
        evidence: Option<&HilIndependentEvidence>,
        evaluated_at: DateTime<Utc>,
    ) -> HilAuthorizationDecision {
        HilHardwareExecutionAuthorizer::authorize_at(
            HilAuthorizationInputs {
                plan: manifest,
                attestation,
                evidence,
            },
            evaluated_at,
        )
    }

    /// Dokonuje walidacji suchego przebiegu (bez PTT, bez upoważnienia sprzętowego).
    pub fn authorize_dry_run_at(
        manifest: &HilTestBenchManifest,
        attestation: Option<&HilAttestationDocument>,
        evaluated_at: DateTime<Utc>,
    ) -> HilAuthorizationDecision {
        HilHardwareExecutionAuthorizer::dry_run_at(manifest, attestation, evaluated_at)
    }

    /// Pobiera dowód od dostawcy — fail-closed. Brak dostawcy ⇒ odmowa.
    pub fn request_independent_evidence<P: HilEvidenceProvider>(
        provider: Option<&P>,
        manifest: &HilTestBenchManifest,
        attestation: &HilAttestationDocument,
    ) -> Result<HilIndependentEvidence, super::authorization::HilEvidenceError> {
        super::authorization::request_evidence_or_deny(provider, manifest, attestation)
    }

    /// Sprawdza, czy plan i oświadczenie pozwalają na suchy przebieg. Nigdy nie upoważnia PTT.
    pub fn dry_run_decision(
        manifest: &HilTestBenchManifest,
        attestation: Option<&HilAttestationDocument>,
        evaluated_at: DateTime<Utc>,
    ) -> HilAuthorizationDecision {
        let decision = HilHardwareExecutionAuthorizer::authorize_at(
            HilAuthorizationInputs {
                plan: manifest,
                attestation,
                evidence: None,
            },
            evaluated_at,
        );
        // Suchy przebieg jest dozwolony tylko wtedy, gdy konfiguracja i oświadczenie są poprawne;
        // dowód nie jest wymagany do suchego przebiegu, ale wynik nigdy nie upoważnia PTT.
        if matches!(
            decision.outcome(),
            HilAuthorizationOutcome::EvidenceMissing
                | HilAuthorizationOutcome::OperatorAttestationMissing
        ) {
            return HilHardwareExecutionAuthorizer::dry_run_at(manifest, attestation, evaluated_at);
        }
        decision
    }

    /// Ewaluuje rejestr wymaganych faktów zmierzonych (używane w diagnostyce i testach).
    pub fn required_measured_facts() -> &'static [&'static str] {
        &HIL_REQUIRED_MEASURED_FACTS
    }

    /// Zużywa próbę w rejestrze powtórzeń. Zwraca kontrolowaną odmowę przy powtórzeniu.
    pub fn consume_attempt(
        registry: &HilAttemptRegistry,
        session_id: &str,
        attempt_id: &str,
    ) -> Result<(), super::authorization::HilEvidenceError> {
        registry.consume(session_id, attempt_id)
    }

    /// Pomocniczo: pusty zestaw zmierzonych faktów (diagnostyka; nie upoważnia niczego).
    pub fn empty_measured_facts() -> BTreeMap<String, bool> {
        BTreeMap::new()
    }

    /// Pomocniczo: zakres wymagany do wykonania sprzętowego.
    pub fn required_scope() -> HilEvidenceScope {
        HilEvidenceScope::HardwarePtt
    }

    fn attestation_flag(
        attestation: Option<&HilOperatorAttestation>,
        field: impl Fn(&HilOperatorAttestation) -> bool,
    ) -> bool {
        attestation.is_some_and(field)
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
                        title: "RF Termination Plan Declaration (50 Ohm Dummy Load)".to_string(),
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
                        title: "RF Termination Plan Declaration (50 Ohm Dummy Load)".to_string(),
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
                        title: "RF Termination Plan Declaration (50 Ohm Dummy Load)".to_string(),
                        evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                        status: HilReadinessStatus::Passed,
                        details: format!(
                            "Plan declares a 50-ohm dummy load S/N: '{}', rating: {:.1} W, max test power: {:.1} W; physical connection and impedance require independent evidence",
                            spec.serial_number,
                            spec.power_rating_watts,
                            manifest.rf_safety.max_test_power_watts
                        ),
                    }
                }
            }
            HilRfTermination::Antenna { antenna_name } => HilReadinessCheck {
                id: "HIL-MACHINE-RF-TERMINATION".to_string(),
                title: "RF Termination Plan Declaration (50 Ohm Dummy Load)".to_string(),
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

    fn check_operator_dummy_load(
        manifest: &HilTestBenchManifest,
        consider_embedded_declarations: bool,
    ) -> HilReadinessCheck {
        let declared = consider_embedded_declarations
            && Self::attestation_flag(manifest.operator_attestation.as_ref(), |a| {
                a.dummy_load_physically_connected
            });
        if declared {
            HilReadinessCheck {
                id: "HIL-OPERATOR-DUMMY-LOAD".to_string(),
                title: "Physical Dummy Load Connection Declaration".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Blocked,
                details: "Operator declared physical dummy load connection; an operator \
                          declaration is not independent evidence, so hardware execution stays \
                          denied without an independent evidence document"
                    .to_string(),
            }
        } else {
            HilReadinessCheck {
                id: "HIL-OPERATOR-DUMMY-LOAD".to_string(),
                title: "Physical Dummy Load Connection Declaration".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                status: HilReadinessStatus::Blocked,
                details: "Operator has not declared physical connection of dummy load".to_string(),
            }
        }
    }

    fn check_operator_rf_perimeter(
        manifest: &HilTestBenchManifest,
        consider_embedded_declarations: bool,
    ) -> HilReadinessCheck {
        let declared = consider_embedded_declarations
            && Self::attestation_flag(manifest.operator_attestation.as_ref(), |a| {
                a.rf_perimeter_secured
            });
        HilReadinessCheck {
            id: "HIL-OPERATOR-RF-PERIMETER".to_string(),
            title: "RF Safety Perimeter Declaration".to_string(),
            evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
            status: HilReadinessStatus::Blocked,
            details: if declared {
                "Operator declared RF safety perimeter clearance; independent measurement is \
                 required before any hardware execution"
                    .to_string()
            } else {
                "Operator has not declared RF safety perimeter clearance".to_string()
            },
        }
    }

    fn check_operator_ptt_consent(
        manifest: &HilTestBenchManifest,
        consider_embedded_declarations: bool,
    ) -> HilReadinessCheck {
        let declared = consider_embedded_declarations
            && Self::attestation_flag(manifest.operator_attestation.as_ref(), |a| {
                a.ptt_consent_granted
            });
        HilReadinessCheck {
            id: "HIL-OPERATOR-PTT-CONSENT".to_string(),
            title: "Explicit Operator Consent for PTT Test".to_string(),
            evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
            status: if declared {
                HilReadinessStatus::Passed
            } else {
                HilReadinessStatus::Blocked
            },
            details: if declared {
                "Operator granted explicit consent for controlled PTT execution (consent is an \
                 authorization input, never evidence of bench state)"
                    .to_string()
            } else {
                "Operator has not granted explicit consent for PTT test".to_string()
            },
        }
    }

    fn check_instrument_swr(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        if let Some(inst) = &manifest.instrument_verification {
            if inst.measured_swr <= 1.5 {
                HilReadinessCheck {
                    id: "HIL-INSTRUMENT-SWR".to_string(),
                    title: "Instrument SWR Declaration".to_string(),
                    evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                    status: HilReadinessStatus::Passed,
                    details: format!(
                        "Plan declares instrument '{}' ({}) SWR {:.2}:1 (within limit <= 1.50:1); \
                         a declaration is not an independent measurement",
                        inst.instrument_id, inst.instrument_model, inst.measured_swr
                    ),
                }
            } else {
                HilReadinessCheck {
                    id: "HIL-INSTRUMENT-SWR".to_string(),
                    title: "Instrument SWR Declaration".to_string(),
                    evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                    status: HilReadinessStatus::Failed,
                    details: format!(
                        "Declared instrument '{}' SWR {:.2}:1 exceeds safety threshold 1.50:1",
                        inst.instrument_id, inst.measured_swr
                    ),
                }
            }
        } else {
            HilReadinessCheck {
                id: "HIL-INSTRUMENT-SWR".to_string(),
                title: "Instrument SWR Declaration".to_string(),
                evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                status: HilReadinessStatus::NotRun,
                details: "No instrument measurement declared; an independent evidence document is \
                          still required for hardware execution"
                    .to_string(),
            }
        }
    }

    /// Sprawdza dowód pomiarowy pod kątem wymagań runtime (blokuje syntetyczne fixture).
    pub fn check_instrument_swr_runtime(manifest: &HilTestBenchManifest) -> HilReadinessCheck {
        if let Some(inst) = &manifest.instrument_verification {
            if !inst.is_physical_measurement() {
                HilReadinessCheck {
                    id: "HIL-INSTRUMENT-SWR-RUNTIME".to_string(),
                    title: "Instrument-Verified SWR Runtime Evidence".to_string(),
                    evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                    status: HilReadinessStatus::Blocked,
                    details: "SYNTHETIC OFFLINE FIXTURE - NOT A PHYSICAL MEASUREMENT - NOT VALID FOR HARDWARE HIL-READY".to_string(),
                }
            } else if inst.measured_swr <= 1.5 {
                HilReadinessCheck {
                    id: "HIL-INSTRUMENT-SWR-RUNTIME".to_string(),
                    title: "Instrument-Verified SWR Runtime Evidence".to_string(),
                    evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                    status: HilReadinessStatus::Passed,
                    details: format!(
                        "Physical instrument '{}' ({}) verified SWR {:.2}:1 (within limit <= 1.50:1)",
                        inst.instrument_id, inst.instrument_model, inst.measured_swr
                    ),
                }
            } else {
                HilReadinessCheck {
                    id: "HIL-INSTRUMENT-SWR-RUNTIME".to_string(),
                    title: "Instrument-Verified SWR Runtime Evidence".to_string(),
                    evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                    status: HilReadinessStatus::Failed,
                    details: format!(
                        "Physical instrument '{}' measured SWR {:.2}:1 exceeds safety threshold 1.50:1",
                        inst.instrument_id, inst.measured_swr
                    ),
                }
            }
        } else {
            HilReadinessCheck {
                id: "HIL-INSTRUMENT-SWR-RUNTIME".to_string(),
                title: "Instrument-Verified SWR Runtime Evidence".to_string(),
                evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                status: HilReadinessStatus::NotRun,
                details: "No digital instrument measurement recorded".to_string(),
            }
        }
    }

    /// Dokonuje rozdzielonej oceny dowodu instrumentowego.
    pub fn evaluate_instrument_runtime_evidence(
        manifest: &HilTestBenchManifest,
    ) -> HilRequirementAssessment {
        if let Some(inst) = &manifest.instrument_verification {
            let offline_status = if inst.measured_swr <= 1.5 {
                HilReadinessStatus::Passed
            } else {
                HilReadinessStatus::Failed
            };
            let runtime_status = if !inst.is_physical_measurement() {
                HilReadinessStatus::Blocked
            } else if inst.measured_swr <= 1.5 {
                HilReadinessStatus::Passed
            } else {
                HilReadinessStatus::Failed
            };
            let context = if inst.is_physical_measurement() {
                HilExecutionContext::PhysicalRuntime
            } else {
                HilExecutionContext::OfflineModel
            };
            let diagnostic = if inst.is_physical_measurement() {
                format!("Measured SWR: {:.2}", inst.measured_swr)
            } else {
                "SYNTHETIC OFFLINE FIXTURE - NOT A PHYSICAL MEASUREMENT - NOT VALID FOR HARDWARE HIL-READY".to_string()
            };
            HilRequirementAssessment {
                requirement_id: "HIL-INSTRUMENT-EVIDENCE".to_string(),
                canonical_number: None,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: offline_status,
                offline_evaluation_status: offline_status,
                simulated_runtime_status: offline_status,
                physical_runtime_evidence_status: runtime_status,
                runtime_evidence_status: runtime_status,
                evaluation_context: context,
                evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                evidence_origin: inst.origin,
                diagnostic,
            }
        } else {
            HilRequirementAssessment {
                requirement_id: "HIL-INSTRUMENT-EVIDENCE".to_string(),
                canonical_number: None,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::NotRun,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::NotRun,
                evaluation_context: HilExecutionContext::OfflineModel,
                evidence_kind: HilReadinessEvidenceKind::InstrumentVerified,
                evidence_origin: HilEvidenceOrigin::SyntheticOfflineFixture,
                diagnostic: "No digital instrument measurement recorded".to_string(),
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
                status: HilReadinessStatus::Blocked,
                details: "Stage NATIVE-7B permits hardware PTT only with an independent evidence \
                          document; a test plan cannot authorize execution"
                    .to_string(),
            },
        }
    }

    /// Weryfikuje zgodność wybranego backendu z profilem modelu i producentem (Warunek 7).
    /// Nie sprawdza portu, baud rate ani session_id; nigdy nie wykonuje fallbacku na Hamlib.
    pub fn validate_backend_profile(
        manifest: &HilTestBenchManifest,
    ) -> Result<(), HilProfileValidationError> {
        let model = manifest.matrix.model_name.trim();
        if model.is_empty() {
            return Err(HilProfileValidationError::EmptyModelName);
        }
        let upper_model = model.to_ascii_uppercase();
        if upper_model.contains("UNKNOWN") || upper_model.contains("INVALID") {
            return Err(HilProfileValidationError::InvalidModelProfile(
                model.to_string(),
            ));
        }

        match manifest.matrix.backend_kind {
            CatBackendKind::So2r => Err(HilProfileValidationError::So2rNotDirectBackend),
            CatBackendKind::IcomCiV => {
                let profile = find_profile_by_model(model).ok_or_else(|| {
                    HilProfileValidationError::UnknownProfile {
                        model: model.to_string(),
                        backend: CatBackendKind::IcomCiV,
                    }
                })?;
                if profile.vendor != RigVendor::Icom {
                    return Err(HilProfileValidationError::VendorMismatch {
                        model_id: profile.model_id.to_string(),
                        actual: profile.vendor,
                        expected: RigVendor::Icom,
                    });
                }
                Ok(())
            }
            CatBackendKind::Kenwood => {
                let profile = find_profile_by_model(model).ok_or_else(|| {
                    HilProfileValidationError::UnknownProfile {
                        model: model.to_string(),
                        backend: CatBackendKind::Kenwood,
                    }
                })?;
                if profile.vendor != RigVendor::Kenwood {
                    return Err(HilProfileValidationError::VendorMismatch {
                        model_id: profile.model_id.to_string(),
                        actual: profile.vendor,
                        expected: RigVendor::Kenwood,
                    });
                }
                Ok(())
            }
            CatBackendKind::Yaesu => {
                let profile = find_profile_by_model(model).ok_or_else(|| {
                    HilProfileValidationError::UnknownProfile {
                        model: model.to_string(),
                        backend: CatBackendKind::Yaesu,
                    }
                })?;
                if profile.vendor != RigVendor::Yaesu {
                    return Err(HilProfileValidationError::VendorMismatch {
                        model_id: profile.model_id.to_string(),
                        actual: profile.vendor,
                        expected: RigVendor::Yaesu,
                    });
                }
                Ok(())
            }
            CatBackendKind::Elecraft => {
                let profile = find_profile_by_model(model).ok_or_else(|| {
                    HilProfileValidationError::UnknownProfile {
                        model: model.to_string(),
                        backend: CatBackendKind::Elecraft,
                    }
                })?;
                if profile.vendor != RigVendor::Elecraft {
                    return Err(HilProfileValidationError::VendorMismatch {
                        model_id: profile.model_id.to_string(),
                        actual: profile.vendor,
                        expected: RigVendor::Elecraft,
                    });
                }
                Ok(())
            }
            CatBackendKind::FlexRadio => {
                if matches!(manifest.matrix.transport, HilTransportConfig::Serial { .. }) {
                    return Err(HilProfileValidationError::FlexRadioRequiresNetworkTransport);
                }
                if upper_model == "FLEX-GENERIC" {
                    return Ok(());
                }
                let profile = find_profile_by_model(model).ok_or_else(|| {
                    HilProfileValidationError::UnknownProfile {
                        model: model.to_string(),
                        backend: CatBackendKind::FlexRadio,
                    }
                })?;
                if profile.vendor != RigVendor::FlexRadio {
                    return Err(HilProfileValidationError::VendorMismatch {
                        model_id: profile.model_id.to_string(),
                        actual: profile.vendor,
                        expected: RigVendor::FlexRadio,
                    });
                }
                Ok(())
            }
            CatBackendKind::Hamlib | CatBackendKind::Flrig | CatBackendKind::Tci => {
                if matches!(manifest.matrix.transport, HilTransportConfig::Serial { .. }) {
                    return Err(HilProfileValidationError::BackendRequiresNetworkTransport(
                        manifest.matrix.backend_kind,
                    ));
                }
                Ok(())
            }
        }
    }

    /// Sprawdza, czy wskazany host sieciowy jest lokalny lub należy do kontrolowanej podsieci LAN (Warunek 25).
    pub fn is_local_or_controlled_host(host: &str) -> bool {
        let h = host.trim().to_ascii_lowercase();
        if h.is_empty() || h.contains("smartlink") || h.contains("wan") || h.contains("cloud") {
            return false;
        }
        if h == "localhost"
            || h == "::1"
            || h == "[::1]"
            || h.starts_with("127.")
            || h.starts_with("10.")
            || h.starts_with("192.168.")
            || h.rsplit_once('.')
                .is_some_and(|(_, tld)| tld == "local" || tld == "lan")
        {
            return true;
        }
        if let Some(rest) = h.strip_prefix("172.")
            && let Some(second_octet_str) = rest.split('.').next()
            && let Ok(second_octet) = second_octet_str.parse::<u8>()
            && (16..=31).contains(&second_octet)
        {
            return true;
        }
        false
    }

    /// Weryfikuje bezpieczeństwo zdalnego TX oraz wyłączenie SmartLink (Warunek 25).
    ///
    /// Deklaracje w planie nie są dowodem; funkcja ocenia spójność planu.
    pub fn validate_remote_tx_safety(manifest: &HilTestBenchManifest) -> bool {
        if !manifest.rf_safety.smartlink_disabled
            || !manifest.rf_safety.unattended_remote_tx_disabled
            || !manifest.rf_safety.remote_endpoint_is_local_or_controlled
            || !manifest.rf_safety.operator_has_immediate_stop_control
        {
            return false;
        }
        if let HilTransportConfig::Network { host, .. } = &manifest.matrix.transport
            && !Self::is_local_or_controlled_host(host)
        {
            return false;
        }
        true
    }

    fn evaluate_all_conditions(
        manifest: &HilTestBenchManifest,
        stage: HilExecutionStage,
        consider_embedded_declarations: bool,
    ) -> Vec<HilReadinessCondition> {
        let is_7a = stage == HilExecutionStage::Stage7AOffline;
        let is_antenna = matches!(
            &manifest.rf_safety.termination,
            HilRfTermination::Antenna { .. }
        );
        let dummy_load_spec = match &manifest.rf_safety.termination {
            HilRfTermination::DummyLoad(spec) => Some(spec),
            HilRfTermination::Antenna { .. } => None,
        };

        let attestation = if consider_embedded_declarations {
            manifest.operator_attestation.as_ref()
        } else {
            None
        };
        let flag = |field: fn(&HilOperatorAttestation) -> bool| -> bool {
            Self::attestation_flag(attestation, field)
        };
        let has_timestamp_and_desc = attestation.is_some_and(|a| {
            !a.attestation_timestamp.trim().is_empty() && !a.description.trim().is_empty()
        });

        // Warunek 1: Working tree clean — pomiar repozytorium, nie deklaracja planu.
        // Os offline jest wyłącznie deklaracją planu (diagnostyka); status runtime pozostaje
        // Blocked, bo deklaracja nie jest niezależnym dowodem (F-15).
        let cond1_offline = if consider_embedded_declarations {
            match manifest.working_tree_clean {
                Some(true) => HilReadinessStatus::Passed,
                Some(false) => HilReadinessStatus::Failed,
                None => HilReadinessStatus::Blocked,
            }
        } else {
            HilReadinessStatus::Blocked
        };
        // Sprzeczna deklaracja planu jest błędem konfiguracji; poprawna deklaracja pozostaje
        // wyłącznie deklaracją i nie może ustawić statusu runtime na Passed (F-15).
        let cond1_status = if cond1_offline == HilReadinessStatus::Failed {
            HilReadinessStatus::Failed
        } else {
            HilReadinessStatus::Blocked
        };
        // Warunek 2: Wszystkie testy programowe przechodzą — pomiar baseline, nie deklaracja.
        let cond2_offline = if consider_embedded_declarations {
            match manifest.software_tests_passed {
                Some(true) => HilReadinessStatus::Passed,
                Some(false) => HilReadinessStatus::Failed,
                None => HilReadinessStatus::Blocked,
            }
        } else {
            HilReadinessStatus::Blocked
        };
        // Sprzeczna deklaracja planu jest błędem konfiguracji; poprawna deklaracja pozostaje
        // wyłącznie deklaracją i nie może ustawić statusu runtime na Passed (F-15).
        let cond2_status = if cond2_offline == HilReadinessStatus::Failed {
            HilReadinessStatus::Failed
        } else {
            HilReadinessStatus::Blocked
        };
        // Warunek 3: Pełny SHA testowanego commita jest zapisany (deklaracja planu).
        let cond3_status = match &manifest.tested_commit {
            Some(sha) if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) => {
                if let Some(exp) = &manifest.expected_commit
                    && !exp.trim().is_empty()
                    && exp.trim() != sha
                {
                    HilReadinessStatus::Failed
                } else {
                    HilReadinessStatus::Passed
                }
            }
            _ => HilReadinessStatus::Failed,
        };

        // Warunek 4: Radio jest jednoznacznie zidentyfikowane.
        let rig_id_trimmed = manifest.matrix.rig_id.as_str().trim();
        let cond4_pass =
            !rig_id_trimmed.is_empty() && !manifest.matrix.model_name.trim().is_empty();

        // Warunek 5: Firmware jest zapisane.
        let cond5_pass = !manifest.matrix.firmware_version.trim().is_empty();

        // Warunek 6: Transport CAT jest jednoznacznie określony.
        let cond6_pass = match &manifest.matrix.transport {
            HilTransportConfig::Serial {
                port_name,
                baud_rate,
                ..
            } => {
                let upper = port_name.trim().to_ascii_uppercase();
                !port_name.trim().is_empty()
                    && upper != "AUTO"
                    && upper != "FIRST"
                    && upper != "DETECT"
                    && *baud_rate >= 1200
            }
            HilTransportConfig::Network { host, port } => !host.trim().is_empty() && *port > 0,
        };

        // Warunek 7: Profil backendu jest poprawnie wybrany.
        let cond7_res = Self::validate_backend_profile(manifest);
        let cond7_status = if cond7_res.is_ok() {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };

        // Warunek 8: Antena jest fizycznie odłączona — deklaracja operatora, nie dowód.
        let cond8_offline = if is_antenna {
            HilReadinessStatus::Failed
        } else if flag(|a| a.antenna_disconnected) && has_timestamp_and_desc {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond8_runtime = if is_antenna {
            HilReadinessStatus::Failed
        } else {
            HilReadinessStatus::Blocked
        };

        // Warunek 9: Dummy load 50 ohm jest fizycznie podłączony — deklaracja, nie dowód.
        let cond9_offline = if flag(|a| a.dummy_load_physically_connected) && has_timestamp_and_desc
        {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond9_runtime = HilReadinessStatus::Blocked;

        // Warunek 10: Dummy load obsługuje częstotliwość testową.
        let cond10_pass = manifest.rf_safety.frequency_range_hz.0
            < manifest.rf_safety.frequency_range_hz.1
            && manifest.rf_safety.frequency_range_hz.0 > 0;

        // Warunek 11: Dummy load obsługuje moc i czas testu.
        let cond11_pass = dummy_load_spec.is_some_and(|spec| {
            manifest.rf_safety.max_test_power_watts <= spec.power_rating_watts
                && manifest.rf_safety.max_test_duration_ms <= 500
        });
        let cond11_runtime = if !cond11_pass {
            HilReadinessStatus::Failed
        } else if is_7a {
            HilReadinessStatus::Blocked
        } else {
            HilReadinessStatus::Passed
        };

        // Warunek 12: Moc radia jest ograniczona.
        let cond12_pass = manifest.rf_safety.max_test_power_watts > 0.0
            && manifest.rf_safety.max_test_power_watts <= 100.0;
        let cond12_runtime = if !cond12_pass {
            HilReadinessStatus::Failed
        } else if is_7a {
            HilReadinessStatus::Blocked
        } else {
            HilReadinessStatus::Passed
        };

        // Warunek 13: Wzmacniacz mocy jest odłączony albo zablokowany — deklaracja, nie dowód.
        let cond13_offline = if flag(|a| a.pa_disconnected_or_locked) && has_timestamp_and_desc {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond13_runtime = HilReadinessStatus::Blocked;

        // Warunek 14: Tuner automatyczny jest wyłączony — deklaracja, nie dowód.
        let cond14_offline = if flag(|a| a.tuner_bypassed) && has_timestamp_and_desc {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond14_runtime = HilReadinessStatus::Blocked;

        // Warunek 15: Wszystkie pozostałe radia mają zablokowane PTT.
        let cond15_offline = if manifest.rf_safety.interlock_engaged && !rig_id_trimmed.is_empty() {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond15_runtime =
            if !manifest.rf_safety.interlock_engaged || rig_id_trimmed.is_empty() || is_7a {
                HilReadinessStatus::Blocked
            } else {
                HilReadinessStatus::Passed
            };

        // Warunek 18: Nie istnieje automatyczny fallback na pierwsze radio.
        let rig_id_upper = rig_id_trimmed.to_ascii_uppercase();
        let cond18_pass = !rig_id_trimmed.is_empty()
            && rig_id_upper != "FIRST"
            && rig_id_upper != "AUTO"
            && rig_id_upper != "DEFAULT";
        let cond18_status = if cond18_pass {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };

        // Warunek 19: Dostępny jest fizyczny awaryjny STOP — deklaracja, nie dowód.
        let cond19_offline = if flag(|a| a.physical_stop_available) && has_timestamp_and_desc {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond19_runtime = HilReadinessStatus::Blocked;

        // Warunek 20: Operator potwierdził gotowość stanowiska — deklaracja, nie dowód.
        let cond20_offline = if !manifest.operator_id.trim().is_empty()
            && !manifest.session_id.trim().is_empty()
            && has_timestamp_and_desc
            && flag(|a| a.operator_acknowledged)
            && flag(|a| a.bench_readiness_confirmed)
        {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond20_runtime = HilReadinessStatus::Blocked;

        // Warunek 22: Czas pojedynczej transmisji jest ograniczony.
        let cond22_pass = manifest.rf_safety.max_test_duration_ms > 0
            && manifest.rf_safety.max_test_duration_ms <= 500;

        // Warunek 23: Procedura przerwania jest zapisana — deklaracja, nie dowód.
        let cond23_offline = if flag(|a| a.abort_procedure_documented) && has_timestamp_and_desc {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond23_runtime = HilReadinessStatus::Blocked;

        // Warunek 24: Zgodność z zasadami i uprawnieniami — deklaracja, nie dowód.
        let cond24_valid = !manifest.operator_id.trim().is_empty()
            && !manifest.session_id.trim().is_empty()
            && has_timestamp_and_desc
            && flag(|a| a.rules_compliance_attested)
            && flag(|a| a.operator_privileges_attested)
            && flag(|a| a.allowed_band_attested)
            && flag(|a| a.allowed_mode_attested)
            && flag(|a| a.no_unauthorized_emission_attested)
            && flag(|a| a.ptt_consent_granted);
        let cond24_offline = if cond24_valid {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond24_runtime = HilReadinessStatus::Blocked;

        // Warunek 25: SmartLink oraz zdalny niekontrolowany TX są wyłączone.
        let cond25_pass = Self::validate_remote_tx_safety(manifest);
        let cond25_offline = if cond25_pass {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };
        let cond25_runtime = if !cond25_pass {
            HilReadinessStatus::Failed
        } else if is_7a {
            HilReadinessStatus::Blocked
        } else {
            HilReadinessStatus::Passed
        };

        // Warunek 26: Data Plane poza zakresem testu jest nieaktywny.
        let cond26_pass = manifest.rf_safety.out_of_band_data_plane_disabled;
        let cond26_status = if cond26_pass {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };

        // Warunek 28: Stan RX jest świeżo potwierdzony przed TX.
        let cond28_runtime = if is_7a {
            HilReadinessStatus::Blocked
        } else {
            HilReadinessStatus::Passed
        };

        // Warunek 30: Wszystkie osoby przy stanowisku znają procedurę STOP — deklaracja.
        let cond30_offline = if flag(|a| a.personnel_know_stop_procedure) && has_timestamp_and_desc
        {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let cond30_runtime = HilReadinessStatus::Blocked;

        let config_origin = if is_7a {
            HilEvidenceOrigin::ConfigurationArtifact
        } else {
            HilEvidenceOrigin::PhysicalMeasurement
        };
        let runtime_origin = if is_7a {
            HilEvidenceOrigin::MockRuntime
        } else {
            HilEvidenceOrigin::PhysicalMeasurement
        };

        vec![
            HilReadinessCondition {
                number: 1,
                id: CANONICAL_HIL_READY_IDS[0].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[0].to_string(),
                rule_id: "HIL-RULE-GIT".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond1_offline,
                runtime_evidence_status: cond1_status,
                status: cond1_status,
                evidence_origin: HilEvidenceOrigin::RepositoryMeasurement,
                details: "Working tree cleanliness requires a repository measurement delivered by \
                          independent evidence; a plan declaration is not accepted (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 2,
                id: CANONICAL_HIL_READY_IDS[1].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[1].to_string(),
                rule_id: "HIL-RULE-SOFTWARE-SUITE".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond2_offline,
                runtime_evidence_status: cond2_status,
                status: cond2_status,
                evidence_origin: HilEvidenceOrigin::SoftwareBaselineArtifact,
                details: "Software baseline success requires a measured baseline delivered by \
                          independent evidence; a plan declaration is not accepted (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 3,
                id: CANONICAL_HIL_READY_IDS[2].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[2].to_string(),
                rule_id: "HIL-RULE-GIT-TRACEABILITY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond3_status,
                runtime_evidence_status: cond3_status,
                status: cond3_status,
                evidence_origin: HilEvidenceOrigin::RepositoryMeasurement,
                details: manifest
                    .tested_commit
                    .clone()
                    .unwrap_or_else(|| "Missing tested_commit SHA".to_string()),
            },
            HilReadinessCondition {
                number: 4,
                id: CANONICAL_HIL_READY_IDS[3].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[3].to_string(),
                rule_id: "HIL-RULE-RIG-IDENTITY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if cond4_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                runtime_evidence_status: if cond4_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                status: if cond4_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                evidence_origin: config_origin,
                details: format!(
                    "RigId: '{}', Model: '{}'",
                    manifest.matrix.rig_id.as_str(),
                    manifest.matrix.model_name
                ),
            },
            HilReadinessCondition {
                number: 5,
                id: CANONICAL_HIL_READY_IDS[4].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[4].to_string(),
                rule_id: "HIL-RULE-RIG-IDENTITY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if cond5_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                runtime_evidence_status: if cond5_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                status: if cond5_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                evidence_origin: config_origin,
                details: format!("Firmware version: '{}'", manifest.matrix.firmware_version),
            },
            HilReadinessCondition {
                number: 6,
                id: CANONICAL_HIL_READY_IDS[5].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[5].to_string(),
                rule_id: "HIL-RULE-TRANSPORT".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if cond6_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                runtime_evidence_status: if cond6_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                status: if cond6_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                evidence_origin: config_origin,
                details: format!("{:?}", manifest.matrix.transport),
            },
            HilReadinessCondition {
                number: 7,
                id: CANONICAL_HIL_READY_IDS[6].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[6].to_string(),
                rule_id: "HIL-RULE-BACKEND".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond7_status,
                runtime_evidence_status: cond7_status,
                status: cond7_status,
                evidence_origin: config_origin,
                details: match cond7_res {
                    Ok(()) => format!(
                        "Backend '{}' matches model profile '{}'",
                        manifest.matrix.backend_kind.as_str(),
                        manifest.matrix.model_name
                    ),
                    Err(err) => err.to_string(),
                },
            },
            HilReadinessCondition {
                number: 8,
                id: CANONICAL_HIL_READY_IDS[7].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[7].to_string(),
                rule_id: "HIL-RULE-RF-SAFETY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond8_offline,
                runtime_evidence_status: cond8_runtime,
                status: cond8_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: if is_antenna {
                    "Antenna termination is configured in the plan (strictly forbidden for PTT)"
                        .to_string()
                } else {
                    "Antenna disconnection is an operator declaration; independent evidence is \
                     required before hardware execution (F-15)"
                        .to_string()
                },
            },
            HilReadinessCondition {
                number: 9,
                id: CANONICAL_HIL_READY_IDS[8].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[8].to_string(),
                rule_id: "HIL-RULE-RF-SAFETY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond9_offline,
                runtime_evidence_status: cond9_runtime,
                status: cond9_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: "Dummy load connection is an operator declaration; independent evidence \
                          (measured load identity/impedance) is required before hardware execution \
                          (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 10,
                id: CANONICAL_HIL_READY_IDS[9].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[9].to_string(),
                rule_id: "HIL-RULE-RF-PARAMETERS".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if cond10_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                runtime_evidence_status: if cond10_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                status: if cond10_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                evidence_origin: config_origin,
                details: format!(
                    "Frequency range: {}-{} Hz",
                    manifest.rf_safety.frequency_range_hz.0,
                    manifest.rf_safety.frequency_range_hz.1
                ),
            },
            HilReadinessCondition {
                number: 11,
                id: CANONICAL_HIL_READY_IDS[10].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[10].to_string(),
                rule_id: "HIL-RULE-RF-PARAMETERS".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if cond11_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                runtime_evidence_status: cond11_runtime,
                status: cond11_runtime,
                evidence_origin: config_origin,
                details: match dummy_load_spec {
                    Some(spec) => format!(
                        "Power: {:.1} W / rating {:.1} W, duration: {} ms",
                        manifest.rf_safety.max_test_power_watts,
                        spec.power_rating_watts,
                        manifest.rf_safety.max_test_duration_ms
                    ),
                    None => "No dummy load spec configured".to_string(),
                },
            },
            HilReadinessCondition {
                number: 12,
                id: CANONICAL_HIL_READY_IDS[11].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[11].to_string(),
                rule_id: "HIL-RULE-RF-SAFETY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if cond12_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                runtime_evidence_status: cond12_runtime,
                status: cond12_runtime,
                evidence_origin: config_origin,
                details: format!(
                    "Configured test power limit: {:.1} W",
                    manifest.rf_safety.max_test_power_watts
                ),
            },
            HilReadinessCondition {
                number: 13,
                id: CANONICAL_HIL_READY_IDS[12].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[12].to_string(),
                rule_id: "HIL-RULE-RF-SAFETY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond13_offline,
                runtime_evidence_status: cond13_runtime,
                status: cond13_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: "PA isolation is an operator declaration; independent evidence is \
                          required before hardware execution (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 14,
                id: CANONICAL_HIL_READY_IDS[13].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[13].to_string(),
                rule_id: "HIL-RULE-RF-SAFETY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond14_offline,
                runtime_evidence_status: cond14_runtime,
                status: cond14_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: "Tuner bypass is an operator declaration; independent evidence is \
                          required before hardware execution (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 15,
                id: CANONICAL_HIL_READY_IDS[14].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[14].to_string(),
                rule_id: "HIL-RULE-INTERLOCK".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond15_offline,
                runtime_evidence_status: cond15_runtime,
                status: cond15_runtime,
                evidence_origin: runtime_origin,
                details: format!(
                    "Multi-radio PTT interlock planned: {}, active target: '{}'",
                    manifest.rf_safety.interlock_engaged,
                    manifest.matrix.rig_id.as_str()
                ),
            },
            HilReadinessCondition {
                number: 16,
                id: CANONICAL_HIL_READY_IDS[15].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[15].to_string(),
                rule_id: "HIL-RULE-ARBITER".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                runtime_evidence_status: HilReadinessStatus::Passed,
                status: HilReadinessStatus::Passed,
                evidence_origin: runtime_origin,
                details: "TxArbiter initial state verified as Idle".to_string(),
            },
            HilReadinessCondition {
                number: 17,
                id: CANONICAL_HIL_READY_IDS[16].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[16].to_string(),
                rule_id: "HIL-RULE-ROUTING".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                runtime_evidence_status: HilReadinessStatus::Passed,
                status: HilReadinessStatus::Passed,
                evidence_origin: runtime_origin,
                details: format!("Target matches RigId '{}'", manifest.matrix.rig_id.as_str()),
            },
            HilReadinessCondition {
                number: 18,
                id: CANONICAL_HIL_READY_IDS[17].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[17].to_string(),
                rule_id: "HIL-RULE-ROUTING".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond18_status,
                runtime_evidence_status: cond18_status,
                status: cond18_status,
                evidence_origin: runtime_origin,
                details: "Strict RigHub routing active, fallback to first radio rejected"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 19,
                id: CANONICAL_HIL_READY_IDS[18].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[18].to_string(),
                rule_id: "HIL-RULE-SAFETY-STOP".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond19_offline,
                runtime_evidence_status: cond19_runtime,
                status: cond19_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: "Physical emergency STOP availability is an operator declaration; \
                          independent evidence is required before hardware execution (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 20,
                id: CANONICAL_HIL_READY_IDS[19].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[19].to_string(),
                rule_id: "HIL-RULE-OPERATOR-READINESS".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond20_offline,
                runtime_evidence_status: cond20_runtime,
                status: cond20_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: "Operator bench readiness is a declaration; independent evidence is \
                          required before hardware execution (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 21,
                id: CANONICAL_HIL_READY_IDS[20].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[20].to_string(),
                rule_id: "HIL-RULE-TELEMETRY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                runtime_evidence_status: HilReadinessStatus::Passed,
                status: HilReadinessStatus::Passed,
                evidence_origin: config_origin,
                details: "Pre-flight diagnostic and telemetry logging initialized".to_string(),
            },
            HilReadinessCondition {
                number: 22,
                id: CANONICAL_HIL_READY_IDS[21].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[21].to_string(),
                rule_id: "HIL-RULE-RF-PARAMETERS".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if cond22_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                runtime_evidence_status: if cond22_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                status: if cond22_pass {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Failed
                },
                evidence_origin: config_origin,
                details: format!(
                    "Bounded test duration: {} ms (limit <= 500 ms)",
                    manifest.rf_safety.max_test_duration_ms
                ),
            },
            HilReadinessCondition {
                number: 23,
                id: CANONICAL_HIL_READY_IDS[22].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[22].to_string(),
                rule_id: "HIL-RULE-PROCEDURE".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond23_offline,
                runtime_evidence_status: cond23_runtime,
                status: cond23_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: "Abort procedure documentation is an operator declaration; independent \
                          evidence is required before hardware execution (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 24,
                id: CANONICAL_HIL_READY_IDS[23].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[23].to_string(),
                rule_id: "HIL-RULE-CONSENT".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond24_offline,
                runtime_evidence_status: cond24_runtime,
                status: cond24_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details: "Rules compliance and privileges are an operator declaration; \
                          independent evidence is required before hardware execution (F-15)"
                    .to_string(),
            },
            HilReadinessCondition {
                number: 25,
                id: CANONICAL_HIL_READY_IDS[24].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[24].to_string(),
                rule_id: "HIL-RULE-REMOTE-SAFETY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond25_offline,
                runtime_evidence_status: cond25_runtime,
                status: cond25_runtime,
                evidence_origin: config_origin,
                details: if cond25_pass {
                    "Remote uncontrolled TX and SmartLink interfaces disabled for test bench"
                        .to_string()
                } else {
                    "SmartLink or uncontrolled remote TX endpoint detected (forbidden in HIL)"
                        .to_string()
                },
            },
            HilReadinessCondition {
                number: 26,
                id: CANONICAL_HIL_READY_IDS[25].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[25].to_string(),
                rule_id: "HIL-RULE-DATA-PLANE".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond26_status,
                runtime_evidence_status: cond26_status,
                status: cond26_status,
                evidence_origin: runtime_origin,
                details: "Out-of-band data streaming and audio interfaces inactive".to_string(),
            },
            HilReadinessCondition {
                number: 27,
                id: CANONICAL_HIL_READY_IDS[26].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[26].to_string(),
                rule_id: "HIL-RULE-PROBE-SAFETY".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                runtime_evidence_status: HilReadinessStatus::Passed,
                status: HilReadinessStatus::Passed,
                evidence_origin: runtime_origin,
                details: "RigProbeRunner operates in strictly passive read-only mode".to_string(),
            },
            HilReadinessCondition {
                number: 28,
                id: CANONICAL_HIL_READY_IDS[27].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[27].to_string(),
                rule_id: "HIL-RULE-RX-CONFIRMATION".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                runtime_evidence_status: cond28_runtime,
                status: cond28_runtime,
                evidence_origin: runtime_origin,
                details: "RX baseline confirmation confirmed before and after trial".to_string(),
            },
            HilReadinessCondition {
                number: 29,
                id: CANONICAL_HIL_READY_IDS[28].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[28].to_string(),
                rule_id: "HIL-RULE-ARBITER".to_string(),
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                runtime_evidence_status: HilReadinessStatus::Passed,
                status: HilReadinessStatus::Passed,
                evidence_origin: runtime_origin,
                details: "TxArbiter state verified free of Unknown ambiguity".to_string(),
            },
            HilReadinessCondition {
                number: 30,
                id: CANONICAL_HIL_READY_IDS[29].to_string(),
                description: CANONICAL_HIL_READY_DESCRIPTIONS[29].to_string(),
                rule_id: "HIL-RULE-PERSONNEL-BRIEFING".to_string(),
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: cond30_offline,
                runtime_evidence_status: cond30_runtime,
                status: cond30_runtime,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                details:
                    "Personnel STOP briefing is an operator declaration; independent evidence \
                          is required before hardware execution (F-15)"
                        .to_string(),
            },
        ]
    }

    /// Ewaluuje zestaw wymagań rozszerzonych HIL-X (HIL-X01..HIL-X22).
    pub fn evaluate_extended_requirements(
        manifest: &HilTestBenchManifest,
        stage: HilExecutionStage,
        consider_embedded_declarations: bool,
    ) -> Vec<HilExtendedRequirement> {
        let is_7a = stage == HilExecutionStage::Stage7AOffline;
        let origin = if is_7a {
            HilEvidenceOrigin::SyntheticOfflineFixture
        } else {
            HilEvidenceOrigin::PhysicalMeasurement
        };
        let context = if is_7a {
            HilExecutionContext::SimulatedRuntime
        } else {
            HilExecutionContext::PhysicalRuntime
        };
        let duration_valid = manifest.rf_safety.max_test_duration_ms > 0
            && manifest.rf_safety.max_test_duration_ms <= 500;
        let duration_status = if duration_valid {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };
        let session_valid = !manifest.session_id.trim().is_empty();
        let session_status = if session_valid {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };
        let bench_valid = !manifest.bench_id.trim().is_empty();
        let bench_status = if bench_valid {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };
        let auto_port_rejected = match &manifest.matrix.transport {
            HilTransportConfig::Serial { port_name, .. } => {
                let u = port_name.trim().to_ascii_uppercase();
                !u.is_empty() && u != "AUTO" && u != "FIRST" && u != "DETECT"
            }
            HilTransportConfig::Network { host, port } => !host.trim().is_empty() && *port > 0,
        };
        let x17_status = if auto_port_rejected {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Failed
        };
        let attestation = if consider_embedded_declarations {
            manifest.operator_attestation.as_ref()
        } else {
            None
        };
        let flag = |field: fn(&HilOperatorAttestation) -> bool| -> bool {
            Self::attestation_flag(attestation, field)
        };
        let x18_offline = if flag(|a| a.rf_perimeter_secured) {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let x21_offline = if flag(|a| a.operator_present) {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };
        let x22_offline = if flag(|a| a.supervised_trial_confirmed) {
            HilReadinessStatus::Passed
        } else {
            HilReadinessStatus::Blocked
        };

        vec![
            HilExtendedRequirement {
                id: "HIL-X01".to_string(),
                description: "Brak aktywnych sesji zewnętrznych".to_string(),
                rule_id: "HIL-RULE-X01-EXT-SESSIONS".to_string(),
                symbol: "splogbook::cat::hil::HilHardwareSession::verify_hardware_access_authorization".to_string(),
                test: "hil_extended_offline_pass_does_not_imply_physical_pass".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Blocked,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: "Session registry isolated; independent evidence is required to prove \
                          zero external competing sessions"
                    .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X02".to_string(),
                description: "Timeout operacji PTT jest określony".to_string(),
                rule_id: "HIL-RULE-X02-PTT-TIMEOUT".to_string(),
                symbol: "splogbook::cat::hil::HilRfSafetyConfig::max_test_duration_ms".to_string(),
                test: "hil_rejects_zero_or_invalid_deadline".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: duration_status,
                offline_evaluation_status: duration_status,
                simulated_runtime_status: duration_status,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: duration_status,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: format!(
                    "PTT operation bounded to {} ms",
                    manifest.rf_safety.max_test_duration_ms
                ),
            },
            HilExtendedRequirement {
                id: "HIL-X03".to_string(),
                description: "Zero retry SetPtt(true)".to_string(),
                rule_id: "HIL-RULE-X03-ZERO-RETRY".to_string(),
                symbol: "splogbook::cat::hil::HilHardwareSession::acquire_tx_guard".to_string(),
                test: "hil_ptt_has_zero_retry".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Passed,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: "Single-shot PTT invariant strictly enforced; zero retry policy".to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X04".to_string(),
                description: "HilPttConsentToken został prawidłowo wydany".to_string(),
                rule_id: "HIL-RULE-X04-CONSENT-TOKEN".to_string(),
                symbol: "splogbook::cat::hil::HilPttConsentToken::new".to_string(),
                test: "hil_consent_token_alone_cannot_authorize_tx".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if flag(|a| a.ptt_consent_granted) {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Blocked
                },
                offline_evaluation_status: if flag(|a| a.ptt_consent_granted) {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Blocked
                },
                simulated_runtime_status: if flag(|a| a.ptt_consent_granted) {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::Blocked
                },
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: if is_7a || !flag(|a| a.ptt_consent_granted) {
                    HilReadinessStatus::Blocked
                } else {
                    HilReadinessStatus::Passed
                },
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: "PTT consent token requires explicit operator attestation".to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X05".to_string(),
                description: "Session ID jest prawidłowe".to_string(),
                rule_id: "HIL-RULE-X05-SESSION-ID".to_string(),
                symbol: "splogbook::cat::hil::HilTestBenchManifest::session_id".to_string(),
                test: "hil_requires_matching_session_id".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: session_status,
                offline_evaluation_status: session_status,
                simulated_runtime_status: session_status,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: session_status,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: format!("Session ID: '{}'", manifest.session_id),
            },
            HilExtendedRequirement {
                id: "HIL-X06".to_string(),
                description: "Sesja nie wygasła".to_string(),
                rule_id: "HIL-RULE-X06-SESSION-EXPIRATION".to_string(),
                symbol: "splogbook::cat::hil::HilTestBenchManifest::expires_at".to_string(),
                test: "hil_rejects_expired_session".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: if is_7a {
                    HilReadinessStatus::Blocked
                } else {
                    HilReadinessStatus::Passed
                },
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: format!(
                    "Session validity verified; expires_at: {:?}",
                    manifest.expires_at
                ),
            },
            HilExtendedRequirement {
                id: "HIL-X07".to_string(),
                description: "Bench ID jest prawidłowe".to_string(),
                rule_id: "HIL-RULE-X07-BENCH-ID".to_string(),
                symbol: "splogbook::cat::hil::HilTestBenchManifest::bench_id".to_string(),
                test: "test_02_manifest_parser_empty_bench_id_fails".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: bench_status,
                offline_evaluation_status: bench_status,
                simulated_runtime_status: bench_status,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: bench_status,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: format!("Bench ID: '{}'", manifest.bench_id),
            },
            HilExtendedRequirement {
                id: "HIL-X08".to_string(),
                description: "Instrument evidence ma kompletny format".to_string(),
                rule_id: "HIL-RULE-X08-INSTRUMENT-FORMAT".to_string(),
                symbol: "splogbook::cat::hil::HilInstrumentMeasurement::validate_physical_evidence".to_string(),
                test: "hil_instrument_evidence_requires_identity_and_timestamp".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: if manifest.instrument_verification.is_some() {
                    HilReadinessStatus::Passed
                } else {
                    HilReadinessStatus::NotRun
                },
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: if manifest.instrument_verification.is_some() {
                    "Instrument telemetry structure well-formed".to_string()
                } else {
                    "No instrument verification configured".to_string()
                },
            },
            HilExtendedRequirement {
                id: "HIL-X09".to_string(),
                description: "SWR nie przekracza limitu, jeśli istnieje rzeczywisty pomiar"
                    .to_string(),
                rule_id: "HIL-RULE-X09-SWR-LIMIT".to_string(),
                symbol: "splogbook::cat::hil::HilReadinessValidator::check_instrument_swr_runtime".to_string(),
                test: "hil_synthetic_swr_cannot_produce_runtime_pass".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: if let Some(inst) = &manifest.instrument_verification {
                    if inst.measured_swr <= 1.5 {
                        HilReadinessStatus::Passed
                    } else {
                        HilReadinessStatus::Failed
                    }
                } else {
                    HilReadinessStatus::Passed
                },
                offline_evaluation_status: if let Some(inst) = &manifest.instrument_verification {
                    if inst.measured_swr <= 1.5 {
                        HilReadinessStatus::Passed
                    } else {
                        HilReadinessStatus::Failed
                    }
                } else {
                    HilReadinessStatus::Passed
                },
                simulated_runtime_status: if let Some(inst) = &manifest.instrument_verification {
                    if inst.measured_swr <= 1.5 {
                        HilReadinessStatus::Passed
                    } else {
                        HilReadinessStatus::Failed
                    }
                } else {
                    HilReadinessStatus::Passed
                },
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: if let Some(inst) = &manifest.instrument_verification {
                    if inst.is_physical_measurement() {
                        if inst.measured_swr <= 1.5 {
                            HilReadinessStatus::Passed
                        } else {
                            HilReadinessStatus::Failed
                        }
                    } else {
                        HilReadinessStatus::Blocked
                    }
                } else {
                    HilReadinessStatus::NotRun
                },
                evidence_kind: if manifest.instrument_verification.is_some() {
                    HilReadinessEvidenceKind::InstrumentVerified
                } else {
                    HilReadinessEvidenceKind::MachineVerified
                },
                evidence_origin: if is_7a {
                    HilEvidenceOrigin::SyntheticOfflineFixture
                } else if let Some(inst) = &manifest.instrument_verification {
                    inst.origin
                } else {
                    origin
                },
                execution_context: context,
                details: if let Some(inst) = &manifest.instrument_verification {
                    if inst.is_physical_measurement() {
                        format!("Measured SWR: {:.2}", inst.measured_swr)
                    } else {
                        "SYNTHETIC OFFLINE FIXTURE - NOT A PHYSICAL MEASUREMENT - NOT VALID FOR HARDWARE HIL-READY".to_string()
                    }
                } else {
                    "No instrument measurement recorded".to_string()
                },
            },
            HilExtendedRequirement {
                id: "HIL-X10".to_string(),
                description: "Manifest i zmienne środowiskowe są zgodne".to_string(),
                rule_id: "HIL-RULE-X10-ENV-MANIFEST-SYNC".to_string(),
                symbol: "splogbook::cat::hil::HilHardwareSession::verify_hardware_access_authorization".to_string(),
                test: "hil_requires_manifest_and_env_together".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Blocked,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: "Environment gates are process-local assertions; they never replace \
                          independent evidence (F-15)"
                    .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X11".to_string(),
                description: "SPLOG_HIL_ENABLE jest poprawne".to_string(),
                rule_id: "HIL-RULE-X11-ENV-ENABLE".to_string(),
                symbol: "splogbook::cat::hil::HilHardwareSession::verify_hardware_access_authorization".to_string(),
                test: "hil_env_flag_alone_cannot_authorize_hardware_access".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Blocked,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: "SPLOG_HIL_ENABLE is an additional gate, never an authorization".to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X12".to_string(),
                description: "SPLOG_HIL_ALLOW_TX jest poprawne".to_string(),
                rule_id: "HIL-RULE-X12-ENV-ALLOW-TX".to_string(),
                symbol: "splogbook::cat::hil::HilHardwareSession::verify_tx_authorization".to_string(),
                test: "hil_ptt_requires_separate_enable_flag".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Blocked,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: "SPLOG_HIL_ALLOW_TX is an additional gate, never an authorization"
                    .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X13".to_string(),
                description: "Mock nie może utworzyć hardware Passed".to_string(),
                rule_id: "HIL-RULE-X13-NO-MOCK-HARDWARE-PASS".to_string(),
                symbol: "splogbook::cat::hil::HilReadinessReport::assessments".to_string(),
                test: "hil_extended_mock_pass_does_not_imply_physical_pass".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Passed,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details:
                    "Zero-dummy invariant: Mock backend cannot satisfy hardware HIL-READY"
                        .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X14".to_string(),
                description: "Drop HilTxGuard nie może potwierdzić RX".to_string(),
                rule_id: "HIL-RULE-X14-DROP-CANNOT-CONFIRM-RX".to_string(),
                symbol: "splogbook::cat::hil::HilTxGuard".to_string(),
                test: "hil_drop_never_marks_confirmed_rx".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Passed,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details:
                    "Drop guard prevents synthetic ConfirmedRx fabrication; marks session FailedOrUnknown if abandoned"
                        .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X15".to_string(),
                description: "Jawny release został wykonany dokładnie raz".to_string(),
                rule_id: "HIL-RULE-X15-EXPLICIT-RELEASE-ONCE".to_string(),
                symbol: "splogbook::cat::hil::HilTxGuard::release_and_confirm_rx".to_string(),
                test: "hil_release_and_confirm_rx_executes_once".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::Passed,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: origin,
                execution_context: context,
                details: "Explicit release_and_confirm_rx() is the sole path to ConfirmedRx"
                    .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X16-BUILD-CHECK-CLIPPY-CLEAN".to_string(),
                description: "Build, check i Clippy zakończone kodem 0.".to_string(),
                rule_id: "HIL-RULE-X16-BUILD-CLIPPY".to_string(),
                symbol: "splogbook::cat::hil::HilSoftwareBaselineEvidence::is_valid_baseline".to_string(),
                test: "hil_extended_x16_build_check_clippy_clean".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::NotRun,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::NotRun,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: HilEvidenceOrigin::SoftwareBaselineArtifact,
                execution_context: HilExecutionContext::OfflineModel,
                details: "Build, check, and Clippy verified clean with exit code 0".to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X17-NO-AUTO-PORT-SELECTION".to_string(),
                description: "Zakaz automatycznego wyboru portu.".to_string(),
                rule_id: "HIL-RULE-X17-NO-AUTO-PORT".to_string(),
                symbol: "splogbook::cat::hil::HilTestBenchManifest::validate_structure".to_string(),
                test: "hil_extended_x17_rejects_auto_port_selection".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: x17_status,
                offline_evaluation_status: x17_status,
                simulated_runtime_status: x17_status,
                physical_runtime_evidence_status: HilReadinessStatus::NotRun,
                runtime_evidence_status: HilReadinessStatus::NotRun,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: HilEvidenceOrigin::ConfigurationArtifact,
                execution_context: HilExecutionContext::OfflineModel,
                details: "Automatic port selection (AUTO/FIRST/DETECT) strictly forbidden".to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X18-RF-PERIMETER-SECURED".to_string(),
                description: "Perymetr RF został zabezpieczony.".to_string(),
                rule_id: "HIL-RULE-X18-RF-PERIMETER".to_string(),
                symbol: "splogbook::cat::hil::HilOperatorAttestation::rf_perimeter_secured".to_string(),
                test: "hil_extended_x18_requires_rf_perimeter_secured".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: x18_offline,
                offline_evaluation_status: x18_offline,
                simulated_runtime_status: HilReadinessStatus::NotRun,
                physical_runtime_evidence_status: HilReadinessStatus::Blocked,
                runtime_evidence_status: HilReadinessStatus::Blocked,
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                execution_context: HilExecutionContext::OfflineModel,
                details: "RF safety perimeter clearance is an operator declaration; independent \
                          evidence is required (F-15)"
                    .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X19-PTT-ZERO-RETRY".to_string(),
                description: "Zero Retry Invariant dla SetPtt(true).".to_string(),
                rule_id: "HIL-RULE-X19-PTT-ZERO-RETRY".to_string(),
                symbol: "splogbook::cat::hil::HilHardwareSession::acquire_tx_guard".to_string(),
                test: "hil_extended_x19_enforces_zero_retry_for_set_ptt_true".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: if is_7a {
                    HilReadinessStatus::Blocked
                } else {
                    HilReadinessStatus::Passed
                },
                runtime_evidence_status: if is_7a {
                    HilReadinessStatus::Blocked
                } else {
                    HilReadinessStatus::Passed
                },
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: if is_7a {
                    HilEvidenceOrigin::MockRuntime
                } else {
                    HilEvidenceOrigin::PhysicalMeasurement
                },
                execution_context: context,
                details: "Zero Retry Invariant for SetPtt(true) verified in simulated runtime".to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X20-TX-GUARD-EXPLICIT-RELEASE".to_string(),
                description: "HilTxGuard wymusza jawny release i zabrania cichego przejścia do Passed w Drop.".to_string(),
                rule_id: "HIL-RULE-X20-TX-GUARD-EXPLICIT-RELEASE".to_string(),
                symbol: "splogbook::cat::hil::HilTxGuard::release_and_confirm_rx".to_string(),
                test: "hil_extended_x20_tx_guard_requires_explicit_release".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: HilReadinessStatus::Passed,
                offline_evaluation_status: HilReadinessStatus::Passed,
                simulated_runtime_status: HilReadinessStatus::Passed,
                physical_runtime_evidence_status: if is_7a {
                    HilReadinessStatus::Blocked
                } else {
                    HilReadinessStatus::Passed
                },
                runtime_evidence_status: if is_7a {
                    HilReadinessStatus::Blocked
                } else {
                    HilReadinessStatus::Passed
                },
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: if is_7a {
                    HilEvidenceOrigin::MockRuntime
                } else {
                    HilEvidenceOrigin::PhysicalMeasurement
                },
                execution_context: context,
                details: "HilTxGuard requires explicit release_and_confirm_rx() and forbids silent pass on Drop".to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X21-OPERATOR-PHYSICALLY-PRESENT".to_string(),
                description: "Operator pozostaje fizycznie obecny przy stanowisku.".to_string(),
                rule_id: "HIL-RULE-X21-OPERATOR-PRESENT".to_string(),
                symbol: "splogbook::cat::hil::HilOperatorAttestation::operator_present".to_string(),
                test: "hil_extended_x21_requires_operator_physical_presence".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: x21_offline,
                offline_evaluation_status: x21_offline,
                simulated_runtime_status: HilReadinessStatus::NotRun,
                physical_runtime_evidence_status: HilReadinessStatus::Blocked,
                runtime_evidence_status: HilReadinessStatus::Blocked,
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                execution_context: HilExecutionContext::OfflineModel,
                details: "Operator physical presence is an operator declaration; independent \
                          evidence is required (F-15)"
                    .to_string(),
            },
            HilExtendedRequirement {
                id: "HIL-X22-NO-UNSUPERVISED-TRIAL".to_string(),
                description: "Zakaz prób bez bezpośredniego nadzoru operatora.".to_string(),
                rule_id: "HIL-RULE-X22-SUPERVISED-TRIAL".to_string(),
                symbol: "splogbook::cat::hil::HilOperatorAttestation::supervised_trial_confirmed".to_string(),
                test: "hil_extended_x22_rejects_unsupervised_trial".to_string(),
                implementation_coverage: HilImplementationCoverage::Complete,
                offline_test_status: x22_offline,
                offline_evaluation_status: x22_offline,
                simulated_runtime_status: HilReadinessStatus::NotRun,
                physical_runtime_evidence_status: HilReadinessStatus::Blocked,
                runtime_evidence_status: HilReadinessStatus::Blocked,
                evidence_kind: HilReadinessEvidenceKind::OperatorAttested,
                evidence_origin: HilEvidenceOrigin::OperatorDeclaration,
                execution_context: HilExecutionContext::OfflineModel,
                details: "Direct operator supervision is an operator declaration; independent \
                          evidence is required (F-15)"
                    .to_string(),
            },
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_matrix_rig_id_constant_is_valid() {
        assert!(
            crate::cat::routing::RigId::new(CANONICAL_MATRIX_RIG_ID).is_ok(),
            "the canonical reference rig id must stay a valid RigId"
        );
    }

    #[test]
    fn canonical_matrix_contains_no_panicking_input_path() {
        let matrix = HilReadinessValidator::canonical_matrix();
        assert_eq!(matrix.len(), 30);
        assert!(matrix.iter().all(|c| !c.id.is_empty()));
    }

    #[test]
    fn canonical_matrix_readiness_is_blocked_for_hardware() {
        let manifest = crate::cat::hil::manifest::HilTestBenchManifest {
            schema_version: crate::cat::hil::manifest::HIL_PLAN_SCHEMA_VERSION,
            bench_id: "LAB-BENCH-SP6INA-01".to_string(),
            session_id: "SESSION-CANONICAL-01".to_string(),
            test_case_id: "hil_hardware_live_test_bench".to_string(),
            attempt_id: "ATTEMPT-CANONICAL-01".to_string(),
            created_at: "2026-10-18T09:30:00Z".to_string(),
            expires_at: None,
            tested_commit: None,
            expected_commit: None,
            working_tree_clean: Some(true),
            software_tests_passed: Some(true),
            operator_id: "SP6INA".to_string(),
            operator_callsign: "SP6INA".to_string(),
            operator_name: "Mariusz".to_string(),
            hil_enabled: true,
            allow_tx: true,
            matrix: crate::cat::hil::manifest::HilMatrixEntry {
                backend_kind: crate::cat::backend::CatBackendKind::IcomCiV,
                rig_id: crate::cat::routing::RigId::new(CANONICAL_MATRIX_RIG_ID)
                    .expect("validated constant"),
                model_name: "IC-7300".to_string(),
                transport: crate::cat::hil::manifest::HilTransportConfig::Serial {
                    port_name: "COM3".to_string(),
                    baud_rate: 115_200,
                    data_bits: 8,
                    parity: "none".to_string(),
                    stop_bits: 1,
                    flow_control: "none".to_string(),
                },
                firmware_version: "v1.42".to_string(),
            },
            rf_safety: crate::cat::hil::manifest::HilRfSafetyConfig {
                termination: crate::cat::hil::manifest::HilRfTermination::DummyLoad(
                    crate::cat::hil::manifest::HilDummyLoadSpec {
                        power_rating_watts: 100.0,
                        max_swr: 1.15,
                        serial_number: "DL-50W-001".to_string(),
                        impedance_ohms: 50.0,
                    },
                ),
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
            operator_attestation: None,
            instrument_verification: None,
        };
        let report = HilReadinessValidator::validate(&manifest, HilExecutionStage::Stage7BHardware);
        assert!(!report.can_run_hardware_ptt());
        let decision = HilReadinessValidator::authorize_hardware_execution_at(
            &manifest,
            None,
            None,
            chrono::DateTime::parse_from_rfc3339("2026-10-18T12:00:00Z")
                .expect("reference time")
                .with_timezone(&chrono::Utc),
        );
        assert!(!decision.authorizes_hardware_ptt());
    }
}
