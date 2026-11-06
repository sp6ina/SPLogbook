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
    pub const SPLIT_FREQ: u8 = 0x25;
}

/// Podkomendy CI-V.
pub mod subcommand {
    pub const PTT_CTRL: u8 = 0x00;
    pub const METER_SMETER: u8 = 0x02;
    pub const LEVEL_RF_POWER: u8 = 0x0A;
    pub const RIT_FREQ: u8 = 0x00;
    pub const RIT_ON_OFF: u8 = 0x01;
    pub const XIT_ON_OFF: u8 = 0x02;
    pub const SPLIT_FREQ_SUB: u8 = 0x01;
}

/// Błędy dekodera i kodera CI-V.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcomCodecError {
    InvalidPreamble,
    TruncatedFrame,
    FrameTooLong { len: usize, max: usize },
    InvalidBcd { byte: u8 },
    InvalidBcdLength { len: usize },
    FrequencyOutOfRange { hz: u64, byte_count: usize },
    LevelOutOfRange { val: u16 },
    ArithmeticOverflow,
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
            Self::InvalidBcdLength { len } => write!(f, "invalid BCD buffer length: {len}"),
            Self::FrequencyOutOfRange { hz, byte_count } => {
                write!(
                    f,
                    "frequency {hz} Hz out of range for {byte_count}-byte BCD representation"
                )
            }
            Self::LevelOutOfRange { val } => write!(f, "level {val} out of range (max 255)"),
            Self::ArithmeticOverflow => write!(f, "arithmetic overflow during BCD decoding"),
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

    /// Czy ramka jest echem komendy wysłanej przez nasz kontroler.
    pub fn is_echo_of(&self, controller: u8, radio: u8) -> bool {
        self.to == radio && self.from == controller
    }

    /// Czy ramka pochodzi z naszego radia i jest skierowana do naszego kontrolera.
    pub fn is_response_to(&self, controller: u8, radio: u8) -> bool {
        self.to == controller && self.from == radio
    }

    /// Czy ramka jest skierowana do innego urządzenia na magistrali CI-V.
    pub fn is_for_other_device(&self, controller: u8) -> bool {
        self.to != controller && !self.is_broadcast()
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
                // Minimalna poprawna długość ramki Icom CI-V to 6 bajtów:
                // FE FE TO FROM CMD FD. Ramki krótsze niż 6 bajtów są obcięte lub wadliwe.
                if frame_bytes.len() < 6 {
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
                    | command::RIT_XIT
                    | command::SPLIT_FREQ => {
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
    pub fn encode_frequency_bcd(hz: u64, byte_count: usize) -> Result<Vec<u8>, IcomCodecError> {
        if byte_count == 0 {
            return Err(IcomCodecError::InvalidBcdLength { len: 0 });
        }
        let mut out = vec![0u8; byte_count];
        let mut rem = hz;
        for byte in out.iter_mut().take(byte_count) {
            let ones = (rem % 10) as u8;
            rem /= 10;
            let tens = (rem % 10) as u8;
            rem /= 10;
            *byte = (tens << 4) | ones;
        }
        if rem > 0 {
            return Err(IcomCodecError::FrequencyOutOfRange { hz, byte_count });
        }
        Ok(out)
    }

    /// Koduje 2-bajtowy poziom Big-Endian BCD (0000..0255) dla komend poziomu i mocy Icom CI-V.
    pub fn encode_level_bcd_be(val: u16) -> Result<[u8; 2], IcomCodecError> {
        if val > 255 {
            return Err(IcomCodecError::LevelOutOfRange { val });
        }
        let hundreds = (val / 100) as u8;
        let rem = val % 100;
        let tens = (rem / 10) as u8;
        let ones = (rem % 10) as u8;
        Ok([hundreds, (tens << 4) | ones])
    }

    /// Dekoduje 2-bajtowy poziom Big-Endian BCD (0000..0255) dla S-metra i mocy Icom CI-V.
    pub fn decode_level_bcd_be(data: &[u8]) -> Result<u16, IcomCodecError> {
        if data.is_empty() {
            return Err(IcomCodecError::UnexpectedEndOfData);
        }
        if data.len() < 2 {
            let b0 = data[0];
            let ones = b0 & 0x0F;
            let tens = (b0 >> 4) & 0x0F;
            if ones > 9 || tens > 9 {
                return Err(IcomCodecError::InvalidBcd { byte: b0 });
            }
            return Ok((tens * 10 + ones) as u16);
        }
        let b0 = data[0];
        let b1 = data[1];
        let thousands = (b0 >> 4) & 0x0F;
        let hundreds = b0 & 0x0F;
        let tens = (b1 >> 4) & 0x0F;
        let ones = b1 & 0x0F;
        if thousands > 9 || hundreds > 9 {
            return Err(IcomCodecError::InvalidBcd { byte: b0 });
        }
        if tens > 9 || ones > 9 {
            return Err(IcomCodecError::InvalidBcd { byte: b1 });
        }
        let raw = (thousands as u16) * 1000
            + (hundreds as u16) * 100
            + (tens as u16) * 10
            + (ones as u16);
        Ok(raw)
    }

    /// Dekoduje częstotliwość z formatu BCD little-endian z rygorystyczną walidacją cyfr
    /// i arytmetyką checked zapobiegającą overflow.
    pub fn decode_frequency_bcd(data: &[u8]) -> Result<u64, IcomCodecError> {
        if data.is_empty() {
            return Err(IcomCodecError::UnexpectedEndOfData);
        }
        let mut hz = 0u64;
        let mut mult = 1u64;
        for (idx, &byte) in data.iter().enumerate() {
            let ones = (byte & 0x0F) as u64;
            let tens = ((byte >> 4) & 0x0F) as u64;
            if ones > 9 || tens > 9 {
                return Err(IcomCodecError::InvalidBcd { byte });
            }
            let pair = tens * 10 + ones;
            let term = pair
                .checked_mul(mult)
                .ok_or(IcomCodecError::ArithmeticOverflow)?;
            hz = hz
                .checked_add(term)
                .ok_or(IcomCodecError::ArithmeticOverflow)?;
            if idx + 1 < data.len() {
                mult = mult
                    .checked_mul(100)
                    .ok_or(IcomCodecError::ArithmeticOverflow)?;
            }
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
        let raw = Self::decode_level_bcd_be(data).unwrap_or(0);

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
        let raw = Self::decode_level_bcd_be(data).unwrap_or(0);

        let ratio = (raw as f32 / 255.0).clamp(0.0, 1.0);
        (ratio * max_power_watts).round()
    }

    /// Koduje offset częstotliwości RIT/XIT w formacie Icom CI-V (komenda 0x21 subkomenda 0x00).
    ///
    /// Format: 2 bajty BCD (little-endian: 1Hz/10Hz, 100Hz/1000Hz) oraz 1 bajt znaku
    /// (0x00 = dodatni, 0x01 = ujemny). Dopuszczalny zakres: -9999..=9999 Hz.
    pub fn encode_rit_offset(hz: i32) -> Result<[u8; 3], IcomCodecError> {
        let is_negative = hz < 0;
        let abs_val = hz.unsigned_abs();
        if abs_val > 9999 {
            return Err(IcomCodecError::LevelOutOfRange {
                val: abs_val.min(u16::MAX as u32) as u16,
            });
        }

        let d0 = (abs_val % 10) as u8;
        let d1 = ((abs_val / 10) % 10) as u8;
        let d2 = ((abs_val / 100) % 10) as u8;
        let d3 = ((abs_val / 1000) % 10) as u8;

        let b0 = d0 | (d1 << 4);
        let b1 = d2 | (d3 << 4);
        let sign = u8::from(is_negative);

        Ok([b0, b1, sign])
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
        match IcomCiVCodec::encode_frequency_bcd(hz, 5) {
            Ok(vec) => {
                let mut arr = [0u8; 5];
                arr.copy_from_slice(&vec[..5]);
                arr
            }
            Err(_) => [0u8; 5],
        }
    }

    pub fn try_encode_frequency(hz: u64) -> Result<[u8; 5], IcomCodecError> {
        let vec = IcomCiVCodec::encode_frequency_bcd(hz, 5)?;
        let mut arr = [0u8; 5];
        arr.copy_from_slice(&vec[..5]);
        Ok(arr)
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
        assert_eq!(buf, [] as [u8; 0]);
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
        assert_eq!(buf, [] as [u8; 0]);
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
        buf.extend(std::iter::repeat_n(0xAA, 200)); // brak 0xFD
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
            let bcd = IcomCiVCodec::encode_frequency_bcd(hz, 5).unwrap();
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
    fn civ_frequency_bcd_roundtrips_valid_values() {
        let test_values = [
            0u64,
            1,
            9,
            10,
            100,
            1_000,
            1_800_000,
            3_500_000,
            7_000_000,
            14_000_000,
            14_074_000,
            21_000_000,
            28_000_000,
            50_000_000,
            145_500_000,
            433_500_000,
            1_296_000_000,
            9_999_999_999,
        ];
        for &hz in &test_values {
            let enc = IcomCiVCodec::encode_frequency_bcd(hz, 5).expect("encode failed");
            assert_eq!(enc.len(), 5);
            let dec = IcomCiVCodec::decode_frequency_bcd(&enc).expect("decode failed");
            assert_eq!(dec, hz, "roundtrip mismatch for {hz}");
        }
    }

    #[test]
    fn civ_frequency_bcd_preserves_zero_digits() {
        let hz = 10_000_005u64;
        let enc = IcomCiVCodec::encode_frequency_bcd(hz, 5).expect("encode");
        assert_eq!(enc, vec![0x05, 0x00, 0x00, 0x10, 0x00]);
        let dec = IcomCiVCodec::decode_frequency_bcd(&enc).expect("decode");
        assert_eq!(dec, hz);

        let zero_enc = IcomCiVCodec::encode_frequency_bcd(0, 5).expect("encode zero");
        assert_eq!(zero_enc, vec![0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(IcomCiVCodec::decode_frequency_bcd(&zero_enc).unwrap(), 0);
    }

    #[test]
    fn civ_frequency_bcd_rejects_invalid_nibbles() {
        for invalid_byte in [0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xA0, 0xB5, 0xFA, 0xFF] {
            let buf = [0x00, 0x00, invalid_byte, 0x00, 0x00];
            let res = IcomCiVCodec::decode_frequency_bcd(&buf);
            assert!(
                matches!(res, Err(IcomCodecError::InvalidBcd { byte }) if byte == invalid_byte),
                "expected rejection for byte 0x{invalid_byte:02X}"
            );
        }
    }

    #[test]
    fn civ_frequency_bcd_rejects_too_large_value() {
        let too_large = 10_000_000_000u64;
        let res = IcomCiVCodec::encode_frequency_bcd(too_large, 5);
        assert_eq!(
            res,
            Err(IcomCodecError::FrequencyOutOfRange {
                hz: too_large,
                byte_count: 5,
            })
        );
    }

    #[test]
    fn civ_frequency_bcd_rejects_wrong_length() {
        let res = IcomCiVCodec::encode_frequency_bcd(14_000_000, 0);
        assert_eq!(res, Err(IcomCodecError::InvalidBcdLength { len: 0 }));

        let dec_res = IcomCiVCodec::decode_frequency_bcd(&[]);
        assert_eq!(dec_res, Err(IcomCodecError::UnexpectedEndOfData));
    }

    #[test]
    fn civ_frequency_bcd_never_silently_truncates() {
        assert!(IcomCiVCodec::encode_frequency_bcd(10_000, 2).is_err());
        assert_eq!(
            IcomCiVCodec::encode_frequency_bcd(9_999, 2).unwrap(),
            vec![0x99, 0x99]
        );
        assert!(IcomCiVCodec::encode_frequency_bcd(10_000_000, 3).is_err());
    }

    #[test]
    fn civ_frequency_bcd_checked_arithmetic_rejects_overflow() {
        let huge_buf = [0x99; 12];
        let res = IcomCiVCodec::decode_frequency_bcd(&huge_buf);
        assert_eq!(res, Err(IcomCodecError::ArithmeticOverflow));
    }

    #[test]
    fn icom_civ_level_and_meter_bcd_encoding_is_big_endian_0000_to_0255() {
        let enc_120 = IcomCiVCodec::encode_level_bcd_be(120).unwrap();
        assert_eq!(enc_120, [0x01, 0x20]);
        let dec_120 = IcomCiVCodec::decode_level_bcd_be(&enc_120).unwrap();
        assert_eq!(dec_120, 120);

        let enc_255 = IcomCiVCodec::encode_level_bcd_be(255).unwrap();
        assert_eq!(enc_255, [0x02, 0x55]);
        let dec_255 = IcomCiVCodec::decode_level_bcd_be(&enc_255).unwrap();
        assert_eq!(dec_255, 255);

        let enc_0 = IcomCiVCodec::encode_level_bcd_be(0).unwrap();
        assert_eq!(enc_0, [0x00, 0x00]);
        assert_eq!(IcomCiVCodec::decode_level_bcd_be(&enc_0).unwrap(), 0);

        assert_eq!(
            IcomCiVCodec::encode_level_bcd_be(256),
            Err(IcomCodecError::LevelOutOfRange { val: 256 })
        );

        let (dbm_s9, unit_s9) = IcomCiVCodec::decode_smeter(&[0x01, 0x20]);
        assert_eq!(unit_s9, "S9");
        assert!((dbm_s9 - (-73.0)).abs() < 1.0);

        let pwr_100w = IcomCiVCodec::decode_power_watts(&[0x02, 0x55], 100.0);
        assert_eq!(pwr_100w, 100.0);
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

    #[test]
    fn icom_civ_encode_rit_offset() {
        assert_eq!(
            IcomCiVCodec::encode_rit_offset(0).unwrap(),
            [0x00, 0x00, 0x00]
        );
        assert_eq!(
            IcomCiVCodec::encode_rit_offset(120).unwrap(),
            [0x20, 0x01, 0x00]
        );
        assert_eq!(
            IcomCiVCodec::encode_rit_offset(-150).unwrap(),
            [0x50, 0x01, 0x01]
        );
        assert_eq!(
            IcomCiVCodec::encode_rit_offset(9999).unwrap(),
            [0x99, 0x99, 0x00]
        );
        assert_eq!(
            IcomCiVCodec::encode_rit_offset(-9999).unwrap(),
            [0x99, 0x99, 0x01]
        );
        assert!(IcomCiVCodec::encode_rit_offset(10000).is_err());
        assert!(IcomCiVCodec::encode_rit_offset(-10000).is_err());
    }

    #[test]
    fn icom_civ_extract_frame_rejects_truncated_and_accepts_valid() {
        let codec = IcomCiVCodec::new(0xE0, 0x94);

        // Ramka ucięta < 6 bajtów: FE FE 94 E0 FD
        let mut buf_short = vec![0xFE, 0xFE, 0x94, 0xE0, 0xFD];
        let res = codec.extract_frame(&mut buf_short).unwrap();
        assert!(res.is_none());

        // Prawidłowa ramka ACK: FE FE E0 94 FB FD (6 bajtów)
        let mut buf_ack = vec![0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD];
        let ack_frame = codec.extract_frame(&mut buf_ack).unwrap().unwrap();
        assert!(ack_frame.is_ack());
        assert_eq!(ack_frame.to, 0xE0);
        assert_eq!(ack_frame.from, 0x94);

        // Prawidłowa ramka NAK: FE FE E0 94 FA FD (6 bajtów)
        let mut buf_nak = vec![0xFE, 0xFE, 0xE0, 0x94, 0xFA, 0xFD];
        let nak_frame = codec.extract_frame(&mut buf_nak).unwrap().unwrap();
        assert!(nak_frame.is_nak());

        // Śmieci przed ramką
        let mut buf_noise = vec![0x00, 0xAA, 0xFF, 0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD];
        let frame_after_noise = codec.extract_frame(&mut buf_noise).unwrap().unwrap();
        assert!(frame_after_noise.is_ack());
        assert_eq!(buf_noise, [] as [u8; 0]);

        // Dwie ramki sklejone w jednym buforze
        let mut buf_two = vec![
            0xFE, 0xFE, 0xE0, 0x94, 0xFB, 0xFD, 0xFE, 0xFE, 0xE0, 0x94, 0x03, 0x00, 0x40, 0x07,
            0x14, 0x00, 0xFD,
        ];
        let f1 = codec.extract_frame(&mut buf_two).unwrap().unwrap();
        assert!(f1.is_ack());
        let f2 = codec.extract_frame(&mut buf_two).unwrap().unwrap();
        assert_eq!(f2.command, 0x03);
        assert_eq!(buf_two, [] as [u8; 0]);
    }

    #[test]
    fn icom_civ_frame_helpers_detect_echo_and_address_mismatch() {
        let controller = 0xE0;
        let radio = 0x94;

        // Echo wysłanej komendy: to=0x94, from=0xE0
        let echo_frame = IcomCiVFrame::new(radio, controller, 0x03, None, vec![]);
        assert!(echo_frame.is_echo_of(controller, radio));
        assert!(!echo_frame.is_response_to(controller, radio));

        // Odpowiedź radia: to=0xE0, from=0x94
        let resp_frame = IcomCiVFrame::new(controller, radio, 0x03, None, vec![0x00, 0x40]);
        assert!(resp_frame.is_response_to(controller, radio));
        assert!(!resp_frame.is_echo_of(controller, radio));
        assert!(!resp_frame.is_for_other_device(controller));

        // Ramka do innego urządzenia na magistrali: to=0x88, from=0x94
        let other_frame = IcomCiVFrame::new(0x88, radio, 0x03, None, vec![]);
        assert!(other_frame.is_for_other_device(controller));
        assert!(!other_frame.is_response_to(controller, radio));
    }
}
