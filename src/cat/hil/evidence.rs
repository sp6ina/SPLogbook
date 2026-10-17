// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

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
    /// Zestaw wszystkich zweryfikowanych warunków.
    pub checks: Vec<HilReadinessCheck>,
}

impl HilReadinessReport {
    /// Zwraca true wtedy i tylko wtedy, gdy wszystkie warunki mają status Passed.
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
}
