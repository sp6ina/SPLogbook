// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Kodek protokołu Icom CI-V (Communication Interface-V).
//!
//! Zapewnia dekodowanie i enkodowanie ramek strumieniowych:
//! `FE FE <to> <from> <cmd> [<sub>] [data..] FD`
//! z obsługą fragmentacji, ACK (FB), NAK (FA), BCD częstotliwości,
//! filtracji echa oraz zdarzeń transceive.

/// Preambuła, znacznik końca ramki i kody specjalne protokołu CI-V.
pub const CI_V_PREAMBLE: [u8; 2] = [0xFE, 0xFE];
pub const CI_V_END: u8 = 0xFD;
pub const CI_V_ACK: u8 = 0xFB;
pub const CI_V_NAK: u8 = 0xFA;
pub const CI_V_CONTROLLER: u8 = 0xE0;
pub const CI_V_BROADCAST: u8 = 0x00;
pub const MAX_CI_V_FRAME_LEN: usize = 128;

/// Komendy CI-V używane do odczytu/zapisu podstawowego stanu radia.
pub mod command {
    pub const SET_FREQUENCY: u8 = 0x05;
    pub const READ_FREQUENCY: u8 = 0x03;
    pub const READ_MODE: u8 = 0x04;
    pub const SET_MODE: u8 = 0x06;
    pub const VFO_SELECT: u8 = 0x07;
    pub const SPLIT_SELECT: u8 = 0x0F;
    pub const LEVEL_SET: u8 = 0x14;
    pub const READ_METER: u8 = 0x15;
    pub const SET_PTT: u8 = 0x1C;
    pub const READ_PTT: u8 = 0x1C;
    pub const RIT_XIT: u8 = 0x21;
}

/// Podkomendy CI-V.
pub mod subcommand {
    pub const PTT_CTRL: u8 = 0x00;
    pub const METER_SMETER: u8 = 0x02;
    pub const LEVEL_RF_POWER: u8 = 0x0A;
    pub const RIT_FREQ: u8 = 0x00;
    pub const XIT_FREQ: u8 = 0x01;
}

/// Błędy dekodera i kodera CI-V.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcomCodecError {
    InvalidPreamble,
    TruncatedFrame,
    FrameTooLong { len: usize, max: usize },
    InvalidBcd { byte: u8 },
    UnexpectedEndOfData,
    NakReceived,
    InvalidMode(String),
}

impl std::fmt::Display for IcomCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPreamble => write!(f, "invalid CI-V preamble"),
            Self::TruncatedFrame => write!(f, "truncated CI-V frame"),
            Self::FrameTooLong { len, max } => {
                write!(f, "CI-V frame too long ({len} bytes, max {max})")
            }
            Self::InvalidBcd { byte } => {
                write!(f, "invalid BCD byte in CI-V data: 0x{byte:02X}")
            }
            Self::UnexpectedEndOfData => write!(f, "unexpected end of CI-V data"),
            Self::NakReceived => write!(f, "radio rejected command (CI-V NAK 0xFA)"),
            Self::InvalidMode(m) => write!(f, "unsupported radio mode: '{m}'"),
        }
    }
}

impl std::error::Error for IcomCodecError {}

/// Ramka protokołu Icom CI-V.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcomCiVFrame {
    pub to: u8,
    pub from: u8,
    pub command: u8,
    pub subcommand: Option<u8>,
    pub data: Vec<u8>,
}

impl IcomCiVFrame {
    pub fn new(
        to: u8,
        from: u8,
        command: u8,
        subcommand: Option<u8>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            to,
            from,
            command,
            subcommand,
            data: data.into(),
        }
    }

    /// Buduje ramkę potwierdzenia ACK (`0xFB`).
    pub fn ack(to: u8, from: u8) -> Self {
        Self {
            to,
            from,
            command: CI_V_ACK,
            subcommand: None,
            data: Vec::new(),
        }
    }

    /// Buduje ramkę negacji NAK (`0xFA`).
    pub fn nak(to: u8, from: u8) -> Self {
        Self {
            to,
            from,
            command: CI_V_NAK,
            subcommand: None,
            data: Vec::new(),
        }
    }

    /// Czy ramka jest potwierdzeniem ACK.
    pub fn is_ack(&self) -> bool {
        self.command == CI_V_ACK
    }

    /// Czy ramka jest odmową NAK.
    pub fn is_nak(&self) -> bool {
        self.command == CI_V_NAK
    }

    /// Czy ramka jest broadcastem transceive (adres docelowy 0x00).
    pub fn is_broadcast(&self) -> bool {
        self.to == CI_V_BROADCAST
    }

    /// Serializuje ramkę do bajtów CI-V z preambułą `FE FE` i terminatorem `FD`.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(7 + self.data.len());
        out.extend_from_slice(&CI_V_PREAMBLE);
        out.push(self.to);
        out.push(self.from);
        out.push(self.command);
        if let Some(sub) = self.subcommand {
            out.push(sub);
        }
        out.extend_from_slice(&self.data);
        out.push(CI_V_END);
        out
    }
}

/// Strumieniowy kodek protokołu Icom CI-V.
pub struct IcomCiVCodec {
    pub controller_address: u8,
    pub radio_address: u8,
    pub max_frame_len: usize,
}

impl IcomCiVCodec {
    pub fn new(controller_address: u8, radio_address: u8) -> Self {
        Self {
            controller_address,
            radio_address,
            max_frame_len: MAX_CI_V_FRAME_LEN,
        }
    }

    /// Próbuje wyekstrahować kompletną ramkę z bufora odbiorczego.
    ///
    /// Odrzuca śmieci przed preambułą, ignoruje niepełne ramki i zwraca błąd
    /// w przypadku przekroczenia limitu rozmiaru ramki.
    pub fn extract_frame(
        &self,
        buffer: &mut Vec<u8>,
    ) -> Result<Option<IcomCiVFrame>, IcomCodecError> {
        loop {
            if buffer.is_empty() {
                return Ok(None);
            }

            // 1. Znajdź preambułę [0xFE, 0xFE]
            let preamble_pos = buffer
                .windows(2)
                .position(|w| w[0] == CI_V_PREAMBLE[0] && w[1] == CI_V_PREAMBLE[1]);

            if let Some(pos) = preamble_pos {
                // Odrzuć śmieci przed preambułą
                if pos > 0 {
                    buffer.drain(..pos);
                }
            } else {
                // Brak pełnej preambuły w buforze
                if buffer.last() == Some(&CI_V_PREAMBLE[0]) {
                    // Zostaw tylko ostatni bajt 0xFE
                    let trim_len = buffer.len() - 1;
                    buffer.drain(..trim_len);
                } else {
                    buffer.clear();
                }
                return Ok(None);
            }

            // 2. Znajdź terminator 0xFD zaczynając od preambuły
            let end_pos = buffer[2..].iter().position(|&b| b == CI_V_END);

            if let Some(relative_end) = end_pos {
                let full_end = 2 + relative_end; // indeks 0xFD w buforze
                let frame_len = full_end + 1;

                if frame_len > self.max_frame_len {
                    // Ramka przekracza limit — usuń wadliwą preambułę i szukaj dalej
                    buffer.drain(..2);
                    return Err(IcomCodecError::FrameTooLong {
                        len: frame_len,
                        max: self.max_frame_len,
                    });
                }

                // Wyciągnij ramkę z bufora
                let frame_bytes: Vec<u8> = buffer.drain(..frame_len).collect();

                // Parsuj strukturę ramki: FE FE <to> <from> <cmd> ... FD
                // Minimalna długość: FE FE TO FROM CMD FD = 6 bajtów
                // Wyjątek: niektóre radia wysyłają echo lub ACK/NAK
                if frame_bytes.len() < 5 {
                    continue;
                }

                let to = frame_bytes[2];
                let from = frame_bytes[3];
                let cmd = frame_bytes[4];

                // Sprawdź ACK (0xFB) lub NAK (0xFA)
                if cmd == CI_V_ACK {
                    return Ok(Some(IcomCiVFrame::ack(to, from)));
                }
                if cmd == CI_V_NAK {
                    return Ok(Some(IcomCiVFrame::nak(to, from)));
                }

                let payload = &frame_bytes[5..frame_bytes.len() - 1];

                // Rozpoznawanie komendy i ewentualnej podkomendy
                let (subcommand, data) = match cmd {
                    command::LEVEL_SET
                    | command::READ_METER
                    | command::SET_PTT
                    | command::RIT_XIT => {
                        if payload.is_empty() {
                            (None, Vec::new())
                        } else {
                            (Some(payload[0]), payload[1..].to_vec())
                        }
                    }
                    _ => (None, payload.to_vec()),
                };

                return Ok(Some(IcomCiVFrame {
                    to,
                    from,
                    command: cmd,
                    subcommand,
                    data,
                }));
            }

            // Brak terminatora w buforze — sprawdź czy nie doszło do przekroczenia limitu
            if buffer.len() > self.max_frame_len {
                buffer.drain(..2);
                return Err(IcomCodecError::FrameTooLong {
                    len: buffer.len(),
                    max: self.max_frame_len,
                });
            }
            return Ok(None);
        }
    }

    /// Koduje częstotliwość w Hz do formatu BCD little-endian (domyślnie 5 bajtów / 10 cyfr).
    pub fn encode_frequency_bcd(hz: u64, byte_count: usize) -> Vec<u8> {
        let mut out = vec![0u8; byte_count];
        let mut rem = hz;
        for byte in out.iter_mut().take(byte_count) {
            let ones = (rem % 10) as u8;
            rem /= 10;
            let tens = (rem % 10) as u8;
            rem /= 10;
            *byte = (tens << 4) | ones;
        }
        out
    }

    /// Dekoduje częstotliwość z formatu BCD little-endian z rygorystyczną walidacją cyfr.
    pub fn decode_frequency_bcd(data: &[u8]) -> Result<u64, IcomCodecError> {
        let mut hz = 0u64;
        let mut mult = 1u64;
        for &byte in data {
            let ones = (byte & 0x0F) as u64;
            let tens = ((byte >> 4) & 0x0F) as u64;
            if ones > 9 || tens > 9 {
                return Err(IcomCodecError::InvalidBcd { byte });
            }
            let pair = tens * 10 + ones;
            hz = hz.saturating_add(pair.saturating_mul(mult));
            mult = mult.saturating_mul(100);
        }
        Ok(hz)
    }

    /// Mapuje nazwę emisji SPLogbook na kod CI-V i opcjonalny numer filtru.
    pub fn mode_to_code(mode: &str) -> (u8, Option<u8>) {
        match mode.to_ascii_uppercase().as_str() {
            "LSB" => (0x00, Some(0x01)),
            "AM" => (0x02, Some(0x01)),
            "CW" => (0x03, Some(0x01)),
            "RTTY" | "FSK" => (0x04, Some(0x01)),
            "FM" => (0x05, Some(0x01)),
            "WFM" => (0x06, None),
            "CW-R" | "CWR" => (0x07, Some(0x01)),
            "RTTY-R" | "FSKR" | "FSK-R" => (0x08, Some(0x01)),
            "DV" | "D-STAR" => (0x17, None),
            _ => (0x01, Some(0x01)), // USB oraz domyślne cyfrowe
        }
    }

    /// Dekoduje kod emisji CI-V na domenową nazwę emisji.
    pub fn code_to_mode(code: u8) -> &'static str {
        match code {
            0x00 => "LSB",
            0x02 => "AM",
            0x03 => "CW",
            0x04 => "RTTY",
            0x05 => "FM",
            0x06 => "WFM",
            0x07 => "CW-R",
            0x08 => "RTTY-R",
            0x17 => "DV",
            _ => "USB",
        }
    }

    /// Dekoduje odczyt S-Metra (0x15 0x02, 2 bajty BCD 0000..0255).
    pub fn decode_smeter(data: &[u8]) -> (f32, String) {
        if data.is_empty() {
            return (-100.0, "S1".to_string());
        }
        let raw = if data.len() >= 2 {
            let tens = ((data[0] >> 4) & 0x0F) * 10 + (data[0] & 0x0F);
            let hundreds = ((data[1] >> 4) & 0x0F) * 10 + (data[1] & 0x0F);
            (hundreds as u16) * 100 + (tens as u16)
        } else {
            let tens = ((data[0] >> 4) & 0x0F) * 10 + (data[0] & 0x0F);
            tens as u16
        };

        // Standard Icom CI-V:
        // 0000 = S0
        // 0120 = S9 (-73 dBm)
        // 0240 = S9+60dB (-13 dBm)
        if raw <= 120 {
            let s_val = (raw as f32 / 120.0 * 9.0).round() as u8;
            let dbm = -127.0 + (s_val as f32 * 6.0);
            (dbm, format!("S{s_val}"))
        } else {
            let over_db = ((raw.saturating_sub(120)) as f32 / 120.0 * 60.0).round() as u8;
            let dbm = -73.0 + (over_db as f32);
            (dbm, format!("S9+{over_db}dB"))
        }
    }

    /// Dekoduje odczyt poziomu mocy (0x14 0x0A, 2 bajty BCD 0000..0255).
    pub fn decode_power_watts(data: &[u8], max_power_watts: f32) -> f32 {
        if data.is_empty() {
            return 0.0;
        }
        let raw = if data.len() >= 2 {
            let tens = ((data[0] >> 4) & 0x0F) * 10 + (data[0] & 0x0F);
            let hundreds = ((data[1] >> 4) & 0x0F) * 10 + (data[1] & 0x0F);
            (hundreds as u16) * 100 + (tens as u16)
        } else {
            let tens = ((data[0] >> 4) & 0x0F) * 10 + (data[0] & 0x0F);
            tens as u16
        };

        let ratio = (raw as f32 / 255.0).clamp(0.0, 1.0);
        (ratio * max_power_watts).round()
    }
}

/// Zachowana kompatybilność z dotychczasowym API pomocniczym `CiV`.
pub struct CiV;

impl CiV {
    pub fn build_frame(to_addr: u8, from_addr: u8, cmd: u8, sub: u8, data: &[u8]) -> Vec<u8> {
        let frame = IcomCiVFrame::new(to_addr, from_addr, cmd, Some(sub), data);
        frame.encode()
    }

    pub fn build_simple_frame(to_addr: u8, from_addr: u8, cmd: u8, data: &[u8]) -> Vec<u8> {
        let frame = IcomCiVFrame::new(to_addr, from_addr, cmd, None, data);
        frame.encode()
    }

    pub fn read_frequency(to_addr: u8) -> Vec<u8> {
        Self::build_simple_frame(to_addr, CI_V_CONTROLLER, command::READ_FREQUENCY, &[])
    }

    pub fn read_mode(to_addr: u8) -> Vec<u8> {
        Self::build_simple_frame(to_addr, CI_V_CONTROLLER, command::READ_MODE, &[])
    }

    pub fn set_frequency(to_addr: u8, hz: u64) -> Vec<u8> {
        let bcd = Self::encode_frequency(hz);
        Self::build_simple_frame(to_addr, CI_V_CONTROLLER, command::SET_FREQUENCY, &bcd)
    }

    pub fn set_mode(to_addr: u8, mode_code: u8) -> Vec<u8> {
        Self::build_simple_frame(to_addr, CI_V_CONTROLLER, command::SET_MODE, &[mode_code])
    }

    pub fn is_valid_frame(frame: &[u8]) -> bool {
        frame.len() >= 6
            && frame[0] == CI_V_PREAMBLE[0]
            && frame[1] == CI_V_PREAMBLE[1]
            && frame[frame.len() - 1] == CI_V_END
    }

    pub fn encode_frequency(hz: u64) -> [u8; 5] {
        let vec = IcomCiVCodec::encode_frequency_bcd(hz, 5);
        let mut arr = [0u8; 5];
        arr.copy_from_slice(&vec[..5]);
        arr
    }

    pub fn decode_frequency(data: &[u8]) -> u64 {
        IcomCiVCodec::decode_frequency_bcd(data).unwrap_or(0)
    }

    pub fn default_address(model: &str) -> u8 {
        let model_upper = model.to_ascii_uppercase();
        let model_upper = model_upper.trim();
        if let Some(addr) = Self::lookup(model_upper) {
            return addr;
        }
        let base: &str = model_upper
            .strip_suffix("MKIIG")
            .or_else(|| model_upper.strip_suffix("MK2"))
            .or_else(|| model_upper.strip_suffix("MKII"))
            .or_else(|| model_upper.strip_suffix("MKIII"))
            .unwrap_or(model_upper);

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
            "IC-728" | "IC-765" => 0x2C,
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
    fn icom_codec_encodes_frame() {
        let frame = IcomCiVFrame::new(
            0x94,
            CI_V_CONTROLLER,
            command::SET_FREQUENCY,
            None,
            vec![0x00, 0x40, 0x07, 0x14, 0x00],
        );
        let encoded = frame.encode();
        assert_eq!(
            encoded,
            &[
                0xFE, 0xFE, 0x94, 0xE0, 0x05, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD
            ]
        );
    }

    #[test]
    fn icom_codec_decodes_fragmented_frame() {
        let codec = IcomCiVCodec::new(CI_V_CONTROLLER, 0x94);
        let mut buf = Vec::new();

        buf.extend_from_slice(&[0xFE, 0xFE]);
        assert_eq!(codec.extract_frame(&mut buf).unwrap(), None);

        buf.extend_from_slice(&[0xE0, 0x94, 0x03]);
        assert_eq!(codec.extract_frame(&mut buf).unwrap(), None);

        buf.extend_from_slice(&[0x00, 0x40, 0x07, 0x14, 0x00, 0xFD]);
        let frame = codec
            .extract_frame(&mut buf)
            .unwrap()
            .expect("frame complete");
        assert_eq!(frame.to, 0xE0);
        assert_eq!(frame.from, 0x94);
        assert_eq!(frame.command, 0x03);
        assert_eq!(
            IcomCiVCodec::decode_frequency_bcd(&frame.data).unwrap(),
            14_074_000
        );
        assert!(buf.is_empty());
    }

    #[test]
    fn icom_codec_decodes_multiple_frames() {
        let codec = IcomCiVCodec::new(CI_V_CONTROLLER, 0x94);
        let mut buf = vec![
            0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD, // ACK
            0xFE, 0xFE, 0xE0, 0x94, 0xFA, 0xFD, // NAK
        ];

        let f1 = codec.extract_frame(&mut buf).unwrap().expect("f1");
        assert!(f1.is_ack());

        let f2 = codec.extract_frame(&mut buf).unwrap().expect("f2");
        assert!(f2.is_nak());
        assert!(buf.is_empty());
    }

    #[test]
    fn icom_codec_rejects_invalid_bcd() {
        // 0x0A nie jest poprawną cyfrą BCD
        let res = IcomCiVCodec::decode_frequency_bcd(&[0x0A, 0x40]);
        assert_eq!(res, Err(IcomCodecError::InvalidBcd { byte: 0x0A }));
    }

    #[test]
    fn icom_codec_handles_ack() {
        let codec = IcomCiVCodec::new(CI_V_CONTROLLER, 0x94);
        let mut buf = vec![0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD];
        let f = codec.extract_frame(&mut buf).unwrap().expect("frame");
        assert!(f.is_ack());
        assert!(!f.is_nak());
    }

    #[test]
    fn icom_codec_handles_nak() {
        let codec = IcomCiVCodec::new(CI_V_CONTROLLER, 0x94);
        let mut buf = vec![0xFE, 0xFE, 0xE0, 0x94, 0xFA, 0xFD];
        let f = codec.extract_frame(&mut buf).unwrap().expect("frame");
        assert!(f.is_nak());
        assert!(!f.is_ack());
    }

    #[test]
    fn icom_codec_handles_transceive_event() {
        let codec = IcomCiVCodec::new(CI_V_CONTROLLER, 0x94);
        // Transceive broadcast ma adres docelowy 0x00
        let mut buf = vec![
            0xFE, 0xFE, 0x00, 0x94, 0x03, 0x00, 0x40, 0x07, 0x14, 0x00, 0xFD,
        ];
        let f = codec.extract_frame(&mut buf).unwrap().expect("broadcast");
        assert!(f.is_broadcast());
        assert_eq!(f.command, 0x03);
    }

    #[test]
    fn icom_codec_enforces_max_frame_length() {
        let codec = IcomCiVCodec::new(CI_V_CONTROLLER, 0x94);
        let mut buf = vec![0xFE, 0xFE];
        buf.extend(std::iter::repeat(0xAA).take(200)); // brak 0xFD
        let res = codec.extract_frame(&mut buf);
        assert!(matches!(res, Err(IcomCodecError::FrameTooLong { .. })));
    }

    #[test]
    fn icom_frequency_roundtrip() {
        for &hz in &[
            1_810_000u64,
            3_573_000,
            7_074_000,
            14_074_000,
            21_074_000,
            28_074_000,
            50_313_000,
            144_300_000,
            432_100_000,
            1_296_200_000,
        ] {
            let bcd = IcomCiVCodec::encode_frequency_bcd(hz, 5);
            let decoded = IcomCiVCodec::decode_frequency_bcd(&bcd).unwrap();
            assert_eq!(decoded, hz, "failed for {hz}");
        }
    }

    #[test]
    fn icom_mode_roundtrip() {
        for &mode in &["LSB", "USB", "CW", "AM", "FM", "RTTY", "CW-R", "RTTY-R"] {
            let (code, _) = IcomCiVCodec::mode_to_code(mode);
            let decoded = IcomCiVCodec::code_to_mode(code);
            assert_eq!(decoded, mode, "mode failed for {mode}");
        }
    }

    #[test]
    fn icom_unknown_frame_does_not_panic() {
        let codec = IcomCiVCodec::new(CI_V_CONTROLLER, 0x94);
        let mut buf = vec![0xFE, 0xFE, 0xE0, 0x94, 0x77, 0x12, 0x34, 0xFD];
        let f = codec
            .extract_frame(&mut buf)
            .unwrap()
            .expect("unknown frame parsed");
        assert_eq!(f.command, 0x77);
        assert_eq!(f.data, vec![0x12, 0x34]);
    }

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
    fn frequency_bcd_known_vector() {
        assert_eq!(
            CiV::encode_frequency(14_074_000),
            [0x00, 0x40, 0x07, 0x14, 0x00]
        );
    }

    #[test]
    fn frequency_bcd_roundtrip() {
        for &hz in &[
            14_074_000u64,
            7_074_000,
            144_390_000,
            430_000_000,
            50_125_000,
        ] {
            let enc = CiV::encode_frequency(hz);
            assert_eq!(CiV::decode_frequency(&enc), hz, "failed for {hz}");
        }
    }

    #[test]
    fn frequency_overflow_wraps_to_10_digits() {
        assert_eq!(
            CiV::decode_frequency(&CiV::encode_frequency(99_999_999_999)),
            9_999_999_999
        );
    }

    #[test]
    fn default_addresses_for_popular_models() {
        assert_eq!(CiV::default_address("IC-7300"), 0x94);
        assert_eq!(CiV::default_address("ic-7610"), 0x98);
        assert_eq!(CiV::default_address("IC-9700"), 0xA2);
        assert_eq!(CiV::default_address("IC-705"), 0xA4);
        assert_eq!(CiV::default_address("IC-706MKIIG"), 0x58);
        assert_eq!(CiV::default_address("IC-756PROIII"), 0x6E);
        assert_eq!(CiV::default_address("FT-991A"), 0x44);
    }

    #[test]
    fn suffix_normalization() {
        assert_eq!(CiV::default_address("IC-7300MK2"), 0x94);
    }
}
