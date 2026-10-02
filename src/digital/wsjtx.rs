// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::qso::QsoRecord;
use std::io::{Cursor, Read};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

const WSJTX_MAGIC: u32 = 0xadbc_cbda;

fn read_u8(rdr: &mut Cursor<&[u8]>) -> Option<u8> {
    let mut buf = [0u8; 1];
    rdr.read_exact(&mut buf).ok()?;
    Some(buf[0])
}

fn read_u32_be(rdr: &mut Cursor<&[u8]>) -> Option<u32> {
    let mut buf = [0u8; 4];
    rdr.read_exact(&mut buf).ok()?;
    Some(u32::from_be_bytes(buf))
}

fn read_i32_be(rdr: &mut Cursor<&[u8]>) -> Option<i32> {
    let mut buf = [0u8; 4];
    rdr.read_exact(&mut buf).ok()?;
    Some(i32::from_be_bytes(buf))
}

fn read_u64_be(rdr: &mut Cursor<&[u8]>) -> Option<u64> {
    let mut buf = [0u8; 8];
    rdr.read_exact(&mut buf).ok()?;
    Some(u64::from_be_bytes(buf))
}

fn read_i64_be(rdr: &mut Cursor<&[u8]>) -> Option<i64> {
    let mut buf = [0u8; 8];
    rdr.read_exact(&mut buf).ok()?;
    Some(i64::from_be_bytes(buf))
}

/// Pakiet zdekodowany z WSJT-X / JTDX
#[derive(Debug, Clone)]
pub enum WsjtxMessage {
    Heartbeat {
        id: String,
    },
    Status {
        id: String,
        dial_freq: u64,
        mode: String,
        dx_call: String,
        report: String,
    },
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
        let mut last_logged_sig: Option<(String, std::time::Instant)> = None;

        loop {
            let (len, _) = match socket.recv_from(&mut buf).await {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("WSJT-X: błąd odbioru UDP: {e}");
                    break;
                }
            };
            if let Some(msg) = Self::parse_packet(&buf[..len]) {
                // WSJT-X wysyła bezpośrednio po sobie pakiet Type 5 (QSO Logged)
                // oraz Type 12 (Logged ADIF) dla tej samej łączności.
                if let WsjtxMessage::QsoLogged(ref qso) = msg {
                    let hhmm: String = qso
                        .time_on
                        .chars()
                        .filter(char::is_ascii_digit)
                        .take(4)
                        .collect();
                    let date_norm: String = qso
                        .qso_date
                        .chars()
                        .filter(char::is_ascii_digit)
                        .take(8)
                        .collect();
                    let sig = format!(
                        "{}|{}|{}|{}|{}",
                        qso.callsign.trim().to_uppercase(),
                        qso.band.trim().to_lowercase(),
                        qso.mode.trim().to_uppercase(),
                        date_norm,
                        hhmm
                    );
                    if let Some((ref prev_sig, prev_ts)) = last_logged_sig
                        && prev_sig == &sig
                        && prev_ts.elapsed() < std::time::Duration::from_secs(5)
                    {
                        log::debug!("WSJT-X: pominięto zduplikowany pakiet QSO ({sig})");
                        continue;
                    }
                    last_logged_sig = Some((sig, std::time::Instant::now()));
                }
                // Zamknięcie kanału przez konsumenta (shutdown aplikacji) kończy nasłuch.
                if sender.send(msg).await.is_err() {
                    log::debug!("WSJT-X: kanał odbiorczy zamknięty, kończę nasłuch.");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Dekoduje strukturę QDateTime ze strumienia Qt QDataStream.
    /// Zwraca `(YYYYMMDD, HHMMSS)` w UTC zgodnie ze standardem ADIF i bazą SQLite.
    fn read_qdatetime(rdr: &mut Cursor<&[u8]>) -> Option<(String, String)> {
        let julian_day = read_i64_be(rdr)?;
        let ms_since_midnight = read_u32_be(rdr)?;
        let timespec = read_u8(rdr)?;
        let utc_offset_secs: i64 = match timespec {
            2 => i64::from(read_i32_be(rdr)?),
            3 => {
                // Qt::TimeZone — w strumieniu znajduje się QByteArray z identyfikatorem IANA
                let _ = Self::read_utf8_string(rdr);
                0
            }
            _ => 0,
        };
        if julian_day <= 0 || ms_since_midnight >= 86_400_000 {
            return None;
        }

        // Julian Day Number (JDN) 2440588 to 1970-01-01 (Unix Epoch)
        let days_from_epoch = julian_day.checked_sub(2_440_588)?;
        let secs = days_from_epoch
            .checked_mul(86400)?
            .checked_add(i64::from(ms_since_midnight / 1000))?
            .checked_sub(utc_offset_secs)?;
        let dt = chrono::DateTime::from_timestamp(secs, 0)?;
        let date_str = dt.format("%Y%m%d").to_string();
        let time_str = dt.format("%H%M%S").to_string();
        Some((date_str, time_str))
    }

    /// Dekoduje binarny pakiet Qt QDataStream z WSJT-X
    pub fn parse_packet(data: &[u8]) -> Option<WsjtxMessage> {
        let mut rdr = Cursor::new(data);
        let magic = read_u32_be(&mut rdr)?;
        if magic != WSJTX_MAGIC {
            return None;
        }

        let _schema = read_u32_be(&mut rdr)?;
        let packet_type = read_u32_be(&mut rdr)?;
        let id = Self::read_utf8_string(&mut rdr)?;

        match packet_type {
            0 => Some(WsjtxMessage::Heartbeat { id }),
            1 => {
                // Status packet
                let dial_freq = read_u64_be(&mut rdr).unwrap_or(0);
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
                // QSO Logged packet (WSJT-X NetworkMessage Type 5)
                let (date_off, time_off) = Self::read_qdatetime(&mut rdr)?;
                let dx_call = Self::read_utf8_string(&mut rdr)?;
                let dx_grid = Self::read_utf8_string(&mut rdr).unwrap_or_default();
                let dial_freq = read_u64_be(&mut rdr).unwrap_or(0);
                let mode = Self::read_utf8_string(&mut rdr).unwrap_or_else(|| "FT8".to_string());
                let rst_sent =
                    Self::read_utf8_string(&mut rdr).unwrap_or_else(|| "-10".to_string());
                let rst_rcvd =
                    Self::read_utf8_string(&mut rdr).unwrap_or_else(|| "-10".to_string());
                let _tx_power = Self::read_utf8_string(&mut rdr);
                let comments = Self::read_utf8_string(&mut rdr);
                let name = Self::read_utf8_string(&mut rdr);
                // Pola 11..17 specyfikacji WSJT-X UDP (opcjonalne dla krótszych ramek testowych)
                let dt_on = Self::read_qdatetime(&mut rdr);
                let _operator_call = Self::read_utf8_string(&mut rdr);
                let _my_call = Self::read_utf8_string(&mut rdr);
                let my_grid = Self::read_utf8_string(&mut rdr);
                let exch_sent = Self::read_utf8_string(&mut rdr);
                let exch_rcvd = Self::read_utf8_string(&mut rdr);
                let prop_mode = Self::read_utf8_string(&mut rdr);

                let freq_mhz = (dial_freq as f64) / 1_000_000.0;
                let band = Self::freq_to_band(dial_freq);

                let (qso_date, time_on) =
                    dt_on.unwrap_or_else(|| (date_off.clone(), time_off.clone()));

                let mut qso = QsoRecord::new(dx_call, band, mode);
                qso.qso_date = qso_date;
                qso.time_on = time_on;
                qso.time_off = Some(time_off);
                qso.freq = Some(freq_mhz);
                qso.rst_sent = rst_sent;
                qso.rst_rcvd = rst_rcvd;
                if !dx_grid.is_empty() {
                    qso.gridsquare = Some(dx_grid);
                }
                if let Some(n) = name.filter(|s| !s.is_empty()) {
                    qso.name = Some(n);
                }
                if let Some(c) = comments.filter(|s| !s.is_empty()) {
                    qso.comment = Some(c);
                }
                if let Some(mg) = my_grid.filter(|s| !s.is_empty()) {
                    qso.my_gridsquare = Some(mg);
                }
                if let Some(es) = exch_sent.filter(|s| !s.is_empty()) {
                    qso.stx_string = Some(es);
                }
                if let Some(er) = exch_rcvd.filter(|s| !s.is_empty()) {
                    qso.srx_string = Some(er);
                }
                if let Some(pm) = prop_mode.filter(|s| !s.is_empty()) {
                    qso.prop_mode = Some(pm);
                }

                Some(WsjtxMessage::QsoLogged(Box::new(qso)))
            }
            12 => {
                // Logged ADIF packet (WSJT-X 2.7+ i 3.0+)
                let adif_text = Self::read_utf8_string(&mut rdr)?;
                let qsos =
                    crate::core::adif::AdifEngine::parse_reader(std::io::Cursor::new(adif_text));
                if let Some(qso) = qsos.into_iter().next() {
                    return Some(WsjtxMessage::QsoLogged(Box::new(qso)));
                }
                None
            }
            _ => None,
        }
    }

    fn read_utf8_string(rdr: &mut Cursor<&[u8]>) -> Option<String> {
        let len = read_u32_be(rdr)?;
        if len == 0xffff_ffff {
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

    fn freq_to_band(freq_hz: u64) -> String {
        crate::core::bandplan::get_band_by_freq(freq_hz)
            .map_or_else(|| "OTHER".to_string(), |b| b.name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_utf8(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u32).to_be_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn test_parse_wsjtx_type_5_qso_logged() {
        let mut packet = Vec::new();
        // Magic
        packet.extend_from_slice(&WSJTX_MAGIC.to_be_bytes());
        // Schema
        packet.extend_from_slice(&2u32.to_be_bytes());
        // Type: 5 (QSO Logged)
        packet.extend_from_slice(&5u32.to_be_bytes());
        // Id: "WSJT-X"
        write_utf8(&mut packet, "WSJT-X");

        // QDateTime Off: QDate (8 bytes) + QTime (4 bytes) + timespec (1 byte)
        packet.extend_from_slice(&2_460_000u64.to_be_bytes()); // Julian day (2023-02-24)
        packet.extend_from_slice(&43_260_000u32.to_be_bytes()); // 12:01:00.000 ms
        packet.push(1); // UTC timespec

        write_utf8(&mut packet, "K1ABC");
        write_utf8(&mut packet, "FN31pr");
        packet.extend_from_slice(&14_074_000u64.to_be_bytes());
        write_utf8(&mut packet, "FT8");
        write_utf8(&mut packet, "-05");
        write_utf8(&mut packet, "-12");
        write_utf8(&mut packet, "50");
        write_utf8(&mut packet, "73 TU");
        write_utf8(&mut packet, "John");

        // QDateTime On (pole 11): 12:00:00 UTC
        packet.extend_from_slice(&2_460_000u64.to_be_bytes());
        packet.extend_from_slice(&43_200_000u32.to_be_bytes());
        packet.push(1);

        // Pola 12..17: Operator, MyCall, MyGrid, ExchSent, ExchRcvd, PropMode
        write_utf8(&mut packet, "SP6INA");
        write_utf8(&mut packet, "SP6INA");
        write_utf8(&mut packet, "JO81WA");
        write_utf8(&mut packet, "599 15");
        write_utf8(&mut packet, "599 05");
        write_utf8(&mut packet, "F2");

        let msg = WsjtxReceiver::parse_packet(&packet);
        assert!(
            msg.is_some(),
            "Pakiet Type 5 powinien zostać poprawnie zdekodowany"
        );

        match msg {
            Some(WsjtxMessage::QsoLogged(qso)) => {
                assert_eq!(qso.callsign, "K1ABC");
                assert_eq!(qso.gridsquare, Some("FN31pr".to_string()));
                assert_eq!(qso.band, "20m");
                assert_eq!(qso.mode, "FT8");
                assert_eq!(qso.rst_sent, "-05");
                assert_eq!(qso.rst_rcvd, "-12");
                assert_eq!(qso.qso_date, "20230224");
                assert_eq!(qso.time_on, "120000");
                assert_eq!(qso.time_off, Some("120100".to_string()));
                assert_eq!(qso.name.as_deref(), Some("John"));
                assert_eq!(qso.comment.as_deref(), Some("73 TU"));
                assert_eq!(qso.my_gridsquare.as_deref(), Some("JO81WA"));
                assert_eq!(qso.stx_string.as_deref(), Some("599 15"));
                assert_eq!(qso.srx_string.as_deref(), Some("599 05"));
                assert_eq!(qso.prop_mode.as_deref(), Some("F2"));
            }
            _ => panic!("Oczekiwano WsjtxMessage::QsoLogged"),
        }
    }
}
