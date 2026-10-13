// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Asynchroniczny transport szeregowy dla protokołów radiowych (Rust 2024 / Tokio).
//!
//! Zapewnia strumieniowe buforowanie, precyzyjne limity rozmiaru ramki,
//! odporność na fragmentację, asynchroniczne timeouty oraz pełną testowalność
//! bez konieczności podłączania fizycznego sprzętu.

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::cat::backend::CatBackendKind;
use crate::cat::rig::RigError;

/// Bity danych portu szeregowego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SerialDataBits {
    Five,
    Six,
    Seven,
    #[default]
    Eight,
}

impl From<SerialDataBits> for tokio_serial::DataBits {
    fn from(db: SerialDataBits) -> Self {
        match db {
            SerialDataBits::Five => Self::Five,
            SerialDataBits::Six => Self::Six,
            SerialDataBits::Seven => Self::Seven,
            SerialDataBits::Eight => Self::Eight,
        }
    }
}

/// Bity stopu portu szeregowego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SerialStopBits {
    #[default]
    One,
    Two,
}

impl From<SerialStopBits> for tokio_serial::StopBits {
    fn from(sb: SerialStopBits) -> Self {
        match sb {
            SerialStopBits::One => Self::One,
            SerialStopBits::Two => Self::Two,
        }
    }
}

/// Parzystość transmisji szeregowej.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SerialParity {
    #[default]
    None,
    Odd,
    Even,
}

impl From<SerialParity> for tokio_serial::Parity {
    fn from(p: SerialParity) -> Self {
        match p {
            SerialParity::None => Self::None,
            SerialParity::Odd => Self::Odd,
            SerialParity::Even => Self::Even,
        }
    }
}

/// Kontrola przepływu portu szeregowego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SerialFlowControl {
    #[default]
    None,
    Software,
    Hardware,
}

impl From<SerialFlowControl> for tokio_serial::FlowControl {
    fn from(fc: SerialFlowControl) -> Self {
        match fc {
            SerialFlowControl::None => Self::None,
            SerialFlowControl::Software => Self::Software,
            SerialFlowControl::Hardware => Self::Hardware,
        }
    }
}

/// Konfiguracja parametrów portu szeregowego.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: SerialDataBits,
    pub stop_bits: SerialStopBits,
    pub parity: SerialParity,
    pub flow_control: SerialFlowControl,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub max_buffer_size: usize,
}

impl SerialConfig {
    pub fn new(port_name: impl Into<String>, baud_rate: u32) -> Self {
        Self {
            port_name: port_name.into(),
            baud_rate,
            data_bits: SerialDataBits::Eight,
            stop_bits: SerialStopBits::One,
            parity: SerialParity::None,
            flow_control: SerialFlowControl::None,
            connect_timeout: Duration::from_millis(2000),
            read_timeout: Duration::from_millis(600),
            write_timeout: Duration::from_millis(600),
            max_buffer_size: 4096,
        }
    }
}

/// Typowane błędy transportu szeregowego.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Connect { port: String, detail: String },
    Read { detail: String },
    Write { detail: String },
    Timeout { operation: &'static str },
    BufferOverflow { max: usize },
    Closed,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect { port, detail } => {
                write!(f, "cannot open serial port '{port}': {detail}")
            }
            Self::Read { detail } => write!(f, "serial read error: {detail}"),
            Self::Write { detail } => write!(f, "serial write error: {detail}"),
            Self::Timeout { operation } => {
                write!(f, "serial operation '{operation}' timed out")
            }
            Self::BufferOverflow { max } => {
                write!(f, "serial buffer overflow (limit {max} bytes)")
            }
            Self::Closed => write!(f, "serial transport is closed"),
        }
    }
}

impl std::error::Error for TransportError {}

impl TransportError {
    pub fn to_rig_error(&self, backend: CatBackendKind, endpoint: &str) -> RigError {
        match self {
            Self::Connect { detail, .. } => RigError::Connect {
                endpoint: endpoint.to_string(),
                detail: detail.clone(),
            },
            Self::Timeout { operation } => RigError::Timeout {
                endpoint: endpoint.to_string(),
                operation,
            },
            Self::Read { detail } | Self::Write { detail } => RigError::Protocol {
                detail: format!("{backend:?} I/O failure: {detail}"),
            },
            Self::BufferOverflow { max } => RigError::Protocol {
                detail: format!("{backend:?} buffer overflow exceeding limit of {max} bytes"),
            },
            Self::Closed => RigError::NotConnected,
        }
    }
}

/// Asynchroniczny transport ramkowy buforujący dane ze strumienia we/wy.
pub struct AsyncFramedTransport<IO> {
    stream: IO,
    buffer: Vec<u8>,
    max_buffer_size: usize,
    read_timeout: Duration,
    write_timeout: Duration,
    closed: bool,
}

impl<IO> AsyncFramedTransport<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin + Send,
{
    /// Tworzy transport z istniejącego strumienia asynchronicznego.
    pub fn from_stream(
        stream: IO,
        max_buffer_size: usize,
        read_timeout: Duration,
        write_timeout: Duration,
    ) -> Self {
        Self {
            stream,
            buffer: Vec::with_capacity(1024.min(max_buffer_size)),
            max_buffer_size,
            read_timeout,
            write_timeout,
            closed: false,
        }
    }

    /// Sprawdza, czy połączenie jest otwarte.
    pub fn is_connected(&self) -> bool {
        !self.closed
    }

    /// Bieżąca zawartość bufora odbiorczego.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Usuwa `amount` przetworzonych bajtów z początku bufora.
    pub fn consume(&mut self, amount: usize) {
        let count = amount.min(self.buffer.len());
        self.buffer.drain(..count);
    }

    /// Czyści bufor odbiorczy.
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Asynchronicznie zapisuje całość bufora danych z limitem czasu.
    pub async fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }

        let write_future = self.stream.write_all(data);
        match tokio::time::timeout(self.write_timeout, write_future).await {
            Ok(Ok(())) => {
                let flush_future = self.stream.flush();
                match tokio::time::timeout(self.write_timeout, flush_future).await {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(TransportError::Write {
                        detail: e.to_string(),
                    }),
                    Err(_) => Err(TransportError::Timeout { operation: "flush" }),
                }
            }
            Ok(Err(e)) => Err(TransportError::Write {
                detail: e.to_string(),
            }),
            Err(_) => Err(TransportError::Timeout { operation: "write" }),
        }
    }

    /// Asynchronicznie odczytuje kolejny fragment danych ze strumienia do bufora.
    pub async fn read_chunk(&mut self) -> Result<usize, TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }

        if self.buffer.len() >= self.max_buffer_size {
            return Err(TransportError::BufferOverflow {
                max: self.max_buffer_size,
            });
        }

        let mut temp = [0u8; 1024];
        let read_future = self.stream.read(&mut temp);
        match tokio::time::timeout(self.read_timeout, read_future).await {
            Ok(Ok(0)) => {
                self.closed = true;
                Err(TransportError::Closed)
            }
            Ok(Ok(n)) => {
                if self.buffer.len().saturating_add(n) > self.max_buffer_size {
                    return Err(TransportError::BufferOverflow {
                        max: self.max_buffer_size,
                    });
                }
                self.buffer.extend_from_slice(&temp[..n]);
                Ok(n)
            }
            Ok(Err(e)) => Err(TransportError::Read {
                detail: e.to_string(),
            }),
            Err(_) => Err(TransportError::Timeout { operation: "read" }),
        }
    }

    /// Bezpiecznie zamyka połączenie (idempotentne).
    pub async fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            let _ = tokio::time::timeout(self.write_timeout, self.stream.shutdown()).await;
        }
        self.buffer.clear();
    }
}

/// Otwiera fizyczny port szeregowy za pomocą biblioteki `tokio-serial`.
pub fn open_serial_port(
    config: &SerialConfig,
) -> Result<AsyncFramedTransport<tokio_serial::SerialStream>, TransportError> {
    use tokio_serial::SerialPortBuilderExt;

    let builder = tokio_serial::new(&config.port_name, config.baud_rate)
        .data_bits(config.data_bits.into())
        .stop_bits(config.stop_bits.into())
        .parity(config.parity.into())
        .flow_control(config.flow_control.into())
        .timeout(config.read_timeout);

    let stream = builder
        .open_native_async()
        .map_err(|e| TransportError::Connect {
            port: config.port_name.clone(),
            detail: e.to_string(),
        })?;

    Ok(AsyncFramedTransport::from_stream(
        stream,
        config.max_buffer_size,
        config.read_timeout,
        config.write_timeout,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn transport_write_and_read_roundtrip() {
        let (client_io, mut server_io) = duplex(256);
        let mut transport = AsyncFramedTransport::from_stream(
            client_io,
            1024,
            Duration::from_millis(200),
            Duration::from_millis(200),
        );

        assert!(transport.is_connected());
        transport.write_all(b"PING").await.expect("write failed");

        let mut server_buf = [0u8; 4];
        server_io
            .read_exact(&mut server_buf)
            .await
            .expect("server read failed");
        assert_eq!(&server_buf, b"PING");

        server_io
            .write_all(b"PONG")
            .await
            .expect("server write failed");
        let n = transport.read_chunk().await.expect("transport read failed");
        assert_eq!(n, 4);
        assert_eq!(transport.buffer(), b"PONG");

        transport.consume(2);
        assert_eq!(transport.buffer(), b"NG");
        transport.consume(10);
        assert!(transport.buffer().is_empty());
    }

    #[tokio::test]
    async fn transport_enforces_buffer_overflow_limit() {
        let (client_io, mut server_io) = duplex(256);
        let mut transport = AsyncFramedTransport::from_stream(
            client_io,
            8, // Maksymalnie 8 bajtów
            Duration::from_millis(200),
            Duration::from_millis(200),
        );

        server_io
            .write_all(b"123456789")
            .await
            .expect("server write failed");
        let res = transport.read_chunk().await;
        assert!(matches!(
            res,
            Err(TransportError::BufferOverflow { max: 8 })
        ));
    }

    #[tokio::test]
    async fn transport_handles_read_timeout() {
        let (client_io, _server_io) = duplex(256);
        let mut transport = AsyncFramedTransport::from_stream(
            client_io,
            1024,
            Duration::from_millis(50),
            Duration::from_millis(50),
        );

        let res = transport.read_chunk().await;
        assert_eq!(res, Err(TransportError::Timeout { operation: "read" }));
    }

    #[tokio::test]
    async fn transport_detects_clean_stream_closure() {
        let (client_io, server_io) = duplex(256);
        let mut transport = AsyncFramedTransport::from_stream(
            client_io,
            1024,
            Duration::from_millis(200),
            Duration::from_millis(200),
        );

        drop(server_io); // Zamknięcie po stronie serwera
        let res = transport.read_chunk().await;
        assert_eq!(res, Err(TransportError::Closed));
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn transport_close_is_idempotent() {
        let (client_io, _server_io) = duplex(256);
        let mut transport = AsyncFramedTransport::from_stream(
            client_io,
            1024,
            Duration::from_millis(200),
            Duration::from_millis(200),
        );

        transport.close().await;
        assert!(!transport.is_connected());
        transport.close().await; // Powtórne zamknięcie nie powinno panikować
        assert!(!transport.is_connected());
    }
}
