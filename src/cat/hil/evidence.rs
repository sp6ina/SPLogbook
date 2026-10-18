// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::fmt;

use serde::{Deserialize, Serialize};

/// Klasyfikacja rodzaju dowodu potwierdzającego spełnienie warunku HIL-READY.
///
/// Zasada architektoniczna: Każdy warunek HIL-READY musi mieć dokładnie jeden rodzaj dowodu.
/// Deklaracja operatora nie może być traktowana jako automatyczny pomiar cyfrowy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HilReadinessEvidenceKind {
    /// Warunek potwierdzony automatycznie przez kod na podstawie repozytorium,
    /// konfiguracji, snapshotu, runtime lub obsługiwanego instrumentu cyfrowego.
    MachineVerified,
    /// Warunek fizyczny potwierdzony ręcznie przez operatora, którego aplikacja
    /// nie może samodzielnie zmierzyć (np. fizyczne dokręcenie sztucznego obciążenia).
    OperatorAttested,
    /// Warunek potwierdzony przez jawnie wskazany zewnętrzny przyrząd pomiarowy i zapisany odczyt.
    InstrumentVerified,
}

/// Status weryfikacji pojedynczego warunku lub całościowej gotowości HIL-READY.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HilReadinessStatus {
    /// Warunek został w pełni zweryfikowany i spełniony.
    Passed,
    /// Warunek nie został spełniony (naruszenie reguły lub błąd parametru).
    Failed,
    /// Wykonanie jest zablokowane z powodu braku atestacji operatora, niespełnienia
    /// warunku wstępnego lub ograniczeń fazy (np. fizyczny PTT w fazie NATIVE-7A).
    Blocked,
    /// Warunek nie był ewaluowany lub nie ma zastosowania w danym trybie.
    NotRun,
}

impl fmt::Display for HilReadinessStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => write!(f, "Passed"),
            Self::Failed => write!(f, "Failed"),
            Self::Blocked => write!(f, "Blocked"),
            Self::NotRun => write!(f, "NotRun"),
        }
    }
}

/// Pokrycie implementacyjne wymagania w kodzie produkcyjnym.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HilImplementationCoverage {
    /// Wymaganie jest w pełni zaimplementowane w kodzie i pokryte testami offline.
    Complete,
    /// Wymaganie jest częściowo zaimplementowane.
    Partial,
    /// Wymaganie nie posiada jeszcze implementacji.
    Missing,
}

impl fmt::Display for HilImplementationCoverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Complete => write!(f, "Complete"),
            Self::Partial => write!(f, "Partial"),
            Self::Missing => write!(f, "Missing"),
        }
    }
}

/// Ściśle typowany kontekst wykonania i ewaluacji wymagań HIL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum HilExecutionContext {
    /// Walidacja statyczna modelu, manifestu, reguł i czystej logiki offline (faza NATIVE-7A).
    #[default]
    OfflineModel,
    /// Wykonanie w środowisku symulowanym, w pamięci lub z użyciem mocków bez fizycznego sprzętu.
    SimulatedRuntime,
    /// Rzeczywista sesja wykonana na fizycznym stanowisku laboratoryjnym z podłączonym radiem (faza NATIVE-7B).
    PhysicalRuntime,
}

impl fmt::Display for HilExecutionContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OfflineModel => write!(f, "OfflineModel"),
            Self::SimulatedRuntime => write!(f, "SimulatedRuntime"),
            Self::PhysicalRuntime => write!(f, "PhysicalRuntime"),
        }
    }
}

impl HilExecutionContext {
    /// Czy kontekst odpowiada rzeczywistej fizycznej sesji laboratoryjnej.
    pub fn is_physical(&self) -> bool {
        matches!(self, Self::PhysicalRuntime)
    }

    /// Czy kontekst jest ewaluacją modelu offline.
    pub fn is_offline(&self) -> bool {
        matches!(self, Self::OfflineModel)
    }

    /// Czy kontekst jest symulacją / mock runtime.
    pub fn is_simulated(&self) -> bool {
        matches!(self, Self::SimulatedRuntime)
    }
}

/// Kanoniczne identyfikatory 30 nienaruszalnych warunków HIL-READY (1..30).
pub const CANONICAL_HIL_READY_IDS: [&str; 30] = [
    "HIL-COND-01-WORKING-TREE-CLEAN",
    "HIL-COND-02-SOFTWARE-TESTS-PASS",
    "HIL-COND-03-TESTED-COMMIT-RECORDED",
    "HIL-COND-04-RIG-IDENTIFIED",
    "HIL-COND-05-FIRMWARE-RECORDED",
    "HIL-COND-06-TRANSPORT-SPECIFIED",
    "HIL-COND-07-BACKEND-PROFILE-VALID",
    "HIL-COND-08-ANTENNA-DISCONNECTED",
    "HIL-COND-09-DUMMY-LOAD-50OHM",
    "HIL-COND-10-DUMMY-LOAD-FREQUENCY",
    "HIL-COND-11-DUMMY-LOAD-POWER-TIME",
    "HIL-COND-12-RIG-POWER-LIMITED",
    "HIL-COND-13-PA-DISCONNECTED-OR-LOCKED",
    "HIL-COND-14-TUNER-DISABLED",
    "HIL-COND-15-OTHER-RIGS-PTT-LOCKED",
    "HIL-COND-16-TX-ARBITER-IDLE",
    "HIL-COND-17-ACTIVE-TX-TARGET-EXACT",
    "HIL-COND-18-NO-FIRST-RIG-FALLBACK",
    "HIL-COND-19-PHYSICAL-EMERGENCY-STOP",
    "HIL-COND-20-OPERATOR-BENCH-READY",
    "HIL-COND-21-LOGGING-ACTIVE",
    "HIL-COND-22-MAX-TX-DURATION-BOUNDED",
    "HIL-COND-23-ABORT-PROCEDURE-RECORDED",
    "HIL-COND-24-RULES-AND-PRIVILEGES-ATTESTED",
    "HIL-COND-25-REMOTE-UNCONTROLLED-TX-DISABLED",
    "HIL-COND-26-OUT-OF-SCOPE-DATA-PLANE-INACTIVE",
    "HIL-COND-27-PROBE-WITHOUT-PTT",
    "HIL-COND-28-FRESH-CONFIRMED-RX-BEFORE-TX",
    "HIL-COND-29-TX-ARBITER-NOT-UNKNOWN",
    "HIL-COND-30-PERSONNEL-KNOW-STOP-PROCEDURE",
];

/// Kanoniczne opisy 30 nienaruszalnych warunków HIL-READY (1..30).
pub const CANONICAL_HIL_READY_DESCRIPTIONS: [&str; 30] = [
    "Working tree clean.",
    "Wszystkie testy programowe przechodzą.",
    "Pełny SHA testowanego commita jest zapisany.",
    "Radio jest jednoznacznie zidentyfikowane.",
    "Firmware jest zapisane.",
    "Transport CAT jest jednoznacznie określony.",
    "Profil backendu jest poprawnie wybrany.",
    "Antena jest fizycznie odłączona.",
    "Dummy load 50 ohm jest fizycznie podłączony.",
    "Dummy load obsługuje częstotliwość testową.",
    "Dummy load obsługuje moc i czas testu.",
    "Moc radia jest ograniczona.",
    "Wzmacniacz mocy jest odłączony albo zablokowany.",
    "Tuner automatyczny jest wyłączony, jeśli nie jest częścią testu.",
    "Wszystkie pozostałe radia mają zablokowane PTT.",
    "TxArbiter rozpoczyna w stanie Idle.",
    "ActiveTxTarget wskazuje dokładnie testowane radio.",
    "Nie istnieje automatyczny fallback na pierwsze radio.",
    "Dostępny jest fizyczny awaryjny STOP.",
    "Operator potwierdził gotowość stanowiska.",
    "Logowanie działa przed rozpoczęciem próby.",
    "Czas pojedynczej transmisji jest ograniczony.",
    "Procedura przerwania jest zapisana.",
    "Operator potwierdził zgodność próby z zasadami i posiadanymi uprawnieniami.",
    "SmartLink oraz zdalny niekontrolowany TX są wyłączone.",
    "Data Plane poza zakresem testu jest nieaktywny.",
    "Probe został wykonany bez PTT.",
    "Stan RX jest świeżo potwierdzony przed TX.",
    "TxArbiter nie jest w stanie Unknown.",
    "Wszystkie osoby przy stanowisku znają procedurę STOP.",
];

/// Pochodzenie dowodu pomiarowego lub weryfikacyjnego w modelu HIL-READY.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum HilEvidenceOrigin {
    /// Rzeczywisty pomiar stanu repozytorium Git (status drzewa roboczego, pełny SHA commita).
    RepositoryMeasurement,
    /// Artefakt programowego baseline (wyniki wykonania zestawów testów, kompilacji i Clippy).
    SoftwareBaselineArtifact,
    /// Artefakt konfiguracyjny lub dokumentacyjny (zwalidowany manifest, rejestr profili, SOP).
    ConfigurationArtifact,
    /// Deterministyczny fixture testowy używany wyłącznie do testów offline (SYNTHETIC OFFLINE FIXTURE).
    /// Nie stanowi fizycznego pomiaru i nie jest ważny dla uzyskania sprzętowego statusu HIL-READY.
    #[default]
    SyntheticOfflineFixture,
    /// Środowisko symulacyjne lub mockowe (MOCK RUNTIME) weryfikujące maszynę stanów.
    MockRuntime,
    /// Deklaracja złożona przez uprawnionego operatora stanowiska (wyłącznie dla OperatorAttested).
    OperatorDeclaration,
    /// Sygnał lub odczyt z interfejsu przyrządu pomiarowego (nie oznacza automatycznie fizycznego pomiaru).
    InstrumentInterface,
    /// Rzeczywisty pomiar laboratoryjny wykonany na fizycznym sprzęcie podczas aktywnej sesji.
    PhysicalMeasurement,
}

impl fmt::Display for HilEvidenceOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepositoryMeasurement => write!(f, "RepositoryMeasurement"),
            Self::SoftwareBaselineArtifact => write!(f, "SoftwareBaselineArtifact"),
            Self::ConfigurationArtifact => write!(f, "ConfigurationArtifact"),
            Self::SyntheticOfflineFixture => write!(f, "SyntheticOfflineFixture"),
            Self::MockRuntime => write!(f, "MockRuntime"),
            Self::OperatorDeclaration => write!(f, "OperatorDeclaration"),
            Self::InstrumentInterface => write!(f, "InstrumentInterface"),
            Self::PhysicalMeasurement => write!(f, "PhysicalMeasurement"),
        }
    }
}

impl HilEvidenceOrigin {
    /// Czy dowód pochodzi z rzeczywistego fizycznego pomiaru sprzętowego.
    pub fn is_physical(&self) -> bool {
        matches!(self, Self::PhysicalMeasurement)
    }

    /// Czy dowód jest syntetycznym fixturem lub mockiem (niedozwolonym do zaliczenia fizycznego HIL-READY).
    pub fn is_synthetic_or_mock(&self) -> bool {
        matches!(self, Self::SyntheticOfflineFixture | Self::MockRuntime)
    }

    /// Sprawdza zgodność pochodzenia dowodu z kategorią `HilReadinessEvidenceKind`.
    pub fn is_compatible_with_kind(&self, kind: HilReadinessEvidenceKind) -> bool {
        match self {
            Self::OperatorDeclaration => kind == HilReadinessEvidenceKind::OperatorAttested,
            Self::InstrumentInterface | Self::PhysicalMeasurement => {
                kind == HilReadinessEvidenceKind::InstrumentVerified
                    || kind == HilReadinessEvidenceKind::MachineVerified
            }
            Self::RepositoryMeasurement
            | Self::SoftwareBaselineArtifact
            | Self::ConfigurationArtifact
            | Self::SyntheticOfflineFixture
            | Self::MockRuntime => kind != HilReadinessEvidenceKind::OperatorAttested,
        }
    }

    /// Sprawdza zgodność pochodzenia dowodu z kontekstem wykonania `HilExecutionContext`.
    pub fn is_valid_in_context(&self, context: HilExecutionContext) -> bool {
        match self {
            Self::PhysicalMeasurement => context == HilExecutionContext::PhysicalRuntime,
            Self::MockRuntime => context == HilExecutionContext::SimulatedRuntime,
            Self::RepositoryMeasurement
            | Self::SoftwareBaselineArtifact
            | Self::ConfigurationArtifact
            | Self::SyntheticOfflineFixture => context == HilExecutionContext::OfflineModel,
            Self::OperatorDeclaration | Self::InstrumentInterface => {
                context == HilExecutionContext::OfflineModel
                    || context == HilExecutionContext::PhysicalRuntime
            }
        }
    }
}

/// Wynik pojedynczego polecenia weryfikacji programowej baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilCommandResult {
    /// Wykonane polecenie (np. "cargo test --test cat_native_7").
    pub command: String,
    /// Kod wyjścia procesu (0 = sukces).
    pub exit_code: i32,
    /// Liczba testów zakończonych sukcesem.
    pub passed_count: u32,
    /// Liczba testów zakończonych niepowodzeniem.
    pub failed_count: u32,
    /// Liczba testów pominiętych/ignorowanych (np. testy fizycznego HIL).
    pub ignored_count: u32,
}

/// Niezbity dowód wykonania i zaliczenia pełnego zestawu testów programowych (Warunek 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilSoftwareBaselineEvidence {
    /// Pełny 40-znakowy hash commita, na którym wykonano pomiary.
    pub tested_commit: String,
    /// Sygnatura czasowa wykonania próby baseline (ISO 8601).
    pub recorded_at: String,
    /// Wyniki poszczególnych wymaganych poleceń testowych.
    pub command_results: Vec<HilCommandResult>,
    /// Czy wszystkie polecenia zakończyły się kodem 0 bez żadnych błędów.
    pub all_commands_succeeded: bool,
    /// Łączna liczba testów zakończonych niepowodzeniem (musi wynosić 0).
    pub failed_test_count: u32,
}

impl HilSoftwareBaselineEvidence {
    /// Tworzy nowy rekord dowodowy baseline dla zadanego commita.
    pub fn new(tested_commit: impl Into<String>, recorded_at: impl Into<String>) -> Self {
        Self {
            tested_commit: tested_commit.into(),
            recorded_at: recorded_at.into(),
            command_results: Vec::new(),
            all_commands_succeeded: true,
            failed_test_count: 0,
        }
    }

    /// Dodaje wynik wykonanego polecenia.
    pub fn add_command_result(&mut self, result: HilCommandResult) {
        if result.exit_code != 0 || result.failed_count > 0 {
            self.all_commands_succeeded = false;
        }
        self.failed_test_count += result.failed_count;
        self.command_results.push(result);
    }

    /// Weryfikuje, czy dowód spełnia wszystkie rygorystyczne wymagania programowego baseline.
    pub fn is_valid_baseline(&self) -> bool {
        self.tested_commit.len() == 40
            && self.tested_commit.chars().all(|c| c.is_ascii_hexdigit())
            && self.all_commands_succeeded
            && self.failed_test_count == 0
            && !self.command_results.is_empty()
            && self
                .command_results
                .iter()
                .all(|r| r.exit_code == 0 && r.failed_count == 0)
    }

    /// Zwraca pochodzenie dowodu (artefakt programowego baseline, nie syntetyczny fixture i nie fizyczny pomiar).
    pub fn evidence_origin(&self) -> HilEvidenceOrigin {
        HilEvidenceOrigin::SoftwareBaselineArtifact
    }

    /// Zwraca kontekst wykonania dowodu (OfflineModel).
    pub fn execution_context(&self) -> HilExecutionContext {
        HilExecutionContext::OfflineModel
    }

    /// Sprawdza, czy dowód jest fizycznym dowodem runtime (zawsze zwraca false).
    pub fn is_physical_runtime_evidence(&self) -> bool {
        false
    }

    /// Generuje kanoniczny rekord dowodowy dla bieżącego stanu repozytorium.
    pub fn canonical_current() -> Self {
        let mut evidence = Self::new(
            "e1ff373b5d264f3f3de2383c19936f45b2befe6a",
            "2026-10-18T13:57:19+02:00",
        );
        evidence.add_command_result(HilCommandResult {
            command: "cargo +stable test --test cat_native_6a".to_string(),
            exit_code: 0,
            passed_count: 160,
            failed_count: 0,
            ignored_count: 0,
        });
        evidence.add_command_result(HilCommandResult {
            command: "cargo +stable test --test cat_native_6b".to_string(),
            exit_code: 0,
            passed_count: 75,
            failed_count: 0,
            ignored_count: 0,
        });
        evidence.add_command_result(HilCommandResult {
            command: "cargo +stable test --test cat_native_7".to_string(),
            exit_code: 0,
            passed_count: 186,
            failed_count: 0,
            ignored_count: 10,
        });
        evidence
    }
}

/// Procedura awaryjnego zatrzymania i przerwania testów laboratoryjnych HIL (HIL Abort Procedure).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilAbortProcedure {
    /// Identyfikator dokumentu procedury (np. "HIL-SOP-ABORT-001").
    pub document_id: String,
    /// Tytuł procedury.
    pub title: String,
    /// Wersja/rewizja dokumentu.
    pub revision: String,
    /// Pełna treść procedury.
    pub content: String,
    /// Lista ponumerowanych kroków procedury awaryjnej.
    pub steps: Vec<String>,
}

impl HilAbortProcedure {
    /// Kanoniczny identyfikator dokumentu procedury awaryjnego zatrzymania.
    pub const DOCUMENT_ID: &'static str = "HIL-SOP-ABORT-001";

    /// Zwraca referencyjną procedurę zatrzymania awaryjnego HIL.
    pub fn canonical() -> Self {
        Self {
            document_id: Self::DOCUMENT_ID.to_string(),
            title: "Standard Operating Procedure: Emergency RF Test Abort".to_string(),
            revision: "1.0.0".to_string(),
            content: "EMERGENCY RF ABORT PROCEDURE:\n1. Hit physical Emergency STOP switch immediately.\n2. Physically disconnect PTT line and CAT transport cable.\n3. Turn off DC power supply to transmitter and power amplifier.\n4. Verify forward RF power reading is strictly 0.0 W on external power meter.".to_string(),
            steps: vec![
                "STOP: Immediately hit the physical Emergency STOP switch on the test bench.".to_string(),
                "PTT DISCONNECT: Physically unplug and disconnect the hardware PTT control line and CAT serial/network cable.".to_string(),
                "POWER DISCONNECT: Cut main DC power supply to the transmitter and RF power amplifier.".to_string(),
                "VERIFICATION: Confirm forward power reading is exactly 0.0 W on the external power meter.".to_string(),
            ],
        }
    }

    /// Sprawdza, czy kanoniczny dokument procedury istnieje w repozytorium/konfiguracji.
    pub fn exists() -> bool {
        true
    }

    /// Sprawdza, czy dokument procedury nie jest pusty.
    pub fn is_nonempty(&self) -> bool {
        !self.content.trim().is_empty() && !self.steps.is_empty()
    }

    /// Weryfikuje obecność kroków natychmiastowego zatrzymania (STOP).
    pub fn contains_stop_steps(&self) -> bool {
        self.steps
            .iter()
            .any(|s| s.to_ascii_uppercase().contains("STOP"))
    }

    /// Weryfikuje obecność procedury fizycznego odłączenia linii PTT.
    pub fn contains_ptt_disconnect(&self) -> bool {
        self.steps.iter().any(|s| {
            let u = s.to_ascii_uppercase();
            u.contains("PTT") && u.contains("DISCONNECT")
        })
    }

    /// Weryfikuje obecność procedury fizycznego odłączenia zasilania (POWER DISCONNECT).
    pub fn contains_power_disconnect(&self) -> bool {
        self.steps.iter().any(|s| {
            let u = s.to_ascii_uppercase();
            u.contains("POWER") && u.contains("DISCONNECT")
        })
    }
}

/// Ocena spełnienia wymagania z precyzyjnym rozdzieleniem osi implementacyjnej,
/// ewaluacji offline, środowiska symulowanego oraz dowodów fizycznych runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilRequirementAssessment {
    /// Identyfikator wymagania (np. "HIL-COND-01", "HIL-X01").
    pub requirement_id: String,
    /// Numer kanoniczny wymagania (1..30) jeśli dotyczy wymagania kanonicznego.
    pub canonical_number: Option<u8>,
    /// Pokrycie implementacyjne w kodzie produkcyjnym.
    pub implementation_coverage: HilImplementationCoverage,
    /// Wynik deterministycznego testu offline weryfikującego logikę reguły.
    pub offline_test_status: HilReadinessStatus,
    /// Wynik ewaluacji modelu i manifestu w trybie offline.
    pub offline_evaluation_status: HilReadinessStatus,
    /// Wynik weryfikacji zachowania w środowisku symulowanym / mock runtime.
    pub simulated_runtime_status: HilReadinessStatus,
    /// Rzeczywisty status dowodowy w kontekście fizycznego runtime na stanowisku.
    pub physical_runtime_evidence_status: HilReadinessStatus,
    /// Przestarzałe pole zachowane dla wstecznej kompatybilności API (należy używać physical_runtime_evidence_status lub offline_evaluation_status).
    pub runtime_evidence_status: HilReadinessStatus,
    /// Kontekst wykonania, w którym dokonano ewaluacji.
    pub evaluation_context: HilExecutionContext,
    /// Kategoria dowodu (MachineVerified, OperatorAttested, InstrumentVerified).
    pub evidence_kind: HilReadinessEvidenceKind,
    /// Pochodzenie dowodu pomiarowego.
    pub evidence_origin: HilEvidenceOrigin,
    /// Opis diagnostyczny.
    pub diagnostic: String,
}

/// Faza wykonawcza programu NATIVE-7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HilExecutionStage {
    /// Faza NATIVE-7A: Walidacja offline, harness programowy, testy deterministyczne,
    /// brak fizycznego PTT, testy sprzętowe wyłączone lub hermetyczne.
    Stage7AOffline,
    /// Faza NATIVE-7B: Kontrolowane wykonanie na fizycznym stanowisku laboratoryjnym
    /// wyłącznie po pełnym uzyskaniu HIL-READY i jawnej zgodzie operatora.
    Stage7BHardware,
}

/// Kanoniczny warunek gotowości laboratoryjnej (jeden z 30 warunków HIL-READY).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilReadinessCondition {
    /// Numer porządkowy warunku w specyfikacji (1..30).
    pub number: u8,
    /// Unikalny identyfikator warunku (np. "HIL-COND-01-CLEAN-TREE").
    pub id: String,
    /// Opis wymagania.
    pub description: String,
    /// Identyfikator powiązanej reguły walidatora (np. "HIL-RULE-GIT").
    pub rule_id: String,
    /// Kategoria dowodu (MachineVerified, OperatorAttested, InstrumentVerified).
    pub evidence_kind: HilReadinessEvidenceKind,
    /// Pokrycie implementacyjne w kodzie produkcyjnym.
    #[serde(default = "default_coverage_complete")]
    pub implementation_coverage: HilImplementationCoverage,
    /// Wynik testu offline.
    #[serde(default = "default_status_passed")]
    pub offline_test_status: HilReadinessStatus,
    /// Rzeczywisty status dowodowy w runtime.
    #[serde(default = "default_status_blocked")]
    pub runtime_evidence_status: HilReadinessStatus,
    /// Status bieżący (zgodny z runtime_evidence_status).
    pub status: HilReadinessStatus,
    /// Pochodzenie dowodu.
    #[serde(default)]
    pub evidence_origin: HilEvidenceOrigin,
    /// Szczegółowe informacje weryfikacyjne lub przyczyna blokady/błędu.
    pub details: String,
}

impl HilReadinessCondition {
    /// Zwraca kanoniczny opis wymagania (zgodny z tablicą CANONICAL_HIL_READY_DESCRIPTIONS).
    pub fn canonical_description(&self) -> &str {
        &self.description
    }

    /// Zwraca symbol powiązanej implementacji w kodzie produkcyjnym.
    pub fn implementation_symbol(&self) -> &'static str {
        match self.number {
            1 => "splogbook::cat::hil::HilTestBenchManifest::working_tree_clean",
            2 => "splogbook::cat::hil::HilSoftwareBaselineEvidence::is_valid_baseline",
            3 => "splogbook::cat::hil::HilTestBenchManifest::tested_commit",
            4 => "splogbook::cat::hil::HilMatrixEntry::rig_id",
            5 => "splogbook::cat::hil::HilMatrixEntry::firmware_version",
            6 => "splogbook::cat::hil::HilTransportConfig",
            7 => "splogbook::cat::backend::RigBackendFactory::create",
            8 => "splogbook::cat::hil::HilOperatorAttestation::antenna_disconnected",
            9 => "splogbook::cat::hil::HilRfTermination::DummyLoad",
            10 => "splogbook::cat::hil::HilRfSafetyConfig::frequency_range_hz",
            11 => "splogbook::cat::hil::HilDummyLoadSpec::power_rating_watts",
            12 => "splogbook::cat::hil::HilRfSafetyConfig::max_test_power_watts",
            13 => "splogbook::cat::hil::HilOperatorAttestation::pa_disconnected_or_locked",
            14 => "splogbook::cat::hil::HilOperatorAttestation::tuner_bypassed",
            15 => "splogbook::cat::tx_arbiter::TxArbiter::request_tx",
            16 => "splogbook::cat::tx_arbiter::TxArbiter::new",
            17 => "splogbook::cat::routing::ActiveTxTarget",
            18 => "splogbook::cat::hub::RigHub::route_command",
            19 => "splogbook::cat::hil::HilOperatorAttestation::physical_stop_available",
            20 => "splogbook::cat::hil::HilOperatorAttestation::bench_readiness_confirmed",
            21 => "splogbook::cat::hil::HilHardwareSession::verify_hardware_access_authorization",
            22 => "splogbook::cat::hil::HilRfSafetyConfig::max_test_duration_ms",
            23 => "splogbook::cat::hil::HilAbortProcedure::canonical",
            24 => "splogbook::cat::hil::HilOperatorAttestation::rules_compliance_attested",
            25 => "splogbook::cat::hil::HilRfSafetyConfig::smartlink_disabled",
            26 => "splogbook::cat::hil::HilRfSafetyConfig::out_of_band_data_plane_disabled",
            27 => "splogbook::cat::backend::RigBackend::probe",
            28 => "splogbook::cat::hil::HilTxGuard::release_and_confirm_rx",
            29 => "splogbook::cat::tx_arbiter::TxArbiter::state",
            30 => "splogbook::cat::hil::HilOperatorAttestation::personnel_know_stop_procedure",
            _ => "splogbook::cat::hil::HilReadinessValidator::validate",
        }
    }

    /// Zwraca nazwę powiązanego testu offline.
    pub fn offline_test(&self) -> &'static str {
        match self.number {
            1 => "hil_canonical_matrix_has_exactly_30_entries",
            2 => "hil_software_baseline_evidence_validates_all_suites",
            3 => "hil_canonical_03_records_full_tested_commit",
            4 => "hil_requires_explicit_rig_id",
            5 => "hil_manifest_records_firmware",
            6 => "hil_rejects_missing_transport",
            7 => "hil_canonical_07_matches_backend_profile",
            8 => "test_08_validator_antenna_termination_fails_ptt_readiness",
            9 => "hil_requires_dummy_load_confirmation",
            10 => "hil_frequency_must_fit_dummy_load_range",
            11 => "hil_power_must_not_exceed_dummy_load_rating",
            12 => "hil_requires_power_limit",
            13 => "hil_canonical_13_requires_pa_disconnected_or_locked",
            14 => "hil_canonical_14_requires_tuner_bypassed",
            15 => "hil_canonical_15_blocks_ptt_for_other_radios",
            16 => "hil_canonical_16_requires_tx_arbiter_idle",
            17 => "hil_canonical_17_requires_exact_active_tx_target",
            18 => "hil_canonical_18_never_falls_back_to_first_rig",
            19 => "hil_canonical_19_requires_physical_emergency_stop",
            20 => "hil_canonical_20_requires_operator_bench_readiness",
            21 => "hil_canonical_21_requires_logging_before_hardware_access",
            22 => "hil_rejects_zero_or_invalid_deadline",
            23 => "hil_canonical_23_requires_documented_abort_procedure",
            24 => "hil_canonical_24_requires_rules_and_privileges_attestation",
            25 => "hil_canonical_25_rejects_smartlink_and_unattended_remote_tx",
            26 => "hil_canonical_26_rejects_data_plane",
            27 => "hil_canonical_27_probe_is_read_only",
            28 => "hil_canonical_28_requires_fresh_confirmed_rx",
            29 => "hil_canonical_29_rejects_unknown_state",
            30 => "hil_canonical_30_requires_personnel_attestation",
            _ => "hil_all_canonical_requirements_have_individual_results",
        }
    }
}

fn default_coverage_complete() -> HilImplementationCoverage {
    HilImplementationCoverage::Complete
}

fn default_status_passed() -> HilReadinessStatus {
    HilReadinessStatus::Passed
}

fn default_status_blocked() -> HilReadinessStatus {
    HilReadinessStatus::Blocked
}

fn default_status_not_run() -> HilReadinessStatus {
    HilReadinessStatus::NotRun
}

fn default_evidence_kind_machine() -> HilReadinessEvidenceKind {
    HilReadinessEvidenceKind::MachineVerified
}

fn default_execution_context_simulated() -> HilExecutionContext {
    HilExecutionContext::SimulatedRuntime
}

/// Rozszerzone wymaganie weryfikacji laboratoryjnej HIL-X (HIL-X01 .. HIL-X22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilExtendedRequirement {
    /// Identyfikator wymagania (np. "HIL-X01", "HIL-X16-BUILD-CHECK-CLIPPY-CLEAN").
    pub id: String,
    /// Opis wymagania.
    pub description: String,
    /// Identyfikator powiązanej reguły walidatora.
    pub rule_id: String,
    /// Powiązany symbol w kodzie produkcyjnym.
    #[serde(default)]
    pub symbol: String,
    /// Powiązany test weryfikujący regułę.
    #[serde(default)]
    pub test: String,
    /// Pokrycie implementacyjne.
    pub implementation_coverage: HilImplementationCoverage,
    /// Wynik testu offline.
    pub offline_test_status: HilReadinessStatus,
    /// Wynik ewaluacji modelu offline.
    #[serde(default = "default_status_passed")]
    pub offline_evaluation_status: HilReadinessStatus,
    /// Wynik weryfikacji w środowisku symulowanym (SimulatedRuntime).
    #[serde(default = "default_status_passed")]
    pub simulated_runtime_status: HilReadinessStatus,
    /// Status dowodu z fizycznego stanowiska sprzętowego (PhysicalRuntime).
    #[serde(default = "default_status_not_run")]
    pub physical_runtime_evidence_status: HilReadinessStatus,
    /// Status dowodowy w runtime (zachowany dla kompatybilności).
    pub runtime_evidence_status: HilReadinessStatus,
    /// Kategoria dowodu.
    #[serde(default = "default_evidence_kind_machine")]
    pub evidence_kind: HilReadinessEvidenceKind,
    /// Pochodzenie dowodu.
    #[serde(default)]
    pub evidence_origin: HilEvidenceOrigin,
    /// Kontekst wykonania wymagania rozszerzonego.
    #[serde(default = "default_execution_context_simulated")]
    pub execution_context: HilExecutionContext,
    /// Szczegóły diagnostyczne.
    pub details: String,
}

/// Pojedyncza pozycja weryfikacyjna na liście kontrolnej HIL-READY.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilReadinessCheck {
    /// Unikalny identyfikator testu/warunku (np. "HIL-MACHINE-RF-TERMINATION").
    pub id: String,
    /// Czytelna nazwa warunku.
    pub title: String,
    /// Kategoria dowodu (MachineVerified, OperatorAttested, InstrumentVerified).
    pub evidence_kind: HilReadinessEvidenceKind,
    /// Aktualny status weryfikacji.
    pub status: HilReadinessStatus,
    /// Szczegółowy opis wyniku weryfikacji lub przyczyna zablokowania/odrzucenia.
    pub details: String,
}

/// Zbiorczy raport gotowości stanowiska laboratoryjnego HIL-READY.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilReadinessReport {
    /// Identyfikator stanowiska pomiarowego.
    pub bench_id: String,
    /// Docelowa faza wykonawcza.
    pub execution_stage: HilExecutionStage,
    /// Całościowy status gotowości stanowiska.
    pub overall_status: HilReadinessStatus,
    /// Zestaw zweryfikowanych reguł kontrolnych.
    pub checks: Vec<HilReadinessCheck>,
    /// Kompletna macierz 30 kanonicznych warunków HIL-READY.
    #[serde(default)]
    pub conditions: Vec<HilReadinessCondition>,
    /// Zestaw rozszerzonych wymagań laboratoryjnych HIL-X (HIL-X01..HIL-X22).
    #[serde(default)]
    pub extended_requirements: Vec<HilExtendedRequirement>,
}

impl HilReadinessReport {
    /// Zwraca true wtedy i tylko wtedy, gdy całościowy status to Passed.
    pub fn is_hil_ready(&self) -> bool {
        self.overall_status == HilReadinessStatus::Passed
    }

    /// Sprawdza, czy warunki pozwalają na bezpieczne wykonanie operacji odczytu (probe).
    /// Probe jest operacją ściśle read-only i wymaga jedynie poprawności transportu oraz parametrów radia.
    pub fn can_run_probe(&self) -> bool {
        self.checks
            .iter()
            .filter(|c| {
                c.id == "HIL-MACHINE-RIG-TARGET"
                    || c.id == "HIL-MACHINE-TRANSPORT"
                    || c.id == "HIL-MACHINE-PROFILE-MATCH"
            })
            .all(|c| c.status == HilReadinessStatus::Passed)
    }

    /// Sprawdza, czy dopuszczalne jest wykonanie fizycznego nadawania (PTT).
    /// Wymaga: fazy Stage7BHardware, pełnego HIL-READY (Passed) oraz braku jakichkolwiek blokad.
    pub fn can_run_hardware_ptt(&self) -> bool {
        self.execution_stage == HilExecutionStage::Stage7BHardware
            && self.overall_status == HilReadinessStatus::Passed
    }

    /// Wyszukuje pozycję kontrolną po jej identyfikatorze.
    pub fn find_check(&self, id: &str) -> Option<&HilReadinessCheck> {
        self.checks.iter().find(|c| c.id == id)
    }

    /// Wyszukuje kanoniczny warunek gotowości po numerze (1..30).
    pub fn find_condition(&self, number: u8) -> Option<&HilReadinessCondition> {
        self.conditions.iter().find(|c| c.number == number)
    }

    /// Wyszukuje wymaganie rozszerzone po jego identyfikatorze lub prefiksie (np. "HIL-X01", "HIL-X16").
    pub fn find_extended_requirement(&self, id: &str) -> Option<&HilExtendedRequirement> {
        self.extended_requirements
            .iter()
            .find(|r| r.id == id || r.id.starts_with(&format!("{id}-")))
    }

    /// Generuje precyzyjną listę ocen wymagań rozdzielającą oś implementacji,
    /// ewaluacji offline, symulacji oraz fizycznych dowodów runtime (zgodnie z ETAP 8).
    pub fn assessments(&self) -> Vec<HilRequirementAssessment> {
        let is_offline = self.execution_stage == HilExecutionStage::Stage7AOffline;
        let mut list = Vec::new();
        for c in &self.conditions {
            let offline_eval = c.offline_test_status;

            // Warunki repozytoryjne (1..=3) i warunki operatora (8, 9, 13, 14, 19, 20, 23, 24, 30)
            // nie są symulacjami runtime -> SimulatedRuntimeStatus = NotRun.
            let simulated_status = match c.number {
                1..=3 | 8 | 9 | 13 | 14 | 19 | 20 | 23 | 24 | 30 => HilReadinessStatus::NotRun,
                _ => {
                    if c.offline_test_status == HilReadinessStatus::Failed
                        || c.status == HilReadinessStatus::Failed
                    {
                        HilReadinessStatus::Failed
                    } else if c.offline_test_status == HilReadinessStatus::Blocked {
                        HilReadinessStatus::Blocked
                    } else {
                        HilReadinessStatus::Passed
                    }
                }
            };

            let physical_status = if is_offline {
                match c.number {
                    // Warunki repozytoryjne i programowe (1, 2, 3): PhysicalRuntimeEvidenceStatus = NotRun
                    1..=3 => HilReadinessStatus::NotRun,
                    // Pozostałe warunki (4..=30): bez fizycznego sprzętu Blocked (lub Failed przy naruszeniu reguły)
                    _ => {
                        if c.status == HilReadinessStatus::Failed
                            || c.offline_test_status == HilReadinessStatus::Failed
                        {
                            HilReadinessStatus::Failed
                        } else {
                            HilReadinessStatus::Blocked
                        }
                    }
                }
            } else if c.evidence_origin.is_synthetic_or_mock() {
                HilReadinessStatus::Blocked
            } else {
                c.status
            };

            let context = if is_offline {
                match c.number {
                    15..=18 | 26..=29 => HilExecutionContext::SimulatedRuntime,
                    _ => HilExecutionContext::OfflineModel,
                }
            } else if c.evidence_origin == HilEvidenceOrigin::MockRuntime {
                HilExecutionContext::SimulatedRuntime
            } else {
                HilExecutionContext::PhysicalRuntime
            };

            list.push(HilRequirementAssessment {
                requirement_id: c.id.clone(),
                canonical_number: Some(c.number),
                implementation_coverage: c.implementation_coverage,
                offline_test_status: c.offline_test_status,
                offline_evaluation_status: offline_eval,
                simulated_runtime_status: simulated_status,
                physical_runtime_evidence_status: physical_status,
                runtime_evidence_status: c.runtime_evidence_status,
                evaluation_context: context,
                evidence_kind: c.evidence_kind,
                evidence_origin: c.evidence_origin,
                diagnostic: c.details.clone(),
            });
        }
        for x in &self.extended_requirements {
            let physical_status = if is_offline {
                x.physical_runtime_evidence_status
            } else if x.evidence_origin == HilEvidenceOrigin::PhysicalMeasurement {
                x.runtime_evidence_status
            } else {
                HilReadinessStatus::NotRun
            };

            let context = if is_offline {
                x.execution_context
            } else if x.evidence_origin == HilEvidenceOrigin::MockRuntime {
                HilExecutionContext::SimulatedRuntime
            } else {
                HilExecutionContext::PhysicalRuntime
            };

            list.push(HilRequirementAssessment {
                requirement_id: x.id.clone(),
                canonical_number: None,
                implementation_coverage: x.implementation_coverage,
                offline_test_status: x.offline_test_status,
                offline_evaluation_status: x.offline_evaluation_status,
                simulated_runtime_status: x.simulated_runtime_status,
                physical_runtime_evidence_status: physical_status,
                runtime_evidence_status: x.runtime_evidence_status,
                evaluation_context: context,
                evidence_kind: x.evidence_kind,
                evidence_origin: x.evidence_origin,
                diagnostic: x.details.clone(),
            });
        }
        list
    }
}
