// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Profile możliwości transceiverów Kenwood dla natywnego backendu CAT.

use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile, RigVendor};
use crate::cat::rig::RigCapabilities;

pub use crate::cat::native::registry::{
    KENWOOD_TS590S, KENWOOD_TS590SG, KENWOOD_TS890S, KENWOOD_TS990S,
};

/// Weryfikuje, czy dany model Kenwood posiada natywny sprzętowy interfejs LAN (Ethernet/TCP).
///
/// Oficjalna specyfikacja producenta potwierdza:
/// - TS-590S / TS-590SG: brak gniazda Ethernet (wyłącznie porty USB Virtual COM i RS-232C).
/// - TS-890S / TS-990S: wbudowany port Ethernet 100BASE-TX (protokół KNS, TCP port 60000).
pub fn is_lan_supported(model_id: &str) -> bool {
    let id = model_id.trim().to_ascii_uppercase();
    id == "TS-890S" || id == "TS-990S"
}

/// Tworzy niestandardowy profil transceivera Kenwood z zadaną prędkością transmisji.
pub fn create_custom_kenwood_profile(model_id: &str, baud_rate: u32) -> RigModelProfile {
    RigModelProfile {
        model_id: "CUSTOM-KENWOOD",
        name: "Custom Kenwood Compatible Transceiver",
        vendor: RigVendor::Kenwood,
        default_baud_rate: baud_rate,
        supported_baud_rates: &[1200, 2400, 4800, 9600, 19200, 38400, 57600, 115_200],
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
            dualwatch: is_lan_supported(model_id),
            main_sub_tracking: is_lan_supported(model_id),
            transceive_broadcast: true,
            attenuator: true,
            preamp: true,
            antenna_select: true,
            filter_width: true,
            noise_reduction: true,
        },
        safe_poll_commands: &["read_if", "read_smeter"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kenwood_lan_support_flags() {
        assert!(!is_lan_supported("TS-590S"));
        assert!(!is_lan_supported("TS-590SG"));
        assert!(is_lan_supported("TS-890S"));
        assert!(is_lan_supported("TS-990S"));
        assert!(!is_lan_supported("TS-2000"));
    }

    #[test]
    fn test_custom_kenwood_profile() {
        let profile = create_custom_kenwood_profile("TS-890S", 115_200);
        assert_eq!(profile.vendor, RigVendor::Kenwood);
        assert_eq!(profile.default_baud_rate, 115_200);
        assert!(profile.supports_baud_rate(115_200));
        assert!(profile.extended_capabilities.transceive_broadcast);
    }
}
