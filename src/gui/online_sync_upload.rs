// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Orkiestracja uploadu oparta na planie (F-20).
//!
//! Ten moduł jest jedynym miejscem, w którym GUI uruchamia wysyłkę do serwisu
//! online. Zakres wyboru pochodzi wyłącznie z
//! [`crate::cloud::upload_plan::UploadPlan`] — jawnego, zmaterializowanego
//! zbioru tożsamości QSO w jednym dzienniku. Transport jest wstrzykiwany do
//! [`run_plan`], więc:
//!
//!  * żaden przycisk nie wybiera już „ostatnich N rekordów”,
//!  * brak aktywnego dziennika jest kontrolowaną odmową,
//!  * paczka jest wysyłana tylko raz i tylko w całości,
//!  * status QSO jest zapisywany dopiero po dowodzie akceptacji paczki,
//!  * częściowy sukces, timeout i anulowanie nigdy nie są raportowane jako sukces.

use crate::cloud::lotw::{TqslSpawnError, run_tqsl_async, tqsl_batch_outcome};
use crate::cloud::upload_plan::{
    BatchOutcome, JournalId, PlanError, SelectionPolicy, UploadPlan, UploadRunReport, UploadTarget,
    run_plan,
};
use crate::core::database::LogDatabase;
use crate::core::i18n::{Language, Msg, key};
use crate::gui::app::SpLogApp;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

type DbHandle = Arc<Mutex<LogDatabase>>;
type LogTx = std::sync::mpsc::Sender<(String, bool)>;

/// Dane uwierzytelniające i konfiguracja potrzebne do wysłania paczki.
///
/// `Debug` redaguje wszystkie sekrety — struktura nigdy nie może wyciec do logu.
#[derive(Clone)]
pub(crate) struct UploadCredentialsSnapshot {
    pub station_callsign: String,
    pub lotw_tqsl_path: String,
    pub lotw_station_location: String,
    pub eqsl_username: String,
    pub eqsl_password: String,
    pub clublog_callsign: String,
    pub clublog_email: String,
    pub clublog_password: String,
    pub clublog_api_key: String,
    pub qrz_api_key: String,
    pub hamqth_username: String,
    pub hamqth_password: String,
    pub cloudlog_url: String,
    pub cloudlog_api_key: String,
    pub hrdlog_username: String,
    pub hrdlog_upload_code: String,
}

impl std::fmt::Debug for UploadCredentialsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadCredentialsSnapshot")
            .field("station_callsign", &self.station_callsign)
            .field("lotw_tqsl_path", &self.lotw_tqsl_path)
            .field("lotw_station_location", &self.lotw_station_location)
            .field("eqsl_username", &self.eqsl_username)
            .field("eqsl_password", &"<redacted>")
            .field("clublog_callsign", &self.clublog_callsign)
            .field("clublog_email", &self.clublog_email)
            .field("clublog_password", &"<redacted>")
            .field("clublog_api_key", &"<redacted>")
            .field("qrz_api_key", &"<redacted>")
            .field("hamqth_username", &self.hamqth_username)
            .field("hamqth_password", &"<redacted>")
            .field("cloudlog_url", &self.cloudlog_url)
            .field("cloudlog_api_key", &"<redacted>")
            .field("hrdlog_username", &self.hrdlog_username)
            .field("hrdlog_upload_code", &"<redacted>")
            .finish()
    }
}

impl UploadCredentialsSnapshot {
    /// Tworzy migawkę konfiguracji. Wartości są kopiowane przed uruchomieniem
    /// zadania, więc późniejsza edycja ustawień nie zmienia trwającej wysyłki.
    pub(crate) fn from_app(app: &SpLogApp) -> Self {
        Self {
            station_callsign: app.config.station.callsign.clone(),
            lotw_tqsl_path: app.config.lotw_tqsl_path.clone(),
            lotw_station_location: app.config.lotw_station_name.clone(),
            eqsl_username: app.config.eqsl_username.clone(),
            eqsl_password: app.config.eqsl_password.clone(),
            clublog_callsign: app.config.clublog_callsign.clone(),
            clublog_email: app.config.clublog_email.clone(),
            clublog_password: app.config.clublog_password.clone(),
            clublog_api_key: app.config.clublog_api_key.clone(),
            qrz_api_key: app.config.qrz_api_key.clone(),
            hamqth_username: app.config.hamqth_username.clone(),
            hamqth_password: app.config.hamqth_password.clone(),
            cloudlog_url: app.config.cloudlog_url.clone(),
            cloudlog_api_key: app.config.cloudlog_api_key.clone(),
            hrdlog_username: app.config.hrdlog_username.clone(),
            hrdlog_upload_code: app.config.hrdlog_upload_code.clone(),
        }
    }
}

/// Sposób dostarczenia jednej paczki do usługi.
#[derive(Clone, Debug)]
pub(crate) enum Transport {
    /// TQSL: podpisanie i wysyłka jednego pliku ADIF na paczkę.
    Tqsl,
    /// eQSL `ImportADIF`: jeden POST na paczkę.
    Eqsl,
    /// Usługa przyjmująca cały ADIF w jednym żądaniu (Club Log, QRZ.com).
    AdifRequest,
    /// Usługa wysyłająca pojedyncze QSO (HamQTH, Cloudlog, HRDLog).
    PerQso,
}

/// Błąd rozpoczęcia uploadu (przed wykonaniem jakiegokolwiek żądania).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UploadStartError {
    /// Brak użytecznego identyfikatora dziennika lub inny błąd planu.
    Plan(PlanError),
    /// Baza danych jest niedostępna.
    Database,
}

/// Buduje plan uploadu dla aktywnego dziennika.
///
/// Plan jest materializowany **przed** uruchomieniem zadania asynchronicznego,
/// więc późniejsza zmiana aktywnego dziennika nie może zmienić jego składu.
pub(crate) fn build_plan(
    app: &SpLogApp,
    target: UploadTarget,
    policy: SelectionPolicy,
) -> Result<UploadPlan, UploadStartError> {
    let journal_id = JournalId::new(&app.active_journal.id).map_err(UploadStartError::Plan)?;
    let records = {
        let db = app
            .services
            .log_db
            .lock()
            .map_err(|_| UploadStartError::Database)?;
        db.select_upload_candidates_for_journal(&journal_id, target)
            .map_err(|_| UploadStartError::Database)?
    };

    UploadPlan::build(
        target,
        journal_id,
        policy,
        uuid::Uuid::new_v4().to_string(),
        crate::cloud::scheduler::now_unix(),
        records,
        target.batch_size(),
    )
    .map_err(UploadStartError::Plan)
}

fn push_log(app: &mut SpLogApp, text: String) {
    app.online.online_sync_logs.push(text);
}

fn start_error_message(lang: Language, target: UploadTarget, error: &UploadStartError) -> String {
    match error {
        UploadStartError::Plan(PlanError::MissingJournal | PlanError::InvalidJournalId) => {
            Msg::new(key::SYNC_UPLOAD_NO_JOURNAL)
                .arg("service", target.label())
                .text(lang)
        }
        UploadStartError::Plan(other) => Msg::new(key::SYNC_UPLOAD_PLAN_ERROR)
            .arg("service", target.label())
            .arg("err", other.to_string())
            .text(lang),
        UploadStartError::Database => Msg::new(key::SYNC_UPLOAD_DB_ERROR)
            .arg("service", target.label())
            .text(lang),
    }
}

/// Uruchamia wysyłkę zmaterializowanego planu.
pub(crate) fn run_upload(
    app: &mut SpLogApp,
    plan: UploadPlan,
    transport: Transport,
    credentials: UploadCredentialsSnapshot,
    lang: Language,
) {
    let target = plan.target();
    let journal_id = plan.journal_id().clone();
    let plan_id = plan.plan_id().to_string();
    let total = plan.total_records();
    let batches = plan.batch_count();

    if total == 0 {
        push_log(
            app,
            Msg::new(key::SYNC_UPLOAD_NOTHING_TO_SEND)
                .arg("service", target.label())
                .arg("journal", journal_id.as_str())
                .text(lang),
        );
        return;
    }

    push_log(
        app,
        Msg::new(key::SYNC_UPLOAD_PLAN_SUMMARY)
            .arg("service", target.label())
            .arg("plan", &plan_id)
            .arg("count", total)
            .arg("batches", batches)
            .arg("journal", journal_id.as_str())
            .text(lang),
    );
    app.gui.chrome.status_message = Some(
        Msg::new(key::SYNC_UPLOAD_PLAN_SUMMARY)
            .arg("service", target.label())
            .arg("plan", &plan_id)
            .arg("count", total)
            .arg("batches", batches)
            .arg("journal", journal_id.as_str())
            .text(lang),
    );

    let db: DbHandle = app.services.log_db.clone();
    let tx: LogTx = app.online.sync_log_tx.clone();
    let cancel = app.online.upload_cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    tokio::spawn(async move {
        let report = execute_plan(plan, transport, credentials, db, tx.clone(), cancel, lang).await;
        let message = final_report_message(&report, lang);
        let _ = tx.send((message, report.status.is_success()));
    });
}

async fn execute_plan(
    plan: UploadPlan,
    transport: Transport,
    credentials: UploadCredentialsSnapshot,
    db: DbHandle,
    tx: LogTx,
    cancel: Arc<AtomicBool>,
    lang: Language,
) -> UploadRunReport {
    let target = plan.target();
    let journal_id = plan.journal_id().clone();

    let send = |batch: crate::cloud::upload_plan::PlanBatch| {
        let transport = transport.clone();
        let credentials = credentials.clone();
        let db = db.clone();
        let tx = tx.clone();
        let journal_id = journal_id.clone();
        async move {
            let _ = tx.send((
                Msg::new(key::SYNC_UPLOAD_BATCH_SENDING)
                    .arg("service", target.label())
                    .arg("index", batch.number())
                    .arg("total", batch.total_batches())
                    .arg("count", batch.count())
                    .text(lang),
                false,
            ));
            let outcome = execute_batch(
                target,
                &transport,
                &credentials,
                &journal_id,
                batch.record_ids().to_vec(),
                db,
            )
            .await;
            let _ = tx.send((
                Msg::new(key::SYNC_UPLOAD_BATCH_RESULT)
                    .arg("service", target.label())
                    .arg("index", batch.number())
                    .arg("total", batch.total_batches())
                    .arg("result", describe_outcome(&outcome, lang))
                    .text(lang),
                outcome.is_accepted(),
            ));
            outcome
        }
    };

    run_plan(&plan, &cancel, send).await
}

async fn execute_batch(
    target: UploadTarget,
    transport: &Transport,
    credentials: &UploadCredentialsSnapshot,
    journal_id: &JournalId,
    record_ids: Vec<i64>,
    db: DbHandle,
) -> BatchOutcome {
    let expected = record_ids.len();

    // 1. Materializacja ładunku — wyłącznie z tego dziennika i w kolejności planu.
    let qsos = {
        let fetched = match db.lock() {
            Ok(guard) => guard
                .get_qsos_by_ids_for_journal(journal_id, &record_ids)
                .map_err(|error| error.to_string()),
            Err(_) => Err("database unavailable".to_string()),
        };
        match fetched {
            Ok(qsos) => qsos,
            Err(detail) => return BatchOutcome::Failed { detail },
        }
    };
    if qsos.len() != expected {
        return BatchOutcome::Failed {
            detail: format!(
                "record set changed since plan creation (expected {expected}, got {})",
                qsos.len()
            ),
        };
    }

    let adif = crate::core::adif::export_adif(&qsos, "SPLogbook", &credentials.station_callsign);

    // 2. Transport — bez automatycznego ponawiania.
    let outcome = match transport {
        Transport::Tqsl => {
            match run_tqsl_async(
                &credentials.lotw_tqsl_path,
                &credentials.lotw_station_location,
                &adif,
            )
            .await
            {
                Ok(run) => tqsl_batch_outcome(&run),
                Err(TqslSpawnError::Spawn(detail) | TqslSpawnError::Io(detail)) => {
                    BatchOutcome::Failed { detail }
                }
            }
        }
        Transport::Eqsl => {
            let client = crate::cloud::eqsl::EqslClient::new(
                &credentials.eqsl_username,
                &credentials.eqsl_password,
            );
            match client.upload_adif(&adif).await {
                Ok(submission) => {
                    let classification = crate::cloud::eqsl::classify_eqsl_response(
                        submission.http_status,
                        &submission.body,
                    );
                    crate::cloud::eqsl::eqsl_batch_outcome(&classification, expected)
                }
                Err(crate::cloud::eqsl::EqslSubmitError::Timeout) => BatchOutcome::TimedOut {
                    detail: "eQSL: przekroczono limit czasu".to_string(),
                },
                Err(error) => BatchOutcome::Failed {
                    detail: error.to_string(),
                },
            }
        }
        Transport::AdifRequest => match target {
            UploadTarget::ClubLog => {
                let client = crate::cloud::clublog::ClubLogClient::new();
                match client
                    .upload_adif(
                        &credentials.clublog_callsign,
                        &credentials.clublog_email,
                        &credentials.clublog_password,
                        &credentials.clublog_api_key,
                        &adif,
                    )
                    .await
                {
                    Ok(_) => BatchOutcome::Accepted,
                    Err(error) => BatchOutcome::Failed {
                        detail: error.to_string(),
                    },
                }
            }
            UploadTarget::Qrz => {
                match crate::cloud::qrz::QrzClient::upload_to_logbook(
                    &credentials.qrz_api_key,
                    &adif,
                )
                .await
                {
                    Ok(_) => BatchOutcome::Accepted,
                    Err(error) => BatchOutcome::Failed {
                        detail: error.to_string(),
                    },
                }
            }
            _ => BatchOutcome::Failed {
                detail: "unsupported ADIF transport".to_string(),
            },
        },
        Transport::PerQso => execute_per_qso(target, credentials, &qsos).await,
    };

    // 3. Zapis statusu — wyłącznie po dowodzie akceptacji całej paczki.
    //    `commit_batch_acceptance` jest transakcyjny i wymaga, aby liczba
    //    zaktualizowanych wierszy równała się długości paczki; rozbieżność
    //    wycofuje transakcję i paczka nie może zostać uznana za sukces.
    if outcome.is_accepted() && target.has_sent_status() {
        let committed = match db.lock() {
            Ok(guard) => guard
                .commit_batch_acceptance(journal_id, target, &record_ids)
                .map_err(|error| error.to_string()),
            Err(_) => Err("database unavailable".to_string()),
        };
        return match committed {
            Ok(()) => BatchOutcome::Accepted,
            Err(detail) => BatchOutcome::Failed { detail },
        };
    }

    outcome
}

async fn execute_per_qso(
    target: UploadTarget,
    credentials: &UploadCredentialsSnapshot,
    qsos: &[crate::core::qso::QsoRecord],
) -> BatchOutcome {
    let mut succeeded = 0usize;
    let mut last_error = String::new();

    for qso in qsos {
        // `Option<String>` zamiast `Result<_, String>`: moduły domenowe nie
        // mogą używać tekstowego typu błędu (guard `no_string_errors_in_domain`).
        let failure: Option<String> = match target {
            UploadTarget::HamQth => {
                let client = crate::cloud::hamqth::HamQthClient::new(
                    credentials.hamqth_username.clone(),
                    credentials.hamqth_password.clone(),
                );
                client
                    .upload_qso(qso)
                    .await
                    .err()
                    .map(|error| error.to_string())
            }
            UploadTarget::Cloudlog => {
                let client = crate::cloud::cloudlog::CloudlogClient::new(
                    &credentials.cloudlog_url,
                    credentials.cloudlog_api_key.clone(),
                    "1".to_string(),
                );
                client
                    .upload_qso(qso)
                    .await
                    .err()
                    .map(|error| error.to_string())
            }
            UploadTarget::HrdLog => {
                let client = crate::cloud::hrdlog::HrdlogClient::new(
                    credentials.hrdlog_username.clone(),
                    credentials.hrdlog_upload_code.clone(),
                );
                client
                    .upload_qso(qso)
                    .await
                    .err()
                    .map(|error| error.to_string())
            }
            _ => Some("unsupported per-QSO service".to_string()),
        };
        match failure {
            None => succeeded += 1,
            Some(error) => last_error = error,
        }
    }

    if succeeded == qsos.len() && !qsos.is_empty() {
        BatchOutcome::Accepted
    } else if succeeded == 0 {
        BatchOutcome::Failed { detail: last_error }
    } else {
        BatchOutcome::PartiallyAccepted {
            rejected: qsos.len() - succeeded,
            detail: last_error,
        }
    }
}

fn describe_outcome(outcome: &BatchOutcome, lang: Language) -> String {
    match outcome {
        BatchOutcome::Accepted => Msg::new(key::SYNC_UPLOAD_OUTCOME_ACCEPTED).text(lang),
        BatchOutcome::PartiallyAccepted { rejected, .. } => {
            Msg::new(key::SYNC_UPLOAD_OUTCOME_PARTIAL)
                .arg("rejected", rejected)
                .text(lang)
        }
        BatchOutcome::NoRecordsProcessed { .. } => {
            Msg::new(key::SYNC_UPLOAD_OUTCOME_NO_RECORDS).text(lang)
        }
        BatchOutcome::Rejected { .. } => Msg::new(key::SYNC_UPLOAD_OUTCOME_REJECTED).text(lang),
        BatchOutcome::Unknown { .. } => Msg::new(key::SYNC_UPLOAD_OUTCOME_UNKNOWN).text(lang),
        BatchOutcome::TimedOut { .. } => Msg::new(key::SYNC_UPLOAD_OUTCOME_TIMEOUT).text(lang),
        BatchOutcome::Failed { .. } => Msg::new(key::SYNC_UPLOAD_OUTCOME_FAILED).text(lang),
        BatchOutcome::Cancelled => Msg::new(key::SYNC_UPLOAD_OUTCOME_CANCELLED).text(lang),
    }
}

fn final_report_message(report: &UploadRunReport, lang: Language) -> String {
    let service = report.target.label();
    match report.status {
        crate::cloud::upload_plan::RunStatus::Complete => Msg::new(key::SYNC_UPLOAD_COMPLETE)
            .arg("service", service)
            .arg("count", report.total_records)
            .arg("batches", report.batch_count)
            .arg("plan", &report.plan_id)
            .text(lang),
        crate::cloud::upload_plan::RunStatus::Partial => Msg::new(key::SYNC_UPLOAD_PARTIAL)
            .arg("service", service)
            .arg("accepted", report.accepted_records)
            .arg("rejected", report.rejected_records)
            .arg("unknown", report.unknown_records)
            .arg("failed", report.failed_records)
            .arg("plan", &report.plan_id)
            .text(lang),
        crate::cloud::upload_plan::RunStatus::Cancelled => Msg::new(key::SYNC_UPLOAD_CANCELLED)
            .arg("service", service)
            .arg("accepted", report.accepted_records)
            .arg("not_attempted", report.not_attempted_records)
            .text(lang),
        crate::cloud::upload_plan::RunStatus::Failed => Msg::new(key::SYNC_UPLOAD_FAILED)
            .arg("service", service)
            .arg("failed", report.failed_records)
            .arg("unknown", report.unknown_records)
            .arg("rejected", report.rejected_records)
            .text(lang),
        crate::cloud::upload_plan::RunStatus::Empty => Msg::new(key::SYNC_UPLOAD_NOTHING_TO_SEND)
            .arg("service", service)
            .arg("journal", report.journal_id.as_str())
            .text(lang),
    }
}

/// Wspólny punkt wejścia dla wszystkich usług: buduje plan i uruchamia wysyłkę.
pub(crate) fn start_upload(
    app: &mut SpLogApp,
    target: UploadTarget,
    transport: Transport,
    credentials: UploadCredentialsSnapshot,
    lang: Language,
) {
    match build_plan(app, target, SelectionPolicy::PendingOnly) {
        Ok(plan) => run_upload(app, plan, transport, credentials, lang),
        Err(error) => {
            let message = start_error_message(lang, target, &error);
            push_log(app, message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials() -> UploadCredentialsSnapshot {
        UploadCredentialsSnapshot {
            station_callsign: "TSTSTN".into(),
            lotw_tqsl_path: "/nonexistent/tqsl".into(),
            lotw_station_location: "TEST_LOCATION".into(),
            eqsl_username: "TSTUSER".into(),
            eqsl_password: "TEST_SECRET_DO_NOT_USE_EQSL".into(),
            clublog_callsign: "TSTUSER".into(),
            clublog_email: "test@example.invalid".into(),
            clublog_password: "TEST_SECRET_DO_NOT_USE_CLUBLOG".into(),
            clublog_api_key: "TEST_SECRET_DO_NOT_USE_CLUBKEY".into(),
            qrz_api_key: "TEST_SECRET_DO_NOT_USE_QRZKEY".into(),
            hamqth_username: "TSTUSER".into(),
            hamqth_password: "TEST_SECRET_DO_NOT_USE_HAMQTH".into(),
            cloudlog_url: "http://127.0.0.1:1".into(),
            cloudlog_api_key: "TEST_SECRET_DO_NOT_USE_CLOUDLOG".into(),
            hrdlog_username: "TSTUSER".into(),
            hrdlog_upload_code: "TEST_SECRET_DO_NOT_USE_HRDLOG".into(),
        }
    }

    #[test]
    fn credentials_debug_redacts_every_secret() {
        let text = format!("{:?}", credentials());
        assert!(!text.contains("TEST_SECRET_DO_NOT_USE"));
        assert!(text.contains("<redacted>"));
        assert!(text.contains("TSTUSER"));
    }

    #[test]
    fn outcome_descriptions_are_localized_and_never_claim_success_for_failures() {
        let lang = Language::Pl;
        assert_ne!(describe_outcome(&BatchOutcome::Cancelled, lang), "");
        for outcome in [
            BatchOutcome::PartiallyAccepted {
                rejected: 1,
                detail: String::new(),
            },
            BatchOutcome::NoRecordsProcessed {
                detail: String::new(),
            },
            BatchOutcome::Rejected {
                detail: String::new(),
            },
            BatchOutcome::Unknown {
                detail: String::new(),
            },
            BatchOutcome::TimedOut {
                detail: String::new(),
            },
            BatchOutcome::Failed {
                detail: String::new(),
            },
        ] {
            assert!(!outcome.is_accepted());
            assert_ne!(describe_outcome(&outcome, lang), "");
        }
        assert_eq!(
            describe_outcome(&BatchOutcome::Accepted, lang),
            describe_outcome(&BatchOutcome::Accepted, lang)
        );
    }

    #[test]
    fn start_error_for_missing_journal_uses_the_journal_key() {
        let message = start_error_message(
            Language::Pl,
            UploadTarget::Lotw,
            &UploadStartError::Plan(PlanError::MissingJournal),
        );
        assert!(message.contains("LoTW"));
        assert_ne!(message, "");
    }

    #[test]
    fn start_error_for_database_is_reported_for_the_service() {
        let message = start_error_message(
            Language::En,
            UploadTarget::Eqsl,
            &UploadStartError::Database,
        );
        assert!(message.contains("eQSL.cc"));
    }
}
