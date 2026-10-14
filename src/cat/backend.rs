// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Warstwa abstrakcji CAT (Computer Aided Transceiver).
//!
//! Ujednolica dostęp do różnych backendów sterowania radiem (Hamlib/`rigctld`,
//! FLRig, TCI, Icom CI-V, SO2R) za pomocą wspólnego traitu [`CatBackend`].
//! Mapowanie napisu konfiguracyjnego na typ backendu realizuje [`CatBackendKind`],
//! a wybór konkretnej implementacji w locie — [`CatController`].

use std::future::Future;

use crate::cat::flrig::FlrigClient;
use crate::cat::hamlib::{HamlibClient, RigState};
pub use crate::cat::native::elecraft::ElecraftBackend;
pub use crate::cat::native::icom::IcomCiVBackend;
pub use crate::cat::native::kenwood::KenwoodBackend;
pub use crate::cat::native::yaesu::YaesuBackend;
use crate::cat::rig::{RigCapabilities, RigCommand, RigError};

/// Nowoczesny kontrakt backendu CAT oparty o trwałą sesję i typowane błędy (Rust Edition 2024).
pub trait RigBackend: Send {
    /// Typ backendu CAT.
    fn kind(&self) -> CatBackendKind;

    /// Możliwości techniczne transceivera i backendu.
    fn capabilities(&self) -> RigCapabilities;

    /// Nawiązuje trwałe połączenie z radiem/demonem.
    fn connect(&mut self) -> impl Future<Output = Result<(), RigError>> + Send;

    /// Sekwencyjny odczyt pełnego stanu radia przez trwałą sesję.
    fn poll(&mut self) -> impl Future<Output = Result<RigState, RigError>> + Send;

    /// Wykonuje polecenie domenowe przez trwałą sesję.
    fn execute(
        &mut self,
        command: &RigCommand,
    ) -> impl Future<Output = Result<(), RigError>> + Send;

    /// Bezpiecznie zamyka połączenie z radiem/demonem.
    fn disconnect(&mut self) -> impl Future<Output = ()> + Send;
}

/// Typy backendów CAT obsługiwane przez program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CatBackendKind {
    /// Demon Hamlib (`rigctld`) — tekstowy protokół TCP.
    #[default]
    Hamlib,
    /// FLRig — XML-RPC po TCP.
    Flrig,
    /// Transceiver Control Interface (radia SDR: SunSDR, ExpertSDR, Thetis).
    Tci,
    /// Bezpośredni protokół Icom CI-V (port szeregowy).
    IcomCiV,
    /// Bezpośredni tekstowy protokół Kenwood CAT.
    Kenwood,
    /// Bezpośredni protokół Yaesu CAT.
    Yaesu,
    /// Bezpośredni natywny protokół Elecraft CAT (K3, K3S, KX2, KX3, K4, K4D).
    Elecraft,
    /// Logika SO2R (dwa radia, blokada pojedynczego TX).
    So2r,
}

impl std::fmt::Display for CatBackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CatBackendKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_str(s))
    }
}

impl CatBackendKind {
    /// Mapuje napis konfiguracyjny (`AppConfig.cat_backend`) na typ backendu.
    /// Nieznane wartości traktowane są jako Hamlib (bezpieczny domyślny backend).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "flrig" | "fldigi" => Self::Flrig,
            "tci" | "sunsdr" | "thetis" => Self::Tci,
            "icom_ci_v" | "icom-civ" | "ci-v" | "civ" | "icom" => Self::IcomCiV,
            "kenwood" | "ts" => Self::Kenwood,
            "yaesu" | "ft" | "ftdx" => Self::Yaesu,
            str_val if str_val.starts_with("ft-") || str_val.starts_with("ftdx") => Self::Yaesu,
            "elecraft" | "k3" | "k3s" | "kx2" | "kx3" | "k4" | "k4d" | "k4hd" => Self::Elecraft,
            "so2r" => Self::So2r,
            _ => Self::Hamlib,
        }
    }

    /// Kanoniczny napis zapisywany w konfiguracji.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hamlib => "hamlib",
            Self::Flrig => "flrig",
            Self::Tci => "tci",
            Self::IcomCiV => "icom_ci_v",
            Self::Kenwood => "kenwood",
            Self::Yaesu => "yaesu",
            Self::Elecraft => "elecraft",
            Self::So2r => "so2r",
        }
    }

    /// Czytelna etykieta dla interfejsu użytkownika i logów.
    pub fn label(self) -> &'static str {
        match self {
            Self::Hamlib => "Hamlib (rigctld)",
            Self::Flrig => "FLRig (XML-RPC)",
            Self::Tci => "TCI (SDR)",
            Self::IcomCiV => "Icom CI-V",
            Self::Kenwood => "Kenwood CAT",
            Self::Yaesu => "Yaesu CAT",
            Self::Elecraft => "Elecraft CAT",
            Self::So2r => "SO2R",
        }
    }

    /// Wszystkie obsługiwane typy (do list rozwijanych).
    pub const ALL: [Self; 8] = [
        Self::Hamlib,
        Self::Flrig,
        Self::Tci,
        Self::IcomCiV,
        Self::Kenwood,
        Self::Yaesu,
        Self::Elecraft,
        Self::So2r,
    ];
}

fn not_implemented(kind: CatBackendKind, op: &str) -> String {
    format!(
        "{}: operacja „{}” nie jest jeszcze zaimplementowana",
        kind.label(),
        op
    )
}

/// Wspólny interfejs wszystkich backendów CAT.
///
/// Implementacje domyślne zwracają czytelny błąd „nie zaimplementowano”,
/// dzięki czemu nowy backend można dodawać przyrostowo (tylko te operacje,
/// które dany protokół faktycznie wspiera).
///
/// `async fn` w traicie jest celowe — trait jest używany wyłącznie wewnętrznie
/// (statyczny wybór implementacji przez [`CatController`], bez `dyn CatBackend`).
#[allow(async_fn_in_trait)]
pub trait CatBackend: Send + Sync {
    /// Typ backendu (diagnostyka i logowanie).
    fn kind(&self) -> CatBackendKind;

    /// Czytelna nazwa backendu.
    fn name(&self) -> &'static str {
        self.kind().label()
    }

    /// Nawiązuje / weryfikuje połączenie z radiem.
    async fn connect(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Odczytuje pełny stan radia (częstotliwość, emisja, S-meter, PTT itd.).
    async fn poll_state(&mut self) -> Result<RigState, String> {
        Err(not_implemented(self.kind(), "poll_state"))
    }

    /// Przestawia częstotliwość VFO (w Hz).
    async fn set_frequency(&self, _freq_hz: u64) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_frequency"))
    }

    /// Przestawia emisję oraz szerokość filtru (Hz).
    async fn set_mode(&self, _mode: &str, _passband_hz: u32) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_mode"))
    }

    /// Włącza/wyłącza tryb Split.
    async fn set_split(&self, _enabled: bool, _tx_vfo: &str) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_split"))
    }

    /// Załącza/wyłącza PTT (nadawanie).
    async fn set_ptt(&self, _ptt: bool) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_ptt"))
    }

    /// Wybiera aktywny VFO (np. "VFOA", "VFOB", "Main", "Sub").
    async fn set_vfo(&self, _vfo: &str) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_vfo"))
    }

    /// Ustawia przesunięcie RIT (Hz, może być ujemne).
    async fn set_rit(&self, _rit_hz: i32) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_rit"))
    }

    /// Ustawia przesunięcie XIT (Hz, może być ujemne).
    async fn set_xit(&self, _xit_hz: i32) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_xit"))
    }

    /// Ustawia moc wyjściową (W).
    async fn set_power(&self, _watts: f32) -> Result<(), String> {
        Err(not_implemented(self.kind(), "set_power"))
    }
}

fn unsupported_backend(kind: CatBackendKind) -> String {
    format!("backend {} nie jest jeszcze zaimplementowany", kind.label())
}

/// Dyspozytor backendu CAT — wybiera konkretną implementację na podstawie
/// [`CatBackendKind`] i deleguje do niej wywołania traitu [`CatBackend`].
pub enum CatController {
    Hamlib(HamlibClient),
    Flrig(FlrigClient),
    Unsupported(CatBackendKind),
}

impl CatController {
    /// Tworzy kontroler dla wskazanego backendu i adresu hosta/portu.
    pub fn new(kind: CatBackendKind, host: &str, port: u16) -> Self {
        match kind {
            CatBackendKind::Hamlib => {
                let (client, _rx) = HamlibClient::new(host, port);
                Self::Hamlib(client)
            }
            CatBackendKind::Flrig => Self::Flrig(FlrigClient::new(host, port)),
            other => Self::Unsupported(other),
        }
    }
}

impl CatBackend for CatController {
    fn kind(&self) -> CatBackendKind {
        match self {
            Self::Hamlib(_) => CatBackendKind::Hamlib,
            Self::Flrig(_) => CatBackendKind::Flrig,
            Self::Unsupported(k) => *k,
        }
    }

    async fn connect(&mut self) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.connect().await,
            Self::Flrig(c) => c.connect().await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn poll_state(&mut self) -> Result<RigState, String> {
        match self {
            Self::Hamlib(c) => c.poll_state().await,
            Self::Flrig(c) => c.poll_state().await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_frequency(&self, freq_hz: u64) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_frequency(freq_hz).await,
            Self::Flrig(c) => c.set_frequency(freq_hz).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_mode(&self, mode: &str, passband_hz: u32) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_mode(mode, passband_hz).await,
            Self::Flrig(c) => CatBackend::set_mode(c, mode, passband_hz).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_split(&self, enabled: bool, tx_vfo: &str) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_split(enabled, tx_vfo).await,
            Self::Flrig(c) => c.set_split(enabled, tx_vfo).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_ptt(&self, ptt: bool) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_ptt(ptt).await,
            Self::Flrig(c) => c.set_ptt(ptt).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_vfo(&self, vfo: &str) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_vfo(vfo).await,
            Self::Flrig(c) => CatBackend::set_vfo(c, vfo).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_rit(&self, rit_hz: i32) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_rit(rit_hz).await,
            Self::Flrig(c) => c.set_rit(rit_hz).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_xit(&self, xit_hz: i32) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_xit(xit_hz).await,
            Self::Flrig(c) => c.set_xit(xit_hz).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }

    async fn set_power(&self, watts: f32) -> Result<(), String> {
        match self {
            Self::Hamlib(c) => c.set_power(watts).await,
            Self::Flrig(c) => c.set_power(watts).await,
            Self::Unsupported(k) => Err(unsupported_backend(*k)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_kind_parses_known_strings() {
        assert_eq!(CatBackendKind::from_str("hamlib"), CatBackendKind::Hamlib);
        assert_eq!(CatBackendKind::from_str("flrig"), CatBackendKind::Flrig);
        assert_eq!(CatBackendKind::from_str("TCI"), CatBackendKind::Tci);
        assert_eq!(CatBackendKind::from_str("ci-v"), CatBackendKind::IcomCiV);
        assert_eq!(CatBackendKind::from_str("kenwood"), CatBackendKind::Kenwood);
        assert_eq!(CatBackendKind::from_str("yaesu"), CatBackendKind::Yaesu);
        assert_eq!(CatBackendKind::from_str("FT-891"), CatBackendKind::Yaesu);
        assert_eq!(CatBackendKind::from_str("so2r"), CatBackendKind::So2r);
    }

    #[test]
    fn backend_kind_falls_back_to_hamlib() {
        assert_eq!(CatBackendKind::from_str("nieznany"), CatBackendKind::Hamlib);
        assert_eq!(CatBackendKind::from_str(""), CatBackendKind::Hamlib);
    }

    #[test]
    fn backend_kind_roundtrips_via_as_str() {
        for kind in CatBackendKind::ALL {
            assert_eq!(CatBackendKind::from_str(kind.as_str()), kind);
        }
    }

    #[test]
    fn controller_kind_matches_requested_backend() {
        let ctl = CatController::new(CatBackendKind::Flrig, "127.0.0.1", 12345);
        assert_eq!(ctl.kind(), CatBackendKind::Flrig);

        let ctl = CatController::new(CatBackendKind::Tci, "127.0.0.1", 40001);
        assert_eq!(ctl.kind(), CatBackendKind::Tci);
    }

    #[test]
    fn backend_kind_parses_yaesu() {
        assert_eq!(CatBackendKind::from_str("yaesu"), CatBackendKind::Yaesu);
        assert_eq!(CatBackendKind::from_str("YAESU"), CatBackendKind::Yaesu);
        assert_eq!(CatBackendKind::from_str("ft-891"), CatBackendKind::Yaesu);
        assert_eq!(CatBackendKind::from_str("FT-991A"), CatBackendKind::Yaesu);
        assert_eq!(CatBackendKind::from_str("ft-710"), CatBackendKind::Yaesu);
        assert_eq!(CatBackendKind::from_str("ftdx10"), CatBackendKind::Yaesu);
    }

    #[test]
    fn backend_kind_yaesu_roundtrip() {
        assert_eq!(CatBackendKind::Yaesu.as_str(), "yaesu");
        assert_eq!(
            CatBackendKind::from_str(CatBackendKind::Yaesu.as_str()),
            CatBackendKind::Yaesu
        );
        assert_eq!(format!("{}", CatBackendKind::Yaesu), "yaesu");
        assert_eq!(CatBackendKind::default(), CatBackendKind::Hamlib);
    }

    #[test]
    fn backend_kind_yaesu_capabilities_match_profile() {
        use crate::cat::native::registry::YAESU_FT891;
        assert_eq!(
            YAESU_FT891.vendor,
            crate::cat::native::profile::RigVendor::Yaesu
        );
        assert!(YAESU_FT891.capabilities.frequency);
        assert!(YAESU_FT891.capabilities.mode);
        assert!(YAESU_FT891.capabilities.ptt);
        assert!(!YAESU_FT891.capabilities.power);
    }

    #[test]
    fn backend_kind_parses_elecraft() {
        assert_eq!(
            CatBackendKind::from_str("elecraft"),
            CatBackendKind::Elecraft
        );
        assert_eq!(
            CatBackendKind::from_str("ELECRAFT"),
            CatBackendKind::Elecraft
        );
        assert_eq!(CatBackendKind::from_str("k3"), CatBackendKind::Elecraft);
        assert_eq!(CatBackendKind::from_str("K3S"), CatBackendKind::Elecraft);
        assert_eq!(CatBackendKind::from_str("kx2"), CatBackendKind::Elecraft);
        assert_eq!(CatBackendKind::from_str("kx3"), CatBackendKind::Elecraft);
        assert_eq!(CatBackendKind::from_str("k4"), CatBackendKind::Elecraft);
        assert_eq!(CatBackendKind::from_str("k4d"), CatBackendKind::Elecraft);
        assert_eq!(CatBackendKind::from_str("k4hd"), CatBackendKind::Elecraft);
    }

    #[test]
    fn backend_kind_elecraft_roundtrip() {
        assert_eq!(CatBackendKind::Elecraft.as_str(), "elecraft");
        assert_eq!(
            CatBackendKind::from_str(CatBackendKind::Elecraft.as_str()),
            CatBackendKind::Elecraft
        );
        assert_eq!(format!("{}", CatBackendKind::Elecraft), "elecraft");
    }

    #[test]
    fn backend_kind_elecraft_capabilities_match_profile() {
        use crate::cat::native::registry::ELECRAFT_K3;
        assert_eq!(
            ELECRAFT_K3.vendor,
            crate::cat::native::profile::RigVendor::Elecraft
        );
        assert!(ELECRAFT_K3.capabilities.frequency);
        assert!(ELECRAFT_K3.capabilities.mode);
        assert!(ELECRAFT_K3.capabilities.ptt);
        assert!(ELECRAFT_K3.capabilities.power);
    }

    #[test]
    fn backend_kind_unknown_fallback_unchanged() {
        assert_eq!(
            CatBackendKind::from_str("unknown_xyz"),
            CatBackendKind::Hamlib
        );
        assert_eq!(CatBackendKind::from_str(""), CatBackendKind::Hamlib);
    }
}
