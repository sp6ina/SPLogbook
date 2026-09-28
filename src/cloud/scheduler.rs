// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Wspólny harmonogram wysyłki QSO do serwisów online (Club Log, QRZ.com, eQSL.cc).
//!
//! Silnik jest czysto deterministyczny (wszystkie decyzje opierają się na jawnie
//! przekazanym `now`), dzięki czemu da się go w pełni testować jednostkowo.
//! Odpowiada za:
//!  * kolejkę offline (serializowaną do JSON na dysku),
//!  * ponawianie z wykładniczym backoffem,
//!  * ograniczenie tempa wysyłki per-serwis (rate-limit),
//!  * status każdego serwisu do wyświetlenia w GUI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Aktualny czas uniksowy w sekundach (używany przez warstwę I/O).
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Serwisy objęte wspólnym harmonogramem wysyłki ADIF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UploadService {
    ClubLog,
    Qrz,
    Eqsl,
}

impl UploadService {
    pub fn label(&self) -> &'static str {
        match self {
            UploadService::ClubLog => "Club Log",
            UploadService::Qrz => "QRZ.com",
            UploadService::Eqsl => "eQSL.cc",
        }
    }

    /// Minimalny odstęp pomiędzy kolejnymi próbami (sekundy) — ochrona przed
    /// limitami API poszczególnych serwisów.
    pub fn rate_limit_secs(&self) -> u64 {
        match self {
            UploadService::ClubLog | UploadService::Qrz => 5,
            UploadService::Eqsl => 10,
        }
    }

    pub fn all() -> [UploadService; 3] {
        [
            UploadService::ClubLog,
            UploadService::Qrz,
            UploadService::Eqsl,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UploadJobStatus {
    Pending,
    InFlight,
    Done,
    Failed,
}

/// Pojedyncze zadanie wysyłki (1 lub więcej QSO jako ADIF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadJob {
    pub id: u64,
    pub service: UploadService,
    pub adif: String,
    pub attempts: u32,
    pub status: UploadJobStatus,
    pub last_error: Option<String>,
    pub created_at: u64,
    pub next_attempt_at: u64,
}

/// Migawka statusu jednego serwisu (do wyświetlenia w GUI).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UploadServiceStatus {
    pub pending: usize,
    pub in_flight: usize,
    pub done: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

/// Wspólny harmonogram wysyłki. Wszystkie metody przyjmują `now` jawnie.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadScheduler {
    jobs: Vec<UploadJob>,
    next_id: u64,
    last_attempt_at: HashMap<UploadService, u64>,
    pub max_attempts: u32,
    pub base_backoff_secs: u64,
    pub max_backoff_secs: u64,
}

impl UploadScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            next_id: 1,
            last_attempt_at: HashMap::new(),
            max_attempts: 5,
            base_backoff_secs: 30,
            max_backoff_secs: 3600,
        }
    }

    /// Dodaje nowe zadanie do kolejki i zwraca jego identyfikator.
    pub fn enqueue(&mut self, service: UploadService, adif: String, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.jobs.push(UploadJob {
            id,
            service,
            adif,
            attempts: 0,
            status: UploadJobStatus::Pending,
            last_error: None,
            created_at: now,
            next_attempt_at: now,
        });
        id
    }

    /// Zwraca identyfikatory zadań gotowych do wysłania w chwili `now`.
    ///
    /// Na serwis przypada maksymalnie jedno zadanie, a serwis musi spełniać
    /// ograniczenie tempa (rate-limit) względem `last_attempt_at`.
    pub fn ready_jobs(&mut self, now: u64) -> Vec<u64> {
        let mut ready: Vec<u64> = Vec::new();
        let mut served: HashMap<UploadService, bool> = HashMap::new();

        for job in &self.jobs {
            if job.status != UploadJobStatus::Pending {
                continue;
            }
            if job.next_attempt_at > now {
                continue;
            }
            if served.get(&job.service).copied().unwrap_or(false) {
                continue;
            }
            let last = self.last_attempt_at.get(&job.service).copied().unwrap_or(0);
            if now.saturating_sub(last) < job.service.rate_limit_secs() {
                continue;
            }
            ready.push(job.id);
            served.insert(job.service, true);
        }
        ready
    }

    /// Oznacza zadanie jako „w trakcie wysyłki" i zapisuje czas próby.
    pub fn mark_in_flight(&mut self, id: u64, now: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = UploadJobStatus::InFlight;
            job.attempts += 1;
            self.last_attempt_at.insert(job.service, now);
        }
    }

    /// Oznacza zadanie jako zakończone sukcesem.
    pub fn mark_success(&mut self, id: u64, _now: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = UploadJobStatus::Done;
            job.last_error = None;
        }
    }

    /// Oznacza zadanie jako nieudane. Zwraca `true`, jeśli zaplanowano ponowną próbę.
    pub fn mark_failure(&mut self, id: u64, error: String, now: u64) -> bool {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.last_error = Some(error);
            if job.attempts >= self.max_attempts {
                job.status = UploadJobStatus::Failed;
                return false;
            }
            let shift = job.attempts.saturating_sub(1).min(20);
            let backoff = self
                .base_backoff_secs
                .saturating_mul(1u64 << shift)
                .min(self.max_backoff_secs);
            job.status = UploadJobStatus::Pending;
            job.next_attempt_at = now.saturating_add(backoff);
            return true;
        }
        false
    }

    /// Pobiera zadanie po identyfikatorze.
    pub fn job(&self, id: u64) -> Option<&UploadJob> {
        self.jobs.iter().find(|j| j.id == id)
    }

    /// Liczba zadań oczekujących (Pending) dla serwisu.
    pub fn pending_count(&self, service: UploadService) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.service == service && j.status == UploadJobStatus::Pending)
            .count()
    }

    /// Migawka statusu dla jednego serwisu.
    pub fn service_status(&self, service: UploadService) -> UploadServiceStatus {
        let mut s = UploadServiceStatus::default();
        for job in self.jobs.iter().filter(|j| j.service == service) {
            match job.status {
                UploadJobStatus::Pending => s.pending += 1,
                UploadJobStatus::InFlight => s.in_flight += 1,
                UploadJobStatus::Done => s.done += 1,
                UploadJobStatus::Failed => {
                    s.failed += 1;
                    s.last_error.clone_from(&job.last_error);
                }
            }
        }
        s
    }

    /// Migawka statusu wszystkich serwisów.
    pub fn status_all(&self) -> HashMap<UploadService, UploadServiceStatus> {
        UploadService::all()
            .into_iter()
            .map(|svc| (svc, self.service_status(svc)))
            .collect()
    }

    /// Ponownie kolejkuje wszystkie zadania zakończone porażką (ręczne „Ponów").
    pub fn retry_failed(&mut self, now: u64) -> usize {
        let mut count = 0;
        for job in &mut self.jobs {
            if job.status == UploadJobStatus::Failed {
                job.status = UploadJobStatus::Pending;
                job.attempts = 0;
                job.last_error = None;
                job.next_attempt_at = now;
                count += 1;
            }
        }
        count
    }

    /// Usuwa zakończone (Done) zadania — utrzymuje kolejkę w rozsądnym rozmiarze.
    pub fn purge_done(&mut self) -> usize {
        let before = self.jobs.len();
        self.jobs.retain(|j| j.status != UploadJobStatus::Done);
        before - self.jobs.len()
    }

    /// Serializuje kolejkę do JSON (do trwałego zapisu).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Odtwarza kolejkę z JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Ścieżka pliku kolejki offline (APPDATA/SPLogbook na Windows, XDG_DATA_HOME lub ~/.local/share/splogbook na innych systemach).
    pub fn queue_path() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata)
                    .join("SPLogbook")
                    .join("upload_queue.json");
            }
        }
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg)
                    .join("splogbook")
                    .join("upload_queue.json");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("splogbook")
                    .join("upload_queue.json");
            }
        }
        PathBuf::from("upload_queue.json")
    }

    /// Wczytuje kolejkę z dysku; w razie braku/zniszczenia pliku zwraca pustą kolejkę.
    /// Zadania pozostawione w stanie `InFlight` (np. po nagłym zamknięciu programu)
    /// są przywracane do stanu `Pending`, aby zostały ponowione.
    pub fn load_from_disk() -> Self {
        let path = Self::queue_path();
        let mut scheduler = match std::fs::read_to_string(&path) {
            Ok(json) => Self::from_json(&json).unwrap_or_else(|e| {
                log::warn!(
                    "Nie udało się odczytać kolejki wysyłki ({}): {}",
                    path.display(),
                    e
                );
                Self::new()
            }),
            Err(_) => Self::new(),
        };
        for job in &mut scheduler.jobs {
            if job.status == UploadJobStatus::InFlight {
                job.status = UploadJobStatus::Pending;
            }
        }
        scheduler
    }

    /// Zapisuje kolejkę na dysk atomowo (best-effort): najpierw plik tymczasowy,
    /// następnie `rename`, aby awaria nie pozostawiła pustego/uszkodzonego JSON
    /// pod docelową nazwą i nie ukryła oczekujących wysyłek.
    pub fn save_to_disk(&self) {
        if let Ok(json) = self.to_json() {
            let path = Self::queue_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut tmp_name = path
                .file_name()
                .map(std::ffi::OsString::from)
                .unwrap_or_default();
            tmp_name.push(".tmp");
            let tmp_path = path.with_file_name(tmp_name);
            if let Err(e) = std::fs::write(&tmp_path, json) {
                log::warn!(
                    "Nie udało się zapisać kolejki wysyłki ({}): {}",
                    tmp_path.display(),
                    e
                );
                return;
            }
            if let Err(e) = std::fs::rename(&tmp_path, &path) {
                log::warn!(
                    "Nie udało się sfinalizować zapisu kolejki ({}): {}",
                    path.display(),
                    e
                );
                let _ = std::fs::remove_file(&tmp_path);
            }
        }
    }
}

/// Dane uwierzytelniające potrzebne do wysłania zadań w tle.
#[derive(Debug, Clone, Default)]
pub struct UploadCredentials {
    pub clublog_callsign: String,
    pub clublog_email: String,
    pub clublog_password: String,
    pub clublog_api_key: String,
    pub qrz_api_key: String,
    pub eqsl_username: String,
    pub eqsl_password: String,
}

/// Wykonuje właściwą wysyłkę ADIF do wskazanego serwisu.
pub async fn execute_upload(
    service: UploadService,
    creds: &UploadCredentials,
    adif: &str,
) -> Result<String, String> {
    match service {
        UploadService::ClubLog => {
            let client = crate::cloud::clublog::ClubLogClient::new();
            client
                .upload_adif(
                    &creds.clublog_callsign,
                    &creds.clublog_email,
                    &creds.clublog_password,
                    &creds.clublog_api_key,
                    adif,
                )
                .await
                .map_err(|e| e.to_string())
        }
        UploadService::Qrz => {
            crate::cloud::qrz::QrzClient::upload_to_logbook(&creds.qrz_api_key, adif)
                .await
                .map_err(|e| e.to_string())
        }
        UploadService::Eqsl => {
            let client = crate::cloud::eqsl::EqslCardDownloader::new(
                &creds.eqsl_username,
                &creds.eqsl_password,
            );
            client.upload_adif(adif).await.map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> UploadService {
        UploadService::ClubLog
    }

    #[test]
    fn enqueue_assigns_unique_ids_and_pending() {
        let mut s = UploadScheduler::new();
        let a = s.enqueue(svc(), "<ADIF>".into(), 100);
        let b = s.enqueue(svc(), "<ADIF>".into(), 100);
        assert_ne!(a, b);
        assert_eq!(s.pending_count(svc()), 2);
        let job = s.job(a).unwrap();
        assert_eq!(job.status, UploadJobStatus::Pending);
        assert_eq!(job.attempts, 0);
    }

    #[test]
    fn ready_jobs_respects_rate_limit() {
        let mut s = UploadScheduler::new();
        s.enqueue(svc(), "<ADIF>".into(), 0);
        s.enqueue(svc(), "<ADIF>".into(), 0);

        // Obydwa gotowe, ale rate-limit pozwala na jeden.
        let ready = s.ready_jobs(100);
        assert_eq!(ready.len(), 1);

        s.mark_in_flight(ready[0], 100);
        // Tuż po próbie drugi serwis jest zablokowany limitem tempa.
        assert!(s.ready_jobs(101).is_empty());
        // Po upływie rate-limit drugie zadanie staje się gotowe.
        let later = 100 + svc().rate_limit_secs() + 1;
        assert_eq!(s.ready_jobs(later).len(), 1);
    }

    #[test]
    fn backoff_doubles_until_failed() {
        let mut s = UploadScheduler::new();
        s.base_backoff_secs = 10;
        s.max_backoff_secs = 1000;
        let id = s.enqueue(svc(), "<ADIF>".into(), 0);

        s.mark_in_flight(id, 0);
        assert!(s.mark_failure(id, "boom".into(), 0));
        let job = s.job(id).unwrap();
        assert_eq!(job.attempts, 1);
        assert_eq!(job.next_attempt_at, 10);

        // Kolejne nieudane próby.
        s.mark_in_flight(id, 10);
        assert!(s.mark_failure(id, "boom".into(), 10));
        assert_eq!(s.job(id).unwrap().next_attempt_at, 30);

        s.mark_in_flight(id, 30);
        assert!(s.mark_failure(id, "boom".into(), 30));
        assert_eq!(s.job(id).unwrap().next_attempt_at, 70);
    }

    #[test]
    fn exceeds_max_attempts_marks_failed() {
        let mut s = UploadScheduler::new();
        s.max_attempts = 2;
        s.base_backoff_secs = 1;
        let id = s.enqueue(svc(), "<ADIF>".into(), 0);

        s.mark_in_flight(id, 0);
        assert!(s.mark_failure(id, "a".into(), 0));
        s.mark_in_flight(id, 1);
        assert!(!s.mark_failure(id, "b".into(), 1));
        let job = s.job(id).unwrap();
        assert_eq!(job.status, UploadJobStatus::Failed);
        assert_eq!(job.last_error.as_deref(), Some("b"));
    }

    #[test]
    fn success_marks_done_and_purge_removes_it() {
        let mut s = UploadScheduler::new();
        let id = s.enqueue(svc(), "<ADIF>".into(), 0);
        s.mark_in_flight(id, 0);
        s.mark_success(id, 1);
        assert_eq!(s.job(id).unwrap().status, UploadJobStatus::Done);
        assert_eq!(s.purge_done(), 1);
        assert!(s.job(id).is_none());
    }

    #[test]
    fn retry_failed_requeues() {
        let mut s = UploadScheduler::new();
        s.max_attempts = 1;
        let id = s.enqueue(svc(), "<ADIF>".into(), 0);
        s.mark_in_flight(id, 0);
        s.mark_failure(id, "x".into(), 0);
        assert_eq!(s.job(id).unwrap().status, UploadJobStatus::Failed);

        assert_eq!(s.retry_failed(500), 1);
        let job = s.job(id).unwrap();
        assert_eq!(job.status, UploadJobStatus::Pending);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.next_attempt_at, 500);
    }

    #[test]
    fn roundtrip_json_preserves_state() {
        let mut s = UploadScheduler::new();
        s.enqueue(UploadService::Qrz, "<ADIF qrz>".into(), 42);
        s.enqueue(UploadService::Eqsl, "<ADIF eqsl>".into(), 43);
        let json = s.to_json().unwrap();
        let restored = UploadScheduler::from_json(&json).unwrap();
        assert_eq!(restored.jobs.len(), 2);
        assert_eq!(restored.next_id, s.next_id);
        assert_eq!(restored.pending_count(UploadService::Qrz), 1);
        assert_eq!(restored.pending_count(UploadService::Eqsl), 1);
    }

    #[test]
    fn status_all_counts_per_service() {
        let mut s = UploadScheduler::new();
        s.enqueue(UploadService::ClubLog, "<ADIF>".into(), 0);
        s.enqueue(UploadService::Qrz, "<ADIF>".into(), 0);
        s.enqueue(UploadService::Qrz, "<ADIF>".into(), 0);
        let st = s.status_all();
        assert_eq!(st[&UploadService::ClubLog].pending, 1);
        assert_eq!(st[&UploadService::Qrz].pending, 2);
        assert_eq!(st[&UploadService::Eqsl].pending, 0);
    }
}
