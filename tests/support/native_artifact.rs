// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// Wspólny, hermetyczny kontrakt natywnych testów instalatorów.
//
// Bramka: STAGE-QA-BASELINE-DETERMINISM-AND-HERMETICITY-GATE (2026-11-09).
//
// Zasady, których pilnuje ten moduł:
// 1. Standardowy baseline NIE MOŻE wykonywać lokalnego instalatora tylko dlatego,
//    że plik istnieje w `dist/`. Wymagany jest jawny opt-in zmienną środowiskową.
// 2. Artefakt jest wskazywany JAWNĄ, ABSOLUTNĄ ścieżką. Brak globów, brak
//    autodetekcji katalogu `dist/`.
// 3. Gdy opt-in jest włączony, każdy brak lub niezgodność jest FAIL-CLOSED
//    (zwracamy `Err`, test panikuje z czytelnym komunikatem).
// 4. Tożsamość artefaktu jest weryfikowana po nazwie zgodnej z wersją pakietu
//    (`CARGO_PKG_VERSION`), rozszerzeniu, typie pliku i opcjonalnym SHA-256.
// 5. Proces instalatora ma ograniczony czas życia wyłącznie jako safety-net
//    przeciw zawieszeniu; time-out nie jest kryterium PASS żadnego testu.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Zmienna opt-in dla natywnego cyklu życia Inno Setup.
pub const INNO_OPT_IN_ENV: &str = "SPLOG_RUN_INNO_E2E";
/// Zmienna jawnej, absolutnej ścieżki artefaktu Inno Setup.
pub const INNO_PATH_ENV: &str = "SPLOG_INNO_INSTALLER_PATH";
/// Opcjonalna zmienna przypiętego SHA-256 artefaktu Inno Setup.
pub const INNO_SHA256_ENV: &str = "SPLOG_INNO_INSTALLER_SHA256";
/// Prefiks kanonicznej nazwy artefaktu Inno Setup.
pub const INNO_FILENAME_PREFIX: &str = "SPLogbook-Windows-x64-Setup-";

/// Zmienna opt-in dla kontroli artefaktu MSI.
pub const MSI_OPT_IN_ENV: &str = "SPLOG_RUN_MSI_E2E";
/// Zmienna jawnej, absolutnej ścieżki artefaktu MSI.
pub const MSI_PATH_ENV: &str = "SPLOG_MSI_INSTALLER_PATH";
/// Opcjonalna zmienna przypiętego SHA-256 artefaktu MSI.
pub const MSI_SHA256_ENV: &str = "SPLOG_MSI_INSTALLER_SHA256";
/// Prefiks kanonicznej nazwy artefaktu MSI.
pub const MSI_FILENAME_PREFIX: &str = "SPLogbook-Windows-x64-MSI-";

/// Wynik rozwiązania kontraktu opt-in.
#[derive(Debug)]
pub enum ArtifactDecision {
    /// Opt-in nie został włączony — test ma jawnie zakończyć się jako
    /// niewykonany fixture (z komunikatem, nie cichym `return`).
    NotRequested,
    /// Opt-in włączony i artefakt przeszedł walidację.
    Requested(RequestedArtifact),
}

/// Zweryfikowany artefakt wskazany jawnie przez użytkownika.
#[derive(Debug)]
pub struct RequestedArtifact {
    pub path: PathBuf,
    pub expected_sha256: Option<String>,
}

/// Rozwiązuje kontrakt opt-in w sposób fail-closed.
///
/// `env` jest wstrzykiwane, aby logikę można było przetestować bez modyfikacji
/// środowiska procesu (zmienne środowiskowe są globalne i współdzielone przez
/// równoległe testy).
#[allow(clippy::too_many_arguments)]
pub fn resolve_artifact<E>(
    env: E,
    opt_in_var: &str,
    path_var: &str,
    sha256_var: &str,
    required_extension: &str,
    filename_prefix: &str,
    expected_version: &str,
) -> Result<ArtifactDecision, String>
where
    E: Fn(&str) -> Option<String>,
{
    match env(opt_in_var) {
        None => return Ok(ArtifactDecision::NotRequested),
        Some(value) if value == "1" => {}
        Some(other) => {
            return Err(format!(
                "{opt_in_var} musi mieć dokładnie wartość `1` (otrzymano {other:?}); \
                 pozostaw zmienną nieustawioną, aby nie uruchamiać testu natywnego"
            ));
        }
    }

    let raw = env(path_var)
        .ok_or_else(|| format!("{opt_in_var}=1 wymaga jawnej, absolutnej ścieżki w {path_var}"))?;
    if raw.trim().is_empty() {
        return Err(format!("{path_var} nie może być pusty"));
    }
    if raw.trim() != raw {
        return Err(format!(
            "{path_var} nie może zawierać wiodących/końcowych białych znaków"
        ));
    }

    let path = PathBuf::from(&raw);
    if !path.is_absolute() {
        return Err(format!(
            "{path_var} musi być ścieżką absolutną (otrzymano {raw:?})"
        ));
    }

    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        format!(
            "artefakt {} nie istnieje lub jest niedostępny: {error}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "artefakt {} nie może być symlinkiem",
            path.display()
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "artefakt {} musi być zwykłym plikiem",
            path.display()
        ));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case(required_extension) {
        return Err(format!(
            "artefakt {} musi mieć rozszerzenie .{required_extension}",
            path.display()
        ));
    }

    let canonical_name = format!("{filename_prefix}{expected_version}.{required_extension}");
    let actual_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if actual_name != canonical_name {
        return Err(format!(
            "tożsamość artefaktu niezgodna: oczekiwano nazwy `{canonical_name}`, \
             otrzymano `{actual_name}` (wersja musi odpowiadać Cargo.toml)"
        ));
    }

    let expected_sha256 = match env(sha256_var) {
        None => None,
        Some(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            if normalized.len() != 64 || !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("{sha256_var} musi być 64-znakowym heksem SHA-256"));
            }
            Some(normalized)
        }
    };

    Ok(ArtifactDecision::Requested(RequestedArtifact {
        path,
        expected_sha256,
    }))
}

/// Oblicza SHA-256 pliku bez ładowania całości do pamięci.
pub fn sha256_hex(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let mut file = fs::File::open(path)
        .map_err(|error| format!("nie można otworzyć {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("błąd odczytu {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

/// Liczy SHA-256 artefaktu i weryfikuje opcjonalne przypięcie.
pub fn verify_sha256(artifact: &RequestedArtifact) -> Result<String, String> {
    let actual = sha256_hex(&artifact.path)?;
    if let Some(expected) = &artifact.expected_sha256
        && !actual.eq_ignore_ascii_case(expected)
    {
        return Err(format!(
            "SHA-256 artefaktu {} nie zgadza się: oczekiwano {expected}, otrzymano {actual}",
            artifact.path.display()
        ));
    }
    Ok(actual)
}

/// Uruchamia proces instalatora i czeka na jego zakończenie z safety-netem.
///
/// Time-out służy wyłącznie temu, aby zawieszony instalator (np. modalny
/// message box w trybie `/SILENT`) nie zawiesił całego baseline bezterminowo.
/// Nie jest kryterium wydajnościowym żadnego testu.
pub fn run_with_timeout(
    executable: &Path,
    arguments: &[String],
    environment: &[(String, String)],
    timeout: Duration,
    label: &str,
) -> Result<i32, String> {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (key, value) in environment {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(|error| {
        format!(
            "nie można uruchomić {label} ({}): {error}",
            executable.display()
        )
    })?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(status.code().unwrap_or(-1));
            }
            Ok(None) => {}
            Err(error) => {
                return Err(format!("błąd oczekiwania na {label}: {error}"));
            }
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{label} przekroczył safety-net {timeout:?} i został przerwany"
            ));
        }
        sleep(Duration::from_millis(25));
    }
}

/// Zwraca końcowe `lines` linii logu (do diagnostyki fail-closed).
pub fn log_tail(path: &Path, lines: usize) -> String {
    let Ok(content) = fs::read_to_string(path) else {
        return format!("(brak logu {})", path.display());
    };
    let all: Vec<&str> = content.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].join("\n")
}
