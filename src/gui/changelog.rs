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
        version: "1.1",
        date: "2026",
        items: &[
            "Dodano pełną obsługę języka włoskiego (Italiano).",
            "Nowy, wysoce zoptymalizowany silnik i18n oparty na plikach JSON w locales/ (o ~65% szybsza kompilacja).",
            "Natywny bezpośredni protokół CAT Kenwood / Elecraft / Yaesu (komendy FA, FB, MD, TX, SM, FT).",
            "Pełna zgodność ze standardem ADIF 3 w UTF-8 (długość pól liczona w bajtach) dla LoTW/TQSL i ClubLog.",
            "Precyzyjne wyliczanie pozycji Słońca i Szarej Linii z Równaniem Czasu (Equation of Time).",
            "Poprawione dekodowanie znaczników czasu UTC z pakietów binarnych UDP WSJT-X / JTDX.",
            "Rozszerzenie pasma 160m (od 1800 kHz) oraz dodanie pasm mikrofalowych 13cm/3cm dla satelity QO-100.",
            "Poprawa logiki DUPE dla łączności Mixed oraz punktacji mnożników per-band w zawodach.",
            "Inicjalizacja env_logger oraz optymalizacja pętli UI dla kolejki Cloud Auto-Upload.",
        ],
    },
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
    let mut do_install = false;
    egui::Window::new("🔄 Sprawdź aktualizacje")
        .open(&mut is_open)
        .default_size([520.0, 340.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Zainstalowana wersja:");
                ui.monospace(env!("CARGO_PKG_VERSION"));
            });

            ui.separator();

            match &app.update_check_status {
                None => {
                    ui.label(
                        "Kliknij „Sprawdź teraz”, aby porównać z najnowszym wydaniem na GitHub.",
                    );
                }
                Some(status) => {
                    ui.label(egui::RichText::new(status).size(13.0));
                }
            }

            if let Some(release) = &app.update_available {
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .id_salt("update_release_body")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&release.body).size(12.0));
                    });
            }

            if let Some(inst) = &app.update_install_status {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(inst).size(13.0));
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("🔍 Sprawdź teraz").clicked() {
                    app.trigger_update_check();
                }
                if app.update_available.is_some()
                    && ui.button("⬇️ Zainstaluj i uruchom ponownie").clicked()
                {
                    do_install = true;
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
    if do_install {
        app.trigger_update_install();
    }
    app.show_update_window = is_open && !close_requested;
}
