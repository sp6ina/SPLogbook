// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Natywny moduł obsługi radiotelefonów FlexRadio SmartSDR (TCP/IP API & UDP Discovery).

pub mod backend;
pub mod client;
pub mod discovery;
pub mod models;
pub mod protocol;
pub mod slice;

pub use backend::{
    DEFAULT_COMMAND_TIMEOUT, FlexRadioBackend, MAX_PENDING_COMMANDS, PendingCommandRegistry,
};
pub use client::{FlexRadioConfig, FlexRadioConnection, HandshakeState};
pub use discovery::{
    FLEX_DISCOVERY_PORT, FlexDiscoveryCodec, FlexDiscoveryPacket, FlexDiscoveryVitaHeader,
    MAX_DISCOVERY_DATAGRAM_SIZE, deduplicate_radios, discover_radios, resolve_connection_port,
};
pub use models::*;
pub use protocol::{
    FlexCommand, FlexInboundMessage, FlexMessage, FlexProtocolCodec, FlexProtocolError,
    FlexResponse, FlexSequenceAllocator, FlexStatus, MAX_LINE_LENGTH,
};
pub use slice::{
    FlexSliceId, FlexSliceState, select_active_slice, select_read_slice, select_tx_slice,
    select_write_slice,
};
