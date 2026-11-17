// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Selektory i aktualizacje statusu dla planu uploadu (F-20).
//!
//! Wszystkie zapytania tutaj są **jawne dziennikowo**: każde ma predykat
//! `journal_id = ?` i żadne nie używa `LIMIT` jako sposobu wyboru zbioru do
//! wysłania. To właśnie usunięcie globalnego selektora i obcinania jest celem
//! F-20.

use super::LogDatabase;
use super::schema::{ORDER_CHRONO_ASC, QSO_SQL, collect_qsos};
use crate::cloud::upload_plan::{JournalId, PlanRecord, UploadRunReport, UploadTarget};
use crate::core::qso::QsoRecord;
use rusqlite::{Result, params};

/// Kolumna statusu wysyłki dla usług, które ją posiadają.
///
/// Zwraca wyłącznie stałe identyfikatory kolumn — nigdy tekst z zewnątrz.
fn sent_column(target: UploadTarget) -> Option<&'static str> {
    match target {
        UploadTarget::Lotw => Some("lotw_qsl_sent"),
        UploadTarget::Eqsl => Some("eqsl_qsl_sent"),
        _ => None,
    }
}

/// Błąd zapisu dowodu akceptacji paczki.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchCommitError {
    /// Liczba zaktualizowanych wierszy nie odpowiada długości paczki —
    /// transakcja została wycofana, a status NIE został zapisany.
    RowCountMismatch { expected: usize, affected: usize },
    /// Błąd bazy danych (transakcja wycofana).
    Database(String),
}

impl std::fmt::Display for BatchCommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RowCountMismatch { expected, affected } => write!(
                f,
                "zapis statusu objął {affected} z {expected} rekordów — wycofano transakcję"
            ),
            Self::Database(detail) => write!(f, "błąd bazy danych: {detail}"),
        }
    }
}

impl std::error::Error for BatchCommitError {}

impl LogDatabase {
    /// Wybiera **wszystkie** rekordy kwalifikujące się do uploadu w jednym
    /// dzienniku, w kanonicznej kolejności rosnącej.
    ///
    /// * Filtr dziennika jest obowiązkowy — brak `journal_id` jest błędem
    ///   walidacji [`JournalId`], nie „wyborem globalnym”.
    /// * Dla LoTW/eQSL obowiązuje predykat `NOT IN ('Y','I')` zgodny ze
    ///   znaczeniem enumeracji ADIF `QSL Sent` (`Y` = „uploaded to, and accepted
    ///   by, the online service”, `I` = „ignore or invalid”).
    /// * Brak `LIMIT` — kompletny zbiór jest materializowany, a podział na
    ///   paczki wykonuje [`crate::cloud::upload_plan::UploadPlan`].
    pub fn select_upload_candidates_for_journal(
        &self,
        journal_id: &JournalId,
        target: UploadTarget,
    ) -> Result<Vec<PlanRecord>> {
        let sql = match sent_column(target) {
            Some(column) => format!(
                "SELECT id, qso_date, time_on FROM qso_records \
                 WHERE journal_id = ?1 AND COALESCE({column}, 'N') NOT IN ('Y', 'I') \
                 ORDER BY {ORDER_CHRONO_ASC}"
            ),
            None => format!(
                "SELECT id, qso_date, time_on FROM qso_records \
                 WHERE journal_id = ?1 ORDER BY {ORDER_CHRONO_ASC}"
            ),
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![journal_id.as_str()], |row| {
            Ok(PlanRecord::new(
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        rows.collect()
    }

    /// Pobiera rekordy QSO o podanych identyfikatorach **wyłącznie** z podanego
    /// dziennika, w kanonicznej kolejności rosnącej.
    ///
    /// Służy do zbudowania ładunku konkretnej paczki: jeśli rekord zniknął lub
    /// zmienił dziennik, liczba zwróconych wierszy będzie mniejsza niż liczba
    /// identyfikatorów i paczka nie zostanie wysłana jako kompletna.
    pub fn get_qsos_by_ids_for_journal(
        &self,
        journal_id: &JournalId,
        ids: &[i64],
    ) -> Result<Vec<QsoRecord>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = (0..ids.len())
            .map(|index| format!("?{}", index + 2))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {} FROM qso_records WHERE journal_id = ?1 AND id IN ({placeholders}) \
             ORDER BY {ORDER_CHRONO_ASC}",
            QSO_SQL.select_columns
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let mut bound: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 1);
        let journal = journal_id.as_str();
        bound.push(&journal);
        for id in ids {
            bound.push(id);
        }

        collect_qsos(&mut stmt, bound.as_slice())
    }

    /// Zapisuje dowód akceptacji paczki — jedyna droga do ustawienia
    /// `<usługa>_qsl_sent = 'Y'`.
    ///
    /// Niezmiennik: status jest zapisywany **tylko** wtedy, gdy udało się
    /// zaktualizować dokładnie tyle wierszy, ile jest rekordów w paczce i
    /// wszystkie należą do podanego dziennika. W przeciwnym razie transakcja
    /// jest wycofywana i zwracany jest [`BatchCommitError`] — „sukces” nigdy nie
    /// pozostawia częściowo zapisanego stanu.
    pub fn commit_batch_acceptance(
        &self,
        journal_id: &JournalId,
        target: UploadTarget,
        ids: &[i64],
    ) -> std::result::Result<(), BatchCommitError> {
        let Some(column) = sent_column(target) else {
            // Usługi bez kolumny statusu nie mają czego zapisać.
            return Ok(());
        };
        if ids.is_empty() {
            return Ok(());
        }

        let placeholders = (0..ids.len())
            .map(|index| format!("?{}", index + 2))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "UPDATE qso_records SET {column} = 'Y' \
             WHERE journal_id = ?1 AND id IN ({placeholders})"
        );

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|error| BatchCommitError::Database(error.to_string()))?;

        let affected = {
            let mut stmt = tx
                .prepare(&sql)
                .map_err(|error| BatchCommitError::Database(error.to_string()))?;
            let mut bound: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 1);
            let journal = journal_id.as_str();
            bound.push(&journal);
            for id in ids {
                bound.push(id);
            }
            stmt.execute(bound.as_slice())
                .map_err(|error| BatchCommitError::Database(error.to_string()))?
        };

        if affected != ids.len() {
            // `tx` zostanie wycofana przy drop, bez commit.
            return Err(BatchCommitError::RowCountMismatch {
                expected: ids.len(),
                affected,
            });
        }

        tx.commit()
            .map_err(|error| BatchCommitError::Database(error.to_string()))?;
        Ok(())
    }

    /// Zapisuje dowód akceptacji dla wszystkich zaakceptowanych paczek raportu.
    ///
    /// Paczki odrzucone, częściowe, niejednoznaczne, przekroczone czasowo i
    /// anulowane nie zmieniają żadnego statusu. Zwraca liczbę rekordów
    /// oznaczonych jako wysłane.
    pub fn apply_accepted_batches(
        &self,
        journal_id: &JournalId,
        target: UploadTarget,
        report: &UploadRunReport,
    ) -> std::result::Result<usize, BatchCommitError> {
        let mut marked = 0usize;
        for batch in &report.batches {
            if !batch.outcome.is_accepted() {
                continue;
            }
            self.commit_batch_acceptance(journal_id, target, &batch.record_ids)?;
            marked += batch.record_ids.len();
        }
        Ok(marked)
    }

    /// Oznacza potwierdzoną akceptację paczki: ustawia `<usługa>_qsl_sent = 'Y'`
    /// dla podanych identyfikatorów w podanym dzienniku, w jednej transakcji.
    ///
    /// Zwraca liczbę zaktualizowanych wierszy. Wywołujący **musi** porównać ją
    /// z długością paczki — rozbieżność oznacza, że zbiór rekordów zmienił się
    /// między wyborem a aktualizacją i paczka nie może zostać uznana za sukces.
    ///
    /// Transakcja obejmuje wyłącznie zapis do bazy; nigdy nie jest trzymana
    /// podczas wywołania sieciowego ani procesu TQSL.
    pub fn mark_upload_sent_scoped(
        &self,
        journal_id: &JournalId,
        target: UploadTarget,
        ids: &[i64],
    ) -> Result<usize> {
        let Some(column) = sent_column(target) else {
            // Usługi bez kolumny statusu nie mają czego zapisać.
            return Ok(0);
        };
        if ids.is_empty() {
            return Ok(0);
        }

        let placeholders = (0..ids.len())
            .map(|index| format!("?{}", index + 2))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "UPDATE qso_records SET {column} = 'Y' \
             WHERE journal_id = ?1 AND id IN ({placeholders})"
        );

        let tx = self.conn.unchecked_transaction()?;
        let affected = {
            let mut stmt = tx.prepare(&sql)?;
            let mut bound: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() + 1);
            let journal = journal_id.as_str();
            bound.push(&journal);
            for id in ids {
                bound.push(id);
            }
            stmt.execute(bound.as_slice())?
        };
        tx.commit()?;
        Ok(affected)
    }

    /// Liczba rekordów kwalifikujących się do uploadu w dzienniku (bez
    /// materializacji) — używana wyłącznie do diagnostyki i testów.
    pub fn count_upload_candidates_for_journal(
        &self,
        journal_id: &JournalId,
        target: UploadTarget,
    ) -> Result<usize> {
        let count: i64 = match sent_column(target) {
            Some(column) => self.conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM qso_records \
                     WHERE journal_id = ?1 AND COALESCE({column}, 'N') NOT IN ('Y', 'I')"
                ),
                [journal_id.as_str()],
                |row| row.get(0),
            )?,
            None => self.conn.query_row(
                "SELECT COUNT(*) FROM qso_records WHERE journal_id = ?1",
                [journal_id.as_str()],
                |row| row.get(0),
            )?,
        };
        Ok(count as usize)
    }
}
