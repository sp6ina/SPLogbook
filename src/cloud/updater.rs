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
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Błąd tworzenia katalogu {}: {e}", parent.display()))?;
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

/// Zaufany klucz publiczny wbudowany w aplikację.
pub struct TrustedKey {
    pub key_id: &'static str,
    pub public_key: [u8; 32],
    pub active: bool,
    pub min_version: Option<&'static str>,
    pub max_version: Option<&'static str>,
}

/// Zaufane klucze publiczne (hardcoded).
pub const TRUSTED_KEYS: &[TrustedKey] = &[TrustedKey {
    key_id: "splogbook-release-2026-01",
    public_key: [
        108, 158, 236, 247, 76, 222, 53, 150, 46, 23, 120, 161, 249, 13, 121, 38, 
        124, 113, 138, 53, 232, 229, 152, 165, 63, 107, 44, 18, 74, 26, 19, 118
    ],
    active: true,
    min_version: Some("1.1.0"),
    max_version: None,
}];

/// Weryfikuje podpis na podstawie surowych bajtów manifestu.
/// Sprawdza po kolei klucze. Zwraca dopasowany klucz i sparsowany manifest.
pub fn verify_manifest_signature(
    manifest_bytes: &[u8],
    signature_base64: &str,
) -> Result<(&'static TrustedKey, ReleaseManifest), String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let sig_bytes = STANDARD
        .decode(signature_base64)
        .map_err(|e| format!("Błąd dekodowania podpisu Base64: {}", e))?;

    if sig_bytes.len() != 64 {
        return Err("Nieprawidłowa długość podpisu Ed25519".into());
    }

    let signature =
        Signature::from_slice(&sig_bytes).map_err(|e| format!("Błąd ładowania podpisu: {}", e))?;

    let mut verified_key = None;
    for key in TRUSTED_KEYS.iter().filter(|k| k.active) {
        if let Ok(verifying_key) = VerifyingKey::from_bytes(&key.public_key) {
            if verifying_key.verify(manifest_bytes, &signature).is_ok() {
                verified_key = Some(key);
                break;
            }
        }
    }

    let key = verified_key
        .ok_or_else(|| "Żaden z zaufanych kluczy nie zweryfikował tego podpisu".to_string())?;

    let manifest: ReleaseManifest = serde_json::from_slice(manifest_bytes).map_err(|e| {
        format!(
            "Błąd parsowania manifestu JSON po weryfikacji podpisu: {}",
            e
        )
    })?;

    if manifest.key_id != key.key_id {
        return Err(format!(
            "Manifest deklaruje key_id = {}, ale został podpisany kluczem {}",
            manifest.key_id, key.key_id
        ));
    }

    manifest.validate()?;

    // SemVer validation against key constraints
    let manifest_ver = semver::Version::parse(&manifest.version)
        .map_err(|e| format!("Nieprawidłowa wersja w manifeście: {}", e))?;

    if let Some(min_v) = key.min_version {
        let min_ver = semver::Version::parse(min_v).unwrap();
        if manifest_ver < min_ver {
            return Err(format!(
                "Wersja {} jest zbyt stara dla tego klucza (min: {})",
                manifest.version, min_v
            ));
        }
    }

    if let Some(max_v) = key.max_version {
        let max_ver = semver::Version::parse(max_v).unwrap();
        if manifest_ver > max_ver {
            return Err(format!(
                "Wersja {} jest zbyt nowa dla tego klucza (max: {})",
                manifest.version, max_v
            ));
        }
    }

    Ok((key, manifest))
}

/// Weryfikuje czy manifest wnosi nowszą wersję w stosunku do aktualnie uruchomionej.
/// Implementuje politykę CRYPTO-SEMVER-POLICY oraz CRYPTO-DOWNGRADE-PROTECTION,
/// zapobiegając instalacji wersji równych oraz starszych.
pub fn verify_downgrade_protection(
    manifest_version: &str,
    current_version: &str,
) -> Result<(), String> {
    let m_ver = semver::Version::parse(manifest_version)
        .map_err(|e| format!("Nieprawidłowa wersja w manifeście: {}", e))?;
    let c_ver = semver::Version::parse(current_version)
        .map_err(|e| format!("Nieprawidłowa bieżąca wersja: {}", e))?;

    if m_ver < c_ver {
        return Err(format!(
            "Odmowa downgrade: wersja w manifeście ({}) jest starsza od zainstalowanej ({}).",
            manifest_version, current_version
        ));
    }
    if m_ver == c_ver {
        return Err(format!(
            "Zainstalowana jest już ta sama wersja ({}) - aktualizacja odrzucona.",
            current_version
        ));
    }

    Ok(())
}

/// Dokument manifestu nowej architektury wydawniczej zabezpieczony Ed25519.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReleaseManifest {
    pub manifest_version: u32,
    pub key_id: String,
    pub product: String,
    pub version: String,
    pub channel: String,
    pub minimum_updater_version: String,
    pub commit: String,
    pub assets: Vec<ManifestAsset>,
}

impl ReleaseManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.manifest_version != 1 {
            return Err(format!(
                "Nieznana wersja manifestu: {}",
                self.manifest_version
            ));
        }
        if self.product != "SPLogbook" {
            return Err(format!(
                "Oczekiwano product = SPLogbook, otrzymano: {}",
                self.product
            ));
        }
        if self.commit.len() != 40 || !self.commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Nieprawidłowy skrót commit SHA-1".into());
        }
        if self.assets.is_empty() {
            return Err("Manifest nie zawiera artefaktów".into());
        }

        let mut names = std::collections::HashSet::new();
        let mut triads = std::collections::HashSet::new();
        for asset in &self.assets {
            if !names.insert(&asset.filename) {
                return Err(format!("Zduplikowany filename: {}", asset.filename));
            }
            let triad = (&asset.platform, &asset.arch, &asset.package_type);
            if !triads.insert(triad) {
                return Err(format!(
                    "Niejednoznaczny pakiet (platform+arch+type): {:?}",
                    triad
                ));
            }
            if asset.sha256.len() != 64 || !asset.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!(
                    "Nieprawidłowy SHA-256 dla pliku {}",
                    asset.filename
                ));
            }
        }
        Ok(())
    }
}

/// Pojedynczy zasób zdefiniowany w manifeście.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestAsset {
    pub platform: String,
    pub arch: String,
    pub package_type: String,
    pub filename: String,
    pub size: u64,
    pub sha256: String,
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
    let mut latest = LatestRelease {
        tag,
        html_url,
        body: release_body,
        assets,
    };

    // 3.1.G Integracja całości - uderza nowym pobieraniem z podaniem zaufanego pakietu
    let current_version = env!("CARGO_PKG_VERSION");
    let manifest = fetch_and_verify_manifest(&latest, current_version).await?;

    // Podmień `assets` na wyłącznie te pliki, które są potwierdzone kryptograficznie w manifeście
    let mut verified_assets = Vec::new();
    for m_asset in manifest.assets {
        if let Some(mut g_asset) = latest
            .assets
            .iter()
            .find(|a| a.name == m_asset.filename)
            .cloned()
        {
            // Nadpisz digest z GitHub wartością z zaufanego manifestu (SHA-256)
            g_asset.digest = Some(format!("sha256:{}", m_asset.sha256));
            verified_assets.push(g_asset);
        }
    }

    if verified_assets.is_empty() {
        return Err(
            "Manifest nie zawiera plików dla tego wydania zbieżnych z GitHub API.".to_string(),
        );
    }

    latest.assets = verified_assets;

    Ok(latest)
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

/// Pobiera plik do pamięci (używane dla manifestu i podpisu).
pub async fn download_to_memory(url: &str, max_size: usize) -> Result<Vec<u8>, String> {
    let client = crate::core::http::http_client_with_timeout(30);

    let mut resp = client
        .get(url)
        .header("User-Agent", "SPLogbook-update-check")
        .send()
        .await
        .map_err(|e| format!("Błąd pobierania url: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Serwer zwrócił status {}.", resp.status()));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Błąd chunk: {e}"))? {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > max_size {
            return Err("Plik przekracza dozwolony limit wielkości.".to_string());
        }
    }

    Ok(bytes)
}

/// Pobiera i weryfikuje manifest, implementując zabezpieczenia kryptograficzne i downgrade protection.
pub async fn fetch_and_verify_manifest(
    release: &LatestRelease,
    current_version: &str,
) -> Result<ReleaseManifest, String> {
    let manifest_asset = release
        .assets
        .iter()
        .find(|a| a.name == "release-manifest.json")
        .ok_or_else(|| "Brak release-manifest.json w wydaniu.".to_string())?;

    let sig_asset = release
        .assets
        .iter()
        .find(|a| a.name == "release-manifest.json.sig")
        .ok_or_else(|| "Brak podpisu release-manifest.json.sig w wydaniu.".to_string())?;

    // Limit 1MB na manifest i podpis (ochrona pamięci)
    let manifest_bytes =
        download_to_memory(&manifest_asset.browser_download_url, 1024 * 1024).await?;
    let sig_bytes = download_to_memory(&sig_asset.browser_download_url, 1024 * 1024).await?;
    let sig_str = String::from_utf8(sig_bytes)
        .map_err(|_| "Podpis nie jest prawidłowym ciągiem UTF-8".to_string())?;

    // CRYPTO-ED25519-VERIFY & CRYPTO-MANIFEST-VALIDATE
    let (_, manifest) = verify_manifest_signature(&manifest_bytes, sig_str.trim())?;

    // CRYPTO-SEMVER-POLICY & CRYPTO-DOWNGRADE-PROTECTION
    verify_downgrade_protection(&manifest.version, current_version)?;

    Ok(manifest)
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
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Błąd tworzenia katalogu {}: {e}", parent.display()))?;
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
    let name_lower = asset.name.to_lowercase();

    #[cfg(target_os = "windows")]
    {
        if name_lower.ends_with(".msi") {
            return Err(
                "Błąd: Pobrany plik to instalator MSI. Zaktualizuj program ręcznie.".to_string(),
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if name_lower.ends_with(".deb") || name_lower.ends_with(".rpm") {
            return Err("Błąd: Pobrany plik to zarządzany pakiet systemowy. Zaktualizuj program używając menedżera pakietów.".to_string());
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
    install_via_tar_gz(current, new)
}

#[cfg(not(target_os = "windows"))]
fn install_via_tar_gz(current: &Path, new: &Path) -> Result<(), String> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let base_dir = current
        .parent()
        .ok_or_else(|| "Brak katalogu nadrzędnego dla pliku wykonywalnego".to_string())?;

    // 1. Sprawdzenie praw dostępu (Target Validation)
    let md =
        fs::metadata(base_dir).map_err(|e| format!("Brak dostępu do katalogu aplikacji: {e}"))?;
    if md.permissions().readonly() {
        return Err("Katalog aplikacji jest chroniony przed zapisem (np. zainstalowano przez root). Zaktualizuj program używając menedżera pakietów lub jako root.".into());
    }

    let staging_dir = base_dir.join(".staging");
    let rollback_dir = base_dir.join(".rollback");
    let health_ok = base_dir.join(".health_ok");

    // Czyszczenie starego staging/rollback
    let _ = fs::remove_dir_all(&staging_dir);
    let _ = fs::remove_dir_all(&rollback_dir);
    let _ = fs::remove_file(&health_ok);

    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu tymczasowego: {e}"))?;
    fs::create_dir_all(&rollback_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu kopii zapasowej: {e}"))?;

    // 2. Ekstrakcja poza strefą działania (Staging / ZipSlip Protection)
    let tar_gz =
        fs::File::open(new).map_err(|e| format!("Nie można otworzyć pobranego archiwum: {e}"))?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    for file in archive
        .entries()
        .map_err(|e| format!("Błąd odczytu wpisów w archiwum: {e}"))?
    {
        let mut file = file.map_err(|e| format!("Błąd wpisu w archiwum: {e}"))?;
        let path = file
            .path()
            .map_err(|e| format!("Błąd ścieżki w archiwum: {e}"))?;

        let path_str = path.to_string_lossy();
        if path_str.contains("..") || path_str.starts_with('/') {
            return Err(format!(
                "Archiwum zawiera potencjalnie niebezpieczną ścieżkę (ZipSlip): {}",
                path_str
            ));
        }

        let out_path = staging_dir.join(&path);

        // Zignoruj wpisy niewspierane
        if file.header().entry_type() != tar::EntryType::Regular
            && file.header().entry_type() != tar::EntryType::Directory
        {
            continue;
        }

        if let Some(p) = out_path.parent() {
            let _ = fs::create_dir_all(p);
        }

        file.unpack(&out_path)
            .map_err(|e| format!("Błąd rozpakowywania pliku {}: {}", path_str, e))?;
    }

    // 3. Kopia Zapasowa (Rollback)
    for entry in walkdir::WalkDir::new(&staging_dir).min_depth(1) {
        let entry = entry.map_err(|e| format!("Błąd przeszukiwania plików instalacyjnych: {e}"))?;
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(&staging_dir).unwrap();
            let orig = base_dir.join(rel);
            if orig.exists() {
                let dest = rollback_dir.join(rel);
                if let Some(p) = dest.parent() {
                    let _ = fs::create_dir_all(p);
                }
                fs::copy(&orig, &dest).map_err(|e| {
                    format!(
                        "Nie można utworzyć kopii zapasowej {}: {}",
                        rel.display(),
                        e
                    )
                })?;
            }
        }
    }

    // 4. Atomowa podmiana - Rename Swap
    for entry in walkdir::WalkDir::new(&staging_dir).min_depth(1) {
        let entry = entry.map_err(|e| format!("Błąd przeszukiwania plików instalacyjnych: {e}"))?;
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(&staging_dir).unwrap();
            let orig = base_dir.join(rel);
            if let Some(p) = orig.parent() {
                let _ = fs::create_dir_all(p);
            }
            fs::rename(entry.path(), &orig)
                .map_err(|e| format!("Błąd podmiany pliku {}: {}", rel.display(), e))?;
        }
    }

    // 5. Health Confirmation & Ping
    let mut child = std::process::Command::new(current)
        .arg("--check-health-startup")
        .spawn()
        .map_err(|e| format!("Nie można uruchomić zaktualizowanego programu: {}", e))?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut success = false;

    while std::time::Instant::now() < deadline {
        if health_ok.exists() {
            success = true;
            break;
        }
        if let Ok(Some(_)) = child.try_wait() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    if success {
        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_dir_all(&rollback_dir);
        let _ = fs::remove_file(&health_ok);
        std::process::exit(0); // Pomyślnie. Zakończ stary proces.
    } else {
        // Rollback!
        let _ = child.kill();
        for entry in walkdir::WalkDir::new(&rollback_dir).min_depth(1) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    let rel = entry.path().strip_prefix(&rollback_dir).unwrap();
                    let orig = base_dir.join(rel);
                    let _ = fs::rename(entry.path(), &orig);
                }
            }
        }
        let _ = std::process::Command::new(current).spawn(); // Odpal starą
        return Err("Zaktualizowana aplikacja uległa natychmiastowej awarii (segfault/panic). Przywrócono starszą wersję.".into());
    }
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
         \n\
         $baseDir = Split-Path -Path $exe -Parent\n\
         $staging = Join-Path $baseDir '.staging'\n\
         $rollback = Join-Path $baseDir '.rollback'\n\
         $healthOk = Join-Path $baseDir '.health_ok'\n\
         \n\
         if (Test-Path $staging) {{ Remove-Item -Recurse -Force $staging }}\n\
         New-Item -ItemType Directory -Force -Path $staging | Out-Null\n\
         Expand-Archive -LiteralPath $new -DestinationPath $staging -Force\n\
         \n\
         if (Test-Path $rollback) {{ Remove-Item -Recurse -Force $rollback }}\n\
         New-Item -ItemType Directory -Force -Path $rollback | Out-Null\n\
         \n\
         $deadline = (Get-Date).AddSeconds(90)\n\
         while (Get-Process -Id $pidToWait -ErrorAction SilentlyContinue) {{\n\
             if ((Get-Date) -gt $deadline) {{ exit 1 }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         Start-Sleep -Milliseconds 500\n\
         \n\
         Get-ChildItem -Path $staging -Recurse -File | ForEach-Object {{\n\
             $rel = $_.FullName.Substring($staging.Length + 1)\n\
             $orig = Join-Path $baseDir $rel\n\
             if (Test-Path $orig) {{\n\
                 $dest = Join-Path $rollback $rel\n\
                 $destDir = Split-Path $dest -Parent\n\
                 if (-not (Test-Path $destDir)) {{ New-Item -ItemType Directory -Force -Path $destDir | Out-Null }}\n\
                 Copy-Item -LiteralPath $orig -Destination $dest -Force\n\
             }}\n\
         }}\n\
         \n\
         Copy-Item -Path \"$staging\\*\" -Destination $baseDir -Recurse -Force\n\
         \n\
         if (Test-Path $healthOk) {{ Remove-Item -Force $healthOk }}\n\
         \n\
         $newProc = Start-Process -FilePath $exe -ArgumentList \"--check-health-startup\" -PassThru\n\
         \n\
         $healthDeadline = (Get-Date).AddSeconds(15)\n\
         $success = $false\n\
         while ((Get-Date) -lt $healthDeadline) {{\n\
             if (Test-Path $healthOk) {{\n\
                 $success = $true\n\
                 break\n\
             }}\n\
             if ($newProc.HasExited) {{\n\
                 break\n\
             }}\n\
             Start-Sleep -Milliseconds 250\n\
         }}\n\
         \n\
         if ($success) {{\n\
             Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue\n\
             Remove-Item -Recurse -Force $rollback -ErrorAction SilentlyContinue\n\
             Remove-Item -Force $healthOk -ErrorAction SilentlyContinue\n\
         }} else {{\n\
             Stop-Process -Id $newProc.Id -Force -ErrorAction SilentlyContinue\n\
             Copy-Item -Path \"$rollback\\*\" -Destination $baseDir -Recurse -Force\n\
             Start-Process -FilePath $exe\n\
         }}\n"
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
    fn downgrade_protection_blocks_older_and_equal_versions() {
        assert!(verify_downgrade_protection("1.2.0", "1.1.0").is_ok()); // Newer
        assert!(verify_downgrade_protection("2.0.0", "1.9.9").is_ok()); // Newer

        assert!(verify_downgrade_protection("1.1.0", "1.1.0").is_err()); // Equal
        assert!(verify_downgrade_protection("1.0.9", "1.1.0").is_err()); // Older
    }

    #[test]
    fn manifest_validation_rejects_invalid_values() {
        let mut m = ReleaseManifest {
            manifest_version: 1,
            key_id: "test".into(),
            product: "SPLogbook".into(),
            version: "1.0".into(),
            channel: "stable".into(),
            minimum_updater_version: "1.0".into(),
            commit: "a1b2c3d4e5f6e7f8a9b0c1d2e3f4a5b6c7d8e9f0".into(),
            assets: vec![ManifestAsset {
                platform: "linux".into(),
                arch: "x86_64".into(),
                package_type: "tar.gz".into(),
                filename: "file.tar.gz".into(),
                size: 100,
                sha256: "3d5f0e4c2f76c58916ec258f246851bea091d14d4247a2fc3e18694461b1816e".into(),
            }],
        };

        assert!(m.validate().is_ok());

        m.manifest_version = 2;
        assert!(m.validate().is_err());
        m.manifest_version = 1;

        m.product = "Other".into();
        assert!(m.validate().is_err());
        m.product = "SPLogbook".into();

        m.commit = "short".into();
        assert!(m.validate().is_err());
        m.commit = "a1b2c3d4e5f6e7f8a9b0c1d2e3f4a5b6c7d8e9f0".into();

        m.assets.push(m.assets[0].clone());
        assert!(m.validate().is_err()); // Duplicate filename and triad
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
        assert!(check_safety_gate(&tar_gz).is_ok());

        let deb = ReleaseAsset {
            name: "SPLogbook-Linux-x86_64.deb".into(),
            browser_download_url: "".into(),
            digest: None,
            size: 0,
        };

        #[cfg(not(target_os = "windows"))]
        assert!(check_safety_gate(&deb).is_err());
    }

    #[test]
    fn safety_gate_blocks_windows_archives() {
        let zip = ReleaseAsset {
            name: "SPLogbook-Windows-x64.zip".into(),
            browser_download_url: "".into(),
            digest: None,
            size: 0,
        };

        #[cfg(target_os = "windows")]
        assert!(check_safety_gate(&zip).is_ok());

        let msi = ReleaseAsset {
            name: "SPLogbook-Windows-x64.msi".into(),
            browser_download_url: "".into(),
            digest: None,
            size: 0,
        };
        #[cfg(target_os = "windows")]
        assert!(check_safety_gate(&msi).is_err());

        #[cfg(not(target_os = "windows"))]
        assert!(check_safety_gate(&zip).is_ok());
    }
}
