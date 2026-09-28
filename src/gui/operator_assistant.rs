// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Asystent operatora — zawsze dostępny pasek decyzyjny, który podpowiada
//! "co robić teraz" na podstawie warunków propagacyjnych, stanu radia (CAT),
//! klastra DX i celów dyplomowych. Funkcja `recommend` jest czysta i testowana
//! niezależnie od GUI.

use crate::core::propagation::{BandOpeningStatus, PropagationEngine, PropagationForecast};
use crate::gui::app::SpLogApp;
use chrono::{Datelike, Timelike};
use eframe::egui;

/// Rekomendacja wygenerowana przez asystenta operatora.
#[derive(Debug, Clone, PartialEq)]
pub struct AssistantRecommendation {
    /// Jednozdaniowa podpowiedź "co robić teraz".
    pub headline: String,
    /// Nazwy najlepszych pasm (najpierw najlepsze).
    pub best_bands: Vec<String>,
    /// Opis stanu aktualnego pasma (np. "Otwarte — 72%").
    pub current_band_status: String,
    /// Dodatkowe uwagi/wskazówki.
    pub advice: Vec<String>,
}

/// Zwraca status pasma jako (ikona, kolor).
pub fn status_style(status: BandOpeningStatus) -> (&'static str, egui::Color32) {
    match status {
        BandOpeningStatus::Open => ("🟢", egui::Color32::from_rgb(34, 197, 94)),
        BandOpeningStatus::Marginal => ("🟡", egui::Color32::from_rgb(234, 179, 8)),
        BandOpeningStatus::Closed => ("🔴", egui::Color32::from_rgb(239, 68, 68)),
    }
}

/// Czysta logika rekomendacji: na podstawie prognoz pasm, aktualnego pasma
/// i indeksu K podpowiada, co robić teraz.
pub fn recommend(
    forecasts: &[(&'static str, PropagationForecast)],
    current_band: &str,
    k_index: u32,
) -> AssistantRecommendation {
    let mut ordered: Vec<(&'static str, PropagationForecast)> = forecasts.to_vec();
    // Sortuj: Otwarte → Trudne → Zamknięte, a w ramach statusu po niezawodności.
    ordered.sort_by(|a, b| {
        let rank = |s: BandOpeningStatus| match s {
            BandOpeningStatus::Open => 0u8,
            BandOpeningStatus::Marginal => 1u8,
            BandOpeningStatus::Closed => 2u8,
        };
        rank(a.1.status)
            .cmp(&rank(b.1.status))
            .then_with(|| b.1.reliability_pct.cmp(&a.1.reliability_pct))
    });

    let open: Vec<(&'static str, PropagationForecast)> = ordered
        .iter()
        .filter(|&(_, f)| f.status == BandOpeningStatus::Open && f.reliability_pct >= 30)
        .cloned()
        .collect();

    let best_bands: Vec<String> = open
        .iter()
        .take(3)
        .map(|(name, f)| format!("{name} ({:>2}%)", f.reliability_pct))
        .collect();

    let current = forecasts
        .iter()
        .find(|(name, _)| *name == current_band)
        .map(|(_, f)| f.clone());

    let current_band_status = match &current {
        Some(f) => format!("{} — {}%", f.status.as_str(), f.reliability_pct),
        None => "brak danych".to_string(),
    };

    let headline = match (&current, best_bands.first()) {
        (Some(f), _) if f.status == BandOpeningStatus::Open => {
            format!("📡 Pasmo {current_band} otwarte ({}%) — pracuj teraz!", f.reliability_pct)
        }
        (_, Some(best)) => {
            format!("📡 Przełącz na {best} — najlepsze warunki")
        }
        (Some(f), _) if f.status == BandOpeningStatus::Marginal => {
            format!("⚠️ {current_band} trudne ({}%) — rozważ 20m/40m", f.reliability_pct)
        }
        _ => "🌑 Wszystkie pasma zamknięte — obserwuj MUF i poczekaj".to_string(),
    };

    let mut advice = Vec::new();
    if k_index >= 6 {
        advice.push(format!(
            "🌪️ Indeks K={k_index}: silna burza geomagnetyczna — duża absorpcja polarna i zaniki."
        ));
    } else if k_index >= 4 {
        advice.push(format!(
            "⚡ Indeks K={k_index}: podwyższony — sygnały mogą być niestabilne."
        ));
    }
    if current.map_or(0, |f| f.reliability_pct) < 30 && !best_bands.is_empty() {
        advice.push(format!(
            "Aktualne pasmo słabe — najlepsze teraz: {}.",
            best_bands.join(", ")
        ));
    }

    AssistantRecommendation {
        headline,
        best_bands,
        current_band_status,
        advice,
    }
}

/// Wyznacza aktualne pasmo na podstawie częstotliwości radia (CAT) lub pola formularza.
fn current_band(app: &SpLogApp) -> String {
    if !app.entry_band.trim().is_empty() {
        return app.entry_band.trim().to_string();
    }
    let mhz = app.rig_state.frequency_hz as f64 / 1_000_000.0;
    if mhz > 0.5 {
        return crate::core::propagation::HF_BANDS
            .iter()
            .min_by(|a, b| {
                (a.1 - mhz)
                    .abs()
                    .partial_cmp(&(b.1 - mhz).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }).map_or_else(|| "20m".to_string(), |&(name, _)| name.to_string());
    }
    "20m".to_string()
}

/// Rysuje okno asystenta operatora, jeśli jest włączone.
pub fn render_operator_assistant(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_operator_assistant {
        return;
    }

    let forecasts = app.solar_band_forecasts();
    let k_index = app.space_weather.k_index;
    let band = current_band(app);
    let rec = recommend(&forecasts, &band, k_index);

    // Wytłumaczalność aktualnego pasma (rzeczywista trasa jeśli znany lokator).
    let explanation: Option<crate::core::propagation::PropagationExplanation> = {
        let sfi = if app.space_weather.sfi > 0 {
            app.space_weather.sfi
        } else {
            140
        };
        let now = chrono::Utc::now();
        let utc_h = now.hour() as f64 + (now.minute() as f64) / 60.0;
        let doy = now.ordinal();
        if app.entry_grid.is_empty() {
            let sp = crate::core::geo::Coordinates::new(51.1, 17.0);
            let dx = crate::core::geo::Coordinates::new(40.7, -74.0);
            let freq = crate::core::propagation::band_to_center_mhz(&band).unwrap_or(14.175);
            Some(PropagationEngine::explain(sp, dx, freq, sfi, k_index as u8, utc_h, doy).1)
        } else {
            PropagationEngine::forecast_explain(
                &app.my_station.gridsquare,
                &app.entry_grid,
                &band,
                sfi,
                k_index as u8,
                utc_h,
                doy,
            )
            .ok()
            .map(|(_, e)| e)
        }
    };

    let mut open = app.show_operator_assistant;
    egui::Window::new("🎛️ Asystent operatora")
        .open(&mut open)
        .default_size([460.0, 380.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (icon, color) = if app.rig_state.connected {
                    ("📻", egui::Color32::from_rgb(56, 189, 248))
                } else {
                    ("⛔", egui::Color32::from_rgb(148, 163, 184))
                };
                ui.label(egui::RichText::new(icon).size(20.0));
                if app.rig_state.connected {
                    let mhz = app.rig_state.frequency_hz as f64 / 1_000_000.0;
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.3} MHz · {} · {}",
                            mhz, app.rig_state.mode, band
                        ))
                        .strong()
                        .size(15.0)
                        .color(color),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!("{band} (CAT rozłączone)"))
                            .strong()
                            .size(15.0)
                            .color(color),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .checkbox(&mut app.operator_assistant_enabled, "Zawsze widoczny")
                        .changed()
                    {
                        app.save_station_config();
                    }
                });
            });

            ui.add_space(4.0);
            // Nagłówek rekomendacji.
            ui.label(
                egui::RichText::new(&rec.headline)
                    .size(16.0)
                    .strong()
                    .color(egui::Color32::from_rgb(56, 189, 248)),
            );

            ui.add_space(6.0);
            ui.label(egui::RichText::new("🏆 Najlepsze pasma teraz").strong());
            if rec.best_bands.is_empty() {
                ui.label(
                    egui::RichText::new("Brak pewnie otwartych pasm.")
                        .italics()
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                );
            } else {
                for b in &rec.best_bands {
                    ui.label(format!("   🟢 {b}"));
                }
            }

            ui.add_space(4.0);
            ui.label(format!(
                "📶 Aktualne pasmo: {}",
                rec.current_band_status
            ));

            if !rec.advice.is_empty() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("💡 Wskazówki").strong());
                for a in &rec.advice {
                    ui.label(egui::RichText::new(a).color(egui::Color32::from_rgb(250, 204, 21)));
                }
            }

            // Prognozy wszystkich pasm (zwarta tabela).
            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("📊 Prognozy pasm").strong());
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    egui::Grid::new("assistant_bands_grid")
                        .num_columns(4)
                        .spacing([16.0, 2.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Pasmo").strong());
                            ui.label(egui::RichText::new("Status").strong());
                            ui.label(egui::RichText::new("Rel.").strong());
                            ui.label(egui::RichText::new("S").strong());
                            ui.end_row();
                            for (name, f) in &forecasts {
                                let (icon, color) = status_style(f.status);
                                ui.label(egui::RichText::new(*name).strong());
                                ui.label(egui::RichText::new(format!("{icon} {}", f.status.as_str())).color(color));
                                ui.label(format!("{}%", f.reliability_pct));
                                ui.label(&f.signal_s_units);
                                ui.end_row();
                            }
                        });
                });

            if let Some(e) = &explanation {
                ui.add_space(6.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("🔬 Dlaczego taki wynik? (wytłumaczalność)")
                        .strong()
                        .color(egui::Color32::from_rgb(134, 239, 172)),
                );
                for bullet in e.bullet_points() {
                    ui.label(egui::RichText::new(format!("• {bullet}")).small());
                }
            }
        });

    app.show_operator_assistant = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fc(status: BandOpeningStatus, rel: u8) -> PropagationForecast {
        PropagationForecast {
            reliability_pct: rel,
            signal_s_units: "S9".to_string(),
            layer: "F2 (1-hop)".to_string(),
            muf_mhz: 21.0,
            luf_mhz: 3.0,
            fot_mhz: 17.8,
            distance_km: 7000.0,
            bearing_deg: 300.0,
            status,
        }
    }

    #[test]
    fn recommends_open_current_band() {
        let forecasts = vec![
            ("20m", fc(BandOpeningStatus::Open, 85)),
            ("40m", fc(BandOpeningStatus::Closed, 5)),
        ];
        let r = recommend(&forecasts, "20m", 1);
        assert!(r.headline.contains("20m otwarte"));
        assert!(r.best_bands.iter().any(|b| b.starts_with("20m")));
    }

    #[test]
    fn recommends_switch_when_current_closed() {
        let forecasts = vec![
            ("10m", fc(BandOpeningStatus::Closed, 5)),
            ("40m", fc(BandOpeningStatus::Open, 70)),
        ];
        let r = recommend(&forecasts, "10m", 1);
        assert!(r.headline.contains("Przełącz na 40m"));
        assert!(r.advice.iter().any(|a| a.contains("Aktualne pasmo słabe")));
    }

    #[test]
    fn warns_on_geomagnetic_storm() {
        let forecasts = vec![("20m", fc(BandOpeningStatus::Marginal, 40))];
        let r = recommend(&forecasts, "20m", 6);
        assert!(r.advice.iter().any(|a| a.contains("silna burza")));
    }

    #[test]
    fn all_closed_fallback() {
        let forecasts = vec![("6m", fc(BandOpeningStatus::Closed, 0))];
        let r = recommend(&forecasts, "6m", 2);
        assert!(r.headline.contains("Wszystkie pasma zamknięte"));
        assert!(r.best_bands.is_empty());
    }
}
