// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz WoÄąĹźniak (SP6INA)
//! TEST-ONLY PROTOCOL DOUBLES (P0-F14 software gate).
//!
//! This module is compiled **only** into test binaries (`tests/`). It is not part of the
//! production library, it cannot be selected from the GUI, the CLI, the configuration or the
//! environment, and it never appears in a release artifact.
//!
//! A double implements the **real** production trait [`RigBackend`] and simulates only the
//! external device behaviour that the official manufacturer / project documentation describes.
//! It must never replace the production parser, frame builder, validation, `TxArbiter`, cleanup,
//! timeout, cancellation or backend logic: the tests drive the real `HilHardwareSession` against
//! this device simulator.
//!
//! Every [`DocumentedPttProtocol`] variant carries the documented PTT ON/OFF frame, the
//! documented acknowledgement shape and the official source (document + revision). The same
//! table is asserted against the real production codecs in `tests/protocol_conformance.rs`.

#![allow(dead_code)]

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use splogbook::cat::backend::{BoxFuture, CatBackendKind, RigBackend};
use splogbook::cat::hamlib::RigState;
use splogbook::cat::rig::{RigCapabilities, RigCommand, RigError};

/// Shape of the device acknowledgement that the official documentation promises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentedAck {
    /// The device answers with its own status frame for the same command (Icom `1C 00`).
    StatusFrame,
    /// The device answers with the same ASCII command (Yaesu/Elecraft `TX1;`/`TQ1;`).
    EchoFrame,
    /// The device answers with a documented error-code line (`RPRT x`, Flex `R<seq>|code|`).
    CodeLine,
    /// The documentation promises no response to the set command (read via status query).
    None,
}

/// One documented PTT protocol family: the exact documented bytes and their official source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentedPttProtocol {
    /// `CatBackendKind` that owns this protocol in production.
    pub backend: CatBackendKind,
    /// Documented frame that keys the transmitter (PTT ON).
    pub ptt_on: &'static str,
    /// Documented frame that returns the transmitter to receive (PTT OFF).
    pub ptt_off: &'static str,
    /// Documented status/read query used to confirm TX/RX, when the protocol has one.
    pub ptt_read: Option<&'static str>,
    /// Documented acknowledgement shape.
    pub ack: DocumentedAck,
    /// Official source: document name, revision/date and the command it documents.
    pub source: &'static str,
}

impl DocumentedPttProtocol {
    /// Icom CI-V `1C 00` Ă˘â‚¬â€ť documented in the IC-7610 CI-V Reference Guide (A7380-7EX-4, Sep. 2025)
    /// command table: `1C 00 *1 00 or 01 Transceiver's status (00=RX, 01=TX)`.
    pub const ICOM_CI_V: Self = Self {
        backend: CatBackendKind::IcomCiV,
        ptt_on: "FE FE 94 E0 1C 00 01 FD",
        ptt_off: "FE FE 94 E0 1C 00 00 FD",
        ptt_read: Some("FE FE 94 E0 1C 00 FD"),
        ack: DocumentedAck::StatusFrame,
        source: "Icom IC-7610 CI-V Reference Guide A7380-7EX-4 (Sep. 2025), 1C 00 (00=RX, 01=TX)",
    };

    /// Yaesu CAT `TX1;` / `TX0;` with `TX;` status read.
    pub const YAESU_CAT: Self = Self {
        backend: CatBackendKind::Yaesu,
        ptt_on: "TX1;",
        ptt_off: "TX0;",
        ptt_read: Some("TX;"),
        ack: DocumentedAck::EchoFrame,
        source: "Yaesu CAT Operation Reference, TX command (TX1;/TX0;) and TX; status read",
    };

    /// Kenwood `TX;` / `RX;` (Kenwood PC CONTROL COMMAND Reference, B5A-0316-20/01).
    pub const KENWOOD_CAT: Self = Self {
        backend: CatBackendKind::Kenwood,
        ptt_on: "TX;",
        ptt_off: "RX;",
        ptt_read: Some("IF;"),
        ack: DocumentedAck::None,
        source: "Kenwood TS-590S/SG/890S/990S PC CONTROL COMMAND Reference (B5A-0316-20/01, 2019-01-30)",
    };

    /// Elecraft `TX;` / `RX;` with `TQ;` status read (Rev. G5 / K4 Rev. D12).
    pub const ELECRAFT_CAT: Self = Self {
        backend: CatBackendKind::Elecraft,
        ptt_on: "TX;",
        ptt_off: "RX;",
        ptt_read: Some("TQ;"),
        ack: DocumentedAck::EchoFrame,
        source: "Elecraft K3S/K3/KX3/KX2 Programmer's Reference Rev. G5 (2019-02-20) TX/RX/TQ; K4 Rev. D12",
    };

    /// FlexRadio SmartSDR `xmit 1` / `xmit 0`, state confirmed by the `interlock` status.
    pub const FLEX_SMARTSDR: Self = Self {
        backend: CatBackendKind::FlexRadio,
        ptt_on: "xmit 1",
        ptt_off: "xmit 0",
        ptt_read: None,
        ack: DocumentedAck::CodeLine,
        source: "FlexRadio SmartSDR TCP/IP API, TCPIP xmit (rev 177f09f, 2024-07-01) + interlock status",
    };

    /// TCI `trx:0,true;` / `trx:0,false;` with an echoed `trx:0,<state>;` reply.
    pub const TCI: Self = Self {
        backend: CatBackendKind::Tci,
        ptt_on: "trx:0,true;",
        ptt_off: "trx:0,false;",
        ptt_read: Some("trx:0;"),
        ack: DocumentedAck::EchoFrame,
        source: "Expert Electronics TCI Protocol 2.1 (Sep. 2026), TRX command",
    };

    /// Hamlib `rigctld` `T 1` / `T 0`, every set command answered with `RPRT x`.
    pub const HAMLIB_RIGCTLD: Self = Self {
        backend: CatBackendKind::Hamlib,
        ptt_on: "T 1",
        ptt_off: "T 0",
        ptt_read: Some("t"),
        ack: DocumentedAck::CodeLine,
        source: "Hamlib 4.7.2 rigctld(1) T/t commands and RPRT return codes (rig.h)",
    };

    /// FLRig XML-RPC `rig.set_ptt` with an integer parameter, state read with `rig.get_ptt`.
    pub const FLRIG: Self = Self {
        backend: CatBackendKind::Flrig,
        ptt_on: "rig.set_ptt <int>1</int>",
        ptt_off: "rig.set_ptt <int>0</int>",
        ptt_read: Some("rig.get_ptt"),
        ack: DocumentedAck::CodeLine,
        source: "FLRig 2.0.00 xml_server.cxx: rig.set_ptt n:i, rig.get_ptt i:n (port 12345)",
    };

    /// The documented frame that carries the requested PTT state.
    pub const fn frame_for(&self, ptt_on: bool) -> &'static str {
        if ptt_on { self.ptt_on } else { self.ptt_off }
    }
}

/// Deterministic fault-injection knobs. No timers, no threads, no randomness.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoubleFaults {
    /// `SetPtt(true)` fails with a protocol error after the device was keyed.
    pub fail_on: bool,
    /// `SetPtt(false)` fails with a protocol error.
    pub fail_off: bool,
    /// `SetPtt(true)` never answers (documented command timeout).
    pub silent_on: bool,
    /// `SetPtt(false)` never answers (documented command timeout).
    pub silent_off: bool,
    /// The device rejects the ON command (documented error/NAK), nothing is keyed.
    pub reject_on: bool,
    /// `poll()` keeps reporting PTT ON even after the OFF command (unconfirmed RX).
    /// `poll()` always reports PTT ON (the radio already transmits before the test starts).
    pub sticky_ptt: bool,
    /// `poll()` reports PTT ON only after the ON command (unconfirmed RX after a release).
    pub sticky_after_on: bool,
    /// `poll()` fails (measurement error).
    pub poll_error: bool,
    /// `poll()` succeeds before PTT ON but fails afterwards (measurement error in the TX state).
    pub poll_error_after_on: bool,
    /// Transport loss immediately after a successful ON (radio keyed, host cannot talk).
    pub disconnect_after_on: bool,
    /// The OFF future is never ready (used only to prove a non-blocking emergency attempt).
    pub pending_off: bool,
    /// `SetPtt(false)` reports a malformed/partial device answer.
    pub malformed_off: bool,
}

/// Observable counters and the documented wire sequence seen by the device double.
#[derive(Debug, Default)]
pub struct DoubleCounters {
    ptt_on: AtomicUsize,
    ptt_off: AtomicUsize,
    polls: AtomicUsize,
    commands: AtomicUsize,
    last_ptt: AtomicBool,
    confirmed_on: AtomicBool,
    connected: AtomicBool,
    seen_on: AtomicBool,
    transport_lost: AtomicBool,
    sequence: Mutex<Vec<String>>,
}

impl DoubleCounters {
    fn record(&self, frame: &str) {
        if let Ok(mut seq) = self.sequence.lock() {
            seq.push(frame.to_string());
        }
    }

    /// Number of `SetPtt(true)` requests observed by the device.
    pub fn ptt_on_count(&self) -> usize {
        self.ptt_on.load(Ordering::SeqCst)
    }

    /// Number of `SetPtt(false)` requests observed by the device.
    pub fn ptt_off_count(&self) -> usize {
        self.ptt_off.load(Ordering::SeqCst)
    }

    /// Number of `poll()` requests.
    pub fn poll_count(&self) -> usize {
        self.polls.load(Ordering::SeqCst)
    }

    /// Last logical PTT state the device holds.
    pub fn last_ptt_state(&self) -> bool {
        self.last_ptt.load(Ordering::SeqCst)
    }

    /// Whether the device ever confirmed the ON transition.
    pub fn confirmed_on(&self) -> bool {
        self.confirmed_on.load(Ordering::SeqCst)
    }

    /// Whether the device ever received an ON request.
    pub fn seen_on(&self) -> bool {
        self.seen_on.load(Ordering::SeqCst)
    }

    /// Documented frames observed, in order.
    pub fn sequence(&self) -> Vec<String> {
        self.sequence.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// TEST-ONLY protocol double: a simulated transceiver that implements the real [`RigBackend`]
/// trait and answers strictly according to [`DocumentedPttProtocol`].
pub struct ProtocolDouble {
    protocol: DocumentedPttProtocol,
    faults: DoubleFaults,
    counters: Arc<DoubleCounters>,
}

impl ProtocolDouble {
    /// Creates a double for the given documented protocol.
    pub fn new(protocol: DocumentedPttProtocol) -> Self {
        Self {
            protocol,
            faults: DoubleFaults::default(),
            counters: Arc::new(DoubleCounters {
                connected: AtomicBool::new(false),
                ..Default::default()
            }),
        }
    }

    /// Creates a double with injected faults.
    pub fn with_faults(protocol: DocumentedPttProtocol, faults: DoubleFaults) -> Self {
        Self {
            protocol,
            faults,
            counters: Arc::new(DoubleCounters {
                connected: AtomicBool::new(false),
                ..Default::default()
            }),
        }
    }

    /// Shared handle to the counters (clone before moving the double into a session).
    pub fn counters(&self) -> Arc<DoubleCounters> {
        self.counters.clone()
    }

    /// The documented protocol this double simulates.
    pub fn protocol(&self) -> DocumentedPttProtocol {
        self.protocol
    }

    fn transport_alive(&self) -> bool {
        !self.counters.transport_lost.load(Ordering::SeqCst)
    }
}

impl RigBackend for ProtocolDouble {
    fn kind(&self) -> CatBackendKind {
        self.protocol.backend
    }

    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities {
            frequency: true,
            mode: true,
            ptt: true,
            ..Default::default()
        }
    }

    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        self.counters.connected.store(true, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }

    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        self.counters.connected.store(false, Ordering::SeqCst);
        Box::pin(async {})
    }

    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        self.counters.polls.fetch_add(1, Ordering::SeqCst);
        let ptt = self.counters.last_ptt_state();
        let sticky =
            self.faults.sticky_ptt || (self.faults.sticky_after_on && self.counters.seen_on());
        let poll_error = self.faults.poll_error;
        let poll_error_after_on = self.faults.poll_error_after_on && self.counters.seen_on();
        let disconnected = self.counters.transport_lost.load(Ordering::SeqCst);
        Box::pin(async move {
            if disconnected {
                return Err(RigError::NotConnected);
            }
            if poll_error || poll_error_after_on {
                return Err(RigError::Protocol {
                    detail: "simulated measurement error during poll()".to_string(),
                });
            }
            Ok(RigState {
                ptt: ptt || sticky,
                ..Default::default()
            })
        })
    }

    fn execute<'a>(&'a mut self, command: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        self.counters.commands.fetch_add(1, Ordering::SeqCst);
        match command {
            RigCommand::SetPtt(true) => {
                self.counters.ptt_on.fetch_add(1, Ordering::SeqCst);
                self.counters.record(self.protocol.frame_for(true));
                if self.counters.transport_lost.load(Ordering::SeqCst) {
                    return Box::pin(async { Err(RigError::NotConnected) });
                }
                if self.faults.reject_on {
                    self.counters.last_ptt.store(false, Ordering::SeqCst);
                    return Box::pin(async { Err(RigError::Rejected { code: -9 }) });
                }
                if self.faults.silent_on {
                    self.counters.last_ptt.store(false, Ordering::SeqCst);
                    return Box::pin(async {
                        Err(RigError::Timeout {
                            endpoint: "test-double".to_string(),
                            operation: "ptt_on",
                        })
                    });
                }
                self.counters.last_ptt.store(true, Ordering::SeqCst);
                self.counters.seen_on.store(true, Ordering::SeqCst);
                if self.faults.disconnect_after_on {
                    self.counters.transport_lost.store(true, Ordering::SeqCst);
                    return Box::pin(async { Ok(()) });
                }
                if self.faults.fail_on {
                    // The device keyed but the acknowledgement was lost.
                    return Box::pin(async {
                        Err(RigError::Protocol {
                            detail: "simulated ON acknowledgement failure".to_string(),
                        })
                    });
                }
                self.counters.confirmed_on.store(true, Ordering::SeqCst);
                Box::pin(async { Ok(()) })
            }
            RigCommand::SetPtt(false) => {
                self.counters.ptt_off.fetch_add(1, Ordering::SeqCst);
                self.counters.record(self.protocol.frame_for(false));
                if self.counters.transport_lost.load(Ordering::SeqCst) {
                    return Box::pin(async { Err(RigError::NotConnected) });
                }
                if self.faults.pending_off {
                    return Box::pin(std::future::pending::<Result<(), RigError>>());
                }
                if self.faults.silent_off {
                    return Box::pin(async {
                        Err(RigError::Timeout {
                            endpoint: "test-double".to_string(),
                            operation: "ptt_off",
                        })
                    });
                }
                if self.faults.fail_off {
                    return Box::pin(async {
                        Err(RigError::Protocol {
                            detail: "simulated OFF acknowledgement failure".to_string(),
                        })
                    });
                }
                if self.faults.malformed_off {
                    return Box::pin(async {
                        Err(RigError::Protocol {
                            detail: "simulated truncated OFF response".to_string(),
                        })
                    });
                }
                self.counters.last_ptt.store(false, Ordering::SeqCst);
                Box::pin(async { Ok(()) })
            }
            _ => Box::pin(async { Ok(()) }),
        }
    }

    fn probe(
        &mut self,
    ) -> BoxFuture<'_, Result<splogbook::cat::backend::RigProbeResult, RigError>> {
        let protocol = self.protocol;
        Box::pin(async move {
            Ok(splogbook::cat::backend::RigProbeResult {
                reachable: true,
                identity: Some(protocol.backend.label().to_string()),
                model: None,
                firmware: None,
                capabilities: RigCapabilities {
                    frequency: true,
                    mode: true,
                    ptt: true,
                    ..Default::default()
                },
                state: None,
                safe_diagnostic: format!("test double {}", protocol.backend.as_str()),
            })
        })
    }
}

/// A minimal future that is pending forever, used to model an unresponsive device future
/// without any timer, thread or wall-clock dependency.
pub struct NeverReady;

impl std::future::Future for NeverReady {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Pending
    }
}
