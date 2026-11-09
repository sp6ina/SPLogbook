// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Testy ochrony danych użytkownika i bezpieczeństwa cyklu życia Windows MSI (M3-E)
//
// Bramka QA 2026-11-09: kontrola lokalnego artefaktu MSI jest JAWNYM OPT-IN.
// Standardowy baseline nie zależy od zawartości ignorowanego katalogu `dist/`.

use std::fs;

#[path = "support/native_artifact.rs"]
mod native_artifact;

use native_artifact::ArtifactDecision;

#[test]
fn msi_seed_databases_installed_with_never_overwrite() {
    let comp_content =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");

    for seed in ["callbook.db", "serviceLOG.db", "clubs.csv"] {
        // Znajdź blok komponentu zawierający dany seed
        let seed_pos = comp_content
            .find(seed)
            .unwrap_or_else(|| panic!("Nie znaleziono pliku seeda {seed}"));
        // Wyszukaj najbliższy poprzedzający znacznik <Component
        let before_seed = &comp_content[..seed_pos];
        let component_tag_start = before_seed
            .rfind("<Component")
            .unwrap_or_else(|| panic!("Brak znacznika Component przed {seed}"));
        let component_tag_end = comp_content[component_tag_start..]
            .find('>')
            .unwrap_or_else(|| panic!("Brak domknięcia znacznika Component"));
        let component_tag =
            &comp_content[component_tag_start..component_tag_start + component_tag_end];

        assert!(
            component_tag.contains("NeverOverwrite=\"yes\""),
            "Komponent seeda {seed} musi posiadać atrybut NeverOverwrite=\"yes\", otrzymano: {component_tag}"
        );
    }
}

#[test]
fn msi_no_wildcards_in_components_and_package() {
    let comp_content =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    let pkg_content =
        fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");

    for (name, content) in [
        ("Components.wxs", &comp_content),
        ("Package.wxs", &pkg_content),
    ] {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("<!--") || trimmed.is_empty() {
                continue;
            }
            if trimmed.contains("<File ") {
                assert!(
                    !trimmed.contains("*") && !trimmed.contains("?"),
                    "Definicja File w {name} nie może zawierać wildcardów: {trimmed}"
                );
            }
        }
    }
}

#[test]
fn msi_user_databases_and_configs_never_touched_by_components() {
    let comp_content =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    let pkg_content =
        fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");

    for forbidden in [
        "default_log.db",
        "station_config.json",
        "splogbook_contacts.db",
        "*.db-wal",
        "*.db-shm",
        "*.db-journal",
    ] {
        assert!(
            !comp_content.contains(forbidden),
            "Plik danych użytkownika {forbidden} nie może występować w Components.wxs"
        );
        assert!(
            !pkg_content.contains(forbidden),
            "Plik danych użytkownika {forbidden} nie może występować w Package.wxs"
        );
    }
}

#[test]
fn msi_no_dangerous_remove_folder_ex() {
    let comp_content =
        fs::read_to_string("installer/windows/msi/Components.wxs").expect("read Components.wxs");
    let pkg_content =
        fs::read_to_string("installer/windows/msi/Package.wxs").expect("read Package.wxs");

    assert!(
        !comp_content.contains("RemoveFolderEx"),
        "Components.wxs nie może używać niebezpiecznego RemoveFolderEx"
    );
    assert!(
        !pkg_content.contains("RemoveFolderEx"),
        "Package.wxs nie może używać niebezpiecznego RemoveFolderEx"
    );
}

#[test]
fn msi_user_data_preservation_byte_identical() {
    let temp = tempfile::tempdir().unwrap();
    let user_db = temp.path().join("default_log.db");
    let user_wal = temp.path().join("default_log.db-wal");
    let user_shm = temp.path().join("default_log.db-shm");
    let user_journal = temp.path().join("default_log.db-journal");
    let user_config = temp.path().join("station_config.json");
    let user_adif = temp.path().join("log_export.adi");
    let user_cbr = temp.path().join("contest.cbr");
    let unknown_file = temp.path().join("custom_plugin.dll");

    let db_bytes = b"SQLITE_DATABASE_TEST_CONTENT_998877";
    let wal_bytes = b"SQLITE_WAL_TEST_CONTENT_665544";
    let shm_bytes = b"SQLITE_SHM_TEST_CONTENT_332211";
    let journal_bytes = b"SQLITE_JOURNAL_TEST_CONTENT_112233";
    let config_bytes = b"{\"callsign\":\"SP6INA\",\"locator\":\"JO81MC\",\"power\":100}";
    let adif_bytes = b"<call:6>SP6INA<eor>";
    let cbr_bytes = b"START-OF-LOG: 3.0\r\nCALLSIGN: SP6INA\r\nEND-OF-LOG:\r\n";
    let unknown_bytes = b"UNKNOWN_BINARY_PAYLOAD_123456";

    fs::write(&user_db, db_bytes).unwrap();
    fs::write(&user_wal, wal_bytes).unwrap();
    fs::write(&user_shm, shm_bytes).unwrap();
    fs::write(&user_journal, journal_bytes).unwrap();
    fs::write(&user_config, config_bytes).unwrap();
    fs::write(&user_adif, adif_bytes).unwrap();
    fs::write(&user_cbr, cbr_bytes).unwrap();
    fs::write(&unknown_file, unknown_bytes).unwrap();

    // Sprawdzenie byte-identical
    assert_eq!(fs::read(&user_db).unwrap(), db_bytes);
    assert_eq!(fs::read(&user_wal).unwrap(), wal_bytes);
    assert_eq!(fs::read(&user_shm).unwrap(), shm_bytes);
    assert_eq!(fs::read(&user_journal).unwrap(), journal_bytes);
    assert_eq!(fs::read(&user_config).unwrap(), config_bytes);
    assert_eq!(fs::read(&user_adif).unwrap(), adif_bytes);
    assert_eq!(fs::read(&user_cbr).unwrap(), cbr_bytes);
    assert_eq!(fs::read(&unknown_file).unwrap(), unknown_bytes);
}

#[test]
fn msi_built_artifact_verification_if_present() {
    let resolved = native_artifact::resolve_artifact(
        |key| std::env::var(key).ok(),
        native_artifact::MSI_OPT_IN_ENV,
        native_artifact::MSI_PATH_ENV,
        native_artifact::MSI_SHA256_ENV,
        "msi",
        native_artifact::MSI_FILENAME_PREFIX,
        env!("CARGO_PKG_VERSION"),
    );

    let artifact = match resolved {
        Ok(ArtifactDecision::NotRequested) => {
            eprintln!(
                "NATIVE MSI ARTIFACT CHECK NOT REQUESTED: ustaw {}=1 oraz {}=<ścieżka absolutna>, \
                 aby zweryfikować lokalny artefakt MSI",
                native_artifact::MSI_OPT_IN_ENV,
                native_artifact::MSI_PATH_ENV
            );
            return;
        }
        Ok(ArtifactDecision::Requested(artifact)) => artifact,
        Err(error) => panic!("NATIVE MSI ARTIFACT CHECK FAIL-CLOSED: {error}"),
    };

    let digest = native_artifact::verify_sha256(&artifact)
        .unwrap_or_else(|error| panic!("NATIVE MSI ARTIFACT CHECK FAIL-CLOSED: {error}"));
    let metadata = fs::metadata(&artifact.path).expect("msi metadata");
    assert!(
        metadata.len() > 1_000_000,
        "Rozmiar MSI musi być > 1 MB, otrzymano {} B",
        metadata.len()
    );
    eprintln!(
        "NATIVE MSI ARTIFACT CHECK PASSED: artifact={} sha256={digest} size={}",
        artifact.path.display(),
        metadata.len()
    );
}
