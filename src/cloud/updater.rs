// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Asynchroniczny aktualizator baz referencyjnych (cty.dat, master.scp, lista użytkowników LoTW)

use std::path::Path;

pub struct DatabaseUpdater;

impl DatabaseUpdater {
    /// Pobiera najnowszy plik definicji krajów i prefiksów (cty.dat)
    pub async fn update_country_file(dest_path: &Path) -> Result<usize, String> {
        let url = "https://www.country-files.com/cty/cty.dat";
        Self::download_file(url, dest_path).await
    }

    /// Pobiera najnowszą bazę Super Check Partial (MASTER.SCP)
    pub async fn update_scp_file(dest_path: &Path) -> Result<usize, String> {
        let url = "https://www.supercheckpartial.com/MASTER.SCP";
        Self::download_file(url, dest_path).await
    }

    /// Pobiera wszystkie bazy referencyjne do wskazanego katalogu
    pub async fn update_all(dir: &Path) -> Result<(), String> {
        let _ = Self::update_all_with_report(dir, crate::core::i18n::Language::Pl).await?;
        Ok(())
    }

    /// Pobiera wszystkie bazy referencyjne (cty.dat, MASTER.SCP, lotw-user-activity.csv)
    /// niezależnie od siebie i zwraca podsumowanie dla paska statusu / powiadomienia Toast.
    pub async fn update_all_with_report(
        dir: &Path,
        lang: crate::core::i18n::Language,
    ) -> Result<String, String> {
        let mut ok_parts = Vec::new();
        let mut err_parts = Vec::new();

        match Self::update_country_file(&dir.join("cty.dat")).await {
            Ok(bytes) => ok_parts.push(format!("cty.dat ({} KB)", bytes / 1024)),
            Err(e) => err_parts.push(format!("cty.dat: {e}")),
        }

        match Self::update_scp_file(&dir.join("MASTER.SCP")).await {
            Ok(bytes) => ok_parts.push(format!("MASTER.SCP ({} KB)", bytes / 1024)),
            Err(e) => err_parts.push(format!("MASTER.SCP: {e}")),
        }

        match Self::update_lotw_users(&dir.join("lotw-user-activity.csv")).await {
            Ok(bytes) => ok_parts.push(format!("LoTW ({} KB)", bytes / 1024)),
            Err(e) => err_parts.push(format!("LoTW: {e}")),
        }

        if ok_parts.is_empty() {
            let prefix = crate::core::i18n::tr_or(
                lang,
                "❌ Błąd aktualizacji baz online",
                "❌ Online database update failed",
            );
            Err(format!("{prefix}: {}", err_parts.join(" | ")))
        } else if err_parts.is_empty() {
            let prefix = crate::core::i18n::tr_or(
                lang,
                "✔ Zaktualizowano bazy online",
                "✔ Online databases updated",
            );
            Ok(format!("{prefix}: {}", ok_parts.join(", ")))
        } else {
            let prefix = crate::core::i18n::tr_or(
                lang,
                "⚠ Częściowa aktualizacja baz",
                "⚠ Partial database update",
            );
            Ok(format!(
                "{prefix}: OK [{}], Err [{}]",
                ok_parts.join(", "),
                err_parts.join("; ")
            ))
        }
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
            .map_err(|e| format!("Błąd pobierania {url}: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Serwer zwrócił status {}: {}", resp.status(), url));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Błąd odczytu danych z {url}: {e}"))?;

        if let Some(parent) = dest_path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    format!("Błąd tworzenia katalogu {}: {e}", parent.display())
                })?;
            }
        }

        tokio::fs::write(dest_path, &bytes)
            .await
            .map_err(|e| format!("Błąd zapisu do pliku {}: {e}", dest_path.display()))?;

        Ok(bytes.len())
    }
}

/// Pojedynczy plik (asset) dołączony do wydania GitHub.
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    /// Suma kontrolna SHA256 (hex) udostępniona przez GitHub, jeśli istnieje.
    pub digest: Option<String>,
    pub size: u64,
}

/// Najnowsze wydanie programu w repozytorium GitHub.
#[derive(Debug, Clone)]
pub struct LatestRelease {
    pub tag: String,
    pub html_url: String,
    pub body: String,
    pub assets: Vec<ReleaseAsset>,
}

/// Wynik sprawdzenia aktualizacji — odróżnia stan „aktualny” od „nowa wersja”.
#[derive(Debug, Clone)]
pub enum UpdateCheckOutcome {
    UpToDate { local: String },
    NewVersion(LatestRelease),
    Error(String),
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
        .map_err(|e| format!("Błąd połączenia z GitHub: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "GitHub zwrócił status {} (sprawdź połączenie z internetem).",
            resp.status()
        ));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Błąd odczytu odpowiedzi: {e}"))?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Błąd parsowania odpowiedzi GitHub: {e}"))?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Brak pola tag_name w odpowiedzi z serwera.".to_string())?
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

    let mut assets = Vec::new();
    if let Some(arr) = json.get("assets").and_then(|v| v.as_array()) {
        for asset in arr {
            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let browser_download_url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let digest = asset
                .get("digest")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string);
            let size = asset
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if !name.is_empty() && !browser_download_url.is_empty() {
                assets.push(ReleaseAsset {
                    name,
                    browser_download_url,
                    digest,
                    size,
                });
            }
        }
    }

    Ok(LatestRelease {
        tag,
        html_url,
        body: release_body,
        assets,
    })
}

/// Oblicza sumę kontrolną SHA256 (hex, małe litery) z bajtów.
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let digest = Sha256::digest(data);
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in &digest {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Weryfikuje sumę kontrolną SHA256 (ignoruje wielkość liter i ewentualny prefiks `sha256:`).
pub fn verify_sha256(data: &[u8], expected: &str) -> bool {
    let expected = expected
        .trim()
        .trim_start_matches("sha256:")
        .trim_start_matches("SHA256:");
    let actual = sha256_hex(data);
    actual.eq_ignore_ascii_case(expected)
}

/// Wybiera najlepszy plik instalacyjny dla bieżącego systemu operacyjnego.
/// Zwraca `None`, jeśli żaden plik z wydania nie pasuje do docelowego systemu.
pub fn select_asset_for_platform(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    if assets.is_empty() {
        return None;
    }

    #[cfg(target_os = "windows")]
    let preferred: &[&str] = &[".exe", ".zip", ".msi"];
    #[cfg(target_os = "macos")]
    let preferred: &[&str] = &[".dmg", ".zip", ".tar.gz"];
    #[cfg(target_os = "linux")]
    let preferred: &[&str] = &[".appimage", ".deb", ".tar.gz"];
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let preferred: &[&str] = &[];

    for ext in preferred {
        if let Some(asset) = assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(&ext.to_lowercase()))
        {
            return Some(asset);
        }
    }

    None
}

/// Pobiera plik instalacyjny wydania do wskazanej lokalizacji.
pub async fn download_release_asset(
    asset: &ReleaseAsset,
    dest_path: &Path,
) -> Result<usize, String> {
    let client = crate::core::http::http_client_with_timeout(300);

    let resp = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "SPLogbook-update-check")
        .send()
        .await
        .map_err(|e| format!("Błąd pobierania aktualizacji: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Serwer zwrócił status {} podczas pobierania.",
            resp.status()
        ));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Błąd odczytu pobranych danych: {e}"))?;

    if let Some(parent) = dest_path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                format!("Błąd tworzenia katalogu {}: {e}", parent.display())
            })?;
        }
    }

    tokio::fs::write(dest_path, &bytes)
        .await
        .map_err(|e| format!("Błąd zapisu pobranego pliku {}: {e}", dest_path.display()))?;

    Ok(bytes.len())
}

/// Pobiera, weryfikuje i podmienia bieżący plik wykonywalny nową wersją.
///
/// Po wywołaniu (w systemie Windows) aplikacja powinna się zamknąć — proces
/// PowerShell czeka na zakończenie bieżącego procesu i dopiero wtedy podmienia
/// plik oraz uruchamia nową wersję.
pub async fn install_update(asset: &ReleaseAsset) -> Result<(), String> {
    let current =
        std::env::current_exe().map_err(|e| format!("Nie można ustalić ścieżki programu: {e}"))?;

    let parent = current
        .parent()
        .ok_or_else(|| "Nie można ustalić katalogu programu.".to_string())?;

    let tmp_path = parent.join(format!(".SPLogbook_update_{}.tmp", std::process::id()));

    // Zabezpieczenie przed uszkodzeniem instalacji
    check_safety_gate(asset)?;

    // 1. Pobierz nowy plik do katalogu programu (ten sam wolumen → atomowe Move-Item).
    download_release_asset(asset, &tmp_path).await?;

    // 2. Zweryfikuj sumę kontrolną, jeśli GitHub ją udostępnił.
    if let Some(expected) = asset.digest.as_deref() {
        let data = tokio::fs::read(&tmp_path)
            .await
            .map_err(|e| format!("Błąd odczytu pobranego pliku: {e}"))?;
        if !verify_sha256(&data, expected) {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(
                "Suma kontrolna SHA256 pobranej aktualizacji nie zgadza się z wartością z GitHub."
                    .to_string(),
            );
        }
    }

    // 3. Podmień plik wykonywalny i uruchom ponownie.
    self_replace(&current, &tmp_path)
}

/// Sprawdza, czy pobierany plik nie jest archiwum, co mogłoby uszkodzić instalację.
pub fn check_safety_gate(asset: &ReleaseAsset) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let _ = asset;

    #[cfg(not(target_os = "windows"))]
    {
        let name_lower = asset.name.to_lowercase();
        if name_lower.ends_with(".tar.gz")
            || name_lower.ends_with(".deb")
            || name_lower.ends_with(".rpm")
        {
            return Err("Błąd: Pobrany plik to archiwum. Nadpisanie aplikacji zniszczyłoby instalację. Zaktualizuj program ręcznie.".to_string());
        }
    }
    Ok(())
}

/// Podmienia działający plik wykonywalny nową wersją.
#[cfg(target_os = "windows")]
fn self_replace(current: &Path, new: &Path) -> Result<(), String> {
    install_via_powershell(current, new)
}

/// Podmienia działający plik wykonywalny nową wersją (systemy Unix).
#[cfg(not(target_os = "windows"))]
fn self_replace(current: &Path, new: &Path) -> Result<(), String> {
    std::fs::rename(new, current)
        .map_err(|e| format!("Nie można podmienić pliku programu: {}", e))?;
    // Uruchom ponownie nową wersję w tle.
    let _ = std::process::Command::new(current).spawn();
    Ok(())
}

/// Windows: uruchamia skrypt PowerShell, który czeka na zamknięcie bieżącego
/// procesu, podmienia plik wykonywalny i uruchamia go ponownie.
#[cfg(target_os = "windows")]
fn install_via_powershell(current: &Path, new: &Path) -> Result<(), String> {
    let script_path =
        std::env::temp_dir().join(format!("SPLogbook_update_{}.ps1", std::process::id()));

    let exe = current.to_string_lossy().replace('\'', "''");
    let new_s = new.to_string_lossy().replace('\'', "''");
    let pid = std::process::id();

    let script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         $exe = '{exe}'\n\
         $new = '{new_s}'\n\
         $pidToWait = {pid}\n\
         $deadline = (Get-Date).AddSeconds(90)\n\
         while (Get-Process -Id $pidToWait -ErrorAction SilentlyContinue) {{\n\
             if ((Get-Date) -gt $deadline) {{ exit 1 }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         Start-Sleep -Milliseconds 500\n\
         Move-Item -Force -LiteralPath $new -Destination $exe\n\
         Start-Process -FilePath $exe\n"
    );

    std::fs::write(&script_path, script)
        .map_err(|e| format!("Nie można zapisać skryptu aktualizacji: {e}"))?;

    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .spawn()
        .map_err(|e| format!("Nie można uruchomić aktualizacji: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_matches_known_vector() {
        // SHA256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_sha256_accepts_case_and_prefix() {
        let expected = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        assert!(verify_sha256(b"abc", expected));
        assert!(verify_sha256(
            b"abc",
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));
        assert!(!verify_sha256(b"abd", expected));
    }

    #[test]
    fn select_asset_prefers_matching_platform_and_rejects_unmatched() {
        let assets = vec![
            ReleaseAsset {
                name: "SPLogbook-1.0.4-win64.zip".into(),
                browser_download_url: "https://example.com/z.zip".into(),
                digest: None,
                size: 0,
            },
            ReleaseAsset {
                name: "SPLogbook-1.0.4-win64.exe".into(),
                browser_download_url: "https://example.com/x.exe".into(),
                digest: None,
                size: 0,
            },
            ReleaseAsset {
                name: "SPLogbook-1.0.4-linux.AppImage".into(),
                browser_download_url: "https://example.com/l.AppImage".into(),
                digest: None,
                size: 0,
            },
            ReleaseAsset {
                name: "SPLogbook-1.0.4-macos.dmg".into(),
                browser_download_url: "https://example.com/m.dmg".into(),
                digest: None,
                size: 0,
            },
        ];

        #[cfg(target_os = "windows")]
        assert_eq!(
            select_asset_for_platform(&assets).unwrap().name,
            "SPLogbook-1.0.4-win64.exe"
        );
        #[cfg(target_os = "linux")]
        assert_eq!(
            select_asset_for_platform(&assets).unwrap().name,
            "SPLogbook-1.0.4-linux.AppImage"
        );
        #[cfg(target_os = "macos")]
        assert_eq!(
            select_asset_for_platform(&assets).unwrap().name,
            "SPLogbook-1.0.4-macos.dmg"
        );

        let unmatched = vec![ReleaseAsset {
            name: "checksums.txt".into(),
            browser_download_url: "https://example.com/checksums.txt".into(),
            digest: None,
            size: 0,
        }];
        assert!(select_asset_for_platform(&unmatched).is_none());
        assert!(select_asset_for_platform(&[]).is_none());
    }

    #[test]
    fn safety_gate_blocks_linux_archives() {
        let tar_gz = ReleaseAsset {
            name: "SPLogbook-Linux-x86_64.tar.gz".into(),
            browser_download_url: "".into(),
            digest: None,
            size: 0,
        };
        
        #[cfg(not(target_os = "windows"))]
        assert!(check_safety_gate(&tar_gz).is_err());
        
        #[cfg(target_os = "windows")]
        assert!(check_safety_gate(&tar_gz).is_ok()); // Windows is tested separately in UPDATE-WIN-SAFETY-GATE
    }
}
