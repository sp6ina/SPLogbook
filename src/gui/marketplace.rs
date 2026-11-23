// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno „Marketplace pluginów”: katalog dodatków z instalacją jednym kliknięciem.
//!
//! Katalog pobierany jest asynchronicznie (zdalny `catalog.json`, z wbudowanym
//! fallbackiem offline). Instalacja zapisuje skrypt `.rhai` do katalogu pluginów
//! użytkownika z weryfikacją SHA256 i przeładowuje silnik Rhai. Wszystkie skrypty
//! działają w piaskownicy (bez dostępu do systemu plików, sieci i procesów).

use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;
use std::path::{Path, PathBuf};

/// Akcja wybrana w oknie marketplace (wykonywana po zamknięciu pętli UI).
enum Action {
    Install(String),
    Uninstall(String),
    Refresh,
}

/// Rysuje okno marketplace, jeśli jest otwarte. Obsługuje pobieranie katalogu
/// oraz instalację/odinstalowanie wtyczek w tle.
pub fn render_marketplace(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.marketplace.show_window {
        return;
    }

    // Odbiór wyników asynchronicznego pobierania katalogu
    if let Some(rx) = &app.marketplace.fetch_rx
        && let Ok(result) = rx.try_recv()
    {
        app.marketplace.loading = false;
        app.marketplace.fetch_rx = None;
        match result {
            Ok(entries) => {
                app.marketplace.catalog = entries;
                app.marketplace.error = None;
            }
            Err(e) => app.marketplace.error = Some(e.to_string()),
        }
    }

    // Odbiór wyniku asynchronicznej instalacji
    if let Some(rx) = &app.marketplace.install_rx
        && let Ok((id, result)) = rx.try_recv()
    {
        app.marketplace.busy_id = None;
        app.marketplace.install_rx = None;
        match result {
            Ok(()) => {
                app.marketplace.status = Some(
                    Msg::new(key::MARKET_INSTALLED_SUCCESS)
                        .arg("id", &id)
                        .text(app.gui.appearance.language),
                );
                reload_plugins(app);
            }
            Err(e) => app.marketplace.status = Some(e.to_string()),
        }
    }

    // Pierwsze otwarcie: pobierz katalog, jeśli jeszcze go nie ma.
    if app.marketplace.catalog.is_empty() && !app.marketplace.loading {
        start_fetch(app);
    }

    let plugins_dir = PathBuf::from(app.config.plugins_dir.clone());
    let catalog = app.marketplace.catalog.clone();
    let mut action: Option<Action> = None;
    let mut open = app.marketplace.show_window;
    let lang = app.gui.appearance.language;

    egui::Window::new(tr(key::MARKET_WINDOW_TITLE, lang))
        .open(&mut open)
        .default_size([680.0, 520.0])
        .resizable(true)
        .show(ctx, |ui| {
            let palette = ui.palette();
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(tr(key::MARKET_WINDOW_TITLE, lang))
                        .size(16.0)
                        .color(palette.accent),
                );
                if ui.button(tr(key::MARKET_REFRESH, lang)).clicked() {
                    action = Some(Action::Refresh);
                }
            });

            ui.label(
                egui::RichText::new(tr(key::MARKET_DESCRIPTION, lang))
                    .small()
                    .color(palette.text_muted),
            );
            ui.add_space(6.0);

            // Pasek wyszukiwania i filtru kategorii
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.add(
                    egui::TextEdit::singleline(&mut app.marketplace.search)
                        .hint_text(tr(key::MARKET_SEARCH_HINT, lang))
                        .desired_width(260.0),
                );
                ui.separator();

                let mut selected = app.marketplace.category.clone();
                let mut categories: Vec<String> =
                    catalog.iter().map(|e| e.category.clone()).collect();
                categories.sort();
                categories.dedup();
                let all_label = tr(key::MARKET_ALL_CATEGORIES, lang);
                egui::ComboBox::from_id_salt("marketplace_category")
                    .selected_text(selected.as_deref().unwrap_or(all_label))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected, None, all_label);
                        for cat in &categories {
                            ui.selectable_value(&mut selected, Some(cat.clone()), cat.as_str());
                        }
                    });
                app.marketplace.category = selected;
            });
            ui.add_space(4.0);

            if app.marketplace.loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr(key::MARKET_FETCHING, lang));
                });
            } else if let Some(err) = app.marketplace.error.clone() {
                ui.colored_label(
                    palette.danger,
                    Msg::new(key::MARKET_FETCH_ERROR).arg("err", err).text(lang),
                );
                if ui.button(tr(key::MARKET_OFFLINE_FALLBACK, lang)).clicked() {
                    app.marketplace.catalog = crate::plugins::marketplace::bundled_catalog();
                    app.marketplace.error = None;
                }
            } else {
                ui.label(
                    egui::RichText::new(
                        Msg::new(key::MARKET_AVAILABLE_COUNT)
                            .arg("count", catalog.len())
                            .text(lang),
                    )
                    .strong()
                    .color(palette.success),
                );

                if let Some(status) = app.marketplace.status.clone() {
                    ui.colored_label(palette.warning, status);
                }
                ui.add_space(4.0);

                let query = app.marketplace.search.to_lowercase();
                let filtered: Vec<&crate::plugins::marketplace::PluginCatalogEntry> = catalog
                    .iter()
                    .filter(|e| {
                        let matches_cat = app
                            .marketplace
                            .category
                            .as_ref()
                            .is_none_or(|c| &e.category == c);
                        let hay = format!("{} {} {}", e.name, e.id, catalog_description(e, lang))
                            .to_lowercase();
                        let matches_q = query.is_empty() || hay.contains(&query);
                        matches_cat && matches_q
                    })
                    .collect();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.label(
                                egui::RichText::new(tr(key::MARKET_NO_MATCHES, lang))
                                    .italics()
                                    .color(palette.text_muted),
                            );
                        }
                        for entry in filtered {
                            render_card(ui, app, &plugins_dir, entry, &mut action);
                            ui.add_space(6.0);
                        }
                    });
            }
        });

    app.marketplace.show_window = open;

    // Wykonanie akcji po zamknięciu pętli UI
    match action {
        Some(Action::Refresh) => start_fetch(app),
        Some(Action::Install(id)) => {
            if let Some(entry) = catalog.iter().find(|e| e.id == id) {
                start_install(app, entry.clone());
            }
        }
        Some(Action::Uninstall(id)) => {
            if let Some(entry) = catalog.iter().find(|e| e.id == id) {
                let dir = PathBuf::from(app.config.plugins_dir.clone());
                match crate::plugins::marketplace::uninstall_entry(&dir, entry) {
                    Ok(()) => {
                        app.marketplace.status = Some(
                            Msg::new(key::MARKET_UNINSTALLED)
                                .arg("name", &entry.name)
                                .text(lang),
                        );
                        reload_plugins(app);
                    }
                    Err(e) => app.marketplace.status = Some(e.to_string()),
                }
            }
        }
        None => {}
    }
}

/// Rysuje kartę pojedynczej wtyczki.
fn render_card(
    ui: &mut egui::Ui,
    app: &mut SpLogApp,
    plugins_dir: &Path,
    entry: &crate::plugins::marketplace::PluginCatalogEntry,
    action: &mut Option<Action>,
) {
    let status = crate::plugins::marketplace::install_status(plugins_dir, entry);
    let busy = app.marketplace.busy_id.as_deref() == Some(entry.id.as_str());
    let lang = app.gui.appearance.language;

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            let palette = ui.palette();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&entry.icon).size(22.0));
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&entry.name).strong().size(14.0));
                        ui.label(
                            egui::RichText::new(format!("v{}", entry.version))
                                .small()
                                .color(palette.text_muted),
                        );
                        ui.label(
                            egui::RichText::new(&entry.category)
                                .small()
                                .background_color(palette.subtle_bg)
                                .color(palette.text_muted),
                        );
                    });
                    ui.label(
                        egui::RichText::new(catalog_description(entry, lang))
                            .small()
                            .color(palette.text_primary),
                    );
                    ui.label(
                        egui::RichText::new(
                            crate::core::i18n::Msg::new(key::MARKET_AUTHOR)
                                .arg("name", &entry.author)
                                .text(lang),
                        )
                        .small()
                        .color(palette.text_subtle),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if busy {
                        ui.spinner();
                        ui.label("…");
                    } else {
                        match status {
                            crate::plugins::marketplace::InstallStatus::NotInstalled => {
                                if ui.button(tr(key::MARKET_INSTALL_BTN, lang)).clicked() {
                                    *action = Some(Action::Install(entry.id.clone()));
                                }
                            }
                            crate::plugins::marketplace::InstallStatus::Installed { .. } => {
                                ui.colored_label(
                                    palette.success,
                                    tr(key::MARKET_INSTALLED_LBL, lang),
                                );
                                if ui
                                    .small_button(tr(key::MARKET_UNINSTALL_BTN, lang))
                                    .clicked()
                                {
                                    *action = Some(Action::Uninstall(entry.id.clone()));
                                }
                            }
                            crate::plugins::marketplace::InstallStatus::UpdateAvailable {
                                installed,
                                latest,
                            } => {
                                ui.label(
                                    egui::RichText::new(format!("{installed} → {latest}"))
                                        .small()
                                        .color(palette.warning),
                                );
                                if ui.button(tr(key::MARKET_UPDATE_BTN, lang)).clicked() {
                                    *action = Some(Action::Install(entry.id.clone()));
                                }
                            }
                        }
                    }
                });
            });
        });
}

/// Rozpoczyna pobieranie katalogu w tle.
fn start_fetch(app: &mut SpLogApp) {
    if app.marketplace.loading {
        return;
    }
    app.marketplace.loading = true;
    app.marketplace.error = None;
    let (tx, rx) = std::sync::mpsc::channel();
    app.marketplace.fetch_rx = Some(rx);
    tokio::spawn(async move {
        let result = crate::plugins::marketplace::fetch_catalog().await;
        let _ = tx.send(result);
    });
}

/// Opis wtyczki w języku interfejsu; wtyczki spoza katalogu wbudowanego zachowują opis
/// z katalogu zdalnego.
pub fn catalog_description(
    entry: &crate::plugins::marketplace::PluginCatalogEntry,
    lang: crate::core::i18n::Language,
) -> String {
    let k = match entry.id.as_str() {
        "pota-helper" => Some(key::MARKET_DESC_POTA_HELPER),
        "sota-helper" => Some(key::MARKET_DESC_SOTA_HELPER),
        "cw-macros" => Some(key::MARKET_DESC_CW_MACROS),
        "contest-assistant" => Some(key::MARKET_DESC_CONTEST_ASSISTANT),
        "rotor-assistant" => Some(key::MARKET_DESC_ROTOR_ASSISTANT),
        "award-tracker" => Some(key::MARKET_DESC_AWARD_TRACKER),
        "dx-spot-alerts" => Some(key::MARKET_DESC_DX_SPOT_ALERTS),
        "propagation-watchdog" => Some(key::MARKET_DESC_PROPAGATION_WATCHDOG),
        "band-activity-logger" => Some(key::MARKET_DESC_BAND_ACTIVITY_LOGGER),
        "qsl-reminder" => Some(key::MARKET_DESC_QSL_REMINDER),
        "voice-keyer-trigger" => Some(key::MARKET_DESC_VOICE_KEYER_TRIGGER),
        "ft8-wsjt-bridge" => Some(key::MARKET_DESC_FT8_WSJT_BRIDGE),
        "wsjtx-new-dxcc" => Some(key::MARKET_DESC_WSJTX_NEW_DXCC),
        "qso-rate-goal" => Some(key::MARKET_DESC_QSO_RATE_GOAL),
        "band-page-switcher" => Some(key::MARKET_DESC_BAND_PAGE_SWITCHER),
        "contest-rate" => Some(key::MARKET_DESC_CONTEST_RATE),
        _ => None,
    };
    k.map_or_else(|| entry.description.clone(), |k| tr(k, lang).to_string())
}

/// Rozpoczyna instalację wtyczki w tle.
fn start_install(app: &mut SpLogApp, entry: crate::plugins::marketplace::PluginCatalogEntry) {
    if app.marketplace.busy_id.is_some() {
        return;
    }
    app.marketplace.busy_id = Some(entry.id.clone());
    app.marketplace.status = Some(
        crate::core::i18n::Msg::new(key::MARKET_INSTALLING)
            .arg("name", &entry.name)
            .text(app.gui.appearance.language),
    );
    let dir = PathBuf::from(app.config.plugins_dir.clone());
    let (tx, rx) = std::sync::mpsc::channel();
    app.marketplace.install_rx = Some(rx);
    tokio::spawn(async move {
        let result = crate::plugins::marketplace::install_entry(&entry, &dir)
            .await
            .map(|_| ());
        let _ = tx.send((entry.id.clone(), result));
    });
}

/// Przeładowuje silnik pluginów po zmianie zestawu skryptów.
fn reload_plugins(app: &mut SpLogApp) {
    let dir = PathBuf::from(app.config.plugins_dir.clone());
    app.services
        .plugin_engine
        .set_enabled(app.config.plugins_enabled);
    app.services.plugin_engine.load_dir(&dir);
    app.services.plugin_engine.run_startup();
    app.save_station_config();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::i18n::Language;

    #[test]
    fn every_bundled_plugin_has_a_translated_description() {
        for entry in crate::plugins::marketplace::bundled_catalog() {
            let mut probe = entry.clone();
            probe.description = "untranslated".to_string();
            for lang in [Language::Pl, Language::En, Language::De, Language::Ru] {
                let text = catalog_description(&probe, lang);
                assert_ne!(text, "untranslated", "{}: brak tłumaczenia opisu", entry.id);
                assert_ne!(text, "", "{}", entry.id);
            }
        }
        let mut remote = crate::plugins::marketplace::bundled_catalog().remove(0);
        remote.id = "remote-only".to_string();
        remote.description = "From the remote catalog".to_string();
        assert_eq!(
            catalog_description(&remote, Language::Pl),
            "From the remote catalog"
        );
    }
}
