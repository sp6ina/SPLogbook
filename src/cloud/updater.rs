// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Asynchroniczny aktualizator baz referencyjnych (cty.dat, master.scp, lista użytkowników LoTW)

use std::path::Path;

pub struct DatabaseUpdater;

impl DatabaseUpdater {
    /// Pobiera najnowszy plik definicji krajów i prefiksów (cty.dat)
    pub async fn update_country_file(dest_path: &Path) -> Result<usize, String> {
        let url = "http://www.country-files.com/cty/cty.dat";
        Self::download_file(url, dest_path).await
    }

    /// Pobiera najnowszą bazę Super Check Partial (MASTER.SCP)
    pub async fn update_scp_file(dest_path: &Path) -> Result<usize, String> {
        let url = "https://www.supercheckpartial.com/MASTER.SCP";
        Self::download_file(url, dest_path).await
    }

    /// Pobiera wszystkie bazy referencyjne do wskazanego katalogu
    pub async fn update_all(dir: &Path) -> Result<(), String> {
        let _ = Self::update_country_file(&dir.join("cty.dat")).await;
        let _ = Self::update_scp_file(&dir.join("MASTER.SCP")).await;
        let _ = Self::update_lotw_users(&dir.join("lotw-user-activity.csv")).await;
        Ok(())
    }

    /// Pobiera listę aktywnych użytkowników LoTW (lotw-user-activity.csv)
    pub async fn update_lotw_users(dest_path: &Path) -> Result<usize, String> {
        let url = "https://lotw.arrl.org/lotw-user-activity.csv";
        Self::download_file(url, dest_path).await
    }

    async fn download_file(url: &str, dest_path: &Path) -> Result<usize, String> {
        let client = crate::core::http::http_client_with_timeout(30);

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Błąd pobierania {}: {}", url, e))?;

        if !resp.status().is_success() {
            return Err(format!("Serwer zwrócił status {}: {}", resp.status(), url));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Błąd odczytu danych z {}: {}", url, e))?;

        if let Some(parent) = dest_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        std::fs::write(dest_path, &bytes)
            .map_err(|e| format!("Błąd zapisu do pliku {:?}: {}", dest_path, e))?;

        Ok(bytes.len())
    }
}

/// Najnowsze wydanie programu w repozytorium GitHub.
#[derive(Debug, Clone)]
pub struct LatestRelease {
    pub tag: String,
    pub html_url: String,
    pub body: String,
}

/// Pobiera metadane najnowszego wydania z GitHub API (bez autoryzacji).
pub async fn latest_release() -> Result<LatestRelease, String> {
    let client = crate::core::http::http_client_with_timeout(15);
    let url = "https://api.github.com/repos/sp6ina/SPLogbook/releases/latest";

    let resp = client
        .get(url)
        .header("User-Agent", "SPLogbook-update-check")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Błąd połączenia z GitHub: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "GitHub zwrócił status {} (sprawdź połączenie z internetem).",
            resp.status()
        ));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Błąd odczytu odpowiedzi: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Błąd parsowania odpowiedzi GitHub: {}", e))?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("nieznana")
        .trim_start_matches('v')
        .to_string();
    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/sp6ina/SPLogbook/releases")
        .to_string();
    let release_body = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(LatestRelease { tag, html_url, body: release_body })
}
