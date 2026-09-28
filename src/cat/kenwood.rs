// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Obsługa protokołu tekstowego Kenwood / Elecraft / Yaesu (CAT).
//!
//! Standard stosowany przez transceivery:
//! - Kenwood: TS-590S/SG, TS-890S, TS-990S, TS-2000, TS-480, TS-570
//! - Elecraft: K2, K3, K3S, KX2, KX3, K4
//! - Yaesu (w trybie CAT tekstowym zgodnym z Kenwood): FT-891, FT-991A, FTDX10, FT-710
//!
//! Komendy kończą się znakiem średnika `;`.

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

    /// Dekoduje wartość S-metra z `SM00015;` (zwraca surową wartość 0..30).
    pub fn parse_smeter_resp(resp: &str) -> Option<u8> {
        let trimmed = resp.trim().trim_end_matches(';');
        if trimmed.starts_with("SM") {
            let digits = trimmed.trim_start_matches(|c: char| !c.is_ascii_digit());
            digits.parse::<u8>().ok()
        } else {
            None
        }
    }

    /// Włącza/wyłącza tryb Split (`FT1;` dla TX na VFO B, `FT0;` dla wyłączenia).
    pub fn set_split_cmd(split: bool) -> &'static str {
        if split { "FT1;" } else { "FT0;" }
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

        assert_eq!(KenwoodCat::parse_mode_resp("MD2;"), Some("USB"));
        assert_eq!(KenwoodCat::parse_mode_resp("MD3;"), Some("CW"));
        assert_eq!(KenwoodCat::parse_mode_resp("MD1;"), Some("LSB"));
    }

    #[test]
    fn test_kenwood_ptt() {
        assert_eq!(KenwoodCat::set_ptt_cmd(true), "TX;");
        assert_eq!(KenwoodCat::set_ptt_cmd(false), "RX;");

        assert_eq!(KenwoodCat::parse_ptt_resp("TX1;"), Some(true));
        assert_eq!(KenwoodCat::parse_ptt_resp("TX0;"), Some(false));
    }

    #[test]
    fn test_kenwood_smeter() {
        assert_eq!(KenwoodCat::read_smeter_cmd(), "SM0;");
        assert_eq!(KenwoodCat::parse_smeter_resp("SM00015;"), Some(15));
    }
}
