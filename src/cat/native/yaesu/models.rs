// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Profile możliwości transceiverów Yaesu dla natywnego backendu CAT.

use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile, RigVendor};
use crate::cat::rig::RigCapabilities;

pub use crate::cat::native::registry::{YAESU_FT710, YAESU_FT891, YAESU_FT991A, YAESU_FTDX10};

/// Weryfikuje, czy dla danego modelu Yaesu zalecany jest port Enhanced COM (zamiast Standard COM).
///
/// Współczesne transceivery Yaesu ze zintegrowanym kontrolerem USB (Silicon Labs CP2105)
/// udostępniają w systemie operacyjnym dwa wirtualne porty COM:
/// 1. Enhanced COM Port — dedykowany do komunikacji CAT i komend sterujących (`FA;`, `MD;`, `TX;`).
/// 2. Standard COM Port — dedykowany wyłącznie do sprzętowego kluczowania PTT/CW przez linie RTS/DTR.
///
/// Wybranie portu Standard COM do komunikacji CAT skutkuje brakiem odpowiedzi na komendy tekstowe.
pub fn is_enhanced_port_recommended(model_id: &str) -> bool {
    let id = model_id.trim().to_ascii_uppercase();
    id.contains("891")
        || id.contains("991")
        || id.contains("710")
        || id.contains("DX10")
        || id.contains("101")
}

/// Weryfikuje, czy dany model należy do klasycznej rodziny binarnej Yaesu (5-bajtowe pakiety bez średnika).
pub fn is_legacy_binary_model(model_id: &str) -> bool {
    let id = model_id.trim().to_ascii_uppercase();
    id == "FT-817"
        || id == "FT-817ND"
        || id == "FT-818"
        || id == "FT-857"
        || id == "FT-857D"
        || id == "FT-897"
        || id == "FT-897D"
        || id == "FT-100"
}

/// Tworzy niestandardowy profil transceivera Yaesu z zadaną prędkością transmisji.
pub fn create_custom_yaesu_profile(model_id: &str, baud_rate: u32) -> RigModelProfile {
    RigModelProfile {
        model_id: "CUSTOM-YAESU",
        name: "Custom Yaesu Compatible Transceiver",
        vendor: RigVendor::Yaesu,
        default_baud_rate: baud_rate,
        supported_baud_rates: &[4800, 9600, 19200, 38400, 57600, 115_200],
        default_ci_v_address: None,
        frequency_bytes: 9,
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
            power: false,
            s_meter: true,
        },
        extended_capabilities: NativeRigCapabilities {
            dualwatch: model_id.contains("101") || model_id.contains("DX10"),
            main_sub_tracking: model_id.contains("101"),
            transceive_broadcast: true,
            attenuator: true,
            preamp: true,
            antenna_select: true,
            filter_width: true,
            noise_reduction: true,
        },
        safe_poll_commands: &["read_freq", "read_mode", "read_smeter", "read_ptt"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_port_detection() {
        assert!(is_enhanced_port_recommended("FT-891"));
        assert!(is_enhanced_port_recommended("FT-991A"));
        assert!(is_enhanced_port_recommended("FT-710"));
        assert!(is_enhanced_port_recommended("FTDX10"));
        assert!(is_enhanced_port_recommended("FTDX-101D"));
        assert!(!is_enhanced_port_recommended("FT-817"));
        assert!(!is_enhanced_port_recommended("FT-857"));
    }

    #[test]
    fn test_legacy_binary_detection() {
        assert!(is_legacy_binary_model("FT-817"));
        assert!(is_legacy_binary_model("FT-818"));
        assert!(is_legacy_binary_model("FT-857D"));
        assert!(is_legacy_binary_model("FT-897"));
        assert!(!is_legacy_binary_model("FT-891"));
        assert!(!is_legacy_binary_model("FT-710"));
    }

    #[test]
    fn test_custom_yaesu_profile() {
        let profile = create_custom_yaesu_profile("FT-891", 115_200);
        assert_eq!(profile.vendor, RigVendor::Yaesu);
        assert_eq!(profile.default_baud_rate, 115_200);
        assert_eq!(profile.frequency_bytes, 9);
        assert!(profile.supports_baud_rate(115_200));
        assert!(profile.extended_capabilities.transceive_broadcast);
    }
}
