// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Typ domenowy identyfikatora próby aktualizacji (M3-A)

use crate::cloud::updater::UpdaterError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Niezmienniczy, bezpieczny typ domenowy reprezentujący unikalny identyfikator
/// próby aktualizacji (CSPRNG UUID v4).
///
/// Gwarancje bezpieczeństwa:
/// - Prywatna reprezentacja uniemożliwia bezpośrednie sfałszowanie wartości.
/// - Brak publicznego konstruktora pozwalającego na ominięcie walidacji.
/// - Bezpieczny do bezpośredniego użycia jako składowa ścieżki pliku (brak separatorów ścieżki).
/// - 122 bity losowości CSPRNG (brak zależności wyłącznie od znacznika czasu).
/// - Serializacja i Debug wykluczają obecność jakichkolwiek sekretów lub danych osobowych.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UpdateAttemptId(Uuid);

impl UpdateAttemptId {
    /// Generuje nowy, losowy identyfikator próby aktualizacji (UUID v4 z CSPRNG).
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Bezpieczne parsowanie ze str do odczytu utrwalonych transakcji i recovery.
    pub fn parse_str(s: &str) -> Result<Self, UpdaterError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(UpdaterError::InvalidAttemptId(
                "Identyfikator próby aktualizacji nie może być pusty.".into(),
            ));
        }

        // Ścisła walidacja znaków pod kątem bezpieczeństwa nazw plików (wykluczenie traversal i separatorów)
        for b in trimmed.bytes() {
            match b {
                b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b'-' => {}
                _ => {
                    return Err(UpdaterError::InvalidAttemptId(format!(
                        "Niedozwolony znak w UpdateAttemptId: '{}'",
                        b as char
                    )));
                }
            }
        }

        let parsed = Uuid::parse_str(trimmed).map_err(|e| {
            UpdaterError::InvalidAttemptId(format!(
                "Niepoprawny format UUID dla UpdateAttemptId: {e}"
            ))
        })?;

        // Odrzucenie Nil UUID (wszystkie zera) jako niezwalidowanego identyfikatora
        if parsed.is_nil() {
            return Err(UpdaterError::InvalidAttemptId(
                "Identyfikator próby aktualizacji nie może być Nil UUID (wszystkie zera).".into(),
            ));
        }

        Ok(Self(parsed))
    }

    /// Zwraca referencję do wewnętrznego identyfikatora UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Bezpieczna składowa nazwy pliku (36 znaków: cyfry szesnastkowe i myślniki).
    pub fn as_file_component(&self) -> String {
        self.0.hyphenated().to_string()
    }
}

impl fmt::Display for UpdateAttemptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.hyphenated())
    }
}

impl fmt::Debug for UpdateAttemptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UpdateAttemptId")
            .field(&self.0.hyphenated().to_string())
            .finish()
    }
}

impl FromStr for UpdateAttemptId {
    type Err = UpdaterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

impl TryFrom<&str> for UpdateAttemptId {
    type Error = UpdaterError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse_str(s)
    }
}

impl Serialize for UpdateAttemptId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.hyphenated().to_string())
    }
}

impl<'de> Deserialize<'de> for UpdateAttemptId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_attempt_id_is_unique() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let id = UpdateAttemptId::generate();
            assert!(
                ids.insert(id),
                "Wygenerowane UpdateAttemptId muszą być unikalne"
            );
        }
        assert_eq!(ids.len(), 1000);
    }

    #[test]
    fn update_attempt_id_round_trips() {
        let original = UpdateAttemptId::generate();
        let formatted = original.to_string();
        let parsed: UpdateAttemptId = formatted.parse().expect("Poprawne parsowanie UUID");
        assert_eq!(original, parsed);

        let json = serde_json::to_string(&original).expect("Serializacja do JSON");
        let deserialized: UpdateAttemptId =
            serde_json::from_str(&json).expect("Deserializacja z JSON");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn update_attempt_id_rejects_empty_value() {
        assert!(matches!(
            UpdateAttemptId::parse_str(""),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
        assert!(matches!(
            UpdateAttemptId::parse_str("   "),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
    }

    #[test]
    fn update_attempt_id_rejects_invalid_value() {
        // Niepoprawna długość
        assert!(matches!(
            UpdateAttemptId::parse_str("1234"),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
        // Nie-szesnastkowe znaki
        assert!(matches!(
            UpdateAttemptId::parse_str("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
        // Separatory ścieżki i próby path traversal
        assert!(matches!(
            UpdateAttemptId::parse_str("../123e4567-e89b-12d3-a456-426614174000"),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
        assert!(matches!(
            UpdateAttemptId::parse_str("123e4567-e89b-12d3-a456-426614174000/sub"),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
        assert!(matches!(
            UpdateAttemptId::parse_str("123e4567-e89b-12d3-a456-426614174000\\sub"),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
        // Nil UUID (wszystkie zera)
        assert!(matches!(
            UpdateAttemptId::parse_str("00000000-0000-0000-0000-000000000000"),
            Err(UpdaterError::InvalidAttemptId(_))
        ));
    }

    #[test]
    fn update_attempt_id_is_safe_for_file_component() {
        let id = UpdateAttemptId::generate();
        let comp = id.as_file_component();
        assert_eq!(comp.len(), 36);
        assert!(!comp.contains('/'));
        assert!(!comp.contains('\\'));
        assert!(!comp.contains(".."));
        assert!(!comp.contains(':'));
        assert!(!comp.contains('\0'));
        for c in comp.chars() {
            assert!(c.is_ascii_hexdigit() || c == '-');
        }
    }

    #[test]
    fn update_attempt_id_serialization_contains_no_secret() {
        let id = UpdateAttemptId::generate();
        let debug_str = format!("{id:?}");
        let display_str = format!("{id}");
        let json_str = serde_json::to_string(&id).unwrap();

        for s in &[debug_str, display_str, json_str] {
            assert!(!s.contains("key"));
            assert!(!s.contains("secret"));
            assert!(!s.contains("token"));
            assert!(!s.contains("password"));
            assert!(!s.contains("callsign"));
        }
    }

    #[test]
    fn update_attempt_id_does_not_depend_only_on_timestamp() {
        // Dwa identyfikatory wygenerowane w tym samym kwancie czasu muszą być różne
        let id1 = UpdateAttemptId::generate();
        let id2 = UpdateAttemptId::generate();
        assert_ne!(id1, id2);
        // Wersja UUID musi wynosić 4 (losowe CSPRNG)
        assert_eq!(id1.as_uuid().get_version(), Some(uuid::Version::Random));
        assert_eq!(id2.as_uuid().get_version(), Some(uuid::Version::Random));
    }
}
