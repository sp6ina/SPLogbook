// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wbudowany dziennik zmian (changelog) i okno sprawdzania aktualizacji.

use crate::cloud::updater::UpdateDelivery;
use crate::core::i18n::{Key, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::theme::ThemeExt;
use eframe::egui;

/// Pojedynczy wpis dziennika zmian.
pub struct ChangeLogEntry {
    pub version: &'static str,
    pub date: &'static str,
    pub items: &'static [Key],
}

/// Historia wydań SPLogbook — trzymana w kodzie jako klucze i18n,
/// dzięki czemu jest w 100% zlokalizowana w każdym języku.
pub const CHANGELOG: &[ChangeLogEntry] = &[
    ChangeLogEntry {
        version: "1.4.0",
        date: "2026-11-27",
        items: &[
            key::CHANGELOG_V1_4_0,
            key::CHANGELOG_V1_4_1,
            key::CHANGELOG_V1_4_2,
            key::CHANGELOG_V1_4_3,
            key::CHANGELOG_V1_4_4,
            key::CHANGELOG_V1_4_5,
        ],
    },
    ChangeLogEntry {
        version: "1.1",
        date: "2026",
        items: &[
            key::CHANGELOG_V1_1_0,
            key::CHANGELOG_V1_1_1,
            key::CHANGELOG_V1_1_2,
            key::CHANGELOG_V1_1_3,
            key::CHANGELOG_V1_1_4,
            key::CHANGELOG_V1_1_5,
            key::CHANGELOG_V1_1_6,
            key::CHANGELOG_V1_1_7,
            key::CHANGELOG_V1_1_8,
        ],
    },
    ChangeLogEntry {
        version: "1.0.3",
        date: "2026",
        items: &[
            key::CHANGELOG_V1_0_3_0,
            key::CHANGELOG_V1_0_3_1,
            key::CHANGELOG_V1_0_3_2,
            key::CHANGELOG_V1_0_3_3,
            key::CHANGELOG_V1_0_3_4,
        ],
    },
    ChangeLogEntry {
        version: "1.0.2",
        date: "2025",
        items: &[
            key::CHANGELOG_V1_0_2_0,
            key::CHANGELOG_V1_0_2_1,
            key::CHANGELOG_V1_0_2_2,
        ],
    },
    ChangeLogEntry {
        version: "1.0.1",
        date: "2025",
        items: &[
            key::CHANGELOG_V1_0_1_0,
            key::CHANGELOG_V1_0_1_1,
            key::CHANGELOG_V1_0_1_2,
        ],
    },
    ChangeLogEntry {
        version: "1.0.0",
        date: "2025",
        items: &[key::CHANGELOG_V1_0_0_0, key::CHANGELOG_V1_0_0_1],
    },
];

/// Okno „Dziennik zmian” pokazujące różnice wersji wprost w aplikacji.
pub fn render_changelog_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.updates.show_changelog_window {
        return;
    }
    let mut is_open = app.updates.show_changelog_window;
    let lang = app.gui.appearance.language;
    egui::Window::new(tr(key::CHANGELOG_WINDOW_TITLE, lang))
        .open(&mut is_open)
        .default_size([520.0, 480.0])
        .resizable(true)
        .show(ctx, |ui| {
            let palette = ui.palette();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in CHANGELOG {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading(
                                    egui::RichText::new(format!("v{}", entry.version))
                                        .size(16.0)
                                        .strong()
                                        .color(palette.accent),
                                );
                                ui.label(
                                    egui::RichText::new(format!("({})", entry.date))
                                        .size(11.0)
                                        .color(palette.text_muted),
                                );
                            });
                            ui.add_space(2.0);
                            for item in entry.items {
                                ui.horizontal(|ui| {
                                    ui.colored_label(palette.success, "•");
                                    ui.label(tr(*item, lang));
                                });
                            }
                        });
                        ui.add_space(6.0);
                    }
                });
        });
    app.updates.show_changelog_window = is_open;
}

/// Wskazówka dla instalacji, których program nie aktualizuje sam.
fn delivery_hint(delivery: UpdateDelivery) -> Option<Key> {
    match delivery {
        UpdateDelivery::InApp => None,
        UpdateDelivery::Deb => Some(key::UPDATE_VIA_DEB),
        UpdateDelivery::Rpm => Some(key::UPDATE_VIA_RPM),
        UpdateDelivery::Flatpak => Some(key::UPDATE_VIA_FLATPAK),
        UpdateDelivery::AppImage => Some(key::UPDATE_VIA_APPIMAGE),
        UpdateDelivery::Manual => Some(key::UPDATE_VIA_MANUAL),
    }
}

/// Okno „Sprawdź aktualizacje” — porównuje wersję lokalną z najnowszym wydaniem GitHub.
pub fn render_update_check_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.updates.show_update_window {
        return;
    }
    let delivery = *app
        .updates
        .delivery
        .get_or_insert_with(crate::cloud::updater::current_update_delivery);
    let mut is_open = app.updates.show_update_window;
    let mut close_requested = false;
    let mut do_install = false;
    let lang = app.gui.appearance.language;
    egui::Window::new(tr(key::UPDATE_WINDOW_TITLE, lang))
        .open(&mut is_open)
        .default_size([520.0, 340.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr(key::UPDATE_INSTALLED_VERSION, lang));
                ui.monospace(env!("CARGO_PKG_VERSION"));
            });

            ui.separator();

            match &app.updates.update_check_status {
                None => {
                    ui.label(tr(key::UPDATE_PROMPT, lang));
                }
                Some(status) => {
                    ui.label(egui::RichText::new(status).size(13.0));
                }
            }

            if let Some(release) = &app.updates.update_available {
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .id_salt("update_release_body")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&release.body).size(12.0));
                    });
            }

            if app.updates.update_available.is_some()
                && let Some(hint) = delivery_hint(delivery)
            {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(tr(hint, lang)).size(13.0));
            }

            if let Some(inst) = &app.updates.update_install_status {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(inst).size(13.0));
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr(key::UPDATE_CHECK_NOW, lang)).clicked() {
                    app.trigger_update_check();
                }
                if app.updates.update_available.is_some()
                    && delivery == UpdateDelivery::InApp
                    && ui.button(tr(key::UPDATE_INSTALL_RESTART, lang)).clicked()
                {
                    do_install = true;
                }
                if ui.button(tr(key::UPDATE_RELEASES_PAGE, lang)).clicked() {
                    let _ = open::that("https://github.com/sp6ina/SPLogbook/releases");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_requested = true;
                    }
                });
            });
        });
    if do_install {
        app.trigger_update_install();
    }
    app.updates.show_update_window = is_open && !close_requested;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::i18n::Language;

    #[test]
    fn newest_changelog_entry_is_the_package_version() {
        assert_eq!(CHANGELOG[0].version, env!("CARGO_PKG_VERSION"));
        for &item in CHANGELOG[0].items {
            assert!(!tr(item, Language::En).is_empty(), "{item:?}");
        }
    }

    #[test]
    fn only_in_app_updates_go_without_a_hint() {
        assert_eq!(delivery_hint(UpdateDelivery::InApp), None);
        for delivery in [
            UpdateDelivery::Deb,
            UpdateDelivery::Rpm,
            UpdateDelivery::Flatpak,
            UpdateDelivery::AppImage,
            UpdateDelivery::Manual,
        ] {
            let hint = delivery_hint(delivery).expect("package installs need a hint");
            assert!(!tr(hint, Language::En).is_empty(), "{delivery:?}");
        }
    }
}
