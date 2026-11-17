// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Rules-file tests. All rules are synthetic and use the unallocated `Q` block.

use super::*;

const HEADER: &str = "#splogbook-rules,1\nrule_type,callsign_or_prefix,dxcc_entity_code,subdivision_code,subdivision_name,cq_zone,itu_zone,continent,latitude,longitude,priority,source,source_version,source_date,notes\n";

fn parse_ok(body: &str) -> RulesParse {
    let p = parse_rules(&format!("{HEADER}{body}"));
    assert!(p.is_valid(), "{:?}", p.issues);
    p
}

fn first_error(body: &str) -> RuleIssue {
    let p = parse_rules(&format!("{HEADER}{body}"));
    p.errors().next().cloned().expect("expected an error")
}

#[test]
fn parses_full_rule_with_all_columns() {
    let p = parse_ok(
        "prefix,qa9,901,QA-N,\"North, Upper\",15,28,eu,51.5,-21.25,5,Local survey,v3,2026-11-01,\"said \"\"hi\"\"\"\n",
    );
    assert_eq!(p.rules.len(), 1);
    let r = &p.rules[0];
    assert_eq!(r.rule_type, RuleType::Prefix);
    assert_eq!(r.key, "QA9");
    assert_eq!(r.priority, 5);
    assert_eq!(r.line, 3);
    let a = &r.attributes;
    assert_eq!(a.dxcc, Some(901));
    assert_eq!(a.subdivision_code.as_deref(), Some("QA-N"));
    assert_eq!(a.subdivision_name.as_deref(), Some("North, Upper"));
    assert_eq!((a.cq_zone, a.itu_zone), (Some(15), Some(28)));
    assert_eq!(a.continent.as_deref(), Some("EU"));
    assert_eq!(a.coordinates, Some((51.5, -21.25)));
    assert_eq!(r.provenance.source, "Local survey");
    assert_eq!(r.provenance.source_version, "v3");
    assert_eq!(r.provenance.source_date, "2026-11-01");
    assert_eq!(r.provenance.notes, "said \"hi\"");
}

#[test]
fn columns_are_matched_by_name_and_optional_ones_may_be_absent() {
    let p = parse_rules(
        "#splogbook-rules,1\n# comment\n\ncq_zone,callsign_or_prefix,rule_type\n7,QB1ABC,exact\n",
    );
    assert!(p.is_valid(), "{:?}", p.issues);
    assert_eq!(p.rules[0].key, "QB1ABC");
    assert_eq!(p.rules[0].attributes.cq_zone, Some(7));
    assert_eq!(p.rules[0].priority, 0);
}

#[test]
fn unknown_column_is_a_warning_only() {
    let p = parse_rules(
        "#splogbook-rules,1\nrule_type,callsign_or_prefix,colour,cq_zone\nprefix,QA,red,3\n",
    );
    assert!(p.is_valid());
    assert_eq!(
        p.issues[0].kind,
        RuleIssueKind::UnknownColumn("colour".into())
    );
    assert_eq!(p.rules[0].attributes.cq_zone, Some(3));
}

#[test]
fn format_marker_and_version_are_required() {
    for (text, kind) in [
        ("", RuleIssueKind::MissingFormatMarker),
        (
            "rule_type,callsign_or_prefix\n",
            RuleIssueKind::MissingFormatMarker,
        ),
        (
            "#splogbook-rules,2\n",
            RuleIssueKind::UnsupportedVersion("2".into()),
        ),
        ("#splogbook-rules,1\n", RuleIssueKind::MissingHeader),
        (
            "#splogbook-rules,1\ncallsign_or_prefix,cq_zone\n",
            RuleIssueKind::MissingRequiredColumn("rule_type"),
        ),
    ] {
        let p = parse_rules(text);
        assert!(!p.is_valid(), "{text:?}");
        assert_eq!(p.errors().next().unwrap().kind, kind, "{text:?}");
    }
    // A UTF-8 BOM before the marker is accepted.
    assert!(
        parse_rules(
            "\u{feff}#splogbook-rules,1\nrule_type,callsign_or_prefix,cq_zone\nprefix,QA,1\n"
        )
        .is_valid()
    );
}

#[test]
fn every_invalid_value_is_reported_with_its_line() {
    let cases: &[(&str, RuleIssueKind)] = &[
        (
            "range,QA,,,,1,,,,,,,,,\n",
            RuleIssueKind::InvalidRuleType("range".into()),
        ),
        (
            "prefix,Q A,,,,1,,,,,,,,,\n",
            RuleIssueKind::InvalidKey("Q A".into()),
        ),
        (
            "pattern,QA(,,,,1,,,,,,,,,\n",
            RuleIssueKind::InvalidPattern(String::new()),
        ),
        (
            "prefix,QA,abc,,,,,,,,,,,,\n",
            RuleIssueKind::InvalidNumber {
                column: "dxcc_entity_code",
                value: "abc".into(),
            },
        ),
        (
            "prefix,QA,0,,,,,,,,,,,,\n",
            RuleIssueKind::OutOfRange {
                column: "dxcc_entity_code",
                value: "0".into(),
            },
        ),
        (
            "prefix,QA,,,,41,,,,,,,,,\n",
            RuleIssueKind::OutOfRange {
                column: "cq_zone",
                value: "41".into(),
            },
        ),
        (
            "prefix,QA,,,,,91,,,,,,,,\n",
            RuleIssueKind::OutOfRange {
                column: "itu_zone",
                value: "91".into(),
            },
        ),
        (
            "prefix,QA,,,,,,XX,,,,,,,\n",
            RuleIssueKind::InvalidContinent("XX".into()),
        ),
        (
            "prefix,QA,,,,,,,91,10,,,,,\n",
            RuleIssueKind::OutOfRange {
                column: "latitude",
                value: "91".into(),
            },
        ),
        (
            "prefix,QA,,,,,,,10,181,,,,,\n",
            RuleIssueKind::OutOfRange {
                column: "longitude",
                value: "181".into(),
            },
        ),
        (
            "prefix,QA,,,,,,,10,,,,,,\n",
            RuleIssueKind::IncompleteCoordinates,
        ),
        (
            "prefix,QA,,,,,,,NaN,10,,,,,\n",
            RuleIssueKind::IncompleteCoordinates,
        ),
        (
            "prefix,QA,,,,1,,,,,1001,,,,\n",
            RuleIssueKind::OutOfRange {
                column: "priority",
                value: "1001".into(),
            },
        ),
        (
            "prefix,QA,,,,1,,,,,,,,2026-13-01,\n",
            RuleIssueKind::InvalidDate("2026-13-01".into()),
        ),
        (
            "prefix,QA,,,,1,,,,,,,,17.11.2026,\n",
            RuleIssueKind::InvalidDate("17.11.2026".into()),
        ),
        ("prefix,QA,,,,,,,,,,,,,\n", RuleIssueKind::EmptyRule),
        (
            "prefix,QA,,,,1,,,,,,,,,,extra\n",
            RuleIssueKind::TooManyFields(16),
        ),
        ("prefix,\"QA,,,,1\n", RuleIssueKind::UnterminatedQuote),
    ];
    for (row, kind) in cases {
        let e = first_error(row);
        assert_eq!(e.line, 3, "{row}");
        match (kind, &e.kind) {
            (RuleIssueKind::InvalidPattern(_), RuleIssueKind::InvalidPattern(_)) => {}
            _ => assert_eq!(&e.kind, kind, "{row}"),
        }
    }
}

#[test]
fn one_invalid_row_rejects_the_whole_file() {
    let p = parse_rules(&format!(
        "{HEADER}prefix,QA,,,,1,,,,,,,,,\nprefix,QB,,,,99,,,,,,,,,\n"
    ));
    assert!(!p.is_valid());
    assert_eq!(p.rules.len(), 1);
    assert_eq!(p.errors().count(), 1);
}

#[test]
fn identical_duplicate_is_kept_once_with_a_warning() {
    let p = parse_ok("prefix,QA,,,,1,,,,,,,,,\nprefix,qa,,,,1,,,,,,,,,\n");
    assert_eq!(p.rules.len(), 1);
    assert_eq!(p.issues[0].kind, RuleIssueKind::DuplicateRule("QA".into()));
}

#[test]
fn conflicting_rule_for_same_key_and_priority_rejects_the_file() {
    let e = first_error("prefix,QA,,,,1,,,,,,,,,\nprefix,QA,,,,2,,,,,,,,,\n");
    assert_eq!(
        e.kind,
        RuleIssueKind::ConflictingRule {
            key: "QA".into(),
            first_line: 3
        }
    );
    // Different priorities for the same key are allowed; the higher one wins.
    let p = parse_ok("prefix,QA,,,,1,,,,,1,,,,\nprefix,QA,,,,2,,,,,9,,,,\n");
    let book = RuleBook::from_rules(p.rules);
    match book.match_prefix("QA1") {
        RuleMatch::Matched(r) => assert_eq!(r.attributes.cq_zone, Some(2)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn matching_order_exact_then_longest_prefix_then_pattern() {
    let p = parse_ok(
        "exact,QA1XYZ,,,,10,,,,,,,,,\n\
         prefix,QA,,,,1,,,,,,,,,\n\
         prefix,QA1,,,,2,,,,,,,,,\n\
         pattern,QB[0-9]Z.*,,,,3,,,,,,,,,\n",
    );
    let book = RuleBook::from_rules(p.rules);
    assert_eq!(book.len(), 4);
    assert_eq!(
        book.match_exact("QA1XYZ/P", "QA1XYZ")
            .unwrap()
            .attributes
            .cq_zone,
        Some(10)
    );
    assert!(book.match_exact("QA1XY", "QA1XY").is_none());
    let zone = |call| match book.match_prefix(call) {
        RuleMatch::Matched(r) => r.attributes.cq_zone,
        _ => None,
    };
    assert_eq!(zone("QA1XYZ"), Some(2));
    assert_eq!(zone("QA2"), Some(1));
    assert_eq!(zone("QB5ZZ"), Some(3));
    assert_eq!(zone("QB5AA"), None);
}

#[test]
fn equal_priority_patterns_are_a_conflict_not_an_arbitrary_choice() {
    let p = parse_ok(
        "pattern,QC.*,,,,4,,,,,1,,,,\npattern,QC9.*,,,,5,,,,,1,,,,\npattern,QC.*9,,,,6,,,,,0,,,,\n",
    );
    let book = RuleBook::from_rules(p.rules);
    match book.match_prefix("QC9AB") {
        RuleMatch::Conflict(rules) => assert_eq!(rules.len(), 2),
        other => panic!("{other:?}"),
    }
    match book.match_prefix("QC1A9") {
        RuleMatch::Matched(r) => assert_eq!(r.attributes.cq_zone, Some(4)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn file_reader_enforces_utf8_and_size() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("rules.csv");
    std::fs::write(&p, b"#splogbook-rules,1\n\xff\xfe").unwrap();
    assert_eq!(read_rules_file(&p).unwrap(), Err(RuleIssueKind::NotUtf8));
    let f = std::fs::File::create(&p).unwrap();
    f.set_len(MAX_RULES_FILE_BYTES + 1).unwrap();
    assert!(matches!(
        read_rules_file(&p).unwrap(),
        Err(RuleIssueKind::TooLarge(_))
    ));
    assert!(read_rules_file(&dir.path().join("missing.csv")).is_err());
}

#[test]
fn parser_never_panics_on_mutated_input() {
    let base = format!(
        "{HEADER}prefix,QA,901,X,\"N, U\",1,2,EU,1.5,2.5,3,s,v,2026-01-01,n\npattern,QB.*,,,,4,,,,,,,,,\n"
    );
    let alphabet: Vec<char> = "\",#\n QAZ09.-()[]*^$\\".chars().collect();
    let mut state: u64 = 0x1234_5678_9abc_def1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..500 {
        let mut s: Vec<char> = base.chars().collect();
        for _ in 0..(next() % 12) {
            let pos = (next() as usize) % s.len();
            s[pos] = alphabet[(next() as usize) % alphabet.len()];
        }
        let p = parse_rules(&s.into_iter().collect::<String>());
        let book = RuleBook::from_rules(p.rules);
        let _ = book.match_prefix("QA1");
        let _ = book.match_exact("QA1", "QA1");
    }
}
