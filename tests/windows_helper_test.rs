//! Testy natywnego pomocnika Windows (`splogbook-updater.exe`) oraz utwardzonego fallbacku PowerShell (M3-B).
//!
//! Sprawdza:
//! 1. Obsługę ścieżek ze spacjami i znakami Unicode;
//! 2. Wykrywanie naruszenia tożsamości planu (`plan_digest`) i unikalnego `attempt_id`;
//! 3. Poprawność zachowania przy przekroczeniu limitu czasu procesu macierzystego;
//! 4. Automatyczny i bezpieczny rollback w przypadku awarii kandydata (crash);
//! 5. Automatyczny rollback w przypadku przekroczenia limitu czasu na dowód zdrowia (`.health_ok`);
//! 6. Odrzucenie niepoprawnego `HealthProof` (stale proof, wrong attempt ID);
//! 7. Bezwzględną ochronę baz SQLite (`serviceLOG.db`, WAL, SHM) i plików konfiguracyjnych;
//! 8. Zachowanie katalogu `.rollback` po udanej aktualizacji (retencja OWNER-M2-08/09);
//! 9. Pełną parzystość polityki bezpieczeństwa między helperem Rust a fallbackiem PowerShell.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

use splogbook::cloud::updater::helper_protocol::{
    CURRENT_HELPER_REQUEST_VERSION, UpdatePlanIdentity, WindowsHelperExitCode, WindowsHelperRequest,
};
use splogbook::cloud::updater::powershell_fallback::generate_powershell_update_script;
use splogbook::cloud::updater::windows_helper::{
    CandidateLauncher, HelperCandidateChild, ProcessMonitor, WindowsHelperEngine,
};
use splogbook::cloud::updater::{
    HealthProof, InstallationType, ManagementMode, UpdateAttemptId, UpdatePlan, UpdateTransaction,
};

/// Mock monitora procesów kontrolujący stan procesu macierzystego.
struct TestProcessMonitor {
    running_pids: std::sync::Mutex<std::collections::HashSet<u32>>,
}

impl TestProcessMonitor {
    fn new(initially_running: &[u32]) -> Self {
        let mut set = std::collections::HashSet::new();
        for &pid in initially_running {
            set.insert(pid);
        }
        Self {
            running_pids: std::sync::Mutex::new(set),
        }
    }
}

impl ProcessMonitor for TestProcessMonitor {
    fn is_process_running(
        &self,
        pid: u32,
        _expected_image: &str,
    ) -> Result<bool, WindowsHelperExitCode> {
        let set = self
            .running_pids
            .lock()
            .map_err(|_| WindowsHelperExitCode::ParentProcessTimeout)?;
        Ok(set.contains(&pid))
    }
}

/// Mock procesu kandydata
struct TestCandidateChild {
    is_alive: bool,
}

impl HelperCandidateChild for TestCandidateChild {
    fn is_alive(&mut self) -> Result<bool, WindowsHelperExitCode> {
        Ok(self.is_alive)
    }

    fn terminate(&mut self) -> Result<(), WindowsHelperExitCode> {
        self.is_alive = false;
        Ok(())
    }
}

/// Mock launchera kandydata
struct TestCandidateLauncher {
    create_valid_proof: bool,
    tamper_proof_attempt: bool,
    crash_on_launch: bool,
    is_alive: bool,
}

impl CandidateLauncher for TestCandidateLauncher {
    fn launch_candidate(
        &self,
        _candidate_exe: &Path,
        attempt_id: &UpdateAttemptId,
        health_proof_path: &Path,
        expected_version: &str,
        expected_commit: &str,
    ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode> {
        if self.crash_on_launch {
            return Err(WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded);
        }

        if self.create_valid_proof {
            let aid = if self.tamper_proof_attempt {
                UpdateAttemptId::generate()
            } else {
                *attempt_id
            };
            let proof = HealthProof::new(
                aid,
                expected_version,
                expected_commit,
                "windows-x86_64",
                InstallationType::WindowsPortableZip,
            );
            if let Ok(json) = serde_json::to_string_pretty(&proof) {
                let _ = fs::write(health_proof_path, json);
            }
        }

        Ok(Box::new(TestCandidateChild {
            is_alive: self.is_alive,
        }))
    }
}

fn create_test_request(
    root: &Path,
    attempt: UpdateAttemptId,
    plan: &UpdatePlan,
) -> WindowsHelperRequest {
    let plan_identity = UpdatePlanIdentity::from_plan(
        attempt,
        root,
        plan,
        "SPLogbook.exe",
        "1.2.1",
        "0000000000000000000000000000000000000000",
        "windows-x86_64",
    )
    .expect("plan identity");
    let plan_digest = plan_identity.compute_digest();

    let staging = root.join(".staging");
    let mut tx = UpdateTransaction::new(
        attempt,
        "1.2.1".into(),
        "v1.2.1".into(),
        "windows".into(),
        "x86_64".into(),
        "windows-portable-zip".into(),
        InstallationType::WindowsPortableZip,
        "0000000000000000000000000000000000000000".into(),
        "SPLogbook-Windows-x64.zip".into(),
        1000,
        "0000000000000000000000000000000000000000000000000000000000000000".into(),
        staging,
        root,
    )
    .expect("tx new");
    tx.plan_digest = Some(plan_digest.clone());
    tx.save_to_dir(root).expect("save tx");
    let tx_path = root.join(".update_transaction.json");

    WindowsHelperRequest {
        request_version: CURRENT_HELPER_REQUEST_VERSION,
        attempt_id: attempt,
        transaction_record_path: tx_path,
        installation_root: root.to_path_buf(),
        current_executable: root.join("SPLogbook.exe"),
        candidate_executable: root.join(".staging").join("SPLogbook.exe"),
        expected_parent_pid: 1234,
        expected_current_binary_identity: None,
        expected_target_version: "1.2.1".to_string(),
        expected_target_commit: "0000000000000000000000000000000000000000".to_string(),
        update_management_mode: ManagementMode::SelfManaged,
        update_plan_identity: plan_identity,
        plan_digest,
        health_proof_path: root.join(".health_ok"),
        parent_process_timeout_seconds: 5,
        candidate_health_timeout_seconds: 5,
        result_record_path: root.join(".update_helper_result.json"),
        diagnostic_log_path: root.join(".update_helper.log"),
    }
}

#[test]
fn windows_helper_paths_with_spaces_and_unicode() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path().join("Katalog SPLogbook łąka sp6 test");
    let staging = root.join(".staging");
    let rollback = root.join(".rollback");
    fs::create_dir_all(&root).expect("root dir");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"SPLogbook v1.2.0 stare").expect("write exe");
    fs::write(&staging_exe, b"SPLogbook v1.2.1 nowe").expect("write staging exe");

    // Dodatkowy zasób ze spacją w nazwie
    let res_dir = root.join("resources");
    let staging_res = staging.join("resources");
    fs::create_dir_all(&res_dir).expect("res_dir");
    fs::create_dir_all(&staging_res).expect("staging_res");
    fs::write(res_dir.join("ikona programu.ico"), b"ICO_V1").expect("res orig");
    fs::write(staging_res.join("ikona programu.ico"), b"ICO_V2").expect("res new");

    let plan = UpdatePlan {
        replace_files: vec![
            (PathBuf::from("SPLogbook.exe"), exe_path.clone()),
            (
                PathBuf::from("resources/ikona programu.ico"),
                res_dir.join("ikona programu.ico"),
            ),
        ],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(&root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::Success,
        "Aktualizacja musi się udać (kod 0)"
    );
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        fs::read(&exe_path).expect("read exe"),
        b"SPLogbook v1.2.1 nowe"
    );
    assert_eq!(
        fs::read(res_dir.join("ikona programu.ico")).expect("read res"),
        b"ICO_V2"
    );

    // Kopia zapasowa w .rollback musi istnieć (retencja)
    assert!(
        rollback.join("SPLogbook.exe").exists(),
        "Rollback musi zachować kopię starej wersji"
    );
}

#[test]
fn windows_helper_detects_tampered_request_digest() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"new").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    // Sfałszowany plan_digest
    req.plan_digest =
        "0000000000000000000000000000000000000000000000000000000000000000".to_string();

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::PlanDigestMismatch,
        "Zmanipulowany skrót planu musi natychmiast przerwać operację z kodem PlanDigestMismatch"
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_detects_tampered_attempt_id() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"new").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    // Podmiana attempt_id w żądaniu przy zachowaniu starego w plan_identity
    req.attempt_id = UpdateAttemptId::generate();

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::InvalidAttemptId,
        "Niezgodność attempt_id musi fail-closed odrzucić żądanie"
    );
}

#[test]
fn windows_helper_parent_process_timeout() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"new").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.expected_parent_pid = 7777;
    req.parent_process_timeout_seconds = 1; // 1 sekunda timeoutu

    // Monitor symuluje, że PID 7777 nigdy się nie kończy
    let monitor = TestProcessMonitor::new(&[7777]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::ParentProcessTimeout,
        "Niezamknięty proces nadrzędny musi skutkować kodem ParentProcessTimeout (16)"
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_candidate_crash_triggers_rollback() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    let new_res_path = root.join("new_component.dll");
    let staging_new_res = staging.join("new_component.dll");

    fs::write(&exe_path, b"oryginalna binarka v1.2.0").expect("write exe");
    fs::write(&staging_exe, b"uszkodzona binarka v1.2.1").expect("write staging exe");
    fs::write(&staging_new_res, b"nowy zasob").expect("write staging new res");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: vec![(PathBuf::from("new_component.dll"), new_res_path.clone())],
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    // Symulacja crasha: kandydat natychmiast kończy działanie (is_alive: false)
    let launcher = TestCandidateLauncher {
        create_valid_proof: false,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded
    );
    // Weryfikacja rollbacku
    assert_eq!(
        fs::read(&exe_path).expect("read exe"),
        b"oryginalna binarka v1.2.0",
        "Oryginalna binarka musi zostać przywrócona z rollbacku"
    );
    assert!(
        !new_res_path.exists(),
        "Nowo utworzone pliki create_files muszą zostać usunięte podczas rollbacku"
    );
}

#[test]
fn windows_helper_health_timeout_triggers_rollback() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"stara wersja").expect("write exe");
    fs::write(&staging_exe, b"zawieszajaca sie nowa wersja").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.candidate_health_timeout_seconds = 1; // 1s na health check

    let monitor = TestProcessMonitor::new(&[]);
    // Nie tworzy dowodu zdrowia, kandydat żyje
    let launcher = TestCandidateLauncher {
        create_valid_proof: false,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::HealthTimeoutRollbackSucceeded,
        "Timeout testu zdrowia musi wykonać rollback i zwrócić kod 23"
    );
    assert_eq!(
        fs::read(&exe_path).expect("read exe"),
        b"stara wersja",
        "Po rollbacku musi być stara wersja"
    );
}

#[test]
fn windows_helper_health_invalid_attempt_id_rejected() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"wersja bazowa").expect("write exe");
    fs::write(&staging_exe, b"kandydat z falszywym attempt_id").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.candidate_health_timeout_seconds = 1;

    let monitor = TestProcessMonitor::new(&[]);
    // Zapisuje dowód zdrowia z INNYM attempt_id
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: true,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::HealthRejectedRollbackSucceeded,
        "Fałszywy dowód zdrowia z obcym attempt_id musi zostać odrzucony i nastąpić rollback"
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"wersja bazowa");
}

#[test]
fn windows_helper_rollback_preserves_sqlite_and_wal() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let db_path = root.join("serviceLOG.db");
    let wal_path = root.join("serviceLOG.db-wal");
    let shm_path = root.join("serviceLOG.db-shm");
    let cfg_path = root.join("station_config.json");

    let db_bytes = b"SQLITE_DATABASE_DATA_BYTES_PROTECTED";
    let wal_bytes = b"SQLITE_WAL_BYTES_PROTECTED";
    let shm_bytes = b"SQLITE_SHM_BYTES_PROTECTED";
    let cfg_bytes = b"{\"callsign\":\"SP6IN\"}";

    fs::write(&db_path, db_bytes).expect("write db");
    fs::write(&wal_path, wal_bytes).expect("write wal");
    fs::write(&shm_path, shm_bytes).expect("write shm");
    fs::write(&cfg_path, cfg_bytes).expect("write cfg");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig exe").expect("write exe");
    fs::write(&staging_exe, b"new exe").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: vec![
            (db_path.clone(), "Preserved DB".into()),
            (wal_path.clone(), "Preserved WAL".into()),
            (shm_path.clone(), "Preserved SHM".into()),
            (cfg_path.clone(), "Preserved Config".into()),
        ],
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.candidate_health_timeout_seconds = 1;

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: false,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let _ = engine.execute();

    assert_eq!(
        fs::read(&db_path).expect("read db"),
        db_bytes,
        "serviceLOG.db musi być w 100% nienaruszona"
    );
    assert_eq!(
        fs::read(&wal_path).expect("read wal"),
        wal_bytes,
        "serviceLOG.db-wal musi być w 100% nienaruszony"
    );
    assert_eq!(
        fs::read(&shm_path).expect("read shm"),
        shm_bytes,
        "serviceLOG.db-shm musi być w 100% nienaruszony"
    );
    assert_eq!(
        fs::read(&cfg_path).expect("read cfg"),
        cfg_bytes,
        "station_config.json musi być w 100% nienaruszona"
    );
}

#[test]
fn powershell_fallback_parity_with_windows_helper() {
    let plan = UpdatePlan {
        replace_files: vec![(
            PathBuf::from("SPLogbook.exe"),
            PathBuf::from("C:\\SPLogbook\\SPLogbook.exe"),
        )],
        create_files: vec![(
            PathBuf::from("data/cty.dat"),
            PathBuf::from("C:\\SPLogbook\\data\\cty.dat"),
        )],
        preserve_files: vec![(
            PathBuf::from("C:\\SPLogbook\\serviceLOG.db"),
            "Preserved SQLite".into(),
        )],
        rejected_files: Vec::new(),
    };

    let script =
        generate_powershell_update_script(Path::new("C:\\SPLogbook\\SPLogbook.exe"), 5555, &plan);

    // Weryfikacja parzystości polityki
    assert!(script.contains("Set-StrictMode -Version Latest"));
    assert!(script.contains("$ErrorActionPreference = 'Stop'"));
    assert!(!script.contains("serviceLOG.db"));
    assert!(!script.contains("$staging\\*"));
    assert!(!script.contains("$rollback\\*"));
    assert!(!script.contains("Invoke-Expression"));
    assert!(!script.contains("iex"));
    assert!(!script.contains("runas"));
    assert!(script.contains("exit 0"));
    assert!(script.contains("exit 2"));
    assert!(script.contains("exit 9"));
}
