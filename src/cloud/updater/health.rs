//! Kontrakt dowodu poprawności działania (Health Proof) dla aktualizatora SPLogbook (M3-A).
//!
//! Zgodnie z decyzjami OWNER-M2-13, OWNER-M2-14 oraz Sekcją 23 specyfikacji bramki M3-A:
//! - Dowód zdrowia (`HealthProof`) jest powiązany z:
//!   * attempt ID (`UpdateAttemptId`);
//!   * docelową wersją (`target_version`);
//!   * skrótem commita (`target_commit`);
//!   * platformą docelową (`platform`);
//!   * typem instalacji (`installation_type`);
//!   * znacznikiem czasu utworzenia (`created_at`);
//!   * wersją formatu dowodu (`proof_version`).
//! - Pliki dowodu bez poprawnego, zgodnego `attempt_id` są bezwzględnie odrzucane.
//! - Przestarzałe dowody (stale proofs przekraczające okno czasowe) są odrzucane.
//! - Dowód nie zawiera żadnych sekretów, kluczy prywatnych ani poświadczeń.
//! - Dowód nie pozwala na pominięcie kroków automatu stanów (wymaga stanu `AwaitingHealth` przed przejściem do `Committed`).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cloud::updater::UpdaterError;
use crate::cloud::updater::attempt_id::UpdateAttemptId;
use crate::cloud::updater::installation::InstallationType;
use crate::cloud::updater::state::{UpdateStateId, UpdateStateMachine};

/// Aktualna wersja schematu dowodu zdrowia.
pub const CURRENT_HEALTH_PROOF_VERSION: u32 = 1;

/// Domyślny czas ważności dowodu zdrowia w sekundach (15 sekund okna diagnostycznego).
pub const DEFAULT_HEALTH_PROOF_MAX_AGE_SECS: u64 = 15;

/// Kanoniczna nazwa pliku dowodu zdrowia kandydata w katalogu roboczym aktualizacji.
pub const HEALTH_PROOF_FILENAME: &str = ".health_ok";

/// Struktura kryptograficznie zweryfikowanego dowodu pomyślnego uruchomienia kandydata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthProof {
    /// Wersja schematu dowodu zdrowia.
    pub proof_version: u32,
    /// Unikalny identyfikator próby aktualizacji (CSPRNG UUID v4).
    pub attempt_id: UpdateAttemptId,
    /// Wersja aplikacji kandydata zgłaszającego dowód.
    pub target_version: String,
    /// Skrót commita Git kandydata zgłaszającego dowód.
    pub target_commit: String,
    /// Identyfikator platformy systemowej (np. "windows-x86_64", "linux-x86_64").
    pub platform: String,
    /// Rodzaj instalacji kandydata.
    pub installation_type: InstallationType,
    /// Znacznik czasu UTC wygenerowania dowodu zdrowia.
    pub created_at: DateTime<Utc>,
}

impl HealthProof {
    /// Tworzy nowy dowód zdrowia powiązany z bieżącą próbą aktualizacji.
    pub fn new(
        attempt_id: UpdateAttemptId,
        target_version: impl Into<String>,
        target_commit: impl Into<String>,
        platform: impl Into<String>,
        installation_type: InstallationType,
    ) -> Self {
        Self {
            proof_version: CURRENT_HEALTH_PROOF_VERSION,
            attempt_id,
            target_version: target_version.into(),
            target_commit: target_commit.into(),
            platform: platform.into(),
            installation_type,
            created_at: Utc::now(),
        }
    }

    /// Dokonuje pełnej walidacji dowodu zdrowia względem parametrów aktywnej transakcji.
    ///
    /// Weryfikuje:
    /// 1. Wersję formatu dowodu;
    /// 2. Zgodność unikalnego `attempt_id`;
    /// 3. Zgodność wersji docelowej (`target_version`);
    /// 4. Zgodność commita Git (`target_commit`);
    /// 5. Wiek dowodu (ochrona przed atakiem powtórzeniowym / stale proof).
    pub fn validate(
        &self,
        current_attempt: &UpdateAttemptId,
        expected_version: &str,
        expected_commit: &str,
        max_age_seconds: u64,
    ) -> Result<(), UpdaterError> {
        // 1. Weryfikacja wersji rekordu dowodu
        if self.proof_version != CURRENT_HEALTH_PROOF_VERSION {
            return Err(UpdaterError::SafetyGate(format!(
                "Nieobsługiwana wersja rekordu dowodu zdrowia: {} (oczekiwano {})",
                self.proof_version, CURRENT_HEALTH_PROOF_VERSION
            )));
        }

        // 2. Bezwzględna ochrona przed obcą próbą aktualizacji
        if self.attempt_id != *current_attempt {
            return Err(UpdaterError::SafetyGate(format!(
                "Dowód zdrowia należy do innej próby aktualizacji: {} (bieżąca próba: {})",
                self.attempt_id, current_attempt
            )));
        }

        // 3. Weryfikacja zgodności wersji kandydata
        if self.target_version != expected_version {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność wersji w dowodzie zdrowia: zgłoszono '{}', oczekiwano '{}'",
                self.target_version, expected_version
            )));
        }

        // 4. Weryfikacja zgodności commita Git kandydata
        if self.target_commit != expected_commit {
            return Err(UpdaterError::SafetyGate(format!(
                "Niezgodność commita w dowodzie zdrowia: zgłoszono '{}', oczekiwano '{}'",
                self.target_commit, expected_commit
            )));
        }

        // 5. Weryfikacja świeżości dowodu (ochrona przed stale / replay proof)
        let now = Utc::now();
        let max_age = Duration::seconds(max_age_seconds as i64);
        let age = now.signed_duration_since(self.created_at);

        if age > max_age {
            return Err(UpdaterError::SafetyGate(format!(
                "Dowód zdrowia wygasł (wiek: {}s, maksymalny dozwolony: {}s)",
                age.num_seconds(),
                max_age_seconds
            )));
        }

        // Zabezpieczenie przed cofniętym zegarem systemowym (> 5 sekund w przyszłość)
        if age < Duration::seconds(-5) {
            return Err(UpdaterError::SafetyGate(format!(
                "Dowód zdrowia posiada nieprawidłowy znacznik czasu z przyszłości: {}",
                self.created_at
            )));
        }

        Ok(())
    }

    /// Bezpieczny, atomowy zapis dowodu zdrowia do pliku w podanym katalogu bazowym.
    pub fn save_to_dir(&self, dir: &Path) -> Result<PathBuf, UpdaterError> {
        let serialized = serde_json::to_string_pretty(self).map_err(|e| {
            UpdaterError::Persistence(format!("Błąd serializacji dowodu zdrowia: {e}"))
        })?;

        let target_path = dir.join(HEALTH_PROOF_FILENAME);
        let temp_path = dir.join(format!(
            ".health_ok.{}.tmp",
            self.attempt_id.as_file_component()
        ));

        let mut file = File::create(&temp_path).map_err(|e| {
            UpdaterError::Io(format!(
                "Nie udało się utworzyć pliku tymczasowego dowodu zdrowia '{}': {e}",
                temp_path.display()
            ))
        })?;

        file.write_all(serialized.as_bytes()).map_err(|e| {
            UpdaterError::Io(format!("Nie udało się zapisać danych dowodu zdrowia: {e}"))
        })?;

        file.flush()
            .map_err(|e| UpdaterError::Io(format!("Błąd flush dowodu zdrowia: {e}")))?;

        #[cfg(not(target_os = "windows"))]
        file.sync_all()
            .map_err(|e| UpdaterError::Io(format!("Błąd sync_all dowodu zdrowia: {e}")))?;

        drop(file);

        std::fs::rename(&temp_path, &target_path).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            UpdaterError::Io(format!(
                "Błąd atomowej zamiany pliku dowodu zdrowia na '{}': {e}",
                target_path.display()
            ))
        })?;

        Ok(target_path)
    }

    /// Odczytuje i parsuje dowód zdrowia z pliku w podanym katalogu.
    pub fn load_from_dir(dir: &Path) -> Result<Self, UpdaterError> {
        let path = dir.join(HEALTH_PROOF_FILENAME);
        if !path.exists() {
            return Err(UpdaterError::Io(format!(
                "Plik dowodu zdrowia '{}' nie istnieje",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(&path).map_err(|e| {
            UpdaterError::Io(format!("Nie udało się odczytać pliku dowodu zdrowia: {e}"))
        })?;

        let proof: Self = serde_json::from_str(&content).map_err(|e| {
            UpdaterError::SafetyGate(format!("Uszkodzony format pliku dowodu zdrowia: {e}"))
        })?;

        Ok(proof)
    }

    /// Weryfikuje dowód zdrowia i dokonuje bezpiecznego przejścia automatu stanów do `Committed`.
    ///
    /// Jeśli automat nie znajduje się w stanie `AwaitingHealth`, próba przejścia jest odrzucana,
    /// uniemożliwiając pominięcie wcześniejszych etapów zabezpieczeń.
    pub fn verify_and_advance_to_committed(
        &self,
        sm: &mut UpdateStateMachine,
        current_attempt: &UpdateAttemptId,
        expected_version: &str,
        expected_commit: &str,
        max_age_seconds: u64,
    ) -> Result<(), UpdaterError> {
        if sm.current_state() != UpdateStateId::AwaitingHealth {
            return Err(UpdaterError::InvalidTransition(format!(
                "Nie można zastosować dowodu zdrowia w stanie '{}'. Wymagany stan to '{}'.",
                sm.current_state(),
                UpdateStateId::AwaitingHealth
            )));
        }

        self.validate(
            current_attempt,
            expected_version,
            expected_commit,
            max_age_seconds,
        )?;
        sm.transition_to(UpdateStateId::Committed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn health_proof_matches_current_attempt() {
        let attempt = UpdateAttemptId::generate();
        let proof = HealthProof::new(
            attempt,
            "1.2.1",
            "abcdef123456",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        assert!(
            proof
                .validate(&attempt, "1.2.1", "abcdef123456", 15)
                .is_ok()
        );
    }

    #[test]
    fn health_proof_rejects_foreign_attempt() {
        let attempt1 = UpdateAttemptId::generate();
        let attempt2 = UpdateAttemptId::generate();
        let proof = HealthProof::new(
            attempt1,
            "1.2.1",
            "abcdef123456",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        let err = proof
            .validate(&attempt2, "1.2.1", "abcdef123456", 15)
            .expect_err("musi odrzucić obcy attempt_id");
        assert!(matches!(err, UpdaterError::SafetyGate(_)));
    }

    #[test]
    fn health_proof_rejects_wrong_version() {
        let attempt = UpdateAttemptId::generate();
        let proof = HealthProof::new(
            attempt,
            "1.2.0", // Starsza lub błędna wersja
            "abcdef123456",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        let err = proof
            .validate(&attempt, "1.2.1", "abcdef123456", 15)
            .expect_err("musi odrzucić błędną wersję");
        assert!(matches!(err, UpdaterError::SafetyGate(_)));
    }

    #[test]
    fn health_proof_rejects_wrong_commit() {
        let attempt = UpdateAttemptId::generate();
        let proof = HealthProof::new(
            attempt,
            "1.2.1",
            "deadbeef9999", // Nieoczekiwany commit
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        let err = proof
            .validate(&attempt, "1.2.1", "abcdef123456", 15)
            .expect_err("musi odrzucić błędny commit");
        assert!(matches!(err, UpdaterError::SafetyGate(_)));
    }

    #[test]
    fn stale_health_proof_is_rejected() {
        let attempt = UpdateAttemptId::generate();
        let mut proof = HealthProof::new(
            attempt,
            "1.2.1",
            "abcdef123456",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        // Ustawienie znacznika czasu na 60 sekund wstecz przy limicie 15s
        proof.created_at = Utc::now() - Duration::seconds(60);

        let err = proof
            .validate(&attempt, "1.2.1", "abcdef123456", 15)
            .expect_err("przestarzały dowód zdrowia musi zostać odrzucony");
        assert!(matches!(err, UpdaterError::SafetyGate(_)));
    }

    #[test]
    fn health_proof_contains_no_secret() {
        let attempt = UpdateAttemptId::generate();
        let proof = HealthProof::new(
            attempt,
            "1.2.1",
            "abcdef123456",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        let json = serde_json::to_string(&proof).expect("serialize");
        let forbidden = [
            "secret",
            "token",
            "password",
            "key",
            "bearer",
            "credential",
            "auth",
        ];
        for word in forbidden {
            assert!(
                !json.to_ascii_lowercase().contains(word),
                "Dowód zdrowia zawiera zabronione słowo '{word}'"
            );
        }
    }

    #[test]
    fn health_proof_cannot_skip_awaiting_health() {
        let attempt = UpdateAttemptId::generate();
        let proof = HealthProof::new(
            attempt,
            "1.2.1",
            "abcdef123456",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        // Nowy automat startuje w Idle
        let mut sm = UpdateStateMachine::new();
        let err = proof
            .verify_and_advance_to_committed(&mut sm, &attempt, "1.2.1", "abcdef123456", 15)
            .expect_err("nie wolno pominąć stanów i przejść bezpośrednio z Idle");
        assert!(matches!(err, UpdaterError::InvalidTransition(_)));
        assert_eq!(sm.current_state(), UpdateStateId::Idle);

        // Przejście do Downloading
        sm.transition_to(UpdateStateId::CheckingRelease)
            .expect("step");
        sm.transition_to(UpdateStateId::FetchingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::VerifyingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::SelectingAsset)
            .expect("step");
        sm.transition_to(UpdateStateId::Downloading).expect("step");

        let err2 = proof
            .verify_and_advance_to_committed(&mut sm, &attempt, "1.2.1", "abcdef123456", 15)
            .expect_err("nie wolno pominąć stanów i przejść bezpośrednio z Downloading");
        assert!(matches!(err2, UpdaterError::InvalidTransition(_)));
    }

    #[test]
    fn valid_health_proof_allows_commit_transition() {
        let dir = tempdir().expect("tempdir");
        let attempt = UpdateAttemptId::generate();
        let proof = HealthProof::new(
            attempt,
            "1.2.1",
            "abcdef123456",
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );

        // Zapis i odczyt crash-safe
        proof.save_to_dir(dir.path()).expect("save proof");
        let loaded_proof = HealthProof::load_from_dir(dir.path()).expect("load proof");
        assert_eq!(proof, loaded_proof);

        // Przygotowanie automatu w poprawnym stanie AwaitingHealth
        let mut sm = UpdateStateMachine::new();
        sm.transition_to(UpdateStateId::CheckingRelease)
            .expect("step");
        sm.transition_to(UpdateStateId::FetchingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::VerifyingManifest)
            .expect("step");
        sm.transition_to(UpdateStateId::SelectingAsset)
            .expect("step");
        sm.transition_to(UpdateStateId::Downloading).expect("step");
        sm.transition_to(UpdateStateId::VerifyingDownload)
            .expect("step");
        sm.transition_to(UpdateStateId::InspectingPackage)
            .expect("step");
        sm.transition_to(UpdateStateId::Staging).expect("step");
        sm.transition_to(UpdateStateId::BuildingUpdatePlan)
            .expect("step");
        sm.transition_to(UpdateStateId::AwaitingApply)
            .expect("step");
        sm.transition_to(UpdateStateId::Applying).expect("step");
        sm.transition_to(UpdateStateId::LaunchingCandidate)
            .expect("step");
        sm.transition_to(UpdateStateId::AwaitingHealth)
            .expect("step");

        loaded_proof
            .verify_and_advance_to_committed(&mut sm, &attempt, "1.2.1", "abcdef123456", 15)
            .expect("poprawny dowód zdrowia musi zatwierdzić Committed");

        assert_eq!(sm.current_state(), UpdateStateId::Committed);
        assert!(sm.current_state().is_terminal_success());
    }
}
