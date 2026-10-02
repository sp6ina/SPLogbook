// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use reqwest::Client;

/// Klient eQSL.cc do przesyłania logów (ADIF) oraz pobierania skrzynki odbiorczej
pub struct EqslClient {
    client: Client,
    username: String,
    password: String,
}

impl EqslClient {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let client = crate::core::http::http_client_with_timeout(15);
        Self {
            client,
            username: username.into(),
            password: password.into(),
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
