// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Profile referencyjnych modeli Icom.

use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile, RigVendor};
use crate::cat::rig::RigCapabilities;

pub use crate::cat::native::registry::{ICOM_IC705, ICOM_IC706MKIIG, ICOM_IC7300, ICOM_IC7610};

/// Tworzy profil Icom z domyślnymi parametrami na podstawie nazwy modelu i adresu.
pub fn create_custom_icom_profile(_model: &str, address: u8, baud_rate: u32) -> RigModelProfile {
    RigModelProfile {
        model_id: "CUSTOM-ICOM",
        name: "Custom Icom Transceiver",
        vendor: RigVendor::Icom,
        default_baud_rate: baud_rate,
        supported_baud_rates: &[1200, 2400, 4800, 9600, 19200, 38400, 57600, 115_200],
        default_ci_v_address: Some(address),
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
            attenuator: false,
            preamp: false,
            antenna_select: false,
            filter_width: false,
            noise_reduction: false,
        },
        safe_poll_commands: &["read_freq", "read_mode", "read_smeter", "read_ptt"],
    }
}
