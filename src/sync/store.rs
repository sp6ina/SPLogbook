// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Abstrakcja magazynu danych dla synchronizacji QSO (`src/sync/store.rs`).

use super::error::SyncError;
use crate::core::ingest::{ImportOptions, QsoIngestor};
use crate::core::qso::{QsoIdentity, QsoRecord};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Obiektowo-bezpieczny interfejs magazynu synchronizacji (kompatybilny z `Arc<dyn SyncStore>`).
pub trait SyncStore: Send + Sync + 'static {
    /// Zwraca zbiór tożsamości `QsoIdentity` dla wskazanego dziennika.
    fn inventory(&self, journal: &str) -> Result<HashSet<QsoIdentity>, SyncError>;

    /// Pobiera pełne rekordy QSO dla wskazanych tożsamości z danego dziennika.
    fn records_for(
        &self,
        journal: &str,
        wanted: &HashSet<QsoIdentity>,
    ) -> Result<Vec<QsoRecord>, SyncError>;

    /// Importuje partię rekordów QSO do wskazanego dziennika bez nadpisywania i usuwania.
    /// Zwraca liczbę nowo wstawionych rekordów.
    fn import(&self, journal: &str, qsos: Vec<QsoRecord>) -> Result<usize, SyncError>;
}

impl SyncStore for QsoIngestor {
    fn inventory(&self, journal: &str) -> Result<HashSet<QsoIdentity>, SyncError> {
        let db = self
            .db()
            .lock()
            .map_err(|e| SyncError::Store(e.to_string()))?;
        db.existing_identities(journal)
            .map_err(|e| SyncError::Store(e.to_string()))
    }

    fn records_for(
        &self,
        journal: &str,
        wanted: &HashSet<QsoIdentity>,
    ) -> Result<Vec<QsoRecord>, SyncError> {
        let db = self
            .db()
            .lock()
            .map_err(|e| SyncError::Store(e.to_string()))?;
        let all_qsos = db
            .get_all_qsos_for_journal(journal)
            .map_err(|e| SyncError::Store(e.to_string()))?;
        Ok(all_qsos
            .into_iter()
            .filter(|q| wanted.contains(&q.identity()))
            .collect())
    }

    fn import(&self, journal: &str, qsos: Vec<QsoRecord>) -> Result<usize, SyncError> {
        let options = ImportOptions {
            journal_id: journal.to_string(),
            enrich: false,
        };
        let report = self
            .import_batch(qsos, &options)
            .map_err(|e| SyncError::Store(e.to_string()))?;
        Ok(report.inserted)
    }
}

/// Magazyn pamięciowy używany do testów jednostkowych i symulacji bez SQLite.
#[derive(Default, Clone)]
pub struct MemorySyncStore {
    data: Arc<Mutex<HashMap<String, HashMap<QsoIdentity, QsoRecord>>>>,
}

impl MemorySyncStore {
    /// Tworzy nowy, pusty magazyn pamięciowy.
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Pomocnicza metoda testowa: wstawia QSO do magazynu.
    pub fn insert_test_qso(&self, journal: &str, qso: QsoRecord) {
        let mut d = self.data.lock().unwrap();
        let journal_map = d.entry(journal.to_string()).or_default();
        journal_map.insert(qso.identity(), qso);
    }

    /// Pomocnicza metoda testowa: zwraca liczbę rekordów w dzienniku.
    pub fn count(&self, journal: &str) -> usize {
        self.data
            .lock()
            .unwrap()
            .get(journal)
            .map_or(0, std::collections::HashMap::len)
    }

    /// Pomocnicza metoda testowa: pobiera wszystkie QSO z danego dziennika.
    pub fn get_all(&self, journal: &str) -> Vec<QsoRecord> {
        self.data
            .lock()
            .unwrap()
            .get(journal)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }
}

impl SyncStore for MemorySyncStore {
    fn inventory(&self, journal: &str) -> Result<HashSet<QsoIdentity>, SyncError> {
        let d = self.data.lock().unwrap();
        Ok(d.get(journal)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn records_for(
        &self,
        journal: &str,
        wanted: &HashSet<QsoIdentity>,
    ) -> Result<Vec<QsoRecord>, SyncError> {
        let d = self.data.lock().unwrap();
        match d.get(journal) {
            Some(m) => Ok(m
                .iter()
                .filter(|(id, _)| wanted.contains(id))
                .map(|(_, q)| q.clone())
                .collect()),
            None => Ok(Vec::new()),
        }
    }

    fn import(&self, journal: &str, qsos: Vec<QsoRecord>) -> Result<usize, SyncError> {
        let mut d = self.data.lock().unwrap();
        let journal_map = d.entry(journal.to_string()).or_default();
        let mut inserted = 0;
        for q in qsos {
            let id = q.identity();
            if let std::collections::hash_map::Entry::Vacant(e) = journal_map.entry(id) {
                e.insert(q);
                inserted += 1;
            }
        }
        Ok(inserted)
    }
}
