// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wysyłanie pakietu Magic Packet (Wake-on-LAN) do zdalnego załączania transceivera lub komputera

use std::net::{SocketAddr, UdpSocket};

/// Błędy podsystemu Wake-on-LAN (WOL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WolError {
    InvalidMacFormat(String),
    ParseByte { byte: String, detail: String },
    InvalidBroadcastAddress { address: String, detail: String },
    SocketBind(String),
    SetBroadcast(String),
    SendPacket(String),
}

impl std::fmt::Display for WolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMacFormat(mac) => write!(f, "Invalid MAC address format: {mac}"),
            Self::ParseByte { byte, detail } => {
                write!(f, "Failed to parse MAC byte '{byte}': {detail}")
            }
            Self::InvalidBroadcastAddress { address, detail } => {
                write!(f, "Invalid broadcast address '{address}': {detail}")
            }
            Self::SocketBind(detail) => write!(f, "Failed to bind UDP socket: {detail}"),
            Self::SetBroadcast(detail) => write!(f, "Failed to set broadcast flag: {detail}"),
            Self::SendPacket(detail) => write!(f, "Failed to send WOL packet: {detail}"),
        }
    }
}

impl std::error::Error for WolError {}

pub struct WolClient;

impl WolClient {
    /// Parsuje adres MAC z ciągów znaków: "AA:BB:CC:DD:EE:FF", "AA-BB-CC-DD-EE-FF" lub "AABBCCDDEEFF"
    pub fn parse_mac(mac_str: &str) -> Result<[u8; 6], WolError> {
        let trimmed = mac_str.trim();

        let is_contiguous_12 =
            trimmed.len() == 12 && trimmed.chars().all(|c| c.is_ascii_hexdigit());

        let is_separated_pairs = |sep: char| -> bool {
            let parts: Vec<&str> = trimmed.split(sep).collect();
            parts.len() == 6
                && parts
                    .iter()
                    .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
        };

        if !is_contiguous_12 && !is_separated_pairs(':') && !is_separated_pairs('-') {
            return Err(WolError::InvalidMacFormat(mac_str.to_string()));
        }

        let clean: String = trimmed.chars().filter(char::is_ascii_hexdigit).collect();
        let mut mac = [0u8; 6];
        for i in 0..6 {
            let byte_str = &clean[i * 2..i * 2 + 2];
            mac[i] = u8::from_str_radix(byte_str, 16).map_err(|e| WolError::ParseByte {
                byte: byte_str.to_string(),
                detail: e.to_string(),
            })?;
        }

        Ok(mac)
    }

    /// Tworzy binarny pakiet Magic Packet (6x 0xFF + 16x MAC)
    pub fn build_magic_packet(mac: &[u8; 6]) -> [u8; 102] {
        let mut packet = [0u8; 102];
        packet[..6].fill(0xFF);
        for rep in 0..16 {
            let offset = 6 + rep * 6;
            packet[offset..offset + 6].copy_from_slice(mac);
        }
        packet
    }

    /// Wysyła pakiet Wake-on-LAN na wskazany adres rozgłoszeniowy (np. "255.255.255.255:9")
    pub fn send_wol(mac_str: &str, target_broadcast: Option<&str>) -> Result<(), WolError> {
        let mac = Self::parse_mac(mac_str)?;
        let packet = Self::build_magic_packet(&mac);

        let dest = target_broadcast.unwrap_or("255.255.255.255:9");
        let dest_addr: SocketAddr = dest
            .parse()
            .map_err(|e| WolError::InvalidBroadcastAddress {
                address: dest.to_string(),
                detail: format!("{e}"),
            })?;

        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| WolError::SocketBind(e.to_string()))?;
        socket
            .set_broadcast(true)
            .map_err(|e| WolError::SetBroadcast(e.to_string()))?;

        socket
            .send_to(&packet, dest_addr)
            .map_err(|e| WolError::SendPacket(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wol_magic_packet() {
        let mac_str = "00:11:22:33:44:55";
        let mac = WolClient::parse_mac(mac_str).unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        let packet = WolClient::build_magic_packet(&mac);
        assert_eq!(packet.len(), 102);
        assert_eq!(&packet[0..6], &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(&packet[6..12], &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert_eq!(&packet[96..102], &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_formats_and_rejects_invalid() {
        assert_eq!(
            WolClient::parse_mac("AA-BB-CC-DD-EE-FF").unwrap(),
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
        );
        assert_eq!(
            WolClient::parse_mac("  aabbccddeeff  ").unwrap(),
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
        );
        assert!(WolClient::parse_mac("AA:BB-CC:DD-EE-FF").is_err());
        assert!(WolClient::parse_mac("A:BB:CC:DD:EE:FFF").is_err());
        assert!(WolClient::parse_mac("random text 001122334455 here").is_err());
    }
}
