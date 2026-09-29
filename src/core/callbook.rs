// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::hamqth::HamQthXmlClient;
use crate::cloud::qrz::{CallbookData, QrzClient};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Źródło danych callbook używane w agregacji z priorytetami.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallbookSource {
    /// Lokalna baza offline (callbook.db + serviceLOG.db).
    Local,
    /// Cache offline wyników wcześniejszych zapytań online.
    Cache,
    /// Darmowe API callook.info (stacje USA z bazy FCC).
    Callook,
    /// HamQTH XML (wymaga loginu/hasła).
    HamQth,
    /// QRZ.com XML (wymaga loginu/hasła).
    Qrz,
}

impl CallbookSource {
    /// Czytelna etykieta źródła (interfejs PL).
    pub fn label(&self) -> &'static str {
        match self {
            CallbookSource::Local => "Lokalna baza (offline)",
            CallbookSource::Cache => "Cache offline",
            CallbookSource::Callook => "Callook.info (USA)",
            CallbookSource::HamQth => "HamQTH",
            CallbookSource::Qrz => "QRZ.com",
        }
    }
}

/// Domyślna kolejność źródeł: najpierw tanie/offline, potem online.
pub fn default_callbook_priority() -> Vec<CallbookSource> {
    vec![
        CallbookSource::Local,
        CallbookSource::Cache,
        CallbookSource::Callook,
        CallbookSource::HamQth,
        CallbookSource::Qrz,
    ]
}

/// Lokalna baza danych Callbook (offline) + cache wyników online
#[derive(Debug, Clone)]
pub struct LocalCallbook {
    callbook_path: Option<PathBuf>,
    servicelog_path: Option<PathBuf>,
    cache_path: Option<PathBuf>,
    cache_ttl_days: u32,
}

impl LocalCallbook {
    pub fn new(callbook_path: Option<PathBuf>, servicelog_path: Option<PathBuf>) -> Self {
        Self {
            callbook_path,
            servicelog_path,
            cache_path: None,
            cache_ttl_days: 30,
        }
    }

    /// Włącza trwały cache offline dla wyników zapytań online.
    pub fn with_cache(mut self, cache_path: Option<PathBuf>, ttl_days: u32) -> Self {
        self.cache_path = cache_path;
        self.cache_ttl_days = ttl_days.max(1);
        self
    }

    /// Pobiera dane z cache offline (jeśli nie starsze niż TTL).
    pub fn cache_lookup(&self, callsign: &str) -> Option<CallbookData> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return None;
        }
        let path = self.cache_path.as_ref()?;
        if !path.exists() {
            return None;
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
        let ttl_secs = u64::from(self.cache_ttl_days) * 86_400;
        let now = unix_now();
        let mut stmt = conn
            .prepare(
                "SELECT Name, QTH, Grid, State, DXCC, Country, Manager, Email, ImageUrl, LastFetched \
                 FROM CallbookCache WHERE Call = ?1 LIMIT 1",
            )
            .ok()?;

        stmt.query_row([&clean], |row| {
            let last_fetched: i64 = row.get(9).unwrap_or(0);
            if last_fetched < 0 || now.saturating_sub(last_fetched as u64) > ttl_secs {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(CallbookData {
                callsign: clean.clone(),
                name: row.get(0).ok().and_then(|s: String| clean_opt_str(&s)),
                qth: row.get(1).ok().and_then(|s: String| clean_opt_str(&s)),
                gridsquare: row.get(2).ok().and_then(|s: String| clean_opt_str(&s)),
                state: row.get(3).ok().and_then(|s: String| clean_opt_str(&s)),
                dxcc: row.get(4).ok(),
                country: row.get(5).ok().and_then(|s: String| clean_opt_str(&s)),
                qsl_manager: row.get(6).ok().and_then(|s: String| clean_opt_str(&s)),
                email: row.get(7).ok().and_then(|s: String| clean_opt_str(&s)),
                image_url: row.get(8).ok().and_then(|s: String| clean_opt_str(&s)),
            })
        })
        .ok()
    }

    /// Zapisuje dane callbook do cache offline (upsert).
    pub fn cache_store(&self, data: &CallbookData) {
        let Some(path) = &self.cache_path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(conn) = Connection::open(path) else {
            return;
        };
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS CallbookCache (
                Call TEXT PRIMARY KEY,
                Name TEXT, QTH TEXT, Grid TEXT, State TEXT,
                DXCC INTEGER, Country TEXT, Manager TEXT, Email TEXT, ImageUrl TEXT,
                LastFetched INTEGER
            )",
            [],
        );
        let _ = conn.execute(
            "INSERT INTO CallbookCache
                (Call, Name, QTH, Grid, State, DXCC, Country, Manager, Email, ImageUrl, LastFetched)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(Call) DO UPDATE SET
                Name = excluded.Name, QTH = excluded.QTH, Grid = excluded.Grid,
                State = excluded.State, DXCC = excluded.DXCC, Country = excluded.Country,
                Manager = excluded.Manager, Email = excluded.Email,
                ImageUrl = excluded.ImageUrl, LastFetched = excluded.LastFetched",
            rusqlite::params![
                data.callsign,
                data.name.as_deref(),
                data.qth.as_deref(),
                data.gridsquare.as_deref(),
                data.state.as_deref(),
                data.dxcc,
                data.country.as_deref(),
                data.qsl_manager.as_deref(),
                data.email.as_deref(),
                data.image_url.as_deref(),
                unix_now() as i64,
            ],
        );
    }

    /// Wyszukuje stację w lokalnej bazie SQLite (callbook.db i serviceLOG.db)
    pub fn lookup(&self, callsign: &str) -> Option<CallbookData> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return None;
        }

        // Pobieramy bazowy znak, np. z DL/SP1ADT bierzemy SP1ADT, z SP1ADT/P bierzemy SP1ADT
        let base_call = extract_base_call(&clean);

        let mut data: Option<CallbookData> = None;

        if let Some(ref path) = self.callbook_path {
            if path.exists() {
                if let Ok(conn) =
                    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                {
                    // Najpierw szukamy dokładnego znaku, a potem bazowego
                    for query_call in &[clean.as_str(), base_call] {
                        let Ok(mut stmt) = conn.prepare(
                            "SELECT Name, QTH, Grid, State, Manager FROM Callbook WHERE Call = ? LIMIT 1"
                        ) else {
                            continue;
                        };

                        let found = stmt
                            .query_row([query_call], |row| {
                                let name: Option<String> =
                                    row.get(0).ok().and_then(|s: String| clean_opt_str(&s));
                                let qth: Option<String> =
                                    row.get(1).ok().and_then(|s: String| clean_opt_str(&s));
                                let grid: Option<String> =
                                    row.get(2).ok().and_then(|s: String| clean_opt_str(&s));
                                let state: Option<String> =
                                    row.get(3).ok().and_then(|s: String| clean_opt_str(&s));
                                let manager: Option<String> =
                                    row.get(4).ok().and_then(|s: String| clean_opt_str(&s));

                                Ok(CallbookData {
                                    callsign: clean.clone(),
                                    name,
                                    qth,
                                    gridsquare: grid,
                                    state,
                                    dxcc: None,
                                    country: None,
                                    qsl_manager: manager,
                                    email: None,
                                    image_url: None,
                                })
                            })
                            .ok();

                        if let Some(res) = found {
                            data = Some(res);
                            break;
                        }
                    }
                }
            }
        }

        // Jeśli brakuje menedżera QSL, odpytujemy bazę serviceLOG.db (tabela managers - 29k wpisów)
        let need_manager = match &data {
            Some(d) => d.qsl_manager.is_none() || d.qsl_manager.as_ref().unwrap().is_empty(),
            None => true,
        };

        if need_manager {
            if let Some(ref path) = self.servicelog_path {
                if path.exists() {
                    if let Ok(conn) =
                        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    {
                        for query_call in &[clean.as_str(), base_call] {
                            let Ok(mut stmt) =
                                conn.prepare("SELECT Manager FROM managers WHERE Call = ? LIMIT 1")
                            else {
                                continue;
                            };

                            let mgr: Option<String> = stmt
                                .query_row([query_call], |row| row.get(0))
                                .ok()
                                .and_then(|s: String| clean_opt_str(&s));

                            if let Some(m) = mgr {
                                if let Some(ref mut d) = data {
                                    d.qsl_manager = Some(m);
                                } else {
                                    data = Some(CallbookData {
                                        callsign: clean.clone(),
                                        name: None,
                                        qth: None,
                                        gridsquare: None,
                                        state: None,
                                        dxcc: None,
                                        country: None,
                                        qsl_manager: Some(m),
                                        email: None,
                                        image_url: None,
                                    });
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        data
    }
}

/// Pomocnik oczyszczający puste ciągi i znaki specjalne
fn clean_opt_str(s: &str) -> Option<String> {
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() || trimmed == "-" || trimmed == "?" {
        None
    } else {
        Some(trimmed)
    }
}

/// Wyciąga bazowy znak z ewentualnych prefiksów/sufiksów
pub fn extract_base_call(call: &str) -> &str {
    let parts: Vec<&str> = call.split('/').collect();
    if parts.len() == 1 {
        return call;
    }
    // Wybieramy najdłuższą część jako właściwy znak stacji
    let mut longest = parts[0];
    for p in &parts[1..] {
        if p.len() > longest.len() {
            longest = p;
        }
    }
    longest
}

/// Klient darmowego API callook.info (stacje USA z bazy FCC)
pub async fn lookup_callook_info(callsign: &str) -> Result<CallbookData, String> {
    let clean = callsign.trim().to_uppercase();
    let url = format!("https://callook.info/{clean}/json");

    let client = crate::core::http::http_client_with_timeout(6);

    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json.get("status").and_then(|v| v.as_str()) != Some("VALID") {
        return Err("Stacja nieznaleziona w FCC".to_string());
    }

    let raw_name = json.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let name = format_american_name(raw_name);

    let mut qth = None;
    let mut state = None;
    if let Some(addr) = json.get("address") {
        if let Some(line2) = addr.get("line2").and_then(|v| v.as_str()) {
            // format: "PERU, MA 01235"
            let parts: Vec<&str> = line2.split(',').collect();
            if !parts.is_empty() {
                qth = Some(parts[0].trim().to_string());
            }
            if parts.len() > 1 {
                let st_zip = parts[1].trim();
                let st_words: Vec<&str> = st_zip.split_whitespace().collect();
                if !st_words.is_empty() {
                    state = Some(st_words[0].trim().to_uppercase());
                }
            }
        }
    }

    let grid = json
        .get("location")
        .and_then(|loc| loc.get("gridsquare"))
        .and_then(|v| v.as_str())
        .map(str::to_uppercase);

    let (dxcc, country) = match state.as_deref() {
        Some("AK") => (Some(6), Some("Alaska".to_string())),
        Some("HI") => (Some(110), Some("Hawaii".to_string())),
        Some("PR") => (Some(202), Some("Puerto Rico".to_string())),
        Some("VI") => (Some(285), Some("US Virgin Islands".to_string())),
        Some("GU") => (Some(103), Some("Guam".to_string())),
        Some("MP") => (Some(166), Some("Mariana Islands".to_string())),
        Some("AS") => (Some(9), Some("American Samoa".to_string())),
        _ => {
            if clean.starts_with("KL")
                || clean.starts_with("AL")
                || clean.starts_with("NL")
                || clean.starts_with("WL")
            {
                (Some(6), Some("Alaska".to_string()))
            } else if clean.starts_with("KH6")
                || clean.starts_with("NH6")
                || clean.starts_with("WH6")
                || clean.starts_with("AH6")
                || clean.starts_with("KH7")
                || clean.starts_with("NH7")
                || clean.starts_with("WH7")
                || clean.starts_with("AH7")
            {
                (Some(110), Some("Hawaii".to_string()))
            } else if clean.starts_with("KP4")
                || clean.starts_with("NP4")
                || clean.starts_with("WP4")
                || clean.starts_with("KP3")
                || clean.starts_with("NP3")
                || clean.starts_with("WP3")
            {
                (Some(202), Some("Puerto Rico".to_string()))
            } else if clean.starts_with("KP2")
                || clean.starts_with("NP2")
                || clean.starts_with("WP2")
            {
                (Some(285), Some("US Virgin Islands".to_string()))
            } else if clean.starts_with("KH2")
                || clean.starts_with("NH2")
                || clean.starts_with("WH2")
                || clean.starts_with("AH2")
            {
                (Some(103), Some("Guam".to_string()))
            } else {
                (Some(291), Some("United States".to_string()))
            }
        }
    };

    Ok(CallbookData {
        callsign: clean,
        name: if name.is_empty() { None } else { Some(name) },
        qth,
        gridsquare: grid,
        state,
        dxcc,
        country,
        qsl_manager: None,
        email: None,
        image_url: None,
    })
}

/// Konwertuje nazwisko z FCC "DOE, JOHN M" na czytelne "John Doe"
fn format_american_name(raw: &str) -> String {
    let clean = raw.trim();
    if clean.is_empty() {
        return String::new();
    }
    if clean.contains(',') {
        let parts: Vec<&str> = clean.split(',').collect();
        let last = to_title_case(parts[0].trim());
        let first = if parts.len() > 1 {
            to_title_case(parts[1].trim())
        } else {
            String::new()
        };
        if first.is_empty() {
            last
        } else {
            format!("{first} {last}")
        }
    } else {
        to_title_case(clean)
    }
}

fn to_title_case(s: &str) -> String {
    let mut result = String::new();
    for word in s.split_whitespace() {
        if !result.is_empty() {
            result.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.extend(chars.flat_map(char::to_lowercase));
        }
    }
    result
}

/// Aktualny czas uniksowy w sekundach.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Scala dane callbook: pola brakujące w bazie uzupełnia z nowego wyniku
/// (źródła o wyższym priorytecie są nadrzędne — nowe dane nie nadpisują już znalezionych).
fn merge_callbook(base: Option<CallbookData>, incoming: CallbookData) -> CallbookData {
    match base {
        None => incoming,
        Some(mut b) => {
            if b.name.is_none() {
                b.name = incoming.name;
            }
            if b.qth.is_none() {
                b.qth = incoming.qth;
            }
            if b.gridsquare.is_none() {
                b.gridsquare = incoming.gridsquare;
            }
            if b.state.is_none() {
                b.state = incoming.state;
            }
            if b.dxcc.is_none() {
                b.dxcc = incoming.dxcc;
            }
            if b.country.is_none() {
                b.country = incoming.country;
            }
            if b.qsl_manager.is_none() {
                b.qsl_manager = incoming.qsl_manager;
            }
            if b.email.is_none() {
                b.email = incoming.email;
            }
            if b.image_url.is_none() {
                b.image_url = incoming.image_url;
            }
            b
        }
    }
}

/// Czy dane callbook są wystarczająco kompletne, by przerwać dalsze zapytania.
fn is_complete(data: &CallbookData) -> bool {
    data.name.is_some() && data.qth.is_some() && data.gridsquare.is_some()
}

/// Zintegrowana asynchroniczna procedura pobierania danych korespondenta.
///
/// Źródła są odpytywane w kolejności `priority`; wyniki są scalane (źródło
/// o wyższym priorytecie wygrywa), a odpowiedzi online zapisywane do cache offline.
pub async fn fetch_callsign_data(
    callsign: String,
    local_callbook: Arc<LocalCallbook>,
    hamqth_user: String,
    hamqth_pass: String,
    qrz_user: String,
    qrz_pass: String,
    priority: &[CallbookSource],
) -> Option<CallbookData> {
    let clean = callsign.trim().to_uppercase();
    if clean.is_empty() {
        return None;
    }

    // Stacja z USA (W, K, N, AA..AL) — najszybsze darmowe źródło to callook.info
    let is_usa = clean.starts_with('W')
        || clean.starts_with('K')
        || clean.starts_with('N')
        || (clean.starts_with('A')
            && clean.len() >= 2
            && clean
                .chars()
                .nth(1)
                .is_some_and(|c| ('A'..='L').contains(&c)));

    let mut result: Option<CallbookData> = None;

    for src in priority {
        let found: Option<CallbookData> = match src {
            CallbookSource::Local => {
                let cb = local_callbook.clone();
                let c = clean.clone();
                tokio::task::spawn_blocking(move || cb.lookup(&c))
                    .await
                    .ok()
                    .flatten()
            }
            CallbookSource::Cache => {
                let cb = local_callbook.clone();
                let c = clean.clone();
                tokio::task::spawn_blocking(move || cb.cache_lookup(&c))
                    .await
                    .ok()
                    .flatten()
            }
            CallbookSource::Callook => {
                if is_usa {
                    lookup_callook_info(&clean).await.ok()
                } else {
                    None
                }
            }
            CallbookSource::HamQth => {
                if !hamqth_user.is_empty() && !hamqth_pass.is_empty() {
                    let mut client = HamQthXmlClient::new(hamqth_user.clone(), hamqth_pass.clone());
                    client.lookup_callsign(&clean).await.ok()
                } else {
                    None
                }
            }
            CallbookSource::Qrz => {
                if !qrz_user.is_empty() && !qrz_pass.is_empty() {
                    let mut client = QrzClient::new(qrz_user.clone(), qrz_pass.clone());
                    client.lookup(&clean).await.ok()
                } else {
                    None
                }
            }
        };

        if let Some(data) = found {
            let from_online = matches!(
                src,
                CallbookSource::Callook | CallbookSource::HamQth | CallbookSource::Qrz
            );
            if from_online {
                let cb = local_callbook.clone();
                let data_for_cache = data.clone();
                let _ = tokio::task::spawn_blocking(move || cb.cache_store(&data_for_cache)).await;
            }
            let merged = merge_callbook(result.take(), data);
            let complete = is_complete(&merged);
            result = Some(merged);
            if complete {
                break;
            }
        }
    }

    if let Some(data) = result {
        return Some(data);
    }

    // Ostateczny fallback dla znanych stacji bazowych
    match clean.as_str() {
        "SP6INA" => Some(CallbookData {
            callsign: clean,
            name: Some("Mariusz Woźniak".to_string()),
            qth: Some("Wrocław".to_string()),
            gridsquare: Some("JO81WA".to_string()),
            state: None,
            dxcc: Some(269),
            country: Some("Poland".to_string()),
            qsl_manager: None,
            email: None,
            image_url: None,
        }),
        "W1AW" => Some(CallbookData {
            callsign: clean,
            name: Some("ARRL HQ Station".to_string()),
            qth: Some("Newington".to_string()),
            gridsquare: Some("FN31PR".to_string()),
            state: Some("CT".to_string()),
            dxcc: Some(291),
            country: Some("United States".to_string()),
            qsl_manager: None,
            email: None,
            image_url: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base_call() {
        assert_eq!(extract_base_call("SP6INA"), "SP6INA");
        assert_eq!(extract_base_call("DL/SP6INA"), "SP6INA");
        assert_eq!(extract_base_call("SP6INA/P"), "SP6INA");
        assert_eq!(extract_base_call("SP/DL1ABC/M"), "DL1ABC");
    }

    #[test]
    fn test_format_american_name() {
        assert_eq!(format_american_name("ROBBINS, DAVID R"), "David R Robbins");
        assert_eq!(format_american_name("DOE, JOHN"), "John Doe");
        assert_eq!(format_american_name("MARIUSZ WOZNIAK"), "Mariusz Wozniak");
    }

    #[test]
    fn test_local_callbook_lookup() {
        let cb_path = PathBuf::from("databases/callbook.db");
        let srv_path = PathBuf::from("databases/serviceLOG.db");
        let cb = LocalCallbook::new(Some(cb_path), Some(srv_path));

        // Test stacji SP1ADT z bazy callbook.db
        if let Some(data) = cb.lookup("SP1ADT") {
            assert_eq!(data.callsign, "SP1ADT");
            assert_eq!(data.name.as_deref(), Some("Andrzej"));
            assert_eq!(data.qth.as_deref(), Some("Zlocieniec"));
            assert_eq!(data.gridsquare.as_deref(), Some("JO83AM"));
        }

        // Test stacji z prefiksem DL/SP1ADT
        if let Some(data) = cb.lookup("DL/SP1ADT") {
            assert_eq!(data.name.as_deref(), Some("Andrzej"));
        }
    }

    #[test]
    fn test_callbook_cache_roundtrip() {
        let cache_path =
            std::env::temp_dir().join(format!("splogbook_cache_test_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&cache_path);

        let cb = LocalCallbook::new(None, None).with_cache(Some(cache_path.clone()), 30);
        let data = CallbookData {
            callsign: "SP6INA".to_string(),
            name: Some("Mariusz".to_string()),
            qth: Some("Wrocław".to_string()),
            gridsquare: Some("JO81WA".to_string()),
            state: None,
            dxcc: Some(269),
            country: Some("Poland".to_string()),
            qsl_manager: None,
            email: None,
            image_url: None,
        };

        cb.cache_store(&data);

        let got = cb
            .cache_lookup("sp6ina")
            .expect("cache should return stored entry");
        assert_eq!(got.callsign, "SP6INA");
        assert_eq!(got.name.as_deref(), Some("Mariusz"));
        assert_eq!(got.qth.as_deref(), Some("Wrocław"));
        assert_eq!(got.gridsquare.as_deref(), Some("JO81WA"));
        assert_eq!(got.dxcc, Some(269));
        assert_eq!(got.country.as_deref(), Some("Poland"));

        let _ = std::fs::remove_file(&cache_path);
    }
}
