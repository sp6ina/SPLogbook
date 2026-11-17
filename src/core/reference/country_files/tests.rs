// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Country-file archive and installation tests. Archives are built in memory
//! from synthetic data (unallocated `Q` prefixes).

use super::*;
use std::io::Write;

const DAT: &str = "\
Alpha Land:               14:  27:  EU:   50.00:   -20.00:    -1.0:  QA:
    QA,QA2(15)[28],=QA1EXACT,=VER20261101;
Beta Isles:               3:   6:   NA:   20.00:   100.00:     8.0:  QB:
    QB;
";
const CSV: &str = "\
QA,Alpha Land,901,EU,14,27,50.00,-20.00,-1.0,QA QA2(15)[28] =QA1EXACT =VER20261101;
QB,Beta Isles,902,NA,3,6,20.00,100.00,8.0,QB;
";

fn zip_with(entries: &[(&str, &[u8], zip::CompressionMethod)]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        for (name, data, method) in entries {
            let opts = zip::write::SimpleFileOptions::default().compression_method(*method);
            w.start_file(*name, opts).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }
    buf.into_inner()
}

fn good_zip() -> Vec<u8> {
    zip_with(&[
        ("README.TXT", b"readme", zip::CompressionMethod::Stored),
        (DAT_FILE, DAT.as_bytes(), zip::CompressionMethod::Deflated),
        (CSV_FILE, CSV.as_bytes(), zip::CompressionMethod::Stored),
    ])
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Minimal Stored-only ZIP writer that allows names and sizes the real
/// writer refuses, to model hostile archives. `declared` overrides the size
/// written in the headers.
fn raw_zip(entries: &[(&str, &[u8], Option<u32>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data, declared) in entries {
        let size = declared.unwrap_or(data.len() as u32);
        let crc = crc32(data);
        let offset = out.len() as u32;
        let mut header = Vec::new();
        header.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        header.extend_from_slice(&20u16.to_le_bytes()); // version needed
        header.extend_from_slice(&0u16.to_le_bytes()); // flags
        header.extend_from_slice(&0u16.to_le_bytes()); // stored
        header.extend_from_slice(&0u16.to_le_bytes()); // time
        header.extend_from_slice(&0x21u16.to_le_bytes()); // date 1980-01-01
        header.extend_from_slice(&crc.to_le_bytes());
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&(name.len() as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&header);
        out.extend_from_slice(data);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // made by
        central.extend_from_slice(&20u16.to_le_bytes()); // needed
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0x21u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[test]
fn reads_both_entries_from_stored_and_deflated_archive() {
    let p = read_country_archive(&good_zip(), &ArchiveLimits::default(), true).unwrap();
    assert_eq!(p.dat, DAT.as_bytes());
    assert_eq!(p.csv.as_deref(), Some(CSV.as_bytes()));
}

#[test]
fn raw_archive_builder_is_readable() {
    let z = raw_zip(&[
        (DAT_FILE, DAT.as_bytes(), None),
        (CSV_FILE, CSV.as_bytes(), None),
    ]);
    let p = read_country_archive(&z, &ArchiveLimits::default(), true).unwrap();
    assert_eq!(p.dat, DAT.as_bytes());
}

#[test]
fn missing_entries_are_reported() {
    let only_csv = zip_with(&[(CSV_FILE, CSV.as_bytes(), zip::CompressionMethod::Stored)]);
    assert_eq!(
        read_country_archive(&only_csv, &ArchiveLimits::default(), false),
        Err(ArchiveError::MissingEntry(DAT_FILE))
    );
    let only_dat = zip_with(&[(DAT_FILE, DAT.as_bytes(), zip::CompressionMethod::Stored)]);
    assert_eq!(
        read_country_archive(&only_dat, &ArchiveLimits::default(), true),
        Err(ArchiveError::MissingEntry(CSV_FILE))
    );
    let p = read_country_archive(&only_dat, &ArchiveLimits::default(), false).unwrap();
    assert_eq!(p.csv, None);
}

#[test]
fn nested_entries_are_not_mistaken_for_the_wanted_files() {
    let z = zip_with(&[
        (
            "old/cty.dat",
            DAT.as_bytes(),
            zip::CompressionMethod::Stored,
        ),
        (CSV_FILE, CSV.as_bytes(), zip::CompressionMethod::Stored),
    ]);
    assert_eq!(
        read_country_archive(&z, &ArchiveLimits::default(), true),
        Err(ArchiveError::MissingEntry(DAT_FILE))
    );
}

#[test]
fn duplicate_wanted_entry_rejects_the_archive() {
    let z = raw_zip(&[
        (DAT_FILE, DAT.as_bytes(), None),
        ("CTY.DAT", b"other", None),
        (CSV_FILE, CSV.as_bytes(), None),
    ]);
    assert_eq!(
        read_country_archive(&z, &ArchiveLimits::default(), true),
        Err(ArchiveError::DuplicateEntry(DAT_FILE))
    );
}

#[test]
fn traversal_and_absolute_names_reject_the_archive() {
    for bad in [
        "../cty.dat",
        "/etc/cty.dat",
        "a/../../x",
        "C:\\cty.dat",
        "..\\x",
        "\\x",
        "a\u{1}b",
    ] {
        let z = raw_zip(&[(bad, b"x", None), (DAT_FILE, DAT.as_bytes(), None)]);
        assert_eq!(
            read_country_archive(&z, &ArchiveLimits::default(), false),
            Err(ArchiveError::UnsafeEntryName(bad.to_string())),
            "{bad}"
        );
    }
}

#[test]
fn size_limits_are_enforced() {
    let limits = ArchiveLimits {
        max_archive_bytes: 100_000,
        max_entries: 3,
        max_entry_bytes: 1024,
    };
    // Archive too large before parsing.
    assert!(matches!(
        read_country_archive(&vec![0u8; 100_001], &limits, false),
        Err(ArchiveError::TooLarge(_))
    ));
    // Too many entries.
    let many = zip_with(&[
        ("a", b"1", zip::CompressionMethod::Stored),
        ("b", b"2", zip::CompressionMethod::Stored),
        ("c", b"3", zip::CompressionMethod::Stored),
        (DAT_FILE, b"4", zip::CompressionMethod::Stored),
    ]);
    assert_eq!(
        read_country_archive(&many, &limits, false),
        Err(ArchiveError::TooManyEntries(4))
    );
    // Highly compressible entry: small archive, declared size over the limit.
    let bomb = zip_with(&[(
        DAT_FILE,
        &vec![b' '; 200_000],
        zip::CompressionMethod::Deflated,
    )]);
    assert!(bomb.len() < 100_000);
    assert_eq!(
        read_country_archive(&bomb, &limits, false),
        Err(ArchiveError::EntryTooLarge {
            name: DAT_FILE,
            size: 200_000
        })
    );
}

#[test]
fn lying_size_header_is_an_error_not_a_panic() {
    let payload = vec![b'Q'; 2000];
    let z = raw_zip(&[(DAT_FILE, &payload, Some(10))]);
    let r = read_country_archive(&z, &ArchiveLimits::default(), false);
    assert!(
        r.is_err() || r.as_ref().is_ok_and(|p| p.dat.len() <= 10),
        "{r:?}"
    );
}

#[test]
fn garbage_is_not_a_zip() {
    assert!(matches!(
        read_country_archive(b"definitely not a zip", &ArchiveLimits::default(), false),
        Err(ArchiveError::NotZip(_))
    ));
    assert!(matches!(
        read_country_archive(&[], &ArchiveLimits::default(), false),
        Err(ArchiveError::NotZip(_))
    ));
}

#[test]
fn validation_requires_matching_csv() {
    let p = read_country_archive(&good_zip(), &ArchiveLimits::default(), true).unwrap();
    let s = validate_payload(&p).unwrap();
    assert_eq!(s.entities, 2);
    assert_eq!(s.version.as_deref(), Some("VER20261101"));
    assert_eq!(s.date.as_deref(), Some("2026-11-01"));
    assert_eq!(s.entity_codes.as_ref().unwrap().matched, 2);

    let partial = CountryFilesPayload {
        dat: DAT.as_bytes().to_vec(),
        csv: Some(CSV.lines().next().unwrap().as_bytes().to_vec()),
    };
    assert!(matches!(
        validate_payload(&partial),
        Err(CountryFilesError::Inconsistent(_))
    ));
    let empty_dat = CountryFilesPayload {
        dat: Vec::new(),
        csv: None,
    };
    assert!(matches!(
        validate_payload(&empty_dat),
        Err(CountryFilesError::Data(_))
    ));
}

#[test]
fn install_load_update_and_rollback() {
    let root = tempfile::tempdir().unwrap();
    assert!(load_installed(root.path()).unwrap().is_none());

    let p = read_country_archive(&good_zip(), &ArchiveLimits::default(), true).unwrap();
    let origin = SetOrigin {
        source: BIG_CTY_ARCHIVE_URL.into(),
        ..SetOrigin::default()
    };
    let (manifest, _) = install_country_files(root.path(), &p, origin.clone()).unwrap();
    assert_eq!(manifest.origin.source_version, "VER20261101");
    assert_eq!(manifest.origin.source_date, "2026-11-01");

    let (m, report, _) = load_installed(root.path()).unwrap().unwrap();
    assert_eq!(report.entities, 2);
    assert_eq!(m.lookup("QA5XYZ").unwrap().dxcc, 901);

    // A newer release replaces it; the previous one is kept.
    let newer = CountryFilesPayload {
        dat: DAT
            .replace("VER20261101", "VER20261115")
            .replace("QA,QA2", "QA,QA7,QA2")
            .into_bytes(),
        csv: Some(CSV.replace("VER20261101", "VER20261115").into_bytes()),
    };
    install_country_files(root.path(), &newer, origin.clone()).unwrap();
    let (_, _, m2) = load_installed(root.path()).unwrap().unwrap();
    assert_eq!(m2.origin.source_date, "2026-11-15");

    // A broken release is rejected and the installed one keeps working.
    let broken = CountryFilesPayload {
        dat: DAT.as_bytes().to_vec(),
        csv: Some(b"QZ,Nowhere,999,EU,1,1,0,0,0,QZ;\n".to_vec()),
    };
    assert!(install_country_files(root.path(), &broken, origin).is_err());
    let (_, _, still) = load_installed(root.path()).unwrap().unwrap();
    assert_eq!(still.origin.source_date, "2026-11-15");

    let back = rollback(root.path()).unwrap();
    assert_eq!(back.origin.source_date, "2026-11-01");
}

#[test]
fn user_file_import_accepts_archive_or_dat_with_csv() {
    let root = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let zip_path = src.path().join("release.zip");
    std::fs::write(&zip_path, good_zip()).unwrap();
    let (m, _) = install_from_user_file(root.path(), &zip_path).unwrap();
    assert_eq!(m.origin.source, "release.zip");

    let dat_path = src.path().join(DAT_FILE);
    std::fs::write(&dat_path, DAT).unwrap();
    assert!(matches!(
        install_from_user_file(root.path(), &dat_path),
        Err(CountryFilesError::Archive(ArchiveError::MissingEntry(
            CSV_FILE
        )))
    ));
    std::fs::write(src.path().join(CSV_FILE), CSV).unwrap();
    let (m, s) = install_from_user_file(root.path(), &dat_path).unwrap();
    assert_eq!(m.origin.source, DAT_FILE);
    assert_eq!(s.entities, 2);
    assert!(install_from_user_file(root.path(), &src.path().join("missing.zip")).is_err());
}
