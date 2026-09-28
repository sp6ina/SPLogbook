// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Marketplace pluginów: katalog, pobieranie, instalacja i odinstalowywanie.
//!
//! Katalog może pochodzić ze zdalnego pliku `catalog.json` (GitHub raw) lub —
//! w trybie offline — z wbudowanego katalogu zapasowego. Instalacja zapisuje
//! skrypt `.rhai` do katalogu pluginów użytkownika, weryfikuje sumę kontrolną
//! SHA256 (gdy jest dostępna) i zapisuje boczny manifest `{id}.json` z wersją,
//! dzięki czemu aplikacja rozpoznaje zainstalowane wtyczki i dostępne aktualizacje.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Zdalny plik katalogu (spójny z kanałem auto-update w `cloud/updater.rs`).
const REMOTE_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/sp6ina/SPLogbook/main/plugins/catalog.json";

/// Pojedynczy wpis katalogu marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCatalogEntry {
    /// Unikalny identyfikator (bez rozszerzenia), np. `pota-helper`.
    pub id: String,
    /// Czytelna nazwa wyświetlana w UI.
    pub name: String,
    pub description: String,
    /// Wersja semantyczna, np. `1.0.0`.
    pub version: String,
    pub author: String,
    /// Kategoria (filtrowanie w UI): `POTA`, `SOTA`, `CW`, `Contest`, `Rotor`, `Awards`, `DX`, `Digital`.
    pub category: String,
    /// Emoji/ikona katalogu.
    pub icon: String,
    /// Nazwa pliku skryptu `.rhai` (np. `pota_helper.rhai`).
    pub file: String,
    /// Adres pobrania skryptu (dla katalogu zdalnego).
    #[serde(default)]
    pub download_url: String,
    /// Oczekiwana suma kontrolna SHA256 (hex). Weryfikowana przy pobieraniu.
    #[serde(default)]
    pub sha256: Option<String>,
    /// Minimalna wersja SPLogbook wymagana przez wtyczkę.
    #[serde(default)]
    pub min_version: Option<String>,
    /// Wbudowane źródło skryptu (tylko katalog zapasowy; nie serializowane).
    #[serde(skip)]
    pub source: Option<String>,
}

impl PluginCatalogEntry {
    /// Buduje wpis katalogu z wbudowanym źródłem (tryb offline).
    fn bundled(
        id: &str,
        name: &str,
        description: &str,
        version: &str,
        author: &str,
        category: &str,
        icon: &str,
        file: &str,
        source: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            version: version.to_string(),
            author: author.to_string(),
            category: category.to_string(),
            icon: icon.to_string(),
            file: file.to_string(),
            download_url: String::new(),
            sha256: Some(crate::cloud::updater::sha256_hex(source.as_bytes())),
            min_version: None,
            source: Some(source.to_string()),
        }
    }
}

/// Stan instalacji wtyczki względem katalogu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStatus {
    NotInstalled,
    Installed { version: String },
    UpdateAvailable { installed: String, latest: String },
}

/// Wbudowany katalog zapasowy (offline). Skrypty są w pełni sandboxowane.
pub fn bundled_catalog() -> Vec<PluginCatalogEntry> {
    vec![
        PluginCatalogEntry::bundled(
            "pota-helper",
            "POTA Helper",
            "Rejestruje aktywacje Parks on the Air i przypomina o referencjach POTA.",
            "1.0.0",
            "SPLogbook",
            "POTA",
            "🌲",
            "pota_helper.rhai",
            r#"
// POTA Helper — powiadamia o łącznościach z referencją POTA.
fn on_startup() {
    log("POTA Helper gotowy.");
}

fn on_qso_logged(call_sign, band, mode, freq, atno) {
    let reference = qso_field("pota_ref");
    if reference != "" {
        notify("POTA: " + call_sign + " @ " + reference);
        log("POTA QSO: " + call_sign + " -> " + reference);
    }
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "sota-helper",
            "SOTA Helper",
            "Śledzi szczyty Summits on the Air i zapisuje referencje SOTA.",
            "1.0.0",
            "SPLogbook",
            "SOTA",
            "⛰",
            "sota_helper.rhai",
            r#"
// SOTA Helper — powiadamia o łącznościach z referencją SOTA.
fn on_startup() {
    log("SOTA Helper gotowy.");
}

fn on_qso_logged(call_sign, band, mode, freq, atno) {
    let reference = qso_field("sota_ref");
    if reference != "" {
        notify("SOTA: " + call_sign + " @ " + reference);
        log("SOTA QSO: " + call_sign + " -> " + reference);
    }
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "cw-macros",
            "CW Macros",
            "Automatyczne makra telegraficzne po zapisaniu łączności.",
            "1.0.0",
            "SPLogbook",
            "CW",
            "⚡",
            "cw_macros.rhai",
            r#"
// CW Macros — automatyczne nadanie potwierdzenia po QSO.
fn on_startup() {
    log("CW Macros gotowe.");
}

fn on_qso_logged(call_sign, band, mode, freq, atno) {
    if mode == "CW" {
        send_cw(call_sign + " TU " + my_call() + " " + qso_field("rst_sent") + " 73");
    }
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "contest-assistant",
            "Contest Assistant",
            "Wspomaga zawody: raporty, numeracja i log kontestowy.",
            "1.0.0",
            "SPLogbook",
            "Contest",
            "🏁",
            "contest_assistant.rhai",
            r#"
// Contest Assistant — automatyczny raport kontestowy z numerem.
fn on_startup() {
    log("Contest Assistant gotowy.");
}

fn on_qso_logged(call_sign, band, mode, freq, atno) {
    log("Kontest: " + call_sign + " na " + band + " (" + mode + ")");
    if mode == "CW" {
        send_cw("5NN %SERIAL%");
    }
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "rotor-assistant",
            "Rotor Assistant",
            "Obraca antenę i reaguje na spoty DX.",
            "1.0.0",
            "SPLogbook",
            "Rotor",
            "🧭",
            "rotor_assistant.rhai",
            r#"
// Rotor Assistant — obrót anteny na spot i pomocnicze komendy.
fn on_startup() {
    log("Rotor Assistant gotowy.");
}

fn on_dx_spot(spotter, dx_call, freq_khz, band, comment, is_ft8) {
    log("Spot DX: " + dx_call + " na " + band + " (od " + spotter + ")");
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "award-tracker",
            "Award Tracker",
            "Monitoruje postępy DXCC, WAZ, WAS, WAC, IOTA i PGA na żywo.",
            "1.0.0",
            "SPLogbook",
            "Awards",
            "🏆",
            "award_tracker.rhai",
            r#"
// Award Tracker — powiadamia o nowych krajach i podaje statystyki nagród.
fn on_startup() {
    log("Award Tracker gotowy.");
    log("DXCC worked: " + dxcc_worked() + ", potwierdzone: " + dxcc_confirmed());
}

fn on_qso_logged(call_sign, band, mode, freq, atno) {
    if atno {
        notify("🏆 ATNO! Nowy kraj DXCC: " + call_sign);
        play_sound("new_dxcc");
    }
    log("DXCC: " + dxcc_worked() + " worked / " + dxcc_confirmed() + " confirmed");
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "dx-spot-alerts",
            "DX Spot Alerts (ATNO)",
            "Alarmuje o nowych krajach i pasmach na klastrze DX.",
            "1.0.0",
            "SPLogbook",
            "DX",
            "📡",
            "dx_spot_alerts.rhai",
            r#"
// DX Spot Alerts — loguje spoty klastra.
fn on_startup() {
    log("DX Spot Alerts gotowe.");
}

fn on_dx_spot(spotter, dx_call, freq_khz, band, comment, is_ft8) {
    log("Spot: " + dx_call + " @" + band);
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "propagation-watchdog",
            "Propagation Watchdog",
            "Powiadamia o otwarciach pasm.",
            "1.0.0",
            "SPLogbook",
            "DX",
            "☀",
            "propagation_watchdog.rhai",
            r#"
// Propagation Watchdog — reaguje na otwarcia pasm.
fn on_startup() {
    log("Propagation Watchdog gotowy.");
}

fn on_band_opened(band) {
    notify("📡 Otwarcie pasma: " + band);
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "band-activity-logger",
            "Band Activity Logger",
            "Zapisuje aktywność klastra z podziałem na pasma.",
            "1.0.0",
            "SPLogbook",
            "DX",
            "📊",
            "band_activity_logger.rhai",
            r#"
// Band Activity Logger — log aktywności klastra.
fn on_startup() {
    log("Band Activity Logger gotowy.");
}

fn on_dx_spot(spotter, dx_call, freq_khz, band, comment, is_ft8) {
    log(band + ": " + dx_call + " (" + spotter + ")");
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "qsl-reminder",
            "QSL Reminder",
            "Przypomina o wysyłce potwierdzeń QSL.",
            "1.0.0",
            "SPLogbook",
            "Awards",
            "📬",
            "qsl_reminder.rhai",
            r#"
// QSL Reminder — przypomina o potwierdzeniach po każdej łączności.
fn on_startup() {
    log("QSL Reminder gotowy.");
}

fn on_qso_logged(call_sign, band, mode, freq, atno) {
    notify("Pamiętaj o wysłaniu QSL do " + call_sign);
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "voice-keyer-trigger",
            "Voice Keyer Trigger",
            "Odtwarza wiadomości głosowe po QSO.",
            "1.0.0",
            "SPLogbook",
            "CW",
            "🎙",
            "voice_keyer_trigger.rhai",
            r#"
// Voice Keyer Trigger — odtwórz podziękowanie po QSO.
fn on_startup() {
    log("Voice Keyer Trigger gotowy.");
}

fn on_qso_logged(call_sign, band, mode, freq, atno) {
    send_voice("Dzięki za łączność, 73!");
}
"#,
        ),
        PluginCatalogEntry::bundled(
            "ft8-wsjt-bridge",
            "FT8/WSJT-X Bridge",
            "Reaguje na zmiany stanu radia przy pracy FT8.",
            "1.0.0",
            "SPLogbook",
            "Digital",
            "💻",
            "ft8_wsjt_bridge.rhai",
            r#"
// FT8/WSJT-X Bridge — monitor stanu radia.
fn on_startup() {
    log("FT8/WSJT-X Bridge gotowy.");
}

fn on_rig_state(freq_mhz, mode, connected) {
    log("Rig: " + mode + " @ " + freq_mhz + " MHz, połączony: " + connected);
}
"#,
        ),
    ]
}

/// Parsuje katalog ze zdalnego JSON (format zgodny z [`PluginCatalogEntry`]).
pub fn parse_catalog(json: &str) -> Result<Vec<PluginCatalogEntry>, String> {
    serde_json::from_str::<Vec<PluginCatalogEntry>>(json)
        .map_err(|e| format!("Błąd parsowania katalogu: {e}"))
}

/// Pobiera katalog: najpierw zdalny, a przy błędzie zwraca wbudowany zapasowy.
pub async fn fetch_catalog() -> Result<Vec<PluginCatalogEntry>, String> {
    match fetch_remote_catalog().await {
        Ok(entries) if !entries.is_empty() => Ok(entries),
        _ => Ok(bundled_catalog()),
    }
}

/// Pobiera i parsuje zdalny katalog (bez fallbacku).
async fn fetch_remote_catalog() -> Result<Vec<PluginCatalogEntry>, String> {
    let client = crate::core::http::http_client_with_timeout(15);
    let resp = client
        .get(REMOTE_CATALOG_URL)
        .header("User-Agent", crate::core::http::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Błąd pobierania katalogu: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Serwer zwrócił status {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    parse_catalog(&body)
}

/// Waliduje nazwę pliku/identyfikator pod kątem prób wyjścia poza katalog (`Path Traversal`).
fn validate_safe_filename(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
    {
        return Err(format!("Nieprawidłowa nazwa pliku wtyczki: {name}"));
    }
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(()),
        _ => Err(format!("Niedozwolona ścieżka wtyczki: {name}")),
    }
}

/// Ścieżka bocznego manifestu instalacji dla wtyczki.
fn manifest_path(plugins_dir: &Path, id: &str) -> PathBuf {
    plugins_dir.join(format!("{id}.json"))
}

/// Ścieżka docelowego skryptu `.rhai` dla wtyczki.
fn script_path(plugins_dir: &Path, entry: &PluginCatalogEntry) -> PathBuf {
    plugins_dir.join(&entry.file)
}

/// Manifest boczny (wersja + suma kontrolna + data instalacji).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledManifest {
    pub id: String,
    pub version: String,
    pub sha256: Option<String>,
}

/// Odczytuje wersję zainstalowanej wtyczki (lub `None`).
pub fn read_installed_version(plugins_dir: &Path, id: &str) -> Option<String> {
    validate_safe_filename(id).ok()?;
    let raw = std::fs::read_to_string(manifest_path(plugins_dir, id)).ok()?;
    serde_json::from_str::<InstalledManifest>(&raw)
        .ok()
        .map(|m| m.version)
}

/// Określa stan instalacji wtyczki względem katalogu.
pub fn install_status(plugins_dir: &Path, entry: &PluginCatalogEntry) -> InstallStatus {
    match read_installed_version(plugins_dir, &entry.id) {
        None => InstallStatus::NotInstalled,
        Some(current) if current == entry.version => InstallStatus::Installed { version: current },
        Some(current) => InstallStatus::UpdateAvailable {
            installed: current,
            latest: entry.version.clone(),
        },
    }
}

/// Pobiera (lub bierze wbudowane) źródło wtyczki i weryfikuje SHA256.
async fn obtain_source(entry: &PluginCatalogEntry) -> Result<Vec<u8>, String> {
    if let Some(source) = &entry.source {
        return Ok(source.as_bytes().to_vec());
    }

    if entry.download_url.is_empty() {
        return Err("Brak adresu pobrania dla wtyczki.".to_string());
    }

    let client = crate::core::http::http_client_with_timeout(60);
    let resp = client
        .get(&entry.download_url)
        .header("User-Agent", crate::core::http::USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Błąd pobierania wtyczki: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Serwer zwrócił status {}", resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();

    // Zdalna zawartość musi mieć obowiązkową sumę kontrolną — bez niej nie
    // instalujemy, aby nie dopuścić do wykonania niezweryfikowanego kodu.
    let expected = entry.sha256.as_deref().ok_or_else(|| {
        "Wtyczka nie zawiera sumy kontrolnej SHA256 — odmowa instalacji.".to_string()
    })?;
    if !crate::cloud::updater::verify_sha256(&bytes, expected) {
        return Err("Suma kontrolna SHA256 pobranej wtyczki nie zgadza się.".to_string());
    }

    Ok(bytes)
}

/// Instaluje wtyczkę do katalogu pluginów. Zwraca ścieżkę zapisanego skryptu.
pub async fn install_entry(
    entry: &PluginCatalogEntry,
    plugins_dir: &Path,
) -> Result<PathBuf, String> {
    validate_safe_filename(&entry.id)?;
    validate_safe_filename(&entry.file)?;

    let bytes = obtain_source(entry).await?;

    let sha256 = crate::cloud::updater::sha256_hex(&bytes);
    // Obowiązkowa weryfikacja sumy kontrolnej — wtyczki bez skrótu nie są instalowane.
    let expected = entry.sha256.as_deref().ok_or_else(|| {
        "Wtyczka nie zawiera sumy kontrolnej SHA256 — odmowa instalacji.".to_string()
    })?;
    if !crate::cloud::updater::verify_sha256(&bytes, expected) {
        return Err("Suma kontrolna SHA256 wtyczki nie zgadza się.".to_string());
    }

    if let Some(parent) = plugins_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::create_dir_all(plugins_dir)
        .map_err(|e| format!("Nie można utworzyć katalogu pluginów: {e}"))?;

    let script = script_path(plugins_dir, entry);
    std::fs::write(&script, &bytes)
        .map_err(|e| format!("Błąd zapisu skryptu {}: {e}", script.display()))?;

    let manifest = InstalledManifest {
        id: entry.id.clone(),
        version: entry.version.clone(),
        sha256: Some(sha256),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Błąd serializacji manifestu: {e}"))?;
    std::fs::write(manifest_path(plugins_dir, &entry.id), manifest_json)
        .map_err(|e| format!("Błąd zapisu manifestu: {e}"))?;

    Ok(script)
}

/// Odinstalowuje wtyczkę (usuwa skrypt i manifest boczny).
pub fn uninstall_entry(plugins_dir: &Path, entry: &PluginCatalogEntry) -> Result<(), String> {
    validate_safe_filename(&entry.id)?;
    validate_safe_filename(&entry.file)?;

    let script = script_path(plugins_dir, entry);
    if script.exists() {
        std::fs::remove_file(&script)
            .map_err(|e| format!("Błąd usuwania skryptu {}: {e}", script.display()))?;
    }
    let manifest = manifest_path(plugins_dir, &entry.id);
    if manifest.exists() {
        let _ = std::fs::remove_file(&manifest);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "splogbook_marketplace_{}_{}_{}",
            name,
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn bundled_catalog_contains_named_plugins() {
        let catalog = bundled_catalog();
        let ids: Vec<&str> = catalog.iter().map(|e| e.id.as_str()).collect();
        for expected in [
            "pota-helper",
            "sota-helper",
            "cw-macros",
            "contest-assistant",
            "rotor-assistant",
            "award-tracker",
        ] {
            assert!(ids.contains(&expected), "brak wtyczki {expected}");
        }
        // Każdy wpis wbudowany ma źródło i zgodną sumę kontrolną.
        for e in &catalog {
            let src = e.source.as_deref().expect("wbudowane źródło");
            assert!(crate::cloud::updater::verify_sha256(
                src.as_bytes(),
                e.sha256.as_deref().unwrap()
            ));
        }
    }

    #[test]
    fn parse_catalog_accepts_json() {
        let json = r#"[{
            "id": "test-plugin",
            "name": "Test",
            "description": "desc",
            "version": "1.0.0",
            "author": "A",
            "category": "DX",
            "icon": "📡",
            "file": "test.rhai",
            "download_url": "https://example.com/test.rhai",
            "sha256": "abc"
        }]"#;
        let parsed = parse_catalog(json).expect("parse");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "test-plugin");
        assert_eq!(parsed[0].sha256.as_deref(), Some("abc"));
        assert!(parsed[0].source.is_none());
    }

    #[tokio::test]
    async fn install_and_uninstall_bundled_entry() {
        let dir = temp_dir("install");
        let entry = bundled_catalog()
            .into_iter()
            .find(|e| e.id == "pota-helper")
            .unwrap();
        let path = install_entry(&entry, &dir).await.expect("install");
        assert!(path.exists());
        assert_eq!(
            install_status(&dir, &entry),
            InstallStatus::Installed {
                version: "1.0.0".into()
            }
        );

        uninstall_entry(&dir, &entry).expect("uninstall");
        assert!(!path.exists());
        assert_eq!(install_status(&dir, &entry), InstallStatus::NotInstalled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn install_refuses_entry_without_checksum() {
        let dir = temp_dir("no_checksum");
        let entry = PluginCatalogEntry {
            id: "no-sum".to_string(),
            name: "No checksum".to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            author: String::new(),
            category: "DX".to_string(),
            icon: String::new(),
            file: "no_sum.rhai".to_string(),
            download_url: String::new(),
            sha256: None,
            min_version: None,
            source: Some("// brak sumy kontrolnej".to_string()),
        };
        let res = install_entry(&entry, &dir).await;
        assert!(
            res.is_err(),
            "instalacja bez sumy kontrolnej musi zostać odrzucona"
        );
        assert!(res.unwrap_err().contains("SHA256"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_status_detects_update() {
        let dir = temp_dir("status");
        std::fs::create_dir_all(&dir).unwrap();
        let mut entry = bundled_catalog()
            .into_iter()
            .find(|e| e.id == "cw-macros")
            .unwrap();
        entry.version = "9.9.9".to_string();

        // Ręcznie zapisz manifest starszej wersji.
        let manifest = InstalledManifest {
            id: entry.id.clone(),
            version: "1.0.0".to_string(),
            sha256: None,
        };
        std::fs::write(
            manifest_path(&dir, &entry.id),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        assert_eq!(
            install_status(&dir, &entry),
            InstallStatus::UpdateAvailable {
                installed: "1.0.0".into(),
                latest: "9.9.9".into()
            }
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
