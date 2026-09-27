// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Pomocnicze funkcje protokołu Icom CI-V (multi-radio): ramki, adresy, BCD częstotliwości.

/// Preambuła, znacznik końca ramki i domyślne adresy protokołu CI-V.
pub const CI_V_PREAMBLE: [u8; 2] = [0xFE, 0xFE];
pub const CI_V_END: u8 = 0xFD;
pub const CI_V_CONTROLLER: u8 = 0xE0;

/// Komendy CI-V używane do odczytu/zapisu podstawowego stanu radia.
pub mod command {
    pub const SET_FREQUENCY: u8 = 0x05;
    pub const READ_FREQUENCY: u8 = 0x03;
    pub const READ_MODE: u8 = 0x04;
    pub const SET_MODE: u8 = 0x06;
    pub const SET_PTT: u8 = 0x1C;
    pub const READ_PTT: u8 = 0x1C;
}

pub struct CiV;

impl CiV {
    /// Buduje ramkę CI-V: `FE FE <to> <from> <cmd> [sub] [data..] FD`.
    pub fn build_frame(to_addr: u8, from_addr: u8, cmd: u8, sub: u8, data: &[u8]) -> Vec<u8> {
        let mut f = Vec::with_capacity(6 + data.len());
        f.extend_from_slice(&CI_V_PREAMBLE);
        f.push(to_addr);
        f.push(from_addr);
        f.push(cmd);
        f.push(sub);
        f.extend_from_slice(data);
        f.push(CI_V_END);
        f
    }

    /// Sprawdza poprawność struktury ramki CI-V (preamble + footer).
    pub fn is_valid_frame(frame: &[u8]) -> bool {
        frame.len() >= 6
            && frame[0] == CI_V_PREAMBLE[0]
            && frame[1] == CI_V_PREAMBLE[1]
            && frame[frame.len() - 1] == CI_V_END
    }

    /// Koduje częstotliwość w Hz do 5 bajtów BCD (little-endian, 10 cyfr).
    pub fn encode_frequency(hz: u64) -> [u8; 5] {
        let clamped = hz % 10_000_000_000;
        let s = format!("{:010}", clamped);
        let b = s.as_bytes();
        let mut out = [0u8; 5];
        for i in 0..5 {
            let tens = b[8 - 2 * i] - b'0';
            let ones = b[9 - 2 * i] - b'0';
            out[i] = (tens << 4) | ones;
        }
        out
    }

    /// Dekoduje częstotliwość z bajtów BCD (little-endian).
    pub fn decode_frequency(data: &[u8]) -> u64 {
        let mut hz = 0u64;
        let mut mult = 1u64;
        for &byte in data {
            let ones = (byte & 0x0F) as u64;
            let tens = ((byte >> 4) & 0x0F) as u64;
            hz += ones * mult + tens * mult * 10;
            mult *= 100;
        }
        hz
    }

    /// Domyślny adres CI-V transceivera dla popularnych modeli Icom.
    /// Zawiera szerokie pokrycie; nieznane modele dostają bezpieczny domyślny 0x44.
    pub fn default_address(model: &str) -> u8 {
        let m = model.to_ascii_uppercase();
        let m = m.trim();
        // Najpierw dokładne dopasowanie (np. "IC-706MKIIG" ma własny adres).
        if let Some(addr) = Self::lookup(m) {
            return addr;
        }
        // Normalizacja przyrostków (np. "IC-7300MK2" -> "IC-7300").
        let base: &str = m
            .strip_suffix("MKIIG")
            .or_else(|| m.strip_suffix("MK2"))
            .or_else(|| m.strip_suffix("MKII"))
            .or_else(|| m.strip_suffix("MKIII"))
            .unwrap_or(m);

        Self::lookup(base).unwrap_or(0x44)
    }

    fn lookup(model: &str) -> Option<u8> {
        let addr = match model {
            "IC-275" => 0x10,
            "IC-375" => 0x12,
            "IC-475" => 0x14,
            "IC-575" => 0x16,
            "IC-725" => 0x28,
            "IC-726" => 0x2A,
            "IC-728" => 0x2C,
            "IC-729" => 0x2E,
            "IC-735" => 0x04,
            "IC-736" => 0x40,
            "IC-737" => 0x3C,
            "IC-738" => 0x44,
            "IC-746" => 0x56,
            "IC-746PRO" => 0x66,
            "IC-751" => 0x1C,
            "IC-756" => 0x50,
            "IC-756PRO" => 0x5C,
            "IC-756PROII" => 0x64,
            "IC-756PROIII" => 0x6E,
            "IC-761" => 0x1E,
            "IC-765" => 0x2C,
            "IC-775" => 0x46,
            "IC-781" => 0x26,
            "IC-78" => 0x62,
            "IC-7000" => 0x70,
            "IC-703" => 0x68,
            "IC-706" => 0x48,
            "IC-706MKII" => 0x4E,
            "IC-706MKIIG" => 0x58,
            "IC-707" => 0x3E,
            "IC-7100" => 0x88,
            "IC-718" => 0x5E,
            "IC-7200" => 0x76,
            "IC-7300" => 0x94,
            "IC-7410" => 0x80,
            "IC-7600" => 0x7A,
            "IC-7610" => 0x98,
            "IC-7700" => 0x74,
            "IC-7800" => 0x6A,
            "IC-7850" | "IC-7851" | "IC-7850/7851" => 0x8E,
            "IC-910" => 0x60,
            "IC-9100" => 0x7C,
            "IC-970" => 0x27,
            "IC-9700" => 0xA2,
            "IC-705" => 0xA4,
            "IC-905" => 0xB4,
            "IC-821H" => 0x4C,
            "IC-820H" => 0x4A,
            "IC-1271" => 0x24,
            "IC-1275" => 0x18,
            "IC-271" => 0x20,
            "IC-471" => 0x22,
            _ => return None,
        };
        Some(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_structure() {
        let f = CiV::build_frame(0x94, CI_V_CONTROLLER, command::READ_FREQUENCY, 0, &[]);
        assert_eq!(&f[..2], &[0xFE, 0xFE]);
        assert_eq!(f[2], 0x94);
        assert_eq!(f[3], 0xE0);
        assert_eq!(f[4], command::READ_FREQUENCY);
        assert_eq!(f[5], 0);
        assert_eq!(f[6], CI_V_END);
        assert!(CiV::is_valid_frame(&f));
    }

    #[test]
    fn invalid_frame_rejected() {
        assert!(!CiV::is_valid_frame(&[0x00, 0x01, 0x02]));
        assert!(!CiV::is_valid_frame(&[0xFE, 0xFE, 0x94, 0xE0, 0x03, 0x00]));
    }

    #[test]
    fn frequency_bcd_roundtrip() {
        for &hz in &[14_074_000u64, 7_074_000, 144_390_000, 430_000_000, 50_125_000] {
            let enc = CiV::encode_frequency(hz);
            assert_eq!(CiV::decode_frequency(&enc), hz, "failed for {}", hz);
        }
    }

    #[test]
    fn frequency_bcd_known_vector() {
        // 14,074,000 Hz -> "0014074000" -> [0x00, 0x40, 0x07, 0x14, 0x00]
        assert_eq!(CiV::encode_frequency(14_074_000), [0x00, 0x40, 0x07, 0x14, 0x00]);
    }

    #[test]
    fn frequency_overflow_wraps_to_10_digits() {
        // Wartości >= 10 GHz są obcinane do 10 cyfr (bez paniki).
        assert_eq!(CiV::decode_frequency(&CiV::encode_frequency(99_999_999_999)), 9_999_999_999);
    }

    #[test]
    fn default_addresses_for_popular_models() {
        assert_eq!(CiV::default_address("IC-7300"), 0x94);
        assert_eq!(CiV::default_address("ic-7610"), 0x98);
        assert_eq!(CiV::default_address("IC-9700"), 0xA2);
        assert_eq!(CiV::default_address("IC-705"), 0xA4);
        assert_eq!(CiV::default_address("IC-706MKIIG"), 0x58);
        assert_eq!(CiV::default_address("IC-756PROIII"), 0x6E);
        assert_eq!(CiV::default_address("FT-991A"), 0x44); // fallback
    }

    #[test]
    fn suffix_normalization() {
        assert_eq!(CiV::default_address("IC-7300MK2"), 0x94);
    }
}
