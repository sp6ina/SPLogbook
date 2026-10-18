// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł Hardware-In-The-Loop (HIL) dla podsystemu Native CAT.
//!
//! Zapewnia:
//! - Struktury manifestu stanowiska laboratoryjnego ([`HilTestBenchManifest`]),
//! - Silnie typowane dowody gotowości ([`HilReadinessEvidenceKind`]),
//! - Deterministyczny walidator HIL-READY ([`HilReadinessValidator`]),
//! - Bezpieczne sesje pomiarowe ([`HilHardwareSession`]),
//! - Izolację faz NATIVE-7A (offline) oraz NATIVE-7B (stanowisko fizyczne).

pub mod error;
pub mod evidence;
pub mod manifest;
pub mod session;
pub mod validator;

pub use error::HilError;
pub use evidence::{
    CANONICAL_HIL_READY_DESCRIPTIONS, CANONICAL_HIL_READY_IDS, HilAbortProcedure, HilCommandResult,
    HilEvidenceOrigin, HilExecutionContext, HilExecutionStage, HilExtendedRequirement,
    HilImplementationCoverage, HilReadinessCheck, HilReadinessCondition, HilReadinessEvidenceKind,
    HilReadinessReport, HilReadinessStatus, HilRequirementAssessment, HilSoftwareBaselineEvidence,
};
pub use manifest::{
    HilDummyLoadSpec, HilInstrumentMeasurement, HilMatrixEntry, HilOperatorAttestation,
    HilRfSafetyConfig, HilRfTermination, HilTestBenchManifest, HilTransportConfig,
};
pub use session::{
    HilDispatcherCase, HilDispatcherReport, HilHardwareSession, HilProbeResult, HilPttConsentToken,
    HilPttExecutionResult, HilSessionState, HilTxGuard, run_isolated_hardware_dispatcher,
    run_isolated_hardware_test,
};
pub use validator::HilReadinessValidator;
