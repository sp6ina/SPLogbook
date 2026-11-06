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

/// Asynchronicznie eksportuje i podpisuje plik ADIF za pomocą TQSL, a następnie przesyła do LoTW
pub async fn export_and_sign_tqsl_async(
    tqsl_path: impl AsRef<Path>,
    station_location: &str,
    adif_content: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let temp_dir = std::env::temp_dir();
    let adif_path = temp_dir.join(format!("splogbook_lotw_{}.adi", uuid::Uuid::new_v4()));
    tokio::fs::write(&adif_path, adif_content).await?;

    let output = tokio::process::Command::new(tqsl_path.as_ref())
        .arg("-d")
        .arg("-u")
        .arg("-a")
        .arg("all")
        .arg("-x")
        .arg("-l")
        .arg(station_location)
        .arg(&adif_path)
        .output()
        .await;

    let _ = tokio::fs::remove_file(&adif_path).await;
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
pub async fn download_lotw_report(
    username: &str,
    password: &str,
    since_date: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = crate::core::http::http_client_with_timeout(30);

    let mut query: Vec<(&str, String)> = vec![
        ("login", username.to_string()),
        ("password", password.to_string()),
    ];
    if let Some(since) = since_date {
        let validated = LotwClient::validate_date(since)?;
        query.push(("qso_qslsince", validated));
    }

    let resp = crate::core::http::retry_async(
        || {
            let client = client.clone();
            let query = query.clone();
            async move {
                client
                    .get("https://lotw.arrl.org/lotwuser/lotwreport.adi")
                    .query(&query)
                    .send()
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                        let sanitized = crate::core::http::sanitize_network_error(&e);
                        LotwClient::redact_credentials(&sanitized).into()
                    })
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
