// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::cloud::hamqth::HamQthXmlClient;
use crate::cloud::qrz::{CallbookData, QrzClient};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::sync::Arc;

/// Lokalna baza danych Callbook (offline)
#[derive(Debug, Clone)]
pub struct LocalCallbook {
    callbook_path: Option<PathBuf>,
    servicelog_path: Option<PathBuf>,
}

impl LocalCallbook {
    pub fn new(callbook_path: Option<PathBuf>, servicelog_path: Option<PathBuf>) -> Self {
        Self {
            callbook_path,
            servicelog_path,
        }
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
                if let Ok(conn) = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
                    // Najpierw szukamy dokładnego znaku, a potem bazowego
                    for query_call in &[clean.as_str(), base_call] {
                        let mut stmt = match conn.prepare(
                            "SELECT Name, QTH, Grid, State, Manager FROM Callbook WHERE Call = ? LIMIT 1"
                        ) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };

                        let found = stmt.query_row([query_call], |row| {
                            let name: Option<String> = row.get(0).ok().and_then(clean_opt_str);
                            let qth: Option<String> = row.get(1).ok().and_then(clean_opt_str);
                            let grid: Option<String> = row.get(2).ok().and_then(clean_opt_str);
                            let state: Option<String> = row.get(3).ok().and_then(clean_opt_str);
                            let manager: Option<String> = row.get(4).ok().and_then(clean_opt_str);

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
                        }).ok();

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
                    if let Ok(conn) = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
                        for query_call in &[clean.as_str(), base_call] {
                            let mut stmt = match conn.prepare(
                                "SELECT Manager FROM managers WHERE Call = ? LIMIT 1"
                            ) {
                                Ok(s) => s,
                                Err(_) => continue,
                            };

                            let mgr: Option<String> = stmt.query_row([query_call], |row| {
                                row.get(0)
                            }).ok().and_then(clean_opt_str);

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
fn clean_opt_str(s: String) -> Option<String> {
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
    let url = format!("https://callook.info/{}/json", clean);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .user_agent("SPLogbook/1.0.0 (SP6INA)")
        .build()
        .map_err(|e| e.to_string())?;

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

    let grid = json.get("location")
        .and_then(|loc| loc.get("gridsquare"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());

    Ok(CallbookData {
        callsign: clean,
        name: if name.is_empty() { None } else { Some(name) },
        qth,
        gridsquare: grid,
        state,
        dxcc: Some(291), // United States
        country: Some("United States".to_string()),
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
            format!("{} {}", first, last)
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
            result.extend(chars.flat_map(|c| c.to_lowercase()));
        }
    }
    result
}

/// Zintegrowana asynchroniczna procedura pobierania danych korespondenta ze wszystkich źródeł
pub async fn fetch_callsign_data(
    callsign: String,
    local_callbook: Arc<LocalCallbook>,
    hamqth_user: String,
    hamqth_pass: String,
    qrz_user: String,
    qrz_pass: String,
) -> Option<CallbookData> {
    let clean = callsign.trim().to_uppercase();
    if clean.is_empty() {
        return None;
    }

    // 1. Sprawdzamy lokalną bazę danych (callbook.db + serviceLOG.db)
    let local_res = local_callbook.lookup(&clean);
    if let Some(ref loc) = local_res {
        // Jeśli lokalna baza ma komplet danych (name i grid), zwracamy natychmiast
        if loc.name.is_some() && loc.gridsquare.is_some() {
            return Some(loc.clone());
        }
    }

    // 2. Jeśli stacja jest z USA (W, K, N, AA..AL), najszybszym darmowym źródłem jest callook.info
    let is_usa = clean.starts_with('W')
        || clean.starts_with('K')
        || clean.starts_with('N')
        || (clean.starts_with('A')
            && clean.len() >= 2
            && clean.chars().nth(1).is_some_and(|c| ('A'..='L').contains(&c)));

    if is_usa {
        if let Ok(data) = lookup_callook_info(&clean).await {
            return Some(data);
        }
    }

    // 3. Sprawdzamy darmowe API HamQTH XML (jeśli skonfigurowano)
    if !hamqth_user.is_empty() && !hamqth_pass.is_empty() {
        let mut hamqth_client = HamQthXmlClient::new(hamqth_user, hamqth_pass);
        if let Ok(data) = hamqth_client.lookup_callsign(&clean).await {
            return Some(data);
        }
    }

    // 4. Sprawdzamy QRZ XML (jeśli skonfigurowano)
    if !qrz_user.is_empty() && !qrz_pass.is_empty() {
        let mut qrz_client = QrzClient::new(qrz_user, qrz_pass);
        if let Ok(data) = qrz_client.lookup(&clean).await {
            return Some(data);
        }
    }

    // 5. Jeśli cokolwiek było w lokalnej bazie, zwracamy chociaż te częściowe dane
    if local_res.is_some() {
        return local_res;
    }

    // 6. Ostateczny fallback dla stacji demonstracyjnych
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
        "DL1ABC" => Some(CallbookData {
            callsign: clean,
            name: Some("Hans Schmidt".to_string()),
            qth: Some("Berlin".to_string()),
            gridsquare: Some("JO62QJ".to_string()),
            state: None,
            dxcc: Some(230),
            country: Some("Fed. Rep. of Germany".to_string()),
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
}
