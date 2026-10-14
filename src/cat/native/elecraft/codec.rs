// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Strumieniowy kodek tekstowy protokołu Elecraft CAT.
//!
//! Zgodny ze specyfikacją:
//! - *Elecraft K3S, K3, KX3, and KX2 Programmer's Reference*, Rev G5.
//! - *Elecraft K4 Programmer's Reference*, Rev D12.
//!
//! Zasady:
//! 1. Każda ramka jest terminowana znakiem średnika `;`.
//! 2. Prefiksy o zmiennej długości (2, 3 lub 4 znaki, np. `FA`, `MD`, `SMH`, `SWT`, `SWH`, `#RVM`).
//! 3. Opcjonalny modyfikator `$` oznaczający VFO B / Sub-RX (np. `MD$`, `BW$`, `SMH$`).
//! 4. Zachowanie ogona niepełnej ramki między odczytami ze strumienia.
//! 5. Obsługa ramki błędu `?;`.
//! 6. Bezpieczny typed error dla błędnych znanych odpowiedzi i bezpieczny `Unknown` dla nieznanych.

use crate::cat::native::elecraft::models::ElecraftProtocolFamily;

/// Domyślny maksymalny dozwolony rozmiar pojedynczej ramki Elecraft CAT (w bajtach).
pub const ELECRAFT_DEFAULT_MAX_FRAME_LEN: usize = 256;

/// Błędy dekodowania strumieniowego protokołu Elecraft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElecraftCodecError {
    /// Przekroczono maksymalną dozwoloną długość ramki bez średnika.
    FrameTooLarge { len: usize, max: usize },
    /// Ramka zawiera niepoprawne bajty UTF-8.
    InvalidUtf8 { detail: String },
    /// Ramka jest pusta lub uszkodzona składniowo.
    MalformedFrame { raw: String },
    /// Niepoprawny format częstotliwości.
    InvalidFrequency { raw: String },
    /// Niepoprawny kod emisji lub podtrybu.
    InvalidMode { raw: String },
    /// Niepoprawna szerokość filtru.
    InvalidPassband { raw: String },
    /// Niepoprawna wartość przesunięcia RIT/XIT.
    InvalidOffset { raw: String },
    /// Niepoprawna wartość mocy wyjściowej.
    InvalidPower { raw: String },
    /// Niepoprawny odczyt miernika.
    InvalidMeter { raw: String },
}

impl std::fmt::Display for ElecraftCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameTooLarge { len, max } => {
                write!(f, "Elecraft CAT frame exceeded max size ({len} > {max})")
            }
            Self::InvalidUtf8 { detail } => {
                write!(f, "Elecraft CAT frame is not valid UTF-8: {detail}")
            }
            Self::MalformedFrame { raw } => {
                write!(f, "malformed Elecraft CAT frame: {raw}")
            }
            Self::InvalidFrequency { raw } => {
                write!(f, "invalid frequency field in Elecraft frame: {raw}")
            }
            Self::InvalidMode { raw } => {
                write!(f, "invalid mode field in Elecraft frame: {raw}")
            }
            Self::InvalidPassband { raw } => {
                write!(f, "invalid passband field in Elecraft frame: {raw}")
            }
            Self::InvalidOffset { raw } => {
                write!(f, "invalid RIT/XIT offset field in Elecraft frame: {raw}")
            }
            Self::InvalidPower { raw } => {
                write!(f, "invalid power field in Elecraft frame: {raw}")
            }
            Self::InvalidMeter { raw } => {
                write!(f, "invalid meter reading in Elecraft frame: {raw}")
            }
        }
    }
}

impl std::error::Error for ElecraftCodecError {}

/// Znane 4-znakowe prefiksy poleceń Elecraft (np. z prefiksem `#`).
pub const KNOWN_PREFIXES_4: &[&str] = &[
    "#AVG", "#DPO", "#FQO", "#K41", "#K42", "#K31", "#PAN", "#RVM", "#VCH",
];

/// Znane 3-znakowe prefiksy poleceń Elecraft.
pub const KNOWN_PREFIXES_3: &[&str] = &[
    "BNR", "DBA", "DND", "DNU", "DSW", "K30", "K31", "K40", "K41", "K42", "SMH", "SWH", "SWT",
];

/// Znane 2-znakowe prefiksy poleceń Elecraft.
pub const KNOWN_PREFIXES_2: &[&str] = &[
    "AG", "AI", "AN", "AP", "BG", "BN", "BW", "CP", "CW", "DN", "DS", "DT", "DV", "FA", "FB", "FI",
    "FP", "FR", "FT", "FW", "GT", "IC", "ID", "IF", "IS", "KS", "KY", "LK", "MD", "MG", "ML", "NB",
    "NL", "NR", "NT", "OM", "PA", "PC", "PL", "PO", "PS", "RA", "RC", "RG", "RL", "RO", "RT", "RU",
    "RV", "RX", "SB", "SG", "SM", "SP", "SQ", "SW", "TB", "TE", "TM", "TQ", "TS", "TX", "UP", "VG",
    "VT", "XF", "XT",
];

/// Pojedyncza ramka protokołu Elecraft wyodrębniona ze strumienia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElecraftFrame {
    /// Prefiks polecenia (np. "FA", "MD", "SMH", "SWT", "?").
    pub prefix: String,
    /// Czy wystąpił modyfikator `$` kierujący komendę do VFO B / Sub-RX.
    pub has_vfo_b_modifier: bool,
    /// Zawartość parametrów bez prefiksu, znaku `$` i średnika.
    pub payload: String,
    /// Oryginalny, kompletny ciąg znaków wraz ze średnikiem.
    pub raw: String,
}

impl ElecraftFrame {
    /// Parsuje surowy ciąg znaków zakończony średnikiem do postaci [`ElecraftFrame`].
    pub fn parse(raw: &str) -> Result<Self, ElecraftCodecError> {
        let trimmed = raw.trim();
        if !trimmed.ends_with(';') {
            return Err(ElecraftCodecError::MalformedFrame {
                raw: raw.to_string(),
            });
        }

        let content = trimmed.trim_end_matches(';').trim();
        if content.is_empty() {
            return Ok(Self {
                prefix: String::new(),
                has_vfo_b_modifier: false,
                payload: String::new(),
                raw: trimmed.to_string(),
            });
        }

        // Odpowiedź błędu "?;"
        if content == "?" {
            return Ok(Self {
                prefix: "?".to_string(),
                has_vfo_b_modifier: false,
                payload: String::new(),
                raw: trimmed.to_string(),
            });
        }

        let upper = content.to_ascii_uppercase();

        let prefix_len = if upper.starts_with('#') {
            if let Some(&p) = KNOWN_PREFIXES_4.iter().find(|&&p| upper.starts_with(p)) {
                p.len()
            } else {
                let cnt = upper[1..]
                    .chars()
                    .take_while(char::is_ascii_alphabetic)
                    .count();
                (1 + cnt).min(4)
            }
        } else if let Some(&p) = KNOWN_PREFIXES_3.iter().find(|&&p| upper.starts_with(p)) {
            p.len()
        } else if let Some(&p) = KNOWN_PREFIXES_2.iter().find(|&&p| upper.starts_with(p)) {
            p.len()
        } else {
            let cnt = upper.chars().take_while(char::is_ascii_alphabetic).count();
            cnt.min(2).max(1)
        };

        let prefix = &content[..prefix_len];
        let rest = &content[prefix_len..];

        let (has_vfo_b_modifier, payload) = if let Some(after_dollar) = rest.strip_prefix('$') {
            (true, after_dollar)
        } else {
            (false, rest)
        };

        Ok(Self {
            prefix: prefix.to_ascii_uppercase(),
            has_vfo_b_modifier,
            payload: payload.to_string(),
            raw: trimmed.to_string(),
        })
    }

    /// Sprawdza, czy ramka jest raportem błędu `?;`.
    pub fn is_error(&self) -> bool {
        self.prefix == "?" || self.raw.trim() == "?;"
    }
}

/// Zdekodowane dane transceivera z ramki informacji o stanie `IF;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElecraftIfInfo {
    /// Częstotliwość bieżącego VFO w Hz (11 cyfr).
    pub frequency_hz: u64,
    /// Przesunięcie RIT/XIT w Hz (wartość ze znakiem).
    pub rit_offset_hz: i32,
    /// Flaga włączenia RIT (funkcja RIT aktywna).
    pub rit_on: bool,
    /// Flaga włączenia XIT (funkcja XIT aktywna).
    pub xit_on: bool,
    /// Stan nadawania PTT (true = TX, false = RX).
    pub tx: bool,
    /// Kod emisji (np. "USB", "LSB", "CW", "FM", "AM", "DATA", "CW-R", "DATA-R").
    pub mode: &'static str,
    /// Wybór VFO (true = VFO B, false = VFO A).
    pub vfo_b: bool,
    /// Flaga skanowania.
    pub scan: bool,
    /// Flaga pracy Split.
    pub split: bool,
}

impl ElecraftIfInfo {
    /// Parsuje zawartość odpowiedzi na komendę `IF;`.
    ///
    /// Struktura `IF`: `IF[11 cyfr f][5 spacji][znak i 4 cyfry RIT][r][x][spacja][00][t][m][v][s][p]...;`
    pub fn parse(resp: &str) -> Option<Self> {
        let trimmed = resp.trim().trim_end_matches(';');
        let body = if let Some(s) = trimmed.strip_prefix("IF") {
            s
        } else {
            trimmed
        };

        if body.len() < 31 {
            return None;
        }

        let chars: Vec<char> = body.chars().collect();

        // 1. Częstotliwość f: indeksy 0..11 (11 cyfr)
        let freq_str: String = chars[0..11].iter().collect();
        let frequency_hz = freq_str.trim().parse::<u64>().ok()?;

        // 2. RIT/XIT offset: indeksy 16..21 (znak i 4 cyfry)
        let mut rit_offset_hz = 0i32;
        if chars.len() >= 21 {
            let offset_str: String = chars[16..21].iter().collect();
            let sign = if offset_str.starts_with('-') { -1 } else { 1 };
            let digits_part = offset_str.trim_start_matches(['+', '-']).trim();
            if let Ok(val) = digits_part.parse::<i32>() {
                rit_offset_hz = sign * val;
            }
        }

        // 3. RIT on (indeks 21)
        let rit_on = chars.get(21).copied() == Some('1');

        // 4. XIT on (indeks 22)
        let xit_on = chars.get(22).copied() == Some('1');

        // 5. TX status (indeks 26: '1' lub '2' = TX, '0' = RX)
        let tx_char = chars.get(26).copied()?;
        let tx = tx_char == '1' || tx_char == '2';

        // 6. Mode (indeks 27)
        let mode_char = chars.get(27).copied()?;
        let mode = match mode_char {
            '1' => "LSB",
            '2' => "USB",
            '3' => "CW",
            '4' => "FM",
            '5' => "AM",
            '6' => "DATA",
            '7' => "CW-R",
            '9' => "DATA-R",
            _ => "UNKNOWN",
        };

        // 7. VFO (indeks 28: '0' = VFO A, '1' = VFO B)
        let vfo_b = chars.get(28).copied() == Some('1');

        // 8. Scan (indeks 29)
        let scan = chars.get(29).copied() == Some('1');

        // 9. Split (indeks 30: '0' = Off, '1' = On)
        let split = chars.get(30).copied() == Some('1');

        Some(Self {
            frequency_hz,
            rit_offset_hz,
            rit_on,
            xit_on,
            tx,
            mode,
            vfo_b,
            scan,
            split,
        })
    }
}

/// Zdekodowany, wysokopoziomowy komunikat protokołu Elecraft.
#[derive(Debug, Clone, PartialEq)]
pub enum ElecraftMessage {
    /// Częstotliwość VFO A lub B w Hz.
    Frequency { vfo_b: bool, hz: u64 },
    /// Emisja VFO A lub B.
    Mode { vfo_b: bool, mode: &'static str },
    /// Podtryb cyfrowy (DATA A, AFSK A, FSK D, PSK D).
    DataSubMode {
        vfo_b: bool,
        submode: u8,
        name: &'static str,
    },
    /// Złożony stan z ramki informacyjnej `IF;`.
    IfInfo(ElecraftIfInfo),
    /// Identyfikator modelu radiostacji (np. "017" dla K3, tekst K4).
    Identification(String),
    /// Stan nadawania (true = TX, false = RX).
    TransmitStatus(bool),
    /// Wybór VFO odbiorczego (`FR0;` / `FR1;`).
    ReceiveVfo { vfo_b: bool },
    /// Przypisanie VFO nadawczego i stan Split (`FT0;` / `FT1;`).
    TransmitVfo { vfo_b: bool, split: bool },
    /// Stan trybu Auto Information (`AI0`..`AI4`).
    AutoInfo(u8),
    /// Zadana moc wyjściowa RF w watach oraz skala ('H' = waty, 'L' = 0.1W).
    PowerControl { watts: f32, raw_scale: char },
    /// Rzeczywisty pomiar mocy nadawania w watach (`PO`).
    PowerOutput(f32),
    /// Szerokość pasma filtru odbiorczego w Hz (`BW`).
    Passband { vfo_b: bool, passband_hz: u32 },
    /// Stan załączenia RIT (`RT`).
    RitState { vfo_b: bool, enabled: bool },
    /// Stan załączenia XIT (`XT`).
    XitState { vfo_b: bool, enabled: bool },
    /// Wartość przesunięcia RIT/XIT w Hz (`RO`).
    RitOffset { vfo_b: bool, offset_hz: i32 },
    /// Surowy odczyt słupków S-metra (0000..0015 / 0042) (`SM`).
    RawSMeter { vfo_b: bool, raw: u16 },
    /// Precyzyjny odczyt sygnału w dBm (`SMH` na K4).
    HighResSMeter { vfo_b: bool, dbm: f32 },
    /// Odczyt segmentów linijki LCD / bargraph (`BG`).
    Bargraph { raw: String },
    /// Odpowiedź na zapytanie o moduły opcjonalne `OM` (np. K3 'S' = KRX3 zainstalowany).
    OptionModule(String),
    /// Odpowiedź błędu `?;`.
    Error,
    /// Bezpieczny odczyt poprawnej, ale nieobsługiwanej ramki.
    Unknown {
        prefix: String,
        vfo_b_modifier: bool,
        raw: String,
    },
}

impl ElecraftMessage {
    /// Weryfikuje, czy dany komunikat odpowiada oczekiwanemu prefiksowi zapytania GET.
    pub fn matches_query_prefix(&self, query_prefix: &str) -> bool {
        let q = query_prefix
            .trim()
            .trim_end_matches(';')
            .to_ascii_uppercase();
        match self {
            Self::Frequency { vfo_b: false, .. } => q == "FA",
            Self::Frequency { vfo_b: true, .. } => q == "FB",
            Self::Mode { vfo_b: false, .. } => q == "MD",
            Self::Mode { vfo_b: true, .. } => q == "MD$" || q == "MD",
            Self::DataSubMode { vfo_b: false, .. } => q == "DT",
            Self::DataSubMode { vfo_b: true, .. } => q == "DT$" || q == "DT",
            Self::IfInfo(_) => q == "IF",
            Self::Identification(_) => q == "ID",
            Self::TransmitStatus(_) => q == "TQ" || q == "TX",
            Self::ReceiveVfo { .. } => q == "FR",
            Self::TransmitVfo { .. } => q == "FT",
            Self::AutoInfo(_) => q == "AI",
            Self::PowerControl { .. } => q == "PC",
            Self::PowerOutput(_) => q == "PO",
            Self::Passband { vfo_b: false, .. } => q == "BW" || q == "FW",
            Self::Passband { vfo_b: true, .. } => q == "BW$" || q == "BW" || q == "FW",
            Self::RitState { .. } => q == "RT" || q == "RT$",
            Self::XitState { .. } => q == "XT" || q == "XT$",
            Self::RitOffset { .. } => q == "RO" || q == "RO$" || q == "RC",
            Self::RawSMeter { .. } => q == "SM" || q == "SM$",
            Self::HighResSMeter { .. } => q == "SMH" || q == "SMH$",
            Self::Bargraph { .. } => q == "BG",
            Self::OptionModule(_) => q == "OM",
            Self::Error => true,
            Self::Unknown { prefix, .. } => q == prefix.to_ascii_uppercase(),
        }
    }
}

/// Klasyfikacja pakietu Elecraft jako odpowiedzi na zapytanie lub zdarzenia Auto-Info.
#[derive(Debug, Clone, PartialEq)]
pub enum ElecraftPacket {
    /// Bezpośrednia odpowiedź na wysłane zapytanie GET.
    Response(ElecraftMessage),
    /// Spontaniczne powiadomienie Auto-Info (np. zmiana gałki VFO).
    AutoInfoEvent(ElecraftMessage),
}

/// Strumieniowy kodek asynchroniczny protokołu Elecraft CAT.
#[derive(Debug, Clone)]
pub struct ElecraftCodec {
    max_frame_len: usize,
}

impl Default for ElecraftCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl ElecraftCodec {
    /// Tworzy nowy kodek z domyślnym limitem długości ramki (256 bajtów).
    pub fn new() -> Self {
        Self {
            max_frame_len: ELECRAFT_DEFAULT_MAX_FRAME_LEN,
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

    /// Próbuje wyodrębnić kompletną ramkę ze strumienia bajtów.
    ///
    /// Zwraca:
    /// - `Ok(Some((frame, consumed_bytes)))` — gdy znaleziono pełną ramkę zakończoną `;`,
    /// - `Ok(None)` — gdy bufor zawiera tylko fragment ramki i oczekuje na dalsze bajty,
    /// - `Err(ElecraftCodecError)` — gdy bufor przekroczył limit bez średnika lub zawiera niepoprawny UTF-8.
    pub fn decode_frame(
        &self,
        buf: &[u8],
    ) -> Result<Option<(ElecraftFrame, usize)>, ElecraftCodecError> {
        if buf.is_empty() {
            return Ok(None);
        }

        if let Some(pos) = buf.iter().position(|&b| b == b';') {
            let frame_bytes = &buf[..=pos];
            let raw_str =
                std::str::from_utf8(frame_bytes).map_err(|e| ElecraftCodecError::InvalidUtf8 {
                    detail: e.to_string(),
                })?;

            let frame = ElecraftFrame::parse(raw_str)?;
            Ok(Some((frame, pos + 1)))
        } else if buf.len() > self.max_frame_len {
            Err(ElecraftCodecError::FrameTooLarge {
                len: buf.len(),
                max: self.max_frame_len,
            })
        } else {
            Ok(None)
        }
    }

    /// Dekoduje surową ramkę [`ElecraftFrame`] do typowanego komunikatu [`ElecraftMessage`].
    pub fn decode_message(
        &self,
        frame: &ElecraftFrame,
        _family: ElecraftProtocolFamily,
    ) -> Result<ElecraftMessage, ElecraftCodecError> {
        if frame.is_error() {
            return Ok(ElecraftMessage::Error);
        }

        match frame.prefix.as_str() {
            "FA" => {
                let hz = frame.payload.trim().parse::<u64>().map_err(|_| {
                    ElecraftCodecError::InvalidFrequency {
                        raw: frame.raw.clone(),
                    }
                })?;
                Ok(ElecraftMessage::Frequency { vfo_b: false, hz })
            }
            "FB" => {
                let hz = frame.payload.trim().parse::<u64>().map_err(|_| {
                    ElecraftCodecError::InvalidFrequency {
                        raw: frame.raw.clone(),
                    }
                })?;
                Ok(ElecraftMessage::Frequency { vfo_b: true, hz })
            }
            "MD" => {
                let code_char = frame.payload.trim().chars().next().ok_or_else(|| {
                    ElecraftCodecError::InvalidMode {
                        raw: frame.raw.clone(),
                    }
                })?;
                let mode = match code_char {
                    '1' => "LSB",
                    '2' => "USB",
                    '3' => "CW",
                    '4' => "FM",
                    '5' => "AM",
                    '6' => "DATA",
                    '7' => "CW-R",
                    '9' => "DATA-R",
                    _ => {
                        return Err(ElecraftCodecError::InvalidMode {
                            raw: frame.raw.clone(),
                        });
                    }
                };
                Ok(ElecraftMessage::Mode {
                    vfo_b: frame.has_vfo_b_modifier,
                    mode,
                })
            }
            "DT" => {
                let submode = frame.payload.trim().parse::<u8>().map_err(|_| {
                    ElecraftCodecError::InvalidMode {
                        raw: frame.raw.clone(),
                    }
                })?;
                let name = match submode {
                    0 => "DATA A",
                    1 => "AFSK A",
                    2 => "FSK D",
                    3 => "PSK D",
                    _ => {
                        return Err(ElecraftCodecError::InvalidMode {
                            raw: frame.raw.clone(),
                        });
                    }
                };
                Ok(ElecraftMessage::DataSubMode {
                    vfo_b: frame.has_vfo_b_modifier,
                    submode,
                    name,
                })
            }
            "IF" => {
                let info = ElecraftIfInfo::parse(&frame.payload).ok_or_else(|| {
                    ElecraftCodecError::MalformedFrame {
                        raw: frame.raw.clone(),
                    }
                })?;
                Ok(ElecraftMessage::IfInfo(info))
            }
            "ID" => Ok(ElecraftMessage::Identification(frame.payload.clone())),
            "TQ" => match frame.payload.trim() {
                "0" => Ok(ElecraftMessage::TransmitStatus(false)),
                "1" => Ok(ElecraftMessage::TransmitStatus(true)),
                _ => Err(ElecraftCodecError::MalformedFrame {
                    raw: frame.raw.clone(),
                }),
            },
            "TX" => match frame.payload.trim() {
                "" | "1" | "2" => Ok(ElecraftMessage::TransmitStatus(true)),
                "0" => Ok(ElecraftMessage::TransmitStatus(false)),
                _ => Err(ElecraftCodecError::MalformedFrame {
                    raw: frame.raw.clone(),
                }),
            },
            "RX" => Ok(ElecraftMessage::TransmitStatus(false)),
            "FR" => match frame.payload.trim() {
                "0" => Ok(ElecraftMessage::ReceiveVfo { vfo_b: false }),
                "1" => Ok(ElecraftMessage::ReceiveVfo { vfo_b: true }),
                _ => Err(ElecraftCodecError::MalformedFrame {
                    raw: frame.raw.clone(),
                }),
            },
            "FT" => match frame.payload.trim() {
                "0" => Ok(ElecraftMessage::TransmitVfo {
                    vfo_b: false,
                    split: false,
                }),
                "1" => Ok(ElecraftMessage::TransmitVfo {
                    vfo_b: true,
                    split: true,
                }),
                _ => Err(ElecraftCodecError::MalformedFrame {
                    raw: frame.raw.clone(),
                }),
            },
            "AI" => {
                let mode = frame.payload.trim().parse::<u8>().map_err(|_| {
                    ElecraftCodecError::MalformedFrame {
                        raw: frame.raw.clone(),
                    }
                })?;
                Ok(ElecraftMessage::AutoInfo(mode))
            }
            "PC" => {
                let payload = frame.payload.trim();
                if payload.is_empty() {
                    return Err(ElecraftCodecError::InvalidPower {
                        raw: frame.raw.clone(),
                    });
                }
                let last_char = payload.chars().last().unwrap_or(' ');
                let (digits_part, scale) = if last_char.is_ascii_alphabetic() {
                    (
                        &payload[..payload.len() - 1],
                        last_char.to_ascii_uppercase(),
                    )
                } else {
                    (payload, 'H')
                };

                let val =
                    digits_part
                        .parse::<f32>()
                        .map_err(|_| ElecraftCodecError::InvalidPower {
                            raw: frame.raw.clone(),
                        })?;

                let watts = if scale == 'L' { val * 0.1 } else { val };
                Ok(ElecraftMessage::PowerControl {
                    watts,
                    raw_scale: scale,
                })
            }
            "PO" => {
                let raw_val = frame.payload.trim().parse::<f32>().map_err(|_| {
                    ElecraftCodecError::InvalidPower {
                        raw: frame.raw.clone(),
                    }
                })?;
                // PO w Elecraft to dziesiąte części wata (np. PO0100 = 10.0W)
                Ok(ElecraftMessage::PowerOutput(raw_val * 0.1))
            }
            "BW" => {
                let units = frame.payload.trim().parse::<u32>().map_err(|_| {
                    ElecraftCodecError::InvalidPassband {
                        raw: frame.raw.clone(),
                    }
                })?;
                // BW podaje szerokość w jednostkach 10 Hz (np. 0240 = 2400 Hz)
                Ok(ElecraftMessage::Passband {
                    vfo_b: frame.has_vfo_b_modifier,
                    passband_hz: units * 10,
                })
            }
            "RT" => match frame.payload.trim() {
                "0" => Ok(ElecraftMessage::RitState {
                    vfo_b: frame.has_vfo_b_modifier,
                    enabled: false,
                }),
                "1" => Ok(ElecraftMessage::RitState {
                    vfo_b: frame.has_vfo_b_modifier,
                    enabled: true,
                }),
                _ => Err(ElecraftCodecError::MalformedFrame {
                    raw: frame.raw.clone(),
                }),
            },
            "XT" => match frame.payload.trim() {
                "0" => Ok(ElecraftMessage::XitState {
                    vfo_b: frame.has_vfo_b_modifier,
                    enabled: false,
                }),
                "1" => Ok(ElecraftMessage::XitState {
                    vfo_b: frame.has_vfo_b_modifier,
                    enabled: true,
                }),
                _ => Err(ElecraftCodecError::MalformedFrame {
                    raw: frame.raw.clone(),
                }),
            },
            "RO" => {
                let offset_str = frame.payload.trim();
                let offset_hz =
                    offset_str
                        .parse::<i32>()
                        .map_err(|_| ElecraftCodecError::InvalidOffset {
                            raw: frame.raw.clone(),
                        })?;
                Ok(ElecraftMessage::RitOffset {
                    vfo_b: frame.has_vfo_b_modifier,
                    offset_hz,
                })
            }
            "RC" => Ok(ElecraftMessage::RitOffset {
                vfo_b: frame.has_vfo_b_modifier,
                offset_hz: 0,
            }),
            "SM" => {
                let raw = frame.payload.trim().parse::<u16>().map_err(|_| {
                    ElecraftCodecError::InvalidMeter {
                        raw: frame.raw.clone(),
                    }
                })?;
                Ok(ElecraftMessage::RawSMeter {
                    vfo_b: frame.has_vfo_b_modifier,
                    raw,
                })
            }
            "SMH" => {
                // SMH na K4 podaje sygnał w dBm (np. "+015", "-085")
                let dbm = frame.payload.trim().parse::<f32>().map_err(|_| {
                    ElecraftCodecError::InvalidMeter {
                        raw: frame.raw.clone(),
                    }
                })?;
                Ok(ElecraftMessage::HighResSMeter {
                    vfo_b: frame.has_vfo_b_modifier,
                    dbm,
                })
            }
            "BG" => Ok(ElecraftMessage::Bargraph {
                raw: frame.payload.clone(),
            }),
            "OM" => Ok(ElecraftMessage::OptionModule(frame.payload.clone())),
            _ => Ok(ElecraftMessage::Unknown {
                prefix: frame.prefix.clone(),
                vfo_b_modifier: frame.has_vfo_b_modifier,
                raw: frame.raw.clone(),
            }),
        }
    }
}

/// Generator i formater poleceń Elecraft CAT.
pub struct ElecraftCat;

impl ElecraftCat {
    pub fn read_freq(vfo_b: bool) -> &'static str {
        if vfo_b { "FB;" } else { "FA;" }
    }

    pub fn set_freq(vfo_b: bool, hz: u64) -> String {
        let prefix = if vfo_b { "FB" } else { "FA" };
        format!("{prefix}{hz:011};")
    }

    pub fn read_mode(vfo_b: bool) -> &'static str {
        if vfo_b { "MD$;" } else { "MD;" }
    }

    pub fn set_mode(vfo_b: bool, code: &str) -> String {
        let prefix = if vfo_b { "MD$" } else { "MD" };
        format!("{prefix}{code};")
    }

    pub fn set_data_submode(vfo_b: bool, submode: u8) -> String {
        let prefix = if vfo_b { "DT$" } else { "DT" };
        format!("{prefix}{submode};")
    }

    pub fn read_if() -> &'static str {
        "IF;"
    }

    pub fn read_id() -> &'static str {
        "ID;"
    }

    pub fn set_ptt(tx: bool) -> &'static str {
        if tx { "TX;" } else { "RX;" }
    }

    pub fn read_tq() -> &'static str {
        "TQ;"
    }

    pub fn set_split(split: bool) -> &'static str {
        if split { "FT1;" } else { "FT0;" }
    }

    pub fn read_split() -> &'static str {
        "FT;"
    }

    pub fn set_vfo(vfo_b: bool) -> &'static str {
        if vfo_b { "FR1;" } else { "FR0;" }
    }

    pub fn read_vfo() -> &'static str {
        "FR;"
    }

    pub fn set_passband(vfo_b: bool, passband_hz: u32) -> String {
        let prefix = if vfo_b { "BW$" } else { "BW" };
        let units = passband_hz / 10;
        format!("{prefix}{units:04};")
    }

    pub fn read_passband(vfo_b: bool) -> &'static str {
        if vfo_b { "BW$;" } else { "BW;" }
    }

    pub fn set_rit(vfo_b: bool, enabled: bool) -> &'static str {
        if vfo_b {
            if enabled { "RT$1;" } else { "RT$0;" }
        } else if enabled {
            "RT1;"
        } else {
            "RT0;"
        }
    }

    pub fn set_xit(vfo_b: bool, enabled: bool) -> &'static str {
        if vfo_b {
            if enabled { "XT$1;" } else { "XT$0;" }
        } else if enabled {
            "XT1;"
        } else {
            "XT0;"
        }
    }

    pub fn set_rit_offset(vfo_b: bool, offset_hz: i32) -> String {
        let prefix = if vfo_b { "RO$" } else { "RO" };
        format!("{prefix}{offset_hz:+05};")
    }

    pub fn clear_rit_offset(vfo_b: bool) -> &'static str {
        if vfo_b { "RC$;" } else { "RC;" }
    }

    pub fn set_power(watts: f32, max_watts: f32, is_k4: bool, is_qrp: bool) -> String {
        let clamped = watts.clamp(0.0, max_watts);
        if (is_k4 || is_qrp) && clamped < 10.0 {
            let tenths = (clamped * 10.0).round() as u32;
            format!("PC{tenths:03}L;")
        } else if is_k4 {
            let w = clamped.round() as u32;
            format!("PC{w:03}H;")
        } else {
            let w = clamped.round() as u32;
            format!("PC{w:03};")
        }
    }

    pub fn read_power() -> &'static str {
        "PC;"
    }

    pub fn read_options() -> &'static str {
        "OM;"
    }

    pub fn read_smeter(vfo_b: bool) -> &'static str {
        if vfo_b { "SM$;" } else { "SM;" }
    }

    pub fn read_high_res_smeter(vfo_b: bool) -> &'static str {
        if vfo_b { "SMH$;" } else { "SMH;" }
    }

    pub fn set_auto_info(mode: u8) -> String {
        format!("AI{mode};")
    }

    pub fn read_auto_info() -> &'static str {
        "AI;"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elecraft_codec_encodes_command() {
        let codec = ElecraftCodec::new();
        assert_eq!(codec.encode_command("FA"), b"FA;");
        assert_eq!(codec.encode_command("FA;"), b"FA;");
        assert_eq!(codec.encode_command("MD$2"), b"MD$2;");
        assert_eq!(codec.encode_command("TX"), b"TX;");
        assert_eq!(codec.encode_command("AI2;"), b"AI2;");
    }

    #[test]
    fn elecraft_codec_decodes_fragmented_response() {
        let codec = ElecraftCodec::new();
        let chunk1 = b"FA000140";
        let res1 = codec.decode_frame(chunk1).expect("valid fragment");
        assert_eq!(res1, None);

        let chunk2 = b"74000;";
        let mut full = chunk1.to_vec();
        full.extend_from_slice(chunk2);

        let res2 = codec.decode_frame(&full).expect("valid full");
        let (frame, consumed) = res2.expect("frame present");
        assert_eq!(consumed, 14);
        assert_eq!(frame.prefix, "FA");
        assert_eq!(frame.payload, "00014074000");
    }

    #[test]
    fn elecraft_codec_decodes_multiple_responses() {
        let codec = ElecraftCodec::new();
        let stream = b"FA00014074000;MD2;TQ0;";
        let (f1, c1) = codec.decode_frame(stream).unwrap().unwrap();
        assert_eq!(f1.prefix, "FA");
        assert_eq!(c1, 14);

        let (f2, c2) = codec.decode_frame(&stream[c1..]).unwrap().unwrap();
        assert_eq!(f2.prefix, "MD");
        assert_eq!(f2.payload, "2");
        assert_eq!(c2, 4);

        let (f3, c3) = codec.decode_frame(&stream[c1 + c2..]).unwrap().unwrap();
        assert_eq!(f3.prefix, "TQ");
        assert_eq!(f3.payload, "0");
        assert_eq!(c3, 4);
    }

    #[test]
    fn elecraft_codec_preserves_partial_message() {
        let codec = ElecraftCodec::new();
        let stream = b"MD2;FB000070";
        let (f1, c1) = codec.decode_frame(stream).unwrap().unwrap();
        assert_eq!(f1.prefix, "MD");
        assert_eq!(c1, 4);

        // Pozostała część bufora nie kończy się średnikiem:
        let remainder = &stream[c1..];
        assert_eq!(remainder, b"FB000070");
        let res = codec.decode_frame(remainder).unwrap();
        assert_eq!(res, None);
    }

    #[test]
    fn elecraft_codec_enforces_max_message_length() {
        let codec = ElecraftCodec::with_max_len(16);
        let oversize_no_semicolon = b"FA00014074000000000000";
        let err = codec.decode_frame(oversize_no_semicolon).unwrap_err();
        assert!(matches!(err, ElecraftCodecError::FrameTooLarge { .. }));
    }

    #[test]
    fn elecraft_codec_handles_error_response() {
        let codec = ElecraftCodec::new();
        let (frame, consumed) = codec.decode_frame(b"?;").unwrap().unwrap();
        assert_eq!(consumed, 2);
        assert!(frame.is_error());
        assert_eq!(frame.prefix, "?");

        let msg = codec
            .decode_message(&frame, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(msg, ElecraftMessage::Error);
    }

    #[test]
    fn elecraft_codec_unknown_message_is_safe() {
        let codec = ElecraftCodec::new();
        let (frame, _) = codec.decode_frame(b"DS123456;").unwrap().unwrap();
        assert_eq!(frame.prefix, "DS");
        assert_eq!(frame.payload, "123456");

        let msg = codec
            .decode_message(&frame, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(
            msg,
            ElecraftMessage::Unknown {
                prefix: "DS".to_string(),
                vfo_b_modifier: false,
                raw: "DS123456;".to_string(),
            }
        );
    }

    #[test]
    fn elecraft_codec_invalid_known_message_is_error() {
        let codec = ElecraftCodec::new();
        let (frame, _) = codec.decode_frame(b"FA_NOT_A_FREQ;").unwrap().unwrap();
        let res = codec.decode_message(&frame, ElecraftProtocolFamily::K3Compatible);
        assert!(matches!(
            res,
            Err(ElecraftCodecError::InvalidFrequency { .. })
        ));
    }

    #[test]
    fn elecraft_codec_supports_variable_prefix_length() {
        let codec = ElecraftCodec::new();
        // 2-znakowy
        let (f2, _) = codec.decode_frame(b"FA00014074000;").unwrap().unwrap();
        assert_eq!(f2.prefix, "FA");

        // 3-znakowy
        let (f3, _) = codec.decode_frame(b"SMH-085;").unwrap().unwrap();
        assert_eq!(f3.prefix, "SMH");
        assert_eq!(f3.payload, "-085");

        let (f3b, _) = codec.decode_frame(b"SWT26;").unwrap().unwrap();
        assert_eq!(f3b.prefix, "SWT");
        assert_eq!(f3b.payload, "26");

        // 4-znakowy z '#'
        let (f4, _) = codec.decode_frame(b"#RVM01;").unwrap().unwrap();
        assert_eq!(f4.prefix, "#RVM");
        assert_eq!(f4.payload, "01");
    }

    #[test]
    fn elecraft_codec_preserves_vfo_b_modifier() {
        let codec = ElecraftCodec::new();
        let (f1, _) = codec.decode_frame(b"MD$3;").unwrap().unwrap();
        assert_eq!(f1.prefix, "MD");
        assert!(f1.has_vfo_b_modifier);
        assert_eq!(f1.payload, "3");

        let msg1 = codec
            .decode_message(&f1, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(
            msg1,
            ElecraftMessage::Mode {
                vfo_b: true,
                mode: "CW",
            }
        );

        let (f2, _) = codec.decode_frame(b"BW$0240;").unwrap().unwrap();
        assert_eq!(f2.prefix, "BW");
        assert!(f2.has_vfo_b_modifier);
        assert_eq!(f2.payload, "0240");

        let msg2 = codec
            .decode_message(&f2, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(
            msg2,
            ElecraftMessage::Passband {
                vfo_b: true,
                passband_hz: 2400,
            }
        );

        let (f3, _) = codec.decode_frame(b"SMH$+015;").unwrap().unwrap();
        assert_eq!(f3.prefix, "SMH");
        assert!(f3.has_vfo_b_modifier);
        assert_eq!(f3.payload, "+015");

        let msg3 = codec
            .decode_message(&f3, ElecraftProtocolFamily::K4)
            .unwrap();
        assert_eq!(
            msg3,
            ElecraftMessage::HighResSMeter {
                vfo_b: true,
                dbm: 15.0,
            }
        );
    }

    #[test]
    fn elecraft_empty_input_does_not_panic() {
        let codec = ElecraftCodec::new();
        assert_eq!(codec.decode_frame(b"").unwrap(), None);
        assert_eq!(codec.decode_frame(&[]).unwrap(), None);

        let frame = ElecraftFrame::parse(";").unwrap();
        assert_eq!(frame.prefix, "");
        assert_eq!(frame.payload, "");
    }

    #[test]
    fn elecraft_auto_info_event_is_distinguished_from_response() {
        let msg = ElecraftMessage::Frequency {
            vfo_b: false,
            hz: 14_074_000,
        };

        // Gdy oczekujemy na FA, to jest to odpowiedź:
        assert!(msg.matches_query_prefix("FA"));
        let pkt_resp = if msg.matches_query_prefix("FA") {
            ElecraftPacket::Response(msg.clone())
        } else {
            ElecraftPacket::AutoInfoEvent(msg.clone())
        };
        assert_eq!(pkt_resp, ElecraftPacket::Response(msg.clone()));

        // Gdy nie oczekujemy na FA (np. w pętli poll oczekujemy na MD lub brak oczekiwania), to jest to AutoInfo:
        assert!(!msg.matches_query_prefix("MD"));
        let pkt_event = if msg.matches_query_prefix("MD") {
            ElecraftPacket::Response(msg.clone())
        } else {
            ElecraftPacket::AutoInfoEvent(msg.clone())
        };
        assert_eq!(pkt_event, ElecraftPacket::AutoInfoEvent(msg));
    }

    #[test]
    fn elecraft_codec_deterministic_prefix_omap() {
        let (f1, _) = ElecraftCodec::new()
            .decode_frame(b"OMAP1;")
            .unwrap()
            .unwrap();
        assert_eq!(f1.prefix, "OM");
        assert!(!f1.has_vfo_b_modifier);
        assert_eq!(f1.payload, "AP1");

        let (f2, _) = ElecraftCodec::new().decode_frame(b"OM;").unwrap().unwrap();
        assert_eq!(f2.prefix, "OM");
        assert_eq!(f2.payload, "");

        let (f3, _) = ElecraftCodec::new()
            .decode_frame(b"SMH-073;")
            .unwrap()
            .unwrap();
        assert_eq!(f3.prefix, "SMH");
        assert!(!f3.has_vfo_b_modifier);
        assert_eq!(f3.payload, "-073");

        let (f4, _) = ElecraftCodec::new()
            .decode_frame(b"SMH$+010;")
            .unwrap()
            .unwrap();
        assert_eq!(f4.prefix, "SMH");
        assert!(f4.has_vfo_b_modifier);
        assert_eq!(f4.payload, "+010");
    }

    #[test]
    fn elecraft_codec_power_control_l_and_h_scale() {
        let codec = ElecraftCodec::new();

        // Skala L (dziesiąte części wata): PC050L = 5.0W
        let (f1, _) = codec.decode_frame(b"PC050L;").unwrap().unwrap();
        let m1 = codec
            .decode_message(&f1, ElecraftProtocolFamily::K4)
            .unwrap();
        assert_eq!(
            m1,
            ElecraftMessage::PowerControl {
                watts: 5.0,
                raw_scale: 'L'
            }
        );

        // Skala H (pełne waty): PC100H = 100.0W
        let (f2, _) = codec.decode_frame(b"PC100H;").unwrap().unwrap();
        let m2 = codec
            .decode_message(&f2, ElecraftProtocolFamily::K4)
            .unwrap();
        assert_eq!(
            m2,
            ElecraftMessage::PowerControl {
                watts: 100.0,
                raw_scale: 'H'
            }
        );

        // Skala domyślna bez litery: PC050 = 50.0W
        let (f3, _) = codec.decode_frame(b"PC050;").unwrap().unwrap();
        let m3 = codec
            .decode_message(&f3, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(
            m3,
            ElecraftMessage::PowerControl {
                watts: 50.0,
                raw_scale: 'H'
            }
        );
    }

    #[test]
    fn elecraft_codec_power_output_tenth_watts() {
        let codec = ElecraftCodec::new();

        let (f1, _) = codec.decode_frame(b"PO0100;").unwrap().unwrap();
        let m1 = codec
            .decode_message(&f1, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(m1, ElecraftMessage::PowerOutput(10.0));

        let (f2, _) = codec.decode_frame(b"PO0050;").unwrap().unwrap();
        let m2 = codec
            .decode_message(&f2, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(m2, ElecraftMessage::PowerOutput(5.0));
    }

    #[test]
    fn elecraft_codec_smeter_k3_raw_vs_k4_smh() {
        let codec = ElecraftCodec::new();

        // K3/K3S/KX2/KX3: surowy odczyt słupków
        let (f1, _) = codec.decode_frame(b"SM0015;").unwrap().unwrap();
        let m1 = codec
            .decode_message(&f1, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(
            m1,
            ElecraftMessage::RawSMeter {
                vfo_b: false,
                raw: 15
            }
        );

        let (f2, _) = codec.decode_frame(b"SM$0021;").unwrap().unwrap();
        let m2 = codec
            .decode_message(&f2, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(
            m2,
            ElecraftMessage::RawSMeter {
                vfo_b: true,
                raw: 21
            }
        );

        // K4: precyzyjny odczyt dBm
        let (f3, _) = codec.decode_frame(b"SMH-073;").unwrap().unwrap();
        let m3 = codec
            .decode_message(&f3, ElecraftProtocolFamily::K4)
            .unwrap();
        assert_eq!(
            m3,
            ElecraftMessage::HighResSMeter {
                vfo_b: false,
                dbm: -73.0
            }
        );

        let (f4, _) = codec.decode_frame(b"SMH$+010;").unwrap().unwrap();
        let m4 = codec
            .decode_message(&f4, ElecraftProtocolFamily::K4)
            .unwrap();
        assert_eq!(
            m4,
            ElecraftMessage::HighResSMeter {
                vfo_b: true,
                dbm: 10.0
            }
        );
    }

    #[test]
    fn elecraft_codec_option_module_message() {
        let codec = ElecraftCodec::new();
        let (f, _) = codec.decode_frame(b"OMAP1S;").unwrap().unwrap();
        let m = codec
            .decode_message(&f, ElecraftProtocolFamily::K3Compatible)
            .unwrap();
        assert_eq!(m, ElecraftMessage::OptionModule("AP1S".to_string()));
        assert!(m.matches_query_prefix("OM"));
    }

    #[test]
    fn elecraft_cat_set_power_qrp_and_k4() {
        // KX2 QRP: poniżej 10W używa skali L
        assert_eq!(ElecraftCat::set_power(5.0, 12.0, false, true), "PC050L;");
        assert_eq!(ElecraftCat::set_power(10.0, 12.0, false, true), "PC010;");

        // K4: poniżej 10W używa skali L, powyżej skali H
        assert_eq!(ElecraftCat::set_power(5.0, 105.0, true, false), "PC050L;");
        assert_eq!(ElecraftCat::set_power(100.0, 105.0, true, false), "PC100H;");

        // K3: używa formatu PCnnn;
        assert_eq!(ElecraftCat::set_power(5.0, 120.0, false, false), "PC005;");
        assert_eq!(ElecraftCat::set_power(100.0, 120.0, false, false), "PC100;");
    }
}
