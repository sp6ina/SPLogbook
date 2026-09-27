// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
// Wbudowany podręcznik użytkownika (Instrukcja obsługi).

use crate::gui::app::SpLogApp;
use eframe::egui;

/// Sekcja podręcznika. Treść trzymana w kodzie (offline), gotowa na
/// późniejszą migrację do plików `.md` w `assets/docs/` per język.
pub struct ManualSection {
    pub id: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

pub const MANUAL_SECTIONS: &[ManualSection] = &[
    ManualSection {
        id: "start",
        title: "Pierwsze kroki",
        body:
"Witaj w SPLogbook — nowoczesnym dzienniku łączności krótkofalarskich.\n\n\
1. Uruchom kreator powitalny (menu Pomoc → Kreator pierwszego uruchomienia), aby\n   skonfigurować znak wywoławczy, imię i lokalizację stacji.\n\
2. Skonfiguruj radio w menu Konfiguracja → CAT / Radio (Hamlib lub TCI).\n\
3. Podłącz klaster DX w menu Widok → Panel DX Cluster (lub konfiguracja klastra).\n\
4. Rozpocznij logowanie: w panelu QSO wpisz znak korespondenta i naciśnij Enter —\n   aplikacja uzupełni dane z QRZ/HamQTH (jeśli skonfigurowano).\n\n\
Wskazówka: naciśnij Ctrl+Shift+P, aby otworzyć wyszukiwarkę poleceń i szybko\nprzejść do dowolnej funkcji.",
    },
    ManualSection {
        id: "station",
        title: "Konfiguracja stacji i CAT",
        body:
"SPLogbook obsługuje sterowanie radiem przez Hamlib (rigctld), natywny Hamlib\noraz TCI (Expert Electronics).\n\n\
- CAT / Radio: wybierz backend, model radiostacji, port szeregowy i prędkość.\n\
- Profile stacji: zapisuj wiele konfiguracji (dom, wyprawa SOTA, kontest).\n\
- Sprzęt: zarządzaj listą transceiverów, wzmacniaczy i anten.\n\n\
Po poprawnym połączeniu panel VFO pokazuje częstotliwość, pasmo, tryb i RST.\n\
Pasek operacyjny (Operating Bar) pozwala śledzić stan CAT, klastra i propagacji\nbez odrywania wzroku od pracy.",
    },
    ManualSection {
        id: "cluster",
        title: "Klaster DX",
        body:
"Klaster DX pokazuje spoty z sieci telnet (domyślnie cluster.sp7pka.ampr.org).\n\n\
- Włącz auto-łączenie i skonfiguruj swój znak w ustawieniach klastra.\n\
- Filtry: ogranicz spoty do bieżącego pasma, ukryj FT8 lub skimmery.\n\
- Kliknij spot, aby przepisać częstotliwość i znak do panelu QSO.\n\n\
Możesz dodać własne adresy klastrów w konfiguracji.",
    },
    ManualSection {
        id: "propagation",
        title: "Propagacja i pogoda kosmiczna",
        body:
"Panel słoneczny pokazuje warunki propagacyjne w czasie rzeczywistym.\n\n\
- SFI / SSN / K-Index / A-Index — bieżące wskaźniki aktywności słonecznej.\n\
- Przewidywane otwarcie pasm (VOACAP-lite): niezawodność obwodu, MUF/LUF,\n  FOT/OWF i status otwarcia (otwarte/marginalne/zamknięte).\n\
- Wskaźnik przy formularzu QSO podpowiada, czy pasmo jest obecnie otwarte.\n\n\
Wyniki są buforowane, aby nie obciążać sieci przy każdym odświeżeniu.",
    },
    ManualSection {
        id: "logging",
        title: "Logowanie QSO",
        body:
"Panel QSO to serce aplikacji.\n\n\
- Znak (Callsign): automatyczna weryfikacja i uzupełnienie kraju/DXCC.\n\
- Częstotliwość i pasmo ustawiane ręcznie lub z CAT/spotu klastra.\n\
- RST nadane/odebrane, raporty LoTW/eQSL/QSL, IOTA, strefy CQ/ITU.\n\
- Nagrywanie audio łączności (mikrofon) i podpinanie pliku do QSO.\n\
- Zapis: przycisk Zapisz lub Enter. QSO trafia do tabeli dziennika.\n\n\
Tabela dziennika obsługuje sortowanie po kolumnach, filtrowanie, zaznaczanie\nwielokrotne i operacje masowe (np. usuwanie).",
    },
    ManualSection {
        id: "awards",
        title: "Nagrody (DXCC, IOTA, WAS)",
        body:
"Moduł nagród śledzi postęp w kierunku dyplomów.\n\n\
- DXCC: pracujące i potwierdzone kraje (LoTW/QSL).\n\
- IOTA: wyspy z podziałem na kontynenty.\n\
- WAS: stany USA (worked/confirmed).\n\n\
Przeglądarki IOTA i WAS pokazują status każdej referencji: potrzebna, zapracowana,\npotwierdzona. Kliknij wpis, aby wstawić referencję bezpośrednio do formularza QSO.",
    },
    ManualSection {
        id: "contest",
        title: "Kontesty",
        body:
"Tryb kontestowy przyspiesza pracę podczas zawodów.\n\n\
- Wybierz szablon kontestu (wymiana raportu, numeracja).\n\
- Panel pokazuje metr tempa, licznik QSO i macierz mnożników.\n\
- Bandmapa podświetla nowe mnożniki.\n\n\
Użyj skrótów klawiszowych do szybkiego zapisu i przechodzenia między polami.",
    },
    ManualSection {
        id: "qsl",
        title: "QSL i wydruk naklejek",
        body:
"Projektant QSL generuje naklejki na karty pocztowe (formaty Avery A4).\n\n\
- Wybierz format arkusza (2x7, 2x8, 3x7, 3x8).\n\
- Reguły wyboru: tylko niepotwierdzone, filtruj po paśmie/trybie/dacie, ustaw limit.\n\
- Kalibracja: dopasuj marginesy, aby naklejki pokrywały pola arkusza.\n\
- Eksport do PDF: aplikacja waliduje kolejkę i znak stacji przed zapisem.\n\n\
Menedżer QSL pozwala oznaczać wysłane/odebrane potwierdzenia i śledzić kolejkę.",
    },
    ManualSection {
        id: "export",
        title: "Eksport i integracje",
        body:
"SPLogbook integruje się z popularnymi serwisami.\n\n\
- ADIF: import i eksport dziennika (pełna zgodność).\n\
- LoTW (Trusted QSL), eQSL, Club Log, QRZ, HRDLog, HamQTH, Cloudlog.\n\
- Raportowanie do PSK Reporter i odbiór zdarzeń z WSJT-X/JS8Call.\n\
- Scheduler uploadów z kolejką offline i limitem zapytań per usługa.\n\n\
Poświadczenia przechowywane są w systemowym magazynie (keyring), nie w pliku.",
    },
    ManualSection {
        id: "shortcuts",
        title: "Skróty klawiszowe",
        body:
"Ważniejsze skróty:\n\n\
- Ctrl+Shift+P — wyszukiwarka poleceń (command palette)\n\
- F1 — pełna lista skrótów klawiszowych\n\
- Enter — zapis QSO\n\
- Esc — zamknij aktywne okno/panel\n\n\
Niniejszy podręcznik otworzysz z menu Pomoc → Instrukcja obsługi.",
    },
];

/// Kontekstowa ikona pomocy „?” — otwiera podręcznik na wskazanej sekcji.
pub fn help_button(app: &mut SpLogApp, ui: &mut egui::Ui, section: &'static str) {
    if ui
        .small_button("?")
        .on_hover_text("Pomoc — otwórz podręcznik")
        .clicked()
    {
        app.show_user_manual = true;
        app.manual_section = Some(section.to_string());
    }
}

fn section_by_id(id: &str) -> usize {
    MANUAL_SECTIONS
        .iter()
        .position(|s| s.id == id)
        .unwrap_or(0)
}

/// Okno „Instrukcja obsługi” — panel boczny z sekcjami + przewijana treść.
pub fn render_user_manual_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_user_manual {
        return;
    }
    let mut is_open = app.show_user_manual;

    // Ustalenie aktywnej sekcji (z ewentualnego linku kontekstowego „?”).
    let selected = if let Some(section) = app.manual_section.take() {
        section_by_id(&section)
    } else {
        app.manual_selected
    };

    let mut new_selected = selected;
    let mut close_requested = false;

    egui::Window::new("📖 Instrukcja obsługi")
        .open(&mut is_open)
        .default_size([760.0, 520.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Panel boczny z listą sekcji.
                egui::ScrollArea::vertical().max_height(440.0).show(ui, |ui| {
                    for (i, sec) in MANUAL_SECTIONS.iter().enumerate() {
                        if ui
                            .selectable_label(new_selected == i, sec.title)
                            .clicked()
                        {
                            new_selected = i;
                        }
                    }
                });

                ui.separator();

                // Treść aktywnej sekcji.
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let sec = &MANUAL_SECTIONS[new_selected.min(MANUAL_SECTIONS.len() - 1)];
                    ui.heading(egui::RichText::new(sec.title).size(18.0).strong());
                    ui.separator();
                    for paragraph in sec.body.split("\n\n") {
                        ui.label(paragraph);
                        ui.add_space(4.0);
                    }
                });
            });

            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Zamknij").clicked() {
                    close_requested = true;
                }
            });
        });

    app.manual_selected = new_selected;
    app.show_user_manual = is_open && !close_requested;
}
