// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::time::sleep;

/// Maksymalna długość pojedynczej linii odpowiedzi rotctld (azymut/elevacja).
const MAX_LINE_LEN: usize = 256;

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

        loop {
            if let Ok(stream) = TcpStream::connect(&addr).await {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);
                let mut current = RotorState {
                    connected: true,
                    ..Default::default()
                };

                while current.connected {
                    if writer.write_all(b"p\n").await.is_err() {
                        break;
                    }

                    let mut az_line = String::new();
                    let mut el_line = String::new();

                    let mut limited = (&mut buf_reader).take((MAX_LINE_LEN + 1) as u64);
                    if limited.read_line(&mut az_line).await.is_err()
                        || az_line.is_empty()
                        || az_line.len() > MAX_LINE_LEN
                    {
                        break;
                    }
                    let mut limited = (&mut buf_reader).take((MAX_LINE_LEN + 1) as u64);
                    if limited.read_line(&mut el_line).await.is_err()
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

                    let _ = self.state_sender.send(current);
                    sleep(Duration::from_millis(poll_interval_ms)).await;
                }
            } else {
                let offline = RotorState {
                    connected: false,
                    ..Default::default()
                };
                let _ = self.state_sender.send(offline);
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
        let mut stream = TcpStream::connect(format!("{host}:{port}")).await?;
        let cmd = format!("P {norm_az:.1} {norm_el:.1}\n");
        stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    /// Zatrzymuje ruch rotora
    pub async fn stop(host: &str, port: u16) -> Result<(), std::io::Error> {
        let mut stream = TcpStream::connect(format!("{host}:{port}")).await?;
        stream.write_all(b"S\n").await?;
        Ok(())
    }
}
