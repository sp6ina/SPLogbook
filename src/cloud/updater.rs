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
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

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
