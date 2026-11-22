// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use super::*;
use crate::cat::rig::{PanadapterSpot, RigCommand};
use crate::cat::tune::{SplitRequest, TuneRequest};
use crate::cluster::spot_info::{self, SplitHint, SpotMode};
use crate::cluster::telnet::DxSpot;
use crate::core::awards::AwardsEngine;
use crate::core::geo::{calculate_bearing_deg, calculate_distance_km, locator_to_coordinates};

impl SpLogApp {
    /// Aktualizuje podpowiedzi SCP, rozpoznanie DXCC, azymut i dystans po wpisaniu znaku
    pub fn on_callsign_changed(&mut self) {
        let clean = self.entry.callsign.trim().to_uppercase();
        if clean.is_empty() {
            self.entry.scp_suggestions.clear();
            self.entry.callsign_corrections.clear();
            self.entry.active_prefix_info = None;
            self.entry.active_polish_district = None;
            self.entry.active_award_status = None;
            self.entry.active_distance_km = 0.0;
            self.entry.active_bearing_deg = 0.0;
            self.entry.active_propagation = None;
            self.entry.active_clubs.clear();
            self.entry.past_qsos_for_active_call.clear();
            return;
        }

        // 1. Podpowiedzi SCP
        {
            let scp = self
                .services
                .scp_engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.entry.scp_suggestions = scp.search(&clean, 6);
        }

        // 1b. Lokalna korekta rozmyta (fuzzy) błędnie wpisanego/odebranego znaku
        {
            let scp = self
                .services
                .scp_engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let candidates = scp.candidates();
            self.entry.callsign_corrections =
                crate::core::callsign_correction::suggest_corrections(&clean, &candidates, 2, 4);
        }

        // 2. Kluby krótkofalarskie (SP-OTC, PGA, SKCC, CWOPS, FOC)
        self.entry.active_clubs = crate::core::clubs::ClubRegistry::check(&clean);

        // 3. Rozpoznanie DXCC, stref i estymacja propagacji
        if let Some(info) = self.services.prefix_matcher.lookup(&clean) {
            let dx_coords = crate::core::geo::Coordinates::new(info.latitude, info.longitude);
            if let Ok(my_coords) = locator_to_coordinates(&self.config.station.gridsquare) {
                self.entry.active_distance_km = calculate_distance_km(my_coords, dx_coords);
                self.entry.active_bearing_deg = calculate_bearing_deg(my_coords, dx_coords);

                let now = jiff::tz::TimeZone::UTC.to_datetime(jiff::Timestamp::now());
                let utc_hour = now.hour() as f64 + (now.minute() as f64) / 60.0;
                let day_of_year = now.day_of_year() as u32;
                let freq_mhz = crate::core::propagation::band_to_center_mhz(&self.entry.band)
                    .unwrap_or(14.175);
                self.entry.active_propagation = self.solar.indices().map(|(sfi, k_index)| {
                    crate::core::propagation::PropagationEngine::calculate(
                        my_coords,
                        dx_coords,
                        freq_mhz,
                        sfi,
                        k_index as u8,
                        utc_hour,
                        day_of_year,
                    )
                });
            }

            self.entry.active_polish_district = AwardsEngine::get_polish_district(&clean);

            let awards = self
                .services
                .awards_engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.entry.active_award_status = Some(awards.check_status_full(
                &clean,
                &self.entry.band,
                &self.entry.mode,
                Some(info.dxcc),
                self.config.station.pga_gmina.as_deref(),
                Some(info.cqz),
                if self.entry.state.trim().is_empty() {
                    None
                } else {
                    Some(self.entry.state.trim())
                },
                Some(&info.continent),
                if self.entry.iota.trim().is_empty() {
                    None
                } else {
                    Some(self.entry.iota.trim())
                },
            ));

            self.entry.active_prefix_info = Some(info);
        }

        // 3. Sprawdzenie poprzednich łączności z tą stacją w lokalnej bazie SQLite
        let prevs_opt = if let Ok(db) = self.services.log_db.lock() {
            db.find_previous_qsos_for_journal(&self.active_journal.id, &clean)
                .ok()
        } else {
            None
        };

        if let Some(prevs) = prevs_opt {
            if let Some(last) = prevs.first() {
                if self.entry.name.is_empty()
                    && let Some(ref n) = last.name
                {
                    self.entry.name = n.clone();
                }
                if self.entry.qth.is_empty()
                    && let Some(ref q) = last.qth
                {
                    self.entry.qth = q.clone();
                }
                if self.entry.grid.is_empty()
                    && let Some(ref g) = last.gridsquare
                {
                    self.entry.grid = g.clone();
                    self.recalculate_distance_from_grid();
                }
                if self.entry.pga.is_empty()
                    && let Some(ref p) = last.pga_ref
                {
                    self.entry.pga = p.clone();
                }
            }
            self.entry.past_qsos_for_active_call = prevs;
        }

        // 4. Baza managerów QSL (tabele użytkownika)
        let mgr_rec = crate::core::reference::loader::user_tables()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .find_manager(&clean);
        if let Some(mgr_rec) = mgr_rec {
            if self.entry.qsl_manager.is_empty() {
                self.entry.qsl_manager.clone_from(&mgr_rec.manager);
            }
            self.entry.active_qsl_manager_record = Some(mgr_rec);
        } else {
            self.entry.active_qsl_manager_record = None;
        }

        // 5. Baza Callbook (tabele użytkownika)
        if clean.len() >= 3
            && let Some(cb_data) = self.services.local_callbook.lookup(&clean)
        {
            if let Some(n) = cb_data.name
                && self.entry.name.is_empty()
            {
                self.entry.name = n;
            }
            if let Some(q) = cb_data.qth
                && self.entry.qth.is_empty()
            {
                self.entry.qth = q;
            }
            if let Some(g) = cb_data.gridsquare
                && self.entry.grid.is_empty()
            {
                self.entry.grid = g;
                self.recalculate_distance_from_grid();
            }
            if let Some(s) = cb_data.state
                && self.entry.state.is_empty()
            {
                self.entry.state = s;
            }
            if let Some(mgr) = cb_data.qsl_manager
                && self.entry.qsl_manager.is_empty()
            {
                self.entry.qsl_manager = mgr;
            }
        }

        // 7. Jeśli włączono automatyczne pobieranie z serwisów online (HamQTH / Callook / QRZ)
        //    i znak ma min. 3 znaki — opóźnij o 600 ms (debounce), aby nie spamować zapytań przy szybkim pisaniu.
        if self.config.qrz_auto_lookup && clean.len() >= 3 {
            self.entry.lookup_debounce_until =
                Some(std::time::Instant::now() + std::time::Duration::from_millis(600));
        }
    }

    /// Wywołuje automatyczny lookup online po upływie debounce'a (wołane co klatkę w update()).
    pub fn process_debounced_lookup(&mut self) {
        if let Some(deadline) = self.entry.lookup_debounce_until
            && std::time::Instant::now() >= deadline
        {
            self.entry.lookup_debounce_until = None;
            let clean = self.entry.callsign.trim().to_uppercase();
            if self.config.qrz_auto_lookup && clean.len() >= 3 {
                self.lookup_active_callsign_online();
            }
        }
    }

    pub fn recalculate_distance_from_grid(&mut self) {
        if self.entry.grid.len() >= 4
            && let (Ok(p1), Ok(p2)) = (
                locator_to_coordinates(&self.config.station.gridsquare),
                locator_to_coordinates(&self.entry.grid),
            )
        {
            self.entry.active_distance_km = calculate_distance_km(p1, p2);
            self.entry.active_bearing_deg = calculate_bearing_deg(p1, p2);

            let now = jiff::tz::TimeZone::UTC.to_datetime(jiff::Timestamp::now());
            let utc_hour = now.hour() as f64 + (now.minute() as f64) / 60.0;
            let day_of_year = now.day_of_year() as u32;
            let freq_mhz =
                crate::core::propagation::band_to_center_mhz(&self.entry.band).unwrap_or(14.175);
            self.entry.active_propagation = self.solar.indices().map(|(sfi, k_index)| {
                crate::core::propagation::PropagationEngine::calculate(
                    p1,
                    p2,
                    freq_mhz,
                    sfi,
                    k_index as u8,
                    utc_hour,
                    day_of_year,
                )
            });
        }
    }

    pub fn lookup_active_callsign_online(&mut self) {
        let call = self.entry.callsign.trim().to_uppercase();
        if call.is_empty() {
            return;
        }

        // Natychmiastowe sprawdzenie lokalnej bazy (tabele użytkownika)
        if let Some(cb_data) = self.services.local_callbook.lookup(&call) {
            if let Some(n) = cb_data.name.clone() {
                self.entry.name = n;
            }
            if let Some(q) = cb_data.qth.clone() {
                self.entry.qth = q;
            }
            if let Some(g) = cb_data.gridsquare.clone() {
                self.entry.grid = g;
                self.recalculate_distance_from_grid();
            }
            if let Some(s) = cb_data.state.clone() {
                self.entry.state = s;
            }
            if let Some(mgr) = cb_data.qsl_manager.clone() {
                self.entry.qsl_manager = mgr;
            }
            self.gui.chrome.status_message = Some(format!("Znaleziono w bazie Callbook: {call}"));
        }

        let lc = self.services.local_callbook.clone();
        let hamqth_u = self.config.hamqth_username.clone();
        let hamqth_p = self.config.hamqth_password.clone();
        let qrz_u = self.config.qrz_username.clone();
        let qrz_p = self.config.qrz_password.clone();
        let priority = self.config.callbook_priority.clone();
        let tx = self.runtime.qrz_lookup_tx.clone();

        tokio::spawn(async move {
            if let Some(data) = crate::core::callbook::fetch_callsign_data(
                call, lc, hamqth_u, hamqth_p, qrz_u, qrz_p, &priority,
            )
            .await
            {
                let _ = tx.send(data);
            }
        });
    }

    /// Ustawia pole formularza QSO na podstawie nazwy (akcja `set_qso_field`).
    pub(crate) fn apply_qso_field(&mut self, field: &str, value: &str) {
        match field.to_lowercase().as_str() {
            "callsign" => self.entry.callsign = value.to_string(),
            "name" => self.entry.name = value.to_string(),
            "qth" => self.entry.qth = value.to_string(),
            "gridsquare" | "grid" => self.entry.grid = value.to_string(),
            "rst_sent" => self.entry.rst_sent = value.to_string(),
            "rst_rcvd" => self.entry.rst_rcvd = value.to_string(),
            "comment" => self.entry.comment = value.to_string(),
            // F-19: pola bez prefiksu MY_ opisują korespondenta, pola MY_* własną stację.
            "sota_ref" | "sota" => self.entry.sota = value.to_string(),
            "pota_ref" | "pota" => self.entry.pota = value.to_string(),
            "my_sota_ref" | "my_sota" => self.entry.my_refs.sota = value.to_string(),
            "my_pota_ref" | "my_pota" => self.entry.my_refs.pota = value.to_string(),
            "pga_ref" | "pga" => self.entry.pga = value.to_string(),
            "iota" => self.entry.iota = value.to_string(),
            "state" => self.entry.state = value.to_string(),
            _ => {}
        }
    }

    /// Rozpoczyna asynchroniczne zapytanie POTA (akcja `pota_lookup`).
    pub(crate) fn trigger_pota_lookup(&mut self, reference: &str) {
        let reference = reference.trim().to_string();
        if reference.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.plugins_ui.lookup_rx = Some(rx);
        let reference = reference.clone();
        tokio::spawn(async move {
            let result = match crate::plugins::lookups::lookup_pota(&reference).await {
                Ok(info) => crate::plugins::bridge::PluginLookupResult::Pota {
                    reference: info.reference,
                    name: info.name,
                    active: info.active,
                },
                Err(e) => crate::plugins::bridge::PluginLookupResult::Error(e.to_string()),
            };
            let _ = tx.send(result);
        });
    }

    /// Rozpoczyna asynchroniczne zapytanie SOTA (akcja `sota_lookup`).
    pub(crate) fn trigger_sota_lookup(&mut self, reference: &str) {
        let reference = reference.trim().to_string();
        if reference.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.plugins_ui.lookup_rx = Some(rx);
        let reference = reference.clone();
        tokio::spawn(async move {
            let result = match crate::plugins::lookups::lookup_sota(&reference).await {
                Ok(info) => crate::plugins::bridge::PluginLookupResult::Sota {
                    reference: info.reference,
                    name: info.name,
                    points: info.points,
                },
                Err(e) => crate::plugins::bridge::PluginLookupResult::Error(e.to_string()),
            };
            let _ = tx.send(result);
        });
    }

    pub fn tune_to_spot(&mut self, dx_call: &str, freq_khz: f64, band: &str) {
        // Ustaw znak, pasmo i częstotliwość lokalnie
        self.entry.callsign = dx_call.to_string();
        self.entry.band = band.to_string();
        let freq_hz = (freq_khz * 1000.0) as u64;
        self.set_vfo_frequency(freq_hz);
        self.on_callsign_changed();
        self.focus_qso_entry();
        self.gui.chrome.status_message = Some(
            Msg::new(key::CLUSTER_AUTO_TUNE_STATUS)
                .arg("call", dx_call)
                .arg("freq", format!("{freq_khz:.1}"))
                .text(self.gui.appearance.language),
        );
    }

    /// Zwraca rodzaj odznaki spotu klastra (nowe DXCC, nowe pasmo, nowa emisja, zrobiona),
    /// korzystając z cache, aby nie przeliczać prefiksu i statusu nagród co klatkę.
    /// Emisja pochodzi z komentarza albo planu pasma; gdy jest nieznana, oceniane są
    /// tylko kraj i pasmo.
    pub fn cluster_spot_badge(&mut self, spot: &DxSpot) -> SpotBadgeKind {
        let freq_hz = spot_info::spot_freq_hz(spot.frequency_khz);
        let mode = spot_info::spot_mode(&spot.comment, freq_hz, spot.is_ft8);
        let mode_name = mode.as_ref().map(SpotMode::log_mode);
        let key = format!(
            "{}|{}|{}",
            spot.dx_call,
            spot.band,
            mode_name.unwrap_or_default()
        );
        if let Some(&badge) = self.cluster.badge_cache.get(&key) {
            return badge;
        }

        let badge = if let Some(info) = self.services.prefix_matcher.lookup(&spot.dx_call) {
            let awards = self
                .services
                .awards_engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let st = awards.check_status_full(
                &spot.dx_call,
                &spot.band,
                mode_name.unwrap_or_default(),
                Some(info.dxcc),
                None,
                Some(info.cqz),
                None,
                Some(info.continent.as_str()),
                None,
            );
            if st.is_new_dxcc {
                SpotBadgeKind::NewDxcc
            } else if st.is_new_band {
                SpotBadgeKind::NewBand
            } else if mode_name.is_some() && st.is_new_mode {
                SpotBadgeKind::NewMode
            } else if mode_name.is_some() && st.is_worked_b4 {
                SpotBadgeKind::Worked
            } else {
                SpotBadgeKind::Standard
            }
        } else {
            SpotBadgeKind::Standard
        };

        self.cluster.badge_cache.insert(key, badge);
        badge
    }

    /// Nanosi spot z bieżącego pasma na panoramę SDR (TCI, FlexRadio), jeśli włączono to
    /// w ustawieniach spotów. Kolor odpowiada odznace spotu.
    pub fn show_spot_on_sdr(&mut self, spot: &DxSpot) {
        let settings = &self.config.cluster_spots;
        if !settings.show_on_sdr || spot.band != self.entry.band {
            return;
        }
        let lifetime_secs = u32::try_from(settings.max_age_secs()).unwrap_or(u32::MAX);
        let rx_hz = spot_info::spot_freq_hz(spot.frequency_khz);
        if rx_hz == 0 {
            return;
        }
        let tx_hz = match spot_info::split_hint(&spot.comment, rx_hz) {
            Some(SplitHint::Tx(tx_hz)) => Some(tx_hz),
            _ => None,
        };
        let mode = spot_info::spot_mode(&spot.comment, rx_hz, spot.is_ft8)
            .map(|m| m.log_mode().to_string());
        let color_argb = match self.cluster_spot_badge(spot) {
            SpotBadgeKind::NewDxcc => spot_info::SDR_COLOR_NEW_DXCC,
            SpotBadgeKind::NewBand => spot_info::SDR_COLOR_NEW_BAND,
            SpotBadgeKind::NewMode => spot_info::SDR_COLOR_NEW_MODE,
            SpotBadgeKind::Worked => spot_info::SDR_COLOR_WORKED,
            SpotBadgeKind::Standard => spot_info::SDR_COLOR_OTHER,
        };
        self.cat
            .runtime
            .execute_cmd(RigCommand::ShowSpot(Box::new(PanadapterSpot {
                callsign: spot.dx_call.clone(),
                rx_hz,
                tx_hz,
                mode,
                color_argb,
                spotter: spot.spotter.clone(),
                comment: spot.comment.clone(),
                lifetime_secs,
            })));
    }

    /// Usuwa spoty z panoramy SDR razem z listą klastra.
    pub fn clear_sdr_spots(&self) {
        if self.config.cluster_spots.show_on_sdr {
            self.cat.runtime.execute_cmd(RigCommand::ClearSpots);
        }
    }

    /// Azymut z lokatora stacji na stację DX (z pliku krajów); `None` bez lokatora stacji
    /// albo dla nieznanego prefiksu.
    pub fn bearing_to_callsign(&self, callsign: &str) -> Option<f32> {
        let info = self.services.prefix_matcher.lookup(callsign)?;
        let mine = locator_to_coordinates(&self.config.station.gridsquare).ok()?;
        let dx = crate::core::geo::Coordinates::new(info.latitude, info.longitude);
        let bearing = calculate_bearing_deg(mine, dx) as f32;
        bearing.is_finite().then_some(bearing)
    }

    /// Strojenie na spot klastra: znak i pasmo do formularza, częstotliwość, emisja i split
    /// do radia jedną sekwencją, opcjonalnie obrót anteny.
    pub fn tune_to_cluster_spot(&mut self, spot: &DxSpot) {
        let settings = self.config.cluster_spots.clone();
        let freq_hz = spot_info::spot_freq_hz(spot.frequency_khz);
        if freq_hz == 0 {
            return;
        }
        let mode = if settings.tune_sets_mode {
            spot_info::spot_mode(&spot.comment, freq_hz, spot.is_ft8)
        } else {
            None
        };
        let split = if settings.tune_sets_split {
            spot_info::split_hint(&spot.comment, freq_hz)
        } else {
            None
        };
        let rig_setting = mode.as_ref().and_then(|m| m.rig_setting(freq_hz));
        let split_request = match split {
            Some(SplitHint::Tx(tx_hz)) => SplitRequest::On(tx_hz),
            _ if settings.tune_sets_split && self.cat.rig_state.split_enabled => SplitRequest::Off,
            _ => SplitRequest::Keep,
        };

        self.entry.callsign.clone_from(&spot.dx_call);
        self.entry.band = crate::core::bandplan::get_band_by_freq(freq_hz)
            .map_or_else(|| spot.band.clone(), |band| band.name.to_string());
        if let Some(mode) = &mode {
            self.entry.mode = mode.log_mode().to_string();
        }
        // Stan lokalny do czasu odczytu z radia.
        self.cat.rig_state.frequency_hz = freq_hz;
        if let Some((rig_mode, passband_hz)) = rig_setting {
            self.cat.rig_state.mode = rig_mode.to_string();
            self.cat.rig_state.passband_hz = passband_hz;
        }
        match split_request {
            SplitRequest::On(_) => {
                self.cat.rig_state.split_enabled = true;
                self.cat.vfo_split = true;
            }
            SplitRequest::Off => {
                self.cat.rig_state.split_enabled = false;
                self.cat.vfo_split = false;
            }
            SplitRequest::Keep => {}
        }
        self.cat.runtime.tune(TuneRequest {
            frequency_hz: freq_hz,
            mode: rig_setting.map(|(m, passband)| (m.to_string(), passband)),
            split: split_request,
        });

        self.on_callsign_changed();
        self.focus_qso_entry();
        let lang = self.gui.appearance.language;
        let mut status = Msg::new(key::CLUSTER_AUTO_TUNE_STATUS)
            .arg("call", &spot.dx_call)
            .arg("freq", format!("{:.1}", spot.frequency_khz))
            .text(lang);
        if let SplitRequest::On(tx_hz) = split_request {
            status.push_str(" · ");
            status.push_str(
                &Msg::new(key::CLUSTER_SPLIT_TX)
                    .arg("freq", format!("{:.1}", tx_hz as f64 / 1000.0))
                    .text(lang),
            );
        }
        self.gui.chrome.status_message = Some(status);
        if split == Some(SplitHint::UpUnspecified) {
            self.gui.chrome.push_status_toast(
                crate::gui::state::ToastSeverity::Warning,
                tr(key::CLUSTER_SPLIT_UNSPECIFIED, lang).to_string(),
            );
        }
        if settings.rotor_follows_spot
            && let Some(azimuth) = self.bearing_to_callsign(&spot.dx_call)
        {
            self.rotate_antenna_to(azimuth.round());
        }
    }

    /// Czyści cache odznak spotów klastra po zmianie stanu nagród (nowe/usunięte QSO)
    /// i zleca przeliczenie wyróżnień znaków w WSJT-X.
    pub fn invalidate_cluster_badges(&mut self) {
        self.cluster.badge_cache.clear();
        self.digital.wsjtx.marks_stale = true;
    }
}
