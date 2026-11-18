// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Testy integracyjne dla bramki `STAGE-P1-F21-SOFTWARE` (`F-21`):
//! - pełny, bezstratny eksport Cabrillo dla dziennika > 10 000 QSO (brak ucięcia najstarszych QSO),
//! - wyliczanie `CLAIMED-SCORE` z tego samego pełnego, kanonicznie posortowanego zbioru,
//! - ścisła separacja między dziennikami (`journal_id`) przy eksporcie i punktacji.

use splogbook::core::cabrillo::{
    CabrilloInput, CabrilloOperatorCategory, CabrilloTransmitterCategory, build,
    canonical_qso_sort_key, format_and_validate_cabrillo_date, format_and_validate_cabrillo_time,
};
use splogbook::core::contest_rules::ContestError;
use splogbook::core::contest_stats::{Scoreboard, compute_scoreboard};
use splogbook::core::database::{Journal, LogDatabase};
use splogbook::core::qso::QsoRecord;

fn make_journal(id: &str, name: &str, is_default: bool) -> Journal {
    Journal {
        id: id.to_string(),
        name: name.to_string(),
        station_callsign: "SP6INA".to_string(),
        operator: "Mariusz".to_string(),
        my_gridsquare: "JO81".to_string(),
        my_pga: String::new(),
        description: "synthetic F-21 contest fixture".to_string(),
        is_default,
    }
}

fn synthetic_contest_qso(
    journal_id: &str,
    index: usize,
    callsign: &str,
    band: &str,
    dxcc: u32,
    cqz: u32,
    continent: &str,
) -> QsoRecord {
    // Rozkładamy 10 500+ łączności na 2 dni zawodów (20260404 i 20260405),
    // z deterministycznym czasem HHMMSS rosnącym co 8 sekund.
    let total_secs = (index as u64) * 8;
    let day_offset = total_secs / 86_400;
    let sec_in_day = total_secs % 86_400;
    let hh = sec_in_day / 3_600;
    let mm = (sec_in_day % 3_600) / 60;
    let ss = sec_in_day % 60;

    let mut q = QsoRecord::new(callsign, band, "CW");
    q.journal_id = Some(journal_id.to_string());
    q.qso_date = format!("202604{:02}", 4 + day_offset);
    q.time_on = format!("{hh:02}{mm:02}{ss:02}");
    q.freq = Some(14.025);
    q.rst_sent = "599".to_string();
    q.rst_rcvd = "599".to_string();
    q.stx = Some((index + 1) as u32);
    q.srx = Some(1);
    q.dxcc = Some(dxcc);
    q.cqz = Some(cqz);
    q.continent = Some(continent.to_string());
    q
}

#[test]
fn p1_f21_cabrillo_export_over_10000_qsos_has_no_truncation_and_scores_full_set() {
    let db = LogDatabase::open_in_memory().expect("in-memory db");
    let journal_a = "CONTEST_A";
    db.create_journal(&make_journal(journal_a, "Contest A", true))
        .expect("create CONTEST_A");

    // Budujemy log 10 500 QSO w CONTEST_A:
    // - Pierwsze 500 QSO (indeksy 0..500, najstarsze chronologicznie — te, które dawniej
    //   odpadały przez `ORDER BY ... DESC LIMIT 10000`):
    //   stacje DX spoza Europy (NA, по 3 pkt w SP DX Contest) oraz 5 unikalnych mnożników
    //   DXCC na paśmie 15m (DXCC 291..=295), które NIE występują nigdzie w pozostałych 10 000 QSO.
    // - Kolejne 10 000 QSO (indeksy 500..10500):
    //   stacje europejskie (DL, DXCC 230, EU, po 1 pkt w SP DX Contest) na paśmie 20m (1 mnożnik: 20m:D230).
    let mut records = Vec::with_capacity(10_500);
    for i in 0..500usize {
        let call = format!("K{}DX{}", (i % 9) + 1, i);
        let dxcc = 291 + (i % 5) as u32; // 5 unikalnych podmiotów DXCC na 15m: 291, 292, 293, 294, 295
        records.push(synthetic_contest_qso(
            journal_a, i, &call, "15m", dxcc, 5, "NA",
        ));
    }
    for i in 500..10_500usize {
        let call = format!("DL{}EU{}", (i % 9) + 1, i);
        records.push(synthetic_contest_qso(
            journal_a, i, &call, "20m", 230, 14, "EU",
        ));
    }

    let inserted = db.batch_insert_qsos(&records).expect("batch insert 10500");
    assert_eq!(inserted, 10_500);
    assert_eq!(db.count_qsos_for_journal(journal_a).expect("count"), 10_500);

    // Pobieramy pełny zbiór ścieżką produkcyjną eksportu Cabrillo (`get_all_qsos_for_journal`)
    let qsos = db
        .get_all_qsos_for_journal(journal_a)
        .expect("get_all_qsos_for_journal");
    assert_eq!(
        qsos.len(),
        10_500,
        "Full journal query must return all 10 500 QSOs without silent 10 000 cap"
    );

    // Weryfikujemy kanoniczny porządek rosnący
    assert!(
        qsos.windows(2)
            .all(|w| canonical_qso_sort_key(&w[0]) <= canonical_qso_sort_key(&w[1])),
        "Returned QSOs must be in canonical chronological ascending order"
    );
    assert_eq!(qsos.first().unwrap().callsign, "K1DX0");
    assert_eq!(qsos.last().unwrap().callsign, "DL6EU10499");

    // Punktacja SP DX Contest dla stacji polskiej (my_dxcc = 269, my_cqzone = 15):
    // - Pierwsze 500 QSO (NA, 15m): 500 * 3 pkt = 1 500 pkt, 5 mnożników (15m:D291..295)
    // - Kolejne 10 000 QSO (EU, 20m): 10 000 * 1 pkt = 10 000 pkt, 1 mnożnik (20m:D230)
    // Razem pełny log: 11 500 pkt * 6 mnożników = 69 000.
    // (Gdyby zbiór był ucięty do 10 000 najnowszych QSO, wynik wynosiłby tylko 10 000 * 1 = 10 000!).
    let scoreboard = compute_scoreboard("SP DX Contest", &[], &qsos, 269, 15);
    assert_eq!(scoreboard.qsos, 10_500);
    assert_eq!(scoreboard.points, 11_500);
    assert_eq!(scoreboard.mults, 6);
    assert_eq!(scoreboard.total, 69_000);

    let input = CabrilloInput {
        contest_name: "SP DX Contest",
        callsign: "SP6INA",
        operator: "Mariusz",
        city: "Wroclaw",
        country: "Poland",
        claimed_score: scoreboard.total,
        qsos: &qsos,
        transmitter_category: Some(CabrilloTransmitterCategory::One),
        operator_category: Some(CabrilloOperatorCategory::SingleOp),
    };
    let cabrillo_text = build(&input).expect("build cabrillo for 10 500 QSOs");

    assert!(
        cabrillo_text.contains("CLAIMED-SCORE: 69000\n"),
        "Cabrillo header must carry CLAIMED-SCORE from the full 10 500-QSO set"
    );

    let qso_lines: Vec<&str> = cabrillo_text
        .lines()
        .filter(|line| line.starts_with("QSO:"))
        .collect();
    assert_eq!(
        qso_lines.len(),
        10_500,
        "Cabrillo output must contain all 10 500 QSO lines"
    );
    assert!(
        qso_lines.first().unwrap().contains("K1DX0"),
        "First Cabrillo QSO line must be the oldest QSO #1 (K1DX0), got: {}",
        qso_lines.first().unwrap()
    );
    assert!(
        qso_lines.last().unwrap().contains("DL6EU10499"),
        "Last Cabrillo QSO line must be QSO #10500 (DL6EU10499), got: {}",
        qso_lines.last().unwrap()
    );
}

#[test]
fn p1_f21_cabrillo_export_and_claimed_score_strictly_isolate_journals() {
    let db = LogDatabase::open_in_memory().expect("in-memory db");
    let journal_a = "JOURNAL_CONTEST_A";
    let journal_b = "JOURNAL_CONTEST_B";
    db.create_journal(&make_journal(journal_a, "Contest A", true))
        .expect("create JOURNAL_CONTEST_A");
    db.create_journal(&make_journal(journal_b, "Contest B", false))
        .expect("create JOURNAL_CONTEST_B");

    // Wstawiamy łączności do dwóch różnych dzienników w tej samej bazie SQLite:
    // - JOURNAL_CONTEST_A: 3 łączności z Europą na 20m (DXCC 230, 231, 232 -> 3 pkt * 3 mults = 9)
    // - JOURNAL_CONTEST_B: 2 łączności z Japonią na 15m (DXCC 339, AS -> 3 pkt * 1 mult = 3)
    let qsos_a = vec![
        synthetic_contest_qso(journal_a, 0, "DL1AAA", "20m", 230, 14, "EU"),
        synthetic_contest_qso(journal_a, 1, "G3BBB", "20m", 231, 14, "EU"),
        synthetic_contest_qso(journal_a, 2, "F5CCC", "20m", 232, 14, "EU"),
    ];
    let qsos_b = vec![
        synthetic_contest_qso(journal_b, 0, "JA1XYZ", "15m", 339, 25, "AS"),
        synthetic_contest_qso(journal_b, 1, "JA2QRS", "15m", 339, 25, "AS"),
    ];

    db.batch_insert_qsos(&qsos_a).expect("insert A");
    db.batch_insert_qsos(&qsos_b).expect("insert B");

    let fetched_a = db.get_all_qsos_for_journal(journal_a).expect("fetch A");
    let fetched_b = db.get_all_qsos_for_journal(journal_b).expect("fetch B");

    assert_eq!(fetched_a.len(), 3);
    assert_eq!(fetched_b.len(), 2);
    assert!(
        fetched_a
            .iter()
            .all(|q| q.journal_id.as_deref() == Some(journal_a))
    );
    assert!(
        fetched_b
            .iter()
            .all(|q| q.journal_id.as_deref() == Some(journal_b))
    );

    let sb_a = compute_scoreboard("SP DX Contest", &[], &fetched_a, 269, 15);
    assert_eq!(sb_a.qsos, 3);
    assert_eq!(sb_a.points, 3);
    assert_eq!(sb_a.mults, 3);
    assert_eq!(sb_a.total, 9);

    let sb_b = compute_scoreboard("SP DX Contest", &[], &fetched_b, 269, 15);
    assert_eq!(sb_b.qsos, 2);
    assert_eq!(sb_b.points, 6);
    assert_eq!(sb_b.mults, 1);
    assert_eq!(sb_b.total, 6);

    let cab_a = build(&CabrilloInput {
        contest_name: "SP DX Contest",
        callsign: "SP6INA",
        operator: "Mariusz",
        city: "Wroclaw",
        country: "Poland",
        claimed_score: sb_a.total,
        qsos: &fetched_a,
        transmitter_category: Some(CabrilloTransmitterCategory::One),
        operator_category: Some(CabrilloOperatorCategory::SingleOp),
    })
    .expect("build A");

    assert!(cab_a.contains("CLAIMED-SCORE: 9\n"));
    assert!(cab_a.contains("DL1AAA") && cab_a.contains("G3BBB") && cab_a.contains("F5CCC"));
    assert!(
        !cab_a.contains("JA1XYZ") && !cab_a.contains("JA2QRS"),
        "Journal A Cabrillo export must never leak records from Journal B"
    );

    let cab_b = build(&CabrilloInput {
        contest_name: "SP DX Contest",
        callsign: "SP6INA",
        operator: "Mariusz",
        city: "Wroclaw",
        country: "Poland",
        claimed_score: sb_b.total,
        qsos: &fetched_b,
        transmitter_category: Some(CabrilloTransmitterCategory::One),
        operator_category: Some(CabrilloOperatorCategory::SingleOp),
    })
    .expect("build B");

    assert!(cab_b.contains("CLAIMED-SCORE: 6\n"));
    assert!(cab_b.contains("JA1XYZ") && cab_b.contains("JA2QRS"));
    assert!(
        !cab_b.contains("DL1AAA") && !cab_b.contains("G3BBB") && !cab_b.contains("F5CCC"),
        "Journal B Cabrillo export must never leak records from Journal A"
    );
}

#[test]
fn p1_f21_cabrillo_date_and_time_validation_is_fail_closed_and_never_uses_wall_clock() {
    assert_eq!(
        format_and_validate_cabrillo_date("20260404").unwrap(),
        "2026-04-04"
    );
    assert_eq!(
        format_and_validate_cabrillo_date(" 2026-04-04 ").unwrap(),
        "2026-04-04"
    );

    for invalid_date in [
        "",
        "   ",
        "20261301",
        "2026-02-30",
        "2026/04/04",
        "ABCDEFGH",
    ] {
        assert!(
            matches!(
                format_and_validate_cabrillo_date(invalid_date),
                Err(ContestError::InvalidQsoTimestamp { .. })
            ),
            "invalid date '{invalid_date}' must be rejected fail-closed"
        );
    }

    assert_eq!(format_and_validate_cabrillo_time("1234").unwrap(), "1234");
    assert_eq!(format_and_validate_cabrillo_time("123456").unwrap(), "1234");
    assert_eq!(format_and_validate_cabrillo_time("12:34").unwrap(), "1234");
    assert_eq!(
        format_and_validate_cabrillo_time("12:34:56").unwrap(),
        "1234"
    );
    assert_eq!(format_and_validate_cabrillo_time("000000").unwrap(), "0000");
    assert_eq!(
        format_and_validate_cabrillo_time("23:59:59").unwrap(),
        "2359"
    );

    for invalid_time in [
        "", "   ", "123", "12345", "2400", "2360", "235960", "24:00", "12:60:00", "12:34:60",
        "ABCD",
    ] {
        assert!(
            matches!(
                format_and_validate_cabrillo_time(invalid_time),
                Err(ContestError::InvalidQsoTimestamp { .. })
            ),
            "invalid time '{invalid_time}' must be rejected fail-closed"
        );
    }

    let mut bad_date_qso = synthetic_contest_qso("J", 0, "W1AW", "20m", 291, 5, "NA");
    bad_date_qso.qso_date.clear();
    let res_date = build(&CabrilloInput {
        contest_name: "CQ WW CW",
        callsign: "SP6INA",
        operator: "Mariusz",
        city: "Wroclaw",
        country: "Poland",
        claimed_score: 3,
        qsos: &[bad_date_qso],
        transmitter_category: None,
        operator_category: None,
    });
    assert!(matches!(
        res_date,
        Err(ContestError::InvalidQsoTimestamp { ref callsign, .. }) if callsign == "W1AW"
    ));

    let mut bad_time_qso = synthetic_contest_qso("J", 0, "DL1XYZ", "20m", 230, 14, "EU");
    bad_time_qso.time_on = "9999".to_string();
    let res_time = build(&CabrilloInput {
        contest_name: "CQ WW CW",
        callsign: "SP6INA",
        operator: "Mariusz",
        city: "Wroclaw",
        country: "Poland",
        claimed_score: 1,
        qsos: &[bad_time_qso],
        transmitter_category: None,
        operator_category: None,
    });
    assert!(matches!(
        res_time,
        Err(ContestError::InvalidQsoTimestamp { ref callsign, .. }) if callsign == "DL1XYZ"
    ));
}

#[test]
fn p1_f21_same_minute_mixed_formats_and_scoreboard_asc_desc_invariance() {
    let mut q_sec30 = synthetic_contest_qso("J", 0, "DL3SEC", "20m", 230, 14, "EU");
    q_sec30.id = Some(10);
    q_sec30.qso_date = "2026-04-04".to_string();
    q_sec30.time_on = "120030".to_string();
    q_sec30.stx = None;

    let mut q_sec05_colon = synthetic_contest_qso("J", 0, "DL2COL", "20m", 230, 14, "EU");
    q_sec05_colon.id = Some(20);
    q_sec05_colon.qso_date = "20260404".to_string();
    q_sec05_colon.time_on = "12:00:05".to_string();
    q_sec05_colon.stx = None;

    let mut q_min_id2 = synthetic_contest_qso("J", 0, "DL1ID2", "20m", 230, 14, "EU");
    q_min_id2.id = Some(2);
    q_min_id2.qso_date = "20260404".to_string();
    q_min_id2.time_on = "120000".to_string();
    q_min_id2.stx = None;

    let mut q_min_id1 = synthetic_contest_qso("J", 0, "DL0ID1", "20m", 230, 14, "EU");
    q_min_id1.id = Some(1);
    q_min_id1.qso_date = "20260404".to_string();
    q_min_id1.time_on = "1200".to_string();
    q_min_id1.stx = None;

    let qsos = vec![q_sec30, q_sec05_colon, q_min_id2, q_min_id1];
    let text = build(&CabrilloInput {
        contest_name: "SP DX Contest",
        callsign: "SP6INA",
        operator: "Mariusz",
        city: "Wroclaw",
        country: "Poland",
        claimed_score: 4,
        qsos: &qsos,
        transmitter_category: None,
        operator_category: None,
    })
    .expect("valid cabrillo");

    let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("QSO:")).collect();
    assert_eq!(lines.len(), 4);
    assert!(
        lines[0].contains("2026-04-04 1200")
            && lines[0].contains("001")
            && lines[0].contains("DL0ID1")
    );
    assert!(
        lines[1].contains("2026-04-04 1200")
            && lines[1].contains("002")
            && lines[1].contains("DL1ID2")
    );
    assert!(
        lines[2].contains("2026-04-04 1200")
            && lines[2].contains("003")
            && lines[2].contains("DL2COL")
    );
    assert!(
        lines[3].contains("2026-04-04 1200")
            && lines[3].contains("004")
            && lines[3].contains("DL3SEC")
    );

    // Niezależność compute_scoreboard od kierunku wejścia (ASC vs DESC) przy duplikatach
    let mut first = synthetic_contest_qso("J", 0, "SP1AA", "20m", 269, 15, "EU");
    first.id = Some(1);
    first.qso_date = "20260404".to_string();
    first.time_on = "120000".to_string();
    first.srx_string = Some("R".to_string());

    let mut dup_later = synthetic_contest_qso("J", 1, "SP1AA", "20m", 269, 15, "EU");
    dup_later.id = Some(2);
    dup_later.qso_date = "20260404".to_string();
    dup_later.time_on = "130000".to_string();
    dup_later.srx_string = None;

    let mut other = synthetic_contest_qso("J", 2, "SP2BB", "40m", 269, 15, "EU");
    other.id = Some(3);
    other.qso_date = "20260404".to_string();
    other.time_on = "123000".to_string();
    other.srx_string = Some("D".to_string());

    let asc = vec![first.clone(), other.clone(), dup_later.clone()];
    let desc = vec![dup_later, other, first];

    let sb_asc = compute_scoreboard("SP DX Contest", &[], &asc, 230, 14);
    let sb_desc = compute_scoreboard("SP DX Contest", &[], &desc, 230, 14);
    assert_eq!(
        sb_asc,
        Scoreboard {
            qsos: 3,
            points: 6,
            mults: 2,
            total: 12,
        }
    );
    assert_eq!(sb_asc, sb_desc);
}
