// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Profile możliwości transceiverów Kenwood dla natywnego backendu CAT.

use crate::cat::native::profile::{NativeRigCapabilities, RigModelProfile, RigVendor};
use crate::cat::rig::RigCapabilities;

pub use crate::cat::native::registry::{
    KENWOOD_TS590S, KENWOOD_TS590SG, KENWOOD_TS890S, KENWOOD_TS990S,
};

/// Udokumentowany kontrakt PTT jednego modelu Kenwood (PC Control).
///
/// Kontrakt jest **modelowy**: każdy wpis wskazuje własny oficjalny dokument producenta.
/// Nie istnieje jeden „wspólny” kontrakt dla całej marki — pole [`Self::document`] jest
/// obowiązkowe, a modele bez dokumentu nie są obsługiwane (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KenwoodPttContract {
    /// Identyfikator modelu ([`RigModelProfile::model_id`]).
    pub model_id: &'static str,
    /// Udokumentowana komenda wejścia w nadawanie (bez parametru P1 = `SEND`/`SEND-PTT`).
    pub ptt_on: &'static str,
    /// Udokumentowana komenda powrotu do odbioru.
    pub ptt_off: &'static str,
    /// Udokumentowana komenda **odczytu** stanu TX/RX, jeżeli dokument modelu ją definiuje.
    ///
    /// `IF` jest udokumentowany wyłącznie dla TS-590S/TS-590SG. Modele bez tej komendy
    /// potwierdzają stan wyłącznie przez odpowiedź Auto Information (`TX P1;` / `RX;`).
    pub status_read: Option<&'static str>,
    /// Udokumentowana komenda odczytu S-metra dla tego modelu (formularz parametru jest modelowy).
    pub smeter_read: Option<&'static str>,
    /// Oficjalny dokument producenta (identyfikator, rewizja, data) wraz ze stronami.
    pub document: &'static str,
    /// Status weryfikacji dokumentacyjnej tego profilu.
    pub documentation_status: &'static str,
}

/// Status: kontrakt zweryfikowany na podstawie dokumentu konkretnego modelu.
pub const KENWOOD_DOC_VERIFIED: &str = "OFFICIAL_MODEL_SPECIFIC_VERIFIED";
/// Status: profil zadeklarowany przez użytkownika jako „kompatybilny z Kenwood” — brak dokumentu
/// konkretnego modelu, więc potwierdzenie stanu nie jest deklarowane.
pub const KENWOOD_DOC_USER_DECLARED: &str = "USER_DECLARED_COMPATIBLE_NO_MODEL_DOCUMENT";

/// TS-590S — B5A-0316-20/01 (2019-01-30): `TX` s. 30, `RX` s. 25, `IF` s. 15, `SM` s. 27.
pub const KENWOOD_PTT_TS590S: KenwoodPttContract = KenwoodPttContract {
    model_id: "TS-590S",
    ptt_on: "TX;",
    ptt_off: "RX;",
    status_read: Some("IF;"),
    smeter_read: Some("SM0;"),
    document: "Kenwood TS-590S/SG PC CONTROL COMMAND Reference B5A-0316-20/01 (2019-01-30): TX p.30, RX p.25, IF p.15, SM p.27",
    documentation_status: KENWOOD_DOC_VERIFIED,
};

/// TS-590SG — ten sam dokument co TS-590S (wpisy oznaczone `[TS-590S / TS-590SG common]`).
pub const KENWOOD_PTT_TS590SG: KenwoodPttContract = KenwoodPttContract {
    model_id: "TS-590SG",
    document: "Kenwood TS-590SG PC CONTROL COMMAND Reference B5A-0316-20/01 (2019-01-30): TX p.30, RX p.25, IF p.15, SM p.27 (entries tagged TS-590S/TS-590SG common)",
    ..KENWOOD_PTT_TS590S
};

/// TS-890S — B5A-2944-00/01 (2019-01-30): `TX` s. 67, `RX` s. 59, `SM` s. 72 — **brak `IF`**.
pub const KENWOOD_PTT_TS890S: KenwoodPttContract = KenwoodPttContract {
    model_id: "TS-890S",
    ptt_on: "TX;",
    ptt_off: "RX;",
    status_read: None,
    smeter_read: Some("SM;"),
    document: "Kenwood TS-890S PC CONTROL COMMAND Reference B5A-2944-00/01 (2019-01-30): TX p.67, RX p.59, SM p.62; no IF command in the reference",
    documentation_status: KENWOOD_DOC_VERIFIED,
};

/// TS-990S — B5A-3170-00 (2019-01-30): `TX` s. 61, `RX` s. 51, `SM` s. 74 — **brak `IF`**.
pub const KENWOOD_PTT_TS990S: KenwoodPttContract = KenwoodPttContract {
    model_id: "TS-990S",
    ptt_on: "TX;",
    ptt_off: "RX;",
    status_read: None,
    smeter_read: Some("SM0;"),
    document: "Kenwood TS-990S PC CONTROL COMMAND Reference B5A-3170-00 (2019-01-30): TX p.61, RX p.51, SM p.57; no IF command in the reference",
    documentation_status: KENWOOD_DOC_VERIFIED,
};

/// Profil zadeklarowany przez użytkownika (`create_custom_kenwood_profile`).
///
/// Komendy PTT są rodzinne (`TX;` / `RX;` — identyczne we wszystkich czterech oficjalnych
/// referencjach), ale **nie istnieje dokument konkretnego modelu**, dlatego backend nie wysyła
/// żadnego zapytania statusowego, nie odpytuje S-metra i nie deklaruje potwierdzenia stanu.
pub const KENWOOD_PTT_CUSTOM: KenwoodPttContract = KenwoodPttContract {
    model_id: "CUSTOM-KENWOOD",
    ptt_on: "TX;",
    ptt_off: "RX;",
    status_read: None,
    smeter_read: None,
    document: "User-declared Kenwood-compatible profile; family-level TX;/RX; as documented in B5A-0316-20/01, B5A-2944-00/01 and B5A-3170-00 (no model-specific document)",
    documentation_status: KENWOOD_DOC_USER_DECLARED,
};

/// Wszystkie udokumentowane kontrakty PTT Kenwood.
pub static KENWOOD_PTT_CONTRACTS: &[KenwoodPttContract] = &[
    KENWOOD_PTT_TS590S,
    KENWOOD_PTT_TS590SG,
    KENWOOD_PTT_TS890S,
    KENWOOD_PTT_TS990S,
    KENWOOD_PTT_CUSTOM,
];

impl KenwoodPttContract {
    /// Zwraca udokumentowany kontrakt PTT dla wskazanego modelu.
    ///
    /// Model spoza tabeli nie ma dokumentu → `None` → backend odmawia wysłania komendy PTT
    /// (`RigError::Unsupported`), zamiast zgadywać komendę.
    pub fn for_model(model_id: &str) -> Option<&'static KenwoodPttContract> {
        let query = model_id.trim();
        KENWOOD_PTT_CONTRACTS
            .iter()
            .find(|contract| contract.model_id.eq_ignore_ascii_case(query))
    }

    /// Czy dokument modelu definiuje odczyt statusu `IF;`.
    pub fn documents_status_read(&self) -> bool {
        self.status_read.is_some()
    }
}

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
            spots: false,
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
