use super::SpLogApp;
use crate::core::i18n::{Msg, key, tr};

/// Polecenie dla rotctld wykonywane w tle.
#[derive(Debug, Clone, Copy)]
enum RotorCommand {
    Position {
        azimuth_deg: f32,
        elevation_deg: f32,
    },
    Stop,
}

/// Komunikat dla operatora: dlaczego rotator nie wykonał polecenia.
fn rotor_failure_msg(err: &crate::cat::rotor::RotctldError, host: &str, port: u16) -> Msg {
    use crate::cat::rotor::RotctldError;
    use std::io::ErrorKind;

    let msg = match err {
        RotctldError::Rejected(code) => Msg::new(key::ROTOR_CMD_REJECTED).arg("code", code),
        RotctldError::Io(e) if e.kind() == ErrorKind::TimedOut => Msg::new(key::ROTOR_CMD_TIMEOUT),
        RotctldError::Io(e) if e.kind() == ErrorKind::ConnectionRefused => {
            Msg::new(key::ROTOR_CMD_UNREACHABLE)
        }
        RotctldError::Io(e) => Msg::new(key::ROTOR_CMD_FAILED).arg("err", e),
    };
    msg.arg("host", host).arg("port", port)
}

impl SpLogApp {
    /// Obraca antenę na podany azymut przez protokół rotctld (Hamlib)
    pub fn rotate_antenna_to(&mut self, azimuth_deg: f32) {
        self.rotate_antenna_to_el(azimuth_deg, 0.0);
    }

    /// Obraca antenę na zadany azymut i elewację (także akcja pluginu `rotate`).
    ///
    /// Powiadomienie pojawia się dopiero po odpowiedzi rotctld: potwierdzenie albo
    /// błąd (brak połączenia, przekroczony czas, odrzucenie polecenia).
    pub fn rotate_antenna_to_el(&mut self, azimuth_deg: f32, elevation_deg: f32) {
        self.cat.rotor_state.azimuth_deg = azimuth_deg;
        self.cat.rotor_state.elevation_deg = elevation_deg;
        let done = if elevation_deg == 0.0 {
            Msg::new(key::ROTOR_CMD_SENT_AZ)
                .arg("az", format!("{azimuth_deg:.0}"))
                .arg("host", &self.config.rotor_host)
                .arg("port", self.config.rotor_port)
        } else {
            Msg::new(key::ROTOR_CMD_SENT_AZ_EL)
                .arg("az", format!("{azimuth_deg:.0}"))
                .arg("el", format!("{elevation_deg:.0}"))
        };
        self.spawn_rotor_command(
            RotorCommand::Position {
                azimuth_deg,
                elevation_deg,
            },
            Some(done),
        );
    }

    fn spawn_rotor_command(&self, command: RotorCommand, done: Option<Msg>) {
        use crate::cat::rotor::RotorClient;
        use crate::gui::state::ToastSeverity;

        let host = self.config.rotor_host.clone();
        let port = self.config.rotor_port;
        let notices = self.runtime.notice_sender();
        tokio::spawn(async move {
            let result = match command {
                RotorCommand::Position {
                    azimuth_deg,
                    elevation_deg,
                } => RotorClient::set_position(&host, port, azimuth_deg, elevation_deg).await,
                RotorCommand::Stop => RotorClient::stop(&host, port).await,
            };
            match result {
                Ok(()) => {
                    if let Some(msg) = done {
                        notices.send(ToastSeverity::Info, msg);
                    }
                }
                Err(err) => {
                    notices.send(ToastSeverity::Error, rotor_failure_msg(&err, &host, port));
                }
            }
        });
    }

    /// Centralne wejście warstwy GUI do intencji PTT (F-17).
    ///
    /// Jedyne miejsce, w którym interakcja użytkownika (skrót, przycisk ekranowy, paleta
    /// poleceń) może zamienić się w żądanie do `TxArbiter`. Buduje migawkę wejścia z produkcyjnych
    /// bramek egui, uzyskuje wynik bramki F-15 i deleguje decyzję do
    /// [`crate::gui::ptt_input::dispatch`].
    #[must_use]
    pub fn dispatch_ptt(
        &mut self,
        source: crate::gui::ptt_input::PttActionSource,
        intent: crate::gui::ptt_input::PttIntent,
    ) -> crate::gui::ptt_input::PttDispatchOutcome {
        let snapshot = self.ptt_input_snapshot(source, intent);
        let mut port = CatTransmitPort::from_runtime(&self.cat.runtime);
        self.dispatch_ptt_with_port(snapshot, &mut port)
    }

    /// Rdzeń dispatchera PTT z jawnie wstrzykniętym kanałem nadawczym.
    ///
    /// Kanał jest wstrzykiwany wyłącznie po to, aby testy mogły **policzyć** wywołania kanału
    /// nadawczego. Autoryzacja F-15 nie jest parametrem: pochodzi z produkcyjnej bramki
    /// [`crate::cat::hil::authorization::evaluate_gui_ptt_authorization`], więc warstwa GUI nie
    /// może jej podać ani sfałszować.
    pub(crate) fn dispatch_ptt_with_port(
        &mut self,
        snapshot: crate::gui::ptt_input::PttInputSnapshot,
        port: &mut dyn crate::gui::ptt_input::PttTransmitPort,
    ) -> crate::gui::ptt_input::PttDispatchOutcome {
        let tx_state = self.ptt_transmit_state();
        let authorization = crate::cat::hil::authorization::evaluate_gui_ptt_authorization();
        let outcome = crate::gui::ptt_input::dispatch(&snapshot, tx_state, authorization, port);
        self.record_ptt_outcome(outcome);
        outcome
    }

    /// Buduje migawkę wejścia PTT dla wskazanego źródła i intencji.
    ///
    /// Bramki pochodzą z bramek wyliczonych w bieżącej klatce przez
    /// [`Self::refresh_shortcut_gates`]; gdy nie zostały jeszcze wyliczone, pozostają zamknięte.
    #[must_use]
    pub fn ptt_input_snapshot(
        &self,
        source: crate::gui::ptt_input::PttActionSource,
        intent: crate::gui::ptt_input::PttIntent,
    ) -> crate::gui::ptt_input::PttInputSnapshot {
        use crate::gui::ptt_input::{PttActionSource, PttInputSnapshot};
        let gates = self.ptt.gates;
        match source {
            // Skrót klawiaturowy: rejestr skrótów wymusił już dokładny kontrakt modyfikatorów.
            PttActionSource::KeyboardShortcut => PttInputSnapshot {
                source,
                intent,
                pressed: true,
                repeat: false,
                modifiers_ok: true,
                gates,
            },
            // Wskaźnik/paleta/zwolnienie po utracie fokusu: brak zdarzenia klawiatury.
            PttActionSource::ScreenButton
            | PttActionSource::CommandPalette
            | PttActionSource::FocusLossRelease => PttInputSnapshot::pointer(source, intent, gates),
        }
    }

    /// Projekcja stanu `TxArbiter` na stan istotny dla decyzji PTT.
    #[must_use]
    pub fn ptt_transmit_state(&self) -> crate::gui::ptt_input::PttTransmitState {
        crate::gui::ptt_input::project_tx_state(&self.cat.runtime.snapshot().tx_state)
    }

    /// Zapisuje jawny wynik dispatchera i sygnalizuje odmowę użytkownikowi.
    ///
    /// Funkcja **nigdy** nie ustawia logicznego stanu TX: `cat.ptt_active` pozostaje wyłącznie
    /// pochodną telemetrii i stanu arbitra.
    fn record_ptt_outcome(&mut self, outcome: crate::gui::ptt_input::PttDispatchOutcome) {
        self.ptt.last_outcome = Some(outcome);
        if !ptt_refusal_is_user_visible(outcome) {
            return;
        }
        let reason = ptt_refusal_reason(outcome);
        let message = Msg::new(key::PTT_REFUSED)
            .arg("reason", reason)
            .text(self.gui.appearance.language);
        self.gui
            .chrome
            .push_status_toast(crate::gui::state::ToastSeverity::Warning, message);
    }

    /// Próbuje wykonać kontrolowane zwolnienie PTT po utracie fokusu okna (F-17).
    ///
    /// Wywoływane raz na klatkę. Emituje **dokładnie jedną** intencję OFF na przejście
    /// „fokus → brak fokusu” i tylko wtedy, gdy arbiter potwierdza lokalne nadawanie.
    pub fn release_ptt_on_focus_loss(&mut self) {
        let focused = self.ptt.gates.window_focused;
        let tx_state = self.ptt_transmit_state();
        if self.ptt.focus_release.observe(focused, tx_state).is_some() {
            let _ = self.dispatch_ptt(
                crate::gui::ptt_input::PttActionSource::FocusLossRelease,
                crate::gui::ptt_input::PttIntent::RequestOff,
            );
        }
    }

    pub fn stop_rotor(&mut self) {
        self.cat.rotor_state.moving = false;
        self.spawn_rotor_command(RotorCommand::Stop, None);
    }

    pub fn turn_rotor_short_path(&mut self) {
        if self.entry.active_bearing_deg >= 0.0 {
            self.rotate_antenna_to(self.entry.active_bearing_deg as f32);
        }
    }

    pub fn turn_rotor_long_path(&mut self) {
        if self.entry.active_bearing_deg >= 0.0 {
            let lp = ((self.entry.active_bearing_deg + 180.0) % 360.0) as f32;
            self.rotate_antenna_to(lp);
        }
    }

    pub fn tune_satellite_frequencies(&mut self) {
        let freq_hz = (self.satellites.downlink_mhz * 1_000_000.0) as u64;
        if freq_hz > 0 {
            self.set_vfo_frequency(freq_hz);
        }
    }

    pub fn point_rotor_to_satellite(&mut self) {
        self.rotate_antenna_to_el(self.satellites.azimuth, self.satellites.elevation);
    }

    pub fn cat_backend_kind(&self) -> crate::cat::backend::CatBackendKind {
        crate::cat::backend::CatBackendKind::from_str(&self.config.cat_backend)
    }

    pub fn start_cat_service(&mut self) {
        if let Some(handle) = self.cat.poll_abort.take() {
            handle.abort();
        }
        if let Some(mut sup) = self.cat.rigctld_supervisor.take() {
            sup.stop();
        }

        // 1. Upewnij się, że stacja CAT jest zainicjalizowana i zsynchronizowana
        if self.config.cat_station.is_none() {
            let mut temp_cfg = self.build_app_config();
            let _ = crate::cat::config::migrate_legacy_cat_config(&mut temp_cfg);
            self.config.cat_station = temp_cfg.cat_station;
        }

        let station_cfg = if let Some(cfg) = self.config.cat_station.clone() {
            cfg
        } else {
            let default_cfg = crate::cat::config::CatStationConfig::default();
            self.config.cat_station = Some(default_cfg.clone());
            default_cfg
        };

        // 2. Jeśli radio to Hamlib serial i ma włączony auto-start rigctld (legacy lub w profilu stacji):
        let should_auto_start_rigctld = self.config.cat_auto_start_rigctld
            || station_cfg.rigs.iter().any(|r| {
                if let crate::cat::config::RigBackendConfig::Hamlib(ref h) = r.backend {
                    h.auto_start_rigctld
                } else {
                    false
                }
            });

        if should_auto_start_rigctld {
            let (port, rig_id, serial_port, baud_rate, hamlib_source, custom_path) = station_cfg
                .rigs
                .iter()
                .find_map(|r| {
                    if let crate::cat::config::RigBackendConfig::Hamlib(ref h) = r.backend
                        && h.auto_start_rigctld
                    {
                        let model_num =
                            h.rig_model.parse::<u32>().unwrap_or(self.config.cat_rig_id);
                        return Some((
                            h.port,
                            model_num,
                            h.serial_port.clone(),
                            h.baud_rate,
                            h.hamlib_source.clone(),
                            h.custom_rigctld_path.clone(),
                        ));
                    }
                    None
                })
                .unwrap_or_else(|| {
                    (
                        self.config.cat_port,
                        self.config.cat_rig_id,
                        self.config.cat_serial_port.clone(),
                        self.config.cat_baud_rate,
                        self.config.cat_hamlib_source.clone(),
                        self.config.cat_custom_rigctld_path.clone(),
                    )
                });

            let mut sup = crate::cat::supervisor::RigctldSupervisor::new(
                port,
                rig_id,
                &serial_port,
                baud_rate,
                &hamlib_source,
                &custom_path,
            );
            match sup.start() {
                Ok(()) => {
                    self.cat.rigctld_supervisor = Some(sup);
                    self.cat.test_result = Some(format!(
                        "Uruchomiono rigctld w tle dla {}!",
                        self.config.cat_rig_model
                    ));
                }
                Err(e) => {
                    self.cat.test_result = Some(
                        Msg::new(key::CAT_RIGCTLD_ERROR)
                            .arg("err", e.to_string())
                            .text(self.gui.appearance.language),
                    );
                }
            }
        }

        // 3. Asynchroniczne uruchomienie CatRuntime przez warstwę Tokio
        let mut old_rt = self.cat.runtime.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.cat.start_rx = Some(rx);

        tokio::spawn(async move {
            let _ = old_rt
                .shutdown(std::time::Duration::from_millis(1500))
                .await;
            let res = crate::cat::runtime::CatRuntime::start(&station_cfg).await;
            let _ = tx.send(res);
        });

        self.cat.test_result = Some(
            tr(
                key::CAT_RUNTIME_INIT_IN_PROGRESS,
                self.gui.appearance.language,
            )
            .to_string(),
        );
    }

    pub fn stop_cat_service(&mut self) {
        if let Some(handle) = self.cat.poll_abort.take() {
            handle.abort();
        }
        self.cat.start_rx = None;
        let mut rt = self.cat.runtime.clone();
        tokio::spawn(async move {
            let _ = rt.shutdown(std::time::Duration::from_millis(1500)).await;
        });
        self.cat.connected = false;
        self.cat.rig_state.connected = false;
        if let Some(mut sup) = self.cat.rigctld_supervisor.take() {
            sup.stop();
        }
        self.cat.test_result =
            Some(tr(key::CAT_STOPPED_RIGCTLD, self.gui.appearance.language).to_string());
    }

    pub fn set_vfo_frequency(&mut self, freq_hz: u64) {
        self.cat.rig_state.frequency_hz = freq_hz;
        if let Some(band_def) = crate::core::bandplan::get_band_by_freq(freq_hz) {
            self.entry.band = band_def.name.to_string();
        }
        self.cat.runtime.set_frequency(freq_hz);
        self.gui.chrome.status_message = Some(format!(
            "VFO dostrojone do: {:.3} kHz",
            (freq_hz as f64) / 1000.0
        ));
    }

    pub fn step_vfo(&mut self, step_hz: i64) {
        let current = self.cat.rig_state.frequency_hz as i64;
        let next = (current + step_hz).max(100_000) as u64;
        self.set_vfo_frequency(next);
    }

    pub fn set_vfo_mode(&mut self, mode: &str) {
        self.cat.rig_state.mode = mode.to_string();
        self.entry.mode = mode.to_string();
        self.cat
            .runtime
            .set_mode(mode.to_string(), self.cat.rig_state.passband_hz);
        self.gui.chrome.status_message = Some(format!("Emisja zmieniona na: {mode}"));
    }

    pub fn set_vfo_split(&mut self, enabled: bool) {
        self.cat.vfo_split = enabled;
        self.cat.rig_state.split_enabled = enabled;
        self.cat.runtime.set_split(enabled, "VFOB".to_string());
        let split_status = if enabled {
            tr(key::CAT_SPLIT_ENABLED_VFO_B, self.gui.appearance.language)
        } else {
            tr(key::CAT_SPLIT_DISABLED, self.gui.appearance.language)
        };
        self.gui.chrome.status_message = Some(
            Msg::new(key::CAT_SPLIT_MODE_MSG)
                .arg("status", split_status)
                .text(self.gui.appearance.language),
        );
    }

    pub fn swap_vfo_ab(&mut self) {
        let next_vfo = if self.cat.rig_state.vfo == "VFOA" {
            "VFOB"
        } else {
            "VFOA"
        };
        self.cat.rig_state.vfo = next_vfo.to_string();
        self.cat.runtime.set_vfo(next_vfo.to_string());
        self.gui.chrome.status_message = Some(
            Msg::new(key::CAT_VFO_SWITCHED)
                .arg("vfo", next_vfo)
                .text(self.gui.appearance.language),
        );
    }

    pub fn set_vfo_filter(&mut self, preset: &str, passband_hz: u32) {
        self.cat.vfo.filter_preset = preset.to_string();
        self.cat.rig_state.passband_hz = passband_hz;
        self.cat
            .runtime
            .set_mode(self.cat.rig_state.mode.clone(), passband_hz);
        self.gui.chrome.status_message = Some(format!("Filtr IF: {preset} ({passband_hz} Hz)"));
    }

    /// Aktualizuje bieżący stan radia i PTT na podstawie telemetrii z warstwy CAT.
    pub fn update_cat_state_from_telemetry(
        &mut self,
        st: crate::cat::hamlib::RigState,
        is_tx: bool,
    ) {
        let s_meter_diff = (st.s_meter_dbm - self.cat.rig_state.s_meter_dbm).abs();
        let s_meter_significant =
            s_meter_diff > 3.0 || st.s_meter_unit != self.cat.rig_state.s_meter_unit;
        let pwr_diff = (st.rf_power_watts - self.cat.rig_state.rf_power_watts).abs();
        let pwr_significant = pwr_diff > 1.0;

        let changed = st.frequency_hz != self.cat.rig_state.frequency_hz
            || st.mode != self.cat.rig_state.mode
            || st.connected != self.cat.connected
            || is_tx != self.cat.ptt_active
            || st.vfo != self.cat.rig_state.vfo
            || st.split_enabled != self.cat.rig_state.split_enabled
            || s_meter_significant
            || pwr_significant;

        self.cat.connected = st.connected;
        self.cat.ptt_active = is_tx;

        if changed {
            self.runtime.request_repaint();
            self.services
                .event_bus
                .publish(crate::core::events::AppEvent::RigState {
                    frequency_hz: st.frequency_hz,
                    mode: st.mode.clone(),
                    connected: st.connected,
                });
            if st.frequency_hz > 0 {
                if let Some(band_def) = crate::core::bandplan::get_band_by_freq(st.frequency_hz) {
                    self.entry.band = band_def.name.to_string();
                }
                if !st.mode.is_empty() {
                    self.entry.mode.clone_from(&st.mode);
                }
            }
            self.services.plugin_engine.run_on_rig_state(
                st.frequency_hz as f64 / 1_000_000.0,
                &st.mode,
                st.connected,
            );
        }
        self.cat.rig_state = st;
    }

    pub fn toggle_cat_proxy_server(&mut self) {
        if self.config.cat_sharing_enabled {
            if let Some(srv) = self.cat.proxy_server.take() {
                srv.stop();
            }
            let snap = self.cat.runtime.snapshot();
            let active_rig = snap.active_rx.as_ref().map_or_else(
                || {
                    crate::cat::routing::RigId::new("rig-1")
                        .unwrap_or_else(|_| crate::cat::routing::RigId::hub())
                },
                |t| t.rig_id.clone(),
            );
            let (mut srv, rx) = crate::cat::server::HamlibProxyServer::new(
                self.config.cat_sharing_port,
                self.cat.shared_state.clone(),
            );
            srv = srv.with_runtime(self.cat.runtime.clone(), active_rig);
            let srv_arc = std::sync::Arc::new(srv);
            self.cat.proxy_server = Some(srv_arc.clone());
            self.cat.proxy_rx = Some(rx);
            self.cat.sharing_active = true;

            let events = self.services.event_bus.clone();
            tokio::spawn(async move {
                if let Err(e) = srv_arc.run().await {
                    log::error!("Hamlib proxy server error: {e}");
                    events.publish(crate::core::events::AppEvent::Toast {
                        level: "error".into(),
                        message: format!("Serwer proxy CAT (Hamlib) zatrzymany: {e}"),
                    });
                }
            });
        } else {
            if let Some(srv) = self.cat.proxy_server.take() {
                srv.stop();
            }
            self.cat.proxy_rx = None;
            self.cat.sharing_active = false;
        }
    }
}

/// Produkcyjny kanał nadawczy GUI → `TxArbiter` (F-17).
///
/// Implementacja wywołuje wyłącznie `CatRuntime::request_tx` / `CatRuntime::release_tx`, czyli
/// pełną, istniejącą ścieżkę arbitrażu nadawania (`RigHub` → `TxArbiter` → backend CAT).
/// Nie konstruuje `RigCommand::SetPtt`, nie omija arbitra i nie ponawia żądań.
struct CatTransmitPort {
    runtime: crate::cat::runtime::CatRuntime,
    rig_id: Option<crate::cat::routing::RigId>,
    shutting_down: bool,
}

impl CatTransmitPort {
    /// Buduje kanał nadawczy z bieżącego stanu CAT (bez otwierania żadnego transportu).
    fn from_runtime(runtime: &crate::cat::runtime::CatRuntime) -> Self {
        let snapshot = runtime.snapshot();
        let rig_id = snapshot
            .active_tx
            .as_ref()
            .map(|target| target.rig_id.clone())
            .or_else(|| {
                snapshot
                    .active_rx
                    .as_ref()
                    .map(|target| target.rig_id.clone())
            });
        Self {
            runtime: runtime.clone(),
            rig_id,
            shutting_down: snapshot.shutting_down,
        }
    }

    /// Wspólna walidacja wstępna: aktywne radio i działający kanał wykonawczy.
    fn ensure_ready(
        &self,
    ) -> Result<crate::cat::routing::RigId, crate::gui::ptt_input::PttPortError> {
        if self.shutting_down {
            return Err(crate::gui::ptt_input::PttPortError::RequestChannelClosed);
        }
        self.rig_id
            .clone()
            .ok_or(crate::gui::ptt_input::PttPortError::NoActiveRig)
    }
}

impl crate::gui::ptt_input::PttTransmitPort for CatTransmitPort {
    fn request_on(&mut self) -> Result<(), crate::gui::ptt_input::PttPortError> {
        let rig_id = self.ensure_ready()?;
        self.runtime
            .request_tx(rig_id, std::time::Duration::from_millis(1500));
        Ok(())
    }

    fn request_off(&mut self) -> Result<(), crate::gui::ptt_input::PttPortError> {
        let rig_id = self.ensure_ready()?;
        self.runtime
            .release_tx(rig_id, std::time::Duration::from_millis(1500));
        Ok(())
    }
}

/// Czy odmowa PTT powinna być pokazana operatorowi jako komunikat statusu.
fn ptt_refusal_is_user_visible(outcome: crate::gui::ptt_input::PttDispatchOutcome) -> bool {
    use crate::gui::ptt_input::PttDispatchOutcome as Outcome;
    matches!(
        outcome,
        Outcome::ShortcutMismatch
            | Outcome::AuthorizationDenied(_)
            | Outcome::RefusedBusy
            | Outcome::RefusedUnknownTxState
            | Outcome::Failed(_)
            | Outcome::IgnoredTextEditing
            | Outcome::IgnoredImeActive
            | Outcome::IgnoredWindowUnfocused
            | Outcome::IgnoredModalOpen
    )
}

/// Czytelny, stabilny powód odmowy PTT (bez sekretów i bez danych uwierzytelniających).
fn ptt_refusal_reason(outcome: crate::gui::ptt_input::PttDispatchOutcome) -> String {
    use crate::gui::ptt_input::PttDispatchOutcome as Outcome;
    match outcome {
        Outcome::AuthorizationDenied(authorization) => {
            format!("{} ({})", outcome.code(), authorization.as_str())
        }
        Outcome::Failed(error) => format!("{} ({})", outcome.code(), error.as_str()),
        other => other.code().to_string(),
    }
}
