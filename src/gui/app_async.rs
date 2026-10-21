// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::*;
use crate::cluster::telnet::ClusterEvent;
use crate::core::i18n::{Msg, key, tr};
use eframe::egui;

impl SpLogApp {
    /// Wczesna faza asynchroniczna: wtyczki, WSPR, synchronizacja API, upload scheduler, satelity i solar.
    pub fn poll_early_async_events(&mut self) {
        // Plugin Rhai: odśwież migawkę stanu, wykonaj polecenia i odbierz wyniki zapytań
        if self.plugins_enabled {
            self.refresh_plugin_snapshot();
            self.process_plugin_commands();
            self.poll_plugin_lookup();
        }

        // Odbieranie wyników asynchronicznych operacji
        self.poll_wspr_results();
        self.sync_cluster_spots_api();
        self.poll_upload_scheduler();
        self.refresh_satellite_tracking();
        self.poll_solar_fetch();
    }

    /// WSPR: sprawdź czy pobieranie zakończyło się
    pub fn poll_wspr_results(&mut self) {
        if self.wspr.loading {
            if let Some(ref slot) = self.wspr.fetch_slot.clone() {
                if let Ok(mut guard) = slot.try_lock() {
                    if let Some(result) = guard.take() {
                        self.wspr.loading = false;
                        self.wspr.fetch_slot = None;
                        match result {
                            Ok(spots) => {
                                self.wspr.spots = spots;
                                self.wspr.last_error = None;
                            }
                            Err(e) => {
                                self.wspr.last_error = Some(e.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Synchronizuj cluster_spots z wątkiem REST API (gdy zmieni się rozmiar lub najnowszy spot)
    pub fn sync_cluster_spots_api(&mut self) {
        if let Some(ref api_slot) = self.cluster_spots_api.clone() {
            if let Ok(mut guard) = api_slot.try_lock() {
                let changed = guard.len() != self.cluster_spots.len()
                    || guard
                        .first()
                        .map(|s| (&s.dx_call, &s.time_utc, s.received_at))
                        != self
                            .cluster_spots
                            .first()
                            .map(|s| (&s.dx_call, &s.time_utc, s.received_at));
                if changed {
                    (*guard).clone_from(&self.cluster_spots);
                }
            }
        }
    }

    /// Główna faza asynchroniczna: aktualizacje, CAT, REST API, proxy CAT, klaster, callbook, sync logi, WSJT-X, JS8Call, Multi-Op
    pub fn poll_main_async_events(&mut self, ctx: &egui::Context) {
        self.poll_update_outcomes(ctx);
        self.poll_cat_outcomes();
        self.sync_cat_shared_state_and_api();
        self.poll_cat_proxy_commands();
        self.poll_cluster_events();
        self.poll_qrz_lookup();
        self.poll_online_sync_logs();
        self.poll_wsjtx();
        self.poll_js8call();
        self.poll_sync_events();
    }

    /// Wynik sprawdzania i instalowania aktualizacji
    pub fn poll_update_outcomes(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.updates.update_check_rx {
            if let Ok(outcome) = rx.try_recv() {
                match outcome {
                    crate::cloud::updater::UpdateCheckOutcome::UpToDate { local } => {
                        self.updates.update_check_status = Some(
                            Msg::new(key::UPDATER_LATEST_VERSION)
                                .arg("local", local)
                                .text(self.current_language),
                        );
                    }
                    crate::cloud::updater::UpdateCheckOutcome::NewVersion(release) => {
                        self.updates.update_check_status = Some(
                            Msg::new(key::UPDATER_NEW_VERSION_AVAILABLE)
                                .arg("new", &release.tag)
                                .arg("local", env!("CARGO_PKG_VERSION"))
                                .text(self.current_language),
                        );
                        self.updates.update_available = Some(release);
                    }
                    crate::cloud::updater::UpdateCheckOutcome::Error(e) => {
                        self.updates.update_check_status = Some(
                            Msg::new(key::UPDATER_CHECK_ERROR)
                                .arg("err", &e.to_string())
                                .text(self.current_language),
                        );
                    }
                }
                self.updates.update_check_rx = None;
            }
        }

        if let Some(rx) = &self.updates.update_install_rx {
            if let Ok(msg) = rx.try_recv() {
                let installed_ok = msg.starts_with("✅");
                self.updates.update_install_status = Some(msg);
                self.updates.update_install_rx = None;
                if installed_ok {
                    // Skrypt podmiany czeka na zakończenie tego procesu — zamknij aplikację.
                    self.updates.pending_restart = true;
                }
            }
        }

        if self.updates.pending_restart {
            self.updates.update_install_status =
                Some(tr(key::UPDATER_INSTALLING_RESTART, self.current_language).to_string());
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Odbiór asynchronicznego wyniku uruchomienia CatRuntime, testu probe oraz telemetrii
    pub fn poll_cat_outcomes(&mut self) {
        if let Some(ref rx) = self.cat_start_rx {
            if let Ok(res) = rx.try_recv() {
                match res {
                    Ok(rt) => {
                        self.cat_runtime = rt;
                        self.cat_connected = true;
                        self.cat_test_result =
                            Some(tr(key::CAT_STARTED_SUCCESS, self.current_language).to_string());
                    }
                    Err(e) => {
                        self.cat_connected = false;
                        self.cat_test_result = Some(
                            Msg::new(key::CAT_START_ERROR)
                                .arg("err", &e.to_string())
                                .text(self.current_language),
                        );
                    }
                }
            }
        }

        if let Some(ref rx) = self.cat_probe_rx {
            if let Ok((probe_gen, res)) = rx.try_recv() {
                if probe_gen == self.cat_probe_generation {
                    self.cat_probe_running = false;
                    match &res {
                        Ok(r) => {
                            self.cat_probe_state = crate::cat::probe::ProbeState::Succeeded {
                                generation: probe_gen,
                                result: r.clone(),
                            };
                        }
                        Err(e) => {
                            self.cat_probe_state = crate::cat::probe::ProbeState::Failed {
                                generation: probe_gen,
                                error: e.to_string(),
                            };
                        }
                    }
                    self.cat_probe_result = Some(res);
                    self.cat_probe_task = None;
                }
            }
        }

        let cat_snap = self.cat_runtime.snapshot();
        let mut active_state_found = false;

        if let Some(ref rx_target) = cat_snap.active_rx {
            if let Some(svc) = cat_snap.rigs.get(&rx_target.rig_id) {
                active_state_found = true;
                let is_conn = matches!(
                    svc.lifecycle,
                    crate::cat::service::RigServiceState::Ready
                        | crate::cat::service::RigServiceState::Degraded
                );
                let mut st = svc.state.clone().unwrap_or_default();
                st.connected = is_conn;

                let is_tx = match cat_snap.tx_state {
                    crate::cat::tx_arbiter::TxArbiterState::Owned { ref rig_id, .. } => {
                        rig_id == &rx_target.rig_id
                    }
                    _ => false,
                };
                self.update_cat_state_from_telemetry(st, is_tx);
            }
        }

        if !active_state_found {
            if let Some((rig_id, svc)) = cat_snap.rigs.iter().next() {
                let is_conn = matches!(
                    svc.lifecycle,
                    crate::cat::service::RigServiceState::Ready
                        | crate::cat::service::RigServiceState::Degraded
                );
                let mut st = svc.state.clone().unwrap_or_default();
                st.connected = is_conn;

                let is_tx = match cat_snap.tx_state {
                    crate::cat::tx_arbiter::TxArbiterState::Owned {
                        rig_id: ref owner, ..
                    } => owner == rig_id,
                    _ => false,
                };
                self.update_cat_state_from_telemetry(st, is_tx);
            }
        }

        while let Ok(st) = self.cat_state_rx.try_recv() {
            let ptt = self.ptt_active;
            self.update_cat_state_from_telemetry(st, ptt);
        }
    }

    /// Synchronizuj stan radia do współdzielonego stanu serwera proxy oraz REST API
    pub fn sync_cat_shared_state_and_api(&mut self) {
        if self.cat_sharing_enabled || self.rest_api_enabled {
            if let Ok(mut shared) = self.cat_shared_state.write() {
                *shared = self.rig_state.clone();
            }
        }

        if self
            .api_reload_flag
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            self.rebuild_awards_full();
            self.reload_qsos();
        }
    }

    /// Odbiór poleceń z serwera Hamlib proxy (wzorzec zbierz, potem zastosuj)
    pub fn poll_cat_proxy_commands(&mut self) {
        let mut proxy_cmds = Vec::new();
        if let Some(ref mut rx) = self.cat_proxy_rx {
            while let Ok(cmd) = rx.try_recv() {
                proxy_cmds.push(cmd);
            }
        }
        for cmd in proxy_cmds {
            match cmd {
                crate::cat::server::RigServerCommand::SetFrequency(freq) => {
                    self.set_vfo_frequency(freq);
                }
                crate::cat::server::RigServerCommand::SetSplitFrequency(freq) => {
                    self.rig_state.tx_frequency_hz = Some(freq);
                    self.cat_runtime
                        .execute_cmd(crate::cat::rig::RigCommand::SetSplitFrequency(freq));
                }
                crate::cat::server::RigServerCommand::SetMode(mode) => {
                    self.set_vfo_mode(&mode);
                }
                crate::cat::server::RigServerCommand::SetPtt(ptt) => {
                    let snap = self.cat_runtime.snapshot();
                    let tx_rig = snap
                        .active_tx
                        .as_ref()
                        .map(|t| t.rig_id.clone())
                        .or_else(|| snap.active_rx.as_ref().map(|r| r.rig_id.clone()));
                    if let Some(rig_id) = tx_rig {
                        if ptt {
                            self.cat_runtime
                                .request_tx(rig_id, std::time::Duration::from_millis(1500));
                        } else {
                            self.cat_runtime
                                .release_tx(rig_id, std::time::Duration::from_millis(1500));
                        }
                    }
                }
                crate::cat::server::RigServerCommand::SetVfo(vfo) => {
                    self.rig_state.vfo.clone_from(&vfo);
                    self.cat_runtime.set_vfo(vfo);
                }
                crate::cat::server::RigServerCommand::SetSplit { enabled, tx_vfo } => {
                    self.vfo_split = enabled;
                    self.rig_state.split_enabled = enabled;
                    self.cat_runtime.set_split(enabled, tx_vfo);
                }
                crate::cat::server::RigServerCommand::SetRit(rit) => {
                    self.rig_state.rit_hz = rit;
                    self.cat_runtime
                        .execute_cmd(crate::cat::rig::RigCommand::SetRit(rit));
                }
                crate::cat::server::RigServerCommand::SetXit(xit) => {
                    self.rig_state.xit_hz = xit;
                    self.cat_runtime
                        .execute_cmd(crate::cat::rig::RigCommand::SetXit(xit));
                }
                crate::cat::server::RigServerCommand::SetPower(watts) => {
                    self.rig_state.rf_power_watts = watts;
                    self.cat_runtime
                        .execute_cmd(crate::cat::rig::RigCommand::SetPower(watts));
                }
                crate::cat::server::RigServerCommand::SetPassband(pb) => {
                    self.rig_state.passband_hz = pb;
                    self.cat_runtime
                        .execute_cmd(crate::cat::rig::RigCommand::SetPassband(pb));
                }
            }
        }
    }

    /// Odbiór zdarzeń i spotów z klastra DX Telnet
    pub fn poll_cluster_events(&mut self) {
        if let Some(ref rx) = self.cluster_event_rx {
            while let Ok(evt) = rx.try_recv() {
                match evt {
                    ClusterEvent::Connected(msg) => {
                        self.cluster_connected = true;
                        self.cluster_connecting = false;
                        self.cluster_status_text.clone_from(&msg);
                        self.status_toast = Some((msg, std::time::Instant::now()));
                        self.event_bus
                            .publish(crate::core::events::AppEvent::ClusterStatus {
                                connected: true,
                            });
                    }
                    ClusterEvent::Disconnected(msg) => {
                        self.cluster_connected = false;
                        self.cluster_connecting = false;
                        self.cluster_status_text = msg;
                        self.event_bus
                            .publish(crate::core::events::AppEvent::ClusterStatus {
                                connected: false,
                            });
                    }
                    ClusterEvent::Spot(spot) => {
                        let is_duplicate = self.cluster_spots.iter().take(30).any(|s| {
                            s.dx_call == spot.dx_call
                                && s.band == spot.band
                                && (s.frequency_khz - spot.frequency_khz).abs() < 2.0
                        });
                        if !is_duplicate {
                            let is_atno =
                                if let Some(info) = self.prefix_matcher.lookup(&spot.dx_call) {
                                    let awards = self
                                        .awards_engine
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                                    let st = awards.check_status_full(
                                        &spot.dx_call,
                                        &spot.band,
                                        if spot.is_ft8 { "FT8" } else { "CW" },
                                        Some(info.dxcc),
                                        None,
                                        Some(info.cqz),
                                        None,
                                        Some(&info.continent),
                                        None,
                                    );
                                    st.is_new_dxcc
                                } else {
                                    false
                                };

                            if is_atno && self.solar.band_alert_enabled {
                                crate::media::sounds::play_new_dxcc_alert();
                                self.status_toast = Some((
                                    format!(
                                        "⭐ ATNO DXCC: {} na pasmie {} ({:.1} kHz)!",
                                        spot.dx_call, spot.band, spot.frequency_khz
                                    ),
                                    std::time::Instant::now(),
                                ));
                            }

                            self.event_bus
                                .publish(crate::core::events::AppEvent::DxSpot {
                                    spotter: spot.spotter.clone(),
                                    dx_call: spot.dx_call.clone(),
                                    frequency_khz: spot.frequency_khz,
                                    band: spot.band.clone(),
                                    comment: spot.comment.clone(),
                                    time_utc: spot.time_utc.clone(),
                                    is_ft8: spot.is_ft8,
                                    is_skimmer: spot.is_skimmer,
                                });
                            self.cluster_spots.insert(0, spot.clone());
                            // Hook pluginów Rhai na nowy spot DX.
                            self.plugin_engine.run_on_dx_spot(
                                &spot.spotter,
                                &spot.dx_call,
                                spot.frequency_khz,
                                &spot.band,
                                &spot.comment,
                                spot.is_ft8,
                            );
                        }
                    }
                    ClusterEvent::RawLine(_) => {}
                }
            }
        }

        let now_ts = chrono::Utc::now().timestamp();
        let spot_max_age_secs: i64 = 60 * 60;
        if self
            .cluster_spots
            .iter()
            .any(|s| now_ts - s.received_at > spot_max_age_secs)
        {
            self.cluster_spots
                .retain(|s| now_ts - s.received_at <= spot_max_age_secs);
        }
        if self.cluster_spots.len() > 200 {
            self.cluster_spots.truncate(200);
        }
    }

    /// Odbiór asynchronicznych danych korespondenta z Callbook / HamQTH / Callook / QRZ
    pub fn poll_qrz_lookup(&mut self) {
        while let Ok(data) = self.qrz_lookup_rx.try_recv() {
            if data.callsign == self.entry_callsign.trim().to_uppercase() {
                if let Some(n) = data.name {
                    if self.entry_name.is_empty() || self.entry_name != n {
                        self.entry_name = n;
                    }
                }
                if let Some(q) = data.qth {
                    if self.entry_qth.is_empty() || self.entry_qth != q {
                        self.entry_qth = q;
                    }
                }
                if let Some(g) = data.gridsquare {
                    if self.entry_grid.is_empty() || self.entry_grid != g {
                        self.entry_grid = g;
                        self.recalculate_distance_from_grid();
                    }
                }
                if let Some(st) = data.state {
                    if self.entry_state.is_empty() || self.entry_state != st {
                        self.entry_state = st;
                    }
                }
                if let Some(mgr) = data.qsl_manager {
                    if self.entry_qsl_manager.is_empty() || self.entry_qsl_manager != mgr {
                        self.entry_qsl_manager = mgr;
                    }
                }
                if let Some(img) = data.image_url {
                    self.photo_viewer_dialog.photo_url = Some(img);
                }
                self.status_message = Some(format!(
                    "Pobrano dane Callbook dla: {} ({})",
                    data.callsign, self.entry_name
                ));
                self.status_toast = Some((
                    format!("Pobrano dane korespondenta {}!", data.callsign),
                    std::time::Instant::now(),
                ));
            }
        }
    }

    /// Odbiór asynchronicznych komunikatów z synchronizacji online (LoTW, eQSL, Club Log, QRZ, Aktualizacje baz)
    pub fn poll_online_sync_logs(&mut self) {
        while let Ok((msg, rebuild_awards)) = self.sync_log_rx.try_recv() {
            self.online_sync_logs.push(msg.clone());
            if self.online_sync_logs.len() > 500 {
                let excess = self.online_sync_logs.len() - 500;
                self.online_sync_logs.drain(0..excess);
            }
            self.status_message = Some(msg.clone());
            self.status_toast = Some((msg, std::time::Instant::now()));
            if rebuild_awards {
                self.rebuild_awards_full();
            }
        }
    }

    /// Odbiór pakietów cyfrowych WSJT-X (UDP 2237)
    pub fn poll_wsjtx(&mut self) {
        while let Ok(msg) = self.wsjtx_rx.try_recv() {
            self.wsjtx_packets_count += 1;
            match msg {
                crate::digital::wsjtx::WsjtxMessage::Status {
                    dial_freq,
                    mode,
                    dx_call,
                    ..
                } => {
                    if !dx_call.is_empty() {
                        self.wsjtx_last_call = Some(dx_call);
                    }
                    if dial_freq > 0 {
                        self.rig_state.frequency_hz = dial_freq;
                        if let Some(band_def) = crate::core::bandplan::get_band_by_freq(dial_freq) {
                            self.entry_band = band_def.name.to_string();
                        }
                    }
                    if !mode.is_empty() {
                        self.entry_mode = mode;
                    }
                }
                crate::digital::wsjtx::WsjtxMessage::QsoLogged(qso) => {
                    self.handle_wsjtx_qso(*qso);
                }
                crate::digital::wsjtx::WsjtxMessage::Heartbeat { .. } => {}
            }
        }
    }

    /// Odbiór aktualizacji stanu i zalogowanych QSO z modułu JS8Call przez TCP API
    pub fn poll_js8call(&mut self) {
        if let Some(ref rx) = self.js8call_state_rx {
            while let Ok(state) = rx.try_recv() {
                if state.dial_freq_hz > 0 && state.dial_freq_hz != self.js8call_state.dial_freq_hz {
                    self.rig_state.frequency_hz = state.dial_freq_hz;
                    if let Some(band_def) =
                        crate::core::bandplan::get_band_by_freq(state.dial_freq_hz)
                    {
                        self.entry_band = band_def.name.to_string();
                    }
                }
                if state.connected && self.entry_mode != "JS8" {
                    self.entry_mode = "JS8".to_string();
                }
                self.js8call_state = state;
            }
        }

        let mut js8_new_qsos = Vec::new();
        if let Some(ref rx) = self.js8call_qso_rx {
            while let Ok(qso) = rx.try_recv() {
                js8_new_qsos.push(qso);
            }
        }
        for qso in js8_new_qsos {
            self.handle_js8call_qso(qso);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_app() -> SpLogApp {
        let deps = AppDeps {
            log_db: Arc::new(Mutex::new(LogDatabase::open_in_memory().unwrap())),
            prefix_matcher: Arc::new(PrefixMatcher::empty()),
            scp_engine: Arc::new(Mutex::new(ScpEngine::new())),
        };
        let init = AppInit {
            config: AppConfig::default(),
            config_file_path: std::path::PathBuf::from("target/test_cfg.json"),
            active_db_path: std::path::PathBuf::from(":memory:"),
            shutdown_policy: ShutdownPolicy::None,
        };
        SpLogApp::for_tests(init, deps)
    }

    #[test]
    fn test_poll_early_async_events_without_network() {
        let mut app = build_test_app();
        app.poll_early_async_events();
        assert!(!app.wspr.loading);
    }

    #[test]
    fn test_poll_main_async_events_without_network() {
        let mut app = build_test_app();
        let ctx = egui::Context::default();
        app.poll_main_async_events(&ctx);
        assert_eq!(app.wsjtx_packets_count, 0);
    }

    #[test]
    fn test_poll_online_sync_logs_receives_messages() {
        let mut app = build_test_app();
        let _ = app
            .sync_log_tx
            .send(("Synchronized 5 QSOs".to_string(), false));
        app.poll_online_sync_logs();
        assert_eq!(app.online_sync_logs.len(), 1);
        assert_eq!(app.online_sync_logs[0], "Synchronized 5 QSOs");
        assert_eq!(app.status_message.as_deref(), Some("Synchronized 5 QSOs"));
    }

    #[test]
    fn test_poll_wsjtx_increments_packets() {
        let mut app = build_test_app();
        let _ = app
            .wsjtx_tx
            .send(crate::digital::wsjtx::WsjtxMessage::Heartbeat {
                id: "WSJT-X".to_string(),
            });
        app.poll_wsjtx();
        assert_eq!(app.wsjtx_packets_count, 1);
    }
}
