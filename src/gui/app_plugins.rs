use super::SpLogApp;
use crate::core::i18n::{Msg, key};

impl SpLogApp {
    /// Buduje migawkę stanu (radio, rotor, nagrody, ostatnia łączność) widoczną
    /// dla getterów pluginów Rhai. Wywoływana co klatkę oraz po zapisie łączności.
    pub fn refresh_plugin_snapshot(&mut self) {
        let awards = {
            let guard = self
                .services
                .awards_engine
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            crate::plugins::bridge::AwardSnapshot {
                dxcc_worked: guard.worked_dxcc_all.len() as i64,
                dxcc_confirmed: guard.confirmed_dxcc.len() as i64,
                waz_worked: guard.worked_waz.len() as i64,
                was_worked: guard.worked_was.len() as i64,
                wac_worked: guard.worked_wac.len() as i64,
                iota_worked: guard.worked_iota.len() as i64,
                pota_parks_worked: guard.worked_pota.len() as i64,
                sota_summits_worked: guard.worked_sota.len() as i64,
                pga_gminas_worked: guard.worked_pga.len() as i64,
            }
        };

        let award = self.entry.active_award_status.as_ref();
        let last_qso = crate::plugins::bridge::QsoSnapshot {
            callsign: self.entry.callsign.clone(),
            band: self.entry.band.clone(),
            mode: self.entry.mode.clone(),
            freq_mhz: self.cat.rig_state.frequency_hz as f64 / 1_000_000.0,
            name: self.entry.name.clone(),
            qth: self.entry.qth.clone(),
            gridsquare: self.entry.grid.clone(),
            country: self
                .entry
                .active_prefix_info
                .as_ref()
                .map(|i| i.country.clone())
                .unwrap_or_default(),
            dxcc: self
                .entry
                .active_prefix_info
                .as_ref()
                .map(|i| i.dxcc.to_string())
                .unwrap_or_default(),
            sota_ref: self.entry.sota.clone(),
            pota_ref: self.entry.pota.clone(),
            pga_ref: self.entry.pga.clone(),
            iota: self.entry.iota.clone(),
            state: self.entry.state.clone(),
            rst_sent: self.entry.rst_sent.clone(),
            rst_rcvd: self.entry.rst_rcvd.clone(),
            comment: self.entry.comment.clone(),
            is_atno: award.is_some_and(|s| s.is_new_dxcc),
            is_new_band: award.is_some_and(|s| s.is_new_band),
            is_new_mode: award.is_some_and(|s| s.is_new_mode),
            is_new_iota: award.is_some_and(|s| s.is_new_iota),
            is_new_waz: award.is_some_and(|s| s.is_new_waz),
            is_new_was: award.is_some_and(|s| s.is_new_was),
            is_new_wac: award.is_some_and(|s| s.is_new_wac),
            is_new_pga: award.is_some_and(|s| s.is_new_pga),
        };

        self.services
            .plugin_engine
            .set_snapshot(crate::plugins::bridge::PluginSnapshot {
                rig_freq_mhz: self.cat.rig_state.frequency_hz as f64 / 1_000_000.0,
                rig_mode: self.cat.rig_state.mode.clone(),
                rig_band: self.entry.band.clone(),
                rig_connected: self.cat.rig_state.connected,
                rotor_azimuth_deg: self.cat.rotor_state.azimuth_deg,
                rotor_elevation_deg: self.cat.rotor_state.elevation_deg,
                my_call: self.config.station.callsign.clone(),
                qso_count: self.log_view.qso_numbers.len() as i64,
                awards,
                last_qso,
            });
    }

    /// Odpytuje kanał wyników zapytań POTA/SOTA z akcji pluginów i uruchamia
    /// odpowiadające haki (`on_pota_info` / `on_sota_info`).
    pub fn poll_plugin_lookup(&mut self) {
        let mut result = None;
        if let Some(rx) = &self.plugins_ui.lookup_rx {
            if let Ok(r) = rx.try_recv() {
                result = Some(r);
            }
        }
        if let Some(r) = result {
            match r {
                crate::plugins::bridge::PluginLookupResult::Pota {
                    reference,
                    name,
                    active,
                } => {
                    self.services
                        .plugin_engine
                        .run_on_pota_info(&reference, &name, active);
                }
                crate::plugins::bridge::PluginLookupResult::Sota {
                    reference,
                    name,
                    points,
                } => {
                    self.services
                        .plugin_engine
                        .run_on_sota_info(&reference, &name, points);
                }
                crate::plugins::bridge::PluginLookupResult::Error(msg) => {
                    self.gui.chrome.status_toast =
                        Some((format!("Plugin: {msg}"), std::time::Instant::now()));
                }
            }
        }
    }

    /// Wykonuje polecenia zakolejkowane przez pluginy Rhai.
    pub fn process_plugin_commands(&mut self) {
        let commands = self.services.plugin_engine.drain_commands();
        for cmd in commands {
            match cmd {
                crate::plugins::bridge::PluginCommand::SendCw { text } => {
                    self.transmit_cw_macro(&text);
                }
                crate::plugins::bridge::PluginCommand::SendVoice { text } => {
                    if text.trim().to_lowercase().ends_with(".wav") {
                        let path = text.trim().to_string();
                        let _ = crate::media::voice_keyer::play_wav_file(&path);
                    } else {
                        self.gui.chrome.status_toast = Some((
                            Msg::new(key::PLUGIN_VOICE_ANNOUNCEMENT)
                                .arg("text", &text)
                                .text(self.gui.appearance.language),
                            std::time::Instant::now(),
                        ));
                    }
                }
                crate::plugins::bridge::PluginCommand::Rotate {
                    azimuth_deg,
                    elevation_deg,
                } => {
                    self.rotate_antenna_to_el(azimuth_deg, elevation_deg);
                }
                crate::plugins::bridge::PluginCommand::Spot {
                    dx_call,
                    freq_khz,
                    comment,
                } => {
                    self.publish_local_spot(&dx_call, freq_khz, &comment);
                }
                crate::plugins::bridge::PluginCommand::SetQsoField { field, value } => {
                    self.apply_qso_field(&field, &value);
                }
                crate::plugins::bridge::PluginCommand::PlaySound { name } => {
                    self.play_plugin_sound(&name);
                }
                crate::plugins::bridge::PluginCommand::PotaLookup { reference } => {
                    self.trigger_pota_lookup(&reference);
                }
                crate::plugins::bridge::PluginCommand::SotaLookup { reference } => {
                    self.trigger_sota_lookup(&reference);
                }
                crate::plugins::bridge::PluginCommand::Notify { message } => {
                    self.gui.chrome.status_toast = Some((message, std::time::Instant::now()));
                }
            }
        }
    }

    /// Wstawia lokalny spot DX (akcja `spot`). Kanał wysyłki telnetowej klastra
    /// nie jest dostępny dla pluginów, więc spot pojawia się w panelu klastra.
    pub(crate) fn publish_local_spot(&mut self, dx_call: &str, freq_khz: f64, comment: &str) {
        let band_str = crate::core::bandplan::get_band_by_freq((freq_khz * 1000.0) as u64)
            .map_or_else(|| "HF".to_string(), |b| b.name.to_string());
        let is_ft8 = comment.to_uppercase().contains("FT8");
        let spot = crate::cluster::telnet::DxSpot {
            frequency_khz: freq_khz,
            dx_call: dx_call.to_string(),
            spotter: self.config.station.callsign.clone(),
            comment: comment.to_string(),
            time_utc: chrono::Utc::now().format("%H%M").to_string(),
            band: band_str,
            is_ft8,
            is_skimmer: false,
            received_at: chrono::Utc::now().timestamp(),
        };
        self.cluster.spots.insert(0, spot);
        self.gui.chrome.status_toast = Some((
            format!("Spot (plugin): {dx_call} na {freq_khz:.1} kHz"),
            std::time::Instant::now(),
        ));
    }

    /// Odtwarza dźwięk powiadomienia o zadanej nazwie (akcja `play_sound`).
    pub(crate) fn play_plugin_sound(&mut self, name: &str) {
        match name.trim().to_lowercase().as_str() {
            "new_dxcc" | "atno" => crate::media::sounds::play_new_dxcc_alert(),
            "duplicate" | "dupe" => crate::media::sounds::play_duplicate_alert(),
            "new_iota" | "iota" => crate::media::sounds::play_new_iota_alert(),
            "qso_saved" | "saved" => crate::media::sounds::play_qso_saved_alert(),
            "band_opened" | "band" => crate::media::sounds::play_band_opened_alert(),
            _ => {
                self.gui.chrome.status_toast = Some((
                    Msg::new(key::PLUGIN_UNKNOWN_SOUND)
                        .arg("name", &name)
                        .text(self.gui.appearance.language),
                    std::time::Instant::now(),
                ));
            }
        }
    }

    pub fn transmit_cw_macro(&mut self, text: &str) {
        let resolved = text
            .replace("%MYCALL%", &self.config.station.callsign)
            .replace("%MYQTH%", &self.config.station.city)
            .replace("%MYLOC%", &self.config.station.gridsquare)
            .replace("%HISCALL%", &self.entry.callsign)
            .replace("%RST%", &self.entry.rst_sent)
            .replace("%SERIAL%", &format!("{:03}", self.contest.stx));
        self.gui.chrome.status_message =
            Some(format!("Nadawanie CW ({} WPM): {}", self.cw.wpm, resolved));
    }
}
