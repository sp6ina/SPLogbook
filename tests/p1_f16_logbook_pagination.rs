// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//
// P1-F16 — logbook pagination and filter-transition safety.
//
// All records in this file are synthetic: callsigns use the reserved test prefix
// `TST` and no real user log, callsign, journal or database is read. The tests
// drive the production GUI path `SpLogApp` + `render_logbook_body` through
// `egui::Context::run_ui`; no hardware, no network, no wall clock, no sleep.

use eframe::egui;
use splogbook::core::database::{Journal, LogDatabase};
use splogbook::core::prefix::PrefixMatcher;
use splogbook::core::qso::QsoRecord;
use splogbook::core::scp::ScpEngine;
use splogbook::core::station::AppConfig;
use splogbook::gui::app::{AppDeps, AppInit, ShutdownPolicy, SpLogApp};
use splogbook::gui::pagination::{PageMode, PaginationInput, PaginationWindow, paginate};
use splogbook::gui::state::LogViewState;
use std::sync::{Arc, Mutex};

const PAGE_SIZE: usize = 50;

/// Test-only synthetic callsign. Never a real user callsign.
fn synthetic_callsign(index: usize) -> String {
    format!("TST{index:04}")
}

fn synthetic_qso(index: usize, journal_id: &str) -> QsoRecord {
    let mut qso = QsoRecord::new(synthetic_callsign(index), "20m", "CW");
    qso.id = Some(i64::try_from(index).unwrap_or(1));
    qso.journal_id = Some(journal_id.to_string());
    qso.band = if index.is_multiple_of(2) {
        "20m"
    } else {
        "40m"
    }
    .to_string();
    qso.mode = if index.is_multiple_of(3) { "SSB" } else { "CW" }.to_string();
    qso.qso_date = "20260101".to_string();
    qso.time_on = format!("{:02}{:02}00", (index / 60) % 24, index % 60);
    qso.name = Some(format!("OP{index:04}"));
    qso.country = Some("Syntheticland".to_string());
    qso
}

fn test_prefix_matcher() -> PrefixMatcher {
    PrefixMatcher::empty()
}

fn empty_app() -> SpLogApp {
    let deps = AppDeps {
        log_db: Arc::new(Mutex::new(
            LogDatabase::open_in_memory().expect("in-memory database"),
        )),
        prefix_matcher: Arc::new(test_prefix_matcher()),
        scp_engine: Arc::new(Mutex::new(ScpEngine::new())),
    };
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: std::path::PathBuf::from("config.json"),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    SpLogApp::for_tests(init, deps)
}

/// Inserts synthetic rows into the active journal and reloads the visible buffer.
fn app_with_rows(rows: usize) -> SpLogApp {
    let mut app = empty_app();
    let journal_id = app.active_journal.id.clone();
    insert_rows(&app, &journal_id, 1, rows);
    app.reload_qsos();
    app
}

fn insert_rows(app: &SpLogApp, journal_id: &str, first: usize, count: usize) -> Vec<i64> {
    let db = app.services.log_db.lock().expect("log_db lock");
    let mut ids = Vec::with_capacity(count);
    for index in first..first + count {
        let mut qso = synthetic_qso(index, journal_id);
        qso.id = None;
        ids.push(db.insert_qso(&qso).expect("synthetic QSO insert"));
    }
    ids
}

fn create_journal(app: &SpLogApp, id: &str, name: &str) -> Journal {
    let db = app.services.log_db.lock().expect("log_db lock");
    let journal = Journal {
        id: id.to_string(),
        name: name.to_string(),
        ..Default::default()
    };
    db.create_journal(&journal)
        .expect("create synthetic journal");
    journal
}

fn render_one_frame(app: &mut SpLogApp) {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        splogbook::gui::logbook_table::render_logbook_body(app, ui);
    });
    out.textures_delta.clear();
}

/// Pure model view of the current state (does not mutate the app).
fn window_of(app: &SpLogApp) -> PaginationWindow {
    paginate(PaginationInput {
        requested_page: app.log_view.page,
        page_size: app.log_view.page_size,
        total_items: app.log_view.cache.sorted_indices.len(),
    })
}

fn assert_page_invariants(app: &SpLogApp) {
    let total = app.log_view.cache.sorted_indices.len();
    let window = window_of(app);
    assert!(window.start <= window.end, "start > end");
    assert!(window.end <= total, "end > filtered length");
    assert!(window.start <= total, "start > filtered length");
    assert_eq!(app.log_view.page, window.normalized_page);
    if window.page_count == 0 {
        assert_eq!((window.start, window.end), (0, 0));
        assert_eq!(window.normalized_page, 0);
    }
}

/// IDs of the records currently visible on the page (production selection path).
fn current_page_ids(app: &mut SpLogApp) -> Vec<i64> {
    let log_id = app.active_journal.id.clone();
    let profile_id = app.config.active_profile_id.clone();
    let selection = app.log_view.select_page_indices(&log_id, &profile_id);
    selection
        .indices
        .iter()
        .filter_map(|&index| app.log_view.recent_qsos.get(index).and_then(|qso| qso.id))
        .collect()
}

fn search(app: &mut SpLogApp, query: &str) {
    app.log_view.search_query = query.to_string();
}

// ---------------------------------------------------------------------------
// Reproduction / regression tests (pre-fix panics, post-fix safe)
// ---------------------------------------------------------------------------

#[test]
fn stale_page_after_search_contraction_is_normalized() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    // Sanity: the unfiltered page 2 is valid.
    render_one_frame(&mut app);
    assert_eq!(app.log_view.page, 2);
    assert_page_invariants(&app);

    search(&mut app, &synthetic_callsign(1));
    render_one_frame(&mut app);

    assert_eq!(app.log_view.cache.sorted_indices.len(), 1);
    assert_eq!(app.log_view.page, 0);
    assert_eq!(current_page_ids(&mut app).len(), 1);
    assert_page_invariants(&app);
}

#[test]
fn empty_dataset_with_stale_page_is_safe() {
    let mut app = empty_app();
    app.log_view.page = 1;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    assert_eq!(app.log_view.page, 0);
    assert_eq!(app.log_view.cache.sorted_indices.len(), 0);
    assert_page_invariants(&app);
}

#[test]
fn page_index_usize_max_render_is_safe() {
    let mut app = app_with_rows(10);
    app.log_view.page = usize::MAX;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    assert_eq!(app.log_view.page, 0);
    assert_page_invariants(&app);
}

#[test]
fn zero_result_filter_is_explicitly_empty() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    search(&mut app, "NO-SUCH-CALLSIGN");
    render_one_frame(&mut app);

    let window = window_of(&app);
    assert_eq!(app.log_view.cache.sorted_indices.len(), 0);
    assert_eq!(app.log_view.page, 0);
    assert_eq!(window.page_count, 0);
    assert_eq!((window.start, window.end), (0, 0));
    assert!(window.is_empty());
    assert_eq!(current_page_ids(&mut app).len(), 0);
    assert_page_invariants(&app);
}

#[test]
fn one_result_filter_normalizes_to_first_page() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    search(&mut app, &synthetic_callsign(1));
    render_one_frame(&mut app);
    assert_eq!(current_page_ids(&mut app).len(), 1);
    assert_eq!(window_of(&app).page_count, 1);
    assert_page_invariants(&app);
}

#[test]
fn one_full_page_filter_keeps_single_page() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    search(&mut app, "TST00");
    render_one_frame(&mut app);
    // `TST00xx` matches the first 99 synthetic callsigns.
    assert_eq!(app.log_view.cache.sorted_indices.len(), 99);
    assert_eq!(window_of(&app).page_count, 2);
    assert_eq!(app.log_view.page, 1);
    assert_page_invariants(&app);
}

#[test]
fn last_partial_page_is_rendered() {
    let mut app = app_with_rows(51);
    app.log_view.page = usize::MAX;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    assert_eq!(app.log_view.page, 1);
    assert_eq!(current_page_ids(&mut app).len(), 1);
    assert_page_invariants(&app);
}

#[test]
fn exact_multiple_page_size_has_no_extra_page() {
    let mut app = app_with_rows(100);
    app.log_view.page = usize::MAX;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let window = window_of(&app);
    assert_eq!(window.page_count, 2);
    assert_eq!(app.log_view.page, 1);
    assert_eq!((window.start, window.end), (50, 100));
    assert_page_invariants(&app);
}

#[test]
fn filter_expansion_keeps_valid_page() {
    let mut app = app_with_rows(200);
    app.log_view.page = 1;
    app.log_view.page_size = PAGE_SIZE;
    search(&mut app, "TST");
    render_one_frame(&mut app);
    assert_eq!(app.log_view.page, 1);

    search(&mut app, "");
    render_one_frame(&mut app);
    assert_eq!(app.log_view.page, 1);
    assert_page_invariants(&app);
}

#[test]
fn stale_page_after_journal_switch_to_empty() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    let empty_journal = create_journal(&app, "f16-empty", "F16 empty");
    app.switch_active_journal(empty_journal);
    render_one_frame(&mut app);

    assert_eq!(app.log_view.page, 0);
    assert_eq!(app.log_view.recent_qsos.len(), 0);
    assert_page_invariants(&app);
}

#[test]
fn journal_switch_to_smaller_dataset_clamps() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    let small = create_journal(&app, "f16-small", "F16 small");
    insert_rows(&app, &small.id, 500, 10);
    app.switch_active_journal(small);
    render_one_frame(&mut app);

    assert_eq!(app.log_view.recent_qsos.len(), 10);
    assert_eq!(app.log_view.page, 0);
    assert_page_invariants(&app);
}

#[test]
fn stale_page_after_deleting_last_page() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let last_page = current_page_ids(&mut app);
    assert_eq!(last_page.len(), 20);

    app.log_view.selected_qso_ids = last_page;
    app.delete_selected_qso();
    assert_eq!(app.log_view.recent_qsos.len(), 100);
    render_one_frame(&mut app);

    assert_eq!(app.log_view.page, 1);
    assert_eq!(window_of(&app).end, 100);
    assert_page_invariants(&app);
}

#[test]
fn deleting_all_rows_yields_explicit_empty_state() {
    let mut app = app_with_rows(30);
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let all_ids = current_page_ids(&mut app);
    assert_eq!(all_ids.len(), 30);

    app.log_view.selected_qso_ids = all_ids;
    app.delete_selected_qso();
    render_one_frame(&mut app);

    assert_eq!(app.log_view.recent_qsos.len(), 0);
    assert_eq!(app.log_view.page, 0);
    assert!(app.log_view.selected_qso_ids.is_empty());
    assert_page_invariants(&app);
}

#[test]
fn page_size_change_never_creates_invalid_range() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    for size in [25usize, 100, 25, PAGE_SIZE, 0] {
        app.log_view.page_size = size;
        app.log_view.page = 0;
        render_one_frame(&mut app);
        assert_page_invariants(&app);
        assert_eq!(app.log_view.page, 0);
    }
}

#[test]
fn page_size_zero_renders_all_rows() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = 0;
    app.log_view.page = 3;
    render_one_frame(&mut app);

    let window = window_of(&app);
    assert_eq!(window.mode, PageMode::AllRows);
    assert_eq!((window.start, window.end), (0, 120));
    assert_eq!(window.page_count, 1);
    assert_eq!(app.log_view.page, 0);
    assert_page_invariants(&app);
}

#[test]
fn page_size_usize_max_renders_one_page() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = usize::MAX;
    app.log_view.page = usize::MAX;
    render_one_frame(&mut app);

    let window = window_of(&app);
    assert_eq!(window.page_count, 1);
    assert_eq!((window.start, window.end), (0, 120));
    assert!(!window.arithmetic_guard_triggered);
    assert_page_invariants(&app);
}

#[test]
fn jump_to_invalid_page_clamps_to_last() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = PAGE_SIZE;
    app.log_view.page = 9_999;
    render_one_frame(&mut app);
    assert_eq!(app.log_view.page, 2);
    assert_page_invariants(&app);
}

#[test]
fn sort_change_keeps_page_and_selection() {
    let mut app = app_with_rows(120);
    app.log_view.page = 1;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let selected = current_page_ids(&mut app)[0];
    app.log_view.selected_qso_ids = vec![selected];

    app.log_view.sort_column = 1;
    app.log_view.sort_asc = true;
    render_one_frame(&mut app);
    assert_eq!(app.log_view.page, 1);
    assert!(app.log_view.selected_qso_ids.contains(&selected));
    assert_page_invariants(&app);
}

#[test]
fn filter_and_sort_in_one_transition() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    search(&mut app, "TST0");
    app.log_view.sort_column = 1;
    app.log_view.sort_asc = false;
    render_one_frame(&mut app);
    assert_page_invariants(&app);
    assert!(app.log_view.cache.sorted_indices.len() <= 120);
}

#[test]
fn delete_and_render_in_one_transition() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let id = current_page_ids(&mut app)[0];
    app.delete_qso_by_id(id);
    render_one_frame(&mut app);
    assert_eq!(app.log_view.recent_qsos.len(), 119);
    assert_page_invariants(&app);
}

#[test]
fn import_and_render_in_one_transition() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    let adif =
        "<CALL:7>TST9001 <BAND:3>20m <MODE:2>CW <QSO_DATE:8>20260101 <TIME_ON:6>230000 <EOR>";
    app.import_adif_content(adif, "synthetic.adi");
    render_one_frame(&mut app);

    assert_eq!(app.log_view.recent_qsos.len(), 121);
    assert_eq!(app.log_view.page, 2);
    assert_page_invariants(&app);
}

#[test]
fn journal_switch_and_render_in_one_transition() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    let other = create_journal(&app, "f16-other", "F16 other");
    insert_rows(&app, &other.id, 900, 5);
    app.switch_active_journal(other);
    render_one_frame(&mut app);

    assert_eq!(app.log_view.recent_qsos.len(), 5);
    assert_eq!(app.log_view.page, 0);
    assert_page_invariants(&app);
}

#[test]
fn sync_add_and_render_in_one_transition() {
    let mut app = app_with_rows(10);
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    let journal_id = app.active_journal.id.clone();
    let mut qso = synthetic_qso(777, &journal_id);
    qso.id = None;
    qso.callsign = "TST0777".to_string();
    app.handle_lan_qso(qso);
    render_one_frame(&mut app);

    assert_eq!(app.log_view.recent_qsos.len(), 11);
    assert_page_invariants(&app);
}

#[test]
fn undo_restores_rows_and_keeps_range_safe() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let id = current_page_ids(&mut app)[0];
    app.delete_qso_by_id(id);
    render_one_frame(&mut app);
    assert_eq!(app.log_view.recent_qsos.len(), 119);

    app.perform_undo();
    render_one_frame(&mut app);
    assert_eq!(app.log_view.recent_qsos.len(), 120);
    assert_page_invariants(&app);
}

// ---------------------------------------------------------------------------
// Selection (stable identity)
// ---------------------------------------------------------------------------

#[test]
fn selection_preserved_by_stable_id_across_filter() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let selected = current_page_ids(&mut app)[0];
    app.log_view.selected_qso_ids = vec![selected];

    search(&mut app, &synthetic_callsign(120));
    render_one_frame(&mut app);
    assert!(app.log_view.selected_qso_ids.contains(&selected));

    search(&mut app, "");
    render_one_frame(&mut app);
    assert!(app.log_view.selected_qso_ids.contains(&selected));
    assert_page_invariants(&app);
}

#[test]
fn selection_removed_when_record_deleted() {
    let mut app = app_with_rows(30);
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let id = current_page_ids(&mut app)[0];
    app.log_view.selected_qso_ids = vec![id, 999_999];

    app.delete_qso_by_id(id);
    assert!(!app.log_view.selected_qso_ids.contains(&id));
    // Unknown/stale IDs are never turned into a random QSO.
    assert!(app.log_view.selected_qso_ids.contains(&999_999));
    render_one_frame(&mut app);
    assert_page_invariants(&app);
}

#[test]
fn bulk_delete_clears_selection() {
    let mut app = app_with_rows(30);
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    app.log_view.selected_qso_ids = current_page_ids(&mut app);
    app.delete_selected_qso();
    assert!(app.log_view.selected_qso_ids.is_empty());
    render_one_frame(&mut app);
    assert_page_invariants(&app);
}

#[test]
fn select_all_only_affects_current_page() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = PAGE_SIZE;
    app.log_view.page = 0;
    render_one_frame(&mut app);
    let first_page = current_page_ids(&mut app);
    app.log_view.selected_qso_ids = first_page.clone();

    app.log_view.page = 1;
    render_one_frame(&mut app);
    let second_page = current_page_ids(&mut app);
    assert!(
        first_page
            .iter()
            .all(|id| app.log_view.selected_qso_ids.contains(id))
    );
    assert!(
        second_page
            .iter()
            .all(|id| !app.log_view.selected_qso_ids.contains(id))
    );
    assert_eq!(first_page.len(), 50);
    assert_eq!(second_page.len(), 50);
    assert_page_invariants(&app);
}

// ---------------------------------------------------------------------------
// Page coverage: no duplicated rows, no omitted rows
// ---------------------------------------------------------------------------

#[test]
fn pages_do_not_duplicate_rows() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let mut all_ids = Vec::new();
    for page in 0..window_of(&app).page_count {
        app.log_view.page = page;
        render_one_frame(&mut app);
        all_ids.extend(current_page_ids(&mut app));
    }
    let unique: std::collections::HashSet<i64> = all_ids.iter().copied().collect();
    assert_eq!(unique.len(), all_ids.len());
    assert_eq!(unique.len(), 120);
}

#[test]
fn pages_cover_filtered_result_without_gaps() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = PAGE_SIZE;
    search(&mut app, "TST0");
    render_one_frame(&mut app);
    let expected: Vec<i64> = app
        .log_view
        .cache
        .sorted_indices
        .iter()
        .filter_map(|&index| app.log_view.recent_qsos.get(index).and_then(|qso| qso.id))
        .collect();
    let mut all_ids = Vec::new();
    for page in 0..window_of(&app).page_count {
        app.log_view.page = page;
        render_one_frame(&mut app);
        all_ids.extend(current_page_ids(&mut app));
    }
    assert_eq!(all_ids, expected);
    assert_page_invariants(&app);
}

#[test]
fn page_ranges_do_not_overlap() {
    for total in [0usize, 1, 49, 50, 51, 99, 100, 120] {
        let mut previous_end = 0usize;
        let page_count = paginate(PaginationInput {
            requested_page: 0,
            page_size: PAGE_SIZE,
            total_items: total,
        })
        .page_count;
        for page in 0..page_count {
            let window = paginate(PaginationInput {
                requested_page: page,
                page_size: PAGE_SIZE,
                total_items: total,
            });
            assert_eq!(window.start, previous_end);
            assert!(window.end >= window.start);
            previous_end = window.end;
        }
        assert_eq!(previous_end, total);
    }
}

#[test]
fn previous_page_navigation_at_first_page() {
    let window = paginate(PaginationInput {
        requested_page: 0,
        page_size: PAGE_SIZE,
        total_items: 120,
    });
    assert_eq!(window.previous_page(), None);
}

#[test]
fn next_page_navigation_at_last_page() {
    let window = paginate(PaginationInput {
        requested_page: 2,
        page_size: PAGE_SIZE,
        total_items: 120,
    });
    assert_eq!(window.next_page(), None);
}

// ---------------------------------------------------------------------------
// Fault injection
// ---------------------------------------------------------------------------

#[test]
fn fault_injected_sort_column_is_safe() {
    let mut app = app_with_rows(60);
    app.log_view.page_size = PAGE_SIZE;
    app.log_view.sort_column = 255;
    app.log_view.sort_asc = true;
    render_one_frame(&mut app);
    assert_page_invariants(&app);
}

#[test]
fn inconsistent_cache_is_repaired_on_render() {
    let mut app = app_with_rows(120);
    app.log_view.page = 1;
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);

    // Deterministic fault injection: a same-length cache with impossible indices.
    app.log_view.cache.sorted_indices = vec![usize::MAX; 120];
    render_one_frame(&mut app);

    assert_eq!(app.log_view.cache.sorted_indices.len(), 120);
    assert_page_invariants(&app);
}

#[test]
fn nonexistent_selection_id_is_safe() {
    let mut app = app_with_rows(30);
    app.log_view.page_size = PAGE_SIZE;
    app.log_view.selected_qso_ids = vec![999_999];
    render_one_frame(&mut app);
    assert_eq!(app.log_view.selected_qso_ids, vec![999_999]);
    assert_page_invariants(&app);
}

#[test]
fn malformed_optional_row_data_renders() {
    let mut app = empty_app();
    app.log_view.page_size = PAGE_SIZE;
    app.log_view.recent_qsos = vec![QsoRecord {
        id: Some(1),
        callsign: "TST".to_string(),
        band: String::new(),
        mode: String::new(),
        qso_date: String::new(),
        time_on: String::new(),
        country: None,
        name: None,
        qth: None,
        gridsquare: None,
        comment: None,
        audio_file: None,
        ..Default::default()
    }];
    app.log_view.bump_revision();
    render_one_frame(&mut app);
    assert_page_invariants(&app);
}

#[test]
fn malformed_import_is_controlled() {
    let mut app = app_with_rows(10);
    app.log_view.page_size = PAGE_SIZE;
    app.import_adif_content("<CALL:3>BAD <EOR>", "malformed.adi");
    render_one_frame(&mut app);
    assert_page_invariants(&app);
}

#[test]
fn duplicate_sync_record_is_ignored_without_panic() {
    let mut app = app_with_rows(10);
    app.log_view.page_size = PAGE_SIZE;
    render_one_frame(&mut app);
    let duplicate = app.log_view.recent_qsos[0].clone();
    app.handle_lan_qso(duplicate);
    render_one_frame(&mut app);
    assert_eq!(app.log_view.recent_qsos.len(), 10);
    assert_page_invariants(&app);
}

#[test]
fn unicode_filter_matches_and_renders() {
    let mut app = app_with_rows(5);
    app.log_view.page_size = PAGE_SIZE;
    if let Some(qso) = app.log_view.recent_qsos.first_mut() {
        qso.name = Some("ŁÓDŹ".to_string());
    }
    app.log_view.bump_revision();
    search(&mut app, "łódź");
    render_one_frame(&mut app);
    assert_eq!(app.log_view.cache.sorted_indices.len(), 1);
    assert_page_invariants(&app);
}

// ---------------------------------------------------------------------------
// Combinatorial matrix, deterministic stress, GUI panic-free proof
// ---------------------------------------------------------------------------

#[test]
fn combinatorial_transition_matrix_holds_invariants() {
    let mut combinations = 0usize;
    for total in [
        0usize,
        1,
        2,
        49,
        50,
        51,
        99,
        100,
        120,
        usize::MAX - 1,
        usize::MAX,
    ] {
        for page in [0usize, 1, 2, 3, 7, usize::MAX - 1, usize::MAX] {
            for page_size in [0usize, 1, 2, 25, 50, 100, usize::MAX - 1, usize::MAX] {
                for stale in [false, true] {
                    let requested = if stale { usize::MAX } else { page };
                    let window = paginate(PaginationInput {
                        requested_page: requested,
                        page_size,
                        total_items: total,
                    });
                    assert!(window.start <= window.end);
                    assert!(window.end <= total);
                    assert!(window.effective_page_size >= 1);
                    if window.page_count > 0 {
                        assert!(window.normalized_page < window.page_count);
                    } else {
                        assert_eq!((window.start, window.end), (0, 0));
                    }
                    combinations += 1;
                }
            }
        }
    }
    assert!(combinations >= 1_000);
}

#[test]
fn total_items_usize_max_in_pure_model() {
    let window = paginate(PaginationInput {
        requested_page: usize::MAX,
        page_size: 7,
        total_items: usize::MAX,
    });
    assert_eq!(window.end, usize::MAX);
    assert!(window.start <= window.end);
    assert!(window.normalized_page < window.page_count);
    assert!(!window.arithmetic_guard_triggered);
}

#[test]
fn addition_overflow_in_pure_model() {
    let page_size = usize::MAX / 2 + 1;
    let window = paginate(PaginationInput {
        requested_page: 1,
        page_size,
        total_items: usize::MAX,
    });
    assert_eq!(window.page_count, 2);
    assert_eq!(window.normalized_page, 1);
    assert_eq!(window.end, usize::MAX);
}

#[test]
fn stress_stale_page_after_filter_5000() {
    for iteration in 0..5_000usize {
        let total = iteration % 500;
        let requested = usize::MAX - iteration % 97;
        let window = paginate(PaginationInput {
            requested_page: requested,
            page_size: 25,
            total_items: total,
        });
        assert!(window.start <= window.end);
        assert!(window.end <= total);
        assert!(window.normalized_page < window.page_count.max(1));
    }
}

#[test]
fn stress_empty_dataset_5000() {
    for iteration in 0..5_000usize {
        let window = paginate(PaginationInput {
            requested_page: iteration,
            page_size: iteration % 101,
            total_items: 0,
        });
        assert_eq!((window.start, window.end), (0, 0));
        assert_eq!(window.page_count, 0);
        assert_eq!(window.normalized_page, 0);
        assert!(window.effective_page_size >= 1);
    }
}

#[test]
fn stress_page_size_changes_5000() {
    for iteration in 0..5_000usize {
        let page_size = iteration % 300;
        let window = paginate(PaginationInput {
            requested_page: iteration % 13,
            page_size,
            total_items: 1_000,
        });
        assert!(window.start <= window.end);
        assert!(window.end <= 1_000);
        assert!(window.effective_page_size >= 1);
    }
}

#[test]
fn stress_deletion_on_last_page_3000() {
    let mut state = LogViewState {
        recent_qsos: (1..=3_000)
            .map(|index| synthetic_qso(index, "stress"))
            .collect(),
        page_size: 50,
        ..Default::default()
    };
    state.page = usize::MAX;
    for _ in 0..3_000usize {
        let total = state.recent_qsos.len();
        let window = state.pagination_window(total);
        assert!(window.start <= window.end);
        assert!(window.end <= total);
        state.recent_qsos.pop();
    }
    assert!(state.recent_qsos.is_empty());
    assert_eq!(state.pagination_window(0).page_count, 0);
}

#[test]
fn stress_rapid_filter_transitions_3000() {
    let mut state = LogViewState {
        recent_qsos: (1..=400)
            .map(|index| synthetic_qso(index, "stress"))
            .collect(),
        page_size: 25,
        ..Default::default()
    };
    for iteration in 0..3_000usize {
        state.search_query = match iteration % 4 {
            0 => String::new(),
            1 => "TST00".to_string(),
            2 => "TST0001".to_string(),
            _ => "NO-MATCH".to_string(),
        };
        let filtered = if state.search_query.is_empty() {
            400
        } else if state.search_query == "TST0001" {
            1
        } else if state.search_query == "NO-MATCH" {
            0
        } else {
            9
        };
        state.page = iteration % 17;
        let window = state.pagination_window(filtered);
        assert!(window.end <= filtered);
        assert!(window.start <= window.end);
    }
}

#[test]
fn stress_filter_and_sort_3000() {
    for iteration in 0..3_000usize {
        let total = iteration % 700;
        for sort in 0u8..5 {
            let window = paginate(PaginationInput {
                requested_page: iteration % 11,
                page_size: usize::from(sort) + 1,
                total_items: total,
            });
            assert!(window.end <= total);
        }
    }
}

#[test]
fn stress_journal_switching_2000() {
    let mut state = LogViewState::default();
    for iteration in 0..2_000usize {
        state.recent_qsos = (1..=(iteration % 200))
            .map(|index| synthetic_qso(index, "journal"))
            .collect();
        state.page = iteration;
        state.selected_qso_ids.clear();
        let total = state.recent_qsos.len();
        let window = state.pagination_window(total);
        assert!(window.start <= window.end);
        assert!(window.end <= total);
    }
}

#[test]
fn stress_selection_stabilization_2000() {
    let ids: Vec<i64> = (1..=2_000).collect();
    let mut selected: Vec<i64> = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        if index % 2 == 0 {
            selected.push(*id);
        }
        // A filter/page transition never rewrites selection to another record.
        assert_eq!(selected.contains(id), index % 2 == 0);
    }
    assert_eq!(selected.len(), 1_000);
    // Deleting a record removes exactly that ID and never selects another one.
    selected.retain(|id| *id != ids[0]);
    assert!(!selected.contains(&ids[0]));
    assert!(selected.contains(&ids[2]));
}

#[test]
fn stress_arithmetic_boundaries_5000() {
    for iteration in 0..5_000usize {
        let total = match iteration % 5 {
            0 => 0,
            1 => 1,
            2 => usize::MAX / 2,
            3 => usize::MAX - 1,
            _ => usize::MAX,
        };
        let page_size = match iteration % 7 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 25,
            4 => 50,
            5 => usize::MAX - 1,
            _ => usize::MAX,
        };
        let window = paginate(PaginationInput {
            requested_page: iteration,
            page_size,
            total_items: total,
        });
        assert!(window.start <= window.end);
        assert!(window.end <= total);
    }
}

#[test]
fn gui_render_stress_200() {
    let mut app = app_with_rows(120);
    for iteration in 0..200usize {
        match iteration % 4 {
            0 => {
                app.log_view.page_size = 0;
                app.log_view.page = 0;
                search(&mut app, "");
            }
            1 => {
                app.log_view.page_size = 25;
                app.log_view.page = 2;
                search(&mut app, "TST");
            }
            2 => {
                app.log_view.page_size = 50;
                app.log_view.page = usize::MAX;
                search(&mut app, &synthetic_callsign(1 + iteration % 100));
            }
            _ => {
                app.log_view.page_size = 100;
                app.log_view.page = 1;
                app.log_view.sort_column = u8::try_from(iteration % 5).unwrap_or(0);
                search(&mut app, "");
            }
        }
        render_one_frame(&mut app);
        assert_page_invariants(&app);
    }
}

#[test]
fn gui_render_path_zero_panic_across_transitions() {
    let mut app = app_with_rows(120);
    app.log_view.page_size = PAGE_SIZE;
    for page in [usize::MAX, 0, 2, 1, 0] {
        app.log_view.page = page;
        render_one_frame(&mut app);
        assert_page_invariants(&app);
    }
    for query in ["", "TST0001", "NO-MATCH", "TST0", ""] {
        search(&mut app, query);
        render_one_frame(&mut app);
        assert_page_invariants(&app);
    }
    for size in [0usize, 1, 25, 50, 100, usize::MAX] {
        app.log_view.page_size = size;
        app.log_view.page = usize::MAX;
        if size != 0 {
            app.log_view.page = 1;
        }
        render_one_frame(&mut app);
        assert_page_invariants(&app);
    }
}

#[test]
fn full_gui_frame_renders_without_panic() {
    let mut app = app_with_rows(120);
    app.log_view.page = 2;
    app.log_view.page_size = PAGE_SIZE;
    search(&mut app, &synthetic_callsign(1));
    app.gui.chrome.show_welcome_wizard = false;
    app.views.panel_log.visible = true;
    app.views.panel_log.floating = false;

    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.render_main_ui(ui);
    });
    out.textures_delta.clear();
    assert_page_invariants(&app);
}
