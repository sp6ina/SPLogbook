// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

pub mod backend;
pub mod flrig;
pub mod hamlib;
pub mod native;
pub use native::icom::codec as icom_ci_v;
pub use native::kenwood::codec as kenwood;
pub mod rig;
pub mod rig_models;
pub mod rotor;
pub mod server;
pub mod so2r;
pub mod supervisor;
pub mod tci;
mod tcp;
pub mod winkeyer;
