// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

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
    for p in DEFAULT_TQSL_PATHS {
        let path = Path::new(p);
        if path.exists() && path.is_file() {
            return Some(path.to_path_buf());
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        let bin_names = ["tqsl", "tqsl.exe"];
        let separator = if cfg!(target_os = "windows") { ';' } else { ':' };
        for p in path_var.split(separator) {
            for b in &bin_names {
                let p_buf = PathBuf::from(p).join(b);
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
) -> Result<String, Box<dyn std::error::Error>> {
    let temp_dir = std::env::temp_dir();
    let adif_path = temp_dir.join(format!("splogbook_lotw_{}.adi", chrono::Utc::now().timestamp()));
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
        .output()?;

    let _ = std::fs::remove_file(&adif_path);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("TQSL sukces: {}\n{}", stdout, stderr).trim().to_string())
    } else {
        Err(format!("Błąd wykonania TQSL (kod {}): {}\n{}", output.status.code().unwrap_or(-1), stdout, stderr).into())
    }
}

/// Pobiera raport potwierdzeń (QSL report) z serwerów ARRL LoTW w formacie ADIF
pub async fn download_lotw_report(
    username: &str,
    password: &str,
    since_date: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("SPLogbook/1.0.0 (SP6INA)")
        .build()?;

    let mut url = format!(
        "https://lotw.arrl.org/lotwuser/lotwreport.adi?login={}&password={}&qso_query=1&qso_qsl=yes",
        urlencoding_simple(username),
        urlencoding_simple(password)
    );

    if let Some(since) = since_date {
        url.push_str(&format!("&qso_qslsince={}", urlencoding_simple(since)));
    }

    let resp = client.get(&url).send().await?;
    let text = resp.text().await?;

    if text.contains("ARRL Logbook of the World") || text.contains("<EOH>") || text.contains("<eoh>") {
        Ok(text)
    } else if text.contains("Password") || text.contains("Unknown Username") {
        Err("Błąd logowania do LoTW: Nieprawidłowy login lub hasło.".into())
    } else {
        Err(format!("Nieznana odpowiedź z serwera LoTW: {}", text.chars().take(200).collect::<String>()).into())
    }
}

/// Proste kodowanie znaków URL dla loginu i hasła
fn urlencoding_simple(s: &str) -> String {
    s.replace('&', "%26").replace('=', "%3D").replace(' ', "%20")
}

/// Parsuje raport ADIF z LoTW i wyciąga potwierdzone łączności
pub fn parse_lotw_confirmations(adif: &str) -> Vec<LotwConfirmation> {
    let mut confirmations = Vec::new();
    let qsos = crate::core::adif::parse_adif(adif);

    for q in qsos {
        let rdate = q.lotw_qslrdate.unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d").to_string());
        confirmations.push(LotwConfirmation {
            callsign: q.callsign,
            band: q.band.to_lowercase(),
            mode: q.mode,
            qso_date: q.qso_date,
            qsl_rdate: rdate,
        });
    }

    confirmations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lotw_adif() {
        let adif = "<CALL:6>SP6INA<BAND:3>20M<MODE:2>CW<QSO_DATE:8>20260920<APP_LOTW_QSL:1>Y<APP_LOTW_RXQSL:8>20260920<EOR>";
        let confs = parse_lotw_confirmations(adif);
        assert_eq!(confs.len(), 1);
        assert_eq!(confs[0].callsign, "SP6INA");
        assert_eq!(confs[0].band, "20m");
        assert_eq!(confs[0].mode, "CW");
        assert_eq!(confs[0].qso_date, "20260920");
    }
}
