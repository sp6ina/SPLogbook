// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Kodek i definicje protokołu tekstowego FlexRadio SmartSDR TCP/IP API.
//!
//! Zgodny ze specyfikacją FlexRadio Systems SmartSDR API:
//! - Komendy klienta: `C<seq>|<command>\n`
//! - Odpowiedzi radia: `R<seq>|<hex_status>[|<message>]\n`
//! - Asynchroniczne statusy: `S<client_handle>|<object> [id] <key>=<value>...\n`
//! - Komunikat wersji: `V<version_string>\n`
//! - Komunikat uchwytu klienta: `H<hex_handle>\n`
//! - Komunikaty diagnostyczne: `M<num>|<text>\n`

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::cat::backend::CatBackendKind;
use crate::cat::rig::RigError;

/// Maksymalna dopuszczalna długość pojedynczej linii protokołu (ochrona przed atakiem bufora).
pub const MAX_LINE_LENGTH: usize = 4096;

/// Typowane błędy protokołu FlexRadio SmartSDR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlexProtocolError {
    InvalidHeader(String),
    InvalidUtf8(String),
    OversizedDatagram {
        max: usize,
        actual: usize,
    },
    MissingRequiredField(String),
    LineTooLong {
        max: usize,
        actual: usize,
    },
    InvalidSequence(String),
    InvalidHandle(String),
    DuplicateInitMessage(String),
    HandshakeIncomplete(String),
    CommandRejected {
        seq: u32,
        code: u32,
        message: String,
    },
    UnexpectedResponse {
        seq: u32,
        expected: u32,
    },
    ParseError(String),
    Timeout {
        operation: &'static str,
    },
    QueueFull,
    NotConnected,
    Transport(String),
    AmbiguousSlice(String),
    SliceNotFound(u32),
    Unsupported(String),
}

impl std::fmt::Display for FlexProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeader(detail) => write!(f, "invalid SmartSDR packet header: {detail}"),
            Self::InvalidUtf8(detail) => write!(f, "invalid UTF-8 in SmartSDR stream: {detail}"),
            Self::OversizedDatagram { max, actual } => {
                write!(
                    f,
                    "discovery datagram too large ({actual} > max {max} bytes)"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(f, "missing required SmartSDR field: '{field}'")
            }
            Self::LineTooLong { max, actual } => {
                write!(f, "line length {actual} exceeds limit of {max} bytes")
            }
            Self::InvalidSequence(detail) => write!(f, "invalid sequence number: {detail}"),
            Self::InvalidHandle(detail) => write!(f, "invalid client handle format: {detail}"),
            Self::DuplicateInitMessage(msg) => write!(f, "duplicate init message received: {msg}"),
            Self::HandshakeIncomplete(detail) => {
                write!(f, "handshake incomplete before ready: {detail}")
            }
            Self::CommandRejected { seq, code, message } => {
                write!(
                    f,
                    "command C{seq} rejected with code 0x{code:08X}: {message}"
                )
            }
            Self::UnexpectedResponse { seq, expected } => {
                write!(f, "received response R{seq} but expected R{expected}")
            }
            Self::ParseError(detail) => write!(f, "SmartSDR parse error: {detail}"),
            Self::Timeout { operation } => write!(f, "SmartSDR operation '{operation}' timed out"),
            Self::QueueFull => write!(f, "SmartSDR pending command queue is full"),
            Self::NotConnected => write!(f, "SmartSDR backend is not connected"),
            Self::Transport(detail) => write!(f, "SmartSDR transport error: {detail}"),
            Self::AmbiguousSlice(detail) => write!(f, "ambiguous slice selection: {detail}"),
            Self::SliceNotFound(id) => write!(f, "slice {id} not found in active slice map"),
            Self::Unsupported(cmd) => write!(f, "unsupported SmartSDR command: {cmd}"),
        }
    }
}

impl std::error::Error for FlexProtocolError {}

impl FlexProtocolError {
    pub fn to_rig_error(&self, endpoint: &str) -> RigError {
        match self {
            Self::CommandRejected { code, .. } => RigError::Rejected { code: *code as i32 },
            Self::Timeout { operation } => RigError::Timeout {
                endpoint: endpoint.to_string(),
                operation,
            },
            Self::QueueFull => RigError::Busy,
            Self::NotConnected => RigError::NotConnected,
            Self::Unsupported(cmd) => {
                let static_cmd = match cmd.as_str() {
                    "SetVfo" => "SetVfo",
                    "SetRit" => "SetRit",
                    "SetXit" => "SetXit",
                    "SetPassband" => "SetPassband",
                    _ => "unsupported_command",
                };
                RigError::Unsupported {
                    backend: CatBackendKind::FlexRadio,
                    command: static_cmd,
                }
            }
            Self::Transport(detail) => RigError::Connect {
                endpoint: endpoint.to_string(),
                detail: detail.clone(),
            },
            other => RigError::Protocol {
                detail: other.to_string(),
            },
        }
    }
}

/// Bezpieczny, monotoniczny alokator numerów sekwencyjnych dla komend FlexRadio.
#[derive(Debug)]
pub struct FlexSequenceAllocator {
    next: AtomicU32,
}

impl Default for FlexSequenceAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl FlexSequenceAllocator {
    pub fn new() -> Self {
        Self {
            next: AtomicU32::new(1),
        }
    }

    /// Pobiera kolejny numer sekwencyjny. Gwarantuje pominięcie 0 (rezerwa protokołu)
    /// oraz bezpieczny obieg przy przepełnieniu u32.
    pub fn next_seq(&self) -> u32 {
        loop {
            let current = self.next.fetch_add(1, Ordering::SeqCst);
            if current != 0 {
                return current;
            }
        }
    }

    /// Resetuje stan alokatora do 1 (np. przy nowej sesji TCP).
    pub fn reset(&self) {
        self.next.store(1, Ordering::SeqCst);
    }
}

/// Polecenie wysyłane z klienta do radia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexCommand {
    pub seq: u32,
    pub command: String,
}

impl FlexCommand {
    pub fn new(seq: u32, command: impl Into<String>) -> Self {
        Self {
            seq,
            command: command.into(),
        }
    }

    /// Koduje komendę do postaci bajtowej zakończonej znakiem nowej linii `\n`.
    pub fn encode(&self) -> String {
        format!("C{}|{}\n", self.seq, self.command)
    }
}

/// Odpowiedź radia na komendę klienta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexResponse {
    pub seq: u32,
    pub code: u32,
    pub message: String,
}

impl FlexResponse {
    pub fn is_success(&self) -> bool {
        self.code == 0
    }
}

/// Asynchroniczny komunikat statusu rozsyłany przez radio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexStatus {
    pub client_handle: Option<String>,
    pub object_class: String,
    pub object_id: Option<String>,
    pub properties: BTreeMap<String, String>,
}

/// Komunikat diagnostyczny / informacyjny radia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexMessage {
    pub number: u32,
    pub text: String,
}

/// Wszelkie typy komunikatów przychodzących z radia przez kanał TCP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlexInboundMessage {
    Version(String),
    Handle(String),
    Response(FlexResponse),
    Status(FlexStatus),
    Message(FlexMessage),
    Unknown(String),
}

/// Kodek protokołu strumieniowego FlexRadio.
#[derive(Debug, Clone, Default)]
pub struct FlexProtocolCodec {
    max_line_len: usize,
}

impl FlexProtocolCodec {
    pub fn new() -> Self {
        Self {
            max_line_len: MAX_LINE_LENGTH,
        }
    }

    pub fn with_max_line_len(max_line_len: usize) -> Self {
        Self { max_line_len }
    }

    /// Wyciąga ze strumienia bajtów wszystkie kompletne linie i dekoduje je na komunikaty.
    /// Częściowa linia na końcu bufora pozostaje w buforze do nadejścia kolejnych danych.
    pub fn decode_stream(
        &self,
        buffer: &mut Vec<u8>,
    ) -> Result<Vec<FlexInboundMessage>, FlexProtocolError> {
        let mut messages = Vec::new();

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            if pos > self.max_line_len {
                buffer.drain(..=pos);
                return Err(FlexProtocolError::LineTooLong {
                    max: self.max_line_len,
                    actual: pos,
                });
            }

            let line_bytes = &buffer[..pos];

            let trimmed_bytes =
                if !line_bytes.is_empty() && line_bytes[line_bytes.len() - 1] == b'\r' {
                    &line_bytes[..line_bytes.len() - 1]
                } else {
                    line_bytes
                };

            let line_str = std::str::from_utf8(trimmed_bytes)
                .map_err(|e| FlexProtocolError::InvalidUtf8(e.to_string()))?;

            if !line_str.is_empty() {
                let msg = self.decode_line(line_str)?;
                messages.push(msg);
            }

            buffer.drain(..=pos);
        }

        if buffer.len() > self.max_line_len {
            return Err(FlexProtocolError::LineTooLong {
                max: self.max_line_len,
                actual: buffer.len(),
            });
        }

        Ok(messages)
    }

    /// Dekoduje pojedynczą wyciętą linię tekstu.
    pub fn decode_line(&self, line: &str) -> Result<FlexInboundMessage, FlexProtocolError> {
        if line.is_empty() {
            return Ok(FlexInboundMessage::Unknown(String::new()));
        }

        let first_char = line.chars().next().unwrap_or('\0');
        let rest = &line[1..];

        match first_char {
            'V' => {
                let version = rest.trim();
                if version.is_empty() {
                    return Err(FlexProtocolError::ParseError("empty version string".into()));
                }
                Ok(FlexInboundMessage::Version(version.to_string()))
            }
            'H' => {
                let handle = rest.trim();
                if handle.is_empty() {
                    return Err(FlexProtocolError::InvalidHandle(
                        "empty client handle".into(),
                    ));
                }
                // Handle w SmartSDR to zapis szesnastkowy (np. 0123ABCD lub 0x0123ABCD)
                let clean_handle = handle.strip_prefix("0x").unwrap_or(handle);
                if !clean_handle.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(FlexProtocolError::InvalidHandle(format!(
                        "handle '{handle}' contains non-hex digits"
                    )));
                }
                Ok(FlexInboundMessage::Handle(clean_handle.to_string()))
            }
            'R' => {
                // Format: R<seq>|<hex_status>[|<message>]
                let parts: Vec<&str> = rest.splitn(3, '|').collect();
                if parts.len() < 2 {
                    return Err(FlexProtocolError::ParseError(format!(
                        "malformed response line: '{line}'"
                    )));
                }
                let seq = parts[0].parse::<u32>().map_err(|e| {
                    FlexProtocolError::InvalidSequence(format!(
                        "cannot parse seq '{}': {e}",
                        parts[0]
                    ))
                })?;

                let code_str = parts[1].trim();
                let code_clean = code_str.strip_prefix("0x").unwrap_or(code_str);
                let code = u32::from_str_radix(code_clean, 16).map_err(|e| {
                    FlexProtocolError::ParseError(format!(
                        "cannot parse hex status code '{}': {e}",
                        parts[1]
                    ))
                })?;

                let message = if parts.len() > 2 {
                    parts[2].to_string()
                } else {
                    String::new()
                };

                Ok(FlexInboundMessage::Response(FlexResponse {
                    seq,
                    code,
                    message,
                }))
            }
            'S' => {
                // Format: S<client_handle>|<class> [id] <key>=<value>...
                // lub: S|<class> [id] <key>=<value>...
                let (handle_part, payload) = match rest.split_once('|') {
                    Some((h, p)) => (h.trim(), p.trim()),
                    None => ("", rest.trim()),
                };

                let client_handle = if handle_part.is_empty() || handle_part == "0" {
                    None
                } else {
                    Some(
                        handle_part
                            .strip_prefix("0x")
                            .unwrap_or(handle_part)
                            .to_string(),
                    )
                };

                let mut tokens = payload.split_whitespace();
                let object_class = match tokens.next() {
                    Some(c) => c.to_ascii_lowercase(),
                    None => {
                        return Ok(FlexInboundMessage::Status(FlexStatus {
                            client_handle,
                            object_class: String::new(),
                            object_id: None,
                            properties: BTreeMap::new(),
                        }));
                    }
                };

                let mut object_id = None;
                let mut properties = BTreeMap::new();

                for token in tokens {
                    if let Some((k, v)) = token.split_once('=') {
                        properties.insert(k.to_ascii_lowercase(), v.to_string());
                    } else if object_id.is_none() && !token.contains('=') {
                        object_id = Some(token.to_string());
                    }
                }

                Ok(FlexInboundMessage::Status(FlexStatus {
                    client_handle,
                    object_class,
                    object_id,
                    properties,
                }))
            }
            'M' => {
                // Format: M<num>|<text>
                let (num_str, text) = match rest.split_once('|') {
                    Some((n, t)) => (n.trim(), t.to_string()),
                    None => (rest.trim(), String::new()),
                };
                let number = num_str.parse::<u32>().unwrap_or(0);
                Ok(FlexInboundMessage::Message(FlexMessage { number, text }))
            }
            _ => Ok(FlexInboundMessage::Unknown(line.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flex_protocol_decodes_version() {
        let codec = FlexProtocolCodec::new();
        let msg = codec.decode_line("V3.7.4.29165").expect("decode failed");
        assert_eq!(msg, FlexInboundMessage::Version("3.7.4.29165".to_string()));
    }

    #[test]
    fn flex_protocol_decodes_handle() {
        let codec = FlexProtocolCodec::new();
        let msg = codec.decode_line("H0123ABCD").expect("decode failed");
        assert_eq!(msg, FlexInboundMessage::Handle("0123ABCD".to_string()));

        let msg2 = codec.decode_line("H0x5F3E").expect("decode failed");
        assert_eq!(msg2, FlexInboundMessage::Handle("5F3E".to_string()));
    }

    #[test]
    fn flex_protocol_encodes_command_with_sequence() {
        let cmd = FlexCommand::new(42, "slice set 0 freq=14.074000");
        assert_eq!(cmd.encode(), "C42|slice set 0 freq=14.074000\n");
    }

    #[test]
    fn flex_protocol_decodes_response() {
        let codec = FlexProtocolCodec::new();
        let msg = codec.decode_line("R42|0|OK").expect("decode failed");
        assert_eq!(
            msg,
            FlexInboundMessage::Response(FlexResponse {
                seq: 42,
                code: 0,
                message: "OK".to_string(),
            })
        );
    }

    #[test]
    fn flex_protocol_correlates_response_by_sequence() {
        let codec = FlexProtocolCodec::new();
        let msg1 = codec.decode_line("R101|00000000|OK").expect("msg1 failed");
        let msg2 = codec
            .decode_line("R102|40000001|Error")
            .expect("msg2 failed");

        match msg1 {
            FlexInboundMessage::Response(r1) => {
                assert_eq!(r1.seq, 101);
                assert!(r1.is_success());
            }
            _ => unreachable!("expected response"),
        }

        match msg2 {
            FlexInboundMessage::Response(r2) => {
                assert_eq!(r2.seq, 102);
                assert!(!r2.is_success());
                assert_eq!(r2.code, 0x4000_0001);
            }
            _ => unreachable!("expected response"),
        }
    }

    #[test]
    fn flex_protocol_decodes_status() {
        let codec = FlexProtocolCodec::new();
        let msg = codec
            .decode_line("S0123ABCD|slice 0 RF_frequency=14.025000 in_use=1 mode=CW")
            .expect("decode failed");

        match msg {
            FlexInboundMessage::Status(status) => {
                assert_eq!(status.client_handle, Some("0123ABCD".to_string()));
                assert_eq!(status.object_class, "slice");
                assert_eq!(status.object_id, Some("0".to_string()));
                assert_eq!(
                    status.properties.get("rf_frequency"),
                    Some(&"14.025000".to_string())
                );
                assert_eq!(status.properties.get("in_use"), Some(&"1".to_string()));
                assert_eq!(status.properties.get("mode"), Some(&"CW".to_string()));
            }
            _ => unreachable!("expected status message"),
        }
    }

    #[test]
    fn flex_protocol_handles_fragmented_line() {
        let codec = FlexProtocolCodec::new();
        let mut buffer = Vec::new();

        buffer.extend_from_slice(b"V3.7.");
        let msgs1 = codec.decode_stream(&mut buffer).expect("chunk 1 failed");
        assert_eq!(msgs1.len(), 0);
        assert_eq!(buffer, b"V3.7.");

        buffer.extend_from_slice(b"4.29165\n");
        let msgs2 = codec.decode_stream(&mut buffer).expect("chunk 2 failed");
        assert_eq!(msgs2.len(), 1);
        assert_eq!(
            msgs2[0],
            FlexInboundMessage::Version("3.7.4.29165".to_string())
        );
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn flex_protocol_handles_multiple_lines() {
        let codec = FlexProtocolCodec::new();
        let mut buffer = b"V3.7.4\nH0123ABCD\nR1|0|\n".to_vec();

        let msgs = codec.decode_stream(&mut buffer).expect("multi failed");
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], FlexInboundMessage::Version("3.7.4".to_string()));
        assert_eq!(msgs[1], FlexInboundMessage::Handle("0123ABCD".to_string()));
        assert_eq!(
            msgs[2],
            FlexInboundMessage::Response(FlexResponse {
                seq: 1,
                code: 0,
                message: String::new(),
            })
        );
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn flex_protocol_preserves_partial_line() {
        let codec = FlexProtocolCodec::new();
        let mut buffer = b"V3.7.4\nH012".to_vec();

        let msgs = codec.decode_stream(&mut buffer).expect("stream failed");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], FlexInboundMessage::Version("3.7.4".to_string()));
        assert_eq!(buffer, b"H012");
    }

    #[test]
    fn flex_protocol_enforces_max_line_length() {
        let codec = FlexProtocolCodec::with_max_line_len(64);
        let mut buffer = vec![b'X'; 100];
        buffer.push(b'\n');

        let err = codec
            .decode_stream(&mut buffer)
            .expect_err("should reject oversized line");
        assert!(matches!(err, FlexProtocolError::LineTooLong { .. }));
    }

    #[test]
    fn flex_protocol_unknown_message_is_safe() {
        let codec = FlexProtocolCodec::new();
        let msg = codec
            .decode_line("Z99|unrecognized syntax")
            .expect("unknown safe");
        assert_eq!(
            msg,
            FlexInboundMessage::Unknown("Z99|unrecognized syntax".to_string())
        );
    }

    #[test]
    fn flex_protocol_invalid_known_message_is_error() {
        let codec = FlexProtocolCodec::new();
        let err = codec
            .decode_line("Rnot_a_number|0")
            .expect_err("should fail");
        assert!(matches!(err, FlexProtocolError::InvalidSequence(_)));
    }

    #[test]
    fn flex_protocol_accepts_documented_line_endings() {
        let codec = FlexProtocolCodec::new();
        let mut buf_crlf = b"V3.1.2\r\nH1234\r\n".to_vec();
        let msgs_crlf = codec.decode_stream(&mut buf_crlf).expect("crlf failed");
        assert_eq!(msgs_crlf.len(), 2);

        let mut buf_lf = b"V3.1.2\nH1234\n".to_vec();
        let msgs_lf = codec.decode_stream(&mut buf_lf).expect("lf failed");
        assert_eq!(msgs_lf.len(), 2);
        assert_eq!(msgs_crlf, msgs_lf);
    }

    #[test]
    fn flex_protocol_rejects_invalid_handle() {
        let codec = FlexProtocolCodec::new();
        let err = codec
            .decode_line("Hnot_hex_chars_!")
            .expect_err("invalid handle");
        assert!(matches!(err, FlexProtocolError::InvalidHandle(_)));

        let err_empty = codec.decode_line("H").expect_err("empty handle");
        assert!(matches!(err_empty, FlexProtocolError::InvalidHandle(_)));
    }

    #[test]
    fn flex_protocol_rejects_invalid_sequence() {
        let codec = FlexProtocolCodec::new();
        let err = codec
            .decode_line("Rbad_seq|00000000|")
            .expect_err("invalid seq");
        assert!(matches!(err, FlexProtocolError::InvalidSequence(_)));
    }

    #[test]
    fn flex_protocol_nonzero_response_is_typed() {
        let codec = FlexProtocolCodec::new();
        let msg = codec
            .decode_line("R15|50000002|slice limit reached")
            .expect("decode failed");

        match msg {
            FlexInboundMessage::Response(resp) => {
                assert!(!resp.is_success());
                assert_eq!(resp.code, 0x5000_0002);
                assert_eq!(resp.message, "slice limit reached");
            }
            _ => unreachable!("expected response"),
        }
    }

    #[test]
    fn flex_protocol_unknown_sequence_does_not_complete_other_command() {
        let mut registry =
            crate::cat::native::flexradio::backend::PendingCommandRegistry::default();
        registry.register(10).expect("register failed");

        assert!(!registry.complete(999)); // odpowiedź dla nieznanego seq 999
        assert_eq!(registry.len(), 1); // komenda 10 nadal oczekuje!
        assert!(registry.complete(10)); // komenda 10 zakończona poprawnie
    }

    #[test]
    fn flex_protocol_empty_input_does_not_panic() {
        let codec = FlexProtocolCodec::new();
        let mut empty_buf = Vec::new();
        let msgs = codec.decode_stream(&mut empty_buf).expect("empty stream");
        assert_eq!(msgs.len(), 0);

        let msg = codec.decode_line("").expect("empty line");
        assert_eq!(msg, FlexInboundMessage::Unknown(String::new()));
    }
}
