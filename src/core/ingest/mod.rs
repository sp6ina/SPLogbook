// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Moduł potoku zapisu QSO (`core::ingest`).

mod enrich;
mod import;
mod origin;

use std::sync::{Arc, Mutex, PoisonError};

use crate::core::awards::AwardsEngine;
use crate::core::database::LogDatabase;
use crate::core::events::{AppEvent, EventBus};
use crate::core::i18n::{Language, Msg, key};
use crate::core::prefix::PrefixMatcher;
use crate::core::qso::{QsoIssue, QsoRecord, QsoWarning};
use crate::core::scp::ScpEngine;

use self::enrich::apply_prefix_info;
pub use self::import::{ImportOptions, ImportReport};
pub use self::origin::{IngestPolicy, PostEffects, QsoOrigin, ValidationMode};

/// Błąd domenowego potoku zapisu QSO (`core::ingest`).
#[derive(Debug)]
pub enum IngestError {
    EmptyCallsign,
    Invalid(QsoIssue),
    Storage(rusqlite::Error),
}

impl IngestError {
    /// Zwraca komunikat błędu przeznaczony do lokalizacji w warstwie prezentacji.
    pub fn msg(&self) -> Msg {
        match self {
            Self::EmptyCallsign => Msg::new(key::QSO_ISSUE_EMPTY_CALLSIGN),
            Self::Invalid(issue) => issue.msg(),
            Self::Storage(error) => Msg::new(key::MSG_QSO_SAVE_FAILED).arg("error", error),
        }
    }
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg().text(Language::En))
    }
}

impl std::error::Error for IngestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EmptyCallsign => None,
            Self::Invalid(issue) => Some(issue),
            Self::Storage(error) => Some(error),
        }
    }
}

/// Wynik pomyślnego zapisu pojedynczego QSO w potoku domenowym.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestOutcome {
    pub origin: QsoOrigin,
    pub qso: QsoRecord,
    pub warnings: Vec<QsoWarning>,
    pub advisory_issue: Option<QsoIssue>,
    pub effects: PostEffects,
}

/// Synchroniczna usługa domenowa obsługująca walidację, wzbogacenie i zapis QSO.
#[derive(Clone)]
pub struct QsoIngestor {
    db: Arc<Mutex<LogDatabase>>,
    awards: Arc<Mutex<AwardsEngine>>,
    scp: Arc<Mutex<ScpEngine>>,
    prefixes: Arc<PrefixMatcher>,
    events: EventBus,
}

impl QsoIngestor {
    /// Tworzy nową instancję `QsoIngestor` ze współdzielonymi uchwytami serwisów domenowych.
    pub fn new(
        db: Arc<Mutex<LogDatabase>>,
        awards: Arc<Mutex<AwardsEngine>>,
        scp: Arc<Mutex<ScpEngine>>,
        prefixes: Arc<PrefixMatcher>,
        events: EventBus,
    ) -> Self {
        Self {
            db,
            awards,
            scp,
            prefixes,
            events,
        }
    }

    /// Przetwarza i zapisuje pojedyncze QSO zgodnie z polityką dla wskazanego źródła (`origin`).
    ///
    /// Kolejność kroków:
    /// 1. Weryfikacja niepustego znaku wywoławczego (`EmptyCallsign`).
    /// 2. Wzbogacenie DXCC/kraju/kontynentu/stref z `PrefixMatcher`, wyłącznie gdy `qso.dxcc.is_none()`.
    /// 3. Walidacja twarda (`Reject`) lub doradcza (`Advisory`) według polityki źródła.
    /// 4. Wyznaczenie ostrzeżeń miękkich (`warnings()`).
    /// 5. Zapis rekordu w `LogDatabase` i nadanie `qso.id`.
    /// 6. Publikacja zdarzenia `AppEvent::QsoLogged` na `EventBus`.
    /// 7. Rejestracja rekordu w `AwardsEngine`.
    /// 8. Dodanie znaku wywoławczego do `ScpEngine`.
    /// 9. Zwrócenie `IngestOutcome` z flagami `PostEffects` wynikającymi z polityki źródła.
    pub fn ingest(
        &self,
        mut qso: QsoRecord,
        origin: QsoOrigin,
    ) -> Result<IngestOutcome, IngestError> {
        if qso.callsign.trim().is_empty() {
            return Err(IngestError::EmptyCallsign);
        }

        if qso.dxcc.is_none()
            && let Some(info) = self.prefixes.lookup(&qso.callsign)
        {
            apply_prefix_info(&mut qso, &info);
        }

        let policy = origin.policy();
        let advisory_issue = match qso.validate() {
            Ok(()) => None,
            Err(issue) => match policy.validation {
                ValidationMode::Reject => return Err(IngestError::Invalid(issue)),
                ValidationMode::Advisory => Some(issue),
            },
        };

        let warnings = qso.warnings();

        {
            let db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
            let id = db.insert_qso(&qso).map_err(IngestError::Storage)?;
            qso.id = Some(id);
        }

        self.events.publish(AppEvent::QsoLogged {
            callsign: qso.callsign.clone(),
            band: qso.band.clone(),
            mode: qso.mode.clone(),
            frequency_hz: (qso.freq.unwrap_or(0.0) * 1_000_000.0) as u64,
            time_utc: qso.time_on.clone(),
        });

        {
            let mut awards = self.awards.lock().unwrap_or_else(PoisonError::into_inner);
            awards.register_qso_record(&qso);
        }

        {
            let mut scp = self.scp.lock().unwrap_or_else(PoisonError::into_inner);
            scp.insert(&qso.callsign);
        }

        Ok(IngestOutcome {
            origin,
            qso,
            warnings,
            advisory_issue,
            effects: policy.effects,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::{Duration, Instant};

    use tokio::sync::broadcast;
    use tokio::sync::broadcast::error::TryRecvError;

    use super::{IngestError, QsoIngestor, QsoOrigin};
    use crate::core::awards::AwardsEngine;
    use crate::core::database::{Journal, LogDatabase};
    use crate::core::events::{AppEvent, EventBus};
    use crate::core::prefix::PrefixMatcher;
    use crate::core::qso::{QsoIssue, QsoRecord, QsoWarning};
    use crate::core::scp::ScpEngine;

    static NEXT_MEMDB_ID: AtomicUsize = AtomicUsize::new(1);

    pub(super) struct TestFixture {
        pub(super) ingestor: QsoIngestor,
        pub(super) events_rx: broadcast::Receiver<AppEvent>,
    }

    impl TestFixture {
        pub(super) fn new() -> Self {
            Self::with_prefixes(build_test_prefix_matcher())
        }

        fn with_prefixes(prefixes: PrefixMatcher) -> Self {
            let db = match LogDatabase::open_in_memory() {
                Ok(db) => Arc::new(Mutex::new(db)),
                Err(e) => panic!("failed to open in-memory LogDatabase: {e}"),
            };
            let awards = Arc::new(Mutex::new(AwardsEngine::new()));
            let scp = Arc::new(Mutex::new(ScpEngine::new()));
            let prefixes = Arc::new(prefixes);
            let events = EventBus::new(16);
            let events_rx = events.subscribe();
            let ingestor = QsoIngestor::new(db, awards, scp, prefixes, events);
            Self {
                ingestor,
                events_rx,
            }
        }

        pub(super) fn count_qsos(&self) -> usize {
            let db = self
                .ingestor
                .db
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            match db.count_all() {
                Ok(n) => n,
                Err(e) => panic!("count_all failed: {e}"),
            }
        }

        pub(super) fn get_qso(&self, id: i64) -> Option<QsoRecord> {
            let db = self
                .ingestor
                .db
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            match db.get_qso_by_id(id) {
                Ok(qso) => qso,
                Err(e) => panic!("get_qso_by_id failed: {e}"),
            }
        }

        pub(super) fn create_journal(&self, id: &str) {
            let db = self
                .ingestor
                .db
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let journal = Journal {
                id: id.to_string(),
                name: format!("Test Journal {id}"),
                is_default: false,
                ..Journal::default()
            };
            if let Err(e) = db.create_journal(&journal) {
                panic!("create_journal({id}) failed: {e}");
            }
        }

        pub(super) fn qsos_for_journal(&self, journal_id: &str) -> Vec<QsoRecord> {
            let db = self
                .ingestor
                .db
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            match db.get_recent_qsos_for_journal(journal_id, 1000) {
                Ok(qsos) => qsos,
                Err(e) => panic!("get_recent_qsos_for_journal({journal_id}) failed: {e}"),
            }
        }
    }

    fn build_test_prefix_matcher() -> PrefixMatcher {
        let id = NEXT_MEMDB_ID.fetch_add(1, Ordering::Relaxed);
        let uri = format!("file:ingest_prefix_test_{id}?mode=memory&cache=shared");
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
            | rusqlite::OpenFlags::SQLITE_OPEN_URI;
        let conn = match rusqlite::Connection::open_with_flags(&uri, flags) {
            Ok(c) => c,
            Err(e) => panic!("failed to open shared in-memory prefix db: {e}"),
        };
        if let Err(e) = conn.execute_batch(
            "CREATE TABLE UniqueCalls (
                Callsign TEXT,
                Country TEXT,
                ARRLPrefix TEXT,
                DXCC TEXT,
                Continent TEXT,
                CQZone INTEGER,
                ITUZone INTEGER,
                Latitude TEXT,
                Longitude TEXT,
                Enable TEXT
            );
            CREATE TABLE CountryDataEx (
                Country TEXT,
                ARRLPrefix TEXT,
                DXCC INTEGER,
                Continent TEXT,
                CQZone INTEGER,
                ITUZone INTEGER,
                Latitude TEXT,
                Longitude TEXT,
                PrefixList TEXT,
                EndDate TEXT
            );
            CREATE TABLE Province (
                Country TEXT,
                ARRLPrefix TEXT,
                DXCC TEXT,
                Continent TEXT,
                CQZone INTEGER,
                ITUZone INTEGER,
                Latitude TEXT,
                Longitude TEXT,
                PrefixList TEXT,
                Comment TEXT,
                EndDate TEXT
            );
            INSERT INTO UniqueCalls VALUES
                ('SP6INA', 'Poland', 'SP', '269', 'EU', 15, 28, '51.1N', '17.0E', '1'),
                ('K1ABC', 'United States', 'K', '291', 'NA', 5, 8, '42.0N', '71.0W', '1'),
                ('ZERO1AA', '', 'ZR', '0', '   ', 0, 0, '0.0N', '0.0E', '1');",
        ) {
            panic!("failed to populate test prefix tables: {e}");
        }

        match PrefixMatcher::load_from_db(&uri) {
            Ok(matcher) => matcher,
            Err(e) => panic!("failed to load PrefixMatcher from URI: {e}"),
        }
    }

    pub(super) fn sample_valid_qso(callsign: &str) -> QsoRecord {
        let mut qso = QsoRecord::new(callsign, "20m", "CW");
        qso.qso_date = "20261013".to_string();
        qso.time_on = "123456".to_string();
        qso.freq = Some(14.025);
        qso.journal_id = Some("DEFAULT".to_string());
        qso
    }

    #[test]
    fn ingest_persists_and_registers_everywhere() {
        let mut fixture = TestFixture::new();
        let mut qso = sample_valid_qso("SP6INA");
        qso.iota = Some("BAD-IOTA".to_string());

        let outcome = match fixture.ingestor.ingest(qso, QsoOrigin::Form) {
            Ok(outcome) => outcome,
            Err(e) => panic!("expected successful ingest, got error: {e}"),
        };

        let Some(id) = outcome.qso.id else {
            panic!("expected qso.id to be assigned after insert");
        };
        assert!(id > 0);
        assert_eq!(outcome.origin, QsoOrigin::Form);
        assert_eq!(outcome.advisory_issue, None);
        assert_eq!(
            outcome.warnings,
            vec![QsoWarning::Iota("BAD-IOTA".to_string())]
        );

        assert_eq!(fixture.count_qsos(), 1);
        let Some(stored) = fixture.get_qso(id) else {
            panic!("expected stored QSO in database for id={id}");
        };
        assert_eq!(stored.id, Some(id));
        assert_eq!(stored.callsign, "SP6INA");
        assert_eq!(stored.dxcc, Some(269));
        assert_eq!(stored.country.as_deref(), Some("Poland"));
        assert_eq!(stored.continent.as_deref(), Some("EU"));
        assert_eq!(stored.cqz, Some(15));
        assert_eq!(stored.ituz, Some(28));

        {
            let awards = fixture
                .ingestor
                .awards
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert!(awards.worked_calls.contains(&(
                "SP6INA".to_string(),
                "20m".to_string(),
                "CW".to_string()
            )));
            assert!(awards.worked_dxcc_all.contains(&269));
        }

        {
            let scp = fixture
                .ingestor
                .scp
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(scp.search("SP6INA", 5), vec!["SP6INA".to_string()]);
        }

        match fixture.events_rx.try_recv() {
            Ok(AppEvent::QsoLogged {
                callsign,
                band,
                mode,
                frequency_hz,
                time_utc,
            }) => {
                assert_eq!(callsign, "SP6INA");
                assert_eq!(band, "20m");
                assert_eq!(mode, "CW");
                assert_eq!(frequency_hz, 14_025_000);
                assert_eq!(time_utc, "123456");
            }
            other => panic!("expected AppEvent::QsoLogged, got {other:?}"),
        }
    }

    #[test]
    fn ingest_rejects_invalid_for_form_and_stores_nothing() {
        let mut fixture = TestFixture::new();
        let mut qso = sample_valid_qso("SP6INA");
        qso.band = String::new();

        let result = fixture.ingestor.ingest(qso, QsoOrigin::Form);
        match result {
            Err(IngestError::Invalid(QsoIssue::EmptyBand)) => {}
            other => panic!("expected Err(IngestError::Invalid(EmptyBand)), got {other:?}"),
        }

        assert_eq!(fixture.count_qsos(), 0);
        assert!(matches!(
            fixture.events_rx.try_recv(),
            Err(TryRecvError::Empty)
        ));

        {
            let awards = fixture
                .ingestor
                .awards
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(awards.worked_calls.len(), 0);
            assert_eq!(awards.worked_dxcc_all.len(), 0);
        }

        {
            let scp = fixture
                .ingestor
                .scp
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(scp.len(), 0);
        }
    }

    #[test]
    fn ingest_advisory_stores_invalid_record_and_reports_issue() {
        let mut fixture = TestFixture::new();
        let qso = sample_valid_qso("<PA3XYZ>");

        let outcome = match fixture.ingestor.ingest(qso, QsoOrigin::WsjtX) {
            Ok(outcome) => outcome,
            Err(e) => panic!("expected advisory ingest to succeed, got error: {e}"),
        };

        assert_eq!(outcome.origin, QsoOrigin::WsjtX);
        assert_eq!(
            outcome.advisory_issue,
            Some(QsoIssue::CallsignChars("<PA3XYZ>".to_string()))
        );
        let Some(id) = outcome.qso.id else {
            panic!("expected advisory record to have assigned id");
        };
        assert!(id > 0);
        assert_eq!(fixture.count_qsos(), 1);

        let Some(stored) = fixture.get_qso(id) else {
            panic!("expected stored QSO in database");
        };
        assert_eq!(stored.callsign, "<PA3XYZ>");

        match fixture.events_rx.try_recv() {
            Ok(AppEvent::QsoLogged { callsign, .. }) => {
                assert_eq!(callsign, "<PA3XYZ>");
            }
            other => panic!("expected AppEvent::QsoLogged for advisory ingest, got {other:?}"),
        }

        {
            let awards = fixture
                .ingestor
                .awards
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert!(awards.worked_calls.contains(&(
                "<PA3XYZ>".to_string(),
                "20m".to_string(),
                "CW".to_string()
            )));
        }

        {
            let scp = fixture
                .ingestor
                .scp
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(scp.search("<PA3XYZ>", 5), vec!["<PA3XYZ>".to_string()]);
        }
    }

    #[test]
    fn ingest_empty_callsign() {
        let mut fixture = TestFixture::new();
        let mut qso = sample_valid_qso("   ");
        // Ustaw również puste pasmo — gdyby potok przeszedł do validate(),
        // albo wywołał zapis, wynik byłby inny niż EmptyCallsign.
        qso.band = String::new();

        let result = fixture.ingestor.ingest(qso, QsoOrigin::WsjtX);
        assert!(
            matches!(result, Err(IngestError::EmptyCallsign)),
            "expected Err(IngestError::EmptyCallsign), got {result:?}"
        );

        assert_eq!(fixture.count_qsos(), 0);
        assert!(matches!(
            fixture.events_rx.try_recv(),
            Err(TryRecvError::Empty)
        ));

        {
            let awards = fixture
                .ingestor
                .awards
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(awards.worked_calls.len(), 0);
        }

        {
            let scp = fixture
                .ingestor
                .scp
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(scp.len(), 0);
        }
    }

    #[test]
    fn ingest_storage_error_has_no_side_effects() {
        let mut fixture = TestFixture::new();
        let mut qso = sample_valid_qso("SP6INA");
        qso.journal_id = Some("NIE_ISTNIEJE".to_string());

        let result = fixture.ingestor.ingest(qso, QsoOrigin::Form);
        match result {
            Err(ref ingest_err @ IngestError::Storage(ref err)) => {
                assert!(std::error::Error::source(ingest_err).is_some());
                assert_ne!(err.to_string(), "");
            }
            other => panic!("expected Err(IngestError::Storage(_)) on FK violation, got {other:?}"),
        }

        assert_eq!(fixture.count_qsos(), 0);
        assert!(matches!(
            fixture.events_rx.try_recv(),
            Err(TryRecvError::Empty)
        ));

        {
            let awards = fixture
                .ingestor
                .awards
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(awards.worked_calls.len(), 0);
            assert_eq!(awards.worked_dxcc_all.len(), 0);
        }

        {
            let scp = fixture
                .ingestor
                .scp
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(scp.len(), 0);
        }
    }

    #[test]
    fn ingest_enrichment_fills_only_missing_valid_values() {
        let fixture = TestFixture::new();

        // 1. Wszystkie pola DXCC brakujące -> uzupełnione wartościami > 0 / niepustymi.
        let qso_missing = sample_valid_qso("SP6INA");
        let outcome_missing = match fixture.ingestor.ingest(qso_missing, QsoOrigin::Form) {
            Ok(outcome) => outcome,
            Err(e) => panic!("unexpected error for SP6INA: {e}"),
        };
        assert_eq!(outcome_missing.qso.dxcc, Some(269));
        assert_eq!(outcome_missing.qso.country.as_deref(), Some("Poland"));
        assert_eq!(outcome_missing.qso.continent.as_deref(), Some("EU"));
        assert_eq!(outcome_missing.qso.cqz, Some(15));
        assert_eq!(outcome_missing.qso.ituz, Some(28));

        // 2. Istniejące dxcc (Some(230)) -> brak wywołania wzbogacania i brak nadpisania pól.
        let mut qso_preset_dxcc = sample_valid_qso("SP6INA");
        qso_preset_dxcc.dxcc = Some(230);
        qso_preset_dxcc.country = Some("Germany".to_string());
        qso_preset_dxcc.continent = Some("AF".to_string());
        qso_preset_dxcc.cqz = Some(14);
        qso_preset_dxcc.ituz = Some(27);

        let outcome_preset = match fixture.ingestor.ingest(qso_preset_dxcc, QsoOrigin::Form) {
            Ok(outcome) => outcome,
            Err(e) => panic!("unexpected error for preset dxcc: {e}"),
        };
        assert_eq!(outcome_preset.qso.dxcc, Some(230));
        assert_eq!(outcome_preset.qso.country.as_deref(), Some("Germany"));
        assert_eq!(outcome_preset.qso.continent.as_deref(), Some("AF"));
        assert_eq!(outcome_preset.qso.cqz, Some(14));
        assert_eq!(outcome_preset.qso.ituz, Some(27));

        // 3. Brak dxcc, ale część pozostałych pól już ustawiona (lub puste napisy):
        // uzupełniane wyłącznie brakujące/puste pola, istniejące poprawne zachowane.
        let mut qso_partial = sample_valid_qso("K1ABC");
        qso_partial.dxcc = None;
        qso_partial.country = Some("Custom USA".to_string());
        qso_partial.continent = Some("   ".to_string());
        qso_partial.cqz = Some(4);
        qso_partial.ituz = None;

        let outcome_partial = match fixture.ingestor.ingest(qso_partial, QsoOrigin::Form) {
            Ok(outcome) => outcome,
            Err(e) => panic!("unexpected error for partial enrichment: {e}"),
        };
        assert_eq!(outcome_partial.qso.dxcc, Some(291));
        assert_eq!(outcome_partial.qso.country.as_deref(), Some("Custom USA"));
        assert_eq!(outcome_partial.qso.continent.as_deref(), Some("NA"));
        assert_eq!(outcome_partial.qso.cqz, Some(4));
        assert_eq!(outcome_partial.qso.ituz, Some(8));

        // 4. Lookup zwraca dxcc = 0, cqz = 0, ituz = 0 oraz puste country/continent:
        // żadna z tych niepoprawnych wartości nie zostaje zapisana.
        let qso_zero = sample_valid_qso("ZERO1AA");
        let outcome_zero = match fixture.ingestor.ingest(qso_zero, QsoOrigin::Form) {
            Ok(outcome) => outcome,
            Err(e) => panic!("unexpected error for ZERO1AA: {e}"),
        };
        assert_eq!(outcome_zero.qso.dxcc, None);
        assert_eq!(outcome_zero.qso.country, None);
        assert_eq!(outcome_zero.qso.continent, None);
        assert_eq!(outcome_zero.qso.cqz, None);
        assert_eq!(outcome_zero.qso.ituz, None);
    }

    #[test]
    fn ingest_publishes_event_once_with_expected_payload() {
        let mut fixture = TestFixture::new();
        let mut qso = QsoRecord::new("SP6INA", "40m", "SSB");
        qso.qso_date = "20261013".to_string();
        qso.time_on = "184512".to_string();
        qso.freq = Some(7.155);
        qso.journal_id = Some("DEFAULT".to_string());

        let outcome = match fixture.ingestor.ingest(qso, QsoOrigin::Form) {
            Ok(outcome) => outcome,
            Err(e) => panic!("expected successful ingest, got error: {e}"),
        };
        assert!(outcome.qso.id.is_some());

        match fixture.events_rx.try_recv() {
            Ok(AppEvent::QsoLogged {
                callsign,
                band,
                mode,
                frequency_hz,
                time_utc,
            }) => {
                assert_eq!(callsign, "SP6INA");
                assert_eq!(band, "40m");
                assert_eq!(mode, "SSB");
                assert_eq!(frequency_hz, 7_155_000);
                assert_eq!(time_utc, "184512");
            }
            other => panic!("expected AppEvent::QsoLogged, got {other:?}"),
        }

        assert!(
            matches!(fixture.events_rx.try_recv(), Err(TryRecvError::Empty)),
            "expected exactly one event to be published"
        );
    }

    #[test]
    fn ingest_runs_steps_in_legacy_order() {
        let mut fixture = TestFixture::new();
        let qso = sample_valid_qso("SP6INA");
        let ingestor_for_thread = fixture.ingestor.clone();

        // Krok A: Trzymamy blokadę `db`, aby potwierdzić, że `event` NIE jest publikowany przed `insert`.
        let db_guard = fixture
            .ingestor
            .db
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let handle = std::thread::spawn(move || ingestor_for_thread.ingest(qso, QsoOrigin::Form));

        std::thread::sleep(Duration::from_millis(30));
        assert!(
            matches!(fixture.events_rx.try_recv(), Err(TryRecvError::Empty)),
            "event must not be published before database insert completes"
        );

        // Krok B: Przed zwolnieniem `db_guard` zajmujemy `awards_guard`.
        // Wątek `ingest` wykona `db.insert_qso`, zwolni blokadę `db`, opublikuje `AppEvent::QsoLogged`
        // i zatrzyma się przed wejściem do `AwardsEngine`.
        let awards_guard = fixture
            .ingestor
            .awards
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        drop(db_guard);

        let deadline = Instant::now() + Duration::from_secs(2);
        let logged_event = loop {
            match fixture.events_rx.try_recv() {
                Ok(ev) => break ev,
                Err(TryRecvError::Empty) => {
                    assert!(
                        Instant::now() < deadline,
                        "timed out waiting for QsoLogged before AwardsEngine lock"
                    );
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(other) => panic!("unexpected broadcast receive error: {other}"),
            }
        };

        match logged_event {
            AppEvent::QsoLogged { ref callsign, .. } => assert_eq!(callsign, "SP6INA"),
            other => panic!("expected AppEvent::QsoLogged, got {other:?}"),
        }

        // W chwili odebrania zdarzenia:
        // 1) rekord jest już w bazie, a blokada `db` została zwolniona (`try_lock` się udaje),
        // 2) `AwardsEngine` jeszcze NIE zarejestrował QSO (`awards_guard` jest pusty),
        // 3) `ScpEngine` jeszcze NIE zawiera znaku (`try_lock` się udaje i jest pusty).
        {
            let Ok(db_now) = fixture.ingestor.db.try_lock() else {
                panic!("db lock must already be released when event is published");
            };
            let count = match db_now.count_all() {
                Ok(n) => n,
                Err(e) => panic!("count_all failed: {e}"),
            };
            assert_eq!(
                count, 1,
                "record must already be inserted in database when QsoLogged is observed"
            );
        }
        assert_eq!(
            awards_guard.worked_calls.len(),
            0,
            "AwardsEngine must not have registered the QSO when QsoLogged is observed"
        );
        {
            let Ok(scp_now) = fixture.ingestor.scp.try_lock() else {
                panic!("scp lock must not be held when event is observed");
            };
            assert_eq!(
                scp_now.len(),
                0,
                "ScpEngine must not have registered the callsign when QsoLogged is observed"
            );
        }

        // Krok C: Zajmujemy `scp_guard` przed zwolnieniem `awards_guard`, aby potwierdzić
        // kolejność `awards` -> `SCP` oraz brak jednoczesnego trzymania blokad `awards` i `scp`.
        let scp_guard = fixture
            .ingestor
            .scp
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        drop(awards_guard);

        let awards_deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(awards_now) = fixture.ingestor.awards.try_lock()
                && awards_now.worked_calls.contains(&(
                    "SP6INA".to_string(),
                    "20m".to_string(),
                    "CW".to_string(),
                ))
            {
                break;
            }
            assert!(
                Instant::now() < awards_deadline,
                "timed out waiting for AwardsEngine registration before ScpEngine lock"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        assert_eq!(
            scp_guard.len(),
            0,
            "ScpEngine must still be empty after AwardsEngine registration while scp lock is held"
        );
        drop(scp_guard);

        let Ok(join_res) = handle.join() else {
            panic!("ingest worker thread panicked");
        };
        let outcome = match join_res {
            Ok(outcome) => outcome,
            Err(e) => panic!("ingest failed: {e}"),
        };
        assert!(outcome.qso.id.is_some());

        let scp_final = fixture
            .ingestor
            .scp
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert_eq!(scp_final.search("SP6INA", 5), vec!["SP6INA".to_string()]);
    }

    #[test]
    fn ingest_outcome_effects_match_policy() {
        const ALL_ORIGINS: [QsoOrigin; 6] = [
            QsoOrigin::Form,
            QsoOrigin::Contest,
            QsoOrigin::WsjtX,
            QsoOrigin::Js8Call,
            QsoOrigin::LanPeer,
            QsoOrigin::RestApi,
        ];

        for origin in ALL_ORIGINS {
            let fixture = TestFixture::new();
            let qso = sample_valid_qso("SP6INA");
            let outcome = match fixture.ingestor.ingest(qso, origin) {
                Ok(outcome) => outcome,
                Err(e) => panic!("ingest failed for origin {origin:?}: {e}"),
            };

            assert_eq!(outcome.origin, origin);
            assert_eq!(outcome.effects, origin.policy().effects);
        }
    }
}
