// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Kanoniczny model konfiguracji stacji wieloradiowej CAT, walidacja i migracja (NATIVE-6B).
//!
//! Zapewnia:
//! - Stabilne identyfikatory [`RigId`],
//! - Jawny numer schematu [`CURRENT_SCHEMA_VERSION`],
//! - Współdzielone, typowane reguły walidacji przed zapisem i po odczycie,
//! - Idempotentną migrację ze starszych wersji `AppConfig` bez ryzyka utraty danych,
//! - Bezpieczne wartości domyślne i brak niejawnego załączania PTT.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

use crate::cat::routing::{ReceiverId, RigId, RigIdError};
use crate::core::station::AppConfig;

/// Bieżący numer schematu konfiguracji stacji wieloradiowej CAT.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Typowane błędy walidacji konfiguracji CAT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatConfigValidationError {
    /// Identyfikator radia jest pusty lub niepoprawny syntaktycznie.
    EmptyRigId,
    /// Błąd walidacji identyfikatora [`RigId`].
    InvalidRigId(String),
    /// Wykryto zduplikowany identyfikator sesji radiowej.
    DuplicateRigId(RigId),
    /// Pusty lub niepoprawny adres hosta sieciowego.
    InvalidHost(String),
    /// Port sieciowy poza dozwolonym zakresem (1..=65535).
    InvalidPort(u16),
    /// Nieobsługiwana lub niepoprawna prędkość transmisji szeregowej (baud rate).
    InvalidBaudRate(u32),
    /// Brak wymaganej nazwy portu szeregowego.
    MissingSerialPort,
    /// Nieprawidłowy adres szyny Icom CI-V (niedozwolone 0x00 lub 0xFF).
    InvalidCivAddress(u8),
    /// Brak wymaganego modelu radiotelefonu.
    MissingModel,
    /// Nieobsługiwany typ transportu.
    UnsupportedTransport(String),
    /// Nieprawidłowy limit czasu polecenia (musi mieścić się w 50..=30000 ms).
    InvalidTimeout(u64),
    /// Nieprawidłowy interwał odpytywania stanu (musi mieścić się w 20..=10000 ms).
    InvalidPollInterval(u64),
    /// Cel Active RX wskazuje na niezarejestrowane radio.
    ActiveRxTargetNotFound(RigId),
    /// Cel Active TX wskazuje na niezarejestrowane radio.
    ActiveTxTargetNotFound(RigId),
    /// Nieobsługiwana wersja schematu konfiguracji CAT.
    UnsupportedSchemaVersion(u32),
}

impl fmt::Display for CatConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRigId => write!(f, "identyfikator radia nie może być pusty"),
            Self::InvalidRigId(err) => write!(f, "nieprawidłowy RigId: {err}"),
            Self::DuplicateRigId(id) => write!(f, "zduplikowany identyfikator radia: '{id}'"),
            Self::InvalidHost(host) => write!(f, "nieprawidłowy adres hosta: '{host}'"),
            Self::InvalidPort(port) => write!(f, "nieprawidłowy port: {port} (musi być 1..=65535)"),
            Self::InvalidBaudRate(baud) => {
                write!(f, "nieprawidłowa prędkość transmisji: {baud} Bd")
            }
            Self::MissingSerialPort => write!(f, "wymagany port szeregowy nie został podany"),
            Self::InvalidCivAddress(addr) => {
                write!(
                    f,
                    "nieprawidłowy adres CI-V: 0x{addr:02X} (musi być 0x01..=0xFE)"
                )
            }
            Self::MissingModel => write!(f, "model radiotelefonu nie został podany"),
            Self::UnsupportedTransport(t) => write!(f, "nieobsługiwany transport: '{t}'"),
            Self::InvalidTimeout(ms) => {
                write!(
                    f,
                    "nieprawidłowy timeout: {ms} ms (dozwolone 50..=30000 ms)"
                )
            }
            Self::InvalidPollInterval(ms) => write!(
                f,
                "nieprawidłowy interwał odpytywania: {ms} ms (dozwolone 20..=10000 ms)"
            ),
            Self::ActiveRxTargetNotFound(id) => {
                write!(f, "cel Active RX wskazuje nieistniejące radio: '{id}'")
            }
            Self::ActiveTxTargetNotFound(id) => {
                write!(f, "cel Active TX wskazuje nieistniejące radio: '{id}'")
            }
            Self::UnsupportedSchemaVersion(v) => {
                write!(f, "nieobsługiwana wersja schematu konfiguracji CAT: {v}")
            }
        }
    }
}

impl std::error::Error for CatConfigValidationError {}

impl From<RigIdError> for CatConfigValidationError {
    fn from(err: RigIdError) -> Self {
        Self::InvalidRigId(err.to_string())
    }
}

/// Błędy procesu migracji konfiguracji CAT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatConfigMigrationError {
    /// Błąd walidacji w zmigrowanej strukturze.
    Validation(CatConfigValidationError),
    /// Nieznany lub nieobsługiwany backend ze starej konfiguracji.
    UnknownBackend(String),
    /// Format konfiguracji jest uszkodzony lub ma nieobsługiwaną wersję.
    UnsupportedSchemaVersion(u32),
}

impl fmt::Display for CatConfigMigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "błąd walidacji po migracji: {e}"),
            Self::UnknownBackend(b) => write!(f, "nieznany backend CAT: '{b}'"),
            Self::UnsupportedSchemaVersion(v) => {
                write!(f, "nieobsługiwana wersja schematu konfiguracji CAT: {v}")
            }
        }
    }
}

impl std::error::Error for CatConfigMigrationError {}

impl From<CatConfigValidationError> for CatConfigMigrationError {
    fn from(e: CatConfigValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<RigIdError> for CatConfigMigrationError {
    fn from(e: RigIdError) -> Self {
        Self::Validation(e.into())
    }
}

/// Kanoniczny model konfiguracji stacji wieloradiowej CAT ([`CatStationConfig`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatStationConfig {
    /// Wersja schematu konfiguracji.
    pub schema_version: u32,
    /// Lista skonfigurowanych radiotelefonów i sesji.
    pub rigs: Vec<RigConfig>,
    /// Skonfigurowany cel Active RX (o ile ustawiony).
    #[serde(default)]
    pub active_rx: Option<RigTargetConfig>,
    /// Skonfigurowany cel Active TX (o ile ustawiony).
    #[serde(default)]
    pub active_tx: Option<RigTargetConfig>,
}

impl Default for CatStationConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            rigs: Vec::new(),
            active_rx: None,
            active_tx: None,
        }
    }
}

impl CatStationConfig {
    /// Weryfikuje poprawność całej konfiguracji stacji CAT.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(CatConfigValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }

        let mut ids = BTreeSet::new();
        for rig in &self.rigs {
            rig.validate()?;
            if !ids.insert(rig.id.clone()) {
                return Err(CatConfigValidationError::DuplicateRigId(rig.id.clone()));
            }
        }

        if let Some(ref rx) = self.active_rx {
            if !ids.contains(&rx.rig_id) {
                return Err(CatConfigValidationError::ActiveRxTargetNotFound(
                    rx.rig_id.clone(),
                ));
            }
        }

        if let Some(ref tx) = self.active_tx {
            if !ids.contains(&tx.rig_id) {
                return Err(CatConfigValidationError::ActiveTxTargetNotFound(
                    tx.rig_id.clone(),
                ));
            }
        }

        Ok(())
    }
}

/// Zapisany cel radiowy dla Active RX lub Active TX.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RigTargetConfig {
    /// Identyfikator sesji radiowej.
    pub rig_id: RigId,
    /// Opcjonalny cel odbiornika (VFO lub Slice).
    #[serde(default)]
    pub receiver_id: Option<ReceiverId>,
}

impl RigTargetConfig {
    /// Tworzy nowy cel wskazujący całe radio.
    pub fn rig_only(rig_id: RigId) -> Self {
        Self {
            rig_id,
            receiver_id: None,
        }
    }

    /// Tworzy nowy cel ze wskazanym odbiornikiem.
    pub fn with_receiver(rig_id: RigId, receiver_id: ReceiverId) -> Self {
        Self {
            rig_id,
            receiver_id: Some(receiver_id),
        }
    }
}

/// Konfiguracja pojedynczego radiotelefonu / sesji radiowej.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RigConfig {
    /// Stabilny, unikalny identyfikator sesji.
    pub id: RigId,
    /// Czy sesja jest włączona i powinna być automatycznie uruchamiana.
    pub enabled: bool,
    /// Przyjazna dla użytkownika nazwa wyświetlana w GUI.
    pub display_name: String,
    /// Konfiguracja wybranego backendu.
    pub backend: RigBackendConfig,
    /// Interwał odpytywania telemetrii (w milisekundach).
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// Dopuszczalny czas oczekiwania na wykonanie polecenia (w milisekundach).
    #[serde(default = "default_command_timeout_ms")]
    pub command_timeout_ms: u64,
}

fn default_poll_interval_ms() -> u64 {
    200
}

fn default_command_timeout_ms() -> u64 {
    1500
}

impl RigConfig {
    /// Weryfikuje konfigurację pojedynczego radia.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if self.id.as_str().trim().is_empty() {
            return Err(CatConfigValidationError::EmptyRigId);
        }
        // Weryfikacja syntaktyczna RigId
        let _ = RigId::new(self.id.as_str())?;

        if !(20..=10000).contains(&self.poll_interval_ms) {
            return Err(CatConfigValidationError::InvalidPollInterval(
                self.poll_interval_ms,
            ));
        }

        if !(50..=30000).contains(&self.command_timeout_ms) {
            return Err(CatConfigValidationError::InvalidTimeout(
                self.command_timeout_ms,
            ));
        }

        self.backend.validate()
    }
}

/// Konfiguracja konkretnego rodzaju backendu CAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RigBackendConfig {
    /// Hamlib (`rigctld`).
    Hamlib(HamlibConfig),
    /// FLRig (XML-RPC).
    Flrig(FlrigConfig),
    /// TCI (WebSocket SDR).
    Tci(TciConfig),
    /// Bezpośredni Icom CI-V.
    Icom(IcomConfig),
    /// Bezpośredni Kenwood CAT.
    Kenwood(KenwoodConfig),
    /// Bezpośredni Yaesu CAT.
    Yaesu(YaesuConfig),
    /// Bezpośredni Elecraft CAT.
    Elecraft(ElecraftConfig),
    /// Bezpośredni FlexRadio SmartSDR.
    FlexRadio(FlexRadioConfig),
}

impl RigBackendConfig {
    /// Weryfikuje parametry backendu.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        match self {
            Self::Hamlib(cfg) => cfg.validate(),
            Self::Flrig(cfg) => cfg.validate(),
            Self::Tci(cfg) => cfg.validate(),
            Self::Icom(cfg) => cfg.validate(),
            Self::Kenwood(cfg) => cfg.validate(),
            Self::Yaesu(cfg) => cfg.validate(),
            Self::Elecraft(cfg) => cfg.validate(),
            Self::FlexRadio(cfg) => cfg.validate(),
        }
    }

    /// Zwraca czytelną nazwę rodzaju backendu.
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Hamlib(_) => "Hamlib",
            Self::Flrig(_) => "FLRig",
            Self::Tci(_) => "TCI",
            Self::Icom(_) => "Icom",
            Self::Kenwood(_) => "Kenwood",
            Self::Yaesu(_) => "Yaesu",
            Self::Elecraft(_) => "Elecraft",
            Self::FlexRadio(_) => "FlexRadio",
        }
    }
}

pub type HamlibBackendConfig = HamlibConfig;
pub type FlrigBackendConfig = FlrigConfig;
pub type TciBackendConfig = TciConfig;
pub type IcomBackendConfig = IcomConfig;
pub type KenwoodBackendConfig = KenwoodConfig;
pub type YaesuBackendConfig = YaesuConfig;
pub type ElecraftBackendConfig = ElecraftConfig;
pub type FlexRadioBackendConfig = FlexRadioConfig;

/// Konfiguracja backendu Hamlib (`rigctld`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HamlibConfig {
    /// Adres hosta demona `rigctld`.
    pub host: String,
    /// Port TCP demona `rigctld`.
    pub port: u16,
    /// Czy SPLogbook ma automatycznie uruchamiać lokalny proces `rigctld`.
    #[serde(default)]
    pub auto_start_rigctld: bool,
    /// Model radiotelefonu w nomenklaturze Hamlib.
    #[serde(default)]
    pub rig_model: String,
    /// Port szeregowy dla lokalnego procesu `rigctld`.
    #[serde(default)]
    pub serial_port: String,
    /// Prędkość portu szeregowego.
    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,
    /// Źródło binarki Hamlib ("bundled" lub "custom").
    #[serde(default = "default_hamlib_source")]
    pub hamlib_source: String,
    /// Własna ścieżka do pliku wykonywalnego `rigctld`.
    #[serde(default)]
    pub custom_rigctld_path: String,
}

fn default_baud_rate() -> u32 {
    19200
}

fn default_hamlib_source() -> String {
    "bundled".to_string()
}

impl HamlibConfig {
    /// Walidacja konfiguracji Hamlib.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if self.host.trim().is_empty() {
            return Err(CatConfigValidationError::InvalidHost(self.host.clone()));
        }
        if self.port == 0 {
            return Err(CatConfigValidationError::InvalidPort(self.port));
        }
        if self.auto_start_rigctld {
            if self.serial_port.trim().is_empty() {
                return Err(CatConfigValidationError::MissingSerialPort);
            }
            if self.baud_rate == 0 {
                return Err(CatConfigValidationError::InvalidBaudRate(self.baud_rate));
            }
        }
        Ok(())
    }
}

impl Default for HamlibConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4532,
            auto_start_rigctld: false,
            rig_model: String::new(),
            serial_port: String::new(),
            baud_rate: default_baud_rate(),
            hamlib_source: default_hamlib_source(),
            custom_rigctld_path: String::new(),
        }
    }
}

/// Konfiguracja backendu FLRig (XML-RPC po TCP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlrigConfig {
    /// Adres hosta procesu FLRig.
    pub host: String,
    /// Port TCP XML-RPC FLRig (zazwyczaj 12345).
    pub port: u16,
}

impl Default for FlrigConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 12345,
        }
    }
}

impl FlrigConfig {
    /// Walidacja konfiguracji FLRig.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if self.host.trim().is_empty() {
            return Err(CatConfigValidationError::InvalidHost(self.host.clone()));
        }
        if self.port == 0 {
            return Err(CatConfigValidationError::InvalidPort(self.port));
        }
        Ok(())
    }
}

/// Konfiguracja backendu TCI (WebSocket SDR).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TciConfig {
    /// Adres hosta serwera TCI (np. ExpertSDR, Thetis).
    pub host: String,
    /// Port WebSocket serwera TCI (zazwyczaj 40001 lub 50001).
    pub port: u16,
}

impl Default for TciConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 40001,
        }
    }
}

impl TciConfig {
    /// Walidacja konfiguracji TCI.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if self.host.trim().is_empty() {
            return Err(CatConfigValidationError::InvalidHost(self.host.clone()));
        }
        if self.port == 0 {
            return Err(CatConfigValidationError::InvalidPort(self.port));
        }
        Ok(())
    }
}

/// Konfiguracja natywnego backendu Icom CI-V.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IcomConfig {
    /// Nazwa portu szeregowego (np. COM3 lub /dev/ttyUSB0).
    pub port_name: String,
    /// Prędkość transmisji (np. 19200, 115200).
    pub baud_rate: u32,
    /// Adres CI-V transceivera (np. 0x94 dla IC-7300, 0x98 dla IC-7610).
    pub civ_address: u8,
    /// Nazwa modelu transceivera z rejestru modeli.
    #[serde(default)]
    pub model: String,
}

impl IcomConfig {
    /// Walidacja konfiguracji Icom CI-V.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if self.port_name.trim().is_empty() {
            return Err(CatConfigValidationError::MissingSerialPort);
        }
        if self.baud_rate == 0 {
            return Err(CatConfigValidationError::InvalidBaudRate(self.baud_rate));
        }
        if self.civ_address == 0x00 || self.civ_address == 0xFF {
            return Err(CatConfigValidationError::InvalidCivAddress(
                self.civ_address,
            ));
        }
        Ok(())
    }
}

impl Default for IcomConfig {
    fn default() -> Self {
        Self {
            port_name: "COM1".to_string(),
            baud_rate: 19200,
            civ_address: 0x94,
            model: "IC-7300".to_string(),
        }
    }
}

/// Transport dla połączenia Kenwood CAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport_type")]
pub enum KenwoodTransportConfig {
    /// Połączenie szeregowe / USB Virtual COM.
    Serial {
        /// Nazwa portu szeregowego.
        port_name: String,
        /// Prędkość transmisji.
        baud_rate: u32,
    },
    /// Połączenie sieciowe KNS LAN (np. TS-890S, TS-990S).
    Lan {
        /// Adres IP / host transceivera.
        host: String,
        /// Port sterowania LAN (zazwyczaj 60000).
        port: u16,
    },
}

impl Default for KenwoodTransportConfig {
    fn default() -> Self {
        Self::Serial {
            port_name: "COM1".to_string(),
            baud_rate: 115200,
        }
    }
}

/// Konfiguracja natywnego backendu Kenwood CAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KenwoodConfig {
    /// Wybrany transport komunikacyjny.
    pub transport: KenwoodTransportConfig,
    /// Nazwa modelu transceivera z rejestru profili.
    #[serde(default)]
    pub model: String,
}

impl Default for KenwoodConfig {
    fn default() -> Self {
        Self {
            transport: KenwoodTransportConfig::default(),
            model: "TS-590SG".to_string(),
        }
    }
}

impl KenwoodConfig {
    /// Walidacja konfiguracji Kenwood.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        match &self.transport {
            KenwoodTransportConfig::Serial {
                port_name,
                baud_rate,
            } => {
                if port_name.trim().is_empty() {
                    return Err(CatConfigValidationError::MissingSerialPort);
                }
                if *baud_rate == 0 {
                    return Err(CatConfigValidationError::InvalidBaudRate(*baud_rate));
                }
            }
            KenwoodTransportConfig::Lan { host, port } => {
                if host.trim().is_empty() {
                    return Err(CatConfigValidationError::InvalidHost(host.clone()));
                }
                if *port == 0 {
                    return Err(CatConfigValidationError::InvalidPort(*port));
                }
            }
        }
        Ok(())
    }
}

/// Konfiguracja natywnego backendu Yaesu CAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YaesuConfig {
    /// Nazwa portu szeregowego / Enhanced VCP.
    pub port_name: String,
    /// Prędkość transmisji.
    pub baud_rate: u32,
    /// Nazwa modelu z rejestru profili Yaesu (np. FT-991A, FTDX10).
    #[serde(default)]
    pub model: String,
}

impl Default for YaesuConfig {
    fn default() -> Self {
        Self {
            port_name: "COM1".to_string(),
            baud_rate: 38400,
            model: "FT-991A".to_string(),
        }
    }
}

impl YaesuConfig {
    /// Walidacja konfiguracji Yaesu.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if self.port_name.trim().is_empty() {
            return Err(CatConfigValidationError::MissingSerialPort);
        }
        if self.baud_rate == 0 {
            return Err(CatConfigValidationError::InvalidBaudRate(self.baud_rate));
        }
        Ok(())
    }
}

/// Transport dla połączenia Elecraft CAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport_type")]
pub enum ElecraftTransportConfig {
    /// Połączenie szeregowe / USB Virtual COM (K3, K3S, KX2, KX3, K4).
    Serial {
        /// Nazwa portu szeregowego.
        port_name: String,
        /// Prędkość transmisji (np. 38400).
        baud_rate: u32,
    },
    /// Bezpośrednie połączenie TCP (K4, K4D, K4HD port 9200).
    Tcp {
        /// Adres IP / host transceivera K4.
        host: String,
        /// Port TCP (domyślnie 9200).
        port: u16,
    },
}

impl Default for ElecraftTransportConfig {
    fn default() -> Self {
        Self::Serial {
            port_name: "COM1".to_string(),
            baud_rate: 38400,
        }
    }
}

/// Konfiguracja natywnego backendu Elecraft CAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElecraftConfig {
    /// Wybrany transport komunikacyjny.
    pub transport: ElecraftTransportConfig,
    /// Model z rejestru profili (K3, K3S, KX2, KX3, K4, K4D, K4HD).
    #[serde(default)]
    pub model: String,
}

impl Default for ElecraftConfig {
    fn default() -> Self {
        Self {
            transport: ElecraftTransportConfig::default(),
            model: "K3".to_string(),
        }
    }
}

impl ElecraftConfig {
    /// Walidacja konfiguracji Elecraft.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        match &self.transport {
            ElecraftTransportConfig::Serial {
                port_name,
                baud_rate,
            } => {
                if port_name.trim().is_empty() {
                    return Err(CatConfigValidationError::MissingSerialPort);
                }
                if *baud_rate == 0 {
                    return Err(CatConfigValidationError::InvalidBaudRate(*baud_rate));
                }
            }
            ElecraftTransportConfig::Tcp { host, port } => {
                if host.trim().is_empty() {
                    return Err(CatConfigValidationError::InvalidHost(host.clone()));
                }
                if *port == 0 {
                    return Err(CatConfigValidationError::InvalidPort(*port));
                }
            }
        }
        Ok(())
    }
}

/// Opcje połączenia dla FlexRadio SmartSDR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "connection_mode")]
pub enum FlexRadioConnectionConfig {
    /// Automatyczne wykrywanie w sieci lokalnej przez VITA-49 discovery.
    AutoDiscovery,
    /// Ręcznie podany adres hosta i port sterowania SmartSDR.
    Manual {
        /// Adres IP / host transceivera FlexRadio.
        host: String,
        /// Port TCP sterowania SmartSDR (domyślnie 4992).
        port: u16,
    },
}

impl Default for FlexRadioConnectionConfig {
    fn default() -> Self {
        Self::AutoDiscovery
    }
}

/// Konfiguracja natywnego backendu FlexRadio SmartSDR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlexRadioConfig {
    /// Tryb połączenia (Discovery lub ręczny host).
    pub connection: FlexRadioConnectionConfig,
    /// Domyślny numer odbiornika (slice index), o ile skonfigurowany na sztywno.
    #[serde(default)]
    pub slice_id: Option<u32>,
    /// Nazwa modelu radia (np. FLEX-6400, FLEX-6600, FLEX-8600).
    #[serde(default)]
    pub model: Option<String>,
}

impl Default for FlexRadioConfig {
    fn default() -> Self {
        Self {
            connection: FlexRadioConnectionConfig::default(),
            slice_id: Some(0),
            model: Some("FLEX-6400".to_string()),
        }
    }
}

impl FlexRadioConfig {
    /// Walidacja konfiguracji FlexRadio.
    pub fn validate(&self) -> Result<(), CatConfigValidationError> {
        if let FlexRadioConnectionConfig::Manual { host, port } = &self.connection {
            if host.trim().is_empty() {
                return Err(CatConfigValidationError::InvalidHost(host.clone()));
            }
            if *port == 0 {
                return Err(CatConfigValidationError::InvalidPort(*port));
            }
        }
        Ok(())
    }
}

/// Migruje dotychczasowe, płaskie ustawienia CAT z [`AppConfig`] do modelu [`CatStationConfig`].
///
/// # Gwarancje:
/// 1. **Idempotentność**: Wielokrotne wywołanie na już zmigrowanej konfiguracji zwraca `Ok(false)`
///    i nie modyfikuje żadnych istniejących wartości.
/// 2. **Nienaruszalność danych wejściowych**: W razie błędu migracja jest wycofywana,
///    a konfiguracja wejściowa pozostaje nienaruszona.
/// 3. **Brak niejawnego Active TX**: Migracja nigdy nie tworzy samoczynnie celu `active_tx`
///    bez jednoznacznego wskazania w konfiguracji.
/// 4. **Brak operacji sprzętowych**: Migracja nie uruchamia żadnych procesów, nie łączy się
///    z siecią ani portami i nie emituje żadnych sygnałów PTT.
pub fn migrate_legacy_cat_config(
    app_config: &mut AppConfig,
) -> Result<bool, CatConfigMigrationError> {
    // 1. Sprawdź, czy konfiguracja została już zmigrowana
    if let Some(ref current) = app_config.cat_station {
        if current.schema_version == CURRENT_SCHEMA_VERSION {
            current.validate()?;
            return Ok(false);
        } else if current.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(CatConfigMigrationError::UnsupportedSchemaVersion(
                current.schema_version,
            ));
        }
    }

    // Kopia bezpieczeństwa przed rozpoczęciem migracji (rollback przy błędzie)
    let backup = app_config.cat_station.clone();

    // 2. Konwersja płaskich pól AppConfig na nowożytną strukturę
    let mig_result = (|| -> Result<CatStationConfig, CatConfigMigrationError> {
        let backend_kind_str = app_config.cat_backend.trim().to_ascii_lowercase();
        let rig_id = RigId::new("rig-1")?;
        let display_name = if !app_config.cat_rig_model.trim().is_empty() {
            app_config.cat_rig_model.clone()
        } else {
            "Radio 1".to_string()
        };

        let backend_cfg = match backend_kind_str.as_str() {
            "hamlib" | "" => RigBackendConfig::Hamlib(HamlibConfig {
                host: if app_config.cat_host.trim().is_empty() {
                    "127.0.0.1".to_string()
                } else {
                    app_config.cat_host.clone()
                },
                port: if app_config.cat_port == 0 {
                    4532
                } else {
                    app_config.cat_port
                },
                auto_start_rigctld: app_config.cat_auto_start_rigctld,
                rig_model: app_config.cat_rig_model.clone(),
                serial_port: app_config.cat_serial_port.clone(),
                baud_rate: if app_config.cat_baud_rate == 0 {
                    19200
                } else {
                    app_config.cat_baud_rate
                },
                hamlib_source: app_config.cat_hamlib_source.clone(),
                custom_rigctld_path: app_config.cat_custom_rigctld_path.clone(),
            }),
            "flrig" | "fldigi" => RigBackendConfig::Flrig(FlrigConfig {
                host: if app_config.cat_host.trim().is_empty() {
                    "127.0.0.1".to_string()
                } else {
                    app_config.cat_host.clone()
                },
                port: if app_config.cat_port == 0 {
                    12345
                } else {
                    app_config.cat_port
                },
            }),
            "tci" | "sunsdr" | "thetis" => RigBackendConfig::Tci(TciConfig {
                host: if app_config.tci_host.trim().is_empty() {
                    "127.0.0.1".to_string()
                } else {
                    app_config.tci_host.clone()
                },
                port: if app_config.tci_port == 0 {
                    40001
                } else {
                    app_config.tci_port
                },
            }),
            "icom_ci_v" | "icom-civ" | "ci-v" | "civ" | "icom" => {
                RigBackendConfig::Icom(IcomConfig {
                    port_name: app_config.cat_serial_port.clone(),
                    baud_rate: if app_config.cat_baud_rate == 0 {
                        19200
                    } else {
                        app_config.cat_baud_rate
                    },
                    civ_address: 0x94, // Standardowy adres IC-7300 jako bezpieczny domyślny
                    model: app_config.cat_rig_model.clone(),
                })
            }
            "kenwood" | "ts" => RigBackendConfig::Kenwood(KenwoodConfig {
                transport: KenwoodTransportConfig::Serial {
                    port_name: app_config.cat_serial_port.clone(),
                    baud_rate: if app_config.cat_baud_rate == 0 {
                        115200
                    } else {
                        app_config.cat_baud_rate
                    },
                },
                model: app_config.cat_rig_model.clone(),
            }),
            "yaesu" | "ft" | "ftdx" => RigBackendConfig::Yaesu(YaesuConfig {
                port_name: app_config.cat_serial_port.clone(),
                baud_rate: if app_config.cat_baud_rate == 0 {
                    38400
                } else {
                    app_config.cat_baud_rate
                },
                model: app_config.cat_rig_model.clone(),
            }),
            "elecraft" | "k3" | "k3s" | "kx2" | "kx3" | "k4" => {
                RigBackendConfig::Elecraft(ElecraftConfig {
                    transport: ElecraftTransportConfig::Serial {
                        port_name: app_config.cat_serial_port.clone(),
                        baud_rate: if app_config.cat_baud_rate == 0 {
                            38400
                        } else {
                            app_config.cat_baud_rate
                        },
                    },
                    model: app_config.cat_rig_model.clone(),
                })
            }
            "flexradio" | "flex" | "smartsdr" => RigBackendConfig::FlexRadio(FlexRadioConfig {
                connection: FlexRadioConnectionConfig::AutoDiscovery,
                slice_id: None,
                model: None,
            }),
            unknown => return Err(CatConfigMigrationError::UnknownBackend(unknown.to_string())),
        };

        let poll_interval_ms = if app_config.cat_poll_rate_ms == 0 {
            200
        } else {
            app_config.cat_poll_rate_ms
        };

        let rig_config = RigConfig {
            id: rig_id.clone(),
            enabled: app_config.cat_enabled,
            display_name,
            backend: backend_cfg,
            poll_interval_ms,
            command_timeout_ms: 1500,
        };

        let mut rigs = Vec::new();
        let active_rx = if app_config.cat_enabled {
            rigs.push(rig_config);
            Some(RigTargetConfig::rig_only(rig_id))
        } else {
            rigs.push(rig_config);
            None
        };

        // Zgodnie z wytycznymi bezpieczeństwa: brak automatycznego tworzenia Active TX przy migracji
        let active_tx = None;

        let new_cat_station = CatStationConfig {
            schema_version: CURRENT_SCHEMA_VERSION,
            rigs,
            active_rx,
            active_tx,
        };

        new_cat_station.validate()?;
        Ok(new_cat_station)
    })();

    match mig_result {
        Ok(station_cfg) => {
            app_config.cat_station = Some(station_cfg);
            Ok(true)
        }
        Err(err) => {
            // Rollback przy błędzie
            app_config.cat_station = backup;
            Err(err)
        }
    }
}
