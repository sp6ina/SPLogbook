// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::path::{Path, PathBuf};
use std::process::Command;

/// Potwierdzenie łączności odebrane z LoTW
#[derive(Debug, Clone, PartialEq)]
pub struct LotwConfirmation {
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub qso_date: String,
    pub qsl_rdate: String,
}

/// Domyślne potencjalne ścieżki do instalacji programu Trusted QSL (tqsl / tqsl.exe)
pub const DEFAULT_TQSL_PATHS: &[&str] = &[
    "C:\\Program Files (x86)\\Trusted QSL\\tqsl.exe",
    "C:\\Program Files\\Trusted QSL\\tqsl.exe",
    "/usr/bin/tqsl",
    "/usr/local/bin/tqsl",
    "/opt/trustedqsl/bin/tqsl",
    "tqsl.exe",
    "tqsl",
];

/// Wykrywa czy program Trusted QSL jest zainstalowany w standardowej lokalizacji lub w PATH
pub fn detect_tqsl_path() -> Option<PathBuf> {
    for candidate in DEFAULT_TQSL_PATHS {
        let path = Path::new(candidate);
        if path.exists() && path.is_file() {
            return Some(path.to_path_buf());
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        let bin_names = ["tqsl", "tqsl.exe"];
        let separator = if cfg!(target_os = "windows") {
            ';'
        } else {
            ':'
        };
        for dir in path_var.split(separator) {
            for bin_name in &bin_names {
                let p_buf = PathBuf::from(dir).join(bin_name);
                if p_buf.exists() && p_buf.is_file() {
                    return Some(p_buf);
                }
            }
        }
    }

    None
}

/// Eksportuje i podpisuje plik ADIF za pomocą TQSL, a następnie przesyła do LoTW
pub fn export_and_sign_tqsl(
    tqsl_path: impl AsRef<Path>,
    station_location: &str,
    adif_content: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let temp_dir = std::env::temp_dir();
    let adif_path = temp_dir.join(format!("splogbook_lotw_{}.adi", uuid::Uuid::new_v4()));
    std::fs::write(&adif_path, adif_content)?;

    // Argumenty TQSL:
    // -d: nie pytaj o hasło jeśli jest zapisane w TQSL
    // -u: automatycznie prześlij podpisany plik do ARRL LoTW przez Internet
    // -a all: podpisz wszystkie łączności z pliku
    // -x: zamknij program po zakończeniu operacji
    // -l <Location>: nazwa lokalizacji stacji zdefiniowana w TQSL
    let output = Command::new(tqsl_path.as_ref())
        .arg("-d")
        .arg("-u")
        .arg("-a")
        .arg("all")
        .arg("-x")
        .arg("-l")
        .arg(station_location)
        .arg(&adif_path)
        .output();

    let _ = std::fs::remove_file(&adif_path);
    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("TQSL sukces: {stdout}\n{stderr}")
            .trim()
            .to_string())
    } else {
        Err(format!(
            "Błąd wykonania TQSL (kod {}): {}\n{}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        )
        .into())
    }
}

/// Limit czasu jednego wywołania TQSL (podpisanie + wysyłka paczki).
///
/// Po jego przekroczeniu proces jest zabijany, a wynik jest klasyfikowany jako
/// [`TqslStatus::TimedOut`] — czyli **nigdy** jako sukces.
pub const TQSL_TIMEOUT_SECS: u64 = 300;

/// Maksymalna długość fragmentu stdout/stderr zachowywana w diagnostyce.
const OUTPUT_EXCERPT_LIMIT: usize = 400;

/// Wynik pojedynczego wywołania TQSL.
///
/// `status` jest klasyfikacją opartą wyłącznie na udokumentowanym kontrakcie
/// kodów wyjścia TQSL (0–11) oraz — pomocniczo — na ostatniej linii stderr
/// w formacie `Final Status: <opis> (<kod>)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TqslRun {
    pub status: TqslStatus,
    pub exit_code: Option<i32>,
    pub stderr_excerpt: String,
    pub stdout_excerpt: String,
}

impl TqslRun {
    /// Czy TQSL zaraportował pełny sukces (kod 0).
    pub fn is_success(&self) -> bool {
        matches!(self.status, TqslStatus::Success)
    }

    /// Krótki opis do komunikatów i logów.
    pub fn describe(&self) -> String {
        match self.exit_code {
            Some(code) => format!("{} (kod {code})", self.status.describe()),
            None => self.status.describe().to_string(),
        }
    }
}

/// Klasyfikacja udokumentowanych kodów wyjścia TQSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TqslStatus {
    /// 0 — wszystkie QSO podpisane i zapisane/wysłane.
    Success,
    /// 1 — anulowane przez użytkownika.
    CancelledByUser,
    /// 2 — odrzucone przez LoTW.
    RejectedByLotw,
    /// 3 — nieoczekiwana odpowiedź serwera TQSL.
    UnexpectedResponse,
    /// 4 — błąd TQSL.
    TqslError,
    /// 5 — błąd TQSLlib.
    TqslLibError,
    /// 6 — nie można otworzyć pliku wejściowego.
    InputFileError,
    /// 7 — nie można otworzyć pliku wyjściowego.
    OutputFileError,
    /// 8 — nie przetworzono żadnego QSO (duplikaty i/lub poza zakresem dat).
    NoQsosProcessed,
    /// 9 — część QSO przetworzona, część pominięta (duplikaty / zakres dat).
    SomeQsosProcessed,
    /// 10 — błąd składni polecenia.
    SyntaxError,
    /// 11 — błąd połączenia z LoTW.
    ConnectionError,
    /// Przekroczono limit czasu — proces zabity, brak dowodu sukcesu.
    TimedOut,
    /// Kod nieudokumentowany lub brak kodu (np. proces zabity sygnałem).
    Unknown,
}

impl TqslStatus {
    /// Stabilny identyfikator tekstowy (do logów i testów).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::CancelledByUser => "cancelled_by_user",
            Self::RejectedByLotw => "rejected_by_lotw",
            Self::UnexpectedResponse => "unexpected_response",
            Self::TqslError => "tqsl_error",
            Self::TqslLibError => "tqsllib_error",
            Self::InputFileError => "input_file_error",
            Self::OutputFileError => "output_file_error",
            Self::NoQsosProcessed => "no_qsos_processed",
            Self::SomeQsosProcessed => "some_qsos_processed",
            Self::SyntaxError => "syntax_error",
            Self::ConnectionError => "connection_error",
            Self::TimedOut => "timed_out",
            Self::Unknown => "unknown",
        }
    }

    /// Krótki opis po polsku do komunikatów użytkownika.
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Success => "TQSL: wszystkie QSO podpisane i wysłane",
            Self::CancelledByUser => "TQSL: anulowane przez użytkownika",
            Self::RejectedByLotw => "TQSL: odrzucone przez LoTW",
            Self::UnexpectedResponse => "TQSL: nieoczekiwana odpowiedź serwera",
            Self::TqslError => "TQSL: błąd TQSL",
            Self::TqslLibError => "TQSL: błąd biblioteki TQSL",
            Self::InputFileError => "TQSL: nie można otworzyć pliku wejściowego",
            Self::OutputFileError => "TQSL: nie można otworzyć pliku wyjściowego",
            Self::NoQsosProcessed => "TQSL: nie przetworzono żadnego QSO",
            Self::SomeQsosProcessed => "TQSL: część QSO przetworzona, część pominięta",
            Self::SyntaxError => "TQSL: błąd składni polecenia",
            Self::ConnectionError => "TQSL: błąd połączenia z LoTW",
            Self::TimedOut => "TQSL: przekroczono limit czasu",
            Self::Unknown => "TQSL: nieznany wynik",
        }
    }
}

/// Mapuje udokumentowany kod wyjścia TQSL na status.
///
/// Kody 0–11 pochodzą z oficjalnej tabeli „Status Information” dokumentacji
/// TQSL. Kod 0 jest jedynym sukcesem; kod 9 jest wynikiem **częściowym**, a kod
/// 8 oznacza brak przetworzonych QSO — żaden z nich nie jest sukcesem.
pub fn classify_tqsl_exit(code: Option<i32>) -> TqslStatus {
    match code {
        Some(0) => TqslStatus::Success,
        Some(1) => TqslStatus::CancelledByUser,
        Some(2) => TqslStatus::RejectedByLotw,
        Some(3) => TqslStatus::UnexpectedResponse,
        Some(4) => TqslStatus::TqslError,
        Some(5) => TqslStatus::TqslLibError,
        Some(6) => TqslStatus::InputFileError,
        Some(7) => TqslStatus::OutputFileError,
        Some(8) => TqslStatus::NoQsosProcessed,
        Some(9) => TqslStatus::SomeQsosProcessed,
        Some(10) => TqslStatus::SyntaxError,
        Some(11) => TqslStatus::ConnectionError,
        _ => TqslStatus::Unknown,
    }
}

/// Wyciąga `(opis, kod)` z ostatniej linii stderr w formacie
/// `HH:MM:SS AM/PM Final Status: <opis> (<kod>)`.
pub fn parse_tqsl_final_status(stderr: &str) -> Option<(String, i32)> {
    for line in stderr.lines().rev() {
        let Some(position) = line.find("Final Status:") else {
            continue;
        };
        let rest = line[position + "Final Status:".len()..].trim();
        let open = rest.rfind('(')?;
        let close = rest.rfind(')')?;
        if close <= open {
            continue;
        }
        let code = rest[open + 1..close].trim().parse::<i32>().ok()?;
        let description = rest[..open].trim().to_string();
        return Some((description, code));
    }
    None
}

/// Klasyfikuje przebieg TQSL: kod wyjścia jest autorytatywny, a linia
/// `Final Status:` służy wyłącznie jako sygnał pomocniczy, gdy kod jest
/// niedostępny.
pub fn classify_tqsl_run(exit_code: Option<i32>, stderr: &str) -> TqslStatus {
    match exit_code {
        Some(code) => classify_tqsl_exit(Some(code)),
        None => match parse_tqsl_final_status(stderr) {
            Some((_, code)) => classify_tqsl_exit(Some(code)),
            None => TqslStatus::Unknown,
        },
    }
}

/// Mapuje wynik TQSL na wynik paczki w centralnym modelu uploadu.
///
/// Tylko kod 0 daje [`BatchOutcome::Accepted`]. Kod 9 (część pominięta) jest
/// wynikiem częściowym, kod 8 (nic nie przetworzono) nie jest sukcesem, a kod 1
/// (anulowanie) nie może zostać uznany za powodzenie.
pub fn tqsl_batch_outcome(run: &TqslRun) -> crate::cloud::upload_plan::BatchOutcome {
    use crate::cloud::upload_plan::BatchOutcome;

    let detail = run.describe();
    match run.status {
        TqslStatus::Success => BatchOutcome::Accepted,
        TqslStatus::CancelledByUser => BatchOutcome::Cancelled,
        TqslStatus::RejectedByLotw => BatchOutcome::Rejected { detail },
        TqslStatus::UnexpectedResponse | TqslStatus::Unknown => BatchOutcome::Unknown { detail },
        TqslStatus::TqslError
        | TqslStatus::TqslLibError
        | TqslStatus::InputFileError
        | TqslStatus::OutputFileError
        | TqslStatus::SyntaxError
        | TqslStatus::ConnectionError => BatchOutcome::Failed { detail },
        TqslStatus::NoQsosProcessed => BatchOutcome::NoRecordsProcessed { detail },
        TqslStatus::SomeQsosProcessed => BatchOutcome::PartiallyAccepted {
            rejected: 0,
            detail: format!("{detail} (liczba pominiętych rekordów nieznana)"),
        },
        TqslStatus::TimedOut => BatchOutcome::TimedOut { detail },
    }
}

/// Błąd uruchomienia procesu TQSL (nie mylić z wynikiem podpisywania).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TqslSpawnError {
    /// Nie udało się uruchomić procesu (np. brak pliku wykonywalnego).
    Spawn(String),
    /// Błąd operacji na plikach tymczasowych.
    Io(String),
}

impl std::fmt::Display for TqslSpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(detail) => write!(f, "nie można uruchomić TQSL: {detail}"),
            Self::Io(detail) => write!(f, "błąd pliku tymczasowego TQSL: {detail}"),
        }
    }
}

impl std::error::Error for TqslSpawnError {}

fn excerpt(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    let mut out: String = trimmed.chars().take(OUTPUT_EXCERPT_LIMIT).collect();
    if trimmed.chars().count() > OUTPUT_EXCERPT_LIMIT {
        out.push('…');
    }
    out
}

/// Asynchronicznie podpisuje i wysyła jeden pakiet ADIF przez TQSL.
///
/// Parametry wywołania TQSL są zgodne z oficjalnym kontraktem CLI:
/// `-d` (bez okna zakresu dat), `-u` (wyślij zamiast zapisywać), `-a all`
/// (podpisz wszystkie QSO z wybranej paczki), `-x` (tryb wsadowy, status na
/// stderr), `-l <lokalizacja stacji>`. Hasło certyfikatu **nigdy** nie jest
/// przekazywane w argumentach (`-p` nie jest używane).
///
/// Zwraca [`TqslRun`] także dla niezerowych kodów wyjścia — błąd procesu nie
/// jest tym samym co nieudany upload, a rozróżnienie jest konieczne dla modelu
/// częściowego sukcesu.
pub async fn run_tqsl_async(
    tqsl_path: impl AsRef<Path>,
    station_location: &str,
    adif_content: &str,
) -> Result<TqslRun, TqslSpawnError> {
    use std::process::Stdio;

    let temp_dir = std::env::temp_dir();
    let adif_path = temp_dir.join(format!("splogbook_lotw_{}.adi", uuid::Uuid::new_v4()));
    tokio::fs::write(&adif_path, adif_content)
        .await
        .map_err(|e| TqslSpawnError::Io(e.to_string()))?;

    let mut command = tokio::process::Command::new(tqsl_path.as_ref());
    command
        .arg("-d")
        .arg("-u")
        .arg("-a")
        .arg("all")
        .arg("-x")
        .arg("-l")
        .arg(station_location)
        .arg(&adif_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = command.spawn();

    let result = match child {
        Ok(child) => {
            match tokio::time::timeout(
                std::time::Duration::from_secs(TQSL_TIMEOUT_SECS),
                child.wait_with_output(),
            )
            .await
            {
                Ok(Ok(output)) => {
                    let stderr = excerpt(&output.stderr);
                    let status = classify_tqsl_run(output.status.code(), &stderr);
                    Ok(TqslRun {
                        status,
                        exit_code: output.status.code(),
                        stderr_excerpt: stderr,
                        stdout_excerpt: excerpt(&output.stdout),
                    })
                }
                Ok(Err(e)) => Err(TqslSpawnError::Spawn(e.to_string())),
                Err(_elapsed) => Ok(TqslRun {
                    status: TqslStatus::TimedOut,
                    exit_code: None,
                    stderr_excerpt: String::new(),
                    stdout_excerpt: String::new(),
                }),
            }
        }
        Err(e) => Err(TqslSpawnError::Spawn(e.to_string())),
    };

    let _ = tokio::fs::remove_file(&adif_path).await;
    result
}

/// Asynchronicznie eksportuje i podpisuje plik ADIF za pomocą TQSL, a następnie przesyła do LoTW
pub async fn export_and_sign_tqsl_async(
    tqsl_path: impl AsRef<Path>,
    station_location: &str,
    adif_content: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let run = run_tqsl_async(tqsl_path, station_location, adif_content).await?;
    if run.is_success() {
        Ok(run.describe())
    } else {
        Err(format!("{}: {}", run.describe(), run.stderr_excerpt).into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LotwError {
    InvalidDateFormat(String),
    Network(String),
    AuthFailed,
    UnknownResponse(String),
}

impl std::fmt::Display for LotwError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDateFormat(d) => {
                write!(f, "Invalid LoTW date format '{d}', expected YYYY-MM-DD")
            }
            Self::Network(msg) => write!(f, "LoTW network error: {msg}"),
            Self::AuthFailed => write!(f, "Błąd logowania do LoTW: Nieprawidłowy login lub hasło."),
            Self::UnknownResponse(msg) => write!(f, "Nieznana odpowiedź z serwera LoTW: {msg}"),
        }
    }
}

impl std::error::Error for LotwError {}

/// Klient API serwera ARRL Logbook of the World (LoTW)
#[derive(Debug, Clone)]
pub struct LotwClient {
    pub username: String,
    pub password: String,
}

impl LotwClient {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Waliduje format daty qso_qslsince (YYYY-MM-DD)
    pub fn validate_date(date_str: &str) -> Result<String, LotwError> {
        let trimmed = date_str.trim();
        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.len() != 3 {
            return Err(LotwError::InvalidDateFormat(trimmed.to_string()));
        }
        let (y, m, d) = (parts[0], parts[1], parts[2]);
        if y.len() != 4 || m.len() != 2 || d.len() != 2 {
            return Err(LotwError::InvalidDateFormat(trimmed.to_string()));
        }
        let year: u32 = y
            .parse()
            .map_err(|_| LotwError::InvalidDateFormat(trimmed.to_string()))?;
        let month: u32 = m
            .parse()
            .map_err(|_| LotwError::InvalidDateFormat(trimmed.to_string()))?;
        let day: u32 = d
            .parse()
            .map_err(|_| LotwError::InvalidDateFormat(trimmed.to_string()))?;

        if !(1900..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day)
        {
            return Err(LotwError::InvalidDateFormat(trimmed.to_string()));
        }
        Ok(format!("{year:04}-{month:02}-{day:02}"))
    }

    /// Buduje pełny URL zapytania raportu LoTW z parametrami, w tym poprawnie zakodowanym qso_qslsince.
    ///
    /// **Wyłącznie do testów.** W kompilacji produkcyjnej funkcja nie istnieje,
    /// ponieważ zwracała adres zawierający hasło jako zwykły `String` — czyli
    /// dokładnie tę postać, która mogła wyciec do logu, błędu lub telemetrii.
    /// Produkcja buduje [`crate::core::http::SecretUrl`] i nigdy nie zamienia go
    /// na tekst.
    #[cfg(test)]
    pub fn build_report_url(&self, since_date: Option<&str>) -> Result<String, LotwError> {
        let mut params = vec![
            ("login", self.username.as_str()),
            ("password", self.password.as_str()),
        ];
        let validated_date;
        if let Some(since) = since_date {
            validated_date = Self::validate_date(since)?;
            params.push(("qso_qslsince", validated_date.as_str()));
        }
        let url = reqwest::Url::parse_with_params(
            "https://lotw.arrl.org/lotwuser/lotwreport.adi",
            &params,
        )
        .map_err(|e| LotwError::Network(e.to_string()))?;

        Ok(url.to_string())
    }

    /// Cenzuruje wrażliwe dane uwierzytelniające (hasło) w logach błędów
    pub fn redact_credentials(text: &str) -> String {
        let mut result = text.to_string();
        if let Some(idx) = result.find("password=") {
            let after = &result[idx + 9..];
            let end_idx = after.find('&').unwrap_or(after.len());
            result.replace_range(idx + 9..idx + 9 + end_idx, "***");
        }
        result
    }

    /// Pobiera raport potwierdzeń z LoTW za pomocą zapytania HTTP.
    pub async fn fetch_confirmations(
        &self,
        since_date: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        download_lotw_report(&self.username, &self.password, since_date).await
    }
}

/// Pobiera raport potwierdzeń (QSL report) z serwerów ARRL LoTW w formacie ADIF.
///
/// Oficjalna dokumentacja (`https://lotw.arrl.org/lotw-help/developer-query-qsos-qsls/`)
/// dopuszcza **wyłącznie** `GET` z poświadczeniami `login`/`password` w query
/// stringu; nie dokumentuje POST, HTTP Basic, sesji, cookies, klucza API ani
/// OAuth. Zmiana metody byłaby wymyśleniem protokołu, dlatego:
///
///  * adres powstaje jako [`crate::core::http::SecretUrl`] i nigdy nie jest zamieniany na tekst,
///  * klient ma **wyłączone przekierowania**, więc query nie może zostać
///    przeniesiony do innego adresu,
///  * każdy błąd transportu jest sanowany, zanim opuści tę funkcję,
///  * adres nie jest logowany, zapisywany ani emitowany w błędzie.
pub async fn download_lotw_report(
    username: &str,
    password: &str,
    since_date: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use crate::core::http::{SecretUrl, http_client_no_redirect, retry_async};

    let client = http_client_no_redirect(30);

    let mut query: Vec<(&str, &str)> = vec![("login", username), ("password", password)];
    let validated;
    if let Some(since) = since_date {
        validated = LotwClient::validate_date(since)?;
        query.push(("qso_qslsince", validated.as_str()));
    }

    let secret_url =
        SecretUrl::parse_with_params("https://lotw.arrl.org/lotwuser/lotwreport.adi", &query)?;

    let resp = retry_async(
        || {
            let client = client.clone();
            let secret_url = secret_url.clone();
            async move {
                secret_url.get(&client).send().await.map_err(
                    |e| -> Box<dyn std::error::Error + Send + Sync> {
                        let sanitized = crate::core::http::sanitize_network_error(&e);
                        LotwClient::redact_credentials(&sanitized).into()
                    },
                )
            }
        },
        3,
    )
    .await?;
    let text = resp.text().await?;

    if text.contains("ARRL Logbook of the World")
        || text.contains("<EOH>")
        || text.contains("<eoh>")
    {
        Ok(text)
    } else if text.contains("Password") || text.contains("Unknown Username") {
        Err(Box::new(LotwError::AuthFailed))
    } else {
        let err_snippet = text.chars().take(200).collect::<String>();
        let redacted = LotwClient::redact_credentials(&err_snippet);
        Err(Box::new(LotwError::UnknownResponse(redacted)))
    }
}

/// Parsuje raport ADIF z LoTW i wyciąga potwierdzone łączności
pub fn parse_lotw_confirmations(adif: &str) -> Vec<LotwConfirmation> {
    let mut confirmations = Vec::new();
    let qsos = crate::core::adif::parse_adif(adif);

    for qso in qsos {
        let rdate = qso
            .lotw_qslrdate
            .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());
        confirmations.push(LotwConfirmation {
            callsign: qso.callsign,
            band: qso.band.to_lowercase(),
            mode: qso.mode,
            qso_date: qso.qso_date,
            qsl_rdate: rdate,
        });
    }

    confirmations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lotw_fetch_confirmations_includes_qso_qslsince_when_provided() {
        let client = LotwClient::new("SP6INA", "secret_pass");
        let url = client.build_report_url(Some("2026-05-15")).unwrap();
        assert!(url.contains("qso_qslsince=2026-05-15"));
    }

    #[test]
    fn lotw_report_includes_qso_qslsince_parameter() {
        let client = LotwClient::new("SP6INA", "secret_pass");
        let url = client.build_report_url(Some("2026-05-15")).unwrap();
        assert!(url.contains("qso_qslsince=2026-05-15"));
    }

    #[test]
    fn lotw_report_formats_date_as_yyyy_mm_dd() {
        let date = LotwClient::validate_date("2026-05-15").unwrap();
        assert_eq!(date, "2026-05-15");
    }

    #[test]
    fn lotw_report_omits_qso_qslsince_when_none() {
        let client = LotwClient::new("SP6INA", "secret_pass");
        let url = client.build_report_url(None).unwrap();
        assert!(!url.contains("qso_qslsince="));
    }

    #[test]
    fn lotw_report_url_encodes_parameter_properly() {
        let client = LotwClient::new("SP6 INA", "p@ss&word=123");
        let url = client.build_report_url(Some("2026-05-15")).unwrap();
        assert!(url.contains("SP6+INA") || url.contains("SP6%20INA"));
        assert!(url.contains("%40"));
        assert!(url.contains("%26"));
    }

    #[test]
    fn lotw_report_does_not_duplicate_parameter() {
        let client = LotwClient::new("SP6INA", "secret_pass");
        let url = client.build_report_url(Some("2026-05-15")).unwrap();
        assert_eq!(url.matches("qso_qslsince=").count(), 1);
    }

    #[test]
    fn lotw_report_handles_utc_day_boundary() {
        let utc_today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let valid = LotwClient::validate_date(&utc_today).unwrap();
        assert_eq!(valid, utc_today);
    }

    #[test]
    fn lotw_report_rejects_invalid_date_format() {
        assert!(LotwClient::validate_date("15-05-2026").is_err());
        assert!(LotwClient::validate_date("2026/05/15").is_err());
        assert!(LotwClient::validate_date("NOT_A_DATE").is_err());
        assert!(LotwClient::validate_date("").is_err());
    }

    #[test]
    fn lotw_report_redacts_credentials_on_error() {
        let sensitive = "Error connecting to https://lotw.arrl.org/?login=SP6INA&password=SuperSecretPassword123&other=param";
        let redacted = LotwClient::redact_credentials(sensitive);
        assert!(!redacted.contains("SuperSecretPassword123"));
        assert!(redacted.contains("password=***"));
    }

    #[test]
    fn lotw_report_test_does_not_call_real_lotw() {
        let client = LotwClient::new("TEST_CALL", "TEST_PASS");
        let url = client.build_report_url(Some("2026-01-01")).unwrap();
        assert!(url.starts_with("https://lotw.arrl.org/lotwuser/lotwreport.adi"));
    }

    #[test]
    fn test_parse_lotw_adif() {
        let adif = "<CALL:6>SP6INA<BAND:3>20M<MODE:2>CW<QSO_DATE:8>20260920<TIME_ON:6>120000<APP_LOTW_QSL:1>Y<APP_LOTW_RXQSL:8>20260920<EOR>";
        let confs = parse_lotw_confirmations(adif);
        assert_eq!(confs.len(), 1);
        assert_eq!(confs[0].callsign, "SP6INA");
        assert_eq!(confs[0].band, "20m");
        assert_eq!(confs[0].mode, "CW");
        assert_eq!(confs[0].qso_date, "20260920");
    }
}

#[cfg(test)]
mod tqsl_contract_tests {
    use super::*;
    use crate::cloud::upload_plan::BatchOutcome;

    fn run_with_status(status: TqslStatus, exit_code: Option<i32>, stderr: &str) -> TqslRun {
        TqslRun {
            status,
            exit_code,
            stderr_excerpt: stderr.to_string(),
            stdout_excerpt: String::new(),
        }
    }

    #[test]
    fn documented_exit_codes_map_to_documented_statuses() {
        let expected = [
            (0, TqslStatus::Success),
            (1, TqslStatus::CancelledByUser),
            (2, TqslStatus::RejectedByLotw),
            (3, TqslStatus::UnexpectedResponse),
            (4, TqslStatus::TqslError),
            (5, TqslStatus::TqslLibError),
            (6, TqslStatus::InputFileError),
            (7, TqslStatus::OutputFileError),
            (8, TqslStatus::NoQsosProcessed),
            (9, TqslStatus::SomeQsosProcessed),
            (10, TqslStatus::SyntaxError),
            (11, TqslStatus::ConnectionError),
        ];
        for (code, status) in expected {
            assert_eq!(
                classify_tqsl_exit(Some(code)),
                status,
                "kod wyjścia TQSL {code}"
            );
        }
    }

    #[test]
    fn undocumented_or_missing_exit_code_is_unknown() {
        assert_eq!(classify_tqsl_exit(Some(12)), TqslStatus::Unknown);
        assert_eq!(classify_tqsl_exit(Some(-1)), TqslStatus::Unknown);
        assert_eq!(classify_tqsl_exit(Some(255)), TqslStatus::Unknown);
        assert_eq!(classify_tqsl_exit(None), TqslStatus::Unknown);
    }

    #[test]
    fn only_exit_code_zero_is_success() {
        for code in [1i32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] {
            assert!(
                !classify_tqsl_exit(Some(code)).as_str().is_empty(),
                "kod {code} musi mieć opis"
            );
            let run = run_with_status(classify_tqsl_exit(Some(code)), Some(code), "");
            assert!(
                !tqsl_batch_outcome(&run).is_accepted(),
                "kod {code} nie może być sukcesem"
            );
        }
        let ok = run_with_status(TqslStatus::Success, Some(0), "");
        assert!(tqsl_batch_outcome(&ok).is_accepted());
    }

    #[test]
    fn partial_exit_code_nine_is_partial_not_success() {
        let run = run_with_status(TqslStatus::SomeQsosProcessed, Some(9), "");
        let outcome = tqsl_batch_outcome(&run);
        assert!(matches!(outcome, BatchOutcome::PartiallyAccepted { .. }));
        assert!(!outcome.is_accepted());
    }

    #[test]
    fn duplicate_or_out_of_range_exit_code_eight_is_not_success() {
        let run = run_with_status(TqslStatus::NoQsosProcessed, Some(8), "");
        assert!(matches!(
            tqsl_batch_outcome(&run),
            BatchOutcome::NoRecordsProcessed { .. }
        ));
    }

    #[test]
    fn cancellation_is_never_success() {
        let run = run_with_status(TqslStatus::CancelledByUser, Some(1), "");
        assert!(matches!(tqsl_batch_outcome(&run), BatchOutcome::Cancelled));
    }

    #[test]
    fn timeout_is_never_success() {
        let run = run_with_status(TqslStatus::TimedOut, None, "");
        assert!(matches!(
            tqsl_batch_outcome(&run),
            BatchOutcome::TimedOut { .. }
        ));
    }

    #[test]
    fn connection_error_is_a_failure_not_a_success() {
        let run = run_with_status(TqslStatus::ConnectionError, Some(11), "");
        assert!(matches!(
            tqsl_batch_outcome(&run),
            BatchOutcome::Failed { .. }
        ));
    }

    #[test]
    fn final_status_line_is_parsed_from_the_last_matching_line() {
        let stderr = "06:05:56 PM: /home/x/k1mu.adi: 414 QSO records were duplicates\n\
                      06:05:56 PM: Final Status: Some QSOs were duplicates or out of date range (9)\n";
        assert_eq!(
            parse_tqsl_final_status(stderr),
            Some((
                "Some QSOs were duplicates or out of date range".to_string(),
                9
            ))
        );
    }

    #[test]
    fn final_status_success_line_is_parsed() {
        let stderr = "17:21:33 PM: Final Status: Success (0)";
        assert_eq!(
            parse_tqsl_final_status(stderr),
            Some(("Success".to_string(), 0))
        );
    }

    #[test]
    fn final_status_without_line_returns_none() {
        assert_eq!(parse_tqsl_final_status(""), None);
        assert_eq!(parse_tqsl_final_status("random output"), None);
        assert_eq!(parse_tqsl_final_status("Final Status: no code"), None);
    }

    #[test]
    fn exit_code_is_authoritative_over_stderr() {
        // Exit code mówi o sukcesie, a stderr zawiera starą linię o duplikatach.
        let stderr = "Final Status: Some QSOs were duplicates or out of date range (9)";
        assert_eq!(classify_tqsl_run(Some(0), stderr), TqslStatus::Success);
    }

    #[test]
    fn stderr_is_the_fallback_when_exit_code_is_missing() {
        let stderr = "06:05:56 PM: Final Status: Success (0)";
        assert_eq!(classify_tqsl_run(None, stderr), TqslStatus::Success);
        assert_eq!(classify_tqsl_run(None, "no status"), TqslStatus::Unknown);
    }

    #[test]
    fn run_description_contains_code_when_known() {
        let run = run_with_status(TqslStatus::Success, Some(0), "");
        assert!(run.describe().contains('0'));
        assert!(run.is_success());
        let unknown = run_with_status(TqslStatus::Unknown, None, "");
        assert!(!unknown.is_success());
    }
}
