// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Strumieniowy kodek protokołu Kenwood PC Control (CAT).
//!
//! Obsługuje ramkowanie ASCII ze znakiem podziału `;`, dekodowanie
//! z buforów asynchronicznych, wykrywanie fragmentacji, komendy
//! Auto Information (`AI`), raport stanu (`IF`) oraz nagłówki KNS LAN (`##`).
//!
//! Standard stosowany przez transceivery:
//! - Kenwood: TS-590S/SG, TS-890S, TS-990S, TS-2000, TS-480, TS-570
//! - Elecraft: K2, K3, K3S, KX2, KX3, K4
//! - Yaesu (w trybie CAT tekstowym zgodnym z Kenwood): FT-891, FT-991A, FTDX10, FT-710
//!
//! Komendy kończą się znakiem średnika `;`.

/// Domyślny maksymalny dozwolony rozmiar pojedynczej ramki Kenwood CAT (w bajtach).
pub const KENWOOD_DEFAULT_MAX_FRAME_LEN: usize = 256;

/// Błędy dekodowania strumieniowego protokołu Kenwood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KenwoodCodecError {
    /// Przekroczono maksymalną dozwoloną długość pojedynczej ramki bez średnika.
    FrameTooLarge { len: usize, max: usize },
    /// Ramka zawiera niepoprawne bajty UTF-8.
    InvalidUtf8 { detail: String },
    /// Ramka jest uszkodzona lub ma nieznaną strukturę.
    MalformedFrame { raw: String },
}

impl std::fmt::Display for KenwoodCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameTooLarge { len, max } => {
                write!(f, "Kenwood CAT frame exceeded max size ({len} > {max})")
            }
            Self::InvalidUtf8 { detail } => {
                write!(f, "Kenwood CAT frame is not valid UTF-8: {detail}")
            }
            Self::MalformedFrame { raw } => {
                write!(f, "malformed Kenwood CAT frame: {raw}")
            }
        }
    }
}

impl std::error::Error for KenwoodCodecError {}

/// Pojedyncza zdekodowana ramka tekstowa protokołu Kenwood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KenwoodFrame {
    /// 2-literowy kod komendy (np. "FA", "MD", "TX", "IF") lub 4-znakowy nagłówek LAN (np. "##CN").
    pub command: String,
    /// Argument polecenia / payload bez nazwy komendy i średnika.
    pub argument: String,
    /// Oryginalny ciąg znaków wraz ze średnikiem.
    pub raw: String,
}

impl KenwoodFrame {
    /// Tworzy ramkę z surowego ciągu znaków ASCII zakończonego średnikiem.
    pub fn parse(raw: &str) -> Result<Self, KenwoodCodecError> {
        let trimmed = raw.trim();
        if !trimmed.ends_with(';') {
            return Err(KenwoodCodecError::MalformedFrame {
                raw: raw.to_string(),
            });
        }
        let content = trimmed.trim_end_matches(';').trim();
        if content.is_empty() {
            return Ok(Self {
                command: String::new(),
                argument: String::new(),
                raw: trimmed.to_string(),
            });
        }

        if content.starts_with("##") {
            let (cmd, arg) = if content.len() >= 4 {
                (&content[..4], &content[4..])
            } else {
                (content, "")
            };
            return Ok(Self {
                command: cmd.to_string(),
                argument: arg.to_string(),
                raw: trimmed.to_string(),
            });
        }

        let (cmd, arg) = if content.len() >= 2 {
            (&content[..2], &content[2..])
        } else {
            (content, "")
        };

        Ok(Self {
            command: cmd.to_string(),
            argument: arg.to_string(),
            raw: trimmed.to_string(),
        })
    }
}

/// Zdekodowane dane transceivera z ramki informacji `IF;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KenwoodIfInfo {
    /// Częstotliwość bieżącego VFO w Hz.
    pub frequency_hz: u64,
    /// Przesunięcie RIT w Hz (wartość ze znakiem).
    pub rit_offset_hz: i32,
    /// Flaga włączenia RIT (funkcja RIT aktywna).
    pub rit_on: bool,
    /// Flaga włączenia XIT (funkcja XIT aktywna).
    pub xit_on: bool,
    /// Stan nadawania PTT (true = TX, false = RX).
    pub tx: bool,
    /// Emisja (np. "USB", "LSB", "CW", "FM", "AM", "RTTY", "PSK").
    pub mode: &'static str,
    /// Wybór VFO (true = VFO B / Sub, false = VFO A / Main).
    pub vfo_b: bool,
    /// Stan pracy Split (true = Split włączony).
    pub split: bool,
}

impl KenwoodIfInfo {
    /// Dekoduje odpowiedź na rozkaz `IF;` (zgodnie ze specyfikacją TS-590S/SG, TS-890S, TS-990S).
    /// Format: `IF[11 cyfr Hz][5 znaków pamięć/krok][znak i 4 cyfry RIT][RIT on][XIT on][3 cyfry mem][TX/RX][Mode][VFO][Scan][Split]...;`
    pub fn parse(resp: &str) -> Option<Self> {
        let trimmed = resp.trim().trim_end_matches(';');
        if !trimmed.starts_with("IF") || trimmed.len() < 33 {
            return None;
        }

        // Częstotliwość: 11 cyfr od indeksu 2 do 13
        let freq_str = &trimmed[2..13];
        let frequency_hz = freq_str.trim().parse::<u64>().ok()?;

        // RIT offset: indeks 18 do 23 (np. "+0000" lub "-0150")
        let rit_offset_str = &trimmed[18..23];
        let rit_offset_hz = rit_offset_str.trim().parse::<i32>().unwrap_or(0);

        let rit_on = trimmed.chars().nth(23)? == '1';
        let xit_on = trimmed.chars().nth(24)? == '1';

        // Stan TX/RX: indeks 28 ('0' = RX, '1' = TX, '2' = TX)
        let tx_char = trimmed.chars().nth(28)?;
        let tx = tx_char == '1' || tx_char == '2';

        // Emisja: indeks 29
        let mode = match trimmed.chars().nth(29)? {
            '1' => "LSB",
            '3' => "CW",
            '4' => "FM",
            '5' => "AM",
            '6' => "RTTY",
            '7' => "CW-R",
            '8' => "RTTY-R",
            '9' => "PSK",
            _ => "USB",
        };

        // Funkcja VFO: indeks 30 ('0' = VFO A, '1' = VFO B, '2' = Memory)
        let vfo_b = trimmed.chars().nth(30)? == '1';

        // Split: indeks 32 ('0' = Off, '1' = On)
        let split = trimmed.chars().nth(32)? == '1';

        Some(Self {
            frequency_hz,
            rit_offset_hz,
            rit_on,
            xit_on,
            tx,
            mode,
            vfo_b,
            split,
        })
    }
}

/// Strumieniowy kodek protokołu Kenwood CAT.
#[derive(Debug, Clone)]
pub struct KenwoodCodec {
    max_frame_len: usize,
}

impl Default for KenwoodCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl KenwoodCodec {
    /// Tworzy kodek ze standardowym limitem długości ramki.
    pub fn new() -> Self {
        Self {
            max_frame_len: KENWOOD_DEFAULT_MAX_FRAME_LEN,
        }
    }

    /// Tworzy kodek z własnym limitem długości ramki.
    pub fn with_max_frame_len(max_frame_len: usize) -> Self {
        Self { max_frame_len }
    }

    /// Dekoduje kolejną ramkę z bufora bajtów.
    /// Zwraca `Ok(Some((ramka, liczba_zużytych_bajtów)))` gdy w buforze znajduje się kompletna ramka,
    /// `Ok(None)` gdy bufor nie zawiera jeszcze znaku `;`,
    /// lub `Err(...)` w przypadku przekroczenia limitu ramki lub błędnego kodowania.
    pub fn decode_frame(
        &self,
        buffer: &[u8],
    ) -> Result<Option<(KenwoodFrame, usize)>, KenwoodCodecError> {
        if buffer.is_empty() {
            return Ok(None);
        }

        // Pomiń ewentualne puste białe znaki wiodące przed początkiem ramki
        let mut start_idx = 0;
        while start_idx < buffer.len()
            && (buffer[start_idx] == b'\r'
                || buffer[start_idx] == b'\n'
                || buffer[start_idx] == b' ')
        {
            start_idx += 1;
        }

        let slice = &buffer[start_idx..];
        if slice.is_empty() {
            return Ok(None);
        }

        // Wyszukaj średnik ';' w buforze
        if let Some(pos) = slice.iter().position(|&b| b == b';') {
            let frame_bytes_len = pos + 1;
            let total_consumed = start_idx + frame_bytes_len;

            if frame_bytes_len > self.max_frame_len {
                return Err(KenwoodCodecError::FrameTooLarge {
                    len: frame_bytes_len,
                    max: self.max_frame_len,
                });
            }

            let frame_slice = &slice[..frame_bytes_len];
            let raw_str =
                std::str::from_utf8(frame_slice).map_err(|e| KenwoodCodecError::InvalidUtf8 {
                    detail: e.to_string(),
                })?;

            let frame = KenwoodFrame::parse(raw_str)?;
            Ok(Some((frame, total_consumed)))
        } else {
            // Brak średnika — sprawdź czy bufor nie przekracza dopuszczalnej długości ramki
            if slice.len() >= self.max_frame_len {
                return Err(KenwoodCodecError::FrameTooLarge {
                    len: slice.len(),
                    max: self.max_frame_len,
                });
            }
            Ok(None)
        }
    }

    /// Koduje tekst komendy do wektora bajtów ASCII z upewnieniem się, że kończy się średnikiem.
    pub fn encode_command(&self, cmd: &str) -> Vec<u8> {
        let trimmed = cmd.trim();
        if trimmed.ends_with(';') {
            trimmed.as_bytes().to_vec()
        } else {
            format!("{trimmed};").into_bytes()
        }
    }
}

/// Narzędzia do kodowania i dekodowania ramek protokołu Kenwood CAT.
pub struct KenwoodCat;

impl KenwoodCat {
    /// Tworzy komendę odczytu częstotliwości VFO A (`FA;`) lub VFO B (`FB;`).
    pub fn read_freq_cmd(vfo_b: bool) -> &'static str {
        if vfo_b { "FB;" } else { "FA;" }
    }

    /// Tworzy komendę ustawienia częstotliwości (11 cyfr w Hz).
    /// Np. 14.074.000 Hz -> `FA00014074000;`
    pub fn set_freq_cmd(vfo_b: bool, hz: u64) -> String {
        let prefix = if vfo_b { "FB" } else { "FA" };
        format!("{prefix}{hz:011};")
    }

    /// Dekoduje odpowiedź na zapytanie o częstotliwość (`FA00014074000;` lub `FB00014074000;`).
    pub fn parse_freq_resp(resp: &str) -> Option<(bool, u64)> {
        let trimmed = resp.trim().trim_end_matches(';');
        if trimmed.starts_with("FA") && trimmed.len() >= 13 {
            let hz = trimmed[2..].parse::<u64>().ok()?;
            Some((false, hz))
        } else if trimmed.starts_with("FB") && trimmed.len() >= 13 {
            let hz = trimmed[2..].parse::<u64>().ok()?;
            Some((true, hz))
        } else {
            None
        }
    }

    /// Tworzy komendę odczytu bieżącej emisji (`MD;`).
    pub fn read_mode_cmd() -> &'static str {
        "MD;"
    }

    /// Tworzy komendę ustawienia emisji:
    /// 1: LSB, 2: USB, 3: CW, 4: FM, 5: AM, 6: FSK/RTTY, 7: CW-R, 8: FSK-R, 9: PSK
    pub fn set_mode_cmd(mode: &str) -> Option<String> {
        let code = match mode.to_ascii_uppercase().as_str() {
            "LSB" => 1,
            "USB" | "FT8" | "FT4" | "JS8" | "DATA" | "DIGI" => 2,
            "CW" => 3,
            "FM" => 4,
            "AM" => 5,
            "FSK" | "RTTY" => 6,
            "CWR" | "CW-R" => 7,
            "FSKR" | "FSK-R" | "RTTYR" => 8,
            "PSK" => 9,
            _ => return None,
        };
        Some(format!("MD{code};"))
    }

    /// Dekoduje odpowiedź emisji (`MD1;` -> `"LSB"` itp.).
    pub fn parse_mode_resp(resp: &str) -> Option<&'static str> {
        let trimmed = resp.trim().trim_end_matches(';');
        if !trimmed.starts_with("MD") || trimmed.len() < 3 {
            return None;
        }
        match trimmed.chars().nth(2)? {
            '1' => Some("LSB"),
            '2' => Some("USB"),
            '3' => Some("CW"),
            '4' => Some("FM"),
            '5' => Some("AM"),
            '6' => Some("RTTY"),
            '7' => Some("CW-R"),
            '8' => Some("RTTY-R"),
            '9' => Some("PSK"),
            _ => None,
        }
    }

    /// Komenda załączenia / rozłączenia nadawania PTT (`TX;` / `RX;`).
    pub fn set_ptt_cmd(ptt: bool) -> &'static str {
        if ptt { "TX;" } else { "RX;" }
    }

    /// Komenda odczytu stanu PTT (`TX;` -> zwraca `TX0;`, `TX1;` lub `TX2;`).
    pub fn read_ptt_cmd() -> &'static str {
        "TX;"
    }

    /// Dekoduje stan PTT z odpowiedzi (`TX0;` = RX, `TX1;` lub `TX2;` = TX).
    pub fn parse_ptt_resp(resp: &str) -> Option<bool> {
        let trimmed = resp.trim().trim_end_matches(';');
        if trimmed == "TX" || trimmed == "TX1" || trimmed == "TX2" {
            Some(true)
        } else if trimmed == "RX" || trimmed == "TX0" {
            Some(false)
        } else {
            None
        }
    }

    /// Tworzy komendę odczytu S-metra (`SM;` lub `SM0;`).
    pub fn read_smeter_cmd() -> &'static str {
        "SM0;"
    }

    /// Dekoduje wartość S-metra z `SM00015;` lub `SM0015;` (zwraca surową wartość 0..30).
    pub fn parse_smeter_resp(resp: &str) -> Option<u8> {
        let trimmed = resp.trim().trim_end_matches(';');
        if let Some(rest) = trimmed.strip_prefix("SM") {
            let digits = rest.trim();
            let meter_str = if digits.len() == 5 && digits.is_ascii() {
                &digits[1..]
            } else {
                digits
            };
            meter_str.parse::<u8>().ok()
        } else {
            None
        }
    }

    /// Włącza/wyłącza tryb Split (`FT1;` dla TX na VFO B, `FT0;` dla wyłączenia).
    pub fn set_split_cmd(split: bool) -> &'static str {
        if split { "FT1;" } else { "FT0;" }
    }

    /// Tworzy komendę odczytu stanu Split (`FT;`).
    pub fn read_split_cmd() -> &'static str {
        "FT;"
    }

    /// Dekoduje odpowiedź stanu Split (`FT0;` = false, `FT1;` = true).
    pub fn parse_split_resp(resp: &str) -> Option<bool> {
        let trimmed = resp.trim().trim_end_matches(';');
        if trimmed == "FT1" {
            Some(true)
        } else if trimmed == "FT0" {
            Some(false)
        } else {
            None
        }
    }

    /// Tworzy komendę odczytu informacji transceivera (`IF;`).
    pub fn read_if_cmd() -> &'static str {
        "IF;"
    }

    /// Tworzy komendę odczytu identyfikatora modelu (`ID;`).
    pub fn read_id_cmd() -> &'static str {
        "ID;"
    }

    /// Tworzy komendę ustawienia poziomu Auto Information (`AI0;`, `AI1;`, `AI2;`).
    pub fn set_ai_cmd(level: u8) -> String {
        format!("AI{level};")
    }

    /// Tworzy komendę odczytu statusu Auto Information (`AI;`).
    pub fn read_ai_cmd() -> &'static str {
        "AI;"
    }

    /// Konwertuje surową wartość S-metra Kenwood (0..30) na wartość w dBm oraz czytelną etykietę (np. "S9", "S9+20dB").
    pub fn smeter_to_dbm_and_unit(raw: u8) -> (f32, String) {
        if raw <= 15 {
            let s_unit = ((raw as f32 / 15.0) * 9.0).round() as u8;
            let dbm = -127.0 + (s_unit as f32 * 6.0);
            (dbm, format!("S{s_unit}"))
        } else {
            let over_db = (((raw - 15) as f32 / 15.0) * 60.0).round() as u8;
            let dbm = -73.0 + over_db as f32;
            (dbm, format!("S9+{over_db}dB"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kenwood_freq_roundtrip() {
        let cmd = KenwoodCat::set_freq_cmd(false, 14_074_000);
        assert_eq!(cmd, "FA00014074000;");

        let parsed = KenwoodCat::parse_freq_resp("FA00014074000;");
        assert_eq!(parsed, Some((false, 14_074_000)));

        let parsed_b = KenwoodCat::parse_freq_resp("FB00007030000;");
        assert_eq!(parsed_b, Some((true, 7_030_000)));
    }

    #[test]
    fn test_kenwood_mode_roundtrip() {
        assert_eq!(KenwoodCat::set_mode_cmd("USB"), Some("MD2;".to_string()));
        assert_eq!(KenwoodCat::set_mode_cmd("CW"), Some("MD3;".to_string()));
        assert_eq!(KenwoodCat::set_mode_cmd("RTTY"), Some("MD6;".to_string()));
        assert_eq!(KenwoodCat::set_mode_cmd("PSK"), Some("MD9;".to_string()));
        assert_eq!(KenwoodCat::set_mode_cmd("UNKNOWN"), None);

        assert_eq!(KenwoodCat::parse_mode_resp("MD2;"), Some("USB"));
        assert_eq!(KenwoodCat::parse_mode_resp("MD3;"), Some("CW"));
        assert_eq!(KenwoodCat::parse_mode_resp("MD1;"), Some("LSB"));
        assert_eq!(KenwoodCat::parse_mode_resp("MD9;"), Some("PSK"));
        assert_eq!(KenwoodCat::parse_mode_resp("MD0;"), None);
    }

    #[test]
    fn test_kenwood_ptt() {
        assert_eq!(KenwoodCat::set_ptt_cmd(true), "TX;");
        assert_eq!(KenwoodCat::set_ptt_cmd(false), "RX;");

        assert_eq!(KenwoodCat::parse_ptt_resp("TX1;"), Some(true));
        assert_eq!(KenwoodCat::parse_ptt_resp("TX2;"), Some(true));
        assert_eq!(KenwoodCat::parse_ptt_resp("TX0;"), Some(false));
        assert_eq!(KenwoodCat::parse_ptt_resp("RX;"), Some(false));
    }

    #[test]
    fn test_kenwood_smeter() {
        assert_eq!(KenwoodCat::read_smeter_cmd(), "SM0;");
        assert_eq!(KenwoodCat::parse_smeter_resp("SM00015;"), Some(15));
        assert_eq!(KenwoodCat::parse_smeter_resp("SM10015;"), Some(15));
        assert_eq!(KenwoodCat::parse_smeter_resp("SM0015;"), Some(15));
        assert_eq!(KenwoodCat::parse_smeter_resp("INVALID;"), None);
    }

    #[test]
    fn test_kenwood_split_roundtrip() {
        assert_eq!(KenwoodCat::set_split_cmd(true), "FT1;");
        assert_eq!(KenwoodCat::set_split_cmd(false), "FT0;");

        assert_eq!(KenwoodCat::parse_split_resp("FT1;"), Some(true));
        assert_eq!(KenwoodCat::parse_split_resp("FT0;"), Some(false));
        assert_eq!(KenwoodCat::parse_split_resp("FT2;"), None);
    }

    #[test]
    fn test_kenwood_codec_fragmented_stream() {
        let codec = KenwoodCodec::new();
        let mut buffer = Vec::new();

        // 1. Część pierwsza: brak średnika
        buffer.extend_from_slice(b"FA000140");
        let res = codec.decode_frame(&buffer).expect("decode partial");
        assert_eq!(res, None);

        // 2. Część druga: nadal brak średnika
        buffer.extend_from_slice(b"74000");
        let res = codec.decode_frame(&buffer).expect("decode partial 2");
        assert_eq!(res, None);

        // 3. Dosłanie średnika
        buffer.extend_from_slice(b";");
        let (frame, consumed) = codec
            .decode_frame(&buffer)
            .expect("decode complete")
            .expect("frame");
        assert_eq!(frame.command, "FA");
        assert_eq!(frame.argument, "00014074000");
        assert_eq!(frame.raw, "FA00014074000;");
        assert_eq!(consumed, buffer.len());
    }

    #[test]
    fn test_kenwood_codec_multiple_frames_in_one_buffer() {
        let codec = KenwoodCodec::new();
        let data = b"FA00014074000;MD2;SM00015;";
        let mut slice = &data[..];

        // 1. Pierwsza ramka: FA
        let (frame1, n1) = codec
            .decode_frame(slice)
            .expect("frame 1")
            .expect("frame 1 exists");
        assert_eq!(frame1.command, "FA");
        assert_eq!(frame1.argument, "00014074000");
        slice = &slice[n1..];

        // 2. Druga ramka: MD
        let (frame2, n2) = codec
            .decode_frame(slice)
            .expect("frame 2")
            .expect("frame 2 exists");
        assert_eq!(frame2.command, "MD");
        assert_eq!(frame2.argument, "2");
        slice = &slice[n2..];

        // 3. Trzecia ramka: SM
        let (frame3, n3) = codec
            .decode_frame(slice)
            .expect("frame 3")
            .expect("frame 3 exists");
        assert_eq!(frame3.command, "SM");
        assert_eq!(frame3.argument, "00015");
        slice = &slice[n3..];

        // Koniec
        assert!(slice.is_empty());
        let res = codec.decode_frame(slice).expect("empty");
        assert_eq!(res, None);
    }

    #[test]
    fn test_kenwood_codec_partial_trailing_message() {
        let codec = KenwoodCodec::new();
        let data = b"FA00014074000;MD";

        let (frame, consumed) = codec
            .decode_frame(data)
            .expect("frame 1")
            .expect("frame 1 exists");
        assert_eq!(frame.command, "FA");
        let rest = &data[consumed..];
        assert_eq!(rest, b"MD");
        assert_eq!(codec.decode_frame(rest).expect("partial"), None);
    }

    #[test]
    fn test_kenwood_codec_max_frame_length_exceeded() {
        let codec = KenwoodCodec::with_max_frame_len(16);
        let long_data = b"FA0001407400000000000000;";

        let err = codec.decode_frame(long_data).unwrap_err();
        match err {
            KenwoodCodecError::FrameTooLarge { len, max } => {
                assert!(len > max);
                assert_eq!(max, 16);
            }
            _ => panic!("expected FrameTooLarge, got {err:?}"),
        }
    }

    #[test]
    fn test_kenwood_codec_invalid_utf8() {
        let codec = KenwoodCodec::new();
        let bad_data = [0xFF, 0xFE, b';'];

        let err = codec.decode_frame(&bad_data).unwrap_err();
        match err {
            KenwoodCodecError::InvalidUtf8 { .. } => {}
            _ => panic!("expected InvalidUtf8, got {err:?}"),
        }
    }

    #[test]
    fn test_kenwood_codec_empty_frames_and_whitespace() {
        let codec = KenwoodCodec::new();
        let buffer = b"\r\n  FA00014074000;\r\n;";

        let (frame1, n1) = codec
            .decode_frame(buffer)
            .expect("frame 1")
            .expect("frame 1");
        assert_eq!(frame1.command, "FA");
        assert_eq!(frame1.argument, "00014074000");

        let rest = &buffer[n1..];
        let (frame2, _) = codec.decode_frame(rest).expect("frame 2").expect("frame 2");
        assert_eq!(frame2.command, "");
    }

    #[test]
    fn test_kenwood_codec_ai_events() {
        let codec = KenwoodCodec::new();
        for code in ["AI0;", "AI1;", "AI2;"] {
            let (frame, _) = codec
                .decode_frame(code.as_bytes())
                .expect("ai")
                .expect("ai frame");
            assert_eq!(frame.command, "AI");
            assert_eq!(format!("AI{};", frame.argument), code);
        }
    }

    #[test]
    fn test_kenwood_codec_lan_frames() {
        let codec = KenwoodCodec::new();
        let lan_data = b"##CN0;##ID1;##BY0;";
        let mut slice = &lan_data[..];

        let (f1, n1) = codec.decode_frame(slice).expect("f1").expect("f1");
        assert_eq!(f1.command, "##CN");
        assert_eq!(f1.argument, "0");
        slice = &slice[n1..];

        let (f2, n2) = codec.decode_frame(slice).expect("f2").expect("f2");
        assert_eq!(f2.command, "##ID");
        assert_eq!(f2.argument, "1");
        slice = &slice[n2..];

        let (f3, _) = codec.decode_frame(slice).expect("f3").expect("f3");
        assert_eq!(f3.command, "##BY");
        assert_eq!(f3.argument, "0");
    }

    #[test]
    fn test_kenwood_if_parser_ts590() {
        // TS-590 format: 38 chars: IF + 11B freq + 5B mem + 5B rit + rit_on + xit_on + mem3 + tx + mode + vfo + scan + split + tone + tonenum2 + ctcss + ;
        // Częstotliwość: 14.074.000 Hz, RIT: +0000, RIT: off, XIT: off, TX: RX (0), Mode: USB (2), VFO: A (0), Split: off (0)
        let resp = "IF0001407400000000+0000000000200000000;";
        let info = KenwoodIfInfo::parse(resp).expect("parse TS-590 IF");

        assert_eq!(info.frequency_hz, 14_074_000);
        assert_eq!(info.rit_offset_hz, 0);
        assert!(!info.rit_on);
        assert!(!info.xit_on);
        assert!(!info.tx);
        assert_eq!(info.mode, "USB");
        assert!(!info.vfo_b);
        assert!(!info.split);
    }

    #[test]
    fn test_kenwood_if_parser_ts890_ts990() {
        // TS-890/TS-990 format z aktywnym Split (1), TX (1), CW (3), VFO B (1), RIT +150Hz włączonym (1)
        let resp = "IF0000703000000000+0150100001310100000;";
        let info = KenwoodIfInfo::parse(resp).expect("parse TS-890/990 IF");

        assert_eq!(info.frequency_hz, 7_030_000);
        assert_eq!(info.rit_offset_hz, 150);
        assert!(info.rit_on);
        assert!(!info.xit_on);
        assert!(info.tx);
        assert_eq!(info.mode, "CW");
        assert!(info.vfo_b);
        assert!(info.split);
    }

    #[test]
    fn test_kenwood_if_parser_malformed() {
        assert_eq!(KenwoodIfInfo::parse("IF000140;"), None);
        assert_eq!(KenwoodIfInfo::parse("FA00014074000;"), None);
        assert_eq!(KenwoodIfInfo::parse(""), None);
    }
}
