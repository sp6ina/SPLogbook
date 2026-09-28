// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Okno „Marketplace pluginów”: katalog dodatków z instalacją jednym kliknięciem.
//!
//! Katalog pobierany jest asynchronicznie (zdalny `catalog.json`, z wbudowanym
//! fallbackiem offline). Instalacja zapisuje skrypt `.rhai` do katalogu pluginów
//! użytkownika z weryfikacją SHA256 i przeładowuje silnik Rhai. Wszystkie skrypty
//! działają w piaskownicy (bez dostępu do systemu plików, sieci i procesów).

use crate::gui::app::SpLogApp;
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
    if !app.show_marketplace {
        return;
    }

    // ——— Odbiór wyników asynchronicznego pobierania katalogu ———
    if let Some(rx) = &app.marketplace_fetch_rx {
        if let Ok(result) = rx.try_recv() {
            app.marketplace_loading = false;
            app.marketplace_fetch_rx = None;
            match result {
                Ok(entries) => {
                    app.marketplace_catalog = entries;
                    app.marketplace_error = None;
                }
                Err(e) => app.marketplace_error = Some(e),
            }
        }
    }

    // ——— Odbiór wyniku asynchronicznej instalacji ———
    if let Some(rx) = &app.marketplace_install_rx {
        if let Ok((id, result)) = rx.try_recv() {
            app.marketplace_busy_id = None;
            app.marketplace_install_rx = None;
            match result {
                Ok(()) => {
                    app.marketplace_status = Some(format!(
                        "Zainstalowano wtyczkę „{id}” — silnik przeładowany."
                    ));
                    reload_plugins(app);
                }
                Err(e) => app.marketplace_status = Some(e),
            }
        }
    }

    // Pierwsze otwarcie: pobierz katalog, jeśli jeszcze go nie ma.
    if app.marketplace_catalog.is_empty() && !app.marketplace_loading {
        start_fetch(app);
    }

    let plugins_dir = PathBuf::from(app.plugins_dir.clone());
    let catalog = app.marketplace_catalog.clone();
    let mut action: Option<Action> = None;
    let mut open = app.show_marketplace;

    egui::Window::new("🛒 Marketplace pluginów")
        .open(&mut open)
        .default_size([680.0, 520.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("🛒 Marketplace pluginów")
                        .size(16.0)
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );
                if ui.button("🔄 Odśwież katalog").clicked() {
                    action = Some(Action::Refresh);
                }
            });

            ui.label(
                egui::RichText::new(
                    "Katalog dodatków z instalacją jednym kliknięciem. Skrypty działają w piaskownicy Rhai — bez dostępu do plików, sieci i procesów.",
                )
                .small()
                .color(egui::Color32::GRAY),
            );
            ui.add_space(6.0);

            // ——— Pasek wyszukiwania i filtru kategorii ———
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.add(
                    egui::TextEdit::singleline(&mut app.marketplace_search)
                        .hint_text("Szukaj po nazwie lub opisie…")
                        .desired_width(260.0),
                );
                ui.separator();

                let mut selected = app
                    .marketplace_category
                    .clone()
                    .unwrap_or_else(|| "Wszystkie".to_string());
                let mut categories: Vec<String> = catalog
                    .iter()
                    .map(|e| e.category.clone())
                    .collect();
                categories.sort();
                categories.dedup();
                let mut all = vec!["Wszystkie".to_string()];
                all.extend(categories);

                egui::ComboBox::from_id_salt("marketplace_category")
                    .selected_text(&selected)
                    .show_ui(ui, |ui| {
                        for cat in &all {
                            ui.selectable_value(&mut selected, cat.clone(), cat.as_str());
                        }
                    });
                app.marketplace_category = if selected == "Wszystkie" {
                    None
                } else {
                    Some(selected)
                };
            });
            ui.add_space(4.0);

            if app.marketplace_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Pobieranie katalogu…");
                });
            } else if let Some(err) = app.marketplace_error.clone() {
                ui.colored_label(
                    egui::Color32::from_rgb(248, 113, 113),
                    format!("Błąd pobierania katalogu: {err}"),
                );
                if ui.button("Użyj wbudowanego katalogu offline").clicked() {
                    app.marketplace_catalog = crate::plugins::marketplace::bundled_catalog();
                    app.marketplace_error = None;
                }
            } else {
                ui.label(
                    egui::RichText::new(format!("Dostępnych wtyczek: {}", catalog.len()))
                        .strong()
                        .color(egui::Color32::from_rgb(134, 239, 172)),
                );

                if let Some(status) = app.marketplace_status.clone() {
                    ui.colored_label(
                        egui::Color32::from_rgb(250, 204, 21),
                        status,
                    );
                }
                ui.add_space(4.0);

                let query = app.marketplace_search.to_lowercase();
                let filtered: Vec<&crate::plugins::marketplace::PluginCatalogEntry> = catalog
                    .iter()
                    .filter(|e| {
                        let matches_cat = app
                            .marketplace_category
                            .as_ref()
                            .is_none_or(|c| &e.category == c);
                        let hay = format!("{} {} {}", e.name, e.id, e.description).to_lowercase();
                        let matches_q = query.is_empty() || hay.contains(&query);
                        matches_cat && matches_q
                    })
                    .collect();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.label(
                                egui::RichText::new("Brak wtyczek spełniających kryteria.")
                                    .italics()
                                    .color(egui::Color32::from_rgb(148, 163, 184)),
                            );
                        }
                        for entry in filtered {
                            render_card(ui, app, &plugins_dir, entry, &mut action);
                            ui.add_space(6.0);
                        }
                    });
            }
        });

    app.show_marketplace = open;

    // ——— Wykonanie akcji po zamknięciu pętli UI ———
    match action {
        Some(Action::Refresh) => start_fetch(app),
        Some(Action::Install(id)) => {
            if let Some(entry) = catalog.iter().find(|e| e.id == id) {
                start_install(app, entry.clone());
            }
        }
        Some(Action::Uninstall(id)) => {
            if let Some(entry) = catalog.iter().find(|e| e.id == id) {
                let dir = PathBuf::from(app.plugins_dir.clone());
                match crate::plugins::marketplace::uninstall_entry(&dir, entry) {
                    Ok(()) => {
                        app.marketplace_status =
                            Some(format!("Odinstalowano wtyczkę „{}”.", entry.name));
                        reload_plugins(app);
                    }
                    Err(e) => app.marketplace_status = Some(e),
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
    let busy = app.marketplace_busy_id.as_deref() == Some(entry.id.as_str());

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&entry.icon).size(22.0));
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&entry.name).strong().size(14.0));
                        ui.label(
                            egui::RichText::new(format!("v{}", entry.version))
                                .small()
                                .color(egui::Color32::from_rgb(148, 163, 184)),
                        );
                        ui.label(
                            egui::RichText::new(&entry.category)
                                .small()
                                .background_color(egui::Color32::from_rgb(30, 41, 59))
                                .color(egui::Color32::from_rgb(148, 163, 184)),
                        );
                    });
                    ui.label(
                        egui::RichText::new(&entry.description)
                            .small()
                            .color(egui::Color32::from_rgb(203, 213, 225)),
                    );
                    ui.label(
                        egui::RichText::new(format!("Autor: {}", entry.author))
                            .small()
                            .color(egui::Color32::from_rgb(100, 116, 139)),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if busy {
                        ui.spinner();
                        ui.label("…");
                    } else {
                        match status {
                            crate::plugins::marketplace::InstallStatus::NotInstalled => {
                                if ui.button("🔽 Instaluj").clicked() {
                                    *action = Some(Action::Install(entry.id.clone()));
                                }
                            }
                            crate::plugins::marketplace::InstallStatus::Installed { .. } => {
                                ui.colored_label(
                                    egui::Color32::from_rgb(34, 197, 94),
                                    "✅ Zainstalowano",
                                );
                                if ui.small_button("🗑 Odinstaluj").clicked() {
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
                                        .color(egui::Color32::from_rgb(250, 204, 21)),
                                );
                                if ui.button("⬆ Aktualizuj").clicked() {
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
    if app.marketplace_loading {
        return;
    }
    app.marketplace_loading = true;
    app.marketplace_error = None;
    let (tx, rx) = std::sync::mpsc::channel();
    app.marketplace_fetch_rx = Some(rx);
    tokio::spawn(async move {
        let result = crate::plugins::marketplace::fetch_catalog().await;
        let _ = tx.send(result);
    });
}

/// Rozpoczyna instalację wtyczki w tle.
fn start_install(app: &mut SpLogApp, entry: crate::plugins::marketplace::PluginCatalogEntry) {
    if app.marketplace_busy_id.is_some() {
        return;
    }
    app.marketplace_busy_id = Some(entry.id.clone());
    app.marketplace_status = Some(format!("Instalowanie „{}”…", entry.name));
    let dir = PathBuf::from(app.plugins_dir.clone());
    let (tx, rx) = std::sync::mpsc::channel();
    app.marketplace_install_rx = Some(rx);
    tokio::spawn(async move {
        let result = crate::plugins::marketplace::install_entry(&entry, &dir)
            .await
            .map(|_| ());
        let _ = tx.send((entry.id.clone(), result));
    });
}

/// Przeładowuje silnik pluginów po zmianie zestawu skryptów.
fn reload_plugins(app: &mut SpLogApp) {
    let dir = PathBuf::from(app.plugins_dir.clone());
    app.plugin_engine.set_enabled(app.plugins_enabled);
    app.plugin_engine.load_dir(&dir);
    app.plugin_engine.run_startup();
    app.save_station_config();
}
