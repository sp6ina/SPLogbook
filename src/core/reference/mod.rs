// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Optional reference data supplied by the user.
//!
//! Nothing in this module ships data. Every source is either a file the user
//! selected or a file the user explicitly downloaded; a missing source yields
//! an empty, valid state and never a panic.

pub mod cty;
pub mod install;
