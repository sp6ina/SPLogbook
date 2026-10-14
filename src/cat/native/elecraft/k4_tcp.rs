// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Asynchroniczny transport sieciowy TCP dla transceivera Elecraft K4 / K4D.
//!
//! Zgodny ze specyfikacją: *Elecraft K4 Programmer's Reference*, Rev D12.
//!
//! Działa w oparciu o protokół TCP na porcie 9200 (standardowy kanał sterowania i komend CAT).
//! K4 natywnie obsługuje wielu równorzędnych klientów sterujących, a każdy z nich
//! korzysta z tego samego tekstowego strumienia ASCII zakończonego średnikami `;`.
//!
//! W warstwie NATIVE-4 implementowany jest wyłącznie Control Plane (sterowanie, status, Auto-Info).
//! Data Plane (strumienie IQ, próbki panadaptera, cyfrowe audio RX/TX) pozostaje poza zakresem.

use std::time::Duration;
use tokio::net::TcpStream;

use crate::cat::native::transport::TransportError;

/// Domyślny oficjalny port TCP protokołu sterowania radiem Elecraft K4.
pub const K4_DEFAULT_TCP_PORT: u16 = 9200;

/// Konfiguracja połączenia sieciowego TCP dla Elecraft K4.
#[derive(Clone, PartialEq, Eq)]
pub struct ElecraftTcpConfig {
    /// Adres IP lub nazwa hosta transceivera K4.
    pub host: String,
    /// Numer portu TCP (domyślnie 9200).
    pub port: u16,
    /// Limit czasu nawiązywania połączenia TCP.
    pub connect_timeout: Duration,
    /// Limit czasu odczytu danych.
    pub read_timeout: Duration,
    /// Limit czasu zapisu danych.
    pub write_timeout: Duration,
    /// Maksymalny rozmiar bufora odbiorczego.
    pub max_buffer_size: usize,
}

impl Default for ElecraftTcpConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: K4_DEFAULT_TCP_PORT,
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_millis(1500),
            write_timeout: Duration::from_millis(1500),
            max_buffer_size: 4096,
        }
    }
}

impl std::fmt::Debug for ElecraftTcpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElecraftTcpConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("write_timeout", &self.write_timeout)
            .field("max_buffer_size", &self.max_buffer_size)
            .finish()
    }
}

/// Asynchronicznie otwiera strumień TCP do transceivera K4 z limitem czasu i opcją TCP_NODELAY.
pub async fn open_k4_tcp_stream(config: &ElecraftTcpConfig) -> Result<TcpStream, TransportError> {
    let endpoint = format!("{}:{}", config.host, config.port);
    let connect_future = TcpStream::connect(&endpoint);

    match tokio::time::timeout(config.connect_timeout, connect_future).await {
        Ok(Ok(stream)) => {
            let _ = stream.set_nodelay(true);
            Ok(stream)
        }
        Ok(Err(e)) => Err(TransportError::Connect {
            port: endpoint.clone(),
            detail: format!("TCP connect to {endpoint} failed: {e}"),
        }),
        Err(_) => Err(TransportError::Timeout {
            operation: "tcp_connect",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::backend::{CatBackendKind, RigBackend};
    use crate::cat::native::elecraft::backend::ElecraftBackend;
    use crate::cat::native::elecraft::models::ELECRAFT_K4_PROFILE;
    use crate::cat::rig::{RigCommand, RigError};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn elecraft_k4_tcp_connects() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 128];
            // K4 Connect faza 1: wysyła ID;
            let n = socket.read(&mut buf).await.unwrap();
            assert!(std::str::from_utf8(&buf[..n]).unwrap().contains("ID;"));
            socket.write_all(b"ID017;").await.unwrap();

            // Następnie czyta FA; FB; MD; IF;
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"FA00014074000;FB00007074000;MD2;IF00014074000     +000000 000200001;")
                .await
                .unwrap();

            // Faza 2: AI2;
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"AI2;").await.unwrap();
        });

        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        backend.connect().await.expect("K4 TCP should connect");
        assert!(backend.is_ai_active());
        assert_eq!(backend.current_state().frequency_hz, 14_074_000);
        assert_eq!(backend.current_state().mode, "USB");

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_reads_identification() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 128];
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"ID017;").await.unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"AI2;").await.unwrap();
        });

        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        backend.connect().await.unwrap();
        assert_eq!(backend.kind(), CatBackendKind::Elecraft);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_handles_multiple_clients_semantics() {
        // Symulacja dwóch niezależnych klientów TCP podłączonych równolegle do K4 port 9200
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            // Obsłuż klienta 1
            let (mut s1, _) = listener.accept().await.unwrap();
            let h1 = tokio::spawn(async move {
                let mut b1 = [0u8; 128];
                let _ = s1.read(&mut b1).await.unwrap();
                s1.write_all(b"ID017;").await.unwrap();
                let _ = s1.read(&mut b1).await.unwrap();
                s1.write_all(b"FA00014074000;FB00007074000;MD2;")
                    .await
                    .unwrap();
                let _ = s1.read(&mut b1).await.unwrap();
                s1.write_all(b"AI2;").await.unwrap();

                // Rozgłoś zdarzenie Auto-Info
                s1.write_all(b"FA00014080000;").await.unwrap();
                let _ = s1.read(&mut b1).await;
            });

            // Obsłuż klienta 2
            let (mut s2, _) = listener.accept().await.unwrap();
            let h2 = tokio::spawn(async move {
                let mut b2 = [0u8; 128];
                let _ = s2.read(&mut b2).await.unwrap();
                s2.write_all(b"ID017;").await.unwrap();
                let _ = s2.read(&mut b2).await.unwrap();
                s2.write_all(b"FA00014074000;FB00007074000;MD2;")
                    .await
                    .unwrap();
                let _ = s2.read(&mut b2).await.unwrap();
                s2.write_all(b"AI2;").await.unwrap();

                // Rozgłoś zdarzenie Auto-Info
                s2.write_all(b"FA00014080000;").await.unwrap();
                let _ = s2.read(&mut b2).await;
            });

            let _ = tokio::join!(h1, h2);
        });

        let cfg1 = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };
        let cfg2 = cfg1.clone();

        let mut c1 = ElecraftBackend::new_tcp(cfg1, ELECRAFT_K4_PROFILE.clone());
        let mut c2 = ElecraftBackend::new_tcp(cfg2, ELECRAFT_K4_PROFILE.clone());

        c1.connect().await.unwrap();
        c2.connect().await.unwrap();

        // Oba klienty odbierają stan i Auto-Info
        let st1 = c1.poll().await.unwrap();
        let st2 = c2.poll().await.unwrap();

        assert_eq!(st1.frequency_hz, 14_080_000);
        assert_eq!(st2.frequency_hz, 14_080_000);

        c1.disconnect().await;
        c2.disconnect().await;
        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_timeout_is_typed() {
        // Port bez nasłuchującego serwera ze sztucznie krótkim czasem
        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port: 1, // Port 1 (zamknięty)
            connect_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        let res = backend.connect().await;
        assert!(matches!(
            res,
            Err(RigError::Connect { .. } | RigError::Timeout { .. })
        ));
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_disconnect_is_idempotent() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 128];
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"ID017;").await.unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"AI2;").await.unwrap();

            // Przy rozłączeniu backend wysyła AI0;
            let n = socket.read(&mut buf).await.unwrap_or(0);
            if n > 0 {
                let s = std::str::from_utf8(&buf[..n]).unwrap_or("");
                assert!(s.contains("AI0;"));
            }
        });

        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        backend.connect().await.unwrap();

        // Wielokrotne rozłączenie
        backend.disconnect().await;
        backend.disconnect().await;
        backend.disconnect().await;

        assert!(!backend.is_ai_active());
        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_auto_info_updates_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 128];
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"ID017;").await.unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"AI2;").await.unwrap();

            // Spontaniczne zdarzenie Auto-Info zmiany emisji na CW:
            socket.write_all(b"MD3;").await.unwrap();

            // Utrzymuj gniazdo otwarte dopóki klient nie rozłączy się
            let _ = socket.read(&mut buf).await;
        });

        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        backend.connect().await.unwrap();

        let st = backend.poll().await.unwrap();
        assert_eq!(st.mode, "CW");

        backend.disconnect().await;
        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_interleaves_responses_and_events() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 128];
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"ID017;").await.unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"AI2;").await.unwrap();

            // Klient wysyła komendę SetFrequency (FA00007040000;),
            // a radio przeplata odpowiedź niespodziewanym zdarzeniem BG (bargraph)
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"BG05R;FA00007040000;").await.unwrap();
        });

        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        backend.connect().await.unwrap();

        backend
            .execute(&RigCommand::SetFrequency(7_040_000))
            .await
            .expect("Interleaved BG event must not break SetFrequency");
        assert_eq!(backend.current_state().frequency_hz, 7_040_000);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_does_not_start_streaming() {
        // Weryfikacja, że K4 TCP nie otwiera strumieni IQ, panadaptera ani audio
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 128];
            let n = socket.read(&mut buf).await.unwrap();
            let s = std::str::from_utf8(&buf[..n]).unwrap();
            // Żadna komenda strumieniowa (np. #IQ, #PAN, #AUD) nie może być wysyłana
            assert!(!s.contains("#IQ"));
            assert!(!s.contains("#PAN"));
            assert!(!s.contains("#AUD"));
            socket.write_all(b"ID017;").await.unwrap();

            let n2 = socket.read(&mut buf).await.unwrap();
            let s2 = std::str::from_utf8(&buf[..n2]).unwrap();
            assert!(!s2.contains("#IQ"));
            assert!(!s2.contains("#PAN"));
            assert!(!s2.contains("#AUD"));
            socket
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();

            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"AI2;").await.unwrap();
        });

        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        backend.connect().await.unwrap();

        // Stan nie zawiera danych bin/dB panadaptera ani strumienia IQ
        let state = backend.current_state();
        assert_eq!(state.frequency_hz, 14_074_000);

        server_jh.await.unwrap();
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_uses_dynamic_test_port() {
        // Wszystkie testy bindują na porcie 0
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert_ne!(port, 0);
        assert_ne!(port, K4_DEFAULT_TCP_PORT);
    }

    #[tokio::test]
    async fn elecraft_k4_tcp_tasks_are_awaited() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server_jh = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 128];
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"ID017;").await.unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"FA00014074000;FB00007074000;MD2;")
                .await
                .unwrap();
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(b"AI2;").await.unwrap();
        });

        let config = ElecraftTcpConfig {
            host: "127.0.0.1".to_string(),
            port,
            ..Default::default()
        };

        let mut backend = ElecraftBackend::new_tcp(config, ELECRAFT_K4_PROFILE.clone());
        backend.connect().await.unwrap();

        // Oczekujemy na zakończenie zadania asynchronicznego
        server_jh
            .await
            .expect("Server task should be awaited without errors");
    }
}
