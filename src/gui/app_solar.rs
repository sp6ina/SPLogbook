use super::SpLogApp;
use crate::cloud::solar::SpaceWeather;
use crate::core::geo::locator_to_coordinates;
use crate::core::i18n::{Msg, key};
use eframe::egui;

impl SpLogApp {
    /// Zwraca prognozy otwarcia pasm dla panelu słonecznego z cache'em kluczowanym
    /// po (SFI, K, godzina, minuta, dzień roku), aby nie przeliczać 11 pasm co klatkę.
    pub fn solar_band_forecasts(
        &mut self,
    ) -> Vec<(&'static str, crate::core::propagation::PropagationForecast)> {
        use chrono::{Datelike, Timelike};

        let sfi = if self.solar.space_weather.sfi > 0 {
            self.solar.space_weather.sfi
        } else {
            140
        };
        let k_index = self.solar.space_weather.k_index;
        let now = chrono::Utc::now();
        let hour = now.hour();
        let minute = now.minute();
        let doy = now.ordinal();
        let key = (sfi, k_index, hour, minute, doy);

        if let Some((cached_key, cached)) = &self.solar.propagation_cache {
            if *cached_key == key {
                return cached.clone();
            }
        }

        let sp = crate::core::geo::Coordinates::new(51.1, 17.0);
        let dx = crate::core::geo::Coordinates::new(40.7, -74.0);
        let utc_h = hour as f64 + (minute as f64) / 60.0;
        let forecasts: Vec<(&'static str, crate::core::propagation::PropagationForecast)> =
            crate::core::propagation::HF_BANDS
                .iter()
                .map(|&(name, freq)| {
                    (
                        name,
                        crate::core::propagation::PropagationEngine::calculate(
                            sp,
                            dx,
                            freq,
                            sfi,
                            k_index as u8,
                            utc_h,
                            doy,
                        ),
                    )
                })
                .collect();

        // Zapis prognoz do historii dokładności oraz wykrywanie przejść
        // otwarcia/zamknięcia pasm (dla alertów dźwiękowych i pluginów).
        let mut newly_opened: Vec<&'static str> = Vec::new();
        for (name, fc) in &forecasts {
            let band = *name;
            self.solar.propagation_history.record_forecast(
                band,
                utc_h,
                sfi,
                k_index as u8,
                fc.reliability_pct,
                fc.status,
            );
            if let Some(prev) = self.solar.previous_band_status.get(band).copied() {
                let was_open = prev == crate::core::propagation::BandOpeningStatus::Open;
                let is_open = fc.status == crate::core::propagation::BandOpeningStatus::Open;
                if !was_open && is_open {
                    newly_opened.push(band);
                }
            }
            self.solar
                .previous_band_status
                .insert(band.to_string(), fc.status);
        }

        if !newly_opened.is_empty() && self.solar.band_alert_enabled {
            let names = newly_opened.join(", ");
            crate::media::sounds::play_band_opened_alert();
            self.gui.chrome.status_toast = Some((
                format!("📡 Otwarcie pasma: {names}"),
                std::time::Instant::now(),
            ));
            for band in newly_opened {
                self.services.plugin_engine.run_on_band_opened(band);
            }
        }

        self.solar.propagation_cache = Some((key, forecasts.clone()));
        forecasts
    }

    /// Odświeża pozycję satelity i listę najbliższych przelotów (przybliżenie keplerowskie).
    /// Wywoływane co klatkę, ale przeliczane co najwyżej raz na minutę.
    pub fn refresh_satellite_tracking(&mut self) {
        let now = crate::cloud::scheduler::now_unix() as f64;
        let minute = (now / 60.0) as u64;
        if !self.satellites.needs_refresh(
            &self.satellites.selected_satellite,
            &self.config.station.gridsquare,
            minute,
        ) {
            return;
        }
        self.satellites.mark_refreshed(
            &self.satellites.selected_satellite.clone(),
            &self.config.station.gridsquare,
            minute,
        );

        let def = crate::core::satellite::default_satellites()
            .into_iter()
            .find(|d| d.name == self.satellites.selected_satellite);

        let coords = locator_to_coordinates(&self.config.station.gridsquare)
            .unwrap_or(crate::core::geo::Coordinates::new(52.2297, 21.0122));
        let obs = crate::core::satellite::Observer {
            lat_deg: coords.latitude,
            lon_deg: coords.longitude,
        };

        if let Some(def) = def {
            self.satellites.passes =
                crate::core::satellite::predict_passes(&def, &obs, now, 10.0, 24.0);
            self.satellites.azimuth = crate::core::satellite::azimuth_deg(&def, &obs, now) as f32;
            self.satellites.elevation =
                crate::core::satellite::elevation_deg(&def, &obs, now) as f32;
            self.satellites.range_km = crate::core::satellite::range_km(&def, &obs, now) as f32;
            self.satellites.altitude_km = def.altitude_km as f32;
            self.satellites.downlink_mhz = def.downlink_mhz;
            self.satellites.uplink_mhz = def.uplink_mhz;
            self.satellites.rx_doppler_khz =
                crate::core::satellite::doppler_khz(&def, &obs, now, def.downlink_mhz) as f32;
            self.satellites.tx_doppler_khz =
                crate::core::satellite::doppler_khz(&def, &obs, now, def.uplink_mhz) as f32;
        } else {
            self.satellites.passes.clear();
        }
    }

    /// Uruchamia asynchroniczne pobranie warunków solarnych (HamQTH).
    pub fn refresh_solar_weather(&mut self, ctx: &egui::Context) {
        if self.solar.loading {
            return;
        }
        self.solar.loading = true;

        let slot: std::sync::Arc<
            std::sync::Mutex<Option<Result<SpaceWeather, crate::cloud::solar::SpaceWeatherError>>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(None));
        let slot_clone = slot.clone();
        let ctx_clone = ctx.clone();

        tokio::spawn(async move {
            let result = crate::cloud::solar::SpaceWeatherClient::new()
                .fetch_hamqth_solar()
                .await;
            *slot_clone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(result);
            ctx_clone.request_repaint();
        });

        self.solar.fetch_slot = Some(slot);
    }

    /// Odbiera wynik asynchronicznego pobrania warunków solarnych.
    pub fn poll_solar_fetch(&mut self) {
        if !self.solar.loading {
            return;
        }
        if let Some(ref slot) = self.solar.fetch_slot.clone() {
            if let Ok(mut guard) = slot.try_lock() {
                if let Some(result) = guard.take() {
                    self.solar.loading = false;
                    self.solar.fetch_slot = None;
                    match result {
                        Ok(weather) => self.record_solar_sample(weather),
                        Err(e) => {
                            self.solar.last_alert = Some(
                                Msg::new(key::SOLAR_FETCH_ERROR)
                                    .arg("err", &e.to_string())
                                    .text(self.gui.appearance.language),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Zapisuje próbkę warunków solarnych do historii i wykrywa istotne zmiany.
    pub fn record_solar_sample(&mut self, weather: SpaceWeather) {
        if let Some(prev) = self.solar.history.last() {
            let k_delta = weather.k_index as i64 - prev.k_index as i64;
            let sfi_delta = weather.sfi as i64 - prev.sfi as i64;
            if k_delta.abs() >= 2 {
                self.solar.last_alert = Some(
                    Msg::new(key::SOLAR_K_INDEX_ALERT)
                        .arg("prev", prev.k_index)
                        .arg("curr", weather.k_index)
                        .text(self.gui.appearance.language),
                );
            } else if sfi_delta.abs() >= 20 {
                self.solar.last_alert =
                    Some(format!("ℹ Zmiana SFI: {} → {}", prev.sfi, weather.sfi));
            } else {
                self.solar.last_alert = None;
            }
        }

        self.solar.space_weather = weather.clone();

        let differs = match self.solar.history.last() {
            Some(p) => {
                p.sfi != weather.sfi
                    || p.ssn != weather.ssn
                    || p.k_index != weather.k_index
                    || p.a_index != weather.a_index
            }
            None => true,
        };
        if differs {
            self.solar.history.push(weather);
            if self.solar.history.len() > 96 {
                self.solar.history.remove(0);
            }
        }
    }
}
