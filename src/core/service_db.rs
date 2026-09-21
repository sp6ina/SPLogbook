// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Obsługa bazy referencyjnej serviceLOG.db (IOTA, Stany/Okręgi, Managerowie QSL, Unikalne znaki)

use rusqlite::{params, Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IotaRecord {
    pub iota: String,
    pub name: String,
    pub prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRecord {
    pub code: String,
    pub name: String,
    pub country: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QslManagerRecord {
    pub call: String,
    pub manager: String,
    pub years: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniqueCallRecord {
    pub call: String,
    pub country: String,
    pub dxcc: u32,
    pub arrl_prefix: String,
    pub continent: String,
    pub cq_zone: u32,
    pub itu_zone: u32,
}

pub struct ServiceDatabase {
    conn: Option<Arc<Mutex<Connection>>>,
}

impl ServiceDatabase {
    /// Otwiera bazę danych serviceLOG.db przeszukując typowe lokalizacje aplikacji
    pub fn open() -> Self {
        let candidates = [
            PathBuf::from("databases/serviceLOG.db"),
            PathBuf::from("Bin/Windows/databases/serviceLOG.db"),
            PathBuf::from("../databases/serviceLOG.db"),
            PathBuf::from("DataBases/serviceLOG.db"),
        ];

        let mut found_path = None;
        for c in &candidates {
            if c.exists() {
                found_path = Some(c.clone());
                break;
            }
        }

        if found_path.is_none() {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let p1 = dir.join("databases/serviceLOG.db");
                    if p1.exists() {
                        found_path = Some(p1);
                    }
                }
            }
        }

        let conn = if let Some(path) = found_path {
            Self::open_at(&path).ok()
        } else {
            None
        };

        Self {
            conn: conn.map(|c| Arc::new(Mutex::new(c))),
        }
    }

    pub fn open_at(path: &Path) -> Result<Connection, rusqlite::Error> {
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI)
    }

    /// Wyszukuje grupy wysp IOTA (po kodzie np. 'EU-001' lub nazwie)
    pub fn search_iota(&self, query: &str) -> Vec<IotaRecord> {
        let clean = query.trim().to_uppercase();
        if let Some(ref conn_mutex) = self.conn {
            if let Ok(conn) = conn_mutex.lock() {
                let pattern = format!("%{}%", clean);
                if let Ok(mut stmt) = conn.prepare("SELECT IOTA, Name, Prefix1 FROM IOTA WHERE IOTA LIKE ?1 OR Name LIKE ?1 ORDER BY IOTA ASC LIMIT 100") {
                    if let Ok(rows) = stmt.query_map(params![pattern], |row| {
                        Ok(IotaRecord {
                            iota: row.get::<_, String>(0)?,
                            name: row.get::<_, String>(1)?,
                            prefix: row.get::<_, String>(2).unwrap_or_default(),
                        })
                    }) {
                        let mut res = Vec::new();
                        for r in rows.flatten() {
                            res.push(r);
                        }
                        if !res.is_empty() {
                            return res;
                        }
                    }
                }
            }
        }

        Self::fallback_iota(&clean)
    }

    pub fn find_iota(&self, code: &str) -> Option<IotaRecord> {
        let clean = code.trim().to_uppercase();
        if let Some(ref conn_mutex) = self.conn {
            if let Ok(conn) = conn_mutex.lock() {
                if let Ok(mut stmt) = conn.prepare("SELECT IOTA, Name, Prefix1 FROM IOTA WHERE IOTA = ?1 LIMIT 1") {
                    if let Ok(mut rows) = stmt.query_map(params![clean], |row| {
                        Ok(IotaRecord {
                            iota: row.get(0)?,
                            name: row.get(1)?,
                            prefix: row.get::<_, String>(2).unwrap_or_default(),
                        })
                    }) {
                        if let Some(Ok(rec)) = rows.next() {
                            return Some(rec);
                        }
                    }
                }
            }
        }
        Self::fallback_iota(&clean).into_iter().find(|i| i.iota == clean)
    }

    /// Wyszukuje stany USA (WAS) oraz rejony z bazy STATE
    pub fn search_states(&self, query: &str) -> Vec<StateRecord> {
        let clean = query.trim().to_uppercase();
        let mut results = Vec::new();

        // 1. Zawsze dołącz pasujące stany USA (50 stanów ARRL WAS)
        for &(code, name) in US_STATES {
            if clean.is_empty()
                || code.contains(&clean)
                || name.to_uppercase().contains(&clean)
            {
                results.push(StateRecord {
                    code: code.to_string(),
                    name: name.to_string(),
                    country: "USA".to_string(),
                });
            }
        }

        // 2. Jeśli podłączono serviceLOG.db, dołącz wpisy z bazy STATE
        if let Some(ref conn_mutex) = self.conn {
            if let Ok(conn) = conn_mutex.lock() {
                let pattern = format!("%{}%", clean);
                if let Ok(mut stmt) = conn.prepare("SELECT State, Name, Country FROM STATE WHERE State LIKE ?1 OR Name LIKE ?1 LIMIT 100") {
                    if let Ok(rows) = stmt.query_map(params![pattern], |row| {
                        Ok(StateRecord {
                            code: row.get(0)?,
                            name: row.get(1)?,
                            country: row.get(2)?,
                        })
                    }) {
                        for r in rows.flatten() {
                            if !results.iter().any(|s| s.code == r.code && s.country == r.country) {
                                results.push(r);
                            }
                        }
                    }
                }
            }
        }

        results.truncate(100);
        results
    }

    /// Wyszukuje managera QSL dla znaku stacji (baza 29 102 managerów)
    pub fn find_manager(&self, callsign: &str) -> Option<QslManagerRecord> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return None;
        }

        if let Some(ref conn_mutex) = self.conn {
            if let Ok(conn) = conn_mutex.lock() {
                if let Ok(mut stmt) = conn.prepare("SELECT Call, Manager, Years, Notes FROM managers WHERE Call = ?1 LIMIT 1") {
                    if let Ok(mut rows) = stmt.query_map(params![clean], |row| {
                        Ok(QslManagerRecord {
                            call: row.get(0)?,
                            manager: row.get(1)?,
                            years: row.get::<_, String>(2).unwrap_or_default(),
                            notes: row.get::<_, String>(3).unwrap_or_default(),
                        })
                    }) {
                        if let Some(Ok(rec)) = rows.next() {
                            return Some(rec);
                        }
                    }
                }
            }
        }

        None
    }

    /// Przeszukuje bazę managerów QSL
    pub fn search_managers(&self, query: &str) -> Vec<QslManagerRecord> {
        let clean = query.trim().to_uppercase();
        if clean.is_empty() {
            return Vec::new();
        }

        if let Some(ref conn_mutex) = self.conn {
            if let Ok(conn) = conn_mutex.lock() {
                let pattern = format!("%{}%", clean);
                if let Ok(mut stmt) = conn.prepare("SELECT Call, Manager, Years, Notes FROM managers WHERE Call LIKE ?1 OR Manager LIKE ?1 ORDER BY Call ASC LIMIT 100") {
                    if let Ok(rows) = stmt.query_map(params![pattern], |row| {
                        Ok(QslManagerRecord {
                            call: row.get(0)?,
                            manager: row.get(1)?,
                            years: row.get::<_, String>(2).unwrap_or_default(),
                            notes: row.get::<_, String>(3).unwrap_or_default(),
                        })
                    }) {
                        return rows.flatten().collect();
                    }
                }
            }
        }

        Vec::new()
    }

    /// Sprawdza czy stacja posiada unikalne przypisanie DXCC / prefiksu (UniqueCalls - 4 310 rekordów)
    pub fn find_unique_call(&self, callsign: &str) -> Option<UniqueCallRecord> {
        let clean = callsign.trim().to_uppercase();
        if clean.is_empty() {
            return None;
        }

        if let Some(ref conn_mutex) = self.conn {
            if let Ok(conn) = conn_mutex.lock() {
                if let Ok(mut stmt) = conn.prepare("SELECT Callsign, Country, DXCC, ARRLPrefix, Continent, CQZone, ITUZone FROM UniqueCalls WHERE Callsign = ?1 LIMIT 1") {
                    if let Ok(mut rows) = stmt.query_map(params![clean], |row| {
                        let dxcc_str: String = row.get::<_, String>(2).unwrap_or_default();
                        let dxcc: u32 = dxcc_str.parse().unwrap_or(0);
                        Ok(UniqueCallRecord {
                            call: row.get(0)?,
                            country: row.get(1)?,
                            dxcc,
                            arrl_prefix: row.get::<_, String>(3).unwrap_or_default(),
                            continent: row.get::<_, String>(4).unwrap_or_default(),
                            cq_zone: row.get::<_, u32>(5).unwrap_or(0),
                            itu_zone: row.get::<_, u32>(6).unwrap_or(0),
                        })
                    }) {
                        if let Some(Ok(rec)) = rows.next() {
                            return Some(rec);
                        }
                    }
                }
            }
        }

        None
    }

    /// Przeszukuje tabelę unikalnych przypisań prefiksów i znaków specjalnych
    pub fn search_unique_calls(&self, query: &str) -> Vec<UniqueCallRecord> {
        let clean = query.trim().to_uppercase();
        if clean.is_empty() {
            return Vec::new();
        }

        if let Some(ref conn_mutex) = self.conn {
            if let Ok(conn) = conn_mutex.lock() {
                let pattern = format!("%{}%", clean);
                if let Ok(mut stmt) = conn.prepare("SELECT Callsign, Country, DXCC, ARRLPrefix, Continent, CQZone, ITUZone FROM UniqueCalls WHERE Callsign LIKE ?1 OR Country LIKE ?1 ORDER BY Callsign ASC LIMIT 100") {
                    if let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
                        let dxcc_str: String = row.get::<_, String>(2).unwrap_or_default();
                        let dxcc: u32 = dxcc_str.parse().unwrap_or(0);
                        Ok(UniqueCallRecord {
                            call: row.get(0)?,
                            country: row.get(1)?,
                            dxcc,
                            arrl_prefix: row.get::<_, String>(3).unwrap_or_default(),
                            continent: row.get::<_, String>(4).unwrap_or_default(),
                            cq_zone: row.get::<_, u32>(5).unwrap_or(0),
                            itu_zone: row.get::<_, u32>(6).unwrap_or(0),
                        })
                    }) {
                        return rows.flatten().collect();
                    }
                }
            }
        }

        Vec::new()
    }

    fn fallback_iota(query: &str) -> Vec<IotaRecord> {
        let sample = [
            ("AF-001", "Agalega Islands", "3B6"),
            ("AF-002", "Amsterdam & St. Paul Islands", "FT(Z)"),
            ("AF-003", "Ascension Island", "ZD8"),
            ("EU-001", "Dodecanese", "SV5"),
            ("EU-005", "Great Britain", "G"),
            ("EU-115", "Ireland", "EI"),
            ("EU-132", "Polish Baltic Sea Coast Islands (Wolin, Usedom)", "SP"),
            ("NA-001", "Greenland", "OX"),
            ("NA-015", "Cuba", "CO"),
            ("OC-001", "Australia", "VK"),
            ("OC-019", "Hawaiian Islands", "KH6"),
        ];

        sample
            .iter()
            .filter(|(code, name, _)| query.is_empty() || code.contains(query) || name.to_uppercase().contains(query))
            .map(|(code, name, pfx)| IotaRecord {
                iota: code.to_string(),
                name: name.to_string(),
                prefix: pfx.to_string(),
            })
            .collect()
    }
}

pub static US_STATES: &[(&str, &str)] = &[
    ("AL", "Alabama"), ("AK", "Alaska"), ("AZ", "Arizona"), ("AR", "Arkansas"),
    ("CA", "California"), ("CO", "Colorado"), ("CT", "Connecticut"), ("DE", "Delaware"),
    ("FL", "Florida"), ("GA", "Georgia"), ("HI", "Hawaii"), ("ID", "Idaho"),
    ("IL", "Illinois"), ("IN", "Indiana"), ("IA", "Iowa"), ("KS", "Kansas"),
    ("KY", "Kentucky"), ("LA", "Louisiana"), ("ME", "Maine"), ("MD", "Maryland"),
    ("MA", "Massachusetts"), ("MI", "Michigan"), ("MN", "Minnesota"), ("MS", "Mississippi"),
    ("MO", "Missouri"), ("MT", "Montana"), ("NE", "Nebraska"), ("NV", "Nevada"),
    ("NH", "New Hampshire"), ("NJ", "New Jersey"), ("NM", "New Mexico"), ("NY", "New York"),
    ("NC", "North Carolina"), ("ND", "North Dakota"), ("OH", "Ohio"), ("OK", "Oklahoma"),
    ("OR", "Oregon"), ("PA", "Pennsylvania"), ("RI", "Rhode Island"), ("SC", "South Carolina"),
    ("SD", "South Dakota"), ("TN", "Tennessee"), ("TX", "Texas"), ("UT", "Utah"),
    ("VT", "Vermont"), ("VA", "Virginia"), ("WA", "Washington"), ("WV", "West Virginia"),
    ("WI", "Wisconsin"), ("WY", "Wyoming"), ("DC", "District of Columbia"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_states_search() {
        let db = ServiceDatabase::open();
        let results = db.search_states("CALIF");
        assert!(!results.is_empty());
        assert_eq!(results[0].code, "CA");
    }

    #[test]
    fn test_iota_lookup() {
        let db = ServiceDatabase::open();
        let iota = db.find_iota("EU-001");
        assert!(iota.is_some());
    }
}
