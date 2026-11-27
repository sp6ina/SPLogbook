// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Asystent operatora — zawsze dostępny pasek decyzyjny, który podpowiada
//! "co robić teraz" na podstawie warunków propagacyjnych, stanu radia (CAT),
//! klastra DX i celów dyplomowych. Funkcja `recommend` jest czysta i testowana
//! niezależnie od GUI.

use crate::core::i18n::{Language, Msg, key, tr};
use crate::core::propagation::{
    BandOpeningStatus, ForecastReason, PropagationEngine, PropagationExplanation,
    PropagationForecast,
};
use crate::gui::app::SpLogApp;
use crate::gui::theme::{Palette, ThemeExt};
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
pub fn status_style(status: BandOpeningStatus, palette: &Palette) -> (&'static str, egui::Color32) {
    match status {
        BandOpeningStatus::Open => ("🟢", palette.success),
        BandOpeningStatus::Marginal => ("🟡", palette.warning),
        BandOpeningStatus::Closed => ("🔴", palette.danger),
    }
}

/// Czysta logika rekomendacji: na podstawie prognoz pasm, aktualnego pasma
/// i indeksu K podpowiada, co robić teraz.
pub fn recommend(
    forecasts: &[(&'static str, PropagationForecast)],
    current_band: &str,
    k_index: u32,
    lang: Language,
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
        Some(f) => format!(
            "{} — {}%",
            band_status_label(f.status, lang),
            f.reliability_pct
        ),
        None => tr(key::ASSISTANT_NO_DATA, lang).to_string(),
    };

    let headline = match (&current, best_bands.first()) {
        (Some(f), _) if f.status == BandOpeningStatus::Open => Msg::new(key::ASSISTANT_BAND_OPEN)
            .arg("band", current_band)
            .arg("pct", f.reliability_pct)
            .text(lang),
        (_, Some(best)) => Msg::new(key::ASSISTANT_SWITCH_TO_BEST)
            .arg("best", best)
            .text(lang),
        (Some(f), _) if f.status == BandOpeningStatus::Marginal => {
            Msg::new(key::ASSISTANT_BAND_DIFFICULT)
                .arg("band", current_band)
                .arg("pct", f.reliability_pct)
                .text(lang)
        }
        _ => tr(key::ASSISTANT_ALL_BANDS_CLOSED, lang).to_string(),
    };

    let mut advice = Vec::new();
    if k_index >= 6 {
        advice.push(
            Msg::new(key::ASSISTANT_K_INDEX_SEVERE)
                .arg("k_index", k_index)
                .text(lang),
        );
    } else if k_index >= 4 {
        advice.push(
            Msg::new(key::ASSISTANT_K_INDEX_ELEVATED)
                .arg("k_index", k_index)
                .text(lang),
        );
    }
    if current.map_or(0, |f| f.reliability_pct) < 30 && !best_bands.is_empty() {
        advice.push(
            Msg::new(key::ASSISTANT_CURRENT_BAND_WEAK)
                .arg("best", best_bands.join(", "))
                .text(lang),
        );
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
    if !app.entry.band.trim().is_empty() {
        return app.entry.band.trim().to_string();
    }
    let mhz = app.cat.rig_state.frequency_hz as f64 / 1_000_000.0;
    if mhz > 0.5 {
        return crate::core::propagation::HF_BANDS
            .iter()
            .min_by(|a, b| {
                (a.1 - mhz)
                    .abs()
                    .partial_cmp(&(b.1 - mhz).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map_or_else(|| "20m".to_string(), |&(name, _)| name.to_string());
    }
    "20m".to_string()
}

/// Rysuje okno asystenta operatora, jeśli jest włączone.
pub fn render_operator_assistant(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.gui.windows.show_operator_assistant {
        return;
    }

    let forecasts = app.solar_band_forecasts();
    let k_index = app.solar.indices().map_or(0, |(_, k)| k);
    let band = current_band(app);
    let lang = app.gui.appearance.language;
    let rec = recommend(&forecasts, &band, k_index, lang);

    // Wytłumaczalność aktualnego pasma: tylko dla prawdziwej trasy (lokator operatora i
    // korespondenta) i prawdziwych danych słonecznych.
    let explanation: Option<crate::core::propagation::PropagationExplanation> =
        match app.solar.indices() {
            Some((sfi, k)) if !app.entry.grid.is_empty() => {
                let now = jiff::tz::TimeZone::UTC.to_datetime(jiff::Timestamp::now());
                let utc_h = now.hour() as f64 + (now.minute() as f64) / 60.0;
                let doy = now.day_of_year() as u32;
                PropagationEngine::forecast_explain(
                    &app.config.station.gridsquare,
                    &app.entry.grid,
                    &band,
                    sfi,
                    k as u8,
                    utc_h,
                    doy,
                )
                .ok()
                .map(|(_, e)| e)
            }
            _ => None,
        };

    let mut open = app.gui.windows.show_operator_assistant;
    egui::Window::new(tr(key::ASSISTANT_WINDOW_TITLE, lang))
        .open(&mut open)
        .default_size([460.0, 380.0])
        .resizable(true)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.horizontal(|ui| {
                let (icon, color) = if app.cat.rig_state.connected {
                    ("📻", palette.accent)
                } else {
                    ("⛔", palette.text_muted)
                };
                ui.label(egui::RichText::new(icon).size(20.0));
                if app.cat.rig_state.connected {
                    let mhz = app.cat.rig_state.frequency_hz as f64 / 1_000_000.0;
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.3} MHz · {} · {}",
                            mhz, app.cat.rig_state.mode, band
                        ))
                        .strong()
                        .size(15.0)
                        .color(color),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(
                            Msg::new(key::ASSISTANT_BAND_CAT_DISCONNECTED)
                                .arg("band", &band)
                                .text(lang),
                        )
                        .strong()
                        .size(15.0)
                        .color(color),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .checkbox(
                            &mut app.config.operator_assistant_enabled,
                            tr(key::ASSISTANT_ALWAYS_VISIBLE, lang),
                        )
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
                    .color(palette.accent),
            );

            ui.add_space(6.0);
            ui.label(egui::RichText::new(tr(key::ASSISTANT_BEST_BANDS_NOW, lang)).strong());
            if rec.best_bands.is_empty() {
                ui.label(
                    egui::RichText::new(tr(key::ASSISTANT_NO_BANDS_OPEN, lang))
                        .italics()
                        .color(palette.text_muted),
                );
            } else {
                for band in &rec.best_bands {
                    ui.label(format!("   🟢 {band}"));
                }
            }

            ui.add_space(4.0);
            ui.label(format!(
                "{}: {}",
                tr(key::ASSISTANT_CURRENT_BAND_LBL, lang),
                rec.current_band_status
            ));

            if !rec.advice.is_empty() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(tr(key::ASSISTANT_HINTS, lang)).strong());
                for advice in &rec.advice {
                    ui.label(egui::RichText::new(advice).color(palette.warning));
                }
            }

            // Prognozy wszystkich pasm (zwarta tabela).
            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new(tr(key::ASSISTANT_BAND_FORECASTS, lang)).strong());
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    egui::Grid::new("assistant_bands_grid")
                        .num_columns(4)
                        .spacing([16.0, 2.0])
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(tr(key::ASSISTANT_COL_BAND, lang)).strong(),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::ASSISTANT_COL_STATUS, lang)).strong(),
                            );
                            ui.label(
                                egui::RichText::new(tr(key::ASSISTANT_COL_RELIABILITY, lang))
                                    .strong(),
                            );
                            ui.label(egui::RichText::new("S").strong());
                            ui.end_row();
                            for (name, forecast) in &forecasts {
                                let (icon, color) = status_style(forecast.status, &palette);
                                ui.label(egui::RichText::new(*name).strong());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{icon} {}",
                                        band_status_label(forecast.status, lang)
                                    ))
                                    .color(color),
                                );
                                ui.label(format!("{}%", forecast.reliability_pct));
                                ui.label(&forecast.signal_s_units);
                                ui.end_row();
                            }
                        });
                });

            if let Some(e) = &explanation {
                ui.add_space(6.0);
                ui.separator();
                ui.label(
                    egui::RichText::new(tr(key::ASSISTANT_EXPLAINABILITY, lang))
                        .strong()
                        .color(palette.success),
                );
                for bullet in explanation_lines(e, lang) {
                    ui.label(egui::RichText::new(format!("• {bullet}")).small());
                }
            }
        });

    if app.gui.windows.show_operator_assistant != open {
        app.gui.windows.show_operator_assistant = open;
        app.config.operator_assistant_enabled = open;
        app.save_station_config();
    }
}

/// Stan otwarcia pasma w języku interfejsu.
pub(crate) fn band_status_label(status: BandOpeningStatus, lang: Language) -> &'static str {
    let k = match status {
        BandOpeningStatus::Open => key::PROP_STATUS_OPEN,
        BandOpeningStatus::Marginal => key::PROP_STATUS_MARGINAL,
        BandOpeningStatus::Closed => key::PROP_STATUS_CLOSED,
    };
    tr(k, lang)
}

/// Mechanizm propagacji w języku interfejsu; nazwy warstw (F2, Es) są uniwersalne.
pub(crate) fn layer_label(layer: &str, lang: Language) -> String {
    match layer {
        crate::core::propagation::LAYER_GROUNDWAVE => {
            tr(key::PROP_LAYER_GROUNDWAVE, lang).to_string()
        }
        crate::core::propagation::LAYER_INVALID => tr(key::PROP_LAYER_INVALID, lang).to_string(),
        other => other.to_string(),
    }
}

/// Przyczyny wyniku prognozy jako lista punktów w języku interfejsu.
fn explanation_lines(e: &PropagationExplanation, lang: Language) -> Vec<String> {
    let mut out = vec![
        Msg::new(key::PROP_WHY_SFI)
            .arg("fof2", format!("{:.1}", e.fof2_mhz))
            .text(lang),
    ];
    out.push(if e.storm_penalty < 1.0 {
        Msg::new(key::PROP_WHY_STORM)
            .arg("pct", format!("{:.0}", (1.0 - e.storm_penalty) * 100.0))
            .text(lang)
    } else {
        tr(key::PROP_WHY_CALM, lang).to_string()
    });
    let lighting = if e.daylight {
        key::PROP_WHY_DAY
    } else {
        key::PROP_WHY_NIGHT
    };
    out.push(tr(lighting, lang).to_string());
    if e.absorption_db > 0.0 {
        out.push(
            Msg::new(key::PROP_WHY_ABSORPTION)
                .arg("db", format!("{:.0}", e.absorption_db))
                .text(lang),
        );
    }
    let reason = match e.reason_kind {
        ForecastReason::InvalidInput => key::PROP_REASON_INVALID,
        ForecastReason::AboveMuf => key::PROP_REASON_ABOVE_MUF,
        ForecastReason::BelowLuf => key::PROP_REASON_BELOW_LUF,
        ForecastReason::InWindow => key::PROP_REASON_IN_WINDOW,
        ForecastReason::Edge => key::PROP_REASON_EDGE,
        ForecastReason::Weak => key::PROP_REASON_WEAK,
    };
    out.push(tr(reason, lang).to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propagation_texts_follow_the_interface_language() {
        let a = crate::core::geo::Coordinates::new(51.1079, 17.0385);
        let b = crate::core::geo::Coordinates::new(40.7128, -74.0060);
        let (_, explanation) = PropagationEngine::explain(a, b, 14.175, 150, 2, 14.0, 100);
        let en = explanation_lines(&explanation, Language::En);
        assert!(en[0].starts_with("Solar activity (SFI): foF2"));
        assert_eq!(en[2], "Path lighting: day (D-layer absorption)");
        // Polska lista kończy się tym samym opisem przyczyny, który trafia do dziennika.
        let pl = explanation_lines(&explanation, Language::Pl);
        assert_eq!(pl.last(), Some(&explanation.reason));

        assert_eq!(
            band_status_label(BandOpeningStatus::Closed, Language::De),
            "Geschlossen"
        );
        assert_eq!(
            layer_label(crate::core::propagation::LAYER_GROUNDWAVE, Language::En),
            "Ground wave"
        );
        assert_eq!(layer_label("F2 (1-hop)", Language::Fr), "F2 (1-hop)");
    }

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
        let r = recommend(&forecasts, "20m", 1, Language::Pl);
        assert!(r.headline.contains("20m otwarte"));
        assert!(r.best_bands.iter().any(|b| b.starts_with("20m")));
    }

    #[test]
    fn recommends_switch_when_current_closed() {
        let forecasts = vec![
            ("10m", fc(BandOpeningStatus::Closed, 5)),
            ("40m", fc(BandOpeningStatus::Open, 70)),
        ];
        let r = recommend(&forecasts, "10m", 1, Language::Pl);
        assert!(r.headline.contains("Przełącz na 40m"));
        assert!(r.advice.iter().any(|a| a.contains("Aktualne pasmo słabe")));
    }

    #[test]
    fn warns_on_geomagnetic_storm() {
        let forecasts = vec![("20m", fc(BandOpeningStatus::Marginal, 40))];
        let r = recommend(&forecasts, "20m", 6, Language::Pl);
        assert!(r.advice.iter().any(|a| a.contains("silna burza")));
    }

    #[test]
    fn all_closed_fallback() {
        let forecasts = vec![("6m", fc(BandOpeningStatus::Closed, 0))];
        let r = recommend(&forecasts, "6m", 2, Language::Pl);
        assert!(r.headline.contains("Wszystkie pasma zamknięte"));
        assert_eq!(r.best_bands, [] as [String; 0]);
    }
}
