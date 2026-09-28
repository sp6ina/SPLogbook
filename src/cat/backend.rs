// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Warstwa abstrakcji CAT (Computer Aided Transceiver).
//!
//! Ujednolica dostęp do różnych backendów sterowania radiem (Hamlib/`rigctld`,
//! FLRig, TCI, Icom CI-V, SO2R) za pomocą wspólnego traitu [`CatBackend`].
//! Mapowanie napisu konfiguracyjnego na typ backendu realizuje [`CatBackendKind`],
//! a wybór konkretnej implementacji w locie — [`CatController`].

use crate::cat::flrig::FlrigClient;
use crate::cat::hamlib::{HamlibClient, RigState};

/// Typy backendów CAT obsługiwane przez program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatBackendKind {
    /// Demon Hamlib (`rigctld`) — tekstowy protokół TCP.
    Hamlib,
    /// FLRig — XML-RPC po TCP.
    Flrig,
    /// Transceiver Control Interface (radia SDR: SunSDR, ExpertSDR, Thetis).
    Tci,
    /// Bezpośredni protokół Icom CI-V (port szeregowy).
    IcomCiV,
    /// Bezpośredni tekstowy protokół Kenwood / Elecraft / Yaesu CAT.
    Kenwood,
    /// Logika SO2R (dwa radia, blokada pojedynczego TX).
    So2r,
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
            "kenwood" | "elecraft" | "yaesu" | "ts" | "k3" => Self::Kenwood,
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
            Self::Kenwood => "Kenwood / Elecraft / Yaesu (CAT)",
            Self::So2r => "SO2R",
        }
    }

    /// Wszystkie obsługiwane typy (do list rozwijanych).
    pub const ALL: [Self; 6] = [Self::Hamlib, Self::Flrig, Self::Tci, Self::IcomCiV, Self::Kenwood, Self::So2r];
}

fn not_implemented(kind: CatBackendKind, op: &str) -> String {
    format!("{}: operacja „{}” nie jest jeszcze zaimplementowana", kind.label(), op)
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
        assert_eq!(CatBackendKind::from_str("yaesu"), CatBackendKind::Kenwood);
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
}
