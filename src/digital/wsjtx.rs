// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

const WSJTX_MAGIC: u32 = 0xadbccbda;

/// Pakiet zdekodowany z WSJT-X / JTDX
#[derive(Debug, Clone)]
pub enum WsjtxMessage {
    Heartbeat { id: String },
    Status { id: String, dial_freq: u64, mode: String, dx_call: String, report: String },
    QsoLogged(Box<QsoRecord>),
}

/// Serwer nasłuchujący pakietów UDP ze stacji WSJT-X / JTDX
pub struct WsjtxReceiver {
    port: u16,
}

impl WsjtxReceiver {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Uruchamia asynchroniczny nasłuch na gnieździe UDP
    pub async fn run(&self, sender: mpsc::Sender<WsjtxMessage>) -> Result<(), std::io::Error> {
        let socket = match UdpSocket::bind(format!("0.0.0.0:{}", self.port)).await {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Nie można powiązać portu UDP {}: {}", self.port, e);
                return Err(e);
            }
        };
        let mut buf = vec![0u8; 8192];

        loop {
            if let Ok((len, _)) = socket.recv_from(&mut buf).await {
                if let Some(msg) = Self::parse_packet(&buf[..len]) {
                    let _ = sender.send(msg).await;
                }
            }
        }
    }

    /// Pomija strukturę QDateTime ze strumienia Qt QDataStream
    /// QDate (8 bajtów) + QTime (4 bajty) + timespec (1 bajt) [+ 4 bajty offsetu gdy timespec==2]
    fn skip_qdatetime(rdr: &mut Cursor<&[u8]>) -> Option<()> {
        use byteorder::ReadBytesExt;
        let _ = rdr.read_u64::<BigEndian>().ok()?;
        let _ = rdr.read_u32::<BigEndian>().ok()?;
        let timespec = rdr.read_u8().ok()?;
        if timespec == 2 {
            let _ = rdr.read_i32::<BigEndian>().ok()?;
        }
        Some(())
    }

    /// Dekoduje binarny pakiet Qt QDataStream z WSJT-X
    pub fn parse_packet(data: &[u8]) -> Option<WsjtxMessage> {
        let mut rdr = Cursor::new(data);
        let magic = rdr.read_u32::<BigEndian>().ok()?;
        if magic != WSJTX_MAGIC {
            return None;
        }

        let _schema = rdr.read_u32::<BigEndian>().ok()?;
        let packet_type = rdr.read_u32::<BigEndian>().ok()?;
        let id = Self::read_utf8_string(&mut rdr)?;

        match packet_type {
            0 => Some(WsjtxMessage::Heartbeat { id }),
            1 => {
                // Status packet
                let dial_freq = rdr.read_u64::<BigEndian>().unwrap_or(0);
                let mode = Self::read_utf8_string(&mut rdr).unwrap_or_default();
                let dx_call = Self::read_utf8_string(&mut rdr).unwrap_or_default();
                let report = Self::read_utf8_string(&mut rdr).unwrap_or_default();

                Some(WsjtxMessage::Status {
                    id,
                    dial_freq,
                    mode,
                    dx_call,
                    report,
                })
            }
            5 => {
                // QSO Logged packet
                // Prawidłowo pomiń date/time off (QDateTime: 8b QDate + 4b QTime + 1b timespec)
                Self::skip_qdatetime(&mut rdr)?;
                let dx_call = Self::read_utf8_string(&mut rdr)?;
                let dx_grid = Self::read_utf8_string(&mut rdr).unwrap_or_default();
                let dial_freq = rdr.read_u64::<BigEndian>().unwrap_or(0);
                let mode = Self::read_utf8_string(&mut rdr).unwrap_or_else(|| "FT8".to_string());
                let rst_sent = Self::read_utf8_string(&mut rdr).unwrap_or_else(|| "-10".to_string());
                let rst_rcvd = Self::read_utf8_string(&mut rdr).unwrap_or_else(|| "-10".to_string());
                let _tx_power = Self::read_utf8_string(&mut rdr);
                let comments = Self::read_utf8_string(&mut rdr);
                let name = Self::read_utf8_string(&mut rdr);

                let freq_mhz = (dial_freq as f64) / 1_000_000.0;
                let band = Self::freq_to_band(freq_mhz);

                let mut qso = QsoRecord::new(dx_call, band, mode);
                qso.freq = Some(freq_mhz);
                qso.rst_sent = rst_sent;
                qso.rst_rcvd = rst_rcvd;
                if !dx_grid.is_empty() {
                    qso.gridsquare = Some(dx_grid);
                }
                if let Some(n) = name {
                    if !n.is_empty() {
                        qso.name = Some(n);
                    }
                }
                if let Some(c) = comments {
                    if !c.is_empty() {
                        qso.comment = Some(c);
                    }
                }

                Some(WsjtxMessage::QsoLogged(Box::new(qso)))
            }
            12 => {
                // Logged ADIF packet (WSJT-X 2.7+ i 3.0+)
                let adif_text = Self::read_utf8_string(&mut rdr)?;
                let qsos = crate::core::adif::AdifEngine::parse_reader(std::io::Cursor::new(adif_text));
                if let Some(qso) = qsos.into_iter().next() {
                    return Some(WsjtxMessage::QsoLogged(Box::new(qso)));
                }
                None
            }
            _ => None,
        }
    }

    fn read_utf8_string(rdr: &mut Cursor<&[u8]>) -> Option<String> {
        let len = rdr.read_u32::<BigEndian>().ok()?;
        if len == 0xffffffff {
            return None; // Null string w Qt
        }
        let len = len as usize;
        // Zabezpieczenie przed atakiem DoS / OOM przy zniekształconym pakiecie UDP
        if len > 4096 {
            return None;
        }
        let mut buf = vec![0u8; len];
        std::io::Read::read_exact(rdr, &mut buf).ok()?;
        String::from_utf8(buf).ok()
    }

    fn freq_to_band(mhz: f64) -> String {
        if (1.8..=2.0).contains(&mhz) {
            "160m".to_string()
        } else if (3.5..=3.8).contains(&mhz) {
            "80m".to_string()
        } else if (5.25..=5.45).contains(&mhz) {
            "60m".to_string()
        } else if (7.0..=7.3).contains(&mhz) {
            "40m".to_string()
        } else if (10.1..=10.15).contains(&mhz) {
            "30m".to_string()
        } else if (14.0..=14.35).contains(&mhz) {
            "20m".to_string()
        } else if (18.068..=18.168).contains(&mhz) {
            "17m".to_string()
        } else if (21.0..=21.45).contains(&mhz) {
            "15m".to_string()
        } else if (24.89..=24.99).contains(&mhz) {
            "12m".to_string()
        } else if (28.0..=29.7).contains(&mhz) {
            "10m".to_string()
        } else if (50.0..=54.0).contains(&mhz) {
            "6m".to_string()
        } else if (69.9..=70.5).contains(&mhz) {
            "4m".to_string()
        } else if (144.0..=148.0).contains(&mhz) {
            "2m".to_string()
        } else if (430.0..=440.0).contains(&mhz) {
            "70cm".to_string()
        } else if (1240.0..=1300.0).contains(&mhz) {
            "23cm".to_string()
        } else {
            "OTHER".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};

    #[test]
    fn test_parse_wsjtx_type_5_qso_logged() {
        let mut packet = Vec::new();
        // Magic
        packet.write_u32::<BigEndian>(WSJTX_MAGIC).unwrap();
        // Schema
        packet.write_u32::<BigEndian>(2).unwrap();
        // Type: 5 (QSO Logged)
        packet.write_u32::<BigEndian>(5).unwrap();
        // Id: "WSJT-X"
        let id = "WSJT-X";
        packet.write_u32::<BigEndian>(id.len() as u32).unwrap();
        packet.extend_from_slice(id.as_bytes());

        // QDateTime: QDate (8 bytes) + QTime (4 bytes) + timespec (1 byte)
        packet.write_u64::<BigEndian>(2460000).unwrap(); // Julian day
        packet.write_u32::<BigEndian>(43200000).unwrap(); // 12:00:00.000 ms
        packet.write_u8(1).unwrap(); // UTC timespec

        // dx_call: "K1ABC"
        let dx_call = "K1ABC";
        packet.write_u32::<BigEndian>(dx_call.len() as u32).unwrap();
        packet.extend_from_slice(dx_call.as_bytes());

        // dx_grid: "FN31pr"
        let dx_grid = "FN31pr";
        packet.write_u32::<BigEndian>(dx_grid.len() as u32).unwrap();
        packet.extend_from_slice(dx_grid.as_bytes());

        // dial_freq: 14074000
        packet.write_u64::<BigEndian>(14_074_000).unwrap();

        // mode: "FT8"
        let mode = "FT8";
        packet.write_u32::<BigEndian>(mode.len() as u32).unwrap();
        packet.extend_from_slice(mode.as_bytes());

        // rst_sent: "-05"
        let rst_sent = "-05";
        packet.write_u32::<BigEndian>(rst_sent.len() as u32).unwrap();
        packet.extend_from_slice(rst_sent.as_bytes());

        // rst_rcvd: "-12"
        let rst_rcvd = "-12";
        packet.write_u32::<BigEndian>(rst_rcvd.len() as u32).unwrap();
        packet.extend_from_slice(rst_rcvd.as_bytes());

        let msg = WsjtxReceiver::parse_packet(&packet);
        assert!(msg.is_some(), "Pakiet Type 5 powinien zostać poprawnie zdekodowany");

        match msg {
            Some(WsjtxMessage::QsoLogged(qso)) => {
                assert_eq!(qso.callsign, "K1ABC");
                assert_eq!(qso.gridsquare, Some("FN31pr".to_string()));
                assert_eq!(qso.band, "20m");
                assert_eq!(qso.mode, "FT8");
                assert_eq!(qso.rst_sent, "-05");
                assert_eq!(qso.rst_rcvd, "-12");
            }
            _ => assert!(false, "Oczekiwano WsjtxMessage::QsoLogged"),
        }
    }
}

