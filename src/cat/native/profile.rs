// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Profile możliwości transceiverów dla natywnych backendów CAT.
//!
//! Każdy profil definiuje precyzyjne parametry techniczne danego modelu,
//! w tym domyślne i akceptowane prędkości transmisji, adresy protokołu,
//! formaty kodowania częstotliwości oraz macierze możliwości (podstawowych i rozszerzonych).

use crate::cat::rig::RigCapabilities;

/// Producent transceivera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RigVendor {
    Icom,
    Kenwood,
    Yaesu,
    Elecraft,
    FlexRadio,
    Xiegu,
    Generic,
}

impl RigVendor {
    pub fn name(self) -> &'static str {
        match self {
            Self::Icom => "Icom",
            Self::Kenwood => "Kenwood",
            Self::Yaesu => "Yaesu",
            Self::Elecraft => "Elecraft",
            Self::FlexRadio => "FlexRadio",
            Self::Xiegu => "Xiegu",
            Self::Generic => "Generic",
        }
    }
}

/// Rozszerzone możliwości sprzętowe transceivera (poza uogólnionym RigCapabilities).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NativeRigCapabilities {
    /// Odbiór równoczesny na dwóch częstotliwościach (Dualwatch / 2nd RX).
    pub dualwatch: bool,
    /// Śledzenie VFO Main i Sub.
    pub main_sub_tracking: bool,
    /// Samorzutne rozsyłanie zmian stanu (Icom Transceive / Kenwood AI).
    pub transceive_broadcast: bool,
    /// Sterowanie tłumikiem wejściowym (Attenuator).
    pub attenuator: bool,
    /// Sterowanie przedwzmacniaczem w.cz. (Preamp).
    pub preamp: bool,
    /// Wybór złącza antenowego (ANT1/ANT2/RX).
    pub antenna_select: bool,
    /// Płynna lub skokowa zmiana szerokości filtru DSP.
    pub filter_width: bool,
    /// Sterowanie redukcją szumów DSP (NR / Noise Reduction).
    pub noise_reduction: bool,
}

/// Kompletny profil techniczny transceivera.
#[derive(Debug, Clone, PartialEq)]
pub struct RigModelProfile {
    pub model_id: &'static str,
    pub name: &'static str,
    pub vendor: RigVendor,
    pub default_baud_rate: u32,
    pub supported_baud_rates: &'static [u32],
    pub default_ci_v_address: Option<u8>,
    pub frequency_bytes: usize,
    pub capabilities: RigCapabilities,
    pub extended_capabilities: NativeRigCapabilities,
    pub safe_poll_commands: &'static [&'static str],
}

impl RigModelProfile {
    /// Liczba cyfr częstotliwości w protokole tekstowym (lub liczba bajtów w protokole binarnym).
    pub fn frequency_digits(&self) -> usize {
        self.frequency_bytes
    }

    /// Weryfikuje, czy dany baud rate jest akceptowany przez ten model.
    pub fn supports_baud_rate(&self, baud: u32) -> bool {
        self.supported_baud_rates.contains(&baud)
    }
}
