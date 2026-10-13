// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Asynchroniczny transport sieciowy Kenwood Network Command System (KNS LAN).
//!
//! Dedykowany dla transceiverów z wbudowanym interfejsem Ethernet (TS-890S, TS-990S).
//! Działa w oparciu o protokół TCP na porcie 60000 z sekwencją handshake `##CN`,
//! opcjonalnym uwierzytelnianiem `##ID` oraz bezpiecznym maskowaniem haseł.

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::cat::backend::CatBackendKind;
use crate::cat::native::kenwood::codec::{KenwoodCodec, KenwoodFrame};
use crate::cat::native::transport::AsyncFramedTransport;
use crate::cat::rig::RigError;

/// Domyślny port TCP dla systemu Kenwood Network Command System.
pub const KENWOOD_KNS_DEFAULT_PORT: u16 = 60000;

/// Cecha strumienia we/wy dla testów deterministycznych i połączeń TCP.
pub trait AsyncReaderWriter: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncReaderWriter for T {}

/// Konfiguracja połączenia sieciowego KNS LAN.
#[derive(Clone, PartialEq, Eq)]
pub struct KenwoodLanConfig {
    /// Adres IP lub nazwa hosta transceivera.
    pub host: String,
    /// Numer portu TCP (domyślnie 60000).
    pub port: u16,
    /// Opcjonalna nazwa użytkownika (jeśli skonfigurowana w menu radia).
    pub username: Option<String>,
    /// Opcjonalne hasło dostępowe do transceivera.
    pub password: Option<String>,
    /// Limit czasu nawiązywania połączenia TCP.
    pub connect_timeout: Duration,
    /// Limit czasu odczytu danych.
    pub read_timeout: Duration,
    /// Limit czasu zapisu danych.
    pub write_timeout: Duration,
    /// Maksymalny rozmiar bufora odbiorczego.
    pub max_buffer_size: usize,
}

impl std::fmt::Debug for KenwoodLanConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KenwoodLanConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "***"))
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("write_timeout", &self.write_timeout)
            .field("max_buffer_size", &self.max_buffer_size)
            .finish()
    }
}

impl KenwoodLanConfig {
    /// Tworzy domyślną konfigurację LAN dla zadanego hosta.
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: KENWOOD_KNS_DEFAULT_PORT,
            username: None,
            password: None,
            connect_timeout: Duration::from_millis(3000),
            read_timeout: Duration::from_millis(1000),
            write_timeout: Duration::from_millis(1000),
            max_buffer_size: 4096,
        }
    }

    /// Ustawia poświadczenia uwierzytelniania KNS.
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }
}

/// Asynchroniczny transport KNS LAN dla protokołu Kenwood.
pub struct KenwoodLanTransport {
    framed: AsyncFramedTransport<Box<dyn AsyncReaderWriter>>,
    endpoint: String,
    codec: KenwoodCodec,
}

impl std::fmt::Debug for KenwoodLanTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KenwoodLanTransport")
            .field("endpoint", &self.endpoint)
            .field("connected", &self.framed.is_connected())
            .finish_non_exhaustive()
    }
}

impl KenwoodLanTransport {
    /// Nawiązuje rzeczywiste połączenie TCP i przeprowadza sekwencję powitania.
    pub async fn connect(config: &KenwoodLanConfig) -> Result<Self, RigError> {
        let endpoint = format!("{}:{}", config.host, config.port);
        let connect_fut = tokio::net::TcpStream::connect(&endpoint);

        let stream = match tokio::time::timeout(config.connect_timeout, connect_fut).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                return Err(RigError::Connect {
                    endpoint,
                    detail: format!("TCP connect failed: {e}"),
                });
            }
            Err(_) => {
                return Err(RigError::Timeout {
                    endpoint,
                    operation: "tcp_connect",
                });
            }
        };

        let boxed_stream: Box<dyn AsyncReaderWriter> = Box::new(stream);
        let framed = AsyncFramedTransport::from_stream(
            boxed_stream,
            config.max_buffer_size,
            config.read_timeout,
            config.write_timeout,
        );

        let mut transport = Self {
            framed,
            endpoint,
            codec: KenwoodCodec::new(),
        };

        transport.handshake(config).await?;
        Ok(transport)
    }

    /// Tworzy transport KNS z wstrzykniętego strumienia I/O (dla testów).
    pub async fn from_stream(
        stream: Box<dyn AsyncReaderWriter>,
        config: &KenwoodLanConfig,
    ) -> Result<Self, RigError> {
        let endpoint = format!("{}:{}", config.host, config.port);
        let framed = AsyncFramedTransport::from_stream(
            stream,
            config.max_buffer_size,
            config.read_timeout,
            config.write_timeout,
        );

        let mut transport = Self {
            framed,
            endpoint,
            codec: KenwoodCodec::new(),
        };

        transport.handshake(config).await?;
        Ok(transport)
    }

    /// Przeprowadza procedurę negocjacji KNS (Connect Request `##CN` i opcjonalne uwierzytelnianie `##ID`).
    async fn handshake(&mut self, config: &KenwoodLanConfig) -> Result<(), RigError> {
        // 1. Wyślij zapytanie o połączenie
        self.framed
            .write_all(b"##CN;\r\n")
            .await
            .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &self.endpoint))?;

        // 2. Oczekuj odpowiedzi na ##CN
        let cn_resp = self.read_frame().await?;
        match cn_resp.command.as_str() {
            "##CN" => {
                match cn_resp.argument.as_str() {
                    // 0 = połączenie zaakceptowane bez hasła
                    "0" => Ok(()),
                    // 1 = wymagane uwierzytelnienie
                    "1" => {
                        let user = config.username.as_deref().unwrap_or("");
                        let pass = config.password.as_deref().unwrap_or("");

                        let auth_cmd = if user.is_empty() {
                            format!("##ID {pass};\r\n")
                        } else {
                            format!("##ID {user} {pass};\r\n")
                        };

                        // Bezpieczny zapis poświadczeń (bez ich logowania)
                        self.framed
                            .write_all(auth_cmd.as_bytes())
                            .await
                            .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &self.endpoint))?;

                        let id_resp = self.read_frame().await?;
                        if id_resp.command == "##ID" && id_resp.argument == "0" {
                            Ok(())
                        } else {
                            Err(RigError::Protocol {
                                detail: "Kenwood LAN authentication failed (invalid credentials)"
                                    .to_string(),
                            })
                        }
                    }
                    // 2 = radio zajęte (limit połączeń)
                    "2" => Err(RigError::Connect {
                        endpoint: self.endpoint.clone(),
                        detail: "Kenwood LAN connection rejected (radio busy / maximum connections reached)"
                            .to_string(),
                    }),
                    other => Err(RigError::Protocol {
                        detail: format!("unexpected Kenwood LAN handshake response: ##CN{other}"),
                    }),
                }
            }
            other => Err(RigError::Protocol {
                detail: format!("expected ##CN handshake response, got: {other}"),
            }),
        }
    }

    /// Odczytuje kolejną ramkę z bufora lub oczekuje na dane z sieci.
    pub async fn read_frame(&mut self) -> Result<KenwoodFrame, RigError> {
        loop {
            // Spróbuj zdekodować z aktualnego bufora
            match self.codec.decode_frame(self.framed.buffer()) {
                Ok(Some((frame, consumed))) => {
                    self.framed.consume(consumed);
                    if frame.command.is_empty() {
                        continue;
                    }
                    return Ok(frame);
                }
                Ok(None) => {
                    // Pobierz kolejną porcję danych
                    self.framed
                        .read_chunk()
                        .await
                        .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &self.endpoint))?;
                }
                Err(e) => {
                    return Err(RigError::Protocol {
                        detail: format!("Kenwood LAN codec error: {e}"),
                    });
                }
            }
        }
    }

    /// Zapisuje tekst komendy do transceivera.
    pub async fn write_command(&mut self, cmd: &str) -> Result<(), RigError> {
        let bytes = self.codec.encode_command(cmd);
        self.framed
            .write_all(&bytes)
            .await
            .map_err(|e| e.to_rig_error(CatBackendKind::Kenwood, &self.endpoint))
    }

    /// Rozłącza sesję sieciową wysyłając `##BY;` (idempotentne).
    pub async fn disconnect(&mut self) {
        if self.framed.is_connected() {
            let _ = self.framed.write_all(b"##BY;\r\n").await;
            self.framed.close().await;
        }
    }

    /// Zwraca referencję do wewnętrznego transportu ramkowego.
    pub fn framed_mut(&mut self) -> &mut AsyncFramedTransport<Box<dyn AsyncReaderWriter>> {
        &mut self.framed
    }

    /// Przekształca transport KNS w bazowy transport ramkowy.
    pub fn into_framed(self) -> AsyncFramedTransport<Box<dyn AsyncReaderWriter>> {
        self.framed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    #[tokio::test]
    async fn test_kenwood_lan_connect_and_auth_success() {
        let (client_io, mut radio_io) = duplex(512);
        let config = KenwoodLanConfig::new("192.168.1.50").with_credentials("sp6ina", "secretpass");

        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            // 1. Odczytaj ##CN;
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"##CN;\r\n");

            // 2. Zażądaj hasła: ##CN1;
            radio_io.write_all(b"##CN1;").await.unwrap();

            // 3. Odczytaj poświadczenia: ##ID sp6ina secretpass;\r\n
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"##ID sp6ina secretpass;\r\n");

            // 4. Potwierdź logowanie: ##ID0;
            radio_io.write_all(b"##ID0;").await.unwrap();
        });

        let mut transport = KenwoodLanTransport::from_stream(Box::new(client_io), &config)
            .await
            .expect("lan handshake success");

        assert!(transport.framed.is_connected());
        transport.disconnect().await;
    }

    #[tokio::test]
    async fn test_kenwood_lan_connect_no_auth_needed() {
        let (client_io, mut radio_io) = duplex(512);
        let config = KenwoodLanConfig::new("192.168.1.50");

        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            let n = radio_io.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"##CN;\r\n");

            // Bezpośredni sukces bez hasła: ##CN0;
            radio_io.write_all(b"##CN0;").await.unwrap();
        });

        let transport = KenwoodLanTransport::from_stream(Box::new(client_io), &config)
            .await
            .expect("lan connect without auth");

        assert!(transport.framed.is_connected());
    }

    #[tokio::test]
    async fn test_kenwood_lan_auth_rejected() {
        let (client_io, mut radio_io) = duplex(512);
        let config = KenwoodLanConfig::new("192.168.1.50").with_credentials("admin", "wrong");

        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"##CN1;").await.unwrap();

            let _ = radio_io.read(&mut buf).await.unwrap();
            // Odrzuć poświadczenia: ##ID1;
            radio_io.write_all(b"##ID1;").await.unwrap();
        });

        let res = KenwoodLanTransport::from_stream(Box::new(client_io), &config).await;
        match res {
            Err(RigError::Protocol { detail }) => {
                assert!(detail.contains("authentication failed"));
            }
            _ => panic!("expected auth failure error, got: {res:?}"),
        }
    }

    #[tokio::test]
    async fn test_kenwood_lan_radio_busy() {
        let (client_io, mut radio_io) = duplex(512);
        let config = KenwoodLanConfig::new("192.168.1.50");

        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            let _ = radio_io.read(&mut buf).await.unwrap();
            // Radio zajęte: ##CN2;
            radio_io.write_all(b"##CN2;").await.unwrap();
        });

        let res = KenwoodLanTransport::from_stream(Box::new(client_io), &config).await;
        match res {
            Err(RigError::Connect { detail, .. }) => {
                assert!(detail.contains("radio busy"));
            }
            _ => panic!("expected busy connect error, got: {res:?}"),
        }
    }

    #[test]
    fn test_kenwood_lan_password_masked_in_debug() {
        let config =
            KenwoodLanConfig::new("10.0.0.1").with_credentials("operator", "supersecret12345");

        let debug_str = format!("{config:?}");
        assert!(!debug_str.contains("supersecret12345"));
        assert!(debug_str.contains("***"));
    }

    #[tokio::test]
    async fn test_kenwood_lan_timeout_handling() {
        let (client_io, _radio_io) = duplex(512);
        let mut config = KenwoodLanConfig::new("192.168.1.50");
        config.read_timeout = Duration::from_millis(50);

        let res = KenwoodLanTransport::from_stream(Box::new(client_io), &config).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_kenwood_lan_disconnect_idempotent() {
        let (client_io, mut radio_io) = duplex(512);
        let config = KenwoodLanConfig::new("192.168.1.50");

        tokio::spawn(async move {
            let mut buf = [0u8; 64];
            let _ = radio_io.read(&mut buf).await.unwrap();
            radio_io.write_all(b"##CN0;").await.unwrap();

            // Odbierz ##BY;
            let _ = radio_io.read(&mut buf).await;
        });

        let mut transport = KenwoodLanTransport::from_stream(Box::new(client_io), &config)
            .await
            .expect("connected");

        transport.disconnect().await;
        transport.disconnect().await; // Idempotent
    }
}
