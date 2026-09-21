// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wysyłanie pakietu Magic Packet (Wake-on-LAN) do zdalnego załączania transceivera lub komputera

use std::net::{SocketAddr, UdpSocket};

pub struct WolClient;

impl WolClient {
    /// Parsuje adres MAC z ciągów znaków: "AA:BB:CC:DD:EE:FF", "AA-BB-CC-DD-EE-FF" lub "AABBCCDDEEFF"
    pub fn parse_mac(mac_str: &str) -> Result<[u8; 6], String> {
        let clean: String = mac_str
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();

        if clean.len() != 12 {
            return Err(format!("Nieprawidłowa długość adresu MAC: {}", mac_str));
        }

        let mut mac = [0u8; 6];
        for i in 0..6 {
            let byte_str = &clean[i * 2..i * 2 + 2];
            mac[i] = u8::from_str_radix(byte_str, 16)
                .map_err(|e| format!("Błąd parsowania bajtu MAC {}: {}", byte_str, e))?;
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
    pub fn send_wol(mac_str: &str, target_broadcast: Option<&str>) -> Result<(), String> {
        let mac = Self::parse_mac(mac_str)?;
        let packet = Self::build_magic_packet(&mac);

        let dest = target_broadcast.unwrap_or("255.255.255.255:9");
        let dest_addr: SocketAddr = dest
            .parse()
            .map_err(|e| format!("Nieprawidłowy adres rozgłoszeniowy {}: {}", dest, e))?;

        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("Błąd bindowania gniazda UDP: {}", e))?;
        socket
            .set_broadcast(true)
            .map_err(|e| format!("Błąd ustawiania flagi broadcast: {}", e))?;

        socket
            .send_to(&packet, dest_addr)
            .map_err(|e| format!("Błąd wysyłania pakietu WOL: {}", e))?;

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
}
