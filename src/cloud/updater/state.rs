// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Kompletny 20-stanowy deterministyczny automat stanów aktualizatora SPLogbook (M3-A)

use crate::cloud::updater::UpdaterError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Identyfikator stanu procesu aktualizacji (dokładnie 20 stanów wg specyfikacji DESIGN 3.0 C).
///
/// Podział kanoniczny:
/// - 14 stanów operacyjnych przepływu podstawowego,
/// - 1 końcowy stan sukcesu (`Committed`),
/// - 5 stanów błędów, rollbacku i recovery (`FailedPreApply`, `FailedPostApply`, `Rollback`, `RollbackVerified`, `Cleanup`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpdateStateId {
    /// 1. Stan spoczynku — brak aktywnej transakcji aktualizacji.
    Idle,
    /// 2. Sprawdzanie dostępności nowego wydania (pobieranie metadanych GitHub / API).
    CheckingRelease,
    /// 3. Pobieranie podpisanego manifestu wydań (release-manifest.json i sygnatury .sig).
    FetchingManifest,
    /// 4. Matematyczna weryfikacja podpisu Ed25519 manifestu i listy zaufanych kluczy.
    VerifyingManifest,
    /// 5. Dobór jednoznacznego assetu instalacyjnego dla bieżącej architektury i platformy.
    SelectingAsset,
    /// 6. Pobieranie zweryfikowanego strumienia danych instalatora/archiwum.
    Downloading,
    /// 7. Weryfikacja integralności pobranego pliku (rozmiar i suma kontrolna SHA-256).
    VerifyingDownload,
    /// 8. Inspekcja archiwum (ochrona przed Zip-Slip, bombami dekompresyjnymi i nodami specjalnymi).
    InspectingPackage,
    /// 9. Rozpakowanie zweryfikowanego pakietu do izolowanego katalogu stagingu.
    Staging,
    /// 10. Budowa i walidacja bezpieczeństwa planu aktualizacji (UpdatePlan).
    BuildingUpdatePlan,
    /// 11. Oczekiwanie na decyzję użytkownika / instalację (ostatni stan z prawem do anulowania).
    AwaitingApply,
    /// 12. Rozpoczęcie fizycznej podmiany plików programu (bezwzględny zakaz anulowania od tego stanu!).
    Applying,
    /// 13. Uruchomienie procesu potomnego kandydata z flagą --check-health-startup.
    LaunchingCandidate,
    /// 14. Oczekiwanie na dowód zdrowia (.health_ok powiązany z attempt_id) w oknie czasowym 15s.
    AwaitingHealth,
    /// 15. Końcowy stan sukcesu (Terminal Success) — wersja zatwierdzona i stabilna.
    Committed,
    /// 16. Błąd przed wejściem w Applying — nie wymaga wycofywania plików programu, wyłącznie czyszczenia.
    FailedPreApply,
    /// 17. Błąd w trakcie lub po podmianie plików programu — bezwzględnie wymaga rollbacku lub recovery.
    FailedPostApply,
    /// 18. Faza przywracania oryginalnych plików programu z katalogu kopii zapasowej (.rollback).
    Rollback,
    /// 19. Rollback pomyślnie potwierdzony i zweryfikowany bajtowo.
    RollbackVerified,
    /// 20. Sprzątanie tymczasowych zasobów transakcji, stagingu i blokady concurrency.
    Cleanup,
}

impl UpdateStateId {
    /// Zwraca czytelną dla człowieka nazwę stanu.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::CheckingRelease => "CheckingRelease",
            Self::FetchingManifest => "FetchingManifest",
            Self::VerifyingManifest => "VerifyingManifest",
            Self::SelectingAsset => "SelectingAsset",
            Self::Downloading => "Downloading",
            Self::VerifyingDownload => "VerifyingDownload",
            Self::InspectingPackage => "InspectingPackage",
            Self::Staging => "Staging",
            Self::BuildingUpdatePlan => "BuildingUpdatePlan",
            Self::AwaitingApply => "AwaitingApply",
            Self::Applying => "Applying",
            Self::LaunchingCandidate => "LaunchingCandidate",
            Self::AwaitingHealth => "AwaitingHealth",
            Self::Committed => "Committed",
            Self::FailedPreApply => "FailedPreApply",
            Self::FailedPostApply => "FailedPostApply",
            Self::Rollback => "Rollback",
            Self::RollbackVerified => "RollbackVerified",
            Self::Cleanup => "Cleanup",
        }
    }

    /// Określa, czy w danym stanie użytkownik może bezpiecznie anulować transakcję.
    /// Zgodnie z decyzją OWNER-M2-13, anulowanie od wejścia w `Applying` jest bezwzględnie zabronione.
    pub fn is_cancellable(&self) -> bool {
        matches!(
            self,
            Self::Idle
                | Self::CheckingRelease
                | Self::FetchingManifest
                | Self::VerifyingManifest
                | Self::SelectingAsset
                | Self::Downloading
                | Self::VerifyingDownload
                | Self::InspectingPackage
                | Self::Staging
                | Self::BuildingUpdatePlan
                | Self::AwaitingApply
        )
    }

    /// Czy stan należy do fazy po rozpoczęciu podmiany plików programu.
    pub fn is_post_apply(&self) -> bool {
        matches!(
            self,
            Self::Applying
                | Self::LaunchingCandidate
                | Self::AwaitingHealth
                | Self::Committed
                | Self::FailedPostApply
                | Self::Rollback
                | Self::RollbackVerified
        )
    }

    /// Czy stan wymaga wycofania zmian w plikach programu (Rollback).
    pub fn requires_rollback(&self) -> bool {
        matches!(self, Self::FailedPostApply | Self::Rollback)
    }

    /// Czy stan oznacza terminalny sukces transakcji.
    pub fn is_terminal_success(&self) -> bool {
        matches!(self, Self::Committed)
    }
}

impl std::fmt::Display for UpdateStateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Deterministyczny automat stanów transakcji aktualizacji egzekwujący dozwolone przejścia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateStateMachine {
    current_state: UpdateStateId,
    history: Vec<(UpdateStateId, DateTime<Utc>)>,
}

impl Default for UpdateStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateStateMachine {
    /// Tworzy nowy automat stanów rozpoczynający w stanie `Idle`.
    pub fn new() -> Self {
        let initial = UpdateStateId::Idle;
        Self {
            current_state: initial,
            history: vec![(initial, Utc::now())],
        }
    }

    /// Zwraca bieżący stan automatu.
    pub fn current_state(&self) -> UpdateStateId {
        self.current_state
    }

    /// Zwraca pełną audytową historię przejść stanów z sygnaturami czasowymi UTC.
    pub fn history(&self) -> &[(UpdateStateId, DateTime<Utc>)] {
        &self.history
    }

    /// Sprawdza, czy przejście z bieżącego stanu do docelowego jest dozwolone przez reguły automatu.
    pub fn can_transition_to(&self, target: UpdateStateId) -> bool {
        Self::is_valid_transition(self.current_state, target)
    }

    /// Dokonuje zwalidowanego przejścia do stanu docelowego lub zwraca `UpdaterError::InvalidTransition`.
    pub fn transition_to(&mut self, target: UpdateStateId) -> Result<(), UpdaterError> {
        if !self.can_transition_to(target) {
            return Err(UpdaterError::InvalidTransition(format!(
                "Niedozwolone przejście ze stanu '{}' do '{}'",
                self.current_state, target
            )));
        }
        self.current_state = target;
        self.history.push((target, Utc::now()));
        Ok(())
    }

    /// Sprawdza poprawność przejścia między dwoma dowolnymi stanami wg reguł DESIGN 3.0 C.
    pub fn is_valid_transition(from: UpdateStateId, to: UpdateStateId) -> bool {
        match from {
            UpdateStateId::Idle => matches!(to, UpdateStateId::CheckingRelease),
            UpdateStateId::CheckingRelease => matches!(
                to,
                UpdateStateId::FetchingManifest
                    | UpdateStateId::Idle
                    | UpdateStateId::FailedPreApply
            ),
            UpdateStateId::FetchingManifest => {
                matches!(
                    to,
                    UpdateStateId::VerifyingManifest | UpdateStateId::FailedPreApply
                )
            }
            UpdateStateId::VerifyingManifest => {
                matches!(
                    to,
                    UpdateStateId::SelectingAsset | UpdateStateId::FailedPreApply
                )
            }
            UpdateStateId::SelectingAsset => {
                matches!(
                    to,
                    UpdateStateId::Downloading | UpdateStateId::FailedPreApply
                )
            }
            UpdateStateId::Downloading => {
                matches!(
                    to,
                    UpdateStateId::VerifyingDownload | UpdateStateId::FailedPreApply
                )
            }
            UpdateStateId::VerifyingDownload => {
                matches!(
                    to,
                    UpdateStateId::InspectingPackage | UpdateStateId::FailedPreApply
                )
            }
            UpdateStateId::InspectingPackage => {
                matches!(to, UpdateStateId::Staging | UpdateStateId::FailedPreApply)
            }
            UpdateStateId::Staging => {
                matches!(
                    to,
                    UpdateStateId::BuildingUpdatePlan | UpdateStateId::FailedPreApply
                )
            }
            UpdateStateId::BuildingUpdatePlan => {
                matches!(
                    to,
                    UpdateStateId::AwaitingApply | UpdateStateId::FailedPreApply
                )
            }
            UpdateStateId::AwaitingApply => matches!(
                to,
                UpdateStateId::Applying | UpdateStateId::Cleanup | UpdateStateId::FailedPreApply
            ),
            UpdateStateId::Applying => {
                matches!(
                    to,
                    UpdateStateId::LaunchingCandidate | UpdateStateId::FailedPostApply
                )
            }
            UpdateStateId::LaunchingCandidate => {
                matches!(
                    to,
                    UpdateStateId::AwaitingHealth | UpdateStateId::FailedPostApply
                )
            }
            UpdateStateId::AwaitingHealth => {
                matches!(
                    to,
                    UpdateStateId::Committed | UpdateStateId::FailedPostApply
                )
            }
            UpdateStateId::Committed => {
                // Committed jest terminalnym sukcesem — może przejść wyłącznie do Cleanup w celu zwolnienia blokad i stagingu
                matches!(to, UpdateStateId::Cleanup)
            }
            UpdateStateId::FailedPreApply => {
                // Błędy przed Applying przechodzą do czyszczenia tymczasowego
                matches!(to, UpdateStateId::Cleanup)
            }
            UpdateStateId::FailedPostApply => {
                // Błędy po Applying bezwzględnie wymagają Rollbacku
                matches!(to, UpdateStateId::Rollback)
            }
            UpdateStateId::Rollback => {
                // Po rollbacku: RollbackVerified przy sukcesie, lub FailedPostApply w razie awarii przywracania
                matches!(
                    to,
                    UpdateStateId::RollbackVerified | UpdateStateId::FailedPostApply
                )
            }
            UpdateStateId::RollbackVerified => {
                matches!(to, UpdateStateId::Cleanup)
            }
            UpdateStateId::Cleanup => {
                matches!(to, UpdateStateId::Idle)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_can_begin_check() {
        let mut sm = UpdateStateMachine::new();
        assert_eq!(sm.current_state(), UpdateStateId::Idle);
        assert!(sm.can_transition_to(UpdateStateId::CheckingRelease));
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        assert_eq!(sm.current_state(), UpdateStateId::CheckingRelease);
    }

    #[test]
    fn checking_release_cannot_jump_to_applying() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        assert!(!sm.can_transition_to(UpdateStateId::Applying));
        let res = sm.transition_to(UpdateStateId::Applying);
        assert!(matches!(res, Err(UpdaterError::InvalidTransition { .. })));
    }

    #[test]
    fn download_cannot_begin_before_asset_selection() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        // Próba bezpośredniego przejścia do Downloading bez SelectingAsset
        assert!(!sm.can_transition_to(UpdateStateId::Downloading));
        let res = sm.transition_to(UpdateStateId::Downloading);
        assert!(matches!(res, Err(UpdaterError::InvalidTransition { .. })));
    }

    #[test]
    fn apply_cannot_begin_before_validated_plan() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        sm.transition_to(UpdateStateId::SelectingAsset).unwrap();
        sm.transition_to(UpdateStateId::Downloading).unwrap();
        sm.transition_to(UpdateStateId::VerifyingDownload).unwrap();
        sm.transition_to(UpdateStateId::InspectingPackage).unwrap();
        sm.transition_to(UpdateStateId::Staging).unwrap();
        // Próba pominięcia BuildingUpdatePlan i AwaitingApply
        assert!(!sm.can_transition_to(UpdateStateId::Applying));
        let res = sm.transition_to(UpdateStateId::Applying);
        assert!(matches!(res, Err(UpdaterError::InvalidTransition { .. })));
    }

    #[test]
    fn committed_is_terminal() {
        let mut sm = UpdateStateMachine::new();
        // Przeprowadzenie pełnego łańcucha sukcesu
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        sm.transition_to(UpdateStateId::SelectingAsset).unwrap();
        sm.transition_to(UpdateStateId::Downloading).unwrap();
        sm.transition_to(UpdateStateId::VerifyingDownload).unwrap();
        sm.transition_to(UpdateStateId::InspectingPackage).unwrap();
        sm.transition_to(UpdateStateId::Staging).unwrap();
        sm.transition_to(UpdateStateId::BuildingUpdatePlan).unwrap();
        sm.transition_to(UpdateStateId::AwaitingApply).unwrap();
        sm.transition_to(UpdateStateId::Applying).unwrap();
        sm.transition_to(UpdateStateId::LaunchingCandidate).unwrap();
        sm.transition_to(UpdateStateId::AwaitingHealth).unwrap();
        sm.transition_to(UpdateStateId::Committed).unwrap();

        assert_eq!(sm.current_state(), UpdateStateId::Committed);
        assert!(sm.current_state().is_terminal_success());

        // Z Committed nie wolno przejść do Applying, Rollback, FailedPostApply ani bezpośrednio do Idle
        assert!(!sm.can_transition_to(UpdateStateId::Applying));
        assert!(!sm.can_transition_to(UpdateStateId::Rollback));
        assert!(!sm.can_transition_to(UpdateStateId::FailedPostApply));
        assert!(!sm.can_transition_to(UpdateStateId::Idle));

        // Jedyne dozwolone przejście to do Cleanup
        assert!(sm.can_transition_to(UpdateStateId::Cleanup));
    }

    #[test]
    fn failed_pre_apply_does_not_require_program_rollback() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::FailedPreApply).unwrap();

        assert!(!sm.current_state().requires_rollback());
        assert!(!sm.can_transition_to(UpdateStateId::Rollback));
        assert!(sm.can_transition_to(UpdateStateId::Cleanup));
        sm.transition_to(UpdateStateId::Cleanup).unwrap();
    }

    #[test]
    fn failed_post_apply_requires_recovery() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        sm.transition_to(UpdateStateId::SelectingAsset).unwrap();
        sm.transition_to(UpdateStateId::Downloading).unwrap();
        sm.transition_to(UpdateStateId::VerifyingDownload).unwrap();
        sm.transition_to(UpdateStateId::InspectingPackage).unwrap();
        sm.transition_to(UpdateStateId::Staging).unwrap();
        sm.transition_to(UpdateStateId::BuildingUpdatePlan).unwrap();
        sm.transition_to(UpdateStateId::AwaitingApply).unwrap();
        sm.transition_to(UpdateStateId::Applying).unwrap();
        sm.transition_to(UpdateStateId::FailedPostApply).unwrap();

        assert!(sm.current_state().requires_rollback());
        // Z FailedPostApply nie wolno przejść bezpośrednio do Cleanup ani Idle
        assert!(!sm.can_transition_to(UpdateStateId::Cleanup));
        assert!(!sm.can_transition_to(UpdateStateId::Idle));
        // Bezwzględnie wymagane przejście do Rollback
        assert!(sm.can_transition_to(UpdateStateId::Rollback));
    }

    #[test]
    fn rollback_verified_requires_rollback_result() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        sm.transition_to(UpdateStateId::SelectingAsset).unwrap();
        sm.transition_to(UpdateStateId::Downloading).unwrap();
        sm.transition_to(UpdateStateId::VerifyingDownload).unwrap();
        sm.transition_to(UpdateStateId::InspectingPackage).unwrap();
        sm.transition_to(UpdateStateId::Staging).unwrap();
        sm.transition_to(UpdateStateId::BuildingUpdatePlan).unwrap();
        sm.transition_to(UpdateStateId::AwaitingApply).unwrap();
        sm.transition_to(UpdateStateId::Applying).unwrap();
        sm.transition_to(UpdateStateId::FailedPostApply).unwrap();

        // Nie można przeskoczyć bezpośrednio z FailedPostApply do RollbackVerified
        assert!(!sm.can_transition_to(UpdateStateId::RollbackVerified));
        sm.transition_to(UpdateStateId::Rollback).unwrap();
        // Z Rollback przejście do RollbackVerified jest dozwolone
        assert!(sm.can_transition_to(UpdateStateId::RollbackVerified));
        sm.transition_to(UpdateStateId::RollbackVerified).unwrap();
    }

    #[test]
    fn cleanup_can_return_transaction_to_idle() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FailedPreApply).unwrap();
        sm.transition_to(UpdateStateId::Cleanup).unwrap();
        assert!(sm.can_transition_to(UpdateStateId::Idle));
        sm.transition_to(UpdateStateId::Idle).unwrap();
        assert_eq!(sm.current_state(), UpdateStateId::Idle);
    }

    #[test]
    fn state_machine_rejects_skipped_integrity_verification() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        sm.transition_to(UpdateStateId::SelectingAsset).unwrap();
        sm.transition_to(UpdateStateId::Downloading).unwrap();
        // Próba pominięcia VerifyingDownload i przejścia od razu do InspectingPackage lub Staging
        assert!(!sm.can_transition_to(UpdateStateId::InspectingPackage));
        assert!(!sm.can_transition_to(UpdateStateId::Staging));
    }

    #[test]
    fn state_machine_rejects_skipped_staging() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        sm.transition_to(UpdateStateId::SelectingAsset).unwrap();
        sm.transition_to(UpdateStateId::Downloading).unwrap();
        sm.transition_to(UpdateStateId::VerifyingDownload).unwrap();
        sm.transition_to(UpdateStateId::InspectingPackage).unwrap();
        // Próba pominięcia Staging i przejścia do BuildingUpdatePlan
        assert!(!sm.can_transition_to(UpdateStateId::BuildingUpdatePlan));
    }

    #[test]
    fn state_machine_rejects_skipped_update_plan() {
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease).unwrap();
        sm.transition_to(UpdateStateId::FetchingManifest).unwrap();
        sm.transition_to(UpdateStateId::VerifyingManifest).unwrap();
        sm.transition_to(UpdateStateId::SelectingAsset).unwrap();
        sm.transition_to(UpdateStateId::Downloading).unwrap();
        sm.transition_to(UpdateStateId::VerifyingDownload).unwrap();
        sm.transition_to(UpdateStateId::InspectingPackage).unwrap();
        sm.transition_to(UpdateStateId::Staging).unwrap();
        // Próba pominięcia planu i przejścia do AwaitingApply
        assert!(!sm.can_transition_to(UpdateStateId::AwaitingApply));
    }
}
