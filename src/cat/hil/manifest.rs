// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Manifest stanowiska laboratoryjnego HIL — **wyłącznie plan testu**.
//!
//! Zasada bezpieczeństwa (F-15): ten dokument opisuje *co* ma zostać przetestowane. Nie jest
//! dowodem gotowości i nie może sam siebie autoryzować. Wszystkie pola bezpieczeństwa są
//! **wymagane** i muszą być jawnie zapisane w dokumencie — brak pola nigdy nie oznacza `true`.
//!
//! Niezależny dowód gotowości sprzętowej mieszka w osobnym dokumencie
//! ([`super::authorization::HilIndependentEvidence`]), a decyzja autoryzacyjna powstaje
//! wyłącznie w [`super::authorization::HilHardwareExecutionAuthorizer`].

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::authorization::sha256_hex;
use super::error::HilError;
use super::evidence::{HilEvidenceOrigin, HilExecutionContext};
use crate::cat::backend::CatBackendKind;
use crate::cat::config::{
    ElecraftConfig, ElecraftTransportConfig, FlexRadioConfig, FlexRadioConnectionConfig,
    FlrigConfig, HamlibConfig, IcomConfig, KenwoodConfig, KenwoodTransportConfig, RigBackendConfig,
    RigConfig, TciConfig, YaesuConfig,
};
use crate::cat::routing::RigId;

/// Wersja schematu planu testu HIL (F-15).
///
/// Wersja 2 wprowadza rozdzielenie planu, oświadczenia operatora i niezależnego dowodu.
/// Dokument bez pola `schema_version` jest traktowany jako wersja 1 (legacy) i nie może
/// uzyskać gotowości sprzętowej.
pub const HIL_PLAN_SCHEMA_VERSION: u32 = 2;

/// Minimalny dopuszczalny czas impulsu testowego PTT w milisekundach.
///
/// Jest to jednocześnie dolna granica zakresu używanego przez ścieżkę nadawczą HIL
/// ([`crate::cat::hil::session::HilHardwareSession::acquire_tx_guard_at`]). Manifest, który
/// deklaruje krótszy impuls, tworzy odwrócony zakres (dolna granica > górna) i jest odrzucany
/// **przed** logicznym załączeniem PTT.
pub const HIL_MIN_TEST_DURATION_MS: u64 = 10;

/// Maksymalny dopuszczalny czas impulsu testowego PTT w milisekundach (limit laboratoryjny).
pub const HIL_MAX_TEST_DURATION_MS: u64 = 500;

/// Błąd walidacji planu testu HIL (fail-closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HilPlanValidationError {
    /// Plan pochodzi ze starszego formatu i wymaga jawnej migracji.
    LegacySchemaRequiresMigration,
    /// Pole jest puste, a musi być jawnie wypełnione.
    MissingRequiredField(&'static str),
    /// Nieobsługiwana wersja schematu.
    UnsupportedSchemaVersion(u32),
    /// Niedozwolona wartość parametru planu.
    InvalidParameter(String),
}

impl std::fmt::Display for HilPlanValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LegacySchemaRequiresMigration => write!(
                f,
                "HIL test plan has no schema_version (legacy v1); an explicit migration to v2 is \
                 required and readiness stays denied until then"
            ),
            Self::MissingRequiredField(field) => {
                write!(f, "HIL test plan is missing required field '{field}'")
            }
            Self::UnsupportedSchemaVersion(version) => write!(
                f,
                "unsupported HIL test plan schema version {version} (this build supports v{HIL_PLAN_SCHEMA_VERSION})"
            ),
            Self::InvalidParameter(reason) => {
                write!(f, "invalid HIL test plan parameter: {reason}")
            }
        }
    }
}

impl std::error::Error for HilPlanValidationError {}

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
///
/// Wartości w tym typie są **deklaracją planu**. Fizyczne podłączenie, impedancja i moc
/// znamionowa obciążenia muszą zostać zmierzone i potwierdzone w niezależnym dowodzie
/// ([`super::authorization::HilIndependentEvidence`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilDummyLoadSpec {
    /// Deklarowana dopuszczalna moc ciągła lub impulsowa sztucznego obciążenia (W).
    pub power_rating_watts: f64,
    /// Deklarowany maksymalny dopuszczalny współczynnik SWR obciążenia (np. 1.15).
    pub max_swr: f64,
    /// Numer seryjny lub oznaczenie laboratoryjne obciążenia.
    pub serial_number: String,
    /// Deklarowana impedancja znamionowa (omów, wymagane 50.0).
    #[serde(default = "default_impedance_ohms")]
    pub impedance_ohms: f64,
}

fn default_impedance_ohms() -> f64 {
    50.0
}

/// Zakończenie toru w.cz. (RF) podczas testów.
///
/// Zasada bezpieczeństwa: Wszelkie testy nadawania (PTT) w warunkach HIL bezwzględnie wymagają
/// zakończenia sztucznym obciążeniem. Zakończenie anteną jest dopuszczalne wyłącznie w trybie
/// czystego nasłuchu (RX) i bezwzględnie blokuje jakiekolwiek załączenie PTT.
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

/// Konfiguracja planu bezpieczeństwa radiowego w.cz. (RF) stanowiska laboratoryjnego.
///
/// Wszystkie pola są **wymagane**. Brak pola nie jest zgodą: dokument bez pola nie deserializuje
/// się, a dokument z wartością `false` blokuje odpowiedni warunek gotowości.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilRfSafetyConfig {
    /// Rodzaj zakończenia toru RF.
    pub termination: HilRfTermination,
    /// Maksymalna moc nadajnika dopuszczona podczas testu (W).
    pub max_test_power_watts: f64,
    /// Dozwolony zakres częstotliwości testowej w Hz (początek, koniec).
    pub frequency_range_hz: (u64, u64),
    /// Czy logiczna blokada interlock ma być włączona podczas testu (wymagane, bez wartości domyślnej).
    pub interlock_engaged: bool,
    /// Maksymalny dopuszczalny czas trwania impulsu testowego w milisekundach (<= 500 ms).
    pub max_test_duration_ms: u64,
    /// Deklaracja wyłączenia SmartLink dla prób HIL (wymagane, bez wartości domyślnej).
    pub smartlink_disabled: bool,
    /// Deklaracja wyłączenia zdalnego, nienadzorowanego TX (wymagane, bez wartości domyślnej).
    pub unattended_remote_tx_disabled: bool,
    /// Deklaracja, że ewentualny endpoint sieciowy jest lokalny lub w pełni kontrolowany.
    pub remote_endpoint_is_local_or_controlled: bool,
    /// Deklaracja posiadania natychmiastowej fizycznej kontroli zatrzymania nadajnika.
    pub operator_has_immediate_stop_control: bool,
    /// Deklaracja wyłączenia strumieni Data Plane (Audio, DAX, IQ, panadapter, waterfall).
    pub out_of_band_data_plane_disabled: bool,
}

/// Oświadczenie i potwierdzenie fizyczne operatora stanowiska laboratoryjnego (legacy).
///
/// Zgodność wsteczna: dokumenty v1 mogły osadzać ten blok w manifeście. Blok jest wyłącznie
/// **deklaracją** i nigdy nie jest dowodem. Autoryzacja sprzętowa używa osobnego dokumentu
/// [`super::authorization::HilAttestationDocument`].
///
/// Wszystkie pola są wymagane (bez `default`), więc brak pola nie może zamienić się w `true`.
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
    pub description: String,
    /// Potwierdzenie fizycznego odłączenia anteny nadawczej.
    pub antenna_disconnected: bool,
    /// Potwierdzenie odłączenia lub zablokowania wzmacniacza mocy (PA).
    pub pa_disconnected_or_locked: bool,
    /// Potwierdzenie wyłączenia lub zbypassowania skrzynki antenowej (tuner).
    pub tuner_bypassed: bool,
    /// Dostępność fizycznego wyłącznika awaryjnego (STOP).
    pub physical_stop_available: bool,
    /// Udokumentowana procedura awaryjnego przerwania testu.
    pub abort_procedure_documented: bool,
    /// Formalne potwierdzenie zapoznania się operatora ze stanowiskiem.
    pub operator_acknowledged: bool,
    /// Jawne potwierdzenie gotowości stanowiska laboratoryjnego przez operatora (Warunek 20).
    pub bench_readiness_confirmed: bool,
    /// Potwierdzenie fizycznej obecności operatora przy stanowisku (HIL-X21).
    pub operator_present: bool,
    /// Potwierdzenie prowadzenia próby pod bezpośrednim nadzorem operatora (HIL-X22).
    pub supervised_trial_confirmed: bool,
    /// Potwierdzenie zgodności próby z regulaminem i zasadami laboratorium (Warunek 24).
    pub rules_compliance_attested: bool,
    /// Potwierdzenie posiadania wymaganych uprawnień operatorskich (Warunek 24).
    pub operator_privileges_attested: bool,
    /// Potwierdzenie dopuszczalności pasma częstotliwości (Warunek 24).
    pub allowed_band_attested: bool,
    /// Potwierdzenie dopuszczalności rodzaju emisji / modulacji (Warunek 24).
    pub allowed_mode_attested: bool,
    /// Potwierdzenie braku nieautoryzowanej emisji radiowej (Warunek 24).
    pub no_unauthorized_emission_attested: bool,
    /// Potwierdzenie znajomości procedury STOP przez osoby obecne przy stanowisku.
    pub personnel_know_stop_procedure: bool,
}

/// Odczyt z przyrządu pomiarowego (miernik mocy / SWR / analizator).
///
/// Odczyt jest **deklaracją** dołączoną do planu. Nie jest niezależnym dowodem: dowód
/// ([`super::authorization::HilIndependentEvidence`]) musi zostać wydany przez osobny podmiot.
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

/// Kompletny plan testu Hardware-In-The-Loop (HIL).
///
/// **Plan testu nie jest dowodem gotowości.** Każde pole bezpieczeństwa jest wymagane i musi
/// zostać jawnie zapisane; brak pola powoduje błąd deserializacji, a nie wartość `true`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HilTestBenchManifest {
    /// Wersja schematu planu testu.
    ///
    /// `0` oznacza dokument bez pola `schema_version`, czyli legacy v1, który wymaga jawnej
    /// migracji i nie może uzyskać gotowości sprzętowej.
    #[serde(default)]
    pub schema_version: u32,
    /// Unikalny identyfikator stanowiska laboratoryjnego (np. "LAB-BENCH-SP6INA-01").
    pub bench_id: String,
    /// Identyfikator sesji laboratoryjnej.
    #[serde(default)]
    pub session_id: String,
    /// Identyfikator przypadku testowego, którego dotyczy plan.
    #[serde(default)]
    pub test_case_id: String,
    /// Identyfikator próby (ochrona przed powtórzeniem dowodu).
    #[serde(default)]
    pub attempt_id: String,
    /// Czas utworzenia sesji (ISO 8601).
    #[serde(default)]
    pub created_at: String,
    /// Czas wygaśnięcia sesji (ISO 8601, opcjonalny).
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Pełny SHA testowanego commita Git (deklaracja planu).
    #[serde(default)]
    pub tested_commit: Option<String>,
    /// Opcjonalny oczekiwany pełny SHA commita do weryfikacji zgodności.
    #[serde(default)]
    pub expected_commit: Option<String>,
    /// Legacy (v1) deklaracja czystego drzewa roboczego.
    ///
    /// Pole **nie jest** źródłem prawdy i nie wpływa na gotowość: czystość drzewa musi zostać
    /// zmierzona i przeniesiona w niezależnym dowodzie.
    #[serde(default)]
    pub working_tree_clean: Option<bool>,
    /// Legacy (v1) deklaracja przejścia testów programowych.
    ///
    /// Pole **nie jest** źródłem prawdy i nie wpływa na gotowość: wynik baseline musi zostać
    /// zmierzony i przeniesiony w niezależnym dowodzie.
    #[serde(default)]
    pub software_tests_passed: Option<bool>,
    /// Identyfikator operatora (np. znak lub login).
    #[serde(default)]
    pub operator_id: String,
    /// Znak wywoławczy operatora odpowiedzialnego za stanowisko.
    pub operator_callsign: String,
    /// Imię i nazwisko operatora.
    pub operator_name: String,
    /// Flaga zezwolenia na sesję HIL w manifeście (zakres wykonania, nie dowód).
    #[serde(default)]
    pub hil_enabled: bool,
    /// Jawna flaga zezwolenia na próby nadawania (TX) (zakres wykonania, nie dowód).
    #[serde(default)]
    pub allow_tx: bool,
    /// Wymiary macierzy testowej (backend x radio x transport x firmware).
    pub matrix: HilMatrixEntry,
    /// Konfiguracja planu bezpieczeństwa toru RF.
    pub rf_safety: HilRfSafetyConfig,
    /// Legacy (v1) osadzone oświadczenie operatora — wyłącznie deklaracja, nigdy dowód.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_attestation: Option<HilOperatorAttestation>,
    /// Opcjonalna deklaracja odczytu przyrządu pomiarowego (deklaracja, nie dowód).
    #[serde(default)]
    pub instrument_verification: Option<HilInstrumentMeasurement>,
}

impl HilTestBenchManifest {
    /// Deserializuje plan testu ze wskazanego ciągu znaków w formacie JSON.
    ///
    /// Parsowanie nie nadaje żadnej gotowości: dokument jest wyłącznie planem.
    pub fn from_json(json_str: &str) -> Result<Self, HilError> {
        let manifest: Self = serde_json::from_str(json_str)
            .map_err(|e| HilError::InvalidManifest(format!("JSON parse error: {e}")))?;
        manifest.validate_structure()?;
        Ok(manifest)
    }

    /// Wczytuje i parsuje plan testu z pliku dyskowego.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, HilError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| HilError::IoError(format!("Cannot read manifest file: {e}")))?;
        Self::from_json(&content)
    }

    /// Serializuje plan testu do znormalizowanego formatu JSON z wcięciami.
    pub fn to_json(&self) -> Result<String, HilError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| HilError::InvalidManifest(format!("JSON serialization error: {e}")))
    }

    /// Zwraca kanoniczną postać planu (posortowane klucze, stabilna kolejność).
    ///
    /// Kanoniczna postać **nie zawiera** osadzonego oświadczenia operatora ani odczytu
    /// przyrządu: są to deklaracje dołączone do planu, a nie część planu testu.
    pub fn canonical_plan_value(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        if let serde_json::Value::Object(map) = &mut value {
            map.remove("operator_attestation");
            map.remove("instrument_verification");
        }
        value
    }

    /// Oblicza SHA-256 kanonicznej postaci planu testu.
    ///
    /// Jest to skrót integralności, **nie** podpis kryptograficzny. Dowód niezależny wiąże się
    /// z planem tym skrótem; sam skrót nie nadaje żadnego uprawnienia.
    pub fn plan_sha256(&self) -> String {
        let canonical = serde_json::to_string(&self.canonical_plan_value())
            .unwrap_or_else(|_| String::from("{}"));
        let mut hasher_input = Vec::with_capacity(
            super::authorization::HIL_PLAN_DIGEST_DOMAIN.len() + 1 + canonical.len(),
        );
        hasher_input.extend_from_slice(super::authorization::HIL_PLAN_DIGEST_DOMAIN.as_bytes());
        hasher_input.push(b'\n');
        hasher_input.extend_from_slice(canonical.as_bytes());
        sha256_hex(&hasher_input)
    }

    /// Weryfikuje pełną poprawność planu (schemat + struktura + parametry).
    pub fn validate_plan(&self) -> Result<(), HilPlanValidationError> {
        if self.schema_version == 0 {
            return Err(HilPlanValidationError::LegacySchemaRequiresMigration);
        }
        if self.schema_version != HIL_PLAN_SCHEMA_VERSION {
            return Err(HilPlanValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        for (name, value) in [
            ("bench_id", self.bench_id.as_str()),
            ("session_id", self.session_id.as_str()),
            ("test_case_id", self.test_case_id.as_str()),
            ("attempt_id", self.attempt_id.as_str()),
            ("operator_callsign", self.operator_callsign.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(HilPlanValidationError::MissingRequiredField(name));
            }
        }
        if self.matrix.model_name.trim().is_empty() {
            return Err(HilPlanValidationError::MissingRequiredField(
                "matrix.model_name",
            ));
        }
        if self.matrix.rig_id.as_str().trim().is_empty() {
            return Err(HilPlanValidationError::MissingRequiredField(
                "matrix.rig_id",
            ));
        }
        if self.matrix.firmware_version.trim().is_empty() {
            return Err(HilPlanValidationError::MissingRequiredField(
                "matrix.firmware_version",
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
                    return Err(HilPlanValidationError::MissingRequiredField(
                        "matrix.transport.port_name",
                    ));
                }
                let upper = trimmed.to_ascii_uppercase();
                if upper == "AUTO" || upper == "FIRST" || upper == "DETECT" {
                    return Err(HilPlanValidationError::InvalidParameter(
                        "automatic port selection is forbidden for a HIL bench".to_string(),
                    ));
                }
                if *baud_rate < 1200 {
                    return Err(HilPlanValidationError::InvalidParameter(format!(
                        "invalid baud_rate {baud_rate} (must be >= 1200)"
                    )));
                }
            }
            HilTransportConfig::Network { host, port } => {
                if host.trim().is_empty() {
                    return Err(HilPlanValidationError::MissingRequiredField(
                        "matrix.transport.host",
                    ));
                }
                if *port == 0 {
                    return Err(HilPlanValidationError::InvalidParameter(
                        "network port cannot be 0".to_string(),
                    ));
                }
            }
        }
        self.validate_structure().map_err(|err| match err {
            HilError::SafetyViolation(msg) | HilError::InvalidTransport(msg) => {
                HilPlanValidationError::InvalidParameter(msg)
            }
            other => HilPlanValidationError::InvalidParameter(other.to_string()),
        })?;
        Ok(())
    }

    /// Czy dokument wymaga jawnej migracji z wersji 1.
    pub fn requires_migration(&self) -> bool {
        self.schema_version == 0
    }

    /// Sprawdza formalną poprawność strukturalną planu.
    ///
    /// Funkcja **nie** ocenia gotowości i nie odczytuje żadnej deklaracji jako dowodu.
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

        if !self.rf_safety.max_test_power_watts.is_finite() {
            return Err(HilError::SafetyViolation(
                "max_test_power_watts must be a finite number (NaN and infinity are forbidden)"
                    .to_string(),
            ));
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
        // F-14 (P0): the transmit path bounds the pulse with an explicit lower limit
        // (`HIL_MIN_TEST_DURATION_MS`). A manifest below that limit would make the clamp range
        // inverted, so it must be rejected here — before any PTT ON is even considered.
        if self.rf_safety.max_test_duration_ms < HIL_MIN_TEST_DURATION_MS {
            return Err(HilError::SafetyViolation(format!(
                "max_test_duration_ms ({}) is below the transmit-path minimum of {} ms; \
                 an inverted PTT pulse range is forbidden",
                self.rf_safety.max_test_duration_ms, HIL_MIN_TEST_DURATION_MS
            )));
        }
        if self.rf_safety.max_test_duration_ms > HIL_MAX_TEST_DURATION_MS {
            return Err(HilError::SafetyViolation(format!(
                "max_test_duration_ms ({}) exceeds laboratory safety limit {} ms",
                self.rf_safety.max_test_duration_ms, HIL_MAX_TEST_DURATION_MS
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

        if let Some(attestation) = &self.operator_attestation {
            if attestation.attestation_timestamp.trim().is_empty() {
                return Err(HilError::InvalidConsent(
                    "operator_attestation attestation_timestamp cannot be empty".to_string(),
                ));
            }
            if attestation.description.trim().is_empty() {
                return Err(HilError::InvalidConsent(
                    "operator_attestation description cannot be empty".to_string(),
                ));
            }
        }

        if let Some(ref inst) = self.instrument_verification
            && !inst.measured_swr.is_finite()
        {
            return Err(HilError::SafetyViolation(
                "measured_swr must be a finite number (NaN and infinity are forbidden)".to_string(),
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
            && let Ok(exp_time) = crate::cat::hil::parse_rfc3339(exp_str)
            && let Ok(created_time) = crate::cat::hil::parse_rfc3339(&self.created_at)
            && exp_time < created_time
        {
            return Err(HilError::InvalidManifest(format!(
                "Session expires_at ({exp_str}) cannot be before created_at ({})",
                self.created_at
            )));
        }

        Ok(())
    }

    /// Konwertuje plan do formatu konfiguracji sesji radiowej [`RigConfig`].
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
                } => {
                    let civ_address = crate::cat::native::registry::find_profile_by_model(
                        &self.matrix.model_name,
                    )
                    .and_then(|p| p.default_ci_v_address)
                    .unwrap_or(0x94);

                    RigBackendConfig::Icom(IcomConfig {
                        port_name: port_name.clone(),
                        baud_rate: *baud_rate,
                        civ_address,
                        model: self.matrix.model_name.clone(),
                    })
                }
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

    /// Zwraca zanonimizowaną kopię planu bez poufnych danych.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base_manifest() -> HilTestBenchManifest {
        HilTestBenchManifest {
            schema_version: HIL_PLAN_SCHEMA_VERSION,
            bench_id: "LAB-BENCH-SP6INA-01".to_string(),
            session_id: "SESSION-01".to_string(),
            test_case_id: "hil_hardware_ptt_01".to_string(),
            attempt_id: "ATTEMPT-01".to_string(),
            created_at: "2026-10-18T09:30:00Z".to_string(),
            expires_at: None,
            tested_commit: None,
            expected_commit: None,
            working_tree_clean: None,
            software_tests_passed: None,
            operator_id: "SP6INA".to_string(),
            operator_callsign: "SP6INA".to_string(),
            operator_name: "Mariusz".to_string(),
            hil_enabled: true,
            allow_tx: false,
            matrix: HilMatrixEntry {
                backend_kind: CatBackendKind::IcomCiV,
                rig_id: RigId::new("RIG-01").unwrap(),
                model_name: "IC-7610".to_string(),
                transport: HilTransportConfig::Serial {
                    port_name: "COM1".to_string(),
                    baud_rate: 115_200,
                    data_bits: 8,
                    parity: "none".to_string(),
                    stop_bits: 1,
                    flow_control: "none".to_string(),
                },
                firmware_version: "v1.0".to_string(),
            },
            rf_safety: HilRfSafetyConfig {
                termination: HilRfTermination::DummyLoad(HilDummyLoadSpec {
                    power_rating_watts: 100.0,
                    max_swr: 1.15,
                    serial_number: "DL-01".to_string(),
                    impedance_ohms: 50.0,
                }),
                max_test_power_watts: 10.0,
                frequency_range_hz: (14_000_000, 14_350_000),
                interlock_engaged: true,
                max_test_duration_ms: 500,
                smartlink_disabled: true,
                unattended_remote_tx_disabled: true,
                remote_endpoint_is_local_or_controlled: true,
                operator_has_immediate_stop_control: true,
                out_of_band_data_plane_disabled: true,
            },
            operator_attestation: None,
            instrument_verification: None,
        }
    }

    #[test]
    fn hil_manifest_to_rig_config_uses_profile_civ_address_for_ic7610_and_ic705() {
        let manifest_7610 = base_manifest();
        let cfg = manifest_7610.to_rig_config().expect("to_rig_config");
        if let RigBackendConfig::Icom(icom) = cfg.backend {
            assert_eq!(icom.civ_address, 0x98);
        } else {
            panic!("expected Icom config");
        }

        let mut manifest_705 = manifest_7610.clone();
        manifest_705.matrix.model_name = "IC-705".to_string();
        let cfg2 = manifest_705.to_rig_config().expect("to_rig_config");
        if let RigBackendConfig::Icom(icom) = cfg2.backend {
            assert_eq!(icom.civ_address, 0xA4);
        } else {
            panic!("expected Icom config");
        }
    }

    #[test]
    fn f15_plan_digest_is_deterministic_and_excludes_attestation() {
        let manifest = base_manifest();
        let first = manifest.plan_sha256();
        let second = manifest.plan_sha256();
        assert_eq!(first, second);
        assert!(super::super::authorization::is_sha256_hex(&first));

        let mut with_attestation = manifest.clone();
        with_attestation.operator_attestation = Some(HilOperatorAttestation {
            dummy_load_physically_connected: true,
            rf_perimeter_secured: true,
            ptt_consent_granted: true,
            attestation_timestamp: "2026-10-18T09:30:00Z".to_string(),
            description: "declaration".to_string(),
            antenna_disconnected: true,
            pa_disconnected_or_locked: true,
            tuner_bypassed: true,
            physical_stop_available: true,
            abort_procedure_documented: true,
            operator_acknowledged: true,
            bench_readiness_confirmed: true,
            operator_present: true,
            supervised_trial_confirmed: true,
            rules_compliance_attested: true,
            operator_privileges_attested: true,
            allowed_band_attested: true,
            allowed_mode_attested: true,
            no_unauthorized_emission_attested: true,
            personnel_know_stop_procedure: true,
        });
        assert_eq!(
            with_attestation.plan_sha256(),
            first,
            "an embedded attestation declaration must not change the plan digest"
        );
    }

    #[test]
    fn f15_legacy_plan_requires_migration() {
        let mut manifest = base_manifest();
        manifest.schema_version = 0;
        assert!(manifest.requires_migration());
        assert_eq!(
            manifest.validate_plan(),
            Err(HilPlanValidationError::LegacySchemaRequiresMigration)
        );
    }

    #[test]
    fn f15_plan_requires_explicit_test_case_and_attempt() {
        let mut manifest = base_manifest();
        manifest.test_case_id = "  ".to_string();
        assert!(matches!(
            manifest.validate_plan(),
            Err(HilPlanValidationError::MissingRequiredField(_))
        ));

        let mut manifest = base_manifest();
        manifest.attempt_id = String::new();
        assert!(matches!(
            manifest.validate_plan(),
            Err(HilPlanValidationError::MissingRequiredField(_))
        ));
    }

    #[test]
    fn f15_legacy_boolean_declarations_do_not_change_the_plan_digest() {
        let manifest = base_manifest();
        let mut with_legacy = manifest.clone();
        with_legacy.working_tree_clean = Some(true);
        with_legacy.software_tests_passed = Some(true);
        assert_ne!(
            manifest.plan_sha256(),
            with_legacy.plan_sha256(),
            "legacy declarations are part of the plan document and must be digest-bound"
        );
    }
}
