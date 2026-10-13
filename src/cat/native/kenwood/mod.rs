// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny moduł obsługi radiotelefonów Kenwood PC Control (CAT) oraz KNS LAN.

pub mod backend;
pub mod codec;
pub mod lan;
pub mod models;

pub use backend::KenwoodBackend;
pub use codec::{KenwoodCat, KenwoodCodec, KenwoodFrame, KenwoodIfInfo};
pub use lan::{KenwoodLanConfig, KenwoodLanTransport};
