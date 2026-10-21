use super::SpLogApp;
use crate::core::i18n::{Msg, key, tr};

impl SpLogApp {
    /// Obraca antenę na podany azymut przez protokół rotctld (Hamlib)
    pub fn rotate_antenna_to(&mut self, azimuth_deg: f32) {
        self.rotor_state.azimuth_deg = azimuth_deg;
        let host = self.rotor_host.clone();
        let port = self.rotor_port;
        tokio::spawn(async move {
            let _ =
                crate::cat::rotor::RotorClient::set_position(&host, port, azimuth_deg, 0.0).await;
        });
        self.gui.chrome.status_toast = Some((
            Msg::new(key::ROTOR_CMD_SENT_AZ)
                .arg("az", format!("{azimuth_deg:.0}"))
                .arg("host", &self.rotor_host)
                .arg("port", self.rotor_port)
                .text(self.gui.appearance.language),
            std::time::Instant::now(),
        ));
    }

    /// Obraca antenę na zadany azymut i elewację (akcja pluginu `rotate`).
    pub fn rotate_antenna_to_el(&mut self, azimuth_deg: f32, elevation_deg: f32) {
        self.rotor_state.azimuth_deg = azimuth_deg;
        self.rotor_state.elevation_deg = elevation_deg;
        let host = self.rotor_host.clone();
        let port = self.rotor_port;
        tokio::spawn(async move {
            let _ = crate::cat::rotor::RotorClient::set_position(
                &host,
                port,
                azimuth_deg,
                elevation_deg,
            )
            .await;
        });
        self.gui.chrome.status_toast = Some((
            Msg::new(key::ROTOR_CMD_SENT_AZ_EL)
                .arg("az", format!("{azimuth_deg:.0}"))
                .arg("el", format!("{elevation_deg:.0}"))
                .text(self.gui.appearance.language),
            std::time::Instant::now(),
        ));
    }

    /// Przełącza stan nadawania PTT (TX/RX) z pełną weryfikacją w TxArbiter.
    pub fn toggle_ptt(&mut self) {
        let snap = self.cat_runtime.snapshot();
        let tx_rig = snap
            .active_tx
            .as_ref()
            .map(|t| t.rig_id.clone())
            .or_else(|| snap.active_rx.as_ref().map(|r| r.rig_id.clone()));

        if let Some(rig_id) = tx_rig {
            let is_transmitting = match snap.tx_state {
                crate::cat::tx_arbiter::TxArbiterState::Owned {
                    rig_id: ref owner, ..
                } => owner == &rig_id,
                _ => false,
            };
            if is_transmitting {
                self.cat_runtime
                    .release_tx(rig_id, std::time::Duration::from_millis(1500));
            } else {
                self.cat_runtime
                    .request_tx(rig_id, std::time::Duration::from_millis(1500));
            }
        }
    }

    pub fn stop_rotor(&mut self) {
        self.rotor_state.moving = false;
        let host = self.rotor_host.clone();
        let port = self.rotor_port;
        tokio::spawn(async move {
            let _ = crate::cat::rotor::RotorClient::stop(&host, port).await;
        });
    }

    pub fn turn_rotor_short_path(&mut self) {
        if self.active_bearing_deg >= 0.0 {
            self.rotate_antenna_to(self.active_bearing_deg as f32);
        }
    }

    pub fn turn_rotor_long_path(&mut self) {
        if self.active_bearing_deg >= 0.0 {
            let lp = ((self.active_bearing_deg + 180.0) % 360.0) as f32;
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
        crate::cat::backend::CatBackendKind::from_str(&self.cat_backend)
    }

    pub fn start_cat_service(&mut self) {
        if let Some(handle) = self.cat_poll_abort.take() {
            handle.abort();
        }
        if let Some(mut sup) = self.rigctld_supervisor.take() {
            sup.stop();
        }

        // 1. Upewnij się, że stacja CAT jest zainicjalizowana i zsynchronizowana
        if self.cat_station.is_none() {
            let mut temp_cfg = self.build_app_config();
            let _ = crate::cat::config::migrate_legacy_cat_config(&mut temp_cfg);
            self.cat_station = temp_cfg.cat_station;
        }

        let station_cfg = match self.cat_station.clone() {
            Some(cfg) => cfg,
            None => {
                let default_cfg = crate::cat::config::CatStationConfig::default();
                self.cat_station = Some(default_cfg.clone());
                default_cfg
            }
        };

        // 2. Jeśli radio to Hamlib serial i ma włączony auto-start rigctld:
        if self.cat_auto_start_rigctld {
            let mut sup = crate::cat::supervisor::RigctldSupervisor::new(
                self.cat_port,
                self.cat_rig_id,
                &self.cat_serial_port,
                self.cat_baud_rate,
                &self.cat_hamlib_source,
                &self.cat_custom_rigctld_path,
            );
            match sup.start() {
                Ok(()) => {
                    self.rigctld_supervisor = Some(sup);
                    self.cat_test_result = Some(format!(
                        "Uruchomiono rigctld w tle dla {}!",
                        self.cat_rig_model
                    ));
                }
                Err(e) => {
                    self.cat_test_result = Some(
                        Msg::new(key::CAT_RIGCTLD_ERROR)
                            .arg("err", &e.to_string())
                            .text(self.gui.appearance.language),
                    );
                }
            }
        }

        // 3. Asynchroniczne uruchomienie CatRuntime przez warstwę Tokio
        let mut old_rt = self.cat_runtime.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.cat_start_rx = Some(rx);

        tokio::spawn(async move {
            let _ = old_rt
                .shutdown(std::time::Duration::from_millis(1500))
                .await;
            let res = crate::cat::runtime::CatRuntime::start(&station_cfg).await;
            let _ = tx.send(res);
        });

        self.cat_test_result = Some(
            tr(
                key::CAT_RUNTIME_INIT_IN_PROGRESS,
                self.gui.appearance.language,
            )
            .to_string(),
        );
    }

    pub fn stop_cat_service(&mut self) {
        if let Some(handle) = self.cat_poll_abort.take() {
            handle.abort();
        }
        self.cat_start_rx = None;
        let mut rt = self.cat_runtime.clone();
        tokio::spawn(async move {
            let _ = rt.shutdown(std::time::Duration::from_millis(1500)).await;
        });
        self.cat_connected = false;
        self.rig_state.connected = false;
        if let Some(mut sup) = self.rigctld_supervisor.take() {
            sup.stop();
        }
        self.cat_test_result =
            Some(tr(key::CAT_STOPPED_RIGCTLD, self.gui.appearance.language).to_string());
    }

    pub fn set_vfo_frequency(&mut self, freq_hz: u64) {
        self.rig_state.frequency_hz = freq_hz;
        if let Some(band_def) = crate::core::bandplan::get_band_by_freq(freq_hz) {
            self.entry_band = band_def.name.to_string();
        }
        self.cat_runtime.set_frequency(freq_hz);
        self.gui.chrome.status_message = Some(format!(
            "VFO dostrojone do: {:.3} kHz",
            (freq_hz as f64) / 1000.0
        ));
    }

    pub fn step_vfo(&mut self, step_hz: i64) {
        let current = self.rig_state.frequency_hz as i64;
        let next = (current + step_hz).max(100_000) as u64;
        self.set_vfo_frequency(next);
    }

    pub fn set_vfo_mode(&mut self, mode: &str) {
        self.rig_state.mode = mode.to_string();
        self.entry_mode = mode.to_string();
        self.cat_runtime
            .set_mode(mode.to_string(), self.rig_state.passband_hz);
        self.gui.chrome.status_message = Some(format!("Emisja zmieniona na: {mode}"));
    }

    pub fn set_vfo_split(&mut self, enabled: bool) {
        self.vfo_split = enabled;
        self.rig_state.split_enabled = enabled;
        self.cat_runtime.set_split(enabled, "VFOB".to_string());
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
        let next_vfo = if self.rig_state.vfo == "VFOA" {
            "VFOB"
        } else {
            "VFOA"
        };
        self.rig_state.vfo = next_vfo.to_string();
        self.cat_runtime.set_vfo(next_vfo.to_string());
        self.gui.chrome.status_message = Some(
            Msg::new(key::CAT_VFO_SWITCHED)
                .arg("vfo", next_vfo)
                .text(self.gui.appearance.language),
        );
    }

    pub fn set_vfo_filter(&mut self, preset: &str, passband_hz: u32) {
        self.vfo_filter_preset = preset.to_string();
        self.rig_state.passband_hz = passband_hz;
        self.cat_runtime
            .set_mode(self.rig_state.mode.clone(), passband_hz);
        self.gui.chrome.status_message = Some(format!("Filtr IF: {preset} ({passband_hz} Hz)"));
    }

    /// Aktualizuje bieżący stan radia i PTT na podstawie telemetrii z warstwy CAT.
    pub fn update_cat_state_from_telemetry(
        &mut self,
        st: crate::cat::hamlib::RigState,
        is_tx: bool,
    ) {
        let changed = st.frequency_hz != self.rig_state.frequency_hz
            || st.mode != self.rig_state.mode
            || st.connected != self.cat_connected
            || is_tx != self.ptt_active;

        self.cat_connected = st.connected;
        self.rig_state = st.clone();
        self.ptt_active = is_tx;

        if changed {
            self.services
                .event_bus
                .publish(crate::core::events::AppEvent::RigState {
                    frequency_hz: st.frequency_hz,
                    mode: st.mode.clone(),
                    connected: st.connected,
                });
            if st.frequency_hz > 0 {
                if let Some(band_def) = crate::core::bandplan::get_band_by_freq(st.frequency_hz) {
                    self.entry_band = band_def.name.to_string();
                }
                if !st.mode.is_empty() {
                    self.entry_mode = st.mode.clone();
                }
            }
            self.services.plugin_engine.run_on_rig_state(
                st.frequency_hz as f64 / 1_000_000.0,
                &st.mode,
                st.connected,
            );
        }
    }

    pub fn toggle_cat_proxy_server(&mut self) {
        if self.cat_sharing_enabled {
            if let Some(srv) = self.cat_proxy_server.take() {
                srv.stop();
            }
            let (srv, rx) = crate::cat::server::HamlibProxyServer::new(
                self.cat_sharing_port,
                self.cat_shared_state.clone(),
            );
            let srv_arc = std::sync::Arc::new(srv);
            self.cat_proxy_server = Some(srv_arc.clone());
            self.cat_proxy_rx = Some(rx);
            self.cat_sharing_active = true;

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
            if let Some(srv) = self.cat_proxy_server.take() {
                srv.stop();
            }
            self.cat_proxy_rx = None;
            self.cat_sharing_active = false;
        }
    }
}
