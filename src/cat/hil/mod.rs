// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł Hardware-In-The-Loop (HIL) dla podsystemu Native CAT.
//!
//! Zapewnia:
//! - Struktury planu stanowiska laboratoryjnego ([`HilTestBenchManifest`]) — wyłącznie plan,
//! - Jawny model niezależnego dowodu ([`HilIndependentEvidence`]) i decyzji autoryzacyjnej
//!   ([`HilAuthorizationDecision`], F-15),
//! - Silnie typowane dowody gotowości ([`HilReadinessEvidenceKind`]),
//! - Deterministyczny walidator konfiguracji HIL-READY ([`HilReadinessValidator`]),
//! - Bezpieczne sesje pomiarowe ([`HilHardwareSession`]),
//! - Izolację faz NATIVE-7A (offline) oraz NATIVE-7B (stanowisko fizyczne).
//!
//! **Zasada F-15:** manifest testu nie jest dowodem gotowości i nie może sam siebie
//! autoryzować. Gotowość sprzętowa wymaga niezależnego dokumentu dowodowego wydanego przez
//! osobny podmiot; w kodzie produkcyjnym nie istnieje żaden dostawca dowodu, więc wynik jest
//! zawsze fail-closed (`EvidenceMissing` → brak PTT).

pub mod authorization;
pub mod error;
pub mod evidence;
pub mod manifest;
pub mod session;
pub mod validator;

pub use authorization::{
    HIL_ATTESTATION_SCHEMA_VERSION, HIL_AUTHORIZATION_SCHEMA_VERSION, HIL_PLAN_DIGEST_DOMAIN,
    HIL_REQUIRED_MEASURED_FACTS, HIL_SUPPORTED_EVIDENCE_ISSUERS, HilAttemptRegistry,
    HilAttestationDocument, HilAuthorizationAudit, HilAuthorizationDecision,
    HilAuthorizationInputs, HilAuthorizationOutcome, HilEvidenceError, HilEvidenceProvider,
    HilEvidenceProviderOutcome, HilEvidenceScope, HilHardwareExecutionAuthorizer,
    HilIndependentEvidence, HilOperatorAttestationStatements, digest_prefix, is_sha256_hex,
    is_supported_issuer, request_evidence_or_deny, sha256_hex,
};
pub use error::HilError;
pub use evidence::{
    CANONICAL_HIL_READY_DESCRIPTIONS, CANONICAL_HIL_READY_IDS, HilAbortProcedure, HilCommandResult,
    HilEvidenceOrigin, HilExecutionContext, HilExecutionStage, HilExtendedRequirement,
    HilImplementationCoverage, HilReadinessCheck, HilReadinessCondition, HilReadinessEvidenceKind,
    HilReadinessReport, HilReadinessStatus, HilRequirementAssessment, HilSoftwareBaselineEvidence,
};
pub use manifest::{
    HIL_MAX_TEST_DURATION_MS, HIL_MIN_TEST_DURATION_MS, HIL_PLAN_SCHEMA_VERSION, HilDummyLoadSpec,
    HilInstrumentMeasurement, HilMatrixEntry, HilOperatorAttestation, HilPlanValidationError,
    HilRfSafetyConfig, HilRfTermination, HilTestBenchManifest, HilTransportConfig,
};
pub use session::{
    HilDispatcherCase, HilDispatcherReport, HilHardwareSession, HilProbeResult, HilPttConsentToken,
    HilPttExecutionResult, HilReleaseOutcome, HilSessionState, HilTxGuard,
    default_hil_evaluation_time, run_isolated_hardware_dispatcher,
    run_isolated_hardware_dispatcher_at, run_isolated_hardware_test,
};
pub use validator::{HilProfileValidationError, HilReadinessValidator};
