// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Okno kalkulatora pozycji Księżyca i Słońca (EME / Doppler / Rotor)

use crate::cat::rotor::RotorState;
use crate::core::astronomy::AstronomyEngine;
use crate::core::geo::locator_to_coordinates;
use chrono::Utc;
use eframe::egui;

pub struct AstronomyDialog {
    pub is_open: bool,
}

impl Default for AstronomyDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl AstronomyDialog {
    pub fn new() -> Self {
        Self { is_open: false }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        my_gridsquare: &str,
        rotor_state: &mut RotorState,
        lang: crate::core::i18n::Language,
    ) {
        use crate::core::i18n::{key, tr};
        if !self.is_open {
            return;
        }

        let mut open = self.is_open;
        let mut close_requested = false;

        let (lat, lon) = match locator_to_coordinates(my_gridsquare) {
            Ok(coords) => (coords.latitude, coords.longitude),
            Err(_) => (52.0, 19.0), // Środek Polski
        };

        let now = Utc::now();
        let eme = AstronomyEngine::calculate_eme(lat, lon, now);
        let moon = &eme.moon;
        let sun = &eme.sun;

        let mut turn_rotor_az = None;
        let mut turn_rotor_el = None;

        egui::Window::new(tr(key::EME_WINDOW_TITLE, lang))
            .open(&mut open)
            .default_width(520.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {} ({:.2}°, {:.2}°) | {} {}",
                            tr(key::EME_LOCATION, lang),
                            my_gridsquare,
                            lat,
                            lon,
                            tr(key::EME_TIME, lang),
                            now.format("%H:%M:%S UTC")
                        ))
                        .small()
                        .color(egui::Color32::GRAY),
                    );
                });
                ui.separator();

                ui.columns(2, |cols| {
                    // Kolumna 1: Księżyc (Moon / EME)
                    cols[0].group(|ui| {
                        ui.heading(
                            egui::RichText::new(tr(key::EME_MOON_HEADING, lang))
                                .color(egui::Color32::from_rgb(186, 230, 253)),
                        );
                        ui.separator();
                        ui.label(format!(
                            "{} {:.1}°",
                            tr(key::EME_AZIMUTH, lang),
                            moon.azimuth_deg
                        ));
                        ui.label(format!(
                            "{} {:.1}°",
                            tr(key::EME_ELEVATION, lang),
                            moon.elevation_deg
                        ));
                        ui.label(format!(
                            "{} {:.0} km",
                            tr(key::EME_DISTANCE, lang),
                            moon.distance_km
                        ));
                        ui.label(format!(
                            "{} {:.2} h",
                            tr(key::EME_RA, lang),
                            moon.right_ascension_hours
                        ));
                        ui.label(format!(
                            "{} {:.1}°",
                            tr(key::EME_DEC, lang),
                            moon.declination_deg
                        ));
                        ui.add_space(4.0);

                        let visible_txt = if moon.is_visible {
                            tr(key::EME_ABOVE_HORIZON, lang)
                        } else {
                            tr(key::EME_BELOW_HORIZON, lang)
                        };
                        ui.label(egui::RichText::new(visible_txt).strong().color(
                            if moon.is_visible {
                                egui::Color32::from_rgb(34, 197, 94)
                            } else {
                                egui::Color32::GRAY
                            },
                        ));

                        ui.add_space(8.0);
                        if ui.button(tr(key::EME_POINT_ROTOR_MOON, lang)).clicked() {
                            turn_rotor_az = Some(moon.azimuth_deg as f32);
                            turn_rotor_el = Some(moon.elevation_deg.max(0.0) as f32);
                        }
                    });

                    // Kolumna 2: Słońce (Sun)
                    cols[1].group(|ui| {
                        ui.heading(
                            egui::RichText::new(tr(key::EME_SUN_HEADING, lang))
                                .color(egui::Color32::from_rgb(253, 224, 71)),
                        );
                        ui.separator();
                        ui.label(format!(
                            "{} {:.1}°",
                            tr(key::EME_AZIMUTH, lang),
                            sun.azimuth_deg
                        ));
                        ui.label(format!(
                            "{} {:.1}°",
                            tr(key::EME_ELEVATION, lang),
                            sun.elevation_deg
                        ));
                        ui.label(format!(
                            "{} {:.3} AU",
                            tr(key::EME_DISTANCE, lang),
                            sun.distance_km / 149_597_870.7
                        ));
                        ui.label(format!(
                            "{} {:.2} h",
                            tr(key::EME_RA, lang),
                            sun.right_ascension_hours
                        ));
                        ui.label(format!(
                            "{} {:.1}°",
                            tr(key::EME_DEC, lang),
                            sun.declination_deg
                        ));
                        ui.add_space(4.0);

                        let visible_txt = if sun.is_visible {
                            tr(key::EME_DAY_ABOVE_HORIZON, lang)
                        } else {
                            tr(key::EME_NIGHT_BELOW_HORIZON, lang)
                        };
                        ui.label(egui::RichText::new(visible_txt).strong().color(
                            if sun.is_visible {
                                egui::Color32::from_rgb(250, 204, 21)
                            } else {
                                egui::Color32::GRAY
                            },
                        ));

                        ui.add_space(8.0);
                        if ui.button(tr(key::EME_POINT_ROTOR_SUN, lang)).clicked() {
                            turn_rotor_az = Some(sun.azimuth_deg as f32);
                            turn_rotor_el = Some(sun.elevation_deg.max(0.0) as f32);
                        }
                    });
                });

                ui.add_space(10.0);

                // Parametry łączności odbiciowej od Księżyca (EME)
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new(tr(key::EME_PARAMS_HEADING, lang))
                            .strong()
                            .color(egui::Color32::from_rgb(134, 239, 172)),
                    );
                    ui.separator();
                    ui.label(format!(
                        "{} {:.1} dB",
                        tr(key::EME_PATH_LOSS, lang),
                        eme.path_loss_144_db
                    ));
                    ui.separator();
                    ui.label(format!(
                        "{} {:+.1} Hz",
                        tr(key::EME_DOPPLER_144, lang),
                        eme.doppler_144_hz
                    ));
                    ui.label(format!(
                        "{} {:+.1} Hz",
                        tr(key::EME_DOPPLER_432, lang),
                        eme.doppler_432_hz
                    ));
                    ui.label(format!(
                        "{} {:+.1} Hz",
                        tr(key::EME_DOPPLER_1296, lang),
                        eme.doppler_1296_hz
                    ));
                });

                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_requested = true;
                    }
                });
            });

        if let Some(az) = turn_rotor_az {
            rotor_state.azimuth_deg = az;
        }
        if let Some(el) = turn_rotor_el {
            rotor_state.elevation_deg = el;
        }

        if close_requested || !open {
            self.is_open = false;
        }
    }
}
