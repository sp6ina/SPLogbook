// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Strumieniowy kodek protokołu Yaesu CAT.
//!
//! Obsługuje ramkowanie ASCII ze znakiem podziału `;`, dekodowanie
//! z buforów asynchronicznych, wykrywanie fragmentacji, ramki błędów `?;`,
//! komendy Auto Information (`AI`), odczyt mierników (`SM`), raport stanu (`IF`)
//! oraz podział na rodziny protokołów Yaesu (współczesną ASCII i klasyczną binarną).
//!
//! Standard ASCII stosowany przez transceivery:
//! - FT-891, FT-991, FT-991A
//! - FT-710, FTDX10, FTDX101D, FTDX101MP
//! - FT-450, FT-450D, FT-950, FT-2000, FTDX-3000, FTDX-5000, FTDX-9000

/// Domyślny maksymalny dozwolony rozmiar pojedynczej ramki Yaesu CAT (w bajtach).
pub const YAESU_DEFAULT_MAX_FRAME_LEN: usize = 256;

/// Rodzina protokołu Yaesu CAT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YaesuProtocolFamily {
    /// Nowoczesny protokół tekstowy ASCII zakończony średnikiem ';' (FT-891, FT-991A, FT-710, FTDX10 itp.).
    Ascii,
    /// Klasyczny 5-bajtowy protokół binarny (FT-817, FT-857, FT-897) - zarezerwowany dla profili legacy.
    LegacyBinary,
}

/// Błędy dekodowania strumieniowego protokołu Yaesu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YaesuCodecError {
    /// Przekroczono maksymalną dozwoloną długość pojedynczej ramki bez średnika.
    FrameTooLarge { len: usize, max: usize },
    /// Ramka zawiera niepoprawne bajty UTF-8.
    InvalidUtf8 { detail: String },
    /// Ramka jest uszkodzona lub ma nieznaną strukturę.
    MalformedFrame { raw: String },
}

impl std::fmt::Display for YaesuCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameTooLarge { len, max } => {
                write!(f, "Yaesu CAT frame exceeded max size ({len} > {max})")
            }
            Self::InvalidUtf8 { detail } => {
                write!(f, "Yaesu CAT frame is not valid UTF-8: {detail}")
            }
            Self::MalformedFrame { raw } => {
                write!(f, "malformed Yaesu CAT frame: {raw}")
            }
        }
    }
}

impl std::error::Error for YaesuCodecError {}

/// Pojedyncza zdekodowana ramka tekstowa protokołu Yaesu CAT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YaesuFrame {
    /// Kod komendy (np. "FA", "FB", "MD", "TX", "SM", "FT", "IF", "?").
    pub command: String,
    /// Argument polecenia / payload bez nazwy komendy i średnika.
    pub argument: String,
    /// Oryginalny ciąg znaków wraz ze średnikiem.
    pub raw: String,
}

impl YaesuFrame {
    /// Tworzy ramkę z surowego ciągu znaków ASCII zakończonego średnikiem.
    pub fn parse(raw: &str) -> Result<Self, YaesuCodecError> {
        let trimmed = raw.trim();
        if !trimmed.ends_with(';') {
            return Err(YaesuCodecError::MalformedFrame {
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

        // Obsługa ramki błędu "?;" zwracanej przez transceivery Yaesu
        if content == "?" {
            return Ok(Self {
                command: "?".to_string(),
                argument: String::new(),
                raw: trimmed.to_string(),
            });
        }

        // Obsługa komend 2-literowych (np. "FA", "FB", "TX", "SM", "MD", "FT", "IF", "AI", "RT", "XT")
        // oraz 3-znakowych wariantów (np. "MD0", "SM0")
        let (cmd, arg) = if content.starts_with("MD0") || content.starts_with("SM0") {
            (&content[..3], &content[3..])
        } else if content.len() >= 2 {
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

    /// Sprawdza, czy ramka jest odpowiedzią błędu `?;`.
    pub fn is_error(&self) -> bool {
        self.command == "?" || self.raw.trim() == "?;"
    }
}

/// Zdekodowane dane transceivera z ramki informacji o stanie `IF;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YaesuIfInfo {
    /// Częstotliwość bieżącego VFO w Hz.
    pub frequency_hz: u64,
    /// Przesunięcie Clarifier/RIT w Hz (wartość ze znakiem).
    pub rit_offset_hz: i32,
    /// Flaga włączenia RIT (funkcja RIT aktywna).
    pub rit_on: bool,
    /// Flaga włączenia XIT (funkcja XIT aktywna).
    pub xit_on: bool,
    /// Stan nadawania PTT (true = TX, false = RX).
    pub tx: bool,
    /// Emisja (np. "USB", "LSB", "CW", "FM", "AM", "RTTY", "DATA-U", "PSK").
    pub mode: &'static str,
    /// Wybór VFO (true = VFO B / Sub, false = VFO A / Main).
    pub vfo_b: bool,
    /// Flaga włączenia pracy Split.
    pub split: bool,
}

impl YaesuIfInfo {
    /// Parsuje surową odpowiedź Yaesu na komendę `IF;`.
    pub fn parse(raw: &str) -> Option<Self> {
        Self::parse_for_model("", raw)
    }

    /// Parsuje surową odpowiedź Yaesu na komendę `IF;` z uwzględnieniem specyfiki danego modelu.
    ///
    /// Typowa struktura ramki Yaesu `IF;` (zgodna z FT-891, FT-991A, FT-710, FTDX10):
    /// `IF[P1:3][P2:9][P3:5][P4:1][P5:1][P6:1][P7:1][P8:1][P9:1][P10:1][P11:1][P12:1];`
    /// - P1 (znaki 2..5): numer kanału pamięci (3 cyfry, np. 000)
    /// - P2 (znaki 5..14): częstotliwość VFO w Hz (9 cyfr)
    /// - P3 (znaki 14..19): kierunek i przesunięcie Clarifier/RIT (+/- 4 cyfry, np. +0000)
    /// - P4 (znak 19): RIT status (0 = OFF, 1 = ON)
    /// - P5 (znak 20): XIT status (0 = OFF, 1 = ON)
    /// - P6 (znak 21): Memory bank / dummy
    /// - P7 (znak 22): VFO/Memory indicator (0 = VFO-A, 1 = VFO-B)
    /// - P8 (znak 23): tryb emisji (kod Yaesu MD: 1..9, A..E)
    /// - P9 (znak 24): stan nadawania (0 = RX, 1 = TX) lub stan skanera
    /// - P10 (znak 25): status Split (0 = OFF, 1 = ON)
    pub fn parse_for_model(model_id: &str, raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if !trimmed.starts_with("IF") || !trimmed.ends_with(';') {
            return None;
        }

        let body = trimmed.strip_prefix("IF")?.strip_suffix(';')?;
        if body.len() < 24 {
            return None;
        }

        let chars: Vec<char> = body.chars().collect();

        // 1. Częstotliwość P2 (9 cyfr, indeksy 3..12)
        if chars.len() < 12 {
            return None;
        }
        let freq_str: String = chars[3..12].iter().collect();
        if !freq_str.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let frequency_hz = freq_str.trim().parse::<u64>().ok()?;

        // 2. RIT offset P3 (5 znaków, indeksy 12..17)
        let mut rit_offset_hz = 0i32;
        if chars.len() >= 17 {
            let rit_str: String = chars[12..17].iter().collect();
            let sign = if rit_str.starts_with('-') { -1 } else { 1 };
            let digits_part = rit_str.trim_start_matches(['+', '-']).trim();
            if let Ok(val) = digits_part.parse::<i32>() {
                rit_offset_hz = sign * val;
            }
        }

        // 3. RIT on P4 (indeks 17)
        let rit_on = chars.get(17).copied() == Some('1');

        // 4. XIT on P5 (indeks 18)
        let xit_on = chars.get(18).copied() == Some('1');

        // 5. VFO selection P7 (indeks 20)
        let vfo_b = chars.get(20).copied() == Some('1');

        // 6. Mode P8 (indeks 21) — brak cichego fallbacku do USB
        let mode_char = chars.get(21).copied()?;
        let mode = if model_id.is_empty() {
            YaesuCat::map_mode_code_opt(mode_char)?
        } else {
            YaesuCat::map_mode_code_for_model(model_id, mode_char)?
        };

        // 7. TX status P9 (indeks 22)
        let tx = chars.get(22).copied() == Some('1');

        // 8. Split P10 (indeks 23)
        let split = chars.get(23).copied() == Some('1');

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

/// Statyczne narzędzia kodowania i parsowania poleceń Yaesu CAT.
pub struct YaesuCat;

impl YaesuCat {
    /// Format polecenia ustawienia częstotliwości VFO A lub B w Hz ze wskazaną liczbą cyfr.
    /// Odrzuca wartości, które nie mieszczą się w zadanym rozmiarze bez obcinania najstarszych cyfr.
    pub fn format_freq_checked(vfo_b: bool, freq_hz: u64, digits: usize) -> Option<String> {
        if digits == 0 {
            return None;
        }
        let s = freq_hz.to_string();
        if s.len() > digits {
            return None;
        }
        let cmd = if vfo_b { "FB" } else { "FA" };
        Some(format!("{cmd}{freq_hz:0digits$};"))
    }

    /// Format polecenia ustawienia częstotliwości VFO A lub B w Hz.
    /// Standard Yaesu ASCII używa 9 cyfr Hz (np. "FA014074000;").
    pub fn set_freq(vfo_b: bool, freq_hz: u64) -> String {
        Self::format_freq_checked(vfo_b, freq_hz, 9).unwrap_or_else(|| {
            let cmd = if vfo_b { "FB" } else { "FA" };
            format!("{cmd}{freq_hz:09};")
        })
    }

    /// Polecenie odczytu częstotliwości VFO A lub B.
    pub fn read_freq(vfo_b: bool) -> &'static str {
        if vfo_b { "FB;" } else { "FA;" }
    }

    /// Format polecenia ustawienia emisji transceivera dla danego profilu modelu.
    pub fn set_mode_for_model(model_id: &str, mode: &str) -> Option<String> {
        let code = match mode.to_ascii_uppercase().as_str() {
            "LSB" => '1',
            "USB" => '2',
            "CW" | "CW-U" => '3',
            "FM" => '4',
            "AM" => '5',
            "RTTY" | "RTTY-L" | "RTTY-LSB" | "FSK" => '6',
            "CW-R" | "CW-L" => '7',
            "DATA-L" | "DATA-LSB" | "PKT-LSB" => '8',
            "RTTY-R" | "RTTY-U" | "RTTY-USB" | "FSK-R" => '9',
            "DATA-FM" => 'A',
            "FM-N" => 'B',
            "DATA" | "DATA-U" | "DATA-USB" | "PKT-USB" | "FT8" => 'C',
            "AM-N" => 'D',
            "PSK" => {
                let id = model_id.trim().to_ascii_uppercase();
                // PSK jest wspierane m.in. na FT-710 i FTDX10 (kod 'E'), ale nie na FT-891
                if id.contains("891") {
                    return None;
                }
                'E'
            }
            "C4FM" => {
                let id = model_id.trim().to_ascii_uppercase();
                if id.contains("991") {
                    'E'
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        Some(format!("MD0{code};"))
    }

    /// Format polecenia ustawienia emisji transceivera.
    /// Yaesu stosuje polecenie `MD0<kod>;` (np. `MD02;` dla USB, `MD0C;` dla DATA-U).
    pub fn set_mode(mode: &str) -> Option<String> {
        Self::set_mode_for_model("", mode)
    }

    /// Polecenie odczytu bieżącej emisji transceivera.
    pub fn read_mode() -> &'static str {
        "MD0;"
    }

    /// Format polecenia ustawienia PTT (nadawania).
    /// `TX1;` załącza nadawanie (general/mic TX), `TX0;` przechodzi do odbioru (RX).
    pub fn set_ptt(tx: bool) -> &'static str {
        if tx { "TX1;" } else { "TX0;" }
    }

    /// Polecenie odczytu stanu PTT.
    pub fn read_ptt() -> &'static str {
        "TX;"
    }

    /// Format polecenia ustawienia trybu Split (`FT1;` = ON, `FT0;` = OFF).
    pub fn set_split(enabled: bool) -> &'static str {
        if enabled { "FT1;" } else { "FT0;" }
    }

    /// Polecenie odczytu stanu Split.
    pub fn read_split() -> &'static str {
        "FT;"
    }

    /// Polecenie odczytu miernika siły sygnału S-meter (`SM0;`).
    pub fn read_smeter() -> &'static str {
        "SM0;"
    }

    /// Polecenie załączenia lub wyłączenia trybu Auto Information (`AI1;` / `AI0;`).
    pub fn set_ai(enabled: bool) -> &'static str {
        if enabled { "AI1;" } else { "AI0;" }
    }

    /// Polecenie odczytu stanu transceivera (`IF;`).
    pub fn read_if() -> &'static str {
        "IF;"
    }

    /// Bezpieczne mapowanie 1-znakowego kodu emisji Yaesu na ujednolicony napis emisji (brak fallbacku do USB).
    pub fn map_mode_code_opt(code: char) -> Option<&'static str> {
        match code {
            '1' => Some("LSB"),
            '2' => Some("USB"),
            '3' => Some("CW-U"),
            '4' => Some("FM"),
            '5' => Some("AM"),
            '6' => Some("RTTY-L"),
            '7' => Some("CW-L"),
            '8' => Some("DATA-L"),
            '9' => Some("RTTY-U"),
            'A' => Some("DATA-FM"),
            'B' => Some("FM-N"),
            'C' => Some("DATA-U"),
            'D' => Some("AM-N"),
            'E' => Some("PSK"),
            _ => None,
        }
    }

    /// Mapuje 1-znakowy kod emisji Yaesu dla konkretnego profilu modelu.
    pub fn map_mode_code_for_model(model_id: &str, code: char) -> Option<&'static str> {
        let id = model_id.trim().to_ascii_uppercase();
        if id.contains("891") {
            match code {
                '1' => Some("LSB"),
                '2' => Some("USB"),
                '3' => Some("CW-U"),
                '4' => Some("FM"),
                '5' => Some("AM"),
                '6' => Some("RTTY-L"),
                '7' => Some("CW-L"),
                '8' => Some("DATA-L"),
                '9' => Some("RTTY-U"),
                'A' => Some("DATA-FM"),
                'B' => Some("FM-N"),
                'C' => Some("DATA-U"),
                'D' => Some("AM-N"),
                _ => None,
            }
        } else if id.contains("991") {
            match code {
                '1' => Some("LSB"),
                '2' => Some("USB"),
                '3' => Some("CW-U"),
                '4' => Some("FM"),
                '5' => Some("AM"),
                '6' => Some("RTTY-L"),
                '7' => Some("CW-L"),
                '8' => Some("DATA-L"),
                '9' => Some("RTTY-U"),
                'A' => Some("DATA-FM"),
                'B' => Some("FM-N"),
                'C' => Some("DATA-U"),
                'D' => Some("AM-N"),
                'E' => Some("C4FM"),
                _ => None,
            }
        } else if id.contains("710") || id.contains("DX10") || id.contains("101") {
            match code {
                '1' => Some("LSB"),
                '2' => Some("USB"),
                '3' => Some("CW-U"),
                '4' => Some("FM"),
                '5' => Some("AM"),
                '6' => Some("RTTY-L"),
                '7' => Some("CW-L"),
                '8' => Some("DATA-L"),
                '9' => Some("RTTY-U"),
                'A' => Some("DATA-FM"),
                'B' => Some("FM-N"),
                'C' => Some("DATA-U"),
                'D' => Some("AM-N"),
                'E' => Some("PSK"),
                _ => None,
            }
        } else {
            Self::map_mode_code_opt(code)
        }
    }

    /// Mapuje 1-znakowy kod emisji Yaesu na ujednolicony napis emisji (dla kompatybilności wstecznej).
    pub fn map_mode_code(code: char) -> &'static str {
        Self::map_mode_code_opt(code).unwrap_or("USB")
    }

    /// Parsuje odpowiedź na odczyt częstotliwości (`FA<częstotliwość>;` lub `FB<częstotliwość>;`).
    /// Zwraca parę `(is_vfo_b, frequency_hz)`.
    pub fn parse_freq_resp(resp: &str) -> Option<(bool, u64)> {
        let trimmed = resp.trim();
        let (is_vfo_b, content) = if let Some(stripped) = trimmed.strip_prefix("FA") {
            (false, stripped)
        } else {
            (true, trimmed.strip_prefix("FB")?)
        };

        let digits = content.trim_end_matches(';').trim();
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let hz = digits.parse::<u64>().ok()?;
        Some((is_vfo_b, hz))
    }

    /// Parsuje odpowiedź na odczyt emisji (`MD0<kod>;` lub `MD<kod>;`).
    pub fn parse_mode_resp(resp: &str) -> Option<&'static str> {
        Self::parse_mode_resp_for_model("", resp)
    }

    /// Parsuje odpowiedź na odczyt emisji dla konkretnego profilu modelu.
    pub fn parse_mode_resp_for_model(model_id: &str, resp: &str) -> Option<&'static str> {
        let trimmed = resp.trim();
        let body = if let Some(s) = trimmed.strip_prefix("MD0") {
            s
        } else {
            trimmed.strip_prefix("MD")?
        };

        let code_char = body.trim_end_matches(';').chars().next()?;
        if model_id.is_empty() {
            Self::map_mode_code_opt(code_char)
        } else {
            Self::map_mode_code_for_model(model_id, code_char)
        }
    }

    /// Parsuje odpowiedź na odczyt stanu PTT (`TX0;` -> RX, `TX1;` lub `TX2;` -> TX).
    pub fn parse_ptt_resp(resp: &str) -> Option<bool> {
        let trimmed = resp.trim();
        let body = trimmed.strip_prefix("TX")?.trim_end_matches(';').trim();
        match body {
            "0" => Some(false),
            "1" | "2" => Some(true),
            _ => None,
        }
    }

    /// Parsuje odpowiedź na odczyt Split (`FT0;` -> OFF, `FT1;` -> ON).
    pub fn parse_split_resp(resp: &str) -> Option<bool> {
        let trimmed = resp.trim();
        let body = trimmed.strip_prefix("FT")?.trim_end_matches(';').trim();
        match body {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        }
    }

    /// Parsuje odpowiedź na odczyt S-metra (`SM0<digits>;` lub `SM<digits>;`).
    pub fn parse_smeter_resp(resp: &str) -> Option<u16> {
        let trimmed = resp.trim();
        let body = if let Some(s) = trimmed.strip_prefix("SM0") {
            s
        } else {
            trimmed.strip_prefix("SM")?
        };

        let digits = body.trim_end_matches(';').trim();
        digits.parse::<u16>().ok()
    }

    /// Przelicza surowy odczyt S-metra Yaesu (0..255) na etykietę jednostki S (np. "S0".."S9", "S9+10dB"..).
    /// Protokół Yaesu nie definiuje w oficjalnej specyfikacji CAT przeliczenia na dBm ani punktów
    /// progowych dla dBm, dlatego backend nie fabrykuje sztucznej wartości dBm.
    pub fn smeter_to_unit(raw: u16) -> String {
        let raw_val = raw.min(255);
        if raw_val <= 120 {
            let s_num = ((raw_val as f32 / 120.0) * 9.0).round() as u8;
            format!("S{s_num}")
        } else {
            let over_fraction = (raw_val - 120) as f32 / 135.0;
            let db_over = (over_fraction * 60.0).round() as u8;
            let step = (db_over / 10) * 10;
            format!("S9+{step}dB")
        }
    }

    /// Pomocnicza funkcja zachowana dla celów testowych/interoperacyjnych.
    /// Wartość dBm jest oznaczana jako przybliżenie referencyjne IARU (S9=-73dBm),
    /// a nie fabryczny parametr sprzętu.
    pub fn smeter_to_dbm_and_unit(raw: u16) -> (f32, String) {
        let unit = Self::smeter_to_unit(raw);
        let raw_val = raw.min(255);
        let dbm = if raw_val <= 120 {
            -127.0 + (raw_val as f32 / 120.0) * 54.0
        } else {
            -73.0 + ((raw_val - 120) as f32 / 135.0) * 60.0
        };
        (dbm, unit)
    }
}

/// Strumieniowy dekoder i enkoder ramek Yaesu CAT.
#[derive(Debug, Clone)]
pub struct YaesuAsciiCodec {
    max_frame_len: usize,
}

impl Default for YaesuAsciiCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl YaesuAsciiCodec {
    /// Tworzy nowy kodek z domyślnym limitem długości ramki.
    pub fn new() -> Self {
        Self {
            max_frame_len: YAESU_DEFAULT_MAX_FRAME_LEN,
        }
    }

    /// Tworzy kodek ze wskazanym limitem długości ramki.
    pub fn with_max_len(max_frame_len: usize) -> Self {
        Self { max_frame_len }
    }

    /// Koduje polecenie do wysłania, dbając o obecność terminatora `;`.
    pub fn encode_command(&self, cmd: &str) -> Vec<u8> {
        let trimmed = cmd.trim();
        if trimmed.ends_with(';') {
            trimmed.as_bytes().to_vec()
        } else {
            format!("{trimmed};").into_bytes()
        }
    }

    /// Próbuje zdekodować kompletną ramkę ze strumienia bajtów.
    ///
    /// Zwraca:
    /// - `Ok(Some((frame, consumed_bytes)))` — gdy znaleziono pełną ramkę zakończoną `;`,
    /// - `Ok(None)` — gdy bufor zawiera tylko fragment ramki i oczekuje na dalsze bajty,
    /// - `Err(YaesuCodecError)` — gdy bufor przekroczył limit bez średnika lub zawiera niepoprawny UTF-8.
    pub fn decode_frame(&self, buf: &[u8]) -> Result<Option<(YaesuFrame, usize)>, YaesuCodecError> {
        if buf.is_empty() {
            return Ok(None);
        }

        // Szukaj terminatora ';' w buforze
        if let Some(pos) = buf.iter().position(|&b| b == b';') {
            let frame_bytes = &buf[..=pos];
            let raw_str =
                std::str::from_utf8(frame_bytes).map_err(|e| YaesuCodecError::InvalidUtf8 {
                    detail: e.to_string(),
                })?;

            let frame = YaesuFrame::parse(raw_str)?;
            Ok(Some((frame, pos + 1)))
        } else if buf.len() > self.max_frame_len {
            Err(YaesuCodecError::FrameTooLarge {
                len: buf.len(),
                max: self.max_frame_len,
            })
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_command_ensures_semicolon() {
        let codec = YaesuAsciiCodec::new();
        assert_eq!(codec.encode_command("FA"), b"FA;");
        assert_eq!(codec.encode_command("FA;"), b"FA;");
        assert_eq!(codec.encode_command("MD02;"), b"MD02;");
        assert_eq!(codec.encode_command("TX0"), b"TX0;");
    }

    #[test]
    fn test_decode_single_frame() {
        let codec = YaesuAsciiCodec::new();
        let data = b"FA014074000;";
        let res = codec.decode_frame(data).expect("decode failed");
        let (frame, consumed) = res.expect("frame should be present");
        assert_eq!(consumed, data.len());
        assert_eq!(frame.command, "FA");
        assert_eq!(frame.argument, "014074000");
        assert_eq!(frame.raw, "FA014074000;");
        assert!(!frame.is_error());
    }

    #[test]
    fn test_decode_error_frame() {
        let codec = YaesuAsciiCodec::new();
        let data = b"?;";
        let res = codec.decode_frame(data).expect("decode failed");
        let (frame, consumed) = res.expect("frame should be present");
        assert_eq!(consumed, 2);
        assert_eq!(frame.command, "?");
        assert!(frame.is_error());
    }

    #[test]
    fn test_decode_multiple_frames_in_one_buffer() {
        let codec = YaesuAsciiCodec::new();
        let mut data = b"FA014074000;MD02;TX0;".to_vec();

        // 1. Pierwsza ramka FA
        let (f1, c1) = codec.decode_frame(&data).unwrap().unwrap();
        assert_eq!(f1.command, "FA");
        assert_eq!(f1.argument, "014074000");
        data.drain(..c1);

        // 2. Druga ramka MD0
        let (f2, c2) = codec.decode_frame(&data).unwrap().unwrap();
        assert_eq!(f2.command, "MD0");
        assert_eq!(f2.argument, "2");
        data.drain(..c2);

        // 3. Trzecia ramka TX
        let (f3, c3) = codec.decode_frame(&data).unwrap().unwrap();
        assert_eq!(f3.command, "TX");
        assert_eq!(f3.argument, "0");
        data.drain(..c3);

        assert!(data.is_empty());
    }

    #[test]
    fn test_fragmented_stream_handling() {
        let codec = YaesuAsciiCodec::new();
        let full = b"FA014074000;";
        let mut buf = Vec::new();

        // Wstrzykuj bajty jeden po drugim aż do przedostatniego
        for &b in &full[..full.len() - 1] {
            buf.push(b);
            let res = codec.decode_frame(&buf).unwrap();
            assert!(res.is_none(), "nie powinno być ramki bez średnika");
        }

        // Dodaj ostatni bajt ';'
        buf.push(b';');
        let (frame, consumed) = codec.decode_frame(&buf).unwrap().unwrap();
        assert_eq!(consumed, full.len());
        assert_eq!(frame.command, "FA");
        assert_eq!(frame.argument, "014074000");
    }

    #[test]
    fn test_frame_exceeds_max_length() {
        let codec = YaesuAsciiCodec::with_max_len(16);
        let long_stream = b"FA01234567890123456789"; // 22 bajty bez ';'
        let err = codec.decode_frame(long_stream).unwrap_err();
        match err {
            YaesuCodecError::FrameTooLarge { len, max } => {
                assert_eq!(len, 22);
                assert_eq!(max, 16);
            }
            other => panic!("nieoczekiwany błąd: {other:?}"),
        }
    }

    #[test]
    fn test_invalid_utf8_rejected() {
        let codec = YaesuAsciiCodec::new();
        let invalid = [0xFF, 0xFE, b';'];
        let err = codec.decode_frame(&invalid).unwrap_err();
        assert!(matches!(err, YaesuCodecError::InvalidUtf8 { .. }));
    }

    #[test]
    fn test_frequency_roundtrip() {
        let cmd = YaesuCat::set_freq(false, 14_074_000);
        assert_eq!(cmd, "FA014074000;");
        let (is_sub, hz) = YaesuCat::parse_freq_resp(&cmd).expect("parse freq");
        assert!(!is_sub);
        assert_eq!(hz, 14_074_000);

        let cmd_sub = YaesuCat::set_freq(true, 7_040_000);
        assert_eq!(cmd_sub, "FB007040000;");
        let (is_sub2, hz2) = YaesuCat::parse_freq_resp(&cmd_sub).expect("parse sub freq");
        assert!(is_sub2);
        assert_eq!(hz2, 7_040_000);

        // format_freq_checked
        assert_eq!(
            YaesuCat::format_freq_checked(false, 14_074_000, 9),
            Some("FA014074000;".to_string())
        );
        // Odrzucenie wartości przekraczającej liczbę cyfr
        assert_eq!(YaesuCat::format_freq_checked(false, 1_000_000_000, 9), None);
        assert_eq!(YaesuCat::format_freq_checked(false, 14_074_000, 0), None);

        // Odrzucenie znaków nienumerycznych w parse_freq_resp
        assert_eq!(YaesuCat::parse_freq_resp("FA+14074000;"), None);
        assert_eq!(YaesuCat::parse_freq_resp("FA14A74000;"), None);
        assert_eq!(YaesuCat::parse_freq_resp("FA;"), None);
    }

    #[test]
    fn test_mode_mappings() {
        let modes = [
            ("USB", "MD02;", "USB"),
            ("LSB", "MD01;", "LSB"),
            ("CW", "MD03;", "CW-U"),
            ("FM", "MD04;", "FM"),
            ("AM", "MD05;", "AM"),
            ("RTTY", "MD06;", "RTTY-L"),
            ("CW-R", "MD07;", "CW-L"),
            ("DATA-L", "MD08;", "DATA-L"),
            ("RTTY-R", "MD09;", "RTTY-U"),
            ("DATA-FM", "MD0A;", "DATA-FM"),
            ("FM-N", "MD0B;", "FM-N"),
            ("DATA-U", "MD0C;", "DATA-U"),
            ("FT8", "MD0C;", "DATA-U"),
            ("AM-N", "MD0D;", "AM-N"),
            ("PSK", "MD0E;", "PSK"),
        ];

        for (name, expected_cmd, expected_mode) in modes {
            let cmd = YaesuCat::set_mode(name).expect(name);
            assert_eq!(cmd, expected_cmd);
            let parsed = YaesuCat::parse_mode_resp(&cmd).expect(name);
            assert_eq!(parsed, expected_mode);
        }

        assert!(YaesuCat::set_mode("UNKNOWN_MODE").is_none());
        assert_eq!(YaesuCat::map_mode_code_opt('X'), None);
        assert_eq!(YaesuCat::map_mode_code_opt('0'), None);
    }

    #[test]
    fn yaesu_mode_maps_are_profile_specific() {
        // FT-891 nie posiada trybu PSK (kod 'E') ani C4FM
        assert_eq!(YaesuCat::set_mode_for_model("FT-891", "PSK"), None);
        assert_eq!(YaesuCat::map_mode_code_for_model("FT-891", 'E'), None);
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FT-891", '1'),
            Some("LSB")
        );
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FT-891", '2'),
            Some("USB")
        );
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FT-891", '3'),
            Some("CW-U")
        );
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FT-891", '7'),
            Some("CW-L")
        );
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FT-891", 'C'),
            Some("DATA-U")
        );

        // FT-991A: kod 'E' to C4FM
        assert_eq!(
            YaesuCat::set_mode_for_model("FT-991A", "C4FM"),
            Some("MD0E;".to_string())
        );
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FT-991A", 'E'),
            Some("C4FM")
        );

        // FT-710 / FTDX10: kod 'E' to PSK
        assert_eq!(
            YaesuCat::set_mode_for_model("FT-710", "PSK"),
            Some("MD0E;".to_string())
        );
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FT-710", 'E'),
            Some("PSK")
        );
        assert_eq!(
            YaesuCat::set_mode_for_model("FTDX10", "PSK"),
            Some("MD0E;".to_string())
        );
        assert_eq!(
            YaesuCat::map_mode_code_for_model("FTDX10", 'E'),
            Some("PSK")
        );

        // Brak fallbacku do USB dla nieznanych kodów
        assert_eq!(YaesuCat::map_mode_code_for_model("FT-891", 'Z'), None);
        assert_eq!(YaesuCat::map_mode_code_for_model("FT-991A", 'Z'), None);
        assert_eq!(YaesuCat::map_mode_code_for_model("FT-710", 'Z'), None);
        assert_eq!(YaesuCat::map_mode_code_for_model("FTDX10", 'Z'), None);
    }

    #[test]
    fn test_ptt_and_split_parsing() {
        assert_eq!(YaesuCat::set_ptt(true), "TX1;");
        assert_eq!(YaesuCat::set_ptt(false), "TX0;");
        assert_eq!(YaesuCat::parse_ptt_resp("TX1;"), Some(true));
        assert_eq!(YaesuCat::parse_ptt_resp("TX2;"), Some(true));
        assert_eq!(YaesuCat::parse_ptt_resp("TX0;"), Some(false));
        assert_eq!(YaesuCat::parse_ptt_resp("TX9;"), None);

        assert_eq!(YaesuCat::set_split(true), "FT1;");
        assert_eq!(YaesuCat::set_split(false), "FT0;");
        assert_eq!(YaesuCat::parse_split_resp("FT1;"), Some(true));
        assert_eq!(YaesuCat::parse_split_resp("FT0;"), Some(false));
    }

    #[test]
    fn test_smeter_parsing_and_conversion() {
        assert_eq!(YaesuCat::read_smeter(), "SM0;");
        assert_eq!(YaesuCat::parse_smeter_resp("SM00120;"), Some(120));
        assert_eq!(YaesuCat::parse_smeter_resp("SM0000;"), Some(0));

        let unit_zero = YaesuCat::smeter_to_unit(0);
        assert_eq!(unit_zero, "S0");

        let unit_s9 = YaesuCat::smeter_to_unit(120);
        assert_eq!(unit_s9, "S9");

        let unit_over = YaesuCat::smeter_to_unit(255);
        assert_eq!(unit_over, "S9+60dB");
    }

    #[test]
    fn yaesu_smeter_does_not_fabricate_dbm() {
        // Protokół Yaesu SM0 zwraca surowe 000..255. Sprawdzamy, że smeter_to_unit
        // zachowuje czystą skalę wskaźnika S bez arbitralnego przeliczania na dBm.
        let u0 = YaesuCat::smeter_to_unit(0);
        assert_eq!(u0, "S0");
        let u60 = YaesuCat::smeter_to_unit(60);
        assert_eq!(u60, "S5");
        let u120 = YaesuCat::smeter_to_unit(120);
        assert_eq!(u120, "S9");
        let u180 = YaesuCat::smeter_to_unit(180);
        assert_eq!(u180, "S9+20dB");
        let u255 = YaesuCat::smeter_to_unit(255);
        assert_eq!(u255, "S9+60dB");
    }

    #[test]
    fn test_if_info_parser_ft991a_ft891() {
        // P1=000, P2=014074000, P3=+0000, P4=0(RIT off), P5=0(XIT off), P6=0, P7=0(VFO-A), P8=2(USB), P9=0(RX), P10=0(no split), P11=0, P12=0
        let raw = "IF000014074000+0000000020000;";
        let info = YaesuIfInfo::parse(raw).expect("parse valid IF frame");
        assert_eq!(info.frequency_hz, 14_074_000);
        assert_eq!(info.rit_offset_hz, 0);
        assert!(!info.rit_on);
        assert!(!info.xit_on);
        assert!(!info.tx);
        assert_eq!(info.mode, "USB");
        assert!(!info.vfo_b);
        assert!(!info.split);

        // Ramka z włączonym RIT, TX, Split i VFO-B
        let raw2 = "IF000007040000-01501001C1100;";
        let info2 = YaesuIfInfo::parse(raw2).expect("parse valid IF frame 2");
        assert_eq!(info2.frequency_hz, 7_040_000);
        assert_eq!(info2.rit_offset_hz, -150);
        assert!(info2.rit_on);
        assert!(!info2.xit_on);
        assert!(info2.tx);
        assert_eq!(info2.mode, "DATA-U");
        assert!(info2.vfo_b);
        assert!(info2.split);
    }

    #[test]
    fn yaesu_if_layouts_are_profile_validated() {
        // Fixture FT-891: 14.074.000 Hz USB, RX, VFO-A, RIT off
        let ft891_raw = "IF000014074000+0000000020000;";
        let ft891_info = YaesuIfInfo::parse_for_model("FT-891", ft891_raw).expect("FT-891 parse");
        assert_eq!(ft891_info.frequency_hz, 14_074_000);
        assert_eq!(ft891_info.mode, "USB");
        assert!(!ft891_info.tx);
        assert!(!ft891_info.vfo_b);

        // Fixture FT-991A: 144.200.000 Hz, C4FM (kod E), RX, VFO-A
        let ft991a_raw = "IF000144200000+00000000E0000;";
        let ft991a_info =
            YaesuIfInfo::parse_for_model("FT-991A", ft991a_raw).expect("FT-991A parse");
        assert_eq!(ft991a_info.frequency_hz, 144_200_000);
        assert_eq!(ft991a_info.mode, "C4FM");

        // Fixture FT-710: 7.074.000 Hz, DATA-U (kod C), TX, VFO-B, Split ON
        let ft710_raw = "IF000007074000+00000001C1100;";
        let ft710_info = YaesuIfInfo::parse_for_model("FT-710", ft710_raw).expect("FT-710 parse");
        assert_eq!(ft710_info.frequency_hz, 7_074_000);
        assert_eq!(ft710_info.mode, "DATA-U");
        assert!(ft710_info.tx);
        assert!(ft710_info.vfo_b);
        assert!(ft710_info.split);

        // Fixture FTDX10: 50.313.000 Hz, PSK (kod E), RX, VFO-A
        let ftdx10_raw = "IF000050313000+00000000E0000;";
        let ftdx10_info = YaesuIfInfo::parse_for_model("FTDX10", ftdx10_raw).expect("FTDX10 parse");
        assert_eq!(ftdx10_info.frequency_hz, 50_313_000);
        assert_eq!(ftdx10_info.mode, "PSK");

        // Odrzucenie niepoprawnego kodu emisji bez cichego fallbacku do USB
        let invalid_mode = "IF000014074000+00000000Z0000;";
        assert!(YaesuIfInfo::parse_for_model("FT-891", invalid_mode).is_none());
    }

    #[test]
    fn test_if_info_parser_malformed() {
        assert!(YaesuIfInfo::parse("").is_none());
        assert!(YaesuIfInfo::parse("FA014074000;").is_none());
        assert!(YaesuIfInfo::parse("IF;").is_none());
        assert!(YaesuIfInfo::parse("IF123;").is_none());
        assert!(YaesuIfInfo::parse("IF000014074000+0000000;").is_none());
    }
}
