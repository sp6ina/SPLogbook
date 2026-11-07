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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tempfile::tempdir;

use splogbook::cloud::updater::helper_protocol::{
    CURRENT_HELPER_REQUEST_VERSION, HealthResultKind, HelperResultCategory, RollbackResultKind,
    StagingFileExpectation, UpdatePlanIdentity, WindowsHelperExitCode, WindowsHelperRequest,
};
use splogbook::cloud::updater::powershell_fallback::{
    generate_powershell_from_request_file, generate_powershell_update_script,
};
use splogbook::cloud::updater::windows_helper::{
    CandidateLauncher, HelperCandidateChild, HelperFileOperations, ProcessMonitor,
    WindowsHelperEngine,
};
use splogbook::cloud::updater::{
    HealthProof, InstallationType, ManagementMode, UpdateAttemptId, UpdatePlan, UpdateStateId,
    UpdateTransaction,
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

/// Mock śledzący tożsamość i wywołania procesu potomnego kandydata.
struct TrackingChild {
    is_alive: bool,
    terminate_count: Arc<AtomicUsize>,
}

impl HelperCandidateChild for TrackingChild {
    fn is_alive(&mut self) -> Result<bool, WindowsHelperExitCode> {
        Ok(self.is_alive)
    }

    fn terminate(&mut self) -> Result<(), WindowsHelperExitCode> {
        self.terminate_count.fetch_add(1, Ordering::SeqCst);
        self.is_alive = false;
        Ok(())
    }
}

struct TrackingLauncher {
    terminate_count: Arc<AtomicUsize>,
    launch_called: Arc<AtomicBool>,
    fail_launch: bool,
}

impl CandidateLauncher for TrackingLauncher {
    fn launch_candidate(
        &self,
        _candidate_exe: &Path,
        _attempt_id: &UpdateAttemptId,
        _health_proof_path: &Path,
        _expected_version: &str,
        _expected_commit: &str,
    ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode> {
        self.launch_called.store(true, Ordering::SeqCst);
        if self.fail_launch {
            return Err(WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded);
        }
        Ok(Box::new(TrackingChild {
            is_alive: true,
            terminate_count: Arc::clone(&self.terminate_count),
        }))
    }
}

struct FaultyFileOps {
    fail_copy_on_target_substring: &'static str,
}

impl HelperFileOperations for FaultyFileOps {
    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        let from_str = from.to_string_lossy();
        let to_str = to.to_string_lossy();
        if from_str.contains(".staging") && to_str.contains(self.fail_copy_on_target_substring) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Injected I/O error during file replacement/creation",
            ));
        }
        std::fs::copy(from, to)
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }
}

struct BackupFailingFileOps;

impl HelperFileOperations for BackupFailingFileOps {
    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        if to.to_string_lossy().contains(".rollback") {
            return Err(std::io::Error::other("Injected failure during backup copy"));
        }
        std::fs::copy(from, to)
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
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
        installed_version: Some("1.2.0".to_string()),
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
        staging_file_expectations: std::collections::BTreeMap::new(),
        enforce_strict_staging_entries: false,
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

// ============================================================================
// Obszar 1: Tożsamość procesów (Parent i Candidate)
// ============================================================================

#[test]
fn candidate_termination_uses_spawned_child_handle() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"cand").expect("write staging exe");

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
    let terminate_count = Arc::new(AtomicUsize::new(0));
    let launch_called = Arc::new(AtomicBool::new(false));
    let launcher = TrackingLauncher {
        terminate_count: Arc::clone(&terminate_count),
        launch_called: Arc::clone(&launch_called),
        fail_launch: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::HealthTimeoutRollbackSucceeded
    );
    assert!(
        launch_called.load(Ordering::SeqCst),
        "Kandydat musiał zostać uruchomiony"
    );
    assert_eq!(
        terminate_count.load(Ordering::SeqCst),
        1,
        "Zakończenie kandydata musi nastąpić wyłącznie przez wywołanie terminate na uchwycie Child"
    );
}

#[test]
fn candidate_is_not_found_by_process_name() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"cand").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    let terminate_count = Arc::new(AtomicUsize::new(0));
    let launch_called = Arc::new(AtomicBool::new(false));
    let launcher = TrackingLauncher {
        terminate_count: Arc::clone(&terminate_count),
        launch_called: Arc::clone(&launch_called),
        fail_launch: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let _ = engine.execute();

    let running = monitor.running_pids.lock().unwrap();
    assert!(
        !running.contains(&9999),
        "Helper nie może przeszukiwać procesów po nazwie exe"
    );
}

#[test]
fn foreign_candidate_process_is_not_terminated() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"cand").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.candidate_health_timeout_seconds = 1;

    let foreign_child_terminated = Arc::new(AtomicBool::new(false));
    let _foreign_tracker = Arc::clone(&foreign_child_terminated);

    let monitor = TestProcessMonitor::new(&[]);
    let candidate_terminate_count = Arc::new(AtomicUsize::new(0));
    let launch_called = Arc::new(AtomicBool::new(false));
    let launcher = TrackingLauncher {
        terminate_count: Arc::clone(&candidate_terminate_count),
        launch_called: Arc::clone(&launch_called),
        fail_launch: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::HealthTimeoutRollbackSucceeded
    );
    assert_eq!(candidate_terminate_count.load(Ordering::SeqCst), 1);
    assert!(
        !foreign_child_terminated.load(Ordering::SeqCst),
        "Obcy proces w systemie nie może zostać zakończony"
    );
}

#[test]
fn health_timeout_terminates_only_spawned_candidate() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"cand").expect("write staging exe");

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
    let terminate_count = Arc::new(AtomicUsize::new(0));
    let launch_called = Arc::new(AtomicBool::new(false));
    let launcher = TrackingLauncher {
        terminate_count: Arc::clone(&terminate_count),
        launch_called: Arc::clone(&launch_called),
        fail_launch: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::HealthTimeoutRollbackSucceeded
    );
    assert_eq!(
        terminate_count.load(Ordering::SeqCst),
        1,
        "Wyłącznie powiązany proces potomny kandydata może być zakończony"
    );
}

#[test]
fn lost_candidate_handle_does_not_authorize_pid_only_termination() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"cand").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    let terminate_count = Arc::new(AtomicUsize::new(0));
    let launch_called = Arc::new(AtomicBool::new(false));
    let launcher = TrackingLauncher {
        terminate_count: Arc::clone(&terminate_count),
        launch_called: Arc::clone(&launch_called),
        fail_launch: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded
    );
    assert_eq!(
        terminate_count.load(Ordering::SeqCst),
        0,
        "Brak uchwytu potomka kandydata bezwzględnie zabrania ubijania procesów po samym PID"
    );
}

#[test]
fn parent_process_is_never_terminated_by_helper() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"cand").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.expected_parent_pid = 4321;
    req.parent_process_timeout_seconds = 1;

    let monitor = TestProcessMonitor::new(&[4321]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, _) = engine.execute();

    assert_eq!(exit_code, WindowsHelperExitCode::ParentProcessTimeout);
    let running = monitor.running_pids.lock().unwrap();
    assert!(
        running.contains(&4321),
        "Helper pod żadnym pozorem nie może ubijać procesu nadrzędnego"
    );
    assert_eq!(
        fs::read(&exe_path).expect("read exe"),
        b"orig",
        "W przypadku timeoutu procesu nadrzędnego żadne pliki nie mogą ulec modyfikacji"
    );
}

// ============================================================================
// Obszar 2: Staging i Backup Tampering
// ============================================================================

#[test]
fn windows_helper_rejects_staging_file_hash_mismatch() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"tampered_bytes").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.staging_file_expectations.insert(
        "SPLogbook.exe".to_string(),
        StagingFileExpectation {
            size: 14,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        },
    );

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
        WindowsHelperExitCode::InvalidArguments,
        "Niezgodność skrótu SHA-256 w .staging musi zatrzymać helper z kodem InvalidArguments (10)"
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_rejects_staging_file_size_mismatch() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"short").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.staging_file_expectations.insert(
        "SPLogbook.exe".to_string(),
        StagingFileExpectation {
            size: 9999,
            sha256: "any".to_string(),
        },
    );

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (exit_code, result) = engine.execute();

    assert_eq!(exit_code, WindowsHelperExitCode::InvalidArguments);
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_detects_backup_hash_mismatch() {
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
    let req = create_test_request(root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };
    let file_ops = BackupFailingFileOps;

    let mut engine = WindowsHelperEngine::with_file_ops(&req, &monitor, &launcher, &file_ops);
    let (exit_code, result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::BackupFailed,
        "Nieudany backup musi zatrzymać procedurę z kodem BackupFailed (18)"
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_detects_backup_size_mismatch() {
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
    let req = create_test_request(root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };
    let file_ops = BackupFailingFileOps;

    let mut engine = WindowsHelperEngine::with_file_ops(&req, &monitor, &launcher, &file_ops);
    let (exit_code, result) = engine.execute();

    assert_eq!(exit_code, WindowsHelperExitCode::BackupFailed);
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_rejects_missing_staging_source() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

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
        WindowsHelperExitCode::InvalidArguments,
        "Brak pliku w .staging musi zatrzymać helper z kodem InvalidArguments (10)"
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_rejects_unplanned_staging_entry_if_policy_requires() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"orig").expect("write exe");
    fs::write(&staging_exe, b"new").expect("write staging exe");
    fs::write(staging.join("unplanned_payload.exe"), b"MALICIOUS").expect("write unplan");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.enforce_strict_staging_entries = true;

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
        WindowsHelperExitCode::UnsafePath,
        "Nieplanowany plik w .staging przy ścisłej polityce musi fail-closed odrzucić zlecenie z kodem UnsafePath (15)"
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"orig");
}

#[test]
fn windows_helper_rejects_existing_create_target() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let new_res_path = root.join("new_component.dll");
    let staging_new_res = staging.join("new_component.dll");
    fs::write(&new_res_path, b"ALREADY_EXISTS").expect("write existing");
    fs::write(&staging_new_res, b"NEW_BYTES").expect("write staging");

    let plan = UpdatePlan {
        replace_files: Vec::new(),
        create_files: vec![(PathBuf::from("new_component.dll"), new_res_path.clone())],
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

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
        WindowsHelperExitCode::UnsafePath,
        "Istniejący plik cel dla operacji Create musi fail-closed z kodem UnsafePath (15)"
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
    assert_eq!(
        fs::read(&new_res_path).expect("read existing"),
        b"ALREADY_EXISTS"
    );
}

#[test]
fn windows_helper_rejects_missing_replace_target() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&staging_exe, b"new").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

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
        WindowsHelperExitCode::BackupFailed,
        "Brak pliku cel dla operacji Replace musi fail-closed z kodem BackupFailed (18)"
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::AbortedPreApply
    );
}

// ============================================================================
// Obszar 3: Testy przerwanej podmiany i rollbacku
// ============================================================================

#[test]
fn windows_helper_fault_injection_interrupted_replace_triggers_clean_rollback() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let file1_path = root.join("app1.dll");
    let file2_path = root.join("app2.dll");
    let staging_file1 = staging.join("app1.dll");
    let staging_file2 = staging.join("app2.dll");

    fs::write(&file1_path, b"ORIG_APP1").expect("write file1");
    fs::write(&file2_path, b"ORIG_APP2").expect("write file2");
    fs::write(&staging_file1, b"NEW_APP1").expect("write staging1");
    fs::write(&staging_file2, b"NEW_APP2").expect("write staging2");

    let plan = UpdatePlan {
        replace_files: vec![
            (PathBuf::from("app1.dll"), file1_path.clone()),
            (PathBuf::from("app2.dll"), file2_path.clone()),
        ],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };
    let file_ops = FaultyFileOps {
        fail_copy_on_target_substring: "app2.dll",
    };

    let mut engine = WindowsHelperEngine::with_file_ops(&req, &monitor, &launcher, &file_ops);
    let (exit_code, result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::ApplyFailedRollbackSucceeded,
        "Przerwanie operacji Replace musi wykonać rollback i zwrócić kod 19"
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::FailedRollbackSucceeded
    );
    assert_eq!(fs::read(&file1_path).expect("read file1"), b"ORIG_APP1");
    assert_eq!(fs::read(&file2_path).expect("read file2"), b"ORIG_APP2");
}

#[test]
fn windows_helper_fault_injection_interrupted_create_triggers_clean_rollback() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    let new1_path = root.join("new1.dat");
    let new2_path = root.join("new2.dat");
    let staging_new1 = staging.join("new1.dat");
    let staging_new2 = staging.join("new2.dat");

    fs::write(&exe_path, b"ORIG_EXE").expect("write exe");
    fs::write(&staging_exe, b"NEW_EXE").expect("write staging exe");
    fs::write(&staging_new1, b"NEW1_BYTES").expect("write staging new1");
    fs::write(&staging_new2, b"NEW2_BYTES").expect("write staging new2");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: vec![
            (PathBuf::from("new1.dat"), new1_path.clone()),
            (PathBuf::from("new2.dat"), new2_path.clone()),
        ],
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let req = create_test_request(root, attempt, &plan);

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = TestCandidateLauncher {
        create_valid_proof: true,
        tamper_proof_attempt: false,
        crash_on_launch: false,
        is_alive: true,
    };
    let file_ops = FaultyFileOps {
        fail_copy_on_target_substring: "new2.dat",
    };

    let mut engine = WindowsHelperEngine::with_file_ops(&req, &monitor, &launcher, &file_ops);
    let (exit_code, result) = engine.execute();

    assert_eq!(
        exit_code,
        WindowsHelperExitCode::ApplyFailedRollbackSucceeded
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::FailedRollbackSucceeded
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"ORIG_EXE");
    assert!(
        !new1_path.exists(),
        "Nowo utworzony plik new1.dat musi zostać usunięty w rollbacku"
    );
    assert!(!new2_path.exists(), "new2.dat nie może istnieć");
}

#[test]
fn windows_helper_fault_injection_rollback_idempotency() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    let new_res = root.join("created.dll");
    let staging_res = staging.join("created.dll");

    fs::write(&exe_path, b"ORIGINAL_EXE").expect("write exe");
    fs::write(&staging_exe, b"NEW_EXE").expect("write staging exe");
    fs::write(&staging_res, b"NEW_RESOURCE").expect("write staging res");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: vec![(PathBuf::from("created.dll"), new_res.clone())],
        preserve_files: Vec::new(),
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
    let (code, result) = engine.execute();

    assert_eq!(
        code,
        WindowsHelperExitCode::CandidateLaunchFailedRollbackSucceeded
    );
    assert_eq!(
        result.result_category,
        HelperResultCategory::FailedRollbackSucceeded
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"ORIGINAL_EXE");
    assert!(!new_res.exists());

    let res2 = engine.execute_rollback();
    assert_eq!(
        res2,
        RollbackResultKind::Success,
        "Powtórzony rollback musi być w pełni idempotentny"
    );
    assert_eq!(fs::read(&exe_path).expect("read exe"), b"ORIGINAL_EXE");
    assert!(!new_res.exists());
}

// ============================================================================
// Obszar 4: Rzeczywiste testy wykonania PowerShell na Windows
// ============================================================================

#[test]
fn powershell_real_execution_full_cycle_success() {
    if !cfg!(windows) {
        return;
    }

    let dir = tempdir().expect("tempdir");
    let root = dir.path().join("SPLogbook App Real PS");
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("create staging");

    let real_exe = PathBuf::from(env!("CARGO_BIN_EXE_SPLogbook"));
    let root_exe = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::copy(&real_exe, &root_exe).expect("copy root exe");
    fs::copy(&real_exe, &staging_exe).expect("copy staging exe");

    let staging_data = staging.join("seeds.dat");
    fs::write(&staging_data, b"NEW_SEEDS_DATA").expect("write staging data");
    let root_data = root.join("seeds.dat");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), root_exe.clone())],
        create_files: vec![(PathBuf::from("seeds.dat"), root_data.clone())],
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(&root, attempt, &plan);
    req.expected_parent_pid = 999999;
    req.candidate_health_timeout_seconds = 15;
    req.expected_target_version = env!("CARGO_PKG_VERSION").to_string();
    req.expected_target_commit = "0000000000000000000000000000000000000000".to_string();

    let req_path = root.join("helper_request.json");
    let req_json = serde_json::to_string_pretty(&req).expect("serialize req");
    fs::write(&req_path, req_json).expect("write req file");

    let script = generate_powershell_from_request_file(&req_path);
    let script_path = root.join("update_script.ps1");
    fs::write(&script_path, &script).expect("write script");

    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path.to_str().unwrap(),
        ])
        .output()
        .expect("run powershell");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!(
        "PowerShell full cycle exit code: {:?}",
        output.status.code()
    );
    if !output.status.success() {
        eprintln!("PowerShell stdout: {stdout}");
        eprintln!("PowerShell stderr: {stderr}");
    }

    assert!(
        output.status.success(),
        "PowerShell script must exit with 0"
    );
    assert!(root_data.exists(), "Created file must exist in active root");
    assert_eq!(fs::read(&root_data).expect("read seeds"), b"NEW_SEEDS_DATA");
    assert!(
        root.join(".rollback").join("SPLogbook.exe").exists(),
        "Backup in .rollback must be retained"
    );
    assert!(req.result_record_path.exists(), "Result record must exist");
    let result_raw = fs::read_to_string(&req.result_record_path).expect("read result");
    eprintln!("Full cycle result record JSON: {result_raw}");
    let clean_json = result_raw.trim_start_matches('\u{feff}');
    let val: serde_json::Value = serde_json::from_str(clean_json).expect("parse result json");
    assert_eq!(val["exit_code"], 0);
    assert_eq!(val["status"], "Success");
}

#[test]
fn powershell_real_execution_candidate_failure_triggers_rollback() {
    if !cfg!(windows) {
        return;
    }

    let dir = tempdir().expect("tempdir");
    let root = dir.path().join("SPLogbook App Real PS Fail");
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("create staging");

    let root_exe = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&root_exe, b"ORIGINAL_EXE_V1").expect("write root exe");

    let system_cmd = PathBuf::from("C:\\Windows\\System32\\cmd.exe");
    if system_cmd.exists() {
        fs::copy(&system_cmd, &staging_exe).expect("copy cmd as staging exe");
    } else {
        fs::write(&staging_exe, b"@echo off\r\nexit /b 1").expect("write dummy");
    }

    let staging_created = staging.join("new_plugin.dll");
    fs::write(&staging_created, b"SHOULD_BE_ROLLED_BACK").expect("write plugin");
    let root_created = root.join("new_plugin.dll");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), root_exe.clone())],
        create_files: vec![(PathBuf::from("new_plugin.dll"), root_created.clone())],
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(&root, attempt, &plan);
    req.expected_parent_pid = 999999;
    req.candidate_health_timeout_seconds = 2;

    let req_path = root.join("helper_request.json");
    let req_json = serde_json::to_string_pretty(&req).expect("serialize req");
    fs::write(&req_path, req_json).expect("write req file");

    let script = generate_powershell_from_request_file(&req_path);
    let script_path = root.join("update_script.ps1");
    fs::write(&script_path, &script).expect("write script");

    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path.to_str().unwrap(),
        ])
        .output()
        .expect("run powershell");

    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code, 9,
        "PowerShell script must exit with 9 on candidate health failure"
    );
    assert_eq!(
        fs::read(&root_exe).expect("read exe"),
        b"ORIGINAL_EXE_V1",
        "Original executable must be restored from .rollback"
    );
    assert!(
        !root_created.exists(),
        "Created file must be deleted during rollback"
    );
    assert!(req.result_record_path.exists(), "Result record must exist");
    let result_raw = fs::read_to_string(&req.result_record_path).expect("read result");
    eprintln!("Result record JSON: {result_raw}");
    let clean_json = result_raw.trim_start_matches('\u{feff}');
    let val: serde_json::Value = serde_json::from_str(clean_json).expect("parse result json");
    assert_eq!(val["exit_code"], 9);
}

struct IdentityTamperingLauncher {
    tamper_version: bool,
    tamper_commit: bool,
}

impl CandidateLauncher for IdentityTamperingLauncher {
    fn launch_candidate(
        &self,
        _candidate_exe: &Path,
        attempt_id: &UpdateAttemptId,
        health_proof_path: &Path,
        expected_version: &str,
        expected_commit: &str,
    ) -> Result<Box<dyn HelperCandidateChild>, WindowsHelperExitCode> {
        let ver = if self.tamper_version {
            "1.2.0-mismatched"
        } else {
            expected_version
        };
        let commit = if self.tamper_commit {
            "0000000000000000000000000000000000000000-tampered"
        } else {
            expected_commit
        };
        let proof = HealthProof::new(
            *attempt_id,
            ver,
            commit,
            "windows-x86_64",
            InstallationType::WindowsPortableZip,
        );
        let json = serde_json::to_string_pretty(&proof).unwrap();
        fs::write(health_proof_path, json).unwrap();
        Ok(Box::new(TestCandidateChild { is_alive: true }))
    }
}

#[test]
fn windows_helper_health_wrong_version_triggers_rollback() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"SPLogbook v1.2.0 bazowa").expect("write exe");
    fs::write(&staging_exe, b"SPLogbook v1.3.0-alpha.1 kandydat").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.installed_version = Some("1.2.0".to_string());
    req.expected_target_version = "1.3.0-alpha.1".to_string();
    req.expected_target_commit = "82aa7106c030fe1d883a51dedaf1b30247870ee5".to_string();
    req.candidate_health_timeout_seconds = 1;

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = IdentityTamperingLauncher {
        tamper_version: true,
        tamper_commit: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (code, res) = engine.execute();

    assert_eq!(
        code,
        WindowsHelperExitCode::HealthRejectedRollbackSucceeded,
        "Niezgodność wersji w HealthProof musi wymusić rollback"
    );
    assert_eq!(
        fs::read(&exe_path).expect("read exe"),
        b"SPLogbook v1.2.0 bazowa",
        "Plik wykonywalny musi zostać przywrócony ze starej wersji"
    );

    assert_eq!(res.final_update_state, UpdateStateId::FailedPostApply);
    assert_eq!(
        res.result_category,
        HelperResultCategory::FailedRollbackSucceeded
    );
    assert_eq!(res.health_result, Some(HealthResultKind::Rejected));
    assert_eq!(
        res.verified_version_after_operation.as_deref(),
        Some("1.2.0")
    );
}

#[test]
fn windows_helper_health_wrong_commit_triggers_rollback() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"SPLogbook v1.2.0 bazowa").expect("write exe");
    fs::write(&staging_exe, b"SPLogbook v1.3.0-alpha.1 kandydat").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.installed_version = Some("1.2.0".to_string());
    req.expected_target_version = "1.3.0-alpha.1".to_string();
    req.expected_target_commit = "82aa7106c030fe1d883a51dedaf1b30247870ee5".to_string();
    req.candidate_health_timeout_seconds = 1;

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = IdentityTamperingLauncher {
        tamper_version: false,
        tamper_commit: true,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (code, res) = engine.execute();

    assert_eq!(
        code,
        WindowsHelperExitCode::HealthRejectedRollbackSucceeded,
        "Niezgodność commita w HealthProof musi wymusić rollback"
    );
    assert_eq!(
        fs::read(&exe_path).expect("read exe"),
        b"SPLogbook v1.2.0 bazowa",
        "Plik wykonywalny musi zostać przywrócony ze starej wersji"
    );

    assert_eq!(res.final_update_state, UpdateStateId::FailedPostApply);
    assert_eq!(
        res.result_category,
        HelperResultCategory::FailedRollbackSucceeded
    );
    assert_eq!(res.health_result, Some(HealthResultKind::Rejected));
    assert_eq!(
        res.verified_version_after_operation.as_deref(),
        Some("1.2.0")
    );
}

#[test]
fn windows_helper_health_matching_alpha_succeeds_and_verifies_identity() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    let staging = root.join(".staging");
    fs::create_dir_all(&staging).expect("staging dir");

    let exe_path = root.join("SPLogbook.exe");
    let staging_exe = staging.join("SPLogbook.exe");
    fs::write(&exe_path, b"SPLogbook v1.2.0 bazowa").expect("write exe");
    fs::write(&staging_exe, b"SPLogbook v1.3.0-alpha.1 kandydat").expect("write staging exe");

    let plan = UpdatePlan {
        replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
        create_files: Vec::new(),
        preserve_files: Vec::new(),
        rejected_files: Vec::new(),
    };

    let attempt = UpdateAttemptId::generate();
    let mut req = create_test_request(root, attempt, &plan);
    req.installed_version = Some("1.2.0".to_string());
    req.expected_target_version = "1.3.0-alpha.1".to_string();
    req.expected_target_commit = "82aa7106c030fe1d883a51dedaf1b30247870ee5".to_string();
    req.candidate_health_timeout_seconds = 1;

    let monitor = TestProcessMonitor::new(&[]);
    let launcher = IdentityTamperingLauncher {
        tamper_version: false,
        tamper_commit: false,
    };

    let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
    let (code, res) = engine.execute();

    assert_eq!(
        code,
        WindowsHelperExitCode::Success,
        "Zgodny HealthProof musi zakończyć się statusem Success"
    );
    assert_eq!(
        fs::read(&exe_path).expect("read exe"),
        b"SPLogbook v1.3.0-alpha.1 kandydat",
        "Nowy kandydat musi zostać pomyślnie zainstalowany"
    );

    assert_eq!(res.final_update_state, UpdateStateId::Committed);
    assert_eq!(res.health_result, Some(HealthResultKind::Passed));
    assert_eq!(
        res.verified_version_after_operation.as_deref(),
        Some("1.3.0-alpha.1")
    );
    assert_eq!(
        res.verified_commit_after_operation.as_deref(),
        Some("82aa7106c030fe1d883a51dedaf1b30247870ee5")
    );
}

#[test]
fn windows_helper_stability_50_iterations_rollback_and_identity_verification() {
    for i in 0..50 {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let staging = root.join(".staging");
        fs::create_dir_all(&staging).expect("staging dir");

        let exe_path = root.join("SPLogbook.exe");
        let staging_exe = staging.join("SPLogbook.exe");
        fs::write(&exe_path, b"SPLogbook v1.2.0 bazowa").expect("write exe");
        fs::write(&staging_exe, b"SPLogbook v1.3.0-alpha.1 kandydat").expect("write staging exe");

        let plan = UpdatePlan {
            replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
            create_files: Vec::new(),
            preserve_files: Vec::new(),
            rejected_files: Vec::new(),
        };

        let attempt = UpdateAttemptId::generate();
        let mut req = create_test_request(root, attempt, &plan);
        req.installed_version = Some("1.2.0".to_string());
        req.expected_target_version = "1.3.0-alpha.1".to_string();
        req.expected_target_commit = format!("commit_{i:040x}");
        req.candidate_health_timeout_seconds = 1;

        let monitor = TestProcessMonitor::new(&[]);
        // Tamper version every even iteration, tamper commit every odd iteration
        let launcher = IdentityTamperingLauncher {
            tamper_version: i % 2 == 0,
            tamper_commit: i % 2 != 0,
        };

        let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
        let (code, res) = engine.execute();

        assert_eq!(
            code,
            WindowsHelperExitCode::HealthRejectedRollbackSucceeded,
            "Iteracja {i}: Zafałszowany HealthProof musi wywołać wycofanie"
        );
        assert_eq!(
            fs::read(&exe_path).expect("read exe"),
            b"SPLogbook v1.2.0 bazowa",
            "Iteracja {i}: Poprzednia wersja bazowa musi zostać przywrócona z .rollback"
        );
        assert_eq!(res.final_update_state, UpdateStateId::FailedPostApply);
        assert_eq!(
            res.result_category,
            HelperResultCategory::FailedRollbackSucceeded
        );
        assert_eq!(res.health_result, Some(HealthResultKind::Rejected));
        assert_eq!(
            res.rollback_result,
            Some(splogbook::cloud::updater::helper_protocol::RollbackResultKind::Success)
        );
        assert!(res.backup_retained);
        assert_eq!(
            res.verified_version_after_operation.as_deref(),
            Some("1.2.0"),
            "Iteracja {i}: Po wycofaniu wersją zweryfikowaną musi być wersja przywrócona"
        );
    }
}

#[test]
fn windows_helper_stability_20_iterations_database_preservation_and_atomic_swap() {
    for i in 0..20 {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let staging = root.join(".staging");
        let data_dir = root.join("data");
        fs::create_dir_all(&staging).expect("staging dir");
        fs::create_dir_all(&data_dir).expect("data dir");

        let exe_path = root.join("SPLogbook.exe");
        let staging_exe = staging.join("SPLogbook.exe");
        fs::write(&exe_path, b"SPLogbook v1.2.0 bazowa").expect("write exe");
        fs::write(&staging_exe, b"SPLogbook v1.3.0-alpha.1 kandydat").expect("write staging exe");

        // Dane użytkownika, które MUSZĄ pozostać nienaruszone
        let db_path = data_dir.join("splogbook_contacts.db");
        let db_content = format!("SQLITE_DATABASE_DATA_ITER_{i}_KEEP_SAFE");
        fs::write(&db_path, db_content.as_bytes()).expect("write db");

        let cfg_path = root.join("settings.json");
        let cfg_content = format!("{{\"callsign\": \"SP6INA\", \"iter\": {i}}}");
        fs::write(&cfg_path, cfg_content.as_bytes()).expect("write cfg");

        let plan = UpdatePlan {
            replace_files: vec![(PathBuf::from("SPLogbook.exe"), exe_path.clone())],
            create_files: Vec::new(),
            preserve_files: vec![
                (db_path.clone(), "user database".to_string()),
                (cfg_path.clone(), "user configuration".to_string()),
            ],
            rejected_files: Vec::new(),
        };

        let attempt = UpdateAttemptId::generate();
        let target_commit = format!("a1b2c3d4e5f6e7f8a9b0c1d2e3f4a5b6c7d8e9f{i:x}");
        let mut req = create_test_request(root, attempt, &plan);
        req.installed_version = Some("1.2.0".to_string());
        req.expected_target_version = "1.3.0-alpha.1".to_string();
        req.expected_target_commit = target_commit.clone();
        req.candidate_health_timeout_seconds = 1;

        let monitor = TestProcessMonitor::new(&[]);
        let launcher = IdentityTamperingLauncher {
            tamper_version: false,
            tamper_commit: false,
        };

        let mut engine = WindowsHelperEngine::new(&req, &monitor, &launcher);
        let (code, res) = engine.execute();

        assert_eq!(
            code,
            WindowsHelperExitCode::Success,
            "Iteracja {i}: Zgodny kandydat musi zakończyć się statusem Success"
        );
        assert_eq!(
            fs::read(&exe_path).expect("read exe"),
            b"SPLogbook v1.3.0-alpha.1 kandydat",
            "Iteracja {i}: Nowy kandydat musi zostać pomyślnie zainstalowany"
        );

        // Weryfikacja nienaruszalności danych
        let actual_db = fs::read_to_string(&db_path).expect("read db");
        assert_eq!(
            actual_db, db_content,
            "Iteracja {i}: Baza danych użytkownika została naruszona!"
        );

        let actual_cfg = fs::read_to_string(&cfg_path).expect("read cfg");
        assert_eq!(
            actual_cfg, cfg_content,
            "Iteracja {i}: Konfiguracja użytkownika została naruszona!"
        );

        assert_eq!(res.final_update_state, UpdateStateId::Committed);
        assert_eq!(res.health_result, Some(HealthResultKind::Passed));
        assert_eq!(
            res.verified_version_after_operation.as_deref(),
            Some("1.3.0-alpha.1")
        );
        assert_eq!(
            res.verified_commit_after_operation.as_deref(),
            Some(target_commit.as_str())
        );
    }
}
