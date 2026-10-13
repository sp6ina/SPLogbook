// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Zbiorczy import rekordów QSO w potoku `core::ingest` (PIPE-3).

use std::sync::PoisonError;

use crate::core::qso::{QsoIssue, QsoRecord};

use super::enrich::apply_prefix_info;
use super::{IngestError, QsoIngestor};

/// Opcje sterujące zbiorczym importem rekordów QSO (`QsoIngestor::import_batch`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportOptions {
    pub journal_id: String,
    pub enrich: bool,
}

/// Raport z wykonania zbiorczego importu rekordów QSO (`QsoIngestor::import_batch`).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportReport {
    pub inserted: usize,
    pub rejected_invalid: usize,
    pub skipped_duplicates: usize,
    pub first_issue: Option<QsoIssue>,
}

impl QsoIngestor {
    /// Przetwarza i zapisuje zbiorczo partię rekordów QSO (np. z importu ADIF).
    ///
    /// Kolejność kroków:
    /// 1. Pobranie istniejących tożsamości `QsoIdentity` dla `options.journal_id` z bazy
    ///    i natychmiastowe zwolnienie blokady `LogDatabase`.
    /// 2. Dla każdego rekordu w kolejności wejściowej (bez trzymania blokad):
    ///    - przypisanie `qso.journal_id = Some(options.journal_id.clone())`,
    ///    - opcjonalne wzbogacenie brakujących pól z `PrefixMatcher` (gdy `options.enrich == true`
    ///      oraz `qso.dxcc.is_none()`),
    ///    - walidacja `qso.validate()` w trybie `Reject` (zliczenie `rejected_invalid` i zapamiętanie
    ///      pierwszego `QsoIssue`),
    ///    - deduplikacja przez `qso.identity()` względem rekordów w bazie i zaakceptowanych
    ///      wcześniej w tej samej partii (`skipped_duplicates`).
    /// 3. Zapis zaakceptowanych rekordów przez `LogDatabase::batch_insert_qsos` pod pojedynczą
    ///    blokadą bazy (pomijany dla pustej partii).
    /// 4. Rejestracja zapisanych rekordów w `AwardsEngine` pod osobną blokadą.
    /// 5. Dodanie znaków wywoławczych zapisanych rekordów do `ScpEngine` pod osobną blokadą.
    pub fn import_batch(
        &self,
        qsos: Vec<QsoRecord>,
        options: &ImportOptions,
    ) -> Result<ImportReport, IngestError> {
        let mut seen_identities = {
            let db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
            db.existing_identities(&options.journal_id)
                .map_err(IngestError::Storage)?
        };

        let mut accepted = Vec::with_capacity(qsos.len());
        let mut rejected_invalid = 0usize;
        let mut skipped_duplicates = 0usize;
        let mut first_issue: Option<QsoIssue> = None;

        for mut qso in qsos {
            qso.journal_id = Some(options.journal_id.clone());

            if options.enrich
                && qso.dxcc.is_none()
                && let Some(info) = self.prefixes.lookup(&qso.callsign)
            {
                apply_prefix_info(&mut qso, &info);
            }

            if let Err(issue) = qso.validate() {
                rejected_invalid += 1;
                if first_issue.is_none() {
                    first_issue = Some(issue);
                }
                continue;
            }

            let identity = qso.identity();
            if !seen_identities.insert(identity) {
                skipped_duplicates += 1;
                continue;
            }

            accepted.push(qso);
        }

        if accepted.is_empty() {
            return Ok(ImportReport {
                inserted: 0,
                rejected_invalid,
                skipped_duplicates,
                first_issue,
            });
        }

        {
            let db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
            db.batch_insert_qsos(&accepted)
                .map_err(IngestError::Storage)?;
        }

        let inserted = accepted.len();

        {
            let mut awards = self.awards.lock().unwrap_or_else(PoisonError::into_inner);
            for qso in &accepted {
                awards.register_qso_record(qso);
            }
        }

        {
            let mut scp = self.scp.lock().unwrap_or_else(PoisonError::into_inner);
            for qso in &accepted {
                scp.insert(&qso.callsign);
            }
        }

        Ok(ImportReport {
            inserted,
            rejected_invalid,
            skipped_duplicates,
            first_issue,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::PoisonError;

    use tokio::sync::broadcast::error::TryRecvError;

    use super::super::tests::{TestFixture, sample_valid_qso};
    use super::{ImportOptions, ImportReport};
    use crate::core::ingest::IngestError;
    use crate::core::qso::{QsoIssue, QsoRecord};

    fn default_options(enrich: bool) -> ImportOptions {
        ImportOptions {
            journal_id: "DEFAULT".to_string(),
            enrich,
        }
    }

    #[test]
    fn import_skips_duplicates_across_hhmm_and_hhmmss() {
        let fixture = TestFixture::new();

        let mut qso_hhmm = sample_valid_qso("SP6INA");
        qso_hhmm.time_on = "1234".to_string();

        let mut qso_hhmmss = sample_valid_qso("SP6INA");
        qso_hhmmss.time_on = "123456".to_string();

        let input_len = 2;
        let report = match fixture
            .ingestor
            .import_batch(vec![qso_hhmm, qso_hhmmss], &default_options(false))
        {
            Ok(report) => report,
            Err(e) => panic!("expected import_batch to succeed, got error: {e}"),
        };

        assert_eq!(
            report,
            ImportReport {
                inserted: 1,
                rejected_invalid: 0,
                skipped_duplicates: 1,
                first_issue: None,
            }
        );
        assert_eq!(
            report.inserted + report.rejected_invalid + report.skipped_duplicates,
            input_len
        );
        assert_eq!(fixture.count_qsos(), 1);

        let stored = fixture.qsos_for_journal("DEFAULT");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].callsign, "SP6INA");
        assert_eq!(stored[0].time_on, "1234");

        // Wariant duplikatu względem rekordu już znajdującego się w bazie (HHMMSS vs zapisane HHMM).
        let mut qso_db_dup = sample_valid_qso("SP6INA");
        qso_db_dup.time_on = "123459".to_string();

        let second_report = match fixture
            .ingestor
            .import_batch(vec![qso_db_dup], &default_options(false))
        {
            Ok(report) => report,
            Err(e) => panic!("expected second import_batch to succeed, got error: {e}"),
        };

        assert_eq!(
            second_report,
            ImportReport {
                inserted: 0,
                rejected_invalid: 0,
                skipped_duplicates: 1,
                first_issue: None,
            }
        );
        assert_eq!(fixture.count_qsos(), 1);
    }

    #[test]
    fn import_counts_invalid_and_keeps_first_issue() {
        let fixture = TestFixture::new();

        let qso_valid = sample_valid_qso("SP6INA");

        let mut qso_invalid_first = sample_valid_qso("K1ABC");
        qso_invalid_first.band = String::new();

        let mut qso_invalid_second = sample_valid_qso("DL1XYZ");
        qso_invalid_second.cqz = Some(99);

        let batch = vec![qso_valid, qso_invalid_first, qso_invalid_second];
        let input_len = batch.len();

        let report = match fixture
            .ingestor
            .import_batch(batch, &default_options(false))
        {
            Ok(report) => report,
            Err(e) => panic!("expected import_batch to succeed, got error: {e}"),
        };

        assert_eq!(report.inserted, 1);
        assert_eq!(report.rejected_invalid, 2);
        assert_eq!(report.skipped_duplicates, 0);
        assert_eq!(report.first_issue, Some(QsoIssue::EmptyBand));
        assert_eq!(
            report.inserted + report.rejected_invalid + report.skipped_duplicates,
            input_len
        );

        assert_eq!(fixture.count_qsos(), 1);
        let stored = fixture.qsos_for_journal("DEFAULT");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].callsign, "SP6INA");
    }

    #[test]
    fn import_assigns_journal() {
        let fixture = TestFixture::new();
        fixture.create_journal("CONTEST_2026");

        let mut qso_none_journal = sample_valid_qso("SP6INA");
        qso_none_journal.journal_id = None;
        qso_none_journal.time_on = "120000".to_string();

        let mut qso_other_journal = sample_valid_qso("K1ABC");
        qso_other_journal.journal_id = Some("INNY".to_string());
        qso_other_journal.time_on = "120500".to_string();

        let options = ImportOptions {
            journal_id: "CONTEST_2026".to_string(),
            enrich: false,
        };

        let report = match fixture
            .ingestor
            .import_batch(vec![qso_none_journal, qso_other_journal], &options)
        {
            Ok(report) => report,
            Err(e) => panic!("expected import_batch to succeed, got error: {e}"),
        };

        assert_eq!(
            report,
            ImportReport {
                inserted: 2,
                rejected_invalid: 0,
                skipped_duplicates: 0,
                first_issue: None,
            }
        );

        assert_eq!(fixture.qsos_for_journal("DEFAULT").len(), 0);
        let stored = fixture.qsos_for_journal("CONTEST_2026");
        assert_eq!(stored.len(), 2);
        for qso in &stored {
            assert_eq!(qso.journal_id.as_deref(), Some("CONTEST_2026"));
        }
    }

    #[test]
    fn import_enrich_option() {
        // Przypadek 1: enrich = false -> brak lookupu i brak modyfikacji pól wzbogacanych.
        let fixture_no_enrich = TestFixture::new();
        let mut qso_unmodified = sample_valid_qso("SP6INA");
        qso_unmodified.dxcc = None;
        qso_unmodified.country = None;
        qso_unmodified.continent = None;
        qso_unmodified.cqz = None;
        qso_unmodified.ituz = None;

        let report_no_enrich = match fixture_no_enrich
            .ingestor
            .import_batch(vec![qso_unmodified], &default_options(false))
        {
            Ok(report) => report,
            Err(e) => panic!("expected import_batch(enrich=false) to succeed, got error: {e}"),
        };
        assert_eq!(report_no_enrich.inserted, 1);

        let stored_no_enrich = fixture_no_enrich.qsos_for_journal("DEFAULT");
        assert_eq!(stored_no_enrich.len(), 1);
        assert_eq!(stored_no_enrich[0].dxcc, None);
        assert_eq!(stored_no_enrich[0].country, None);
        assert_eq!(stored_no_enrich[0].continent, None);
        assert_eq!(stored_no_enrich[0].cqz, None);
        assert_eq!(stored_no_enrich[0].ituz, None);

        // Przypadek 2: enrich = true -> uzupełniane tylko brakujące poprawne pola, istniejące zachowane.
        let fixture_enrich = TestFixture::new();

        // 2a. Wszystkie pola DXCC brakujące -> pełne wzbogacenie z PrefixMatcher.
        let mut qso_missing = sample_valid_qso("SP6INA");
        qso_missing.time_on = "120000".to_string();

        // 2b. Brak dxcc, ale część pól ustawiona -> uzupełnienie tylko brakujących, bez nadpisywania.
        let mut qso_partial = sample_valid_qso("K1ABC");
        qso_partial.time_on = "120100".to_string();
        qso_partial.dxcc = None;
        qso_partial.country = Some("Custom USA".to_string());
        qso_partial.continent = Some("   ".to_string());
        qso_partial.cqz = Some(4);
        qso_partial.ituz = None;

        // 2c. Istniejące dxcc -> brak nadpisania jakichkolwiek pól.
        let mut qso_preset = sample_valid_qso("SP6INA");
        qso_preset.band = "40m".to_string();
        qso_preset.time_on = "120200".to_string();
        qso_preset.dxcc = Some(230);
        qso_preset.country = Some("Germany".to_string());
        qso_preset.continent = Some("AF".to_string());
        qso_preset.cqz = Some(14);
        qso_preset.ituz = Some(27);

        // 2d. Rekord zwracający zerowe/puste wartości z PrefixMatcher -> brak zapisu 0/pustych.
        let mut qso_zero = sample_valid_qso("ZERO1AA");
        qso_zero.time_on = "120300".to_string();

        let report_enrich = match fixture_enrich.ingestor.import_batch(
            vec![qso_missing, qso_partial, qso_preset, qso_zero],
            &default_options(true),
        ) {
            Ok(report) => report,
            Err(e) => panic!("expected import_batch(enrich=true) to succeed, got error: {e}"),
        };
        assert_eq!(report_enrich.inserted, 4);

        let stored = fixture_enrich.qsos_for_journal("DEFAULT");
        assert_eq!(stored.len(), 4);

        let find_by_time = |time_on: &str| -> &QsoRecord {
            match stored.iter().find(|q| q.time_on == time_on) {
                Some(q) => q,
                None => panic!("expected stored QSO with time_on={time_on}"),
            }
        };

        let s_missing = find_by_time("120000");
        assert_eq!(s_missing.dxcc, Some(269));
        assert_eq!(s_missing.country.as_deref(), Some("Poland"));
        assert_eq!(s_missing.continent.as_deref(), Some("EU"));
        assert_eq!(s_missing.cqz, Some(15));
        assert_eq!(s_missing.ituz, Some(28));

        let s_partial = find_by_time("120100");
        assert_eq!(s_partial.dxcc, Some(291));
        assert_eq!(s_partial.country.as_deref(), Some("Custom USA"));
        assert_eq!(s_partial.continent.as_deref(), Some("NA"));
        assert_eq!(s_partial.cqz, Some(4));
        assert_eq!(s_partial.ituz, Some(8));

        let s_preset = find_by_time("120200");
        assert_eq!(s_preset.dxcc, Some(230));
        assert_eq!(s_preset.country.as_deref(), Some("Germany"));
        assert_eq!(s_preset.continent.as_deref(), Some("AF"));
        assert_eq!(s_preset.cqz, Some(14));
        assert_eq!(s_preset.ituz, Some(27));

        let s_zero = find_by_time("120300");
        assert_eq!(s_zero.dxcc, None);
        assert_eq!(s_zero.country, None);
        assert_eq!(s_zero.continent, None);
        assert_eq!(s_zero.cqz, None);
        assert_eq!(s_zero.ituz, None);
    }

    #[test]
    fn import_registers_awards_and_scp() {
        let mut fixture = TestFixture::new();

        let mut qso_valid_1 = sample_valid_qso("SP6INA");
        qso_valid_1.time_on = "120000".to_string();

        let mut qso_valid_2 = QsoRecord::new("K1ABC", "40m", "SSB");
        qso_valid_2.qso_date = "20261013".to_string();
        qso_valid_2.time_on = "120500".to_string();
        qso_valid_2.freq = Some(7.150);

        let mut qso_duplicate = sample_valid_qso("SP6INA");
        qso_duplicate.time_on = "1200".to_string();

        let mut qso_invalid = sample_valid_qso("DL1XYZ");
        qso_invalid.band = String::new();
        qso_invalid.time_on = "121000".to_string();

        let batch = vec![qso_valid_1, qso_valid_2, qso_duplicate, qso_invalid];
        let input_len = batch.len();

        let report = match fixture.ingestor.import_batch(batch, &default_options(true)) {
            Ok(report) => report,
            Err(e) => panic!("expected import_batch to succeed, got error: {e}"),
        };

        assert_eq!(
            report,
            ImportReport {
                inserted: 2,
                rejected_invalid: 1,
                skipped_duplicates: 1,
                first_issue: Some(QsoIssue::EmptyBand),
            }
        );
        assert_eq!(
            report.inserted + report.rejected_invalid + report.skipped_duplicates,
            input_len
        );
        assert_eq!(fixture.count_qsos(), 2);

        {
            let awards = fixture
                .ingestor
                .awards
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(awards.worked_calls.len(), 2);
            assert!(awards.worked_calls.contains(&(
                "SP6INA".to_string(),
                "20m".to_string(),
                "CW".to_string()
            )));
            assert!(awards.worked_calls.contains(&(
                "K1ABC".to_string(),
                "40m".to_string(),
                "SSB".to_string()
            )));
            assert!(awards.worked_dxcc_all.contains(&269));
            assert!(awards.worked_dxcc_all.contains(&291));
            assert_eq!(
                awards.details_wpx.get("SP6").map(Vec::len),
                Some(1),
                "duplicate record must not be registered a second time in AwardsEngine"
            );
        }

        {
            let scp = fixture
                .ingestor
                .scp
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            assert_eq!(scp.len(), 2);
            assert_eq!(scp.search("SP6INA", 5), vec!["SP6INA".to_string()]);
            assert_eq!(scp.search("K1ABC", 5), vec!["K1ABC".to_string()]);
            assert_eq!(scp.search("DL1XYZ", 5), Vec::<String>::new());
        }

        assert!(
            matches!(fixture.events_rx.try_recv(), Err(TryRecvError::Empty)),
            "import_batch must not publish per-record events on EventBus"
        );
    }

    #[test]
    fn import_empty_batch_returns_zero_report() {
        let mut fixture = TestFixture::new();

        let report = match fixture
            .ingestor
            .import_batch(Vec::new(), &default_options(true))
        {
            Ok(report) => report,
            Err(e) => panic!("expected empty import_batch to succeed, got error: {e}"),
        };

        assert_eq!(
            report,
            ImportReport {
                inserted: 0,
                rejected_invalid: 0,
                skipped_duplicates: 0,
                first_issue: None,
            }
        );
        assert_eq!(fixture.count_qsos(), 0);
        assert!(matches!(
            fixture.events_rx.try_recv(),
            Err(TryRecvError::Empty)
        ));
    }

    #[test]
    fn import_storage_error_does_not_update_awards_or_scp() {
        let mut fixture = TestFixture::new();
        let qso_1 = sample_valid_qso("SP6INA");
        let mut qso_2 = sample_valid_qso("K1ABC");
        qso_2.time_on = "124000".to_string();

        let invalid_journal_options = ImportOptions {
            journal_id: "NIE_ISTNIEJE".to_string(),
            enrich: true,
        };

        let result = fixture
            .ingestor
            .import_batch(vec![qso_1, qso_2], &invalid_journal_options);
        assert!(
            matches!(result, Err(IngestError::Storage(_))),
            "expected Err(IngestError::Storage(_)) on FK violation, got {result:?}"
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
    fn import_report_accounts_for_every_input_record() {
        let fixture = TestFixture::new();

        let mut qso_empty_call = sample_valid_qso("   ");
        qso_empty_call.time_on = "110000".to_string();

        let mut qso_ok_1 = sample_valid_qso("SP6INA");
        qso_ok_1.time_on = "110500".to_string();

        let mut qso_dup_1 = sample_valid_qso("sp6ina");
        qso_dup_1.time_on = "1105".to_string();

        let mut qso_bad_band = sample_valid_qso("K1ABC");
        qso_bad_band.band = String::new();

        let mut qso_ok_2 = sample_valid_qso("K1ABC");
        qso_ok_2.time_on = "111000".to_string();

        let batch = vec![qso_empty_call, qso_ok_1, qso_dup_1, qso_bad_band, qso_ok_2];
        let input_len = batch.len();

        let report = match fixture
            .ingestor
            .import_batch(batch, &default_options(false))
        {
            Ok(report) => report,
            Err(e) => panic!("expected import_batch to succeed, got error: {e}"),
        };

        assert_eq!(report.inserted, 2);
        assert_eq!(report.rejected_invalid, 2);
        assert_eq!(report.skipped_duplicates, 1);
        assert_eq!(report.first_issue, Some(QsoIssue::EmptyCallsign));
        assert_eq!(
            report.inserted + report.rejected_invalid + report.skipped_duplicates,
            input_len
        );
    }

    #[test]
    fn import_skips_duplicate_already_in_database() {
        let fixture = TestFixture::new();

        let mut qso_existing = sample_valid_qso("SP6INA");
        qso_existing.time_on = "123400".to_string();

        let first_report = match fixture
            .ingestor
            .import_batch(vec![qso_existing], &default_options(false))
        {
            Ok(report) => report,
            Err(e) => panic!("expected initial import_batch to succeed, got error: {e}"),
        };
        assert_eq!(first_report.inserted, 1);
        assert_eq!(fixture.count_qsos(), 1);

        let mut qso_dup_in_db = sample_valid_qso("SP6INA");
        qso_dup_in_db.time_on = "1234".to_string();

        let mut qso_new = sample_valid_qso("K1ABC");
        qso_new.time_on = "130000".to_string();

        let second_report = match fixture
            .ingestor
            .import_batch(vec![qso_dup_in_db, qso_new], &default_options(false))
        {
            Ok(report) => report,
            Err(e) => panic!("expected second import_batch to succeed, got error: {e}"),
        };

        assert_eq!(
            second_report,
            ImportReport {
                inserted: 1,
                rejected_invalid: 0,
                skipped_duplicates: 1,
                first_issue: None,
            }
        );
        assert_eq!(fixture.count_qsos(), 2);
    }
}
