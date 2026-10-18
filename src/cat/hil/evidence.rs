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

/// Pochodzenie dowodu pomiarowego (fizyczny pomiar vs syntetyczny fixture testowy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum HilEvidenceOrigin {
    /// Rzeczywisty pomiar laboratoryjny wykonany na fizycznym sprzęcie podczas aktywnej sesji.
    PhysicalMeasurement,
    /// Deterministyczny fixture testowy używany wyłącznie do testów offline (SYNTHETIC OFFLINE FIXTURE).
    /// Nie stanowi fizycznego pomiaru i nie jest ważny dla uzyskania sprzętowego statusu HIL-READY.
    #[default]
    SyntheticOfflineFixture,
}

impl fmt::Display for HilEvidenceOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicalMeasurement => write!(f, "PhysicalMeasurement"),
            Self::SyntheticOfflineFixture => write!(f, "SyntheticOfflineFixture"),
        }
    }
}

/// Ocena spełnienia wymagania z precyzyjnym rozdzieleniem osi implementacyjnej od runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilRequirementAssessment {
    /// Identyfikator wymagania (np. "HIL-COND-01", "HIL-X01").
    pub requirement_id: String,
    /// Pokrycie implementacyjne w kodzie produkcyjnym.
    pub implementation_coverage: HilImplementationCoverage,
    /// Wynik deterministycznego testu offline weryfikującego logikę reguły.
    pub offline_test_status: HilReadinessStatus,
    /// Rzeczywisty status dowodowy w runtime (np. Blocked przy braku fizycznego sprzętu).
    pub runtime_evidence_status: HilReadinessStatus,
    /// Kategoria dowodu.
    pub evidence_kind: HilReadinessEvidenceKind,
    /// Pochodzenie dowodu.
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

fn default_coverage_complete() -> HilImplementationCoverage {
    HilImplementationCoverage::Complete
}

fn default_status_passed() -> HilReadinessStatus {
    HilReadinessStatus::Passed
}

fn default_status_blocked() -> HilReadinessStatus {
    HilReadinessStatus::Blocked
}

/// Rozszerzone wymaganie weryfikacji laboratoryjnej HIL-X (HIL-X01 .. HIL-X15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HilExtendedRequirement {
    /// Identyfikator wymagania (np. "HIL-X01").
    pub id: String,
    /// Opis wymagania.
    pub description: String,
    /// Identyfikator powiązanej reguły walidatora.
    pub rule_id: String,
    /// Pokrycie implementacyjne.
    pub implementation_coverage: HilImplementationCoverage,
    /// Wynik testu offline.
    pub offline_test_status: HilReadinessStatus,
    /// Status dowodowy w runtime.
    pub runtime_evidence_status: HilReadinessStatus,
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
    /// Zestaw rozszerzonych wymagań laboratoryjnych HIL-X (HIL-X01..HIL-X15).
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

    /// Wyszukuje wymaganie rozszerzone po jego identyfikatorze (np. "HIL-X01").
    pub fn find_extended_requirement(&self, id: &str) -> Option<&HilExtendedRequirement> {
        self.extended_requirements.iter().find(|r| r.id == id)
    }

    /// Generuje precyzyjną listę ocen wymagań rozdzielającą oś implementacji od runtime.
    pub fn assessments(&self) -> Vec<HilRequirementAssessment> {
        let mut list = Vec::new();
        for c in &self.conditions {
            list.push(HilRequirementAssessment {
                requirement_id: c.id.clone(),
                implementation_coverage: c.implementation_coverage,
                offline_test_status: c.offline_test_status,
                runtime_evidence_status: c.runtime_evidence_status,
                evidence_kind: c.evidence_kind,
                evidence_origin: c.evidence_origin,
                diagnostic: c.details.clone(),
            });
        }
        for x in &self.extended_requirements {
            list.push(HilRequirementAssessment {
                requirement_id: x.id.clone(),
                implementation_coverage: x.implementation_coverage,
                offline_test_status: x.offline_test_status,
                runtime_evidence_status: x.runtime_evidence_status,
                evidence_kind: HilReadinessEvidenceKind::MachineVerified,
                evidence_origin: HilEvidenceOrigin::SyntheticOfflineFixture,
                diagnostic: x.details.clone(),
            });
        }
        list
    }
}
