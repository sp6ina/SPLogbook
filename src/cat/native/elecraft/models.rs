// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Profile możliwości transceiverów Elecraft dla natywnego backendu CAT.
//!
//! Zdefiniowane w oparciu o oficjalne źródła producenta:
//! 1. *Elecraft K3S, K3, KX3, and KX2 Programmer's Reference*, Rev G5, February 20, 2019.
//! 2. *Elecraft K4 Programmer's Reference*, Rev D12, May 5, 2026.
//!
//! Rozróżnia dwie główne rodziny protokołów:
//! - [`ElecraftProtocolFamily::K3Compatible`] (K3, K3S, KX2, KX3)
//! - [`ElecraftProtocolFamily::K4`] (K4, K4D)

use std::time::Duration;

use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile, RigVendor};
use crate::cat::native::transport::{
    SerialDataBits, SerialFlowControl, SerialParity, SerialStopBits,
};
use crate::cat::rig::RigCapabilities;

/// Rodzina protokołu Elecraft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElecraftProtocolFamily {
    /// Klasyczna architektura K3/K3S/KX3/KX2 oparta o rozszerzenia K31 / Kenwood-like.
    K3Compatible,
    /// Architektura direct-sampling SDR K4 / K4D z obsługą TCP, wielu klientów i K41.
    K4,
}

/// Dedykowany, szczegółowy profil modelu transceivera Elecraft.
#[derive(Debug, Clone, PartialEq)]
pub struct ElecraftModelProfile {
    pub model_id: &'static str,
    pub name: &'static str,
    pub vendor: RigVendor,
    pub family: ElecraftProtocolFamily,
    pub default_baud_rate: u32,
    pub supported_baud_rates: &'static [u32],
    pub data_bits: SerialDataBits,
    pub stop_bits: SerialStopBits,
    pub parity: SerialParity,
    pub flow_control: SerialFlowControl,
    pub command_interval: Duration,
    pub max_message_length: usize,
    pub frequency_digits: usize,
    pub receiver_count: u8,
    pub transmitter_count: u8,
    pub supports_auto_info: bool,
    pub preferred_auto_info_mode: u8,
    pub supports_subreceiver: bool,
    pub supports_vfo_b_modifier: bool,
    pub supports_tcp: bool,
    pub default_tcp_port: Option<u16>,
    pub capabilities: RigCapabilities,
    pub extended_capabilities: NativeRigCapabilities,
    pub safe_poll_commands: &'static [&'static str],
    pub firmware_constraints: &'static str,
}

impl ElecraftModelProfile {
    /// Tworzy ogólny profil [`RigModelProfile`] do rejestru wspólnego Native-CAT.
    pub fn to_rig_model_profile(&self) -> RigModelProfile {
        RigModelProfile {
            model_id: self.model_id,
            name: self.name,
            vendor: self.vendor,
            default_baud_rate: self.default_baud_rate,
            supported_baud_rates: self.supported_baud_rates,
            default_ci_v_address: None,
            frequency_bytes: self.frequency_digits,
            capabilities: self.capabilities,
            extended_capabilities: self.extended_capabilities,
            safe_poll_commands: self.safe_poll_commands,
        }
    }

    /// Weryfikuje, czy dany baud rate jest akceptowany przez profil.
    pub fn supports_baud_rate(&self, baud: u32) -> bool {
        self.supported_baud_rates.contains(&baud)
    }

    /// Mapuje nazwę emisji na kod protokołu Elecraft `MD` i opcjonalny podtryb cyfrowy `DT`.
    /// Zwraca `None`, jeśli emisja nie jest obsługiwana przez radio.
    pub fn map_mode_to_code(&self, mode: &str) -> Option<(&'static str, Option<u8>)> {
        match mode.to_ascii_uppercase().as_str() {
            "LSB" => Some(("1", None)),
            "USB" => Some(("2", None)),
            "CW" | "CW-U" => Some(("3", None)),
            "FM" => Some(("4", None)),
            "AM" => Some(("5", None)),
            "DATA" | "DATA-A" | "FT8" | "FT4" | "JS8" => Some(("6", Some(0))), // DATA A
            "AFSK" | "AFSK-A" => Some(("6", Some(1))),                         // AFSK A
            "FSK" | "FSK-D" | "RTTY" => Some(("6", Some(2))),                  // FSK D
            "PSK" | "PSK-D" => Some(("6", Some(3))),                           // PSK D
            "CW-R" | "CW-REV" | "CW-L" => Some(("7", None)),
            "DATA-R" | "DATA-REV" => Some(("9", None)),
            _ => None,
        }
    }

    /// Dekoduje kod numeryczny emisji `MD` i podtryb `DT` na ustandaryzowaną nazwę emisji SPLogbook.
    pub fn map_code_to_mode(&self, code: char, submode: Option<u8>) -> Option<&'static str> {
        match code {
            '1' => Some("LSB"),
            '2' => Some("USB"),
            '3' => Some("CW"),
            '4' => Some("FM"),
            '5' => Some("AM"),
            '6' => match submode {
                Some(1) => Some("AFSK"),
                Some(2) => Some("RTTY"),
                Some(3) => Some("PSK"),
                _ => Some("DATA"),
            },
            '7' => Some("CW-R"),
            '9' => Some("DATA-R"),
            _ => None,
        }
    }
}

// ============================================================================
// PROFILE REFERENCYJNE ELECRAFT
// ============================================================================

/// Elecraft K3 — flagowy transceiver HF/50MHz z podwójnym odbiornikiem KRX3 (opcja).
pub static ELECRAFT_K3_PROFILE: ElecraftModelProfile = ElecraftModelProfile {
    model_id: "K3",
    name: "Elecraft K3",
    vendor: RigVendor::Elecraft,
    family: ElecraftProtocolFamily::K3Compatible,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    data_bits: SerialDataBits::Eight,
    stop_bits: SerialStopBits::One,
    parity: SerialParity::None,
    flow_control: SerialFlowControl::None,
    command_interval: Duration::from_millis(10),
    max_message_length: 128,
    frequency_digits: 11,
    receiver_count: 2,
    transmitter_count: 1,
    supports_auto_info: true,
    preferred_auto_info_mode: 2,
    supports_subreceiver: true,
    supports_vfo_b_modifier: true,
    supports_tcp: false,
    default_tcp_port: None,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: true,
        power: true,
        // K3 raportuje S-metr w surowych słupkach LCD (0000..0015 / 0021).
        // Brak fabrycznej kalibracji dBm/S-unit w instrukcji Programmer's Ref.
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: true,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["FA", "FB", "MD", "IF", "TQ"],
    firmware_constraints: "MCU 5.0+ dla pełnego zestawu rozszerzeń K31",
};

/// Elecraft K3S — zmodernizowany transceiver HF/50MHz ze zintegrowanym portem USB.
pub static ELECRAFT_K3S_PROFILE: ElecraftModelProfile = ElecraftModelProfile {
    model_id: "K3S",
    name: "Elecraft K3S",
    vendor: RigVendor::Elecraft,
    family: ElecraftProtocolFamily::K3Compatible,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    data_bits: SerialDataBits::Eight,
    stop_bits: SerialStopBits::One,
    parity: SerialParity::None,
    flow_control: SerialFlowControl::None,
    command_interval: Duration::from_millis(10),
    max_message_length: 128,
    frequency_digits: 11,
    receiver_count: 2,
    transmitter_count: 1,
    supports_auto_info: true,
    preferred_auto_info_mode: 2,
    supports_subreceiver: true,
    supports_vfo_b_modifier: true,
    supports_tcp: false,
    default_tcp_port: None,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: true,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["FA", "FB", "MD", "IF", "TQ"],
    firmware_constraints: "MCU 5.0+",
};

/// Elecraft KX2 — ultrakompaktowy transceiver QRP HF z pojedynczym odbiornikiem.
pub static ELECRAFT_KX2_PROFILE: ElecraftModelProfile = ElecraftModelProfile {
    model_id: "KX2",
    name: "Elecraft KX2",
    vendor: RigVendor::Elecraft,
    family: ElecraftProtocolFamily::K3Compatible,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    data_bits: SerialDataBits::Eight,
    stop_bits: SerialStopBits::One,
    parity: SerialParity::None,
    flow_control: SerialFlowControl::None,
    command_interval: Duration::from_millis(15),
    max_message_length: 128,
    frequency_digits: 11,
    receiver_count: 1,
    transmitter_count: 1,
    supports_auto_info: true,
    preferred_auto_info_mode: 2,
    // KX2 jest urządzeniem jednoukładowym bez modułu drugiego odbiornika
    supports_subreceiver: false,
    supports_vfo_b_modifier: false,
    supports_tcp: false,
    default_tcp_port: None,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: false,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: false,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["FA", "FB", "MD", "IF", "TQ"],
    firmware_constraints: "MCU 1.0+",
};

/// Elecraft KX3 — przenośny transceiver QRP HF/50MHz z DSP dual-watch w jednym paśmie.
pub static ELECRAFT_KX3_PROFILE: ElecraftModelProfile = ElecraftModelProfile {
    model_id: "KX3",
    name: "Elecraft KX3",
    vendor: RigVendor::Elecraft,
    family: ElecraftProtocolFamily::K3Compatible,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    data_bits: SerialDataBits::Eight,
    stop_bits: SerialStopBits::One,
    parity: SerialParity::None,
    flow_control: SerialFlowControl::None,
    command_interval: Duration::from_millis(15),
    max_message_length: 128,
    frequency_digits: 11,
    receiver_count: 1,
    transmitter_count: 1,
    supports_auto_info: true,
    preferred_auto_info_mode: 2,
    // KX3 nie ma fizycznego niezależnego drugiego toru w.cz. (tylko programowy dual-watch w tym samym paśmie)
    supports_subreceiver: false,
    supports_vfo_b_modifier: false,
    supports_tcp: false,
    default_tcp_port: None,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: false,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["FA", "FB", "MD", "IF", "TQ"],
    firmware_constraints: "MCU 2.0+",
};

/// Elecraft K4 — bezpośrednio próbkujący transceiver SDR HF/50/70MHz ze złączem USB i Ethernet TCP.
pub static ELECRAFT_K4_PROFILE: ElecraftModelProfile = ElecraftModelProfile {
    model_id: "K4",
    name: "Elecraft K4",
    vendor: RigVendor::Elecraft,
    family: ElecraftProtocolFamily::K4,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 115_200],
    data_bits: SerialDataBits::Eight,
    stop_bits: SerialStopBits::One,
    parity: SerialParity::None,
    flow_control: SerialFlowControl::None,
    command_interval: Duration::from_millis(5),
    max_message_length: 256,
    frequency_digits: 11,
    receiver_count: 2,
    transmitter_count: 1,
    supports_auto_info: true,
    preferred_auto_info_mode: 2,
    supports_subreceiver: true,
    supports_vfo_b_modifier: true,
    supports_tcp: true,
    default_tcp_port: Some(9200),
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: true,
        power: true,
        // K4 udostępnia skalibrowany odczyt sygnału w dBm przez rozkaz SMH / SMH$
        s_meter: true,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: true,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["FA", "FB", "MD", "IF", "TQ", "SMH"],
    firmware_constraints: "K4 OS 2.0+",
};

/// Elecraft K4D — wersja z fabrycznie zainstalowanym w pełni niezależnym drugim odbiornikiem KRX4.
pub static ELECRAFT_K4D_PROFILE: ElecraftModelProfile = ElecraftModelProfile {
    model_id: "K4D",
    name: "Elecraft K4D",
    vendor: RigVendor::Elecraft,
    family: ElecraftProtocolFamily::K4,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 115_200],
    data_bits: SerialDataBits::Eight,
    stop_bits: SerialStopBits::One,
    parity: SerialParity::None,
    flow_control: SerialFlowControl::None,
    command_interval: Duration::from_millis(5),
    max_message_length: 256,
    frequency_digits: 11,
    receiver_count: 2,
    transmitter_count: 1,
    supports_auto_info: true,
    preferred_auto_info_mode: 2,
    supports_subreceiver: true,
    supports_vfo_b_modifier: true,
    supports_tcp: true,
    default_tcp_port: Some(9200),
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: true,
        power: true,
        s_meter: true,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: true,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["FA", "FB", "MD", "IF", "TQ", "SMH"],
    firmware_constraints: "K4 OS 2.0+",
};

// Statyczne referencje dla rejestru Native CAT:
pub static ELECRAFT_K3: RigModelProfile = RigModelProfile {
    model_id: "K3",
    name: "Elecraft K3",
    vendor: RigVendor::Elecraft,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    default_ci_v_address: None,
    frequency_bytes: 11,
    capabilities: ELECRAFT_K3_PROFILE.capabilities,
    extended_capabilities: ELECRAFT_K3_PROFILE.extended_capabilities,
    safe_poll_commands: ELECRAFT_K3_PROFILE.safe_poll_commands,
};

pub static ELECRAFT_K3S: RigModelProfile = RigModelProfile {
    model_id: "K3S",
    name: "Elecraft K3S",
    vendor: RigVendor::Elecraft,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    default_ci_v_address: None,
    frequency_bytes: 11,
    capabilities: ELECRAFT_K3S_PROFILE.capabilities,
    extended_capabilities: ELECRAFT_K3S_PROFILE.extended_capabilities,
    safe_poll_commands: ELECRAFT_K3S_PROFILE.safe_poll_commands,
};

pub static ELECRAFT_KX2: RigModelProfile = RigModelProfile {
    model_id: "KX2",
    name: "Elecraft KX2",
    vendor: RigVendor::Elecraft,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    default_ci_v_address: None,
    frequency_bytes: 11,
    capabilities: ELECRAFT_KX2_PROFILE.capabilities,
    extended_capabilities: ELECRAFT_KX2_PROFILE.extended_capabilities,
    safe_poll_commands: ELECRAFT_KX2_PROFILE.safe_poll_commands,
};

pub static ELECRAFT_KX3: RigModelProfile = RigModelProfile {
    model_id: "KX3",
    name: "Elecraft KX3",
    vendor: RigVendor::Elecraft,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400],
    default_ci_v_address: None,
    frequency_bytes: 11,
    capabilities: ELECRAFT_KX3_PROFILE.capabilities,
    extended_capabilities: ELECRAFT_KX3_PROFILE.extended_capabilities,
    safe_poll_commands: ELECRAFT_KX3_PROFILE.safe_poll_commands,
};

pub static ELECRAFT_K4: RigModelProfile = RigModelProfile {
    model_id: "K4",
    name: "Elecraft K4",
    vendor: RigVendor::Elecraft,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 115_200],
    default_ci_v_address: None,
    frequency_bytes: 11,
    capabilities: ELECRAFT_K4_PROFILE.capabilities,
    extended_capabilities: ELECRAFT_K4_PROFILE.extended_capabilities,
    safe_poll_commands: ELECRAFT_K4_PROFILE.safe_poll_commands,
};

pub static ELECRAFT_K4D: RigModelProfile = RigModelProfile {
    model_id: "K4D",
    name: "Elecraft K4D",
    vendor: RigVendor::Elecraft,
    default_baud_rate: 38400,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 115_200],
    default_ci_v_address: None,
    frequency_bytes: 11,
    capabilities: ELECRAFT_K4D_PROFILE.capabilities,
    extended_capabilities: ELECRAFT_K4D_PROFILE.extended_capabilities,
    safe_poll_commands: ELECRAFT_K4D_PROFILE.safe_poll_commands,
};

/// Wszystkie oficjalne profile Elecraft.
pub static ALL_ELECRAFT_PROFILES: &[&ElecraftModelProfile] = &[
    &ELECRAFT_K3_PROFILE,
    &ELECRAFT_K3S_PROFILE,
    &ELECRAFT_KX2_PROFILE,
    &ELECRAFT_KX3_PROFILE,
    &ELECRAFT_K4_PROFILE,
    &ELECRAFT_K4D_PROFILE,
];

/// Wyszukuje profil Elecraft po nazwie modelu.
pub fn find_elecraft_profile(name: &str) -> Option<&'static ElecraftModelProfile> {
    let query = name.trim().to_ascii_uppercase();
    ALL_ELECRAFT_PROFILES.iter().copied().find(|p| {
        p.model_id.to_ascii_uppercase() == query
            || p.name.to_ascii_uppercase() == query
            || query.contains(&p.model_id.to_ascii_uppercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elecraft_k3_profile_matches_manual() {
        let p = &ELECRAFT_K3_PROFILE;
        assert_eq!(p.model_id, "K3");
        assert_eq!(p.family, ElecraftProtocolFamily::K3Compatible);
        assert_eq!(p.default_baud_rate, 38400);
        assert_eq!(p.supported_baud_rates, &[4800, 9600, 19200, 38400]);
        assert_eq!(p.frequency_digits, 11);
        assert!(p.supports_auto_info);
        assert_eq!(p.preferred_auto_info_mode, 2);
        assert!(p.supports_subreceiver);
        assert!(p.supports_vfo_b_modifier);
        assert!(!p.supports_tcp);
        assert_eq!(p.default_tcp_port, None);
        assert!(p.capabilities.frequency);
        assert!(p.capabilities.mode);
        assert!(p.capabilities.ptt);
        assert!(p.capabilities.power);
        assert!(!p.capabilities.s_meter); // K3 raw bars only
    }

    #[test]
    fn elecraft_k3s_profile_matches_manual() {
        let p = &ELECRAFT_K3S_PROFILE;
        assert_eq!(p.model_id, "K3S");
        assert_eq!(p.family, ElecraftProtocolFamily::K3Compatible);
        assert_eq!(p.default_baud_rate, 38400);
        assert_eq!(p.supported_baud_rates, &[4800, 9600, 19200, 38400]);
        assert!(p.supports_subreceiver);
        assert!(p.supports_vfo_b_modifier);
        assert!(!p.supports_tcp);
        assert!(!p.capabilities.s_meter);
    }

    #[test]
    fn elecraft_kx2_profile_matches_manual() {
        let p = &ELECRAFT_KX2_PROFILE;
        assert_eq!(p.model_id, "KX2");
        assert_eq!(p.family, ElecraftProtocolFamily::K3Compatible);
        assert_eq!(p.default_baud_rate, 38400);
        assert_eq!(p.receiver_count, 1);
        assert!(!p.supports_subreceiver);
        assert!(!p.supports_vfo_b_modifier);
        assert!(!p.extended_capabilities.dualwatch);
        assert!(!p.extended_capabilities.antenna_select);
        assert!(!p.supports_tcp);
    }

    #[test]
    fn elecraft_kx3_profile_matches_manual() {
        let p = &ELECRAFT_KX3_PROFILE;
        assert_eq!(p.model_id, "KX3");
        assert_eq!(p.family, ElecraftProtocolFamily::K3Compatible);
        assert_eq!(p.default_baud_rate, 38400);
        assert_eq!(p.receiver_count, 1);
        assert!(!p.supports_subreceiver);
        assert!(!p.supports_vfo_b_modifier);
        assert!(p.extended_capabilities.dualwatch); // In-band DSP dual-watch
        assert!(!p.extended_capabilities.antenna_select);
        assert!(!p.supports_tcp);
    }

    #[test]
    fn elecraft_k4_profile_matches_manual() {
        let p = &ELECRAFT_K4_PROFILE;
        assert_eq!(p.model_id, "K4");
        assert_eq!(p.family, ElecraftProtocolFamily::K4);
        assert_eq!(p.default_baud_rate, 38400);
        assert_eq!(p.supported_baud_rates, &[4800, 9600, 19200, 38400, 115_200]);
        assert!(p.supports_tcp);
        assert_eq!(p.default_tcp_port, Some(9200));
        assert!(p.supports_subreceiver);
        assert!(p.supports_vfo_b_modifier);
        assert!(p.capabilities.s_meter); // K4 SMH provides calibrated dBm
        assert!(p.extended_capabilities.dualwatch);
        assert!(p.extended_capabilities.antenna_select);
    }

    #[test]
    fn elecraft_profiles_have_explicit_serial_parameters() {
        for profile in ALL_ELECRAFT_PROFILES {
            assert_eq!(profile.data_bits, SerialDataBits::Eight);
            assert_eq!(profile.stop_bits, SerialStopBits::One);
            assert_eq!(profile.parity, SerialParity::None);
            assert_eq!(profile.flow_control, SerialFlowControl::None);
            assert!(profile.supports_baud_rate(38400));
        }
    }

    #[test]
    fn elecraft_profiles_do_not_claim_no_effect_commands() {
        // KX2 nie deklaruje obsługi subreceivera ani wyboru anteny
        assert!(!ELECRAFT_KX2_PROFILE.supports_subreceiver);
        assert!(!ELECRAFT_KX2_PROFILE.extended_capabilities.antenna_select);
        assert!(!ELECRAFT_KX2_PROFILE.extended_capabilities.dualwatch);

        // KX3 nie deklaruje wyboru anteny w menu w.cz.
        assert!(!ELECRAFT_KX3_PROFILE.extended_capabilities.antenna_select);
        assert!(!ELECRAFT_KX3_PROFILE.supports_subreceiver);
    }

    #[test]
    fn elecraft_subreceiver_capability_is_profile_specific() {
        assert!(ELECRAFT_K3_PROFILE.supports_subreceiver);
        assert!(ELECRAFT_K3S_PROFILE.supports_subreceiver);
        assert!(!ELECRAFT_KX2_PROFILE.supports_subreceiver);
        assert!(!ELECRAFT_KX3_PROFILE.supports_subreceiver);
        assert!(ELECRAFT_K4_PROFILE.supports_subreceiver);
        assert!(ELECRAFT_K4D_PROFILE.supports_subreceiver);
    }

    #[test]
    fn elecraft_vfo_b_modifier_is_profile_specific() {
        assert!(ELECRAFT_K3_PROFILE.supports_vfo_b_modifier);
        assert!(ELECRAFT_K3S_PROFILE.supports_vfo_b_modifier);
        assert!(!ELECRAFT_KX2_PROFILE.supports_vfo_b_modifier);
        assert!(!ELECRAFT_KX3_PROFILE.supports_vfo_b_modifier);
        assert!(ELECRAFT_K4_PROFILE.supports_vfo_b_modifier);
        assert!(ELECRAFT_K4D_PROFILE.supports_vfo_b_modifier);
    }

    #[test]
    fn elecraft_streaming_not_advertised_as_rig_state() {
        // Ani RigState, ani RigCapabilities nie zawierają strumieni IQ/Audio/Panadapter
        for profile in ALL_ELECRAFT_PROFILES {
            // Standardowe capabilities CAT nie mogą reklamować panadaptera jako stanu
            assert!(profile.capabilities.frequency);
            assert!(profile.capabilities.mode);
        }
    }

    #[test]
    fn elecraft_meter_semantics_are_explicit() {
        // K3, K3S, KX2, KX3 nie deklarują kalibracji s_meter w RigCapabilities
        assert!(!ELECRAFT_K3_PROFILE.capabilities.s_meter);
        assert!(!ELECRAFT_K3S_PROFILE.capabilities.s_meter);
        assert!(!ELECRAFT_KX2_PROFILE.capabilities.s_meter);
        assert!(!ELECRAFT_KX3_PROFILE.capabilities.s_meter);

        // K4 ma potwierdzoną w Programmer's Ref komendę SMH (dBm)
        assert!(ELECRAFT_K4_PROFILE.capabilities.s_meter);
        assert!(ELECRAFT_K4D_PROFILE.capabilities.s_meter);
    }

    #[test]
    fn elecraft_k4_tcp_capability_matches_manual() {
        assert!(ELECRAFT_K4_PROFILE.supports_tcp);
        assert_eq!(ELECRAFT_K4_PROFILE.default_tcp_port, Some(9200));
        assert!(ELECRAFT_K4D_PROFILE.supports_tcp);
        assert_eq!(ELECRAFT_K4D_PROFILE.default_tcp_port, Some(9200));

        assert!(!ELECRAFT_K3_PROFILE.supports_tcp);
        assert_eq!(ELECRAFT_K3_PROFILE.default_tcp_port, None);
        assert!(!ELECRAFT_KX2_PROFILE.supports_tcp);
        assert!(!ELECRAFT_KX3_PROFILE.supports_tcp);
    }
}
