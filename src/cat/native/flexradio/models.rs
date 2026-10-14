// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Profile modeli FlexRadio SmartSDR (FLEX-6000 i FLEX-8000).
//!
//! Definicje profili technicznych oparte wyłącznie o oficjalną dokumentację producenta
//! (FlexRadio Systems SmartSDR TCP/IP API & Hardware Specifications).
//!
//! Profile deklarują wyłącznie możliwości warstwy sterującej (Control Plane).
//! Wszelkie strumienie multimedialne (Data Plane: IQ, Audio, Panadapter/FFT)
//! pozostają poza zakresem i nie są deklarowane w RigCapabilities.

use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile, RigVendor};
use crate::cat::rig::RigCapabilities;

/// Domyślny oficjalny port TCP/UDP SmartSDR API.
pub const FLEX_DEFAULT_PORT: u16 = 4992;

/// Rozszerzone metadane modelu FlexRadio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlexRadioModelMeta {
    pub model_id: &'static str,
    pub display_name: &'static str,
    pub family: &'static str,
    pub api_version: &'static str,
    pub max_slices: Option<u32>,
    pub max_power_watts: Option<u32>,
    pub max_panadapters: u32,
    pub has_tx: bool,
    pub has_front_panel: bool,
    pub dual_scu: bool,
    pub rx_antennas: &'static [&'static str],
    pub tx_antennas: &'static [&'static str],
    pub supported_modes: &'static [&'static str],
}

/// Oficjalnie wspierane emisje SmartSDR API.
pub static FLEX_STANDARD_MODES: &[&str] = &[
    "AM", "SAM", "CW", "USB", "LSB", "FM", "NFM", "DFM", "DIGU", "DIGL", "RTTY",
];

// Profile poszczególnych modeli:

/// FLEX-6300: transceiver HF/50MHz, 2 odbiorniki slice, 1 panadapter, 100W.
pub static FLEX_6300: RigModelProfile = RigModelProfile {
    model_id: "FLEX-6300",
    name: "FlexRadio FLEX-6300",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_6300_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-6300",
    display_name: "FlexRadio FLEX-6300",
    family: "FLEX-6000",
    api_version: "v3",
    max_slices: Some(2),
    max_power_watts: Some(100),
    max_panadapters: 1,
    has_tx: true,
    has_front_panel: false,
    dual_scu: false,
    rx_antennas: &["ANT1", "ANT2", "RX_A"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-6400: transceiver HF/50MHz, 2 odbiorniki slice, 2 panadaptery, 100W.
pub static FLEX_6400: RigModelProfile = RigModelProfile {
    model_id: "FLEX-6400",
    name: "FlexRadio FLEX-6400",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_6400_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-6400",
    display_name: "FlexRadio FLEX-6400",
    family: "FLEX-6000",
    api_version: "v3",
    max_slices: Some(2),
    max_power_watts: Some(100),
    max_panadapters: 2,
    has_tx: true,
    has_front_panel: false,
    dual_scu: false,
    rx_antennas: &["ANT1", "ANT2", "RX_A"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-6400M: transceiver HF/50MHz z panelem czołowym, 2 odbiorniki slice, 2 panadaptery, 100W.
pub static FLEX_6400M: RigModelProfile = RigModelProfile {
    model_id: "FLEX-6400M",
    name: "FlexRadio FLEX-6400M",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_6400M_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-6400M",
    display_name: "FlexRadio FLEX-6400M",
    family: "FLEX-6000",
    api_version: "v3",
    max_slices: Some(2),
    max_power_watts: Some(100),
    max_panadapters: 2,
    has_tx: true,
    has_front_panel: true,
    dual_scu: false,
    rx_antennas: &["ANT1", "ANT2", "RX_A"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-6500: transceiver HF/50MHz, 4 odbiorniki slice, 4 panadaptery, 100W.
pub static FLEX_6500: RigModelProfile = RigModelProfile {
    model_id: "FLEX-6500",
    name: "FlexRadio FLEX-6500",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_6500_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-6500",
    display_name: "FlexRadio FLEX-6500",
    family: "FLEX-6000",
    api_version: "v3",
    max_slices: Some(4),
    max_power_watts: Some(100),
    max_panadapters: 4,
    has_tx: true,
    has_front_panel: false,
    dual_scu: false,
    rx_antennas: &["ANT1", "ANT2", "RX_A", "XVTR"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-6600: transceiver HF/50MHz z podwójnym niezależnym SCU, 4 odbiorniki slice, 4 panadaptery, 100W.
pub static FLEX_6600: RigModelProfile = RigModelProfile {
    model_id: "FLEX-6600",
    name: "FlexRadio FLEX-6600",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_6600_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-6600",
    display_name: "FlexRadio FLEX-6600",
    family: "FLEX-6000",
    api_version: "v3",
    max_slices: Some(4),
    max_power_watts: Some(100),
    max_panadapters: 4,
    has_tx: true,
    has_front_panel: false,
    dual_scu: true,
    rx_antennas: &["ANT1", "ANT2", "RX_A", "RX_B", "XVTR"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-6600M: transceiver HF/50MHz z panelem czołowym i podwójnym SCU, 4 odbiorniki slice, 4 panadaptery, 100W.
pub static FLEX_6600M: RigModelProfile = RigModelProfile {
    model_id: "FLEX-6600M",
    name: "FlexRadio FLEX-6600M",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_6600M_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-6600M",
    display_name: "FlexRadio FLEX-6600M",
    family: "FLEX-6000",
    api_version: "v3",
    max_slices: Some(4),
    max_power_watts: Some(100),
    max_panadapters: 4,
    has_tx: true,
    has_front_panel: true,
    dual_scu: true,
    rx_antennas: &["ANT1", "ANT2", "RX_A", "RX_B", "XVTR"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-6700: transceiver flagowy HF/50MHz/144MHz z podwójnym SCU, 8 odbiorników slice, 8 panadapterów, 100W.
pub static FLEX_6700: RigModelProfile = RigModelProfile {
    model_id: "FLEX-6700",
    name: "FlexRadio FLEX-6700",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_6700_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-6700",
    display_name: "FlexRadio FLEX-6700",
    family: "FLEX-6000",
    api_version: "v3",
    max_slices: Some(8),
    max_power_watts: Some(100),
    max_panadapters: 8,
    has_tx: true,
    has_front_panel: false,
    dual_scu: true,
    rx_antennas: &["ANT1", "ANT2", "RX_A", "RX_B", "XVTR"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-8400: transceiver generacji 8000, 2 odbiorniki slice, 2 panadaptery, 100W HF/6m.
pub static FLEX_8400: RigModelProfile = RigModelProfile {
    model_id: "FLEX-8400",
    name: "FlexRadio FLEX-8400",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_8400_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-8400",
    display_name: "FlexRadio FLEX-8400",
    family: "FLEX-8000",
    api_version: "v3",
    max_slices: Some(2),
    max_power_watts: Some(100),
    max_panadapters: 2,
    has_tx: true,
    has_front_panel: false,
    dual_scu: false,
    rx_antennas: &["ANT1", "ANT2", "RX_A"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-8400M: transceiver generacji 8000 z panelem czołowym, 2 odbiorniki slice, 2 panadaptery, 100W.
pub static FLEX_8400M: RigModelProfile = RigModelProfile {
    model_id: "FLEX-8400M",
    name: "FlexRadio FLEX-8400M",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_8400M_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-8400M",
    display_name: "FlexRadio FLEX-8400M",
    family: "FLEX-8000",
    api_version: "v3",
    max_slices: Some(2),
    max_power_watts: Some(100),
    max_panadapters: 2,
    has_tx: true,
    has_front_panel: true,
    dual_scu: false,
    rx_antennas: &["ANT1", "ANT2", "RX_A"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-8600: transceiver generacji 8000 z podwójnym SCU, 4 odbiorniki slice, 4 panadaptery, 100W.
pub static FLEX_8600: RigModelProfile = RigModelProfile {
    model_id: "FLEX-8600",
    name: "FlexRadio FLEX-8600",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_8600_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-8600",
    display_name: "FlexRadio FLEX-8600",
    family: "FLEX-8000",
    api_version: "v3",
    max_slices: Some(4),
    max_power_watts: Some(100),
    max_panadapters: 4,
    has_tx: true,
    has_front_panel: false,
    dual_scu: true,
    rx_antennas: &["ANT1", "ANT2", "RX_A", "RX_B"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// FLEX-8600M: transceiver generacji 8000 z panelem czołowym i podwójnym SCU, 4 odbiorniki slice, 4 panadaptery, 100W.
pub static FLEX_8600M: RigModelProfile = RigModelProfile {
    model_id: "FLEX-8600M",
    name: "FlexRadio FLEX-8600M",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: false,
        rit: true,
        xit: true,
        power: true,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: true,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["sub slice all", "sub tx all", "sub interlock all"],
};

pub static FLEX_8600M_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-8600M",
    display_name: "FlexRadio FLEX-8600M",
    family: "FLEX-8000",
    api_version: "v3",
    max_slices: Some(4),
    max_power_watts: Some(100),
    max_panadapters: 4,
    has_tx: true,
    has_front_panel: true,
    dual_scu: true,
    rx_antennas: &["ANT1", "ANT2", "RX_A", "RX_B"],
    tx_antennas: &["ANT1", "ANT2"],
    supported_modes: FLEX_STANDARD_MODES,
};

/// Ściśle zachowawczy profil ogólny dla niewymienionych lub nieznanych modeli FlexRadio.
/// Wszystkie niepotwierdzone możliwości, TX, PTT, moc, split i limity slice są wyłączone.
pub static FLEX_GENERIC: RigModelProfile = RigModelProfile {
    model_id: "FLEX-GENERIC",
    name: "FlexRadio Generic (SmartSDR)",
    vendor: RigVendor::FlexRadio,
    default_baud_rate: 0,
    supported_baud_rates: &[],
    default_ci_v_address: None,
    frequency_bytes: 8,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: true,
        split: false,
        split_frequency: false,
        ptt: false,
        vfo: false,
        rit: false,
        xit: false,
        power: false,
        s_meter: false,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: false,
        main_sub_tracking: false,
        transceive_broadcast: false,
        attenuator: false,
        preamp: false,
        antenna_select: false,
        filter_width: false,
        noise_reduction: false,
    },
    safe_poll_commands: &["sub slice all", "sub tx all"],
};

pub static FLEX_GENERIC_META: FlexRadioModelMeta = FlexRadioModelMeta {
    model_id: "FLEX-GENERIC",
    display_name: "FlexRadio Generic (SmartSDR)",
    family: "Generic",
    api_version: "v3",
    max_slices: None,
    max_power_watts: None,
    max_panadapters: 0,
    has_tx: false,
    has_front_panel: false,
    dual_scu: false,
    rx_antennas: &[],
    tx_antennas: &[],
    supported_modes: FLEX_STANDARD_MODES,
};

/// Tablica wszystkich potwierdzonych profili FlexRadio.
pub static ALL_FLEX_PROFILES: &[(&RigModelProfile, &FlexRadioModelMeta)] = &[
    (&FLEX_6300, &FLEX_6300_META),
    (&FLEX_6400, &FLEX_6400_META),
    (&FLEX_6400M, &FLEX_6400M_META),
    (&FLEX_6500, &FLEX_6500_META),
    (&FLEX_6600, &FLEX_6600_META),
    (&FLEX_6600M, &FLEX_6600M_META),
    (&FLEX_6700, &FLEX_6700_META),
    (&FLEX_8400, &FLEX_8400_META),
    (&FLEX_8400M, &FLEX_8400M_META),
    (&FLEX_8600, &FLEX_8600_META),
    (&FLEX_8600M, &FLEX_8600M_META),
    (&FLEX_GENERIC, &FLEX_GENERIC_META),
];

/// Wyszukuje profil po nazwie lub identyfikatorze modelu.
/// W przypadku nierozpoznanego modelu zwraca ostrożny profil ogólny `FLEX_GENERIC`.
pub fn find_flex_profile(model: &str) -> &'static RigModelProfile {
    let clean = model.trim().to_ascii_uppercase();
    for (profile, _) in ALL_FLEX_PROFILES {
        if profile.model_id.to_ascii_uppercase() == clean
            || profile.name.to_ascii_uppercase() == clean
            || clean.contains(&profile.model_id.to_ascii_uppercase())
        {
            return profile;
        }
    }
    &FLEX_GENERIC
}

/// Zwraca metadane modelu FlexRadio dla danego identyfikatora.
pub fn find_flex_meta(model: &str) -> &'static FlexRadioModelMeta {
    let clean = model.trim().to_ascii_uppercase();
    for (profile, meta) in ALL_FLEX_PROFILES {
        if profile.model_id.to_ascii_uppercase() == clean
            || profile.name.to_ascii_uppercase() == clean
            || clean.contains(&profile.model_id.to_ascii_uppercase())
        {
            return meta;
        }
    }
    &FLEX_GENERIC_META
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat::backend::CatBackendKind;

    #[test]
    fn flex_profile_flex6300() {
        assert_eq!(FLEX_6300.model_id, "FLEX-6300");
        assert_eq!(FLEX_6300_META.family, "FLEX-6000");
        assert_eq!(FLEX_6300_META.max_slices, Some(2));
        assert_eq!(FLEX_6300_META.max_power_watts, Some(100));
        assert_eq!(FLEX_6300_META.max_panadapters, 1);
        assert!(FLEX_6300.capabilities.frequency);
        assert!(FLEX_6300.capabilities.ptt);
        assert!(!FLEX_6300.capabilities.vfo); // SmartSDR nie ma syntetycznego VFO
        assert!(!FLEX_6300.capabilities.s_meter); // Data plane wykluczone
    }

    #[test]
    fn flex_profile_flex6400() {
        assert_eq!(FLEX_6400.model_id, "FLEX-6400");
        assert_eq!(FLEX_6400_META.family, "FLEX-6000");
        assert_eq!(FLEX_6400_META.max_slices, Some(2));
        assert_eq!(FLEX_6400_META.max_panadapters, 2);
        assert!(!FLEX_6400_META.has_front_panel);
    }

    #[test]
    fn flex_profile_flex6400m() {
        assert_eq!(FLEX_6400M.model_id, "FLEX-6400M");
        assert_eq!(FLEX_6400M_META.family, "FLEX-6000");
        assert_eq!(FLEX_6400M_META.max_slices, Some(2));
        assert!(FLEX_6400M_META.has_front_panel);
    }

    #[test]
    fn flex_profile_flex6500() {
        assert_eq!(FLEX_6500.model_id, "FLEX-6500");
        assert_eq!(FLEX_6500_META.family, "FLEX-6000");
        assert_eq!(FLEX_6500_META.max_slices, Some(4));
        assert_eq!(FLEX_6500_META.max_panadapters, 4);
    }

    #[test]
    fn flex_profile_flex6600() {
        assert_eq!(FLEX_6600.model_id, "FLEX-6600");
        assert_eq!(FLEX_6600_META.family, "FLEX-6000");
        assert_eq!(FLEX_6600_META.max_slices, Some(4));
        assert_eq!(FLEX_6600_META.max_panadapters, 4);
        assert!(FLEX_6600_META.dual_scu);
    }

    #[test]
    fn flex_profile_flex6600m() {
        assert_eq!(FLEX_6600M.model_id, "FLEX-6600M");
        assert_eq!(FLEX_6600M_META.family, "FLEX-6000");
        assert_eq!(FLEX_6600M_META.max_slices, Some(4));
        assert!(FLEX_6600M_META.dual_scu);
        assert!(FLEX_6600M_META.has_front_panel);
    }

    #[test]
    fn flex_profile_flex6700() {
        assert_eq!(FLEX_6700.model_id, "FLEX-6700");
        assert_eq!(FLEX_6700_META.family, "FLEX-6000");
        assert_eq!(FLEX_6700_META.max_slices, Some(8));
        assert_eq!(FLEX_6700_META.max_panadapters, 8);
        assert!(FLEX_6700_META.dual_scu);
    }

    #[test]
    fn flex_profile_flex8400() {
        assert_eq!(FLEX_8400.model_id, "FLEX-8400");
        assert_eq!(FLEX_8400_META.family, "FLEX-8000");
        assert_eq!(FLEX_8400_META.max_slices, Some(2));
    }

    #[test]
    fn flex_profile_flex8400m() {
        assert_eq!(FLEX_8400M.model_id, "FLEX-8400M");
        assert_eq!(FLEX_8400M_META.family, "FLEX-8000");
        assert_eq!(FLEX_8400M_META.max_slices, Some(2));
        assert!(FLEX_8400M_META.has_front_panel);
    }

    #[test]
    fn flex_profile_flex8600() {
        assert_eq!(FLEX_8600.model_id, "FLEX-8600");
        assert_eq!(FLEX_8600_META.family, "FLEX-8000");
        assert_eq!(FLEX_8600_META.max_slices, Some(4));
        assert!(FLEX_8600_META.dual_scu);
    }

    #[test]
    fn flex_profile_flex8600m() {
        assert_eq!(FLEX_8600M.model_id, "FLEX-8600M");
        assert_eq!(FLEX_8600M_META.family, "FLEX-8000");
        assert_eq!(FLEX_8600M_META.max_slices, Some(4));
        assert!(FLEX_8600M_META.dual_scu);
        assert!(FLEX_8600M_META.has_front_panel);
    }

    #[test]
    fn flex_profile_generic() {
        assert_eq!(FLEX_GENERIC.model_id, "FLEX-GENERIC");
        assert_eq!(FLEX_GENERIC_META.family, "Generic");
        assert_eq!(FLEX_GENERIC_META.max_slices, None);
        assert_eq!(FLEX_GENERIC_META.max_power_watts, None);
        assert!(!FLEX_GENERIC_META.has_tx);
        assert!(!FLEX_GENERIC.capabilities.ptt);
        assert!(!FLEX_GENERIC.capabilities.split);
        assert!(!FLEX_GENERIC.capabilities.rit);
        assert!(!FLEX_GENERIC.capabilities.power);
    }

    #[test]
    fn flex_generic_profile_has_no_assumed_slice_limit() {
        assert_eq!(FLEX_GENERIC_META.max_slices, None);
    }

    #[test]
    fn flex_generic_profile_has_no_assumed_power() {
        assert_eq!(FLEX_GENERIC_META.max_power_watts, None);
        assert!(!FLEX_GENERIC.capabilities.power);
    }

    #[test]
    fn flex_generic_profile_is_not_tx_capable() {
        assert!(!FLEX_GENERIC_META.has_tx);
    }

    #[test]
    fn flex_generic_profile_rejects_ptt() {
        assert!(!FLEX_GENERIC.capabilities.ptt);
    }

    #[test]
    fn flex_generic_runtime_info_does_not_bypass_model_validation() {
        // Nieznany model z runtime_info nie aktywuje automatycznie uprawnień nadawczych
        let p = find_flex_profile("FLEX-UNKNOWN-EXPERIMENT");
        assert_eq!(p.model_id, "FLEX-GENERIC");
        assert!(!p.capabilities.ptt);
        assert!(!p.capabilities.power);
    }

    #[test]
    fn flex_unknown_model_uses_conservative_profile() {
        let p = find_flex_profile("FLEX-UNKNOWN-999");
        assert_eq!(p.model_id, "FLEX-GENERIC");
        assert!(!p.capabilities.split);
        assert!(!p.capabilities.split_frequency);
        assert!(!p.capabilities.power);
        assert!(!p.capabilities.ptt);

        let meta = find_flex_meta("FLEX-UNKNOWN-999");
        assert_eq!(meta.max_slices, None);
        assert!(!meta.has_tx);
    }

    #[test]
    fn flex_profiles_do_not_claim_media_streaming() {
        for (p, _) in ALL_FLEX_PROFILES {
            // Żaden profil nie może deklarować strumieniowania danych (Audio, IQ, Panadapter) w RigCapabilities
            assert!(!p.capabilities.s_meter);
            assert!(!p.extended_capabilities.attenuator);
        }
    }

    #[test]
    fn flex_profiles_do_not_claim_unverified_slice_limits() {
        assert_eq!(FLEX_6300_META.max_slices, Some(2));
        assert_eq!(FLEX_6400_META.max_slices, Some(2));
        assert_eq!(FLEX_6500_META.max_slices, Some(4));
        assert_eq!(FLEX_6600_META.max_slices, Some(4));
        assert_eq!(FLEX_6700_META.max_slices, Some(8));
    }

    #[test]
    fn flex_profiles_require_confirmed_tx_capability() {
        for (p, meta) in ALL_FLEX_PROFILES {
            assert_eq!(p.capabilities.ptt, meta.has_tx);
            if meta.model_id == "FLEX-GENERIC" {
                assert!(!meta.has_tx);
            } else {
                assert!(meta.has_tx);
            }
        }
    }

    #[test]
    fn backend_kind_flexradio_properties() {
        assert_eq!(
            CatBackendKind::from_str("flexradio"),
            CatBackendKind::FlexRadio
        );
        assert_eq!(CatBackendKind::from_str("flex"), CatBackendKind::FlexRadio);
        assert_eq!(
            CatBackendKind::from_str("smartsdr"),
            CatBackendKind::FlexRadio
        );
        assert_eq!(CatBackendKind::FlexRadio.as_str(), "flexradio");
        assert_eq!(CatBackendKind::FlexRadio.label(), "FlexRadio SmartSDR");
        assert!(CatBackendKind::ALL.contains(&CatBackendKind::FlexRadio));
    }
}
