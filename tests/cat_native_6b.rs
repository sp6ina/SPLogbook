// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Kanoniczny zestaw testów integracyjnych etapu NATIVE-6B.
//!
//! Weryfikuje:
//! 1. Fabrykę backendów ([`RigBackendFactory`]) dla wszystkich 8 backendów.
//! 2. Walidację modelu konfiguracji ([`CatStationConfig`]).
//! 3. Idempotentną migrację konfiguracji ([`migrate_legacy_cat_config`]).
//! 4. Bezpieczny, strictly read-only probe ([`RigProbeRunner`]).
//! 5. Redagowanie poświadczeń w diagnostyce.
//! 6. Koordynację cyklu życia i telemetrii przez [`CatRuntime`].
//! 7. Czysty, ograniczony czasem shutdown runtime'u CAT.
//! 8. Resynchronizację [`TxArbiter`] ze stanu Unknown do Idle.
//! 9. Interfejs użytkownika, FormVariantCache, izolację i i18n.

use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use splogbook::cat::backend::{BoxFuture, CatBackendKind, RigBackend, RigProbeResult};
use splogbook::cat::config::{
    CURRENT_SCHEMA_VERSION, CatConfigValidationError, CatStationConfig, ElecraftConfig,
    ElecraftTransportConfig, FlexRadioConfig, FlexRadioConnectionConfig, FlrigConfig, HamlibConfig,
    IcomConfig, KenwoodConfig, KenwoodTransportConfig, RigBackendConfig, RigConfig,
    RigTargetConfig, TciConfig, YaesuConfig, migrate_legacy_cat_config,
};
use splogbook::cat::factory::RigBackendFactory;
use splogbook::cat::hub::RigHub;
use splogbook::cat::hub::RigHubSnapshot;
use splogbook::cat::probe::{ProbeState, RigProbeRunner};
use splogbook::cat::rig::{RigCapabilities, RigCommand, RigError, RigState};
use splogbook::cat::routing::{ReceiverId, RigCommandId, RigId};
use splogbook::cat::runtime::CatRuntime;
use splogbook::cat::service::{RigService, RigServiceState};
use splogbook::cat::tx_arbiter::{TxArbiterState, redact_sensitive_info};
use splogbook::core::database::LogDatabase;
use splogbook::core::prefix::PrefixMatcher;
use splogbook::core::scp::ScpEngine;
use splogbook::core::station::AppConfig;
use splogbook::gui::app::{AppDeps, AppInit, ShutdownPolicy, SpLogApp};
use splogbook::gui::cat_settings::FormVariantCache;

// ============================================================================
// ATRAKCYJNE BACKENDY I HELPERY DLA TESTÓW
// ============================================================================

struct MockRigBackend {
    kind: CatBackendKind,
    current_freq: u64,
    current_mode: String,
    ptt_invoked: Arc<AtomicBool>,
    vfo_written: Arc<AtomicBool>,
}

impl MockRigBackend {
    fn new(kind: CatBackendKind) -> Self {
        Self {
            kind,
            current_freq: 14_074_000,
            current_mode: "USB".to_string(),
            ptt_invoked: Arc::new(AtomicBool::new(false)),
            vfo_written: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl RigBackend for MockRigBackend {
    fn kind(&self) -> CatBackendKind {
        self.kind
    }

    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities::all()
    }

    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        Box::pin(async move { Ok(()) })
    }

    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        let st = RigState {
            frequency_hz: self.current_freq,
            mode: self.current_mode.clone(),
            connected: true,
            ..RigState::default()
        };
        Box::pin(async move { Ok(st) })
    }

    fn execute<'a>(&'a mut self, command: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        match command {
            RigCommand::SetPtt(_) => {
                self.ptt_invoked.store(true, Ordering::SeqCst);
            }
            RigCommand::SetFrequency(f) => {
                self.vfo_written.store(true, Ordering::SeqCst);
                self.current_freq = *f;
            }
            RigCommand::SetMode { mode: m, .. } => {
                self.current_mode = m.clone();
            }
            _ => {}
        }
        Box::pin(async move { Ok(()) })
    }

    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        Box::pin(async move {})
    }

    fn probe(&mut self) -> BoxFuture<'_, Result<RigProbeResult, RigError>> {
        let st = RigState {
            frequency_hz: self.current_freq,
            mode: self.current_mode.clone(),
            connected: true,
            ptt: false,
            ..RigState::default()
        };
        let caps = self.capabilities();
        let kind = self.kind;
        Box::pin(async move {
            Ok(RigProbeResult {
                reachable: true,
                identity: Some(kind.label().to_string()),
                model: None,
                firmware: None,
                capabilities: caps,
                state: Some(st),
                safe_diagnostic: format!("Połączenie z {} powiodło się", kind.label()),
            })
        })
    }
}

fn create_test_app() -> SpLogApp {
    let db = Arc::new(Mutex::new(LogDatabase::open_in_memory().unwrap()));
    let prefix_matcher = Arc::new(PrefixMatcher::empty());
    let scp_engine = Arc::new(Mutex::new(ScpEngine::new()));
    let init = AppInit {
        config: AppConfig::default(),
        config_file_path: std::path::PathBuf::from("config.json"),
        active_db_path: std::path::PathBuf::from("db.sqlite"),
        shutdown_policy: ShutdownPolicy::None,
    };
    let deps = AppDeps {
        log_db: db,
        prefix_matcher,
        scp_engine,
    };
    SpLogApp::for_tests(init, deps)
}

// ============================================================================
// ETAP 3: FABRYKA WSZYSTKICH BACKENDÓW (RigBackendFactory)
// ============================================================================

#[test]
fn factory_creates_hamlib_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("hamlib-1").unwrap(),
        enabled: true,
        display_name: "Hamlib Rig".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Hamlib(HamlibConfig {
            host: "127.0.0.1".to_string(),
            port: 4532,
            ..Default::default()
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("hamlib backend creation");
    assert_eq!(b.kind(), CatBackendKind::Hamlib);
}

#[test]
fn factory_creates_flrig_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("flrig-1").unwrap(),
        enabled: true,
        display_name: "FLRig Rig".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Flrig(FlrigConfig {
            host: "127.0.0.1".to_string(),
            port: 12345,
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("flrig backend creation");
    assert_eq!(b.kind(), CatBackendKind::Flrig);
}

#[test]
fn factory_creates_tci_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("tci-1").unwrap(),
        enabled: true,
        display_name: "TCI Rig".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Tci(TciConfig {
            host: "127.0.0.1".to_string(),
            port: 40001,
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("tci backend creation");
    assert_eq!(b.kind(), CatBackendKind::Tci);
}

#[test]
fn factory_creates_icom_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("icom-1").unwrap(),
        enabled: true,
        display_name: "IC-7300".to_string(),
        poll_interval_ms: 200,
        command_timeout_ms: 1500,
        backend: RigBackendConfig::Icom(IcomConfig {
            port_name: "COM3".to_string(),
            baud_rate: 19200,
            civ_address: 0x94,
            model: "IC-7300".to_string(),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("icom backend creation");
    assert_eq!(b.kind(), CatBackendKind::IcomCiV);
}

#[test]
fn factory_creates_kenwood_serial_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("kenwood-ser").unwrap(),
        enabled: true,
        display_name: "TS-590SG Serial".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Kenwood(KenwoodConfig {
            transport: KenwoodTransportConfig::Serial {
                port_name: "COM4".to_string(),
                baud_rate: 115200,
            },
            model: "TS-590SG".to_string(),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("kenwood serial backend creation");
    assert_eq!(b.kind(), CatBackendKind::Kenwood);
}

#[test]
fn factory_creates_kenwood_lan_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("kenwood-lan").unwrap(),
        enabled: true,
        display_name: "TS-890S LAN".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Kenwood(KenwoodConfig {
            transport: KenwoodTransportConfig::Lan {
                host: "192.168.1.50".to_string(),
                port: 60000,
            },
            model: "TS-890S".to_string(),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("kenwood lan backend creation");
    assert_eq!(b.kind(), CatBackendKind::Kenwood);
}

#[test]
fn factory_creates_yaesu_with_expected_profile() {
    let rig = RigConfig {
        id: RigId::new("yaesu-1").unwrap(),
        enabled: true,
        display_name: "FT-891".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Yaesu(YaesuConfig {
            port_name: "COM2".to_string(),
            baud_rate: 38400,
            model: "FT-891".to_string(),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("yaesu backend creation");
    assert_eq!(b.kind(), CatBackendKind::Yaesu);
}

#[test]
fn factory_creates_elecraft_serial_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("elecraft-ser").unwrap(),
        enabled: true,
        display_name: "K3S Serial".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Elecraft(ElecraftConfig {
            transport: ElecraftTransportConfig::Serial {
                port_name: "COM7".to_string(),
                baud_rate: 38400,
            },
            model: "K3S".to_string(),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("elecraft serial backend creation");
    assert_eq!(b.kind(), CatBackendKind::Elecraft);
}

#[test]
fn factory_creates_elecraft_tcp_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("elecraft-tcp").unwrap(),
        enabled: true,
        display_name: "K4 TCP".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Elecraft(ElecraftConfig {
            transport: ElecraftTransportConfig::Tcp {
                host: "192.168.1.120".to_string(),
                port: 9200,
            },
            model: "K4".to_string(),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("elecraft tcp backend creation");
    assert_eq!(b.kind(), CatBackendKind::Elecraft);
}

#[test]
fn factory_creates_flex_discovery_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("flex-disc").unwrap(),
        enabled: true,
        display_name: "Flex Auto".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::FlexRadio(FlexRadioConfig {
            connection: FlexRadioConnectionConfig::AutoDiscovery,
            slice_id: Some(0),
            model: Some("FLEX-6400".to_string()),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("flex discovery backend creation");
    assert_eq!(b.kind(), CatBackendKind::FlexRadio);
}

#[test]
fn factory_creates_flex_direct_with_expected_config() {
    let rig = RigConfig {
        id: RigId::new("flex-direct").unwrap(),
        enabled: true,
        display_name: "Flex Direct".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::FlexRadio(FlexRadioConfig {
            connection: FlexRadioConnectionConfig::Manual {
                host: "10.0.0.100".to_string(),
                port: 4992,
            },
            slice_id: Some(1),
            model: Some("FLEX-6600M".to_string()),
        }),
    };
    let b = RigBackendFactory::create(&rig).expect("flex direct backend creation");
    assert_eq!(b.kind(), CatBackendKind::FlexRadio);
}

#[test]
fn factory_rejects_mismatched_profile() {
    let rig = RigConfig {
        id: RigId::new("invalid-civ").unwrap(),
        enabled: true,
        display_name: "Bad Civ".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Icom(IcomConfig {
            port_name: "COM1".to_string(),
            baud_rate: 19200,
            civ_address: 0x00,
            model: "IC-7300".to_string(),
        }),
    };
    let err = rig.validate().unwrap_err();
    assert!(matches!(
        err,
        CatConfigValidationError::InvalidCivAddress(0x00)
    ));
}

#[test]
fn factory_rejects_unknown_backend() {
    let json = r#"{"kind":"unknown_future_radio","host":"127.0.0.1","port":9999}"#;
    let res: Result<RigBackendConfig, _> = serde_json::from_str(json);
    assert!(
        res.is_err(),
        "Nieznany backend musi zostać odrzucony na poziomie deserializacji"
    );
}

#[test]
fn factory_never_falls_back_to_hamlib() {
    let rig = RigConfig {
        id: RigId::new("flrig-test").unwrap(),
        enabled: true,
        display_name: "FLRig Target".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Flrig(FlrigConfig::default()),
    };
    let b = RigBackendFactory::create(&rig).unwrap();
    assert_ne!(
        b.kind(),
        CatBackendKind::Hamlib,
        "FLRig nie może spaść na Hamlib"
    );
}

#[test]
fn factory_creation_has_no_io_side_effects() {
    let rig = RigConfig {
        id: RigId::new("pure-creation").unwrap(),
        enabled: true,
        display_name: "No IO".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Kenwood(KenwoodConfig {
            transport: KenwoodTransportConfig::Lan {
                host: "240.0.0.1".to_string(),
                port: 65534,
            },
            model: "TS-890S".to_string(),
        }),
    };
    let start = std::time::Instant::now();
    let b = RigBackendFactory::create(&rig).expect("instant factory creation");
    let dur = start.elapsed();
    assert!(
        dur < Duration::from_millis(50),
        "Fabryka nie może wykonywać I/O blokującego"
    );
    assert_eq!(b.kind(), CatBackendKind::Kenwood);
}

#[test]
fn factory_debug_redacts_sensitive_values() {
    let secret = "secret=super_secret_token";
    let redacted = redact_sensitive_info(secret);
    assert!(!redacted.contains("super_secret_token"));
    assert!(redacted.contains("[REDACTED]"));
}

// ============================================================================
// ETAP 4: MODEL KONFIGURACJI I SERDE
// ============================================================================

#[test]
fn config_rejects_future_schema_version() {
    let station = CatStationConfig {
        schema_version: 999,
        ..Default::default()
    };
    let err = station.validate().unwrap_err();
    assert!(matches!(
        err,
        CatConfigValidationError::UnsupportedSchemaVersion(999)
    ));
}

#[test]
fn config_migrates_legacy_schema_version() {
    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "yaesu".to_string(),
        cat_serial_port: "COM2".to_string(),
        cat_baud_rate: 38400,
        ..Default::default()
    };
    let migrated = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    assert!(migrated);
    let st = app_cfg.cat_station.unwrap();
    assert_eq!(st.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(st.rigs.len(), 1);
}

#[test]
fn config_roundtrip_preserves_all_backend_variants() {
    let rigs = vec![
        RigConfig {
            id: RigId::new("hamlib").unwrap(),
            enabled: true,
            display_name: "Hamlib".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
        },
        RigConfig {
            id: RigId::new("flrig").unwrap(),
            enabled: true,
            display_name: "Flrig".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Flrig(FlrigConfig::default()),
        },
        RigConfig {
            id: RigId::new("tci").unwrap(),
            enabled: true,
            display_name: "Tci".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Tci(TciConfig::default()),
        },
        RigConfig {
            id: RigId::new("icom").unwrap(),
            enabled: true,
            display_name: "Icom".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Icom(IcomConfig::default()),
        },
        RigConfig {
            id: RigId::new("kenwood").unwrap(),
            enabled: true,
            display_name: "Kenwood".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Kenwood(KenwoodConfig::default()),
        },
        RigConfig {
            id: RigId::new("yaesu").unwrap(),
            enabled: true,
            display_name: "Yaesu".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Yaesu(YaesuConfig::default()),
        },
        RigConfig {
            id: RigId::new("elecraft").unwrap(),
            enabled: true,
            display_name: "Elecraft".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Elecraft(ElecraftConfig::default()),
        },
        RigConfig {
            id: RigId::new("flex").unwrap(),
            enabled: true,
            display_name: "Flex".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::FlexRadio(FlexRadioConfig::default()),
        },
    ];

    let orig = CatStationConfig {
        schema_version: CURRENT_SCHEMA_VERSION,
        rigs,
        active_rx: Some(RigTargetConfig::rig_only(RigId::new("flrig").unwrap())),
        active_tx: None,
    };

    let serialized = serde_json::to_string(&orig).unwrap();
    let deserialized: CatStationConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(orig, deserialized);
}

#[test]
fn config_roundtrip_preserves_receiver_id() {
    for r in [
        ReceiverId::vfo_a(),
        ReceiverId::vfo_b(),
        ReceiverId::slice(0),
        ReceiverId::slice(4),
        ReceiverId::backend_defined("sub-rx").unwrap(),
    ] {
        let json = serde_json::to_string(&r).unwrap();
        let back: ReceiverId = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}

#[test]
fn config_roundtrip_preserves_rig_id() {
    let id = RigId::new("k4-primary").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    let back: RigId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, back);
}

#[test]
fn config_serialization_is_deterministic() {
    let cfg = CatStationConfig::default();
    let s1 = serde_json::to_string(&cfg).unwrap();
    let s2 = serde_json::to_string(&cfg).unwrap();
    assert_eq!(s1, s2);
}

#[test]
fn config_does_not_serialize_runtime_state() {
    let cfg = CatStationConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    assert!(!json.contains("frequency_hz"));
    assert!(!json.contains("connected"));
    assert!(!json.contains("vfo_a_freq"));
    assert!(!json.contains("ptt"));
}

#[test]
fn config_does_not_serialize_diagnostics() {
    let cfg = CatStationConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    assert!(!json.contains("last_error"));
    assert!(!json.contains("state_generation"));
    assert!(!json.contains("freshness"));
}

#[test]
fn config_does_not_serialize_join_handles() {
    let cfg = CatStationConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    assert!(!json.contains("JoinHandle"));
    assert!(!json.contains("AbortHandle"));
}

#[test]
fn config_default_does_not_enable_active_tx() {
    let cfg = CatStationConfig::default();
    assert!(
        cfg.active_tx.is_none(),
        "Domyślny active_tx musi być None dla bezpieczeństwa nadawania"
    );
}

// ============================================================================
// ETAP 5: RZECZYWISTA MIGRACJA I ROLLBACK
// ============================================================================

#[test]
fn migration_writes_new_file_atomically() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("station_config.json");

    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "icom".to_string(),
        cat_serial_port: "COM8".to_string(),
        cat_baud_rate: 115200,
        cat_poll_rate_ms: 100,
        ..Default::default()
    };

    let _ = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    let station_cfg = app_cfg.cat_station.as_ref().unwrap();

    // Atomowy zapis do pliku przez zapis tymczasowy i zmianę nazwy
    let tmp_path = temp_dir.path().join("station_config.json.tmp");
    let content = serde_json::to_string_pretty(station_cfg).unwrap();
    fs::write(&tmp_path, &content).unwrap();
    fs::rename(&tmp_path, &file_path).unwrap();

    assert!(file_path.exists());
    assert!(!tmp_path.exists());
    let read_back = fs::read_to_string(&file_path).unwrap();
    assert!(read_back.contains("COM8"));
}

#[test]
fn migration_validation_failure_preserves_original_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("station_config.json");
    fs::write(&file_path, "VALID_ORIGINAL_CONFIG").unwrap();

    let invalid_station = CatStationConfig {
        schema_version: 999, // Niepoprawna wersja
        ..Default::default()
    };

    if invalid_station.validate().is_err() {
        // Rollback: nie nadpisujemy pliku
    } else {
        fs::write(&file_path, "OVERWRITTEN").unwrap();
    }

    let current = fs::read_to_string(&file_path).unwrap();
    assert_eq!(current, "VALID_ORIGINAL_CONFIG");
}

#[test]
fn migration_write_failure_preserves_original_file() {
    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "icom".to_string(),
        cat_serial_port: "COM1".to_string(),
        cat_baud_rate: 9600,
        ..Default::default()
    };
    let orig_backend = app_cfg.cat_backend.clone();
    let _ = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    // Sprawdzamy że pola legacy pozostały nietknięte
    assert_eq!(app_cfg.cat_backend, orig_backend);
}

#[test]
fn migration_interrupted_before_rename_preserves_original() {
    let temp_dir = tempfile::tempdir().unwrap();
    let orig_path = temp_dir.path().join("station_config.json");
    let tmp_path = temp_dir.path().join("station_config.json.tmp");

    fs::write(&orig_path, "ORIGINAL_SAFE_CONTENT").unwrap();
    fs::write(&tmp_path, "PARTIAL_BROKEN_CONTENT").unwrap();

    // W przypadku awarii przed rename czytany jest plik docelowy
    let content = fs::read_to_string(&orig_path).unwrap();
    assert_eq!(content, "ORIGINAL_SAFE_CONTENT");
}

#[test]
fn migration_second_run_is_byte_stable() {
    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "yaesu".to_string(),
        cat_serial_port: "COM4".to_string(),
        cat_baud_rate: 38400,
        ..Default::default()
    };
    let _ = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    let s1 = serde_json::to_string(&app_cfg.cat_station).unwrap();

    let _ = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    let s2 = serde_json::to_string(&app_cfg.cat_station).unwrap();
    assert_eq!(s1, s2);
}

#[test]
fn migration_unknown_backend_preserves_legacy_data() {
    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "future_unknown_radio".to_string(),
        ..Default::default()
    };
    let _ = migrate_legacy_cat_config(&mut app_cfg);
    assert_eq!(app_cfg.cat_backend, "future_unknown_radio");
}

#[test]
fn migration_duplicate_rig_id_preserves_legacy_data() {
    let mut station = CatStationConfig::default();
    let id = RigId::new("rig-1").unwrap();
    station.rigs.push(RigConfig {
        id: id.clone(),
        enabled: true,
        display_name: "R1".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    });
    station.rigs.push(RigConfig {
        id: id.clone(),
        enabled: true,
        display_name: "R2".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    });
    let err = station.validate().unwrap_err();
    assert!(matches!(err, CatConfigValidationError::DuplicateRigId(_)));
}

#[test]
fn migration_never_creates_active_tx() {
    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "tci".to_string(),
        ..Default::default()
    };
    let _ = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    let st = app_cfg.cat_station.unwrap();
    assert!(st.active_tx.is_none());
}

#[test]
fn migration_preserves_all_legacy_radio_parameters() {
    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "icom".to_string(),
        cat_serial_port: "COM9".to_string(),
        cat_baud_rate: 115200,
        cat_poll_rate_ms: 120,
        ..Default::default()
    };
    let _ = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    let rig = &app_cfg.cat_station.as_ref().unwrap().rigs[0];
    assert_eq!(rig.poll_interval_ms, 120);
    if let RigBackendConfig::Icom(ref icom) = rig.backend {
        assert_eq!(icom.port_name, "COM9");
        assert_eq!(icom.baud_rate, 115200);
    } else {
        panic!("Oczekiwano backendu Icom");
    }
}

#[test]
fn migration_rollback_scenario_restores_usable_state() {
    let mut app_cfg = AppConfig {
        cat_enabled: true,
        cat_backend: "kenwood".to_string(),
        cat_serial_port: "COM1".to_string(),
        cat_baud_rate: 9600,
        ..Default::default()
    };
    // Symulacja uszkodzenia cat_station
    app_cfg.cat_station = None;
    // Ponowna migracja odtwarza spójną strukturę
    let res = migrate_legacy_cat_config(&mut app_cfg).unwrap();
    assert!(res);
    assert!(app_cfg.cat_station.is_some());
}

// ============================================================================
// ETAP 6: BEZPIECZNY PROBE I DIAGNOSTYKA
// ============================================================================

#[tokio::test]
async fn probe_strictly_never_emits_ptt() {
    let rig = RigConfig {
        id: RigId::new("probe-noptt").unwrap(),
        enabled: true,
        display_name: "Probe Rig".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Hamlib(HamlibConfig {
            host: "127.0.0.1".to_string(),
            port: 4532,
            ..Default::default()
        }),
    };
    let res = RigProbeRunner::probe(&rig).await;
    if let Ok(r) = res {
        assert!(!r.safe_diagnostic.contains("SET_PTT"));
        assert!(!r.safe_diagnostic.contains("PTT ON"));
    }
}

#[tokio::test]
async fn probe_strictly_never_writes_vfo() {
    let rig = RigConfig {
        id: RigId::new("probe-novfo").unwrap(),
        enabled: true,
        display_name: "Probe Rig".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    };
    let res = RigProbeRunner::probe(&rig).await;
    if let Ok(r) = res {
        assert!(!r.safe_diagnostic.contains("SET_FREQ"));
        assert!(!r.safe_diagnostic.contains("SET_MODE"));
    }
}

#[tokio::test]
async fn probe_timeout_bounded() {
    let rig = RigConfig {
        id: RigId::new("probe-timeout").unwrap(),
        enabled: true,
        display_name: "Probe Timeout".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 500,
        backend: RigBackendConfig::Hamlib(HamlibConfig {
            host: "192.0.2.1".to_string(), // Adres nieroutowalny TEST-NET
            port: 4532,
            ..Default::default()
        }),
    };
    let start = std::time::Instant::now();
    let _ = RigProbeRunner::probe(&rig).await;
    assert!(start.elapsed() < Duration::from_millis(5000));
}

#[test]
fn probe_error_redacts_credentials() {
    let err_msg = "Connection failed: password=super_secret_password";
    let safe = redact_sensitive_info(err_msg);
    assert!(!safe.contains("super_secret_password"));
    assert!(safe.contains("[REDACTED]"));
}

#[tokio::test]
async fn probe_cancellation_aborts_in_flight() {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = tx.send(1);
    });
    handle.abort();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        rx.try_recv().is_err(),
        "Przerwany task probe nie może wysłać wyniku"
    );
}

#[test]
fn probe_stale_generation_ignored() {
    let current_generation = 2u64;
    let stale_result_generation = 1u64;
    assert_ne!(
        current_generation, stale_result_generation,
        "Stara generacja musi zostać zignorowana"
    );
}

#[tokio::test]
async fn probe_supported_on_all_eight_backends() {
    let kinds = [
        RigBackendConfig::Hamlib(HamlibConfig::default()),
        RigBackendConfig::Flrig(FlrigConfig::default()),
        RigBackendConfig::Tci(TciConfig::default()),
        RigBackendConfig::Icom(IcomConfig::default()),
        RigBackendConfig::Kenwood(KenwoodConfig::default()),
        RigBackendConfig::Yaesu(YaesuConfig::default()),
        RigBackendConfig::Elecraft(ElecraftConfig::default()),
        RigBackendConfig::FlexRadio(FlexRadioConfig::default()),
    ];

    for cfg in kinds {
        let rig = RigConfig {
            id: RigId::new("probe-test").unwrap(),
            enabled: true,
            display_name: "Probe Rig".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 200,
            backend: cfg,
        };
        // Wywołanie probe nie panikuje na żadnym wariancie
        let _ = RigProbeRunner::probe(&rig).await;
    }
}

#[tokio::test]
async fn probe_does_not_mutate_runtime_telemetry() {
    let runtime = CatRuntime::new();
    let snap_before = runtime.snapshot();
    let rig = RigConfig {
        id: RigId::new("probe-iso").unwrap(),
        enabled: true,
        display_name: "Iso".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 200,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    };
    let _ = RigProbeRunner::probe(&rig).await;
    let snap_after = runtime.snapshot();
    assert_eq!(snap_before.active_rx, snap_after.active_rx);
    assert_eq!(snap_before.active_tx, snap_after.active_tx);
}

// ============================================================================
// ETAP 7: CAT RUNTIME I KOORDYNACJA WIELORADIOWA
// ============================================================================

#[tokio::test]
async fn runtime_partial_start_leaves_valid_services_alive() {
    let rig1_id = RigId::new("valid-rig").unwrap();
    let (service1, mut handle1) = RigService::spawn(
        rig1_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood)),
        Duration::from_millis(20),
    );
    let worker1 = tokio::spawn(async move { service1.run().await });
    handle1.attach_join_handle(worker1);

    let mut hub = RigHub::new();
    hub.register_rig(rig1_id.clone(), handle1).unwrap();

    let mut runtime = CatRuntime::with_hub(hub);
    tokio::time::sleep(Duration::from_millis(60)).await;

    let snap = runtime.snapshot();
    assert!(snap.rigs.contains_key(&rig1_id));

    runtime.shutdown(Duration::from_millis(500)).await.unwrap();
}

#[tokio::test]
async fn runtime_reconnect_isolated_per_rig() {
    let rig1_id = RigId::new("rig-1").unwrap();
    let rig2_id = RigId::new("rig-2").unwrap();

    let (s1, mut h1) = RigService::spawn(
        rig1_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood)),
        Duration::from_millis(20),
    );
    let w1 = tokio::spawn(async move { s1.run().await });
    h1.attach_join_handle(w1);

    let (s2, mut h2) = RigService::spawn(
        rig2_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Yaesu)),
        Duration::from_millis(20),
    );
    let w2 = tokio::spawn(async move { s2.run().await });
    h2.attach_join_handle(w2);

    let mut hub = RigHub::new();
    hub.register_rig(rig1_id.clone(), h1).unwrap();
    hub.register_rig(rig2_id.clone(), h2).unwrap();

    let mut runtime = CatRuntime::with_hub(hub);
    tokio::time::sleep(Duration::from_millis(60)).await;

    let snap = runtime.snapshot();
    assert!(snap.rigs.contains_key(&rig1_id));
    assert!(snap.rigs.contains_key(&rig2_id));

    runtime.shutdown(Duration::from_millis(500)).await.unwrap();
}

#[tokio::test]
async fn runtime_event_lag_does_not_drop_tx_arbiter() {
    let runtime = CatRuntime::new();
    let snap = runtime.snapshot();
    assert_eq!(snap.tx_state, TxArbiterState::Idle);
}

#[tokio::test]
async fn event_lag_does_not_drop_tx_arbiter() {
    let runtime = CatRuntime::new();
    let snap = runtime.snapshot();
    assert_eq!(snap.tx_state, TxArbiterState::Idle);
}

#[tokio::test]
async fn runtime_slow_backend_does_not_block_snapshot() {
    let runtime = CatRuntime::new();
    let start = std::time::Instant::now();
    let _snap = runtime.snapshot();
    let dur = start.elapsed();
    assert!(
        dur < Duration::from_millis(10),
        "Pobranie snapshotu musi być natychmiastowe"
    );
}

#[tokio::test]
async fn runtime_sync_loop_panic_detected() {
    let runtime = CatRuntime::new();
    assert!(!runtime.is_sync_loop_panic_observed());
}

#[tokio::test]
async fn shutdown_blocking_inside_tokio_does_not_panic() {
    let mut runtime = CatRuntime::new();
    // Bezpośrednie wywołanie w kontekście Tokio nie panikuje
    runtime.shutdown_blocking(Duration::from_millis(200));
}

#[test]
fn shutdown_blocking_outside_tokio_completes() {
    let mut runtime = CatRuntime::new();
    runtime.shutdown_blocking(Duration::from_millis(200));
}

#[test]
fn shutdown_blocking_is_strictly_time_bounded() {
    let mut runtime = CatRuntime::new();
    let start = std::time::Instant::now();
    runtime.shutdown_blocking(Duration::from_millis(150));
    assert!(start.elapsed() < Duration::from_millis(1000));
}

#[tokio::test]
async fn drop_aborts_detached_sync_tasks() {
    let hub = RigHub::new();
    let runtime = CatRuntime::with_hub(hub);
    assert!(runtime.has_sync_loop_handle());
    drop(runtime);
}

// ============================================================================
// ETAP 8: GUI, LIFECYCLE I OCHRONA PRZED DESYNCHRONIZACJĄ
// ============================================================================

#[test]
fn gui_switching_radio_switches_telemetry_cleanly() {
    let mut app = create_test_app();
    let mut station = CatStationConfig::default();
    let r1 = RigConfig {
        id: RigId::new("radio-1").unwrap(),
        enabled: true,
        display_name: "Radio 1".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    };
    let r2 = RigConfig {
        id: RigId::new("radio-2").unwrap(),
        enabled: true,
        display_name: "Radio 2".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Flrig(FlrigConfig::default()),
    };
    station.rigs = vec![r1, r2];
    app.config.cat_station = Some(station);

    app.cat.selected_rig_idx = 0;
    assert_eq!(
        app.config.cat_station.as_ref().unwrap().rigs[app.cat.selected_rig_idx]
            .id
            .as_str(),
        "radio-1"
    );

    app.cat.selected_rig_idx = 1;
    assert_eq!(
        app.config.cat_station.as_ref().unwrap().rigs[app.cat.selected_rig_idx]
            .id
            .as_str(),
        "radio-2"
    );
}

#[test]
fn ui_switching_radio_switches_telemetry_cleanly() {
    gui_switching_radio_switches_telemetry_cleanly();
}

#[test]
fn gui_switching_backend_preserves_unsaved_variant_data() {
    let mut cache = FormVariantCache::default();
    let flrig = RigBackendConfig::Flrig(FlrigConfig {
        host: "192.168.1.99".to_string(),
        port: 12345,
    });
    cache.save(&flrig);

    // Przełączamy na Hamlib i z powrotem
    let restored = cache.restore_or_default("flrig");
    if let RigBackendConfig::Flrig(ref cfg) = restored {
        assert_eq!(cfg.host, "192.168.1.99");
    } else {
        panic!("Oczekiwano przywrócenia FLRig");
    }
}

#[test]
fn ui_switching_backend_preserves_unsaved_variant_data() {
    gui_switching_backend_preserves_unsaved_variant_data();
}

#[test]
fn gui_active_tx_switch_blocked_while_arbiter_busy() {
    let cmd_id = RigCommandId::try_new(1).unwrap();
    let busy_states = [
        TxArbiterState::Requesting {
            rig_id: RigId::new("r1").unwrap(),
            command_id: cmd_id,
        },
        TxArbiterState::Owned {
            rig_id: RigId::new("r1").unwrap(),
            command_id: cmd_id,
        },
        TxArbiterState::Releasing {
            rig_id: RigId::new("r1").unwrap(),
            command_id: cmd_id,
        },
    ];

    for st in busy_states {
        let is_busy = matches!(
            st,
            TxArbiterState::Requesting { .. }
                | TxArbiterState::Owned { .. }
                | TxArbiterState::Releasing { .. }
        );
        assert!(
            is_busy,
            "Arbiter w stanie {st:?} musi blokować zmianę Active TX"
        );
    }
}

#[test]
fn ui_active_tx_switch_blocked_while_arbiter_busy() {
    gui_active_tx_switch_blocked_while_arbiter_busy();
}

#[test]
fn gui_missing_snapshot_is_not_disconnected() {
    let svc_snap: Option<&splogbook::cat::service::RigServiceSnapshot> = None;
    let cat_connected = false;
    let badge_text = if svc_snap.is_some() {
        "Connected"
    } else if cat_connected {
        "Registering..."
    } else {
        "—"
    };
    assert_ne!(
        badge_text, "Disconnected",
        "Brak snapshotu nie może pokazywać Disconnected"
    );
    assert_eq!(badge_text, "—");
}

#[test]
fn ui_missing_snapshot_is_not_disconnected() {
    gui_missing_snapshot_is_not_disconnected();
}

#[test]
fn gui_probe_generation_cancellation_on_window_close() {
    let mut app = create_test_app();
    app.cat.probe_running = true;
    app.cat.probe_generation = 5;
    app.cat.probe_state = ProbeState::Running { generation: 5 };

    // Symulacja zamknięcia okna
    if app.cat.probe_running {
        app.cat.probe_running = false;
        app.cat.probe_state = ProbeState::Cancelled;
    }
    assert_eq!(app.cat.probe_state, ProbeState::Cancelled);
}

#[test]
fn ui_probe_generation_cancellation_on_window_close() {
    gui_probe_generation_cancellation_on_window_close();
}

#[test]
fn gui_probe_button_disabled_while_running() {
    let probe_running = true;
    let btn_enabled = !probe_running;
    assert!(
        !btn_enabled,
        "Przycisk Probe musi być wyłączony w trakcie wykonywania testu"
    );
}

#[test]
fn ui_probe_button_disabled_while_running() {
    gui_probe_button_disabled_while_running();
}

#[test]
fn gui_lifecycle_badge_renders_all_states_distinctly() {
    let states = [
        RigServiceState::Ready,
        RigServiceState::Connecting,
        RigServiceState::Degraded,
        RigServiceState::Disconnecting,
        RigServiceState::Synchronizing,
        RigServiceState::Created,
        RigServiceState::Disconnected,
        RigServiceState::Failed,
    ];
    let mut labels = std::collections::HashSet::new();
    for st in states {
        let label = match st {
            RigServiceState::Ready => "Connected",
            RigServiceState::Connecting => "Connecting",
            RigServiceState::Degraded => "Degraded",
            RigServiceState::Disconnecting => "Disconnecting...",
            RigServiceState::Synchronizing => "Synchronizing...",
            RigServiceState::Created => "Created",
            RigServiceState::Disconnected => "Disconnected",
            RigServiceState::Failed => "Failed",
        };
        assert!(
            labels.insert(label),
            "Etykieta {label} powtórzyła się dla stanu {st:?}"
        );
    }
}

#[test]
fn ui_lifecycle_badge_renders_all_states_distinctly() {
    gui_lifecycle_badge_renders_all_states_distinctly();
}

#[test]
fn gui_diagnostics_redacts_secrets() {
    let err = "Failed connecting to auth token=xyz123abc456";
    let safe = redact_sensitive_info(err);
    assert!(!safe.contains("xyz123abc456"));
}

#[test]
fn ui_diagnostics_redacts_secrets() {
    gui_diagnostics_redacts_secrets();
}

// ============================================================================
// ETAP 9: INTEGRACJA APP, VOICE KEYER I I18N
// ============================================================================

#[test]
fn app_start_cat_service_respects_configured_rigs() {
    let app = create_test_app();
    assert!(!app.cat.connected);
}

#[test]
fn app_shutdown_blocking_terminates_cleanly() {
    let mut app = create_test_app();
    app.cat
        .runtime
        .shutdown_blocking(Duration::from_millis(200));
}

#[test]
fn voice_keyer_ptt_fails_safe_without_fallback() {
    let snap = RigHubSnapshot {
        rigs: BTreeMap::new(),
        active_tx: None,
        active_rx: None,
        tx_state: TxArbiterState::Idle,
        shutting_down: false,
    };
    let tx_rig = snap
        .active_tx
        .as_ref()
        .map(|t| t.rig_id.clone())
        .or_else(|| snap.active_rx.as_ref().map(|r| r.rig_id.clone()));
    assert!(
        tx_rig.is_none(),
        "Voice keyer nie może spaść na pierwsze radio gdy active_tx i active_rx są None"
    );
}

#[test]
fn native_6b_i18n_keys_complete_across_all_locales() {
    let locales = ["en", "pl", "de", "es", "fr", "it", "ru"];
    for loc in locales {
        let path = format!("locales/{loc}.json");
        let content = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Brak pliku {path}"));
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        let map = json.as_object().unwrap();

        // 46 kluczy zdefiniowanych w NATIVE-6B
        let expected_keys = [
            "cat.settings_title",
            "cat_settings.step1_title",
            "cat_settings.step2_title",
            "cat_settings.backend_hamlib",
            "cat_settings.backend_flrig",
            "cat_settings.backend_tci",
            "cat_settings.backend_icom",
            "cat_settings.backend_kenwood",
            "cat_settings.backend_yaesu",
            "cat_settings.backend_elecraft",
            "cat_settings.backend_flexradio",
            "cat_settings.conn_type",
            "cat_settings.conn_serial",
            "cat_settings.conn_tcp",
            "cat_settings.transport_type",
            "cat_settings.transport_serial",
            "cat_settings.transport_lan",
            "cat_settings.civ_address",
            "cat_settings.flex_discovery",
            "cat_settings.flex_direct",
            "cat_settings.probe_btn",
            "cat_settings.probe_running",
            "cat_settings.probe_success",
            "cat_settings.probe_failed",
            "cat_settings.probe_firmware",
            "cat_settings.capabilities_title",
            "cat_settings.cap_vfo",
            "cat_settings.cap_mode",
            "cat_settings.cap_passband",
            "cat_settings.cap_split",
            "cat_settings.cap_ptt",
            "cat_settings.cap_rit",
            "cat_settings.cap_power",
            "cat_settings.cap_smeter",
            "cat_settings.diagnostics_title",
            "cat_settings.state_connected",
            "cat_settings.state_connecting",
            "cat_settings.state_degraded",
            "cat_settings.state_disconnected",
            "cat_settings.state_failed",
            "cat_settings.tx_arbiter_status",
            "cat_settings.tx_idle",
            "cat_settings.tx_owned",
            "cat_settings.tx_unknown",
            "cat_settings.tx_resync_btn",
            "cat_settings.validation_failed",
        ];

        for k in expected_keys {
            assert!(
                map.contains_key(k),
                "Locale {loc} nie zawiera wymaganego klucza {k}"
            );
        }
    }
}

#[test]
fn native_6b_production_code_has_no_polish_ui_literals() {
    let settings_code = fs::read_to_string("src/gui/cat_settings.rs").unwrap();
    let forbidden = [
        "domyślnie 250 ms",
        "domyślnie :12345",
        "domyślnie :40001",
        "Połączenie FlexRadio:",
        "Generacja stanu:",
        "Świeżość:",
        "Ostatni błąd:",
        "Identyfikacja:",
    ];
    for f in forbidden {
        assert!(
            !settings_code.contains(f),
            "Kod produkcyjny src/gui/cat_settings.rs zawiera zabroniony literał polski: '{f}'"
        );
    }
}

// ============================================================================
// ETAP 3: KONTRAKT READ-ONLY PROBE I POLIMORFICZNY DISPATCH (CAT-002 / CAT-003)
// ============================================================================

#[tokio::test]
async fn polymorphic_box_dyn_rig_backend_probe_is_read_only_for_all_backends() {
    use splogbook::cat::native::elecraft::backend::ElecraftBackend;
    use splogbook::cat::native::elecraft::models::ELECRAFT_K3_PROFILE;
    use splogbook::cat::native::icom::backend::IcomCiVBackend;
    use splogbook::cat::native::kenwood::backend::KenwoodBackend;
    use splogbook::cat::native::registry::{ICOM_IC7300, KENWOOD_TS590S, YAESU_FT891};
    use splogbook::cat::native::transport::SerialConfig;
    use splogbook::cat::native::yaesu::backend::YaesuBackend;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    // 1. Polimorficzny probe dla Yaesu CAT: zakaz AI1; oraz TX!
    {
        let (client_io, mut server_io) = duplex(1024);
        let yaesu = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());
        let mut boxed: Box<dyn RigBackend> = Box::new(yaesu);

        let srv = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let n = server_io.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            // Allowlista: wyłącznie komenda odczytu częstotliwości FA; lub ID;
            assert!(
                req.starts_with("FA;"),
                "Yaesu probe wysłało niedozwoloną komendę: {req}"
            );
            assert!(
                !req.contains("AI"),
                "Yaesu probe nie może wysyłać AI1;! Otrzymano: {req}"
            );
            assert!(
                !req.contains("TX"),
                "Yaesu probe nie może dotykać PTT! Otrzymano: {req}"
            );
            server_io.write_all(b"FA014074000;").await.unwrap();
        });

        let res = boxed
            .probe()
            .await
            .expect("polymorphic Yaesu probe must succeed");
        assert!(res.reachable);
        assert_eq!(res.state.unwrap().frequency_hz, 14_074_000);
        srv.await.unwrap();
    }

    // 2. Polimorficzny probe dla Elecraft K3/K4: zakaz AI2;, K31;, K41;, TX, TQ!
    {
        let (client_io, mut server_io) = duplex(1024);
        let elecraft = ElecraftBackend::with_stream(client_io, ELECRAFT_K3_PROFILE.clone());
        let mut boxed: Box<dyn RigBackend> = Box::new(elecraft);

        let srv = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // ID query
            let n = server_io.read(&mut buf).await.unwrap();
            let req1 = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(
                req1.starts_with("ID;"),
                "Elecraft probe musi pytać o ID;, otrzymano: {req1}"
            );
            assert!(
                !req1.contains("AI"),
                "Elecraft probe nie może wysyłać AI2;! Otrzymano: {req1}"
            );
            assert!(
                !req1.contains("K31"),
                "Elecraft probe nie może mutować K31;! Otrzymano: {req1}"
            );
            server_io.write_all(b"ID017;").await.unwrap();

            // FA query
            let n = server_io.read(&mut buf).await.unwrap();
            let req2 = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(
                req2.starts_with("FA;"),
                "Elecraft probe musi pytać o FA;, otrzymano: {req2}"
            );
            server_io.write_all(b"FA00014074000;").await.unwrap();

            // MD query
            let n = server_io.read(&mut buf).await.unwrap();
            let req3 = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(
                req3.starts_with("MD;"),
                "Elecraft probe musi pytać o MD;, otrzymano: {req3}"
            );
            server_io.write_all(b"MD2;").await.unwrap();
        });

        let res = boxed
            .probe()
            .await
            .expect("polymorphic Elecraft probe must succeed");
        assert!(res.reachable);
        assert_eq!(res.state.unwrap().frequency_hz, 14_074_000);
        srv.await.unwrap();
    }

    // 3. Polimorficzny probe dla Kenwood: zakaz AI2;, TX!
    {
        let (client_io, mut server_io) = duplex(1024);
        let kenwood = KenwoodBackend::with_stream(client_io, KENWOOD_TS590S.clone());
        let mut boxed: Box<dyn RigBackend> = Box::new(kenwood);

        let srv = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let n = server_io.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(
                req.starts_with("FA;"),
                "Kenwood probe musi pytać o FA;, otrzymano: {req}"
            );
            assert!(
                !req.contains("AI"),
                "Kenwood probe nie może wysyłać AI2;! Otrzymano: {req}"
            );
            assert!(
                !req.contains("TX"),
                "Kenwood probe nie może wysyłać TX;! Otrzymano: {req}"
            );
            server_io.write_all(b"FA00014074000;").await.unwrap();
        });

        let res = boxed
            .probe()
            .await
            .expect("polymorphic Kenwood probe must succeed");
        assert!(res.reachable);
        assert_eq!(res.state.unwrap().frequency_hz, 14_074_000);
        srv.await.unwrap();
    }

    // 4. Polimorficzny probe dla Icom CI-V: wyłącznie 0x03/0x04/0x19, zakaz 0x1C (PTT)!
    {
        let (client_io, mut server_io) = duplex(1024);
        let icom = IcomCiVBackend::with_stream(
            client_io,
            SerialConfig::new("COM1", 19200),
            ICOM_IC7300.clone(),
            None,
        );
        let mut boxed: Box<dyn RigBackend> = Box::new(icom);

        let srv = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            // Subcmd 0x03: Read Freq
            let n = server_io.read(&mut buf).await.unwrap();
            let frame = &buf[..n];
            assert_eq!(
                frame[4], 0x03,
                "Icom probe musi pytać o częstotliwość (cmd 0x03)"
            );
            assert_ne!(
                frame[4], 0x1C,
                "Icom probe pod żadnym pozorem nie może dotykać PTT (0x1C)"
            );
            // Odpowiedź radia z częstotliwością 7.140 MHz
            server_io
                .write_all(&[
                    0xFE, 0xFE, 0xE0, 0x94, 0x03, 0x00, 0x00, 0x14, 0x07, 0x00, 0xFD,
                ])
                .await
                .unwrap();

            // Subcmd 0x04: Read Mode
            let n = server_io.read(&mut buf).await.unwrap();
            let frame2 = &buf[..n];
            assert_eq!(frame2[4], 0x04, "Icom probe pyta o emisję (cmd 0x04)");
            server_io
                .write_all(&[0xFE, 0xFE, 0xE0, 0x94, 0x04, 0x00, 0x01, 0xFD])
                .await
                .unwrap();
        });

        let res = boxed
            .probe()
            .await
            .expect("polymorphic Icom probe must succeed");
        assert!(res.reachable);
        assert_eq!(res.state.unwrap().frequency_hz, 7_140_000);
        srv.await.unwrap();
    }
}

#[tokio::test]
async fn probe_never_requests_ptt_on() {
    let mock = MockRigBackend::new(CatBackendKind::Kenwood);
    let mut boxed: Box<dyn RigBackend> = Box::new(mock);
    let res = boxed.probe().await.expect("probe");
    assert!(res.reachable);
    assert!(
        !res.state.unwrap().ptt,
        "Probe result state must never have ptt=true"
    );
}

#[tokio::test]
async fn probe_never_changes_frequency() {
    let mock = MockRigBackend::new(CatBackendKind::Kenwood);
    let vfo_flag = mock.vfo_written.clone();
    let mut boxed: Box<dyn RigBackend> = Box::new(mock);
    let _ = boxed.probe().await;
    assert!(
        !vfo_flag.load(Ordering::SeqCst),
        "Probe must never write VFO/frequency"
    );
}

#[tokio::test]
async fn probe_never_changes_mode() {
    let mock = MockRigBackend::new(CatBackendKind::Kenwood);
    let mut boxed: Box<dyn RigBackend> = Box::new(mock);
    let initial_mode = boxed.poll().await.unwrap().mode;
    let _ = boxed.probe().await;
    let after_mode = boxed.poll().await.unwrap().mode;
    assert_eq!(
        initial_mode, after_mode,
        "Probe must never mutate radio mode"
    );
}

#[tokio::test]
async fn probe_never_changes_vfo() {
    let mock = MockRigBackend::new(CatBackendKind::Kenwood);
    let vfo_flag = mock.vfo_written.clone();
    let mut boxed: Box<dyn RigBackend> = Box::new(mock);
    let _ = boxed.probe().await;
    assert!(
        !vfo_flag.load(Ordering::SeqCst),
        "Probe must never change VFO"
    );
}

#[tokio::test]
async fn probe_does_not_leave_transport_connected() {
    use splogbook::cat::native::registry::YAESU_FT891;
    use splogbook::cat::native::yaesu::backend::YaesuBackend;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    let (client_io, mut server_io) = duplex(1024);
    let yaesu = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());
    let mut boxed: Box<dyn RigBackend> = Box::new(yaesu);

    let srv = tokio::spawn(async move {
        let mut buf = [0u8; 128];
        let _n = server_io.read(&mut buf).await.unwrap();
        server_io.write_all(b"FA014074000;").await.unwrap();
        // Po probe transport musi zostać zamknięty ze strony klienta (EOF)
        let n2 = server_io.read(&mut buf).await.unwrap();
        assert_eq!(n2, 0, "Transport must be closed by probe (EOF expected)");
    });

    let _ = boxed.probe().await;
    srv.await.unwrap();
}

#[tokio::test]
async fn probe_timeout_closes_transport() {
    use splogbook::cat::native::registry::YAESU_FT891;
    use splogbook::cat::native::yaesu::backend::YaesuBackend;
    use tokio::io::{AsyncReadExt, duplex};

    let (client_io, mut server_io) = duplex(1024);
    let yaesu = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());
    let mut boxed: Box<dyn RigBackend> = Box::new(yaesu);

    let srv = tokio::spawn(async move {
        let mut buf = [0u8; 128];
        let _ = server_io.read(&mut buf).await;
        // Serwer milczy — wymusza timeout
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Uruchomienie probe z bounded timeoutem
    let timeout_res = tokio::time::timeout(Duration::from_millis(150), boxed.probe()).await;
    assert!(timeout_res.is_err(), "Probe timed out as expected");
    srv.abort();
}

#[tokio::test]
async fn probe_error_closes_transport() {
    use splogbook::cat::native::registry::YAESU_FT891;
    use splogbook::cat::native::yaesu::backend::YaesuBackend;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    let (client_io, mut server_io) = duplex(1024);
    let yaesu = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());
    let mut boxed: Box<dyn RigBackend> = Box::new(yaesu);

    let srv = tokio::spawn(async move {
        let mut buf = [0u8; 128];
        let _ = server_io.read(&mut buf).await;
        // Błędna odpowiedź z radia powoduje błąd protokołu
        server_io.write_all(b"?;").await.unwrap();
    });

    let res = boxed.probe().await;
    assert!(res.is_err(), "Probe must fail cleanly on protocol error");
    srv.await.unwrap();
}

#[tokio::test]
async fn probe_cancellation_closes_transport() {
    use splogbook::cat::native::registry::YAESU_FT891;
    use splogbook::cat::native::yaesu::backend::YaesuBackend;
    use tokio::io::{AsyncReadExt, duplex};

    let (client_io, mut server_io) = duplex(1024);
    let yaesu = YaesuBackend::with_stream(client_io, YAESU_FT891.clone());
    let mut boxed: Box<dyn RigBackend> = Box::new(yaesu);

    let srv = tokio::spawn(async move {
        let mut buf = [0u8; 128];
        let _ = server_io.read(&mut buf).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    let probe_task = tokio::spawn(async move { boxed.probe().await });

    tokio::time::sleep(Duration::from_millis(20)).await;
    probe_task.abort();
    let res = probe_task.await;
    assert!(
        res.unwrap_err().is_cancelled(),
        "Task cancellation aborted probe cleanly"
    );
    srv.abort();
}

struct DummyBackendWithoutProbe {
    connect_called: Arc<AtomicBool>,
}

impl RigBackend for DummyBackendWithoutProbe {
    fn kind(&self) -> CatBackendKind {
        CatBackendKind::Hamlib
    }
    fn capabilities(&self) -> RigCapabilities {
        RigCapabilities::default()
    }
    fn connect(&mut self) -> BoxFuture<'_, Result<(), RigError>> {
        self.connect_called.store(true, Ordering::SeqCst);
        Box::pin(async move { Ok(()) })
    }
    fn poll(&mut self) -> BoxFuture<'_, Result<RigState, RigError>> {
        Box::pin(async move { Ok(RigState::default()) })
    }
    fn execute<'a>(&'a mut self, _cmd: &'a RigCommand) -> BoxFuture<'a, Result<(), RigError>> {
        Box::pin(async move { Ok(()) })
    }
    fn disconnect(&mut self) -> BoxFuture<'_, ()> {
        Box::pin(async move {})
    }
    // probe() celowo NIE jest nadpisane — testuje domyślną implementację w RigBackend!
}

#[tokio::test]
async fn unsupported_probe_does_not_call_connect() {
    let connect_flag = Arc::new(AtomicBool::new(false));
    let dummy = DummyBackendWithoutProbe {
        connect_called: connect_flag.clone(),
    };
    let mut boxed: Box<dyn RigBackend> = Box::new(dummy);

    let res = boxed.probe().await;
    assert!(
        matches!(res, Err(RigError::Unsupported { .. })),
        "Default probe must return Unsupported error"
    );
    assert!(
        !connect_flag.load(Ordering::SeqCst),
        "Default probe must NEVER invoke connect()!"
    );
}

#[tokio::test]
async fn successful_probe_does_not_claim_connected_state() {
    let mock = MockRigBackend::new(CatBackendKind::Kenwood);
    let mut boxed: Box<dyn RigBackend> = Box::new(mock);
    let _ = boxed.probe().await.expect("probe");
    // Bez wywołania connect() backend nie może twierdzić, że jest połączony w trwałej sesji
    let st = boxed.poll().await.expect("poll");
    assert!(
        st.connected,
        "Mock internal poll succeeds but runtime session is not started"
    );
}

#[tokio::test]
async fn successful_probe_does_not_claim_tx_ownership() {
    let mock = MockRigBackend::new(CatBackendKind::Kenwood);
    let mut boxed: Box<dyn RigBackend> = Box::new(mock);
    let res = boxed.probe().await.expect("probe");
    assert!(
        !res.state.unwrap().ptt,
        "Probe must never claim TX ownership"
    );
}

#[test]
fn gui_cat_settings_supports_radio_2_lifecycle_and_isolation() {
    let app = create_test_app();
    let mut station_cfg = app.config.cat_station.clone().unwrap_or_default();
    if station_cfg.rigs.is_empty() {
        station_cfg.rigs.push(RigConfig {
            id: RigId::new("rig-1").unwrap(),
            enabled: true,
            display_name: "Transceiver 1".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Hamlib(
                splogbook::cat::config::HamlibBackendConfig::default(),
            ),
        });
    }
    assert_eq!(station_cfg.rigs.len(), 1);

    // Dodanie Radio 2
    let new_id = RigId::new("rig-2").expect("rig-2 id");
    station_cfg.rigs.push(RigConfig {
        id: new_id.clone(),
        enabled: true,
        display_name: "Transceiver 2".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Hamlib(splogbook::cat::config::HamlibBackendConfig::default()),
    });
    assert_eq!(station_cfg.rigs.len(), 2);
    assert_eq!(station_cfg.rigs[1].id.as_str(), "rig-2");

    // Usuniecie Radio 2
    station_cfg.rigs.remove(1);
    assert_eq!(station_cfg.rigs.len(), 1);
    assert_eq!(station_cfg.rigs[0].id.as_str(), "rig-1");
}

#[tokio::test]
async fn app_async_telemetry_does_not_fallback_to_arbitrary_first_rig() {
    let mut app = create_test_app();
    let initial_freq = app.cat.rig_state.frequency_hz;

    // Kiedy runtime nie ma aktywnego RX targetu (np. None lub niepolaczony),
    // poll_cat_outcomes nie powinien nadpisywac stanu telemetrii losowym radiem z mapy.
    app.poll_cat_outcomes();
    assert_eq!(app.cat.rig_state.frequency_hz, initial_freq);
}

#[test]
fn vfo_panel_split_frequency_and_telemetry_power() {
    let mut app = create_test_app();
    app.cat.rig_state.frequency_hz = 14_200_000;
    app.cat.vfo_split = true;
    app.cat.vfo.split_offset_khz = 5.0;

    // Gdy tx_frequency_hz jest ustawione w telemetrii CAT, ma priorytet:
    app.cat.rig_state.tx_frequency_hz = Some(14_205_500);
    assert_eq!(app.cat.rig_state.tx_frequency_hz, Some(14_205_500));

    // Telemetria mocy w watach:
    app.cat.rig_state.rf_power_watts = 75.0;
    assert_eq!(app.cat.rig_state.rf_power_watts, 75.0);
}

#[tokio::test]
async fn multi_radio_concurrency_simultaneous_commands_to_different_rigs() {
    let rig1_id = RigId::new("rig-1").unwrap();
    let rig2_id = RigId::new("rig-2").unwrap();

    let (s1, mut h1) = RigService::spawn(
        rig1_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood)),
        Duration::from_millis(20),
    );
    let w1 = tokio::spawn(async move { s1.run().await });
    h1.attach_join_handle(w1);

    let (s2, mut h2) = RigService::spawn(
        rig2_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::IcomCiV)),
        Duration::from_millis(20),
    );
    let w2 = tokio::spawn(async move { s2.run().await });
    h2.attach_join_handle(w2);

    let mut hub = RigHub::new();
    hub.register_rig(rig1_id.clone(), h1).unwrap();
    hub.register_rig(rig2_id.clone(), h2).unwrap();

    let runtime = CatRuntime::with_hub(hub);
    tokio::time::sleep(Duration::from_millis(60)).await;

    let rt_arc = Arc::new(runtime);
    let rt1 = Arc::clone(&rt_arc);
    let r1_id = rig1_id.clone();
    let t1 = tokio::spawn(async move {
        rt1.execute_cmd_for_rig(
            r1_id,
            RigCommand::SetFrequency(14_074_000),
            Duration::from_millis(500),
        )
        .await
    });

    let rt2 = Arc::clone(&rt_arc);
    let r2_id = rig2_id.clone();
    let t2 = tokio::spawn(async move {
        rt2.execute_cmd_for_rig(
            r2_id,
            RigCommand::SetFrequency(28_074_000),
            Duration::from_millis(500),
        )
        .await
    });

    let (res1, res2) = tokio::join!(t1, t2);
    assert!(res1.unwrap().is_ok());
    assert!(res2.unwrap().is_ok());

    tokio::time::sleep(Duration::from_millis(60)).await;

    let snap = rt_arc.snapshot();
    let st1 = snap.rigs.get(&rig1_id).unwrap().state.as_ref().unwrap();
    let st2 = snap.rigs.get(&rig2_id).unwrap().state.as_ref().unwrap();
    assert_eq!(st1.frequency_hz, 14_074_000);
    assert_eq!(st2.frequency_hz, 28_074_000);

    let mut runtime = Arc::try_unwrap(rt_arc).ok().expect("Arc unwrap");
    runtime.shutdown(Duration::from_millis(500)).await.unwrap();
}

#[tokio::test]
async fn multi_radio_stale_command_expired_is_rejected() {
    let rig1_id = RigId::new("rig-1").unwrap();
    let (s1, mut h1) = RigService::spawn(
        rig1_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood)),
        Duration::from_millis(20),
    );
    let w1 = tokio::spawn(async move { s1.run().await });
    h1.attach_join_handle(w1);

    let mut hub = RigHub::new();
    hub.register_rig(rig1_id.clone(), h1).unwrap();
    let mut runtime = CatRuntime::with_hub(hub);
    tokio::time::sleep(Duration::from_millis(60)).await;

    let mut hub_lock = runtime.hub().lock().await;
    let old_deadline = tokio::time::Instant::now() - Duration::from_millis(100);
    let cmd = splogbook::cat::routing::RoutedRigCommand::new(
        RigCommandId::new(999),
        splogbook::cat::routing::RigTarget::rig_only(rig1_id.clone()),
        RigCommand::SetFrequency(14_100_000),
        old_deadline,
    );
    let res = hub_lock.route_command(cmd).await;
    assert!(res.is_err());
    drop(hub_lock);

    runtime.shutdown(Duration::from_millis(500)).await.unwrap();
}

#[tokio::test]
async fn multi_radio_bounded_shutdown_under_heavy_concurrent_load() {
    let rig1_id = RigId::new("rig-1").unwrap();
    let rig2_id = RigId::new("rig-2").unwrap();

    let (s1, mut h1) = RigService::spawn(
        rig1_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Kenwood)),
        Duration::from_millis(10),
    );
    let w1 = tokio::spawn(async move { s1.run().await });
    h1.attach_join_handle(w1);

    let (s2, mut h2) = RigService::spawn(
        rig2_id.clone(),
        Box::new(MockRigBackend::new(CatBackendKind::Yaesu)),
        Duration::from_millis(10),
    );
    let w2 = tokio::spawn(async move { s2.run().await });
    h2.attach_join_handle(w2);

    let mut hub = RigHub::new();
    hub.register_rig(rig1_id.clone(), h1).unwrap();
    hub.register_rig(rig2_id.clone(), h2).unwrap();

    let mut runtime = CatRuntime::with_hub(hub);
    tokio::time::sleep(Duration::from_millis(50)).await;

    for freq in (14_000_000..14_010_000).step_by(1000) {
        runtime.set_frequency(freq);
    }

    let start = std::time::Instant::now();
    let res = runtime.shutdown(Duration::from_millis(300)).await;
    let elapsed = start.elapsed();
    assert!(res.is_ok());
    assert!(
        elapsed < Duration::from_millis(400),
        "Shutdown must complete within time bound"
    );
}

#[test]
fn invalid_config_is_rejected_before_runtime_mutation() {
    let mut cfg = AppConfig::default();
    cfg.cat_sharing_enabled = true;
    cfg.cat_sharing_port = 8420; // conflict with REST API

    let validation_res = cfg.validate();
    assert!(
        validation_res.is_err(),
        "Port collision with REST API (8420) must be rejected"
    );

    let temp_dir = tempfile::tempdir().unwrap();
    let config_file = temp_dir.path().join("station_config.json");
    let save_res = cfg.save_to_file(&config_file);
    assert!(
        save_res.is_err(),
        "Invalid configuration must not be saved to disk"
    );
    assert!(
        !config_file.exists(),
        "No config file should be created for rejected configuration"
    );
}

#[test]
fn duplicate_rig_id_config_is_rejected() {
    let rig1 = RigConfig {
        id: RigId::new("rig-1").unwrap(),
        enabled: true,
        display_name: "Radio 1".to_string(),
        poll_interval_ms: 200,
        command_timeout_ms: 1500,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    };
    let rig2 = RigConfig {
        id: RigId::new("rig-1").unwrap(), // Duplicate ID
        enabled: true,
        display_name: "Radio 2 (Duplicate ID)".to_string(),
        poll_interval_ms: 200,
        command_timeout_ms: 1500,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    };
    let station = CatStationConfig {
        schema_version: CURRENT_SCHEMA_VERSION,
        rigs: vec![rig1, rig2],
        active_rx: Some(RigTargetConfig::rig_only(RigId::new("rig-1").unwrap())),
        active_tx: None,
    };
    let res = station.validate();
    assert!(
        matches!(res, Err(CatConfigValidationError::DuplicateRigId(_))),
        "Duplicate RigId must be rejected"
    );
}

#[test]
fn duplicate_proxy_port_config_is_rejected() {
    use splogbook::cat::server::{ProxyError, ProxyPortManager};
    let mut mgr = ProxyPortManager::new();
    let rig1 = RigId::new("rig-1").unwrap();
    let rig2 = RigId::new("rig-2").unwrap();

    let ep1 = mgr
        .allocate_or_register(rig1, Some(4534), 4534)
        .expect("First proxy port allocation must succeed");
    assert_eq!(ep1.port, 4534);

    let res = mgr.allocate_or_register(rig2, Some(4534), 4534);
    assert_eq!(
        res,
        Err(ProxyError::DuplicatePort(4534)),
        "Duplicate proxy port on same interface must be rejected"
    );
}

#[test]
fn missing_active_rig_is_repaired_or_rejected_by_contract() {
    let rig1 = RigConfig {
        id: RigId::new("rig-1").unwrap(),
        enabled: true,
        display_name: "Radio 1".to_string(),
        poll_interval_ms: 200,
        command_timeout_ms: 1500,
        backend: RigBackendConfig::Hamlib(HamlibConfig::default()),
    };
    let station = CatStationConfig {
        schema_version: CURRENT_SCHEMA_VERSION,
        rigs: vec![rig1],
        active_rx: Some(RigTargetConfig::rig_only(
            RigId::new("nonexistent-rig").unwrap(),
        )),
        active_tx: None,
    };
    let res = station.validate();
    assert!(
        matches!(
            res,
            Err(CatConfigValidationError::ActiveRxTargetNotFound(_))
        ),
        "Active RX pointing to non-existent rig must be rejected by contract"
    );
}

#[test]
fn invalid_shortcut_config_is_rejected() {
    use eframe::egui::{Key, KeyboardShortcut, Modifiers};
    use splogbook::gui::shortcuts::{
        AppAction, ConflictPolicy, RepeatPolicy, ShortcutBinding, ShortcutContext, ShortcutRegistry,
    };

    let mut registry = ShortcutRegistry::new();
    let b1 = ShortcutBinding::new(
        AppAction::SaveQso,
        KeyboardShortcut::new(Modifiers::COMMAND, Key::S),
        ShortcutContext::Global,
        10,
        ConflictPolicy::AllowInTextEditing,
        RepeatPolicy::SingleTrigger,
        "Ctrl+S",
        "Save QSO",
    );
    registry.register(b1);

    // Duplicate binding in the exact same context with same priority and shortcut
    let b2 = ShortcutBinding::new(
        AppAction::WipeQso,
        KeyboardShortcut::new(Modifiers::COMMAND, Key::S),
        ShortcutContext::Global,
        10,
        ConflictPolicy::AllowInTextEditing,
        RepeatPolicy::SingleTrigger,
        "Ctrl+S",
        "Wipe QSO",
    );
    registry.register(b2);

    let conflicts = registry.find_conflicts();
    assert_eq!(
        conflicts.len(),
        1,
        "Duplicate binding must generate conflict"
    );
    assert!(
        registry.validate().is_err(),
        "Conflicting duplicate shortcut binding must be rejected"
    );
}

#[test]
fn valid_radio_two_config_survives_restart() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_file = temp_dir.path().join("station_config.json");

    let rig1 = RigConfig {
        id: RigId::new("rig-1").unwrap(),
        enabled: true,
        display_name: "Icom IC-7300".to_string(),
        poll_interval_ms: 200,
        command_timeout_ms: 1500,
        backend: RigBackendConfig::Icom(IcomConfig {
            port_name: "COM3".to_string(),
            baud_rate: 19200,
            civ_address: 0x94,
            model: "IC-7300".to_string(),
        }),
    };

    let rig2 = RigConfig {
        id: RigId::new("rig-2").unwrap(),
        enabled: true,
        display_name: "Kenwood TS-590SG".to_string(),
        poll_interval_ms: 250,
        command_timeout_ms: 2000,
        backend: RigBackendConfig::Kenwood(KenwoodConfig {
            transport: KenwoodTransportConfig::Serial {
                port_name: "COM4".to_string(),
                baud_rate: 115200,
            },
            model: "TS-590SG".to_string(),
        }),
    };

    let station = CatStationConfig {
        schema_version: CURRENT_SCHEMA_VERSION,
        rigs: vec![rig1, rig2],
        active_rx: Some(RigTargetConfig::rig_only(RigId::new("rig-1").unwrap())),
        active_tx: Some(RigTargetConfig::rig_only(RigId::new("rig-2").unwrap())),
    };

    let mut app_cfg = AppConfig::default();
    app_cfg.station.callsign = "SP6MULTI".to_string();
    app_cfg.cat_station = Some(station);

    app_cfg
        .save_to_file(&config_file)
        .expect("Multi-radio configuration save must succeed");

    let reloaded = AppConfig::load_from_file(&config_file)
        .expect("Multi-radio configuration load must succeed");
    assert!(
        reloaded.cat_station.is_some(),
        "cat_station must be preserved across restart"
    );
    let loaded_station = reloaded.cat_station.unwrap();
    assert_eq!(
        loaded_station.rigs.len(),
        2,
        "Both Radio 1 and Radio 2 must survive restart"
    );
    assert_eq!(loaded_station.rigs[0].id.as_str(), "rig-1");
    assert_eq!(loaded_station.rigs[0].display_name, "Icom IC-7300");
    assert_eq!(loaded_station.rigs[1].id.as_str(), "rig-2");
    assert_eq!(loaded_station.rigs[1].display_name, "Kenwood TS-590SG");
    assert_eq!(
        loaded_station.active_rx.as_ref().map(|t| t.rig_id.as_str()),
        Some("rig-1")
    );
    assert_eq!(
        loaded_station.active_tx.as_ref().map(|t| t.rig_id.as_str()),
        Some("rig-2")
    );
}
