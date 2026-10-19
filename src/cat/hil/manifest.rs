// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::error::HilError;
use super::evidence::{HilEvidenceOrigin, HilExecutionContext};
use crate::cat::backend::CatBackendKind;
use crate::cat::config::{
    ElecraftConfig, ElecraftTransportConfig, FlexRadioConfig, FlexRadioConnectionConfig,
    FlrigConfig, HamlibConfig, IcomConfig, KenwoodConfig, KenwoodTransportConfig, RigBackendConfig,
    RigConfig, TciConfig, YaesuConfig,
};
use crate::cat::routing::RigId;

/// Wymiary macierzy testowej HIL: backend x radio x transport x firmware.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilMatrixEntry {
    /// Rodzaj backendu CAT (np. IcomCiV, Kenwood, Yaesu, Elecraft, FlexRadio, Hamlib, FLRig, TCI).
    pub backend_kind: CatBackendKind,
    /// Unikalny identyfikator instancji radia w systemie (np. "RIG-IC7300-01").
    pub rig_id: RigId,
    /// Model radiotelefonu (z rejestru profili).
    pub model_name: String,
    /// Precyzyjna konfiguracja transportu szeregowego lub sieciowego.
    pub transport: HilTransportConfig,
    /// Wersja oprogramowania układowego (firmware) testowanego transceivera.
    pub firmware_version: String,
}

/// Konfiguracja transportu komunikacyjnego dla stanowiska HIL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HilTransportConfig {
    /// Port szeregowy / Virtual COM.
    Serial {
        /// Nazwa portu szeregowego (np. "COM3", "/dev/ttyUSB0").
        port_name: String,
        /// Prędkość transmisji (np. 19200, 38400, 115200).
        baud_rate: u32,
        /// Liczba bitów danych (standardowo 8).
        #[serde(default = "default_data_bits")]
        data_bits: u8,
        /// Kontrola parzystości: "none", "odd", "even".
        #[serde(default = "default_parity")]
        parity: String,
        /// Bity stopu: 1 lub 2.
        #[serde(default = "default_stop_bits")]
        stop_bits: u8,
        /// Kontrola przepływu: "none", "rts_cts", "xon_xoff".
        #[serde(default = "default_flow_control")]
        flow_control: String,
    },
    /// Połączenie sieciowe TCP / WebSocket.
    Network {
        /// Adres IP lub nazwa hosta (np. "192.168.1.100", "127.0.0.1").
        host: String,
        /// Port TCP.
        port: u16,
    },
}

fn default_data_bits() -> u8 {
    8
}

fn default_parity() -> String {
    "none".to_string()
}

fn default_stop_bits() -> u8 {
    1
}

fn default_flow_control() -> String {
    "none".to_string()
}

/// Specyfikacja sztucznego obciążenia 50 omów wymaganego do testów HIL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilDummyLoadSpec {
    /// Dopuszczalna moc ciągła lub impulsowa sztucznego obciążenia (W).
    pub power_rating_watts: f64,
    /// Maksymalny dopuszczalny współczynnik SWR obciążenia (np. 1.15).
    pub max_swr: f64,
    /// Numer seryjny lub oznaczenie laboratoryjne obciążenia.
    pub serial_number: String,
    /// Impedancja znamionowa (omów, wymagane 50.0).
    #[serde(default = "default_impedance_ohms")]
    pub impedance_ohms: f64,
}

fn default_impedance_ohms() -> f64 {
    50.0
}

/// Zakończenie toru w.cz. (RF) podczas testów.
///
/// Zasada bezpieczeństwa: Wszelkie testy nadawania (PTT) w warunkach HIL
/// bezwzględnie wymagają zakończenia sztucznym obciążeniem. Zakończenie anteną
/// jest dopuszczalne wyłącznie w trybie czystego nasłuchu (RX) i bezwzględnie
/// blokuje jakiekolwiek załączenie PTT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HilRfTermination {
    /// Sztuczne obciążenie 50 omów (wymagane dla prób PTT).
    DummyLoad(HilDummyLoadSpec),
    /// Antena rzeczywista (bezwzględnie zabroniona dla prób PTT w HIL).
    Antenna {
        /// Identyfikator lub nazwa anteny.
        antenna_name: String,
    },
}

/// Konfiguracja bezpieczeństwa radiowego w.cz. (RF) stanowiska laboratoryjnego.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilRfSafetyConfig {
    /// Rodzaj zakończenia toru RF.
    pub termination: HilRfTermination,
    /// Maksymalna moc nadajnika dopuszczona podczas testu (W).
    pub max_test_power_watts: f64,
    /// Dozwolony zakres częstotliwości testowej w Hz (początek, koniec).
    pub frequency_range_hz: (u64, u64),
    /// Czy logiczna blokada interlock jest włączona.
    #[serde(default = "default_interlock_engaged")]
    pub interlock_engaged: bool,
    /// Maksymalny dopuszczalny czas trwania impulsu testowego w milisekundach (<= 500 ms).
    #[serde(default = "default_max_duration_ms")]
    pub max_test_duration_ms: u64,
    /// Potwierdzenie wyłączenia SmartLink dla prób HIL (Warunek 25).
    #[serde(default = "default_true")]
    pub smartlink_disabled: bool,
    /// Potwierdzenie wyłączenia zdalnego, nienadzorowanego TX (Warunek 25).
    #[serde(default = "default_true")]
    pub unattended_remote_tx_disabled: bool,
    /// Potwierdzenie, że ewentualny endpoint sieciowy jest lokalny lub w pełni kontrolowany (Warunek 25).
    #[serde(default = "default_true")]
    pub remote_endpoint_is_local_or_controlled: bool,
    /// Potwierdzenie posiadania natychmiastowej fizycznej kontroli zatrzymania nadajnika (Warunek 25).
    #[serde(default = "default_true")]
    pub operator_has_immediate_stop_control: bool,
    /// Potwierdzenie wyłączenia strumieni Data Plane (Audio, DAX, IQ, panadapter, waterfall) poza zakresem testu (Warunek 26).
    #[serde(default = "default_true")]
    pub out_of_band_data_plane_disabled: bool,
}

fn default_interlock_engaged() -> bool {
    true
}

fn default_max_duration_ms() -> u64 {
    500
}

/// Oświadczenie i potwierdzenie fizyczne operatora stanowiska laboratoryjnego.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilOperatorAttestation {
    /// Potwierdzenie fizycznego dokręcenia i podłączenia sztucznego obciążenia 50 omów.
    pub dummy_load_physically_connected: bool,
    /// Potwierdzenie zabezpieczenia strefy bezpieczeństwa przed emisją RF i osobami postronnymi.
    pub rf_perimeter_secured: bool,
    /// Świadoma, jawna zgoda operatora na przeprowadzenie prób załączenia nadajnika (PTT).
    pub ptt_consent_granted: bool,
    /// Sygnatura czasowa atestacji operatora (ISO 8601).
    pub attestation_timestamp: String,
    /// Opis stanu stanowiska i procedury weryfikacji.
    #[serde(default = "default_description")]
    pub description: String,
    /// Potwierdzenie fizycznego odłączenia anteny nadawczej.
    #[serde(default = "default_true")]
    pub antenna_disconnected: bool,
    /// Potwierdzenie odłączenia lub zablokowania wzmacniacza mocy (PA).
    #[serde(default = "default_true")]
    pub pa_disconnected_or_locked: bool,
    /// Potwierdzenie wyłączenia lub zbypassowania skrzynki antenowej (tuner).
    #[serde(default = "default_true")]
    pub tuner_bypassed: bool,
    /// Dostępność fizycznego wyłącznika awaryjnego (STOP).
    #[serde(default = "default_true")]
    pub physical_stop_available: bool,
    /// Udokumentowana procedura awaryjnego przerwania testu.
    #[serde(default = "default_true")]
    pub abort_procedure_documented: bool,
    /// Formalne potwierdzenie zapoznania się operatora ze stanowiskiem.
    #[serde(default = "default_true")]
    pub operator_acknowledged: bool,
    /// Jawne potwierdzenie gotowości stanowiska laboratoryjnego przez operatora (Warunek 20).
    #[serde(default = "default_true")]
    pub bench_readiness_confirmed: bool,
    /// Potwierdzenie fizycznej obecności operatora przy stanowisku (HIL-X21).
    #[serde(default = "default_true")]
    pub operator_present: bool,
    /// Potwierdzenie prowadzenia próby pod bezpośrednim nadzorem operatora (HIL-X22).
    #[serde(default = "default_true")]
    pub supervised_trial_confirmed: bool,
    /// Potwierdzenie zgodności próby z regulaminem i zasadami laboratorium (Warunek 24).
    #[serde(default = "default_true")]
    pub rules_compliance_attested: bool,
    /// Potwierdzenie posiadania wymaganych uprawnień operatorskich (Warunek 24).
    #[serde(default = "default_true")]
    pub operator_privileges_attested: bool,
    /// Potwierdzenie dopuszczalności pasma częstotliwości (Warunek 24).
    #[serde(default = "default_true")]
    pub allowed_band_attested: bool,
    /// Potwierdzenie dopuszczalności rodzaju emisji / modulacji (Warunek 24).
    #[serde(default = "default_true")]
    pub allowed_mode_attested: bool,
    /// Potwierdzenie braku nieautoryzowanej emisji radiowej (Warunek 24).
    #[serde(default = "default_true")]
    pub no_unauthorized_emission_attested: bool,
    /// Potwierdzenie znajomości procedury STOP przez osoby obecne przy stanowisku.
    #[serde(default = "default_true")]
    pub personnel_know_stop_procedure: bool,
}

fn default_description() -> String {
    "Verified laboratory test bench configuration".to_string()
}

fn default_true() -> bool {
    true
}

/// Odczyt z certyfikowanego przyrządu pomiarowego (miernik mocy / SWR / analizator).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilInstrumentMeasurement {
    /// Identyfikator laboratoryjny przyrządu pomiarowego.
    pub instrument_id: String,
    /// Model i producent przyrządu (np. "Bird 43", "Rohde&Schwarz FSH4").
    pub instrument_model: String,
    /// Zmierzona wartość współczynnika fali stojącej (SWR).
    pub measured_swr: f64,
    /// Opcjonalnie zmierzona moc w.cz. (W).
    #[serde(default)]
    pub measured_power_watts: Option<f64>,
    /// Ważność kalibracji przyrządu.
    #[serde(default)]
    pub calibration_valid_until: Option<String>,
    /// Sygnatura czasowa pomiaru.
    #[serde(default)]
    pub measurement_timestamp: Option<String>,
    /// Pochodzenie dowodu pomiarowego (fizyczny pomiar vs syntetyczny fixture testowy).
    #[serde(default)]
    pub origin: HilEvidenceOrigin,
}

impl HilInstrumentMeasurement {
    /// Czy pomiar pochodzi z rzeczywistego fizycznego przyrządu laboratoryjnego.
    pub fn is_physical_measurement(&self) -> bool {
        self.origin == HilEvidenceOrigin::PhysicalMeasurement
    }

    /// Weryfikuje, czy odczyt przyrządu stanowi ważny fizyczny dowód runtime.
    pub fn validate_physical_evidence(
        &self,
        session_id: &str,
        context: HilExecutionContext,
    ) -> Result<(), HilError> {
        if context != HilExecutionContext::PhysicalRuntime {
            return Err(HilError::NotHilReady(
                "Physical measurement requires PhysicalRuntime context".into(),
            ));
        }
        if !self.is_physical_measurement() {
            return Err(HilError::NotHilReady(
                "Evidence origin is not PhysicalMeasurement".into(),
            ));
        }
        if session_id.trim().is_empty() {
            return Err(HilError::InvalidManifest(
                "Missing session_id for physical measurement".into(),
            ));
        }
        if self
            .measurement_timestamp
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            return Err(HilError::InvalidManifest(
                "Missing measurement_timestamp for physical measurement".into(),
            ));
        }
        if self.instrument_id.trim().is_empty() || self.instrument_model.trim().is_empty() {
            return Err(HilError::InvalidManifest(
                "Missing instrument identity (instrument_id and instrument_model required)".into(),
            ));
        }
        Ok(())
    }
}

/// Kompletny manifest stanowiska testowego Hardware-In-The-Loop (HIL).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilTestBenchManifest {
    /// Unikalny identyfikator stanowiska laboratoryjnego (np. "LAB-BENCH-SP6INA-01").
    pub bench_id: String,
    /// Identyfikator sesji laboratoryjnej.
    #[serde(default)]
    pub session_id: String,
    /// Czas utworzenia sesji (ISO 8601).
    #[serde(default)]
    pub created_at: String,
    /// Czas wygaśnięcia sesji (ISO 8601, opcjonalny).
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Pełny SHA testowanego commita Git.
    #[serde(default)]
    pub tested_commit: Option<String>,
    /// Opcjonalny oczekiwany pełny SHA commita do weryfikacji zgodności (Warunek 3).
    #[serde(default)]
    pub expected_commit: Option<String>,
    /// Potwierdzenie czystego drzewa roboczego Git (Warunek 1).
    #[serde(default = "default_true")]
    pub working_tree_clean: bool,
    /// Potwierdzenie przejścia wszystkich testów programowych (Warunek 2).
    #[serde(default = "default_true")]
    pub software_tests_passed: bool,
    /// Identyfikator operatora (np. znak lub login).
    #[serde(default)]
    pub operator_id: String,
    /// Znak wywoławczy operatora odpowiedzialnego za stanowisko.
    pub operator_callsign: String,
    /// Imię i nazwisko operatora.
    pub operator_name: String,
    /// Flaga zezwolenia na sesję HIL w manifeście.
    #[serde(default)]
    pub hil_enabled: bool,
    /// Jawna flaga zezwolenia na próby nadawania (TX).
    #[serde(default)]
    pub allow_tx: bool,
    /// Wymiary macierzy testowej (backend x radio x transport x firmware).
    pub matrix: HilMatrixEntry,
    /// Konfiguracja bezpieczeństwa toru RF.
    pub rf_safety: HilRfSafetyConfig,
    /// Atestacja fizyczna operatora.
    pub operator_attestation: HilOperatorAttestation,
    /// Opcjonalna weryfikacja zewnętrznym instrumentem pomiarowym.
    #[serde(default)]
    pub instrument_verification: Option<HilInstrumentMeasurement>,
}

impl HilTestBenchManifest {
    /// Deserializuje manifest ze wskazanego ciągu znaków w formacie JSON.
    pub fn from_json(json_str: &str) -> Result<Self, HilError> {
        let manifest: Self = serde_json::from_str(json_str)
            .map_err(|e| HilError::InvalidManifest(format!("JSON parse error: {e}")))?;
        manifest.validate_structure()?;
        Ok(manifest)
    }

    /// Wczytuje i parsuje manifest z pliku dyskowego.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, HilError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| HilError::IoError(format!("Cannot read manifest file: {e}")))?;
        Self::from_json(&content)
    }

    /// Serializuje manifest do znormalizowanego formatu JSON z wcięciami.
    pub fn to_json(&self) -> Result<String, HilError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| HilError::InvalidManifest(format!("JSON serialization error: {e}")))
    }

    /// Sprawdza formalną poprawność strukturalną manifestu.
    pub fn validate_structure(&self) -> Result<(), HilError> {
        if self.bench_id.trim().is_empty() {
            return Err(HilError::InvalidManifest(
                "bench_id cannot be empty".to_string(),
            ));
        }
        if self.operator_callsign.trim().is_empty() {
            return Err(HilError::InvalidManifest(
                "operator_callsign cannot be empty".to_string(),
            ));
        }
        if self.matrix.model_name.trim().is_empty() {
            return Err(HilError::InvalidManifest(
                "model_name cannot be empty".to_string(),
            ));
        }
        if self.matrix.rig_id.as_str().trim().is_empty() {
            return Err(HilError::InvalidManifest(
                "matrix rig_id cannot be empty".to_string(),
            ));
        }

        match &self.matrix.transport {
            HilTransportConfig::Serial {
                port_name,
                baud_rate,
                ..
            } => {
                let trimmed = port_name.trim();
                if trimmed.is_empty() {
                    return Err(HilError::InvalidTransport(
                        "Serial port_name cannot be empty".to_string(),
                    ));
                }
                let upper = trimmed.to_ascii_uppercase();
                if upper == "AUTO" || upper == "FIRST" || upper == "DETECT" {
                    return Err(HilError::InvalidTransport(
                        "Automatic port selection is forbidden for HIL test bench; explicit port required".to_string(),
                    ));
                }
                if *baud_rate < 1200 {
                    return Err(HilError::InvalidTransport(format!(
                        "Invalid baud_rate: {baud_rate} (must be >= 1200)"
                    )));
                }
            }
            HilTransportConfig::Network { host, port } => {
                if host.trim().is_empty() {
                    return Err(HilError::InvalidTransport(
                        "Network host cannot be empty".to_string(),
                    ));
                }
                if *port == 0 {
                    return Err(HilError::InvalidTransport(
                        "Network port cannot be 0".to_string(),
                    ));
                }
            }
        }

        if self.rf_safety.max_test_power_watts <= 0.0 {
            return Err(HilError::SafetyViolation(
                "max_test_power_watts must be positive and non-zero".to_string(),
            ));
        }
        if self.rf_safety.max_test_duration_ms == 0 {
            return Err(HilError::SafetyViolation(
                "max_test_duration_ms cannot be zero".to_string(),
            ));
        }
        if self.rf_safety.max_test_duration_ms > 500 {
            return Err(HilError::SafetyViolation(format!(
                "max_test_duration_ms ({}) exceeds laboratory safety limit 500 ms",
                self.rf_safety.max_test_duration_ms
            )));
        }
        if self.rf_safety.frequency_range_hz.0 >= self.rf_safety.frequency_range_hz.1 {
            return Err(HilError::SafetyViolation(
                "Invalid frequency range: min must be strictly less than max".to_string(),
            ));
        }

        if let HilRfTermination::DummyLoad(spec) = &self.rf_safety.termination
            && self.rf_safety.max_test_power_watts > spec.power_rating_watts
        {
            return Err(HilError::SafetyViolation(format!(
                "max_test_power_watts ({:.1} W) exceeds dummy load rating ({:.1} W)",
                self.rf_safety.max_test_power_watts, spec.power_rating_watts
            )));
        }

        if self
            .operator_attestation
            .attestation_timestamp
            .trim()
            .is_empty()
        {
            return Err(HilError::InvalidConsent(
                "operator_attestation attestation_timestamp cannot be empty".to_string(),
            ));
        }
        if self.operator_attestation.description.trim().is_empty() {
            return Err(HilError::InvalidConsent(
                "operator_attestation description cannot be empty".to_string(),
            ));
        }

        if let Some(ref inst) = self.instrument_verification
            && inst.measured_swr <= 0.0
        {
            return Err(HilError::SafetyViolation(
                "measured_swr must be positive".to_string(),
            ));
        }

        if let Some(exp_str) = &self.expires_at
            && let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(exp_str)
            && let Ok(created_time) = chrono::DateTime::parse_from_rfc3339(&self.created_at)
            && exp_time < created_time
        {
            return Err(HilError::InvalidManifest(format!(
                "Session expires_at ({exp_str}) cannot be before created_at ({})",
                self.created_at
            )));
        }

        Ok(())
    }

    /// Konwertuje manifest do formatu konfiguracji sesji radiowej [`RigConfig`].
    pub fn to_rig_config(&self) -> Result<RigConfig, HilError> {
        let backend = match self.matrix.backend_kind {
            CatBackendKind::Hamlib => match &self.matrix.transport {
                HilTransportConfig::Network { host, port } => {
                    RigBackendConfig::Hamlib(HamlibConfig {
                        host: host.clone(),
                        port: *port,
                        auto_start_rigctld: false,
                        rig_model: self.matrix.model_name.clone(),
                        serial_port: String::new(),
                        baud_rate: 19_200,
                        hamlib_source: "bundled".to_string(),
                        custom_rigctld_path: String::new(),
                    })
                }
                HilTransportConfig::Serial { .. } => {
                    return Err(HilError::InvalidTransport(
                        "Hamlib backend requires Network transport (host:port to rigctld)"
                            .to_string(),
                    ));
                }
            },
            CatBackendKind::Flrig => match &self.matrix.transport {
                HilTransportConfig::Network { host, port } => {
                    RigBackendConfig::Flrig(FlrigConfig {
                        host: host.clone(),
                        port: *port,
                    })
                }
                HilTransportConfig::Serial { .. } => {
                    return Err(HilError::InvalidTransport(
                        "FLRig backend requires Network transport (XML-RPC host:port)".to_string(),
                    ));
                }
            },
            CatBackendKind::Tci => match &self.matrix.transport {
                HilTransportConfig::Network { host, port } => RigBackendConfig::Tci(TciConfig {
                    host: host.clone(),
                    port: *port,
                }),
                HilTransportConfig::Serial { .. } => {
                    return Err(HilError::InvalidTransport(
                        "TCI backend requires Network transport (WebSocket host:port)".to_string(),
                    ));
                }
            },
            CatBackendKind::IcomCiV => match &self.matrix.transport {
                HilTransportConfig::Serial {
                    port_name,
                    baud_rate,
                    ..
                } => RigBackendConfig::Icom(IcomConfig {
                    port_name: port_name.clone(),
                    baud_rate: *baud_rate,
                    civ_address: 0x94, // Standard Icom CI-V address for IC-7300 or profile default
                    model: self.matrix.model_name.clone(),
                }),
                HilTransportConfig::Network { .. } => {
                    return Err(HilError::InvalidTransport(
                        "Icom CI-V native backend requires Serial transport".to_string(),
                    ));
                }
            },
            CatBackendKind::Kenwood => match &self.matrix.transport {
                HilTransportConfig::Serial {
                    port_name,
                    baud_rate,
                    ..
                } => RigBackendConfig::Kenwood(KenwoodConfig {
                    transport: KenwoodTransportConfig::Serial {
                        port_name: port_name.clone(),
                        baud_rate: *baud_rate,
                    },
                    model: self.matrix.model_name.clone(),
                }),
                HilTransportConfig::Network { host, port } => {
                    RigBackendConfig::Kenwood(KenwoodConfig {
                        transport: KenwoodTransportConfig::Lan {
                            host: host.clone(),
                            port: *port,
                        },
                        model: self.matrix.model_name.clone(),
                    })
                }
            },
            CatBackendKind::Yaesu => match &self.matrix.transport {
                HilTransportConfig::Serial {
                    port_name,
                    baud_rate,
                    ..
                } => RigBackendConfig::Yaesu(YaesuConfig {
                    port_name: port_name.clone(),
                    baud_rate: *baud_rate,
                    model: self.matrix.model_name.clone(),
                }),
                HilTransportConfig::Network { .. } => {
                    return Err(HilError::InvalidTransport(
                        "Yaesu CAT native backend requires Serial transport".to_string(),
                    ));
                }
            },
            CatBackendKind::Elecraft => match &self.matrix.transport {
                HilTransportConfig::Serial {
                    port_name,
                    baud_rate,
                    ..
                } => RigBackendConfig::Elecraft(ElecraftConfig {
                    transport: ElecraftTransportConfig::Serial {
                        port_name: port_name.clone(),
                        baud_rate: *baud_rate,
                    },
                    model: self.matrix.model_name.clone(),
                }),
                HilTransportConfig::Network { host, port } => {
                    RigBackendConfig::Elecraft(ElecraftConfig {
                        transport: ElecraftTransportConfig::Tcp {
                            host: host.clone(),
                            port: *port,
                        },
                        model: self.matrix.model_name.clone(),
                    })
                }
            },
            CatBackendKind::FlexRadio => match &self.matrix.transport {
                HilTransportConfig::Network { host, port } => {
                    RigBackendConfig::FlexRadio(FlexRadioConfig {
                        connection: FlexRadioConnectionConfig::Manual {
                            host: host.clone(),
                            port: *port,
                        },
                        slice_id: Some(0),
                        model: Some(self.matrix.model_name.clone()),
                    })
                }
                HilTransportConfig::Serial { .. } => {
                    return Err(HilError::InvalidTransport(
                        "FlexRadio SmartSDR requires Network transport (TCP 4992)".to_string(),
                    ));
                }
            },
            CatBackendKind::So2r => {
                return Err(HilError::InvalidManifest(
                    "SO2R is a dual-radio coordinator layer, not a direct radio backend"
                        .to_string(),
                ));
            }
        };

        Ok(RigConfig {
            id: self.matrix.rig_id.clone(),
            enabled: true,
            display_name: format!(
                "HIL: {} ({})",
                self.matrix.model_name,
                self.matrix.backend_kind.as_str()
            ),
            backend,
            poll_interval_ms: 200,
            command_timeout_ms: 1500,
        })
    }

    /// Zwraca zanonimizowaną kopię manifestu bez poufnych danych.
    pub fn redacted_manifest(&self) -> Self {
        let mut copy = self.clone();
        if let HilTransportConfig::Network { host, .. } = &mut copy.matrix.transport
            && host != "127.0.0.1"
            && host != "localhost"
        {
            *host = "[REDACTED_HOST]".to_string();
        }
        copy
    }
}
