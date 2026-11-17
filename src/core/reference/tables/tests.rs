// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! User table tests. Every record is synthetic (unallocated `Q` callsigns,
//! invented names and places).

use super::*;

const IOTA: &str = "#splogbook-iota,1\nreference,name,prefix\nEU-901,Testing Isle,QA\neu-902,\"Second, Rock\",QB\nAS-903,Far Island,\n";
const STATES: &str = "#splogbook-states,1\ncode,name,country\nqn,North Test,Alpha Land\nQS,South Test,Alpha Land\nQN,North Other,Beta Isles\n";
const MANAGERS: &str = "#splogbook-qsl-managers,1\ncallsign,manager,years,notes\nqa1aa,qb1bb,2020-2026,via bureau\nQA2AA,QB1BB,,\n";
const CALLBOOK: &str = "#splogbook-callbook,1\ncallsign,name,qth,grid,state,manager,notes\nQA1AA,Test Operator,Testville,jo81WA,,QB1BB,\nQA2AA,,,KN11,,,\n";

fn parse_ok(kind: TableKind, text: &str) -> TableParse {
    let p = parse_table(kind, text);
    assert!(p.is_valid(), "{:?}", p.issues);
    p
}

#[test]
fn parses_every_table_format() {
    let p = parse_ok(TableKind::Iota, IOTA);
    assert_eq!(p.rows.len(), 3);
    assert_eq!(p.rows[1], vec!["EU-902", "Second, Rock", "QB"]);
    assert_eq!(parse_ok(TableKind::States, STATES).rows.len(), 3);
    let m = parse_ok(TableKind::QslManagers, MANAGERS);
    assert_eq!(m.rows[0], vec!["QA1AA", "QB1BB", "2020-2026", "via bureau"]);
    let c = parse_ok(TableKind::Callbook, CALLBOOK);
    assert_eq!(c.rows[0][3], "JO81wa");
    assert_eq!(c.rows[1][3], "KN11");
}

#[test]
fn marker_must_match_the_table_kind() {
    let p = parse_table(TableKind::States, IOTA);
    assert_eq!(
        p.errors().next().unwrap().kind,
        TableIssueKind::Format(RuleIssueKind::MissingFormatMarker)
    );
    let p = parse_table(TableKind::Iota, &IOTA.replace(",1\n", ",7\n"));
    assert_eq!(
        p.errors().next().unwrap().kind,
        TableIssueKind::Format(RuleIssueKind::UnsupportedVersion("7".into()))
    );
    let p = parse_table(TableKind::Iota, "#splogbook-iota,1\nname,prefix\n");
    assert_eq!(
        p.errors().next().unwrap().kind,
        TableIssueKind::Format(RuleIssueKind::MissingRequiredColumn("reference"))
    );
}

#[test]
fn invalid_values_are_rejected_with_their_line() {
    let cases: &[(TableKind, &str, &str)] = &[
        (
            TableKind::Iota,
            "#splogbook-iota,1\nreference,name\nXX-001,Bad\n",
            "reference",
        ),
        (
            TableKind::Iota,
            "#splogbook-iota,1\nreference,name\nEU-01,Bad\n",
            "reference",
        ),
        (
            TableKind::Iota,
            "#splogbook-iota,1\nreference,name\nEU-001,\n",
            "name",
        ),
        (
            TableKind::States,
            "#splogbook-states,1\ncode,name,country\nQN,North,\n",
            "country",
        ),
        (
            TableKind::QslManagers,
            "#splogbook-qsl-managers,1\ncallsign,manager\nQA1 AA,QB1BB\n",
            "callsign",
        ),
        (
            TableKind::QslManagers,
            "#splogbook-qsl-managers,1\ncallsign,manager\nQA1AA,\n",
            "manager",
        ),
        (
            TableKind::Callbook,
            "#splogbook-callbook,1\ncallsign,grid\nQA1AA,ZZ99\n",
            "grid",
        ),
        (
            TableKind::Callbook,
            "#splogbook-callbook,1\ncallsign,grid\nQA1AA,JO8\n",
            "grid",
        ),
        (
            TableKind::Callbook,
            "#splogbook-callbook,1\ncallsign,manager\nQA1AA,not a call\n",
            "manager",
        ),
    ];
    for (kind, text, column) in cases {
        let p = parse_table(*kind, text);
        let e = p.errors().next().unwrap_or_else(|| panic!("{text}"));
        assert_eq!(e.line, 3, "{text}");
        match &e.kind {
            TableIssueKind::InvalidValue { column: c, .. } => assert_eq!(c, column, "{text}"),
            other => panic!("{text}: {other:?}"),
        }
    }
    let p = parse_table(
        TableKind::Callbook,
        "#splogbook-callbook,1\ncallsign,name\nQA1AA,\n",
    );
    assert_eq!(p.errors().next().unwrap().kind, TableIssueKind::EmptyRow);
    let long = format!(
        "#splogbook-iota,1\nreference,name\nEU-001,{}\n",
        "x".repeat(300)
    );
    assert!(!parse_table(TableKind::Iota, &long).is_valid());
}

#[test]
fn duplicates_are_kept_once_and_conflicts_reject() {
    let p = parse_ok(
        TableKind::Iota,
        "#splogbook-iota,1\nreference,name\nEU-001,A\neu-001,A\n",
    );
    assert_eq!(p.rows.len(), 1);
    assert_eq!(
        p.issues[0].kind,
        TableIssueKind::DuplicateRow("EU-001".into())
    );
    let p = parse_table(
        TableKind::Iota,
        "#splogbook-iota,1\nreference,name\nEU-001,A\nEU-001,B\n",
    );
    assert_eq!(
        p.errors().next().unwrap().kind,
        TableIssueKind::ConflictingRow {
            key: "EU-001".into(),
            first_line: 3
        }
    );
    // The same state code in two countries is not a conflict.
    assert_eq!(parse_ok(TableKind::States, STATES).rows.len(), 3);
}

#[test]
fn import_load_query_and_keep_previous_on_error() {
    let root = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let write = |name: &str, text: &str| {
        let p = src.path().join(name);
        std::fs::write(&p, text).unwrap();
        p
    };

    let (empty, failures) = UserTables::load(root.path());
    assert!(failures.is_empty());
    for kind in TableKind::ALL {
        assert!(!empty.is_installed(kind));
        assert_eq!(empty.row_count(kind), 0);
    }

    for (kind, text) in [
        (TableKind::Iota, IOTA),
        (TableKind::States, STATES),
        (TableKind::QslManagers, MANAGERS),
        (TableKind::Callbook, CALLBOOK),
    ] {
        let r = import_table(
            root.path(),
            kind,
            &write(&format!("{}.csv", kind.set_name()), text),
        )
        .unwrap();
        assert!(r.rows > 0);
    }
    let bad = write("bad.csv", "#splogbook-iota,1\nreference,name\nEU-1,x\n");
    assert!(matches!(
        import_table(root.path(), TableKind::Iota, &bad),
        Err(TableImportError::Invalid(_))
    ));

    let (t, failures) = UserTables::load(root.path());
    assert!(failures.is_empty());
    assert_eq!(t.row_count(TableKind::Iota), 3);
    // Ordered by reference, case-insensitive name match.
    let found: Vec<String> = t.search_iota("ROCK").into_iter().map(|r| r.iota).collect();
    assert_eq!(found, vec!["EU-902"]);
    let all: Vec<String> = t.search_iota("").into_iter().map(|r| r.iota).collect();
    assert_eq!(all, vec!["AS-903", "EU-901", "EU-902"]);
    assert_eq!(t.find_iota("EU-901").unwrap().name, "Testing Isle");
    assert_eq!(t.search_states("NORTH").len(), 2);
    assert_eq!(t.find_manager("QA1AA").unwrap().manager, "QB1BB");
    assert_eq!(t.search_managers("QB1BB").len(), 2);
    let cb = t.find_callbook("QA1AA").unwrap();
    assert_eq!(cb.name.as_deref(), Some("Test Operator"));
    assert_eq!(cb.grid.as_deref(), Some("JO81wa"));
    assert_eq!(t.find_callbook("QA2AA").unwrap().name, None);

    let out = src.path().join("export.csv");
    export_table(root.path(), TableKind::States, &out).unwrap();
    assert_eq!(std::fs::read_to_string(&out).unwrap(), STATES);
}

#[test]
fn search_is_limited_to_one_hundred_rows() {
    let mut text = String::from("#splogbook-iota,1\nreference,name\n");
    for i in 0..150 {
        use std::fmt::Write as _;
        let _ = writeln!(text, "EU-{i:03},Isle {i}");
    }
    let root = tempfile::tempdir().unwrap();
    let p = root.path().join("i.csv");
    std::fs::write(&p, text).unwrap();
    import_table(root.path(), TableKind::Iota, &p).unwrap();
    let (t, _) = UserTables::load(root.path());
    assert_eq!(t.search_iota("ISLE").len(), 100);
    assert_eq!(t.search_iota("ISLE")[0].iota, "EU-000");
}

#[test]
fn parser_never_panics_on_mutated_input() {
    let alphabet: Vec<char> = "\",#\n QA19-.é\u{0}".chars().collect();
    let mut state: u64 = 0xdead_beef_1234_5678;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for (kind, base) in [
        (TableKind::Iota, IOTA),
        (TableKind::States, STATES),
        (TableKind::QslManagers, MANAGERS),
        (TableKind::Callbook, CALLBOOK),
    ] {
        for _ in 0..200 {
            let mut s: Vec<char> = base.chars().collect();
            for _ in 0..(next() % 8) {
                let pos = (next() as usize) % s.len();
                s[pos] = alphabet[(next() as usize) % alphabet.len()];
            }
            let _ = parse_table(kind, &s.into_iter().collect::<String>());
        }
    }
}
