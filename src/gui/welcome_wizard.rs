// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Wozniak (SP6INA)

use crate::core::i18n::{tr, Language};
use crate::gui::app::SpLogApp;
use eframe::egui;

/// Ekran powitalny / kreator pierwszego uruchomienia.
/// Wzorowany na: Log4OM, HamRadioDeluxe, QLog.
pub fn render_welcome_wizard(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_welcome_wizard { return; }
    let lang = app.current_language;
    let mut close_wizard = false;

    // Obsługa zamknięcia klawiszem Escape
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close_wizard = true;
    }

    // WAZNE:
    // 1. Stale ID okna oraz staly tytul okna (niezalezny od jezyka) eliminuja reset stanu okna w egui.
    // 2. .order(egui::Order::Foreground) gwarantuje, ze okno kreatora ZAWSZE pozostaje na wierzchu
    //    i nie chowa sie pod oknami modulow na pulpicie MDI.
    // 3. Bezposrednie przyciski wyboru jezyka [PL] [EN] [DE] [FR] [ES] [RU] w naglowku zamiast popupu ComboBox,
    //    dzieki czemu klikniecie nie wywoluje zdarzen zamkniecia popupu ani przesloniecia okna.
    egui::Window::new("SPLogbook — Setup Wizard")
        .id(egui::Id::new("splogbook_welcome_wizard"))
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(680.0)
        .min_size([580.0, 540.0])
        .show(ctx, |ui| {
            // --- Naglowek ---
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("SPLogbook")
                    .size(24.0).color(egui::Color32::from_rgb(56, 189, 248)));
                ui.label(egui::RichText::new("v1.0.3")
                    .size(13.0).color(egui::Color32::from_rgb(100, 116, 139)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Dedykowany przycisk [✕] w nagłówku
                    if ui.button(egui::RichText::new(" ✕ ").size(13.0).strong())
                        .on_hover_text(tr("btn.cancel", lang))
                        .clicked() {
                        close_wizard = true;
                    }

                    ui.add_space(8.0);

                    // Bezpośrednie przyciski wyboru języka z flagami (1-klik, brak popupu)
                    for (l, flag, name) in [
                        (Language::Ru, "🇷🇺", "RU"),
                        (Language::Es, "🇪🇸", "ES"),
                        (Language::Fr, "🇫🇷", "FR"),
                        (Language::De, "🇩🇪", "DE"),
                        (Language::En, "🇬🇧", "EN"),
                        (Language::Pl, "🇵🇱", "PL"),
                    ] {
                        let is_sel = lang == l;
                        let fill = if is_sel {
                            egui::Color32::from_rgb(30, 64, 100)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(45, 45, 55, 200)
                        };
                        let btn = egui::Button::new(
                            egui::RichText::new(format!("{} {}", flag, name))
                                .size(11.0)
                                .color(if is_sel { egui::Color32::WHITE } else { egui::Color32::from_rgb(180, 190, 205) })
                        ).fill(fill);

                        if ui.add(btn).clicked() {
                            app.current_language = l;
                        }
                    }

                    ui.label(egui::RichText::new(format!("{}:", tr("menu.language", lang))).size(11.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)));
                });
            });

            ui.label(egui::RichText::new(tr("wizard.welcome", lang))
                .size(13.0).color(egui::Color32::from_rgb(148, 163, 184)));
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // --- Zakladki kreatora (jak Log4OM) ---
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.wizard_tab, 0,
                    format!("1. {}", tr("wizard.tab.station", lang)));
                ui.selectable_value(&mut app.wizard_tab, 1,
                    format!("2. {}", tr("wizard.tab.radio", lang)));
                ui.selectable_value(&mut app.wizard_tab, 2,
                    format!("3. {}", tr("wizard.tab.services", lang)));
                ui.selectable_value(&mut app.wizard_tab, 3,
                    format!("4. {}", tr("wizard.tab.appearance", lang)));
            });
            ui.separator();
            ui.add_space(6.0);

            match app.wizard_tab {
                0 => render_tab_station(app, ui, lang),
                1 => render_tab_radio(app, ui, lang),
                2 => render_tab_services(app, ui, lang),
                _ => render_tab_appearance(app, ui, lang, ctx),
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);

            // --- Przyciski nawigacji ---
            ui.horizontal(|ui| {
                if app.wizard_tab > 0 {
                    if ui.button(format!("< {}", tr("btn.back", lang))).clicked() {
                        app.wizard_tab -= 1;
                    }
                }
                if app.wizard_tab < 3 {
                    if ui.button(
                        egui::RichText::new(format!("{} >", tr("btn.next", lang))).strong()
                    ).clicked() {
                        app.wizard_tab += 1;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr("btn.cancel", lang)).clicked() {
                        close_wizard = true;
                    }
                    if ui.button(
                        egui::RichText::new(format!("  {}  ", tr("wizard.save_and_start", lang)))
                            .size(13.0).strong().color(egui::Color32::WHITE)
                    ).clicked() {
                        app.show_welcome_wizard = false;
                        app.save_station_config();
                        close_wizard = true;
                        app.status_message = Some(format!(
                            "Stacja {} skonfigurowana. Miłej pracy!", app.my_station.callsign
                        ));
                    }
                });
            });
        });

    if close_wizard {
        app.show_welcome_wizard = false;
    }
}

// === ZAKLADKA 1: Dane stacji ================================================
fn render_tab_station(app: &mut SpLogApp, ui: &mut egui::Ui, lang: Language) {
    egui::Grid::new("wiz_station_grid")
        .num_columns(2).spacing([12.0, 8.0]).striped(true)
        .show(ui, |ui| {
            // Znak wywoławczy
            ui.label(egui::RichText::new(format!("(*) {}:", tr("qso.callsign", lang))).strong());
            let call = ui.add(
                egui::TextEdit::singleline(&mut app.my_station.callsign)
                    .hint_text("np. SP6INA, W1AW, DL1ABC")
                    .desired_width(260.0)
            );
            if call.changed() {
                app.my_station.callsign = app.my_station.callsign.to_uppercase();
            }
            ui.end_row();

            // Operator
            ui.label(egui::RichText::new(format!("{}:", tr("station.operator", lang))).strong());
            ui.add(egui::TextEdit::singleline(&mut app.my_station.operator)
                .hint_text("np. Jan Kowalski").desired_width(260.0));
            ui.end_row();

            // Lokator Maidenhead
            ui.label(egui::RichText::new(format!("(*) {}:", tr("geo.locator", lang))).strong());
            let loc = ui.add(
                egui::TextEdit::singleline(&mut app.my_station.gridsquare)
                    .hint_text("np. JO81WA, KO02AA")
                    .desired_width(260.0)
            );
            if loc.changed() {
                app.my_station.gridsquare = app.my_station.gridsquare.to_uppercase();
            }
            ui.end_row();

            // QTH
            ui.label(egui::RichText::new("QTH:").strong());
            ui.add(egui::TextEdit::singleline(&mut app.my_station.city)
                .hint_text("np. Wroclaw, Krakow").desired_width(260.0));
            ui.end_row();

            // Kraj / DXCC
            ui.label(egui::RichText::new(format!("{}:", tr("geo.dxcc", lang))).strong());
            ui.add(egui::TextEdit::singleline(&mut app.my_station.country)
                .hint_text("np. Poland, Germany").desired_width(260.0));
            ui.end_row();

            // Gmina PGA
            ui.label(egui::RichText::new(format!("{}:", tr("qso.pga", lang))).strong());
            let mut pga = app.my_station.pga_gmina.clone().unwrap_or_default();
            if ui.add(egui::TextEdit::singleline(&mut pga)
                .hint_text("np. WR01").desired_width(260.0)).changed() {
                let u = pga.to_uppercase();
                app.my_station.pga_gmina = if u.trim().is_empty() { None } else { Some(u) };
            }
            ui.end_row();

            // Strefy CQ / ITU
            ui.label(egui::RichText::new(
                format!("{} / {}:", tr("geo.cq_zone", lang), tr("geo.itu_zone", lang))
            ).strong());
            ui.horizontal(|ui| {
                ui.label("CQ:");
                ui.add(egui::DragValue::new(&mut app.my_station.cq_zone).range(1..=40));
                ui.label("ITU:");
                ui.add(egui::DragValue::new(&mut app.my_station.itu_zone).range(1..=90));
            });
            ui.end_row();
        });

    ui.add_space(6.0);
    ui.label(egui::RichText::new(tr("wizard.required_hint", lang))
        .small().color(egui::Color32::from_rgb(148, 163, 184)));
}

// === ZAKLADKA 2: Sprzet radiowy =============================================
fn render_tab_radio(app: &mut SpLogApp, ui: &mut egui::Ui, lang: Language) {
    egui::Grid::new("wiz_radio_grid")
        .num_columns(2).spacing([12.0, 8.0]).striped(true)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(format!("{}:", tr("wizard.transceiver", lang))).strong());
            let mut rig = app.my_station.default_rig.clone().unwrap_or_default();
            if ui.add(egui::TextEdit::singleline(&mut rig)
                .hint_text("np. Icom IC-7300, Kenwood TS-890S")
                .desired_width(260.0)).changed() {
                app.my_station.default_rig =
                    if rig.trim().is_empty() { None } else { Some(rig) };
            }
            ui.end_row();

            ui.label(egui::RichText::new(format!("{}:", tr("wizard.antenna", lang))).strong());
            let mut ant = app.my_station.default_antenna.clone().unwrap_or_default();
            if ui.add(egui::TextEdit::singleline(&mut ant)
                .hint_text("np. Dipol 80m, Yagi 6el 20m, GP 40m")
                .desired_width(260.0)).changed() {
                app.my_station.default_antenna =
                    if ant.trim().is_empty() { None } else { Some(ant) };
            }
            ui.end_row();

            ui.label(egui::RichText::new(format!("{}:", tr("wizard.tx_power", lang))).strong());
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut app.my_station.power_watts)
                    .range(1..=1500).suffix(" W"));
            });
            ui.end_row();

            ui.label(egui::RichText::new("CAT / Hamlib (rigctld):").strong());
            ui.horizontal(|ui| {
                ui.label("Host:");
                ui.add(egui::TextEdit::singleline(&mut app.cat_host)
                    .hint_text("127.0.0.1").desired_width(120.0));
                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut app.cat_port).range(1..=65535));
            });
            ui.end_row();

            ui.label(egui::RichText::new("JS8Call:").strong());
            ui.horizontal(|ui| {
                ui.checkbox(&mut app.js8call_enabled, tr("wizard.enable_js8call", lang));
                ui.label(egui::RichText::new("(port 2237)")
                    .color(egui::Color32::from_rgb(148, 163, 184)).small());
            });
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new(
        tr("wizard.cat_hint", lang)
    ).small().color(egui::Color32::from_rgb(148, 163, 184)));
}

// === ZAKLADKA 3: Serwisy online =============================================
fn render_tab_services(app: &mut SpLogApp, ui: &mut egui::Ui, lang: Language) {
    ui.label(egui::RichText::new(
        tr("wizard.services_sub", lang)
    ).strong());
    ui.add_space(8.0);

    egui::Grid::new("wiz_services_grid")
        .num_columns(2).spacing([12.0, 8.0]).striped(true)
        .show(ui, |ui| {
            // LoTW
            ui.label(egui::RichText::new("LoTW (ARRL):").strong());
            ui.horizontal(|ui| {
                ui.label("Login:");
                ui.add(egui::TextEdit::singleline(&mut app.lotw_username)
                    .hint_text("callsign LoTW").desired_width(100.0));
                ui.label("Haslo:");
                ui.add(egui::TextEdit::singleline(&mut app.lotw_password)
                    .password(true).desired_width(100.0));
            });
            ui.end_row();

            // QRZ.com
            ui.label(egui::RichText::new("QRZ.com XML API:").strong());
            ui.add(egui::TextEdit::singleline(&mut app.qrz_api_key)
                .hint_text("Klucz API XML ze strony QRZ.com").desired_width(260.0));
            ui.end_row();

            // Club Log
            ui.label(egui::RichText::new("Club Log:").strong());
            ui.horizontal(|ui| {
                ui.label("Email:");
                ui.add(egui::TextEdit::singleline(&mut app.clublog_email)
                    .desired_width(150.0));
                ui.label("API:");
                ui.add(egui::TextEdit::singleline(&mut app.clublog_api_key)
                    .desired_width(100.0));
            });
            ui.end_row();

            // REST API
            ui.label(egui::RichText::new("Lokalny REST API:").strong());
            ui.horizontal(|ui| {
                ui.checkbox(&mut app.rest_api_enabled, tr("wizard.enable_http", lang));
                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut app.rest_api_port)
                    .range(1024..=65535));
            });
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(egui::RichText::new(
        tr("wizard.services_hint", lang)
    ).small().color(egui::Color32::from_rgb(148, 163, 184)));
}

// === ZAKLADKA 4: Wyglad =====================================================
fn render_tab_appearance(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    lang: Language,
    ctx: &egui::Context,
) {
    // --- Jezyk ---
    ui.label(egui::RichText::new(format!("{} / Interface language:", tr("menu.language", lang))).strong());
    ui.add_space(4.0);

    ui.horizontal_wrapped(|ui| {
        for (l, flag, name) in [
            (Language::Pl, "PL", "Polski"),
            (Language::En, "EN", "English"),
            (Language::De, "DE", "Deutsch"),
            (Language::Fr, "FR", "Francais"),
            (Language::Es, "ES", "Espanol"),
            (Language::Ru, "RU", "Russkiy"),
        ] {
            let selected = lang == l;
            let fill = if selected {
                egui::Color32::from_rgb(30, 64, 100)
            } else {
                egui::Color32::from_rgba_unmultiplied(40, 40, 40, 200)
            };
            let btn = egui::Button::new(
                egui::RichText::new(format!("[{}] {}", flag, name)).size(12.0)
            ).fill(fill);
            if ui.add_sized([115.0, 34.0], btn).clicked() {
                app.current_language = l;
            }
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);

    // --- Motyw ---
    ui.label(egui::RichText::new(tr("wizard.theme_label", lang)).strong());
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.add_sized([140.0, 30.0],
            egui::Button::new(egui::RichText::new(tr("wizard.theme_dark", lang)).size(12.0))
                .fill(egui::Color32::from_rgb(30, 30, 50))
        ).clicked() {
            ctx.set_visuals(egui::Visuals::dark());
            app.dark_theme = true;
            app.save_station_config();
        }
        if ui.add_sized([140.0, 30.0],
            egui::Button::new(egui::RichText::new(tr("wizard.theme_light", lang)).size(12.0))
                .fill(egui::Color32::from_rgb(220, 220, 230))
        ).clicked() {
            ctx.set_visuals(egui::Visuals::light());
            app.dark_theme = false;
            app.save_station_config();
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);

    // --- Pasek szybkiego dostepu ---
    ui.label(egui::RichText::new(tr("wizard.quick_access_label", lang)).strong());
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        let qa = &mut app.quick_access;
        ui.checkbox(&mut qa.show_vfo,        tr("tab.vfo", lang));
        ui.checkbox(&mut qa.show_log,        tr("tab.logbook", lang));
        ui.checkbox(&mut qa.show_cluster,    tr("tab.dx_cluster", lang));
        ui.checkbox(&mut qa.show_map,        tr("tab.map", lang));
        ui.checkbox(&mut qa.show_awards,     tr("tab.awards", lang));
        ui.checkbox(&mut qa.show_contest,    tr("toolbar.contest", lang));
        ui.checkbox(&mut qa.show_solar,      tr("tab.solar", lang));
        ui.checkbox(&mut qa.show_satellites, tr("toolbar.satellites", lang));
        ui.checkbox(&mut qa.show_bandmap,    tr("tab.band_map", lang));
        ui.checkbox(&mut qa.show_lotw,       "LoTW");
        ui.checkbox(&mut qa.show_cw,         "CW");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_language_switch_keeps_window_open() {
        let ctx = egui::Context::default();
        let mut show_wizard = true;
        let mut selected_lang = Language::Pl;

        for &target_lang in &[
            Language::En,
            Language::De,
            Language::Fr,
            Language::Es,
            Language::Ru,
            Language::Pl,
        ] {
            let close_wizard = false;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::Window::new("SPLogbook — Setup Wizard")
                    .id(egui::Id::new("splogbook_welcome_wizard"))
                    .order(egui::Order::Foreground)
                    .collapsible(false)
                    .show(ctx, |_ui| {
                        selected_lang = target_lang;
                    });
            });

            if close_wizard {
                show_wizard = false;
            }

            assert!(show_wizard, "Kreator nie powinien się zamykać przy zmianie na {:?}", target_lang);
            assert_eq!(selected_lang, target_lang);
        }
    }

    #[test]
    fn test_wizard_escape_closes_window() {
        let ctx = egui::Context::default();
        let mut show_wizard = true;

        let mut input = egui::RawInput::default();
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });

        let mut close_wizard = false;
        let _ = ctx.run(input, |ctx| {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                close_wizard = true;
            }
            egui::Window::new("SPLogbook — Setup Wizard")
                .id(egui::Id::new("splogbook_welcome_wizard"))
                .order(egui::Order::Foreground)
                .show(ctx, |_ui| {});
        });

        if close_wizard {
            show_wizard = false;
        }

        assert!(!show_wizard, "Naciśnięcie klawisza Escape powinno zamknąć kreator");
    }
}