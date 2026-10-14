// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

pub mod backend;
pub mod flrig;
pub mod hamlib;
pub mod hub;
pub mod native;
pub use native::elecraft::codec as elecraft;
pub use native::icom::codec as icom_ci_v;
pub use native::kenwood::codec as kenwood;
pub use native::yaesu::codec as yaesu;
pub mod rig;
pub mod rig_models;
pub mod rotor;
pub mod routing;
pub mod server;
pub mod service;
pub mod so2r;
pub mod supervisor;
pub mod tci;
mod tcp;
pub mod tx_arbiter;
pub mod winkeyer;

pub use hub::{RigHub, RigHubError, RigHubSnapshot};
pub use routing::{
    ActiveRxTarget, ActiveTxTarget, ReceiverId, ReceiverIdError, RigCommandId, RigHubEvent, RigId,
    RigIdError, RigTarget, RoutedRigCommand, RoutedRigEvent, RoutingError,
};
pub use service::{
    RigService, RigServiceError, RigServiceHandle, RigServiceSnapshot, RigServiceState,
    StateFreshness,
};
pub use tx_arbiter::{TxArbiter, TxArbiterError, TxArbiterState, TxUnknownReason};
