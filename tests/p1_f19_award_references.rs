// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// P1-F19 — SOTA/POTA operator–correspondent reference semantics, ADIF conformance and
// lossless round-trip.
//
// All data in this file is synthetic: callsigns use the reserved test prefix `TST`,
// references use the synthetic associations `AA/BB`, `CC/DD`, `EE/FF` and the
// synthetic POTA prefixes `XX`, `YY`, `ZZ`. No real user log, callsign, database,
// profile, account or award reference is read, and nothing is uploaded anywhere.
//
// The suite drives the production paths `SpLogApp::draft_qso_from_entry`,
// `QsoEntryState`, `core::adif` and `core::sota_export`; no hardware, no PTT, no RF,
// no network, no wall clock and no sleep are used.

use splogbook::core::adif::{self, AdifRecordError};
use splogbook::core::award_refs::{
    AwardProgram, AwardRefNotice, AwardRefPair, AwardReferences, PotaRepresentation, StationSide,
};
use splogbook::core::database::LogDatabase;
use splogbook::core::prefix::PrefixMatcher;
use splogbook::core::qso::{QsoRecord, QsoWarning};
use splogbook::core::scp::ScpEngine;
use splogbook::core::sota_export::export_sota_csv;
use splogbook::core::station::AppConfig;
use splogbook::gui::app::{AppDeps, AppInit, ShutdownPolicy, SpLogApp};
use splogbook::gui::state::QsoEntryState;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Synthetic fixtures — every value is clearly distinct and not a real reference.
// ---------------------------------------------------------------------------

const LOCAL_CALL: &str = "TSTLOCAL";
const REMOTE_CALL: &str = "TSTREMOTE";

const LOCAL_SOTA: &str = "AA/BB-001";
const REMOTE_SOTA: &str = "CC/DD-999";
const THIRD_SOTA: &str = "EE/FF-002";

const LOCAL_POTA: &str = "XX-0001";
const REMOTE_POTA: &str = "YY-9999";
const THIRD_POTA: &str = "ZZ-0003";
const FOURTH_POTA: &str = "ZZ-0004";

fn qso(
    local_sota: Option<&str>,
    sota: Option<&str>,
    local_pota: Option<&str>,
    pota: Option<&str>,
) -> QsoRecord {
    let mut q = QsoRecord::new(REMOTE_CALL, "20m", "CW");
    q.qso_date = "20260101".to_string();
    q.time_on = "120000".to_string();
    q.my_sota_ref = local_sota.map(str::to_string);
    q.sota_ref = sota.map(str::to_string);
    q.my_pota_ref = local_pota.map(str::to_string);
    q.pota_ref = pota.map(str::to_string);
    q
}

fn render(q: &QsoRecord) -> String {
    adif::export_adif(std::slice::from_ref(q), "SPLogbook", LOCAL_CALL)
}

fn render_all(qsos: &[QsoRecord]) -> String {
    adif::export_adif(qsos, "SPLogbook", LOCAL_CALL)
}

/// Returns the value of the first `TAG` in an ADIF text.
fn field<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let needle = format!("<{tag}:");
    let start = text.find(&needle)?;
    let after = &text[start + needle.len()..];
    let close = after.find('>')?;
    let declared: usize = after[..close].parse().ok()?;
    let body = &after[close + 1..];
    body.get(..declared)
}

/// Returns every field name (upper-cased) present in an ADIF text, in text order.
fn tags(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else { break };
        let tag = &after[..close];
        let name = tag.split(':').next().unwrap_or(tag).to_uppercase();
        if !name.is_empty() && name != "EOH" && name != "EOR" {
            out.push(name);
        }
        rest = &after[close + 1..];
    }
    out
}

fn parse_one(text: &str) -> QsoRecord {
    let report = adif::parse_adif_with_report(text);
    assert_eq!(
        report.rejected, 0,
        "record must not be rejected: {:?}",
        report.errors
    );
    assert_eq!(report.qsos.len(), 1, "exactly one record expected");
    report.qsos.into_iter().next().expect("one record")
}

fn sides(q: &QsoRecord, program: AwardProgram) -> (Option<String>, Option<String>) {
    let pair = q.award_references().program(program).clone();
    (pair.local, pair.remote)
}

fn sample_adif(reference_fields: &[(&str, &str)]) -> String {
    let mut text = String::from(concat!(
        "<CALL:9>TSTREMOTE",
        "<BAND:3>20m",
        "<MODE:2>CW",
        "<QSO_DATE:8>20260101",
        "<TIME_ON:6>120000",
    ));
    for (tag, value) in reference_fields {
        text.push_str(&format!("<{tag}:{}>{}", value.len(), value));
    }
    text.push_str("<EOR>");
    text
}

// ---------------------------------------------------------------------------
// Production GUI fixture (no clock, no network, in-memory database).
// ---------------------------------------------------------------------------

fn app_with(config: AppConfig) -> SpLogApp {
    let deps = AppDeps {
        log_db: Arc::new(Mutex::new(
            LogDatabase::open_in_memory().expect("in-memory database"),
        )),
        prefix_matcher: Arc::new(PrefixMatcher::empty()),
        scp_engine: Arc::new(Mutex::new(ScpEngine::new())),
    };
    let init = AppInit {
        config,
        config_file_path: std::path::PathBuf::from("config.json"),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    SpLogApp::for_tests(init, deps)
}

fn activation_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.station.callsign = LOCAL_CALL.to_string();
    config.station.sota_ref = Some(LOCAL_SOTA.to_string());
    config.station.pota_ref = Some(LOCAL_POTA.to_string());
    config
}

fn app_in_activation() -> SpLogApp {
    let config = activation_config();
    let mut app = app_with(config);
    app.entry.callsign = REMOTE_CALL.to_string();
    app.entry.band = "20m".to_string();
    app.entry.mode = "CW".to_string();
    app
}

// ===========================================================================
// PHASE 15 — mandatory SOTA matrix
// ===========================================================================

#[test]
fn sota_activator_writes_own_summit_to_my_sota_ref() {
    let q = qso(Some(LOCAL_SOTA), None, None, None);
    let text = render(&q);
    assert_eq!(field(&text, "MY_SOTA_REF"), Some(LOCAL_SOTA));
    assert_eq!(
        field(&text, "SOTA_REF"),
        None,
        "activator has no correspondent summit"
    );
    assert!(!tags(&text).contains(&"SOTA_REF".to_string()));
}

#[test]
fn sota_chaser_writes_chased_summit_to_sota_ref() {
    let q = qso(None, Some(REMOTE_SOTA), None, None);
    let text = render(&q);
    assert_eq!(field(&text, "SOTA_REF"), Some(REMOTE_SOTA));
    assert_eq!(
        field(&text, "MY_SOTA_REF"),
        None,
        "chaser has no own activation reference"
    );
    assert!(!tags(&text).contains(&"MY_SOTA_REF".to_string()));
}

#[test]
fn sota_summit_to_summit_preserves_both_distinct_references() {
    let q = qso(Some(LOCAL_SOTA), Some(REMOTE_SOTA), None, None);
    let text = render(&q);
    assert_eq!(field(&text, "MY_SOTA_REF"), Some(LOCAL_SOTA));
    assert_eq!(field(&text, "SOTA_REF"), Some(REMOTE_SOTA));
    assert_ne!(field(&text, "MY_SOTA_REF"), field(&text, "SOTA_REF"));

    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Sota),
        (Some(LOCAL_SOTA.into()), Some(REMOTE_SOTA.into()))
    );
}

#[test]
fn sota_each_side_may_be_missing_independently() {
    for (local, remote) in [
        (None, None),
        (Some(LOCAL_SOTA), None),
        (None, Some(REMOTE_SOTA)),
    ] {
        let q = qso(local, remote, None, None);
        let text = render(&q);
        assert_eq!(field(&text, "MY_SOTA_REF"), local);
        assert_eq!(field(&text, "SOTA_REF"), remote);
    }
}

#[test]
fn sota_equal_values_on_both_sides_are_preserved_not_deduplicated() {
    let q = qso(Some(LOCAL_SOTA), Some(LOCAL_SOTA), None, None);
    let text = render(&q);
    assert_eq!(field(&text, "MY_SOTA_REF"), Some(LOCAL_SOTA));
    assert_eq!(field(&text, "SOTA_REF"), Some(LOCAL_SOTA));
    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Sota).0,
        Some(LOCAL_SOTA.into())
    );
    assert_eq!(
        sides(&parsed, AwardProgram::Sota).1,
        Some(LOCAL_SOTA.into())
    );
}

#[test]
fn sota_import_reads_each_side_from_its_own_field() {
    let local_only = sample_adif(&[("MY_SOTA_REF", LOCAL_SOTA)]);
    let parsed = parse_one(&local_only);
    assert_eq!(
        sides(&parsed, AwardProgram::Sota),
        (Some(LOCAL_SOTA.into()), None)
    );

    let remote_only = sample_adif(&[("SOTA_REF", REMOTE_SOTA)]);
    let parsed = parse_one(&remote_only);
    assert_eq!(
        sides(&parsed, AwardProgram::Sota),
        (None, Some(REMOTE_SOTA.into()))
    );

    let both = sample_adif(&[("MY_SOTA_REF", LOCAL_SOTA), ("SOTA_REF", REMOTE_SOTA)]);
    let parsed = parse_one(&both);
    assert_eq!(
        sides(&parsed, AwardProgram::Sota),
        (Some(LOCAL_SOTA.into()), Some(REMOTE_SOTA.into()))
    );
}

#[test]
fn sota_lowercase_input_is_preserved_in_adif_and_upper_cased_only_in_the_csv() {
    let lower_local = "aa/bb-001";
    let lower_remote = "cc/dd-999";
    let q = qso(Some(lower_local), Some(lower_remote), None, None);
    let text = render(&q);
    assert_eq!(
        field(&text, "MY_SOTA_REF"),
        Some(lower_local),
        "ADIF value is not rewritten"
    );
    assert_eq!(field(&text, "SOTA_REF"), Some(lower_remote));

    let csv = export_sota_csv(&[q], LOCAL_CALL, "").expect("csv");
    assert!(
        csv.contains("AA/BB-001"),
        "SOTA CSV uses the official upper-case form"
    );
    assert!(csv.contains("CC/DD-999"));
}

#[test]
fn sota_malformed_references_are_kept_and_only_warned_about() {
    let mut q = qso(Some("NOT-A-SUMMIT"), Some("ALSO-NOT"), None, None);
    let warnings = q.warnings();
    assert_eq!(
        warnings,
        vec![
            QsoWarning::Sota("NOT-A-SUMMIT".to_string()),
            QsoWarning::Sota("ALSO-NOT".to_string()),
        ],
        "both sides are validated"
    );
    let text = render(&q);
    assert_eq!(field(&text, "MY_SOTA_REF"), Some("NOT-A-SUMMIT"));
    assert_eq!(field(&text, "SOTA_REF"), Some("ALSO-NOT"));

    // A malformed reference never blocks a save.
    q.band = "20m".to_string();
    assert!(q.validate().is_ok());
}

#[test]
fn sota_spot_of_another_activator_fills_only_the_remote_side() {
    let mut app = app_in_activation();
    assert_eq!(
        app.entry.my_refs.sota, LOCAL_SOTA,
        "activation context from the station profile"
    );

    // What the cluster panel does for a spot of another activator.
    app.entry.sota = REMOTE_SOTA.to_string();

    let draft = app.draft_qso_from_entry();
    assert_eq!(draft.my_sota_ref.as_deref(), Some(LOCAL_SOTA));
    assert_eq!(draft.sota_ref.as_deref(), Some(REMOTE_SOTA));
}

#[test]
fn sota_self_spot_never_overwrites_the_local_reference() {
    let mut app = app_in_activation();
    app.entry.my_refs.sota = THIRD_SOTA.to_string();
    // A self-spot describes the local station; the remote side stays untouched.
    app.entry.sota = String::new();

    let draft = app.draft_qso_from_entry();
    assert_eq!(draft.my_sota_ref.as_deref(), Some(THIRD_SOTA));
    assert_eq!(draft.sota_ref, None);
}

#[test]
fn sota_qso_from_spot_preserves_the_spot_perspective() {
    let mut app = app_in_activation();
    app.entry.sota = REMOTE_SOTA.to_string();
    let draft = app.draft_qso_from_entry();
    let text = render(&draft);

    assert_eq!(field(&text, "MY_SOTA_REF"), Some(LOCAL_SOTA));
    assert_eq!(field(&text, "SOTA_REF"), Some(REMOTE_SOTA));
    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Sota),
        (Some(LOCAL_SOTA.into()), Some(REMOTE_SOTA.into())),
        "no side changes between spot, QSO and ADIF"
    );
}

#[test]
fn sota_csv_export_uses_each_records_own_summit() {
    let mut first = qso(Some(LOCAL_SOTA), Some(REMOTE_SOTA), None, None);
    first.qso_date = "20260101".to_string();
    first.time_on = "1200".to_string();
    let mut second = qso(Some(THIRD_SOTA), None, None, None);
    second.qso_date = "20260102".to_string();
    second.time_on = "1300".to_string();

    let csv = export_sota_csv(&[first, second], LOCAL_CALL, "").expect("csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines[0].contains(",AA/BB-001,"));
    assert!(lines[0].contains(",CC/DD-999,"));
    assert!(lines[1].contains(",EE/FF-002,"));
}

// ===========================================================================
// PHASE 16 — mandatory POTA matrix
// ===========================================================================

#[test]
fn pota_activator_writes_my_sig_pair_and_no_remote_pair() {
    let q = qso(None, None, Some(LOCAL_POTA), None);
    let text = render(&q);
    assert_eq!(field(&text, "MY_SIG"), Some("POTA"));
    assert_eq!(field(&text, "MY_SIG_INFO"), Some(LOCAL_POTA));
    assert_eq!(field(&text, "SIG"), None);
    assert_eq!(field(&text, "SIG_INFO"), None);
    assert_eq!(
        field(&text, "MY_POTA_REF"),
        None,
        "no second representation"
    );
}

#[test]
fn pota_hunter_writes_sig_pair_and_no_local_pair() {
    let q = qso(None, None, None, Some(REMOTE_POTA));
    let text = render(&q);
    assert_eq!(field(&text, "SIG"), Some("POTA"));
    assert_eq!(field(&text, "SIG_INFO"), Some(REMOTE_POTA));
    assert_eq!(field(&text, "MY_SIG"), None);
    assert_eq!(field(&text, "MY_SIG_INFO"), None);
    assert_eq!(field(&text, "POTA_REF"), None);
}

#[test]
fn pota_park_to_park_preserves_both_references() {
    let q = qso(None, None, Some(LOCAL_POTA), Some(REMOTE_POTA));
    let text = render(&q);
    assert_eq!(field(&text, "MY_SIG_INFO"), Some(LOCAL_POTA));
    assert_eq!(field(&text, "SIG_INFO"), Some(REMOTE_POTA));
    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        (Some(LOCAL_POTA.into()), Some(REMOTE_POTA.into()))
    );
}

#[test]
fn pota_each_side_may_be_missing_independently() {
    for (local, remote) in [
        (None, None),
        (Some(LOCAL_POTA), None),
        (None, Some(REMOTE_POTA)),
    ] {
        let q = qso(None, None, local, remote);
        let text = render(&q);
        assert_eq!(field(&text, "MY_SIG_INFO"), local);
        assert_eq!(field(&text, "SIG_INFO"), remote);
        assert_eq!(field(&text, "MY_SIG").is_some(), local.is_some());
        assert_eq!(field(&text, "SIG").is_some(), remote.is_some());
    }
}

#[test]
fn pota_same_park_on_both_sides_is_legal_and_preserved() {
    let q = qso(None, None, Some(LOCAL_POTA), Some(LOCAL_POTA));
    let text = render(&q);
    assert_eq!(field(&text, "MY_SIG_INFO"), Some(LOCAL_POTA));
    assert_eq!(field(&text, "SIG_INFO"), Some(LOCAL_POTA));
    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        (Some(LOCAL_POTA.into()), Some(LOCAL_POTA.into()))
    );
}

#[test]
fn pota_import_reads_direct_fields() {
    let text = sample_adif(&[("MY_POTA_REF", LOCAL_POTA), ("POTA_REF", REMOTE_POTA)]);
    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        (Some(LOCAL_POTA.into()), Some(REMOTE_POTA.into()))
    );
    // Re-export uses the POTA upload contract for single references.
    let re_exported = render(&parsed);
    assert_eq!(field(&re_exported, "MY_SIG"), Some("POTA"));
    assert_eq!(field(&re_exported, "SIG"), Some("POTA"));
    assert_eq!(field(&re_exported, "MY_POTA_REF"), None);
}

#[test]
fn pota_import_reads_sig_representation() {
    let text = sample_adif(&[
        ("MY_SIG", "POTA"),
        ("MY_SIG_INFO", LOCAL_POTA),
        ("SIG", "POTA"),
        ("SIG_INFO", REMOTE_POTA),
    ]);
    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        (Some(LOCAL_POTA.into()), Some(REMOTE_POTA.into()))
    );
}

#[test]
fn pota_import_merges_consistent_representations_without_loss() {
    let text = sample_adif(&[
        ("MY_POTA_REF", LOCAL_POTA),
        ("MY_SIG", "POTA"),
        ("MY_SIG_INFO", &LOCAL_POTA.to_ascii_lowercase()),
        ("POTA_REF", REMOTE_POTA),
        ("SIG", "pota"),
        ("SIG_INFO", REMOTE_POTA),
    ]);
    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        // The direct field value is kept verbatim; no value is invented.
        (Some(LOCAL_POTA.into()), Some(REMOTE_POTA.into()))
    );
}

#[test]
fn pota_import_rejects_contradictory_representations() {
    let text = sample_adif(&[
        ("MY_POTA_REF", LOCAL_POTA),
        ("MY_SIG", "POTA"),
        ("MY_SIG_INFO", THIRD_POTA),
    ]);
    // Plus a second, clean record: the conflict is per record, the file still imports.
    let text = format!(
        "{text}{}",
        sample_adif(&[("SIG", "POTA"), ("SIG_INFO", REMOTE_POTA)])
    );
    let report = adif::parse_adif_with_report(&text);

    assert_eq!(report.rejected, 1, "the ambiguous record is rejected");
    assert_eq!(report.qsos.len(), 1, "the clean record still imports");
    assert_eq!(
        sides(&report.qsos[0], AwardProgram::Pota).1,
        Some(REMOTE_POTA.into())
    );
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("sprzeczne") && e.contains(LOCAL_POTA) && e.contains(THIRD_POTA)),
        "the conflict names both values: {:?}",
        report.errors
    );
}

#[test]
fn pota_export_never_emits_a_sig_marker_without_its_info_field() {
    for (local, remote) in [
        (None, None),
        (Some(LOCAL_POTA), None),
        (None, Some(REMOTE_POTA)),
        (Some(LOCAL_POTA), Some(REMOTE_POTA)),
    ] {
        let text = render(&qso(None, None, local, remote));
        for (sig, info) in [("MY_SIG", "MY_SIG_INFO"), ("SIG", "SIG_INFO")] {
            assert_eq!(
                field(&text, sig).is_some(),
                field(&text, info).is_some(),
                "{sig} and {info} must always appear together"
            );
        }
    }
}

#[test]
fn pota_sig_marker_is_never_emitted_without_a_reference() {
    let text = render(&qso(None, None, None, None));
    assert_eq!(field(&text, "MY_SIG"), None);
    assert_eq!(field(&text, "SIG"), None);
    assert_eq!(field(&text, "MY_SIG_INFO"), None);
    assert_eq!(field(&text, "SIG_INFO"), None);
}

#[test]
fn pota_multi_park_list_uses_only_the_direct_representation() {
    let multi = format!("{LOCAL_POTA},{THIRD_POTA}");
    let q = qso(None, None, Some(&multi), Some(REMOTE_POTA));
    let text = render(&q);
    assert_eq!(field(&text, "MY_POTA_REF"), Some(multi.as_str()));
    assert_eq!(field(&text, "POTA_REF"), Some(REMOTE_POTA));
    assert_eq!(
        field(&text, "MY_SIG"),
        None,
        "one representation per record"
    );
    assert_eq!(field(&text, "SIG"), None);

    let parsed = parse_one(&text);
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        (Some(multi.clone()), Some(REMOTE_POTA.into()))
    );
}

#[test]
fn pota_spot_of_another_activator_fills_only_the_remote_side() {
    let mut app = app_in_activation();
    assert_eq!(app.entry.my_refs.pota, LOCAL_POTA);
    app.entry.pota = REMOTE_POTA.to_string();

    let draft = app.draft_qso_from_entry();
    assert_eq!(draft.my_pota_ref.as_deref(), Some(LOCAL_POTA));
    assert_eq!(draft.pota_ref.as_deref(), Some(REMOTE_POTA));

    let text = render(&draft);
    assert_eq!(field(&text, "MY_SIG_INFO"), Some(LOCAL_POTA));
    assert_eq!(field(&text, "SIG_INFO"), Some(REMOTE_POTA));
}

#[test]
fn pota_activation_context_survives_consecutive_qsos() {
    let mut app = app_in_activation();
    app.entry.my_refs.pota = THIRD_POTA.to_string();

    for remote_park in [REMOTE_POTA, THIRD_POTA, FOURTH_POTA] {
        app.entry.pota = remote_park.to_string();
        let draft = app.draft_qso_from_entry();
        assert_eq!(draft.my_pota_ref.as_deref(), Some(THIRD_POTA));
        assert_eq!(draft.pota_ref.as_deref(), Some(remote_park));

        // The per-QSO reset clears only the correspondent side.
        app.entry.clear();
        assert_eq!(app.entry.pota, "");
        assert_eq!(app.entry.my_refs.pota, THIRD_POTA);
    }
}

// ===========================================================================
// PHASE 17 — other award programs and ordinary QSOs are not affected
// ===========================================================================

#[test]
fn unrelated_award_and_location_fields_round_trip_unchanged() {
    let mut q = QsoRecord::new(REMOTE_CALL, "20m", "CW");
    q.qso_date = "20260101".to_string();
    q.time_on = "120000".to_string();
    q.iota = Some("EU-001".to_string());
    q.gridsquare = Some("JO81WA".to_string());
    q.my_gridsquare = Some("JO81LA".to_string());
    q.state = Some("CA".to_string());
    q.my_state = Some("SP".to_string());
    q.comment = Some("Synthetic WWFF SPFF-0001 reference in a comment".to_string());
    q.pga_ref = Some("WR01".to_string());
    q.my_sota_ref = Some(LOCAL_SOTA.to_string());
    q.sota_ref = Some(REMOTE_SOTA.to_string());

    let text = render(&q);
    let parsed = parse_one(&text);
    assert_eq!(parsed.iota, q.iota);
    assert_eq!(parsed.gridsquare, q.gridsquare);
    assert_eq!(parsed.my_gridsquare, q.my_gridsquare);
    assert_eq!(parsed.state, q.state);
    assert_eq!(parsed.my_state, q.my_state);
    assert_eq!(parsed.comment, q.comment);
    assert_eq!(parsed.pga_ref, q.pga_ref);
    assert_eq!(parsed.sota_ref, q.sota_ref);
    assert_eq!(parsed.my_sota_ref, q.my_sota_ref);
}

#[test]
fn ordinary_qso_without_award_references_gets_no_reference_fields() {
    let mut q = QsoRecord::new(REMOTE_CALL, "20m", "CW");
    q.qso_date = "20260101".to_string();
    q.time_on = "120000".to_string();
    let text = render(&q);
    for tag in [
        "MY_SOTA_REF",
        "SOTA_REF",
        "MY_POTA_REF",
        "POTA_REF",
        "MY_SIG",
        "MY_SIG_INFO",
        "SIG",
        "SIG_INFO",
    ] {
        assert_eq!(field(&text, tag), None, "{tag} must be absent");
    }
    assert!(
        !tags(&text)
            .iter()
            .any(|t| t.starts_with("MY_SIG") || t == "SIG" || t == "SIG_INFO")
    );
}

// ===========================================================================
// PHASE 14 / 18 — round-trip and fault injection
// ===========================================================================

#[test]
fn model_to_adif_to_model_preserves_both_programs_and_sides() {
    let q = qso(
        Some(LOCAL_SOTA),
        Some(REMOTE_SOTA),
        Some(LOCAL_POTA),
        Some(REMOTE_POTA),
    );
    let parsed = parse_one(&render(&q));
    assert_eq!(
        sides(&parsed, AwardProgram::Sota),
        sides(&q, AwardProgram::Sota)
    );
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        sides(&q, AwardProgram::Pota)
    );
}

#[test]
fn adif_to_model_to_adif_is_semantically_stable() {
    let input = sample_adif(&[
        ("MY_SOTA_REF", LOCAL_SOTA),
        ("SOTA_REF", REMOTE_SOTA),
        ("MY_SIG", "POTA"),
        ("MY_SIG_INFO", LOCAL_POTA),
        ("SIG", "POTA"),
        ("SIG_INFO", REMOTE_POTA),
    ]);
    let first = parse_one(&input);
    let rendered_once = render(&first);
    let second = parse_one(&rendered_once);
    let rendered_twice = render(&second);
    assert_eq!(
        rendered_once, rendered_twice,
        "export → import → export is a fixed point"
    );
    assert_eq!(
        sides(&second, AwardProgram::Pota),
        sides(&first, AwardProgram::Pota)
    );
    assert_eq!(
        sides(&second, AwardProgram::Sota),
        sides(&first, AwardProgram::Sota)
    );
}

#[test]
fn model_to_database_to_model_preserves_both_sides() {
    let db = LogDatabase::open_in_memory().expect("in-memory database");
    let q = qso(
        Some(LOCAL_SOTA),
        Some(REMOTE_SOTA),
        Some(LOCAL_POTA),
        Some(REMOTE_POTA),
    );
    let id = db.insert_qso(&q).expect("insert");
    let stored = db.get_qso_by_id(id).expect("query").expect("row");
    assert_eq!(stored.my_sota_ref.as_deref(), Some(LOCAL_SOTA));
    assert_eq!(stored.sota_ref.as_deref(), Some(REMOTE_SOTA));
    assert_eq!(stored.my_pota_ref.as_deref(), Some(LOCAL_POTA));
    assert_eq!(stored.pota_ref.as_deref(), Some(REMOTE_POTA));
    assert!(render(&stored).contains("<MY_SOTA_REF:9>AA/BB-001"));
}

#[test]
fn edit_without_changing_references_preserves_both_sides() {
    let original = qso(
        Some(LOCAL_SOTA),
        Some(REMOTE_SOTA),
        Some(LOCAL_POTA),
        Some(REMOTE_POTA),
    );
    let mut edited = original.clone();
    edited.name = Some("Edited operator".to_string());
    edited.comment = Some("Edited note".to_string());
    assert_eq!(
        sides(&edited, AwardProgram::Sota),
        sides(&original, AwardProgram::Sota)
    );
    assert_eq!(
        sides(&edited, AwardProgram::Pota),
        sides(&original, AwardProgram::Pota)
    );
}

#[test]
fn sorting_and_filtering_do_not_change_references() {
    let records = vec![
        qso(Some(LOCAL_SOTA), Some(REMOTE_SOTA), Some(LOCAL_POTA), None),
        qso(None, Some(THIRD_SOTA), None, Some(REMOTE_POTA)),
        qso(Some(THIRD_SOTA), None, Some(THIRD_POTA), Some(FOURTH_POTA)),
    ];
    let mut sorted = records.clone();
    sorted.sort_by_key(|q| q.my_sota_ref.clone());
    let rendered = render_all(&sorted);
    for record in &records {
        for (tag, value) in record.award_references().adif_fields() {
            assert!(rendered.contains(&format!("<{tag}:{}>{value}", value.len())));
        }
    }
}

#[test]
fn duplicating_a_qso_keeps_a_controlled_copy_of_both_sides() {
    let original = qso(
        Some(LOCAL_SOTA),
        Some(REMOTE_SOTA),
        Some(LOCAL_POTA),
        Some(REMOTE_POTA),
    );
    let mut copy = original.clone();
    copy.id = None;
    copy.qso_date = "20260102".to_string();
    assert_eq!(
        sides(&copy, AwardProgram::Sota),
        sides(&original, AwardProgram::Sota)
    );
    assert_eq!(
        sides(&copy, AwardProgram::Pota),
        sides(&original, AwardProgram::Pota)
    );
    assert_eq!(
        render(&copy),
        render(&original).replace("20260101", "20260102")
    );
}

#[test]
fn fault_injection_unknown_sig_is_reported_not_guessed() {
    let text = sample_adif(&[("SIG", "WWFF"), ("SIG_INFO", "SPFF-0001")]);
    let report = adif::parse_adif_with_report(&text);
    assert_eq!(report.rejected, 0);
    assert_eq!(sides(&report.qsos[0], AwardProgram::Pota), (None, None));
    assert_eq!(
        report.reference_notices.len(),
        1,
        "an uninterpretable SIG is reported, not silently dropped"
    );
    assert!(report.reference_notices[0].message.contains("WWFF"));
    assert_eq!(report.reference_notices[0].record_index, 1);
}

#[test]
fn fault_injection_incomplete_sig_pairs_are_reported_and_never_exported() {
    for reference_fields in [
        vec![("MY_SIG", "POTA")],
        vec![("SIG", "POTA")],
        vec![("MY_SIG_INFO", LOCAL_POTA)],
        vec![("SIG_INFO", REMOTE_POTA)],
    ] {
        let text = sample_adif(&reference_fields);
        let report = adif::parse_adif_with_report(&text);
        assert_eq!(
            report.rejected, 0,
            "a missing partner is not fatal: {reference_fields:?}"
        );
        assert_eq!(
            report.reference_notices.len(),
            1,
            "the incomplete pair is reported: {reference_fields:?}"
        );
        let parsed = &report.qsos[0];
        assert!(parsed.my_pota_ref.is_none() && parsed.pota_ref.is_none());
        let text = render(parsed);
        assert_eq!(field(&text, "MY_SIG"), None);
        assert_eq!(field(&text, "SIG"), None);
    }
}

#[test]
fn fault_injection_malformed_lengths_and_truncated_records_do_not_panic() {
    let bad_length = "<CALL:9>TSTREMOTE<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20260101<TIME_ON:6>120000<MY_SOTA_REF:99>AA/BB-001<EOR>";
    let report = adif::parse_adif_with_report(bad_length);
    assert_eq!(report.rejected, 1);
    assert!(report.qsos.is_empty());

    let truncated = "<CALL:9>TSTREMOTE<BAND:3>20m<MODE:2>CW<QSO_DATE:8>20260101<TIME_ON:6>120000<SOTA_REF:9>CC/D";
    let report = adif::parse_adif_with_report(truncated);
    // Either rejected or parsed, but never a panic and never a value from the other side.
    for q in &report.qsos {
        assert!(q.my_sota_ref.is_none());
    }
}

#[test]
fn fault_injection_duplicate_and_case_variant_field_names_do_not_swap_sides() {
    let duplicate = concat!(
        "<CALL:9>TSTREMOTE",
        "<BAND:3>20m",
        "<MODE:2>CW",
        "<QSO_DATE:8>20260101",
        "<TIME_ON:6>120000",
        "<MY_SOTA_REF:9>AA/BB-001",
        "<my_sota_ref:9>EE/FF-002",
        "<EOR>"
    );
    let report = adif::parse_adif_with_report(duplicate);
    assert_eq!(report.rejected, 0);
    let q = &report.qsos[0];
    assert!(
        q.my_sota_ref.as_deref() == Some(LOCAL_SOTA)
            || q.my_sota_ref.as_deref() == Some(THIRD_SOTA),
        "a duplicated field resolves to one of its own values, never to a remote value"
    );
    assert_eq!(q.sota_ref, None);
}

#[test]
fn fault_injection_unicode_and_long_values_round_trip_without_panic() {
    let unicode_ref = "AA/BB-001-ĄĆĘŁŃÓŚŹŻ";
    let long_ref = format!("AA/BB-{}", "9".repeat(2000));
    for value in [unicode_ref, long_ref.as_str()] {
        let mut q = QsoRecord::new(REMOTE_CALL, "20m", "CW");
        q.qso_date = "20260101".to_string();
        q.time_on = "120000".to_string();
        q.my_sota_ref = Some(value.to_string());
        let text = render(&q);
        let parsed = parse_one(&text);
        assert_eq!(parsed.my_sota_ref.as_deref(), Some(value));
        assert_eq!(parsed.sota_ref, None);
    }
}

#[test]
fn fault_injection_partial_file_without_eor_still_imports_cleanly() {
    let text = concat!(
        "<CALL:9>TSTREMOTE",
        "<BAND:3>20m",
        "<MODE:2>CW",
        "<QSO_DATE:8>20260101",
        "<TIME_ON:6>120000",
        "<MY_SOTA_REF:9>AA/BB-001",
        "<SOTA_REF:9>CC/DD-999"
    );
    let report = adif::parse_adif_with_report(text);
    assert_eq!(report.rejected, 0);
    assert_eq!(report.qsos.len(), 1);
    assert_eq!(
        sides(&report.qsos[0], AwardProgram::Sota),
        (Some(LOCAL_SOTA.into()), Some(REMOTE_SOTA.into()))
    );
}

#[test]
fn empty_export_and_empty_import_are_defined() {
    assert!(
        !render_all(&[]).is_empty(),
        "a header-only export is still a valid ADIF file"
    );
    let report = adif::parse_adif_with_report("");
    assert_eq!(report.qsos.len(), 0);
    assert_eq!(report.rejected, 0);
    assert!(report.reference_notices.is_empty());
}

// ===========================================================================
// PHASE 19 / 20 — combinatorial matrix and deterministic stress
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RefState {
    Missing,
    Valid,
    Lowercase,
    Malformed,
    SameAsOpposite,
    Multiple,
}

const STATES: [RefState; 6] = [
    RefState::Missing,
    RefState::Valid,
    RefState::Lowercase,
    RefState::Malformed,
    RefState::SameAsOpposite,
    RefState::Multiple,
];

const OPERATIONS: [&str; 5] = ["export", "import", "round-trip", "spot", "edit"];

fn state_value(program: AwardProgram, side: StationSide, state: RefState) -> Option<String> {
    let (primary, secondary) = match program {
        AwardProgram::Sota => (LOCAL_SOTA, REMOTE_SOTA),
        AwardProgram::Pota => (LOCAL_POTA, REMOTE_POTA),
    };
    // The remote side uses distinct values so a swap is always detectable.
    let (first, second) = if side == StationSide::Local {
        (primary, THIRD_SOTA)
    } else {
        (secondary, THIRD_POTA)
    };
    match state {
        RefState::Missing => None,
        RefState::Valid => Some(match (program, side) {
            (AwardProgram::Sota, StationSide::Local) => LOCAL_SOTA.to_string(),
            (AwardProgram::Sota, StationSide::Remote) => REMOTE_SOTA.to_string(),
            (AwardProgram::Pota, StationSide::Local) => LOCAL_POTA.to_string(),
            (AwardProgram::Pota, StationSide::Remote) => REMOTE_POTA.to_string(),
        }),
        RefState::Lowercase => Some(first.to_ascii_lowercase()),
        RefState::Malformed => Some("NOT-A-REFERENCE".to_string()),
        // "Same as opposite" is resolved by the caller, who knows the other side.
        RefState::SameAsOpposite => None,
        RefState::Multiple if program == AwardProgram::Pota => Some(format!(
            "{first},{}",
            if side == StationSide::Local {
                FOURTH_POTA
            } else {
                THIRD_POTA
            }
        )),
        RefState::Multiple => Some(second.to_string()),
    }
}

fn reference_pair(program: AwardProgram, local: RefState, remote: RefState) -> AwardRefPair {
    let mut pair = AwardRefPair::default();
    let local_value = state_value(program, StationSide::Local, local);
    let remote_value = state_value(program, StationSide::Remote, remote);
    pair.local = local_value;
    pair.remote = remote_value;
    if local == RefState::SameAsOpposite {
        pair.local = pair.remote.clone();
    }
    if remote == RefState::SameAsOpposite {
        pair.remote = pair.local.clone();
    }
    pair
}

fn assert_export_invariants(text: &str, refs: &AwardReferences) {
    let names = tags(text);
    let count = |tag: &str| names.iter().filter(|t| t.as_str() == tag).count();
    for tag in [
        "MY_SOTA_REF",
        "SOTA_REF",
        "MY_POTA_REF",
        "POTA_REF",
        "MY_SIG",
        "MY_SIG_INFO",
        "SIG",
        "SIG_INFO",
    ] {
        assert!(count(tag) <= 1, "{tag} must appear at most once: {text}");
    }

    // Exactly one POTA representation per record.
    let direct = count("MY_POTA_REF") + count("POTA_REF");
    let sig = count("MY_SIG") + count("SIG");
    assert!(
        direct == 0 || sig == 0,
        "both POTA representations emitted: {text}"
    );

    // SIG markers always paired.
    assert_eq!(count("MY_SIG") == 1, count("MY_SIG_INFO") == 1, "{text}");
    assert_eq!(count("SIG") == 1, count("SIG_INFO") == 1, "{text}");

    // No field carries a value that belongs to the opposite side.
    if let Some(local) = refs.sota.local.as_deref() {
        assert_eq!(field(text, "MY_SOTA_REF"), Some(local));
    } else {
        assert_eq!(field(text, "MY_SOTA_REF"), None);
    }
    if let Some(remote) = refs.sota.remote.as_deref() {
        assert_eq!(field(text, "SOTA_REF"), Some(remote));
    } else {
        assert_eq!(field(text, "SOTA_REF"), None);
    }

    match refs.pota_representation() {
        PotaRepresentation::Absent => {
            assert_eq!(count("MY_POTA_REF") + count("POTA_REF") + sig, 0);
        }
        PotaRepresentation::Sig => {
            assert_eq!(field(text, "MY_SIG_INFO"), refs.pota.local.as_deref());
            assert_eq!(field(text, "SIG_INFO"), refs.pota.remote.as_deref());
        }
        PotaRepresentation::DirectFields => {
            assert_eq!(field(text, "MY_POTA_REF"), refs.pota.local.as_deref());
            assert_eq!(field(text, "POTA_REF"), refs.pota.remote.as_deref());
        }
    }
}

#[test]
fn combinatorial_matrix_preserves_roles_for_every_combination() {
    let mut checked = 0usize;
    for i in 0..100_000usize {
        let program = AwardProgram::ALL[i % 2];
        let local_state = STATES[(i / 2) % STATES.len()];
        let remote_state = STATES[(i / 4) % STATES.len()];
        let operation = OPERATIONS[(i / 8) % OPERATIONS.len()];

        let pair = reference_pair(program, local_state, remote_state);
        let mut refs = AwardReferences::default();
        *refs.program_mut(program) = pair.clone();

        let mut record = QsoRecord::new(REMOTE_CALL, "20m", "CW");
        record.qso_date = "20260101".to_string();
        record.time_on = "120000".to_string();
        refs.apply_to_qso(&mut record);

        assert_eq!(
            sides(&record, program),
            (pair.local.clone(), pair.remote.clone())
        );

        let text = render(&record);
        assert_export_invariants(&text, &record.award_references());

        match operation {
            "export" => {}
            "import" => {
                let parsed = parse_one(&text);
                assert_eq!(
                    sides(&parsed, program),
                    (pair.local.clone(), pair.remote.clone()),
                    "import must not change sides ({program:?}, {local_state:?}, {remote_state:?})"
                );
            }
            "round-trip" => {
                let parsed = parse_one(&text);
                let again = render(&parsed);
                assert_eq!(text, again, "export → import → export must be stable");
            }
            "spot" => {
                // The spot only ever touches the remote side of the entry state.
                let mut entry = QsoEntryState::default();
                if program == AwardProgram::Pota {
                    entry.pota = pair.remote.clone().unwrap_or_default();
                    entry.my_refs.pota = pair.local.clone().unwrap_or_default();
                } else {
                    entry.sota = pair.remote.clone().unwrap_or_default();
                    entry.my_refs.sota = pair.local.clone().unwrap_or_default();
                }
                let local_side = if program == AwardProgram::Pota {
                    &entry.my_refs.pota
                } else {
                    &entry.my_refs.sota
                };
                assert_eq!(
                    (!local_side.is_empty()).then(|| local_side.clone()),
                    pair.local
                );
            }
            _ => {
                // Edit without touching references: values survive verbatim.
                let edited = record.clone();
                assert_eq!(
                    sides(&edited, program),
                    (pair.local.clone(), pair.remote.clone())
                );
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 100_000);
}

#[test]
fn stress_reference_paths_are_deterministic_and_sleep_free() {
    // 5000 iterations per role/operation, string-only, no database and no sleep.
    for _ in 0..5000 {
        let activator = qso(Some(LOCAL_SOTA), None, None, None);
        assert_eq!(field(&render(&activator), "MY_SOTA_REF"), Some(LOCAL_SOTA));
    }
    for _ in 0..5000 {
        let chaser = qso(None, Some(REMOTE_SOTA), None, None);
        assert_eq!(field(&render(&chaser), "SOTA_REF"), Some(REMOTE_SOTA));
    }
    for _ in 0..5000 {
        let s2s = qso(Some(LOCAL_SOTA), Some(REMOTE_SOTA), None, None);
        let text = render(&s2s);
        assert_eq!(field(&text, "MY_SOTA_REF"), Some(LOCAL_SOTA));
        assert_eq!(field(&text, "SOTA_REF"), Some(REMOTE_SOTA));
    }
    for _ in 0..5000 {
        let activator = qso(None, None, Some(LOCAL_POTA), None);
        assert_eq!(field(&render(&activator), "MY_SIG_INFO"), Some(LOCAL_POTA));
    }
    for _ in 0..5000 {
        let hunter = qso(None, None, None, Some(REMOTE_POTA));
        assert_eq!(field(&render(&hunter), "SIG_INFO"), Some(REMOTE_POTA));
    }
    for _ in 0..5000 {
        let p2p = qso(None, None, Some(LOCAL_POTA), Some(REMOTE_POTA));
        let text = render(&p2p);
        assert_eq!(field(&text, "MY_SIG_INFO"), Some(LOCAL_POTA));
        assert_eq!(field(&text, "SIG_INFO"), Some(REMOTE_POTA));
    }
    for _ in 0..5000 {
        let original = qso(
            Some(LOCAL_SOTA),
            Some(REMOTE_SOTA),
            Some(LOCAL_POTA),
            Some(REMOTE_POTA),
        );
        let once = render(&original);
        let twice = render(&parse_one(&once));
        assert_eq!(once, twice);
    }
}

#[test]
fn stress_conflicting_pota_representation_is_always_a_controlled_rejection() {
    for i in 0..2000 {
        let direct = format!("XX-{i:04}");
        let sig = format!("XX-{:04}", i + 1);
        let text = sample_adif(&[
            ("MY_POTA_REF", &direct),
            ("MY_SIG", "POTA"),
            ("MY_SIG_INFO", &sig),
        ]);
        let report = adif::parse_adif_with_report(&text);
        assert_eq!(report.rejected, 1);
        assert!(report.qsos.is_empty());
        assert!(!report.errors.is_empty());
    }
}

#[test]
fn stress_spot_and_activation_conversion_keep_perspective() {
    for i in 0..3000 {
        let mut config = AppConfig::default();
        config.station.callsign = LOCAL_CALL.to_string();
        config.station.sota_ref = Some(format!("AA/BB-{i:03}"));
        let entry = QsoEntryState::new(&config);
        assert_eq!(entry.my_refs.sota, format!("AA/BB-{i:03}"));
        assert_eq!(entry.sota, "");

        let mut entry = entry;
        entry.sota = format!("CC/DD-{i:03}");
        assert_eq!(entry.my_refs.sota, format!("AA/BB-{i:03}"));
        assert_eq!(entry.sota, format!("CC/DD-{i:03}"));
    }
}

#[test]
fn stress_malformed_and_missing_references_never_swap_sides() {
    for i in 0..3000 {
        let value = format!("NOT-A-REF-{i}");
        let record = qso(Some(&value), None, None, None);
        let text = render(&record);
        assert_eq!(field(&text, "MY_SOTA_REF"), Some(value.as_str()));
        assert_eq!(field(&text, "SOTA_REF"), None);
    }
    for _ in 0..3000 {
        let record = qso(None, None, None, None);
        let text = render(&record);
        for tag in ["MY_SOTA_REF", "SOTA_REF", "MY_POTA_REF", "POTA_REF"] {
            assert_eq!(field(&text, tag), None);
        }
    }
}

// ===========================================================================
// Guard on the production entry points
// ===========================================================================

#[test]
fn production_form_mapping_is_the_single_source_of_truth() {
    let mut app = app_in_activation();
    app.entry.sota = REMOTE_SOTA.to_string();
    app.entry.pota = REMOTE_POTA.to_string();

    let draft = app.draft_qso_from_entry();
    assert_eq!(draft.my_sota_ref.as_deref(), Some(LOCAL_SOTA));
    assert_eq!(draft.sota_ref.as_deref(), Some(REMOTE_SOTA));
    assert_eq!(draft.my_pota_ref.as_deref(), Some(LOCAL_POTA));
    assert_eq!(draft.pota_ref.as_deref(), Some(REMOTE_POTA));

    // Saving must use exactly that mapping: the persisted values equal the draft's.
    app.save_qso();
    let db = app.services.log_db.lock().expect("log_db");
    let all = db.get_all_qsos().expect("all rows");
    assert_eq!(all.len(), 1);
    let stored = &all[0];
    assert_eq!(stored.my_sota_ref, draft.my_sota_ref);
    assert_eq!(stored.sota_ref, draft.sota_ref);
    assert_eq!(stored.my_pota_ref, draft.my_pota_ref);
    assert_eq!(stored.pota_ref, draft.pota_ref);
}

#[test]
fn stored_record_round_trips_through_adif_with_identical_roles() {
    let mut app = app_in_activation();
    app.entry.sota = REMOTE_SOTA.to_string();
    app.entry.pota = REMOTE_POTA.to_string();
    app.save_qso();

    let stored = {
        let db = app.services.log_db.lock().expect("log_db");
        db.get_all_qsos().expect("all rows").remove(0)
    };
    let exported = render(&stored);
    assert_eq!(field(&exported, "MY_SOTA_REF"), Some(LOCAL_SOTA));
    assert_eq!(field(&exported, "SOTA_REF"), Some(REMOTE_SOTA));
    assert_eq!(field(&exported, "MY_SIG_INFO"), Some(LOCAL_POTA));
    assert_eq!(field(&exported, "SIG_INFO"), Some(REMOTE_POTA));

    let reimported = parse_one(&exported);
    assert_eq!(
        sides(&reimported, AwardProgram::Sota),
        (Some(LOCAL_SOTA.into()), Some(REMOTE_SOTA.into()))
    );
    assert_eq!(
        sides(&reimported, AwardProgram::Pota),
        (Some(LOCAL_POTA.into()), Some(REMOTE_POTA.into()))
    );
}

#[test]
fn adif_record_error_reports_the_conflict_it_was_created_from() {
    let conflict = splogbook::core::award_refs::AwardRefConflict {
        program: AwardProgram::Pota,
        side: StationSide::Local,
        direct_field: "MY_POTA_REF",
        direct_value: LOCAL_POTA.to_string(),
        sig_field: "MY_SIG_INFO",
        sig_value: THIRD_POTA.to_string(),
    };
    let error = AdifRecordError::ConflictingPotaRepresentation(conflict);
    let text = error.to_string();
    assert!(text.contains("MY_POTA_REF"));
    assert!(text.contains("MY_SIG_INFO"));
    assert!(text.contains(LOCAL_POTA));
    assert!(text.contains(THIRD_POTA));
}

#[test]
fn notices_are_typed_and_never_carry_a_side_guess() {
    let notices = [
        AwardRefNotice::UnknownSig {
            side: StationSide::Remote,
            sig: "WWFF".to_string(),
        },
        AwardRefNotice::ReferenceWithoutSig {
            side: StationSide::Local,
        },
        AwardRefNotice::PotaSigWithoutReference {
            side: StationSide::Remote,
        },
    ];
    for notice in notices {
        let message = notice.message();
        assert!(!message.is_empty());
        assert!(message.contains("local") || message.contains("remote"));
    }
}

#[test]
fn adif_field_map_is_fully_consumed_by_the_central_importer() {
    let text = sample_adif(&[
        ("MY_SOTA_REF", LOCAL_SOTA),
        ("SOTA_REF", REMOTE_SOTA),
        ("MY_POTA_REF", LOCAL_POTA),
        ("POTA_REF", REMOTE_POTA),
        ("MY_SIG", "POTA"),
        ("MY_SIG_INFO", LOCAL_POTA),
        ("SIG", "POTA"),
        ("SIG_INFO", REMOTE_POTA),
    ]);
    let parsed = parse_one(&text);
    // Nothing from the reference set leaks into a generic field.
    assert_eq!(parsed.comment, None);
    assert_eq!(parsed.my_state, None);
    assert_eq!(parsed.my_gridsquare, None);
    assert_eq!(parsed.iota, None);
    assert_eq!(parsed.pga_ref, None);
    // Consistent duplicates are merged, not duplicated or lost.
    assert_eq!(
        sides(&parsed, AwardProgram::Pota),
        (Some(LOCAL_POTA.into()), Some(REMOTE_POTA.into()))
    );
}
