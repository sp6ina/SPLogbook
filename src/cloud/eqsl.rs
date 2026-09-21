// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;

/// Klient pobierania graficznych kart e-QSL bezpośrednio z serwerów eQSL.cc (wzorem QLog)
pub struct EqslCardDownloader {
    client: Client,
    username: String,
    password: String,
    cache_dir: PathBuf,
}

pub type EqslClient = EqslCardDownloader;

impl EqslCardDownloader {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("SPLogbook/1.0.0 (SP6INA)")
            .build()
            .unwrap_or_default();

        Self {
            client,
            username: username.into(),
            password: password.into(),
            cache_dir: std::env::temp_dir().join("splogbook_eqsl"),
        }
    }

    pub fn with_cache_dir(username: impl Into<String>, password: impl Into<String>, cache_dir: impl AsRef<Path>) -> Self {
        let mut s = Self::new(username, password);
        s.cache_dir = cache_dir.as_ref().to_path_buf();
        s
    }

    /// Pobiera obraz graficznej karty eQSL dla podanej łączności i zapisuje go w lokalnym cache
    #[allow(clippy::too_many_arguments)]
    pub async fn download_card(
        &self,
        dx_call: &str,
        band: &str,
        mode: &str,
        year: &str,
        month: &str,
        day: &str,
        hour: &str,
        minute: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        create_dir_all(&self.cache_dir).await?;

        let file_name = format!(
            "eqsl_{}_{}_{}_{}{}{}_{}{}.jpg",
            dx_call.to_uppercase(),
            band,
            mode,
            year,
            month,
            day,
            hour,
            minute
        );
        let target_path = self.cache_dir.join(&file_name);

        // Jeśli plik już jest w cache, zwróć go bez ponownego pobierania
        if target_path.exists() {
            return Ok(target_path);
        }

        let resp = self
            .client
            .get("https://www.eqsl.cc/qslcard/GeteQSL.cfm")
            .query(&[
                ("Username", &self.username),
                ("Password", &self.password),
                ("CallsignFrom", &dx_call.to_string()),
                ("Band", &band.to_string()),
                ("Mode", &mode.to_string()),
                ("Year", &year.to_string()),
                ("Month", &month.to_string()),
                ("Day", &day.to_string()),
                ("Hour", &hour.to_string()),
                ("Minute", &minute.to_string()),
            ])
            .send()
            .await?;
        let bytes = resp.bytes().await?;

        // Sprawdź czy odpowiedź to faktycznie obraz JPEG (nagłówek JFIF / Exif: 0xFF, 0xD8)
        if bytes.len() > 100 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
            let mut file = File::create(&target_path).await?;
            file.write_all(&bytes).await?;
            Ok(target_path)
        } else {
            Err("Brak dostępnej karty graficznej na serwerze eQSL.cc".into())
        }
    }

    /// Przesyła łączności ADIF do serwisu eQSL.cc
    pub async fn upload_adif(&self, adif_content: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .client
            .post("https://www.eqsl.cc/qslcard/ImportADIF.txt")
            .form(&[
                ("EQSL_USER", self.username.as_str()),
                ("EQSL_PSWD", self.password.as_str()),
                ("ADIFData", adif_content),
            ])
            .send()
            .await?;

        let body = resp.text().await?;
        if body.contains("Result: 200") || body.contains("records were added") || body.contains("Success") || body.contains("imported") {
            Ok(format!("eQSL sukces: {}", body.trim()))
        } else {
            Err(format!("eQSL błąd: {}", body.trim()).into())
        }
    }

    /// Pobiera skrzynkę odbiorczą potwierdzonych łączności z serwisu eQSL.cc
    pub async fn download_inbox_adif(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .client
            .get("https://www.eqsl.cc/qslcard/DownloadInBox.cfm")
            .query(&[
                ("UserName", &self.username),
                ("Password", &self.password),
            ])
            .send()
            .await?;

        let body = resp.text().await?;
        if body.contains("<EOH>") || body.contains("<eoh>") || body.contains("<CALL:") || body.contains("<call:") {
            Ok(body)
        } else {
            Err(format!("eQSL błąd pobierania skrzynki: {}", body.trim()).into())
        }
    }
}
