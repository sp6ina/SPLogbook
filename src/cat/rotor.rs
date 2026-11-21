// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::tcp::{self, MAX_LINE_LEN};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::time::sleep;

const DAEMON: &str = "rotctld";

async fn connect_timeout(addr: &str) -> Result<TcpStream, std::io::Error> {
    tcp::connect_timeout(addr, DAEMON).await
}

async fn write_cmd<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    cmd: &[u8],
) -> Result<(), std::io::Error> {
    tcp::write_cmd(writer, cmd, DAEMON).await
}

async fn read_line_timeout<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
    line: &mut String,
) -> Result<usize, std::io::Error> {
    tcp::read_line_timeout(reader, line, DAEMON).await
}

/// Błąd polecenia wysłanego do rotctld.
#[derive(Debug)]
pub enum RotctldError {
    /// Brak połączenia, przekroczony czas albo inny błąd gniazda.
    Io(std::io::Error),
    /// rotctld odpowiedział `RPRT <kod>` z kodem różnym od zera.
    Rejected(i32),
}

impl std::fmt::Display for RotctldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Rejected(code) => write!(f, "RPRT {code}"),
        }
    }
}

impl std::error::Error for RotctldError {}

/// Kod z odpowiedzi `RPRT <n>`; `None`, gdy linia nie jest raportem rotctld.
fn rprt_code(reply: &str) -> Option<i32> {
    reply.trim().strip_prefix("RPRT")?.trim().parse().ok()
}

async fn send_rotor_cmd(host: &str, port: u16, cmd: &str) -> Result<(), RotctldError> {
    let reply = tcp::send_cmd_read_reply(&format!("{host}:{port}"), cmd, DAEMON)
        .await
        .map_err(RotctldError::Io)?;
    match reply.as_deref().and_then(rprt_code) {
        Some(code) if code != 0 => Err(RotctldError::Rejected(code)),
        _ => Ok(()),
    }
}

/// Identyfikator rotora antenowego w konfiguracji wielourządzeniowej
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RotatorId(pub String);

impl RotatorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Polecenie nakierowania rotora na zadany azymut i elewację z jawnym RotatorId
#[derive(Debug, Clone, PartialEq)]
pub struct EmeRotorCommand {
    pub rotator_id: RotatorId,
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RotorError {
    InvalidRotatorId(String),
    InvalidAzimuth(f32),
    InvalidElevation(f32),
    NonFiniteCoordinate,
    Transport(String),
    Timeout,
}

impl std::fmt::Display for RotorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRotatorId(id) => write!(f, "Invalid or empty rotator ID: '{id}'"),
            Self::InvalidAzimuth(az) => write!(f, "Azimuth out of range [0.0, 360.0]: {az}"),
            Self::InvalidElevation(el) => write!(f, "Elevation out of range [-10.0, 90.0]: {el}"),
            Self::NonFiniteCoordinate => write!(f, "Coordinate cannot be NaN or Infinite"),
            Self::Transport(detail) => write!(f, "Rotor transport error: {detail}"),
            Self::Timeout => write!(f, "Rotor command timed out"),
        }
    }
}

impl std::error::Error for RotorError {}

impl EmeRotorCommand {
    pub fn new(
        rotator_id: RotatorId,
        azimuth_deg: f32,
        elevation_deg: f32,
    ) -> Result<Self, RotorError> {
        if rotator_id.as_str().trim().is_empty() {
            return Err(RotorError::InvalidRotatorId(rotator_id.0));
        }
        if !azimuth_deg.is_finite() || !elevation_deg.is_finite() {
            return Err(RotorError::NonFiniteCoordinate);
        }
        if !(0.0..=360.0).contains(&azimuth_deg) {
            return Err(RotorError::InvalidAzimuth(azimuth_deg));
        }
        if !(-10.0..=90.0).contains(&elevation_deg) {
            return Err(RotorError::InvalidElevation(elevation_deg));
        }
        Ok(Self {
            rotator_id,
            azimuth_deg,
            elevation_deg,
        })
    }
}

/// Trait transportu rotora dla mockowania i testów
pub trait RotorTransport: Send + Sync {
    fn send_command(&mut self, cmd: &EmeRotorCommand) -> Result<(), RotorError>;
}

/// Mock transport rotora
#[derive(Debug, Default)]
pub struct MockRotorTransport {
    pub sent_commands: Vec<EmeRotorCommand>,
    pub simulate_error: Option<String>,
    pub simulate_timeout: bool,
}

impl MockRotorTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RotorTransport for MockRotorTransport {
    fn send_command(&mut self, cmd: &EmeRotorCommand) -> Result<(), RotorError> {
        if self.simulate_timeout {
            return Err(RotorError::Timeout);
        }
        if let Some(ref err) = self.simulate_error {
            return Err(RotorError::Transport(err.clone()));
        }
        self.sent_commands.push(cmd.clone());
        Ok(())
    }
}

/// Stan położenia rotora antenowego
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotorState {
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub moving: bool,
    pub connected: bool,
}

impl Default for RotorState {
    fn default() -> Self {
        Self {
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            moving: false,
            connected: false,
        }
    }
}

/// Asynchroniczny klient do rotora antenowego przez protokół rotctld (Hamlib)
pub struct RotorClient {
    host: String,
    port: u16,
    state_sender: broadcast::Sender<RotorState>,
}

impl RotorClient {
    pub fn new(host: impl Into<String>, port: u16) -> (Self, broadcast::Receiver<RotorState>) {
        let (tx, rx) = broadcast::channel(32);
        (
            Self {
                host: host.into(),
                port,
                state_sender: tx,
            },
            rx,
        )
    }

    /// Pętla odpytywania aktualnego azymutu anteny w tle
    pub async fn run_poll_loop(&self, poll_interval_ms: u64) {
        let addr = format!("{}:{}", self.host, self.port);

        'poll: loop {
            if let Ok(stream) = connect_timeout(&addr).await {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);
                let mut current = RotorState {
                    connected: true,
                    ..Default::default()
                };

                while current.connected {
                    if write_cmd(&mut writer, b"p\n").await.is_err() {
                        break;
                    }

                    let mut az_line = String::new();
                    let mut el_line = String::new();

                    if read_line_timeout(&mut buf_reader, &mut az_line)
                        .await
                        .is_err()
                        || az_line.is_empty()
                        || az_line.len() > MAX_LINE_LEN
                    {
                        break;
                    }

                    if !az_line.trim().starts_with("RPRT") {
                        if read_line_timeout(&mut buf_reader, &mut el_line)
                            .await
                            .is_err()
                            || el_line.is_empty()
                            || el_line.len() > MAX_LINE_LEN
                        {
                            break;
                        }

                        if let (Ok(az), Ok(el)) =
                            (az_line.trim().parse::<f32>(), el_line.trim().parse::<f32>())
                        {
                            current.azimuth_deg = az;
                            current.elevation_deg = el;
                        }
                    }

                    if self.state_sender.send(current).is_err() {
                        break 'poll;
                    }
                    sleep(Duration::from_millis(poll_interval_ms)).await;
                }
            } else {
                let offline = RotorState {
                    connected: false,
                    ..Default::default()
                };
                if self.state_sender.send(offline).is_err() {
                    break 'poll;
                }
                sleep(Duration::from_secs(3)).await;
            }
        }
    }

    /// Obraca antenę na zadany azymut (i ewentualną elewację)
    pub async fn set_position(
        host: &str,
        port: u16,
        azimuth_deg: f32,
        elevation_deg: f32,
    ) -> Result<(), RotctldError> {
        let norm_az = (azimuth_deg % 360.0 + 360.0) % 360.0;
        let norm_el = elevation_deg.clamp(-10.0, 90.0);
        let cmd = format!("P {norm_az:.1} {norm_el:.1}\n");
        send_rotor_cmd(host, port, &cmd).await
    }

    /// Zatrzymuje ruch rotora
    pub async fn stop(host: &str, port: u16) -> Result<(), RotctldError> {
        send_rotor_cmd(host, port, "S\n").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jednorazowy serwer rotctld: odbiera jedną linię i odpowiada `reply` (pusty = milczy).
    async fn one_shot_rotctld(reply: &'static str) -> (u16, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut line = String::new();
            BufReader::new(reader).read_line(&mut line).await.unwrap();
            if !reply.is_empty() {
                writer.write_all(reply.as_bytes()).await.unwrap();
            }
            line
        });
        (port, handle)
    }

    #[test]
    fn rprt_code_parses_only_rotctld_reports() {
        assert_eq!(rprt_code("RPRT 0\n"), Some(0));
        assert_eq!(rprt_code("RPRT -8"), Some(-8));
        assert_eq!(rprt_code("180.000000"), None);
        assert_eq!(rprt_code(""), None);
    }

    #[tokio::test]
    async fn set_position_sends_the_normalised_command_and_accepts_rprt_zero() {
        let (port, server) = one_shot_rotctld("RPRT 0\n").await;
        RotorClient::set_position("127.0.0.1", port, -90.0, 100.0)
            .await
            .unwrap();
        assert_eq!(server.await.unwrap(), "P 270.0 90.0\n");
    }

    #[tokio::test]
    async fn a_negative_rprt_is_reported_as_rejected() {
        let (port, server) = one_shot_rotctld("RPRT -8\n").await;
        let err = RotorClient::stop("127.0.0.1", port).await.unwrap_err();
        assert!(matches!(err, RotctldError::Rejected(-8)), "{err:?}");
        assert_eq!(server.await.unwrap(), "S\n");
    }

    #[tokio::test]
    async fn an_unreachable_rotctld_is_an_io_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let err = RotorClient::stop("127.0.0.1", port).await.unwrap_err();
        assert!(matches!(err, RotctldError::Io(_)), "{err:?}");
    }

    #[test]
    fn eme_rotator_uses_explicit_rotator_id() {
        let id = RotatorId::new("eme_az_el_rotator");
        let cmd = EmeRotorCommand::new(id.clone(), 145.5, 32.0).unwrap();
        assert_eq!(cmd.rotator_id, id);
        assert_eq!(cmd.rotator_id.as_str(), "eme_az_el_rotator");
    }

    #[test]
    fn eme_rotator_sends_azimuth_and_elevation_once() {
        let mut transport = MockRotorTransport::new();
        let cmd = EmeRotorCommand::new(RotatorId::new("eme_rotator"), 180.0, 45.0).unwrap();

        transport.send_command(&cmd).unwrap();
        assert_eq!(transport.sent_commands.len(), 1);
        assert_eq!(transport.sent_commands[0].azimuth_deg, 180.0);
        assert_eq!(transport.sent_commands[0].elevation_deg, 45.0);
    }

    #[test]
    fn eme_rotator_rejects_invalid_azimuth() {
        let res_negative = EmeRotorCommand::new(RotatorId::new("rotor1"), -5.0, 20.0);
        assert_eq!(res_negative, Err(RotorError::InvalidAzimuth(-5.0)));

        let res_too_high = EmeRotorCommand::new(RotatorId::new("rotor1"), 365.0, 20.0);
        assert_eq!(res_too_high, Err(RotorError::InvalidAzimuth(365.0)));
    }

    #[test]
    fn eme_rotator_rejects_invalid_elevation() {
        let res_low = EmeRotorCommand::new(RotatorId::new("rotor1"), 120.0, -15.0);
        assert_eq!(res_low, Err(RotorError::InvalidElevation(-15.0)));

        let res_high = EmeRotorCommand::new(RotatorId::new("rotor1"), 120.0, 95.0);
        assert_eq!(res_high, Err(RotorError::InvalidElevation(95.0)));
    }

    #[test]
    fn eme_rotator_rejects_nan() {
        let res_nan_az = EmeRotorCommand::new(RotatorId::new("rotor1"), f32::NAN, 20.0);
        assert_eq!(res_nan_az, Err(RotorError::NonFiniteCoordinate));

        let res_nan_el = EmeRotorCommand::new(RotatorId::new("rotor1"), 120.0, f32::NAN);
        assert_eq!(res_nan_el, Err(RotorError::NonFiniteCoordinate));

        let res_inf = EmeRotorCommand::new(RotatorId::new("rotor1"), f32::INFINITY, 20.0);
        assert_eq!(res_inf, Err(RotorError::NonFiniteCoordinate));
    }

    #[test]
    fn eme_rotator_does_not_fallback_to_first_device() {
        // Pusty identyfikator rotora jest bezwzględnie odrzucany — brak cichego fallbacku na domyślne urządzenie lub radio
        let res = EmeRotorCommand::new(RotatorId::new(""), 120.0, 20.0);
        assert_eq!(res, Err(RotorError::InvalidRotatorId(String::new())));

        let res_whitespace = EmeRotorCommand::new(RotatorId::new("   "), 120.0, 20.0);
        assert_eq!(
            res_whitespace,
            Err(RotorError::InvalidRotatorId("   ".to_string()))
        );
    }

    #[test]
    fn eme_rotator_backend_error_is_visible() {
        let mut transport = MockRotorTransport::new();
        transport.simulate_error = Some("Rotor motor overload error".to_string());

        let cmd = EmeRotorCommand::new(RotatorId::new("rotor_eme"), 90.0, 10.0).unwrap();
        let res = transport.send_command(&cmd);

        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            RotorError::Transport("Rotor motor overload error".to_string())
        );
    }

    #[test]
    fn eme_rotator_timeout_is_bounded() {
        let mut transport = MockRotorTransport::new();
        transport.simulate_timeout = true;

        let cmd = EmeRotorCommand::new(RotatorId::new("rotor_eme"), 90.0, 10.0).unwrap();
        let start = std::time::Instant::now();
        let res = transport.send_command(&cmd);
        let elapsed = start.elapsed();

        assert_eq!(res, Err(RotorError::Timeout));
        assert!(elapsed < std::time::Duration::from_millis(50));
    }

    #[test]
    fn eme_rotator_test_uses_mock_transport_only() {
        let mut transport = MockRotorTransport::new();
        let cmd = EmeRotorCommand::new(RotatorId::new("isolated_mock"), 270.0, 60.0).unwrap();
        assert!(transport.send_command(&cmd).is_ok());
        assert_eq!(transport.sent_commands.len(), 1);
    }
}
