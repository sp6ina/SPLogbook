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
    HilExecutionStage, HilReadinessCheck, HilReadinessCondition, HilReadinessEvidenceKind,
    HilReadinessReport, HilReadinessStatus,
};
pub use manifest::{
    HilDummyLoadSpec, HilInstrumentMeasurement, HilMatrixEntry, HilOperatorAttestation,
    HilRfSafetyConfig, HilRfTermination, HilTestBenchManifest, HilTransportConfig,
};
pub use session::{
    HilHardwareSession, HilProbeResult, HilPttConsentToken, HilPttExecutionResult, HilTxGuard,
};
pub use validator::HilReadinessValidator;
