// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs::{File, create_dir_all};
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
        let client = crate::core::http::http_client_with_timeout(15);

        Self {
            client,
            username: username.into(),
            password: password.into(),
            cache_dir: std::env::temp_dir().join("splogbook_eqsl"),
        }
    }

    pub fn with_cache_dir(
        username: impl Into<String>,
        password: impl Into<String>,
        cache_dir: impl AsRef<Path>,
    ) -> Self {
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

        let sanitize = |s: &str| -> String {
            s.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect()
        };
        let safe_call = sanitize(&dx_call.to_uppercase());
        let safe_band = sanitize(band);
        let safe_mode = sanitize(mode);

        let file_name = format!(
            "eqsl_{}_{}_{}_{}{}{}_{}{}.jpg",
            safe_call,
            safe_band,
            safe_mode,
            sanitize(year),
            sanitize(month),
            sanitize(day),
            sanitize(hour),
            sanitize(minute)
        );
        let target_path = self.cache_dir.join(&file_name);

        // Jeśli plik już jest w cache, zwróć go bez ponownego pobierania
        if tokio::fs::try_exists(&target_path).await.unwrap_or(false) {
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
    pub async fn upload_adif(
        &self,
        adif_content: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.client.clone();
        let username = self.username.clone();
        let password = self.password.clone();
        let adif = adif_content.to_string();
        let resp = crate::core::http::retry_async(
            || {
                let client = client.clone();
                let username = username.clone();
                let password = password.clone();
                let adif = adif.clone();
                async move {
                    client
                        .post("https://www.eqsl.cc/qslcard/ImportADIF.txt")
                        .form(&[
                            ("EQSL_USER", username.as_str()),
                            ("EQSL_PSWD", password.as_str()),
                            ("ADIFData", adif.as_str()),
                        ])
                        .send()
                        .await
                }
            },
            3,
        )
        .await?;

        let body = resp.text().await?;
        if body.contains("Result: 200")
            || body.contains("records were added")
            || body.contains("Success")
            || body.contains("imported")
        {
            Ok(format!("eQSL sukces: {}", body.trim()))
        } else {
            Err(format!("eQSL błąd: {}", body.trim()).into())
        }
    }

    /// Pobiera skrzynkę odbiorczą potwierdzonych łączności z serwisu eQSL.cc
    pub async fn download_inbox_adif(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .client
            .get("https://www.eqsl.cc/qslcard/DownloadInBox.cfm")
            .query(&[("UserName", &self.username), ("Password", &self.password)])
            .send()
            .await?;

        let body = resp.text().await?;
        if body.contains("<EOH>")
            || body.contains("<eoh>")
            || body.contains("<CALL:")
            || body.contains("<call:")
        {
            Ok(body)
        } else {
            Err(format!("eQSL błąd pobierania skrzynki: {}", body.trim()).into())
        }
    }
}
