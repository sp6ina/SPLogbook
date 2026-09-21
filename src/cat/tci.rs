// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Transceiver Control Interface (TCI) Protocol for SDR Radios (SunSDR / ExpertSDR / Thetis)

#[derive(Debug, Clone, PartialEq)]
pub enum TciMessage {
    Vfo { receiver: u8, vfo: u8, freq_hz: u64 },
    Modulation { receiver: u8, mode: String },
    Trx { receiver: u8, transmitting: bool },
    Spot { callsign: String, freq_hz: u64, color: u32, text: String },
    CwMacro { speed_wpm: u8, text: String },
    Unknown(String),
}

pub struct TciProtocol;

impl TciProtocol {
    /// Formatuje komendę ustawienia VFO (częstotliwości w Hz)
    pub fn cmd_set_vfo(receiver: u8, vfo: u8, freq_hz: u64) -> String {
        format!("vfo:{},{},{};", receiver, vfo, freq_hz)
    }

    /// Formatuje komendę zmiany modulacji (USB, LSB, CW, DIGI_U, etc.)
    pub fn cmd_set_mode(receiver: u8, mode: &str) -> String {
        format!("modulation:{},{};", receiver, mode.to_uppercase())
    }

    /// Formatuje komendę PTT (nadawanie / odbiór)
    pub fn cmd_set_trx(receiver: u8, enable_tx: bool) -> String {
        format!("trx:{},{};", receiver, enable_tx)
    }

    /// Formatuje komendę naniesienia spotu DX na wodospad SDR
    pub fn cmd_add_spot(callsign: &str, freq_hz: u64, color_argb: u32, text: &str) -> String {
        format!("spot:{},{},{},{};", callsign.trim().to_uppercase(), freq_hz, color_argb, text.trim())
    }

    /// Formatuje komendę nadania tekstu CW
    pub fn cmd_send_cw(speed_wpm: u8, text: &str) -> String {
        format!("cw_macros:{},{};", speed_wpm, text)
    }

    /// Parsuje pojedynczą odpowiedź lub zdarzenie TCI
    pub fn parse_message(raw: &str) -> TciMessage {
        let trimmed = raw.trim().trim_end_matches(';');
        if let Some(rest) = trimmed.strip_prefix("vfo:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() >= 3 {
                let receiver = parts[0].parse::<u8>().unwrap_or(0);
                let vfo = parts[1].parse::<u8>().unwrap_or(0);
                let freq_hz = parts[2].parse::<u64>().unwrap_or(0);
                return TciMessage::Vfo { receiver, vfo, freq_hz };
            }
        } else if let Some(rest) = trimmed.strip_prefix("modulation:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() >= 2 {
                let receiver = parts[0].parse::<u8>().unwrap_or(0);
                let mode = parts[1].to_string();
                return TciMessage::Modulation { receiver, mode };
            }
        } else if let Some(rest) = trimmed.strip_prefix("trx:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() >= 2 {
                let receiver = parts[0].parse::<u8>().unwrap_or(0);
                let transmitting = parts[1].trim().eq_ignore_ascii_case("true");
                return TciMessage::Trx { receiver, transmitting };
            }
        } else if let Some(rest) = trimmed.strip_prefix("spot:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() >= 4 {
                let callsign = parts[0].to_string();
                let freq_hz = parts[1].parse::<u64>().unwrap_or(0);
                let color = parts[2].parse::<u32>().unwrap_or(0xFF00FF00);
                let text = parts[3].to_string();
                return TciMessage::Spot { callsign, freq_hz, color, text };
            }
        }
        TciMessage::Unknown(raw.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tci_formatters() {
        assert_eq!(TciProtocol::cmd_set_vfo(0, 0, 14074000), "vfo:0,0,14074000;");
        assert_eq!(TciProtocol::cmd_set_mode(0, "usb"), "modulation:0,USB;");
        assert_eq!(TciProtocol::cmd_set_trx(0, true), "trx:0,true;");
        assert_eq!(
            TciProtocol::cmd_add_spot("SP6INA", 14025000, 0xFFFF0000, "599 SP"),
            "spot:SP6INA,14025000,4294901760,599 SP;"
        );
    }

    #[test]
    fn test_tci_parser() {
        let msg = TciProtocol::parse_message("vfo:0,0,7074000;");
        assert_eq!(msg, TciMessage::Vfo { receiver: 0, vfo: 0, freq_hz: 7074000 });

        let msg_mod = TciProtocol::parse_message("modulation:0,CW;");
        assert_eq!(msg_mod, TciMessage::Modulation { receiver: 0, mode: "CW".to_string() });

        let msg_trx = TciProtocol::parse_message("trx:0,true;");
        assert_eq!(msg_trx, TciMessage::Trx { receiver: 0, transmitting: true });
    }
}
