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
    let mut v = state
        .commands
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if v.len() < 256 {
        v.push(cmd);
    }
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

        // --- Bezpieczne API udostępniane skryptom ---
        let log_state = Arc::clone(&state);
        engine.register_fn("log", move |msg: &str| {
            let mut v = log_state
                .log
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            v.push(msg.to_string());
            if v.len() > 500 {
                v.remove(0);
            }
        });

        let notify_state = Arc::clone(&state);
        engine.register_fn("notify", move |msg: &str| {
            let mut v = notify_state
                .notifications
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            v.push(msg.to_string());
            if v.len() > 100 {
                v.remove(0);
            }
        });

        let count_state = Arc::clone(&state);
        engine.register_fn("qso_count", move || {
            count_state.qso_count.load(Ordering::Relaxed)
        });

        // --- Gettery stanu (radio, rotor, nagrody, stacja, ostatnia łączność) ---
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("rig_freq_mhz", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .rig_freq_mhz
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("rig_mode", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .rig_mode
                .clone()
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("rig_band", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .rig_band
                .clone()
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("rig_connected", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .rig_connected
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("rotor_azimuth", move || {
            f64::from(
                snap.read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .rotor_azimuth_deg,
            )
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("rotor_elevation", move || {
            f64::from(
                snap.read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .rotor_elevation_deg,
            )
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("my_call", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .my_call
                .clone()
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("dxcc_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .dxcc_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("dxcc_confirmed", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .dxcc_confirmed
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("waz_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .waz_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("was_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .was_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("wac_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .wac_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("iota_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .iota_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("pota_parks_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .pota_parks_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("sota_summits_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .sota_summits_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("pga_gminas_worked", move || {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .awards
                .pga_gminas_worked
        });
        let snap = Arc::clone(&state.snapshot);
        engine.register_fn("qso_field", move |name: &str| {
            snap.read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .last_qso
                .field(name)
        });

        // --- Akcje (kolejkowane jako PluginCommand i wykonywane przez aplikację) ---
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
        }
    }

    /// Ustawia migawkę stanu widoczną dla getterów pluginów.
    pub fn set_snapshot(&self, snapshot: PluginSnapshot) {
        let mut w = self
            .state
            .snapshot
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *w = snapshot;
    }

    /// Pobiera zakolejkowane przez pluginy polecenia i czyści bufor.
    pub fn drain_commands(&self) -> Vec<PluginCommand> {
        let mut v = self
            .state
            .commands
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *v)
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
    pub fn eval(&mut self, source: &str) -> Result<(), String> {
        let ast = self.engine.compile(source).map_err(|e| e.to_string())?;
        self.engine.run_ast(&ast).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Zwraca metadane o wszystkich wykrytych pluginach.
    pub fn infos(&self) -> &[PluginInfo] {
        &self.infos
    }

    /// Pobiera zebrane komunikaty `log(...)` i czyści bufor.
    pub fn drain_log(&self) -> Vec<String> {
        let mut v = self
            .state
            .log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *v)
    }

    /// Pobiera zebrane powiadomienia `notify(...)` i czyści bufor.
    pub fn drain_notifications(&self) -> Vec<String> {
        let mut v = self
            .state
            .notifications
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *v)
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
            if let Err(e) = self.engine.call_fn::<rhai::Dynamic>(
                &mut rhai::Scope::new(),
                ast,
                hook,
                args.to_vec(),
            ) {
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
        assert!(engine.drain_log().is_empty());
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
