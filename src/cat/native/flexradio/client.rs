// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Klient połączenia TCP control/status FlexRadio SmartSDR API.
//!
//! Zapewnia asynchroniczne połączenie TCP (domyślnie port 4992),
//! bezpieczny state machine procedury handshake (oczekiwanie na V i H w dowolnej kolejności),
//! zachowanie statusów nadeszłych w trakcie inicjalizacji,
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
    pub expected_model: Option<String>,
    pub expected_serial: Option<String>,
    pub client_id: Option<String>,
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
            expected_model: None,
            expected_serial: None,
            client_id: None,
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
            .field("expected_model", &self.expected_model)
            .field("expected_serial", &self.expected_serial)
            .field("client_id", &self.client_id)
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
    early_messages: Vec<FlexInboundMessage>,
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
            early_messages: Vec::new(),
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
    /// Akceptuje dowolną kolejność nagłówków, odrzuca sprzeczne duplikaty,
    /// buforuje statusy nadeszłe w trakcie procedury oraz stosuje łączny deadline handshake.
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
            if remaining.is_zero() {
                return Err(FlexProtocolError::Timeout {
                    operation: "handshake",
                });
            }

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
                        operation: "handshake",
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
                        if let Some(existing) = self.version.as_deref().filter(|&e| e != ver) {
                            return Err(FlexProtocolError::DuplicateInitMessage(format!(
                                "conflicting version received: '{existing}' vs '{ver}'"
                            )));
                        }
                        self.version = Some(ver);
                    }
                    FlexInboundMessage::Handle(h) => {
                        if let Some(existing) = self.client_handle.as_deref().filter(|&e| e != h) {
                            return Err(FlexProtocolError::DuplicateInitMessage(format!(
                                "conflicting handle received: '{existing}' vs '{h}'"
                            )));
                        }
                        self.client_handle = Some(h);
                    }
                    status @ FlexInboundMessage::Status(_) => {
                        self.early_messages.push(status);
                    }
                    other => {
                        self.early_messages.push(other);
                    }
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
    /// Zwraca również komunikaty przechowane podczas handshake (`early_messages`).
    pub async fn read_messages(&mut self) -> Result<Vec<FlexInboundMessage>, FlexProtocolError> {
        let mut messages = std::mem::take(&mut self.early_messages);
        let buffered = self.codec.decode_stream(&mut self.read_buffer)?;
        messages.extend(buffered);

        let mut temp = [0u8; 4096];
        let read_fut = self.stream.read(&mut temp);
        match tokio::time::timeout(self.config.read_timeout, read_fut).await {
            Ok(Ok(0)) => {
                self.handshake_state = HandshakeState::Closing;
                if !messages.is_empty() {
                    return Ok(messages);
                }
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
                // Timeout odczytu nie oznacza błędu, jeśli radio po prostu nie ma nowych asynchronicznych komunikatów
            }
        }

        Ok(messages)
    }

    /// Idempotentne, bezpieczne zamknięcie połączenia TCP.
    pub async fn disconnect(&mut self) {
        self.handshake_state = HandshakeState::Disconnected;
        let _ = self.stream.shutdown().await;
        self.read_buffer.clear();
        self.early_messages.clear();
        self.version = None;
        self.client_handle = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    async fn setup_mock_socket() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
        let port = listener.local_addr().expect("port mock").port();
        (listener, port)
    }

    #[tokio::test]
    async fn flex_handshake_accepts_documented_order() {
        // Test 1: V potem H
        let (listener1, port1) = setup_mock_socket().await;
        let s1 = tokio::spawn(async move {
            let (mut sock, _) = listener1.accept().await.expect("accept");
            sock.write_all(b"V3.7.4\nH0123ABCD\n").await.expect("write");
        });
        let config1 = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port: port1,
            ..Default::default()
        };
        let mut conn1 = FlexRadioConnection::connect(config1)
            .await
            .expect("connect");
        let (ver1, handle1) = conn1.perform_handshake().await.expect("handshake");
        assert_eq!(ver1, "3.7.4");
        assert_eq!(handle1, "0123ABCD");
        s1.await.expect("server join");
        conn1.disconnect().await;

        // Test 2: H potem V (odwrotna kolejność)
        let (listener2, port2) = setup_mock_socket().await;
        let s2 = tokio::spawn(async move {
            let (mut sock, _) = listener2.accept().await.expect("accept");
            sock.write_all(b"HFEEDFACE\nV3.8.0\n").await.expect("write");
        });
        let config2 = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port: port2,
            ..Default::default()
        };
        let mut conn2 = FlexRadioConnection::connect(config2)
            .await
            .expect("connect");
        let (ver2, handle2) = conn2.perform_handshake().await.expect("handshake");
        assert_eq!(ver2, "3.8.0");
        assert_eq!(handle2, "FEEDFACE");
        s2.await.expect("server join");
        conn2.disconnect().await;
    }

    #[tokio::test]
    async fn flex_handshake_handles_interleaved_status() {
        let (listener, port) = setup_mock_socket().await;
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.expect("accept");
            // Status nadsyłany między V a H
            sock.write_all(
                b"V3.7.4\nS0123ABCD|slice 0 in_use=1 rf_frequency=14.074000\nH0123ABCD\n",
            )
            .await
            .expect("write");
            let mut buf = [0u8; 16];
            let _ = sock.read(&mut buf).await;
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            read_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut conn = FlexRadioConnection::connect(config).await.expect("connect");
        let (ver, handle) = conn.perform_handshake().await.expect("handshake");
        assert_eq!(ver, "3.7.4");
        assert_eq!(handle, "0123ABCD");

        // Status nie został utracony i pojawia się przy read_messages
        let msgs = conn.read_messages().await.expect("read failed");
        assert!(
            msgs.iter()
                .any(|m| matches!(m, FlexInboundMessage::Status(s) if s.object_class == "slice"))
        );

        conn.disconnect().await;
        server.await.expect("server join");
    }

    #[tokio::test]
    async fn flex_handshake_rejects_conflicting_version() {
        let (listener, port) = setup_mock_socket().await;
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.expect("accept");
            sock.write_all(b"V3.7.4\nV3.8.0\nH0123ABCD\n")
                .await
                .expect("write");
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut conn = FlexRadioConnection::connect(config).await.expect("connect");
        let err = conn
            .perform_handshake()
            .await
            .expect_err("should reject conflicting version");
        assert!(matches!(err, FlexProtocolError::DuplicateInitMessage(_)));

        server.await.expect("server join");
        conn.disconnect().await;
    }

    #[tokio::test]
    async fn flex_handshake_rejects_conflicting_handle() {
        let (listener, port) = setup_mock_socket().await;
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.expect("accept");
            sock.write_all(b"V3.7.4\nH1111\nH2222\n")
                .await
                .expect("write");
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut conn = FlexRadioConnection::connect(config).await.expect("connect");
        let err = conn
            .perform_handshake()
            .await
            .expect_err("should reject conflicting handle");
        assert!(matches!(err, FlexProtocolError::DuplicateInitMessage(_)));

        server.await.expect("server join");
        conn.disconnect().await;
    }

    #[tokio::test]
    async fn flex_handshake_uses_total_deadline() {
        let (listener, port) = setup_mock_socket().await;
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.expect("accept");
            // Wysyłamy V, a potem milczymy
            sock.write_all(b"V3.7.4\n").await.expect("write");
            tokio::time::sleep(Duration::from_millis(150)).await;
        });

        let config = FlexRadioConfig {
            host: "127.0.0.1".to_string(),
            port,
            connect_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut conn = FlexRadioConnection::connect(config).await.expect("connect");
        let err = conn.perform_handshake().await.expect_err("should timeout");
        assert!(matches!(
            err,
            FlexProtocolError::Timeout {
                operation: "handshake"
            }
        ));

        server.await.expect("server join");
        conn.disconnect().await;
    }
}
