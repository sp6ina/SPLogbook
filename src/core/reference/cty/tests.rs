// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Country-file parser tests. Every record is synthetic: entity names are
//! invented and prefixes use the `Q` block, which is not allocated to any
//! country, so no fixture can be mistaken for real reference data.

use super::*;

/// Synthetic `cty.dat` exercising every documented construct.
const SYNTH_DAT: &str = "\
Alpha Land:               14:  27:  EU:   50.00:   -20.00:    -1.0:  QA:
    QA,QA2,QA25(15)[28],=QA1EXACT(16)[29]{AS}<10.5/-30.25>~5.5~;
Beta Isles:               3:   6:   NA:   20.00:   100.00:     8.0:  QB:
    QB,QB9(4)[7],
    =QB1EX/P,QBA{SA};
Alpha South:              33:  37:  AF:  -10.00:   -15.00:    -2.0:  *QA9:
    QA9,=QA9WAE;
";

/// Companion `cty.csv` for [`SYNTH_DAT`].
const SYNTH_CSV: &str = "\
QA,Alpha Land,901,EU,14,27,50.00,-20.00,-1.0,QA QA2 QA25(15)[28] =QA1EXACT;
QB,Beta Isles,902,NA,3,6,20.00,100.00,8.0,QB QB9(4)[7] =QB1EX/P QBA{SA};
*QA9,Alpha South,901,AF,33,37,-10.00,-15.00,-2.0,QA9 =QA9WAE;
";

fn synth() -> CtyData {
    let mut d = CtyData::parse(SYNTH_DAT).expect("synthetic cty.dat parses");
    let (rows, issues) = parse_cty_csv(SYNTH_CSV);
    assert!(issues.is_empty(), "{issues:?}");
    let report = d.apply_entity_codes(&rows);
    assert_eq!(report.matched, 3);
    d
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// ---------------------------------------------------------------- Level A ---

#[test]
fn parses_entities_and_header_fields() {
    let d = synth();
    assert!(d.issues.is_empty(), "{:?}", d.issues);
    assert_eq!(d.entities().len(), 3);
    let a = &d.entities()[0];
    assert_eq!(a.name, "Alpha Land");
    assert_eq!((a.cq_zone, a.itu_zone), (14, 27));
    assert_eq!(a.continent, "EU");
    assert!(approx(a.latitude, 50.0));
    // +W in the file becomes -W / +E internally.
    assert!(approx(a.longitude, 20.0));
    assert!(approx(a.utc_offset, -1.0));
    assert_eq!(a.primary_prefix, "QA");
    assert!(!a.wae_only);
    let b = &d.entities()[1];
    assert!(approx(b.longitude, -100.0));
}

#[test]
fn plain_prefix_match() {
    let m = synth().resolve_prefix("QB5XYZ").unwrap();
    assert_eq!(m.entity_name, "Beta Isles");
    assert_eq!(m.matched, "QB");
    assert!(!m.exact);
    assert_eq!((m.cq_zone, m.itu_zone), (3, 6));
}

#[test]
fn longest_prefix_wins() {
    let d = synth();
    assert_eq!(d.resolve_prefix("QA2ZZ").unwrap().matched, "QA2");
    let m = d.resolve_prefix("QA25Z").unwrap();
    assert_eq!(m.matched, "QA25");
    assert_eq!((m.cq_zone, m.itu_zone), (15, 28));
    assert_eq!(d.resolve_prefix("QA3ZZ").unwrap().matched, "QA");
}

#[test]
fn exact_call_requires_exact_match() {
    let d = synth();
    let m = d.resolve_exact("QA1EXACT").unwrap();
    assert!(m.exact);
    assert_eq!(m.matched, "QA1EXACT");
    assert!(d.resolve_exact("QA1EXAC").is_none());
    assert!(d.resolve_exact("QA1EXACTX").is_none());
    // An exact entry is never used as a prefix.
    assert_eq!(d.resolve_prefix("QA1EXACTX").unwrap().matched, "QA");
    // Exact calls may contain '/'.
    assert!(d.resolve_exact("QB1EX/P").is_some());
}

#[test]
fn exact_call_overrides_all_fields() {
    let m = synth().resolve_exact("QA1EXACT").unwrap();
    assert_eq!(m.cq_zone, 16);
    assert_eq!(m.itu_zone, 29);
    assert_eq!(m.continent, "AS");
    assert!(approx(m.latitude, 10.5));
    assert!(approx(m.longitude, 30.25));
    assert!(approx(m.utc_offset, 5.5));
    assert_eq!(m.entity_name, "Alpha Land");
}

#[test]
fn continent_override_on_prefix() {
    let m = synth().resolve_prefix("QBA1").unwrap();
    assert_eq!(m.continent, "SA");
    assert_eq!(m.entity_name, "Beta Isles");
}

#[test]
fn wae_only_entity_and_entity_codes() {
    let d = synth();
    let m = d.resolve_prefix("QA9AB").unwrap();
    assert_eq!(m.entity_name, "Alpha South");
    assert!(m.wae_only);
    assert_eq!(m.primary_prefix, "QA9");
    // The WAE-only entity carries the code of its DXCC parent.
    assert_eq!(m.dxcc, Some(901));
    assert_eq!(d.resolve_prefix("QB1").unwrap().dxcc, Some(902));
}

#[test]
fn without_csv_entity_codes_are_unknown() {
    let d = CtyData::parse(SYNTH_DAT).unwrap();
    assert_eq!(d.resolve_prefix("QA1").unwrap().dxcc, None);
}

#[test]
fn malformed_record_is_skipped_and_reported() {
    let text = "\
Broken:  99:  27:  EU:  50.0:  -20.0:  -1.0:  QX:
    QX;
Good:    14:  27:  EU:  50.0:  -20.0:  -1.0:  QG:
    QG;
";
    let d = CtyData::parse(text).unwrap();
    assert_eq!(d.entities().len(), 1);
    assert_eq!(d.entities()[0].name, "Good");
    assert!(d.resolve_prefix("QX1").is_none());
    assert_eq!(d.issues.len(), 1);
    assert_eq!(d.issues[0].line, 1);
}

#[test]
fn unknown_modifier_rejects_only_that_alias() {
    let text = "Good: 14: 27: EU: 50.0: -20.0: -1.0: QG:\n    QG,QG7^9^,QG8;\n";
    let d = CtyData::parse(text).unwrap();
    assert!(d.resolve_prefix("QG8").is_some());
    assert_eq!(d.resolve_prefix("QG7").unwrap().matched, "QG");
    assert_eq!(d.issues.len(), 1);
    assert!(d.issues[0].message.contains("unknown modifier"));
}

#[test]
fn empty_and_whitespace_files_are_errors_without_panic() {
    assert!(matches!(CtyData::parse(""), Err(CtyError::NoEntities)));
    assert!(matches!(
        CtyData::parse(" \n\t\n"),
        Err(CtyError::NoEntities)
    ));
    assert!(matches!(
        CtyData::parse("\u{feff}"),
        Err(CtyError::NoEntities)
    ));
}

#[test]
fn missing_file_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let r = CtyData::load(&dir.path().join("cty.dat"));
    assert!(matches!(r, Err(CtyError::Io(_))));
    assert!(matches!(
        load_cty_csv(&dir.path().join("cty.csv")),
        Err(CtyError::Io(_))
    ));
}

#[test]
fn binary_file_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("cty.dat");
    std::fs::write(&p, b"QA:\0\0\0").unwrap();
    assert!(matches!(CtyData::load(&p), Err(CtyError::NotText)));
}

#[test]
fn load_from_tempdir_and_reload_is_stable() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("cty.dat");
    std::fs::write(&p, SYNTH_DAT).unwrap();
    let first = CtyData::load(&p).unwrap();
    let second = CtyData::load(&p).unwrap();
    for call in ["QA1", "QA25X", "QB9Z", "QA9Q", "QBA"] {
        assert_eq!(first.resolve_prefix(call), second.resolve_prefix(call));
    }
    // Reloading after an update picks up the new content.
    std::fs::write(&p, SYNTH_DAT.replace("QA2,", "QA2,QA7,")).unwrap();
    let third = CtyData::load(&p).unwrap();
    assert_eq!(third.resolve_prefix("QA7").unwrap().matched, "QA7");
    assert_eq!(first.resolve_prefix("QA7").unwrap().matched, "QA");
}

#[test]
fn exact_calls_listing_is_sorted() {
    let calls: Vec<String> = synth()
        .exact_calls()
        .into_iter()
        .map(|m| m.matched)
        .collect();
    assert_eq!(calls, vec!["QA1EXACT", "QA9WAE", "QB1EX/P"]);
}

#[test]
fn csv_reports_missing_unknown_and_mismatched_names() {
    let mut d = CtyData::parse(SYNTH_DAT).unwrap();
    let csv = "\
QA,Alpha Land,901,EU,14,27,50.00,-20.00,-1.0,QA;
QB,Renamed Isles,902,NA,3,6,20.00,100.00,8.0,QB;
QZ,Unlisted,999,EU,14,27,50.00,-20.00,-1.0,QZ;
";
    let (rows, issues) = parse_cty_csv(csv);
    assert_eq!(issues, Vec::<CtyIssue>::new());
    let r = d.apply_entity_codes(&rows);
    assert_eq!(r.matched, 2);
    assert_eq!(r.missing_in_csv, vec!["*QA9".to_string()]);
    assert_eq!(r.unknown_in_csv, vec!["QZ".to_string()]);
    assert_eq!(r.name_mismatches, vec!["QB".to_string()]);
}

#[test]
fn csv_rejects_malformed_rows() {
    let csv = "\
QA,Alpha Land,901,EU,14,27,50.00,-20.00,-1.0,QA;
QB,short,row
QC,Bad Code,abc,EU,14,27,50,-20,-1,QC;
QD,Zero Code,0,EU,14,27,50,-20,-1,QD;
Q!,Bad Prefix,903,EU,14,27,50,-20,-1,Q;
QA,Duplicate,904,EU,14,27,50,-20,-1,QA;
";
    let (rows, issues) = parse_cty_csv(csv);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].dxcc, 901);
    assert_eq!(issues.len(), 5);
    let lines: Vec<usize> = issues.iter().map(|i| i.line).collect();
    assert_eq!(lines, vec![2, 3, 4, 5, 6]);
}

#[test]
fn oversized_file_is_rejected_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("cty.dat");
    let f = std::fs::File::create(&p).unwrap();
    f.set_len(MAX_COUNTRY_FILE_BYTES + 1).unwrap();
    assert!(matches!(CtyData::load(&p), Err(CtyError::TooLarge(_))));
}

// ---------------------------------------------------------------- Level B ---

/// Deterministic regression corpus: synthetic entities with overlapping
/// prefixes, exact calls, overrides, duplicates and conflicts.
const CORPUS_DAT: &str = "\
Short Land:               5:   8:   NA:   40.00:    75.00:     5.0:  Q:
    Q,QZ0;
Overlap North:            1:   2:   NA:   60.00:   150.00:     9.0:  QN:
    QN,QN1,QN12,QN123(2)[3],=QN1AA,
    QN9{AS}<55.0/-80.0>;
Overlap South:            40:  90:  AN:  -80.00:     0.00:     0.0:  QS:
    QS,QN12A,=QN1AA,QS(9);
Duplicate Holder:         20:  30:  AS:   10.00:   -50.00:    -3.0:  QD:
    QD,QD,QN,=QD1X,=QD1X(21);
Long Prefix Land:         10:  12:  SA:  -20.00:    60.00:     4.0:  QLONGPRE:
    QLONGPRE,QL;
Bad Zone:                 41:  12:  SA:  -20.00:    60.00:     4.0:  QBZ:
    QBZ;
Bad Coords:               10:  12:  SA:  -95.00:    60.00:     4.0:  QBC:
    QBC;
Bad Number:               x1:  12:  SA:  -20.00:    60.00:     4.0:  QBN:
    QBN;
Missing Fields:           10:  12:  SA:  QMF:
    QMF;
Bad Continent:            10:  12:  XX:  -20.00:    60.00:     4.0:  QBK:
    QBK;
";

struct Expect {
    full: &'static str,
    prefix: &'static str,
    entity: Option<&'static str>,
    matched: &'static str,
    cq: u32,
    itu: u32,
    cont: &'static str,
}

const fn e(
    full: &'static str,
    prefix: &'static str,
    entity: Option<&'static str>,
    matched: &'static str,
    cq: u32,
    itu: u32,
    cont: &'static str,
) -> Expect {
    Expect {
        full,
        prefix,
        entity,
        matched,
        cq,
        itu,
        cont,
    }
}

const CORPUS: &[Expect] = &[
    // short / long / overlapping prefixes
    e("Q7AB", "Q7AB", Some("Short Land"), "Q", 5, 8, "NA"),
    e("QZ0AB", "QZ0AB", Some("Short Land"), "QZ0", 5, 8, "NA"),
    e("QN5AB", "QN5AB", Some("Overlap North"), "QN", 1, 2, "NA"),
    e("QN1XY", "QN1XY", Some("Overlap North"), "QN1", 1, 2, "NA"),
    e("QN12X", "QN12X", Some("Overlap North"), "QN12", 1, 2, "NA"),
    e("QN123", "QN123", Some("Overlap North"), "QN123", 2, 3, "NA"),
    e(
        "QN12AB",
        "QN12AB",
        Some("Overlap South"),
        "QN12A",
        40,
        90,
        "AN",
    ),
    e(
        "QLONGPRE1",
        "QLONGPRE1",
        Some("Long Prefix Land"),
        "QLONGPRE",
        10,
        12,
        "SA",
    ),
    e("QL1", "QL1", Some("Long Prefix Land"), "QL", 10, 12, "SA"),
    // first occurrence wins for duplicated prefixes and exact calls
    e("QS1", "QS1", Some("Overlap South"), "QS", 40, 90, "AN"),
    e("QN1AA", "QN1AA", Some("Overlap North"), "QN1AA", 1, 2, "NA"),
    e(
        "QD1X",
        "QD1X",
        Some("Duplicate Holder"),
        "QD1X",
        20,
        30,
        "AS",
    ),
    // continent and coordinate overrides
    e("QN9Z", "QN9Z", Some("Overlap North"), "QN9", 1, 2, "AS"),
    // portable / mobile / broken calls: the caller passes the normalised form
    e("QN5AB/P", "QN5AB", Some("Overlap North"), "QN", 1, 2, "NA"),
    e("QN5AB/M", "QN5AB", Some("Overlap North"), "QN", 1, 2, "NA"),
    e("QS/QN5AB", "QS", Some("Overlap South"), "QS", 40, 90, "AN"),
    // case and whitespace
    e(
        " qn1aa ",
        " qn1aa ",
        Some("Overlap North"),
        "QN1AA",
        1,
        2,
        "NA",
    ),
    e("qz0ab", "qz0ab", Some("Short Land"), "QZ0", 5, 8, "NA"),
    // records rejected for invalid data are not matched
    e("QBZ1", "QBZ1", Some("Short Land"), "Q", 5, 8, "NA"),
    e("QBC1", "QBC1", Some("Short Land"), "Q", 5, 8, "NA"),
    e("QBN1", "QBN1", Some("Short Land"), "Q", 5, 8, "NA"),
    e("QBK1", "QBK1", Some("Short Land"), "Q", 5, 8, "NA"),
    // no match at all
    e("X1ABC", "X1ABC", None, "", 0, 0, ""),
    e("", "", None, "", 0, 0, ""),
    e("1", "1", None, "", 0, 0, ""),
];

#[test]
fn regression_corpus_resolves_deterministically() {
    let d = CtyData::parse(CORPUS_DAT).unwrap();
    for case in CORPUS {
        let got = d.resolve(case.full, case.prefix);
        match (case.entity, &got) {
            (None, None) => {}
            (Some(name), Some(m)) => {
                assert_eq!(m.entity_name, name, "{}", case.full);
                assert_eq!(m.matched, case.matched, "{}", case.full);
                assert_eq!(m.cq_zone, case.cq, "{}", case.full);
                assert_eq!(m.itu_zone, case.itu, "{}", case.full);
                assert_eq!(m.continent, case.cont, "{}", case.full);
            }
            _ => panic!("{}: expected {:?}, got {:?}", case.full, case.entity, got),
        }
    }
}

#[test]
fn regression_corpus_reports_every_invalid_record() {
    let d = CtyData::parse(CORPUS_DAT).unwrap();
    assert_eq!(d.entities().len(), 5);
    let msgs: Vec<&str> = d.issues.iter().map(|i| i.message.as_str()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("invalid CQ zone '41'")),
        "{msgs:?}"
    );
    assert!(
        msgs.iter().any(|m| m.contains("coordinates out of range")),
        "{msgs:?}"
    );
    assert!(
        msgs.iter().any(|m| m.contains("invalid CQ zone 'x1'")),
        "{msgs:?}"
    );
    assert!(
        msgs.iter().any(|m| m.contains("invalid continent 'XX'")),
        "{msgs:?}"
    );
    // "Missing Fields" reaches ';' before its eighth ':' and is rejected as an
    // incomplete header; the parser resynchronises on that ';'.
    assert!(
        msgs.iter().any(|m| m.contains("incomplete entity header")),
        "{msgs:?}"
    );
    assert_eq!(d.issues.len(), 5, "{msgs:?}");
}

#[test]
fn coordinate_override_uses_east_positive_sign() {
    let d = CtyData::parse(CORPUS_DAT).unwrap();
    let m = d.resolve_prefix("QN9Z").unwrap();
    assert!(approx(m.latitude, 55.0));
    assert!(approx(m.longitude, 80.0));
    let base = d.resolve_prefix("QN5").unwrap();
    assert!(approx(base.longitude, -150.0));
}

#[test]
fn parser_never_panics_on_generated_input() {
    // Deterministic pseudo-random mutation of the corpus.
    let alphabet: Vec<char> = "QAZN019:;,=()[]<>{}~/.- \n\t*".chars().collect();
    let mut state: u64 = 0x5eed_1234_abcd_0001;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..500 {
        let mut s: Vec<char> = CORPUS_DAT.chars().collect();
        for _ in 0..(next() % 20) {
            let pos = (next() as usize) % s.len();
            let c = alphabet[(next() as usize) % alphabet.len()];
            s[pos] = c;
        }
        let text: String = s.into_iter().collect();
        if let Ok(d) = CtyData::parse(&text) {
            for call in ["QN12AB", "Q", "QD1X", "QN1AA/P"] {
                let _ = d.resolve(call, call);
            }
        }
        let _ = parse_cty_csv(&text);
    }
}
