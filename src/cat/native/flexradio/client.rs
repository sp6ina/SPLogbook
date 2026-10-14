// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Klient połączenia TCP control/status FlexRadio SmartSDR API.
//!
//! Zapewnia asynchroniczne połączenie TCP (domyślnie port 4992),
//! bezpieczny state machine procedury handshake (oczekiwanie na V i H),
//! pojedynczego właściciela strumienia (brak Arc<Mutex<TcpStream>>),
//! asynchroniczne timeouty, buforowanie i idempotentne rozłączenie.

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::models::FLEX_DEFAULT_PORT;
use super::protocol::{
    FlexCommand, FlexInboundMessage, FlexProtocolCodec, FlexProtocolError, MAX_LINE_LENGTH,
};
use super::slice::FlexSliceId;

/// Konfiguracja połączenia z radiem FlexRadio.
#[derive(Clone, PartialEq, Eq)]
pub struct FlexRadioConfig {
    pub host: String,
    pub port: u16,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub max_buffer_size: usize,
    pub preferred_slice: Option<FlexSliceId>,
}

impl Default for FlexRadioConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: FLEX_DEFAULT_PORT,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_millis(1500),
            write_timeout: Duration::from_millis(1500),
            max_buffer_size: 65536,
            preferred_slice: None,
        }
    }
}

impl std::fmt::Debug for FlexRadioConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlexRadioConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("write_timeout", &self.write_timeout)
            .field("max_buffer_size", &self.max_buffer_size)
            .field("preferred_slice", &self.preferred_slice)
            .finish()
    }
}

/// Stany maszyny stanów procedury połączenia SmartSDR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Disconnected,
    Connecting,
    AwaitingInit,
    Ready,
    Closing,
}

/// Niskopoziomowy klient połączenia TCP z radiem.
pub struct FlexRadioConnection {
    config: FlexRadioConfig,
    stream: TcpStream,
    read_buffer: Vec<u8>,
    codec: FlexProtocolCodec,
    handshake_state: HandshakeState,
    version: Option<String>,
    client_handle: Option<String>,
}

impl FlexRadioConnection {
    /// Otwiera nowe asynchroniczne połączenie TCP z radiem.
    pub async fn connect(config: FlexRadioConfig) -> Result<Self, FlexProtocolError> {
        let endpoint = format!("{}:{}", config.host, config.port);
        let connect_fut = TcpStream::connect(&endpoint);

        let stream = match tokio::time::timeout(config.connect_timeout, connect_fut).await {
            Ok(Ok(s)) => {
                let _ = s.set_nodelay(true);
                s
            }
            Ok(Err(e)) => {
                return Err(FlexProtocolError::Transport(format!(
                    "cannot connect to SmartSDR at {endpoint}: {e}"
                )));
            }
            Err(_) => {
                return Err(FlexProtocolError::Timeout {
                    operation: "tcp_connect",
                });
            }
        };

        Ok(Self {
            config,
            stream,
            read_buffer: Vec::with_capacity(4096),
            codec: FlexProtocolCodec::with_max_line_len(MAX_LINE_LENGTH),
            handshake_state: HandshakeState::Connecting,
            version: None,
            client_handle: None,
        })
    }

    pub fn state(&self) -> HandshakeState {
        self.handshake_state
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn client_handle(&self) -> Option<&str> {
        self.client_handle.as_deref()
    }

    /// Przeprowadza pełną procedurę handshake: odbiera `V<version>` i `H<handle>`.
    /// Ustawia stan `Ready` dopiero po poprawnym odebraniu obu wymaganych nagłówków.
    pub async fn perform_handshake(&mut self) -> Result<(String, String), FlexProtocolError> {
        self.handshake_state = HandshakeState::AwaitingInit;

        let start = tokio::time::Instant::now();
        let timeout_duration = self.config.connect_timeout;

        while self.version.is_none() || self.client_handle.is_none() {
            if start.elapsed() >= timeout_duration {
                return Err(FlexProtocolError::Timeout {
                    operation: "handshake",
                });
            }

            let remaining = timeout_duration.saturating_sub(start.elapsed());
            let mut temp = [0u8; 1024];

            let n = match tokio::time::timeout(remaining, self.stream.read(&mut temp)).await {
                Ok(Ok(0)) => {
                    self.handshake_state = HandshakeState::Closing;
                    return Err(FlexProtocolError::Transport(
                        "connection closed by radio during handshake".to_string(),
                    ));
                }
                Ok(Ok(bytes_read)) => bytes_read,
                Ok(Err(e)) => {
                    self.handshake_state = HandshakeState::Closing;
                    return Err(FlexProtocolError::Transport(format!(
                        "read error during handshake: {e}"
                    )));
                }
                Err(_) => {
                    return Err(FlexProtocolError::Timeout {
                        operation: "handshake_read",
                    });
                }
            };

            if self.read_buffer.len() + n > self.config.max_buffer_size {
                return Err(FlexProtocolError::LineTooLong {
                    max: self.config.max_buffer_size,
                    actual: self.read_buffer.len() + n,
                });
            }

            self.read_buffer.extend_from_slice(&temp[..n]);
            let messages = self.codec.decode_stream(&mut self.read_buffer)?;

            for msg in messages {
                match msg {
                    FlexInboundMessage::Version(ver) => {
                        if self.version.is_some() {
                            return Err(FlexProtocolError::DuplicateInitMessage(format!(
                                "version '{ver}' received more than once"
                            )));
                        }
                        self.version = Some(ver);
                    }
                    FlexInboundMessage::Handle(h) => {
                        if self.client_handle.is_some() {
                            return Err(FlexProtocolError::DuplicateInitMessage(format!(
                                "handle '{h}' received more than once"
                            )));
                        }
                        self.client_handle = Some(h);
                    }
                    // Ignorujemy wstępne komunikaty statusu przed zakończeniem nagłówków
                    _ => {}
                }
            }
        }

        let ver = self
            .version
            .clone()
            .ok_or_else(|| FlexProtocolError::HandshakeIncomplete("missing version".into()))?;
        let handle = self
            .client_handle
            .clone()
            .ok_or_else(|| FlexProtocolError::HandshakeIncomplete("missing handle".into()))?;

        self.handshake_state = HandshakeState::Ready;
        Ok((ver, handle))
    }

    /// Wysyła komendę do radia przez strumień TCP z limitem czasu.
    pub async fn send_command(&mut self, cmd: &FlexCommand) -> Result<(), FlexProtocolError> {
        let line = cmd.encode();
        let bytes = line.as_bytes();

        let write_fut = self.stream.write_all(bytes);
        match tokio::time::timeout(self.config.write_timeout, write_fut).await {
            Ok(Ok(())) => {
                let _ = self.stream.flush().await;
                Ok(())
            }
            Ok(Err(e)) => Err(FlexProtocolError::Transport(format!(
                "failed to send command C{}: {e}",
                cmd.seq
            ))),
            Err(_) => Err(FlexProtocolError::Timeout {
                operation: "write_command",
            }),
        }
    }

    /// Odczytuje dostępne komunikaty ze strumienia TCP (nie blokuje w nieskończoność).
    pub async fn read_messages(&mut self) -> Result<Vec<FlexInboundMessage>, FlexProtocolError> {
        let mut temp = [0u8; 4096];

        // Sprawdź czy mamy już coś w buforze
        let mut messages = self.codec.decode_stream(&mut self.read_buffer)?;

        let read_fut = self.stream.read(&mut temp);
        match tokio::time::timeout(self.config.read_timeout, read_fut).await {
            Ok(Ok(0)) => {
                self.handshake_state = HandshakeState::Closing;
                return Err(FlexProtocolError::Transport(
                    "connection closed by radio".to_string(),
                ));
            }
            Ok(Ok(n)) => {
                if self.read_buffer.len() + n > self.config.max_buffer_size {
                    return Err(FlexProtocolError::LineTooLong {
                        max: self.config.max_buffer_size,
                        actual: self.read_buffer.len() + n,
                    });
                }
                self.read_buffer.extend_from_slice(&temp[..n]);
                let more = self.codec.decode_stream(&mut self.read_buffer)?;
                messages.extend(more);
            }
            Ok(Err(e)) => {
                return Err(FlexProtocolError::Transport(format!(
                    "read stream error: {e}"
                )));
            }
            Err(_) => {
                // Timeout odczytu nie oznacza błędu, jeśli odebraliśmy już jakieś wiadomości
                // lub radio po prostu nie ma nowych asynchronicznych komunikatów
            }
        }

        Ok(messages)
    }

    /// Idempotentne, bezpieczne zamknięcie połączenia TCP.
    pub async fn disconnect(&mut self) {
        self.handshake_state = HandshakeState::Disconnected;
        let _ = self.stream.shutdown().await;
        self.read_buffer.clear();
        self.version = None;
        self.client_handle = None;
    }
}
