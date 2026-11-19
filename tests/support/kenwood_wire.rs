// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! TEST-ONLY Kenwood PC Control *wire* double (CAT-018 / CAT-019).
//!
//! This module lives **only** in `tests/support/` and is included explicitly with
//! `#[path = "support/kenwood_wire.rs"]`. It is never part of the production library, it is not
//! reachable from the GUI, the CLI, the configuration or the environment, and it never appears
//! in a release artifact.
//!
//! The double simulates only the *transceiver side of the wire*: it reads the exact bytes that
//! the real production [`splogbook::cat::native::kenwood::KenwoodBackend`] writes into a
//! test-owned in-memory `tokio::io::duplex` stream and answers with the frames documented in the
//! official Kenwood PC CONTROL COMMAND references:
//!
//! * `FA;` → `FA00014074000;` (Read/Answer, documented for every supported model)
//! * `ID;` → `ID<PPP>;` (documented model number: 021 / 023 / 022 / 024)
//! * `AI2;` → `AI2;` (documented Answer, pushed when AI is ON) — optional
//! * `TX;` → `TX0;` (documented Answer `TX P1;` with P1 = 0 = SEND) — optional/silent/error
//! * `RX;` → `RX;` (documented Answer) — optional/silent/error
//! * `IF;` → the documented `IF` status frame (P8 0 = RX, 1 = TX) — **only** for models whose
//!   reference document defines `IF` (TS-590S / TS-590SG); for other models the frame is recorded
//!   and answered with nothing, so a test can prove that an undocumented command was not sent
//! * `SM;` / `SM0;` → the documented S-meter answer for that model
//! * `?;` / `E;` / `O;` — the documented error messages (p. 3 of each reference)
//!
//! No serial port, no USB device, no COM port, no network and no transceiver is involved.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

/// Documented Kenwood error messages (`?;`, `E;`, `O;`), PC CONTROL COMMAND p. 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireErrorFrame {
    /// `?;` — command syntax incorrect, or the command was not executed in the current state.
    Syntax,
    /// `E;` — serial communication error (overrun or framing).
    Communication,
    /// `O;` — receive buffer overrun.
    Overrun,
}

impl WireErrorFrame {
    pub fn wire(self) -> &'static str {
        match self {
            Self::Syntax => "?;",
            Self::Communication => "E;",
            Self::Overrun => "O;",
        }
    }
}

/// How the simulated transceiver answers the documented `TX;` / `RX;` set commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttAnswer {
    /// The documented Answer frame (`TX P1;` / `RX;`), pushed because AI is ON.
    DocumentedAnswer,
    /// The documentation promises no Answer for a set command when AI is OFF.
    Silent,
    /// The documented error message (`?;`, `E;`, `O;`).
    Error(WireErrorFrame),
}

/// Behaviour of the simulated transceiver.
#[derive(Debug, Clone)]
pub struct KenwoodWireConfig {
    /// Documented `ID` number: `021` TS-590S, `023` TS-590SG, `022` TS-990S, `024` TS-890S.
    pub id_number: &'static str,
    /// `true` only for models whose official reference documents the `IF` command.
    pub documents_if: bool,
    /// Documented `SM` read form (`SM0;` for TS-590S/SG/990S, `SM;` for TS-890S).
    pub smeter_read_form: &'static str,
    /// `true` → `SM` answers with five digits after `SM`, `false` → four digits.
    pub smeter_five_digits: bool,
    /// Whether `AI2;` is answered (documented Answer is sent when AI is ON).
    pub answer_ai: bool,
    /// Whole seconds of the simulated answer for `TX;`.
    pub tx_answer: PttAnswer,
    /// Whole seconds of the simulated answer for `RX;`.
    pub rx_answer: PttAnswer,
    /// Push an unsolicited documented `TX0;` Auto-Information frame once, right after the AI answer
    /// (the operator keyed the transmitter on the front panel).
    pub push_unsolicited_tx0: bool,
    /// Push raw bytes once, right after the documented `AI` answer (stale/duplicate/partial frames).
    pub push_raw_after_ai: Option<&'static str>,
    /// Push raw bytes once, right after the documented `TX` answer (stale/duplicate frames).
    pub push_raw_after_tx: Option<&'static str>,
    /// Force the documented `IF` answer to report this state instead of the simulated one.
    pub if_status_override: Option<bool>,
    /// Never answer the documented `IF` status read (silent transceiver).
    pub silent_status_read: bool,
    /// Close the transport immediately after the `TX;` write (radio keyed, transport lost).
    pub close_after_tx: bool,
    /// S-meter dot count returned in the documented answer.
    pub smeter_dots: u8,
}

impl KenwoodWireConfig {
    /// Documented configuration of a TS-590S (B5A-0316-20/01).
    pub fn ts590s() -> Self {
        Self {
            id_number: "021",
            documents_if: true,
            smeter_read_form: "SM0;",
            smeter_five_digits: true,
            answer_ai: true,
            tx_answer: PttAnswer::DocumentedAnswer,
            rx_answer: PttAnswer::DocumentedAnswer,
            push_unsolicited_tx0: false,
            push_raw_after_ai: None,
            push_raw_after_tx: None,
            if_status_override: None,
            silent_status_read: false,
            close_after_tx: false,
            smeter_dots: 15,
        }
    }

    /// Documented configuration of a TS-590SG (B5A-0316-20/01).
    pub fn ts590sg() -> Self {
        Self {
            id_number: "023",
            ..Self::ts590s()
        }
    }

    /// Documented configuration of a TS-890S (B5A-2944-00/01) — **no `IF` command**.
    pub fn ts890s() -> Self {
        Self {
            id_number: "024",
            documents_if: false,
            smeter_read_form: "SM;",
            smeter_five_digits: false,
            ..Self::ts590s()
        }
    }

    /// Documented configuration of a TS-990S (B5A-3170-00) — **no `IF` command**.
    pub fn ts990s() -> Self {
        Self {
            id_number: "022",
            documents_if: false,
            smeter_read_form: "SM0;",
            smeter_five_digits: true,
            ..Self::ts590s()
        }
    }
}

/// Handle to a running wire double.
pub struct KenwoodWire {
    received: Arc<Mutex<Vec<String>>>,
    notify: Arc<tokio::sync::Notify>,
    handle: tokio::task::JoinHandle<()>,
}

impl KenwoodWire {
    /// Every frame the simulated transceiver received, in order, exactly as written on the wire.
    pub fn frames(&self) -> Vec<String> {
        self.received.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Number of received frames equal to `frame` (e.g. `"RX;"`).
    pub fn count(&self, frame: &str) -> usize {
        self.frames().iter().filter(|f| *f == frame).count()
    }

    /// `true` when the production client wrote the frame at least once.
    pub fn sent(&self, frame: &str) -> bool {
        self.count(frame) > 0
    }

    /// Waits (cooperatively, without `sleep`) until the frame is observed or the deadline passes.
    pub async fn wait_for(&self, frame: &str, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.notify.notified();
            if self.sent(frame) {
                return true;
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return false;
            }
            let _ = tokio::time::timeout(deadline - now, notified).await;
        }
    }

    /// Stops the simulated transceiver and closes the wire.
    pub async fn shutdown(self) {
        self.handle.abort();
        let _ = tokio::time::timeout(Duration::from_millis(500), self.handle).await;
    }
}

/// Builds the documented `IF` answer frame (`Reads the transceiver status`, TS-590S/SG p. 15).
///
/// `IF P1(11) P2(5 spaces) P3(5) P4 P5 P6 P7 P7 P8 P9 P10 P11 P12 P13 P14 P14 P15 ;`
pub fn documented_if_frame(tx: bool, frequency_hz: u64, mode: char) -> String {
    let mut frame = String::from("IF");
    frame.push_str(&format!("{frequency_hz:011}"));
    frame.push_str("     "); // P2: five spaces
    frame.push_str("+0000"); // P3: RIT/XIT frequency
    frame.push('0'); // P4: RIT off
    frame.push('0'); // P5: XIT off
    frame.push_str("000"); // P6,P7,P7: memory channel
    frame.push(if tx { '1' } else { '0' }); // P8: 0 = RX, 1 = TX
    frame.push(mode); // P9: operating mode
    frame.push('0'); // P10: FR/FT function
    frame.push('0'); // P11: scan
    frame.push('0'); // P12: simplex
    frame.push('0'); // P13: tone off
    frame.push_str("00"); // P14: tone frequency
    frame.push('0'); // P15
    frame.push(';');
    frame
}

/// Spawns the documented transceiver simulator on the transceiver side of `io`.
pub fn spawn_kenwood_wire(io: DuplexStream, cfg: KenwoodWireConfig) -> KenwoodWire {
    let received = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&received);
    let notify = Arc::new(tokio::sync::Notify::new());
    let notifier = Arc::clone(&notify);

    let handle = tokio::spawn(async move {
        let mut io = io;
        let mut buffer: Vec<u8> = Vec::new();
        let mut tmp = [0u8; 256];
        let mut tx = false;
        let mut announced = false;

        loop {
            let n = match io.read(&mut tmp).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            buffer.extend_from_slice(&tmp[..n]);

            while let Some(pos) = buffer.iter().position(|b| *b == b';') {
                let raw: Vec<u8> = buffer.drain(..=pos).collect();
                let frame = String::from_utf8_lossy(&raw).to_string();
                if let Ok(mut rec) = recorder.lock() {
                    rec.push(frame.clone());
                }
                notifier.notify_one();

                let command: String = frame.chars().take(2).collect();
                let mut reply: Option<String> = None;

                match command.as_str() {
                    "FA" => reply = Some("FA00014074000;".to_string()),
                    "FB" => reply = Some("FB00007030000;".to_string()),
                    "ID" => reply = Some(format!("ID{};", cfg.id_number)),
                    "AI" => {
                        if cfg.answer_ai {
                            reply = Some("AI2;".to_string());
                        }
                    }
                    "MD" => reply = Some("MD2;".to_string()),
                    "PC" => reply = Some("PC100;".to_string()),
                    "RT" => reply = Some("RT0;".to_string()),
                    "XT" => reply = Some("XT0;".to_string()),
                    "FT" => reply = Some("FT0;".to_string()),
                    "FR" => reply = Some("FR0;".to_string()),
                    "SM" => {
                        let dots = cfg.smeter_dots.min(70);
                        reply = Some(if cfg.smeter_five_digits {
                            format!("SM{dots:05};")
                        } else {
                            format!("SM{dots:04};")
                        });
                    }
                    "TX" => {
                        tx = true;
                        match cfg.tx_answer {
                            PttAnswer::DocumentedAnswer => reply = Some("TX0;".to_string()),
                            PttAnswer::Silent => {}
                            PttAnswer::Error(e) => reply = Some(e.wire().to_string()),
                        }
                        if cfg.close_after_tx {
                            // The transmitter was keyed and the transport disappears.
                            return;
                        }
                    }
                    "RX" => {
                        tx = false;
                        match cfg.rx_answer {
                            PttAnswer::DocumentedAnswer => reply = Some("RX;".to_string()),
                            PttAnswer::Silent => {}
                            PttAnswer::Error(e) => reply = Some(e.wire().to_string()),
                        }
                    }
                    "IF" => {
                        // Documented only for TS-590S / TS-590SG.
                        if cfg.documents_if && !cfg.silent_status_read {
                            let status = cfg.if_status_override.unwrap_or(tx);
                            reply = Some(documented_if_frame(status, 14_074_000, '2'));
                        }
                    }
                    _ => reply = Some("?;".to_string()),
                }

                if let Some(reply) = reply
                    && io.write_all(reply.as_bytes()).await.is_err()
                {
                    return;
                }

                // The documented Answer to `AI2;` finishes the connect handshake; afterwards the
                // transceiver may push unsolicited Auto-Information frames (the operator keyed the
                // transmitter on the front panel, or a stale/duplicate frame is still in flight).
                if command == "AI" && !announced {
                    announced = true;
                    if cfg.push_unsolicited_tx0 && io.write_all(b"TX0;").await.is_err() {
                        return;
                    }
                    if let Some(raw) = cfg.push_raw_after_ai
                        && io.write_all(raw.as_bytes()).await.is_err()
                    {
                        return;
                    }
                }
                if command == "TX"
                    && let Some(raw) = cfg.push_raw_after_tx
                    && io.write_all(raw.as_bytes()).await.is_err()
                {
                    return;
                }
            }
        }

        let _ = io.shutdown().await;
    });

    KenwoodWire {
        received,
        notify,
        handle,
    }
}

/// A TS-590S that pushes raw bytes (e.g. a frame without the documented `;` terminator) once,
/// right after the documented `AI` answer.
pub fn spawn_raw_kenwood_wire(io: DuplexStream, raw: &'static str) -> KenwoodWire {
    spawn_kenwood_wire(
        io,
        KenwoodWireConfig {
            push_raw_after_ai: Some(raw),
            ..KenwoodWireConfig::ts590s()
        },
    )
}

/// Connects the real production backend to a documented wire double.
///
/// Returns `(backend, wire)`. The caller must `disconnect()` the backend and `shutdown()` the wire.
pub async fn connect_backend(
    profile: splogbook::cat::native::profile::RigModelProfile,
    cfg: KenwoodWireConfig,
) -> (splogbook::cat::native::kenwood::KenwoodBackend, KenwoodWire) {
    let (client_io, radio_io) = tokio::io::duplex(2048);
    let wire = spawn_kenwood_wire(radio_io, cfg);
    let mut backend =
        splogbook::cat::native::kenwood::KenwoodBackend::with_stream(client_io, profile);
    use splogbook::cat::backend::RigBackend;
    backend.connect().await.expect("documented connect");
    (backend, wire)
}
