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

async fn send_rotor_cmd(host: &str, port: u16, cmd: &str) -> Result<(), std::io::Error> {
    tcp::send_cmd_ignore_reply(&format!("{host}:{port}"), cmd, DAEMON).await
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
    ) -> Result<(), std::io::Error> {
        let norm_az = (azimuth_deg % 360.0 + 360.0) % 360.0;
        let norm_el = elevation_deg.clamp(-10.0, 90.0);
        let cmd = format!("P {norm_az:.1} {norm_el:.1}\n");
        send_rotor_cmd(host, port, &cmd).await
    }

    /// Zatrzymuje ruch rotora
    pub async fn stop(host: &str, port: u16) -> Result<(), std::io::Error> {
        send_rotor_cmd(host, port, "S\n").await
    }
}
