// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::station::AppConfig;

pub(crate) trait CredentialStore {
    fn put(&self, path: &Path, id: &str, value: &str) -> io::Result<()>;
    fn get(&self, path: &Path, id: &str) -> io::Result<String>;
    fn remove(&self, path: &Path, id: &str) -> io::Result<()>;
}

pub(crate) struct SystemCredentialStore;

impl SystemCredentialStore {
    fn entry(path: &Path, id: &str) -> io::Result<keyring::Entry> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let absolute = std::fs::canonicalize(parent)?;
        let name = path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Nieprawidłowa ścieżka konfiguracji",
            )
        })?;
        let account = format!("{}\\{}:{}", absolute.display(), name.to_string_lossy(), id);
        keyring::Entry::new("SPLogbook", &account).map_err(store_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeStore {
        items: Mutex<HashMap<String, String>>,
        fail_put: bool,
    }

    impl CredentialStore for FakeStore {
        fn put(&self, _path: &Path, id: &str, value: &str) -> io::Result<()> {
            if self.fail_put {
                return Err(io::Error::other("Magazyn niedostępny"));
            }
            self.items
                .lock()
                .unwrap()
                .insert(id.to_string(), value.to_string());
            Ok(())
        }
        fn get(&self, _path: &Path, id: &str) -> io::Result<String> {
            self.items
                .lock()
                .unwrap()
                .get(id)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Brak poświadczeń"))
        }
        fn remove(&self, _path: &Path, id: &str) -> io::Result<()> {
            self.items.lock().unwrap().remove(id);
            Ok(())
        }
    }

    #[test]
    fn migrates_plaintext_and_reloads_without_leaking_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station_config.json");
        let mut legacy = AppConfig::default();
        legacy.lotw_password = "private-lotw-value".into();
        legacy.clublog_api_key = "private-clublog-value".into();
        legacy.lan_sync_secret = "private-lan-value".into();
        std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        // Simulate an older config format containing plaintext credentials.
        let mut data: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        data["lotw_password"] = "private-lotw-value".into();
        data["clublog_api_key"] = "private-clublog-value".into();
        data["lan_sync_secret"] = "private-lan-value".into();
        std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();

        let store = FakeStore::default();
        let loaded = AppConfig::load_with_store(&path, &store).unwrap();
        assert_eq!(loaded.lotw_password, "private-lotw-value");
        assert_eq!(loaded.clublog_api_key, "private-clublog-value");
        assert_eq!(loaded.lan_sync_secret, "private-lan-value");
        let disk = std::fs::read_to_string(&path).unwrap();
        for secret in [
            "private-lotw-value",
            "private-clublog-value",
            "private-lan-value",
        ] {
            assert!(!disk.contains(secret));
        }
        assert_eq!(
            AppConfig::load_with_store(&path, &store)
                .unwrap()
                .lotw_password,
            "private-lotw-value"
        );
        store.items.lock().unwrap().clear();
        assert!(AppConfig::load_with_store(&path, &store).is_err());
    }

    #[test]
    fn failed_migration_does_not_overwrite_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station_config.json");
        let json = r#"{"lotw_password":"legacy-private"}"#;
        std::fs::write(&path, json).unwrap();
        let store = FakeStore {
            fail_put: true,
            ..FakeStore::default()
        };
        assert!(AppConfig::load_with_store(&path, &store).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
    }

    #[test]
    fn save_replaces_and_clears_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station_config.json");
        let store = FakeStore::default();
        let mut config = AppConfig::default();
        config.qrz_api_key = "first-private".into();
        config.save_with_store(&path, &store).unwrap();
        let old_id = config.secret_store_id.clone();
        config.station.callsign = "SP6INA".into();
        config.save_with_store(&path, &store).unwrap();
        assert_eq!(config.secret_store_id, old_id);
        assert_eq!(store.items.lock().unwrap().len(), 1);
        config.qrz_api_key = "second-private".into();
        config.save_with_store(&path, &store).unwrap();
        assert!(store.get(&path, &old_id).is_err());
        assert_eq!(
            AppConfig::load_with_store(&path, &store)
                .unwrap()
                .qrz_api_key,
            "second-private"
        );
        config.qrz_api_key.clear();
        config.save_with_store(&path, &store).unwrap();
        assert!(config.secret_store_id.is_empty());
        assert!(store.items.lock().unwrap().is_empty());
    }

    #[test]
    fn every_secret_is_absent_from_config_and_restored_from_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station_config.json");
        let store = FakeStore::default();
        let mut config = AppConfig::default();
        config.lan_sync_secret = "secret-01".into();
        config.lotw_password = "secret-02".into();
        config.qrz_password = "secret-03".into();
        config.qrz_api_key = "secret-04".into();
        config.eqsl_password = "secret-05".into();
        config.clublog_password = "secret-06".into();
        config.clublog_api_key = "secret-07".into();
        config.cloudlog_api_key = "secret-08".into();
        config.rest_api_key = "secret-09".into();
        config.hrdlog_upload_code = "secret-10".into();
        config.hamqth_password = "secret-11".into();
        config.save_with_store(&path, &store).unwrap();
        let disk = std::fs::read_to_string(&path).unwrap();
        for index in 1..=11 {
            assert!(!disk.contains(&format!("secret-{index:02}")));
        }
        let loaded = AppConfig::load_with_store(&path, &store).unwrap();
        assert_eq!(
            serde_json::to_value(Secrets::from_config(&config)).unwrap(),
            serde_json::to_value(Secrets::from_config(&loaded)).unwrap()
        );
    }

    #[test]
    fn protected_config_never_silently_uses_plaintext_or_missing_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station_config.json");
        let store = FakeStore::default();
        let mut config = AppConfig::default();
        config.qrz_api_key = "stored-private".into();
        config.save_with_store(&path, &store).unwrap();

        let original = std::fs::read(&path).unwrap();
        store.items.lock().unwrap().clear();
        assert!(AppConfig::load_with_store(&path, &store).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);

        let mut data: serde_json::Value = serde_json::from_slice(&original).unwrap();
        data["qrz_api_key"] = "legacy-private".into();
        std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        assert!(AppConfig::load_with_store(&path, &store).is_err());
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("legacy-private"));
    }
}

fn store_error(error: keyring::Error) -> io::Error {
    io::Error::other(format!("Systemowy magazyn poświadczeń: {error}"))
}

impl CredentialStore for SystemCredentialStore {
    fn put(&self, path: &Path, id: &str, value: &str) -> io::Result<()> {
        Self::entry(path, id)?
            .set_password(value)
            .map_err(store_error)
    }

    fn get(&self, path: &Path, id: &str) -> io::Result<String> {
        Self::entry(path, id)?.get_password().map_err(store_error)
    }

    fn remove(&self, path: &Path, id: &str) -> io::Result<()> {
        Self::entry(path, id)?
            .delete_credential()
            .map_err(store_error)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct Secrets {
    lan_sync_secret: String,
    lotw_password: String,
    qrz_password: String,
    qrz_api_key: String,
    eqsl_password: String,
    clublog_password: String,
    clublog_api_key: String,
    cloudlog_api_key: String,
    rest_api_key: String,
    hrdlog_upload_code: String,
    hamqth_password: String,
}

impl Secrets {
    fn from_config(c: &AppConfig) -> Self {
        Self {
            lan_sync_secret: c.lan_sync_secret.clone(),
            lotw_password: c.lotw_password.clone(),
            qrz_password: c.qrz_password.clone(),
            qrz_api_key: c.qrz_api_key.clone(),
            eqsl_password: c.eqsl_password.clone(),
            clublog_password: c.clublog_password.clone(),
            clublog_api_key: c.clublog_api_key.clone(),
            cloudlog_api_key: c.cloudlog_api_key.clone(),
            rest_api_key: c.rest_api_key.clone(),
            hrdlog_upload_code: c.hrdlog_upload_code.clone(),
            hamqth_password: c.hamqth_password.clone(),
        }
    }

    fn is_empty(&self) -> bool {
        [
            &self.lan_sync_secret,
            &self.lotw_password,
            &self.qrz_password,
            &self.qrz_api_key,
            &self.eqsl_password,
            &self.clublog_password,
            &self.clublog_api_key,
            &self.cloudlog_api_key,
            &self.rest_api_key,
            &self.hrdlog_upload_code,
            &self.hamqth_password,
        ]
        .iter()
        .all(|value| value.is_empty())
    }

    fn apply(self, c: &mut AppConfig) {
        c.lan_sync_secret = self.lan_sync_secret;
        c.lotw_password = self.lotw_password;
        c.qrz_password = self.qrz_password;
        c.qrz_api_key = self.qrz_api_key;
        c.eqsl_password = self.eqsl_password;
        c.clublog_password = self.clublog_password;
        c.clublog_api_key = self.clublog_api_key;
        c.cloudlog_api_key = self.cloudlog_api_key;
        c.rest_api_key = self.rest_api_key;
        c.hrdlog_upload_code = self.hrdlog_upload_code;
        c.hamqth_password = self.hamqth_password;
    }
}

impl AppConfig {
    pub(crate) fn load_with_store(path: &Path, store: &impl CredentialStore) -> io::Result<Self> {
        let data = match std::fs::read_to_string(path) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        let mut config: Self = serde_json::from_str(&data).map_err(io::Error::other)?;
        if !config.secret_store_id.is_empty() {
            if !Secrets::from_config(&config).is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Konfiguracja zawiera jednocześnie jawne poświadczenia i identyfikator magazynu",
                ));
            }
            let saved = store.get(path, &config.secret_store_id)?;
            let secrets: Secrets = serde_json::from_str(&saved).map_err(io::Error::other)?;
            secrets.apply(&mut config);
        } else if !Secrets::from_config(&config).is_empty() {
            config.save_with_store(path, store)?;
        }
        Ok(config)
    }

    pub(crate) fn save_with_store(
        &mut self,
        path: &Path,
        store: &impl CredentialStore,
    ) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let secrets = Secrets::from_config(self);
        let old_id = self.secret_store_id.clone();
        let unchanged = if old_id.is_empty() {
            false
        } else {
            let saved = store.get(path, &old_id)?;
            let previous: Secrets = serde_json::from_str(&saved).map_err(io::Error::other)?;
            previous == secrets
        };
        let new_id = if unchanged {
            old_id.clone()
        } else if secrets.is_empty() {
            String::new()
        } else {
            uuid::Uuid::new_v4().to_string()
        };
        if !new_id.is_empty() && !unchanged {
            store.put(
                path,
                &new_id,
                &serde_json::to_string(&secrets).map_err(io::Error::other)?,
            )?;
        }

        self.secret_store_id = new_id.clone();
        let result = (|| {
            let data = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
            let tmp_path = path.with_extension("json.tmp");
            std::fs::write(&tmp_path, data)?;
            std::fs::rename(&tmp_path, path)
        })();
        if let Err(e) = result {
            self.secret_store_id = old_id;
            if !new_id.is_empty() && !unchanged {
                if let Err(cleanup) = store.remove(path, &new_id) {
                    log::error!("Nie udało się usunąć nieużywanych poświadczeń: {cleanup}");
                }
            }
            return Err(e);
        }
        if !old_id.is_empty() && old_id != new_id {
            if let Err(e) = store.remove(path, &old_id) {
                log::warn!(
                    "Konfiguracja zapisana; nie udało się usunąć starego wpisu poświadczeń: {e}"
                );
            }
        }
        Ok(())
    }
}
