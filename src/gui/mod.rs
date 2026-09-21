// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

pub mod app;
pub mod awards_matrix;
pub mod bandmap;
pub mod cat_settings;
pub mod cluster_panel;
pub mod contest;
pub mod cw_macros;
pub mod logbook_table;
pub mod menu;
pub mod online_sync;
pub mod qso_entry;
pub mod satellites;
pub mod solar_panel;
pub mod station_ledger;
pub mod statistics;
pub mod vfo_panel;
pub mod welcome_wizard;
pub mod world_map;
pub mod journal_manager;
pub mod advanced_filter;
pub mod cw_terminal;
pub mod qsl_designer;
pub mod send_spot;
pub mod iota_browser;
pub mod states_browser;
pub mod qsl_manager;
pub mod photo_viewer;
pub mod prefix_manager;
pub mod mini_hud;
pub mod astronomy_dialog;
pub mod wol_dialog;
pub mod sota_dialog;
pub mod wspr_panel;

/// Pomocnik rysujący mały przycisk "↗" (Odepnij do osobnego okna OS) bezpośrednio na pasku tytułowym okna egui.
pub fn render_titlebar_popout_button(
    ctx: &eframe::egui::Context,
    id_str: &str,
    parent_layer: eframe::egui::LayerId,
    window_rect: eframe::egui::Rect,
    popout_target: &mut bool,
) {
    render_titlebar_popout_button_if(ctx, id_str, parent_layer, window_rect, popout_target, true);
}

/// Wersja warunkowa — rejestruje przycisk jako `sublayer` okna w pamięci egui.
/// Dzięki temu przycisk ZAWSZE pozostaje na wierzchu paska tytułowego swojego okna rodzica,
/// ale gdy inne okno przykryje to okno, przycisk prawidłowo chowa się pod oknem wierzchnim.
pub fn render_titlebar_popout_button_if(
    ctx: &eframe::egui::Context,
    id_str: &str,
    parent_layer: eframe::egui::LayerId,
    window_rect: eframe::egui::Rect,
    popout_target: &mut bool,
    show: bool,
) {
    if !show { return; }
    let btn_pos = window_rect.min + eframe::egui::vec2(22.0, 2.0);
    let area_id = eframe::egui::Id::new(id_str);
    let child_layer = eframe::egui::LayerId::new(eframe::egui::Order::Middle, area_id);

    // Rejestracja jako sublayer okna rodzica w egui:
    ctx.memory_mut(|mem| {
        mem.areas_mut().set_sublayer(parent_layer, child_layer);
    });

    eframe::egui::Area::new(area_id)
        .fixed_pos(btn_pos)
        .order(eframe::egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = eframe::egui::vec2(0.0, 0.0);
            ui.spacing_mut().button_padding = eframe::egui::vec2(3.0, 1.0);
            let btn = eframe::egui::Button::new(
                eframe::egui::RichText::new("↗").size(11.0).strong()
            );
            if ui.add(btn).on_hover_text("Otwórz w osobnym oknie systemu Windows (np. na drugi monitor)").clicked() {
                *popout_target = true;
            }
        });
}