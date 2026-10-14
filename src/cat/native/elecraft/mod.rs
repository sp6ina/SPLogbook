// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny moduł obsługi radiotelefonów Elecraft (K3, K3S, KX2, KX3, K4, K4D).

pub mod backend;
pub mod codec;
pub mod k4_tcp;
pub mod models;

pub use backend::{AsyncReaderWriter, ElecraftBackend, ElecraftConnection};
pub use codec::{
    ElecraftCat, ElecraftCodec, ElecraftCodecError, ElecraftFrame, ElecraftIfInfo, ElecraftMessage,
    ElecraftPacket,
};
pub use k4_tcp::{ElecraftTcpConfig, open_k4_tcp_stream};
pub use models::*;
