// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Obsługa bazy referencyjnej serviceLOG.db (IOTA, Stany/Okręgi, Managerowie QSL, Unikalne znaki)

use crate::core::reference::tables::{TableKind, UserTables};
use crate::core::text::escape_like;
use rusqlite::{Connection, OpenFlags, params};
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
    /// Tabele zaimportowane przez użytkownika (pierwszeństwo przed starszą bazą).
    user: Arc<std::sync::RwLock<UserTables>>,
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

        if found_path.is_none()
            && let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            let p1 = dir.join("databases/serviceLOG.db");
            if p1.exists() {
                found_path = Some(p1);
            }
        }

        let conn = if let Some(path) = found_path {
            Self::open_at(&path).ok()
        } else {
            None
        };

        Self {
            conn: conn.map(|c| Arc::new(Mutex::new(c))),
            user: crate::core::reference::loader::user_tables(),
        }
    }

    /// Baza bez starszego pliku, korzystająca wyłącznie z podanych tabel użytkownika.
    pub fn with_user_tables(user: Arc<std::sync::RwLock<UserTables>>) -> Self {
        Self { conn: None, user }
    }

    fn user(&self) -> std::sync::RwLockReadGuard<'_, UserTables> {
        self.user
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// `true`, gdy otwarto starszą bazę referencyjną (warstwa zgodności).
    pub fn has_legacy_database(&self) -> bool {
        self.conn.is_some()
    }

    /// `true`, gdy dla kategorii istnieje jakiekolwiek źródło danych
    /// (tabela użytkownika lub starsza baza).
    pub fn has_data(&self, kind: TableKind) -> bool {
        self.user().is_installed(kind) || self.conn.is_some()
    }

    pub fn open_at(path: &Path) -> Result<Connection, rusqlite::Error> {
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
    }

    /// Wyszukuje grupy wysp IOTA (po kodzie np. 'EU-001' lub nazwie)
    pub fn search_iota(&self, query: &str) -> Vec<IotaRecord> {
        let clean = query.trim().to_uppercase();
        {
            let user = self.user();
            if user.is_installed(TableKind::Iota) {
                let res = user.search_iota(&clean);
                if !res.is_empty() {
                    return res;
                }
            }
        }
        if let Some(ref conn_mutex) = self.conn
            && let Ok(conn) = conn_mutex.lock()
        {
            let pattern = format!("%{}%", escape_like(&clean));
            if let Ok(mut stmt) = conn.prepare("SELECT IOTA, Name, Prefix1 FROM IOTA WHERE IOTA LIKE ?1 ESCAPE '\\' OR Name LIKE ?1 ESCAPE '\\' ORDER BY IOTA ASC LIMIT 100")
                && let Ok(rows) = stmt.query_map(params![pattern], |row| {
                    Ok(IotaRecord {
                        iota: row.get::<_, String>(0)?,
                        name: row.get::<_, String>(1)?,
                        prefix: row.get::<_, String>(2).unwrap_or_default(),
                    })
                })
            {
                let mut res = Vec::new();
                for r in rows {
                    match r {
                        Ok(r) => res.push(r),
                        Err(e) => eprintln!("Ignorowanie błędu w service_db.rs: {e}"),
                    }
                }
                if !res.is_empty() {
                    return res;
                }
            }
        }

        Self::fallback_iota(&clean)
    }

    pub fn find_iota(&self, code: &str) -> Option<IotaRecord> {
        let clean = code.trim().to_uppercase();
        if let Some(rec) = self.user().find_iota(&clean) {
            return Some(rec);
        }
        if let Some(ref conn_mutex) = self.conn
            && let Ok(conn) = conn_mutex.lock()
            && let Ok(mut stmt) =
                conn.prepare("SELECT IOTA, Name, Prefix1 FROM IOTA WHERE IOTA = ?1 LIMIT 1")
            && let Ok(mut rows) = stmt.query_map(params![clean], |row| {
                Ok(IotaRecord {
                    iota: row.get(0)?,
                    name: row.get(1)?,
                    prefix: row.get::<_, String>(2).unwrap_or_default(),
                })
            })
            && let Some(Ok(rec)) = rows.next()
        {
            return Some(rec);
        }
        Self::fallback_iota(&clean)
            .into_iter()
            .find(|i| i.iota == clean)
    }

    /// Wyszukuje stany USA (WAS) oraz rejony z bazy STATE
    pub fn search_states(&self, query: &str) -> Vec<StateRecord> {
        let clean = query.trim().to_uppercase();
        let mut results = Vec::new();

        // 1. Zawsze dołącz pasujące stany USA (50 stanów ARRL WAS)
        for &(code, name) in US_STATES {
            if clean.is_empty() || code.contains(&clean) || name.to_uppercase().contains(&clean) {
                results.push(StateRecord {
                    code: code.to_string(),
                    name: name.to_string(),
                    country: "USA".to_string(),
                });
            }
        }

        // 2. Tabela stanów i jednostek administracyjnych użytkownika
        for r in self.user().search_states(&clean) {
            if !results
                .iter()
                .any(|s| s.code == r.code && s.country == r.country)
            {
                results.push(r);
            }
        }

        // 3. Jeśli podłączono serviceLOG.db, dołącz wpisy z bazy STATE
        if let Some(ref conn_mutex) = self.conn
            && let Ok(conn) = conn_mutex.lock()
        {
            let pattern = format!("%{}%", escape_like(&clean));
            if let Ok(mut stmt) = conn.prepare("SELECT State, Name, Country FROM STATE WHERE State LIKE ?1 ESCAPE '\\' OR Name LIKE ?1 ESCAPE '\\' LIMIT 100")
                && let Ok(rows) = stmt.query_map(params![pattern], |row| {
                    Ok(StateRecord {
                        code: row.get(0)?,
                        name: row.get(1)?,
                        country: row.get(2)?,
                    })
                })
            {
                for r in rows {
                    match r {
                        Ok(r) => {
                            if !results.iter().any(|s| s.code == r.code && s.country == r.country) {
                                results.push(r);
                            }
                        }
                        Err(e) => eprintln!("Ignorowanie błędu w service_db.rs: {e}"),
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
        if let Some(rec) = self.user().find_manager(&clean) {
            return Some(rec);
        }

        if let Some(ref conn_mutex) = self.conn
            && let Ok(conn) = conn_mutex.lock()
            && let Ok(mut stmt) = conn
                .prepare("SELECT Call, Manager, Years, Notes FROM managers WHERE Call = ?1 LIMIT 1")
            && let Ok(mut rows) = stmt.query_map(params![clean], |row| {
                Ok(QslManagerRecord {
                    call: row.get(0)?,
                    manager: row.get(1)?,
                    years: row.get::<_, String>(2).unwrap_or_default(),
                    notes: row.get::<_, String>(3).unwrap_or_default(),
                })
            })
            && let Some(Ok(rec)) = rows.next()
        {
            return Some(rec);
        }

        None
    }

    /// Przeszukuje bazę managerów QSL
    pub fn search_managers(&self, query: &str) -> Vec<QslManagerRecord> {
        let clean = query.trim().to_uppercase();
        if clean.is_empty() {
            return Vec::new();
        }
        {
            let user = self.user();
            if user.is_installed(TableKind::QslManagers) {
                let res = user.search_managers(&clean);
                if !res.is_empty() {
                    return res;
                }
            }
        }

        if let Some(ref conn_mutex) = self.conn
            && let Ok(conn) = conn_mutex.lock()
        {
            let pattern = format!("%{}%", escape_like(&clean));
            if let Ok(mut stmt) = conn.prepare("SELECT Call, Manager, Years, Notes FROM managers WHERE Call LIKE ?1 ESCAPE '\\' OR Manager LIKE ?1 ESCAPE '\\' ORDER BY Call ASC LIMIT 100")
                && let Ok(rows) = stmt.query_map(params![pattern], |row| {
                    Ok(QslManagerRecord {
                        call: row.get(0)?,
                        manager: row.get(1)?,
                        years: row.get::<_, String>(2).unwrap_or_default(),
                        notes: row.get::<_, String>(3).unwrap_or_default(),
                    })
                })
            {
                let mut res = Vec::new();
                for r in rows {
                    match r {
                        Ok(r) => res.push(r),
                        Err(e) => eprintln!("Ignorowanie błędu w service_db.rs: {e}"),
                    }
                }
                return res;
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

        if let Some(ref conn_mutex) = self.conn
            && let Ok(conn) = conn_mutex.lock()
            && let Ok(mut stmt) = conn.prepare("SELECT Callsign, Country, DXCC, ARRLPrefix, Continent, CQZone, ITUZone FROM UniqueCalls WHERE Callsign = ?1 LIMIT 1")
            && let Ok(mut rows) = stmt.query_map(params![clean], |row| {
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
            })
            && let Some(Ok(rec)) = rows.next()
        {
            return Some(rec);
        }

        None
    }

    /// Wyjątki pełnych znaków: ze starszej bazy, a bez niej — z załadowanych
    /// plików krajów (`=ZNAK`). Dopasowanie po znaku lub nazwie kraju,
    /// sortowanie po znaku, najwyżej 100 wyników.
    pub fn search_exception_calls(
        &self,
        query: &str,
        matcher: &crate::core::prefix::PrefixMatcher,
    ) -> Vec<UniqueCallRecord> {
        if self.conn.is_some() {
            return self.search_unique_calls(query);
        }
        let clean = query.trim().to_uppercase();
        if clean.is_empty() {
            return Vec::new();
        }
        matcher
            .exception_calls()
            .into_iter()
            .filter(|i| i.callsign.contains(&clean) || i.country.to_uppercase().contains(&clean))
            .take(100)
            .map(|i| UniqueCallRecord {
                call: i.callsign,
                country: i.country,
                dxcc: i.dxcc,
                arrl_prefix: i.arrl_prefix,
                continent: i.continent,
                cq_zone: i.cqz,
                itu_zone: i.ituz,
            })
            .collect()
    }

    /// Przeszukuje tabelę unikalnych przypisań prefiksów i znaków specjalnych
    pub fn search_unique_calls(&self, query: &str) -> Vec<UniqueCallRecord> {
        let clean = query.trim().to_uppercase();
        if clean.is_empty() {
            return Vec::new();
        }

        if let Some(ref conn_mutex) = self.conn
            && let Ok(conn) = conn_mutex.lock()
        {
            let pattern = format!("%{}%", escape_like(&clean));
            if let Ok(mut stmt) = conn.prepare("SELECT Callsign, Country, DXCC, ARRLPrefix, Continent, CQZone, ITUZone FROM UniqueCalls WHERE Callsign LIKE ?1 ESCAPE '\\' OR Country LIKE ?1 ESCAPE '\\' ORDER BY Callsign ASC LIMIT 100")
                && let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
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
                })
            {
                let mut res = Vec::new();
                for r in rows {
                    match r {
                        Ok(r) => res.push(r),
                        Err(e) => eprintln!("Ignorowanie błędu w service_db.rs: {e}"),
                    }
                }
                return res;
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
            (
                "EU-132",
                "Polish Baltic Sea Coast Islands (Wolin, Usedom)",
                "SP",
            ),
            ("NA-001", "Greenland", "OX"),
            ("NA-015", "Cuba", "CO"),
            ("OC-001", "Australia", "VK"),
            ("OC-019", "Hawaiian Islands", "KH6"),
        ];

        sample
            .iter()
            .filter(|(code, name, _)| {
                query.is_empty() || code.contains(query) || name.to_uppercase().contains(query)
            })
            .map(|(code, name, pfx)| IotaRecord {
                iota: code.to_string(),
                name: name.to_string(),
                prefix: pfx.to_string(),
            })
            .collect()
    }
}

pub static US_STATES: &[(&str, &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
    ("DC", "District of Columbia"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_states_search() {
        let db = ServiceDatabase::open();
        let results = db.search_states("CALIF");
        assert_ne!(results, []);
        assert_eq!(results[0].code, "CA");
    }

    #[test]
    fn test_iota_lookup() {
        let db = ServiceDatabase::open();
        let iota = db.find_iota("EU-001");
        assert!(iota.is_some());
    }
}

#[cfg(test)]
mod user_table_tests {
    use super::*;
    use crate::core::reference::tables::{TableKind, UserTables, import_table};
    use std::sync::RwLock;

    fn tables_with(files: &[(TableKind, &str)]) -> Arc<RwLock<UserTables>> {
        let root = tempfile::tempdir().unwrap();
        for (kind, text) in files {
            let p = root.path().join(format!("{}.csv", kind.set_name()));
            std::fs::write(&p, text).unwrap();
            import_table(root.path(), *kind, &p).unwrap();
        }
        Arc::new(RwLock::new(UserTables::load(root.path()).0))
    }

    #[test]
    fn empty_user_tables_give_data_not_installed_and_safe_results() {
        let db = ServiceDatabase::with_user_tables(Arc::default());
        for kind in TableKind::ALL {
            assert!(!db.has_data(kind));
        }
        assert!(db.find_manager("QA1AA").is_none());
        assert_eq!(db.search_managers("QA"), Vec::<QslManagerRecord>::new());
        // The built-in US list stays available without any data file.
        assert_eq!(db.search_states("").len(), US_STATES.len());
        // The built-in IOTA sample stays available as before.
        assert_ne!(db.search_iota("AF"), Vec::<IotaRecord>::new());
    }

    #[test]
    fn imported_tables_drive_every_browser_query() {
        let shared = tables_with(&[
            (
                TableKind::Iota,
                "#splogbook-iota,1\nreference,name,prefix\nEU-901,Testing Isle,QA\n",
            ),
            (
                TableKind::States,
                "#splogbook-states,1\ncode,name,country\nQN,North Test,Alpha Land\n",
            ),
            (
                TableKind::QslManagers,
                "#splogbook-qsl-managers,1\ncallsign,manager,years,notes\nQA1AA,QB1BB,2026,test\n",
            ),
        ]);
        let db = ServiceDatabase::with_user_tables(shared.clone());
        assert!(db.has_data(TableKind::Iota));
        assert_eq!(db.search_iota("testing")[0].iota, "EU-901");
        assert_eq!(db.find_iota("eu-901").unwrap().name, "Testing Isle");
        let states = db.search_states("north");
        assert!(
            states
                .iter()
                .any(|s| s.code == "QN" && s.country == "Alpha Land")
        );
        assert_eq!(db.find_manager("qa1aa").unwrap().manager, "QB1BB");
        assert_eq!(db.search_managers("QB1").len(), 1);

        // Reload through the shared handle: no restart, same ServiceDatabase.
        *shared.write().unwrap() = UserTables::default();
        assert!(db.find_manager("QA1AA").is_none());
    }
}
