// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

pub mod backend;
pub mod config;
pub mod factory;
pub mod flrig;
pub mod hamlib;
pub mod hub;
pub mod native;
pub use native::elecraft::codec as elecraft;
pub use native::icom::codec as icom_ci_v;
pub use native::kenwood::codec as kenwood;
pub use native::yaesu::codec as yaesu;
pub mod hil;
pub mod probe;
pub mod rig;
pub mod rig_models;
pub mod rotor;
pub mod routing;
pub mod runtime;
pub mod server;
pub mod service;
pub mod so2r;
pub mod supervisor;
pub mod tci;
mod tcp;
pub mod tx_arbiter;
pub mod winkeyer;

pub use config::{
    CURRENT_SCHEMA_VERSION, CatConfigMigrationError, CatConfigValidationError, CatStationConfig,
    ElecraftConfig, ElecraftTransportConfig, FlexRadioConfig, FlexRadioConnectionConfig,
    FlrigConfig, HamlibConfig, IcomConfig, KenwoodConfig, KenwoodTransportConfig, RigBackendConfig,
    RigConfig, RigTargetConfig, TciConfig, YaesuConfig, migrate_legacy_cat_config,
};
pub use factory::{FactoryError, RigBackendFactory};
pub use hil::{
    HilDummyLoadSpec, HilError, HilExecutionStage, HilHardwareSession, HilInstrumentMeasurement,
    HilMatrixEntry, HilOperatorAttestation, HilProbeResult, HilPttConsentToken,
    HilPttExecutionResult, HilReadinessCheck, HilReadinessEvidenceKind, HilReadinessReport,
    HilReadinessStatus, HilReadinessValidator, HilRfSafetyConfig, HilRfTermination,
    HilTestBenchManifest, HilTransportConfig,
};
pub use hub::{PendingCommandGuard, RigHub, RigHubError, RigHubSnapshot};
pub use probe::RigProbeRunner;
pub use routing::{
    ActiveRxTarget, ActiveTxTarget, ReceiverId, ReceiverIdError, RigCommandId, RigHubEvent, RigId,
    RigIdError, RigTarget, RoutedRigCommand, RoutedRigEvent, RoutingError,
};
pub use runtime::{CatRuntime, CatRuntimeError};
pub use service::{
    RigService, RigServiceError, RigServiceHandle, RigServiceLifecycle, RigServiceSnapshot,
    RigServiceState, RigServiceSupervisor, StateFreshness,
};
pub use tx_arbiter::{TxArbiter, TxArbiterError, TxArbiterState, TxUnknownReason};
