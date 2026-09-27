// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wbudowany dziennik zmian (changelog) i okno sprawdzania aktualizacji.

use crate::gui::app::SpLogApp;
use eframe::egui;

/// Pojedynczy wpis dziennika zmian.
pub struct ChangeLogEntry {
    pub version: &'static str,
    pub date: &'static str,
    pub items: &'static [&'static str],
}

/// Historia wydań SPLogbook — trzymana w kodzie, aby była dostępna offline
/// i nie wymagała przebudowy dla drobnych poprawek tekstu.
pub const CHANGELOG: &[ChangeLogEntry] = &[
    ChangeLogEntry {
        version: "1.0.3",
        date: "2026",
        items: &[
            "Dodano wyszukiwarkę poleceń (Command Palette) pod Ctrl+Shift+P.",
            "Przeprojektowano motywy kolorystyczne z paletą przyjazną dla daltonistów (Okabe-Ito).",
            "Dodano pasek operacyjny, Mini HUD i wybór jednostek odległości (km/mile/NM).",
            "Reorganizacja menu, skalowanie czcionek, legenda statusów i czytelne komunikaty błędów.",
            "Cache prefiksów DXCC i wyników propagacji dla płynniejszego działania.",
        ],
    },
    ChangeLogEntry {
        version: "1.0.2",
        date: "2025",
        items: &[
            "Współdzielony klient HTTP z ponawianiem i backoffem przy błędach sieci.",
            "Walidacja obliczeń propagacji (VOACAP-lite) i integralności dziennika.",
            "Raportowanie błędów importu/eksportu ADIF w interfejsie.",
        ],
    },
    ChangeLogEntry {
        version: "1.0.1",
        date: "2025",
        items: &[
            "Naprawiono błędy krytyczne (P0) związane ze stabilnością i utratą danych.",
            "Przechowywanie poświadczeń w systemowym magazynie (keyring) zamiast w pliku konfiguracji.",
            "Stabilny, chronologiczny numer QSO niezależny od sortowania i filtrowania.",
        ],
    },
    ChangeLogEntry {
        version: "1.0.0",
        date: "2025",
        items: &[
            "Pierwsze wydanie SPLogbook — nowoczesny dziennik krótkofalarski.",
            "Integracja z Hamlib, TCI, WSJT-X, QRZ, HamQTH, eQSL, LoTW i Club Log.",
        ],
    },
];

/// Okno „Dziennik zmian” pokazujące różnice wersji wprost w aplikacji.
pub fn render_changelog_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_changelog_window {
        return;
    }
    let mut is_open = app.show_changelog_window;
    egui::Window::new("📜 Dziennik zmian (Changelog)")
        .open(&mut is_open)
        .default_size([520.0, 480.0])
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for entry in CHANGELOG {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(
                                egui::RichText::new(format!("v{}", entry.version))
                                    .size(16.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(56, 189, 248)),
                            );
                            ui.label(
                                egui::RichText::new(format!("({})", entry.date))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(148, 163, 184)),
                            );
                        });
                        ui.add_space(2.0);
                        for item in entry.items {
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "•");
                                ui.label(*item);
                            });
                        }
                    });
                    ui.add_space(6.0);
                }
            });
        });
    app.show_changelog_window = is_open;
}

/// Okno „Sprawdź aktualizacje” — porównuje wersję lokalną z najnowszym wydaniem GitHub.
pub fn render_update_check_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_update_window {
        return;
    }
    let mut is_open = app.show_update_window;
    let mut close_requested = false;
    egui::Window::new("🔄 Sprawdź aktualizacje")
        .open(&mut is_open)
        .default_size([460.0, 220.0])
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Zainstalowana wersja:");
                ui.monospace(env!("CARGO_PKG_VERSION"));
            });

            ui.separator();

            match &app.update_check_status {
                None => {
                    ui.label("Kliknij „Sprawdź teraz”, aby porównać z najnowszym wydaniem na GitHub.");
                }
                Some(status) => {
                    ui.label(egui::RichText::new(status).size(13.0));
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("🔍 Sprawdź teraz").clicked() {
                    app.trigger_update_check();
                }
                if ui.button("🌐 Otwórz stronę wydań").clicked() {
                    let _ = open::that("https://github.com/sp6ina/SPLogbook/releases");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Zamknij").clicked() {
                        close_requested = true;
                    }
                });
            });
        });
    app.show_update_window = is_open && !close_requested;
}
