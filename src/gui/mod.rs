// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

pub mod advanced_filter;
pub mod app;
pub mod astronomy_dialog;
pub mod awards_matrix;
pub mod bandmap;
pub mod cat_settings;
pub mod changelog;
pub mod cluster_panel;
pub mod command_palette;
pub mod contest;
pub mod csv_export_dialog;
pub mod cw_macros;
pub mod cw_terminal;
pub mod find_duplicates;
pub mod icons;
pub mod iota_browser;
pub mod journal_manager;
pub mod logbook_table;
pub mod marketplace;
pub mod menu;
pub mod mini_hud;
pub mod online_sync;
pub mod operator_assistant;
pub mod photo_viewer;
pub mod plugin_manager;
pub mod prefix_manager;
pub mod qsl_designer;
pub mod qsl_manager;
pub mod qso_entry;
pub mod satellites;
pub mod send_spot;
pub mod solar_panel;
pub mod sota_dialog;
pub mod states_browser;
pub mod station_ledger;
pub mod station_profiles;
pub mod statistics;
pub mod theme;
pub mod user_manual;
pub mod vfo_panel;
pub mod voice_keyer;
pub mod waterfall_panel;
pub mod welcome_wizard;
pub mod wol_dialog;
pub mod workspace_profiles;
pub mod world_map;
pub mod wspr_panel;

/// Pomocnik rysujący mały przycisk "↗" (Odepnij do osobnego okna OS) bezpośrednio na pasku tytułowym okna egui.

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
    if !show {
        return;
    }
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
            let btn =
                eframe::egui::Button::new(eframe::egui::RichText::new("↗").size(11.0).strong());
            if ui
                .add(btn)
                .on_hover_text("Otwórz w osobnym oknie systemu Windows (np. na drugi monitor)")
                .clicked()
            {
                *popout_target = true;
            }
        });
}
