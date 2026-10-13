// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Rejestr profili transceiverów dla natywnych protokołów CAT.
//!
//! Zapewnia scentralizowany dostęp do zweryfikowanych parametrów modeli
//! radiowych i ich ograniczeń.

use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile, RigVendor};
use crate::cat::rig::RigCapabilities;

// Referencyjne profile Icom CI-V (zweryfikowane z oficjalną dokumentacją Icom):

/// Icom IC-7300 — nowoczesny transceiver HF/50MHz z bezpośrednią przemianą i USB CI-V.
pub static ICOM_IC7300: RigModelProfile = RigModelProfile {
    model_id: "IC-7300",
    name: "Icom IC-7300",
    vendor: RigVendor::Icom,
    default_baud_rate: 115_200,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
    default_ci_v_address: Some(0x94),
    frequency_bytes: 5,
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
        dualwatch: false,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: false,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["read_freq", "read_mode", "read_smeter", "read_ptt"],
};

/// Icom IC-7610 — dwuodbiornikowy flagowy transceiver HF/50MHz.
pub static ICOM_IC7610: RigModelProfile = RigModelProfile {
    model_id: "IC-7610",
    name: "Icom IC-7610",
    vendor: RigVendor::Icom,
    default_baud_rate: 115_200,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
    default_ci_v_address: Some(0x98),
    frequency_bytes: 5,
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
    safe_poll_commands: &["read_freq", "read_mode", "read_smeter", "read_ptt"],
};

/// Icom IC-705 — przenośny transceiver QRP HF/VHF/UHF ze złączem USB i Bluetooth CI-V.
pub static ICOM_IC705: RigModelProfile = RigModelProfile {
    model_id: "IC-705",
    name: "Icom IC-705",
    vendor: RigVendor::Icom,
    default_baud_rate: 115_200,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
    default_ci_v_address: Some(0xA4),
    frequency_bytes: 5,
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
        dualwatch: false,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: false,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["read_freq", "read_mode", "read_smeter", "read_ptt"],
};

/// Icom IC-706MKIIG — klasyczny transceiver mobilny z tradycyjnym kablem CI-V (jack 3.5mm).
pub static ICOM_IC706MKIIG: RigModelProfile = RigModelProfile {
    model_id: "IC-706MKIIG",
    name: "Icom IC-706MKIIG",
    vendor: RigVendor::Icom,
    default_baud_rate: 19200,
    supported_baud_rates: &[1200, 2400, 4800, 9600, 19200],
    default_ci_v_address: Some(0x58),
    frequency_bytes: 5,
    capabilities: RigCapabilities {
        frequency: true,
        mode: true,
        passband: false,
        split: true,
        split_frequency: true,
        ptt: true,
        vfo: true,
        rit: true,
        xit: false,
        power: false,
        s_meter: true,
    },
    extended_capabilities: NativeRigCapabilities {
        dualwatch: false,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: false,
        preamp: false,
        antenna_select: false,
        filter_width: false,
        noise_reduction: false,
    },
    safe_poll_commands: &["read_freq", "read_mode", "read_smeter", "read_ptt"],
};

// Referencyjne profile Kenwood PC Control (zweryfikowane z oficjalną dokumentacją Kenwood):

/// Kenwood TS-590S — transceiver HF/50MHz z odbiornikiem o bezpośredniej przemianie (down-conversion) i USB Virtual COM.
pub static KENWOOD_TS590S: RigModelProfile = RigModelProfile {
    model_id: "TS-590S",
    name: "Kenwood TS-590S",
    vendor: RigVendor::Kenwood,
    default_baud_rate: 115_200,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
    default_ci_v_address: None,
    frequency_bytes: 11,
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
        dualwatch: false,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["read_if", "read_smeter"],
};

/// Kenwood TS-590SG — unowocześniona wersja TS-590S z ulepszonym odbiornikiem, funkcją CW tune i Auto Information AI2.
pub static KENWOOD_TS590SG: RigModelProfile = RigModelProfile {
    model_id: "TS-590SG",
    name: "Kenwood TS-590SG",
    vendor: RigVendor::Kenwood,
    default_baud_rate: 115_200,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
    default_ci_v_address: None,
    frequency_bytes: 11,
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
        dualwatch: false,
        main_sub_tracking: false,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["read_if", "read_smeter"],
};

/// Kenwood TS-890S — zaawansowany transceiver HF/50MHz/70MHz z architekturą hybrid SDR, USB oraz wbudowanym interfejsem KNS LAN (Ethernet).
pub static KENWOOD_TS890S: RigModelProfile = RigModelProfile {
    model_id: "TS-890S",
    name: "Kenwood TS-890S",
    vendor: RigVendor::Kenwood,
    default_baud_rate: 115_200,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
    default_ci_v_address: None,
    frequency_bytes: 11,
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
        dualwatch: false,
        main_sub_tracking: true,
        transceive_broadcast: true,
        attenuator: true,
        preamp: true,
        antenna_select: true,
        filter_width: true,
        noise_reduction: true,
    },
    safe_poll_commands: &["read_if", "read_smeter"],
};

/// Kenwood TS-990S — flagowy dwuodbiornikowy (Main + Sub) transceiver HF/50MHz 200W z podwójnym wyświetlaczem TFT, USB i interfejsem KNS LAN.
pub static KENWOOD_TS990S: RigModelProfile = RigModelProfile {
    model_id: "TS-990S",
    name: "Kenwood TS-990S",
    vendor: RigVendor::Kenwood,
    default_baud_rate: 115_200,
    supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
    default_ci_v_address: None,
    frequency_bytes: 11,
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
    safe_poll_commands: &["read_if", "read_smeter"],
};

/// Wszystkie oficjalnie zarejestrowane profile.
pub static ALL_PROFILES: &[&RigModelProfile] = &[
    &ICOM_IC7300,
    &ICOM_IC7610,
    &ICOM_IC705,
    &ICOM_IC706MKIIG,
    &KENWOOD_TS590S,
    &KENWOOD_TS590SG,
    &KENWOOD_TS890S,
    &KENWOOD_TS990S,
];

/// Wyszukuje profil modelu po nazwie lub identyfikatorze (case-insensitive).
pub fn find_profile_by_model(name: &str) -> Option<&'static RigModelProfile> {
    let query = name.trim().to_ascii_uppercase();
    ALL_PROFILES.iter().copied().find(|p| {
        p.model_id.to_ascii_uppercase() == query
            || p.name.to_ascii_uppercase() == query
            || query.contains(&p.model_id.to_ascii_uppercase())
    })
}

/// Wyszukuje profil modelu Icom po domyślnym adresie CI-V.
pub fn find_icom_profile_by_address(address: u8) -> Option<&'static RigModelProfile> {
    ALL_PROFILES
        .iter()
        .copied()
        .find(|p| p.vendor == RigVendor::Icom && p.default_ci_v_address == Some(address))
}

/// Zwraca listę profili dla danego producenta.
pub fn profiles_by_vendor(vendor: RigVendor) -> Vec<&'static RigModelProfile> {
    ALL_PROFILES
        .iter()
        .copied()
        .filter(|p| p.vendor == vendor)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_profiles_by_name() {
        let p = find_profile_by_model("IC-7300").expect("IC-7300 profile missing");
        assert_eq!(p.default_ci_v_address, Some(0x94));
        assert_eq!(p.default_baud_rate, 115_200);
        assert!(p.supports_baud_rate(19200));
        assert!(p.supports_baud_rate(115_200));
        assert!(!p.supports_baud_rate(1200));

        let p2 = find_profile_by_model("Icom IC-7610").expect("IC-7610 missing");
        assert_eq!(p2.default_ci_v_address, Some(0x98));
        assert!(p2.extended_capabilities.dualwatch);
    }

    #[test]
    fn test_lookup_by_ci_v_address() {
        let p = find_icom_profile_by_address(0x94).expect("0x94 missing");
        assert_eq!(p.model_id, "IC-7300");

        let p2 = find_icom_profile_by_address(0xA4).expect("0xA4 missing");
        assert_eq!(p2.model_id, "IC-705");
    }

    #[test]
    fn test_profiles_by_vendor() {
        let icom_profiles = profiles_by_vendor(RigVendor::Icom);
        assert!(icom_profiles.len() >= 4);

        let kenwood_profiles = profiles_by_vendor(RigVendor::Kenwood);
        assert_eq!(kenwood_profiles.len(), 4);
    }

    #[test]
    fn test_lookup_kenwood_profiles() {
        let ts590 = find_profile_by_model("TS-590S").expect("TS-590S missing");
        assert_eq!(ts590.vendor, RigVendor::Kenwood);
        assert_eq!(ts590.default_baud_rate, 115_200);
        assert!(ts590.extended_capabilities.transceive_broadcast);

        let ts590sg = find_profile_by_model("TS-590SG").expect("TS-590SG missing");
        assert_eq!(ts590sg.vendor, RigVendor::Kenwood);

        let ts890 = find_profile_by_model("Kenwood TS-890S").expect("TS-890S missing");
        assert_eq!(ts890.vendor, RigVendor::Kenwood);
        assert!(ts890.extended_capabilities.main_sub_tracking);

        let ts990 = find_profile_by_model("TS-990S").expect("TS-990S missing");
        assert_eq!(ts990.vendor, RigVendor::Kenwood);
        assert!(ts990.extended_capabilities.dualwatch);
    }
}
