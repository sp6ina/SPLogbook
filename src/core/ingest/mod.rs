// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł potoku zapisu QSO (`core::ingest`).

mod origin;

pub use origin::{IngestPolicy, PostEffects, QsoOrigin, ValidationMode};
