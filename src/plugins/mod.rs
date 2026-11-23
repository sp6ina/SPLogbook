// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! System pluginów użytkownika oparty na osadzonym języku **Rhai**.
//!
//! Rhai jest w pełni sandboxowany: skrypty nie mają dostępu do systemu plików,
//! sieci ani procesów, dopóki nie zarejestrujemy odpowiednich funkcji. Silnik
//! udostępnia ograniczone, bezpieczne API (logowanie, powiadomienia, licznik QSO)
//! oraz wywołuje opcjonalne haki cyklu życia zdefiniowane przez skrypt:
//! `on_startup()`, `on_qso_logged(call, band, mode, freq_mhz, is_atno)`
//! oraz `on_band_opened(band)`.

pub mod api;
pub mod bridge;
pub mod lookups;
pub mod marketplace;

use bridge::{PluginCommand, PluginSnapshot};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Informacja o pojedynczym skrypcie pluginu (ładowanie i błędy).
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub path: String,
    pub loaded: bool,
    pub error: Option<String>,
}

/// Współdzielony stan roboczy silnika (logi, powiadomienia, licznik QSO,
/// migawka stanu oraz kolejka poleceń do wykonania przez aplikację).
#[derive(Debug, Default)]
struct PluginState {
    log: Mutex<Vec<String>>,
    notifications: Mutex<Vec<String>>,
    qso_count: AtomicI64,
    snapshot: Arc<RwLock<PluginSnapshot>>,
    commands: Mutex<Vec<PluginCommand>>,
}

/// Dodaje polecenie do kolejki współdzielonego stanu pluginów.
fn push_command(state: &Arc<PluginState>, cmd: PluginCommand) {
    let mut commands = state
        .commands
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if commands.len() < 256 {
        commands.push(cmd);
    }
}

/// Rejestruje bezargumentowy getter Rhai odczytujący wartość z migawki stanu.
fn register_snapshot_getter<T: std::any::Any + Clone + Send + Sync>(
    engine: &mut rhai::Engine,
    state: &Arc<PluginState>,
    name: &str,
    get: impl Fn(&PluginSnapshot) -> T + Send + Sync + 'static,
) {
    let snap = Arc::clone(&state.snapshot);
    engine.register_fn(name, move || {
        get(&snap
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner))
    });
}

/// Silnik pluginów Rhai. Wczytuje i uruchamia skrypty `.rhai` z katalogu pluginów.
pub struct PluginEngine {
    engine: rhai::Engine,
    state: Arc<PluginState>,
    /// Zestaw załadowanych pluginów: nazwa skryptu -> skompilowane AST.
    scripts: HashMap<String, rhai::AST>,
    /// Metadane o każdym pliku w katalogu pluginów (także o błędach).
    infos: Vec<PluginInfo>,
    enabled: bool,
    api: Arc<api::ApiState>,
}

/// Dane przekazywane do haka `on_qso_logged`.
#[derive(Debug, Clone, Default)]
pub struct QsoHookContext {
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub freq_mhz: f64,
    pub is_atno: bool,
}

/// Błędy ewaluacji skryptów pluginów Rhai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginEvalError {
    Compile(String),
    Execution(String),
}

impl std::fmt::Display for PluginEvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(e) => write!(f, "Script compilation error: {e}"),
            Self::Execution(e) => write!(f, "Script execution error: {e}"),
        }
    }
}

impl std::error::Error for PluginEvalError {}

impl PluginEngine {
    /// Tworzy pusty silnik z zarejestrowanym bezpiecznym API.
    pub fn new() -> Self {
        let mut engine = rhai::Engine::new();
        // Limity chroniące przed nadmiernym zużyciem pamięci / DoS ze skryptu.
        engine.set_max_string_size(64 * 1024);
        engine.set_max_expr_depths(64, 32);
        engine.set_max_operations(100_000);
        engine.set_max_array_size(4096);
        engine.set_max_map_size(1024);
        engine.set_max_call_levels(32);

        let state = Arc::new(PluginState::default());
        let api = Arc::new(api::ApiState::default());
        api::register(&mut engine, &api);

        // Bezpieczne API udostępniane skryptom
        let log_state = Arc::clone(&state);
        engine.register_fn("log", move |msg: &str| {
            let mut log = log_state
                .log
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            log.push(msg.to_string());
            if log.len() > 500 {
                log.remove(0);
            }
        });

        let notify_state = Arc::clone(&state);
        engine.register_fn("notify", move |msg: &str| {
            let mut notifications = notify_state
                .notifications
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            notifications.push(msg.to_string());
            if notifications.len() > 100 {
                notifications.remove(0);
            }
        });

        let count_state = Arc::clone(&state);
        engine.register_fn("qso_count", move || {
            count_state.qso_count.load(Ordering::Relaxed)
        });

        // Gettery stanu (radio, rotor, nagrody, stacja, ostatnia łączność)
        register_snapshot_getter(&mut engine, &state, "rig_freq_mhz", |s| s.rig_freq_mhz);
        register_snapshot_getter(&mut engine, &state, "rig_mode", |s| s.rig_mode.clone());
        register_snapshot_getter(&mut engine, &state, "rig_band", |s| s.rig_band.clone());
        register_snapshot_getter(&mut engine, &state, "rig_connected", |s| s.rig_connected);
        register_snapshot_getter(&mut engine, &state, "rotor_azimuth", |s| {
            f64::from(s.rotor_azimuth_deg)
        });
        register_snapshot_getter(&mut engine, &state, "rotor_elevation", |s| {
            f64::from(s.rotor_elevation_deg)
        });
        register_snapshot_getter(&mut engine, &state, "my_call", |s| s.my_call.clone());
        register_snapshot_getter(&mut engine, &state, "dxcc_worked", |s| s.awards.dxcc_worked);
        register_snapshot_getter(&mut engine, &state, "dxcc_confirmed", |s| {
            s.awards.dxcc_confirmed
        });
        register_snapshot_getter(&mut engine, &state, "waz_worked", |s| s.awards.waz_worked);
        register_snapshot_getter(&mut engine, &state, "was_worked", |s| s.awards.was_worked);
        register_snapshot_getter(&mut engine, &state, "wac_worked", |s| s.awards.wac_worked);
        register_snapshot_getter(&mut engine, &state, "iota_worked", |s| s.awards.iota_worked);
        register_snapshot_getter(&mut engine, &state, "pota_parks_worked", |s| {
            s.awards.pota_parks_worked
        });
        register_snapshot_getter(&mut engine, &state, "sota_summits_worked", |s| {
            s.awards.sota_summits_worked
        });
        register_snapshot_getter(&mut engine, &state, "pga_gminas_worked", |s| {
            s.awards.pga_gminas_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("qso_field", move |name: &str| {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .last_qso
                .field(name)
        });

        // Akcje (kolejkowane jako PluginCommand i wykonywane przez aplikację)
        let cmd = Arc::clone(&state);
        engine.register_fn("send_cw", move |text: &str| {
            push_command(
                &cmd,
                PluginCommand::SendCw {
                    text: text.to_string(),
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("send_voice", move |text: &str| {
            push_command(
                &cmd,
                PluginCommand::SendVoice {
                    text: text.to_string(),
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("rotate", move |azimuth_deg: f32| {
            push_command(
                &cmd,
                PluginCommand::Rotate {
                    azimuth_deg,
                    elevation_deg: 0.0,
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("rotate", move |azimuth_deg: f32, elevation_deg: f32| {
            push_command(
                &cmd,
                PluginCommand::Rotate {
                    azimuth_deg,
                    elevation_deg,
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("rotate", move |azimuth_deg: f64| {
            push_command(
                &cmd,
                PluginCommand::Rotate {
                    azimuth_deg: azimuth_deg as f32,
                    elevation_deg: 0.0,
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("rotate", move |azimuth_deg: f64, elevation_deg: f64| {
            push_command(
                &cmd,
                PluginCommand::Rotate {
                    azimuth_deg: azimuth_deg as f32,
                    elevation_deg: elevation_deg as f32,
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("rotate", move |azimuth_deg: i64| {
            push_command(
                &cmd,
                PluginCommand::Rotate {
                    azimuth_deg: azimuth_deg as f32,
                    elevation_deg: 0.0,
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("rotate", move |azimuth_deg: i64, elevation_deg: i64| {
            push_command(
                &cmd,
                PluginCommand::Rotate {
                    azimuth_deg: azimuth_deg as f32,
                    elevation_deg: elevation_deg as f32,
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn(
            "spot",
            move |dx_call: &str, freq_khz: f64, comment: &str| {
                push_command(
                    &cmd,
                    PluginCommand::Spot {
                        dx_call: dx_call.to_string(),
                        freq_khz,
                        comment: comment.to_string(),
                    },
                );
            },
        );
        let cmd = Arc::clone(&state);
        engine.register_fn("set_qso_field", move |field: &str, value: &str| {
            push_command(
                &cmd,
                PluginCommand::SetQsoField {
                    field: field.to_string(),
                    value: value.to_string(),
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("play_sound", move |name: &str| {
            push_command(
                &cmd,
                PluginCommand::PlaySound {
                    name: name.to_string(),
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("pota_lookup", move |reference: &str| {
            push_command(
                &cmd,
                PluginCommand::PotaLookup {
                    reference: reference.to_string(),
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("switch_page", move |name: &str| {
            push_command(
                &cmd,
                PluginCommand::SwitchPage {
                    name: name.to_string(),
                },
            );
        });
        let cmd = Arc::clone(&state);
        engine.register_fn("sota_lookup", move |reference: &str| {
            push_command(
                &cmd,
                PluginCommand::SotaLookup {
                    reference: reference.to_string(),
                },
            );
        });

        Self {
            engine,
            state,
            scripts: HashMap::new(),
            infos: Vec::new(),
            enabled: true,
            api,
        }
    }

    /// Ustawia migawkę stanu widoczną dla getterów pluginów.
    pub fn set_snapshot(&self, snapshot: PluginSnapshot) {
        let mut snapshot_guard = self
            .state
            .snapshot
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *snapshot_guard = snapshot;
    }

    /// Pobiera zakolejkowane przez pluginy polecenia i czyści bufor.
    pub fn drain_commands(&self) -> Vec<PluginCommand> {
        let mut commands = self
            .state
            .commands
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *commands)
    }

    /// Włącza/wyłącza wykonywanie pluginów (bez wyładowywania skryptów).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Ustawia licznik łączności widoczny dla skryptów jako `qso_count()`.
    pub fn set_qso_count(&self, count: i64) {
        self.state.qso_count.store(count, Ordering::Relaxed);
    }

    /// Wczytuje wszystkie pliki `*.rhai` z podanego katalogu.
    /// Nie rzuca błędów — błędne skrypty są odnotowane w `infos()` i pomijane.
    pub fn load_dir(&mut self, dir: &Path) {
        self.scripts.clear();
        self.infos.clear();

        let Ok(entries) = std::fs::read_dir(dir) else {
            return; // katalog nie istnieje — brak pluginów
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "rhai"))
            .collect();
        paths.sort();

        for path in paths {
            let name = path.file_stem().map_or_else(
                || path.display().to_string(),
                |s| s.to_string_lossy().to_string(),
            );
            self.load_script(&name, &path);
        }
    }

    /// Wczytuje i kompiluje pojedynczy skrypt. W razie błędu zapisuje go w `infos()`.
    pub fn load_script(&mut self, name: &str, path: &Path) {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.infos.push(PluginInfo {
                    name: name.to_string(),
                    path: path.display().to_string(),
                    loaded: false,
                    error: Some(format!("Nie można odczytać pliku: {e}")),
                });
                return;
            }
        };

        match self.engine.compile(&source) {
            Ok(ast) => {
                self.scripts.insert(name.to_string(), ast);
                self.infos.push(PluginInfo {
                    name: name.to_string(),
                    path: path.display().to_string(),
                    loaded: true,
                    error: None,
                });
            }
            Err(e) => {
                self.infos.push(PluginInfo {
                    name: name.to_string(),
                    path: path.display().to_string(),
                    loaded: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    /// Kompiluje i uruchamia skrypt wprost z kodu źródłowego (testy / podgląd).
    pub fn eval(&mut self, source: &str) -> Result<(), PluginEvalError> {
        let ast = self
            .engine
            .compile(source)
            .map_err(|e| PluginEvalError::Compile(e.to_string()))?;
        self.engine
            .run_ast(&ast)
            .map_err(|e| PluginEvalError::Execution(e.to_string()))?;
        Ok(())
    }

    /// Zwraca metadane o wszystkich wykrytych pluginach.
    pub fn infos(&self) -> &[PluginInfo] {
        &self.infos
    }

    /// Pobiera zebrane komunikaty `log(...)` i czyści bufor.
    pub fn drain_log(&self) -> Vec<String> {
        let mut log = self
            .state
            .log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *log)
    }

    /// Pobiera zebrane powiadomienia `notify(...)` i czyści bufor.
    pub fn drain_notifications(&self) -> Vec<String> {
        let mut notifications = self
            .state
            .notifications
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *notifications)
    }

    fn run_hook(&self, hook: &str, args: &[rhai::Dynamic]) {
        if !self.enabled {
            return;
        }
        for (name, ast) in &self.scripts {
            // Pomijamy skrypty bez zdefiniowanego haka — `call_fn` rzuciłby błąd.
            if !ast.iter_functions().any(|f| f.name == hook) {
                continue;
            }
            self.api.begin_call(name);
            let result = self.engine.call_fn::<rhai::Dynamic>(
                &mut rhai::Scope::new(),
                ast,
                hook,
                args.to_vec(),
            );
            self.api.end_call();
            if let Err(e) = result {
                let mut log = self
                    .state
                    .log
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                log.push(format!("[{name}] {hook}: {e}"));
                if log.len() > 500 {
                    log.remove(0);
                }
            }
        }
    }

    /// Wywołuje hak `on_startup()` we wszystkich pluginach.
    pub fn run_startup(&self) {
        self.run_hook("on_startup", &[]);
    }

    /// Wywołuje hak `on_qso_logged(call_sign, band, mode, freq_mhz, is_atno)`.
    /// (Parametr `call_sign` zamiast `call`, ponieważ `call` to słowo zastrzeżone w Rhai.)
    pub fn run_on_qso_logged(&self, ctx: &QsoHookContext) {
        let args = vec![
            ctx.callsign.clone().into(),
            ctx.band.clone().into(),
            ctx.mode.clone().into(),
            ctx.freq_mhz.into(),
            ctx.is_atno.into(),
        ];
        self.run_hook("on_qso_logged", &args);
    }

    /// Wywołuje hak `on_band_opened(band)`.
    pub fn run_on_band_opened(&self, band: &str) {
        self.run_hook("on_band_opened", &[band.to_string().into()]);
    }

    /// Wywołuje hak `on_workspace_changed(name)` po przełączeniu profilu
    /// układu operatorskiego (workspace).
    pub fn run_on_workspace_changed(&self, name: &str) {
        self.run_hook("on_workspace_changed", &[name.to_string().into()]);
    }

    /// Wywołuje hak `on_dx_spot(spotter, dx_call, freq_khz, band, comment, is_ft8)`.
    pub fn run_on_dx_spot(
        &self,
        spotter: &str,
        dx_call: &str,
        freq_khz: f64,
        band: &str,
        comment: &str,
        is_ft8: bool,
    ) {
        self.run_hook(
            "on_dx_spot",
            &[
                spotter.to_string().into(),
                dx_call.to_string().into(),
                freq_khz.into(),
                band.to_string().into(),
                comment.to_string().into(),
                is_ft8.into(),
            ],
        );
    }

    /// Wywołuje hak `on_rig_state(freq_mhz, mode, connected)`.
    pub fn run_on_rig_state(&self, freq_mhz: f64, mode: &str, connected: bool) {
        self.run_hook(
            "on_rig_state",
            &[freq_mhz.into(), mode.to_string().into(), connected.into()],
        );
    }

    /// Wywołuje hak `on_pota_info(reference, name, active)` po zapytaniu POTA.
    pub fn run_on_pota_info(&self, reference: &str, name: &str, active: bool) {
        self.run_hook(
            "on_pota_info",
            &[
                reference.to_string().into(),
                name.to_string().into(),
                active.into(),
            ],
        );
    }

    /// Answers for `worked_before` and `dxcc_of`.
    pub fn set_queries(&self, queries: Arc<dyn api::PluginQueries>) {
        self.api.set_queries(queries);
    }

    /// Band plan used by `bandplan_mode`.
    pub fn set_bandplan(&self, plan: crate::core::bandplan::BandPlan) {
        self.api.set_bandplan(plan);
    }

    /// Interface language code returned by `language()`.
    pub fn set_language(&self, code: &str) {
        self.api.set_language(code);
    }

    /// Whether any loaded script defines `hook`; lets callers skip preparing arguments.
    pub fn has_hook(&self, hook: &str) -> bool {
        self.enabled
            && self
                .scripts
                .values()
                .any(|ast| ast.iter_functions().any(|f| f.name == hook))
    }

    /// Wywołuje hak `on_contest_qso(call_sign, band, mode, sent, received, contest)` po
    /// zapisie łączności w module zawodów.
    pub fn run_on_contest_qso(&self, ctx: &ContestQsoHookContext) {
        self.run_hook(
            "on_contest_qso",
            &[
                ctx.callsign.clone().into(),
                ctx.band.clone().into(),
                ctx.mode.clone().into(),
                ctx.sent.clone().into(),
                ctx.received.clone().into(),
                ctx.contest.clone().into(),
            ],
        );
    }

    /// Wywołuje hak `on_wsjtx_decode(call_sign, grid, snr, freq_hz, message, is_cq, mark)`.
    pub fn run_on_wsjtx_decode(&self, ctx: &DecodeHookContext) {
        self.run_hook(
            "on_wsjtx_decode",
            &[
                ctx.callsign.clone().into(),
                ctx.grid.clone().into(),
                ctx.snr.into(),
                ctx.freq_hz.into(),
                ctx.message.clone().into(),
                ctx.is_cq.into(),
                ctx.mark.clone().into(),
            ],
        );
    }

    /// Wywołuje hak `on_spot_badge(dx_call, band, freq_khz, badge)` dla nowego spotu;
    /// `badge` to `new_dxcc`, `new_band`, `new_mode`, `worked` albo `standard`.
    pub fn run_on_spot_badge(&self, dx_call: &str, band: &str, freq_khz: f64, badge: &str) {
        self.run_hook(
            "on_spot_badge",
            &[
                dx_call.to_string().into(),
                band.to_string().into(),
                freq_khz.into(),
                badge.to_string().into(),
            ],
        );
    }

    /// Wywołuje hak `on_sota_info(reference, name, points)` po zapytaniu SOTA.
    pub fn run_on_sota_info(&self, reference: &str, name: &str, points: i64) {
        self.run_hook(
            "on_sota_info",
            &[
                reference.to_string().into(),
                name.to_string().into(),
                points.into(),
            ],
        );
    }
}

/// Dane przekazywane do haka `on_contest_qso`.
#[derive(Debug, Clone, Default)]
pub struct ContestQsoHookContext {
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub sent: String,
    pub received: String,
    pub contest: String,
}

/// Dane przekazywane do haka `on_wsjtx_decode`; `mark` to `new_dxcc`, `new_band`,
/// `worked` albo `other`.
#[derive(Debug, Clone, Default)]
pub struct DecodeHookContext {
    pub callsign: String,
    pub grid: String,
    pub snr: i64,
    pub freq_hz: i64,
    pub message: String,
    pub is_cq: bool,
    pub mark: String,
}

impl Default for PluginEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_plugin(name: &str, source: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "splogbook_plugin_test_{}_{}",
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, source).unwrap();
        path
    }

    #[test]
    fn loads_script_and_runs_startup_log() {
        let path = temp_plugin(
            "startup.rhai",
            r#"
                fn on_startup() {
                    log("hello from rhai");
                }
            "#,
        );
        let mut engine = PluginEngine::new();
        engine.load_script("startup", &path);
        engine.run_startup();
        assert_eq!(engine.drain_log(), vec!["hello from rhai".to_string()]);
    }

    #[test]
    fn qso_hook_receives_context() {
        let path = temp_plugin(
            "qso.rhai",
            r#"
                fn on_qso_logged(call_sign, band, mode, freq, atno) {
                    log(call_sign + "|" + band + "|" + mode);
                    notify("atno=" + atno);
                }
            "#,
        );
        let mut engine = PluginEngine::new();
        engine.load_script("qso", &path);
        engine.run_on_qso_logged(&QsoHookContext {
            callsign: "K1ABC".into(),
            band: "20m".into(),
            mode: "SSB".into(),
            freq_mhz: 14.195,
            is_atno: true,
        });
        assert_eq!(engine.drain_log(), vec!["K1ABC|20m|SSB".to_string()]);
        assert_eq!(engine.drain_notifications(), vec!["atno=true".to_string()]);
    }

    #[test]
    fn disabled_engine_does_not_run_hooks() {
        let mut engine = PluginEngine::new();
        engine
            .eval(
                r#"
                    fn on_startup() { log("should not run"); }
                "#,
            )
            .unwrap();
        engine.set_enabled(false);
        engine.run_startup();
        assert_eq!(engine.drain_log(), [] as [String; 0]);
    }

    #[test]
    fn load_script_reports_errors_without_panicking() {
        let path = temp_plugin("broken.rhai", "fn broken( {");
        let mut engine = PluginEngine::new();
        engine.load_script("broken", &path);
        let info = &engine.infos()[0];
        assert!(!info.loaded);
        assert!(info.error.is_some());
    }

    struct FakeLog;

    impl api::PluginQueries for FakeLog {
        fn worked_before(
            &self,
            callsign: &str,
            band: Option<&str>,
            _mode: Option<&str>,
        ) -> Option<bool> {
            Some(callsign == "SP9ABC" && band.is_none_or(|b| b == "20m"))
        }

        fn dxcc_of(&self, callsign: &str) -> Option<api::DxccInfo> {
            callsign.starts_with("SP").then(|| api::DxccInfo {
                country: "Poland".to_string(),
                dxcc: 269,
                continent: "EU".to_string(),
                cq_zone: 15,
                itu_zone: 28,
            })
        }
    }

    fn engine_with(name: &str, source: &str) -> PluginEngine {
        let path = temp_plugin(&format!("{name}.rhai"), source);
        let mut engine = PluginEngine::new();
        engine.load_script(name, &path);
        assert!(engine.infos()[0].loaded, "{:?}", engine.infos()[0].error);
        engine
    }

    #[test]
    fn new_hooks_receive_their_arguments() {
        let engine = engine_with(
            "hooks",
            r#"
                fn on_contest_qso(call_sign, band, mode, sent, received, contest) {
                    log("contest " + call_sign + " " + band + " " + mode + " " + sent + " " + received + " " + contest);
                }
                fn on_wsjtx_decode(call_sign, grid, snr, freq_hz, message, is_cq, mark) {
                    log("decode " + call_sign + " " + grid + " " + snr + " " + freq_hz + " " + is_cq + " " + mark);
                }
                fn on_spot_badge(dx_call, band, freq_khz, badge) {
                    log("badge " + dx_call + " " + band + " " + badge);
                }
            "#,
        );
        assert!(engine.has_hook("on_wsjtx_decode"));
        assert!(!engine.has_hook("on_startup"));
        engine.run_on_contest_qso(&ContestQsoHookContext {
            callsign: "SP9ABC".into(),
            band: "20m".into(),
            mode: "CW".into(),
            sent: "001".into(),
            received: "015".into(),
            contest: "SP-DX".into(),
        });
        engine.run_on_wsjtx_decode(&DecodeHookContext {
            callsign: "K1ABC".into(),
            grid: "FN42".into(),
            snr: -12,
            freq_hz: 14_075_500,
            message: "CQ K1ABC FN42".into(),
            is_cq: true,
            mark: "new_dxcc".into(),
        });
        engine.run_on_spot_badge("JA1ABC", "15m", 21_025.0, "new_band");
        assert_eq!(
            engine.drain_log(),
            [
                "contest SP9ABC 20m CW 001 015 SP-DX",
                "decode K1ABC FN42 -12 14075500 true new_dxcc",
                "badge JA1ABC 15m new_band",
            ]
        );
    }

    #[test]
    fn read_only_functions_and_memory_work_from_scripts() {
        let engine = engine_with(
            "api",
            r#"
                fn on_band_opened(band) {
                    let n = memory_get("seen");
                    let count = if n == "" { 1 } else { parse_int(n) + 1 };
                    memory_set("seen", count.to_string());
                    let info = dxcc_of("SP9ABC");
                    log(band_of(14.074) + " " + bandplan_mode(14.025) + " " + language()
                        + " " + worked_before("sp9abc") + " " + worked_before("SP9ABC", "40m", "CW")
                        + " " + info.dxcc + " " + count + " " + utc_date().len() + " " + utc_time().len());
                }
            "#,
        );
        engine.set_queries(Arc::new(FakeLog));
        engine.set_language("en");
        engine.run_on_band_opened("20m");
        engine.run_on_band_opened("20m");
        assert_eq!(
            engine.drain_log(),
            [
                "20m CW en true false 269 1 10 8",
                "20m CW en true false 269 2 10 8",
            ]
        );
    }

    #[test]
    fn memory_is_kept_apart_between_plugins() {
        let mut engine = PluginEngine::new();
        for (name, value) in [("a", "1"), ("b", "2")] {
            let source = format!(
                r#"
                    fn on_startup() {{
                        if memory_get("v") == "" {{ memory_set("v", "{value}"); }}
                        log("{name}=" + memory_get("v"));
                    }}
                "#
            );
            let path = temp_plugin(&format!("{name}.rhai"), &source);
            engine.load_script(name, &path);
        }
        engine.run_startup();
        engine.run_startup();
        let mut log = engine.drain_log();
        log.sort();
        assert_eq!(log, ["a=1", "a=1", "b=2", "b=2"]);
    }

    #[test]
    fn switch_page_is_a_queued_command() {
        let engine = engine_with("page", r#"fn on_band_opened(band) { switch_page("VHF"); }"#);
        engine.run_on_band_opened("6m");
        let commands = engine.drain_commands();
        assert!(matches!(
            commands.as_slice(),
            [bridge::PluginCommand::SwitchPage { name }] if name == "VHF"
        ));
    }

    #[test]
    fn runaway_script_is_stopped_and_logged() {
        let engine = engine_with(
            "runaway",
            r"
                fn on_startup() {
                    let x = 0;
                    loop { x += 1; }
                }
            ",
        );
        let started = std::time::Instant::now();
        engine.run_startup();
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        let log = engine.drain_log();
        assert_eq!(log.len(), 1);
        assert!(log[0].starts_with("[runaway] on_startup:"), "{log:?}");
    }

    #[test]
    fn qso_count_reflects_state() {
        let mut engine = PluginEngine::new();
        engine.set_qso_count(7);
        assert_eq!(
            engine.eval("if qso_count() != 7 { throw \"bad\"; }"),
            Ok(())
        );
    }

    #[test]
    fn workspace_changed_hook_receives_name() {
        let path = temp_plugin(
            "workspace.rhai",
            r#"
                fn on_workspace_changed(name) {
                    log("ws=" + name);
                }
            "#,
        );
        let mut engine = PluginEngine::new();
        engine.load_script("workspace", &path);
        engine.run_on_workspace_changed("Kontest");
        assert_eq!(engine.drain_log(), vec!["ws=Kontest".to_string()]);
    }

    #[test]
    fn sandbox_has_no_filesystem_access() {
        let mut engine = PluginEngine::new();
        // Próba otwarcia pliku powinna zakończyć się błędem — Rhai bez
        // zarejestrowanych funkcji IO nie ma dostępu do systemu plików.
        let r = engine.eval(r#"let f = file_open("x");"#);
        assert!(r.is_err());
    }
}
